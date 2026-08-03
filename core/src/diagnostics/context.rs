use alloc::string::ToString;

use crate::api::RelatedInfo;
use crate::parser::Span;
use crate::{String, format};

/// Context information for error messages.
///
/// Provides additional information about where an error occurred,
/// such as "in function call", "while unifying types", etc.
/// Each context entry can be converted to a RelatedInfo for diagnostic display.
#[derive(Debug, Clone)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub enum Context {
    /// In a function call
    InFunctionCall { name: Option<String>, span: Span },
    /// While unifying types
    WhileUnifying { what: String, span: Span },
    /// Where something was defined
    DefinedHere { what: String, span: Span },
    /// Where a type was inferred
    InferredHere { type_name: String, span: Span },
    /// In an expression
    InExpression { kind: String, span: Span },
    /// Where a polymorphic function was instantiated
    InstantiatedHere { span: Span },
}

impl Context {
    /// Convert to a RelatedInfo for diagnostic display
    pub fn to_related_info(&self) -> RelatedInfo {
        match self {
            Context::InFunctionCall { name, span } => RelatedInfo {
                span: span.clone(),
                message: match name {
                    Some(n) => format!("in call to function '{}'", n),
                    None => "in function call".to_string(),
                },
            },
            Context::WhileUnifying { what, span } => RelatedInfo {
                span: span.clone(),
                message: format!("while checking {}", what),
            },
            Context::DefinedHere { what, span } => RelatedInfo {
                span: span.clone(),
                message: format!("{} defined here", what),
            },
            Context::InferredHere { type_name, span } => RelatedInfo {
                span: span.clone(),
                message: format!("type '{}' inferred here", type_name),
            },
            Context::InExpression { kind, span } => RelatedInfo {
                span: span.clone(),
                message: format!("in {}", kind),
            },
            Context::InstantiatedHere { span } => RelatedInfo {
                span: span.clone(),
                message: "when instantiated here".to_string(),
            },
        }
    }
}
