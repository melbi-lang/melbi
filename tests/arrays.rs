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
    name: empty_array,
    input: { "[]" },
    formatted: { "[]" },
}

test_case! {
    name: single_line_array,
    input: { "[1,2,3]" },
    formatted: { "[1, 2, 3]" },
}

test_case! {
    name: multi_line_array,
    input: {r"
[1,
 2,3]".trim_start()},
    formatted: { r"
[
    1,
    2,
    3,
]".trim_start() },
}

test_case! {
    name: delete_trailing_comma_single_line,
    input: { "[1,2,3,]" },
    formatted: { "[1, 2, 3]" },
}

test_case! {
    name: multi_line_array_respects_newlines,
    input: {r"
[
1,2,3]".trim_start()},
    formatted: {r"
[
    1, 2, 3,
]".trim_start()},
}

test_case! {
    name: nested_arrays,
    input: { "[[1,2],[3,4]]" },
    formatted: { "[[1, 2], [3, 4]]" },
}

test_case! {
    name: array_with_expressions,
    input: { "[1+2, 3*4, 5-6]" },
    formatted: { "[1 + 2, 3 * 4, 5 - 6]" },
}

test_case! {
    name: array_with_weird_spacing,
    input: { "[  1  ,\t2,3   ]" },
    // Normalize weird whitespace and tabs
    formatted: { "[1, 2, 3]" },
}

test_case! {
    name: array_with_weird_spacing_and_newlines,
    input: {"
[  1  ,
\t2,3   ]
"},
    // Normalize weird whitespace and newline implies multi-line
    formatted: { r"
[
    1,
    2,
    3,
]
".trim_start() },
}

test_case! {
    name: array_with_comments,
    input: {r"
[
    1,     // first
    2,// second
    3 // third
]".trim_start()},
    // Comments in arrays - should align vertically and add trailing comma
    formatted: {r"
[
    1, // first
    2, // second
    3, // third
]".trim_start()},
}

test_case! {
    name: array_mixed_expressions,
    input: { "[1+2*3, (4-5)/6, 7^8]" },
    // Complex expressions with operator precedence
    formatted: { "[1 + 2 * 3, (4 - 5) / 6, 7 ^ 8]" },
}

test_case! {
    name: array_with_where,
    input: { "[1+2*3, (a-b)/c where {a = 4, b = 5, c = 6}, 7^8]" },
    // Complex expressions with operator precedence
    formatted: { "[1 + 2 * 3, (a - b) / c where { a = 4, b = 5, c = 6 }, 7 ^ 8]" },
}

test_case! {
    name: array_with_multiline_where,
    input: {"
[1+2*3, (a-b)/c where {
                    a = 4,
                    b = 5,
                    c = 6
                }, 7^8]".trim_start()},
    // Complex expressions with operator precedence
    formatted: {r"
[
    1 + 2 * 3,
    (a - b) / c where {
        a = 4,
        b = 5,
        c = 6,
    },
    7 ^ 8,
]".trim_start()},
}

test_case! {
    name: array_empty_with_whitespace,
    input: { "[\n\n]" },
    // Empty array with newlines inside
    formatted: { "[]" },
}

test_case! {
    name: array_single_element_trailing_comma,
    input: { "[42,]" },
    // Single element with trailing comma - should remove it
    formatted: { "[42]" },
}
