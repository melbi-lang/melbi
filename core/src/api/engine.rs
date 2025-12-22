//! The Melbi compilation engine.

use super::{CompileOptionsOverride, CompiledExpression, EngineOptions, EnvironmentBuilder, Error};
use crate::types::{Type, manager::TypeManager};
use crate::values::builder::Binder;
use crate::values::dynamic::Value;
use crate::{Vec, analyzer, parser};
use bumpalo::Bump;

/// The Melbi compilation and execution engine.
///
/// The engine manages:
/// - Runtime configuration (EngineOptions)
/// - Type system (TypeManager)
/// - Global environment (constants, functions, packages)
///
/// # Lifetimes
///
/// - `'arena`: Lifetime of the arena holding types and environment data.
///   All compiled expressions borrow from this arena.
///
/// # Example
///
/// ```
/// use melbi_core::api::{Engine, EngineOptions};
/// use melbi_core::values::dynamic::Value;
/// use melbi_core::values::builder::Binder;
/// use bumpalo::Bump;
///
/// let arena = Bump::new();
/// let options = EngineOptions::default();
///
/// let engine = Engine::new(options, &arena, |_arena, type_mgr, env| {
///     // Register a constant
///     env.bind("PI", Value::float(type_mgr, std::f64::consts::PI))
/// });
///
/// // Compile an expression
/// let expr = engine.compile(Default::default(), "PI * 2.0", &[]).unwrap();
///
/// // Execute
/// let val_arena = Bump::new();
/// let result = expr.run(Default::default(), &val_arena, &[]).unwrap();
/// assert!((result.as_float().unwrap() - 6.28318).abs() < 0.0001);
/// ```
pub struct Engine<'arena> {
    arena: &'arena Bump,
    type_manager: &'arena TypeManager<'arena>,
    environment: &'arena [(&'arena str, Value<'arena, 'arena>)],
    /// Precomputed globals for analyzer (name, type) pairs
    /// TODO: Switch to TypeScheme when generic functions are supported
    globals_for_analyzer: &'arena [(&'arena str, &'arena Type<'arena>)],
    options: EngineOptions,
}

impl<'arena> Engine<'arena> {
    /// Create a new engine with a custom environment.
    ///
    /// The initialization closure receives:
    /// - `arena`: The arena for allocating environment data
    /// - `type_mgr`: The type builder for creating types
    /// - `env`: The environment builder for registering globals
    ///
    /// The closure must return the modified environment builder.
    ///
    /// # Example
    ///
    /// ```
    /// use melbi_core::api::{Engine, EngineOptions};
    /// use melbi_core::values::dynamic::Value;
    /// use melbi_core::values::builder::Binder;
    /// use bumpalo::Bump;
    ///
    /// let arena = Bump::new();
    /// let options = EngineOptions::default();
    /// let engine = Engine::new(options, &arena, |_arena, type_mgr, env| {
    ///     env.bind("pi", Value::float(type_mgr, std::f64::consts::PI))
    /// });
    /// ```
    pub fn new(
        options: EngineOptions,
        arena: &'arena Bump,
        init: impl FnOnce(
            &'arena Bump,
            &'arena TypeManager<'arena>,
            EnvironmentBuilder<'arena>,
        ) -> EnvironmentBuilder<'arena>,
    ) -> Self {
        // Create type manager
        let type_manager = TypeManager::new(arena);

        // Build environment using the initialization closure
        let env_builder = EnvironmentBuilder::new(arena);
        let env_builder = init(arena, type_manager, env_builder);
        let environment = env_builder
            .build()
            .expect("Environment should build successfully");

        // Precompute globals for analyzer (convert Value to Type)
        // TODO: Switch to TypeScheme when generic functions are supported
        let globals: Vec<(&'arena str, &'arena Type<'arena>)> = environment
            .iter()
            .map(|(name, value)| (*name, value.ty))
            .collect();
        let globals_for_analyzer = arena.alloc_slice_copy(&globals);

        Self {
            arena,
            type_manager,
            environment,
            globals_for_analyzer,
            options,
        }
    }

    /// Access the type manager.
    ///
    /// Useful for creating types when building expressions programmatically.
    pub fn type_manager(&self) -> &'arena TypeManager<'arena> {
        self.type_manager
    }

    /// Access the global environment.
    ///
    /// Returns a sorted slice of (name, value) pairs.
    pub fn environment(&self) -> &[(&'arena str, Value<'arena, 'arena>)] {
        self.environment
    }

    /// Access the engine options.
    pub fn options(&self) -> &EngineOptions {
        &self.options
    }

    /// Compile a Melbi expression.
    ///
    /// # Parameters
    ///
    /// - `options`: Compilation options (use `CompileOptions::default()` for defaults)
    /// - `source`: The source code of the expression
    /// - `params`: Parameters for the expression as (name, type) pairs
    ///
    /// # Returns
    ///
    /// A compiled expression ready for execution, or a compilation error.
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
    ///
    /// // Compile a parameterized expression
    /// let type_mgr = engine.type_manager();
    /// let int_ty = type_mgr.int();
    /// let expr = engine.compile(Default::default(), "x + y", &[("x", int_ty), ("y", int_ty)]).unwrap();
    ///
    /// // Execute with arguments
    /// let val_arena = Bump::new();
    /// let result = expr.run(
    ///     Default::default(),
    ///     &val_arena,
    ///     &[Value::int(type_mgr, 10), Value::int(type_mgr, 32)]).unwrap();
    /// assert_eq!(result.as_int().unwrap(), 42);
    /// ```
    pub fn compile(
        &self,
        options_override: CompileOptionsOverride,
        source: &'arena str,
        params: &[(&'arena str, &'arena Type<'arena>)],
    ) -> Result<CompiledExpression<'arena>, Error> {
        // Merge compilation options (defaults + provided)
        let mut _options = self.options.default_compile_options.clone();
        _options.override_with(&options_override);
        // TODO: Use merged_options when CompileOptions has fields

        // Parse the source
        let parsed = parser::parse(self.arena, source)?;

        // Prepare parameters for analysis - copy to arena
        // Since params is already (&str, &Type), we can just copy the slice directly
        let params_slice = self.arena.alloc_slice_copy(params);

        // Type check the expression using precomputed globals
        let typed_expr = analyzer::analyze(
            self.type_manager,
            self.arena,
            &parsed,
            self.globals_for_analyzer,
            params_slice,
        )?;

        // Create compiled expression with default run options
        Ok(CompiledExpression::new(
            typed_expr,
            self.type_manager,
            params_slice,
            self.environment,
            self.options.default_run_options.clone(),
        ))
    }
}
