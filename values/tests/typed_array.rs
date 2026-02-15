extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

use bumpalo::Bump;
use melbi_types::ty;
use melbi_values::{
    builders::{ArenaValueBuilder, BoxValueBuilder},
    dynamic::Value,
    traits::{ArrayView, ValueBuilder, ValueView},
    typed::{Array, Marshal},
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
// Integer arrays
// =============================================================================

fn int_array_create_and_access<B: ValueBuilder>(b: &B) {
    let arr = Array::<_, i64>::new(b, vec![10, 20, 30]);

    assert_eq!(arr.len(), 3);
    assert!(!arr.is_empty());
    assert_eq!(arr.get(0), Some(10));
    assert_eq!(arr.get(1), Some(20));
    assert_eq!(arr.get(2), Some(30));
}
test_both_builders!(int_array_create_and_access);

fn int_array_out_of_bounds<B: ValueBuilder>(b: &B) {
    let arr = Array::<_, i64>::new(b, vec![1, 2]);

    assert_eq!(arr.get(2), None);
    assert_eq!(arr.get(100), None);
}
test_both_builders!(int_array_out_of_bounds);

fn int_array_empty<B: ValueBuilder>(b: &B) {
    let arr = Array::<_, i64>::new(b, vec![]);

    assert_eq!(arr.len(), 0);
    assert!(arr.is_empty());
    assert_eq!(arr.get(0), None);
}
test_both_builders!(int_array_empty);

// =============================================================================
// Boolean arrays
// =============================================================================

fn bool_array<B: ValueBuilder>(b: &B) {
    let arr = Array::<_, bool>::new(b, vec![true, false, true]);

    assert_eq!(arr.len(), 3);
    assert_eq!(arr.get(0), Some(true));
    assert_eq!(arr.get(1), Some(false));
    assert_eq!(arr.get(2), Some(true));
}
test_both_builders!(bool_array);

// =============================================================================
// Float arrays
// =============================================================================

fn float_array<B: ValueBuilder>(b: &B) {
    let arr = Array::<_, f64>::new(b, vec![1.5, 2.5, 3.5]);

    assert_eq!(arr.len(), 3);
    assert_eq!(arr.get(0), Some(1.5));
    assert_eq!(arr.get(1), Some(2.5));
    assert_eq!(arr.get(2), Some(3.5));
}
test_both_builders!(float_array);

// =============================================================================
// Nested arrays
// =============================================================================

fn nested_array<B: ValueBuilder>(b: &B) {
    let inner1 = Array::<_, i64>::new(b, vec![1, 2]);
    let inner2 = Array::<_, i64>::new(b, vec![3, 4]);
    let outer = Array::<_, Array<_, i64>>::new(b, vec![inner1, inner2]);

    assert_eq!(outer.len(), 2);

    let first = outer.get(0).unwrap();
    assert_eq!(first.len(), 2);
    assert_eq!(first.get(0), Some(1));
    assert_eq!(first.get(1), Some(2));

    let second = outer.get(1).unwrap();
    assert_eq!(second.get(0), Some(3));
    assert_eq!(second.get(1), Some(4));
}
test_both_builders!(nested_array);

// =============================================================================
// into_value
// =============================================================================

fn into_value_preserves_values<B: ValueBuilder>(b: &B) {
    let arr = Array::<_, i64>::new(b, vec![10, 20, 30]);
    let value = arr.into_value(b);

    let dyn_arr = value.as_array().unwrap();
    assert_eq!(dyn_arr.len(), 3);
    assert_eq!(dyn_arr.get(0).and_then(|e| e.as_int()), Some(10));
    assert_eq!(dyn_arr.get(1).and_then(|e| e.as_int()), Some(20));
    assert_eq!(dyn_arr.get(2).and_then(|e| e.as_int()), Some(30));
}
test_both_builders!(into_value_preserves_values);

fn into_value_has_correct_type<B: ValueBuilder>(b: &B) {
    let tb = b.ty_builder().clone();
    let arr = Array::<_, i64>::new(b, vec![1, 2]);
    let value = arr.into_value(b);

    assert_eq!(*value.ty(), ty!(tb, Array[Int]));
}
test_both_builders!(into_value_has_correct_type);

fn nested_into_value_has_correct_type<B: ValueBuilder>(b: &B) {
    let tb = b.ty_builder().clone();
    let inner = Array::<_, i64>::new(b, vec![1]);
    let outer = Array::<_, Array<_, i64>>::new(b, vec![inner]);
    let value = outer.into_value(b);

    assert_eq!(*value.ty(), ty!(tb, Array[Array[Int]]));
}
test_both_builders!(nested_into_value_has_correct_type);

// =============================================================================
// from_value
// =============================================================================

fn from_value_succeeds<B: ValueBuilder>(b: &B) {
    let tb = b.ty_builder().clone();
    let elements = vec![Value::int(b, 10), Value::int(b, 20)];
    let value = Value::array(b, ty!(tb, Int), elements);

    let arr = Array::<_, i64>::from_value(&value).unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr.get(0), Some(10));
    assert_eq!(arr.get(1), Some(20));
}
test_both_builders!(from_value_succeeds);

fn from_value_wrong_element_type_returns_none<B: ValueBuilder>(b: &B) {
    let tb = b.ty_builder().clone();
    let elements = vec![Value::bool(b, true)];
    let value = Value::array(b, ty!(tb, Bool), elements);

    // Try to interpret Array[Bool] as Array[Int]
    let result = Array::<_, i64>::from_value(&value);
    assert!(result.is_none());
}
test_both_builders!(from_value_wrong_element_type_returns_none);

fn from_value_non_array_returns_none<B: ValueBuilder>(b: &B) {
    let value = Value::int(b, 42);

    let result = Array::<_, i64>::from_value(&value);
    assert!(result.is_none());
}
test_both_builders!(from_value_non_array_returns_none);

fn from_value_nested<B: ValueBuilder>(b: &B) {
    let tb = b.ty_builder().clone();
    let int_ty = ty!(tb, Int);

    let inner1 = Value::array(
        b,
        int_ty.clone(),
        vec![Value::int(b, 1), Value::int(b, 2)],
    );
    let inner2 = Value::array(b, int_ty, vec![Value::int(b, 3)]);
    let inner_ty = ty!(tb, Array[Int]);
    let outer = Value::array(b, inner_ty, vec![inner1, inner2]);

    let arr = Array::<_, Array<_, i64>>::from_value(&outer).unwrap();
    assert_eq!(arr.len(), 2);

    let first = arr.get(0).unwrap();
    assert_eq!(first.get(0), Some(1));
    assert_eq!(first.get(1), Some(2));

    let second = arr.get(1).unwrap();
    assert_eq!(second.get(0), Some(3));
}
test_both_builders!(from_value_nested);

// =============================================================================
// Round-trip: typed -> dynamic -> typed
// =============================================================================

fn round_trip<B: ValueBuilder>(b: &B) {
    let arr = Array::<_, i64>::new(b, vec![10, 20, 30]);

    let value = arr.into_value(b);
    let arr2 = Array::<_, i64>::from_value(&value).unwrap();

    assert_eq!(arr2.len(), 3);
    assert_eq!(arr2.get(0), Some(10));
    assert_eq!(arr2.get(1), Some(20));
    assert_eq!(arr2.get(2), Some(30));
}
test_both_builders!(round_trip);

fn round_trip_nested<B: ValueBuilder>(b: &B) {
    let inner = Array::<_, i64>::new(b, vec![1, 2]);
    let outer = Array::<_, Array<_, i64>>::new(b, vec![inner]);

    let value = outer.into_value(b);
    let outer2 = Array::<_, Array<_, i64>>::from_value(&value).unwrap();

    let inner2 = outer2.get(0).unwrap();
    assert_eq!(inner2.get(0), Some(1));
    assert_eq!(inner2.get(1), Some(2));
}
test_both_builders!(round_trip_nested);

// =============================================================================
// iter()
// =============================================================================

fn iter_values<B: ValueBuilder>(b: &B) {
    let arr = Array::<_, i64>::new(b, vec![10, 20, 30]);

    let collected: Vec<i64> = arr.iter().collect();
    assert_eq!(collected, vec![10, 20, 30]);
}
test_both_builders!(iter_values);

fn iter_empty<B: ValueBuilder>(b: &B) {
    let arr = Array::<_, i64>::new(b, vec![]);

    let collected: Vec<i64> = arr.iter().collect();
    assert!(collected.is_empty());
}
test_both_builders!(iter_empty);

// =============================================================================
// Clone
// =============================================================================

fn clone_is_independent<B: ValueBuilder>(b: &B) {
    let arr1 = Array::<_, i64>::new(b, vec![1, 2, 3]);
    let arr2 = arr1.clone();

    assert_eq!(arr1.get(0), Some(1));
    assert_eq!(arr2.get(0), Some(1));
    assert_eq!(arr1.len(), arr2.len());
}
test_both_builders!(clone_is_independent);

// =============================================================================
// Copy — arena arrays are Copy, box arrays are not
// =============================================================================

static_assertions::assert_not_impl_any!(Array<BoxValueBuilder, i64>: Copy);

fn assert_copy<T: Copy>(_: &T) {}

#[test]
fn arena_array_is_copy() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let arr = Array::<_, i64>::new(&b, vec![1, 2, 3]);

    assert_copy(&arr);

    // Use after implicit copy — original remains valid
    let arr2 = arr;
    assert_eq!(arr.get(0), Some(1));
    assert_eq!(arr2.get(0), Some(1));
}

#[test]
fn arena_nested_array_is_copy() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let inner = Array::<_, i64>::new(&b, vec![1, 2]);
    let outer = Array::<_, Array<_, i64>>::new(&b, vec![inner]);

    assert_copy(&outer);

    let outer2 = outer;
    assert_eq!(outer.get(0).unwrap().get(0), Some(1));
    assert_eq!(outer2.get(0).unwrap().get(0), Some(1));
}
