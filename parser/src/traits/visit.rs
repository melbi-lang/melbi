//! Generic tree traversal.

use super::{Tree, TreeBuilder, TreeNode};

/// A traversal over a tree, producing some [`Output`](Self::Output).
///
/// Both the context and the output are free, which is what lets a single trait
/// cover passes that look nothing alike:
///
/// - **Rebuilding a tree.** `C` bundles the destination builder with whatever
///   mutable state the pass needs (type environment, substitution, diagnostics),
///   and `Output = Tree<Out>`. Note that the destination builder is reached
///   through `C`, so the source and destination lifetimes never meet in a
///   signature.
/// - **Evaluating a tree.** `Output` is a runtime value and no tree is built
///   at all.
/// - **Collecting information.** `Output = ()` and the result accumulates in `C`.
///
/// `C` is taken by `&mut`, so a pass may thread mutable state through the
/// traversal. Implement this on [`TreeNode<B>`]; the blanket impl below
/// forwards from [`Tree<B>`], so a traversal recurses with `child.visit(ctx)`
/// without resolving handles by hand.
///
/// There is deliberately no `walk` default method: this trait knows nothing
/// about the shape of `TreeKind`, so it cannot enumerate a node's children.
/// Recursion is written out in the `match` over the kind, where it is visible.
pub trait Visit<B: TreeBuilder, C> {
    /// What this traversal produces for one node.
    type Output;

    /// Visit a single node.
    fn visit(&self, ctx: &mut C) -> Self::Output;
}

/// Forwards `Tree::visit` to the node it points at, so implementations only
/// have to cover [`TreeNode<B>`].
impl<B: TreeBuilder, C> Visit<B, C> for Tree<B>
where
    TreeNode<B>: Visit<B, C>,
{
    type Output = <TreeNode<B> as Visit<B, C>>::Output;

    fn visit(&self, ctx: &mut C) -> Self::Output {
        self.node().visit(ctx)
    }
}
