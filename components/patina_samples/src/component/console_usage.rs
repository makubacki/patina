//! Console Services Usage Examples
//!
//! This module demonstrates how to use ConsoleServices for text output and console management.
//!
//! ## Examples Included
//!
//! - **BasicConsoleOutput**: Simple text output to console
//! - **FormattedConsoleOutput**: Formatted output with cursor positioning
//! - **InteractiveConsole**: Console manipulation (clear, cursor control)
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0

extern crate alloc;

use patina::{
    component::{IntoComponent, service::Service},
    error::Result,
};
use patina_uefi_services::service::console::ConsoleServices;

/// Example component demonstrating basic console output.
///
/// This component shows the simplest use case: writing text to the console.
/// Note that the ConsoleServices dependency is requested via the component's entry point.
#[derive(IntoComponent)]
pub struct BasicConsoleOutput;

impl BasicConsoleOutput {
    /// Component entry point that outputs a simple message to the console.
    ///
    /// # Arguments
    ///
    /// * `console` - The console services for text output
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error status
    pub fn entry_point(self, console: Service<dyn ConsoleServices>) -> Result<()> {
        console.output_string("Hello from Patina Console Services!\r\n")?;
        console.output_string("This is a basic console output example.\r\n")?;

        Ok(())
    }
}

/// Example component demonstrating formatted console output with cursor positioning.
///
/// This component shows how to use cursor positioning and formatted output.
#[derive(IntoComponent)]
pub struct FormattedConsoleOutput;

impl FormattedConsoleOutput {
    /// Component entry point that demonstrates formatted console output.
    ///
    /// # Arguments
    ///
    /// * `console` - The console services for text output and cursor control
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error status
    pub fn entry_point(self, console: Service<dyn ConsoleServices>) -> Result<()> {
        // Output a header
        console.output_string("\r\n=== Formatted Console Example ===\r\n\r\n")?;

        // Create a simple table with cursor positioning
        console.set_cursor_position(0, 5)?;
        console.output_string("Column 1")?;

        console.set_cursor_position(20, 5)?;
        console.output_string("Column 2")?;

        console.set_cursor_position(40, 5)?;
        console.output_string("Column 3")?;

        // Add data rows
        console.set_cursor_position(0, 6)?;
        console.output_string("Data A")?;

        console.set_cursor_position(20, 6)?;
        console.output_string("Data B")?;

        console.set_cursor_position(40, 6)?;
        console.output_string("Data C")?;

        // Get final cursor position to show where we ended up
        let (_col, row) = console.get_cursor_position()?;
        console.set_cursor_position(0, row + 2)?;
        console.output_string("Table complete!\r\n")?;

        Ok(())
    }
}

/// Example component demonstrating interactive console manipulation.
///
/// This component shows how to use console management features like
/// clearing the screen, cursor visibility, and positioning.
#[derive(IntoComponent)]
pub struct InteractiveConsole;

impl InteractiveConsole {
    /// Component entry point that demonstrates console manipulation.
    ///
    /// # Arguments
    ///
    /// * `console` - The console services for full console control
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error status
    pub fn entry_point(self, console: Service<dyn ConsoleServices>) -> Result<()> {
        // Clear the screen first
        console.clear_screen()?;

        // Show cursor manipulation
        console.output_string("Demonstrating cursor control...\r\n")?;

        // Hide cursor
        console.enable_cursor(false)?;
        console.output_string("Cursor is now hidden\r\n")?;

        // Show cursor again
        console.enable_cursor(true)?;
        console.output_string("Cursor is now visible\r\n")?;

        // Draw a simple pattern using cursor positioning
        console.output_string("\r\nDrawing a pattern:\r\n\r\n")?;
        for i in 0..5 {
            console.set_cursor_position(i * 4, 10 + i)?;
            console.output_string("*")?;
        }

        // Return cursor to bottom
        console.set_cursor_position(0, 16)?;
        console.output_string("\r\nConsole demonstration complete!\r\n")?;

        Ok(())
    }
}
