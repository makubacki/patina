//! Image Services Component Implementation
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0

use crate::{service::image::ImageServices, types::Handle};
use alloc::vec::Vec;
use patina::{
    boot_services::{BootServices, StandardBootServices},
    component::{IntoComponent, params::Commands, service::IntoService},
    error::{EfiError, Result},
};
use r_efi::efi;

/// Standard implementation of image services using UEFI Boot Services.
#[derive(IntoService)]
#[service(dyn ImageServices)]
pub struct StandardImageServices {
    boot_services: StandardBootServices,
}

impl StandardImageServices {
    /// Creates a new `StandardImageServices` instance.
    ///
    /// # Arguments
    ///
    /// * `boot_services` - The underlying boot services implementation
    pub fn new(boot_services: StandardBootServices) -> Self {
        Self { boot_services }
    }
}

impl ImageServices for StandardImageServices {
    unsafe fn load_image(
        &self,
        parent_image_handle: Handle,
        device_path: *mut efi::protocols::device_path::Protocol,
        source_buffer: Option<Vec<u8>>,
        _source_size: usize,
    ) -> Result<Handle> {
        // SAFETY: Caller guarantees that the device_path pointer is valid and properly formatted.
        // We delegate to the underlying boot services which performs the actual unsafe operation.
        // Convert Vec<u8> to &[u8] for the underlying service
        let source_ref = source_buffer.as_deref();
        self.boot_services
            .load_image(true, parent_image_handle.as_raw(), device_path, source_ref)
            .map(Handle::new)
            .map_err(EfiError::from)
    }

    fn start_image(&self, image_handle: Handle) -> Result<Vec<u8>> {
        match self.boot_services.start_image(image_handle.as_raw()) {
            Ok(()) => Ok(Vec::new()),
            Err((status, _exit_data)) => {
                // If there's exit data, convert it to Vec<u8> and return it with error
                // For now, we'll treat any start_image error as an EfiError
                // The exit data, if present, could be extracted but for simplicity
                // we'll focus on the error status
                Err(EfiError::from(status))
            }
        }
    }

    fn exit(&self, image_handle: Handle, exit_status: efi::Status, _exit_data: Option<Vec<u8>>) -> Result<()> {
        // For simplicity, we don't currently support exit data in the high-level service
        // Pass None to the underlying boot services for now
        self.boot_services.exit(image_handle.as_raw(), exit_status, None).map_err(EfiError::from)
    }

    fn unload_image(&self, image_handle: Handle) -> Result<()> {
        self.boot_services.unload_image(image_handle.as_raw()).map_err(EfiError::from)
    }
}

/// Component that provides `ImageServices` to the system.
///
/// This component registers the `StandardImageServices` implementation,
/// making image operations available to other Patina Components.
#[derive(IntoComponent)]
pub struct ImageServicesProvider;

impl ImageServicesProvider {
    /// Entry point for the `ImageServicesProvider` component.
    ///
    /// This registers the `StandardImageServices` implementation, making it available
    /// to other components that depend on `ImageServices`.
    pub fn entry_point(self, mut commands: Commands, boot_services: StandardBootServices) -> Result<()> {
        let image_services = StandardImageServices::new(boot_services);
        commands.add_service(image_services);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_image_services_creation() {
        let boot_services = StandardBootServices::new_uninit();
        let _image_services = StandardImageServices::new(boot_services);
    }

    #[test]
    fn test_image_services_provider_creation() {
        let _provider = ImageServicesProvider;
    }
}
