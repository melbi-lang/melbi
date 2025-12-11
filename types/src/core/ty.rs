use core::hash;
use core::ops::Deref;

use super::builder::TyBuilder;
use super::flags::TyFlags;
use super::kind::TyKind;

/// Lightweight wrapper around the builder's representation.
///
/// Note: `PartialEq`, `Eq`, and `Hash` are implemented by each builder module
/// (arena_builder.rs, box_builder.rs) to allow different equality semantics
/// (pointer-based for interning builders, structural for others).
#[derive(Clone, Debug)]
pub struct Ty<B: TyBuilder>(B::TyHandle);

impl<B: TyBuilder> Ty<B> {
    pub fn new(builder: &B, kind: TyKind<B>) -> Self {
        Self(builder.alloc(kind))
    }

    pub fn handle(&self) -> B::TyHandle {
        self.0.clone()
    }

    pub fn node(&self) -> &TyNode<B> {
        self.0.as_ref()
    }

    pub fn kind(&self) -> &TyKind<B> {
        self.node().kind()
    }

    pub fn flags(&self) -> TyFlags {
        self.node().flags()
    }
}

// Implement Copy when TyHandle is Copy (e.g., for ArenaBuilder)
impl<B: TyBuilder> Copy for Ty<B> where B::TyHandle: Copy {}

/// Note: `Hash` is implemented by each builder module to allow different
/// hashing semantics (by-kind-only for interning, structural for others).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TyNode<B: TyBuilder>(TyFlags, TyKind<B>);

impl<B: TyBuilder> TyNode<B> {
    pub fn new(kind: TyKind<B>) -> Self {
        let flags = kind.compute_flags();
        Self(flags, kind)
    }
}

impl<B: TyBuilder> TyNode<B> {
    pub fn flags(&self) -> TyFlags {
        self.0
    }

    pub fn kind(&self) -> &TyKind<B> {
        &self.1
    }
}

impl<B: TyBuilder> AsRef<TyNode<B>> for TyNode<B> {
    fn as_ref(&self) -> &TyNode<B> {
        self
    }
}

impl<B: TyBuilder> hash::Hash for TyNode<B> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        // Flags are computed from the kind, so we don't need to hash them.
        self.kind().hash(state);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Ident<B: TyBuilder>(B::IdentHandle);

impl<B: TyBuilder> Ident<B> {
    pub fn new(builder: &B, name: impl AsRef<str>) -> Self {
        Self(builder.alloc_ident(name))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }
}

// Implement Copy when IdentHandle is Copy (e.g., for ArenaBuilder)
impl<B: TyBuilder> Copy for Ident<B> where B::IdentHandle: Copy {}

// === TyList ===

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TyList<B: TyBuilder>(B::TyListHandle);

impl<B: TyBuilder> TyList<B> {
    pub fn from_iter(
        builder: &B,
        iter: impl IntoIterator<Item = B::Ty, IntoIter: ExactSizeIterator>,
    ) -> Self {
        Self(builder.alloc_ty_list(iter))
    }

    pub fn iter(&self) -> core::slice::Iter<'_, B::Ty> {
        self.0.iter()
    }
}

impl<B: TyBuilder> Deref for TyList<B> {
    type Target = [B::Ty];

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<'a, B: TyBuilder> IntoIterator for &'a TyList<B> {
    type Item = &'a B::Ty;
    type IntoIter = core::slice::Iter<'a, B::Ty>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

// === IdentList ===

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IdentList<B: TyBuilder>(B::IdentListHandle);

impl<B: TyBuilder> IdentList<B> {
    pub fn from_iter(
        builder: &B,
        iter: impl IntoIterator<Item = B::Ident, IntoIter: ExactSizeIterator>,
    ) -> Self {
        Self(builder.alloc_ident_list(iter))
    }

    pub fn iter(&self) -> core::slice::Iter<'_, B::Ident> {
        self.0.iter()
    }
}

impl<B: TyBuilder> Deref for IdentList<B> {
    type Target = [B::Ident];

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<'a, B: TyBuilder> IntoIterator for &'a IdentList<B> {
    type Item = &'a B::Ident;
    type IntoIter = core::slice::Iter<'a, B::Ident>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

// === FieldList ===

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FieldList<B: TyBuilder>(B::FieldListHandle);

impl<B: TyBuilder> FieldList<B> {
    pub fn from_iter(
        builder: &B,
        iter: impl IntoIterator<Item = (B::Ident, B::Ty), IntoIter: ExactSizeIterator>,
    ) -> Self {
        Self(builder.alloc_field_list(iter))
    }

    pub fn iter(&self) -> core::slice::Iter<'_, (B::Ident, B::Ty)> {
        self.0.iter()
    }
}

impl<B: TyBuilder> Deref for FieldList<B> {
    type Target = [(B::Ident, B::Ty)];

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<'a, B: TyBuilder> IntoIterator for &'a FieldList<B> {
    type Item = &'a (B::Ident, B::Ty);
    type IntoIter = core::slice::Iter<'a, (B::Ident, B::Ty)>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
