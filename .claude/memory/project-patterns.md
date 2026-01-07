---
name: project-patterns
description: Key design patterns and optimization techniques used in Melbi. Understand these patterns to write idiomatic code and contribute effectively.
---

# Melbi Design Patterns

## Writing Style for Docs

- Be concise: bullet points, not paragraphs
- Claude is smart - no need to over-explain
- Use "Bad/Good" pairs for patterns, not full explanations
- Inline code in bullets: `` `foo()` instead of `bar()` ``
- Skip filler words, articles where possible
- Goal: scannable, low token cost

## NonNull Pointers

- Use NonNull API directly, avoid `.as_ptr()` conversion
  - Bad: `ptr.as_ptr().add(offset).cast::<T>()`
  - Good: `ptr.add(offset).cast::<T>()`
- Use `NonNull::from_ref()` when starting from a reference (avoids unsafe)
  - Bad: `unsafe { NonNull::new_unchecked(ptr as *mut u8) }`
  - Good: `NonNull::from_ref(value).cast()`
- Use `.cast::<T>().as_ref()` instead of `&*(ptr.as_ptr() as *const T)`
- NonNull has `add()`, `sub()`, `offset()`, `write()`, `cast()` - use them

## Sealed Traits

- Use when you need trait dispatch but want to prevent external impls
- Pattern: `mod private { pub trait Sealed {} }` + `pub trait Foo: private::Sealed`
- Example: `ThinRef` uses this for different deref behavior per type (sized vs `[T]` vs `str`)
