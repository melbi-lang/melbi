//! Shared test utilities for CLI integration tests.

#![allow(dead_code)]

use assert_cmd::Command;
use expect_test::Expect;
use std::io::Write;
use std::process::ExitStatus;

/// Create a new command for the melbi binary.
pub fn melbi() -> Command {
    Command::new(env!("CARGO_BIN_EXE_melbi"))
}

/// Create a temporary file with the given content.
pub fn temp_file(content: &str) -> tempfile::NamedTempFile {
    let mut file = tempfile::Builder::new()
        .suffix(".melbi")
        .tempfile()
        .unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file
}

/// Run a command and check that stdout matches the expected output.
/// Returns the exit status so callers can verify success/failure if needed.
pub fn check_stdout(args: &[&str], stdin: Option<&str>, expected: Expect) -> ExitStatus {
    let mut cmd = melbi();
    cmd.args(args);
    if let Some(input) = stdin {
        cmd.write_stdin(input);
    }
    let output = cmd.output().expect("failed to execute command");
    let stdout = trim_trailing_whitespace(&String::from_utf8_lossy(&output.stdout));
    expected.assert_eq(&stdout);
    output.status
}

/// Run a command and check that stderr matches the expected output.
/// Returns the exit status so callers can verify success/failure if needed.
pub fn check_stderr(args: &[&str], stdin: Option<&str>, expected: Expect) -> ExitStatus {
    let mut cmd = melbi();
    cmd.args(args);
    if let Some(input) = stdin {
        cmd.write_stdin(input);
    }
    let output = cmd.output().expect("failed to execute command");
    let stderr = trim_trailing_whitespace(&String::from_utf8_lossy(&output.stderr));
    expected.assert_eq(&stderr);
    output.status
}

/// Run a command and check both stdout and stderr.
/// Returns the exit status so callers can verify success/failure if needed.
pub fn check_output(
    args: &[&str],
    stdin: Option<&str>,
    expected_stdout: Expect,
    expected_stderr: Expect,
) -> ExitStatus {
    let mut cmd = melbi();
    cmd.args(args);
    if let Some(input) = stdin {
        cmd.write_stdin(input);
    }
    let output = cmd.output().expect("failed to execute command");
    let stdout = trim_trailing_whitespace(&String::from_utf8_lossy(&output.stdout));
    let stderr = trim_trailing_whitespace(&String::from_utf8_lossy(&output.stderr));
    expected_stdout.assert_eq(&stdout);
    expected_stderr.assert_eq(&stderr);
    output.status
}

fn trim_trailing_whitespace(s: &str) -> String {
    let mut result = String::with_capacity(s.len());

    for line in s.lines() {
        result.push_str(line.trim_end());
        result.push('\n');
    }

    // If the original input didn't end with a newline,
    // we have one extra '\n' at the end of 'result'.
    if !s.ends_with('\n') {
        result.pop();
    }

    result.to_string()
}
