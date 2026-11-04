//! Maps: dynamic and typed construction, lookup, and interop between the two.

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

use bumpalo::Bump;
use melbi_types::Ty;
use melbi_values::builders::{ArenaValueBuilder, BoxValueBuilder};
use melbi_values::copy::copy_value;
use melbi_values::dynamic::{MapMaker, Value};
use melbi_values::traits::{ArrayView, MapView, ValueBuilder, ValueView};
use melbi_values::typed::{Array, Bool, Float, Int, Map, Marshal, Str};

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

fn ty<B: ValueBuilder, T: Marshal<B>>(b: &B) -> Ty<B::TB> {
    T::ty(b.ty_builder())
}

const NEGATIVE_NAN: f64 = f64::from_bits(0xfff8_0000_0000_0000);

// =============================================================================
// Dynamic maps
// =============================================================================

fn dynamic_map_lookup<B: ValueBuilder>(b: &B) {
    let map = Value::map(
        b,
        ty::<B, Str<B>>(b),
        ty::<B, Int>(b),
        vec![
            (Value::str(b, "one"), Value::int(b, 1)),
            (Value::str(b, "two"), Value::int(b, 2)),
        ],
    );
    let view = map.as_map().unwrap();

    assert_eq!(view.len(), 2);
    assert_eq!(view.get(&Value::str(b, "two")).unwrap().as_int(), Some(2));
    assert!(view.get(&Value::str(b, "three")).is_none());
    // A key of the wrong type is simply not found.
    assert!(view.get(&Value::int(b, 1)).is_none());
    assert!(Value::int(b, 1).as_map().is_none());
}
test_both_builders!(dynamic_map_lookup);

fn last_insertion_wins<B: ValueBuilder>(b: &B) {
    let mut maker = MapMaker::new(b, ty::<B, Int>(b), ty::<B, Str<B>>(b));
    maker.insert(Value::int(b, 1), Value::str(b, "first"));
    maker.insert(Value::int(b, 2), Value::str(b, "other"));
    maker.insert(Value::int(b, 1), Value::str(b, "last"));
    let map = maker.build();
    let view = map.as_map().unwrap();

    assert_eq!(view.len(), 2);
    assert_eq!(view.get(&Value::int(b, 1)).unwrap().as_str(), Some("last"));
}
test_both_builders!(last_insertion_wins);

fn empty_map_is_typed<B: ValueBuilder>(b: &B) {
    let map = MapMaker::new(b, ty::<B, Str<B>>(b), ty::<B, Int>(b)).build();

    assert_eq!(*map.ty(), ty::<B, Map<B, Str<B>, Int>>(b));
    assert!(map.as_map().unwrap().is_empty());
}
test_both_builders!(empty_map_is_typed);

fn float_keys_follow_the_float_rule<B: ValueBuilder>(b: &B) {
    let map = Value::map(
        b,
        ty::<B, Float>(b),
        ty::<B, Int>(b),
        vec![
            (Value::float(b, -0.0), Value::int(b, 1)),
            (Value::float(b, f64::NAN), Value::int(b, 2)),
            (Value::float(b, NEGATIVE_NAN), Value::int(b, 3)), // same key as NaN
        ],
    );
    let view = map.as_map().unwrap();

    assert_eq!(view.len(), 2);
    assert_eq!(view.get(&Value::float(b, 0.0)).unwrap().as_int(), Some(1));
    assert_eq!(
        view.get(&Value::float(b, f64::NAN)).unwrap().as_int(),
        Some(3)
    );
}
test_both_builders!(float_keys_follow_the_float_rule);

fn iteration_order_is_reproducible<B: ValueBuilder>(b: &B) {
    let build = || {
        let entries = (0..50)
            .map(|i| (Value::int(b, i * 7919), Value::int(b, i)))
            .collect();
        Value::map(b, ty::<B, Int>(b), ty::<B, Int>(b), entries)
    };
    let keys = |map: &Value<B>| -> Vec<i64> {
        map.as_map()
            .unwrap()
            .iter()
            .map(|(key, _)| key.as_int().unwrap())
            .collect()
    };

    assert_eq!(keys(&build()), keys(&build()));
}
test_both_builders!(iteration_order_is_reproducible);

// =============================================================================
// Typed maps, and interop with dynamic ones
// =============================================================================

fn typed_map_lookup<B: ValueBuilder>(b: &B) {
    let map = Map::<B, Str<B>, Int>::new(b, [(Str::new(b, "a"), 1), (Str::new(b, "b"), 2)]);

    assert_eq!(map.len(), 2);
    assert_eq!(map.get(&Str::new(b, "a")), Some(Int(1)));
    assert_eq!(map.get(&Str::new(b, "z")), None);
    assert!(Map::<B, Str<B>, Int>::empty(b).is_empty());
}
test_both_builders!(typed_map_lookup);

fn typed_map_read_dynamically<B: ValueBuilder>(b: &B) {
    let typed = Map::<B, Float, Array<B, Int>>::new(
        b,
        [
            (Float(1.5), Array::new(b, [1, 2])),
            (Float(f64::NAN), Array::new(b, [3])),
        ],
    );
    let value = typed.into_value(b);
    let view = value.as_map().unwrap();

    let found = view.get(&Value::float(b, NEGATIVE_NAN)).unwrap();
    assert_eq!(found.as_array().unwrap().len(), 1);
    assert!(view.get(&Value::float(b, 1.5)).is_some());
}
test_both_builders!(typed_map_read_dynamically);

fn dynamic_map_read_typed<B: ValueBuilder>(b: &B) {
    let key = |items: &[i64]| Array::<B, Int>::new(b, items.iter().copied()).into_value(b);
    let value = Value::map(
        b,
        ty::<B, Array<B, Int>>(b),
        ty::<B, Bool>(b),
        vec![
            (key(&[1, 2]), Value::bool(b, true)),
            (key(&[]), Value::bool(b, false)),
        ],
    );
    let typed = Map::<B, Array<B, Int>, Bool>::from_value(&value).unwrap();

    assert_eq!(typed.get(&Array::new(b, [1, 2])), Some(Bool(true)));
    assert_eq!(typed.get(&Array::empty(b)), Some(Bool(false)));
    assert_eq!(typed.get(&Array::new(b, [2, 1])), None);
    assert!(Map::<B, Int, Bool>::from_value(&value).is_none());
}
test_both_builders!(dynamic_map_read_typed);

fn typed_and_dynamic_maps_have_the_same_layout<B: ValueBuilder>(b: &B) {
    let typed = Map::<B, Int, Int>::new(b, (0..50).map(|i| (i * 7919, i)));
    let entries = (0..50)
        .map(|i| (Value::int(b, i * 7919), Value::int(b, i)))
        .collect();
    let dynamic = Value::map(b, ty::<B, Int>(b), ty::<B, Int>(b), entries);

    let typed_order: Vec<i64> = typed.iter().map(|(key, _)| key.0).collect();
    let dynamic_order: Vec<i64> = dynamic
        .as_map()
        .unwrap()
        .iter()
        .map(|(key, _)| key.as_int().unwrap())
        .collect();
    assert_eq!(typed_order, dynamic_order);
}
test_both_builders!(typed_and_dynamic_maps_have_the_same_layout);

// =============================================================================
// Maps inside maps
// =============================================================================

fn maps_as_keys_and_values<B: ValueBuilder>(b: &B) {
    type Inner<B> = Map<B, Int, Int>;
    // Equal maps built in a different order are the same key.
    let forward = Inner::<B>::new(b, [(1, 10), (2, 20)]);
    let backward = Inner::<B>::new(b, [(2, 20), (1, 10)]);
    assert_eq!(forward, backward);

    let outer = Map::<B, Inner<B>, Inner<B>>::new(b, [(forward, Inner::new(b, [(0, 0)]))]);
    let found = outer.get(&backward).unwrap();
    assert_eq!(found.get(&Int(0)), Some(Int(0)));

    // The same lookup through the dynamic view.
    let value = outer.into_value(b);
    let key = Inner::<B>::new(b, [(2, 20), (1, 10)]).into_value(b);
    assert!(value.as_map().unwrap().get(&key).is_some());
}
test_both_builders!(maps_as_keys_and_values);

// =============================================================================
// Copying and size
// =============================================================================

#[test]
fn copy_maps_between_builders() {
    let arena = Bump::new();
    let arena_builder = ArenaValueBuilder::new(&arena);
    let box_builder = BoxValueBuilder::new();

    let original = Map::<_, Str<_>, Int>::new(
        &box_builder,
        [
            (Str::new(&box_builder, "a"), 1),
            (Str::new(&box_builder, "b"), 2),
        ],
    )
    .into_value(&box_builder);

    let in_arena = copy_value(&original, &arena_builder);
    let typed = Map::<_, Str<_>, Int>::from_value(&in_arena).unwrap();
    assert_eq!(typed.get(&Str::new(&arena_builder, "b")), Some(Int(2)));

    let back = copy_value(&in_arena, &box_builder);
    assert!(back == original);
}

#[test]
fn arena_maps_are_one_word() {
    use core::mem::size_of;
    assert_eq!(
        size_of::<Map<ArenaValueBuilder<'static>, Int, Int>>(),
        size_of::<usize>()
    );
    assert_eq!(
        size_of::<Value<ArenaValueBuilder<'static>>>(),
        2 * size_of::<usize>()
    );
}
