//! Integration tests for the `fmt` command.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
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

// ============================================================================
// Default behavior: diff output
// ============================================================================

#[test]
fn fmt_shows_diff_by_default() {
    let file = temp_file("1   +    2");

    melbi()
        .args(["fmt", file.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("---"))
        .stdout(predicate::str::contains("+++"))
        .stdout(predicate::str::contains("-1   +    2"))
        .stdout(predicate::str::contains("+1 + 2"));
}

#[test]
fn fmt_no_output_when_already_formatted() {
    let file = temp_file("1 + 2");

    melbi()
        .args(["fmt", file.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn fmt_diff_has_colors_by_default() {
    let file = temp_file("1   +    2");

    melbi()
        .args(["fmt", file.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b["));
}

#[test]
fn fmt_no_color_flag() {
    let file = temp_file("1   +    2");

    melbi()
        .args(["--no-color", "fmt", file.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[").not());
}

// ============================================================================
// --write flag
// ============================================================================

#[test]
fn fmt_write_modifies_file() {
    let file = temp_file("1   +    2");
    let path = file.path().to_path_buf();

    melbi()
        .args(["fmt", "--write", path.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("formatted"));

    let content = fs::read_to_string(&path).unwrap();
    assert_eq!(content, "1 + 2");
}

#[test]
fn fmt_write_no_output_when_already_formatted() {
    let file = temp_file("1 + 2");
    let path = file.path().to_path_buf();

    melbi()
        .args(["fmt", "--write", path.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    // File unchanged
    let content = fs::read_to_string(&path).unwrap();
    assert_eq!(content, "1 + 2");
}

// ============================================================================
// --check flag
// ============================================================================

#[test]
fn fmt_check_exits_1_when_unformatted() {
    let file = temp_file("1   +    2");

    melbi()
        .args(["fmt", "--check", file.path().to_str().unwrap()])
        .assert()
        .failure()
        .stdout(predicate::str::contains("needs formatting"));
}

#[test]
fn fmt_check_exits_0_when_formatted() {
    let file = temp_file("1 + 2");

    melbi()
        .args(["fmt", "--check", file.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

// ============================================================================
// Multiple files
// ============================================================================

#[test]
fn fmt_multiple_files() {
    let file1 = temp_file("1   +   2");
    let file2 = temp_file("3   *   4");

    melbi()
        .args([
            "fmt",
            file1.path().to_str().unwrap(),
            file2.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("+1 + 2"))
        .stdout(predicate::str::contains("+3 * 4"));
}

#[test]
fn fmt_check_multiple_files_some_unformatted() {
    let formatted = temp_file("1 + 2");
    let unformatted = temp_file("3   *   4");

    melbi()
        .args([
            "fmt",
            "--check",
            formatted.path().to_str().unwrap(),
            unformatted.path().to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("needs formatting"));
}

#[test]
fn fmt_write_multiple_files() {
    let file1 = temp_file("1   +   2");
    let file2 = temp_file("3   *   4");
    let path1 = file1.path().to_path_buf();
    let path2 = file2.path().to_path_buf();

    melbi()
        .args([
            "fmt",
            "--write",
            path1.to_str().unwrap(),
            path2.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("formatted").count(2));

    assert_eq!(fs::read_to_string(&path1).unwrap(), "1 + 2");
    assert_eq!(fs::read_to_string(&path2).unwrap(), "3 * 4");
}

// ============================================================================
// Error handling
// ============================================================================

#[test]
fn fmt_parse_error() {
    let file = temp_file("1 + +");

    melbi()
        .args(["fmt", file.path().to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("parse error"));
}

#[test]
fn fmt_nonexistent_file() {
    melbi()
        .args(["fmt", "/nonexistent/path/to/file.melbi"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No such file"));
}

// ============================================================================
// Formatting specific constructs
// ============================================================================

#[test]
fn fmt_where_binding() {
    let file = temp_file("x+y where{x=1,y=2}");

    melbi()
        .args(["fmt", file.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("+x + y where { x = 1, y = 2 }"));
}

#[test]
fn fmt_array() {
    let file = temp_file("[1,2,3]");

    melbi()
        .args(["fmt", file.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("+[1, 2, 3]"));
}

#[test]
fn fmt_record() {
    let file = temp_file("{x=1,y=2}");

    melbi()
        .args(["fmt", file.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("+{ x = 1, y = 2 }"));
}
