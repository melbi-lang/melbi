//! Parsing, provided by `melbi-parser`.
//!
//! This module adapts `melbi-parser` to the rest of `melbi-core`: it pairs the
//! parsed tree with its source and converts parse errors into core's diagnostics.

pub mod error;
mod syntax;

use bumpalo::Bump;
use melbi_parser::ArenaBuilder;
use melbi_parser::parse::{DEFAULT_MAX_PARSE_DEPTH, ParseOptions};

pub use error::ParseError;
pub use melbi_parser::ast::parsed::{
    Binding, BindingKind, Expr, ExprKind, LiteralKind, MapEntry, MapEntryKind, MatchArm,
    MatchArmKind, Pattern, PatternKind, Sexpr, TypeExpr, TypeExprKind, TypeField, TypeFieldKind,
};
pub use melbi_parser::ast::{BinaryOp, BoolOp, ComparisonOp, UnaryOp};
pub use melbi_parser::parse::{ExpressionParser, ParseErrorKind, Rule};
pub use syntax::{AnnotatedSource, Span};

/// A parsed tree, allocated in an arena.
pub type Tree<'a, D> = melbi_parser::Tree<ArenaBuilder<'a>, D>;

/// A literal in a parsed tree allocated in an arena.
pub type Literal<'a> = LiteralKind<ArenaBuilder<'a>>;

/// A parsed program: the root expression and the source it was parsed from.
#[derive(Debug, Clone, Copy)]
pub struct ParsedExpr<'a> {
    pub source: &'a str,
    pub expr: Tree<'a, Expr>,
}

/// Parses a Melbi expression.
pub fn parse<'a>(arena: &'a Bump, source: &'a str) -> Result<&'a ParsedExpr<'a>, ParseError> {
    parse_with_max_depth(arena, source, DEFAULT_MAX_PARSE_DEPTH)
}

/// Parses a Melbi expression with a custom maximum nesting depth.
///
/// This is useful for security-critical contexts where you want stricter limits,
/// or for testing depth limit behavior.
pub fn parse_with_max_depth<'a>(
    arena: &'a Bump,
    source: &'a str,
    max_depth: usize,
) -> Result<&'a ParsedExpr<'a>, ParseError> {
    let builder = ArenaBuilder::new(arena);
    let expr =
        melbi_parser::parse::parse_with_options(&builder, source, ParseOptions { max_depth })
            .map_err(|error| ParseError::new(error, source))?;
    Ok(arena.alloc(ParsedExpr { source, expr }))
}
