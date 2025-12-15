//! Shared utilities for Melbi procedural macros.

use proc_macro2::TokenStream as TokenStream2;
use syn::{Attribute, Expr, Lit, Meta};

/// Result of extracting a Melbi name from an attribute.
pub enum NameExtraction {
    /// Explicit name provided via `name = "X"`
    Explicit(String),
    /// Name derived from function name (already case-converted)
    Derived(String),
    /// Attribute not found on this item
    NotFound,
}

/// Convert snake_case to PascalCase.
///
/// Examples:
/// - `add` -> `Add`
/// - `safe_div` -> `SafeDiv`
/// - `get_first_element` -> `GetFirstElement`
pub fn to_pascal_case(s: &str) -> String {
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

/// Convert snake_case to SCREAMING_SNAKE_CASE.
///
/// Examples:
/// - `pi` -> `PI`
/// - `euler_constant` -> `EULER_CONSTANT`
/// - `speed_of_light` -> `SPEED_OF_LIGHT`
pub fn to_screaming_snake_case(s: &str) -> String {
    s.to_ascii_uppercase()
}

/// Parse melbi_fn name from attribute tokens.
///
/// Returns explicit name if `name = "X"` is provided, otherwise derives from
/// function name as PascalCase.
pub fn parse_melbi_fn_name(tokens: TokenStream2, rust_fn_name: &str) -> syn::Result<String> {
    match parse_explicit_name(tokens)? {
        Some(name) => Ok(name),
        None => Ok(to_pascal_case(rust_fn_name)),
    }
}

/// Parse explicit name from attribute arguments.
///
/// Call this with the token content of a melbi attribute (what's inside the parens).
///
/// # Returns
/// - `Ok(Some(name))` if `name = "X"` is specified
/// - `Ok(None)` if empty (name should be derived)
/// - `Err(...)` if malformed
fn parse_explicit_name(tokens: TokenStream2) -> syn::Result<Option<String>> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let meta: Meta = syn::parse2(tokens)?;
    match meta {
        Meta::NameValue(nv) if nv.path.is_ident("name") => {
            if let Expr::Lit(expr_lit) = &nv.value {
                if let Lit::Str(lit) = &expr_lit.lit {
                    return Ok(Some(lit.value()));
                }
            }
            Err(syn::Error::new_spanned(
                &nv.value,
                "[melbi] name must be a string literal",
            ))
        }
        _ => Err(syn::Error::new_spanned(
            meta,
            "[melbi] expected `name = \"...\"` or no arguments",
        )),
    }
}

/// Extract Melbi name from an attribute on a function.
///
/// # Arguments
/// - `attrs`: The function's attributes
/// - `attr_name`: "melbi_fn" or "melbi_const"
/// - `rust_fn_name`: The Rust function name (for derivation)
///
/// # Returns
/// - `Ok(NotFound)` - attribute not present
/// - `Ok(Explicit(name))` - `#[attr(name = "X")]` found
/// - `Ok(Derived(name))` - `#[attr]` or `#[attr()]` found, name derived from rust_fn_name
/// - `Err(...)` - malformed attribute
pub fn extract_melbi_name(
    attrs: &[Attribute],
    attr_name: &str,
    rust_fn_name: &str,
) -> syn::Result<NameExtraction> {
    // Find attribute matching attr_name
    let attr = attrs.iter().find(|a| a.path().is_ident(attr_name));
    let Some(attr) = attr else {
        return Ok(NameExtraction::NotFound);
    };

    // Helper to derive the name based on attribute type
    let derive_name = || match attr_name {
        "melbi_fn" => to_pascal_case(rust_fn_name),
        "melbi_const" => to_screaming_snake_case(rust_fn_name),
        _ => rust_fn_name.to_string(),
    };

    // Extract tokens from the attribute
    let tokens = match &attr.meta {
        Meta::Path(_) => {
            // #[melbi_fn] - no arguments
            TokenStream2::new()
        }
        Meta::List(list) => {
            // #[melbi_fn(...)] - get the tokens inside parens
            list.tokens.clone()
        }
        Meta::NameValue(_) => {
            // #[melbi_fn = ...] - invalid syntax
            return Err(syn::Error::new_spanned(
                attr,
                "[melbi] invalid attribute syntax",
            ));
        }
    };

    // Use shared parsing logic
    match parse_explicit_name(tokens)? {
        Some(name) => Ok(NameExtraction::Explicit(name)),
        None => Ok(NameExtraction::Derived(derive_name())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("add"), "Add");
        assert_eq!(to_pascal_case("safe_div"), "SafeDiv");
        assert_eq!(to_pascal_case("get_first_element"), "GetFirstElement");
        // Edge cases - underscores are stripped
        assert_eq!(to_pascal_case("_private"), "Private");
        assert_eq!(to_pascal_case("foo__bar"), "FooBar");
    }

    #[test]
    fn test_to_screaming_snake_case() {
        assert_eq!(to_screaming_snake_case("pi"), "PI");
        assert_eq!(to_screaming_snake_case("euler_constant"), "EULER_CONSTANT");
        assert_eq!(to_screaming_snake_case("speed_of_light"), "SPEED_OF_LIGHT");
        // Edge cases - underscores are preserved
        assert_eq!(to_screaming_snake_case("_private"), "_PRIVATE");
        assert_eq!(to_screaming_snake_case("foo__bar"), "FOO__BAR");
    }
}
