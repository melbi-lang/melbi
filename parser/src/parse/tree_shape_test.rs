//! Tree-shape tests ported from `melbi-core`'s original parser.
//!
//! Each test parses a source and compares the tree, printed by [`Sexpr`], to the tree the original parser produced for the same source.
//! That pins down the shape that `melbi-core` relied on before switching to this
//! crate. Spans are tested separately, in `parse_test.rs`.

use alloc::string::{String, ToString};

use bumpalo::Bump;

use super::{ParseErrorKind, parse};
use crate::ast::parsed::Sexpr;
use crate::builders::ArenaBuilder;

// --- harness -----------------------------------------------------------------

/// Parse `source` and print its tree, or panic with the error.
#[track_caller]
fn tree_of(source: &str) -> String {
    let arena = Bump::new();
    let builder = ArenaBuilder::new(&arena);
    match parse(&builder, source) {
        Ok(tree) => Sexpr(&tree).to_string(),
        Err(error) => panic!("failed to parse {source:?}: {error}"),
    }
}

/// Assert that `source` parses to the tree printed as `expected`.
#[track_caller]
fn assert_tree(source: &str, expected: &str) {
    assert_eq!(tree_of(source), expected, "tree of {source:?}");
}

/// Parse `source`, expecting it to be rejected, and return the error kind.
#[track_caller]
fn error_kind_of(source: &str) -> ParseErrorKind {
    let arena = Bump::new();
    let builder = ArenaBuilder::new(&arena);
    match parse(&builder, source) {
        Ok(_) => panic!("expected {source:?} to be rejected"),
        Err(error) => error.kind,
    }
}

// --- tests -------------------------------------------------------------------

#[test]
fn decimal_integers() {
    assert_tree("0", "(Lit Int 0)");
    assert_tree("42", "(Lit Int 42)");
    assert_tree("123", "(Lit Int 123)");
    assert_tree("1_000", "(Lit Int 1000)");
    assert_tree("1_000_000", "(Lit Int 1000000)");
    // multiple underscores
    assert_tree("1__000", "(Lit Int 1000)");
    // trailing underscore
    assert_tree("42_", "(Lit Int 42)");
}

#[test]
fn binary_integers() {
    assert_tree("0b0", "(Lit Int 0)");
    assert_tree("0b1", "(Lit Int 1)");
    assert_tree("0b1010", "(Lit Int 10)");
    assert_tree("0b1111_0000", "(Lit Int 240)");
    // underscore after prefix
    assert_tree("0b_1010", "(Lit Int 10)");
    // trailing underscore
    assert_tree("0b1010_", "(Lit Int 10)");
    // multiple underscores after prefix
    assert_tree("0b__1010", "(Lit Int 10)");
    // underscores between digits
    assert_tree("0b1_0_1_0", "(Lit Int 10)");
}

#[test]
fn octal_integers() {
    assert_tree("0o0", "(Lit Int 0)");
    assert_tree("0o7", "(Lit Int 7)");
    assert_tree("0o755", "(Lit Int 493)");
    assert_tree("0o7654_3210", "(Lit Int 16434824)");
    // underscore after prefix
    assert_tree("0o_755", "(Lit Int 493)");
    // trailing underscore
    assert_tree("0o755_", "(Lit Int 493)");
}

#[test]
fn hexadecimal_integers() {
    assert_tree("0x0", "(Lit Int 0)");
    assert_tree("0xA", "(Lit Int 10)");
    assert_tree("0x1A3F", "(Lit Int 6719)");
    assert_tree("0xDEADBEEF", "(Lit Int 3735928559)");
    assert_tree("0xDEAD_BEEF", "(Lit Int 3735928559)");
    // underscore after prefix
    assert_tree("0x_DEAD_BEEF", "(Lit Int 3735928559)");
    // trailing underscore
    assert_tree("0xABCD_", "(Lit Int 43981)");
    // lowercase
    assert_tree("0xabcd", "(Lit Int 43981)");
    // mixed case
    assert_tree("0xAbCd", "(Lit Int 43981)");
}

#[test]
fn integers_with_suffix() {
    assert_tree("42`kg`", "(Lit Int 42 suffix=(Ident \"kg\"))");
    assert_tree(
        "100`m/s`",
        "(Lit Int 100 suffix=(Binary Div (Ident \"m\") (Ident \"s\")))",
    );
    assert_tree("0b1010`bits`", "(Lit Int 10 suffix=(Ident \"bits\"))");
    assert_tree("0xFF`bytes`", "(Lit Int 255 suffix=(Ident \"bytes\"))");
    // negative exponent in suffix
    assert_tree(
        "440`s^-1`",
        "(Lit Int 440 suffix=(Binary Pow (Ident \"s\") (Lit Int -1)))",
    );
    assert_tree(
        "100`kg*m/s^2`",
        "(Lit Int 100 suffix=(Binary Div (Binary Mul (Ident \"kg\") (Ident \"m\")) (Binary Pow (Ident \"s\") (Lit Int 2))))",
    );
}

#[test]
fn float_literals() {
    assert_tree("3.14", "(Lit Float 3.14)");
    assert_tree("3.", "(Lit Float 3.0)");
    assert_tree(".5", "(Lit Float 0.5)");
    assert_tree("0.0", "(Lit Float 0.0)");
    assert_tree("1.0", "(Lit Float 1.0)");
    assert_tree("1_000.5", "(Lit Float 1000.5)");
    assert_tree("1_000.5_000", "(Lit Float 1000.5)");
    // trailing underscore
    assert_tree("1.5_", "(Lit Float 1.5)");
}

#[test]
fn float_with_exponent() {
    assert_tree("1e10", "(Lit Float 10000000000.0)");
    assert_tree("1e-10", "(Lit Float 1e-10)");
    assert_tree("1e+10", "(Lit Float 10000000000.0)");
    assert_tree("1.0e10", "(Lit Float 10000000000.0)");
    assert_tree("3.14e-5", "(Lit Float 3.14e-5)");
    assert_tree(".5e2", "(Lit Float 50.0)");
    assert_tree("6.022e23", "(Lit Float 6.022e23)");
    // capital E
    assert_tree("1.6E-19", "(Lit Float 1.6e-19)");
    assert_tree("1_000e10", "(Lit Float 10000000000000.0)");
    assert_tree("1_000.0e+3", "(Lit Float 1000000.0)");
    // no fractional part before exponent
    assert_tree("3.e10", "(Lit Float 30000000000.0)");
}

#[test]
fn floats_with_suffix() {
    assert_tree("3.14`f32`", "(Lit Float 3.14 suffix=(Ident \"f32\"))");
    assert_tree(
        "9.81`m/s^2`",
        "(Lit Float 9.81 suffix=(Binary Div (Ident \"m\") (Binary Pow (Ident \"s\") (Lit Int 2))))",
    );
    assert_tree(
        "6.022e23`mol`",
        "(Lit Float 6.022e23 suffix=(Ident \"mol\"))",
    );
}

#[test]
fn negative_numbers() {
    // negative integers (parsed as negative literals)
    assert_tree("-42", "(Lit Int -42)");
    assert_tree("-3.14", "(Lit Float -3.14)");
    // negation with space: a unary operator, not a literal
    assert_tree("- 42", "(Unary Neg (Lit Int 42))");
}

#[test]
fn integer_overflow() {
    assert!(matches!(
        error_kind_of("9223372036854775808"),
        ParseErrorKind::InvalidLiteral { .. }
    ));
    assert!(matches!(
        error_kind_of("9223372036854775809"),
        ParseErrorKind::InvalidLiteral { .. }
    ));
}

#[test]
fn integer_min_value() {
    // i64::MIN now works with negative literal support
    assert_tree("-9223372036854775808", "(Lit Int -9223372036854775808)");
}

#[test]
fn pattern_matching_syntax() {
    assert_tree(
        "x match { 1 -> 2, _ -> 3 }",
        "(Match (Ident \"x\") (arm (PLit Int 1) (Lit Int 2)) (arm _ (Lit Int 3)))",
    );
    assert_tree(
        "x match { true -> 1, false -> 0 }",
        "(Match (Ident \"x\") (arm (PLit Bool true) (Lit Int 1)) (arm (PLit Bool false) (Lit Int 0)))",
    );
    assert_tree(
        "x match { _ -> 42 }",
        "(Match (Ident \"x\") (arm _ (Lit Int 42)))",
    );
    assert_tree(
        "x match { y -> y }",
        "(Match (Ident \"x\") (arm (PVar \"y\") (Ident \"y\")))",
    );
    assert_tree(
        "value match { x -> x + 1 }",
        "(Match (Ident \"value\") (arm (PVar \"x\") (Binary Add (Ident \"x\") (Lit Int 1))))",
    );
    assert_tree(
        "option match { some x -> x, none -> 0 }",
        "(Match (Ident \"option\") (arm (PSome (PVar \"x\")) (Ident \"x\")) (arm PNone (Lit Int 0)))",
    );
    assert_tree(
        "x match { some y -> y, none -> -1 }",
        "(Match (Ident \"x\") (arm (PSome (PVar \"y\")) (Ident \"y\")) (arm PNone (Lit Int -1)))",
    );
    assert_tree(
        "x match { some (some y) -> y, some none -> -1, none -> 0 }",
        "(Match (Ident \"x\") (arm (PSome (PSome (PVar \"y\"))) (Ident \"y\")) (arm (PSome PNone) (Lit Int -1)) (arm PNone (Lit Int 0)))",
    );
    assert_tree(
        "value match { some (some (some z)) -> z, _ -> 0 }",
        "(Match (Ident \"value\") (arm (PSome (PSome (PSome (PVar \"z\")))) (Ident \"z\")) (arm _ (Lit Int 0)))",
    );
    assert_tree(
        "x match { 1 -> 'one', 2 -> 'two', _ -> 'other' }",
        "(Match (Ident \"x\") (arm (PLit Int 1) (Lit Str \"one\")) (arm (PLit Int 2) (Lit Str \"two\")) (arm _ (Lit Str \"other\")))",
    );
    assert_tree(
        "x match { 3.14 -> 'pi', 2.71 -> 'e', _ -> 'unknown' }",
        "(Match (Ident \"x\") (arm (PLit Float 3.14) (Lit Str \"pi\")) (arm (PLit Float 2.71) (Lit Str \"e\")) (arm _ (Lit Str \"unknown\")))",
    );
    assert_tree(
        "x match { \"hello\" -> 1, \"world\" -> 2, _ -> 0 }",
        "(Match (Ident \"x\") (arm (PLit Str \"hello\") (Lit Int 1)) (arm (PLit Str \"world\") (Lit Int 2)) (arm _ (Lit Int 0)))",
    );
    assert_tree(
        "x match { b\"abc\" -> true, _ -> false }",
        "(Match (Ident \"x\") (arm (PLit Bytes [97, 98, 99]) (Lit Bool true)) (arm _ (Lit Bool false)))",
    );
    assert_tree(
        "x match { none -> 0, some 1 -> 10, some _ -> 100 }",
        "(Match (Ident \"x\") (arm PNone (Lit Int 0)) (arm (PSome (PLit Int 1)) (Lit Int 10)) (arm (PSome _) (Lit Int 100)))",
    );
    assert_tree(
        "x match { 1 -> 2, }",
        "(Match (Ident \"x\") (arm (PLit Int 1) (Lit Int 2)))",
    );
    assert_tree(
        "x match { some x -> x, none -> 0, }",
        "(Match (Ident \"x\") (arm (PSome (PVar \"x\")) (Ident \"x\")) (arm PNone (Lit Int 0)))",
    );
}

#[test]
fn addition_vs_subtraction() {
    assert_tree(
        "a + b - c",
        "(Binary Sub (Binary Add (Ident \"a\") (Ident \"b\")) (Ident \"c\"))",
    );
    assert_tree(
        "(a + b) - c",
        "(Binary Sub (Binary Add (Ident \"a\") (Ident \"b\")) (Ident \"c\"))",
    );
    assert_tree(
        "a - b + c",
        "(Binary Add (Binary Sub (Ident \"a\") (Ident \"b\")) (Ident \"c\"))",
    );
    assert_tree(
        "(a - b) + c",
        "(Binary Add (Binary Sub (Ident \"a\") (Ident \"b\")) (Ident \"c\"))",
    );
    assert_tree(
        "a + b - c + d - e + f",
        "(Binary Add (Binary Sub (Binary Add (Binary Sub (Binary Add (Ident \"a\") (Ident \"b\")) (Ident \"c\")) (Ident \"d\")) (Ident \"e\")) (Ident \"f\"))",
    );
    assert_tree(
        "((((a + b) - c) + d) - e) + f",
        "(Binary Add (Binary Sub (Binary Add (Binary Sub (Binary Add (Ident \"a\") (Ident \"b\")) (Ident \"c\")) (Ident \"d\")) (Ident \"e\")) (Ident \"f\"))",
    );
}

#[test]
fn multiplication_vs_division() {
    assert_tree(
        "a * b / c",
        "(Binary Div (Binary Mul (Ident \"a\") (Ident \"b\")) (Ident \"c\"))",
    );
    assert_tree(
        "(a * b) / c",
        "(Binary Div (Binary Mul (Ident \"a\") (Ident \"b\")) (Ident \"c\"))",
    );
    assert_tree(
        "a / b * c",
        "(Binary Mul (Binary Div (Ident \"a\") (Ident \"b\")) (Ident \"c\"))",
    );
    assert_tree(
        "(a / b) * c",
        "(Binary Mul (Binary Div (Ident \"a\") (Ident \"b\")) (Ident \"c\"))",
    );
    assert_tree(
        "a * b / c * d / e * f",
        "(Binary Mul (Binary Div (Binary Mul (Binary Div (Binary Mul (Ident \"a\") (Ident \"b\")) (Ident \"c\")) (Ident \"d\")) (Ident \"e\")) (Ident \"f\"))",
    );
    assert_tree(
        "((((a * b) / c) * d) / e) * f",
        "(Binary Mul (Binary Div (Binary Mul (Binary Div (Binary Mul (Ident \"a\") (Ident \"b\")) (Ident \"c\")) (Ident \"d\")) (Ident \"e\")) (Ident \"f\"))",
    );
}

#[test]
fn addition_vs_multiplication() {
    assert_tree(
        "a + b * c",
        "(Binary Add (Ident \"a\") (Binary Mul (Ident \"b\") (Ident \"c\")))",
    );
    assert_tree(
        "a + (b * c)",
        "(Binary Add (Ident \"a\") (Binary Mul (Ident \"b\") (Ident \"c\")))",
    );
    assert_tree(
        "a * b + c",
        "(Binary Add (Binary Mul (Ident \"a\") (Ident \"b\")) (Ident \"c\"))",
    );
    assert_tree(
        "(a * b) + c",
        "(Binary Add (Binary Mul (Ident \"a\") (Ident \"b\")) (Ident \"c\"))",
    );
}

#[test]
fn and_vs_or() {
    assert_tree(
        "true and false or true",
        "(Boolean Or (Boolean And (Lit Bool true) (Lit Bool false)) (Lit Bool true))",
    );
    assert_tree(
        "(true and false) or true",
        "(Boolean Or (Boolean And (Lit Bool true) (Lit Bool false)) (Lit Bool true))",
    );
    assert_tree(
        "true or false and true",
        "(Boolean Or (Lit Bool true) (Boolean And (Lit Bool false) (Lit Bool true)))",
    );
    assert_tree(
        "true or (false and true)",
        "(Boolean Or (Lit Bool true) (Boolean And (Lit Bool false) (Lit Bool true)))",
    );
}

#[test]
fn unary_vs_binary() {
    assert_tree("--a", "(Unary Neg (Unary Neg (Ident \"a\")))");
    assert_tree("-(-a)", "(Unary Neg (Unary Neg (Ident \"a\")))");
    assert_tree(
        "-a + b",
        "(Binary Add (Unary Neg (Ident \"a\")) (Ident \"b\"))",
    );
    assert_tree(
        "(-a) + b",
        "(Binary Add (Unary Neg (Ident \"a\")) (Ident \"b\"))",
    );
}

#[test]
fn exponentiation() {
    assert_tree(
        "a ^ b ^ c",
        "(Binary Pow (Ident \"a\") (Binary Pow (Ident \"b\") (Ident \"c\")))",
    );
    assert_tree(
        "a ^ (b ^ c)",
        "(Binary Pow (Ident \"a\") (Binary Pow (Ident \"b\") (Ident \"c\")))",
    );
    assert_tree(
        "a ^ b ^ c ^ d",
        "(Binary Pow (Ident \"a\") (Binary Pow (Ident \"b\") (Binary Pow (Ident \"c\") (Ident \"d\"))))",
    );
    assert_tree(
        "a ^ (b ^ (c ^ d))",
        "(Binary Pow (Ident \"a\") (Binary Pow (Ident \"b\") (Binary Pow (Ident \"c\") (Ident \"d\"))))",
    );
}

#[test]
fn exponentiation_vs_multiplication() {
    assert_tree(
        "a * b ^ c",
        "(Binary Mul (Ident \"a\") (Binary Pow (Ident \"b\") (Ident \"c\")))",
    );
    assert_tree(
        "a * (b ^ c)",
        "(Binary Mul (Ident \"a\") (Binary Pow (Ident \"b\") (Ident \"c\")))",
    );
    assert_tree(
        "(a * b) ^ c",
        "(Binary Pow (Binary Mul (Ident \"a\") (Ident \"b\")) (Ident \"c\"))",
    );
}

#[test]
fn exponentiation_vs_negation() {
    assert_tree(
        "-a ^ b",
        "(Unary Neg (Binary Pow (Ident \"a\") (Ident \"b\")))",
    );
    assert_tree(
        "- (a  ^ b)",
        "(Unary Neg (Binary Pow (Ident \"a\") (Ident \"b\")))",
    );
    assert_tree(
        "a ^ -b",
        "(Binary Pow (Ident \"a\") (Unary Neg (Ident \"b\")))",
    );
    assert_tree(
        "a ^ ( -b )",
        "(Binary Pow (Ident \"a\") (Unary Neg (Ident \"b\")))",
    );
}

#[test]
fn if_vs_binary() {
    assert_tree(
        "if a then b + c else d",
        "(If (Ident \"a\") (Binary Add (Ident \"b\") (Ident \"c\")) (Ident \"d\"))",
    );
    assert_tree(
        "if a then (b + c) else d",
        "(If (Ident \"a\") (Binary Add (Ident \"b\") (Ident \"c\")) (Ident \"d\"))",
    );
    assert_tree(
        "if a then b else c + d",
        "(If (Ident \"a\") (Ident \"b\") (Binary Add (Ident \"c\") (Ident \"d\")))",
    );
    assert_tree(
        "if a then b else (c + d)",
        "(If (Ident \"a\") (Ident \"b\") (Binary Add (Ident \"c\") (Ident \"d\")))",
    );
}

#[test]
fn lambda_vs_everything() {
    // This couldn't be any other way. And it makes no sense semantically.
    assert_tree("-(a) => b", "(Unary Neg (Lambda [\"a\"] (Ident \"b\")))");
    assert_tree("-((a) => b)", "(Unary Neg (Lambda [\"a\"] (Ident \"b\")))");
    assert_tree(
        "(a) => a or b",
        "(Lambda [\"a\"] (Boolean Or (Ident \"a\") (Ident \"b\")))",
    );
    assert_tree(
        "(a) => (a or b)",
        "(Lambda [\"a\"] (Boolean Or (Ident \"a\") (Ident \"b\")))",
    );
    assert_tree(
        "(a) => a + b",
        "(Lambda [\"a\"] (Binary Add (Ident \"a\") (Ident \"b\")))",
    );
    assert_tree(
        "(a) => (a + b)",
        "(Lambda [\"a\"] (Binary Add (Ident \"a\") (Ident \"b\")))",
    );
    assert_tree("(a) => a()", "(Lambda [\"a\"] (Call (Ident \"a\")))");
    assert_tree("(a) => (a())", "(Lambda [\"a\"] (Call (Ident \"a\")))");
    assert_tree(
        "(a) => a[0]",
        "(Lambda [\"a\"] (Index (Ident \"a\") (Lit Int 0)))",
    );
    assert_tree(
        "(a) => (a[0])",
        "(Lambda [\"a\"] (Index (Ident \"a\") (Lit Int 0)))",
    );
    assert_tree(
        "(a) => a.field",
        "(Lambda [\"a\"] (Field \"field\" (Ident \"a\")))",
    );
    assert_tree(
        "(a) => (a.field)",
        "(Lambda [\"a\"] (Field \"field\" (Ident \"a\")))",
    );
    assert_tree(
        "(a) => a as String",
        "(Lambda [\"a\"] (Cast (Ident \"a\") (TPath \"String\")))",
    );
    assert_tree(
        "(a) => (a as String)",
        "(Lambda [\"a\"] (Cast (Ident \"a\") (TPath \"String\")))",
    );
    assert_tree(
        "(a) => a[0] otherwise ''",
        "(Lambda [\"a\"] (Otherwise (Index (Ident \"a\") (Lit Int 0)) (Lit Str \"\")))",
    );
    assert_tree(
        "(a) => (a[0] otherwise '')",
        "(Lambda [\"a\"] (Otherwise (Index (Ident \"a\") (Lit Int 0)) (Lit Str \"\")))",
    );
    assert_tree(
        "(a) => a otherwise ''",
        "(Lambda [\"a\"] (Otherwise (Ident \"a\") (Lit Str \"\")))",
    );
    assert_tree(
        "(a) => (a otherwise '')",
        "(Lambda [\"a\"] (Otherwise (Ident \"a\") (Lit Str \"\")))",
    );
    assert_tree(
        "(a) => a where { x = 1 }",
        "(Lambda [\"a\"] (Where (Ident \"a\") (bind \"x\" (Lit Int 1))))",
    );
    assert_tree(
        "(a) => (a where { x = 1 })",
        "(Lambda [\"a\"] (Where (Ident \"a\") (bind \"x\" (Lit Int 1))))",
    );
    assert_tree(
        "(a) => a where { x = 1 } otherwise ''",
        "(Lambda [\"a\"] (Otherwise (Where (Ident \"a\") (bind \"x\" (Lit Int 1))) (Lit Str \"\")))",
    );
    assert_tree(
        "(a) => (a where { x = 1 }) otherwise ''",
        "(Lambda [\"a\"] (Otherwise (Where (Ident \"a\") (bind \"x\" (Lit Int 1))) (Lit Str \"\")))",
    );
}

#[test]
fn where_vs_prefix_operations() {
    assert_tree(
        "-a where { x = 1 }",
        "(Where (Unary Neg (Ident \"a\")) (bind \"x\" (Lit Int 1)))",
    );
    assert_tree(
        "(-a) where { x = 1 }",
        "(Where (Unary Neg (Ident \"a\")) (bind \"x\" (Lit Int 1)))",
    );
    assert_tree(
        "not a where { x = 1 }",
        "(Where (Unary Not (Ident \"a\")) (bind \"x\" (Lit Int 1)))",
    );
    assert_tree(
        "(not a) where { x = 1 }",
        "(Where (Unary Not (Ident \"a\")) (bind \"x\" (Lit Int 1)))",
    );
}

#[test]
fn where_vs_binary() {
    assert_tree(
        "a + b where { x = 1 }",
        "(Where (Binary Add (Ident \"a\") (Ident \"b\")) (bind \"x\" (Lit Int 1)))",
    );
    assert_tree(
        "(a + b) where { x = 1 }",
        "(Where (Binary Add (Ident \"a\") (Ident \"b\")) (bind \"x\" (Lit Int 1)))",
    );
    assert_tree(
        "a where { x = 1 } + b",
        "(Binary Add (Where (Ident \"a\") (bind \"x\" (Lit Int 1))) (Ident \"b\"))",
    );
    assert_tree(
        "(a where { x = 1 }) + b",
        "(Binary Add (Where (Ident \"a\") (bind \"x\" (Lit Int 1))) (Ident \"b\"))",
    );
}

#[test]
fn otherwise_vs_binary() {
    assert_tree(
        "a + b otherwise c",
        "(Otherwise (Binary Add (Ident \"a\") (Ident \"b\")) (Ident \"c\"))",
    );
    assert_tree(
        "(a + b) otherwise c",
        "(Otherwise (Binary Add (Ident \"a\") (Ident \"b\")) (Ident \"c\"))",
    );
    assert_tree(
        "a otherwise b + c",
        "(Otherwise (Ident \"a\") (Binary Add (Ident \"b\") (Ident \"c\")))",
    );
    assert_tree(
        "a otherwise (b + c)",
        "(Otherwise (Ident \"a\") (Binary Add (Ident \"b\") (Ident \"c\")))",
    );
}

#[test]
fn cast_vs_binary() {
    assert_tree(
        "a + b as String",
        "(Binary Add (Ident \"a\") (Cast (Ident \"b\") (TPath \"String\")))",
    );
    assert_tree(
        "a + (b as String)",
        "(Binary Add (Ident \"a\") (Cast (Ident \"b\") (TPath \"String\")))",
    );
    assert_tree(
        "a as String + b",
        "(Binary Add (Cast (Ident \"a\") (TPath \"String\")) (Ident \"b\"))",
    );
    assert_tree(
        "(a as String) + b",
        "(Binary Add (Cast (Ident \"a\") (TPath \"String\")) (Ident \"b\"))",
    );
}

#[test]
fn cast_vs_field_accessor() {
    assert_tree(
        "a.b as String",
        "(Cast (Field \"b\" (Ident \"a\")) (TPath \"String\"))",
    );
    assert_tree(
        "( a.b ) as String",
        "(Cast (Field \"b\" (Ident \"a\")) (TPath \"String\"))",
    );
    assert_tree(
        "a as String . b",
        "(Field \"b\" (Cast (Ident \"a\") (TPath \"String\")))",
    );
    assert_tree(
        "(a as String).b",
        "(Field \"b\" (Cast (Ident \"a\") (TPath \"String\")))",
    );
}

#[test]
fn grouped_vs_binary() {
    assert_tree(
        "a + (b * c)",
        "(Binary Add (Ident \"a\") (Binary Mul (Ident \"b\") (Ident \"c\")))",
    );
    assert_tree(
        "(a + b) * c",
        "(Binary Mul (Binary Add (Ident \"a\") (Ident \"b\")) (Ident \"c\"))",
    );
}

#[test]
fn record_vs_binary() {
    assert_tree(
        "a + { x = 1 }",
        "(Binary Add (Ident \"a\") (Record (bind \"x\" (Lit Int 1))))",
    );
    assert_tree(
        "a + ({ x = 1 })",
        "(Binary Add (Ident \"a\") (Record (bind \"x\" (Lit Int 1))))",
    );
    assert_tree(
        "{ x = 1 } + a",
        "(Binary Add (Record (bind \"x\" (Lit Int 1))) (Ident \"a\"))",
    );
    assert_tree(
        "({ x = 1 }) + a",
        "(Binary Add (Record (bind \"x\" (Lit Int 1))) (Ident \"a\"))",
    );
}

#[test]
fn map_vs_binary() {
    assert_tree(
        "a + { x: 1, y: 2 }",
        "(Binary Add (Ident \"a\") (Map (entry (Ident \"x\") (Lit Int 1)) (entry (Ident \"y\") (Lit Int 2))))",
    );
    assert_tree(
        "a + ({ x: 1, y: 2 })",
        "(Binary Add (Ident \"a\") (Map (entry (Ident \"x\") (Lit Int 1)) (entry (Ident \"y\") (Lit Int 2))))",
    );
    assert_tree(
        "{ x: 1, y: 2 } + a",
        "(Binary Add (Map (entry (Ident \"x\") (Lit Int 1)) (entry (Ident \"y\") (Lit Int 2))) (Ident \"a\"))",
    );
    assert_tree(
        "({ x: 1, y: 2 }) + a",
        "(Binary Add (Map (entry (Ident \"x\") (Lit Int 1)) (entry (Ident \"y\") (Lit Int 2))) (Ident \"a\"))",
    );
}

#[test]
fn array_vs_binary() {
    assert_tree(
        "a + [1, 2, 3]",
        "(Binary Add (Ident \"a\") (Array (Lit Int 1) (Lit Int 2) (Lit Int 3)))",
    );
    assert_tree(
        "a + ([1, 2, 3])",
        "(Binary Add (Ident \"a\") (Array (Lit Int 1) (Lit Int 2) (Lit Int 3)))",
    );
    assert_tree(
        "[1, 2, 3] + a",
        "(Binary Add (Array (Lit Int 1) (Lit Int 2) (Lit Int 3)) (Ident \"a\"))",
    );
    assert_tree(
        "([1, 2, 3]) + a",
        "(Binary Add (Array (Lit Int 1) (Lit Int 2) (Lit Int 3)) (Ident \"a\"))",
    );
}

#[test]
fn attr_access_vs_binary() {
    assert_tree(
        "a + obj.field",
        "(Binary Add (Ident \"a\") (Field \"field\" (Ident \"obj\")))",
    );
    assert_tree(
        "a + (obj.field)",
        "(Binary Add (Ident \"a\") (Field \"field\" (Ident \"obj\")))",
    );
    assert_tree(
        "obj.field + a",
        "(Binary Add (Field \"field\" (Ident \"obj\")) (Ident \"a\"))",
    );
    assert_tree(
        "(obj.field) + a",
        "(Binary Add (Field \"field\" (Ident \"obj\")) (Ident \"a\"))",
    );
}

#[test]
fn index_access_vs_binary() {
    assert_tree(
        "a + arr[0]",
        "(Binary Add (Ident \"a\") (Index (Ident \"arr\") (Lit Int 0)))",
    );
    assert_tree(
        "a + (arr[0])",
        "(Binary Add (Ident \"a\") (Index (Ident \"arr\") (Lit Int 0)))",
    );
    assert_tree(
        "arr[0] + a",
        "(Binary Add (Index (Ident \"arr\") (Lit Int 0)) (Ident \"a\"))",
    );
    assert_tree(
        "(arr[0]) + a",
        "(Binary Add (Index (Ident \"arr\") (Lit Int 0)) (Ident \"a\"))",
    );
}

#[test]
fn function_call_vs_binary() {
    assert_tree(
        "a + foo(1, 2)",
        "(Binary Add (Ident \"a\") (Call (Ident \"foo\") (Lit Int 1) (Lit Int 2)))",
    );
    assert_tree(
        "a + (foo(1, 2))",
        "(Binary Add (Ident \"a\") (Call (Ident \"foo\") (Lit Int 1) (Lit Int 2)))",
    );
    assert_tree(
        "foo(1, 2) + a",
        "(Binary Add (Call (Ident \"foo\") (Lit Int 1) (Lit Int 2)) (Ident \"a\"))",
    );
    assert_tree(
        "(foo(1, 2)) + a",
        "(Binary Add (Call (Ident \"foo\") (Lit Int 1) (Lit Int 2)) (Ident \"a\"))",
    );
}

#[test]
fn otherwise() {
    assert_tree(
        "a otherwise b otherwise c otherwise d",
        "(Otherwise (Ident \"a\") (Otherwise (Ident \"b\") (Otherwise (Ident \"c\") (Ident \"d\"))))",
    );
    assert_tree(
        "a otherwise (b otherwise (c otherwise d))",
        "(Otherwise (Ident \"a\") (Otherwise (Ident \"b\") (Otherwise (Ident \"c\") (Ident \"d\"))))",
    );
}

#[test]
fn otherwise_vs_if() {
    assert_tree(
        "if a then b else c otherwise d",
        "(Otherwise (If (Ident \"a\") (Ident \"b\") (Ident \"c\")) (Ident \"d\"))",
    );
    assert_tree(
        "(if a then b else c) otherwise d",
        "(Otherwise (If (Ident \"a\") (Ident \"b\") (Ident \"c\")) (Ident \"d\"))",
    );
}

#[test]
fn otherwise_vs_where() {
    assert_tree(
        "a where { x = 1 } otherwise b",
        "(Otherwise (Where (Ident \"a\") (bind \"x\" (Lit Int 1))) (Ident \"b\"))",
    );
    assert_tree(
        "(a where { x = 1 }) otherwise b",
        "(Otherwise (Where (Ident \"a\") (bind \"x\" (Lit Int 1))) (Ident \"b\"))",
    );
    assert_tree(
        "a otherwise b where { x = 1 }",
        "(Where (Otherwise (Ident \"a\") (Ident \"b\")) (bind \"x\" (Lit Int 1)))",
    );
    assert_tree(
        "(a otherwise b) where { x = 1 }",
        "(Where (Otherwise (Ident \"a\") (Ident \"b\")) (bind \"x\" (Lit Int 1)))",
    );
}

#[test]
fn if_vs_where() {
    assert_tree(
        "if a then b where { x = 1 } else c",
        "(If (Ident \"a\") (Where (Ident \"b\") (bind \"x\" (Lit Int 1))) (Ident \"c\"))",
    );
    assert_tree(
        "if a then (b where { x = 1 }) else c",
        "(If (Ident \"a\") (Where (Ident \"b\") (bind \"x\" (Lit Int 1))) (Ident \"c\"))",
    );
    assert_tree(
        "if a then b else c where { x = 1 }",
        "(Where (If (Ident \"a\") (Ident \"b\") (Ident \"c\")) (bind \"x\" (Lit Int 1)))",
    );
    assert_tree(
        "(if a then b else c) where { x = 1 }",
        "(Where (If (Ident \"a\") (Ident \"b\") (Ident \"c\")) (bind \"x\" (Lit Int 1)))",
    );
}

#[test]
fn otherwise_vs_cast() {
    assert_tree(
        "a as b otherwise c",
        "(Otherwise (Cast (Ident \"a\") (TPath \"b\")) (Ident \"c\"))",
    );
    assert_tree(
        "(a as b) otherwise c",
        "(Otherwise (Cast (Ident \"a\") (TPath \"b\")) (Ident \"c\"))",
    );
    assert_tree(
        "a otherwise b as c",
        "(Otherwise (Ident \"a\") (Cast (Ident \"b\") (TPath \"c\")))",
    );
    assert_tree(
        "a otherwise (b as c)",
        "(Otherwise (Ident \"a\") (Cast (Ident \"b\") (TPath \"c\")))",
    );
}

#[test]
fn if_vs_cast() {
    assert_tree(
        "if a then b as c else d",
        "(If (Ident \"a\") (Cast (Ident \"b\") (TPath \"c\")) (Ident \"d\"))",
    );
    assert_tree(
        "if a then (b as c) else d",
        "(If (Ident \"a\") (Cast (Ident \"b\") (TPath \"c\")) (Ident \"d\"))",
    );
    assert_tree(
        "if a then b else c as d",
        "(If (Ident \"a\") (Ident \"b\") (Cast (Ident \"c\") (TPath \"d\")))",
    );
    assert_tree(
        "if a then b else (c as d)",
        "(If (Ident \"a\") (Ident \"b\") (Cast (Ident \"c\") (TPath \"d\")))",
    );
}

#[test]
fn where_vs_cast() {
    assert_tree(
        "a as b where { x = 1 }",
        "(Where (Cast (Ident \"a\") (TPath \"b\")) (bind \"x\" (Lit Int 1)))",
    );
    assert_tree(
        "(a as b) where { x = 1 }",
        "(Where (Cast (Ident \"a\") (TPath \"b\")) (bind \"x\" (Lit Int 1)))",
    );
    assert_tree(
        "a where { x = 1 } as b",
        "(Cast (Where (Ident \"a\") (bind \"x\" (Lit Int 1))) (TPath \"b\"))",
    );
    assert_tree(
        "(a where { x = 1 }) as b",
        "(Cast (Where (Ident \"a\") (bind \"x\" (Lit Int 1))) (TPath \"b\"))",
    );
}

#[test]
fn otherwise_vs_grouped() {
    assert_tree(
        "a + (b otherwise c)",
        "(Binary Add (Ident \"a\") (Otherwise (Ident \"b\") (Ident \"c\")))",
    );
    assert_tree(
        "(a + b) otherwise c",
        "(Otherwise (Binary Add (Ident \"a\") (Ident \"b\")) (Ident \"c\"))",
    );
}

#[test]
fn otherwise_vs_division_and_addition() {
    assert_tree(
        "a / b otherwise b + c",
        "(Otherwise (Binary Div (Ident \"a\") (Ident \"b\")) (Binary Add (Ident \"b\") (Ident \"c\")))",
    );
    assert_tree(
        "(a / b) otherwise (b + c)",
        "(Otherwise (Binary Div (Ident \"a\") (Ident \"b\")) (Binary Add (Ident \"b\") (Ident \"c\")))",
    );
}

#[test]
fn otherwise_vs_and_or() {
    assert_tree(
        "a and b otherwise c or d",
        "(Otherwise (Boolean And (Ident \"a\") (Ident \"b\")) (Boolean Or (Ident \"c\") (Ident \"d\")))",
    );
    assert_tree(
        "(a and b) otherwise (c or d)",
        "(Otherwise (Boolean And (Ident \"a\") (Ident \"b\")) (Boolean Or (Ident \"c\") (Ident \"d\")))",
    );
    assert_tree(
        "a otherwise b and c or d",
        "(Otherwise (Ident \"a\") (Boolean Or (Boolean And (Ident \"b\") (Ident \"c\")) (Ident \"d\")))",
    );
    assert_tree(
        "a otherwise ((b and c) or d)",
        "(Otherwise (Ident \"a\") (Boolean Or (Boolean And (Ident \"b\") (Ident \"c\")) (Ident \"d\")))",
    );
}

#[test]
fn complex_nested_expression() {
    assert_tree(
        "if a then 0 else b + c where { x = 1 } otherwise d and e or f",
        "(Otherwise (Where (If (Ident \"a\") (Lit Int 0) (Binary Add (Ident \"b\") (Ident \"c\"))) (bind \"x\" (Lit Int 1))) (Boolean Or (Boolean And (Ident \"d\") (Ident \"e\")) (Ident \"f\")))",
    );
    assert_tree(
        "(if a then 0 else (b + c) where { x = 1 }) otherwise ((d and e) or f)",
        "(Otherwise (Where (If (Ident \"a\") (Lit Int 0) (Binary Add (Ident \"b\") (Ident \"c\"))) (bind \"x\" (Lit Int 1))) (Boolean Or (Boolean And (Ident \"d\") (Ident \"e\")) (Ident \"f\")))",
    );
}

#[test]
fn excessive_parentheses() {
    assert_tree("(((a + b)))", "(Binary Add (Ident \"a\") (Ident \"b\"))");
    assert_tree("a + b", "(Binary Add (Ident \"a\") (Ident \"b\"))");
}

#[test]
fn exponentiation_associativity() {
    assert_tree(
        "a ^ b ^ c",
        "(Binary Pow (Ident \"a\") (Binary Pow (Ident \"b\") (Ident \"c\")))",
    );
    assert_tree(
        "a ^ (b ^ c)",
        "(Binary Pow (Ident \"a\") (Binary Pow (Ident \"b\") (Ident \"c\")))",
    );
}

#[test]
fn function_call_with_complex_arguments() {
    assert_tree(
        "foo(a + b, c * d)",
        "(Call (Ident \"foo\") (Binary Add (Ident \"a\") (Ident \"b\")) (Binary Mul (Ident \"c\") (Ident \"d\")))",
    );
    assert_tree(
        "foo((a + b), (c * d))",
        "(Call (Ident \"foo\") (Binary Add (Ident \"a\") (Ident \"b\")) (Binary Mul (Ident \"c\") (Ident \"d\")))",
    );
}

#[test]
fn chained_constructs() {
    assert_tree(
        "if a then b where { x = 1 } otherwise c else d",
        "(If (Ident \"a\") (Otherwise (Where (Ident \"b\") (bind \"x\" (Lit Int 1))) (Ident \"c\")) (Ident \"d\"))",
    );
    assert_tree(
        "if a then (b where { x = 1 } otherwise c) else d",
        "(If (Ident \"a\") (Otherwise (Where (Ident \"b\") (bind \"x\" (Lit Int 1))) (Ident \"c\")) (Ident \"d\"))",
    );
    assert_tree(
        "if a then b else c where { x = 1 } otherwise d",
        "(Otherwise (Where (If (Ident \"a\") (Ident \"b\") (Ident \"c\")) (bind \"x\" (Lit Int 1))) (Ident \"d\"))",
    );
    assert_tree(
        "((if a then b else c) where { x = 1 }) otherwise d",
        "(Otherwise (Where (If (Ident \"a\") (Ident \"b\") (Ident \"c\")) (bind \"x\" (Lit Int 1))) (Ident \"d\"))",
    );
}

#[test]
fn not_vs_and_or() {
    assert_tree(
        "not a and b",
        "(Boolean And (Unary Not (Ident \"a\")) (Ident \"b\"))",
    );
    assert_tree(
        "(not a) and b",
        "(Boolean And (Unary Not (Ident \"a\")) (Ident \"b\"))",
    );
    assert_tree(
        "a or not b",
        "(Boolean Or (Ident \"a\") (Unary Not (Ident \"b\")))",
    );
    assert_tree(
        "a or (not b)",
        "(Boolean Or (Ident \"a\") (Unary Not (Ident \"b\")))",
    );
}

#[test]
fn deeply_nested_expressions() {
    assert_tree(
        "a + (b * (c - (d / e)))",
        "(Binary Add (Ident \"a\") (Binary Mul (Ident \"b\") (Binary Sub (Ident \"c\") (Binary Div (Ident \"d\") (Ident \"e\")))))",
    );
}

#[test]
fn in_operator_basic() {
    assert_tree(
        "5 in [1, 2, 3]",
        "(Comparison In (Lit Int 5) (Array (Lit Int 1) (Lit Int 2) (Lit Int 3)))",
    );
    assert_tree(
        "\"lo\" in \"hello\"",
        "(Comparison In (Lit Str \"lo\") (Lit Str \"hello\"))",
    );
    assert_tree(
        "\"key\" in {\"a\": 1}",
        "(Comparison In (Lit Str \"key\") (Map (entry (Lit Str \"a\") (Lit Int 1))))",
    );
}

#[test]
fn not_in_operator_basic() {
    assert_tree(
        "5 not in [1, 2, 3]",
        "(Comparison NotIn (Lit Int 5) (Array (Lit Int 1) (Lit Int 2) (Lit Int 3)))",
    );
    assert_tree(
        "\"lo\" not in \"hello\"",
        "(Comparison NotIn (Lit Str \"lo\") (Lit Str \"hello\"))",
    );
}

#[test]
fn in_vs_logical_and() {
    // "in" should have higher precedence than "and"
    assert_tree(
        "a in b and c",
        "(Boolean And (Comparison In (Ident \"a\") (Ident \"b\")) (Ident \"c\"))",
    );
    assert_tree(
        "(a in b) and c",
        "(Boolean And (Comparison In (Ident \"a\") (Ident \"b\")) (Ident \"c\"))",
    );
    assert_tree(
        "a and b in c",
        "(Boolean And (Ident \"a\") (Comparison In (Ident \"b\") (Ident \"c\")))",
    );
    assert_tree(
        "a and (b in c)",
        "(Boolean And (Ident \"a\") (Comparison In (Ident \"b\") (Ident \"c\")))",
    );
}

#[test]
fn in_vs_logical_or() {
    assert_tree(
        "a in b or c",
        "(Boolean Or (Comparison In (Ident \"a\") (Ident \"b\")) (Ident \"c\"))",
    );
    assert_tree(
        "(a in b) or c",
        "(Boolean Or (Comparison In (Ident \"a\") (Ident \"b\")) (Ident \"c\"))",
    );
    assert_tree(
        "a or b in c",
        "(Boolean Or (Ident \"a\") (Comparison In (Ident \"b\") (Ident \"c\")))",
    );
    assert_tree(
        "a or (b in c)",
        "(Boolean Or (Ident \"a\") (Comparison In (Ident \"b\") (Ident \"c\")))",
    );
}

#[test]
fn not_vs_in() {
    // "not" (logical) should have lower precedence than "in"
    assert_tree(
        "not a in b",
        "(Unary Not (Comparison In (Ident \"a\") (Ident \"b\")))",
    );
    assert_tree(
        "not (a in b)",
        "(Unary Not (Comparison In (Ident \"a\") (Ident \"b\")))",
    );
}

#[test]
fn in_vs_arithmetic() {
    // arithmetic should have higher precedence than "in"
    assert_tree(
        "x in array + 1",
        "(Comparison In (Ident \"x\") (Binary Add (Ident \"array\") (Lit Int 1)))",
    );
    assert_tree(
        "x in (array + 1)",
        "(Comparison In (Ident \"x\") (Binary Add (Ident \"array\") (Lit Int 1)))",
    );
    assert_tree(
        "a + b in c",
        "(Comparison In (Binary Add (Ident \"a\") (Ident \"b\")) (Ident \"c\"))",
    );
    assert_tree(
        "(a + b) in c",
        "(Comparison In (Binary Add (Ident \"a\") (Ident \"b\")) (Ident \"c\"))",
    );
    assert_tree(
        "x in array * 2",
        "(Comparison In (Ident \"x\") (Binary Mul (Ident \"array\") (Lit Int 2)))",
    );
    assert_tree(
        "x in (array * 2)",
        "(Comparison In (Ident \"x\") (Binary Mul (Ident \"array\") (Lit Int 2)))",
    );
}

#[test]
fn not_in_vs_logical_operators() {
    assert_tree(
        "a not in b and c",
        "(Boolean And (Comparison NotIn (Ident \"a\") (Ident \"b\")) (Ident \"c\"))",
    );
    assert_tree(
        "(a not in b) and c",
        "(Boolean And (Comparison NotIn (Ident \"a\") (Ident \"b\")) (Ident \"c\"))",
    );
    assert_tree(
        "a not in b or c",
        "(Boolean Or (Comparison NotIn (Ident \"a\") (Ident \"b\")) (Ident \"c\"))",
    );
    assert_tree(
        "(a not in b) or c",
        "(Boolean Or (Comparison NotIn (Ident \"a\") (Ident \"b\")) (Ident \"c\"))",
    );
}

#[test]
fn in_and_not_in_combined() {
    assert_tree(
        "a in b and c not in d",
        "(Boolean And (Comparison In (Ident \"a\") (Ident \"b\")) (Comparison NotIn (Ident \"c\") (Ident \"d\")))",
    );
    assert_tree(
        "(a in b) and (c not in d)",
        "(Boolean And (Comparison In (Ident \"a\") (Ident \"b\")) (Comparison NotIn (Ident \"c\") (Ident \"d\")))",
    );
}

#[test]
fn some_vs_binary() {
    assert_tree(
        "some a + b",
        "(Binary Add (Some (Ident \"a\")) (Ident \"b\"))",
    );
    assert_tree(
        "(some a) + b",
        "(Binary Add (Some (Ident \"a\")) (Ident \"b\"))",
    );
    assert_tree(
        "a + some b",
        "(Binary Add (Ident \"a\") (Some (Ident \"b\")))",
    );
    assert_tree(
        "a + (some b)",
        "(Binary Add (Ident \"a\") (Some (Ident \"b\")))",
    );
    assert_tree(
        "some a * b",
        "(Binary Mul (Some (Ident \"a\")) (Ident \"b\"))",
    );
    assert_tree(
        "(some a) * b",
        "(Binary Mul (Some (Ident \"a\")) (Ident \"b\"))",
    );
}

#[test]
fn some_vs_negation() {
    assert_tree("-some a", "(Unary Neg (Some (Ident \"a\")))");
    assert_tree("-(some a)", "(Unary Neg (Some (Ident \"a\")))");
    assert_tree("some -a", "(Some (Unary Neg (Ident \"a\")))");
    assert_tree("some (-a)", "(Some (Unary Neg (Ident \"a\")))");
}

#[test]
fn some_vs_not() {
    assert_tree("not some a", "(Unary Not (Some (Ident \"a\")))");
    assert_tree("not (some a)", "(Unary Not (Some (Ident \"a\")))");
}

#[test]
fn some_vs_postfix() {
    assert_tree("some a.field", "(Some (Field \"field\" (Ident \"a\")))");
    assert_tree("some (a.field)", "(Some (Field \"field\" (Ident \"a\")))");
    assert_tree("some a[0]", "(Some (Index (Ident \"a\") (Lit Int 0)))");
    assert_tree("some (a[0])", "(Some (Index (Ident \"a\") (Lit Int 0)))");
    assert_tree("some f()", "(Some (Call (Ident \"f\")))");
    assert_tree("some (f())", "(Some (Call (Ident \"f\")))");
}

#[test]
fn some_nested() {
    assert_tree("some some a", "(Some (Some (Ident \"a\")))");
    assert_tree("some (some a)", "(Some (Some (Ident \"a\")))");
}

#[test]
fn some_vs_otherwise() {
    assert_tree(
        "some a otherwise b",
        "(Otherwise (Some (Ident \"a\")) (Ident \"b\"))",
    );
    assert_tree(
        "(some a) otherwise b",
        "(Otherwise (Some (Ident \"a\")) (Ident \"b\"))",
    );
}

#[test]
fn none_vs_binary() {
    assert_tree("none + a", "(Binary Add (None) (Ident \"a\"))");
    assert_tree("a + none", "(Binary Add (Ident \"a\") (None))");
}

#[test]
fn some_vs_cast() {
    assert_tree(
        "some a as String",
        "(Some (Cast (Ident \"a\") (TPath \"String\")))",
    );
    assert_tree(
        "some (a as String)",
        "(Some (Cast (Ident \"a\") (TPath \"String\")))",
    );
}

#[test]
fn simple_binary_expr() {
    assert_tree("1 + 2", "(Binary Add (Lit Int 1) (Lit Int 2))");
}

#[test]
fn boolean_and_expr() {
    assert_tree(
        "true and false",
        "(Boolean And (Lit Bool true) (Lit Bool false))",
    );
}

#[test]
fn boolean_or_expr() {
    assert_tree(
        "true or false",
        "(Boolean Or (Lit Bool true) (Lit Bool false))",
    );
}

#[test]
fn boolean_chain_same_op() {
    assert_tree(
        "a and b and c",
        "(Boolean And (Boolean And (Ident \"a\") (Ident \"b\")) (Ident \"c\"))",
    );
}

#[test]
fn boolean_chain_mixed_ops() {
    assert_tree(
        "a and b or c",
        "(Boolean Or (Boolean And (Ident \"a\") (Ident \"b\")) (Ident \"c\"))",
    );
}

#[test]
fn boolean_with_not() {
    assert_tree(
        "not a and b",
        "(Boolean And (Unary Not (Ident \"a\")) (Ident \"b\"))",
    );
}

#[test]
fn if_expr() {
    assert_tree(
        "if not false then false else true",
        "(If (Unary Not (Lit Bool false)) (Lit Bool false) (Lit Bool true))",
    );
}

#[test]
fn lambda_expr() {
    assert_tree(
        "(x) => x + 1",
        "(Lambda [\"x\"] (Binary Add (Ident \"x\") (Lit Int 1)))",
    );
}

#[test]
fn cast_expr_type_names() {
    assert_tree(
        "m as Map[String, Integer]",
        "(Cast (Ident \"m\") (TParam \"Map\" (TPath \"String\") (TPath \"Integer\")))",
    );
}

#[test]
fn array_literal() {
    assert_tree("[1, 2, 3]", "(Array (Lit Int 1) (Lit Int 2) (Lit Int 3))");
}

#[test]
fn map_literal() {
    assert_tree(
        "{a: 1, b: 2}",
        "(Map (entry (Ident \"a\") (Lit Int 1)) (entry (Ident \"b\") (Lit Int 2)))",
    );
}

#[test]
fn record_literal() {
    assert_tree(
        "{ x = 1, y = 2 }",
        "(Record (bind \"x\" (Lit Int 1)) (bind \"y\" (Lit Int 2)))",
    );
}

#[test]
fn where_expr() {
    assert_tree(
        "x + y where { x = 1, y = 2 }",
        "(Where (Binary Add (Ident \"x\") (Ident \"y\")) (bind \"x\" (Lit Int 1)) (bind \"y\" (Lit Int 2)))",
    );
}

#[test]
fn lambda_no_argument() {
    assert_tree("() => 42", "(Lambda [] (Lit Int 42))");
}

#[test]
fn empty_record_literal() {
    assert_tree("Record {}", "(Record)");
}

#[test]
fn format_string_with_interpolation() {
    assert_tree(
        "f\" Hello, {a + b} !\\n \"",
        "(FormatStr [\" Hello, \", \" !\\n \"] (Binary Add (Ident \"a\") (Ident \"b\")))",
    );
}

#[test]
fn where_expr_with_bindings() {
    assert_tree(
        "x + y where { x = 1, y = 2, }",
        "(Where (Binary Add (Ident \"x\") (Ident \"y\")) (bind \"x\" (Lit Int 1)) (bind \"y\" (Lit Int 2)))",
    );
}

#[test]
fn function_call() {
    assert_tree(
        "foo(1, 2, 3)",
        "(Call (Ident \"foo\") (Lit Int 1) (Lit Int 2) (Lit Int 3))",
    );
}

#[test]
fn index_access() {
    assert_tree("arr[42]", "(Index (Ident \"arr\") (Lit Int 42))");
}

#[test]
fn attr_access() {
    assert_tree("obj.field", "(Field \"field\" (Ident \"obj\"))");
}

#[test]
fn string_literal() {
    assert_tree("\"Hello, world!\"", "(Lit Str \"Hello, world!\")");
}

#[test]
fn bytes_literal() {
    assert_tree(
        "b\"Hello, bytes!\"",
        "(Lit Bytes [72, 101, 108, 108, 111, 44, 32, 98, 121, 116, 101, 115, 33])",
    );
}

#[test]
fn bytes_escape_sequences() {
    assert_tree(
        "b\"hello\\nworld\"",
        "(Lit Bytes [104, 101, 108, 108, 111, 10, 119, 111, 114, 108, 100])",
    );
    assert_tree("b\"a\\tb\\rc\"", "(Lit Bytes [97, 9, 98, 13, 99])");
    assert_tree(
        "b\"quote:\\\" slash:\\\\ end\"",
        "(Lit Bytes [113, 117, 111, 116, 101, 58, 34, 32, 115, 108, 97, 115, 104, 58, 92, 32, 101, 110, 100])",
    );
}

#[test]
fn bytes_hex_escapes() {
    assert_tree(
        "b\"\\x48\\x65\\x6c\\x6c\\x6f\"",
        "(Lit Bytes [72, 101, 108, 108, 111])",
    );
    assert_tree(
        "b\"test\\x20data\"",
        "(Lit Bytes [116, 101, 115, 116, 32, 100, 97, 116, 97])",
    );
    assert_tree("b\"\\x00\\xff\\x42\"", "(Lit Bytes [0, 255, 66])");
}

#[test]
fn bytes_quote_styles() {
    assert_tree("b\"hello\"", "(Lit Bytes [104, 101, 108, 108, 111])");
    assert_tree("b'hello'", "(Lit Bytes [104, 101, 108, 108, 111])");
    assert_tree("b\"it's\"", "(Lit Bytes [105, 116, 39, 115])");
    assert_tree(
        "b'say \"hi\"'",
        "(Lit Bytes [115, 97, 121, 32, 34, 104, 105, 34])",
    );
}

#[test]
fn bytes_null_escape() {
    assert_tree(
        "b\"before\\0after\"",
        "(Lit Bytes [98, 101, 102, 111, 114, 101, 0, 97, 102, 116, 101, 114])",
    );
    assert_tree("b\"\\0\\0\\0\"", "(Lit Bytes [0, 0, 0])");
}

#[test]
fn bytes_line_continuation() {
    assert_tree(
        "b\"hello\\\nworld\"",
        "(Lit Bytes [104, 101, 108, 108, 111, 119, 111, 114, 108, 100])",
    );
    assert_tree(
        "b\"hello\\\n    world\"",
        "(Lit Bytes [104, 101, 108, 108, 111, 32, 32, 32, 32, 119, 111, 114, 108, 100])",
    );
    assert_tree("b\"a\\\nb\\\nc\"", "(Lit Bytes [97, 98, 99])");
}

#[test]
fn bytes_reject_non_ascii() {
    assert!(matches!(
        error_kind_of("b\"café\""),
        ParseErrorKind::InvalidLiteral { .. }
    ));
    assert!(matches!(
        error_kind_of("b\"hello 🌍\""),
        ParseErrorKind::InvalidLiteral { .. }
    ));
}

#[test]
fn string_escape_sequences() {
    assert_tree("\"hello\\nworld\"", "(Lit Str \"hello\\nworld\")");
    assert_tree("\"tab\\there\"", "(Lit Str \"tab\\there\")");
    assert_tree("\"back\\\\slash\"", "(Lit Str \"back\\\\slash\")");
}

#[test]
fn string_unicode_escapes() {
    assert_tree(
        "\"\\u0048\\u0065\\u006c\\u006c\\u006f\"",
        "(Lit Str \"Hello\")",
    );
    assert_tree("\"caf\\u00e9\"", "(Lit Str \"café\")");
    assert_tree("\"\\U0001F30D\"", "(Lit Str \"🌍\")");
}

#[test]
fn string_quote_styles() {
    assert_tree("\"hello\"", "(Lit Str \"hello\")");
    assert_tree("'hello'", "(Lit Str \"hello\")");
    assert_tree("\"say \\\"hi\\\"\"", "(Lit Str \"say \\\"hi\\\"\")");
    assert_tree("'say \\'hi\\''", "(Lit Str \"say 'hi'\")");
}

#[test]
fn string_utf8_in_source() {
    assert_tree("\"café\"", "(Lit Str \"café\")");
    assert_tree("\"🌍\"", "(Lit Str \"🌍\")");
    assert_tree("\"hello世界\"", "(Lit Str \"hello世界\")");
}

#[test]
fn string_null_escape() {
    assert_tree("\"null\\0byte\"", "(Lit Str \"null\\0byte\")");
}

#[test]
fn string_line_continuation() {
    assert_tree("\"hello\\\nworld\"", "(Lit Str \"helloworld\")");
    assert_tree("\"hello\\\n  world\"", "(Lit Str \"hello  world\")");
}

#[test]
fn format_string_escape_sequences() {
    assert_tree("f\"hello\\nworld\"", "(FormatStr [\"hello\\nworld\"])");
    assert_tree("f\"tab\\there\"", "(FormatStr [\"tab\\there\"])");
}

#[test]
fn format_string_single_quotes_escape_sequences() {
    assert_tree("f'hello\\nworld'", "(FormatStr [\"hello\\nworld\"])");
    assert_tree("f'tab\\there'", "(FormatStr [\"tab\\there\"])");
}

#[test]
fn format_string_unicode_escapes() {
    assert_tree("f\"\\u0048ello\"", "(FormatStr [\"Hello\"])");
    assert_tree("f\"\\U0001F30D planet\"", "(FormatStr [\"🌍 planet\"])");
}

#[test]
fn format_string_brace_and_escapes() {
    assert_tree("f\"{{\\n}}\"", "(FormatStr [\"{\\n}\"])");
    assert_tree(
        "f\"Line 1\\nLine 2\\t{{literal}}\"",
        "(FormatStr [\"Line 1\\nLine 2\\t{literal}\"])",
    );
}

#[test]
fn format_string_mixed() {
    assert_tree(
        "f\"text {x} more\\ntext {{literal}}\"",
        "(FormatStr [\"text \", \" more\\ntext {literal}\"] (Ident \"x\"))",
    );
}

#[test]
fn format_string_empty_parts_consecutive_exprs() {
    assert_tree(
        "f\"x{0}{1}{2}\"",
        "(FormatStr [\"x\", \"\", \"\", \"\"] (Lit Int 0) (Lit Int 1) (Lit Int 2))",
    );
}

#[test]
fn format_string_empty_parts_starts_with_expr() {
    assert_tree(
        "f\"{0}x{1}\"",
        "(FormatStr [\"\", \"x\", \"\"] (Lit Int 0) (Lit Int 1))",
    );
}

#[test]
fn format_string_empty_parts_ends_with_expr() {
    assert_tree(
        "f\"{0}{1}x\"",
        "(FormatStr [\"\", \"\", \"x\"] (Lit Int 0) (Lit Int 1))",
    );
}

#[test]
fn format_string_empty_parts_only_exprs() {
    assert_tree(
        "f\"{0}{1}\"",
        "(FormatStr [\"\", \"\", \"\"] (Lit Int 0) (Lit Int 1))",
    );
}

#[test]
fn format_string_empty_parts_alternating() {
    assert_tree(
        "f\"a{0}b{1}c\"",
        "(FormatStr [\"a\", \"b\", \"c\"] (Lit Int 0) (Lit Int 1))",
    );
}

#[test]
fn single_quoted_string_literal() {
    assert_tree("'Hello, world!'", "(Lit Str \"Hello, world!\")");
}

#[test]
fn single_quoted_bytes_literal() {
    assert_tree(
        "b'Hello, bytes!'",
        "(Lit Bytes [72, 101, 108, 108, 111, 44, 32, 98, 121, 116, 101, 115, 33])",
    );
}

#[test]
fn integer_overflow_parser() {
    assert!(matches!(
        error_kind_of("9223372036854775808"),
        ParseErrorKind::InvalidLiteral { .. }
    ));
}

#[test]
fn cast_simple_type() {
    assert_tree("42 as Int", "(Cast (Lit Int 42) (TPath \"Int\"))");
}

#[test]
fn cast_parametrized_type() {
    assert_tree(
        "x as Array[Int]",
        "(Cast (Ident \"x\") (TParam \"Array\" (TPath \"Int\")))",
    );
}

#[test]
fn cast_map_type() {
    assert_tree(
        "m as Map[String, Int]",
        "(Cast (Ident \"m\") (TParam \"Map\" (TPath \"String\") (TPath \"Int\")))",
    );
}

#[test]
fn cast_nested_parametrized_type() {
    assert_tree(
        "x as Array[Array[Int]]",
        "(Cast (Ident \"x\") (TParam \"Array\" (TParam \"Array\" (TPath \"Int\"))))",
    );
}

#[test]
fn cast_record_type() {
    assert_tree(
        "r as Record[name: String, age: Int]",
        "(Cast (Ident \"r\") (TRecord (\"name\" (TPath \"String\")) (\"age\" (TPath \"Int\"))))",
    );
}

#[test]
fn depth_tracking_shallow() {
    assert_tree("((((((((((1))))))))))", "(Lit Int 1)");
}

#[test]
fn some_prefix() {
    assert_tree("some 42", "(Some (Lit Int 42))");
}

#[test]
fn some_nested_parser() {
    assert_tree("some some 42", "(Some (Some (Lit Int 42)))");
}

#[test]
fn some_complex_expr() {
    assert_tree(
        "some (x + 1)",
        "(Some (Binary Add (Ident \"x\") (Lit Int 1)))",
    );
}

#[test]
fn some_none() {
    assert_tree("some none", "(Some (None))");
}

#[test]
fn keyword_not_in_identifier() {
    // `notdefined` should be a single identifier, not `not defined`
    assert_tree("notdefined", "(Ident \"notdefined\")");
    // `notify` should be a single identifier
    assert_tree("notify", "(Ident \"notify\")");
    // `not_a_thing` should be a single identifier
    assert_tree("not_a_thing", "(Ident \"not_a_thing\")");
}

#[test]
fn keyword_and_in_identifier() {
    // `android` should be a single identifier, not `and roid`
    assert_tree("android", "(Ident \"android\")");
    // `and_also` should be a single identifier
    assert_tree("and_also", "(Ident \"and_also\")");
}

#[test]
fn keyword_or_in_identifier() {
    // `order` should be a single identifier, not `or der`
    assert_tree("order", "(Ident \"order\")");
    // `orange` should be a single identifier
    assert_tree("orange", "(Ident \"orange\")");
    // `or_else` should be a single identifier
    assert_tree("or_else", "(Ident \"or_else\")");
}

#[test]
fn keyword_in_in_identifier() {
    // `index` should be a single identifier, not `in dex`
    assert_tree("index", "(Ident \"index\")");
    // `input` should be a single identifier
    assert_tree("input", "(Ident \"input\")");
    // `in_range` should be a single identifier
    assert_tree("in_range", "(Ident \"in_range\")");
}

#[test]
fn keyword_some_in_identifier() {
    // `something` should be a single identifier, not `some thing`
    assert_tree("something", "(Ident \"something\")");
    // `some_value` should be a single identifier
    assert_tree("some_value", "(Ident \"some_value\")");
}

#[test]
fn keyword_none_in_identifier() {
    // `nonetheless` should be a single identifier, not `none theless`
    assert_tree("nonetheless", "(Ident \"nonetheless\")");
    // `none_value` should be a single identifier
    assert_tree("none_value", "(Ident \"none_value\")");
}

#[test]
fn keyword_as_standalone() {
    // `not true` should be `not` applied to `true`
    assert_tree("not true", "(Unary Not (Lit Bool true))");
    // `x and y` should be `x and y`
    assert_tree("x and y", "(Boolean And (Ident \"x\") (Ident \"y\"))");
    // `some 42` should be `some` applied to `42`
    assert_tree("some 42", "(Some (Lit Int 42))");
}

#[test]
fn infix_keyword_boundary_regression_and() {
    assert!(matches!(
        error_kind_of("x andy"),
        ParseErrorKind::UnexpectedToken { .. }
    ));
}

#[test]
fn infix_keyword_boundary_regression_or() {
    assert!(matches!(
        error_kind_of("x ory"),
        ParseErrorKind::UnexpectedToken { .. }
    ));
}

#[test]
fn infix_keyword_boundary_regression_in() {
    assert!(matches!(
        error_kind_of("x inside"),
        ParseErrorKind::UnexpectedToken { .. }
    ));
}

#[test]
fn infix_keyword_boundary_regression_otherwise() {
    assert!(matches!(
        error_kind_of("1 otherwisely"),
        ParseErrorKind::UnexpectedToken { .. }
    ));
}

#[test]
fn pattern_keyword_boundary_some_value() {
    // `some_value` should parse as a variable pattern, not `some`
    assert_tree(
        "x match { some_value -> 1, _ -> 0 }",
        "(Match (Ident \"x\") (arm (PVar \"some_value\") (Lit Int 1)) (arm _ (Lit Int 0)))",
    );
}

#[test]
fn pattern_keyword_boundary_none_value() {
    // `none_value` should parse as a variable pattern, not `none`
    assert_tree(
        "x match { none_value -> 1, _ -> 0 }",
        "(Match (Ident \"x\") (arm (PVar \"none_value\") (Lit Int 1)) (arm _ (Lit Int 0)))",
    );
}

#[test]
fn pattern_some_with_binding() {
    // `some y` (with space) should parse as `some` applied to a variable pattern
    assert_tree(
        "x match { some y -> y, none -> 0 }",
        "(Match (Ident \"x\") (arm (PSome (PVar \"y\")) (Ident \"y\")) (arm PNone (Lit Int 0)))",
    );
}

#[test]
fn pattern_keyword_boundary_something() {
    // `something` should be a variable pattern, not `some`
    assert_tree(
        "x match { something -> 1, _ -> 0 }",
        "(Match (Ident \"x\") (arm (PVar \"something\") (Lit Int 1)) (arm _ (Lit Int 0)))",
    );
}

#[test]
fn pattern_keyword_boundary_nonetheless() {
    // `nonetheless` should be a variable pattern, not `none`
    assert_tree(
        "x match { nonetheless -> 1, _ -> 0 }",
        "(Match (Ident \"x\") (arm (PVar \"nonetheless\") (Lit Int 1)) (arm _ (Lit Int 0)))",
    );
}
