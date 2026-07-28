# Tree design probes

Three self-contained crates, kept because each records something that is easy to
get wrong by reasoning alone and cheap to check by compiling. No dependencies:

```sh
rustc --edition 2024 --crate-type lib -o /dev/null <file>.rs
```

| File | Expected | Records |
|---|---|---|
| `typed_handles_overflow.rs` | **E0275** | Inlining the node into the handle (`&'a (Data, Kind<Self>)`) overflows the solver. This is why `Tree` and `TreeNode` are two types, and why their `Clone`/`Debug`/`PartialEq` impls are hand-written and *unconditional*. |
| `bundling_descriptors_needs_turbofish.rs` | **E0283** | Grouping descriptors behind an `Ast` trait so a pass can be generic over the whole AST. Projections are not injective, so `A` is unrecoverable from `Tree<B, A::Expr>` and every call needs a turbofish. Name the descriptor directly. |
| `descriptor_design.rs` | compiles | The adopted design in miniature, including a **typed stage that `parser/src` does not have yet** — executable evidence that descriptors carry across a stage boundary, that per-tree data works, and that literals fold away into values with no typed counterpart. |

The rationale for the design itself lives in `parser/src/traits/tree_builder.rs`,
not here. These files only pin down the alternatives and their failure modes.

`descriptor_design.rs` retires once the typed stage lands in `parser/src`; the
other two do not, because they document things working code cannot show.
