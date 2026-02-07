# Melbi MVP Roadmap

**Last Updated**: October 30, 2025

This document provides a comprehensive view of Melbi's development status, what's complete, what remains for MVP (v1.0), and the post-MVP roadmap.

---

## Project Status Overview

**Overall MVP Progress**: ~65% Complete

| Component | Status | Completion | Notes |
|-----------|--------|-----------|-------|
| **Parser** | ✅ Complete | 100% | PEST-based, Pratt parsing, span tracking |
| **Type Checker** | ✅ Complete | 100% | Hindley-Milner inference, union types, error effects |
| **Evaluator (Tree-walker)** | 🚧 In Progress | 61% | Core features done, advanced features in progress |
| **Formatter** | ✅ Complete | 100% | Topiary-based, idempotency verified |
| **Public API** | ✍️ Planned | 0% | Design doc complete, implementation pending |
| **Standard Library** | ⚠️ Minimal | 10% | Basic types only, needs function packages |
| **CLI** | ⚠️ Minimal | 20% | Exists but needs eval mode, REPL |
| **LSP** | ⚠️ Skeleton | 10% | Structure exists, needs full implementation |
| **Editor Extensions** | ✅ Basic | 70% | Syntax highlighting works; advanced features depend on LSP |
| **Documentation** | 🚧 In Progress | 40% | Design docs good, user docs needed |

**Legend**: ✅ Complete | 🚧 In Progress | ✍️ Planned | ⚠️ Needs Work

---

## Core Language Features

### ✅ Complete Features

**Expressions**:
- Constants (Int, Float, Bool, String, Bytes)
- Variables and identifiers
- Arithmetic operators (`+`, `-`, `*`, `/`, `^`)
- Boolean operators (`and`, `or`, `not`)
- Unary operators (`-`, `not`)

- If/else conditionals
- Where bindings (sequential binding semantics)
- Arrays (construction, indexing with bounds checking)
- Records (construction, field access)
- Format strings with interpolation
- Otherwise operator (error recovery)

**Type System**:
- Hindley-Milner type inference
- Parametric polymorphism (generics)
- Error effect tracking
- Type interning for efficiency

**Runtime**:
- Arena-based allocation (bumpalo)
- No runtime errors if type-checked
- Wrapping arithmetic (no overflow panics)
- Stack depth limits (default 1000, configurable)

### 🚧 In Progress

**Currently**: Performance benchmarking and profiling infrastructure

### ✍️ Remaining for MVP

**Critical for v1.0**:
- **Comparison Operators**: `==`, `!=`, `<`, `>`, `<=`, `>=`
  - Parser: ⬜ Not started
  - Analyzer: ⬜ Not started
  - Evaluator: ⬜ Not started
  - Complexity: Low-Medium (parser needs precedence rules, evaluator needs value comparison)
  - **Essential** - Can't do much without comparisons
  
- **Maps**: Key-value data structures
  - Analyzer: ✅ Complete
  - Evaluator: ⬜ Not started
  - Complexity: Medium (requires value hashing, Display implementation)
  - See TODOs in `core/src/values/dynamic.rs`
  
- **Cast Operator**: Type conversions
  - Analyzer: ✅ Complete (effect tracking TODO)
  - Evaluator: ✅ Complete
  - Casting Library: ✅ Complete (`core/src/casting.rs` fully implemented)
  - Complexity: Medium (DONE)
  - Supports: Int↔Float, Str↔Bytes (UTF-8)
  
- **Call/Lambda Evaluation**: Function values with capture
  - Analyzer: ✅ Complete
  - Evaluator: ⬜ Not started
  - Complexity: Medium (closure capture less complex than initially thought)
  - **Optional for MVP** - Can defer to v1.1 if needed

**Will be provided via standard library**:
- String operations (substring, split, join, etc.)
- Byte array operations
- Regex matching
- Math functions (trigonometry, logarithms, etc.)
- Statistical functions

---

## Evaluator Implementation Status

Melbi currently uses a **tree-walking AST interpreter** for evaluation. A bytecode VM is planned for post-MVP.

### Progress Summary

**Reference**: `docs/evaluator-implementation-plan.md`

**Detailed tracking**: See evaluator implementation plan for full task breakdown.

**Summary**:
- Phase 0 (Infrastructure): ✅ 100%
- Phase 1 (Core MVP): ✅ 100% 
- Phase 2 (Advanced): 🚧 ~12%
- Phase 3 (Optimization): ⬜ Not started

**What's missing**:
- Comparison operators (not in parser/analyzer/evaluator yet)
- Maps evaluation (analyzer done, evaluator TODO)
- Call/Lambda evaluation (optional for MVP)
- Span tracking in evaluator errors (mostly TODOs)
- Map/Function/Symbol Display implementations (TODOs in `core/src/values/dynamic.rs`)

### Performance Characteristics

**Benchmarked against CEL (Common Expression Language)**:

| Benchmark | Melbi | CEL | Result |
|-----------|-------|-----|--------|
| Eval only (800 ops) | 25-26 µs | 25.85 µs | Tied / Slightly faster |
| Full pipeline (800 ops) | 910 µs | 5.56 ms | **6.1x faster** |

- **Evaluation**: Melbi is competitive with CEL's tree-walking interpreter (~10-20% faster at small sizes)
- **Full pipeline**: Melbi's parser + type checker is 4-6x faster than CEL's compiler
- **Throughput**: ~30-50 million simple operations/second (tree-walker)

---

## Optimizations (Pre-Bytecode)

These optimizations can be implemented on the typed AST before bytecode generation:

### ✍️ Planned Optimizations

1. **Constant Folding** (High Priority)
   - Evaluate constant expressions at compile-time
   - Replace `1 + 2 + 3` with `6`
   - CEL doesn't do this (competitive advantage)
   - Complexity: Medium
   - Impact: High for expression-heavy workloads

2. **Dead Code Elimination** (Medium Priority)
   - Remove unreachable branches: `if false then ... else ...`
   - Remove unused where bindings
   - Complexity: Low
   - Impact: Medium

3. **Common Subexpression Elimination** (Low Priority)
   - Cache repeated computations within an expression
   - Complexity: High
   - Impact: Low (expressions typically don't repeat subexpressions)

**Why optimize before bytecode?**
- Easier to implement and verify on AST than bytecode
- Provides immediate performance wins
- Optimized AST makes bytecode generation simpler
- Classic compiler architecture: parse → analyze → **optimize** → codegen → VM
- CEL doesn't do constant folding (competitive advantage)

---

## Bytecode VM (Post-MVP)

**Status**: Design phase, not started

**Target**: 2-5x faster than optimized tree-walker for arithmetic-heavy workloads

### Advantages over Tree-Walking

- **Flat instruction array**: Better cache locality
- **Dispatch loop**: Less function call overhead
- **Direct operand encoding**: No AST node traversal
- **Register/stack allocation**: Fewer memory accesses
- **Instruction-level optimization**: Peephole optimization, instruction combining

### Design Decisions Needed

1. **VM Architecture**:
   - Stack-based (simpler, like Python bytecode)
   - Register-based (faster, like Lua)

2. **Instruction Set**:
   - RISC-like (many simple instructions)
   - CISC-like (fewer complex instructions)

3. **Compilation Strategy**:
   - Direct translation from typed AST
   - Or with intermediate representation (IR)

4. **Optimization Pipeline**:
   - Where to apply optimizations (AST, bytecode, or both)

**Priority**: High priority post-MVP, but not blocking v1.0 release

---

## Public API

**Status**: Design complete, implementation pending

**Reference**: `docs/design/public-api.md`

### Required for MVP

**Core API**:
- `Engine` - Compilation and execution environment
- `Engine::compile()` - Parse + analyze expressions
- `Engine::eval()` - Evaluate compiled expressions
- `Environment` - Global constants, functions, packages
- `EngineOptions` - Runtime configuration (stack depth, limits)

**Value API**:
- `Value` construction (from Rust types)
- `Value` extraction (to Rust types)
- Type-safe wrappers for common types

**Error Handling**:
- Compilation errors (parse, type-check)
- Runtime errors (division by zero, index out of bounds)
- Error reporting with spans and suggestions

**FFI System** (for host functions - **Critical for MVP**):
- Register Rust functions callable from Melbi
- Automatic type conversion
- Support for generic functions
- Error propagation
- **Must work well** - Standard library depends on FFI

### API Design Goals

1. **Ergonomic for Rust users**: Zero-cost abstractions, builder patterns
2. **Safe by default**: Type-checked where possible, clear error messages
3. **FFI-friendly**: Can be exposed to C, Python, JavaScript
4. **Flexible**: Support both dynamic and static typing patterns
5. **Performant**: Minimal overhead, arena-managed memory

**Complexity**: High (API design is critical and affects everything)

**Priority**: **Critical for MVP** - needed for any real-world usage

---

## Tooling & Developer Experience

### Parser

**Status**: ✅ Complete

- PEST-based grammar (`core/src/parser/expression.pest`)
- Pratt parsing for operator precedence
- Comprehensive span tracking for error messages
- Stack overflow protection (depth limit: 1000)
- 378 passing parser tests

### Type Checker (Analyzer)

**Status**: ✅ Complete

- Hindley-Milner type inference
- Parametric polymorphism (generics)
- Union types with pattern matching
- Error effect tracking
- Type interning for performance
- Rich error messages with spans

### Formatter

**Status**: ✅ Complete

- Topiary-based with custom query rules
- Idempotency guaranteed: `format(format(x)) == format(x)`
- Integrated into editor extensions
- Custom formatting rules in `topiary-queries/queries/melbi.scm`

### Language Server Protocol (LSP)

**Status**: ⚠️ Skeleton exists, needs implementation

**Current**:
- Project structure in `lsp/`
- tower-lsp integration

**Needed for MVP**:
- Syntax highlighting (delegate to Tree-sitter)
- Diagnostics (parse and type errors)
- Formatting (delegate to Topiary)
- Go to definition
- Hover information (show types)

**Optional for v1.0** (defer to v1.1):
- Auto-completion
- Rename refactoring
- Find references

### CLI

**Status**: ⚠️ Minimal, needs work

**Current**:
- Basic binary exists in `cli/`
- Can be invoked with `cargo run --bin melbi`

**Needed for MVP**:
- Eval mode: `melbi eval "1 + 2"`
- File evaluation: `melbi run script.melbi`
- REPL: `melbi repl`
- Type-check mode: `melbi check script.melbi`
- Format mode: `melbi fmt script.melbi`

### Editor Extensions

**VS Code** (`vscode/`):
- ✅ Syntax highlighting
- ✅ Language server integration
- ⬜ Snippets
- ⬜ Debugger integration (far future)

**Zed** (`zed/`):
- ✅ Syntax highlighting via Tree-sitter
- ✅ Basic language support
- ✅ Formatting integration

### Debugging Tools

**Available**:
- `parser_debug` - Parse and type-check expressions from command line
- Topiary CLI - Format code and debug formatting issues
- Criterion benchmarks - Performance measurement
- Flamegraph profiling - Performance analysis with pprof

---

## Testing & Quality Assurance

### Test Infrastructure

**Framework**: Custom `test_case!` macro (see `tests/cases/mod.rs`)

Supports declarative testing:
```rust
test_case!(
    name: test_arithmetic,
    input: "1 + 2",
    formatted: "1 + 2",
    error: None,
);
```

### Test Coverage

**As of October 2025**:
- **543 passing tests** (2 ignored)
- Parser tests: ~150 tests
- Analyzer tests: ~150 tests
- Evaluator tests: 134 tests
- Integration tests: ~100+ tests

**Coverage by component**:
- Parser: ✅ Comprehensive
- Type checker: ✅ Comprehensive
- Evaluator: 🚧 Good coverage for Phase 1, partial for Phase 2
- Formatter: ✅ Comprehensive (idempotency verified)

### Benchmarking

**Infrastructure**:
- Criterion.rs for statistical benchmarking
- pprof integration for flamegraph profiling
- CEL comparison benchmarks

**Location**: `core/benches/evaluator.rs`

**Run benchmarks**:
```bash
cd core/
cargo bench --bench evaluator

# With profiling
cargo bench --bench evaluator -- --profile-time=5 eval_only/800

# Flamegraph output
open ../target/criterion/eval_only/800/profile/flamegraph.svg
```

---

## MVP Definition (v1.0)

### Must Have

**Language Features**:
- ✅ All core expressions (arithmetic, boolean, if/else, where, arrays, records, format strings, otherwise)
- ✅ Cast operator (Int↔Float, Str↔Bytes)
- ⬜ **Comparison operators** (`==`, `!=`, `<`, `>`, `<=`, `>=`) - **Essential**
- ⬜ **Maps** (construction and indexing)
- ⬜ Call/Lambda evaluation (optional - can defer to v1.1)

**Runtime**:
- ✅ Tree-walking evaluator (core features complete)
- ✅ No runtime errors if type-checked
- ✅ Configurable stack depth limits
- ⬜ Standard library basics (Math, String, Bytes, Stats, Regex via FFI)

**Public API** - **Critical**:
- ⬜ Engine and compilation API
- ⬜ Environment/globals management
- ⬜ Value construction/extraction
- ⬜ **FFI for host functions** (enables standard library)
- ⬜ Comprehensive error handling

**Developer Tools**:
- ✅ Parser and type checker
- ✅ Code formatter
- ⬜ CLI with eval/run/check/fmt modes
- ⬜ LSP with basic features (diagnostics, formatting, go-to-def, hover)
- ✅ Editor extensions (syntax highlighting and formatting work)

**Quality & Documentation**:
- ⬜ Test coverage >80% for all components
- ✅ Benchmarking infrastructure (Criterion + profiling)
- ⬜ User documentation (getting started, language guide, cookbook)
- ⬜ API documentation (rustdoc)
- ⬜ Examples for common use cases

### Nice to Have (may defer to v1.1+)

- REPL with multi-line editing and history
- LSP advanced features (completion, rename, find references)
- Constant folding optimization (high value, but not blocking)
- Advanced CLI features (watch mode, multi-file evaluation)
- Package system for user-contributed libraries

---

## Post-MVP Roadmap

**Note**: No specific timelines - features will be prioritized based on user feedback and contribution interest.

### v1.1 - Usability & Ecosystem

**Focus**: Make Melbi more pleasant to use

- Call/Lambda evaluation (if not in v1.0)
- REPL with multi-line editing and history
- LSP advanced features (completion, rename, find references)
- Improved error messages with suggestions and "did you mean?"
- More comprehensive standard library functions
- User guide, cookbook, and examples
- Language bindings begin (C FFI, Python)

### v1.x - Performance Optimizations

**Focus**: Speed improvements (high priority, flexible timing)

- Constant folding on typed AST
- Dead code elimination
- **Bytecode VM** design and implementation
- Compiler from typed AST to bytecode
- Instruction-level optimizations
- Performance benchmarks vs CEL, Lua, QuickJS
- JIT compilation exploration (stretch goal)

### v1.x - Advanced Type System

**Focus**: Richer type features (version TBD, ~v1.4+)

- Union types implementation
- Pattern matching on union types
- Exhaustiveness checking
- Type aliases
- Nominal types (newtype pattern)

### v2.0 - Production Ready

**Focus**: Robustness and production deployment

- Advanced sandboxing (timeouts, memory limits, resource quotas)
- Serialization of compiled expressions (caching)
- Module system implementation
- Concurrent evaluation support (thread-safety)
- Security audit and fuzzing
- Production case studies and testimonials
- Comprehensive language bindings (C, Python, JavaScript, Go)

### Future Exploration

**Ideas under consideration**:
- Async/await support for IO-bound workloads
- Streaming evaluation for large datasets
- GPU acceleration for parallel computation
- Language server debugging protocol (step-through debugging)
- Package registry for user-contributed libraries
- **Formal verification tooling** (high interest, contributor-friendly)
- Compilation to WebAssembly
- LLVM backend for native code generation

---

## How to Contribute

Melbi welcomes contributors! Whether you're interested in implementation, testing, documentation, or research.

### Getting Started

1. **Build and explore**: `cargo build && cargo test`
2. **Try the tools**: `cargo run --bin parser_debug -- "1 + 2"`
3. **Read the design docs**: Check `docs/design/` for architecture decisions
4. **Look for TODOs**: `grep -r "TODO\|todo!\|unimplemented!" --include="*.rs"` to find specific missing pieces

### Ways to Contribute

**Implementation**:
- **Comparison operators**: Add parser rules, analyzer support, evaluator implementation (good first contribution)
- **Maps evaluation**: Complete the evaluator for Maps (see TODOs in `core/src/values/dynamic.rs`)
- **Casting library**: Implement type conversion rules (`core/src/casting.rs` is currently a stub)
- **Standard library functions**: Implement Math, String, Regex packages via FFI
- **CLI improvements**: Add eval/run/check modes
- **LSP features**: Implement diagnostics, go-to-definition, hover

**Research & Design** (great for contributors with specific interests):
- **Formal verification**: Design verification tooling for Melbi expressions
- **Bytecode VM**: Research instruction set design, register vs stack architecture
- **Optimization**: Design and implement constant folding, dead code elimination
- **Type system extensions**: Union types, pattern matching, exhaustiveness checking

**Quality & Documentation**:
- **Write tests**: Expand test coverage for edge cases
- **Benchmarking**: Add more realistic benchmarks beyond arithmetic chains
- **User documentation**: Getting started guide, language tutorial, cookbook
- **API examples**: Show common integration patterns

**Process**:
1. **Discuss first**: Open an issue or discussion for non-trivial changes
2. **Small PRs**: Break large features into reviewable chunks
3. **Follow TDD**: Write tests first, ensure they fail, implement, verify
4. **Document**: Update relevant design docs and roadmap

### Good First Contributions

- Add comparison operators (self-contained, touches all layers)
- Implement missing Display for Map/Function/Symbol types
- Add span tracking to evaluator errors (improve error messages)
- Write property-based tests using proptest/quickcheck
- Improve parser error messages with suggestions
- Add more benchmark scenarios (arrays, records, where bindings)

---

## Questions & Discussion

For questions about the roadmap or to suggest changes:

- Open an issue on GitHub
- Join discussions in the project repository
- Review design documents in `docs/design/`

**Last Updated**: October 30, 2025
