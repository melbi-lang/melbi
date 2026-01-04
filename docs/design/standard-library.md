---
title: Standard Library Design
---

# Design Doc: Standard Library

**Author**: @NiltonVolpato (with Claude assistance)

**Date**: 2025-01-17

## Introduction

### Background

Melbi is a statically-typed embedded expression language designed for safe evaluation of user-provided expressions in sandboxed environments. While the core language provides primitives (integers, floats, strings, arrays, records, maps, options) and control flow (if/else, where, match), it lacks a standard library of common operations.

Users need:
- Mathematical operations (trigonometry, rounding, etc.)
- String manipulation (upper, lower, split, etc.)
- Array transformations (map, filter, fold)
- Pattern matching with regex
- Statistical calculations
- Type-safe utilities for Option types

### Current Functionality

Currently, Melbi has:
- Basic arithmetic operators (`+`, `-`, `*`, `/`, `^`, `%`)
- Comparison operators (`==`, `!=`, `<`, `>`, `<=`, `>=`)
- Logical operators (`and`, `or`, `not`)
- Membership operators (`in`, `not in`)
- Fallback operator (`otherwise`)
- Type casting (`as`)
- No standard library functions

### In Scope

This design covers:
- Core standard library packages: Math, String, Array, Map, Option, Stats, Regex, Bytes
- Naming conventions for built-in functions vs user-defined functions
- Implementation strategy (FFI vs pure Melbi)
- Phased rollout plan
- Future extensibility (pipe operator)

### Out of Scope

- Date/Time package (requires custom types - not yet implemented)
- Method call syntax (decision: not implementing)
- Module system / user-defined packages
- I/O operations (intentionally excluded for sandboxing)
- Filesystem access
- Network operations
- Random number generation (non-deterministic)

### Assumptions & Dependencies

- **FFI system exists**: Can call Rust functions from Melbi
- **Public API complete**: Can register functions in the environment
- **Arena allocation**: All values use arena allocator for performance
- **Type inference works**: Functions can have generic types
- **Pattern matching works**: Option utilities rely on match expressions

### Terminology

- **Package**: A record containing functions and constants (e.g., `Math`, `String`)
- **FFI**: Foreign Function Interface - calling Rust code from Melbi
- **Built-in function**: Function implemented in Rust and registered via FFI
- **Pure Melbi function**: Function written in Melbi code itself (no FFI)
- **Pipe operator**: Future syntax `|>` for chaining function calls

## Considerations

### Concerns

1. **Performance**: FFI calls have overhead - need to benchmark
2. **Error handling**: How do functions report errors? Return Option? Panic?
3. **Type safety**: Generic functions need proper type checking
4. **API stability**: Once released, hard to change function signatures
5. **Discoverability**: How do users learn what's available?
6. **Documentation**: Each function needs clear docs with examples
7. **Testing**: Comprehensive test suite for all standard library functions

### Operational Readiness Considerations

- **Deployment**: Standard library ships with Melbi core
- **Versioning**: Need semantic versioning for breaking changes
- **Deprecation**: Strategy for removing/changing functions
- **Performance monitoring**: Track FFI call overhead
- **Testing**: Unit tests for all functions, integration tests for common patterns

### Open Questions

1. How do FFI functions respect the arithmetic mode (wrapping vs checked)? See `numeric-safety.md`
2. Should regex patterns be compiled and cached, or recompiled each time?
3. Should `Math.Sqrt(-1)` return `NaN` (current behavior, IEEE 754) or use error effect? (Leaning toward `NaN` for simplicity)
4. Regex literal syntax: `/pattern/` (JavaScript-style) or `"pattern"Regex` (suffix-style) when we implement it?

### Cross-Region Considerations

N/A - Melbi is an embedded language, not a distributed system.

## Proposed Design

### Solution

The standard library will be implemented as **packages** - records containing functions and constants. Packages are pre-populated in the global environment before user code executes.

**Key design decisions:**
1. **Packages are just records** - No special syntax or type system changes
2. **Capitalized naming** - Built-in functions use `UpperCamelCase` (e.g., `Math.Sin`)
3. **FFI for most packages** - Implemented in Rust for performance
4. **Pure Melbi for Option** - Option utilities can be written in Melbi itself
5. **No method syntax** - Use function calls: `String.Upper(s)` not `s.Upper()`

### System Architecture

```
┌─────────────────┐
│  User Melbi Code│
│                 │
│  Math.Sin(x)    │
│  String.Upper() │
└────────┬────────┘
         │
         ├──> Pure Melbi Packages (Option)
         │    - Compiled to bytecode
         │    - No FFI overhead
         │
         └──> FFI Packages (Math, String, etc.)
              - Rust implementations
              - Registered at startup
              - Type-checked at compile time
```

### Data Model

**Package Structure:**
```melbi
// Example package (conceptual)
Math.Sin(Math.PI)
where {
    Math = {
        // Constants
        PI = 3.14159265359,
        E = 2.71828182846,
        TAU = 6.28318530718,
        
        // Functions
        Sin = <builtin function (Float) => Float>,
        Cos = <builtin function (Float) => Float>,
        Abs = <builtin function (Int) => Int>,  // Overloaded
        Abs = <builtin function (Float) => Float>,
        // etc.
    }
}
```

**FFI Registration (Rust side):**
```rust
// Each package has its own registration function (modular)
pub fn register_math_package(env: &mut Environment) {
    let math_pkg = Record::builder()
        .add_constant("PI", RawValue::from_float(std::f64::consts::PI))
        .add_constant("E", RawValue::from_float(std::f64::consts::E))
        .add_function("Sin", math_sin)
        .add_function("Cos", math_cos)
        .build();
    env.register("Math", math_pkg);
}

pub fn register_string_package(env: &mut Environment) {
    let string_pkg = Record::builder()
        .add_function("Upper", string_upper)
        .add_function("Lower", string_lower)
        .build();
    env.register("String", string_pkg);
}

// Convenience function to register all core packages (minimal binary size)
pub fn register_all_stdlib(env: &mut Environment) {
    register_math_package(env);
    register_string_package(env);
    register_array_package(env);
    register_map_package(env);
    register_stats_package(env);
    register_regex_package(env);
    register_bytes_package(env);
}

// Optional: Register Unicode package for full Unicode support (~100-200KB)
pub fn register_all_stdlib_with_unicode(env: &mut Environment) {
    register_all_stdlib(env);
    register_unicode_package(env);  // Opt-in for Unicode support
}

// Users can cherry-pick packages:
// env.register_math_package();
// env.register_string_package();
// env.register_unicode_package();  // Only if Unicode support needed
```

### Interface / API Definitions

## Package: `Math`

**Constants:**
```melbi
Math.PI: Float        // 3.14159265359
Math.E: Float         // 2.71828182846
Math.TAU: Float       // 6.28318530718 (2π)
Math.INFINITY: Float  // Positive infinity
Math.NAN: Float       // Not a number
```

**Functions:**
```melbi
// Basic operations (polymorphic with Numeric type-class)
Math.Abs(x: T) => T where T: Numeric
Math.Min(a: T, b: T) => T where T: Numeric
Math.Max(a: T, b: T) => T where T: Numeric
Math.Clamp(value: T, min: T, max: T) => T where T: Numeric

// Rounding
Math.Floor(x: Float) => Int
Math.Ceil(x: Float) => Int
Math.Round(x: Float) => Int

// Exponentiation
Math.Sqrt(x: Float) => Float
Math.Pow(base: Float, exp: Float) => Float

// Trigonometry
Math.Sin(x: Float) => Float
Math.Cos(x: Float) => Float
Math.Tan(x: Float) => Float
Math.Asin(x: Float) => Float
Math.Acos(x: Float) => Float
Math.Atan(x: Float) => Float
Math.Atan2(y: Float, x: Float) => Float

// Logarithms
Math.Log(x: Float) => Float      // Natural log
Math.Log10(x: Float) => Float    // Base 10
Math.Exp(x: Float) => Float      // e^x
```

## Package: `String`

**Note:** String operations are designed for minimal binary size. Case operations (`Upper`, `Lower`) are **ASCII-only** to avoid including large Unicode case-folding tables (~100-200KB). For full Unicode support, use the optional `Unicode` package (see Phase 3).

**Functions:**
```melbi
// Inspection
String.Len(s: String) => Int            // Number of UTF-8 codepoints (not bytes)
String.IsEmpty(s: String) => Bool
String.Contains(haystack: String, needle: String) => Bool
String.StartsWith(s: String, prefix: String) => Bool
String.EndsWith(s: String, suffix: String) => Bool

// Transformation (ASCII-only for minimal binary size)
String.Upper(s: String) => String       // ASCII-only: 'a'-'z' → 'A'-'Z'
String.Lower(s: String) => String       // ASCII-only: 'A'-'Z' → 'a'-'z'
String.Trim(s: String) => String
String.TrimStart(s: String) => String
String.TrimEnd(s: String) => String
String.Replace(s: String, from: String, to: String) => String
String.ReplaceN(s: String, from: String, to: String, count: Int) => String

// Splitting and joining
String.Split(s: String, delimiter: String) => Array[String]
String.Join(parts: Array[String], separator: String) => String

// Extraction
String.Substring(s: String, start: Int, end: Int) => String

// Parsing
String.ToInt(s: String) => Option[Int]      // Parse string to integer
String.ToFloat(s: String) => Option[Float]  // Parse string to float
```

**Design Notes:**
- `String.Chars()` is deliberately omitted. Most character-level operations are better handled by **Regex** (pattern-based), **Unicode.GraphemeClusters()** (when you need an array), or direct string operations.
- `String.FromInt()` and `String.FromFloat()` are deliberately omitted. Use Melbi's built-in format strings instead: `f"{value}"` or `f"{price:.2f}"`. Format strings are part of the language syntax and provide full formatting control without needing library functions.

## Package: `Array`

**Functions:**
```melbi
// Inspection
Array.Len(arr: Array[T]) => Int
Array.IsEmpty(arr: Array[T]) => Bool
Array.Contains(arr: Array[T], item: T) => Bool

// Transformation
Array.Map(arr: Array[T], fn: (T) => U) => Array[U]
Array.Filter(arr: Array[T], predicate: (T) => Bool) => Array[T]
Array.Fold(arr: Array[T], initial: U, fn: (U, T) => U) => U
Array.Reduce(arr: Array[T], fn: (T, T) => T) => Option[T]

// Extraction
Array.First(arr: Array[T]) => Option[T]
Array.Last(arr: Array[T]) => Option[T]
Array.Get(arr: Array[T], index: Int) => Option[T]
Array.Slice(arr: Array[T], start: Int, end: Int) => Array[T]

// Combination
Array.Concat(a: Array[T], b: Array[T]) => Array[T]
Array.Flatten(arr: Array[Array[T]]) => Array[T]
Array.Zip(a: Array[T], b: Array[U]) => Array[Record[first: T, second: U]]

// Ordering
Array.Sort(arr: Array[T]) => Array[T]  // where T is comparable
Array.SortBy(arr: Array[T], key: (T) => U) => Array[T]
Array.Reverse(arr: Array[T]) => Array[T]

// Searching
Array.Find(arr: Array[T], predicate: (T) => Bool) => Option[T]
Array.FindIndex(arr: Array[T], predicate: (T) => Bool) => Option[Int]
Array.Any(arr: Array[T], predicate: (T) => Bool) => Bool
Array.All(arr: Array[T], predicate: (T) => Bool) => Bool

// Note: Sum, Min, Max are in Stats package, not Array
```

## Package: `Map`

**Functions:**
```melbi
// Inspection
Map.Len(map: Map[K, V]) => Int
Map.IsEmpty(map: Map[K, V]) => Bool
// Note: Use `key in map` operator instead of Map.HasKey

// Access
Map.Keys(map: Map[K, V]) => Array[K]
Map.Values(map: Map[K, V]) => Array[V]
Map.Entries(map: Map[K, V]) => Array[(K, V)]
// Note: Use `map[key] otherwise default` instead of Map.Get

// Transformation
Map.MapValues(m: Map[K, V], fn: (V) => U) => Map[K, U]
Map.FilterKeys(m: Map[K, V], predicate: (K) => Bool) => Map[K, V]
Map.FilterValues(m: Map[K, V], predicate: (V) => Bool) => Map[K, V]

// Combination
Map.Merge(a: Map[K, V], b: Map[K, V]) => Map[K, V]  // b overwrites a
```

## Package: `Option`

**Functions:**
```melbi
expression
where {
  Option = {
      // Unwrapping
      UnwrapOr = (opt: Option[T], default: T) => T {
          opt match {
              some value -> value,
              none -> default,
          }
      },
      
      // Transformation
      Map = (opt: Option[T], fn: (T) => U) => Option[U] {
          opt match {
              some value -> some (fn(value)),
              none -> none,
          }
      },
      
      AndThen = (opt: Option[T], fn: (T) => Option[U]) => Option[U] {
          opt match {
              some value -> fn(value),
              none -> none,
          }
      },
      
      Or = (a: Option[T], b: Option[T]) => Option[T] {
          a match {
              some _ -> a,
              none -> b,
          }
      },
      
      // Inspection
      IsSome = (opt: Option[T]) => Bool {
          opt match {
              some _ -> true,
              none -> false,
          }
      },
      
      IsNone = (opt: Option[T]) => Bool {
          opt match {
              some _ -> false,
              none -> true,
          }
      },
  }
}
```

## Package: `Stats`

**Functions:**
```melbi
// Basic aggregation
Stats.Sum(arr: Array[Int]) => Int
Stats.Sum(arr: Array[Float]) => Float
Stats.Mean(arr: Array[Float]) => Option[Float]  // None if empty
Stats.Median(arr: Array[Float]) => Option[Float]
Stats.Mode(arr: Array[T]) => Option[T]  // Most frequent

// Range
Stats.Min(arr: Array[T]) => Option[T]
Stats.Max(arr: Array[T]) => Option[T]
Stats.Range(arr: Array[Float]) => Option[Float]  // Max - Min

// Variance and standard deviation
Stats.Variance(arr: Array[Float]) => Option[Float]
Stats.StdDev(arr: Array[Float]) => Option[Float]

// Counting
Stats.Count(arr: Array[T], predicate: (T) => Bool) => Int
Stats.CountUnique(arr: Array[T]) => Int
```

## Package: `Regex`

**Functions:**
```melbi
// Matching (text-first for pipe operator compatibility)
Regex.Matches(text: String, pattern: String) => Bool
Regex.IsMatch(text: String, pattern: String) => Bool  // Alias

// Extraction (text-first)
Regex.Extract(text: String, pattern: String) => Option[String]  // First match
Regex.ExtractAll(text: String, pattern: String) => Array[String]
Regex.Captures(text: String, pattern: String) => Option[Array[String]]  // Capture groups

// Replacement (text-first, supports string or function replacement)
Regex.Replace(text: String, pattern: String, replacement: String) => String
Regex.Replace(text: String, pattern: String, fn: (String) => String) => String
Regex.ReplaceAll(text: String, pattern: String, replacement: String) => String
Regex.ReplaceAll(text: String, pattern: String, fn: (String) => String) => String

// Splitting (text-first)
Regex.Split(text: String, pattern: String) => Array[String]
```

**Note:** Regex patterns are compiled at runtime. Invalid patterns should return `none` or empty results rather than panicking. Text-first argument order enables pipe operator usage: `text |> Regex.Replace("[0-9]+", "NUM")`.

## Package: `Bytes`

**Functions:**
```melbi
// Inspection
Bytes.Len(b: Bytes) => Int              // Number of bytes (not codepoints)
Bytes.IsEmpty(b: Bytes) => Bool

// Conversion
Bytes.ToString(b: Bytes) => String      // UTF-8 decode
Bytes.FromString(s: String) => Bytes    // UTF-8 encode
Bytes.ToHex(b: Bytes) => String
Bytes.FromHex(s: String) => Option[Bytes]

// Combination
Bytes.Concat(a: Bytes, b: Bytes) => Bytes

// Extraction
Bytes.Slice(b: Bytes, start: Int, end: Int) => Bytes
```

**Note:** For byte length of a string, use: `Bytes.Len(string as Bytes)`

## Package: `Unicode` (Optional)

**Note:** This package adds ~100-200KB to the binary due to Unicode case-folding tables and normalization data. Only register this package if full Unicode support is needed. For ASCII-only operations, use the `String` package instead.

**Functions:**
```melbi
// Case conversion (Full Unicode support)
Unicode.Upper(s: String) => String      // Handles all Unicode: 'é' → 'É', 'ß' → 'SS'
Unicode.Lower(s: String) => String      // Handles all Unicode: 'Σ' → 'σ'

// Normalization
Unicode.Normalize(s: String, form: String) => String
// Forms: "NFC" (canonical composition), "NFD" (canonical decomposition)
//        "NFKC" (compatibility composition), "NFKD" (compatibility decomposition)

// Display and width
Unicode.Width(s: String) => Int         // Display width for terminal output
Unicode.GraphemeClusters(s: String) => Array[String]  // Proper character boundaries
```

**Usage Note:** Only include this package when internationalization is required:
```rust
// Minimal runtime (no Unicode package)
let string = build_string_package(arena, type_mgr)?;
env.register("String", string)?;

// Full Unicode support (opt-in)
let unicode = build_unicode_package(arena, type_mgr)?;
env.register("Unicode", unicode)?;
```

### Business Logic

**Type Polymorphism:**
Functions like `Math.Abs`, `Math.Min`, `Math.Max` need to work with both Int and Float. This is achieved through type-class polymorphism:
```melbi
Math.Abs(x: T) => T where T: Numeric
```

The `Numeric` type-class includes Int and Float. The type system automatically selects the correct implementation based on the argument type.

**No Panics Policy:**
Standard library functions **MUST NOT panic**. Instead:

1. **Missing/invalid values**: Return `Option[T]` (e.g., `String.ToInt`, `Array.First`, `Stats.Mean`)
2. **Invalid operations that signal errors**: Use the `!` effect system (for operations that can fail in exceptional ways)
3. **Math edge cases**: Return mathematically appropriate values:
   - `Math.Sqrt(-1.0)` → `Math.NAN` (IEEE 754 behavior)
   - Division by zero → `Math.INFINITY` or `Math.NAN` (IEEE 754 behavior)
4. **Regex errors**: Invalid patterns return `none` or empty results

**Error Handling Summary:**
- **Option for expected failures**: Parsing, indexing, searching (e.g., `String.ToInt("abc")` → `none`)
- **Effect system for exceptional failures**: Operations that should normally succeed but can fail (e.g., file operations in future packages)
- **NaN/Infinity for math edge cases**: Follow IEEE 754 semantics (e.g., `Math.Sqrt(-1.0)` → `NaN`)
- **Never panic**: All operations must be safe

**The `otherwise` operator:**
- Used for **error handling** with the `!` effect, NOT for Option types
- Example: `risky_operation() otherwise default_value`
- For Options, use pattern matching: `opt match { some x -> x, none -> default }`

**Performance Considerations:**
- **Regex compilation**: Cache compiled regexes? Or recompile each time? (Open question)
- **String operations**: UTF-8 aware (proper character handling)
- **Array operations**: Eager evaluation (e.g., `Array.Map` immediately creates new array; future Views will enable lazy field access)

**Binary Size Considerations:**
- **Minimal by default**: Core packages (String, Array, Math, etc.) avoid large dependencies
- **ASCII-only strings**: `String.Upper`/`String.Lower` use ASCII-only operations to avoid Unicode tables (~100-200KB)
- **Optional packages**: Unicode support is opt-in via separate `Unicode` package
- **No feature flags**: Users control binary size by choosing which packages to register, not through compile-time features
- **Package-based approach**: Better than feature flags because it's explicit in user code and doesn't require recompilation

### Migration Strategy

N/A - This is a new feature, not a migration.

### Work Required

**Phase 1: Foundation (MVP - 2-3 weeks)**
- [ ] FFI system implementation
- [ ] Public API for registering functions
- [ ] Pure Melbi Option package
- [ ] Math basics: Abs, Min, Max, Floor, Ceil, Round, constants
- [ ] String basics: Upper, Lower, Trim, Split, Join, Len, Contains
- [ ] Array basics: Map, Filter, Len, First, Last, Contains

**Phase 2: High-Value Features (2-3 weeks)**
- [ ] Regex package: Matches, Extract, Replace
- [ ] Stats package: Sum, Mean, Min, Max
- [ ] More String ops: Replace, Substring, StartsWith, EndsWith
- [ ] More Array ops: Fold, Concat, Flatten, Find

**Phase 3: Advanced Features (2-3 weeks)**
- [ ] Advanced Array: Sort, SortBy, Reverse, Zip
- [ ] Map package: All operations
- [ ] Bytes package: All operations
- [ ] Unicode package: Full Unicode support (optional, ~100-200KB)
- [ ] Advanced Math: Trigonometry, logarithms
- [ ] Advanced Stats: Median, Variance, StdDev

**Future (Post-MVP):**
- [ ] Pipe operator `|>` implementation
- [ ] Date/Time package (requires custom types)
- [ ] Performance optimizations
- [ ] Comprehensive documentation

### Work Sequence

1. **FFI infrastructure** → **Option package** → **Test**
2. **Math package (Phase 1)** → **Test**
3. **String package (Phase 1)** → **Test**
4. **Array package (Phase 1)** → **Test**
5. **Regex package** → **Test**
6. **Stats package** → **Test**
7. Continue with Phase 2 and 3...

### High-level Test Plan

**Unit Tests (Rust):**
- Test each FFI function in isolation
- Edge cases: empty arrays, null bytes, invalid regex
- Type safety: ensure proper type checking

**Integration Tests (Melbi):**
- Real-world usage patterns
- Function composition
- Performance benchmarks

**Example Test:**
```melbi
// Test Array.Map
result = Array.Map([1, 2, 3], (x) => x * 2)
assert(result == [2, 4, 6])

// Test chaining (with where)
result where {
    filtered = Array.Filter([1, 2, 3, 4, 5], (x) => x > 2),
    mapped = Array.Map(filtered, (x) => x * 2),
    result = Array.Sum(mapped),
}
assert(result == 24)  // (3 + 4 + 5) * 2 = 24
```

### Deployment Sequence

1. Merge FFI infrastructure
2. Merge Option package (pure Melbi)
3. Merge Phase 1 packages (Math, String, Array basics)
4. Merge Phase 2 (Regex, Stats)
5. Merge Phase 3 (Advanced features)

Each merge should include documentation updates and examples.

## Impact

### Performance Impact

**Positive:**
- FFI functions are Rust-native (very fast)
- No overhead vs hand-written Rust code
- Pure Melbi Option has zero FFI overhead

**Negative:**
- FFI call overhead (minimal, ~1-2ns per call)
- Regex compilation overhead (mitigated by caching)

**Mitigation:**
- Benchmark critical paths
- Cache compiled regexes
- Consider JIT for hot paths (future)

### Security Impact

**Positive:**
- All operations are sandboxed
- No I/O, filesystem, or network access
- Regex is safe (no ReDoS with proper timeout)

**Concerns:**
- Regex complexity attacks (ReDoS) - need timeout/limit
- Memory exhaustion from large arrays - need limits

### User Experience Impact

**Positive:**
- Rich standard library improves productivity
- Familiar function names (like Python/JavaScript)
- Type safety prevents errors

**Negative:**
- Learning curve for capitalized naming
- No autocomplete without IDE support (future LSP)

## Alternatives

### Alternative 1: Method Syntax

**Example:**
```melbi
arr.Map((x) => x * 2).Filter((x) => x > 5).First()
```

**Pros:** More ergonomic, chainable, familiar
**Cons:** Complex type system, parser complexity, name conflicts
**Decision:** Rejected - use function calls instead

### Alternative 2: UFCS (Uniform Function Call Syntax)

**Example:**
```melbi
// Both work
Map(arr, fn)
arr.Map(fn)
```

**Pros:** Best of both worlds
**Cons:** Ambiguity, complexity
**Decision:** Rejected - stick with one way

### Alternative 3: No Standard Library

**Example:** Users write everything themselves

**Pros:** Simplicity
**Cons:** Poor user experience, reinventing the wheel
**Decision:** Rejected - standard library is essential

## Looking into the Future

### Pipe Operator `|>`

**Future syntax:**
```melbi
result = arr
    |> Array.Filter((x) => x > 2)
    |> Array.Map((x) => x * 2)
    |> Stats.Sum()
```

**Implementation:**
- Syntactic sugar for function calls
- First argument is piped value
- Simple parser change, no type system changes
- **Decision:** Implement post-MVP

**Design note:** This is why Regex functions use text-first argument order (`Regex.Replace(text, pattern, replacement)` instead of `Regex.Replace(pattern, text, replacement)`).

### Views for Lazy Field Access

**Motivation:** When Melbi accesses external objects (via FFI), we want lazy field access without materializing all data upfront.

**Future feature:**
```melbi
// External object with many fields
user = GetUserFromDatabase(id)  // Returns a View, not a full record

// Only fetch the fields we actually use
name = user.name         // Lazy: fetches only 'name' field
email = user.email       // Lazy: fetches only 'email' field
// user.password is never accessed, so never fetched
```

**Benefits:**
- Zero-copy performance for large external objects
- Only fetch/compute what's needed
- Transparent to users (looks like normal record access)

**Decision:** Post-MVP feature, requires type system changes

### Regex Literal Syntax

**Motivation:** String-based regex patterns are verbose and require escaping.

**Future syntax options:**
```melbi
// Option 1: Slash syntax (like JavaScript)
pattern = /[0-9]+/
text |> Regex.Replace(pattern, "NUM")

// Option 2: String suffix (like Python's r"...")
pattern = "[0-9]+"Regex
text |> Regex.Replace(pattern, "NUM")
```

**Benefits:**
- More concise
- Pattern validation at compile time (syntax errors caught early)
- Potential for regex optimization

**Decision:** Post-MVP, requires parser changes

### Reflection and Introspection

**Motivation:** Users may want to inspect types and structure at runtime.

**Future package:**
```melbi
Reflect.TypeOf(value) => String          // "Int", "Array[String]", etc.
Reflect.Fields(record) => Array[String]  // Field names
Reflect.HasField(record, name) => Bool
Reflect.GetField(record, name) => Option[T]
```

**Use cases:**
- Generic serialization/deserialization
- Dynamic field access
- Debugging and introspection

**Decision:** Post-MVP, requires runtime type information

### Date/Time Package

**Blocked on:** Custom types implementation

**Future API:**
```melbi
Date.Parse("2025-01-17") => Option[Date]
Date.Format(date, "YYYY-MM-DD") => String
Date.Diff(a, b, "days") => Int
```

**Decision:** Defer until custom types exist

### Lazy Evaluation for Arrays

**Future optimization:**
```melbi
// Don't materialize intermediate arrays
Array.Range(1, 1000000)
    |> Array.Map((x) => x * 2)
    |> Array.Filter((x) => x > 10)
    |> Array.Take(5)  // Only compute first 5
```

**Decision:** Defer - eager evaluation for MVP. This overlaps with Views concept.

### User-Defined Packages

**Future feature:**
```melbi
MyUtils = {
    Double = (x) => x * 2,
    Triple = (x) => x * 3,
}
```

**Decision:** Already possible! Just assign records. No special work needed.

---

**End of Design Document**
