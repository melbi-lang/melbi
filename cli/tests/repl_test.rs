//! Integration tests for the `repl` command using termlens.

use std::time::Duration;

use termlens::{Key, Terminal};

fn spawn_repl() -> Result<Terminal, termlens::Error> {
    let bin = env!("CARGO_BIN_EXE_melbi");
    Terminal::builder()
        .size(80, 24)
        .env("TERM", "xterm-256color")
        .timeout(Duration::from_secs(5))
        .args(["repl"])
        .spawn(bin)
}

#[test]
fn validator_completeness() {
    use melbi_cli::commands::repl::MelbiValidator;

    // Complete expressions
    assert!(!MelbiValidator::is_incomplete("1 + 2"));
    assert!(!MelbiValidator::is_incomplete("if true then 1 else 2"));
    assert!(!MelbiValidator::is_incomplete(
        "1 + 1 where { a = 1, b = 2 }"
    ));
    assert!(!MelbiValidator::is_incomplete(""));

    // Incomplete expressions requiring more input
    assert!(MelbiValidator::is_incomplete("1 + 1 where {"));
    assert!(MelbiValidator::is_incomplete("1 + 1 where {\n    a = 1,"));
    assert!(MelbiValidator::is_incomplete("[1, 2,"));
    assert!(MelbiValidator::is_incomplete("(\"hello"));
}

#[test]
fn repl_multiline_where_expression() {
    let mut repl = spawn_repl().expect("Failed to spawn REPL");
    repl.resize(52, 10).expect("Failed to resize REPL");

    repl.wait_until(|screen| screen.contains("Melbi REPL"))
        .unwrap();

    repl.send_str("1 + 1 where {").unwrap();
    repl.send(Key::Enter).unwrap();
    repl.send_str("a = 1,").unwrap();
    repl.send(Key::Enter).unwrap();
    repl.send_str("b = 2,").unwrap();
    repl.send(Key::Enter).unwrap();
    repl.send_str("}").unwrap();
    repl.send(Key::Enter).unwrap();

    repl.wait_until(|screen| screen.contains("2")).unwrap();

    repl.send(Key::Ctrl('d')).unwrap();
    repl.wait_until(|screen| screen.contains("Goodbye"))
        .unwrap();

    insta::assert_snapshot!(repl.screen().with_styles());
}

#[test]
fn repl_simple_expression() {
    let mut repl = spawn_repl().expect("Failed to spawn REPL");
    repl.resize(52, 7).expect("Failed to resize REPL");

    repl.wait_until(|screen| screen.contains("Melbi REPL"))
        .unwrap();

    repl.send_str("1 + 2").unwrap();
    repl.send(Key::Enter).unwrap();

    repl.wait_until(|screen| screen.contains("3")).unwrap();

    repl.send(Key::Ctrl('d')).unwrap();
    repl.wait_until(|screen| screen.contains("Goodbye"))
        .unwrap();

    insta::assert_snapshot!(repl.screen().with_styles());
}

#[test]
fn repl_multiple_expressions() {
    let mut repl = spawn_repl().expect("Failed to spawn REPL");

    repl.wait_until(|screen| screen.contains("Melbi REPL"))
        .unwrap();

    repl.send_str("10 * 5").unwrap();
    repl.send(Key::Enter).unwrap();
    repl.wait_until(|screen| screen.contains("50")).unwrap();

    repl.send_str("true and false").unwrap();
    repl.send(Key::Enter).unwrap();
    repl.wait_until(|screen| screen.contains("false")).unwrap();

    repl.send(Key::Ctrl('d')).unwrap();
    repl.wait_until(|screen| screen.contains("Goodbye"))
        .unwrap();
}

#[test]
fn repl_where_binding() {
    let mut repl = spawn_repl().expect("Failed to spawn REPL");

    repl.wait_until(|screen| screen.contains("Melbi REPL"))
        .unwrap();

    repl.send_str("x + y where { x = 1, y = 2 }").unwrap();
    repl.send(Key::Enter).unwrap();
    repl.wait_until(|screen| screen.contains("3")).unwrap();

    repl.send(Key::Ctrl('d')).unwrap();
}

#[test]
fn repl_ctrl_c_aborts_entry() {
    let mut repl = spawn_repl().expect("Failed to spawn REPL");

    repl.wait_until(|screen| screen.contains("Melbi REPL"))
        .unwrap();

    repl.send_str("1 + ").unwrap();
    repl.wait_until(|screen| screen.contains("1 +")).unwrap();

    repl.send(Key::Ctrl('c')).unwrap();
    repl.wait_until(|screen| screen.text().contains("  > 1 +\n  >"))
        .unwrap();

    repl.send_str("42").unwrap();
    repl.send(Key::Enter).unwrap();
    repl.wait_until(|screen| screen.contains("42")).unwrap();

    repl.send(Key::Ctrl('d')).unwrap();
}

#[test]
fn repl_recovers_from_type_error() {
    let mut repl = spawn_repl().expect("Failed to spawn REPL");
    repl.wait_until(|screen| screen.contains("Melbi REPL"))
        .unwrap();

    repl.send_str("1 + true").unwrap();
    repl.send(Key::Enter).unwrap();
    repl.wait_until(|screen| screen.contains("Type")).unwrap();

    repl.send_str("1 + 2").unwrap();
    repl.send(Key::Enter).unwrap();
    repl.wait_until(|screen| screen.contains("3")).unwrap();

    repl.send(Key::Ctrl('d')).unwrap();
}
