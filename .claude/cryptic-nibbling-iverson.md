# Phase 2 Remaining: run, check, fmt Commands

## Overview

Implement the remaining Phase 2 commands for melbi-cli:
- `run FILE...` - Run Melbi files
- `check FILE...` - Type-check without running
- `fmt FILE...` - Format Melbi files

## Files to Create/Modify

### New Files
- `cli/src/commands/run.rs` - Run command
- `cli/src/commands/check.rs` - Check command
- `cli/src/commands/fmt.rs` - Format command
- `cli/src/common/input.rs` - Shared file reading utilities

### Modify
- `cli/src/cli.rs` - Add Run, Check, Fmt to Command enum
- `cli/src/commands/mod.rs` - Add new modules
- `cli/src/main.rs` - Add dispatch for new commands
- `cli/Cargo.toml` - Add dependencies for fmt (topiary-core, topiary-tree-sitter-facade)

---

## 1. Add CLI Definitions (`cli/src/cli.rs`)

Add to `Command` enum:
```rust
/// Run Melbi file(s)
Run(RunArgs),

/// Type-check file(s) without running
Check(CheckArgs),

/// Format Melbi file(s)
Fmt(FmtArgs),
```

Add args structs:
```rust
#[derive(Args, Debug)]
pub struct RunArgs {
    /// Files to run
    #[arg(required = true)]
    pub files: Vec<PathBuf>,

    /// Runtime to use for evaluation
    #[arg(long, default_value = "both")]
    pub runtime: Runtime,
}

#[derive(Args, Debug)]
pub struct CheckArgs {
    /// Files to type-check
    #[arg(required = true)]
    pub files: Vec<PathBuf>,
}

#[derive(Args, Debug)]
pub struct FmtArgs {
    /// Files to format
    #[arg(required = true)]
    pub files: Vec<PathBuf>,

    /// Write formatted output back to files (default: print to stdout)
    #[arg(long, short)]
    pub write: bool,

    /// Check if files are formatted (exit 1 if not)
    #[arg(long)]
    pub check: bool,
}
```

---

## 2. Create Shared Input Utilities (`cli/src/common/input.rs`)

```rust
use std::fs;
use std::path::Path;

/// Read a file's contents, returning a user-friendly error on failure.
pub fn read_file(path: &Path) -> Result<String, String> {
    fs::read_to_string(path)
        .map_err(|e| format!("Failed to read '{}': {}", path.display(), e))
}
```

Update `cli/src/common/mod.rs`:
```rust
pub mod engine;
pub mod error;
pub mod input;
```

---

## 3. Implement `run` Command (`cli/src/commands/run.rs`)

```rust
//! The `run` command - run Melbi file(s).

use std::path::PathBuf;
use crate::cli::{RunArgs, Runtime};
use crate::common::{CliResult, input::read_file};
use super::eval::interpret_input;
// ... (use build_stdlib, TypeManager, Bump)

pub fn run(args: RunArgs, no_color: bool) -> CliResult<()> {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let (globals_types, globals_values) = build_stdlib(&arena, type_manager);

    for file in &args.files {
        run_file(file, type_manager, globals_types, globals_values, args.runtime, no_color)?;
    }
    Ok(())
}

fn run_file(
    path: &PathBuf,
    type_manager: &TypeManager,
    globals_types: &[...],
    globals_values: &[...],
    runtime: Runtime,
    no_color: bool,
) -> CliResult<()> {
    let content = read_file(path).map_err(|e| {
        eprintln!("{}", e);
        // Return Ok to continue with other files, or could return error
    })?;

    // Reuse interpret_input from eval
    interpret_input(type_manager, globals_types, globals_values, &content, runtime, no_color)
}
```

**Note:** Consider whether to stop on first error or continue processing files. Recommend: print errors but continue, return non-zero exit if any failed.

---

## 4. Implement `check` Command (`cli/src/commands/check.rs`)

Similar to `run` but stops after type-checking (no evaluation):

```rust
//! The `check` command - type-check Melbi file(s).

use bumpalo::Bump;
use melbi::{RenderConfig, render_error_to};
use melbi_core::{analyzer::analyze, parser, types::manager::TypeManager};

use crate::cli::CheckArgs;
use crate::common::{CliResult, engine::build_stdlib, input::read_file};

pub fn run(args: CheckArgs, no_color: bool) -> CliResult<()> {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let (globals_types, _globals_values) = build_stdlib(&arena, type_manager);

    let mut had_errors = false;

    for file in &args.files {
        if let Err(()) = check_file(&file, type_manager, globals_types, no_color) {
            had_errors = true;
        }
    }

    if had_errors {
        std::process::exit(1);
    }
    Ok(())
}

fn check_file(
    path: &std::path::Path,
    type_manager: &TypeManager,
    globals_types: &[(&str, &melbi_core::types::Type)],
    no_color: bool,
) -> Result<(), ()> {
    let content = match read_file(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", e);
            return Err(());
        }
    };

    let arena = Bump::new();
    let config = RenderConfig { color: !no_color, ..Default::default() };

    // Parse
    let ast = match parser::parse(&arena, &content) {
        Ok(ast) => ast,
        Err(e) => {
            render_error_to(&e.into(), &mut std::io::stderr(), &config).ok();
            return Err(());
        }
    };

    // Type check
    match analyze(type_manager, &arena, &ast, globals_types, &[]) {
        Ok(_) => {
            println!("{}: OK", path.display());
            Ok(())
        }
        Err(e) => {
            render_error_to(&e.into(), &mut std::io::stderr(), &config).ok();
            Err(())
        }
    }
}
```

---

## 5. Implement `fmt` Command (`cli/src/commands/fmt.rs`)

Copy formatting logic from `fmt/src/lib.rs`, simplified:

### Add dependencies to `cli/Cargo.toml`:
```toml
topiary-core = "0.6.1"
topiary-tree-sitter-facade = "0.6.2"
# tree-sitter-melbi is already a dependency
```

### Implementation:
```rust
//! The `fmt` command - format Melbi file(s).

use std::fs;
use topiary_core::{FormatterError, Operation, TopiaryQuery};

use crate::cli::FmtArgs;
use crate::common::input::read_file;

const QUERY: &str = include_str!("../../../topiary-queries/queries/melbi.scm");

pub fn run(args: FmtArgs, _no_color: bool) -> crate::common::CliResult<()> {
    let mut had_errors = false;

    for file in &args.files {
        match format_file(&file, args.write, args.check) {
            Ok(changed) => {
                if args.check && changed {
                    eprintln!("{}: needs formatting", file.display());
                    had_errors = true;
                }
            }
            Err(e) => {
                eprintln!("{}: {}", file.display(), e);
                had_errors = true;
            }
        }
    }

    if had_errors {
        std::process::exit(1);
    }
    Ok(())
}

fn format_file(path: &std::path::Path, write: bool, check: bool) -> Result<bool, String> {
    let input = read_file(path)?;
    let formatted = format_source(&input)?;

    let changed = input != formatted;

    if check {
        return Ok(changed);
    }

    if write {
        if changed {
            fs::write(path, &formatted)
                .map_err(|e| format!("Failed to write: {}", e))?;
            println!("{}: formatted", path.display());
        }
    } else {
        print!("{}", formatted);
    }

    Ok(changed)
}

fn format_source(input: &str) -> Result<String, String> {
    let grammar = topiary_tree_sitter_facade::Language::from(tree_sitter_melbi::LANGUAGE);

    let query = TopiaryQuery::new(&grammar, QUERY)
        .map_err(|e| format!("Query error: {:?}", e))?;

    let language = topiary_core::Language {
        name: "melbi".to_string(),
        indent: Some("    ".to_string()),
        grammar,
        query,
    };

    let mut output = Vec::new();
    topiary_core::formatter(
        &mut input.as_bytes(),
        &mut output,
        &language,
        Operation::Format {
            skip_idempotence: false,
            tolerate_parsing_errors: false,
        },
    )
    .map_err(|e| match e {
        FormatterError::Parsing { start_line, start_column, .. } => {
            format!("Parse error at line {}, column {}", start_line, start_column)
        }
        FormatterError::Idempotence => "Idempotency check failed".to_string(),
        e => format!("Format error: {:?}", e),
    })?;

    let mut result = String::from_utf8(output)
        .map_err(|e| format!("UTF-8 error: {}", e))?;

    // Match input's trailing newline behavior
    if !input.ends_with('\n') {
        result = result.trim_end().to_string();
    }

    Ok(result)
}
```

---

## 6. Update `cli/src/commands/mod.rs`

```rust
pub mod check;
pub mod completions;
pub mod debug;
pub mod eval;
pub mod fmt;
pub mod repl;
pub mod run;
```

---

## 7. Update `cli/src/main.rs`

Add to match:
```rust
Command::Run(args) => commands::run::run(args, cli.no_color),
Command::Check(args) => commands::check::run(args, cli.no_color),
Command::Fmt(args) => commands::fmt::run(args, cli.no_color),
```

---

## Verification

```bash
# Build
cargo build -p melbi-cli

# Test run
echo '1 + 2' > /tmp/test.melbi
melbi run /tmp/test.melbi
# Expected: 3

# Test check
melbi check /tmp/test.melbi
# Expected: /tmp/test.melbi: OK

echo '1 + true' > /tmp/bad.melbi
melbi check /tmp/bad.melbi
# Expected: type error, exit 1

# Test fmt
echo '1+2' > /tmp/ugly.melbi
melbi fmt /tmp/ugly.melbi
# Expected: 1 + 2

melbi fmt --check /tmp/ugly.melbi
# Expected: needs formatting, exit 1

melbi fmt --write /tmp/ugly.melbi
cat /tmp/ugly.melbi
# Expected: 1 + 2

# All tests pass
cargo t -p melbi-cli
```

---

## Notes

- `run` and `check` share the arena/type_manager setup - could extract to common
- `fmt` doesn't need melbi-core at all, just tree-sitter
- Add `--quiet` flag `check` (no messages on error or success, only status code)
- Add stdin support for `fmt` (`melbi fmt -` reads from stdin)
- Update `render_error` to take a filename as argument. (Also add a few error tests that compare against the output)
