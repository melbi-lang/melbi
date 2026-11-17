//! Statically-typed string and byte string handles: [`Str`] and [`Bytes`].
//!
//! Both wrap the builder's handle type directly, so no copy is made when a
//! value is read back out of storage. Their contents are read through `Deref`.
//!
//! # Example
//!
//! ```
//! use melbi_values::builders::BoxValueBuilder;
//! use melbi_values::typed::{Marshal, Str};
//!
//! let builder = BoxValueBuilder::new();
//! let value = Str::new(&builder, "hello").into_value(&builder);
//! let text = Str::from_value(&value).unwrap();
//! assert_eq!(&*text, "hello");
//! ```

use core::cmp::Ordering;
use core::fmt::{self, Debug};
use core::hash::{Hash, Hasher};
use core::ops::Deref;

use melbi_types::{Scalar, Ty, TyKind};

use super::Marshal;
use super::marshal::Sealed;
use crate::traits::{Val, ValueBuilder};

/// Melbi `Str`: a UTF-8 string, stored by the builder.
pub struct Str<B: ValueBuilder>(B::StrHandle);

impl<B: ValueBuilder> Str<B> {
    /// Allocate a string in the builder.
    pub fn new(builder: &B, value: &str) -> Self {
        Self(builder.alloc_str(value))
    }
}

impl<B: ValueBuilder> Deref for Str<B> {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

// --- Manual Clone/Copy/Debug/Eq to avoid B: Clone/Copy/Debug/Eq bounds ---

#[expect(
    clippy::expl_impl_clone_on_copy,
    reason = "derive(Clone) would add a `B: Clone` bound, which this type must not have"
)]
impl<B: ValueBuilder> Clone for Str<B> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<B: ValueBuilder> Copy for Str<B> where B::StrHandle: Copy {}

impl<B: ValueBuilder> Debug for Str<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Str").field(&self.0).finish()
    }
}

impl<B: ValueBuilder> PartialEq for Str<B> {
    fn eq(&self, other: &Self) -> bool {
        let (this, that): (&str, &str) = (self, other);
        this == that
    }
}

impl<B: ValueBuilder> Eq for Str<B> {}

impl<B: ValueBuilder> PartialOrd for Str<B> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<B: ValueBuilder> Ord for Str<B> {
    fn cmp(&self, other: &Self) -> Ordering {
        (**self).cmp(&**other)
    }
}

impl<B: ValueBuilder> Hash for Str<B> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}

/// Melbi `Bytes`: a byte string, stored by the builder.
pub struct Bytes<B: ValueBuilder>(B::BytesHandle);

impl<B: ValueBuilder> Bytes<B> {
    /// Allocate a byte string in the builder.
    pub fn new(builder: &B, value: &[u8]) -> Self {
        Self(builder.alloc_bytes(value))
    }
}

impl<B: ValueBuilder> Deref for Bytes<B> {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.0
    }
}

// --- Manual Clone/Copy/Debug/Eq to avoid B: Clone/Copy/Debug/Eq bounds ---

#[expect(
    clippy::expl_impl_clone_on_copy,
    reason = "derive(Clone) would add a `B: Clone` bound, which this type must not have"
)]
impl<B: ValueBuilder> Clone for Bytes<B> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<B: ValueBuilder> Copy for Bytes<B> where B::BytesHandle: Copy {}

impl<B: ValueBuilder> Debug for Bytes<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Bytes").field(&self.0).finish()
    }
}

impl<B: ValueBuilder> PartialEq for Bytes<B> {
    fn eq(&self, other: &Self) -> bool {
        let (this, that): (&[u8], &[u8]) = (self, other);
        this == that
    }
}

impl<B: ValueBuilder> Eq for Bytes<B> {}

impl<B: ValueBuilder> PartialOrd for Bytes<B> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<B: ValueBuilder> Ord for Bytes<B> {
    fn cmp(&self, other: &Self) -> Ordering {
        (**self).cmp(&**other)
    }
}

impl<B: ValueBuilder> Hash for Bytes<B> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}

// --- Marshal ---

impl<B: ValueBuilder> Sealed for Str<B> {}

impl<B: ValueBuilder> Marshal<B> for Str<B> {
    fn ty(tb: &B::TB) -> Ty<B::TB> {
        TyKind::Scalar(Scalar::Str).alloc(tb)
    }

    fn matches_ty_kind(kind: &TyKind<B::TB>) -> bool {
        matches!(kind, TyKind::Scalar(Scalar::Str))
    }

    fn from_val_unchecked(val: &Val<B>) -> Self {
        Self(val.as_str_unchecked().clone())
    }

    fn into_val(self, _builder: &B) -> Val<B> {
        Val::str(self.0)
    }
}

impl<B: ValueBuilder> Sealed for Bytes<B> {}

impl<B: ValueBuilder> Marshal<B> for Bytes<B> {
    fn ty(tb: &B::TB) -> Ty<B::TB> {
        TyKind::Scalar(Scalar::Bytes).alloc(tb)
    }

    fn matches_ty_kind(kind: &TyKind<B::TB>) -> bool {
        matches!(kind, TyKind::Scalar(Scalar::Bytes))
    }

    fn from_val_unchecked(val: &Val<B>) -> Self {
        Self(val.as_bytes_unchecked().clone())
    }

    fn into_val(self, _builder: &B) -> Val<B> {
        Val::bytes(self.0)
    }
}
