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
    name: single_line_where,
    input: { "0   where{    a=1  ,b=2}" },
    formatted: { "0 where { a = 1, b = 2 }" },
    // Format and normalize weird spacing.
}

test_case! {
    name: multi_line_where_newline_before,
    input: { r"
0
where{a=1 + 2,b=3*4}".trim_start() },
    formatted: { r"
0
where { a = 1 + 2, b = 3 * 4 }".trim_start() },
}

test_case! {
    name: multi_line_where_bindings_on_separate_lines,
    input: { r"
0 where {a=1,
         b=2}".trim_start() },
    formatted: { r"
0 where {
    a = 1,
    b = 2,
}".trim_start() },
}

test_case! {
    name: delete_trailing_comma_single_line,
    input: { "0 where {a=1,b=2,}" },
    formatted: { "0 where { a = 1, b = 2 }" },
}

test_case! {
    name: multi_line_where_respects_newlines,
    input: { r"
0 where {
a=1, b=2}".trim_start() },
    formatted: { r"
0 where {
    a = 1,
    b = 2,
}".trim_start() },
}

test_case! {
    name: where_with_comments,
    input: { r"
0 where {
    a = 1// first binding
    , b = 2   // second binding
    , c = 30 // third binding
}".trim_start() },
    formatted: { r"
0 where {
    a = 1, // first binding
    b = 2, // second binding
    c = 30, // third binding
}".trim_start() },
    // Comments in where bindings - should align vertically and add trailing comma
}
