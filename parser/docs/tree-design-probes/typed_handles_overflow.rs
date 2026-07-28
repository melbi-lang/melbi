//! **Expected: E0275.** One builder with a typed handle per tree, where the node
//! is inlined into the handle (`&'a (Data, Kind)`) instead of going through a
//! separate node type.
//!
//! This is the shape everyone reaches for first, and nothing about the
//! definitions hints that it is wrong — the failure is a property of how the
//! trait solver unrolls the bound, not of the design. Bounding the handle
//! `PartialEq` asks it to prove `Expr<ArenaAst>: PartialEq`, whose proof needs
//! `ArenaAst::ExprTree: PartialEq`, which is where it started. Expect to
//! rediscover this by reading rustc rather than by reasoning about the types.
//!
//! The fix is the `Tree`/`TreeNode` split in `tree_builder.rs`: the bounds move
//! onto the descriptor, where they are discharged once at a concrete
//! descriptor, and the hand-written unconditional impls on `Tree` stop the
//! recursion. See `descriptor_design.rs` for the version that works.

use std::fmt::Debug;

// --- a stand-in arena, so this file has no dependencies ---------------------
pub struct Arena;
impl Arena {
    #[allow(clippy::mut_from_ref)]
    fn alloc<T>(&self, value: T) -> &'static T {
        Box::leak(Box::new(value))
    }
    fn alloc_slice<T>(&self, items: impl IntoIterator<Item = T>) -> &'static [T] {
        Box::leak(items.into_iter().collect::<Vec<_>>().into_boxed_slice())
    }
}

// --- the builder ------------------------------------------------------------

pub trait AstBuilder: Sized + Clone + Debug + PartialEq {
    type Str: AsRef<str> + Clone + Debug;

    type ExprData: Clone + Debug + PartialEq;
    type PatData: Clone + Debug + PartialEq;

    type ExprTree: Clone + Debug + PartialEq;
    type PatTree: Clone + Debug + PartialEq;

    type ExprList: std::ops::Deref<Target = [Self::ExprTree]> + Clone + Debug + PartialEq;
    type PatList: std::ops::Deref<Target = [Self::PatTree]> + Clone + Debug + PartialEq;

    fn alloc_expr(&self, data: Self::ExprData, kind: Expr<Self>) -> Self::ExprTree;
    fn alloc_pat(&self, data: Self::PatData, kind: Pattern<Self>) -> Self::PatTree;

    fn resolve_expr<'a>(&self, h: &'a Self::ExprTree) -> (&'a Self::ExprData, &'a Expr<Self>);
    fn resolve_pat<'a>(&self, h: &'a Self::PatTree) -> (&'a Self::PatData, &'a Pattern<Self>);

    fn alloc_expr_list(&self, items: impl IntoIterator<Item = Self::ExprTree>) -> Self::ExprList;
    fn alloc_pat_list(&self, items: impl IntoIterator<Item = Self::PatTree>) -> Self::PatList;
}

// --- the node enums, parameterized directly by the builder ------------------

#[derive(Debug, Clone, PartialEq)]
pub enum Expr<B: AstBuilder> {
    Lit(Lit<B>),
    Var(B::Str),
    Add(B::ExprTree, B::ExprTree),
    Array(B::ExprList),
    Match { scrutinee: B::ExprTree, arms: B::PatList },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern<B: AstBuilder> {
    Wildcard,
    Binding(B::Str),
    Lit(Lit<B>),
    Tuple(B::PatList),
}

/// Shared, un-allocated, and mutually recursive with `Expr` through the suffix.
#[derive(Debug, Clone, PartialEq)]
pub enum Lit<B: AstBuilder> {
    Int { value: i64, suffix: Option<B::ExprTree> },
    Str(B::Str),
}

// --- a concrete arena builder ----------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span(pub usize, pub usize);

#[derive(Clone, Debug, PartialEq)]
pub struct ArenaAst;

impl AstBuilder for ArenaAst {
    type Str = &'static str;

    type ExprData = Span;
    type PatData = Span;

    type ExprTree = &'static (Span, Expr<Self>);
    type PatTree = &'static (Span, Pattern<Self>);

    type ExprList = &'static [Self::ExprTree];
    type PatList = &'static [Self::PatTree];

    fn alloc_expr(&self, data: Span, kind: Expr<Self>) -> Self::ExprTree {
        Arena.alloc((data, kind))
    }
    fn alloc_pat(&self, data: Span, kind: Pattern<Self>) -> Self::PatTree {
        Arena.alloc((data, kind))
    }
    fn resolve_expr<'a>(&self, h: &'a Self::ExprTree) -> (&'a Span, &'a Expr<Self>) {
        (&h.0, &h.1)
    }
    fn resolve_pat<'a>(&self, h: &'a Self::PatTree) -> (&'a Span, &'a Pattern<Self>) {
        (&h.0, &h.1)
    }
    fn alloc_expr_list(&self, items: impl IntoIterator<Item = Self::ExprTree>) -> Self::ExprList {
        Arena.alloc_slice(items)
    }
    fn alloc_pat_list(&self, items: impl IntoIterator<Item = Self::PatTree>) -> Self::PatList {
        Arena.alloc_slice(items)
    }
}

// --- a pass that crosses categories -----------------------------------------

#[derive(Default)]
pub struct Census {
    pub exprs: usize,
    pub pats: usize,
}

pub fn walk_expr<B: AstBuilder>(b: &B, h: &B::ExprTree, c: &mut Census) {
    c.exprs += 1;
    let (_data, kind) = b.resolve_expr(h);
    match kind {
        Expr::Lit(lit) => walk_lit(b, lit, c),
        Expr::Var(_) => {}
        Expr::Add(l, r) => {
            walk_expr(b, l, c);
            walk_expr(b, r, c);
        }
        Expr::Array(items) => {
            for item in items.iter() {
                walk_expr(b, item, c);
            }
        }
        Expr::Match { scrutinee, arms } => {
            walk_expr(b, scrutinee, c);
            for arm in arms.iter() {
                walk_pat(b, arm, c);
            }
        }
    }
}

pub fn walk_pat<B: AstBuilder>(b: &B, h: &B::PatTree, c: &mut Census) {
    c.pats += 1;
    let (_data, kind) = b.resolve_pat(h);
    match kind {
        Pattern::Wildcard | Pattern::Binding(_) => {}
        Pattern::Lit(lit) => walk_lit(b, lit, c),
        Pattern::Tuple(items) => {
            for item in items.iter() {
                walk_pat(b, item, c);
            }
        }
    }
}

pub fn walk_lit<B: AstBuilder>(b: &B, lit: &Lit<B>, c: &mut Census) {
    if let Lit::Int { suffix: Some(s), .. } = lit {
        walk_expr(b, s, c);
    }
}

// --- exercise it ------------------------------------------------------------

pub fn build() -> Census {
    let b = ArenaAst;
    let one = b.alloc_expr(Span(0, 1), Expr::Lit(Lit::Int { value: 1, suffix: None }));
    let x = b.alloc_expr(Span(2, 3), Expr::Var("x"));
    let sum = b.alloc_expr(Span(0, 3), Expr::Add(one, x));
    let arms = b.alloc_pat_list([b.alloc_pat(Span(4, 5), Pattern::Wildcard)]);
    let root = b.alloc_expr(Span(0, 9), Expr::Match { scrutinee: sum, arms });

    let mut census = Census::default();
    walk_expr(&b, &root, &mut census);
    census
}
