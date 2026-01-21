//! Panic handler for user-friendly crash reporting.

use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::panic::PanicHookInfo;

thread_local! {
    /// The current expression being evaluated (for REPL crash reports).
    static CURRENT_EXPRESSION: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Set the current expression being evaluated.
///
/// Call this before evaluating user input in the REPL.
pub fn set_current_expression(expr: &str) {
    CURRENT_EXPRESSION.with(|cell| {
        *cell.borrow_mut() = Some(expr.to_string());
    });
}

/// Clear the current expression.
///
/// Call this after evaluation completes (success or handled error).
pub fn clear_current_expression() {
    CURRENT_EXPRESSION.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

/// Get the current expression if set.
fn get_current_expression() -> Option<String> {
    CURRENT_EXPRESSION.with(|cell| cell.borrow().clone())
}

/// Install the custom panic handler.
///
/// This should be called early in main() before any other initialization.
pub fn install_handler() {
    std::panic::set_hook(Box::new(panic_hook));
}

fn panic_hook(info: &PanicHookInfo<'_>) {
    eprintln!("\n💥 Melbi crashed unexpectedly!\n");
    eprintln!("{info}");

    let json = encode_panic_info(info);

    // Shell-escape the JSON for safe copy-paste
    let escaped = shlex::try_quote(&json).unwrap_or_else(|_| json.clone().into());

    eprintln!("\nTo report this bug, run:");
    eprintln!("  melbi bug --panic-info {escaped}");
    eprintln!("\n(You'll be able to review the report before it's submitted)");
    eprintln!("\nOr open an issue directly:");
    eprintln!("  https://github.com/melbi-lang/melbi/issues/new");
}

fn encode_panic_info(info: &PanicHookInfo<'_>) -> String {
    let message = info
        .payload()
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| {
            info.payload()
                .downcast_ref::<String>()
                .map(|s| s.as_str())
        })
        .unwrap_or("unknown");

    let location = info
        .location()
        .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
        .unwrap_or_else(|| "unknown".to_string());

    // Collect command line args (skip program name)
    let command_line: Vec<String> = std::env::args().skip(1).collect();

    // Get current expression if in REPL
    let expression = get_current_expression();

    let panic_info = PanicInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        location,
        command_line,
        expression,
        message: message.to_string(),
    };

    serde_json::to_string(&panic_info).unwrap_or_default()
}

/// Decode panic info from JSON string.
pub fn decode_panic_info(json: &str) -> Option<PanicInfo> {
    serde_json::from_str(json).ok()
}

/// Panic information for bug reporting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PanicInfo {
    pub version: String,
    pub os: String,
    pub arch: String,
    pub location: String,
    /// Command line arguments (each arg as a separate element).
    pub command_line: Vec<String>,
    /// The expression being evaluated (for REPL crashes).
    pub expression: Option<String>,
    pub message: String,
}

impl PanicInfo {
    /// Format the command line for display, properly shell-escaped.
    pub fn format_command_line(&self) -> Option<String> {
        if self.command_line.is_empty() {
            return None;
        }
        Some(format!(
            "melbi {}",
            shlex::try_join(self.command_line.iter().map(|s| s.as_str())).ok()?
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_roundtrip_preserves_all_fields() {
        let original = PanicInfo {
            version: "0.1.0".to_string(),
            os: "macos".to_string(),
            arch: "aarch64".to_string(),
            location: "src/main.rs:42:5".to_string(),
            command_line: vec!["eval".to_string(), "1 + 2".to_string()],
            expression: None,
            message: "test panic message".to_string(),
        };

        let json = serde_json::to_string(&original).unwrap();
        let decoded = decode_panic_info(&json).expect("should decode");

        assert_eq!(decoded, original);
    }

    #[test]
    fn json_roundtrip_with_multiline_expression() {
        let original = PanicInfo {
            version: "0.1.0".to_string(),
            os: "linux".to_string(),
            arch: "x86_64".to_string(),
            location: "file.rs:1:1".to_string(),
            command_line: vec!["repl".to_string()],
            expression: Some("1 + 2\n* 3\n  where { x = 4 }".to_string()),
            message: "panic".to_string(),
        };

        let json = serde_json::to_string(&original).unwrap();
        let decoded = decode_panic_info(&json).expect("should decode");

        assert_eq!(decoded, original);
    }

    #[test]
    fn json_roundtrip_with_special_characters() {
        let original = PanicInfo {
            version: "0.1.0".to_string(),
            os: "windows".to_string(),
            arch: "x86_64".to_string(),
            location: "C:\\Users\\test\\file.rs:1:1".to_string(),
            command_line: vec!["eval".to_string(), "\"hello\" + 'world'".to_string()],
            expression: Some("f\"Value: {x}\" where { x = 42 }".to_string()),
            message: "assertion failed: x > 0\n\tat test::foo".to_string(),
        };

        let json = serde_json::to_string(&original).unwrap();
        let decoded = decode_panic_info(&json).expect("should decode");

        assert_eq!(decoded, original);
    }

    #[test]
    fn format_command_line_escapes_spaces() {
        let info = PanicInfo {
            version: "0.1.0".to_string(),
            os: "macos".to_string(),
            arch: "aarch64".to_string(),
            location: "file.rs:1:1".to_string(),
            command_line: vec!["eval".to_string(), "hello world".to_string()],
            expression: None,
            message: "panic".to_string(),
        };

        assert_eq!(
            info.format_command_line(),
            Some("melbi eval 'hello world'".to_string())
        );
    }

    #[test]
    fn format_command_line_escapes_quotes() {
        let info = PanicInfo {
            version: "0.1.0".to_string(),
            os: "macos".to_string(),
            arch: "aarch64".to_string(),
            location: "file.rs:1:1".to_string(),
            command_line: vec!["eval".to_string(), "it's a \"test\"".to_string()],
            expression: None,
            message: "panic".to_string(),
        };

        let formatted = info.format_command_line().unwrap();
        // Should properly escape both single and double quotes
        assert!(formatted.starts_with("melbi eval "));
        // Verify it can be parsed back
        let parts: Vec<_> = shlex::split(&formatted).unwrap();
        assert_eq!(parts, vec!["melbi", "eval", "it's a \"test\""]);
    }

    #[test]
    fn format_command_line_empty_returns_none() {
        let info = PanicInfo {
            version: "0.1.0".to_string(),
            os: "macos".to_string(),
            arch: "aarch64".to_string(),
            location: "file.rs:1:1".to_string(),
            command_line: vec![],
            expression: None,
            message: "panic".to_string(),
        };

        assert_eq!(info.format_command_line(), None);
    }

    #[test]
    fn thread_local_expression_set_get_clear() {
        // Should start empty
        assert!(get_current_expression().is_none());

        set_current_expression("test expr");
        assert_eq!(get_current_expression(), Some("test expr".to_string()));

        clear_current_expression();
        assert!(get_current_expression().is_none());
    }

    #[test]
    fn decode_invalid_json_returns_none() {
        assert!(decode_panic_info("not json").is_none());
        assert!(decode_panic_info("").is_none());
        assert!(decode_panic_info("{}").is_none()); // Missing required fields
        assert!(decode_panic_info("{\"version\": \"1.0\"}").is_none());
    }
}
