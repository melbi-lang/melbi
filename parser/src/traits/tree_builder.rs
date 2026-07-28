//! The two tree traits and the core tree types.
//!
//! # Design Philosophy
//!
//! A tree is pinned down along two independent axes, and the whole design falls
//! out of keeping them independent:
//!
//! - **What a tree is** — [`TreeDescriptor`]. Its navigational half
//!   ([`Kind`](TreeDescriptor::Kind), usually an enum whose variants describe
//!   the shape of a node and whose children are [`Tree`]s) and its associated
//!   data ([`Data`](TreeDescriptor::Data), carried by every node of that tree).
//! - **How a tree is stored** — [`TreeBuilder`]. Heap, arena, interning.
//!
//! Keeping the navigational and data halves apart is what makes a traversal
//! readable: matching on the kind never has to mention the data, and reading the
//! data never has to match on the kind.
//!
//! Keeping *what* and *how* apart is what lets one builder host every tree in
//! the AST, and lets one tree be stored several ways. Because the storage types
//! are associated types of the builder, the allocator's lifetime never appears
//! in code that consumes a tree — a pass is generic over `B: TreeBuilder`, not
//! over `'arena`. Copying a tree from one builder to another is then just a
//! traversal that allocates into a different builder, with no lifetime
//! bookkeeping at the call sites.
//!
//! # A compiler stage is a descriptor, not a builder
//!
//! `ParsedExpr` and `TypedExpr` are two descriptors, each with its own `Data`
//! *and* its own `Kind`. That is what makes the stages genuinely different
//! types: literals fold away entirely after type-checking, `Ident(name)` becomes
//! a resolved slot, and nodes that only exist after inference have no parsed
//! counterpart. A single enum spanning both stages would have to be their union,
//! with every pass carrying unreachable arms for the variants its stage cannot
//! produce.
//!
//! Because the stage lives in the descriptor, [`TreeDescriptor::Data`] is a
//! plain associated type rather than something indexed by the builder, and
//! [`TreeBuilder`] knows nothing about stages at all.
//!
//! # Why one builder with descriptor-indexed types
//!
//! The obvious alternative is one builder trait per tree, each naming the others
//! through paired associated types with `<Expr = Self>`-style equality
//! constraints. That works for two trees and collapses beyond them: `N` trees
//! need `N` traits holding `N-1` associated types each, and every pass has to
//! restate the agreement between the data of each tree it touches. Indexing the
//! builder's types by the descriptor instead costs nothing per tree, and a pass
//! that crosses four trees still writes a single `B: TreeBuilder`.
//!
//! A supertrait bundling several builders does not work at all: `S::Expr` is an
//! associated-type projection, projections are not injective, so `S` cannot be
//! recovered from `Tree<S::Expr>` — every call needs a turbofish (E0283) and the
//! forwarding impl cannot be written (E0207). `Tree<B, D>` *is* injective in
//! both parameters, so inference works with no annotations.
//!
//! Probes for these claims, with exact error codes, are in
//! `parser/docs/tree-design-probes/`.

use core::fmt::Debug;
use core::hash::{Hash, Hasher};
use core::ops::Deref;

/// One tree of the AST, at one compiler stage.
///
/// Implemented on an empty marker struct — `ParsedExpr`, `TypedExpr`,
/// `ParsedLiteral` — which is then used as a type argument to [`Tree`],
/// [`TreeNode`] and the builder's storage types. That marker is doing real
/// work: `Tree<B, ParsedExpr>` and `Tree<B, ParsedLiteral>` are distinct,
/// injective types, which is what multiplexes the tree machinery across every
/// tree instead of it being hand-expanded per tree, and what keeps [`Visit`]
/// impls from overlapping.
///
/// [`Visit`]: crate::Visit
pub trait TreeDescriptor: Sized + 'static {
    /// Data carried by *every* node of this tree.
    ///
    /// At minimum the node's span; a typed stage's data also carries the node's
    /// type. Cached summaries of a subtree (flags computed once at allocation,
    /// to avoid repeated traversals) belong here too.
    ///
    /// This is a plain struct chosen by the descriptor, so reading it is a
    /// direct field access — `tree.data().span` — with no trait or bound
    /// involved, in any pass, without a where-clause.
    //
    // TODO: decide whether `span` should be hoisted out of `Data` into a
    // dedicated field of `TreeNode`, so that a node is impossible to construct
    // without one and `Tree::span()` works without knowing the concrete `Data`.
    // Kept inside `Data` for now.
    type Data: Clone + Debug + PartialEq;

    /// The navigational, node-specific half. Usually an enum whose variants hold
    /// their children as `Tree<B, _>`.
    ///
    /// Indexed by the builder because a kind holds trees, and a tree's storage
    /// is the builder's business.
    ///
    /// `Eq` is deliberately not required — an AST holds float literals, and
    /// `f64` is not `Eq` — but see the conditional `Eq` impls below for kinds
    /// that can satisfy it.
    type Kind<B: TreeBuilder>: Clone + Debug + PartialEq;
}

/// Allocation strategy for every tree of the AST.
///
/// Purely about storage: which allocator nodes come from, and how strings,
/// slices and byte strings are held. It knows nothing about what any tree
/// contains, nor which stage it is in — that is [`TreeDescriptor`]'s job.
///
/// # Why the supertrait bounds
///
/// `Clone + Debug + Eq + Hash` are not here because anything clones, prints,
/// compares or hashes a *builder*. They are here so that a user-defined
/// `Kind<B>` can be derived: a derive on a generic type bounds its type
/// parameters, so `#[derive(Debug)] enum ParsedExprKind<B: TreeBuilder>` expands
/// to an impl requiring `B: Debug`, whether or not any `B` is ever printed.
///
/// Requiring them up front means a builder author is told so by the trait,
/// instead of discovering it as an error inside somebody else's kind.
///
/// # Why `PartialEq` on the storage types
///
/// `List`, `StrList` and `Bytes` require `PartialEq` for the same reason, but a
/// step further along. A descriptor promises `Kind<B>: PartialEq` for *every*
/// builder, so that proof has to go through generically, which in turn needs
/// every field of the kind to be `PartialEq` generically. Under the previous
/// one-builder-per-tree design these were discharged lazily at each concrete
/// builder and could be left unstated; now they are assumptions, stated once.
pub trait TreeBuilder: Sized + Clone + Debug + Eq + Hash {
    /// Handle to an allocated node of tree `D`.
    /// Examples: `Rc<TreeNode<Self, D>>`, `&'arena TreeNode<Self, D>`.
    type Handle<D: TreeDescriptor>: AsRef<TreeNode<Self, D>> + Clone + Debug;

    /// Storage for a list of child trees of tree `D`.
    /// Examples: `Rc<[Tree<Self, D>]>`, `&'arena [Tree<Self, D>]`.
    type List<D: TreeDescriptor>: Deref<Target = [Tree<Self, D>]> + Clone + Debug + PartialEq;

    /// Storage for a string.
    /// Examples: `Rc<str>`, `&'arena str`.
    type Str: AsRef<str> + Clone + Debug + Eq;

    /// Storage for a list of strings, for nodes that hold names but not trees
    /// (e.g. the literal pieces of a format string).
    /// Examples: `Rc<[Self::Str]>`, `&'arena [Self::Str]`.
    type StrList: Deref<Target = [Self::Str]> + Clone + Debug + PartialEq;

    /// Storage for a byte string literal.
    /// Examples: `Rc<[u8]>`, `&'arena [u8]`.
    type Bytes: Deref<Target = [u8]> + Clone + Debug + PartialEq;

    /// Internal: allocate a node and return a handle to it.
    /// Call instead: `TreeNode::new(data, kind).alloc(builder)`.
    fn alloc<D: TreeDescriptor>(&self, node: TreeNode<Self, D>) -> Self::Handle<D>;

    /// Internal: allocate a list of child trees.
    fn alloc_list<D: TreeDescriptor>(
        &self,
        items: impl IntoIterator<Item = Tree<Self, D>, IntoIter: ExactSizeIterator>,
    ) -> Self::List<D>;

    /// Internal: allocate (or intern) a string.
    fn alloc_str(&self, s: &str) -> Self::Str;

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

/// A handle to a node of tree `D`, and the type every child field uses.
///
/// This is a thin wrapper around [`TreeBuilder::Handle`]. It is `Copy` whenever
/// the underlying handle is (arena builders), and cheap to clone otherwise
/// (reference counted builders).
pub struct Tree<B: TreeBuilder, D: TreeDescriptor>(B::Handle<D>);

impl<B: TreeBuilder, D: TreeDescriptor> Tree<B, D> {
    /// Allocate `node` in `builder` and return a handle to it.
    pub fn new(builder: &B, node: TreeNode<B, D>) -> Self {
        Self(builder.alloc(node))
    }

    /// Resolve the handle to the node it points at.
    pub fn node(&self) -> &TreeNode<B, D> {
        self.0.as_ref()
    }

    /// The node's associated data.
    pub fn data(&self) -> &D::Data {
        self.node().data()
    }

    /// The node's navigational half.
    pub fn kind(&self) -> &D::Kind<B> {
        self.node().kind()
    }

    /// The underlying builder-specific handle.
    pub fn handle(&self) -> &B::Handle<D> {
        &self.0
    }
}

// The impls below are written by hand rather than derived, and are deliberately
// *unconditional*.
//
// `derive` cannot be used: it would bound the type parameters (`B: Clone`,
// `D: PartialEq`, …), which says nothing about the associated types the fields
// actually have.
//
// A hand-written impl with a where-clause — `where TreeNode<B, D>: PartialEq` —
// compiles but is a trap. A tree is cyclic at the type level: a node's kind
// holds `Tree<B, _>`, which resolves to a node, whose kind holds `Tree<B, _>`,
// and the solver walks that circle forever (E0275). Stating the bounds on
// `Data` and `Kind` in `TreeDescriptor` instead turns them into assumptions,
// discharged once where the concrete descriptor is defined.
//
// This is also why `Tree` and `TreeNode` are two types rather than one: the
// recursion has to pass through an intermediate type for the solver to have
// somewhere to break the cycle. The split is load-bearing, not cosmetic —
// collapsing the node into the handle (`&'a (Data, Kind)`) reintroduces E0275
// immediately.

impl<B: TreeBuilder, D: TreeDescriptor> Clone for Tree<B, D> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<B: TreeBuilder, D: TreeDescriptor> Copy for Tree<B, D> where B::Handle<D>: Copy {}

impl<B: TreeBuilder, D: TreeDescriptor> Debug for Tree<B, D> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.node().fmt(f)
    }
}

/// Equality is structural — it compares the nodes pointed at, not the handles —
/// so a tree rebuilt into a different builder compares equal to its source.
///
/// Note that this includes `Data`, and therefore spans: two identical subtrees
/// at different source offsets are *not* equal.
//
// TODO: if a pass needs "same expression, ignoring spans", add `tree_eq`/
// `tree_hash` hooks on `TreeBuilder`, the way `TyBuilder` has `ty_eq`/`ty_hash`,
// so a builder can override the comparison.
impl<B: TreeBuilder, D: TreeDescriptor> PartialEq for Tree<B, D> {
    fn eq(&self, other: &Self) -> bool {
        self.node() == other.node()
    }
}

// =============================================================================
// TreeNode - the node itself
// =============================================================================

/// A node: its associated data plus its navigational half.
///
/// This is what a [`TreeBuilder::Handle`] points at. Construct one with
/// [`TreeNode::new`] and allocate it with [`TreeNode::alloc`].
pub struct TreeNode<B: TreeBuilder, D: TreeDescriptor> {
    data: D::Data,
    kind: D::Kind<B>,
}

impl<B: TreeBuilder, D: TreeDescriptor> TreeNode<B, D> {
    /// Create a node. Every node must be given its data explicitly.
    //
    // TODO: this pushes the full data onto every caller, including passes that
    // only want to change the kind and keep the data (or vice versa). A better
    // API would let a pass opt out of rebuilding the part it does not touch —
    // e.g. mapping the data from the source node automatically. Deliberately
    // starting with the explicit, less ergonomic version.
    pub fn new(data: D::Data, kind: D::Kind<B>) -> Self {
        Self { data, kind }
    }

    /// Allocate this node in `builder`.
    pub fn alloc(self, builder: &B) -> Tree<B, D> {
        Tree::new(builder, self)
    }

    pub fn data(&self) -> &D::Data {
        &self.data
    }

    pub fn kind(&self) -> &D::Kind<B> {
        &self.kind
    }
}

/// Lets `&TreeNode<B, D>` satisfy the `AsRef<TreeNode<B, D>>` bound on
/// [`TreeBuilder::Handle`], so arena builders can use a plain reference as their
/// handle.
impl<B: TreeBuilder, D: TreeDescriptor> AsRef<TreeNode<B, D>> for TreeNode<B, D> {
    fn as_ref(&self) -> &TreeNode<B, D> {
        self
    }
}

// Unconditional, for the reason given above `impl Clone for Tree`.

impl<B: TreeBuilder, D: TreeDescriptor> Clone for TreeNode<B, D> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            kind: self.kind.clone(),
        }
    }
}

impl<B: TreeBuilder, D: TreeDescriptor> Debug for TreeNode<B, D> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TreeNode")
            .field("data", &self.data)
            .field("kind", &self.kind)
            .finish()
    }
}

impl<B: TreeBuilder, D: TreeDescriptor> PartialEq for TreeNode<B, D> {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data && self.kind == other.kind
    }
}

// `Eq` is offered conditionally rather than required of `Kind`, because a kind
// holding float literals cannot be `Eq`. A kind that *can* be will not get this
// through `#[derive(Eq)]` — the derive on a generic `Kind<B>` asks for
// `B: Eq`, which says nothing about the fields — so such a kind has to write its
// own `impl Eq`. This is an ordinary missing bound, not the solver cycle
// described above.
impl<B: TreeBuilder, D: TreeDescriptor> Eq for TreeNode<B, D>
where
    D::Data: Eq,
    D::Kind<B>: Eq,
{
}

impl<B: TreeBuilder, D: TreeDescriptor> Eq for Tree<B, D>
where
    D::Data: Eq,
    D::Kind<B>: Eq,
{
}

// Hashing mirrors equality: structural, over the node rather than the handle,
// so that a tree and its copy in another builder hash alike.
impl<B: TreeBuilder, D: TreeDescriptor> Hash for TreeNode<B, D>
where
    D::Data: Hash,
    D::Kind<B>: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.data.hash(state);
        self.kind.hash(state);
    }
}

impl<B: TreeBuilder, D: TreeDescriptor> Hash for Tree<B, D>
where
    D::Data: Hash,
    D::Kind<B>: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.node().hash(state);
    }
}
