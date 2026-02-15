extern crate alloc;

use alloc::vec;

use bumpalo::Bump;
use melbi_types::ty;
use melbi_values::{
    builders::{ArenaValueBuilder, BoxValueBuilder},
    dynamic::Value,
    traits::{ArrayView, ValueBuilder, ValueView},
};

/// Generates `#[test]` wrappers that run a generic test function against both
/// `BoxValueBuilder` and `ArenaValueBuilder`.
macro_rules! test_both_builders {
    ($name:ident) => {
        mod $name {
            use super::*;

            #[test]
            fn box_builder() {
                super::$name(&BoxValueBuilder::new());
            }

            #[test]
            fn arena_builder() {
                let arena = Bump::new();
                super::$name(&ArenaValueBuilder::new(&arena));
            }
        }
    };
}

// =============================================================================
// Integer values
// =============================================================================

fn int_value<B: ValueBuilder>(b: &B) {
    let v = Value::int(b, 42);
    assert_eq!(v.as_int(), Some(42));
}
test_both_builders!(int_value);

fn int_negative<B: ValueBuilder>(b: &B) {
    let v = Value::int(b, -100);
    assert_eq!(v.as_int(), Some(-100));
}
test_both_builders!(int_negative);

fn int_zero<B: ValueBuilder>(b: &B) {
    let v = Value::int(b, 0);
    assert_eq!(v.as_int(), Some(0));
}
test_both_builders!(int_zero);

fn int_wrong_type_returns_none<B: ValueBuilder>(b: &B) {
    let v = Value::int(b, 42);
    assert_eq!(v.as_bool(), None);
    assert_eq!(v.as_float(), None);
}
test_both_builders!(int_wrong_type_returns_none);

fn int_has_correct_type<B: ValueBuilder>(b: &B) {
    let v = Value::int(b, 42);
    let tb = b.ty_builder().clone();
    assert_eq!(*v.ty(), ty!(tb, Int));
}
test_both_builders!(int_has_correct_type);

// =============================================================================
// Boolean values
// =============================================================================

fn bool_true<B: ValueBuilder>(b: &B) {
    let v = Value::bool(b, true);
    assert_eq!(v.as_bool(), Some(true));
}
test_both_builders!(bool_true);

fn bool_false<B: ValueBuilder>(b: &B) {
    let v = Value::bool(b, false);
    assert_eq!(v.as_bool(), Some(false));
}
test_both_builders!(bool_false);

fn bool_wrong_type_returns_none<B: ValueBuilder>(b: &B) {
    let v = Value::bool(b, true);
    assert_eq!(v.as_int(), None);
    assert_eq!(v.as_float(), None);
}
test_both_builders!(bool_wrong_type_returns_none);

fn bool_has_correct_type<B: ValueBuilder>(b: &B) {
    let v = Value::bool(b, true);
    let tb = b.ty_builder().clone();
    assert_eq!(*v.ty(), ty!(tb, Bool));
}
test_both_builders!(bool_has_correct_type);

// =============================================================================
// Float values
// =============================================================================

fn float_value<B: ValueBuilder>(b: &B) {
    let v = Value::float(b, 3.14);
    assert_eq!(v.as_float(), Some(3.14));
}
test_both_builders!(float_value);

fn float_negative<B: ValueBuilder>(b: &B) {
    let v = Value::float(b, -2.5);
    assert_eq!(v.as_float(), Some(-2.5));
}
test_both_builders!(float_negative);

fn float_zero<B: ValueBuilder>(b: &B) {
    let v = Value::float(b, 0.0);
    assert_eq!(v.as_float(), Some(0.0));
}
test_both_builders!(float_zero);

fn float_wrong_type_returns_none<B: ValueBuilder>(b: &B) {
    let v = Value::float(b, 3.14);
    assert_eq!(v.as_int(), None);
    assert_eq!(v.as_bool(), None);
}
test_both_builders!(float_wrong_type_returns_none);

fn float_has_correct_type<B: ValueBuilder>(b: &B) {
    let v = Value::float(b, 1.0);
    let tb = b.ty_builder().clone();
    assert_eq!(*v.ty(), ty!(tb, Float));
}
test_both_builders!(float_has_correct_type);

// =============================================================================
// Array values
// =============================================================================

fn array_of_ints<B: ValueBuilder>(b: &B) {
    let tb = b.ty_builder().clone();
    let elem_ty = ty!(tb, Int);

    let elements = vec![Value::int(b, 10), Value::int(b, 20), Value::int(b, 30)];
    let v = Value::array(b, elem_ty, elements);

    let array = v.as_array().expect("should be an array");
    assert_eq!(array.len(), 3);
    assert!(!array.is_empty());

    assert_eq!(array.get(0).and_then(|e| e.as_int()), Some(10));
    assert_eq!(array.get(1).and_then(|e| e.as_int()), Some(20));
    assert_eq!(array.get(2).and_then(|e| e.as_int()), Some(30));
}
test_both_builders!(array_of_ints);

fn array_out_of_bounds_returns_none<B: ValueBuilder>(b: &B) {
    let tb = b.ty_builder().clone();

    let elements = vec![Value::int(b, 1), Value::int(b, 2)];
    let v = Value::array(b, ty!(tb, Int), elements);

    let array = v.as_array().unwrap();
    assert!(array.get(2).is_none());
    assert!(array.get(100).is_none());
}
test_both_builders!(array_out_of_bounds_returns_none);

fn empty_array<B: ValueBuilder>(b: &B) {
    let tb = b.ty_builder().clone();

    let v = Value::array(b, ty!(tb, Int), vec![]);

    let array = v.as_array().unwrap();
    assert_eq!(array.len(), 0);
    assert!(array.is_empty());
    assert!(array.get(0).is_none());
}
test_both_builders!(empty_array);

fn array_has_correct_type<B: ValueBuilder>(b: &B) {
    let tb = b.ty_builder().clone();

    let v = Value::array(b, ty!(tb, Int), vec![Value::int(b, 1)]);

    assert_eq!(*v.ty(), ty!(tb, Array[Int]));
}
test_both_builders!(array_has_correct_type);

fn array_elements_have_correct_type<B: ValueBuilder>(b: &B) {
    let tb = b.ty_builder().clone();
    let int_ty = ty!(tb, Int);

    let elements = vec![Value::int(b, 10), Value::int(b, 20)];
    let v = Value::array(b, int_ty, elements);

    let array = v.as_array().unwrap();
    let elem = array.get(0).unwrap();
    assert_eq!(*elem.ty(), ty!(tb, Int));
}
test_both_builders!(array_elements_have_correct_type);

fn array_wrong_type_returns_none<B: ValueBuilder>(b: &B) {
    let tb = b.ty_builder().clone();

    let v = Value::array(b, ty!(tb, Int), vec![Value::int(b, 1)]);

    assert_eq!(v.as_int(), None);
    assert_eq!(v.as_bool(), None);
    assert_eq!(v.as_float(), None);
}
test_both_builders!(array_wrong_type_returns_none);

// =============================================================================
// Nested arrays
// =============================================================================

fn nested_array<B: ValueBuilder>(b: &B) {
    let tb = b.ty_builder().clone();
    let int_ty = ty!(tb, Int);

    let inner1 = Value::array(b, int_ty.clone(), vec![Value::int(b, 1), Value::int(b, 2)]);
    let inner2 = Value::array(b, int_ty.clone(), vec![Value::int(b, 3), Value::int(b, 4)]);

    let inner_ty = ty!(tb, Array[Int]);
    let outer = Value::array(b, inner_ty, vec![inner1, inner2]);

    assert_eq!(*outer.ty(), ty!(tb, Array[Array[Int]]));

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
test_both_builders!(nested_array);

// =============================================================================
// Array of booleans and floats
// =============================================================================

fn array_of_bools<B: ValueBuilder>(b: &B) {
    let tb = b.ty_builder().clone();

    let elements = vec![Value::bool(b, true), Value::bool(b, false)];
    let v = Value::array(b, ty!(tb, Bool), elements);

    let array = v.as_array().unwrap();
    assert_eq!(array.get(0).and_then(|e| e.as_bool()), Some(true));
    assert_eq!(array.get(1).and_then(|e| e.as_bool()), Some(false));
}
test_both_builders!(array_of_bools);

fn array_of_floats<B: ValueBuilder>(b: &B) {
    let tb = b.ty_builder().clone();

    let elements = vec![Value::float(b, 1.5), Value::float(b, 2.5)];
    let v = Value::array(b, ty!(tb, Float), elements);

    let array = v.as_array().unwrap();
    assert_eq!(array.get(0).and_then(|e| e.as_float()), Some(1.5));
    assert_eq!(array.get(1).and_then(|e| e.as_float()), Some(2.5));
}
test_both_builders!(array_of_floats);

// =============================================================================
// Clone semantics
// =============================================================================

fn value_clone_is_independent<B: ValueBuilder>(b: &B) {
    let v1 = Value::int(b, 42);
    let v2 = v1.clone();

    assert_eq!(v1.as_int(), Some(42));
    assert_eq!(v2.as_int(), Some(42));
}
test_both_builders!(value_clone_is_independent);
