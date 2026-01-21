//! Integration tests for the `check` command.

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
