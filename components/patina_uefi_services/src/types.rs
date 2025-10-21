//! Type wrappers for UEFI FFI types
//!
//! This module provides idiomatic Rust wrappers around raw UEFI FFI types from r_efi.
//!
//! These wrappers provide a cleaner interface for Pure Rust code while maintaining
//! zero-cost abstractions over the underlying FFI types.
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0

use core::fmt;
use r_efi::efi;

/// A handle to a UEFI device or protocol.
///
/// This is a type-safe wrapper around the raw `r_efi::efi::Handle` type.
/// Handles are used throughout UEFI to identify devices, images, and protocol instances.
///
/// # Examples
///
/// ```rust
/// use patina_uefi_services::types::Handle;
///
/// let handle = Handle::null();
/// assert!(handle.is_null());
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Handle(efi::Handle);

impl Handle {
    /// Creates a new Handle from a raw EFI handle.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use patina_uefi_services::types::Handle;
    /// use core::ptr;
    ///
    /// let raw_handle = ptr::null_mut();
    /// let handle = Handle::new(raw_handle);
    /// ```
    #[inline]
    pub const fn new(handle: efi::Handle) -> Self {
        Self(handle)
    }

    /// Creates a null Handle.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use patina_uefi_services::types::Handle;
    ///
    /// let handle = Handle::null();
    /// assert!(handle.is_null());
    /// ```
    #[inline]
    pub const fn null() -> Self {
        Self(core::ptr::null_mut())
    }

    /// Returns true if this handle is null.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use patina_uefi_services::types::Handle;
    ///
    /// let handle = Handle::null();
    /// assert!(handle.is_null());
    /// ```
    #[inline]
    pub const fn is_null(self) -> bool {
        self.0.is_null()
    }

    /// Returns the raw EFI handle.
    ///
    /// This is useful when you need to pass the handle to FFI functions.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use patina_uefi_services::types::Handle;
    /// use core::ptr;
    ///
    /// let handle = Handle::null();
    /// assert_eq!(handle.as_raw(), ptr::null_mut());
    /// ```
    #[inline]
    pub const fn as_raw(self) -> efi::Handle {
        self.0
    }
}

impl fmt::Debug for Handle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_null() { f.write_str("Handle(null)") } else { write!(f, "Handle({:p})", self.0) }
    }
}

impl fmt::Display for Handle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_null() { f.write_str("null") } else { write!(f, "{:p}", self.0) }
    }
}

impl From<efi::Handle> for Handle {
    #[inline]
    fn from(handle: efi::Handle) -> Self {
        Self::new(handle)
    }
}

impl From<Handle> for efi::Handle {
    #[inline]
    fn from(handle: Handle) -> Self {
        handle.as_raw()
    }
}

impl AsRef<efi::Handle> for Handle {
    #[inline]
    fn as_ref(&self) -> &efi::Handle {
        &self.0
    }
}

/// A handle to a UEFI event.
///
/// This is a type-safe wrapper around the raw `r_efi::efi::Event` type.
/// Events are used for asynchronous notifications and timer operations in UEFI.
///
/// # Examples
///
/// ```rust
/// use patina_uefi_services::types::Event;
///
/// let event = Event::null();
/// assert!(event.is_null());
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Event(efi::Event);

impl Event {
    /// Creates a new Event from a raw EFI event.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use patina_uefi_services::types::Event;
    /// use core::ptr;
    ///
    /// let raw_event = ptr::null_mut();
    /// let event = Event::new(raw_event);
    /// ```
    #[inline]
    pub const fn new(event: efi::Event) -> Self {
        Self(event)
    }

    /// Creates a null Event.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use patina_uefi_services::types::Event;
    ///
    /// let event = Event::null();
    /// assert!(event.is_null());
    /// ```
    #[inline]
    pub const fn null() -> Self {
        Self(core::ptr::null_mut())
    }

    /// Returns true if this event is null.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use patina_uefi_services::types::Event;
    ///
    /// let event = Event::null();
    /// assert!(event.is_null());
    /// ```
    #[inline]
    pub const fn is_null(self) -> bool {
        self.0.is_null()
    }

    /// Returns the raw EFI event.
    ///
    /// This is useful when you need to pass the event to FFI functions.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use patina_uefi_services::types::Event;
    /// use core::ptr;
    ///
    /// let event = Event::null();
    /// assert_eq!(event.as_raw(), ptr::null_mut());
    /// ```
    #[inline]
    pub const fn as_raw(self) -> efi::Event {
        self.0
    }
}

impl fmt::Debug for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_null() { f.write_str("Event(null)") } else { write!(f, "Event({:p})", self.0) }
    }
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_null() { f.write_str("null") } else { write!(f, "{:p}", self.0) }
    }
}

impl From<efi::Event> for Event {
    #[inline]
    fn from(event: efi::Event) -> Self {
        Self::new(event)
    }
}

impl From<Event> for efi::Event {
    #[inline]
    fn from(event: Event) -> Self {
        event.as_raw()
    }
}

impl AsRef<efi::Event> for Event {
    #[inline]
    fn as_ref(&self) -> &efi::Event {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;
    use core::ptr;

    #[test]
    fn test_handle_null() {
        let handle = Handle::null();
        assert!(handle.is_null());
        assert_eq!(handle.as_raw(), ptr::null_mut());
    }

    #[test]
    fn test_handle_non_null() {
        let raw = 0x1234 as efi::Handle;
        let handle = Handle::new(raw);
        assert!(!handle.is_null());
        assert_eq!(handle.as_raw(), raw);
    }

    #[test]
    fn test_handle_conversion() {
        let raw = 0x5678 as efi::Handle;
        let handle = Handle::from(raw);
        let converted_back: efi::Handle = handle.into();
        assert_eq!(raw, converted_back);
    }

    #[test]
    fn test_handle_clone() {
        let handle1 = Handle::new(0x1234 as efi::Handle);
        let handle2 = handle1;
        assert_eq!(handle1, handle2);
    }

    #[test]
    fn test_handle_debug() {
        let handle = Handle::null();
        let debug_str = format!("{:?}", handle);
        assert_eq!(debug_str, "Handle(null)");
    }

    #[test]
    fn test_event_null() {
        let event = Event::null();
        assert!(event.is_null());
        assert_eq!(event.as_raw(), ptr::null_mut());
    }

    #[test]
    fn test_event_non_null() {
        let raw = 0xABCD as efi::Event;
        let event = Event::new(raw);
        assert!(!event.is_null());
        assert_eq!(event.as_raw(), raw);
    }

    #[test]
    fn test_event_conversion() {
        let raw = 0xDEAD as efi::Event;
        let event = Event::from(raw);
        let converted_back: efi::Event = event.into();
        assert_eq!(raw, converted_back);
    }

    #[test]
    fn test_event_clone() {
        let event1 = Event::new(0xBEEF as efi::Event);
        let event2 = event1;
        assert_eq!(event1, event2);
    }

    #[test]
    fn test_event_debug() {
        let event = Event::null();
        let debug_str = format!("{:?}", event);
        assert_eq!(debug_str, "Event(null)");
    }
}
