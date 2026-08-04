use alloc::boxed::Box;

use hashbrown::HashSet;

use crate::Vec;
use crate::types::Type;
use crate::values::RawValue;
use crate::vm::{FunctionAdapter, GenericAdapter, Instruction};

pub struct Code<'t> {
    pub constants: Vec<RawValue>,
    /// Function call adapters (specialized for performance).
    pub adapters: Vec<FunctionAdapter<'t>>,
    /// Generic adapters for other operations (Cast, `FormatStr`, etc.).
    pub generic_adapters: Vec<Box<dyn GenericAdapter + 't>>,
    pub instructions: Vec<Instruction>,
    pub num_locals: usize,
    pub max_stack_size: usize,
    /// Nested lambda bytecode (for closures).
    pub lambdas: Vec<LambdaCode<'t>>,
}

/// Bytecode for a lambda/closure, including its type and capture count.
pub struct LambdaCode<'t> {
    /// Function type for this lambda instantiation.
    pub lambda_type: &'t Type<'t>,
    /// Number of captured values from the enclosing scope.
    pub num_captures: u32,
    /// The kind of lambda (monomorphic or polymorphic).
    pub kind: LambdaKind<'t>,
}

/// Distinguishes between monomorphic and polymorphic lambdas.
#[derive(Debug)]
pub enum LambdaKind<'t> {
    /// A monomorphic lambda with a single concrete type and its compiled bytecode.
    Mono { code: Code<'t> },
    /// A polymorphic lambda with multiple monomorphized instantiations.
    /// Contains indices into `Code.lambdas` pointing to Mono entries.
    Poly { monos: Vec<u32> },
}

/// Extract jump offset from an instruction, if it's a jump instruction.
fn get_jump_offset(instr: &Instruction) -> Option<u8> {
    match instr {
        Instruction::JumpForward(offset)
        | Instruction::PopJumpIfFalse(offset)
        | Instruction::PopJumpIfTrue(offset)
        | Instruction::PushOtherwise(offset)
        | Instruction::PopOtherwiseAndJump(offset)
        | Instruction::MatchSomeOrJump(offset)
        | Instruction::MatchNoneOrJump(offset) => Some(*offset),
        _ => None,
    }
}

impl core::fmt::Debug for Code<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "Code {{")?;
        writeln!(f, "  num_locals: {}", self.num_locals)?;
        writeln!(f, "  max_stack_size: {}", self.max_stack_size)?;

        // Print constants pool
        if self.constants.is_empty() {
            writeln!(f, "  constants: []")?;
        } else {
            writeln!(f, "  constants: [")?;
            for (i, constant) in self.constants.iter().enumerate() {
                writeln!(f, "    [{i}] = {constant:?}")?;
            }
            writeln!(f, "  ]")?;
        }

        // First pass: collect all jump targets to determine which addresses need labels
        let mut jump_targets: HashSet<usize> = HashSet::new();
        let mut wide_arg: usize = 0;

        for (addr, instr) in self.instructions.iter().enumerate() {
            if let Instruction::WideArg(high) = instr {
                wide_arg = (wide_arg | (*high as usize)) << 8;
                continue;
            }

            if let Some(offset) = get_jump_offset(instr) {
                let full_offset = wide_arg | (offset as usize);
                // Jump is relative to NEXT instruction: target = addr + 1 + offset
                let target = addr + 1 + full_offset;
                jump_targets.insert(target);
            }
            wide_arg = 0;
        }

        // Assign label numbers to targets (sorted for deterministic output)
        let mut sorted_targets: Vec<_> = jump_targets.into_iter().collect();
        sorted_targets.sort_unstable();
        let label_map: hashbrown::HashMap<usize, usize> = sorted_targets
            .into_iter()
            .enumerate()
            .map(|(i, addr)| (addr, i))
            .collect();

        // Second pass: print instructions with labels
        writeln!(f, "  instructions:")?;
        wide_arg = 0;

        for (addr, instr) in self.instructions.iter().enumerate() {
            // Print label if this address is a jump target
            let label_prefix = if let Some(&label_num) = label_map.get(&addr) {
                alloc::format!("L{label_num}:")
            } else {
                alloc::string::String::new()
            };

            // Handle WideArg accumulation
            if let Instruction::WideArg(high) = instr {
                wide_arg = (wide_arg | (*high as usize)) << 8;
                writeln!(f, "    {addr:4} {label_prefix:>4}  {instr:?}")?;
                continue;
            }

            // Format jump instructions with target label
            if let Some(offset) = get_jump_offset(instr) {
                let full_offset = wide_arg | (offset as usize);
                let target = addr + 1 + full_offset;
                let target_label = label_map
                    .get(&target).map_or_else(|| alloc::format!("@{target}"), |l| alloc::format!("L{l}"));
                writeln!(
                    f,
                    "    {addr:4} {label_prefix:>4}  {instr:?} (to {target_label})"
                )?;
            } else if let Instruction::CallGenericAdapter(idx) = instr {
                // Show adapter name inline for CallGenericAdapter
                let full_idx = wide_arg | (*idx as usize);
                let adapter_name = self
                    .generic_adapters
                    .get(full_idx).map_or_else(|| alloc::string::String::from("???"), |a| a.name());
                writeln!(
                    f,
                    "    {addr:4} {label_prefix:>4}  {instr:?}  [ {adapter_name} ]"
                )?;
            } else {
                writeln!(f, "    {addr:4} {label_prefix:>4}  {instr:?}")?;
            }
            wide_arg = 0;
        }

        // Print nested lambdas
        if !self.lambdas.is_empty() {
            writeln!(f, "  lambdas:")?;
            for (i, lambda) in self.lambdas.iter().enumerate() {
                writeln!(f, "    [{i}] {lambda:?}")?;
            }
        }

        if !self.adapters.is_empty() {
            writeln!(f, "  adapters:")?;
            for (i, adapter) in self.adapters.iter().enumerate() {
                writeln!(f, "    [{}] {}", i, adapter.name())?;
            }
        }

        if !self.generic_adapters.is_empty() {
            writeln!(f, "  generic_adapters:")?;
            for (i, adapter) in self.generic_adapters.iter().enumerate() {
                writeln!(f, "    [{}] {}", i, adapter.name())?;
            }
        }

        write!(f, "}}")
    }
}

impl core::fmt::Debug for LambdaCode<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "LambdaCode {{")?;
        writeln!(f, "      type: {}", self.lambda_type)?;
        writeln!(f, "      num_captures: {}", self.num_captures)?;

        match &self.kind {
            LambdaKind::Mono { code } => {
                writeln!(f, "      kind: Mono")?;
                writeln!(f, "      num_locals: {}", code.num_locals)?;
                writeln!(f, "      max_stack_size: {}", code.max_stack_size)?;

                // Print constants pool
                if !code.constants.is_empty() {
                    writeln!(f, "      constants: [")?;
                    for (i, constant) in code.constants.iter().enumerate() {
                        writeln!(f, "        [{i}] = {constant:?}")?;
                    }
                    writeln!(f, "      ]")?;
                }

                // Print instructions (simplified - no label tracking for nested)
                writeln!(f, "      instructions:")?;
                let mut wide_arg: usize = 0;
                for (addr, instr) in code.instructions.iter().enumerate() {
                    if let Instruction::WideArg(high) = instr {
                        wide_arg = (wide_arg | (*high as usize)) << 8;
                        writeln!(f, "        {addr:4}  {instr:?}")?;
                        continue;
                    }

                    if let Instruction::CallGenericAdapter(idx) = instr {
                        let full_idx = wide_arg | (*idx as usize);
                        let adapter_name = code
                            .generic_adapters
                            .get(full_idx).map_or_else(|| alloc::string::String::from("???"), |a| a.name());
                        writeln!(f, "        {addr:4}  {instr:?}  [ {adapter_name} ]")?;
                    } else {
                        writeln!(f, "        {addr:4}  {instr:?}")?;
                    }
                    wide_arg = 0;
                }

                // Print nested lambdas recursively
                if !code.lambdas.is_empty() {
                    writeln!(f, "      lambdas:")?;
                    for (i, lambda) in code.lambdas.iter().enumerate() {
                        writeln!(f, "        [{i}] {lambda:?}")?;
                    }
                }
            }
            LambdaKind::Poly { monos } => {
                writeln!(f, "      kind: Poly {{ monos: {monos:?} }}")?;
            }
        }

        write!(f, "    }}")
    }
}
