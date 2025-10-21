//! UEFI Services Provider Component
//!
//! This component provides all UEFI services by registering the appropriate service implementations.
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0

use crate::component::misc::{
    StandardConfigurationServices, StandardMemoryUtilityServices, StandardSystemUtilityServices, StandardTimingServices,
};
use patina::{
    boot_services::StandardBootServices,
    component::{IntoComponent, params::Commands},
    error::Result,
};

/// Component that provides all UEFI Services.
#[derive(IntoComponent)]
pub struct UefiServicesProvider;

impl UefiServicesProvider {
    /// Entry point for the UEFI Services provider component.
    ///
    /// This registers most UEFI service implementations, making them available
    /// to other components that depend on UEFI services.
    ///
    /// Note: The following services are not registered here and installed in a dedicated component: Console, Event,
    /// Image, Protocol, Runtime, System Table, Variable
    ///
    /// - Memory services are provided by `MemoryServicesProvider` using `MemoryManager`.
    pub fn entry_point(self, mut commands: Commands, boot_services: StandardBootServices) -> Result<()> {
        // Create and register timing services
        let timing_services = StandardTimingServices::new(boot_services.clone());
        commands.add_service(timing_services);

        // Create and register memory utility services
        let memory_utility_services = StandardMemoryUtilityServices::new(boot_services.clone());
        commands.add_service(memory_utility_services);

        // Create and register system utility services
        let system_utility_services = StandardSystemUtilityServices::new(boot_services.clone());
        commands.add_service(system_utility_services);

        // Create and register the configuration services
        let configuration_services = StandardConfigurationServices::new(boot_services);
        commands.add_service(configuration_services);

        Ok(())
    }
}
