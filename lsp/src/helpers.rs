use melbi_core::parser::Rule;
use pest::error::{ErrorVariant, LineColLocation};
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

pub trait IntoRange {
    fn into_range(self) -> Range;
}

impl IntoRange for LineColLocation {
    fn into_range(self) -> Range {
        match self {
            Self::Pos((line, col)) => {
                let pos = Position::new(line as u32 - 1, col as u32 - 1);
                Range::new(pos, pos)
            }
            Self::Span((start_line, start_col), (end_line, end_col)) => Range::new(
                Position::new(start_line as u32 - 1, start_col as u32 - 1),
                Position::new(end_line as u32 - 1, end_col as u32 - 1),
            ),
        }
    }
}

pub trait IntoDiagnostics {
    fn into_diagnostics(self) -> Vec<Diagnostic>;
}

impl IntoDiagnostics for Vec<pest::error::Error<Rule>> {
    fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.iter()
            .map(|e| {
                Diagnostic::new(
                    e.line_col.clone().into_range(),
                    Some(DiagnosticSeverity::ERROR),
                    None,
                    Some("Melbi Language Server".to_owned()),
                    match &e.variant {
                        ErrorVariant::ParsingError {
                            positives,
                            negatives,
                        } => {
                            let mut message = "Parsing error".to_owned();
                            if !positives.is_empty() {
                                message.push_str(" (expected ");
                                message.push_str(
                                    positives
                                        .iter()
                                        .map(|s| format!("\"{s:#?}\""))
                                        .collect::<Vec<String>>()
                                        .join(", ")
                                        .as_str(),
                                );
                                message.push(')');
                            }

                            if !negatives.is_empty() {
                                message.push_str(" (unexpected ");
                                message.push_str(
                                    negatives
                                        .iter()
                                        .map(|s| format!("\"{s:#?}\""))
                                        .collect::<Vec<String>>()
                                        .join(", ")
                                        .as_str(),
                                );
                                message.push(')');
                            }

                            message
                        }
                        ErrorVariant::CustomError { message } => {
                            let mut c = message.chars();
                            match c.next() {
                                None => String::new(),
                                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                            }
                        }
                    },
                    None,
                    None,
                )
            })
            .collect()
    }
}
