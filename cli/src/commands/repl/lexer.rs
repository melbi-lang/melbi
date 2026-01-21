//! Lexer for tracking nesting depth in the REPL.

use logos::Logos;

/// Token types recognized by the Melbi lexer for depth calculation.
///
/// This lexer is designed for tracking nesting depth in the REPL,
/// not for full language parsing.
///
/// We're only interested in counting braces, brackets, and parentheses. So,
/// the list of tokens we need to track must be closed under the relation of
/// "can contain another token".
///
/// Note that this lexer doesn't parse the expressions interpolated in format
/// strings. As such, it doesn't allow indentation inside them. However, it
/// should successfully lex any valid Melbi expression.
#[derive(Logos, Debug, PartialEq)]
#[logos(skip r"[ \t\n\f]+")]
pub enum Token {
    #[token("{")]
    LBrace,

    #[token("}")]
    RBrace,

    #[token("[")]
    LBracket,

    #[token("]")]
    RBracket,

    #[token("(")]
    LParen,

    #[token(")")]
    RParen,

    #[regex(r"//.*")]
    Comment,

    /// Quoted Identifier (can contain double slashes)
    #[regex(r"`[A-Za-z0-9\-_.:\/]+`")]
    QuotedId,

    /// Double Quoted String (can contain any other token)
    #[regex(r#"(?:b|f)?"(?:[^"\\]|\\.)*""#)]
    StringDouble,

    /// Single Quoted String (can contain any other token)
    #[regex(r#"(?:b|f)?'(?:[^'\\]|\\.)*'"#)]
    StringSingle,

    /// Any other token we don't care about.
    #[regex(r#"[^ \t\n\f\{\}\[\]\(\)\"'`]+"#)]
    Other,
}

/// Calculates the nesting depth of delimiters in the given buffer.
///
/// Returns `Some(depth)` where depth is the net nesting level (≥ 0),
/// or `None` if the buffer contains invalid/incomplete tokens.
pub fn calculate_depth(buffer: &str) -> Option<usize> {
    let mut depth: isize = 0;

    for token_res in Token::lexer(buffer) {
        match token_res {
            Ok(Token::LBrace) | Ok(Token::LBracket) | Ok(Token::LParen) => depth += 1,
            Ok(Token::RBrace) | Ok(Token::RBracket) | Ok(Token::RParen) => depth -= 1,

            // Valid tokens that don't affect depth
            Ok(_) => {}

            // STRICT BEHAVIOR:
            // If we hit an unclosed string (or any unknown char), abort immediately.
            Err(_) => {
                return None;
            }
        }
    }

    if depth < 0 {
        Some(0)
    } else {
        Some(depth as usize)
    }
}
