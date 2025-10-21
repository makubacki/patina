//! Miscellaneous UEFI Services implementations.
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0

use crate::service::misc::{ConfigurationServices, MemoryUtilityServices, SystemUtilityServices, TimingServices};
use alloc::boxed::Box;
use core::ffi::c_void;
use patina::{
    BinaryGuid,
    boot_services::{BootServices, StandardBootServices},
    component::{IntoComponent, params::Commands},
    error::{EfiError, Result},
};
use patina_macro::IntoService;

/// Standard implementation of `TimingServices` that delegates to `StandardBootServices`.
#[derive(IntoService)]
#[service(dyn TimingServices)]
pub struct StandardTimingServices {
    boot_services: StandardBootServices,
}

impl StandardTimingServices {
    /// Creates a new `StandardTimingServices` instance.
    ///
    /// # Arguments
    ///
    /// * `boot_services` - The underlying boot services to delegate to
    pub fn new(boot_services: StandardBootServices) -> Self {
        Self { boot_services }
    }
}

impl TimingServices for StandardTimingServices {
    fn set_watchdog_timer(&self, timeout: usize) -> Result<()> {
        self.boot_services.set_watchdog_timer(timeout).map_err(EfiError::from)
    }

    fn stall(&self, microseconds: usize) -> Result<()> {
        self.boot_services.stall(microseconds).map_err(EfiError::from)
    }
}

/// Standard implementation of `MemoryUtilityServices` that delegates to `StandardBootServices`.
#[derive(IntoService)]
#[service(StandardMemoryUtilityServices)]
pub struct StandardMemoryUtilityServices {
    boot_services: StandardBootServices,
}

impl StandardMemoryUtilityServices {
    /// Creates a new `StandardMemoryUtilityServices` instance.
    ///
    /// # Arguments
    ///
    /// * `boot_services` - The underlying boot services to delegate to
    pub fn new(boot_services: StandardBootServices) -> Self {
        Self { boot_services }
    }
}

impl MemoryUtilityServices for StandardMemoryUtilityServices {
    fn copy_mem<T: 'static>(&self, dest: &mut T, src: &T) {
        self.boot_services.copy_mem(dest, src);
    }

    unsafe fn copy_mem_unchecked(&self, dest: *mut c_void, src: *const c_void, length: usize) {
        unsafe {
            self.boot_services.copy_mem_unchecked(dest, src, length);
        }
    }

    fn set_mem(&self, buffer: &mut [u8], value: u8) {
        self.boot_services.set_mem(buffer, value);
    }
}

/// Standard implementation of `SystemUtilityServices` that delegates to `StandardBootServices`.
#[derive(IntoService)]
#[service(StandardSystemUtilityServices)]
pub struct StandardSystemUtilityServices {
    boot_services: StandardBootServices,
}

impl StandardSystemUtilityServices {
    /// Creates a new `StandardSystemUtilityServices` instance.
    ///
    /// # Arguments
    ///
    /// * `boot_services` - The underlying boot services to delegate to
    pub fn new(boot_services: StandardBootServices) -> Self {
        Self { boot_services }
    }
}

impl SystemUtilityServices for StandardSystemUtilityServices {
    fn get_next_monotonic_count(&self) -> Result<u64> {
        self.boot_services.get_next_monotonic_count().map_err(EfiError::from)
    }

    fn calculate_crc_32<T: 'static>(&self, data: &T) -> Result<u32> {
        self.boot_services.calculate_crc_32(data).map_err(EfiError::from)
    }

    unsafe fn calculate_crc_32_unchecked(&self, data: *const c_void, data_size: usize) -> Result<u32> {
        unsafe { self.boot_services.calculate_crc_32_unchecked(data, data_size).map_err(EfiError::from) }
    }
}

/// Standard implementation of `ConfigurationServices` that delegates to `StandardBootServices`.
#[derive(IntoService)]
#[service(StandardConfigurationServices)]
pub struct StandardConfigurationServices {
    boot_services: StandardBootServices,
}

impl StandardConfigurationServices {
    /// Creates a new `StandardConfigurationServices` instance.
    ///
    /// # Arguments
    ///
    /// * `boot_services` - The underlying boot services to delegate to
    pub fn new(boot_services: StandardBootServices) -> Self {
        Self { boot_services }
    }

    /// Installs a configuration table.
    ///
    /// This is considered a safe interface to install configuration tables because:
    /// - `Box<T>` is used to ensure proper ownership and lifetime management
    /// - Type safety is provided through generics
    /// - Raw pointers for interaction with underlying UEFI defined interfaces are
    ///   handled internal to the implementation
    ///
    /// The table data will be leaked (using `Box::into_raw`) to ensure it remains
    /// valid for the lifetime of the system, as required by UEFI configuration tables.
    ///
    /// # Type Parameters
    /// - `T` - The type of the configuration table data (must be `'static`)
    ///
    /// # Parameters
    /// - `guid` - GUID that identifies the configuration table
    /// - `table` - Boxed table data that will be installed
    ///
    /// # Returns
    /// - `Ok(())` on success
    /// - `Err(status)` on failure
    ///
    /// # Example
    /// ```rust
    /// use r_efi::efi;
    /// use patina::{component::{IntoComponent, prelude::Service}, BinaryGuid};
    /// use patina_uefi_services::component::misc::StandardConfigurationServices;
    /// use patina::boot_services::StandardBootServices;
    ///
    /// #[derive(Debug)]
    /// struct MyConfigTable {
    ///     version: u32,
    ///     data: [u8; 16],
    /// }
    ///
    /// #[derive(IntoComponent)]
    /// struct MyComponent;
    ///
    /// impl MyComponent {
    ///     fn entry_point(self, config: Service<StandardConfigurationServices>) -> patina::error::Result<()> {
    ///         let guid = BinaryGuid::from_fields(0x12345678, 0x1234, 0x5678, 0x12, 0x34, &[0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0]);
    ///         let table = Box::new(MyConfigTable { version: 1, data: [0; 16] });
    ///         config.install_configuration_table(&guid, table)?;
    ///         Ok(())
    ///     }
    /// }
    /// ```
    pub fn install_configuration_table<T: 'static>(&self, guid: &BinaryGuid, table: Box<T>) -> Result<()> {
        // Convert Box to raw pointer and delegate to unchecked version
        let table_ptr = Box::into_raw(table) as *mut c_void;
        unsafe { self.install_configuration_table_unchecked(guid, table_ptr) }
    }
}

impl ConfigurationServices for StandardConfigurationServices {
    fn remove_configuration_table(&self, guid: &BinaryGuid) -> Result<()> {
        unsafe { self.install_configuration_table_unchecked(guid, core::ptr::null_mut()) }
    }

    unsafe fn install_configuration_table_unchecked(&self, guid: &BinaryGuid, table: *mut c_void) -> Result<()> {
        // SAFETY: Caller guarantees that the table pointer is valid or null.
        let efi_guid = guid;

        unsafe { self.boot_services.install_configuration_table_unchecked(efi_guid, table).map_err(EfiError::from) }
    }
}

/// Component that provides `TimingServices` to the system.
#[derive(IntoComponent)]
pub struct TimingServicesProvider;

impl TimingServicesProvider {
    /// Component entry point.
    pub fn entry_point(self, boot_services: StandardBootServices, mut commands: Commands) -> Result<()> {
        let timing_services = StandardTimingServices::new(boot_services);
        commands.add_service(timing_services);
        Ok(())
    }
}

/// Component that provides `MemoryUtilityServices` to the system.
#[derive(IntoComponent)]
pub struct MemoryUtilityServicesProvider;

impl MemoryUtilityServicesProvider {
    /// Component entry point.
    pub fn entry_point(self, boot_services: StandardBootServices, mut commands: Commands) -> Result<()> {
        let memory_utility_services = StandardMemoryUtilityServices::new(boot_services);
        commands.add_service(memory_utility_services);
        Ok(())
    }
}

/// Component that provides `SystemUtilityServices` to the system.
#[derive(IntoComponent)]
pub struct SystemUtilityServicesProvider;

impl SystemUtilityServicesProvider {
    /// Component entry point.
    pub fn entry_point(self, boot_services: StandardBootServices, mut commands: Commands) -> Result<()> {
        let system_utility_services = StandardSystemUtilityServices::new(boot_services);
        commands.add_service(system_utility_services);
        Ok(())
    }
}

/// Component that provides `ConfigurationServices` to the system.
#[derive(IntoComponent)]
pub struct ConfigurationServicesProvider;

impl ConfigurationServicesProvider {
    /// Component entry point.
    pub fn entry_point(self, boot_services: StandardBootServices, mut commands: Commands) -> Result<()> {
        let configuration_services = StandardConfigurationServices::new(boot_services);
        commands.add_service(configuration_services);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_creation() {
        let _timing_provider = TimingServicesProvider;
        let _memory_utility_provider = MemoryUtilityServicesProvider;
        let _system_utility_provider = SystemUtilityServicesProvider;
        let _configuration_provider = ConfigurationServicesProvider;
    }
}
