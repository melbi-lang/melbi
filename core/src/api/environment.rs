//! Environment builder for registering global values.

use crate::{
    String, ToString, Vec,
    values::binder::{Binder, Error},
    values::dynamic::Value,
};

use alloc::collections::BTreeMap;
use bumpalo::Bump;

/// Builder for constructing the global environment.
///
/// The environment contains constants, functions, and packages that are
/// globally available to all expressions compiled with the engine.
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
///
/// // EnvironmentBuilder is used inside Engine::new
/// let engine = Engine::new(EngineOptions::default(), &arena, |_arena, type_mgr, env| {
///     // Register constant
///     env.bind("PI", Value::float(type_mgr, std::f64::consts::PI))
/// });
/// ```
pub struct EnvironmentBuilder<'arena> {
    arena: &'arena Bump,
    entries: BTreeMap<&'arena str, Value<'arena, 'arena>>,
    duplicates: Vec<String>,
}

impl<'arena> EnvironmentBuilder<'arena> {
    /// Create a new environment builder.
    pub fn new(arena: &'arena Bump) -> Self {
        Self {
            arena,
            entries: BTreeMap::new(),
            duplicates: Vec::new(),
        }
    }
}

impl<'arena> Binder<'arena, 'arena> for EnvironmentBuilder<'arena> {
    type Output = &'arena [(&'arena str, Value<'arena, 'arena>)];

    /// Register a global value (constant, function, or package).
    ///
    /// The name is interned in the arena. Values are sorted by name at build time
    /// for efficient binary search during compilation and evaluation.
    ///
    /// Returns the builder for chaining. If a duplicate name is encountered,
    /// an error is stored and returned when `build()` is called.
    fn bind(mut self, name: &str, value: Value<'arena, 'arena>) -> Self {
        // Check if name already exists
        if self.entries.contains_key(name) {
            self.duplicates.push(name.to_string());
            return self;
        }

        let name = self.arena.alloc_str(name);
        self.entries.insert(name, value);
        self
    }

    /// Build the final sorted environment slice.
    ///
    /// The resulting slice is sorted by name for efficient binary search
    /// during lookups.
    ///
    /// Returns an error if any registration failed (e.g. duplicates).
    fn build(mut self) -> Result<&'arena [(&'arena str, Value<'arena, 'arena>)], Error> {
        if !self.duplicates.is_empty() {
            return Err(Error::DuplicateBinding(core::mem::take(
                &mut self.duplicates,
            )));
        }

        // Sort by name for efficient binary search during lookup
        Ok(self.arena.alloc_slice_fill_iter(self.entries.into_iter()))
    }
}
