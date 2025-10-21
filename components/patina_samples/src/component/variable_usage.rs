//! Variable Service Usage Examples
//!
//! Demonstrates usage patterns for UEFI variable operations through the
//! [`patina_uefi_services::service::variable::UefiSpecVariableServices`] trait.
//!
//! Includes three example components showcasing variable reading, writing,
//! and enumeration operations.
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
//!
extern crate alloc;

use alloc::{string::String, vec, vec::Vec};
use patina::{BinaryGuid, component::IntoComponent, component::prelude::Service, error::Result};
use patina_uefi_services::service::variable::UefiSpecVariableServices;

// UEFI Global Variable GUID
const GLOBAL_VARIABLE_GUID: BinaryGuid =
    BinaryGuid::from_fields(0x8BE4DF61, 0x93CA, 0x11D2, 0xAA, 0x0D, &[0x00, 0xE0, 0x98, 0x03, 0x2B, 0x8C]);

// Variable attribute constants
const EFI_VARIABLE_BOOTSERVICE_ACCESS: u32 = 0x00000002;
const EFI_VARIABLE_RUNTIME_ACCESS: u32 = 0x00000004;
const EFI_VARIABLE_NON_VOLATILE: u32 = 0x00000001;

/// Example component demonstrating reading a standard UEFI variable.
///
/// This component reads the BootOrder variable which contains the boot device order.
#[derive(IntoComponent)]
pub struct ReadBootOrder;

impl ReadBootOrder {
    /// Component entry point that reads the BootOrder variable.
    ///
    /// # Arguments
    ///
    /// * `variables` - The variable services for reading UEFI variables
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error status
    pub fn entry_point(self, variables: Service<dyn UefiSpecVariableServices>) -> Result<()> {
        // Read the BootOrder variable from the Global Variable namespace
        match (**variables).get_variable("BootOrder", &GLOBAL_VARIABLE_GUID) {
            Ok((data, _attributes)) => {
                // BootOrder is an array of UINT16 values
                let _boot_order_count = data.len() / core::mem::size_of::<u16>();
                // Process boot order data...
                Ok(())
            }
            Err(patina::error::EfiError::NotFound) => {
                // BootOrder not found is acceptable (system might not have it set yet)
                Ok(())
            }
            Err(e) => Err(e),
        }
    }
}

/// Example component demonstrating writing a custom UEFI variable.
///
/// This component creates a new variable with custom data and verifies it was written correctly.
#[derive(IntoComponent)]
pub struct WriteCustomVariable;

impl WriteCustomVariable {
    /// Component entry point that writes and verifies a custom variable.
    ///
    /// # Arguments
    ///
    /// * `variables` - The variable services for writing UEFI variables
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error status
    pub fn entry_point(self, variables: Service<dyn UefiSpecVariableServices>) -> Result<()> {
        let variable_name = "MyCustomVariable";
        let variable_guid = &GLOBAL_VARIABLE_GUID;

        // Create some data to store
        let data: Vec<u8> = vec![
            0x50, 0x61, 0x74, 0x69, 0x6E, 0x61, // "Patina" in ASCII
        ];

        // Set variable attributes (boot services access, runtime access, non-volatile)
        let attributes = EFI_VARIABLE_BOOTSERVICE_ACCESS | EFI_VARIABLE_RUNTIME_ACCESS | EFI_VARIABLE_NON_VOLATILE;

        // Write the variable
        (**variables).set_variable(variable_name, variable_guid, attributes, &data)?;

        // Verify the write by reading it back
        let (read_data, read_attributes) = (**variables).get_variable(variable_name, variable_guid)?;

        // Verify data matches
        assert_eq!(data, read_data, "Variable data mismatch");
        assert_eq!(attributes, read_attributes, "Variable attributes mismatch");

        Ok(())
    }
}

/// Example component demonstrating enumeration of all UEFI variables.
///
/// This component iterates through all variables in the system using the
/// get_next_variable_name API.
#[derive(IntoComponent)]
pub struct EnumerateVariables;

impl EnumerateVariables {
    /// Component entry point that enumerates all system variables.
    ///
    /// # Arguments
    ///
    /// * `variables` - The variable services for enumerating UEFI variables
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error status
    pub fn entry_point(self, variables: Service<dyn UefiSpecVariableServices>) -> Result<()> {
        // Start with an empty name to get the first variable
        let mut variable_name = String::new();
        let mut vendor_guid = GLOBAL_VARIABLE_GUID;

        while (**variables).get_next_variable_name(&mut variable_name, &mut vendor_guid).is_ok() {
            // Process the variable (name and guid are now updated in place)
            // In a real implementation, you would use variable_name and vendor_guid here
        }

        Ok(())
    }
}
