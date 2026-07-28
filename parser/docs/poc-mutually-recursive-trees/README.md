# Mutually recursive trees: what compiles and what doesn't

Standalone probes written while designing `parser/src/traits/tree_builder.rs`,
kept because each one records a conclusion that is easy to get wrong by
reasoning alone. Every file is a self-contained crate with no dependencies:

```sh
rustc --edition 2024 --crate-type lib <file>.rs
```

Three of them are *supposed* to fail; the expected error is named below.

## The question

`ExprKind` and `LiteralKind` refer to each other — a numeric literal carries a
unit suffix that is itself an expression (``9.81`m/s^2` ``), and an expression
contains literals. Two mutually recursive trees. How should that be expressed,
and how does it scale when patterns, match arms, bindings, map entries and type
syntax arrive?

## The answer the crate uses

**One `TreeBuilder`, which is purely a storage strategy, plus one
`TreeDescriptor` per (tree × stage) carrying that tree's `Data` and `Kind`.**
Every tree type is indexed by the descriptor: `Tree<B, D>`, `TreeNode<B, D>`,
`B::Handle<D>`, `B::List<D>`.

`descriptor_design.rs` — **expected: compiles.**

```rust
pub trait TreeDescriptor: Sized + 'static {
    type Data: Clone + Debug + PartialEq;
    type Kind<B: TreeBuilder>: Clone + Debug + PartialEq;
}
pub struct TreeNode<B: TreeBuilder, D: TreeDescriptor> { data: D::Data, kind: D::Kind<B> }
```

Why this shape:

- `Tree<B, D>` is injective in **both** parameters, so inference works with no
  turbofish, including when a pass crosses from one tree into another and back.
- A pass pays **zero** where-clauses beyond `B: TreeBuilder`, no matter how many
  trees it touches, because `D::Data` is concrete. Under the previous design each
  pass had to restate the agreement between the data of every tree it visited.
- Adding a tree costs one marker struct and one impl. It changes no existing
  pass and does not touch `TreeBuilder`.
- A compiler stage is a descriptor, not a builder, so `ParsedExpr` and
  `TypedExpr` are different types with different kinds. Literals can fold away
  entirely (lowering a `ParsedLiteral` yields a value, not a node) and typed-only
  nodes have no parsed counterpart.

## Conclusions

### 1. Inlining the node into the handle overflows the solver

`typed_handles_overflow.rs` — **expected: E0275.**

```rust
type ExprTree = &'static (Span, Expr<Self>);   // bounded `PartialEq`
```

Proving `&(Span, Expr<B>): PartialEq` needs `Expr<B>: PartialEq`, which needs
`B::ExprTree: PartialEq`, which is where it started. This is the trap that makes
the `Tree`/`TreeNode` split load-bearing rather than cosmetic: the recursion has
to pass through an intermediate type whose impls are written by hand and
*unconditionally*, so the solver has somewhere to stop.

The same reasoning is why `Tree`'s `Clone`/`Debug`/`PartialEq` impls carry no
where-clauses, and why the bounds live on `TreeDescriptor::Data` and
`TreeDescriptor::Kind` — there they become assumptions, discharged once where a
concrete descriptor is defined.

### 2. Bundling builders behind a supertrait does not work

`projection_needs_turbofish.rs` — **expected: E0283.**

```rust
trait System { type Expr: TreeBuilder<TreeKind = ExprKind<Self>>; }
fn count<S: System>(e: &Tree<S::Expr>) -> usize { count(e) }
```

`S::Expr` is an associated-type projection, and projections are not injective:
`S` cannot be recovered from `Tree<S::Expr>`. Every call in every pass needs
`::<S>`, forever.

`projection_blocks_blanket_impl.rs` — **expected: E0207 and E0283.**

Worse, the convenience impl that forwards `Visit` from `Tree` to its contents
cannot even be written:

```rust
impl<S: System, C> Visit<C> for Tree<S::Expr> {}   // E0207: `S` is unconstrained
```

This is why there is no `System` trait bundling builders. Note that the adopted
design sidesteps the whole problem: `D` is a plain type parameter, not a
projection.

### 3. Builder traits naming each other work, but do not scale

`visit_on_kind_works.rs` — **expected: compiles.**

```rust
trait ExprBuilder: TreeBuilder<TreeKind = ExprKind<Self>> { type Lit: LitBuilder<Expr = Self>; }
trait LitBuilder:  TreeBuilder<TreeKind = LitKind<Self>>  { type Expr: ExprBuilder<Lit = Self>; }
```

The mutual `<Expr = Self>` / `<Lit = Self>` equality constraints hold this
together and the concrete fixed point resolves. **This was the crate's design
until it was replaced**, and it works fine for two trees.

It does not survive more. `N` trees need `N` traits holding `N-1` associated
types each, one builder struct per tree, and — the part that actually hurts —
every pass must restate the data agreement for each tree it touches:

```rust
impl<B> Visit<B, Census> for ExprKind<B>
where B: ExprBuilder<TreeData = Span>, B::Lit: LiteralBuilder<TreeData = Span>
```

That clause list grows with the number of trees in the pass. Descriptors reduce
it to nothing.

### 4. `Visit` goes on the *kind*, not on `TreeNode`

`visit_on_node_fails.rs` — **expected: E0119.**

```rust
impl<B: ExprBuilder> Visit<B, Count> for TreeNode<B> {}   // first
impl<B: LitBuilder>  Visit<B, Count> for TreeNode<B> {}   // conflicting implementation
```

Under the old design, coherence could not rule out one builder implementing both
traits, so two blanket impls on the shared `TreeNode<B>` overlapped.

`one_builder_cannot_be_both.rs` — **expected: E0271** — shows the conflict was
*false*: no type can implement both builder traits, because `TreeKind` would have
to be `ExprKind<Self>` and `LiteralKind<Self>` at once. Overlap checking does not
consider associated-type equality constraints, so it could not see the
disjointness the definitions guaranteed.

Under the descriptor design this hazard is gone — the tree is named by the `D`
parameter rather than by which trait `B` implements — but `Visit` stays on the
kind anyway, because `Self` should be the type a pass actually writes an impl
for. `Tree`/`TreeNode` forward to it through inherent methods.

### 5. Passing the system as a type parameter also works, but costs a parameter everywhere

`system_parameter.rs` — **expected: compiles.**

```rust
trait TreeBuilder<S: System> { type TreeKind; type TreeHandle; }
struct Tree<B: TreeBuilder<S>, S: System>(B::TreeHandle);
```

Viable, but the associated types now live on `TreeBuilder<S>`, so `S` has to be
named to reach them and `Tree` carries two parameters instead of one.

Notably, the adopted design *also* carries two parameters — `Tree<B, D>` — but
the second one earns its place: it selects the tree, which is information the
call site wants anyway, rather than threading a bundle that adds nothing.

## Two dead ends worth not re-deriving

Both were tried on the way to `descriptor_design.rs` and rejected:

- **Data on the builder, indexed by descriptor** (`type Data<D: TreeDescriptor>`).
  Works, but a pass must then pin the data per tree (`B: TreeBuilder<Ty = ()>`)
  to get a concrete type, which is a clause the descriptor design does not need.
- **A separate `Stage` trait** the data varies over (`type Data<S: Stage>`, with
  `TreeBuilder::Stage`). Also works, and is what you need *if* a single set of
  kind enums has to serve several stages. Making the stage part of the descriptor
  instead makes it unnecessary: `ParsedExpr` and `TypedExpr` already are
  different types, so their data can just be concrete. If it is ever
  reintroduced, note that `Stage` must **not** be `'static`, or a typed stage
  can never carry an arena-lifetime `Ty`.
