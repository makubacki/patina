//! UEFI Specification Variable Services Implementation.
//!
//! The UEFI variable interface is an area that could benefit from an entirely new (and Rusty) interface. This
//! service is not intended to deviate from UEFI spec-like interfaces. They are not opinionated but rather to provide a
//! clean, object-safe interface for UEFI variable operations. An entirely new interface could be considered in the
//! future that enforces certain constraints in the interface such as UEFI variable policy that would be under a
//! different type than `UefiSpecVariableServices`.
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0

use crate::service::variable::UefiSpecVariableServices;
use alloc::{boxed::Box, string::String, vec, vec::Vec};
use core::ffi::c_void;
use patina::{
    BinaryGuid,
    boot_services::{BootServices, StandardBootServices, event::EventType, tpl::Tpl},
    component::{IntoComponent, params::Commands},
    error::{EfiError, Result},
    runtime_services::{RuntimeServices, StandardRuntimeServices, variable_services::VariableInfo},
};
use patina_macro::IntoService;
use r_efi::efi;
use spin::Mutex;

/// Standard implementation of `UefiSpecVariableServices` that delegates to `StandardRuntimeServices`.
///
/// This struct serves as an adapter between the high-level variable service interface
/// and the underlying UEFI runtime services, handling string conversions and error mapping.
#[derive(IntoService)]
#[service(dyn UefiSpecVariableServices)]
pub struct StandardVariableServices {
    runtime_services: StandardRuntimeServices,
}

impl StandardVariableServices {
    /// Creates a new StandardVariableServices instance.
    ///
    /// # Arguments
    ///
    /// * `runtime_services` - The underlying runtime services implementation
    pub fn new(runtime_services: StandardRuntimeServices) -> Self {
        Self { runtime_services }
    }
}

impl UefiSpecVariableServices for StandardVariableServices {
    fn get_variable(&self, variable_name: &str, vendor_guid: &BinaryGuid) -> Result<(Vec<u8>, u32)> {
        // Convert String to null-terminated UTF-16
        let name_utf16: Vec<u16> = variable_name.encode_utf16().chain([0]).collect();

        match self.runtime_services.get_variable::<Vec<u8>>(&name_utf16, vendor_guid, None) {
            Ok((data, attributes)) => Ok((data, attributes)),
            Err(status) => Err(EfiError::from(status)),
        }
    }

    fn set_variable(&self, variable_name: &str, vendor_guid: &BinaryGuid, attributes: u32, data: &[u8]) -> Result<()> {
        // Convert String to null-terminated UTF-16
        let name_utf16: Vec<u16> = variable_name.encode_utf16().chain([0]).collect();

        // Convert slice to Vec to satisfy Sized requirement
        let data_vec = data.to_vec();

        self.runtime_services.set_variable(&name_utf16, vendor_guid, attributes, &data_vec).map_err(EfiError::from)
    }

    fn get_next_variable_name(&self, variable_name: &mut String, vendor_guid: &mut BinaryGuid) -> Result<()> {
        // Convert current name to UTF-16
        let current_name_utf16: Vec<u16> = if variable_name.is_empty() {
            vec![0] // Start with an empty null-terminated string
        } else {
            variable_name.encode_utf16().chain([0]).collect()
        };

        let mut next_name_utf16 = Vec::new();

        let current_vendor_guid = **vendor_guid;
        let mut next_vendor_guid = current_vendor_guid;

        // Get next variable name
        unsafe {
            self.runtime_services
                .get_next_variable_name_unchecked(
                    &current_name_utf16,
                    &current_vendor_guid,
                    &mut next_name_utf16,
                    &mut next_vendor_guid,
                )
                .map_err(EfiError::from)?;
        }

        // Convert UTF-16 back to String (remove null terminator)
        if let Some(null_pos) = next_name_utf16.iter().position(|&c| c == 0) {
            next_name_utf16.truncate(null_pos);
        }

        *variable_name = String::from_utf16(&next_name_utf16).map_err(|_| EfiError::InvalidParameter)?;
        *vendor_guid = BinaryGuid::from(next_vendor_guid);

        Ok(())
    }

    fn query_variable_info(&self, attributes: u32) -> Result<(u64, u64, u64)> {
        match self.runtime_services.query_variable_info(attributes) {
            Ok(VariableInfo {
                maximum_variable_storage_size,
                remaining_variable_storage_size,
                maximum_variable_size,
            }) => Ok((maximum_variable_storage_size, remaining_variable_storage_size, maximum_variable_size)),
            Err(status) => Err(EfiError::from(status)),
        }
    }
}

/// Component that produces `UefiSpecVariableServices` for other components.
///
/// This component uses protocol notifications to ensure that variable services are only registered
/// after both the Variable Architectural Protocol and Variable Write Architectural Protocol are installed.
#[derive(IntoComponent)]
pub struct VariableServicesProvider;

/// Context passed to protocol notify callback
struct NotifyContext<'a> {
    commands: Mutex<Commands<'a>>,
    runtime_services: StandardRuntimeServices,
    boot_services: StandardBootServices,
    service_registered: Mutex<bool>,
}

impl VariableServicesProvider {
    /// Entry point for the `VariableServicesProvider` component.
    ///
    /// Sets up protocol notifications for the Variable Architectural Protocol and Variable Write
    /// Architectural Protocol. The variable services are only registered once both protocols are installed.
    pub fn entry_point(
        self,
        commands: Commands,
        runtime_services: StandardRuntimeServices,
        boot_services: StandardBootServices,
    ) -> Result<()> {
        // Variable Architectural Protocol GUID
        const VARIABLE_ARCH_PROTOCOL_GUID: efi::Guid =
            efi::Guid::from_fields(0x1e5668e2, 0x8481, 0x11d4, 0xbc, 0xf1, &[0x00, 0x80, 0xc7, 0x3c, 0x88, 0x81]);

        // Variable Write Architectural Protocol GUID
        const VARIABLE_WRITE_ARCH_PROTOCOL_GUID: efi::Guid =
            efi::Guid::from_fields(0x6441f818, 0x6362, 0x4e44, 0xb5, 0x70, &[0x7d, 0xba, 0x31, 0xdd, 0x24, 0x53]);

        // Create shared context that will be passed to both callbacks
        let context = Box::into_raw(Box::new(NotifyContext {
            commands: Mutex::new(commands),
            runtime_services,
            boot_services: boot_services.clone(),
            service_registered: Mutex::new(false),
        }));

        // Create a single event that will be used for both protocol notifications
        let event = boot_services.create_event(
            EventType::NOTIFY_SIGNAL,
            Tpl::NOTIFY,
            Some(Self::protocol_notify_callback),
            context as *mut c_void,
        )?;

        // Register the same event for both protocols
        // The callback will check if both are available each time it's triggered
        boot_services.register_protocol_notify(&VARIABLE_ARCH_PROTOCOL_GUID, event)?;

        boot_services.register_protocol_notify(&VARIABLE_WRITE_ARCH_PROTOCOL_GUID, event)?;

        Ok(())
    }

    /// Callback triggered when either protocol is installed.
    /// Checks if both protocols are now available and registers the service if so.
    extern "efiapi" fn protocol_notify_callback(_event: efi::Event, context: *mut c_void) {
        if context.is_null() {
            return;
        }

        // SAFETY: We control the context pointer lifetime
        let ctx = unsafe { &*(context as *const NotifyContext) };

        // Check if we've already registered the service
        if *ctx.service_registered.lock() {
            return;
        }

        // Variable Architectural Protocol GUID
        const VARIABLE_ARCH_PROTOCOL_GUID: efi::Guid =
            efi::Guid::from_fields(0x1e5668e2, 0x8481, 0x11d4, 0xbc, 0xf1, &[0x00, 0x80, 0xc7, 0x3c, 0x88, 0x81]);

        // Variable Write Architectural Protocol GUID
        const VARIABLE_WRITE_ARCH_PROTOCOL_GUID: efi::Guid =
            efi::Guid::from_fields(0x6441f818, 0x6362, 0x4e44, 0xb5, 0x70, &[0x7d, 0xba, 0x31, 0xdd, 0x24, 0x53]);

        // Check if both protocols are now available
        let var_arch_available = ctx.boot_services.locate_protocol_marker(&VARIABLE_ARCH_PROTOCOL_GUID, None).is_ok();
        let var_write_arch_available =
            ctx.boot_services.locate_protocol_marker(&VARIABLE_WRITE_ARCH_PROTOCOL_GUID, None).is_ok();

        if var_arch_available && var_write_arch_available {
            // Both protocols available - register the service
            let variable_services = StandardVariableServices::new(ctx.runtime_services.clone());

            ctx.commands.lock().add_service(variable_services);
            *ctx.service_registered.lock() = true;
        }
    }
}

#[cfg(all(test, feature = "mockall"))]
mod tests {
    use super::*;
    use patina::runtime_services::MockRuntimeServices;
    use r_efi::efi;

    // Test constants
    const TEST_VARIABLE_ATTRIBUTES: u32 = 0x07;
    const TEST_VARIABLE_DATA: &[u8] = &[0x41, 0x42, 0x43];

    /// Test-only wrapper for variable services that uses MockRuntimeServices
    /// This enables testing without trying to use trait objects (which don't work with generic methods)
    struct TestVariableServices {
        runtime_services: MockRuntimeServices,
    }

    impl TestVariableServices {
        fn new(runtime_services: MockRuntimeServices) -> Self {
            Self { runtime_services }
        }

        fn get_variable(&self, variable_name: &str, vendor_guid: &efi::Guid) -> Result<(Vec<u8>, u32)> {
            let name_utf16: Vec<u16> = variable_name.encode_utf16().chain(Some(0)).collect();
            match self.runtime_services.get_variable::<Vec<u8>>(&name_utf16, vendor_guid, None) {
                Ok((data, attributes)) => Ok((data, attributes)),
                Err(e) => Err(EfiError::from(e)),
            }
        }

        fn set_variable(
            &self,
            variable_name: &str,
            vendor_guid: &efi::Guid,
            attributes: u32,
            data: &[u8],
        ) -> Result<()> {
            let name_utf16: Vec<u16> = variable_name.encode_utf16().chain(Some(0)).collect();
            let data_vec = data.to_vec();
            self.runtime_services.set_variable(&name_utf16, vendor_guid, attributes, &data_vec).map_err(EfiError::from)
        }

        fn get_next_variable_name(
            &self,
            variable_name: &mut alloc::string::String,
            vendor_guid: &mut efi::Guid,
        ) -> Result<()> {
            let current_name_utf16: Vec<u16> = variable_name.encode_utf16().chain(Some(0)).collect();
            match self.runtime_services.get_next_variable_name(&current_name_utf16, vendor_guid) {
                Ok((mut next_name, next_guid)) => {
                    // Remove null terminator from returned name
                    if let Some(null_pos) = next_name.iter().position(|&c| c == 0) {
                        next_name.truncate(null_pos);
                    }
                    *variable_name = alloc::string::String::from_utf16_lossy(&next_name);
                    *vendor_guid = next_guid;
                    Ok(())
                }
                Err(e) => Err(EfiError::from(e)),
            }
        }

        fn query_variable_info(&self, attributes: u32) -> Result<(u64, u64, u64)> {
            match self.runtime_services.query_variable_info(attributes) {
                Ok(info) => Ok((
                    info.maximum_variable_storage_size,
                    info.remaining_variable_storage_size,
                    info.maximum_variable_size,
                )),
                Err(e) => Err(EfiError::from(e)),
            }
        }
    }

    #[test]
    fn test_get_variable_not_found() {
        let mut mock_runtime_services = MockRuntimeServices::new();

        // Set up expectation for get_variable with UTF-16 encoded "NonExistent"
        mock_runtime_services
            .expect_get_variable::<Vec<u8>>()
            .once()
            .withf(|name, namespace, size_hint| {
                let expected_name: Vec<u16> = "NonExistent".encode_utf16().chain([0]).collect();
                assert_eq!(name, expected_name.as_slice());
                let expected_guid = efi::Guid::from_fields(0, 0, 0, 0, 0, &[0; 6]);
                assert_eq!(namespace, &expected_guid);
                assert_eq!(size_hint, &None);
                true
            })
            .returning(|_, _, _| Err(r_efi::efi::Status::NOT_FOUND));

        let variable_services = TestVariableServices::new(mock_runtime_services);

        let result = variable_services.get_variable("NonExistent", &efi::Guid::from_fields(0, 0, 0, 0, 0, &[0; 6]));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), EfiError::NotFound);
    }

    #[test]
    fn test_get_variable_success() {
        let mut mock_runtime_services = MockRuntimeServices::new();

        // Set up expectation for successful get_variable
        let test_data = TEST_VARIABLE_DATA.to_vec();
        mock_runtime_services
            .expect_get_variable::<Vec<u8>>()
            .once()
            .withf(|name, namespace, size_hint| {
                let expected_name: Vec<u16> = "TestVar".encode_utf16().chain([0]).collect();
                assert_eq!(name, expected_name.as_slice());
                let expected_guid = efi::Guid::from_fields(0, 0, 0, 0, 0, &[0; 6]);
                assert_eq!(namespace, &expected_guid);
                assert_eq!(size_hint, &None);
                true
            })
            .returning(move |_, _, _| Ok((test_data.clone(), TEST_VARIABLE_ATTRIBUTES)));

        let variable_services = TestVariableServices::new(mock_runtime_services);

        let result = variable_services.get_variable("TestVar", &efi::Guid::from_fields(0, 0, 0, 0, 0, &[0; 6]));
        assert!(result.is_ok());
        let (data, attributes) = result.unwrap();
        assert_eq!(data.as_slice(), TEST_VARIABLE_DATA);
        assert_eq!(attributes, TEST_VARIABLE_ATTRIBUTES);
    }

    #[test]
    fn test_set_variable_success() {
        let mut mock_runtime_services = MockRuntimeServices::new();

        // Set up expectation for successful set_variable
        mock_runtime_services
            .expect_set_variable::<Vec<u8>>()
            .once()
            .withf(|name, namespace, attributes, data| {
                let expected_name: Vec<u16> = "TestVar".encode_utf16().chain([0]).collect();
                assert_eq!(name, expected_name.as_slice());
                let expected_guid = efi::Guid::from_fields(0, 0, 0, 0, 0, &[0; 6]);
                assert_eq!(namespace, &expected_guid);
                assert_eq!(attributes, &TEST_VARIABLE_ATTRIBUTES);
                assert_eq!(data.as_slice(), TEST_VARIABLE_DATA);
                true
            })
            .returning(|_, _, _, _| Ok(()));

        let variable_services = TestVariableServices::new(mock_runtime_services);

        let result = variable_services.set_variable(
            "TestVar",
            &efi::Guid::from_fields(0, 0, 0, 0, 0, &[0; 6]),
            TEST_VARIABLE_ATTRIBUTES,
            TEST_VARIABLE_DATA,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_next_variable_name_success() {
        use alloc::string::String;

        let mut mock_runtime_services = MockRuntimeServices::new();

        // Set up expectation for successful get_next_variable_name
        let next_name_utf16: Vec<u16> = "NextVar".encode_utf16().chain([0]).collect();
        let next_guid = efi::Guid::from_fields(1, 1, 1, 1, 1, &[1; 6]);

        mock_runtime_services
            .expect_get_next_variable_name()
            .once()
            .withf(|prev_name, prev_namespace| {
                let expected_prev_name: Vec<u16> = vec![0]; // Just null terminator for first call
                assert_eq!(prev_name, expected_prev_name.as_slice());
                let expected_guid = efi::Guid::from_fields(0, 0, 0, 0, 0, &[0; 6]);
                assert_eq!(prev_namespace, &expected_guid);
                true
            })
            .returning(move |_, _| Ok((next_name_utf16.clone(), next_guid)));

        let variable_services = TestVariableServices::new(mock_runtime_services);

        let mut variable_name = String::new(); // Start with empty name
        let mut vendor_guid = efi::Guid::from_fields(0, 0, 0, 0, 0, &[0; 6]);

        let result = variable_services.get_next_variable_name(&mut variable_name, &mut vendor_guid);
        assert!(result.is_ok());
        assert_eq!(variable_name, "NextVar");
        assert_eq!(vendor_guid, next_guid);
    }

    #[test]
    fn test_query_variable_info_success() {
        let mut mock_runtime_services = MockRuntimeServices::new();

        const MAX_STORAGE: u64 = 1024 * 1024;
        const REMAINING_STORAGE: u64 = 512 * 1024;
        const MAX_VAR_SIZE: u64 = 64 * 1024;

        mock_runtime_services
            .expect_query_variable_info()
            .once()
            .withf(|attributes| {
                assert_eq!(attributes, &TEST_VARIABLE_ATTRIBUTES);
                true
            })
            .returning(|_| {
                Ok(VariableInfo {
                    maximum_variable_storage_size: MAX_STORAGE,
                    remaining_variable_storage_size: REMAINING_STORAGE,
                    maximum_variable_size: MAX_VAR_SIZE,
                })
            });

        let variable_services = TestVariableServices::new(mock_runtime_services);

        let result = variable_services.query_variable_info(TEST_VARIABLE_ATTRIBUTES);
        assert!(result.is_ok());
        let (max_storage, remaining_storage, max_var_size) = result.unwrap();
        assert_eq!(max_storage, MAX_STORAGE);
        assert_eq!(remaining_storage, REMAINING_STORAGE);
        assert_eq!(max_var_size, MAX_VAR_SIZE);
    }

    #[test]
    fn test_variable_services_provider_creation() {
        let _provider = VariableServicesProvider;
    }
}
