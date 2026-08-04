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
    name: empty_map,
    input: { "{}" },
    formatted: { "{}" },
}

test_case! {
    name: single_line_map,
    input: { "{a:1,b:2}" },
    formatted: { "{a: 1, b: 2}" },
}

test_case! {
    name: multi_line_map,
    input: { r"
{a:1,
 b:2}".trim_start()},
    formatted: { r"
{
    a: 1,
    b: 2,
}".trim_start() },
}

test_case! {
    name: delete_trailing_comma_single_line,
    input: { "{a:1,b:2,}" },
    formatted: { "{a: 1, b: 2}" },
}

test_case! {
    name: multi_line_map_respects_newlines,
    input: { r"
{
a:1, b:2}".trim_start()},
    formatted: { r"
{
    a: 1, b: 2,
}".trim_start() },
}

test_case! {
    name: map_with_complex_keys,
    input: { "{1+2:3, 4:5*6}" },
    formatted: { "{1 + 2: 3, 4: 5 * 6}" },
}

test_case! {
    name: map_with_weird_spacing,
    input: { "{  a:1  ,  b:2  }" },
    formatted: { "{a: 1, b: 2}" },
    // Normalize weird whitespace
}

test_case! {
    name: map_with_comments,
    input: { r"
{
    a   : 1, // first
    b
    : 2 // second
    , c: 3_000_000   // third
}".trim_start()},
    formatted: { r"
{
    a: 1, // first
    b: 2, // second
    c: 3_000_000, // third
}".trim_start() },
    // Comments in maps - should align vertically and add trailing comma
}

test_case! {
    name: map_nested,
    input: { "{{a:1}: {b:2}}" },
    formatted: { "{{a: 1}: {b: 2}}" },
    // Nested maps - no spaces inside braces
}

test_case! {
    name: map_empty_with_whitespace,
    input: { "{\n\n}" },
    formatted: { "{}" },
    // Empty map with newlines inside
}
