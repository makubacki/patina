//! Image Services Usage Examples
//!
//! This module demonstrates how to use ImageServices for loading and executing UEFI images.
//!
//! ## Examples Included
//!
//! - **LoadImageFromBuffer**: Loading an image from memory buffer
//! - **LoadAndExecuteImage**: Loading and executing an image
//!
//! ## Safety Note
//!
//! Image services involve loading and executing code, which inherently requires
//! unsafe operations. These examples demonstrate safe patterns for using these services
//! but they are minimal examples of simply using the interfaces available.
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0

extern crate alloc;

use alloc::vec::Vec;
use patina::{
    component::{IntoComponent, service::Service},
    error::Result,
};
use patina_uefi_services::{service::image::ImageServices, types::Handle};

/// Example component demonstrating loading an image from a memory buffer.
///
/// This component shows how to load a UEFI image that is already present
/// in memory (as opposed to loading from a device path).
#[derive(IntoComponent)]
pub struct LoadImageFromBuffer;

impl LoadImageFromBuffer {
    /// Component entry point that loads an image from a buffer.
    ///
    /// # Arguments
    ///
    /// * `image_services` - The image services for loading images
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error status
    pub fn entry_point(self, image_services: Service<dyn ImageServices>) -> Result<()> {
        // In a real scenario, you would have actual image data here
        // For this example, just the API usage pattern is demonstrated

        // Get the current image handle to use as parent
        // (In practice, this would come from your component's context)
        let parent_handle = Handle::null();

        // Example: Load an image from a buffer
        // In a real implementation, image_buffer would contain a valid PE/COFF image
        let image_buffer: Vec<u8> = Vec::new();
        let buffer_size = image_buffer.len();

        // SAFETY: We're passing a null device path and empty buffer for demonstration.
        //
        // In a real implementation, you would either provide:
        // 1. A valid device path (device_path != null) with source_buffer = None, OR
        // 2. A null device path with a valid image buffer in source_buffer
        let _loaded_image_handle = unsafe {
            image_services.load_image(
                parent_handle,
                core::ptr::null_mut(), // No device path (loading from buffer)
                Some(image_buffer),
                buffer_size,
            )
        };

        // In a real component, you would check the result and handle the loaded image
        // For this example, we're just demonstrating the API pattern

        Ok(())
    }
}

/// Example component demonstrating loading and executing an image.
///
/// This component shows the complete workflow of loading an image and
/// then starting its execution.
#[derive(IntoComponent)]
pub struct LoadAndExecuteImage;

impl LoadAndExecuteImage {
    /// Component entry point that loads and executes an image.
    ///
    /// # Arguments
    ///
    /// * `image_services` - The image services for loading and executing images
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error status
    pub fn entry_point(self, image_services: Service<dyn ImageServices>) -> Result<()> {
        // Get parent image handle (in practice, from component context)
        let parent_handle = Handle::null();

        // Example image buffer (in practice, this would be a valid UEFI image)
        let image_buffer: Vec<u8> = Vec::new();
        let buffer_size = image_buffer.len();

        // Step 1: Load the image
        // SAFETY: See safety note in LoadImageFromBuffer
        let loaded_image_handle = unsafe {
            image_services.load_image(parent_handle, core::ptr::null_mut(), Some(image_buffer), buffer_size)?
        };

        // Step 2: Start the image
        // This transfers control to the loaded image's entry point
        let exit_data = image_services.start_image(loaded_image_handle)?;

        // Step 3: Handle exit data if any was provided
        if !exit_data.is_empty() {
            // Process exit data from the image
            // In a real component, you would interpret this data based on
            // your specific requirements
        }

        Ok(())
    }
}
