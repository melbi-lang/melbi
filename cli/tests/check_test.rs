//! Integration tests for the `check` command.

use assert_cmd::Command;
use predicates::prelude::*;
use std::io::Write;

fn melbi() -> Command {
    Command::new(env!("CARGO_BIN_EXE_melbi"))
}

fn temp_file(content: &str) -> tempfile::NamedTempFile {
    let mut file = tempfile::Builder::new()
        .suffix(".melbi")
        .tempfile()
        .unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file
}

#[test]
fn check_valid_expression() {
    let file = temp_file("1 + 2");

    melbi()
        .args(["check", file.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("OK"));
}

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
        .stdout(predicate::str::contains("OK"));
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

    melbi()
        .args(["check", "--quiet", file.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn check_quiet_failure() {
    let file = temp_file("1 + true");

    melbi()
        .args(["check", "--quiet", file.path().to_str().unwrap()])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn check_quiet_multiple_files() {
    let valid = temp_file("1 + 2");
    let invalid = temp_file("1 + true");

    melbi()
        .args([
            "check",
            "--quiet",
            valid.path().to_str().unwrap(),
            invalid.path().to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn check_quiet_short_flag() {
    let file = temp_file("1 + 2");

    melbi()
        .args(["check", "-q", file.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

// ============================================================================
// Stdin support
// ============================================================================

#[test]
fn check_from_stdin() {
    melbi()
        .args(["check", "-"])
        .write_stdin("1 + 2")
        .assert()
        .success()
        .stdout(predicate::str::contains("<stdin>: OK"));
}

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
    melbi()
        .args(["check", "--quiet", "-"])
        .write_stdin("1 + 2")
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn check_from_stdin_quiet_error() {
    melbi()
        .args(["check", "--quiet", "-"])
        .write_stdin("1 + true")
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

// ============================================================================
// Error output format tests
// ============================================================================

#[test]
fn check_error_shows_filename() {
    let file = temp_file("1 + true");
    let path = file.path();
    let path_str = path.to_str().unwrap();

    let output = melbi()
        .args(["--no-color", "check", path_str])
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8_lossy(&output);

    // Error should contain the filename
    assert!(
        stderr.contains(path_str),
        "Error should contain filename, got:\n{}",
        stderr
    );
}

#[test]
fn check_error_shows_stdin_label() {
    let output = melbi()
        .args(["--no-color", "check", "-"])
        .write_stdin("1 + true")
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8_lossy(&output);

    // Error should show <stdin> as the source
    assert!(
        stderr.contains("<stdin>"),
        "Error should contain <stdin>, got:\n{}",
        stderr
    );
}

#[test]
fn check_type_error_output_format() {
    let output = melbi()
        .args(["--no-color", "check", "-"])
        .write_stdin("1 + true")
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8_lossy(&output);

    // Verify key parts of the error format
    assert!(stderr.contains("[E001]"), "Should have error code E001");
    assert!(stderr.contains("Type mismatch"), "Should mention type mismatch");
    assert!(stderr.contains("expected Int"), "Should mention expected Int");
    assert!(stderr.contains("found Bool"), "Should mention found Bool");
    assert!(stderr.contains("1 + true"), "Should show the source code");
    assert!(stderr.contains("<stdin>:1:"), "Should show line number");
}

#[test]
fn check_undefined_variable_output_format() {
    let output = melbi()
        .args(["--no-color", "check", "-"])
        .write_stdin("undefined_var + 1")
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8_lossy(&output);

    // Verify key parts of the error format
    assert!(stderr.contains("[E002]"), "Should have error code E002");
    assert!(
        stderr.contains("Undefined variable"),
        "Should mention undefined variable"
    );
    assert!(
        stderr.contains("undefined_var"),
        "Should mention the variable name"
    );
}

#[test]
fn check_parse_error_output_format() {
    let output = melbi()
        .args(["--no-color", "check", "-"])
        .write_stdin("1 + +")
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8_lossy(&output);

    // Parse errors should show the source
    assert!(stderr.contains("1 + +"), "Should show the source code");
    assert!(stderr.contains("<stdin>"), "Should show stdin as source");
}
