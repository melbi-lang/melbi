//! Implementation of the `#[melbi_fn]` attribute macro

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    Expr, FnArg, GenericArgument, ItemFn, Lit, Meta, Pat, PatType, PathArguments, ReturnType, Type,
    parse_macro_input,
};

pub fn melbi_fn_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);

    // Parse the attribute to extract the Melbi function name
    let melbi_name = match parse_attribute(attr) {
        Ok(name) => name,
        Err(err) => return err.to_compile_error().into(),
    };

    // Parse the function signature
    let sig_info = match parse_function_signature(&input_fn) {
        Ok(info) => info,
        Err(err) => return err.to_compile_error().into(),
    };

    // Generate all the code
    match generate_code(&melbi_name, &sig_info, &input_fn) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// How the function receives context resources (arena, type_mgr, etc.)
#[derive(Debug, Clone, Copy, PartialEq)]
enum ContextMode {
    /// Full context: fn(ctx: &FfiContext, args...)
    FullContext,
    /// Arena only: fn(arena: &Bump, args...)
    ArenaOnly,
    /// Type manager only: fn(type_mgr: &TypeManager, args...)
    TypeMgrOnly,
    /// Legacy mode: fn(arena: &Bump, type_mgr: &TypeManager, args...)
    Legacy,
    /// Pure function: fn(args...) - no context at all
    Pure,
}

/// Information extracted from the function signature
struct SignatureInfo {
    /// Function name
    fn_name: syn::Ident,
    /// Generic parameters from the function signature
    generics: syn::Generics,
    /// How the function receives context resources
    context_mode: ContextMode,
    /// Lifetime from context/arena parameter (extracted from reference type)
    /// For Pure mode, this is a generated 'arena lifetime
    arena_lifetime: syn::Lifetime,
    /// Lifetime from type_mgr parameter (extracted from reference type)
    /// For Pure/ArenaOnly modes, this is a generated 'types lifetime
    type_mgr_lifetime: syn::Lifetime,
    /// Parameter names and types (business logic params, excluding context params)
    params: Vec<(syn::Ident, Box<Type>)>,
    /// Return type
    return_type: Box<Type>,
}

/// Extract the lifetime from a reference type like &'a Bump
/// If no lifetime is specified, returns an anonymous lifetime '_
fn extract_lifetime(ty: &Type) -> syn::Result<syn::Lifetime> {
    if let Type::Reference(type_ref) = ty {
        if let Some(lifetime) = &type_ref.lifetime {
            return Ok(lifetime.clone());
        }
        // No explicit lifetime - use anonymous lifetime '_
        return Ok(syn::Lifetime::new("'_", proc_macro2::Span::call_site()));
    }
    Err(syn::Error::new_spanned(
        ty,
        "Expected a reference type (e.g., &'a Bump)",
    ))
}

/// Check if a type is `Result<T, E>` and extract the Ok type `T`.
/// Returns `Some(T)` if it's a Result, `None` otherwise.
fn extract_result_ok_type(ty: &Type) -> Option<Box<Type>> {
    if let Type::Path(type_path) = ty {
        let segments = &type_path.path.segments;
        if let Some(last_segment) = segments.last() {
            if last_segment.ident == "Result" {
                if let PathArguments::AngleBracketed(args) = &last_segment.arguments {
                    // Get the first generic argument (the Ok type)
                    if let Some(GenericArgument::Type(ok_type)) = args.args.first() {
                        return Some(Box::new(ok_type.clone()));
                    }
                }
            }
        }
    }
    None
}

/// Check if a type looks like FfiContext (contains "FfiContext" in path)
fn is_ffi_context_type(ty: &Type) -> bool {
    if let Type::Reference(type_ref) = ty {
        if let Type::Path(type_path) = &*type_ref.elem {
            return type_path.path.segments.iter().any(|s| s.ident == "FfiContext");
        }
    }
    false
}

/// Check if a type looks like Bump (contains "Bump" in path)
fn is_bump_type(ty: &Type) -> bool {
    if let Type::Reference(type_ref) = ty {
        if let Type::Path(type_path) = &*type_ref.elem {
            return type_path.path.segments.iter().any(|s| s.ident == "Bump");
        }
    }
    false
}

/// Check if a type looks like TypeManager (contains "TypeManager" in path)
fn is_type_manager_type(ty: &Type) -> bool {
    if let Type::Reference(type_ref) = ty {
        if let Type::Path(type_path) = &*type_ref.elem {
            return type_path.path.segments.iter().any(|s| s.ident == "TypeManager");
        }
    }
    false
}

/// Extract parameter name and type from a FnArg
fn extract_param_info(arg: &FnArg) -> Option<(String, &Type)> {
    if let FnArg::Typed(PatType { pat, ty, .. }) = arg {
        if let Pat::Ident(pat_ident) = &**pat {
            return Some((pat_ident.ident.to_string(), ty));
        }
    }
    None
}

/// Parse the function signature and extract parameter and return types
fn parse_function_signature(func: &ItemFn) -> syn::Result<SignatureInfo> {
    let fn_name = func.sig.ident.clone();
    let generics = func.sig.generics.clone();

    let mut inputs_iter = func.sig.inputs.iter().peekable();

    // Detect context mode by examining first parameter(s)
    let (context_mode, arena_lifetime, type_mgr_lifetime, params) =
        detect_context_mode_and_params(&mut inputs_iter, &generics)?;

    // Extract return type
    let return_type = match &func.sig.output {
        ReturnType::Default => {
            return Err(syn::Error::new_spanned(
                &func.sig,
                "melbi_fn functions must have an explicit return type",
            ));
        }
        ReturnType::Type(_, ty) => ty.clone(),
    };

    Ok(SignatureInfo {
        fn_name,
        generics,
        context_mode,
        arena_lifetime,
        type_mgr_lifetime,
        params,
        return_type,
    })
}

/// Detect context mode from function parameters and extract business params
fn detect_context_mode_and_params<'a, I>(
    inputs_iter: &mut core::iter::Peekable<I>,
    generics: &syn::Generics,
) -> syn::Result<(ContextMode, syn::Lifetime, syn::Lifetime, Vec<(syn::Ident, Box<Type>)>)>
where
    I: Iterator<Item = &'a FnArg>,
{
    // Default lifetimes if we can't extract them from parameters
    let default_arena_lifetime = syn::Lifetime::new("'arena", proc_macro2::Span::call_site());
    let default_types_lifetime = syn::Lifetime::new("'types", proc_macro2::Span::call_site());

    // Try to extract lifetimes from generics if available
    let (arena_lt_from_generics, types_lt_from_generics) = extract_lifetimes_from_generics(generics);

    // Check first parameter
    let first = match inputs_iter.next() {
        None => {
            // No parameters at all - Pure mode
            let arena_lt = arena_lt_from_generics.unwrap_or(default_arena_lifetime);
            let types_lt = types_lt_from_generics.unwrap_or(default_types_lifetime);
            return Ok((ContextMode::Pure, arena_lt, types_lt, Vec::new()));
        }
        Some(arg) => arg,
    };

    let (first_name, first_ty) = extract_param_info(first)
        .ok_or_else(|| syn::Error::new_spanned(first, "Expected typed parameter"))?;

    // Check for FfiContext
    if is_ffi_context_type(first_ty) {
        let arena_lt = extract_lifetime(first_ty).unwrap_or(default_arena_lifetime.clone());
        let types_lt = arena_lt.clone(); // FfiContext uses same lifetime for both
        let params = collect_remaining_params(inputs_iter);
        return Ok((ContextMode::FullContext, arena_lt, types_lt, params));
    }

    // Check for arena (Bump)
    if is_bump_type(first_ty) && (first_name == "arena" || first_name == "_arena") {
        let arena_lt = extract_lifetime(first_ty).unwrap_or(default_arena_lifetime);

        // Check if second param is type_mgr
        if let Some(second) = inputs_iter.peek() {
            if let Some((second_name, second_ty)) = extract_param_info(second) {
                if is_type_manager_type(second_ty) && (second_name == "type_mgr" || second_name == "_type_mgr") {
                    // Legacy mode: (arena, type_mgr, ...)
                    let _ = inputs_iter.next(); // consume second param
                    let types_lt = extract_lifetime(second_ty).unwrap_or(default_types_lifetime);
                    let params = collect_remaining_params(inputs_iter);
                    return Ok((ContextMode::Legacy, arena_lt, types_lt, params));
                }
            }
        }

        // ArenaOnly mode: (arena, ...)
        let types_lt = types_lt_from_generics.unwrap_or(default_types_lifetime);
        let params = collect_remaining_params(inputs_iter);
        return Ok((ContextMode::ArenaOnly, arena_lt, types_lt, params));
    }

    // Check for type_mgr only
    if is_type_manager_type(first_ty) && (first_name == "type_mgr" || first_name == "_type_mgr") {
        let types_lt = extract_lifetime(first_ty).unwrap_or(default_types_lifetime);
        let arena_lt = arena_lt_from_generics.unwrap_or(default_arena_lifetime);
        let params = collect_remaining_params(inputs_iter);
        return Ok((ContextMode::TypeMgrOnly, arena_lt, types_lt, params));
    }

    // First param is not a context param - Pure mode
    // But we need to include this first param in the business params
    let arena_lt = arena_lt_from_generics.unwrap_or(default_arena_lifetime);
    let types_lt = types_lt_from_generics.unwrap_or(default_types_lifetime);

    let mut params = Vec::new();
    // Add the first param back
    if let FnArg::Typed(PatType { pat, ty, .. }) = first {
        if let Pat::Ident(pat_ident) = &**pat {
            params.push((pat_ident.ident.clone(), ty.clone()));
        }
    }
    // Collect remaining
    params.extend(collect_remaining_params(inputs_iter));

    Ok((ContextMode::Pure, arena_lt, types_lt, params))
}

/// Extract lifetimes from generics (e.g., <'types, 'arena>)
fn extract_lifetimes_from_generics(generics: &syn::Generics) -> (Option<syn::Lifetime>, Option<syn::Lifetime>) {
    let mut arena_lt = None;
    let mut types_lt = None;

    for param in &generics.params {
        if let syn::GenericParam::Lifetime(lt_param) = param {
            let lt_name = lt_param.lifetime.ident.to_string();
            if lt_name == "arena" || lt_name == "a" {
                arena_lt = Some(lt_param.lifetime.clone());
            } else if lt_name == "types" || lt_name == "t" {
                types_lt = Some(lt_param.lifetime.clone());
            }
        }
    }

    (arena_lt, types_lt)
}

/// Collect remaining parameters from iterator
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

/// Parse the attribute to extract the name parameter
fn parse_attribute(attr: TokenStream) -> syn::Result<String> {
    // When used as #[melbi_fn(name = "FunctionName")], attr contains just: name = "FunctionName"
    // Parse it as a NameValue meta
    let meta = syn::parse::<Meta>(attr)?;

    if let Meta::NameValue(nv) = meta {
        if nv.path.is_ident("name") {
            if let Expr::Lit(expr_lit) = &nv.value {
                if let Lit::Str(lit) = &expr_lit.lit {
                    return Ok(lit.value());
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
        "expected attribute format: #[melbi_fn(name = \"FunctionName\")]",
    ))
}

/// Generate all the code: impl function, struct, and trait implementations
fn generate_code(
    melbi_name: &str,
    sig_info: &SignatureInfo,
    input_fn: &ItemFn,
) -> syn::Result<TokenStream2> {
    let struct_name = syn::Ident::new(melbi_name, proc_macro2::Span::call_site());

    // Extract components
    let param_names: Vec<_> = sig_info.params.iter().map(|(name, _)| name).collect();
    let param_types: Vec<_> = sig_info.params.iter().map(|(_, ty)| ty).collect();
    let return_type = &sig_info.return_type;

    // Determine if we should use user's generics or generate standard lifetimes
    let has_generics = !sig_info.generics.params.is_empty();

    // Copy input function as is.
    let impl_function = quote! {
        #input_fn
    };

    // Generate struct definition (only store the function type)
    // Use PhantomData to properly mark lifetime usage
    let struct_def = if has_generics {
        let generics = &sig_info.generics;
        let type_mgr_lifetime = &sig_info.type_mgr_lifetime;
        let arena_lifetime = &sig_info.arena_lifetime;
        quote! {
            pub struct #struct_name #generics
            where
                #type_mgr_lifetime: #arena_lifetime,
            {
                fn_type: & #type_mgr_lifetime ::melbi_core::types::Type< #type_mgr_lifetime >,
                _phantom_types: ::core::marker::PhantomData<& #type_mgr_lifetime ()>,
                _phantom_arena: ::core::marker::PhantomData<& #arena_lifetime ()>,
            }
        }
    } else {
        quote! {
            pub struct #struct_name<'types> {
                fn_type: &'types ::melbi_core::types::Type<'types>,
            }
        }
    };

    // Generate constructor
    let constructor = generate_constructor(&struct_name, sig_info, &param_types, return_type)?;

    // Generate Function trait impl
    let function_impl = generate_function_impl(
        &struct_name,
        sig_info,
        melbi_name,
        &param_names,
        &param_types,
        return_type,
    )?;

    // Generate AnnotatedFunction trait impl with inlined metadata
    // file!(), line!(), column!() will expand at the call site
    let annotated_impl = if has_generics {
        let generics = &sig_info.generics;
        let type_mgr_lifetime = &sig_info.type_mgr_lifetime;
        let arena_lifetime = &sig_info.arena_lifetime;
        quote! {
            impl #generics ::melbi_core::values::function::AnnotatedFunction< #type_mgr_lifetime > for #struct_name #generics
            where
                #type_mgr_lifetime: #arena_lifetime,
            {
                fn name(&self) -> &str {
                    #melbi_name
                }

                fn location(&self) -> (&str, &str, &str, u32, u32) {
                    (env!("CARGO_CRATE_NAME"), env!("CARGO_PKG_VERSION"), file!(), line!(), column!())
                }

                fn doc(&self) -> Option<&str> {
                    None
                }
            }
        }
    } else {
        quote! {
            impl<'types> ::melbi_core::values::function::AnnotatedFunction<'types> for #struct_name<'types> {
                fn name(&self) -> &str {
                    #melbi_name
                }

                fn location(&self) -> (&str, &str, &str, u32, u32) {
                    (env!("CARGO_CRATE_NAME"), env!("CARGO_PKG_VERSION"), file!(), line!(), column!())
                }

                fn doc(&self) -> Option<&str> {
                    None
                }
            }
        }
    };

    Ok(quote! {
        #impl_function

        #struct_def

        #constructor

        #function_impl

        #annotated_impl
    })
}

/// Generate the constructor method
fn generate_constructor(
    struct_name: &syn::Ident,
    sig_info: &SignatureInfo,
    param_types: &[&Box<Type>],
    return_type: &Type,
) -> syn::Result<TokenStream2> {
    let has_generics = !sig_info.generics.params.is_empty();

    // If return type is Result<T, E>, use T for the Melbi function type
    let result_ok_type = extract_result_ok_type(return_type);
    let melbi_return_type: &Type = result_ok_type.as_deref().unwrap_or(return_type);

    if has_generics {
        let generics = &sig_info.generics;
        let type_mgr_lifetime = &sig_info.type_mgr_lifetime;
        let arena_lifetime = &sig_info.arena_lifetime;

        Ok(quote! {
            impl #generics #struct_name #generics
            where
                #type_mgr_lifetime: #arena_lifetime,
            {
                pub fn new(type_mgr: & #type_mgr_lifetime ::melbi_core::types::manager::TypeManager< #type_mgr_lifetime >) -> Self {
                    use ::melbi_core::values::typed::Bridge;

                    let fn_type = type_mgr.function(
                        &[#( <#param_types as Bridge>::type_from(type_mgr) ),*],
                        <#melbi_return_type as Bridge>::type_from(type_mgr),
                    );

                    Self {
                        fn_type,
                        _phantom_types: ::core::marker::PhantomData,
                        _phantom_arena: ::core::marker::PhantomData,
                    }
                }
            }
        })
    } else {
        Ok(quote! {
            impl<'types> #struct_name<'types> {
                pub fn new(type_mgr: &'types ::melbi_core::types::manager::TypeManager<'types>) -> Self {
                    use ::melbi_core::values::typed::Bridge;

                    let fn_type = type_mgr.function(
                        &[#( <#param_types as Bridge>::type_from(type_mgr) ),*],
                        <#melbi_return_type as Bridge>::type_from(type_mgr),
                    );

                    Self {
                        fn_type,
                    }
                }
            }
        })
    }
}

/// Generate the user function call based on context mode
fn generate_user_fn_call(
    impl_fn_name: &syn::Ident,
    context_mode: ContextMode,
    param_names: &[&syn::Ident],
) -> TokenStream2 {
    match context_mode {
        ContextMode::Pure => {
            quote! { #impl_fn_name(#( #param_names ),*) }
        }
        ContextMode::ArenaOnly => {
            quote! { #impl_fn_name(ctx.arena(), #( #param_names ),*) }
        }
        ContextMode::TypeMgrOnly => {
            quote! { #impl_fn_name(ctx.type_mgr(), #( #param_names ),*) }
        }
        ContextMode::Legacy => {
            quote! { #impl_fn_name(ctx.arena(), ctx.type_mgr(), #( #param_names ),*) }
        }
        ContextMode::FullContext => {
            quote! { #impl_fn_name(ctx, #( #param_names ),*) }
        }
    }
}

/// Generate the Function trait implementation
fn generate_function_impl(
    struct_name: &syn::Ident,
    sig_info: &SignatureInfo,
    melbi_name: &str,
    param_names: &[&syn::Ident],
    param_types: &[&Box<Type>],
    return_type: &Type,
) -> syn::Result<TokenStream2> {
    let impl_fn_name = &sig_info.fn_name;
    let has_generics = !sig_info.generics.params.is_empty();
    let arity = param_names.len();
    let context_mode = sig_info.context_mode;

    // Check if return type is Result<T, E>
    let result_ok_type = extract_result_ok_type(return_type);
    let is_result = result_ok_type.is_some();
    let melbi_return_type: &Type = result_ok_type.as_deref().unwrap_or(return_type);

    // Generate parameter extraction code
    let param_extractions: Vec<_> = param_names.iter().zip(param_types.iter()).enumerate().map(|(i, (name, ty))| {
        quote! {
            let #name = unsafe { <#ty as ::melbi_core::values::typed::RawConvertible>::from_raw_value(args[#i].raw()) };
        }
    }).collect();

    // Generate the user function call based on context mode
    let user_fn_call = generate_user_fn_call(impl_fn_name, context_mode, param_names);

    // Generate result handling code based on whether return type is Result or not
    let result_handling = if is_result {
        // For Result<T, E>: map the error to ExecutionError and unwrap with ?
        quote! {
            let result = #user_fn_call
                .map_err(|e| ::melbi_core::evaluator::ExecutionError {
                    kind: e.into(),
                    // TODO: Add proper source and span information for native functions
                    source: ::alloc::string::String::new(),
                    span: ::melbi_core::parser::Span(0..0),
                })?;

            let raw = <#melbi_return_type as ::melbi_core::values::typed::RawConvertible>::to_raw_value(ctx.arena(), result);
            let ty = <#melbi_return_type as ::melbi_core::values::typed::Bridge>::type_from(ctx.type_mgr());

            // SAFETY: We just created the raw value from the correct type, so it matches
            Ok(::melbi_core::values::dynamic::Value::from_raw_unchecked(ty, raw))
        }
    } else {
        // For plain T: convert directly
        quote! {
            let result = #user_fn_call;

            let raw = <#melbi_return_type as ::melbi_core::values::typed::RawConvertible>::to_raw_value(ctx.arena(), result);
            let ty = <#melbi_return_type as ::melbi_core::values::typed::Bridge>::type_from(ctx.type_mgr());

            // SAFETY: We just created the raw value from the correct type, so it matches
            Ok(::melbi_core::values::dynamic::Value::from_raw_unchecked(ty, raw))
        }
    };

    if has_generics {
        let generics = &sig_info.generics;
        let arena_lifetime = &sig_info.arena_lifetime;
        let type_mgr_lifetime = &sig_info.type_mgr_lifetime;

        Ok(quote! {
            impl #generics ::melbi_core::values::function::Function< #type_mgr_lifetime, #arena_lifetime > for #struct_name #generics
            where
                #type_mgr_lifetime: #arena_lifetime,
            {
                fn ty(&self) -> & #type_mgr_lifetime ::melbi_core::types::Type< #type_mgr_lifetime > {
                    self.fn_type
                }

                unsafe fn call_unchecked(
                    &self,
                    ctx: & ::melbi_core::values::function::FfiContext< #type_mgr_lifetime, #arena_lifetime >,
                    args: &[::melbi_core::values::dynamic::Value< #type_mgr_lifetime, #arena_lifetime >],
                ) -> Result<::melbi_core::values::dynamic::Value< #type_mgr_lifetime, #arena_lifetime >, ::melbi_core::evaluator::ExecutionError> {
                    use ::melbi_core::values::typed::Bridge;

                    debug_assert_eq!(
                        args.len(),
                        #arity,
                        "{} expects {} argument(s), got {}",
                        #melbi_name,
                        #arity,
                        args.len()
                    );

                    #( #param_extractions )*

                    #result_handling
                }
            }
        })
    } else {
        Ok(quote! {
            impl<'types, 'arena> ::melbi_core::values::function::Function<'types, 'arena> for #struct_name<'types> {
                fn ty(&self) -> &'types ::melbi_core::types::Type<'types> {
                    self.fn_type
                }

                unsafe fn call_unchecked(
                    &self,
                    ctx: & ::melbi_core::values::function::FfiContext<'types, 'arena>,
                    args: &[::melbi_core::values::dynamic::Value<'types, 'arena>],
                ) -> Result<::melbi_core::values::dynamic::Value<'types, 'arena>, ::melbi_core::evaluator::ExecutionError> {
                    use ::melbi_core::values::typed::Bridge;

                    debug_assert_eq!(
                        args.len(),
                        #arity,
                        "{} expects {} argument(s), got {}",
                        #melbi_name,
                        #arity,
                        args.len()
                    );

                    #( #param_extractions )*

                    #result_handling
                }
            }
        })
    }
}
