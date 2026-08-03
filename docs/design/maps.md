---
title: Maps - Design and Implementation
---

# Design Doc: Maps

**Date**: 2026-08-02

**Status**: designed, not implemented. Written up from a design discussion;
the decisions are @NiltonVolpato's.

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

Any type with structural equality may be a key, including composite types and
floats. Function values may not.

### Floats are allowed, with canonicalization

`Float` keys are permitted, on the condition that keys are canonicalized on
**both insertion and lookup**:

- `-0.0` becomes `0.0`
- every `NaN` bit pattern becomes one canonical `NaN`

After canonicalization, equality and hashing are bitwise. Two consequences are
deliberate and user-visible:

- `0.0` and `-0.0` are the same key. This *matches* `==`.
- `NaN` is a usable key and equals itself. This *differs* from `==`, where
  `NaN != NaN`. Without it, a `NaN` key could be inserted and never retrieved,
  which is worse than the inconsistency.

Note this is only tractable because lookup needs **equality**, not a total
order. A sorted representation would need `NaN` to sit somewhere in a total
order, and getting that subtly wrong yields silently missed lookups rather
than a visible failure. See "Representation" below.

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

Operations are therefore type-directed native functions:

```rust
fn hash(key_ty: &Ty<B::TB>, value: &Val<B>) -> u64;
fn eq(key_ty: &Ty<B::TB>, a: &Val<B>, b: &Val<B>) -> bool;
```

recursing structurally with the type as the guide, exactly as `Array::get`
re-attaches `element_ty` when reading an element.

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

Chosen inside `build()`; both remain viable and the choice is not
load-bearing for anything else in this document.

- **Sorted array + binary search.** What `core/src/values/dynamic.rs:1213`
  does today. Needs a *total order*, which is what makes float keys awkward.
- **`hashbrown::HashTable`.** Note `HashTable`, not `HashMap`: it takes hash
  and eq **closures per operation**, so type-erased keys need no `Hash`/`Eq`
  impls, and it is generic over `A: Allocator`, so the arena works through
  `allocator-api2` (already a workspace dependency). Needs only equality, not
  an order.

Since Melbi values are immutable, a map is built once and never mutated, so
either structure can be finalized at `build()` time. Small maps may not justify
hashing at all.

---

## See also

- `docs/design/polymorphic-lambda-type-resolution.md` — the instantiation
  machinery this design depends on, and which uses maps as its example.
- `values/src/traits/builder.rs` — `ValueBuilder`, and the `TODO` listing the
  handles still to add.
