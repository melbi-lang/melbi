//! Builds real Melbi expressions across the two mutually recursive trees.
//!
//! The pair of builders here is the point: `ExprAst` and `LitAst` each name the
//! other, both allocate from the same arena, and the passes below cross freely
//! between the trees with no turbofish and no lifetime in any signature.

use bumpalo::Bump;

use crate::ast::{BinaryOp, BoolOp, ComparisonOp, ExprBuilder, ExprKind, LiteralBuilder};
use crate::ast::{LiteralKind, UnaryOp};
use crate::test_utils::Span;
use crate::{Tree, TreeBuilder, TreeNode, Visit};

// --- The paired builders -----------------------------------------------------

#[derive(Clone, Copy)]
pub struct ExprAst<'arena> {
    arena: &'arena Bump,
}

#[derive(Clone, Copy)]
pub struct LitAst<'arena> {
    arena: &'arena Bump,
}

/// `Bump` implements none of `Debug`, `PartialEq` or `Hash`, so a builder holding
/// one cannot derive them. Identity is the arena it allocates from.
macro_rules! arena_builder_identity {
    ($t:ident) => {
        impl core::fmt::Debug for $t<'_> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_str(stringify!($t))
            }
        }
        impl PartialEq for $t<'_> {
            fn eq(&self, other: &Self) -> bool {
                core::ptr::eq(self.arena, other.arena)
            }
        }
        impl Eq for $t<'_> {}
        impl core::hash::Hash for $t<'_> {
            fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
                core::ptr::hash(self.arena, state);
            }
        }
    };
}
arena_builder_identity!(ExprAst);
arena_builder_identity!(LitAst);

/// The storage half is identical for both builders; only `TreeKind` differs.
macro_rules! arena_storage {
    ($t:ident, $kind:ty) => {
        impl<'arena> TreeBuilder for $t<'arena> {
            type TreeData = Span;
            type TreeKind = $kind;
            type TreeHandle = &'arena TreeNode<Self>;
            type Str = &'arena str;
            type List = &'arena [Tree<Self>];
            type StrList = &'arena [&'arena str];
            type Bytes = &'arena [u8];

            fn alloc(&self, node: TreeNode<Self>) -> Self::TreeHandle {
                self.arena.alloc(node)
            }
            fn alloc_str(&self, s: &str) -> Self::Str {
                self.arena.alloc_str(s)
            }
            fn alloc_list(
                &self,
                items: impl IntoIterator<Item = Tree<Self>, IntoIter: ExactSizeIterator>,
            ) -> Self::List {
                self.arena.alloc_slice_fill_iter(items)
            }
            fn alloc_str_list(
                &self,
                items: impl IntoIterator<Item = Self::Str, IntoIter: ExactSizeIterator>,
            ) -> Self::StrList {
                self.arena.alloc_slice_fill_iter(items)
            }
            fn alloc_bytes(&self, bytes: &[u8]) -> Self::Bytes {
                self.arena.alloc_slice_copy(bytes)
            }
        }
    };
}
arena_storage!(ExprAst, ExprKind<Self>);
arena_storage!(LitAst, LiteralKind<Self>);

// The mutual knot: each builder names the other, and the `<Expr = Self>` /
// `<Lit = Self>` constraints in the trait definitions close it.
impl<'arena> ExprBuilder for ExprAst<'arena> {
    type Lit = LitAst<'arena>;
}
impl<'arena> LiteralBuilder for LitAst<'arena> {
    type Expr = ExprAst<'arena>;
}

/// A whole AST: one arena, both builders.
#[derive(Clone, Copy)]
struct Ast<'arena> {
    expr: ExprAst<'arena>,
    lit: LitAst<'arena>,
}

impl<'arena> Ast<'arena> {
    fn new(arena: &'arena Bump) -> Self {
        Self {
            expr: ExprAst { arena },
            lit: LitAst { arena },
        }
    }

    /// Spans are irrelevant to these tests, but the builder still demands one —
    /// which is the point of requiring the data explicitly.
    fn expr(&self, kind: ExprKind<ExprAst<'arena>>) -> Tree<ExprAst<'arena>> {
        TreeNode::new(Span(0, 0), kind).alloc(&self.expr)
    }

    fn lit(&self, kind: LiteralKind<LitAst<'arena>>) -> Tree<ExprAst<'arena>> {
        self.expr(ExprKind::Literal(
            TreeNode::new(Span(0, 0), kind).alloc(&self.lit),
        ))
    }

    fn int(&self, value: i64) -> Tree<ExprAst<'arena>> {
        self.lit(LiteralKind::Int {
            value,
            suffix: None,
        })
    }

    fn ident(&self, name: &str) -> Tree<ExprAst<'arena>> {
        self.expr(ExprKind::Ident(self.expr.alloc_str(name)))
    }
}

// --- A traversal spanning both trees -----------------------------------------

#[derive(Default)]
struct Census {
    exprs: usize,
    literals: usize,
    idents: usize,
    ints: i64,
}

// Crossing trees costs one extra bound: the pass needs to know that the paired
// literal builder carries the same node data.
impl<B> Visit<B, Census> for ExprKind<B>
where
    B: ExprBuilder<TreeData = Span>,
    B::Lit: LiteralBuilder<TreeData = Span>,
{
    type Output = ();

    fn visit(&self, _data: &Span, ctx: &mut Census) {
        ctx.exprs += 1;
        match self {
            // Crossing into the literal tree is an ordinary child visit.
            ExprKind::Literal(lit) => lit.visit(ctx),
            ExprKind::Ident(_) => ctx.idents += 1,
            ExprKind::None => {}

            ExprKind::Unary { expr, .. } | ExprKind::Some(expr) | ExprKind::Cast { expr } => {
                expr.visit(ctx)
            }
            ExprKind::Field { value, .. } => value.visit(ctx),
            ExprKind::Lambda { body, .. } => body.visit(ctx),

            ExprKind::Binary { left, right, .. }
            | ExprKind::Boolean { left, right, .. }
            | ExprKind::Comparison { left, right, .. } => {
                left.visit(ctx);
                right.visit(ctx);
            }
            ExprKind::Index { value, index } => {
                value.visit(ctx);
                index.visit(ctx);
            }
            ExprKind::Otherwise { primary, fallback } => {
                primary.visit(ctx);
                fallback.visit(ctx);
            }

            ExprKind::If {
                cond,
                then_branch,
                else_branch,
            } => {
                cond.visit(ctx);
                then_branch.visit(ctx);
                else_branch.visit(ctx);
            }

            ExprKind::Call { callable, args } => {
                callable.visit(ctx);
                for arg in args.iter() {
                    arg.visit(ctx);
                }
            }
            ExprKind::Array(items) | ExprKind::FormatStr { exprs: items, .. } => {
                for item in items.iter() {
                    item.visit(ctx);
                }
            }
        }
    }
}

impl<B> Visit<B, Census> for LiteralKind<B>
where
    B: LiteralBuilder<TreeData = Span>,
    B::Expr: ExprBuilder<TreeData = Span>,
{
    type Output = ();

    fn visit(&self, _data: &Span, ctx: &mut Census) {
        ctx.literals += 1;
        if let LiteralKind::Int { value, .. } = self {
            ctx.ints += value;
        }
        // ...and back into the expression tree, via the unit suffix.
        if let Some(suffix) = self.suffix() {
            suffix.visit(ctx);
        }
    }
}

// --- Tests -------------------------------------------------------------------

#[test]
fn builds_a_conditional() {
    let arena = Bump::new();
    let a = Ast::new(&arena);

    // if n > 0 then some [n, 2] else none
    let n = a.ident("n");
    let cond = a.expr(ExprKind::Comparison {
        op: ComparisonOp::Gt,
        left: n,
        right: a.int(0),
    });
    let array = a.expr(ExprKind::Array(a.expr.alloc_list([n, a.int(2)])));
    let root = a.expr(ExprKind::If {
        cond,
        then_branch: a.expr(ExprKind::Some(array)),
        else_branch: a.expr(ExprKind::None),
    });

    let mut census = Census::default();
    root.visit(&mut census);

    // Expressions: if, cond, n, lit(0), some, array, n, lit(2), none.
    assert_eq!(census.exprs, 9);
    // Each literal is an expression node wrapping a literal node.
    assert_eq!(census.literals, 2);
    assert_eq!(census.idents, 2);
    assert_eq!(census.ints, 2);
}

#[test]
fn unit_suffixes_cross_back_into_the_expression_tree() {
    let arena = Bump::new();
    let a = Ast::new(&arena);

    // 9.81`m/s^2`
    let per_second_squared = a.expr(ExprKind::Binary {
        op: BinaryOp::Div,
        left: a.ident("m"),
        right: a.expr(ExprKind::Binary {
            op: BinaryOp::Pow,
            left: a.ident("s"),
            right: a.int(2),
        }),
    });
    let g = a.lit(LiteralKind::Float {
        value: 9.81,
        suffix: Some(per_second_squared),
    });

    let mut census = Census::default();
    g.visit(&mut census);

    // The suffix is reached from the literal tree: two idents and the `2` inside
    // it are all counted, which only happens by crossing back.
    assert_eq!(census.idents, 2);
    assert_eq!(census.ints, 2);
    assert_eq!(census.literals, 2); // the 9.81 itself, and the exponent 2
}

#[test]
fn a_suffixed_literal_differs_from_a_bare_one() {
    let arena = Bump::new();
    let a = Ast::new(&arena);

    // 100`seconds` versus a bare 100
    let with_unit = a.lit(LiteralKind::Int {
        value: 100,
        suffix: Some(a.ident("seconds")),
    });
    let bare = a.int(100);

    assert_ne!(with_unit, bare);
}

#[test]
fn every_expression_variant_is_constructible() {
    let arena = Bump::new();
    let a = Ast::new(&arena);

    let x = a.ident("x");
    let one = a.int(1);

    let variants = [
        one,
        a.lit(LiteralKind::Bool(true)),
        a.lit(LiteralKind::Str(a.lit.alloc_str("hello"))),
        a.lit(LiteralKind::Bytes(a.lit.alloc_bytes(b"hi"))),
        a.lit(LiteralKind::Float {
            value: 1.5,
            suffix: None,
        }),
        a.expr(ExprKind::FormatStr {
            strs: a
                .expr
                .alloc_str_list([a.expr.alloc_str("n = "), a.expr.alloc_str("!")]),
            exprs: a.expr.alloc_list([x]),
        }),
        x,
        a.expr(ExprKind::Binary {
            op: BinaryOp::Pow,
            left: x,
            right: one,
        }),
        a.expr(ExprKind::Boolean {
            op: BoolOp::And,
            left: x,
            right: x,
        }),
        a.expr(ExprKind::Comparison {
            op: ComparisonOp::NotIn,
            left: x,
            right: x,
        }),
        a.expr(ExprKind::Unary {
            op: UnaryOp::Not,
            expr: x,
        }),
        a.expr(ExprKind::Call {
            callable: x,
            args: a.expr.alloc_list([one]),
        }),
        a.expr(ExprKind::Index {
            value: x,
            index: one,
        }),
        a.expr(ExprKind::Field {
            value: x,
            field: a.expr.alloc_str("field"),
        }),
        a.expr(ExprKind::Cast { expr: x }),
        a.expr(ExprKind::If {
            cond: x,
            then_branch: one,
            else_branch: one,
        }),
        a.expr(ExprKind::Otherwise {
            primary: x,
            fallback: one,
        }),
        a.expr(ExprKind::Some(x)),
        a.expr(ExprKind::None),
        a.expr(ExprKind::Array(a.expr.alloc_list([x, one]))),
        a.expr(ExprKind::Lambda {
            params: a
                .expr
                .alloc_str_list([a.expr.alloc_str("a"), a.expr.alloc_str("b")]),
            body: x,
        }),
    ];

    // Every variant of both enums is built above. Adding a variant breaks the
    // exhaustive matches in `Census`, and this count is the reminder to cover it
    // here too.
    assert_eq!(variants.len(), 21);

    let mut census = Census::default();
    for variant in variants {
        variant.visit(&mut census);
    }
    assert!(census.exprs > variants.len());
    // Five standalone literals, plus the shared `one` reached through seven
    // other variants — a shared subtree is visited once per edge.
    assert_eq!(census.literals, 12);
}

#[test]
fn trees_compare_structurally_across_arenas() {
    fn build<'a>(a: &Ast<'a>) -> Tree<ExprAst<'a>> {
        a.expr(ExprKind::Binary {
            op: BinaryOp::Add,
            left: a.ident("x"),
            right: a.int(1),
        })
    }

    let left_arena = Bump::new();
    let right_arena = Bump::new();

    // The same expression in two unrelated arenas: equal by structure, distinct
    // in memory. Equality crosses into the literal tree too.
    assert_eq!(
        build(&Ast::new(&left_arena)),
        build(&Ast::new(&right_arena))
    );
}
