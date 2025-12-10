use crate::traits::{Ty, TyBuilder, TyNode};
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
    type Ty = self::Ty<Self>;
    type TyHandle = Rc<TyNode<Self>>;
    type Ident = DefaultAtom;
    type TyList = Vec<Self::Ty>;
    type IdentList = Vec<Self::Ident>;
    type FieldList = Vec<(Self::Ident, Self::Ty)>;

    fn alloc(&self, node: TyNode<Self>) -> Self::TyHandle {
        Rc::new(node)
    }

    fn alloc_ident(&self, s: impl AsRef<str>) -> Self::Ident {
        DefaultAtom::from(s.as_ref())
    }

    fn alloc_ty_list(&self, iter: impl IntoIterator<Item = Self::Ty>) -> Self::TyList {
        iter.into_iter().collect()
    }

    fn alloc_ident_list(&self, iter: impl IntoIterator<Item = impl AsRef<str>>) -> Self::IdentList {
        iter.into_iter().map(|s| self.alloc_ident(s)).collect()
    }

    fn alloc_field_list(
        &self,
        iter: impl IntoIterator<Item = (Self::Ident, Self::Ty)>,
    ) -> Self::FieldList {
        iter.into_iter().collect()
    }
}
