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
use crate::backends::common::Opcode;

// Re-export types for conversion
pub use crate::middle::ir::{Type as IrType, ConstValue};

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

    // =====================
    // Function Call
    // =====================
    /// Static dispatch call
    CallStatic {
        dst: Option<Reg>,
        func: FunctionRef,
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
            BytecodeInstr::ArcNew { .. } => Opcode::ArcNew,
            BytecodeInstr::ArcClone { .. } => Opcode::ArcClone,
            BytecodeInstr::ArcDrop { .. } => Opcode::ArcDrop,
            BytecodeInstr::CallStatic { .. } => Opcode::CallStatic,
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
            BytecodeInstr::ArcNew { .. } => 4,
            BytecodeInstr::ArcClone { .. } => 4,
            BytecodeInstr::ArcDrop { .. } => 2,
            BytecodeInstr::CallStatic { args, .. } => 4 + args.len() * 2,
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
    pub params: Vec<super::super::middle::ir::Type>,
    /// Return type
    pub return_type: super::super::middle::ir::Type,
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
    pub type_table: Vec<super::super::middle::ir::Type>,
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

impl From<crate::middle::codegen::bytecode::BytecodeFile> for BytecodeModule {
    fn from(file: crate::middle::codegen::bytecode::BytecodeFile) -> Self {
        let name = "main".to_string(); // Default module name

        // Convert functions
        let mut functions = Vec::new();
        for func in file.code_section.functions {
            let byte_func = BytecodeFunction {
                name: func.name,
                params: func.params.into_iter().map(|t| t.into()).collect(),
                return_type: func.return_type.into(),
                local_count: func.local_count,
                upvalue_count: 0,               // Not stored in BytecodeFile
                instructions: Vec::new(),       // Will be populated during execution
                labels: HashMap::new(),         // Will be populated during execution
                exception_handlers: Vec::new(), // Not implemented yet
            };
            functions.push(byte_func);
        }

        BytecodeModule {
            name,
            constants: file.const_pool,
            functions,
            type_table: file.type_table.into_iter().map(|t| t.into()).collect(),
            globals: Vec::new(), // Not stored in BytecodeFile yet
            entry_point: if file.header.entry_point > 0 {
                Some(file.header.entry_point as usize)
            } else {
                None
            },
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
