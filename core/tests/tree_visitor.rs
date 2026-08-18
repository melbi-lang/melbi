//! Prototype for trait-based tree visitor pattern.
//!
//! This demonstrates the design we'll use for the analyzer expression tree visitors.
//! The pattern is based on the existing TypeView/TypeBuilder/TypeVisitor traits.

use core::fmt::Debug;
use core::hash::Hash;

use bumpalo::Bump;

// === Traits ===

pub trait TreeBuilder: Copy + Clone + Debug + Eq {
    type TreeViewRepr: TreeView<Self> + Clone + Debug + Eq + Hash;
    type DataRepr: Debug + Clone + PartialEq + Eq + Hash;

    fn build(self, kind: TreeKind<Self>) -> Self::TreeViewRepr;
}

pub trait TreeView<B: TreeBuilder>: Sized + Clone {
    fn view(self) -> TreeKind<B>;
    fn data(self) -> Option<B::DataRepr>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TreeKind<B: TreeBuilder> {
    Num(i32),
    Add(B::TreeViewRepr, B::TreeViewRepr),
    Mul(B::TreeViewRepr, B::TreeViewRepr),
    Neg(B::TreeViewRepr),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TreeData<B: TreeBuilder> {
    pub kind: TreeKind<B>,
    pub data: B::DataRepr,
}

impl<B: TreeBuilder> TreeKind<B> {
    /// Deep structural equality check.
    ///
    /// Since `TreeKind` contains `TreeViewRepr` (references), we can't easily
    /// construct tree literals for comparison. This recursively compares structure.
    pub fn structural_eq(&self, other: &Self) -> bool
    where
        B::TreeViewRepr: Copy,
    {
        match (self, other) {
            (Self::Num(a), Self::Num(b)) => a == b,
            (Self::Add(l1, r1), Self::Add(l2, r2)) | (Self::Mul(l1, r1), Self::Mul(l2, r2)) => {
                l1.view().structural_eq(&l2.view()) && r1.view().structural_eq(&r2.view())
            }
            (Self::Neg(a), Self::Neg(b)) => a.view().structural_eq(&b.view()),
            _ => false,
        }
    }

    /// Pattern matching helper: check if this is a Num with expected value.
    pub fn is_num(&self, expected: i32) -> bool {
        matches!(self, Self::Num(n) if *n == expected)
    }

    /// Check if this is an Add node.
    pub fn is_add(&self) -> bool {
        matches!(self, Self::Add(_, _))
    }

    /// Check if this is a Mul node.
    pub fn is_mul(&self) -> bool {
        matches!(self, Self::Mul(_, _))
    }

    /// Check if this is a Neg node.
    pub fn is_neg(&self) -> bool {
        matches!(self, Self::Neg(_))
    }
}

// === Implementation ===

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
struct BoxedTreeBuilder;

impl TreeBuilder for BoxedTreeBuilder {
    type TreeViewRepr = Box<TreeData<Self>>;
    type DataRepr = String;

    fn build(self, kind: TreeKind<Self>) -> Self::TreeViewRepr {
        Box::new(TreeData {
            kind,
            data: "Hello 🖖".to_string(),
        })
    }
}

impl<B: TreeBuilder> TreeView<B> for Box<TreeData<B>> {
    fn view(self) -> TreeKind<B> {
        self.kind.clone()
    }
    fn data(self) -> Option<B::DataRepr> {
        Some(self.data.clone())
    }
}

/// Arena-based tree builder (like Melbi's analyzer uses).
///
/// Allocates trees in a bump allocator for efficient memory usage and
/// automatic cleanup. Trees are borrowed from the arena with lifetime 'arena.
#[derive(Debug, Clone, Copy)]
struct ArenaTreeBuilder<'arena> {
    arena: &'arena Bump,
}

impl<'arena> ArenaTreeBuilder<'arena> {
    fn new(arena: &'arena Bump) -> Self {
        Self { arena }
    }
}

// Manual trait implementations since Bump doesn't implement these traits
impl PartialEq for ArenaTreeBuilder<'_> {
    fn eq(&self, other: &Self) -> bool {
        core::ptr::eq(self.arena, other.arena)
    }
}

impl Eq for ArenaTreeBuilder<'_> {}

impl core::hash::Hash for ArenaTreeBuilder<'_> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        core::ptr::hash(self.arena, state);
    }
}

impl<'arena> TreeBuilder for ArenaTreeBuilder<'arena> {
    type TreeViewRepr = &'arena TreeData<Self>;
    type DataRepr = String;

    fn build(self, kind: TreeKind<Self>) -> Self::TreeViewRepr {
        self.arena.alloc(TreeData {
            kind,
            data: "Arena-allocated 🚀".to_string(),
        })
    }
}

impl<B: TreeBuilder> TreeView<B> for &TreeData<B> {
    fn view(self) -> TreeKind<B> {
        self.kind.clone()
    }
    fn data(self) -> Option<B::DataRepr> {
        Some(self.data.clone())
    }
}

// === TreeVisitor ===

pub trait TreeVisitor<B: TreeBuilder> {
    fn visit(&mut self, tree: B::TreeViewRepr) {
        self.visit_default(tree);
    }

    fn visit_default(&mut self, tree: B::TreeViewRepr) {
        match tree.view() {
            TreeKind::Num(_) => {
                // Leaf node, nothing to traverse
            }
            TreeKind::Add(left, right) | TreeKind::Mul(left, right) => {
                self.visit(left);
                self.visit(right);
            }
            TreeKind::Neg(inner) => {
                self.visit(inner);
            }
        }
    }
}

// === TreeTransformer ===
/// Generic transformer trait that can transform trees into any type.
///
/// For tree-to-tree transformations, set `ReturnType = B::TreeViewRepr`.
/// For evaluation, set `ReturnType` to the value type (e.g., `i32`).
pub trait TreeTransformer<B: TreeBuilder> {
    type ReturnType;

    fn builder(&self) -> B;

    fn transform(&self, tree: B::TreeViewRepr) -> Self::ReturnType;
}

// ============================================================================
// Example Implementations
// ============================================================================

/// Example visitor: Count the number of nodes in a tree.
struct NodeCounter {
    count: usize,
}

impl NodeCounter {
    fn new() -> Self {
        Self { count: 0 }
    }
}

impl TreeVisitor<BoxedTreeBuilder> for NodeCounter {
    fn visit(&mut self, tree: Box<TreeData<BoxedTreeBuilder>>) {
        self.count += 1;
        self.visit_default(tree);
    }
}

/// Example of `TreeTransformer` with `ReturnType` = () (acts as a visitor).
///
/// This demonstrates that transformers can also do side-effect-only traversals.
struct MaxDepthFinder {
    max_depth: core::cell::Cell<usize>,
}

impl MaxDepthFinder {
    fn new() -> Self {
        Self {
            max_depth: core::cell::Cell::new(0),
        }
    }

    fn max_depth(&self) -> usize {
        self.max_depth.get()
    }

    fn traverse_with_depth(&self, tree: Box<TreeData<BoxedTreeBuilder>>, depth: usize) {
        // Update max depth
        if depth > self.max_depth.get() {
            self.max_depth.set(depth);
        }

        match tree.view() {
            TreeKind::Num(_) => {}
            TreeKind::Add(left, right) | TreeKind::Mul(left, right) => {
                self.traverse_with_depth(left, depth + 1);
                self.traverse_with_depth(right, depth + 1);
            }
            TreeKind::Neg(inner) => {
                self.traverse_with_depth(inner, depth + 1);
            }
        }
    }
}

impl TreeTransformer<BoxedTreeBuilder> for MaxDepthFinder {
    type ReturnType = (); // Visitor: returns nothing, does side effects

    fn builder(&self) -> BoxedTreeBuilder {
        BoxedTreeBuilder
    }

    fn transform(&self, tree: Box<TreeData<BoxedTreeBuilder>>) -> Self::ReturnType {
        self.traverse_with_depth(tree, 0);
    }
}

/// Example transformer: Negate all number literals.
struct NegateNumbers;

impl NegateNumbers {
    fn new() -> Self {
        Self
    }
}

impl TreeTransformer<BoxedTreeBuilder> for NegateNumbers {
    type ReturnType = Box<TreeData<BoxedTreeBuilder>>;

    fn builder(&self) -> BoxedTreeBuilder {
        BoxedTreeBuilder
    }

    fn transform(&self, tree: Box<TreeData<BoxedTreeBuilder>>) -> Self::ReturnType {
        match tree.view() {
            TreeKind::Num(n) => self.builder().build(TreeKind::Num(-n)),
            TreeKind::Add(left, right) => {
                let left_t = self.transform(left);
                let right_t = self.transform(right);
                self.builder().build(TreeKind::Add(left_t, right_t))
            }
            TreeKind::Mul(left, right) => {
                let left_t = self.transform(left);
                let right_t = self.transform(right);
                self.builder().build(TreeKind::Mul(left_t, right_t))
            }
            TreeKind::Neg(inner) => {
                let inner_t = self.transform(inner);
                self.builder().build(TreeKind::Neg(inner_t))
            }
        }
    }
}

/// Example evaluator: Transform tree to its computed value.
///
/// This demonstrates `ReturnType` being different from `B::TreeViewRepr`.
struct Evaluator;

impl Evaluator {
    fn new() -> Self {
        Self
    }
}

impl TreeTransformer<BoxedTreeBuilder> for Evaluator {
    type ReturnType = i32;

    fn builder(&self) -> BoxedTreeBuilder {
        BoxedTreeBuilder
    }

    fn transform(&self, tree: Box<TreeData<BoxedTreeBuilder>>) -> Self::ReturnType {
        match tree.view() {
            TreeKind::Num(n) => n,
            TreeKind::Add(left, right) => self.transform(left) + self.transform(right),
            TreeKind::Mul(left, right) => self.transform(left) * self.transform(right),
            TreeKind::Neg(inner) => -self.transform(inner),
        }
    }
}

/// Example transformer: Constant folding (evaluate constant expressions).
struct ConstantFolder;

impl ConstantFolder {
    fn new() -> Self {
        Self
    }
}

impl TreeTransformer<BoxedTreeBuilder> for ConstantFolder {
    type ReturnType = Box<TreeData<BoxedTreeBuilder>>;

    fn builder(&self) -> BoxedTreeBuilder {
        BoxedTreeBuilder
    }

    fn transform(&self, tree: Box<TreeData<BoxedTreeBuilder>>) -> Self::ReturnType {
        match tree.view() {
            TreeKind::Num(n) => self.builder().build(TreeKind::Num(n)),
            TreeKind::Add(left, right) => {
                let left_t = self.transform(left);
                let right_t = self.transform(right);

                // If both operands are constants, fold them
                match (left_t.clone().view(), right_t.clone().view()) {
                    (TreeKind::Num(l), TreeKind::Num(r)) => {
                        self.builder().build(TreeKind::Num(l + r))
                    }
                    _ => self.builder().build(TreeKind::Add(left_t, right_t)),
                }
            }
            TreeKind::Mul(left, right) => {
                let left_t = self.transform(left);
                let right_t = self.transform(right);

                // If both operands are constants, fold them
                match (left_t.clone().view(), right_t.clone().view()) {
                    (TreeKind::Num(l), TreeKind::Num(r)) => {
                        self.builder().build(TreeKind::Num(l * r))
                    }
                    _ => self.builder().build(TreeKind::Mul(left_t, right_t)),
                }
            }
            TreeKind::Neg(inner) => {
                let inner_t = self.transform(inner);

                // If operand is constant, fold it
                match inner_t.clone().view() {
                    TreeKind::Num(n) => self.builder().build(TreeKind::Num(-n)),
                    _ => self.builder().build(TreeKind::Neg(inner_t)),
                }
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_example_tree() -> Box<TreeData<BoxedTreeBuilder>> {
        let b = BoxedTreeBuilder;
        // (2 + 3) * -(4 + 5)
        b.build(TreeKind::Mul(
            b.build(TreeKind::Add(
                b.build(TreeKind::Num(2)),
                b.build(TreeKind::Num(3)),
            )),
            b.build(TreeKind::Neg(b.build(TreeKind::Add(
                b.build(TreeKind::Num(4)),
                b.build(TreeKind::Num(5)),
            )))),
        ))
    }

    #[test]
    fn node_counter() {
        let tree = make_example_tree();
        let mut counter = NodeCounter::new();
        counter.visit(tree);
        assert_eq!(counter.count, 8); // 2 Adds, 1 Mul, 1 Neg, 4 Nums
    }

    #[test]
    fn evaluator() {
        let tree = make_example_tree();
        // (2 + 3) * -(4 + 5) = 5 * -9 = -45
        let evaluator = Evaluator::new();
        let result = evaluator.transform(tree);
        assert_eq!(result, -45);
    }

    #[test]
    fn max_depth_finder() {
        let tree = make_example_tree();
        // Tree structure: Mul -> Add/Neg -> Num/Add -> Num
        // Max depth is 3 (Mul -> Neg -> Add -> Num)
        let finder = MaxDepthFinder::new();
        finder.transform(tree);
        assert_eq!(finder.max_depth(), 3);
    }

    #[test]
    fn compare_trees() {
        let arena = Bump::new();
        let b = ArenaTreeBuilder::new(&arena);

        // Single node comparison works with direct equality
        let a = b.build(TreeKind::Num(1));
        assert_eq!(a.view(), TreeKind::Num(1));

        // Build trees: 2 + 3 and 2 + 3
        let tree1 = b.build(TreeKind::Add(
            b.build(TreeKind::Num(2)),
            b.build(TreeKind::Num(3)),
        ));
        let tree2 = b.build(TreeKind::Add(
            b.build(TreeKind::Num(2)),
            b.build(TreeKind::Num(3)),
        ));

        // Can't use simple equality because children are different references
        // assert_eq!(tree1.view(), tree2.view()); // Won't work!

        // Use structural equality instead
        assert!(tree1.view().structural_eq(&tree2.view()));

        // Or use pattern matching helpers
        assert!(tree1.view().is_add());
        if let TreeKind::Add(left, right) = tree1.view() {
            assert!(left.view().is_num(2));
            assert!(right.view().is_num(3));
        }
    }

    #[test]
    fn arena_tree_builder() {
        let arena = Bump::new();
        let b = ArenaTreeBuilder::new(&arena);

        // Build a simple tree: 2 + 3
        let tree = b.build(TreeKind::Add(
            b.build(TreeKind::Num(2)),
            b.build(TreeKind::Num(3)),
        ));

        // Evaluate it
        let evaluator = ArenaEvaluator;
        let result = evaluator.transform(tree);
        assert_eq!(result, 5);

        // Check data is present
        assert_eq!(tree.data().unwrap(), "Arena-allocated 🚀");
    }

    /// Evaluator for arena-allocated trees.
    struct ArenaEvaluator;

    impl<'arena> TreeTransformer<ArenaTreeBuilder<'arena>> for ArenaEvaluator {
        type ReturnType = i32;

        fn builder(&self) -> ArenaTreeBuilder<'arena> {
            // We don't actually need the builder for evaluation
            // This shows a limitation - maybe builder() should be optional?
            panic!("Evaluator doesn't need a builder")
        }

        fn transform(&self, tree: &'arena TreeData<ArenaTreeBuilder<'arena>>) -> Self::ReturnType {
            match tree.view() {
                TreeKind::Num(n) => n,
                TreeKind::Add(left, right) => self.transform(left) + self.transform(right),
                TreeKind::Mul(left, right) => self.transform(left) * self.transform(right),
                TreeKind::Neg(inner) => -self.transform(inner),
            }
        }
    }

    #[test]
    fn negate_numbers() {
        let b = BoxedTreeBuilder;
        let tree = b.build(TreeKind::Add(
            b.build(TreeKind::Num(5)),
            b.build(TreeKind::Num(10)),
        ));
        let transformer = NegateNumbers::new();
        let result = transformer.transform(tree);
        assert_eq!(
            result.view(),
            TreeKind::Add(b.build(TreeKind::Num(-5)), b.build(TreeKind::Num(-10)))
        );
    }

    #[test]
    fn constant_folder() {
        let tree = make_example_tree();
        // (2 + 3) * -(4 + 5) should fold to 5 * -9 = -45
        let folder = ConstantFolder::new();
        let result = folder.transform(tree);
        assert_eq!(result.view(), TreeKind::Num(-45));
    }

    #[test]
    fn partial_constant_fold() {
        let b = BoxedTreeBuilder;
        // Add(2, Neg(x)) where x is unknown - should only fold the 2
        // For this test, we'll use a tree with mixed constants and non-constants
        let tree = b.build(TreeKind::Add(
            b.build(TreeKind::Add(
                b.build(TreeKind::Num(2)),
                b.build(TreeKind::Num(3)),
            )),
            b.build(TreeKind::Neg(
                b.build(TreeKind::Neg(b.build(TreeKind::Num(5)))),
            )),
        ));

        let folder = ConstantFolder::new();
        let result = folder.transform(tree);

        // Should fold to: Add(5, 5) -> 10
        assert_eq!(result.view(), TreeKind::Num(10));
    }
}
