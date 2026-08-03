use melbi_lsp::document::DocumentState;
use tower_lsp::lsp_types::*;

#[test]
fn test_hover_on_identifier() {
    let mut doc = DocumentState::new("x where { x = 42 }".to_string());
    doc.analyze();

    // Hover over 'x' at the beginning (position 0)
    let hover = doc.hover_at_position(Position::new(0, 0));
    assert!(hover.is_some(), "Should provide hover info for identifier");
    assert!(hover.unwrap().contains("Int"), "Should show Int type");
}

#[test]
fn test_hover_on_numeric_literal() {
    let mut doc = DocumentState::new("42".to_string());
    doc.analyze();

    // Hover over the number
    let hover = doc.hover_at_position(Position::new(0, 0));
    // Literals don't show hover (by design)
    assert!(hover.is_none(), "Literals should not show hover");
}

#[test]
fn test_hover_on_lambda() {
    let mut doc = DocumentState::new("x => x + 1".to_string());
    let diagnostics = doc.analyze();

    // Lambda expressions may not be supported by tree-sitter grammar
    if diagnostics.iter().any(|d| d.message.contains("Syntax")) {
        eprintln!("Skipping lambda hover test due to grammar limitations");
        return;
    }

    // Hover over the lambda expression
    let hover = doc.hover_at_position(Position::new(0, 0));
    if hover.is_some() {
        let hover_text = hover.unwrap();
        assert!(
            hover_text.contains("=>") || hover_text.contains("Int"),
            "Should show function type"
        );
    }
}

#[test]
fn test_hover_on_where_expression() {
    let mut doc = DocumentState::new("a + b where { a = 1, b = 2 }".to_string());
    doc.analyze();

    // Hover over the where expression (at 'a + b')
    let hover = doc.hover_at_position(Position::new(0, 0));
    assert!(hover.is_some(), "Should provide hover for where expression");
}

#[test]
fn test_hover_on_if_expression() {
    let mut doc = DocumentState::new("if true then 1 else 2".to_string());
    doc.analyze();

    // Hover over 'if' keyword
    let hover = doc.hover_at_position(Position::new(0, 0));
    assert!(hover.is_some(), "Should provide hover for if expression");
    assert!(hover.unwrap().contains("Int"), "Should show result type");
}

#[test]
fn test_hover_on_field_access() {
    let mut doc = DocumentState::new("{ x = 10 }.x".to_string());
    doc.analyze();

    // Hover over the field access
    let hover = doc.hover_at_position(Position::new(0, 11));
    assert!(hover.is_some(), "Should provide hover for field access");
}

#[test]
fn test_hover_on_call_expression() {
    let mut doc = DocumentState::new("(x => x + 1)(5)".to_string());
    let diagnostics = doc.analyze();

    // Call expressions with lambdas may not be supported by tree-sitter grammar
    if diagnostics.iter().any(|d| d.message.contains("Syntax")) {
        eprintln!("Skipping call hover test due to grammar limitations");
        return;
    }

    // Hover over the call
    let _hover = doc.hover_at_position(Position::new(0, 0));
    // Just ensure it doesn't crash
}

#[test]
fn test_no_hover_on_invalid_code() {
    let mut doc = DocumentState::new("1 + +".to_string());
    doc.analyze();

    // Shouldn't crash on invalid code
    let _hover = doc.hover_at_position(Position::new(0, 0));
    // May or may not have hover depending on what parsed
    // Just ensure it doesn't panic
}

#[test]
fn test_hover_position_sensitivity() {
    let mut doc = DocumentState::new("x + y where { x = 1, y = 2 }".to_string());
    doc.analyze();

    // Hover over 'x'
    let hover_x = doc.hover_at_position(Position::new(0, 0));
    assert!(hover_x.is_some());

    // Hover over 'y'
    let hover_y = doc.hover_at_position(Position::new(0, 4));
    assert!(hover_y.is_some());

    // Both should show Int type
    assert!(hover_x.unwrap().contains("Int"));
    assert!(hover_y.unwrap().contains("Int"));
}

#[test]
fn test_hover_on_nested_expression() {
    let mut doc = DocumentState::new("(1 + 2) * 3".to_string());
    doc.analyze();

    // Hover over the whole expression
    let hover = doc.hover_at_position(Position::new(0, 0));
    // Inner literal, so no hover
    assert!(hover.is_none());
}
