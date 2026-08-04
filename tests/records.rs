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
    name: empty_record,
    input: { "Record {}" },
    formatted: { "Record{}" },
}

test_case! {
    name: empty_record_with_newlines,
    input: { r"
Record

{
}".trim_start() },
    formatted: { "Record{}" },
}

test_case! {
    name: single_line_record,
    input: { "{x =  1   ,    y     =      2       }" },
    formatted: { "{ x = 1, y = 2 }" },
}

test_case! {
    name: multi_line_record,
    input: { r"
{x=1,
y=2}".trim_start() },
    formatted: { r"
{
    x = 1,
    y = 2,
}".trim_start() },
}

test_case! {
    name: delete_trailing_comma_single_line,
    input: { "{x=1+2,y=3*4,}" },
    formatted: { "{ x = 1 + 2, y = 3 * 4 }" },
}

test_case! {
    name: multi_line_record_respects_newlines,
    input: { r"
{
x=1,y={z=3}}".trim_start() },
    formatted: { r"
{
    x = 1, y = { z = 3 },
}".trim_start() },
}

test_case! {
    name: record_with_comments,
    input: { r"
{
    x = 1, // first

    y = 10,// second
    z = 100     // third
}".trim_start() },
    formatted: { r"
{
    x = 1, // first
    y = 10, // second
    z = 100, // third
}".trim_start() },
    // Comments in records - should align vertically and add trailing comma
}
