//! Marshalling trait for zero-cost interop between Rust types and Melbi values.
//!
//! `Marshal<B>` maps Rust types to the builder's raw storage and back. It is the
//! successor to the legacy `Bridge` + `RawConvertible` traits, unified into a
//! single trait and made generic over the builder.
//!
//! # Implemented for
//!
//! - `i64` (Melbi `Int`)
//! - `bool` (Melbi `Bool`)
//! - `f64` (Melbi `Float`)
//! - [`Array<B, E>`](super::Array) (Melbi `Array[E]`) — see `typed/array.rs`

use melbi_types::{Scalar, Ty, TyKind};

use crate::traits::{Val, ValueBuilder};

/// Marshalling between a Rust type and a builder's raw value storage.
///
/// Each implementation defines how to:
/// - Construct the Melbi `Ty` for this Rust type
/// - Check if a `TyKind` matches (allocation-free)
/// - Extract a value from raw storage (unchecked — type must be verified first)
/// - Store a value into the builder
pub trait Marshal<B: ValueBuilder>: Sized {
    /// Construct the `Ty` for this type (e.g., `Scalar::Int` for `i64`).
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

    /// Allocate this value in the builder and return a handle.
    fn into_value_handle(self, builder: &B) -> B::ValueHandle;
}

// =============================================================================
// Primitive implementations
// =============================================================================

impl<B: ValueBuilder> Marshal<B> for i64 {
    fn ty(tb: &B::TB) -> Ty<B::TB> {
        TyKind::Scalar(Scalar::Int).alloc(tb)
    }

    fn matches_ty_kind(kind: &TyKind<B::TB>) -> bool {
        matches!(kind, TyKind::Scalar(Scalar::Int))
    }

    fn from_val_unchecked(val: &Val<B>) -> Self {
        val.as_int_unchecked()
    }

    fn into_value_handle(self, builder: &B) -> B::ValueHandle {
        builder.alloc_int(self)
    }
}

impl<B: ValueBuilder> Marshal<B> for bool {
    fn ty(tb: &B::TB) -> Ty<B::TB> {
        TyKind::Scalar(Scalar::Bool).alloc(tb)
    }

    fn matches_ty_kind(kind: &TyKind<B::TB>) -> bool {
        matches!(kind, TyKind::Scalar(Scalar::Bool))
    }

    fn from_val_unchecked(val: &Val<B>) -> Self {
        val.as_bool_unchecked()
    }

    fn into_value_handle(self, builder: &B) -> B::ValueHandle {
        builder.alloc_bool(self)
    }
}

impl<B: ValueBuilder> Marshal<B> for f64 {
    fn ty(tb: &B::TB) -> Ty<B::TB> {
        TyKind::Scalar(Scalar::Float).alloc(tb)
    }

    fn matches_ty_kind(kind: &TyKind<B::TB>) -> bool {
        matches!(kind, TyKind::Scalar(Scalar::Float))
    }

    fn from_val_unchecked(val: &Val<B>) -> Self {
        val.as_float_unchecked()
    }

    fn into_value_handle(self, builder: &B) -> B::ValueHandle {
        builder.alloc_float(self)
    }
}
