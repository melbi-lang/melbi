use crate::traits::{Ty, TyBuilder, TyNode};
use bumpalo::Bump;
use core::{fmt::Debug, hash::Hash};

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

impl<'arena> core::hash::Hash for ArenaBuilder<'arena> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
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
    type Str = &'arena str;
    type List<T>
        = &'arena [T]
    where
        T: Debug + PartialEq + Eq + Clone + Hash;

    fn alloc(&self, node: TyNode<Self>) -> Self::TyHandle {
        self.arena.alloc(node)
    }

    fn alloc_str(&self, s: impl AsRef<str>) -> Self::Str {
        self.arena.alloc_str(s.as_ref())
    }

    fn alloc_list<T>(&self, iter: impl IntoIterator<Item = T>) -> Self::List<T>
    where
        T: Debug + PartialEq + Eq + Clone + Hash,
    {
        self.arena.alloc_slice_fill_iter(iter)
    }
}
