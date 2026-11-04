//! Marshalling trait for zero-cost interop between Rust types and Melbi values.
//!
//! `Marshal<B>` maps Rust types to the builder's raw storage and back. It is the
//! successor to the legacy `Bridge` + `RawConvertible` traits, unified into a
//! single trait and made generic over the builder.
//!
//! # Implemented for
//!
//! - [`Int`](super::Int) (Melbi `Int`)
//! - [`Float`](super::Float) (Melbi `Float`)
//! - [`Bool`](super::Bool) (Melbi `Bool`)
//! - [`Str`](super::Str) (Melbi `Str`)
//! - [`Bytes`](super::Bytes) (Melbi `Bytes`)
//! - [`Array<B, E>`](super::Array) (Melbi `Array[E]`) — see `typed/array.rs`
//!
//! Only Melbi types implement `Marshal`; Rust primitives are wrapped in the
//! newtypes above instead.

use melbi_types::{Ty, TyKind};

use crate::dynamic::Value;
use crate::traits::{Val, ValueBuilder, ValueView};

/// Seals [`Marshal`]: only types in this crate can implement it.
///
/// Melbi supports only types that marshal efficiently into the one-word `Val`
/// storage cell, which is what lets collections store elements inline. How
/// embedders map their own Rust types belongs to the design of user-defined
/// types; until then, new marshalled types are added here.
mod sealed {
    pub trait Sealed {}
}

pub(crate) use sealed::Sealed;

/// Marshalling between a Rust type and a builder's raw value storage.
///
/// Each implementation defines how to:
/// - Construct the Melbi `Ty` for this Rust type
/// - Check if a `TyKind` matches (allocation-free)
/// - Extract a value from raw storage (unchecked — type must be verified first)
/// - Store a value into the builder
pub trait Marshal<B: ValueBuilder>: Sized + Sealed {
    /// Construct the `Ty` for this type (e.g., `Scalar::Int` for [`Int`](super::Int)).
    fn ty(tb: &B::TB) -> Ty<B::TB>;

    /// Allocation-free structural type check.
    ///
    /// Returns `true` if the given `TyKind` matches this Rust type's Melbi
    /// representation. Used by `Array::from_value` to verify types without
    /// needing a builder reference or allocating a `Ty`.
    fn matches_ty_kind(kind: &TyKind<B::TB>) -> bool;

    /// Extract a value from raw storage.
    ///
    /// The caller must have already verified the type matches (e.g., via
    /// `matches_ty_kind`). Calling this on a mismatched type is a logic error.
    fn from_val_unchecked(val: &Val<B>) -> Self;

    /// Move this value into the builder's raw storage.
    ///
    /// The `builder` is needed by types that own an allocation (strings, maps,
    /// arrays built from a Rust collection); scalars ignore it.
    fn into_val(self, builder: &B) -> Val<B>;

    /// Convert this Rust value into a dynamic [`Value`].
    fn into_value(self, builder: &B) -> Value<B> {
        let ty = Self::ty(builder.ty_builder());
        let val = self.into_val(builder);
        Value::new(ty, val)
    }

    /// Try to extract this Rust type from a dynamic [`Value`].
    ///
    /// Returns `None` if the value's type does not match.
    /// Type checking is structural and allocation-free.
    fn from_value(value: &Value<B>) -> Option<Self> {
        if !Self::matches_ty_kind(value.ty().kind()) {
            return None;
        }
        Some(Self::from_val_unchecked(value.val()))
    }
}
