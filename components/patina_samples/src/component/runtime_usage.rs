//! Runtime Services Usage Examples
//!
//! This module demonstrates how to use UEFI Runtime Services including time/date management
//! and system reset operations.
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0

extern crate alloc;

use patina::{component::IntoComponent, component::prelude::Service, error::Result};
use patina_uefi_services::service::runtime::{RuntimeResetServices, RuntimeTimeServices};
use r_efi::{efi, system};

//
// Runtime Time Services Examples
//

/// Reading the current system time.
///
/// This component demonstrates how to read the current time and date from
/// the system's real-time clock (RTC).
///
/// ## Example Usage
///
/// ```ignore
/// use patina::component::IntoComponent;
/// use patina_samples::component::runtime_usage::ReadSystemTimeExample;
///
/// // Register the component with the core
/// dxe_core.with_component(ReadSystemTimeExample);
/// ```
#[derive(IntoComponent)]
pub struct ReadSystemTimeExample;

impl ReadSystemTimeExample {
    fn entry_point(self, time: Service<dyn RuntimeTimeServices>) -> Result<()> {
        // Get the current system time
        let current_time = time.get_time()?;

        // The time structure contains:
        // - Year, Month, Day
        // - Hour, Minute, Second, Nanosecond
        // - TimeZone, Daylight savings information

        // Example: Access time components
        let _year = current_time.year;
        let _month = current_time.month;
        let _day = current_time.day;
        let _hour = current_time.hour;
        let _minute = current_time.minute;
        let _second = current_time.second;

        Ok(())
    }
}

/// Setting the system time.
///
/// This component demonstrates how to update the system's real-time clock
/// with a new time and date.
///
/// ## Example Usage
///
/// ```ignore
/// use patina::component::IntoComponent;
/// use patina_samples::component::runtime_usage::SetSystemTimeExample;
///
/// // Register the component with the core
/// dxe_core.with_component(SetSystemTimeExample);
/// ```
#[derive(IntoComponent)]
pub struct SetSystemTimeExample;

impl SetSystemTimeExample {
    fn entry_point(self, time: Service<dyn RuntimeTimeServices>) -> Result<()> {
        // Create a new time structure for January 1, 2025, 12:00:00
        let new_time = system::Time {
            year: 2025,
            month: 1,
            day: 1,
            hour: 12,
            minute: 0,
            second: 0,
            nanosecond: 0,
            timezone: efi::UNSPECIFIED_TIMEZONE,
            daylight: 0,
            pad1: 0,
            pad2: 0,
        };

        // Set the system time
        time.set_time(&new_time)?;

        // Verify the time was set
        let current_time = time.get_time()?;
        assert_eq!(current_time.year, 2025);
        assert_eq!(current_time.month, 1);

        Ok(())
    }
}

/// Managing wakeup alarm timers.
///
/// This component demonstrates how to configure and query the system's
/// wakeup alarm timer functionality.
///
/// ## Example Usage
///
/// ```ignore
/// use patina::component::IntoComponent;
/// use patina_samples::component::runtime_usage::WakeupAlarmExample;
///
/// // Register the component with the core
/// dxe_core.with_component(WakeupAlarmExample);
/// ```
#[derive(IntoComponent)]
pub struct WakeupAlarmExample;

impl WakeupAlarmExample {
    fn entry_point(self, time: Service<dyn RuntimeTimeServices>) -> Result<()> {
        // Query the current wakeup alarm status
        let (enabled, pending, _alarm_time) = time.get_wakeup_time()?;

        // Check if alarm is enabled and pending
        let _is_active = enabled && pending;

        // Set a new wakeup alarm for tomorrow at 6:00 AM
        let wakeup_time = system::Time {
            year: 2025,
            month: 1,
            day: 2,
            hour: 6,
            minute: 0,
            second: 0,
            nanosecond: 0,
            timezone: efi::UNSPECIFIED_TIMEZONE,
            daylight: 0,
            pad1: 0,
            pad2: 0,
        };

        time.set_wakeup_time(true, Some(wakeup_time))?;

        // Disable the wakeup alarm
        time.set_wakeup_time(false, None)?;

        Ok(())
    }
}

//
// Runtime Reset Services Examples
//

/// Performing a cold system reset.
///
/// This component demonstrates how to perform a cold reset of the entire system.
///
/// ## Safety Note
///
/// This function never returns on success - the system will reset immediately.
/// Ensure all critical data is saved before calling this function.
///
/// ## Example Usage
///
/// ```ignore
/// use patina::component::IntoComponent;
/// use patina_samples::component::runtime_usage::ColdResetExample;
///
/// // Register the component with the core
/// dxe_core.with_component(ColdResetExample);
/// ```
#[derive(IntoComponent)]
pub struct ColdResetExample;

impl ColdResetExample {
    fn entry_point(self, reset: Service<dyn RuntimeResetServices>) -> Result<()> {
        // Perform a cold reset
        // Note: In production firmware, this never returns!
        #[cfg(test)]
        {
            reset.reset_system(system::RESET_COLD, efi::Status::SUCCESS, None)?;
            Ok(())
        }

        #[cfg(not(test))]
        reset.reset_system(system::RESET_COLD, efi::Status::SUCCESS, None)
    }
}

/// Performing a warm system reset.
///
/// This component demonstrates how to perform a warm reset, which is faster
/// than a cold reset and may preserve some system state.
///
/// ## Safety Note
///
/// This function never returns on success - the system will reset immediately.
///
/// ## Example Usage
///
/// ```ignore
/// use patina::component::IntoComponent;
/// use patina_samples::component::runtime_usage::WarmResetExample;
///
/// // Register the component with the core
/// dxe_core.with_component(WarmResetExample);
/// ```
#[derive(IntoComponent)]
pub struct WarmResetExample;

impl WarmResetExample {
    fn entry_point(self, reset: Service<dyn RuntimeResetServices>) -> Result<()> {
        // Perform a warm reset
        // Note: In production firmware, this never returns!
        #[cfg(test)]
        {
            reset.reset_system(system::RESET_WARM, efi::Status::SUCCESS, None)?;
            Ok(())
        }

        #[cfg(not(test))]
        reset.reset_system(system::RESET_WARM, efi::Status::SUCCESS, None)
    }
}

/// System shutdown operation.
///
/// This component demonstrates how to shut down the system cleanly.
///
/// ## Safety Note
///
/// This function never returns on success - the system will shut down immediately.
///
/// ## Example Usage
///
/// ```ignore
/// use patina::component::IntoComponent;
/// use patina_samples::component::runtime_usage::SystemShutdownExample;
///
/// // Register the component with the core
/// dxe_core.with_component(SystemShutdownExample);
/// ```
#[derive(IntoComponent)]
pub struct SystemShutdownExample;

impl SystemShutdownExample {
    fn entry_point(self, reset: Service<dyn RuntimeResetServices>) -> Result<()> {
        // Perform system shutdown
        // Note: In production firmware, this never returns!
        #[cfg(test)]
        {
            reset.reset_system(system::RESET_SHUTDOWN, efi::Status::SUCCESS, None)?;
            Ok(())
        }

        #[cfg(not(test))]
        reset.reset_system(system::RESET_SHUTDOWN, efi::Status::SUCCESS, None)
    }
}

/// System reset with additional data.
///
/// This component demonstrates how to perform a reset with additional data
/// that can be used by the system firmware or operating system.
///
/// ## Safety Note
///
/// This function never returns on success - the system will reset immediately.
///
/// ## Example Usage
///
/// ```ignore
/// use patina::component::IntoComponent;
/// use patina_samples::component::runtime_usage::ResetWithDataExample;
///
/// // Register the component with the core
/// dxe_core.with_component(ResetWithDataExample);
/// ```
#[derive(IntoComponent)]
pub struct ResetWithDataExample;

impl ResetWithDataExample {
    fn entry_point(self, reset: Service<dyn RuntimeResetServices>) -> Result<()> {
        // Perform a reset operation
        // Note: In a real implementation, you would pass reset data here. This
        // example uses None for simplicity.
        #[cfg(test)]
        {
            reset.reset_system(system::RESET_COLD, efi::Status::SUCCESS, None)?;
            Ok(())
        }

        #[cfg(not(test))]
        reset.reset_system(system::RESET_COLD, efi::Status::SUCCESS, None)
    }
}
