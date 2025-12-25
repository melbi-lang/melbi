//! Shared utilities for Melbi procedural macros.

use proc_macro2::TokenStream as TokenStream2;
use quote::format_ident;
use syn::{Attribute, Expr, Ident, Meta};

/// Parse a single `key = "value"` from attribute tokens.
///
/// # Arguments
/// - `tokens`: The token stream inside the attribute's parentheses.
/// - `key`: The identifier to look for (e.g., "name", "builder").
///
/// # Returns
/// - `Ok(Some(value))` if `key = "value"` is found.
/// - `Ok(None)` if the tokens are empty.
/// - `Err(...)` if malformed or another key is present.
pub(crate) fn parse_name_value(tokens: TokenStream2, key: &str) -> syn::Result<Option<Ident>> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let meta: Meta = syn::parse2(tokens)?;
    match meta {
        Meta::NameValue(nv) if nv.path.is_ident(key) => {
            // Handle `key = ident`
            if let Expr::Path(expr_path) = &nv.value {
                if let Some(ident) = expr_path.path.get_ident() {
                    return Ok(Some(ident.clone()));
                }
            }
            Err(syn::Error::new_spanned(
                &nv.value,
                format!("[melbi] {} must be an identifier", key),
            ))
        }
        _ => Err(syn::Error::new_spanned(
            meta,
            format!("[melbi] expected `{} = identifier`, or no arguments", key),
        )),
    }
}

/// Public entry-point for name resolution when inspecting an item's attributes.
///
/// # Arguments
/// - `item_attrs`: A slice of the item's attributes.
/// - `attr_name`: The name of the attribute to look for (e.g., "melbi_fn", "melbi_const").
/// - `rust_item_name`: The Rust item's name (e.g., function name) for derivation.
///
/// # Returns
/// - `Ok(Some(Ident))`: If the attribute is found and name resolved.
/// - `Ok(None)`: If the attribute is not found on the item.
/// - `Err(...)`: If the attribute is found but malformed.
pub(crate) fn get_name_from_item(
    item_attrs: &[Attribute],
    attr_name: &str,
    key: &str,
    rust_item_name: &Ident,
) -> syn::Result<Option<Ident>> {
    // Find attribute matching attr_name
    let attr = item_attrs.iter().find(|a| a.path().is_ident(attr_name));
    let Some(attr) = attr else {
        return Ok(None);
    };

    // Extract tokens from the attribute
    let tokens = match &attr.meta {
        Meta::Path(_) => TokenStream2::new(), // #[attr] - no arguments
        Meta::List(list) => list.tokens.clone(), // #[attr(...)] - get the tokens inside parens
        Meta::NameValue(_) => {
            // #[attr = ...] - invalid syntax for these attributes
            return Err(syn::Error::new_spanned(
                attr,
                format!("[melbi] invalid attribute syntax for `#[{}]`", attr_name),
            ));
        }
    };

    // Use the central resolve_name logic
    get_name_from_tokens(tokens.into(), attr_name, key, rust_item_name).map(Some)
}

/// Public entry-point for name resolution from macro attribute tokens.
///
/// # Arguments
/// - `attr_tokens`: The token stream passed directly to the attribute macro (e.g., `attr: TokenStream`).
/// - `attr_name`: The name of the attribute macro (e.g., "melbi_fn", "melbi_package").
/// - `item_name`: The Rust item's name (e.g., function name, module name) for derivation.
///
/// # Returns
/// The resolved Melbi name as a `Ident`.
pub(crate) fn get_name_from_tokens(
    attr_tokens: proc_macro::TokenStream,
    attr_name: &str,
    key: &str,
    item_name: &Ident,
) -> syn::Result<Ident> {
    // Parse the tokens for an explicit name using the common helper
    let explicit_name = parse_name_value(attr_tokens.into(), key)?;

    if let Some(name) = explicit_name {
        return Ok(name);
    }

    // If no explicit name, derive it based on the attribute type
    let derived_name_str = match attr_name {
        "melbi_fn" => to_pascal_case(&item_name.to_string()),
        "melbi_const" => to_screaming_snake_case(&item_name.to_string()),
        "melbi_package" => to_pascal_case(&item_name.to_string()), // math -> Math (package name)
        _ => item_name.to_string(), // Should not happen given the key match above
    };
    Ok(format_ident!(
        "{}",
        derived_name_str,
        span = item_name.span()
    ))
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
