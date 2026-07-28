pub trait TreeBuilder: Sized { type TreeKind; }
pub struct Tree<B: TreeBuilder>(core::marker::PhantomData<B>);

pub trait ExprBuilder: TreeBuilder<TreeKind = ExprKind<Self>> { type Lit: LiteralBuilder<Expr = Self>; }
pub trait LiteralBuilder: TreeBuilder<TreeKind = LiteralKind<Self>> { type Expr: ExprBuilder<Lit = Self>; }
pub enum ExprKind<B: ExprBuilder> { Lit(Tree<B::Lit>) }
pub enum LiteralKind<B: LiteralBuilder> { Int(i64), Suffixed(Tree<B::Expr>) }

// Attempt: one builder type playing both roles.
pub struct Both;
impl TreeBuilder for Both { type TreeKind = ExprKind<Self>; }
impl ExprBuilder for Both { type Lit = Self; }
impl LiteralBuilder for Both { type Expr = Self; }
