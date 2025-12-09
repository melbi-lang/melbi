use core::{
    fmt::{Debug, Display},
    hash::Hash,
    ops::Deref,
};

use crate::{kind::TyKind, traits::TyFlags};

pub trait TyBuilder: Copy + Clone + Eq + Sized + Debug + Hash {
    /// Top-level wrapper type over the allocated/interned value (`Ty<Self>`).
    type Ty: Clone + Debug + Eq + Hash;

    /// Examples: `&'a TyNode<Self>`, `Rc<TyNode<Self>>`.
    type TyHandle: AsRef<TyNode<Self>> + Clone + Debug + Eq + Hash;

    /// Examples: `&'a str`, `Rc<str>`, `ecow::EcoString`.
    type Str: AsRef<str> + Clone + Debug + Eq + Hash + Display;

    /// "List<T>" will be `Vec<T>` for Heap, and `&'a [T]` for Arena.
    /// We add bounds so the AST can derive Debug/Eq automatically.
    type List<T>: Deref<Target = [T]> + Clone + Debug + Eq + Hash
    where
        T: Debug + PartialEq + Eq + Clone + Hash;

    /// Internal: Allocate a new type with the given kind.
    /// Call instead: `TypeKind(...).alloc(builder)`.
    fn alloc(&self, node: TyNode<Self>) -> Self::TyHandle;

    fn alloc_str(&self, s: impl AsRef<str>) -> Self::Str;

    // Generic Allocator for Lists
    fn alloc_list<T>(&self, iter: impl IntoIterator<Item = T>) -> Self::List<T>
    where
        T: Debug + PartialEq + Eq + Clone + Hash;
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
