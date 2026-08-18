// ============================================================================
// REVIEWED & LOCKED - Test expectations are set in stone
// Date: 2024-10-14
// All test expectations in this file have been reviewed and approved.
// DO NOT change expectations without explicit discussion.
// If tests fail, fix the formatter, not the tests.
// ============================================================================

mod cases;

test_case! {
    name: simple_lambda,
    input: { "(x)=>x + 1" },
    formatted: { "(x) => x + 1" },
}

test_case! {
    name: lambda_no_params,
    input: {r"
(

) =>
    42".trim_start()},
    formatted: { "() => 42" },
}

test_case! {
    name: lambda_multiple_params,
    input: { r"
(x,y,z)
=>
x+y+z
" },
    formatted: { "(x, y, z) => x + y + z\n" },
}

test_case! {
    name: lambda_multiple_params_with_comments,
    input: { r"
(
    name,  // user's name
    age,  // user's age
    email,  // user's email
) => {
    name = name,
    age = age,
    email = email,
}
".trim_start() },
    formatted: { r"
(
    name, // user's name
    age, // user's age
    email, // user's email
) => {
    name = name,
    age = age,
    email = email,
}
".trim_start() },
}

test_case! {
    name: lambda_trailing_comma,
    input: { "(x,y,) => x+y" },
    formatted: { "(x, y) => x + y" },
}

test_case! {
    name: lambda_with_where,
    input: { "(a,b,c) => result where{delta=b^2-4*a*c,result=[1,2]}" },
    formatted: { "(a, b, c) => result where { delta = b ^ 2 - 4 * a * c, result = [1, 2] }" },
}

test_case! {
    name: lambda_with_multiline_where,
    input: { r"
(a, b, c) => result where {
    delta = b^2 - 4*a*c, r0 = (-b + delta^0.5) / (2*a), r1 = (-b - delta^0.5) / (2*a), result = [r0, r1]
}".trim_start()},
    formatted: { r"
(a, b, c) => result where {
    delta = b ^ 2 - 4 * a * c,
    r0 = (-b + delta ^ 0.5) / (2 * a),
    r1 = (-b - delta ^ 0.5) / (2 * a),
    result = [r0, r1],
}".trim_start() },
}

test_case! {
    name: lambda_with_multiline_where_hanging,
    input: { r"
(a, b, c) => result where {delta = b^2 - 4*a*c,
                            r0 = (-b + delta^0.5) / (2*a),
                            r1 = (-b - delta^0.5) / (2*a),
                            result = [r0, r1]}".trim_start() },
    formatted: { r"
(a, b, c) => result where {
    delta = b ^ 2 - 4 * a * c,
    r0 = (-b + delta ^ 0.5) / (2 * a),
    r1 = (-b - delta ^ 0.5) / (2 * a),
    result = [r0, r1],
}".trim_start() },
}

test_case! {
    name: nested_lambda,
    input: { "(x)=>(y)=>x+y" },
    formatted: { "(x) => (y) => x + y" },
}
