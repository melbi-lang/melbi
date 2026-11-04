extern crate alloc;

use bumpalo::Bump;
use melbi_types::ty;
use melbi_values::builders::{ArenaValueBuilder, BoxValueBuilder};
use melbi_values::dynamic::Value;
use melbi_values::traits::{ValueBuilder, ValueView};
use melbi_values::typed::{Bool, Bytes, Float, Int, Marshal, Str};

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

// Melbi types only: Rust primitives must not be marshallable.
static_assertions::assert_not_impl_any!(i64: Marshal<BoxValueBuilder>);
static_assertions::assert_not_impl_any!(f64: Marshal<BoxValueBuilder>);
static_assertions::assert_not_impl_any!(bool: Marshal<BoxValueBuilder>);

// `Float` compares by Melbi's float rule (tested in `tests/equality.rs`), which
// is total, so it can be a map key and be sorted.
static_assertions::assert_impl_all!(Float: Eq, Ord, core::hash::Hash);

// =============================================================================
// Int, Float, Bool
// =============================================================================

fn int_round_trip<B: ValueBuilder>(b: &B) {
    let value = Int(42).into_value(b);
    assert_eq!(value.as_int(), Some(42));
    assert_eq!(Int::from_value(&value), Some(Int(42)));
}
test_both_builders!(int_round_trip);

fn int_has_correct_type<B: ValueBuilder>(b: &B) {
    let tb = b.ty_builder().clone();
    let value = Int(1).into_value(b);
    assert_eq!(*value.ty(), ty!(tb, Int));
}
test_both_builders!(int_has_correct_type);

fn int_deref_and_from<B: ValueBuilder>(_b: &B) {
    let int: Int = 7.into();
    assert_eq!(*int, 7);
    assert_eq!(int, Int(7));
}
test_both_builders!(int_deref_and_from);

fn int_from_value_wrong_type_returns_none<B: ValueBuilder>(b: &B) {
    assert_eq!(Int::from_value(&Value::bool(b, true)), None);
    assert_eq!(Int::from_value(&Value::float(b, 1.0)), None);
}
test_both_builders!(int_from_value_wrong_type_returns_none);

fn float_round_trip<B: ValueBuilder>(b: &B) {
    let value = Float(-2.5).into_value(b);
    assert_eq!(value.as_float(), Some(-2.5));
    assert_eq!(Float::from_value(&value).map(|f| f.0), Some(-2.5));
}
test_both_builders!(float_round_trip);

fn float_has_correct_type<B: ValueBuilder>(b: &B) {
    let tb = b.ty_builder().clone();
    let value = Float(1.0).into_value(b);
    assert_eq!(*value.ty(), ty!(tb, Float));
}
test_both_builders!(float_has_correct_type);

fn float_deref_and_from<B: ValueBuilder>(_b: &B) {
    let float: Float = 1.5.into();
    assert_eq!(*float, 1.5);
}
test_both_builders!(float_deref_and_from);

fn float_from_value_wrong_type_returns_none<B: ValueBuilder>(b: &B) {
    assert!(Float::from_value(&Value::int(b, 1)).is_none());
}
test_both_builders!(float_from_value_wrong_type_returns_none);

fn bool_round_trip<B: ValueBuilder>(b: &B) {
    let value = Bool(true).into_value(b);
    assert_eq!(value.as_bool(), Some(true));
    assert_eq!(Bool::from_value(&value), Some(Bool(true)));
}
test_both_builders!(bool_round_trip);

fn bool_has_correct_type<B: ValueBuilder>(b: &B) {
    let tb = b.ty_builder().clone();
    let value = Bool(false).into_value(b);
    assert_eq!(*value.ty(), ty!(tb, Bool));
}
test_both_builders!(bool_has_correct_type);

fn bool_deref_and_from<B: ValueBuilder>(_b: &B) {
    let flag: Bool = true.into();
    assert!(*flag);
    assert_eq!(flag, Bool(true));
}
test_both_builders!(bool_deref_and_from);

fn bool_from_value_wrong_type_returns_none<B: ValueBuilder>(b: &B) {
    assert!(Bool::from_value(&Value::int(b, 1)).is_none());
}
test_both_builders!(bool_from_value_wrong_type_returns_none);

// =============================================================================
// Str
// =============================================================================

fn str_round_trip<B: ValueBuilder>(b: &B) {
    let value = Str::new(b, "héllo 🦀").into_value(b);
    assert_eq!(value.as_str(), Some("héllo 🦀"));

    let text = Str::from_value(&value).unwrap();
    assert_eq!(&*text, "héllo 🦀");
}
test_both_builders!(str_round_trip);

fn str_has_correct_type<B: ValueBuilder>(b: &B) {
    let tb = b.ty_builder().clone();
    let value = Str::new(b, "x").into_value(b);
    assert_eq!(*value.ty(), ty!(tb, Str));
}
test_both_builders!(str_has_correct_type);

fn str_empty<B: ValueBuilder>(b: &B) {
    let value = Str::new(b, "").into_value(b);
    assert_eq!(Str::from_value(&value).map(|s| s.len()), Some(0));
}
test_both_builders!(str_empty);

fn str_equality_compares_contents<B: ValueBuilder>(b: &B) {
    assert_eq!(Str::new(b, "same"), Str::new(b, "same"));
    assert_ne!(Str::new(b, "same"), Str::new(b, "other"));
}
test_both_builders!(str_equality_compares_contents);

fn str_from_value_wrong_type_returns_none<B: ValueBuilder>(b: &B) {
    assert!(Str::from_value(&Value::bytes(b, b"x")).is_none());
    assert!(Str::from_value(&Value::int(b, 1)).is_none());
}
test_both_builders!(str_from_value_wrong_type_returns_none);

// =============================================================================
// Bytes
// =============================================================================

fn bytes_round_trip<B: ValueBuilder>(b: &B) {
    let value = Bytes::new(b, &[0x00, 0x7f, 0xff]).into_value(b);
    assert_eq!(value.as_bytes(), Some(&[0x00, 0x7f, 0xff][..]));

    let bytes = Bytes::from_value(&value).unwrap();
    assert_eq!(&*bytes, &[0x00, 0x7f, 0xff][..]);
}
test_both_builders!(bytes_round_trip);

fn bytes_has_correct_type<B: ValueBuilder>(b: &B) {
    let tb = b.ty_builder().clone();
    let value = Bytes::new(b, &[1]).into_value(b);
    assert_eq!(*value.ty(), ty!(tb, Bytes));
}
test_both_builders!(bytes_has_correct_type);

fn bytes_equality_compares_contents<B: ValueBuilder>(b: &B) {
    assert_eq!(Bytes::new(b, &[0xff]), Bytes::new(b, &[0xff]));
    assert_ne!(Bytes::new(b, &[0xff]), Bytes::new(b, &[0x00]));
}
test_both_builders!(bytes_equality_compares_contents);

fn bytes_from_value_wrong_type_returns_none<B: ValueBuilder>(b: &B) {
    assert!(Bytes::from_value(&Value::str(b, "x")).is_none());
    assert!(Bytes::from_value(&Value::int(b, 1)).is_none());
}
test_both_builders!(bytes_from_value_wrong_type_returns_none);

// =============================================================================
// Storage layout — a string or byte handle is one word in the arena builder
// =============================================================================

// Only on 64-bit targets, where the arena handle is a single word.
#[cfg(target_pointer_width = "64")]
static_assertions::assert_eq_size!(Str<ArenaValueBuilder<'static>>, usize);

#[cfg(target_pointer_width = "64")]
static_assertions::assert_eq_size!(Bytes<ArenaValueBuilder<'static>>, usize);

// Copy only where the handle is Copy: arena yes, box no.
static_assertions::assert_impl_all!(Str<ArenaValueBuilder<'static>>: Copy);
static_assertions::assert_not_impl_any!(Str<BoxValueBuilder>: Copy);
static_assertions::assert_impl_all!(Bytes<ArenaValueBuilder<'static>>: Copy);
static_assertions::assert_not_impl_any!(Bytes<BoxValueBuilder>: Copy);
