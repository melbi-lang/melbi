//! **Expected: compiles.** The design the crate actually adopted, in miniature:
//! one descriptor per (tree × stage), each with its own data *and* its own kind
//! enum, and one builder that is purely a storage strategy.
//!
//! This file goes further than `parser/src` does today — it has a typed stage,
//! which the crate does not yet — so it is the executable record that the design
//! carries across a stage boundary before that code is written.
//!
//! Consequences, all checked below:
//!  - `TreeDescriptor::Data` is a plain associated type — no builder, no GAT.
//!  - `TreeBuilder` loses `Ty`/`Stage` and becomes purely a storage strategy.
//!  - A pass pays *zero* where-clauses beyond `B: TreeBuilder`.
//!  - The parsed and typed trees need not have corresponding nodes: literals
//!    fold away entirely, and `Local` exists only after type-checking.
//!  - One builder can host both stages, because a builder is now only storage.

use std::fmt::Debug;
use std::hash::Hash;
use std::ops::Deref;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Span(pub usize, pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeId(pub u32);

/// Stand-in for a folded constant.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
}

// --- the two traits ---------------------------------------------------------

/// One tree of the AST, at one compiler stage.
pub trait TreeDescriptor: Sized + 'static {
    /// Data on every node of this tree. Concrete: the stage is baked into the
    /// descriptor, so nothing needs to vary.
    type Data: Clone + Debug + PartialEq;
    /// The navigational enum. Indexed by the builder because kinds hold trees.
    type Kind<B: TreeBuilder>: Clone + Debug + PartialEq;
}

/// Purely a storage strategy: arena, heap, interning. Knows nothing about
/// stages or node data.
pub trait TreeBuilder: Sized + Clone + Debug + Eq + Hash {
    type Handle<D: TreeDescriptor>: AsRef<TreeNode<Self, D>> + Clone + Debug;
    type List<D: TreeDescriptor>: Deref<Target = [Tree<Self, D>]> + Clone + Debug + PartialEq;
    type Str: AsRef<str> + Clone + Debug + Eq;

    fn alloc<D: TreeDescriptor>(&self, node: TreeNode<Self, D>) -> Self::Handle<D>;
    fn alloc_list<D: TreeDescriptor>(
        &self,
        items: impl IntoIterator<Item = Tree<Self, D>, IntoIter: ExactSizeIterator>,
    ) -> Self::List<D>;
    fn alloc_str(&self, s: &str) -> Self::Str;
}

// --- Tree / TreeNode (unchanged machinery, shared by every descriptor) -------

pub struct Tree<B: TreeBuilder, D: TreeDescriptor>(B::Handle<D>);

impl<B: TreeBuilder, D: TreeDescriptor> Tree<B, D> {
    pub fn node(&self) -> &TreeNode<B, D> {
        self.0.as_ref()
    }
    pub fn data(&self) -> &D::Data {
        &self.node().data
    }
    pub fn kind(&self) -> &D::Kind<B> {
        &self.node().kind
    }
}

pub struct TreeNode<B: TreeBuilder, D: TreeDescriptor> {
    data: D::Data,
    kind: D::Kind<B>,
}

impl<B: TreeBuilder, D: TreeDescriptor> TreeNode<B, D> {
    pub fn new(data: D::Data, kind: D::Kind<B>) -> Self {
        Self { data, kind }
    }
    pub fn alloc(self, builder: &B) -> Tree<B, D> {
        Tree(builder.alloc(self))
    }
}

impl<B: TreeBuilder, D: TreeDescriptor> AsRef<TreeNode<B, D>> for TreeNode<B, D> {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl<B: TreeBuilder, D: TreeDescriptor> Clone for Tree<B, D> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<B: TreeBuilder, D: TreeDescriptor> Copy for Tree<B, D> where B::Handle<D>: Copy {}
impl<B: TreeBuilder, D: TreeDescriptor> Debug for Tree<B, D> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.node().fmt(f)
    }
}
impl<B: TreeBuilder, D: TreeDescriptor> PartialEq for Tree<B, D> {
    fn eq(&self, other: &Self) -> bool {
        self.node() == other.node()
    }
}
impl<B: TreeBuilder, D: TreeDescriptor> Clone for TreeNode<B, D> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            kind: self.kind.clone(),
        }
    }
}
impl<B: TreeBuilder, D: TreeDescriptor> Debug for TreeNode<B, D> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TreeNode")
            .field("data", &self.data)
            .field("kind", &self.kind)
            .finish()
    }
}
impl<B: TreeBuilder, D: TreeDescriptor> PartialEq for TreeNode<B, D> {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data && self.kind == other.kind
    }
}

pub trait Visit<B: TreeBuilder, D: TreeDescriptor, Ctx> {
    type Output;
    fn visit(&self, data: &D::Data, ctx: &mut Ctx) -> Self::Output;
}

impl<B: TreeBuilder, D: TreeDescriptor> Tree<B, D> {
    pub fn visit<Ctx>(&self, ctx: &mut Ctx) -> <D::Kind<B> as Visit<B, D, Ctx>>::Output
    where
        D::Kind<B>: Visit<B, D, Ctx>,
    {
        self.kind().visit(self.data(), ctx)
    }
}

// =============================================================================
// The parsed stage
// =============================================================================

pub struct ParsedExpr;
pub struct ParsedLit;
pub struct ParsedPat;

#[derive(Clone, Debug, PartialEq)]
pub struct ParsedData {
    pub span: Span,
}

/// Pattern data carries a parse-time fact the other trees have no use for.
#[derive(Clone, Debug, PartialEq)]
pub struct ParsedPatData {
    pub span: Span,
    pub binds_a_name: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParsedExprKind<B: TreeBuilder> {
    /// Only exists before type-checking: folded away below.
    Literal(Tree<B, ParsedLit>),
    /// A *name*, unresolved.
    Ident(B::Str),
    Add(Tree<B, ParsedExpr>, Tree<B, ParsedExpr>),
    /// `x as T` with the annotation still unresolved — no typed counterpart.
    UnresolvedCast { expr: Tree<B, ParsedExpr>, ty_name: B::Str },
    Match {
        scrutinee: Tree<B, ParsedExpr>,
        arms: B::List<ParsedPat>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParsedLitKind<B: TreeBuilder> {
    /// The unit suffix is the back-edge into the expression tree.
    Int { value: i64, suffix: Option<Tree<B, ParsedExpr>> },
    Float { value: f64, suffix: Option<Tree<B, ParsedExpr>> },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParsedPatKind<B: TreeBuilder> {
    Wildcard,
    Binding(B::Str),
    Literal(Tree<B, ParsedLit>),
    Tuple(B::List<ParsedPat>),
}

impl TreeDescriptor for ParsedExpr {
    type Data = ParsedData;
    type Kind<B: TreeBuilder> = ParsedExprKind<B>;
}
impl TreeDescriptor for ParsedLit {
    type Data = ParsedData;
    type Kind<B: TreeBuilder> = ParsedLitKind<B>;
}
impl TreeDescriptor for ParsedPat {
    type Data = ParsedPatData;
    type Kind<B: TreeBuilder> = ParsedPatKind<B>;
}

// =============================================================================
// The typed stage — a different set of enums, and one fewer tree
// =============================================================================

pub struct TypedExpr;
pub struct TypedPat;
// NOTE: there is deliberately no `TypedLit`. Literals denote values, so they
// fold into `Constant`. The tree simply does not exist after type-checking.

#[derive(Clone, Debug, PartialEq)]
pub struct TypedData {
    pub span: Span,
    pub ty: TypeId,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TypedPatData {
    pub span: Span,
    pub ty: TypeId,
    /// Filled in by exhaustiveness checking — meaningless before it runs.
    pub reachable: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedExprKind<B: TreeBuilder> {
    /// Only exists *after* type-checking. No parsed counterpart.
    Constant(Value),
    /// A resolved slot, not a name.
    Local(u32),
    Add(Tree<B, TypedExpr>, Tree<B, TypedExpr>),
    /// The target type moved into `TypedData::ty`, so a cast carries only its operand.
    Cast { expr: Tree<B, TypedExpr> },
    Match {
        scrutinee: Tree<B, TypedExpr>,
        arms: B::List<TypedPat>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedPatKind<B: TreeBuilder> {
    Wildcard,
    /// Binds a slot, not a name.
    Binding(u32),
    Constant(Value),
    Tuple(B::List<TypedPat>),
}

impl TreeDescriptor for TypedExpr {
    type Data = TypedData;
    type Kind<B: TreeBuilder> = TypedExprKind<B>;
}
impl TreeDescriptor for TypedPat {
    type Data = TypedPatData;
    type Kind<B: TreeBuilder> = TypedPatKind<B>;
}

// =============================================================================
// A pass over the parsed tree: NO where-clauses beyond `B: TreeBuilder`
// =============================================================================

#[derive(Default)]
pub struct Census {
    pub exprs: usize,
    pub lits: usize,
    pub pats: usize,
    pub ints: i64,
    pub last_span: Span,
}

impl<B: TreeBuilder> Visit<B, ParsedExpr, Census> for ParsedExprKind<B> {
    type Output = ();
    fn visit(&self, data: &ParsedData, ctx: &mut Census) {
        ctx.exprs += 1;
        ctx.last_span = data.span;
        match self {
            ParsedExprKind::Literal(l) => l.visit(ctx),
            ParsedExprKind::Ident(_) => {}
            ParsedExprKind::Add(l, r) => {
                l.visit(ctx);
                r.visit(ctx);
            }
            ParsedExprKind::UnresolvedCast { expr, .. } => expr.visit(ctx),
            ParsedExprKind::Match { scrutinee, arms } => {
                scrutinee.visit(ctx);
                for a in arms.iter() {
                    a.visit(ctx);
                }
            }
        }
    }
}

impl<B: TreeBuilder> Visit<B, ParsedLit, Census> for ParsedLitKind<B> {
    type Output = ();
    fn visit(&self, _data: &ParsedData, ctx: &mut Census) {
        ctx.lits += 1;
        match self {
            ParsedLitKind::Int { value, suffix } => {
                ctx.ints += value;
                if let Some(s) = suffix {
                    s.visit(ctx);
                }
            }
            ParsedLitKind::Float { suffix, .. } => {
                if let Some(s) = suffix {
                    s.visit(ctx);
                }
            }
        }
    }
}

impl<B: TreeBuilder> Visit<B, ParsedPat, Census> for ParsedPatKind<B> {
    type Output = ();
    fn visit(&self, data: &ParsedPatData, ctx: &mut Census) {
        ctx.pats += if data.binds_a_name { 101 } else { 1 };
        match self {
            ParsedPatKind::Wildcard | ParsedPatKind::Binding(_) => {}
            ParsedPatKind::Literal(l) => l.visit(ctx),
            ParsedPatKind::Tuple(items) => {
                for i in items.iter() {
                    i.visit(ctx);
                }
            }
        }
    }
}

// =============================================================================
// Lowering: parsed -> typed. Crosses stages *and* changes the set of trees.
// =============================================================================

pub struct Lower<Out: TreeBuilder> {
    pub out: Out,
    pub next_ty: u32,
    pub next_slot: u32,
}

impl<Out: TreeBuilder> Lower<Out> {
    fn ty(&mut self) -> TypeId {
        self.next_ty += 1;
        TypeId(self.next_ty)
    }
    fn slot(&mut self) -> u32 {
        self.next_slot += 1;
        self.next_slot
    }
}

impl<In: TreeBuilder, Out: TreeBuilder> Visit<In, ParsedExpr, Lower<Out>> for ParsedExprKind<In> {
    type Output = Tree<Out, TypedExpr>;
    fn visit(&self, data: &ParsedData, ctx: &mut Lower<Out>) -> Tree<Out, TypedExpr> {
        let kind = match self {
            // The literal tree folds away here: a `Tree<In, ParsedLit>` becomes a
            // `Value` inline, and no typed literal node is ever allocated.
            ParsedExprKind::Literal(l) => TypedExprKind::Constant(l.visit(ctx)),
            ParsedExprKind::Ident(_) => TypedExprKind::Local(ctx.slot()),
            ParsedExprKind::Add(l, r) => TypedExprKind::Add(l.visit(ctx), r.visit(ctx)),
            // Unresolved becomes resolved: the target type moves into the data.
            ParsedExprKind::UnresolvedCast { expr, .. } => {
                TypedExprKind::Cast { expr: expr.visit(ctx) }
            }
            ParsedExprKind::Match { scrutinee, arms } => {
                let scrutinee = scrutinee.visit(ctx);
                let out = ctx.out.clone();
                TypedExprKind::Match {
                    scrutinee,
                    arms: out.alloc_list(arms.iter().map(|a| a.visit(ctx))),
                }
            }
        };
        let data = TypedData { span: data.span, ty: ctx.ty() };
        TreeNode::new(data, kind).alloc(&ctx.out)
    }
}

/// Note the output type: lowering a literal produces a `Value`, not a tree.
impl<In: TreeBuilder, Out: TreeBuilder> Visit<In, ParsedLit, Lower<Out>> for ParsedLitKind<In> {
    type Output = Value;
    fn visit(&self, _data: &ParsedData, _ctx: &mut Lower<Out>) -> Value {
        match self {
            ParsedLitKind::Int { value, .. } => Value::Int(*value),
            ParsedLitKind::Float { value, .. } => Value::Float(*value),
        }
    }
}

impl<In: TreeBuilder, Out: TreeBuilder> Visit<In, ParsedPat, Lower<Out>> for ParsedPatKind<In> {
    type Output = Tree<Out, TypedPat>;
    fn visit(&self, data: &ParsedPatData, ctx: &mut Lower<Out>) -> Tree<Out, TypedPat> {
        let kind = match self {
            ParsedPatKind::Wildcard => TypedPatKind::Wildcard,
            ParsedPatKind::Binding(_) => TypedPatKind::Binding(ctx.slot()),
            ParsedPatKind::Literal(l) => TypedPatKind::Constant(l.visit(ctx)),
            ParsedPatKind::Tuple(items) => {
                let out = ctx.out.clone();
                TypedPatKind::Tuple(out.alloc_list(items.iter().map(|i| i.visit(ctx))))
            }
        };
        let data = TypedPatData {
            span: data.span,
            ty: ctx.ty(),
            reachable: true,
        };
        TreeNode::new(data, kind).alloc(&ctx.out)
    }
}

// =============================================================================
// One storage strategy, used for both stages
// =============================================================================

pub struct Arena;
impl Arena {
    fn alloc<T>(&self, v: T) -> &'static T {
        Box::leak(Box::new(v))
    }
    fn alloc_slice<T>(&self, items: impl IntoIterator<Item = T>) -> &'static [T] {
        Box::leak(items.into_iter().collect::<Vec<_>>().into_boxed_slice())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArenaAst;

impl TreeBuilder for ArenaAst {
    type Handle<D: TreeDescriptor> = &'static TreeNode<Self, D>;
    type List<D: TreeDescriptor> = &'static [Tree<Self, D>];
    type Str = &'static str;

    fn alloc<D: TreeDescriptor>(&self, node: TreeNode<Self, D>) -> Self::Handle<D> {
        Arena.alloc(node)
    }
    fn alloc_list<D: TreeDescriptor>(
        &self,
        items: impl IntoIterator<Item = Tree<Self, D>, IntoIter: ExactSizeIterator>,
    ) -> Self::List<D> {
        Arena.alloc_slice(items)
    }
    fn alloc_str(&self, s: &str) -> Self::Str {
        Box::leak(s.to_string().into_boxed_str())
    }
}

// --- exercise ---------------------------------------------------------------

pub fn demo() {
    let b = ArenaAst;
    let pd = ParsedData { span: Span(0, 0) };
    let e = |k| TreeNode::<ArenaAst, ParsedExpr>::new(pd.clone(), k).alloc(&b);
    let l = |k| TreeNode::<ArenaAst, ParsedLit>::new(pd.clone(), k).alloc(&b);
    let p = |k, binds| {
        TreeNode::<ArenaAst, ParsedPat>::new(
            ParsedPatData { span: Span(0, 0), binds_a_name: binds },
            k,
        )
        .alloc(&b)
    };

    // 9.81`m/s^2` -- the suffix crosses back into the expression tree.
    let m = e(ParsedExprKind::Ident(b.alloc_str("m")));
    let g = l(ParsedLitKind::Float { value: 9.81, suffix: Some(m) });
    let one = l(ParsedLitKind::Int { value: 1, suffix: None });

    let arms = b.alloc_list([
        p(ParsedPatKind::Wildcard, false),
        p(ParsedPatKind::Literal(one), true),
    ]);
    let root = e(ParsedExprKind::Match {
        scrutinee: e(ParsedExprKind::Literal(g)),
        arms,
    });

    let mut census = Census::default();
    root.visit(&mut census);
    assert_eq!((census.exprs, census.lits, census.pats, census.ints), (3, 2, 102, 1));

    // Lower into the typed stage — same builder, same arena, different trees.
    let mut lower = Lower { out: ArenaAst, next_ty: 0, next_slot: 0 };
    let typed: Tree<ArenaAst, TypedExpr> = root.visit(&mut lower);

    // The literal tree is gone: `9.81` is now a constant inside the typed expr.
    match typed.kind() {
        TypedExprKind::Match { scrutinee, arms } => {
            assert_eq!(*scrutinee.kind(), TypedExprKind::Constant(Value::Float(9.81)));
            assert_eq!(arms.len(), 2);
            assert_eq!(*arms[1].kind(), TypedPatKind::Constant(Value::Int(1)));
            assert!(arms[1].data().reachable);
        }
        other => panic!("expected a match, got {other:?}"),
    }
    // Four typed nodes were allocated: match, scrutinee, and two patterns. The
    // two literals produced no nodes at all.
    assert_eq!(lower.next_ty, 4);
}
