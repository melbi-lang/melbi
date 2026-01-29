# melbi-lsp

Language Server Protocol implementation for Melbi.

## Features

| Feature | Status | Notes |
|---------|--------|-------|
| **Diagnostics** | ✅ | Parse and type errors |
| **Hover** | ✅ | Type information on hover |
| **Semantic Tokens** | ✅ | Syntax highlighting data |
| **Formatting** | ✅ | Via `melbi-fmt` |
| **Completion** | 🚧 | Basic (triggered on `.`) |
| **Go to Definition** | ⬜ | Planned |
| **Find References** | ⬜ | Planned |
| **Rename** | ⬜ | Planned |

## Installation

```bash
cargo install --path lsp
```

## Usage

The LSP server is typically started by an editor extension:

```bash
melbi-lsp
```

The server communicates via stdio using the Language Server Protocol.

## Editor Integration

### VS Code

Use the [Melbi VS Code extension](../vscode/), which automatically starts the LSP server.

### Zed

Use the [Melbi Zed extension](../zed/).

### Other Editors

Configure your editor to run `melbi-lsp` as an LSP server for `.melbi` files.

## Architecture

```text
lsp/
├── src/
│   ├── main.rs           # Server entry point
│   ├── lib.rs            # Library exports
│   ├── document.rs       # Document state management
│   ├── semantic_tokens.rs # Semantic token provider
│   └── helpers.rs        # Utility functions
```

## Development

```bash
# Build
cargo build -p melbi-lsp

# Run tests
cargo test -p melbi-lsp

# Run with logging
RUST_LOG=debug cargo run -p melbi-lsp
```

## Protocol

Built on [tower-lsp](https://github.com/ebkalderon/tower-lsp), implementing LSP 3.17.

## Related

- [Language Server Protocol Specification](https://microsoft.github.io/language-server-protocol/)
- [tower-lsp](https://github.com/ebkalderon/tower-lsp) — LSP framework
