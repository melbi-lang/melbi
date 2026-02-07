//! The `run` command - run a Melbi file.

use std::process::ExitCode;

use bumpalo::Bump;
use melbi_core::types::manager::TypeManager;

use crate::cli::RunArgs;
use crate::common::engine::build_stdlib;
use crate::common::input::{read_input, strip_shebang};

use super::eval::interpret_input;

/// Run the run command.
pub fn run(args: RunArgs, no_color: bool) -> ExitCode {
    let (content, display_name) = match read_input(&args.file) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {}", e);
            return ExitCode::FAILURE;
        }
    };

    // Strip shebang line if present (e.g., #!/usr/bin/env melbi run)
    let (_, content) = strip_shebang(&content);

    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let (globals_types, globals_values) = build_stdlib(&arena, type_manager);

    interpret_input(
        type_manager,
        globals_types,
        globals_values,
        &content,
        Some(&display_name),
        args.runtime,
        no_color,
        args.time,
    )
}
