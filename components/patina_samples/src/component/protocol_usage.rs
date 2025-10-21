//! Protocol Service Usage Examples
//!
//! Demonstrates usage patterns for UEFI protocol operations through the
//! [`patina_uefi_services::service::protocol::ProtocolServices`] trait.
//!
//! This example uses a test protocol to demonstrate protocol installation, lookup,
//! and uninstallation without interfering with real protocols on the system.
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
//!
extern crate alloc;

use alloc::boxed::Box;
use core::ffi::c_void;
use patina::{
    BinaryGuid,
    boot_services::protocol_handler::HandleSearchType,
    component::{IntoComponent, prelude::Service},
    error::Result,
};
use patina_uefi_services::{service::protocol::ProtocolServices, types::Handle};
use r_efi::efi;

// Define a test protocol GUID that won't conflict (ideally) with other protocols
// Using a custom GUID: {12345678-1234-5678-9ABC-DEF012345678}
static TEST_PROTOCOL_GUID: BinaryGuid =
    BinaryGuid::from_fields(0x12345678, 0x1234, 0x5678, 0x9A, 0xBC, &[0xDE, 0xF0, 0x12, 0x34, 0x56, 0x78]);

/// Test protocol interface structure.
///
/// A simple protocol interface for demonstration purposes.
#[repr(C)]
struct TestProtocol {
    revision: u32,
    get_value: extern "efiapi" fn(*const TestProtocol) -> u32,
    set_value: extern "efiapi" fn(*mut TestProtocol, u32),
    value: u32,
}

impl TestProtocol {
    /// Creates a new test protocol instance.
    fn new(initial_value: u32) -> Self {
        extern "efiapi" fn get_value(this: *const TestProtocol) -> u32 {
            // SAFETY: The caller must ensure that the `this` pointer points to a valid TestProtocol when called via
            // protocol interface. In this case, the pointer is properly aligned as it originates from a
            // Box<TestProtocol>.
            unsafe { (*this).value }
        }

        extern "efiapi" fn set_value(this: *mut TestProtocol, new_value: u32) {
            // SAFETY: The caller must ensure that the `this` pointer points to a valid, mutable TestProtocol when
            // called. In this case, the pointer is properly aligned and exclusively owned during the call.
            unsafe { (*this).value = new_value }
        }

        Self { revision: 1, get_value, set_value, value: initial_value }
    }
}

/// Example component demonstrating basic protocol installation and lookup.
///
/// Shows how to install a protocol on a handle, locate it, and clean it up properly.
#[derive(IntoComponent)]
pub struct BasicProtocolExample;

impl BasicProtocolExample {
    fn entry_point(self, protocol_services: Service<dyn ProtocolServices>) -> Result<()> {
        log::info!("Starting basic protocol example...");

        // Create a test protocol instance on the heap
        let mut test_protocol = Box::new(TestProtocol::new(42));
        let protocol_ptr = test_protocol.as_mut() as *mut TestProtocol as *mut c_void;

        // Create a new handle and install the protocol
        let mut handle = Handle::null();
        log::info!("Installing test protocol on new handle...");

        // SAFETY: We own the protocol instance (via Box), the pointer is valid and properly aligned.
        // The interface type matches the protocol (NATIVE_INTERFACE for standard protocols).
        // The protocol will remain valid until we explicitly uninstall it.
        unsafe {
            protocol_services.install_protocol_interface(
                &mut handle,
                &TEST_PROTOCOL_GUID,
                efi::NATIVE_INTERFACE,
                protocol_ptr,
            )?;
        }

        log::info!("Protocol installed on handle: {:?}", handle);

        // Verify that the protocol can be found on the handle
        log::info!("Querying handle for protocol...");

        // SAFETY: The handle contains the installed protocol. The pointer is casted to match the original
        // type. The protocol remains valid as we haven't uninstalled it.
        let found_ptr = unsafe { protocol_services.handle_protocol(handle, &TEST_PROTOCOL_GUID)? };

        if found_ptr == protocol_ptr {
            log::info!("Successfully located protocol on handle");
        } else {
            log::warn!("Protocol pointer mismatch!");
        }

        // Clean up: uninstall the protocol
        log::info!("Uninstalling protocol...");

        // SAFETY: The protocol pointer matches what was installed, the handle is valid, and no other
        // code is using the protocol (we're about to drop it).
        unsafe {
            protocol_services.uninstall_protocol_interface(handle, &TEST_PROTOCOL_GUID, protocol_ptr)?;
        }

        log::info!("Protocol uninstalled successfully");

        // Cleanup the protocol instance (Box will drop it)
        drop(test_protocol);

        log::info!("Basic protocol example completed successfully");
        Ok(())
    }
}

/// Example component demonstrating protocol location across multiple handles.
///
/// Shows how to install protocols on multiple handles and locate all instances.
#[derive(IntoComponent)]
pub struct MultiHandleProtocolExample;

impl MultiHandleProtocolExample {
    fn entry_point(self, protocol_services: Service<dyn ProtocolServices>) -> Result<()> {
        log::info!("Starting multi-handle protocol example...");

        // Create multiple protocol instances
        let mut protocol1 = Box::new(TestProtocol::new(100));
        let mut protocol2 = Box::new(TestProtocol::new(200));
        let mut protocol3 = Box::new(TestProtocol::new(300));

        let ptr1 = protocol1.as_mut() as *mut TestProtocol as *mut c_void;
        let ptr2 = protocol2.as_mut() as *mut TestProtocol as *mut c_void;
        let ptr3 = protocol3.as_mut() as *mut TestProtocol as *mut c_void;

        // Install protocols on separate handles
        let mut handle1 = Handle::null();
        let mut handle2 = Handle::null();
        let mut handle3 = Handle::null();

        log::info!("Installing test protocols on three handles...");

        // SAFETY: Each protocol instance is owned via Box and remains valid until uninstalled.
        // Each pointer is properly aligned and points to a valid TestProtocol structure.
        // The interface type matches the protocol requirements.
        unsafe {
            protocol_services.install_protocol_interface(
                &mut handle1,
                &TEST_PROTOCOL_GUID,
                efi::NATIVE_INTERFACE,
                ptr1,
            )?;
            protocol_services.install_protocol_interface(
                &mut handle2,
                &TEST_PROTOCOL_GUID,
                efi::NATIVE_INTERFACE,
                ptr2,
            )?;
            protocol_services.install_protocol_interface(
                &mut handle3,
                &TEST_PROTOCOL_GUID,
                efi::NATIVE_INTERFACE,
                ptr3,
            )?;
        }

        log::info!("Protocols installed on handles: {:?}, {:?}, {:?}", handle1, handle2, handle3);

        // Locate all handles with our test protocol
        log::info!("Locating all handles with test protocol...");
        let handles = protocol_services.locate_handle_buffer(HandleSearchType::ByProtocol(&TEST_PROTOCOL_GUID))?;

        log::info!("Found {} handles with test protocol", handles.len());

        // Verify we found our handles
        let mut found_count = 0;
        for handle in &handles {
            if *handle == handle1 || *handle == handle2 || *handle == handle3 {
                found_count += 1;
                log::info!("  Found expected handle: {:?}", handle);
            }
        }

        if found_count >= 3 {
            log::info!("Successfully located all installed protocols");
        } else {
            log::warn!("Only found {} of 3 expected handles", found_count);
        }

        // Clean up all protocols
        log::info!("Cleaning up all protocols...");

        // SAFETY: Each pointer matches what was installed on the corresponding handle.
        unsafe {
            protocol_services.uninstall_protocol_interface(handle1, &TEST_PROTOCOL_GUID, ptr1)?;
            protocol_services.uninstall_protocol_interface(handle2, &TEST_PROTOCOL_GUID, ptr2)?;
            protocol_services.uninstall_protocol_interface(handle3, &TEST_PROTOCOL_GUID, ptr3)?;
        }

        // Drop protocol instances
        drop(protocol1);
        drop(protocol2);
        drop(protocol3);

        log::info!("Multi-handle protocol example completed successfully");
        Ok(())
    }
}

/// Example component demonstrating protocol location without prior knowledge of handles.
///
/// Shows how to use `locate_protocol` to find the first instance of a protocol.
#[derive(IntoComponent)]
pub struct LocateProtocolExample;

impl LocateProtocolExample {
    fn entry_point(self, protocol_services: Service<dyn ProtocolServices>) -> Result<()> {
        log::info!("Starting locate protocol example...");

        // Create and install a test protocol
        let mut test_protocol = Box::new(TestProtocol::new(777));
        let protocol_ptr = test_protocol.as_mut() as *mut TestProtocol as *mut c_void;

        let mut handle = Handle::null();
        log::info!("Installing test protocol...");

        // SAFETY: Protocol instance is owned via Box, pointer is valid and properly aligned.
        // The interface type matches the protocol. Protocol remains valid until uninstalled.
        unsafe {
            protocol_services.install_protocol_interface(
                &mut handle,
                &TEST_PROTOCOL_GUID,
                efi::NATIVE_INTERFACE,
                protocol_ptr,
            )?;
        }

        log::info!("Protocol installed on handle: {:?}", handle);

        // Locate the protocol without knowing the handle
        log::info!("Locating protocol by GUID...");

        // SAFETY: We search for an installed protocol by GUID. The returned pointer will be valid
        // as long as the protocol remains installed (which it is until we uninstall it).
        let located_ptr = unsafe { protocol_services.locate_protocol(&TEST_PROTOCOL_GUID, None)? };

        // Verify we got the right protocol
        if located_ptr == protocol_ptr {
            log::info!("Successfully located protocol instance");

            // Access the protocol
            // SAFETY: The pointer came from locate_protocol, points to our valid TestProtocol,
            // is properly aligned, and the protocol is still installed so the data is valid.
            let protocol_ref = unsafe { &*(located_ptr as *const TestProtocol) };
            let value = (protocol_ref.get_value)(protocol_ref);
            log::info!("Protocol value: {}", value);

            if value == 777 {
                log::info!("Protocol contains expected value");
            }
        } else {
            log::warn!("Located protocol pointer does not match installed pointer");
        }

        // Clean up
        log::info!("Uninstalling protocol...");

        // SAFETY: The protocol pointer matches what we installed. Handle and GUID are valid.
        // No other code is using the protocol as we're about to drop it.
        unsafe {
            protocol_services.uninstall_protocol_interface(handle, &TEST_PROTOCOL_GUID, protocol_ptr)?;
        }

        drop(test_protocol);

        log::info!("Locate protocol example completed successfully");
        Ok(())
    }
}
