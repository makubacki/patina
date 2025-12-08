//! Macros for interacting with allocators.
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0

/// Macro to iterate over all static allocators and execute an expression for each.
/// Returns `true` if any allocator returns `true` from the expression.
/// The variable `$alloc` is available in the expression and represents each allocator.
///
/// # Example
/// ```ignore
/// if for_each_static_allocator!(alloc => alloc.total_allocated() > 0) {
///     // At least one allocator has allocated memory
/// }
/// ```
#[macro_export]
macro_rules! for_each_static_allocator {
    ($alloc:ident => $action:expr) => {{
        ({
            let $alloc = &$crate::allocator::EFI_BOOT_SERVICES_DATA_ALLOCATOR;
            $action
        }) || ({
            let $alloc = &$crate::allocator::EFI_LOADER_CODE_ALLOCATOR;
            $action
        }) || ({
            let $alloc = &$crate::allocator::EFI_BOOT_SERVICES_CODE_ALLOCATOR;
            $action
        }) || ({
            let $alloc = &$crate::allocator::EFI_RUNTIME_SERVICES_CODE_ALLOCATOR;
            $action
        }) || ({
            let $alloc = &$crate::allocator::EFI_RUNTIME_SERVICES_DATA_ALLOCATOR;
            $action
        })
    }};
}

/// Macro to try an operation on each static allocator and return the first success.
/// Sets a mutable variable to the memory type if successful.
/// The variable `$alloc` is available in the expression and represents each allocator.
///
/// # Example
/// ```ignore
/// let mut memory_type = efi::BOOT_SERVICES_DATA;
/// if try_each_static_allocator!(memory_type, alloc => alloc.allocate_pages(pages)) {
///     // memory_type now contains the type of the allocator that succeeded
/// }
/// ```
#[macro_export]
macro_rules! try_each_static_allocator {
    ($memory_type_var:ident, $alloc:ident => $action:expr) => {{
        if {
            let $alloc = &$crate::allocator::EFI_BOOT_SERVICES_DATA_ALLOCATOR;
            $action.is_ok()
        } {
            $memory_type_var = r_efi::efi::BOOT_SERVICES_DATA;
            true
        } else if {
            let $alloc = &$crate::allocator::EFI_LOADER_CODE_ALLOCATOR;
            $action.is_ok()
        } {
            $memory_type_var = r_efi::efi::LOADER_CODE;
            true
        } else if {
            let $alloc = &$crate::allocator::EFI_BOOT_SERVICES_CODE_ALLOCATOR;
            $action.is_ok()
        } {
            $memory_type_var = r_efi::efi::BOOT_SERVICES_CODE;
            true
        } else if {
            let $alloc = &$crate::allocator::EFI_RUNTIME_SERVICES_CODE_ALLOCATOR;
            $action.is_ok()
        } {
            $memory_type_var = r_efi::efi::RUNTIME_SERVICES_CODE;
            true
        } else if {
            let $alloc = &$crate::allocator::EFI_RUNTIME_SERVICES_DATA_ALLOCATOR;
            $action.is_ok()
        } {
            $memory_type_var = r_efi::efi::RUNTIME_SERVICES_DATA;
            true
        } else {
            false
        }
    }};
}

/// Macro to match a memory type and execute an action on the corresponding static allocator.
/// Falls back to a default expression if the memory type doesn't match any static allocator.
///
/// # Example
/// ```ignore
/// match_static_allocator!(memory_type, alloc => alloc.get_memory_ranges().collect(), {
///     // Fallback for non-static allocators
///     Vec::new()
/// })
/// ```
#[macro_export]
macro_rules! match_static_allocator {
    ($memory_type:expr, $alloc:ident => $action:expr, $fallback:expr) => {
        match $memory_type {
            r_efi::efi::BOOT_SERVICES_DATA => {
                let $alloc = &$crate::allocator::EFI_BOOT_SERVICES_DATA_ALLOCATOR;
                $action
            }
            r_efi::efi::LOADER_CODE => {
                let $alloc = &$crate::allocator::EFI_LOADER_CODE_ALLOCATOR;
                $action
            }
            r_efi::efi::BOOT_SERVICES_CODE => {
                let $alloc = &$crate::allocator::EFI_BOOT_SERVICES_CODE_ALLOCATOR;
                $action
            }
            r_efi::efi::RUNTIME_SERVICES_CODE => {
                let $alloc = &$crate::allocator::EFI_RUNTIME_SERVICES_CODE_ALLOCATOR;
                $action
            }
            r_efi::efi::RUNTIME_SERVICES_DATA => {
                let $alloc = &$crate::allocator::EFI_RUNTIME_SERVICES_DATA_ALLOCATOR;
                $action
            }
            _ => $fallback,
        }
    };
}
