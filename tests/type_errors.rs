/*
 * Type Error Reporting Tests
 *
 * Tests that verify error messages, span accuracy, and provenance tracking
 * for various type checking scenarios.
 */

mod cases;

test_case! {
    name: if_condition_must_be_boolean,
    input: "if 1 then 0 else 0",
    error: { r"
[E001] Error: Type mismatch: expected Bool, found Int
   ╭─[ <unknown>:1:4 ]
   │
 1 │ if 1 then 0 else 0
   │    ┬
   │    ╰── Type mismatch: expected Bool, found Int
   │
   │ Help: Condition of 'if' must be Bool
───╯
".trim_start() },
}

test_case! {
    name: otherwise_branch_type_mismatch,
    input: r#"123 + b + (1/0 otherwise "") where { b = 10 }"#,
    error: { r#"
[E001] Error: Type mismatch: expected Int, found Str
   ╭─[ <unknown>:1:26 ]
   │
 1 │ 123 + b + (1/0 otherwise "") where { b = 10 }
   │                          ─┬
   │                           ╰── Type mismatch: expected Int, found Str
   │
   │ Help: Types must match in this context
───╯
"#.trim_start() },
}

test_case! {
    name: undefined_variable,
    input: "x + 1",
    error: { r"
[E002] Error: Undefined variable 'x'
   ╭─[ <unknown>:1:1 ]
   │
 1 │ x + 1
   │ ┬
   │ ╰── Undefined variable 'x'
   │
   │ Help: Make sure the variable is declared before use
───╯
".trim_start() },
}

test_case! {
    name: numeric_operation_on_bool,
    input: "true + false",
    error: { r"
[E005] Error: Type 'Bool' does not implement Numeric
   ╭─[ <unknown>:1:1 ]
   │
 1 │ true + false
   │ ──────┬─────
   │       ╰─────── Type 'Bool' does not implement Numeric
   │
   │ Help 1: Numeric is required for arithmetic operations (+, -, *, /, ^)
   │
   │ Help 2: Numeric is implemented for: Int, Float
───╯
".trim_start() },
}

test_case! {
    name: duplicate_binding_in_where,
    input: "x where { x = 1, x = 2 }",
    error: { r"
[E016] Error: Duplicate binding name 'x'
   ╭─[ <unknown>:1:1 ]
   │
 1 │ x where { x = 1, x = 2 }
   │ ────────────┬───────────
   │             ╰───────────── Duplicate binding name 'x'
   │
   │ Help: Each binding in a where clause must have a unique name
───╯
".trim_start() },
}

test_case! {
    name: if_branch_type_mismatch,
    input: r#"if true then 1 else "hello""#,
    error: { r#"
[E001] Error: Type mismatch: expected Int, found Str
   ╭─[ <unknown>:1:21 ]
   │
 1 │ if true then 1 else "hello"
   │                     ───┬───
   │                        ╰───── Type mismatch: expected Int, found Str
   │
   │ Help: Types must match in this context
───╯
"#.trim_start() },
}

test_case! {
    name: unary_negation_on_bool,
    input: "-true",
    error: { r"
[E005] Error: Type 'Bool' does not implement Numeric
   ╭─[ <unknown>:1:1 ]
   │
 1 │ -true
   │ ──┬──
   │   ╰──── Type 'Bool' does not implement Numeric
   │
   │ Help 1: Numeric is required for arithmetic operations (+, -, *, /, ^)
   │
   │ Help 2: Numeric is implemented for: Int, Float
───╯
".trim_start() },
}

test_case! {
    name: logical_not_on_int,
    input: "not 42",
    error: { r"
[E001] Error: Type mismatch: expected Bool, found Int
   ╭─[ <unknown>:1:5 ]
   │
 1 │ not 42
   │     ─┬
   │      ╰── Type mismatch: expected Bool, found Int
   │
   │ Help: Operand of 'not' must be Bool
───╯
".trim_start() },
}

test_case! {
    name: mixed_type_arithmetic,
    input: "1 + 2.5",
    error: { r"
[E001] Error: Type mismatch: expected Int, found Float
   ╭─[ <unknown>:1:5 ]
   │
 1 │ 1 + 2.5
   │     ─┬─
   │      ╰─── Type mismatch: expected Int, found Float
   │
   │ Help: Types must match in this context
───╯
".trim_start() },
}

test_case! {
    name: duplicate_lambda_parameter,
    input: "(x, x) => x + 1",
    error: { r"
[E015] Error: Duplicate parameter name 'x'
   ╭─[ <unknown>:1:1 ]
   │
 1 │ (x, x) => x + 1
   │ ───────┬───────
   │        ╰───────── Duplicate parameter name 'x'
   │
   │ Help: Each parameter must have a unique name
───╯
".trim_start() },
}

test_case! {
    name: ordering_comparison_on_bool,
    input: "lt(false, true) where { lt = (a, b) => a < b }",
    error: { r"
[E005] Error: Type 'Bool' does not implement Ord
   ╭─[ <unknown>:1:40 ]
   │
 1 │ lt(false, true) where { lt = (a, b) => a < b }
   │ ─┬                                     ──┬──
   │  ╰──────────────────────────────────────────── when instantiated here
   │                                          │
   │                                          ╰──── Type 'Bool' does not implement Ord
   │
   │ Help 1: Ord is required for comparison operations (<, >, <=, >=)
   │
   │ Help 2: Ord is implemented for: Int, Float, Str, Bytes
───╯
".trim_start() },
}

test_case! {
    name: numeric_operation_on_bool_polymorphic,
    input: "f(false, true) where { f = (a, b) => a + b }",
    error: { r"
[E005] Error: Type 'Bool' does not implement Numeric
   ╭─[ <unknown>:1:38 ]
   │
 1 │ f(false, true) where { f = (a, b) => a + b }
   │ ┬                                    ──┬──
   │ ╰─────────────────────────────────────────── when instantiated here
   │                                        │
   │                                        ╰──── Type 'Bool' does not implement Numeric
   │
   │ Help 1: Numeric is required for arithmetic operations (+, -, *, /, ^)
   │
   │ Help 2: Numeric is implemented for: Int, Float
───╯
".trim_start() },
}

test_case! {
    name: match_pattern_type_mismatch,
    input: r#"1 match { 1 -> 2, "foo" -> 10 }"#,
    error: { r#"
[E001] Error: Type mismatch: expected Int, found Str
   ╭─[ <unknown>:1:1 ]
   │
 1 │ 1 match { 1 -> 2, "foo" -> 10 }
   │ ───────────────┬───────────────
   │                ╰───────────────── Type mismatch: expected Int, found Str
   │
   │ Help: Pattern literal must match the type of the matched expression
───╯
"#.trim_start() },
}

test_case! {
    name: function_call_argument_type_mismatch,
    input: r#"f(1, "foo") where { f = (a, b) => a + b }"#,
    error: { r#"
[E001] Error: Type mismatch: expected Str, found Int
   ╭─[ <unknown>:1:1 ]
   │
 1 │ f(1, "foo") where { f = (a, b) => a + b }
   │ ─────┬─────
   │      ╰─────── Type mismatch: expected Str, found Int
   │
   │ Help: Types must match in this context
───╯
"#.trim_start() },
}

test_case! {
    name: fails_indexable_constraint_polymorphic,
    input: r"f([1, 2, 3], false) where { f = (container, index) => container[index] }",
    error: { r"
[E005] Error: Indexable constraint not satisfied for 'Array[Int]': array indexing requires Int index, found Bool
   ╭─[ <unknown>:1:55 ]
   │
 1 │ f([1, 2, 3], false) where { f = (container, index) => container[index] }
   │ ┬                                                     ────────┬───────
   │ ╰─────────────────────────────────────────────────────────────────────── when instantiated here
   │                                                               │
   │                                                               ╰───────── Indexable constraint not satisfied for 'Array[Int]': array indexing requires Int index, found Bool
   │
   │ Help 1: Indexable is required for indexing operations (value[index])
   │
   │ Help 2: Indexable is implemented for: Array, Map, Bytes
───╯
".trim_start() },
}

test_case! {
    name: fails_map_indexable_constraint_polymorphic,
    input: r#"f({"a": 1}, 123) where { f = (m, k) => m[k] }"#,
    error: { r#"
[E005] Error: Indexable constraint not satisfied for 'Map[Str, Int]': map indexing requires Str key, found Int
   ╭─[ <unknown>:1:40 ]
   │
 1 │ f({"a": 1}, 123) where { f = (m, k) => m[k] }
   │ ┬                                      ──┬─
   │ ╰──────────────────────────────────────────── when instantiated here
   │                                          │
   │                                          ╰─── Indexable constraint not satisfied for 'Map[Str, Int]': map indexing requires Str key, found Int
   │
   │ Help 1: Indexable is required for indexing operations (value[index])
   │
   │ Help 2: Indexable is implemented for: Array, Map, Bytes
───╯
"#.trim_start() },
}

test_case! {
    name: fails_containable_not_implemented,
    input: "((x) => 1 in x)(true)",
    error: { r"
[E005] Error: Type 'Bool' does not implement Containable
   ╭─[ <unknown>:1:9 ]
   │
 1 │ ((x) => 1 in x)(true)
   │         ───┬──
   │            ╰──── Type 'Bool' does not implement Containable
   │
   │ Help 1: Containable is required for containment operations (in, not in)
   │
   │ Help 2: Containable is implemented for: (Str, Str), (Bytes, Bytes), (element, Array), (key, Map)
───╯
".trim_start() },
}

test_case! {
    name: fails_containable_constraint_polymorphic,
    input: r#"f([1, 2, 3], "hello") where { f = (arr, x) => x in arr }"#,
    error: { r#"
[E005] Error: Containable constraint not satisfied for 'Array[Int]': array containment requires Int element, found Str
   ╭─[ <unknown>:1:47 ]
   │
 1 │ f([1, 2, 3], "hello") where { f = (arr, x) => x in arr }
   │ ┬                                             ────┬───
   │ ╰─────────────────────────────────────────────────────── when instantiated here
   │                                                   │
   │                                                   ╰───── Containable constraint not satisfied for 'Array[Int]': array containment requires Int element, found Str
   │
   │ Help 1: Containable is required for containment operations (in, not in)
   │
   │ Help 2: Containable is implemented for: (Str, Str), (Bytes, Bytes), (element, Array), (key, Map)
───╯
"#.trim_start() },
}

test_case! {
    name: fails_nested_polymorphic_instantiation_chain,
    input: r"h([1,2,3], false) where { f = (container, index) => container[index], g = (c, i) => f(c, i), h = (x, y) => g(x, y) }",
    error: { r"
[E005] Error: Indexable constraint not satisfied for 'Array[Int]': array indexing requires Int index, found Bool
   ╭─[ <unknown>:1:53 ]
   │
 1 │ h([1,2,3], false) where { f = (container, index) => container[index], g = (c, i) => f(c, i), h = (x, y) => g(x, y) }
   │ ┬                                                   ────────┬───────                ┬                      ┬
   │ ╰───────────────────────────────────────────────────────────────────────────────────────────────────────────── when instantiated here
   │                                                             │                       │                      │
   │                                                             ╰───────────────────────────────────────────────── Indexable constraint not satisfied for 'Array[Int]': array indexing requires Int index, found Bool
   │                                                                                     │                      │
   │                                                                                     ╰───────────────────────── when instantiated here
   │                                                                                                            │
   │                                                                                                            ╰── when instantiated here
   │
   │ Help 1: Indexable is required for indexing operations (value[index])
   │
   │ Help 2: Indexable is implemented for: Array, Map, Bytes
───╯
".trim_start() },
}

test_case! {
    name: fails_nested_polymorphic_instantiation_chain_multiline,
    input: r"
h([1,2,3], false)
where {
    f = (container, index) => container[index],
    g = (c, i) => f(c, i),
    h = (x, y) => g(x, y),
}".trim_start(),
    error: { r"
[E005] Error: Indexable constraint not satisfied for 'Array[Int]': array indexing requires Int index, found Bool
   ╭─[ <unknown>:3:31 ]
   │
 3 │     f = (container, index) => container[index],
   │                               ────────┬───────
   │                                       ╰───────── Indexable constraint not satisfied for 'Array[Int]': array indexing requires Int index, found Bool
 4 │     g = (c, i) => f(c, i),
   │                   ┬
   │                   ╰── when instantiated here
 5 │     h = (x, y) => g(x, y),
   │                   ┬
   │                   ╰── when instantiated here
   │
   ├─[ <unknown>:3:31 ]
   │
 1 │ h([1,2,3], false)
   │ ┬
   │ ╰── when instantiated here
   │
   │ Help 1: Indexable is required for indexing operations (value[index])
   │
   │ Help 2: Indexable is implemented for: Array, Map, Bytes
───╯
".trim_start() },
}
