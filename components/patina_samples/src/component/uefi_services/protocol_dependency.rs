//! Protocol Dependency Sample Component
//!
//! `Protocol<P>` lets a component express a dispatch dependency on a specific UEFI protocol
//! directly in its `entry_point` signature. It is used to gate dispatch. The component is not
//! dispatched until the protocol is installed, but the parameter does not give access to the
//! interface. The interface is located through `Service<dyn ProtocolServices>` (see
//! [`super::protocol_consumer`] for ways to access the protocol interface).
//!
//! [`ProtocolDependencySample`] shows a hard protocol dependency as it does not run until the protocol
//! is installed. [`OptionalProtocolDependencySample`] wraps the parameter in `Option` so the component
//! runs immediately either way, reading the protocol only when it was already installed at dispatch.
//!
//! Both samples consume [`SampleVendorProtocol`], published by [`super::protocol_publisher`].
//!
//! Note: If you are creating new components that do not have a pre-existing requirement to use protocols,
//! Patina services should be used for sharing functionality instead of protocols.
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
//!
use patina::{
    component::{
        component,
        protocol::Protocol,
        service::{
            Service,
            uefi_services::protocol::{ProtocolServices, ProtocolServicesExt},
        },
    },
    error::Result,
};

use super::protocol_publisher::SampleVendorProtocol;

/// Only dispatched once [`SampleVendorProtocol`] is installed.
#[derive(Default)]
pub struct ProtocolDependencySample;

#[component]
impl ProtocolDependencySample {
    /// Creates a new instance of the component.
    pub fn new() -> Self {
        Self
    }

    fn entry_point(
        self,
        _svp: Protocol<SampleVendorProtocol>,
        protocols: Service<dyn ProtocolServices>,
    ) -> Result<()> {
        // The dispatcher only calls this entry point once the protocol is confirmed installed,
        // but the interface is still located through the protocol service.
        let interface = protocols.locate_protocol::<SampleVendorProtocol>()?;
        let status = (interface.get_status)();
        log::info!(
            "Dispatched now that SampleVendorProtocol (rev {:#x}) is installed, status is {status:#x}",
            interface.revision
        );
        Ok(())
    }
}

/// This component is always dispatched immediately. It only reads [`SampleVendorProtocol`]
/// if it is already installed.
#[derive(Default)]
pub struct OptionalProtocolDependencySample;

#[component]
impl OptionalProtocolDependencySample {
    /// Creates a new instance of the component.
    pub fn new() -> Self {
        Self
    }

    fn entry_point(
        self,
        installed: Option<Protocol<SampleVendorProtocol>>,
        protocols: Service<dyn ProtocolServices>,
    ) -> Result<()> {
        // Unlike `Protocol<P>` by itself, `Option<Protocol<P>>` does not block dispatch: this component
        // runs immediately, and `installed` is `None` until SampleVendorProtocol is installed.
        match installed {
            Some(_) => {
                let status = (protocols.locate_protocol::<SampleVendorProtocol>()?.get_status)();
                log::info!("SampleVendorProtocol is already installed, status is {status:#x}");
            }
            None => log::debug!("SampleVendorProtocol is not installed yet, continuing without it"),
        }
        Ok(())
    }
}
