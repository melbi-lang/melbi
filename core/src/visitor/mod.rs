//! Generic tree visitor pattern for traversing and transforming ASTs.
//!
//! This module provides a trait-based infrastructure for working with tree structures
//! in a type-safe and reusable way. It supports both arena and heap allocation,
//! mutable and immutable traversals, and transformations to arbitrary output types.

use core::fmt::Debug;
use core::hash::Hash;

/// Builder for constructing tree nodes.
///
/// This trait abstracts over different allocation strategies (arena vs heap)
/// and allows tree algorithms to be generic over allocation method.
pub trait TreeBuilder: Copy + Clone + Debug + Eq {
    /// The representation of a tree node view (reference or pointer).
    type TreeViewRepr: TreeView<Self> + Clone + Debug + Eq + Hash;

    /// Optional data attached to each node (use `()` if not needed).
    type DataRepr: Debug + Clone + PartialEq + Eq + Hash;
}

/// View into a tree node, allowing deconstruction.
///
/// This trait provides read-only access to a tree node's structure.
pub trait TreeView<B: TreeBuilder>: Sized + Clone {
    /// Get the kind/variant of this node.
    ///
    /// Note: The "kind" is application-specific. For Melbi's `TypedExpr`,
    /// this would return the `ExprInner` enum.
    type Kind;

    /// Deconstruct this node into its kind.
    fn view(self) -> Self::Kind;

    /// Get optional data attached to this node.
    fn data(self) -> Option<B::DataRepr>;
}

/// Generic transformer for tree structures.
///
/// This trait can be used for:
/// - Tree transformations (Output = tree type)
/// - Evaluation (Output = value type, e.g., i32)
/// - Side-effect traversals (Output = (), e.g., validation, bytecode generation)
/// - Analysis (Output = analysis result)
///
/// The transformer can be stateful (uses `&mut self`) to accumulate results,
/// track context, or maintain mutable state during traversal.
pub trait TreeTransformer<B: TreeBuilder> {
    /// The type of value produced by the transformation.
    ///
    /// Examples:
    /// - `()` for side-effect-only traversals (visitors)
    /// - `i32` for evaluation
    /// - `B::TreeViewRepr` for tree-to-tree transformations
    /// - `Vec<Instruction>` for compilation
    type Output;

    /// Transform a tree node.
    ///
    /// This method is called recursively to traverse and transform the tree.
    /// The transformer is responsible for recursing into children as needed.
    fn transform(&mut self, tree: B::TreeViewRepr) -> Self::Output;
}

#[cfg(test)]
mod tests;
