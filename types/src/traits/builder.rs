use core::{fmt::Debug, hash::Hash, ops::Deref};

use crate::{kind::TyKind, traits::TyFlags};

pub trait TyBuilder: Copy + Clone + Debug + Eq + Hash + Sized {
    /// Top-level wrapper type over the allocated/interned value (`Ty<Self>`).
    type Ty: Clone + Debug + Eq + Hash;

    /// Examples: `&'a TyNode<Self>`, `Rc<TyNode<Self>>`.
    type TyHandle: AsRef<TyNode<Self>> + Clone + Debug + Eq + Hash;

    /// Examples: `string_cache::DefaultAtom`, `&'a str`, `Rc<str>`, `ecow::EcoString`.
    type Ident: AsRef<str> + Clone + Debug + Eq + Hash;

    /// Lists could be `Vec<T>` for Heap, and `&'a [T]` for Arena.
    /// We can't use a GAT like `List<T>` as that makes lifetime handling more complex.
    type TyList: Deref<Target = [Self::Ty]> + Clone + Debug + Eq + Hash;
    type IdentList: Deref<Target = [Self::Ident]> + Clone + Debug + Eq + Hash;
    type FieldList: Deref<Target = [(Self::Ident, Self::Ty)]> + Clone + Debug + Eq + Hash;

    /// Internal: Allocate a new type with the given kind.
    /// Call instead: `TypeKind(...).alloc(builder)`.
    fn alloc(&self, node: TyNode<Self>) -> Self::TyHandle;

    fn alloc_ty_list(&self, iter: impl IntoIterator<Item = Self::Ty>) -> Self::TyList;

    fn alloc_ident(&self, ident: impl AsRef<str>) -> Self::Ident;

    fn alloc_ident_list(&self, iter: impl IntoIterator<Item = impl AsRef<str>>) -> Self::IdentList;

    fn alloc_field_list(
        &self,
        iter: impl IntoIterator<Item = (Self::Ident, Self::Ty)>,
    ) -> Self::FieldList;
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

impl<B: TyBuilder<Ty = Ty<B>>> TyNode<B> {
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
