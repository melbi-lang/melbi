//! Shared test utilities for CLI integration tests.

#![allow(dead_code)]

use assert_cmd::Command;
use expect_test::Expect;
use std::io::Write;

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
pub fn check_stdout(args: &[&str], stdin: Option<&str>, expected: Expect) {
    let mut cmd = melbi();
    cmd.args(args);
    if let Some(input) = stdin {
        cmd.write_stdin(input);
    }
    let output = cmd.output().expect("failed to execute command");
    let stdout = String::from_utf8_lossy(&output.stdout);
    expected.assert_eq(&stdout);
}

/// Run a command and check that stderr matches the expected output.
pub fn check_stderr(args: &[&str], stdin: Option<&str>, expected: Expect) {
    let mut cmd = melbi();
    cmd.args(args);
    if let Some(input) = stdin {
        cmd.write_stdin(input);
    }
    let output = cmd.output().expect("failed to execute command");
    let stderr = String::from_utf8_lossy(&output.stderr);
    expected.assert_eq(&stderr);
}

/// Run a command and check both stdout and stderr.
pub fn check_output(
    args: &[&str],
    stdin: Option<&str>,
    expected_stdout: Expect,
    expected_stderr: Expect,
) {
    let mut cmd = melbi();
    cmd.args(args);
    if let Some(input) = stdin {
        cmd.write_stdin(input);
    }
    let output = cmd.output().expect("failed to execute command");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    expected_stdout.assert_eq(&stdout);
    expected_stderr.assert_eq(&stderr);
}
