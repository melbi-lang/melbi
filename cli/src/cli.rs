//! Command-line interface definitions.
//!
//! This module contains only clap struct definitions - no business logic.
//! All command implementations are in the `commands` module.

use clap::{Args, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;

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

    /// Generate shell completions
    Completions(CompletionsArgs),

    /// Debug commands (for development)
    #[command(hide = true)]
    Debug(DebugArgs),
    // Phase 2 commands (remaining):
    // Run(RunArgs),
    // Check(CheckArgs),
    // Fmt(FmtArgs),
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

/// Arguments for the `completions` command.
#[derive(Args, Debug)]
pub struct CompletionsArgs {
    /// Shell to generate completions for
    pub shell: Shell,
}

/// Arguments for the `debug` command.
#[derive(Args, Debug)]
pub struct DebugArgs {
    #[command(subcommand)]
    pub command: DebugCommand,
}

/// Debug subcommands.
#[derive(Subcommand, Debug)]
pub enum DebugCommand {
    /// Print the parsed AST
    Parser(DebugInputArgs),

    /// Print the typed expression
    Analyzer(DebugInputArgs),

    /// Print the compiled bytecode
    Bytecode(DebugInputArgs),
}

/// Arguments for debug subcommands that take an expression.
#[derive(Args, Debug)]
pub struct DebugInputArgs {
    /// Expression to debug
    pub expression: String,

    /// Runtime to use for evaluation (only for bytecode)
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
