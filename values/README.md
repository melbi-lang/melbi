# melbi-values

Runtime value types for the Melbi expression language.

## Overview

This crate provides the runtime value representation used by the Melbi evaluator. It re-exports value types from `melbi-core::values`.

## Value Types

| Type | Rust Representation | Notes |
|------|---------------------|-------|
| `Int` | `i64` | 64-bit signed integer |
| `Float` | `f64` | 64-bit floating point |
| `Bool` | `bool` | Boolean |
| `Str` | `&str` | UTF-8 string (arena-allocated) |
| `Bytes` | `&[u8]` | Byte array (arena-allocated) |
| `Array` | `&[Value]` | Homogeneous array |
| `Record` | Field map | Named fields |
| `Map` | Hash map | Key-value pairs |
| `Option` | `Some(Value)` / `None` | Optional value |
| `Lambda` | Closure | Function with captured environment |

## Design

- **Arena-allocated** — Values are allocated in a `bumpalo` arena for efficient memory management
- **Copy semantics** — Values are `Copy` (they're references into the arena)
- **Type-tagged** — Each value carries its type for runtime introspection

## Usage

```rust
use melbi_core::values::dynamic::Value;
use melbi_core::types::Type;

// Values are typically created by the evaluator
// Direct construction is primarily for FFI/host functions
```

## Related

- `melbi-core::values` — The actual implementation
- `melbi-core::evaluator` — Uses values for evaluation
- `melbi-types` — Type system
