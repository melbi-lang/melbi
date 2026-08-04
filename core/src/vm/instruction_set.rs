//! Melbi VM Instructions - Fixed 16-bit Format
//!
//! This module defines the instruction set for Melbi's stack-based virtual machine.
//!
//! # Instruction Format
//!
//! **ALL instructions are exactly 16 bits (2 bytes)**:
//! ```text
//! ┌────────────┬────────────┐
//! │    Tag     │  Operand   │
//! │  (8 bits)  │  (8 bits)  │
//! └────────────┴────────────┘
//! ```
//!
//! Using `#[repr(C, u8)]`, the enum naturally maps to this 2-byte layout:
//! - Discriminant (tag) = 1 byte
//! - Payload (operand) = 0 or 1 byte
//! - Total size = 2 bytes with no padding
//!
//! # Design Principles
//!
//! - **Stack-based**: All operations consume operands from stack and push results
//! - **Fixed-width**: Every instruction is exactly 16 bits (simpler VM, better performance)
//! - **Type-explicit**: Separate instructions for Int vs Float operations (no runtime dispatch)
//! - **Arena-allocated**: All heap values allocated in bump allocator
//! - **Parameterized ops**: Binary operations use operand to encode operation (saves opcodes)
//!
//! # Wide Arguments
//!
//! For values > 255, use the `WideArg` prefix:
//! ```ignore
//! WideArg(high_byte)      // Sets high byte for next instruction
//! ConstLoad(low_byte)     // Combined: (high << 8) | low = 16-bit index
//! ```
//!
//! # Stack Discipline
//!
//! Stack effect notation: `[..., operand1, operand2] -> [..., result]`

use core::fmt;

use crate::parser::ComparisonOp;

/// A single VM instruction (exactly 16 bits)
///
/// The `#[repr(C, u8)]` ensures:
/// - First byte is the discriminant (opcode)
/// - Second byte is the operand (if present)
/// - Total size is exactly 2 bytes
/// - Can be safely transmuted to/from `[u8; 2]`
#[repr(C, u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Instruction {
    // ========================================================================
    // Special (0x00)
    // ========================================================================
    /// Halt execution
    ///
    /// Having Halt at 0x00 is a safety feature:
    /// - Uninitialized memory (zeros) will halt instead of executing garbage
    /// - Null-terminated C strings won't accidentally execute
    /// - Better fail-safe behavior on bytecode corruption
    Halt = 0x00,

    // ========================================================================
    // Stack & Constants (0x01 - 0x0F)
    // ========================================================================
    /// Push constant from pool (index 0-255)
    /// Operand: u8 index | Stack: [...] -> [..., value]
    ConstLoad(u8) = 0x01,

    /// Push small signed integer (-128 to 127)
    /// Operand: i8 value | Stack: [...] -> [..., int]
    /// Does not support `WideArg`.
    ConstInt(i8) = 0x02,

    /// Push unsigned byte (0 to 255)
    /// Operand: u8 value | Stack: [...] -> [..., int]
    /// Does not support `WideArg`.
    ConstUInt(u8) = 0x03,

    /// Push the Bool value (arg != 0)
    /// Stack: [...] -> [..., arg != 0]
    /// Does not support `WideArg`.
    ConstBool(u8) = 0x04,

    /// Wide argument prefix - modifies next instruction's operand
    ///
    /// The next instruction will use a 16-bit operand:
    /// `(this_operand << 8) | next_operand`
    ///
    /// Example:
    /// ```ignore
    /// WideArg(0x03)       // High byte
    /// ConstLoad(0xE8)     // Low byte -> loads constant 1000 (0x03E8)
    /// ```
    WideArg(u8) = 0x05,

    /// Duplicate value at depth N (top is N=0)
    /// Operand: u8 depth | Stack: [..., aN, ...] -> [..., aN, ..., aN]
    /// Does not support `WideArg`.
    DupN(u8) = 0x07,

    /// Pop top value
    /// Stack: [..., a] -> [...]
    Pop = 0x08,

    /// Swap top two values
    /// Stack: [..., a, b] -> [..., b, a]
    Swap = 0x09,

    /// Load local variable
    /// Operand: u8 index | Stack: [...] -> [..., value]
    LoadLocal(u8) = 0x0A,

    /// Store to local variable
    /// Operand: u8 index | Stack: [..., value] -> [...]
    StoreLocal(u8) = 0x0B,

    /// Load captured variable from closure
    /// Operand: u8 index | Stack: [...] -> [..., value]
    ///
    /// Captures are variables copied from enclosing scopes at closure creation.
    /// Each closure instance has its own captures array.
    /// This is how closures "remember" their environment.
    ///
    /// Note: Unlike Python (which captures by reference and is broken),
    /// we capture by value at closure creation time. Each closure gets
    /// its own snapshot of captured variables.
    LoadCapture(u8) = 0x0C,

    // 0x0D reserved (was StoreUpvalue, removed - captures are immutable)

    // 0x0F reserved

    // ========================================================================
    // Arithmetic - Integer (0x10 - 0x1F)
    // ========================================================================
    /// Integer binary operation
    ///
    /// Operand encodes the operation:
    /// - `b'+'` (0x2B): Addition
    /// - `b'-'` (0x2D): Subtraction
    /// - `b'*'` (0x2A): Multiplication
    /// - `b'/'` (0x2F): Division (can error)
    /// - `b'%'` (0x25): Modulo (can error)
    /// - `b'^'` (0x5E): Power (can error)
    ///
    /// Stack: [..., a: Int, b: Int] -> [..., result: Int(|!)]
    IntBinOp(u8) = 0x10,

    /// Integer unary negation: -a
    /// Stack: [..., a: Int] -> [..., -a: Int]
    NegInt = 0x11,

    /// Integer comparison operation
    ///
    /// Stack: [..., a: Int, b: Int] -> [..., result: Bool]
    IntCmpOp(ComparisonOp) = 0x14,

    // 0x15-0x1F reserved for future int operations

    // ========================================================================
    // Arithmetic - Float (0x20 - 0x2F)
    // ========================================================================
    /// Float binary operation
    ///
    /// Same operand encoding as `IntBinOp`:
    /// - `b'+'`: Addition
    /// - `b'-'`: Subtraction
    /// - `b'*'`: Multiplication
    /// - `b'/'`: Division
    /// - `b'^'`: Power
    ///
    /// Stack: [..., a: Float, b: Float] -> [..., result: Float]
    FloatBinOp(u8) = 0x20,

    /// Float unary negation: -a
    /// Stack: [..., a: Float] -> [..., -a: Float]
    NegFloat = 0x21,

    /// Float comparison operation
    ///
    /// Stack: [..., a: Float, b: Float] -> [..., result: Bool]
    FloatCmpOp(ComparisonOp) = 0x22,

    // 0x23-0x2F reserved for future float operations

    // ========================================================================
    // Logical Operations (0x30 - 0x37)
    // ========================================================================
    /// Logical AND: a && b
    /// Stack: [..., a: Bool, b: Bool] -> [..., a&&b: Bool]
    And = 0x30,

    /// Logical OR: a || b
    /// Stack: [..., a: Bool, b: Bool] -> [..., a||b: Bool]
    Or = 0x31,

    /// Logical NOT: !a
    /// Stack: [..., a: Bool] -> [..., !a: Bool]
    Not = 0x32,

    /// Boolean equality
    /// Stack: [..., a: Bool, b: Bool] -> [..., a==b: Bool]
    EqBool = 0x33,

    // 0x34-0x37 reserved

    // ========================================================================
    // Control Flow (0x38 - 0x4F)
    // ========================================================================
    /// Unconditional jump (unsigned byte offset in instructions, not bytes)
    ///
    /// Operand: u8 offset (in instructions)
    /// Stack: [...] -> [...]
    ///
    /// Jump is relative to the NEXT instruction.
    /// Offset is in instruction count, not bytes (each instruction is 2 bytes).
    ///
    /// Example: `Jump(3)` skips forward 3 instructions (6 bytes)
    JumpForward(u8) = 0x38,

    /// Pop and Jump if false
    /// Operand: u8 offset | Stack: [..., cond: Bool] -> [...]
    PopJumpIfFalse(u8) = 0x39,

    /// Pop and Jump if true
    /// Operand: u8 offset | Stack: [..., cond: Bool] -> [...]
    PopJumpIfTrue(u8) = 0x3A,

    /// Return from function
    /// Stack: [..., retval] -> [retval]
    Return = 0x3E,

    /// Call function
    /// Operand: u8 index to adapter | Stack: [..., args..., func] -> [..., result]
    Call(u8) = 0x3F,

    /// Push otherwise error handler
    /// Operand: u8 offset (forward) to fallback code
    /// Pushes `OtherwiseBlock` { fallback: ip + offset, `stack_size`: `current_stack_size` } to `otherwise_stack`
    PushOtherwise(u8) = 0x42,

    /// Pop otherwise error handler (normal cleanup)
    /// Pops the top `OtherwiseBlock` from `otherwise_stack`
    /// Used in fallback code to clean up handler
    PopOtherwise = 0x43,

    /// Pop otherwise handler and jump (success case)
    /// Operand: u8 offset (forward) to done label
    /// Pops `OtherwiseBlock` and jumps past fallback code
    /// Used when primary expression succeeds
    PopOtherwiseAndJump(u8) = 0x44,

    // 0x45-0x4F reserved for control flow

    // ========================================================================
    // Function & Closure Operations (0x50 - 0x5F)
    // ========================================================================
    /// Create closure
    ///
    /// Operand: u8 function index in constant pool
    /// Stack: [..., upval1, upval2, ..., upvalN] -> [..., closure]
    ///
    /// Creates a closure by:
    /// 1. Loading function descriptor from constant pool
    /// 2. Popping N upvalues from stack (N specified in function descriptor)
    /// 3. Creating closure object with function + captured upvalues
    /// 4. Pushing closure object onto stack
    ///
    /// The number of upvalues is stored in the `FunctionConstant`.
    MakeClosure(u8) = 0x50,

    // 0x51-0x5F reserved for function operations

    // ========================================================================
    // Array Operations (0x60 - 0x6F)
    // ========================================================================
    /// Make array with N elements
    /// Operand: u8 count | Stack: [..., e1, ..., eN] -> [..., array]
    MakeArray(u8) = 0x60,

    /// Get array length
    /// Stack: [..., array: Array[T]] -> [..., len: Int]
    ArrayLen = 0x61,

    /// Get array element
    /// Stack: [..., array: Array[T], index: Int] -> [..., elem: T!]
    ArrayGet = 0x62,

    /// Get array element at constant index
    /// Operand: u8 index | Stack: [..., array: Array[T]] -> [..., elem: T!]
    ArrayGetConst(u8) = 0x63,

    /// Concatenate two arrays
    /// Stack: [..., a1: Array[T], a2: Array[T]] -> [..., result: Array[T]]
    ArrayConcat = 0x64,

    /// Slice array
    /// Stack: [..., arr: Array[T], start: Int, end: Int] -> [..., slice: Array[T]!]
    ArraySlice = 0x65,

    /// Append element to array (creates new array)
    /// Stack: [..., array: Array[T], elem: T] -> [..., `new_array`: Array[T]]
    ArrayAppend = 0x66,

    // 0x67-0x6F reserved for array operations

    // ========================================================================
    // Map Operations (0x70 - 0x7F)
    // ========================================================================
    /// Make map with N key-value pairs
    /// Operand: u8 count | Stack: [..., k1, v1, ..., kN, vN] -> [..., map]
    MakeMap(u8) = 0x70,

    /// Get map size
    /// Stack: [..., map: Map[K,V]] -> [..., size: Int]
    MapLen = 0x71,

    /// Get value from map
    /// Stack: [..., map: Map[K,V], key: K] -> [..., value: V!]
    MapGet = 0x72,

    /// Check if key exists
    /// Stack: [..., map: Map[K,V], key: K] -> [..., exists: Bool]
    MapHas = 0x73,

    /// Insert key-value (creates new map)
    /// Stack: [..., map: Map[K,V], key: K, val: V] -> [..., `new_map`: Map[K,V]]
    MapInsert = 0x74,

    /// Remove key (creates new map)
    /// Stack: [..., map: Map[K,V], key: K] -> [..., `new_map`: Map[K,V]]
    MapRemove = 0x75,

    /// Get all keys as array
    /// Stack: [..., map: Map[K,V]] -> [..., keys: Array[K]]
    MapKeys = 0x76,

    /// Get all values as array
    /// Stack: [..., map: Map[K,V]] -> [..., values: Array[V]]
    MapValues = 0x77,

    // 0x78-0x7F reserved for map operations

    // ========================================================================
    // Record Operations (0x80 - 0x8F)
    // ========================================================================
    /// Make record (type descriptor in constant pool)
    /// Operand: u8 type index | Stack: [..., f1, ..., fN] -> [..., record]
    MakeRecord(u8) = 0x80,

    /// Get field by index
    /// Operand: u8 field index | Stack: [..., record] -> [..., value!]
    RecordGet(u8) = 0x81,

    /// Merge two records
    /// Stack: [..., rec1, rec2] -> [..., merged]
    RecordMerge = 0x83,

    // 0x84-0x8F reserved for record operations

    // ========================================================================
    // String Operations (0x90 - 0x9F)
    // ========================================================================
    /// String operations

    /// Format string (f-string)
    /// Operand: u8 arg count | Stack: [..., args..., template] -> [..., result]
    StringFormat(u8) = 0x98,

    /// String comparison operation (lexicographic)
    ///
    /// Stack: [..., a: String, b: String] -> [..., result: Bool]
    StringCmpOp(ComparisonOp) = 0x99,

    // 0x9A-0x9F reserved for string operations

    // ========================================================================
    // Bytes Operations (0xA0 - 0xAF)
    // ========================================================================
    /// Bytes operations

    /// Get byte at index
    /// Stack: [..., bytes: Bytes, index: Int] -> [..., byte: Int!]
    BytesGet = 0xA2,

    /// Get byte at constant index
    /// Operand: u8 index | Stack: [..., bytes: Bytes] -> [..., byte: Int!]
    BytesGetConst(u8) = 0xA3,

    /// Slice bytes
    /// Stack: [..., bytes: Bytes, start: Int, end: Int] -> [..., slice: Bytes!]
    BytesSlice = 0xA4,

    /// String to bytes (UTF-8 encode)
    /// Stack: [..., str: String] -> [..., bytes: Bytes]
    StringToBytes = 0xA5,

    /// Bytes to string (UTF-8 decode)
    /// Stack: [..., bytes: Bytes] -> [..., str: String!]
    BytesToString = 0xA6,

    /// Bytes comparison (lexicographic)
    /// Stack: [..., a: Bytes, b: Bytes] -> [..., result: Bool]
    BytesCmpOp(ComparisonOp) = 0xA7,

    // 0xA8-0xAF reserved for bytes operations

    // ========================================================================
    // Type & Error Operations (0xB0 - 0xBF)
    // ========================================================================
    /// Call a generic adapter (Cast, `FormatStr`, etc.)
    /// Operand: u8 adapter index | Stack: [..., args...] -> [..., result(!)]
    ///
    /// The number of args popped depends on the adapter's `num_args()`.
    /// This instruction can error depending on the adapter implementation.
    CallGenericAdapter(u8) = 0xB0,

    /// Generic equality (structural comparison for arrays, maps, records)
    /// Stack: [..., a: T, b: T] -> [..., a==b: Bool]
    Eq = 0xB5,

    /// Generic inequality
    /// Stack: [..., a: T, b: T] -> [..., a!=b: Bool]
    NotEq = 0xB6,

    /// Make Option value
    /// Operand: 0 for none, 1 for some
    /// - MakeOption(0): Stack: [...] -> [..., none]
    /// - MakeOption(1): Stack: [..., value] -> [..., some(value)]
    MakeOption(u8) = 0xB7,

    // 0xB8-0xBF reserved

    // ========================================================================
    // Pattern Matching (0xC0 - 0xCF)
    // ========================================================================
    /// Match Some pattern: if Option is Some, extract inner value and fall through;
    /// if None, jump forward by offset.
    ///
    /// Operand: u8 offset (forward jump if None)
    /// Stack on Some: [..., option] -> [..., `inner_value`] (falls through)
    /// Stack on None: [..., option] -> [...] (jumps forward)
    MatchSomeOrJump(u8) = 0xC0,

    /// Match None pattern: if Option is None, fall through;
    /// if Some, jump forward by offset.
    ///
    /// Operand: u8 offset (forward jump if Some)
    /// Stack on None: [..., option] -> [...] (falls through)
    /// Stack on Some: [..., option] -> [...] (jumps forward)
    MatchNoneOrJump(u8) = 0xC1,

    // 0xC2-0xCF reserved for pattern matching

    // ========================================================================
    // Meta & Debug Operations (0xD0 - 0xDF)
    // ========================================================================
    /// No operation
    Nop = 0xD0,

    /// Breakpoint for debugger
    /// Operand: u8 breakpoint ID
    Breakpoint(u8) = 0xD1,

    /// Check execution limits (sandboxing)
    CheckLimits = 0xD2,

    /// Trace execution (profiling)
    /// Operand: u8 trace point ID
    Trace(u8) = 0xD3,

    /// Inline cache hint
    /// Operand: u8 cache ID
    InlineCache(u8) = 0xD4,
    // 0xD5-0xDF reserved

    // 0xE0-0xFF reserved for future expansion
}
static_assertions::assert_eq_size!(Instruction, [u8; 2]);

impl Instruction {
    /// Size of an instruction in bytes
    pub const SIZE: usize = 2;
}

impl fmt::Debug for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Binary operations - show operator as char
            Self::IntBinOp(op) => write!(f, "IntBinOp({})", *op as char),
            Self::FloatBinOp(op) => write!(f, "FloatBinOp({})", *op as char),

            // Comparisons - use ComparisonOp's Debug
            Self::IntCmpOp(op) => write!(f, "IntCmpOp({op:?})"),
            Self::FloatCmpOp(op) => write!(f, "FloatCmpOp({op:?})"),
            Self::StringCmpOp(op) => write!(f, "StringCmpOp({op:?})"),
            Self::BytesCmpOp(op) => write!(f, "BytesCmpOp({op:?})"),

            // Default formatting for everything else
            Self::Halt => write!(f, "Halt"),
            Self::ConstLoad(idx) => write!(f, "ConstLoad({idx})"),
            Self::ConstInt(val) => write!(f, "ConstInt({val})"),
            Self::ConstUInt(val) => write!(f, "ConstUInt({val})"),
            Self::ConstBool(val) => write!(f, "ConstBool({val})"),
            Self::WideArg(high) => write!(f, "WideArg(0x{high:02X})"),
            Self::DupN(depth) => write!(f, "DupN({depth})"),
            Self::Pop => write!(f, "Pop"),
            Self::Swap => write!(f, "Swap"),
            Self::LoadLocal(idx) => write!(f, "LoadLocal({idx})"),
            Self::StoreLocal(idx) => write!(f, "StoreLocal({idx})"),
            Self::LoadCapture(idx) => write!(f, "LoadCapture({idx})"),
            Self::NegInt => write!(f, "NegInt"),
            Self::NegFloat => write!(f, "NegFloat"),
            Self::And => write!(f, "And"),
            Self::Or => write!(f, "Or"),
            Self::Not => write!(f, "Not"),
            Self::EqBool => write!(f, "EqBool"),
            Self::JumpForward(offset) => write!(f, "JumpForward({offset})"),
            Self::PopJumpIfFalse(offset) => write!(f, "{:18} {}", "PopJumpIfFalse", offset),
            Self::PopJumpIfTrue(offset) => write!(f, "{:18} {}", "PopJumpIfTrue", offset),
            Self::Return => write!(f, "Return"),
            Self::Call(argc) => write!(f, "Call({argc})"),
            Self::MakeClosure(idx) => write!(f, "MakeClosure({idx})"),
            Self::MakeArray(count) => write!(f, "MakeArray({count})"),
            Self::ArrayLen => write!(f, "ArrayLen"),
            Self::ArrayGet => write!(f, "ArrayGet"),
            Self::ArrayGetConst(idx) => write!(f, "ArrayGetConst({idx})"),
            Self::ArrayConcat => write!(f, "ArrayConcat"),
            Self::ArraySlice => write!(f, "ArraySlice"),
            Self::ArrayAppend => write!(f, "ArrayAppend"),
            Self::MakeMap(count) => write!(f, "MakeMap({count})"),
            Self::MapLen => write!(f, "MapLen"),
            Self::MapGet => write!(f, "MapGet"),
            Self::MapHas => write!(f, "MapHas"),
            Self::MapInsert => write!(f, "MapInsert"),
            Self::MapRemove => write!(f, "MapRemove"),
            Self::MapKeys => write!(f, "MapKeys"),
            Self::MapValues => write!(f, "MapValues"),
            Self::MakeRecord(ty_idx) => write!(f, "MakeRecord({ty_idx})"),
            Self::RecordGet(idx) => write!(f, "RecordGet({idx})"),
            Self::RecordMerge => write!(f, "RecordMerge"),
            Self::StringFormat(argc) => write!(f, "StringFormat({argc})"),
            Self::BytesGet => write!(f, "BytesGet"),
            Self::BytesGetConst(idx) => write!(f, "BytesGetConst({idx})"),
            Self::BytesSlice => write!(f, "BytesSlice"),
            Self::StringToBytes => write!(f, "StringToBytes"),
            Self::BytesToString => write!(f, "BytesToString"),
            Self::CallGenericAdapter(idx) => write!(f, "CallGenericAdapter({idx})"),
            Self::Eq => write!(f, "Eq"),
            Self::NotEq => write!(f, "NotEq"),
            Self::MatchSomeOrJump(offset) => write!(f, "MatchSomeOrJump({offset})"),
            Self::MatchNoneOrJump(offset) => write!(f, "MatchNoneOrJump({offset})"),
            Self::Nop => write!(f, "Nop"),
            Self::Breakpoint(id) => write!(f, "Breakpoint({id})"),
            Self::CheckLimits => write!(f, "CheckLimits"),
            Self::Trace(id) => write!(f, "Trace({id})"),
            Self::InlineCache(id) => write!(f, "InlineCache({id})"),
            Self::PushOtherwise(offset) => write!(f, "PushOtherwise({offset:+})"),
            Self::PopOtherwise => write!(f, "PopOtherwise"),
            Self::PopOtherwiseAndJump(offset) => write!(f, "PopOtherwiseAndJump({offset:+})"),
            Self::MakeOption(0) => write!(f, "MakeOption(none)"),
            Self::MakeOption(1) => write!(f, "MakeOption(some)"),
            Self::MakeOption(n) => write!(f, "MakeOption({n})"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instruction_size() {
        // Critical: instructions must be exactly 2 bytes
        assert_eq!(core::mem::size_of::<Instruction>(), 2);
        assert_eq!(Instruction::SIZE, 2);
    }

    #[test]
    fn instruction_alignment() {
        // Should have alignment of 1 (no padding)
        assert_eq!(core::mem::align_of::<Instruction>(), 1);
    }

    #[test]
    fn parameterized_ops() {
        // Test that parameterized ops work correctly
        let add = Instruction::IntBinOp(b'+');
        let sub = Instruction::IntBinOp(b'-');
        assert_ne!(add, sub);

        let lt = Instruction::IntCmpOp(ComparisonOp::Lt);
        let gt = Instruction::IntCmpOp(ComparisonOp::Gt);
        assert_ne!(lt, gt);
    }

    #[test]
    fn debug_formatting() {
        let inst = Instruction::IntBinOp(b'+');
        let debug = format!("{inst:?}");
        assert_eq!(debug, "IntBinOp(+)");

        let cmp = Instruction::IntCmpOp(ComparisonOp::Lt);
        let debug = format!("{cmp:?}");
        assert_eq!(debug, "IntCmpOp(Lt)");
    }
}
