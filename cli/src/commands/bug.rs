//! Bug reporting command.
//!
//! Opens a GitHub issue with pre-filled system information.

use crate::cli::BugArgs;
use crate::common::panic::{decode_panic_info, PanicInfo};

const GITHUB_ISSUES_URL: &str = "https://github.com/melbi-lang/melbi/issues/new";

/// Run the bug command.
pub fn run(args: BugArgs) {
    let panic_info = args
        .panic_info
        .as_ref()
        .and_then(|s| decode_panic_info(s));

    let url = build_issue_url(panic_info.as_ref());

    println!("Opening bug report in your browser...");
    println!("\n{url}\n");

    if let Err(e) = open::that(&url) {
        eprintln!("Could not open browser automatically: {e}");
        eprintln!("Please copy the URL above and open it manually.");
    }
}

fn build_issue_url(panic_info: Option<&PanicInfo>) -> String {
    let version = env!("CARGO_PKG_VERSION");
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    let (title, body) = if let Some(info) = panic_info {
        let title = format!("Crash: {}", truncate(&info.message, 50));

        // Build optional sections
        let command_section = info
            .format_command_line()
            .map(|cmd| format!("### Command\n```\n{cmd}\n```\n\n"))
            .unwrap_or_default();

        let expression_section = info
            .expression
            .as_ref()
            .map(|expr| format!("### Expression\n```melbi\n{expr}\n```\n\n"))
            .unwrap_or_default();

        let body = format!(
            "## Crash Report\n\n\
             **Version:** {}\n\
             **OS:** {}\n\
             **Arch:** {}\n\
             **Location:** {}\n\n\
             {command_section}\
             {expression_section}\
             ### Panic Message\n```\n{}\n```\n\n\
             ### Steps to Reproduce\n<!-- What were you doing when this happened? -->\n\n\
             ### Additional Context\n<!-- Any other relevant information -->",
            info.version, info.os, info.arch, info.location, info.message
        );
        (title, body)
    } else {
        let title = "Bug: ".to_string();
        let body = format!(
            "## Bug Report\n\n\
             **Version:** {version}\n\
             **OS:** {os}\n\
             **Arch:** {arch}\n\n\
             ### Description\n<!-- Describe the bug -->\n\n\
             ### Steps to Reproduce\n<!-- How can we reproduce this? -->\n\n\
             ### Expected Behavior\n<!-- What should happen? -->\n\n\
             ### Actual Behavior\n<!-- What actually happens? -->"
        );
        (title, body)
    };

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

    /// Helper to parse URL and extract decoded title and body.
    fn parse_issue_url(url: &str) -> (String, String) {
        let url = url::Url::parse(url).expect("valid URL");
        let mut title = String::new();
        let mut body = String::new();

        for (key, value) in url.query_pairs() {
            match key.as_ref() {
                "title" => title = value.into_owned(),
                "body" => body = value.into_owned(),
                _ => {}
            }
        }

        (title, body)
    }

    /// End-to-end test: PanicInfo → JSON → decode → URL → verify decoded content
    #[test]
    fn end_to_end_crash_report_simple() {
        // 1. Create panic info (as panic handler would)
        let original = PanicInfo {
            version: "0.1.0".to_string(),
            os: "macos".to_string(),
            arch: "aarch64".to_string(),
            location: "src/eval.rs:42:5".to_string(),
            command_line: vec!["eval".to_string(), "1 + 2".to_string()],
            expression: None,
            message: "assertion failed".to_string(),
        };

        // 2. Encode to JSON (simulating panic handler output)
        let json = serde_json::to_string(&original).unwrap();

        // 3. Decode from JSON (simulating --panic-info parsing)
        let decoded = decode_panic_info(&json).expect("should decode");
        assert_eq!(decoded, original);

        // 4. Build URL
        let url = build_issue_url(Some(&decoded));

        // 5. Parse and verify the decoded content
        let (title, body) = parse_issue_url(&url);

        assert_eq!(title, "Crash: assertion failed");
        assert!(body.contains("## Crash Report"));
        assert!(body.contains("**Version:** 0.1.0"));
        assert!(body.contains("**OS:** macos"));
        assert!(body.contains("**Arch:** aarch64"));
        assert!(body.contains("**Location:** src/eval.rs:42:5"));
        assert!(body.contains("### Command\n```\nmelbi eval '1 + 2'\n```"));
        assert!(body.contains("### Panic Message\n```\nassertion failed\n```"));
        // Should NOT have expression section
        assert!(!body.contains("### Expression"));
    }

    #[test]
    fn end_to_end_crash_report_with_multiline_expression() {
        let original = PanicInfo {
            version: "0.2.0".to_string(),
            os: "linux".to_string(),
            arch: "x86_64".to_string(),
            location: "src/vm.rs:100:1".to_string(),
            command_line: vec!["repl".to_string()],
            expression: Some("1 + 2\n* 3\n  where { x = 4 }".to_string()),
            message: "stack overflow".to_string(),
        };

        let json = serde_json::to_string(&original).unwrap();
        let decoded = decode_panic_info(&json).expect("should decode");
        assert_eq!(decoded, original);

        let url = build_issue_url(Some(&decoded));
        let (title, body) = parse_issue_url(&url);

        assert_eq!(title, "Crash: stack overflow");
        assert!(body.contains("**Version:** 0.2.0"));
        assert!(body.contains("**OS:** linux"));
        assert!(body.contains("### Command\n```\nmelbi repl\n```"));
        // Expression with actual newlines preserved
        assert!(body.contains("### Expression\n```melbi\n1 + 2\n* 3\n  where { x = 4 }\n```"));
        assert!(body.contains("### Panic Message\n```\nstack overflow\n```"));
    }

    #[test]
    fn end_to_end_crash_report_with_special_characters() {
        let original = PanicInfo {
            version: "0.1.0".to_string(),
            os: "windows".to_string(),
            arch: "x86_64".to_string(),
            location: "C:\\Users\\test\\file.rs:1:1".to_string(),
            command_line: vec!["eval".to_string(), "\"hello\" + 'world'".to_string()],
            expression: Some("f\"Value: {x}\" where { x = 42 }".to_string()),
            message: "panic at 'index out of bounds'".to_string(),
        };

        let json = serde_json::to_string(&original).unwrap();
        let decoded = decode_panic_info(&json).expect("should decode");
        assert_eq!(decoded, original);

        let url = build_issue_url(Some(&decoded));
        let (title, body) = parse_issue_url(&url);

        assert_eq!(title, "Crash: panic at 'index out of bounds'");
        assert!(body.contains("**Location:** C:\\Users\\test\\file.rs:1:1"));
        // Command should be shell-escaped - shlex will quote the argument
        assert!(body.contains("melbi eval"));
        // The shell-escaped version will have escaped quotes - verify round-trip works
        let cmd = decoded.format_command_line().unwrap();
        let parsed_args = shlex::split(&cmd).unwrap();
        assert_eq!(parsed_args, vec!["melbi", "eval", "\"hello\" + 'world'"]);
        // Expression with quotes preserved
        assert!(body.contains("f\"Value: {x}\" where { x = 42 }"));
    }

    #[test]
    fn end_to_end_crash_report_empty_command_line() {
        let original = PanicInfo {
            version: "0.1.0".to_string(),
            os: "macos".to_string(),
            arch: "aarch64".to_string(),
            location: "src/main.rs:10:1".to_string(),
            command_line: vec![],
            expression: None,
            message: "early panic".to_string(),
        };

        let json = serde_json::to_string(&original).unwrap();
        let decoded = decode_panic_info(&json).expect("should decode");

        let url = build_issue_url(Some(&decoded));
        let (title, body) = parse_issue_url(&url);

        assert_eq!(title, "Crash: early panic");
        // Should NOT have command section when empty
        assert!(!body.contains("### Command"));
        assert!(!body.contains("### Expression"));
    }

    #[test]
    fn end_to_end_bug_report_no_panic_info() {
        let url = build_issue_url(None);
        let (title, body) = parse_issue_url(&url);

        assert_eq!(title, "Bug: ");
        assert!(body.contains("## Bug Report"));
        assert!(body.contains("**Version:**"));
        assert!(body.contains("**OS:**"));
        assert!(body.contains("**Arch:**"));
        assert!(body.contains("### Description"));
        assert!(body.contains("### Steps to Reproduce"));
        assert!(body.contains("### Expected Behavior"));
        assert!(body.contains("### Actual Behavior"));
        // Should NOT have crash-specific sections
        assert!(!body.contains("### Command"));
        assert!(!body.contains("### Expression"));
        assert!(!body.contains("### Panic Message"));
        assert!(!body.contains("**Location:**"));
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
        // Multi-byte characters should not be split
        assert_eq!(truncate("héllo", 3), "hél");
        assert_eq!(truncate("日本語", 2), "日本");
    }

    #[test]
    fn title_truncates_long_messages() {
        let original = PanicInfo {
            version: "0.1.0".to_string(),
            os: "macos".to_string(),
            arch: "aarch64".to_string(),
            location: "file.rs:1:1".to_string(),
            command_line: vec![],
            expression: None,
            message: "a]".repeat(100), // Very long message
        };

        let url = build_issue_url(Some(&original));
        let (title, _body) = parse_issue_url(&url);

        // Title should be truncated to 50 chars + "Crash: " prefix
        assert!(title.starts_with("Crash: "));
        assert!(title.len() <= "Crash: ".len() + 50);
    }
}
