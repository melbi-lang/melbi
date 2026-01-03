# Support Crates

This directory contains **support crates** — foundational utilities that support the main Melbi implementation but are not intrinsic parts of the language itself.

## Purpose

Support crates provide:
- **Generic, reusable implementations** that could benefit other projects
- **Low-level data structures** with performance-critical optimizations
- **Well-encapsulated unsafe code** in focused, auditable modules
- **Standalone utilities** that may be useful independently of Melbi

These crates are:
- ✅ Small and focused (single responsibility)
- ✅ Generic and reusable (not Melbi-specific)
- ✅ Potentially useful to external users
- ✅ Publishable to crates.io as standalone libraries

## Characteristics

### What Belongs Here
- **Generic, reusable implementations** - Code that does something well and isn't Melbi-specific
- **Performance-critical data structures** - e.g., inline-optimized collections
- **Foundational types** used across multiple Melbi crates
- **Standalone utilities** with clear, narrow scope that could benefit the broader Rust ecosystem

Even if a Melbi crate allows unsafe code, consider moving generic utilities here if they:
- Have value as standalone libraries
- Deserve focused documentation and testing
- Could be reused by other projects

### What Doesn't Belong Here
- **Melbi-specific features** - Parser, type system, VM, language semantics, etc.
- **Business logic** - Code tightly coupled to Melbi's design
- **Vague "utils"** - General-purpose grab-bags without a clear, focused purpose

## Organization

Each support crate lives in its own subdirectory (e.g., `small-str/`, `teeny-vec/`) and is:
- Named with the `melbi-` prefix for published crates
- Independently versioned (though typically released in lockstep with Melbi)
- Documented as if it were a standalone library

If this directory grows to contain many crates, it may be subdivided into categories like:
- `support/container/` - Container/collection types
- `support/text/` - String and text utilities
- etc.

Until then, a flat structure keeps things simple.

## Guidelines for Contributors

When adding or modifying support crates:

1. **Justify unsafe code** - Every `unsafe` block must have a `// SAFETY:` comment explaining why it's sound
2. **Keep scope narrow** - Each crate should do one thing well
3. **Document thoroughly** - These may be used independently; write docs for external users
4. **Test extensively** - Unsafe code requires extra scrutiny and test coverage
5. **Consider alternatives** - Are there public crates that already do that? Could this be implemented safely? Is the performance gain worth the maintenance cost?

## Examples

Current support crates include utilities like `small-str` (inline string optimization) and `teeny-vec` (inline vector for small collections), but check the directory contents for the current list.
