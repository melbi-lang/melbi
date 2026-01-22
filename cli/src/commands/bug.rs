//! Bug reporting command.
//!
//! Opens a GitHub issue with pre-filled system information.

const GITHUB_ISSUES_URL: &str = "https://github.com/melbi-lang/melbi/issues/new";

/// Run the bug command.
pub(crate) fn run() {
    let url = build_bug_report_url();

    println!("Opening bug report in your browser...");
    println!("\n{url}\n");

    if let Err(e) = open::that(&url) {
        eprintln!("Could not open browser automatically: {e}");
        eprintln!("Please copy the URL above and open it manually.");
    }
}

fn build_bug_report_url() -> String {
    let version = env!("CARGO_PKG_VERSION");
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    let title = "Bug: ";
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

    format!(
        "{}?title={}&body={}",
        GITHUB_ISSUES_URL,
        urlencoding::encode(title),
        urlencoding::encode(&body)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn bug_report_url_contains_system_info() {
        let url = build_bug_report_url();
        let (title, body) = parse_issue_url(&url);

        assert_eq!(title, "Bug: ");
        assert!(body.contains("## Bug Report"));
        assert!(body.contains(&format!("**Version:** {}", env!("CARGO_PKG_VERSION"))));
        assert!(body.contains(&format!("**OS:** {}", std::env::consts::OS)));
        assert!(body.contains(&format!("**Arch:** {}", std::env::consts::ARCH)));
        assert!(body.contains("### Description"));
        assert!(body.contains("### Steps to Reproduce"));
        assert!(body.contains("### Expected Behavior"));
        assert!(body.contains("### Actual Behavior"));
    }
}
