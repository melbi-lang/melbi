//! Implementation of the `#[melbi_const]` attribute macro
//!
//! This is a simple pass-through macro that serves as a marker for `#[melbi_package]`.
//! The `#[melbi_package]` macro scans for functions marked with `#[melbi_const]` and
//! includes them as constants in the generated package builder.

use proc_macro::TokenStream;

/// Pass-through implementation for `#[melbi_const]`.
///
/// This macro doesn't transform the function - it just returns it unchanged.
/// The attribute serves as a marker that `#[melbi_package]` recognizes.
///
/// # Example
///
/// ```ignore
/// #[melbi_const]  // Name automatically inferred
/// fn math_pi<'a>(type_mgr: &'a TypeManager<'a>) -> Value<'a, 'a> {
///     Value::float(type_mgr, core::f64::consts::PI)
/// }
///
/// #[melbi_const(name = "SPEED_OF_LIGHT")]  // Name explicitly provided
/// fn physics_speed_of_light<'a>(type_mgr: &'a TypeManager<'a>) -> Value<'a, 'a> {
///     Value::float(type_mgr, ...)
/// }

/// ```
pub fn melbi_const_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Just return the item unchanged - this is only a marker attribute
    item
}
