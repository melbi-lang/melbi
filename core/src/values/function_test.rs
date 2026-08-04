//! Unit tests for function values (trait-based FFI).

use bumpalo::Bump;

use crate::evaluator::ExecutionError;
use crate::types::manager::TypeManager;
use crate::values::dynamic::Value;
use crate::values::from_raw::TypeError;
use crate::values::function::{FfiContext, NativeFunction};

// ============================================================================
// Test FFI Functions
// ============================================================================

/// Simple test function: add two integers
fn test_add<'types, 'arena>(
    ctx: &FfiContext<'types, 'arena>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    assert_eq!(args.len(), 2);
    let a = args[0].as_int().unwrap();
    let b = args[1].as_int().unwrap();
    Ok(Value::int(ctx.type_mgr(), a + b))
}

/// Simple test function: negate a boolean
fn test_not<'types, 'arena>(
    ctx: &FfiContext<'types, 'arena>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    assert_eq!(args.len(), 1);
    let b = args[0].as_bool().unwrap();
    Ok(Value::bool(ctx.type_mgr(), !b))
}

// ============================================================================
// Value::function() Tests
// ============================================================================

#[test]
fn value_function_construction() {
    let bump = Bump::new();
    let type_mgr = TypeManager::new(&bump);

    // Create function type: (Int, Int) -> Int
    let func_ty = type_mgr.function(&[type_mgr.int(), type_mgr.int()], type_mgr.int());

    // Create function value
    let func_value = Value::function(&bump, NativeFunction::new(func_ty, test_add));

    assert!(
        func_value.is_ok(),
        "Should create function value successfully"
    );

    let func_value = func_value.unwrap();
    assert!(core::ptr::eq(func_value.ty, func_ty), "Type should match");
}

#[test]
fn value_function_wrong_type() {
    let bump = Bump::new();
    let type_mgr = TypeManager::new(&bump);

    // Try to create function with non-function type (Int)
    let int_ty = type_mgr.int();
    let func_value = Value::function(&bump, NativeFunction::new(int_ty, test_add));

    assert!(func_value.is_err(), "Should reject non-function type");
    assert!(matches!(func_value, Err(TypeError::Mismatch)));
}

#[test]
fn value_function_different_signatures() {
    let bump = Bump::new();
    let type_mgr = TypeManager::new(&bump);

    // (Int, Int) -> Int
    let add_ty = type_mgr.function(&[type_mgr.int(), type_mgr.int()], type_mgr.int());
    let add_value = Value::function(&bump, NativeFunction::new(add_ty, test_add)).unwrap();

    // (Bool) -> Bool
    let not_ty = type_mgr.function(&[type_mgr.bool()], type_mgr.bool());
    let not_value = Value::function(&bump, NativeFunction::new(not_ty, test_not)).unwrap();

    // Both should succeed - no runtime signature validation
    assert!(core::ptr::eq(add_value.ty, add_ty));
    assert!(core::ptr::eq(not_value.ty, not_ty));
}

// ============================================================================
// Value::as_function() Tests
// ============================================================================

#[test]
fn value_as_function_extraction() {
    let bump = Bump::new();
    let type_mgr = TypeManager::new(&bump);

    let func_ty = type_mgr.function(&[type_mgr.int(), type_mgr.int()], type_mgr.int());
    let func_value = Value::function(&bump, NativeFunction::new(func_ty, test_add)).unwrap();

    // Extract function trait object
    let func_trait = func_value.as_function();
    assert!(func_trait.is_ok(), "Should extract function trait object");
}

#[test]
fn value_as_function_wrong_type() {
    let bump = Bump::new();
    let type_mgr = TypeManager::new(&bump);

    // Create an Int value, try to extract as function
    let int_value = Value::int(type_mgr, 42);
    let func_trait = int_value.as_function();

    assert!(func_trait.is_err(), "Should reject non-function value");
    assert!(matches!(func_trait, Err(TypeError::Mismatch)));
}

#[test]
fn value_as_function_call_through() {
    let bump = Bump::new();
    let type_mgr = TypeManager::new(&bump);

    // Create function value
    let func_ty = type_mgr.function(&[type_mgr.int(), type_mgr.int()], type_mgr.int());
    let func_value = Value::function(&bump, NativeFunction::new(func_ty, test_add)).unwrap();

    // Extract and call via trait
    let func_trait = func_value.as_function().unwrap();
    let args = [Value::int(type_mgr, 100), Value::int(type_mgr, 23)];

    // SAFETY: We constructed the function with correct type (Int, Int) -> Int
    // and are passing two Int arguments as expected.
    let ctx = FfiContext::new(&bump, type_mgr);
    let result = unsafe { func_trait.call_unchecked(&ctx, &args) };
    assert!(result.is_ok());

    let result_value = result.unwrap().as_int().unwrap();
    assert_eq!(result_value, 123);
}

// ============================================================================
// Memory and Lifetime Tests
// ============================================================================

#[test]
fn multiple_functions_same_arena() {
    let bump = Bump::new();
    let type_mgr = TypeManager::new(&bump);

    // Create multiple functions in same arena
    let add_ty = type_mgr.function(&[type_mgr.int(), type_mgr.int()], type_mgr.int());
    let not_ty = type_mgr.function(&[type_mgr.bool()], type_mgr.bool());

    let add_value = Value::function(&bump, NativeFunction::new(add_ty, test_add)).unwrap();
    let not_value = Value::function(&bump, NativeFunction::new(not_ty, test_not)).unwrap();

    // Both should be extractable
    assert!(add_value.as_function().is_ok());
    assert!(not_value.as_function().is_ok());

    // Verify they're different functions (different trait object pointers)
    let add_ptr = add_value.as_function().unwrap() as *const _;
    let not_ptr = not_value.as_function().unwrap() as *const _;

    assert_ne!(
        add_ptr, not_ptr,
        "Different functions should have different pointers"
    );
}

#[test]
fn trait_object_size() {
    // Verify trait object is a fat pointer (2 words)
    use core::mem::size_of;

    use crate::values::function::Function;

    // Trait object reference is a fat pointer: data pointer + vtable pointer
    assert_eq!(
        size_of::<&dyn Function>(),
        size_of::<usize>() * 2,
        "Trait object should be a fat pointer (2 words)"
    );
}
