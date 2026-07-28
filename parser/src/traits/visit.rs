//! Generic tree traversal.

use super::{Tree, TreeBuilder, TreeDescriptor, TreeNode};

/// A traversal over tree `D`, producing some [`Output`](Self::Output).
///
/// Both the context and the output are free, which is what lets a single trait
/// cover passes that look nothing alike:
///
/// - **Rebuilding a tree.** `C` bundles the destination builder with whatever
///   mutable state the pass needs (type environment, substitution, diagnostics),
///   and `Output = Tree<Out, _>`. The destination builder is reached through
///   `C`, so the source and destination lifetimes never meet in a signature.
/// - **Lowering between stages.** The output tree's descriptor need not match
///   the input's, and need not exist at all: lowering `ParsedLiteral` yields a
///   folded `Value`, not a tree, which is how literals disappear after
///   type-checking.
/// - **Evaluating a tree.** `Output` is a runtime value and no tree is built.
/// - **Collecting information.** `Output = ()` and the result accumulates in `C`.
///
/// # Implement this on the *kind*, not on the node
///
/// `Self` is the pass-specific type, so the impls a pass writes are on
/// `ParsedExprKind<B>`, `ParsedLiteralKind<B>`, … — distinct types that can
/// never overlap. `melbi-types` puts `Visit` on `TyKind<B>` for the same reason.
///
/// Under the previous one-builder-per-tree design this was also forced by
/// coherence: two blanket impls on the shared `TreeNode<B>`, one per builder
/// trait, were rejected with E0119 because nothing ruled out a builder
/// implementing both traits. That hazard is gone — the tree is now named by the
/// `D` parameter rather than by which trait `B` implements — but implementing on
/// the kind remains the right shape, and `Tree`/`TreeNode` forward to it through
/// the inherent methods below.
///
/// See `parser/docs/tree-design-probes/` for the probes.
///
/// Because the kind alone does not know its own node, the node's data is passed
/// in: a pass needs the span to report errors, and the type once inference has
/// run. `D::Data` is concrete, so that argument is an ordinary struct and a pass
/// reads its fields directly, with no bound of its own.
///
/// There is deliberately no `walk` default method: this trait knows nothing
/// about the shape of a kind, so it cannot enumerate a node's children.
/// Recursion is written out in the `match` over the kind, where it is visible.
//
// TODO: that last point is a real cost — every pass re-implements the child
// recursion. rustc splits this into `TypeVisitable` (the walk skeleton, written
// once per data type) and `TypeVisitor` (the pass), so a pass overrides only the
// cases it cares about and delegates the rest to `super_visit_with`. Adopting
// that split would also let the recursion be swapped for an explicit-stack
// driver without touching any pass.
pub trait Visit<B: TreeBuilder, D: TreeDescriptor, C> {
    /// What this traversal produces for one node.
    type Output;

    /// Visit a node, given the data of the node this kind belongs to.
    fn visit(&self, data: &D::Data, ctx: &mut C) -> Self::Output;
}

impl<B: TreeBuilder, D: TreeDescriptor> Tree<B, D> {
    /// Visit this tree, resolving the handle and dispatching on the kind.
    pub fn visit<C>(&self, ctx: &mut C) -> <D::Kind<B> as Visit<B, D, C>>::Output
    where
        D::Kind<B>: Visit<B, D, C>,
    {
        self.node().visit(ctx)
    }
}

impl<B: TreeBuilder, D: TreeDescriptor> TreeNode<B, D> {
    /// Visit this node, dispatching on its kind and supplying its data.
    pub fn visit<C>(&self, ctx: &mut C) -> <D::Kind<B> as Visit<B, D, C>>::Output
    where
        D::Kind<B>: Visit<B, D, C>,
    {
        self.kind().visit(self.data(), ctx)
    }
}
