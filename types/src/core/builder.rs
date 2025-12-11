use core::{fmt::Debug, hash::Hash, ops::Deref};

use super::ty::{Ident, Ty, TyNode};

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
