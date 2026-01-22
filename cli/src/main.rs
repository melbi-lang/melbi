//! Melbi CLI - A safe, fast, embeddable expression language.

mod cli;
mod commands;
mod common;

use clap::Parser;
use cli::{Cli, Command};

fn main() {
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

    let result = match cli.command {
        Command::Eval(args) => commands::eval::run(args, cli.no_color),
        Command::Run(args) => commands::run::run(args, cli.no_color),
        Command::Check(args) => commands::check::run(args, cli.no_color),
        Command::Fmt(args) => commands::fmt::run(args, cli.no_color),
        Command::Repl(args) => commands::repl::run(args, cli.no_color),
        Command::Completions(args) => {
            commands::completions::run(args);
            Ok(())
        }
        Command::Bug => {
            commands::bug::run();
            Ok(())
        }
        Command::Debug(args) => {
            commands::debug::run(args, cli.no_color);
            Ok(())
        }
    };

    if let Err(e) = result {
        common::error::render_and_exit(e, cli.no_color);
    }
}
