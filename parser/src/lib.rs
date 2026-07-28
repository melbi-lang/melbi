#![no_std]
#![deny(unsafe_code)]

extern crate alloc;

pub mod ast;
mod span;
mod traits;

pub use span::Span;
pub use traits::{Tree, TreeBuilder, TreeDescriptor, TreeNode, Visit};

// TODO: port the rest of the AST (patterns, bindings, match arms, type syntax)
// and then the typed stage. Each is a new `TreeDescriptor`; none of them changes
// `TreeBuilder` or any existing pass.
//
// The storage strategies (arena, reference-counted heap) and the passes over a
// sample tree live in `tests/`, since they exercise the crate purely through its
// public API.
