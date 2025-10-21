//! Runtime Services Abstraction
//!
//! While UEFI Runtime Services are, by definition, persistent after ExitBootServices(), a given implementation
//! of this trait may not be. Since this is only intended to be used by Patina components and Patina components are
//! not supported at runtime at this time, this should not be an issue for component writers. However, if runtime
//! components are supported, the runtime services component producing this service must persist through runtime.
//!
//! At this time, it is expected that Runtime Services will ultimately map to C drivers through an FFI, and these
//! services simply provide safe wrapper for Patina components directly to those services.
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
//!

use alloc::{string::String, vec::Vec};
use patina::BinaryGuid;
use patina::error::Result;
use r_efi::efi;

#[cfg(any(test, feature = "mockall"))]
use mockall::automock;

/// Variable storage and retrieval operations.
#[cfg_attr(any(test, feature = "mockall"), automock)]
pub trait RuntimeVariableServices {
    /// Gets the value of a variable.
    ///
    /// # Arguments
    ///
    /// * `variable_name` - Name of the variable to retrieve
    /// * `vendor_guid` - GUID identifying the variable namespace
    ///
    /// # Returns
    ///
    /// Variable data as a `Vec<u8>`
    fn get_variable(&self, variable_name: &str, vendor_guid: &BinaryGuid) -> Result<Vec<u8>>;

    /// Sets the value of a variable.
    ///
    /// # Arguments
    ///
    /// * `variable_name` - Name of the variable to set
    /// * `vendor_guid` - GUID identifying the variable namespace
    /// * `attributes` - Variable attributes (boot service, runtime, etc.)
    /// * `data` - Data to store in the variable
    fn set_variable(&self, variable_name: &str, vendor_guid: &BinaryGuid, attributes: u32, data: &[u8]) -> Result<()>;

    /// Enumerates the current variable names.
    ///
    /// # Arguments
    ///
    /// * `variable_name` - Current variable name (modified on return)
    /// * `vendor_guid` - Current vendor GUID (modified on return)
    fn get_next_variable_name(&self, variable_name: &mut String, vendor_guid: &mut BinaryGuid) -> Result<()>;

    /// Returns information about the EFI variables.
    ///
    /// # Arguments
    ///
    /// * `attributes` - Variable attributes to query
    ///
    /// # Returns
    ///
    /// Tuple of (MaximumVariableStorageSize, RemainingVariableStorageSize, MaximumVariableSize)
    fn query_variable_info(&self, attributes: u32) -> Result<(u64, u64, u64)>;
}

/// Time and date management operations.
#[cfg_attr(any(test, feature = "mockall"), automock)]
pub trait RuntimeTimeServices {
    /// Returns the current time and date information.
    fn get_time(&self) -> Result<efi::Time>;

    /// Sets the current local time and date information.
    ///
    /// # Arguments
    ///
    /// * `time` - Time structure to set
    fn set_time(&self, time: &efi::Time) -> Result<()>;

    /// Returns the current wakeup alarm clock setting.
    ///
    /// # Returns
    ///
    /// Tuple of (Enabled, Pending, Time)
    fn get_wakeup_time(&self) -> Result<(bool, bool, efi::Time)>;

    /// Sets the system wakeup alarm clock time.
    ///
    /// # Arguments
    ///
    /// * `enable` - Whether to enable the wakeup alarm
    /// * `time` - Optional time to set for the alarm (efi::Time is Copy, so passed by value)
    fn set_wakeup_time(&self, enable: bool, time: Option<efi::Time>) -> Result<()>;
}

/// System reset operations.
#[cfg_attr(any(test, feature = "mockall"), automock)]
pub trait RuntimeResetServices {
    /// Resets the entire platform.
    ///
    /// Note: This function never returns on success (system resets).
    /// In test/mock environments, returns Ok(()) to allow testing.
    ///
    /// # Arguments
    ///
    /// * `reset_type` - Type of reset to perform
    /// * `reset_status` - Status code for the reset
    /// * `data` - Optional additional reset data (Vec for mockall compatibility)
    #[cfg(not(any(test, feature = "mockall")))]
    fn reset_system(&self, reset_type: efi::ResetType, reset_status: efi::Status, data: Option<&[u8]>) -> !;

    /// Resets the entire platform (mock version for testing).
    ///
    /// This is the test/mock version that returns Result instead of never type
    /// and uses Vec<u8> instead of &[u8] to avoid lifetime issues with mockall.
    ///
    /// # Arguments
    ///
    /// * `reset_type` - Type of reset to perform
    /// * `reset_status` - Status code for the reset
    /// * `data` - Optional additional reset data
    #[cfg(any(test, feature = "mockall"))]
    fn reset_system(
        &self,
        reset_type: efi::ResetType,
        reset_status: efi::Status,
        data: Option<alloc::vec::Vec<u8>>,
    ) -> Result<()>;
}
