//! Melbi type system with pluggable type builders.
//!
//! This crate provides a generic type representation that works with
//! different storage strategies (arena, RC-based, encoded, etc.).
//!
//! # Example
//!
//! ```
//! use melbi_types::{ArenaBuilder, traits::TyBuilder, kind::{TyKind, Scalar}};
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
pub mod kind;
pub mod traits;

// Concrete builder implementations
mod arena_builder;
mod box_builder;

// TODO: Re-export top-level symbols.
pub use arena_builder::ArenaBuilder;
pub use box_builder::BoxBuilder;
