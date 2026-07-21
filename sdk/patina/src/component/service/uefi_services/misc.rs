//! Miscellaneous UEFI utility services for Patina components.
//!
//! [`MiscServices`] exposes small, general-purpose UEFI boot service utilities that do not warrant
//! their own service group. Currently this is the CRC-32 calculation used for table checksums.
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
//!

#[cfg(any(test, feature = "mockall"))]
use mockall::automock;

/// Miscellaneous UEFI utility services.
///
/// This service is implemented by the Patina DXE Core. Components consume it by adding a
/// [`Service<dyn MiscServices>`](crate::component::service::Service) parameter to their entry
/// point.
///
/// # Examples
///
/// ```rust,no_run
/// use patina::component::service::{Service, uefi_services::misc::MiscServices};
/// use patina::error::Result;
///
/// fn entry_point(misc: Service<dyn MiscServices>) -> Result<()> {
///     let checksum = misc.calculate_crc32(b"hello");
///     log::info!("crc32 = {checksum:#x}");
///     Ok(())
/// }
/// ```
#[cfg_attr(any(test, feature = "mockall"), automock)]
pub trait MiscServices {
    /// Computes the CRC-32 of the given data, matching the UEFI `CalculateCrc32` boot service.
    fn calculate_crc32(&self, data: &[u8]) -> u32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_misc_services_mock_crc32() {
        let mut mock = MockMiscServices::new();
        mock.expect_calculate_crc32().times(1).returning(|data| data.len() as u32);
        assert_eq!(mock.calculate_crc32(b"hello"), 5);
    }
}
