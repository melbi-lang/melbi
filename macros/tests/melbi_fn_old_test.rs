//! Test the #[melbi_fn] macro

extern crate alloc;

use bumpalo::Bump;
use melbi_core::{
    evaluator::{ExecutionErrorKind, RuntimeError},
    types::manager::TypeManager,
    values::{
        FfiContext,
        dynamic::Value,
        function::{AnnotatedFunction, Function},
        typed::Str,
    },
};
use melbi_macros::melbi_fn_old;

/// Simple integer addition function
#[melbi_fn_old(name = "Add")]
fn add_function(_arena: &Bump, _type_mgr: &TypeManager, a: i64, b: i64) -> i64 {
    a + b
}

/// String length function
#[melbi_fn_old(name = "Len")]
fn len_function(_arena: &Bump, _type_mgr: &TypeManager, s: Str) -> i64 {
    s.chars().count() as i64
}

/// String uppercase function with explicit lifetimes
#[melbi_fn_old(name = "Upper")]
fn string_upper<'a>(arena: &'a Bump, _type_mgr: &'a TypeManager, s: Str<'a>) -> Str<'a> {
    let upper = s.to_ascii_uppercase();
    Str::from_str(arena, &upper)
}

#[test]
fn test_macro_generates_struct() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    // Should be able to create instances
    let add_fn = Add::new(type_mgr);
    let len_fn = Len::new(type_mgr);

    // Check metadata
    assert_eq!(add_fn.name(), "Add");
    assert_eq!(len_fn.name(), "Len");

    // Check locations are set
    let (crate_name, version, file, line, col) = add_fn.location();
    // The file path will be from the macro expansion location
    assert!(
        crate_name.contains("melbi") || crate_name.contains("macro_test"),
        "{}",
        crate_name
    );
    assert!(!version.is_empty());
    assert!(file.contains("melbi_fn_old_test.rs"));
    assert!(line > 0);
    assert!(col > 0);
}

#[test]
fn test_function_trait_impl() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let add_fn = Add::new(type_mgr);

    // Check type is correct - just verify we can get the type
    let _fn_ty = add_fn.ty();

    // Create Value and call the function
    let value = Value::function(&arena, add_fn).unwrap();

    // Create arguments
    let a = Value::int(type_mgr, 5);
    let b = Value::int(type_mgr, 3);
    let args = [a, b];

    // Call the function
    let ctx = FfiContext::new(&arena, type_mgr);
    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    // Check result
    assert_eq!(result.as_int().unwrap(), 8);
}

#[test]
fn test_annotated_function_register() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    // Register the function using RecordBuilder
    let add_fn = Add::new(type_mgr);
    let builder = Value::record_builder(type_mgr);
    let builder = add_fn.register(&arena, builder).unwrap();

    // Build the record and verify it has the function
    let record = builder.build(&arena).unwrap();
    assert!(record.as_record().is_ok());
}

#[test]
fn test_string_function() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let len_fn = Len::new(type_mgr);
    let value = Value::function(&arena, len_fn).unwrap();

    // Create a string argument
    let str_ty = type_mgr.str();
    let s = Value::str(&arena, str_ty, "hello");
    let args = [s];

    // Call the function
    let ctx = FfiContext::new(&arena, type_mgr);
    let result = unsafe {
        value
            .as_function()
            .unwrap()
            .call_unchecked(&ctx, &args)
            .unwrap()
    };

    // Check result
    assert_eq!(result.as_int().unwrap(), 5);
}

// ============================================================================
// Tests for Result-returning functions
// ============================================================================

/// Division function that returns Result for error handling
#[melbi_fn_old(name = "SafeDiv")]
fn safe_div(_arena: &Bump, _type_mgr: &TypeManager, a: i64, b: i64) -> Result<i64, RuntimeError> {
    if b == 0 {
        Err(RuntimeError::DivisionByZero {})
    } else {
        Ok(a / b)
    }
}

/// Function that can return overflow error
#[melbi_fn_old(name = "CheckedNegate")]
fn checked_negate(_arena: &Bump, _type_mgr: &TypeManager, a: i64) -> Result<i64, RuntimeError> {
    if a == i64::MIN {
        Err(RuntimeError::IntegerOverflow {})
    } else {
        Ok(-a)
    }
}

#[test]
fn test_result_function_success() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let div_fn = SafeDiv::new(type_mgr);
    let value = Value::function(&arena, div_fn).unwrap();

    // 10 / 2 = 5
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
fn test_result_function_division_by_zero_error() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let div_fn = SafeDiv::new(type_mgr);
    let value = Value::function(&arena, div_fn).unwrap();

    // 10 / 0 should return DivisionByZero error
    let a = Value::int(type_mgr, 10);
    let b = Value::int(type_mgr, 0);
    let args = [a, b];

    let ctx = FfiContext::new(&arena, type_mgr);
    let result = unsafe { value.as_function().unwrap().call_unchecked(&ctx, &args) };

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(
            err.kind,
            ExecutionErrorKind::Runtime(RuntimeError::DivisionByZero {})
        ),
        "Expected DivisionByZero error, got {:?}",
        err.kind
    );
}

#[test]
fn test_result_function_overflow_error() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let negate_fn = CheckedNegate::new(type_mgr);
    let value = Value::function(&arena, negate_fn).unwrap();

    // -i64::MIN overflows
    let a = Value::int(type_mgr, i64::MIN);
    let args = [a];

    let ctx = FfiContext::new(&arena, type_mgr);
    let result = unsafe { value.as_function().unwrap().call_unchecked(&ctx, &args) };

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(
            err.kind,
            ExecutionErrorKind::Runtime(RuntimeError::IntegerOverflow {})
        ),
        "Expected IntegerOverflow error, got {:?}",
        err.kind
    );
}

#[test]
fn test_result_function_metadata() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    // Result-returning functions should have the same metadata support
    let div_fn = SafeDiv::new(type_mgr);

    assert_eq!(div_fn.name(), "SafeDiv");

    let (crate_name, version, file, line, col) = div_fn.location();
    assert!(!crate_name.is_empty());
    assert!(!version.is_empty());
    assert!(file.contains("melbi_fn_old_test.rs"));
    assert!(line > 0);
    assert!(col > 0);
}

#[test]
fn test_result_function_type_is_unwrapped() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    // The function type should be (Int, Int) -> Int, not (Int, Int) -> Result<Int, ...>
    let div_fn = SafeDiv::new(type_mgr);
    let fn_ty = div_fn.ty();

    // The return type should be Int (the Ok type), not Result
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

// ============================================================================
// Tests for new context modes
// ============================================================================

/// Pure function - no context needed at all
#[melbi_fn_old(name = "PureAdd")]
fn pure_add(a: i64, b: i64) -> i64 {
    a + b
}

/// Pure function with Result return type
#[melbi_fn_old(name = "PureCheckedAdd")]
fn pure_checked_add(a: i64, b: i64) -> Result<i64, RuntimeError> {
    a.checked_add(b).ok_or(RuntimeError::IntegerOverflow {})
}

#[test]
fn test_pure_mode_function() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let add_fn = PureAdd::new(type_mgr);

    // Check metadata
    assert_eq!(add_fn.name(), "PureAdd");

    // Create and call
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
fn test_pure_mode_with_result() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let add_fn = PureCheckedAdd::new(type_mgr);

    // Check metadata
    assert_eq!(add_fn.name(), "PureCheckedAdd");

    // Success case
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
fn test_pure_mode_overflow_error() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let add_fn = PureCheckedAdd::new(type_mgr);
    let value = Value::function(&arena, add_fn).unwrap();

    // Overflow case
    let a = Value::int(type_mgr, i64::MAX);
    let b = Value::int(type_mgr, 1);
    let args = [a, b];

    let ctx = FfiContext::new(&arena, type_mgr);
    let result = unsafe { value.as_function().unwrap().call_unchecked(&ctx, &args) };

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(
            err.kind,
            ExecutionErrorKind::Runtime(RuntimeError::IntegerOverflow {})
        ),
        "Expected IntegerOverflow error, got {:?}",
        err.kind
    );
}
