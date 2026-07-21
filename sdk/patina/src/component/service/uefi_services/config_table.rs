//! Configuration table services for Patina components.
//!
//! [`ConfigurationTableServices`] exposes UEFI configuration table installation and lookup. The
//! trait is object-safe and works with the opaque [`ConfigTablePtr`] token. Type-safe access is
//! provided by [`ConfigurationTableServicesExt`].
//!
//! Configuration tables associate a GUID with a pointer to a vendor-defined table (for example
//! ACPI or SMBIOS tables).
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
//!

use core::ffi::c_void;
use core::ptr::NonNull;

use r_efi::efi;

use crate::base::error::EfiError;

#[cfg(any(test, feature = "mockall"))]
use mockall::automock;

/// An opaque pointer to a vendor configuration table.
///
/// This token is consumed by [`ConfigurationTableServices::install_table`] and returned by
/// [`ConfigurationTableServices::get_table`]. Components should generally use the typed methods on
/// [`ConfigurationTableServicesExt`] rather than handling this token directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigTablePtr(NonNull<c_void>);

impl ConfigTablePtr {
    /// Wraps a raw table pointer.
    ///
    /// This is intended for use by service implementations and the typed extension methods, not
    /// component authors.
    #[doc(hidden)]
    pub fn from_raw(table: *mut c_void) -> Option<Self> {
        NonNull::new(table).map(Self)
    }

    /// Returns the raw table pointer.
    ///
    /// This is intended for use by service implementations and the typed extension methods, not
    /// component authors.
    #[doc(hidden)]
    pub fn as_raw(&self) -> *mut c_void {
        self.0.as_ptr()
    }
}

/// Errors that can occur when using [`ConfigurationTableServices`].
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ConfigTableError {
    /// A provided parameter was invalid.
    InvalidParameter,
    /// The requested table (or the system table) was not found.
    NotFound,
    /// The system is out of resources to complete the operation.
    OutOfResources,
    /// An unexpected internal error occurred.
    Internal,
}

impl From<ConfigTableError> for EfiError {
    fn from(value: ConfigTableError) -> Self {
        match value {
            ConfigTableError::InvalidParameter => EfiError::InvalidParameter,
            ConfigTableError::NotFound => EfiError::NotFound,
            ConfigTableError::OutOfResources => EfiError::OutOfResources,
            ConfigTableError::Internal => EfiError::Unsupported,
        }
    }
}

impl From<EfiError> for ConfigTableError {
    fn from(value: EfiError) -> Self {
        match value {
            EfiError::InvalidParameter => ConfigTableError::InvalidParameter,
            EfiError::NotFound => ConfigTableError::NotFound,
            EfiError::OutOfResources => ConfigTableError::OutOfResources,
            _ => ConfigTableError::Internal,
        }
    }
}

/// Configuration table installation and lookup services.
///
/// This trait is object-safe and deals in opaque tokens. Component authors should generally use the
/// type-safe methods provided by [`ConfigurationTableServicesExt`].
///
/// This service is implemented by the Patina DXE Core. Components consume it by adding a
/// [`Service<dyn ConfigurationTableServices>`](crate::component::service::Service) parameter to
/// their entry point.
///
/// # Examples
///
/// ```rust,no_run
/// use patina::BinaryGuid;
/// use patina::component::service::{
///     Service,
///     uefi_services::config_table::{ConfigurationTableServices, ConfigurationTableServicesExt},
/// };
/// use patina::error::Result;
///
/// #[repr(C)]
/// struct VendorTable {
///     version: u32,
/// }
/// static TABLE: VendorTable = VendorTable { version: 1 };
/// const GUID: BinaryGuid = BinaryGuid::from_string("0fedcba9-8765-4321-fedc-ba9876543210");
///
/// fn entry_point(config: Service<dyn ConfigurationTableServices>) -> Result<()> {
///     // Publish the table so an OS or later component can find it by GUID.
///     config.install_configuration_table(GUID.into_inner(), &TABLE)?;
///     Ok(())
/// }
/// ```
#[cfg_attr(any(test, feature = "mockall"), automock)]
pub trait ConfigurationTableServices {
    /// Installs or replaces the configuration table associated with `guid`.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigTableError::NotFound`] if the system table is not available, or
    /// [`ConfigTableError::OutOfResources`] if the table could not be stored.
    fn install_table(&self, guid: efi::Guid, table: ConfigTablePtr) -> Result<(), ConfigTableError>;

    /// Removes the configuration table associated with `guid`.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigTableError::NotFound`] if no table is installed for `guid`.
    fn remove_table(&self, guid: efi::Guid) -> Result<(), ConfigTableError>;

    /// Returns the configuration table associated with `guid`, if present.
    fn get_table(&self, guid: efi::Guid) -> Option<ConfigTablePtr>;
}

/// Type-safe extension methods for [`ConfigurationTableServices`].
///
/// The generic methods let callers install and retrieve typed tables. Because configuration tables
/// have no GUID-to-type binding, [`Self::get_configuration_table`] is `unsafe`: the caller asserts
/// that the table installed under `guid` actually has type `T`.
pub trait ConfigurationTableServicesExt: ConfigurationTableServices {
    /// Installs `table` under `guid`.
    ///
    /// The table must live for the lifetime of the configuration table entry (`'static`).
    ///
    /// # Errors
    ///
    /// Returns [`ConfigTableError::InvalidParameter`] if the table could not be installed.
    fn install_configuration_table<T>(&self, guid: efi::Guid, table: &'static T) -> Result<(), ConfigTableError> {
        let ptr =
            ConfigTablePtr::from_raw(table as *const T as *mut c_void).ok_or(ConfigTableError::InvalidParameter)?;
        self.install_table(guid, ptr)
    }

    /// Returns a typed reference to the table installed under `guid`, if present.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the table installed under `guid` was created with type `T` (or a
    /// type with a compatible layout), and that it remains valid for `'static`.
    unsafe fn get_configuration_table<T>(&self, guid: efi::Guid) -> Option<&'static T> {
        let ptr = self.get_table(guid)?;
        // SAFETY: the caller guarantees the table under `guid` has type `T` and `'static` lifetime.
        Some(unsafe { &*(ptr.as_raw() as *const T) })
    }
}

impl<T: ConfigurationTableServices + ?Sized> ConfigurationTableServicesExt for T {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_configuration_table_services_error_conversions() {
        assert_eq!(EfiError::from(ConfigTableError::InvalidParameter), EfiError::InvalidParameter);
        assert_eq!(EfiError::from(ConfigTableError::NotFound), EfiError::NotFound);
        assert_eq!(EfiError::from(ConfigTableError::OutOfResources), EfiError::OutOfResources);
        assert_eq!(EfiError::from(ConfigTableError::Internal), EfiError::Unsupported);
        assert_eq!(ConfigTableError::from(EfiError::NotFound), ConfigTableError::NotFound);
        assert_eq!(ConfigTableError::from(EfiError::DeviceError), ConfigTableError::Internal);
    }

    #[test]
    fn test_configuration_table_services_ptr_round_trip() {
        assert!(ConfigTablePtr::from_raw(core::ptr::null_mut()).is_none());
    }

    #[test]
    fn test_configuration_table_services_ext_typed_flow() {
        #[repr(C)]
        struct FakeTable {
            value: u32,
        }
        static GUID: efi::Guid = efi::Guid::from_fields(0x1234_5678, 0x1234, 0x5678, 0x12, 0x34, &[0, 1, 2, 3, 4, 5]);
        static TABLE: FakeTable = FakeTable { value: 7 };

        let mut mock = MockConfigurationTableServices::new();
        mock.expect_install_table().times(1).returning(|guid, _| {
            assert_eq!(guid, GUID);
            Ok(())
        });
        mock.expect_get_table()
            .times(1)
            .returning(|_| ConfigTablePtr::from_raw(&TABLE as *const FakeTable as *mut c_void));

        mock.install_configuration_table(GUID, &TABLE).unwrap();
        // SAFETY: the mock returns a pointer to `TABLE`, which is a `FakeTable`.
        let table = unsafe { mock.get_configuration_table::<FakeTable>(GUID) }.unwrap();
        assert_eq!(table.value, 7);
    }
}
