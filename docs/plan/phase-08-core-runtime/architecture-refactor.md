# YaoXiang 架构重构设计文档

> **状态说明**: 本文档描述的架构已部分实现。`backends/` 目录结构已建立，但各组件实现为简化版本，后续需要逐步完善。

## 一、当前架构分析

### 1.1 现有结构

```
src/
├── lib.rs                      # 导出点，直接使用 VM
├── frontend/                   # 前端
│   ├── lexer/
│   ├── parser/
│   └── typecheck/
├── middle/                     # 中层
│   ├── ir.rs                   # 高层 IR (Instruction, Operand)
│   ├── codegen/
│   │   ├── bytecode.rs         # 字节码格式和编译
│   │   ├── ir_builder.rs       # IRBuilder
│   │   └── gen/                # 代码生成
│   ├── optimizer.rs
│   └── lifetime/
├── runtime/                    # 运行时（共享组件）
│   ├── memory/allocator.rs     # 内存分配
│   ├── value/                  # RuntimeValue
│   ├── scheduler/              # 任务调度
│   └── dag/                    # DAG 依赖
└── vm/                         # 解释器
    ├── executor.rs             # 2400+ 行巨型 VM
    ├── opcode.rs               # TypedOpcode 定义
    └── errors.rs
```

### 1.2 核心问题

| 问题 | 表现 | 影响 |
|------|------|------|
| **职责不清** | VM 同时包含执行逻辑和调试功能 | 难以复用 |
| **缺少抽象层** | codegen 直接产出 `TypedOpcode` | 无法支持多后端 |
| **巨型文件** | `executor.rs` 2400+ 行 | 难以维护 |
| **重复代码** | VM 和未来的解释器会有大量重复 | 维护噩梦 |
| **IR 层不完整** | 只有高层 IR，缺少低层 IR | 无法解耦编译和执行 |

---

## 二、目标架构：后端抽象层

### 2.1 新的目录结构

```
src/
├── lib.rs                      # 导出后端抽象
├── frontend/                   # 前端（保持不变）
│   ├── lexer/
│   ├── parser/
│   └── typecheck/
├── middle/                     # 中层（重构 IR）
│   ├── ir.rs                   # 高层 IR (保持)
│   ├── bytecode.rs             # 【新增】低层 IR：字节码格式
│   └── codegen/                # 代码生成
│       ├── builder.rs          # IR Builder
│       └── target.rs           # 目标后端选择
├── backends/                   # 【新增】后端抽象层
│   ├── mod.rs                  # 后端公共接口
│   ├── common/                 # 【新增】共享组件
│   │   ├── mod.rs
│   │   ├── opcode.rs           # 统一的Opcode定义
│   │   ├── value.rs            # RuntimeValue（从runtime/value移动）
│   │   ├── heap.rs             # Heap存储（从runtime/value移动）
│   │   └── allocator.rs        # 内存分配器
│   ├── interpreter/            # 解释器后端
│   │   ├── mod.rs
│   │   ├── executor.rs         # 精简的解释器（~800行）
│   │   ├── registers.rs        # 寄存器文件
│   │   └── frames.rs           # 调用帧
│   ├── dev/                    # 【新增】开发环境专用
│   │   ├── mod.rs
│   │   ├── debugger.rs         # 断点、单步调试
│   │   ├── repl.rs             # REPL 交互
│   │   └── shell.rs            # 开发 Shell
│   └── runtime/                # 【新增】运行时支持
│       ├── mod.rs
│       └── task.rs             # 任务调度（基于 RFC-008）
├── std/                        # 标准库（保持不变）
└── util/                       # 工具（保持不变）
```

### 2.2 架构分层

```
源代码 → Parser → TypeCheck → 高层 IR (FunctionIR)
                                      ↓
                    +----------------+----------------+
                    ↓                                 ↓
            字节码生成器                       （未来）AOT 编译器
                    ↓                                 ↓
            低层 IR (BytecodeIR)              低层 IR (MachineIR)
                    ↓                                 ↓
                    +----------------+----------------+
                                      ↓
                        【后端抽象层】
                                      ↓
                    +----------------+----------------+
                    ↓                                 ↓
            解释器后端                       （未来）AOT 后端
            (interpreter/)                  (aot/)
                                      ↓
                    +----------------+----------------+
                    ↓                                 ↓
            开发模式                      生产模式
            (dev/shell.rs)               (直接使用 interpreter)
                                      ↓
                        【运行时】
                        (runtime/)
                        └── task.rs  → 任务调度（DAG 调度器）
```

> **设计依据**:
> - **RFC-008**: 三层运行时架构（Embedded/Standard/Full）
> - **RFC-009**: ❌ **无 GC**，使用 `ref=Arc` 内存管理
> - 任务边界 = 泄漏边界，循环引用由编译器检测
```

---

## 三、核心抽象设计

### 3.1 低层 IR：BytecodeIR

**设计原则**：与执行方式无关的、可序列化的指令表示。

```rust
// src/middle/bytecode.rs

/// 字节码指令（低层 IR）
///
/// 与 TypedOpcode 的区别：
/// - TypedOpcode 是"编码格式"，直接对应字节
/// - BytecodeIR 是"抽象表示"，包含语义信息
#[derive(Debug, Clone)]
pub enum BytecodeInstr {
    // 控制流
    Nop,
    Return { value: Option<Reg> },
    Jmp { target: Label },
    JmpIf { cond: Reg, target: Label },
    JmpIfNot { cond: Reg, target: Label },

    // 移动和加载
    Mov { dst: Reg, src: Reg },
    LoadConst { dst: Reg, const_idx: u16 },
    LoadLocal { dst: Reg, local_idx: u8 },
    StoreLocal { local_idx: u8, src: Reg },

    // 算术运算（统一的运算接口）
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

    // 比较
    Compare {
        dst: Reg,
        lhs: Reg,
        rhs: Reg,
        cmp: CompareOp,
    },

    // 函数调用
    Call {
        dst: Option<Reg>,
        func: FunctionRef,
        args: Vec<Reg>,
    },
    CallVirt {
        dst: Option<Reg>,
        obj: Reg,
        method: String,
        args: Vec<Reg>,
    },

    // 内存操作
    Load { dst: Reg, base: Reg, offset: i16 },
    Store { base: Reg, offset: i16, src: Reg },

    // 对象操作
    NewList { dst: Reg, capacity: Option<u16> },
    GetField { dst: Reg, src: Reg, field: u16 },
    SetField { src: Reg, field: u16, value: Reg },

    // 闭包
    MakeClosure { dst: Reg, func: FunctionRef, env: Vec<Reg> },
    LoadUpvalue { dst: Reg, upvalue_idx: u8 },
    StoreUpvalue { upvalue_idx: u8, src: Reg },

    // 调试指令（解释器专用）
    Breakpoint,
    LineInfo { line: u32, column: u16 },
}

/// 二元运算类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add, Sub, Mul, Div, Rem,
    And, Or, Xor,
    Shl, Shr, Sar,
}

/// 比较运算类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    Eq, Ne, Lt, Le, Gt, Ge,
}

/// 函数引用
#[derive(Debug, Clone)]
pub enum FunctionRef {
    /// 静态函数引用
    Static { name: String },
    /// 通过索引引用（编译后使用）
    Index(u32),
}

/// 字节码函数
#[derive(Debug, Clone)]
pub struct BytecodeFunction {
    pub name: String,
    pub params: Vec<Type>,
    pub return_type: Type,
    pub locals: usize,
    pub instructions: Vec<BytecodeInstr>,
    /// 标签到指令索引的映射
    pub labels: HashMap<Label, usize>,
}

/// 字节码模块
#[derive(Debug, Clone)]
pub struct BytecodeModule {
    pub constants: Vec<ConstValue>,
    pub functions: Vec<BytecodeFunction>,
    pub globals: Vec<(String, Type, Option<ConstValue>)>,
}
```

### 3.2 后端 trait 抽象

```rust
// src/backends/mod.rs

use crate::middle::bytecode::{BytecodeModule, BytecodeFunction};
use crate::backends::common::value::RuntimeValue;

/// 执行器 trait - 所有后端必须实现
pub trait Executor {
    /// 错误类型
    type Error: std::fmt::Debug;

    /// 执行模块
    fn execute_module(&mut self, module: &BytecodeModule) -> Result<(), Self::Error>;

    /// 执行单个函数
    fn execute_function(
        &mut self,
        func: &BytecodeFunction,
        args: &[RuntimeValue],
    ) -> Result<RuntimeValue, Self::Error>;

    /// 重置状态
    fn reset(&mut self);
}

/// 可调试的执行器
pub trait DebuggableExecutor: Executor {
    /// 设置断点
    fn set_breakpoint(&mut self, offset: usize);

    /// 移除断点
    fn remove_breakpoint(&mut self, offset: usize);

    /// 单步执行
    fn step(&mut self) -> Result<(), Self::Error>;

    /// 获取当前状态（用于调试）
    fn debug_state(&self) -> DebugState;
}

/// 调试状态
#[derive(Debug, Clone)]
pub struct DebugState {
    pub current_function: Option<String>,
    pub ip: usize,
    pub registers: Vec<RuntimeValue>,
    pub call_stack: Vec<FrameInfo>,
}

/// 调用帧信息
#[derive(Debug, Clone)]
pub struct FrameInfo {
    pub function: String,
    pub ip: usize,
    pub locals: Vec<(String, RuntimeValue)>,
}
```

### 3.3 共享组件设计

```rust
// src/backends/common/mod.rs

pub mod opcode;           // 统一的操作码定义
pub mod value;            // RuntimeValue 类型
pub mod heap;             // 堆存储
pub mod allocator;        // 内存分配器

// 公共 trait
pub trait Value: Clone + std::fmt::Debug + PartialEq {}

// 公共常量
pub const GENERAL_PURPOSE_REGS: usize = 32;
```

### 3.4 运行时模块设计

> **设计依据**: RFC-008（运行时并发模型）+ RFC-009（所有权模型）
> - ❌ **无 GC** - 使用 `ref=Arc` 进行内存管理
> - 三层运行时：Embedded / Standard / Full

#### 3.4.1 任务调度 (`runtime/task.rs`)

```rust
/// 任务 ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(usize);

/// 任务优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum TaskPriority {
    #[default] Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// 任务状态
#[derive(Debug, Clone, PartialEq)]
pub enum TaskState {
    Pending,      // 创建但未开始
    Running,      // 正在执行
    Completed,    // 成功完成
    Failed(String), // 失败
    Cancelled,    // 被取消
}

/// 任务配置
#[derive(Debug, Clone, Default)]
pub struct TaskConfig {
    pub priority: TaskPriority,
    pub name: String,
    pub stack_size: usize,
    pub parent_id: Option<TaskId>,  // 任务树支持
}

/// 任务
///
/// Per RFC-009: 任务边界 = 泄漏边界
/// 任务内循环引用允许，任务结束后一起释放
#[derive(Debug)]
pub struct Task {
    id: TaskId,
    config: TaskConfig,
    state: TaskState,
    result: Option<Result<Arc<dyn Any + Send + Sync>, Arc<dyn Any + Send + Sync>>>,
}

/// 任务上下文（解释器执行状态）
#[derive(Debug, Default)]
pub struct TaskContext {
    task_id: TaskId,
    registers: Vec<Arc<dyn Any + Send + Sync>>,
    stack: Vec<Arc<dyn Any + Send + Sync>>,
    locals: HashMap<usize, Arc<dyn Any + Send + Sync>>,
    entry_ip: usize,
}

/// 调度器 Trait（泛型脱耦 per RFC-008）
pub trait Scheduler: Send + Sync {
    fn spawn(&self, task: Arc<Task>, config: TaskConfig) -> TaskId;
    fn await_task(&self, task_id: TaskId) -> Result<(), RuntimeError>;
    fn spawn_with_deps(&self, task: Arc<Task>, config: TaskConfig, deps: &[TaskId]) -> TaskId;
    fn await_all(&self, task_ids: &[TaskId]) -> Result<(), RuntimeError>;
    fn cancel(&self, task_id: TaskId) -> Result<(), RuntimeError>;
    fn is_complete(&self, task_id: TaskId) -> bool;
    fn stats(&self) -> SchedulerStats;
}

/// 任务 Spawner（主入口）
///
/// 使用泛型支持不同调度器实现：
/// - SingleThreadScheduler: 异步执行（num_workers=1）
/// - MultiThreadScheduler: 并行执行（num_workers>1）
#[derive(Debug)]
pub struct TaskSpawner<S: Scheduler> {
    scheduler: Arc<S>,
    next_id: usize,
}
```

> **三层运行时支持**:
> - **Embedded Runtime**: 即时执行器，无 DAG，同步执行
> - **Standard Runtime**: DAG 调度器，惰性求值，异步/并发
> - **Full Runtime**: + WorkStealer，并行优化

---

## 四、重构步骤

### 阶段 1：创建后端目录结构

```bash
# 1. 创建目录结构
mkdir -p src/backends/common
mkdir -p src/backends/interpreter
mkdir -p src/backends/dev

# 2. 创建模块文件
touch src/backends/mod.rs
touch src/backends/common/mod.rs
touch src/backends/common/opcode.rs
touch src/backends/common/value.rs
touch src/backends/common/heap.rs
touch src/backends/common/allocator.rs
touch src/backends/interpreter/mod.rs
touch src/backends/interpreter/executor.rs
touch src/backends/interpreter/registers.rs
touch src/backends/interpreter/frames.rs
touch src/backends/dev/mod.rs
touch src/backends/dev/debugger.rs
touch src/backends/dev/repl.rs
touch src/backends/dev/shell.rs

# 3. 创建低层 IR 文件
touch src/middle/bytecode.rs
```

### 阶段 2：创建共享组件

1. **创建统一的 `Opcode` 定义** (`backends/common/opcode.rs`)
   - 将 `vm/opcode.rs` 复制并重命名为 `Opcode`
   - 移除 VM 特定部分

2. **创建共享的 `Value` 类型** (`backends/common/value.rs`)
   - 将 `runtime/value/runtime_value.rs` 复制过来
   - 添加 `Value` trait

3. **创建共享的 `Heap` 类型** (`backends/common/heap.rs`)
   - 将 `runtime/value/heap.rs` 复制过来

4. **创建共享的 `Allocator`** (`backends/common/allocator.rs`)
   - 将 `runtime/memory/allocator.rs` 复制过来

### 阶段 3：创建低层 IR

1. **创建 `src/middle/bytecode.rs`**
   - 定义 `BytecodeInstr` 枚举
   - 定义 `BytecodeFunction` 结构体
   - 定义 `BytecodeModule` 结构体

2. **修改代码生成器**
   - `codegen/ir_builder.rs` 产出 `BytecodeInstr` 而非直接产出字节码

### 阶段 4：创建解释器后端

1. **精简解释器** (`interpreter/executor.rs`)
   - 接收 `BytecodeModule` 而非 `CompiledModule`
   - 约 800 行（当前 VM 的 1/3）

2. **提取寄存器文件** (`interpreter/registers.rs`)
   - 简化后的 `RegisterFile`

3. **保留调用帧** (`interpreter/frames.rs`)
   - 精简后的 `Frame`

### 阶段 5：创建开发专用模块

1. **调试器** (`dev/debugger.rs`)
   - 实现 `DebuggableExecutor` trait
   - 断点管理、单步执行

2. **REPL** (`dev/repl.rs`)
   - 交互式输入处理
   - 历史记录

3. **Shell** (`dev/shell.rs`)
   - 组合调试器和 REPL
   - 提供 CLI 界面

### 阶段 6：更新导出和入口

```rust
// src/lib.rs

pub mod front;        // 前端 (原 frontend)
pub mod middle;       // 中层
pub mod backends;     // 后端抽象层

// 便捷导出
pub use backends::{Executor, DebuggableExecutor};
pub use backends::interpreter::Interpreter;
pub use backends::dev::{DevShell, Debugger};
```

---

## 五、关键文件变化

### 5.1 文件移动

| 原位置 | 新位置 | 说明 |
|--------|--------|------|
| `vm/opcode.rs` | `backends/common/opcode.rs` | 重命名为 `Opcode` |
| `runtime/value/runtime_value.rs` | `backends/common/value.rs` | 添加 `Value` trait |
| `runtime/value/heap.rs` | `backends/common/heap.rs` | 保持不变 |
| `runtime/memory/allocator.rs` | `backends/common/allocator.rs` | 重导出 |

### 5.2 文件拆分

| 原文件 | 拆分后 |
|--------|--------|
| `vm/executor.rs` (2400行) | `interpreter/executor.rs` (800行) + `dev/debugger.rs` (500行) + `dev/shell.rs` (300行) |
| `vm/frames.rs` | `interpreter/frames.rs` |

### 5.3 新增文件

| 文件 | 行数估算 | 说明 | 当前状态 |
|------|----------|------|----------|
| `middle/bytecode.rs` | 300行 | 低层 IR 定义 | ✅ 已实现 |
| `backends/mod.rs` | 100行 | trait 定义 | ✅ 已实现 |
| `backends/common/opcode.rs` | 400行 | 统一操作码 | ✅ 已实现 |
| `backends/common/value.rs` | 500行 | RuntimeValue | ✅ 已实现 |
| `backends/common/heap.rs` | 200行 | Handle 堆存储 | ✅ 已实现 |
| `backends/common/allocator.rs` | 100行 | 分配器接口 | ✅ 已实现 |
| `backends/interpreter/executor.rs` | 800行 | 精简解释器 | ✅ 已实现 |
| `backends/interpreter/registers.rs` | 150行 | 寄存器文件 | ✅ 已实现 |
| `backends/interpreter/frames.rs` | 150行 | 调用帧 | ✅ 已实现 |
| `backends/dev/debugger.rs` | 500行 | 调试功能 | ✅ 已实现 |
| `backends/dev/repl.rs` | 300行 | REPL 功能 | ✅ 已实现 |
| `backends/dev/shell.rs` | 300行 | Shell 界面 | ✅ 已实现 |
| `backends/runtime/mod.rs` | 20行 | 运行时模块 | ✅ 已实现 |
| `backends/runtime/task.rs` | 300行 | 任务调度（DAG调度器） | ✅ 已实现（RFC-008） |

---

## 六、API 变化

### 6.1 当前 API

```rust
// 当前
use yaoxiang::{run, VM, CompiledModule};

let mut vm = VM::new();
vm.execute_module(&compiled)?;
```

### 6.2 新 API

```rust
// 生产模式：使用解释器
use yaoxiang::backends::interpreter::Interpreter;

let mut interpreter = Interpreter::new();
interpreter.execute_module(&bytecode_module)?;

// 开发模式：使用 DevShell
use yaoxiang::backends::dev::DevShell;

let mut shell = DevShell::new();
shell.load_module(&bytecode_module)?;
shell.set_breakpoint(100);
shell.run()?;

// 调试模式
use yaoxiang::backends::Debugger;

let mut debugger = Debugger::new();
debugger.load_module(&bytecode_module)?;
while let Some(state) = debugger.step()? {
    println!("{:?}", state.registers);
}
```

---

## 七、风险评估

| 风险 | 级别 | 缓解措施 |
|------|------|----------|
| 重构期间 regression | 高 | 每个阶段运行完整测试 |
| API 破坏性变更 | 中 | 提供兼容层（type alias） |
| 性能下降 | 低 | 解释器保持相同实现 |
| 工作量过大 | 高 | 分阶段执行，每个阶段可独立工作 |

---

## 八、预期收益

1. **清晰的架构**：前后端分离，职责明确
2. **可维护性**：巨型文件拆分为小模块
3. **可扩展性**：添加 AOT/JIT 后端只需实现 trait
4. **可测试性**：每个组件可独立测试
5. **开发友好**：调试功能和 REPL 分离

---

## 九、启动指令

```bash
# 阶段 1：创建目录结构
mkdir -p src/backends/common src/backends/interpreter src/backends/dev src/backends/runtime

# 创建模块文件
touch src/backends/mod.rs
touch src/backends/common/mod.rs
touch src/backends/common/opcode.rs
touch src/backends/common/value.rs
touch src/backends/common/heap.rs
touch src/backends/common/allocator.rs
touch src/backends/interpreter/mod.rs
touch src/backends/interpreter/executor.rs
touch src/backends/interpreter/registers.rs
touch src/backends/interpreter/frames.rs
touch src/backends/dev/mod.rs
touch src/backends/dev/debugger.rs
touch src/backends/dev/repl.rs
touch src/backends/dev/shell.rs
touch src/backends/runtime/mod.rs
touch src/backends/runtime/task.rs

# 创建低层 IR 文件
touch src/middle/bytecode.rs

# 阶段 2：验证编译
cargo check

# 阶段 3：开始迁移共享组件
# ...
```

---

## 十、阶段详细任务

### 阶段 1：目录结构创建

- [x] 创建 `src/backends/` 目录
- [x] 创建 `src/backends/common/` 共享组件目录
- [x] 创建 `src/backends/interpreter/` 解释器目录
- [x] 创建 `src/backends/dev/` 开发工具目录
- [x] 创建 `src/backends/runtime/` 运行时目录
- [x] 创建所有模块文件（空文件即可）
- [x] 运行 `cargo check` 验证结构正确

### 阶段 2：共享组件迁移

- [ ] 迁移 `Opcode` 定义
- [ ] 迁移 `RuntimeValue` 类型
- [ ] 迁移 `Heap` 存储
- [ ] 迁移 `Allocator` 分配器
- [ ] 创建 `backends/common/mod.rs` 统一导出
- [ ] 运行测试验证迁移正确

### 阶段 3：低层 IR 设计

- [ ] 设计 `BytecodeInstr` 枚举
- [ ] 设计 `BytecodeFunction` 结构体
- [ ] 设计 `BytecodeModule` 结构体
- [ ] 实现 IR 到字节码的序列化
- [ ] 更新代码生成器使用新 IR

### 阶段 4：解释器后端实现

- [ ] 实现 `Executor` trait
- [ ] 创建精简的寄存器文件
- [ ] 创建精简的调用帧
- [ ] 实现指令解释循环
- [ ] 验证功能正确性

### 阶段 5：开发工具实现

- [ ] 实现 `DebuggableExecutor` trait
- [ ] 实现断点管理
- [ ] 实现单步执行
- [ ] 实现 REPL 交互
- [ ] 实现 Shell 界面

### 阶段 6：API 更新

- [ ] 更新 `lib.rs` 导出
- [ ] 更新 `main.rs` 使用新 API
- [ ] 更新文档和示例
- [ ] 运行完整测试套件

---

## 十一、后续扩展

### 未来可能的扩展方向

1. **AOT 后端** (`backends/aot/`)
   - 将 `BytecodeIR` 编译为机器码
   - 利用 LLVM 或 Cranelift

2. **JIT 后端** (`backends/jit/`)
   - 运行时编译热点函数
   - 与解释器混合使用

3. **WASM 后端** (`backends/wasm/`)
   - 编译为 WebAssembly
   - 浏览器环境运行

### Runtime 模块完善计划

> **说明**: 根据 RFC-009，YaoXiang **无 GC**，使用 `ref=Arc` 内存管理。

| 组件 | 当前状态 | 目标状态 | 优先级 |
|------|----------|----------|--------|
| `TaskSpawner<SingleThreadScheduler>` | ✅ 已实现 | 单线程异步调度 | - |
| `TaskSpawner<MultiThreadScheduler>` | ❌ 未实现 | 多线程并行调度 | 高 |
| `WorkStealer` | ❌ 未实现 | Full Runtime 负载均衡 | 高 |
| `DAG Scheduler` | ❌ 未实现 | Standard Runtime 惰性求值 | 高 |
| `TaskContext` | ✅ 已实现 | 完整 TLS 支持 | 中 |

---

**文档版本**: 1.2
**创建日期**: 2026-01-23
**最后更新**: 2026-01-23
**状态**: 后端抽象层已实现，运行时模块已按 RFC-008/009 重新设计（无 GC）
