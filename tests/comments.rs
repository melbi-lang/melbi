// ============================================================================
// REVIEWED & LOCKED - Test expectations are set in stone
// Date: 2024-10-14
// All test expectations in this file have been reviewed and approved.
// DO NOT change expectations without explicit discussion.
// If tests fail, fix the formatter, not the tests.
// ============================================================================

mod cases;

// TODO: Comment formatting has many edge cases to handle
// Comments use `//` to end of line

test_case! {
    name: comment_after_expression,
    input: { "42// the answer" },
    formatted: { "42 // the answer" },
} // Two spaces before end-of-line comments (Google style)

test_case! {
    name: comment_in_record,
    input: { r"
{
    a = 1,  // first
    b = 2          // second
}".trim_start()},
    formatted: { r"
{
    a = 1, // first
    b = 2, // second
}".trim_start() },
} // Two spaces before end-of-line comments, trailing comma added

test_case! {
    name: comment_attached_to_comma,
    input: { r"
{
    a = 1,
    b = 7  // my favorite number
    ,
}".trim_start()},
    formatted: { r"
{
    a = 1,
    b = 7, // my favorite number
}".trim_start() },
} // Tricky: comment semantically attached to value, but syntactically moves with comma

test_case! {
    name: standalone_comment_in_block,
    input: { r"
{
    // This is a standalone comment
    a = 1,
    b = 2
}".trim_start()},
    formatted: { r"
{
    // This is a standalone comment
    a = 1,
    b = 2,
}".trim_start()},
}

test_case! {
    name: multiple_comments,
    input: { r"
// Header comment
// More header
42// inline comment
".trim_start() },
    formatted: { r"
// Header comment
// More header
42 // inline comment
".trim_start() },
}

test_case! {
    name: comment_in_where,
    input: { r"
result where {
    // Calculate delta
    delta = b^2 - 4*a*c,
    // First root
    r0 = (- b + delta^0.5) / (2*a)// positive discriminant
}".trim_start()},
    formatted: { r"
result where {
    // Calculate delta
    delta = b ^ 2 - 4 * a * c,
    // First root
    r0 = (-b + delta ^ 0.5) / (2 * a), // positive discriminant
}".trim_start() },
} // Two spaces before end-of-line comment

test_case! {
    name: comment_after_operator,
    input: { r"
a +// what comes next?
b".trim_start() },
    formatted: { r"
a + // what comes next?
b".trim_start() },
} // Two spaces before comment, even after operator

test_case! {
    name: comment_in_format_string,
    input: { r#"
f"Hello {
    // arbitrary expressions accepted
    "Copilot"
} from Melbi 🖖"
"#.trim_start()},
    formatted: { r#"
f"Hello {
    // arbitrary expressions accepted
    "Copilot"
} from Melbi 🖖"
"#.trim_start() },
} // Comments inside format string interpolations
