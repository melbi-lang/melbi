//! Tests for the parser.
//!
//! These lean on two things the old parser could not check. Spans are asserted
//! on *inner* nodes, not just the root, because every node now carries its own
//! rather than being looked up in a side table. And the five small trees —
//! bindings, map entries, match arms, type expressions, type fields — are
//! asserted to have spans of their own, which is the whole reason they exist.
//!
//! Trees are compared by walking them rather than with `assert_eq!` on a whole
//! subtree: `PartialEq` on a `Tree` recurses through the kind's derives, which
//! is fine here but a trap on deep trees.

use alloc::string::String;
use alloc::vec::Vec;

use bumpalo::Bump;

use super::{ParseError, ParseErrorKind, ParseOptions, parse, parse_with_options};
use crate::ast::parsed::{Expr, ExprKind, LiteralKind, PatternKind, TypeExprKind};
use crate::ast::{BinaryOp, BoolOp, ComparisonOp, UnaryOp};
use crate::builders::ArenaBuilder;
use crate::{Span, Tree};

// --- harness -----------------------------------------------------------------

/// Parse `source`, or panic with the error.
///
/// The arena is passed in because the tree borrows from it, so it has to outlive
/// the assertions in the caller.
#[track_caller]
fn parse_in<'a>(arena: &'a Bump, source: &str) -> Tree<ArenaBuilder<'a>, Expr> {
    let builder = ArenaBuilder::new(arena);
    match parse(&builder, source) {
        Ok(tree) => tree,
        Err(error) => panic!("failed to parse {source:?}: {error}"),
    }
}

#[track_caller]
fn parse_error(source: &str) -> ParseError {
    let arena = Bump::new();
    let builder = ArenaBuilder::new(&arena);
    match parse(&builder, source) {
        Ok(_) => panic!("expected {source:?} to be rejected"),
        Err(error) => error,
    }
}

/// The source text a node's span points at.
///
/// This is the assertion that matters for spans: rather than checking offsets by
/// hand, check that the span selects the text the reader would expect.
#[track_caller]
fn snippet<'s>(source: &'s str, span: Span) -> &'s str {
    &source[span.start() as usize..span.end() as usize]
}

type ArenaExpr<'a> = Tree<ArenaBuilder<'a>, Expr>;

/// Assert that `tree`'s span covers exactly `expected` within `source`.
#[track_caller]
fn assert_spans(source: &str, tree: &ArenaExpr<'_>, expected: &str) {
    assert_eq!(
        snippet(source, tree.data().span),
        expected,
        "span {:?} of {:?}",
        tree.data().span,
        tree.kind()
    );
}

// --- literals ----------------------------------------------------------------

#[test]
fn parses_integer_literals_in_every_base() {
    let arena = Bump::new();
    for (source, expected) in [
        ("42", 42),
        ("-42", -42),
        ("0b101010", 42),
        ("0o52", 42),
        ("0x2a", 42),
        ("999_999_999", 999_999_999),
        ("-0x2a", -42),
    ] {
        let tree = parse_in(&arena, source);
        let ExprKind::Literal(LiteralKind::Int { value, suffix }) = tree.kind() else {
            panic!(
                "expected an integer literal for {source:?}, got {:?}",
                tree.kind()
            );
        };
        assert_eq!(*value, expected, "value of {source:?}");
        assert!(suffix.is_none(), "unexpected suffix on {source:?}");
        assert_spans(source, &tree, source);
    }
}

#[test]
fn parses_float_literals() {
    let arena = Bump::new();
    for (source, expected) in [
        ("3.14", 3.14),
        ("0.5", 0.5),
        (".5", 0.5),
        ("3.", 3.0),
        ("1.5e10", 1.5e10),
        ("1.5E+10", 1.5e10),
        ("1.5e-10", 1.5e-10),
        ("1_000.5", 1000.5),
        ("-3.14", -3.14),
    ] {
        let tree = parse_in(&arena, source);
        let ExprKind::Literal(LiteralKind::Float { value, .. }) = tree.kind() else {
            panic!(
                "expected a float literal for {source:?}, got {:?}",
                tree.kind()
            );
        };
        assert_eq!(*value, expected, "value of {source:?}");
    }
}

#[test]
fn parses_boolean_string_and_bytes_literals() {
    let arena = Bump::new();

    let tree = parse_in(&arena, "true");
    assert_eq!(*tree.kind(), ExprKind::Literal(LiteralKind::Bool(true)));

    let tree = parse_in(&arena, r#""hello\nworld""#);
    let ExprKind::Literal(LiteralKind::Str(text)) = tree.kind() else {
        panic!("expected a string literal, got {:?}", tree.kind());
    };
    assert_eq!(*text, "hello\nworld");

    // Single quotes are equivalent.
    let tree = parse_in(&arena, "'hello'");
    let ExprKind::Literal(LiteralKind::Str(text)) = tree.kind() else {
        panic!("expected a string literal, got {:?}", tree.kind());
    };
    assert_eq!(*text, "hello");

    let tree = parse_in(&arena, r#"b"hi\x21""#);
    let ExprKind::Literal(LiteralKind::Bytes(bytes)) = tree.kind() else {
        panic!("expected a bytes literal, got {:?}", tree.kind());
    };
    assert_eq!(*bytes, b"hi!");
}

#[test]
fn parses_a_unit_suffix_as_an_expression() {
    let arena = Bump::new();
    let source = "9.81`m/s^2`";
    let tree = parse_in(&arena, source);

    let ExprKind::Literal(LiteralKind::Float { value, suffix }) = tree.kind() else {
        panic!("expected a float literal, got {:?}", tree.kind());
    };
    assert_eq!(*value, 9.81);

    // The suffix is a whole expression — here a quotient — and it is an edge
    // back into the expression tree.
    let suffix = suffix.as_ref().expect("expected a unit suffix");
    assert_spans(source, suffix, "m/s^2");
    let ExprKind::Binary {
        op: BinaryOp::Div, ..
    } = suffix.kind()
    else {
        panic!("expected a division, got {:?}", suffix.kind());
    };
}

#[test]
fn parses_none_and_some() {
    let arena = Bump::new();

    assert_eq!(*parse_in(&arena, "none").kind(), ExprKind::None);

    let source = "some 42";
    let tree = parse_in(&arena, source);
    let ExprKind::Some(inner) = tree.kind() else {
        panic!("expected `some`, got {:?}", tree.kind());
    };
    assert_spans(source, inner, "42");
    // The `some` node spans the operator as well as its operand.
    assert_spans(source, &tree, "some 42");
}

// --- operators and precedence ------------------------------------------------

#[test]
fn parses_a_binary_expression_with_spans_on_every_node() {
    let arena = Bump::new();
    let source = "1 + 2";
    let tree = parse_in(&arena, source);

    let ExprKind::Binary { op, left, right } = tree.kind() else {
        panic!("expected a binary expression, got {:?}", tree.kind());
    };
    assert_eq!(*op, BinaryOp::Add);

    // The operands' spans are the point: the old AST had no per-node span.
    assert_spans(source, &tree, "1 + 2");
    assert_spans(source, left, "1");
    assert_spans(source, right, "2");
}

#[test]
fn multiplication_binds_tighter_than_addition() {
    let arena = Bump::new();
    let source = "1 + 2 * 3";
    let tree = parse_in(&arena, source);

    let ExprKind::Binary {
        op: BinaryOp::Add,
        left,
        right,
    } = tree.kind()
    else {
        panic!("expected an addition at the root, got {:?}", tree.kind());
    };
    assert_spans(source, left, "1");
    // The right operand is the whole product, which its span shows.
    assert_spans(source, right, "2 * 3");
}

#[test]
fn power_is_right_associative() {
    let arena = Bump::new();
    let source = "2 ^ 3 ^ 2";
    let tree = parse_in(&arena, source);

    let ExprKind::Binary {
        op: BinaryOp::Pow,
        left,
        right,
    } = tree.kind()
    else {
        panic!("expected a power at the root, got {:?}", tree.kind());
    };
    assert_spans(source, left, "2");
    assert_spans(source, right, "3 ^ 2");
}

#[test]
fn subtraction_is_left_associative() {
    let arena = Bump::new();
    let source = "1 - 2 - 3";
    let tree = parse_in(&arena, source);

    let ExprKind::Binary {
        op: BinaryOp::Sub,
        left,
        right,
    } = tree.kind()
    else {
        panic!("expected a subtraction at the root, got {:?}", tree.kind());
    };
    assert_spans(source, left, "1 - 2");
    assert_spans(source, right, "3");
}

#[test]
fn parses_the_logical_and_comparison_operators() {
    let arena = Bump::new();

    let tree = parse_in(&arena, "a and b");
    let ExprKind::Boolean { op, .. } = tree.kind() else {
        panic!("expected a boolean operator, got {:?}", tree.kind());
    };
    assert_eq!(*op, BoolOp::And);

    // `and` binds tighter than `or`, so the root is the `or`.
    let source = "a or b and c";
    let tree = parse_in(&arena, source);
    let ExprKind::Boolean {
        op: BoolOp::Or,
        right,
        ..
    } = tree.kind()
    else {
        panic!("expected an `or` at the root, got {:?}", tree.kind());
    };
    assert_spans(source, right, "b and c");

    for (source, expected) in [
        ("a == b", ComparisonOp::Eq),
        ("a != b", ComparisonOp::Neq),
        ("a < b", ComparisonOp::Lt),
        ("a > b", ComparisonOp::Gt),
        ("a <= b", ComparisonOp::Le),
        ("a >= b", ComparisonOp::Ge),
        ("a in b", ComparisonOp::In),
        ("a not in b", ComparisonOp::NotIn),
    ] {
        let tree = parse_in(&arena, source);
        let ExprKind::Comparison { op, .. } = tree.kind() else {
            panic!(
                "expected a comparison for {source:?}, got {:?}",
                tree.kind()
            );
        };
        assert_eq!(*op, expected, "operator of {source:?}");
    }
}

#[test]
fn parses_the_prefix_operators() {
    let arena = Bump::new();

    let source = "not a";
    let tree = parse_in(&arena, source);
    let ExprKind::Unary {
        op: UnaryOp::Not,
        expr,
    } = tree.kind()
    else {
        panic!("expected a `not`, got {:?}", tree.kind());
    };
    assert_spans(source, expr, "a");

    // `-x` is a negation, while `-1` lexes as a negative literal.
    let tree = parse_in(&arena, "-x");
    let ExprKind::Unary {
        op: UnaryOp::Neg, ..
    } = tree.kind()
    else {
        panic!("expected a negation, got {:?}", tree.kind());
    };
}

#[test]
fn parentheses_override_precedence() {
    let arena = Bump::new();
    let source = "(1 + 2) * 3";
    let tree = parse_in(&arena, source);

    let ExprKind::Binary {
        op: BinaryOp::Mul,
        left,
        ..
    } = tree.kind()
    else {
        panic!(
            "expected a multiplication at the root, got {:?}",
            tree.kind()
        );
    };
    let ExprKind::Binary {
        op: BinaryOp::Add, ..
    } = left.kind()
    else {
        panic!("expected the sum to be grouped, got {:?}", left.kind());
    };
    // The parentheses leave no node — the nesting already says what they said —
    // so the sum's own span excludes them.
    assert_spans(source, left, "1 + 2");
    // The parent still covers them, so its span is a piece of syntax.
    assert_spans(source, &tree, "(1 + 2) * 3");
}

/// A node's span excludes parentheses it does not represent, but a *parent*
/// combines the source its operands occupy, so no span ever starts or ends
/// inside a bracket.
///
/// This is the case `test_grouped_expression_span_bug` records in
/// `core/src/parser/parser.rs`, where the multiplication spans `2 + 3) * 4`.
#[test]
fn a_parent_span_never_cuts_through_a_bracket() {
    let arena = Bump::new();
    let source = "1 + (2 + 3) * 4";
    let tree = parse_in(&arena, source);

    let ExprKind::Binary {
        op: BinaryOp::Add,
        left,
        right,
    } = tree.kind()
    else {
        panic!("expected an addition at the root, got {:?}", tree.kind());
    };
    assert_spans(source, &tree, source);
    assert_spans(source, left, "1");
    // Not `2 + 3) * 4`.
    assert_spans(source, right, "(2 + 3) * 4");

    let ExprKind::Binary {
        op: BinaryOp::Mul,
        left: product_left,
        right: product_right,
    } = right.kind()
    else {
        panic!("expected a multiplication, got {:?}", right.kind());
    };
    assert_spans(source, product_left, "2 + 3");
    assert_spans(source, product_right, "4");
}

#[test]
fn prefix_and_postfix_spans_cover_surrounding_parentheses() {
    let arena = Bump::new();

    // A prefix over a grouped operand reaches the closing bracket.
    let source = "-(1 + 2)";
    let tree = parse_in(&arena, source);
    let ExprKind::Unary {
        op: UnaryOp::Neg,
        expr,
    } = tree.kind()
    else {
        panic!("expected a negation, got {:?}", tree.kind());
    };
    assert_spans(source, &tree, "-(1 + 2)");
    assert_spans(source, expr, "1 + 2");

    // A postfix over a grouped operand reaches back to the opening bracket.
    let source = "(f)(x)";
    let tree = parse_in(&arena, source);
    let ExprKind::Call { callable, .. } = tree.kind() else {
        panic!("expected a call, got {:?}", tree.kind());
    };
    assert_spans(source, &tree, "(f)(x)");
    assert_spans(source, callable, "f");

    // Redundant brackets nest without confusing anything.
    let source = "((1 + 2)) * 3";
    let tree = parse_in(&arena, source);
    assert_spans(source, &tree, "((1 + 2)) * 3");
}

#[test]
fn pattern_spans_do_not_cut_through_brackets() {
    let arena = Bump::new();
    let source = "v match { some (some x) -> 1, _ -> 0 }";
    let tree = parse_in(&arena, source);

    let ExprKind::Match { arms, .. } = tree.kind() else {
        panic!("expected a `match`, got {:?}", tree.kind());
    };

    let outer = &arms[0].kind().pattern;
    // Not `some (some x`.
    assert_eq!(snippet(source, outer.data().span), "some (some x)");

    let PatternKind::Some(inner) = outer.kind() else {
        panic!("expected a `some` pattern, got {:?}", outer.kind());
    };
    assert_eq!(snippet(source, inner.data().span), "some x");
}

/// The pattern brackets are recovered from the source rather than from a pair,
/// so whitespace around them and redundant nesting both have to work.
#[test]
fn pattern_bracket_recovery_handles_whitespace_and_nesting() {
    let arena = Bump::new();

    for (source, expected) in [
        (
            "v match { some ( some x ) -> 1, _ -> 0 }",
            "some ( some x )",
        ),
        ("v match { some((some x)) -> 1, _ -> 0 }", "some((some x))"),
        (
            "v match { some\n  (some x) -> 1, _ -> 0 }",
            "some\n  (some x)",
        ),
        // No brackets at all: the span is just the operator and its operand.
        ("v match { some some x -> 1, _ -> 0 }", "some some x"),
    ] {
        let tree = parse_in(&arena, source);
        let ExprKind::Match { arms, .. } = tree.kind() else {
            panic!("expected a `match` for {source:?}, got {:?}", tree.kind());
        };
        assert_eq!(
            snippet(source, arms[0].kind().pattern.data().span),
            expected,
            "pattern span for {source:?}"
        );
    }
}

#[test]
fn parses_otherwise() {
    let arena = Bump::new();
    let source = "v[i] otherwise 0";
    let tree = parse_in(&arena, source);

    let ExprKind::Otherwise { primary, fallback } = tree.kind() else {
        panic!("expected an `otherwise`, got {:?}", tree.kind());
    };
    assert_spans(source, primary, "v[i]");
    assert_spans(source, fallback, "0");
}

// --- postfix -----------------------------------------------------------------

#[test]
fn parses_calls_indexing_and_field_access() {
    let arena = Bump::new();

    let source = "f(1, 2)";
    let tree = parse_in(&arena, source);
    let ExprKind::Call { callable, args } = tree.kind() else {
        panic!("expected a call, got {:?}", tree.kind());
    };
    assert_spans(source, callable, "f");
    assert_eq!(args.len(), 2);
    assert_spans(source, &args[1], "2");
    assert_spans(source, &tree, "f(1, 2)");

    let tree = parse_in(&arena, "f()");
    let ExprKind::Call { args, .. } = tree.kind() else {
        panic!("expected a call, got {:?}", tree.kind());
    };
    assert!(args.is_empty());

    let source = "a[0]";
    let tree = parse_in(&arena, source);
    let ExprKind::Index { value, index } = tree.kind() else {
        panic!("expected an index, got {:?}", tree.kind());
    };
    assert_spans(source, value, "a");
    assert_spans(source, index, "0");

    let source = "user.name";
    let tree = parse_in(&arena, source);
    let ExprKind::Field { value, field } = tree.kind() else {
        panic!("expected a field access, got {:?}", tree.kind());
    };
    assert_eq!(*field, "name");
    assert_spans(source, value, "user");
}

#[test]
fn postfix_operators_chain_left_to_right() {
    let arena = Bump::new();
    let source = "a.b[0](x)";
    let tree = parse_in(&arena, source);

    let ExprKind::Call { callable, .. } = tree.kind() else {
        panic!("expected a call at the root, got {:?}", tree.kind());
    };
    assert_spans(source, callable, "a.b[0]");

    let ExprKind::Index { value, .. } = callable.kind() else {
        panic!("expected an index, got {:?}", callable.kind());
    };
    assert_spans(source, value, "a.b");
}

// --- control flow ------------------------------------------------------------

#[test]
fn parses_if_expressions() {
    let arena = Bump::new();
    let source = "if x > 0 then x else -x";
    let tree = parse_in(&arena, source);

    let ExprKind::If {
        cond,
        then_branch,
        else_branch,
    } = tree.kind()
    else {
        panic!("expected an `if`, got {:?}", tree.kind());
    };
    assert_spans(source, cond, "x > 0");
    assert_spans(source, then_branch, "x");
    assert_spans(source, else_branch, "-x");
    assert_spans(source, &tree, source);
}

#[test]
fn parses_lambdas() {
    let arena = Bump::new();

    let source = "(x, y) => x + y";
    let tree = parse_in(&arena, source);
    let ExprKind::Lambda { params, body } = tree.kind() else {
        panic!("expected a lambda, got {:?}", tree.kind());
    };
    assert_eq!(params.len(), 2);
    assert_eq!(params[0], "x");
    assert_eq!(params[1], "y");
    assert_spans(source, body, "x + y");

    let tree = parse_in(&arena, "() => 42");
    let ExprKind::Lambda { params, .. } = tree.kind() else {
        panic!("expected a lambda, got {:?}", tree.kind());
    };
    assert!(params.is_empty());
}

#[test]
fn parses_where_bindings_each_with_its_own_span() {
    let arena = Bump::new();
    let source = "x + y where { x = 1, y = 2 }";
    let tree = parse_in(&arena, source);

    let ExprKind::Where { expr, bindings } = tree.kind() else {
        panic!("expected a `where`, got {:?}", tree.kind());
    };
    assert_spans(source, expr, "x + y");
    assert_eq!(bindings.len(), 2);

    // A binding is its own node, so it has a span an error can point at — the
    // thing the old tuple-in-a-slice representation could not do.
    assert_eq!(snippet(source, bindings[0].data().span), "x = 1");
    assert_eq!(snippet(source, bindings[1].data().span), "y = 2");

    assert_eq!(bindings[0].kind().name, "x");
    assert_spans(source, &bindings[0].kind().value, "1");
}

#[test]
fn parses_match_arms_each_with_its_own_span() {
    let arena = Bump::new();
    let source = "v match { some x -> x, none -> 0 }";
    let tree = parse_in(&arena, source);

    let ExprKind::Match { scrutinee, arms } = tree.kind() else {
        panic!("expected a `match`, got {:?}", tree.kind());
    };
    assert_spans(source, scrutinee, "v");
    assert_eq!(arms.len(), 2);

    assert_eq!(snippet(source, arms[0].data().span), "some x -> x");
    assert_eq!(snippet(source, arms[1].data().span), "none -> 0");

    // `some x` binds `x`.
    let first = arms[0].kind();
    let PatternKind::Some(inner) = first.pattern.kind() else {
        panic!("expected a `some` pattern, got {:?}", first.pattern.kind());
    };
    let PatternKind::Binding(name) = inner.kind() else {
        panic!("expected a binding pattern, got {:?}", inner.kind());
    };
    assert_eq!(*name, "x");
    assert_eq!(snippet(source, first.pattern.data().span), "some x");

    assert_eq!(*arms[1].kind().pattern.kind(), PatternKind::None);
}

#[test]
fn parses_the_pattern_forms() {
    let arena = Bump::new();
    let source = "v match { _ -> 0, 1 -> 1, true -> 2, \"s\" -> 3 }";
    let tree = parse_in(&arena, source);

    let ExprKind::Match { arms, .. } = tree.kind() else {
        panic!("expected a `match`, got {:?}", tree.kind());
    };
    assert_eq!(arms.len(), 4);

    assert_eq!(*arms[0].kind().pattern.kind(), PatternKind::Wildcard);
    assert_eq!(
        *arms[1].kind().pattern.kind(),
        PatternKind::Literal(LiteralKind::Int {
            value: 1,
            suffix: None
        })
    );
    assert_eq!(
        *arms[2].kind().pattern.kind(),
        PatternKind::Literal(LiteralKind::Bool(true))
    );
    let PatternKind::Literal(LiteralKind::Str(text)) = arms[3].kind().pattern.kind() else {
        panic!("expected a string pattern");
    };
    assert_eq!(*text, "s");
}

// --- collections -------------------------------------------------------------

#[test]
fn parses_arrays() {
    let arena = Bump::new();

    let source = "[1, 2, 3]";
    let tree = parse_in(&arena, source);
    let ExprKind::Array(items) = tree.kind() else {
        panic!("expected an array, got {:?}", tree.kind());
    };
    assert_eq!(items.len(), 3);
    assert_spans(source, &items[2], "3");
    assert_spans(source, &tree, "[1, 2, 3]");

    let tree = parse_in(&arena, "[]");
    let ExprKind::Array(items) = tree.kind() else {
        panic!("expected an array, got {:?}", tree.kind());
    };
    assert!(items.is_empty());

    // A trailing comma is allowed.
    let tree = parse_in(&arena, "[1, 2,]");
    let ExprKind::Array(items) = tree.kind() else {
        panic!("expected an array, got {:?}", tree.kind());
    };
    assert_eq!(items.len(), 2);
}

#[test]
fn parses_records() {
    let arena = Bump::new();

    let source = "{ x = 1, y = 2 }";
    let tree = parse_in(&arena, source);
    let ExprKind::Record(fields) = tree.kind() else {
        panic!("expected a record, got {:?}", tree.kind());
    };
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].kind().name, "x");
    assert_eq!(snippet(source, fields[1].data().span), "y = 2");

    // The empty record is spelled `Record{}`, since `{}` is the empty map.
    let tree = parse_in(&arena, "Record{}");
    let ExprKind::Record(fields) = tree.kind() else {
        panic!("expected a record, got {:?}", tree.kind());
    };
    assert!(fields.is_empty());
}

#[test]
fn parses_maps_with_expression_keys() {
    let arena = Bump::new();

    let source = "{1 + 2: 3, 4: 5}";
    let tree = parse_in(&arena, source);
    let ExprKind::Map(entries) = tree.kind() else {
        panic!("expected a map, got {:?}", tree.kind());
    };
    assert_eq!(entries.len(), 2);

    // An entry is its own node with its own span.
    assert_eq!(snippet(source, entries[0].data().span), "1 + 2: 3");
    assert_spans(source, &entries[0].kind().key, "1 + 2");
    assert_spans(source, &entries[0].kind().value, "3");

    let tree = parse_in(&arena, "{}");
    let ExprKind::Map(entries) = tree.kind() else {
        panic!("expected the empty map, got {:?}", tree.kind());
    };
    assert!(entries.is_empty());
}

#[test]
fn parses_format_strings() {
    let arena = Bump::new();

    let source = r#"f"Hello { name }!""#;
    let tree = parse_in(&arena, source);
    let ExprKind::FormatStr { strs, exprs } = tree.kind() else {
        panic!("expected a format string, got {:?}", tree.kind());
    };
    assert_eq!(exprs.len(), 1);
    // The invariant: one more literal piece than holes.
    assert_eq!(strs.len(), exprs.len() + 1);
    assert_eq!(strs[0], "Hello ");
    assert_eq!(strs[1], "!");
    assert_spans(source, &exprs[0], "name");
}

#[test]
fn format_string_keeps_one_more_string_than_expressions() {
    let arena = Bump::new();

    // Every shape that needs a synthetic empty piece: leading hole, trailing
    // hole, and two holes in a row.
    for source in [
        r#"f"{ a }""#,
        r#"f"{ a }{ b }""#,
        r#"f"x{ a }""#,
        r#"f"{ a }x""#,
        r#"f"{ a }x{ b }""#,
    ] {
        let tree = parse_in(&arena, source);
        let ExprKind::FormatStr { strs, exprs } = tree.kind() else {
            panic!(
                "expected a format string for {source:?}, got {:?}",
                tree.kind()
            );
        };
        assert_eq!(
            strs.len(),
            exprs.len() + 1,
            "invariant broken for {source:?}: {strs:?} / {} exprs",
            exprs.len()
        );
    }

    // Doubled braces are literal, and are not holes.
    let tree = parse_in(&arena, r#"f"{{not a hole}}""#);
    let ExprKind::FormatStr { strs, exprs } = tree.kind() else {
        panic!("expected a format string, got {:?}", tree.kind());
    };
    assert!(exprs.is_empty());
    assert_eq!(strs.len(), 1);
    assert_eq!(strs[0], "{not a hole}");
}

// --- type syntax -------------------------------------------------------------

#[test]
fn parses_casts_with_a_span_on_the_type() {
    let arena = Bump::new();

    let source = "x as Int";
    let tree = parse_in(&arena, source);
    let ExprKind::Cast { expr, ty } = tree.kind() else {
        panic!("expected a cast, got {:?}", tree.kind());
    };
    assert_spans(source, expr, "x");
    // The type is a tree now, so "unknown type" can underline just the name.
    assert_eq!(snippet(source, ty.data().span), "Int");
    let TypeExprKind::Path(path) = ty.kind() else {
        panic!("expected a type path, got {:?}", ty.kind());
    };
    assert_eq!(*path, "Int");
}

#[test]
fn parses_parametrized_and_record_types() {
    let arena = Bump::new();

    let source = "x as Map[Str, Int]";
    let tree = parse_in(&arena, source);
    let ExprKind::Cast { ty, .. } = tree.kind() else {
        panic!("expected a cast, got {:?}", tree.kind());
    };
    let TypeExprKind::Parametrized { path, params } = ty.kind() else {
        panic!("expected a parametrized type, got {:?}", ty.kind());
    };
    assert_eq!(*path, "Map");
    assert_eq!(params.len(), 2);
    assert_eq!(snippet(source, params[1].data().span), "Int");

    // Nested parameters.
    let source = "x as Array[Array[Int]]";
    let tree = parse_in(&arena, source);
    let ExprKind::Cast { ty, .. } = tree.kind() else {
        panic!("expected a cast, got {:?}", tree.kind());
    };
    let TypeExprKind::Parametrized { params, .. } = ty.kind() else {
        panic!("expected a parametrized type, got {:?}", ty.kind());
    };
    assert_eq!(snippet(source, params[0].data().span), "Array[Int]");

    let source = "x as Record[a: Int, b: Str]";
    let tree = parse_in(&arena, source);
    let ExprKind::Cast { ty, .. } = tree.kind() else {
        panic!("expected a cast, got {:?}", tree.kind());
    };
    let TypeExprKind::Record(fields) = ty.kind() else {
        panic!("expected a record type, got {:?}", ty.kind());
    };
    assert_eq!(fields.len(), 2);
    // A type field is its own node with its own span.
    assert_eq!(snippet(source, fields[0].data().span), "a: Int");
    assert_eq!(fields[0].kind().name, "a");
    assert_eq!(snippet(source, fields[0].kind().ty.data().span), "Int");
}

// --- whole programs ----------------------------------------------------------

#[test]
fn parses_a_realistic_program() {
    let arena = Bump::new();
    let source = "\
(a, b, c) => [r0, r1] where {
    delta = b ^ 2 - 4 * a * c,
    r0 = (-b + delta ^ 0.5) / (2 * a),
    r1 = (-b - delta ^ 0.5) / (2 * a),
}";
    let tree = parse_in(&arena, source);

    let ExprKind::Lambda { params, body } = tree.kind() else {
        panic!("expected a lambda, got {:?}", tree.kind());
    };
    assert_eq!(params.len(), 3);

    let ExprKind::Where { expr, bindings } = body.kind() else {
        panic!("expected a `where`, got {:?}", body.kind());
    };
    assert_eq!(bindings.len(), 3);
    assert_spans(source, expr, "[r0, r1]");
    assert_eq!(bindings[0].kind().name, "delta");
}

#[test]
fn comments_are_ignored() {
    let arena = Bump::new();
    let tree = parse_in(&arena, "1 + // a comment\n 2");
    let ExprKind::Binary {
        op: BinaryOp::Add, ..
    } = tree.kind()
    else {
        panic!("expected an addition, got {:?}", tree.kind());
    };
}

// --- failures ----------------------------------------------------------------

#[test]
fn rejects_syntactically_invalid_input() {
    for source in ["1 +", "(1", "", "if x then 1", "f(1,,2)"] {
        let error = parse_error(source);
        assert!(
            matches!(error.kind, ParseErrorKind::UnexpectedToken { .. }),
            "expected a syntax error for {source:?}, got {:?}",
            error.kind
        );
    }
}

#[test]
fn rejects_an_integer_too_large_for_i64() {
    let error = parse_error("99999999999999999999999999");
    assert!(
        matches!(error.kind, ParseErrorKind::InvalidLiteral { .. }),
        "got {:?}",
        error.kind
    );
}

/// An unknown escape is caught by the *grammar*, not by the unescaper: the
/// `string` rule accepts either a known escape or any character that is not a
/// backslash, so `\q` leaves the literal unmatched entirely.
#[test]
fn rejects_an_unknown_escape_as_a_syntax_error() {
    let error = parse_error(r#""bad \q escape""#);
    assert!(
        matches!(error.kind, ParseErrorKind::UnexpectedToken { .. }),
        "got {:?}",
        error.kind
    );
}

/// The escapes that *are* well-formed to the grammar but still meaningless are
/// the ones the unescaper reports.
#[test]
fn rejects_a_well_formed_but_invalid_escape() {
    // Four hex digits satisfy the grammar, but a surrogate is not a character.
    let error = parse_error("\"\\uD800\"");
    let ParseErrorKind::InvalidLiteral { message } = &error.kind else {
        panic!("expected an invalid literal, got {:?}", error.kind);
    };
    assert!(
        message.contains("Unicode scalar"),
        "unhelpful message: {message}"
    );

    // A bytes literal takes any character, and rejects non-ASCII afterwards.
    let error = parse_error(r#"b"non-ascii: é""#);
    let ParseErrorKind::InvalidLiteral { message } = &error.kind else {
        panic!("expected an invalid literal, got {:?}", error.kind);
    };
    assert!(
        message.contains("non-ASCII"),
        "unhelpful message: {message}"
    );
}

#[test]
fn rejects_a_unit_suffix_in_a_pattern() {
    let error = parse_error("v match { 1`m` -> 0, _ -> 1 }");
    let ParseErrorKind::InvalidLiteral { message } = &error.kind else {
        panic!("expected an invalid literal, got {:?}", error.kind);
    };
    assert!(message.contains("suffix"), "unhelpful message: {message}");
}

#[test]
fn an_error_points_at_the_offending_text() {
    let source = "1 + b\"caf\u{e9}\" + 2";
    let error = parse_error(source);
    // The span selects the offending literal, not the whole expression.
    assert_eq!(snippet(source, error.span), "b\"caf\u{e9}\"");
}

#[test]
fn rejects_nesting_past_the_depth_limit() {
    let arena = Bump::new();
    let builder = ArenaBuilder::new(&arena);

    // Deep enough to exceed the limit set below, shallow enough to parse fast.
    let source: String = "("
        .repeat(60)
        .chars()
        .chain("1".chars())
        .collect::<String>()
        + &")".repeat(60);

    let options = ParseOptions { max_depth: 10 };
    let error = parse_with_options(&builder, &source, options)
        .expect_err("expected the depth limit to be enforced");

    let ParseErrorKind::MaxDepthExceeded { depth, max_depth } = error.kind else {
        panic!("expected a depth error, got {:?}", error.kind);
    };
    assert_eq!(max_depth, 10);
    assert_eq!(depth, 10);

    // The same source parses when the limit allows it.
    assert!(
        parse_with_options(&builder, &source, ParseOptions::default()).is_ok(),
        "the default limit should accept 60 levels"
    );
}

/// A run of prefix operators is flat in the grammar — `- - - 1` is one
/// `expression` pair — but the Pratt parser recurses once per operator. Without
/// a guard that counts them, this overflows the stack at around 8000 rather than
/// reporting an error.
#[test]
fn deeply_nested_prefixes_are_rejected_rather_than_overflowing_the_stack() {
    let arena = Bump::new();
    let builder = ArenaBuilder::new(&arena);

    for count in [10_000, 50_000] {
        let source: String = "-".repeat(count) + "1";
        let error =
            parse(&builder, &source).expect_err("expected the default limit to reject this");
        assert!(
            matches!(error.kind, ParseErrorKind::MaxDepthExceeded { .. }),
            "got {:?} for {count} prefixes",
            error.kind
        );
    }

    // The same applies to patterns, through `some`.
    let source = alloc::format!("v match {{ {}x -> 1, _ -> 0 }}", "some ".repeat(10_000));
    let error = parse(&builder, &source).expect_err("expected the pattern depth to be limited");
    assert!(
        matches!(error.kind, ParseErrorKind::MaxDepthExceeded { .. }),
        "got {:?}",
        error.kind
    );
}

/// The guard charges the longest *consecutive* run, not the total, so an
/// expression that is wide but shallow still parses.
#[test]
fn many_non_consecutive_prefixes_are_accepted() {
    let arena = Bump::new();
    let builder = ArenaBuilder::new(&arena);

    // Well past the default limit of 500 prefix operators, none adjacent.
    let terms: Vec<String> = (0..2_000).map(|i| alloc::format!("-a{i}")).collect();
    let source = terms.join(" + ");

    assert!(
        parse(&builder, &source).is_ok(),
        "2000 separate negations are wide, not deep, and should parse"
    );
}

// --- storage independence ----------------------------------------------------

#[test]
fn the_parser_is_generic_over_storage() {
    // The same source parsed into two separate arenas gives structurally equal
    // trees: equality is over the nodes, not the handles.
    let first_arena = Bump::new();
    let second_arena = Bump::new();

    let source = "f(x) + [1, 2] where { x = 1 }";
    let first = parse_in(&first_arena, source);
    let second = parse_in(&second_arena, source);

    assert_eq!(first, second);
}

#[test]
fn trees_from_the_parser_are_copy() {
    let arena = Bump::new();
    let tree = parse_in(&arena, "1 + 2");

    // No clone: an arena handle is a plain reference.
    let alias = tree;
    assert_eq!(alias.data().span, tree.data().span);
}
