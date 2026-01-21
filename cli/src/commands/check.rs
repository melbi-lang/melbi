//! The `check` command - type-check Melbi files without running.

use std::path::Path;

use bumpalo::Bump;
use melbi::{RenderConfig, render_error_to};
use melbi_core::{analyzer::analyze, parser, types::manager::TypeManager};

use crate::cli::CheckArgs;
use crate::common::engine::build_stdlib;
use crate::common::input::read_file;
use crate::common::CliResult;

/// Run the check command.
pub fn run(args: CheckArgs, no_color: bool) -> CliResult<()> {
    let mut has_errors = false;

    for file in &args.files {
        if !check_file(file, no_color) {
            has_errors = true;
        }
    }

    if has_errors {
        std::process::exit(1);
    }

    Ok(())
}

/// Check a single file. Returns true if OK, false if errors.
fn check_file(path: &Path, no_color: bool) -> bool {
    let content = match read_file(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {}", e);
            return false;
        }
    };

    let config = RenderConfig {
        color: !no_color,
        ..Default::default()
    };
    let render_err = |e: melbi::Error| {
        render_error_to(&e, &mut std::io::stderr(), &config).ok();
    };

    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let (globals_types, _globals_values) = build_stdlib(&arena, type_manager);

    // Parse
    let ast = match parser::parse(&arena, &content) {
        Ok(ast) => ast,
        Err(e) => {
            render_err(e.into());
            return false;
        }
    };

    // Type check
    if let Err(e) = analyze(type_manager, &arena, &ast, globals_types, &[]) {
        render_err(e.into());
        return false;
    }

    println!("{}: OK", path.display());
    true
}
