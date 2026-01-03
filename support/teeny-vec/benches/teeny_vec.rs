//! Benchmarks for TeenyVec vs SmallVec vs Vec
//!
//! Run with: `cargo bench --bench teeny_vec`

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use melbi_teeny_vec::TeenyVec;
use smallvec::SmallVec;

fn bench_push_small(c: &mut Criterion) {
    let mut group = c.benchmark_group("push_small_inline");

    // Small sizes that fit inline (14 bytes for TeenyVec)
    for size in [1, 4, 8, 12, 14] {
        group.bench_with_input(BenchmarkId::new("TeenyVec", size), &size, |b, &size| {
            b.iter(|| {
                let mut vec = TeenyVec::new();
                for i in 0..size {
                    vec.push(black_box(i as u8));
                }
                black_box(vec);
            });
        });

        group.bench_with_input(BenchmarkId::new("SmallVec<16>", size), &size, |b, &size| {
            b.iter(|| {
                let mut vec = SmallVec::<[u8; 16]>::new();
                for i in 0..size {
                    vec.push(black_box(i as u8));
                }
                black_box(vec);
            });
        });

        group.bench_with_input(BenchmarkId::new("Vec", size), &size, |b, &size| {
            b.iter(|| {
                let mut vec = Vec::<u8>::new();
                for i in 0..size {
                    vec.push(black_box(i as u8));
                }
                black_box(vec);
            });
        });
    }

    group.finish();
}

fn bench_push_medium(c: &mut Criterion) {
    let mut group = c.benchmark_group("push_medium_heap");

    // Medium sizes that require heap allocation
    for size in [20, 32, 64, 128] {
        group.bench_with_input(BenchmarkId::new("TeenyVec", size), &size, |b, &size| {
            b.iter(|| {
                let mut vec = TeenyVec::new();
                for i in 0..size {
                    vec.push(black_box(i as u8));
                }
                black_box(vec);
            });
        });

        group.bench_with_input(BenchmarkId::new("SmallVec<16>", size), &size, |b, &size| {
            b.iter(|| {
                let mut vec = SmallVec::<[u8; 16]>::new();
                for i in 0..size {
                    vec.push(black_box(i as u8));
                }
                black_box(vec);
            });
        });

        group.bench_with_input(BenchmarkId::new("Vec", size), &size, |b, &size| {
            b.iter(|| {
                let mut vec = Vec::<u8>::new();
                for i in 0..size {
                    vec.push(black_box(i as u8));
                }
                black_box(vec);
            });
        });
    }

    group.finish();
}

fn bench_clone(c: &mut Criterion) {
    let mut group = c.benchmark_group("clone");

    // Cloning small inline vecs
    group.bench_function("TeenyVec_small_8", |b| {
        let mut vec = TeenyVec::new();
        for i in 0..8 {
            vec.push(i);
        }
        b.iter(|| {
            let cloned = vec.clone();
            black_box(cloned);
        });
    });

    group.bench_function("SmallVec_small_8", |b| {
        let mut vec = SmallVec::<[u8; 16]>::new();
        for i in 0..8 {
            vec.push(i);
        }
        b.iter(|| {
            let cloned = vec.clone();
            black_box(cloned);
        });
    });

    group.bench_function("Vec_small_8", |b| {
        let mut vec = Vec::<u8>::new();
        for i in 0..8 {
            vec.push(i);
        }
        b.iter(|| {
            let cloned = vec.clone();
            black_box(cloned);
        });
    });

    group.finish();
}

fn bench_size_of(c: &mut Criterion) {
    c.bench_function("size_of_TeenyVec", |b| {
        b.iter(|| black_box(core::mem::size_of::<TeenyVec>()));
    });

    c.bench_function("size_of_SmallVec16", |b| {
        b.iter(|| black_box(core::mem::size_of::<SmallVec<[u8; 16]>>()));
    });

    c.bench_function("size_of_Vec", |b| {
        b.iter(|| black_box(core::mem::size_of::<Vec<u8>>()));
    });
}

criterion_group!(
    benches,
    bench_push_small,
    bench_push_medium,
    bench_clone,
    bench_size_of
);
criterion_main!(benches);
