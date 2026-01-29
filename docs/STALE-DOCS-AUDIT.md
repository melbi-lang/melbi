# Stale Documentation Audit

**Created**: 2025-01-28  
**Purpose**: Identify outdated documentation that could mislead contributors (especially AI)

---

## Summary

The MVP is **100% complete**, not 65% as stated in `mvp-roadmap.md`. Several documents contain outdated information about feature status, test counts, and implementation state.

---

## Critical Staleness

### 1. `docs/mvp-roadmap.md` — MAJOR STALENESS

**Last Updated (claimed)**: October 30, 2025  
**Actual state as of January 2025**: Very outdated

| Claim | Reality |
|-------|---------|
| "Overall MVP Progress: ~65% Complete" | **100% complete** |
| "543 passing tests" | **1360+ passing tests** |
| Comparison operators "Not started" | ✅ Fully implemented |
| Maps evaluation "Not started" | ✅ Fully implemented |
| Call/Lambda evaluation "Not started" | ✅ Fully implemented |
| Pattern matching "implicit only" | ✅ Full `match` expression with exhaustiveness checking |
| `in`/`not in` operators not mentioned | ✅ Fully implemented |
| Option type not mentioned | ✅ Fully implemented with `some`/`none` |

**Recommendation**: This document needs a complete rewrite to reflect actual state. The completion table and "Remaining for MVP" section are misleading.

---

### 2. `docs/TODO.tasks.md` — SIGNIFICANT STALENESS

**Issues Found:**

| Task | Listed Status | Reality |
|------|--------------|---------|
| "Implement 'in' and 'not in' operators" (P1) | Incomplete `[ ]` | ✅ Complete |
| "Implement map indexing evaluation" (P0 CRITICAL) | Listed as "crashes" | ✅ Working |
| "Comparison operators" | Not mentioned as TODO | ✅ Complete |

**Recommendation**: Review all tasks, mark completed items with `[x]`, remove obsolete items.

---

### 3. `docs/TODO.projects.md` — MODERATE STALENESS

**Issues:**
- References "lambda-closure-implementation-plan.md" which no longer exists (or is renamed)
- Some design decisions marked as "needed" may already be resolved
- Type classes section may need update based on current implementation

**Recommendation**: Review each project's status against current codebase.

---

## Documents That Are Accurate

### ✅ `docs/melbi-lang-cheat-sheet.md`
Comprehensive and matches implementation. Documents:
- Pattern matching with `match` expressions
- Option types (`some`/`none`)
- Comparison operators
- `in`/`not in` membership operators
- Lambdas and function calls
- Maps with various key types

### ✅ `CLAUDE.md`
Current coding guidelines, testing rules, and useful commands.

### ✅ `docs/pattern-matching-plan.md`
Marked as "Completed: 2025-11-16" — accurately reflects implementation status.

---

## Documents Needing Review

These may contain stale information but require deeper investigation:

| Document | Concern |
|----------|---------|
| `docs/design/public-api.md` | May describe planned vs implemented API |
| `docs/design/standard-library.md` | Need to verify against actual FFI/builtins |
| `docs/design/effects.md` | Verify effect tracking implementation status |
| `docs/design/error-handling.md` | Verify against current error types |
| `docs/CONTRIBUTING.md` | May reference outdated workflow |
| `core/src/vm/vm_architecture.md` | Bytecode VM is post-MVP; verify it's clearly marked as future |
| `core/src/vm/README.md` | Same as above |

---

## Missing Documentation

### Crate READMEs
Most crates lack a README.md:

| Crate | Has README? |
|-------|-------------|
| `core/` | ✅ |
| `cli/` | ✅ |
| `parser/` | ✅ |
| `types/` | ✅ |
| `values/` | ✅ |
| `fmt/` | ✅ |
| `lsp/` | ✅ |
| `macros/` | ✅ |
| `playground/` | ✅ |
| `playground/worker/` | ❌ |
| `support/small-str/` | ❌ |
| `support/teeny-vec/` | ✅ |
| `support/thin-ref/` | ❌ |
| `vscode/` | ✅ |
| `zed/` | ✅ |

### Root README
The repository root lacks a `README.md` with:
- Project overview
- Quick start guide
- Links to documentation
- Build instructions

---

## Next Steps

1. **Immediate**: Fix critical staleness in `mvp-roadmap.md` and `TODO.tasks.md`
2. **Short-term**: Create crate READMEs (start with `core/`, `parser/`, `cli/`)
3. **Short-term**: Create root `README.md`
4. **Medium-term**: Review design docs against implementation
5. **Ongoing**: Add "Last verified" dates to documents

---

## Methodology

Verification performed by:
1. Running `cargo test --workspace` — 1360+ tests pass
2. Examining `core/src/evaluator/` for feature implementations
3. Reading `docs/melbi-lang-cheat-sheet.md` as ground truth for syntax
4. Checking for comparison, pattern matching, and lambda code in evaluator
5. Cross-referencing claims in roadmap against actual test files
