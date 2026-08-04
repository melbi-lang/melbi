//! Test-specific tree types and implementations demonstrating the visitor pattern.

use core::fmt::Debug;
use core::hash::Hash;

use bumpalo::Bump;

use super::{TreeBuilder, TreeTransformer, TreeView};

// === Test-specific tree types ===

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

impl<B: TreeBuilder> TreeKind<B>
where
    B::TreeViewRepr: TreeView<B, Kind = TreeKind<B>>,
{
    /// Deep structural equality check.
    ///
    /// Since TreeKind contains TreeViewRepr (references), we can't easily
    /// construct tree literals for comparison. This recursively compares structure.
    pub fn structural_eq(&self, other: &Self) -> bool
    where
        B::TreeViewRepr: Copy,
    {
        match (self, other) {
            (TreeKind::Num(a), TreeKind::Num(b)) => a == b,
            (TreeKind::Add(l1, r1), TreeKind::Add(l2, r2))
            | (TreeKind::Mul(l1, r1), TreeKind::Mul(l2, r2)) => {
                l1.view().structural_eq(&l2.view()) && r1.view().structural_eq(&r2.view())
            }
            (TreeKind::Neg(a), TreeKind::Neg(b)) => a.view().structural_eq(&b.view()),
            _ => false,
        }
    }

    /// Pattern matching helper: check if this is a Num with expected value.
    pub fn is_num(&self, expected: i32) -> bool {
        matches!(self, TreeKind::Num(n) if *n == expected)
    }

    /// Check if this is an Add node.
    pub fn is_add(&self) -> bool {
        matches!(self, TreeKind::Add(_, _))
    }

    /// Check if this is a Mul node.
    pub fn is_mul(&self) -> bool {
        matches!(self, TreeKind::Mul(_, _))
    }

    /// Check if this is a Neg node.
    pub fn is_neg(&self) -> bool {
        matches!(self, TreeKind::Neg(_))
    }
}

// === Example implementations ===

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
struct BoxedTreeBuilder;

impl TreeBuilder for BoxedTreeBuilder {
    type TreeViewRepr = Box<TreeData<Self>>;
    type DataRepr = String;
}

impl BoxedTreeBuilder {
    fn build(self, kind: TreeKind<Self>) -> Box<TreeData<Self>> {
        Box::new(TreeData {
            kind,
            data: "Hello 🖖".to_string(),
        })
    }
}

impl<B: TreeBuilder> TreeView<B> for Box<TreeData<B>> {
    type Kind = TreeKind<B>;

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
impl<'arena> PartialEq for ArenaTreeBuilder<'arena> {
    fn eq(&self, other: &Self) -> bool {
        core::ptr::eq(self.arena, other.arena)
    }
}

impl<'arena> Eq for ArenaTreeBuilder<'arena> {}

impl<'arena> core::hash::Hash for ArenaTreeBuilder<'arena> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        core::ptr::hash(self.arena, state);
    }
}

impl<'arena> TreeBuilder for ArenaTreeBuilder<'arena> {
    type TreeViewRepr = &'arena TreeData<Self>;
    type DataRepr = String;
}

impl<'arena> ArenaTreeBuilder<'arena> {
    fn build(self, kind: TreeKind<Self>) -> &'arena TreeData<Self> {
        self.arena.alloc(TreeData {
            kind,
            data: "Arena-allocated 🚀".to_string(),
        })
    }
}

impl<'arena, B: TreeBuilder> TreeView<B> for &'arena TreeData<B> {
    type Kind = TreeKind<B>;

    fn view(self) -> TreeKind<B> {
        self.kind.clone()
    }

    fn data(self) -> Option<B::DataRepr> {
        Some(self.data.clone())
    }
}

// === TreeVisitor ===

pub trait TreeVisitor<B: TreeBuilder>
where
    B::TreeViewRepr: TreeView<B, Kind = TreeKind<B>>,
{
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

/// Example of TreeTransformer with Output = () (acts as a visitor).
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

    fn traverse_with_depth(&mut self, tree: Box<TreeData<BoxedTreeBuilder>>, depth: usize) {
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
    type Output = (); // Visitor: returns nothing, does side effects

    fn transform(&mut self, tree: Box<TreeData<BoxedTreeBuilder>>) -> Self::Output {
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
    type Output = Box<TreeData<BoxedTreeBuilder>>;

    fn transform(&mut self, tree: Box<TreeData<BoxedTreeBuilder>>) -> Self::Output {
        match tree.clone().view() {
            TreeKind::Num(n) => Box::new(TreeData {
                kind: TreeKind::Num(-n),
                data: "Hello 🖖".to_string(),
            }),
            TreeKind::Add(left, right) => {
                let left_t = self.transform(left);
                let right_t = self.transform(right);
                Box::new(TreeData {
                    kind: TreeKind::Add(left_t, right_t),
                    data: "Hello 🖖".to_string(),
                })
            }
            TreeKind::Mul(left, right) => {
                let left_t = self.transform(left);
                let right_t = self.transform(right);
                Box::new(TreeData {
                    kind: TreeKind::Mul(left_t, right_t),
                    data: "Hello 🖖".to_string(),
                })
            }
            TreeKind::Neg(inner) => {
                let inner_t = self.transform(inner);
                Box::new(TreeData {
                    kind: TreeKind::Neg(inner_t),
                    data: "Hello 🖖".to_string(),
                })
            }
        }
    }
}

/// Example evaluator: Transform tree to its computed value.
///
/// This demonstrates Output being different from B::TreeViewRepr.
struct Evaluator;

impl Evaluator {
    fn new() -> Self {
        Self
    }
}

impl TreeTransformer<BoxedTreeBuilder> for Evaluator {
    type Output = i32;

    fn transform(&mut self, tree: Box<TreeData<BoxedTreeBuilder>>) -> Self::Output {
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
    type Output = Box<TreeData<BoxedTreeBuilder>>;

    fn transform(&mut self, tree: Box<TreeData<BoxedTreeBuilder>>) -> Self::Output {
        match tree.clone().view() {
            TreeKind::Num(n) => Box::new(TreeData {
                kind: TreeKind::Num(n),
                data: "Hello 🖖".to_string(),
            }),
            TreeKind::Add(left, right) => {
                let left_t = self.transform(left);
                let right_t = self.transform(right);

                // If both operands are constants, fold them
                match (left_t.clone().view(), right_t.clone().view()) {
                    (TreeKind::Num(l), TreeKind::Num(r)) => Box::new(TreeData {
                        kind: TreeKind::Num(l + r),
                        data: "Hello 🖖".to_string(),
                    }),
                    _ => Box::new(TreeData {
                        kind: TreeKind::Add(left_t, right_t),
                        data: "Hello 🖖".to_string(),
                    }),
                }
            }
            TreeKind::Mul(left, right) => {
                let left_t = self.transform(left);
                let right_t = self.transform(right);

                // If both operands are constants, fold them
                match (left_t.clone().view(), right_t.clone().view()) {
                    (TreeKind::Num(l), TreeKind::Num(r)) => Box::new(TreeData {
                        kind: TreeKind::Num(l * r),
                        data: "Hello 🖖".to_string(),
                    }),
                    _ => Box::new(TreeData {
                        kind: TreeKind::Mul(left_t, right_t),
                        data: "Hello 🖖".to_string(),
                    }),
                }
            }
            TreeKind::Neg(inner) => {
                let inner_t = self.transform(inner);

                // If operand is constant, fold it
                match inner_t.clone().view() {
                    TreeKind::Num(n) => Box::new(TreeData {
                        kind: TreeKind::Num(-n),
                        data: "Hello 🖖".to_string(),
                    }),
                    _ => Box::new(TreeData {
                        kind: TreeKind::Neg(inner_t),
                        data: "Hello 🖖".to_string(),
                    }),
                }
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

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
fn test_node_counter() {
    let tree = make_example_tree();
    let mut counter = NodeCounter::new();
    counter.visit(tree);
    assert_eq!(counter.count, 8); // 2 Adds, 1 Mul, 1 Neg, 4 Nums
}

#[test]
fn test_evaluator() {
    let tree = make_example_tree();
    // (2 + 3) * -(4 + 5) = 5 * -9 = -45
    let mut evaluator = Evaluator::new();
    let result = evaluator.transform(tree);
    assert_eq!(result, -45);
}

#[test]
fn test_max_depth_finder() {
    let tree = make_example_tree();
    // Tree structure: Mul -> Add/Neg -> Num/Add -> Num
    // Max depth is 3 (Mul -> Neg -> Add -> Num)
    let mut finder = MaxDepthFinder::new();
    finder.transform(tree);
    assert_eq!(finder.max_depth(), 3);
}

#[test]
fn test_compare_trees() {
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
fn test_arena_tree_builder() {
    let arena = Bump::new();
    let b = ArenaTreeBuilder::new(&arena);

    // Build a simple tree: 2 + 3
    let tree = b.build(TreeKind::Add(
        b.build(TreeKind::Num(2)),
        b.build(TreeKind::Num(3)),
    ));

    // Evaluate it
    let mut evaluator = ArenaEvaluator;
    let result = evaluator.transform(tree);
    assert_eq!(result, 5);

    // Check data is present
    assert_eq!(tree.data().unwrap(), "Arena-allocated 🚀");
}

/// Evaluator for arena-allocated trees.
struct ArenaEvaluator;

impl<'arena> TreeTransformer<ArenaTreeBuilder<'arena>> for ArenaEvaluator {
    type Output = i32;

    fn transform(&mut self, tree: &'arena TreeData<ArenaTreeBuilder<'arena>>) -> Self::Output {
        match tree.view() {
            TreeKind::Num(n) => n,
            TreeKind::Add(left, right) => self.transform(left) + self.transform(right),
            TreeKind::Mul(left, right) => self.transform(left) * self.transform(right),
            TreeKind::Neg(inner) => -self.transform(inner),
        }
    }
}

#[test]
fn test_negate_numbers() {
    let b = BoxedTreeBuilder;
    let tree = b.build(TreeKind::Add(
        b.build(TreeKind::Num(5)),
        b.build(TreeKind::Num(10)),
    ));
    let mut transformer = NegateNumbers::new();
    let result = transformer.transform(tree);
    assert_eq!(
        result.view(),
        TreeKind::Add(b.build(TreeKind::Num(-5)), b.build(TreeKind::Num(-10)))
    );
}

#[test]
fn test_constant_folder() {
    let tree = make_example_tree();
    // (2 + 3) * -(4 + 5) should fold to 5 * -9 = -45
    let mut folder = ConstantFolder::new();
    let result = folder.transform(tree);
    assert_eq!(result.view(), TreeKind::Num(-45));
}

#[test]
fn test_partial_constant_fold() {
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

    let mut folder = ConstantFolder::new();
    let result = folder.transform(tree);

    // Should fold to: Add(5, 5) -> 10
    assert_eq!(result.view(), TreeKind::Num(10));
}
