//! Protocol Services Abstraction
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
//!

use crate::types::Handle;
use alloc::vec::Vec;
use core::ffi::c_void;
use patina::{BinaryGuid, boot_services::protocol_handler::HandleSearchType, error::Result};
use r_efi::efi;

#[cfg(any(test, feature = "mockall"))]
use mockall::automock;

/// Protocol management operations abstraction.
///
/// It is recommended Patina components do not use protocols directly at all. A Patina service should be added instead
/// and Patina Components depend on the service. In the case the protocol must be used or a Patina component is wrapping
/// protocol access to provide a service, this service group provides protocol access to components.
#[cfg_attr(any(test, feature = "mockall"), automock)]
pub trait ProtocolServices {
    /// Installs a protocol interface on a device handle.
    ///
    /// Creates a new handle or adds a protocol to an existing handle.
    /// This is typically used by drivers to publish their services.
    ///
    /// # Arguments
    ///
    /// * `handle` - Mutable reference to handle (null creates new handle)
    /// * `protocol` - GUID of the protocol being installed (must be a static constant)
    /// * `interface_type` - Type of interface (typically `NATIVE_INTERFACE`)
    /// * `interface` - Pointer to the protocol interface structure
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    /// - The `interface` pointer is valid and points to a properly initialized protocol interface structure
    /// - The interface structure matches the type expected for the given protocol GUID
    /// - The interface structure remains valid for the lifetime it will be installed
    /// - The interface pointer is properly aligned for the protocol type
    ///
    /// # Notes
    ///
    /// This is a low-level interface for protocol installation. For type-safe installation,
    /// prefer using the higher-level `BootServices` methods when available.
    unsafe fn install_protocol_interface(
        &self,
        handle: &mut Handle,
        protocol: &'static BinaryGuid,
        interface_type: efi::InterfaceType,
        interface: *mut c_void,
    ) -> Result<()>;

    /// Removes a protocol interface from a device handle.
    ///
    /// Uninstalls a previously installed protocol interface.
    ///
    /// # Arguments
    ///
    /// * `handle` - Handle containing the protocol to remove
    /// * `protocol` - GUID of the protocol to remove (must be a static constant)
    /// * `interface` - Pointer to the interface that was installed
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    /// - The `interface` pointer matches the exact pointer that was used during installation
    /// - The handle and protocol combination is valid
    /// - No other code is currently using the protocol interface being removed
    unsafe fn uninstall_protocol_interface(
        &self,
        handle: Handle,
        protocol: &'static BinaryGuid,
        interface: *mut c_void,
    ) -> Result<()>;

    /// Queries a handle to determine if it supports a specified protocol.
    ///
    /// Returns a pointer to the protocol interface if found.
    ///
    /// # Arguments
    ///
    /// * `handle` - Handle to query
    /// * `protocol` - GUID of the protocol to find (must be a static constant)
    ///
    /// # Returns
    ///
    /// Pointer to the protocol interface if found.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    /// - The returned pointer is cast to the correct protocol interface type matching the GUID
    /// - The protocol interface is not used after the handle is destroyed or the protocol is uninstalled
    /// - The protocol interface structure is accessed according to its defined memory layout
    ///
    /// # Notes
    ///
    /// For type-safe protocol access, prefer using the higher-level `BootServices` methods when available.
    unsafe fn handle_protocol(&self, handle: Handle, protocol: &'static BinaryGuid) -> Result<*mut c_void>;

    /// Locates the first instance of a protocol.
    ///
    /// Finds the first protocol instance in the system that matches the GUID.
    ///
    /// # Arguments
    ///
    /// * `protocol` - GUID of the protocol to locate (must be a static constant)
    /// * `registration` - Optional registration key from protocol notifications
    ///
    /// # Returns
    ///
    /// Pointer to the first matching protocol interface.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    /// - The returned pointer is cast to the correct protocol interface type matching the GUID
    /// - The protocol interface is not used after it has been uninstalled from all handles
    /// - If `registration` is provided, it must be a valid registration key from a protocol notification
    /// - The protocol interface structure is accessed according to its defined memory layout
    unsafe fn locate_protocol(
        &self,
        protocol: &'static BinaryGuid,
        registration: Option<*mut c_void>,
    ) -> Result<*mut c_void>;

    /// Returns an array of handles that support the requested protocol.
    ///
    /// Finds all handles in the system that have the specified protocol installed.
    ///
    /// # Arguments
    ///
    /// * `search_type` - Type of search to perform (AllHandle, ByProtocol, etc.)
    ///
    /// # Returns
    ///
    /// Vector of handles that match the search criteria.
    fn locate_handle_buffer(&self, search_type: HandleSearchType) -> Result<Vec<Handle>>;
}
