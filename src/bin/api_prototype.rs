use std::fmt;
use std::marker::PhantomData;

use bumpalo::Bump;
use melbi_core::types::Type;
use melbi_core::types::manager::TypeManager;
use melbi_core::values::dynamic::Value;

// ============================================================================
// Context
// ============================================================================

pub struct Context<'arena> {
    type_manager: &'arena TypeManager<'arena>,
    // In reality: type_arena, packages, etc.
}

impl<'arena> Context<'arena> {
    pub fn new(arena: &'arena Bump) -> Self {
        Self {
            type_manager: TypeManager::new(arena),
        }
    }

    #[must_use]
    pub fn type_manager(&self) -> &TypeManager<'arena> {
        self.type_manager
    }

    pub fn compile<'ctx>(
        &'ctx self,
        source: &str,
        params: &[(&str, &'arena Type<'arena>)],
    ) -> Result<CompiledExpression<'arena>, CompileError>
    where
        'arena: 'ctx,
    {
        // Fake compilation - we'll just store the params and pretend to compile
        println!("Compiling: {source}");

        Ok(CompiledExpression {
            type_manager: self.type_manager,
            source: source.to_string(),
            param_types: params.iter().map(|(_, ty)| *ty).collect(),
            param_names: params.iter().map(|(name, _)| name.to_string()).collect(),
            return_type: self.type_manager.int(), // Always return int for our fake a+b
        })
    }
}

// ============================================================================
// Compiled Expression
// ============================================================================

pub struct CompiledExpression<'arena> {
    type_manager: &'arena TypeManager<'arena>,
    source: String,
    param_types: Vec<&'arena Type<'arena>>,
    param_names: Vec<String>,
    return_type: &'arena Type<'arena>,
}

impl<'arena> CompiledExpression<'arena> {
    pub fn run<'val>(
        &self,
        _arena: &'val Bump,
        args: &[Value<'arena, 'val>],
    ) -> Result<Value<'arena, 'val>, ValidationError> {
        // Validate argument count
        if args.len() != self.param_types.len() {
            return Err(ValidationError::ArgumentCountMismatch {
                expected: self.param_types.len(),
                got: args.len(),
            });
        }

        // Validate argument types
        for (i, (arg, &expected)) in args.iter().zip(&self.param_types).enumerate() {
            if !std::ptr::eq(arg.ty, expected) {
                return Err(ValidationError::TypeMismatch { param_index: i });
            }
        }

        // Fake execution: always compute a + b
        println!("Executing: {}", self.source);
        let a = args[0].as_int()?;
        let b = args[1].as_int()?;
        let result = a + b;

        Ok(Value::int(self.type_manager, result))
    }

    #[must_use]
    pub fn param_types(&self) -> &[&'arena Type<'arena>] {
        &self.param_types
    }

    #[must_use]
    pub fn return_type(&self) -> &'arena Type<'arena> {
        self.return_type
    }

    #[must_use]
    pub fn param_names(&self) -> &[String] {
        &self.param_names
    }

    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }
}

// ============================================================================
// Error Types
// ============================================================================

#[derive(Debug)]
pub enum CompileError {
    SyntaxError(String),
    UndefinedVariable(String),
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SyntaxError(s) => write!(f, "Syntax error: {s}"),
            Self::UndefinedVariable(name) => write!(f, "Undefined variable: {name}"),
        }
    }
}

impl std::error::Error for CompileError {}

#[derive(Debug)]
pub enum ValidationError {
    ArgumentCountMismatch { expected: usize, got: usize },
    TypeMismatch { param_index: usize },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArgumentCountMismatch { expected, got } => {
                write!(
                    f,
                    "Argument count mismatch: expected {expected}, got {got}"
                )
            }
            Self::TypeMismatch { param_index } => {
                write!(f, "Type mismatch for parameter {param_index}")
            }
        }
    }
}

impl std::error::Error for ValidationError {}

impl From<melbi_core::values::from_raw::TypeError> for ValidationError {
    fn from(err: melbi_core::values::from_raw::TypeError) -> Self {
        match err {
            melbi_core::values::from_raw::TypeError::Mismatch => {
                Self::TypeMismatch { param_index: 0 }
            } // Adjust param_index as needed
            melbi_core::values::from_raw::TypeError::IndexOutOfBounds => {
                Self::ArgumentCountMismatch {
                    expected: 0,
                    got: 0,
                }
            } // Placeholder; adjust for context
        }
    }
}

// ============================================================================
// Static Typing API - Type-level Cons List
// ============================================================================

/// Type-level cons list for representing heterogeneous argument lists
pub struct Cons<Head, Tail>(PhantomData<(Head, Tail)>);

/// Trait for converting Rust types to/from Melbi types
pub trait MelbiType: Sized {
    fn melbi_type<'arena>(ty_mgr: &'arena TypeManager<'arena>) -> &'arena Type<'arena>;
    fn to_value<'arena, 'val>(self, ty_mgr: &'arena TypeManager<'arena>) -> Value<'arena, 'val>;
    fn from_value<'arena>(
        val: Value<'arena, '_>,
        ty_mgr: &'arena TypeManager<'arena>,
    ) -> Result<Self, ValidationError>;
}

impl MelbiType for i64 {
    fn melbi_type<'arena>(ty_mgr: &'arena TypeManager<'arena>) -> &'arena Type<'arena> {
        ty_mgr.int()
    }

    fn to_value<'arena, 'val>(self, ty_mgr: &'arena TypeManager<'arena>) -> Value<'arena, 'val> {
        Value::int(ty_mgr, self)
    }

    fn from_value<'arena>(
        val: Value<'arena, '_>,
        _ty_mgr: &'arena TypeManager<'arena>,
    ) -> Result<Self, ValidationError> {
        val.as_int().map_err(Into::into)
    }
}

// impl MelbiType for String {
//     fn melbi_type<'arena>(ty_mgr: &TypeManager<'arena>) -> &'arena Type<'arena> {
//         ty_mgr.str()
//     }

//     fn to_value<'arena, 'val>(
//         self,
//         arena: &'val Bump,
//         ty: &'arena Type<'arena>,
//     ) -> Value<'arena, 'val> {
//         Value::str(arena, ty, self)
//     }

//     fn from_value<'arena, 'val>(
//         val: Value<'arena, 'val>,
//         ty_mgr: &TypeManager<'arena>,
//     ) -> Result<Self, RuntimeError> {
//         val.get::<String>(ty_mgr).map_err(Into::into)
//     }
// }

/// Trait for handling argument lists (implemented for Cons chains)
pub trait MelbiArgs {
    type Values;

    fn arg_types<'arena>(ty_mgr: &'arena TypeManager<'arena>) -> Vec<&'arena Type<'arena>>;
    fn values_to_melbi<'arena, 'val>(
        values: Self::Values,
        ty_mgr: &'arena TypeManager<'arena>,
    ) -> Vec<Value<'arena, 'val>>;
}

// Base case: empty argument list
impl MelbiArgs for () {
    type Values = ();

    fn arg_types<'arena>(_ty_mgr: &'arena TypeManager<'arena>) -> Vec<&'arena Type<'arena>> {
        vec![]
    }

    fn values_to_melbi<'arena, 'val>(
        _values: (),
        _ty_mgr: &'arena TypeManager<'arena>,
    ) -> Vec<Value<'arena, 'val>> {
        vec![]
    }
}

// Recursive case: Cons<Head, Tail>
impl<H: MelbiType, T: MelbiArgs> MelbiArgs for Cons<H, T> {
    type Values = (H, T::Values);

    fn arg_types<'arena>(ty_mgr: &'arena TypeManager<'arena>) -> Vec<&'arena Type<'arena>> {
        let mut types = vec![H::melbi_type(ty_mgr)];
        types.extend(T::arg_types(ty_mgr));
        types
    }

    fn values_to_melbi<'arena, 'val>(
        values: (H, T::Values),
        ty_mgr: &'arena TypeManager<'arena>,
    ) -> Vec<Value<'arena, 'val>> {
        let (head, tail) = values;
        let mut result = vec![head.to_value(ty_mgr)];
        result.extend(T::values_to_melbi(tail, ty_mgr));
        result
    }
}

// ============================================================================
// Typed Expression
// ============================================================================

pub struct TypedExpression<'arena, Args, Ret> {
    inner: CompiledExpression<'arena>,
    _phantom: PhantomData<(Args, Ret)>,
}

impl<Args, Ret> TypedExpression<'_, Args, Ret>
where
    Args: MelbiArgs,
    Ret: MelbiType,
{
    pub fn eval(&self, arena: &Bump, args: Args::Values) -> Result<Ret, ValidationError> {
        let values = Args::values_to_melbi(args, self.inner.type_manager);

        // Run the expression (could skip validation in optimized version)
        let result = self.inner.run(arena, &values)?;

        // Convert result back to Rust type
        Ret::from_value(result, self.inner.type_manager)
    }
}

// ============================================================================
// Context: Static Typing API
// ============================================================================

impl<'arena> Context<'arena> {
    pub fn compile_typed<'ctx, Args, Ret>(
        &'arena self,
        source: &str,
        param_names: &[&str],
    ) -> Result<TypedExpression<'arena, Args, Ret>, CompileError>
    where
        'arena: 'ctx,
        Args: MelbiArgs,
        Ret: MelbiType,
    {
        let arg_types = Args::arg_types(self.type_manager());

        if arg_types.len() != param_names.len() {
            return Err(CompileError::SyntaxError(format!(
                "Parameter count mismatch: expected {}, got {}",
                arg_types.len(),
                param_names.len()
            )));
        }

        // Build params list
        let params: Vec<_> = param_names
            .iter()
            .zip(&arg_types)
            .map(|(&name, &ty)| (name, ty))
            .collect();

        // Compile using dynamic API
        let inner = self.compile(source, &params)?;

        // Validate return type
        let expected_ret = Ret::melbi_type(self.type_manager());
        if !std::ptr::eq(inner.return_type(), expected_ret) {
            return Err(CompileError::SyntaxError(
                "Return type mismatch".to_string(),
            ));
        }

        Ok(TypedExpression {
            inner,
            _phantom: PhantomData,
        })
    }
}

// ============================================================================
// Macro for Function Syntax
// ============================================================================

#[macro_export]
macro_rules! melbi_compile {
    ($ctx:expr, fn() -> $ret:ty) => {
        |source: &str, param_names: &[&str]| $ctx.compile_typed::<(), $ret>(source, param_names)
    };
    ($ctx:expr, fn($head:ty) -> $ret:ty) => {
        |source: &str, param_names: &[&str]| $ctx.compile_typed::<Cons<$head, ()>, $ret>(source, param_names)
    };
    ($ctx:expr, fn($head:ty, $($tail:ty),+) -> $ret:ty) => {
        |source: &str, param_names: &[&str]| $ctx.compile_typed::<Cons<$head, melbi_compile!(@cons_chain $($tail),+)>, $ret>(source, param_names)
    };

    // Helper: build the Cons chain
    (@cons_chain $head:ty) => {
        Cons<$head, ()>
    };
    (@cons_chain $head:ty, $($tail:ty),+) => {
        Cons<$head, melbi_compile!(@cons_chain $($tail),+)>
    };
}

#[macro_export]
macro_rules! melbi_eval {
    // No arguments case
    ($expr:expr, $arena:expr) => {
        $expr.melbi_eval($arena, ())
    };
    // With arguments
    ($expr:expr, $arena:expr, $($arg:expr),+ $(,)?) => {
        $expr.eval($arena, melbi_eval!(@nest $($arg),+))
    };

    // Helper: convert flat args to nested tuple structure
    (@nest $arg:expr) => {
        ($arg, ())
    };
    (@nest $arg:expr, $($rest:expr),+) => {
        ($arg, melbi_eval!(@nest $($rest),+))
    };
}

// ============================================================================
// Static Typing Demo
// ============================================================================

fn static_typing_works() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Testing Static Typing API ===");

    let context_arena = Bump::new();
    let context = Context::new(&context_arena);

    // Example 1: Two integer arguments
    let expr = melbi_compile![context, fn(i64, i64) -> i64]("a + b", &["a", "b"])?;

    let arena = Bump::new();
    let result: i64 = melbi_eval![expr, &arena, 40, 2]?;
    println!("Static typed result (40 + 2): {result}");
    assert_eq!(42, result);

    // Example 2: Three arguments
    let expr3 = melbi_compile![context, fn(i64, i64, i64) -> i64]("a + b", &["a", "b", "c"])?;
    let result3: i64 = melbi_eval![expr3, &arena, 10, 20, 30]?;
    println!("Static typed result (10 + 20 [ignoring c]): {result3}");
    assert_eq!(30, result3);

    println!("✓ Static typing API works!");

    Ok(())
}

// ============================================================================
// Main - Demo Usage
// ============================================================================

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let context_arena = Bump::new();
    let context = Context::new(&context_arena);

    // Get type manager and types
    let type_mgr = context.type_manager();
    let int_ty = type_mgr.int();

    // Compile expression
    let expr = context.compile("a + b", &[("a", int_ty), ("b", int_ty)])?;

    println!("\nExpression info:");
    println!("  Source: {}", expr.source());
    println!("  Params: {:?}", expr.param_names());
    println!("  Param types: {} types", expr.param_types().len());

    // Execute in an arena
    {
        let arena = Bump::new();
        let result = expr.run(&arena, &[Value::int(type_mgr, 40), Value::int(type_mgr, 2)])?;
        let value = result.as_int()?;
        println!("\nResult: {value}");
        assert_eq!(42, value);
    }

    // Execute again with different values (reusing the compiled expression)
    {
        let arena = Bump::new();
        let result = expr.run(
            &arena,
            &[Value::int(type_mgr, 100), Value::int(type_mgr, 23)],
        )?;
        let value = result.as_int()?;
        println!("Result: {value}");
        assert_eq!(123, value);
    }

    // Test error handling - wrong number of arguments
    {
        let arena = Bump::new();
        match expr.run(&arena, &[Value::int(type_mgr, 42)]) {
            Err(ValidationError::ArgumentCountMismatch { expected, got }) => {
                println!(
                    "\n✓ Correctly caught argument count mismatch: expected {expected}, got {got}"
                );
            }
            _ => panic!("Should have failed with argument count mismatch"),
        }
    }

    println!("\n✓ All tests passed!");

    // Add static typing tests
    static_typing_works()?;

    Ok(())
}
