extern crate alloc;

use alloc::vec;

use bumpalo::Bump;
use melbi_types::ty;
use melbi_values::{
    builders::ArenaValueBuilder,
    dynamic::Value,
    traits::{ArrayView, ValueBuilder, ValueView},
};

// =============================================================================
// Integer values
// =============================================================================

#[test]
fn int_value() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let v = Value::int(&b, 42);

    assert_eq!(v.as_int(), Some(42));
}

#[test]
fn int_negative() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let v = Value::int(&b, -100);

    assert_eq!(v.as_int(), Some(-100));
}

#[test]
fn int_zero() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let v = Value::int(&b, 0);

    assert_eq!(v.as_int(), Some(0));
}

#[test]
fn int_wrong_type_returns_none() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let v = Value::int(&b, 42);

    assert_eq!(v.as_bool(), None);
    assert_eq!(v.as_float(), None);
}

#[test]
fn int_has_correct_type() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let v = Value::int(&b, 42);
    let tb = *b.ty_builder();

    assert_eq!(v.ty(), ty!(tb, Int));
}

// =============================================================================
// Boolean values
// =============================================================================

#[test]
fn bool_true() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let v = Value::bool(&b, true);

    assert_eq!(v.as_bool(), Some(true));
}

#[test]
fn bool_false() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let v = Value::bool(&b, false);

    assert_eq!(v.as_bool(), Some(false));
}

#[test]
fn bool_wrong_type_returns_none() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let v = Value::bool(&b, true);

    assert_eq!(v.as_int(), None);
    assert_eq!(v.as_float(), None);
}

#[test]
fn bool_has_correct_type() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let v = Value::bool(&b, true);
    let tb = *b.ty_builder();

    assert_eq!(v.ty(), ty!(tb, Bool));
}

// =============================================================================
// Float values
// =============================================================================

#[test]
fn float_value() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let v = Value::float(&b, 3.14);

    assert_eq!(v.as_float(), Some(3.14));
}

#[test]
fn float_negative() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let v = Value::float(&b, -2.5);

    assert_eq!(v.as_float(), Some(-2.5));
}

#[test]
fn float_zero() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let v = Value::float(&b, 0.0);

    assert_eq!(v.as_float(), Some(0.0));
}

#[test]
fn float_wrong_type_returns_none() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let v = Value::float(&b, 3.14);

    assert_eq!(v.as_int(), None);
    assert_eq!(v.as_bool(), None);
}

#[test]
fn float_has_correct_type() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let v = Value::float(&b, 1.0);
    let tb = *b.ty_builder();

    assert_eq!(v.ty(), ty!(tb, Float));
}

// =============================================================================
// Array values
// =============================================================================

#[test]
fn array_of_ints() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let tb = *b.ty_builder();
    let elem_ty = ty!(tb, Int);

    let elements = vec![Value::int(&b, 10), Value::int(&b, 20), Value::int(&b, 30)];
    let v = Value::array(&b, elem_ty, elements);

    let array = v.as_array().expect("should be an array");
    assert_eq!(array.len(), 3);
    assert!(!array.is_empty());

    assert_eq!(array.get(0).and_then(|e| e.as_int()), Some(10));
    assert_eq!(array.get(1).and_then(|e| e.as_int()), Some(20));
    assert_eq!(array.get(2).and_then(|e| e.as_int()), Some(30));
}

#[test]
fn array_out_of_bounds_returns_none() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let tb = *b.ty_builder();

    let elements = vec![Value::int(&b, 1), Value::int(&b, 2)];
    let v = Value::array(&b, ty!(tb, Int), elements);

    let array = v.as_array().unwrap();
    assert!(array.get(2).is_none());
    assert!(array.get(100).is_none());
}

#[test]
fn empty_array() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let tb = *b.ty_builder();

    let v = Value::array(&b, ty!(tb, Int), vec![]);

    let array = v.as_array().unwrap();
    assert_eq!(array.len(), 0);
    assert!(array.is_empty());
    assert!(array.get(0).is_none());
}

#[test]
fn array_has_correct_type() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let tb = *b.ty_builder();

    let v = Value::array(&b, ty!(tb, Int), vec![Value::int(&b, 1)]);

    assert_eq!(v.ty(), ty!(tb, Array[Int]));
}

#[test]
fn array_elements_have_correct_type() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let tb = *b.ty_builder();
    let int_ty = ty!(tb, Int);

    let elements = vec![Value::int(&b, 10), Value::int(&b, 20)];
    let v = Value::array(&b, int_ty, elements);

    let array = v.as_array().unwrap();
    let elem = array.get(0).unwrap();
    assert_eq!(elem.ty(), ty!(tb, Int));
}

#[test]
fn array_wrong_type_returns_none() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let tb = *b.ty_builder();

    let v = Value::array(&b, ty!(tb, Int), vec![Value::int(&b, 1)]);

    assert_eq!(v.as_int(), None);
    assert_eq!(v.as_bool(), None);
    assert_eq!(v.as_float(), None);
}

// =============================================================================
// Nested arrays
// =============================================================================

#[test]
fn nested_array() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let tb = *b.ty_builder();
    let int_ty = ty!(tb, Int);

    let inner1 = Value::array(&b, int_ty.clone(), vec![Value::int(&b, 1), Value::int(&b, 2)]);
    let inner2 = Value::array(&b, int_ty.clone(), vec![Value::int(&b, 3), Value::int(&b, 4)]);

    let inner_ty = ty!(tb, Array[Int]);
    let outer = Value::array(&b, inner_ty, vec![inner1, inner2]);

    assert_eq!(outer.ty(), ty!(tb, Array[Array[Int]]));

    let outer_arr = outer.as_array().unwrap();
    assert_eq!(outer_arr.len(), 2);

    let first_inner = outer_arr.get(0).unwrap();
    let first_inner_arr = first_inner.as_array().unwrap();
    assert_eq!(first_inner_arr.len(), 2);
    assert_eq!(first_inner_arr.get(0).and_then(|e| e.as_int()), Some(1));
    assert_eq!(first_inner_arr.get(1).and_then(|e| e.as_int()), Some(2));

    let second_inner = outer_arr.get(1).unwrap();
    let second_inner_arr = second_inner.as_array().unwrap();
    assert_eq!(second_inner_arr.get(0).and_then(|e| e.as_int()), Some(3));
    assert_eq!(second_inner_arr.get(1).and_then(|e| e.as_int()), Some(4));
}

// =============================================================================
// Array of booleans and floats
// =============================================================================

#[test]
fn array_of_bools() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let tb = *b.ty_builder();

    let elements = vec![Value::bool(&b, true), Value::bool(&b, false)];
    let v = Value::array(&b, ty!(tb, Bool), elements);

    let array = v.as_array().unwrap();
    assert_eq!(array.get(0).and_then(|e| e.as_bool()), Some(true));
    assert_eq!(array.get(1).and_then(|e| e.as_bool()), Some(false));
}

#[test]
fn array_of_floats() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let tb = *b.ty_builder();

    let elements = vec![Value::float(&b, 1.5), Value::float(&b, 2.5)];
    let v = Value::array(&b, ty!(tb, Float), elements);

    let array = v.as_array().unwrap();
    assert_eq!(array.get(0).and_then(|e| e.as_float()), Some(1.5));
    assert_eq!(array.get(1).and_then(|e| e.as_float()), Some(2.5));
}

// =============================================================================
// Clone semantics
// =============================================================================

#[test]
fn value_clone_is_independent() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let v1 = Value::int(&b, 42);
    let v2 = v1.clone();

    assert_eq!(v1.as_int(), Some(42));
    assert_eq!(v2.as_int(), Some(42));
}
