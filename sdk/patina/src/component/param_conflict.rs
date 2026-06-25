//! Shared conflict rules for component parameters.
//!
//! This module defines the set of rules used to decide whether two component parameters
//! can coexist. It is duplicated verbatim between runtime checks in the `patina` crate
//! and compile-time checks in the `patina_macro` crate.
//!
//! The crates cannot share a library because `patina_macro` is a proc-macro crate and
//! `patina` depends on it.
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0

use alloc::string::String;

/// The kinds of component parameters that participate in conflict detection.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParamKind {
    Config,
    ConfigMut,
    Storage,
    StorageMut,
    Commands,
    BootServices,
    RuntimeServices,
    Other,
}

/// Returns the message describing why parameters of kind `a` and `b` cannot coexist, or
/// `None` if they may be used together.
///
/// `same_resource` indicates whether the two parameters target the same underlying
/// resource (the same config type). `ty` is the concrete config type involved in the
/// conflict, used to make the message specific. It falls back to `T` when no concrete
/// type is available.
pub(crate) fn conflict(a: ParamKind, b: ParamKind, same_resource: bool, ty: Option<&str>) -> Option<String> {
    use ParamKind::*;
    let ty = ty.unwrap_or("T");
    match (a, b) {
        (ConfigMut, ConfigMut) if same_resource => {
            Some(alloc::format!("Each ConfigMut<{ty}> type can only appear once in a component's entry point."))
        }
        (Config, ConfigMut) | (ConfigMut, Config) if same_resource => {
            Some(alloc::format!("You cannot have both Config<{ty}> and ConfigMut<{ty}> for the same type."))
        }
        (StorageMut, Config) | (Config, StorageMut) | (StorageMut, ConfigMut) | (ConfigMut, StorageMut) => Some(
            alloc::format!("You cannot use &mut Storage together with Config<{ty}> or ConfigMut<{ty}> parameters."),
        ),
        (Storage, ConfigMut) | (ConfigMut, Storage) => {
            Some(alloc::format!("You cannot use &Storage together with ConfigMut<{ty}> parameters."))
        }
        (Commands, Commands) => Some(String::from("Only one Commands parameter is allowed.")),
        (BootServices, BootServices) => Some(String::from("Only one StandardBootServices parameter is allowed.")),
        (RuntimeServices, RuntimeServices) => {
            Some(String::from("Only one StandardRuntimeServices parameter is allowed."))
        }
        (StorageMut, Storage) | (Storage, StorageMut) => {
            Some(String::from("You cannot use &mut Storage together with &Storage parameters."))
        }
        (StorageMut, StorageMut) => Some(String::from("Only one &mut Storage parameter is allowed.")),
        _ => None,
    }
}

#[cfg(test)]
#[coverage(off)]
mod tests {
    use super::{ParamKind::*, *};

    #[test]
    fn duplicate_config_mut_conflicts_only_for_same_type() {
        assert!(conflict(ConfigMut, ConfigMut, true, Some("u32")).is_some());
        assert!(conflict(ConfigMut, ConfigMut, false, Some("u32")).is_none());
    }

    #[test]
    fn config_and_config_mut_conflict_only_for_same_type() {
        assert!(conflict(Config, ConfigMut, true, Some("u32")).is_some());
        assert!(conflict(ConfigMut, Config, true, Some("u32")).is_some());
        assert!(conflict(Config, ConfigMut, false, Some("u32")).is_none());
    }

    #[test]
    fn duplicate_config_is_allowed() {
        assert!(conflict(Config, Config, true, Some("u32")).is_none());
    }

    #[test]
    fn storage_mut_conflicts_with_config_and_config_mut() {
        assert!(conflict(StorageMut, Config, false, Some("u32")).is_some());
        assert!(conflict(Config, StorageMut, false, Some("u32")).is_some());
        assert!(conflict(StorageMut, ConfigMut, false, Some("u32")).is_some());
        assert!(conflict(ConfigMut, StorageMut, false, Some("u32")).is_some());
    }

    #[test]
    fn storage_conflicts_with_config_mut_but_not_config() {
        assert!(conflict(Storage, ConfigMut, false, Some("u32")).is_some());
        assert!(conflict(ConfigMut, Storage, false, Some("u32")).is_some());
        assert!(conflict(Storage, Config, false, Some("u32")).is_none());
        assert!(conflict(Config, Storage, false, Some("u32")).is_none());
    }

    #[test]
    fn duplicate_storage_is_allowed() {
        assert!(conflict(Storage, Storage, false, None).is_none());
    }

    #[test]
    fn storage_mut_conflicts_with_storage_and_itself() {
        assert!(conflict(StorageMut, Storage, false, None).is_some());
        assert!(conflict(Storage, StorageMut, false, None).is_some());
        assert!(conflict(StorageMut, StorageMut, false, None).is_some());
    }

    #[test]
    fn duplicate_singletons_conflict() {
        assert!(conflict(Commands, Commands, false, None).is_some());
        assert!(conflict(BootServices, BootServices, false, None).is_some());
        assert!(conflict(RuntimeServices, RuntimeServices, false, None).is_some());
    }

    #[test]
    fn other_never_conflicts() {
        assert!(conflict(Other, Other, true, Some("u32")).is_none());
        assert!(conflict(Other, ConfigMut, true, Some("u32")).is_none());
        assert!(conflict(Config, Other, true, Some("u32")).is_none());
    }

    #[test]
    fn message_includes_concrete_type_with_t_fallback() {
        assert_eq!(
            conflict(ConfigMut, ConfigMut, true, Some("TestType")).unwrap(),
            "Each ConfigMut<TestType> type can only appear once in a component's entry point."
        );
        assert_eq!(
            conflict(ConfigMut, ConfigMut, true, None).unwrap(),
            "Each ConfigMut<T> type can only appear once in a component's entry point."
        );
    }
}
