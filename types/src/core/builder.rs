use core::{fmt::Debug, hash, hash::Hash, ops::Deref};

use super::{kind::TyKind, ty::TyNode};

pub trait TyBuilder: Clone + Debug + Eq + Hash + Sized {
    type Ty: Clone + Debug + Eq + Hash;
    type Ident: Deref<Target = str> + Clone + Debug + Eq + Hash;
    type TyList: Deref<Target = [Self::Ty]> + Clone + Debug + Eq + Hash;
    type IdentList: Deref<Target = [Self::Ident]> + Clone + Debug + Eq + Hash;
    type FieldList: Deref<Target = [(Self::Ident, Self::Ty)]> + Clone + Debug + Eq + Hash;

    /// Examples: `&'a TyNode<Self>`, `Rc<TyNode<Self>>`.
    type TyHandle: AsRef<TyNode<Self>> + Clone + Debug;

    /// Examples: `string_cache::DefaultAtom`, `&'a str`, `Rc<str>`.
    type IdentHandle: AsRef<str> + Clone + Debug;

    /// Lists could be `Vec<T>` for Heap, and `&'a [T]` for Arena.
    /// We don't use a GAT like `List<T>` as that makes lifetime handling more complex.
    type TyListHandle: Deref<Target = [Self::Ty]> + Clone + Debug;
    type IdentListHandle: Deref<Target = [Self::Ident]> + Clone + Debug;
    type FieldListHandle: Deref<Target = [(Self::Ident, Self::Ty)]> + Clone + Debug;

    /// Internal: Allocate a new type with the given kind.
    /// Call instead: `TypeKind(...).alloc(builder)`.
    fn alloc(&self, kind: TyKind<Self>) -> Self::TyHandle;

    fn alloc_ident(&self, ident: impl AsRef<str>) -> Self::IdentHandle;

    fn alloc_ty_list(
        &self,
        iter: impl IntoIterator<Item = Self::Ty, IntoIter: ExactSizeIterator>,
    ) -> Self::TyListHandle;

    fn alloc_ident_list(
        &self,
        iter: impl IntoIterator<Item = Self::Ident, IntoIter: ExactSizeIterator>,
    ) -> Self::IdentListHandle;

    fn alloc_field_list(
        &self,
        iter: impl IntoIterator<Item = (Self::Ident, Self::Ty), IntoIter: ExactSizeIterator>,
    ) -> Self::FieldListHandle;

    fn resolve_ty_node(ty: &Self::Ty) -> &TyNode<Self>;

    /// Compare two types for equality.
    /// Default: structural equality via TyNode.
    fn ty_eq(a: &Self::Ty, b: &Self::Ty) -> bool {
        Self::resolve_ty_node(a) == Self::resolve_ty_node(b)
    }

    /// Hash a type.
    /// Default: structural hash via TyNode.
    fn ty_hash<H: hash::Hasher>(ty: &Self::Ty, state: &mut H) {
        Self::resolve_ty_node(ty).hash(state)
    }

    /// Compare two identifiers for equality.
    /// Default: structural equality via string content.
    fn ident_eq(a: &Self::Ident, b: &Self::Ident) -> bool {
        *a == *b
    }
}
