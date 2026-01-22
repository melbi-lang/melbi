//! Beautiful error rendering using ariadne
//!
//! This module provides utilities for rendering Melbi errors with
//! rich formatting, source code snippets, and helpful annotations.

use crate::{Diagnostic, Error, Severity};
use ariadne::{ColorGenerator, Label, Report, ReportKind, Source};
use std::io::Write;

/// Character set for rendering error messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CharSet {
    /// Use Unicode characters for rich visual output.
    #[default]
    Unicode,
    /// Use ASCII-only characters for compatibility.
    Ascii,
}

/// Configuration for error rendering.
#[derive(Debug, Clone)]
pub struct RenderConfig<'a> {
    /// Whether to use ANSI color codes in output.
    pub color: bool,
    /// The filename to display in error messages.
    /// Defaults to "<unknown>" if not provided.
    pub filename: Option<&'a str>,
    /// The character set to use for rendering.
    /// Defaults to Unicode for rich visual output.
    pub charset: CharSet,
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
            charset: CharSet::Unicode,
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

        let ariadne_charset = match config.charset {
            CharSet::Unicode => ariadne::CharSet::Unicode,
            CharSet::Ascii => ariadne::CharSet::Ascii,
        };
        let ariadne_config = ariadne::Config::default()
            .with_color(config.color)
            .with_char_set(ariadne_charset);

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
    use expect_test::{expect, Expect};

    const UNICODE_CONFIG: RenderConfig = RenderConfig {
        color: false,
        filename: Some("test.melbi"),
        charset: CharSet::Unicode,
    };

    const ASCII_CONFIG: RenderConfig = RenderConfig {
        color: false,
        filename: Some("test.melbi"),
        charset: CharSet::Ascii,
    };

    fn render_error_string(source: &str, config: &RenderConfig) -> String {
        let arena = Bump::new();
        let engine = Engine::new(EngineOptions::default(), &arena, |_, _, env| env);
        let result = engine.compile(Default::default(), source, &[]);

        match result {
            Err(e) => {
                let mut buf = Vec::new();
                render_error_to(&e, &mut buf, config).unwrap();
                String::from_utf8_lossy(&buf).into_owned()
            }
            Ok(_) => panic!("Expected compilation error for source: {source}"),
        }
    }

    fn check_error(source: &str, config: &RenderConfig, expected: Expect) {
        let output = render_error_string(source, config);
        expected.assert_eq(&output);
    }

    #[test]
    fn test_parse_error_unicode() {
        check_error(
            "1 + + 2",
            &UNICODE_CONFIG,
            expect![[r#"
                [P001] Error: Expected expression, literal or identifier, found unexpected token
                   ╭─[ test.melbi:1:5 ]
                   │
                 1 │ 1 + + 2
                   │     │ 
                   │     ╰─ Expected expression, literal or identifier, found unexpected token
                ───╯
            "#]],
        );
    }

    #[test]
    fn test_parse_error_ascii() {
        check_error(
            "1 + + 2",
            &ASCII_CONFIG,
            expect![[r#"
                [P001] Error: Expected expression, literal or identifier, found unexpected token
                   ,-[ test.melbi:1:5 ]
                   |
                 1 | 1 + + 2
                   |     | 
                   |     `- Expected expression, literal or identifier, found unexpected token
                ---'
            "#]],
        );
    }

    #[test]
    fn test_type_error_unicode() {
        check_error(
            "1 + true",
            &UNICODE_CONFIG,
            expect![[r#"
                [E001] Error: Type mismatch: expected Int, found Bool
                   ╭─[ test.melbi:1:5 ]
                   │
                 1 │ 1 + true
                   │     ──┬─  
                   │       ╰─── Type mismatch: expected Int, found Bool
                   │ 
                   │ Help: Types must match in this context
                ───╯
            "#]],
        );
    }

    #[test]
    fn test_type_error_ascii() {
        check_error(
            "1 + true",
            &ASCII_CONFIG,
            expect![[r#"
                [E001] Error: Type mismatch: expected Int, found Bool
                   ,-[ test.melbi:1:5 ]
                   |
                 1 | 1 + true
                   |     ^^|^  
                   |       `--- Type mismatch: expected Int, found Bool
                   | 
                   | Help: Types must match in this context
                ---'
            "#]],
        );
    }

    #[test]
    fn test_unknown_identifier_unicode() {
        check_error(
            "foo + 1",
            &UNICODE_CONFIG,
            expect![[r#"
                [E002] Error: Undefined variable 'foo'
                   ╭─[ test.melbi:1:1 ]
                   │
                 1 │ foo + 1
                   │ ─┬─  
                   │  ╰─── Undefined variable 'foo'
                   │ 
                   │ Help: Make sure the variable is declared before use
                ───╯
            "#]],
        );
    }

    #[test]
    fn test_unknown_identifier_ascii() {
        check_error(
            "foo + 1",
            &ASCII_CONFIG,
            expect![[r#"
                [E002] Error: Undefined variable 'foo'
                   ,-[ test.melbi:1:1 ]
                   |
                 1 | foo + 1
                   | ^|^  
                   |  `--- Undefined variable 'foo'
                   | 
                   | Help: Make sure the variable is declared before use
                ---'
            "#]],
        );
    }

    #[test]
    fn test_unclosed_brace_unicode() {
        check_error(
            "{ x = 1",
            &UNICODE_CONFIG,
            expect![[r#"
                [P001] Error: Expected expression, found unexpected token
                   ╭─[ test.melbi:1:8 ]
                   │
                 1 │ { x = 1
                   │        │ 
                   │        ╰─ Expected expression, found unexpected token
                ───╯
            "#]],
        );
    }

    #[test]
    fn test_unclosed_brace_ascii() {
        check_error(
            "{ x = 1",
            &ASCII_CONFIG,
            expect![[r#"
                [P001] Error: Expected expression, found unexpected token
                   ,-[ test.melbi:1:8 ]
                   |
                 1 | { x = 1
                   |        | 
                   |        `- Expected expression, found unexpected token
                ---'
            "#]],
        );
    }

    #[test]
    fn test_charset_default_is_unicode() {
        assert_eq!(CharSet::default(), CharSet::Unicode);
    }

    #[test]
    fn test_render_config_default_charset() {
        let config = RenderConfig::default();
        assert_eq!(config.charset, CharSet::Unicode);
    }
}
