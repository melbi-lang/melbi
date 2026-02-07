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
