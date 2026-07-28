pub trait TreeBuilder: Sized { type TreeKind; }
pub struct Tree<B: TreeBuilder>(core::marker::PhantomData<B>);
impl<B: TreeBuilder> Tree<B> { pub fn kind(&self) -> &B::TreeKind { unimplemented!() } }

pub trait System: Sized {
    type Expr: TreeBuilder<TreeKind = ExprKind<Self>>;
}
pub enum ExprKind<S: System> { Neg(Tree<S::Expr>), Lit }

// The real shape: a trait, with the pass implemented on the Kind.
pub trait Visit<C> { fn visit(&self, ctx: &mut C); }

impl<S: System, C> Visit<C> for Tree<S::Expr> {
    fn visit(&self, ctx: &mut C) { self.kind().visit(ctx) }
}
impl<S: System, C> Visit<C> for ExprKind<S> {
    fn visit(&self, ctx: &mut C) {
        match self {
            ExprKind::Neg(inner) => inner.visit(ctx),  // no turbofish
            ExprKind::Lit => {}
        }
    }
}
