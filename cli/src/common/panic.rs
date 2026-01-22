//! Panic handler for user-friendly crash reporting.

use nu_ansi_term::Color;
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
    use std::io::IsTerminal;

    eprintln!("\n💥 Melbi crashed unexpectedly 💥\n");
    eprintln!("{info}");

    let url = build_crash_report_url(info);

    eprintln!();
    if std::io::stderr().is_terminal() {
        // Use OSC 8 terminal hyperlink: \x1b]8;;URL\x07TEXT\x1b]8;;\x07
        let link_style = Color::LightBlue;
        let link_text = link_style.paint("[click here]");
        eprintln!("To report this bug \x1b]8;;{url}\x07{link_text}\x1b]8;;\x07.");
    } else {
        // Not a terminal - print full URL without escape sequences
        eprintln!("To report this bug, open:");
        eprintln!("  {url}");
    }
    eprintln!("\n(You'll be able to review the report before it's submitted)");
}

const GITHUB_ISSUES_URL: &str = "https://github.com/melbi-lang/melbi/issues/new";

fn build_crash_report_url(info: &PanicHookInfo<'_>) -> String {
    let version = env!("CARGO_PKG_VERSION");
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    let message = info
        .payload()
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| info.payload().downcast_ref::<String>().map(|s| s.as_str()))
        .unwrap_or("unknown");

    let location = info
        .location()
        .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
        .unwrap_or_else(|| "unknown".to_string());

    // Collect command line args (skip program name)
    let command_line: Vec<String> = std::env::args().skip(1).collect();
    let command_section = if command_line.is_empty() {
        String::new()
    } else {
        let cmd = shlex::try_join(command_line.iter().map(|s| s.as_str()))
            .unwrap_or_else(|_| command_line.join(" "));
        format!("### Command\n```\nmelbi {cmd}\n```\n\n")
    };

    // Get current expression if in REPL
    let expression_section = get_current_expression()
        .map(|expr| format!("### Expression\n```melbi\n{expr}\n```\n\n"))
        .unwrap_or_default();

    let title = format!("Crash: {}", truncate(message, 50));
    let body = format!(
        "## Crash Report\n\n\
         **Version:** {version}\n\
         **OS:** {os}\n\
         **Arch:** {arch}\n\
         **Location:** {location}\n\n\
         {command_section}\
         {expression_section}\
         ### Panic Message\n```\n{message}\n```\n\n\
         ### Steps to Reproduce\n<!-- What were you doing when this happened? -->\n\n\
         ### Additional Context\n<!-- Any other relevant information -->"
    );

    format!(
        "{}?title={}&body={}",
        GITHUB_ISSUES_URL,
        urlencoding::encode(&title),
        urlencoding::encode(&body)
    )
}

fn truncate(s: &str, max: usize) -> &str {
    match s.char_indices().nth(max) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thread_local_expression_set_get_clear() {
        assert!(get_current_expression().is_none());

        set_current_expression("test expr");
        assert_eq!(get_current_expression(), Some("test expr".to_string()));

        clear_current_expression();
        assert!(get_current_expression().is_none());
    }

    #[test]
    fn truncate_respects_char_boundaries() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 5), "hello");
        assert_eq!(truncate("", 5), "");
        assert_eq!(truncate("hello", 3), "hel");
    }

    #[test]
    fn truncate_handles_unicode() {
        assert_eq!(truncate("héllo", 3), "hél");
        assert_eq!(truncate("日本語", 2), "日本");
    }
}
