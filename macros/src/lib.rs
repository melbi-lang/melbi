//! Procedural macros for Melbi FFI functions
//!
//! This crate provides the `#[melbi_fn]` attribute macro for generating
//! type-safe FFI bindings between Rust and Melbi.

extern crate proc_macro;

use proc_macro::TokenStream;

mod melbi_fn;
mod melbi_fn_new;

/// Generate a type-safe FFI function for Melbi.
///
/// This attribute macro transforms a clean Rust function into a struct that
/// implements both `Function` and `AnnotatedFunction` traits, enabling
/// zero-cost type-safe calls from Melbi code.
///
/// # Example
///
/// ```ignore
/// #[melbi_fn(name = "Len")]
/// fn string_len(_arena: &Bump, _type_mgr: &TypeManager, s: Str) -> i64 {
///     s.chars().count() as i64
/// }
/// ```
///
/// This generates:
/// - Implementation function `string_len_impl`
/// - Struct `Len` with metadata (name, location, doc)
/// - `Function` trait implementation for runtime execution
/// - `AnnotatedFunction` trait implementation for registration
///
/// # Required Attribute
///
/// - `name`: The Melbi function name (string literal). This becomes the struct name.
///
/// # Parameters
///
/// Functions can accept any type that implements the `Bridge` trait:
/// - Primitives: `i64`, `f64`, `bool`
/// - Strings: `Str` (zero-copy wrapper)
/// - Collections: `Array<T>`, `Map<K, V>`
/// - Options: `Optional<T>`
///
/// The first two parameters should be `_arena: &Bump` and `_type_mgr: &TypeManager`
/// (can be omitted if unused).
///
/// # Returns
///
/// Functions must return a type that implements `Bridge`.
///
/// # Registration
///
/// ```ignore
/// // In package builder:
/// Len::new(type_mgr).register(arena, type_mgr, env)?;
/// ```
#[proc_macro_attribute]
pub fn melbi_fn(attr: TokenStream, item: TokenStream) -> TokenStream {
    melbi_fn::melbi_fn_impl(attr, item)
}

/// Generate a type-safe FFI function for Melbi (new implementation).
///
/// This is a simpler implementation that generates a call to the `melbi_fn_generate!`
/// declarative macro. The proc macro handles parsing and validation, while the
/// declarative macro handles code generation.
///
/// # Example
///
/// ```ignore
/// #[melbi_fn_new]  // name derived from fn name as PascalCase
/// fn safe_div(a: i64, b: i64) -> Result<i64, RuntimeError> {
///     if b == 0 { return Err(RuntimeError::DivisionByZero {}); }
///     Ok(a / b)
/// }
/// ```
///
/// This generates:
/// ```ignore
/// fn safe_div(...) { ... }  // Original function unchanged
///
/// melbi_fn_generate!(
///     name = SafeDiv,
///     fn_name = safe_div,
///     lt = '__a,
///     context = Pure,
///     signature = { a: i64, b: i64 } -> i64,
///     fallible = true
/// );
/// ```
///
/// # Attributes
///
/// - `name` (optional): Override the struct name. Default: PascalCase of fn name.
///
/// # Context Modes
///
/// The macro automatically detects the context mode from parameters:
/// - `Pure`: No context params - `fn add(a: i64, b: i64)`
/// - `ArenaOnly`: `fn alloc(arena: &Bump, ...)`
/// - `TypeMgrOnly`: `fn get_type(type_mgr: &TypeManager, ...)`
/// - `Legacy`: `fn old(arena: &Bump, type_mgr: &TypeManager, ...)`
/// - `FullContext`: `fn ctx_fn(ctx: &FfiContext, ...)`
#[proc_macro_attribute]
pub fn melbi_fn_new(attr: TokenStream, item: TokenStream) -> TokenStream {
    melbi_fn_new::melbi_fn_new_impl(attr, item)
}
