//! Bytecode compiler implementation.

use alloc::boxed::Box;

use crate::{
    Vec,
    analyzer::typed_expr::{Expr, ExprBuilder, LambdaInstantiations, TypedExpr},
    parser::ComparisonOp,
    scope_stack::{CompleteScope, IncompleteScope, ScopeStack},
    types::{
        Type,
        manager::TypeManager,
        traits::{TypeKind, TypeView},
        unification::Unification,
    },
    values::dynamic::Value,
    visitor::TreeTransformer,
    vm::{
        ArrayContainsAdapter, CastAdapter, Code, FormatStrAdapter, FunctionAdapter, GenericAdapter,
        Instruction, LambdaCode, LambdaKind,
    },
};
use bumpalo::Bump;

use super::error::CompileError;

/// A pending jump that needs to be patched to the next match arm.
///
/// Stores the placeholder index and the instruction constructor to use when patching.
struct PatternJump {
    placeholder: usize,
    make_jump: fn(u8) -> Instruction,
}

/// Entry in the scope stack: local, capture, or global.
#[derive(Clone, Copy)]
enum ScopeEntry<'types, 'arena> {
    /// Local variable slot index
    Local(u32),
    /// Captured variable index (from enclosing lambda scope)
    Capture(u32),
    /// Global value (e.g., Math package) to add to constants
    Global(Value<'types, 'arena>),
}

/// Bytecode compiler that transforms typed expressions into VM bytecode.
///
/// The compiler implements the TreeTransformer pattern to traverse the AST
/// and emit bytecode instructions. It tracks the operand stack precisely
/// to set exact max_stack_size for debugging.
pub struct BytecodeCompiler<'types, 'arena> {
    /// Type manager for creating function adapters
    type_mgr: &'types TypeManager<'types>,

    /// Arena for allocations
    arena: &'arena Bump,

    /// Constant pool for literal values
    ///
    /// We store `Value` (not `RawValue`) to preserve type information for debugging.
    /// At runtime, the VM will extract the RawValue when loading constants.
    ///
    /// Future: In release mode, we could strip types and store only RawValue.
    constants: alloc::vec::Vec<Value<'types, 'arena>>,

    /// Constant deduplication map: Value -> index
    ///
    /// Maps values to their index in the constants pool to avoid duplicates.
    constant_map: hashbrown::HashMap<Value<'types, 'arena>, usize>,

    /// Bytecode instructions
    instructions: alloc::vec::Vec<Instruction>,

    /// Number of local variables
    num_locals: usize,

    /// Scope stack for lexical scoping
    ///
    /// Uses ScopeStack from scope_stack.rs which handles:
    /// - Globals (Math, String packages, etc.) at the bottom
    /// - Expression params (future) in the middle
    /// - Where bindings (pushed/popped dynamically) at the top
    scope_stack: ScopeStack<'arena, ScopeEntry<'types, 'arena>>,

    /// Function adapters for FFI calls
    ///
    /// Each adapter stores parameter types for a call site.
    /// TODO: Deduplicate adapters with same parameter types.
    adapters: alloc::vec::Vec<FunctionAdapter<'types>>,

    /// Generic adapters for other operations (Cast, FormatStr, etc.)
    ///
    /// These use dynamic dispatch to allow different adapter types.
    generic_adapters: alloc::vec::Vec<Box<dyn GenericAdapter + 'types>>,

    /// Current stack depth during compilation
    current_stack_depth: usize,

    /// Maximum stack depth observed (exact tracking for debugging)
    max_stack_size: usize,

    /// Nested lambda bytecode
    lambdas: alloc::vec::Vec<LambdaCode<'types>>,

    /// Lambda instantiation info from the analyzer.
    /// Maps lambda expression pointers to their type instantiations.
    /// Used to compile polymorphic lambdas with multiple specializations.
    lambda_instantiations: Option<
        &'arena hashbrown::HashMap<
            *const Expr<'types, 'arena>,
            LambdaInstantiations<'types, 'arena>,
            hashbrown::DefaultHashBuilder,
            &'arena Bump,
        >,
    >,

    /// Type unification for the current lambda instantiation being compiled.
    /// Used to resolve type variables to concrete types.
    /// None for top-level code and monomorphic lambdas.
    monomorphism: Option<Unification<'types, &'types TypeManager<'types>>>,
}

impl<'types, 'arena> BytecodeCompiler<'types, 'arena> {
    /// Create a new bytecode compiler.
    ///
    /// # Arguments
    /// * `type_mgr` - Type manager for creating function adapters
    /// * `arena` - Arena for allocations
    /// * `globals` - Global values (e.g., Math package) sorted by name
    /// * `lambda_instantiations` - Optional map of lambda instantiations for polymorphic lambdas
    pub fn new(
        type_mgr: &'types TypeManager<'types>,
        arena: &'arena Bump,
        globals: &'arena [(&'arena str, Value<'types, 'arena>)],
        lambda_instantiations: Option<
            &'arena hashbrown::HashMap<
                *const Expr<'types, 'arena>,
                LambdaInstantiations<'types, 'arena>,
                hashbrown::DefaultHashBuilder,
                &'arena Bump,
            >,
        >,
    ) -> Self {
        // Convert globals slice to ScopeEntry format
        let globals_entries: &'arena [(&'arena str, ScopeEntry<'types, 'arena>)] = arena
            .alloc_slice_fill_iter(
                globals
                    .iter()
                    .map(|(name, value)| (*name, ScopeEntry::Global(*value))),
            );

        // Initialize scope stack with globals at the bottom
        let mut scope_stack = ScopeStack::new();
        scope_stack.push(CompleteScope::from_sorted(globals_entries));

        Self {
            type_mgr,
            arena,
            constants: alloc::vec::Vec::new(),
            constant_map: hashbrown::HashMap::new(),
            instructions: alloc::vec::Vec::new(),
            num_locals: 0,
            scope_stack,
            adapters: alloc::vec::Vec::new(),
            generic_adapters: alloc::vec::Vec::new(),
            current_stack_depth: 0,
            max_stack_size: 0,
            lambdas: alloc::vec::Vec::new(),
            lambda_instantiations,
            monomorphism: None,
        }
    }

    /// Create a new bytecode compiler for compiling a lambda body.
    ///
    /// Unlike `new`, this constructor:
    /// - Has no globals (lambdas access outer scope via captures)
    /// - Sets up captures as a scope layer for resolving captured variables
    ///
    /// # Arguments
    /// * `type_mgr` - Type manager for creating function adapters
    /// * `arena` - Arena for allocations
    /// * `captures` - Names of captured variables (in order)
    /// * `monomorphism` - Optional type unification for polymorphic lambda instantiations
    fn new_for_lambda(
        type_mgr: &'types TypeManager<'types>,
        arena: &'arena Bump,
        captures: &[&'arena str],
        monomorphism: Option<Unification<'types, &'types TypeManager<'types>>>,
    ) -> Self {
        // Build captures scope: name -> Capture(index)
        let captures_entries: &[(&str, ScopeEntry)] = arena.alloc_slice_fill_iter(
            captures
                .iter()
                .enumerate()
                .map(|(i, &name)| (name, ScopeEntry::Capture(i as u32))),
        );

        // Initialize scope stack with captures at the bottom
        let mut scope_stack = ScopeStack::new();
        scope_stack.push(CompleteScope::from_sorted(captures_entries));

        Self {
            type_mgr,
            arena,
            constants: alloc::vec::Vec::new(),
            constant_map: hashbrown::HashMap::new(),
            instructions: alloc::vec::Vec::new(),
            num_locals: 0,
            scope_stack,
            adapters: alloc::vec::Vec::new(),
            generic_adapters: alloc::vec::Vec::new(),
            current_stack_depth: 0,
            max_stack_size: 0,
            lambdas: alloc::vec::Vec::new(),
            lambda_instantiations: None, // Lambda compilers don't need instantiation info
            monomorphism,
        }
    }

    /// Finalize compilation and return the bytecode.
    ///
    /// Converts Value constants (with type info) to RawValue for VM execution.
    pub fn finalize(self) -> Code<'types> {
        // Convert Values to RawValues for VM
        // TODO: In debug mode, we could keep Values for better error messages
        let raw_constants = self
            .constants
            .into_iter()
            .map(|value| value.as_raw())
            .collect();

        Code {
            constants: raw_constants,
            adapters: self.adapters,
            generic_adapters: self.generic_adapters,
            instructions: self.instructions,
            num_locals: self.num_locals,
            max_stack_size: self.max_stack_size,
            lambdas: self.lambdas,
        }
    }

    /// Convenience method to compile an expression in one call.
    ///
    /// # Arguments
    /// * `type_mgr` - Type manager for creating function adapters
    /// * `arena` - Arena for allocations
    /// * `globals` - Global values (e.g., Math package) sorted by name
    /// * `typed_expr` - The typed expression (with lambda instantiation info) to compile
    pub fn compile(
        type_mgr: &'types TypeManager<'types>,
        arena: &'arena Bump,
        globals: &'arena [(&'arena str, Value<'types, 'arena>)],
        typed_expr: &'arena TypedExpr<'types, 'arena>,
    ) -> Result<Code<'types>, CompileError> {
        let lambda_instantiations = if typed_expr.lambda_instantiations.is_empty() {
            None
        } else {
            Some(&typed_expr.lambda_instantiations)
        };
        let mut compiler = Self::new(type_mgr, arena, globals, lambda_instantiations);
        compiler.transform(typed_expr.expr)?;
        debug_assert_eq!(compiler.current_stack_depth, 1);
        // Emit Return instruction to signal end of execution
        compiler.emit(Instruction::Return);
        Ok(compiler.finalize())
    }

    // === Stack Management ===

    /// Push a value onto the stack (increases depth by 1).
    fn push_stack(&mut self) {
        self.current_stack_depth += 1;
        if self.current_stack_depth > self.max_stack_size {
            self.max_stack_size = self.current_stack_depth;
        }
    }

    /// Pop a value from the stack (decreases depth by 1).
    fn pop_stack(&mut self) {
        debug_assert!(self.current_stack_depth > 0, "Stack underflow");
        self.current_stack_depth -= 1;
    }

    /// Pop N values from the stack.
    fn pop_stack_n(&mut self, n: usize) {
        debug_assert!(
            self.current_stack_depth >= n,
            "Stack underflow: trying to pop {} but depth is {}",
            n,
            self.current_stack_depth
        );
        self.current_stack_depth -= n;
    }

    // === Type Resolution ===

    /// Resolve a type through the current monomorphism.
    ///
    /// If we're compiling a polymorphic lambda instantiation, this applies the
    /// substitution to resolve type variables to concrete types.
    /// For top-level code and monomorphic lambdas, returns the type unchanged.
    fn resolve_type(&self, ty: &'types Type<'types>) -> &'types Type<'types> {
        self.monomorphism
            .as_ref()
            .map(|m| m.fully_resolve(ty))
            .unwrap_or(ty)
    }

    // === Instruction Emission ===

    /// Emit an instruction without an argument.
    fn emit(&mut self, instruction: Instruction) {
        self.instructions.push(instruction);
    }

    /// Emit an instruction with a u32 argument, handling WideArg automatically.
    ///
    /// This is the non-generic implementation to avoid code bloat from monomorphization.
    /// For arg 0x00_12_34_56:
    ///   - 0x56 goes in the instruction itself (passed in `instruction`)
    ///   - 0x00 is not emitted (leading zero)
    ///   - Emit WideArg(0x12), WideArg(0x34) before the instruction
    fn emit_with_arg_impl(&mut self, instruction: Instruction, mut remaining: u32) {
        // Max 3 WideArgs for u32
        let mut wide_bytes = alloc::vec::Vec::with_capacity(3);
        while remaining > 0 {
            wide_bytes.push((remaining & 0xFF) as u8);
            remaining >>= 8;
        }
        // Emit in reverse (most significant byte first)
        for &byte in wide_bytes.iter().rev() {
            self.instructions.push(Instruction::WideArg(byte));
        }
        self.instructions.push(instruction);
    }

    /// Emit an instruction with a u32 argument.
    ///
    /// The generic wrapper constructs the instruction with the low byte,
    /// then delegates to emit_with_arg_impl for WideArg handling.
    fn emit_with_arg(&mut self, make_instr: fn(u8) -> Instruction, arg: u32) {
        self.emit_with_arg_impl(make_instr((arg & 0xFF) as u8), arg >> 8);
    }

    // === Local Variable Management ===

    /// Allocate a new local variable slot.
    ///
    /// Always creates a new slot (does not add to scope - that's done separately).
    /// This enables proper variable shadowing.
    fn allocate_local(&mut self) -> Result<u32, CompileError> {
        let index = self.num_locals;
        self.num_locals += 1;
        index.try_into().map_err(|_| CompileError::TooManyLocals)
    }

    /// Compile loading a variable by name.
    ///
    /// Lookup order is determined by scope stack: locals -> captures -> globals.
    /// Emits the appropriate load instruction.
    fn compile_variable_load(&mut self, name: &'arena str) -> Result<(), CompileError> {
        match self.scope_stack.lookup(name) {
            Some(ScopeEntry::Local(index)) => {
                self.emit_with_arg(Instruction::LoadLocal, *index);
            }
            Some(ScopeEntry::Capture(index)) => {
                self.emit_with_arg(Instruction::LoadCapture, *index as u32);
            }
            Some(ScopeEntry::Global(value)) => {
                let const_index = self.add_constant(*value)?;
                self.emit_with_arg(Instruction::ConstLoad, const_index);
            }
            None => {
                panic!(
                    "Undefined variable '{}' (should be caught by type checker)",
                    name
                );
            }
        }
        self.push_stack();
        Ok(())
    }

    /// Compile a lambda body into a LambdaCode with Mono kind.
    ///
    /// Creates a fresh compiler for the lambda, sets up parameters as locals,
    /// compiles the body, and returns the resulting LambdaCode.
    ///
    /// # Arguments
    /// * `params` - Parameter names
    /// * `body` - Lambda body expression
    /// * `captures` - Names of captured variables
    /// * `lambda_type` - The concrete type of this lambda instantiation
    /// * `monomorphism` - Optional type unification for polymorphic lambdas
    fn compile_lambda_body(
        &self,
        params: &[&'arena str],
        body: &Expr<'types, 'arena>,
        captures: &[&'arena str],
        lambda_type: &'types Type<'types>,
        monomorphism: Option<Unification<'types, &'types TypeManager<'types>>>,
    ) -> Result<LambdaCode<'types>, CompileError> {
        // Create fresh compiler for lambda
        let mut lambda_compiler =
            BytecodeCompiler::new_for_lambda(self.type_mgr, self.arena, captures, monomorphism);

        // Set up parameters as locals (in order)
        // Parameters are passed by the caller via VM locals
        lambda_compiler.scope_stack.push(
            IncompleteScope::new(self.arena, params)
                .expect("Duplicate parameter names (should be caught by type checker)"),
        );

        for &param in params {
            let local_idx = lambda_compiler.allocate_local()?;
            lambda_compiler
                .scope_stack
                .bind_in_current(param, ScopeEntry::Local(local_idx))
                .expect("Parameter binding");
        }

        // Compile body
        lambda_compiler.transform(body)?;

        // Emit Return
        lambda_compiler.emit(Instruction::Return);

        // Return the compiled LambdaCode
        let num_captures = captures.len();

        let code = Code {
            constants: lambda_compiler
                .constants
                .into_iter()
                .map(|v| v.as_raw())
                .collect(),
            adapters: lambda_compiler.adapters,
            generic_adapters: lambda_compiler.generic_adapters,
            instructions: lambda_compiler.instructions,
            num_locals: lambda_compiler.num_locals,
            max_stack_size: lambda_compiler.max_stack_size,
            lambdas: lambda_compiler.lambdas,
        };

        Ok(LambdaCode {
            lambda_type,
            num_captures: num_captures as u32,
            kind: LambdaKind::Mono { code },
        })
    }

    // === Constant Pool Management ===

    /// Add a constant to the pool (or reuse existing) and return its index.
    ///
    /// Deduplicates constants by value equality.
    /// Returns the index as u32 - emit_with_arg handles WideArg if needed.
    fn add_constant(&mut self, value: Value<'types, 'arena>) -> Result<u32, CompileError> {
        // Check if this constant already exists
        if let Some(&existing_index) = self.constant_map.get(&value) {
            return Ok(existing_index as u32);
        }

        // Add new constant
        let index = self.constants.len();
        self.constants.push(value);
        self.constant_map.insert(value, index);
        index.try_into().map_err(|_| CompileError::TooManyConstants)
    }

    // === Jump Patching Infrastructure ===

    /// Reserve space for a jump instruction and return its index.
    ///
    /// The jump target will be patched later with `patch_jump`.
    /// We reserve 2 instructions to support 64K jump range.
    fn jump_placeholder(&mut self, make_jump: fn(u8) -> Instruction) -> usize {
        let placeholder_index = self.instructions.len();
        // Reserve space - we'll patch these instructions later
        self.emit(make_jump(0));
        self.emit(Instruction::Nop);
        placeholder_index
    }

    /// Get the current instruction index (for use as a jump label).
    fn label(&self) -> usize {
        self.instructions.len()
    }

    /// Patch a jump placeholder with the actual jump instruction.
    ///
    /// # Arguments
    /// * `placeholder_index` - The index returned by `jump_placeholder()`
    /// * `target_label` - The target instruction index from `label()`
    /// * `make_jump` - Function that creates the jump instruction with the offset
    ///
    /// Uses 1 instruction for offsets <= 255, or 2 instructions (WideArg + Jump) for larger offsets.
    fn patch_jump(
        &mut self,
        placeholder_index: usize,
        target_label: usize,
        make_jump: fn(u8) -> Instruction,
    ) -> Result<(), CompileError> {
        // Calculate the offset from the jump instruction to the target
        // The VM loop automatically increments the instruction pointer after each instruction,
        // so: offset = target - current - 1
        debug_assert!(target_label >= placeholder_index);
        let offset = target_label - placeholder_index - 1;
        debug_assert_eq!(self.instructions[placeholder_index], make_jump(0));

        if offset <= u8::MAX as usize {
            // Single instruction: Jump(offset)
            self.instructions[placeholder_index] = make_jump(offset as u8);
            // Second slot stays Nop (already there from placeholder)
        } else {
            // Two instructions: WideArg(high_byte), Jump(low_byte)
            // The jump happens from placeholder_index + 1, so subtract 1 from offset
            let adjusted_offset: u16 = (offset - 1)
                .try_into()
                .map_err(|_| CompileError::JumpTooFar)?;
            self.instructions[placeholder_index] =
                Instruction::WideArg((adjusted_offset >> 8) as u8);
            self.instructions[placeholder_index + 1] = make_jump((adjusted_offset & 0xFF) as u8);
        }
        Ok(())
    }

    /// Compile a pattern check.
    ///
    /// The pattern consumes the value on top of the stack. If the pattern matches,
    /// execution falls through (possibly with local variable bindings set up).
    /// If the pattern fails, it jumps to the next arm.
    ///
    /// # Arguments
    /// * `pattern` - The pattern to compile
    /// * `value_type` - The type of the value being matched
    ///
    /// # Returns
    /// A vector of `PatternJump` containing placeholder indices and their jump instruction types.
    /// Variable bindings are stored directly in the current scope (caller must set up scope first).
    fn compile_pattern(
        &mut self,
        pattern: &'arena crate::analyzer::typed_expr::TypedPattern<'types, 'arena>,
        value_type: &'types crate::types::Type<'types>,
    ) -> Result<alloc::vec::Vec<PatternJump>, CompileError> {
        use crate::analyzer::typed_expr::TypedPattern;
        use crate::types::traits::{TypeKind, TypeView};

        let mut fail_jumps = alloc::vec::Vec::new();

        match pattern {
            TypedPattern::Wildcard => {
                // Wildcard: always matches, discard the value
                self.emit(Instruction::Pop);
                self.pop_stack();
            }

            TypedPattern::Var(name) => {
                // Variable: always matches, bind to local variable
                let index = self.allocate_local()?;
                self.emit_with_arg(Instruction::StoreLocal, index);
                self.pop_stack();

                // Bind in the current scope (caller must have pushed a scope)
                self.scope_stack
                    .bind_in_current(name, ScopeEntry::Local(index))
                    .expect("Pattern binding");
            }

            TypedPattern::Literal(value) => {
                // Literal: load constant, compare, jump if not equal
                // At entry: stack has the value to match against (depth = N)

                let const_index = self.add_constant(*value)?;
                self.emit_with_arg(Instruction::ConstLoad, const_index);
                self.push_stack(); // depth = N + 1

                // Emit type-specific comparison (consumes both matched value and constant)
                self.pop_stack_n(2); // depth = N - 1 (value consumed)
                match value_type.view() {
                    TypeKind::Int => {
                        self.emit(Instruction::IntCmpOp(crate::parser::ComparisonOp::Eq))
                    }
                    TypeKind::Float => {
                        self.emit(Instruction::FloatCmpOp(crate::parser::ComparisonOp::Eq))
                    }
                    TypeKind::Str => {
                        self.emit(Instruction::StringCmpOp(crate::parser::ComparisonOp::Eq))
                    }
                    TypeKind::Bytes => {
                        self.emit(Instruction::BytesCmpOp(crate::parser::ComparisonOp::Eq))
                    }
                    TypeKind::Bool => self.emit(Instruction::EqBool),
                    _ => panic!("Literal pattern on unsupported type (type checker bug)"),
                }
                self.push_stack(); // Comparison result (bool)

                // Reserve placeholder for PopJumpIfFalse - will jump to next arm if not equal
                let placeholder = self.jump_placeholder(Instruction::PopJumpIfFalse);
                fail_jumps.push(PatternJump {
                    placeholder,
                    make_jump: Instruction::PopJumpIfFalse,
                });
                self.pop_stack(); // PopJumpIfFalse consumes the bool
            }

            TypedPattern::Some(inner_pattern) => {
                // Some pattern: use MatchSomeOrJump
                // - If Some: extracts inner value and falls through
                // - If None: jumps forward (to next arm)

                // Reserve placeholder for MatchSomeOrJump
                let placeholder = self.jump_placeholder(Instruction::MatchSomeOrJump);
                fail_jumps.push(PatternJump {
                    placeholder,
                    make_jump: Instruction::MatchSomeOrJump,
                });

                // Stack effect: option consumed, inner value pushed (if Some)
                self.pop_stack();
                self.push_stack();

                // Get the inner type for recursive pattern matching
                let inner_type = match value_type.view() {
                    TypeKind::Option(inner) => inner,
                    _ => panic!(
                        "Some pattern on non-Option type (type checker bug): value_type = {:?}",
                        value_type
                    ),
                };

                // Recursively compile the inner pattern
                let inner_fail_jumps = self.compile_pattern(inner_pattern, inner_type)?;
                fail_jumps.extend(inner_fail_jumps);
            }

            TypedPattern::None => {
                // None pattern: use MatchNoneOrJump
                // - If None: falls through (value consumed)
                // - If Some: jumps forward

                // Reserve placeholder for MatchNoneOrJump
                let placeholder = self.jump_placeholder(Instruction::MatchNoneOrJump);
                fail_jumps.push(PatternJump {
                    placeholder,
                    make_jump: Instruction::MatchNoneOrJump,
                });

                // Stack effect: option consumed
                self.pop_stack();
            }
        }

        Ok(fail_jumps)
    }
}

impl<'types, 'arena> TreeTransformer<ExprBuilder<'types, 'arena>>
    for BytecodeCompiler<'types, 'arena>
where
    'types: 'arena,
{
    type Output = Result<(), CompileError>;

    fn transform(&mut self, tree: &'arena Expr<'types, 'arena>) -> Self::Output {
        use crate::{
            analyzer::typed_expr::ExprInner,
            parser::{BinaryOp, BoolOp},
            visitor::TreeView,
        };

        match tree.view() {
            // === Constants ===
            ExprInner::Constant(value) => {
                if let Ok(i) = value.as_int() {
                    // Use immediate encoding for small integers
                    if i >= i8::MIN as i64 && i <= i8::MAX as i64 {
                        self.emit(Instruction::ConstInt(i as i8));
                        self.push_stack();
                    } else if i >= 0 && i <= u8::MAX as i64 {
                        self.emit(Instruction::ConstUInt(i as u8));
                        self.push_stack();
                    } else {
                        // Large integer - use constant pool
                        let const_index = self.add_constant(value)?;
                        self.emit_with_arg(Instruction::ConstLoad, const_index);
                        self.push_stack();
                    }
                } else if let Ok(b) = value.as_bool() {
                    // Use immediate encoding for booleans
                    if b {
                        self.emit(Instruction::ConstBool(1));
                    } else {
                        self.emit(Instruction::ConstBool(0));
                    }
                    self.push_stack();
                } else {
                    // Other types (float, string, etc.) - use constant pool
                    let const_index = self.add_constant(value)?;
                    self.emit_with_arg(Instruction::ConstLoad, const_index);
                    self.push_stack();
                }
            }

            // === Binary Operations ===
            ExprInner::Binary { op, left, right } => {
                // Compile left operand
                self.transform(left)?;

                // Compile right operand
                self.transform(right)?;

                // Emit operation instruction (pops 2, pushes 1)
                self.pop_stack_n(2);
                let op_byte = match op {
                    BinaryOp::Add => b'+',
                    BinaryOp::Sub => b'-',
                    BinaryOp::Mul => b'*',
                    BinaryOp::Div => b'/',
                    BinaryOp::Pow => b'^',
                };

                // Check if this is a float or int operation based on the result type
                // Use resolve_type to handle polymorphic lambdas
                let resolved_type = self.resolve_type(tree.0);
                match resolved_type.view() {
                    TypeKind::Float => self.emit(Instruction::FloatBinOp(op_byte)),
                    TypeKind::Int => self.emit(Instruction::IntBinOp(op_byte)),
                    _ => panic!(
                        "Binary operation on non-numeric type: {} (type checker bug)",
                        resolved_type
                    ),
                }
                self.push_stack();
            }

            // === Unary Operations ===
            ExprInner::Unary { op, expr } => {
                use crate::parser::UnaryOp;

                // Compile operand
                self.transform(expr)?;

                // Emit operation (pops 1, pushes 1)
                self.pop_stack();
                match op {
                    UnaryOp::Neg => {
                        // Check if this is float or int negation based on operand type
                        // Use resolve_type to handle polymorphic lambdas
                        let resolved_type = self.resolve_type(expr.0);
                        match resolved_type.view() {
                            TypeKind::Float => self.emit(Instruction::NegFloat),
                            TypeKind::Int => self.emit(Instruction::NegInt),
                            _ => panic!(
                                "Negation on non-numeric type: {} (type checker bug)",
                                resolved_type
                            ),
                        }
                    }
                    UnaryOp::Not => {
                        self.emit(Instruction::Not);
                    }
                }
                self.push_stack();
            }

            // === Comparison Operations ===
            ExprInner::Comparison { op, left, right } => {
                // Compile left operand
                self.transform(left)?;

                // Compile right operand
                self.transform(right)?;

                // Emit comparison instruction (pops 2, pushes 1)
                self.pop_stack_n(2);

                // For containment operations (In/NotIn), dispatch based on the right operand
                // type (the haystack). For other comparisons, use the left operand type.
                if matches!(op, ComparisonOp::In | ComparisonOp::NotIn) {
                    let haystack_type = self.resolve_type(right.0);
                    match haystack_type.view() {
                        TypeKind::Str => self.emit(Instruction::StringCmpOp(op)),
                        TypeKind::Bytes => self.emit(Instruction::BytesCmpOp(op)),
                        TypeKind::Array(element_type) => {
                            // Use adapter for dynamic element comparison
                            let adapter = ArrayContainsAdapter::new(element_type, op);
                            let adapter_index = self.generic_adapters.len();
                            self.generic_adapters.push(Box::new(adapter));
                            self.emit_with_arg(
                                Instruction::CallGenericAdapter,
                                adapter_index as u32,
                            );
                        }
                        TypeKind::Map(_, _) => {
                            self.emit(Instruction::MapHas);
                            if op == ComparisonOp::NotIn {
                                self.emit(Instruction::Not);
                            }
                        }
                        _ => panic!(
                            "Containment on unsupported type: {} (type checker bug)",
                            haystack_type
                        ),
                    }
                } else {
                    let resolved_type = self.resolve_type(left.0);
                    match resolved_type.view() {
                        TypeKind::Float => self.emit(Instruction::FloatCmpOp(op)),
                        TypeKind::Int => self.emit(Instruction::IntCmpOp(op)),
                        TypeKind::Str => self.emit(Instruction::StringCmpOp(op)),
                        TypeKind::Bytes => self.emit(Instruction::BytesCmpOp(op)),
                        _ => panic!(
                            "Comparison on unsupported type: {} (type checker bug)",
                            resolved_type
                        ),
                    }
                }
                self.push_stack();
            }

            // === Boolean Operations (Short-Circuit Evaluation) ===
            ExprInner::Boolean { op, left, right } => {
                // Short-circuit evaluation:
                // - `left and right`: if left is false, result is false (don't eval right)
                // - `left or right`: if left is true, result is true (don't eval right)
                //
                // Bytecode pattern for AND:
                //   compile(left)
                //   PopJumpIfFalse(to_push_false)  -- if false, skip right
                //   compile(right)                 -- right's value is result
                //   JumpForward(to_end)
                //   to_push_false: ConstBool(0)   -- push false
                //   to_end:
                //
                // Bytecode pattern for OR:
                //   compile(left)
                //   PopJumpIfTrue(to_push_true)   -- if true, skip right
                //   compile(right)                -- right's value is result
                //   JumpForward(to_end)
                //   to_push_true: ConstBool(1)    -- push true
                //   to_end:

                // Compile left operand
                self.transform(left)?;
                self.pop_stack(); // Consumed by PopJumpIf*

                // Reserve space for short-circuit jump
                let short_circuit_jump = match op {
                    BoolOp::And => self.jump_placeholder(Instruction::PopJumpIfFalse),
                    BoolOp::Or => self.jump_placeholder(Instruction::PopJumpIfTrue),
                };

                // Compile right operand (only executed if left doesn't short-circuit)
                self.transform(right)?;
                // Right leaves one result on stack (stack depth is now correct)

                // Jump over the constant push
                let end_jump = self.jump_placeholder(Instruction::JumpForward);

                // Label for short-circuit case
                let short_circuit_label = self.label();
                match op {
                    BoolOp::And => {
                        self.patch_jump(
                            short_circuit_jump,
                            short_circuit_label,
                            Instruction::PopJumpIfFalse,
                        )?;
                        self.emit(Instruction::ConstBool(0)); // Push false
                    }
                    BoolOp::Or => {
                        self.patch_jump(
                            short_circuit_jump,
                            short_circuit_label,
                            Instruction::PopJumpIfTrue,
                        )?;
                        self.emit(Instruction::ConstBool(1)); // Push true
                    }
                }

                // Patch end jump
                let end_label = self.label();
                self.patch_jump(end_jump, end_label, Instruction::JumpForward)?;

                // Stack depth already correct from transform(right)
            }

            // === If Expressions ===
            ExprInner::If {
                cond,
                then_branch,
                else_branch,
            } => {
                // Compile condition
                self.transform(cond)?;
                self.pop_stack(); // Condition consumed by PopJumpIfFalse

                // Reserve space for jump to else branch
                let else_jump = self.jump_placeholder(Instruction::PopJumpIfFalse);

                // Compile then branch (leaves one result on stack)
                self.transform(then_branch)?;

                // Reserve space for jump over else branch
                let end_jump = self.jump_placeholder(Instruction::JumpForward);

                // Patch the else jump to point here
                let else_label = self.label();
                self.patch_jump(else_jump, else_label, Instruction::PopJumpIfFalse)?;

                // Pop the then result for stack tracking (else branch runs instead at runtime)
                self.pop_stack();

                // Compile else branch (leaves one result on stack)
                self.transform(else_branch)?;

                // Patch the end jump to point here
                let end_label = self.label();
                self.patch_jump(end_jump, end_label, Instruction::JumpForward)?;

                // Stack depth is correct: else_branch pushed one result
            }

            // === Array Construction ===
            ExprInner::Array { elements } => {
                // Compile all element expressions
                // They will be pushed onto the stack in order
                for element in elements.iter() {
                    self.transform(element)?;
                }

                // MakeArray pops N elements and pushes 1 array
                let count = elements.len();
                self.pop_stack_n(count);

                // Emit MakeArray instruction
                self.emit_with_arg(Instruction::MakeArray, count as u32);
                self.push_stack();
            }

            // === Variable Access ===
            ExprInner::Ident(name) => {
                self.compile_variable_load(name)?;
            }

            // === Where Bindings ===
            ExprInner::Where { expr, bindings } => {
                // Collect binding names for the incomplete scope
                let names: alloc::vec::Vec<_> = bindings.iter().map(|(name, _)| *name).collect();

                // Push an incomplete scope for the bindings
                self.scope_stack.push(
                    IncompleteScope::new(self.arena, &names)
                        .expect("Duplicate binding names (should be caught by type checker)"),
                );

                // Compile all bindings first (in order)
                for (name, value_expr) in bindings.iter() {
                    // Compile the value expression
                    self.transform(value_expr)?;
                    self.pop_stack();

                    // Allocate a new local slot
                    let index = self.allocate_local()?;
                    self.emit_with_arg(Instruction::StoreLocal, index);

                    // Bind the name to the local slot in the current scope
                    self.scope_stack
                        .bind_in_current(name, ScopeEntry::Local(index))
                        .expect("Failed to bind variable (should not happen)");
                }

                // Then compile the main expression (which can reference the bindings)
                self.transform(expr)?;
                // Result is left on stack

                // Pop the scope when done
                self.scope_stack.pop().expect("Scope stack underflow");
            }

            // === Index Operations ===
            ExprInner::Index { value, index } => {
                use crate::types::traits::TypeKind;

                // Compile the value expression (array, map, or bytes)
                self.transform(value)?;

                // Resolve the container type (applies substitution for polymorphic lambdas)
                let container_type = self.resolve_type(value.0);

                // Check if index is a constant for optimization
                if let ExprInner::Constant(idx_val) = index.view() {
                    if let Ok(i) = idx_val.as_int() {
                        if 0 <= i && i <= i8::MAX as i64 {
                            // Use constant index optimization for arrays and bytes
                            match container_type.view() {
                                TypeKind::Array(_) => {
                                    self.pop_stack(); // Pop array
                                    self.emit_with_arg(Instruction::ArrayGetConst, i as u32);
                                    self.push_stack(); // Push result
                                    return Ok(());
                                }
                                TypeKind::Bytes => {
                                    self.pop_stack(); // Pop bytes
                                    self.emit(Instruction::BytesGetConst(i as u8));
                                    self.push_stack(); // Push result (Int)
                                    return Ok(());
                                }
                                _ => {} // Fall through to generic case (including maps)
                            }
                        }
                    }
                }

                // Dynamic index: compile index expression
                self.transform(index)?;

                // Emit appropriate get instruction based on value type
                self.pop_stack_n(2); // Pop index and container
                match container_type.view() {
                    TypeKind::Array(_) => {
                        self.emit(Instruction::ArrayGet);
                    }
                    TypeKind::Map(_, _) => {
                        self.emit(Instruction::MapGet);
                    }
                    TypeKind::Bytes => {
                        self.emit(Instruction::BytesGet);
                    }
                    _ => panic!(
                        "Index operation on non-indexable type (type checker bug): {}",
                        container_type
                    ),
                }
                self.push_stack(); // Push result
            }

            // === Field Access ===
            ExprInner::Field { value, field } => {
                use crate::types::traits::TypeKind;

                // Compile the record expression
                self.transform(value)?;

                // Resolve the record type (applies substitution for polymorphic lambdas)
                let record_type = self.resolve_type(value.0);

                // Look up field index in the record type
                let field_index = match record_type.view() {
                    TypeKind::Record(fields) => {
                        // Fields are sorted by name, find the index
                        let mut idx = None;
                        for (i, (name, _ty)) in fields.enumerate() {
                            if name == field {
                                idx = Some(i);
                                break;
                            }
                        }
                        idx.expect(
                            "Field not found in record type (should be caught by type checker)",
                        )
                    }
                    _ => panic!("Field access on non-record type (type checker bug)"),
                };

                // Emit RecordGet instruction
                self.pop_stack(); // Pop record
                self.emit_with_arg(Instruction::RecordGet, field_index as u32);
                self.push_stack(); // Push field value
            }

            // === Record Construction ===
            ExprInner::Record { fields } => {
                // Sort fields by name to match the type's field order
                // (TypeManager::record sorts fields alphabetically)
                let mut sorted_fields: Vec<_> = fields.iter().collect();
                sorted_fields.sort_by_key(|(name, _)| *name);

                // Compile field values in sorted order
                for (_name, value_expr) in sorted_fields.iter() {
                    self.transform(value_expr)?;
                }

                // MakeRecord pops N values and pushes 1 record
                let count = fields.len();
                self.pop_stack_n(count);

                // Emit MakeRecord instruction
                self.emit_with_arg(Instruction::MakeRecord, count as u32);
                self.push_stack();
            }

            // === Map Construction ===
            ExprInner::Map { elements } => {
                // Compile all key-value pairs
                // Each pair pushes key then value onto the stack
                for (key_expr, value_expr) in elements.iter() {
                    self.transform(key_expr)?; // Push key
                    self.transform(value_expr)?; // Push value
                }

                // MakeMap pops 2*N values (N key-value pairs) and pushes 1 map
                let num_pairs = elements.len();
                self.pop_stack_n(num_pairs * 2);

                // Emit MakeMap instruction
                self.emit_with_arg(Instruction::MakeMap, num_pairs as u32);
                self.push_stack();
            }

            ExprInner::Otherwise { primary, fallback } => {
                // Reserve placeholder for PushOtherwise (jump to fallback on error)
                let push_placeholder = self.jump_placeholder(Instruction::PushOtherwise);

                // Remember stack depth before primary
                let depth_before = self.current_stack_depth;

                // Compile primary expression (may error)
                self.transform(primary)?;

                // Primary succeeded: stack now has one more value
                self.pop_stack();
                assert_eq!(self.current_stack_depth, depth_before);

                // Reserve placeholder for PopOtherwiseAndJump (jump to done on success)
                // Note: this does not pop from the stack (it pops from the otherwise stack).
                let pop_jump_placeholder = self.jump_placeholder(Instruction::PopOtherwiseAndJump);

                // Fallback label - patch PushOtherwise to jump here
                let fallback_label = self.label();
                self.patch_jump(push_placeholder, fallback_label, Instruction::PushOtherwise)?;

                // Pop the otherwise handler and compile fallback
                // At this point, if we took this path, the primary's result was discarded
                // So reset stack depth to before primary for fallback compilation
                self.emit(Instruction::PopOtherwise);
                self.transform(fallback)?;

                // Done label - patch PopOtherwiseAndJump to jump here
                // Both paths leave exactly one value on the stack
                let done_label = self.label();
                self.patch_jump(
                    pop_jump_placeholder,
                    done_label,
                    Instruction::PopOtherwiseAndJump,
                )?;
            }

            // === Option Construction ===
            ExprInner::Option { inner } => {
                match inner {
                    Some(value_expr) => {
                        // some expr: compile the inner expression, then wrap with MakeOption(1)
                        self.transform(value_expr)?;
                        // MakeOption(1) pops 1 value and pushes 1 option
                        self.pop_stack();
                        self.emit(Instruction::MakeOption(1));
                        self.push_stack();
                    }
                    None => {
                        // none: just create a None value with MakeOption(0)
                        self.emit(Instruction::MakeOption(0));
                        self.push_stack();
                    }
                }
            }

            // === Function Calls ===
            ExprInner::Call { callable, args } => {
                use crate::types::traits::{TypeKind, TypeView};

                // 1. Compile arguments first (they go on stack before function)
                for arg in args.iter() {
                    self.transform(arg)?;
                }

                // 2. Compile the callable (pushes function value on stack)
                self.transform(callable)?;

                // 3. Extract parameter types from callable's function type
                let param_types: alloc::vec::Vec<_> = match callable.0.view() {
                    TypeKind::Function { params, .. } => params.collect(),
                    _ => panic!("Call on non-function (should be caught by type checker)"),
                };

                // 4. Create and store the adapter
                // TODO: Deduplicate adapters with same parameter types
                let adapter = FunctionAdapter::new(self.type_mgr, param_types);
                let adapter_index = self.adapters.len();
                self.adapters.push(adapter);

                // 5. Emit Call instruction
                self.pop_stack_n(args.len() + 1); // Pop args + function
                self.emit_with_arg(Instruction::Call, adapter_index as u32);
                self.push_stack(); // Push result
            }

            ExprInner::Cast { expr: inner_expr } => {
                // Compile the expression to cast
                self.transform(inner_expr)?;

                // Get source and target types
                let source_type = inner_expr.0;
                let target_type = tree.0;

                // Create cast adapter and store it
                let adapter = CastAdapter::new(self.type_mgr, source_type, target_type);
                let adapter_index = self.generic_adapters.len();
                self.generic_adapters.push(Box::new(adapter));

                // Emit CallGenericAdapter instruction (pops 1, pushes 1)
                self.pop_stack();
                self.emit_with_arg(Instruction::CallGenericAdapter, adapter_index as u32);
                self.push_stack();
            }

            ExprInner::Lambda {
                params,
                body,
                captures,
            } => {
                // Push captured values onto stack (for MakeClosure to consume)
                for &capture_name in captures.iter() {
                    self.compile_variable_load(capture_name)?;
                }

                // Check if this is a polymorphic lambda with multiple instantiations
                let lambda_ptr = tree.as_ptr();
                let instantiations = self
                    .lambda_instantiations
                    .and_then(|map| map.get(&lambda_ptr));

                let closure_index = match instantiations {
                    Some(info) if !info.substitutions.is_empty() => {
                        // Polymorphic lambda: compile Mono entries first, then add Poly entry
                        let num_captures = captures.len() as u32;

                        // Compile each Mono instantiation and collect their indices
                        let mut monos = Vec::new();
                        for substitution in info.substitutions.iter() {
                            let mono_index = self.lambdas.len() as u32;
                            monos.push(mono_index);

                            // Create unification from substitution
                            let subst_map: hashbrown::HashMap<u16, &'types Type<'types>> =
                                substitution.iter().map(|(&k, &v)| (k, v)).collect();
                            let monomorphism =
                                Unification::from_substitution(self.type_mgr, subst_map);

                            // Apply substitution to get concrete function type
                            let concrete_type = monomorphism.fully_resolve(tree.0);

                            // Compile as a Mono lambda
                            let lambda_code = self.compile_lambda_body(
                                params,
                                body,
                                captures,
                                concrete_type,
                                Some(monomorphism),
                            )?;
                            self.lambdas.push(lambda_code);
                        }

                        // Add the Poly entry last - this is what MakeClosure references
                        let poly_index = self.lambdas.len();
                        let poly_entry = LambdaCode {
                            lambda_type: tree.0,
                            num_captures,
                            kind: LambdaKind::Poly { monos },
                        };
                        self.lambdas.push(poly_entry);
                        poly_index
                    }
                    _ => {
                        // Monomorphic lambda: compile once
                        let mono_index = self.lambdas.len();
                        let lambda_code =
                            self.compile_lambda_body(params, body, captures, tree.0, None)?;
                        self.lambdas.push(lambda_code);
                        mono_index
                    }
                };

                // MakeClosure pops captures and pushes closure
                self.pop_stack_n(captures.len());
                self.emit_with_arg(Instruction::MakeClosure, closure_index as u32);
                self.push_stack();
            }

            ExprInner::Match { expr, arms } => {
                // Pattern matching compilation strategy:
                // 1. Compile the matched expression (push value on stack)
                // 2. For each arm:
                //    a. DupN(0) to preserve the matched value for next arm (except last)
                //    b. Push scope for pattern bindings (if any vars)
                //    c. Compile pattern check (consumes value, returns jumps to patch on failure)
                //    d. Pop the original matched value (pattern succeeded, except last arm)
                //    e. Compile body
                //    f. Pop the pattern scope
                //    g. Jump to end (except last arm)
                //    h. Patch pattern fail jumps to next arm
                // 3. Patch all end jumps

                // Compile the matched expression
                self.transform(expr)?;

                // Collect jump placeholders to patch at end
                let mut end_jumps: Vec<usize> = Vec::new();

                for (i, arm) in arms.iter().enumerate() {
                    let is_last_arm = i == arms.len() - 1;

                    // Duplicate the matched value (so we can try next arm if pattern fails)
                    if !is_last_arm {
                        self.emit(Instruction::DupN(0));
                        self.push_stack();
                    }

                    // Push a scope for pattern bindings (if any vars)
                    let has_scope = !arm.vars.is_empty();
                    if has_scope {
                        self.scope_stack.push(
                            IncompleteScope::new(self.arena, arm.vars).expect("Pattern bindings"),
                        );
                    }

                    // Compile pattern check
                    // This consumes the (duplicated) value and either:
                    // - Falls through if pattern matches (bindings are set up in current scope)
                    // - Has fail_jumps that need to be patched to jump to next arm
                    let fail_jumps = self.compile_pattern(arm.pattern, expr.0)?;

                    // Pop the original matched value (it's still on stack under the dup)
                    if !is_last_arm {
                        self.emit(Instruction::Pop);
                        self.pop_stack();
                    }

                    // Compile body (leaves result on stack)
                    self.transform(arm.body)?;

                    // Pop the pattern scope
                    if has_scope {
                        self.scope_stack.pop().expect("Scope stack underflow");
                    }

                    // Jump to end (except for last arm)
                    if !is_last_arm {
                        let end_jump = self.jump_placeholder(Instruction::JumpForward);
                        end_jumps.push(end_jump);
                    }

                    // Patch pattern fail jumps to next arm (after end_jump for non-last arms)
                    let next_arm_label = self.label();
                    for fail_jump in fail_jumps {
                        self.patch_jump(
                            fail_jump.placeholder,
                            next_arm_label,
                            fail_jump.make_jump,
                        )?;
                    }
                }

                // Patch all end jumps to point here
                let end_label = self.label();
                for end_jump in end_jumps {
                    self.patch_jump(end_jump, end_label, Instruction::JumpForward)?;
                }

                // Stack depth: matched expr was consumed, body result is on stack
            }

            ExprInner::FormatStr { strs, exprs } => {
                // 1. Compile all expressions (push values onto stack in order)
                for expr in exprs.iter() {
                    self.transform(expr)?;
                }

                // 2. Collect expression types for the adapter
                let expr_types: alloc::vec::Vec<_> = exprs.iter().map(|e| e.0).collect();

                // 3. Create and store FormatStrAdapter (copies strings internally)
                let adapter = FormatStrAdapter::new(self.type_mgr, &expr_types, strs);
                let adapter_index = self.generic_adapters.len();
                self.generic_adapters.push(Box::new(adapter));

                // 4. Emit CallGenericAdapter instruction
                // Stack effect: pops N expression values, pushes 1 result string
                self.pop_stack_n(exprs.len());
                self.emit_with_arg(Instruction::CallGenericAdapter, adapter_index as u32);
                self.push_stack();
            }
        }
        Ok(())
    }
}
