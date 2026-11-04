# Contributing to Melbi

Thank you for your interest in contributing to Melbi! This document outlines well-defined, reasonably independent sub-projects that are suitable for external contributors.

## Getting Started

1. **Read the development guide**: See [`CLAUDE.md`](../CLAUDE.md) for building, testing, and development workflow
2. **Understand TDD approach**: We follow strict Test-Driven Development - write tests first, then implement
3. **Check existing issues**: Look for issues tagged `good-first-issue` or `help-wanted`
4. **Ask questions**: Open a discussion or issue if anything is unclear

## Project Opportunities

We've organized contribution opportunities by difficulty level. Each project includes estimated time, required skills, and specific files to modify.

---

## 🟢 Easy Projects (2-10 hours)

Perfect for first-time contributors or getting familiar with the codebase.

### ~~1. Complete Display Implementations for Complex Types~~ ✅ **MOSTLY COMPLETED**

**Description**: ~~Implement `Display` trait for Map and Symbol types, which currently show placeholder addresses.~~ Map Display is complete. Symbol type doesn't have a runtime representation yet (no symbol literals exist), so Symbol Display is N/A.

**Why contribute this**:
- Quick win with immediate visible impact
- Clear specification (follow the Function Display implementation we just completed)
- Improves debugging and error messages
- Good introduction to Melbi's value system

**Skills Required**:
- Basic Rust (traits, pattern matching)
- Understanding of Display vs Debug traits

**Files to Modify**:
- `core/src/values/dynamic.rs` (lines 72-77 for Map, lines 94-98 for Symbol)
- `core/src/values/display_test.rs` (add tests similar to function Display tests)

**Success Criteria**:
- Map displays as `<Map @ 0x...: {key_type => value_type}>`
- Symbol displays with meaningful information
- All tests pass
- Both Display and Debug implementations

**Estimated Time**: 2-4 hours

---

### ~~2. Add Span Tracking to Evaluator Errors~~ ✅ **COMPLETED**

**Description**: ~~Many evaluator errors have `span: None`. Add proper source location tracking for better error messages.~~ Evaluator errors now include proper source spans.

**Why contribute this**:
- Significantly improves developer experience
- Well-defined task (grep for `span: None`)
- Learn Melbi's error handling architecture
- Non-breaking change

**Skills Required**:
- Basic Rust
- Understanding of source spans
- Reading error flow through call stacks

**Files to Modify**:
- `core/src/evaluator/eval.rs` (lines 359, 369, and others with `span: None`)
- `core/src/evaluator/error.rs` (ensure span is propagated)
- Add tests verifying error spans are correct

**Success Criteria**:
- All evaluator errors include source spans where available
- Error messages point to exact location in source
- Tests verify span accuracy

**Estimated Time**: 4-6 hours

---

### 3. Improve CLI with Subcommands

**Description**: Add proper CLI modes: `melbi eval`, `melbi check`, `melbi fmt`, `melbi run <file>`.

**Why contribute this**:
- User-facing improvement with immediate value
- Self-contained (CLI is separate package)
- Existing REPL code provides examples
- Clear requirements from MVP roadmap

**Skills Required**:
- Rust basics
- clap library (already used in the project)
- File I/O

**Files to Modify**:
- `cli/src/main.rs` (currently 215 lines - needs expansion)
- Add integration tests in `cli/tests/`

**Required Subcommands**:
- `melbi eval "1 + 2"` - evaluate expression and print result
- `melbi check script.melbi` - type-check file without running
- `melbi fmt script.melbi` - format file (or `--check` for CI)
- `melbi run script.melbi` - evaluate file and print result
- Keep existing REPL as default with no args

**Success Criteria**:
- All four subcommands work correctly
- Help text is clear and complete
- Error handling is user-friendly
- Integration tests cover each mode

**Estimated Time**: 6-10 hours

---

### 4. Add More Benchmark Scenarios

**Description**: Expand benchmarks beyond arithmetic chains to test arrays, records, where bindings, pattern matching.

**Why contribute this**:
- Helps identify performance bottlenecks
- Low risk (doesn't change functionality)
- Good introduction to Criterion.rs
- Learn Melbi's evaluation system

**Skills Required**:
- Rust basics
- Criterion.rs (examples already exist in the codebase)
- Understanding of Melbi syntax

**Files to Modify**:
- `core/benches/evaluator.rs`
- Potentially add new benchmark files for specific features

**Suggested Benchmarks**:
- Array operations (creation, indexing, iteration)
- Record operations (creation, field access)
- Where bindings (simple vs nested)
- Pattern matching (when available)
- String operations
- Complex nested expressions

**Success Criteria**:
- At least 5 new meaningful benchmark scenarios
- Benchmarks run successfully with `cargo bench`
- Results are reproducible

**Estimated Time**: 4-8 hours

---

## 🟡 Medium Projects (12-30 hours)

Substantial features requiring deeper understanding of Melbi's architecture.

### ~~5. Implement Comparison Operators~~ ✅ **COMPLETED**

**Description**: ~~Add `==`, `!=`, `<`, `>`, `<=`, `>=` operators to parser, analyzer, and evaluator.~~ All comparison operators have been implemented.

**Why contribute this**:
- **Essential for MVP** - marked as critical in roadmap
- Touches all three major systems (great learning opportunity)
- Clear specification from existing arithmetic operators
- High impact - enables actually useful programs

**Skills Required**:
- Rust (intermediate level)
- PEST parser grammar
- Type system basics
- Pattern matching

**Files to Modify**:
1. **Parser** (`parser/src/grammar/expression.pest`):
   - Add comparison operator grammar rules
   - Define precedence (lower than arithmetic, higher than logical)
   - Mirror the change in `tree-sitter/grammar.js`

2. **Parser** (`parser/src/parse/`):
   - Add operator precedence handling
   - Add to Pratt parser

3. **Analyzer** (`core/src/analyzer/analyzer.rs`):
   - Add type checking for comparison operators
   - All comparisons return `Bool`
   - Handle type compatibility (can't compare incompatible types)

4. **Evaluator** (`core/src/evaluator/eval.rs`):
   - Implement evaluation for each operator
   - Handle different value types (Int, Float, String, Bool)

5. **Tests** (`core/tests/` and `core/src/evaluator/eval_test.rs`):
   - Comprehensive test coverage for all operators and types

**Implementation Notes**:
- Look at existing arithmetic operators (`+`, `-`, `*`, `/`) as templates
- Equality (`==`, `!=`) works on all types
- Ordering (`<`, `>`, `<=`, `>=`) works on Int, Float, String (lexicographic)
- Consider: should `3.0 == 3` be true? (type coercion?)
- All comparison operators return `Bool`

**Success Criteria**:
- All six operators parse correctly
- Type checking rejects invalid comparisons
- Evaluation produces correct results
- Comprehensive test coverage
- Formatter handles comparison operators

**Estimated Time**: 12-20 hours

---

### ~~6. Complete Maps Evaluation~~ ✅ **COMPLETED**

**Description**: ~~Analyzer is complete for Maps, but evaluator needs implementation.~~ Maps are now fully implemented in the evaluator.

**Why contribute this**:
- Clear scope (evaluator only - analyzer done)
- High user value (key-value data structures)
- TODOs in code mark exact locations
- Analyzer provides complete specification

**Skills Required**:
- Rust (intermediate level)
- HashMap understanding
- Value hashing implementation
- Arena allocation patterns

**Files to Modify**:
- `core/src/evaluator/eval.rs` - Implement map creation and access
- `core/src/values/dynamic.rs` - Implement Hash for Value, improve Display
- `core/src/values/map.rs` (may need to create) - Map data structure
- Add comprehensive tests

**Implementation Notes**:
- Values need Hash trait implementation for use as map keys
- Consider HashMap vs BTreeMap (determinism for testing)
- Arena allocation for map storage
- Map operations: creation `{k1: v1, k2: v2}`, access `map[key]`
- Need to handle: duplicate keys, missing keys, type checking

**Dependencies**:
- Display implementation for Map (Project #1) helps with debugging

**Success Criteria**:
- Map creation works correctly
- Map access returns correct values
- Missing keys handled appropriately
- All tests pass
- Display shows map contents usefully

**Estimated Time**: 15-25 hours

---

### ~~7. LSP Diagnostics and Go-to-Definition~~ ✅ **MOSTLY COMPLETED**

**Description**: ~~Implement core LSP features: diagnostics (errors/warnings), go-to-definition, hover information.~~ Diagnostics are complete. Go-to-definition and hover may need additional work.

**Why contribute this**:
- Major improvement to developer experience
- Self-contained (LSP package separate from core)
- Structure already exists (764 lines)
- Good for learning LSP protocol

**Skills Required**:
- Rust (intermediate level)
- tower-lsp library
- LSP protocol understanding
- Parser/analyzer integration

**Files to Modify**:
- `lsp/src/document.rs` (TODOs on lines 140, 268, 413, 426)
- `lsp/src/main.rs` - Handler implementations
- `lsp/tests/` - Integration tests

**Features to Implement**:

1. **Diagnostics** ✅ **DONE**:
   - Parse errors with proper ranges
   - Type errors with proper ranges
   - Warnings for unused variables

2. **Go to Definition** (may need work):
   - Jump to variable bindings
   - Jump to function definitions
   - Handle where bindings

3. **Hover Information** (may need work):
   - Show inferred types
   - Show function signatures
   - Show documentation (when available)

4. **Formatting** (already exists):
   - Verify integration works correctly

**Success Criteria**:
- VS Code extension shows errors/warnings
- Go-to-definition works for variables and functions
- Hover shows type information
- All LSP tests pass

**Estimated Time**: 20-30 hours

---

### 8. Property-Based Testing Infrastructure

**Description**: Add proptest or quickcheck-based tests for finding edge cases and ensuring correctness properties.

**Why contribute this**:
- Significantly improves code quality
- Self-contained (new test files)
- Good learning opportunity for property-based testing
- Complements existing unit tests

**Skills Required**:
- Rust (intermediate level)
- proptest or quickcheck library
- Understanding of property-based testing concepts
- Melbi language semantics

**Files to Create**:
- `core/tests/property_tests.rs` (or multiple files by category)
- Integration with existing test infrastructure

**Example Properties to Test**:

1. **Parser Properties**:
   - Parsing is deterministic
   - Format preserves semantics: `parse(format(parse(x))) ≈ parse(x)`
   - Invalid syntax always produces parse error

2. **Type Checker Properties**:
   - Type checking is deterministic
   - Well-typed programs don't produce type errors
   - Type inference is consistent

3. **Evaluator Properties**:
   - Evaluation produces values matching their types
   - Arithmetic properties: commutativity, associativity
   - Array operations preserve length correctly

4. **Formatter Properties**:
   - Idempotency: `format(format(x)) == format(x)`
   - Parsing succeeds: `parse(format(x))` succeeds if `parse(x)` succeeds
   - Semantics preserved: `eval(format(x)) == eval(x)`

**Success Criteria**:
- At least 20 meaningful properties tested
- Tests find edge cases (document any bugs found!)
- Integration with `cargo test`
- CI runs property tests

**Estimated Time**: 20-30 hours

---

## 🔴 Hard Projects (30+ hours)

Complex projects requiring deep understanding of compiler/interpreter architecture.

### 9. Constant Folding Optimization ⭐ **COMPETITIVE ADVANTAGE**

**Description**: Implement compile-time evaluation of constant expressions on the typed AST.

**Why contribute this**:
- **Competitive advantage** - CEL doesn't do this
- Clear performance win (eliminate runtime computation)
- Well-defined optimization pass
- Independent of other features

**Skills Required**:
- Rust (advanced level)
- Compiler optimization techniques
- AST traversal and transformation
- Deep understanding of Melbi's type system
- Arena allocation patterns

**Files to Create/Modify**:
- `core/src/optimizer/` (new module)
- `core/src/optimizer/constant_folding.rs` (main implementation)
- `core/src/analyzer/typed_expr.rs` (may need modifications)
- Integration with compilation pipeline
- Comprehensive tests and benchmarks

**Implementation Approach**:

1. **Operate on TypedExpr** (after type checking):
   - Input: Typed AST from analyzer
   - Output: Optimized AST with constants folded
   - Preserve all type information and spans

2. **Fold Constant Expressions**:
   - Arithmetic: `2 + 3` → `5`
   - Boolean: `true and false` → `false`
   - String: `"hello" + " world"` → `"hello world"`
   - Conditionals: `if true then a else b` → `a`
   - Array/Record creation with constant elements

3. **Preserve Semantics**:
   - Don't fold expressions with side effects (if any)
   - Don't fold expressions that might error at runtime
   - Maintain span information for debugging

4. **Optimization Opportunities**:
   - Dead code elimination (unreachable branches)
   - Constant propagation through where bindings
   - Pre-compute array lengths, record field types

**Success Criteria**:
- Constant expressions are evaluated at compile time
- Semantics are preserved (same results as runtime eval)
- Spans preserved for error reporting
- Benchmarks show performance improvement
- All tests pass

**Estimated Time**: 30-50 hours

---

### ~~10. Public API Implementation~~ ✅ **COMPLETED**

**Description**: ~~Design and implement the three-tier public API: unsafe (C FFI), dynamic (heap-allocated), and static (zero-cost).~~ Public API has been implemented.

**Why contribute this**:
- **Critical for MVP** - enables all real-world Rust usage
- Well-documented (50+ page design doc exists)
- Comprehensive learning opportunity
- High impact on user experience

**Skills Required**:
- Rust (expert level)
- API design expertise
- Lifetime and ownership mastery
- Generic programming and trait design
- FFI and C interop knowledge
- Performance optimization

**Components to Implement**:

1. **Engine and Compilation API** ✅:
   - `Engine` - compilation context
   - `Script` - compiled program
   - Error handling with proper spans

2. **Environment Builder** ✅:
   - FFI function registration
   - Global variable injection
   - Type-safe wrappers

3. **Value Construction/Extraction** ✅:
   - Dynamic API: heap-allocated, safe, ergonomic
   - Static API: zero-cost, compile-time checking
   - Conversion between Rust and Melbi types

4. **Unsafe/C FFI Layer** ✅:
   - Raw pointers and manual memory management
   - C-compatible function signatures
   - Header generation

5. **Error Handling** ✅:
   - Rich error types with context
   - Result types for Rust API
   - Error codes for C API

**Files to Create**:
- `core/src/api/` (new module structure)
- `core/src/api/engine.rs`
- `core/src/api/environment.rs`
- `core/src/api/value.rs`
- `core/src/api/ffi.rs`
- Comprehensive examples in `examples/`
- Integration tests

**Reference**:
- Design doc: `docs/public-api-design.md` (assumed to exist)
- Study existing embedded scripting languages (Lua, Rhai, etc.)

**Success Criteria**:
- Clean, ergonomic Rust API
- Zero-cost static API where possible
- Complete C FFI with header generation
- Comprehensive examples and documentation
- All tests pass
- Benchmarks show performance

**Estimated Time**: 80-120 hours (2-3 months part-time)

---

### 11. Formal Verification Integration (CBMC) ⭐ **UNIQUE DIFFERENTIATOR**

**Description**: Implement optional formal verification using CBMC for safety-critical applications.

**Why contribute this**:
- **Unique differentiator** for Melbi in the embedded scripting space
- Comprehensive design doc exists (50+ pages)
- Interesting research opportunity
- High value for specific domains (automotive, aerospace, finance)
- Can be contributor-led exploration

**Skills Required**:
- Rust (advanced level)
- Formal methods understanding
- CBMC knowledge and experience
- Compiler/translator design
- C code generation
- SMT solvers and verification concepts

**Phases**:

**Phase 1: Foundation** (30-40 hours)
- Directive parsing (`#[verify]`, `#[requires]`, etc.)
- API design for verification annotations
- Integration with parser

**Phase 2: Basic CBMC Integration** (40-60 hours)
- Melbi → C translator for simple expressions
- CBMC invocation and result parsing
- CI/CD integration

**Phase 3: Advanced Features** (40-60 hours)
- Recursion and loops
- Arrays and complex data structures
- Property checking (assertions, invariants)

**Phase 4: Production Hardening** (30-40 hours)
- Performance optimization
- Error handling and diagnostics
- Documentation and examples

**Files to Create**:
- `melbi-verify/` (new package)
- `melbi-verify/src/translator.rs` - Melbi to C translation
- `melbi-verify/src/cbmc.rs` - CBMC integration
- `melbi-verify/src/properties.rs` - Property specification
- Examples in `examples/verification/`

**Reference**:
- Design doc: `docs/formal-verification-plan.md` (assumed to exist)
- CBMC documentation
- Study other verified languages

**Success Criteria**:
- Can verify simple Melbi programs with CBMC
- Properties can be specified and checked
- CI integration demonstrates usage
- Documentation with examples
- Known limitations clearly documented

**Estimated Time**: 150-250 hours (6-9 months part-time)

**Note**: This is a long-term project suitable for academic research, thesis work, or sustained open-source contribution.

---

## 📚 Documentation Projects

### 12. User Documentation and Cookbook

**Description**: Write comprehensive user-facing documentation: getting started guide, language tutorial, and cookbook with practical examples.

**Why contribute this**:
- Critical for adoption and user onboarding
- No code changes required (lower risk)
- Great for technical writers and educators
- Helps identify UX issues in the language

**Skills Required**:
- Technical writing
- Understanding of Melbi syntax and semantics
- Example creation and testing
- Documentation tools (mdbook, rustdoc, etc.)

**Deliverables**:

1. **Getting Started Guide**:
   - Installation instructions
   - First Melbi program
   - Basic syntax overview
   - REPL usage

2. **Language Tutorial**:
   - Types and literals
   - Operators and expressions
   - Where bindings
   - Arrays and records
   - Pattern matching
   - Functions and lambdas
   - Error handling

3. **Language Reference**:
   - Complete syntax specification
   - Type system rules
   - Standard library (when available)
   - Error messages explained

4. **Cookbook**:
   - Common patterns and idioms
   - Data validation examples
   - Configuration processing
   - Integration with Rust
   - Performance tips

5. **Integration Examples**:
   - Embedding in Rust applications
   - FFI function registration
   - Error handling patterns
   - Testing embedded scripts

**Files to Create**:
- `docs/book/` (mdbook structure)
- `examples/` (runnable examples)
- Update `README.md` with links

**Success Criteria**:
- Complete coverage of language features
- All examples tested and working
- Clear, beginner-friendly writing
- Good search and navigation
- Published to GitHub Pages

**Estimated Time**: 40-60 hours

---

## How to Contribute

### 1. Choose a Project

- Pick a project matching your skill level and interests
- Check if anyone else is working on it (open issues/PRs)
- If unsure, open an issue to discuss before starting

### 2. Set Up Development Environment

```bash
# Clone the repository
git clone https://github.com/yourusername/melbi.git
cd melbi

# Build and test
cargo build
cargo test

# See CLAUDE.md for detailed development workflow
```

### 3. Follow TDD (Test-Driven Development)

Melbi follows strict TDD practices:

1. **Write tests first**: Add failing tests that specify the desired behavior
2. **Verify tests fail**: Run tests and confirm they fail for the right reason
3. **Implement feature**: Write minimal code to make tests pass
4. **Verify tests pass**: Run tests and confirm they pass
5. **Refactor**: Improve code while keeping tests green

See `CLAUDE.md` for detailed testing guidelines.

### 4. Create a Branch

```bash
git checkout -b feature/your-feature-name
```

### 5. Make Changes

- Follow existing code style and patterns
- Add comprehensive tests
- Update documentation as needed
- Run tests frequently: `cargo test`
- Use efficient test output: `cargo test 2>&1 | grep "test result:"`

### 6. Submit a Pull Request

- Push your branch to GitHub
- Create a Pull Request with:
  - Clear description of what you changed
  - Why the change is needed
  - How you tested it
  - Link to any related issues
- Be responsive to review feedback

## Getting Help

- **Questions**: Open a GitHub Discussion or Issue
- **Bugs**: Report via GitHub Issues
- **Design discussions**: Open a GitHub Discussion
- **Documentation**: Check `CLAUDE.md` and `docs/` directory

## Code of Conduct

- Be respectful and inclusive
- Provide constructive feedback
- Focus on the code, not the person
- Help others learn and grow

## Review Process

1. Maintainers will review your PR
2. May request changes or ask questions
3. Once approved, PR will be merged
4. Your contribution will be credited

## Recognition

All contributors are recognized in:
- Git commit history
- Release notes
- Contributors list (when established)

---

## Questions?

If you have questions about any of these projects or want to propose a new contribution, please:

1. Check existing documentation (`CLAUDE.md`, `docs/`)
2. Search existing issues and discussions
3. Open a new issue or discussion

Thank you for contributing to Melbi! 🎉
