//! The `completions` command - generate shell completions.

use std::process::ExitCode;

use clap::CommandFactory;
use clap_complete::generate;

use crate::cli::{Cli, CompletionsArgs};

/// Run the completions command.
pub fn run(args: CompletionsArgs) -> ExitCode {
    let mut cmd = Cli::command();
    generate(args.shell, &mut cmd, "melbi", &mut std::io::stdout());
    ExitCode::SUCCESS
}
