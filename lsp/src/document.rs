use bumpalo::Bump;
use tower_lsp::lsp_types::{Diagnostic, Range, Position, DiagnosticSeverity, NumberOrString, CompletionItem, CompletionItemKind, InsertTextFormat, SemanticToken};

use crate::semantic_tokens as st;

/// Represents the state of a document being edited
#[derive(Debug)]
pub struct DocumentState {
    /// The source code
    pub source: String,

    /// Tree-sitter parse tree (if parsing succeeded)
    pub tree: Option<tree_sitter::Tree>,

    /// Current diagnostics for this document
    pub diagnostics: Vec<Diagnostic>,

    /// Whether the document type-checked successfully
    pub type_checked: bool,
}

impl DocumentState {
    #[must_use]
    pub fn new(source: String) -> Self {
        Self {
            source,
            tree: None,
            diagnostics: Vec::new(),
            type_checked: false,
        }
    }

    /// Update the document with new source code
    pub fn update(&mut self, source: String) {
        self.source = source;
        self.tree = None;
        self.diagnostics.clear();
        self.type_checked = false;
    }

    /// Parse and analyze the document, returning all diagnostics
    pub fn analyze(&mut self) -> Vec<Diagnostic> {
        let mut all_diagnostics = Vec::new();

        // Parse with tree-sitter for syntax errors
        let syntax_diagnostics = self.parse_with_tree_sitter();
        all_diagnostics.extend(syntax_diagnostics);

        // If tree-sitter parsing succeeded, try semantic analysis
        if self.tree.is_some() {
            let type_diagnostics = self.type_check();
            all_diagnostics.extend(type_diagnostics);
        }

        self.diagnostics = all_diagnostics.clone();
        all_diagnostics
    }

    /// Parse the document using tree-sitter
    fn parse_with_tree_sitter(&mut self) -> Vec<Diagnostic> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_melbi::LANGUAGE.into())
            .expect("Error loading Melbi grammar");

        let tree = match parser.parse(&self.source, None) {
            Some(tree) => tree,
            None => {
                return vec![Diagnostic {
                    range: Range::new(Position::new(0, 0), Position::new(0, 0)),
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: None,
                    source: Some("melbi".to_string()),
                    message: "Failed to parse document".to_string(),
                    ..Default::default()
                }];
            }
        };

        let mut diagnostics = Vec::new();

        // Check for syntax errors
        if tree.root_node().has_error() {
            self.collect_syntax_errors(tree.root_node(), &mut diagnostics);
        }

        self.tree = Some(tree);
        diagnostics
    }

    /// Recursively collect syntax errors from tree-sitter parse tree
    fn collect_syntax_errors(&self, node: tree_sitter::Node, diagnostics: &mut Vec<Diagnostic>) {
        if node.is_error() || node.is_missing() {
            let start = node.start_position();
            let end = node.end_position();

            diagnostics.push(Diagnostic {
                range: Range::new(
                    Position::new(start.row as u32, start.column as u32),
                    Position::new(end.row as u32, end.column as u32),
                ),
                severity: Some(DiagnosticSeverity::ERROR),
                code: None,
                source: Some("melbi".to_string()),
                message: if node.is_missing() {
                    format!("Missing {}", node.kind())
                } else {
                    "Syntax error".to_string()
                },
                ..Default::default()
            });
        }

        // Recursively check children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_syntax_errors(child, diagnostics);
        }
    }

    /// Analyze the document for type errors
    fn type_check(&mut self) -> Vec<Diagnostic> {
        use melbi_core::types::manager::TypeManager;
        use melbi_core::{analyzer, parser};

        // Create arena for this analysis
        let arena = Bump::new();

        // Parse with Pest
        let parsed = match parser::parse(&arena, &self.source) {
            Ok(p) => p,
            Err(_) => {
                // Parsing failed - tree-sitter already reported errors
                return Vec::new();
            }
        };

        // Create type manager
        let type_manager = TypeManager::new(&arena);

        // Analyze with empty globals and variables for now
        // TODO: Add support for providing globals (stdlib functions)
        let globals: &[(&str, &_)] = &[];
        let variables: &[(&str, &_)] = &[];

        match analyzer::analyze(type_manager, &arena, parsed, globals, variables) {
            Ok(_typed_expr) => {
                self.type_checked = true;
                Vec::new()
            }
            Err(e) => {
                self.type_checked = false;
                vec![self.error_to_diagnostic(&e)]
            }
        }
    }

    /// Convert a Melbi `TypeError` to an LSP diagnostic
    fn error_to_diagnostic(&self, error: &melbi_core::analyzer::TypeError) -> Diagnostic {
        // Use the error's built-in to_diagnostic() method
        let diag = error.to_diagnostic();

        // Convert Span to LSP Range
        let start_pos = self.offset_to_position(diag.span.0.start);
        let end_pos = self.offset_to_position(diag.span.0.end);
        let range = Range::new(start_pos, end_pos);

        // Convert severity
        let severity = match diag.severity {
            melbi_core::api::Severity::Error => DiagnosticSeverity::ERROR,
            melbi_core::api::Severity::Warning => DiagnosticSeverity::WARNING,
            melbi_core::api::Severity::Info => DiagnosticSeverity::INFORMATION,
        };

        Diagnostic {
            range,
            severity: Some(severity),
            code: diag.code.map(NumberOrString::String),
            source: Some("melbi".to_string()),
            message: diag.message,
            ..Default::default()
        }
    }

    /// Convert byte offset to LSP Position
    fn offset_to_position(&self, offset: usize) -> Position {
        let mut line = 0;
        let mut col = 0;
        let mut current_offset = 0;

        for ch in self.source.chars() {
            if current_offset >= offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
            current_offset += ch.len_utf8();
        }

        Position::new(line, col)
    }

    /// Convert LSP Position to byte offset
    fn position_to_offset(&self, position: Position) -> Option<usize> {
        let mut offset = 0;
        let mut current_line = 0;
        let mut current_col = 0;

        for ch in self.source.chars() {
            if current_line == position.line && current_col == position.character {
                return Some(offset);
            }

            if ch == '\n' {
                current_line += 1;
                current_col = 0;
            } else {
                current_col += 1;
            }
            offset += ch.len_utf8();
        }

        // If we reached the end and match the position, return the offset
        if current_line == position.line && current_col == position.character {
            Some(offset)
        } else {
            None
        }
    }

    /// Get hover information at a position
    #[must_use]
    pub fn hover_at_position(&self, position: Position) -> Option<String> {
        use melbi_core::types::manager::TypeManager;
        use melbi_core::{analyzer, parser};

        // Only provide hover if type checking succeeded
        if !self.type_checked {
            return None;
        }

        // Convert position to byte offset
        let offset = self.position_to_offset(position)?;

        // Re-run analysis to get typed expression
        // TODO: Cache the typed expression to avoid re-analysis
        let arena = Bump::new();
        let parsed = parser::parse(&arena, &self.source).ok()?;
        let type_manager = TypeManager::new(&arena);
        let globals: &[(&str, &_)] = &[];
        let variables: &[(&str, &_)] = &[];

        let typed_expr =
            analyzer::analyze(type_manager, &arena, parsed, globals, variables).ok()?;

        // Find the most specific expression at the cursor position
        let expr_at_cursor = self.find_expr_at_offset(typed_expr.expr, typed_expr.ann, offset)?;

        // Only show hover for identifiers and calls - not for literals or operators
        use melbi_core::analyzer::typed_expr::ExprInner;
        let should_show_hover = matches!(
            &expr_at_cursor.1,
            ExprInner::Ident(_)
                | ExprInner::Call { .. }
                | ExprInner::Field { .. }
                | ExprInner::Lambda { .. }
                | ExprInner::Where { .. }
                | ExprInner::If { .. }
        );

        if !should_show_hover {
            return None;
        }

        // Format the hover response
        let type_str = format!("{}", expr_at_cursor.0);
        let hover_text = format!("```melbi\n{type_str}\n```");

        // TODO: When documentation support is added, append it here:
        // if let Some(doc) = get_documentation_for_expr(expr_at_cursor) {
        //     hover_text.push_str("\n\n---\n\n");
        //     hover_text.push_str(doc);
        // }

        Some(hover_text)
    }

    /// Find the most specific (smallest) expression at the given offset
    fn find_expr_at_offset<'types, 'arena>(
        &self,
        expr: &'arena melbi_core::analyzer::typed_expr::Expr<'types, 'arena>,
        ann: &'arena melbi_core::parser::AnnotatedSource<
            'arena,
            melbi_core::analyzer::typed_expr::Expr<'types, 'arena>,
        >,
        offset: usize,
    ) -> Option<&'arena melbi_core::analyzer::typed_expr::Expr<'types, 'arena>> {
        use melbi_core::analyzer::typed_expr::ExprInner;

        // Check if this expression's span contains the offset
        let span = ann.span_of(expr)?;
        if !span.0.contains(&offset) {
            return None;
        }

        // Try to find a more specific child expression
        // If we find one, return it; otherwise return this expression
        let child = match &expr.1 {
            ExprInner::Binary { left, right, .. } => self
                .find_expr_at_offset(left, ann, offset)
                .or_else(|| self.find_expr_at_offset(right, ann, offset)),
            ExprInner::Boolean { left, right, .. } => self
                .find_expr_at_offset(left, ann, offset)
                .or_else(|| self.find_expr_at_offset(right, ann, offset)),
            ExprInner::Comparison { left, right, .. } => self
                .find_expr_at_offset(left, ann, offset)
                .or_else(|| self.find_expr_at_offset(right, ann, offset)),
            ExprInner::Unary { expr: inner, .. } => self.find_expr_at_offset(inner, ann, offset),
            ExprInner::Call { callable, args, .. } => {
                self.find_expr_at_offset(callable, ann, offset).or_else(|| {
                    args.iter()
                        .find_map(|arg| self.find_expr_at_offset(arg, ann, offset))
                })
            }
            ExprInner::Option { inner } => match inner {
                Some(inner) => self.find_expr_at_offset(inner, ann, offset),
                None => None,
            },
            ExprInner::Index { value, index, .. } => self
                .find_expr_at_offset(value, ann, offset)
                .or_else(|| self.find_expr_at_offset(index, ann, offset)),
            ExprInner::Field { value, .. } => self.find_expr_at_offset(value, ann, offset),
            ExprInner::Cast { expr: inner, .. } => self.find_expr_at_offset(inner, ann, offset),
            ExprInner::Lambda { body, .. } => self.find_expr_at_offset(body, ann, offset),
            ExprInner::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => self
                .find_expr_at_offset(cond, ann, offset)
                .or_else(|| self.find_expr_at_offset(then_branch, ann, offset))
                .or_else(|| self.find_expr_at_offset(else_branch, ann, offset)),
            ExprInner::Where {
                expr: inner,
                bindings,
                ..
            } => {
                // Check bindings first (they're more specific)
                bindings
                    .iter()
                    .find_map(|(_, binding_expr)| {
                        self.find_expr_at_offset(binding_expr, ann, offset)
                    })
                    .or_else(|| self.find_expr_at_offset(inner, ann, offset))
            }
            ExprInner::Otherwise {
                primary, fallback, ..
            } => self
                .find_expr_at_offset(primary, ann, offset)
                .or_else(|| self.find_expr_at_offset(fallback, ann, offset)),
            ExprInner::Record { fields, .. } => fields
                .iter()
                .find_map(|(_, field_expr)| self.find_expr_at_offset(field_expr, ann, offset)),
            ExprInner::Map { elements, .. } => elements.iter().find_map(|(key, value)| {
                self.find_expr_at_offset(key, ann, offset)
                    .or_else(|| self.find_expr_at_offset(value, ann, offset))
            }),
            ExprInner::Array { elements, .. } => elements
                .iter()
                .find_map(|elem| self.find_expr_at_offset(elem, ann, offset)),
            ExprInner::FormatStr { exprs, .. } => exprs
                .iter()
                .find_map(|e| self.find_expr_at_offset(e, ann, offset)),
            ExprInner::Match { expr: inner, arms } => {
                self.find_expr_at_offset(inner, ann, offset).or_else(|| {
                    arms.iter()
                        .find_map(|arm| self.find_expr_at_offset(arm.body, ann, offset))
                })
            }
            // Leaf nodes - no children to search
            ExprInner::Constant(_) | ExprInner::Ident(_) => None,
        };

        // Return the most specific expression found
        child.or(Some(expr))
    }

    /// Get completion items at a position
    #[must_use]
    pub fn completions_at_position(&self, position: Position) -> Vec<CompletionItem> {
        use melbi_core::types::manager::TypeManager;
        use melbi_core::{analyzer, parser};

        // Convert position to offset to check context
        let offset = match self.position_to_offset(position) {
            Some(o) => o,
            None => return Vec::new(),
        };

        // Check if we're after a dot (field completion)
        let is_field_completion = self.is_after_dot(offset);

        if is_field_completion {
            // TODO: Implement record field completion
            // Need to figure out the type of the expression before the dot
            // and suggest its fields
            // This requires parsing incomplete code, which is tricky
            return Vec::new();
        }

        // Always provide keyword completions
        let mut completions = self.get_keyword_completions();

        // If type checking succeeded, also provide scope-based completions
        if self.type_checked {
            // Re-run analysis to get scope information
            // TODO: Cache the typed expression and scope stack to avoid re-analysis
            let arena = Bump::new();
            if let Ok(parsed) = parser::parse(&arena, &self.source) {
                let type_manager = TypeManager::new(&arena);
                let globals: &[(&str, &_)] = &[];
                let variables: &[(&str, &_)] = &[];

                if let Ok(typed_expr) =
                    analyzer::analyze(type_manager, &arena, parsed, globals, variables)
                {
                    // Add variable completions from scope
                    let scope_completions =
                        self.collect_scope_completions(typed_expr.expr, typed_expr.ann, offset);
                    completions.extend(scope_completions);
                }
            }
        }

        completions
    }

    /// Get keyword completions
    fn get_keyword_completions(&self) -> Vec<CompletionItem> {
        vec![
            CompletionItem {
                label: "where".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("where { bindings }".to_string()),
                insert_text: Some("where {\n    $1\n}".to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            },
            CompletionItem {
                label: "if".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("if-then-else expression".to_string()),
                insert_text: Some("if $1 then $2 else $3".to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            },
            CompletionItem {
                label: "otherwise".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("fallback value".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "true".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                ..Default::default()
            },
            CompletionItem {
                label: "false".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                ..Default::default()
            },
            CompletionItem {
                label: "and".to_string(),
                kind: Some(CompletionItemKind::OPERATOR),
                ..Default::default()
            },
            CompletionItem {
                label: "or".to_string(),
                kind: Some(CompletionItemKind::OPERATOR),
                ..Default::default()
            },
            CompletionItem {
                label: "not".to_string(),
                kind: Some(CompletionItemKind::OPERATOR),
                ..Default::default()
            },
        ]
    }

    /// Check if the cursor is right after a '.'
    fn is_after_dot(&self, offset: usize) -> bool {
        if offset == 0 {
            return false;
        }

        // Look backwards for non-whitespace
        let bytes = self.source.as_bytes();
        for i in (0..offset).rev() {
            match bytes[i] {
                b'.' => return true,
                b' ' | b'\t' | b'\n' | b'\r' => continue,
                _ => return false,
            }
        }
        false
    }

    /// Collect completion items from identifiers in scope
    fn collect_scope_completions<'types, 'arena>(
        &self,
        expr: &'arena melbi_core::analyzer::typed_expr::Expr<'types, 'arena>,
        ann: &'arena melbi_core::parser::AnnotatedSource<
            'arena,
            melbi_core::analyzer::typed_expr::Expr<'types, 'arena>,
        >,
        offset: usize,
    ) -> Vec<CompletionItem> {
        use std::collections::HashSet;

        let mut completions = Vec::new();
        let mut seen = HashSet::new();

        // Walk the AST and collect all identifiers that are in scope at the cursor position
        self.collect_identifiers_in_scope(expr, ann, offset, &mut completions, &mut seen);

        completions
    }

    fn collect_identifiers_in_scope<'types, 'arena>(
        &self,
        expr: &'arena melbi_core::analyzer::typed_expr::Expr<'types, 'arena>,
        ann: &'arena melbi_core::parser::AnnotatedSource<
            'arena,
            melbi_core::analyzer::typed_expr::Expr<'types, 'arena>,
        >,
        offset: usize,
        completions: &mut Vec<CompletionItem>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        use melbi_core::analyzer::typed_expr::ExprInner;

        // Check if this expression contains the cursor position
        let span = match ann.span_of(expr) {
            Some(s) => s,
            None => return,
        };

        // Only collect from expressions that contain the cursor
        if !span.0.contains(&offset) && span.0.end < offset {
            return;
        }

        // Collect identifiers from where bindings (they're in scope for the rest of the expression)
        if let ExprInner::Where {
            bindings,
            expr: inner,
            ..
        } = &expr.1
        {
            for (name, _) in *bindings {
                if !seen.contains(*name) {
                    seen.insert(name.to_string());
                    completions.push(CompletionItem {
                        label: name.to_string(),
                        kind: Some(CompletionItemKind::VARIABLE),
                        detail: Some("(where binding)".to_string()),
                        ..Default::default()
                    });
                }
            }
            // Continue searching in the inner expression
            self.collect_identifiers_in_scope(inner, ann, offset, completions, seen);
        }

        // Collect lambda parameters
        if let ExprInner::Lambda { params, body, .. } = &expr.1 {
            for param in *params {
                if !seen.contains(*param) {
                    seen.insert(param.to_string());
                    completions.push(CompletionItem {
                        label: param.to_string(),
                        kind: Some(CompletionItemKind::VARIABLE),
                        detail: Some("(parameter)".to_string()),
                        ..Default::default()
                    });
                }
            }
            // Continue searching in the body
            self.collect_identifiers_in_scope(body, ann, offset, completions, seen);
        }

        // Recursively search other expression types
        match &expr.1 {
            ExprInner::Binary { left, right, .. } => {
                self.collect_identifiers_in_scope(left, ann, offset, completions, seen);
                self.collect_identifiers_in_scope(right, ann, offset, completions, seen);
            }
            ExprInner::Boolean { left, right, .. } => {
                self.collect_identifiers_in_scope(left, ann, offset, completions, seen);
                self.collect_identifiers_in_scope(right, ann, offset, completions, seen);
            }
            ExprInner::Unary { expr: inner, .. } => {
                self.collect_identifiers_in_scope(inner, ann, offset, completions, seen);
            }
            ExprInner::Call { callable, args, .. } => {
                self.collect_identifiers_in_scope(callable, ann, offset, completions, seen);
                for arg in *args {
                    self.collect_identifiers_in_scope(arg, ann, offset, completions, seen);
                }
            }
            ExprInner::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                self.collect_identifiers_in_scope(cond, ann, offset, completions, seen);
                self.collect_identifiers_in_scope(then_branch, ann, offset, completions, seen);
                self.collect_identifiers_in_scope(else_branch, ann, offset, completions, seen);
            }
            ExprInner::Otherwise {
                primary, fallback, ..
            } => {
                self.collect_identifiers_in_scope(primary, ann, offset, completions, seen);
                self.collect_identifiers_in_scope(fallback, ann, offset, completions, seen);
            }
            _ => {
                // For other expression types, we don't need to recurse for completion purposes
            }
        }
    }

    /// Get semantic tokens for the entire document
    #[must_use]
    pub fn semantic_tokens(&self) -> Option<Vec<SemanticToken>> {
        let tree = self.tree.as_ref()?;
        let mut tokens = Vec::new();

        self.collect_semantic_tokens(tree.root_node(), &mut tokens);

        // Sort by position (line, then character)
        tokens.sort_by(|a, b| {
            a.delta_line
                .cmp(&b.delta_line)
                .then(a.delta_start.cmp(&b.delta_start))
        });

        // Convert to delta encoding (required by LSP)
        let mut encoded_tokens = Vec::new();
        let mut prev_line = 0;
        let mut prev_start = 0;

        for token in tokens {
            let delta_line = token.delta_line - prev_line;
            let delta_start = if delta_line == 0 {
                token.delta_start - prev_start
            } else {
                token.delta_start
            };

            encoded_tokens.push(SemanticToken {
                delta_line,
                delta_start,
                length: token.length,
                token_type: token.token_type,
                token_modifiers_bitset: token.token_modifiers_bitset,
            });

            prev_line = token.delta_line;
            prev_start = token.delta_start;
        }

        Some(encoded_tokens)
    }

    fn collect_semantic_tokens(&self, node: tree_sitter::Node, tokens: &mut Vec<SemanticToken>) {
        let kind = node.kind();
        let start = node.start_position();

        // Check if we're inside a number suffix - if so, highlight the whole thing as a number
        let is_in_suffix = node
            .parent()
            .is_some_and(|p| matches!(p.kind(), "integer" | "float") && node.kind() == "expression");

        // Map tree-sitter node kinds to semantic token types
        let token_type = match kind {
            // Numbers (including the whole suffix expression)
            "integer" | "float" => Some(st::NUMBER),

            // If we're inside a suffix expression, treat everything as part of the number
            _ if is_in_suffix => {
                // Don't recurse into suffix - we'll highlight the whole thing as one token
                // by just highlighting the parent integer/float node
                return;
            }

            // Keywords
            "if" | "then" | "else" | "where" | "otherwise" | "as" | "and" | "or" | "not" => {
                Some(st::KEYWORD)
            }
            "true" | "false" => Some(st::KEYWORD),

            // Operators
            "+" | "-" | "*" | "/" | "^" | "=>" => Some(st::OPERATOR),

            // Strings
            "string" | "bytes" | "format_string" => Some(st::STRING),

            // Comments
            "comment" => Some(st::COMMENT),

            // Types
            "type_path" | "type_application" | "record_type" => Some(st::TYPE),

            // Identifiers - distinguish between function calls and variables
            "identifier" => {
                // Check if this identifier is being called (parent is call_expression)
                if let Some(parent) = node.parent() {
                    if parent.kind() == "call_expression"
                        && parent.child_by_field_name("function") == Some(node)
                    {
                        Some(st::FUNCTION)
                    } else {
                        Some(st::VARIABLE)
                    }
                } else {
                    Some(st::VARIABLE)
                }
            }

            "unquoted_identifier" | "quoted_identifier" => {
                // Check if this is a binding name (left side of =)
                if let Some(parent) = node.parent() {
                    if parent.kind() == "binding"
                        && parent.child_by_field_name("name") == Some(node)
                    {
                        // This is a binding definition
                        Some(st::VARIABLE)
                    } else if parent.kind() == "lambda_params" {
                        // Lambda parameter
                        Some(st::PARAMETER)
                    } else if parent.kind() == "field_expression" {
                        // Field access
                        Some(st::PROPERTY)
                    } else {
                        Some(st::VARIABLE)
                    }
                } else {
                    Some(st::VARIABLE)
                }
            }

            _ => None,
        };

        if let Some(token_type_idx) = token_type {
            let length = (node.end_byte() - node.start_byte()) as u32;
            tokens.push(SemanticToken {
                delta_line: start.row as u32,
                delta_start: start.column as u32,
                length,
                token_type: token_type_idx,
                token_modifiers_bitset: 0,
            });
        }

        // Only recurse if we're not inside a suffix
        if !is_in_suffix {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                self.collect_semantic_tokens(child, tokens);
            }
        }
    }

    /// Format the document using melbi-fmt
    #[must_use]
    pub fn format(&self) -> Option<String> {
        melbi_fmt::format(&self.source, false, true).ok()
    }
}
