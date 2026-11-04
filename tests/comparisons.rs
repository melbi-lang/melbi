mod cases;

// ======== Equality Operators (==, !=) ========

test_case! {
    name: integer_equality,
    input: "5 == 5",
    ast: { "(Comparison Eq (Lit Int 5) (Lit Int 5))" },
    formatted: { "5 == 5" },
}

test_case! {
    name: integer_inequality,
    input: "5 != 3",
    ast: { "(Comparison Neq (Lit Int 5) (Lit Int 3))" },
    formatted: { "5 != 3" },
}

test_case! {
    name: float_equality,
    input: "3.14 == 3.14",
    formatted: { "3.14 == 3.14" },
}

test_case! {
    name: boolean_equality,
    input: "true == false",
    formatted: { "true == false" },
}

test_case! {
    name: string_equality,
    input: r#""hello" == "world""#,
    formatted: { r#""hello" == "world""# },
}

// ======== Ordering Operators (<, >, <=, >=) ========

test_case! {
    name: less_than,
    input: "3 < 5",
    ast: { "(Comparison Lt (Lit Int 3) (Lit Int 5))" },
    formatted: { "3 < 5" },
}

test_case! {
    name: greater_than,
    input: "10 > 5",
    ast: { "(Comparison Gt (Lit Int 10) (Lit Int 5))" },
    formatted: { "10 > 5" },
}

test_case! {
    name: less_than_or_equal,
    input: "5 <= 5",
    ast: { "(Comparison Le (Lit Int 5) (Lit Int 5))" },
    formatted: { "5 <= 5" },
}

test_case! {
    name: greater_than_or_equal,
    input: "7 >= 3",
    ast: { "(Comparison Ge (Lit Int 7) (Lit Int 3))" },
    formatted: { "7 >= 3" },
}

test_case! {
    name: float_less_than,
    input: "2.5 < 3.7",
    formatted: { "2.5 < 3.7" },
}

test_case! {
    name: float_greater_than,
    input: "5.5 > 2.2",
    formatted: { "5.5 > 2.2" },
}

test_case! {
    name: string_less_than,
    input: r#""apple" < "banana""#,
    formatted: { r#""apple" < "banana""# },
}

test_case! {
    name: string_greater_than,
    input: r#""zebra" > "apple""#,
    formatted: { r#""zebra" > "apple""# },
}

test_case! {
    name: string_less_than_or_equal,
    input: r#""hello" <= "hello""#,
    formatted: { r#""hello" <= "hello""# },
}

test_case! {
    name: bytes_less_than,
    input: r#"b"abc" < b"def""#,
    formatted: { r#"b"abc" < b"def""# },
}

// ======== Comparison with Expressions ========

test_case! {
    name: comparison_with_arithmetic,
    input: "x + 1 < y * 2",
    formatted: { "x + 1 < y * 2" },
}

test_case! {
    name: comparison_with_variables,
    input: "age >= 18",
    formatted: { "age >= 18" },
}

test_case! {
    name: chained_comparison_expression,
    input: "a == b and b == c",
    formatted: { "a == b and b == c" },
}

// ======== Comparison with Spaces ========

test_case! {
    name: comparison_with_spaces,
    input: "  5   ==   5  ",
    formatted: { "5 == 5" },
}

test_case! {
    name: comparison_no_spaces,
    input: "5==5",
    formatted: { "5 == 5" },
}

// ======== Precedence Tests ========

test_case! {
    name: comparison_lower_precedence_than_arithmetic,
    input: "1 + 2 == 3",
    ast: { "(Comparison Eq (Binary Add (Lit Int 1) (Lit Int 2)) (Lit Int 3))" },
    formatted: { "1 + 2 == 3" },
}

test_case! {
    name: comparison_higher_precedence_than_logical,
    input: "x > 5 and y < 10",
    formatted: { "x > 5 and y < 10" },
}

test_case! {
    name: parenthesized_comparison,
    input: "(x == y)",
    formatted: { "(x == y)" },
}

test_case! {
    name: complex_precedence,
    input: "a * b > c + d",
    formatted: { "a * b > c + d" },
}

// ======== Comparison in Control Flow ========

test_case! {
    name: comparison_in_if_condition,
    input: "if x > 0 then x else -x",
    formatted: { "if x > 0 then x else -x" },
}

test_case! {
    name: comparison_with_otherwise,
    input: "x < 10 otherwise false",
    formatted: { "x < 10 otherwise false" },
}

// ======== Negative Numbers ========

test_case! {
    name: comparison_with_negative,
    input: "-5 < 0",
    formatted: { "-5 < 0" },
}

test_case! {
    name: comparison_of_negatives,
    input: "-10 > -20",
    formatted: { "-10 > -20" },
}
