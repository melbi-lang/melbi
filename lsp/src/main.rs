use dashmap::DashMap;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{
    CompletionOptions, CompletionParams, CompletionResponse, DidChangeTextDocumentParams,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, DocumentFormattingParams, Hover,
    HoverContents, HoverParams, HoverProviderCapability, InitializeParams, InitializeResult,
    InitializedParams, MarkupContent, MarkupKind, MessageType, OneOf, Position, Range,
    SemanticTokens, SemanticTokensFullOptions, SemanticTokensOptions, SemanticTokensParams,
    SemanticTokensResult, SemanticTokensServerCapabilities, ServerCapabilities, ServerInfo,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextEdit, Url,
};
use tower_lsp::{Client, LanguageServer, LspService, Server};

mod document;
mod semantic_tokens;

use document::DocumentState;

#[derive(Debug)]
struct Backend {
    client: Client,
    /// Document cache, keyed by URI
    documents: DashMap<Url, DocumentState>,
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            documents: DashMap::new(),
        }
    }

    /// Analyze a document and publish diagnostics
    async fn analyze_document(&self, uri: Url) {
        // Analyze the document
        let all_diagnostics = {
            if let Some(mut doc) = self.documents.get_mut(&uri) {
                doc.analyze()
            } else {
                Vec::new()
            }
        }; // DashMap reference dropped here

        // Publish diagnostics
        self.client
            .publish_diagnostics(uri, all_diagnostics, None)
            .await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".to_string()]),
                    ..Default::default()
                }),
                document_formatting_provider: Some(OneOf::Left(true)),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: semantic_tokens::get_legend(),
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            range: None,
                            ..Default::default()
                        },
                    ),
                ),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "Melbi Language Server".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Melbi LSP initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "File opened!")
            .await;

        let document = params.text_document;
        let uri = document.uri;
        let source = document.text;

        // Create document state
        let doc_state = DocumentState::new(source);
        self.documents.insert(uri.clone(), doc_state);

        // Analyze and publish diagnostics
        self.analyze_document(uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "File changed!")
            .await;

        let DidChangeTextDocumentParams {
            text_document,
            content_changes,
        } = params;
        let uri = text_document.uri;

        // We're using FULL sync, so there should be exactly one change
        if let Some(change) = content_changes.into_iter().next() {
            // Update document
            if let Some(mut doc) = self.documents.get_mut(&uri) {
                doc.update(change.text);
            }

            // Analyze and publish diagnostics
            self.analyze_document(uri).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        // Remove document from cache
        self.documents.remove(&params.text_document.uri);
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let hover_text = {
            self.documents
                .get(&uri)
                .and_then(|doc| doc.hover_at_position(position))
        }; // DashMap reference dropped here

        Ok(hover_text.map(|text| Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: text,
            }),
            range: None,
        }))
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        let items = {
            self.documents
                .get(&uri)
                .map(|doc| doc.completions_at_position(position))
                .unwrap_or_default()
        }; // DashMap reference dropped here

        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;

        // Get formatted text and source, then drop the DashMap reference
        let (formatted, source) = {
            match self.documents.get(&uri) {
                Some(doc) => {
                    let formatted = doc.format();
                    let source = doc.source.clone();
                    (formatted, source)
                }
                None => return Ok(None),
            }
        }; // DashMap reference dropped here

        if let Some(formatted_text) = formatted {
            // If the formatted text is the same, no edits needed
            if formatted_text == source {
                return Ok(None);
            }

            // Calculate the range of the entire document
            // Count actual lines (including empty ones) and get the length of the last line
            let line_count = source.chars().filter(|&c| c == '\n').count();
            let last_line_start = source.rfind('\n').map_or(0, |pos| pos + 1);
            let last_line_len = source.len() - last_line_start;

            let range = Range {
                start: Position::new(0, 0),
                end: Position::new(line_count as u32, last_line_len as u32),
            };

            Ok(Some(vec![TextEdit {
                range,
                new_text: formatted_text,
            }]))
        } else {
            self.client
                .log_message(MessageType::ERROR, "Format error".to_string())
                .await;
            Ok(None)
        }
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri;

        let tokens = {
            self.documents
                .get(&uri)
                .and_then(|doc| doc.semantic_tokens())
        }; // DashMap reference dropped here

        Ok(tokens.map(|data| {
            SemanticTokensResult::Tokens(SemanticTokens {
                result_id: None,
                data,
            })
        }))
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
