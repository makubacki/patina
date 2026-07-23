//! DXE Core implementation of [`ConfigurationTableServices`].
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
//!

use patina::component::service::{
    IntoService,
    uefi_services::config_table::{ConfigTableError, ConfigTablePtr, ConfigurationTableServices},
};
use patina::standard::efi;

use crate::config_tables::{core_install_configuration_table, get_configuration_table};
use crate::systemtables::SYSTEM_TABLE;

/// Core implementation of [`ConfigurationTableServices`], operating on the global system table via
/// the core's internal Rust APIs.
#[derive(IntoService)]
#[service(dyn ConfigurationTableServices)]
pub(crate) struct CoreConfigurationTableServices;

impl ConfigurationTableServices for CoreConfigurationTableServices {
    fn install_table(&self, guid: efi::Guid, table: ConfigTablePtr) -> Result<(), ConfigTableError> {
        let mut st_guard = SYSTEM_TABLE.lock();
        let st = st_guard.as_mut().ok_or(ConfigTableError::NotFound)?;
        core_install_configuration_table(guid, table.as_raw(), st).map(|_| ()).map_err(ConfigTableError::from)
    }

    fn remove_table(&self, guid: efi::Guid) -> Result<(), ConfigTableError> {
        let mut st_guard = SYSTEM_TABLE.lock();
        let st = st_guard.as_mut().ok_or(ConfigTableError::NotFound)?;
        // Installing a null table removes the entry for the GUID.
        core_install_configuration_table(guid, core::ptr::null_mut(), st).map(|_| ()).map_err(ConfigTableError::from)
    }

    fn get_table(&self, guid: efi::Guid) -> Option<ConfigTablePtr> {
        get_configuration_table(&guid).and_then(|table| ConfigTablePtr::from_raw(table.as_ptr()))
    }
}
