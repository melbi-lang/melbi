use alloc::string::ToString;

use crate::types::manager::TypeManager;
use crate::values::TypeError;
use crate::values::dynamic::Value;
use crate::{String, format};

#[test]
fn array_value() {
    let arena = bumpalo::Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let int_ty = type_mgr.int();
    let array_ty = type_mgr.array(int_ty);

    // Use new dynamic API instead of raw construction
    let array_value = Value::array(
        &arena,
        array_ty,
        &[
            Value::int(type_mgr, 1),
            Value::int(type_mgr, 2),
            Value::int(type_mgr, 3),
        ],
    )
    .unwrap();

    // Use dynamic array API
    let array = array_value.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array.get(0).unwrap().as_int().unwrap(), 1);
    assert_eq!(array.get(1).unwrap().as_int().unwrap(), 2);
    assert_eq!(array.get(2).unwrap().as_int().unwrap(), 3);
}

// --- New Dynamic API Tests ---

#[test]
fn primitives() {
    let arena = bumpalo::Bump::new();
    let type_mgr = TypeManager::new(&arena);

    // Primitives: type from TypeManager, no Result, no arena
    let int_val = Value::int(type_mgr, 42);
    let float_val = Value::float(type_mgr, 3.14);
    let bool_val = Value::bool(type_mgr, true);

    // Extract using dynamic API (no compile-time type knowledge)
    assert_eq!(int_val.as_int().unwrap(), 42);
    assert_eq!(float_val.as_float().unwrap(), 3.14);
    assert!(bool_val.as_bool().unwrap());
}

#[test]
fn array_simple() {
    let arena = bumpalo::Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let int_ty = type_mgr.int();
    let array_ty = type_mgr.array(int_ty);

    // Array: explicit type (needed for empty case), returns Result
    let array_val = Value::array(
        &arena,
        array_ty,
        &[
            Value::int(type_mgr, 1),
            Value::int(type_mgr, 2),
            Value::int(type_mgr, 3),
        ],
    )
    .unwrap();

    // Access using DynamicArray - no compile-time type knowledge
    let arr = array_val.as_array().unwrap();
    assert_eq!(arr.len(), 3);

    // Get elements as Values, not as compile-time types
    assert_eq!(arr.get(0).unwrap().as_int().unwrap(), 1);
    assert_eq!(arr.get(1).unwrap().as_int().unwrap(), 2);
    assert_eq!(arr.get(2).unwrap().as_int().unwrap(), 3);
}

#[test]
fn array_nested() {
    let arena = bumpalo::Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let int_ty = type_mgr.int();
    let array_ty = type_mgr.array(int_ty);
    let array_of_array_ty = type_mgr.array(array_ty);

    // Create nested array: [[1, 2], [3, 4]]
    let nested = Value::array(
        &arena,
        array_of_array_ty,
        &[
            Value::array(
                &arena,
                array_ty,
                &[Value::int(type_mgr, 1), Value::int(type_mgr, 2)],
            )
            .unwrap(),
            Value::array(
                &arena,
                array_ty,
                &[Value::int(type_mgr, 3), Value::int(type_mgr, 4)],
            )
            .unwrap(),
        ],
    )
    .unwrap();

    // Dynamic access to nested structure
    let outer = nested.as_array().unwrap();
    assert_eq!(outer.len(), 2);

    let inner0 = outer.get(0).unwrap().as_array().unwrap();
    assert_eq!(inner0.len(), 2);
    assert_eq!(inner0.get(0).unwrap().as_int().unwrap(), 1);
    assert_eq!(inner0.get(1).unwrap().as_int().unwrap(), 2);
}

#[test]
fn array_type_mismatch() {
    use crate::values::TypeError;

    let arena = bumpalo::Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let int_ty = type_mgr.int();
    let array_ty = type_mgr.array(int_ty);

    // Try to create array with wrong element type - should fail
    let result = Value::array(
        &arena,
        array_ty,
        &[
            Value::int(type_mgr, 1),
            Value::float(type_mgr, 2.0), // Wrong type!
        ],
    );

    assert!(result.is_err());
    assert!(matches!(result, Err(TypeError::Mismatch)));
}

#[test]
fn array_empty() {
    let arena = bumpalo::Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let int_ty = type_mgr.int();
    let array_ty = type_mgr.array(int_ty);

    // Empty array: type must be explicit (can't infer from elements)
    let empty = Value::array(&arena, array_ty, &[]).unwrap();

    let arr = empty.as_array().unwrap();
    assert_eq!(arr.len(), 0);
}

#[test]
fn dynamic_array_formatting() {
    use crate::types::Type;

    let arena = bumpalo::Bump::new();
    let type_mgr = TypeManager::new(&arena);

    // Helper function that formats any array without knowing element type at compile time
    fn format_array(value: &Value) -> Result<String, TypeError> {
        let arr = value.as_array()?;

        let mut result = String::from("[");
        for i in 0..arr.len() {
            if i > 0 {
                result.push_str(", ");
            }

            let elem = arr.get(i).unwrap();

            // Recursively format based on runtime type
            let formatted = match elem.ty {
                Type::Int => elem.as_int()?.to_string(),
                Type::Float => elem.as_float()?.to_string(),
                Type::Bool => elem.as_bool()?.to_string(),
                Type::Str => format!("\"{}\"", elem.as_str()?),
                Type::Array(_) => format_array(&elem)?, // Recursive!
                _ => "?".to_string(),
            };

            result.push_str(&formatted);
        }
        result.push(']');

        Ok(result)
    }

    // Test 1: Array of integers [1, 2, 3]
    let int_array = Value::array(
        &arena,
        type_mgr.array(type_mgr.int()),
        &[
            Value::int(type_mgr, 1),
            Value::int(type_mgr, 2),
            Value::int(type_mgr, 3),
        ],
    )
    .unwrap();

    assert_eq!(format_array(&int_array).unwrap(), "[1, 2, 3]");

    // Test 2: Nested array of integers [[10, 20], [30, 40]]
    let nested_array = Value::array(
        &arena,
        type_mgr.array(type_mgr.array(type_mgr.int())),
        &[
            Value::array(
                &arena,
                type_mgr.array(type_mgr.int()),
                &[Value::int(type_mgr, 10), Value::int(type_mgr, 20)],
            )
            .unwrap(),
            Value::array(
                &arena,
                type_mgr.array(type_mgr.int()),
                &[Value::int(type_mgr, 30), Value::int(type_mgr, 40)],
            )
            .unwrap(),
        ],
    )
    .unwrap();

    assert_eq!(format_array(&nested_array).unwrap(), "[[10, 20], [30, 40]]");
}
