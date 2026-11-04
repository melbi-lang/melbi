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
    name: simple_int,
    input: { "42" },
    ast: { "(Lit Int 42)" },
    formatted: { "42" },
}
// Simple integer literal

test_case! {
    name: binary_int,
    input: { "0b101010" },
    ast: { "(Lit Int 42)" },
    formatted: { "0b101010" },
}
// Simple integer literal

test_case! {
    name: oct_int,
    input: { "0o52" },
    ast: { "(Lit Int 42)" },
    formatted: { "0o52" },
}
// Simple integer literal

test_case! {
    name: hex_int,
    input: { "0x2A" },
    ast: { "(Lit Int 42)" },
    formatted: { "0x2A" },
}
// Simple integer literal

test_case! {
    name: invalid_binary_int,
    input: { "0b3" },
    ast: { Err(_) },
}
// Invalid binary digit

test_case! {
    name: int_with_leading_zeros_spaces,
    input: { "  007  " },
    formatted: { "007" },
}
// Trim whitespace, keep leading zeros

test_case! {
    name: negative_int,
    input: { "-123" },
    formatted: { "-123" },
}
// Negative integers

test_case! {
    name: zero,
    input: { "0" },
    formatted: { "0" },
}
// Zero

test_case! {
    name: large_int_with_underscores,
    input: { "999_999_999_999_999" },
    formatted: { "999_999_999_999_999" },
}
// Large integers, keep underscores

test_case! {
    name: int_with_comment,
    input: { "42// answer to everything" },
    formatted: { "42 // answer to everything" },
}
// Comments after integers
