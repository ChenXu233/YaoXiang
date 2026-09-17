//! Intermediate Representation

pub use crate::frontend::core::parser::ast::Type;
use std::collections::HashMap;

use crate::frontend::core::typecheck::MonoType;
use crate::frontend::module::symbol::DefId;
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
        span: Span,
    },
    Load {
        dst: Operand,
        src: Operand,
        span: Span,
    },
    Store {
        dst: Operand,
        src: Operand,
        /// Source span for error reporting
        span: Span,
    },
    Push {
        src: Operand,
        span: Span,
    },
    Pop {
        dst: Operand,
        span: Span,
    },
    Dup {
        span: Span,
    },
    Swap {
        span: Span,
    },
    Add {
        dst: Operand,
        lhs: Operand,
        rhs: Operand,
        span: Span,
    },
    Sub {
        dst: Operand,
        lhs: Operand,
        rhs: Operand,
        span: Span,
    },
    Mul {
        dst: Operand,
        lhs: Operand,
        rhs: Operand,
        span: Span,
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
        span: Span,
    },
    Or {
        dst: Operand,
        lhs: Operand,
        rhs: Operand,
        span: Span,
    },
    Xor {
        dst: Operand,
        lhs: Operand,
        rhs: Operand,
        span: Span,
    },
    Shl {
        dst: Operand,
        lhs: Operand,
        rhs: Operand,
        span: Span,
    },
    Shr {
        dst: Operand,
        lhs: Operand,
        rhs: Operand,
        span: Span,
    },
    Sar {
        dst: Operand,
        lhs: Operand,
        rhs: Operand,
        span: Span,
    },
    Neg {
        dst: Operand,
        src: Operand,
        span: Span,
    },
    Not {
        dst: Operand,
        src: Operand,
        span: Span,
    },
    Eq {
        dst: Operand,
        lhs: Operand,
        rhs: Operand,
        span: Span,
    },
    Ne {
        dst: Operand,
        lhs: Operand,
        rhs: Operand,
        span: Span,
    },
    Lt {
        dst: Operand,
        lhs: Operand,
        rhs: Operand,
        span: Span,
    },
    Le {
        dst: Operand,
        lhs: Operand,
        rhs: Operand,
        span: Span,
    },
    Gt {
        dst: Operand,
        lhs: Operand,
        rhs: Operand,
        span: Span,
    },
    Ge {
        dst: Operand,
        lhs: Operand,
        rhs: Operand,
        span: Span,
    },
    Jmp {
        target: usize,
        span: Span,
    },
    JmpIf {
        cond: Operand,
        target: usize,
        span: Span,
    },
    JmpIfNot {
        cond: Operand,
        target: usize,
        span: Span,
    },
    Call {
        dst: Option<Operand>,
        func: Operand,
        args: Vec<Operand>,
        /// 静态调用目标的绑定身份（字节码函数为 Some；std 原生/外部名为 None，按名走 FFI）
        def: Option<DefId>,
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
        /// 静态调用目标的绑定身份（同 `Call::def`）
        def: Option<DefId>,
        span: Span,
    },
    Ret {
        value: Option<Operand>,
        span: Span,
    },
    Alloc {
        dst: Operand,
        size: Operand,
        span: Span,
    },
    Free {
        src: Operand,
        span: Span,
    },
    AllocArray {
        dst: Operand,
        size: Operand,
        elem_size: Operand,
        span: Span,
    },
    /// 定长数组构造（#299 §2）：分配 N 个 Void 占位元素，
    /// 由字面量上下文落点生成——仅当 Array(T,N) 注解直接作用于 List 字面量
    AllocFixedArray {
        dst: Operand,
        count: usize,
        span: Span,
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
    /// membership 谓词（#299 §3）：`dst = elem in container` → Bool
    /// 容器：List/Array/Tuple 线性扫、Dict 键查、String 子串；
    /// Range 不走此指令——ir_gen 直接脱糖成比较链
    Contains {
        dst: Operand,
        elem: Operand,
        container: Operand,
        span: Span,
    },
    // 注意：迭代器协议已通过 Call 指令实现，无需独立的 IR 指令
    Cast {
        dst: Operand,
        src: Operand,
        target_type: Type,
        span: Span,
    },
    TypeTest {
        src: Operand,
        ty: Type,
        span: Span,
    },
    /// Spawn a new task (for cycle detection: track args and result)
    Spawn {
        /// 每个直接子表达式对应一个闭包
        closures: Vec<Operand>,
        /// 编译期生成的执行计划
        plan: ExecutionPlan,
        /// spawn 块返回值寄存器
        result: Operand,
        span: Span,
    },
    /// 从 List 寄存器动态读取闭包并 spawn（RFC-024 §2.4 spawn for）
    SpawnFromList {
        /// 闭包列表寄存器（运行时动态填充）
        closures_list: Operand,
        /// 编译期生成的执行计划
        plan: ExecutionPlan,
        /// spawn 块返回值寄存器
        result: Operand,
        span: Span,
    },
    Yield {
        span: Span,
    },
    // Phase 5 additions
    HeapAlloc {
        dst: Operand,
        type_id: usize,
        span: Span,
    },
    /// 创建结构体实例
    /// type_name: 结构体类型名
    /// fields: 各字段值的操作数（按字段顺序）
    CreateStruct {
        dst: Operand,
        type_name: String,
        fields: Vec<Operand>,
        span: Span,
    },
    /// 创建字典实例
    /// keys: 键的操作数列表
    /// values: 值的操作数列表（与 keys 一一对应）
    NewDict {
        dst: Operand,
        keys: Vec<Operand>,
        values: Vec<Operand>,
        span: Span,
    },
    /// 创建元组实例（SPEC §3.6）
    /// items: 各元素的操作数列表（按元素顺序）
    NewTuple {
        dst: Operand,
        items: Vec<Operand>,
        span: Span,
    },
    /// 创建 Range 值（#302）：三标量不可变记录，正式运行时身份
    NewRange {
        dst: Operand,
        start: Operand,
        end: Operand,
        step: Operand,
        span: Span,
    },
    /// RFC-011a §6: 包装具体值为存在类型变体（Animal$Group.Dog(payload)）。
    /// group: 合成变体类型名（接口名 + "$Group"）；variant: 编译期类型收集定序的变体号
    CreateVariant {
        dst: Operand,
        group: String,
        variant: u32,
        payload: Operand,
        span: Span,
    },
    /// RFC-011a §6: 提取存在类型值的变体号（Int）。守卫：obj 必须是 group 的
    /// 变体值，否则运行时报错——漏包装在运行时守卫层显式暴露，绝不静默错数据
    VariantTag {
        dst: Operand,
        obj: Operand,
        group: String,
        span: Span,
    },
    /// RFC-011a §6: 提取存在类型变体的负载（守卫同 VariantTag）
    VariantPayload {
        dst: Operand,
        obj: Operand,
        group: String,
        span: Span,
    },
    MakeClosure {
        dst: Operand,
        func: String,
        /// 闭包目标函数的绑定身份（生成期 intern，必有值；测试手工构造可为 None）
        def: Option<DefId>,
        env: Vec<Operand>,
        span: Span,
    },
    /// Drop a value (ownership-based cleanup)
    Drop {
        src: Operand,
        span: Span,
    },
    /// Create Arc (atomic reference count = 1)
    ArcNew {
        dst: Operand,
        src: Operand,
        span: Span,
    },
    /// Create Rc (non-atomic reference count = 1)
    RcNew {
        dst: Operand,
        src: Operand,
        span: Span,
    },
    /// Clone Arc (atomic reference count + 1)
    ArcClone {
        dst: Operand,
        src: Operand,
        span: Span,
    },
    /// Drop Arc (atomic reference count - 1, free if zero)
    ArcDrop {
        src: Operand,
        span: Span,
    },
    // =====================
    // unsafe 块和裸指针指令
    // =====================
    /// Mark the start of an unsafe block
    UnsafeBlockStart {
        span: Span,
    },
    /// Mark the end of an unsafe block
    UnsafeBlockEnd {
        span: Span,
    },
    /// Create raw pointer from value: ptr = &value
    PtrFromRef {
        dst: Operand,
        src: Operand,
        span: Span,
    },
    /// Dereference pointer: value = *ptr
    PtrDeref {
        dst: Operand,
        src: Operand,
        span: Span,
    },
    /// Store through pointer: *ptr = value
    PtrStore {
        dst: Operand,
        src: Operand,
        span: Span,
    },
    /// Load from pointer: value = *ptr (combined deref and load)
    PtrLoad {
        dst: Operand,
        src: Operand,
        span: Span,
    },
    // =====================
    // 字符串指令
    // =====================
    StringLength {
        dst: Operand,
        src: Operand,
        span: Span,
    },
    StringConcat {
        dst: Operand,
        lhs: Operand,
        rhs: Operand,
        span: Span,
    },
    StringGetChar {
        dst: Operand,
        src: Operand,
        index: Operand,
        span: Span,
    },
    StringFromInt {
        dst: Operand,
        src: Operand,
        span: Span,
    },
    StringFromFloat {
        dst: Operand,
        src: Operand,
        span: Span,
    },
    // =====================
    // 闭包 Upvalue 指令
    // =====================
    LoadUpvalue {
        dst: Operand,
        upvalue_idx: usize,
        span: Span,
    },
    StoreUpvalue {
        src: Operand,
        upvalue_idx: usize,
        span: Span,
    },
    CloseUpvalue {
        src: Operand,
        span: Span,
    },
}

impl Instruction {
    /// 指令的源码位置。
    ///
    /// 刻意**不设通配臂**：新增变体若忘了带 span，此处编译失败，
    /// 而不是静默退化到"无位置"。这是位置信息可信的唯一强制手段。
    pub fn span(&self) -> Span {
        match self {
            Instruction::Move { span, .. } => *span,
            Instruction::Load { span, .. } => *span,
            Instruction::Store { span, .. } => *span,
            Instruction::Push { span, .. } => *span,
            Instruction::Pop { span, .. } => *span,
            Instruction::Dup { span, .. } => *span,
            Instruction::Swap { span, .. } => *span,
            Instruction::Add { span, .. } => *span,
            Instruction::Sub { span, .. } => *span,
            Instruction::Mul { span, .. } => *span,
            Instruction::Div { span, .. } => *span,
            Instruction::Mod { span, .. } => *span,
            Instruction::And { span, .. } => *span,
            Instruction::Or { span, .. } => *span,
            Instruction::Xor { span, .. } => *span,
            Instruction::Shl { span, .. } => *span,
            Instruction::Shr { span, .. } => *span,
            Instruction::Sar { span, .. } => *span,
            Instruction::Neg { span, .. } => *span,
            Instruction::Not { span, .. } => *span,
            Instruction::Eq { span, .. } => *span,
            Instruction::Ne { span, .. } => *span,
            Instruction::Lt { span, .. } => *span,
            Instruction::Le { span, .. } => *span,
            Instruction::Gt { span, .. } => *span,
            Instruction::Ge { span, .. } => *span,
            Instruction::Jmp { span, .. } => *span,
            Instruction::JmpIf { span, .. } => *span,
            Instruction::JmpIfNot { span, .. } => *span,
            Instruction::Call { span, .. } => *span,
            Instruction::CallVirt { span, .. } => *span,
            Instruction::CallDyn { span, .. } => *span,
            Instruction::TailCall { span, .. } => *span,
            Instruction::Ret { span, .. } => *span,
            Instruction::Alloc { span, .. } => *span,
            Instruction::Free { span, .. } => *span,
            Instruction::AllocArray { span, .. } => *span,
            Instruction::AllocFixedArray { span, .. } => *span,
            Instruction::LoadField { span, .. } => *span,
            Instruction::StoreField { span, .. } => *span,
            Instruction::LoadIndex { span, .. } => *span,
            Instruction::StoreIndex { span, .. } => *span,
            Instruction::Contains { span, .. } => *span,
            Instruction::Cast { span, .. } => *span,
            Instruction::TypeTest { span, .. } => *span,
            Instruction::Spawn { span, .. } => *span,
            Instruction::SpawnFromList { span, .. } => *span,
            Instruction::Yield { span, .. } => *span,
            Instruction::HeapAlloc { span, .. } => *span,
            Instruction::CreateStruct { span, .. } => *span,
            Instruction::NewDict { span, .. } => *span,
            Instruction::NewTuple { span, .. } => *span,
            Instruction::NewRange { span, .. } => *span,
            Instruction::CreateVariant { span, .. } => *span,
            Instruction::VariantTag { span, .. } => *span,
            Instruction::VariantPayload { span, .. } => *span,
            Instruction::MakeClosure { span, .. } => *span,
            Instruction::Drop { span, .. } => *span,
            Instruction::ArcNew { span, .. } => *span,
            Instruction::RcNew { span, .. } => *span,
            Instruction::ArcClone { span, .. } => *span,
            Instruction::ArcDrop { span, .. } => *span,
            Instruction::UnsafeBlockStart { span, .. } => *span,
            Instruction::UnsafeBlockEnd { span, .. } => *span,
            Instruction::PtrFromRef { span, .. } => *span,
            Instruction::PtrDeref { span, .. } => *span,
            Instruction::PtrStore { span, .. } => *span,
            Instruction::PtrLoad { span, .. } => *span,
            Instruction::StringLength { span, .. } => *span,
            Instruction::StringConcat { span, .. } => *span,
            Instruction::StringGetChar { span, .. } => *span,
            Instruction::StringFromInt { span, .. } => *span,
            Instruction::StringFromFloat { span, .. } => *span,
            Instruction::LoadUpvalue { span, .. } => *span,
            Instruction::StoreUpvalue { span, .. } => *span,
            Instruction::CloseUpvalue { span, .. } => *span,
        }
    }
}

/// Basic block
#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub label: usize,
    pub instructions: Vec<Instruction>,
    pub successors: Vec<usize>,
}

/// 函数体形态
///
/// RFC-010 §328：`{}` 在 YaoXiang 中是依赖驱动计算单元。
/// 当返回类型是 `Type` 时，`{}` 是类型字面量（字段/变体列表）；
/// 当返回类型是值类型时，`{}` 是代码块（指令序列）。
#[derive(Debug, Clone)]
pub enum FunctionBody {
    /// 返回值是值 → {} 是代码块
    Code {
        blocks: Vec<BasicBlock>,
        entry: usize,
        locals: Vec<LocalSlot>,
    },
    /// 返回值是类型 → {} 是类型字面量
    TypeDecl { definition: Type },
}

/// 局部变量槽位：类型 + 源码名 + 作用域深度。
///
/// `name` 为 `None` 表示编译器内部临时寄存器（无对应源码变量）。
/// 用 `Option` 而非空串：调用方必须显式处理“无名字”，
/// 避免 `""` 被当成合法名字传播到诊断里。
#[derive(Debug, Clone)]
pub struct LocalSlot {
    /// 源码变量名；临时寄存器为 None
    pub name: Option<String>,
    pub ty: MonoType,
    /// 作用域深度（0 = 函数参数层），供嵌套作用域重名消歧
    pub scope_depth: usize,
}

impl LocalSlot {
    /// 无名槽位（编译器临时寄存器）
    pub fn temp(ty: MonoType) -> Self {
        Self {
            name: None,
            ty,
            scope_depth: 0,
        }
    }
}

/// Function IR
#[derive(Debug, Clone)]
pub struct FunctionIR {
    pub name: String,
    /// 绑定身份（生成期经 SymbolTable intern；测试手工构造可为 None）
    pub def: Option<DefId>,
    pub params: Vec<MonoType>,
    pub return_type: MonoType,
    pub body: FunctionBody,
    /// 泛型参数列表
    /// Some(["T", "U"]) 表示泛型函数定义
    /// None 表示普通函数或已特化函数
    pub generic_params: Option<Vec<String>>,
}

impl FunctionIR {
    /// 迭代所有指令（TypeDecl 返回空迭代器）
    pub fn all_instructions(&self) -> Box<dyn Iterator<Item = &Instruction> + '_> {
        match &self.body {
            FunctionBody::Code { blocks, .. } => {
                Box::new(blocks.iter().flat_map(|block| block.instructions.iter()))
            }
            FunctionBody::TypeDecl { .. } => Box::new(std::iter::empty()),
        }
    }

    /// 获取基本块列表（仅 Code 体有效）
    pub fn blocks(&self) -> &[BasicBlock] {
        match &self.body {
            FunctionBody::Code { blocks, .. } => blocks,
            FunctionBody::TypeDecl { .. } => &[],
        }
    }

    /// 获取可变基本块列表（仅 Code 体有效）
    pub fn blocks_mut(&mut self) -> &mut Vec<BasicBlock> {
        match &mut self.body {
            FunctionBody::Code { blocks, .. } => blocks,
            FunctionBody::TypeDecl { .. } => panic!("blocks_mut called on TypeDecl"),
        }
    }

    /// 获取局部变量槽位（仅 Code 体有效）
    pub fn locals(&self) -> &[LocalSlot] {
        match &self.body {
            FunctionBody::Code { locals, .. } => locals,
            FunctionBody::TypeDecl { .. } => &[],
        }
    }

    /// 获取入口基本块索引（仅 Code 体有效）
    pub fn entry(&self) -> usize {
        match &self.body {
            FunctionBody::Code { entry, .. } => *entry,
            FunctionBody::TypeDecl { .. } => 0,
        }
    }

    /// 判断是否是类型定义（而非代码函数）
    pub fn is_type_decl(&self) -> bool {
        matches!(self.body, FunctionBody::TypeDecl { .. })
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

/// 模块级全局槽位（顶层绑定）。
///
/// 顶层绑定编译为「一个槽位 + 一条初始化指令」：初始化指令在模块初始化
/// 序列中运行时求值并写入槽位，读取处走 `Operand::Global(idx)`。
#[derive(Debug, Clone)]
pub struct GlobalSlot {
    /// 绑定名（多文件模式下为限定名 `{module}.{name}`）
    pub name: String,
    /// 绑定类型
    pub ty: MonoType,
    /// 绝对槽位号（`Operand::Global(index)` 的值）。
    /// 多文件模式下由编排器预先分配，各文件共享同一布局，故跨文件引用
    /// 与定义方用同一个索引；单文件模式下即本文件内的声明顺序。
    pub index: usize,
}

/// Module IR
#[derive(Debug, Clone, Default)]
pub struct ModuleIR {
    /// 模块级全局槽位（顶层绑定）。索引即 `Operand::Global(idx)` 的槽位号。
    pub globals: Vec<GlobalSlot>,
    pub functions: Vec<FunctionIR>,
    /// 模块初始化序列：按依赖顺序求值并写入全局槽位的指令。
    /// 入口执行前先跑它（Script 模式下它本身就是程序）。
    pub init: Vec<Instruction>,
    /// FFI 库绑定 — 编译期链接的外部库
    pub ffi_libs: Vec<FfiLibBinding>,
    /// FFI 绑定 — 不透明类型或外部函数
    pub ffi_bindings: Vec<FfiBinding>,
    /// RFC-029: 入口函数的完整限定名（多文件模式）。None 时回退到查找 "main"。
    pub entry_function: Option<String>,
    /// 多文件模式：源文件路径列表（orchestrator 发现顺序），索引即 debug span 的
    /// file_id。单文件模式为空（translator 固定 file_id 0，由 CLI 装入伤口）。
    pub source_files: Vec<String>,
    /// 多文件模式：函数名 → source_files 索引（链接时按来源模块记录，#252）。
    /// mono 特化的新函数名不在表中，查询回退 0。
    pub function_files: HashMap<String, usize>,
}
