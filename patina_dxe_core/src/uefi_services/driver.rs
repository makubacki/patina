//! DXE Core implementation of [`DriverServices`].
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
//!

use alloc::vec::Vec;

use patina::component::service::{
    IntoService,
    uefi_services::driver::{DriverError, DriverServices, Handle},
};

use crate::driver_services::{core_connect_controller, core_disconnect_controller};

/// Core implementation of [`DriverServices`], delegating to the core driver model by calling the internal
/// `core_*` Rust APIs.
#[derive(IntoService)]
#[service(dyn DriverServices)]
pub(crate) struct CoreDriverServices;

impl DriverServices for CoreDriverServices {
    fn connect_controller(&self, controller: Handle, recursive: bool) -> Result<(), DriverError> {
        // SAFETY: No remaining device path is passed, so there is no device-path pointer whose
        // validity the caller must uphold. Handles are validated inside the core routine.
        unsafe { core_connect_controller(controller.as_raw(), Vec::new(), None, recursive) }.map_err(DriverError::from)
    }

    fn disconnect_controller(
        &self,
        controller: Handle,
        driver: Option<Handle>,
        child: Option<Handle>,
    ) -> Result<(), DriverError> {
        // SAFETY: All handles are validated inside `core_disconnect_controller`.
        unsafe {
            core_disconnect_controller(controller.as_raw(), driver.map(|h| h.as_raw()), child.map(|h| h.as_raw()))
        }
        .map_err(DriverError::from)
    }
}
