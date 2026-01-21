//! Integration tests for the `run` command.

use assert_cmd::Command;
use predicates::prelude::*;
use std::io::Write;

fn melbi() -> Command {
    Command::new(env!("CARGO_BIN_EXE_melbi-cli"))
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
fn run_simple_expression() {
    let file = temp_file("1 + 2");

    melbi()
        .args(["run", file.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout("3\n");
}

#[test]
fn run_arithmetic() {
    let file = temp_file("10 * 5 - 3");

    melbi()
        .args(["run", file.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout("47\n");
}

#[test]
fn run_where_binding() {
    let file = temp_file("x + y where { x = 10, y = 20 }");

    melbi()
        .args(["run", file.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout("30\n");
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

    melbi()
        .args(["run", file.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout("3\n");
}

#[test]
fn run_stdlib_function() {
    let file = temp_file("Math.Floor(3.7)");

    melbi()
        .args(["run", file.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout("3\n");
}

#[test]
fn run_with_runtime_evaluator() {
    let file = temp_file("1 + 2");

    melbi()
        .args(["run", "--runtime", "evaluator", file.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout("3\n");
}

#[test]
fn run_with_runtime_vm() {
    let file = temp_file("1 + 2");

    melbi()
        .args(["run", "--runtime", "vm", file.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout("3\n");
}

#[test]
fn run_type_error() {
    let file = temp_file("1 + true");

    melbi()
        .args(["run", file.path().to_str().unwrap()])
        .assert()
        .success() // CLI exits 0 but prints error
        .stderr(predicate::str::contains("Type mismatch"));
}

#[test]
fn run_parse_error() {
    let file = temp_file("1 + +");

    melbi()
        .args(["run", file.path().to_str().unwrap()])
        .assert()
        .success() // CLI exits 0 but prints error
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
        .success()
        .stderr(predicate::str::contains("Type mismatch"))
        .stderr(predicate::str::contains("\x1b[").not());
}

// ============================================================================
// Stdin support
// ============================================================================

#[test]
fn run_from_stdin() {
    melbi()
        .args(["run", "-"])
        .write_stdin("1 + 2")
        .assert()
        .success()
        .stdout("3\n");
}

#[test]
fn run_from_stdin_multiline() {
    melbi()
        .args(["run", "-"])
        .write_stdin("x + y where {\n    x = 10,\n    y = 20,\n}")
        .assert()
        .success()
        .stdout("30\n");
}

#[test]
fn run_from_stdin_with_runtime() {
    melbi()
        .args(["run", "--runtime", "vm", "-"])
        .write_stdin("5 * 6")
        .assert()
        .success()
        .stdout("30\n");
}

#[test]
fn run_from_stdin_error() {
    melbi()
        .args(["run", "-"])
        .write_stdin("1 + true")
        .assert()
        .success() // CLI exits 0 but prints error
        .stderr(predicate::str::contains("Type mismatch"));
}

// ============================================================================
// Error output format tests
// ============================================================================

#[test]
fn run_error_shows_filename() {
    let file = temp_file("1 + true");
    let path = file.path();
    let path_str = path.to_str().unwrap();

    let output = melbi()
        .args(["--no-color", "run", path_str])
        .assert()
        .success() // CLI exits 0 but prints error
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
fn run_error_shows_stdin_label() {
    let output = melbi()
        .args(["--no-color", "run", "-"])
        .write_stdin("1 + true")
        .assert()
        .success() // CLI exits 0 but prints error
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
