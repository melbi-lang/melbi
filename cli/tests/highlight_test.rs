use melbi_cli::commands::repl::highlighter::Highlighter;
use nu_ansi_term::Color;
use reedline::Highlighter as _;

#[test]
fn test_highlight_number() {
    let highlighter = Highlighter::new();

    // "1" should be an integer -> @number -> Cyan
    let output = highlighter.highlight("1", 0);

    // Check if we got any styling
    assert!(!output.buffer.is_empty(), "Output should not be empty");

    // The first segment should be the number 1
    // And it should have the color for numbers (Cyan in our palette)
    let (style, text) = &output.buffer[0];
    assert_eq!(text, "1");

    // In our palette, "number" is Cyan
    assert_eq!(
        style.foreground,
        Some(Color::Cyan),
        "Number should be Cyan, got {:?}",
        style.foreground
    );
}

#[test]
fn test_highlight_keyword() {
    let highlighter = Highlighter::new();

    // "if" should be a keyword -> @keyword -> Magenta
    let output = highlighter.highlight("if", 0);

    let (style, text) = &output.buffer[0];
    assert_eq!(text, "if");
    assert_eq!(
        style.foreground,
        Some(Color::Magenta),
        "Keyword should be Magenta, got {:?}",
        style.foreground
    );
}

#[test]
fn test_highlight_complex() {
    let highlighter = Highlighter::new();
    let output = highlighter.highlight("if true then 1 else 0", 0);

    let expected = vec![
        (Some(Color::Magenta), "if"),
        (Some(Color::White), " "),
        (Some(Color::Cyan), "true"),
        (Some(Color::White), " "),
        (Some(Color::Magenta), "then"),
        (Some(Color::White), " "),
        (Some(Color::Cyan), "1"),
        (Some(Color::White), " "),
        (Some(Color::Magenta), "else"),
        (Some(Color::White), " "),
        (Some(Color::Cyan), "0"),
    ];

    let actual: Vec<_> = output
        .buffer
        .iter()
        .map(|(style, text)| (style.foreground, text.as_str()))
        .collect();

    assert_eq!(
        expected, actual,
        "Complex expression was not highlighted as expected"
    );
}

#[test]
fn test_highlight_error() {
    let highlighter = Highlighter::new();
    let output = highlighter.highlight("1 + @", 0);

    let expected = vec![
        (Some(Color::Cyan), None, "1"),
        (Some(Color::White), None, " "),
        (Some(Color::White), None, "+"),
        (Some(Color::White), None, " "),
        (Some(Color::White), Some(Color::Rgb(0x80, 0x22, 0x3e)), "@"),
    ];

    let actual: Vec<_> = output
        .buffer
        .iter()
        .map(|(style, text)| (style.foreground, style.background, text.as_str()))
        .collect();

    assert_eq!(
        expected, actual,
        "Error token was not highlighted as expected"
    );
}
