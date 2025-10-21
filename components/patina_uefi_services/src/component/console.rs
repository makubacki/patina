//! Console Services Component Implementation
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0

use crate::service::{console::ConsoleServices, system_table::SystemTableService};
use alloc::{boxed::Box, vec::Vec};
use core::ffi::c_void;
use patina::{
    boot_services::{BootServices, StandardBootServices, event::EventType, tpl::Tpl},
    component::{IntoComponent, params::Commands, prelude::Service, service::IntoService},
    error::{EfiError, Result},
};
use r_efi::efi;
use spin::Mutex;

/// Standard implementation of console services using UEFI Boot Services.
///
/// This implementation provides UEFI console operations by delegating to the underlying `StandardBootServices` and
/// accessing the system table's console input and output protocols through the `SystemTableService`.
#[derive(IntoService)]
#[service(dyn ConsoleServices)]
pub struct StandardConsoleServices {
    #[allow(dead_code)]
    boot_services: StandardBootServices,
    system_table_service: Service<dyn SystemTableService>,
}

impl StandardConsoleServices {
    /// Creates a new `StandardConsoleServices` instance.
    ///
    /// # Arguments
    ///
    /// * `boot_services` - The underlying boot services implementation
    /// * `system_table_service` - Service providing access to the system table
    pub fn new(boot_services: StandardBootServices, system_table_service: Service<dyn SystemTableService>) -> Self {
        Self { boot_services, system_table_service }
    }
}

impl ConsoleServices for StandardConsoleServices {
    fn output_string(&self, text: &str) -> Result<()> {
        let con_out = self.system_table_service.get_console_output()?;

        // Convert Rust string to UTF-16
        let mut utf16_string: Vec<u16> = text.encode_utf16().collect();
        utf16_string.push(0); // Null terminator

        let status = (con_out.output_string)(con_out, utf16_string.as_ptr() as *mut efi::Char16);

        if status == efi::Status::SUCCESS { Ok(()) } else { Err(EfiError::from(status)) }
    }

    fn clear_screen(&self) -> Result<()> {
        let con_out = self.system_table_service.get_console_output()?;

        let status = (con_out.clear_screen)(con_out);

        if status == efi::Status::SUCCESS { Ok(()) } else { Err(EfiError::from(status)) }
    }

    fn set_cursor_position(&self, column: usize, row: usize) -> Result<()> {
        let con_out = self.system_table_service.get_console_output()?;

        let status = (con_out.set_cursor_position)(con_out, column, row);

        if status == efi::Status::SUCCESS { Ok(()) } else { Err(EfiError::from(status)) }
    }

    fn get_cursor_position(&self) -> Result<(usize, usize)> {
        let con_out = self.system_table_service.get_console_output()?;

        // Access the cursor position from the mode structure
        let mode = unsafe { con_out.mode.as_ref().ok_or(EfiError::DeviceError)? };

        Ok((mode.cursor_column as usize, mode.cursor_row as usize))
    }

    fn enable_cursor(&self, visible: bool) -> Result<()> {
        let con_out = self.system_table_service.get_console_output()?;

        let status = (con_out.enable_cursor)(con_out, if visible { efi::Boolean::TRUE } else { efi::Boolean::FALSE });

        if status == efi::Status::SUCCESS { Ok(()) } else { Err(EfiError::from(status)) }
    }

    fn read_key_stroke(&self) -> Result<u16> {
        let con_in = self.system_table_service.get_console_input()?;

        let mut input_key = efi::protocols::simple_text_input::InputKey { scan_code: 0, unicode_char: 0 };

        let status = (con_in.read_key_stroke)(con_in, &mut input_key);

        if status == efi::Status::SUCCESS { Ok(input_key.unicode_char) } else { Err(EfiError::from(status)) }
    }

    fn is_key_available(&self) -> Result<bool> {
        let con_in = self.system_table_service.get_console_input()?;

        // Use the WaitForEvent mechanism to check if a key is available
        let status = self.boot_services.check_event(con_in.wait_for_key);

        match status {
            Ok(_) => Ok(true),                        // Event is signaled, key is available
            Err(efi::Status::NOT_READY) => Ok(false), // Event is not signaled, no key available
            Err(e) => Err(EfiError::from(e)),         // Some other error occurred
        }
    }

    fn reset_input(&self, extended_verification: bool) -> Result<()> {
        let con_in = self.system_table_service.get_console_input()?;

        let status =
            (con_in.reset)(con_in, if extended_verification { efi::Boolean::TRUE } else { efi::Boolean::FALSE });

        if status == efi::Status::SUCCESS { Ok(()) } else { Err(EfiError::from(status)) }
    }

    fn query_mode(&self) -> Result<(usize, usize)> {
        let con_out = self.system_table_service.get_console_output()?;

        let mode = unsafe { con_out.mode.as_ref().ok_or(EfiError::DeviceError)? };

        let mut columns: usize = 0;
        let mut rows: usize = 0;

        let status = (con_out.query_mode)(con_out, mode.mode as usize, &mut columns, &mut rows);

        if status == efi::Status::SUCCESS { Ok((columns, rows)) } else { Err(EfiError::from(status)) }
    }
}

/// Component that provides `ConsoleServices` for other components.
///
/// This component uses protocol notifications to ensure that console services are only registered
/// after both `simple_text_input` and `simple_text_output` protocols are installed.
#[derive(IntoComponent)]
pub struct ConsoleServicesProvider;

/// Context passed to protocol notify callback
struct NotifyContext<'a> {
    commands: Mutex<Commands<'a>>,
    boot_services: StandardBootServices,
    system_table_service: Service<dyn SystemTableService>,
    service_registered: Mutex<bool>,
}

impl ConsoleServicesProvider {
    /// Entry point for the `ConsoleServicesProvider` component.
    ///
    /// Sets up protocol notifications for `simple_text_input` and `simple_text_output` protocols.
    /// The console services are only registered once both protocols are installed.
    pub fn entry_point(
        self,
        commands: Commands,
        boot_services: StandardBootServices,
        system_table_service: Service<dyn SystemTableService>,
    ) -> Result<()> {
        // Create shared context that will be passed to both callbacks
        let context = Box::into_raw(Box::new(NotifyContext {
            commands: Mutex::new(commands),
            boot_services: boot_services.clone(),
            system_table_service,
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
        boot_services.register_protocol_notify(&efi::protocols::simple_text_input::PROTOCOL_GUID, event)?;

        boot_services.register_protocol_notify(&efi::protocols::simple_text_output::PROTOCOL_GUID, event)?;

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

        // Check if both protocols are now available
        let input_available = ctx.system_table_service.get_console_input().is_ok();
        let output_available = ctx.system_table_service.get_console_output().is_ok();

        if input_available && output_available {
            // Both protocols available - register the service
            let console_services =
                StandardConsoleServices::new(ctx.boot_services.clone(), ctx.system_table_service.clone());

            ctx.commands.lock().add_service(console_services);
            *ctx.service_registered.lock() = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{boxed::Box, vec};
    use core::{cell::RefCell, ptr::NonNull};
    use patina::component::prelude::Service;
    use r_efi::efi;

    // Mock console input protocol that captures calls and provides controlled responses
    #[derive(Clone)]
    struct MockConsoleInput {
        wait_for_key: efi::Event,
    }

    impl MockConsoleInput {
        fn new() -> Self {
            Self { wait_for_key: NonNull::dangling().as_ptr() }
        }

        fn to_protocol(&self) -> efi::protocols::simple_text_input::Protocol {
            extern "efiapi" fn mock_reset(
                _this: *mut efi::protocols::simple_text_input::Protocol,
                _extended_verification: efi::Boolean,
            ) -> efi::Status {
                efi::Status::SUCCESS
            }

            extern "efiapi" fn mock_read_key_stroke(
                _this: *mut efi::protocols::simple_text_input::Protocol,
                key: *mut efi::protocols::simple_text_input::InputKey,
            ) -> efi::Status {
                if !key.is_null() {
                    unsafe {
                        (*key).scan_code = 0;
                        (*key).unicode_char = 0x41; // 'A'
                    }
                }
                efi::Status::SUCCESS
            }

            efi::protocols::simple_text_input::Protocol {
                reset: mock_reset,
                read_key_stroke: mock_read_key_stroke,
                wait_for_key: self.wait_for_key,
            }
        }
    }

    // Mock console output protocol that captures output and provides controlled responses
    #[derive(Clone)]
    struct MockConsoleOutput {
        mode: RefCell<efi::protocols::simple_text_output::Mode>,
    }

    impl MockConsoleOutput {
        fn new() -> Self {
            Self {
                mode: RefCell::new(efi::protocols::simple_text_output::Mode {
                    max_mode: 1,
                    mode: 0,
                    attribute: 0,
                    cursor_column: 0,
                    cursor_row: 0,
                    cursor_visible: efi::Boolean::TRUE,
                }),
            }
        }

        fn to_protocol(&self) -> efi::protocols::simple_text_output::Protocol {
            extern "efiapi" fn mock_output_string(
                _this: *mut efi::protocols::simple_text_output::Protocol,
                _string: *mut efi::Char16,
            ) -> efi::Status {
                efi::Status::SUCCESS
            }

            extern "efiapi" fn mock_clear_screen(
                _this: *mut efi::protocols::simple_text_output::Protocol,
            ) -> efi::Status {
                efi::Status::SUCCESS
            }

            extern "efiapi" fn mock_set_cursor_position(
                _this: *mut efi::protocols::simple_text_output::Protocol,
                _column: usize,
                _row: usize,
            ) -> efi::Status {
                efi::Status::SUCCESS
            }

            extern "efiapi" fn mock_enable_cursor(
                _this: *mut efi::protocols::simple_text_output::Protocol,
                _visible: efi::Boolean,
            ) -> efi::Status {
                efi::Status::SUCCESS
            }

            extern "efiapi" fn mock_query_mode(
                _this: *mut efi::protocols::simple_text_output::Protocol,
                _mode_number: usize,
                columns: *mut usize,
                rows: *mut usize,
            ) -> efi::Status {
                if !columns.is_null() && !rows.is_null() {
                    unsafe {
                        *columns = 80;
                        *rows = 25;
                    }
                }
                efi::Status::SUCCESS
            }

            extern "efiapi" fn mock_reset_stub(
                _this: *mut efi::protocols::simple_text_output::Protocol,
                _extended_verification: efi::Boolean,
            ) -> efi::Status {
                efi::Status::SUCCESS
            }

            extern "efiapi" fn mock_test_string_stub(
                _this: *mut efi::protocols::simple_text_output::Protocol,
                _string: *mut efi::Char16,
            ) -> efi::Status {
                efi::Status::SUCCESS
            }

            extern "efiapi" fn mock_set_mode_stub(
                _this: *mut efi::protocols::simple_text_output::Protocol,
                _mode_number: usize,
            ) -> efi::Status {
                efi::Status::SUCCESS
            }

            extern "efiapi" fn mock_set_attribute_stub(
                _this: *mut efi::protocols::simple_text_output::Protocol,
                _attribute: usize,
            ) -> efi::Status {
                efi::Status::SUCCESS
            }

            efi::protocols::simple_text_output::Protocol {
                reset: mock_reset_stub,
                output_string: mock_output_string,
                test_string: mock_test_string_stub,
                query_mode: mock_query_mode,
                set_mode: mock_set_mode_stub,
                set_attribute: mock_set_attribute_stub,
                clear_screen: mock_clear_screen,
                set_cursor_position: mock_set_cursor_position,
                enable_cursor: mock_enable_cursor,
                mode: self.mode.as_ptr(),
            }
        }
    }

    // Mock SystemTableService that provides our mock console protocols
    #[derive(Clone)]
    struct MockSystemTableService {
        console_input: MockConsoleInput,
        console_output: MockConsoleOutput,
    }

    impl MockSystemTableService {
        fn new() -> Self {
            Self { console_input: MockConsoleInput::new(), console_output: MockConsoleOutput::new() }
        }
    }

    impl SystemTableService for MockSystemTableService {
        fn get_console_input(&self) -> Result<&'static mut efi::protocols::simple_text_input::Protocol> {
            // Leak a fake static reference for testing
            let protocol = Box::leak(Box::new(self.console_input.to_protocol()));
            Ok(protocol)
        }

        fn get_console_output(&self) -> Result<&'static mut efi::protocols::simple_text_output::Protocol> {
            // Leak a fake static reference for testing
            let protocol = Box::leak(Box::new(self.console_output.to_protocol()));
            Ok(protocol)
        }

        fn get_standard_error(&self) -> Result<&'static mut efi::protocols::simple_text_output::Protocol> {
            self.get_console_output() // Reuse console output for stderr in tests
        }
    }

    fn create_test_console_service() -> (StandardConsoleServices, Box<MockSystemTableService>) {
        let mock_system_table = Box::new(MockSystemTableService::new());
        let boot_services = StandardBootServices::new_uninit();
        let system_table_service =
            Service::<dyn SystemTableService>::mock(mock_system_table.clone() as Box<dyn SystemTableService>);

        let console_services = StandardConsoleServices::new(boot_services, system_table_service);
        (console_services, mock_system_table)
    }

    #[test]
    fn test_standard_console_services_creation() {
        let uninit_boot_service = StandardBootServices::new_uninit();
        let uninit_system_table_service = Service::<dyn SystemTableService>::new_uninit();
        let _console_services = StandardConsoleServices::new(uninit_boot_service, uninit_system_table_service);
        // The test is just checking that a panic does not occur
    }

    #[test]
    fn test_console_services_provider_creation() {
        let _provider = ConsoleServicesProvider;
        // The test is just checking that a panic does not occur
    }

    #[test]
    fn test_output_string_success() {
        let (console_services, _mock_system_table) = create_test_console_service();

        // Test outputting a simple string
        let test_string = "Hello From Console Services!";
        let result = console_services.output_string(test_string);

        assert!(result.is_ok(), "output_string should succeed");
    }

    #[test]
    fn test_output_string_with_special_characters() {
        let (console_services, _mock_system_table) = create_test_console_service();

        // Test strings with special characters and unicode
        let test_strings = vec![
            "Test with newlines\r\n",
            "Unicode: äöüß 🦀",
            "", // Empty string
            "Tab\tCharacter",
        ];

        for test_string in test_strings {
            let result = console_services.output_string(test_string);
            assert!(result.is_ok(), "output_string should succeed for: {}", test_string);
        }
    }

    #[test]
    fn test_clear_screen_success() {
        let (console_services, _mock_system_table) = create_test_console_service();

        let result = console_services.clear_screen();

        assert!(result.is_ok(), "clear_screen should succeed");
    }

    #[test]
    fn test_set_cursor_position_success() {
        let (console_services, _mock_system_table) = create_test_console_service();

        let test_positions = vec![(0, 0), (10, 5), (79, 24), (100, 50)];

        for (col, row) in test_positions {
            let result = console_services.set_cursor_position(col, row);
            assert!(result.is_ok(), "set_cursor_position should succeed for ({}, {})", col, row);
        }
    }

    #[test]
    fn test_get_cursor_position_success() {
        let (console_services, _mock_system_table) = create_test_console_service();

        let result = console_services.get_cursor_position();

        assert!(result.is_ok(), "get_cursor_position should succeed");
        let (col, row) = result.unwrap();
        // The mock initializes cursor to (0, 0)
        assert_eq!(col, 0, "Column should be 0");
        assert_eq!(row, 0, "Row should be 0");
    }

    #[test]
    fn test_enable_cursor_success() {
        let (console_services, _mock_system_table) = create_test_console_service();

        // Test enabling cursor
        let result = console_services.enable_cursor(true);
        assert!(result.is_ok(), "enable_cursor(true) should succeed");

        // Test disabling cursor
        let result = console_services.enable_cursor(false);
        assert!(result.is_ok(), "enable_cursor(false) should succeed");
    }

    #[test]
    fn test_read_key_stroke_success() {
        let (console_services, _mock_system_table) = create_test_console_service();

        let result = console_services.read_key_stroke();

        assert!(result.is_ok(), "read_key_stroke should succeed");
        let key = result.unwrap();
        assert_eq!(key, 0x41, "Should return 'A' character");
    }

    #[test]
    fn test_reset_input_success() {
        let (console_services, _mock_system_table) = create_test_console_service();

        // Test reset without extended verification
        let result = console_services.reset_input(false);
        assert!(result.is_ok(), "reset_input(false) should succeed");

        // Test reset with extended verification
        let result = console_services.reset_input(true);
        assert!(result.is_ok(), "reset_input(true) should succeed");
    }

    #[test]
    fn test_query_mode_success() {
        let (console_services, _mock_system_table) = create_test_console_service();

        let result = console_services.query_mode();

        assert!(result.is_ok(), "query_mode should succeed");
        let (cols, rows) = result.unwrap();
        assert_eq!(cols, 80, "Columns should match expected value");
        assert_eq!(rows, 25, "Rows should match expected value");
    }

    #[test]
    fn test_string_encoding_roundtrip() {
        let (console_services, _mock_system_table) = create_test_console_service();

        // Test that UTF-16 conversion is working as expected
        let test_strings = vec![
            "ASCII only",
            "Latin-1: café",
            "Unicode: 🦀 Rust",
            "Mixed: Hello 世界 🌍",
            "Numbers: 0123456789",
            "Special: !@#$%^&*()",
        ];

        for test_string in &test_strings {
            let result = console_services.output_string(test_string);
            assert!(result.is_ok(), "Should successfully output: {}", test_string);
        }
    }

    #[test]
    fn test_null_terminator_handling() {
        let (console_services, _mock_system_table) = create_test_console_service();

        // The string should be null terminated
        let result = console_services.output_string("Test");
        assert!(result.is_ok(), "Should successfully output string");

        // An empty string should work
        let result = console_services.output_string("");
        assert!(result.is_ok(), "Should successfully output empty string");
    }
}
