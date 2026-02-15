extern crate alloc;

use alloc::vec;

use bumpalo::Bump;
use melbi_types::ty;
use melbi_values::{
    builders::{ArenaValueBuilder, BoxValueBuilder},
    copy::copy_value,
    dynamic::Value,
    traits::{ArrayView, ValueBuilder, ValueView},
};

// In every test, the source builder lives in an inner scope and is dropped
// before assertions. This validates that copied values don't dangle — they
// must be fully owned by the destination builder.

// =============================================================================
// Helpers
// =============================================================================

fn box_builder() -> BoxValueBuilder {
    BoxValueBuilder::new()
}

// =============================================================================
// Box → Arena: scalars
// =============================================================================

#[test]
fn int_box_to_arena() {
    let arena = Bump::new();
    let dst = ArenaValueBuilder::new(&arena);
    let copied = {
        let src = box_builder();
        let v = Value::int(&src, 42);
        copy_value(&v, &dst)
    };

    assert_eq!(copied.as_int(), Some(42));
}

#[test]
fn bool_box_to_arena() {
    let arena = Bump::new();
    let dst = ArenaValueBuilder::new(&arena);
    let copied = {
        let src = box_builder();
        let v = Value::bool(&src, true);
        copy_value(&v, &dst)
    };

    assert_eq!(copied.as_bool(), Some(true));
}

#[test]
fn float_box_to_arena() {
    let arena = Bump::new();
    let dst = ArenaValueBuilder::new(&arena);
    let copied = {
        let src = box_builder();
        let v = Value::float(&src, 3.14);
        copy_value(&v, &dst)
    };

    assert_eq!(copied.as_float(), Some(3.14));
}

// =============================================================================
// Arena → Box: scalars
// =============================================================================

#[test]
fn int_arena_to_box() {
    let dst = box_builder();
    let copied = {
        let arena = Bump::new();
        let src = ArenaValueBuilder::new(&arena);
        let v = Value::int(&src, -99);
        copy_value(&v, &dst)
    };

    assert_eq!(copied.as_int(), Some(-99));
}

#[test]
fn bool_arena_to_box() {
    let dst = box_builder();
    let copied = {
        let arena = Bump::new();
        let src = ArenaValueBuilder::new(&arena);
        let v = Value::bool(&src, false);
        copy_value(&v, &dst)
    };

    assert_eq!(copied.as_bool(), Some(false));
}

#[test]
fn float_arena_to_box() {
    let dst = box_builder();
    let copied = {
        let arena = Bump::new();
        let src = ArenaValueBuilder::new(&arena);
        let v = Value::float(&src, -2.5);
        copy_value(&v, &dst)
    };

    assert_eq!(copied.as_float(), Some(-2.5));
}

// =============================================================================
// Box → Box: scalars
// =============================================================================

#[test]
fn int_box_to_box() {
    let dst = box_builder();
    let copied = {
        let src = box_builder();
        let v = Value::int(&src, 7);
        copy_value(&v, &dst)
    };

    assert_eq!(copied.as_int(), Some(7));
}

#[test]
fn bool_box_to_box() {
    let dst = box_builder();
    let copied = {
        let src = box_builder();
        let v = Value::bool(&src, true);
        copy_value(&v, &dst)
    };

    assert_eq!(copied.as_bool(), Some(true));
}

#[test]
fn float_box_to_box() {
    let dst = box_builder();
    let copied = {
        let src = box_builder();
        let v = Value::float(&src, 2.718);
        copy_value(&v, &dst)
    };

    assert_eq!(copied.as_float(), Some(2.718));
}

// =============================================================================
// Box → Box: arrays
// =============================================================================

#[test]
fn array_of_ints_box_to_box() {
    let dst = box_builder();
    let copied = {
        let src = box_builder();
        let src_tb = src.ty_builder().clone();
        let elements = vec![Value::int(&src, 4), Value::int(&src, 5), Value::int(&src, 6)];
        let v = Value::array(&src, ty!(src_tb, Int), elements);
        copy_value(&v, &dst)
    };

    let array = copied.as_array().expect("should be an array");
    assert_eq!(array.len(), 3);
    assert_eq!(array.get(0).and_then(|e| e.as_int()), Some(4));
    assert_eq!(array.get(1).and_then(|e| e.as_int()), Some(5));
    assert_eq!(array.get(2).and_then(|e| e.as_int()), Some(6));
}

#[test]
fn empty_array_box_to_box() {
    let dst = box_builder();
    let copied = {
        let src = box_builder();
        let src_tb = src.ty_builder().clone();
        let v = Value::array(&src, ty!(src_tb, Int), vec![]);
        copy_value(&v, &dst)
    };

    let array = copied.as_array().unwrap();
    assert_eq!(array.len(), 0);
    assert!(array.is_empty());
}

// =============================================================================
// Arena → Arena: scalars (separate arenas)
// =============================================================================

#[test]
fn int_arena_to_arena() {
    let arena_dst = Bump::new();
    let dst = ArenaValueBuilder::new(&arena_dst);
    let copied = {
        let arena_src = Bump::new();
        let src = ArenaValueBuilder::new(&arena_src);
        let v = Value::int(&src, 123);
        copy_value(&v, &dst)
    };

    assert_eq!(copied.as_int(), Some(123));
}

#[test]
fn bool_arena_to_arena() {
    let arena_dst = Bump::new();
    let dst = ArenaValueBuilder::new(&arena_dst);
    let copied = {
        let arena_src = Bump::new();
        let src = ArenaValueBuilder::new(&arena_src);
        let v = Value::bool(&src, true);
        copy_value(&v, &dst)
    };

    assert_eq!(copied.as_bool(), Some(true));
}

#[test]
fn float_arena_to_arena() {
    let arena_dst = Bump::new();
    let dst = ArenaValueBuilder::new(&arena_dst);
    let copied = {
        let arena_src = Bump::new();
        let src = ArenaValueBuilder::new(&arena_src);
        let v = Value::float(&src, 0.5);
        copy_value(&v, &dst)
    };

    assert_eq!(copied.as_float(), Some(0.5));
}

// =============================================================================
// Box → Arena: arrays
// =============================================================================

#[test]
fn array_of_ints_box_to_arena() {
    let arena = Bump::new();
    let dst = ArenaValueBuilder::new(&arena);
    let copied = {
        let src = box_builder();
        let src_tb = src.ty_builder().clone();
        let elements = vec![Value::int(&src, 10), Value::int(&src, 20), Value::int(&src, 30)];
        let v = Value::array(&src, ty!(src_tb, Int), elements);
        copy_value(&v, &dst)
    };

    let array = copied.as_array().expect("should be an array");
    assert_eq!(array.len(), 3);
    assert_eq!(array.get(0).and_then(|e| e.as_int()), Some(10));
    assert_eq!(array.get(1).and_then(|e| e.as_int()), Some(20));
    assert_eq!(array.get(2).and_then(|e| e.as_int()), Some(30));
}

#[test]
fn empty_array_box_to_arena() {
    let arena = Bump::new();
    let dst = ArenaValueBuilder::new(&arena);
    let copied = {
        let src = box_builder();
        let src_tb = src.ty_builder().clone();
        let v = Value::array(&src, ty!(src_tb, Int), vec![]);
        copy_value(&v, &dst)
    };

    let array = copied.as_array().unwrap();
    assert_eq!(array.len(), 0);
    assert!(array.is_empty());
}

// =============================================================================
// Arena → Box: arrays
// =============================================================================

#[test]
fn array_of_ints_arena_to_box() {
    let dst = box_builder();
    let copied = {
        let arena = Bump::new();
        let src = ArenaValueBuilder::new(&arena);
        let src_tb = *src.ty_builder();
        let elements = vec![Value::int(&src, 5), Value::int(&src, 6)];
        let v = Value::array(&src, ty!(src_tb, Int), elements);
        copy_value(&v, &dst)
    };

    let array = copied.as_array().unwrap();
    assert_eq!(array.len(), 2);
    assert_eq!(array.get(0).and_then(|e| e.as_int()), Some(5));
    assert_eq!(array.get(1).and_then(|e| e.as_int()), Some(6));
}

#[test]
fn empty_array_arena_to_box() {
    let dst = box_builder();
    let copied = {
        let arena = Bump::new();
        let src = ArenaValueBuilder::new(&arena);
        let src_tb = *src.ty_builder();
        let v = Value::array(&src, ty!(src_tb, Int), vec![]);
        copy_value(&v, &dst)
    };

    let array = copied.as_array().unwrap();
    assert!(array.is_empty());
}

// =============================================================================
// Arena → Arena: arrays (separate arenas)
// =============================================================================

#[test]
fn array_of_ints_arena_to_arena() {
    let arena_dst = Bump::new();
    let dst = ArenaValueBuilder::new(&arena_dst);
    let copied = {
        let arena_src = Bump::new();
        let src = ArenaValueBuilder::new(&arena_src);
        let src_tb = *src.ty_builder();
        let elements = vec![Value::int(&src, 100), Value::int(&src, 200)];
        let v = Value::array(&src, ty!(src_tb, Int), elements);
        copy_value(&v, &dst)
    };

    let array = copied.as_array().unwrap();
    assert_eq!(array.len(), 2);
    assert_eq!(array.get(0).and_then(|e| e.as_int()), Some(100));
    assert_eq!(array.get(1).and_then(|e| e.as_int()), Some(200));
}

// =============================================================================
// Nested arrays
// =============================================================================

#[test]
fn nested_array_box_to_arena() {
    let arena = Bump::new();
    let dst = ArenaValueBuilder::new(&arena);
    let copied = {
        let src = box_builder();
        let src_tb = src.ty_builder().clone();
        let int_ty = ty!(src_tb, Int);
        let inner1 = Value::array(&src, int_ty.clone(), vec![Value::int(&src, 1), Value::int(&src, 2)]);
        let inner2 = Value::array(&src, int_ty.clone(), vec![Value::int(&src, 3), Value::int(&src, 4)]);
        let inner_ty = ty!(src_tb, Array[Int]);
        let outer = Value::array(&src, inner_ty, vec![inner1, inner2]);
        copy_value(&outer, &dst)
    };

    let outer_arr = copied.as_array().unwrap();
    assert_eq!(outer_arr.len(), 2);

    let first = outer_arr.get(0).unwrap();
    let first_arr = first.as_array().unwrap();
    assert_eq!(first_arr.get(0).and_then(|e| e.as_int()), Some(1));
    assert_eq!(first_arr.get(1).and_then(|e| e.as_int()), Some(2));

    let second = outer_arr.get(1).unwrap();
    let second_arr = second.as_array().unwrap();
    assert_eq!(second_arr.get(0).and_then(|e| e.as_int()), Some(3));
    assert_eq!(second_arr.get(1).and_then(|e| e.as_int()), Some(4));
}

#[test]
fn nested_array_arena_to_box() {
    let dst = box_builder();
    let copied = {
        let arena = Bump::new();
        let src = ArenaValueBuilder::new(&arena);
        let src_tb = *src.ty_builder();
        let int_ty = ty!(src_tb, Int);
        let inner1 = Value::array(&src, int_ty.clone(), vec![Value::int(&src, 10), Value::int(&src, 20)]);
        let inner2 = Value::array(&src, int_ty.clone(), vec![Value::int(&src, 30)]);
        let inner_ty = ty!(src_tb, Array[Int]);
        let outer = Value::array(&src, inner_ty, vec![inner1, inner2]);
        copy_value(&outer, &dst)
    };

    let outer_arr = copied.as_array().unwrap();
    assert_eq!(outer_arr.len(), 2);

    let first = outer_arr.get(0).unwrap();
    let first_arr = first.as_array().unwrap();
    assert_eq!(first_arr.len(), 2);
    assert_eq!(first_arr.get(0).and_then(|e| e.as_int()), Some(10));

    let second = outer_arr.get(1).unwrap();
    let second_arr = second.as_array().unwrap();
    assert_eq!(second_arr.len(), 1);
    assert_eq!(second_arr.get(0).and_then(|e| e.as_int()), Some(30));
}

#[test]
fn nested_array_arena_to_arena() {
    let arena_dst = Bump::new();
    let dst = ArenaValueBuilder::new(&arena_dst);
    let copied = {
        let arena_src = Bump::new();
        let src = ArenaValueBuilder::new(&arena_src);
        let src_tb = *src.ty_builder();
        let int_ty = ty!(src_tb, Int);
        let inner = Value::array(&src, int_ty.clone(), vec![Value::int(&src, 77)]);
        let inner_ty = ty!(src_tb, Array[Int]);
        let outer = Value::array(&src, inner_ty, vec![inner]);
        copy_value(&outer, &dst)
    };

    let outer_arr = copied.as_array().unwrap();
    let inner_val = outer_arr.get(0).unwrap();
    let inner_arr = inner_val.as_array().unwrap();
    assert_eq!(inner_arr.get(0).and_then(|e| e.as_int()), Some(77));
}

// =============================================================================
// Round-trips (each intermediate stage is scoped and dropped)
// =============================================================================

#[test]
fn round_trip_box_arena_box_int() {
    let b2 = box_builder();
    let back = {
        let arena = Bump::new();
        let a = ArenaValueBuilder::new(&arena);
        let via_arena = {
            let b1 = box_builder();
            let original = Value::int(&b1, 42);
            copy_value(&original, &a)
        };
        copy_value(&via_arena, &b2)
    };

    assert_eq!(back.as_int(), Some(42));
}

#[test]
fn round_trip_arena_box_arena_int() {
    let arena2 = Bump::new();
    let a2 = ArenaValueBuilder::new(&arena2);
    let back = {
        let b = box_builder();
        let via_box = {
            let arena1 = Bump::new();
            let a1 = ArenaValueBuilder::new(&arena1);
            let original = Value::int(&a1, -7);
            copy_value(&original, &b)
        };
        copy_value(&via_box, &a2)
    };

    assert_eq!(back.as_int(), Some(-7));
}

#[test]
fn round_trip_box_arena_box_array() {
    let b2 = box_builder();
    let back = {
        let arena = Bump::new();
        let a = ArenaValueBuilder::new(&arena);
        let via_arena = {
            let b1 = box_builder();
            let tb1 = b1.ty_builder().clone();
            let elements = vec![Value::int(&b1, 1), Value::int(&b1, 2), Value::int(&b1, 3)];
            let original = Value::array(&b1, ty!(tb1, Int), elements);
            copy_value(&original, &a)
        };
        copy_value(&via_arena, &b2)
    };

    let array = back.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array.get(0).and_then(|e| e.as_int()), Some(1));
    assert_eq!(array.get(1).and_then(|e| e.as_int()), Some(2));
    assert_eq!(array.get(2).and_then(|e| e.as_int()), Some(3));
}

#[test]
fn round_trip_arena_box_arena_nested_array() {
    let arena2 = Bump::new();
    let a2 = ArenaValueBuilder::new(&arena2);
    let back = {
        let b = box_builder();
        let via_box = {
            let arena1 = Bump::new();
            let a1 = ArenaValueBuilder::new(&arena1);
            let tb1 = *a1.ty_builder();
            let int_ty = ty!(tb1, Int);
            let inner = Value::array(&a1, int_ty, vec![Value::int(&a1, 10), Value::int(&a1, 20)]);
            let inner_ty = ty!(tb1, Array[Int]);
            let original = Value::array(&a1, inner_ty, vec![inner]);
            copy_value(&original, &b)
        };
        copy_value(&via_box, &a2)
    };

    let outer_arr = back.as_array().unwrap();
    assert_eq!(outer_arr.len(), 1);
    let inner_val = outer_arr.get(0).unwrap();
    let inner_arr = inner_val.as_array().unwrap();
    assert_eq!(inner_arr.get(0).and_then(|e| e.as_int()), Some(10));
    assert_eq!(inner_arr.get(1).and_then(|e| e.as_int()), Some(20));
}

// =============================================================================
// Type preservation (source dropped before type check)
// =============================================================================

#[test]
fn type_preserved_int_box_to_arena() {
    let arena = Bump::new();
    let dst = ArenaValueBuilder::new(&arena);
    let dst_tb = *dst.ty_builder();
    let copied = {
        let src = box_builder();
        let v = Value::int(&src, 1);
        copy_value(&v, &dst)
    };

    assert_eq!(*copied.ty(), ty!(dst_tb, Int));
}

#[test]
fn type_preserved_array_arena_to_box() {
    let dst = box_builder();
    let dst_tb = dst.ty_builder().clone();
    let copied = {
        let arena = Bump::new();
        let src = ArenaValueBuilder::new(&arena);
        let src_tb = *src.ty_builder();
        let v = Value::array(&src, ty!(src_tb, Int), vec![Value::int(&src, 1)]);
        copy_value(&v, &dst)
    };

    assert_eq!(*copied.ty(), ty!(dst_tb, Array[Int]));
}

#[test]
fn type_preserved_nested_array() {
    let arena = Bump::new();
    let dst = ArenaValueBuilder::new(&arena);
    let dst_tb = *dst.ty_builder();
    let copied = {
        let src = box_builder();
        let src_tb = src.ty_builder().clone();
        let int_ty = ty!(src_tb, Int);
        let inner = Value::array(&src, int_ty, vec![Value::int(&src, 1)]);
        let inner_ty = ty!(src_tb, Array[Int]);
        let outer = Value::array(&src, inner_ty, vec![inner]);
        copy_value(&outer, &dst)
    };

    assert_eq!(*copied.ty(), ty!(dst_tb, Array[Array[Int]]));
}
