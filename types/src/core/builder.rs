use core::{fmt::Debug, hash, hash::Hash, ops::Deref};

use crate::{Ident, Ty};

use super::{kind::TyKind, ty::TyNode};

pub trait TyBuilder: Clone + Debug + Eq + Hash + Sized {
    // Relevant concrete types:
    // * Ty<Self>
    // * Ident<Self>
    // * TyList<Self>
    // * IdentList<Self>
    // * FieldList<Self>

    /// Examples: `&'a TyNode<Self>`, `Rc<TyNode<Self>>`.
    type TyHandle: AsRef<TyNode<Self>> + Clone + Debug;

    /// Examples: `string_cache::DefaultAtom`, `&'a str`, `Rc<str>`.
    type IdentHandle: AsRef<str> + Clone + Debug;

    /// Lists could be `Vec<T>` for Heap, and `&'a [T]` for Arena.
    /// We don't use a GAT like `List<T>` as that makes lifetime handling more complex.
    type TyListHandle: Deref<Target = [Ty<Self>]> + Clone + Debug + Eq + Hash;
    type IdentListHandle: Deref<Target = [Ident<Self>]> + Clone + Debug + Eq + Hash;
    type FieldListHandle: Deref<Target = [(Ident<Self>, Ty<Self>)]> + Clone + Debug + Eq + Hash;

    /// Internal: Allocate a new type with the given kind.
    /// Call instead: `TypeKind(...).alloc(builder)`.
    fn alloc(&self, kind: TyKind<Self>) -> Self::TyHandle;

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

    fn resolve_ty_node(ty: &Ty<Self>) -> &TyNode<Self>;

    /// Compare two types for equality.
    /// Default: structural equality via TyNode.
    #[inline]
    fn ty_eq(a: &Ty<Self>, b: &Ty<Self>) -> bool {
        Self::resolve_ty_node(a) == Self::resolve_ty_node(b)
    }

    /// Hash a type.
    /// Default: structural hash via TyNode.
    #[inline]
    fn ty_hash<H: hash::Hasher>(ty: &Ty<Self>, state: &mut H) {
        Self::resolve_ty_node(ty).hash(state)
    }

    /// Compare two identifiers for equality.
    /// Default: structural equality via string content.
    #[inline]
    fn ident_eq(a: &Ident<Self>, b: &Ident<Self>) -> bool {
        *a == *b
    }

    /// Hash an identifier.
    /// Default: structural hash via string content.
    #[inline]
    fn ident_hash<H: hash::Hasher>(ident: &Ident<Self>, state: &mut H) {
        ident.hash(state)
    }
}
