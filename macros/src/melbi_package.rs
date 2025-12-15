//! Implementation of the `#[melbi_package]` attribute macro
//!
//! This macro transforms a module containing `#[melbi_fn]` and `#[melbi_const]` items
//! into a package with an auto-generated builder function.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{Item, ItemMod, parse_macro_input};

use crate::common::{get_name_from_item, get_name_from_tokens};

/// Information about a function marked with `#[melbi_fn]`
struct MelbiFnInfo {
    /// The Melbi name (explicit or derived)
    melbi_name: String,
}

/// Information about a constant marked with `#[melbi_const]`
struct MelbiConstInfo {
    /// The Melbi name (explicit or derived)
    melbi_name: String,
    /// The Rust function name
    rust_fn_name: syn::Ident,
}

pub fn melbi_package_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_mod = parse_macro_input!(item as ItemMod);
    let mod_name = input_mod.ident.to_string();
    let builder_name = match get_name_from_tokens(attr, "melbi_package", "builder", &mod_name) {
        Ok(name) => name,
        Err(err) => return err.to_compile_error().into(),
    };

    match generate_package(&builder_name, input_mod) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Generate the package with builder function
fn generate_package(builder_name: &str, mut input_mod: ItemMod) -> syn::Result<TokenStream2> {
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
            let fn_name = item_fn.sig.ident.to_string();

            // Check for melbi_fn attribute
            if let Some(name) = get_name_from_item(&item_fn.attrs, "melbi_fn", "name", &fn_name)? {
                functions.push(MelbiFnInfo { melbi_name: name });
            }

            // Check for melbi_const attribute
            if let Some(name) = get_name_from_item(&item_fn.attrs, "melbi_const", "name", &fn_name)?
            {
                constants.push(MelbiConstInfo {
                    melbi_name: name,
                    rust_fn_name: item_fn.sig.ident.clone(),
                });
            }
        }
    }

    // Generate the builder function
    let builder_ident = format_ident!("{}", builder_name);
    let builder_fn = generate_builder_function(&builder_ident, &functions, &constants);

    // Append the builder function to the module content
    content.push(syn::parse2(builder_fn)?);

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

/// Generate the builder function
fn generate_builder_function(
    builder_name: &syn::Ident,
    functions: &[MelbiFnInfo],
    constants: &[MelbiConstInfo],
) -> TokenStream2 {
    // Generate constant registration code
    // Constants receive both arena and type_mgr to support all value types
    let const_registrations: Vec<_> = constants
        .iter()
        .map(|c| {
            let melbi_name = &c.melbi_name;
            let rust_fn = &c.rust_fn_name;
            quote! {
                builder = builder.field(#melbi_name, #rust_fn(arena, type_mgr));
            }
        })
        .collect();

    // Generate function registration code
    let fn_registrations: Vec<_> = functions
        .iter()
        .map(|f| {
            let struct_name = format_ident!("{}", f.melbi_name);
            quote! {
                builder = #struct_name::new(type_mgr).register(arena, builder)?;
            }
        })
        .collect();

    quote! {
        pub fn #builder_name<'arena>(
            arena: &'arena ::bumpalo::Bump,
            type_mgr: &'arena ::melbi_core::types::manager::TypeManager<'arena>,
        ) -> ::core::result::Result<
            ::melbi_core::values::dynamic::Value<'arena, 'arena>,
            ::melbi_core::values::from_raw::TypeError,
        > {
            use ::melbi_core::values::function::AnnotatedFunction;

            let mut builder = ::melbi_core::values::dynamic::Value::record_builder(type_mgr);

            #(#const_registrations)*

            #(#fn_registrations)*

            builder.build(arena)
        }
    }
}
