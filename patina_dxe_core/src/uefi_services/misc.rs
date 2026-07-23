//! DXE Core implementation of [`MiscServices`].
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
//!

use patina::component::service::{IntoService, uefi_services::misc::MiscServices};

/// Core implementation of [`MiscServices`].
#[derive(IntoService)]
#[service(dyn MiscServices)]
pub(crate) struct CoreMiscServices;

impl MiscServices for CoreMiscServices {
    fn calculate_crc32(&self, data: &[u8]) -> u32 {
        crc32fast::hash(data)
    }
}
