//! Error handling utilities for the CLI.

use melbi::{Error, RenderConfig, render_error_to};

/// Result type for CLI commands.
pub type CliResult<T> = Result<T, Error>;

/// Render an error to stderr and exit with code 1.
pub fn render_and_exit(error: Error, no_color: bool) -> ! {
    let config = RenderConfig {
        color: !no_color,
        ..Default::default()
    };
    render_error_to(&error, &mut std::io::stderr(), &config).ok();
    std::process::exit(1);
}
