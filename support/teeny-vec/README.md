# TeenyVec

A tiny inline vector optimized for small byte sequences that automatically promotes to heap storage when needed.

## Motivation

Many data structures contain small byte sequences that rarely exceed a few bytes. Standard `Vec<u8>` always heap-allocates, which adds overhead for these common small cases. `SmallVec` addresses this but uses 24+ bytes of storage.

TeenyVec is designed for scenarios where:
- Most sequences are small (under 14 bytes)
- Memory footprint matters (e.g., storing many sequences in arrays or structs)
- The vector is frequently passed around or returned from functions

## Features

- **Exactly 16 bytes** (2 machine words on 64-bit architectures)
- **14 bytes inline capacity** without heap allocation
- **Automatic heap promotion** when capacity is exceeded
- **Zero-cost discriminant** using odd/even encoding (no extra tag byte)
- **Register-friendly**: can be passed/returned via registers instead of stack pointers

## Design Rationale

### Size: 16 bytes (2 words)

The 16-byte size is intentional:
- Fits in exactly 2 CPU registers on x86-64 and arm64
- Can be passed to/from functions via registers (avoiding stack spills)
- Enables efficient `Clone` for inline data (just copy 16 bytes)

### Discriminant: Odd/Even Encoding

TeenyVec distinguishes stack vs heap storage without a dedicated tag byte:
- **Heap mode**: capacity is always even (power-of-two allocations)
- **Stack mode**: length is encoded as `2 * actual_len + 1` (always odd)

This allows checking the first 2 bytes to determine the storage mode, leaving all remaining bytes for actual data.

### Layout

```text
Stack: [len: u16 (odd)] [data: 14 bytes]
Heap:  [cap: u16 (even)] [len: u16] [ptr: 8 bytes]
```

## When to Use TeenyVec

**Good fit:**
- Byte sequences typically under 14 bytes
- Memory-constrained environments
- High-frequency allocation/deallocation patterns
- Data structures containing many small vectors

**Consider alternatives:**
- Sequences frequently exceed 14 bytes → use `Vec<u8>`
- Need to store non-byte types → use `SmallVec` or `Vec`
- Require `no_std` without `alloc` → inline-only solution needed

## Benchmark

Benchmarks compare TeenyVec against `Vec<u8>` and `SmallVec<[u8; 16]>`. Times shown are absolute for the winner, with percentage overhead for others. All benchmarks run on the same hardware with the same methodology.

### push_small_inline

Measures the time to create a new vector and push N bytes, where N stays within inline capacity (14 bytes). This is TeenyVec's primary use case. The lack of heap allocation gives TeenyVec a significant advantage across the board for inline sizes, beating both `Vec` and `SmallVec`! It outperforms even `SmallVec` which is a data-structure designed for this specific use-case (unlike `Vec` which is designed to be generic).

| Implementation | N=1 | N=4 | N=8 | N=12 | N=14 | Overall |
| --- | --- | --- | --- | --- | --- | --- |
| TeenyVec | 🏆(3.5ns)  | 🏆(6.8ns)  | 🏆(13.8ns)  | 🏆(20.9ns)  | 🏆(23.0ns)  | 🏆 x 5 |
| Vec | +251.0%  | +110.0%  | +2.3%  | +31.0%  | +25.4%  | - |
| SmallVec<16> | +179.6%  | +79.3%  | +63.5%  | +34.4%  | +45.0%  | - |
| --- | --- | --- | --- | --- | --- | --- |

### push_medium_heap

Measures the time to push N bytes where N exceeds inline capacity, forcing heap allocation. Here `Vec` wins for larger sizes due to its optimized allocator integration. TeenyVec still wins at N=20 (just past the inline threshold) but falls behind as sizes grow. This benchmark shows TeenyVec's tradeoff: optimized for small data, not large allocations.

| Implementation | N=20 | N=32 | N=64 | N=128 | Overall |
| --- | --- | --- | --- | --- | --- |
| Vec | +11.5%  | 🏆(56.4ns)  | 🏆(95.6ns)  | 🏆(149.6ns)  | 🏆 x 3 |
| TeenyVec | 🏆(46.8ns)  | +13.8%  | +43.8%  | +90.9%  | 🏆 x 1 |
| SmallVec<16> | +60.9%  | +96.3%  | +115.8%  | +132.2%  | - |
| --- | --- | --- | --- | --- | --- |

### clone

Measures the time to clone a vector containing N bytes (within inline capacity). TeenyVec's clone is a simple 16-byte memcpy with no heap interaction, making it dramatically faster. This is particularly valuable when vectors are frequently copied (e.g., in functional-style code or copy-on-write patterns).

| Implementation | N=8 | Overall |
| --- | --- | --- |
| TeenyVec | 🏆(1.5ns)  | 🏆 x 1 |
| SmallVec | +137.1%  | - |
| Vec | +663.8%  | - |
| --- | --- | --- |
