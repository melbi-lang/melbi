# Melbi MVP Roadmap

**Last Updated**: January 28, 2025

---

## MVP Status: ✅ COMPLETE

Melbi's MVP (v1.0) is **100% complete**. The language is fully functional with a comprehensive feature set, robust type system, and extensive test coverage.

---

## What's in the MVP

### Core Language Features

| Feature | Status | Notes |
|---------|--------|-------|
| **Literals** | ✅ | Int, Float, Bool, String, Bytes, Format Strings |
| **Operators** | ✅ | Arithmetic, comparison, logical, membership (`in`/`not in`) |
| **Collections** | ✅ | Arrays, Records, Maps (construction and indexing) |
| **Control Flow** | ✅ | If/else, where bindings |
| **Pattern Matching** | ✅ | `match` expressions with exhaustiveness checking |
| **Option Type** | ✅ | `some`/`none` with pattern matching |
| **Functions** | ✅ | Lambdas with closures, function calls |
| **Error Handling** | ✅ | `otherwise` operator for error recovery |
| **Type Casting** | ✅ | `as` operator (Int↔Float, Str↔Bytes) |

### Type System

| Feature | Status | Notes |
|---------|--------|-------|
| **Hindley-Milner Inference** | ✅ | Full type inference without annotations |
| **Parametric Polymorphism** | ✅ | Generic functions and types |
| **Error Effect Tracking** | ✅ | Compile-time tracking of fallible operations |
| **Type Interning** | ✅ | Efficient memory usage |
| **Exhaustiveness Checking** | ✅ | Pattern match coverage verification |

### Runtime

| Feature | Status | Notes |
|---------|--------|-------|
| **Tree-Walking Evaluator** | ✅ | Fast, arena-allocated evaluation |
| **Wrapping Arithmetic** | ✅ | No overflow panics |
| **Stack Depth Limits** | ✅ | Configurable (default 1000) |
| **Bounds Checking** | ✅ | Safe array/map access with errors |

### Tooling

| Tool | Status | Notes |
|------|--------|-------|
| **Parser** | ✅ | PEST-based with Pratt parsing, span tracking |
| **Analyzer** | ✅ | Complete type checking and inference |
| **Formatter** | ✅ | Topiary-based, idempotent |
| **CLI** | ✅ | Debug and evaluation modes |
| **VS Code Extension** | ✅ | Syntax highlighting, LSP integration |
| **Zed Extension** | ✅ | Tree-sitter based |
| **Playground** | ✅ | Web-based WASM playground |

### Quality

| Metric | Value |
|--------|-------|
| **Passing Tests** | 1360+ |
| **Test Coverage** | Parser, Analyzer, Evaluator, Formatter |
| **Benchmarking** | Criterion.rs with CEL comparison |
| **Profiling** | pprof/flamegraph integration |

---

## Post-MVP Roadmap

No specific timelines — features prioritized based on user feedback and contribution interest.

### Near-term Priorities

**Standard Library Expansion**
- String operations (split, join, trim, etc.)
- Math functions (abs, sqrt, trig, etc.)
- Array operations (map, filter, reduce, sort)
- Regex support via FFI

**Developer Experience**
- REPL with history and multi-line editing
- LSP advanced features (completion, rename, find references)
- Improved error messages with suggestions

**Documentation**
- User guide and tutorials
- API documentation (rustdoc)
- Cookbook with common patterns

### Medium-term Ideas

**Performance Optimizations**
- Constant folding on typed AST
- Dead code elimination
- Bytecode VM (2-5x speedup target for arithmetic-heavy workloads)

**Type System Extensions**
- Type classes/traits for constrained polymorphism
- Union types with exhaustive matching
- Numeric safety modes (`@strict` annotations)

**Language Bindings**
- C FFI for native integration
- Python bindings
- JavaScript/WASM improvements

### Long-term Exploration

- Serialization of compiled expressions
- Concurrent evaluation support
- Module system
- Formal verification tooling
- LLVM backend for native codegen

---

## Performance

**Benchmarked against CEL (Common Expression Language)**:

| Benchmark | Melbi | CEL | Result |
|-----------|-------|-----|--------|
| Eval only (800 ops) | 25-26 µs | 25.85 µs | ~Tied |
| Full pipeline (800 ops) | 910 µs | 5.56 ms | **6.1x faster** |

- **Evaluation**: Competitive with CEL's tree-walking interpreter
- **Full pipeline**: Parser + type checker is 4-6x faster than CEL's compiler
- **Throughput**: ~30-50 million simple operations/second

---

## How to Contribute

See `docs/CONTRIBUTING.md` for detailed contribution guidelines.

**Quick start:**
```bash
cargo build && cargo test --workspace
cargo run --bin melbi-cli -- --debug-type "1 + 2"
```

**Good first contributions:**
- Documentation improvements
- Additional test cases
- Standard library functions
- Error message improvements
- Benchmark scenarios

---

## Related Documents

- `docs/melbi-lang-cheat-sheet.md` — Language syntax reference
- `docs/design/` — Design documents for various features
- `docs/CONTRIBUTING.md` — Contribution guidelines
- `CLAUDE.md` — Coding and testing guidelines
