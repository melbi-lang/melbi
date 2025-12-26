#![allow(unsafe_code)]

use core::cmp::Ordering;

use bumpalo::Bump;

use super::instruction_set::Instruction;

use crate::{
    String, Vec,
    evaluator::{ExecutionError, ExecutionErrorKind, RuntimeError},
    format,
    parser::{ComparisonOp, Span},
    values::{ArrayData, BytecodeLambda, LambdaInstantiation, MapData, RawValue, RecordData},
    vm::{Code, GenericAdapter, LambdaKind, Stack},
};

struct OtherwiseBlock {
    fallback: *const Instruction,
    stack_size: usize,
}

pub struct VM<'a, 'b, 'c> {
    arena: &'a Bump,
    code: &'b Code<'c>,
    ip: *const Instruction,
    stack: Stack<RawValue>,
    locals: Vec<RawValue>,
    otherwise_stack: Vec<OtherwiseBlock>,
    /// Captured values for the current closure (empty for top-level code)
    captures: &'a [RawValue],
}

impl<'a, 'b, 'c> VM<'a, 'b, 'c> {
    /// Create a new VM.
    ///
    /// # Arguments
    /// * `arena` - Arena for allocations during execution
    /// * `code` - The bytecode to execute
    /// * `locals` - Initial local variables (e.g., function arguments)
    /// * `captures` - Captured values for closure execution
    pub fn new(
        arena: &'a Bump,
        code: &'b Code<'c>,
        locals: Vec<RawValue>,
        captures: &'a [RawValue],
    ) -> Self {
        VM {
            arena,
            code,
            ip: unsafe { code.instructions.as_ptr().sub(1) },
            stack: Stack::new(code.max_stack_size),
            locals,
            otherwise_stack: Vec::new(),
            captures,
        }
    }

    pub fn execute(arena: &'a Bump, code: &'b Code<'c>) -> Result<RawValue, ExecutionError> {
        let mut vm = VM::new(arena, code, Vec::new(), &[]);
        vm.run()
    }

    pub fn run(&mut self) -> Result<RawValue, ExecutionError> {
        let result = self.run_control_loop();
        debug_assert!(self.stack.is_empty(), "Stack should be empty.");
        result
    }

    #[inline(always)]
    fn run_control_loop(&mut self) -> Result<RawValue, ExecutionError> {
        loop {
            let result = self.run_main_loop();
            match result {
                Err(e) => {
                    // If we are within an area that is covered by an `otherwise` block
                    // then `otherwise_stack` will be non empty.
                    if let Some(block) = self.otherwise_stack.last() {
                        // `otherwise` can only handle `Runtime` error kind.
                        if let ExecutionErrorKind::Runtime(runtime_error) = e {
                            tracing::debug!(error = %runtime_error, "Handled by `otherwise` block");
                            self.ip = block.fallback;
                            self.stack.pop_n(self.stack.len() - block.stack_size);
                            continue;
                        }
                    }
                    self.stack.clear();
                    return Err(e).map_err(|e| ExecutionError {
                        kind: e,
                        // TODO: Add source and span information.
                        source: String::new(),
                        span: Span(0..0),
                    });
                }
                Ok(()) => {
                    return Ok(self.stack.pop());
                }
            }
        }
    }

    #[inline(always)]
    pub fn run_main_loop(&mut self) -> Result<(), ExecutionErrorKind> {
        let mut wide_arg: usize = 0;
        loop {
            self.ip = unsafe { self.ip.add(1) };

            use Instruction::*;
            match unsafe { *self.ip } {
                ConstLoad(arg) => {
                    let index = wide_arg | arg as usize;
                    self.stack.push(self.code.constants[index]);
                }
                ConstInt(value) => {
                    self.stack.push(RawValue::make_int(value as i64));
                }
                ConstUInt(value) => {
                    self.stack.push(RawValue::make_int(value as i64));
                }
                ConstBool(value) => {
                    self.stack.push(RawValue::make_bool(value != 0));
                }
                WideArg(arg) => {
                    wide_arg |= arg as usize;
                    wide_arg <<= 8;
                    continue;
                }
                IntBinOp(b'+') => {
                    let b = self.stack.pop();
                    self.stack[0] = RawValue::make_int(
                        self.stack[0]
                            .as_int_unchecked()
                            .wrapping_add(b.as_int_unchecked()),
                    );
                }
                IntBinOp(b'-') => {
                    let b = self.stack.pop();
                    self.stack[0] = RawValue::make_int(
                        self.stack[0]
                            .as_int_unchecked()
                            .wrapping_sub(b.as_int_unchecked()),
                    );
                }
                IntBinOp(b'*') => {
                    let b = self.stack.pop();
                    self.stack[0] = RawValue::make_int(
                        self.stack[0]
                            .as_int_unchecked()
                            .wrapping_mul(b.as_int_unchecked()),
                    );
                }
                IntBinOp(b'/') => {
                    let b = self.stack.pop();
                    let a = self.stack.pop();

                    if b.as_int_unchecked() == 0 {
                        return Err(RuntimeError::DivisionByZero {}.into());
                    }
                    if a.as_int_unchecked() == i64::MIN && b.as_int_unchecked() == -1 {
                        return Err(RuntimeError::IntegerOverflow {}.into());
                    }

                    self.stack.push(RawValue::make_int(
                        a.as_int_unchecked().div_euclid(b.as_int_unchecked()),
                    ));
                }
                IntBinOp(b'%') => {
                    let b = self.stack.pop();
                    let a = self.stack.pop();

                    if b.as_int_unchecked() == 0 {
                        return Err(RuntimeError::DivisionByZero {}.into());
                    }
                    if a.as_int_unchecked() == i64::MIN && b.as_int_unchecked() == -1 {
                        return Err(RuntimeError::IntegerOverflow {}.into());
                    }

                    self.stack.push(RawValue::make_int(
                        a.as_int_unchecked().rem_euclid(b.as_int_unchecked()),
                    ));
                }
                IntBinOp(b'^') => {
                    let b = self.stack.pop().as_int_unchecked();
                    let a = self.stack.pop().as_int_unchecked();
                    let result = if b < 0 {
                        0
                    } else if b > u32::MAX as i64 {
                        0
                    } else {
                        a.wrapping_pow(b as u32)
                    };
                    self.stack.push(RawValue::make_int(result));
                }

                // Integer unary operations
                NegInt => {
                    let a = self.stack.pop().as_int_unchecked();
                    self.stack.push(RawValue::make_int(a.wrapping_neg()));
                }

                // Integer comparisons
                IntCmpOp(op) => {
                    let b = self.stack.pop().as_int_unchecked();
                    let a = self.stack.pop().as_int_unchecked();
                    let result = match op {
                        ComparisonOp::Lt => a < b,
                        ComparisonOp::Gt => a > b,
                        ComparisonOp::Eq => a == b,
                        ComparisonOp::Neq => a != b,
                        ComparisonOp::Le => a <= b,
                        ComparisonOp::Ge => a >= b,
                        ComparisonOp::In | ComparisonOp::NotIn => {
                            panic!("In/NotIn not valid for integers (type checker bug)")
                        }
                    };
                    self.stack.push(RawValue::make_bool(result));
                }

                // Float binary operations
                FloatBinOp(b'+') => {
                    let b = self.stack.pop();
                    self.stack[0] = RawValue::make_float(
                        self.stack[0].as_float_unchecked() + b.as_float_unchecked(),
                    );
                }
                FloatBinOp(b'-') => {
                    let b = self.stack.pop();
                    self.stack[0] = RawValue::make_float(
                        self.stack[0].as_float_unchecked() - b.as_float_unchecked(),
                    );
                }
                FloatBinOp(b'*') => {
                    let b = self.stack.pop();
                    self.stack[0] = RawValue::make_float(
                        self.stack[0].as_float_unchecked() * b.as_float_unchecked(),
                    );
                }
                FloatBinOp(b'/') => {
                    let b = self.stack.pop();
                    self.stack[0] = RawValue::make_float(
                        self.stack[0].as_float_unchecked() / b.as_float_unchecked(),
                    );
                }
                FloatBinOp(b'^') => {
                    let b = self.stack.pop();
                    let a = self.stack.pop();
                    self.stack.push(RawValue::make_float(
                        a.as_float_unchecked().powf(b.as_float_unchecked()),
                    ));
                }

                NegFloat => {
                    let a = self.stack.pop();
                    self.stack
                        .push(RawValue::make_float(-a.as_float_unchecked()));
                }

                // Float comparisons
                FloatCmpOp(op) => {
                    let b = self.stack.pop().as_float_unchecked();
                    let a = self.stack.pop().as_float_unchecked();
                    let result = match op {
                        ComparisonOp::Lt => a < b,
                        ComparisonOp::Gt => a > b,
                        ComparisonOp::Eq => a == b,
                        ComparisonOp::Neq => a != b,
                        ComparisonOp::Le => a <= b,
                        ComparisonOp::Ge => a >= b,
                        ComparisonOp::In | ComparisonOp::NotIn => {
                            panic!("In/NotIn not valid for floats (type checker bug)")
                        }
                    };
                    self.stack.push(RawValue::make_bool(result));
                }

                BytesGet => {
                    let index_i64 = self.stack.pop().as_int_unchecked();
                    let bytes = self.stack.pop().as_bytes_unchecked();
                    let Some(index) = calculate_index(index_i64, bytes.len()) else {
                        return Err(RuntimeError::IndexOutOfBounds {
                            index: index_i64,
                            len: bytes.len(),
                        }
                        .into());
                    };
                    self.stack.push(RawValue::make_int(bytes[index] as i64));
                }

                BytesGetConst(arg) => {
                    let index = wide_arg | arg as usize;
                    let bytes = self.stack.pop().as_bytes_unchecked();
                    if index > bytes.len() {
                        return Err(RuntimeError::IndexOutOfBounds {
                            index: index as i64,
                            len: bytes.len(),
                        }
                        .into());
                    }
                    self.stack.push(RawValue::make_int(bytes[index] as i64));
                }

                BytesCmpOp(op) => {
                    let b = self.stack.pop().as_bytes_unchecked();
                    let a = self.stack.pop().as_bytes_unchecked();
                    let ordering = a.cmp(b);
                    let result = match op {
                        ComparisonOp::Lt => ordering == Ordering::Less,
                        ComparisonOp::Gt => ordering == Ordering::Greater,
                        ComparisonOp::Eq => ordering == Ordering::Equal,
                        ComparisonOp::Neq => ordering != Ordering::Equal,
                        ComparisonOp::Le => ordering != Ordering::Greater,
                        ComparisonOp::Ge => ordering != Ordering::Less,
                        ComparisonOp::In | ComparisonOp::NotIn => {
                            // needle `a` in haystack `b`
                            let contains = if a.is_empty() {
                                true
                            } else if a.len() > b.len() {
                                false
                            } else {
                                b.windows(a.len()).any(|w| w == a)
                            };
                            if op == ComparisonOp::In {
                                contains
                            } else {
                                !contains
                            }
                        }
                    };
                    self.stack.push(RawValue::make_bool(result));
                }

                StringCmpOp(op) => {
                    let b = self.stack.pop().as_str_unchecked();
                    let a = self.stack.pop().as_str_unchecked();
                    let ordering = a.cmp(b);
                    let result = match op {
                        ComparisonOp::Lt => ordering == Ordering::Less,
                        ComparisonOp::Gt => ordering == Ordering::Greater,
                        ComparisonOp::Eq => ordering == Ordering::Equal,
                        ComparisonOp::Neq => ordering != Ordering::Equal,
                        ComparisonOp::Le => ordering != Ordering::Greater,
                        ComparisonOp::Ge => ordering != Ordering::Less,
                        ComparisonOp::In | ComparisonOp::NotIn => {
                            // needle `a` in haystack `b`
                            let contains = b.contains(a);
                            if op == ComparisonOp::In {
                                contains
                            } else {
                                !contains
                            }
                        }
                    };
                    self.stack.push(RawValue::make_bool(result));
                }

                // Logical operations
                And => {
                    let b = self.stack.pop();
                    let a = self.stack.pop();
                    self.stack.push(RawValue::make_bool(
                        a.as_bool_unchecked() && b.as_bool_unchecked(),
                    ));
                }
                Or => {
                    let b = self.stack.pop();
                    let a = self.stack.pop();
                    self.stack.push(RawValue::make_bool(
                        a.as_bool_unchecked() || b.as_bool_unchecked(),
                    ));
                }
                Not => {
                    let a = self.stack.pop();
                    self.stack.push(RawValue::make_bool(!a.as_bool_unchecked()));
                }
                EqBool => {
                    let b = self.stack.pop();
                    let a = self.stack.pop();
                    self.stack.push(RawValue::make_bool(
                        a.as_bool_unchecked() == b.as_bool_unchecked(),
                    ));
                }

                // Stack operations
                DupN(depth) => {
                    let val = *self.stack.peek_at(depth as usize).unwrap();
                    self.stack.push(val);
                }
                Pop => {
                    self.stack.pop();
                }
                Swap => {
                    let b = self.stack.pop();
                    let a = self.stack.pop();
                    self.stack.push(b);
                    self.stack.push(a);
                }

                // Local variables
                LoadLocal(arg) => {
                    let index = wide_arg | arg as usize;
                    self.stack.push(self.locals[index]);
                }
                StoreLocal(arg) => {
                    let index = wide_arg | arg as usize;
                    let val = self.stack.pop();
                    if self.locals.len() <= index {
                        self.locals.resize(index + 1, RawValue::make_int(0));
                    }
                    self.locals[index] = val;
                }

                // Control flow
                JumpForward(arg) => {
                    let delta = wide_arg | arg as usize;
                    self.ip = unsafe { self.ip.add(delta) };
                }
                PopJumpIfFalse(arg) => {
                    let delta = wide_arg | arg as usize;
                    let cond = self.stack.pop();
                    if !cond.as_bool_unchecked() {
                        self.ip = unsafe { self.ip.add(delta) };
                    }
                }
                PopJumpIfTrue(arg) => {
                    let delta = wide_arg | arg as usize;
                    let cond = self.stack.pop();
                    if cond.as_bool_unchecked() {
                        self.ip = unsafe { self.ip.add(delta) };
                    }
                }

                Halt => {
                    return Ok(());
                }
                Return => {
                    return Ok(());
                }

                // === Otherwise Error Handling ===
                PushOtherwise(arg) => {
                    // Calculate fallback instruction pointer
                    let delta = wide_arg | arg as usize;
                    let fallback_ip = unsafe { self.ip.add(delta) };

                    // Push handler onto otherwise_stack
                    self.otherwise_stack.push(OtherwiseBlock {
                        fallback: fallback_ip,
                        stack_size: self.stack.len(),
                    });
                }

                PopOtherwise => {
                    // Remove the top otherwise handler (called in fallback code)
                    self.otherwise_stack
                        .pop()
                        .expect("PopOtherwise called with empty otherwise_stack");
                }

                PopOtherwiseAndJump(arg) => {
                    // Remove the otherwise handler (not needed, primary succeeded)
                    let delta = wide_arg | arg as usize;
                    self.otherwise_stack
                        .pop()
                        .expect("PopOtherwiseAndJump called with empty otherwise_stack");

                    // Jump past fallback code to done label
                    self.ip = unsafe { self.ip.add(delta) };
                }

                Nop => {
                    // No operation
                }

                MakeArray(arg) => {
                    let len = wide_arg | arg as usize;
                    let array = ArrayData::new_with(self.arena, self.stack.top_n(len));
                    self.stack.pop_n(len);
                    self.stack.push(array.as_raw_value());
                }

                Call(arg) => {
                    let adapter_index = wide_arg | arg as usize;

                    let adapter = &self.code.adapters[adapter_index];
                    let num_args = adapter.num_args();
                    let args = self.stack.top_n(num_args);

                    let result = adapter.call(self.arena, args)?;

                    // Pop arguments from stack after the call
                    self.stack.pop_n(num_args);

                    // Push the result
                    self.stack.push(result);
                }

                CallGenericAdapter(arg) => {
                    let adapter_index = wide_arg | arg as usize;

                    let adapter = &self.code.generic_adapters[adapter_index];
                    let num_args = adapter.num_args();
                    let args = self.stack.top_n(num_args);

                    let result = adapter.call(self.arena, args)?;

                    // Pop arguments from stack after the call
                    self.stack.pop_n(num_args);

                    // Push the result
                    self.stack.push(result);
                }

                // === Closure Operations ===
                LoadCapture(arg) => {
                    let index = wide_arg | arg as usize;
                    let value = self.captures[index];
                    self.stack.push(value);
                }

                MakeClosure(arg) => {
                    let lambda_index = wide_arg | arg as usize;
                    let lambda_code = &self.code.lambdas[lambda_index];
                    let num_captures = lambda_code.num_captures as usize;

                    // Copy captures from stack to arena
                    let capture_values = self.stack.top_n(num_captures);
                    let captures = self.arena.alloc_slice_copy(capture_values);

                    // Build instantiations slice based on lambda kind
                    let instantiations: &[LambdaInstantiation] = match &lambda_code.kind {
                        LambdaKind::Mono { code } => {
                            // Single instantiation - use this lambda directly
                            self.arena.alloc_slice_copy(&[LambdaInstantiation {
                                fn_type: lambda_code.lambda_type,
                                code,
                            }])
                        }
                        LambdaKind::Poly { monos } => {
                            // Multiple instantiations - look up each by index
                            self.arena.alloc_slice_fill_iter(monos.iter().map(|&idx| {
                                let mono = &self.code.lambdas[idx as usize];
                                let LambdaKind::Mono { code } = &mono.kind else {
                                    panic!(
                                        "Poly lambda references non-Mono lambda at index {}",
                                        idx
                                    )
                                };
                                LambdaInstantiation {
                                    fn_type: mono.lambda_type,
                                    code,
                                }
                            }))
                        }
                    };

                    // Create BytecodeLambda with all instantiations
                    let lambda =
                        BytecodeLambda::new(lambda_code.lambda_type, instantiations, captures);
                    let raw = RawValue::make_function(self.arena, lambda);

                    self.stack.pop_n(num_captures);
                    self.stack.push(raw);
                }

                // === Array Operations ===
                ArrayGet => {
                    // Stack: [..., array, index] -> [..., element]
                    let index_i64 = self.stack.pop().as_int_unchecked();
                    let array_raw = self.stack.pop();
                    let array = ArrayData::from_raw_value(array_raw);

                    let Some(index) = calculate_index(index_i64, array.length()) else {
                        return Err(RuntimeError::IndexOutOfBounds {
                            index: index_i64,
                            len: array.length(),
                        }
                        .into());
                    };

                    let element = unsafe { array.get_unchecked(index) };
                    self.stack.push(element);
                }

                ArrayGetConst(arg) => {
                    // Stack: [..., array] -> [..., element]
                    let index = wide_arg | arg as usize;
                    let array_raw = self.stack.pop();
                    let array = ArrayData::from_raw_value(array_raw);

                    // Check bounds
                    if index >= array.length() {
                        return Err(RuntimeError::IndexOutOfBounds {
                            index: index as i64,
                            len: array.length(),
                        }
                        .into());
                    }

                    let element = unsafe { array.get_unchecked(index) };
                    self.stack.push(element);
                }

                ArrayLen | ArrayConcat | ArraySlice | ArrayAppend => {
                    todo!("Other array operations")
                }

                // === Map Operations ===
                MapGet => {
                    // Stack: [..., map, key] -> [..., value]
                    let key = self.stack.pop();
                    let map_raw = self.stack.pop();
                    let map = MapData::from_raw_value(map_raw);

                    // Linear search for the key
                    // TODO: Use binary search since map is sorted
                    let mut found = None;
                    for i in 0..map.length() {
                        let entry_key = unsafe { map.get_key(i) };
                        // For now, do simple bitwise comparison
                        // This works for Int, Bool, and other primitive types
                        // TODO: Proper value equality for complex types
                        if entry_key.as_int_unchecked() == key.as_int_unchecked() {
                            found = Some(unsafe { map.get_value(i) });
                            break;
                        }
                    }

                    match found {
                        Some(value) => self.stack.push(value),
                        None => {
                            // Format key for error message (simple int display for now)
                            let key_display = format!("{}", key.as_int_unchecked());
                            return Err(RuntimeError::KeyNotFound { key_display }.into());
                        }
                    }
                }

                MakeMap(arg) => {
                    // Stack: [..., key1, val1, key2, val2, ..., keyN, valN] -> [..., map]
                    use crate::Vec;
                    use crate::values::raw::MapEntry;

                    let num_pairs = wide_arg | arg as usize;
                    let num_values = num_pairs * 2;

                    // Get all key-value pairs from stack
                    let values = self.stack.top_n(num_values);

                    // Create MapEntry structs
                    let mut entries: Vec<MapEntry> = Vec::with_capacity(num_pairs);
                    for i in 0..num_pairs {
                        let key_idx = i * 2;
                        let val_idx = i * 2 + 1;
                        entries.push(MapEntry {
                            key: values[key_idx],
                            value: values[val_idx],
                        });
                    }

                    // Sort entries by key (integer comparison for now)
                    // TODO: Proper multi-type key comparison
                    entries.sort_by(|a, b| a.key.as_int_unchecked().cmp(&b.key.as_int_unchecked()));

                    // Create the map
                    let map = MapData::new_with_sorted(self.arena, &entries);

                    // Pop the 2*N elements
                    self.stack.pop_n(num_values);

                    // Push the map result
                    self.stack.push(map.as_raw_value());
                }

                MapHas => {
                    // Stack: [..., key, map] -> [..., result: Bool]
                    // For now, just push false (placeholder implementation)
                    let _map = self.stack.pop();
                    let _key = self.stack.pop();
                    self.stack.push(RawValue::make_bool(false));
                }

                MapLen | MapInsert | MapRemove | MapKeys | MapValues => {
                    todo!("Other map operations")
                }

                // === Record Operations ===
                MakeRecord(arg) => {
                    // Stack: [..., val0, val1, ..., valN] -> [..., record]
                    let num_fields = wide_arg | arg as usize;
                    // Get the top N elements to create the record
                    let record = RecordData::new_with(self.arena, self.stack.top_n(num_fields));
                    // Pop the N elements that were used to create the record
                    self.stack.pop_n(num_fields);
                    // Push the record result
                    self.stack.push(record.as_raw_value());
                }

                RecordGet(arg) => {
                    // Stack: [..., record] -> [..., field_value]
                    let index = wide_arg | arg as usize;
                    let record_raw = self.stack.pop();
                    let record = RecordData::from_raw_value(record_raw);
                    debug_assert!(index < record.length());

                    let field_value = unsafe { record.get(index) };
                    self.stack.push(field_value);
                }

                RecordMerge => {
                    todo!("Other record operations")
                }

                // === Option Construction ===
                MakeOption(is_some) => {
                    let option_value = match is_some {
                        0 => None,
                        1 => {
                            let value = self.stack.pop();
                            Some(value)
                        }
                        _ => panic!("Invalid MakeOption operand: {}", is_some),
                    };
                    self.stack
                        .push(RawValue::make_optional(self.arena, option_value));
                }

                StringFormat(_) => {
                    todo!("String operations")
                }
                BytesSlice | StringToBytes | BytesToString => {
                    todo!("Bytes operations")
                }
                Eq | NotEq => {
                    todo!("Equality operations")
                }
                MatchSomeOrJump(arg) => {
                    let delta = wide_arg | arg as usize;
                    let option = self.stack.pop();
                    match option.as_optional_unchecked() {
                        // Some: push inner value and fall through
                        Some(inner) => {
                            self.stack.push(inner);
                        }
                        // None: jump forward
                        None => {
                            self.ip = unsafe { self.ip.add(delta) };
                        }
                    }
                }
                MatchNoneOrJump(arg) => {
                    let delta = wide_arg | arg as usize;
                    let option = self.stack.pop();
                    match option.as_optional_unchecked() {
                        Some(_) => {
                            // Some: jump forward
                            self.ip = unsafe { self.ip.add(delta) };
                        }
                        None => {
                            // None: fall through (value already popped)
                        }
                    }
                }
                Breakpoint(_) | CheckLimits | Trace(_) | InlineCache(_) => {
                    todo!("Debug/meta operations")
                }

                _ => {
                    panic!("Unsupported operation: {:?}", unsafe { *self.ip });
                }
            }
            wide_arg = 0;
        }
    }
}

/// Calculate the index for an array or bytes value, supporting negative indices,
/// and checking for out-of-bounds errors.
fn calculate_index(mut index: i64, len: usize) -> Option<usize> {
    if index < 0 {
        index = index.checked_add(len as i64)?;
    }
    let index_usize: usize = index.try_into().ok()?;
    if index_usize >= len {
        return None;
    }
    Some(index_usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_works() {
        use Instruction::*;
        let code = Code {
            constants: vec![RawValue::make_int(42)],
            adapters: vec![],
            generic_adapters: vec![],
            instructions: vec![ConstLoad(0), ConstInt(2), IntBinOp(b'*'), Return],
            num_locals: 0,
            max_stack_size: 2,
            lambdas: vec![],
        };
        let arena = Bump::new();
        let mut vm = VM::new(&arena, &code, Vec::new(), &[]);
        assert_eq!(vm.run().unwrap().as_int_unchecked(), 84);
    }

    #[test]
    fn test_wide() {
        use Instruction::*;
        let mut code = Code {
            constants: vec![RawValue::make_int(2)],
            adapters: vec![],
            generic_adapters: vec![],
            instructions: vec![
                ConstLoad(0),
                WideArg(0x01),
                ConstLoad(0x00),
                IntBinOp(b'*'),
                Return,
            ],
            num_locals: 0,
            max_stack_size: 2,
            lambdas: vec![],
        };
        code.constants.resize(257, RawValue::make_int(0));
        code.constants[256] = RawValue::make_int(42);
        let arena = Bump::new();
        let mut vm = VM::new(&arena, &code, Vec::new(), &[]);
        assert_eq!(vm.run().unwrap().as_int_unchecked(), 84);
    }

    #[test]
    fn test_int_comparisons() {
        use Instruction::*;

        // Test <
        let code = Code {
            constants: vec![],
            adapters: vec![],
            generic_adapters: vec![],
            instructions: vec![
                ConstInt(5),
                ConstInt(10),
                IntCmpOp(ComparisonOp::Lt),
                Return,
            ],
            num_locals: 0,
            max_stack_size: 2,
            lambdas: vec![],
        };
        let arena = Bump::new();
        let mut vm = VM::new(&arena, &code, Vec::new(), &[]);
        assert_eq!(vm.run().unwrap().as_bool_unchecked(), true);

        // Test ==
        let code = Code {
            constants: vec![],
            adapters: vec![],
            generic_adapters: vec![],
            instructions: vec![
                ConstInt(42),
                ConstInt(42),
                IntCmpOp(ComparisonOp::Eq),
                Return,
            ],
            num_locals: 0,
            max_stack_size: 2,
            lambdas: vec![],
        };
        let mut vm = VM::new(&arena, &code, Vec::new(), &[]);
        assert!(vm.run().unwrap().as_bool_unchecked());
    }

    #[test]
    fn test_float_ops() {
        use Instruction::*;

        let code = Code {
            constants: vec![RawValue::make_float(3.5), RawValue::make_float(2.0)],
            adapters: vec![],
            generic_adapters: vec![],
            instructions: vec![ConstLoad(0), ConstLoad(1), FloatBinOp(b'+'), Return],
            num_locals: 0,
            max_stack_size: 2,
            lambdas: vec![],
        };
        let arena = Bump::new();
        let mut vm = VM::new(&arena, &code, Vec::new(), &[]);
        assert_eq!(vm.run().unwrap().as_float_unchecked(), 5.5);
    }

    #[test]
    fn test_logical_ops() {
        use Instruction::*;

        // Test AND
        let code = Code {
            constants: vec![],
            adapters: vec![],
            generic_adapters: vec![],
            instructions: vec![ConstBool(1), ConstBool(0), And, Return],
            num_locals: 0,
            max_stack_size: 2,
            lambdas: vec![],
        };
        let arena = Bump::new();
        let mut vm = VM::new(&arena, &code, Vec::new(), &[]);
        assert_eq!(vm.run().unwrap().as_bool_unchecked(), false);

        // Test OR
        let code = Code {
            constants: vec![],
            adapters: vec![],
            generic_adapters: vec![],
            instructions: vec![ConstBool(1), ConstBool(0), Or, Return],
            num_locals: 0,
            max_stack_size: 2,
            lambdas: vec![],
        };
        let mut vm = VM::new(&arena, &code, Vec::new(), &[]);
        assert!(vm.run().unwrap().as_bool_unchecked());

        // Test NOT
        let code = Code {
            constants: vec![],
            adapters: vec![],
            generic_adapters: vec![],
            instructions: vec![ConstBool(0), Not, Return],
            num_locals: 0,
            max_stack_size: 1,
            lambdas: vec![],
        };
        let mut vm = VM::new(&arena, &code, Vec::new(), &[]);
        assert!(vm.run().unwrap().as_bool_unchecked());
    }

    #[test]
    fn test_stack_ops() {
        use Instruction::*;

        // Test Dup
        let code = Code {
            constants: vec![],
            adapters: vec![],
            generic_adapters: vec![],
            instructions: vec![ConstInt(42), DupN(0), IntBinOp(b'+'), Return],
            num_locals: 0,
            max_stack_size: 2,
            lambdas: vec![],
        };
        let arena = Bump::new();
        let mut vm = VM::new(&arena, &code, Vec::new(), &[]);
        assert_eq!(vm.run().unwrap().as_int_unchecked(), 84);

        // Test Swap
        let code = Code {
            constants: vec![],
            adapters: vec![],
            generic_adapters: vec![],
            instructions: vec![ConstInt(10), ConstInt(5), Swap, IntBinOp(b'-'), Return],
            num_locals: 0,
            max_stack_size: 2,
            lambdas: vec![],
        };
        let mut vm = VM::new(&arena, &code, Vec::new(), &[]);
        assert_eq!(vm.run().unwrap().as_int_unchecked(), -5);
    }

    #[test]
    fn test_local_vars() {
        use Instruction::*;

        // Store and load local variable
        let code = Code {
            constants: vec![],
            adapters: vec![],
            generic_adapters: vec![],
            instructions: vec![
                ConstInt(42),
                StoreLocal(0),
                ConstInt(10),
                LoadLocal(0),
                IntBinOp(b'+'),
                Return,
            ],
            num_locals: 1,
            max_stack_size: 2,
            lambdas: vec![],
        };
        let arena = Bump::new();
        let mut vm = VM::new(&arena, &code, Vec::new(), &[]);
        assert_eq!(vm.run().unwrap().as_int_unchecked(), 52);
    }

    #[test]
    fn test_jumps() {
        use Instruction::*;

        // Unconditional jump
        let code = Code {
            constants: vec![],
            adapters: vec![],
            generic_adapters: vec![],
            instructions: vec![
                ConstInt(1),
                JumpForward(2), // Skip next 2 instructions
                ConstInt(50),   // Skipped
                ConstInt(60),   // Skipped
                ConstInt(3),
                IntBinOp(b'+'),
                Return,
            ],
            num_locals: 0,
            max_stack_size: 2,
            lambdas: vec![],
        };
        let arena = Bump::new();
        let mut vm = VM::new(&arena, &code, Vec::new(), &[]);
        assert_eq!(vm.run().unwrap().as_int_unchecked(), 4);
    }

    #[test]
    fn test_conditional_jumps() {
        use Instruction::*;

        // JumpIfTrue - should jump
        let code = Code {
            constants: vec![],
            adapters: vec![],
            generic_adapters: vec![],
            instructions: vec![
                ConstBool(1),
                PopJumpIfTrue(1), // Skip next instruction
                ConstInt(99),     // Skipped
                ConstInt(42),
                Return,
            ],
            num_locals: 0,
            max_stack_size: 2,
            lambdas: vec![],
        };
        let arena = Bump::new();
        let mut vm = VM::new(&arena, &code, Vec::new(), &[]);
        assert_eq!(vm.run().unwrap().as_int_unchecked(), 42);

        // JumpIfFalse - should not jump
        let code = Code {
            constants: vec![],
            adapters: vec![],
            generic_adapters: vec![],
            instructions: vec![
                ConstBool(1),
                PopJumpIfFalse(1), // Don't jump
                ConstInt(42),
                Return,
                ConstInt(99),
            ],
            num_locals: 0,
            max_stack_size: 2,
            lambdas: vec![],
        };
        let mut vm = VM::new(&arena, &code, Vec::new(), &[]);
        assert_eq!(vm.run().unwrap().as_int_unchecked(), 42);
    }

    #[test]
    fn test_unary_ops() {
        use Instruction::*;

        // NegInt
        let code = Code {
            constants: vec![],
            adapters: vec![],
            generic_adapters: vec![],
            instructions: vec![ConstInt(42), NegInt, Return],
            num_locals: 0,
            max_stack_size: 1,
            lambdas: vec![],
        };
        let arena = Bump::new();
        let mut vm = VM::new(&arena, &code, Vec::new(), &[]);
        assert_eq!(vm.run().unwrap().as_int_unchecked(), -42);
    }

    // ========================================================================
    // Division Tests (Euclidean)
    // ========================================================================

    #[test]
    fn test_int_division_basic() {
        use Instruction::*;

        let code = Code {
            constants: vec![],
            adapters: vec![],
            generic_adapters: vec![],
            instructions: vec![ConstInt(10), ConstInt(3), IntBinOp(b'/'), Return],
            num_locals: 0,
            max_stack_size: 2,
            lambdas: vec![],
        };
        let arena = Bump::new();
        let mut vm = VM::new(&arena, &code, Vec::new(), &[]);
        assert_eq!(vm.run().unwrap().as_int_unchecked(), 3);
    }

    #[test]
    fn test_int_division_euclidean_negative_dividend() {
        use Instruction::*;

        // Euclidean: -7 / 3 = -3 (not -2 like truncated)
        // because -7 = -3 * 3 + 2 (remainder is non-negative)
        let code = Code {
            constants: vec![],
            adapters: vec![],
            generic_adapters: vec![],
            instructions: vec![ConstInt(-7), ConstInt(3), IntBinOp(b'/'), Return],
            num_locals: 0,
            max_stack_size: 2,
            lambdas: vec![],
        };
        let arena = Bump::new();
        let mut vm = VM::new(&arena, &code, Vec::new(), &[]);
        assert_eq!(vm.run().unwrap().as_int_unchecked(), -3);
    }

    #[test]
    fn test_int_division_euclidean_negative_divisor() {
        use Instruction::*;

        // Euclidean: 7 / -3 = -2
        // because 7 = -2 * (-3) + 1
        let code = Code {
            constants: vec![],
            adapters: vec![],
            generic_adapters: vec![],
            instructions: vec![ConstInt(7), ConstInt(-3), IntBinOp(b'/'), Return],
            num_locals: 0,
            max_stack_size: 2,
            lambdas: vec![],
        };
        let arena = Bump::new();
        let mut vm = VM::new(&arena, &code, Vec::new(), &[]);
        assert_eq!(vm.run().unwrap().as_int_unchecked(), -2);
    }

    #[test]
    fn test_int_division_euclidean_both_negative() {
        use Instruction::*;

        // Euclidean: -7 / -3 = 3 (not 2 like truncated)
        // because -7 = 3 * (-3) + 2
        let code = Code {
            constants: vec![],
            adapters: vec![],
            generic_adapters: vec![],
            instructions: vec![ConstInt(-7), ConstInt(-3), IntBinOp(b'/'), Return],
            num_locals: 0,
            max_stack_size: 2,
            lambdas: vec![],
        };
        let arena = Bump::new();
        let mut vm = VM::new(&arena, &code, Vec::new(), &[]);
        assert_eq!(vm.run().unwrap().as_int_unchecked(), 3);
    }

    #[test]
    fn test_int_division_by_zero() {
        use Instruction::*;

        let code = Code {
            constants: vec![],
            adapters: vec![],
            generic_adapters: vec![],
            instructions: vec![ConstInt(10), ConstInt(0), IntBinOp(b'/'), Return],
            num_locals: 0,
            max_stack_size: 2,
            lambdas: vec![],
        };
        let arena = Bump::new();
        let mut vm = VM::new(&arena, &code, Vec::new(), &[]);
        let result = vm.run();
        assert!(matches!(
            result,
            Err(ExecutionError {
                kind: ExecutionErrorKind::Runtime(RuntimeError::DivisionByZero {}),
                ..
            })
        ));
    }

    #[test]
    fn test_int_division_i64_min_overflow() {
        use Instruction::*;

        // i64::MIN / -1 would overflow
        // Use constants pool for i64::MIN since ConstInt only takes i8
        let code = Code {
            constants: vec![RawValue::make_int(i64::MIN)],
            adapters: vec![],
            generic_adapters: vec![],
            instructions: vec![ConstLoad(0), ConstInt(-1), IntBinOp(b'/'), Return],
            num_locals: 0,
            max_stack_size: 2,
            lambdas: vec![],
        };
        let arena = Bump::new();
        let mut vm = VM::new(&arena, &code, Vec::new(), &[]);
        let result = vm.run();
        assert!(matches!(
            result,
            Err(ExecutionError {
                kind: ExecutionErrorKind::Runtime(RuntimeError::IntegerOverflow {}),
                ..
            })
        ));
    }

    // ========================================================================
    // Modulo Tests (Euclidean)
    // ========================================================================

    #[test]
    fn test_int_modulo_basic() {
        use Instruction::*;

        let code = Code {
            constants: vec![],
            adapters: vec![],
            generic_adapters: vec![],
            instructions: vec![ConstInt(10), ConstInt(3), IntBinOp(b'%'), Return],
            num_locals: 0,
            max_stack_size: 2,
            lambdas: vec![],
        };
        let arena = Bump::new();
        let mut vm = VM::new(&arena, &code, Vec::new(), &[]);
        assert_eq!(vm.run().unwrap().as_int_unchecked(), 1);
    }

    #[test]
    fn test_int_modulo_exact_division() {
        use Instruction::*;

        let code = Code {
            constants: vec![],
            adapters: vec![],
            generic_adapters: vec![],
            instructions: vec![ConstInt(9), ConstInt(3), IntBinOp(b'%'), Return],
            num_locals: 0,
            max_stack_size: 2,
            lambdas: vec![],
        };
        let arena = Bump::new();
        let mut vm = VM::new(&arena, &code, Vec::new(), &[]);
        assert_eq!(vm.run().unwrap().as_int_unchecked(), 0);
    }

    #[test]
    fn test_int_modulo_euclidean_negative_dividend() {
        use Instruction::*;

        // Euclidean: -7 % 3 = 2 (always non-negative)
        // Truncated would give -1
        let code = Code {
            constants: vec![],
            adapters: vec![],
            generic_adapters: vec![],
            instructions: vec![ConstInt(-7), ConstInt(3), IntBinOp(b'%'), Return],
            num_locals: 0,
            max_stack_size: 2,
            lambdas: vec![],
        };
        let arena = Bump::new();
        let mut vm = VM::new(&arena, &code, Vec::new(), &[]);
        assert_eq!(vm.run().unwrap().as_int_unchecked(), 2);
    }

    #[test]
    fn test_int_modulo_euclidean_negative_divisor() {
        use Instruction::*;

        // Euclidean: 7 % -3 = 1 (always non-negative)
        let code = Code {
            constants: vec![],
            adapters: vec![],
            generic_adapters: vec![],
            instructions: vec![ConstInt(7), ConstInt(-3), IntBinOp(b'%'), Return],
            num_locals: 0,
            max_stack_size: 2,
            lambdas: vec![],
        };
        let arena = Bump::new();
        let mut vm = VM::new(&arena, &code, Vec::new(), &[]);
        assert_eq!(vm.run().unwrap().as_int_unchecked(), 1);
    }

    #[test]
    fn test_int_modulo_euclidean_both_negative() {
        use Instruction::*;

        // Euclidean: -7 % -3 = 2 (always non-negative)
        let code = Code {
            constants: vec![],
            adapters: vec![],
            generic_adapters: vec![],
            instructions: vec![ConstInt(-7), ConstInt(-3), IntBinOp(b'%'), Return],
            num_locals: 0,
            max_stack_size: 2,
            lambdas: vec![],
        };
        let arena = Bump::new();
        let mut vm = VM::new(&arena, &code, Vec::new(), &[]);
        assert_eq!(vm.run().unwrap().as_int_unchecked(), 2);
    }

    #[test]
    fn test_int_modulo_by_zero() {
        use Instruction::*;

        let code = Code {
            constants: vec![],
            adapters: vec![],
            generic_adapters: vec![],
            instructions: vec![ConstInt(10), ConstInt(0), IntBinOp(b'%'), Return],
            num_locals: 0,
            max_stack_size: 2,
            lambdas: vec![],
        };
        let arena = Bump::new();
        let mut vm = VM::new(&arena, &code, Vec::new(), &[]);
        let result = vm.run();
        assert!(matches!(
            result,
            Err(ExecutionError {
                kind: ExecutionErrorKind::Runtime(RuntimeError::DivisionByZero {}),
                ..
            })
        ));
    }

    #[test]
    fn test_int_modulo_i64_min_overflow() {
        use Instruction::*;

        // i64::MIN % -1 would overflow during computation
        // Use constants pool for i64::MIN since ConstInt only takes i8
        let code = Code {
            constants: vec![RawValue::make_int(i64::MIN)],
            adapters: vec![],
            generic_adapters: vec![],
            instructions: vec![ConstLoad(0), ConstInt(-1), IntBinOp(b'%'), Return],
            num_locals: 0,
            max_stack_size: 2,
            lambdas: vec![],
        };
        let arena = Bump::new();
        let mut vm = VM::new(&arena, &code, Vec::new(), &[]);
        let result = vm.run();
        assert!(matches!(
            result,
            Err(ExecutionError {
                kind: ExecutionErrorKind::Runtime(RuntimeError::IntegerOverflow {}),
                ..
            })
        ));
    }

    #[test]
    fn test_int_division_modulo_invariant() {
        use Instruction::*;

        // Verify the invariant: a == (a / b) * b + (a % b)
        // For -7 and 3: -7 == (-3) * 3 + 2 == -9 + 2 == -7 ✓
        let arena = Bump::new();

        // Get quotient: -7 / 3
        let code_div = Code {
            constants: vec![],
            adapters: vec![],
            generic_adapters: vec![],
            instructions: vec![ConstInt(-7), ConstInt(3), IntBinOp(b'/'), Return],
            num_locals: 0,
            max_stack_size: 2,
            lambdas: vec![],
        };
        let mut vm = VM::new(&arena, &code_div, Vec::new(), &[]);
        let quotient = vm.run().unwrap().as_int_unchecked();

        // Get remainder: -7 % 3
        let code_mod = Code {
            constants: vec![],
            adapters: vec![],
            generic_adapters: vec![],
            instructions: vec![ConstInt(-7), ConstInt(3), IntBinOp(b'%'), Return],
            num_locals: 0,
            max_stack_size: 2,
            lambdas: vec![],
        };
        let mut vm = VM::new(&arena, &code_mod, Vec::new(), &[]);
        let remainder = vm.run().unwrap().as_int_unchecked();

        // Verify: a == q * b + r
        assert_eq!(quotient * 3 + remainder, -7);
        assert_eq!(quotient, -3);
        assert_eq!(remainder, 2);
    }
}
