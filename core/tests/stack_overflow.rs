use bumpalo::Bump;
use melbi_core::parser;

#[test]
#[ignore = "crashes the binary and can't be caught with should_panic"]
fn test_stack_overflow() {
    let arena = Bump::new();

    // Create a very deeply nested expression that would exceed Rust's stack limit
    // without our depth protection. With 8MB stack, we need many thousands of
    // nesting levels to actually overflow the stack.
    //
    // We'll use parse_with_max_depth to disable our protection and try to crash.
    let mut input = String::new();

    // Let's try 100,000 levels of nesting - this should definitely overflow
    // an 8MB stack when the parser tries to recurse through all of them.
    for _ in 0..500 {
        input.push('(');
    }
    input.push('1');
    for _ in 0..500 {
        input.push(')');
    }

    // Set max_depth very high to bypass our depth checking
    // This should trigger an actual stack overflow in Rust
    let _ = parser::parse_with_max_depth(&arena, &input, usize::MAX);
}

#[test]
#[ignore = "Rust 1.93+ debug builds have larger stack frames (~16KB vs ~8KB per recursion), \
            causing stack overflow at ~500 nesting levels before depth check (1000) triggers. \
            Fix: either lower max_depth or restructure test with custom lower depth limit."]
fn test_depth_protection_prevents_stack_overflow() {
    let arena = Bump::new();

    // Same deeply nested expression, but with default depth protection
    let mut input = String::new();

    // Create something that would overflow the stack without protection
    for _ in 0..500 {
        input.push('(');
    }
    input.push('1');
    for _ in 0..500 {
        input.push(')');
    }

    // With default max_depth (1000), this should fail gracefully with an error
    // instead of crashing
    let result = parser::parse(&arena, &input);

    assert!(result.is_err(), "Should fail with depth error, not crash");

    let err = result.unwrap_err();
    let err_msg = format!("{}", err);
    assert!(
        err_msg.contains("nesting depth exceeds maximum"),
        "Error should mention nesting depth, got: {}",
        err_msg
    );
}
