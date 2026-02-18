//! Opcode definitions for YaoXiang bytecode
//!
//! Unified operation codes used across all backends.
//! This is the abstract representation, distinct from TypedOpcode
//! which is the encoded format for the VM.

use std::fmt;

/// Bytecode operation code
///
/// Represents semantic operations without encoding details.
/// Each variant corresponds to a logical operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Opcode {
    // =====================
    // Control Flow (0x00-0x1F)
    // =====================
    /// No-op
    Nop = 0x00,

    /// Return without value
    Return = 0x01,

    /// Return with value
    ReturnValue = 0x02,

    /// Unconditional jump
    Jmp = 0x03,

    /// Conditional jump (if true)
    JmpIf = 0x04,

    /// Conditional jump (if false)
    JmpIfNot = 0x05,

    /// Switch/case dispatch
    Switch = 0x06,

    /// Loop start (iterator elimination)
    LoopStart = 0x07,

    /// Loop increment
    LoopInc = 0x08,

    /// Tail call (TCO)
    TailCall = 0x09,

    /// Yield (async scheduling)
    Yield = 0x0A,

    /// Label definition
    Label = 0x0B,

    // =====================
    // Register Operations (0x10-0x1F)
    // =====================
    /// Register move
    Mov = 0x10,

    /// Load constant
    LoadConst = 0x11,

    /// Load local variable
    LoadLocal = 0x12,

    /// Store local variable
    StoreLocal = 0x13,

    /// Load function argument
    LoadArg = 0x14,

    // =====================
    // Integer Operations (0x20-0x3F)
    // =====================
    /// I64 add
    I64Add = 0x20,
    I64Sub = 0x21,
    I64Mul = 0x22,
    I64Div = 0x23,
    I64Rem = 0x24,
    I64And = 0x25,
    I64Or = 0x26,
    I64Xor = 0x27,
    I64Shl = 0x28,
    I64Sar = 0x29,
    I64Shr = 0x2A,
    I64Neg = 0x2B,

    /// I32 operations
    I32Add = 0x30,
    I32Sub = 0x31,
    I32Mul = 0x32,
    I32Div = 0x33,
    I32Rem = 0x34,
    I32And = 0x35,
    I32Or = 0x36,
    I32Xor = 0x37,
    I32Shl = 0x38,
    I32Sar = 0x39,
    I32Shr = 0x3A,
    I32Neg = 0x3B,

    /// I64 constant load
    I64Const = 0x2E,

    /// I32 constant load
    I32Const = 0x3E,

    // =====================
    // Float Operations (0x40-0x5F)
    // =====================
    /// F64 operations
    F64Add = 0x40,
    F64Sub = 0x41,
    F64Mul = 0x42,
    F64Div = 0x43,
    F64Rem = 0x44,
    F64Sqrt = 0x45,
    F64Neg = 0x46,

    /// F64 constant load
    F64Const = 0x49,

    /// F32 operations
    F32Add = 0x50,
    F32Sub = 0x51,
    F32Mul = 0x52,
    F32Div = 0x53,
    F32Rem = 0x54,
    F32Sqrt = 0x55,
    F32Neg = 0x56,

    /// F32 constant load
    F32Const = 0x59,

    // =====================
    // Comparison Operations (0x60-0x7F)
    // =====================
    /// I64 comparisons
    I64Eq = 0x60,
    I64Ne = 0x61,
    I64Lt = 0x62,
    I64Le = 0x63,
    I64Gt = 0x64,
    I64Ge = 0x65,

    /// F64 comparisons
    F64Eq = 0x66,
    F64Ne = 0x67,
    F64Lt = 0x68,
    F64Le = 0x69,
    F64Gt = 0x6A,
    F64Ge = 0x6B,

    /// F32 comparisons
    F32Eq = 0x6C,
    F32Ne = 0x6D,
    F32Lt = 0x6E,
    F32Le = 0x6F,
    F32Gt = 0x70,
    F32Ge = 0x71,

    // =====================
    // Memory & Object Operations (0x72-0x7F)
    // =====================
    /// Stack allocation
    StackAlloc = 0x73,

    /// Heap allocation
    HeapAlloc = 0x72,

    /// Drop value
    Drop = 0x74,

    /// Get struct field
    GetField = 0x75,

    /// Set struct field
    SetField = 0x76,

    /// Load element from array/list
    LoadElement = 0x77,

    /// Store element to array/list
    StoreElement = 0x78,

    /// List with capacity
    NewListWithCap = 0x7A,

    /// Create struct instance
    CreateStruct = 0x79,

    /// Arc operations
    ArcNew = 0x7B,
    ArcClone = 0x7C,
    ArcDrop = 0x7D,
    /// Weak reference operations
    WeakNew = 0x7E,
    WeakUpgrade = 0x7F,

    // =====================
    // Function Call (0x80-0x8F)
    // =====================
    /// Static dispatch call
    CallStatic = 0x80,

    /// Virtual dispatch call
    CallVirt = 0x81,

    /// Dynamic dispatch call
    CallDyn = 0x82,

    /// Create closure
    MakeClosure = 0x83,

    /// Load upvalue
    LoadUpvalue = 0x84,

    /// Store upvalue
    StoreUpvalue = 0x85,

    /// Close upvalue
    CloseUpvalue = 0x86,

    /// Native function call (FFI)
    CallNative = 0x87,

    // =====================
    // String Operations (0x90-0x9F)
    // =====================
    StringLength = 0x90,
    StringConcat = 0x91,
    StringEqual = 0x92,
    StringGetChar = 0x93,
    StringFromInt = 0x94,
    StringFromFloat = 0x95,

    // =====================
    // Exception Handling (0xA0-0xAF)
    // =====================
    TryBegin = 0xA0,
    TryEnd = 0xA1,
    Throw = 0xA2,
    Rethrow = 0xA3,

    // =====================
    // Debug Operations (0xB0-0xBF)
    // =====================
    BoundsCheck = 0xB0,

    // =====================
    // Type Operations (0xC0-0xCF)
    // =====================
    TypeCheck = 0xC0,
    Cast = 0xC1,

    // =====================
    // Reflection (0xD0-0xDF)
    // =====================
    TypeOf = 0xD0,

    // =====================
    // Reserved (0xE0-0xFF)
    // =====================
    Custom0 = 0xE0,
    Custom1 = 0xE1,
    Custom2 = 0xE2,
    Custom3 = 0xE3,
    Invalid = 0xFF,
}

impl Opcode {
    /// Get instruction name
    pub fn name(&self) -> &'static str {
        match self {
            Opcode::Nop => "Nop",
            Opcode::Return => "Return",
            Opcode::ReturnValue => "ReturnValue",
            Opcode::Jmp => "Jmp",
            Opcode::JmpIf => "JmpIf",
            Opcode::JmpIfNot => "JmpIfNot",
            Opcode::Switch => "Switch",
            Opcode::LoopStart => "LoopStart",
            Opcode::LoopInc => "LoopInc",
            Opcode::TailCall => "TailCall",
            Opcode::Yield => "Yield",
            Opcode::Label => "Label",
            Opcode::Mov => "Mov",
            Opcode::LoadConst => "LoadConst",
            Opcode::LoadLocal => "LoadLocal",
            Opcode::StoreLocal => "StoreLocal",
            Opcode::LoadArg => "LoadArg",
            Opcode::I64Add => "I64Add",
            Opcode::I64Sub => "I64Sub",
            Opcode::I64Mul => "I64Mul",
            Opcode::I64Div => "I64Div",
            Opcode::I64Rem => "I64Rem",
            Opcode::I64And => "I64And",
            Opcode::I64Or => "I64Or",
            Opcode::I64Xor => "I64Xor",
            Opcode::I64Shl => "I64Shl",
            Opcode::I64Sar => "I64Sar",
            Opcode::I64Shr => "I64Shr",
            Opcode::I64Neg => "I64Neg",
            Opcode::I32Add => "I32Add",
            Opcode::I32Sub => "I32Sub",
            Opcode::I32Mul => "I32Mul",
            Opcode::I32Div => "I32Div",
            Opcode::I32Rem => "I32Rem",
            Opcode::I32And => "I32And",
            Opcode::I32Or => "I32Or",
            Opcode::I32Xor => "I32Xor",
            Opcode::I32Shl => "I32Shl",
            Opcode::I32Sar => "I32Sar",
            Opcode::I32Shr => "I32Shr",
            Opcode::I32Neg => "I32Neg",
            Opcode::I64Const => "I64Const",
            Opcode::I32Const => "I32Const",
            Opcode::F64Add => "F64Add",
            Opcode::F64Sub => "F64Sub",
            Opcode::F64Mul => "F64Mul",
            Opcode::F64Div => "F64Div",
            Opcode::F64Rem => "F64Rem",
            Opcode::F64Sqrt => "F64Sqrt",
            Opcode::F64Neg => "F64Neg",
            Opcode::F64Const => "F64Const",
            Opcode::F32Add => "F32Add",
            Opcode::F32Sub => "F32Sub",
            Opcode::F32Mul => "F32Mul",
            Opcode::F32Div => "F32Div",
            Opcode::F32Rem => "F32Rem",
            Opcode::F32Sqrt => "F32Sqrt",
            Opcode::F32Neg => "F32Neg",
            Opcode::F32Const => "F32Const",
            Opcode::I64Eq => "I64Eq",
            Opcode::I64Ne => "I64Ne",
            Opcode::I64Lt => "I64Lt",
            Opcode::I64Le => "I64Le",
            Opcode::I64Gt => "I64Gt",
            Opcode::I64Ge => "I64Ge",
            Opcode::F64Eq => "F64Eq",
            Opcode::F64Ne => "F64Ne",
            Opcode::F64Lt => "F64Lt",
            Opcode::F64Le => "F64Le",
            Opcode::F64Gt => "F64Gt",
            Opcode::F64Ge => "F64Ge",
            Opcode::F32Eq => "F32Eq",
            Opcode::F32Ne => "F32Ne",
            Opcode::F32Lt => "F32Lt",
            Opcode::F32Le => "F32Le",
            Opcode::F32Gt => "F32Gt",
            Opcode::F32Ge => "F32Ge",
            Opcode::StackAlloc => "StackAlloc",
            Opcode::HeapAlloc => "HeapAlloc",
            Opcode::Drop => "Drop",
            Opcode::GetField => "GetField",
            Opcode::SetField => "SetField",
            Opcode::LoadElement => "LoadElement",
            Opcode::StoreElement => "StoreElement",
            Opcode::NewListWithCap => "NewListWithCap",
            Opcode::CreateStruct => "CreateStruct",
            Opcode::ArcNew => "ArcNew",
            Opcode::ArcClone => "ArcClone",
            Opcode::ArcDrop => "ArcDrop",
            Opcode::WeakNew => "WeakNew",
            Opcode::WeakUpgrade => "WeakUpgrade",
            Opcode::CallStatic => "CallStatic",
            Opcode::CallVirt => "CallVirt",
            Opcode::CallDyn => "CallDyn",
            Opcode::MakeClosure => "MakeClosure",
            Opcode::LoadUpvalue => "LoadUpvalue",
            Opcode::StoreUpvalue => "StoreUpvalue",
            Opcode::CloseUpvalue => "CloseUpvalue",
            Opcode::CallNative => "CallNative",
            Opcode::StringLength => "StringLength",
            Opcode::StringConcat => "StringConcat",
            Opcode::StringEqual => "StringEqual",
            Opcode::StringGetChar => "StringGetChar",
            Opcode::StringFromInt => "StringFromInt",
            Opcode::StringFromFloat => "StringFromFloat",
            Opcode::TryBegin => "TryBegin",
            Opcode::TryEnd => "TryEnd",
            Opcode::Throw => "Throw",
            Opcode::Rethrow => "Rethrow",
            Opcode::BoundsCheck => "BoundsCheck",
            Opcode::TypeCheck => "TypeCheck",
            Opcode::Cast => "Cast",
            Opcode::TypeOf => "TypeOf",
            Opcode::Custom0 => "Custom0",
            Opcode::Custom1 => "Custom1",
            Opcode::Custom2 => "Custom2",
            Opcode::Custom3 => "Custom3",
            Opcode::Invalid => "Invalid",
        }
    }

    /// Check if this is a numeric operation
    pub fn is_numeric_op(&self) -> bool {
        matches!(
            self,
            Opcode::I64Add
                | Opcode::I64Sub
                | Opcode::I64Mul
                | Opcode::I64Div
                | Opcode::I64Rem
                | Opcode::I64And
                | Opcode::I64Or
                | Opcode::I64Xor
                | Opcode::I64Shl
                | Opcode::I64Sar
                | Opcode::I64Shr
                | Opcode::I64Neg
                | Opcode::I32Add
                | Opcode::I32Sub
                | Opcode::I32Mul
                | Opcode::I32Div
                | Opcode::I32Rem
                | Opcode::I32And
                | Opcode::I32Or
                | Opcode::I32Xor
                | Opcode::I32Shl
                | Opcode::I32Sar
                | Opcode::I32Shr
                | Opcode::I32Neg
                | Opcode::F64Add
                | Opcode::F64Sub
                | Opcode::F64Mul
                | Opcode::F64Div
                | Opcode::F64Rem
                | Opcode::F64Neg
                | Opcode::F32Add
                | Opcode::F32Sub
                | Opcode::F32Mul
                | Opcode::F32Div
                | Opcode::F32Rem
                | Opcode::F32Neg
        )
    }

    /// Check if this is a call instruction
    pub fn is_call_op(&self) -> bool {
        matches!(
            self,
            Opcode::CallStatic | Opcode::CallVirt | Opcode::CallDyn | Opcode::CallNative
        )
    }

    /// Check if this is a return instruction
    pub fn is_return_op(&self) -> bool {
        matches!(
            self,
            Opcode::Return | Opcode::ReturnValue | Opcode::TailCall
        )
    }

    /// Check if this is a jump instruction
    pub fn is_jump_op(&self) -> bool {
        matches!(
            self,
            Opcode::Jmp
                | Opcode::JmpIf
                | Opcode::JmpIfNot
                | Opcode::Switch
                | Opcode::LoopStart
                | Opcode::LoopInc
        )
    }

    /// Check if this is a load instruction
    pub fn is_load_op(&self) -> bool {
        matches!(
            self,
            Opcode::LoadConst | Opcode::LoadLocal | Opcode::LoadArg | Opcode::LoadElement
        )
    }

    /// Check if this is a store instruction
    pub fn is_store_op(&self) -> bool {
        matches!(self, Opcode::StoreLocal | Opcode::StoreElement)
    }

    /// Get the number of operands for this opcode
    pub fn operand_count(&self) -> u8 {
        match self {
            // 0 operands
            Opcode::Nop | Opcode::Return | Opcode::Yield | Opcode::Invalid => 0,

            // 1 operand
            Opcode::ReturnValue
            | Opcode::Label
            | Opcode::Drop
            | Opcode::CloseUpvalue
            | Opcode::Throw
            | Opcode::Rethrow
            | Opcode::BoundsCheck
            | Opcode::TypeCheck
            | Opcode::StackAlloc
            | Opcode::ArcDrop
            | Opcode::TryBegin
            | Opcode::TypeOf => 1,

            // 2 operands
            Opcode::JmpIf
            | Opcode::JmpIfNot
            | Opcode::Mov
            | Opcode::LoadConst
            | Opcode::LoadLocal
            | Opcode::StoreLocal
            | Opcode::LoadArg
            | Opcode::I64Const
            | Opcode::I32Const
            | Opcode::F64Const
            | Opcode::F32Const
            | Opcode::I64Neg
            | Opcode::I32Neg
            | Opcode::F64Neg
            | Opcode::F32Neg
            | Opcode::HeapAlloc
            | Opcode::ArcNew
            | Opcode::ArcClone
            | Opcode::WeakNew
            | Opcode::WeakUpgrade
            | Opcode::StringLength
            | Opcode::StringFromInt
            | Opcode::StringFromFloat
            | Opcode::Cast
            | Opcode::LoadUpvalue
            | Opcode::StoreUpvalue => 2,

            // 3 operands
            Opcode::Switch
            | Opcode::LoopInc
            | Opcode::I64Add
            | Opcode::I64Sub
            | Opcode::I64Mul
            | Opcode::I64Div
            | Opcode::I64Rem
            | Opcode::I64And
            | Opcode::I64Or
            | Opcode::I64Xor
            | Opcode::I64Shl
            | Opcode::I64Sar
            | Opcode::I64Shr
            | Opcode::I32Add
            | Opcode::I32Sub
            | Opcode::I32Mul
            | Opcode::I32Div
            | Opcode::I32Rem
            | Opcode::I32And
            | Opcode::I32Or
            | Opcode::I32Xor
            | Opcode::I32Shl
            | Opcode::I32Sar
            | Opcode::I32Shr
            | Opcode::F64Add
            | Opcode::F64Sub
            | Opcode::F64Mul
            | Opcode::F64Div
            | Opcode::F64Rem
            | Opcode::F32Add
            | Opcode::F32Sub
            | Opcode::F32Mul
            | Opcode::F32Div
            | Opcode::F32Rem
            | Opcode::I64Eq
            | Opcode::I64Ne
            | Opcode::I64Lt
            | Opcode::I64Le
            | Opcode::I64Gt
            | Opcode::I64Ge
            | Opcode::F64Eq
            | Opcode::F64Ne
            | Opcode::F64Lt
            | Opcode::F64Le
            | Opcode::F64Gt
            | Opcode::F64Ge
            | Opcode::F32Eq
            | Opcode::F32Ne
            | Opcode::F32Lt
            | Opcode::F32Le
            | Opcode::F32Gt
            | Opcode::F32Ge
            | Opcode::GetField
            | Opcode::SetField
            | Opcode::NewListWithCap => 3,

            // Variable operands (like calls)
            Opcode::CreateStruct => 5,

            // 4 operands
            Opcode::LoopStart
            | Opcode::TailCall
            | Opcode::MakeClosure
            | Opcode::LoadElement
            | Opcode::StoreElement
            | Opcode::StringConcat
            | Opcode::StringEqual
            | Opcode::StringGetChar => 4,

            // 5 operands (function calls)
            Opcode::CallStatic | Opcode::CallVirt | Opcode::CallDyn | Opcode::CallNative => 5,

            // Default
            _ => 0,
        }
    }
}

impl fmt::Display for Opcode {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Convert from byte value
impl TryFrom<u8> for Opcode {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(Opcode::Nop),
            0x01 => Ok(Opcode::Return),
            0x02 => Ok(Opcode::ReturnValue),
            0x03 => Ok(Opcode::Jmp),
            0x04 => Ok(Opcode::JmpIf),
            0x05 => Ok(Opcode::JmpIfNot),
            0x06 => Ok(Opcode::Switch),
            0x07 => Ok(Opcode::LoopStart),
            0x08 => Ok(Opcode::LoopInc),
            0x09 => Ok(Opcode::TailCall),
            0x0A => Ok(Opcode::Yield),
            0x0B => Ok(Opcode::Label),
            0x10 => Ok(Opcode::Mov),
            0x11 => Ok(Opcode::LoadConst),
            0x12 => Ok(Opcode::LoadLocal),
            0x13 => Ok(Opcode::StoreLocal),
            0x14 => Ok(Opcode::LoadArg),
            0x20..=0x2B => Ok(unsafe { std::mem::transmute::<u8, Opcode>(value) }),
            0x30..=0x3B => Ok(unsafe { std::mem::transmute::<u8, Opcode>(value) }),
            0x40..=0x46 => Ok(unsafe { std::mem::transmute::<u8, Opcode>(value) }),
            0x50..=0x56 => Ok(unsafe { std::mem::transmute::<u8, Opcode>(value) }),
            0x60..=0x71 => Ok(unsafe { std::mem::transmute::<u8, Opcode>(value) }),
            0x72 => Ok(Opcode::HeapAlloc),
            0x73 => Ok(Opcode::StackAlloc),
            0x74 => Ok(Opcode::Drop),
            0x75 => Ok(Opcode::GetField),
            0x76 => Ok(Opcode::SetField),
            0x77 => Ok(Opcode::LoadElement),
            0x78 => Ok(Opcode::StoreElement),
            0x7A => Ok(Opcode::NewListWithCap),
            0x79 => Ok(Opcode::CreateStruct),
            0x7B => Ok(Opcode::ArcNew),
            0x7C => Ok(Opcode::ArcClone),
            0x7D => Ok(Opcode::ArcDrop),
            0x7E => Ok(Opcode::WeakNew),
            0x7F => Ok(Opcode::WeakUpgrade),
            0x80 => Ok(Opcode::CallStatic),
            0x81 => Ok(Opcode::CallVirt),
            0x82 => Ok(Opcode::CallDyn),
            0x83 => Ok(Opcode::MakeClosure),
            0x84 => Ok(Opcode::LoadUpvalue),
            0x85 => Ok(Opcode::StoreUpvalue),
            0x86 => Ok(Opcode::CloseUpvalue),
            0x87 => Ok(Opcode::CallNative),
            0x90 => Ok(Opcode::StringLength),
            0x91 => Ok(Opcode::StringConcat),
            0x92 => Ok(Opcode::StringEqual),
            0x93 => Ok(Opcode::StringGetChar),
            0x94 => Ok(Opcode::StringFromInt),
            0x95 => Ok(Opcode::StringFromFloat),
            0xA0 => Ok(Opcode::TryBegin),
            0xA1 => Ok(Opcode::TryEnd),
            0xA2 => Ok(Opcode::Throw),
            0xA3 => Ok(Opcode::Rethrow),
            0xB0 => Ok(Opcode::BoundsCheck),
            0xC0 => Ok(Opcode::TypeCheck),
            0xC1 => Ok(Opcode::Cast),
            0xD0 => Ok(Opcode::TypeOf),
            0xE0 => Ok(Opcode::Custom0),
            0xE1 => Ok(Opcode::Custom1),
            0xE2 => Ok(Opcode::Custom2),
            0xE3 => Ok(Opcode::Custom3),
            0xFF => Ok(Opcode::Invalid),
            _ => Err(()),
        }
    }
}
