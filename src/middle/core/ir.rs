//! Intermediate Representation

pub use crate::frontend::core::parser::ast::Type;
use crate::frontend::core::typecheck::MonoType;
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

/// 任务组：组内任务可并行执行，组间串行
#[derive(Debug, Clone)]
pub struct TaskGroup {
    /// 本组内任务在 closures 列表中的索引
    pub task_indices: Vec<usize>,
}

/// spawn 块的编译期执行计划
///
/// `groups` 定义拓扑排序顺序（组 0 先执行）。
/// `task_deps[i]` = 任务 i 依赖的任务索引列表（硬依赖）。
/// `task_resources[i]` = 任务 i 使用的资源变量名列表。
#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    /// 任务组列表，按拓扑排序顺序排列
    /// 第一组无依赖可立即并行，后续组等待前置组完成
    pub groups: Vec<TaskGroup>,
    /// 每个任务的依赖列表，task_deps\[i\] = 任务 i 依赖的任务索引
    pub task_deps: Vec<Vec<usize>>,
    /// 每个任务使用的资源变量名，task_resources\[i\] = 任务 i 的资源变量名
    pub task_resources: Vec<Vec<String>>,
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
        /// Source span for error reporting
        span: Span,
    },
    Mod {
        dst: Operand,
        lhs: Operand,
        rhs: Operand,
        /// Source span for error reporting
        span: Span,
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
        /// Source span for error reporting
        span: Span,
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
        /// Source span for error reporting
        span: Span,
    },
    /// 动态调用：直接调用寄存器中的函数值（闭包）
    CallDyn {
        dst: Option<Operand>,
        func: Operand,
        args: Vec<Operand>,
        /// Source span for error reporting
        span: Span,
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
        /// Source span for error reporting
        span: Span,
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
        /// Source span for error reporting
        span: Span,
    },
    StoreIndex {
        dst: Operand,
        index: Operand,
        src: Operand,
        /// Source span for error reporting
        span: Span,
    },
    // 注意：迭代器协议已通过 Call 指令实现，无需独立的 IR 指令
    Cast {
        dst: Operand,
        src: Operand,
        target_type: Type,
    },
    TypeTest(Operand, Type),
    /// Spawn a new task (for cycle detection: track args and result)
    Spawn {
        /// 每个直接子表达式对应一个闭包
        closures: Vec<Operand>,
        /// 编译期生成的执行计划
        plan: ExecutionPlan,
        /// spawn 块返回值寄存器
        result: Operand,
    },
    /// 从 List 寄存器动态读取闭包并 spawn（RFC-024 §2.4 spawn for）
    SpawnFromList {
        /// 闭包列表寄存器（运行时动态填充）
        closures_list: Operand,
        /// 编译期生成的执行计划
        plan: ExecutionPlan,
        /// spawn 块返回值寄存器
        result: Operand,
    },
    Yield,
    // Phase 5 additions
    HeapAlloc {
        dst: Operand,
        type_id: usize,
    },
    /// 创建结构体实例
    /// type_name: 结构体类型名
    /// fields: 各字段值的操作数（按字段顺序）
    CreateStruct {
        dst: Operand,
        type_name: String,
        fields: Vec<Operand>,
    },
    /// 创建字典实例
    /// keys: 键的操作数列表
    /// values: 值的操作数列表（与 keys 一一对应）
    NewDict {
        dst: Operand,
        keys: Vec<Operand>,
        values: Vec<Operand>,
    },
    MakeClosure {
        dst: Operand,
        func: String,
        env: Vec<Operand>,
    },
    /// Drop a value (ownership-based cleanup)
    Drop(Operand),
    /// Create Arc (atomic reference count = 1)
    ArcNew {
        dst: Operand,
        src: Operand,
    },
    /// Create Rc (non-atomic reference count = 1)
    RcNew {
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
    /// ShareRef: 将值包装为 Arc 以支持跨任务共享。
    /// 由 `ref` 语法触发，运行时自动选择 Rc/Arc。
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
    pub locals: Vec<MonoType>,
    pub blocks: Vec<BasicBlock>,
    pub entry: usize,
    /// 泛型参数列表
    /// Some(["T", "U"]) 表示泛型函数定义
    /// None 表示普通函数或已特化函数
    pub generic_params: Option<Vec<String>>,
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
    LibraryRef {
        mechanism: String,
        lib: String,
    },
    ExternRef {
        mechanism: String,
        lib: String,
        symbol: String,
    },
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
            (
                Self::LibraryRef {
                    mechanism: l0,
                    lib: l1,
                },
                Self::LibraryRef {
                    mechanism: r0,
                    lib: r1,
                },
            ) => l0 == r0 && l1 == r1,
            (
                Self::ExternRef {
                    mechanism: l0,
                    lib: l1,
                    symbol: l2,
                },
                Self::ExternRef {
                    mechanism: r0,
                    lib: r1,
                    symbol: r2,
                },
            ) => l0 == r0 && l1 == r1 && l2 == r2,
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
            Self::LibraryRef { mechanism, lib } => {
                mechanism.hash(state);
                lib.hash(state);
            }
            Self::ExternRef {
                mechanism,
                lib,
                symbol,
            } => {
                mechanism.hash(state);
                lib.hash(state);
                symbol.hash(state);
            }
        }
    }
}

/// FFI 库绑定 — 编译期链接的外部库
#[derive(Debug, Clone)]
pub struct FfiLibBinding {
    pub id: usize,
    pub mechanism: String,
    pub lib_name: String,
}

/// FFI 绑定 — 不透明类型或外部函数
#[derive(Debug, Clone)]
pub enum FfiBinding {
    /// 不透明类型绑定: SqliteDb: Type = lib("sym")
    TypeBinding {
        type_name: String,
        lib_id: usize,
        symbol: String,
    },
    /// 函数绑定: open: sig = lib("sym")
    FuncBinding {
        func_name: String,
        lib_id: usize,
        symbol: String,
    },
}

/// Module IR
#[derive(Debug, Clone, Default)]
pub struct ModuleIR {
    pub types: Vec<Type>,
    pub globals: Vec<(String, Type, Option<ConstValue>)>,
    pub functions: Vec<FunctionIR>,
    /// 每个函数的可变局部变量索引映射 (function_name -> set of mutable local indices)
    pub mut_locals: std::collections::HashMap<String, std::collections::HashSet<usize>>,
    /// 每个函数的循环绑定变量索引映射 (function_name -> set of loop binding local indices)
    /// 这些变量的 Store 是"绑定"操作，不是"修改"
    pub loop_binding_locals: std::collections::HashMap<String, std::collections::HashSet<usize>>,
    /// 每个函数的局部变量名列表 (function_name -> 变量名列表，按索引顺序)
    pub local_names: std::collections::HashMap<String, Vec<String>>,
    /// FFI 库绑定 — 编译期链接的外部库
    pub ffi_libs: Vec<FfiLibBinding>,
    /// FFI 绑定 — 不透明类型或外部函数
    pub ffi_bindings: Vec<FfiBinding>,
}
