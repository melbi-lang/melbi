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
    name: arithmetic_addition,
    input: { "1+2" },
    formatted: { "1 + 2" },
}

test_case! {
    name: arithmetic_subtraction,
    input: { "3-4" },
    formatted: { "3 - 4" },
}

test_case! {
    name: arithmetic_multiplication,
    input: { "5*6" },
    formatted: { "5 * 6" },
}

test_case! {
    name: arithmetic_division,
    input: { "7/8" },
    formatted: { "7 / 8" },
}

test_case! {
    name: arithmetic_power,
    input: { "2^3" },
    formatted: { "2 ^ 3" },
}

test_case! {
    name: logical_and,
    input: { "true and false" },
    formatted: { "true and false" },
}

test_case! {
    name: logical_or,
    input: { "true or false" },
    formatted: { "true or false" },
}

test_case! {
    name: operator_precedence,
    input: { "1+2*3" },
    formatted: { "1 + 2 * 3" },
}

test_case! {
    name: operator_precedence_with_parens,
    input: { "(1+2)*3" },
    formatted: { "(1 + 2) * 3" },
}

test_case! {
    name: weird_spacing,
    input: { "  1   +    2  " },
    formatted: { "1 + 2" },
}

test_case! {
    name: chained_operations,
    input: { "1+2-3*4/5" },
    formatted: { "1 + 2 - 3 * 4 / 5" },
}

test_case! {
    name: complex_expression,
    input: { "2+3*4-5/2" },
    formatted: { "2 + 3 * 4 - 5 / 2" },
}

test_case! {
    name: parenthesized_expression,
    input: { "(2+3)*(4-1)" },
    formatted: { "(2 + 3) * (4 - 1)" },
}

test_case! {
    name: binary_with_comments,
    input: { "1 + 2// addition" },
    // 2 spaces before end-of-line comments (Google style)
    formatted: { "1 + 2 // addition" },
}

test_case! {
    name: multiline_expression,
    input: {r"
1 +
2 *
3".trim_start()},
    // Write simple expressions on a single line
    formatted: { "1 + 2 * 3" },
}
