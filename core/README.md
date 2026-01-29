# melbi-core

The core implementation of the Melbi expression language.

## Overview

`melbi-core` provides the complete pipeline for parsing, analyzing, and evaluating Melbi expressions:

```
Source → Parser → AST → Analyzer → Typed AST → Evaluator → Value
```

## Modules

| Module | Purpose |
|--------|---------|
| `parser` | PEST-based parser with Pratt parsing for operators |
| `syntax` | AST definitions and span tracking |
| `analyzer` | Hindley-Milner type inference and checking |
| `types` | Type system (inference, unification, interning) |
| `evaluator` | Tree-walking interpreter |
| `values` | Runtime value representation |
| `casting` | Type conversion rules (`Int↔Float`, `Str↔Bytes`) |
| `compiler` | Compilation pipeline orchestration |
| `api` | Public API types and error handling |
| `diagnostics` | Error formatting and reporting |
| `stdlib` | Built-in functions and packages |
| `vm` | Bytecode VM (post-MVP, not yet implemented) |

## Features

- **`std`** — Enable standard library features (default: off for `no_std` support)
- **`experimental_maps`** — Enable experimental map features

## Usage

```rust
use melbi_core::{compiler, evaluator};

// Parse and type-check an expression
let source = "1 + 2 * 3";
let compiled = compiler::compile(source)?;

// Evaluate to get a result
let result = evaluator::evaluate(&compiled)?;
```

## Key Design Decisions

- **Arena allocation** — Uses `bumpalo` for efficient memory management
- **Type interning** — Types are interned for fast comparison and low memory usage  
- **No runtime errors after type-check** — If it compiles, it runs (modulo resource limits)
- **Wrapping arithmetic** — Integer overflow wraps instead of panicking

## Testing

```bash
cargo test -p melbi-core
```

The crate has 1000+ tests covering parser, analyzer, and evaluator.

## Related Crates

- `melbi-parser` — Standalone parser (re-exports core parser)
- `melbi-types` — Standalone types (re-exports core types)
- `melbi-values` — Standalone values (re-exports core values)
- `melbi-fmt` — Code formatter (Topiary-based)
- `melbi-cli` — Command-line interface
