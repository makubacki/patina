//! Miscellaneous Services Abstractions
//!
//! This module provides trait definitions for various UEFI utility functions.
//!
//! These services are broken into cohesive functional areas to allow Patina Components to declare specific
//! dependencies so what they actually depend on is more apparent in their dependency-injected parameter list.
//!
//! ## Service Groups
//!
//! - [`TimingServices`] - Watchdog timer and delay operations
//! - [`MemoryUtilityServices`] - Memory copying and filling operations
//! - [`SystemUtilityServices`] - System counters and data integrity calculations
//! - [`ConfigurationServices`] - System configuration table management
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0

use core::ffi::c_void;
use patina::{BinaryGuid, error::Result};

#[cfg(any(test, feature = "mockall"))]
use mockall::automock;

/// Timing-related UEFI services for watchdog timers and delays.
///
/// This service group provides timing control operations including watchdog timer management and precise delay
/// functions.
///
/// ## Design Notes
///
/// This trait is intended to be object-safe to support dynamic dispatch.
///
/// ## Example Usage
///
/// ```rust
/// use patina::component::{IntoComponent, prelude::Service};
/// use patina_uefi_services::service::misc::TimingServices;
/// use patina::error::Result;
///
/// #[derive(IntoComponent)]
/// struct MyTimingComponent;
///
/// impl MyTimingComponent {
///     fn entry_point(self, timing: Service<dyn TimingServices>) -> Result<()> {
///         // Set a 5-second watchdog timer
///         timing.set_watchdog_timer(5)?;
///         // Delay for 1000 microseconds
///         timing.stall(1000)?;
///         Ok(())
///     }
/// }
/// ```
#[cfg_attr(any(test, feature = "mockall"), automock)]
pub trait TimingServices {
    /// Sets the system's watchdog timer.
    ///
    /// The watchdog timer provides a mechanism to reset the system if it becomes unresponsive. Setting timeout to 0
    /// disables the watchdog timer.
    ///
    /// # Parameters
    /// - `timeout` - Timeout value in seconds (0 disables the timer)
    ///
    /// # Returns
    /// - `Ok(())` on success
    /// - `Err(status)` on failure
    fn set_watchdog_timer(&self, timeout: usize) -> Result<()>;

    /// Induces a fine-grained delay.
    ///
    /// This function stalls the processor for at least the specified number of microseconds. This is a blocking
    /// operation.
    ///
    /// # Parameters
    /// - `microseconds` - Number of microseconds to delay
    ///
    /// # Returns
    /// - `Ok(())` on success
    /// - `Err(status)` on failure
    fn stall(&self, microseconds: usize) -> Result<()>;
}

/// Memory utility services for copying and filling memory.
///
/// This service group provides memory manipulation utilities including safe memory copying and buffer filling
/// operations.
///
/// ## Design Notes
///
/// This trait is object-safe to support dynamic dispatch through `Service<dyn MemoryUtilityServices>`.
///
/// ## Usage Notes
///
/// This service provides very basic memory operations. It is made available to simply provide a safe wrapper to the
/// equivalent function in the boot services table if that specific need arises in a Patina component. If that is not
/// the case, use native Rust mechanisms in place of these functions in Patina components.
///
/// ## Example Usage
///
/// ```rust
/// use patina::component::{IntoComponent, prelude::Service};
/// use patina_uefi_services::component::misc::StandardMemoryUtilityServices;
/// use patina_uefi_services::service::misc::MemoryUtilityServices;
/// use patina::error::Result;
///
/// #[derive(IntoComponent)]
/// struct MyMemoryComponent;
///
/// impl MyMemoryComponent {
///     fn entry_point(self, mem_utils: Service<StandardMemoryUtilityServices>) -> Result<()> {
///         let src = 42u32;
///         let mut dest = 0u32;
///         // Copy memory safely
///         mem_utils.copy_mem(&mut dest, &src);
///         // Fill a buffer with zeros
///         let mut buffer = [0xFFu8; 16];
///         mem_utils.set_mem(&mut buffer, 0);
///         Ok(())
///     }
/// }
/// ```
#[cfg_attr(any(test, feature = "mockall"), automock)]
pub trait MemoryUtilityServices {
    /// Copies the contents of one buffer to another buffer.
    ///
    /// This is a safe memory copy operation that ensures type safety and
    /// proper size calculations.
    ///
    /// # Parameters
    /// - `dest` - Destination buffer (mutable reference)
    /// - `src` - Source buffer (immutable reference)
    fn copy_mem<T: 'static>(&self, dest: &mut T, src: &T);

    /// Copies memory from source to destination (unsafe version).
    ///
    /// This provides direct access to the underlying unsafe memory copy
    /// operation when needed for performance or specific use cases.
    ///
    /// # Safety
    /// - `dest` and `src` must be valid pointers to continuous memory
    /// - `length` must not exceed the size of either buffer
    /// - Buffers must not overlap (undefined behavior)
    ///
    /// # Parameters
    /// - `dest` - Destination pointer
    /// - `src` - Source pointer
    /// - `length` - Number of bytes to copy
    unsafe fn copy_mem_unchecked(&self, dest: *mut c_void, src: *const c_void, length: usize);

    /// Fills a buffer with a specified value.
    ///
    /// This function sets all bytes in the buffer to the specified value,
    /// similar to the C library `memset` function.
    ///
    /// # Parameters
    /// - `buffer` - Buffer to fill (mutable slice)
    /// - `value` - Byte value to fill the buffer with
    fn set_mem(&self, buffer: &mut [u8], value: u8);
}

/// System utility services for counters and data integrity.
///
/// This service group provides system-level utilities including monotonic counters and CRC32 calculations.
///
/// ## Usage Notes
///
/// This service provides very basic memory operations. It is made available to simply provide a safe wrapper to the
/// equivalent function in the boot services table if that specific need arises in a Patina component. If that is not
/// the case, use native Rust mechanisms in place of these functions in Patina components.
///
/// ## Example Usage
///
/// ```rust
/// use patina::component::{IntoComponent, prelude::Service};
/// use patina_uefi_services::component::misc::StandardSystemUtilityServices;
/// use patina_uefi_services::service::misc::SystemUtilityServices;
/// use patina::error::Result;
///
/// #[derive(IntoComponent)]
/// struct MySystemComponent;
///
/// impl MySystemComponent {
///     fn entry_point(self, sys_utils: Service<StandardSystemUtilityServices>) -> Result<()> {
///         // Get a unique counter value
///         let count = sys_utils.get_next_monotonic_count()?;
///         // Calculate CRC32 of some data
///         let data = 0x12345678u32;
///         let crc = sys_utils.calculate_crc_32(&data)?;
///         Ok(())
///     }
/// }
/// ```
#[cfg_attr(any(test, feature = "mockall"), automock)]
pub trait SystemUtilityServices {
    /// Returns a monotonically increasing count for the platform.
    ///
    /// This function returns a 64-bit value that is guaranteed to increase
    /// monotonically during the current boot. This can be used for unique
    /// identifiers or sequence numbers.
    ///
    /// # Returns
    /// - `Ok(count)` - The next monotonic count value
    /// - `Err(status)` on failure
    fn get_next_monotonic_count(&self) -> Result<u64>;

    /// Computes and returns a 32-bit CRC for data.
    ///
    /// This function calculates a CRC32 checksum for the provided data,
    /// which can be used for data integrity verification.
    ///
    /// # Parameters
    /// - `data` - Data to calculate CRC32 for
    ///
    /// # Returns
    /// - `Ok(crc32)` - The calculated CRC32 value
    /// - `Err(status)` on failure
    fn calculate_crc_32<T: 'static>(&self, data: &T) -> Result<u32>;

    /// Computes CRC32 for raw data (unsafe version).
    ///
    /// This provides direct access to the underlying unsafe CRC32 calculation
    /// when needed for performance or specific use cases.
    ///
    /// # Safety
    /// - `data` must be a valid pointer to continuous memory
    /// - `data_size` must not exceed the actual size of the data
    ///
    /// # Parameters
    /// - `data` - Pointer to data
    /// - `data_size` - Size of data in bytes
    ///
    /// # Returns
    /// - `Ok(crc32)` - The calculated CRC32 value
    /// - `Err(status)` on failure
    unsafe fn calculate_crc_32_unchecked(&self, data: *const c_void, data_size: usize) -> Result<u32>;
}

/// Configuration services for system configuration table management.
///
/// This service group provides configuration table management operations.
///
/// ## Example Usage
///
/// ```rust
/// use patina::component::{IntoComponent, prelude::Service};
/// use patina_uefi_services::service::misc::ConfigurationServices;
/// use patina::error::Result;
/// use patina::BinaryGuid;
///
/// #[derive(IntoComponent)]
/// struct MyConfigComponent;
///
/// impl MyConfigComponent {
///     fn entry_point(self, config: Service<dyn ConfigurationServices>) -> Result<()> {
///         let guid = BinaryGuid::from_fields(0x12345678, 0x1234, 0x5678, 0x12, 0x34, &[0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0]);
///         let table_data = 0x12345678u32;
///         // Install a configuration table (unsafe example)
///         unsafe {
///             config.install_configuration_table_unchecked(
///                 &guid,
///                 &table_data as *const u32 as *mut core::ffi::c_void
///             )?;
///         }
///         Ok(())
///     }
/// }
/// ```
#[cfg_attr(any(test, feature = "mockall"), automock)]
pub trait ConfigurationServices {
    /// Removes a configuration table entry from the EFI System Table.
    ///
    /// # Parameters
    /// - `guid` - GUID that identifies the configuration table to remove
    ///
    /// # Returns
    /// - `Ok(())` on success
    /// - `Err(status)` on failure (e.g., if the table doesn't exist)
    fn remove_configuration_table(&self, guid: &BinaryGuid) -> Result<()>;

    /// Adds, updates, or removes a configuration table entry from the EFI System Table (unsafe).
    ///
    /// This is the low-level unsafe interface. For type-safe installation of configuration
    /// tables, consider using the `StandardConfigurationServices` concrete type directly
    /// which provides a safe `install_configuration_table` method.
    ///
    /// Configuration tables provide a way to pass data structures between
    /// different phases of the boot process or to the operating system.
    ///
    /// # Safety
    /// - The table pointer must match the expected type for the GUID
    /// - The table data must remain valid for the lifetime of the system
    /// - Passing a null pointer removes the configuration table entry
    ///
    /// # Parameters
    /// - `guid` - GUID that identifies the configuration table
    /// - `table` - Pointer to the configuration table data
    ///
    /// # Returns
    /// - `Ok(())` on success
    /// - `Err(status)` on failure
    unsafe fn install_configuration_table_unchecked(&self, guid: &BinaryGuid, table: *mut c_void) -> Result<()>;
}
