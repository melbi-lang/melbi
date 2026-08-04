use bumpalo::Bump;

use super::parser::parse;
use crate::parser::Expr;

// Helper function to parse an expression and return the AST.
//
// We test precedence by comparing whether two expressions parenthesized in
// different ways yield the same AST.
fn ast<'a>(arena: &'a Bump, source: &'a str) -> &'a Expr<'a> {
    let parsed = parse(arena, source)
        .unwrap_or_else(|e| panic!("Expression parsing failed: {source}\n{e}"));
    parsed.expr
}

#[test]
fn addition_vs_subtraction() {
    let arena = Bump::new();
    assert_eq!(ast(&arena, "a + b - c"), ast(&arena, "(a + b) - c"));
    assert_eq!(ast(&arena, "a - b + c"), ast(&arena, "(a - b) + c"));
    assert_eq!(
        ast(&arena, "a + b - c + d - e + f"),
        ast(&arena, "((((a + b) - c) + d) - e) + f")
    );
}

#[test]
fn multiplication_vs_division() {
    let arena = Bump::new();
    assert_eq!(ast(&arena, "a * b / c"), ast(&arena, "(a * b) / c"));
    assert_eq!(ast(&arena, "a / b * c"), ast(&arena, "(a / b) * c"));
    assert_eq!(
        ast(&arena, "a * b / c * d / e * f"),
        ast(&arena, "((((a * b) / c) * d) / e) * f")
    );
}

#[test]
fn addition_vs_multiplication() {
    let arena = Bump::new();
    assert_eq!(ast(&arena, "a + b * c"), ast(&arena, "a + (b * c)"));
    assert_eq!(ast(&arena, "a * b + c"), ast(&arena, "(a * b) + c"));
}

#[test]
fn and_vs_or() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "true and false or true"),
        ast(&arena, "(true and false) or true")
    );
    assert_eq!(
        ast(&arena, "true or false and true"),
        ast(&arena, "true or (false and true)")
    );
}

#[test]
fn unary_vs_binary() {
    let arena = Bump::new();
    assert_eq!(ast(&arena, "--a"), ast(&arena, "-(-a)"));
    assert_eq!(ast(&arena, "-a + b"), ast(&arena, "(-a) + b"));

    // TODO: "a + -b" raises a parsing error currently, but it should be equivalent to "a + (-b)".
    // assert_eq!(ast(&arena, "a + -b"), ast(&arena, "a + (-b)"));
}

#[test]
fn exponentiation() {
    let arena = Bump::new();
    assert_eq!(ast(&arena, "a ^ b ^ c"), ast(&arena, "a ^ (b ^ c)"));
    assert_eq!(
        ast(&arena, "a ^ b ^ c ^ d"),
        ast(&arena, "a ^ (b ^ (c ^ d))")
    );
}

#[test]
fn exponentiation_vs_multiplication() {
    let arena = Bump::new();
    assert_eq!(ast(&arena, "a * b ^ c"), ast(&arena, "a * (b ^ c)"));
    assert_eq!(ast(&arena, "(a * b) ^ c"), ast(&arena, "(a * b) ^ c"));
}

#[test]
fn exponentiation_vs_negation() {
    let arena = Bump::new();
    assert_eq!(ast(&arena, "-a ^ b"), ast(&arena, "- (a  ^ b)"));
    assert_eq!(ast(&arena, "a ^ -b"), ast(&arena, "a ^ ( -b )"));
}

#[test]
fn if_vs_binary() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "if a then b + c else d"),
        ast(&arena, "if a then (b + c) else d")
    );
    assert_eq!(
        ast(&arena, "if a then b else c + d"),
        ast(&arena, "if a then b else (c + d)")
    );
}

#[test]
fn lambda_vs_everything() {
    let arena = Bump::new();
    assert_eq!(ast(&arena, "-(a) => b"), ast(&arena, "-((a) => b)")); // This couldn't be any other way. And it makes no sense semantically.
    assert_eq!(ast(&arena, "(a) => a or b"), ast(&arena, "(a) => (a or b)"));
    assert_eq!(ast(&arena, "(a) => a + b"), ast(&arena, "(a) => (a + b)"));
    assert_eq!(ast(&arena, "(a) => a()"), ast(&arena, "(a) => (a())"));
    assert_eq!(ast(&arena, "(a) => a[0]"), ast(&arena, "(a) => (a[0])"));
    assert_eq!(
        ast(&arena, "(a) => a.field"),
        ast(&arena, "(a) => (a.field)")
    );
    assert_eq!(
        ast(&arena, "(a) => a as String"),
        ast(&arena, "(a) => (a as String)")
    );
    assert_eq!(
        ast(&arena, "(a) => a[0] otherwise ''"),
        ast(&arena, "(a) => (a[0] otherwise '')")
    );
    assert_eq!(
        ast(&arena, "(a) => a otherwise ''"),
        ast(&arena, "(a) => (a otherwise '')")
    );
    assert_eq!(
        ast(&arena, "(a) => a where { x = 1 }"),
        ast(&arena, "(a) => (a where { x = 1 })")
    );
    assert_eq!(
        ast(&arena, "(a) => a where { x = 1 } otherwise ''"),
        ast(&arena, "(a) => (a where { x = 1 }) otherwise ''")
    );
}

#[test]
fn where_vs_prefix_operations() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "-a where { x = 1 }"),
        ast(&arena, "(-a) where { x = 1 }")
    );
    assert_eq!(
        ast(&arena, "not a where { x = 1 }"),
        ast(&arena, "(not a) where { x = 1 }")
    );
}

#[test]
fn where_vs_binary() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "a + b where { x = 1 }"),
        ast(&arena, "(a + b) where { x = 1 }")
    );
    assert_eq!(
        ast(&arena, "a where { x = 1 } + b"),
        ast(&arena, "(a where { x = 1 }) + b")
    );
}

#[test]
fn otherwise_vs_binary() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "a + b otherwise c"),
        ast(&arena, "(a + b) otherwise c")
    );
    assert_eq!(
        ast(&arena, "a otherwise b + c"),
        ast(&arena, "a otherwise (b + c)")
    );
}

#[test]
fn cast_vs_binary() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "a + b as String"),
        ast(&arena, "a + (b as String)")
    );
    assert_eq!(
        ast(&arena, "a as String + b"),
        ast(&arena, "(a as String) + b")
    );
}

#[test]
fn cast_vs_field_accessor() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "a.b as String"),
        ast(&arena, "( a.b ) as String")
    );
    assert_eq!(
        ast(&arena, "a as String . b"),
        ast(&arena, "(a as String).b")
    );
}

#[test]
fn grouped_vs_binary() {
    let arena = Bump::new();
    assert_eq!(ast(&arena, "a + (b * c)"), ast(&arena, "a + (b * c)"));
    assert_eq!(ast(&arena, "(a + b) * c"), ast(&arena, "(a + b) * c"));
}

#[test]
fn record_vs_binary() {
    let arena = Bump::new();
    assert_eq!(ast(&arena, "a + { x = 1 }"), ast(&arena, "a + ({ x = 1 })"));
    assert_eq!(ast(&arena, "{ x = 1 } + a"), ast(&arena, "({ x = 1 }) + a"));
}

#[test]
fn map_vs_binary() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "a + { x: 1, y: 2 }"),
        ast(&arena, "a + ({ x: 1, y: 2 })")
    );
    assert_eq!(
        ast(&arena, "{ x: 1, y: 2 } + a"),
        ast(&arena, "({ x: 1, y: 2 }) + a")
    );
}

#[test]
fn array_vs_binary() {
    let arena = Bump::new();
    assert_eq!(ast(&arena, "a + [1, 2, 3]"), ast(&arena, "a + ([1, 2, 3])"));
    assert_eq!(ast(&arena, "[1, 2, 3] + a"), ast(&arena, "([1, 2, 3]) + a"));
}

#[test]
fn attr_access_vs_binary() {
    let arena = Bump::new();
    assert_eq!(ast(&arena, "a + obj.field"), ast(&arena, "a + (obj.field)"));
    assert_eq!(ast(&arena, "obj.field + a"), ast(&arena, "(obj.field) + a"));
}

#[test]
fn index_access_vs_binary() {
    let arena = Bump::new();
    assert_eq!(ast(&arena, "a + arr[0]"), ast(&arena, "a + (arr[0])"));
    assert_eq!(ast(&arena, "arr[0] + a"), ast(&arena, "(arr[0]) + a"));
}

#[test]
fn function_call_vs_binary() {
    let arena = Bump::new();
    assert_eq!(ast(&arena, "a + foo(1, 2)"), ast(&arena, "a + (foo(1, 2))"));
    assert_eq!(ast(&arena, "foo(1, 2) + a"), ast(&arena, "(foo(1, 2)) + a"));
}

#[test]
fn otherwise() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "a otherwise b otherwise c otherwise d"),
        ast(&arena, "a otherwise (b otherwise (c otherwise d))")
    );
}

#[test]
fn otherwise_vs_if() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "if a then b else c otherwise d"),
        ast(&arena, "(if a then b else c) otherwise d")
    );
}

#[test]
fn otherwise_vs_where() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "a where { x = 1 } otherwise b"),
        ast(&arena, "(a where { x = 1 }) otherwise b")
    );
    assert_eq!(
        ast(&arena, "a otherwise b where { x = 1 }"),
        ast(&arena, "(a otherwise b) where { x = 1 }")
    );
}

#[test]
fn if_vs_where() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "if a then b where { x = 1 } else c"),
        ast(&arena, "if a then (b where { x = 1 }) else c")
    );
    assert_eq!(
        ast(&arena, "if a then b else c where { x = 1 }"),
        ast(&arena, "(if a then b else c) where { x = 1 }")
    );
}

#[test]
fn otherwise_vs_cast() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "a as b otherwise c"),
        ast(&arena, "(a as b) otherwise c")
    );
    assert_eq!(
        ast(&arena, "a otherwise b as c"),
        ast(&arena, "a otherwise (b as c)")
    );
}

#[test]
fn if_vs_cast() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "if a then b as c else d"),
        ast(&arena, "if a then (b as c) else d")
    );
    assert_eq!(
        ast(&arena, "if a then b else c as d"),
        ast(&arena, "if a then b else (c as d)")
    );
}

#[test]
fn where_vs_cast() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "a as b where { x = 1 }"),
        ast(&arena, "(a as b) where { x = 1 }")
    );
    assert_eq!(
        ast(&arena, "a where { x = 1 } as b"),
        ast(&arena, "(a where { x = 1 }) as b")
    );
}

#[test]
fn otherwise_vs_grouped() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "a + (b otherwise c)"),
        ast(&arena, "a + (b otherwise c)")
    );
    assert_eq!(
        ast(&arena, "(a + b) otherwise c"),
        ast(&arena, "(a + b) otherwise c")
    );
}

#[test]
fn otherwise_vs_division_and_addition() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "a / b otherwise b + c"),
        ast(&arena, "(a / b) otherwise (b + c)")
    );
}

#[test]
fn otherwise_vs_and_or() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "a and b otherwise c or d"),
        ast(&arena, "(a and b) otherwise (c or d)")
    );
    assert_eq!(
        ast(&arena, "a otherwise b and c or d"),
        ast(&arena, "a otherwise ((b and c) or d)")
    );
}

#[test]
fn complex_nested_expression() {
    let arena = Bump::new();
    assert_eq!(
        ast(
            &arena,
            "if a then 0 else b + c where { x = 1 } otherwise d and e or f"
        ),
        ast(
            &arena,
            "(if a then 0 else (b + c) where { x = 1 }) otherwise ((d and e) or f)"
        )
    );
}

#[test]
fn excessive_parentheses() {
    let arena = Bump::new();
    assert_eq!(ast(&arena, "(((a + b)))"), ast(&arena, "a + b"));
}

#[test]
fn exponentiation_associativity() {
    let arena = Bump::new();
    assert_eq!(ast(&arena, "a ^ b ^ c"), ast(&arena, "a ^ (b ^ c)"));
}

#[test]
fn function_call_with_complex_arguments() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "foo(a + b, c * d)"),
        ast(&arena, "foo((a + b), (c * d))")
    );
}

#[test]
fn chained_constructs() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "if a then b where { x = 1 } otherwise c else d"),
        ast(&arena, "if a then (b where { x = 1 } otherwise c) else d")
    );

    assert_eq!(
        ast(&arena, "if a then b else c where { x = 1 } otherwise d"),
        ast(&arena, "((if a then b else c) where { x = 1 }) otherwise d")
    );
}

#[test]
fn not_vs_and_or() {
    let arena = Bump::new();
    assert_eq!(ast(&arena, "not a and b"), ast(&arena, "(not a) and b"));
    assert_eq!(ast(&arena, "a or not b"), ast(&arena, "a or (not b)"));
}

#[test]
fn deeply_nested_expressions() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "a + (b * (c - (d / e)))"),
        ast(&arena, "a + (b * (c - (d / e)))")
    );
}

#[test]
fn in_operator_basic() {
    let arena = Bump::new();
    // Basic "in" operator parsing
    assert_eq!(ast(&arena, "5 in [1, 2, 3]"), ast(&arena, "5 in [1, 2, 3]"));
    assert_eq!(
        ast(&arena, "\"lo\" in \"hello\""),
        ast(&arena, "\"lo\" in \"hello\"")
    );
    assert_eq!(
        ast(&arena, "\"key\" in {\"a\": 1}"),
        ast(&arena, "\"key\" in {\"a\": 1}")
    );
}

#[test]
fn not_in_operator_basic() {
    let arena = Bump::new();
    // Basic "not in" operator parsing
    assert_eq!(
        ast(&arena, "5 not in [1, 2, 3]"),
        ast(&arena, "5 not in [1, 2, 3]")
    );
    assert_eq!(
        ast(&arena, "\"lo\" not in \"hello\""),
        ast(&arena, "\"lo\" not in \"hello\"")
    );
}

#[test]
fn in_vs_logical_and() {
    let arena = Bump::new();
    // "in" should have higher precedence than "and"
    assert_eq!(ast(&arena, "a in b and c"), ast(&arena, "(a in b) and c"));
    assert_eq!(ast(&arena, "a and b in c"), ast(&arena, "a and (b in c)"));
}

#[test]
fn in_vs_logical_or() {
    let arena = Bump::new();
    // "in" should have higher precedence than "or"
    assert_eq!(ast(&arena, "a in b or c"), ast(&arena, "(a in b) or c"));
    assert_eq!(ast(&arena, "a or b in c"), ast(&arena, "a or (b in c)"));
}

#[test]
fn not_vs_in() {
    let arena = Bump::new();
    // "not" (logical) should have lower precedence than "in"
    // So "not a in b" means "not (a in b)"
    assert_eq!(ast(&arena, "not a in b"), ast(&arena, "not (a in b)"));
}

#[test]
fn in_vs_arithmetic() {
    let arena = Bump::new();
    // Arithmetic should have higher precedence than "in"
    assert_eq!(
        ast(&arena, "x in array + 1"),
        ast(&arena, "x in (array + 1)")
    );
    assert_eq!(ast(&arena, "a + b in c"), ast(&arena, "(a + b) in c"));
    assert_eq!(
        ast(&arena, "x in array * 2"),
        ast(&arena, "x in (array * 2)")
    );
}

#[test]
fn not_in_vs_logical_operators() {
    let arena = Bump::new();
    // "not in" should have same precedence as "in"
    assert_eq!(
        ast(&arena, "a not in b and c"),
        ast(&arena, "(a not in b) and c")
    );
    assert_eq!(
        ast(&arena, "a not in b or c"),
        ast(&arena, "(a not in b) or c")
    );
}

#[test]
fn in_and_not_in_combined() {
    let arena = Bump::new();
    // Multiple containment checks
    assert_eq!(
        ast(&arena, "a in b and c not in d"),
        ast(&arena, "(a in b) and (c not in d)")
    );
}

#[test]
fn some_vs_binary() {
    let arena = Bump::new();
    // some should have same precedence as negation (prefix)
    assert_eq!(ast(&arena, "some a + b"), ast(&arena, "(some a) + b"));
    assert_eq!(ast(&arena, "a + some b"), ast(&arena, "a + (some b)"));
    assert_eq!(ast(&arena, "some a * b"), ast(&arena, "(some a) * b"));
}

#[test]
fn some_vs_negation() {
    let arena = Bump::new();
    // some and negation at same precedence level
    assert_eq!(ast(&arena, "-some a"), ast(&arena, "-(some a)"));
    assert_eq!(ast(&arena, "some -a"), ast(&arena, "some (-a)"));
}

#[test]
fn some_vs_not() {
    let arena = Bump::new();
    // some should be at higher precedence than logical not
    assert_eq!(ast(&arena, "not some a"), ast(&arena, "not (some a)"));
}

#[test]
fn some_vs_postfix() {
    let arena = Bump::new();
    // Postfix operations should bind tighter than prefix
    assert_eq!(ast(&arena, "some a.field"), ast(&arena, "some (a.field)"));
    assert_eq!(ast(&arena, "some a[0]"), ast(&arena, "some (a[0])"));
    assert_eq!(ast(&arena, "some f()"), ast(&arena, "some (f())"));
}

#[test]
fn some_nested() {
    let arena = Bump::new();
    // Nested some should work
    assert_eq!(ast(&arena, "some some a"), ast(&arena, "some (some a)"));
}

#[test]
fn some_vs_otherwise() {
    let arena = Bump::new();
    assert_eq!(
        ast(&arena, "some a otherwise b"),
        ast(&arena, "(some a) otherwise b")
    );
}

#[test]
fn none_vs_binary() {
    let arena = Bump::new();
    // none is a literal, should work like other literals
    assert_eq!(ast(&arena, "none + a"), ast(&arena, "none + a"));
    assert_eq!(ast(&arena, "a + none"), ast(&arena, "a + none"));
}

#[test]
fn some_vs_cast() {
    let arena = Bump::new();
    // Postfix `as` should bind tighter than prefix `some`
    assert_eq!(
        ast(&arena, "some a as String"),
        ast(&arena, "some (a as String)")
    );
}
