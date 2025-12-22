//! Implementation of the `#[melbi_fn]` attribute macro
//!
//! Parses function signatures, validates them, and directly generates the wrapper
//! struct and FFI glue code needed to bridge Rust functions into the Melbi runtime.
//!
//! We want to cover all kinds of polymorphism supported in Melbi, which are;
//!
//! 1. **No polymorphism:** All types are concrete types, so no polymorphism.
//!
//! 2. **Parametric polymorphism (Generics):** The same unaltered code works for
//!    any types (e.g. a function that takes `Array[T]` and returns its length).
//!    In this case T is opaque, you can't do much with it except pass it around.
//!
//! 3. **Constrained polymorphism:** For each generic argument, either Melbi
//!    automatically infers a trait bound from the body of the lambda, like
//!    `(x, y) => x + y` then `(T, T) => T where T: Numeric` is inferred. But
//!    for functions defined via FFI, these bounds need to be explicitly declared.
//!
//! To support polymorphism in FFI functions we'll use a combination of certain
//! strategies, depending on the type of parameter
//!
//! A. Unconstrained generic parameters:
//!    - They need no monomorphization.
//!    - The same identical code works for all types.
//!    - **How to handle:** Instantiate the generic code with a unique opaque
//!      type for each unique type variable. Example:
//!
//!       `get_or_default<K, V>(map: Map<K, V>, key: K, default: V) -> V`
//!
//!      which simply gets called as:
//!
//!       `get_or_default::<Any<0>, Any<1>>(map, key, default)`
//!
//! B. Constrained Generic Parameters:
//!    - The generic parameters are constrained by a trait.
//!    - The trait is implemented by a subset of types.
//!    - And similarly to Rust, there can be associated types.
//!    - These are split into two cases:
//!
//! B1. Expansion is both finite and small cardinality:
//!    - **How to handle:** In this case, we first expand the generic call
//!      using the types that implement the trait.
//!    - Example:
//!
//!      `index<C: Indexable<Index=I, Output=O>, I, O>(container: C, index: I) -> O`
//!
//!      And since we know only Array and Map implement Indexable, that expands into:
//!
//!      `index<E>(container: Array<E>, index: i64) -> E`
//!
//!      and
//!
//!      `index<K, V>(container: Map<K, V>, index: K) -> V`
//!
//!      and, then we can apply method A.
//!
//! B2. Expansion is infinite or there are too many cases to expand to.
//!     - The expansion is infinite when you can keep expanding and you'll never
//!       reach an unconstrained generic parameter. For instance, `T = Array[E]`
//!       satisfies `T: Hashable` when `E: Hashable`.
//!     - Traits implemented for a number of types or functions taking more than
//!       one trait, could have a very large number of instances when expanded.
//!       For instance:
//!
//!       `mix<T1: Ord, T2: Ord>(a: T1, b: T2, c: T2, d: T2) -> Bytes`
//!
//!       `Ord` is implemented for 4 types, so this would expand to 16 instantiations.
//!
//!     - **How to handle:** In this case, we use dynamic dispatch. Similar to
//!       approach A, but instead of `Any<0>` we use a similarly named type that
//!       contains its runtime type information.

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
    params: Vec<ParsedParam>,
    /// The "okay" return type - unwrapped if Result<T, E>
    ok_return_type: Box<Type>,
    /// Shape of the return type (concrete or type variable)
    return_shape: TypeShape,
    /// Whether the function returns Result<T, E>
    is_fallible: bool,
    /// Generic type parameters (e.g., T in `fn foo<T: Numeric>`)
    generic_params: Vec<ParsedGenericParam>,
}

/// A parsed function parameter with its type shape.
struct ParsedParam {
    name: syn::Ident,
    ty: Box<Type>,
    shape: TypeShape,
}

/// Parsed type parameter with its trait bound.
struct ParsedGenericParam {
    ident: syn::Ident,
    trait_name: String, // e.g., "Numeric"
}

/// Tracks how a parameter/return type uses type variables.
#[derive(Clone)]
enum TypeShape {
    /// Concrete type (e.g., i64, bool, Array<i64>)
    Concrete,
    /// Bare type variable (e.g., T where T: Numeric)
    TypeVar(syn::Ident),
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
                "[melbi] name attribute must be a string literal",
            ));
        }
        return Err(syn::Error::new_spanned(
            nv.path,
            "[melbi] expected 'name' attribute",
        ));
    }

    Err(syn::Error::new_spanned(
        meta,
        "[melbi] expected attribute format: #[melbi_fn] or #[melbi_fn(name = \"FunctionName\")]",
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

    // Parse generic parameters (lifetime and type params)
    let (lifetime, type_params) = parse_generics(&func.sig.generics)?;

    // Extract return type
    let return_type = parse_return_type(&func.sig)?;

    // Check if return type is Result<T, E> and extract okay type
    let (ok_return_type, is_fallible) = analyze_return_type(&return_type);

    // Classify return type shape
    let return_shape = classify_type(&ok_return_type, &type_params)?;

    // Detect context and extract business parameters
    let (has_context, params) = parse_params(&func.sig, &type_params)?;

    // Validate: each type parameter must be used in at least one input parameter
    // (needed for runtime dispatch)
    for tp in &type_params {
        let used_in_param = params
            .iter()
            .any(|p| matches!(&p.shape, TypeShape::TypeVar(id) if *id == tp.ident));
        if !used_in_param {
            return Err(syn::Error::new_spanned(
                &tp.ident,
                format!(
                    "[melbi] type parameter '{}' must be used in at least one input parameter \
                     for runtime dispatch",
                    tp.ident
                ),
            ));
        }
    }

    Ok(ParsedSignature {
        fn_name,
        lifetime,
        has_context,
        params,
        ok_return_type,
        return_shape,
        is_fallible,
        generic_params: type_params,
    })
}

/// Parse generic parameters: lifetime and type parameters.
///
/// Returns (lifetime, type_params).
fn parse_generics(
    generics: &syn::Generics,
) -> syn::Result<(Option<syn::Lifetime>, Vec<ParsedGenericParam>)> {
    let mut lifetime = Option::None;
    let mut type_params = Vec::new();

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
                // Phase 1: Only allow single type parameter
                if !type_params.is_empty() {
                    return Err(syn::Error::new_spanned(
                        type_param,
                        "[melbi] only single type parameter is currently supported",
                    ));
                }

                let parsed = parse_type_param(type_param)?;
                type_params.push(parsed);
            }
            GenericParam::Const(const_param) => {
                return Err(syn::Error::new_spanned(
                    const_param,
                    "[melbi] const generics are not supported",
                ));
            }
        };
    }
    Ok((lifetime, type_params))
}

/// Parse a type parameter and validate its trait bounds.
///
/// We only accept one trait, except for the `Melbi` trait, which doesn't impose
/// any constraints on the type, so we ignore it. For compound bounds like
/// `T: Numeric + Ord, for instance, we'll return an error.
fn parse_type_param(type_param: &syn::TypeParam) -> syn::Result<ParsedGenericParam> {
    let mut traits = Vec::new();

    // Collect all recognized trait bounds, keeping the most restrictive
    for bound in &type_param.bounds {
        if let syn::TypeParamBound::Trait(trait_bound) = bound {
            if let Some(last_seg) = trait_bound.path.segments.last() {
                let trait_name = last_seg.ident.to_string();
                match trait_name.as_str() {
                    // Type must be a number.
                    "Numeric" => traits.push("Numeric"),
                    // Type doesn't have any special constraints.
                    // So we don't even care about adding it.
                    "Melbi" => {}
                    other => {
                        return Err(syn::Error::new_spanned(
                            &trait_bound.path,
                            format!(
                                "[melbi] trait bound '{}' is not supported. \
                                 Supported: Melbi, Numeric",
                                other
                            ),
                        ));
                    }
                }
            }
        }
    }

    match traits.as_slice() {
        &[] => Err(syn::Error::new_spanned(
            type_param,
            "[melbi] generic type parameter must have a trait bound (e.g., T: Melbi or T: Numeric)",
        )),
        &[trait_name] => Ok(ParsedGenericParam {
            ident: type_param.ident.clone(),
            trait_name: trait_name.to_string(),
        }),
        &[..] => Err(syn::Error::new_spanned(
            type_param,
            "[melbi] multiple traits are not supported",
        )),
    }
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
fn parse_params(
    sig: &syn::Signature,
    type_params: &[ParsedGenericParam],
) -> syn::Result<(bool, Vec<ParsedParam>)> {
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

        let shape = classify_type(ty, type_params)?;
        params.push(ParsedParam {
            name: pat_ident.ident.clone(),
            ty: ty.clone(),
            shape,
        });
    }

    Ok((has_context, params))
}

/// Classify a type as concrete or a type variable.
fn classify_type(ty: &Type, type_params: &[ParsedGenericParam]) -> syn::Result<TypeShape> {
    if let Type::Path(type_path) = ty {
        // Check for bare type variable (e.g., T)
        if type_path.path.segments.len() == 1 {
            let seg = &type_path.path.segments[0];
            for tp in type_params {
                if seg.ident == tp.ident {
                    // Phase 1: reject type vars with generic args (e.g., Array<T>)
                    if !seg.arguments.is_empty() {
                        return Err(syn::Error::new_spanned(
                            ty,
                            "[melbi] type variables with generic arguments not yet supported",
                        ));
                    }
                    return Ok(TypeShape::TypeVar(tp.ident.clone()));
                }
            }
        }

        // For Phase 1, we only check immediate type arguments.
        // TODO(generic-ffi-phase2): Use recursive checking for nested containers.
        for seg in &type_path.path.segments {
            if let PathArguments::AngleBracketed(args) = &seg.arguments {
                for arg in &args.args {
                    if let GenericArgument::Type(inner_ty) = arg {
                        if contains_type_var(inner_ty, type_params) {
                            return Err(syn::Error::new_spanned(
                                ty,
                                "[melbi] type variables inside containers (e.g., Array<T>) not yet supported",
                            ));
                        }
                    }
                }
            }
        }
    }

    Ok(TypeShape::Concrete)
}

/// Check if a type contains any of the type parameters.
fn contains_type_var(ty: &Type, type_params: &[ParsedGenericParam]) -> bool {
    if let Type::Path(type_path) = ty {
        if type_path.path.segments.len() == 1 {
            let seg = &type_path.path.segments[0];
            if type_params.iter().any(|tp| tp.ident == seg.ident) {
                return true;
            }
        }
        // Recursively check generic arguments
        for seg in &type_path.path.segments {
            if let PathArguments::AngleBracketed(args) = &seg.arguments {
                for arg in &args.args {
                    if let GenericArgument::Type(inner_ty) = arg {
                        if contains_type_var(inner_ty, type_params) {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
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
fn generate_output(input_fn: &ItemFn, attr: &MelbiAttr, sig: &ParsedSignature) -> TokenStream2 {
    // Determine the struct name
    let struct_name = match &attr.name {
        Some(name) => syn::Ident::new(name, proc_macro2::Span::call_site()),
        None => {
            let pascal_name = to_pascal_case(&sig.fn_name.to_string());
            syn::Ident::new(&pascal_name, sig.fn_name.span())
        }
    };
    let struct_name_str = struct_name.to_string();

    // Determine the lifetime to use
    let lt = match &sig.lifetime {
        Some(lt) => lt.clone(),
        None => syn::Lifetime::new("'__a", proc_macro2::Span::call_site()),
    };

    let arity = sig.params.len();

    // Generate the type signature for `new()`
    let type_sig_body = generate_type_signature(sig);

    // Generate the call_unchecked body
    let call_body = generate_call_body(sig);

    quote! {
        #input_fn

        #[doc(hidden)]
        pub struct #struct_name<#lt> {
            __fn_type: &#lt ::melbi_core::types::Type<#lt>,
        }

        impl<#lt> #struct_name<#lt> {
            pub fn new(__type_mgr: &#lt ::melbi_core::types::manager::TypeManager<#lt>) -> Self {
                #type_sig_body
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

                #call_body
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

/// Generate the type signature for `new()`.
fn generate_type_signature(sig: &ParsedSignature) -> TokenStream2 {
    let ok_ty = &sig.ok_return_type;

    if sig.generic_params.is_empty() {
        // Non-generic: use Bridge::type_from for each param
        let param_types: Vec<_> = sig.params.iter().map(|p| &p.ty).collect();
        quote! {
            use ::melbi_core::values::typed::Bridge;
            let __fn_type = __type_mgr.function(
                &[#( <#param_types as Bridge>::type_from(__type_mgr) ),*],
                <#ok_ty as Bridge>::type_from(__type_mgr),
            );
            Self { __fn_type }
        }
    } else {
        // Generic: use fresh_type_var for type params
        let type_param = &sig.generic_params[0]; // Phase 1: single type param
        let type_var_name = syn::Ident::new(
            &format!("__typevar_{}", type_param.ident.to_string().to_lowercase()),
            type_param.ident.span(),
        );

        // Generate param types - use type var or Bridge::type_from
        let param_type_exprs: Vec<TokenStream2> = sig
            .params
            .iter()
            .map(|p| match &p.shape {
                TypeShape::TypeVar(_) => quote!(#type_var_name),
                TypeShape::Concrete => {
                    let ty = &p.ty;
                    quote!(<#ty as ::melbi_core::values::typed::Bridge>::type_from(__type_mgr))
                }
            })
            .collect();

        // Generate return type
        let return_type_expr = match &sig.return_shape {
            TypeShape::TypeVar(_) => quote!(#type_var_name),
            TypeShape::Concrete => {
                quote!(<#ok_ty as ::melbi_core::values::typed::Bridge>::type_from(__type_mgr))
            }
        };

        // TODO: include type-class constraint here.
        quote! {
            let #type_var_name = __type_mgr.fresh_type_var();
            let __fn_type = __type_mgr.function(
                &[#(#param_type_exprs),*],
                #return_type_expr,
            );
            Self { __fn_type }
        }
    }
}

/// Generate the call_unchecked body.
fn generate_call_body(sig: &ParsedSignature) -> TokenStream2 {
    if sig.generic_params.is_empty() {
        generate_monomorphic_call(sig)
    } else {
        generate_polymorphic_call(sig)
    }
}

/// Generate call body for non-generic functions.
fn generate_monomorphic_call(sig: &ParsedSignature) -> TokenStream2 {
    let fn_name = &sig.fn_name;
    let ok_ty = &sig.ok_return_type;

    let param_names: Vec<_> = sig.params.iter().map(|p| &p.name).collect();
    let param_types: Vec<_> = sig.params.iter().map(|p| &p.ty).collect();
    let param_indices: Vec<usize> = (0..sig.params.len()).collect();

    // Generate the function call expression
    let call_expr = if sig.has_context {
        quote! { #fn_name(__ctx, #(#param_names),*) }
    } else {
        quote! { #fn_name(#(#param_names),*) }
    };

    // Generate result handling
    let result_handling = generate_result_handling(sig.is_fallible);

    quote! {
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

/// Generate result handling code (for fallible vs infallible functions).
fn generate_result_handling(is_fallible: bool) -> TokenStream2 {
    if is_fallible {
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
    }
}

/// Generate call body for generic functions with runtime dispatch.
fn generate_polymorphic_call(sig: &ParsedSignature) -> TokenStream2 {
    let type_param = &sig.generic_params[0]; // Phase 1: single type param

    // Find the first parameter using this type var (the "representative")
    let rep_idx = sig
        .params
        .iter()
        .position(|p| matches!(&p.shape, TypeShape::TypeVar(id) if *id == type_param.ident))
        .expect("Type parameter must be used by at least one parameter");

    // Generate match arms based on trait bound
    let (match_arms, trait_display) = generate_dispatch_arms_for_trait(sig, type_param);

    quote! {
        match __args[#rep_idx].ty {
            #match_arms
            _ => {
                Err(::melbi_core::evaluator::ExecutionError {
                    kind: ::melbi_core::evaluator::ExecutionErrorKind::Runtime(
                        ::melbi_core::evaluator::RuntimeError::CastError {
                            message: alloc::format!(
                                "expected {}, got {}",
                                #trait_display,
                                __args[#rep_idx].ty
                            ),
                        }
                    ),
                    source: ::melbi_core::shim::String::new(),
                    span: ::melbi_core::parser::Span::new(0, 0),
                })
            }
        }
    }
}

/// Generate dispatch arms for a specific trait bound.
/// Returns (match_arms, trait_display_name).
fn generate_dispatch_arms_for_trait(
    sig: &ParsedSignature,
    type_param: &ParsedGenericParam,
) -> (TokenStream2, &'static str) {
    match type_param.trait_name.as_str() {
        "Numeric" => (
            generate_numeric_dispatch_arms(sig, &type_param.ident),
            "Numeric (Int or Float)",
        ),
        other => panic!("Unsupported trait: {}", other),
    }
}

/// Generate dispatch arms for Numeric trait (Int -> i64, Float -> f64).
fn generate_numeric_dispatch_arms(sig: &ParsedSignature, type_var: &syn::Ident) -> TokenStream2 {
    let int_arm = generate_dispatch_arm(
        sig,
        type_var,
        "Int",
        quote!(i64),
        quote!(__ctx.type_mgr().int()),
    );
    let float_arm = generate_dispatch_arm(
        sig,
        type_var,
        "Float",
        quote!(f64),
        quote!(__ctx.type_mgr().float()),
    );

    quote! {
        ::melbi_core::types::Type::Int => { #int_arm }
        ::melbi_core::types::Type::Float => { #float_arm }
    }
}

/// Generate a single dispatch arm for a concrete type.
fn generate_dispatch_arm(
    sig: &ParsedSignature,
    type_var: &syn::Ident,
    _type_name: &str,
    rust_type: TokenStream2,
    type_constructor: TokenStream2,
) -> TokenStream2 {
    let fn_name = &sig.fn_name;
    let ok_ty = &sig.ok_return_type;

    let param_names: Vec<_> = sig.params.iter().map(|p| &p.name).collect();

    // Generate parameter extractions - substitute type var with concrete type
    let param_extractions: Vec<TokenStream2> = sig
        .params
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let name = &p.name;
            let extract_ty = match &p.shape {
                TypeShape::TypeVar(id) if *id == *type_var => rust_type.clone(),
                _ => {
                    let ty = &p.ty;
                    quote!(#ty)
                }
            };
            quote! {
                let #name = unsafe {
                    <#extract_ty as RawConvertible>::from_raw_value(__args[#i].raw())
                };
            }
        })
        .collect();

    // Generate function call with turbofish
    let call_expr = if sig.has_context {
        quote! { #fn_name::<#rust_type>(__ctx, #(#param_names),*) }
    } else {
        quote! { #fn_name::<#rust_type>(#(#param_names),*) }
    };

    // Generate result handling
    let result_handling = generate_result_handling(sig.is_fallible);

    // Generate return value construction
    let return_construction = match &sig.return_shape {
        TypeShape::TypeVar(id) if *id == *type_var => {
            // Return type is the type variable - use the matched type
            quote! {
                let __raw = <#rust_type as RawConvertible>::to_raw_value(__ctx.arena(), __ok_result);
                let __ty = #type_constructor;
                Ok(::melbi_core::values::dynamic::Value::from_raw_unchecked(__ty, __raw))
            }
        }
        _ => {
            // Return type is concrete
            quote! {
                let __raw = <#ok_ty as RawConvertible>::to_raw_value(__ctx.arena(), __ok_result);
                let __ty = <#ok_ty as Bridge>::type_from(__ctx.type_mgr());
                Ok(::melbi_core::values::dynamic::Value::from_raw_unchecked(__ty, __raw))
            }
        }
    };

    quote! {
        #(#param_extractions)*
        let __call_result = #call_expr;
        #result_handling
        #return_construction
    }
}
