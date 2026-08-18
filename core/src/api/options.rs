//! Configuration options for the Melbi engine.

/// Configuration options for the Melbi engine.
///
/// These options set the defaults for compilation and execution,
/// which can be overridden on a per-call basis.
///
/// # Example
///
/// ```
/// use melbi_core::api::{EngineOptions, CompileOptions, RunOptions};
///
/// let options = EngineOptions {
///     default_compile_options: CompileOptions::default(),
///     default_run_options: RunOptions {
///         max_depth: 500,
///         max_iterations: Some(10_000),
///     },
/// };
/// ```
#[derive(Debug, Clone, Default)]
pub struct EngineOptions {
    /// Default options for compilation.
    ///
    /// These can be overridden when calling `Engine::compile()`.
    pub default_compile_options: CompileOptions,

    /// Default options for execution.
    ///
    /// These can be overridden when calling `CompiledExpression::run()`.
    pub default_run_options: RunOptions,
}

/// Configuration options for compilation.
///
/// These options control compile-time behavior and optimizations.
///
/// # Example
///
/// ```
/// use melbi_core::api::CompileOptions;
///
/// let options = CompileOptions::default();
/// ```
#[derive(Debug, Clone, Default)]
pub struct CompileOptions {
    // Future: optimization level, type checking strictness, etc.
}

impl CompileOptions {
    /// Override this options with another, preferring values from `other` when specified.
    ///
    /// For each field, if `other` specifies a value (is `Some`), use it.
    /// Otherwise, keep the value from `self`.
    pub fn override_with(&mut self, _other: &CompileOptionsOverride) {}
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CompileOptionsOverride {}

/// Configuration options for expression execution.
///
/// All fields are `Option` to support partial specification and merging.
/// `None` means "not specified, use default". When merging, values from
/// the override take precedence over the base.
///
/// # Example
///
/// ```
/// use melbi_core::api::RunOptions;
///
/// // Specify only max_depth, use default for max_iterations
/// let options = RunOptions {
///     max_depth: 500,
///     max_iterations: None,
/// };
/// ```
#[derive(Debug, Clone, Copy)]
pub struct RunOptions {
    /// Maximum evaluation stack depth (for recursion protection).
    ///
    /// `None` means not specified (use default: 1000).
    pub max_depth: usize,

    /// Maximum number of iterations in loops.
    ///
    /// - `None` = not specified, use default (unlimited)
    /// - `Some(None)` = explicitly unlimited
    /// - `Some(Some(n))` = explicitly limited to n iterations
    ///
    /// TODO: Consider using a custom enum like `IterationLimit { Unlimited, Limited(usize) }`
    /// instead of nested Option for better ergonomics.
    pub max_iterations: Option<usize>,
}

impl RunOptions {
    /// Override this options with another, preferring values from `other` when specified.
    ///
    /// For each field, if `other` specifies a value (is `Some`), use it.
    /// Otherwise, keep the value from `self`.
    pub fn override_with(&mut self, other: &RunOptionsOverride) {
        if let Some(max_depth) = other.max_depth {
            self.max_depth = max_depth;
        }
        if let Some(max_iterations) = other.max_iterations {
            self.max_iterations = max_iterations;
        }
    }
}

impl Default for RunOptions {
    fn default() -> Self {
        Self {
            max_depth: 1000,
            max_iterations: None, // Unlimited by default
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RunOptionsOverride {
    pub max_depth: Option<usize>,
    pub max_iterations: Option<Option<usize>>,
}
