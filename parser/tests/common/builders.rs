//! A reference-counted storage strategy, defined outside the crate.
//!
//! The arena builder the parser uses ships with the crate
//! ([`melbi_parser::ArenaBuilder`]). This one deliberately does not: it exists
//! to prove that [`TreeBuilder`] is implementable from the outside, and that
//! every pass stays generic over storage — `visit_rebuilds_from_an_arena_into_
//! the_heap` rebuilds an arena tree into this one with no lifetime bookkeeping.

use std::rc::Rc;

use melbi_parser::{Tree, TreeBuilder, TreeDescriptor, TreeNode};

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
