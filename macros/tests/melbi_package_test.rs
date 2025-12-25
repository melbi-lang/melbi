//! Test the #[melbi_package] macro

extern crate alloc;

use bumpalo::Bump;
use melbi_core::{
    types::manager::TypeManager,
    values::{FfiContext, binder::Binder, dynamic::Value},
};
use melbi_macros::{melbi_const, melbi_fn, melbi_package};

// ============================================================================
// Basic package with only functions
// ============================================================================

#[melbi_package]
mod basic_pkg {
    use super::*;

    #[melbi_fn]
    fn add(a: i64, b: i64) -> i64 {
        a + b
    }

    #[melbi_fn(name = Mul)]
    fn pkg_mul(a: i64, b: i64) -> i64 {
        a * b
    }
}

#[test]
fn test_basic_package_builds() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let builder = Value::record_builder(&arena, type_mgr);
    let pkg = basic_pkg::register_basic_pkg_functions(&arena, type_mgr, builder)
        .build()
        .unwrap();
    let record = pkg.as_record().unwrap();

    // Should have both functions
    assert_eq!(record.len(), 2);
}

#[test]
fn test_basic_package_functions_work() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let builder = Value::record_builder(&arena, type_mgr);
    let pkg = basic_pkg::register_basic_pkg_functions(&arena, type_mgr, builder)
        .build()
        .unwrap();
    let record = pkg.as_record().unwrap();

    // Get the Add function
    let add_fn = record.get("Add").unwrap();
    let add_fn = add_fn.as_function().unwrap();

    // Call Add(3, 4) = 7
    let ctx = FfiContext::new(&arena, type_mgr);
    let a = Value::int(type_mgr, 3);
    let b = Value::int(type_mgr, 4);
    let result = unsafe { add_fn.call_unchecked(&ctx, &[a, b]).unwrap() };
    assert_eq!(result.as_int().unwrap(), 7);

    // Get the Mul function
    let mul_fn = record.get("Mul").unwrap();
    let mul_fn = mul_fn.as_function().unwrap();

    // Call Mul(3, 4) = 12
    let a = Value::int(type_mgr, 3);
    let b = Value::int(type_mgr, 4);
    let result = unsafe { mul_fn.call_unchecked(&ctx, &[a, b]).unwrap() };
    assert_eq!(result.as_int().unwrap(), 12);
}

// ============================================================================
// Package with functions and constants
// ============================================================================

#[melbi_package]
mod math_pkg {
    use super::*;

    #[melbi_const(name = PI)]
    fn pi<'a>(_arena: &'a Bump, type_mgr: &'a TypeManager<'a>) -> Value<'a, 'a> {
        Value::float(type_mgr, core::f64::consts::PI)
    }

    #[melbi_const(name = E)]
    fn e<'a>(_arena: &'a Bump, type_mgr: &'a TypeManager<'a>) -> Value<'a, 'a> {
        Value::float(type_mgr, core::f64::consts::E)
    }

    #[melbi_fn(name = Abs)]
    fn math_abs(value: f64) -> f64 {
        value.abs()
    }

    #[melbi_fn(name = Sqrt)]
    fn math_sqrt(value: f64) -> f64 {
        value.sqrt()
    }
}

#[test]
fn test_package_with_constants_builds() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let builder = Value::record_builder(&arena, type_mgr);
    let pkg = math_pkg::register_math_pkg_functions(&arena, type_mgr, builder)
        .build()
        .unwrap();
    let record = pkg.as_record().unwrap();

    // Should have 2 constants + 2 functions = 4 fields
    assert_eq!(record.len(), 4);
}

#[test]
fn test_package_constants_have_correct_values() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let builder = Value::record_builder(&arena, type_mgr);
    let pkg = math_pkg::register_math_pkg_functions(&arena, type_mgr, builder)
        .build()
        .unwrap();
    let record = pkg.as_record().unwrap();

    // Check PI
    let pi = record.get("PI").unwrap();
    assert!((pi.as_float().unwrap() - core::f64::consts::PI).abs() < 1e-10);

    // Check E
    let e = record.get("E").unwrap();
    assert!((e.as_float().unwrap() - core::f64::consts::E).abs() < 1e-10);
}

#[test]
fn test_package_with_constants_functions_work() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let builder = Value::record_builder(&arena, type_mgr);
    let pkg = math_pkg::register_math_pkg_functions(&arena, type_mgr, builder)
        .build()
        .unwrap();
    let record = pkg.as_record().unwrap();

    // Get and call Abs
    let abs_fn = record.get("Abs").unwrap();
    let abs_fn = abs_fn.as_function().unwrap();

    let ctx = FfiContext::new(&arena, type_mgr);
    let value = Value::float(type_mgr, -3.5);
    let result = unsafe { abs_fn.call_unchecked(&ctx, &[value]).unwrap() };
    assert_eq!(result.as_float().unwrap(), 3.5);

    // Get and call Sqrt
    let sqrt_fn = record.get("Sqrt").unwrap();
    let sqrt_fn = sqrt_fn.as_function().unwrap();

    let value = Value::float(type_mgr, 16.0);
    let result = unsafe { sqrt_fn.call_unchecked(&ctx, &[value]).unwrap() };
    assert_eq!(result.as_float().unwrap(), 4.0);
}

// ============================================================================
// Custom package name
// ============================================================================

#[melbi_package(name = Custom)]
mod custom_pkg {
    use super::*;

    #[melbi_fn(name = Double)]
    fn double_it(x: i64) -> i64 {
        x * 2
    }
}

#[test]
fn test_custom_package_name() {
    use melbi_core::api::EnvironmentBuilder;

    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    // register_custom_pkg_package should bind the package as "Custom"
    let env = custom_pkg::register_custom_pkg_package(&arena, type_mgr, EnvironmentBuilder::new(&arena));
    let env = env.build().unwrap();

    // Should have one entry named "Custom"
    assert_eq!(env.len(), 1);
    assert_eq!(env[0].0, "Custom");

    // The value should be a record with "Double" function
    let pkg = env[0].1;
    let record = pkg.as_record().unwrap();
    assert_eq!(record.len(), 1);

    // Verify function works
    let double_fn = record.get("Double").unwrap();
    let double_fn = double_fn.as_function().unwrap();

    let ctx = FfiContext::new(&arena, type_mgr);
    let value = Value::int(type_mgr, 21);
    let result = unsafe { double_fn.call_unchecked(&ctx, &[value]).unwrap() };
    assert_eq!(result.as_int().unwrap(), 42);
}

// ============================================================================
// Module visibility preservation
// ============================================================================

#[melbi_package]
pub mod public_pkg {
    use super::*;

    #[melbi_fn(name = Inc)]
    fn increment(x: i64) -> i64 {
        x + 1
    }
}

#[test]
fn test_public_module_stays_public() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    // The module should still be public (this test compiles = visibility preserved)
    let builder = Value::record_builder(&arena, type_mgr);
    let pkg = public_pkg::register_public_pkg_functions(&arena, type_mgr, builder)
        .build()
        .unwrap();
    let record = pkg.as_record().unwrap();

    assert_eq!(record.len(), 1);
}

// ============================================================================
// Empty package (only constants, no functions)
// ============================================================================

#[melbi_package]
mod constants_only_pkg {
    use super::*;

    #[melbi_const(name = ANSWER)]
    fn answer<'a>(_arena: &'a Bump, type_mgr: &'a TypeManager<'a>) -> Value<'a, 'a> {
        Value::int(type_mgr, 42)
    }

    #[melbi_const(name = GREETING)]
    fn greeting<'a>(arena: &'a Bump, type_mgr: &'a TypeManager<'a>) -> Value<'a, 'a> {
        let str_ty = type_mgr.str();
        Value::str(arena, str_ty, "Hello")
    }
}

#[test]
fn test_constants_only_package() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let builder = Value::record_builder(&arena, type_mgr);
    let pkg = constants_only_pkg::register_constants_only_pkg_functions(&arena, type_mgr, builder)
        .build()
        .unwrap();
    let record = pkg.as_record().unwrap();

    assert_eq!(record.len(), 2);

    // Check ANSWER
    let answer = record.get("ANSWER").unwrap();
    assert_eq!(answer.as_int().unwrap(), 42);

    // Check GREETING
    let greeting = record.get("GREETING").unwrap();
    assert_eq!(greeting.as_str().unwrap(), "Hello");
}

// ============================================================================
// Derived names (no explicit name attribute)
// ============================================================================

#[melbi_package]
mod derived_names_pkg {
    use super::*;

    // Derived constant: fn speed_of_light -> "SPEED_OF_LIGHT"
    #[melbi_const]
    fn speed_of_light<'a>(_arena: &'a Bump, type_mgr: &'a TypeManager<'a>) -> Value<'a, 'a> {
        Value::float(type_mgr, 299_792_458.0)
    }

    // Derived function: fn get_answer -> "GetAnswer"
    #[melbi_fn]
    fn get_answer() -> i64 {
        42
    }
}

#[test]
fn test_derived_names() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let builder = Value::record_builder(&arena, type_mgr);
    let pkg = derived_names_pkg::register_derived_names_pkg_functions(&arena, type_mgr, builder)
        .build()
        .unwrap();
    let record = pkg.as_record().unwrap();

    assert_eq!(record.len(), 2);

    // Check derived constant name (SCREAMING_SNAKE_CASE)
    let speed = record.get("SPEED_OF_LIGHT").unwrap();
    assert_eq!(speed.as_float().unwrap(), 299_792_458.0);

    // Check derived function name (PascalCase)
    let get_answer_fn = record.get("GetAnswer").unwrap();
    let get_answer_fn = get_answer_fn.as_function().unwrap();

    let ctx = FfiContext::new(&arena, type_mgr);
    let result = unsafe { get_answer_fn.call_unchecked(&ctx, &[]).unwrap() };
    assert_eq!(result.as_int().unwrap(), 42);
}

// ============================================================================
// Mix of explicit and derived names
// ============================================================================

#[melbi_package]
mod mixed_names_pkg {
    use super::*;

    // Explicit name
    #[melbi_const(name = PI)]
    fn math_pi<'a>(_arena: &'a Bump, type_mgr: &'a TypeManager<'a>) -> Value<'a, 'a> {
        Value::float(type_mgr, core::f64::consts::PI)
    }

    // Derived name: euler_number -> EULER_NUMBER
    #[melbi_const]
    fn euler_number<'a>(_arena: &'a Bump, type_mgr: &'a TypeManager<'a>) -> Value<'a, 'a> {
        Value::float(type_mgr, core::f64::consts::E)
    }

    // Explicit name
    #[melbi_fn(name = Add)]
    fn do_add(a: i64, b: i64) -> i64 {
        a + b
    }

    // Derived name: multiply_values -> MultiplyValues
    #[melbi_fn]
    fn multiply_values(a: i64, b: i64) -> i64 {
        a * b
    }
}

#[test]
fn test_mixed_explicit_and_derived_names() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let builder = Value::record_builder(&arena, type_mgr);
    let pkg = mixed_names_pkg::register_mixed_names_pkg_functions(&arena, type_mgr, builder)
        .build()
        .unwrap();
    let record = pkg.as_record().unwrap();

    // Should have 4 items: PI, EULER_NUMBER, Add, MultiplyValues
    assert_eq!(record.len(), 4);

    // Check explicit constant
    let pi = record.get("PI").unwrap();
    assert!((pi.as_float().unwrap() - core::f64::consts::PI).abs() < 1e-10);

    // Check derived constant
    let e = record.get("EULER_NUMBER").unwrap();
    assert!((e.as_float().unwrap() - core::f64::consts::E).abs() < 1e-10);

    // Check explicit function
    assert!(record.get("Add").is_some());

    // Check derived function
    assert!(record.get("MultiplyValues").is_some());
}

// ============================================================================
// Empty parentheses variants
// ============================================================================

#[melbi_package]
mod empty_parens_pkg {
    use super::*;

    // #[melbi_const()] with empty parens - same as no parens
    #[melbi_const()]
    fn golden_ratio<'a>(_arena: &'a Bump, type_mgr: &'a TypeManager<'a>) -> Value<'a, 'a> {
        Value::float(type_mgr, 1.618033988749895)
    }

    // #[melbi_fn()] with empty parens - same as no parens
    #[melbi_fn()]
    fn double_value(x: i64) -> i64 {
        x * 2
    }
}

#[test]
fn test_empty_parentheses_variants() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let builder = Value::record_builder(&arena, type_mgr);
    let pkg = empty_parens_pkg::register_empty_parens_pkg_functions(&arena, type_mgr, builder)
        .build()
        .unwrap();
    let record = pkg.as_record().unwrap();

    assert_eq!(record.len(), 2);

    // Check derived constant name from empty parens: golden_ratio -> GOLDEN_RATIO
    let golden = record.get("GOLDEN_RATIO").unwrap();
    assert!((golden.as_float().unwrap() - 1.618033988749895).abs() < 1e-10);

    // Check derived function name from empty parens: double_value -> DoubleValue
    let double_fn = record.get("DoubleValue").unwrap();
    let double_fn = double_fn.as_function().unwrap();

    let ctx = FfiContext::new(&arena, type_mgr);
    let result = unsafe {
        double_fn
            .call_unchecked(&ctx, &[Value::int(type_mgr, 21)])
            .unwrap()
    };
    assert_eq!(result.as_int().unwrap(), 42);
}
