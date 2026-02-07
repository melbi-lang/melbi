//! The `eval` command - evaluate an expression.

use std::process::ExitCode;
use std::time::{Duration, Instant};

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
use nu_ansi_term::Style;

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
        args.time,
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
    show_time: bool,
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
    let mut eval_duration = None;
    let mut vm_result = None;
    let mut vm_duration = None;

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
        let start = Instant::now();
        eval_result = Some(evaluator.eval());
        eval_duration = Some(start.elapsed());
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
        let start = Instant::now();
        vm_result = Some(
            VM::execute(&arena, &bytecode).map(|raw| Value::from_raw_unchecked(result_type, raw)),
        );
        vm_duration = Some(start.elapsed());
    }

    let exit_code = match runtime {
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
    };

    // Print timing information
    if show_time {
        print_timing(runtime, eval_duration, vm_duration, no_color);
    }

    exit_code
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

/// Format a duration in a human-readable way.
fn format_duration(duration: Duration) -> String {
    let nanos = duration.as_nanos();
    if nanos < 1_000 {
        format!("{}ns", nanos)
    } else if nanos < 1_000_000 {
        format!("{:.2}µs", nanos as f64 / 1_000.0)
    } else if nanos < 1_000_000_000 {
        format!("{:.2}ms", nanos as f64 / 1_000_000.0)
    } else {
        format!("{:.2}s", duration.as_secs_f64())
    }
}

/// Print timing information for the runtimes.
fn print_timing(
    runtime: Runtime,
    eval_duration: Option<Duration>,
    vm_duration: Option<Duration>,
    no_color: bool,
) {
    let dimmed = if no_color {
        Style::new()
    } else {
        Style::new().dimmed()
    };

    match runtime {
        Runtime::Evaluator => {
            if let Some(duration) = eval_duration {
                eprintln!(
                    "{}",
                    dimmed.paint(format!("⏱ Evaluator: {}", format_duration(duration)))
                );
            }
        }
        Runtime::Vm => {
            if let Some(duration) = vm_duration {
                eprintln!(
                    "{}",
                    dimmed.paint(format!("⏱ VM: {}", format_duration(duration)))
                );
            }
        }
        Runtime::Both => {
            if let (Some(eval_dur), Some(vm_dur)) = (eval_duration, vm_duration) {
                let eval_nanos = eval_dur.as_nanos() as f64;
                let vm_nanos = vm_dur.as_nanos() as f64;

                // Calculate how VM compares to Evaluator
                let diff_str = if eval_nanos > 0.0 && vm_nanos > 0.0 {
                    let (ratio, is_faster) = if vm_nanos < eval_nanos {
                        (eval_nanos / vm_nanos, true)
                    } else {
                        (vm_nanos / eval_nanos, false)
                    };

                    let speed_word = if is_faster { "faster" } else { "slower" };

                    if ratio >= 2.0 {
                        // Use multiplier for large differences
                        format!("{:.1}x {}", ratio, speed_word)
                    } else {
                        // Use percentage for small differences
                        let percent = (ratio - 1.0) * 100.0;
                        format!("{:.0}% {}", percent, speed_word)
                    }
                } else {
                    String::new()
                };

                let comparison = if diff_str.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", diff_str)
                };

                eprintln!(
                    "{}",
                    dimmed.paint(format!(
                        "⏱ Evaluator: {} | VM: {}{}",
                        format_duration(eval_dur),
                        format_duration(vm_dur),
                        comparison
                    ))
                );
            }
        }
    }
}
