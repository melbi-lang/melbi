//! Compiled Melbi expressions.

use super::{Error, RunOptions, RunOptionsOverride};
use crate::analyzer::typed_expr::TypedExpr;
use crate::evaluator::{Evaluator, EvaluatorOptions};
use crate::types::{Type, manager::TypeManager};
use crate::values::dynamic::Value;
use crate::{Vec, format};
use bumpalo::Bump;

/// A compiled Melbi expression ready for execution.
///
/// Compiled expressions borrow from the Engine's arena and can be executed
/// multiple times with different arguments and value arenas.
///
/// # Example
///
/// ```
/// use melbi_core::api::{Engine, EngineOptions};
/// use melbi_core::values::{builder::Binder, dynamic::Value};
/// use bumpalo::Bump;
///
/// let arena = Bump::new();
/// let engine = Engine::new(EngineOptions::default(), &arena, |_, _, env| env);
/// let type_mgr = engine.type_manager();
/// let int_ty = type_mgr.int();
/// let expr = engine.compile(
///     Default::default(),
///     "x + y",
///     &[("x", int_ty), ("y", int_ty)]
/// ).unwrap();
///
/// // Execute with validation
/// let val_arena = Bump::new();
/// let result = expr.run(
///     Default::default(),
///     &val_arena,
///     &[Value::int(type_mgr, 10), Value::int(type_mgr, 32)],
/// ).unwrap();
/// assert_eq!(result.as_int().unwrap(), 42);
///
/// // Execute without validation (unsafe, but faster)
/// let val_arena2 = Bump::new();
/// let result = unsafe {
///     expr.run_unchecked(
///         Default::default(),
///         &val_arena2,
///         &[Value::int(type_mgr, 10), Value::int(type_mgr, 32)],
///     )
/// }.unwrap();
/// assert_eq!(result.as_int().unwrap(), 42);
/// ```
pub struct CompiledExpression<'arena> {
    /// The type-checked AST
    typed_expr: &'arena TypedExpr<'arena, 'arena>,

    /// Type manager for creating values
    type_manager: &'arena TypeManager<'arena>,

    /// Parameters for validation
    params: &'arena [(&'arena str, &'arena Type<'arena>)],

    /// Global environment for evaluation
    environment: &'arena [(&'arena str, Value<'arena, 'arena>)],

    /// Default run-time options
    default_run_options: RunOptions,
}

impl<'arena> CompiledExpression<'arena> {
    /// Create a new compiled expression.
    ///
    /// This is called internally by Engine::compile().
    pub(crate) fn new(
        typed_expr: &'arena TypedExpr<'arena, 'arena>,
        type_manager: &'arena TypeManager<'arena>,
        params: &'arena [(&'arena str, &'arena Type<'arena>)],
        environment: &'arena [(&'arena str, Value<'arena, 'arena>)],
        default_run_options: RunOptions,
    ) -> Self {
        Self {
            typed_expr,
            type_manager,
            params,
            environment,
            default_run_options,
        }
    }

    /// Execute the expression with runtime validation.
    ///
    /// This is the **safe dynamic API** - it validates:
    /// - Argument count matches parameters
    /// - Argument types match parameter types
    ///
    /// # Parameters
    ///
    /// - `options`: Optional execution options to override defaults
    /// - `arena`: Arena for allocating the result value
    /// - `args`: Argument values (must match parameter types)
    ///
    /// # Returns
    ///
    /// The result value, or a runtime/validation error.
    ///
    /// # Example
    ///
    /// ```
    /// use melbi_core::api::{Engine, EngineOptions, RunOptionsOverride};
    /// use melbi_core::values::{builder::Binder, dynamic::Value};
    /// use bumpalo::Bump;
    ///
    /// let arena = Bump::new();
    /// let engine = Engine::new(EngineOptions::default(), &arena, |_, _, env| env);
    /// let type_mgr = engine.type_manager();
    /// let int_ty = type_mgr.int();
    /// let expr = engine.compile(
    ///     Default::default(),
    ///     "x + y",
    ///     &[("x", int_ty), ("y", int_ty)]
    /// ).unwrap();
    ///
    /// // Use default execution options
    /// let val_arena = Bump::new();
    /// let result = expr.run(
    ///     Default::default(),
    ///     &val_arena,
    ///     &[Value::int(type_mgr, 10), Value::int(type_mgr, 32)],
    /// ).unwrap();
    /// assert_eq!(result.as_int().unwrap(), 42);
    ///
    /// // Override execution options (only specify max_depth)
    /// let custom_opts = RunOptionsOverride { max_depth: Some(500), ..Default::default() };
    /// let val_arena2 = Bump::new();
    /// let result = expr.run(
    ///     custom_opts,
    ///     &val_arena2,
    ///     &[Value::int(type_mgr, 10), Value::int(type_mgr, 32)],
    /// ).unwrap();
    /// assert_eq!(result.as_int().unwrap(), 42);
    /// ```
    pub fn run<'value_arena>(
        &self,
        options_override: RunOptionsOverride,
        arena: &'value_arena Bump,
        args: &[Value<'arena, 'value_arena>],
    ) -> Result<Value<'arena, 'value_arena>, Error> {
        // Validate argument count
        if args.len() != self.params.len() {
            return Err(Error::Api(format!(
                "Argument count mismatch: expected {}, got {}",
                self.params.len(),
                args.len()
            )));
        }

        // Validate argument types using pointer equality (types are interned)
        for (i, (arg, (_param_name, expected_ty))) in
            args.iter().zip(self.params.iter()).enumerate()
        {
            if !core::ptr::eq(arg.ty, *expected_ty) {
                return Err(Error::Api(format!(
                    "Type mismatch for parameter {}: types don't match",
                    i
                )));
            }
        }

        // Execute with validation complete
        unsafe { self.run_unchecked(options_override, arena, args) }
    }

    /// Execute the expression without validation.
    ///
    /// **⚠️ Prefer using `run()` for safety.** This method skips validation and should
    /// only be used when you have already validated arguments or are certain they match
    /// the expected types.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    /// - Argument count matches `self.params().len()`
    /// - Each argument's type matches the corresponding parameter type
    /// - Arguments were created with the same TypeManager as the expression
    ///
    /// Violating these invariants may cause panics or incorrect results.
    ///
    /// # Parameters
    ///
    /// - `arena`: Arena for allocating the result value
    /// - `args`: Argument values (must match parameter types - not checked!)
    /// - `execution_options`: Optional execution options to override defaults
    ///
    /// # Returns
    ///
    /// The result value, or a runtime error (e.g., division by zero, index out of bounds).
    /// Note that even type-checked expressions can fail at runtime due to dynamic errors.
    ///
    /// # Example
    ///
    /// ```
    /// use melbi_core::api::{Engine, EngineOptions};
    /// use melbi_core::values::dynamic::Value;
    /// use bumpalo::Bump;
    ///
    /// let arena = Bump::new();
    /// let engine = Engine::new(EngineOptions::default(), &arena, |_, _, env| env);
    /// let type_mgr = engine.type_manager();
    /// let int_ty = type_mgr.int();
    /// let expr = engine.compile(
    ///     Default::default(),
    ///     "x + y",
    ///     &[("x", int_ty), ("y", int_ty)]
    /// ).unwrap();
    ///
    /// // SAFETY: We know the expression expects (Int, Int) and we're passing (Int, Int)
    /// let val_arena = Bump::new();
    /// let result = unsafe {
    ///     expr.run_unchecked(Default::default(), &val_arena, &[
    ///         Value::int(type_mgr, 10),
    ///         Value::int(type_mgr, 32),
    ///     ])
    /// }.unwrap();
    /// assert_eq!(result.as_int().unwrap(), 42);
    /// ```
    pub unsafe fn run_unchecked<'value_arena>(
        &self,
        options_override: RunOptionsOverride,
        arena: &'value_arena Bump,
        args: &[Value<'arena, 'value_arena>],
    ) -> Result<Value<'arena, 'value_arena>, Error> {
        // Merge execution options (defaults + provided)
        let mut run_options = self.default_run_options.clone();
        run_options.override_with(&options_override);

        // Create evaluator options from execution options
        // TODO: EvaluatorOptions should use RunOptions directly or provide a From impl
        // Note: EvaluatorOptions currently only supports max_depth
        // When EvaluatorOptions gains more fields, update this conversion
        let evaluator_opts = EvaluatorOptions {
            max_depth: run_options.max_depth,
        };

        // Prepare variables for evaluation (params = args)
        // Copy parameter names into the value arena so lifetimes match
        let mut variables = Vec::new();
        for ((name, _ty), value) in self.params.iter().zip(args.iter()) {
            let name_in_value_arena: &'value_arena str = arena.alloc_str(name);
            variables.push((name_in_value_arena, *value));
        }
        let variables_slice = arena.alloc_slice_copy(&variables);

        let globals: &[(&str, Value<'arena, 'value_arena>)] = self.environment;

        // Evaluate the expression
        // SAFETY: We transmute the expression lifetime to match the evaluator's arena lifetime.
        // This is safe because:
        // 1. The expression is only borrowed for the duration of eval()
        // 2. The actual data lives in 'arena which outlives 'value_arena in practice
        // 3. The evaluator doesn't store the expression reference
        let expr_for_eval: &'value_arena TypedExpr<'arena, 'value_arena> =
            unsafe { core::mem::transmute(self.typed_expr) };

        // Create evaluator and execute
        let mut evaluator = Evaluator::new(
            evaluator_opts,
            arena,
            self.type_manager,
            expr_for_eval,
            globals,
            variables_slice,
        );

        // Evaluate and convert errors to public Error type
        evaluator.eval().map_err(Error::from)
    }

    /// Get the expression's parameters.
    ///
    /// Returns a slice of (name, type) pairs.
    pub fn params(&self) -> &[(&'arena str, &'arena Type<'arena>)] {
        self.params
    }

    /// Get the expression's return type.
    pub fn return_type(&self) -> &'arena Type<'arena> {
        self.typed_expr.expr.0
    }
}
