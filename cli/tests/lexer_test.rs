use logos::Logos;
use melbi_cli::commands::repl::lexer::{Token, calculate_depth};

#[test]
fn test_lexer_brackets() {
    let lexer = Token::lexer("{}[]()");
    let tokens: Vec<_> = lexer.map(|token_res| token_res.unwrap()).collect();
    assert_eq!(
        tokens,
        vec![
            Token::LBrace,
            Token::RBrace,
            Token::LBracket,
            Token::RBracket,
            Token::LParen,
            Token::RParen,
        ]
    );
}

#[test]
fn test_lexer_comment() {
    let lexer = Token::lexer("// This is a comment\n{");
    let tokens: Vec<_> = lexer.map(|token_res| token_res.unwrap()).collect();
    assert_eq!(tokens, vec![Token::Comment, Token::LBrace]);
}

#[test]
fn test_lexer_quoted_id() {
    let lexer = Token::lexer("`my-id`");
    let tokens: Vec<_> = lexer.map(|token_res| token_res.unwrap()).collect();
    assert_eq!(tokens, vec![Token::QuotedId]);
}

#[test]
fn test_lexer_string_double() {
    let lexer = Token::lexer(r#""hello""#);
    let tokens: Vec<_> = lexer.map(|token_res| token_res.unwrap()).collect();
    assert_eq!(tokens, vec![Token::StringDouble]);

    let lexer_escaped = Token::lexer(r#""string with \" escape""#);
    let tokens_escaped: Vec<_> = lexer_escaped.map(|token_res| token_res.unwrap()).collect();
    assert_eq!(tokens_escaped, vec![Token::StringDouble]);

    let lexer_prefix = Token::lexer(r#"f"formatted""#);
    let tokens_prefix: Vec<_> = lexer_prefix.map(|token_res| token_res.unwrap()).collect();
    assert_eq!(tokens_prefix, vec![Token::StringDouble]);
}

#[test]
fn test_lexer_string_single() {
    let lexer = Token::lexer(r#" 'hello' "#);
    let tokens: Vec<_> = lexer.map(|token_res| token_res.unwrap()).collect();
    assert_eq!(tokens, vec![Token::StringSingle]);

    let lexer_escaped = Token::lexer(r#"'string with \' escape'"#);
    let tokens_escaped: Vec<_> = lexer_escaped.map(|token_res| token_res.unwrap()).collect();
    assert_eq!(tokens_escaped, vec![Token::StringSingle]);

    let lexer_prefix = Token::lexer(r#"f'formatted'"#);
    let tokens_prefix: Vec<_> = lexer_prefix.map(|token_res| token_res.unwrap()).collect();
    assert_eq!(tokens_prefix, vec![Token::StringSingle]);
}

#[test]
fn test_lexer_other() {
    let lexer = Token::lexer("some_other_text");
    let tokens: Vec<_> = lexer.map(|token_res| token_res.unwrap()).collect();
    assert_eq!(tokens, vec![Token::Other]);
}

#[test]
fn test_lexer_mixed() {
    // Note: `//comment` consumes rest of line, so all the test is part of it.
    let lexer = Token::lexer("{`id` //comment \"string\" other}");
    let tokens: Vec<_> = lexer.map(|token_res| token_res.unwrap()).collect();
    assert_eq!(
        tokens,
        vec![Token::LBrace, Token::QuotedId, Token::Comment,]
    );
}

#[test]
fn test_lexer_unclosed_string_error() {
    let mut lexer = Token::lexer(r#""unclosed string"#);
    assert!(lexer.next().unwrap().is_err());

    let mut lexer = Token::lexer("`unclosed id");
    assert!(lexer.next().unwrap().is_err());
}

#[test]
fn test_calculate_depth_empty() {
    assert_eq!(calculate_depth(""), Some(0));
}

#[test]
fn test_calculate_depth_simple() {
    assert_eq!(calculate_depth("{}"), Some(0));
    assert_eq!(calculate_depth("{ { } }"), Some(0));
    assert_eq!(calculate_depth("{ [ ( ) ] }"), Some(0));
    assert_eq!(calculate_depth("{ "), Some(1));
    assert_eq!(calculate_depth("{{"), Some(2));
    assert_eq!(calculate_depth("{}}"), Some(0)); // Should be 0, not -1
}

#[test]
fn test_calculate_depth_with_comments() {
    assert_eq!(calculate_depth("{ // comment\n}"), Some(0));
    assert_eq!(calculate_depth("{ /* not a comment */ }"), Some(0)); // multiline comment is "Other"
    assert_eq!(calculate_depth("{ // { [ (\n }"), Some(0));
}

#[test]
fn test_calculate_depth_mismatched() {
    // Mismatched delimiters still affect depth tracking
    assert_eq!(calculate_depth("{]"), Some(0)); // depth: +1, -1 = 0
    assert_eq!(calculate_depth("(}"), Some(0));
    assert_eq!(calculate_depth("{[)"), Some(1)); // depth: +1, +1, -1 = 1
}

#[test]
fn test_calculate_depth_with_strings() {
    assert_eq!(calculate_depth(r###"{"hello"}"###), Some(0));
    assert_eq!(calculate_depth(r###"{"{"}"###), Some(0));
    assert_eq!(calculate_depth(r###"{"}"###), None);
    assert_eq!(calculate_depth(r###"{"\""}"###), Some(0));
    assert_eq!(calculate_depth("{`id`}"), Some(0));
}

#[test]
fn test_calculate_depth_unclosed_string() {
    assert_eq!(calculate_depth(r###"{"unclosed"###), None);
    assert_eq!(calculate_depth("`unclosed`{"), Some(1));
    assert_eq!(calculate_depth(r###"`unclosed"###), None);
}

#[test]
fn test_calculate_depth_quoted_chars() {
    assert_eq!(calculate_depth("{`id`{"), Some(2));
    assert_eq!(calculate_depth("{`id`"), Some(1));
}

#[test]
fn test_calculate_depth_nested() {
    assert_eq!(calculate_depth("{ { { } } }"), Some(0));
    assert_eq!(calculate_depth("{ ( [ ] ) }"), Some(0));
    assert_eq!(calculate_depth("({["), Some(3));
    assert_eq!(calculate_depth(")]}"), Some(0)); // Negative depth at end should be 0
}
