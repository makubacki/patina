//! Image Services Abstraction
//!
//! Trait definitions for UEFI image related operations.
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0

use crate::types::Handle;
use alloc::vec::Vec;
use patina::error::Result;
use r_efi::efi;

#[cfg(any(test, feature = "mockall"))]
use mockall::automock;

/// Image loading and execution operations.
///
/// # Note
///
/// Because the concept of "image services" is inherently tied to supporting Platform Initialization (PI) Spec based
/// images, this service interface does accept a Device Path protocol directly as opposed to an abstraction. As the
/// abstraction is expected to not be needed in a Pure Rust Patina-based dispatch process.
#[cfg_attr(any(test, feature = "mockall"), automock)]
pub trait ImageServices {
    /// Loads the image described by the given Device Path protocol instance into memory.
    ///
    /// # Arguments
    /// * `parent_image_handle` - Handle of the parent image that is loading this image
    /// * `device_path` - The device path from which to load the image
    /// * `source_buffer` - Optional buffer containing the image data to load
    /// * `source_size` - Size of the source buffer
    ///
    /// # Returns
    /// * `Result<Handle>` - Handle to the loaded image on success
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    /// - The `device_path` pointer, if not null, points to a valid and properly formatted UEFI Device Path Protocol structure
    /// - The device path structure remains valid for the duration of this call
    /// - If `source_buffer` is provided, `source_size` accurately reflects the buffer's length
    /// - The device path is properly null-terminated according to UEFI Device Path Protocol specifications
    unsafe fn load_image(
        &self,
        parent_image_handle: Handle,
        device_path: *mut efi::protocols::device_path::Protocol,
        source_buffer: Option<Vec<u8>>,
        source_size: usize,
    ) -> Result<Handle>;

    /// Transfers control to a loaded image's entry point.
    ///
    /// Starts execution of a previously loaded image. If the image returns
    /// with exit data, it will be included in the result.
    ///
    /// # Arguments
    ///
    /// * `image_handle` - Handle to the image to start
    ///
    /// # Returns
    ///
    /// Empty Vec if successful with no exit data, or Vec containing exit data
    /// if the image provided exit information.
    fn start_image(&self, image_handle: Handle) -> Result<Vec<u8>>;

    /// Terminates a loaded EFI image and returns control to boot services.
    ///
    /// Used by a loaded image to exit and return control to the entity that
    /// started it. Can optionally provide exit data.
    ///
    /// # Arguments
    ///
    /// * `image_handle` - Handle of the currently running image
    /// * `exit_status` - Exit status to return
    /// * `exit_data` - Optional exit data to provide to caller
    ///
    /// # Returns
    ///
    /// This function typically does not return as it transfers control.
    fn exit(&self, image_handle: Handle, exit_status: efi::Status, exit_data: Option<Vec<u8>>) -> Result<()>;

    /// Unloads an image from memory.
    ///
    /// Unloads a previously loaded image, freeing its memory and resources.
    /// The image must not be currently executing.
    ///
    /// # Arguments
    ///
    /// * `image_handle` - Handle to the image to unload
    ///
    /// # Returns
    ///
    /// Success if the image was unloaded successfully.
    fn unload_image(&self, image_handle: Handle) -> Result<()>;
}
