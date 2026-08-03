pub mod error;
mod parsed_expr;
pub mod parser;
mod syntax;

// Re-export the parser and rule enum for external use
pub use error::{ParseError, ParseErrorKind};
pub use parsed_expr::{Expr, Literal, MatchArm, ParsedExpr, Pattern, TypeExpr};
pub use parser::{ExpressionParser, Rule, parse, parse_with_max_depth};
pub use syntax::{AnnotatedSource, BinaryOp, BoolOp, ComparisonOp, Span, UnaryOp};

#[cfg(test)]
mod literals_test;

#[cfg(test)]
mod parse_test;

#[cfg(test)]
mod rule_valid_test;

#[cfg(test)]
mod precedence_test;
