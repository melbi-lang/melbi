use crate::core::{Ident, Ty, TyBuilder, TyNode};
use bumpalo::Bump;
use core::cell::RefCell;
use core::{fmt, hash};
use hashbrown::{DefaultHashBuilder, HashSet};

/// An interned string reference with pointer-based equality.
///
/// Two `InternedStr` values are equal if and only if they point to the
/// same memory location. This is guaranteed when using `ArenaBuilder`
/// since identical strings are deduplicated during interning.
#[derive(Clone, Copy)]
pub struct InternedStr<'arena>(&'arena str);

impl<'arena> InternedStr<'arena> {
    /// Returns the string slice.
    pub fn as_str(&self) -> &'arena str {
        self.0
    }
}

impl<'arena> AsRef<str> for InternedStr<'arena> {
    fn as_ref(&self) -> &str {
        self.0
    }
}

impl<'arena> fmt::Debug for InternedStr<'arena> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.0, f)
    }
}

impl<'arena> fmt::Display for InternedStr<'arena> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.0, f)
    }
}

impl<'arena> PartialEq for InternedStr<'arena> {
    fn eq(&self, other: &Self) -> bool {
        core::ptr::eq(self.0.as_ptr(), other.0.as_ptr())
    }
}

impl<'arena> Eq for InternedStr<'arena> {}

impl<'arena> hash::Hash for InternedStr<'arena> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.0.as_ptr().hash(state)
    }
}

type StringSet<'arena> = HashSet<&'arena str, DefaultHashBuilder, &'arena Bump>;

/// Interner that uses arena allocation.
///
/// Types are allocated in a `Bump` arena. Identifiers are interned
/// (deduplicated), so the same string always returns the same pointer,
/// enabling fast pointer-based equality checks.
///
/// Following Chalk's design, we compute type flags during interning and
/// wrap the TyKind in TyData.
///
/// # Example
///
/// ```
/// use melbi_types::{ArenaBuilder, TyBuilder, TyKind, Scalar, Ident};
/// use bumpalo::Bump;
///
/// let arena = Bump::new();
/// let builder = ArenaBuilder::new(&arena);
///
/// let int_ty = TyKind::Scalar(Scalar::Int).alloc(&builder);
/// let arr_ty = TyKind::Array(int_ty).alloc(&builder);
///
/// // Interned identifiers with same content are equal (pointer equality)
/// let id1 = Ident::new(&builder, "foo");
/// let id2 = Ident::new(&builder, "foo");
/// assert_eq!(id1, id2);
/// ```
#[derive(Copy, Clone)]
pub struct ArenaBuilder<'arena> {
    arena: &'arena Bump,
    interned_strs: &'arena RefCell<StringSet<'arena>>,
}

impl<'arena> fmt::Debug for ArenaBuilder<'arena> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ArenaBuilder")
            .field("arena", &(self.arena as *const Bump))
            .finish_non_exhaustive()
    }
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
        let interned_strs = arena.alloc(RefCell::new(HashSet::with_capacity_in(256, arena)));
        Self {
            arena,
            interned_strs,
        }
    }
}

impl<'arena> TyBuilder for ArenaBuilder<'arena> {
    type TyHandle = &'arena TyNode<Self>;
    type IdentHandle = InternedStr<'arena>;
    type TyListHandle = &'arena [Ty<Self>];
    type IdentListHandle = &'arena [Ident<Self>];
    type FieldListHandle = &'arena [(Ident<Self>, Ty<Self>)];

    fn alloc(&self, node: TyNode<Self>) -> Self::TyHandle {
        self.arena.alloc(node)
    }

    fn alloc_ident(&self, ident: impl AsRef<str>) -> Self::IdentHandle {
        let s = ident.as_ref();
        let mut set = self.interned_strs.borrow_mut();
        if let Some(&interned) = set.get(s) {
            return InternedStr(interned);
        }
        let allocated = self.arena.alloc_str(s);
        set.insert(allocated);
        InternedStr(allocated)
    }

    fn alloc_ty_list(
        &self,
        iter: impl IntoIterator<Item = Ty<Self>, IntoIter: ExactSizeIterator>,
    ) -> Self::TyListHandle {
        self.arena.alloc_slice_fill_iter(iter)
    }

    fn alloc_ident_list(
        &self,
        iter: impl IntoIterator<Item = Ident<Self>, IntoIter: ExactSizeIterator>,
    ) -> Self::IdentListHandle {
        self.arena.alloc_slice_fill_iter(iter)
    }

    fn alloc_field_list(
        &self,
        iter: impl IntoIterator<Item = (Ident<Self>, Ty<Self>), IntoIter: ExactSizeIterator>,
    ) -> Self::FieldListHandle {
        self.arena.alloc_slice_fill_iter(iter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Ident;

    #[test]
    fn test_interned_str_equality() {
        let arena = Bump::new();
        let builder = ArenaBuilder::new(&arena);

        let id1 = Ident::new(&builder, "foo");
        let id2 = Ident::new(&builder, "foo");
        let id3 = Ident::new(&builder, "bar");

        // Same string content -> same pointer -> equal
        assert_eq!(id1, id2);

        // Different string content -> different pointer -> not equal
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_interned_str_hash() {
        use core::hash::{BuildHasher, Hash, Hasher};

        let arena = Bump::new();
        let builder = ArenaBuilder::new(&arena);

        let id1 = Ident::new(&builder, "foo");
        let id2 = Ident::new(&builder, "foo");

        // Use the same hasher builder for both to ensure consistent hashing
        let hash_builder = hashbrown::DefaultHashBuilder::default();

        let hash1 = {
            let mut hasher = hash_builder.build_hasher();
            id1.hash(&mut hasher);
            hasher.finish()
        };

        let hash2 = {
            let mut hasher = hash_builder.build_hasher();
            id2.hash(&mut hasher);
            hasher.finish()
        };

        // Same interned string -> same hash
        assert_eq!(hash1, hash2);
    }
}
