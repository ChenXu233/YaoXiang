//! Intermediate Representation

pub use crate::frontend::core::parser::ast::Type;
use crate::frontend::typecheck::MonoType;
use crate::util::span::Span;

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
        /// Source span for error reporting
        span: Span,
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
    // =====================
    // 位运算指令
    // =====================
    And {
        dst: Operand,
        lhs: Operand,
        rhs: Operand,
    },
    Or {
        dst: Operand,
        lhs: Operand,
        rhs: Operand,
    },
    Xor {
        dst: Operand,
        lhs: Operand,
        rhs: Operand,
    },
    Shl {
        dst: Operand,
        lhs: Operand,
        rhs: Operand,
    },
    Shr {
        dst: Operand,
        lhs: Operand,
        rhs: Operand,
    },
    Sar {
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
    // =====================
    // 虚函数调用指令
    // =====================
    /// 虚方法调用：通过 vtable 查找方法
    /// obj: 对象寄存器
    /// method_name: 方法名
    /// args: 包含 obj 的完整参数列表
    CallVirt {
        dst: Option<Operand>,
        obj: Operand,
        method_name: String,
        args: Vec<Operand>,
    },
    /// 动态调用：直接调用寄存器中的函数值（闭包）
    CallDyn {
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
        /// 结构体类型名（用于字段可变性检查）
        type_name: Option<String>,
        /// 字段名（用于错误信息）
        field_name: Option<String>,
        /// Source span for error reporting
        span: Span,
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
        /// Source span for error reporting
        span: Span,
    },
    Cast {
        dst: Operand,
        src: Operand,
        target_type: Type,
    },
    TypeTest(Operand, Type),
    /// Spawn a new task (for cycle detection: track args and result)
    Spawn {
        func: Operand,
        args: Vec<Operand>,
        result: Operand,
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
    /// Create Arc (atomic reference count = 1)
    ArcNew {
        dst: Operand,
        src: Operand,
    },
    /// Clone Arc (atomic reference count + 1)
    ArcClone {
        dst: Operand,
        src: Operand,
    },
    /// Drop Arc (atomic reference count - 1, free if zero)
    ArcDrop(Operand),
    /// Share reference across threads (requires Sync)
    /// Used for thread-local sharing, requires the type to be Sync
    ShareRef {
        dst: Operand,
        src: Operand,
    },
    // =====================
    // unsafe 块和裸指针指令
    // =====================
    /// Mark the start of an unsafe block
    UnsafeBlockStart,
    /// Mark the end of an unsafe block
    UnsafeBlockEnd,
    /// Create raw pointer from value: ptr = &value
    PtrFromRef {
        dst: Operand,
        src: Operand,
    },
    /// Dereference pointer: value = *ptr
    PtrDeref {
        dst: Operand,
        src: Operand,
    },
    /// Store through pointer: *ptr = value
    PtrStore {
        dst: Operand,
        src: Operand,
    },
    /// Load from pointer: value = *ptr (combined deref and load)
    PtrLoad {
        dst: Operand,
        src: Operand,
    },
    // =====================
    // 字符串指令
    // =====================
    StringLength {
        dst: Operand,
        src: Operand,
    },
    StringConcat {
        dst: Operand,
        lhs: Operand,
        rhs: Operand,
    },
    StringGetChar {
        dst: Operand,
        src: Operand,
        index: Operand,
    },
    StringFromInt {
        dst: Operand,
        src: Operand,
    },
    StringFromFloat {
        dst: Operand,
        src: Operand,
    },
    // =====================
    // 闭包 Upvalue 指令
    // =====================
    LoadUpvalue {
        dst: Operand,
        upvalue_idx: usize,
    },
    StoreUpvalue {
        src: Operand,
        upvalue_idx: usize,
    },
    CloseUpvalue(Operand),
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
        self.blocks
            .iter()
            .flat_map(|block| block.instructions.iter())
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
#[derive(Debug, Clone)]
pub struct ModuleIR {
    pub types: Vec<Type>,
    pub globals: Vec<(String, Type, Option<ConstValue>)>,
    pub functions: Vec<FunctionIR>,
    /// 每个函数的可变局部变量索引映射 (function_name -> set of mutable local indices)
    pub mut_locals: std::collections::HashMap<String, std::collections::HashSet<usize>>,
    /// 每个函数的循环绑定变量索引映射 (function_name -> set of loop binding local indices)
    /// 这些变量的 Store 是"绑定"操作，不是"修改"
    pub loop_binding_locals: std::collections::HashMap<String, std::collections::HashSet<usize>>,
    /// 用户声明的 native 函数绑定 (func_name -> native_symbol)
    ///
    /// 当源码中出现 `my_func: (a: Int) -> Int = Native("symbol")` 时，
    /// IR 生成器会在此记录映射 `"my_func" -> "symbol"`，
    /// 代码生成器会将这些函数名注册为 native，使调用点生成 `CallNative` 指令。
    pub native_bindings: Vec<crate::std::ffi::NativeBinding>,
}

impl Default for ModuleIR {
    fn default() -> Self {
        Self {
            types: Vec::new(),
            globals: Vec::new(),
            functions: Vec::new(),
            mut_locals: std::collections::HashMap::new(),
            loop_binding_locals: std::collections::HashMap::new(),
            native_bindings: Vec::new(),
        }
    }
}
