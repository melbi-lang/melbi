//! Tree builder trait and the core tree types.
//!
//! # Design Philosophy
//!
//! A tree is split into two halves that are kept deliberately out of band:
//!
//! - The **navigational** half ([`TreeBuilder::TreeKind`]) — usually an enum
//!   whose variants describe the shape of the node and whose children are
//!   [`Tree<Self>`].
//! - The **associated data** half ([`TreeBuilder::TreeData`]) — carried by every
//!   node, uniform across the whole tree, and varying per compiler stage.
//!
//! Keeping them apart is what makes a traversal readable: matching on the kind
//! never has to mention the data, and reading the data never has to match on
//! the kind.
//!
//! The builder additionally abstracts the *storage strategy* (heap, arena, …).
//! Because the handle types are associated types of the builder, the allocator's
//! lifetime never appears in code that consumes the tree — a function is generic
//! over `B: TreeBuilder`, not over `'arena`. Copying a tree from one builder to
//! another is then just a traversal that allocates into a different builder,
//! with no lifetime bookkeeping at the call sites.

use core::fmt::Debug;
use core::hash::{Hash, Hasher};
use core::ops::Deref;

/// Allocation strategy and node types for one tree.
///
/// The associated types pin down both *what* a node contains ([`TreeData`] and
/// [`TreeKind`]) and *how* it is stored ([`TreeHandle`], [`Str`], [`List`], …).
///
/// [`TreeData`]: Self::TreeData
/// [`TreeKind`]: Self::TreeKind
/// [`TreeHandle`]: Self::TreeHandle
/// [`Str`]: Self::Str
/// [`List`]: Self::List
/// # Why the supertrait bounds
///
/// `Clone + Debug + Eq + Hash` are not here because anything clones, prints,
/// compares or hashes a *builder*. They are here so that a user-defined
/// `Kind<B>` can be derived: a derive on a generic type bounds its type
/// parameters, so `#[derive(Debug)] enum ExprKind<B: TreeBuilder>` expands to an
/// impl requiring `B: Debug`, whether or not any `B` is ever printed.
///
/// Requiring them up front means a builder author is told so by the trait,
/// instead of discovering it as an error inside somebody else's kind.
///
/// This works for `Clone`, `Debug` and `PartialEq`, whose `Tree` impls below are
/// unconditional. It does *not* extend to `Eq` and `Hash`: those `Tree` impls
/// are conditional on `TreeKind`, so a recursive kind cannot derive them and has
/// to write them by hand. They are required here anyway, to mirror `TyBuilder`
/// in `melbi-types` and to keep the option of making those impls unconditional
/// later.
pub trait TreeBuilder: Sized + Clone + Debug + Eq + Hash {
    /// Data associated with *every* node of the tree, and the only part that
    /// changes between compiler stages.
    ///
    /// At minimum this carries the node's span; after type inference it also
    /// carries the node's type. Cached summaries of a subtree (flags computed
    /// once at allocation, to avoid repeated traversals) belong here too.
    ///
    /// This is a plain struct chosen by the builder, so reading it is a direct
    /// field access — `tree.data().span` — with no trait or bound involved.
    //
    // TODO: decide whether `span` should be hoisted out of `TreeData` into a
    // dedicated field of `TreeNode`, so that a node is impossible to construct
    // without one and `Tree::span()` works without knowing the concrete
    // `TreeData`. Kept inside `TreeData` for now.
    type TreeData: Clone + Debug + PartialEq;

    /// The navigational, node-specific half of the tree. Usually an enum whose
    /// variants hold their children as [`Tree<Self>`].
    ///
    /// `Eq` is deliberately not required here — an AST holds float literals,
    /// and `f64` is not `Eq` — but see the conditional `Eq` impls below for
    /// kinds that can satisfy it.
    type TreeKind: Clone + Debug + PartialEq;

    /// Handle to an allocated node.
    /// Examples: `Rc<TreeNode<Self>>`, `&'arena TreeNode<Self>`.
    type TreeHandle: AsRef<TreeNode<Self>> + Clone + Debug;

    /// Storage for a string.
    /// Examples: `Rc<str>`, `&'arena str`.
    type Str: AsRef<str> + Clone + Debug + Eq;

    /// Storage for a list of child trees.
    /// Examples: `Rc<[Tree<Self>]>`, `&'arena [Tree<Self>]`.
    type List: Deref<Target = [Tree<Self>]> + Clone + Debug;

    /// Storage for a list of strings, for nodes that hold names but not trees
    /// (e.g. the literal pieces of a format string).
    /// Examples: `Rc<[Self::Str]>`, `&'arena [Self::Str]`.
    type StrList: Deref<Target = [Self::Str]> + Clone + Debug;

    /// Storage for a byte string literal.
    /// Examples: `Rc<[u8]>`, `&'arena [u8]`.
    type Bytes: Deref<Target = [u8]> + Clone + Debug;

    /// Internal: allocate a node and return a handle to it.
    /// Call instead: `TreeNode::new(data, kind).alloc(builder)`.
    fn alloc(&self, node: TreeNode<Self>) -> Self::TreeHandle;

    /// Internal: allocate (or intern) a string.
    fn alloc_str(&self, s: &str) -> Self::Str;

    /// Internal: allocate a list of child trees.
    fn alloc_list(
        &self,
        items: impl IntoIterator<Item = Tree<Self>, IntoIter: ExactSizeIterator>,
    ) -> Self::List;

    /// Internal: allocate a list of strings.
    fn alloc_str_list(
        &self,
        items: impl IntoIterator<Item = Self::Str, IntoIter: ExactSizeIterator>,
    ) -> Self::StrList;

    /// Internal: allocate a byte string.
    fn alloc_bytes(&self, bytes: &[u8]) -> Self::Bytes;
}

// =============================================================================
// Tree - handle to a node
// =============================================================================

/// A handle to a node, and the type every child field uses.
///
/// This is a thin wrapper around [`TreeBuilder::TreeHandle`]. It is `Copy`
/// whenever the underlying handle is (arena builders), and cheap to clone
/// otherwise (reference counted builders).
pub struct Tree<B: TreeBuilder>(B::TreeHandle);

impl<B: TreeBuilder> Tree<B> {
    /// Allocate `node` in `builder` and return a handle to it.
    pub fn new(builder: &B, node: TreeNode<B>) -> Self {
        Self(builder.alloc(node))
    }

    /// Resolve the handle to the node it points at.
    pub fn node(&self) -> &TreeNode<B> {
        self.0.as_ref()
    }

    /// The node's associated data.
    pub fn data(&self) -> &B::TreeData {
        self.node().data()
    }

    /// The node's navigational half.
    pub fn kind(&self) -> &B::TreeKind {
        self.node().kind()
    }

    /// The underlying builder-specific handle.
    pub fn handle(&self) -> &B::TreeHandle {
        &self.0
    }
}

// The impls below are written by hand rather than derived, and are deliberately
// *unconditional*.
//
// `derive` cannot be used: it would bound the builder (`B: Clone`, `B: PartialEq`,
// …), which says nothing about the associated types the fields actually have.
//
// A hand-written impl with a where-clause — `where TreeNode<B>: PartialEq` —
// compiles but is a trap. A tree is cyclic at the type level: a node's kind
// holds `Tree<B>`, which resolves to a node, whose kind holds `Tree<B>`, and the
// solver walks that circle forever (E0275). Stating the bounds on `TreeData` and
// `TreeKind` in the trait instead turns them into assumptions that are
// discharged once, where the concrete builder is defined.
//
// This is also why `Tree` and `TreeNode` are two types rather than one: the
// recursion has to pass through an intermediate type for the solver to have
// somewhere to break the cycle. The split is load-bearing, not cosmetic.

impl<B: TreeBuilder> Clone for Tree<B> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<B: TreeBuilder> Copy for Tree<B> where B::TreeHandle: Copy {}

impl<B: TreeBuilder> Debug for Tree<B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.node().fmt(f)
    }
}

/// Equality is structural — it compares the nodes pointed at, not the handles —
/// so a tree rebuilt into a different builder compares equal to its source.
///
/// Note that this includes `TreeData`, and therefore spans: two identical
/// subtrees at different source offsets are *not* equal.
//
// TODO: if a pass needs "same expression, ignoring spans", add `tree_eq`/
// `tree_hash` hooks on `TreeBuilder`, the way `TyBuilder` has `ty_eq`/`ty_hash`,
// so a builder can override the comparison.
impl<B: TreeBuilder> PartialEq for Tree<B> {
    fn eq(&self, other: &Self) -> bool {
        self.node() == other.node()
    }
}

// =============================================================================
// TreeNode - the node itself
// =============================================================================

/// A node: its associated data plus its navigational half.
///
/// This is what a [`TreeBuilder::TreeHandle`] points at. Construct one with
/// [`TreeNode::new`] and allocate it with [`TreeNode::alloc`].
pub struct TreeNode<B: TreeBuilder> {
    data: B::TreeData,
    kind: B::TreeKind,
}

impl<B: TreeBuilder> TreeNode<B> {
    /// Create a node. Every node must be given its data explicitly.
    //
    // TODO: this pushes the full data onto every caller, including passes that
    // only want to change the kind and keep the data (or vice versa). A better
    // API would let a pass opt out of rebuilding the part it does not touch —
    // e.g. mapping the data from the source node automatically. Deliberately
    // starting with the explicit, less ergonomic version.
    pub fn new(data: B::TreeData, kind: B::TreeKind) -> Self {
        Self { data, kind }
    }

    /// Allocate this node in `builder`.
    pub fn alloc(self, builder: &B) -> Tree<B> {
        Tree::new(builder, self)
    }

    pub fn data(&self) -> &B::TreeData {
        &self.data
    }

    pub fn kind(&self) -> &B::TreeKind {
        &self.kind
    }
}

/// Lets `&TreeNode<B>` satisfy the `AsRef<TreeNode<B>>` bound on
/// [`TreeBuilder::TreeHandle`], so arena builders can use a plain reference as
/// their handle.
impl<B: TreeBuilder> AsRef<TreeNode<B>> for TreeNode<B> {
    fn as_ref(&self) -> &TreeNode<B> {
        self
    }
}

// Unconditional, for the reason given above `impl Clone for Tree`.

impl<B: TreeBuilder> Clone for TreeNode<B> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            kind: self.kind.clone(),
        }
    }
}

impl<B: TreeBuilder> Debug for TreeNode<B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TreeNode")
            .field("data", &self.data)
            .field("kind", &self.kind)
            .finish()
    }
}

impl<B: TreeBuilder> PartialEq for TreeNode<B> {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data && self.kind == other.kind
    }
}

// `Eq` is offered conditionally rather than required of `TreeKind`, because a
// kind holding float literals cannot be `Eq`. A kind that *can* be will not get
// this through `#[derive(Eq)]` — the derive on a generic `Kind<B>` asks for
// `B::TreeKind: Eq`, which `TreeBuilder` does not promise — so such a kind has
// to write its own `impl Eq`. This is an ordinary missing bound, not the
// solver cycle described above.
impl<B: TreeBuilder> Eq for TreeNode<B>
where
    B::TreeData: Eq,
    B::TreeKind: Eq,
{
}

impl<B: TreeBuilder> Eq for Tree<B>
where
    B::TreeData: Eq,
    B::TreeKind: Eq,
{
}

// Hashing mirrors equality: structural, over the node rather than the handle,
// so that a tree and its copy in another builder hash alike.
impl<B: TreeBuilder> Hash for TreeNode<B>
where
    B::TreeData: Hash,
    B::TreeKind: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.data.hash(state);
        self.kind.hash(state);
    }
}

impl<B: TreeBuilder> Hash for Tree<B>
where
    B::TreeData: Hash,
    B::TreeKind: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.node().hash(state);
    }
}
