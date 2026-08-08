//! A wait-free, write-once cell for sharing a value with code that cannot receive it through
//! dependency injection.
//!
//! [`Service<T>`](super::Service) and other [`Param`](crate::component::params::Param) types are retrieved through
//! the component dispatcher, which requires a `Storage` to resolve from. Some code has no such context to work
//! with. Two cases come up in practice:
//!
//! - An `extern "efiapi"` function backing an EDK II protocol, whose signature is fixed by the protocol definition
//!   and carries no context/`this` parameter to route a value through.
//! - A `struct` or `static` that must exist before the dependency injection system does, such as one built with a
//!   `const fn` constructor so it can be a top-level `static`. The dependency is only available later, once a
//!   component's entry point runs and resolves it.
//!
//! [`ServiceCell<T>`] handles both cases. A component's entry point (or an init method it calls) publishes a value
//! into a `static ServiceCell`, or a `ServiceCell` field, and the context-free function or not-yet-ready struct
//! reads it back later through [`ServiceCell::get`], which returns `None` until the value is published.
//!
//! ## Avoiding `spin::Once`, `TplMutex`, or a plain `Mutex`
//!
//! See [Synchronization](https://github.com/OpenDevicePartnership/patina/blob/main/docs/src/dxe_core/synchronization.md)
//! for Patina's main synchronization design principles. The explanation below briefly states why `ServiceCell<T>` is
//! designed to be "wait-free".
//!
//! - Non-TPL-aware spinning primitives (`spin::mutex`, `spin::rwlock`, and `spin::Once`) are prone to TPL-inversion
//!   deadlocks in UEFI code. If a lower-TPL context is interrupted by a higher-TPL context that spins waiting for it,
//!   the lower-TPL context can never resume to unblock it. `spin::Once::call_once` blocks the caller if another call
//!   is in progress. [`ServiceCell::publish`] never blocks or spins and returns the rejected value immediately.
//! - `TplMutex` requires choosing a fixed TPL ceiling that no caller may exceed. While some services have TPL caller
//!   restrictions defined in the UEFI Specification, many do not. And because many protocols are invoked by C code
//!   outside of Patina, a TPL ceiling is difficult to enforce in practice.
//!
//! [`ServiceCell<T>`] avoids these problems by being "wait-free". Every operation completes in a bounded number of
//! steps using atomic operations.
//!
//! ## Example
//!
//! ```rust
//! use patina::component::service::cell::ServiceCell;
//!
//! static CURRENT: ServiceCell<u32> = ServiceCell::new();
//!
//! // From a component's entry point, once the real value is available:
//! CURRENT.publish(42).expect("published exactly once");
//!
//! // From a context-free callback, later:
//! assert_eq!(CURRENT.get(), Some(&42));
//! ```
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
//!
use core::{
    cell::OnceCell,
    sync::atomic::{AtomicBool, Ordering},
};

/// A wait-free, write-once-then-read-many cell.
///
/// See the [module documentation](self) for the motivation and the reasoning behind its design.
pub struct ServiceCell<T> {
    value: OnceCell<T>,
    writing: AtomicBool,
}

// SAFETY: All writes to `value` go through `publish`, which is serialized by a `compare_exchange` on `writing` so
// that at most one writer ever calls `OnceCell::set`. Reads through `get` only observe `value` after confirming (with
//  an `Acquire` load of `writing`) that no write is in progress, which paired with the `Release` store at the end of
// `publish`, establishes a happens-before relationship with the completed write. `T: Sync` is required because
// `get` can hand out `&T` to multiple contexts. `T: Send` is required because the value can be published from one
// context and read from another.
unsafe impl<T: Send + Sync> Sync for ServiceCell<T> {}

impl<T> ServiceCell<T> {
    /// Creates a new, empty cell.
    pub const fn new() -> Self {
        Self { value: OnceCell::new(), writing: AtomicBool::new(false) }
    }

    /// Publishes `value` into the cell.
    ///
    /// Never blocks. If another call to `publish` is in progress, or the cell is already published, `value` is
    /// returned back to the caller immediately rather than waiting.
    ///
    /// # Errors
    ///
    /// Returns `Err(value)` if the cell was already published, or another call to `publish` is currently in
    /// progress.
    pub fn publish(&self, value: T) -> Result<(), T> {
        if self.writing.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_ok() {
            let result = self.value.set(value);
            self.writing.store(false, Ordering::Release);
            return result;
        }
        Err(value)
    }

    /// Returns a reference to the published value, if any.
    ///
    /// Never blocks. RReturns `None` both when nothing has been published yet and while a concurrent call to
    /// `publish` is in progress.
    pub fn get(&self) -> Option<&T> {
        if !self.writing.load(Ordering::Acquire) { self.value.get() } else { None }
    }

    /// Returns `true` if a value has been published.
    pub fn is_published(&self) -> bool {
        self.get().is_some()
    }
}

impl<T> Default for ServiceCell<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_cell_get_before_publish_returns_none() {
        let cell: ServiceCell<u32> = ServiceCell::new();
        assert_eq!(cell.get(), None);
        assert!(!cell.is_published());
    }

    #[test]
    fn test_service_cell_publish_then_get() {
        let cell = ServiceCell::new();
        assert_eq!(cell.publish(7u32), Ok(()));
        assert_eq!(cell.get(), Some(&7));
        assert!(cell.is_published());
    }

    #[test]
    fn test_service_cell_double_publish_returns_value() {
        let cell = ServiceCell::new();
        assert_eq!(cell.publish(1u32), Ok(()));
        assert_eq!(cell.publish(2u32), Err(2));
        // The first published value is unchanged.
        assert_eq!(cell.get(), Some(&1));
    }

    #[test]
    fn test_service_cell_get_returns_none_while_write_in_progress() {
        let cell: ServiceCell<u32> = ServiceCell::new();
        // Simulate another context that has won the publish race but hasn't finished yet.
        cell.writing.store(true, Ordering::Release);
        assert_eq!(cell.get(), None);
        assert!(!cell.is_published());
    }

    #[test]
    fn test_service_cell_publish_returns_value_while_write_in_progress() {
        let cell: ServiceCell<u32> = ServiceCell::new();
        cell.writing.store(true, Ordering::Release);
        assert_eq!(cell.publish(5u32), Err(5));
    }

    #[test]
    fn test_service_cell_default_is_empty() {
        let cell: ServiceCell<u32> = ServiceCell::default();
        assert_eq!(cell.get(), None);
    }
}
