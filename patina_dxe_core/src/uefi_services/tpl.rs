//! DXE Core implementation of [`TplServices`].
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
//!

use patina::component::service::{
    IntoService,
    uefi_services::tpl::{PreviousTpl, Tpl, TplServices},
};

use crate::events::{raise_tpl, restore_tpl};
use crate::uefi_services::event::tpl_to_efi;
/// Core implementation of [`TplServices`], delegating to the core TPL primitives.
#[derive(IntoService)]
#[service(dyn TplServices)]
pub(crate) struct CoreTplServices;

impl TplServices for CoreTplServices {
    fn raise_tpl(&self, tpl: Tpl) -> PreviousTpl {
        PreviousTpl::from_raw(raise_tpl(tpl_to_efi(tpl)))
    }

    fn restore_tpl(&self, previous: PreviousTpl) {
        restore_tpl(previous.as_raw());
    }
}
