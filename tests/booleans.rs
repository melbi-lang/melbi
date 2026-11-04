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
    name: true_literal,
    input: { "true" },
    formatted: { "true" },
}

test_case! {
    name: false_literal,
    input: { "false" },
    formatted: { "false" },
}

test_case! {
    name: boolean_with_spaces,
    input: { "  true  " },
    formatted: { "true" },
}

test_case! {
    name: boolean_mixed_case,
    input: "True", // Not really a boolean, just an identifier
    ast: { r#"(Ident "True")"# },
    formatted: { "True" },
}

test_case! {
    name: boolean_uppercase,
    input: "FALSE", // Not really a boolean, just an identifier
    ast: { r#"(Ident "FALSE")"# },
    formatted: { "FALSE" },
}

test_case! {
    name: boolean_in_expression,
    input: { "true   and    false" },
    // Boolean operators - should this be formatted?
    formatted: { "true and false" },
}

test_case! {
    name: boolean_with_comment,
    input: { "true // this is true" },
    // Comments after booleans
    formatted: { "true // this is true" },
}
