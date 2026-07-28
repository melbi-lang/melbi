// Design B: TreeBuilder parameterized by the System.
pub trait System: Sized {
    type Expr: TreeBuilder<Self>;
    type Lit: TreeBuilder<Self>;
}
pub trait TreeBuilder<S: System>: Sized {
    type TreeKind;
    type TreeHandle: Clone;
}
// Tree wants to name B's associated types, which now live on TreeBuilder<S>.
pub struct Tree<B: TreeBuilder<S>, S: System>(B::TreeHandle);
