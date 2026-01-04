use crate::*;

// --- 1. User Data & Kind ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span(pub usize, pub usize);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr<B: TreeBuilder> {
    Lit(i32),
    // We now use Tree<B> instead of B::Handle directly.
    // This is much cleaner for the user.
    Add(Tree<B>, Tree<B>),
}

// --- 2. The Builder Implementation ---

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct HeapBuilder;

impl TreeBuilder for HeapBuilder {
    type TreeData = Span;
    type TreeKind = Expr<Self>;
    type Handle = Box<TreeNode<Self>>;

    fn build(&self, node: TreeNode<Self>) -> Self::Handle {
        Box::new(node)
    }

    fn node(handle: &Self::Handle) -> &TreeNode<Self> {
        handle.as_ref()
    }
}

// --- 3. Fold Implementation ---

// Helper to reduce boilerplate.
// Accepts &Tree<In> and returns Tree<Out>
fn fold_tree<In, Out>(input: &In, output: &Out, tree: &Tree<In>) -> Tree<Out>
where
    In: TreeBuilder<TreeData = Span>,
    Out: TreeBuilder<TreeData = Span, TreeKind = Expr<Out>>,
    TreeNode<In>: Fold<In, Out>,
{
    let node = tree.node();
    node.fold(input, output).alloc(output)
}

impl Fold<HeapBuilder, HeapBuilder> for Expr<HeapBuilder> {
    fn fold(&self, input: &HeapBuilder, output: &HeapBuilder) -> Expr<HeapBuilder> {
        match self {
            Expr::Lit(x) => Expr::Lit(*x),
            Expr::Add(l, r) => {
                // l and r are of type Tree<HeapBuilder>
                let new_l = fold_tree(input, output, l);
                let new_r = fold_tree(input, output, r);
                Expr::Add(new_l, new_r)
            }
        }
    }
}

// --- 4. Test ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heap_builder() {
        let input = HeapBuilder;

        // 1. Construction
        let l1 = TreeNode(Span(0, 1), Expr::Lit(10)).alloc(&input);
        let l2 = TreeNode(Span(2, 3), Expr::Lit(20)).alloc(&input);

        // Notice how clean this syntax is now:
        let root = TreeNode(Span(0, 3), Expr::Add(l1.clone(), l2.clone())).alloc(&input);

        // 2. Resolve / Inspection
        // We access the handle to resolve
        let node = root.node();
        assert_eq!(*node.data(), Span(0, 3));

        if let Expr::Add(child1, _) = node.kind() {
            // Recursively resolve child1
            let child = child1.node();
            assert_eq!(*child.data(), Span(0, 1));
            assert_eq!(*child.kind(), Expr::Lit(10));
        } else {
            panic!("Wrong kind");
        }

        // 3. Folding (Deep Clone)
        // We pass the builder as both input and output
        let output = HeapBuilder;
        let node = root.node();
        let cloned_kind = node.kind().fold(&input, &output);
        let cloned_root = TreeNode(*node.data(), cloned_kind).alloc(&output);

        assert_eq!(root, cloned_root);
    }
}
