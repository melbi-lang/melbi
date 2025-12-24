use bumpalo::Bump;
use js_sys::JSON;
use melbi_core::api::{
    Diagnostic as CoreDiagnostic, Engine, EngineOptions, Error, RelatedInfo, Severity,
};
use melbi_core::parser::Span;
use melbi_core::stdlib;
use melbi_core::values::dynamic::Value;
use serde::Serialize;
use wasm_bindgen::prelude::*;
use web_sys::window;

#[wasm_bindgen]
pub struct PlaygroundEngine {
    engine_arena: &'static Bump,
    engine: Engine<'static>,
}

#[wasm_bindgen]
impl PlaygroundEngine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> PlaygroundEngine {
        let arena = Box::leak(Box::new(Bump::new()));
        let engine = Engine::new(
            EngineOptions::default(),
            arena,
            |arena, type_mgr, env_builder| {
                stdlib::register_stdlib(arena, type_mgr, env_builder)
                    .expect("registration should succeed")
            },
        );

        PlaygroundEngine {
            engine_arena: arena,
            engine,
        }
    }

    /// Compile and execute the provided Melbi expression.
    #[wasm_bindgen]
    pub fn evaluate(&self, source: &str) -> Result<JsValue, JsValue> {
        let response = self.evaluate_internal(source);
        to_js_value(&response)
    }
}

impl PlaygroundEngine {
    fn evaluate_internal(&self, source: &str) -> WorkerResponse<EvaluationSuccess> {
        let source_in_arena = self.engine_arena.alloc_str(source);
        let source_ref: &'static str = source_in_arena;
        let compile_result = self.engine.compile(Default::default(), source_ref, &[]);

        match compile_result {
            Ok(expr) => {
                let value_arena = Bump::new();

                // Measure evaluation time (not including compilation)
                let start = window()
                    .and_then(|w| w.performance())
                    .map(|p| p.now())
                    .unwrap_or(0.0);

                let result = expr.run(Default::default(), &value_arena, &[]);

                let end = window()
                    .and_then(|w| w.performance())
                    .map(|p| p.now())
                    .unwrap_or(0.0);

                let duration_ms = end - start;

                match result {
                    Ok(value) => {
                        WorkerResponse::ok(EvaluationSuccess::from_value(value, duration_ms))
                    }
                    Err(err) => WorkerResponse::err(err),
                }
            }
            Err(err) => WorkerResponse::err(err),
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum WorkerResponse<T> {
    Ok { data: T },
    Err { error: WorkerError },
}

impl<T> WorkerResponse<T> {
    fn ok(data: T) -> Self {
        WorkerResponse::Ok { data }
    }

    fn err(error: Error) -> Self {
        WorkerResponse::Err {
            error: WorkerError::from(error),
        }
    }
}

#[derive(Serialize)]
pub struct WorkerError {
    kind: &'static str,
    message: String,
    diagnostics: Option<Vec<DiagnosticPayload>>,
}

#[derive(Serialize)]
pub struct DiagnosticPayload {
    severity: &'static str,
    message: String,
    span: RangePayload,
    help: Option<String>,
    code: Option<String>,
    related: Vec<RelatedInfoPayload>,
}

#[derive(Serialize)]
pub struct RelatedInfoPayload {
    span: RangePayload,
    message: String,
}

#[derive(Serialize)]
pub struct RangePayload {
    start: usize,
    end: usize,
}

#[derive(Serialize)]
pub struct EvaluationSuccess {
    value: String,
    type_name: String,
    duration_ms: f64,
}

impl EvaluationSuccess {
    fn from_value(arg: Value<'static, '_>, duration_ms: f64) -> Self {
        let mut value = String::new();
        html_escape::encode_safe_to_string(format!("{:?}", arg), &mut value);
        let mut type_name = String::new();
        html_escape::encode_safe_to_string(format!("{}", arg.ty), &mut type_name);
        Self {
            value,
            type_name,
            duration_ms,
        }
    }
}

impl From<Error> for WorkerError {
    fn from(err: Error) -> Self {
        match err {
            Error::Api(message) => WorkerError {
                kind: "api",
                message,
                diagnostics: None,
            },
            Error::Compilation { diagnostics, .. } => WorkerError {
                kind: "compilation",
                message: format!(
                    "Compilation failed with {} diagnostic(s)",
                    diagnostics.len()
                ),
                diagnostics: Some(
                    diagnostics
                        .into_iter()
                        .map(DiagnosticPayload::from)
                        .collect(),
                ),
            },
            Error::Runtime { diagnostic, .. } => WorkerError {
                kind: "runtime",
                message: diagnostic.message.clone(),
                diagnostics: Some(vec![DiagnosticPayload::from(diagnostic)]),
            },
            Error::ResourceExceeded(message) => WorkerError {
                kind: "resource_exceeded",
                message,
                diagnostics: None,
            },
        }
    }
}

impl From<CoreDiagnostic> for DiagnosticPayload {
    fn from(diag: CoreDiagnostic) -> Self {
        Self {
            severity: severity_to_str(diag.severity),
            message: diag.message,
            span: RangePayload::from(diag.span),
            help: diag.help.get(0).map(|s| s.clone()),
            code: diag.code,
            related: diag
                .related
                .into_iter()
                .map(RelatedInfoPayload::from)
                .collect(),
        }
    }
}

impl From<RelatedInfo> for RelatedInfoPayload {
    fn from(info: RelatedInfo) -> Self {
        Self {
            span: RangePayload::from(info.span),
            message: info.message,
        }
    }
}

impl From<Span> for RangePayload {
    fn from(span: Span) -> Self {
        RangePayload {
            start: span.0.start,
            end: span.0.end,
        }
    }
}

fn severity_to_str(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

fn to_js_value<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    let serialized = serde_json::to_string(value)
        .map_err(|err| JsValue::from_str(&format!("serialization error: {}", err)))?;
    JSON::parse(&serialized).map_err(|err| err)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // This test uses WASM-specific APIs (window().performance()) and fails outside WASM environments.
    fn evaluates_basic_expression() {
        let engine = PlaygroundEngine::new();
        match engine.evaluate_internal("40 + 2") {
            WorkerResponse::Ok { data } => {
                assert_eq!(data.value, "42");
                assert_eq!(data.type_name, "Int");
                assert!(data.duration_ms >= 0.0);
            }
            WorkerResponse::Err { error } => panic!("evaluation failed: {}", error.message),
        }
    }
}
