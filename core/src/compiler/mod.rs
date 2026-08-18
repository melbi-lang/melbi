//! Bytecode compiler for Melbi expressions.
//!
//! This module provides a bytecode compiler that transforms typed AST expressions
//! into VM bytecode instructions. The compiler uses the visitor pattern to traverse
//! the expression tree and emit bytecode.
//!
//! ## Design
//!
//! - Uses `TreeTransformer` pattern for AST traversal
//! - Tracks stack depth precisely for debugging
//! - Implements jump patching for control flow (if/else, boolean short-circuit)
//! - Builds Code struct for VM execution

mod bytecode;
mod error;

#[cfg(test)]
mod bytecode_test;

pub use bytecode::BytecodeCompiler;
pub use error::CompileError;
