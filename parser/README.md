# melbi-parser

Standalone parser crate for the Melbi expression language.

## Overview

This crate provides a standalone parser interface that re-exports the parser from `melbi-core`. It's intended for use cases where you only need parsing without the full analysis and evaluation pipeline.

## Status

⚠️ **Note**: This crate is currently in development. The core parser functionality lives in `melbi-core::parser`.

## Usage

For most use cases, prefer using `melbi-core` directly:

```rust
use melbi_core::parser;
use bumpalo::Bump;

let arena = Bump::new();
let ast = parser::parse(&arena, "1 + 2 * 3")?;
```

## Related Crates

- `melbi-core` — Full implementation including parser, analyzer, and evaluator
- `melbi-types` — Type system
- `melbi-values` — Runtime values
