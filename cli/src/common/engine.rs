//! Shared engine setup with stdlib.

use bumpalo::Bump;
use melbi_core::stdlib::register_stdlib;
use melbi_core::types::Type;
use melbi_core::types::manager::TypeManager;
use melbi_core::values::dynamic::Value;

/// Environment containing stdlib types and values.
pub struct StdlibEnv<'arena> {
    pub types: &'arena [(&'arena str, &'arena Type<'arena>)],
    pub values: &'arena [(&'arena str, Value<'arena, 'arena>)],
}

/// Build the standard library environment for Melbi evaluation.
///
/// Initializes stdlib modules (Math, String, Array, etc.) and returns
/// type and value bindings for the analyzer and evaluator.
///
/// # Arguments
/// * `arena` - Bump allocator for stdlib values and types
/// * `type_manager` - Type manager for creating and interning types
///
/// # Panics
/// Panics if environment build fails (line 22-23 `.expect()`).
pub fn build_stdlib<'arena>(
    arena: &'arena Bump,
    type_manager: &'arena TypeManager<'arena>,
) -> StdlibEnv<'arena> {
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

    StdlibEnv {
        types: globals_types,
        values: globals_values,
    }
}
