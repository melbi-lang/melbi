# CLI Refactor Master Plan

## Goals

1. Transform `melbi-cli` into a subcommand-based CLI
2. Remove `miette` dependency, use `melbi::render_error` instead
3. Keep UI (clap definitions) separate from implementation
4. One module per command, factor out common code

## Architecture

```
cli/src/
├── main.rs              # Entry point: parse args, dispatch
├── lib.rs               # Re-exports for testing
├── cli.rs               # Clap definitions ONLY (no business logic)
├── commands/            # One module per command
│   ├── mod.rs
│   ├── eval.rs
│   ├── repl/            # REPL with supporting modules
│   │   ├── mod.rs
│   │   ├── highlighter.rs
│   │   └── lexer.rs
│   ├── completions.rs   # Shell completions
│   ├── debug.rs         # Hidden debug subcommands
│   ├── run.rs           # (TODO)
│   ├── check.rs         # (TODO)
│   └── fmt.rs           # (TODO)
└── common/
    ├── mod.rs
    ├── error.rs         # Error rendering, exit codes
    └── engine.rs        # Shared Engine setup with stdlib
```

**NOTE:** Add unit tests and integration tests for CLI commands as appropriate.

## Phase 1 - Architecture Refactor ✅

- [x] Create `cli.rs` with clap definitions (Command enum, args structs)
- [x] Create `commands/` directory structure
- [x] Create `common/` with shared utilities (error, engine)
- [x] Wire up `main.rs` as pure dispatch
- [x] Remove `miette` and `thiserror` dependencies from `melbi-cli`
- [x] Move `highlighter.rs` and `lexer.rs` into `commands/repl/`
- [x] Add integration tests for `eval` command

## Phase 2 - Implement Commands

- [x] `eval EXPR` - evaluate inline expression
- [x] `repl` - interactive REPL
- [x] `completions SHELL` - generate shell completions
- [x] `debug parser|analyzer|bytecode` - hidden debug commands
- [x] `run FILE...` - run files (supports globs)
- [x] `check FILE...` - type-check without running
- [x] `fmt FILE...` - format files (copy topiary logic from `melbi-fmt`, remove that crate later)

## Phase 3 - Enhancements

- [x] `bug` command - open GitHub issues URL with system info
- [ ] `doc SYMBOL` - show documentation (extract from `#[doc]` on FFI functions)
- [x] Panic handler that prompts user to report bugs

## Phase 4 - Major Features (Backlog)

- [ ] `test FILE...` - run tests (see `docs/design/unit-testing.md`)
- [ ] `lsp` - start LSP server (use `melbi-lsp` crate)
- [ ] `compile FILE` - generate bytecode (blocked: needs serialization format)

## Commands Summary

| Command | Description |
|---------|-------------|
| `eval EXPR` | Evaluate an expression |
| `run FILE...` | Run Melbi files |
| `check FILE...` | Type-check files without running |
| `repl` | Start interactive REPL |
| `fmt FILE...` | Format Melbi files |
| `test FILE...` | Run tests |
| `doc SYMBOL` | Show documentation for a symbol |
| `bug` | Report a bug |
| `completions SHELL` | Generate shell completions |
| `debug <stage>` | Debug commands (hidden) |

## Design Decisions

- **`fmt`**: Copy topiary/tree-sitter logic from `melbi-fmt` into `commands/fmt.rs`, delete `melbi-fmt` crate later (it's just a thin wrapper)
- **`lsp`**: Subcommand (not separate binary) for single-binary distribution
- **`debug`**: Hidden from `--help` to reduce noise for users
- **Error handling**: Use `melbi::render_error_to` with ariadne. Remove `miette` from both `melbi-cli` and `melbi-fmt` (including `FormatError` struct)

## Panic Handler with Bug Report

```rust
// common/panic.rs
pub fn install_handler() {
    std::panic::set_hook(Box::new(|info| {
        eprintln!("\n💥 Melbi crashed unexpectedly!\n");
        eprintln!("{}", info);

        // Collect system info
        let version = env!("CARGO_PKG_VERSION");
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;

        eprintln!("\nWould you like to report this bug?");
        eprintln!("Run: melbi bug --panic-info \"{}\"", /* encoded info */);
        eprintln!("Or visit: https://github.com/melbi-lang/melbi/issues/new");
    }));
}
```

---

* Tell the user they will have a chance to see what's being sent.
* Include command line that failed.
* If it's REPL include the expression that failed.
