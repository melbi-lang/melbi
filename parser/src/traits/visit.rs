//! Generic tree traversal.

use super::{Tree, TreeBuilder, TreeNode};

/// A traversal over a tree, producing some [`Output`](Self::Output).
///
/// Both the context and the output are free, which is what lets a single trait
/// cover passes that look nothing alike:
///
/// - **Rebuilding a tree.** `C` bundles the destination builder with whatever
///   mutable state the pass needs (type environment, substitution, diagnostics),
///   and `Output = Tree<Out>`. The destination builder is reached through `C`,
///   so the source and destination lifetimes never meet in a signature.
/// - **Evaluating a tree.** `Output` is a runtime value and no tree is built.
/// - **Collecting information.** `Output = ()` and the result accumulates in `C`.
///
/// # Implement this on the *kind*, not on the node
///
/// With more than one tree in play, a blanket impl per builder trait —
/// `impl<B: ExprBuilder> Visit<B, C> for TreeNode<B>` alongside
/// `impl<B: LiteralBuilder> Visit<B, C> for TreeNode<B>` — is rejected with
/// E0119: coherence cannot rule out a builder implementing both traits, so the
/// two impls overlap. Implementing on `ExprKind<B>` and `LiteralKind<B>` gives
/// distinct `Self` types and no overlap. `melbi-types` puts `Visit` on
/// `TyKind<B>` for the same reason.
///
/// See `parser/docs/poc-mutually-recursive-trees/` for the probe.
///
/// Because the kind alone does not know its own node, the node's data is passed
/// in: a pass needs the span to report errors, and the type once inference has
/// run.
///
/// There is deliberately no `walk` default method: this trait knows nothing
/// about the shape of `TreeKind`, so it cannot enumerate a node's children.
/// Recursion is written out in the `match` over the kind, where it is visible.
//
// TODO: that last point is a real cost — every pass re-implements the child
// recursion. rustc splits this into `TypeVisitable` (the walk skeleton, written
// once per data type) and `TypeVisitor` (the pass), so a pass overrides only the
// cases it cares about and delegates the rest to `super_visit_with`. Adopting
// that split would also let the recursion be swapped for an explicit-stack
// driver without touching any pass.
pub trait Visit<B: TreeBuilder, C> {
    /// What this traversal produces for one node.
    type Output;

    /// Visit a node, given the data of the node this kind belongs to.
    fn visit(&self, data: &B::TreeData, ctx: &mut C) -> Self::Output;
}

impl<B: TreeBuilder> Tree<B> {
    /// Visit this tree, resolving the handle and dispatching on the kind.
    ///
    /// This is an inherent method rather than a `Visit` impl on `Tree<B>`: a
    /// blanket impl would have to be written per builder trait and would run
    /// into the same E0119 overlap described on [`Visit`].
    pub fn visit<C>(&self, ctx: &mut C) -> <B::TreeKind as Visit<B, C>>::Output
    where
        B::TreeKind: Visit<B, C>,
    {
        self.node().visit(ctx)
    }
}

impl<B: TreeBuilder> TreeNode<B> {
    /// Visit this node, dispatching on its kind and supplying its data.
    pub fn visit<C>(&self, ctx: &mut C) -> <B::TreeKind as Visit<B, C>>::Output
    where
        B::TreeKind: Visit<B, C>,
    {
        self.kind().visit(self.data(), ctx)
    }
}
