//! Type traversal utilities for folding and visiting type structures.
//!
//! This module provides two traversal patterns:
//! - [`Fold`]: A bottom-up traversal that can transform types and combine results
//! - [`Visit`]: A simple visitor pattern for read-only traversal
//!
//! # Example
//!
//! ```
//! use melbi_types::core::traversal::{Fold, FoldStep, drive_fold};
//! // ... usage example
//! ```

mod fold;
mod visit;

pub use fold::{Fold, FoldStep, TypeFolder, drive_fold, fold_type};
pub use visit::Visit;
