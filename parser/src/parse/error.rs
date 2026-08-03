//! What the parser reports when it cannot build a tree.
//!
//! Ported from `core/src/parser/error.rs`, with three differences.
//!
//! **No `Diagnostic`.** The original converted itself into `melbi_core`'s
//! `Diagnostic`, which is why it lived in `core`. This crate reports a plain
//! structured error and leaves rendering to whoever owns the diagnostics.
//!
//! **No copy of the source.** The original cloned the entire source text into
//! every error. The [`Span`] alone locates the failure, and the caller already
//! has the text it passed in.
//!
//! **Depth errors are structured.** The original formatted the depth into an
//! English message, then recovered the numbers by searching that message for
//! digits. Internal failures no longer travel as `pest` custom errors at all, so
//! [`MaxDepthExceeded`] simply carries its two numbers.
//!
//! [`MaxDepthExceeded`]: ParseErrorKind::MaxDepthExceeded

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::Span;

use super::grammar::Rule;

/// A parse failure, and where in the source it happened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub span: Span,
}

impl ParseError {
    pub fn new(kind: ParseErrorKind, span: Span) -> Self {
        Self { kind, span }
    }
}

/// The kinds of failure the parser distinguishes.
///
/// The split that matters is [`Malformed`] against everything else: the other
/// variants are the user's input being wrong, while `Malformed` means the parse
/// tree did not have the shape the grammar guarantees — a bug in this crate, not
/// in the input.
///
/// The original also declared `UnclosedDelimiter` and `InvalidNumber`. Both were
/// matched on when rendering and never constructed anywhere, so neither is
/// ported.
///
/// [`Malformed`]: ParseErrorKind::Malformed
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    /// The grammar did not accept the input here.
    UnexpectedToken { expected: String, found: String },

    /// A literal was well-formed to the grammar but could not be interpreted:
    /// an integer too large for `i64`, a bad escape sequence, a suffix on a
    /// pattern literal.
    InvalidLiteral { message: String },

    /// Nesting ran deeper than the configured limit. This is a guard against
    /// stack exhaustion on hostile input, not a language restriction.
    MaxDepthExceeded { depth: usize, max_depth: usize },

    /// The parse tree did not match what the grammar promises — a missing
    /// child, or a rule in a position the parser does not handle.
    ///
    /// Reaching this means the grammar and this parser have drifted apart.
    Malformed { message: String },
}

impl core::fmt::Display for ParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match &self.kind {
            ParseErrorKind::UnexpectedToken { expected, found } => {
                write!(f, "expected {expected}, found {found}")
            }
            ParseErrorKind::InvalidLiteral { message } => write!(f, "{message}"),
            ParseErrorKind::MaxDepthExceeded { max_depth, .. } => write!(
                f,
                "expression nesting depth exceeds the maximum of {max_depth} levels"
            ),
            ParseErrorKind::Malformed { message } => {
                write!(f, "internal parser error: {message}")
            }
        }
    }
}

impl ParseError {
    /// Convert the error `pest` produces when the grammar rejects the input.
    pub(super) fn from_pest(error: pest::error::Error<Rule>) -> Self {
        let span = match error.location {
            pest::error::InputLocation::Pos(pos) => Span::new(pos as u32, pos as u32),
            pest::error::InputLocation::Span((start, end)) => Span::new(start as u32, end as u32),
        };

        let kind = match error.variant {
            pest::error::ErrorVariant::ParsingError {
                positives,
                negatives,
            } => ParseErrorKind::UnexpectedToken {
                expected: describe_expected(&positives),
                found: describe_found(&negatives),
            },
            // The parser no longer routes its own failures through `pest`, so
            // this is only reachable if the grammar itself raises one.
            pest::error::ErrorVariant::CustomError { message } => {
                ParseErrorKind::Malformed { message }
            }
        };

        Self::new(kind, span)
    }
}

/// Describe the rules that would have been accepted, in the user's vocabulary
/// rather than the grammar's.
fn describe_expected(rules: &[Rule]) -> String {
    let mut concepts: Vec<&str> = Vec::new();

    for rule in rules {
        let concept = match rule {
            Rule::integer | Rule::float | Rule::boolean | Rule::string | Rule::bytes => "literal",
            Rule::ident => "identifier",
            Rule::EOI => "end of input",
            // Everything else — operators, groupings, the prefix rules — reads
            // as "an expression was expected here".
            _ => "expression",
        };
        if !concepts.contains(&concept) {
            concepts.push(concept);
        }
    }

    match concepts.split_last() {
        None => "something else".to_string(),
        Some((last, [])) => (*last).to_string(),
        Some((last, rest)) => format!("{} or {}", rest.join(", "), last),
    }
}

/// Describe what was actually found.
fn describe_found(rules: &[Rule]) -> String {
    let Some(rule) = rules.first() else {
        return "unexpected token".to_string();
    };

    match rule {
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
        other => format!("{other:?}"),
    }
}
