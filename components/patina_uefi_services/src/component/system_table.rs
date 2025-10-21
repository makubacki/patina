//! System Table Service Implementation
//!
//! Provides the actual implementation of system table access using boot services protocol location.
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0

use crate::service::system_table::SystemTableService;
use patina::{
    boot_services::{BootServices, StandardBootServices},
    component::{IntoComponent, params::Commands, service::IntoService},
    error::{EfiError, Result},
};
use r_efi::efi;

/// Standard implementation of system table services.
///
/// This implementation provides access to console protocols by locating them
/// through the boot services LocateProtocol interface rather than direct system table access.
#[derive(IntoService)]
#[service(dyn SystemTableService)]
pub struct StandardSystemTableService {
    boot_services: StandardBootServices,
}

impl StandardSystemTableService {
    /// Creates a new StandardSystemTableService instance.
    pub fn new(boot_services: StandardBootServices) -> Self {
        Self { boot_services }
    }
}

impl SystemTableService for StandardSystemTableService {
    fn get_console_input(&self) -> Result<&'static mut efi::protocols::simple_text_input::Protocol> {
        // Use LocateProtocol to find the console input protocol
        unsafe {
            self.boot_services
                .locate_protocol::<efi::protocols::simple_text_input::Protocol>(None)
                .map_err(EfiError::from)
        }
    }

    fn get_console_output(&self) -> Result<&'static mut efi::protocols::simple_text_output::Protocol> {
        // Use LocateProtocol to find the console output protocol
        unsafe {
            self.boot_services
                .locate_protocol::<efi::protocols::simple_text_output::Protocol>(None)
                .map_err(EfiError::from)
        }
    }

    fn get_standard_error(&self) -> Result<&'static mut efi::protocols::simple_text_output::Protocol> {
        // In UEFI, standard error typically uses the same protocol as console output
        // but may be on a different handle. For now, return the same as console output.
        self.get_console_output()
    }
}

/// Component that produces `SystemTableService` for other components.
#[derive(IntoComponent)]
pub struct SystemTableServiceProvider;

impl SystemTableServiceProvider {
    /// Entry point for the `SystemTableServiceProvider` component.
    pub fn entry_point(self, mut commands: Commands, boot_services: StandardBootServices) -> Result<()> {
        let system_table_service = StandardSystemTableService::new(boot_services);
        commands.add_service(system_table_service);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_system_table_service_creation() {
        // Test that StandardSystemTableService can be created
        let boot_services = StandardBootServices::new_uninit();
        let _system_table_service = StandardSystemTableService::new(boot_services);
        // This test just checks that creation does not panic
    }

    #[test]
    fn test_system_table_service_provider_creation() {
        // Test that SystemTableServiceProvider can be created
        let _provider = SystemTableServiceProvider;
        // This test just checks that creation does not panic
    }
}
