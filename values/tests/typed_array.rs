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

// =============================================================================
// Helpers
// =============================================================================

fn box_builder() -> BoxValueBuilder {
    BoxValueBuilder::new()
}

// =============================================================================
// Integer arrays — BoxValueBuilder
// =============================================================================

#[test]
fn box_int_array_create_and_access() {
    let b = box_builder();
    let arr = Array::<_, i64>::new(&b, vec![10, 20, 30]);

    assert_eq!(arr.len(), 3);
    assert!(!arr.is_empty());
    assert_eq!(arr.get(0), Some(10));
    assert_eq!(arr.get(1), Some(20));
    assert_eq!(arr.get(2), Some(30));
}

#[test]
fn box_int_array_out_of_bounds() {
    let b = box_builder();
    let arr = Array::<_, i64>::new(&b, vec![1, 2]);

    assert_eq!(arr.get(2), None);
    assert_eq!(arr.get(100), None);
}

#[test]
fn box_int_array_empty() {
    let b = box_builder();
    let arr = Array::<_, i64>::new(&b, vec![]);

    assert_eq!(arr.len(), 0);
    assert!(arr.is_empty());
    assert_eq!(arr.get(0), None);
}

// =============================================================================
// Boolean arrays — BoxValueBuilder
// =============================================================================

#[test]
fn box_bool_array() {
    let b = box_builder();
    let arr = Array::<_, bool>::new(&b, vec![true, false, true]);

    assert_eq!(arr.len(), 3);
    assert_eq!(arr.get(0), Some(true));
    assert_eq!(arr.get(1), Some(false));
    assert_eq!(arr.get(2), Some(true));
}

// =============================================================================
// Float arrays — BoxValueBuilder
// =============================================================================

#[test]
fn box_float_array() {
    let b = box_builder();
    let arr = Array::<_, f64>::new(&b, vec![1.5, 2.5, 3.5]);

    assert_eq!(arr.len(), 3);
    assert_eq!(arr.get(0), Some(1.5));
    assert_eq!(arr.get(1), Some(2.5));
    assert_eq!(arr.get(2), Some(3.5));
}

// =============================================================================
// Nested arrays — BoxValueBuilder
// =============================================================================

#[test]
fn box_nested_array() {
    let b = box_builder();
    let inner1 = Array::<_, i64>::new(&b, vec![1, 2]);
    let inner2 = Array::<_, i64>::new(&b, vec![3, 4]);
    let outer = Array::<_, Array<_, i64>>::new(&b, vec![inner1, inner2]);

    assert_eq!(outer.len(), 2);

    let first = outer.get(0).unwrap();
    assert_eq!(first.len(), 2);
    assert_eq!(first.get(0), Some(1));
    assert_eq!(first.get(1), Some(2));

    let second = outer.get(1).unwrap();
    assert_eq!(second.get(0), Some(3));
    assert_eq!(second.get(1), Some(4));
}

// =============================================================================
// into_value — BoxValueBuilder
// =============================================================================

#[test]
fn box_into_value_preserves_values() {
    let b = box_builder();
    let arr = Array::<_, i64>::new(&b, vec![10, 20, 30]);
    let value = arr.into_value(&b);

    let dyn_arr = value.as_array().unwrap();
    assert_eq!(dyn_arr.len(), 3);
    assert_eq!(dyn_arr.get(0).and_then(|e| e.as_int()), Some(10));
    assert_eq!(dyn_arr.get(1).and_then(|e| e.as_int()), Some(20));
    assert_eq!(dyn_arr.get(2).and_then(|e| e.as_int()), Some(30));
}

#[test]
fn box_into_value_has_correct_type() {
    let b = box_builder();
    let tb = b.ty_builder().clone();
    let arr = Array::<_, i64>::new(&b, vec![1, 2]);
    let value = arr.into_value(&b);

    assert_eq!(*value.ty(), ty!(tb, Array[Int]));
}

#[test]
fn box_nested_into_value_has_correct_type() {
    let b = box_builder();
    let tb = b.ty_builder().clone();
    let inner = Array::<_, i64>::new(&b, vec![1]);
    let outer = Array::<_, Array<_, i64>>::new(&b, vec![inner]);
    let value = outer.into_value(&b);

    assert_eq!(*value.ty(), ty!(tb, Array[Array[Int]]));
}

// =============================================================================
// from_value — BoxValueBuilder
// =============================================================================

#[test]
fn box_from_value_succeeds() {
    let b = box_builder();
    let tb = b.ty_builder().clone();
    let elements = vec![Value::int(&b, 10), Value::int(&b, 20)];
    let value = Value::array(&b, ty!(tb, Int), elements);

    let arr = Array::<_, i64>::from_value(&value).unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr.get(0), Some(10));
    assert_eq!(arr.get(1), Some(20));
}

#[test]
fn box_from_value_wrong_element_type_returns_none() {
    let b = box_builder();
    let tb = b.ty_builder().clone();
    let elements = vec![Value::bool(&b, true)];
    let value = Value::array(&b, ty!(tb, Bool), elements);

    // Try to interpret Array[Bool] as Array[Int]
    let result = Array::<_, i64>::from_value(&value);
    assert!(result.is_none());
}

#[test]
fn box_from_value_non_array_returns_none() {
    let b = box_builder();
    let value = Value::int(&b, 42);

    let result = Array::<_, i64>::from_value(&value);
    assert!(result.is_none());
}

#[test]
fn box_from_value_nested() {
    let b = box_builder();
    let tb = b.ty_builder().clone();
    let int_ty = ty!(tb, Int);

    let inner1 = Value::array(
        &b,
        int_ty.clone(),
        vec![Value::int(&b, 1), Value::int(&b, 2)],
    );
    let inner2 = Value::array(&b, int_ty, vec![Value::int(&b, 3)]);
    let inner_ty = ty!(tb, Array[Int]);
    let outer = Value::array(&b, inner_ty, vec![inner1, inner2]);

    let arr = Array::<_, Array<_, i64>>::from_value(&outer).unwrap();
    assert_eq!(arr.len(), 2);

    let first = arr.get(0).unwrap();
    assert_eq!(first.get(0), Some(1));
    assert_eq!(first.get(1), Some(2));

    let second = arr.get(1).unwrap();
    assert_eq!(second.get(0), Some(3));
}

// =============================================================================
// Round-trip: typed -> dynamic -> typed — BoxValueBuilder
// =============================================================================

#[test]
fn box_round_trip() {
    let b = box_builder();
    let arr = Array::<_, i64>::new(&b, vec![10, 20, 30]);

    let value = arr.into_value(&b);
    let arr2 = Array::<_, i64>::from_value(&value).unwrap();

    assert_eq!(arr2.len(), 3);
    assert_eq!(arr2.get(0), Some(10));
    assert_eq!(arr2.get(1), Some(20));
    assert_eq!(arr2.get(2), Some(30));
}

#[test]
fn box_round_trip_nested() {
    let b = box_builder();
    let inner = Array::<_, i64>::new(&b, vec![1, 2]);
    let outer = Array::<_, Array<_, i64>>::new(&b, vec![inner]);

    let value = outer.into_value(&b);
    let outer2 = Array::<_, Array<_, i64>>::from_value(&value).unwrap();

    let inner2 = outer2.get(0).unwrap();
    assert_eq!(inner2.get(0), Some(1));
    assert_eq!(inner2.get(1), Some(2));
}

// =============================================================================
// iter() — BoxValueBuilder
// =============================================================================

#[test]
fn box_iter() {
    let b = box_builder();
    let arr = Array::<_, i64>::new(&b, vec![10, 20, 30]);

    let collected: Vec<i64> = arr.iter().collect();
    assert_eq!(collected, vec![10, 20, 30]);
}

#[test]
fn box_iter_empty() {
    let b = box_builder();
    let arr = Array::<_, i64>::new(&b, vec![]);

    let collected: Vec<i64> = arr.iter().collect();
    assert!(collected.is_empty());
}

// =============================================================================
// Clone — BoxValueBuilder
// =============================================================================

#[test]
fn box_clone_is_independent() {
    let b = box_builder();
    let arr1 = Array::<_, i64>::new(&b, vec![1, 2, 3]);
    let arr2 = arr1.clone();

    assert_eq!(arr1.get(0), Some(1));
    assert_eq!(arr2.get(0), Some(1));
    assert_eq!(arr1.len(), arr2.len());
}

// =============================================================================
// Integer arrays — ArenaValueBuilder
// =============================================================================

#[test]
fn arena_int_array_create_and_access() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let arr = Array::<_, i64>::new(&b, vec![10, 20, 30]);

    assert_eq!(arr.len(), 3);
    assert!(!arr.is_empty());
    assert_eq!(arr.get(0), Some(10));
    assert_eq!(arr.get(1), Some(20));
    assert_eq!(arr.get(2), Some(30));
}

#[test]
fn arena_int_array_out_of_bounds() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let arr = Array::<_, i64>::new(&b, vec![1, 2]);

    assert_eq!(arr.get(2), None);
    assert_eq!(arr.get(100), None);
}

#[test]
fn arena_int_array_empty() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let arr = Array::<_, i64>::new(&b, vec![]);

    assert_eq!(arr.len(), 0);
    assert!(arr.is_empty());
    assert_eq!(arr.get(0), None);
}

// =============================================================================
// Boolean arrays — ArenaValueBuilder
// =============================================================================

#[test]
fn arena_bool_array() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let arr = Array::<_, bool>::new(&b, vec![true, false, true]);

    assert_eq!(arr.len(), 3);
    assert_eq!(arr.get(0), Some(true));
    assert_eq!(arr.get(1), Some(false));
    assert_eq!(arr.get(2), Some(true));
}

// =============================================================================
// Float arrays — ArenaValueBuilder
// =============================================================================

#[test]
fn arena_float_array() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let arr = Array::<_, f64>::new(&b, vec![1.5, 2.5, 3.5]);

    assert_eq!(arr.len(), 3);
    assert_eq!(arr.get(0), Some(1.5));
    assert_eq!(arr.get(1), Some(2.5));
    assert_eq!(arr.get(2), Some(3.5));
}

// =============================================================================
// Nested arrays — ArenaValueBuilder
// =============================================================================

#[test]
fn arena_nested_array() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let inner1 = Array::<_, i64>::new(&b, vec![1, 2]);
    let inner2 = Array::<_, i64>::new(&b, vec![3, 4]);
    let outer = Array::<_, Array<_, i64>>::new(&b, vec![inner1, inner2]);

    assert_eq!(outer.len(), 2);

    let first = outer.get(0).unwrap();
    assert_eq!(first.len(), 2);
    assert_eq!(first.get(0), Some(1));
    assert_eq!(first.get(1), Some(2));

    let second = outer.get(1).unwrap();
    assert_eq!(second.get(0), Some(3));
    assert_eq!(second.get(1), Some(4));
}

// =============================================================================
// into_value — ArenaValueBuilder
// =============================================================================

#[test]
fn arena_into_value_preserves_values() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let arr = Array::<_, i64>::new(&b, vec![10, 20, 30]);
    let value = arr.into_value(&b);

    let dyn_arr = value.as_array().unwrap();
    assert_eq!(dyn_arr.len(), 3);
    assert_eq!(dyn_arr.get(0).and_then(|e| e.as_int()), Some(10));
    assert_eq!(dyn_arr.get(1).and_then(|e| e.as_int()), Some(20));
    assert_eq!(dyn_arr.get(2).and_then(|e| e.as_int()), Some(30));
}

#[test]
fn arena_into_value_has_correct_type() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let tb = *b.ty_builder();
    let arr = Array::<_, i64>::new(&b, vec![1, 2]);
    let value = arr.into_value(&b);

    assert_eq!(*value.ty(), ty!(tb, Array[Int]));
}

#[test]
fn arena_nested_into_value_has_correct_type() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let tb = *b.ty_builder();
    let inner = Array::<_, i64>::new(&b, vec![1]);
    let outer = Array::<_, Array<_, i64>>::new(&b, vec![inner]);
    let value = outer.into_value(&b);

    assert_eq!(*value.ty(), ty!(tb, Array[Array[Int]]));
}

// =============================================================================
// from_value — ArenaValueBuilder
// =============================================================================

#[test]
fn arena_from_value_succeeds() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let tb = *b.ty_builder();
    let elements = vec![Value::int(&b, 10), Value::int(&b, 20)];
    let value = Value::array(&b, ty!(tb, Int), elements);

    let arr = Array::<_, i64>::from_value(&value).unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr.get(0), Some(10));
    assert_eq!(arr.get(1), Some(20));
}

#[test]
fn arena_from_value_wrong_element_type_returns_none() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let tb = *b.ty_builder();
    let elements = vec![Value::bool(&b, true)];
    let value = Value::array(&b, ty!(tb, Bool), elements);

    let result = Array::<_, i64>::from_value(&value);
    assert!(result.is_none());
}

#[test]
fn arena_from_value_non_array_returns_none() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let value = Value::int(&b, 42);

    let result = Array::<_, i64>::from_value(&value);
    assert!(result.is_none());
}

#[test]
fn arena_from_value_nested() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let tb = *b.ty_builder();
    let int_ty = ty!(tb, Int);

    let inner1 = Value::array(
        &b,
        int_ty.clone(),
        vec![Value::int(&b, 1), Value::int(&b, 2)],
    );
    let inner2 = Value::array(&b, int_ty, vec![Value::int(&b, 3)]);
    let inner_ty = ty!(tb, Array[Int]);
    let outer = Value::array(&b, inner_ty, vec![inner1, inner2]);

    let arr = Array::<_, Array<_, i64>>::from_value(&outer).unwrap();
    assert_eq!(arr.len(), 2);

    let first = arr.get(0).unwrap();
    assert_eq!(first.get(0), Some(1));
    assert_eq!(first.get(1), Some(2));

    let second = arr.get(1).unwrap();
    assert_eq!(second.get(0), Some(3));
}

// =============================================================================
// Round-trip: typed -> dynamic -> typed — ArenaValueBuilder
// =============================================================================

#[test]
fn arena_round_trip() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let arr = Array::<_, i64>::new(&b, vec![10, 20, 30]);

    let value = arr.into_value(&b);
    let arr2 = Array::<_, i64>::from_value(&value).unwrap();

    assert_eq!(arr2.len(), 3);
    assert_eq!(arr2.get(0), Some(10));
    assert_eq!(arr2.get(1), Some(20));
    assert_eq!(arr2.get(2), Some(30));
}

#[test]
fn arena_round_trip_nested() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let inner = Array::<_, i64>::new(&b, vec![1, 2]);
    let outer = Array::<_, Array<_, i64>>::new(&b, vec![inner]);

    let value = outer.into_value(&b);
    let outer2 = Array::<_, Array<_, i64>>::from_value(&value).unwrap();

    let inner2 = outer2.get(0).unwrap();
    assert_eq!(inner2.get(0), Some(1));
    assert_eq!(inner2.get(1), Some(2));
}

// =============================================================================
// iter() — ArenaValueBuilder
// =============================================================================

#[test]
fn arena_iter() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let arr = Array::<_, i64>::new(&b, vec![10, 20, 30]);

    let collected: Vec<i64> = arr.iter().collect();
    assert_eq!(collected, vec![10, 20, 30]);
}

#[test]
fn arena_iter_empty() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let arr = Array::<_, i64>::new(&b, vec![]);

    let collected: Vec<i64> = arr.iter().collect();
    assert!(collected.is_empty());
}

// =============================================================================
// Clone — ArenaValueBuilder
// =============================================================================

#[test]
fn arena_clone_is_independent() {
    let arena = Bump::new();
    let b = ArenaValueBuilder::new(&arena);
    let arr1 = Array::<_, i64>::new(&b, vec![1, 2, 3]);
    let arr2 = arr1.clone();

    assert_eq!(arr1.get(0), Some(1));
    assert_eq!(arr2.get(0), Some(1));
    assert_eq!(arr1.len(), arr2.len());
}

// =============================================================================
// Copy — ArenaValueBuilder (arena arrays are Copy, box arrays are not)
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
