# melbi-fmt

Code formatter for the Melbi expression language.

## Overview

`melbi-fmt` provides automatic code formatting for Melbi expressions using [Topiary](https://github.com/tweag/topiary), a Tree-sitter-based formatting engine.

## Features

- **Idempotent** — `format(format(x)) == format(x)`
- **Tree-sitter based** — Uses the official Melbi grammar
- **Configurable** — Custom query rules for Melbi-specific formatting

## Usage

### As a Library

```rust
use melbi_fmt::format;

let formatted = format("1+2*3")?;
assert_eq!(formatted, "1 + 2 * 3");
```

### As a CLI

```bash
# Format a file
melbi-fmt input.melbi

# Format multiple files in parallel
melbi-fmt *.melbi
```

## Formatting Rules

The formatter applies consistent style rules:

- Operators surrounded by spaces: `1 + 2`, not `1+2`
- Trailing commas in multi-line constructs
- Consistent indentation (4 spaces)
- Line breaks for readability in complex expressions

## Custom Queries

Formatting behavior is defined in `topiary-queries/queries/melbi.scm`. See the [Topiary documentation](https://github.com/tweag/topiary) for query syntax.

## Integration

- **VS Code** — Integrated via the Melbi VS Code extension
- **Zed** — Integrated via the Melbi Zed extension
- **CLI** — Available as standalone binary

## Related

- [Topiary](https://github.com/tweag/topiary) — The formatting engine
- [tree-sitter-melbi](https://github.com/melbi-lang/tree-sitter-melbi) — Tree-sitter grammar
