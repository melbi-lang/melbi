//! Implementation of the `#[melbi_package]` attribute macro
//!
//! This macro transforms a module containing `#[melbi_fn]` and `#[melbi_const]` items
//! into a package with an auto-generated builder function.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{Expr, Item, ItemMod, Lit, Meta, parse_macro_input};

use crate::common::{NameExtraction, extract_melbi_name};

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

/// Configuration parsed from `#[melbi_package(...)]` attribute
struct PackageConfig {
    /// Custom builder function name (if specified)
    builder_name: Option<String>,
}

pub fn melbi_package_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let config = match parse_package_attribute(attr) {
        Ok(config) => config,
        Err(err) => return err.to_compile_error().into(),
    };

    let input_mod = parse_macro_input!(item as ItemMod);

    match generate_package(&config, input_mod) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Parse the `#[melbi_package(...)]` attribute
fn parse_package_attribute(attr: TokenStream) -> syn::Result<PackageConfig> {
    if attr.is_empty() {
        return Ok(PackageConfig { builder_name: None });
    }

    let meta = syn::parse::<Meta>(attr)?;

    match meta {
        Meta::NameValue(nv) if nv.path.is_ident("builder") => {
            if let Expr::Lit(expr_lit) = &nv.value {
                if let Lit::Str(lit) = &expr_lit.lit {
                    return Ok(PackageConfig {
                        builder_name: Some(lit.value()),
                    });
                }
            }
            Err(syn::Error::new_spanned(
                &nv.value,
                "[melbi] builder attribute must be a string literal",
            ))
        }
        _ => Err(syn::Error::new_spanned(
            meta,
            "[melbi] expected #[melbi_package] or #[melbi_package(builder = \"name\")]",
        )),
    }
}

/// Generate the package with builder function
fn generate_package(config: &PackageConfig, mut input_mod: ItemMod) -> syn::Result<TokenStream2> {
    let mod_name = &input_mod.ident;
    let mod_vis = &input_mod.vis;
    let mod_attrs = &input_mod.attrs;

    // Determine builder function name
    let builder_name = match &config.builder_name {
        Some(name) => format_ident!("{}", name),
        None => format_ident!("build_{}_package", mod_name),
    };

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
            match extract_melbi_name(&item_fn.attrs, "melbi_fn", &fn_name)? {
                NameExtraction::Explicit(name) | NameExtraction::Derived(name) => {
                    functions.push(MelbiFnInfo { melbi_name: name });
                }
                NameExtraction::NotFound => {}
            }

            // Check for melbi_const attribute
            match extract_melbi_name(&item_fn.attrs, "melbi_const", &fn_name)? {
                NameExtraction::Explicit(name) | NameExtraction::Derived(name) => {
                    constants.push(MelbiConstInfo {
                        melbi_name: name,
                        rust_fn_name: item_fn.sig.ident.clone(),
                    });
                }
                NameExtraction::NotFound => {}
            }
        }
    }

    // Generate the builder function
    let builder_fn = generate_builder_function(&builder_name, &functions, &constants);

    // Append the builder function to the module content
    content.push(syn::parse2(builder_fn)?);

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
