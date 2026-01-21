//! Command-line interface definitions.
//!
//! This module contains only clap struct definitions - no business logic.
//! All command implementations are in the `commands` module.

use clap::{Args, Parser, Subcommand, ValueEnum};

/// Melbi - A safe, fast, embeddable expression language
#[derive(Parser, Debug)]
#[command(name = "melbi", version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Disable colored output
    #[arg(long, global = true)]
    pub no_color: bool,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Evaluate an expression
    Eval(EvalArgs),

    /// Start interactive REPL
    Repl(ReplArgs),
    // Phase 2 commands:
    // Run(RunArgs),
    // Check(CheckArgs),
    // Fmt(FmtArgs),
    // Completions(CompletionsArgs),
    // #[command(hide = true)]
    // Debug(DebugArgs),
}

/// Arguments for the `eval` command.
#[derive(Args, Debug)]
pub struct EvalArgs {
    /// Expression to evaluate
    pub expression: String,

    /// Runtime to use for evaluation
    #[arg(long, default_value = "both")]
    pub runtime: Runtime,
}

/// Arguments for the `repl` command.
#[derive(Args, Debug)]
pub struct ReplArgs {
    /// Runtime to use for evaluation
    #[arg(long, default_value = "both")]
    pub runtime: Runtime,
}

/// Runtime to use for evaluation.
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum Runtime {
    /// Tree-walking evaluator
    Evaluator,
    /// Bytecode VM
    Vm,
    /// Run both and compare results
    #[default]
    Both,
}
