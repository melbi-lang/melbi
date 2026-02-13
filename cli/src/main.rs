//! Melbi CLI - A safe, fast, embeddable expression language.
//!
//! # Architecture
//!
//! Each sub-command is responsible for its own UI (output and error rendering).
//! Sub-commands return `ExitCode` directly - they handle all printing before returning.
//!
//! This design follows the model of tools like `git` and `cargo`, where each
//! sub-command is essentially its own binary with full control over its interface.
//!
//! For testability, test the core logic (parser, analyzer, evaluator) directly,
//! and use integration tests for CLI behavior.

use std::process::ExitCode;

use clap::Parser;
use melbi_cli::{
    cli::{Cli, Command},
    commands, common,
};

fn main() -> ExitCode {
    // Install panic handler for user-friendly crash reporting
    common::panic::install_handler();

    // Initialize logging subscriber
    use tracing_subscriber::{EnvFilter, fmt};

    // Use RUST_LOG environment variable to control log level
    // Default to WARN if not set
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("warn"))
        .unwrap();

    fmt()
        .compact()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_target(false)
        .without_time()
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Eval(args) => commands::eval::run(args, cli.no_color),
        Command::Run(args) => commands::run::run(args, cli.no_color),
        Command::Check(args) => commands::check::run(args, cli.no_color),
        Command::Fmt(args) => commands::fmt::run(args, cli.no_color),
        Command::Repl(args) => commands::repl::run(args, cli.no_color),
        Command::Completions(args) => commands::completions::run(args),
        Command::Bug => commands::bug::run(),
        Command::Debug(args) => commands::debug::run(args, cli.no_color),
    }
}
