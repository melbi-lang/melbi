# TeenyVec

An inline vector with automatic promotion to the heap optimized for inline
storage.

It provides a compact vector type that:
* Is exactly 2 words in size (2 registers). Which allows it to:
  - Be kept on registers (if the compiler decides to)
  - Be passed to function calls via registers instead of pointer to stack object
* ...

## Benchmark

### push_small_inline

| Implementation | N=1 | N=4 | N=8 | N=12 | N=14 | Overall |
| --- | --- | --- | --- | --- | --- | --- |
| TeenyVec | 🏆(3.5ns)  | 🏆(6.8ns)  | 🏆(13.8ns)  | 🏆(20.9ns)  | 🏆(23.0ns)  | 🏆 x 5 |
| Vec | +251.0%  | +110.0%  | +2.3%  | +31.0%  | +25.4%  | - |
| SmallVec<16> | +179.6%  | +79.3%  | +63.5%  | +34.4%  | +45.0%  | - |
| --- | --- | --- | --- | --- | --- | --- |

### push_medium_heap

| Implementation | N=20 | N=32 | N=64 | N=128 | Overall |
| --- | --- | --- | --- | --- | --- |
| Vec | +11.5%  | 🏆(56.4ns)  | 🏆(95.6ns)  | 🏆(149.6ns)  | 🏆 x 3 |
| TeenyVec | 🏆(46.8ns)  | +13.8%  | +43.8%  | +90.9%  | 🏆 x 1 |
| SmallVec<16> | +60.9%  | +96.3%  | +115.8%  | +132.2%  | - |
| --- | --- | --- | --- | --- | --- |

### clone

| Implementation | N=8 | Overall |
| --- | --- | --- |
| TeenyVec | 🏆(1.5ns)  | 🏆 x 1 |
| SmallVec | +137.1%  | - |
| Vec | +663.8%  | - |
| --- | --- | --- |
