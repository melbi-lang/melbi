# melbi-types

Type system foundation for the Melbi expression language.

This crate provides a generic type representation that works with different storage strategies (arena allocation, reference counting, etc.).

## Directory Structure

```
src/
├── lib.rs                      # Crate root, re-exports
├── core/                       # Core type system abstractions
│   ├── builder.rs              # TyBuilder trait
│   ├── ty.rs                   # Ty, TyNode, TyList, FieldList, IdentList, Ident
│   ├── kind.rs                 # TyKind enum, Scalar enum
│   ├── flags.rs                # TyFlags bitflags
│   └── traversal/              # Generic traversal traits
│       └── visit.rs            # Visit trait
├── algo/                       # Concrete algorithm implementations
│   └── (future: substitute.rs, resolve.rs, collect.rs)
└── builders/                   # TyBuilder implementations
    ├── box_builder.rs          # Rc-based allocation
    └── arena_builder.rs        # Bumpalo arena allocation
```

## Where to Put New Code

| What you're adding | Where to put it |
|--------------------|-----------------|
| New traversal trait (transform, fold, etc.) | `core/traversal/` |
| Traversal implementation (substitute, resolve) | `algo/` |
| New allocation strategy | `builders/` |
| New type kind variant | `core/kind.rs` |
| Type scheme, type class | Root level (`src/scheme.rs`, etc.) |

## Key Types

- **`TyBuilder`**: Trait defining how types are allocated. Implementations provide different storage strategies.
- **`Ty<B>`**: Lightweight wrapper around an allocated type.
- **`TyNode<B>`**: Type node containing flags and kind.
- **`TyKind<B>`**: The actual type variants (Scalar, Array, Map, Record, Function, Symbol, TypeVar).
- **`TyFlags`**: Cached properties of a type (e.g., whether it contains type variables).

## Usage

```rust
use melbi_types::{BoxBuilder, TyBuilder, TyKind, Scalar};

let builder = BoxBuilder::new();
let int_ty = TyKind::Scalar(Scalar::Int).alloc(&builder);
let arr_ty = TyKind::Array(int_ty).alloc(&builder);
```

With arena allocation:

```rust
use melbi_types::{ArenaBuilder, TyBuilder, TyKind, Scalar};
use bumpalo::Bump;

let arena = Bump::new();
let builder = ArenaBuilder::new(&arena);

let int_ty = TyKind::Scalar(Scalar::Int).alloc(&builder);
let arr_ty = TyKind::Array(int_ty).alloc(&builder);
```

## Adding a New TyBuilder

1. Create a new file in `builders/`
2. Implement `TyBuilder` for your type, providing:
   - `TyHandle`: How individual types are stored
   - `IdentHandle`: How identifiers are stored
   - `TyListHandle`, `IdentListHandle`, `FieldListHandle`: How lists are stored
   - Allocation methods for each handle type
3. Re-export from `builders/mod.rs`

## Adding a New Traversal Algorithm

1. Define the trait in `core/traversal/` (if it's a new pattern)
2. Implement the algorithm in `algo/`
3. Re-export as needed
