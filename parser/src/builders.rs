//! Storage strategies shipped with the crate.
//!
//! Only [`ArenaBuilder`] lives here. It is the one the parser itself allocates
//! into, so it has to be part of the crate rather than a fixture: a caller
//! cannot use [`parse`](crate::parse) without naming the builder it built into.
//!
//! Other strategies — a reference-counted heap builder, an interning builder —
//! are deliberately *not* shipped. Nothing in the crate needs one, and
//! [`TreeBuilder`] is a public trait precisely so a consumer can write its own;
//! `parser/tests/common/builders.rs` has a reference-counted one that exists to
//! prove the passes stay generic.

use core::hash::{Hash, Hasher};
use core::{fmt, ptr};

use bumpalo::Bump;

use crate::{Tree, TreeBuilder, TreeDescriptor, TreeNode};

/// Allocates every node, list and string into a bump arena.
///
/// The whole tree is freed at once when the arena is dropped, which is the right
/// shape for a parser: nothing is ever freed individually, and the AST dies as a
/// unit once evaluation is done.
///
/// The handle is a plain `&'arena TreeNode<…>`, which makes `Tree<ArenaBuilder,
/// D>` [`Copy`] — a subtree can be passed around freely without a clone or a
/// refcount bump.
///
/// The builder itself is a `&Bump` wrapper and so is cheap to copy; hand it
/// around by value.
#[derive(Clone, Copy)]
pub struct ArenaBuilder<'arena> {
    arena: &'arena Bump,
}

impl<'arena> ArenaBuilder<'arena> {
    pub fn new(arena: &'arena Bump) -> Self {
        Self { arena }
    }

    /// The arena this builder allocates from.
    pub fn arena(&self) -> &'arena Bump {
        self.arena
    }
}

impl fmt::Debug for ArenaBuilder<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ArenaBuilder")
    }
}

// `PartialEq`/`Eq`/`Hash` exist only to satisfy `TreeBuilder`'s supertraits,
// which are there so a generic `Kind<B>` can be derived. Identity is the arena
// allocated from — two builders over the same `Bump` are the same builder.

impl PartialEq for ArenaBuilder<'_> {
    fn eq(&self, other: &Self) -> bool {
        ptr::eq(self.arena, other.arena)
    }
}

impl Eq for ArenaBuilder<'_> {}

impl Hash for ArenaBuilder<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        ptr::hash(self.arena, state);
    }
}

impl<'arena> TreeBuilder for ArenaBuilder<'arena> {
    type Handle<D: TreeDescriptor> = &'arena TreeNode<Self, D>;
    type List<D: TreeDescriptor> = &'arena [Tree<Self, D>];
    type Str = &'arena str;
    type StrList = &'arena [&'arena str];
    type Bytes = &'arena [u8];

    fn alloc<D: TreeDescriptor>(&self, node: TreeNode<Self, D>) -> Self::Handle<D> {
        self.arena.alloc(node)
    }

    fn alloc_list<D: TreeDescriptor>(
        &self,
        items: impl IntoIterator<Item = Tree<Self, D>, IntoIter: ExactSizeIterator>,
    ) -> Self::List<D> {
        self.arena.alloc_slice_fill_iter(items)
    }

    fn alloc_str(&self, s: &str) -> Self::Str {
        self.arena.alloc_str(s)
    }

    fn alloc_str_list(
        &self,
        items: impl IntoIterator<Item = Self::Str, IntoIter: ExactSizeIterator>,
    ) -> Self::StrList {
        self.arena.alloc_slice_fill_iter(items)
    }

    fn alloc_bytes(&self, bytes: &[u8]) -> Self::Bytes {
        self.arena.alloc_slice_copy(bytes)
    }
}
