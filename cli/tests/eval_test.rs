//! Integration tests for the `eval` command.

use assert_cmd::Command;
use predicates::prelude::*;

fn melbi() -> Command {
    Command::new(env!("CARGO_BIN_EXE_melbi"))
}

#[test]
fn eval_simple_expression() {
    melbi()
        .args(["eval", "1 + 2"])
        .assert()
        .success()
        .stdout("3\n");
}

#[test]
fn eval_arithmetic() {
    melbi()
        .args(["eval", "10 * 5 - 3"])
        .assert()
        .success()
        .stdout("47\n");
}

#[test]
fn eval_string() {
    melbi()
        .args(["eval", r#""hello""#])
        .assert()
        .success()
        .stdout("\"hello\"\n");
}

#[test]
fn eval_boolean() {
    melbi()
        .args(["eval", "true and false"])
        .assert()
        .success()
        .stdout("false\n");
}

#[test]
fn eval_array() {
    melbi()
        .args(["eval", "[1, 2, 3]"])
        .assert()
        .success()
        .stdout("[1, 2, 3]\n");
}

#[test]
fn eval_record() {
    melbi()
        .args(["eval", "{ x = 1, y = 2 }"])
        .assert()
        .success()
        .stdout(predicate::str::contains("x = 1"))
        .stdout(predicate::str::contains("y = 2"));
}

#[test]
fn eval_where_binding() {
    melbi()
        .args(["eval", "x + y where { x = 10, y = 20 }"])
        .assert()
        .success()
        .stdout("30\n");
}

#[test]
fn eval_if_expression() {
    melbi()
        .args(["eval", "if true then 42 else 0"])
        .assert()
        .success()
        .stdout("42\n");
}

#[test]
fn eval_lambda() {
    melbi()
        .args(["eval", "((x) => x * 2)(21)"])
        .assert()
        .success()
        .stdout("42\n");
}

#[test]
fn eval_stdlib_math() {
    melbi()
        .args(["eval", "Math.Floor(3.7)"])
        .assert()
        .success()
        .stdout("3\n");
}

#[test]
fn eval_stdlib_string() {
    melbi()
        .args(["eval", "String.Len(\"hello\")"])
        .assert()
        .success()
        .stdout("5\n");
}

#[test]
fn eval_type_error_shows_error() {
    melbi()
        .args(["eval", "1 + true"])
        .assert()
        .success() // CLI exits 0 but prints error
        .stderr(predicate::str::contains("Type mismatch"));
}

#[test]
fn eval_parse_error_shows_error() {
    melbi()
        .args(["eval", "1 + +"])
        .assert()
        .success() // CLI exits 0 but prints error
        .stderr(predicate::str::contains("Error"));
}

#[test]
fn eval_runtime_error_shows_error() {
    melbi()
        .args(["eval", "1 / 0"])
        .assert()
        .success() // CLI exits 0 but prints error
        .stderr(predicate::str::contains("division by zero").or(predicate::str::contains("Division")));
}

#[test]
fn eval_with_runtime_evaluator() {
    melbi()
        .args(["eval", "--runtime", "evaluator", "1 + 2"])
        .assert()
        .success()
        .stdout("3\n");
}

#[test]
fn eval_with_runtime_vm() {
    melbi()
        .args(["eval", "--runtime", "vm", "1 + 2"])
        .assert()
        .success()
        .stdout("3\n");
}

#[test]
fn eval_no_color_flag() {
    melbi()
        .args(["--no-color", "eval", "1 + true"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Type mismatch"))
        // No ANSI escape codes when --no-color is used
        .stderr(predicate::str::contains("\x1b[").not());
}
