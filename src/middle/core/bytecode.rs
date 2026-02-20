//! Bytecode Intermediate Representation (Low-level IR)
//!
//! This module defines the bytecode IR - a low-level, platform-agnostic
//! representation of compiled YaoXiang code. It serves as the interface
//! between the code generator and the execution backend.
//!
//! Unlike high-level IR (middle/ir.rs), this IR:
//! - Is closer to the actual execution model
//! - Is suitable for serialization
//! - Can be interpreted or compiled further

use std::collections::HashMap;
use crate::tlog;
use crate::util::i18n::MSG;
use crate::backends::common::Opcode;

// Re-export types for conversion
pub use crate::middle::core::ir::{Type as IrType, ConstValue};

/// Register index in the virtual machine
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Reg(pub u16);

impl Reg {
    /// Create a new register
    pub fn new(index: u16) -> Self {
        Self(index)
    }

    /// Get the register index
    pub fn index(&self) -> u16 {
        self.0
    }
}

impl std::fmt::Display for Reg {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "r{}", self.0)
    }
}

/// Label for jump targets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Label(pub u32);

impl Label {
    /// Create a new label
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the label id
    pub fn id(&self) -> u32 {
        self.0
    }
}

impl std::fmt::Display for Label {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "L{}", self.0)
    }
}

/// Binary operation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    /// Addition (+)
    Add,
    /// Subtraction (-)
    Sub,
    /// Multiplication (*)
    Mul,
    /// Division (/)
    Div,
    /// Modulo (%)
    Rem,
    /// Bitwise AND (&)
    And,
    /// Bitwise OR (|)
    Or,
    /// Bitwise XOR (^)
    Xor,
    /// Left shift (<<)
    Shl,
    /// Arithmetic right shift (>>)
    Sar,
    /// Logical right shift (>>>)
    Shr,
}

/// Unary operation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    /// Negation (-)
    Neg,
    /// Bitwise NOT (!)
    Not,
}

/// Comparison operation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    /// Equal (==)
    Eq,
    /// Not equal (!=)
    Ne,
    /// Less than (<)
    Lt,
    /// Less than or equal (<=)
    Le,
    /// Greater than (>)
    Gt,
    /// Greater than or equal (>=)
    Ge,
}

/// Function reference
#[derive(Debug, Clone)]
pub enum FunctionRef {
    /// Static function reference by name
    Static {
        /// Module name (empty for current module)
        module: String,
        /// Function name
        name: String,
    },
    /// Reference by index (after linking)
    Index(u32),
}

/// Bytecode instruction
///
/// This is the low-level instruction format. Each instruction has:
/// - An opcode (operation to perform)
/// - Operands (arguments, if any)
#[derive(Debug, Clone)]
pub enum BytecodeInstr {
    // =====================
    // Control Flow
    // =====================
    /// No-op
    Nop,

    /// Return without value
    Return,

    /// Return with value
    ReturnValue {
        value: Reg,
    },

    /// Unconditional jump
    Jmp {
        target: Label,
    },

    /// Conditional jump (if true)
    JmpIf {
        cond: Reg,
        target: Label,
    },

    /// Conditional jump (if false)
    JmpIfNot {
        cond: Reg,
        target: Label,
    },

    /// Switch/case dispatch
    Switch {
        value: Reg,
        /// (default_target, [(value, target), ...])
        targets: Vec<(Option<Label>, Label)>,
    },

    // =====================
    // Register Operations
    // =====================
    /// Register move
    Mov {
        dst: Reg,
        src: Reg,
    },

    /// Load constant
    LoadConst {
        dst: Reg,
        const_idx: u16,
    },

    /// Load local variable
    LoadLocal {
        dst: Reg,
        local_idx: u8,
    },

    /// Store local variable
    StoreLocal {
        local_idx: u8,
        src: Reg,
    },

    /// Load function argument
    LoadArg {
        dst: Reg,
        arg_idx: u8,
    },

    // =====================
    // Binary Operations
    // =====================
    BinaryOp {
        dst: Reg,
        lhs: Reg,
        rhs: Reg,
        op: BinaryOp,
    },

    UnaryOp {
        dst: Reg,
        src: Reg,
        op: UnaryOp,
    },

    // =====================
    // Comparison
    // =====================
    Compare {
        dst: Reg,
        lhs: Reg,
        rhs: Reg,
        cmp: CompareOp,
    },

    // =====================
    // Memory Operations
    // =====================
    /// Stack allocation
    StackAlloc {
        dst: Reg,
        size: u16,
    },

    /// Heap allocation
    HeapAlloc {
        dst: Reg,
        type_id: u16,
    },

    /// Drop value
    Drop {
        value: Reg,
    },

    /// Get struct field
    GetField {
        dst: Reg,
        src: Reg,
        field_idx: u16,
    },

    /// Set struct field
    SetField {
        src: Reg,
        field_idx: u16,
        value: Reg,
    },

    /// Load element from array/list
    LoadElement {
        dst: Reg,
        array: Reg,
        index: Reg,
    },

    /// Store element to array/list
    StoreElement {
        array: Reg,
        index: Reg,
        value: Reg,
    },

    /// Create list with capacity
    NewListWithCap {
        dst: Reg,
        capacity: u16,
    },

    /// 创建结构体实例
    CreateStruct {
        dst: Reg,
        type_name: String,
        fields: Vec<Reg>,
    },

    // =====================
    // Arc Operations
    // =====================
    ArcNew {
        dst: Reg,
        src: Reg,
    },
    ArcClone {
        dst: Reg,
        src: Reg,
    },
    ArcDrop {
        src: Reg,
    },
    /// Create Weak from Arc
    WeakNew {
        dst: Reg,
        src: Reg,
    },
    /// Upgrade Weak to Arc (returns Option)
    WeakUpgrade {
        dst: Reg,
        src: Reg,
    },

    // =====================
    // Function Call
    // =====================
    /// Static dispatch call
    CallStatic {
        dst: Option<Reg>,
        func: FunctionRef,
        args: Vec<Reg>,
    },

    /// Native function call (FFI)
    CallNative {
        dst: Option<Reg>,
        func_name: String,
        args: Vec<Reg>,
    },

    /// Virtual dispatch call
    CallVirt {
        dst: Option<Reg>,
        obj: Reg,
        method_idx: u16,
        args: Vec<Reg>,
    },

    /// Dynamic dispatch call
    CallDyn {
        dst: Option<Reg>,
        obj: Reg,
        name_idx: u16,
        args: Vec<Reg>,
    },

    /// Create closure
    MakeClosure {
        dst: Reg,
        func: FunctionRef,
        env: Vec<Reg>,
    },

    /// Load upvalue
    LoadUpvalue {
        dst: Reg,
        upvalue_idx: u8,
    },

    /// Store upvalue
    StoreUpvalue {
        src: Reg,
        upvalue_idx: u8,
    },

    /// Close upvalue
    CloseUpvalue {
        src: Reg,
    },

    // =====================
    // String Operations
    // =====================
    StringLength {
        dst: Reg,
        src: Reg,
    },
    StringConcat {
        dst: Reg,
        str1: Reg,
        str2: Reg,
    },
    StringEqual {
        dst: Reg,
        str1: Reg,
        str2: Reg,
    },
    StringGetChar {
        dst: Reg,
        src: Reg,
        index: Reg,
    },
    StringFromInt {
        dst: Reg,
        src: Reg,
    },
    StringFromFloat {
        dst: Reg,
        src: Reg,
    },

    // =====================
    // Exception Handling
    // =====================
    TryBegin {
        catch_target: Label,
    },
    TryEnd,
    Throw {
        error: Reg,
    },

    // =====================
    // Debug Operations
    // =====================
    /// Bounds check (only in debug mode)
    BoundsCheck {
        array: Reg,
        index: Reg,
    },

    /// Type check (only in debug mode)
    TypeCheck {
        value: Reg,
        type_id: u16,
    },

    /// Cast value to type
    Cast {
        dst: Reg,
        src: Reg,
        target_type_id: u16,
    },

    // =====================
    // Reflection
    // =====================
    TypeOf {
        dst: Reg,
        src: Reg,
    },
}

impl BytecodeInstr {
    /// Get the opcode for this instruction
    pub fn opcode(&self) -> Opcode {
        match self {
            BytecodeInstr::Nop => Opcode::Nop,
            BytecodeInstr::Return => Opcode::Return,
            BytecodeInstr::ReturnValue { .. } => Opcode::ReturnValue,
            BytecodeInstr::Jmp { .. } => Opcode::Jmp,
            BytecodeInstr::JmpIf { .. } => Opcode::JmpIf,
            BytecodeInstr::JmpIfNot { .. } => Opcode::JmpIfNot,
            BytecodeInstr::Switch { .. } => Opcode::Switch,
            BytecodeInstr::Mov { .. } => Opcode::Mov,
            BytecodeInstr::LoadConst { .. } => Opcode::LoadConst,
            BytecodeInstr::LoadLocal { .. } => Opcode::LoadLocal,
            BytecodeInstr::StoreLocal { .. } => Opcode::StoreLocal,
            BytecodeInstr::LoadArg { .. } => Opcode::LoadArg,
            BytecodeInstr::BinaryOp { op, .. } => match op {
                BinaryOp::Add => Opcode::I64Add,
                BinaryOp::Sub => Opcode::I64Sub,
                BinaryOp::Mul => Opcode::I64Mul,
                BinaryOp::Div => Opcode::I64Div,
                BinaryOp::Rem => Opcode::I64Rem,
                BinaryOp::And => Opcode::I64And,
                BinaryOp::Or => Opcode::I64Or,
                BinaryOp::Xor => Opcode::I64Xor,
                BinaryOp::Shl => Opcode::I64Shl,
                BinaryOp::Sar => Opcode::I64Sar,
                BinaryOp::Shr => Opcode::I64Shr,
            },
            BytecodeInstr::UnaryOp { .. } => Opcode::I64Neg,
            BytecodeInstr::Compare { cmp, .. } => match cmp {
                CompareOp::Eq => Opcode::I64Eq,
                CompareOp::Ne => Opcode::I64Ne,
                CompareOp::Lt => Opcode::I64Lt,
                CompareOp::Le => Opcode::I64Le,
                CompareOp::Gt => Opcode::I64Gt,
                CompareOp::Ge => Opcode::I64Ge,
            },
            BytecodeInstr::StackAlloc { .. } => Opcode::StackAlloc,
            BytecodeInstr::HeapAlloc { .. } => Opcode::HeapAlloc,
            BytecodeInstr::Drop { .. } => Opcode::Drop,
            BytecodeInstr::GetField { .. } => Opcode::GetField,
            BytecodeInstr::SetField { .. } => Opcode::SetField,
            BytecodeInstr::LoadElement { .. } => Opcode::LoadElement,
            BytecodeInstr::StoreElement { .. } => Opcode::StoreElement,
            BytecodeInstr::NewListWithCap { .. } => Opcode::NewListWithCap,
            BytecodeInstr::CreateStruct { .. } => Opcode::CreateStruct,
            BytecodeInstr::ArcNew { .. } => Opcode::ArcNew,
            BytecodeInstr::ArcClone { .. } => Opcode::ArcClone,
            BytecodeInstr::ArcDrop { .. } => Opcode::ArcDrop,
            BytecodeInstr::WeakNew { .. } => Opcode::WeakNew,
            BytecodeInstr::WeakUpgrade { .. } => Opcode::WeakUpgrade,
            BytecodeInstr::CallStatic { .. } => Opcode::CallStatic,
            BytecodeInstr::CallNative { .. } => Opcode::CallNative,
            BytecodeInstr::CallVirt { .. } => Opcode::CallVirt,
            BytecodeInstr::CallDyn { .. } => Opcode::CallDyn,
            BytecodeInstr::MakeClosure { .. } => Opcode::MakeClosure,
            BytecodeInstr::LoadUpvalue { .. } => Opcode::LoadUpvalue,
            BytecodeInstr::StoreUpvalue { .. } => Opcode::StoreUpvalue,
            BytecodeInstr::CloseUpvalue { .. } => Opcode::CloseUpvalue,
            BytecodeInstr::StringLength { .. } => Opcode::StringLength,
            BytecodeInstr::StringConcat { .. } => Opcode::StringConcat,
            BytecodeInstr::StringEqual { .. } => Opcode::StringEqual,
            BytecodeInstr::StringGetChar { .. } => Opcode::StringGetChar,
            BytecodeInstr::StringFromInt { .. } => Opcode::StringFromInt,
            BytecodeInstr::StringFromFloat { .. } => Opcode::StringFromFloat,
            BytecodeInstr::TryBegin { .. } => Opcode::TryBegin,
            BytecodeInstr::TryEnd => Opcode::TryEnd,
            BytecodeInstr::Throw { .. } => Opcode::Throw,
            BytecodeInstr::BoundsCheck { .. } => Opcode::BoundsCheck,
            BytecodeInstr::TypeCheck { .. } => Opcode::TypeCheck,
            BytecodeInstr::Cast { .. } => Opcode::Cast,
            BytecodeInstr::TypeOf { .. } => Opcode::TypeOf,
        }
    }

    /// Get the instruction size in bytes
    pub fn size(&self) -> usize {
        1 + match self {
            BytecodeInstr::Nop => 0,
            BytecodeInstr::Return => 0,
            BytecodeInstr::ReturnValue { .. } => 2,
            BytecodeInstr::Jmp { .. } => 4,
            BytecodeInstr::JmpIf { .. } => 4,
            BytecodeInstr::JmpIfNot { .. } => 4,
            BytecodeInstr::Switch { targets, .. } => 2 + targets.len() * 4,
            BytecodeInstr::Mov { .. } => 4,
            BytecodeInstr::LoadConst { .. } => 4,
            BytecodeInstr::LoadLocal { .. } => 3,
            BytecodeInstr::StoreLocal { .. } => 3,
            BytecodeInstr::LoadArg { .. } => 3,
            BytecodeInstr::BinaryOp { .. } => 6,
            BytecodeInstr::UnaryOp { .. } => 4,
            BytecodeInstr::Compare { .. } => 6,
            BytecodeInstr::StackAlloc { .. } => 4,
            BytecodeInstr::HeapAlloc { .. } => 4,
            BytecodeInstr::Drop { .. } => 2,
            BytecodeInstr::GetField { .. } => 4,
            BytecodeInstr::SetField { .. } => 4,
            BytecodeInstr::LoadElement { .. } => 4,
            BytecodeInstr::StoreElement { .. } => 4,
            BytecodeInstr::NewListWithCap { .. } => 4,
            BytecodeInstr::CreateStruct {
                fields, type_name, ..
            } => 6 + type_name.len() + fields.len() * 2,
            BytecodeInstr::ArcNew { .. } => 4,
            BytecodeInstr::ArcClone { .. } => 4,
            BytecodeInstr::ArcDrop { .. } => 2,
            BytecodeInstr::WeakNew { .. } => 4,
            BytecodeInstr::WeakUpgrade { .. } => 4,
            BytecodeInstr::CallStatic { args, .. } => 4 + args.len() * 2,
            BytecodeInstr::CallNative {
                args, func_name, ..
            } => 4 + func_name.len() + args.len() * 2,
            BytecodeInstr::CallVirt { args, .. } => 4 + args.len() * 2,
            BytecodeInstr::CallDyn { args, .. } => 4 + args.len() * 2,
            BytecodeInstr::MakeClosure { env, .. } => 4 + env.len() * 2,
            BytecodeInstr::LoadUpvalue { .. } => 3,
            BytecodeInstr::StoreUpvalue { .. } => 3,
            BytecodeInstr::CloseUpvalue { .. } => 2,
            BytecodeInstr::StringLength { .. } => 4,
            BytecodeInstr::StringConcat { .. } => 4,
            BytecodeInstr::StringEqual { .. } => 4,
            BytecodeInstr::StringGetChar { .. } => 4,
            BytecodeInstr::StringFromInt { .. } => 4,
            BytecodeInstr::StringFromFloat { .. } => 4,
            BytecodeInstr::TryBegin { .. } => 4,
            BytecodeInstr::TryEnd => 0,
            BytecodeInstr::Throw { .. } => 2,
            BytecodeInstr::BoundsCheck { .. } => 4,
            BytecodeInstr::TypeCheck { .. } => 4,
            BytecodeInstr::Cast { .. } => 4,
            BytecodeInstr::TypeOf { .. } => 4,
        }
    }
}

/// Bytecode function
#[derive(Debug, Clone)]
pub struct BytecodeFunction {
    /// Function name
    pub name: String,
    /// Parameter types
    pub params: Vec<crate::middle::core::ir::Type>,
    /// Return type
    pub return_type: crate::middle::core::ir::Type,
    /// Number of local variables
    pub local_count: usize,
    /// Number of upvalues
    pub upvalue_count: usize,
    /// Instructions
    pub instructions: Vec<BytecodeInstr>,
    /// Label to instruction index mapping
    pub labels: HashMap<Label, usize>,
    /// Exception handlers (try-catch blocks)
    pub exception_handlers: Vec<ExceptionHandler>,
}

/// Exception handler information
#[derive(Debug, Clone)]
pub struct ExceptionHandler {
    /// Try block start label
    pub try_start: Label,
    /// Try block end label
    pub try_end: Label,
    /// Catch block start label
    pub catch_start: Label,
    /// Exception type to catch (0 for all)
    pub exception_type: u16,
}

/// Bytecode module
#[derive(Debug, Clone)]
pub struct BytecodeModule {
    /// Module name
    pub name: String,
    /// Constant pool
    pub constants: Vec<ConstValue>,
    /// Functions defined in this module
    pub functions: Vec<BytecodeFunction>,
    /// Type table
    pub type_table: Vec<crate::middle::core::ir::Type>,
    /// Global variables
    pub globals: Vec<GlobalInfo>,
    /// Entry point function index
    pub entry_point: Option<usize>,
}

/// Global variable information
#[derive(Debug, Clone)]
pub struct GlobalInfo {
    /// Variable name
    pub name: String,
    /// Variable type
    pub type_id: u16,
    /// Initial value (if compile-time constant)
    pub initializer: Option<ConstValue>,
    /// Is mutable
    pub is_mutable: bool,
}

impl BytecodeModule {
    /// Create a new empty module
    pub fn new(name: String) -> Self {
        Self {
            name,
            constants: Vec::new(),
            functions: Vec::new(),
            type_table: Vec::new(),
            globals: Vec::new(),
            entry_point: None,
        }
    }

    /// Add a constant and return its index
    pub fn add_constant(
        &mut self,
        value: ConstValue,
    ) -> u16 {
        let idx = self.constants.len() as u16;
        self.constants.push(value);
        idx
    }

    /// Add a function and return its index
    pub fn add_function(
        &mut self,
        func: BytecodeFunction,
    ) -> usize {
        let idx = self.functions.len();
        self.functions.push(func);
        idx
    }
}

impl From<crate::middle::passes::codegen::bytecode::BytecodeFile> for BytecodeModule {
    fn from(file: crate::middle::passes::codegen::bytecode::BytecodeFile) -> Self {
        let name = "main".to_string(); // Default module name

        // Convert functions
        let mut functions = Vec::new();
        for func in file.code_section.functions {
            // Decode instructions from BytecodeInstruction to BytecodeInstr
            let mut decoded_instructions = Vec::new();
            let mut labels = std::collections::HashMap::new();
            let mut ip = 0;
            while ip < func.instructions.len() {
                let instr = &func.instructions[ip];
                // Decode the instruction based on opcode
                match Opcode::try_from(instr.opcode) {
                    Ok(opcode) => {
                        match opcode {
                            Opcode::Label => {
                                if !instr.operands.is_empty() {
                                    let label = u32::from_le_bytes([
                                        instr.operands[0],
                                        *instr.operands.get(1).unwrap_or(&0),
                                        *instr.operands.get(2).unwrap_or(&0),
                                        *instr.operands.get(3).unwrap_or(&0),
                                    ]);
                                    labels.insert(Label(label), decoded_instructions.len());
                                }
                            }
                            Opcode::Jmp => {
                                if !instr.operands.is_empty() {
                                    let target = u32::from_le_bytes([
                                        instr.operands[0],
                                        *instr.operands.get(1).unwrap_or(&0),
                                        *instr.operands.get(2).unwrap_or(&0),
                                        *instr.operands.get(3).unwrap_or(&0),
                                    ]);
                                    decoded_instructions.push(BytecodeInstr::Jmp {
                                        target: Label(target),
                                    });
                                }
                            }
                            Opcode::JmpIf => {
                                if instr.operands.len() >= 5 {
                                    let cond = instr.operands[0] as u16;
                                    let target = u32::from_le_bytes([
                                        instr.operands[1],
                                        instr.operands[2],
                                        instr.operands[3],
                                        instr.operands[4],
                                    ]);
                                    decoded_instructions.push(BytecodeInstr::JmpIf {
                                        cond: Reg(cond),
                                        target: Label(target),
                                    });
                                }
                            }
                            Opcode::JmpIfNot => {
                                if instr.operands.len() >= 5 {
                                    let cond = instr.operands[0] as u16;
                                    let target = u32::from_le_bytes([
                                        instr.operands[1],
                                        instr.operands[2],
                                        instr.operands[3],
                                        instr.operands[4],
                                    ]);
                                    decoded_instructions.push(BytecodeInstr::JmpIfNot {
                                        cond: Reg(cond),
                                        target: Label(target),
                                    });
                                }
                            }
                            Opcode::I64Add => {
                                tlog!(
                                    debug,
                                    MSG::BytecodeDecodeI64Add,
                                    &instr.operands.len().to_string()
                                );
                                if instr.operands.len() >= 6 {
                                    let dst =
                                        u16::from_le_bytes([instr.operands[0], instr.operands[1]]);
                                    let lhs =
                                        u16::from_le_bytes([instr.operands[2], instr.operands[3]]);
                                    let rhs =
                                        u16::from_le_bytes([instr.operands[4], instr.operands[5]]);
                                    decoded_instructions.push(BytecodeInstr::BinaryOp {
                                        op: BinaryOp::Add,
                                        dst: Reg(dst),
                                        lhs: Reg(lhs),
                                        rhs: Reg(rhs),
                                    });
                                } else {
                                    tlog!(warn, MSG::BytecodeDecodeI64AddTooShort);
                                }
                            }
                            Opcode::I64Sub => {
                                if instr.operands.len() >= 3 {
                                    let dst = instr.operands[0] as u16;
                                    let lhs = instr.operands[1] as u16;
                                    let rhs = instr.operands[2] as u16;
                                    decoded_instructions.push(BytecodeInstr::BinaryOp {
                                        op: BinaryOp::Sub,
                                        dst: Reg(dst),
                                        lhs: Reg(lhs),
                                        rhs: Reg(rhs),
                                    });
                                }
                            }
                            Opcode::I64Mul => {
                                if instr.operands.len() >= 3 {
                                    let dst = instr.operands[0] as u16;
                                    let lhs = instr.operands[1] as u16;
                                    let rhs = instr.operands[2] as u16;
                                    decoded_instructions.push(BytecodeInstr::BinaryOp {
                                        op: BinaryOp::Mul,
                                        dst: Reg(dst),
                                        lhs: Reg(lhs),
                                        rhs: Reg(rhs),
                                    });
                                }
                            }
                            Opcode::I64Div => {
                                if instr.operands.len() >= 3 {
                                    let dst = instr.operands[0] as u16;
                                    let lhs = instr.operands[1] as u16;
                                    let rhs = instr.operands[2] as u16;
                                    decoded_instructions.push(BytecodeInstr::BinaryOp {
                                        op: BinaryOp::Div,
                                        dst: Reg(dst),
                                        lhs: Reg(lhs),
                                        rhs: Reg(rhs),
                                    });
                                }
                            }
                            Opcode::I64Rem => {
                                if instr.operands.len() >= 3 {
                                    let dst = instr.operands[0] as u16;
                                    let lhs = instr.operands[1] as u16;
                                    let rhs = instr.operands[2] as u16;
                                    decoded_instructions.push(BytecodeInstr::BinaryOp {
                                        op: BinaryOp::Rem,
                                        dst: Reg(dst),
                                        lhs: Reg(lhs),
                                        rhs: Reg(rhs),
                                    });
                                }
                            }
                            Opcode::I64And => {
                                if instr.operands.len() >= 3 {
                                    let dst = instr.operands[0] as u16;
                                    let lhs = instr.operands[1] as u16;
                                    let rhs = instr.operands[2] as u16;
                                    decoded_instructions.push(BytecodeInstr::BinaryOp {
                                        op: BinaryOp::And,
                                        dst: Reg(dst),
                                        lhs: Reg(lhs),
                                        rhs: Reg(rhs),
                                    });
                                }
                            }
                            Opcode::I64Or => {
                                if instr.operands.len() >= 3 {
                                    let dst = instr.operands[0] as u16;
                                    let lhs = instr.operands[1] as u16;
                                    let rhs = instr.operands[2] as u16;
                                    decoded_instructions.push(BytecodeInstr::BinaryOp {
                                        op: BinaryOp::Or,
                                        dst: Reg(dst),
                                        lhs: Reg(lhs),
                                        rhs: Reg(rhs),
                                    });
                                }
                            }
                            Opcode::I64Xor => {
                                if instr.operands.len() >= 3 {
                                    let dst = instr.operands[0] as u16;
                                    let lhs = instr.operands[1] as u16;
                                    let rhs = instr.operands[2] as u16;
                                    decoded_instructions.push(BytecodeInstr::BinaryOp {
                                        op: BinaryOp::Xor,
                                        dst: Reg(dst),
                                        lhs: Reg(lhs),
                                        rhs: Reg(rhs),
                                    });
                                }
                            }
                            Opcode::I64Shl => {
                                if instr.operands.len() >= 3 {
                                    let dst = instr.operands[0] as u16;
                                    let lhs = instr.operands[1] as u16;
                                    let rhs = instr.operands[2] as u16;
                                    decoded_instructions.push(BytecodeInstr::BinaryOp {
                                        op: BinaryOp::Shl,
                                        dst: Reg(dst),
                                        lhs: Reg(lhs),
                                        rhs: Reg(rhs),
                                    });
                                }
                            }
                            Opcode::I64Sar => {
                                if instr.operands.len() >= 3 {
                                    let dst = instr.operands[0] as u16;
                                    let lhs = instr.operands[1] as u16;
                                    let rhs = instr.operands[2] as u16;
                                    decoded_instructions.push(BytecodeInstr::BinaryOp {
                                        op: BinaryOp::Sar,
                                        dst: Reg(dst),
                                        lhs: Reg(lhs),
                                        rhs: Reg(rhs),
                                    });
                                }
                            }
                            Opcode::I64Shr => {
                                if instr.operands.len() >= 3 {
                                    let dst = instr.operands[0] as u16;
                                    let lhs = instr.operands[1] as u16;
                                    let rhs = instr.operands[2] as u16;
                                    decoded_instructions.push(BytecodeInstr::BinaryOp {
                                        op: BinaryOp::Shr,
                                        dst: Reg(dst),
                                        lhs: Reg(lhs),
                                        rhs: Reg(rhs),
                                    });
                                }
                            }
                            Opcode::I64Lt => {
                                if instr.operands.len() >= 3 {
                                    let dst = instr.operands[0] as u16;
                                    let lhs = instr.operands[1] as u16;
                                    let rhs = instr.operands[2] as u16;
                                    decoded_instructions.push(BytecodeInstr::Compare {
                                        cmp: CompareOp::Lt,
                                        dst: Reg(dst),
                                        lhs: Reg(lhs),
                                        rhs: Reg(rhs),
                                    });
                                }
                            }
                            Opcode::I64Le => {
                                if instr.operands.len() >= 3 {
                                    let dst = instr.operands[0] as u16;
                                    let lhs = instr.operands[1] as u16;
                                    let rhs = instr.operands[2] as u16;
                                    decoded_instructions.push(BytecodeInstr::Compare {
                                        cmp: CompareOp::Le,
                                        dst: Reg(dst),
                                        lhs: Reg(lhs),
                                        rhs: Reg(rhs),
                                    });
                                }
                            }
                            Opcode::I64Gt => {
                                if instr.operands.len() >= 3 {
                                    let dst = instr.operands[0] as u16;
                                    let lhs = instr.operands[1] as u16;
                                    let rhs = instr.operands[2] as u16;
                                    decoded_instructions.push(BytecodeInstr::Compare {
                                        cmp: CompareOp::Gt,
                                        dst: Reg(dst),
                                        lhs: Reg(lhs),
                                        rhs: Reg(rhs),
                                    });
                                }
                            }
                            Opcode::I64Ge => {
                                if instr.operands.len() >= 3 {
                                    let dst = instr.operands[0] as u16;
                                    let lhs = instr.operands[1] as u16;
                                    let rhs = instr.operands[2] as u16;
                                    decoded_instructions.push(BytecodeInstr::Compare {
                                        cmp: CompareOp::Ge,
                                        dst: Reg(dst),
                                        lhs: Reg(lhs),
                                        rhs: Reg(rhs),
                                    });
                                }
                            }
                            Opcode::I64Ne => {
                                if instr.operands.len() >= 3 {
                                    let dst = instr.operands[0] as u16;
                                    let lhs = instr.operands[1] as u16;
                                    let rhs = instr.operands[2] as u16;
                                    decoded_instructions.push(BytecodeInstr::Compare {
                                        cmp: CompareOp::Ne,
                                        dst: Reg(dst),
                                        lhs: Reg(lhs),
                                        rhs: Reg(rhs),
                                    });
                                }
                            }
                            Opcode::I64Eq => {
                                if instr.operands.len() >= 3 {
                                    let dst = instr.operands[0] as u16;
                                    let lhs = instr.operands[1] as u16;
                                    let rhs = instr.operands[2] as u16;
                                    decoded_instructions.push(BytecodeInstr::Compare {
                                        cmp: CompareOp::Eq,
                                        dst: Reg(dst),
                                        lhs: Reg(lhs),
                                        rhs: Reg(rhs),
                                    });
                                }
                            }
                            Opcode::I64Neg => {
                                // Unary negation: -x
                                // Operands: dst(1) + src(1)
                                if instr.operands.len() >= 2 {
                                    let dst = instr.operands[0] as u16;
                                    let src = instr.operands[1] as u16;
                                    decoded_instructions.push(BytecodeInstr::UnaryOp {
                                        dst: Reg(dst),
                                        src: Reg(src),
                                        op: UnaryOp::Neg,
                                    });
                                }
                            }
                            Opcode::CallStatic => {
                                // CallStatic: dst(1) + func_id(4) + base_arg_reg(1) + arg_count(1) + args(2*count)
                                if instr.operands.len() >= 7 {
                                    let dst = instr.operands[0] as u16;
                                    let func_id = u32::from_le_bytes([
                                        instr.operands[1],
                                        instr.operands[2],
                                        instr.operands[3],
                                        instr.operands[4],
                                    ]);
                                    let _base_arg_reg = instr.operands[5];
                                    let arg_count = instr.operands[6] as usize;

                                    // Create function reference from func_id
                                    let func_ref = FunctionRef::Index(func_id);

                                    // Parse arguments
                                    let mut args = Vec::new();
                                    for i in 0..arg_count {
                                        if 7 + i * 2 + 1 < instr.operands.len() {
                                            let arg_reg = u16::from_le_bytes([
                                                instr.operands[7 + i * 2],
                                                instr.operands[7 + i * 2 + 1],
                                            ]);
                                            args.push(Reg(arg_reg));
                                        }
                                    }

                                    // Create CallStatic instruction
                                    // Note: dst=0 is a valid register (reg 0), not None
                                    // The distinction between "has return value" and "no return value"
                                    // should be determined by the function signature, not the dst register
                                    let dst_reg = Some(Reg(dst));
                                    let call_instr = BytecodeInstr::CallStatic {
                                        dst: dst_reg,
                                        func: func_ref,
                                        args,
                                    };
                                    decoded_instructions.push(call_instr);
                                } else {
                                    // Fallback: push Nop
                                    decoded_instructions.push(BytecodeInstr::Nop);
                                }
                            }
                            Opcode::CallNative => {
                                // CallNative: dst(1) + func_name_idx(4) + base_arg_reg(1) + arg_count(1) + args(2*count)
                                if instr.operands.len() >= 7 {
                                    let dst = instr.operands[0] as u16;
                                    let func_name_idx = u32::from_le_bytes([
                                        instr.operands[1],
                                        instr.operands[2],
                                        instr.operands[3],
                                        instr.operands[4],
                                    ]);
                                    let _base_arg_reg = instr.operands[5];
                                    let arg_count = instr.operands[6] as usize;

                                    // Resolve function name from constant pool
                                    let func_name = if let Some(ConstValue::String(s)) =
                                        file.const_pool.get(func_name_idx as usize)
                                    {
                                        s.clone()
                                    } else {
                                        format!("native_{}", func_name_idx)
                                    };

                                    // Parse arguments
                                    let mut args = Vec::new();
                                    for i in 0..arg_count {
                                        if 7 + i * 2 + 1 < instr.operands.len() {
                                            let arg_reg = u16::from_le_bytes([
                                                instr.operands[7 + i * 2],
                                                instr.operands[7 + i * 2 + 1],
                                            ]);
                                            args.push(Reg(arg_reg));
                                        }
                                    }

                                    let dst_reg = Some(Reg(dst));
                                    decoded_instructions.push(BytecodeInstr::CallNative {
                                        dst: dst_reg,
                                        func_name,
                                        args,
                                    });
                                } else {
                                    decoded_instructions.push(BytecodeInstr::Nop);
                                }
                            }
                            Opcode::Return => {
                                decoded_instructions.push(BytecodeInstr::Return);
                            }
                            Opcode::LoadConst => {
                                // LoadConst: dst(1) + const_idx(2)
                                if instr.operands.len() >= 3 {
                                    let dst = instr.operands[0] as u16;
                                    let const_idx =
                                        u16::from_le_bytes([instr.operands[1], instr.operands[2]]);
                                    decoded_instructions.push(BytecodeInstr::LoadConst {
                                        dst: Reg(dst),
                                        const_idx,
                                    });
                                } else {
                                    decoded_instructions.push(BytecodeInstr::Nop);
                                }
                            }
                            Opcode::Mov => {
                                // Mov: dst(1) + src(1)
                                if instr.operands.len() >= 2 {
                                    let dst = instr.operands[0] as u16;
                                    let src = instr.operands[1] as u16;
                                    decoded_instructions.push(BytecodeInstr::Mov {
                                        dst: Reg(dst),
                                        src: Reg(src),
                                    });
                                } else {
                                    decoded_instructions.push(BytecodeInstr::Nop);
                                }
                            }
                            Opcode::LoadLocal => {
                                // LoadLocal: dst(1) + local_idx(1)
                                if instr.operands.len() >= 2 {
                                    let dst = instr.operands[0] as u16;
                                    let local_idx = instr.operands[1];
                                    decoded_instructions.push(BytecodeInstr::LoadLocal {
                                        dst: Reg(dst),
                                        local_idx,
                                    });
                                } else {
                                    decoded_instructions.push(BytecodeInstr::Nop);
                                }
                            }
                            Opcode::StoreLocal => {
                                // StoreLocal: local_idx(1) + src(1)
                                if instr.operands.len() >= 2 {
                                    let local_idx = instr.operands[0];
                                    let src = instr.operands[1] as u16;
                                    decoded_instructions.push(BytecodeInstr::StoreLocal {
                                        local_idx,
                                        src: Reg(src),
                                    });
                                } else {
                                    decoded_instructions.push(BytecodeInstr::Nop);
                                }
                            }
                            Opcode::LoadArg => {
                                // LoadArg: dst(1) + arg_idx(1)
                                if instr.operands.len() >= 2 {
                                    let dst = instr.operands[0] as u16;
                                    let arg_idx = instr.operands[1];
                                    decoded_instructions.push(BytecodeInstr::LoadArg {
                                        dst: Reg(dst),
                                        arg_idx,
                                    });
                                } else {
                                    decoded_instructions.push(BytecodeInstr::Nop);
                                }
                            }
                            Opcode::ReturnValue => {
                                // ReturnValue: value(1) [legacy], or value(2)
                                if instr.operands.len() >= 2 {
                                    let value =
                                        u16::from_le_bytes([instr.operands[0], instr.operands[1]]);
                                    decoded_instructions
                                        .push(BytecodeInstr::ReturnValue { value: Reg(value) });
                                } else if instr.operands.len() == 1 {
                                    let value = instr.operands[0] as u16;
                                    decoded_instructions
                                        .push(BytecodeInstr::ReturnValue { value: Reg(value) });
                                } else {
                                    decoded_instructions.push(BytecodeInstr::Return);
                                }
                            }
                            Opcode::NewListWithCap => {
                                // NewListWithCap: dst(1) + capacity(2)
                                if instr.operands.len() >= 3 {
                                    let dst = instr.operands[0] as u16;
                                    let capacity =
                                        u16::from_le_bytes([instr.operands[1], instr.operands[2]]);
                                    decoded_instructions.push(BytecodeInstr::NewListWithCap {
                                        dst: Reg(dst),
                                        capacity,
                                    });
                                } else {
                                    decoded_instructions.push(BytecodeInstr::Nop);
                                }
                            }
                            Opcode::LoadElement => {
                                // LoadElement: dst(1) + array(1) + index(1)
                                if instr.operands.len() >= 3 {
                                    let dst = instr.operands[0] as u16;
                                    let array = instr.operands[1] as u16;
                                    let index = instr.operands[2] as u16;
                                    decoded_instructions.push(BytecodeInstr::LoadElement {
                                        dst: Reg(dst),
                                        array: Reg(array),
                                        index: Reg(index),
                                    });
                                } else {
                                    decoded_instructions.push(BytecodeInstr::Nop);
                                }
                            }
                            Opcode::CreateStruct => {
                                // CreateStruct: dst(1) + type_name_idx(4) + field_count(1) + fields(2*count)
                                if instr.operands.len() >= 6 {
                                    let dst = instr.operands[0] as u16;
                                    let type_name_idx = u32::from_le_bytes([
                                        instr.operands[1],
                                        instr.operands[2],
                                        instr.operands[3],
                                        instr.operands[4],
                                    ]);
                                    let field_count = instr.operands[5] as usize;

                                    // Resolve type name from constant pool
                                    let type_name = if let Some(ConstValue::String(s)) =
                                        file.const_pool.get(type_name_idx as usize)
                                    {
                                        s.clone()
                                    } else {
                                        format!("struct_{}", type_name_idx)
                                    };

                                    // Parse field registers
                                    let mut fields = Vec::new();
                                    for i in 0..field_count {
                                        if 6 + i * 2 + 1 < instr.operands.len() {
                                            let field_reg = u16::from_le_bytes([
                                                instr.operands[6 + i * 2],
                                                instr.operands[6 + i * 2 + 1],
                                            ]);
                                            fields.push(Reg(field_reg));
                                        }
                                    }

                                    decoded_instructions.push(BytecodeInstr::CreateStruct {
                                        dst: Reg(dst),
                                        type_name,
                                        fields,
                                    });
                                } else {
                                    decoded_instructions.push(BytecodeInstr::Nop);
                                }
                            }
                            Opcode::StoreElement => {
                                // StoreElement: array(1) + index(1) + value(1)
                                if instr.operands.len() >= 3 {
                                    let array = instr.operands[0] as u16;
                                    let index = instr.operands[1] as u16;
                                    let value = instr.operands[2] as u16;
                                    decoded_instructions.push(BytecodeInstr::StoreElement {
                                        array: Reg(array),
                                        index: Reg(index),
                                        value: Reg(value),
                                    });
                                } else {
                                    decoded_instructions.push(BytecodeInstr::Nop);
                                }
                            }
                            Opcode::MakeClosure => {
                                // MakeClosure: dst(1) + func_id(4) + env_count(1) + env_regs(2*count)
                                if instr.operands.len() >= 6 {
                                    let dst = instr.operands[0] as u16;
                                    let func_id = u32::from_le_bytes([
                                        instr.operands[1],
                                        instr.operands[2],
                                        instr.operands[3],
                                        instr.operands[4],
                                    ]);
                                    let env_count = instr.operands[5] as usize;

                                    let mut env = Vec::new();
                                    for i in 0..env_count {
                                        if 6 + i * 2 + 1 < instr.operands.len() {
                                            let env_reg = u16::from_le_bytes([
                                                instr.operands[6 + i * 2],
                                                instr.operands[6 + i * 2 + 1],
                                            ]);
                                            env.push(Reg(env_reg));
                                        }
                                    }

                                    decoded_instructions.push(BytecodeInstr::MakeClosure {
                                        dst: Reg(dst),
                                        func: FunctionRef::Index(func_id),
                                        env,
                                    });
                                } else {
                                    decoded_instructions.push(BytecodeInstr::Nop);
                                }
                            }
                            _ => {
                                // For other opcodes, we need to implement decoding
                                // For now, just use Nop as placeholder
                                decoded_instructions.push(BytecodeInstr::Nop);
                            }
                        }
                    }
                    Err(_) => {
                        // Unknown opcode, use Nop
                        decoded_instructions.push(BytecodeInstr::Nop);
                    }
                }
                ip += 1;
            }

            let byte_func = BytecodeFunction {
                name: func.name,
                params: func.params.into_iter().map(|t| t.into()).collect(),
                return_type: func.return_type.into(),
                local_count: func.local_count,
                upvalue_count: 0, // Not stored in BytecodeFile
                instructions: decoded_instructions,
                labels,                         // Populated from Opcode::Label
                exception_handlers: Vec::new(), // Not implemented yet
            };
            functions.push(byte_func);
        }

        // Determine entry point
        let entry_point = if file.header.entry_point > 0 {
            Some(file.header.entry_point as usize)
        } else if file.header.entry_point == 0 && !functions.is_empty() {
            // If entry_point is 0 but we have functions, use 0 as valid entry
            Some(0)
        } else {
            None
        };

        BytecodeModule {
            name,
            constants: file.const_pool,
            functions,
            type_table: file.type_table.into_iter().map(|t| t.into()).collect(),
            globals: Vec::new(), // Not stored in BytecodeFile yet
            entry_point,
        }
    }
}

/// Convert MonoType to IrType
impl From<crate::frontend::typecheck::MonoType> for IrType {
    fn from(_mono: crate::frontend::typecheck::MonoType) -> Self {
        // Simplified conversion - needs proper implementation
        // For now, return Void type as placeholder
        IrType::Void
    }
}

impl std::fmt::Display for BinaryOp {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            BinaryOp::Add => write!(f, "Add"),
            BinaryOp::Sub => write!(f, "Sub"),
            BinaryOp::Mul => write!(f, "Mul"),
            BinaryOp::Div => write!(f, "Div"),
            BinaryOp::Rem => write!(f, "Rem"),
            BinaryOp::And => write!(f, "And"),
            BinaryOp::Or => write!(f, "Or"),
            BinaryOp::Xor => write!(f, "Xor"),
            BinaryOp::Shl => write!(f, "Shl"),
            BinaryOp::Sar => write!(f, "Sar"),
            BinaryOp::Shr => write!(f, "Shr"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instruction_size() {
        let nop = BytecodeInstr::Nop;
        assert_eq!(nop.size(), 1);

        let mov = BytecodeInstr::Mov {
            dst: Reg(0),
            src: Reg(1),
        };
        assert_eq!(mov.size(), 5);
    }

    #[test]
    fn test_register_display() {
        assert_eq!(format!("{}", Reg(0)), "r0");
        assert_eq!(format!("{}", Reg(15)), "r15");
    }

    #[test]
    fn test_label_display() {
        assert_eq!(format!("{}", Label(0)), "L0");
        assert_eq!(format!("{}", Label(10)), "L10");
    }
}
