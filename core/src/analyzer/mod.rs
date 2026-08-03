pub mod analyzer;
pub mod error;
pub mod typed_expr;

#[cfg(test)]
mod analyzer_test;

pub use analyzer::analyze;
pub use error::{TypeError, TypeErrorKind};
