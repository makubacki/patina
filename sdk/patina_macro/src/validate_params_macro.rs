//! Attribute macro for validating component parameter conflicts at compile time.
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
//!

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{FnArg, ItemFn, Pat, Type, TypePath, parse2};

/// Validates that a component entry point function has valid parameters.
///
/// This macro performs compile-time validation of component entry point parameters and
/// emits a marker trait proving validation occurred. Use this for standalone entry point functions.
///
/// ## Usage
///
/// ```rust, ignore
/// #[derive(IntoComponent)]
/// #[entry_point(path = my_entry)]
/// pub struct MyComponent;
///
/// #[component_entry_point]
/// fn my_entry(comp: MyComponent, config: Config<u32>) -> Result<()> {
///     Ok(())
/// }
/// ```
///
/// ## Validation Rules
///
/// - First parameter must be the component instance (not `self`)
/// - No duplicate `ConfigMut<T>` parameters with the same type T
/// - Cannot have both `Config<T>` and `ConfigMut<T>` for the same type T
/// - Cannot use `&mut Storage` with `Config<T>` or `ConfigMut<T>`
/// - Cannot use `&Storage` with `ConfigMut<T>`
/// - Cannot have multiple `Commands` parameters or multiple service table parameters
///
/// ## Enforcement
///
/// When used with `#[derive(IntoComponent)]` on standalone entry point functions,
/// validation is automatically enforced. Impl methods should use
/// `#[validate_impl_entry_point]` instead.
pub(crate) fn component_entry_point(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = match parse2::<ItemFn>(item.clone()) {
        Ok(func) => func,
        Err(e) => return e.to_compile_error(),
    };

    // Validate first parameter is not 'self'
    if let Err(error) = check_first_param_not_self(&func) {
        return error;
    }

    // Validate parameters for conflicts
    if let Err(error) = check_param_conflicts(&func) {
        return error;
    }

    let func_name = &func.sig.ident;
    let func_name_str = func_name.to_string();

    // Generate a unique marker type name based on the function
    let marker_type = quote::format_ident!("__ValidatedEntryPoint_{}", func_name_str);

    // Return the original function unchanged (validation passed) plus a marker type
    // that proves this function has been validated
    quote! {
        #func

        #[allow(non_camel_case_types)]
        struct #marker_type;

        impl patina::component::ValidatedEntryPoint for #marker_type {}
    }
}

/// Validates that an impl method entry point has valid parameters.
///
/// This macro performs compile-time validation of impl method entry point parameters.
/// Use this on impl methods (functions with `self` parameter).
///
/// Optionally, you can specify the component type name to create a validation marker:
/// `#[validate_impl_entry_point(MyComponent)]`
///
/// ## Usage
///
/// ```rust, ignore
/// #[derive(IntoComponent)]
/// pub struct MyComponent;
///
/// impl MyComponent {
///     #[validate_impl_entry_point(MyComponent)]
///     fn entry_point(self, config: Config<u32>) -> Result<()> {
///         Ok(())
///     }
/// }
/// ```
///
/// ## Validation Rules
///
/// - First parameter must be `self` or `mut self`
/// - No duplicate `ConfigMut<T>` parameters with the same type T
/// - Cannot have both `Config<T>` and `ConfigMut<T>` for the same type T
/// - Cannot use `&mut Storage` with `Config<T>` or `ConfigMut<T>`
/// - Cannot use `&Storage` with `ConfigMut<T>`
/// - Cannot have multiple `Commands` parameters or multiple service table parameters
pub(crate) fn validate_impl_entry_point(attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = match parse2::<ItemFn>(item.clone()) {
        Ok(func) => func,
        Err(e) => return e.to_compile_error(),
    };

    // Validate first parameter IS 'self' (required for impl methods)
    if let Err(error) = check_impl_method_has_self(&func) {
        return error;
    }

    // Validate parameters for conflicts (same rules as standalone functions)
    if let Err(error) = check_param_conflicts(&func) {
        return error;
    }

    // Parse optional component type name from attribute
    let marker = if !attr.is_empty() {
        // Parse component type name from attribute: #[validate_impl_entry_point(MyComponent)]
        let component_type = match parse2::<Ident>(attr) {
            Ok(ident) => ident,
            Err(_) => {
                return quote! {
                    compile_error!("Expected component type name in attribute: #[validate_impl_entry_point(MyComponent)]");
                    #func
                };
            }
        };

        let func_name = &func.sig.ident;
        let marker_type = format_ident!("__ValidatedImplMethod_{}_{}", component_type, func_name);

        quote! {
            #[allow(non_camel_case_types)]
            struct #marker_type;

            impl patina::component::ValidatedEntryPoint for #marker_type {}
        }
    } else {
        // No marker if component type not specified
        quote! {}
    };

    quote! {
        #func
        #marker
    }
}

/// Validates an entire impl block and ensures the entry_point method has valid parameters.
///
/// This macro performs compile-time validation of component impl blocks and emits a marker
/// proving validation occurred. This is required for impl-based components when using
/// `#[derive(IntoComponent)]`.
///
/// ## Usage
///
/// ```rust, ignore
/// #[derive(IntoComponent)]
/// pub struct MyComponent;
///
/// #[component_impl]
/// impl MyComponent {
///     fn entry_point(self, config: Config<u32>) -> Result<()> {
///         Ok(())
///     }
/// }
/// ```
///
/// ## Validation Rules
///
/// - Impl block must contain an `entry_point` method
/// - First parameter must be `self` or `mut self`
/// - No duplicate `ConfigMut<T>` parameters with the same type T
/// - Cannot have both `Config<T>` and `ConfigMut<T>` for the same type T
/// - Cannot use `&mut Storage` with `Config<T>` or `ConfigMut<T>`
/// - Cannot use `&Storage` with `ConfigMut<T>`
/// - Cannot have multiple `Commands` parameters or multiple service table parameters
pub(crate) fn component_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    use syn::ItemImpl;

    let impl_block = match parse2::<ItemImpl>(item.clone()) {
        Ok(impl_block) => impl_block,
        Err(e) => return e.to_compile_error(),
    };

    // Extract the component type name from the impl
    let component_type = match impl_block.self_ty.as_ref() {
        Type::Path(type_path) => type_path.path.segments.last().map(|seg| &seg.ident).cloned(),
        _ => None,
    };

    let _component_type = match component_type {
        Some(ty) => ty,
        None => {
            return quote! {
                compile_error!("Could not determine component type from impl block");
                #impl_block
            };
        }
    };

    // Find the entry_point method
    let entry_point_method = impl_block.items.iter().find_map(|item| {
        if let syn::ImplItem::Fn(method) = item
            && method.sig.ident == "entry_point"
        {
            return Some(method);
        }
        None
    });

    let entry_point_method = match entry_point_method {
        Some(method) => method,
        None => {
            return quote! {
                compile_error!("Component impl block must contain an 'entry_point' method");
                #impl_block
            };
        }
    };

    // Create a "synthetic" ItemFn for validation
    let synthetic_fn = ItemFn {
        attrs: entry_point_method.attrs.clone(),
        vis: entry_point_method.vis.clone(),
        sig: entry_point_method.sig.clone(),
        block: Box::new(entry_point_method.block.clone()),
    };

    // Validate first parameter is 'self'
    if let Err(error_tokens) = check_impl_method_has_self(&synthetic_fn) {
        return quote! {
            #error_tokens
            #impl_block
        };
    }

    // Validate parameters for conflicts
    if let Err(error_tokens) = check_param_conflicts(&synthetic_fn) {
        return quote! {
            #error_tokens
            #impl_block
        };
    }

    // Get the full type with generics from the impl block
    let self_ty = &impl_block.self_ty;
    let (impl_generics, _ty_generics, where_clause) = impl_block.generics.split_for_impl();

    // Return the original impl block plus trait implementation on the actual type
    // Note: self_ty includes generic parameters (e.g., GenericStruct<T>)
    quote! {
        #impl_block

        // Mark this component type as validated by implementing the marker trait
        impl #impl_generics patina::component::ValidatedComponentImpl for #self_ty #where_clause {}
    }
}

/// Validates that the first function parameter is not 'self'.
/// This is used for standalone entry point functions.
pub(crate) fn check_first_param_not_self(func: &ItemFn) -> Result<(), TokenStream> {
    let first_param = func.sig.inputs.first();

    match first_param {
        Some(FnArg::Receiver(_)) => {
            let error_msg = "Component entry point cannot use 'self' as first parameter. Use an explicit component type parameter instead (e.g., 'comp: MyComponent').";
            Err(quote! {
                compile_error!(#error_msg);
                #func
            })
        }
        Some(FnArg::Typed(_)) => Ok(()),
        None => {
            let error_msg = "Component entry point must have at least one parameter: the component instance (e.g., 'comp: MyComponent').";
            Err(quote! {
                compile_error!(#error_msg);
                #func
            })
        }
    }
}

/// Validates that an impl method has `self` or `mut self` as the first parameter.
/// This is used for impl method entry points.
pub(crate) fn check_impl_method_has_self(func: &ItemFn) -> Result<(), TokenStream> {
    let first_param = func.sig.inputs.first();

    match first_param {
        Some(FnArg::Receiver(_)) => Ok(()), // Has self - good for impl methods
        Some(FnArg::Typed(_)) | None => {
            let error_msg = "Impl method entry point must use 'self' or 'mut self' as first parameter.";
            Err(quote! {
                compile_error!(#error_msg);
                #func
            })
        }
    }
}

/// Validates that a component's entry_point function doesn't have conflicting parameters.
///
/// This macro analyzes the function signature and emits compile errors if it detects:
/// - Duplicate `ConfigMut<T>` parameters with the same type T
/// - Both `Config<T>` and `ConfigMut<T>` for the same type T
/// - `&mut Storage` combined with any `Config<T>` or `ConfigMut<T>`
/// - `&Storage` combined with `ConfigMut<T>`
/// - Cannot have multiple `Commands` parameters or multiple service table parameters
pub(crate) fn validate_component_params2(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = match parse2::<ItemFn>(item.clone()) {
        Ok(func) => func,
        Err(e) => return e.to_compile_error(),
    };

    // Analyze parameters for conflicts
    if let Err(error) = check_param_conflicts(&func) {
        return error;
    }

    // If no conflicts, return the original function unchanged
    quote! { #func }
}

/// Represents a parameter type we care about for conflict detection
#[derive(Debug, Clone, PartialEq, Eq)]
enum ParamType {
    Config(String),          // Config<T> where String is T
    ConfigMut(String),       // ConfigMut<T> where String is T
    Storage,                 // &Storage
    StorageMut,              // &mut Storage
    Commands,                // Commands
    StandardBootServices,    // StandardBootServices (UEFI Boot Services)
    StandardRuntimeServices, // StandardRuntimeServices (UEFI Runtime Services)
    Other,                   // Any other parameter type
}

/// Normalize a type path to its canonical string representation.
///
/// This function converts type paths to a consistent format that allows comparing
/// qualified and unqualified paths. For example:
/// - `Config` -> "Config"
/// - `patina::component::Config` -> "patina::component::Config"
/// - `crate::Config` -> "crate::Config"
fn normalize_type_path(path: &syn::Path) -> String {
    let segments: Vec<String> = path.segments.iter().map(|seg| seg.ident.to_string()).collect();
    segments.join("::")
}

/// Extract the inner type from a generic type like Config<T> or ConfigMut<T>
/// and return its normalized canonical representation.
fn extract_generic_inner(path: &TypePath) -> Option<String> {
    if let Some(segment) = path.path.segments.last()
        && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
        && let Some(syn::GenericArgument::Type(ty)) = args.args.first()
    {
        // Normalize the inner type for consistent comparison
        return Some(normalize_type(ty));
    }
    None
}

/// Normalize a type to its canonical string representation.
///
/// This handles various type forms and converts them to a consistent format:
/// - Path types: normalized path representation
/// - Generic types: includes normalized generic arguments
/// - Reference types: includes mutability
fn normalize_type(ty: &Type) -> String {
    match ty {
        Type::Path(type_path) => {
            let base_path = normalize_type_path(&type_path.path);

            // Handle generic arguments
            if let Some(segment) = type_path.path.segments.last()
                && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
            {
                let inner_types: Vec<String> = args
                    .args
                    .iter()
                    .map(|arg| match arg {
                        syn::GenericArgument::Type(inner_ty) => normalize_type(inner_ty),
                        other => quote!(#other).to_string(),
                    })
                    .collect();

                if !inner_types.is_empty() {
                    return format!("{}<{}>", base_path, inner_types.join(", "));
                }
            }

            base_path
        }
        Type::Reference(type_ref) => {
            let inner = normalize_type(&type_ref.elem);
            if type_ref.mutability.is_some() { format!("&mut {}", inner) } else { format!("&{}", inner) }
        }
        _ => quote!(#ty).to_string(),
    }
}

/// Get the base type name from a type path (the last segment without qualifiers).
///
/// Examples:
/// - `Config` -> "Config"
/// - `patina::component::Config` -> "Config"
/// - `crate::something::ConfigMut` -> "ConfigMut"
fn get_base_type_name(path: &syn::Path) -> Option<String> {
    path.segments.last().map(|seg| seg.ident.to_string())
}

/// Classify a parameter type
fn classify_param(ty: &Type) -> ParamType {
    match ty {
        Type::Path(type_path) => {
            // Get the base type name (last segment) for matching
            let base_name = match get_base_type_name(&type_path.path) {
                Some(name) => name,
                None => return ParamType::Other,
            };

            // Check for Config<T>
            if base_name == "Config"
                && let Some(inner) = extract_generic_inner(type_path)
            {
                return ParamType::Config(inner);
            }

            // Check for ConfigMut<T>
            if base_name == "ConfigMut"
                && let Some(inner) = extract_generic_inner(type_path)
            {
                return ParamType::ConfigMut(inner);
            }

            // Check for Commands
            if base_name == "Commands" {
                return ParamType::Commands;
            }

            // Check for StandardBootServices
            if base_name == "StandardBootServices" {
                return ParamType::StandardBootServices;
            }

            // Check for StandardRuntimeServices
            if base_name == "StandardRuntimeServices" {
                return ParamType::StandardRuntimeServices;
            }

            ParamType::Other
        }
        Type::Reference(type_ref) => {
            if let Type::Path(type_path) = &*type_ref.elem {
                let base_name = match get_base_type_name(&type_path.path) {
                    Some(name) => name,
                    None => return ParamType::Other,
                };

                // Check for &Storage or &mut Storage
                if base_name == "Storage" {
                    if type_ref.mutability.is_some() {
                        return ParamType::StorageMut;
                    } else {
                        return ParamType::Storage;
                    }
                }
            }
            ParamType::Other
        }
        _ => ParamType::Other,
    }
}

/// Check for parameter conflicts and return compile error if found
/// Checks for parameter conflicts in the function signature.
pub(crate) fn check_param_conflicts(func: &ItemFn) -> Result<(), TokenStream> {
    let mut params: Vec<(usize, ParamType, String)> = Vec::new();

    // Collect all parameters (skip 'self')
    for (idx, arg) in func.sig.inputs.iter().enumerate() {
        if let FnArg::Typed(pat_type) = arg {
            let param_type = classify_param(&pat_type.ty);
            let param_name = match &*pat_type.pat {
                Pat::Ident(ident) => ident.ident.to_string(),
                _ => format!("param_{}", idx),
            };
            params.push((idx, param_type, param_name));
        }
    }

    // Check for conflicts
    for i in 0..params.len() {
        for j in (i + 1)..params.len() {
            let (idx1, type1, name1) = &params[i];
            let (idx2, type2, name2) = &params[j];

            match (type1, type2) {
                // Duplicate ConfigMut<T>
                (ParamType::ConfigMut(t1), ParamType::ConfigMut(t2)) if t1 == t2 => {
                    let error_msg = format!(
                        "Duplicate parameter type detected: parameter '{}' (position {}) and parameter '{}' (position {}) both have type ConfigMut<{}>. Each ConfigMut type can only appear once in a component's entry point.",
                        name1, idx1, name2, idx2, t1
                    );
                    return Err(quote! {
                        compile_error!(#error_msg);
                        #func
                    });
                }

                // Config<T> conflicts with ConfigMut<T>
                (ParamType::Config(t1), ParamType::ConfigMut(t2))
                | (ParamType::ConfigMut(t1), ParamType::Config(t2))
                    if t1 == t2 =>
                {
                    let error_msg = format!(
                        "Parameter conflict detected: parameter '{}' (position {}) with type Config<{}> conflicts with parameter '{}' (position {}) with type ConfigMut<{}>. You cannot have both Config<T> and ConfigMut<T> for the same type T.",
                        name1, idx1, t1, name2, idx2, t2
                    );
                    return Err(quote! {
                        compile_error!(#error_msg);
                        #func
                    });
                }

                // &mut Storage conflicts with Config<T> or ConfigMut<T>
                (ParamType::StorageMut, ParamType::Config(_))
                | (ParamType::Config(_), ParamType::StorageMut)
                | (ParamType::StorageMut, ParamType::ConfigMut(_))
                | (ParamType::ConfigMut(_), ParamType::StorageMut) => {
                    let error_msg = format!(
                        "Parameter conflict detected: parameter '{}' (position {}) and parameter '{}' (position {}) conflict. You cannot use &mut Storage together with Config<T> or ConfigMut<T> parameters.",
                        name1, idx1, name2, idx2
                    );
                    return Err(quote! {
                        compile_error!(#error_msg);
                        #func
                    });
                }

                // &Storage conflicts with ConfigMut<T>
                (ParamType::Storage, ParamType::ConfigMut(_)) | (ParamType::ConfigMut(_), ParamType::Storage) => {
                    let error_msg = format!(
                        "Parameter conflict detected: parameter '{}' (position {}) and parameter '{}' (position {}) conflict. You cannot use &Storage together with ConfigMut<T> parameters.",
                        name1, idx1, name2, idx2
                    );
                    return Err(quote! {
                        compile_error!(#error_msg);
                        #func
                    });
                }

                // Duplicate Commands
                (ParamType::Commands, ParamType::Commands) => {
                    let error_msg = format!(
                        "Duplicate Commands parameter detected: parameter '{}' (position {}) and parameter '{}' (position {}) both have type Commands. Only one Commands parameter is allowed.",
                        name1, idx1, name2, idx2
                    );
                    return Err(quote! {
                        compile_error!(#error_msg);
                        #func
                    });
                }

                // Duplicate StandardBootServices
                (ParamType::StandardBootServices, ParamType::StandardBootServices) => {
                    let error_msg = format!(
                        "Duplicate StandardBootServices parameter detected: parameter '{}' (position {}) and parameter '{}' (position {}) both have type StandardBootServices. Only one StandardBootServices parameter is allowed.",
                        name1, idx1, name2, idx2
                    );
                    return Err(quote! {
                        compile_error!(#error_msg);
                        #func
                    });
                }

                // Duplicate StandardRuntimeServices
                (ParamType::StandardRuntimeServices, ParamType::StandardRuntimeServices) => {
                    let error_msg = format!(
                        "Duplicate StandardRuntimeServices parameter detected: parameter '{}' (position {}) and parameter '{}' (position {}) both have type StandardRuntimeServices. Only one StandardRuntimeServices parameter is allowed.",
                        name1, idx1, name2, idx2
                    );
                    return Err(quote! {
                        compile_error!(#error_msg);
                        #func
                    });
                }

                _ => {} // No conflict
            }
        }
    }

    Ok(())
}

#[cfg(test)]
#[coverage(off)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn test_allows_valid_params() {
        let input = quote! {
            fn entry_point(self, config: Config<u32>, other: Service<Foo>) -> Result<()> {
                Ok(())
            }
        };

        let result = validate_component_params2(quote!(), input);
        // Should not contain compile_error
        assert!(!result.to_string().contains("compile_error"));
    }

    #[test]
    fn test_detects_duplicate_config_mut() {
        let input = quote! {
            fn entry_point(self, c1: ConfigMut<u32>, c2: ConfigMut<u32>) -> Result<()> {
                Ok(())
            }
        };

        let result = validate_component_params2(quote!(), input);
        assert!(result.to_string().contains("compile_error"));
        assert!(result.to_string().contains("Duplicate parameter type"));
    }

    #[test]
    fn test_detects_config_and_config_mut_conflict() {
        let input = quote! {
            fn entry_point(self, c1: Config<u32>, c2: ConfigMut<u32>) -> Result<()> {
                Ok(())
            }
        };

        let result = validate_component_params2(quote!(), input);
        assert!(result.to_string().contains("compile_error"));
        assert!(result.to_string().contains("Parameter conflict"));
    }

    #[test]
    fn test_detects_storage_mut_and_config_conflict() {
        let input = quote! {
            fn entry_point(self, storage: &mut Storage, config: Config<u32>) -> Result<()> {
                Ok(())
            }
        };

        let result = validate_component_params2(quote!(), input);
        assert!(result.to_string().contains("compile_error"));
        assert!(result.to_string().contains("&mut Storage"));
    }

    #[test]
    fn test_component_entry_point_validates_name() {
        let input = quote! {
            fn entry_point(comp: MyComponent, config: Config<u32>) -> Result<()> {
                Ok(())
            }
        };

        let result = component_entry_point(quote!(), input);
        assert!(!result.to_string().contains("compile_error"));
    }

    #[test]
    fn test_component_entry_point_rejects_wrong_name() {
        // This test now validates that the macro accepts any function name
        // and generates the correct marker trait based on that name
        let input = quote! {
            fn my_custom_function(comp: MyComponent, config: Config<u32>) -> Result<()> {
                Ok(())
            }
        };

        let result = component_entry_point(quote!(), input);
        // Should not contain compile_error - any name is now valid
        assert!(!result.to_string().contains("compile_error"));
        // Should generate marker trait with the function's actual name
        assert!(result.to_string().contains("__ValidatedEntryPoint_my_custom_function"));
    }

    #[test]
    fn test_component_entry_point_rejects_self() {
        let input = quote! {
            fn entry_point(self, config: Config<u32>) -> Result<()> {
                Ok(())
            }
        };

        let result = component_entry_point(quote!(), input);
        assert!(result.to_string().contains("compile_error"));
        assert!(result.to_string().contains("self"));
    }

    #[test]
    fn test_component_entry_point_validates_params() {
        let input = quote! {
            fn entry_point(comp: MyComponent, c1: ConfigMut<u32>, c2: ConfigMut<u32>) -> Result<()> {
                Ok(())
            }
        };

        let result = component_entry_point(quote!(), input);
        assert!(result.to_string().contains("compile_error"));
        assert!(result.to_string().contains("Duplicate"));
    }

    #[test]
    fn test_component_impl_requires_entry_point_method() {
        let input = quote! {
            impl MyComponent {
                fn some_other_method(self) -> Result<()> {
                    Ok(())
                }
            }
        };

        let result = component_impl(quote!(), input);
        assert!(result.to_string().contains("compile_error"));
        assert!(result.to_string().contains("entry_point"));
    }

    #[test]
    fn test_component_impl_requires_self_parameter() {
        let input = quote! {
            impl MyComponent {
                fn entry_point(comp: MyComponent) -> Result<()> {
                    Ok(())
                }
            }
        };

        let result = component_impl(quote!(), input);
        assert!(result.to_string().contains("compile_error"));
        assert!(result.to_string().contains("self"));
    }

    #[test]
    fn test_component_impl_accepts_self() {
        let input = quote! {
            impl MyComponent {
                fn entry_point(self) -> Result<()> {
                    Ok(())
                }
            }
        };

        let result = component_impl(quote!(), input);
        assert!(!result.to_string().contains("compile_error"));
        assert!(result.to_string().contains("ValidatedComponentImpl"));
        assert!(result.to_string().contains("MyComponent"));
    }

    #[test]
    fn test_component_impl_accepts_mut_self() {
        let input = quote! {
            impl MyComponent {
                fn entry_point(mut self) -> Result<()> {
                    Ok(())
                }
            }
        };

        let result = component_impl(quote!(), input);
        assert!(!result.to_string().contains("compile_error"));
        assert!(result.to_string().contains("ValidatedComponentImpl"));
        assert!(result.to_string().contains("MyComponent"));
    }

    #[test]
    fn test_component_impl_validates_parameters() {
        let input = quote! {
            impl MyComponent {
                fn entry_point(self, c1: ConfigMut<u32>, c2: ConfigMut<u32>) -> Result<()> {
                    Ok(())
                }
            }
        };

        let result = component_impl(quote!(), input);
        assert!(result.to_string().contains("compile_error"));
        assert!(result.to_string().contains("Duplicate"));
    }

    #[test]
    fn test_component_impl_with_valid_params() {
        let input = quote! {
            impl MyComponent {
                fn entry_point(self, config: Config<u32>, service: Service<Foo>) -> Result<()> {
                    Ok(())
                }
            }
        };

        let result = component_impl(quote!(), input);
        assert!(!result.to_string().contains("compile_error"));
        assert!(result.to_string().contains("ValidatedComponentImpl"));
        assert!(result.to_string().contains("MyComponent"));
    }

    #[test]
    fn test_detects_duplicate_config_mut_different_positions() {
        let input = quote! {
            fn entry_point(
                comp: MyComponent,
                service: Service<Foo>,
                c1: ConfigMut<u32>,
                other: Service<Bar>,
                c2: ConfigMut<u32>
            ) -> Result<()> {
                Ok(())
            }
        };

        let result = validate_component_params2(quote!(), input);
        assert!(result.to_string().contains("compile_error"));
        assert!(result.to_string().contains("Duplicate parameter type"));
        assert!(result.to_string().contains("ConfigMut"));
    }

    #[test]
    fn test_allows_different_config_mut_types() {
        let input = quote! {
            fn entry_point(comp: MyComponent, c1: ConfigMut<u32>, c2: ConfigMut<String>) -> Result<()> {
                Ok(())
            }
        };

        let result = validate_component_params2(quote!(), input);
        assert!(!result.to_string().contains("compile_error"));
    }

    #[test]
    fn test_detects_duplicate_config_mut_complex_types() {
        let input = quote! {
            fn entry_point(
                comp: MyComponent,
                c1: ConfigMut<Vec<String>>,
                c2: ConfigMut<Vec<String>>
            ) -> Result<()> {
                Ok(())
            }
        };

        let result = validate_component_params2(quote!(), input);
        assert!(result.to_string().contains("compile_error"));
        assert!(result.to_string().contains("Duplicate parameter type"));
    }

    #[test]
    fn test_detects_config_mut_and_config_conflict() {
        let input = quote! {
            fn entry_point(self, c1: ConfigMut<u32>, c2: Config<u32>) -> Result<()> {
                Ok(())
            }
        };

        let result = validate_component_params2(quote!(), input);
        assert!(result.to_string().contains("compile_error"));
        assert!(result.to_string().contains("Parameter conflict"));
        assert!(result.to_string().contains("Config<"));
        assert!(result.to_string().contains("ConfigMut<"));
    }

    #[test]
    fn test_allows_config_and_config_mut_different_types() {
        let input = quote! {
            fn entry_point(comp: MyComponent, c1: Config<u32>, c2: ConfigMut<String>) -> Result<()> {
                Ok(())
            }
        };

        let result = validate_component_params2(quote!(), input);
        assert!(!result.to_string().contains("compile_error"));
    }

    #[test]
    fn test_allows_multiple_config_same_type() {
        let input = quote! {
            fn entry_point(comp: MyComponent, c1: Config<u32>, c2: Config<u32>) -> Result<()> {
                Ok(())
            }
        };

        let result = validate_component_params2(quote!(), input);
        assert!(!result.to_string().contains("compile_error"));
    }

    #[test]
    fn test_detects_storage_mut_and_config_mut_conflict() {
        let input = quote! {
            fn entry_point(self, storage: &mut Storage, config: ConfigMut<u32>) -> Result<()> {
                Ok(())
            }
        };

        let result = validate_component_params2(quote!(), input);
        assert!(result.to_string().contains("compile_error"));
        assert!(result.to_string().contains("&mut Storage"));
    }

    #[test]
    fn test_detects_storage_and_config_mut_conflict() {
        let input = quote! {
            fn entry_point(self, storage: &Storage, config: ConfigMut<u32>) -> Result<()> {
                Ok(())
            }
        };

        let result = validate_component_params2(quote!(), input);
        assert!(result.to_string().contains("compile_error"));
        assert!(result.to_string().contains("&Storage"));
        assert!(result.to_string().contains("ConfigMut"));
    }

    #[test]
    fn test_detects_config_mut_and_storage_conflict() {
        let input = quote! {
            fn entry_point(self, config: ConfigMut<u32>, storage: &Storage) -> Result<()> {
                Ok(())
            }
        };

        let result = validate_component_params2(quote!(), input);
        assert!(result.to_string().contains("compile_error"));
        assert!(result.to_string().contains("ConfigMut"));
    }

    #[test]
    fn test_allows_storage_and_config() {
        let input = quote! {
            fn entry_point(comp: MyComponent, storage: &Storage, config: Config<u32>) -> Result<()> {
                Ok(())
            }
        };

        let result = validate_component_params2(quote!(), input);
        assert!(!result.to_string().contains("compile_error"));
    }

    #[test]
    fn test_allows_storage_mut_without_configs() {
        let input = quote! {
            fn entry_point(comp: MyComponent, storage: &mut Storage, service: Service<Foo>) -> Result<()> {
                Ok(())
            }
        };

        let result = validate_component_params2(quote!(), input);
        assert!(!result.to_string().contains("compile_error"));
    }

    #[test]
    fn test_detects_duplicate_commands() {
        let input = quote! {
            fn entry_point(self, cmd1: Commands, cmd2: Commands) -> Result<()> {
                Ok(())
            }
        };

        let result = validate_component_params2(quote!(), input);
        assert!(result.to_string().contains("compile_error"));
        assert!(result.to_string().contains("Duplicate Commands"));
    }

    #[test]
    fn test_allows_single_commands() {
        let input = quote! {
            fn entry_point(comp: MyComponent, cmd: Commands) -> Result<()> {
                Ok(())
            }
        };

        let result = validate_component_params2(quote!(), input);
        assert!(!result.to_string().contains("compile_error"));
    }

    #[test]
    fn test_detects_duplicate_commands_with_other_params() {
        let input = quote! {
            fn entry_point(
                self,
                config: Config<u32>,
                cmd1: Commands,
                service: Service<Foo>,
                cmd2: Commands
            ) -> Result<()> {
                Ok(())
            }
        };

        let result = validate_component_params2(quote!(), input);
        assert!(result.to_string().contains("compile_error"));
        assert!(result.to_string().contains("Commands"));
    }

    #[test]
    fn test_check_first_param_not_self_rejects_self() {
        let input = quote! {
            fn entry_point(self, config: Config<u32>) -> Result<()> {
                Ok(())
            }
        };

        let func = parse2::<ItemFn>(input).unwrap();
        let result = check_first_param_not_self(&func);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("self"));
    }

    #[test]
    fn test_check_first_param_not_self_accepts_typed_param() {
        let input = quote! {
            fn entry_point(comp: MyComponent, config: Config<u32>) -> Result<()> {
                Ok(())
            }
        };

        let func = parse2::<ItemFn>(input).unwrap();
        let result = check_first_param_not_self(&func);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_first_param_not_self_rejects_no_params() {
        let input = quote! {
            fn entry_point() -> Result<()> {
                Ok(())
            }
        };

        let func = parse2::<ItemFn>(input).unwrap();
        let result = check_first_param_not_self(&func);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("at least one parameter"));
    }

    #[test]
    fn test_check_impl_method_has_self_accepts_self() {
        let input = quote! {
            fn entry_point(self, config: Config<u32>) -> Result<()> {
                Ok(())
            }
        };

        let func = parse2::<ItemFn>(input).unwrap();
        let result = check_impl_method_has_self(&func);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_impl_method_has_self_accepts_mut_self() {
        let input = quote! {
            fn entry_point(mut self, config: Config<u32>) -> Result<()> {
                Ok(())
            }
        };

        let func = parse2::<ItemFn>(input).unwrap();
        let result = check_impl_method_has_self(&func);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_impl_method_has_self_rejects_typed_param() {
        let input = quote! {
            fn entry_point(comp: MyComponent, config: Config<u32>) -> Result<()> {
                Ok(())
            }
        };

        let func = parse2::<ItemFn>(input).unwrap();
        let result = check_impl_method_has_self(&func);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("self"));
    }

    #[test]
    fn test_check_impl_method_has_self_rejects_no_params() {
        let input = quote! {
            fn entry_point() -> Result<()> {
                Ok(())
            }
        };

        let func = parse2::<ItemFn>(input).unwrap();
        let result = check_impl_method_has_self(&func);
        assert!(result.is_err());
    }

    #[test]
    fn test_allows_many_valid_params() {
        let input = quote! {
            fn entry_point(
                comp: MyComponent,
                config1: Config<u32>,
                config2: Config<String>,
                config3: Config<Vec<u8>>,
                service1: Service<Foo>,
                service2: Service<Bar>,
                hob: Hob<MyHob>,
                storage: &Storage,
                commands: Commands
            ) -> Result<()> {
                Ok(())
            }
        };

        let result = validate_component_params2(quote!(), input);
        assert!(!result.to_string().contains("compile_error"));
    }

    #[test]
    fn test_detects_multiple_conflicts() {
        let input = quote! {
            fn entry_point(
                self,
                c1: ConfigMut<u32>,
                c2: ConfigMut<u32>,
                storage: &mut Storage,
                config: Config<String>
            ) -> Result<()> {
                Ok(())
            }
        };

        let result = validate_component_params2(quote!(), input);
        assert!(result.to_string().contains("compile_error"));
        // Should detect at least one of the conflicts
    }

    #[test]
    fn test_allows_config_mut_without_conflicts() {
        let input = quote! {
            fn entry_point(
                comp: MyComponent,
                config_mut: ConfigMut<u32>,
                service: Service<Foo>,
                hob: Hob<MyHob>
            ) -> Result<()> {
                Ok(())
            }
        };

        let result = validate_component_params2(quote!(), input);
        assert!(!result.to_string().contains("compile_error"));
    }

    #[test]
    fn test_allows_empty_params_after_component() {
        let input = quote! {
            fn entry_point(comp: MyComponent) -> Result<()> {
                Ok(())
            }
        };

        let result = validate_component_params2(quote!(), input);
        assert!(!result.to_string().contains("compile_error"));
    }

    #[test]
    fn test_allows_only_self() {
        let input = quote! {
            fn entry_point(self) -> Result<()> {
                Ok(())
            }
        };

        let result = validate_component_params2(quote!(), input);
        assert!(!result.to_string().contains("compile_error"));
    }

    #[test]
    fn test_classifies_qualified_config() {
        let input = quote! {
            fn entry_point(comp: MyComponent, c: patina::component::Config<u32>) -> Result<()> {
                Ok(())
            }
        };

        let result = validate_component_params2(quote!(), input);
        assert!(!result.to_string().contains("compile_error"));
    }

    #[test]
    fn test_detects_conflict_with_qualified_types() {
        // This test verifies that qualified type paths are properly normalized and
        // detected as conflicts. For example, Config<u32> should conflict with
        // patina::component::ConfigMut<u32> because they both operate on the same
        // inner type u32.
        let input = quote! {
            fn entry_point(
                comp: MyComponent,
                c1: Config<u32>,
                c2: patina::component::ConfigMut<u32>
            ) -> Result<()> {
                Ok(())
            }
        };

        let result = validate_component_params2(quote!(), input);
        assert!(result.to_string().contains("compile_error"));
        assert!(result.to_string().contains("conflict"));
    }

    #[test]
    fn test_allows_option_wrapped_params() {
        let input = quote! {
            fn entry_point(
                comp: MyComponent,
                config: Option<Config<u32>>,
                service: Option<Service<Foo>>
            ) -> Result<()> {
                Ok(())
            }
        };

        let result = validate_component_params2(quote!(), input);
        // Option-wrapped types should be treated as Other and not conflict
        assert!(!result.to_string().contains("compile_error"));
    }

    #[test]
    fn test_component_entry_point_with_multiple_params() {
        let input = quote! {
            fn my_entry(
                comp: MyComponent,
                config: Config<u32>,
                service: Service<Foo>,
                hob: Hob<MyHob>
            ) -> Result<()> {
                Ok(())
            }
        };

        let result = component_entry_point(quote!(), input);
        assert!(!result.to_string().contains("compile_error"));
        assert!(result.to_string().contains("__ValidatedEntryPoint_my_entry"));
    }

    #[test]
    fn test_component_entry_point_detects_duplicate_config_mut() {
        let input = quote! {
            fn entry_point(
                comp: MyComponent,
                c1: ConfigMut<u32>,
                c2: ConfigMut<u32>
            ) -> Result<()> {
                Ok(())
            }
        };

        let result = component_entry_point(quote!(), input);
        assert!(result.to_string().contains("compile_error"));
        assert!(result.to_string().contains("Duplicate"));
    }

    #[test]
    fn test_component_entry_point_no_params_error() {
        let input = quote! {
            fn entry_point() -> Result<()> {
                Ok(())
            }
        };

        let result = component_entry_point(quote!(), input);
        assert!(result.to_string().contains("compile_error"));
        assert!(result.to_string().contains("at least one parameter"));
    }

    #[test]
    fn test_qualified_config_mut_duplicate_detection() {
        let input = quote! {
            fn entry_point(
                comp: MyComponent,
                c1: ConfigMut<u32>,
                c2: patina::component::ConfigMut<u32>
            ) -> Result<()> {
                Ok(())
            }
        };

        let result = validate_component_params2(quote!(), input);
        assert!(result.to_string().contains("compile_error"));
        assert!(result.to_string().contains("Duplicate"));
    }

    #[test]
    fn test_fully_qualified_both_sides() {
        let input = quote! {
            fn entry_point(
                comp: MyComponent,
                c1: patina::component::Config<String>,
                c2: patina::component::ConfigMut<String>
            ) -> Result<()> {
                Ok(())
            }
        };

        let result = validate_component_params2(quote!(), input);
        assert!(result.to_string().contains("compile_error"));
        assert!(result.to_string().contains("conflict"));
    }

    #[test]
    fn test_crate_qualified_paths() {
        let input = quote! {
            fn entry_point(
                comp: MyComponent,
                c1: crate::Config<u64>,
                c2: crate::ConfigMut<u64>
            ) -> Result<()> {
                Ok(())
            }
        };

        let result = validate_component_params2(quote!(), input);
        assert!(result.to_string().contains("compile_error"));
        assert!(result.to_string().contains("conflict"));
    }

    #[test]
    fn test_mixed_qualified_storage_conflicts() {
        let input = quote! {
            fn entry_point(
                comp: MyComponent,
                storage: &mut patina::Storage,
                config: Config<u32>
            ) -> Result<()> {
                Ok(())
            }
        };

        let result = validate_component_params2(quote!(), input);
        assert!(result.to_string().contains("compile_error"));
        assert!(result.to_string().contains("Storage"));
    }

    #[test]
    fn test_normalized_complex_inner_types() {
        let input = quote! {
            fn entry_point(
                comp: MyComponent,
                c1: Config<Vec<String>>,
                c2: patina::component::ConfigMut<Vec<String>>
            ) -> Result<()> {
                Ok(())
            }
        };

        let result = validate_component_params2(quote!(), input);
        assert!(result.to_string().contains("compile_error"));
        assert!(result.to_string().contains("conflict"));
    }

    #[test]
    fn test_detects_duplicate_standard_boot_services() {
        let input = quote! {
            fn entry_point(self, bs1: StandardBootServices, bs2: StandardBootServices) -> Result<()> {
                Ok(())
            }
        };

        let result = validate_component_params2(quote!(), input);
        assert!(result.to_string().contains("compile_error"));
        assert!(result.to_string().contains("Duplicate StandardBootServices"));
    }

    #[test]
    fn test_allows_single_standard_boot_services() {
        let input = quote! {
            fn entry_point(comp: MyComponent, bs: StandardBootServices) -> Result<()> {
                Ok(())
            }
        };

        let result = validate_component_params2(quote!(), input);
        assert!(!result.to_string().contains("compile_error"));
    }

    #[test]
    fn test_detects_duplicate_standard_runtime_services() {
        let input = quote! {
            fn entry_point(self, rs1: StandardRuntimeServices, rs2: StandardRuntimeServices) -> Result<()> {
                Ok(())
            }
        };

        let result = validate_component_params2(quote!(), input);
        assert!(result.to_string().contains("compile_error"));
        assert!(result.to_string().contains("Duplicate StandardRuntimeServices"));
    }

    #[test]
    fn test_allows_single_standard_runtime_services() {
        let input = quote! {
            fn entry_point(comp: MyComponent, rs: StandardRuntimeServices) -> Result<()> {
                Ok(())
            }
        };

        let result = validate_component_params2(quote!(), input);
        assert!(!result.to_string().contains("compile_error"));
    }

    #[test]
    fn test_allows_both_boot_and_runtime_services() {
        let input = quote! {
            fn entry_point(
                comp: MyComponent,
                bs: StandardBootServices,
                rs: StandardRuntimeServices
            ) -> Result<()> {
                Ok(())
            }
        };

        let result = validate_component_params2(quote!(), input);
        assert!(!result.to_string().contains("compile_error"));
    }

    #[test]
    fn test_detects_qualified_standard_boot_services() {
        let input = quote! {
            fn entry_point(
                self,
                bs1: StandardBootServices,
                bs2: patina::boot_services::StandardBootServices
            ) -> Result<()> {
                Ok(())
            }
        };

        let result = validate_component_params2(quote!(), input);
        assert!(result.to_string().contains("compile_error"));
        assert!(result.to_string().contains("StandardBootServices"));
    }

    #[test]
    fn test_detects_qualified_standard_runtime_services() {
        let input = quote! {
            fn entry_point(
                self,
                rs1: StandardRuntimeServices,
                rs2: patina::runtime_services::StandardRuntimeServices
            ) -> Result<()> {
                Ok(())
            }
        };

        let result = validate_component_params2(quote!(), input);
        assert!(result.to_string().contains("compile_error"));
        assert!(result.to_string().contains("StandardRuntimeServices"));
    }

    #[test]
    fn test_allows_services_with_other_params() {
        let input = quote! {
            fn entry_point(
                comp: MyComponent,
                bs: StandardBootServices,
                rs: StandardRuntimeServices,
                config: Config<u32>,
                service: Service<Foo>,
                commands: Commands
            ) -> Result<()> {
                Ok(())
            }
        };

        let result = validate_component_params2(quote!(), input);
        assert!(!result.to_string().contains("compile_error"));
    }
}
