use crate::traits::{Ident, Ty, TyBuilder, TyNode};
use alloc::rc::Rc;
use alloc::vec::Vec;
use core::{fmt::Debug, hash::Hash};
use string_cache::DefaultAtom;

/// Interner that uses reference counting (no deduplication).
///
/// Types are allocated with `Rc` and no interning is performed.
/// This is useful for:
/// - Testing (simpler than arena)
/// - Situations where deduplication isn't needed
/// - Comparing performance with/without interning
///
/// Following Chalk's design, we compute type flags during interning and
/// wrap the TyKind in TyData.
///
/// # Example
///
/// ```ignore
/// let builder = BoxBuilder::new();
/// let int_ty = TyKind::Scalar(Scalar::Int).alloc(builder);
/// let arr_ty = TyKind::Array(int_ty).alloc(builder);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BoxBuilder;

impl BoxBuilder {
    /// Create a new box builder.
    pub fn new() -> Self {
        Self
    }
}

impl TyBuilder for BoxBuilder {
    type TyHandle = Rc<TyNode<Self>>;
    type IdentHandle = DefaultAtom;
    type TyListHandle = Vec<Ty<Self>>;
    type IdentListHandle = Vec<Ident<Self>>;
    type FieldListHandle = Vec<(Ident<Self>, Ty<Self>)>;

    fn alloc(&self, node: TyNode<Self>) -> Self::TyHandle {
        Rc::new(node)
    }

    fn alloc_ident(&self, ident: impl AsRef<str>) -> Self::IdentHandle {
        DefaultAtom::from(ident.as_ref())
    }

    fn alloc_ty_list(
        &self,
        iter: impl IntoIterator<Item = Ty<Self>, IntoIter: ExactSizeIterator>,
    ) -> Self::TyListHandle {
        iter.into_iter().collect()
    }

    fn alloc_ident_list(
        &self,
        iter: impl IntoIterator<Item = Ident<Self>, IntoIter: ExactSizeIterator>,
    ) -> Self::IdentListHandle {
        iter.into_iter().collect()
    }

    fn alloc_field_list(
        &self,
        iter: impl IntoIterator<Item = (Ident<Self>, Ty<Self>), IntoIter: ExactSizeIterator>,
    ) -> Self::FieldListHandle {
        iter.into_iter().collect()
    }
}
