//! The `run` command - run a Melbi file.

use std::process::ExitCode;

use bumpalo::Bump;
use melbi_core::types::manager::TypeManager;

use super::eval::interpret_input;
use crate::cli::RunArgs;
use crate::common::engine::build_stdlib;
use crate::common::input::{read_input, strip_shebang};

/// Run the run command.
#[must_use]
pub fn run(args: RunArgs, no_color: bool) -> ExitCode {
    let (content, display_name) = match read_input(&args.file) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };

    // Strip shebang line if present (e.g., #!/usr/bin/env melbi run)
    // Prefix with newline to preserve line numbers in error messages
    let (shebang, rest) = strip_shebang(&content);
    let content = if shebang.is_some() {
        format!("\n{rest}")
    } else {
        rest.to_string()
    };

    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let env = build_stdlib(&arena, type_manager);

    interpret_input(
        type_manager,
        &env,
        &content,
        Some(&display_name),
        args.runtime,
        no_color,
        args.time,
    )
}
