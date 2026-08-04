//! Benchmark: Function call dispatch methods
//!
//! Compares performance of:
//! 1. Enum with pattern matching + function pointer
//! 2. Trait object with dynamic dispatch
//!
//! Run with: `cargo bench --bench function_dispatch`

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};

// ============================================================================
// Approach 1: Enum + Pattern Matching + Function Pointer
// ============================================================================

enum FunctionData {
    Native(fn(i64, i64) -> i64),
    // Add more variants so Rust can't optimize away the enum
    Closure(Box<dyn Fn(i64, i64) -> i64>),
    Builtin(BuiltinFunction),
}

#[derive(Copy, Clone)]
enum BuiltinFunction {
    Add,
    Sub,
    Mul,
}

impl FunctionData {
    fn call(&self, a: i64, b: i64) -> i64 {
        match self {
            FunctionData::Native(f) => f(a, b),
            FunctionData::Closure(f) => f(a, b),
            FunctionData::Builtin(bf) => match bf {
                BuiltinFunction::Add => a + b,
                BuiltinFunction::Sub => a - b,
                BuiltinFunction::Mul => a * b,
            },
        }
    }
}

// ============================================================================
// Approach 2: Trait Object (Dynamic Dispatch)
// ============================================================================

trait Function {
    fn call(&self, a: i64, b: i64) -> i64;
}

struct NativeFunction<F>
where
    F: Fn(i64, i64) -> i64,
{
    func: F,
}

impl<F> NativeFunction<F>
where
    F: Fn(i64, i64) -> i64,
{
    fn new(f: F) -> Self {
        NativeFunction { func: f }
    }
}

impl<F> Function for NativeFunction<F>
where
    F: Fn(i64, i64) -> i64,
{
    fn call(&self, a: i64, b: i64) -> i64 {
        (self.func)(a, b)
    }
}

// ============================================================================
// Test Functions
// ============================================================================

fn add(a: i64, b: i64) -> i64 {
    a + b
}

fn mul(a: i64, b: i64) -> i64 {
    a * b
}

fn sub(a: i64, b: i64) -> i64 {
    a - b
}

// ============================================================================
// Benchmarks
// ============================================================================

fn bench_enum_dispatch(c: &mut Criterion) {
    let func_native = FunctionData::Native(add);
    c.bench_function("enum_dispatch_native", |b| {
        b.iter(|| {
            let mut sum = 0i64;
            for i in 0i64..1000 {
                sum = func_native.call(black_box(sum), black_box(i));
            }
            sum
        })
    });

    let func_closure = FunctionData::Closure(Box::new(|a, b| a - b));
    c.bench_function("enum_dispatch_closure", |b| {
        b.iter(|| {
            let mut sum = 0i64;
            for i in 0i64..1000 {
                sum = func_closure.call(black_box(sum), black_box(i));
            }
            sum
        })
    });

    // Also benchmark with different variants to show realistic case
    let funcs = vec![
        FunctionData::Native(add),
        FunctionData::Native(mul),
        FunctionData::Native(sub),
        FunctionData::Builtin(BuiltinFunction::Add),
        FunctionData::Builtin(BuiltinFunction::Mul),
        FunctionData::Builtin(BuiltinFunction::Sub),
    ];

    c.bench_function("enum_dispatch_mixed", |b| {
        b.iter(|| {
            let mut sum = 0i64;
            for i in 0i64..1000 {
                let func = &funcs[(i % funcs.len() as i64) as usize];
                sum = func.call(black_box(sum), black_box(i));
            }
            sum
        })
    });
}

fn bench_trait_dispatch(c: &mut Criterion) {
    let func: Box<dyn Function> = Box::new(NativeFunction::new(add));

    c.bench_function("trait_dispatch_native", |b| {
        b.iter(|| {
            let mut sum = 0i64;
            for i in 0i64..1000 {
                sum = func.call(black_box(sum), black_box(i));
            }
            sum
        })
    });

    // Mixed variants
    let funcs: Vec<Box<dyn Function>> = vec![
        Box::new(NativeFunction::new(add)),
        Box::new(NativeFunction::new(mul)),
        Box::new(NativeFunction::new(sub)),
        Box::new(NativeFunction::new(|a, b| a + b)),
    ];

    c.bench_function("trait_dispatch_mixed", |b| {
        b.iter(|| {
            let mut sum = 0i64;
            for i in 0i64..1000 {
                let func = &funcs[(i % funcs.len() as i64) as usize];
                sum = func.call(black_box(sum), black_box(i));
            }
            sum
        })
    });
}

fn bench_direct_call(c: &mut Criterion) {
    // Baseline: direct function call (no indirection)
    c.bench_function("direct_call", |b| {
        b.iter(|| {
            let mut sum = 0i64;
            for i in 0i64..1000 {
                sum = add(black_box(sum), black_box(i));
            }
            sum
        })
    });
}

criterion_group!(
    benches,
    bench_direct_call,
    bench_enum_dispatch,
    bench_trait_dispatch
);
criterion_main!(benches);
