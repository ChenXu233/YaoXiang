//! Intermediate Representation

use crate::frontend::parser::ast::Type;
use crate::frontend::typecheck::MonoType;

/// Instruction operand
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Operand {
    Const(ConstValue),
    Local(usize),
    Arg(usize),
    Temp(usize),
    Global(usize),
    Label(usize),
    Register(u8), // Added for codegen
}

/// Instruction
#[derive(Debug, Clone)]
pub enum Instruction {
    Move {
        dst: Operand,
        src: Operand,
    },
    Load {
        dst: Operand,
        src: Operand,
    },
    Store {
        dst: Operand,
        src: Operand,
    },
    Push(Operand),
    Pop(Operand),
    Dup,
    Swap,
    Add {
        dst: Operand,
        lhs: Operand,
        rhs: Operand,
    },
    Sub {
        dst: Operand,
        lhs: Operand,
        rhs: Operand,
    },
    Mul {
        dst: Operand,
        lhs: Operand,
        rhs: Operand,
    },
    Div {
        dst: Operand,
        lhs: Operand,
        rhs: Operand,
    },
    Mod {
        dst: Operand,
        lhs: Operand,
        rhs: Operand,
    },
    Neg {
        dst: Operand,
        src: Operand,
    },
    Eq {
        dst: Operand,
        lhs: Operand,
        rhs: Operand,
    },
    Ne {
        dst: Operand,
        lhs: Operand,
        rhs: Operand,
    },
    Lt {
        dst: Operand,
        lhs: Operand,
        rhs: Operand,
    },
    Le {
        dst: Operand,
        lhs: Operand,
        rhs: Operand,
    },
    Gt {
        dst: Operand,
        lhs: Operand,
        rhs: Operand,
    },
    Ge {
        dst: Operand,
        lhs: Operand,
        rhs: Operand,
    },
    Jmp(usize),
    JmpIf(Operand, usize),
    JmpIfNot(Operand, usize),
    Call {
        dst: Option<Operand>,
        func: Operand,
        args: Vec<Operand>,
    },
    // 注意：根据 RFC-008，await 不是关键字
    // CallAsync 和 Await 指令已移除，由运行时自动处理
    TailCall {
        func: Operand,
        args: Vec<Operand>,
    },
    Ret(Option<Operand>),
    Alloc {
        dst: Operand,
        size: Operand,
    },
    Free(Operand),
    AllocArray {
        dst: Operand,
        size: Operand,
        elem_size: Operand,
    },
    LoadField {
        dst: Operand,
        src: Operand,
        field: usize,
    },
    StoreField {
        dst: Operand,
        field: usize,
        src: Operand,
    },
    LoadIndex {
        dst: Operand,
        src: Operand,
        index: Operand,
    },
    StoreIndex {
        dst: Operand,
        index: Operand,
        src: Operand,
    },
    Cast {
        dst: Operand,
        src: Operand,
        target_type: Type,
    },
    TypeTest(Operand, Type),
    Spawn {
        func: Operand,
    },
    Yield,
    // Phase 5 additions
    HeapAlloc {
        dst: Operand,
        type_id: usize,
    },
    MakeClosure {
        dst: Operand,
        func: usize,
        env: Vec<Operand>,
    },
    /// Drop a value (ownership-based cleanup)
    Drop(Operand),
}

/// Basic block
#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub label: usize,
    pub instructions: Vec<Instruction>,
    pub successors: Vec<usize>,
}

/// Function IR
#[derive(Debug, Clone)]
pub struct FunctionIR {
    pub name: String,
    pub params: Vec<MonoType>,
    pub return_type: MonoType,
    pub is_async: bool,
    pub locals: Vec<MonoType>,
    pub blocks: Vec<BasicBlock>,
    pub entry: usize,
}

impl FunctionIR {
    /// 迭代所有指令
    pub fn all_instructions(&self) -> impl Iterator<Item = &Instruction> {
        self.blocks.iter().flat_map(|block| block.instructions.iter())
    }
}

/// Constant value
#[derive(Debug, Clone)]
pub enum ConstValue {
    Void,
    Bool(bool),
    Int(i128),
    Float(f64),
    Char(char),
    String(String),
    Bytes(Vec<u8>),
}

impl PartialEq for ConstValue {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        match (self, other) {
            (Self::Void, Self::Void) => true,
            (Self::Bool(l0), Self::Bool(r0)) => l0 == r0,
            (Self::Int(l0), Self::Int(r0)) => l0 == r0,
            (Self::Float(l0), Self::Float(r0)) => l0.to_bits() == r0.to_bits(),
            (Self::Char(l0), Self::Char(r0)) => l0 == r0,
            (Self::String(l0), Self::String(r0)) => l0 == r0,
            (Self::Bytes(l0), Self::Bytes(r0)) => l0 == r0,
            _ => false,
        }
    }
}

impl Eq for ConstValue {}

impl std::hash::Hash for ConstValue {
    fn hash<H: std::hash::Hasher>(
        &self,
        state: &mut H,
    ) {
        core::mem::discriminant(self).hash(state);
        match self {
            Self::Void => {}
            Self::Bool(b) => b.hash(state),
            Self::Int(i) => i.hash(state),
            Self::Float(f) => f.to_bits().hash(state),
            Self::Char(c) => c.hash(state),
            Self::String(s) => s.hash(state),
            Self::Bytes(b) => b.hash(state),
        }
    }
}

/// Module IR
#[derive(Debug, Clone, Default)]
pub struct ModuleIR {
    pub types: Vec<Type>,
    pub constants: Vec<ConstValue>,
    pub globals: Vec<(String, Type, Option<ConstValue>)>,
    pub functions: Vec<FunctionIR>,
}
