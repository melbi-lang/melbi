//! Type-directed equality and hashing for values whose type is only known at
//! run time.
//!
//! These are the reference implementations. Typed values implement `Eq` and
//! `Hash` to match them exactly, because a map built from typed values and one
//! built from dynamic values share one layout; `tests/equality.rs` checks every
//! typed implementation against these.
//!
//! Values carry no type, so the type guides the recursion, the same way
//! `Array::get` re-attaches the element type.

use core::hash::{Hash, Hasher};

use melbi_types::{Scalar, Ty, TyKind};

use crate::float;
use crate::traits::{Val, ValueBuilder};

/// Whether `a` and `b`, both of type `ty`, are equal.
pub(crate) fn values_equal<B: ValueBuilder>(ty: &Ty<B::TB>, a: &Val<B>, b: &Val<B>) -> bool {
    match ty.kind() {
        TyKind::Scalar(Scalar::Int) => a.as_int_unchecked() == b.as_int_unchecked(),
        TyKind::Scalar(Scalar::Bool) => a.as_bool_unchecked() == b.as_bool_unchecked(),
        TyKind::Scalar(Scalar::Float) => float::eq(a.as_float_unchecked(), b.as_float_unchecked()),
        TyKind::Scalar(Scalar::Str) => **a.as_str_unchecked() == **b.as_str_unchecked(),
        TyKind::Scalar(Scalar::Bytes) => **a.as_bytes_unchecked() == **b.as_bytes_unchecked(),
        TyKind::Array(element_ty) => {
            let (a, b) = (
                a.as_array_unchecked().as_ref(),
                b.as_array_unchecked().as_ref(),
            );
            a.len() == b.len() && a.iter().zip(b).all(|(x, y)| values_equal(element_ty, x, y))
        }
        TyKind::Map(..)
        | TyKind::Record(_)
        | TyKind::Function { .. }
        | TyKind::Symbol(_)
        | TyKind::TypeVar(_) => unreachable!("no values of type {ty:?} exist yet"),
    }
}

/// Feed `value`, of type `ty`, into `state`.
///
/// Only the value is hashed, not its type: values compared for equality always
/// share a type, and typed values have no type to hash.
pub(crate) fn hash_value<B: ValueBuilder>(ty: &Ty<B::TB>, value: &Val<B>, state: &mut impl Hasher) {
    match ty.kind() {
        TyKind::Scalar(Scalar::Int) => value.as_int_unchecked().hash(state),
        TyKind::Scalar(Scalar::Bool) => value.as_bool_unchecked().hash(state),
        TyKind::Scalar(Scalar::Float) => float::hash(value.as_float_unchecked(), state),
        TyKind::Scalar(Scalar::Str) => (**value.as_str_unchecked()).hash(state),
        TyKind::Scalar(Scalar::Bytes) => (**value.as_bytes_unchecked()).hash(state),
        TyKind::Array(element_ty) => {
            let elements = value.as_array_unchecked().as_ref();
            state.write_usize(elements.len());
            for element in elements {
                hash_value(element_ty, element, state);
            }
        }
        TyKind::Map(..)
        | TyKind::Record(_)
        | TyKind::Function { .. }
        | TyKind::Symbol(_)
        | TyKind::TypeVar(_) => unreachable!("no values of type {ty:?} exist yet"),
    }
}
