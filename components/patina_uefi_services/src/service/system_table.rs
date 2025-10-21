//! System Table Service Abstraction
//!
//! Provides access to UEFI System Table exposed functionality.
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0

use patina::error::Result;
use r_efi::efi;

#[cfg(any(test, feature = "mockall"))]
use mockall::automock;

/// System Table access service.
#[cfg_attr(any(test, feature = "mockall"), automock)]
pub trait SystemTableService {
    /// Gets the console input protocol.
    ///
    /// # Returns
    /// * `Result<&'static mut efi::protocols::simple_text_input::Protocol>` - Console input protocol
    ///
    /// # Errors
    /// * `EfiError::NotFound` - If console input protocol is not available
    /// * `EfiError::NotReady` - If system table is not initialized
    fn get_console_input(&self) -> Result<&'static mut efi::protocols::simple_text_input::Protocol>;

    /// Gets the console output protocol.
    ///
    /// # Returns
    /// * `Result<&'static mut efi::protocols::simple_text_output::Protocol>` - Console output protocol
    ///
    /// # Errors
    /// * `EfiError::NotFound` - If console output protocol is not available
    /// * `EfiError::NotReady` - If system table is not initialized
    fn get_console_output(&self) -> Result<&'static mut efi::protocols::simple_text_output::Protocol>;

    /// Gets the standard error output protocol.
    ///
    /// # Returns
    /// * `Result<&'static mut efi::protocols::simple_text_output::Protocol>` - Standard error output protocol
    ///
    /// # Errors
    /// * `EfiError::NotFound` - If standard error protocol is not available
    /// * `EfiError::NotReady` - If system table is not initialized
    fn get_standard_error(&self) -> Result<&'static mut efi::protocols::simple_text_output::Protocol>;
}
