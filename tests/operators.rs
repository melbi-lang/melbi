/*
 * REVIEWED & LOCKED
 *
 * This test file has been reviewed and its expectations locked in.
 * Any future changes to these test expectations must be explicitly
 * discussed and agreed upon before implementation.
 *
 * Last reviewed: October 15, 2025
 */

mod cases;

test_case! {
    name: otherwise_operator,
    input: "x otherwise 0".trim_start(),
    ast: { r#"(Otherwise (Ident "x") (Lit Int 0))"# },
    formatted: { "x otherwise 0" },
}

test_case! {
    name: negation,
    input: "-5",
    ast: { "(Lit Int -5)" },
    formatted: { "-5" },
}

test_case! {
    name: negation_with_spaces,
    input: "  -  x  ",
    ast: { r#"(Unary Neg (Ident "x"))"# },
    formatted: { "-x" },
    // Trim spaces around unary minus
}

test_case! {
    name: logical_not_with_spaces,
    input: "  not   true  ",
    formatted: { "not true" },
    // Trim spaces around logical not
}

test_case! {
    name: otherwise_with_complex_expressions,
    input: "x + y otherwise a * b",
    formatted: { "x + y otherwise a * b" },
    // Otherwise with complex expressions
}

test_case! {
    name: double_negation,
    input: "--5",
    formatted: { "--5" },
    // Double unary operators
}

test_case! {
    name: not_with_parentheses,
    input: "not(true)",
    formatted: { "not (true)" },
    // Parentheses after not
}

test_case! {
    name: operators_with_comments,
    input: "- 5// negation",
    formatted: { "-5 // negation" },
    // Comments after unary operators
}
