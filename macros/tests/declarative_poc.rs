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
    // @struct: Generate the struct definition (extract bridge type, then normalize lifetime)
    // ========================================================================

    // Extract bridge type from Result<T, E> -> T
    (@struct $name:ident, [$($lt:lifetime)?], [$($param_ty:ty),*], Result<$ok_ty:ty, $err_ty:ty>) => {
        melbi_fn_impl!(@struct_lt $name, [$($lt)?], [$($param_ty),*], $ok_ty);
    };
    // Plain return type -> use as-is
    (@struct $name:ident, [$($lt:lifetime)?], [$($param_ty:ty),*], $ret_ty:ty) => {
        melbi_fn_impl!(@struct_lt $name, [$($lt)?], [$($param_ty),*], $ret_ty);
    };

    // Normalize lifetime: user-provided
    (@struct_lt $name:ident, [$lt:lifetime], [$($param_ty:ty),*], $bridge_ty:ty) => {
        melbi_fn_impl!(@struct_body $name, $lt, [$($param_ty),*], $bridge_ty);
    };
    // Normalize lifetime: default to 'types
    (@struct_lt $name:ident, [], [$($param_ty:ty),*], $bridge_ty:ty) => {
        melbi_fn_impl!(@struct_body $name, 'types, [$($param_ty),*], $bridge_ty);
    };

    // Actual struct generation (single rule)
    (@struct_body $name:ident, $lt:lifetime, [$($param_ty:ty),*], $bridge_ty:ty) => {
        pub struct $name<$lt> {
            fn_type: &$lt ::melbi_core::types::Type<$lt>,
        }

        impl<$lt> $name<$lt> {
            pub fn new(type_mgr: &$lt ::melbi_core::types::manager::TypeManager<$lt>) -> Self {
                use ::melbi_core::values::typed::Bridge;
                let fn_type = type_mgr.function(
                    &[$( <$param_ty as Bridge>::type_from(type_mgr) ),*],
                    <$bridge_ty as Bridge>::type_from(type_mgr),
                );
                Self { fn_type }
            }
        }
    };

    // ========================================================================
    // @function_impl: Generate the Function trait implementation
    // ========================================================================

    // Normalize lifetime: user-provided
    (@function_impl $name:ident, $fn_name:ident, [$lt:lifetime], $context:ident, [$($param_name:ident : $param_ty:ty),*], $($ret:tt)+) => {
        melbi_fn_impl!(@function_impl_body $name, $fn_name, $lt, $context, [$($param_name : $param_ty),*], $($ret)+);
    };
    // Normalize lifetime: default to 'types
    (@function_impl $name:ident, $fn_name:ident, [], $context:ident, [$($param_name:ident : $param_ty:ty),*], $($ret:tt)+) => {
        melbi_fn_impl!(@function_impl_body $name, $fn_name, 'types, $context, [$($param_name : $param_ty),*], $($ret)+);
    };

    // Actual implementation (single rule)
    (@function_impl_body $name:ident, $fn_name:ident, $lt:lifetime, $context:ident, [$($param_name:ident : $param_ty:ty),*], $($ret:tt)+) => {
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
    (@invoke $fn_name:ident, $ctx:ident, Legacy, [$($param_name:ident),*], Result<$ok_ty:ty, $err_ty:ty>) => {
        melbi_fn_impl!(@wrap_result $ctx, $ok_ty,
            $fn_name($ctx.arena(), $ctx.type_mgr(), $($param_name),*))
    };
    // Legacy + plain
    (@invoke $fn_name:ident, $ctx:ident, Legacy, [$($param_name:ident),*], $ret_ty:ty) => {
        melbi_fn_impl!(@wrap_ok $ctx, $ret_ty,
            $fn_name($ctx.arena(), $ctx.type_mgr(), $($param_name),*))
    };
    // Pure + Result
    (@invoke $fn_name:ident, $ctx:ident, Pure, [$($param_name:ident),*], Result<$ok_ty:ty, $err_ty:ty>) => {
        melbi_fn_impl!(@wrap_result $ctx, $ok_ty, $fn_name($($param_name),*))
    };
    // Pure + plain
    (@invoke $fn_name:ident, $ctx:ident, Pure, [$($param_name:ident),*], $ret_ty:ty) => {
        melbi_fn_impl!(@wrap_ok $ctx, $ret_ty, $fn_name($($param_name),*))
    };

    // @wrap_result: Handle Result return (map_err + unwrap ok value)
    (@wrap_result $ctx:ident, $ok_ty:ty, $call:expr) => {{
        use ::melbi_core::values::typed::{Bridge, RawConvertible};
        let result = $call.map_err(|e| ::melbi_core::evaluator::ExecutionError {
            kind: e.into(),
            source: ::alloc::string::String::new(),
            span: ::melbi_core::parser::Span(0..0),
        })?;
        let raw = <$ok_ty as RawConvertible>::to_raw_value($ctx.arena(), result);
        let ty = <$ok_ty as Bridge>::type_from($ctx.type_mgr());
        Ok(::melbi_core::values::dynamic::Value::from_raw_unchecked(ty, raw))
    }};

    // @wrap_ok: Handle plain return (just wrap value)
    (@wrap_ok $ctx:ident, $ret_ty:ty, $call:expr) => {{
        use ::melbi_core::values::typed::{Bridge, RawConvertible};
        let result = $call;
        let raw = <$ret_ty as RawConvertible>::to_raw_value($ctx.arena(), result);
        let ty = <$ret_ty as Bridge>::type_from($ctx.type_mgr());
        Ok(::melbi_core::values::dynamic::Value::from_raw_unchecked(ty, raw))
    }};

    // ========================================================================
    // @annotated_impl: Generate the AnnotatedFunction trait implementation
    // ========================================================================

    // Normalize lifetime: user-provided
    (@annotated_impl $name:ident, [$lt:lifetime]) => {
        melbi_fn_impl!(@annotated_body $name, $lt);
    };
    // Normalize lifetime: default to 'types
    (@annotated_impl $name:ident, []) => {
        melbi_fn_impl!(@annotated_body $name, 'types);
    };

    // Actual implementation (single rule)
    (@annotated_body $name:ident, $lt:lifetime) => {
        impl<$lt> ::melbi_core::values::function::AnnotatedFunction<$lt> for $name<$lt> {
            fn name(&self) -> &'static str { stringify!($name) }
            fn location(&self) -> (&'static str, &'static str, &'static str, u32, u32) {
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
// Test Helpers
// ============================================================================

use melbi_core::evaluator::{ExecutionError, ExecutionErrorKind};

/// Test context providing arena and type manager for FFI tests.
struct TestCtx<'a> {
    arena: &'a Bump,
    type_mgr: &'a TypeManager<'a>,
}

impl<'a> TestCtx<'a> {
    fn new(arena: &'a Bump) -> Self {
        let type_mgr = TypeManager::new(arena);
        Self { arena, type_mgr }
    }

    fn int(&self, v: i64) -> Value<'a, 'a> {
        Value::int(self.type_mgr, v)
    }

    fn float(&self, v: f64) -> Value<'a, 'a> {
        Value::float(self.type_mgr, v)
    }

    fn bool(&self, v: bool) -> Value<'a, 'a> {
        Value::bool(self.type_mgr, v)
    }

    fn str(&self, s: &str) -> Value<'a, 'a> {
        Value::str(self.arena, self.type_mgr.str(), s)
    }

    fn int_array(&self, values: &[i64]) -> Value<'a, 'a> {
        let array = Array::<i64>::new(self.arena, values);
        let ty = self.type_mgr.array(self.type_mgr.int());
        Value::from_raw_unchecked(ty, array.as_raw_value())
    }

    fn str_array(&self, values: Vec<&str>) -> Value<'a, 'a> {
        let array = Array::from_strs(self.arena, values);
        let ty = self.type_mgr.array(self.type_mgr.str());
        Value::from_raw_unchecked(ty, array.as_raw_value())
    }

    fn optional_int(&self, v: Option<i64>) -> Value<'a, 'a> {
        let opt = match v {
            Some(x) => Optional::some(self.arena, x),
            None => Optional::<i64>::none(),
        };
        let ty = self.type_mgr.option(self.type_mgr.int());
        Value::from_raw_unchecked(
            ty,
            <Optional<i64> as RawConvertible>::to_raw_value(self.arena, opt),
        )
    }

    fn call<F: Function<'a, 'a> + AnnotatedFunction<'a> + 'a>(
        &self,
        f: F,
        args: &[Value<'a, 'a>],
    ) -> Result<Value<'a, 'a>, ExecutionError> {
        let value = Value::function(self.arena, f).unwrap();
        let ctx = FfiContext::new(self.arena, self.type_mgr);
        unsafe { value.as_function().unwrap().call_unchecked(&ctx, args) }
    }

    fn call_ok<F: Function<'a, 'a> + AnnotatedFunction<'a> + 'a>(
        &self,
        f: F,
        args: &[Value<'a, 'a>],
    ) -> Value<'a, 'a> {
        self.call(f, args).expect("expected successful call")
    }
}

// ============================================================================
// Tests
// ============================================================================

#[test]
fn test_legacy_mode_plain() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);

    let add_fn = DeclAdd::new(ctx.type_mgr);
    assert_eq!(add_fn.name(), "DeclAdd");

    let result = ctx.call_ok(add_fn, &[ctx.int(5), ctx.int(3)]);
    assert_eq!(result.as_int().unwrap(), 8);
}

#[test]
fn test_legacy_mode_result_success() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);

    let result = ctx.call_ok(DeclSafeDiv::new(ctx.type_mgr), &[ctx.int(10), ctx.int(2)]);
    assert_eq!(result.as_int().unwrap(), 5);
}

#[test]
fn test_legacy_mode_result_error() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);

    let err = ctx
        .call(DeclSafeDiv::new(ctx.type_mgr), &[ctx.int(10), ctx.int(0)])
        .unwrap_err();
    assert!(matches!(
        err.kind,
        ExecutionErrorKind::Runtime(RuntimeError::DivisionByZero {})
    ));
}

#[test]
fn test_pure_mode_plain() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);

    let add_fn = DeclPureAdd::new(ctx.type_mgr);
    assert_eq!(add_fn.name(), "DeclPureAdd");

    let result = ctx.call_ok(add_fn, &[ctx.int(10), ctx.int(32)]);
    assert_eq!(result.as_int().unwrap(), 42);
}

#[test]
fn test_pure_mode_result_success() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);

    let result = ctx.call_ok(
        DeclPureCheckedAdd::new(ctx.type_mgr),
        &[ctx.int(1), ctx.int(2)],
    );
    assert_eq!(result.as_int().unwrap(), 3);
}

#[test]
fn test_pure_mode_result_error() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);

    let err = ctx
        .call(
            DeclPureCheckedAdd::new(ctx.type_mgr),
            &[ctx.int(i64::MAX), ctx.int(1)],
        )
        .unwrap_err();
    assert!(matches!(
        err.kind,
        ExecutionErrorKind::Runtime(RuntimeError::IntegerOverflow {})
    ));
}

#[test]
fn test_with_lifetimes() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);

    let upper_fn = DeclUpper::new(ctx.type_mgr);
    assert_eq!(upper_fn.name(), "DeclUpper");

    let result = ctx.call_ok(upper_fn, &[ctx.str("hello")]);
    assert_eq!(&*result.as_str().unwrap(), "HELLO");
}

#[test]
fn test_function_type_unwraps_result() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);

    // The function type should be (Int, Int) -> Int, not (Int, Int) -> Result<Int, ...>
    let div_fn = DeclSafeDiv::new(ctx.type_mgr);
    let fn_ty_str = format!("{}", div_fn.ty());

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
    let ctx = TestCtx::new(&arena);

    let add_fn = DeclAdd::new(ctx.type_mgr);
    let builder = Value::record_builder(ctx.type_mgr);
    let builder = add_fn.register(ctx.arena, builder).unwrap();

    let record = builder.build(ctx.arena).unwrap();
    assert!(record.as_record().is_ok());
}

// ============================================================================
// ADVERSARIAL TESTS - Probing for hidden bugs
// ============================================================================
//
// These tests specifically target edge cases and potential failure modes
// in the melbi_fn_impl! macro implementation.

use melbi_core::values::typed::{Array, Optional, RawConvertible};

/// Helper to extract Optional<i64> from a Value
fn extract_optional_int(v: &Value) -> Option<i64> {
    v.as_option().unwrap().map(|inner| inner.as_int().unwrap())
}

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

// 1. ZERO-ARGUMENT FUNCTIONS
// Tests that empty params = [] works correctly.

#[test]
fn test_zero_args_legacy_mode() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    assert_eq!(
        ctx.call_ok(DeclZeroArgs::new(ctx.type_mgr), &[])
            .as_int()
            .unwrap(),
        42
    );
}

#[test]
fn test_zero_args_pure_mode() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    assert_eq!(
        ctx.call_ok(DeclPureZeroArgs::new(ctx.type_mgr), &[])
            .as_int()
            .unwrap(),
        42
    );
}

#[test]
fn test_zero_args_with_result() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    assert_eq!(
        ctx.call_ok(DeclZeroArgsResult::new(ctx.type_mgr), &[])
            .as_int()
            .unwrap(),
        42
    );
}

// 2. MANY-ARGUMENT FUNCTIONS
// Tests macro repetition patterns with multiple parameters.

#[test]
fn test_many_args_all_same_type() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    let args = [ctx.int(1), ctx.int(2), ctx.int(3), ctx.int(4), ctx.int(5)];
    assert_eq!(
        ctx.call_ok(DeclManyArgs::new(ctx.type_mgr), &args)
            .as_int()
            .unwrap(),
        15
    );
}

#[test]
fn test_single_arg() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    assert_eq!(
        ctx.call_ok(DeclSingleArg::new(ctx.type_mgr), &[ctx.int(21)])
            .as_int()
            .unwrap(),
        42
    );
}

// 3. NUMERIC BOUNDARY VALUES
// Tests extreme integer values through RawConvertible.

#[test]
fn test_i64_max_value() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    assert_eq!(
        ctx.call_ok(DeclAdd::new(ctx.type_mgr), &[ctx.int(i64::MAX), ctx.int(0)])
            .as_int()
            .unwrap(),
        i64::MAX
    );
}

#[test]
fn test_i64_min_value() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    assert_eq!(
        ctx.call_ok(DeclAdd::new(ctx.type_mgr), &[ctx.int(i64::MIN), ctx.int(0)])
            .as_int()
            .unwrap(),
        i64::MIN
    );
}

#[test]
fn test_zero_values() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    assert_eq!(
        ctx.call_ok(DeclAdd::new(ctx.type_mgr), &[ctx.int(0), ctx.int(0)])
            .as_int()
            .unwrap(),
        0
    );
}

#[test]
fn test_negative_one() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    // -1 has all bits set (0xFFFFFFFFFFFFFFFF)
    assert_eq!(
        ctx.call_ok(DeclAdd::new(ctx.type_mgr), &[ctx.int(-1), ctx.int(0)])
            .as_int()
            .unwrap(),
        -1
    );
}

// 4. MIXED PARAMETER TYPES
// Tests correct indexing into args[] with different types.

#[test]
fn test_mixed_types_i64_f64_bool() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    // i=10, f=0.5, b=true => (10 + 0.5) = 10.5
    let result = ctx.call_ok(
        DeclMixedTypes::new(ctx.type_mgr),
        &[ctx.int(10), ctx.float(0.5), ctx.bool(true)],
    );
    assert_eq!(result.as_float().unwrap(), 10.5);
}

#[test]
fn test_mixed_types_with_false_flag() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    // i=10, f=0.5, b=false => -(10 + 0.5) = -10.5
    let result = ctx.call_ok(
        DeclMixedTypes::new(ctx.type_mgr),
        &[ctx.int(10), ctx.float(0.5), ctx.bool(false)],
    );
    assert_eq!(result.as_float().unwrap(), -10.5);
}

// 5. BOOL RETURN TYPE
// Tests Bridge for bool.

#[test]
fn test_returns_bool_true() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    assert_eq!(
        ctx.call_ok(DeclReturnsBool::new(ctx.type_mgr), &[ctx.int(5)])
            .as_bool()
            .unwrap(),
        true
    );
}

#[test]
fn test_returns_bool_false() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    assert_eq!(
        ctx.call_ok(DeclReturnsBool::new(ctx.type_mgr), &[ctx.int(-5)])
            .as_bool()
            .unwrap(),
        false
    );
}

#[test]
fn test_returns_bool_zero_edge_case() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    // 0 > 0 is false (boundary test)
    assert_eq!(
        ctx.call_ok(DeclReturnsBool::new(ctx.type_mgr), &[ctx.int(0)])
            .as_bool()
            .unwrap(),
        false
    );
}

// 6. FLOAT PARAMETER AND RETURN
// Tests float representation quirks (NaN, Inf, -0.0).

#[test]
fn test_float_zero() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    assert_eq!(
        ctx.call_ok(DeclTakesFloat::new(ctx.type_mgr), &[ctx.float(0.0)])
            .as_float()
            .unwrap(),
        0.0
    );
}

#[test]
fn test_float_negative_zero() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    let result = ctx
        .call_ok(DeclTakesFloat::new(ctx.type_mgr), &[ctx.float(-0.0)])
        .as_float()
        .unwrap();
    // -0.0 * 2.0 = -0.0 (IEEE 754 preserves sign of zero), but -0.0 == 0.0 in Rust
    assert!(result.is_sign_negative() || result == 0.0);
}

#[test]
fn test_float_infinity() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    assert_eq!(
        ctx.call_ok(
            DeclTakesFloat::new(ctx.type_mgr),
            &[ctx.float(f64::INFINITY)]
        )
        .as_float()
        .unwrap(),
        f64::INFINITY
    );
}

#[test]
fn test_float_neg_infinity() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    assert_eq!(
        ctx.call_ok(
            DeclTakesFloat::new(ctx.type_mgr),
            &[ctx.float(f64::NEG_INFINITY)]
        )
        .as_float()
        .unwrap(),
        f64::NEG_INFINITY
    );
}

#[test]
fn test_float_nan() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    // NaN * 2.0 = NaN
    assert!(
        ctx.call_ok(DeclTakesFloat::new(ctx.type_mgr), &[ctx.float(f64::NAN)])
            .as_float()
            .unwrap()
            .is_nan()
    );
}

// 7. STRING EDGE CASES
// Tests empty strings, unicode, special characters.

#[test]
fn test_string_empty() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    assert_eq!(
        &*ctx
            .call_ok(DeclUpper::new(ctx.type_mgr), &[ctx.str("")])
            .as_str()
            .unwrap(),
        ""
    );
}

#[test]
fn test_string_single_char() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    assert_eq!(
        &*ctx
            .call_ok(DeclUpper::new(ctx.type_mgr), &[ctx.str("a")])
            .as_str()
            .unwrap(),
        "A"
    );
}

#[test]
fn test_string_unicode() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    // to_ascii_uppercase only affects ASCII
    assert_eq!(
        &*ctx
            .call_ok(DeclUpper::new(ctx.type_mgr), &[ctx.str("hello world")])
            .as_str()
            .unwrap(),
        "HELLO WORLD"
    );
}

// 8. COMPLEX GENERIC TYPES (Array<T>)

#[test]
fn test_takes_array_empty() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    assert_eq!(
        ctx.call_ok(DeclTakesArray::new(ctx.type_mgr), &[ctx.int_array(&[])])
            .as_int()
            .unwrap(),
        0
    );
}

#[test]
fn test_takes_array_single_element() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    assert_eq!(
        ctx.call_ok(DeclTakesArray::new(ctx.type_mgr), &[ctx.int_array(&[42])])
            .as_int()
            .unwrap(),
        42
    );
}

#[test]
fn test_takes_array_multiple_elements() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    assert_eq!(
        ctx.call_ok(
            DeclTakesArray::new(ctx.type_mgr),
            &[ctx.int_array(&[1, 2, 3, 4, 5])]
        )
        .as_int()
        .unwrap(),
        15
    );
}

#[test]
fn test_returns_array() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);

    let result = ctx.call_ok(DeclReturnsArray::new(ctx.type_mgr), &[ctx.int(5)]);
    let arr = result.as_array().unwrap();

    // Function returns [x, x*2, x*3] = [5, 10, 15]
    assert_eq!(arr.len(), 3);
    assert_eq!(arr.get(0).unwrap().as_int().unwrap(), 5);
    assert_eq!(arr.get(1).unwrap().as_int().unwrap(), 10);
    assert_eq!(arr.get(2).unwrap().as_int().unwrap(), 15);
}

// 9. OPTIONAL TYPE
// Tests Optional<T> which uses null pointer for None.

#[test]
fn test_takes_optional_some() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    assert_eq!(
        ctx.call_ok(
            DeclTakesOptional::new(ctx.type_mgr),
            &[ctx.optional_int(Some(42))]
        )
        .as_int()
        .unwrap(),
        42
    );
}

#[test]
fn test_takes_optional_none() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    // None => unwrap_or(0) => 0
    assert_eq!(
        ctx.call_ok(
            DeclTakesOptional::new(ctx.type_mgr),
            &[ctx.optional_int(None)]
        )
        .as_int()
        .unwrap(),
        0
    );
}

#[test]
fn test_returns_optional_some() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    // x > 0 returns Some(x)
    let result = ctx.call_ok(DeclReturnsOptional::new(ctx.type_mgr), &[ctx.int(42)]);

    assert_eq!(extract_optional_int(&result), Some(42));
}

#[test]
fn test_returns_optional_none() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    // x <= 0 returns None
    let result = ctx.call_ok(DeclReturnsOptional::new(ctx.type_mgr), &[ctx.int(-5)]);
    assert_eq!(extract_optional_int(&result), None);
}

// 10. RESULT WITH LIFETIME IN OK TYPE
// Tests Result<Str<'a>, E> with lifetimes in the Ok type.

#[test]
fn test_result_with_lifetime_success() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    assert_eq!(
        &*ctx
            .call_ok(
                DeclResultWithLifetime::new(ctx.type_mgr),
                &[ctx.str("hello")]
            )
            .as_str()
            .unwrap(),
        "HELLO"
    );
}

#[test]
fn test_result_with_lifetime_error() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    // Empty string triggers error
    let err = ctx
        .call(DeclResultWithLifetime::new(ctx.type_mgr), &[ctx.str("")])
        .unwrap_err();
    assert!(matches!(
        err.kind,
        ExecutionErrorKind::Runtime(RuntimeError::CastError { .. })
    ));
}

// 11. PURE MODE WITH RESULT RETURNING NON-INT

#[test]
fn test_pure_result_returns_float_success() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    assert_eq!(
        ctx.call_ok(
            DeclPureResultComplex::new(ctx.type_mgr),
            &[ctx.int(10), ctx.int(4)]
        )
        .as_float()
        .unwrap(),
        2.5
    );
}

#[test]
fn test_pure_result_returns_float_error() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    let err = ctx
        .call(
            DeclPureResultComplex::new(ctx.type_mgr),
            &[ctx.int(10), ctx.int(0)],
        )
        .unwrap_err();
    assert!(matches!(
        err.kind,
        ExecutionErrorKind::Runtime(RuntimeError::DivisionByZero {})
    ));
}

// 12. FUNCTION TYPE CORRECTNESS

#[test]
fn test_zero_args_function_type() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    let fn_ty_str = format!("{}", DeclZeroArgs::new(ctx.type_mgr).ty());
    assert!(
        fn_ty_str.contains("Int"),
        "Zero-arg function type should contain Int: {}",
        fn_ty_str
    );
}

#[test]
fn test_many_args_function_type() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    let fn_ty_str = format!("{}", DeclManyArgs::new(ctx.type_mgr).ty());
    assert!(
        fn_ty_str.contains("Int"),
        "Many-arg function type should contain Int: {}",
        fn_ty_str
    );
}

#[test]
fn test_mixed_types_function_type() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    let fn_ty_str = format!("{}", DeclMixedTypes::new(ctx.type_mgr).ty());
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

// 13. ANNOTATED FUNCTION NAME

#[test]
fn test_annotated_name_matches_struct() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);

    assert_eq!(DeclZeroArgs::new(ctx.type_mgr).name(), "DeclZeroArgs");
    assert_eq!(DeclManyArgs::new(ctx.type_mgr).name(), "DeclManyArgs");
    assert_eq!(DeclTakesArray::new(ctx.type_mgr).name(), "DeclTakesArray");
}

// 14. NESTED ARRAY TYPE (Array<Str<'a>>)

#[test]
fn test_takes_str_array_empty() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    assert_eq!(
        ctx.call_ok(
            DeclTakesStrArray::new(ctx.type_mgr),
            &[ctx.str_array(vec![])]
        )
        .as_int()
        .unwrap(),
        0
    );
}

#[test]
fn test_takes_str_array_with_elements() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    assert_eq!(
        ctx.call_ok(
            DeclTakesStrArray::new(ctx.type_mgr),
            &[ctx.str_array(vec!["hello", "world", "test"])]
        )
        .as_int()
        .unwrap(),
        3
    );
}

// 15. PURE MODE WITH NESTED GENERICS AND RESULT

#[test]
fn test_array_first_success() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    let result = ctx.call_ok(
        DeclArrayFirst::new(ctx.type_mgr),
        &[ctx.str_array(vec!["first", "second", "third"])],
    );
    assert_eq!(&*result.as_str().unwrap(), "first");
}

#[test]
fn test_array_first_empty_error() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    let err = ctx
        .call(DeclArrayFirst::new(ctx.type_mgr), &[ctx.str_array(vec![])])
        .unwrap_err();
    assert!(matches!(
        err.kind,
        ExecutionErrorKind::Runtime(RuntimeError::CastError { .. })
    ));
}
