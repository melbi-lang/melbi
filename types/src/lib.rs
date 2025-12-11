//! Melbi type system with pluggable type builders.
//!
//! This crate provides a generic type representation that works with
//! different storage strategies (arena, RC-based, encoded, etc.).
//!
//! # Example
//!
//! ```
//! use melbi_types::{ArenaBuilder, TyBuilder, TyKind, Scalar};
//! use bumpalo::Bump;
//!
//! let arena = Bump::new();
//! let builder = ArenaBuilder::new(&arena);
//!
//! let int_ty = TyKind::Scalar(Scalar::Int).alloc(&builder);
//! let arr_ty = TyKind::Array(int_ty).alloc(&builder);
//! ```

#![no_std]
extern crate alloc;

pub mod algo;
pub mod builders;
pub mod core;

// Re-export commonly used types for convenience
pub use builders::{ArenaBuilder, BoxBuilder};
pub use core::{Ident, Scalar, Ty, TyBuilder, TyKind, TyNode};
