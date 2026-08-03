//! Source text to a [`parsed`](crate::ast::parsed) tree.
//!
//! [`parse`] is the whole interface: hand it a [`TreeBuilder`] and some source,
//! and get back the one expression that is the program.
//!
//! ```ignore
//! let arena = bumpalo::Bump::new();
//! let builder = melbi_parser::ArenaBuilder::new(&arena);
//! let tree = melbi_parser::parse::parse(&builder, "1 + 2")?;
//! ```
//!
//! The parser is generic over storage, so the same call fills an arena, a
//! reference-counted heap, or anything else implementing [`TreeBuilder`].
//!
//! [`TreeBuilder`]: crate::TreeBuilder

mod context;
mod error;
mod grammar;

pub use context::{DEFAULT_MAX_PARSE_DEPTH, ParseOptions, parse, parse_with_options};
pub use error::{ParseError, ParseErrorKind};
pub use grammar::{ExpressionParser, Rule};

#[cfg(test)]
#[path = "parse_test.rs"]
mod parse_test;
