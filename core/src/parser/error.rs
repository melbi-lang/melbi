use alloc::string::ToString;

use crate::api::{Diagnostic, Severity};
use crate::diagnostics::context::Context;
use crate::parser::{Rule, Span};
use crate::{String, Vec, format, vec};

/// Parser error with context
#[derive(Debug)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub source: String,
    pub span: Span,
    pub context: Vec<Context>,
}

/// Specific kinds of parse errors
#[derive(Debug)]
pub enum ParseErrorKind {
    /// Unexpected token
    UnexpectedToken { expected: String, found: String },
    /// Unclosed delimiter
    UnclosedDelimiter { delimiter: char },
    /// Invalid number literal
    InvalidNumber { text: String },
    /// Maximum nesting depth exceeded
    MaxDepthExceeded { depth: usize, max_depth: usize },
    /// Other parse errors (catch-all for Pest errors we don't specifically handle)
    Other { message: String },
}

impl ParseError {
    /// Create a new `ParseError` with no context
    #[must_use]
    pub fn new(kind: ParseErrorKind, source: String, span: Span) -> Self {
        Self {
            kind,
            source,
            span,
            context: Vec::new(),
        }
    }

    /// Convert to a Diagnostic for API boundary
    #[must_use]
    pub fn to_diagnostic(&self) -> Diagnostic {
        let (message, code, help) = match &self.kind {
            ParseErrorKind::UnexpectedToken {
                expected, found, ..
            } => (
                format!("Expected {expected}, found {found}"),
                Some("P001"),
                vec![],
            ),
            ParseErrorKind::UnclosedDelimiter { delimiter, .. } => (
                format!("Unclosed delimiter '{delimiter}'"),
                Some("P002"),
                vec!["Add the missing closing delimiter".to_string()],
            ),
            ParseErrorKind::InvalidNumber { text, .. } => (
                format!("Invalid number literal '{text}'"),
                Some("P003"),
                vec!["Check the number format".to_string()],
            ),
            ParseErrorKind::MaxDepthExceeded { max_depth, .. } => (
                format!(
                    "Expression nesting depth exceeds maximum of {max_depth} levels"
                ),
                Some("P004"),
                vec!["Reduce nesting or simplify the expression".to_string()],
            ),
            ParseErrorKind::Other { message, .. } => (message.clone(), Some("P999"), vec![]),
        };

        Diagnostic {
            severity: Severity::Error,
            message,
            span: self.span.clone(),
            related: self
                .context
                .iter()
                .map(super::super::diagnostics::context::Context::to_related_info)
                .collect(),
            help,
            code: code.map(alloc::string::ToString::to_string),
        }
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

/// Convert Pest error to human-readable `ParseError`
#[must_use]
pub fn convert_pest_error(err: pest::error::Error<Rule>, source: &str) -> ParseError {
    use pest::error::ErrorVariant;

    let span = match err.location {
        pest::error::InputLocation::Pos(pos) => Span(pos..pos),
        pest::error::InputLocation::Span((start, end)) => Span(start..end),
    };

    let kind = match err.variant {
        ErrorVariant::ParsingError {
            positives,
            negatives,
        } => {
            // Convert technical Pest messages to human-readable ones
            let expected = format_expected_rules(&positives);
            let found = format_found_rules(&negatives);

            ParseErrorKind::UnexpectedToken { expected, found }
        }
        ErrorVariant::CustomError { message } => {
            // Check if it's a depth error
            if message.contains("nesting depth") {
                // Try to extract current depth: try "depth" first, then "of" as fallback
                let depth_opt = extract_number_from_message(&message, "depth").or_else(|| {
                    // If we can't find it after "depth", try the first number after "depth exceeds"
                    if let Some(pos) = message.find("depth exceeds") {
                        let after = &message[pos + "depth exceeds".len()..];
                        extract_first_number(after)
                    } else {
                        None
                    }
                });

                // Try to extract max_depth: try "maximum" first, then "of" as fallback
                let max_depth_opt = extract_number_from_message(&message, "maximum")
                    .or_else(|| extract_number_from_message(&message, "of"));

                // If we found at least the max_depth, construct the error
                if let Some(max_depth_str) = max_depth_opt
                    && let Ok(max_depth) = max_depth_str.parse::<usize>()
                {
                    // Try to parse depth, or use max_depth as fallback (since we exceeded it)
                    let depth = depth_opt
                        .and_then(|s| s.parse::<usize>().ok())
                        .unwrap_or(max_depth);

                    return ParseError::new(
                        ParseErrorKind::MaxDepthExceeded { depth, max_depth },
                        source.to_string(),
                        span,
                    );
                }
            }

            ParseErrorKind::Other { message }
        }
    };

    ParseError::new(kind, source.to_string(), span)
}

/// Format expected rules in a human-readable way
fn format_expected_rules(rules: &[Rule]) -> String {
    if rules.is_empty() {
        return "something else".to_string();
    }

    // Group related rules into higher-level concepts
    let mut concepts = Vec::new();

    for rule in rules {
        match rule {
            Rule::grouped | Rule::neg | Rule::not | Rule::if_op | Rule::lambda_op => {
                if !concepts.contains(&"expression") {
                    concepts.push("expression");
                }
            }
            Rule::integer | Rule::float | Rule::boolean | Rule::string | Rule::bytes => {
                if !concepts.contains(&"literal") {
                    concepts.push("literal");
                }
            }
            Rule::ident => {
                if !concepts.contains(&"identifier") {
                    concepts.push("identifier");
                }
            }
            Rule::EOI => {
                if !concepts.contains(&"end of input") {
                    concepts.push("end of input");
                }
            }
            _ => {
                // For other rules, just note "expression"
                if !concepts.contains(&"expression") {
                    concepts.push("expression");
                }
            }
        }
    }

    if concepts.is_empty() {
        return "something else".to_string();
    }

    if concepts.len() == 1 {
        concepts[0].to_string()
    } else {
        let last = concepts.pop().unwrap();
        format!("{} or {}", concepts.join(", "), last)
    }
}

/// Format found rules in a human-readable way
fn format_found_rules(rules: &[Rule]) -> String {
    if rules.is_empty() {
        return "unexpected token".to_string();
    }

    // Map to human-readable description
    match rules[0] {
        Rule::ident => "identifier".to_string(),
        Rule::integer => "integer".to_string(),
        Rule::float => "floating-point number".to_string(),
        Rule::boolean => "boolean".to_string(),
        Rule::string => "string".to_string(),
        Rule::bytes => "byte string".to_string(),
        Rule::EOI => "end of input".to_string(),
        Rule::grouped => "grouped expression".to_string(),
        Rule::neg => "negation".to_string(),
        Rule::not => "logical not".to_string(),
        Rule::if_op => "if expression".to_string(),
        Rule::lambda_op => "lambda expression".to_string(),
        _ => format!("{:?}", rules[0]),
    }
}

/// Extract a number from a message string after a keyword
fn extract_number_from_message(message: &str, keyword: &str) -> Option<String> {
    // Look for "keyword N" pattern
    let keyword_pos = message.find(keyword)?;
    let after_keyword = &message[keyword_pos + keyword.len()..];

    // Skip whitespace and "of"
    let trimmed = after_keyword.trim_start();
    let trimmed = if let Some(stripped) = trimmed.strip_prefix("of") {
        stripped.trim_start()
    } else {
        trimmed
    };

    // Extract digits
    let digits: String = trimmed.chars().take_while(char::is_ascii_digit).collect();

    if digits.is_empty() {
        None
    } else {
        Some(digits)
    }
}

/// Extract the first number found in a string
fn extract_first_number(s: &str) -> Option<String> {
    let trimmed = s.trim_start();
    let digits: String = trimmed.chars().take_while(char::is_ascii_digit).collect();

    if digits.is_empty() {
        None
    } else {
        Some(digits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_error_to_diagnostic() {
        let error = ParseError::new(
            ParseErrorKind::UnexpectedToken {
                expected: "expression".to_string(),
                found: "comma".to_string(),
            },
            "test source".to_string(),
            Span(10..20),
        );

        let diagnostic = error.to_diagnostic();
        assert_eq!(diagnostic.severity, Severity::Error);
        assert!(diagnostic.message.contains("Expected expression"));
        assert!(diagnostic.message.contains("found comma"));
        assert_eq!(diagnostic.code, Some("P001".to_string()));
    }

    #[test]
    fn format_expected_rules_works() {
        let rules = vec![Rule::integer, Rule::float];
        let formatted = format_expected_rules(&rules);
        assert_eq!(formatted, "literal");
    }

    #[test]
    fn extract_number_from_message_works() {
        let message = "nesting depth 150 exceeds maximum of 100 levels";
        assert_eq!(
            extract_number_from_message(message, "depth"),
            Some("150".to_string())
        );
        assert_eq!(
            extract_number_from_message(message, "maximum"),
            Some("100".to_string())
        );
    }

    #[test]
    fn depth_error_conversion_with_both_numbers() {
        // Test with format that includes both current depth and max depth
        let pest_err = pest::error::Error::<Rule>::new_from_pos(
            pest::error::ErrorVariant::CustomError {
                message: "nesting depth 150 exceeds maximum of 100 levels".to_string(),
            },
            pest::Position::from_start("test"),
        );

        let parse_err = convert_pest_error(pest_err, "test");
        match parse_err.kind {
            ParseErrorKind::MaxDepthExceeded {
                depth, max_depth, ..
            } => {
                assert_eq!(depth, 150);
                assert_eq!(max_depth, 100);
            }
            _ => panic!("Expected MaxDepthExceeded error"),
        }
    }

    #[test]
    fn depth_error_conversion_with_only_max() {
        // Test with format that only includes max depth (actual parser format)
        let pest_err = pest::error::Error::<Rule>::new_from_pos(
            pest::error::ErrorVariant::CustomError {
                message: "Expression nesting depth exceeds maximum of 500 levels. \
                         This likely indicates excessively nested parentheses or other constructs."
                    .to_string(),
            },
            pest::Position::from_start("test"),
        );

        let parse_err = convert_pest_error(pest_err, "test");
        match parse_err.kind {
            ParseErrorKind::MaxDepthExceeded {
                depth, max_depth, ..
            } => {
                assert_eq!(max_depth, 500);
                // When current depth is not in message, we use max_depth as fallback
                assert_eq!(depth, 500);
            }
            _ => panic!("Expected MaxDepthExceeded error, got {:?}", parse_err.kind),
        }
    }
}
