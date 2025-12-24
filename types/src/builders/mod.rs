//! Type builder implementations for different storage strategies.
//!
//! - [`ArenaBuilder`]: Arena-based allocation with type interning for deduplication
//! - [`BoxBuilder`]: RC-based allocation without interning, suitable for simpler use cases

mod arena_builder;
mod box_builder;

pub use arena_builder::ArenaBuilder;
pub use box_builder::BoxBuilder;
