//! DXE Core implementation of [`ProtocolServices`].
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
//!

use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use core::ffi::c_void;

use patina::component::service::{
    IntoService,
    uefi_services::protocol::{
        Handle, NotifyCallback, NotifyRegistration, ProtocolError, ProtocolPtr, ProtocolServices, Tpl,
    },
};
use patina::error::EfiError;
use patina::standard::efi;

use crate::events::EVENT_DB;
use crate::protocols::{PROTOCOL_DB, core_install_protocol_interface, core_uninstall_protocol_interface};

use super::event::tpl_to_efi;

/// Owns a component-supplied installation-notify closure for the lifetime of a registration (until cancelled).
struct NotifyHolder {
    callback: NotifyCallback,
    registration: *mut c_void,
}

/// C-ABI trampoline signaled when a watched protocol is installed.
///
/// Drains the handles that the registration has flagged as installed and invokes the
/// component-supplied closure for each.
extern "efiapi" fn notify_install_trampoline(_event: efi::Event, context: *mut c_void) {
    if context.is_null() {
        return;
    }
    // SAFETY: `context` is the `NotifyHolder` created in `register_install_notify`, valid until
    // `cancel_install_notify` reclaims it. Notifications dispatch serially at the event's TPL.
    let holder = unsafe { &mut *(context as *mut NotifyHolder) };
    while let Some(handle) = PROTOCOL_DB.next_handle_for_registration(holder.registration) {
        if let Some(handle) = Handle::from_raw(handle) {
            (holder.callback)(handle);
        }
    }
}

/// Core implementation of [`ProtocolServices`], delegating to the core protocol database through the
/// internal `core_*` Rust APIs.
#[derive(IntoService)]
#[service(dyn ProtocolServices)]
pub(crate) struct CoreProtocolServices;

impl ProtocolServices for CoreProtocolServices {
    fn install_interface(
        &self,
        handle: Option<Handle>,
        protocol: efi::Guid,
        interface: ProtocolPtr,
    ) -> Result<Handle, ProtocolError> {
        let caller_handle = handle.map(|handle| handle.as_raw());
        let installed = core_install_protocol_interface(caller_handle, protocol, interface.as_raw())
            .map_err(ProtocolError::from)?;
        Handle::from_raw(installed).ok_or(ProtocolError::Internal)
    }

    fn uninstall_interface(
        &self,
        handle: Handle,
        protocol: efi::Guid,
        interface: ProtocolPtr,
    ) -> Result<(), ProtocolError> {
        core_uninstall_protocol_interface(handle.as_raw(), protocol, interface.as_raw()).map_err(ProtocolError::from)
    }

    fn locate_interface(&self, protocol: efi::Guid) -> Result<ProtocolPtr, ProtocolError> {
        let interface = PROTOCOL_DB.locate_protocol(protocol).map_err(ProtocolError::from)?;
        ProtocolPtr::from_raw(interface).ok_or(ProtocolError::NotFound)
    }

    fn locate_handles(&self, protocol: efi::Guid) -> Result<Vec<Handle>, ProtocolError> {
        // "No handles present" is not an error, report it as an empty list so callers can iterate
        // without special-casing absence.
        match PROTOCOL_DB.locate_handles(Some(protocol)) {
            Ok(handles) => Ok(handles.into_iter().filter_map(Handle::from_raw).collect()),
            Err(EfiError::NotFound) => Ok(Vec::new()),
            Err(err) => Err(ProtocolError::from(err)),
        }
    }

    fn interface_on_handle(&self, handle: Handle, protocol: efi::Guid) -> Result<ProtocolPtr, ProtocolError> {
        let interface = PROTOCOL_DB.get_interface_for_handle(handle.as_raw(), protocol).map_err(ProtocolError::from)?;
        ProtocolPtr::from_raw(interface).ok_or(ProtocolError::NotFound)
    }

    fn register_install_notify(
        &self,
        protocol: efi::Guid,
        notify_tpl: Tpl,
        callback: NotifyCallback,
    ) -> Result<NotifyRegistration, ProtocolError> {
        let holder = Box::new(NotifyHolder { callback, registration: core::ptr::null_mut() });
        let context = Box::into_raw(holder) as *mut c_void;

        // Create a notify-signal event whose closure drains newly installed handles.
        let event = match EVENT_DB.create_event(
            efi::EVT_NOTIFY_SIGNAL,
            tpl_to_efi(notify_tpl),
            Some(notify_install_trampoline),
            Some(context),
            None,
        ) {
            Ok(event) => event,
            Err(err) => {
                // SAFETY: `context` came from `Box::into_raw` above and has not been freed.
                drop(unsafe { Box::from_raw(context as *mut NotifyHolder) });
                return Err(ProtocolError::from(err));
            }
        };

        let registration = match PROTOCOL_DB.register_protocol_notify(protocol, event) {
            Ok(registration) => registration,
            Err(err) => {
                let _ = EVENT_DB.close_event(event);
                // SAFETY: `context` came from `Box::into_raw` above and has not been freed.
                drop(unsafe { Box::from_raw(context as *mut NotifyHolder) });
                return Err(ProtocolError::from(err));
            }
        };

        // Record the registration key so the trampoline can drain matching handles. This runs
        // before any notification can fire, since queued notifications dispatch only once the TPL
        // drops below TPL_CALLBACK after this call returns.
        // SAFETY: `context` is the `NotifyHolder` pointer created above and has not been freed.
        unsafe {
            (*(context as *mut NotifyHolder)).registration = registration;
        }

        // Deliver handles that already have the protocol installed.
        let _ = EVENT_DB.signal_event(event);

        Ok(NotifyRegistration::from_raw(event, registration, context))
    }

    fn cancel_install_notify(&self, registration: NotifyRegistration) -> Result<(), ProtocolError> {
        let event = registration.event();
        let context = registration.context();

        PROTOCOL_DB.unregister_protocol_notify_events(vec![event]);

        EVENT_DB.close_event(event).map_err(ProtocolError::from)?;

        if !context.is_null() {
            // SAFETY: `context` is the `NotifyHolder` created in `register_install_notify`. The
            // event is now closed and unregistered, so no further notification can reference it.
            drop(unsafe { Box::from_raw(context as *mut NotifyHolder) });
        }

        Ok(())
    }
}
