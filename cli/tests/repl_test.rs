//! Integration tests for the `repl` command.
//!
//! These tests use rexpect to interact with the REPL.
//!
//! **NOTE**: These tests are currently ignored because reedline's terminal handling
//! (cursor position queries, etc.) doesn't work well with rexpect's pseudo-terminal.
//! The tests are kept here as documentation of intended REPL behavior and may be
//! enabled if we find a way to make reedline work with rexpect, or if we switch
//! to a different testing approach.

use rexpect::spawn;
use std::time::Duration;

const TIMEOUT: u64 = 5000; // 5 seconds

fn spawn_repl() -> Result<rexpect::session::PtySession, rexpect::error::Error> {
    let bin = env!("CARGO_BIN_EXE_melbi-cli");
    spawn(&format!("{} repl", bin), Some(TIMEOUT))
}

#[test]
#[ignore = "rexpect tests may not work in all environments"]
fn repl_simple_expression() {
    let mut repl = spawn_repl().expect("Failed to spawn REPL");

    // Wait for prompt
    repl.exp_string("Melbi REPL").unwrap();

    // Send expression
    repl.send_line("1 + 2").unwrap();

    // Expect result
    repl.exp_string("3").unwrap();

    // Exit
    repl.send_control('d').unwrap();
    repl.exp_string("Goodbye").unwrap();
}

#[test]
#[ignore = "rexpect tests may not work in all environments"]
fn repl_multiple_expressions() {
    let mut repl = spawn_repl().expect("Failed to spawn REPL");

    repl.exp_string("Melbi REPL").unwrap();

    repl.send_line("10 * 5").unwrap();
    repl.exp_string("50").unwrap();

    repl.send_line("true and false").unwrap();
    repl.exp_string("false").unwrap();

    repl.send_control('d').unwrap();
}

#[test]
#[ignore = "rexpect tests may not work in all environments"]
fn repl_where_binding() {
    let mut repl = spawn_repl().expect("Failed to spawn REPL");

    repl.exp_string("Melbi REPL").unwrap();

    repl.send_line("x + y where { x = 1, y = 2 }").unwrap();
    repl.exp_string("3").unwrap();

    repl.send_control('d').unwrap();
}

#[test]
#[ignore = "rexpect tests may not work in all environments"]
fn repl_ctrl_c_aborts_entry() {
    let mut repl = spawn_repl().expect("Failed to spawn REPL");

    repl.exp_string("Melbi REPL").unwrap();

    // Start typing something
    repl.send("1 + ").unwrap();
    std::thread::sleep(Duration::from_millis(100));

    // Abort with Ctrl+C
    repl.send_control('c').unwrap();

    // Should still be able to enter new expressions
    repl.send_line("42").unwrap();
    repl.exp_string("42").unwrap();

    repl.send_control('d').unwrap();
}
