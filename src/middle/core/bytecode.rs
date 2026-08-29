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
use crate::backends::common::opcode;

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

    /// Yield execution (cooperative scheduling)
    Yield,

    /// Spawn a new concurrent task (dynamic call).
    /// 支持多闭包 + 每任务依赖和资源元数据（RFC-024）
    Spawn {
        dst: Reg,
        closures: Vec<Reg>,
        task_deps: Vec<Vec<u32>>,
        task_resources: Vec<Vec<String>>,
    },

    /// 从 List 寄存器动态读取闭包并 spawn（RFC-024 §2.4 spawn for）
    SpawnFromList {
        dst: Reg,
        closures_list: Reg,
        task_deps: Vec<Vec<u32>>,
        task_resources: Vec<Vec<String>>,
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

    /// 创建字典实例
    NewDict {
        dst: Reg,
        keys: Vec<Reg>,
        values: Vec<Reg>,
    },

    /// 创建元组实例（SPEC §3.6）
    NewTuple {
        dst: Reg,
        items: Vec<Reg>,
    },

    /// 创建 Range 值（#302）：dst(2) + start(2) + end(2) + step(2)
    NewRange {
        dst: Reg,
        start: Reg,
        end: Reg,
        step: Reg,
    },

    /// RFC-011a §6: 包装具体值为存在类型变体
    CreateVariant {
        dst: Reg,
        /// 合成变体类型名在常量池中的索引（"Animal$Group"）
        group_idx: u16,
        /// 变体号（编译期类型收集定序）
        variant: u32,
        payload: Reg,
    },
    /// RFC-011a §6: 提取存在类型值的变体号（守卫：非 group 变体值 → 运行时错误）
    VariantTag {
        dst: Reg,
        obj: Reg,
        group_idx: u16,
    },
    /// RFC-011a §6: 提取存在类型变体的负载（守卫同 VariantTag）
    VariantPayload {
        dst: Reg,
        obj: Reg,
        group_idx: u16,
    },

    /// 定长数组构造：分配 N 个元素、以默认值填充（#299 §2）
    NewArray {
        /// 目标寄存器
        dst: Reg,
        /// 元素数量
        count: u32,
    },
    /// membership 谓词（#299 §3）：dst = elem in container → Bool
    Contains {
        dst: Reg,
        elem: Reg,
        container: Reg,
    },

    // =====================
    // Arc Operations
    // =====================
    ArcNew {
        dst: Reg,
        src: Reg,
    },
    RcNew {
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
    // Borrow Token Operations
    // =====================
    /// Create borrow token (ZST, runtime ≈ Mov)
    Borrow {
        dst: Reg,
        src: Reg,
        mutable: bool,
    },
    /// Release borrow token (ZST, runtime ≈ Nop)
    Release {
        src: Reg,
    },

    // =====================
    // Function Call
    // =====================
    /// Static dispatch call
    CallStatic {
        dst: Option<Reg>,
        /// 函数表索引（codegen 期解析，解释器按 functions_by_id 直接分发）
        func: u32,
        args: Vec<Reg>,
    },

    /// Native function call (FFI)
    CallNative {
        dst: Option<Reg>,
        func_name: String,
        mechanism: String, // "c" or "rs" — FFI mechanism
        lib: String,       // "libsqlite3" — library name
        symbol: String,    // "sqlite3_open" — C symbol
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
        /// 函数表索引（codegen 期解析）
        func: u32,
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
    pub fn opcode(&self) -> u8 {
        match self {
            BytecodeInstr::Nop => opcode::NOP,
            BytecodeInstr::Return => opcode::RETURN,
            BytecodeInstr::ReturnValue { .. } => opcode::RETURN_VALUE,
            BytecodeInstr::Yield => opcode::YIELD,
            BytecodeInstr::Spawn { .. } => opcode::SPAWN,
            BytecodeInstr::SpawnFromList { .. } => opcode::SPAWN_FROM_LIST,
            BytecodeInstr::Jmp { .. } => opcode::JMP,
            BytecodeInstr::JmpIf { .. } => opcode::JMP_IF,
            BytecodeInstr::JmpIfNot { .. } => opcode::JMP_IF_NOT,
            BytecodeInstr::Switch { .. } => opcode::SWITCH,
            BytecodeInstr::Mov { .. } => opcode::MOV,
            BytecodeInstr::LoadConst { .. } => opcode::LOAD_CONST,
            BytecodeInstr::LoadLocal { .. } => opcode::LOAD_LOCAL,
            BytecodeInstr::StoreLocal { .. } => opcode::STORE_LOCAL,
            BytecodeInstr::LoadArg { .. } => opcode::LOAD_ARG,
            BytecodeInstr::BinaryOp { op, .. } => match op {
                BinaryOp::Add => opcode::I64_ADD,
                BinaryOp::Sub => opcode::I64_SUB,
                BinaryOp::Mul => opcode::I64_MUL,
                BinaryOp::Div => opcode::I64_DIV,
                BinaryOp::Rem => opcode::I64_REM,
                BinaryOp::And => opcode::I64_AND,
                BinaryOp::Or => opcode::I64_OR,
                BinaryOp::Xor => opcode::I64_XOR,
                BinaryOp::Shl => opcode::I64_SHL,
                BinaryOp::Sar => opcode::I64_SAR,
                BinaryOp::Shr => opcode::I64_SHR,
            },
            BytecodeInstr::UnaryOp { .. } => opcode::I64_NEG,
            BytecodeInstr::Compare { cmp, .. } => match cmp {
                CompareOp::Eq => opcode::I64_EQ,
                CompareOp::Ne => opcode::I64_NE,
                CompareOp::Lt => opcode::I64_LT,
                CompareOp::Le => opcode::I64_LE,
                CompareOp::Gt => opcode::I64_GT,
                CompareOp::Ge => opcode::I64_GE,
            },
            BytecodeInstr::StackAlloc { .. } => opcode::STACK_ALLOC,
            BytecodeInstr::HeapAlloc { .. } => opcode::HEAP_ALLOC,
            BytecodeInstr::Drop { .. } => opcode::DROP,
            BytecodeInstr::GetField { .. } => opcode::GET_FIELD,
            BytecodeInstr::SetField { .. } => opcode::SET_FIELD,
            BytecodeInstr::LoadElement { .. } => opcode::LOAD_ELEMENT,
            BytecodeInstr::StoreElement { .. } => opcode::STORE_ELEMENT,
            BytecodeInstr::NewListWithCap { .. } => opcode::NEW_LIST_WITH_CAP,
            BytecodeInstr::CreateStruct { .. } => opcode::CREATE_STRUCT,
            BytecodeInstr::NewDict { .. } => opcode::NEW_DICT,
            BytecodeInstr::NewTuple { .. } => opcode::NEW_TUPLE,
            BytecodeInstr::NewRange { .. } => opcode::NEW_RANGE,
            BytecodeInstr::CreateVariant { .. } => opcode::CREATE_VARIANT,
            BytecodeInstr::VariantTag { .. } => opcode::VARIANT_TAG,
            BytecodeInstr::VariantPayload { .. } => opcode::VARIANT_PAYLOAD,
            BytecodeInstr::NewArray { .. } => opcode::NEW_ARRAY,
            BytecodeInstr::Contains { .. } => opcode::CONTAINS,
            BytecodeInstr::ArcNew { .. } => opcode::ARC_NEW,
            BytecodeInstr::RcNew { .. } => opcode::RC_NEW,
            BytecodeInstr::ArcClone { .. } => opcode::ARC_CLONE,
            BytecodeInstr::ArcDrop { .. } => opcode::ARC_DROP,
            BytecodeInstr::WeakNew { .. } => opcode::WEAK_NEW,
            BytecodeInstr::WeakUpgrade { .. } => opcode::WEAK_UPGRADE,
            BytecodeInstr::Borrow { .. } => opcode::BORROW,
            BytecodeInstr::Release { .. } => opcode::RELEASE,
            BytecodeInstr::CallStatic { .. } => opcode::CALL_STATIC,
            BytecodeInstr::CallNative { .. } => opcode::CALL_NATIVE,
            BytecodeInstr::CallVirt { .. } => opcode::CALL_VIRT,
            BytecodeInstr::CallDyn { .. } => opcode::CALL_DYN,
            BytecodeInstr::MakeClosure { .. } => opcode::MAKE_CLOSURE,
            BytecodeInstr::LoadUpvalue { .. } => opcode::LOAD_UPVALUE,
            BytecodeInstr::StoreUpvalue { .. } => opcode::STORE_UPVALUE,
            BytecodeInstr::CloseUpvalue { .. } => opcode::CLOSE_UPVALUE,
            BytecodeInstr::StringLength { .. } => opcode::STRING_LENGTH,
            BytecodeInstr::StringConcat { .. } => opcode::STRING_CONCAT,
            BytecodeInstr::StringEqual { .. } => opcode::STRING_EQUAL,
            BytecodeInstr::StringGetChar { .. } => opcode::STRING_GET_CHAR,
            BytecodeInstr::StringFromInt { .. } => opcode::STRING_FROM_INT,
            BytecodeInstr::StringFromFloat { .. } => opcode::STRING_FROM_FLOAT,
            BytecodeInstr::TryBegin { .. } => opcode::TRY_BEGIN,
            BytecodeInstr::TryEnd => opcode::TRY_END,
            BytecodeInstr::Throw { .. } => opcode::THROW,
            BytecodeInstr::BoundsCheck { .. } => opcode::BOUNDS_CHECK,
            BytecodeInstr::TypeCheck { .. } => opcode::TYPE_CHECK,
            BytecodeInstr::Cast { .. } => opcode::CAST,
            BytecodeInstr::TypeOf { .. } => opcode::TYPE_OF,
        }
    }

    /// Get the instruction size in bytes
    pub fn size(&self) -> usize {
        1 + match self {
            BytecodeInstr::Nop => 0,
            BytecodeInstr::Return => 0,
            BytecodeInstr::ReturnValue { .. } => 2,
            BytecodeInstr::Yield => 0,
            BytecodeInstr::Spawn {
                closures,
                task_deps,
                task_resources,
                ..
            } => {
                // dst(2) + closures.len(4) + closures(2*len)
                // + task_deps.len(4) + for each task: deps.len(4) + deps(4*each)
                // + task_resources.len(4) + for each task: res.len(4) + for each res: str.len(4) + str_bytes
                let mut s = 4 + closures.len() * 2;
                s += 4; // task_deps.len
                for deps in task_deps {
                    s += 4 + deps.len() * 4;
                }
                s += 4; // task_resources.len
                for res in task_resources {
                    s += 4; // res.len
                    for r in res {
                        s += 4 + r.len();
                    }
                }
                s
            }
            BytecodeInstr::SpawnFromList {
                task_deps,
                task_resources,
                ..
            } => {
                // dst(2) + closures_list(2)
                // + task_deps.len(4) + for each task: deps.len(4) + deps(4*each)
                // + task_resources.len(4) + for each task: res.len(4) + for each res: str.len(4) + str_bytes
                let mut s = 2 + 2;
                s += 4; // task_deps.len
                for deps in task_deps {
                    s += 4 + deps.len() * 4;
                }
                s += 4; // task_resources.len
                for res in task_resources {
                    s += 4; // res.len
                    for r in res {
                        s += 4 + r.len();
                    }
                }
                s
            }
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
            BytecodeInstr::NewDict { keys, .. } => {
                // dst(2) + pair_count(4) + keys(2*count) + values(2*count)
                6 + keys.len() * 4
            }
            BytecodeInstr::NewTuple { items, .. } => {
                // dst(2) + item_count(4) + items(2*count)
                6 + items.len() * 2
            }
            BytecodeInstr::NewRange { .. } => {
                // dst(2) + start(2) + end(2) + step(2)
                8
            }
            BytecodeInstr::CreateVariant { .. } => {
                // dst(2) + group_idx(2) + variant(4) + payload(2)
                10
            }
            BytecodeInstr::VariantTag { .. } => {
                // dst(2) + obj(2) + group_idx(2)
                6
            }
            BytecodeInstr::VariantPayload { .. } => 6,
            BytecodeInstr::NewArray { .. } => {
                // dst(1) + count(4) = 5
                5
            }
            BytecodeInstr::Contains { .. } => {
                // dst(1) + elem(1) + container(1) = 3
                3
            }
            BytecodeInstr::ArcNew { .. } => 4,
            BytecodeInstr::RcNew { .. } => 4,
            BytecodeInstr::ArcClone { .. } => 4,
            BytecodeInstr::ArcDrop { .. } => 2,
            BytecodeInstr::WeakNew { .. } => 4,
            BytecodeInstr::WeakUpgrade { .. } => 4,
            BytecodeInstr::Borrow { .. } => 5, // dst(2) + src(2) + mutable(1)
            BytecodeInstr::Release { .. } => 2, // src(2)
            BytecodeInstr::CallStatic { args, .. } => 4 + args.len() * 2,
            BytecodeInstr::CallNative {
                args,
                func_name,
                mechanism,
                ..
            } => {
                let base = 4 + func_name.len() + args.len() * 2;
                if mechanism.is_empty() {
                    base
                } else {
                    base + 12
                }
            }
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
    /// Debug info: mapping from IP to Span
    pub debug_map: HashMap<usize, crate::util::span::DebugSpan>,
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
    /// 编译期类型方法表（type_name → [(裸方法名, 函数表索引)]）。
    /// 解释器加载期直建 vtable 缓存，分发全程按函数表索引。
    pub vtables: Vec<(String, Vec<(String, u32)>)>,
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
            vtables: Vec::new(),
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

/// 从操作数流读取 u16（小端），不足返回 None
fn op_u16(
    operands: &[u8],
    off: usize,
) -> Option<u16> {
    let b = operands.get(off..off + 2)?;
    Some(u16::from_le_bytes([b[0], b[1]]))
}

/// 从操作数流读取 u32（小端），不足返回 None
fn op_u32(
    operands: &[u8],
    off: usize,
) -> Option<u32> {
    let b = operands.get(off..off + 4)?;
    Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
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
            let debug_map = func.debug_map;
            let mut ip = 0;
            while ip < func.instructions.len() {
                let instr = &func.instructions[ip];
                // Decode the instruction based on opcode
                match instr.opcode {
                    opcode::LABEL => {
                        if !instr.operands.is_empty() {
                            let label = op_u32(&instr.operands, 0).unwrap_or(0);
                            labels.insert(Label(label), decoded_instructions.len());
                        }
                    }
                    opcode::JMP => {
                        if !instr.operands.is_empty() {
                            let target = op_u32(&instr.operands, 0).unwrap_or(0);
                            decoded_instructions.push(BytecodeInstr::Jmp {
                                target: Label(target),
                            });
                        }
                    }
                    opcode::JMP_IF => {
                        if instr.operands.len() >= 5 {
                            let cond = instr.operands[0] as u16;
                            let target = op_u32(&instr.operands, 1).unwrap_or(0);
                            decoded_instructions.push(BytecodeInstr::JmpIf {
                                cond: Reg(cond),
                                target: Label(target),
                            });
                        }
                    }
                    opcode::JMP_IF_NOT => {
                        if instr.operands.len() >= 5 {
                            let cond = instr.operands[0] as u16;
                            let target = op_u32(&instr.operands, 1).unwrap_or(0);
                            decoded_instructions.push(BytecodeInstr::JmpIfNot {
                                cond: Reg(cond),
                                target: Label(target),
                            });
                        }
                    }
                    opcode::I64_ADD => {
                        if instr.operands.len() >= 3 {
                            let dst = instr.operands[0] as u16;
                            let lhs = instr.operands[1] as u16;
                            let rhs = instr.operands[2] as u16;
                            decoded_instructions.push(BytecodeInstr::BinaryOp {
                                op: BinaryOp::Add,
                                dst: Reg(dst),
                                lhs: Reg(lhs),
                                rhs: Reg(rhs),
                            });
                        } else {
                            decoded_instructions.push(BytecodeInstr::Nop);
                        }
                    }
                    opcode::I64_SUB => {
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
                    opcode::I64_MUL => {
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
                    opcode::I64_DIV => {
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
                    opcode::I64_REM => {
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
                    opcode::I64_AND => {
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
                    opcode::I64_OR => {
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
                    opcode::I64_XOR => {
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
                    opcode::I64_SHL => {
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
                    opcode::I64_SAR => {
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
                    opcode::I64_SHR => {
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
                    opcode::I64_LT => {
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
                    opcode::I64_LE => {
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
                    opcode::I64_GT => {
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
                    opcode::I64_GE => {
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
                    opcode::I64_NE => {
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
                    opcode::I64_EQ => {
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
                    opcode::I64_NEG => {
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
                    opcode::CALL_STATIC => {
                        // CallStatic: dst(1) + func_id(4) + base_arg_reg(1) + arg_count(1) + args(2*count)
                        if instr.operands.len() >= 7 {
                            let dst = instr.operands[0] as u16;
                            let func_id = op_u32(&instr.operands, 1).unwrap_or(0);
                            let _base_arg_reg = instr.operands[5];
                            let arg_count = instr.operands[6] as usize;

                            // Parse arguments
                            let mut args = Vec::new();
                            for i in 0..arg_count {
                                if 7 + i * 2 + 1 < instr.operands.len() {
                                    let arg_reg = op_u16(&instr.operands, 7 + i * 2).unwrap_or(0);
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
                                func: func_id,
                                args,
                            };
                            decoded_instructions.push(call_instr);
                        } else {
                            // Fallback: push Nop
                            decoded_instructions.push(BytecodeInstr::Nop);
                        }
                    }
                    opcode::CALL_NATIVE => {
                        // CallNative decode: supports old and FFI format
                        // Old:  dst(1) + func_name_idx(4) + base(1) + count(1) + args(2*count)
                        // FFI:  dst(1) + func_name_idx(4) + mech(4) + lib(4) + sym(4) + base(1) + count(1) + args(2*count)
                        if instr.operands.len() >= 7 {
                            let dst = instr.operands[0] as u16;
                            let func_name_idx = op_u32(&instr.operands, 1).unwrap_or(0);

                            // Resolve function name from constant pool
                            let func_name = if let Some(ConstValue::String(s)) =
                                file.const_pool.get(func_name_idx as usize)
                            {
                                s.clone()
                            } else {
                                format!("native_{}", func_name_idx)
                            };

                            // 检查是否有 FFI 元数据（mechanism/lib/symbol 索引）
                            // 如果 operands[6] 作为 arg_count 算出的总量不匹配，说明有额外字段
                            let arg_count_try = instr.operands[6] as usize;
                            let has_ffi_meta = 7 + 2 * arg_count_try != instr.operands.len();

                            let (mechanism, lib, symbol, _base_arg_reg, arg_count, args_start) =
                                if has_ffi_meta {
                                    let mech_idx = op_u32(&instr.operands, 5).unwrap_or(0);
                                    let lib_idx = op_u32(&instr.operands, 9).unwrap_or(0);
                                    let sym_idx = op_u32(&instr.operands, 13).unwrap_or(0);
                                    let mechanism =
                                        resolve_const_string(&file.const_pool, mech_idx as usize);
                                    let lib =
                                        resolve_const_string(&file.const_pool, lib_idx as usize);
                                    let symbol =
                                        resolve_const_string(&file.const_pool, sym_idx as usize);
                                    let _base_arg_reg = instr.operands[17];
                                    let arg_count = instr.operands[18] as usize;
                                    (mechanism, lib, symbol, _base_arg_reg, arg_count, 19)
                                } else {
                                    let _base_arg_reg = instr.operands[5];
                                    let arg_count = arg_count_try;
                                    (
                                        String::new(),
                                        String::new(),
                                        func_name.clone(),
                                        _base_arg_reg,
                                        arg_count,
                                        7,
                                    )
                                };

                            // Parse arguments
                            let mut args = Vec::new();
                            for i in 0..arg_count {
                                if args_start + i * 2 + 1 < instr.operands.len() {
                                    let arg_reg =
                                        op_u16(&instr.operands, args_start + i * 2).unwrap_or(0);
                                    args.push(Reg(arg_reg));
                                }
                            }

                            let dst_reg = Some(Reg(dst));
                            decoded_instructions.push(BytecodeInstr::CallNative {
                                dst: dst_reg,
                                func_name,
                                mechanism,
                                lib,
                                symbol,
                                args,
                            });
                        } else {
                            decoded_instructions.push(BytecodeInstr::Nop);
                        }
                    }
                    opcode::CALL_VIRT => {
                        // CallVirt: dst(1) + obj(1) + method_name_idx(2) + args(1*count) + arg_count(1)
                        if instr.operands.len() >= 5 {
                            let dst = instr.operands[0] as u16;
                            let obj = instr.operands[1] as u16;
                            let method_idx = op_u16(&instr.operands, 2).unwrap_or(0);
                            let arg_count = instr.operands[instr.operands.len() - 1] as usize;
                            let mut args = Vec::with_capacity(arg_count);
                            for i in 0..arg_count {
                                let idx = 4 + i;
                                if idx < instr.operands.len() - 1 {
                                    args.push(Reg(instr.operands[idx] as u16));
                                }
                            }
                            decoded_instructions.push(BytecodeInstr::CallVirt {
                                dst: Some(Reg(dst)),
                                obj: Reg(obj),
                                method_idx,
                                args,
                            });
                        } else {
                            decoded_instructions.push(BytecodeInstr::Nop);
                        }
                    }
                    opcode::CALL_DYN => {
                        // CallDyn: dst(1) + obj(1) + name_idx(2) + args(N) + arg_count(1)
                        if instr.operands.len() >= 5 {
                            let dst = instr.operands[0] as u16;
                            let obj = instr.operands[1] as u16;
                            let _name_idx = op_u16(&instr.operands, 2).unwrap_or(0);
                            let arg_count = instr.operands[instr.operands.len() - 1] as usize;
                            let mut args = Vec::with_capacity(arg_count);
                            for i in 0..arg_count {
                                let idx = 4 + i;
                                if idx < instr.operands.len() - 1 {
                                    args.push(Reg(instr.operands[idx] as u16));
                                }
                            }
                            let dst_reg = Some(Reg(dst));
                            let call_instr = BytecodeInstr::CallDyn {
                                dst: dst_reg,
                                obj: Reg(obj),
                                name_idx: _name_idx,
                                args,
                            };
                            decoded_instructions.push(call_instr);
                        } else {
                            decoded_instructions.push(BytecodeInstr::Nop);
                        }
                    }
                    opcode::RETURN => {
                        decoded_instructions.push(BytecodeInstr::Return);
                    }
                    opcode::YIELD => {
                        decoded_instructions.push(BytecodeInstr::Yield);
                    }
                    opcode::SPAWN => {
                        // Spawn: dst(2) + closures.len(4) + closures(2*len)
                        // + task_deps.len(4) + for each task: deps.len(4) + deps(4*each)
                        // + task_resources.len(4) + for each task: res.len(4) + for each res: str.len(4) + str_bytes
                        if instr.operands.len() >= 8 {
                            let dst = op_u16(&instr.operands, 0).unwrap_or(0);
                            let closures_count = op_u32(&instr.operands, 2).unwrap_or(0) as usize;
                            let mut closures = Vec::with_capacity(closures_count);
                            for i in 0..closures_count {
                                let offset = 6 + i * 2;
                                if offset + 1 < instr.operands.len() {
                                    let reg = op_u16(&instr.operands, offset).unwrap_or(0);
                                    closures.push(Reg(reg));
                                }
                            }
                            let mut pos = 6 + closures_count * 2;
                            // Read task_deps
                            let mut task_deps: Vec<Vec<u32>> = Vec::new();
                            if pos + 3 < instr.operands.len() {
                                let deps_len = op_u32(&instr.operands, pos).unwrap_or(0) as usize;
                                pos += 4;
                                task_deps.reserve(deps_len);
                                for _ in 0..deps_len {
                                    if pos + 3 < instr.operands.len() {
                                        let dep_count =
                                            op_u32(&instr.operands, pos).unwrap_or(0) as usize;
                                        pos += 4;
                                        let mut deps = Vec::with_capacity(dep_count);
                                        for _ in 0..dep_count {
                                            if pos + 3 < instr.operands.len() {
                                                let dep = op_u32(&instr.operands, pos).unwrap_or(0);
                                                deps.push(dep);
                                                pos += 4;
                                            }
                                        }
                                        task_deps.push(deps);
                                    }
                                }
                            }
                            // Read task_resources
                            let mut task_resources: Vec<Vec<String>> = Vec::new();
                            if pos + 3 < instr.operands.len() {
                                let res_len = op_u32(&instr.operands, pos).unwrap_or(0) as usize;
                                pos += 4;
                                task_resources.reserve(res_len);
                                for _ in 0..res_len {
                                    if pos + 3 < instr.operands.len() {
                                        let str_count =
                                            op_u32(&instr.operands, pos).unwrap_or(0) as usize;
                                        pos += 4;
                                        let mut resources = Vec::with_capacity(str_count);
                                        for _ in 0..str_count {
                                            if pos + 3 < instr.operands.len() {
                                                let str_len = op_u32(&instr.operands, pos)
                                                    .unwrap_or(0)
                                                    as usize;
                                                pos += 4;
                                                if pos + str_len <= instr.operands.len() {
                                                    let s = String::from_utf8_lossy(
                                                        &instr.operands[pos..pos + str_len],
                                                    )
                                                    .to_string();
                                                    resources.push(s);
                                                    pos += str_len;
                                                }
                                            }
                                        }
                                        task_resources.push(resources);
                                    }
                                }
                            }
                            decoded_instructions.push(BytecodeInstr::Spawn {
                                dst: Reg(dst),
                                closures,
                                task_deps,
                                task_resources,
                            });
                        } else {
                            decoded_instructions.push(BytecodeInstr::Nop);
                        }
                    }
                    opcode::SPAWN_FROM_LIST => {
                        // SpawnFromList: dst(2) + closures_list(2)
                        // + task_deps.len(4) + for each task: deps.len(4) + deps(4*each)
                        // + task_resources.len(4) + for each task: res.len(4) + for each res: str.len(4) + str_bytes
                        if instr.operands.len() >= 4 {
                            let dst = op_u16(&instr.operands, 0).unwrap_or(0);
                            let closures_list = op_u16(&instr.operands, 2).unwrap_or(0);
                            let mut pos = 4;
                            // Read task_deps
                            let mut task_deps: Vec<Vec<u32>> = Vec::new();
                            if pos + 3 < instr.operands.len() {
                                let deps_len = op_u32(&instr.operands, pos).unwrap_or(0) as usize;
                                pos += 4;
                                task_deps.reserve(deps_len);
                                for _ in 0..deps_len {
                                    if pos + 3 < instr.operands.len() {
                                        let dep_count =
                                            op_u32(&instr.operands, pos).unwrap_or(0) as usize;
                                        pos += 4;
                                        let mut deps = Vec::with_capacity(dep_count);
                                        for _ in 0..dep_count {
                                            if pos + 3 < instr.operands.len() {
                                                let dep = op_u32(&instr.operands, pos).unwrap_or(0);
                                                deps.push(dep);
                                                pos += 4;
                                            }
                                        }
                                        task_deps.push(deps);
                                    }
                                }
                            }
                            // Read task_resources
                            let mut task_resources: Vec<Vec<String>> = Vec::new();
                            if pos + 3 < instr.operands.len() {
                                let res_len = op_u32(&instr.operands, pos).unwrap_or(0) as usize;
                                pos += 4;
                                task_resources.reserve(res_len);
                                for _ in 0..res_len {
                                    if pos + 3 < instr.operands.len() {
                                        let str_count =
                                            op_u32(&instr.operands, pos).unwrap_or(0) as usize;
                                        pos += 4;
                                        let mut resources = Vec::with_capacity(str_count);
                                        for _ in 0..str_count {
                                            if pos + 3 < instr.operands.len() {
                                                let str_len = op_u32(&instr.operands, pos)
                                                    .unwrap_or(0)
                                                    as usize;
                                                pos += 4;
                                                if pos + str_len <= instr.operands.len() {
                                                    let s = String::from_utf8_lossy(
                                                        &instr.operands[pos..pos + str_len],
                                                    )
                                                    .to_string();
                                                    resources.push(s);
                                                    pos += str_len;
                                                }
                                            }
                                        }
                                        task_resources.push(resources);
                                    }
                                }
                            }
                            decoded_instructions.push(BytecodeInstr::SpawnFromList {
                                dst: Reg(dst),
                                closures_list: Reg(closures_list),
                                task_deps,
                                task_resources,
                            });
                        } else {
                            decoded_instructions.push(BytecodeInstr::Nop);
                        }
                    }
                    opcode::LOAD_CONST => {
                        // LoadConst: dst(1) + const_idx(2)
                        if instr.operands.len() >= 3 {
                            let dst = instr.operands[0] as u16;
                            let const_idx = op_u16(&instr.operands, 1).unwrap_or(0);
                            decoded_instructions.push(BytecodeInstr::LoadConst {
                                dst: Reg(dst),
                                const_idx,
                            });
                        } else {
                            decoded_instructions.push(BytecodeInstr::Nop);
                        }
                    }
                    opcode::MOV => {
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
                    opcode::LOAD_LOCAL => {
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
                    opcode::STORE_LOCAL => {
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
                    opcode::LOAD_ARG => {
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
                    opcode::RETURN_VALUE => {
                        // ReturnValue: value(1) [legacy], or value(2)
                        if instr.operands.len() >= 2 {
                            let value = op_u16(&instr.operands, 0).unwrap_or(0);
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
                    opcode::NEW_LIST_WITH_CAP => {
                        // NewListWithCap: dst(1) + capacity(2)
                        if instr.operands.len() >= 3 {
                            let dst = instr.operands[0] as u16;
                            let capacity = op_u16(&instr.operands, 1).unwrap_or(0);
                            decoded_instructions.push(BytecodeInstr::NewListWithCap {
                                dst: Reg(dst),
                                capacity,
                            });
                        } else {
                            decoded_instructions.push(BytecodeInstr::Nop);
                        }
                    }
                    opcode::LOAD_ELEMENT => {
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
                    opcode::CREATE_STRUCT => {
                        // CreateStruct: dst(1) + type_name_idx(4) + field_count(1) + fields(2*count)
                        if instr.operands.len() >= 6 {
                            let dst = instr.operands[0] as u16;
                            let type_name_idx = op_u32(&instr.operands, 1).unwrap_or(0);
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
                                    let field_reg = op_u16(&instr.operands, 6 + i * 2).unwrap_or(0);
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
                    opcode::NEW_DICT => {
                        // NewDict: dst(2) + pair_count(4) + keys(2*count) + values(2*count)
                        if instr.operands.len() >= 6 {
                            let dst = op_u16(&instr.operands, 0).unwrap_or(0);
                            let pair_count = op_u32(&instr.operands, 2).unwrap_or(0) as usize;

                            let mut keys = Vec::with_capacity(pair_count);
                            let mut values = Vec::with_capacity(pair_count);
                            for i in 0..pair_count {
                                let key_offset = 6 + i * 2;
                                let val_offset = 6 + pair_count * 2 + i * 2;
                                if key_offset + 1 < instr.operands.len() {
                                    let key_reg = op_u16(&instr.operands, key_offset).unwrap_or(0);
                                    keys.push(Reg(key_reg));
                                }
                                if val_offset + 1 < instr.operands.len() {
                                    let val_reg = op_u16(&instr.operands, val_offset).unwrap_or(0);
                                    values.push(Reg(val_reg));
                                }
                            }

                            decoded_instructions.push(BytecodeInstr::NewDict {
                                dst: Reg(dst),
                                keys,
                                values,
                            });
                        } else {
                            decoded_instructions.push(BytecodeInstr::Nop);
                        }
                    }
                    opcode::NEW_TUPLE => {
                        // NewTuple: dst(2) + item_count(4) + items(2*count)
                        if instr.operands.len() >= 6 {
                            let dst = op_u16(&instr.operands, 0).unwrap_or(0);
                            let item_count = op_u32(&instr.operands, 2).unwrap_or(0) as usize;

                            let mut items = Vec::with_capacity(item_count);
                            for i in 0..item_count {
                                let item_offset = 6 + i * 2;
                                if item_offset + 1 < instr.operands.len() {
                                    let item_reg =
                                        op_u16(&instr.operands, item_offset).unwrap_or(0);
                                    items.push(Reg(item_reg));
                                }
                            }

                            decoded_instructions.push(BytecodeInstr::NewTuple {
                                dst: Reg(dst),
                                items,
                            });
                        } else {
                            decoded_instructions.push(BytecodeInstr::Nop);
                        }
                    }
                    opcode::NEW_RANGE => {
                        // NewRange: dst(2) + start(2) + end(2) + step(2)
                        if instr.operands.len() >= 8 {
                            let dst = op_u16(&instr.operands, 0).unwrap_or(0);
                            let start = op_u16(&instr.operands, 2).unwrap_or(0);
                            let end = op_u16(&instr.operands, 4).unwrap_or(0);
                            let step = op_u16(&instr.operands, 6).unwrap_or(0);
                            decoded_instructions.push(BytecodeInstr::NewRange {
                                dst: Reg(dst),
                                start: Reg(start),
                                end: Reg(end),
                                step: Reg(step),
                            });
                        } else {
                            decoded_instructions.push(BytecodeInstr::Nop);
                        }
                    }
                    opcode::CREATE_VARIANT => {
                        // CreateVariant: dst(2) + group_idx(2) + variant(4) + payload(2)
                        if instr.operands.len() >= 10 {
                            let dst = op_u16(&instr.operands, 0).unwrap_or(0);
                            let group_idx = op_u16(&instr.operands, 2).unwrap_or(0);
                            let variant = op_u32(&instr.operands, 4).unwrap_or(0);
                            let payload = op_u16(&instr.operands, 8).unwrap_or(0);
                            decoded_instructions.push(BytecodeInstr::CreateVariant {
                                dst: Reg(dst),
                                group_idx,
                                variant,
                                payload: Reg(payload),
                            });
                        } else {
                            decoded_instructions.push(BytecodeInstr::Nop);
                        }
                    }
                    opcode::VARIANT_TAG => {
                        // VariantTag: dst(2) + obj(2) + group_idx(2)
                        if instr.operands.len() >= 6 {
                            decoded_instructions.push(BytecodeInstr::VariantTag {
                                dst: Reg(op_u16(&instr.operands, 0).unwrap_or(0)),
                                obj: Reg(op_u16(&instr.operands, 2).unwrap_or(0)),
                                group_idx: op_u16(&instr.operands, 4).unwrap_or(0),
                            });
                        } else {
                            decoded_instructions.push(BytecodeInstr::Nop);
                        }
                    }
                    opcode::VARIANT_PAYLOAD => {
                        // VariantPayload: dst(2) + obj(2) + group_idx(2)
                        if instr.operands.len() >= 6 {
                            decoded_instructions.push(BytecodeInstr::VariantPayload {
                                dst: Reg(op_u16(&instr.operands, 0).unwrap_or(0)),
                                obj: Reg(op_u16(&instr.operands, 2).unwrap_or(0)),
                                group_idx: op_u16(&instr.operands, 4).unwrap_or(0),
                            });
                        } else {
                            decoded_instructions.push(BytecodeInstr::Nop);
                        }
                    }
                    opcode::NEW_ARRAY => {
                        // NewArray: dst(1) + count(4)
                        if instr.operands.len() >= 5 {
                            let dst = instr.operands[0] as u16;
                            let count = op_u32(&instr.operands, 1).unwrap_or(0);
                            decoded_instructions.push(BytecodeInstr::NewArray {
                                dst: Reg(dst),
                                count,
                            });
                        } else {
                            decoded_instructions.push(BytecodeInstr::Nop);
                        }
                    }
                    opcode::CONTAINS => {
                        // Contains: dst(1) + elem(1) + container(1)
                        if instr.operands.len() >= 3 {
                            decoded_instructions.push(BytecodeInstr::Contains {
                                dst: Reg(instr.operands[0] as u16),
                                elem: Reg(instr.operands[1] as u16),
                                container: Reg(instr.operands[2] as u16),
                            });
                        } else {
                            decoded_instructions.push(BytecodeInstr::Nop);
                        }
                    }
                    opcode::STORE_ELEMENT => {
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
                    opcode::MAKE_CLOSURE => {
                        // MakeClosure: dst(1) + func_id(4) + env_count(1) + env_regs(2*count)
                        if instr.operands.len() >= 6 {
                            let dst = instr.operands[0] as u16;
                            let func_id = op_u32(&instr.operands, 1).unwrap_or(0);
                            let env_count = instr.operands[5] as usize;

                            let mut env = Vec::new();
                            for i in 0..env_count {
                                if 6 + i * 2 + 1 < instr.operands.len() {
                                    let env_reg = op_u16(&instr.operands, 6 + i * 2).unwrap_or(0);
                                    env.push(Reg(env_reg));
                                }
                            }

                            decoded_instructions.push(BytecodeInstr::MakeClosure {
                                dst: Reg(dst),
                                func: func_id,
                                env,
                            });
                        } else {
                            decoded_instructions.push(BytecodeInstr::Nop);
                        }
                    }
                    // #254：LoadUpvalue/StoreUpvalue 解码（此前缺失 → 落 Nop 占位，
                    // 闭包捕获在字节码层静默失效；curry 走 LoadArg 不经此路故未暴露）
                    opcode::LOAD_UPVALUE => {
                        // LoadUpvalue: dst(1) + upvalue_idx(1)
                        if instr.operands.len() >= 2 {
                            let dst = instr.operands[0] as u16;
                            let upvalue_idx = instr.operands[1];
                            decoded_instructions.push(BytecodeInstr::LoadUpvalue {
                                dst: Reg(dst),
                                upvalue_idx,
                            });
                        } else {
                            decoded_instructions.push(BytecodeInstr::Nop);
                        }
                    }
                    opcode::STORE_UPVALUE => {
                        // StoreUpvalue: src(1) + upvalue_idx(1)
                        if instr.operands.len() >= 2 {
                            let src = instr.operands[0] as u16;
                            let upvalue_idx = instr.operands[1];
                            decoded_instructions.push(BytecodeInstr::StoreUpvalue {
                                src: Reg(src),
                                upvalue_idx,
                            });
                        } else {
                            decoded_instructions.push(BytecodeInstr::Nop);
                        }
                    }
                    opcode::BORROW => {
                        // Borrow: dst(2) + src(2) + mutable(1)
                        if instr.operands.len() >= 5 {
                            let dst = op_u16(&instr.operands, 0).unwrap_or(0);
                            let src = op_u16(&instr.operands, 2).unwrap_or(0);
                            let mutable = instr.operands[4] != 0;
                            decoded_instructions.push(BytecodeInstr::Borrow {
                                dst: Reg(dst),
                                src: Reg(src),
                                mutable,
                            });
                        } else {
                            decoded_instructions.push(BytecodeInstr::Nop);
                        }
                    }
                    opcode::RELEASE => {
                        // Release: src(2)
                        if instr.operands.len() >= 2 {
                            let src = op_u16(&instr.operands, 0).unwrap_or(0);
                            decoded_instructions.push(BytecodeInstr::Release { src: Reg(src) });
                        } else {
                            decoded_instructions.push(BytecodeInstr::Nop);
                        }
                    }
                    opcode::GET_FIELD => {
                        // GetField: dst(1) + src(1) + field_idx(2)
                        if instr.operands.len() >= 4 {
                            let dst = instr.operands[0] as u16;
                            let src = instr.operands[1] as u16;
                            let field_idx = op_u16(&instr.operands, 2).unwrap_or(0);
                            decoded_instructions.push(BytecodeInstr::GetField {
                                dst: Reg(dst),
                                src: Reg(src),
                                field_idx,
                            });
                        } else {
                            decoded_instructions.push(BytecodeInstr::Nop);
                        }
                    }
                    opcode::SET_FIELD => {
                        // SetField: src(1) + field_idx(2) + value(1)
                        if instr.operands.len() >= 4 {
                            let src = instr.operands[0] as u16;
                            let field_idx = op_u16(&instr.operands, 1).unwrap_or(0);
                            let value = instr.operands[3] as u16;
                            decoded_instructions.push(BytecodeInstr::SetField {
                                src: Reg(src),
                                field_idx,
                                value: Reg(value),
                            });
                        } else {
                            decoded_instructions.push(BytecodeInstr::Nop);
                        }
                    }
                    _ => {
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
                labels,                         // Populated from opcode::LABEL
                exception_handlers: Vec::new(), // Not implemented yet
                debug_map,
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
            vtables: file.vtables,
            globals: Vec::new(), // Not stored in BytecodeFile yet
            entry_point,
        }
    }
}

/// Convert MonoType to IrType (ast::Type)
impl From<crate::frontend::core::typecheck::MonoType> for IrType {
    fn from(ty: crate::frontend::core::typecheck::MonoType) -> Self {
        use crate::frontend::core::typecheck::MonoType;
        match ty {
            MonoType::Int(w) => IrType::Int(w),
            MonoType::Float(w) => IrType::Float(w),
            MonoType::Bool => IrType::Bool,
            MonoType::Char => IrType::Char,
            MonoType::Void => IrType::Void,
            MonoType::Fn {
                params,
                return_type,
                ..
            } => IrType::Fn {
                params: params.into_iter().map(|t| t.into()).collect(),
                return_type: Box::new((*return_type).into()),
            },
            // #299：String/Bytes/Tuple/List/Dict/Option/Result/Range/Arc/Weak 现为 Generic 表示
            MonoType::Generic { name, args } => match name.as_str() {
                "String" => IrType::String,
                "Bytes" => IrType::Bytes,
                "Tuple" => IrType::Tuple(args.clone().into_iter().map(|t| t.into()).collect()),
                _ => IrType::Void,
            },
            // Struct, Enum, Ref, TypeVar, TypeRef, Union, Intersection, AssocType — unresolved or no IR form
            MonoType::Struct(_)
            | MonoType::Enum(_)
            | MonoType::Ref { .. }
            | MonoType::TypeVar(_)
            | MonoType::TypeRef(_)
            | MonoType::Union(_)
            | MonoType::Intersection(_)
            | MonoType::AssocType { .. } => IrType::Void,
            // 残留旧变体（Task 1.7 删除枚举变体前的过渡分支）
            _ => IrType::Void,
        }
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
/// 从常量池解析字符串
fn resolve_const_string(
    pool: &[ConstValue],
    idx: usize,
) -> String {
    if let Some(ConstValue::String(s)) = pool.get(idx) {
        s.clone()
    } else {
        String::new()
    }
}
