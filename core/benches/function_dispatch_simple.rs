use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};

fn bench_simple(c: &mut Criterion) {
    c.bench_function("simple_add", |b| b.iter(|| black_box(1 + 1)));
}

criterion_group!(benches, bench_simple);
criterion_main!(benches);
