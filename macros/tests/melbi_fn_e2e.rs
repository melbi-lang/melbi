use bumpalo::Bump;
use melbi_core::{
    evaluator::{ExecutionError, ExecutionErrorKind, RuntimeError},
    types::manager::TypeManager,
    values::{
        FfiContext,
        dynamic::Value,
        function::{AnnotatedFunction, Function},
        typed::{Array, Optional, RawConvertible, Str},
    },
};
use melbi_macros::melbi_fn;

// ============================================================================
// Test functions (what the user would write)
// ============================================================================

#[melbi_fn(name = DeclAdd)]
fn add_impl(_ctx: &FfiContext, a: i64, b: i64) -> i64 {
    a + b
}

#[melbi_fn(name = DeclSafeDiv)]
fn safe_div_impl(_ctx: &FfiContext, a: i64, b: i64) -> Result<i64, RuntimeError> {
    if b == 0 {
        Err(RuntimeError::DivisionByZero {})
    } else {
        Ok(a / b)
    }
}

/// NoContext mode: no context
#[melbi_fn(name = DeclNoContextAdd)]
fn no_context_add_impl(a: i64, b: i64) -> i64 {
    a + b
}

/// NoContext mode with Result
#[melbi_fn(name = DeclNoContextCheckedAdd)]
fn no_context_checked_add_impl(a: i64, b: i64) -> Result<i64, RuntimeError> {
    a.checked_add(b).ok_or(RuntimeError::IntegerOverflow {})
}

/// Legacy mode with lifetimes (string handling)
#[melbi_fn(name = DeclUpper)]
fn string_upper_impl<'a>(ctx: &FfiContext<'a, 'a>, s: Str<'a>) -> Str<'a> {
    let upper = s.to_ascii_uppercase();
    Str::from_str(ctx.arena(), &upper)
}

// ============================================================================
// Test Helpers
// ============================================================================

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
fn test_no_context_mode_plain() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);

    let add_fn = DeclNoContextAdd::new(ctx.type_mgr);
    assert_eq!(add_fn.name(), "DeclNoContextAdd");

    let result = ctx.call_ok(add_fn, &[ctx.int(10), ctx.int(32)]);
    assert_eq!(result.as_int().unwrap(), 42);
}

#[test]
fn test_no_context_mode_result_success() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);

    let result = ctx.call_ok(
        DeclNoContextCheckedAdd::new(ctx.type_mgr),
        &[ctx.int(1), ctx.int(2)],
    );
    assert_eq!(result.as_int().unwrap(), 3);
}

#[test]
fn test_no_context_mode_result_error() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);

    let err = ctx
        .call(
            DeclNoContextCheckedAdd::new(ctx.type_mgr),
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

/// Helper to extract Optional<i64> from a Value
fn extract_optional_int(v: &Value) -> Option<i64> {
    v.as_option().unwrap().map(|inner| inner.as_int().unwrap())
}

// ----------------------------------------------------------------------------
// Test Functions for Edge Cases
// ----------------------------------------------------------------------------

/// Zero-argument function (Legacy mode) - tests empty parameter list handling
#[melbi_fn(name = DeclZeroArgs)]
fn zero_args_impl(_ctx: &FfiContext) -> i64 {
    42
}

/// Zero-argument function (NoContext mode)
#[melbi_fn(name = DeclNoContextZeroArgs)]
fn no_context_zero_args_impl() -> i64 {
    42
}

/// Zero-argument function returning Result
#[melbi_fn(name = DeclZeroArgsResult)]
fn zero_args_result_impl(_ctx: &FfiContext) -> Result<i64, RuntimeError> {
    Ok(42)
}

/// Many-argument function (5 parameters) - tests macro repetition patterns
#[melbi_fn(name = DeclManyArgs)]
fn many_args_impl(_ctx: &FfiContext, a: i64, b: i64, c: i64, d: i64, e: i64) -> i64 {
    a + b + c + d + e
}

/// Single-argument function - minimal case
#[melbi_fn(name = DeclSingleArg)]
fn single_arg_impl(_ctx: &FfiContext, x: i64) -> i64 {
    x * 2
}

/// Function returning bool - tests non-numeric Bridge type
#[melbi_fn(name = DeclReturnsBool)]
fn returns_bool_impl(_ctx: &FfiContext, x: i64) -> bool {
    x > 0
}

/// Function returning f64 - tests floating point Bridge type
#[melbi_fn(name = DeclReturnsFloat)]
fn returns_float_impl(_ctx: &FfiContext, x: i64) -> f64 {
    x as f64 * 1.5
}

/// Function taking bool parameter
#[melbi_fn(name = DeclTakesBool)]
fn takes_bool_impl(_ctx: &FfiContext, flag: bool) -> i64 {
    if flag { 1 } else { 0 }
}

/// Function taking f64 parameter
#[melbi_fn(name = DeclTakesFloat)]
fn takes_float_impl(_ctx: &FfiContext, x: f64) -> f64 {
    x * 2.0
}

/// Function taking mixed parameter types
#[melbi_fn(name = DeclMixedTypes)]
fn mixed_types_impl(_ctx: &FfiContext, i: i64, f: f64, b: bool) -> f64 {
    let base = i as f64 + f;
    if b { base } else { -base }
}

/// Function with Array parameter - tests complex generic types
#[melbi_fn(name = DeclTakesArray)]
fn takes_array_impl<'a>(ctx: &FfiContext<'a, 'a>, arr: Array<'a, i64>) -> i64 {
    let _ = ctx;
    arr.iter().sum()
}

/// Function returning Array - tests complex generic return type
#[melbi_fn(name = DeclReturnsArray)]
fn returns_array_impl<'a>(ctx: &FfiContext<'a, 'a>, x: i64) -> Array<'a, i64> {
    Array::new(ctx.arena(), &[x, x * 2, x * 3])
}

/// Function with nested generic type: Array<Str<'a>>
#[melbi_fn(name = DeclTakesStrArray)]
fn takes_str_array_impl<'a>(ctx: &FfiContext<'a, 'a>, arr: Array<'a, Str<'a>>) -> i64 {
    let _ = ctx;
    arr.len() as i64
}

/// Function with Optional parameter
#[melbi_fn(name = DeclTakesOptional)]
fn takes_optional_impl<'a>(ctx: &FfiContext<'a, 'a>, opt: Optional<'a, i64>) -> i64 {
    let _ = ctx;
    opt.as_option().unwrap_or(0)
}

/// Function returning Optional
#[melbi_fn(name = DeclReturnsOptional)]
fn returns_optional_impl<'a>(ctx: &FfiContext<'a, 'a>, x: i64) -> Optional<'a, i64> {
    if x > 0 {
        Optional::some(ctx.arena(), x)
    } else {
        Optional::none()
    }
}

/// Function with Result<Str<'a>, E> - Result with lifetime in ok type
#[melbi_fn(name = DeclResultWithLifetime)]
fn result_with_lifetime_impl<'a>(
    ctx: &FfiContext<'a, 'a>,
    s: Str<'a>,
) -> Result<Str<'a>, RuntimeError> {
    if s.is_empty() {
        Err(RuntimeError::CastError {
            message: "Empty string not allowed".into(),
        })
    } else {
        Ok(Str::from_str(ctx.arena(), &s.to_ascii_uppercase()))
    }
}

/// NoContext function with Result returning complex type
#[melbi_fn(name = DeclNoContextResultComplex)]
fn no_context_result_complex_impl(a: i64, b: i64) -> Result<f64, RuntimeError> {
    if b == 0 {
        Err(RuntimeError::DivisionByZero {})
    } else {
        Ok(a as f64 / b as f64)
    }
}

/// NoContext function with nested generics and Result<Str<'value>, E>
#[melbi_fn(name = DeclArrayFirst)]
fn array_first<'value>(
    arr: Array<'value, Str<'value>>,
) -> Result<Str<'value>, melbi_core::evaluator::ExecutionErrorKind> {
    arr.iter().next().ok_or_else(|| {
        melbi_core::evaluator::ExecutionErrorKind::Runtime(RuntimeError::CastError {
            message: "Array is empty".into(),
        })
    })
}

// NOTE: Only a single lifetime is supported (e.g., 'a).
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
fn test_zero_args_no_context_mode() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    assert_eq!(
        ctx.call_ok(DeclNoContextZeroArgs::new(ctx.type_mgr), &[])
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
            .call_ok(DeclUpper::new(ctx.type_mgr), &[ctx.str("Γεια σας κόσμε")])
            .as_str()
            .unwrap(),
        "Γεια σας κόσμε"
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

// 11. NO-CONTEXT MODE WITH RESULT RETURNING NON-INT

#[test]
fn test_no_context_result_returns_float_success() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    assert_eq!(
        ctx.call_ok(
            DeclNoContextResultComplex::new(ctx.type_mgr),
            &[ctx.int(10), ctx.int(4)]
        )
        .as_float()
        .unwrap(),
        2.5
    );
}

#[test]
fn test_no_context_result_returns_float_error() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    let err = ctx
        .call(
            DeclNoContextResultComplex::new(ctx.type_mgr),
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

// ============================================================================
// 15. DERIVED NAMES (no explicit name attribute)
// ============================================================================

/// Test #[melbi_fn] without explicit name - derives PascalCase from function name
#[melbi_fn]
fn derived_add(a: i64, b: i64) -> i64 {
    a + b
}

/// Test #[melbi_fn()] with empty parentheses - same as no parentheses
#[melbi_fn()]
fn empty_parens_mul(a: i64, b: i64) -> i64 {
    a * b
}

/// Test derived name with underscores: get_first_item -> GetFirstItem
#[melbi_fn]
fn get_first_item(a: i64, _b: i64) -> i64 {
    a
}

#[test]
fn test_derived_name_no_parens() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);

    // Should generate struct named DerivedAdd
    let func = DerivedAdd::new(ctx.type_mgr);
    assert_eq!(func.name(), "DerivedAdd");

    let result = ctx.call_ok(func, &[ctx.int(3), ctx.int(4)]);
    assert_eq!(result.as_int().unwrap(), 7);
}

#[test]
fn test_derived_name_empty_parens() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);

    // Should generate struct named EmptyParensMul
    let func = EmptyParensMul::new(ctx.type_mgr);
    assert_eq!(func.name(), "EmptyParensMul");

    let result = ctx.call_ok(func, &[ctx.int(3), ctx.int(4)]);
    assert_eq!(result.as_int().unwrap(), 12);
}

#[test]
fn test_derived_name_with_underscores() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);

    // Should generate struct named GetFirstItem (underscores removed, each word capitalized)
    let func = GetFirstItem::new(ctx.type_mgr);
    assert_eq!(func.name(), "GetFirstItem");

    let result = ctx.call_ok(func, &[ctx.int(42), ctx.int(0)]);
    assert_eq!(result.as_int().unwrap(), 42);
}
