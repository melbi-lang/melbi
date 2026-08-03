//! Lexical helpers for literal *contents*, independent of the grammar.
//!
//! These take the text between a literal's delimiters and resolve its escape
//! sequences. They are deliberately free of both the grammar and the tree: the
//! parser calls them and interns the result into its builder.
//!
//! Each submodule keeps its own `UnescapeError`, as in the original, because the
//! two sets of failures barely overlap — a bytes literal has no Unicode escape,
//! and a string literal has no non-ASCII restriction.
//!
//! # Where an invalid escape is caught
//!
//! Today the split is uneven, inherited from the grammar. An *unknown* escape
//! like `"\q"` never reaches this module: the `string` rule accepts a known
//! escape or any non-backslash character, so `\q` makes the whole literal fail
//! to match and the user gets a generic syntax error pointing at the opening
//! quote. Only escapes that are well-formed but meaningless — `"\uD800"`, a
//! non-ASCII byte in `b"…"` — get here, and those produce a precise message and
//! an offset.
//
// TODO: worth evaluating whether the grammar should accept *any* `\` followed by
// a character and leave every rejection to this module. That trades one generic
// "expected a literal" for "invalid escape sequence '\q' at position 5", which
// is the better error; the cost is that the grammar stops describing which
// escapes exist, and a mis-typed escape inside an unterminated string may now
// resynchronise differently. Nilton's call — not a change to make quietly, since
// it also affects `tree-sitter/grammar.js`.
//
// TODO: only the unescape direction is here. `escape_string`/`escape_bytes` and
// `QuoteStyle` are still in `core/src/syntax/`, used by the formatter and the
// value printer; they belong beside these once the formatter migrates.

pub mod bytes;
pub mod string;

#[cfg(test)]
#[path = "unescape_test.rs"]
mod unescape_test;
