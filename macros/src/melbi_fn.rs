//! Implementation of the `#[melbi_fn]` attribute macro
//!
//! Parses function signatures, validates them, and directly generates the wrapper struct
//! and FFI glue code needed to bridge Rust functions into the Melbi runtime.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    FnArg, GenericArgument, GenericParam, ItemFn, Pat, PatType, PathArguments, ReturnType, Type,
    parse_macro_input,
};

use crate::common::get_name_from_tokens;

/// Entry point for the `#[melbi_fn]` attribute macro.
pub fn melbi_fn_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);
    let fn_name = input_fn.sig.ident.to_string();

    // Parse the attribute to get the Melbi name (explicit or derived)
    let melbi_name = match get_name_from_tokens(attr, "melbi_fn", "name", &fn_name) {
        Ok(name) => name,
        Err(err) => return err.to_compile_error().into(),
    };

    // Parse and validate the function signature
    let sig = match parse_signature(&input_fn) {
        Ok(sig) => sig,
        Err(err) => return err.to_compile_error().into(),
    };

    // Generate the output
    generate_output(&input_fn, &melbi_name, &sig).into()
}

// ============================================================================
// Data Structures
// ============================================================================

/// Parsed function signature information
struct ParsedSignature {
    /// The Rust function name
    fn_name: syn::Ident,
    /// Lifetime from the function (if any). None means use default '__a.
    lifetime: Option<syn::Lifetime>,
    /// Whether the first parameter is &FfiContext
    has_context: bool,
    /// Business logic parameters (excluding context params)
    params: Vec<(syn::Ident, Box<Type>)>,
    /// The "okay" return type - unwrapped if Result<T, E>
    ok_return_type: Box<Type>,
    /// Whether the function returns Result<T, E>
    is_fallible: bool,
}

// ============================================================================
// Signature Parsing
// ============================================================================

/// Parse and validate the function signature.
fn parse_signature(func: &ItemFn) -> syn::Result<ParsedSignature> {
    let fn_name = func.sig.ident.clone();

    // Validate and extract lifetime
    let lifetime = parse_generics(&func.sig.generics)?;

    // Extract return type
    let return_type = parse_return_type(&func.sig)?;

    // Check if return type is Result<T, E> and extract okay type
    let (ok_return_type, is_fallible) = analyze_return_type(&return_type);

    // Detect context and extract business parameters
    let (has_context, params) = parse_params(&func.sig)?;

    Ok(ParsedSignature {
        fn_name,
        lifetime,
        has_context,
        params,
        ok_return_type,
        is_fallible,
    })
}

/// Validate that there is at most one lifetime parameter and extract it.
fn parse_generics(generics: &syn::Generics) -> syn::Result<Option<syn::Lifetime>> {
    let mut lifetime = Option::None;

    for param in &generics.params {
        match param {
            GenericParam::Lifetime(lifetime_param) => {
                if lifetime.is_some() {
                    return Err(syn::Error::new_spanned(
                        generics,
                        "[melbi] please rewrite your function to use a single lifetime parameter",
                    ));
                }
                lifetime.replace(lifetime_param.lifetime.clone());
            }
            GenericParam::Type(type_param) => {
                return Err(syn::Error::new_spanned(
                    type_param,
                    "[melbi] generic type parameters not supported",
                ));
            }
            GenericParam::Const(const_param) => {
                return Err(syn::Error::new_spanned(
                    const_param,
                    "[melbi] const generics are not supported",
                ));
            }
        };
    }
    Ok(lifetime)
}

/// Extract the return type from the function signature.
fn parse_return_type(sig: &syn::Signature) -> syn::Result<Box<Type>> {
    match &sig.output {
        ReturnType::Default => Err(syn::Error::new_spanned(
            sig,
            "[melbi] functions must have an explicit return type",
        )),
        ReturnType::Type(_, ty) => Ok(ty.clone()),
    }
}

/// Check if a type is `Result<T, E>` and extract the Ok type `T`.
/// Returns (ok_type, is_fallible).
fn analyze_return_type(ty: &Type) -> (Box<Type>, bool) {
    if let Some(ok_type) = extract_result_ok_type(ty) {
        (ok_type, true)
    } else {
        (Box::new(ty.clone()), false)
    }
}

/// Check if a type is `Result<T, E>` and extract the Ok type `T`.
fn extract_result_ok_type(ty: &Type) -> Option<Box<Type>> {
    if let Type::Path(type_path) = ty {
        let segments = &type_path.path.segments;
        if let Some(last_segment) = segments.last() {
            if last_segment.ident == "Result" {
                if let PathArguments::AngleBracketed(args) = &last_segment.arguments {
                    if let Some(GenericArgument::Type(ok_type)) = args.args.first() {
                        return Some(Box::new(ok_type.clone()));
                    }
                }
            }
        }
    }
    None
}

/// Detect if first param is FfiContext and extract business params.
fn parse_params(sig: &syn::Signature) -> syn::Result<(bool, Vec<(syn::Ident, Box<Type>)>)> {
    let mut params = Vec::new();
    let mut has_context = false;

    for (i, input) in sig.inputs.iter().enumerate() {
        let FnArg::Typed(PatType { pat, ty, .. }) = input else {
            return Err(syn::Error::new_spanned(
                input,
                "[melbi] normal typed argument expected (name: type)",
            ));
        };
        let Pat::Ident(pat_ident) = &**pat else {
            return Err(syn::Error::new_spanned(
                pat,
                "[melbi] no pattern matching supported in melbi functions",
            ));
        };

        // Skip first param if it's FfiContext
        if i == 0 && is_ffi_context_type(ty) {
            has_context = true;
            continue;
        }

        params.push((pat_ident.ident.clone(), ty.clone()));
    }

    Ok((has_context, params))
}

/// Check if a type looks like FfiContext (contains "FfiContext" in path).
fn is_ffi_context_type(ty: &Type) -> bool {
    if let Type::Reference(type_ref) = ty {
        if let Type::Path(type_path) = &*type_ref.elem {
            return type_path
                .path
                .segments
                .iter()
                .any(|s| s.ident == "FfiContext");
        }
    }
    false
}

// ============================================================================
// Code Generation
// ============================================================================

/// Generate the output: original function + wrapper struct + trait impls.
fn generate_output(input_fn: &ItemFn, melbi_name: &str, sig: &ParsedSignature) -> TokenStream2 {
    // Create the struct name identifier
    let struct_name = syn::Ident::new(melbi_name, proc_macro2::Span::call_site());
    let struct_name_str = struct_name.to_string();

    let fn_name = &sig.fn_name;

    // Determine the lifetime to use
    let lt = match &sig.lifetime {
        Some(lt) => lt.clone(),
        None => syn::Lifetime::new("'__a", proc_macro2::Span::call_site()),
    };

    let param_names: Vec<_> = sig.params.iter().map(|(name, _)| name).collect();
    let param_types: Vec<_> = sig.params.iter().map(|(_, ty)| ty).collect();
    let param_indices: Vec<_> = (0..sig.params.len()).collect();
    let arity = param_indices.len();

    let ok_ty = &sig.ok_return_type;

    // Generate the function call expression
    let call_expr = if sig.has_context {
        quote! { #fn_name(__ctx, #(#param_names),*) }
    } else {
        quote! { #fn_name(#(#param_names),*) }
    };

    // Generate result handling
    let result_handling = if sig.is_fallible {
        quote! {
            let __ok_result = __call_result.map_err(|e| ::melbi_core::evaluator::ExecutionError {
                kind: e.into(),
                source: ::melbi_core::shim::String::new(),
                span: ::melbi_core::parser::Span(0..0),
            })?;
        }
    } else {
        quote! {
            let __ok_result = __call_result;
        }
    };

    quote! {
        #input_fn

        #[doc(hidden)]
        pub struct #struct_name<#lt> {
            __fn_type: &#lt ::melbi_core::types::Type<#lt>,
        }

        impl<#lt> #struct_name<#lt> {
            pub fn new(__type_mgr: &#lt ::melbi_core::types::manager::TypeManager<#lt>) -> Self {
                use ::melbi_core::values::typed::Bridge;
                let __fn_type = __type_mgr.function(
                    &[#( <#param_types as Bridge>::type_from(__type_mgr) ),*],
                    <#ok_ty as Bridge>::type_from(__type_mgr),
                );
                Self { __fn_type }
            }
        }

        impl<#lt> ::melbi_core::values::function::Function<#lt, #lt> for #struct_name<#lt> {
            fn ty(&self) -> &#lt ::melbi_core::types::Type<#lt> {
                self.__fn_type
            }

            #[allow(unused_variables)]
            unsafe fn call_unchecked(
                &self,
                __ctx: &::melbi_core::values::function::FfiContext<#lt, #lt>,
                __args: &[::melbi_core::values::dynamic::Value<#lt, #lt>],
            ) -> Result<::melbi_core::values::dynamic::Value<#lt, #lt>, ::melbi_core::evaluator::ExecutionError> {
                use ::melbi_core::values::typed::{Bridge, RawConvertible};
                debug_assert_eq!(__args.len(), #arity);

                // Extract parameters
                #(
                    let #param_names = unsafe {
                        <#param_types as RawConvertible>::from_raw_value(__args[#param_indices].raw())
                    };
                )*

                // Call the user function
                let __call_result = #call_expr;

                // Handle the result
                #result_handling

                let __raw = <#ok_ty as RawConvertible>::to_raw_value(__ctx.arena(), __ok_result);
                let __ty = <#ok_ty as Bridge>::type_from(__ctx.type_mgr());
                Ok(::melbi_core::values::dynamic::Value::from_raw_unchecked(__ty, __raw))
            }
        }

        impl<#lt> ::melbi_core::values::function::AnnotatedFunction<#lt> for #struct_name<#lt> {
            fn name(&self) -> &'static str {
                #struct_name_str
            }

            fn location(&self) -> (&'static str, &'static str, &'static str, u32, u32) {
                (env!("CARGO_CRATE_NAME"), env!("CARGO_PKG_VERSION"), file!(), line!(), column!())
            }

            fn doc(&self) -> Option<&str> {
                None
            }
        }
    }
}
