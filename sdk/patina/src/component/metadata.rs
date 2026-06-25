//! Component metadata.
//!
//! The metadata is used by the scheduler for multiple purposes including, but not limited to:
//! - Managing access requirements for components.
//! - Logging and debugging.
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
use alloc::{borrow::Cow, string::String, vec::Vec};

use crate::component::param_conflict::{ParamKind, conflict};

/// Metadata for a component. Not used for execution, but referenced by the scheduler.
#[derive(Default, Debug)]
pub struct MetaData {
    /// The read/write parameter access requirements for the component.
    access: Access,
    /// The name of the component.
    name: Cow<'static, str>,
    /// The error message preventing the component from being dispatched.
    error_message: Option<Cow<'static, str>>,
}

impl MetaData {
    /// Creates a new metadata object for a component.
    pub fn new<S>() -> Self {
        Self { access: Access::new(), name: Cow::from(super::type_name::normalized::<S>()), error_message: None }
    }

    /// Returns the name of the component, including the module path.
    #[inline(always)]
    pub fn name(&self) -> Cow<'static, str> {
        self.name.clone()
    }

    /// Sets the name of the `param` that could not be retrieved from storage when attempting to dispatch the function.
    #[inline(always)]
    pub fn set_error_message(&mut self, error: Cow<'static, str>) {
        self.error_message = Some(error);
    }

    /// Returns the name of the last `param` that could not be retrieved from storage.
    #[inline(always)]
    pub fn error_message(&self) -> Option<Cow<'static, str>> {
        self.error_message.clone()
    }

    /// Returns mutable access to the param usage metadata for the component.
    #[inline(always)]
    pub(crate) fn access_mut(&mut self) -> &mut Access {
        &mut self.access
    }
}

/// Access requirements for a component.
///
/// Records the conflict-relevant parameters registered during component initialization so
/// that incompatible combinations can be rejected.
#[derive(Default, Debug)]
pub struct Access {
    /// Registered parameters as `(kind, config id, config type name)`. The config id and
    /// type name identify the resource for `Config`/`ConfigMut` parameters and are `None`
    /// for all other kinds.
    params: Vec<(ParamKind, Option<usize>, Option<String>)>,
}

impl Access {
    /// Creates a new `Access` instance with no registered parameters.
    pub const fn new() -> Self {
        Self { params: Vec::new() }
    }

    /// Registers a parameter of the given `kind`, returning an error message if it conflicts
    /// with a previously registered parameter.
    ///
    /// `config_id` and `type_name` identify the config resource for `Config`/`ConfigMut`
    /// parameters and are `None` for all other kinds. `type_name` is used to make conflict
    /// messages refer to the concrete type.
    pub fn register(
        &mut self,
        kind: ParamKind,
        config_id: Option<usize>,
        type_name: Option<&str>,
    ) -> Result<(), Cow<'static, str>> {
        for (prior_kind, prior_id, prior_type) in &self.params {
            let same_resource = config_id.is_some() && config_id == *prior_id;
            let ty = type_name.or(prior_type.as_deref());
            if let Some(message) = conflict(kind, *prior_kind, same_resource, ty) {
                return Err(Cow::from(message));
            }
        }
        self.params.push((kind, config_id, type_name.map(String::from)));
        Ok(())
    }
}

#[cfg(test)]
#[coverage(off)]
mod tests {
    use super::*;

    #[test]
    fn test_register_allows_independent_params() {
        let mut access = Access::new();
        assert!(access.register(ParamKind::Config, Some(0), Some("u32")).is_ok());
        assert!(access.register(ParamKind::Config, Some(1), Some("i32")).is_ok());
        assert!(access.register(ParamKind::Commands, None, None).is_ok());
    }

    #[test]
    fn test_register_detects_duplicate_config_mut() {
        let mut access = Access::new();
        assert!(access.register(ParamKind::ConfigMut, Some(0), Some("u32")).is_ok());
        assert!(access.register(ParamKind::ConfigMut, Some(0), Some("u32")).is_err());
        // A different config resource does not conflict.
        assert!(access.register(ParamKind::ConfigMut, Some(1), Some("i32")).is_ok());
    }

    #[test]
    fn test_register_detects_config_and_config_mut_same_resource() {
        let mut access = Access::new();
        assert!(access.register(ParamKind::Config, Some(0), Some("u32")).is_ok());
        assert!(access.register(ParamKind::ConfigMut, Some(0), Some("u32")).is_err());
    }

    #[test]
    fn test_register_detects_duplicate_singletons() {
        let mut access = Access::new();
        assert!(access.register(ParamKind::Commands, None, None).is_ok());
        assert!(access.register(ParamKind::Commands, None, None).is_err());
    }
}
