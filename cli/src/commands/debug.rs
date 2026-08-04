//! The `debug` command - debugging tools for development.

use std::process::ExitCode;

use bumpalo::Bump;
use melbi::{RenderConfig, render_error_to};
use melbi_core::analyzer::analyze;
use melbi_core::compiler::BytecodeCompiler;
use melbi_core::parser;
use melbi_core::types::manager::TypeManager;

use crate::cli::{DebugArgs, DebugCommand, DebugInputArgs};
use crate::common::engine::build_stdlib;

/// Run the debug command.
pub fn run(args: DebugArgs, no_color: bool) -> ExitCode {
    match args.command {
        DebugCommand::Parser(input) => run_parser(input, no_color),
        DebugCommand::Analyzer(input) => run_analyzer(input, no_color),
        DebugCommand::Bytecode(input) => run_bytecode(input, no_color),
    }
}

fn render_err(e: melbi::Error, no_color: bool) {
    let config = RenderConfig {
        color: !no_color,
        ..Default::default()
    };
    render_error_to(&e, &mut std::io::stderr(), &config).ok();
}

fn run_parser(args: DebugInputArgs, no_color: bool) -> ExitCode {
    let arena = Bump::new();

    let ast = match parser::parse(&arena, &args.expression) {
        Ok(ast) => ast,
        Err(e) => {
            render_err(e.into(), no_color);
            return ExitCode::FAILURE;
        }
    };

    println!("=== Parsed AST ===");
    println!("{:#?}", ast.expr);
    ExitCode::SUCCESS
}

fn run_analyzer(args: DebugInputArgs, no_color: bool) -> ExitCode {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let env = build_stdlib(&arena, type_manager);

    let ast = match parser::parse(&arena, &args.expression) {
        Ok(ast) => ast,
        Err(e) => {
            render_err(e.into(), no_color);
            return ExitCode::FAILURE;
        }
    };

    let typed = match analyze(type_manager, &arena, ast, env.types, &[]) {
        Ok(typed) => typed,
        Err(e) => {
            render_err(e.into(), no_color);
            return ExitCode::FAILURE;
        }
    };

    println!("=== Typed Expression ===");
    println!("{:#?}", typed.expr);
    println!();
    println!("=== Lambda Instantiations ===");
    println!("{:#?}", typed.lambda_instantiations);
    ExitCode::SUCCESS
}

fn run_bytecode(args: DebugInputArgs, no_color: bool) -> ExitCode {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let env = build_stdlib(&arena, type_manager);

    let ast = match parser::parse(&arena, &args.expression) {
        Ok(ast) => ast,
        Err(e) => {
            render_err(e.into(), no_color);
            return ExitCode::FAILURE;
        }
    };

    let typed = match analyze(type_manager, &arena, ast, env.types, &[]) {
        Ok(typed) => typed,
        Err(e) => {
            render_err(e.into(), no_color);
            return ExitCode::FAILURE;
        }
    };

    let bytecode = match BytecodeCompiler::compile(type_manager, &arena, env.values, typed) {
        Ok(code) => code,
        Err(e) => {
            render_err(e.into(), no_color);
            return ExitCode::FAILURE;
        }
    };

    println!("=== Bytecode ===");
    println!("{:#?}", bytecode);
    ExitCode::SUCCESS
}
