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

    /// Tail call (TCO)
    TailCall = 0x09,

    /// Yield (async scheduling)
    Yield = 0x0A,

    /// Label definition
    Label = 0x0B,

    /// Spawn a new concurrent task (dynamic call)
    Spawn = 0x0E,

    /// 从 List 寄存器动态读取闭包并 spawn
    SpawnFromList = 0x0F,

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

    /// Borrow token (ZST, runtime ~ Mov)
    Borrow = 0x15,

    /// Release borrow token (ZST, runtime ~ Nop)
    Release = 0x16,

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

    // =====================
    // Float Operations (0x40-0x5F)
    // =====================

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

    /// Create dict instance
    NewDict = 0x88,

    /// Create Rc (non-atomic reference count)
    RcNew = 0x89,

    /// Create tuple instance (SPEC §3.6)
    NewTuple = 0x8A,

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
            Opcode::TailCall => "TailCall",
            Opcode::Yield => "Yield",
            Opcode::Label => "Label",
            Opcode::Spawn => "Spawn",
            Opcode::SpawnFromList => "SpawnFromList",
            Opcode::Mov => "Mov",
            Opcode::LoadConst => "LoadConst",
            Opcode::LoadLocal => "LoadLocal",
            Opcode::StoreLocal => "StoreLocal",
            Opcode::LoadArg => "LoadArg",
            Opcode::Borrow => "Borrow",
            Opcode::Release => "Release",
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
            Opcode::I64Eq => "I64Eq",
            Opcode::I64Ne => "I64Ne",
            Opcode::I64Lt => "I64Lt",
            Opcode::I64Le => "I64Le",
            Opcode::I64Gt => "I64Gt",
            Opcode::I64Ge => "I64Ge",
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
            Opcode::RcNew => "RcNew",
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
            Opcode::NewDict => "NewDict",
            Opcode::NewTuple => "NewTuple",
            Opcode::StringLength => "StringLength",
            Opcode::StringConcat => "StringConcat",
            Opcode::StringEqual => "StringEqual",
            Opcode::StringGetChar => "StringGetChar",
            Opcode::StringFromInt => "StringFromInt",
            Opcode::StringFromFloat => "StringFromFloat",
            Opcode::TryBegin => "TryBegin",
            Opcode::TryEnd => "TryEnd",
            Opcode::Throw => "Throw",
            Opcode::BoundsCheck => "BoundsCheck",
            Opcode::TypeCheck => "TypeCheck",
            Opcode::Cast => "Cast",
            Opcode::TypeOf => "TypeOf",
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
            0x09 => Ok(Opcode::TailCall),
            0x0A => Ok(Opcode::Yield),
            0x0B => Ok(Opcode::Label),
            0x0E => Ok(Opcode::Spawn),
            0x0F => Ok(Opcode::SpawnFromList),
            0x10 => Ok(Opcode::Mov),
            0x11 => Ok(Opcode::LoadConst),
            0x12 => Ok(Opcode::LoadLocal),
            0x13 => Ok(Opcode::StoreLocal),
            0x14 => Ok(Opcode::LoadArg),
            0x15 => Ok(Opcode::Borrow),
            0x16 => Ok(Opcode::Release),
            0x20..=0x2B => Ok(unsafe { std::mem::transmute::<u8, Opcode>(value) }),
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
            0x88 => Ok(Opcode::NewDict),
            0x89 => Ok(Opcode::RcNew),
            0x8A => Ok(Opcode::NewTuple),
            0x90 => Ok(Opcode::StringLength),
            0x91 => Ok(Opcode::StringConcat),
            0x92 => Ok(Opcode::StringEqual),
            0x93 => Ok(Opcode::StringGetChar),
            0x94 => Ok(Opcode::StringFromInt),
            0x95 => Ok(Opcode::StringFromFloat),
            0xA0 => Ok(Opcode::TryBegin),
            0xA1 => Ok(Opcode::TryEnd),
            0xA2 => Ok(Opcode::Throw),
            0xB0 => Ok(Opcode::BoundsCheck),
            0xC0 => Ok(Opcode::TypeCheck),
            0xC1 => Ok(Opcode::Cast),
            0xD0 => Ok(Opcode::TypeOf),
            _ => Err(()),
        }
    }
}
