//! Protocol Services Implementation
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
//!

use crate::{service::protocol::ProtocolServices, types::Handle};
use alloc::vec::Vec;
use core::ffi::c_void;
use patina::{
    BinaryGuid,
    boot_services::{BootServices, StandardBootServices, protocol_handler::HandleSearchType},
    component::{IntoComponent, params::Commands},
    error::{EfiError, Result},
};
use patina_macro::IntoService;
use r_efi::efi;

/// Standard implementation of `ProtocolServices` that delegates to `StandardBootServices`.
#[derive(IntoService)]
#[service(dyn ProtocolServices)]
pub struct StandardProtocolServices {
    boot_services: StandardBootServices,
}

impl StandardProtocolServices {
    /// Creates a new StandardProtocolServices instance.
    ///
    /// # Arguments
    ///
    /// * `boot_services` - The underlying boot services to delegate to
    pub fn new(boot_services: StandardBootServices) -> Self {
        Self { boot_services }
    }
}

impl ProtocolServices for StandardProtocolServices {
    unsafe fn install_protocol_interface(
        &self,
        handle: &mut Handle,
        protocol: &'static BinaryGuid,
        _interface_type: efi::InterfaceType,
        interface: *mut c_void,
    ) -> Result<()> {
        // SAFETY: Caller guarantees that the interface pointer is valid and matches the protocol type.
        // We delegate to the underlying boot services which performs the actual unsafe operation.

        let efi_guid: &'static efi::Guid = protocol.as_efi_guid();

        let new_handle = unsafe {
            self.boot_services
                .install_protocol_interface_unchecked(
                    if handle.is_null() { None } else { Some(handle.as_raw()) },
                    efi_guid,
                    interface,
                )
                .map_err(EfiError::from)?
        };

        // Update the handle if it was created
        *handle = Handle::new(new_handle);
        Ok(())
    }
    unsafe fn uninstall_protocol_interface(
        &self,
        handle: Handle,
        protocol: &'static BinaryGuid,
        interface: *mut c_void,
    ) -> Result<()> {
        // SAFETY: Caller guarantees that this is safe to uninstall (handle and protocol match),
        // and that no other code is using the protocol interface.

        let efi_guid: &'static efi::Guid = protocol;

        unsafe {
            self.boot_services
                .uninstall_protocol_interface_unchecked(handle.as_raw(), efi_guid, interface)
                .map_err(EfiError::from)
        }
    }

    unsafe fn handle_protocol(&self, handle: Handle, protocol: &'static BinaryGuid) -> Result<*mut c_void> {
        // SAFETY: Caller is responsible for casting the returned pointer to the correct type
        // and ensuring the protocol is not used after being uninstalled.

        let efi_guid: &'static efi::Guid = protocol;

        unsafe { self.boot_services.handle_protocol_unchecked(handle.as_raw(), efi_guid).map_err(EfiError::from) }
    }

    unsafe fn locate_protocol(
        &self,
        protocol: &'static BinaryGuid,
        registration: Option<*mut c_void>,
    ) -> Result<*mut c_void> {
        // SAFETY: Caller is responsible for casting the returned pointer to the correct type
        // and ensuring proper lifetime management of the protocol interface.

        let efi_guid: &'static efi::Guid = protocol;

        unsafe {
            self.boot_services
                .locate_protocol_unchecked(efi_guid, registration.unwrap_or(core::ptr::null_mut()))
                .map_err(EfiError::from)
        }
    }

    fn locate_handle_buffer(&self, search_type: HandleSearchType) -> Result<Vec<Handle>> {
        let handles_box = self.boot_services.locate_handle_buffer(search_type).map_err(EfiError::from)?;

        // Convert from BootServicesBox to Vec, wrapping each handle in our Handle newtype
        Ok(handles_box.iter().copied().map(Handle::new).collect())
    }
}

/// Component that provides `ProtocolServices` to the system.
#[derive(IntoComponent)]
pub struct ProtocolServicesProvider;

impl ProtocolServicesProvider {
    /// Component entry point.
    ///
    /// # Arguments
    ///
    /// * `boot_services` - The underlying boot services
    /// * `commands` - Commands interface for service registration
    pub fn entry_point(self, boot_services: StandardBootServices, mut commands: Commands) -> Result<()> {
        let protocol_services = StandardProtocolServices::new(boot_services);
        commands.add_service(protocol_services);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_protocol_services_creation() {
        let boot_services = StandardBootServices::new_uninit();
        let _protocol_services = StandardProtocolServices::new(boot_services);
    }

    #[test]
    fn test_protocol_services_provider_creation() {
        // Just test that that the provider component can be instantiated
        let _provider = ProtocolServicesProvider;
    }
}
