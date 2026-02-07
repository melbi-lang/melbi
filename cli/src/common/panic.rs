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

    let context = collect_crash_context(info);
    let url = build_crash_report_url(&context);

    let style = if std::io::stderr().is_terminal() {
        Color::White.bold()
    } else {
        Color::White.normal()
    };

    eprintln!();
    eprintln!(
        "{}",
        style.paint("Melbi has crashed! We'd appreciate a bug report.")
    );
    eprintln!();
    eprintln!("We would include the following information:");
    eprintln!(
        "  melbi {} — {} — {}",
        context.version, context.os, context.arch
    );
    if let Some(ref cmd) = context.command_line {
        if cmd.contains('\n') {
            let cmd = format!("\n{}", cmd).replace('\n', "\n     | ");
            eprintln!("  {}:\n     | melbi {cmd}", style.paint("Command"));
        } else {
            eprintln!("  {}: melbi {}", style.paint("Command"), cmd);
        }
    }
    if let Some(ref expr) = context.expression {
        eprintln!("  {}: {expr}", style.paint("Expression"));
    }
    eprintln!("  {}: {}", style.paint("Location"), context.location);
    if context.message.contains('\n') {
        let message = format!("\n{}", context.message).replace('\n', "\n     | ");
        eprintln!("  {}: {}", style.paint("Message"), message);
    } else {
        eprintln!("  {}: {}", style.paint("Message"), context.message);
    }
    eprintln!();

    if std::io::stderr().is_terminal() {
        let link = Color::LightBlue.paint("click here").hyperlink(&url);
        eprintln!("If this data is free of PII and secrets, {link} to report.");
    } else {
        // Not a terminal - print full URL without escape sequences
        eprintln!("If this data is free of PII and secrets, open this URL to report:");
        eprintln!("  {url}");
    }
    eprintln!();
    eprintln!("You can still review and edit the report before submitting.");
}

const GITHUB_ISSUES_URL: &str = "https://github.com/melbi-lang/melbi/issues/new";

/// Crash context data collected for bug reports.
struct CrashContext {
    version: &'static str,
    os: &'static str,
    arch: &'static str,
    message: String,
    location: String,
    command_line: Option<String>,
    expression: Option<String>,
}

fn collect_crash_context(info: &PanicHookInfo<'_>) -> CrashContext {
    let message = info
        .payload()
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| info.payload().downcast_ref::<String>().map(|s| s.as_str()))
        .unwrap_or("unknown")
        .to_string();

    let location = info
        .location()
        .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
        .unwrap_or_else(|| "unknown".to_string());

    // Collect command line args (skip program name)
    let command_line = {
        let args: Vec<String> = std::env::args().skip(1).collect();
        if args.is_empty() {
            None
        } else {
            Some(
                shlex::try_join(args.iter().map(|s| s.as_str())).unwrap_or_else(|_| args.join(" ")),
            )
        }
    };

    // Get current expression if in REPL
    let expression = get_current_expression();

    CrashContext {
        version: env!("CARGO_PKG_VERSION"),
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        message,
        location,
        command_line,
        expression,
    }
}

fn build_crash_report_url(context: &CrashContext) -> String {
    let command_section = context
        .command_line
        .as_ref()
        .map(|cmd| format!("### Command\n```\nmelbi {cmd}\n```\n\n"))
        .unwrap_or_default();

    let expression_section = context
        .expression
        .as_ref()
        .map(|expr| format!("### Expression\n```melbi\n{expr}\n```\n\n"))
        .unwrap_or_default();

    let title = format!("Crash: {}", truncate(&context.message, 50));
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
         ### Additional Context\n<!-- Any other relevant information -->",
        version = context.version,
        os = context.os,
        arch = context.arch,
        location = context.location,
        message = context.message,
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
