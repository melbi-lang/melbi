//! Integration tests for the `eval` command.

mod common;

use common::{check_stderr, check_stdout, melbi};
use expect_test::expect;
use predicates::prelude::*;

// ============================================================================
// Success tests with full output verification
// ============================================================================

#[test]
fn eval_simple_expression() {
    check_stdout(&["eval", "1 + 2"], None, expect!["3\n"]);
}

#[test]
fn eval_arithmetic() {
    check_stdout(&["eval", "10 * 5 - 3"], None, expect!["47\n"]);
}

#[test]
fn eval_string() {
    check_stdout(&["eval", r#""hello""#], None, expect!["\"hello\"\n"]);
}

#[test]
fn eval_boolean() {
    check_stdout(&["eval", "true and false"], None, expect!["false\n"]);
}

#[test]
fn eval_array() {
    check_stdout(&["eval", "[1, 2, 3]"], None, expect!["[1, 2, 3]\n"]);
}

#[test]
fn eval_record() {
    check_stdout(&["eval", "{ x = 1, y = 2 }"], None, expect![[r#"
        {x = 1, y = 2}
    "#]]);
}

#[test]
fn eval_where_binding() {
    check_stdout(
        &["eval", "x + y where { x = 10, y = 20 }"],
        None,
        expect!["30\n"],
    );
}

#[test]
fn eval_if_expression() {
    check_stdout(&["eval", "if true then 42 else 0"], None, expect!["42\n"]);
}

#[test]
fn eval_lambda() {
    check_stdout(&["eval", "((x) => x * 2)(21)"], None, expect!["42\n"]);
}

#[test]
fn eval_stdlib_math() {
    check_stdout(&["eval", "Math.Floor(3.7)"], None, expect!["3\n"]);
}

#[test]
fn eval_stdlib_string() {
    check_stdout(&["eval", "String.Len(\"hello\")"], None, expect!["5\n"]);
}

// ============================================================================
// Error tests with full output verification
// ============================================================================

/// Test that type errors are displayed correctly with full error message.
///
/// This test uses exact string matching to catch double-rendering bugs
/// where errors might be printed multiple times.
#[test]
fn eval_type_error_shows_error() {
    check_stderr(
        &["--no-color", "eval", "1 + true"],
        None,
        expect![[r#"
            [E001] Error: Type mismatch: expected Int, found Bool
               ╭─[ <unknown>:1:5 ]
               │
             1 │ 1 + true
               │     ──┬─  
               │       ╰─── Type mismatch: expected Int, found Bool
               │ 
               │ Help: Types must match in this context
            ───╯
        "#]],
    );
}

#[test]
fn eval_parse_error_shows_error() {
    check_stderr(
        &["--no-color", "eval", "1 + +"],
        None,
        expect![[r#"
            [P001] Error: Expected expression, literal or identifier, found unexpected token
               ╭─[ <unknown>:1:5 ]
               │
             1 │ 1 + +
               │     │ 
               │     ╰─ Expected expression, literal or identifier, found unexpected token
            ───╯
        "#]],
    );
}

// ============================================================================
// Runtime behavior tests
// ============================================================================

#[test]
fn eval_runtime_error_shows_error() {
    melbi()
        .args(["eval", "1 / 0"])
        .assert()
        .failure() // Runtime errors exit non-zero
        .stderr(predicate::str::contains("division by zero").or(predicate::str::contains("Division")));
}

#[test]
fn eval_with_runtime_evaluator() {
    check_stdout(&["eval", "--runtime", "evaluator", "1 + 2"], None, expect!["3\n"]);
}

#[test]
fn eval_with_runtime_vm() {
    check_stdout(&["eval", "--runtime", "vm", "1 + 2"], None, expect!["3\n"]);
}

#[test]
fn eval_no_color_flag() {
    melbi()
        .args(["--no-color", "eval", "1 + true"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Type mismatch"))
        // No ANSI escape codes when --no-color is used
        .stderr(predicate::str::contains("\x1b[").not());
}
