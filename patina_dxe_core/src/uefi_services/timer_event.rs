//! DXE Core implementation of [`TimerEventServices`].
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
//!

use patina::component::service::{
    IntoService,
    uefi_services::{
        event::{Event, EventError, EventNotifyCallback},
        timer_event::{TimerEventServices, TimerType, Tpl},
    },
};
use patina::error::EfiError;
use patina::standard::efi;

use crate::events::set_timer as core_set_timer;

use super::event::create_event_internal;

/// Core implementation of [`TimerEventServices`], delegating to the core event database.
///
/// Registered with the component dispatcher only once the Timer Architectural Protocol is
/// installed, so components depending on this service are not dispatched until `set_timer` can
/// actually take effect.
#[derive(IntoService)]
#[service(dyn TimerEventServices)]
pub(crate) struct CoreTimerEventServices;

impl TimerEventServices for CoreTimerEventServices {
    fn create_timer_event(&self, notify_tpl: Tpl, callback: EventNotifyCallback) -> Result<Event, EventError> {
        create_event_internal(efi::EVT_TIMER | efi::EVT_NOTIFY_SIGNAL, notify_tpl, callback, None)
    }

    fn set_timer(&self, event: Event, timer_type: TimerType) -> Result<(), EventError> {
        // Note: UEFI timer intervals are expressed in units of 100ns.
        let (delay, trigger_time) = match timer_type {
            TimerType::Cancel => (efi::TIMER_CANCEL, 0),
            TimerType::Relative(interval) => (efi::TIMER_RELATIVE, (interval.as_nanos() / 100) as u64),
            TimerType::Periodic(interval) => (efi::TIMER_PERIODIC, (interval.as_nanos() / 100) as u64),
        };

        match core_set_timer(event.as_raw(), delay, trigger_time) {
            efi::Status::SUCCESS => Ok(()),
            status => Err(EventError::from(EfiError::status_to_result(status).unwrap_err())),
        }
    }
}
