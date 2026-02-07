//! The `eval` command - evaluate an expression.

use std::process::ExitCode;

use bumpalo::Bump;
use melbi::{RenderConfig, render_error_to};
use melbi_core::{
    analyzer::analyze,
    compiler::BytecodeCompiler,
    evaluator::{Evaluator, EvaluatorOptions, ExecutionError},
    parser,
    types::{Type, manager::TypeManager},
    values::dynamic::Value,
    vm::VM,
};

use crate::cli::{EvalArgs, Runtime};
use crate::common::engine::build_stdlib;

/// Run the eval command.
pub fn run(args: EvalArgs, no_color: bool) -> ExitCode {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let (globals_types, globals_values) = build_stdlib(&arena, type_manager);

    interpret_input(
        type_manager,
        globals_types,
        globals_values,
        &args.expression,
        None, // eval command has no filename
        args.runtime,
        no_color,
    )
}

/// Interpret a single expression and print the result.
pub fn interpret_input<'types>(
    type_manager: &'types TypeManager<'types>,
    globals_types: &[(&'types str, &'types Type<'types>)],
    globals_values: &'types [(&'types str, Value<'types, 'types>)],
    input: &str,
    filename: Option<&str>,
    runtime: Runtime,
    no_color: bool,
) -> ExitCode {
    let render_err = |e: melbi::Error| {
        let config = RenderConfig {
            color: !no_color,
            ..Default::default()
        };
        let e = e.with_filename_opt(filename);
        render_error_to(&e, &mut std::io::stderr(), &config).ok();
    };

    let arena = Bump::new();

    // Parse
    let ast = match parser::parse(&arena, input) {
        Ok(ast) => ast,
        Err(e) => {
            render_err(e.into());
            return ExitCode::FAILURE;
        }
    };

    // Type check
    let typed = match analyze(type_manager, &arena, &ast, globals_types, &[]) {
        Ok(typed) => typed,
        Err(e) => {
            render_err(e.into());
            return ExitCode::FAILURE;
        }
    };

    // Run with selected runtime(s)
    let run_evaluator = matches!(runtime, Runtime::Evaluator | Runtime::Both);
    let run_vm = matches!(runtime, Runtime::Vm | Runtime::Both);

    let mut eval_result = None;
    let mut vm_result = None;

    // Evaluator
    if run_evaluator {
        let mut evaluator = Evaluator::new(
            EvaluatorOptions::default(),
            &arena,
            type_manager,
            &typed,
            globals_values,
            &[],
        );
        eval_result = Some(evaluator.eval());
    }

    // VM
    if run_vm {
        let bytecode = match BytecodeCompiler::compile(type_manager, &arena, globals_values, &typed)
        {
            Ok(code) => code,
            Err(e) => {
                render_err(e.into());
                return ExitCode::FAILURE;
            }
        };

        let result_type = typed.expr.0;
        vm_result = Some(
            VM::execute(&arena, &bytecode).map(|raw| Value::from_raw_unchecked(result_type, raw)),
        );
    }

    match runtime {
        Runtime::Evaluator => {
            output_single_result(eval_result.expect("Evaluator result should exist"), &render_err)
        }
        Runtime::Vm => {
            output_single_result(vm_result.expect("VM result should exist"), &render_err)
        }
        Runtime::Both => output_both_results(
            eval_result.expect("Evaluator result should exist"),
            vm_result.expect("VM result should exist"),
            &render_err,
        ),
    }
}

/// Output a single runtime result.
fn output_single_result(
    result: Result<Value, ExecutionError>,
    render_err: &impl Fn(melbi::Error),
) -> ExitCode {
    match result {
        Ok(value) => {
            println!("{:?}", value);
            ExitCode::SUCCESS
        }
        Err(e) => {
            render_err(e.into());
            ExitCode::FAILURE
        }
    }
}

/// Output results when running both evaluator and VM, checking for mismatches.
fn output_both_results(
    eval_res: Result<Value, ExecutionError>,
    vm_res: Result<Value, ExecutionError>,
    render_err: &impl Fn(melbi::Error),
) -> ExitCode {
    match (eval_res, vm_res) {
        (Ok(eval_val), Ok(vm_val)) => {
            if eval_val == vm_val {
                println!("{:?}", eval_val);
                ExitCode::SUCCESS
            } else {
                eprintln!("MISMATCH!");
                eprintln!("  Evaluator: {:?}", eval_val);
                eprintln!("  VM:        {:?}", vm_val);
                ExitCode::FAILURE
            }
        }
        (Err(e), Ok(vm_val)) => {
            eprintln!("MISMATCH!");
            eprintln!("  Evaluator: error");
            render_err(e.into());
            eprintln!("  VM:        {:?}", vm_val);
            ExitCode::FAILURE
        }
        (Ok(eval_val), Err(e)) => {
            eprintln!("MISMATCH!");
            eprintln!("  Evaluator: {:?}", eval_val);
            eprintln!("  VM:        error");
            render_err(e.into());
            ExitCode::FAILURE
        }
        (Err(eval_e), Err(vm_e)) => {
            if eval_e.kind == vm_e.kind {
                render_err(eval_e.into());
            } else {
                eprintln!("MISMATCH (both errors but different)!");
                eprintln!("  Evaluator:");
                render_err(eval_e.into());
                eprintln!("  VM:");
                render_err(vm_e.into());
            }
            ExitCode::FAILURE
        }
    }
}
