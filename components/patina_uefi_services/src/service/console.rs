//! Console Services Abstraction
//!
//! Trait definitions for a service that provides UEFI console operations.
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0

use patina::error::Result;

#[cfg(any(test, feature = "mockall"))]
use mockall::automock;

/// Console input and output operations abstraction.
///
/// Provides text input, text output, and basic console management.
#[cfg_attr(any(test, feature = "mockall"), automock)]
pub trait ConsoleServices {
    /// Writes a string to the console output.
    ///
    /// # Arguments
    /// * `text` - The text string to output to the console
    ///
    /// # Returns
    /// * `Result<()>` - Result status
    ///
    /// # Example
    /// ```no_run
    /// # use patina_uefi_services::service::console::ConsoleServices;
    /// # use patina::error::Result;
    /// # fn example(console_services: &dyn ConsoleServices) -> Result<()> {
    /// console_services.output_string("Hello, UEFI!")?;
    /// # Ok(())
    /// # }
    /// ```
    fn output_string(&self, text: &str) -> Result<()>;

    /// Clears the console output screen.
    ///
    /// # Returns
    /// * `Result<()>` - Result status
    ///
    /// # Example
    /// ```no_run
    /// # use patina_uefi_services::service::console::ConsoleServices;
    /// # use patina::error::Result;
    /// # fn example(console_services: &dyn ConsoleServices) -> Result<()> {
    /// console_services.clear_screen()?;
    /// # Ok(())
    /// # }
    /// ```
    fn clear_screen(&self) -> Result<()>;

    /// Sets the cursor position on the console.
    ///
    /// # Arguments
    /// * `column` - Column position (0-based)
    /// * `row` - Row position (0-based)
    ///
    /// # Returns
    /// * `Result<()>` - Result status
    ///
    /// # Example
    /// ```no_run
    /// # use patina_uefi_services::service::console::ConsoleServices;
    /// # use patina::error::Result;
    /// # fn example(console_services: &dyn ConsoleServices) -> Result<()> {
    /// console_services.set_cursor_position(10, 5)?;
    /// # Ok(())
    /// # }
    /// ```
    fn set_cursor_position(&self, column: usize, row: usize) -> Result<()>;

    /// Gets the current cursor position.
    ///
    /// # Returns
    /// * `Result<(usize, usize)>` - Tuple of (column, row) position
    ///
    /// # Example
    /// ```no_run
    /// # use patina_uefi_services::service::console::ConsoleServices;
    /// # use patina::error::Result;
    /// # fn example(console_services: &dyn ConsoleServices) -> Result<()> {
    /// let (col, row) = console_services.get_cursor_position()?;
    /// # Ok(())
    /// # }
    /// ```
    fn get_cursor_position(&self) -> Result<(usize, usize)>;

    /// Enables or disables the cursor visibility.
    ///
    /// # Arguments
    /// * `visible` - True to make cursor visible, false to hide it
    ///
    /// # Returns
    /// * `Result<()>` - Result status
    ///
    /// # Example
    /// ```no_run
    /// # use patina_uefi_services::service::console::ConsoleServices;
    /// # use patina::error::Result;
    /// # fn example(console_services: &dyn ConsoleServices) -> Result<()> {
    /// console_services.enable_cursor(true)?;  // Show cursor
    /// console_services.enable_cursor(false)?; // Hide cursor
    /// # Ok(())
    /// # }
    /// ```
    fn enable_cursor(&self, visible: bool) -> Result<()>;

    /// Reads a keystroke from the console input.
    ///
    /// This function will block until a key is pressed.
    ///
    /// # Returns
    /// * `Result<u16>` - The Unicode character code of the key pressed
    ///
    /// # Example
    /// ```no_run
    /// # use patina_uefi_services::service::console::ConsoleServices;
    /// # use patina::error::Result;
    /// # fn example(console_services: &dyn ConsoleServices) -> Result<()> {
    /// let key = console_services.read_key_stroke()?;
    /// if key == 0x0D { // Enter key
    ///     println!("Enter pressed!");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    fn read_key_stroke(&self) -> Result<u16>;

    /// Checks if a keystroke is available without blocking.
    ///
    /// # Returns
    /// * `Result<bool>` - True if a keystroke is available, false otherwise
    ///
    /// # Example
    /// ```no_run
    /// # use patina_uefi_services::service::console::ConsoleServices;
    /// # use patina::error::Result;
    /// # fn example(console_services: &dyn ConsoleServices) -> Result<()> {
    /// if console_services.is_key_available()? {
    ///     let key = console_services.read_key_stroke()?;
    ///     // Process the key...
    /// }
    /// # Ok(())
    /// # }
    /// ```
    fn is_key_available(&self) -> Result<bool>;

    /// Resets the console input buffer.
    ///
    /// # Arguments
    /// * `extended_verification` - Perform extended verification of input device
    ///
    /// # Returns
    /// * `Result<()>` - Result status
    ///
    /// # Example
    /// ```no_run
    /// # use patina_uefi_services::service::console::ConsoleServices;
    /// # use patina::error::Result;
    /// # fn example(console_services: &dyn ConsoleServices) -> Result<()> {
    /// console_services.reset_input(false)?;
    /// # Ok(())
    /// # }
    /// ```
    fn reset_input(&self, extended_verification: bool) -> Result<()>;

    /// Gets the current console mode information.
    ///
    /// # Returns
    /// * `Result<(usize, usize)>` - Tuple of (columns, rows) for current mode
    ///
    /// # Example
    /// ```no_run
    /// # use patina_uefi_services::service::console::ConsoleServices;
    /// # use patina::error::Result;
    /// # fn example(console_services: &dyn ConsoleServices) -> Result<()> {
    /// let (cols, rows) = console_services.query_mode()?;
    /// # Ok(())
    /// # }
    /// ```
    fn query_mode(&self) -> Result<(usize, usize)>;
}
