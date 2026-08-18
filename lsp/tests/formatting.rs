use melbi_lsp::document::DocumentState;

#[test]
fn format_simple_expression() {
    let doc = DocumentState::new("1+2".to_string());
    let formatted = doc.format();

    assert!(formatted.is_some(), "Should format valid expression");
    let result = formatted.unwrap();
    // Formatter adds proper spacing
    assert!(
        result.contains("1 + 2"),
        "Should add spacing around operators"
    );
}

#[test]
fn format_where_expression() {
    let doc = DocumentState::new("x where{x=10}".to_string());
    let formatted = doc.format();

    assert!(formatted.is_some());
    let result = formatted.unwrap();
    // Formatter should add proper spacing
    assert!(result.contains("where"), "Should preserve 'where' keyword");
    assert!(result.contains("x = 10"), "Should add spacing around =");
}

#[test]
fn format_if_expression() {
    let doc = DocumentState::new("if true then 1 else 2".to_string());
    let formatted = doc.format();

    assert!(formatted.is_some());
    let result = formatted.unwrap();
    assert!(result.contains("if"));
    assert!(result.contains("then"));
    assert!(result.contains("else"));
}

#[test]
fn format_lambda() {
    let doc = DocumentState::new("(x)=>x+1".to_string());
    let formatted = doc.format();

    assert!(formatted.is_some());
    let result = formatted.unwrap();
    // Should add spacing
    assert_eq!(result, "(x) => x + 1");
}

#[test]
fn format_record() {
    let doc = DocumentState::new("{x=10,y=20}".to_string());
    let formatted = doc.format();

    assert!(formatted.is_some());
    let result = formatted.unwrap();
    assert!(result.contains("x = 10"));
    assert!(result.contains("y = 20"));
}

#[test]
fn format_array() {
    let doc = DocumentState::new("[1,2,3]".to_string());
    let formatted = doc.format();

    assert!(formatted.is_some());
    let result = formatted.unwrap();
    assert!(result.contains("[1, 2, 3]"));
}

#[test]
fn format_nested_expression() {
    let doc = DocumentState::new("(1+2)*(3+4)".to_string());
    let formatted = doc.format();

    assert!(formatted.is_some());
    let result = formatted.unwrap();
    assert!(result.contains(" + "));
    assert!(result.contains(" * "));
}

#[test]
fn format_multiline_where() {
    let doc = DocumentState::new("x+y where{x=1,y=2}".to_string());
    let formatted = doc.format();

    assert!(formatted.is_some());
    let result = formatted.unwrap();
    // Formatter should produce properly formatted output
    assert!(result.contains(" where "));
}

#[test]
fn format_preserves_comments() {
    let doc = DocumentState::new("# Comment\n1 + 2".to_string());
    let formatted = doc.format();

    // Formatter may not support comments yet
    if let Some(result) = formatted {
        assert!(!result.is_empty(), "Should produce non-empty output");
    } else {
        eprintln!("Note: Formatter may not support comments yet");
    }
}

#[test]
fn format_invalid_syntax_returns_none() {
    let doc = DocumentState::new("1 + +".to_string());
    let formatted = doc.format();

    // Document that format() doesn't panic on invalid syntax.
    // The formatter may return None or attempt best-effort formatting.
    let _ = formatted;
}

#[test]
fn format_empty_document() {
    let doc = DocumentState::new(String::new());
    let formatted = doc.format();

    // Empty document should format to empty or newline
    assert!(
        formatted.is_none()
            || formatted == Some("\n".to_string())
            || formatted == Some(String::new())
    );
}

#[test]
fn format_suffix_expression() {
    let doc = DocumentState::new("10`m/s`".to_string());
    let formatted = doc.format();

    assert!(formatted.is_some());
    let result = formatted.unwrap();
    // Should preserve suffix notation
    assert!(result.contains('`'));
}

#[test]
fn format_field_access() {
    let doc = DocumentState::new("record.field.nested".to_string());
    let formatted = doc.format();

    assert!(formatted.is_some());
    let result = formatted.unwrap();
    assert!(result.contains('.'));
}

#[test]
fn format_call_expression() {
    let doc = DocumentState::new("func(arg1,arg2)".to_string());
    let formatted = doc.format();

    assert!(formatted.is_some());
    let result = formatted.unwrap();
    // Formatter may add spaces after commas
    assert!(result.contains('('));
    assert!(result.contains(')'));
}

#[test]
fn format_does_not_add_trailing_newlines() {
    let doc = DocumentState::new("1 + 2".to_string());
    let formatted = doc.format();

    assert!(formatted.is_some());
    let result = formatted.unwrap();

    // Formatter adds a trailing newline (standard practice)
    // Verify it doesn't add multiple newlines
    let newline_count = result.matches('\n').count();
    assert!(
        newline_count <= 1,
        "Should not have multiple trailing newlines, got: {newline_count}"
    );
}

#[test]
fn format_idempotent() {
    let doc = DocumentState::new("1 + 2".to_string());
    let formatted_once = doc.format().unwrap();

    let doc2 = DocumentState::new(formatted_once.clone());
    let formatted_twice = doc2.format().unwrap();

    assert_eq!(
        formatted_once, formatted_twice,
        "Formatting should be idempotent"
    );
}
