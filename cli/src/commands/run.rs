//! The `run` command - run a Melbi file.

use bumpalo::Bump;
use melbi_core::types::manager::TypeManager;

use crate::cli::RunArgs;
use crate::common::engine::build_stdlib;
use crate::common::input::read_input;
use crate::common::CliResult;

use super::eval::interpret_input;

/// Run the run command.
pub fn run(args: RunArgs, no_color: bool) -> CliResult<()> {
    let (content, _display_name) = match read_input(&args.file) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };

    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let (globals_types, globals_values) = build_stdlib(&arena, type_manager);

    interpret_input(
        type_manager,
        globals_types,
        globals_values,
        &content,
        args.runtime,
        no_color,
    )
}
