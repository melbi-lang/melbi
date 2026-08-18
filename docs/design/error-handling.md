# Design Doc: Error Handling, Collection, and Public API

**Author**: @NiltonVolpato

**Date**: 10-26-2025

## Introduction

### Background

Melbi aims to be an embeddable expression language with excellent error messages. Research shows that compiler error quality significantly impacts developer productivity ([amazingcto.com comparison](https://www.amazingcto.com/developer-productivity-compiler-errors/)). Languages like Rust and Elm demonstrate that detailed, helpful error messages with multiple code locations and suggestions dramatically improve the user experience.

Current problems with error handling:
- **Internal errors were using miette**: Heavy std dependency, feature flag complexity, not suitable for no_std core
- **No error collection**: Stops at first error, forcing users to fix one issue at a time
- **No public API design**: Internal error types would leak to users, causing breaking changes
- **Inconsistent context**: No standard way to add provenance information to errors

**Stakeholders**:
- **Melbi users**: Need clear, actionable error messages
- **IDE/LSP developers**: Need structured diagnostics with spans
- **Embedding applications**: Need stable API that won't break
- **Melbi maintainers**: Need freedom to evolve error messages

### Current Functionality

Currently (as of this design):
- Errors are defined per-module with various structures
- miette is used for formatting (std-only, not compatible with no_std goal)
- Single error per operation (no collection)
- No clear boundary between internal and public error types

### In Scope

This design addresses:
1. **Internal error representation**: Rich, structured errors with full context
2. **Error collection mechanism**: Ability to return multiple errors from compilation
3. **Public API design**: Stable, opaque error types for library users
4. **Error categories**: Clear distinction between API, compilation, runtime, and resource errors
5. **LSP integration**: Structure that maps cleanly to LSP diagnostics
6. **no_std compatibility**: Core errors work without std, formatting optional

### Out of Scope

- **Error recovery strategies**: How to continue after errors (parser/type checker specific)
- **Specific error messages**: Exact wording (evolves over time)
- **IDE integration implementation**: LSP server implementation details
- **Error analytics/telemetry**: Tracking which errors users encounter
- **Localization/i18n**: Multi-language error messages

### Assumptions & Dependencies

- Arena-based allocation is used for AST and types
- Spans are tracked for all expressions
- Type checker will be refactored to support error collection
- Public API stability is a priority

### Terminology

- **Diagnostic**: User-facing error/warning with span, message, and context
- **ErrorKind**: Specific type of error (TypeMismatch, UnboundVariable, etc.)
- **Context**: Breadcrumb trail explaining how we reached the error
- **Span**: Source location (line, column range)
- **Error collection**: Accumulating multiple errors before returning
- **Public API**: Stable error types exposed to library users
- **Internal errors**: Rich error types used within Melbi implementation

## Considerations

### Concerns

**Complexity**: Error handling can become complex with context chains, collection, and multiple representations. Must balance richness with maintainability.

**Performance**: Error collection requires allocating vectors and accumulating errors. Must ensure this doesn't impact the happy path.

**no_std compatibility**: Core library should work without std, but still provide good error messages when std is available.

**API stability**: Once public API is released, changes are breaking. Must get the design right.

**Memory usage**: Accumulating errors in arenas might increase memory usage. Need to profile.

### Operational Readiness Considerations

**Metrics to track**:
- Average number of errors per failed compilation
- Most common error types
- Time spent in error collection vs type checking
- Memory overhead of error collection

**Debugging**:
- Each error includes full context chain
- Spans point to exact source locations
- Internal errors preserve maximum detail

**Monitoring**:
- Count of errors by ErrorKind
- Error collection performance
- Public API usage patterns

### Open Questions

1. **Should warnings be separate from errors, or unified?**
   - **Answer**: Separate - different severity levels, different handling

2. **How many errors should we collect before stopping?**
   - **Answer**: No hard limit initially, add if needed for performance

3. **Should error codes be stable across versions?**
   - **Answer**: Yes - document error codes and maintain compatibility

4. **Should Context be generic over error type, or shared?**
   - **Answer**: Shared - contexts are similar across subsystems

### Cross-Region Considerations

Not applicable - this is a library design without deployment concerns.

## Proposed Design

### Solution

Implement a three-layer error system:

1. **Internal layer**: Rich, structured errors with full context (per subsystem)
2. **Collection layer**: Mechanism to accumulate multiple errors
3. **Public layer**: Stable, opaque error types for library users

This separation allows:
- Internal evolution without breaking users
- Multiple error reporting for better UX
- Clean LSP integration
- no_std core with optional std formatting

### System Architecture

```
┌─────────────────────────────────────┐
│         Public API Layer            │
│  (melbi crate, stable interface)    │
│                                     │
│  pub enum Error {                   │
│    Api(String),                     │
│    Compilation { diagnostics },     │
│    Runtime(String),                 │
│    ResourceExceeded(String),        │
│  }                                  │
└──────────────┬──────────────────────┘
               │
               │ Conversion
               ↓
┌─────────────────────────────────────┐
│      Error Collection Layer         │
│  (core/src/diagnostics)             │
│                                     │
│  CheckResult<T> {                   │
│    value: Option<T>,                │
│    errors: Vec<InternalError>,      │
│    warnings: Vec<InternalError>,    │
│  }                                  │
└──────────────┬──────────────────────┘
               │
               │ Used by
               ↓
┌─────────────────────────────────────┐
│       Internal Error Layer          │
│   (per-subsystem error types)       │
│                                     │
│  ParseError { kind, context }       │
│  TypeError { kind, context }        │
│  EvalError { kind, context }        │
└─────────────────────────────────────┘
```

### Data Model

#### Internal Errors (per subsystem)

```rust
// In core/src/parser/error.rs
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub context: Vec<Context>,
}

pub enum ParseErrorKind {
    UnexpectedToken { expected: String, found: Token, span: Span },
    UnclosedDelimiter { delimiter: char, span: Span },
    InvalidNumber { text: String, span: Span },
    // ... more variants
}

// In core/src/analyzer/error.rs
pub struct TypeError {
    pub kind: TypeErrorKind,
    pub context: Vec<Context>,
}

pub enum TypeErrorKind {
    TypeMismatch { expected: Type, found: Type, span: Span },
    UnboundVariable { name: String, span: Span },
    UnhandledError { span: Span },
    OccursCheck { var: TypeVar, ty: Type, span: Span },
    // ... more variants
}

// In core/src/evaluator/error.rs
pub struct EvalError {
    pub kind: EvalErrorKind,
    // Note: Runtime errors don't use Context chain - they show runtime values instead
}

pub enum EvalErrorKind {
    DivisionByZero {
        span: Span,
        divisor_expr: String,  // Show what evaluated to 0
    },
    IndexOutOfBounds {
        span: Span,
        index: i64,
        len: usize,
        array_preview: String,  // First few elements
    },
    KeyNotFound {
        span: Span,
        key: String,           // Show the actual key that wasn't found
        available_keys: Vec<String>,  // Show what keys exist (if small map)
    },
    StackOverflow { span: Span },
    // ... more variants
}
```

#### Context (shared across parser and analyzer)

```rust
// In core/src/diagnostics/context.rs
// Note: Context is primarily for compile-time errors (parser and analyzer)
// Runtime errors show actual values instead of context chains
pub enum Context {
    InFunctionCall {
        name: Option<String>,
        span: Span,
    },
    WhileUnifying {
        what: String,  // "argument 0", "return type"
        span: Span,
    },
    DefinedHere {
        what: String,  // "variable", "function"
        span: Span,
    },
    InferredHere {
        type_name: String,
        span: Span,
    },
    InExpression {
        kind: String,  // "array literal", "if expression"
        span: Span,
    },
}

impl Context {
    pub fn to_related_info(&self) -> RelatedInfo {
        match self {
            Context::InFunctionCall { name, span } => {
                RelatedInfo {
                    span: *span,
                    message: match name {
                        Some(n) => format!("in call to function '{}'", n),
                        None => "in function call".to_string(),
                    },
                }
            }
            Context::WhileUnifying { what, span } => {
                RelatedInfo {
                    span: *span,
                    message: format!("while checking {}", what),
                }
            }
            // ... other conversions
        }
    }
}
```

#### Error Collection

```rust
// In core/src/diagnostics/collection.rs
pub struct CheckResult<T> {
    pub value: Option<T>,
    pub errors: Vec<TypeError>,  // Or generic over error type
    pub warnings: Vec<TypeError>,
}

impl<T> CheckResult<T> {
    pub fn ok(value: T) -> Self {
        CheckResult {
            value: Some(value),
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn error(error: TypeError) -> Self {
        CheckResult {
            value: None,
            errors: vec![error],
            warnings: Vec::new(),
        }
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn merge(self, other: Self) -> Self {
        CheckResult {
            value: self.value.or(other.value),
            errors: self.errors.into_iter().chain(other.errors).collect(),
            warnings: self.warnings.into_iter().chain(other.warnings).collect(),
        }
    }
}

pub trait OrCollect<T, E> {
    fn or_collect(self, collector: &mut ErrorCollector<E>) -> Option<T>;
}

impl<T, E> OrCollect<T, E> for Result<T, E> {
    fn or_collect(self, collector: &mut ErrorCollector<E>) -> Option<T> {
        match self {
            Ok(val) => Some(val),
            Err(err) => {
                collector.add(err);
                None
            }
        }
    }
}

pub struct ErrorCollector<E> {
    errors: Vec<E>,
}

impl<E> ErrorCollector<E> {
    pub fn new() -> Self {
        ErrorCollector { errors: Vec::new() }
    }

    pub fn add(&mut self, error: E) {
        self.errors.push(error);
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn into_result<T>(self, value: T) -> CheckResult<T> {
        if self.errors.is_empty() {
            CheckResult::ok(value)
        } else {
            CheckResult {
                value: Some(value),
                errors: self.errors,
                warnings: Vec::new(),
            }
        }
    }
}
```

#### Public API

```rust
// In melbi/src/error.rs (public crate)
#[derive(Debug)]
pub enum Error {
    /// Invalid API usage (e.g., null pointer, invalid UTF-8)
    Api(String),

    /// Compilation errors (parse errors, type errors)
    Compilation {
        diagnostics: Vec<Diagnostic>,
    },

    /// Runtime errors during evaluation
    Runtime(String),

    /// Resource limits exceeded (memory, time, stack)
    ResourceExceeded(String),
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub span: Span,
    pub related: Vec<RelatedInfo>,
    pub help: Option<String>,
    pub code: Option<String>,  // e.g., "E001"
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone)]
pub struct RelatedInfo {
    pub span: Span,
    pub message: String,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Api(msg) => write!(f, "API error: {}", msg),
            Error::Compilation { diagnostics } => {
                write!(f, "Compilation failed with {} error(s)",
                    diagnostics.iter().filter(|d| d.severity == Severity::Error).count())
            }
            Error::Runtime(msg) => write!(f, "Runtime error: {}", msg),
            Error::ResourceExceeded(msg) => write!(f, "Resource limit exceeded: {}", msg),
        }
    }
}

impl std::error::Error for Error {}
```

### Interface / API Definitions

#### Type Checker Interface

```rust
impl<'a> TypeChecker<'a> {
    /// Type check an expression, collecting multiple errors
    pub fn check_expr(&mut self, expr: &Expr<'a>) -> CheckResult<Type<'a>> {
        let mut collector = ErrorCollector::new();

        let result = match expr {
            Expr::Binary { op, left, right } => {
                let left_ty = self.check_expr(left).or_collect(&mut collector)?;
                let right_ty = self.check_expr(right).or_collect(&mut collector)?;

                // Both sub-expressions succeeded, now check operation
                self.check_binary_op(op, left_ty, right_ty, expr)
            }
            // ... other cases
        };

        match result {
            Ok(ty) => collector.into_result(ty),
            Err(e) => {
                collector.add(e);
                CheckResult {
                    value: None,
                    errors: collector.errors,
                    warnings: Vec::new(),
                }
            }
        }
    }

    /// Add context to errors
    fn with_context<T>(&mut self, ctx: Context, f: impl FnOnce(&mut Self) -> CheckResult<T>) -> CheckResult<T> {
        self.context_stack.push(ctx);
        let result = f(self);
        self.context_stack.pop();
        result
    }

    /// Create error with current context
    fn error(&self, kind: TypeErrorKind) -> TypeError {
        TypeError {
            kind,
            context: self.context_stack.clone(),
        }
    }
}
```

#### Public API Usage

```rust
use melbi::{compile, Error};

fn main() {
    let source = "1 + true";

    match compile(source) {
        Ok(program) => {
            println!("Compiled successfully");
        }
        Err(Error::Compilation { diagnostics }) => {
            for diag in diagnostics {
                println!("{}: {}", diag.severity, diag.message);
                println!("  at {}:{}", diag.span.line, diag.span.column);

                for related in diag.related {
                    println!("  note: {}", related.message);
                    println!("    at {}:{}", related.span.line, related.span.column);
                }

                if let Some(help) = diag.help {
                    println!("  help: {}", help);
                }
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
}
```

### Business Logic

#### Error Construction

```rust
impl<'a> TypeChecker<'a> {
    fn unify(&mut self, t1: Type<'a>, t2: Type<'a>) -> Result<Type<'a>, TypeError> {
        if !self.can_unify(t1, t2) {
            return Err(self.error(TypeErrorKind::TypeMismatch {
                expected: t1,
                found: t2,
                span: self.current_span(),
            }));
        }

        Ok(t1)
    }

    fn check_call(&mut self, func: &Expr<'a>, args: &[Expr<'a>]) -> CheckResult<Type<'a>> {
        self.with_context(
            Context::InFunctionCall {
                name: self.get_function_name(func),
                span: self.span_of(func),
            },
            |this| {
                let mut collector = ErrorCollector::new();

                let func_ty = this.check_expr(func).or_collect(&mut collector)?;

                // Check each argument with context
                for (i, arg) in args.iter().enumerate() {
                    this.with_context(
                        Context::WhileUnifying {
                            what: format!("argument {}", i),
                            span: this.span_of(arg),
                        },
                        |this| {
                            let arg_ty = this.check_expr(arg).or_collect(&mut collector)?;
                            this.unify(param_ty, arg_ty).or_collect(&mut collector);
                            CheckResult::ok(())
                        }
                    );
                }

                collector.into_result(result_ty)
            }
        )
    }
}
```

#### Conversion to Public API

```rust
impl TypeError {
    pub fn to_diagnostic(&self) -> Diagnostic {
        let (message, code) = match &self.kind {
            TypeErrorKind::TypeMismatch { expected, found, .. } => (
                format!("Type mismatch: expected {}, found {}", expected, found),
                Some("E001"),
            ),
            TypeErrorKind::UnboundVariable { name, .. } => (
                format!("Undefined variable '{}'", name),
                Some("E002"),
            ),
            // ... other variants
        };

        // Get primary span from ErrorKind
        let span = self.kind.span();

        // Convert context to related info
        let related = self.context.iter()
            .map(|ctx| ctx.to_related_info())
            .collect();

        Diagnostic {
            severity: Severity::Error,
            message,
            span,
            related,
            help: None,  // Can add per-error help text
            code: code.map(|s| s.to_string()),
        }
    }
}

impl CheckResult<TypedExpr> {
    pub fn into_public_error(self) -> Result<TypedExpr, Error> {
        if self.errors.is_empty() {
            Ok(self.value.expect("No errors but no value"))
        } else {
            let diagnostics = self.errors.iter()
                .map(|e| e.to_diagnostic())
                .chain(self.warnings.iter().map(|w| {
                    let mut diag = w.to_diagnostic();
                    diag.severity = Severity::Warning;
                    diag
                }))
                .collect();

            Err(Error::Compilation { diagnostics })
        }
    }
}
```

### Migration Strategy

This is a new design, not a migration. Current ad-hoc error handling will be replaced incrementally:

**Phase 1**: Implement internal error structures
- Define ErrorKind enum with initial variants
- Define Context enum
- Implement Error struct with context chain

**Phase 2**: Implement error collection
- Create CheckResult and ErrorCollector
- Refactor type checker to use CheckResult
- Add .or_collect() extension trait

**Phase 3**: Define public API
- Create public Error enum
- Implement conversion from internal → public
- Update melbi crate to use public API

**Phase 4**: Improve error messages
- Add help text to common errors
- Improve Display formatting
- Add error codes

### Work Required

1. **Core error structures** (~200 lines)
   - ErrorKind enum with ~5 initial variants
   - Context enum with ~5 initial variants
   - Error struct with context chain
   - Display implementations

2. **Error collection** (~150 lines)
   - CheckResult struct
   - ErrorCollector struct
   - OrCollect trait
   - Tests for multiple error collection

3. **Public API** (~150 lines)
   - Public Error enum
   - Diagnostic struct
   - Conversion functions
   - Error method implementations

4. **Type checker refactor** (~500 lines changes)
   - Switch from Result to CheckResult
   - Add error collection throughout
   - Test multiple errors returned

5. **Tests and documentation** (~300 lines)
   - Unit tests for error construction
   - Integration tests for multiple errors
   - Documentation examples

**Total estimate**: ~1300 lines of code + refactoring

### Work Sequence

1. Implement internal error structures (can be incremental)
2. Add error collection mechanism
3. Refactor one module (e.g., unification) to use CheckResult
4. Validate multiple errors work correctly
5. Define public API
6. Implement conversion layer
7. Refactor remaining modules
8. Add error codes and help text
9. Write documentation and examples

### High-level Test Plan

**Unit Tests**:
- ErrorKind construction
- Context chain building
- CheckResult merging
- OrCollect behavior

**Integration Tests**:
```rust
#[test]
fn multiple_type_errors() {
    let source = r#"
        {
            a = 1 + "string",
            b = undefined_var,
            c = [1, "mixed", true]
        }
    "#;

    let result = compile(source);
    assert!(result.is_err());

    let Error::Compilation { diagnostics } = result.unwrap_err() else {
        panic!("Expected compilation error");
    };

    assert_eq!(diagnostics.len(), 3);
    assert!(diagnostics[0].message.contains("Type mismatch"));
    assert!(diagnostics[1].message.contains("Undefined"));
}
```

**Snapshot Tests**:
- Error message formatting
- Full diagnostic output
- Regression tests for error quality

### Deployment Sequence

Not applicable - this is a library design, not a deployment.

## Impact

### Performance

**Positive impacts**:
- Arena allocation means error strings are cheap
- Error collection allows fixing multiple issues per compile
- No heap fragmentation from individual error allocations

**Potential concerns**:
- Vec allocation for error collection (negligible)
- String formatting for messages (only on error path)
- Multiple passes to collect all errors (acceptable trade-off)

**Mitigation**:
- Benchmark type checking with/without collection
- Consider error collection limit if performance issues
- Profile memory usage of error Vecs

### Security

**Sandboxing**:
- ResourceExceeded errors enable safe sandboxing
- Time limits prevent infinite loops
- Memory limits prevent OOM attacks
- Stack depth limits prevent stack overflow

**Information Disclosure**:
- Error messages may reveal internal structure
- Spans could expose source code fragments
- Not a concern for Melbi's use case

### Usability

**Major positive impact**:
- Users see multiple errors at once
- Helpful context and suggestions
- Clear error categories for different handling
- LSP-ready structure for IDE integration

**Based on research**:
- Rust/Elm-style errors improve productivity
- Multiple locations in diagnostics help understanding
- Help text accelerates learning

### Cost Analysis

**Development cost**: ~1 week of implementation + 1 week refactoring

**Maintenance cost**: Low - stable public API means internal changes don't break users

**Runtime cost**: Negligible - errors are on the cold path

## Alternatives

### Alternative 1: Use anyhow throughout

**Pros**: Very ergonomic, minimal code

**Cons**:
- Type erasure loses structured information
- Can't pattern match on error types
- Not suitable for library public API
- Loses LSP integration potential

**Rejected because**: Need structured errors for IDE support

### Alternative 2: Use snafu with feature flags

**Pros**: Derive macros reduce boilerplate, supports no_std

**Cons**:
- Feature flag complexity for error formatting
- Couples public API to derive macro behavior
- Harder to control exact error structure

**Rejected because**: Want full control over public API stability

### Alternative 3: Stop at first error (no collection)

**Pros**: Simpler implementation, less memory

**Cons**:
- Forces users to fix one error at a time
- Reduces productivity significantly
- Not competitive with modern compilers

**Rejected because**: Multiple error collection is essential for good UX

### Alternative 4: Expose internal error types publicly

**Pros**: No conversion overhead, users see full detail

**Cons**:
- Breaking changes every time we improve errors
- Can't evolve error messages without versioning
- Lifetime parameters leak into public API

**Rejected because**: Need stable public API for library users

## Looking into the Future

### Next Steps

1. **Error codes documentation**: Website explaining each error with examples
2. **Error code explanations**: `melbi --explain E001` like rustc
3. **LSP integration**: Map diagnostics directly to LSP protocol
4. **Quick fixes**: Suggest code actions in IDE
5. **Error recovery**: Continue parsing after syntax errors
6. **Internationalization**: Translate error messages

### Nice to Haves

- **Color output**: Colored error messages in terminal
- **Source code snippets**: Show code with highlighting in errors
- **Did you mean?**: Suggest similar names for typos
- **Type diff**: Show detailed type differences for complex types
- **Error analytics**: Track which errors are most common
- **Custom error renderers**: Let users customize error formatting

### Evolution Path

The design supports gradual enhancement:
- Error variants added internally without breaking public API
- Context types expanded as needed
- Display formatting improved continuously
- Error collection extended to parsing (currently just type checking)

The separation between internal and public errors means we can continuously improve error quality without breaking users.

---

**Document Status**: Design phase - ready for implementation
**Next Action**: Implement internal error structures in core/src/diagnostics/
