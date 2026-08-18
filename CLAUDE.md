# MELBI
- Melbi is a safe, fast, embeddable expression language.
- The entire program is a single expression.
- Read @docs/melbi-lang-cheat-sheet.md for a syntax reference.

# CODING GUIDELINES
- Avoid `unsafe` or `transmute`.
- Avoid code duplication.
- Avoid abbreviations. Except for well-established ones.

# USEFUL COMMANDS
- `just check -p package`, `just test`, `just test -p <package> --test <test_name>`, `just fmt`, `just clippy`, etc.
- `just --list` for all recipes
- Comprehensive verification: `just verify`
- `RUST_LOG=debug cargo run -q -p melbi-cli -- eval --no-color "1 + 2"` - Evaluates `1 + 2`, enable logging, etc.

# LOGGING / DEBUGGING
- Use crate `tracing` for logging key aspects.
  - `tracing::debug!(var_id = id, binding = %ty, "Binding type variable");`
  - Enable in tests with: `crate::test_utils::init_test_logging();`
  - `cargo test -p melbi-core test_array_type_inference -- --nocapture`
