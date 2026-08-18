// Tests with valid expressions for each rule in the parser.

use pest::Parser;
use pest::iterators::Pair;

use crate::parser::{ExpressionParser, Rule};

fn contains_rule(pair: Pair<Rule>, target: Rule) -> bool {
    if pair.as_rule() == target {
        return true;
    }
    for inner in pair.into_inner() {
        if contains_rule(inner, target) {
            return true;
        }
    }
    false
}

macro_rules! rule_examples {
    ( $($rule:ident => [$($expr:expr),* $(,)?]),* $(,)? ) => {
        $(
            #[test]
            fn $rule() {
                let inputs = vec![$($expr),*];
                for input in inputs {
                    let result = ExpressionParser::parse(Rule::main, input)
                        .unwrap_or_else(|e| panic!("Failed to parse '{}': {}", input, e));
                    let root = result.into_iter().next().unwrap();
                    assert!(
                        contains_rule(root.clone(), Rule::$rule),
                        "Expected to find rule {:?} in parse tree for input '{}'",
                        Rule::$rule,
                        input
                    );
                }
            }
        )*
    };
}

rule_examples! {
    integer => ["42", "-99", "42`m`", "100`seconds`", "3`m/s`", "0b1010`K`", "0o755`B/s`"],
    dec_integer => ["0", "123", "1_000", "0_1_0_0"],
    bin_integer => ["0b0", "0b1010", "0b1111_0000"],
    oct_integer => ["0o0", "0o1234", "0o7654_3210"],
    hex_integer => ["0x0", "0x1A3F", "0xDEAD_BEEF"],
    float => ["3.14", "-0.001", "-10.0", "2.", ".5", "6.022e23", "1.6E-19", "1_000.0", "1_000.0e+3", "1.0`eV`", "0.1`Hz`", "9.81`m/s^2`"],
    suffix => ["42`m`", "100`seconds`", "3.14`m/s`", "0b1010`K`", "0o755`B/s`", "1.0`eV`", "0.1`Hz`", "9.81`m/s^2`"],
    string => ["\"hello\"", "'world'", "\"escaped\nnewline\"", "\"unicode: \\u0041\""],
    bytes => ["b\"abc\"", "b\"\\x41\""],
    boolean => ["true", "false"],
    ident => ["foo", "_bar123", "`0`", "`some-name`", "`with.dots`", "`:`", "`/path`"],
    call_op => ["foo()", "foo(1)", "foo(1, 2, 3)", "f(\"x\")", "foo.bar(x)"],
    array => ["[]", "[1]", "[1, 2, 3]", "[a, b,]"],
    map => ["{}", "{a: 1}", "{a: 1, b: 2,}", "{foo(): bar()}"],
    record => ["{x = 1}", "{x = 1, y = 2}", "Record {}"],
    cast_op => ["1 as Integer", "\"abc\" as Bytes", "{x = 1} as Record[x: Integer]"],
    add => ["1 + 2", "a * (b + c)"],
    mul => ["1 * 2", "a * ( b + c )"],
    pow => ["2 ^ 3", "a ^ b"],
    and => ["true and false", "a and b"],
    if_op => ["if true then 1 else 0", "if x then y else z"],
    where_op => ["a where {a = 1}", "x + y where {x = 1, y = 2}"],
    format_string => ["f\"Hello, {name}!\"", "f'Value: {x}'"],
    field_op => ["foo.bar", "a.b.c"],
    lambda_op => [
        "(a) => a + 1",
        "(x, y) => x * y",
        "() => 42",
        "(a, b,) => a - b",
        "[].map((x) => x + 1)",
        "{f: (x) => x * x}",
        "(a, b) => a + b otherwise 0",
        "(a) => (b) => a + b",
        "((x) => x + 1)(5)",
        "{add = (a, b) => a + b}.add(1, 2)",
    ],
    index_op => ["arr[0]", "matrix[1][2]", "map[key]"],
    neg => ["- 1", "-a"],
    not => ["not true", "-not x"],
    in_op => ["5 in [1, 2, 3]", "\"lo\" in \"hello\"", "key in map"],
    not_in => ["5 not in [1, 2, 3]", "\"x\" not in \"hello\"", "key not in map"],
    otherwise_op => ["1 / 0 otherwise -1", "map[key] otherwise \"\""],
    type_expr => [
        "value as Integer",
        "value as Map[String, Float]",
        "value as Record[x: Integer]",
        "value as Array[Map[String, Integer]]",
        "value as Map[String, Float]",
        "value as Array[Record[x: Integer]]",
    ],
    type_path => ["value as Integer", "value as Map", "value as SomeThing"],
}
