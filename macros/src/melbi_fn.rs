//! Implementation of the `#[melbi_fn]` attribute macro
//!
//! This proc macro parses function signatures and generates calls to the
//! `melbi_fn_generate!` declarative macro, which handles the actual code generation.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    Expr, FnArg, GenericArgument, GenericParam, ItemFn, Lit, Meta, Pat, PatType, PathArguments,
    ReturnType, Type, parse_macro_input,
};

/// Entry point for the `#[melbi_fn]` attribute macro.
pub fn melbi_fn_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);

    // Parse the attribute to extract optional name
    let attr = match parse_attribute(attr) {
        Ok(attr) => attr,
        Err(err) => return err.to_compile_error().into(),
    };

    // Parse and validate the function signature
    let sig = match parse_signature(&input_fn) {
        Ok(sig) => sig,
        Err(err) => return err.to_compile_error().into(),
    };

    // Generate the output
    generate_output(&input_fn, &attr, &sig).into()
}

// ============================================================================
// Data Structures
// ============================================================================

/// Parsed attribute from `#[melbi_fn]` or `#[melbi_fn(name = "...")]`
struct MelbiAttr {
    /// The Melbi function name. If None, derive from fn name as PascalCase.
    name: Option<String>,
}

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
    /// The "bridge" return type - unwrapped if Result<T, E>
    bridge_return_type: Box<Type>,
    /// Whether the function returns Result<T, E>
    is_fallible: bool,
}

// ============================================================================
// Attribute Parsing
// ============================================================================

/// Parse the attribute to extract optional name parameter.
///
/// Supports:
/// - `#[melbi_fn]` - no args
/// - `#[melbi_fn(name = "CustomName")]` - explicit name
fn parse_attribute(attr: TokenStream) -> syn::Result<MelbiAttr> {
    // Empty attribute - derive name from function
    if attr.is_empty() {
        return Ok(MelbiAttr { name: None });
    }

    // Parse as Meta
    let meta = syn::parse::<Meta>(attr)?;

    if let Meta::NameValue(nv) = meta {
        if nv.path.is_ident("name") {
            if let Expr::Lit(expr_lit) = &nv.value {
                if let Lit::Str(lit) = &expr_lit.lit {
                    return Ok(MelbiAttr {
                        name: Some(lit.value()),
                    });
                }
            }
            return Err(syn::Error::new_spanned(
                &nv.value,
                "name attribute must be a string literal",
            ));
        }
        return Err(syn::Error::new_spanned(
            nv.path,
            "expected 'name' attribute",
        ));
    }

    Err(syn::Error::new_spanned(
        meta,
        "expected attribute format: #[melbi_fn] or #[melbi_fn(name = \"FunctionName\")]",
    ))
}

/// Convert snake_case to PascalCase.
///
/// Examples:
/// - `add` -> `Add`
/// - `safe_div` -> `SafeDiv`
/// - `add_numbers` -> `AddNumbers`
fn to_pascal_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut capitalize_next = true;

    for c in s.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }

    result
}

// ============================================================================
// Signature Parsing
// ============================================================================

/// Parse and validate the function signature.
fn parse_signature(func: &ItemFn) -> syn::Result<ParsedSignature> {
    let fn_name = func.sig.ident.clone();

    // Validate and extract lifetime
    let lifetime = validate_and_extract_lifetime(&func.sig.generics)?;

    // Validate no type parameters (for now)
    validate_no_type_params(&func.sig.generics)?;

    // Extract return type
    let return_type = extract_return_type(&func.sig)?;

    // Check if return type is Result<T, E> and extract bridge type
    let (bridge_return_type, is_fallible) = analyze_return_type(&return_type);

    // Detect context and extract business parameters
    let (has_context, params) = detect_context_and_params(&func.sig)?;

    Ok(ParsedSignature {
        fn_name,
        lifetime,
        has_context,
        params,
        bridge_return_type,
        is_fallible,
    })
}

/// Validate that there is at most one lifetime parameter and extract it.
fn validate_and_extract_lifetime(generics: &syn::Generics) -> syn::Result<Option<syn::Lifetime>> {
    let lifetimes: Vec<_> = generics
        .params
        .iter()
        .filter_map(|p| {
            if let GenericParam::Lifetime(lt) = p {
                Some(lt)
            } else {
                None
            }
        })
        .collect();

    match lifetimes.len() {
        0 => Ok(None),
        1 => Ok(Some(lifetimes[0].lifetime.clone())),
        _ => Err(syn::Error::new_spanned(
            generics,
            "melbi_fn functions can have at most one lifetime parameter",
        )),
    }
}

/// Validate that there are no type parameters (for now).
fn validate_no_type_params(generics: &syn::Generics) -> syn::Result<()> {
    for param in &generics.params {
        if let GenericParam::Type(type_param) = param {
            return Err(syn::Error::new_spanned(
                type_param,
                "melbi_fn does not yet support type parameters. \
                 This feature is planned for a future version.",
            ));
        }
        if let GenericParam::Const(const_param) = param {
            return Err(syn::Error::new_spanned(
                const_param,
                "melbi_fn does not support const generics",
            ));
        }
    }
    Ok(())
}

/// Extract the return type from the function signature.
fn extract_return_type(sig: &syn::Signature) -> syn::Result<Box<Type>> {
    match &sig.output {
        ReturnType::Default => Err(syn::Error::new_spanned(
            sig,
            "melbi_fn functions must have an explicit return type",
        )),
        ReturnType::Type(_, ty) => Ok(ty.clone()),
    }
}

/// Check if a type is `Result<T, E>` and extract the Ok type `T`.
/// Returns (bridge_type, is_fallible).
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
fn detect_context_and_params(
    sig: &syn::Signature,
) -> syn::Result<(bool, Vec<(syn::Ident, Box<Type>)>)> {
    let mut inputs_iter = sig.inputs.iter();

    // Check first parameter
    let first = match inputs_iter.next() {
        None => return Ok((false, Vec::new())),
        Some(arg) => arg,
    };

    let (_first_name, first_ty) = extract_param_info(first)
        .ok_or_else(|| syn::Error::new_spanned(first, "Expected typed parameter"))?;

    // Check for FfiContext
    if is_ffi_context_type(first_ty) {
        let params = collect_remaining_params(&mut inputs_iter);
        return Ok((true, params));
    }

    // First param is not FfiContext - include it in business params
    let mut params = Vec::new();
    if let FnArg::Typed(PatType { pat, ty, .. }) = first {
        if let Pat::Ident(pat_ident) = &**pat {
            params.push((pat_ident.ident.clone(), ty.clone()));
        }
    }
    params.extend(collect_remaining_params(&mut inputs_iter));

    Ok((false, params))
}

/// Extract parameter name and type from a FnArg.
fn extract_param_info(arg: &FnArg) -> Option<(String, &Type)> {
    if let FnArg::Typed(PatType { pat, ty, .. }) = arg {
        if let Pat::Ident(pat_ident) = &**pat {
            return Some((pat_ident.ident.to_string(), ty));
        }
    }
    None
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

/// Collect remaining parameters from iterator.
fn collect_remaining_params<'a, I>(inputs_iter: &mut I) -> Vec<(syn::Ident, Box<Type>)>
where
    I: Iterator<Item = &'a FnArg>,
{
    let mut params = Vec::new();
    for input in inputs_iter {
        if let FnArg::Typed(PatType { pat, ty, .. }) = input {
            if let Pat::Ident(pat_ident) = &**pat {
                params.push((pat_ident.ident.clone(), ty.clone()));
            }
        }
    }
    params
}

// ============================================================================
// Code Generation
// ============================================================================

/// Generate the output: original function + melbi_fn_generate! call.
fn generate_output(input_fn: &ItemFn, attr: &MelbiAttr, sig: &ParsedSignature) -> TokenStream2 {
    // Determine the struct name
    let struct_name = match &attr.name {
        Some(name) => syn::Ident::new(name, proc_macro2::Span::call_site()),
        None => {
            let pascal_name = to_pascal_case(&sig.fn_name.to_string());
            syn::Ident::new(&pascal_name, sig.fn_name.span())
        }
    };

    let fn_name = &sig.fn_name;

    // Determine the lifetime to use
    let lifetime = match &sig.lifetime {
        Some(lt) => lt.clone(),
        None => syn::Lifetime::new("'__a", proc_macro2::Span::call_site()),
    };

    // Generate parameter list: { a: i64, b: i64 }
    let param_names: Vec<_> = sig.params.iter().map(|(name, _)| name).collect();
    let param_types: Vec<_> = sig.params.iter().map(|(_, ty)| ty).collect();

    // Generate bridge return type
    let bridge_return_type = &sig.bridge_return_type;

    // Generate flags
    let has_context = sig.has_context;
    let fallible = sig.is_fallible;

    quote! {
        #input_fn

        melbi_fn_generate!(
            name = #struct_name,
            fn_name = #fn_name,
            lt = #lifetime,
            context_arg = #has_context,
            signature = { #( #param_names : #param_types ),* } -> #bridge_return_type,
            fallible = #fallible
        );
    }
}
