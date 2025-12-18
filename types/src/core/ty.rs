use core::hash;
use core::ops::Deref;

use super::builder::TyBuilder;
use super::flags::TyFlags;
use super::kind::TyKind;

/// Lightweight wrapper around the builder's representation.
///
/// Equality and hashing delegate to `TyBuilder::ty_eq` and `TyBuilder::ty_hash`,
/// allowing builders to customize behavior (e.g., pointer-based for interning).
#[derive(Clone, Debug)]
pub struct Ty<B: TyBuilder>(B::TyHandle);

impl<B: TyBuilder> PartialEq for Ty<B> {
    fn eq(&self, other: &Self) -> bool {
        B::ty_eq(self, other)
    }
}

impl<B: TyBuilder> Eq for Ty<B> {}

impl<B: TyBuilder> hash::Hash for Ty<B> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        B::ty_hash(self, state)
    }
}

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

// === Ident ===

/// Identifier (interned string).
///
/// Equality delegates to `TyBuilder::ident_eq`, allowing builders to customize
/// behavior (e.g., pointer-based for interning).
#[derive(Debug, Clone)]
pub struct Ident<B: TyBuilder>(B::IdentHandle);

impl<B: TyBuilder> PartialEq for Ident<B> {
    fn eq(&self, other: &Self) -> bool {
        B::ident_eq(self, other)
    }
}

impl<B: TyBuilder> Eq for Ident<B> {}

impl<B: TyBuilder> hash::Hash for Ident<B> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        B::ident_hash(self, state)
    }
}

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

impl<B: TyBuilder> Deref for Ident<B> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

// === TyList ===

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TyList<B: TyBuilder>(B::TyListHandle);

impl<B: TyBuilder> TyList<B> {
    pub fn from_iter(
        builder: &B,
        iter: impl IntoIterator<Item = Ty<B>, IntoIter: ExactSizeIterator>,
    ) -> Self {
        Self(builder.alloc_ty_list(iter))
    }

    pub fn iter(&self) -> core::slice::Iter<'_, Ty<B>> {
        self.0.iter()
    }
}

impl<B: TyBuilder> Deref for TyList<B> {
    type Target = [Ty<B>];

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<'a, B: TyBuilder> IntoIterator for &'a TyList<B> {
    type Item = &'a Ty<B>;
    type IntoIter = core::slice::Iter<'a, Ty<B>>;

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
        iter: impl IntoIterator<Item = Ident<B>, IntoIter: ExactSizeIterator>,
    ) -> Self {
        Self(builder.alloc_ident_list(iter))
    }

    pub fn iter(&self) -> core::slice::Iter<'_, Ident<B>> {
        self.0.iter()
    }
}

impl<B: TyBuilder> Deref for IdentList<B> {
    type Target = [Ident<B>];

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<'a, B: TyBuilder> IntoIterator for &'a IdentList<B> {
    type Item = &'a Ident<B>;
    type IntoIter = core::slice::Iter<'a, Ident<B>>;

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
        iter: impl IntoIterator<Item = (Ident<B>, Ty<B>), IntoIter: ExactSizeIterator>,
    ) -> Self {
        Self(builder.alloc_field_list(iter))
    }

    pub fn iter(&self) -> core::slice::Iter<'_, (Ident<B>, Ty<B>)> {
        self.0.iter()
    }
}

impl<B: TyBuilder> Deref for FieldList<B> {
    type Target = [(Ident<B>, Ty<B>)];

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<'a, B: TyBuilder> IntoIterator for &'a FieldList<B> {
    type Item = &'a (Ident<B>, Ty<B>);
    type IntoIter = core::slice::Iter<'a, (Ident<B>, Ty<B>)>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
