//! A bump-arena builder.
//!
//! The point of interest here is that `Tree<ArenaBuilder<'_>, Sample>` is
//! `Copy`, and that rebuilding from one arena into another is the same generic
//! pass used by the heap builder — neither arena's lifetime appears anywhere in
//! it.

use core::fmt;
use core::ptr;

use bumpalo::Bump;

use crate::test_utils::{Expr, Rebuild, SumLiterals, sample, sample_with_literals};
use crate::{Span, Tree, TreeBuilder, TreeDescriptor, TreeNode};

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

impl core::hash::Hash for ArenaBuilder<'_> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
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

    fn alloc_str(&self, s: &str) -> Self::Str {
        self.arena.alloc_str(s)
    }

    fn alloc_list<D: TreeDescriptor>(
        &self,
        items: impl IntoIterator<Item = Tree<Self, D>, IntoIter: ExactSizeIterator>,
    ) -> Self::List<D> {
        self.arena.alloc_slice_fill_iter(items)
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

#[test]
fn builds_and_inspects_a_tree() {
    let arena = Bump::new();
    let builder = ArenaBuilder::new(&arena);
    let root = sample(&builder);

    assert_eq!(*root.data(), Span(0, 12));

    let Expr::Sum(items) = root.kind() else {
        panic!("expected Sum, got {:?}", root.kind());
    };
    assert_eq!(items.len(), 2);
    assert_eq!(*items[0].kind(), Expr::Lit(1));
}

#[test]
fn trees_are_copy() {
    let arena = Bump::new();
    let builder = ArenaBuilder::new(&arena);
    let root = sample(&builder);

    // No clone: an arena handle is a plain reference, so `Tree` is `Copy`.
    let alias = root;
    assert_eq!(*alias.data(), *root.data());
}

#[test]
fn visit_threads_mutable_state() {
    let arena = Bump::new();
    let builder = ArenaBuilder::new(&arena);
    let root = sample(&builder);

    let mut ctx = SumLiterals::default();
    root.visit(&mut ctx);

    assert_eq!(ctx.total, 4);
    assert_eq!(ctx.nodes_seen, 7);
}

#[test]
fn visit_rebuilds_from_one_arena_into_another() {
    let source_arena = Bump::new();
    let source = ArenaBuilder::new(&source_arena);
    let root = sample(&source);

    let dest_arena = Bump::new();
    let mut ctx = Rebuild {
        out: ArenaBuilder::new(&dest_arena),
    };
    let rebuilt = root.visit(&mut ctx);

    // Structural equality across two unrelated arenas. Note that neither
    // lifetime was named to get here.
    assert_eq!(root, rebuilt);
    assert!(!ptr::eq(root.node(), rebuilt.node()));
}

#[test]
fn visit_rebuilds_from_an_arena_into_the_heap() {
    use crate::heap_builder_test::HeapBuilder;

    let source_arena = Bump::new();
    let source = ArenaBuilder::new(&source_arena);
    let root = sample(&source);

    let mut ctx = Rebuild { out: HeapBuilder };
    let rebuilt = root.visit(&mut ctx);

    // The rebuilt tree outlives nothing in particular: it is reference counted,
    // while the source is arena allocated.
    let mut ctx = SumLiterals::default();
    rebuilt.visit(&mut ctx);
    assert_eq!(ctx.total, 4);
    assert_eq!(ctx.nodes_seen, 7);
}

#[test]
fn covers_bytes_and_format_strings() {
    let arena = Bump::new();
    let builder = ArenaBuilder::new(&arena);
    let root = sample_with_literals(&builder);

    let mut ctx = SumLiterals::default();
    root.visit(&mut ctx);
    assert_eq!(ctx.total, 7);

    let dest_arena = Bump::new();
    let mut ctx = Rebuild {
        out: ArenaBuilder::new(&dest_arena),
    };
    assert_eq!(root, root.visit(&mut ctx));
}
