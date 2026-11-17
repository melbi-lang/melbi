# GitHub Copilot Instructions

## Project Architecture

**Melbi** is a safe, fast, embeddable expression language designed for
configuration, scripting, and safe evaluation of untrusted code. The entire
program is a single expression, making it ideal for embedded use cases.

### Core Components
- **`core/`**: PEST parser, AST, types, values, analyzer (type-checker), optimizer, runtime.
- **`fmt/`**: Code formatter using Topiary with custom queries in `topiary-queries/queries/melbi.scm`
- **`lsp/`**: Language server using tower-lsp for IDE integration
- **`cli/`**: Command-line interface (currently minimal)
- **`vscode/` & `zed/`**: Editor extensions for VS Code and Zed editors

### Key Design Principles
- **No runtime errors**: If it compiles, it won't crash
- **Hindley-Milner type inference** with union types and pattern matching
- **Parametric polymorphism** for generic types and functions
- **Error effect tracking**: Types annotated for operations that can fail
- **Arena-based allocation** using bumpalo for fast execution
- **Expression-based**: Everything is an expression, no statements

### Data Flow
```
Source Code → Pest Parser → AST → Type Checker → (Optimizers)* → Bytecode Generator → VM
                     ↓
               Topiary Formatter
                     ↓
                Formatted Code
```

## Critical Developer Workflows

### Testing
- Use `test_case!` macro from `tests/cases/mod.rs` for declarative tests
- Tests support `input`, and expected outputs: `ast`, `formatted`. More can be added.
- Each test file in `tests/` compiles as a separate integration test crate
- Run with `cargo test`

### Code Formatting
- **Topiary-based** formatter with custom Melbi grammar
- See `.github/instructions/fmt.instructions.md` for detailed formatting workflow and query development

### Building
- Standard Cargo workspace: `cargo build`, `cargo test`, `cargo run`
- Dependencies managed via `Cargo.toml` workspace configuration

## Project-Specific Guidelines

### Component-Specific Instructions

When working on specific components, refer to dedicated instruction files in `.github/instructions/`:

- **`fmt.instructions.md`**: When working with code formatting, Topiary queries, or formatter tests. Includes detailed guidelines for idempotency testing, query development, and multi-line formatted expectations.

### Code Quality Standards

**Consistency requirements:**

- Follow Rust idioms and best practices
- Comprehensive documentation, with examples, for the public API
- Examples for all major features

**The public API** is what the `melbi` crate (at the workspace root) exports,
including items it re-exports from other crates. Items that are `pub` in an
internal workspace crate (`melbi-core`, `melbi-values`, `melbi-types`,
`melbi-parser`, …) only organize the workspace and are not public API: the
rules here that mention the public API do not apply to them. Documenting them is
welcome but optional; weigh it against the cost of keeping docs current in code
that changes quickly.

**Review requirements:**

- Changes to or affecting the public API require design review
- Breaking changes to the public API require a migration guide
- Performance-critical code requires benchmarks

**Module boundaries:**

- Clear separation: parser → AST → type system → optimizer →
  compiler/interpreter, etc
- Each module independently testable
- Minimize coupling between modules

### Test-Driven Development (NON-NEGOTIABLE)

**MUST follow strict TDD cycle for all language features.**

Red-Green-Refactor cycle:

1. Write test cases (including expected formatted output for formatter tests)
2. Verify tests fail for the right reasons
3. Implement feature to pass tests
4. Refactor while keeping tests green

**Testing requirements:**

- Parser: Test precedence, associativity, edge cases
- Formatter: Test idempotency (`format(format(x)) == format(x)` MUST hold)
- Type system: Test error messages with provenance tracking
- VM/Interpreter: Test correctness, performance, sandboxing limits
- Tests and code must come together

### Testing Infrastructure

- Use the `test_case!` macro from `tests/cases/mod.rs` for declarative test writing
- The macro supports optional fields: `input`, `formatted`, `ast`, `error`
- Use `indoc!` for readable multi-line string literals in tests
- Each test file in `tests/` is compiled as a separate integration test crate

### Error Handling Patterns

- **Error effect tracking**: Operations that can fail return types annotated with error.
- **Contagious errors**: Errors propagate through expressions automatically
- **`otherwise` operator**: Handles the error case by providing fallback values
- **Pattern matching**: Avoids errors by handling all cases

Example:
```melbi
(10 / x) otherwise 0  // will always return an integer.
```

### Type System Conventions

- **No implicit conversions**: Types must match exactly
- **Union types** for expressing alternatives
- **Pattern matching** with `match` for decomposing unions
- **Parametric polymorphism** (generics) for reusable code

## Integration Points

### Editor Support
- **VS Code**: Extension in `vscode/` using Language Client protocol
- **Zed**: Extension in `zed/` with Tree-sitter grammar integration
- **LSP**: Server in `lsp/` provides syntax highlighting, diagnostics, and formatting

### External Dependencies
- **Tree-sitter grammar**: Separate repository for syntax highlighting
- **Topiary**: External formatter with Melbi-specific query extensions
- **Pest**: Parser generator for the core grammar

## General Workflow

1. **Documentation first, experimentation second**: When encountering unfamiliar tools or libraries, check for reference documentation before trial-and-error debugging
2. **Read the error messages carefully**: Topiary provides detailed error messages with diffs for formatting issues
3. **Test incrementally**: Run tests frequently to catch issues early
4. **Verify formatting idempotency**: Always test that formatting is stable across multiple runs
