//! DXE Core implementation of [`TimingServices`].
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
//!

use patina::component::service::{
    IntoService,
    uefi_services::timing::{TimingError, TimingServices},
};

use crate::misc_boot_services::{core_set_watchdog_timer, core_stall};

/// Core implementation of [`TimingServices`], delegating to the metronome and watchdog
/// architectural support through the core's internal Rust APIs.
///
/// Registered with the component dispatcher only once both the Metronome and Watchdog Timer
/// Architectural Protocols are installed, so components depending on this service are not dispatched
/// until stall and watchdog timer control is available.
#[derive(IntoService)]
#[service(dyn TimingServices)]
pub(crate) struct CoreTimingServices;

impl TimingServices for CoreTimingServices {
    fn stall(&self, duration: core::time::Duration) -> Result<(), TimingError> {
        core_stall(duration.as_micros() as usize).map_err(TimingError::from)
    }

    fn set_watchdog_timer(&self, timeout_seconds: u64, watchdog_code: u64) -> Result<(), TimingError> {
        core_set_watchdog_timer(timeout_seconds as usize, watchdog_code).map_err(TimingError::from)
    }
}
