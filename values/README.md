# melbi-values

Runtime value types for the Melbi expression language.

## Overview

This crate defines the core value traits (`Value`, `ValueBuilder`, `ValueView`) used by the Melbi evaluator. The actual implementations live in `melbi-core::values`.

## Value Types

| Type     | Rust Representation   | Notes                             |
| -------- | --------------------- | --------------------------------- |
| `Int`    | `i64`                 | 64-bit signed integer             |
| `Float`  | `f64`                 | 64-bit floating point             |
| `Bool`   | `bool`                | Boolean                           |
| `Str`    | `&str`                | UTF-8 string (arena-allocated)    |
| `Bytes`  | `&[u8]`               | Byte array (arena-allocated)      |
| `Array`  | `&[Value]`            | Homogeneous array                 |
| `Record` | Field map             | Named fields                      |
| `Map`    | Hash map              | Key-value pairs                   |
| `Option` | `Some(Value)` / `None` | Optional value                    |
| `Lambda` | Closure               | Function with captured environment |

## Design

- **Arena-allocated** — Values are allocated in a `bumpalo` arena for efficient memory management
- **Copy semantics** — Values are `Copy` (they're references into the arena)
- **Type-tagged** — Each value carries its type for runtime introspection

## Usage

```rust
use melbi_values::{
    raw::RawValue,
    traits::{Value, ValueBuilder},
};
use melbi_types::{BoxBuilder, ty};

// Implement ValueBuilder for your allocation strategy
// (see melbi-core for HeapBuilder example)

fn create_values_example<VB: ValueBuilder>(builder: &VB, tb: BoxBuilder) {
    // Create typed values using the trait-based interface
    let int_ty = ty!(tb, Int);
    let int_val = Value::new(int_ty, RawValue::new_int(42));

    let bool_ty = ty!(tb, Bool);
    let bool_val = Value::new(bool_ty, RawValue::new_bool(true));

    // Allocate to get a handle managed by the builder
    let handle = int_val.alloc(builder);
}
```

## Related

- `melbi-core::values` — The actual implementation
- `melbi-core::evaluator` — Uses values for evaluation
- `melbi-types` — Type system
