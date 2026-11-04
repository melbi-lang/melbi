//! Statically-typed scalar newtypes: [`Int`], [`Float`] and [`Bool`].
//!
//! Each newtype wraps the Rust primitive it represents and implements
//! [`Marshal`] so it can be stored in, and read from, a builder's raw storage.
//!
//! # Example
//!
//! ```
//! use melbi_values::builders::BoxValueBuilder;
//! use melbi_values::typed::{Int, Marshal};
//!
//! let builder = BoxValueBuilder::new();
//! let value = Int(42).into_value(&builder);
//! assert_eq!(Int::from_value(&value), Some(Int(42)));
//! ```

use core::cmp::Ordering;
use core::hash::{Hash, Hasher};
use core::ops::Deref;

use melbi_types::{Scalar, Ty, TyKind};

use super::Marshal;
use super::marshal::Sealed;
use crate::float;
use crate::traits::{Val, ValueBuilder};

/// Melbi `Int`, backed by `i64`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Int(pub i64);

impl Deref for Int {
    type Target = i64;

    fn deref(&self) -> &i64 {
        &self.0
    }
}

impl From<i64> for Int {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

/// Melbi `Float`, backed by `f64`.
///
/// Deliberately does not implement `PartialEq`, `PartialOrd` or `Hash`: Melbi
/// defines its own float comparison rules, which are not the IEEE ones.
#[derive(Debug, Clone, Copy)]
pub struct Float(pub f64);

impl Deref for Float {
    type Target = f64;

    fn deref(&self) -> &f64 {
        &self.0
    }
}

// Comparisons follow Melbi's float rule, not IEEE: every `NaN` equals every
// other `NaN` and is greater than every number. See `crate::float`.

impl PartialEq for Float {
    fn eq(&self, other: &Self) -> bool {
        float::eq(self.0, other.0)
    }
}

impl Eq for Float {}

impl PartialOrd for Float {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Float {
    fn cmp(&self, other: &Self) -> Ordering {
        float::cmp(self.0, other.0)
    }
}

impl Hash for Float {
    fn hash<H: Hasher>(&self, state: &mut H) {
        float::hash(self.0, state);
    }
}

impl From<f64> for Float {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

/// Melbi `Bool`, backed by `bool`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Bool(pub bool);

impl Deref for Bool {
    type Target = bool;

    fn deref(&self) -> &bool {
        &self.0
    }
}

impl From<bool> for Bool {
    fn from(value: bool) -> Self {
        Self(value)
    }
}

// --- Marshal ---

impl Sealed for Int {}

impl<B: ValueBuilder> Marshal<B> for Int {
    fn ty(tb: &B::TB) -> Ty<B::TB> {
        TyKind::Scalar(Scalar::Int).alloc(tb)
    }

    fn matches_ty_kind(kind: &TyKind<B::TB>) -> bool {
        matches!(kind, TyKind::Scalar(Scalar::Int))
    }

    fn from_val_unchecked(val: &Val<B>) -> Self {
        Self(val.as_int_unchecked())
    }

    fn into_val(self, _builder: &B) -> Val<B> {
        Val::int(self.0)
    }
}

impl Sealed for Float {}

impl<B: ValueBuilder> Marshal<B> for Float {
    fn ty(tb: &B::TB) -> Ty<B::TB> {
        TyKind::Scalar(Scalar::Float).alloc(tb)
    }

    fn matches_ty_kind(kind: &TyKind<B::TB>) -> bool {
        matches!(kind, TyKind::Scalar(Scalar::Float))
    }

    fn from_val_unchecked(val: &Val<B>) -> Self {
        Self(val.as_float_unchecked())
    }

    fn into_val(self, _builder: &B) -> Val<B> {
        Val::float(self.0)
    }
}

impl Sealed for Bool {}

impl<B: ValueBuilder> Marshal<B> for Bool {
    fn ty(tb: &B::TB) -> Ty<B::TB> {
        TyKind::Scalar(Scalar::Bool).alloc(tb)
    }

    fn matches_ty_kind(kind: &TyKind<B::TB>) -> bool {
        matches!(kind, TyKind::Scalar(Scalar::Bool))
    }

    fn from_val_unchecked(val: &Val<B>) -> Self {
        Self(val.as_bool_unchecked())
    }

    fn into_val(self, _builder: &B) -> Val<B> {
        Val::bool(self.0)
    }
}
