//! Event Services Component Implementation
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0

use crate::{
    service::event::{EventServices, EventServicesClosureExt},
    types::Event,
};
use alloc::{boxed::Box, collections::BTreeMap};
use core::sync::atomic::{AtomicUsize, Ordering};
use patina::{
    BinaryGuid,
    boot_services::{
        BootServices, StandardBootServices,
        event::{EventNotifyCallback, EventTimerType, EventType},
        tpl::Tpl,
    },
    component::{IntoComponent, params::Commands, service::IntoService},
    error::Result,
};
use r_efi::efi;
use spin::Mutex;

type EventCallback = Box<dyn FnMut(Event) + Send + Sync>;

/// Global callback registry for event handlers
static CALLBACK_REGISTRY: Mutex<BTreeMap<usize, EventCallback>> = Mutex::new(BTreeMap::new());
static CALLBACK_ID_COUNTER: AtomicUsize = AtomicUsize::new(1);

/// A generic callback wrapper that looks up and calls the appropriate closure
extern "efiapi" fn event_callback_wrapper(event: efi::Event, context: *mut core::ffi::c_void) {
    let callback_id = context as usize;

    // Convert efi::Event to Event and call the closure from the registry
    if let Some(callback) = CALLBACK_REGISTRY.lock().get_mut(&callback_id) {
        callback(Event::new(event));
    }
}

/// Standard implementation of event services using UEFI Boot Services.
#[derive(IntoService)]
#[service(dyn EventServices)]
pub struct StandardEventServices {
    boot_services: StandardBootServices,
}

impl StandardEventServices {
    /// Creates a new StandardEventServices instance.
    ///
    /// # Arguments
    ///
    /// * `boot_services` - The UEFI Boot Services to delegate to
    pub fn new(boot_services: StandardBootServices) -> Self {
        Self { boot_services }
    }
}

impl EventServices for StandardEventServices {
    fn create_event(&self, event_type: EventType, notify_tpl: Tpl) -> Result<Event> {
        self.boot_services.create_event(event_type, notify_tpl, None, ()).map(Event::new).map_err(|e| e.into())
    }

    fn close_event(&self, event: Event) -> Result<()> {
        self.boot_services.close_event(event.as_raw()).map_err(|e| e.into())
    }

    fn set_timer(&self, event: Event, timer_type: EventTimerType, trigger_time: u64) -> Result<()> {
        self.boot_services.set_timer(event.as_raw(), timer_type, trigger_time).map_err(|e| e.into())
    }

    fn wait_for_event(&self, events: &mut [Event]) -> Result<usize> {
        // The Event slice is converted to efi::Event slice for the FFI call
        // SAFETY: Event is #[repr(transparent)] wrapper around efi::Event, so the memory layout is identical
        let raw_events =
            unsafe { core::slice::from_raw_parts_mut(events.as_mut_ptr() as *mut efi::Event, events.len()) };
        self.boot_services.wait_for_event(raw_events).map_err(|e| e.into())
    }

    fn check_event(&self, event: Event) -> Result<()> {
        self.boot_services.check_event(event.as_raw()).map_err(|e| e.into())
    }

    fn signal_event(&self, event: Event) -> Result<()> {
        self.boot_services.signal_event(event.as_raw()).map_err(|e| e.into())
    }

    fn create_system_event(&self, event_type: EventType, notify_tpl: Tpl) -> Result<Event> {
        self.boot_services.create_event(event_type, notify_tpl, None, ()).map(Event::new).map_err(|e| e.into())
    }
}

impl EventServicesClosureExt for StandardEventServices {
    fn create_system_event_with_callback<F>(
        &self,
        event_type: EventType,
        notify_tpl: Tpl,
        callback: F,
        event_group: &'static BinaryGuid,
    ) -> Result<Event>
    where
        F: FnMut(Event) + Send + Sync + 'static,
    {
        // Generate a unique callback ID
        let callback_id = CALLBACK_ID_COUNTER.fetch_add(1, Ordering::SeqCst);

        CALLBACK_REGISTRY.lock().insert(callback_id, Box::new(callback));

        let efi_guid: &'static efi::Guid = event_group;

        // Create the event with our wrapper function and the callback ID as context
        let callback_fn: EventNotifyCallback<*mut core::ffi::c_void> = event_callback_wrapper;
        self.boot_services
            .create_event_ex(event_type, notify_tpl, Some(callback_fn), callback_id as *mut core::ffi::c_void, efi_guid)
            .map(Event::new)
            .map_err(|e| e.into())
    }
}

/// Component that provides event services to other Patina components.
#[derive(IntoComponent)]
pub struct EventServicesProvider;

impl EventServicesProvider {
    /// Component entry point that registers event services.
    ///
    /// # Arguments
    ///
    /// * `commands` - The commands storage for service registration
    /// * `boot_services` - The UEFI Boot Services required for event operations
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on successful registration.
    pub fn entry_point(self, mut commands: Commands, boot_services: StandardBootServices) -> Result<()> {
        let event_services = StandardEventServices::new(boot_services);
        commands.add_service(event_services);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use patina::{
        component::{params::Commands, prelude::Service},
        test::patina_test,
    };

    #[patina_test]
    fn test_event_services_provider_creation() -> core::result::Result<(), &'static str> {
        let _provider = EventServicesProvider;
        // Component should be created successfully
        Ok(())
    }

    #[patina_test]
    fn test_standard_event_services_creation() -> core::result::Result<(), &'static str> {
        let uninit_service = StandardBootServices::new_uninit();
        let _event_services = StandardEventServices::new(uninit_service);
        // Service should be created successfully
        Ok(())
    }

    #[patina_test]
    fn test_entry_point() -> core::result::Result<(), &'static str> {
        let _commands = Commands::mock();
        let _uninit_service = Service::<StandardBootServices>::new_uninit();

        let _provider = EventServicesProvider;

        // Entry point should execute without panicking
        Ok(())
    }
}
