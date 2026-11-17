//! Equality, ordering and hashing.
//!
//! The central property: a typed value and the same value held dynamically
//! compare and hash identically. Typed and dynamic maps share one layout, so
//! any disagreement would make a map built on one side give wrong lookups on
//! the other.

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;
use core::fmt::Debug;
use core::hash::{Hash, Hasher};

use bumpalo::Bump;
use melbi_values::builders::{ArenaValueBuilder, BoxValueBuilder};
use melbi_values::dynamic::Value;
use melbi_values::traits::ValueBuilder;
use melbi_values::typed::{Array, Bool, Bytes, Float, Hashable, Int, Map, Str};

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
// Harness
// =============================================================================

/// A hasher that records every call, so two values can be checked to feed a
/// hasher exactly the same calls. Comparing bytes alone would not do: real
/// hashers treat `write_u64(x)` and `write(&x.to_ne_bytes())` differently.
#[derive(Default)]
struct Recorder(Vec<(&'static str, Vec<u8>)>);

impl Recorder {
    fn record(&mut self, call: &'static str, bytes: &[u8]) {
        self.0.push((call, bytes.to_vec()));
    }
}

impl Hasher for Recorder {
    fn finish(&self) -> u64 {
        0
    }
    fn write(&mut self, bytes: &[u8]) {
        self.record("write", bytes);
    }
    fn write_u8(&mut self, i: u8) {
        self.record("u8", &i.to_ne_bytes());
    }
    fn write_u16(&mut self, i: u16) {
        self.record("u16", &i.to_ne_bytes());
    }
    fn write_u32(&mut self, i: u32) {
        self.record("u32", &i.to_ne_bytes());
    }
    fn write_u64(&mut self, i: u64) {
        self.record("u64", &i.to_ne_bytes());
    }
    fn write_u128(&mut self, i: u128) {
        self.record("u128", &i.to_ne_bytes());
    }
    fn write_usize(&mut self, i: usize) {
        self.record("usize", &i.to_ne_bytes());
    }
    fn write_i8(&mut self, i: i8) {
        self.record("i8", &i.to_ne_bytes());
    }
    fn write_i16(&mut self, i: i16) {
        self.record("i16", &i.to_ne_bytes());
    }
    fn write_i32(&mut self, i: i32) {
        self.record("i32", &i.to_ne_bytes());
    }
    fn write_i64(&mut self, i: i64) {
        self.record("i64", &i.to_ne_bytes());
    }
    fn write_i128(&mut self, i: i128) {
        self.record("i128", &i.to_ne_bytes());
    }
    fn write_isize(&mut self, i: isize) {
        self.record("isize", &i.to_ne_bytes());
    }
}

fn hash_calls(value: &impl Hash) -> Vec<(&'static str, Vec<u8>)> {
    let mut recorder = Recorder::default();
    value.hash(&mut recorder);
    recorder.0
}

/// For every value and every pair of values: the typed and dynamic forms agree
/// on equality and on hashing, and equal values hash equally.
fn assert_typed_matches_dynamic<B: ValueBuilder, T: Hashable<B> + Clone + Debug>(
    builder: &B,
    values: &[T],
) {
    let dynamic: Vec<Value<B>> = values
        .iter()
        .map(|value| value.clone().into_value(builder))
        .collect();

    for (typed, value) in values.iter().zip(&dynamic) {
        assert_eq!(
            hash_calls(typed),
            hash_calls(value),
            "typed and dynamic hash differ for {typed:?}"
        );
    }

    for (i, a) in values.iter().enumerate() {
        for (j, b) in values.iter().enumerate() {
            assert_eq!(
                a == b,
                dynamic[i] == dynamic[j],
                "typed and dynamic equality differ for {a:?} and {b:?}"
            );
            if a == b {
                assert_eq!(
                    hash_calls(a),
                    hash_calls(b),
                    "{a:?} == {b:?} but hashes differ"
                );
            }
        }
    }
}

// Every NaN must behave as one value, whatever its sign or payload.
const NEGATIVE_NAN: f64 = f64::from_bits(0xfff8_0000_0000_0000); // x86's 0.0 / 0.0
const PAYLOAD_NAN: f64 = f64::from_bits(0x7ff0_0000_0000_0001);

fn floats() -> Vec<Float> {
    [
        0.0,
        -0.0,
        1.5,
        -1.5,
        f64::MIN_POSITIVE,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        NEGATIVE_NAN,
        PAYLOAD_NAN,
    ]
    .map(Float)
    .to_vec()
}

// =============================================================================
// Typed and dynamic agree
// =============================================================================

fn scalars_agree<B: ValueBuilder>(b: &B) {
    assert_typed_matches_dynamic(b, &[0, 1, -1, i64::MIN, i64::MAX].map(Int));
    assert_typed_matches_dynamic(b, &floats());
    assert_typed_matches_dynamic(b, &[Bool(true), Bool(false)]);
}
test_both_builders!(scalars_agree);

fn strings_and_bytes_agree<B: ValueBuilder>(b: &B) {
    let strings = ["", "a", "ab", "é", "a\0b"].map(|s| Str::new(b, s));
    assert_typed_matches_dynamic(b, &strings);

    let bytes: [&[u8]; 5] = [b"", b"\0", b"\0\0", b"\xff", b"ab"];
    assert_typed_matches_dynamic(b, &bytes.map(|v| Bytes::new(b, v)));
}
test_both_builders!(strings_and_bytes_agree);

fn arrays_agree<B: ValueBuilder>(b: &B) {
    let ints: Vec<Array<B, Int>> = vec![
        Array::empty(b),
        Array::new(b, [1]),
        Array::new(b, [1, 2]),
        Array::new(b, [2, 1]),
    ];
    assert_typed_matches_dynamic(b, &ints);

    let floats: Vec<Array<B, Float>> = vec![
        Array::empty(b),
        Array::new(b, [0.0]),
        Array::new(b, [-0.0]),
        Array::new(b, [f64::NAN]),
        Array::new(b, [NEGATIVE_NAN]),
    ];
    assert_typed_matches_dynamic(b, &floats);

    // Nested empties: the length prefix is what keeps these apart.
    let strs = |items: &[&str]| Array::<B, Str<B>>::new(b, items.iter().map(|s| Str::new(b, s)));
    let nested: Vec<Array<B, Array<B, Str<B>>>> = vec![
        Array::empty(b),
        Array::new(b, [strs(&[])]),
        Array::new(b, [strs(&["a"])]),
        Array::new(b, [strs(&[]), strs(&["a"])]),
        Array::new(b, [strs(&["a"]), strs(&[])]),
    ];
    assert_typed_matches_dynamic(b, &nested);
}
test_both_builders!(arrays_agree);

fn maps_agree<B: ValueBuilder>(b: &B) {
    let s = |text: &str| Str::new(b, text);
    let maps: Vec<Map<B, Int, Str<B>>> = vec![
        Map::empty(b),
        Map::new(b, [(1, s("a"))]),
        Map::new(b, [(1, s("b"))]),
        Map::new(b, [(2, s("a"))]),
        // Equal maps built in different orders: equal, and hashed alike.
        Map::new(b, [(1, s("a")), (2, s("b"))]),
        Map::new(b, [(2, s("b")), (1, s("a"))]),
    ];
    assert_typed_matches_dynamic(b, &maps);

    let float_keys: Vec<Map<B, Float, Int>> = vec![
        Map::new(b, [(f64::NAN, 1)]),
        Map::new(b, [(NEGATIVE_NAN, 1)]),
        Map::new(b, [(0.0, 1)]),
        Map::new(b, [(-0.0, 1)]),
    ];
    assert_typed_matches_dynamic(b, &float_keys);
}
test_both_builders!(maps_agree);

// =============================================================================
// The float rule
// =============================================================================

#[test]
fn floats_equal_like_ieee_except_nan() {
    assert_eq!(Float(-0.0), Float(0.0));
    assert_eq!(Float(f64::NAN), Float(f64::NAN));
    assert_eq!(Float(f64::NAN), Float(NEGATIVE_NAN));
    assert_eq!(Float(f64::NAN), Float(PAYLOAD_NAN));
    assert_ne!(Float(1.0), Float(f64::NAN));
}

#[test]
fn nan_is_greater_than_every_number() {
    let ordered = [
        f64::NEG_INFINITY,
        -1.0,
        0.0,
        f64::MIN_POSITIVE,
        1.0,
        f64::INFINITY,
        f64::NAN,
    ];
    for window in ordered.windows(2) {
        assert!(
            Float(window[0]) < Float(window[1]),
            "{} < {}",
            window[0],
            window[1]
        );
    }
    assert!(Float(NEGATIVE_NAN) > Float(f64::INFINITY));
}

#[test]
fn sorting_keeps_zeros_in_input_order_and_puts_nan_last() {
    let mut values = [f64::NAN, 1.0, -0.0, f64::NEG_INFINITY, 0.0, NEGATIVE_NAN].map(Float);
    values.sort(); // stable

    let bits: Vec<u64> = values.iter().map(|value| value.0.to_bits()).collect();
    let expected = [f64::NEG_INFINITY, -0.0, 0.0, 1.0, f64::NAN, NEGATIVE_NAN].map(f64::to_bits);
    assert_eq!(bits, expected);
}

fn dynamic_floats_follow_the_rule<B: ValueBuilder>(b: &B) {
    assert!(Value::float(b, f64::NAN) == Value::float(b, NEGATIVE_NAN));
    assert!(Value::float(b, -0.0) == Value::float(b, 0.0));
}
test_both_builders!(dynamic_floats_follow_the_rule);

// =============================================================================
// Other rules
// =============================================================================

fn values_of_different_types_are_not_equal<B: ValueBuilder>(b: &B) {
    assert!(Value::int(b, 1) != Value::float(b, 1.0));
    assert!(Value::str(b, "") != Value::bytes(b, b""));
}
test_both_builders!(values_of_different_types_are_not_equal);

fn strings_bytes_and_arrays_order_lexicographically<B: ValueBuilder>(b: &B) {
    assert!(Str::new(b, "a") < Str::new(b, "ab"));
    assert!(Str::new(b, "ab") < Str::new(b, "b"));
    assert!(Bytes::new(b, b"\x01") < Bytes::new(b, b"\x01\x00"));
    assert!(Array::<B, Int>::new(b, [1]) < Array::new(b, [1, 2]));
    assert!(Array::<B, Int>::new(b, [1, 2]) < Array::new(b, [2]));
    assert!(Array::<B, Int>::empty(b) < Array::new(b, [i64::MIN]));
}
test_both_builders!(strings_bytes_and_arrays_order_lexicographically);
