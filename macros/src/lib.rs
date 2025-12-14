//! Procedural macros for Melbi FFI functions.
//!
//! This crate provides the `#[melbi_fn]` attribute macro for generating
//! type-safe FFI bindings between Rust and Melbi.

extern crate proc_macro;

use proc_macro::TokenStream;

mod melbi_fn;

/// Generate a type-safe FFI function for Melbi.
///
/// This attribute macro transforms a Rust function into a struct that implements
/// both [`Function`] and [`AnnotatedFunction`] traits, enabling type-safe calls
/// from Melbi code.
///
/// [`Function`]: melbi_core::values::function::Function
/// [`AnnotatedFunction`]: melbi_core::values::function::AnnotatedFunction
///
/// # Basic Example
///
/// ```ignore
/// use melbi_macros::melbi_fn;
///
/// #[melbi_fn]
/// fn add(a: i64, b: i64) -> i64 {
///     a + b
/// }
///
/// // Generated struct can be used as:
/// let add_fn = Add::new(type_mgr);
/// add_fn.register(arena, builder)?;
/// ```
///
/// # Naming
///
/// By default, the struct name is derived from the function name in PascalCase:
/// - `add` → `Add`
/// - `safe_div` → `SafeDiv`
/// - `get_first_element` → `GetFirstElement`
///
/// You can override this with the `name` attribute:
///
/// ```ignore
/// #[melbi_fn(name = "Sum")]
/// fn add_numbers(a: i64, b: i64) -> i64 {
///     a + b
/// }
/// // Generates struct `Sum` instead of `AddNumbers`
/// ```
///
/// # Context Access
///
/// If your function needs access to the arena or type manager (e.g., for allocating
/// strings or arrays), add `&FfiContext` as the first parameter:
///
/// ```ignore
/// use melbi_core::values::{FfiContext, typed::Str};
///
/// #[melbi_fn]
/// fn to_upper<'a>(ctx: &FfiContext<'a, 'a>, s: Str<'a>) -> Str<'a> {
///     Str::from_str(ctx.arena(), &s.to_ascii_uppercase())
/// }
/// ```
///
/// # Fallible Functions
///
/// Functions can return `Result<T, E>` where `E: Into<ExecutionErrorKind>`:
///
/// ```ignore
/// use melbi_core::evaluator::RuntimeError;
///
/// #[melbi_fn]
/// fn safe_div(a: i64, b: i64) -> Result<i64, RuntimeError> {
///     if b == 0 {
///         return Err(RuntimeError::DivisionByZero {});
///     }
///     Ok(a / b)
/// }
/// ```
///
/// The generated function type will be `(Int, Int) -> Int` (the `Result` is unwrapped).
///
/// # Supported Types
///
/// Parameters and return types must implement the [`Bridge`] trait.
///
/// [`Bridge`]: melbi_core::values::typed::Bridge
///
/// # Restrictions
///
/// - Functions must have an explicit return type
/// - At most one lifetime parameter is allowed
/// - Generic type parameters are not supported
/// - Pattern matching in parameters is not supported
#[proc_macro_attribute]
pub fn melbi_fn(attr: TokenStream, item: TokenStream) -> TokenStream {
    melbi_fn::melbi_fn_impl(attr, item)
}
