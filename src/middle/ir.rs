//! Intermediate Representation

use crate::frontend::parser::ast::Type;
use crate::util::span::Span;

/// Instruction operand
#[derive(Debug, Clone)]
pub enum Operand {
    Const(ConstValue),
    Local(usize),
    Arg(usize),
    Temp(usize),
    Global(usize),
    Label(usize),
}

/// Instruction
#[derive(Debug, Clone)]
pub enum Instruction {
    Move { dst: Operand, src: Operand },
    Load { dst: Operand, src: Operand },
    Store { dst: Operand, src: Operand },
    Push(Operand),
    Pop(Operand),
    Dup,
    Swap,
    Add { dst: Operand, lhs: Operand, rhs: Operand },
    Sub { dst: Operand, lhs: Operand, rhs: Operand },
    Mul { dst: Operand, lhs: Operand, rhs: Operand },
    Div { dst: Operand, lhs: Operand, rhs: Operand },
    Mod { dst: Operand, lhs: Operand, rhs: Operand },
    Neg { dst: Operand, src: Operand },
    Cmp { dst: Operand, lhs: Operand, rhs: Operand },
    Jmp(usize),
    JmpIf(Operand, usize),
    JmpIfNot(Operand, usize),
    Call { dst: Option<Operand>, func: Operand, args: Vec<Operand> },
    CallAsync { dst: Operand, func: Operand, args: Vec<Operand> },
    TailCall { func: Operand, args: Vec<Operand> },
    Ret(Option<Operand>),
    Alloc { dst: Operand, size: Operand },
    Free(Operand),
    AllocArray { dst: Operand, size: Operand, elem_size: Operand },
    LoadField { dst: Operand, src: Operand, field: usize },
    StoreField { dst: Operand, field: usize, src: Operand },
    LoadIndex { dst: Operand, src: Operand, index: Operand },
    StoreIndex { dst: Operand, index: Operand, src: Operand },
    Cast { dst: Operand, src: Operand, target_type: Type },
    TypeTest(Operand, Type),
    Spawn { func: Operand },
    Await(Operand),
    Yield,
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
    pub params: Vec<Type>,
    pub return_type: Type,
    pub is_async: bool,
    pub locals: Vec<Type>,
    pub blocks: Vec<BasicBlock>,
    pub entry: usize,
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

/// Module IR
#[derive(Debug, Clone)]
pub struct ModuleIR {
    pub types: Vec<Type>,
    pub constants: Vec<ConstValue>,
    pub globals: Vec<(String, Type, Option<ConstValue>)>,
    pub functions: Vec<FunctionIR>,
}
