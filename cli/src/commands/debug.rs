//! The `debug` command - debugging tools for development.

use std::process::ExitCode;

use bumpalo::Bump;
use melbi::{RenderConfig, render_error_to};
use melbi_core::{
    analyzer::analyze,
    compiler::BytecodeCompiler,
    parser,
    types::manager::TypeManager,
};

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
    let (globals_types, _globals_values) = build_stdlib(&arena, type_manager);

    let ast = match parser::parse(&arena, &args.expression) {
        Ok(ast) => ast,
        Err(e) => {
            render_err(e.into(), no_color);
            return ExitCode::FAILURE;
        }
    };

    let typed = match analyze(type_manager, &arena, &ast, globals_types, &[]) {
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
    let (globals_types, globals_values) = build_stdlib(&arena, type_manager);

    let ast = match parser::parse(&arena, &args.expression) {
        Ok(ast) => ast,
        Err(e) => {
            render_err(e.into(), no_color);
            return ExitCode::FAILURE;
        }
    };

    let typed = match analyze(type_manager, &arena, &ast, globals_types, &[]) {
        Ok(typed) => typed,
        Err(e) => {
            render_err(e.into(), no_color);
            return ExitCode::FAILURE;
        }
    };

    let bytecode = match BytecodeCompiler::compile(type_manager, &arena, globals_values, &typed) {
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
