//! Proof of concept for generic FFI functions with runtime dispatch.
//!
//! This demonstrates the "Internal Dispatch" strategy from ffi_generics.md:
//!
//! 1. **Closed Expansion** (Numeric): Fully specialized to concrete types (i64, f64)
//! 2. **Structural Expansion** (Indexable): Uses `Any<N>` for infinite type families
//!
//! Key insight: A SINGLE wrapper struct handles ALL instantiations via runtime dispatch.

extern crate alloc;

use bumpalo::Bump;
use core::marker::PhantomData;
use melbi_core::{
    evaluator::{ExecutionError, ExecutionErrorKind, RuntimeError},
    parser::Span,
    types::{Type, manager::TypeManager},
    values::{
        FfiContext,
        dynamic::Value,
        function::{AnnotatedFunction, Function},
        raw::RawValue,
        typed::{Array, Bridge, Optional, RawConvertible},
    },
};

// ============================================================================
// Helper for type mismatch errors
// ============================================================================

fn type_mismatch_error(expected: &str, actual: &str) -> ExecutionError {
    ExecutionError {
        kind: ExecutionErrorKind::Runtime(RuntimeError::CastError {
            message: format!("expected {}, got {}", expected, actual),
        }),
        source: String::new(),
        span: Span::new(0, 0),
    }
}

// ============================================================================
// PART 1: Closed Expansion for Numeric trait
// ============================================================================
//
// The Numeric trait has a CLOSED set of types: Int (i64) and Float (f64).
// We generate exhaustive match arms for each - no Any<N> needed.

/// Numeric trait - implemented by i64 and f64.
/// This would typically be defined in melbi_core.
trait Numeric:
    Copy
    + Sized
    + core::ops::Mul<Output = Self>
    + core::ops::Add<Output = Self>
    + core::ops::Sub<Output = Self>
    + core::ops::Div<Output = Self>
{
}

impl Numeric for i64 {}

impl Numeric for f64 {}

/// User-written function: square a numeric value.
///
/// This is what the user actually writes - a GENERIC function.
/// The macro generates dispatch code that calls this with concrete types.
///
/// ```ignore
/// #[melbi_fn]
/// fn square<T: Numeric>(x: T) -> T { x * x }
/// ```
fn square<T: Numeric>(x: T) -> T {
    x * x
}

/// Generated wrapper for `square<T: Numeric>`.
///
/// Unlike the compile-time generic approach, this wrapper:
/// - Is NOT generic over T
/// - Contains a match statement that dispatches based on runtime type
/// - Handles ALL Numeric types (Int, Float) in a single struct
pub struct Square<'a> {
    ty: &'a Type<'a>,
}

impl<'a> Square<'a> {
    pub fn new(type_mgr: &'a TypeManager<'a>) -> Self {
        // Currently we have no way to specify that `number` must implement the
        // `Numeric` type-class, but we'd specify that in the type here. That
        // constraint would have been automatically transferred here by the
        // #[melbi_fn] macro.
        let number = type_mgr.fresh_type_var();
        Self {
            ty: type_mgr.function(&[number], number),
        }
    }
}

impl<'a> Function<'a, 'a> for Square<'a> {
    fn ty(&self) -> &'a Type<'a> {
        self.ty
    }

    unsafe fn call_unchecked(
        &self,
        ctx: &FfiContext<'a, 'a>,
        args: &[Value<'a, 'a>],
    ) -> Result<Value<'a, 'a>, ExecutionError> {
        // RUNTIME DISPATCH based on the actual argument type
        match args[0].ty {
            Type::Int => {
                // Dispatch to square::<i64>
                let x = unsafe { i64::from_raw_value(args[0].raw()) };
                let result = square::<i64>(x);
                let raw = i64::to_raw_value(ctx.arena(), result);
                let ty = ctx.type_mgr().int();
                Ok(Value::from_raw_unchecked(ty, raw))
            }
            Type::Float => {
                // Dispatch to square::<f64>
                let x = unsafe { f64::from_raw_value(args[0].raw()) };
                let result = square::<f64>(x);
                Ok(Value::float(ctx.type_mgr(), result))
            }
            _ => {
                // Type error: not a Numeric type
                Err(type_mismatch_error(
                    "Numeric (Int or Float)",
                    &format!("{}", args[0].ty),
                ))
            }
        }
    }
}

impl<'a> AnnotatedFunction<'a> for Square<'a> {
    fn name(&self) -> &'static str {
        "Square"
    }

    fn location(&self) -> (&'static str, &'static str, &'static str, u32, u32) {
        (
            env!("CARGO_CRATE_NAME"),
            env!("CARGO_PKG_VERSION"),
            file!(),
            line!(),
            column!(),
        )
    }

    fn doc(&self) -> Option<&str> {
        Some("Square a numeric value (works with Int or Float).")
    }
}

// ============================================================================
// PART 2: Any<N> for Structural Expansion
// ============================================================================
//
// For traits like Indexable, we can't enumerate all types (Array<Int>,
// Array<Array<Int>>, etc.). Instead, we use Any<N> as a type-erased wrapper.

/// A type-erased value for parametric polymorphism.
///
/// The const `N` ensures different type parameters don't get mixed up:
/// - T maps to Any<0>
/// - U maps to Any<1>
/// - etc.
///
/// For parametric polymorphism (unconstrained generics), `Any<N>` just carries
/// raw values around. No type info is needed during function execution - the
/// dispatch code tracks types and reconstructs them when converting back to Value.
#[derive(Clone, Copy)]
pub struct Any<'a, const N: usize> {
    raw: RawValue,
    _phantom: PhantomData<&'a ()>,
}

impl<'a, const N: usize> Any<'a, N> {
    pub fn raw(self) -> RawValue {
        self.raw
    }
}

// RawConvertible impl for Any<N>
impl<'a, const N: usize> RawConvertible<'a> for Any<'a, N> {
    fn to_raw_value(_arena: &'a Bump, value: Self) -> RawValue {
        value.raw
    }

    unsafe fn from_raw_value(raw: RawValue) -> Self {
        // For parametric polymorphism, we just wrap the raw value.
        // No type info needed - dispatch handles type reconstruction.
        Self {
            raw,
            _phantom: PhantomData,
        }
    }
}

// Bridge impl for Any<N> - needed for Array<Any<N>>, Optional<Any<N>>, etc.
impl<'a, const N: usize> Bridge<'a> for Any<'a, N> {
    type Raw = RawValue;

    fn type_from(_type_mgr: &'a TypeManager<'a>) -> &'a Type<'a> {
        // This is never called in practice for parametric polymorphism.
        // The dispatch code provides the actual type when converting back to Value.
        panic!("Any<N>::type_from should not be called - dispatch handles types")
    }
}

// ============================================================================
// PART 3: Parametric Polymorphism for first_element
// ============================================================================
//
// For `first_element<T>(array: Array<T>) -> Optional<T>`, T is unconstrained.
// This is parametric polymorphism - the function behaves identically for all T.
//
// The macro calls it with T = Any<0>. Since Any<0> just wraps raw values,
// and the function doesn't need to know what T is, this just works.

/// User-written function: get the first element of an array.
///
/// This is EXACTLY what the user writes. Copy is needed for array.get().
/// The macro will call it with T = Any<0>.
fn first_element<'a, T: Bridge<'a> + Copy>(
    ctx: &FfiContext<'a, 'a>,
    array: Array<'a, T>,
) -> Optional<'a, T> {
    match array.get(0) {
        Some(elem) => Optional::some(ctx.arena(), elem),
        None => Optional::none(),
    }
}

/// Generated wrapper for `first_element<T>` using parametric polymorphism.
///
/// This wrapper handles ANY array type. The dispatch:
/// 1. Extracts the element type from the input array
/// 2. Calls first_element::<Any<0>>(array)
/// 3. Reconstructs the proper Optional<ElemTy> type for the result
pub struct FirstElement<'a> {
    ty: &'a Type<'a>,
}

impl<'a> FirstElement<'a> {
    pub fn new(type_mgr: &'a TypeManager<'a>) -> Self {
        // Polymorphic type: Array<T> -> Optional<T>
        let t = type_mgr.fresh_type_var();
        let array_t = type_mgr.array(t);
        let optional_t = type_mgr.option(t);
        Self {
            ty: type_mgr.function(&[array_t], optional_t),
        }
    }
}

impl<'a> Function<'a, 'a> for FirstElement<'a> {
    fn ty(&self) -> &'a Type<'a> {
        self.ty
    }

    unsafe fn call_unchecked(
        &self,
        ctx: &FfiContext<'a, 'a>,
        args: &[Value<'a, 'a>],
    ) -> Result<Value<'a, 'a>, ExecutionError> {
        // Extract element type from input array
        let Type::Array(elem_ty) = args[0].ty else {
            return Err(type_mismatch_error("Array<T>", &format!("{}", args[0].ty)));
        };

        // Extract array as Array<Any<0>> - just reinterprets raw data
        let array: Array<'a, Any<'a, 0>> = unsafe { Array::from_raw_value(args[0].raw()) };

        // Call the user's generic function with T = Any<0>
        let result = first_element::<Any<0>>(ctx, array);

        // Convert result back to Value, reconstructing the proper type
        let opt_ty = ctx.type_mgr().option(elem_ty);
        let raw = Optional::<Any<0>>::to_raw_value(ctx.arena(), result);
        Ok(Value::from_raw_unchecked(opt_ty, raw))
    }
}

impl<'a> AnnotatedFunction<'a> for FirstElement<'a> {
    fn name(&self) -> &'static str {
        "FirstElement"
    }

    fn location(&self) -> (&'static str, &'static str, &'static str, u32, u32) {
        (
            env!("CARGO_CRATE_NAME"),
            env!("CARGO_PKG_VERSION"),
            file!(),
            line!(),
            column!(),
        )
    }

    fn doc(&self) -> Option<&str> {
        Some("Get the first element of any array type.")
    }
}

// ============================================================================
// Test Helpers
// ============================================================================

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

    fn nested_int_array(&self, values: Vec<Vec<i64>>) -> Value<'a, 'a> {
        let inner_arrays: Vec<Array<i64>> = values
            .iter()
            .map(|v| Array::<i64>::new(self.arena, v))
            .collect();
        let outer = Array::from_iter(self.arena, inner_arrays);
        let inner_ty = self.type_mgr.array(self.type_mgr.int());
        let ty = self.type_mgr.array(inner_ty);
        Value::from_raw_unchecked(ty, outer.as_raw_value())
    }

    fn call<F: Function<'a, 'a>>(
        &self,
        f: &F,
        args: &[Value<'a, 'a>],
    ) -> Result<Value<'a, 'a>, ExecutionError> {
        let ctx = FfiContext::new(self.arena, self.type_mgr);
        unsafe { f.call_unchecked(&ctx, args) }
    }

    fn call_ok<F: Function<'a, 'a>>(&self, f: &F, args: &[Value<'a, 'a>]) -> Value<'a, 'a> {
        self.call(f, args).expect("expected successful call")
    }
}

fn extract_optional_int(v: &Value) -> Option<i64> {
    v.as_option().unwrap().map(|inner| inner.as_int().unwrap())
}

fn check_optional_str(v: &Value, expected: Option<&str>) {
    let opt = v.as_option().unwrap();
    match (opt, expected) {
        (None, None) => {}
        (Some(inner), Some(exp)) => {
            assert_eq!(&*inner.as_str().unwrap(), exp);
        }
        (None, Some(exp)) => panic!("Expected Some({:?}), got None", exp),
        (Some(inner), None) => {
            panic!("Expected None, got Some({:?})", &*inner.as_str().unwrap())
        }
    }
}

// ============================================================================
// Tests: Closed Expansion (Numeric)
// ============================================================================

#[test]
fn test_square_int() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);

    let square_fn = Square::new(ctx.type_mgr);

    // Works with Int
    let result = ctx.call_ok(&square_fn, &[ctx.int(5)]);
    assert_eq!(result.as_int().unwrap(), 25);
}

#[test]
fn test_square_float() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);

    let square_fn = Square::new(ctx.type_mgr);

    // Works with Float
    let result = ctx.call_ok(&square_fn, &[ctx.float(2.5)]);
    assert_eq!(result.as_float().unwrap(), 6.25);
}

#[test]
fn test_square_type_mismatch() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);

    let square_fn = Square::new(ctx.type_mgr);

    // Fails with non-Numeric type (array)
    let err = ctx
        .call(&square_fn, &[ctx.int_array(&[1, 2, 3])])
        .unwrap_err();
    assert!(
        matches!(
            err.kind,
            ExecutionErrorKind::Runtime(RuntimeError::CastError { .. })
        ),
        "Expected CastError, got {:?}",
        err
    );
}

#[test]
fn test_square_negative() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);

    let square_fn = Square::new(ctx.type_mgr);

    let result = ctx.call_ok(&square_fn, &[ctx.int(-7)]);
    assert_eq!(result.as_int().unwrap(), 49);
}

#[test]
fn test_square_zero() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);

    let square_fn = Square::new(ctx.type_mgr);

    let result = ctx.call_ok(&square_fn, &[ctx.float(0.0)]);
    assert_eq!(result.as_float().unwrap(), 0.0);
}

// ============================================================================
// Tests: Structural Expansion (FirstElement)
// ============================================================================

#[test]
fn test_first_element_int_array() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);

    // Single wrapper handles ALL array types
    let first_fn = FirstElement::new(ctx.type_mgr);

    let result = ctx.call_ok(&first_fn, &[ctx.int_array(&[42, 1, 2, 3])]);
    assert_eq!(extract_optional_int(&result), Some(42));
}

#[test]
fn test_first_element_str_array() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);

    // SAME wrapper, different element type
    let first_fn = FirstElement::new(ctx.type_mgr);

    let result = ctx.call_ok(&first_fn, &[ctx.str_array(vec!["hello", "world"])]);
    check_optional_str(&result, Some("hello"));
}

#[test]
fn test_first_element_nested_array() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);

    // Works with Array<Array<Int>> too!
    let first_fn = FirstElement::new(ctx.type_mgr);

    let nested = ctx.nested_int_array(vec![vec![1, 2], vec![3, 4, 5]]);
    let result = ctx.call_ok(&first_fn, &[nested]);

    // Result is Optional<Array<Int>>, containing [1, 2]
    let inner = result.as_option().unwrap().unwrap();
    let inner_array = inner.as_array().unwrap();
    assert_eq!(inner_array.len(), 2);
    assert_eq!(inner_array.get(0).unwrap().as_int().unwrap(), 1);
}

#[test]
fn test_first_element_empty_array() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);

    let first_fn = FirstElement::new(ctx.type_mgr);

    let result = ctx.call_ok(&first_fn, &[ctx.int_array(&[])]);
    assert_eq!(extract_optional_int(&result), None);
}

#[test]
fn test_first_element_type_mismatch() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);

    let first_fn = FirstElement::new(ctx.type_mgr);

    // Fails with non-Array type
    let err = ctx.call(&first_fn, &[ctx.int(42)]).unwrap_err();
    assert!(
        matches!(
            err.kind,
            ExecutionErrorKind::Runtime(RuntimeError::CastError { .. })
        ),
        "Expected CastError, got {:?}",
        err
    );
}

// ============================================================================
// Design Notes: Expansion Table
// ============================================================================
//
// The macro uses a hardcoded expansion table for each trait:
//
// | Trait     | Expansion  | Mappings                                    |
// |-----------|------------|---------------------------------------------|
// | Numeric   | Closed     | Int → i64, Float → f64                      |
// | Indexable | Structural | Array<E> → match on Array, elem via Any<0>  |
// | Ord       | Mixed      | Int/Float fast path, fallback to Any<0>     |
//
// For Closed traits:
// - Generate exhaustive match arms
// - Call monomorphized functions directly (square_i64, square_f64)
// - No runtime overhead beyond the match
//
// For Structural traits:
// - Generate match on outer structure (Array, Map, etc.)
// - Use Any<N> for type-erased inner values
// - Any<N> carries runtime type for dynamic operations
//
// For Mixed traits:
// - Fast path for known types (Int, Float)
// - Fallback to Any<N> for everything else
