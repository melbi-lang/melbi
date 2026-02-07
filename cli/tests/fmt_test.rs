//! Integration tests for the `fmt` command.

mod common;

use common::{check_output, check_stderr, check_stdout, melbi, temp_file};
use expect_test::expect;
use predicates::prelude::*;
use std::fs;

// ============================================================================
// Success tests with full output verification
// ============================================================================

#[test]
fn fmt_from_stdin() {
    check_stdout(&["fmt", "-"], Some("1   +   2"), expect!["1 + 2"]);
}

#[test]
fn fmt_from_stdin_already_formatted() {
    check_stdout(&["fmt", "-"], Some("1 + 2"), expect![""]);
}

#[test]
fn fmt_from_stdin_multiline() {
    check_stdout(
        &["fmt", "-"],
        Some("x+y where{x=1,y=2}"),
        expect!["x + y where { x = 1, y = 2 }"],
    );
}

// ============================================================================
// Error tests with full output verification
// ============================================================================

#[test]
fn fmt_parse_error() {
    check_stderr(
        &["fmt", "-"],
        Some("1 + +"),
        expect![[r#"
            error: -: parse error at 1:3
        "#]],
    );
}

#[test]
fn fmt_from_stdin_write_error() {
    check_stderr(
        &["fmt", "--write", "-"],
        Some("1   +   2"),
        expect![[r#"
            error: -: cannot use --write with stdin
        "#]],
    );
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
    check_stdout(&["fmt", file.path().to_str().unwrap()], None, expect![""]);
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
// Idempotency
// ============================================================================

#[test]
fn fmt_is_idempotent() {
    let inputs = ["{x=1,y=2}", "1+2+3", "x+y where{x=1,y=2}"];

    for input in inputs {
        let file = temp_file(input);
        let path = file.path().to_str().unwrap();

        // First format
        melbi().args(["fmt", "--write", path]).assert().success();
        let first = fs::read_to_string(file.path()).unwrap();

        // Second format should not change
        melbi().args(["fmt", "--write", path]).assert().success();
        let second = fs::read_to_string(file.path()).unwrap();

        assert_eq!(first, second, "Formatting should be idempotent for: {}", input);
    }
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

    check_stdout(&["fmt", "--write", path.to_str().unwrap()], None, expect![""]);

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
    check_stdout(
        &["fmt", "--check", file.path().to_str().unwrap()],
        None,
        expect![""],
    );
}

#[test]
fn fmt_from_stdin_check() {
    check_stdout(
        &["fmt", "--check", "-"],
        Some("1   +   2"),
        expect!["<stdin> needs formatting\n"],
    );
}

#[test]
fn fmt_from_stdin_check_formatted() {
    check_stdout(&["fmt", "--check", "-"], Some("1 + 2"), expect![""]);
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
fn fmt_file_parse_error() {
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

// ============================================================================
// --quiet flag
// ============================================================================

#[test]
fn fmt_quiet_no_output() {
    let file = temp_file("1   +   2");
    check_output(
        &["fmt", "--quiet", file.path().to_str().unwrap()],
        None,
        expect![""],
        expect![""],
    );
}

#[test]
fn fmt_quiet_short_flag() {
    let file = temp_file("1   +   2");
    check_output(
        &["fmt", "-q", file.path().to_str().unwrap()],
        None,
        expect![""],
        expect![""],
    );
}

#[test]
fn fmt_check_quiet_unformatted() {
    let file = temp_file("1   +   2");
    check_output(
        &["fmt", "--check", "--quiet", file.path().to_str().unwrap()],
        None,
        expect![""],
        expect![""],
    );
}

#[test]
fn fmt_check_quiet_formatted() {
    let file = temp_file("1 + 2");
    check_output(
        &["fmt", "--check", "--quiet", file.path().to_str().unwrap()],
        None,
        expect![""],
        expect![""],
    );
}

#[test]
fn fmt_write_quiet() {
    let file = temp_file("1   +   2");
    let path = file.path().to_path_buf();

    check_output(
        &["fmt", "--write", "--quiet", path.to_str().unwrap()],
        None,
        expect![""],
        expect![""],
    );

    // File should still be modified
    assert_eq!(fs::read_to_string(&path).unwrap(), "1 + 2");
}

#[test]
fn fmt_quiet_stdin() {
    check_output(&["fmt", "--quiet", "-"], Some("1   +   2"), expect![""], expect![""]);
}

#[test]
fn fmt_check_quiet_stdin() {
    check_output(
        &["fmt", "--check", "--quiet", "-"],
        Some("1   +   2"),
        expect![""],
        expect![""],
    );
}

#[test]
fn fmt_quiet_error() {
    let file = temp_file("1 + +");
    check_output(
        &["fmt", "--quiet", file.path().to_str().unwrap()],
        None,
        expect![""],
        expect![""],
    );
}

// ============================================================================
// Shebang support
// ============================================================================

#[test]
fn fmt_preserves_shebang() {
    let file = temp_file("#!/usr/bin/env melbi run\n1+2+3");

    melbi()
        .args(["fmt", file.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("#!/usr/bin/env melbi run"))
        .stdout(predicate::str::contains("+1 + 2 + 3"));
}

#[test]
fn fmt_write_preserves_shebang() {
    let file = temp_file("#!/usr/bin/env melbi run\n1+2+3");
    let path = file.path().to_path_buf();

    melbi()
        .args(["fmt", "--write", path.to_str().unwrap()])
        .assert()
        .success();

    let content = fs::read_to_string(&path).unwrap();
    assert!(content.starts_with("#!/usr/bin/env melbi run\n"));
    assert!(content.contains("1 + 2 + 3"));
}

#[test]
fn fmt_check_with_shebang_unformatted() {
    let file = temp_file("#!/usr/bin/env melbi run\n1+2+3");

    melbi()
        .args(["fmt", "--check", file.path().to_str().unwrap()])
        .assert()
        .failure()
        .stdout(predicate::str::contains("needs formatting"));
}

#[test]
fn fmt_check_with_shebang_formatted() {
    let file = temp_file("#!/usr/bin/env melbi run\n1 + 2 + 3");
    check_stdout(
        &["fmt", "--check", file.path().to_str().unwrap()],
        None,
        expect![""],
    );
}

#[test]
fn fmt_shebang_from_stdin() {
    check_stdout(
        &["fmt", "-"],
        Some("#!/usr/bin/env melbi run\n1+2"),
        expect!["#!/usr/bin/env melbi run\n1 + 2"],
    );
}

#[test]
fn fmt_shebang_already_formatted() {
    let file = temp_file("#!/usr/bin/env melbi run\n1 + 2 + 3");
    check_stdout(&["fmt", file.path().to_str().unwrap()], None, expect![""]);
}
