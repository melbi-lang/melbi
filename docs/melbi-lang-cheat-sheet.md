# Melbi Language Cheat Sheet

A quick reference for Melbi's expression language syntax.

## Overview

Melbi is a type-safe, functional expression language featuring:
- **Hindley-Milner type inference** - Types inferred automatically
- **Pattern matching** with exhaustiveness checking
- **Immutable by default** - Pure functional programming
- **No null** - Uses `Option[T]` instead
- **Arena-allocated** for performance
- **Expression-based** - Everything is an expression (no statements)

---

## Literals

### Integers
```melbi
42             // Decimal
-123           // Negative
0b101010       // Binary
0o52           // Octal
0x2a           // Hexadecimal
999_999_999    // Underscores for readability
```

### Floats
```melbi
3.14           // Standard decimal
0.5            // Leading zero
.5             // No leading zero
3.             // Trailing dot
1.5e10         // Scientific notation
1.5E+10        // Uppercase E, explicit sign
1.5e-10        // Negative exponent
1_000.5_000    // Underscores for readability
```

### Booleans
```melbi
true
false
```

### Strings
```melbi
"hello"               // Double quotes
'hello'               // Single quotes
"hello\nworld"        // Escape sequences: \n \r \t \0 \\ \" \'
"unicode: \u0041"     // Unicode escape (4 hex digits)
"unicode: \U00000041" // Unicode escape (8 hex digits)
```

### Bytes
```melbi
b"hello"         // Byte string
b"hex: \x48\x65" // Hex escape sequences
```

### Format Strings
```melbi
f"Hello { name }" // Simple interpolation
f"{ x } + { y } = { x + y }" // Expressions in braces
f"Result: { result where { x = 1, y = 2, result = x + y } }" // Complex expressions
f"Literal braces: {{not interpolated}}" // {{ and }} escape braces
```

### Options
```melbi
none           // None value
some 42        // Some with value
some (some 10) // Nested Options
```

### Arrays
```melbi
[]               // Empty array
[1, 2, 3]        // Simple array
[1 + 2, 3 * 4]   // Expressions as elements
[[1, 2], [3, 4]] // Nested arrays
```

### Records
```melbi
Record{}           // Empty record
{ x = 1, y = 2 }   // Record with fields
{ a = { b = 3 } }  // Nested records
```

### Maps
```melbi
{} // Empty map
{a: 1, b: 2} // String keys (identifiers)
{1: "one", 2: "two"} // Integer keys
{"key": "value"} // String literal keys
{1 + 2: 3, 4: 5 * 6} // Expression keys and values
```

---

## Operators

### Arithmetic
```melbi
2 ^ 3 // Power (exponentiation)
5 * 6 // Multiplication
7 / 8 // Division
1 + 2 // Addition
3 - 4 // Subtraction
-5    // Unary negation
```

### Comparison
```melbi
5 == 5 // Equal
5 != 3 // Not equal
3 < 5  // Less than
10 > 5 // Greater than
5 <= 5 // Less than or equal
7 >= 3 // Greater than or equal
```

### Logical
```melbi
not true        // Logical NOT
true and false  // Logical AND
true or false   // Logical OR
```

### Membership
```melbi
5 in [1, 2, 3, 4, 5] // Element in array
"lo" in "hello"      // Substring in string
b"oob" in b"foobar"  // Bytes in bytes
key in {a: 1, b: 2}  // Key in map
5 not in [1, 2, 3]   // Negated membership
```

### Error Handling
```melbi
v[i] otherwise 0          // On error evaluates and returns fallback
x / y + z otherwise a * b // Works with complex expressions
```

### Operator Precedence (high to low)
1. Postfix: `()` `[]` `.` `as`
2. Power: `^` (right-associative)
3. Prefix: `-` `some`
4. Multiplicative: `*` `/`
5. Additive: `+` `-`
6. Comparison and membership: `==` `!=` `<` `>` `<=` `>=` `in` `not in`
7. Logical NOT (prefix): `not`
8. Logical AND: `and`
9. Logical OR: `or`
10. IF expression (prefix): `if ... then ... else`
11. Error handling: `otherwise`
12. Postfix: `where {...}` `match {...}`
13. Lambda: `(...) =>`

---

## Control Flow

### If Expressions
```melbi
if true then 1 else 2   // Basic if-else (else required)
if x > 0 then x else -x // Conditional expression

// Multi-line
if condition
then value1
else value2

// Nested
if a then if b then 1 else 2 else 3
```

### Where Bindings
```melbi
2 * x + y where { x = 1, y = 2 }  // Single-line

x + 2 * y where {                 // Multi-line
    x = 1,
    y = 3 * x + 2,
}

(a - b) / c otherwise 0
where { a = 5, b = 2, c = 3 }    // Complex expression

{ a = z, b = z + y } where { x = 2, y = 3, z = x + y } // In records
```

### Pattern Matching
```melbi
// Option patterns
value match { some x -> x * 2, none -> 0 }

// Literal patterns
x match { 1 -> "one", 2 -> "two", _ -> "other" }
flag match { true -> "yes", false -> "no" }

// Nested Option patterns
opt match { some (some x) -> x, some none -> -1, none -> 0 }

// Wildcard pattern
x match { _ -> 42 } // Matches anything

// Variable binding
x match { value -> value + 1 } // Binds x to 'value'
```

**Exhaustiveness Checking:**
- `Bool`: Must cover `true` and `false` (or wildcard)
- `Option[T]`: Must cover `some _` and `none` (or wildcard)
- Other types: Require explicit wildcard

---

## Functions

### Lambda Syntax
```melbi
(x) => x + 1     // Single parameter
(x, y) => x + y  // Multiple parameters
() => 42         // No parameters

// With where bindings
(a, b, c) => result where {
    delta = b ^ 2 - 4 * a * c,
    result = [1, 2],
}

// Nested lambdas (currying)
(x) => (y) => x + y
```

### Function Calls
```melbi
double(21) // Call with argument
add(1, 2) // Multiple arguments
func() // No arguments
```

## Packages
```melbi
Math.PI // Package-level constant
Math.Sin(Math.PI) // Uppercase package and function names
String.Trim("  hello  ") // Types usually have a corresponding package
```
---

## Postfix Operations

### Field Access
```melbi
record.field // Access record field
user.name // Example
```

### Indexing
```melbi
array[0] // Array indexing
map[key] // Map indexing
bytes[i] // Bytes indexing
```

### Type Casting
```melbi
value as Int // Cast to Int
x as Float   // Cast to Float
```

---

## Type System

### Primitive Types
```melbi
Int     // Integer
Float   // Floating point
Bool    // Boolean
Str     // UTF-8 string
Bytes   // Byte array
```

### Collection Types
```melbi
Array[T] // Homogeneous array
Map[K,V] // Key-value map
Record[field1: T1, field2:T2] // Structural record type
```

### Function Types
```melbi
(T1, T2) => R // Function from T1, T2 to R
(Int) => Int  // Example
() => Bool    // No parameters
```

### Option Type
```melbi
Option[T]    // Optional value (`some ...` or `none`)
Option[Int]  // Example: optional integer
Option[Option[T]] // Nested options allowed
```

---

## Comments

```melbi
// Single-line comment
42 // Comment after expression
{
    a = 1, // Inline comment
    b = 2,
}
```

---

## Complete Examples

### Simple Calculation
```melbi
price * quantity * (1.0 - discount) where {
    discount = if premium then 0.2 else 0.1,
}
```

### Format String with Bindings
```melbi
f"Hello { name }, your score is { score * 100 }!" where {
    name = "Alice",
    score = 0.95,
}
```

### Lambda with Pattern Matching
```melbi
(x) => x match {
    some y -> y * 2,
    none -> 0,
}
```

### Quadratic Formula
```melbi
(a, b, c) => [r0, r1] where {
    delta = b ^ 2 - 4 * a * c,
    r0 = (-b + delta ^ 0.5) / (2 * a),
    r1 = (-b - delta ^ 0.5) / (2 * a),
}
```

### Nested Structures
```melbi
{
    users = [
        { name = "Alice", age = 30 },
        { name = "Bob", age = 25 },
    ],
    messages = [
        { sender = "Alice", content = "Hi!" },
        { sender = "Bob", content = "Hello!" },
    ],
}
```

### Safe Array Access
```melbi
array[index] otherwise -1
```

---

## Escape Sequences

### Strings & Format Strings
```melbi
\n          // Newline
\r          // Carriage return
\t          // Tab
\0          // Null
\\          // Backslash
\"          // Double quote
\'          // Single quote
\           // Line continuation (backslash + newline)
\uXXXX      // Unicode (4 hex digits)
\UXXXXXXXX  // Unicode (8 hex digits)
```

### Bytes
```melbi
\xXX // Hex byte (2 hex digits)
// Plus all non-unicode escapes from strings
```

### Format Strings
```melbi
{{      // Literal {
}}      // Literal }
```

---

## Key Features

### Type Inference
- Types inferred automatically without annotations
- Polymorphic generics supported
- Type errors caught at compile time

### Exhaustiveness Checking
- Pattern matching verified at compile time
- Missing cases cause errors
- Ensures safe handling of all variants

### Immutability
- All values are immutable
- No variable reassignment (but shadowing allowed)

### No Null
- Uses `Option[T]` instead of null
- Pattern matching enforces handling both cases
- Safe by construction
