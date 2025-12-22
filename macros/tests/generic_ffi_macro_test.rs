//! Tests for generic FFI function support in `#[melbi_fn]`.
//!
//! # Function Signatures to Support
//!
//! ## Phase 1: Single type parameter, bare usage only [x] DONE
//!
//! - `fn square<T: Numeric>(x: T) -> T`
//! - `fn add<T: Numeric>(a: T, b: T) -> T`
//! - `fn is_positive<T: Numeric>(x: T) -> bool` (generic input, concrete output)
//!
//! ## Phase 2: Single type parameter in containers [ ] TODO(generic-ffi-phase2)
//!
//! - `fn sum<T: Numeric>(arr: Array<T>) -> T`
//! - `fn first_numeric<T: Numeric>(arr: Array<T>) -> Optional<T>`
//! - `fn scale<T: Numeric>(arr: Array<T>, factor: T) -> Array<T>` (mixed bare + container)
//! - `fn get_value<T: Numeric>(m: Map<Str, T>, key: Str) -> Optional<T>`
//!
//! ## Phase 3: Multiple type parameters (same trait) [ ] TODO(generic-ffi-phase3)
//!
//! - `fn convert<T: Numeric, U: Numeric>(x: T) -> U` (cross-product: 2x2 = 4 cases)
//! - `fn zip_numeric<T: Numeric, U: Numeric>(a: Array<T>, b: Array<U>) -> Array<Record<{a: T, b: U}>>`
//!
//! ## Phase 4: Structural expansion (unconstrained / Indexable) [ ] TODO(generic-ffi-phase4)
//!
//! - `fn first<T: Melbi>(arr: Array<T>) -> Optional<T>` (T unconstrained, uses Any<0>)
//! - `fn len<T: Indexable>(container: T) -> Int`
//! - `fn get<T: Indexable>(container: T, index: Int) -> Optional<T::Element>`
//!
//! ## Phase 5: Other closed traits [ ] TODO(generic-ffi-phase5)
//!
//! - `fn max<T: Ord>(a: T, b: T) -> T`
//! - `fn sort<T: Ord>(arr: Array<T>) -> Array<T>`
//! - `fn as_key<T: Hashable>(x: T) -> T` (identity, but validates hashability)
//!
//! ## Edge cases to consider
//!
//! - Functions with FfiContext: `fn double<T: Numeric>(ctx: &FfiContext, x: T) -> T`
//! - Fallible functions: `fn safe_div<T: Numeric>(a: T, b: T) -> Result<T, DivByZero>`
//! - Lifetime parameters: `fn square<'a, T: Numeric>(x: T) -> T`
//!
//! ## Compile-fail cases (should produce helpful errors)
//!
//! - `fn bad<T>(x: T) -> T` (no trait bound - should suggest `: Melbi`)
//! - `fn bad<T: Unknown>(x: T) -> T` (unknown trait)
//! - `fn bad<T: Numeric>(x: Array<T>) -> T` (Phase 1: container usage rejected)
//! - `fn bad<T, U>(x: T) -> U` (Phase 1: multiple type params rejected)

extern crate alloc;

use bumpalo::Bump;
use melbi_core::{
    evaluator::{ExecutionError, ExecutionErrorKind, RuntimeError},
    types::manager::TypeManager,
    values::{FfiContext, dynamic::Value, function::Function},
};
use melbi_macros::melbi_fn;

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

// ============================================================================
// Phase 1 Tests: Single type parameter, bare usage
// ============================================================================

use melbi_core::values::{Melbi, Numeric};

#[melbi_fn]
fn square<T: Numeric>(x: T) -> T {
    x * x
}

#[test]
fn test_square_int() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    let square_fn = Square::new(ctx.type_mgr);
    let result = ctx.call_ok(&square_fn, &[ctx.int(5)]);
    assert_eq!(result.as_int().unwrap(), 25);
}

#[test]
fn test_square_float() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    let square_fn = Square::new(ctx.type_mgr);
    let result = ctx.call_ok(&square_fn, &[ctx.float(2.5)]);
    assert_eq!(result.as_float().unwrap(), 6.25);
}

#[test]
fn test_square_negative() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    let square_fn = Square::new(ctx.type_mgr);
    let result = ctx.call_ok(&square_fn, &[ctx.int(-7)]);
    assert_eq!(result.as_int().unwrap(), 49);
}

#[melbi_fn]
fn add_nums<T: Numeric>(a: T, b: T) -> T {
    a + b
}

#[test]
fn test_add_int() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    let add_fn = AddNums::new(ctx.type_mgr);
    let result = ctx.call_ok(&add_fn, &[ctx.int(3), ctx.int(4)]);
    assert_eq!(result.as_int().unwrap(), 7);
}

#[test]
fn test_add_float() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    let add_fn = AddNums::new(ctx.type_mgr);
    let result = ctx.call_ok(&add_fn, &[ctx.float(1.5), ctx.float(2.5)]);
    assert_eq!(result.as_float().unwrap(), 4.0);
}

#[test]
fn test_square_type_mismatch() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    let square_fn = Square::new(ctx.type_mgr);

    // Bool is not Numeric
    let bool_val = Value::bool(ctx.type_mgr, true);
    let err = ctx.call(&square_fn, &[bool_val]).unwrap_err();
    assert!(
        matches!(
            err.kind,
            ExecutionErrorKind::Runtime(RuntimeError::CastError { .. })
        ),
        "Expected CastError, got {:?}",
        err
    );
}

// TODO: uncomment after supporting full parametric polymorphism.
// Test with Melbi bound (base trait)
// #[melbi_fn]
// fn identity<T: Melbi>(x: T) -> T {
//     x
// }

// #[test]
// fn test_identity_int() {
//     let arena = Bump::new();
//     let ctx = TestCtx::new(&arena);
//     let identity_fn = Identity::new(ctx.type_mgr);
//     let result = ctx.call_ok(&identity_fn, &[ctx.int(42)]);
//     assert_eq!(result.as_int().unwrap(), 42);
// }

// #[test]
// fn test_identity_float() {
//     let arena = Bump::new();
//     let ctx = TestCtx::new(&arena);
//     let identity_fn = Identity::new(ctx.type_mgr);
//     let result = ctx.call_ok(&identity_fn, &[ctx.float(3.14)]);
//     assert_eq!(result.as_float().unwrap(), 3.14);
// }

// Test with compound bounds (Melbi + Numeric uses Numeric for dispatch)
#[melbi_fn]
fn double<T: Melbi + Numeric>(x: T) -> T {
    x + x
}

#[test]
fn test_double_int() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    let double_fn = Double::new(ctx.type_mgr);
    let result = ctx.call_ok(&double_fn, &[ctx.int(21)]);
    assert_eq!(result.as_int().unwrap(), 42);
}

#[test]
fn test_double_float() {
    let arena = Bump::new();
    let ctx = TestCtx::new(&arena);
    let double_fn = Double::new(ctx.type_mgr);
    let result = ctx.call_ok(&double_fn, &[ctx.float(1.5)]);
    assert_eq!(result.as_float().unwrap(), 3.0);
}
