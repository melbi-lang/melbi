//! Implementation of the `#[melbi_package]` attribute macro
//!
//! This macro transforms a module containing `#[melbi_fn]` and `#[melbi_const]` items
//! into a package with an auto-generated builder function.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{Ident, Item, ItemMod, parse_macro_input};

use crate::common::{get_name_from_item, get_name_from_tokens};

/// Information about a function marked with `#[melbi_fn]`
struct MelbiFnInfo {
    /// The Melbi name (explicit or derived)
    melbi_name: Ident,
}

/// Information about a constant marked with `#[melbi_const]`
struct MelbiConstInfo {
    /// The Melbi name (explicit or derived)
    melbi_name: Ident,
    /// The Rust function name
    rust_fn_name: Ident,
}

pub fn melbi_package_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_mod = parse_macro_input!(item as ItemMod);
    let mod_name = input_mod.ident.clone();

    // Get package name from `name` attribute or derive from module name (math -> Math)
    let package_name = match get_name_from_tokens(attr, "melbi_package", "name", &mod_name) {
        Ok(name) => name,
        Err(err) => return err.to_compile_error().into(),
    };

    // Derive function names from module name
    let functions_fn_name = format_ident!("register_{}_functions", mod_name);
    let package_fn_name = format_ident!("register_{}_package", mod_name);

    match generate_package(
        &package_name,
        &functions_fn_name,
        &package_fn_name,
        input_mod,
    ) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Generate the package with registration functions
fn generate_package(
    package_name: &Ident,
    functions_fn_name: &Ident,
    package_fn_name: &Ident,
    mut input_mod: ItemMod,
) -> syn::Result<TokenStream2> {
    // Get the module content
    let content = match &mut input_mod.content {
        Some((_, items)) => items,
        None => {
            return Err(syn::Error::new_spanned(
                &input_mod,
                "[melbi] melbi_package requires a module with inline content (not a file module)",
            ));
        }
    };

    // Collect melbi_fn and melbi_const items
    let mut functions: Vec<MelbiFnInfo> = Vec::new();
    let mut constants: Vec<MelbiConstInfo> = Vec::new();

    for item in content.iter() {
        if let Item::Fn(item_fn) = item {
            let fn_name = item_fn.sig.ident.clone();

            // Check for melbi_fn attribute
            if let Some(name) = get_name_from_item(&item_fn.attrs, "melbi_fn", "name", &fn_name)? {
                functions.push(MelbiFnInfo { melbi_name: name });
            }

            // Check for melbi_const attribute
            if let Some(name) = get_name_from_item(&item_fn.attrs, "melbi_const", "name", &fn_name)?
            {
                constants.push(MelbiConstInfo {
                    melbi_name: name,
                    rust_fn_name: fn_name,
                });
            }
        }
    }

    // Generate both registration functions
    let (functions_fn, package_fn) = generate_registration_functions(
        package_name,
        functions_fn_name,
        package_fn_name,
        &functions,
        &constants,
    );

    // Append both registration functions to the module content
    content.push(syn::parse2(functions_fn)?);
    content.push(syn::parse2(package_fn)?);

    let mod_name = &input_mod.ident;
    let mod_vis = &input_mod.vis;
    let mod_attrs = &input_mod.attrs;

    // Reconstruct the module
    Ok(quote! {
        #(#mod_attrs)*
        #mod_vis mod #mod_name {
            #(#content)*
        }
    })
}

/// Generate both registration functions:
/// - `register_<mod>_functions`: Registers functions directly to any Binder
/// - `register_<mod>_package`: Creates a Record and binds it to the Binder with the package name
///
/// Returns a tuple of (functions_fn, package_fn) as separate TokenStream2 values.
fn generate_registration_functions(
    package_name: &Ident,
    functions_fn_name: &Ident,
    package_fn_name: &Ident,
    functions: &[MelbiFnInfo],
    constants: &[MelbiConstInfo],
) -> (TokenStream2, TokenStream2) {
    // Generate constant registration code
    let const_registrations: Vec<_> = constants
        .iter()
        .map(|c| {
            let melbi_name = &c.melbi_name;
            let rust_fn = &c.rust_fn_name;
            quote! {
                builder = builder.bind(stringify!(#melbi_name), #rust_fn(arena, type_mgr));
            }
        })
        .collect();

    // Generate function registration code
    let fn_registrations: Vec<_> = functions
        .iter()
        .map(|f| {
            let struct_name = &f.melbi_name;
            quote! {
                builder = #struct_name::new(type_mgr).register(arena, builder);
            }
        })
        .collect();

    let package_name_str = package_name.to_string();

    let functions_fn = quote! {
        /// Registers all functions and constants from this package directly to a Binder.
        ///
        /// Use this to flatten the package's contents into a global environment or another record.
        pub fn #functions_fn_name<'arena, B>(
            arena: &'arena ::bumpalo::Bump,
            type_mgr: &'arena ::melbi_core::types::manager::TypeManager<'arena>,
            mut builder: B,
        ) -> B
        where
            B: ::melbi_core::values::binder::Binder<'arena, 'arena>,
        {
            use ::melbi_core::values::function::AnnotatedFunction;

            #(#const_registrations)*

            #(#fn_registrations)*

            builder
        }
    };

    let package_fn = quote! {
        /// Creates a Record containing all functions and constants, then binds it to the Binder.
        ///
        /// The record is bound with the package name (e.g., "Math").
        pub fn #package_fn_name<'arena, B>(
            arena: &'arena ::bumpalo::Bump,
            type_mgr: &'arena ::melbi_core::types::manager::TypeManager<'arena>,
            builder: B,
        ) -> B
        where
            B: ::melbi_core::values::binder::Binder<'arena, 'arena>,
        {
            use ::melbi_core::values::binder::Binder;

            let record_builder = ::melbi_core::values::dynamic::Value::record_builder(arena, type_mgr);
            let record = #functions_fn_name(arena, type_mgr, record_builder)
                .build()
                .expect("duplicate binding in package - check #[melbi_fn] names");
            builder.bind(#package_name_str, record)
        }
    };

    (functions_fn, package_fn)
}
