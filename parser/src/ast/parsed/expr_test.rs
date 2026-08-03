//! Builds real Melbi expressions across all seven parsed trees.
//!
//! One builder hosts every tree, so the passes below cross between them with no
//! turbofish, no lifetime in any signature, and no where-clause beyond
//! `B: TreeBuilder`.
//!
//! The seven `Visit` impls in `Census` are also the clearest argument for the
//! visitor rework: each re-implements the child recursion by hand, and that cost
//! is per-pass, not per-tree.

use bumpalo::Bump;

use super::test_support::{Ast, ExprTree};
use crate::ast::parsed::{
    self, BindingKind, Data, ExprKind, LiteralKind, MapEntryKind, MatchArmKind, PatternKind,
    TypeExprKind, TypeFieldKind,
};
use crate::ast::{BinaryOp, BoolOp, ComparisonOp, UnaryOp};
use crate::{TreeBuilder, Visit};

// --- A traversal spanning every tree -----------------------------------------

#[derive(Default)]
struct Census {
    exprs: usize,
    literals: usize,
    idents: usize,
    ints: i64,
    patterns: usize,
    arms: usize,
    bindings: usize,
    entries: usize,
    type_exprs: usize,
    type_fields: usize,
}

// Crossing trees costs nothing: the data type comes from the descriptor, so no
// impl needs a bound tying two trees' data together.
impl<B: TreeBuilder> Visit<B, parsed::Expr, Census> for ExprKind<B> {
    type Output = ();

    fn visit(&self, _data: &Data, ctx: &mut Census) {
        ctx.exprs += 1;
        match self {
            ExprKind::Literal(literal) => visit_literal(literal, ctx),
            ExprKind::Ident(_) => ctx.idents += 1,
            ExprKind::None => {}

            ExprKind::Unary { expr, .. } | ExprKind::Some(expr) => expr.visit(ctx),
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

            // Into the type-syntax tree.
            ExprKind::Cast { expr, ty } => {
                expr.visit(ctx);
                ty.visit(ctx);
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

            // Into the match-arm tree, and from there into patterns.
            ExprKind::Match { scrutinee, arms } => {
                scrutinee.visit(ctx);
                for arm in arms.iter() {
                    arm.visit(ctx);
                }
            }
            // Into the binding tree.
            ExprKind::Where { expr, bindings } => {
                expr.visit(ctx);
                for binding in bindings.iter() {
                    binding.visit(ctx);
                }
            }
            ExprKind::Record(bindings) => {
                for binding in bindings.iter() {
                    binding.visit(ctx);
                }
            }
            // Into the map-entry tree.
            ExprKind::Map(entries) => {
                for entry in entries.iter() {
                    entry.visit(ctx);
                }
            }
        }
    }
}

/// Literals are inline, so they get a plain helper rather than a `Visit` impl —
/// `Visit` is keyed on a descriptor and an inline enum has none. Called from the
/// expression arm and the pattern arm, which is the whole cost of inlining.
fn visit_literal<B: TreeBuilder>(literal: &LiteralKind<B>, ctx: &mut Census) {
    ctx.literals += 1;
    if let LiteralKind::Int { value, .. } = literal {
        ctx.ints += value;
    }
    // ...and back into the expression tree, via the unit suffix.
    if let Some(suffix) = literal.suffix() {
        suffix.visit(ctx);
    }
}

impl<B: TreeBuilder> Visit<B, parsed::Pattern, Census> for PatternKind<B> {
    type Output = ();

    fn visit(&self, _data: &Data, ctx: &mut Census) {
        ctx.patterns += 1;
        match self {
            PatternKind::Wildcard | PatternKind::Binding(_) | PatternKind::None => {}
            // A pattern holds the very same literal enum an expression does.
            PatternKind::Literal(literal) => visit_literal(literal, ctx),
            PatternKind::Some(inner) => inner.visit(ctx),
        }
    }
}

impl<B: TreeBuilder> Visit<B, parsed::MatchArm, Census> for MatchArmKind<B> {
    type Output = ();

    fn visit(&self, _data: &Data, ctx: &mut Census) {
        ctx.arms += 1;
        self.pattern.visit(ctx);
        self.body.visit(ctx);
    }
}

impl<B: TreeBuilder> Visit<B, parsed::Binding, Census> for BindingKind<B> {
    type Output = ();

    fn visit(&self, _data: &Data, ctx: &mut Census) {
        ctx.bindings += 1;
        self.value.visit(ctx);
    }
}

impl<B: TreeBuilder> Visit<B, parsed::MapEntry, Census> for MapEntryKind<B> {
    type Output = ();

    fn visit(&self, _data: &Data, ctx: &mut Census) {
        ctx.entries += 1;
        self.key.visit(ctx);
        self.value.visit(ctx);
    }
}

impl<B: TreeBuilder> Visit<B, parsed::TypeExpr, Census> for TypeExprKind<B> {
    type Output = ();

    fn visit(&self, _data: &Data, ctx: &mut Census) {
        ctx.type_exprs += 1;
        match self {
            TypeExprKind::Path(_) => {}
            TypeExprKind::Parametrized { params, .. } => {
                for param in params.iter() {
                    param.visit(ctx);
                }
            }
            TypeExprKind::Record(fields) => {
                for field in fields.iter() {
                    field.visit(ctx);
                }
            }
        }
    }
}

impl<B: TreeBuilder> Visit<B, parsed::TypeField, Census> for TypeFieldKind<B> {
    type Output = ();

    fn visit(&self, _data: &Data, ctx: &mut Census) {
        ctx.type_fields += 1;
        self.ty.visit(ctx);
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
    let array = a.expr(ExprKind::Array(a.alloc_list([n, a.int(2)])));
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
fn match_arms_reach_patterns_and_the_shared_literal_tree() {
    let arena = Bump::new();
    let a = Ast::new(&arena);

    // x match { some 1 -> x, _ -> 0 }
    let one_pattern = a.pattern(PatternKind::Literal(LiteralKind::Int {
        value: 1,
        suffix: None,
    }));
    let x = a.ident("x");
    let root = a.expr(ExprKind::Match {
        scrutinee: x,
        arms: a.alloc_list([
            a.arm(a.pattern(PatternKind::Some(one_pattern)), x),
            a.arm(a.pattern(PatternKind::Wildcard), a.int(0)),
        ]),
    });

    let mut census = Census::default();
    root.visit(&mut census);

    assert_eq!(census.arms, 2);
    // `some 1` is two pattern nodes; `_` is one.
    assert_eq!(census.patterns, 3);
    // The `1` inside the pattern and the `0` in the second arm's body: a pattern
    // reaches the very same literal tree the expression side uses.
    assert_eq!(census.literals, 2);
    assert_eq!(census.ints, 1);
}

#[test]
fn where_and_records_share_the_binding_tree() {
    let arena = Bump::new();
    let a = Ast::new(&arena);

    // { a = y, b = 2 } where { y = 1 }
    let record =
        a.expr(ExprKind::Record(a.alloc_list([
            a.binding("a", a.ident("y")),
            a.binding("b", a.int(2)),
        ])));
    let root = a.expr(ExprKind::Where {
        expr: record,
        bindings: a.alloc_list([a.binding("y", a.int(1))]),
    });

    let mut census = Census::default();
    root.visit(&mut census);

    // Two record fields plus one `where` binding, all the same tree.
    assert_eq!(census.bindings, 3);
    assert_eq!(census.ints, 3);
}

#[test]
fn map_entries_hold_expression_keys() {
    let arena = Bump::new();
    let a = Ast::new(&arena);

    // { 1 + 2: 3, k: v }
    let key = a.expr(ExprKind::Binary {
        op: BinaryOp::Add,
        left: a.int(1),
        right: a.int(2),
    });
    let root = a.expr(ExprKind::Map(a.alloc_list([
        a.entry(key, a.int(3)),
        a.entry(a.ident("k"), a.ident("v")),
    ])));

    let mut census = Census::default();
    root.visit(&mut census);

    assert_eq!(census.entries, 2);
    assert_eq!(census.ints, 6); // 1 + 2 + 3
    assert_eq!(census.idents, 2);
}

#[test]
fn casts_reach_the_type_syntax_tree() {
    let arena = Bump::new();
    let a = Ast::new(&arena);

    // x as Map[Str, Record[a: Int]]
    let inner = a.ty(TypeExprKind::Record(a.alloc_list([
        a.field("a", a.ty(TypeExprKind::Path(a.alloc_str("Int")))),
    ])));
    let ty = a.ty(TypeExprKind::Parametrized {
        path: a.alloc_str("Map"),
        params: a.alloc_list([a.ty(TypeExprKind::Path(a.alloc_str("Str"))), inner]),
    });
    let root = a.expr(ExprKind::Cast {
        expr: a.ident("x"),
        ty,
    });

    let mut census = Census::default();
    root.visit(&mut census);

    // Map[…], Str, Record[…], Int
    assert_eq!(census.type_exprs, 4);
    assert_eq!(census.type_fields, 1);
    // The type tree contains no expressions, so nothing leaks back.
    assert_eq!(census.exprs, 2); // the cast and `x`
    assert_eq!(census.literals, 0);
}

#[test]
fn a_pattern_and_an_expression_are_different_types() {
    // `some 1` written both ways. They are unrelated types, so `ExprKind::Some`
    // cannot be handed a pattern and `PatternKind::Some` cannot be handed an
    // expression — the census below only confirms each stayed in its own tree.
    let arena = Bump::new();
    let a = Ast::new(&arena);

    let as_expr = a.expr(ExprKind::Some(a.int(1)));
    let as_pattern = a.pattern(PatternKind::Some(a.pattern(PatternKind::Literal(
        LiteralKind::Int {
            value: 1,
            suffix: None,
        },
    ))));

    let mut expr_census = Census::default();
    as_expr.visit(&mut expr_census);
    let mut pattern_census = Census::default();
    as_pattern.visit(&mut pattern_census);

    assert_eq!((expr_census.exprs, expr_census.patterns), (2, 0));
    assert_eq!((pattern_census.exprs, pattern_census.patterns), (0, 2));
    // Both still reach the one shared literal tree.
    assert_eq!((expr_census.literals, pattern_census.literals), (1, 1));
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
        a.lit(LiteralKind::Str(a.alloc_str("hello"))),
        a.lit(LiteralKind::Bytes(a.alloc_bytes(b"hi"))),
        a.lit(LiteralKind::Float {
            value: 1.5,
            suffix: None,
        }),
        a.expr(ExprKind::FormatStr {
            strs: a.alloc_str_list([a.alloc_str("n = "), a.alloc_str("!")]),
            exprs: a.alloc_list([x]),
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
            args: a.alloc_list([one]),
        }),
        a.expr(ExprKind::Index {
            value: x,
            index: one,
        }),
        a.expr(ExprKind::Field {
            value: x,
            field: a.alloc_str("field"),
        }),
        a.expr(ExprKind::Cast {
            expr: x,
            ty: a.ty(TypeExprKind::Path(a.alloc_str("Int"))),
        }),
        a.expr(ExprKind::If {
            cond: x,
            then_branch: one,
            else_branch: one,
        }),
        a.expr(ExprKind::Otherwise {
            primary: x,
            fallback: one,
        }),
        a.expr(ExprKind::Match {
            scrutinee: x,
            arms: a.alloc_list([a.arm(a.pattern(PatternKind::Wildcard), one)]),
        }),
        a.expr(ExprKind::Where {
            expr: x,
            bindings: a.alloc_list([a.binding("y", one)]),
        }),
        a.expr(ExprKind::Some(x)),
        a.expr(ExprKind::None),
        a.expr(ExprKind::Array(a.alloc_list([x, one]))),
        a.expr(ExprKind::Record(a.alloc_list([a.binding("a", one)]))),
        a.expr(ExprKind::Map(a.alloc_list([a.entry(x, one)]))),
        a.expr(ExprKind::Lambda {
            params: a.alloc_str_list([a.alloc_str("a"), a.alloc_str("b")]),
            body: x,
        }),
    ];

    // Every variant of `ExprKind` and `LiteralKind` is built above. Adding a
    // variant breaks the exhaustive matches in `Census`, and this count is the
    // reminder to cover it here too.
    assert_eq!(variants.len(), 25);

    let mut census = Census::default();
    for variant in variants {
        variant.visit(&mut census);
    }
    assert!(census.exprs > variants.len());
    assert_eq!(census.arms, 1);
    assert_eq!(census.bindings, 2); // one `where`, one record field
    assert_eq!(census.entries, 1);
    assert_eq!(census.type_exprs, 1);
    assert_eq!(census.patterns, 1);
}

#[test]
fn trees_compare_structurally_across_arenas() {
    fn build<'a>(a: &Ast<'a>) -> ExprTree<'a> {
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
