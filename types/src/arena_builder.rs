use crate::traits::{Ty, TyBuilder, TyNode};
use alloc::vec::Vec;
use bumpalo::Bump;
use core::{fmt::Debug, hash};

/// Interner that uses arena allocation.
///
/// Types are allocated in a `Bump` arena. For now, we don't do actual
/// interning (deduplication), just allocation. This keeps the implementation
/// simple for the POC.
///
/// Following Chalk's design, we compute type flags during interning and
/// wrap the TyKind in TyData.
///
/// # Example
///
/// ```
/// use melbi_types::{TypeBuilder, ArenaBuilder, Scalar, TypeKind};
/// use bumpalo::Bump;
///
/// let arena = Bump::new();
/// let builder = ArenaBuilder::new(&arena);
///
/// let int_ty = TypeKind::Scalar(Scalar::Int).intern(builder);
/// let arr_ty = TypeKind::Array(int_ty).intern(builder);
/// ```
#[derive(Copy, Clone, Debug)]
pub struct ArenaBuilder<'arena> {
    arena: &'arena Bump,
}

// Manual implementations since Bump doesn't implement PartialEq/Eq/Hash
// We use pointer equality - two builders are equal if they point to the same arena
impl<'arena> PartialEq for ArenaBuilder<'arena> {
    fn eq(&self, other: &Self) -> bool {
        core::ptr::eq(self.arena, other.arena)
    }
}

impl<'arena> Eq for ArenaBuilder<'arena> {}

impl<'arena> hash::Hash for ArenaBuilder<'arena> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        core::ptr::hash(self.arena, state)
    }
}

impl<'arena> ArenaBuilder<'arena> {
    /// Create a new arena builder.
    pub fn new(arena: &'arena Bump) -> Self {
        Self { arena }
    }
}

impl<'arena> TyBuilder for ArenaBuilder<'arena> {
    type Ty = Ty<Self>;
    type TyHandle = &'arena TyNode<Self>;
    type Ident = &'arena str;
    type TyList = &'arena [Self::Ty];
    type IdentList = &'arena [Self::Ident];
    type FieldList = &'arena [(Self::Ident, Self::Ty)];

    fn alloc(&self, node: TyNode<Self>) -> Self::TyHandle {
        self.arena.alloc(node)
    }

    fn alloc_ty_list(&self, iter: impl IntoIterator<Item = Self::Ty>) -> Self::TyList {
        let vec = iter.into_iter().collect::<Vec<_>>();
        self.arena.alloc_slice_copy(&vec)
    }

    fn alloc_ident(&self, ident: impl AsRef<str>) -> Self::Ident {
        self.arena.alloc_str(ident.as_ref())
    }

    fn alloc_ident_list(&self, iter: impl IntoIterator<Item = impl AsRef<str>>) -> Self::IdentList {
        let vec = iter
            .into_iter()
            .map(|ident| self.alloc_ident(ident))
            .collect::<Vec<_>>();
        self.arena.alloc_slice_copy(&vec)
    }

    fn alloc_field_list(
        &self,
        iter: impl IntoIterator<Item = (Self::Ident, Self::Ty)>,
    ) -> Self::FieldList {
        let vec = iter.into_iter().collect::<Vec<_>>();
        self.arena.alloc_slice_copy(&vec)
    }
}
