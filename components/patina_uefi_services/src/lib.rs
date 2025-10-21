//! UEFI Services Component
//!
//! ## Usage
//!
//! To use UEFI services in your component, add the relevant service as a dependency in the component entry point
//! parameter list. For example, to use console services:
//!
//! ```rust,no_run
//! use patina::component::{IntoComponent, prelude::Service};
//! use patina_uefi_services::service::console::ConsoleServices;
//! use patina::error::Result;
//!
//! #[derive(IntoComponent)]
//! struct MyComponent;
//!
//! impl MyComponent {
//!     fn entry_point(
//!         self,
//!         console: Service<dyn ConsoleServices>
//!     ) -> Result<()> {
//!         console.clear_screen()?;
//!         console.output_string("Hello from Patina!")?;
//!         console.set_cursor_position(10, 5)?;
//!         console.output_string("Positioned text!")?;
//!         Ok(())
//!     }
//! }
//! ```
//!
//! ## Integration
//!
//! To integrate this component into your Patina system, add the provider component:
//!
//! ```rust,no_run
//! use patina_uefi_services::UefiServicesProvider;
//! // In your main function or component setup:
//! let provider = UefiServicesProvider;
//! // ... register with your system ...
//! ```
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0

#![cfg_attr(all(not(feature = "std"), not(test), not(feature = "mockall")), no_std)]

extern crate alloc;

pub mod component;
pub mod service;
pub mod types;

// Re-exported for easier access
pub use component::provider::UefiServicesProvider;

#[cfg(all(test, feature = "std"))]
mod integration_tests {
    use super::*;
    use patina::{boot_services::StandardBootServices, component::params::Commands, component::prelude::Service};

    #[test]
    fn test_uefi_services_provider_integration() {
        // Test that the UefiServicesProvider can be instantiated and called
        let _provider = UefiServicesProvider;
        let _commands = Commands::mock();
        let _boot_services = Service::<StandardBootServices>::new_uninit();
        // Note: The test is considered successful if there is no panic
    }
}
