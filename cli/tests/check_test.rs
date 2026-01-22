//! Integration tests for the `check` command.

mod common;

use common::{check_output, check_stderr, check_stdout, melbi, temp_file};
use expect_test::expect;
use predicates::prelude::*;

// ============================================================================
// Success tests with full output verification
// ============================================================================

#[test]
fn check_valid_expression() {
    let file = temp_file("1 + 2");
    melbi()
        .args(["check", file.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::ends_with(": OK\n"));
}

#[test]
fn check_from_stdin() {
    check_stdout(&["check", "-"], Some("1 + 2"), expect!["<stdin>: OK\n"]);
}

#[test]
fn check_complex_expression() {
    let file = temp_file(
        r#"result where {
    double = (x) => x * 2,
    result = double(21),
}"#,
    );
    melbi()
        .args(["check", file.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::ends_with(": OK\n"));
}

// ============================================================================
// Error tests with full output verification
// ============================================================================

#[test]
fn check_type_error_output_format() {
    check_stderr(
        &["--no-color", "check", "-"],
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

#[test]
fn check_undefined_variable_output_format() {
    check_stderr(
        &["--no-color", "check", "-"],
        Some("undefined_var + 1"),
        expect![[r#"
            [E002] Error: Undefined variable 'undefined_var'
               ╭─[ <stdin>:1:1 ]
               │
             1 │ undefined_var + 1
               │ ──────┬──────  
               │       ╰──────── Undefined variable 'undefined_var'
               │ 
               │ Help: Make sure the variable is declared before use
            ───╯
        "#]],
    );
}

#[test]
fn check_parse_error_output_format() {
    check_stderr(
        &["--no-color", "check", "-"],
        Some("1 + +"),
        expect![[r#"
            [P001] Error: Expected expression, literal or identifier, found unexpected token
               ╭─[ <stdin>:1:5 ]
               │
             1 │ 1 + +
               │     │ 
               │     ╰─ Expected expression, literal or identifier, found unexpected token
            ───╯
        "#]],
    );
}

// ============================================================================
// Multiple files
// ============================================================================

#[test]
fn check_multiple_valid_files() {
    let file1 = temp_file("1 + 2");
    let file2 = temp_file("true and false");
    let file3 = temp_file("[1, 2, 3]");

    melbi()
        .args([
            "check",
            file1.path().to_str().unwrap(),
            file2.path().to_str().unwrap(),
            file3.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("OK").count(3));
}

#[test]
fn check_type_error() {
    let file = temp_file("1 + true");

    melbi()
        .args(["check", file.path().to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Type mismatch"));
}

#[test]
fn check_parse_error() {
    let file = temp_file("1 + +");

    melbi()
        .args(["check", file.path().to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error"));
}

#[test]
fn check_undefined_variable() {
    let file = temp_file("undefined_var + 1");

    melbi()
        .args(["check", file.path().to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Undefined variable"));
}

#[test]
fn check_mixed_valid_and_invalid() {
    let valid = temp_file("1 + 2");
    let invalid = temp_file("1 + true");

    melbi()
        .args([
            "check",
            valid.path().to_str().unwrap(),
            invalid.path().to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("OK"))
        .stderr(predicate::str::contains("Type mismatch"));
}

#[test]
fn check_nonexistent_file() {
    melbi()
        .args(["check", "/nonexistent/path/to/file.melbi"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No such file"));
}

#[test]
fn check_no_color_flag() {
    let file = temp_file("1 + true");

    melbi()
        .args(["--no-color", "check", file.path().to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Type mismatch"))
        .stderr(predicate::str::contains("\x1b[").not());
}

// ============================================================================
// --quiet flag
// ============================================================================

#[test]
fn check_quiet_success() {
    let file = temp_file("1 + 2");
    check_output(
        &["check", "--quiet", file.path().to_str().unwrap()],
        None,
        expect![""],
        expect![""],
    );
}

#[test]
fn check_quiet_failure() {
    let file = temp_file("1 + true");
    check_output(
        &["check", "--quiet", file.path().to_str().unwrap()],
        None,
        expect![""],
        expect![""],
    );
}

#[test]
fn check_quiet_multiple_files() {
    let valid = temp_file("1 + 2");
    let invalid = temp_file("1 + true");
    check_output(
        &[
            "check",
            "--quiet",
            valid.path().to_str().unwrap(),
            invalid.path().to_str().unwrap(),
        ],
        None,
        expect![""],
        expect![""],
    );
}

#[test]
fn check_quiet_short_flag() {
    let file = temp_file("1 + 2");
    check_output(
        &["check", "-q", file.path().to_str().unwrap()],
        None,
        expect![""],
        expect![""],
    );
}

// ============================================================================
// Stdin support
// ============================================================================

#[test]
fn check_from_stdin_error() {
    melbi()
        .args(["check", "-"])
        .write_stdin("1 + true")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Type mismatch"));
}

#[test]
fn check_from_stdin_quiet() {
    check_output(&["check", "--quiet", "-"], Some("1 + 2"), expect![""], expect![""]);
}

#[test]
fn check_from_stdin_quiet_error() {
    check_output(
        &["check", "--quiet", "-"],
        Some("1 + true"),
        expect![""],
        expect![""],
    );
}

// ============================================================================
// Error output format tests
// ============================================================================

#[test]
fn check_error_shows_filename() {
    let file = temp_file("1 + true");
    let path_str = file.path().to_str().unwrap();

    melbi()
        .args(["--no-color", "check", path_str])
        .assert()
        .failure()
        .stderr(predicate::str::contains(path_str));
}

#[test]
fn check_error_shows_stdin_label() {
    melbi()
        .args(["--no-color", "check", "-"])
        .write_stdin("1 + true")
        .assert()
        .failure()
        .stderr(predicate::str::contains("<stdin>"));
}
