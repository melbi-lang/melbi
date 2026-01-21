//! The `eval` command - evaluate an expression.

use bumpalo::Bump;
use melbi::{RenderConfig, render_error_to};
use melbi_core::{
    analyzer::analyze,
    compiler::BytecodeCompiler,
    evaluator::{Evaluator, EvaluatorOptions},
    parser,
    types::{Type, manager::TypeManager},
    values::dynamic::Value,
    vm::VM,
};

use crate::cli::{EvalArgs, Runtime};
use crate::common::{CliResult, engine::build_stdlib};

/// Run the eval command.
pub fn run(args: EvalArgs, no_color: bool) -> CliResult<()> {
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
) -> CliResult<()> {
    let config = RenderConfig {
        color: !no_color,
        filename,
        ..Default::default()
    };
    let render_err = |e: melbi::Error| {
        render_error_to(&e, &mut std::io::stderr(), &config).ok();
    };

    let arena = Bump::new();

    // Parse
    let ast = match parser::parse(&arena, input) {
        Ok(ast) => ast,
        Err(e) => {
            render_err(e.into());
            return Ok(());
        }
    };

    // Type check
    let typed = match analyze(type_manager, &arena, &ast, globals_types, &[]) {
        Ok(typed) => typed,
        Err(e) => {
            render_err(e.into());
            return Ok(());
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
                eprintln!("Bytecode compilation error: {:?}", e);
                return Ok(());
            }
        };

        let result_type = typed.expr.0;
        vm_result = Some(
            VM::execute(&arena, &bytecode).map(|raw| Value::from_raw_unchecked(result_type, raw)),
        );
    }

    // Output results
    match (runtime, eval_result, vm_result) {
        (Runtime::Evaluator, Some(Ok(value)), _) => {
            println!("{:?}", value);
        }
        (Runtime::Evaluator, Some(Err(e)), _) => {
            render_err(e.into());
        }
        (Runtime::Vm, _, Some(Ok(value))) => {
            println!("{:?}", value);
        }
        (Runtime::Vm, _, Some(Err(e))) => {
            render_err(e.into());
        }
        (Runtime::Both, Some(eval_res), Some(vm_res)) => {
            match (eval_res, vm_res) {
                (Ok(eval_val), Ok(vm_val)) => {
                    if eval_val == vm_val {
                        println!("{:?}", eval_val);
                    } else {
                        eprintln!("MISMATCH!");
                        eprintln!("  Evaluator: {:?}", eval_val);
                        eprintln!("  VM:        {:?}", vm_val);
                    }
                }
                (Err(e), Ok(vm_val)) => {
                    eprintln!("MISMATCH!");
                    eprintln!("  Evaluator: error");
                    render_err(e.into());
                    eprintln!("  VM:        {:?}", vm_val);
                }
                (Ok(eval_val), Err(e)) => {
                    eprintln!("MISMATCH!");
                    eprintln!("  Evaluator: {:?}", eval_val);
                    eprintln!("  VM:        error");
                    render_err(e.into());
                }
                (Err(eval_e), Err(vm_e)) => {
                    // Both errored - check if same kind of error
                    if eval_e.kind == vm_e.kind {
                        render_err(eval_e.into());
                    } else {
                        eprintln!("MISMATCH (both errors but different)!");
                        eprintln!("  Evaluator:");
                        render_err(eval_e.into());
                        eprintln!("  VM:");
                        render_err(vm_e.into());
                    }
                }
            }
        }
        _ => unreachable!(),
    }

    Ok(())
}
