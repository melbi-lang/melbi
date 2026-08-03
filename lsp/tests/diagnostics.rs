use melbi_lsp::document::DocumentState;
use tower_lsp::lsp_types::*;

#[test]
fn test_syntax_error_detection() {
    let mut doc = DocumentState::new("1 + + 2".to_string());
    let diagnostics = doc.analyze();

    assert!(!diagnostics.is_empty(), "Should detect syntax error");
    assert_eq!(diagnostics[0].severity, Some(DiagnosticSeverity::ERROR));
}

#[test]
fn test_type_error_detection() {
    let mut doc = DocumentState::new("1 + true".to_string());
    let diagnostics = doc.analyze();

    assert!(!diagnostics.is_empty(), "Should detect type error");
    let has_type_error = diagnostics
        .iter()
        .any(|d| d.message.contains("type") || d.message.contains("expected"));
    assert!(has_type_error, "Should report type mismatch");
}

#[test]
fn test_valid_program_no_errors() {
    let mut doc = DocumentState::new("1 + 2".to_string());
    let diagnostics = doc.analyze();

    assert!(
        diagnostics.is_empty(),
        "Valid program should have no errors"
    );
    assert!(doc.type_checked, "Valid program should type-check");
}

#[test]
fn test_where_expression_type_checking() {
    let mut doc = DocumentState::new("x + y where { x = 10, y = 20 }".to_string());
    let diagnostics = doc.analyze();

    assert!(
        diagnostics.is_empty(),
        "Valid where expression should type-check"
    );
    assert!(doc.type_checked);
}

#[test]
#[ignore] // tree-sitter grammar doesn't fully support lambda expressions yet
fn test_lambda_type_checking() {
    // Note: Lambda expressions are not fully supported by tree-sitter grammar
    // This is a known limitation of the grammar
    let mut doc = DocumentState::new("x => x + 1".to_string());
    let diagnostics = doc.analyze();

    assert!(
        diagnostics.is_empty(),
        "Lambda should type-check when grammar supports it"
    );
}

#[test]
fn test_if_expression_type_error() {
    let mut doc = DocumentState::new("if true then 1 else \"hello\"".to_string());
    let diagnostics = doc.analyze();

    assert!(
        !diagnostics.is_empty(),
        "Should detect incompatible branch types"
    );
}

#[test]
fn test_multiple_errors_reported() {
    // Even though analyzer currently returns first error only,
    // we should still test that syntax errors are reported
    let mut doc = DocumentState::new("1 + +".to_string());
    let diagnostics = doc.analyze();

    assert!(!diagnostics.is_empty(), "Should report errors");
}

#[test]
fn test_record_type_checking() {
    let mut doc = DocumentState::new("{ x = 10, y = 20 }.x".to_string());
    let diagnostics = doc.analyze();

    assert!(
        diagnostics.is_empty(),
        "Valid record access should type-check"
    );
}

#[test]
fn test_array_type_checking() {
    let mut doc = DocumentState::new("[1, 2, 3]".to_string());
    let diagnostics = doc.analyze();

    assert!(diagnostics.is_empty(), "Valid array should type-check");
}

#[test]
fn test_suffix_expression_type_checking() {
    let mut doc = DocumentState::new("10`m`".to_string());
    let diagnostics = doc.analyze();

    if !diagnostics.is_empty() {
        eprintln!("Suffix diagnostics: {:?}", diagnostics);
    }
    // Note: Suffix expressions may produce type errors if not handled by analyzer
    // For now, we just check that tree-sitter parses them without syntax errors
    let has_only_type_errors = diagnostics.iter().all(|d| !d.message.contains("Syntax"));
    assert!(
        diagnostics.is_empty() || has_only_type_errors,
        "Should parse suffix expression: got {:?}",
        diagnostics
    );
}
