use crate::core::{FieldList, Ident, IdentList, Ty, TyBuilder, TyKind, TyList, TyNode};
use alloc::rc::Rc;
use alloc::vec::Vec;
use core::hash;
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
/// ```
/// use melbi_types::{BoxBuilder, TyBuilder, TyKind, Scalar};
///
/// let builder = BoxBuilder::new();
/// let int_ty = TyKind::Scalar(Scalar::Int).alloc(&builder);
/// let arr_ty = TyKind::Array(int_ty).alloc(&builder);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BoxBuilder;

impl BoxBuilder {
    /// Create a new box builder.
    pub fn new() -> Self {
        Self
    }
}

impl Default for BoxBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TyBuilder for BoxBuilder {
    type Ty = Ty<Self>;
    type Ident = Ident<Self>;
    type TyList = TyList<Self>;
    type IdentList = IdentList<Self>;
    type FieldList = FieldList<Self>;

    type TyHandle = Rc<TyNode<Self>>;
    type IdentHandle = DefaultAtom;
    type TyListHandle = Vec<Ty<Self>>;
    type IdentListHandle = Vec<Ident<Self>>;
    type FieldListHandle = Vec<(Ident<Self>, Ty<Self>)>;

    fn alloc(&self, kind: TyKind<Self>) -> Self::TyHandle {
        Rc::new(TyNode::new(kind))
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

    fn resolve_ty_node(ty: &Self::Ty) -> &TyNode<Self> {
        ty.node()
    }
}

// --- Ty<BoxBuilder> impls: structural equality via Rc ---

impl PartialEq for Ty<BoxBuilder> {
    fn eq(&self, other: &Self) -> bool {
        self.handle() == other.handle()
    }
}

impl Eq for Ty<BoxBuilder> {}

impl hash::Hash for Ty<BoxBuilder> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.handle().hash(state)
    }
}

// --- TyNode<BoxBuilder> impl: structural hash ---

impl hash::Hash for TyNode<BoxBuilder> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.flags().hash(state);
        self.kind().hash(state);
    }
}
