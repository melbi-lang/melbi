//! Runtime evaluation errors.
//!
//! These are errors that can occur during expression evaluation.
//! Note: Many error conditions (type mismatches, undefined variables in typed code, etc.)
//! are caught by the analyzer and will never occur if the expression is type-checked first.
//!
//! # Error Categories
//!
//! - **Runtime errors**: Validation/logic errors during evaluation that can be caught
//!   by the `otherwise` operator (e.g., division by zero, index out of bounds).
//!
//! - **Resource exceeded errors**: Fatal resource limit violations that cannot be caught
//!   (e.g., stack overflow). These propagate through `otherwise` to prevent hiding
//!   serious resource exhaustion issues.
//!
//! - **Internal errors**: Compiler/interpreter bugs that cannot be caught (e.g., invariant
//!   violations). These propagate through `otherwise` to prevent masking serious bugs that
//!   need to be reported and fixed.

use alloc::string::ToString;
use core::fmt;

use crate::parser::Span;
use crate::{String, format, vec};

/// Execution error.
#[derive(Debug)]
pub struct ExecutionError {
    pub kind: ExecutionErrorKind,
    pub source: String,
    pub span: Span,
}

/// Variants of execution error.
#[derive(Debug, PartialEq, Eq)]
pub enum ExecutionErrorKind {
    /// Runtime error that can be caught by the `otherwise` operator.
    Runtime(RuntimeError),

    /// Resource limit exceeded (cannot be caught by `otherwise`).
    ResourceExceeded(ResourceExceededError),

    /// Internal compiler/interpreter bug (cannot be caught by `otherwise`).
    Internal(InternalError),
}

/// Runtime errors that can be caught by the `otherwise` operator.
///
/// These represent validation/logic errors during expression evaluation
/// that are part of normal program flow and can be recovered from using
/// the `otherwise` operator.
#[derive(Debug, PartialEq, Eq)]
pub enum RuntimeError {
    /// Division by zero (integer or float division).
    DivisionByZero {},

    /// Integer overflow (e.g., `i64::MIN` / -1).
    IntegerOverflow {},

    /// Array index out of bounds.
    IndexOutOfBounds { index: i64, len: usize },

    /// Map key not found during indexing operation.
    KeyNotFound { key_display: String },

    /// Cast error (e.g., invalid UTF-8 when casting Bytes → Str).
    ///
    /// TODO(effects): When effect system is implemented, mark fallible casts
    /// with `!` effect and make them catchable with `otherwise`.
    CastError { message: String },
}

/// Resource limit exceeded errors that cannot be caught.
///
/// These represent fatal resource exhaustion that terminates evaluation.
/// The `otherwise` operator does not catch these errors to prevent hiding
/// serious resource issues like stack overflow.
#[derive(Debug, PartialEq, Eq)]
pub enum ResourceExceededError {
    /// Evaluation recursion depth exceeded.
    StackOverflow { depth: usize, max_depth: usize },
    // Future resource limits:
    // MemoryExceeded { bytes: usize, max_bytes: usize },
    // TimeExceeded { millis: u64, max_millis: u64 },
}

/// Internal errors that indicate bugs in the compiler/interpreter (cannot be caught).
///
/// These represent violations of internal invariants that should never occur in
/// well-typed programs. The `otherwise` operator does not catch these errors to
/// prevent masking serious compiler bugs that need to be reported and fixed.
///
/// If you encounter these errors, please report them as bugs.
#[derive(Debug, PartialEq, Eq)]
pub enum InternalError {
    /// Internal invariant violation (indicates a bug in the type checker or evaluator).
    ///
    /// This should never occur in a well-typed program. If this error appears,
    /// it indicates a bug in the compiler/interpreter that should be reported.
    InvariantViolation { message: String },
}

impl ExecutionError {
    /// Convert to a Diagnostic for API boundary
    pub fn to_diagnostic(&self) -> crate::api::Diagnostic {
        use crate::api::{Diagnostic, Severity};

        let (message, code, help) = match &self.kind {
            ExecutionErrorKind::Runtime(RuntimeError::DivisionByZero {}) => (
                String::from("Division by zero"),
                Some("R001"),
                vec!["Check that divisor is not zero before division".to_string()],
            ),
            ExecutionErrorKind::Runtime(RuntimeError::IntegerOverflow {}) => (
                String::from("Integer overflow"),
                Some("R007"),
                vec!["The operation would overflow the integer range".to_string()],
            ),
            ExecutionErrorKind::Runtime(RuntimeError::IndexOutOfBounds { index, len }) => (
                format!("Index {index} out of bounds (length: {len})"),
                Some("R002"),
                vec!["Ensure index is within valid range [0, length)".to_string()],
            ),
            ExecutionErrorKind::Runtime(RuntimeError::KeyNotFound { key_display }) => (
                format!("Key not found: {key_display}"),
                Some("R003"),
                vec!["Use 'otherwise' to provide a fallback value for missing keys".to_string()],
            ),
            ExecutionErrorKind::Runtime(RuntimeError::CastError { message }) => (
                format!("Cast error: {message}"),
                Some("R004"),
                vec!["Verify the value can be safely converted to the target type".to_string()],
            ),
            ExecutionErrorKind::ResourceExceeded(ResourceExceededError::StackOverflow {
                depth,
                max_depth,
            }) => (
                format!("Stack overflow: depth {depth} exceeds maximum of {max_depth}"),
                Some("R005"),
                vec!["Reduce recursion depth or increase stack limit".to_string()],
            ),
            ExecutionErrorKind::Internal(InternalError::InvariantViolation { message }) => (
                format!("Internal error: {message}"),
                Some("R006"),
                vec!["This is a bug in the compiler/interpreter - please report it".to_string()],
            ),
        };

        Diagnostic {
            severity: Severity::Error,
            message,
            span: self.span.clone(),
            related: crate::Vec::new(),
            help,
            code: code.map(String::from),
        }
    }
}

impl fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} @ {}", self.kind, format_span(&self.span))
    }
}

impl fmt::Display for ExecutionErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Runtime(e) => write!(f, "{e}"),
            Self::ResourceExceeded(e) => write!(f, "{e}"),
            Self::Internal(e) => write!(f, "{e}"),
        }
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DivisionByZero {} => {
                write!(f, "Division by zero")
            }
            Self::IntegerOverflow {} => {
                write!(f, "Integer overflow")
            }
            Self::IndexOutOfBounds { index, len } => {
                write!(f, "Index {index} out of bounds (length: {len})")
            }
            Self::KeyNotFound { key_display } => {
                write!(f, "Key not found: {key_display}")
            }
            Self::CastError { message } => {
                write!(f, "Cast error: {message}")
            }
        }
    }
}

impl fmt::Display for ResourceExceededError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StackOverflow { depth, max_depth } => {
                write!(
                    f,
                    "Evaluation stack overflow: depth {depth} exceeds maximum of {max_depth}"
                )
            }
        }
    }
}

impl fmt::Display for InternalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvariantViolation { message } => {
                write!(f, "Internal error: {message}")
            }
        }
    }
}

/// Format a span as a byte range for error messages.
///
/// Returns a string like "5..12" representing the byte range.
/// In the future, this could be enhanced to show line:column information
/// if source text is available.
fn format_span(span: &Span) -> String {
    alloc::format!("{}..{}", span.0.start, span.0.end)
}

// Convenient conversions for error construction
impl From<RuntimeError> for ExecutionErrorKind {
    fn from(e: RuntimeError) -> Self {
        Self::Runtime(e)
    }
}

impl From<ResourceExceededError> for ExecutionErrorKind {
    fn from(e: ResourceExceededError) -> Self {
        Self::ResourceExceeded(e)
    }
}

impl From<InternalError> for ExecutionErrorKind {
    fn from(e: InternalError) -> Self {
        Self::Internal(e)
    }
}

impl From<crate::casting::CastError> for ExecutionErrorKind {
    fn from(e: crate::casting::CastError) -> Self {
        // Convert CastError to RuntimeError::CastError for uniform error handling
        Self::Runtime(RuntimeError::CastError {
            message: alloc::format!("{e}"),
        })
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ExecutionError {}

#[cfg(feature = "std")]
impl std::error::Error for RuntimeError {}

#[cfg(feature = "std")]
impl std::error::Error for ResourceExceededError {}

#[cfg(feature = "std")]
impl std::error::Error for InternalError {}
