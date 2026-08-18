//! Bytecode compilation errors.

use crate::api::{Diagnostic, Severity};
use crate::parser::Span;
use crate::{String, Vec};

/// Errors that can occur during bytecode compilation.
///
/// These are resource limit errors that can legitimately occur with very large programs.
/// Type-related errors should be caught by the type checker before compilation.
#[derive(Debug, Clone)]
pub enum CompileError {
    /// Too many local variables (limit: ~4 billion)
    TooManyLocals,
    /// Too many constants in constant pool (limit: ~4 billion)
    TooManyConstants,
    /// Jump distance exceeds maximum (limit: 65535 instructions)
    JumpTooFar,
}

impl core::fmt::Display for CompileError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::TooManyLocals => {
                write!(f, "Too many local variables (limit: ~4 billion)")
            }
            Self::TooManyConstants => {
                write!(f, "Too many constants (limit: ~4 billion)")
            }
            Self::JumpTooFar => {
                write!(f, "Jump distance too large (limit: 65535 instructions)")
            }
        }
    }
}

impl CompileError {
    /// Convert to a Diagnostic for API boundary.
    #[must_use]
    pub fn to_diagnostic(&self) -> Diagnostic {
        Diagnostic {
            severity: Severity::Error,
            message: String::from(match self {
                Self::TooManyLocals => "Too many local variables (limit: ~4 billion)",
                Self::TooManyConstants => "Too many constants (limit: ~4 billion)",
                Self::JumpTooFar => "Jump distance too large (limit: 65535 instructions)",
            }),
            span: Span::new(0, 0),
            related: Vec::new(),
            help: Vec::new(),
            code: None,
        }
    }
}
