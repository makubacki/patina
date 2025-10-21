//! Miscellaneous UEFI Services Usage Examples
//!
//! This module demonstrates how to use various miscellaneous UEFI services including timing,
//! memory utilities, system utilities, and configuration table management.
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0

extern crate alloc;

use core::ffi::c_void;
use patina::{BinaryGuid, component::IntoComponent, component::prelude::Service, error::Result};
use patina_uefi_services::{
    component::misc::{StandardMemoryUtilityServices, StandardSystemUtilityServices},
    service::misc::{ConfigurationServices, MemoryUtilityServices, SystemUtilityServices, TimingServices},
};

//
// Timing Services Examples
//

/// Basic watchdog timer usage.
///
/// This component demonstrates how to configure the system watchdog timer
/// to reset the system if it becomes unresponsive.
///
/// ## Example Usage
///
/// ```ignore
/// use patina::component::IntoComponent;
/// use patina_samples::component::misc_usage::WatchdogTimerExample;
///
/// // Register the component with the core
/// dxe_core.with_component(WatchdogTimerExample);
/// ```
#[derive(IntoComponent)]
pub struct WatchdogTimerExample;

impl WatchdogTimerExample {
    fn entry_point(self, timing: Service<dyn TimingServices>) -> Result<()> {
        // Set a 5-minute (300 second) watchdog timer
        // The system will reset if we don't complete our work within this time
        timing.set_watchdog_timer(300)?;

        // Perform some critical operations...
        // (In a real component, this would be your actual work)

        // Disable the watchdog timer when done
        timing.set_watchdog_timer(0)?;

        Ok(())
    }
}

/// Precise delay operations using stall.
///
/// This component demonstrates how to use the stall function for precise
/// timing delays in microseconds.
///
/// ## Example Usage
///
/// ```ignore
/// use patina::component::IntoComponent;
/// use patina_samples::component::misc_usage::PreciseDelayExample;
///
/// // Register the component with the core
/// dxe_core.with_component(PreciseDelayExample);
/// ```
#[derive(IntoComponent)]
pub struct PreciseDelayExample;

impl PreciseDelayExample {
    fn entry_point(self, timing: Service<dyn TimingServices>) -> Result<()> {
        // Wait 1 millisecond (1000 microseconds)
        timing.stall(1000)?;

        // Wait 100 microseconds
        timing.stall(100)?;

        // Note: stall is a blocking operation - use sparingly
        // For longer delays, consider using event timers instead

        Ok(())
    }
}

//
// Memory Utility Services Examples
//

/// Safe memory copying operations.
///
/// This component demonstrates type-safe memory copying using the MemoryUtilityServices.
///
/// ## Example Usage
///
/// ```ignore
/// use patina::component::IntoComponent;
/// use patina_samples::component::misc_usage::SafeMemoryCopyExample;
///
/// // Register the component with the core
/// dxe_core.with_component(SafeMemoryCopyExample);
/// ```
#[derive(IntoComponent)]
pub struct SafeMemoryCopyExample;

impl SafeMemoryCopyExample {
    fn entry_point(self, mem_utils: Service<StandardMemoryUtilityServices>) -> Result<()> {
        // Note that `mem_utils.copy_mem()` is a type-safe memory copy
        let source = 0x12345678u32;
        let mut destination = 0u32;

        mem_utils.copy_mem(&mut destination, &source);

        // Verify the copy
        assert_eq!(destination, 0x12345678);

        Ok(())
    }
}

/// Memory buffer filling operations.
///
/// This component demonstrates how to fill memory buffers with specific values.
///
/// ## Example Usage
///
/// ```ignore
/// use patina::component::IntoComponent;
/// use patina_samples::component::misc_usage::MemoryFillExample;
///
/// // Register the component with the core
/// dxe_core.with_component(MemoryFillExample);
/// ```
#[derive(IntoComponent)]
pub struct MemoryFillExample;

impl MemoryFillExample {
    fn entry_point(self, mem_utils: Service<StandardMemoryUtilityServices>) -> Result<()> {
        // Create a buffer and fill it with zeros
        let mut buffer = [0xFFu8; 64];
        mem_utils.set_mem(&mut buffer, 0x00);

        // Verify all bytes are zero
        assert!(buffer.iter().all(|&b| b == 0x00));

        // Fill with a different value
        mem_utils.set_mem(&mut buffer, 0xAA);

        // Verify all bytes are 0xAA
        assert!(buffer.iter().all(|&b| b == 0xAA));

        Ok(())
    }
}

//
// System Utility Services Examples
//

/// Monotonic counter usage for unique identifiers.
///
/// This component demonstrates how to use the monotonic counter to generate
/// unique sequence numbers during the current boot.
///
/// ## Example Usage
///
/// ```ignore
/// use patina::component::IntoComponent;
/// use patina_samples::component::misc_usage::MonotonicCounterExample;
///
/// // Register the component with the core
/// dxe_core.with_component(MonotonicCounterExample);
/// ```
#[derive(IntoComponent)]
pub struct MonotonicCounterExample;

impl MonotonicCounterExample {
    fn entry_point(self, sys_utils: Service<StandardSystemUtilityServices>) -> Result<()> {
        // Get three sequential counter values
        let count1 = sys_utils.get_next_monotonic_count()?;
        let count2 = sys_utils.get_next_monotonic_count()?;
        let count3 = sys_utils.get_next_monotonic_count()?;

        // Verify they are monotonically increasing
        assert!(count2 > count1);
        assert!(count3 > count2);

        Ok(())
    }
}

/// CRC32 calculation for data integrity.
///
/// This component demonstrates how to calculate CRC32 checksums for data
/// integrity verification.
///
/// ## Example Usage
///
/// ```ignore
/// use patina::component::IntoComponent;
/// use patina_samples::component::misc_usage::Crc32CalculationExample;
///
/// // Register the component with the core
/// dxe_core.with_component(Crc32CalculationExample);
/// ```
#[derive(IntoComponent)]
pub struct Crc32CalculationExample;

impl Crc32CalculationExample {
    fn entry_point(self, sys_utils: Service<StandardSystemUtilityServices>) -> Result<()> {
        // Calculate CRC32 for a simple value
        let data = 0x12345678u32;
        let crc = sys_utils.calculate_crc_32(&data)?;

        // Calculate CRC for a larger structure
        let complex_data = (0x11223344u32, 0x55667788u32);
        let complex_crc = sys_utils.calculate_crc_32(&complex_data)?;

        // CRCs should differ for different data
        assert_ne!(crc, complex_crc);

        Ok(())
    }
}

//
// Configuration Services Examples
//

/// Installing a configuration table.
///
/// This component demonstrates how to install a configuration table that can
/// be accessed by other components or the operating system.
///
/// ## Safety Note
///
/// Configuration tables must remain valid for the lifetime of the system.
/// The data should be allocated in a persistent memory region.
///
/// ## Example Usage
///
/// ```ignore
/// use patina::component::IntoComponent;
/// use patina_samples::component::misc_usage::ConfigurationTableExample;
///
/// // Register the component with the core
/// dxe_core.with_component(ConfigurationTableExample);
/// ```
#[derive(IntoComponent)]
pub struct ConfigurationTableExample;

impl ConfigurationTableExample {
    fn entry_point(self, config: Service<dyn ConfigurationServices>) -> Result<()> {
        // Define a custom GUID for the configuration table
        let custom_guid =
            BinaryGuid::from_fields(0x12345678, 0x1234, 0x5678, 0x12, 0x34, &[0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0]);

        // For this example, we'll just use a static value for the table data
        let table_data = 0x12345678u32;

        // Install the configuration table (unsafe - table data must remain valid)
        unsafe {
            config.install_configuration_table_unchecked(&custom_guid, &table_data as *const u32 as *mut c_void)?;
        }

        // The table is now accessible via the system table

        Ok(())
    }
}

/// Removing a configuration table.
///
/// This component demonstrates how to remove a previously installed
/// configuration table,
///
/// ## Example Usage
///
/// ```ignore
/// use patina::component::IntoComponent;
/// use patina_samples::component::misc_usage::RemoveConfigurationTableExample;
///
/// // Register the component with the core
/// dxe_core.with_component(RemoveConfigurationTableExample);
/// ```
#[derive(IntoComponent)]
pub struct RemoveConfigurationTableExample;

impl RemoveConfigurationTableExample {
    fn entry_point(self, config: Service<dyn ConfigurationServices>) -> Result<()> {
        // Define the GUID of the table to remove
        let custom_guid =
            BinaryGuid::from_fields(0x12345678, 0x1234, 0x5678, 0x12, 0x34, &[0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0]);

        // Remove the configuration table
        config.remove_configuration_table(&custom_guid)?;

        Ok(())
    }
}
