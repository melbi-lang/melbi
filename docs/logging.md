# Logging in Melbi

This document describes the logging infrastructure in melbi-core and how to use it for debugging and development.

## Overview

Melbi uses the [`tracing`](https://docs.rs/tracing) crate for structured logging with **compile-time level filtering** for zero-cost abstractions.

### Why Tracing?

- **Structured logging**: Capture key-value pairs, not just text
- **Hierarchical context**: Track nested operations with spans
- **no_std compatible**: Works with melbi-core's no_std design
- **Industry standard**: The modern standard for Rust logging (2025)
- **Zero cost in release**: Debug/trace levels compiled out via `release_max_level_warn`

## How Logging Works

### Compile-Time Filtering

Melbi uses tracing's `release_max_level_warn` feature:

- **Debug builds**: All log levels available (trace, debug, info, warn, error)
- **Release builds**: Only warn and error levels compiled in (debug/trace removed)
- **Result**: Zero runtime overhead in production, full logging during development

### Runtime Configuration

Logging output is controlled via environment variables:

#### Using RUST_LOG

```bash
# Show all logs at DEBUG level
RUST_LOG=debug cargo run -p melbi-cli

# Show only melbi-core logs at TRACE level
RUST_LOG=melbi_core=trace cargo run -p melbi-cli

# Show specific modules
RUST_LOG=melbi_core::analyzer=debug cargo run -p melbi-cli
RUST_LOG=melbi_core::types::unification=trace cargo run -p melbi-cli

# Multiple modules with different levels
RUST_LOG=melbi_core::analyzer=debug,melbi_core::evaluator=trace cargo run -p melbi-cli
```

#### CLI Usage Examples

```bash
# Normal usage - silent (only warnings/errors)
cargo run -p melbi-cli -- "1 + 2"

# Debug type inference
RUST_LOG=debug cargo run -p melbi-cli -- "1 + 2"

# Trace every unification step
RUST_LOG=melbi_core::types::unification=trace cargo run -p melbi-cli -- "x + y"

# Interactive REPL with logging
RUST_LOG=debug cargo run -p melbi-cli
```

## Log Levels

Melbi uses the standard log levels:

- **ERROR**: Fatal errors (shouldn't happen in normal operation)
- **WARN**: Recoverable issues, type inference failures
- **INFO**: High-level operations (analysis started, compilation phases)
- **DEBUG**: Detailed type checking steps, unification decisions
- **TRACE**: Every expression analyzed, every type variable created

### What's Logged at Each Level

#### ERROR
- Internal invariant violations
- Unexpected error conditions

#### WARN (Default for CLI)
- Occurs check failures in unification
- Type constraint violations
- Partial analysis failures

#### INFO
- Analysis started (with scope info)
- Compilation phase transitions
- Major operations completed

#### DEBUG
- Type unification attempts
- Type variable bindings
- Type expectation checks
- Expression type inference decisions

#### TRACE
- Every expression kind analyzed
- Fresh type variable creation
- Resolved types after substitution
- Successful unification fast paths

## Instrumented Modules

The following modules have logging instrumentation:

### Type Unification (`types::unification`)

**Key operations logged:**
- `unifies_to()`: Every unification attempt (DEBUG)
- Type resolution before unification (TRACE)
- Type variable bindings (DEBUG)
- Occurs check failures (WARN)
- Successful fast-path equality (TRACE)

**Example output:**
```
DEBUG melbi_core::types::unification: Attempting unification t1=_0 t2=Int
TRACE melbi_core::types::unification: Types after resolution t1_resolved=_0 t2_resolved=Int
DEBUG melbi_core::types::unification: Binding type variable var_id=0 binding=Int
```

### Type Analyzer (`analyzer::analyzer`)

**Key operations logged:**
- `analyze()`: Entry point for type analysis (INFO)
- Expression analysis by kind (TRACE)
- Type expectation checks (DEBUG)

**Example output:**
```
INFO melbi_core::analyzer: Starting type analysis globals_count=0 variables_count=0
TRACE melbi_core::analyzer: Analyzing expression expr_kind="Binary"
DEBUG melbi_core::analyzer: Checking type expectation context="operands must have same type" got=Int expected=Int
```

### Type Manager (`types::manager`)

**Key operations logged:**
- Fresh type variable creation (TRACE)

**Example output:**
```
TRACE melbi_core::types::manager: Creating fresh type variable var_id=0
TRACE melbi_core::types::manager: Creating fresh type variable var_id=1
```

## Using Logging in Tests

### Enable Logging for Specific Tests

```rust
#[cfg(test)]
mod tests {
    use crate::test_utils::init_test_logging;

    #[test]
    fn type_inference() {
        init_test_logging();  // Call this to enable logging
        
        // Your test code here
        // Logs will appear in test output
    }
}
```

### Running Tests with Logging

```bash
# Run all tests with logging at DEBUG level
cargo test -p melbi-core

# Run specific test with TRACE level
RUST_LOG=trace cargo test -p melbi-core test_type_inference

# Show test output including logs (don't capture)
cargo test -p melbi-core -- --nocapture

# Specific module at TRACE level
RUST_LOG=melbi_core::types::unification=trace cargo test -p melbi-core
```

**Note**: In debug builds, all log levels are available. In release builds (`cargo test --release`), debug and trace logs are compiled out.

## Debugging Type Inference

Type inference is the most common use case for logging. Here's a workflow:

### 1. Identify the Expression

```bash
# Your expression that has type issues
echo "x + y" | RUST_LOG=debug cargo run -p melbi-cli
```

### 2. Enable Unification Tracing

```bash
# See every unification step
echo "x + y" | RUST_LOG=melbi_core::types::unification=trace cargo run -p melbi-cli
```

**Example output:**
```
DEBUG melbi_core::types::unification: Attempting unification t1=_0 t2=_1
TRACE melbi_core::types::unification: Types after resolution t1_resolved=_0 t2_resolved=_1
DEBUG melbi_core::types::unification: Binding type variable var_id=0 binding=_1
```

### 3. Track Type Variable Creation

```bash
# See when type variables are allocated
echo "x + y" | RUST_LOG=melbi_core::types=trace cargo run -p melbi-cli
```

### 4. Combine Multiple Module Traces

```bash
# Full type checking trace
echo "x + y" | RUST_LOG=melbi_core::analyzer=debug,melbi_core::types=trace cargo run -p melbi-cli
```

## Common Debugging Scenarios

### Debug "Type Mismatch" Errors

```bash
# Enable unification logging to see what types are being unified
RUST_LOG=melbi_core::types::unification=debug cargo run -p melbi-cli -- "your expression"
```

### Debug "Occurs Check Failed" Errors

```bash
# Occurs check failures are logged at WARN level
RUST_LOG=warn cargo run -p melbi-cli -- "your expression"
```

### Track Function Type Inference

```bash
# See how lambda parameters are typed
RUST_LOG=melbi_core::analyzer=trace cargo run -p melbi-cli -- "(x) => x + 1"
```

### Debug Record/Map Type Inference

```bash
# Trace record field unification
RUST_LOG=trace cargo run -p melbi-cli -- "{x: 1, y: 2.0}"
```

## Performance Considerations

### Zero Cost in Release Builds

With `release_max_level_warn` configured:
- **Debug builds**: All log levels available (full overhead possible)
- **Release builds**: Debug/trace code **completely removed** at compile time
- **Binary size**: Release binaries don't include debug/trace logging code
- **Production ready**: Zero overhead for detailed logging in production

### Minimal Cost in Debug Builds

When logging is compiled in but filtered out at runtime:
- **Cheap checks**: Fast check if log level is enabled
- **No allocations**: No work done if log level is filtered
- **Acceptable for development**: Suitable for development builds

### Cost When Actively Logging

When running with `RUST_LOG=trace`:
- **Some overhead**: String formatting and I/O
- **Acceptable for debugging**: Worth it for debugging type inference
- **Debug builds only**: Automatically disabled in release builds

## Adding More Logging

### Guidelines for Adding Logs

When adding new logging instrumentation:

1. **Use appropriate levels**:
   - TRACE: Low-level details (every operation)
   - DEBUG: Key decisions (important steps)
   - INFO: Major operations (top-level phases)
   - WARN: Issues that might indicate problems
   - ERROR: Serious issues

2. **Use structured fields**:
   ```rust
   // Good: Structured
   tracing::debug!(var_id = id, binding = %ty, "Binding type variable");
   
   // Less good: Just text
   tracing::debug!("Binding type variable {} to {}", id, ty);
   ```

3. **No feature guards needed**:
   ```rust
   // Just use tracing directly - it's always available
   tracing::debug!("Your message");
   
   // Compile-time filtering handles the rest
   // Debug/trace removed in release builds automatically
   ```

4. **Include context**:
   ```rust
   tracing::debug!(
       context = "function call",
       arg_count = args.len(),
       "Type checking function arguments"
   );
   ```

### Example: Adding Logging to a New Module

```rust
pub fn my_function(x: Type, y: Type) -> Result<Type> {
    tracing::debug!(
        x = %display_type(x),
        y = %display_type(y),
        "Entering my_function"
    );
    
    // ... your code ...
    
    tracing::trace!(result = %display_type(result), "Function completed");
    
    Ok(result)
}
```

## Integration with Other Tools

### With LSP Server

The LSP server can be configured to use logging (future work):
- Log to stderr for development
- Use `client.log_message()` for important events
- File-based logging for production debugging

### With Profiling

Logging complements the planned profiling system:
- **Logging**: Development debugging, trace-level insights
- **Profiling**: Production performance monitoring
- **Metrics**: Aggregated statistics

Both share the same span infrastructure.

## Troubleshooting

### Logs Not Appearing

1. **Check feature flag**: `cargo build --features logging`
2. **Check environment variable**: `RUST_LOG=debug`
3. **Check output**: Logs go to stderr, not stdout

### Too Much Output

1. **Filter by module**: `RUST_LOG=melbi_core::analyzer=debug`
2. **Raise log level**: Use `info` instead of `debug`
3. **Target specific operations**: Filter to just the module you care about

### Logs in Tests Not Showing

1. **Use `--nocapture`**: `cargo test -- --nocapture`
2. **Enable feature**: `cargo test --features logging`
3. **Call `init_test_logging()`** in your test

## Examples

### Example 1: Debug Type Inference for Lambda

```bash
RUST_LOG=melbi_core=debug cargo run -p melbi-cli -- "(x, y) => x + y"
```

**Output:**
```
INFO melbi_core::analyzer: Starting type analysis globals_count=0 variables_count=0
TRACE melbi_core::types::manager: Creating fresh type variable var_id=0
TRACE melbi_core::types::manager: Creating fresh type variable var_id=1
DEBUG melbi_core::types::unification: Attempting unification t1=_0 t2=_1
DEBUG melbi_core::types::unification: Binding type variable var_id=0 binding=_1
```

### Example 2: Trace Unification Only

```bash
RUST_LOG=melbi_core::types::unification=trace cargo run -p melbi-cli -- "[1, 2, 3]"
```

### Example 3: Debug Test with Logging

```rust
#[test]
fn array_type_inference() {
    test_utils::init_test_logging();
    
    let arena = Bump::new();
    let tm = TypeManager::new(&arena);
    // ... test code
}
```

Run with:
```bash
cargo test -p melbi-core test_array_type_inference -- --nocapture
```

## Future Enhancements

Potential future additions:
- Span instrumentation for hierarchical traces
- Performance tracking within logs
- Integration with tokio-console for async operations
- LSP server logging integration
- Flamegraph generation from trace data

## References

- [tracing documentation](https://docs.rs/tracing)
- [tracing-subscriber documentation](https://docs.rs/tracing-subscriber)
- [RUST_LOG environment variable](https://docs.rs/env_logger/latest/env_logger/#enabling-logging)
