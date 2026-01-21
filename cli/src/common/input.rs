//! File input utilities.

use std::path::Path;

/// Read the contents of a file as a string.
///
/// Returns an error message suitable for display if reading fails.
pub fn read_file(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("{}: {}", path.display(), e))
}
