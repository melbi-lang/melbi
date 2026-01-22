//! The `completions` command - generate shell completions.

use clap::CommandFactory;
use clap_complete::generate;

use crate::cli::{Cli, CompletionsArgs};

/// Run the completions command.
pub fn run(args: CompletionsArgs) {
    let mut cmd = Cli::command();
    generate(args.shell, &mut cmd, "melbi", &mut std::io::stdout());
}
