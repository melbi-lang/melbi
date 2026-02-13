# Fix: Add Call Stack Traces to Execution Errors

> **Note:** This design document was generated with AI assistance and has not
> been fully reviewed. The general approach is sound, but implementation details
> (e.g., instrumentation strategy, output formatting) may change.

## Problem

Execution errors (from FFI functions and lambdas) lack proper call stack information:
- FFI functions return `ExecutionError` with dummy `Span(0..0)` and empty source
- Lambda errors show where the error occurred inside, but not the call chain
- Users can't see the full context of how an error was reached

Example:

```bash
% melbi eval 'f(0) where { f = (x) => 1/x }'
[R001] Error: Division by zero
   ╭─[ <unknown>:1:25 ]
   │
 1 │ f(0) where { f = (x) => 1/x }
   │                         ─┬─
   │                          ╰─── Division by zero
   │
   │ Help: Check that divisor is not zero before division
───╯
```

## Solution: Store `tracing::Span` handle, rehydrate to `SpanTrace` on display

Key insight: `tracing::Span` is no_std compatible (just a handle), while `SpanTrace` requires std.
By storing the Span handle at error creation and "rehydrating" it to SpanTrace on the consumer side,
we get call stack traces while keeping melbi-core no_std.

### How It Works

1. **Instrument evaluator functions** with `#[instrument]`, including Melbi source locations as fields
2. **Store Span handle on error** - When error is created, store `tracing::Span::current()`
3. **Rehydrate on consumer side** - Consumer enters the span scope and captures SpanTrace
4. **Display** - SpanTrace formats like a stack trace showing all Melbi call locations

### Benefits

- melbi-core stays no_std compatible
- Works for both FFI and lambda errors
- Captures the full call chain, not just the immediate error location
- Integrates with existing tracing infrastructure (see `docs/logging.md`)
- Virtually zero overhead when tracing is disabled

## Implementation Steps

### 1. Add `tracing::Span` field to ExecutionError

**File:** `core/src/evaluator/error.rs`

```rust
pub struct ExecutionError {
    pub kind: ExecutionErrorKind,
    pub source: String,
    pub source_span: Span,               // Melbi source location (may be 0..0 for FFI)
    pub tracing_span: tracing::Span,     // Handle to tracing span hierarchy
}

impl ExecutionError {
    /// Create error with captured tracing span
    pub fn new(kind: ExecutionErrorKind, source: String, source_span: Span) -> Self {
        Self {
            kind,
            source,
            source_span,
            tracing_span: tracing::Span::current(),
        }
    }
}
```

### 2. Instrument the evaluator's Call handling

**File:** `core/src/evaluator/eval.rs`

```rust
use tracing::instrument;

impl<'types, 'arena> Evaluator<'types, 'arena, '_> {
    #[instrument(
        skip_all,
        fields(
            call_site = %format_span(&expr.span),  // e.g., "line 5, col 10"
        )
    )]
    fn eval_call(
        &mut self,
        expr: &'arena Expr<'types, 'arena>,
        callable: &'arena Expr<'types, 'arena>,
        args: &'arena [Expr<'types, 'arena>],
    ) -> Result<Value<'types, 'arena>, ExecutionError> {
        // ... existing implementation
    }
}
```

### 3. Update all error creation sites

Anywhere `ExecutionError` is created, use the new constructor:

```rust
// Before
ExecutionError { kind, source, span }

// After
ExecutionError::new(kind, source, span)
```

### 4. Update FFI macro

**File:** `melbi_macros/src/lib.rs`

The `#[melbi_fn]` macro should create errors using `ExecutionError::new()`.

### 5. Add `tracing-error` to CLI

**File:** `cli/Cargo.toml`

```toml
[dependencies]
tracing-error = "0.2"
```

### 6. Enable ErrorLayer in CLI subscriber

**File:** `cli/src/main.rs`

```rust
use tracing_error::ErrorLayer;
use tracing_subscriber::prelude::*;

let subscriber = tracing_subscriber::registry()
    .with(tracing_subscriber::fmt::layer()...)
    .with(ErrorLayer::default());  // <-- enables SpanTrace capture

tracing::subscriber::set_global_default(subscriber).unwrap();
```

### 7. Rehydrate SpanTrace when displaying errors

**File:** `cli/src/main.rs` or error rendering code

```rust
fn render_error_with_trace(error: &ExecutionError) {
    // Enter the error's span context and capture full trace
    let span_trace = error.tracing_span.in_scope(|| {
        tracing_error::SpanTrace::capture()
    });

    // Display error kind
    eprintln!("Error: {}", error.kind);

    // Display Melbi source location if available
    if error.source_span != Span(0..0) {
        eprintln!("  at {}", error.source_span);
    }

    // Display call stack
    eprintln!("\nCall stack:");
    eprintln!("{}", span_trace);
}
```

## Files to Modify

1. `core/src/evaluator/error.rs` - Rename `span` to `source_span`, add `tracing_span` field
2. `core/src/evaluator/eval.rs` - Instrument Call handling with `#[instrument]`
3. `melbi_macros/src/lib.rs` - Update error creation in macro
4. `cli/Cargo.toml` - Add tracing-error dependency
5. `cli/src/main.rs` - Add ErrorLayer, rehydrate SpanTrace on display

## Example Output

```text
Error: Division by zero

Call stack:
   0: eval_call
           with call_site="line 3, col 15"
             at core/src/evaluator/eval.rs:640
   1: eval_call
           with call_site="line 7, col 1"
             at core/src/evaluator/eval.rs:640
```

## Future Enhancements

- Include function names in span fields (when available from lambda metadata)
- Custom SpanTrace formatting to show only Melbi source locations (filter out Rust internals)
- Add source code snippet display for each call site
