//! File input utilities.

use std::io::Read;

/// Read input from a file path or stdin if path is "-".
///
/// Returns the content and a display name for error messages.
pub fn read_input(path: &str) -> Result<(String, String), String> {
    if path == "-" {
        let mut content = String::new();
        std::io::stdin()
            .read_to_string(&mut content)
            .map_err(|e| format!("<stdin>: {}", e))?;
        Ok((content, "<stdin>".to_string()))
    } else {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("{}: {}", path, e))?;
        Ok((content, path.to_string()))
    }
}

/// Check if the path represents stdin.
pub fn is_stdin(path: &str) -> bool {
    path == "-"
}

/// Strip a shebang line from the input, if present.
///
/// Returns `(Some(shebang_line_with_newline), rest)` if a shebang is found,
/// or `(None, input)` if no shebang is present.
///
/// A shebang must start with `#!/` (e.g., `#!/usr/bin/env melbi run`).
pub fn strip_shebang(input: &str) -> (Option<&str>, &str) {
    if input.starts_with("#!/") {
        match input.find('\n') {
            Some(pos) => (Some(&input[..=pos]), &input[pos + 1..]),
            None => (Some(input), ""),
        }
    } else {
        (None, input)
    }
}
