//! Benchmarks for the `TypeManager`.
//!
//! This file contains benchmarks to measure type interning and serialization performance.
//! Run with: `cargo bench --bench type_manager` in the core/ directory.
//!
//! Benchmark groups:
//! 1. `record_creation`: Creating new record types (not yet interned)
//! 2. `record_interning`: Getting already-interned record types
//! 3. serialization: Serializing types to bytes
//! 4. deserialization: Deserializing types from bytes

use std::hint::black_box;

use bumpalo::Bump;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use melbi_core::types::Type;
use melbi_core::types::encoding::{decode, encode};
use melbi_core::types::manager::TypeManager;

/// Helper function to create various complex types by name
fn create_complex_type<'a>(manager: &'a TypeManager<'a>, name: &str) -> &'a Type<'a> {
    match name {
        "int" => manager.int(),
        "float" => manager.float(),
        "bool" => manager.bool(),
        "string" => manager.str(),
        "bytes" => manager.bytes(),
        "array_int" => manager.array(manager.int()),
        "map_string_int" => manager.map(manager.str(), manager.int()),
        "simple_record" => {
            let fields = vec![
                ("name", manager.str()),
                ("age", manager.int()),
                ("active", manager.bool()),
            ];
            manager.record(fields)
        }
        "nested_record" => {
            let inner_fields = vec![("street", manager.str()), ("city", manager.str())];
            let inner = manager.record(inner_fields);

            let outer_fields = vec![("name", manager.str()), ("address", inner)];
            manager.record(outer_fields)
        }
        "function" => {
            let params = vec![manager.int(), manager.str(), manager.bool()];
            manager.function(&params, manager.int())
        }
        _ => panic!("Unknown type name: {name}"),
    }
}

/// Benchmark: Creating new record types (first time, not yet interned).
///
/// Measures the cost of creating a record type with N fields, including:
/// - String interning for field names
/// - Sorting fields
/// - Type interning
/// - Arena allocation
fn bench_record_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("record_creation");

    let type_names = [
        "int",
        "float",
        "bool",
        "string",
        "array_int",
        "map_string_int",
    ];

    for num_fields in [5, 10, 50] {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_fields),
            &num_fields,
            |b, &num_fields| {
                // Setup: Create arena and manager OUTSIDE the benchmark
                let arena = Bump::new();
                let manager = TypeManager::new(&arena);
                let mut counter = 0u64;

                b.iter(|| {
                    // Prepare field data with varied types and unique names per iteration
                    let fields: Vec<(&str, _)> = (0..num_fields)
                        .map(|i| {
                            // Make field names unique per iteration to avoid cache hits
                            let name = arena.alloc_str(&format!("field_{i}_{counter}"));
                            let type_name = type_names[i % type_names.len()];
                            let ty = create_complex_type(manager, type_name);
                            (name as &str, ty)
                        })
                        .collect();

                    counter += 1;

                    // Benchmark: Create the record type (fresh each time)
                    let record = manager.record(black_box(fields));
                    black_box(record);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Getting already-interned record types.
///
/// Measures the cost of getting a record type that's already been interned.
/// This should be fast - just a `HashMap` lookup.
fn bench_record_interning(c: &mut Criterion) {
    let mut group = c.benchmark_group("record_interning");

    let type_names = [
        "int",
        "float",
        "bool",
        "string",
        "array_int",
        "map_string_int",
    ];

    for num_fields in [5, 10, 50] {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_fields),
            &num_fields,
            |b, &num_fields| {
                // Setup: Create arena, manager, and pre-intern the record
                let arena = Bump::new();
                let manager = TypeManager::new(&arena);

                let fields: Vec<(&str, _)> = (0..num_fields)
                    .map(|i| {
                        let name = arena.alloc_str(&format!("field_{i}"));
                        let type_name = type_names[i % type_names.len()];
                        let ty = create_complex_type(manager, type_name);
                        (name as &str, ty)
                    })
                    .collect();

                // Pre-intern the record
                let _first = manager.record(fields.clone());

                // Benchmark: Get the already-interned record
                b.iter(|| {
                    let fields: Vec<(&str, _)> = (0..num_fields)
                        .map(|i| {
                            let name = arena.alloc_str(&format!("field_{i}"));
                            let type_name = type_names[i % type_names.len()];
                            let ty = create_complex_type(manager, type_name);
                            (name as &str, ty)
                        })
                        .collect();

                    let record = manager.record(black_box(fields.clone()));
                    black_box(record)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Serializing types to bytes.
///
/// Measures the cost of serializing types to postcard format.
/// Tests a simple type and a complex record (cost varies by size).
fn bench_type_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("type_serialization");

    // Simple type
    group.bench_function("simple_postcard", |b| {
        let arena = Bump::new();
        let manager = TypeManager::new(&arena);
        let ty = manager.array(manager.int());

        b.iter(|| {
            let bytes = manager
                .serialize_type(black_box(ty))
                .expect("Serialization failed");
            black_box(bytes)
        });
    });

    // Simple Melbi encoding
    group.bench_function("simple_melbi", |b| {
        let arena = Bump::new();
        let manager = TypeManager::new(&arena);
        let ty = manager.array(manager.int());

        b.iter(|| {
            let bytes = encode(black_box(ty));
            black_box(bytes)
        });
    });

    // Complex record (size matters for serialization)
    group.bench_function("complex_record_postcard", |b| {
        let arena = Bump::new();
        let manager = TypeManager::new(&arena);

        let fields: Vec<(&str, _)> = (0..10)
            .map(|i| {
                let name = arena.alloc_str(&format!("field_{i}"));
                (name as &str, manager.int())
            })
            .collect();

        let record = manager.record(fields);

        b.iter(|| {
            let bytes = manager
                .serialize_type(black_box(record))
                .expect("Serialization failed");
            black_box(bytes)
        });
    });

    // Complex record (size matters for serialization)
    group.bench_function("complex_record_melbi", |b| {
        let arena = Bump::new();
        let manager = TypeManager::new(&arena);

        let fields: Vec<(&str, _)> = (0..10)
            .map(|i| {
                let name = arena.alloc_str(&format!("field_{i}"));
                (name as &str, manager.int())
            })
            .collect();

        let record = manager.record(fields);

        b.iter(|| {
            let bytes = encode(black_box(record));
            black_box(bytes)
        });
    });

    group.finish();
}

/// Benchmark: Deserializing types from bytes.
///
/// Measures the cost of deserializing types from postcard format.
/// Tests a simple type and a complex record (cost varies by size).
fn bench_type_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("type_deserialization");

    // Simple type
    group.bench_function("simple_postcard", |b| {
        let arena = Bump::new();
        let manager = TypeManager::new(&arena);
        let ty = manager.array(manager.int());
        let bytes = manager.serialize_type(ty).expect("Serialization failed");

        b.iter(|| {
            let deserialized = manager
                .deserialize_type(black_box(&bytes))
                .expect("Deserialization failed");
            black_box(deserialized)
        });
    });

    group.bench_function("simple_melbi", |b| {
        let arena = Bump::new();
        let manager = TypeManager::new(&arena);
        let ty = manager.array(manager.int());

        let bytes = encode(ty);

        b.iter(|| {
            let deserialized = decode(black_box(&bytes), manager).expect("Deserialization failed");
            black_box(deserialized)
        });
    });

    // Complex record (size matters for deserialization)
    group.bench_function("complex_record_postcard", |b| {
        let arena = Bump::new();
        let manager = TypeManager::new(&arena);

        let fields: Vec<(&str, _)> = (0..10)
            .map(|i| {
                let name = arena.alloc_str(&format!("field_{i}"));
                (name as &str, manager.int())
            })
            .collect();

        let record = manager.record(fields);
        let bytes = manager
            .serialize_type(record)
            .expect("Serialization failed");

        b.iter(|| {
            let deserialized = manager
                .deserialize_type(black_box(&bytes))
                .expect("Deserialization failed");
            black_box(deserialized)
        });
    });

    // Complex record (size matters for deserialization)
    group.bench_function("complex_record_melbi", |b| {
        let arena = Bump::new();
        let manager = TypeManager::new(&arena);

        let fields: Vec<(&str, _)> = (0..10)
            .map(|i| {
                let name = arena.alloc_str(&format!("field_{i}"));
                (name as &str, manager.int())
            })
            .collect();

        let record = manager.record(fields);
        let bytes = encode(record);

        b.iter(|| {
            let deserialized = decode(black_box(&bytes), manager).expect("Deserialization failed");
            black_box(deserialized)
        });
    });

    group.finish();
}

/// Benchmark: Comparing types using serialized byte arrays.
///
/// Byte comparison cost varies by size, so test small and large types.
fn bench_type_equality_bytes(c: &mut Criterion) {
    let mut group = c.benchmark_group("type_equality_bytes");

    // Small type (few bytes)
    group.bench_function("small_postcard", |b| {
        let arena = Bump::new();
        let manager = TypeManager::new(&arena);
        let ty = manager.int();

        let bytes1 = manager.serialize_type(ty).expect("Serialization failed");
        let bytes2 = bytes1.clone();

        b.iter(|| {
            let result = black_box(&bytes1) == black_box(&bytes2);
            black_box(result)
        });
    });

    // Small type (few bytes)
    group.bench_function("small_melbi", |b| {
        let arena = Bump::new();
        let manager = TypeManager::new(&arena);
        let ty = manager.int();

        let bytes1 = encode(ty);
        let bytes2 = bytes1.clone();

        b.iter(|| {
            let result = black_box(&bytes1) == black_box(&bytes2);
            black_box(result)
        });
    });

    // Large type (many bytes)
    group.bench_function("large_postcard", |b| {
        let arena = Bump::new();
        let manager = TypeManager::new(&arena);

        let fields: Vec<(&str, _)> = (0..50)
            .map(|i| {
                let name = arena.alloc_str(&format!("field_{i}"));
                (name as &str, manager.int())
            })
            .collect();

        let record = manager.record(fields);
        let bytes1 = manager
            .serialize_type(record)
            .expect("Serialization failed");
        let bytes2 = bytes1.clone();

        b.iter(|| {
            let result = black_box(&bytes1) == black_box(&bytes2);
            black_box(result)
        });
    });

    // Large type (many bytes)
    group.bench_function("large_melbi", |b| {
        let arena = Bump::new();
        let manager = TypeManager::new(&arena);

        let fields: Vec<(&str, _)> = (0..50)
            .map(|i| {
                let name = arena.alloc_str(&format!("field_{i}"));
                (name as &str, manager.int())
            })
            .collect();

        let record = manager.record(fields);
        let bytes1 = encode(record);
        let bytes2 = bytes1.clone();

        b.iter(|| {
            let result = black_box(&bytes1) == black_box(&bytes2);
            black_box(result)
        });
    });

    group.finish();
}

/// Benchmark: Comparing types using pointer equality (current approach).
///
/// Pointer comparison is constant time regardless of type.
fn bench_type_equality_pointers(c: &mut Criterion) {
    let mut group = c.benchmark_group("type_equality_pointers");

    group.bench_function("pointer_eq", |b| {
        let arena = Bump::new();
        let manager = TypeManager::new(&arena);
        let ty1 = manager.array(manager.int());
        let ty2 = ty1; // Same pointer

        b.iter(|| {
            let result = core::ptr::eq(black_box(ty1), black_box(ty2));
            black_box(result)
        });
    });

    group.finish();
}

/// Benchmark: Reading type discriminant from serialized bytes.
///
/// Measures the cost of zero-copy inspection (just array indexing).
/// Cost is constant regardless of type.
fn bench_read_discriminant(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_discriminant");

    group.bench_function("read_byte", |b| {
        let arena = Bump::new();
        let manager = TypeManager::new(&arena);
        let ty = manager.array(manager.int());
        let bytes = manager.serialize_type(ty).expect("Serialization failed");

        b.iter(|| {
            let discriminant = black_box(&bytes)[0];
            black_box(discriminant)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_record_creation,
    bench_record_interning,
    bench_type_serialization,
    bench_type_deserialization,
    bench_type_equality_bytes,
    bench_type_equality_pointers,
    bench_read_discriminant
);
criterion_main!(benches);
