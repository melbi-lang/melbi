//! Error handling utilities for the CLI.

use melbi::{Error, RenderConfig, render_error_to};

/// CLI-specific error type.
#[derive(Debug)]
pub enum CliError {
    /// A Melbi error that needs to be rendered.
    Melbi(Error),
    /// An error that has already been rendered to stderr.
    /// The caller should exit with failure but not re-render.
    Handled,
}

impl From<Error> for CliError {
    fn from(e: Error) -> Self {
        CliError::Melbi(e)
    }
}

/// Result type for CLI commands.
pub type CliResult<T> = Result<T, CliError>;

/// Render an error to stderr and exit with code 1.
/// If the error is `CliError::Handled`, just exits without rendering.
pub fn render_and_exit(error: CliError, no_color: bool) -> ! {
    if let CliError::Melbi(e) = error {
        let config = RenderConfig {
            color: !no_color,
            ..Default::default()
        };
        render_error_to(&e, &mut std::io::stderr(), &config).ok();
    }
    std::process::exit(1);
}
