# melbi-cli

Command-line interface for the Melbi expression language.

## Installation

```bash
cargo install --path cli
```

Or run directly:

```bash
cargo run -p melbi-cli -- "1 + 2"
```

## Usage

### Evaluate an Expression

```bash
melbi "1 + 2 * 3"
# Output: 7
```

### Interactive REPL

```bash
melbi
# 🖖 Melbi REPL. Ctrl+D to exit; Ctrl+C to abort entry
#   > 
```

REPL features:
- **Syntax highlighting** — Tree-sitter based
- **History** — Persistent across sessions
- **Multi-line input** — Automatic for incomplete expressions; `Alt+Enter` for manual newlines
- **Auto-indentation** — Smart indent for `{`, dedent for `}`

### Pipe Input

```bash
echo "1 + 2" | melbi
# Output: 3
```

### Debug Modes

```bash
# Show parsed AST
melbi --debug parser "1 + 2"

# Show typed expression  
melbi --debug analyzer "1 + 2"

# Show compiled bytecode (experimental)
melbi --debug bytecode "1 + 2"

# Multiple debug stages
melbi --debug parser,analyzer "1 + 2"
```

### Runtime Selection

```bash
# Tree-walking evaluator only
melbi --runtime evaluator "1 + 2"

# Bytecode VM only (experimental)
melbi --runtime vm "1 + 2"

# Both (default) - compares results
melbi --runtime both "1 + 2"
```

### Options

| Flag | Description |
|------|-------------|
| `--debug <stage>` | Print debug info: `parser`, `analyzer`, `bytecode` |
| `--runtime <rt>` | Runtime: `evaluator`, `vm`, `both` (default) |
| `--no-color` | Disable colored output |

## Examples

```bash
# Basic arithmetic
melbi "2 ^ 10"
# 1024

# String interpolation
melbi 'f"Hello { name }" where { name = "World" }'
# "Hello World"

# Pattern matching
melbi 'some 42 match { some x -> x * 2, none -> 0 }'
# 84

# Array operations
melbi '[1, 2, 3, 4, 5][2]'
# 3
```

## Configuration

History is stored in `~/.config/melbi/history`.

## Environment Variables

| Variable | Description |
|----------|-------------|
| `RUST_LOG` | Logging level (`debug`, `info`, `warn`, `error`) |

## Related

- [Melbi Language Cheat Sheet](../docs/melbi-lang-cheat-sheet.md)
- [melbi-core](../core/) — Core language implementation
