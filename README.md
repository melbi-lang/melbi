# Melbi

A safe, fast, embeddable expression language.

## Features

- **Type-safe** — Hindley-Milner type inference catches errors at compile time
- **Fast** — Arena-allocated, competitive with CEL for evaluation, 6x faster full pipeline
- **Embeddable** — No runtime errors after type-check, configurable resource limits
- **Expressive** — Pattern matching, lambdas, format strings, rich collections

## Quick Start

```bash
# Clone and build
git clone https://github.com/melbi-lang/melbi.git
cd melbi
cargo build

# Run the REPL
cargo run -p melbi-cli

# Evaluate an expression
cargo run -p melbi-cli -- "1 + 2 * 3"
```

## Language Overview

```melbi
// Literals
42                        // Int
3.14                      // Float  
"hello"                   // String
[1, 2, 3]                 // Array
{ x = 1, y = 2 }          // Record
{ a: 1, b: 2 }            // Map

// Operators
1 + 2 * 3                 // Arithmetic
x == y and a < b          // Comparison & logic
"lo" in "hello"           // Membership

// Pattern matching
value match {
    some x -> x * 2,
    none -> 0,
}

// Lambdas
(x, y) => x + y

// Where bindings
result where {
    x = 1,
    y = 2,
    result = x + y,
}

// Format strings
f"Hello { name }!"

// Error handling
arr[i] otherwise default
```

See [docs/melbi-lang-cheat-sheet.md](docs/melbi-lang-cheat-sheet.md) for the complete syntax reference.

## Project Structure

| Crate | Description |
|-------|-------------|
| [`core/`](core/) | Core implementation (parser, analyzer, evaluator) |
| [`cli/`](cli/) | Command-line interface and REPL |
| [`fmt/`](fmt/) | Code formatter (Topiary-based) |
| [`lsp/`](lsp/) | Language Server Protocol implementation |
| [`macros/`](macros/) | Procedural macros |
| [`parser/`](parser/) | Standalone parser |
| [`types/`](types/) | Type system |
| [`values/`](values/) | Runtime values |
| [`playground/`](playground/) | Web-based playground (WASM) |
| [`vscode/`](vscode/) | VS Code extension |
| [`zed/`](zed/) | Zed extension |

## Testing

```bash
# Run all tests
cargo test --workspace

# Run tests for a specific crate
cargo test -p melbi-core

# Run with failure output only (recommended)
cargo t --failure-output=never
```

## Documentation

- [Language Cheat Sheet](docs/melbi-lang-cheat-sheet.md) — Complete syntax reference
- [MVP Roadmap](docs/mvp-roadmap.md) — Current status and future plans
- [Contributing](docs/CONTRIBUTING.md) — Contribution guidelines
- [Design Documents](docs/design/) — Architecture and design decisions

## Performance

Benchmarked against CEL (Common Expression Language):

| Benchmark | Melbi | CEL | Result |
|-----------|-------|-----|--------|
| Evaluation only | 25-26 µs | 25.85 µs | ~Tied |
| Full pipeline | 910 µs | 5.56 ms | **6.1x faster** |

*Benchmarks run on Apple M2 Pro, macOS 14.x, Rust 1.82. Source: `benches/` directory, January 2025.*

## License

See [LICENSE](LICENSE) for details.

## Contributing

We welcome contributions! See [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md) for guidelines.

Before submitting a PR:
1. Run `cargo test --workspace`
2. Run `cargo fmt --check`
3. Follow the guidelines in [CLAUDE.md](CLAUDE.md)
