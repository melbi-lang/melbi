#![no_std]
#![deny(unsafe_code)]

extern crate alloc;

pub mod ast;
mod span;
pub mod traits;

pub use span::Span;
pub use traits::{Tree, TreeBuilder, TreeDescriptor, TreeNode, Visit};

// TODO: port the rest of the AST (patterns, bindings, match arms, type syntax)
// and then the typed stage. Each is a new `TreeDescriptor`; none of them changes
// `TreeBuilder` or any existing pass.

#[cfg(test)]
mod test_utils;

#[cfg(test)]
mod arena_builder_test;

#[cfg(test)]
mod heap_builder_test;
