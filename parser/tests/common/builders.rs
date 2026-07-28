//! The two storage strategies exercised by the tests.
//!
//! Both live here rather than beside their own tests because
//! `visit_rebuilds_from_an_arena_into_the_heap` needs both at once, and separate
//! integration-test binaries cannot import from each other.

use std::fmt;
use std::ptr;
use std::rc::Rc;

use bumpalo::Bump;

use melbi_parser::{Tree, TreeBuilder, TreeDescriptor, TreeNode};

// --- A bump-arena builder ----------------------------------------------------

/// The point of interest is that `Tree<ArenaBuilder<'_>, D>` is `Copy`, because
/// the handle is a plain reference.
#[derive(Clone, Copy)]
pub struct ArenaBuilder<'arena> {
    arena: &'arena Bump,
}

impl<'arena> ArenaBuilder<'arena> {
    pub fn new(arena: &'arena Bump) -> Self {
        Self { arena }
    }
}

impl fmt::Debug for ArenaBuilder<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ArenaBuilder")
    }
}

impl PartialEq for ArenaBuilder<'_> {
    fn eq(&self, other: &Self) -> bool {
        // Two builders are the same builder when they allocate from the same
        // arena.
        ptr::eq(self.arena, other.arena)
    }
}

impl Eq for ArenaBuilder<'_> {}

impl std::hash::Hash for ArenaBuilder<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Consistent with `PartialEq` above: identity is the arena it allocates
        // from.
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

// --- A reference-counted heap builder ----------------------------------------

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct HeapBuilder;

impl TreeBuilder for HeapBuilder {
    type Handle<D: TreeDescriptor> = Rc<TreeNode<Self, D>>;
    type List<D: TreeDescriptor> = Rc<[Tree<Self, D>]>;
    type Str = Rc<str>;
    type StrList = Rc<[Rc<str>]>;
    type Bytes = Rc<[u8]>;

    fn alloc<D: TreeDescriptor>(&self, node: TreeNode<Self, D>) -> Self::Handle<D> {
        Rc::new(node)
    }

    fn alloc_list<D: TreeDescriptor>(
        &self,
        items: impl IntoIterator<Item = Tree<Self, D>, IntoIter: ExactSizeIterator>,
    ) -> Self::List<D> {
        items.into_iter().collect()
    }

    fn alloc_str(&self, s: &str) -> Self::Str {
        Rc::from(s)
    }

    fn alloc_str_list(
        &self,
        items: impl IntoIterator<Item = Self::Str, IntoIter: ExactSizeIterator>,
    ) -> Self::StrList {
        items.into_iter().collect()
    }

    fn alloc_bytes(&self, bytes: &[u8]) -> Self::Bytes {
        Rc::from(bytes)
    }
}
