//! Event Service Usage Examples
//!
//! Demonstrates usage patterns for UEFI event and timer operations through the
//! [`patina_uefi_services::service::event::EventServices`] trait.
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
//!
use core::time::Duration;
use patina::{
    boot_services::{event::EventType, tpl::Tpl},
    component::{IntoComponent, prelude::Service},
    error::Result,
};
use patina_uefi_services::service::event::EventServices;

/// Example component demonstrating basic event creation and signaling patterns.
///
/// This component creates an event, manually signals it, and waits for it to trigger.
/// It demonstrates the core event lifecycle: create, signal, wait, and close.
///
/// **Note**: Timer events require a hardware timer architecture protocol to function.
/// This example uses manual signaling which works in all environments.
#[derive(IntoComponent)]
pub struct BasicEventExample;

impl BasicEventExample {
    fn manually_signal_an_event(&self, event_services: &Service<dyn EventServices>) -> Result<()> {
        log::info!("Creating an event...");

        // Create an event with NOTIFY_WAIT flag so it can be used with wait_for_event
        let event = event_services.create_event(EventType::NOTIFY_WAIT, Tpl::NOTIFY)?;

        // Verify event is not signaled
        match event_services.check_event(event) {
            Ok(()) => log::warn!("Event unexpectedly signaled"),
            Err(_) => log::info!("Event is correctly not signaled yet"),
        }

        log::info!("Manually signaling the event...");

        // Manually signal the event (in real usage, this might be done by another component or callback)
        event_services.signal_event(event)?;

        log::info!("Waiting for the event to be signaled...");

        // Wait for the event to be signaled
        let mut events = [event];
        let index = event_services.wait_for_event(&mut events)?;

        log::info!("The event was signaled (index: {})", index);

        // Clean up the event
        event_services.close_event(event)?;

        Ok(())
    }

    fn timer_event(&self, event_services: &Service<dyn EventServices>) -> Result<()> {
        log::info!("Creating a timer event...");

        // Create a timer event at CALLBACK TPL
        let event = event_services.create_event(EventType::TIMER, Tpl::CALLBACK)?;

        // Set timer to trigger after 1 second (10,000,000 * 100ns units)
        event_services.set_timer(event, patina::boot_services::event::EventTimerType::Relative, 10_000_000)?;

        log::info!("Waiting for the timer event to trigger...");

        // Wait for the event to be signaled
        let mut events = [event];
        let index = event_services.wait_for_event(&mut events)?;

        log::info!("Timer event triggered (index: {})", index);

        Ok(())
    }

    fn entry_point(self, event_services: Service<dyn EventServices>) -> Result<()> {
        self.manually_signal_an_event(&event_services)?;
        self.timer_event(&event_services)?;

        Ok(())
    }
}

/// Example component demonstrating multiple event handling and polling patterns.
///
/// Shows how to create multiple events, poll their status without blocking,
/// and wait for any one of them to signal.
///
/// **Note**: This example uses timer events which require a hardware timer architecture
/// protocol to be registered in the platform. Without it, `wait_for_event` will hang
/// because timer ticks never occur to signal the events.
#[derive(IntoComponent)]
pub struct MultiEventExample;

impl MultiEventExample {
    fn entry_point(self, event_services: Service<dyn EventServices>) -> Result<()> {
        log::info!("Creating multiple timer events...");

        // Create three timer events with different intervals
        // IMPORTANT: Timer events that will be waited on need the NOTIFY_WAIT flag
        let event1 = event_services.create_event(EventType::TIMER | EventType::NOTIFY_WAIT, Tpl::CALLBACK)?;
        let event2 = event_services.create_event(EventType::TIMER | EventType::NOTIFY_WAIT, Tpl::CALLBACK)?;
        let event3 = event_services.create_event(EventType::TIMER | EventType::NOTIFY_WAIT, Tpl::CALLBACK)?;

        // Set different timer intervals (100ms, 200ms, 300ms)
        event_services.set_timer(
            event1,
            patina::boot_services::event::EventTimerType::Relative,
            1_000_000, // 100ms
        )?;
        event_services.set_timer(
            event2,
            patina::boot_services::event::EventTimerType::Relative,
            2_000_000, // 200ms
        )?;
        event_services.set_timer(
            event3,
            patina::boot_services::event::EventTimerType::Relative,
            3_000_000, // 300ms
        )?;

        // Check event status without blocking
        match event_services.check_event(event1) {
            Ok(()) => log::info!("Event 1 already signaled"),
            Err(_) => log::info!("Event 1 not yet signaled"),
        }

        // Wait for the first event to trigger
        let mut events = [event1, event2, event3];
        let index = event_services.wait_for_event(&mut events)?;

        log::info!("First event to trigger: index {}", index);

        // Clean up all events
        event_services.close_event(event1)?;
        event_services.close_event(event2)?;
        event_services.close_event(event3)?;

        log::info!("Multi-event example completed successfully");
        Ok(())
    }
}

/// Example component demonstrating a system event (Exit Boot Services) notification.
///
/// System events are automatically signaled by the firmware at specific milestones
/// (e.g., ExitBootServices, SetVirtualAddressMap).
#[derive(IntoComponent)]
pub struct FirmwareEventExample;

impl FirmwareEventExample {
    fn entry_point(self, event_services: Service<dyn EventServices>) -> Result<()> {
        log::info!("Creating a system notification event...");

        // Create an event with the signal flag
        let ebs_event = event_services
            .create_system_event(EventType::NOTIFY_SIGNAL | EventType::SIGNAL_EXIT_BOOT_SERVICES, Tpl::NOTIFY)?;

        log::info!("Event created successfully: {:?}", ebs_event);
        log::info!("This event will be automatically signaled when ExitBootServices is called");

        // Check the current state
        match event_services.check_event(ebs_event) {
            Ok(()) => log::info!("Exit Boot Services event is already signaled"),
            Err(_) => log::info!("Exit Boot Services event is not yet signaled (expected before ExitBootServices)"),
        }

        // Clean up
        event_services.close_event(ebs_event)?;

        log::info!("System event example completed successfully");
        Ok(())
    }
}

/// Example component demonstrating duration-based timer operations.
///
/// - Shows how to use the EventServices with Duration for convenient time-based operations.
/// - Demonstrates manual timer creation using different duration units.
#[derive(IntoComponent)]
pub struct DurationBasedTimerExample;

impl DurationBasedTimerExample {
    fn entry_point(self, event_services: Service<dyn EventServices>) -> Result<()> {
        log::info!("Starting duration-based timer example...");

        // Example 1: Create a timer using Duration (100ms)
        log::info!("Creating 100ms timer using Duration...");
        let event1 = event_services.create_event(EventType::TIMER | EventType::NOTIFY_WAIT, Tpl::CALLBACK)?;
        let duration1 = Duration::from_millis(100);
        let timer_100ns = duration1.as_nanos() / 100; // Convert to 100ns units
        event_services.set_timer(event1, patina::boot_services::event::EventTimerType::Relative, timer_100ns as u64)?;

        let mut events = [event1];
        event_services.wait_for_event(&mut events)?;
        log::info!("100ms timer completed");
        event_services.close_event(event1)?;

        // Example 2: Create a timer using microseconds (50ms = 50,000μs)
        log::info!("Creating 50ms timer using microseconds...");
        let event2 = event_services.create_event(EventType::TIMER | EventType::NOTIFY_WAIT, Tpl::CALLBACK)?;
        let duration2 = Duration::from_micros(50_000);
        let timer_100ns = duration2.as_nanos() / 100;
        event_services.set_timer(event2, patina::boot_services::event::EventTimerType::Relative, timer_100ns as u64)?;

        let mut events = [event2];
        event_services.wait_for_event(&mut events)?;
        log::info!("50ms timer completed");
        event_services.close_event(event2)?;

        // Example 3: Very short timer using microseconds (10ms = 10,000μs)
        log::info!("Creating 10ms timer...");
        let event3 = event_services.create_event(EventType::TIMER | EventType::NOTIFY_WAIT, Tpl::CALLBACK)?;
        let duration3 = Duration::from_micros(10_000);
        let timer_100ns = duration3.as_nanos() / 100;
        event_services.set_timer(event3, patina::boot_services::event::EventTimerType::Relative, timer_100ns as u64)?;

        let mut events = [event3];
        event_services.wait_for_event(&mut events)?;
        log::info!("10ms timer completed");
        event_services.close_event(event3)?;

        log::info!("Duration-based timer example completed successfully");
        log::info!("Note: UEFI timers operate in 100ns units as per the UEFI Specification");
        Ok(())
    }
}

/// Example component demonstrating periodic timer events.
///
/// Shows how to set up a periodic timer that triggers repeatedly at regular intervals,
/// useful for polling operations or periodic maintenance tasks.
#[derive(IntoComponent)]
pub struct PeriodicTimerExample;

impl PeriodicTimerExample {
    fn entry_point(self, event_services: Service<dyn EventServices>) -> Result<()> {
        log::info!("Creating periodic timer event...");

        // Create a timer event
        // IMPORTANT: Timer events that will be waited on need the NOTIFY_WAIT flag
        let event = event_services.create_event(EventType::TIMER | EventType::NOTIFY_WAIT, Tpl::CALLBACK)?;

        // Set as periodic timer (triggers every 500ms)
        event_services.set_timer(event, patina::boot_services::event::EventTimerType::Periodic, 5_000_000)?;

        log::info!("Periodic timer set to trigger every 500ms");

        // Wait for the timer to trigger 3 times
        for i in 1..=3 {
            let mut events = [event];
            event_services.wait_for_event(&mut events)?;
            log::info!("Periodic timer triggered (iteration {})", i);
        }

        // Cancel the periodic timer
        log::info!("Canceling periodic timer...");
        event_services.set_timer(event, patina::boot_services::event::EventTimerType::Cancel, 0)?;

        // Verify timer is canceled by checking if event is signaled
        match event_services.check_event(event) {
            Ok(()) => log::info!("Event still signaled after cancel (will clear on next wait)"),
            Err(_) => log::info!("Event not signaled after cancel"),
        }

        // Close the event now that the demo is done
        event_services.close_event(event)?;

        log::info!("Periodic timer example completed successfully");
        Ok(())
    }
}
