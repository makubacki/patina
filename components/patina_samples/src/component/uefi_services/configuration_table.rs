//! Configuration Table Sample Component
//!
//! This component demonstrates [`ConfigurationTableServices`] by installing a vendor configuration
//! table under a GUID and reading it back. Configuration tables are how firmware publishes system-wide,
//! pointer-addressable tables such as ACPI RSDP. An OS or later component finds them by GUID in the UEFI
//! system table.
//!
//! Because configuration tables carry no GUID-to-type binding, retrieval with
//! [`get_configuration_table`](patina::component::service::uefi_services::config_table::ConfigurationTableServicesExt::get_configuration_table)
//! is `unsafe`. The caller asserts that the installed table actually has the requested type.
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
//!

use patina::{
    BinaryGuid,
    component::{
        component,
        service::{
            Service,
            uefi_services::config_table::{ConfigurationTableServices, ConfigurationTableServicesExt},
        },
    },
    error::Result,
};

/// A sample vendor table laid out the way a typical firmware table would be as `#[repr(C)]`, with a
/// signature and version so consumers can validate it.
#[repr(C)]
pub struct SampleVendorTable {
    /// Four-character signature identifying the table (`b"PTNA"`).
    pub signature: [u8; 4],
    /// Table format version.
    pub version: u32,
    /// Number of vendor-defined entries that follow in the real table.
    pub entry_count: u32,
}

/// GUID under which the table is installed in the system table.
const VENDOR_TABLE_GUID: BinaryGuid = BinaryGuid::from_string("0fedcba9-8765-4321-fedc-ba9876543210");

/// The table instance. It must be `'static` as the system table stores a pointer to it that outlives
/// this component's entry point.
static VENDOR_TABLE: SampleVendorTable = SampleVendorTable { signature: *b"PTNA", version: 1, entry_count: 0 };

/// Installs [`SampleVendorTable`] into the system configuration table, then reads it back to
/// confirm the table was installed correctly.
#[derive(Default)]
pub struct ConfigurationTableSample;

#[component]
impl ConfigurationTableSample {
    /// Creates a new instance of the component.
    pub fn new() -> Self {
        Self
    }

    fn entry_point(self, config: Service<dyn ConfigurationTableServices>) -> Result<()> {
        // Install the table. `into_inner()` converts the typed `BinaryGuid` into the raw
        // `efi::Guid` the service expects.
        config.install_configuration_table(VENDOR_TABLE_GUID.into_inner(), &VENDOR_TABLE)?;
        log::info!("Installed SampleVendorTable v{}", VENDOR_TABLE.version);

        // Read it back. This is `unsafe` because only the caller knows the type stored under the
        // GUID. It was installed here as a `SampleVendorTable`, so requesting that type is sound.
        // SAFETY: This component installed a `SampleVendorTable` under `VENDOR_TABLE_GUID` above,
        // and `VENDOR_TABLE` is `'static`, so the type and lifetime assertions hold.
        let table = unsafe { config.get_configuration_table::<SampleVendorTable>(VENDOR_TABLE_GUID.into_inner()) };
        if let Some(table) = table {
            log::info!(
                "Read back signature {:?}, {} entries",
                core::str::from_utf8(&table.signature).unwrap_or("????"),
                table.entry_count
            );
        }

        Ok(())
    }
}
