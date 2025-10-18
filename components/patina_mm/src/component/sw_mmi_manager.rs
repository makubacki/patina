//! Software Management Mode (MM) Interrupt Component
//!
//! Provides the `SwMmiTrigger` service to trigger software management mode interrupts (SWMMIs) in the MM environment.
//!
//! ## Logging
//!
//! Detailed logging is available for this component using the `sw_mmi` log target.
//!
//! ## License
//!
//! Copyright (C) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
//!
use crate::config::{MmCommunicationConfiguration, MmiPort};
use crate::service::platform_mm_control::PlatformMmControl;
use patina::component::{
    IntoComponent,
    params::{Commands, Config},
    service::{IntoService, Service},
};

#[cfg(any(feature = "doc", all(target_os = "uefi", target_arch = "x86_64")))]
use x86_64::instructions::port;

#[cfg(any(test, feature = "mockall"))]
use mockall::automock;

/// Software Management Mode (MM) Interrupt Trigger Service
///
/// Provides a mechanism to trigger software management mode interrupts (MMIs) in the MM environment. These are
/// synchronous interrupts that can be used to signal MM handlers to perform specific tasks or operations usually
/// invoking a specific MM handler registered to handle MMI requests from a correspnding driver or component outside
/// of the MM environment.
///
/// ## Safety
///
/// This trait is unsafe because an implementation needs to:
///
/// 1. Ensure that the platform hardware is capable of handling MMIs.
/// 2. Ensure that the service is only invoked after hardware initialization for MMIs is complete and that the
///    system is in a safe state to handle MMIs.
#[cfg_attr(any(test, feature = "mockall"), automock)]
pub unsafe trait SwMmiTrigger {
    /// Triggers a software Management Mode Interrupt (MMI).
    fn trigger_sw_mmi(&self, cmd_port_value: u8, data_port_value: u8) -> patina::error::Result<()>;
}

/// A component that provides the `SwMmiTrigger` service.
#[derive(Debug, IntoComponent, IntoService)]
#[service(dyn SwMmiTrigger)]
pub struct SwMmiManager {
    inner_config: MmCommunicationConfiguration,
}

impl SwMmiManager {
    /// Create a new `SwMmiManager` instance.
    pub fn new() -> Self {
        Self { inner_config: MmCommunicationConfiguration::default() }
    }

    /// Initialize the `SwMmiManager` instance.
    ///
    /// Sets up the `SwMmiManager` with the provided configuration and registers it as a service. This function expects
    /// the platform to have initialized the MM environment prior to its execution. The platform may optionally provide
    /// a `PlatformMmControl` service that will be invoked before this component makes the `SwMmiTrigger` service
    /// available.
    fn entry_point(
        mut self,
        config: Config<MmCommunicationConfiguration>,
        platform_mm_control: Option<Service<dyn PlatformMmControl>>,
        mut commands: Commands,
    ) -> patina::error::Result<()> {
        log::info!(target: "sw_mmi", "Initializing SwMmiManager...");
        log::debug!(target: "sw_mmi", "MM config - cmd_port: {:?}, data_port: {:?}, acpi_base: {:?}",
            config.cmd_port, config.data_port, config.acpi_base);

        if platform_mm_control.is_some() {
            log::debug!(target: "sw_mmi", "Platform MM Control is available. Calling platform-specific init...");
            platform_mm_control.unwrap().init().inspect_err(|&err| {
                log::error!(target: "sw_mmi", "Platform MM Control initialization failed: {:?}", err);
            })?;
            log::trace!(target: "sw_mmi", "Platform MM Control initialization completed successfully");
        } else {
            log::trace!(target: "sw_mmi", "No platform MM Control service available - using default initialization");
        }

        self.inner_config = config.clone();
        log::debug!(target: "sw_mmi", "SwMmiManager configuration applied successfully");

        commands.add_service(self);
        log::info!(target: "sw_mmi", "SwMmiManager service registered and ready");

        Ok(())
    }
}

// SAFETY: SwMmiManager does not produce the SwMmiTrigger service until its entry point has executed after the
//         platform has published MM configuration and had an opportunity to provide a platform-specific MM control
//         service.
unsafe impl SwMmiTrigger for SwMmiManager {
    fn trigger_sw_mmi(&self, _cmd_port_value: u8, _data_port_value: u8) -> patina::error::Result<()> {
        log::debug!(target: "sw_mmi", "Triggering SW MMI with cmd_port_value=0x{:02X}, data_port_value=0x{:02X}", _cmd_port_value, _data_port_value);

        log::trace!(target: "sw_mmi", "Writing to MMI command port...");
        match self.inner_config.cmd_port {
            MmiPort::Smi(_port) => {
                log::trace!(target: "sw_mmi", "Using SMI command port: 0x{:04X}", _port);
                cfg_if::cfg_if! {
                    if #[cfg(any(feature = "doc", all(target_os = "uefi", target_arch = "x86_64")))] {
                        log::trace!(target: "sw_mmi", "Writing SMI command port: {_port:#X}");
                        unsafe { port::Port::new(_port).write(_cmd_port_value); }
                        log::trace!(target: "sw_mmi", "SMI command port write completed");
                    } else {
                        log::trace!(target: "sw_mmi", "SMI command port write skipped (not on target platform)");
                    }
                }
            }
            MmiPort::Smc(_smc_port) => {
                log::warn!(target: "sw_mmi", "SMC communication not implemented yet for port: 0x{:08X}", _smc_port);
                todo!("SMC communication not implemented yet.");
            }
        }

        log::trace!(target: "sw_mmi", "Writing to MMI data port...");
        match self.inner_config.data_port {
            MmiPort::Smi(_port) => {
                log::trace!(target: "sw_mmi", "Using SMI data port: 0x{:04X}", _port);
                cfg_if::cfg_if! {
                    if #[cfg(any(feature = "doc", all(target_os = "uefi", target_arch = "x86_64")))] {
                        log::trace!(target: "sw_mmi", "Writing SMI data port: {_port:#X}");
                        unsafe { port::Port::new(_port).write(_data_port_value); }
                        log::trace!(target: "sw_mmi", "SMI data port write completed");
                    } else {
                        log::trace!(target: "sw_mmi", "SMI data port write skipped (not on target platform)");
                    }
                }
            }
            MmiPort::Smc(_smc_port) => {
                log::warn!(target: "sw_mmi", "SMC communication not implemented yet for port: 0x{:08X}", _smc_port);
                todo!("SMC communication not implemented yet.");
            }
        }

        log::debug!(target: "sw_mmi", "SW MMI triggered successfully");
        Ok(())
    }
}

impl Default for SwMmiManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[coverage(off)]
mod tests {
    use super::*;
    use crate::config::MmCommunicationConfiguration;
    use crate::service::platform_mm_control::{MockPlatformMmControl, PlatformMmControl};
    use patina::component::params::Commands;

    #[test]
    fn test_sw_mmi_manager_without_platform_mm_control() {
        let sw_mmi_manager = SwMmiManager::new();
        assert!(
            sw_mmi_manager
                .entry_point(Config::mock(MmCommunicationConfiguration::default()), None, Commands::mock())
                .is_ok()
        );
    }

    #[test]
    fn test_sw_mmi_manager_with_platform_mm_control() {
        let sw_mmi_manager = SwMmiManager::new();

        let mut mock_platform_mm_control = MockPlatformMmControl::new();
        mock_platform_mm_control.expect_init().once().returning(|| Ok(()));
        let platform_mm_control_service: Service<dyn PlatformMmControl> =
            Service::mock(Box::new(mock_platform_mm_control));

        assert!(
            sw_mmi_manager
                .entry_point(
                    Config::mock(MmCommunicationConfiguration::default()),
                    Some(platform_mm_control_service),
                    Commands::mock()
                )
                .is_ok()
        );
    }
}
