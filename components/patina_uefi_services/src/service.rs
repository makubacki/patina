//! Patina UEFI Services
//!
//! This crate provides an abstraction to "UEFI services" for Patina components. The goal of these abstractions is to
//! provide safe, idiomatic access to UEFI Services, so even if the underlying details of how the UEFI services are
//! implemented change over time, Patina components can use a stable, well-defined interface.
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0

pub mod console;
pub mod event;
pub mod image;
pub mod misc;
pub mod protocol;
pub mod runtime;
pub mod system_table;
pub mod variable;
