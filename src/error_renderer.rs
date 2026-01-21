//! Beautiful error rendering using ariadne
//!
//! This module provides utilities for rendering Melbi errors with
//! rich formatting, source code snippets, and helpful annotations.

use crate::{Diagnostic, Error, Severity};
use ariadne::{ColorGenerator, Label, Report, ReportKind, Source};
use std::io::Write;

/// Configuration for error rendering.
#[derive(Debug, Clone)]
pub struct RenderConfig<'a> {
    /// Whether to use ANSI color codes in output.
    pub color: bool,
    /// The filename to display in error messages.
    /// Defaults to "<unknown>" if not provided.
    pub filename: Option<&'a str>,
}

impl Default for RenderConfig<'_> {
    fn default() -> Self {
        RenderConfig::default()
    }
}

impl RenderConfig<'_> {
    const fn default() -> Self {
        Self {
            color: true,
            filename: None,
        }
    }
}

/// Render an error with beautiful formatting to stderr using default config.
///
/// # Example
/// ```no_run
/// use melbi::{Engine, EngineOptions, render_error};
/// use melbi_core::values::binder::Binder;
/// use bumpalo::Bump;
///
/// let arena = Bump::new();
/// let engine = Engine::new(EngineOptions::default(), &arena, |_, _, env| env);
///
/// let source = "1 + true";
/// match engine.compile(Default::default(), source, &[]) {
///     Err(e) => render_error(&e),
///     Ok(_) => {}
/// }
/// ```
pub fn render_error(error: &Error) {
    render_error_to(error, &mut std::io::stderr(), &RenderConfig::default()).ok();
}

/// Render an error to a writer with the given configuration.
///
/// This is the main rendering function. Use this when you need control over
/// the output destination or rendering options.
///
/// # Example
/// ```no_run
/// use melbi::{Engine, EngineOptions, render_error_to, RenderConfig};
/// use melbi_core::values::binder::Binder;
/// use bumpalo::Bump;
///
/// let arena = Bump::new();
/// let engine = Engine::new(EngineOptions::default(), &arena, |_, _, env| env);
///
/// let source = "1 + true";
/// match engine.compile(Default::default(), source, &[]) {
///     Err(e) => {
///         // Render without colors to a string
///         let mut buf = Vec::new();
///         let config = RenderConfig { color: false, ..Default::default() };
///         render_error_to(&e, &mut buf, &config).ok();
///         let output = String::from_utf8_lossy(&buf);
///         println!("{}", output);
///     }
///     Ok(_) => {}
/// }
/// ```
pub fn render_error_to(
    error: &Error,
    writer: &mut dyn Write,
    config: &RenderConfig,
) -> std::io::Result<()> {
    let filename = config.filename.unwrap_or("<unknown>");

    match error {
        Error::Compilation {
            diagnostics,
            source,
        } => render_diagnostics(source, diagnostics, writer, config, filename),
        Error::Runtime { diagnostic, source } => {
            render_diagnostics(source, &[diagnostic.clone()], writer, config, filename)
        }
        Error::ResourceExceeded(msg) => {
            writeln!(writer, "Resource limit exceeded: {}", msg)
        }
        Error::Api(msg) => {
            writeln!(writer, "API error: {}", msg)
        }
    }
}

fn render_diagnostics(
    source: &str,
    diagnostics: &[Diagnostic],
    writer: &mut dyn Write,
    config: &RenderConfig,
    filename: &str,
) -> std::io::Result<()> {
    for diag in diagnostics {
        let mut colors = ColorGenerator::new();
        colors.next(); // Skip the first color.

        let kind = match diag.severity {
            Severity::Error => ReportKind::Error,
            Severity::Warning => ReportKind::Warning,
            Severity::Info => ReportKind::Advice,
        };

        let ariadne_config = ariadne::Config::default().with_color(config.color);

        let mut report = Report::build(kind, (filename, diag.span.0.clone()))
            .with_message(&diag.message)
            .with_config(ariadne_config);

        // Add error code if present
        if let Some(code) = &diag.code {
            report = report.with_code(code);
        }

        // Primary label with the main error span
        let color = colors.next();
        report = report.with_label(
            Label::new((filename, diag.span.0.clone()))
                .with_message(&diag.message)
                .with_color(color),
        );

        // Related info as secondary labels (shows context breadcrumbs!)
        for related in &diag.related {
            let color = colors.next();
            report = report.with_label(
                Label::new((filename, related.span.0.clone()))
                    .with_message(&related.message)
                    .with_color(color),
            );
        }

        // Help text as notes
        for help_msg in &diag.help {
            report = report.with_help(help_msg);
        }

        // Render to the writer (need to reborrow to avoid moving)
        report
            .finish()
            .write((filename, Source::from(source)), &mut *writer)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Engine, EngineOptions};
    use bumpalo::Bump;

    const TEST_CONFIG: RenderConfig = RenderConfig {
        color: false,
        ..RenderConfig::default()
    };

    #[test]
    fn test_render_parse_error() {
        let arena = Bump::new();
        let engine = Engine::new(EngineOptions::default(), &arena, |_, _, env| env);

        let source = "1 + + 2"; // Invalid syntax
        let result = engine.compile(Default::default(), source, &[]);

        assert!(result.is_err());
        if let Err(e) = result {
            let mut buf = Vec::new();
            render_error_to(&e, &mut buf, &TEST_CONFIG).unwrap();
            let output = String::from_utf8_lossy(&buf);

            // Should contain error indicator
            assert!(output.contains("Error") || output.contains("error"));
            // Should show the source
            assert!(output.contains("1 + + 2"));
        }
    }

    #[test]
    fn test_render_type_error() {
        let arena = Bump::new();
        let engine = Engine::new(EngineOptions::default(), &arena, |_, _, env| env);

        let source = "1 + \"hello\""; // Type mismatch
        let result = engine.compile(Default::default(), source, &[]);

        assert!(result.is_err());
        if let Err(e) = result {
            let mut buf = Vec::new();
            render_error_to(&e, &mut buf, &TEST_CONFIG).unwrap();
            let output = String::from_utf8_lossy(&buf);

            // Should indicate type error
            assert!(output.contains("Type") || output.contains("type"));
        }
    }

    #[test]
    fn test_render_to_string_captures_output() {
        let arena = Bump::new();
        let engine = Engine::new(EngineOptions::default(), &arena, |_, _, env| env);

        let source = "bad syntax {";
        let result = engine.compile(Default::default(), source, &[]);

        assert!(result.is_err());
        if let Err(e) = result {
            let mut buf = Vec::new();
            render_error_to(&e, &mut buf, &TEST_CONFIG).unwrap();
            let output = String::from_utf8_lossy(&buf);

            // Output should not be empty
            assert!(!output.is_empty());
            // Should be multi-line (ariadne adds formatting)
            assert!(output.lines().count() > 1);
        }
    }
}
