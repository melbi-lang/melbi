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
    name: simple_if,
    input: { "if true then 1 else 2" },
    formatted: { "if true then 1 else 2" },
}

test_case! {
    name: if_with_newline_before_then,
    input: { r"
if true
then 1 else 2".trim_start() },
    formatted: { r"
if true
then 1 else 2".trim_start() },
}

test_case! {
    name: if_with_newline_before_else,
    input: { r"
if true then 1
else 2".trim_start() },
    formatted: { r"
if true then 1
else 2".trim_start() },
}

test_case! {
    name: if_all_on_separate_lines,
    input: { r"
if true
then 1
else 2".trim_start() },
    formatted: { r"
if true
then 1
else 2".trim_start() },
}

test_case! {
    name: nested_if,
    input: { "if a then if b then 1 else 2 else 3" },
    formatted: { "if a then if b then 1 else 2 else 3" },
}

test_case! {
    name: if_with_weird_spacing,
    input: { "if   true   then   1   else   2" },
    formatted: { "if true then 1 else 2" },
}
// Normalize weird spacing around keywords

test_case! {
    name: if_with_comments,
    input: { r"
if true then  // condition
    1        // then branch
else         // else branch
    2".trim_start() },
    // This is a valid formatting; but it can be controversial.
    // I think what could be argued is that the "then" should be on the next line.
    formatted: { r"
if true then // condition
1 // then branch
else // else branch
2".trim_start() },
}

test_case! {
    name: if_complex_condition,
    input: { "if a and b or c then x + y else z * w" },
    formatted: { "if a and b or c then x + y else z * w" },
}
// Complex boolean conditions

test_case! {
    name: if_multiline_condition,
    input: { r"
if a and
   b then 1 else 2".trim_start() },
    formatted: { r"
if a and b then 1 else 2".trim_start() },
}
// Multi-line conditions

test_case! {
    name: if_multiline_branches,
    input: { r"
if true then
    x + y +
    z
else
    a * b -
    c".trim_start() },
    formatted: { r"
if true then x + y + z
else a * b - c".trim_start() },
}
// Multi-line branches
