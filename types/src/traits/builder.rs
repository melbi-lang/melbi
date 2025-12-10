use core::{fmt::Debug, hash::Hash, ops::Deref};

use crate::{kind::TyKind, traits::TyFlags};

pub trait TyBuilder: Copy + Clone + Debug + Eq + Hash + Sized {
    /// Examples: `&'a TyNode<Self>`, `Rc<TyNode<Self>>`.
    type TyHandle: AsRef<TyNode<Self>> + Clone + Debug + Eq + Hash;

    /// Examples: `string_cache::DefaultAtom`, `&'a str`, `Rc<str>`.
    type IdentHandle: AsRef<str> + Clone + Debug + Eq + Hash;

    /// Lists could be `Vec<T>` for Heap, and `&'a [T]` for Arena.
    /// We don't use a GAT like `List<T>` as that makes lifetime handling more complex.
    type TyListHandle: Deref<Target = [Ty<Self>]> + Clone + Debug + Eq + Hash;
    type IdentListHandle: Deref<Target = [Ident<Self>]> + Clone + Debug + Eq + Hash;
    type FieldListHandle: Deref<Target = [(Ident<Self>, Ty<Self>)]> + Clone + Debug + Eq + Hash;

    /// Internal: Allocate a new type with the given kind.
    /// Call instead: `TypeKind(...).alloc(builder)`.
    fn alloc(&self, node: TyNode<Self>) -> Self::TyHandle;

    fn alloc_ident(&self, ident: impl AsRef<str>) -> Self::IdentHandle;

    fn alloc_ty_list(
        &self,
        iter: impl IntoIterator<Item = Ty<Self>, IntoIter: ExactSizeIterator>,
    ) -> Self::TyListHandle;

    fn alloc_ident_list(
        &self,
        iter: impl IntoIterator<Item = Ident<Self>, IntoIter: ExactSizeIterator>,
    ) -> Self::IdentListHandle;

    fn alloc_field_list(
        &self,
        iter: impl IntoIterator<Item = (Ident<Self>, Ty<Self>), IntoIter: ExactSizeIterator>,
    ) -> Self::FieldListHandle;
}

/// Lightweight wrapper around the builder's representation.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Ty<B: TyBuilder>(B::TyHandle);

impl<B: TyBuilder> Ty<B> {
    pub fn new(builder: &B, node: TyNode<B>) -> Self {
        Self(builder.alloc(node))
    }

    pub fn handle(&self) -> &B::TyHandle {
        &self.0
    }

    pub fn node(&self) -> &TyNode<B> {
        self.handle().as_ref()
    }
}

// Implement Copy when InternedTy is Copy (e.g., for ArenaBuilder)
impl<B: TyBuilder> Copy for Ty<B> where B::TyHandle: Copy {}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TyNode<B: TyBuilder>(TyFlags, TyKind<B>);

impl<B: TyBuilder> TyNode<B> {
    pub fn new(kind: TyKind<B>) -> Self {
        let flags = kind.compute_flags();
        Self(flags, kind)
    }

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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Ident<B: TyBuilder>(B::IdentHandle);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TyList<B: TyBuilder>(B::TyListHandle);

impl<'a, B: TyBuilder> IntoIterator for &'a TyList<B> {
    type Item = &'a Ty<B>;
    type IntoIter = core::slice::Iter<'a, Ty<B>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IdentList<B: TyBuilder>(B::IdentListHandle);

impl<'a, B: TyBuilder> IntoIterator for &'a IdentList<B> {
    type Item = &'a Ident<B>;
    type IntoIter = core::slice::Iter<'a, Ident<B>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FieldList<B: TyBuilder>(B::FieldListHandle);

impl<'a, B: TyBuilder> IntoIterator for &'a FieldList<B> {
    type Item = &'a (Ident<B>, Ty<B>);
    type IntoIter = core::slice::Iter<'a, (Ident<B>, Ty<B>)>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
