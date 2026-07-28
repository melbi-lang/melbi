use std::rc::Rc;
pub trait TreeBuilder: Sized {
    type TreeKind;
    type TreeHandle: AsRef<TreeNode<Self>> + Clone;
    fn alloc(&self, n: TreeNode<Self>) -> Self::TreeHandle;
}
pub struct Tree<B: TreeBuilder>(B::TreeHandle);
pub struct TreeNode<B: TreeBuilder>(B::TreeKind);
impl<B: TreeBuilder> TreeNode<B> { fn kind(&self) -> &B::TreeKind { &self.0 } }
impl<B: TreeBuilder> AsRef<TreeNode<B>> for TreeNode<B> { fn as_ref(&self) -> &Self { self } }
impl<B: TreeBuilder> Tree<B> {
    fn new(b: &B, k: B::TreeKind) -> Self { Tree(b.alloc(TreeNode(k))) }
    fn node(&self) -> &TreeNode<B> { self.0.as_ref() }
    fn kind(&self) -> &B::TreeKind { self.node().kind() }
}

pub trait ExprBuilder: TreeBuilder<TreeKind = ExprKind<Self>> { type Lit: LitBuilder<Expr = Self>; }
pub trait LitBuilder: TreeBuilder<TreeKind = LitKind<Self>> { type Expr: ExprBuilder<Lit = Self>; }
pub enum ExprKind<B: ExprBuilder> { Neg(Tree<B>), Lit(Tree<B::Lit>) }
pub enum LitKind<B: LitBuilder> { Int(i64), Suffixed(i64, Tree<B::Expr>) }

// --- concrete instantiation: does the mutual fixed point resolve? ---
#[derive(Clone)] pub struct E;
#[derive(Clone)] pub struct L;
impl TreeBuilder for E {
    type TreeKind = ExprKind<Self>;
    type TreeHandle = Rc<TreeNode<Self>>;
    fn alloc(&self, n: TreeNode<Self>) -> Self::TreeHandle { Rc::new(n) }
}
impl ExprBuilder for E { type Lit = L; }
impl TreeBuilder for L {
    type TreeKind = LitKind<Self>;
    type TreeHandle = Rc<TreeNode<Self>>;
    fn alloc(&self, n: TreeNode<Self>) -> Self::TreeHandle { Rc::new(n) }
}
impl LitBuilder for L { type Expr = E; }

// --- Visit, the shape we actually use ---
pub trait Visit<B: TreeBuilder, C> { type Output; fn visit(&self, ctx: &mut C) -> Self::Output; }
impl<B: TreeBuilder, C> Visit<B, C> for Tree<B> where B::TreeKind: Visit<B, C> {
    type Output = <B::TreeKind as Visit<B, C>>::Output;
    fn visit(&self, ctx: &mut C) -> Self::Output { self.kind().visit(ctx) }
}

#[derive(Default)] pub struct Count(usize);
// Two blanket impls, one per builder trait -- do they overlap?
impl<B: ExprBuilder> Visit<B, Count> for ExprKind<B> {
    type Output = ();
    fn visit(&self, ctx: &mut Count) {
        ctx.0 += 1;
        match self { ExprKind::Neg(i) => i.visit(ctx), ExprKind::Lit(l) => l.visit(ctx) }
    }
}
impl<B: LitBuilder> Visit<B, Count> for LitKind<B> {
    type Output = ();
    fn visit(&self, ctx: &mut Count) {
        ctx.0 += 1;
        match self { LitKind::Int(_) => {}, LitKind::Suffixed(_, s) => s.visit(ctx) }
    }
}

pub fn demo() -> usize {
    let (e, l) = (E, L);
    let inner = Tree::new(&e, ExprKind::Neg(Tree::new(&e, ExprKind::Lit(Tree::new(&l, LitKind::Int(1))))));
    let mut c = Count::default();
    inner.visit(&mut c);
    c.0
}
