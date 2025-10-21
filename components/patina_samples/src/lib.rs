//! Sample Patina Components
//!
//! This crate provides example component implementations demonstrating various
//! Patina component patterns and usage models.
//!
//! ## Examples
//!
//! ### Basic Components
//!
//! - [`component::hello_world::HelloStruct`]: Demonstrates a struct-based component with default entry point
//! - [`component::hello_world::GreetingsEnum`]: Demonstrates an enum-based component with custom entry point
//!
//! ### Service Usage
//!
//! #### Event Services
//!
//! - [`component::event_usage::BasicEventExample`]: Basic event creation, timer configuration, and cleanup
//! - [`component::event_usage::MultiEventExample`]: Multiple event handling and polling patterns
//!
//! #### Protocol Services
//!
//! - [`component::protocol_usage::BasicProtocolExample`]: Protocol installation and lookup on a handle
//! - [`component::protocol_usage::MultiHandleProtocolExample`]: Installing protocols across multiple handles
//! - [`component::protocol_usage::LocateProtocolExample`]: Locating protocols without prior handle knowledge
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
//!
#![cfg_attr(not(feature = "std"), no_std)]
#![feature(coverage_attribute)]
#![coverage(off)]

pub mod component;
