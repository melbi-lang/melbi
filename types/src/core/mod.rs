//! Core type system components.
//!
//! This module provides the fundamental building blocks for the type system:
//!
//! - [`TyBuilder`]: Trait for type allocation strategies
//! - [`Ty`] and [`TyNode`]: Type handles and their underlying nodes
//! - [`TyKind`]: The different kinds of types (scalars, arrays, maps, etc.)
//! - [`TyFlags`]: Cached type properties for efficient queries
//!
//! See the [`traversal`] submodule for type folding and visiting utilities.

mod builder;
mod flags;
mod kind;
pub mod traversal;
mod ty;

pub use builder::TyBuilder;
pub use flags::TyFlags;
pub use kind::{Scalar, TyKind};
pub use ty::{FieldList, Ident, IdentList, Ty, TyList, TyNode};
