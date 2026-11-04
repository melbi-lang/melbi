use alloc::string::ToString;

use crate::api::{Diagnostic, Severity};
use crate::parser::{ParseErrorKind, Span};
use crate::{String, format, vec};

/// A parse error, together with the source it was found in.
#[derive(Debug)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub source: String,
    pub span: Span,
    message: String,
}

impl ParseError {
    #[must_use]
    pub fn new(error: melbi_parser::parse::ParseError, source: &str) -> Self {
        // melbi-parser's messages follow the Rust convention of starting in
        // lowercase; diagnostics start with a capital, like type errors do.
        let message = capitalize(&error.to_string());
        Self {
            span: error.span.into(),
            kind: error.kind,
            source: source.to_string(),
            message,
        }
    }

    /// Convert to a Diagnostic for API boundary
    #[must_use]
    pub fn to_diagnostic(&self) -> Diagnostic {
        let (code, help) = match &self.kind {
            ParseErrorKind::UnexpectedToken { .. } => ("P001", vec![]),
            ParseErrorKind::MaxDepthExceeded { .. } => (
                "P004",
                vec!["Reduce nesting or simplify the expression".to_string()],
            ),
            ParseErrorKind::InvalidLiteral { .. } => ("P005", vec![]),
            ParseErrorKind::Malformed { .. } => ("P999", vec![]),
        };

        Diagnostic {
            severity: Severity::Error,
            message: self.message.clone(),
            span: self.span.clone(),
            related: vec![],
            help,
            code: Some(code.to_string()),
        }
    }
}

fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
        None => String::new(),
    }
}

impl core::fmt::Display for ParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let diagnostic = self.to_diagnostic();
        write!(f, "{}: {}", diagnostic.severity, diagnostic.message)?;
        if let Some(ref code) = diagnostic.code {
            write!(f, " [{code}]")?;
        }
        for help_msg in &diagnostic.help {
            write!(f, "\nhelp: {help_msg}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use bumpalo::Bump;

    use crate::parser::parse;

    fn diagnostic_of(source: &str) -> crate::api::Diagnostic {
        let arena = Bump::new();
        let source = arena.alloc_str(source);
        parse(&arena, source).unwrap_err().to_diagnostic()
    }

    #[test]
    fn unexpected_token_diagnostic() {
        let diagnostic = diagnostic_of("1 +");
        assert_eq!(diagnostic.code.as_deref(), Some("P001"));
        assert!(
            diagnostic.message.starts_with("Expected "),
            "{}",
            diagnostic.message
        );
        assert!(diagnostic.help.is_empty());
    }

    #[test]
    fn max_depth_diagnostic() {
        let diagnostic = diagnostic_of(&format!("{}1{}", "(".repeat(600), ")".repeat(600)));
        assert_eq!(diagnostic.code.as_deref(), Some("P004"));
        assert_eq!(
            diagnostic.message,
            "Expression nesting depth exceeds the maximum of 500 levels"
        );
        assert_eq!(
            diagnostic.help,
            ["Reduce nesting or simplify the expression"]
        );
    }

    #[test]
    fn invalid_literal_diagnostic() {
        let diagnostic = diagnostic_of("99999999999999999999");
        assert_eq!(diagnostic.code.as_deref(), Some("P005"));
        assert_eq!(
            diagnostic.message,
            "Integer literal does not fit in a 64-bit integer"
        );
        assert_eq!(diagnostic.span, crate::parser::Span(0..20));
    }
}
