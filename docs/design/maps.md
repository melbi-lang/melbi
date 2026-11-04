---
title: Maps - Design and Implementation
---

# Design Doc: Maps

**Date**: 2026-08-02

**Status**: designed, not implemented. Written up from design discussions;
the decisions are @NiltonVolpato's. Updated 2026-11-03: one float comparison
rule for the whole language, hashed representation, `Hashable` trait.

**Scope**: how `Map` values are represented and constructed in the new
`melbi-values` crate, and how the VM builds one without having types. Covers
both the language-visible semantics and the implementation architecture,
because the two are unusually entangled here.

---

## Why maps are the hard case

Every other value kind can be interpreted from its raw storage plus a type
supplied by the caller. Maps cannot: looking a key up requires comparing it
against stored keys, and in the new design **stored values carry no type**.
`Val<B>` is raw storage, and the type lives externally in `Value<B>` at the
outermost level only (see `values/src/traits/builder.rs`).

The old implementation in `core/src/values/dynamic.rs` avoids the problem by
carrying the type inside every `Value`, which lets it write `impl Ord for
Value` (`dynamic.rs:144`) and sort. That option is gone by construction here,
and it is the reason work stopped at maps rather than continuing past them.

So the question the whole design answers is: **where does the key type come
from, at the moment a key must be compared?**

---

## Language-visible semantics

### Key types

Keys must be `Hashable`: every type except functions, including records and
other composite types, and floats. Function values may not be keys.

### One comparison rule for floats

Floats are compared the same way everywhere — `==`, `<`, sorting, and map
keys — by canonicalizing and then using a total order:

```rust
fn float_cmp(a: f64, b: f64) -> Ordering {
    canonical(a).total_cmp(&canonical(b))
}

fn canonical(x: f64) -> f64 {
    if x.is_nan() { f64::NAN }   // one NaN, positive
    else if x == 0.0 { 0.0 }     // -0.0 → 0.0
    else { x }
}
```

This matches IEEE except for `NaN`:

| | IEEE | Melbi |
|---|---|---|
| `-0.0 == 0.0` | true | true |
| `NaN == NaN` | false | **true** |
| `NaN` vs numbers | unordered | **greater than all** |

The deviation buys one rule with no exceptions: no separate key equality, no
separate sort order, and `NaN` is a usable map key. `x != x` no longer detects
`NaN`, so the standard library needs an explicit `IsNaN`.

Rejected alternatives:

- **IEEE everywhere.** Sorting then has no defined result once a `NaN` is
  present (and Rust's `sort_by` may panic on a non-total comparator), and a
  `NaN` key could be inserted but never found.
- **IEEE for `<`, a total order for sorting and keys.** Two rules for the same
  values.
- **Raw `total_cmp`.** `-0.0 == 0.0` becomes false, which common arithmetic
  produces (`0.0 * -1.0`), and `NaN`'s position depends on its sign bit, which
  differs between x86 and ARM for `0.0 / 0.0`.

### Duplicate keys: last wins

`{k1: a, k2: b}` where `k1` and `k2` evaluate equal keeps the last.

Keys are arbitrary runtime expressions, so detecting duplicates would mean a
*runtime error* on map construction, putting every map literal on the error
path. Constant-folded literals could be detected at compile time, but a rule
that applies only to constants is a worse rule than one that always applies.
Last-wins also falls out of repeated insertion for free.

### Iteration order

Not guaranteed by the language. If the implementation uses hashing, iteration
order must still be *reproducible within a run* so that display, serialization
and test output are stable.

---

## The witness is the type

The operations a map needs — `hash`, `eq`, and later `display` — are **total
functions of the key type**. Nothing else is required. So the type itself is
the witness; there is no separate witness or dictionary structure.

This matters mainly for what it deletes. An explicit witness struct would need
a composition rule (`witness(Array[T])` from `witness(T)`), interning, and a
lifetime story. Types already have all of that, because type composition *is*
witness composition.

Dynamic values use type-directed native functions, recursing structurally with
the type as the guide, exactly as `Array::get` re-attaches `element_ty` when
reading an element. Typed Rust values implement a trait named after the type
class:

```rust
// Dynamic, type-directed: the reference implementation.
fn key_eq<B: ValueBuilder>(ty: &Ty<B::TB>, a: &Val<B>, b: &Val<B>) -> bool;
fn key_hash<B: ValueBuilder>(ty: &Ty<B::TB>, value: &Val<B>, state: &mut impl Hasher);

// Typed keys, e.g. i64, f64, typed Str, typed Array<B, E>.
pub trait Hashable<B: ValueBuilder>: Marshal<B> {
    fn key_eq(&self, other: &Self) -> bool;
    fn key_hash(&self, state: &mut impl Hasher);
}
```

A map built by typed code and one built dynamically share one layout, so the
typed implementations **must agree exactly** with the dynamic ones — same
equality, same hash. Tests check every typed implementation against the
dynamic function on the same values.

`Display` has the same shape — fully dynamic on the type — which is the
argument that one mechanism should cover all of them rather than maps getting
a special case.

### Explicitly rejected

- **A cache inside `Ty`** (a `OnceCell` holding compiled ops). It puts interior
  mutability into the most-shared structure in the system, and it is not needed:
  if a type ever has custom `eq`, that is the dynamic version, recursively for
  its subtypes.
- **A separate witness table passed alongside values.** Denormalizes what the
  type already carries.

### Future: compiled operations

The eventual fast path is a *bytecode generator* trait, implemented by types
with custom methods, itself constant and generic, working by invoking the
generators of its subtypes. A generated `eq` runs in its own Frame, so it is
not reentrant with the caller and may allocate freely.

**Out of scope for now.** The dynamic, type-guided version is the reference
implementation and must exist regardless — see "FFI" below — so it is what
gets built first. Nothing above changes when compiled operations arrive.

---

## Where the key type comes from

Three sources, one mechanism.

### 1. Compiled code: captured by the adapter

After type checking the compiler knows the key type at every map-construction
site and captures it in a `MapAdapter`.

### 2. Polymorphic lambdas: still compile time

This is not a special case, because **polymorphic lambdas are compiled once per
instantiation**. `core/src/compiler/bytecode.rs:1240` compiles a `Mono` entry
for each substitution, with a `Unification::from_substitution` per one, and
emits a `Poly` entry listing them. `BytecodeCompiler::resolve_type`
(`bytecode.rs:304`) applies that substitution, so types are concrete by the
time an adapter is built.

Consequence: **a `MapAdapter` capturing a key type is a correct compile-time
constant**, and no resolve-at-call path is needed. `CastAdapter`
(`bytecode.rs:1213`) already relies on exactly this.

### 3. FFI: runtime types

Native functions are compiled once, before any Melbi instantiation exists, so
nothing inside them can be specialized. The classic case is:

```
Tally: (Array[a]) -> Map[a, Int]
```

The *call site* knows `a`, so the type arrives as a runtime value and the
native code stays generic. This is the path that can never be deleted, which
is why it is the reference implementation.

This is **not** gradual typing. Nothing is unchecked and no cast can fail —
the type checker has already proven the instantiation, and the type travels as
a witness to a proven fact. The relevant axis is erased vs reified generics;
the closest analogue is Swift, which compiles generics once and passes type
metadata plus witness tables, treating specialization as an optimization.

---

## `MapMaker`

One implementation, reachable two ways.

```rust
let mut maker = MapMaker::new(key_type, value_type);
maker.insert(k, v);
let map = maker.build();   // takes self
```

It solves four things at once:

- **Empty maps.** `MapMaker::new(k, v).build()` is correctly typed with zero
  entries. Inferring the type from the elements fundamentally cannot be.
- **FFI accumulation.** `Tally` does not know its count up front.
- **Last-wins**, for free, from repeated insertion.
- **Representation choice**, made inside `build()` where no caller sees it.

### Transience

A `MapMaker` is mutable until built, which is the one place this cuts against
Melbi's immutability. It must be consumed exactly once and never be reachable
from a built map.

This is enforced rather than merely documented: **`MapMaker` is not a Melbi
type**, so an FFI function cannot return one without a type error. `build`
takes `self` by value so the Rust side gets the same guarantee.

---

## VM integration

**No new instruction, and one deleted.** `MakeMap` goes away.

`core/src/vm/generic_adapter.rs` already defines `GenericAdapter` — "VM
operations that need type information at runtime", which is this problem
exactly — and `core/src/vm/code.rs:17` already holds
`generic_adapters: Vec<Box<dyn GenericAdapter>>`, dispatched by
`CallGenericAdapter` (`instruction_set.rs:419`).

So a `MapAdapter` joins `CastAdapter`, `FormatStrAdapter` and
`ArrayContainsAdapter`, capturing the key and value types and wrapping
`MapMaker`. `num_args()` returns the entry count. Argument width is not a
constraint: `WideArg` extends any instruction argument a byte at a time.

### Frame vs VM

Adapters live in `Code`, which is per-function — that is, **Frame-local**,
along with bytecode, constants and the precomputed stack size. What belongs to
the VM rather than a Frame: the arenas, globally shared constants, and the fuel
budget.

---

## Representation

**`hashbrown::HashTable`**, with a fixed-seed hasher so iteration order is
reproducible between runs. Note `HashTable`, not `HashMap`: it takes hash and
eq **closures per operation**, so type-erased keys need no `Hash`/`Eq` impls,
and it is generic over `A: Allocator`, so the arena works through
`allocator-api2` (already a workspace dependency).

The previous implementation (`core/src/values/dynamic.rs`) used a sorted array,
which made every value orderable just to support it, and mixed IEEE `==` for
deduplication with `total_cmp` for lookup. Hashing needs only equality, so key
types need `Hashable`, not an order.

Since Melbi values are immutable, a map is built once and never mutated, so
the table is finalized at `build()` time. Small maps may not justify hashing at
all; that remains an implementation choice inside `build()`.

## Ordering

`Ord` is one type class used by both `<` and sorting, so they cannot disagree.
Its instances extend structurally:

| Type | Ordered |
|---|---|
| `Int`, `Float`, `Str`, `Bytes` | yes (floats by the rule above) |
| `Bool` | `false < true` |
| `Option[e: Ord]` | `none < some x` |
| `Array[e: Ord]` | lexicographic |
| Record with all fields `Ord` | field by field, in field-name order |
| `Map`, `Function` | no |

## See also

- `docs/design/polymorphic-lambda-type-resolution.md` — the instantiation
  machinery this design depends on, and which uses maps as its example.
- `values/src/traits/builder.rs` — `ValueBuilder`, and the `TODO` listing the
  handles still to add.
