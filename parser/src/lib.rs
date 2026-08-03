#![no_std]
#![deny(unsafe_code)]

extern crate alloc;

pub mod ast;
pub mod builders;
pub mod literal;
pub mod parse;
mod span;
mod traits;

pub use builders::ArenaBuilder;
pub use span::Span;
pub use traits::{Tree, TreeBuilder, TreeDescriptor, TreeNode, Visit};

// TODO: the typed stage. It is a second set of `TreeDescriptor`s and changes
// neither `TreeBuilder` nor any existing pass, but it needs `melbi-values`
// first: a typed literal folds into a constant, so there has to be something to
// fold it into.
//
// The parsed stage is complete and the parser fills it, so `melbi-core` can
// start consuming this crate whenever the analyzer is ready — which is also what
// collapses the two copies of `expression.pest` back into one.
//
// A reference-counted builder and the passes over a sample tree live in
// `tests/`, since they exercise the crate purely through its public API and
// prove `TreeBuilder` is implementable from outside it.
