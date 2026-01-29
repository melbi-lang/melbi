# melbi-macros

Procedural macros for Melbi FFI bindings.

## Overview

This crate provides attribute macros for creating type-safe FFI bindings between Rust and Melbi. Use these macros to expose Rust functions to Melbi code.

## Macros

### `#[melbi_fn]`

Transform a Rust function into a Melbi-callable function:

```rust
use melbi_macros::melbi_fn;

#[melbi_fn]
fn add(a: i64, b: i64) -> i64 {
    a + b
}
```

This generates a struct implementing both `Function` and `AnnotatedFunction` traits.

### `#[melbi_const]`

Mark a function as a package constant:

```rust
use melbi_macros::melbi_const;

#[melbi_const]
fn pi() -> f64 {
    std::f64::consts::PI
}
```

### `#[melbi_package]`

Generate a package builder from a module:

```rust
use melbi_macros::melbi_package;

#[melbi_package]
mod Math {
    #[melbi_const]
    fn PI() -> f64 { std::f64::consts::PI }
    
    #[melbi_fn]
    fn Sin(x: f64) -> f64 { x.sin() }
}
```

## Type Mapping

| Melbi Type | Rust Type |
|------------|-----------|
| `Int` | `i64` |
| `Float` | `f64` |
| `Bool` | `bool` |
| `Str` | `&str` |
| `Bytes` | `&[u8]` |
| `Array[T]` | `&[T]` |
| `Option[T]` | `Option<T>` |

## Error Handling

Functions can return `Result<T, E>` where `E: Into<EvalError>`:

```rust
#[melbi_fn]
fn divide(a: f64, b: f64) -> Result<f64, &'static str> {
    if b == 0.0 {
        Err("division by zero")
    } else {
        Ok(a / b)
    }
}
```

## Usage in Standard Library

See `core/src/stdlib/` for examples of how these macros are used to implement Melbi's standard library.

## Related

- `melbi-core::stdlib` — Standard library using these macros
- `melbi-core::values::function` — Function trait definitions
