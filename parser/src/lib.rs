#![no_std]
#![deny(unsafe_code)]

extern crate alloc;

pub mod ast;
pub mod traits;

pub use traits::{Tree, TreeBuilder, TreeNode, Visit};

// TODO: port the AST itself (`ExprKind`, patterns, type syntax) once the base
// tree is settled. That additionally needs mutually recursive trees, which this
// single-tree scaffolding does not cover.

#[cfg(test)]
mod test_utils;

#[cfg(test)]
mod arena_builder_test;

#[cfg(test)]
mod heap_builder_test;
