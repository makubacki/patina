//! Event Services Abstraction
//!
//! A Patina service abstraction for UEFI event and timer operations.
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0

use crate::types::Event;
use patina::{
    BinaryGuid,
    boot_services::{
        event::{EventTimerType, EventType},
        tpl::Tpl,
    },
    error::Result,
};

#[cfg(any(test, feature = "mockall"))]
use mockall::automock;

/// Provides UEFI event and timer operations through a memory-safe interface.
///
/// ## Memory Safety
///
/// Provides type-safe event handle management and automatic cleanup.
#[cfg_attr(any(test, feature = "mockall"), automock)]
pub trait EventServices {
    /// Creates a new UEFI event with the specified type and notification TPL.
    ///
    /// # Arguments
    ///
    /// * `event_type` - The type of event to create (timer, notification, etc.)
    /// * `notify_tpl` - The Task Priority Level for event notifications
    ///
    /// # Returns
    ///
    /// Returns an `Event` handle that can be used with other event operations.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use patina_uefi_services::service::event::EventServices;
    /// # use patina::boot_services::{event::EventType, tpl::Tpl};
    /// # fn example(event_services: &dyn EventServices) -> patina::error::Result<()> {
    /// let event = event_services.create_event(EventType::TIMER, Tpl::APPLICATION)?;
    /// // Use event...
    /// event_services.close_event(event)?;
    /// # Ok(())
    /// # }
    /// ```
    fn create_event(&self, event_type: EventType, notify_tpl: Tpl) -> Result<Event>;

    /// Closes and deallocates a UEFI event.
    ///
    /// # Arguments
    ///
    /// * `event` - The event handle to close
    ///
    /// # Safety
    ///
    /// After calling this method, the event handle becomes invalid and must not be used.
    fn close_event(&self, event: Event) -> Result<()>;

    /// Configures a timer event with the specified timing parameters.
    ///
    /// # Arguments
    ///
    /// * `event` - The timer event to configure
    /// * `timer_type` - Type of timer (relative, periodic, or cancel)
    /// * `trigger_time` - Time in 100ns units for the timer to trigger
    fn set_timer(&self, event: Event, timer_type: EventTimerType, trigger_time: u64) -> Result<()>;

    /// Waits for one or more events to be signaled.
    ///
    /// # Arguments
    ///
    /// * `events` - Mutable slice of events to wait for
    ///
    /// # Returns
    ///
    /// Returns the index of the event that was signaled.
    ///
    /// # Safety
    ///
    /// The events slice is modified during the wait operation to track event states.
    fn wait_for_event(&self, events: &mut [Event]) -> Result<usize>;

    /// Checks if an event is currently signaled without blocking.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to check
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the event is signaled, or an error if not signaled or invalid.
    fn check_event(&self, event: Event) -> Result<()>;

    /// Manually signals an event.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to signal
    fn signal_event(&self, event: Event) -> Result<()>;

    /// Creates a system event that is automatically signaled at system milestones.
    ///
    /// # Arguments
    ///
    /// * `event_type` - The type of system event to create
    /// * `notify_tpl` - The Task Priority Level for event notifications
    ///
    /// # Returns
    ///
    /// Returns an `Event` handle for the system event.
    fn create_system_event(&self, event_type: EventType, notify_tpl: Tpl) -> Result<Event>;
}

/// An extension trait for `EventServices` that provides closure-based event creation.
///
/// Note: This trait is separate from `EventServices` because generic methods are not compatible with dynamic
/// dispatch (dyn trait objects).
pub trait EventServicesClosureExt: EventServices {
    /// Creates a system event with a closure callback.
    ///
    /// This is a convenience method that creates an event and registers it with a system event group for automatic
    /// signaling at system milestones.
    ///
    /// # Arguments
    ///
    /// * `event_type` - The type of system event to create
    /// * `notify_tpl` - The Task Priority Level for event notifications
    /// * `callback` - Closure to execute when the event is signaled
    /// * `event_group` - Optional event group for system events
    ///
    /// # Returns
    ///
    /// Returns an `Event` handle for the system event.
    fn create_system_event_with_callback<F>(
        &self,
        event_type: EventType,
        notify_tpl: Tpl,
        callback: F,
        event_group: &'static BinaryGuid,
    ) -> Result<Event>
    where
        F: FnMut(Event) + Send + Sync + 'static;
}
