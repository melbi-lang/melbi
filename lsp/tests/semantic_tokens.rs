use melbi_lsp::document::DocumentState;

#[test]
fn semantic_tokens_for_simple_expression() {
    let mut doc = DocumentState::new("1 + 2".to_string());
    doc.analyze();

    let tokens = doc.semantic_tokens();
    assert!(tokens.is_some(), "Should generate semantic tokens");

    let token_data = tokens.unwrap();
    assert!(
        !token_data.is_empty(),
        "Should have tokens for numbers and operator"
    );
}

#[test]
fn semantic_tokens_for_identifiers() {
    let mut doc = DocumentState::new("x where { x = 10 }".to_string());
    doc.analyze();

    let tokens = doc.semantic_tokens();
    assert!(tokens.is_some());

    // Should have tokens for identifiers and keyword
    let token_data = tokens.unwrap();
    assert!(token_data.len() >= 3, "Should have multiple tokens");
}

#[test]
fn semantic_tokens_for_keywords() {
    let mut doc = DocumentState::new("if true then 1 else 2".to_string());
    doc.analyze();

    let tokens = doc.semantic_tokens();
    assert!(tokens.is_some());

    // Should highlight 'if', 'then', 'else' as keywords
    let token_data = tokens.unwrap();
    assert!(
        token_data.len() >= 5,
        "Should have tokens for keywords and values"
    );
}

#[test]
fn semantic_tokens_for_string() {
    let mut doc = DocumentState::new(r#""hello""#.to_string());
    doc.analyze();

    let tokens = doc.semantic_tokens();
    assert!(tokens.is_some());
}

#[test]
fn semantic_tokens_for_lambda() {
    let mut doc = DocumentState::new("x => x + 1".to_string());
    doc.analyze();

    let tokens = doc.semantic_tokens();
    assert!(tokens.is_some());

    let token_data = tokens.unwrap();
    assert!(!token_data.is_empty(), "Should have tokens for lambda");
}

#[test]
fn semantic_tokens_for_record() {
    let mut doc = DocumentState::new("{ x = 10, y = 20 }".to_string());
    doc.analyze();

    let tokens = doc.semantic_tokens();
    assert!(tokens.is_some());

    // Should have tokens for field names and values
    let token_data = tokens.unwrap();
    assert!(
        token_data.len() >= 4,
        "Should have tokens for fields and values"
    );
}

#[test]
fn semantic_tokens_for_array() {
    let mut doc = DocumentState::new("[1, 2, 3]".to_string());
    doc.analyze();

    let tokens = doc.semantic_tokens();
    assert!(tokens.is_some());
}

#[test]
fn semantic_tokens_for_suffix_expression() {
    let mut doc = DocumentState::new("10`m/s`".to_string());
    doc.analyze();

    let tokens = doc.semantic_tokens();
    assert!(tokens.is_some());

    // The entire suffix expression should be highlighted as one unit
    let token_data = tokens.unwrap();
    // Should have one token for the entire suffix expression
    assert!(!token_data.is_empty(), "Should have token for suffix");
}

#[test]
fn semantic_tokens_for_comment() {
    let mut doc = DocumentState::new("# This is a comment\n42".to_string());
    doc.analyze();

    let tokens = doc.semantic_tokens();
    assert!(tokens.is_some());

    // Should have tokens for comment and number
    let token_data = tokens.unwrap();
    assert!(
        token_data.len() >= 2,
        "Should have tokens for comment and number"
    );
}

#[test]
fn semantic_tokens_multiline() {
    let mut doc = DocumentState::new("x + y\nwhere {\n  x = 1,\n  y = 2\n}".to_string());
    doc.analyze();

    let tokens = doc.semantic_tokens();
    assert!(tokens.is_some());

    // Delta encoding should handle line transitions
    let token_data = tokens.unwrap();
    assert!(
        token_data.len() >= 5,
        "Should have tokens across multiple lines"
    );
}

#[test]
fn semantic_tokens_on_error() {
    let mut doc = DocumentState::new("1 + +".to_string());
    doc.analyze();

    let _ = doc.semantic_tokens();
    // Tree-sitter error recovery may allow partial highlighting even with syntax errors.
    // We don't assert a specific outcome here, as it depends on the parser's error recovery.
    // This test documents that semantic_tokens() doesn't panic on invalid input.
}

#[test]
fn semantic_tokens_operators() {
    let mut doc = DocumentState::new("1 + 2 - 3 * 4 / 5".to_string());
    doc.analyze();

    let tokens = doc.semantic_tokens();
    assert!(tokens.is_some());

    // Should have tokens for numbers and operators
    let token_data = tokens.unwrap();
    assert!(
        token_data.len() >= 9,
        "Should have tokens for numbers and operators"
    );
}

#[test]
fn semantic_tokens_field_access() {
    let mut doc = DocumentState::new("record.field".to_string());
    doc.analyze();

    let tokens = doc.semantic_tokens();
    assert!(tokens.is_some());

    // Should have tokens for identifier and property
    let token_data = tokens.unwrap();
    assert!(
        token_data.len() >= 2,
        "Should have tokens for record and field"
    );
}

#[test]
fn semantic_tokens_boolean_operators() {
    let mut doc = DocumentState::new("true and false or not true".to_string());
    doc.analyze();

    let tokens = doc.semantic_tokens();
    assert!(tokens.is_some());

    let token_data = tokens.unwrap();
    assert!(
        token_data.len() >= 6,
        "Should have tokens for booleans and operators"
    );
}

#[test]
fn semantic_tokens_empty_document() {
    let mut doc = DocumentState::new(String::new());
    doc.analyze();

    let tokens = doc.semantic_tokens();
    // Empty document may have no tokens
    match tokens {
        None => (),
        Some(tokens) => assert!(tokens.is_empty(), "Empty doc should have no tokens"),
    }
}
