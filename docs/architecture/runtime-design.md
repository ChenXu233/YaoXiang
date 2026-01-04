# YaoXiang 运行时设计文档

> 版本：v2.0.0
> 状态：正式
> 作者：沫郁酱
> 日期：2025-01-04

---

## 目录

1. [概述](#一概述)
2. [虚拟机设计](#二虚拟机设计)
3. [并作模型](#三并作模型)
4. [任务调度器](#四任务调度器)
5. [内存管理](#五内存管理)
6. [内联缓存](#六内联缓存)
7. [性能优化](#七性能优化)
8. [错误处理](#八错误处理)

---

## 一、概述

YaoXiang 运行时系统负责执行编译生成的字节码，并提供程序运行所需的基础设施。运行时设计的核心目标是：

- **高效执行**：优化的字节码解释器，最大化执行性能
- **安全并发**：无数据竞争的任务调度，支持并作模型
- **内存安全**：自动内存管理，防止内存泄漏和悬挂指针
- **可观测性**：提供运行时监控和诊断能力

### 运行时组件总览

```
┌─────────────────────────────────────────────────────────┐
│                     运行时 (Runtime)                     │
├─────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐  │
│  │   虚拟机    │  │  调度器     │  │    内存管理     │  │
│  │  Executor  │  │ Scheduler  │  │    Memory      │  │
│  └──────┬──────┘  └──────┬──────┘  └────────┬────────┘  │
│         │                │                   │           │
│         └────────────────┼───────────────────┘           │
│                          │                               │
│                   ┌──────┴──────┐                        │
│                   │   并作图    │                        │
│                   │    DAG      │                        │
│                   └─────────────┘                        │
└─────────────────────────────────────────────────────────┘
```

### 核心文件结构

```
src/runtime/
├── mod.rs                  # 运行时模块入口
├── dag/                    # 并作图（DAG）
│   ├── mod.rs              # DAG 模块入口
│   ├── node_id.rs          # 节点 ID 管理
│   ├── node.rs             # 节点定义
│   ├── graph.rs            # 图操作
│   └── tests/
├── scheduler/              # 任务调度器
│   ├── mod.rs              # 调度器入口
│   ├── task.rs             # 任务定义
│   ├── queue.rs            # 任务队列
│   ├── work_stealer.rs     # 工作窃取算法
│   └── tests/
└── memory/                 # 内存管理
    ├── mod.rs              # 内存管理入口
    └── tests/

src/vm/
├── mod.rs                  # 虚拟机入口
├── executor.rs             # 执行器
├── frames.rs               # 调用帧
├── instructions.rs         # 指令集
├── opcode.rs               # 操作码定义
├── inline_cache.rs         # 内联缓存
├── errors.rs               # VM 错误
└── tests/
```

---

## 二、虚拟机设计

### 2.1 虚拟机架构

**核心文件**：`src/vm/mod.rs`, `src/vm/executor.rs`

YaoXiang 虚拟机采用基于栈的字节码解释器设计，兼顾执行效率和实现简洁性。

```rust
/// 虚拟机
///
/// 基于栈的解释器，执行编译生成的字节码。
pub struct VM {
    /// 操作数栈
    pub stack: Vec<Value>,

    /// 调用帧栈
    pub frames: Vec<CallFrame>,

    /// 指令指针
    pub ip: usize,

    /// 常量池
    pub constant_pool: Vec<Value>,

    /// 全局变量
    pub globals: HashMap<String, Value>,

    /// 配置
    pub config: VMConfig,

    /// 内联缓存
    pub inline_cache: InlineCache,

    /// 调度器（用于并发任务）
    pub scheduler: Option<Scheduler>,
}

#[derive(Debug, Clone)]
pub struct VMConfig {
    /// 最大栈大小
    pub max_stack_size: usize,
    /// 最大调用帧深度
    pub max_call_depth: usize,
    /// 是否启用内联缓存
    pub enable_inline_cache: bool,
    /// 跟踪执行
    pub trace_execution: bool,
}

impl Default for VMConfig {
    fn default() -> Self {
        Self {
            max_stack_size: 1024,
            max_call_depth: 256,
            enable_inline_cache: true,
            trace_execution: false,
        }
    }
}

/// 虚拟机状态
#[derive(Debug, Clone, PartialEq)]
pub enum VMStatus {
    /// 正常运行
    Running,
    /// 等待（阻塞）
    Waiting,
    /// 已暂停
    Paused,
    /// 已完成
    Completed,
    /// 错误
    Error(String),
}

/// 虚拟机执行结果
pub type VMResult<T> = Result<T, VMError>;
```

### 2.2 值类型系统

**核心文件**：`src/vm/executor.rs`

```rust
/// 运行时代值
///
/// 表示运行时可能出现的所有值类型。
#[derive(Debug, Clone)]
pub enum Value {
    /// 布尔值
    Bool(bool),

    /// 整数 (64位)
    Int(i64),

    /// 浮点数 (64位)
    Float(f64),

    /// 字符
    Char(char),

    /// 字符串
    String(String),

    /// 空值
    Unit,

    /// 列表
    List(Rc<RefCell<Vec<Value>>>),

    /// 字典
    Dict(Rc<RefCell<HashMap<String, Value>>>),

    /// 元组
    Tuple(Vec<Value>),

    /// 函数
    Function(FuncId),

    /// 闭包
    Closure(Closure),

    /// 任务（并发）
    Task(TaskId),

    /// 可变引用
    Ref(RefCell<Value>),

    /// 堆对象指针
    Object(HeapPtr),
}

/// 闭包
#[derive(Debug, Clone)]
pub struct Closure {
    /// 函数 ID
    pub func_id: FuncId,
    /// 环境变量
    pub env: Vec<Value>,
}

/// 堆指针
#[derive(Debug, Clone)]
pub struct HeapPtr(pub usize);

impl Value {
    /// 获取值的类型名称
    pub fn type_name(&self) -> String {
        match self {
            Value::Bool(_) => "bool".to_string(),
            Value::Int(_) => "int".to_string(),
            Value::Float(_) => "float".to_string(),
            Value::Char(_) => "char".to_string(),
            Value::String(_) => "string".to_string(),
            Value::Unit => "void".to_string(),
            Value::List(_) => "list".to_string(),
            Value::Dict(_) => "dict".to_string(),
            Value::Tuple(_) => "tuple".to_string(),
            Value::Function(_) => "function".to_string(),
            Value::Closure(_) => "closure".to_string(),
            Value::Task(_) => "task".to_string(),
            Value::Ref(_) => "ref".to_string(),
            Value::Object(_) => "object".to_string(),
        }
    }

    /// 检查是否为真值
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Int(n) => *n != 0,
            Value::Float(f) => *f != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Unit => false,
            Value::List(v) => !v.borrow().is_empty(),
            Value::Dict(d) => !d.borrow().is_empty(),
            _ => true,
        }
    }
}
```

### 2.3 调用帧管理

**核心文件**：`src/vm/frames.rs`

```rust
/// 调用帧
///
/// 表示函数调用时的执行上下文，包含：
/// - 返回地址
/// - 局部变量
/// - 参数
/// - 闭包环境
pub struct CallFrame {
    /// 函数索引
    pub func_idx: u32,

    /// 返回地址
    pub return_addr: u32,

    /// 栈基址
    pub base_ptr: u32,

    /// 闭包（如果是闭包调用）
    pub closure: Option<Closure>,

    /// 局部变量槽
    pub locals: Vec<Value>,

    /// 指令指针（捕获以支持调试）
    pub ip: usize,
}

impl CallFrame {
    /// 创建新调用帧
    pub fn new(
        func_idx: u32,
        return_addr: u32,
        base_ptr: u32,
        arg_count: usize,
        closure: Option<Closure>,
    ) -> Self {
        Self {
            func_idx,
            return_addr,
            base_ptr,
            closure,
            locals: vec![Value::Unit; arg_count],
            ip: 0,
        }
    }

    /// 获取局部变量
    #[inline]
    pub fn get_local(&self, index: u32) -> Option<&Value> {
        self.locals.get(index as usize)
    }

    /// 设置局部变量
    #[inline]
    pub fn set_local(&mut self, index: u32, value: Value) {
        if let Some(slot) = self.locals.get_mut(index as usize) {
            *slot = value;
        }
    }
}
```

### 2.4 指令集

**核心文件**：`src/vm/instructions.rs`, `src/vm/opcode.rs`

```rust
/// 虚拟机指令
///
/// 所有字节码指令的定义。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Instruction {
    // =====================================================================
    // 栈操作
    // =====================================================================
    /// Nop - 空操作
    Nop,
    /// Pop - 弹出栈顶值
    Pop,
    /// Dup - 复制栈顶值
    Dup,
    /// Swap - 交换栈顶两个值
    Swap,

    // =====================================================================
    // 常量加载
    // =====================================================================
    /// LoadConst { index: u16 } - 从常量池加载常量
    LoadConst(u16),

    // =====================================================================
    // 局部变量操作
    // =====================================================================
    /// LoadLocal { index: u8 } - 加载局部变量
    LoadLocal(u8),
    /// StoreLocal { index: u8 } - 存储局部变量
    StoreLocal(u8),
    /// LoadArg { index: u8 } - 加载参数
    LoadArg(u8),

    // =====================================================================
    // 全局变量操作
    // =====================================================================
    /// LoadGlobal { index: u16 } - 加载全局变量
    LoadGlobal(u16),
    /// StoreGlobal { index: u16 } - 存储全局变量
    StoreGlobal(u16),

    // =====================================================================
    // 算术运算 (64位整数)
    // =====================================================================
    /// I64Add - 整数加法
    I64Add,
    /// I64Sub - 整数减法
    I64Sub,
    /// I64Mul - 整数乘法
    I64Mul,
    /// I64Div - 整数除法
    I64Div,
    /// I64Rem - 整数取余
    I64Rem,
    /// I64Neg - 整数取负
    I64Neg,

    // =====================================================================
    // 浮点运算 (64位)
    // =====================================================================
    /// F64Add - 浮点加法
    F64Add,
    /// F64Sub - 浮点减法
    F64Sub,
    /// F64Mul - 浮点乘法
    F64Mul,
    /// F64Div - 浮点除法
    F64Div,
    /// F64Neg - 浮点取负
    F64Neg,

    // =====================================================================
    // 比较运算
    // =====================================================================
    /// Eq - 相等比较
    Eq,
    /// Ne - 不等比较
    Ne,
    /// Lt - 小于比较
    Lt,
    /// Le - 小于等于比较
    Le,
    /// Gt - 大于比较
    Gt,
    /// Ge - 大于等于比较
    Ge,

    // =====================================================================
    // 逻辑运算
    // =====================================================================
    /// And - 逻辑与
    And,
    /// Or - 逻辑或
    Or,
    /// Not - 逻辑非
    Not,

    // =====================================================================
    // 控制流
    // =====================================================================
    /// Jmp { offset: i16 } - 无条件跳转
    Jmp(i16),
    /// JmpIf { offset: i16 } - 条件为真跳转
    JmpIf(i16),
    /// JmpIfNot { offset: i16 } - 条件为假跳转
    JmpIfNot(i16),

    // =====================================================================
    // 函数调用
    // =====================================================================
    /// Call { arg_count: u8 } - 函数调用
    Call(u8),
    /// CallIndirect - 间接调用（通过函数值）
    CallIndirect,
    /// TailCall { arg_count: u8 } - 尾调用优化
    TailCall(u8),
    /// Return - 返回
    Return,
    /// ReturnValue - 返回值
    ReturnValue,

    // =====================================================================
    // 内存操作
    // =====================================================================
    /// StackAlloc { size: u16 } - 栈分配
    StackAlloc(u16),
    /// HeapAlloc { type_id: u16 } - 堆分配
    HeapAlloc(u16),

    // =====================================================================
    // 列表操作
    // =====================================================================
    /// NewList - 创建空列表
    NewList,
    /// NewListWithCap { cap: u16 } - 创建带容量的列表
    NewListWithCap(u16),
    /// LoadElement - 加载列表元素
    LoadElement,
    /// StoreElement - 存储列表元素
    StoreElement,

    // =====================================================================
    // 字典操作
    // =====================================================================
    /// NewDict - 创建空字典
    NewDict,
    /// LoadDictElement - 加载字典元素
    LoadDictElement,
    /// StoreDictElement - 存储字典元素
    StoreDictElement,

    // =====================================================================
    // 字段操作
    // =====================================================================
    /// GetField { index: u16 } - 获取结构体字段
    GetField(u16),
    /// SetField { index: u16 } - 设置结构体字段
    SetField(u16),

    // =====================================================================
    // 类型操作
    // =====================================================================
    /// Cast { target_type: u16 } - 类型转换
    Cast(u16),
    /// TypeCheck { type_id: u16 } - 类型检查
    TypeCheck(u16),
    /// IsInstance { type_id: u16 } - 实例检查
    IsInstance(u16),

    // =====================================================================
    // 闭包操作
    // =====================================================================
    /// MakeClosure { func_id: u16, env_size: u8 } - 创建闭包
    MakeClosure(u16, u8),
    /// LoadEnv { index: u8 } - 加载环境变量
    LoadEnv(u8),
    /// LoadClosureVar { index: u8 } - 加载闭包变量
    LoadClosureVar(u8),

    // =====================================================================
    // 并发操作
    // =====================================================================
    /// Spawn - 创建新任务
    Spawn,
    /// Await - 等待任务完成
    Await,
    /// Yield - 让出执行权
    Yield,
    /// Join - 加入任务
    Join,

    // =====================================================================
    // 引用操作
    // =====================================================================
    /// Ref - 创建引用
    Ref,
    /// Deref - 解引用
    Deref,
    /// DerefMut - 可变解引用
    DerefMut,

    // =====================================================================
    // 迭代器操作
    // =====================================================================
    /// Iter - 创建迭代器
    Iter,
    /// IterNext - 迭代下一步
    IterNext,
    /// IterEnd - 迭代结束检查
    IterEnd,
}

/// 操作码（指令的字节编码）
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Opcode {
    Nop = 0x00,
    Pop = 0x01,
    Dup = 0x02,
    Swap = 0x03,

    LoadConst = 0x10,
    LoadLocal = 0x11,
    StoreLocal = 0x12,
    LoadArg = 0x13,
    LoadGlobal = 0x14,
    StoreGlobal = 0x15,

    I64Add = 0x20,
    I64Sub = 0x21,
    I64Mul = 0x22,
    I64Div = 0x23,
    I64Rem = 0x24,
    I64Neg = 0x25,

    F64Add = 0x30,
    F64Sub = 0x31,
    F64Mul = 0x32,
    F64Div = 0x33,
    F64Neg = 0x34,

    Eq = 0x40,
    Ne = 0x41,
    Lt = 0x42,
    Le = 0x43,
    Gt = 0x44,
    Ge = 0x45,

    And = 0x50,
    Or = 0x51,
    Not = 0x52,

    Jmp = 0x60,
    JmpIf = 0x61,
    JmpIfNot = 0x62,

    Call = 0x70,
    CallIndirect = 0x71,
    TailCall = 0x72,
    Return = 0x73,
    ReturnValue = 0x74,

    StackAlloc = 0x80,
    HeapAlloc = 0x81,

    NewList = 0x90,
    NewListWithCap = 0x91,
    LoadElement = 0x92,
    StoreElement = 0x93,

    NewDict = 0xA0,
    LoadDictElement = 0xA1,
    StoreDictElement = 0xA2,

    GetField = 0xB0,
    SetField = 0xB1,

    Cast = 0xC0,
    TypeCheck = 0xC1,
    IsInstance = 0xC2,

    MakeClosure = 0xD0,
    LoadEnv = 0xD1,
    LoadClosureVar = 0xD2,

    Spawn = 0xE0,
    Await = 0xE1,
    Yield = 0xE2,
    Join = 0xE3,

    Ref = 0xF0,
    Deref = 0xF1,
    DerefMut = 0xF2,

    Iter = 0xF8,
    IterNext = 0xF9,
    IterEnd = 0xFA,
}
```

### 2.5 执行器核心

**核心文件**：`src/vm/executor.rs`

```rust
impl VM {
    /// 创建新虚拟机
    pub fn new(config: Option<VMConfig>, bytecode: &BytecodeFile) -> Self {
        let config = config.unwrap_or_default();

        VM {
            stack: Vec::with_capacity(config.max_stack_size),
            frames: Vec::with_capacity(config.max_call_depth),
            ip: 0,
            constant_pool: bytecode.const_pool.clone(),
            globals: HashMap::new(),
            config,
            inline_cache: InlineCache::new(),
            scheduler: None,
        }
    }

    /// 执行字节码
    pub fn run(&mut self) -> VMResult<Value> {
        self.ip = 0;

        loop {
            // 获取当前调用帧
            let frame = self.frames.last_mut()
                .ok_or(VMError::NoCallFrame)?;

            // 检查是否到达函数末尾
            if frame.ip >= self.get_current_function().code.len() {
                // 默认返回 Unit
                return Ok(Value::Unit);
            }

            // 获取并执行指令
            let instr = self.get_current_function().code[frame.ip];
            frame.ip += 1;

            if self.config.trace_execution {
                println!("[VM] IP={} Instr={:?}", frame.ip - 1, instr);
            }

            self.execute_instruction(instr)?;
        }
    }

    /// 执行单条指令
    fn execute_instruction(&mut self, instr: Instruction) -> VMResult<()> {
        match instr {
            // 栈操作
            Instruction::Nop => {}
            Instruction::Pop => {
                self.stack.pop()
                    .ok_or(VMError::StackUnderflow)?;
            }
            Instruction::Dup => {
                let top = self.stack.last()
                    .ok_or(VMError::StackUnderflow)?
                    .clone();
                self.stack.push(top);
            }

            // 常量加载
            Instruction::LoadConst(idx) => {
                let value = self.constant_pool.get(idx as usize)
                    .ok_or(VMError::InvalidConstantIndex(idx))?
                    .clone();
                self.stack.push(value);
            }

            // 局部变量操作
            Instruction::LoadLocal(idx) => {
                let frame = self.frames.last_mut()
                    .ok_or(VMError::NoCallFrame)?;
                let value = frame.locals.get(idx as usize)
                    .ok_or(VMError::InvalidLocalIndex(idx))?
                    .clone();
                self.stack.push(value);
            }
            Instruction::StoreLocal(idx) => {
                let value = self.stack.pop()
                    .ok_or(VMError::StackUnderflow)?;
                let frame = self.frames.last_mut()
                    .ok_or(VMError::NoCallFrame)?;
                if let Some(slot) = frame.locals.get_mut(idx as usize) {
                    *slot = value;
                }
            }

            // 算术运算
            Instruction::I64Add => {
                let (b, a) = self.pop_two()?;
                match (a, b) {
                    (Value::Int(lhs), Value::Int(rhs)) => {
                        self.stack.push(Value::Int(lhs + rhs));
                    }
                    _ => return Err(VMError::TypeError("expected int".to_string())),
                }
            }
            // ... 其他算术运算类似

            // 比较运算
            Instruction::Eq => {
                let (b, a) = self.pop_two()?;
                self.stack.push(Value::Bool(a == b));
            }
            // ... 其他比较运算类似

            // 控制流
            Instruction::Jmp(offset) => {
                let frame = self.frames.last_mut()
                    .ok_or(VMError::NoCallFrame)?;
                frame.ip = (frame.ip as i32 + offset as i32) as usize;
            }
            Instruction::JmpIf(offset) => {
                let cond = self.pop_one()?;
                if cond.is_truthy() {
                    let frame = self.frames.last_mut()
                        .ok_or(VMError::NoCallFrame)?;
                    frame.ip = (frame.ip as i32 + offset as i32) as usize;
                }
            }

            // 函数调用
            Instruction::Call(arg_count) => {
                let func = self.pop_one()?;
                match func {
                    Value::Function(func_id) => {
                        self.call_function(func_id, arg_count as usize)?;
                    }
                    Value::Closure(closure) => {
                        self.call_closure(closure, arg_count as usize)?;
                    }
                    _ => return Err(VMError::NotCallable(func.type_name())),
                }
            }
            Instruction::Return => {
                self.frames.pop();
                if self.frames.is_empty() {
                    return Ok(Value::Unit);
                }
            }
            Instruction::ReturnValue => {
                let value = self.pop_one()?;
                self.frames.pop();
                if self.frames.is_empty() {
                    return Ok(value);
                }
                self.stack.push(value);
            }

            // ... 其他指令

            // 异常情况
            _ => return Err(VMError::UnimplementedInstruction(format!("{:?}", instr))),
        }

        Ok(())
    }

    /// 调用函数
    fn call_function(&mut self, func_id: u32, arg_count: usize) -> VMResult<()> {
        let frame = CallFrame::new(
            func_id,
            self.get_current_ip() as u32,
            self.stack.len() as u32,
            arg_count,
            None,
        );
        self.frames.push(frame);
        Ok(())
    }

    /// 辅助方法
    fn pop_two(&mut self) -> VMResult<(Value, Value)> {
        let b = self.stack.pop()
            .ok_or(VMError::StackUnderflow)?;
        let a = self.stack.pop()
            .ok_or(VMError::StackUnderflow)?;
        Ok((b, a))
    }

    fn pop_one(&mut self) -> VMResult<Value> {
        self.stack.pop()
            .ok_or(VMError::StackUnderflow)
    }
}
```

---

## 三、并作模型

### 3.1 并作图 (DAG)

**核心文件**：`src/runtime/dag/mod.rs`, `src/runtime/dag/node.rs`, `src/runtime/dag/graph.rs`

YaoXiang 的并作模型基于有向无环图（DAG）表示任务间的依赖关系。

```rust
/// 并作图
///
/// 管理并作任务的依赖关系图。
pub struct ComputationDAG {
    /// 节点映射
    nodes: HashMap<NodeId, DAGNode>,

    /// 出边（从节点到依赖它的节点）
    outgoing_edges: HashMap<NodeId, HashSet<NodeId>>,

    /// 入边（从节点到它依赖的节点）
    incoming_edges: HashMap<NodeId, HashSet<NodeId>>,

    /// 节点 ID 生成器
    id_generator: NodeIdGenerator,

    /// 根节点（没有入边的节点）
    roots: HashSet<NodeId>,
}

/// 并作图节点
#[derive(Debug, Clone)]
pub struct DAGNode {
    /// 节点 ID
    pub id: NodeId,

    /// 节点类型
    pub kind: DAGNodeKind,

    /// 依赖的节点
    pub dependencies: Vec<NodeId>,

    /// 节点状态
    pub status: NodeStatus,

    /// 结果值（计算完成后）
    pub result: Option<Value>,

    /// 错误（如果计算失败）
    pub error: Option<String>,
}

/// 节点类型
#[derive(Debug, Clone)]
pub enum DAGNodeKind {
    /// 值节点（叶子节点）
    Value(Value),

    /// 计算节点
    Computation {
        /// 函数 ID
        func_id: u32,
        /// 参数
        args: Vec<NodeId>,
    },

    /// 任务节点
    Task {
        /// 任务 ID
        task_id: TaskId,
    },

    /// 等待节点（等待其他节点完成）
    Wait {
        /// 依赖的节点
        dependencies: Vec<NodeId>,
    },
}

/// 节点状态
#[derive(Debug, Clone, PartialEq)]
pub enum NodeStatus {
    /// 等待依赖
    Pending,

    /// 就绪（依赖已满足）
    Ready,

    /// 执行中
    Running,

    /// 已完成
    Completed,

    /// 失败
    Failed(String),
}

impl ComputationDAG {
    /// 创建新的 DAG
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            outgoing_edges: HashMap::new(),
            incoming_edges: HashMap::new(),
            id_generator: NodeIdGenerator::new(),
            roots: HashSet::new(),
        }
    }

    /// 添加值节点
    pub fn add_value(&mut self, value: Value) -> NodeId {
        let id = self.id_generator.next();

        let node = DAGNode {
            id,
            kind: DAGNodeKind::Value(value),
            dependencies: Vec::new(),
            status: NodeStatus::Completed,
            result: None,
            error: None,
        };

        self.nodes.insert(id, node);
        self.roots.insert(id);

        id
    }

    /// 添加计算节点
    pub fn add_computation(&mut self, func_id: u32, args: Vec<NodeId>) -> NodeId {
        let id = self.id_generator.next();

        // 检查依赖是否都已完成
        let mut all_ready = true;
        for &dep_id in &args {
            if let Some(dep) = self.nodes.get(&dep_id) {
                if dep.status != NodeStatus::Completed {
                    all_ready = false;
                    break;
                }
            }
        }

        let node = DAGNode {
            id,
            kind: DAGNodeKind::Computation { func_id, args: args.clone() },
            dependencies: args.clone(),
            status: if all_ready { NodeStatus::Ready } else { NodeStatus::Pending },
            result: None,
            error: None,
        };

        self.nodes.insert(id, node);

        // 建立边关系
        for &dep_id in &args {
            self.outgoing_edges.entry(dep_id)
                .or_insert_with(HashSet::new)
                .insert(id);
            self.incoming_edges.entry(id)
                .or_insert_with(HashSet::new)
                .insert(dep_id);
        }

        // 如果有依赖未完成，添加到入边
        if !args.is_empty() {
            self.roots.remove(&id);
        } else {
            self.roots.insert(id);
        }

        id
    }

    /// 获取所有就绪节点
    pub fn get_ready_nodes(&self) -> Vec<NodeId> {
        self.nodes.iter()
            .filter(|(_, node)| node.status == NodeStatus::Ready)
            .map(|(id, _)| *id)
            .collect()
    }

    /// 完成节点计算
    pub fn complete(&mut self, node_id: NodeId, result: Value) {
        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.status = NodeStatus::Completed;
            node.result = Some(result);
        }

        // 标记依赖此节点的节点为就绪
        if let Some(dependents) = self.outgoing_edges.get(&node_id) {
            for &dep_id in dependents {
                if let Some(dep_node) = self.nodes.get_mut(&dep_id) {
                    // 检查是否所有依赖都已完成
                    let all_deps_done = dep_node.dependencies.iter()
                        .all(|&d| {
                            self.nodes.get(&d)
                                .map(|n| n.status == NodeStatus::Completed)
                                .unwrap_or(false)
                        });

                    if all_deps_done {
                        dep_node.status = NodeStatus::Ready;
                    }
                }
            }
        }
    }

    /// 获取已完成节点的值
    pub fn get_value(&self, node_id: NodeId) -> Option<&Value> {
        self.nodes.get(&node_id)
            .and_then(|n| n.result.as_ref())
    }
}
```

### 3.2 节点 ID 管理

**核心文件**：`src/runtime/dag/node_id.rs`

```rust
/// 节点 ID
///
/// DAG 中节点的唯一标识符。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub u32);

impl NodeId {
    /// 获取内部索引
    pub fn index(&self) -> u32 {
        self.0
    }
}

/// 线程安全的节点 ID 生成器
#[derive(Debug)]
pub struct NodeIdGenerator {
    next_id: AtomicU32,
}

impl NodeIdGenerator {
    /// 创建新的 ID 生成器
    pub fn new() -> Self {
        Self {
            next_id: AtomicU32::new(0),
        }
    }

    /// 生成下一个 ID
    pub fn next(&self) -> NodeId {
        NodeId(self.next_id.fetch_add(1, Ordering::SeqCst))
    }
}
```

---

## 四、任务调度器

### 4.1 调度器架构

**核心文件**：`src/runtime/scheduler/mod.rs`, `src/runtime/scheduler/task.rs`, `src/runtime/scheduler/queue.rs`, `src/runtime/scheduler/work_stealer.rs`

```rust
/// 调度器
///
/// 负责并作任务的调度和执行。
pub struct Scheduler {
    /// 工作线程
    workers: Vec<Worker>,

    /// 全局任务队列
    global_queue: Arc<PriorityTaskQueue>,

    /// 所有任务
    tasks: HashMap<TaskId, Task>,

    /// 并作图
    dag: Arc<RwLock<ComputationDAG>>,

    /// 配置
    config: SchedulerConfig,

    /// 统计信息
    stats: Arc<SchedulerStats>,
}

/// 调度器配置
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// 工作线程数量
    pub num_workers: usize,
    /// 默认栈大小
    pub default_stack_size: usize,
    /// 工作窃取批次大小
    pub steal_batch: usize,
    /// 每个工作队列的最大大小
    pub max_queue_size: usize,
    /// 是否启用工作窃取
    pub use_work_stealing: bool,
    /// 空闲超时
    pub idle_timeout: Duration,
    /// 是否收集统计信息
    pub enable_stats: bool,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        let num_cpus = thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);

        Self {
            num_workers: num_cpus,
            default_stack_size: 2 * 1024 * 1024,
            steal_batch: 4,
            max_queue_size: 1024,
            use_work_stealing: true,
            idle_timeout: Duration::from_millis(1),
            enable_stats: false,
        }
    }
}

/// 调度器统计
#[derive(Debug, Default)]
pub struct SchedulerStats {
    /// 调度任务总数
    pub tasks_scheduled: AtomicUsize,
    /// 完成任务总数
    pub tasks_completed: AtomicUsize,
    /// 被窃取的任务数
    pub tasks_stolen: AtomicUsize,
    /// 窃取尝试次数
    pub steal_attempts: AtomicUsize,
    /// 成功窃取次数
    pub steal_success: AtomicUsize,
}

impl SchedulerStats {
    /// 记录调度的任务
    pub fn record_scheduled(&self) {
        self.tasks_scheduled.fetch_add(1, Ordering::SeqCst);
    }

    /// 记录完成的任务
    pub fn record_completed(&self, duration_us: usize) {
        self.tasks_completed.fetch_add(1, Ordering::SeqCst);
    }
}
```

### 4.2 任务定义

**核心文件**：`src/runtime/scheduler/task.rs`

```rust
/// 任务 ID
///
/// 任务的唯一标识符。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(pub u64);

impl TaskId {
    /// 获取内部索引
    pub fn index(&self) -> u64 {
        self.0
    }
}

/// 任务
///
/// 表示一个可并作执行的工作单元。
pub struct Task {
    /// 任务 ID
    pub id: TaskId,

    /// 函数 ID
    pub func_id: FuncId,

    /// 参数
    pub args: Vec<Value>,

    /// 状态
    pub status: TaskStatus,

    /// 优先级
    pub priority: TaskPriority,

    /// 结果
    pub result: Option<Value>,

    /// 错误
    pub error: Option<String>,
}

/// 任务状态
#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    /// 等待调度
    Pending,

    /// 就绪（可执行）
    Ready,

    /// 执行中
    Running,

    /// 等待（阻塞）
    Waiting,

    /// 已完成
    Completed,

    /// 失败
    Failed(String),

    /// 取消
    Cancelled,
}

/// 任务优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    /// 低优先级
    Low = 0,
    /// 正常优先级
    Normal = 1,
    /// 高优先级
    High = 2,
    /// 关键优先级
    Critical = 3,
}

/// 任务配置
#[derive(Debug, Clone)]
pub struct TaskConfig {
    /// 栈大小
    pub stack_size: usize,
    /// 优先级
    pub priority: TaskPriority,
    /// 是否可窃取
    pub stealable: bool,
    /// 依赖的任务
    pub dependencies: Vec<TaskId>,
}

/// 任务构建器
pub struct TaskBuilder {
    config: TaskConfig,
}

impl TaskBuilder {
    /// 创建新的任务构建器
    pub fn new() -> Self {
        Self {
            config: TaskConfig {
                stack_size: 0,
                priority: TaskPriority::Normal,
                stealable: true,
                dependencies: Vec::new(),
            },
        }
    }

    /// 设置栈大小
    pub fn stack_size(mut self, size: usize) -> Self {
        self.config.stack_size = size;
        self
    }

    /// 设置优先级
    pub fn priority(mut self, priority: TaskPriority) -> Self {
        self.config.priority = priority;
        self
    }

    /// 设置是否可窃取
    pub fn stealable(mut self, stealable: bool) -> Self {
        self.config.stealable = stealable;
        self
    }

    /// 添加依赖
    pub fn depends_on(mut self, task_id: TaskId) -> Self {
        self.config.dependencies.push(task_id);
        self
    }

    /// 构建任务
    pub fn build(self, id: TaskId, func_id: FuncId, args: Vec<Value>) -> Task {
        Task {
            id,
            func_id,
            args,
            status: TaskStatus::Pending,
            priority: self.config.priority,
            result: None,
            error: None,
        }
    }
}
```

### 4.3 任务队列

**核心文件**：`src/runtime/scheduler/queue.rs`

```rust
/// 任务队列
///
/// 支持优先级和阻塞等待的任务队列。
pub struct TaskQueue {
    /// 内部队列（使用 VecDeque 实现双端队列）
    queue: Mutex<VecDeque<TaskId>>,

    /// 条件变量（用于阻塞等待）
    condvar: Condvar,

    /// 是否已关闭
    closed: AtomicBool,
}

/// 优先级任务队列
pub struct PriorityTaskQueue {
    /// 按优先级分组的队列
    queues: [Arc<TaskQueue>; 4],

    /// 总任务数
    len: AtomicUsize,
}

impl TaskQueue {
    /// 创建新任务队列
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            condvar: Condvar::new(),
            closed: AtomicBool::new(false),
        }
    }

    /// 入队
    pub fn push(&self, task_id: TaskId) {
        let mut queue = self.queue.lock().unwrap();
        queue.push_back(task_id);
        self.condvar.notify_one();
    }

    /// 出队（阻塞）
    pub fn pop(&self) -> Option<TaskId> {
        let mut queue = self.queue.lock().unwrap();

        while queue.is_empty() && !self.closed.load(Ordering::SeqCst) {
            queue = self.condvar.wait(queue).unwrap();
        }

        queue.pop_front()
    }

    /// 尝试出队（非阻塞）
    pub fn try_pop(&self) -> Option<TaskId> {
        self.queue.lock().unwrap().pop_front()
    }

    /// 关闭队列
    pub fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
        self.condvar.notify_all();
    }
}

impl PriorityTaskQueue {
    /// 创建新的优先级队列
    pub fn new() -> Self {
        Self {
            queues: [
                Arc::new(TaskQueue::new()), // Low
                Arc::new(TaskQueue::new()), // Normal
                Arc::new(TaskQueue::new()), // High
                Arc::new(TaskQueue::new()), // Critical
            ],
            len: AtomicUsize::new(0),
        }
    }

    /// 入队（按优先级）
    pub fn push(&self, task_id: TaskId, priority: TaskPriority) {
        let idx = priority as usize;
        self.queues[idx].push(task_id);
        self.len.fetch_add(1, Ordering::SeqCst);
    }

    /// 出队（高优先级优先）
    pub fn pop(&self) -> Option<TaskId> {
        // 从高优先级到低优先级检查
        for i in (0..4).rev() {
            if let Some(task_id) = self.queues[i].try_pop() {
                self.len.fetch_sub(1, Ordering::SeqCst);
                return Some(task_id);
            }
        }
        None
    }
}
```

### 4.4 工作窃取算法

**核心文件**：`src/runtime/scheduler/work_stealer.rs`

```rust
/// 工作窃取器
///
/// 实现工作窃取算法，用于负载均衡。
pub struct WorkStealer {
    /// 被窃取的队列
    victim_queues: Vec<Arc<TaskQueue>>,

    /// 窃取策略
    strategy: StealStrategy,

    /// 窃取批次大小
    batch_size: usize,

    /// 统计信息
    stats: Arc<StealStats>,
}

/// 窃取策略
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StealStrategy {
    /// 随机选择队列
    Random,
    /// 轮询选择队列
    RoundRobin,
    /// 选择最空闲的队列
    LeastLoaded,
    /// 选择最长的队列（从头部窃取）
    LongestFirst,
}

/// 窃取统计
#[derive(Debug, Default)]
pub struct StealStats {
    /// 总尝试次数
    attempts: AtomicUsize,
    /// 成功次数
    successes: AtomicUsize,
    /// 窃取失败原因统计
    failures: AtomicUsize,
}

impl WorkStealer {
    /// 创建新的工作窃取器
    pub fn new(queues: Vec<Arc<TaskQueue>>, strategy: StealStrategy, batch_size: usize) -> Self {
        Self {
            victim_queues: queues,
            strategy,
            batch_size,
            stats: Arc::new(StealStats::default()),
        }
    }

    /// 尝试从其他队列窃取任务
    pub fn steal(&self, my_queue: &Arc<TaskQueue>) -> Vec<TaskId> {
        self.stats.attempts.fetch_add(1, Ordering::SeqCst);

        // 根据策略选择要窃取的队列
        let victim_id = self.select_victim();

        if let Some(victim) = self.victim_queues.get(victim_id) {
            // 尝试从 victim 队列的尾部窃取
            let mut stolen = Vec::new();

            for _ in 0..self.batch_size {
                if let Some(task_id) = self.steal_one(victim) {
                    stolen.push(task_id);
                } else {
                    break;
                }
            }

            if !stolen.is_empty() {
                self.stats.successes.fetch_add(1, Ordering::SeqCst);
                return stolen;
            }
        }

        self.stats.failures.fetch_add(1, Ordering::SeqCst);
        Vec::new()
    }

    /// 从单个队列窃取一个任务
    fn steal_one(&self, victim: &Arc<TaskQueue>) -> Option<TaskId> {
        // 从尾部窃取（双端队列特性）
        let mut queue = victim.queue.lock().unwrap();

        // 从尾部窃取，减少对 victim 的竞争
        queue.pop_back()
    }

    /// 根据策略选择要窃取的队列
    fn select_victim(&self) -> usize {
        match self.strategy {
            StealStrategy::Random => {
                use rand::random;
                random::<usize>() % self.victim_queues.len()
            }
            StealStrategy::RoundRobin => {
                // 使用原子计数器实现轮询
                static COUNTER: AtomicUsize = AtomicUsize::new(0);
                COUNTER.fetch_add(1, Ordering::SeqCst) % self.victim_queues.len()
            }
            StealStrategy::LeastLoaded => {
                // 选择队列最短的
                let mut min_len = usize::MAX;
                let mut min_id = 0;

                for (i, queue) in self.victim_queues.iter().enumerate() {
                    let len = queue.queue.lock().unwrap().len();
                    if len < min_len {
                        min_len = len;
                        min_id = i;
                    }
                }
                min_id
            }
            StealStrategy::LongestFirst => {
                // 选择队列最长的
                let mut max_len = 0;
                let mut max_id = 0;

                for (i, queue) in self.victim_queues.iter().enumerate() {
                    let len = queue.queue.lock().unwrap().len();
                    if len > max_len {
                        max_len = len;
                        max_id = i;
                    }
                }
                max_id
            }
        }
    }
}
```

---

## 五、内存管理

### 5.1 内存分配器

**核心文件**：`src/runtime/memory/mod.rs`

```rust
/// 内存管理模块
///
/// 提供栈分配和堆分配的管理。
pub struct MemoryManager {
    /// 堆分配器
    heap_allocator: HeapAllocator,

    /// 栈大小限制
    max_stack_size: usize,

    /// 当前栈使用量
    current_stack_size: AtomicUsize,

    /// GC 配置
    gc_config: GCConfig,
}

/// 堆分配器
pub struct HeapAllocator {
    /// 内存块列表
    blocks: Mutex<Vec<MemoryBlock>>,

    /// 总分配大小
    total_allocated: AtomicUsize,

    /// 最大分配大小
    max_allocated: AtomicUsize,
}

/// 内存块
#[derive(Debug)]
pub struct MemoryBlock {
    /// 起始地址
    pub start: usize,

    /// 大小
    pub size: usize,

    /// 类型
    pub kind: MemoryKind,

    /// 引用计数（用于 GC）
    ref_count: AtomicUsize,

    /// 是否可达（用于 GC）
    reachable: AtomicBool,
}

/// 内存类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MemoryKind {
    /// 值类型（小对象）
    Value,

    /// 列表
    List,

    /// 字典
    Dict,

    /// 闭包环境
    Closure,

    /// 字符串
    String,

    /// 用户对象
    Object,
}

/// GC 配置
#[derive(Debug, Clone)]
pub struct GCConfig {
    /// 触发 GC 的阈值（字节）
    pub threshold: usize,

    /// 是否启用增量 GC
    pub incremental: bool,

    /// 是否启用分代 GC
    pub generational: bool,

    /// 最小 GC 间隔（毫秒）
    pub min_gc_interval: u64,
}

impl Default for GCConfig {
    fn default() -> Self {
        Self {
            threshold: 10 * 1024 * 1024, // 10MB
            incremental: true,
            generational: true,
            min_gc_interval: 10,
        }
    }
}

impl MemoryManager {
    /// 创建新的内存管理器
    pub fn new(max_stack_size: usize) -> Self {
        Self {
            heap_allocator: HeapAllocator::new(),
            max_stack_size,
            current_stack_size: AtomicUsize::new(0),
            gc_config: GCConfig::default(),
        }
    }

    /// 栈分配
    pub fn stack_alloc(&self, size: usize) -> Result<StackAllocation, MemoryError> {
        let current = self.current_stack_size.load(Ordering::SeqCst);

        if current + size > self.max_stack_size {
            return Err(MemoryError::StackOverflow);
        }

        self.current_stack_size.store(current + size, Ordering::SeqCst);

        Ok(StackAllocation {
            size,
            manager: self,
        })
    }

    /// 堆分配
    pub fn heap_alloc(&self, size: usize, kind: MemoryKind) -> Result<HeapPtr, MemoryError> {
        self.heap_allocator.alloc(size, kind)
    }

    /// 分配列表
    pub fn alloc_list(&self, capacity: usize) -> Result<Value, MemoryError> {
        let ptr = self.heap_allocator.alloc(
            capacity * std::mem::size_of::<Value>(),
            MemoryKind::List,
        )?;

        Ok(Value::List(Rc::new(RefCell::new(Vec::with_capacity(capacity)))))
    }
}

impl HeapAllocator {
    /// 创建新的堆分配器
    pub fn new() -> Self {
        Self {
            blocks: Mutex::new(Vec::new()),
            total_allocated: AtomicUsize::new(0),
            max_allocated: AtomicUsize::new(0),
        }
    }

    /// 分配内存
    pub fn alloc(&self, size: usize, kind: MemoryKind) -> Result<HeapPtr, MemoryError> {
        // 使用系统分配器
        let layout = std::alloc::Layout::from_size_align(size, std::mem::align_of::<u8>())
            .map_err(|_| MemoryError::InvalidSize)?;

        let ptr = unsafe { std::alloc::alloc(layout) };

        if ptr.is_null() {
            return Err(MemoryError::OutOfMemory);
        }

        let block = MemoryBlock {
            start: ptr as usize,
            size,
            kind,
            ref_count: AtomicUsize::new(1),
            reachable: AtomicBool::new(true),
        };

        self.blocks.lock().unwrap().push(block);

        let total = self.total_allocated.fetch_add(size, Ordering::SeqCst) + size;
        let mut max = self.max_allocated.load(Ordering::SeqCst);
        while total > max {
            match self.max_allocated.compare_exchange(max, total, Ordering::SeqCst, Ordering::SeqCst) {
                Ok(_) => break,
                Err(new_max) => max = new_max,
            }
        }

        Ok(HeapPtr(ptr as usize))
    }

    /// 释放内存
    pub fn free(&self, ptr: HeapPtr) {
        let mut blocks = self.heap_allocator.blocks.lock().unwrap();

        if let Some(pos) = blocks.iter().position(|b| b.start == ptr.0) {
            let block = blocks.remove(pos);

            let layout = std::alloc::Layout::from_size_align(block.size, std::mem::align_of::<u8>())
                .unwrap();

            unsafe {
                std::alloc::dealloc(block.start as *mut u8, layout);
            }

            self.total_allocated.fetch_sub(block.size, Ordering::SeqCst);
        }
    }
}
```

### 5.2 垃圾回收

```rust
impl MemoryManager {
    /// 执行垃圾回收
    pub fn gc(&mut self) {
        // 标记阶段
        self.mark();

        // 清除阶段
        self.sweep();
    }

    /// 标记可达对象
    fn mark(&self) {
        // 从根集（全局变量、栈变量）开始标记
        // ...
    }

    /// 清除不可达对象
    fn sweep(&self) {
        let mut blocks = self.heap_allocator.blocks.lock().unwrap();

        blocks.retain(|block| {
            if block.reachable.load(Ordering::SeqCst) {
                // 重置可达性，供下次 GC 使用
                block.reachable.store(true, Ordering::SeqCst);
                true
            } else {
                // 释放内存
                drop(block);
                self.free(HeapPtr(block.start));
                false
            }
        });
    }

    /// 增加引用计数
    pub fn inc_ref(&self, ptr: HeapPtr) {
        if let Some(block) = self.find_block(ptr) {
            block.ref_count.fetch_add(1, Ordering::SeqCst);
        }
    }

    /// 减少引用计数
    pub fn dec_ref(&self, ptr: HeapPtr) {
        if let Some(block) = self.find_block(ptr) {
            let count = block.ref_count.fetch_sub(1, Ordering::SeqCst);
            if count == 1 {
                // 引用计数为0，立即释放
                self.free(ptr);
            }
        }
    }

    fn find_block(&self, ptr: HeapPtr) -> Option<MemoryBlock> {
        let blocks = self.heap_allocator.blocks.lock().unwrap();
        blocks.iter().find(|b| b.start == ptr.0).cloned()
    }
}
```

---

## 六、内联缓存

### 6.1 内联缓存设计

**核心文件**：`src/vm/inline_cache.rs`

```rust
/// 内联缓存
///
/// 用于缓存类型检查和方法分派的结果，加速动态分派。
pub struct InlineCache {
    /// 缓存条目
    caches: HashMap<CacheKey, CacheEntry>,
}

impl InlineCache {
    /// 创建一个新的内联缓存
    pub fn new() -> Self {
        Self {
            caches: HashMap::new(),
        }
    }

    /// 查找缓存
    pub fn lookup(&mut self, receiver_type: TypeId, method: &str) -> Option<FuncId> {
        let key = CacheKey {
            receiver_type,
            method: method.to_string(),
        };

        self.caches.get(&key).map(|entry| {
            entry.hit_count += 1;
            entry.func_id
        })
    }

    /// 更新缓存
    pub fn update(&mut self, receiver_type: TypeId, method: &str, func_id: FuncId) {
        let key = CacheKey {
            receiver_type,
            method: method.to_string(),
        };

        self.caches.insert(key, CacheEntry {
            func_id,
            hit_count: 0,
            compiled_at: std::time::Instant::now(),
        });
    }

    /// 获取缓存统计
    pub fn stats(&self) -> CacheStats {
        let total_hits: usize = self.caches.values().map(|e| e.hit_count).sum();
        let total_misses = self.caches.len(); // 简化计算

        CacheStats {
            cache_size: self.caches.len(),
            total_hits,
            total_misses,
            hit_rate: if total_hits + total_misses > 0 {
                total_hits as f64 / (total_hits + total_misses) as f64
            } else {
                0.0
            },
        }
    }
}

/// 缓存键
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    receiver_type: TypeId,
    method: String,
}

/// 缓存条目
#[derive(Debug, Clone)]
struct CacheEntry {
    func_id: FuncId,
    hit_count: usize,
    compiled_at: std::time::Instant,
}

/// 缓存统计
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub cache_size: usize,
    pub total_hits: usize,
    pub total_misses: usize,
    pub hit_rate: f64,
}
```

---

## 七、性能优化

### 7.1 虚拟机优化策略

| 优化策略 | 描述 | 效果 |
|---------|------|------|
| **Computed Goto** | 使用跳转表替代 switch | 减少分支预测失败 |
| **栈顶缓存** | 缓存频繁访问的栈顶值 | 减少栈操作 |
| **指令内联** | 将常用指令序列内联 | 减少指令开销 |
| **本地变量缓存** | 缓存频繁访问的局部变量 | 减少内存访问 |
| **常量传播** | 预加载常用常量 | 减少常量池查找 |

### 7.2 调度器优化策略

| 优化策略 | 描述 | 效果 |
|---------|------|------|
| **工作窃取** | 空闲线程从繁忙线程窃取任务 | 负载均衡 |
| **亲和性调度** | 优先在相同线程执行相关任务 | 缓存友好 |
| **批量处理** | 一次处理多个任务 | 减少上下文切换 |
| **自适应线程池** | 根据负载动态调整线程数 | 资源高效利用 |

### 7.3 内存优化策略

| 优化策略 | 描述 | 效果 |
|---------|------|------|
| **小对象优化** | 使用栈分配小对象 | 减少堆分配 |
| **内存池** | 预分配常用大小的内存块 | 减少分配开销 |
| **引用计数** | 自动管理对象生命周期 | 减少内存泄漏 |
| **分代 GC** | 新对象在年轻代，频繁回收 | 减少 GC 开销 |

---

## 八、错误处理

### 8.1 错误类型

**核心文件**：`src/vm/errors.rs`

```rust
/// 虚拟机错误
#[derive(Debug, Error)]
pub enum VMError {
    #[error("Stack underflow")]
    StackUnderflow,

    #[error("Stack overflow (max: {0})")]
    StackOverflow(usize),

    #[error("Invalid constant index: {0}")]
    InvalidConstantIndex(u16),

    #[error("Invalid local index: {0}")]
    InvalidLocalIndex(u8),

    #[error("Invalid global index: {0}")]
    InvalidGlobalIndex(u16),

    #[error("No call frame")]
    NoCallFrame,

    #[error("Call stack overflow (max depth: {0})")]
    CallStackOverflow(usize),

    #[error("Type error: {0}")]
    TypeError(String),

    #[error("Division by zero")]
    DivisionByZero,

    #[error("Not callable: {0}")]
    NotCallable(String),

    #[error("Index out of bounds: {0}")]
    IndexOutOfBounds(usize),

    #[error("Key not found: {0}")]
    KeyNotFound(String),

    #[error("Unimplemented instruction: {0}")]
    UnimplementedInstruction(String),

    #[error("Runtime error: {0}")]
    RuntimeError(String),

    #[error("Task error: {0}")]
    TaskError(String),
}
```

### 8.2 错误恢复

```rust
impl VM {
    /// 执行并捕获错误
    pub fn run_catch(&mut self) -> Result<Value, VMError> {
        match self.run() {
            Ok(value) => Ok(value),
            Err(error) => {
                // 生成错误回溯
                let backtrace = self.generate_backtrace();
                eprintln!("Error: {}", error);
                eprintln!("Backtrace:");
                for (i, frame_info) in backtrace.iter().enumerate() {
                    eprintln!("  #{}: {}", i, frame_info);
                }
                Err(error)
            }
        }
    }

    /// 生成错误回溯
    fn generate_backtrace(&self) -> Vec<String> {
        self.frames.iter()
            .enumerate()
            .map(|(i, frame)| {
                let func = &self.get_function(frame.func_id);
                format!("at {} ({}:{})", func.name, func.filename, frame.ip)
            })
            .collect()
    }
}
```

---

## 九、总结

YaoXiang 运行时系统采用模块化设计：

1. **虚拟机**：基于栈的解释器，高效执行字节码
2. **并作模型**：DAG 表示任务依赖，自动并行执行
3. **调度器**：工作窃取算法，负载均衡
4. **内存管理**：引用计数 + 垃圾回收，自动内存管理
5. **内联缓存**：优化动态分派性能

**核心创新点**：

- **同步思维，并发执行**：用户使用同步 API，编译器自动构建 DAG
- **工作窃取负载均衡**：高效利用多核资源
- **类型化字节码**：减少运行时类型检查开销
- **增量 GC**：减少垃圾回收暂停时间

**最后更新**：2025-01-04
