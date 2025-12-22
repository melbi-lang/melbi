# MELBI
- Melbi is a safe, fast, embeddable expression language.
- The entire program is a single expression.
- **IMPORTANT**: Read @docs/melbi-lang-cheat-sheet.md for a syntax reference.

# CRITICAL SAFETY GUIDELINES
- **NEVER run without asking first:**
  - `git checkout` - You don't know when the last commit was. This has lost hours of work.
  - `perl` or complex `sed` - You WILL get replacements wrong.
    - Use `sed` only for simple single-line substitutions with `-i.bkp` backups.
  - File deletions, git operations (reset, stash), bulk find/replace
- **If anything goes wrong, STOP immediately:**
  - Don't run more commands to "undo" damage
  - Don't try to restore from memory
  - STOP and ask the user for help

# CODING GUIDELINES
- **Unsafe**: Do not use `unsafe` or `transmute` without asking first.
  - Permission applies only to that specific instance.
  - Document safety invariants thoroughly.
- **Code duplication**: Do not duplicate code!
  - Extract common code into helper functions or modules.
  - Prefer generic functions over multiple specialized ones.
  - Do not duplicate constructors because a new field is added.
  - But also be mindful of a complex API design, too many parameters, etc.
    - Consider factoring out some parameters into a `Options` struct.
    - Consider creating a builder for complex cases.
- **Abbrev., etc.**: Avoid abbreviations in variable or type names (except pretty standardized ones).

# TESTING GUIDELINES
- Think about good test cases covering normal and corner cases.
- Test for success and failure scenarios.
  - Success scenarios should validate the answer.
  - Failure scenarios should validate the expected error kind and other relevant details.
- Write high-level tests **before** implementing the code.
- Use helper functions to reduce code duplication.
- **NEVER REMOVE A FAILING TEST:**
  - If a test fails, do not remove or modify it.
  - Your job is to find failing tests and bugs, and not hide them!
  - Ask the user for help if you think you found a bug or if the test had wrong assumptions.
- When adding tests, add the full suite of tests for the feature being implemented.
  - Even if you're not implementing everything immediately, test for all cases.
  - If the tests compile successfully but fail when running then annotate the with #[ignore = "reason"]
  - If they don't even compile, then comment them out. Be sure to leave a `TODO:` for each test.

# LOGGING / DEBUGGING
- Use crate `tracing` for logging key aspects.
  - `tracing::debug!(var_id = id, binding = %ty, "Binding type variable");`
  - Enable in tests with: `crate::test_utils::init_test_logging();`
  - `cargo test -p melbi-core test_array_type_inference -- --nocapture`

# USEFUL COMMANDS
- `cargo test -p melbi-core` - Don't forget this is a cargo workspace. Use `--workspace` to test all packages.
- `RUST_LOG=debug cargo run -q -p melbi-cli -- --no-color --debug-type "1 + 2"` - Evaluates `1 + 2`, enable logging, etc.
