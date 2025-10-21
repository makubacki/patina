//! Variable Services Abstraction
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0

use alloc::{string::String, vec::Vec};
use patina::BinaryGuid;
use patina::error::Result;

#[cfg(any(test, feature = "mockall"))]
use mockall::automock;

/// An interface for interacting with UEFI Variables.
///
/// ## Example Usage
///
/// ```rust
/// use patina::component::{IntoComponent, prelude::Service};
/// use patina_uefi_services::service::variable::UefiSpecVariableServices;
/// use patina::error::Result;
/// use patina::BinaryGuid;
///
/// #[derive(IntoComponent)]
/// struct MyComponent;
///
/// impl MyComponent {
///     fn entry_point(
///         self,
///         variable_services: Service<dyn UefiSpecVariableServices>
///     ) -> Result<()> {
///         let guid = BinaryGuid::from_fields(0x8BE4DF61, 0x93CA, 0x11D2, 0xAA, 0x0D, &[0x00, 0xE0, 0x98, 0x03, 0x2B, 0x8C]);
///         let (data, attributes) = variable_services.get_variable("BootOrder", &guid)?;
///         Ok(())
///     }
/// }
/// ```
#[cfg_attr(any(test, feature = "mockall"), automock)]
pub trait UefiSpecVariableServices {
    /// Gets the value of a UEFI variable.
    ///
    /// # Arguments
    /// * `variable_name` - Name of the variable to retrieve
    /// * `vendor_guid` - GUID namespace of the variable
    ///
    /// # Returns
    /// * `Result<(Vec<u8>, u32)>` - Variable data and attributes on success
    ///
    /// # Example
    /// ```no_run
    /// # use patina_uefi_services::service::variable::UefiSpecVariableServices;
    /// # use patina::error::Result;
    /// # use patina::BinaryGuid;
    /// # const GLOBAL_VARIABLE_GUID: BinaryGuid = BinaryGuid::from_fields(0x8BE4DF61, 0x93CA, 0x11D2, 0xAA, 0x0D, &[0x00, 0xE0, 0x98, 0x03, 0x2B, 0x8C]);
    /// # fn example(variable_services: &dyn UefiSpecVariableServices) -> Result<()> {
    /// let (data, attributes) = variable_services.get_variable("BootOrder", &GLOBAL_VARIABLE_GUID)?;
    /// # Ok(())
    /// # }
    /// ```
    fn get_variable(&self, variable_name: &str, vendor_guid: &BinaryGuid) -> Result<(Vec<u8>, u32)>;

    /// Sets the value of a UEFI variable.
    ///
    /// # Arguments
    /// * `variable_name` - Name of the variable to set
    /// * `vendor_guid` - GUID namespace of the variable
    /// * `attributes` - Variable attributes (runtime, boot service access, etc.)
    /// * `data` - Data to store in the variable
    ///
    /// # Returns
    /// * `Result<()>` - Success status
    ///
    /// # Example
    /// ```no_run
    /// # use patina_uefi_services::service::variable::UefiSpecVariableServices;
    /// # use patina::error::Result;
    /// # use patina::BinaryGuid;
    /// # const EFI_VARIABLE_BOOTSERVICE_ACCESS: u32 = 0x00000002;
    /// # fn example(variable_services: &dyn UefiSpecVariableServices) -> Result<()> {
    /// # let my_guid = BinaryGuid::from_fields(0, 0, 0, 0, 0, &[0; 6]);
    /// # let data = vec![1, 2, 3, 4];
    /// variable_services.set_variable(
    ///     "MyVar",
    ///     &my_guid,
    ///     EFI_VARIABLE_BOOTSERVICE_ACCESS,
    ///     &data
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    fn set_variable(&self, variable_name: &str, vendor_guid: &BinaryGuid, attributes: u32, data: &[u8]) -> Result<()>;

    /// Enumerates the current variable names.
    ///
    /// This method provides a way to iterate through all variables by repeatedly
    /// calling it with the results from the previous call.
    ///
    /// # Arguments
    /// * `variable_name` - On input: previous variable name; on output: next variable name
    /// * `vendor_guid` - On input: previous vendor GUID; on output: next vendor GUID
    ///
    /// # Returns
    /// * `Result<()>` - Success status; returns NOT_FOUND when no more variables
    ///
    /// # Example
    /// ```ignore
    /// # use patina_uefi_services::service::variable::UefiSpecVariableServices;
    /// # use patina::error::Result;
    /// # use patina::OwnedGuid;
    /// # use alloc::string::String;
    /// # fn example(variable_services: &dyn UefiSpecVariableServices) -> Result<()> {
    /// let mut name = String::new();
    /// let mut guid = OwnedGuid::from_fields(0, 0, 0, 0, 0, [0; 6]);
    /// while variable_services.get_next_variable_name(&mut name, &mut guid).is_ok() {
    ///     println!("Variable: {} in namespace {:?}", name, guid);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    fn get_next_variable_name(&self, variable_name: &mut String, vendor_guid: &mut BinaryGuid) -> Result<()>;

    /// Returns information about the EFI variables.
    ///
    /// # Arguments
    /// * `attributes` - Variable attributes to query information for
    ///
    /// # Returns
    /// * `Result<(u64, u64, u64)>` - Tuple of (MaximumVariableStorageSize, RemainingVariableStorageSize, MaximumVariableSize)
    ///
    /// # Example
    /// ```no_run
    /// # use patina_uefi_services::service::variable::UefiSpecVariableServices;
    /// # use patina::error::Result;
    /// # const EFI_VARIABLE_BOOTSERVICE_ACCESS: u32 = 0x00000002;
    /// # const EFI_VARIABLE_RUNTIME_ACCESS: u32 = 0x00000004;
    /// # fn example(variable_services: &dyn UefiSpecVariableServices) -> Result<()> {
    /// let (max_storage, remaining, max_size) = variable_services.query_variable_info(
    ///     EFI_VARIABLE_BOOTSERVICE_ACCESS | EFI_VARIABLE_RUNTIME_ACCESS
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    fn query_variable_info(&self, attributes: u32) -> Result<(u64, u64, u64)>;
}
