// Minimal independent repro of the projection-inference claim.
pub trait TreeBuilder: Sized { type TreeKind; }
pub struct Tree<B: TreeBuilder>(core::marker::PhantomData<B>);

pub trait System: Sized {
    type Expr: TreeBuilder<TreeKind = ExprKind<Self>>;
}
pub enum ExprKind<S: System> { Neg(Tree<S::Expr>), Lit }

// A pass over the tree, exactly as a real visitor would be written.
fn count<S: System>(e: &Tree<S::Expr>) -> usize {
    let _ = e;
    // recursive call on a child, no turbofish:
    count(e)
}
fn main() {}
