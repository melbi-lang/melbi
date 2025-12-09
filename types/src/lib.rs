//! Melbi type system with pluggable type builders.
//!
//! This crate provides a generic type representation that works with
//! different storage strategies (arena, RC-based, encoded, etc.).
//!
//! # Example
//!
//! ```ignore
//! use melbi_types::{TypeBuilder, ArenaBuilder, Scalar};
//! use bumpalo::Bump;
//!
//! let arena = Bump::new();
//! let builder = ArenaBuilder::new(&arena);
//!
//! let int_ty = builder.int();
//! let arr_ty = builder.array(int_ty);
//! ```

#![no_std]
extern crate alloc;

pub mod algo;
pub mod kind;
pub mod traits;

// Concrete builder implementations
mod arena_builder;
mod box_builder;

// TODO: Re-export top-level symbols.
pub use arena_builder::ArenaBuilder;
pub use box_builder::BoxBuilder;
