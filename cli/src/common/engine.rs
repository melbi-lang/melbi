//! Shared engine setup with stdlib.

use bumpalo::Bump;
use melbi_core::stdlib::register_stdlib;
use melbi_core::types::{Type, manager::TypeManager};
use melbi_core::values::dynamic::Value;

/// Build the standard library environment for Melbi evaluation.
///
/// Initializes stdlib modules (Math, String, Array, etc.) and returns
/// type and value bindings for the analyzer and evaluator.
///
/// # Arguments
/// * `arena` - Bump allocator for stdlib values and types
/// * `type_manager` - Type manager for creating and interning types
///
/// # Returns
/// * `globals_types` - Type bindings for `analyze()`
/// * `globals_values` - Value bindings for `Evaluator` or `VM`
///
/// # Panics
/// Panics if environment build fails (line 22-23 `.expect()`).
pub fn build_stdlib<'arena>(
    arena: &'arena Bump,
    type_manager: &'arena TypeManager<'arena>,
) -> (
    &'arena [(&'arena str, &'arena Type<'arena>)],
    &'arena [(&'arena str, Value<'arena, 'arena>)],
) {
    use melbi_core::api::EnvironmentBuilder;
    use melbi_core::values::binder::Binder;

    let env_builder = EnvironmentBuilder::new(arena);
    let env_builder = register_stdlib(arena, type_manager, env_builder);
    let globals_values = env_builder
        .build()
        .expect("Environment build should succeed");

    // Convert to types for analyzer
    let globals_types: Vec<(&'arena str, &'arena Type<'arena>)> = globals_values
        .iter()
        .map(|(name, value)| (*name, value.ty))
        .collect();
    let globals_types = arena.alloc_slice_copy(&globals_types);

    (globals_types, globals_values)
}
