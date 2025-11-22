//! A crate containing macros to be re-exported in the `patina` crate.
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
//!

#![feature(coverage_attribute)]

mod component_macro;
mod hob_macro;
mod service_macro;
mod smbios_record_macro;
mod test_macro;
mod validate_params_macro;

/// Derive Macro for implementing the `IntoComponent` trait for a type.
///
/// This macro automatically implements the necessary traits for the provided type implementation to be used as a
/// `Component`. By default, the component attribute macro will assume a function, `Self::entry_point`, exists on the
/// type, but that can be overridden with the `entry_point` attribute.
///
/// ## Supported types
///
/// - Struct
/// - Enum
///
/// ## Macro Attribute
///
/// - `entry_point`: The function to be called when the component is executed.
///
/// ## Examples
///
/// ```rust, ignore
/// use patina::{
///     error::Result,
///     component::{
///         IntoComponent,
///         params::Config,
///     },
/// };
///
/// #[derive(IntoComponent)]
/// struct MyStruct(u32);
///
/// impl MyStruct {
///
///     fn entry_point(self, _cfg: Config<String>) -> Result<()> {
///         Ok(())
///     }
/// }
///
/// #[derive(IntoComponent)]
/// #[entry_point(path = driver)]
/// struct MyStruct2(u32);
///
/// fn driver(s: MyStruct2, _cfg: Config<String>) -> Result<()> {
///    Ok(())
/// }
///
/// #[derive(IntoComponent)]
/// #[entry_point(path = MyEnum::run_me)]
/// enum MyEnum {
///    A,
///    B,
/// }
///
/// impl MyEnum {
///    fn run_me(self, _cfg: Config<String>) -> Result<()> {
///       Ok(())
///   }
/// }
/// ```
#[proc_macro_derive(IntoComponent, attributes(entry_point, protocol))]
pub fn component(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    component_macro::component2(item.into()).into()
}

/// Component attribute macro that derives IntoComponent.
///
/// This macro provides a convenient single-attribute solution for defining components.
/// It automatically derives the IntoComponent trait for the struct or enum.
///
/// For parameter validation on the entry_point function, use `#[component_entry_point]`
/// on the entry_point function itself.
///
/// ## Usage
///
/// ```rust,ignore
/// use patina::component::prelude::*;
///
/// #[component]
/// pub struct MyComponent {
///     config: u32,
/// }
///
/// impl MyComponent {
///     #[component_entry_point]
///     fn entry_point(self, config: Config<u32>) -> Result<()> {
///         Ok(())
///     }
/// }
/// ```
///
/// ## Attributes
///
/// - `entry_point`: Override the default entry point path (optional)
///
/// ## Example with custom entry point
///
/// ```rust,ignore
/// #[component]
/// #[entry_point(path = my_custom_entry)]
/// pub struct MyComponent;
///
/// fn my_custom_entry(comp: MyComponent) -> Result<()> {
///     Ok(())
/// }
/// ```
#[proc_macro_attribute]
pub fn component_attr(attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    component_macro::component_attribute(attr.into(), item.into()).into()
}

/// Derive Macro for implementing the `IntoService` trait for a type.
///
/// This macro automatically implements the necessary traits for the provided type implementation to be used as a
/// `Service`. By default the derive macro assumes the service is the same as the deriver, but that can be overridden
/// with the `service` attribute to specify that the service is actually a dyn \<Trait\> that the underlying type
/// implements.
///
/// ## Macro Attribute
///
/// - `service`: The service trait(s) that the type implements.
/// - `protocol`: Publishes the entire struct as a protocol with the given GUID.
///
/// ## Member Attributes
///
/// - `protocol`: Publishes the field as a protocol with the given GUID.
///
/// ## Pure Rust Example
///
/// ```rust, ignore
/// use patina::{
///    error::Result,
///    component::{
///      IntoService,
///      params::Service,
///    },
/// };
///
/// trait MyService {
///   fn do_something(&self) -> Result<()>;
/// }
///
/// #[derive(IntoService)]
/// #[service(MyService)]
/// struct MyStruct;
///
/// impl MyService for MyStruct {
///   fn do_something(&self) -> Result<()> {
///    Ok(())
///   }
/// }
/// ```
#[proc_macro_derive(IntoService, attributes(service))]
pub fn service(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    service_macro::service2(item.into()).into()
}

/// Derive Macro for implementing the `HobConfig` trait for a type.
///
/// This macro uses the [zerocopy::FromBytes](https://docs.rs/zerocopy/latest/zerocopy/trait.FromBytes.html)
/// implementation to safely create an instance of the type from a byte slice. If FromBytes is not implemented on the
/// type, a compile time error will be produced.
///
/// ## Macro Attribute
///
/// - `guid`: The guid to associate with the type.
///
/// ## Examples
///
/// ```rust, ignore
/// use patina::component::FromHob;
///
/// #[derive(FromHob, zerocopy::FromBytes)]
/// #[guid = "8be4df61-93ca-11d2-aa0d-00e098032b8c"]
/// struct MyConfig {
///   field1: u32,
///   field2: u32,
/// }
/// ```
#[proc_macro_derive(FromHob, attributes(hob))]
pub fn hob_config(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    hob_macro::hob_config2(item.into()).into()
}

/// A proc-macro that registers the annotated function as a test case to be run by patina_test component.
///
/// There is a distinct difference between doing a #[cfg_attr(..., skip)] and a
/// #[cfg_attr(..., patina_test)]. The first still compiles the test case, but skips it at runtime. The second does not
/// compile the test case at all.
///
/// ## Attributes
///
/// - `#[should_fail]`: Indicates that the test is expected to fail. If the test passes, the test runner will log an
///   error.
/// - `#[should_fail = "message"]`: Indicates that the test is expected to fail with the given message. If the test
///   passes or fails with a different message, the test runner will log an error.
/// - `#[skip]`: Indicates that the test should be skipped.
///
/// ## Example
///
/// ```ignore
/// use patina::test::*;
/// use patina::boot_services::StandardBootServices;
/// use patina::test::patina_test;
/// use patina::{u_assert, u_assert_eq};
///
/// #[patina_test]
/// fn test_case() -> Result {
///     todo!()
/// }
///
/// #[patina_test]
/// #[should_fail]
/// fn failing_test_case() -> Result {
///     u_assert_eq!(1, 2);
///     Ok(())
/// }
///
/// #[patina_test]
/// #[should_fail = "This test failed"]
/// fn failing_test_case_with_msg() -> Result {
///    u_assert_eq!(1, 2, "This test failed");
///    Ok(())
/// }
///
/// #[patina_test]
/// #[skip]
/// fn skipped_test_case() -> Result {
///    todo!()
/// }
///
/// #[patina_test]
/// #[cfg_attr(not(target_arch = "x86_64"), skip)]
/// fn x86_64_only_test_case(bs: StandardBootServices) -> Result {
///   todo!()
/// }
/// ```
#[proc_macro_attribute]
pub fn patina_test(_: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    test_macro::patina_test2(item.into()).into()
}

/// Derive Macro for implementing the `SmbiosRecordStructure` trait.
///
/// This macro automatically generates a complete `SmbiosRecordStructure` trait
/// implementation, eliminating the need for manual boilerplate code.
///
/// ## Macro Attributes
///
/// - `#[smbios(record_type = N)]`: **Required**. Specifies the SMBIOS type number (0-255).
///
/// ## Member Attributes
///
/// - `#[string_pool]`: Marks a field as the string pool (must be `Vec<String>`).
///   Only one field per struct can have this attribute.
///
/// ## Examples
///
/// ```rust, ignore
/// use patina_macro::SmbiosRecord;
/// use patina_smbios::{SmbiosTableHeader, SmbiosRecordStructure};
/// use alloc::{string::String, vec::Vec};
///
/// // Vendor-specific OEM record (Type 0x80-0xFF)
/// #[derive(SmbiosRecord)]
/// #[smbios(record_type = 0x80)]
/// pub struct VendorOemRecord {
///     pub header: SmbiosTableHeader,
///     pub oem_field: u32,
///     #[string_pool]
///     pub string_pool: Vec<String>,
/// }
///
/// // Custom record without strings
/// #[derive(SmbiosRecord)]
/// #[smbios(record_type = 0x81)]
/// pub struct CustomData {
///     pub header: SmbiosTableHeader,
///     pub value1: u16,
///     pub value2: u32,
/// }
/// ```
///
/// The macro generates:
/// - `const RECORD_TYPE: u8`
/// - `fn to_bytes(&self) -> Vec<u8>` - Complete serialization
/// - `fn validate(&self) -> Result<(), SmbiosError>` - String validation
/// - `fn string_pool(&self) -> &[String]` - String pool accessor
/// - `fn string_pool_mut(&mut self) -> &mut Vec<String>` - Mutable accessor
#[proc_macro_derive(SmbiosRecord, attributes(smbios, string_pool))]
pub fn smbios_record(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    smbios_record_macro::smbios_record_derive(item.into()).into()
}

/// Attribute macro for validating component entry_point parameters at compile time.
///
/// This macro analyzes the function signature and emits compile errors if it detects
/// parameter conflicts.
///
/// ## Usage
///
/// ```rust, ignore
/// use patina::component::IntoComponent;
/// use patina_macro::validate_component_params;
///
/// #[derive(IntoComponent)]
/// struct MyComponent;
///
/// impl MyComponent {
///     #[validate_component_params]
///     fn entry_point(self, config: ConfigMut<u32>) -> Result<()> {
///         Ok(())
///     }
/// }
/// ```
///
/// ## Example Error
///
/// ```compile_fail
/// #[validate_component_params]
/// fn entry_point(self, c1: ConfigMut<u32>, c2: ConfigMut<u32>) -> Result<()> {
///     // Compile error: Duplicate parameter type detected
/// }
/// ```
#[proc_macro_attribute]
pub fn validate_component_params(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    validate_params_macro::validate_component_params2(attr.into(), item.into()).into()
}

/// Attribute macro for component entry points with automatic parameter validation.
///
/// This macro validates component entry point parameters at compile time and emits a
/// marker trait proving validation occurred.
///
/// ## Usage with Standalone Functions (functions outside an `impl` block)
///
/// ```rust, ignore
/// use patina::component::{IntoComponent, component_entry_point};
///
/// #[derive(IntoComponent)]
/// #[entry_point(path = my_entry)]
/// pub struct MyComponent {
///     data: u32,
/// }
///
/// #[component_entry_point]
/// fn my_entry(
///     comp: MyComponent,
///     config: Config<u32>,
///     commands: Commands,
/// ) -> patina::error::Result<()> {
///     commands.add_service(MyService::new(comp.data));
///     Ok(())
/// }
/// ```
///
/// ## Usage with Impl Methods
///
/// ```rust, ignore
/// #[derive(IntoComponent)]
/// pub struct MyComponent;
///
/// impl MyComponent {
///     fn entry_point(self, config: Config<u32>) -> Result<()> {
///         // Impl methods don't require #[component_entry_point]
///         Ok(())
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn component_entry_point(attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    validate_params_macro::component_entry_point(attr.into(), item.into()).into()
}

/// Attribute macro for validating impl method entry points.
///
/// Use this macro on impl method entry points to validate parameters at compile time.
///
/// ## Usage
///
/// ```rust, ignore
/// use patina::component::{IntoComponent, validate_impl_entry_point};
///
/// #[derive(IntoComponent)]
/// pub struct MyComponent {
///     data: u32,
/// }
///
/// impl MyComponent {
///     #[validate_impl_entry_point]
///     fn entry_point(self, config: Config<u32>, commands: Commands) -> patina::error::Result<()> {
///         commands.add_service(MyService::new(self.data));
///         Ok(())
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn validate_impl_entry_point(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    validate_params_macro::validate_impl_entry_point(attr.into(), item.into()).into()
}

/// Attribute macro for validating component impl blocks.
///
/// This macro validates an entire impl block and ensures the entry_point method has valid
/// parameters. It emits a marker proving validation occurred, which is checked by the
/// `#[derive(IntoComponent)]` macro.
///
/// This attribute is mandatory for impl-based components.
///
/// ## Usage
///
/// ```rust, ignore
/// use patina::component::{IntoComponent, component_impl};
///
/// #[derive(IntoComponent)]
/// pub struct MyComponent {
///     data: u32,
/// }
///
/// #[component_impl]
/// impl MyComponent {
///     fn entry_point(self, config: Config<u32>) -> Result<()> {
///         Ok(())
///     }
/// }
/// ```
///
/// ## Enforcement
///
/// The `#[derive(IntoComponent)]` macro automatically checks for the validation marker.
#[proc_macro_attribute]
pub fn component_impl(attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    validate_params_macro::component_impl(attr.into(), item.into()).into()
}
