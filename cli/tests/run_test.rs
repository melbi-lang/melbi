//! Integration tests for the `run` command.

mod common;

use common::{check_stderr, check_stdout, melbi, temp_file};
use expect_test::expect;
use predicates::prelude::*;

// ============================================================================
// Success tests with full output verification
// ============================================================================

#[test]
fn run_simple_expression() {
    let file = temp_file("1 + 2");
    check_stdout(&["run", file.path().to_str().unwrap()], None, expect!["3\n"]);
}

#[test]
fn run_arithmetic() {
    let file = temp_file("10 * 5 - 3");
    check_stdout(&["run", file.path().to_str().unwrap()], None, expect!["47\n"]);
}

#[test]
fn run_where_binding() {
    let file = temp_file("x + y where { x = 10, y = 20 }");
    check_stdout(&["run", file.path().to_str().unwrap()], None, expect!["30\n"]);
}

#[test]
fn run_multiline_expression() {
    let file = temp_file(
        r#"result where {
    a = 1,
    b = 2,
    result = a + b,
}"#,
    );
    check_stdout(&["run", file.path().to_str().unwrap()], None, expect!["3\n"]);
}

#[test]
fn run_stdlib_function() {
    let file = temp_file("Math.Floor(3.7)");
    check_stdout(&["run", file.path().to_str().unwrap()], None, expect!["3\n"]);
}

#[test]
fn run_with_runtime_evaluator() {
    let file = temp_file("1 + 2");
    check_stdout(
        &["run", "--runtime", "evaluator", file.path().to_str().unwrap()],
        None,
        expect!["3\n"],
    );
}

#[test]
fn run_with_runtime_vm() {
    let file = temp_file("1 + 2");
    check_stdout(
        &["run", "--runtime", "vm", file.path().to_str().unwrap()],
        None,
        expect!["3\n"],
    );
}

#[test]
fn run_with_runtime_both_explicit() {
    let file = temp_file("1 + 2");
    check_stdout(
        &["run", "--runtime", "both", file.path().to_str().unwrap()],
        None,
        expect!["3\n"],
    );
}

// ============================================================================
// Error tests with full output verification
// ============================================================================

#[test]
fn run_type_error() {
    let file = temp_file("1 + true");

    melbi()
        .args(["run", file.path().to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Type mismatch"));
}

#[test]
fn run_parse_error() {
    let file = temp_file("1 + +");

    melbi()
        .args(["run", file.path().to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error"));
}

#[test]
fn run_nonexistent_file() {
    melbi()
        .args(["run", "/nonexistent/path/to/file.melbi"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No such file"));
}

#[test]
fn run_no_color_flag() {
    let file = temp_file("1 + true");

    melbi()
        .args(["--no-color", "run", file.path().to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Type mismatch"))
        .stderr(predicate::str::contains("\x1b[").not());
}

// ============================================================================
// Stdin support
// ============================================================================

#[test]
fn run_from_stdin() {
    check_stdout(&["run", "-"], Some("1 + 2"), expect!["3\n"]);
}

#[test]
fn run_from_stdin_multiline() {
    check_stdout(
        &["run", "-"],
        Some("x + y where {\n    x = 10,\n    y = 20,\n}"),
        expect!["30\n"],
    );
}

#[test]
fn run_from_stdin_with_runtime() {
    check_stdout(&["run", "--runtime", "vm", "-"], Some("5 * 6"), expect!["30\n"]);
}

/// Test that errors from stdin are displayed correctly with full error message.
///
/// This test uses exact string matching to catch double-rendering bugs
/// where errors might be printed multiple times.
#[test]
fn run_from_stdin_error() {
    check_stderr(
        &["--no-color", "run", "-"],
        Some("1 + true"),
        expect![[r#"
            [E001] Error: Type mismatch: expected Int, found Bool
               ╭─[ <stdin>:1:5 ]
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

// ============================================================================
// Error output format tests
// ============================================================================

#[test]
fn run_error_shows_filename() {
    let file = temp_file("1 + true");
    let path_str = file.path().to_str().unwrap();

    melbi()
        .args(["--no-color", "run", path_str])
        .assert()
        .failure()
        .stderr(predicate::str::contains(path_str));
}

#[test]
fn run_error_shows_stdin_label() {
    melbi()
        .args(["--no-color", "run", "-"])
        .write_stdin("1 + true")
        .assert()
        .failure()
        .stderr(predicate::str::contains("<stdin>"));
}
