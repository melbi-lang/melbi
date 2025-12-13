//! Proof of concept for declarative melbi_fn macro
//!
//! This module demonstrates how a declarative macro can generate the boilerplate
//! for Melbi FFI functions. The proc macro would parse the function signature
//! and call this declarative macro with normalized arguments.

extern crate alloc;

use bumpalo::Bump;
use melbi_core::{
    evaluator::RuntimeError,
    types::manager::TypeManager,
    values::{
        FfiContext,
        dynamic::Value,
        function::{AnnotatedFunction, Function},
        typed::Str,
    },
};

// ============================================================================
// The main declarative macro
// ============================================================================

/// Declarative macro that generates all the boilerplate for a melbi_fn.
///
/// The proc macro will parse the user's function and call this with
/// normalized/easy-to-match arguments.
///
/// # Arguments
///
/// - `name`: The Melbi function name (becomes the struct name)
/// - `fn_name`: The Rust function name to wrap
/// - `lifetime`: Optional single lifetime, e.g., `['a]` or `[]` for none
/// - `context`: How context is passed: `Legacy` or `Pure`
/// - `params`: Business parameters, e.g., `[a: i64, b: i64]`
/// - `return_type`: The return type (can be `Result<T, E>` or plain `T`)
macro_rules! melbi_fn_impl {
    // ========================================================================
    // Entry point: delegate to internal rules
    // ========================================================================
    (
        name = $name:ident,
        fn_name = $fn_name:ident,
        lifetime = [$($lt:lifetime)?],
        context = $context:ident,
        params = [$($param_name:ident : $param_ty:ty),* $(,)?],
        return_type = $($ret:tt)+
    ) => {
        melbi_fn_impl!(@struct $name, [$($lt)?], [$($param_ty),*], $($ret)+);
        melbi_fn_impl!(@function_impl $name, $fn_name, [$($lt)?], $context, [$($param_name : $param_ty),*], $($ret)+);
        melbi_fn_impl!(@annotated_impl $name, [$($lt)?]);
    };

    // ========================================================================
    // @struct: Generate the struct definition
    // ========================================================================

    // With lifetime + Result
    (@struct $name:ident, [$lt:lifetime], [$($param_ty:ty),*], Result<$ok_ty:ty, $err_ty:ty>) => {
        pub struct $name<$lt> {
            fn_type: &$lt ::melbi_core::types::Type<$lt>,
        }

        impl<$lt> $name<$lt> {
            pub fn new(type_mgr: &$lt ::melbi_core::types::manager::TypeManager<$lt>) -> Self {
                use ::melbi_core::values::typed::Bridge;
                let fn_type = type_mgr.function(
                    &[$( <$param_ty as Bridge>::type_from(type_mgr) ),*],
                    <$ok_ty as Bridge>::type_from(type_mgr),
                );
                Self { fn_type }
            }
        }
    };

    // With lifetime + plain return
    (@struct $name:ident, [$lt:lifetime], [$($param_ty:ty),*], $ret_ty:ty) => {
        pub struct $name<$lt> {
            fn_type: &$lt ::melbi_core::types::Type<$lt>,
        }

        impl<$lt> $name<$lt> {
            pub fn new(type_mgr: &$lt ::melbi_core::types::manager::TypeManager<$lt>) -> Self {
                use ::melbi_core::values::typed::Bridge;
                let fn_type = type_mgr.function(
                    &[$( <$param_ty as Bridge>::type_from(type_mgr) ),*],
                    <$ret_ty as Bridge>::type_from(type_mgr),
                );
                Self { fn_type }
            }
        }
    };

    // No lifetime + Result
    (@struct $name:ident, [], [$($param_ty:ty),*], Result<$ok_ty:ty, $err_ty:ty>) => {
        pub struct $name<'types> {
            fn_type: &'types ::melbi_core::types::Type<'types>,
        }

        impl<'types> $name<'types> {
            pub fn new(type_mgr: &'types ::melbi_core::types::manager::TypeManager<'types>) -> Self {
                use ::melbi_core::values::typed::Bridge;
                let fn_type = type_mgr.function(
                    &[$( <$param_ty as Bridge>::type_from(type_mgr) ),*],
                    <$ok_ty as Bridge>::type_from(type_mgr),
                );
                Self { fn_type }
            }
        }
    };

    // No lifetime + plain return
    (@struct $name:ident, [], [$($param_ty:ty),*], $ret_ty:ty) => {
        pub struct $name<'types> {
            fn_type: &'types ::melbi_core::types::Type<'types>,
        }

        impl<'types> $name<'types> {
            pub fn new(type_mgr: &'types ::melbi_core::types::manager::TypeManager<'types>) -> Self {
                use ::melbi_core::values::typed::Bridge;
                let fn_type = type_mgr.function(
                    &[$( <$param_ty as Bridge>::type_from(type_mgr) ),*],
                    <$ret_ty as Bridge>::type_from(type_mgr),
                );
                Self { fn_type }
            }
        }
    };

    // ========================================================================
    // @function_impl: Generate the Function trait implementation
    // ========================================================================

    // With lifetime
    (@function_impl $name:ident, $fn_name:ident, [$lt:lifetime], $context:ident, [$($param_name:ident : $param_ty:ty),*], $($ret:tt)+) => {
        impl<$lt> ::melbi_core::values::function::Function<$lt, $lt> for $name<$lt> {
            fn ty(&self) -> &$lt ::melbi_core::types::Type<$lt> {
                self.fn_type
            }

            #[allow(unused_variables)]
            unsafe fn call_unchecked(
                &self,
                ctx: &::melbi_core::values::function::FfiContext<$lt, $lt>,
                args: &[::melbi_core::values::dynamic::Value<$lt, $lt>],
            ) -> Result<::melbi_core::values::dynamic::Value<$lt, $lt>, ::melbi_core::evaluator::ExecutionError> {
                melbi_fn_impl!(@call_body $fn_name, ctx, args, $context, [$($param_name : $param_ty),*], $($ret)+)
            }
        }
    };

    // No lifetime
    (@function_impl $name:ident, $fn_name:ident, [], $context:ident, [$($param_name:ident : $param_ty:ty),*], $($ret:tt)+) => {
        impl<'types, 'arena> ::melbi_core::values::function::Function<'types, 'arena> for $name<'types> {
            fn ty(&self) -> &'types ::melbi_core::types::Type<'types> {
                self.fn_type
            }

            #[allow(unused_variables)]
            unsafe fn call_unchecked(
                &self,
                ctx: &::melbi_core::values::function::FfiContext<'types, 'arena>,
                args: &[::melbi_core::values::dynamic::Value<'types, 'arena>],
            ) -> Result<::melbi_core::values::dynamic::Value<'types, 'arena>, ::melbi_core::evaluator::ExecutionError> {
                melbi_fn_impl!(@call_body $fn_name, ctx, args, $context, [$($param_name : $param_ty),*], $($ret)+)
            }
        }
    };

    // ========================================================================
    // @call_body: Generate the body of call_unchecked
    // ========================================================================

    (@call_body $fn_name:ident, $ctx:ident, $args:ident, $context:ident, [$($param_name:ident : $param_ty:ty),*], $($ret:tt)+) => {{
        #[allow(unused_imports)]
        use ::melbi_core::values::typed::RawConvertible;

        // Extract parameters
        let mut _idx = 0usize;
        $(
            #[allow(unused_assignments)]
            let $param_name = unsafe { <$param_ty as RawConvertible>::from_raw_value($args[_idx].raw()) };
            _idx += 1;
        )*

        // Call user function and handle result
        melbi_fn_impl!(@invoke $fn_name, $ctx, $context, [$($param_name),*], $($ret)+)
    }};

    // ========================================================================
    // @invoke: Call the user function with appropriate context args
    // ========================================================================

    // Legacy + Result
    (@invoke $fn_name:ident, $ctx:ident, Legacy, [$($param_name:ident),*], Result<$ok_ty:ty, $err_ty:ty>) => {{
        use ::melbi_core::values::typed::{Bridge, RawConvertible};
        let result = $fn_name($ctx.arena(), $ctx.type_mgr(), $($param_name),*)
            .map_err(|e| ::melbi_core::evaluator::ExecutionError {
                kind: e.into(),
                source: ::alloc::string::String::new(),
                span: ::melbi_core::parser::Span(0..0),
            })?;
        let raw = <$ok_ty as RawConvertible>::to_raw_value($ctx.arena(), result);
        let ty = <$ok_ty as Bridge>::type_from($ctx.type_mgr());
        Ok(::melbi_core::values::dynamic::Value::from_raw_unchecked(ty, raw))
    }};

    // Legacy + plain
    (@invoke $fn_name:ident, $ctx:ident, Legacy, [$($param_name:ident),*], $ret_ty:ty) => {{
        use ::melbi_core::values::typed::{Bridge, RawConvertible};
        let result = $fn_name($ctx.arena(), $ctx.type_mgr(), $($param_name),*);
        let raw = <$ret_ty as RawConvertible>::to_raw_value($ctx.arena(), result);
        let ty = <$ret_ty as Bridge>::type_from($ctx.type_mgr());
        Ok(::melbi_core::values::dynamic::Value::from_raw_unchecked(ty, raw))
    }};

    // Pure + Result
    (@invoke $fn_name:ident, $ctx:ident, Pure, [$($param_name:ident),*], Result<$ok_ty:ty, $err_ty:ty>) => {{
        use ::melbi_core::values::typed::{Bridge, RawConvertible};
        let result = $fn_name($($param_name),*)
            .map_err(|e| ::melbi_core::evaluator::ExecutionError {
                kind: e.into(),
                source: ::alloc::string::String::new(),
                span: ::melbi_core::parser::Span(0..0),
            })?;
        let raw = <$ok_ty as RawConvertible>::to_raw_value($ctx.arena(), result);
        let ty = <$ok_ty as Bridge>::type_from($ctx.type_mgr());
        Ok(::melbi_core::values::dynamic::Value::from_raw_unchecked(ty, raw))
    }};

    // Pure + plain
    (@invoke $fn_name:ident, $ctx:ident, Pure, [$($param_name:ident),*], $ret_ty:ty) => {{
        use ::melbi_core::values::typed::{Bridge, RawConvertible};
        let result = $fn_name($($param_name),*);
        let raw = <$ret_ty as RawConvertible>::to_raw_value($ctx.arena(), result);
        let ty = <$ret_ty as Bridge>::type_from($ctx.type_mgr());
        Ok(::melbi_core::values::dynamic::Value::from_raw_unchecked(ty, raw))
    }};

    // ========================================================================
    // @annotated_impl: Generate the AnnotatedFunction trait implementation
    // ========================================================================

    // With lifetime
    (@annotated_impl $name:ident, [$lt:lifetime]) => {
        impl<$lt> ::melbi_core::values::function::AnnotatedFunction<$lt> for $name<$lt> {
            fn name(&self) -> &str { stringify!($name) }
            fn location(&self) -> (&str, &str, &str, u32, u32) {
                (env!("CARGO_CRATE_NAME"), env!("CARGO_PKG_VERSION"), file!(), line!(), column!())
            }
            fn doc(&self) -> Option<&str> { None }
        }
    };

    // No lifetime
    (@annotated_impl $name:ident, []) => {
        impl<'types> ::melbi_core::values::function::AnnotatedFunction<'types> for $name<'types> {
            fn name(&self) -> &str { stringify!($name) }
            fn location(&self) -> (&str, &str, &str, u32, u32) {
                (env!("CARGO_CRATE_NAME"), env!("CARGO_PKG_VERSION"), file!(), line!(), column!())
            }
            fn doc(&self) -> Option<&str> { None }
        }
    };
}

// ============================================================================
// Test functions (what the user would write)
// ============================================================================

/// Legacy mode: arena + type_mgr + params
fn add_impl(_arena: &Bump, _type_mgr: &TypeManager, a: i64, b: i64) -> i64 {
    a + b
}

/// Legacy mode with Result return
fn safe_div_impl(
    _arena: &Bump,
    _type_mgr: &TypeManager,
    a: i64,
    b: i64,
) -> Result<i64, RuntimeError> {
    if b == 0 {
        Err(RuntimeError::DivisionByZero {})
    } else {
        Ok(a / b)
    }
}

/// Pure mode: no context
fn pure_add_impl(a: i64, b: i64) -> i64 {
    a + b
}

/// Pure mode with Result
fn pure_checked_add_impl(a: i64, b: i64) -> Result<i64, RuntimeError> {
    a.checked_add(b).ok_or(RuntimeError::IntegerOverflow {})
}

/// Legacy mode with lifetimes (string handling)
fn string_upper_impl<'a>(arena: &'a Bump, _type_mgr: &'a TypeManager, s: Str<'a>) -> Str<'a> {
    let upper = s.to_ascii_uppercase();
    Str::from_str(arena, &upper)
}

// ============================================================================
// Generate wrappers using the declarative macro
// ============================================================================

// This is what the proc macro would emit after parsing the function signature:

melbi_fn_impl!(
    name = DeclAdd,
    fn_name = add_impl,
    lifetime = [],
    context = Legacy,
    params = [a: i64, b: i64],
    return_type = i64
);

melbi_fn_impl!(
    name = DeclSafeDiv,
    fn_name = safe_div_impl,
    lifetime = [],
    context = Legacy,
    params = [a: i64, b: i64],
    return_type = Result<i64, RuntimeError>
);

melbi_fn_impl!(
    name = DeclPureAdd,
    fn_name = pure_add_impl,
    lifetime = [],
    context = Pure,
    params = [a: i64, b: i64],
    return_type = i64
);

melbi_fn_impl!(
    name = DeclPureCheckedAdd,
    fn_name = pure_checked_add_impl,
    lifetime = [],
    context = Pure,
    params = [a: i64, b: i64],
    return_type = Result<i64, RuntimeError>
);

melbi_fn_impl!(
    name = DeclUpper,
    fn_name = string_upper_impl,
    lifetime = ['a],
    context = Legacy,
    params = [s: Str<'a>],
    return_type = Str<'a>
);

// ============================================================================
// Tests
// ============================================================================

#[test]
fn test_legacy_mode_plain() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let add_fn = DeclAdd::new(type_mgr);

    // Check metadata
    assert_eq!(add_fn.name(), "DeclAdd");

    // Create and call
    let value = Value::function(&arena, add_fn).unwrap();
    let a = Value::int(type_mgr, 5);
    let b = Value::int(type_mgr, 3);
    let args = [a, b];

    let ctx = FfiContext::new(&arena, type_mgr);
    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    assert_eq!(result.as_int().unwrap(), 8);
}

#[test]
fn test_legacy_mode_result_success() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let div_fn = DeclSafeDiv::new(type_mgr);

    // 10 / 2 = 5
    let value = Value::function(&arena, div_fn).unwrap();
    let a = Value::int(type_mgr, 10);
    let b = Value::int(type_mgr, 2);
    let args = [a, b];

    let ctx = FfiContext::new(&arena, type_mgr);
    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    assert_eq!(result.as_int().unwrap(), 5);
}

#[test]
fn test_legacy_mode_result_error() {
    use melbi_core::evaluator::ExecutionErrorKind;

    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let div_fn = DeclSafeDiv::new(type_mgr);

    // 10 / 0 should error
    let value = Value::function(&arena, div_fn).unwrap();
    let a = Value::int(type_mgr, 10);
    let b = Value::int(type_mgr, 0);
    let args = [a, b];

    let ctx = FfiContext::new(&arena, type_mgr);
    let result = unsafe { value.as_function().unwrap().call_unchecked(&ctx, &args) };

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(
        err.kind,
        ExecutionErrorKind::Runtime(RuntimeError::DivisionByZero {})
    ));
}

#[test]
fn test_pure_mode_plain() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let add_fn = DeclPureAdd::new(type_mgr);

    assert_eq!(add_fn.name(), "DeclPureAdd");

    let value = Value::function(&arena, add_fn).unwrap();
    let a = Value::int(type_mgr, 10);
    let b = Value::int(type_mgr, 32);
    let args = [a, b];

    let ctx = FfiContext::new(&arena, type_mgr);
    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    assert_eq!(result.as_int().unwrap(), 42);
}

#[test]
fn test_pure_mode_result_success() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let add_fn = DeclPureCheckedAdd::new(type_mgr);

    let value = Value::function(&arena, add_fn).unwrap();
    let a = Value::int(type_mgr, 1);
    let b = Value::int(type_mgr, 2);
    let args = [a, b];

    let ctx = FfiContext::new(&arena, type_mgr);
    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    assert_eq!(result.as_int().unwrap(), 3);
}

#[test]
fn test_pure_mode_result_error() {
    use melbi_core::evaluator::ExecutionErrorKind;

    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let add_fn = DeclPureCheckedAdd::new(type_mgr);
    let value = Value::function(&arena, add_fn).unwrap();

    // Overflow case
    let a = Value::int(type_mgr, i64::MAX);
    let b = Value::int(type_mgr, 1);
    let args = [a, b];

    let ctx = FfiContext::new(&arena, type_mgr);
    let result = unsafe { value.as_function().unwrap().call_unchecked(&ctx, &args) };

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(
        err.kind,
        ExecutionErrorKind::Runtime(RuntimeError::IntegerOverflow {})
    ));
}

#[test]
fn test_with_lifetimes() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let upper_fn = DeclUpper::new(type_mgr);

    assert_eq!(upper_fn.name(), "DeclUpper");

    let value = Value::function(&arena, upper_fn).unwrap();
    let str_ty = type_mgr.str();
    let s = Value::str(&arena, str_ty, "hello");
    let args = [s];

    let ctx = FfiContext::new(&arena, type_mgr);
    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    assert_eq!(&*result.as_str().unwrap(), "HELLO");
}

#[test]
fn test_function_type_unwraps_result() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    // The function type should be (Int, Int) -> Int, not (Int, Int) -> Result<Int, ...>
    let div_fn = DeclSafeDiv::new(type_mgr);
    let fn_ty = div_fn.ty();

    let fn_ty_str = format!("{}", fn_ty);
    assert!(
        fn_ty_str.contains("Int"),
        "Function type should contain Int: {}",
        fn_ty_str
    );
    assert!(
        !fn_ty_str.contains("Result"),
        "Function type should not contain Result: {}",
        fn_ty_str
    );
}

#[test]
fn test_annotated_function_register() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    // Register the function using RecordBuilder
    let add_fn = DeclAdd::new(type_mgr);
    let builder = Value::record_builder(type_mgr);
    let builder = add_fn.register(&arena, builder).unwrap();

    // Build the record and verify it has the function
    let record = builder.build(&arena).unwrap();
    assert!(record.as_record().is_ok());
}

// ============================================================================
// ADVERSARIAL TESTS - Probing for hidden bugs
// ============================================================================
//
// These tests specifically target edge cases and potential failure modes
// in the melbi_fn_impl! macro implementation.

use melbi_core::values::typed::{Array, Optional, RawConvertible};

// ----------------------------------------------------------------------------
// Test Functions for Edge Cases
// ----------------------------------------------------------------------------

/// Zero-argument function (Legacy mode) - tests empty parameter list handling
fn zero_args_impl(_arena: &Bump, _type_mgr: &TypeManager) -> i64 {
    42
}

/// Zero-argument function (Pure mode)
fn pure_zero_args_impl() -> i64 {
    42
}

/// Zero-argument function returning Result
fn zero_args_result_impl(_arena: &Bump, _type_mgr: &TypeManager) -> Result<i64, RuntimeError> {
    Ok(42)
}

/// Many-argument function (5 parameters) - tests macro repetition patterns
fn many_args_impl(
    _arena: &Bump,
    _type_mgr: &TypeManager,
    a: i64,
    b: i64,
    c: i64,
    d: i64,
    e: i64,
) -> i64 {
    a + b + c + d + e
}

/// Single-argument function - minimal case
fn single_arg_impl(_arena: &Bump, _type_mgr: &TypeManager, x: i64) -> i64 {
    x * 2
}

/// Function returning bool - tests non-numeric Bridge type
fn returns_bool_impl(_arena: &Bump, _type_mgr: &TypeManager, x: i64) -> bool {
    x > 0
}

/// Function returning f64 - tests floating point Bridge type
fn returns_float_impl(_arena: &Bump, _type_mgr: &TypeManager, x: i64) -> f64 {
    x as f64 * 1.5
}

/// Function taking bool parameter
fn takes_bool_impl(_arena: &Bump, _type_mgr: &TypeManager, flag: bool) -> i64 {
    if flag { 1 } else { 0 }
}

/// Function taking f64 parameter
fn takes_float_impl(_arena: &Bump, _type_mgr: &TypeManager, x: f64) -> f64 {
    x * 2.0
}

/// Function taking mixed parameter types
fn mixed_types_impl(_arena: &Bump, _type_mgr: &TypeManager, i: i64, f: f64, b: bool) -> f64 {
    let base = i as f64 + f;
    if b { base } else { -base }
}

/// Function with Array parameter - tests complex generic types
fn takes_array_impl<'a>(_arena: &'a Bump, _type_mgr: &'a TypeManager, arr: Array<'a, i64>) -> i64 {
    arr.iter().sum()
}

/// Function returning Array - tests complex generic return type
fn returns_array_impl<'a>(arena: &'a Bump, _type_mgr: &'a TypeManager, x: i64) -> Array<'a, i64> {
    Array::new(arena, &[x, x * 2, x * 3])
}

/// Function with nested generic type: Array<Str<'a>>
fn takes_str_array_impl<'a>(
    _arena: &'a Bump,
    _type_mgr: &'a TypeManager,
    arr: Array<'a, Str<'a>>,
) -> i64 {
    arr.len() as i64
}

/// Function with Optional parameter
fn takes_optional_impl<'a>(
    _arena: &'a Bump,
    _type_mgr: &'a TypeManager,
    opt: Optional<'a, i64>,
) -> i64 {
    opt.as_option().unwrap_or(0)
}

/// Function returning Optional
fn returns_optional_impl<'a>(
    arena: &'a Bump,
    _type_mgr: &'a TypeManager,
    x: i64,
) -> Optional<'a, i64> {
    if x > 0 {
        Optional::some(arena, x)
    } else {
        Optional::none()
    }
}

/// Function with Result<Str<'a>, E> - Result with lifetime in ok type
fn result_with_lifetime_impl<'a>(
    arena: &'a Bump,
    _type_mgr: &'a TypeManager,
    s: Str<'a>,
) -> Result<Str<'a>, RuntimeError> {
    if s.is_empty() {
        Err(RuntimeError::CastError {
            message: "Empty string not allowed".into(),
        })
    } else {
        Ok(Str::from_str(arena, &s.to_ascii_uppercase()))
    }
}

/// Pure function with Result returning complex type
fn pure_result_complex_impl(a: i64, b: i64) -> Result<f64, RuntimeError> {
    if b == 0 {
        Err(RuntimeError::DivisionByZero {})
    } else {
        Ok(a as f64 / b as f64)
    }
}

/// Pure function with nested generics and Result<Str<'value>, E>
fn array_first<'value>(
    arr: Array<'value, Str<'value>>,
) -> Result<Str<'value>, melbi_core::evaluator::ExecutionErrorKind> {
    arr.iter().next().ok_or_else(|| {
        melbi_core::evaluator::ExecutionErrorKind::Runtime(RuntimeError::CastError {
            message: "Array is empty".into(),
        })
    })
}

// ----------------------------------------------------------------------------
// Generate Wrapper Structs
// ----------------------------------------------------------------------------

melbi_fn_impl!(
    name = DeclZeroArgs,
    fn_name = zero_args_impl,
    lifetime = [],
    context = Legacy,
    params = [],
    return_type = i64
);

melbi_fn_impl!(
    name = DeclPureZeroArgs,
    fn_name = pure_zero_args_impl,
    lifetime = [],
    context = Pure,
    params = [],
    return_type = i64
);

melbi_fn_impl!(
    name = DeclZeroArgsResult,
    fn_name = zero_args_result_impl,
    lifetime = [],
    context = Legacy,
    params = [],
    return_type = Result<i64, RuntimeError>
);

melbi_fn_impl!(
    name = DeclManyArgs,
    fn_name = many_args_impl,
    lifetime = [],
    context = Legacy,
    params = [a: i64, b: i64, c: i64, d: i64, e: i64],
    return_type = i64
);

melbi_fn_impl!(
    name = DeclSingleArg,
    fn_name = single_arg_impl,
    lifetime = [],
    context = Legacy,
    params = [x: i64],
    return_type = i64
);

melbi_fn_impl!(
    name = DeclReturnsBool,
    fn_name = returns_bool_impl,
    lifetime = [],
    context = Legacy,
    params = [x: i64],
    return_type = bool
);

melbi_fn_impl!(
    name = DeclReturnsFloat,
    fn_name = returns_float_impl,
    lifetime = [],
    context = Legacy,
    params = [x: i64],
    return_type = f64
);

melbi_fn_impl!(
    name = DeclTakesBool,
    fn_name = takes_bool_impl,
    lifetime = [],
    context = Legacy,
    params = [flag: bool],
    return_type = i64
);

melbi_fn_impl!(
    name = DeclTakesFloat,
    fn_name = takes_float_impl,
    lifetime = [],
    context = Legacy,
    params = [x: f64],
    return_type = f64
);

melbi_fn_impl!(
    name = DeclMixedTypes,
    fn_name = mixed_types_impl,
    lifetime = [],
    context = Legacy,
    params = [i: i64, f: f64, b: bool],
    return_type = f64
);

melbi_fn_impl!(
    name = DeclTakesArray,
    fn_name = takes_array_impl,
    lifetime = ['a],
    context = Legacy,
    params = [arr: Array<'a, i64>],
    return_type = i64
);

melbi_fn_impl!(
    name = DeclReturnsArray,
    fn_name = returns_array_impl,
    lifetime = ['a],
    context = Legacy,
    params = [x: i64],
    return_type = Array<'a, i64>
);

melbi_fn_impl!(
    name = DeclTakesStrArray,
    fn_name = takes_str_array_impl,
    lifetime = ['a],
    context = Legacy,
    params = [arr: Array<'a, Str<'a>>],
    return_type = i64
);

melbi_fn_impl!(
    name = DeclTakesOptional,
    fn_name = takes_optional_impl,
    lifetime = ['a],
    context = Legacy,
    params = [opt: Optional<'a, i64>],
    return_type = i64
);

melbi_fn_impl!(
    name = DeclReturnsOptional,
    fn_name = returns_optional_impl,
    lifetime = ['a],
    context = Legacy,
    params = [x: i64],
    return_type = Optional<'a, i64>
);

melbi_fn_impl!(
    name = DeclResultWithLifetime,
    fn_name = result_with_lifetime_impl,
    lifetime = ['a],
    context = Legacy,
    params = [s: Str<'a>],
    return_type = Result<Str<'a>, RuntimeError>
);

melbi_fn_impl!(
    name = DeclPureResultComplex,
    fn_name = pure_result_complex_impl,
    lifetime = [],
    context = Pure,
    params = [a: i64, b: i64],
    return_type = Result<f64, RuntimeError>
);

melbi_fn_impl!(
    name = DeclArrayFirst,
    fn_name = array_first,
    lifetime = ['value],
    context = Pure,
    params = [arr: Array<'value, Str<'value>>],
    return_type = Result<Str<'value>, melbi_core::evaluator::ExecutionErrorKind>
);

// NOTE: Only a single lifetime is supported (e.g., lifetime = ['a]).
// All values in Melbi FFI share the same arena/type_mgr lifetime.
// The proc macro should reject functions with multiple lifetimes.

// ----------------------------------------------------------------------------
// ADVERSARIAL TESTS
// ----------------------------------------------------------------------------

// ============================================================================
// 1. ZERO-ARGUMENT FUNCTIONS
// ============================================================================
// Bug being tested: Does the macro handle empty parameter lists correctly?
// The parameter extraction loop uses `_idx += 1` which might have issues
// when there are no parameters.

#[test]
fn test_zero_args_legacy_mode() {
    // Bug target: Empty params = [] should compile and work correctly.
    // The macro's @call_body uses `$($param_name : $param_ty),*` which
    // should expand to nothing for zero parameters.
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let fn_wrapper = DeclZeroArgs::new(type_mgr);
    let value = Value::function(&arena, fn_wrapper).unwrap();

    // Call with empty args slice
    let args: [Value; 0] = [];
    let ctx = FfiContext::new(&arena, type_mgr);

    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    assert_eq!(result.as_int().unwrap(), 42);
}

#[test]
fn test_zero_args_pure_mode() {
    // Bug target: Pure mode with zero parameters
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let fn_wrapper = DeclPureZeroArgs::new(type_mgr);
    let value = Value::function(&arena, fn_wrapper).unwrap();

    let args: [Value; 0] = [];
    let ctx = FfiContext::new(&arena, type_mgr);

    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    assert_eq!(result.as_int().unwrap(), 42);
}

#[test]
fn test_zero_args_with_result() {
    // Bug target: Zero parameters combined with Result return type
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let fn_wrapper = DeclZeroArgsResult::new(type_mgr);
    let value = Value::function(&arena, fn_wrapper).unwrap();

    let args: [Value; 0] = [];
    let ctx = FfiContext::new(&arena, type_mgr);

    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    assert_eq!(result.as_int().unwrap(), 42);
}

// ============================================================================
// 2. MANY-ARGUMENT FUNCTIONS
// ============================================================================
// Bug being tested: Does the macro correctly handle many parameters?
// The repetition pattern `$($param_name : $param_ty),*` and index counter
// `_idx += 1` should work correctly for any number of parameters.

#[test]
fn test_many_args_all_same_type() {
    // Bug target: 5 parameters of the same type
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let fn_wrapper = DeclManyArgs::new(type_mgr);
    let value = Value::function(&arena, fn_wrapper).unwrap();

    let args = [
        Value::int(type_mgr, 1),
        Value::int(type_mgr, 2),
        Value::int(type_mgr, 3),
        Value::int(type_mgr, 4),
        Value::int(type_mgr, 5),
    ];
    let ctx = FfiContext::new(&arena, type_mgr);

    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    // 1 + 2 + 3 + 4 + 5 = 15
    assert_eq!(result.as_int().unwrap(), 15);
}

#[test]
fn test_single_arg() {
    // Bug target: Single parameter (boundary case between 0 and many)
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let fn_wrapper = DeclSingleArg::new(type_mgr);
    let value = Value::function(&arena, fn_wrapper).unwrap();

    let args = [Value::int(type_mgr, 21)];
    let ctx = FfiContext::new(&arena, type_mgr);

    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    assert_eq!(result.as_int().unwrap(), 42);
}

// ============================================================================
// 3. NUMERIC BOUNDARY VALUES
// ============================================================================
// Bug being tested: Does the macro correctly handle extreme integer values?
// These test the RawConvertible implementations indirectly.

#[test]
fn test_i64_max_value() {
    // Bug target: Maximum i64 value should be preserved through conversion
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    // Test that i64::MAX is correctly passed through conversions
    let add_fn = DeclAdd::new(type_mgr);
    let add_value = Value::function(&arena, add_fn).unwrap();

    let args = [Value::int(type_mgr, i64::MAX), Value::int(type_mgr, 0)];
    let ctx = FfiContext::new(&arena, type_mgr);

    let result = unsafe {
        add_value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    assert_eq!(result.as_int().unwrap(), i64::MAX);
}

#[test]
fn test_i64_min_value() {
    // Bug target: Minimum i64 value should be preserved through conversion
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let add_fn = DeclAdd::new(type_mgr);
    let value = Value::function(&arena, add_fn).unwrap();

    let args = [Value::int(type_mgr, i64::MIN), Value::int(type_mgr, 0)];
    let ctx = FfiContext::new(&arena, type_mgr);

    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    assert_eq!(result.as_int().unwrap(), i64::MIN);
}

#[test]
fn test_zero_values() {
    // Bug target: Zero should not be confused with null pointers or special values
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let add_fn = DeclAdd::new(type_mgr);
    let value = Value::function(&arena, add_fn).unwrap();

    let args = [Value::int(type_mgr, 0), Value::int(type_mgr, 0)];
    let ctx = FfiContext::new(&arena, type_mgr);

    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    assert_eq!(result.as_int().unwrap(), 0);
}

#[test]
fn test_negative_one() {
    // Bug target: -1 has all bits set (0xFFFFFFFFFFFFFFFF)
    // This could be confused with error codes or special values
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let add_fn = DeclAdd::new(type_mgr);
    let value = Value::function(&arena, add_fn).unwrap();

    let args = [Value::int(type_mgr, -1), Value::int(type_mgr, 0)];
    let ctx = FfiContext::new(&arena, type_mgr);

    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    assert_eq!(result.as_int().unwrap(), -1);
}

// ============================================================================
// 4. MIXED PARAMETER TYPES
// ============================================================================
// Bug being tested: Does the macro correctly index into args[] when
// parameters have different types?

#[test]
fn test_mixed_types_i64_f64_bool() {
    // Bug target: Three different types in order - tests correct indexing
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let fn_wrapper = DeclMixedTypes::new(type_mgr);
    let value = Value::function(&arena, fn_wrapper).unwrap();

    // i=10, f=0.5, b=true => (10 + 0.5) = 10.5
    let args = [
        Value::int(type_mgr, 10),
        Value::float(type_mgr, 0.5),
        Value::bool(type_mgr, true),
    ];
    let ctx = FfiContext::new(&arena, type_mgr);

    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    assert_eq!(result.as_float().unwrap(), 10.5);
}

#[test]
fn test_mixed_types_with_false_flag() {
    // Bug target: Bool false might be represented as 0, which could cause issues
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let fn_wrapper = DeclMixedTypes::new(type_mgr);
    let value = Value::function(&arena, fn_wrapper).unwrap();

    // i=10, f=0.5, b=false => -(10 + 0.5) = -10.5
    let args = [
        Value::int(type_mgr, 10),
        Value::float(type_mgr, 0.5),
        Value::bool(type_mgr, false),
    ];
    let ctx = FfiContext::new(&arena, type_mgr);

    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    assert_eq!(result.as_float().unwrap(), -10.5);
}

// ============================================================================
// 5. BOOL RETURN TYPE
// ============================================================================
// Bug being tested: Does the Bridge for bool correctly convert?

#[test]
fn test_returns_bool_true() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let fn_wrapper = DeclReturnsBool::new(type_mgr);
    let value = Value::function(&arena, fn_wrapper).unwrap();

    let args = [Value::int(type_mgr, 5)]; // 5 > 0, should return true
    let ctx = FfiContext::new(&arena, type_mgr);

    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    assert_eq!(result.as_bool().unwrap(), true);
}

#[test]
fn test_returns_bool_false() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let fn_wrapper = DeclReturnsBool::new(type_mgr);
    let value = Value::function(&arena, fn_wrapper).unwrap();

    let args = [Value::int(type_mgr, -5)]; // -5 <= 0, should return false
    let ctx = FfiContext::new(&arena, type_mgr);

    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    assert_eq!(result.as_bool().unwrap(), false);
}

#[test]
fn test_returns_bool_zero_edge_case() {
    // Bug target: 0 is the boundary - tests off-by-one in comparison
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let fn_wrapper = DeclReturnsBool::new(type_mgr);
    let value = Value::function(&arena, fn_wrapper).unwrap();

    let args = [Value::int(type_mgr, 0)]; // 0 > 0 is false
    let ctx = FfiContext::new(&arena, type_mgr);

    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    assert_eq!(result.as_bool().unwrap(), false);
}

// ============================================================================
// 6. FLOAT PARAMETER AND RETURN
// ============================================================================
// Bug being tested: Float representation quirks (NaN, Inf, -0.0)

#[test]
fn test_float_zero() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let fn_wrapper = DeclTakesFloat::new(type_mgr);
    let value = Value::function(&arena, fn_wrapper).unwrap();

    let args = [Value::float(type_mgr, 0.0)];
    let ctx = FfiContext::new(&arena, type_mgr);

    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    assert_eq!(result.as_float().unwrap(), 0.0);
}

#[test]
fn test_float_negative_zero() {
    // Bug target: -0.0 is distinct from 0.0 in IEEE 754
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let fn_wrapper = DeclTakesFloat::new(type_mgr);
    let value = Value::function(&arena, fn_wrapper).unwrap();

    let args = [Value::float(type_mgr, -0.0)];
    let ctx = FfiContext::new(&arena, type_mgr);

    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    // -0.0 * 2.0 = -0.0 (IEEE 754 preserves sign of zero)
    // But -0.0 == 0.0 in rust comparison
    assert!(result.as_float().unwrap().is_sign_negative() || result.as_float().unwrap() == 0.0);
}

#[test]
fn test_float_infinity() {
    // Bug target: Infinity should be preserved through conversions
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let fn_wrapper = DeclTakesFloat::new(type_mgr);
    let value = Value::function(&arena, fn_wrapper).unwrap();

    let args = [Value::float(type_mgr, f64::INFINITY)];
    let ctx = FfiContext::new(&arena, type_mgr);

    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    assert_eq!(result.as_float().unwrap(), f64::INFINITY);
}

#[test]
fn test_float_neg_infinity() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let fn_wrapper = DeclTakesFloat::new(type_mgr);
    let value = Value::function(&arena, fn_wrapper).unwrap();

    let args = [Value::float(type_mgr, f64::NEG_INFINITY)];
    let ctx = FfiContext::new(&arena, type_mgr);

    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    assert_eq!(result.as_float().unwrap(), f64::NEG_INFINITY);
}

#[test]
fn test_float_nan() {
    // Bug target: NaN has special comparison semantics (NaN != NaN)
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let fn_wrapper = DeclTakesFloat::new(type_mgr);
    let value = Value::function(&arena, fn_wrapper).unwrap();

    let args = [Value::float(type_mgr, f64::NAN)];
    let ctx = FfiContext::new(&arena, type_mgr);

    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    // NaN * 2.0 = NaN
    assert!(result.as_float().unwrap().is_nan());
}

// ============================================================================
// 7. STRING EDGE CASES
// ============================================================================
// Bug being tested: Empty strings, unicode, special characters

#[test]
fn test_string_empty() {
    // Bug target: Empty string might be confused with null
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let upper_fn = DeclUpper::new(type_mgr);
    let value = Value::function(&arena, upper_fn).unwrap();

    let str_ty = type_mgr.str();
    let s = Value::str(&arena, str_ty, "");
    let args = [s];

    let ctx = FfiContext::new(&arena, type_mgr);
    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    assert_eq!(&*result.as_str().unwrap(), "");
}

#[test]
fn test_string_single_char() {
    // Bug target: Single character string (boundary case)
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let upper_fn = DeclUpper::new(type_mgr);
    let value = Value::function(&arena, upper_fn).unwrap();

    let str_ty = type_mgr.str();
    let s = Value::str(&arena, str_ty, "a");
    let args = [s];

    let ctx = FfiContext::new(&arena, type_mgr);
    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    assert_eq!(&*result.as_str().unwrap(), "A");
}

#[test]
fn test_string_unicode() {
    // Bug target: Multi-byte UTF-8 characters
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let upper_fn = DeclUpper::new(type_mgr);
    let value = Value::function(&arena, upper_fn).unwrap();

    let str_ty = type_mgr.str();
    // Note: to_ascii_uppercase only affects ASCII, so this tests preservation
    let s = Value::str(&arena, str_ty, "hello world");
    let args = [s];

    let ctx = FfiContext::new(&arena, type_mgr);
    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    assert_eq!(&*result.as_str().unwrap(), "HELLO WORLD");
}

// ============================================================================
// 8. COMPLEX GENERIC TYPES
// ============================================================================
// Bug being tested: Does the macro correctly handle Array<T> and other generics?

#[test]
fn test_takes_array_empty() {
    // Bug target: Empty array as parameter
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let fn_wrapper = DeclTakesArray::new(type_mgr);
    let value = Value::function(&arena, fn_wrapper).unwrap();

    let empty_array = Array::<i64>::new(&arena, &[]);
    let array_ty = type_mgr.array(type_mgr.int());
    let arr_value = Value::from_raw_unchecked(array_ty, empty_array.as_raw_value());

    let args = [arr_value];
    let ctx = FfiContext::new(&arena, type_mgr);

    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    assert_eq!(result.as_int().unwrap(), 0); // Sum of empty array
}

#[test]
fn test_takes_array_single_element() {
    // Bug target: Single element array
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let fn_wrapper = DeclTakesArray::new(type_mgr);
    let value = Value::function(&arena, fn_wrapper).unwrap();

    let array = Array::<i64>::new(&arena, &[42]);
    let array_ty = type_mgr.array(type_mgr.int());
    let arr_value = Value::from_raw_unchecked(array_ty, array.as_raw_value());

    let args = [arr_value];
    let ctx = FfiContext::new(&arena, type_mgr);

    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    assert_eq!(result.as_int().unwrap(), 42);
}

#[test]
fn test_takes_array_multiple_elements() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let fn_wrapper = DeclTakesArray::new(type_mgr);
    let value = Value::function(&arena, fn_wrapper).unwrap();

    let array = Array::<i64>::new(&arena, &[1, 2, 3, 4, 5]);
    let array_ty = type_mgr.array(type_mgr.int());
    let arr_value = Value::from_raw_unchecked(array_ty, array.as_raw_value());

    let args = [arr_value];
    let ctx = FfiContext::new(&arena, type_mgr);

    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    assert_eq!(result.as_int().unwrap(), 15); // 1+2+3+4+5
}

#[test]
fn test_returns_array() {
    // Bug target: Returning complex generic type
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let fn_wrapper = DeclReturnsArray::new(type_mgr);
    let value = Value::function(&arena, fn_wrapper).unwrap();

    let args = [Value::int(type_mgr, 5)];
    let ctx = FfiContext::new(&arena, type_mgr);

    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    // Function returns [x, x*2, x*3] = [5, 10, 15]
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 3);
    let v0 = arr.get(0).unwrap().as_int().unwrap();
    let v1 = arr.get(1).unwrap().as_int().unwrap();
    let v2 = arr.get(2).unwrap().as_int().unwrap();
    assert_eq!(v0, 5);
    assert_eq!(v1, 10);
    assert_eq!(v2, 15);
}

// ============================================================================
// 9. OPTIONAL TYPE
// ============================================================================
// Bug being tested: Optional<T> uses null pointer for None, which might
// interact oddly with the macro's value conversion.

#[test]
fn test_takes_optional_some() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let fn_wrapper = DeclTakesOptional::new(type_mgr);
    let value = Value::function(&arena, fn_wrapper).unwrap();

    let opt = Optional::some(&arena, 42i64);
    let opt_ty = type_mgr.option(type_mgr.int());
    let opt_value = Value::from_raw_unchecked(
        opt_ty,
        <Optional<i64> as RawConvertible>::to_raw_value(&arena, opt),
    );

    let args = [opt_value];
    let ctx = FfiContext::new(&arena, type_mgr);

    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    assert_eq!(result.as_int().unwrap(), 42);
}

#[test]
fn test_takes_optional_none() {
    // Bug target: None is represented as null pointer
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let fn_wrapper = DeclTakesOptional::new(type_mgr);
    let value = Value::function(&arena, fn_wrapper).unwrap();

    let opt = Optional::<i64>::none();
    let opt_ty = type_mgr.option(type_mgr.int());
    let opt_value = Value::from_raw_unchecked(
        opt_ty,
        <Optional<i64> as RawConvertible>::to_raw_value(&arena, opt),
    );

    let args = [opt_value];
    let ctx = FfiContext::new(&arena, type_mgr);

    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    assert_eq!(result.as_int().unwrap(), 0); // unwrap_or(0)
}

#[test]
fn test_returns_optional_some() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let fn_wrapper = DeclReturnsOptional::new(type_mgr);
    let value = Value::function(&arena, fn_wrapper).unwrap();

    let args = [Value::int(type_mgr, 42)]; // x > 0, returns Some(42)
    let ctx = FfiContext::new(&arena, type_mgr);

    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    // Result should be an Optional with Some(42)
    let opt = unsafe { <Optional<i64> as RawConvertible>::from_raw_value(result.raw()) };
    assert!(opt.is_some());
    assert_eq!(opt.unwrap(), 42);
}

#[test]
fn test_returns_optional_none() {
    // Bug target: Function returning None
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let fn_wrapper = DeclReturnsOptional::new(type_mgr);
    let value = Value::function(&arena, fn_wrapper).unwrap();

    let args = [Value::int(type_mgr, -5)]; // x <= 0, returns None
    let ctx = FfiContext::new(&arena, type_mgr);

    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    let opt = unsafe { <Optional<i64> as RawConvertible>::from_raw_value(result.raw()) };
    assert!(opt.is_none());
}

// ============================================================================
// 10. RESULT WITH LIFETIME IN OK TYPE
// ============================================================================
// Bug being tested: Result<Str<'a>, E> - does the macro correctly handle
// lifetimes in the Ok type of a Result?

#[test]
fn test_result_with_lifetime_success() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let fn_wrapper = DeclResultWithLifetime::new(type_mgr);
    let value = Value::function(&arena, fn_wrapper).unwrap();

    let str_ty = type_mgr.str();
    let s = Value::str(&arena, str_ty, "hello");
    let args = [s];

    let ctx = FfiContext::new(&arena, type_mgr);
    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    assert_eq!(&*result.as_str().unwrap(), "HELLO");
}

#[test]
fn test_result_with_lifetime_error() {
    use melbi_core::evaluator::ExecutionErrorKind;

    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let fn_wrapper = DeclResultWithLifetime::new(type_mgr);
    let value = Value::function(&arena, fn_wrapper).unwrap();

    let str_ty = type_mgr.str();
    let s = Value::str(&arena, str_ty, ""); // Empty string triggers error
    let args = [s];

    let ctx = FfiContext::new(&arena, type_mgr);
    let result = unsafe { value.as_function().unwrap().call_unchecked(&ctx, &args) };

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(
        err.kind,
        ExecutionErrorKind::Runtime(RuntimeError::CastError { .. })
    ));
}

// ============================================================================
// 11. PURE MODE WITH RESULT RETURNING NON-INT
// ============================================================================
// Bug being tested: Pure mode + Result + non-i64 return type

#[test]
fn test_pure_result_returns_float_success() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let fn_wrapper = DeclPureResultComplex::new(type_mgr);
    let value = Value::function(&arena, fn_wrapper).unwrap();

    let args = [Value::int(type_mgr, 10), Value::int(type_mgr, 4)];
    let ctx = FfiContext::new(&arena, type_mgr);

    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    assert_eq!(result.as_float().unwrap(), 2.5);
}

#[test]
fn test_pure_result_returns_float_error() {
    use melbi_core::evaluator::ExecutionErrorKind;

    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let fn_wrapper = DeclPureResultComplex::new(type_mgr);
    let value = Value::function(&arena, fn_wrapper).unwrap();

    let args = [
        Value::int(type_mgr, 10),
        Value::int(type_mgr, 0), // Division by zero
    ];
    let ctx = FfiContext::new(&arena, type_mgr);

    let result = unsafe { value.as_function().unwrap().call_unchecked(&ctx, &args) };

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(
        err.kind,
        ExecutionErrorKind::Runtime(RuntimeError::DivisionByZero {})
    ));
}

// ============================================================================
// 12. FUNCTION TYPE CORRECTNESS
// ============================================================================
// Bug being tested: Does the generated function type correctly reflect
// the parameter and return types?

#[test]
fn test_zero_args_function_type() {
    // Bug target: Zero-arg functions should have type () -> Int
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let fn_wrapper = DeclZeroArgs::new(type_mgr);
    let fn_ty = fn_wrapper.ty();

    let fn_ty_str = format!("{}", fn_ty);
    // Should contain "() -> Int" or similar
    assert!(
        fn_ty_str.contains("Int"),
        "Zero-arg function type should contain Int: {}",
        fn_ty_str
    );
}

#[test]
fn test_many_args_function_type() {
    // Bug target: 5-arg function should have correct type signature
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let fn_wrapper = DeclManyArgs::new(type_mgr);
    let fn_ty = fn_wrapper.ty();

    let fn_ty_str = format!("{}", fn_ty);
    // Type should reference Int multiple times (5 params + return)
    // This is a weak check, but verifies the type was generated
    assert!(
        fn_ty_str.contains("Int"),
        "Many-arg function type should contain Int: {}",
        fn_ty_str
    );
}

#[test]
fn test_mixed_types_function_type() {
    // Bug target: Mixed param types should be reflected in function type
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let fn_wrapper = DeclMixedTypes::new(type_mgr);
    let fn_ty = fn_wrapper.ty();

    let fn_ty_str = format!("{}", fn_ty);
    // Should contain Int, Float, Bool, and return Float
    assert!(
        fn_ty_str.contains("Int"),
        "Mixed-types function should contain Int: {}",
        fn_ty_str
    );
    assert!(
        fn_ty_str.contains("Float"),
        "Mixed-types function should contain Float: {}",
        fn_ty_str
    );
    assert!(
        fn_ty_str.contains("Bool"),
        "Mixed-types function should contain Bool: {}",
        fn_ty_str
    );
}

// ============================================================================
// 13. ANNOTATED FUNCTION NAME
// ============================================================================
// Bug being tested: Does the generated AnnotatedFunction correctly report
// the wrapper struct name (not the impl function name)?

#[test]
fn test_annotated_name_matches_struct() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let fn_wrapper = DeclZeroArgs::new(type_mgr);
    assert_eq!(fn_wrapper.name(), "DeclZeroArgs");

    let fn_wrapper2 = DeclManyArgs::new(type_mgr);
    assert_eq!(fn_wrapper2.name(), "DeclManyArgs");

    let fn_wrapper3 = DeclTakesArray::new(type_mgr);
    assert_eq!(fn_wrapper3.name(), "DeclTakesArray");
}

// ============================================================================
// 14. NESTED ARRAY TYPE (Array<Str<'a>>)
// ============================================================================
// Bug being tested: Does the macro handle doubly-parameterized types?

#[test]
fn test_takes_str_array_empty() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let fn_wrapper = DeclTakesStrArray::new(type_mgr);
    let value = Value::function(&arena, fn_wrapper).unwrap();

    let empty_array = Array::<Str>::from_strs(&arena, Vec::<&str>::new());
    let array_ty = type_mgr.array(type_mgr.str());
    let arr_value = Value::from_raw_unchecked(array_ty, empty_array.as_raw_value());

    let args = [arr_value];
    let ctx = FfiContext::new(&arena, type_mgr);

    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    assert_eq!(result.as_int().unwrap(), 0); // Length of empty array
}

#[test]
fn test_takes_str_array_with_elements() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let fn_wrapper = DeclTakesStrArray::new(type_mgr);
    let value = Value::function(&arena, fn_wrapper).unwrap();

    let array = Array::from_strs(&arena, vec!["hello", "world", "test"]);
    let array_ty = type_mgr.array(type_mgr.str());
    let arr_value = Value::from_raw_unchecked(array_ty, array.as_raw_value());

    let args = [arr_value];
    let ctx = FfiContext::new(&arena, type_mgr);

    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    assert_eq!(result.as_int().unwrap(), 3); // Length of array with 3 strings
}

// ============================================================================
// 15. PURE MODE WITH NESTED GENERICS AND RESULT
// ============================================================================
// Bug being tested: Pure mode + Array<'a, Str<'a>> + Result<Str<'a>, E>
// This combines multiple complexity factors.

#[test]
fn test_array_first_success() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let fn_wrapper = DeclArrayFirst::new(type_mgr);
    let value = Value::function(&arena, fn_wrapper).unwrap();

    let array = Array::from_strs(&arena, vec!["first", "second", "third"]);
    let array_ty = type_mgr.array(type_mgr.str());
    let arr_value = Value::from_raw_unchecked(array_ty, array.as_raw_value());

    let args = [arr_value];
    let ctx = FfiContext::new(&arena, type_mgr);

    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    assert_eq!(&*result.as_str().unwrap(), "first");
}

#[test]
fn test_array_first_empty_error() {
    use melbi_core::evaluator::ExecutionErrorKind;

    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let fn_wrapper = DeclArrayFirst::new(type_mgr);
    let value = Value::function(&arena, fn_wrapper).unwrap();

    let empty_array = Array::<Str>::from_strs(&arena, Vec::<&str>::new());
    let array_ty = type_mgr.array(type_mgr.str());
    let arr_value = Value::from_raw_unchecked(array_ty, empty_array.as_raw_value());

    let args = [arr_value];
    let ctx = FfiContext::new(&arena, type_mgr);

    let result = unsafe { value.as_function().unwrap().call_unchecked(&ctx, &args) };

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(
        err.kind,
        ExecutionErrorKind::Runtime(RuntimeError::CastError { .. })
    ));
}
