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
    let mut doc = DocumentState::new("(x) => x + 1".to_string());
    doc.analyze();

    // Hover over the lambda expression
    let hover = doc.hover_at_position(Position::new(0, 0));
    assert_eq!(hover, Some("```melbi\n(Int) => Int\n```".to_string()));
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
    let mut doc = DocumentState::new("((x) => x + 1)(5)".to_string());
    let diagnostics = doc.analyze();

    eprintln!("diagnostics: {}", diagnostics.len());
    for d in diagnostics {
        eprintln!("MSG: {}", d.message);
    }

    // Hover over the call
    let hover = doc.hover_at_position(Position::new(0, "((x) => x + 1)".len() as u32));
    assert_eq!(hover, Some("```melbi\nInt\n```".to_string()));
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
    let expr = "x + f(y) where { x = 1, y = \"foo\", f = (s) => 1 }";
    let mut doc = DocumentState::new(expr.to_string());
    doc.analyze();

    // Hover over 'x'
    let hover_x = doc.hover_at_position(Position::new(0, expr.find("x").unwrap() as u32));
    assert_eq!(hover_x, Some("```melbi\nInt\n```".to_string()));

    // Hover over 'y'
    let hover_y = doc.hover_at_position(Position::new(0, expr.find("y").unwrap() as u32));
    assert_eq!(hover_y, Some("```melbi\nStr\n```".to_string()));
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
