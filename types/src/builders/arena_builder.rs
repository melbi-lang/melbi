use crate::core::{Ident, Ty, TyBuilder, TyKind, TyNode};
use bumpalo::Bump;
use core::cell::RefCell;
use core::hash::Hash;
use core::{fmt, hash};
use hashbrown::{Equivalent, HashSet};

type StringSet<'arena> = HashSet<&'arena str, hashbrown::DefaultHashBuilder, &'arena Bump>;
type TypeSet<'arena> =
    HashSet<&'arena TyNode<ArenaBuilder<'arena>>, hashbrown::DefaultHashBuilder, &'arena Bump>;

// --- Equivalent impl for heterogeneous lookup: lookup by TyKind, store &TyNode ---

impl<'arena> Equivalent<&'arena TyNode<ArenaBuilder<'arena>>> for TyKind<ArenaBuilder<'arena>> {
    fn equivalent(&self, key: &&'arena TyNode<ArenaBuilder<'arena>>) -> bool {
        self == key.kind()
    }
}

/// Interner that uses arena allocation.
///
/// Types and identifiers are allocated in a `Bump` arena and interned
/// (deduplicated), so structurally identical values share the same pointer.
/// This enables O(1) pointer-based equality checks via `==`.
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
/// // Types are interned - same structure, same pointer
/// let int1 = TyKind::Scalar(Scalar::Int).alloc(&builder);
/// let int2 = TyKind::Scalar(Scalar::Int).alloc(&builder);
/// assert_eq!(int1, int2);  // O(1) pointer equality
///
/// // Identifiers are also interned
/// let id1 = Ident::new(&builder, "foo");
/// let id2 = Ident::new(&builder, "foo");
/// assert_eq!(id1, id2);
/// ```
#[derive(Copy, Clone)]
pub struct ArenaBuilder<'arena> {
    arena: &'arena Bump,
    interned_strs: &'arena RefCell<StringSet<'arena>>,
    interned_types: &'arena RefCell<TypeSet<'arena>>,
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
        let interned_strs = arena.alloc(RefCell::new(StringSet::with_capacity_in(256, arena)));
        let interned_types = arena.alloc(RefCell::new(TypeSet::with_capacity_in(256, arena)));
        Self {
            arena,
            interned_strs,
            interned_types,
        }
    }
}

impl<'arena> TyBuilder for ArenaBuilder<'arena> {
    type TyHandle = &'arena TyNode<Self>;
    type IdentHandle = &'arena str;
    type TyListHandle = &'arena [Ty<Self>];
    type IdentListHandle = &'arena [Ident<Self>];
    type FieldListHandle = &'arena [(Ident<Self>, Ty<Self>)];

    fn alloc(&self, kind: TyKind<Self>) -> Self::TyHandle {
        let mut set = self.interned_types.borrow_mut();

        // Look up by TyKind using Equivalent trait
        if let Some(&existing) = set.get(&kind) {
            return existing;
        }

        // Allocate and insert
        let allocated = self.arena.alloc(TyNode::new(kind));
        set.insert(allocated);
        allocated
    }

    fn alloc_ident(&self, ident: impl AsRef<str>) -> Self::IdentHandle {
        // TODO: intern short strings inline in `Self::IdentHandle`.
        let s = ident.as_ref();
        let mut set = self.interned_strs.borrow_mut();
        if let Some(&interned) = set.get(s) {
            return interned;
        }
        let allocated = self.arena.alloc_str(s);
        set.insert(allocated);
        allocated
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

    fn resolve_ty_node(ty: &Ty<Self>) -> &TyNode<Self> {
        ty.node()
    }

    fn ty_eq(a: &Ty<Self>, b: &Ty<Self>) -> bool {
        core::ptr::eq(a.handle(), b.handle())
    }

    fn ty_hash<H: hash::Hasher>(ty: &Ty<Self>, state: &mut H) {
        (ty.handle() as *const TyNode<Self>).hash(state)
    }

    fn ident_eq(a: &Ident<Self>, b: &Ident<Self>) -> bool {
        core::ptr::eq(a.as_str().as_ptr(), b.as_str().as_ptr())
    }

    fn ident_hash<H: hash::Hasher>(ident: &Ident<Self>, state: &mut H) {
        ident.as_str().as_ptr().hash(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Scalar;

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

    #[test]
    fn test_type_interning_equality() {
        let arena = Bump::new();
        let builder = ArenaBuilder::new(&arena);

        // Same scalar type -> same pointer -> equal
        let int1 = TyKind::Scalar(Scalar::Int).alloc(&builder);
        let int2 = TyKind::Scalar(Scalar::Int).alloc(&builder);
        assert_eq!(int1, int2);

        // Different scalar types -> different pointers -> not equal
        let float_ty = TyKind::Scalar(Scalar::Float).alloc(&builder);
        assert_ne!(int1, float_ty);
    }

    #[test]
    fn test_nested_type_interning() {
        let arena = Bump::new();
        let builder = ArenaBuilder::new(&arena);

        let int_ty = TyKind::Scalar(Scalar::Int).alloc(&builder);

        // Array types with same element type -> same pointer -> equal
        let arr1 = TyKind::Array(int_ty).alloc(&builder);
        let arr2 = TyKind::Array(int_ty).alloc(&builder);
        assert_eq!(arr1, arr2);

        // Map types with same key/value types -> same pointer -> equal
        let str_ty = TyKind::Scalar(Scalar::Str).alloc(&builder);
        let map1 = TyKind::Map(str_ty, int_ty).alloc(&builder);
        let map2 = TyKind::Map(str_ty, int_ty).alloc(&builder);
        assert_eq!(map1, map2);

        // Different element types -> different pointers -> not equal
        let float_ty = TyKind::Scalar(Scalar::Float).alloc(&builder);
        let arr_float = TyKind::Array(float_ty).alloc(&builder);
        assert_ne!(arr1, arr_float);
    }

    #[test]
    fn test_type_interning_hash() {
        use core::hash::{BuildHasher, Hash, Hasher};

        let arena = Bump::new();
        let builder = ArenaBuilder::new(&arena);

        let int1 = TyKind::Scalar(Scalar::Int).alloc(&builder);
        let int2 = TyKind::Scalar(Scalar::Int).alloc(&builder);

        let hash_builder = hashbrown::DefaultHashBuilder::default();

        let hash1 = {
            let mut hasher = hash_builder.build_hasher();
            int1.hash(&mut hasher);
            hasher.finish()
        };

        let hash2 = {
            let mut hasher = hash_builder.build_hasher();
            int2.hash(&mut hasher);
            hasher.finish()
        };

        // Same interned type -> same hash
        assert_eq!(hash1, hash2);
    }
}
