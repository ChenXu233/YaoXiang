# YaoXiang 项目架构文档

> 版本：v2.0.0
> 状态：正式
> 作者：沫郁酱
> 日期：2025-01-04

---

## 目录

1. [概述](#一概述)
2. [目录结构](#二目录结构)
3. [核心模块](#三核心模块)
4. [编译器架构](#四编译器架构)
5. [运行时架构](#五运行时架构)
6. [模块依赖关系](#六模块依赖关系)
7. [开发工作流](#七开发工作流)

---

## 一、概述

YaoXiang 是一个采用 Rust 编写的实验性编程语言项目，采用现代编译器架构设计。项目整体遵循清晰的分层架构，从前端词法分析到后端代码生成，再到运行时执行，每个阶段都有明确的职责边界。

### 设计原则

- **模块化**：每个组件独立，易于测试和替换
- **分层架构**：前端、中端、后端清晰分离
- **零成本抽象**：编译时优化，运行时无额外开销
- **可扩展性**：支持 JIT/AOT 编译，易于添加新特性

### 技术栈

- **语言**：Rust 2021 Edition
- **构建工具**：Cargo
- **测试框架**：builtin test + proptest + quickcheck
- **并发支持**：parking_lot, crossbeam, rayon

---

## 二、目录结构

```
YaoXiang/
├── .claude/                      # Claude AI 配置
│   └── plan/                     # 任务计划
│
├── .github/                      # GitHub 配置
│   └── workflows/
│
├── docs/                         # 文档系统
│   ├── architecture/             # 架构文档
│   │   ├── project-structure.md  # 项目结构（本文档）
│   │   ├── compiler-design.md    # 编译器设计
│   │   └── runtime-design.md     # 运行时设计
│   ├── guides/                   # 用户指南
│   │   ├── getting-started.md    # 快速开始
│   │   ├── error-system-design.md# 错误系统
│   │   └── dev/                  # 开发者指南
│   │       ├── commit-convention.md
│   │       └── release-guide.md
│   ├── works/                    # 工作文档
│   │   ├── old/                  # 历史方案
│   │   ├── phase/                # 阶段性文档
│   │   └── plans/                # 规划文档
│   └── YaoXiang-book.md          # 语言指南
│
├── src/                          # 源代码
│   ├── frontend/                 # 前端：词法分析、语法分析、类型检查
│   │   ├── lexer/
│   │   │   ├── mod.rs            # 词法分析器入口
│   │   │   ├── tokens.rs         # Token 定义
│   │   │   └── tests/
│   │   ├── parser/
│   │   │   ├── mod.rs            # 解析器入口和状态管理
│   │   │   ├── ast.rs            # AST 节点定义
│   │   │   ├── expr.rs           # 表达式解析
│   │   │   ├── nud.rs            # 前缀解析（Pratt Parser）
│   │   │   ├── led.rs            # 中缀解析（Pratt Parser）
│   │   │   ├── stmt.rs           # 语句解析
│   │   │   ├── state.rs          # 解析器状态
│   │   │   ├── type_parser.rs    # 类型解析
│   │   │   └── tests/
│   │   └── typecheck/
│   │       ├── mod.rs            # 类型检查入口
│   │       ├── types.rs          # 类型定义
│   │       ├── infer.rs          # 类型推断
│   │       ├── check.rs          # 类型验证
│   │       ├── specialize.rs     # 泛型特化
│   │       ├── errors.rs         # 类型错误
│   │       └── tests/
│   │
│   ├── middle/                   # 中端：优化、中间表示、代码生成
│   │   ├── mod.rs                # 中端模块入口
│   │   ├── ir.rs                 # 中间表示定义（单文件）
│   │   ├── optimizer.rs          # 优化器（单文件）
│   │   ├── codegen/              # 代码生成器
│   │   │   ├── mod.rs            # 代码生成入口
│   │   │   ├── bytecode.rs       # 字节码格式
│   │   │   ├── expr.rs           # 表达式代码生成
│   │   │   ├── stmt.rs           # 语句代码生成
│   │   │   ├── control_flow.rs   # 控制流代码生成
│   │   │   ├── loop_gen.rs       # 循环代码生成
│   │   │   ├── switch.rs         # 模式匹配代码生成
│   │   │   ├── closure.rs        # 闭包处理
│   │   │   ├── generator.rs      # 代码生成器核心
│   │   │   └── tests/
│   │   ├── monomorphize/         # 单态化
│   │   │   ├── mod.rs            # 单态化入口
│   │   │   ├── instance.rs       # 实例管理
│   │   │   └── tests/
│   │   ├── escape_analysis/      # 逃逸分析
│   │   │   └── mod.rs
│   │   └── lifetime/             # 生命周期分析
│   │       └── mod.rs
│   │
│   ├── backends/                # 后端模块
│   │   └── dev/
│   │       └── tui_repl/        # TUI REPL 开发工具
│   │
│   ├── std/                      # 标准库
│   │   └── mod.rs                # 标准库入口
│   │
│   ├── util/                     # 工具库
│   │   ├── mod.rs                # 工具入口
│   │   ├── span.rs               # 源码位置
│   │   ├── diagnostic.rs         # 诊断系统
│   │   └── cache.rs              # 缓存工具
│   │
│   ├── lib.rs                    # 库入口
│   └── main.rs                   # 可执行文件入口
│
├── tests/                        # 测试
│   ├── integration/              # 集成测试
│   │   ├── codegen.rs
│   │   └── execution.rs
│   └── unit/                     # 单元测试
│       └── codegen.rs
│
├── Cargo.toml                    # Rust 项目配置
├── Cargo.lock                    # 依赖锁定
├── clippy.toml                   # Clippy 配置
├── rustfmt.toml                  # 代码格式化配置
├── .gitignore
└── README.md
```

### 与旧版文档的差异说明

| 旧版描述 | 实际结构 | 说明 |
|---------|---------|------|
| `middle/optimizer/` | `middle/optimizer.rs` | 优化器为单文件，非目录 |
| `middle/ir/` | `middle/ir.rs` | IR 为单文件，非目录 |
| 无 `middle/lifetime/` | `middle/lifetime/` | 新增生命周期分析模块 |
| `vm/instructions.rs` | 已删除 | 已废弃，被 TypedOpcode 替代 |
| 无 `runtime/extfunc.rs` | `runtime/extfunc.rs` | 新增外部函数模块 |
| 无 `runtime/value/` | `runtime/value/` | 新增值类型模块 |

---

## 三、核心模块详解

### 3.1 前端 (src/frontend/)

前端负责将源代码转换为中间表示（IR），包含三个主要阶段：

#### 3.1.1 词法分析器 (lexer/)
**职责**：将源代码字符串转换为 Token 流

**核心文件**：
- `mod.rs` - 词法分析器入口
- `tokens.rs` - Token 定义和类型

**关键数据结构**：
```rust
// Token 定义
pub enum Token {
    Identifier(String),
    Integer(i64),
    Float(f64),
    String(String),
    BoolLiteral(bool),
    // 关键字
    Type, Pub, Use, Spawn, Ref, Mut,
    If, Elif, Else, Match, While, For, Return,
    // 符号
    LParen, RParen, LBrace, RBrace,
    Comma, Colon, Semicolon,
    // 运算符
    Equal, Arrow, Plus, Minus, Star, Slash,
    Eq, Ne, Lt, Le, Gt, Ge,
    // 特殊
    Eof,
}

// 词法分析器
pub struct Lexer {
    input: String,
    chars: Vec<char>,
    position: usize,
    line: usize,
    column: usize,
}
```

**处理流程**：
1. 按字符读取源代码
2. 识别关键字、标识符、字面量
3. 记录位置信息（行号、列号）
4. 生成 Token 流

#### 3.1.2 语法分析器 (parser/)
**职责**：将 Token 流转换为抽象语法树 (AST)

**核心文件**：
- `mod.rs` - 解析器入口和状态管理
- `ast.rs` - AST 节点定义
- `expr.rs` - 表达式解析
- `nud.rs` / `led.rs` - Pratt Parser 核心
- `stmt.rs` - 语句解析
- `state.rs` - 解析器状态
- `type_parser.rs` - 类型解析

**关键数据结构**：
```rust
// AST 节点
pub enum Expr {
    Literal(Literal),
    Identifier(String),
    Binary { left: Box<Expr>, op: BinOp, right: Box<Expr> },
    Lambda { params: Vec<Param>, body: Box<Expr> },
    Call { func: Box<Expr>, args: Vec<Expr> },
    Match { expr: Box<Expr>, arms: Vec<MatchArm> },
    If { cond: Box<Expr>, then_branch: Box<Expr>, else_branch: Option<Box<Expr>> },
    Block(Vec<Stmt>),
    // ...
}

pub enum Stmt {
    Let { name: String, ty: Option<Type>, value: Expr },
    Function { name: String, params: Vec<Param>, ret_ty: Type, body: Expr },
    TypeDef { name: String, variants: Vec<Variant> },
    Return(Option<Expr>),
    // ...
}

// 解析器
pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
    state: ParserState,
}
```

**解析策略**：
- **Pratt Parser**：处理表达式的优先级和结合性
- **递归下降**：处理语句和声明
- **状态管理**：使用 `ParserState` 跟踪解析上下文

#### 3.1.3 类型检查器 (typecheck/)
**职责**：验证 AST 的类型正确性，进行类型推断

**核心文件**：
- `mod.rs` - 类型检查入口
- `types.rs` - 类型定义
- `infer.rs` - 类型推断
- `check.rs` - 类型验证
- `specialize.rs` - 泛型特化
- `errors.rs` - 类型错误
- `tests/` - 测试用例

**关键数据结构**：
```rust
// 类型定义
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    // 原子类型
    Void, Bool, Int, Float, Char, String,

    // 复合类型
    List(Box<Type>),
    Dict(Box<Type>, Box<Type>),
    Tuple(Vec<Type>),

    // 函数类型
    Fn { params: Vec<Type>, ret: Box<Type>, is_async: bool },

    // 类型引用
    TypeRef(String),

    // 类型变量（用于推断）
    Variable(TypeVar),

    // 泛型
    Generic { name: String, args: Vec<Type> },
}

// 类型变量
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeVar(pub u32);

// 类型上下文
pub struct TypeContext {
    pub vars: HashMap<String, TypeScheme>,
    pub structs: HashMap<String, StructDef>,
    pub constraints: Vec<Constraint>,
    pub next_var: u32,
}
```

**核心算法**：
- **Hindley-Milner 类型推断**：支持泛型和多态
- **约束求解**：解决类型变量的约束
- **单态化**：将泛型函数展开为具体版本

### 3.2 中端 (src/middle/)

中端负责从 AST 生成优化的字节码。

#### 3.2.1 中间表示 (ir.rs)
**职责**：定义编译器内部的中间表示形式（单文件模块）

**关键数据结构**：
```rust
// IR 模块
pub struct ModuleIR {
    pub types: Vec<Type>,
    pub constants: Vec<ConstValue>,
    pub globals: Vec<GlobalIR>,
    pub functions: Vec<FunctionIR>,
}

// IR 指令
#[derive(Debug, Clone)]
pub enum Instruction {
    // 移动和加载
    Move { dst: Operand, src: Operand },
    Load { dst: Operand, src: Operand },
    Store { dst: Operand, src: Operand },

    // 算术运算
    Add { dst: Operand, lhs: Operand, rhs: Operand },
    Sub { dst: Operand, lhs: Operand, rhs: Operand },
    Mul { dst: Operand, lhs: Operand, rhs: Operand },
    Div { dst: Operand, lhs: Operand, rhs: Operand },
    Mod { dst: Operand, lhs: Operand, rhs: Operand },
    Neg { dst: Operand, src: Operand },

    // 比较
    Eq { dst: Operand, lhs: Operand, rhs: Operand },
    Ne { dst: Operand, lhs: Operand, rhs: Operand },
    Lt { dst: Operand, lhs: Operand, rhs: Operand },
    Le { dst: Operand, lhs: Operand, rhs: Operand },
    Gt { dst: Operand, lhs: Operand, rhs: Operand },
    Ge { dst: Operand, lhs: Operand, rhs: Operand },

    // 控制流
    Jmp(u32),
    JmpIf(Operand, u32),
    JmpIfNot(Operand, u32),
    Ret(Option<Operand>),

    // 函数调用
    Call { dst: Option<Operand>, func: Operand, args: Vec<Operand> },
    TailCall { func: Operand, args: Vec<Operand> },

    // 内存操作
    Alloc { dst: Operand, size: Operand },
    Free(Operand),
    HeapAlloc { dst: Operand, type_id: u32 },

    // 类型操作
    Cast { dst: Operand, src: Operand, target_type: u32 },
    TypeTest(Operand, u32),

    // 并发
    Spawn { func: Operand },
    Await(Operand),
    Yield,

    // 闭包
    MakeClosure { dst: Operand, func: Operand, env: Vec<Operand> },
}
```

#### 3.2.2 代码生成器 (codegen/)
**职责**：将 IR 转换为类型化字节码

**核心文件**：
- `mod.rs` - 代码生成入口和配置
- `bytecode.rs` - 字节码文件格式
- `expr.rs` - 表达式代码生成
- `stmt.rs` - 语句代码生成
- `control_flow.rs` - 控制流代码生成
- `loop_gen.rs` - 循环代码生成
- `switch.rs` - 模式匹配代码生成
- `closure.rs` - 闭包处理
- `generator.rs` - 代码生成器核心
- `tests/` - 测试用例

**关键数据结构**：
```rust
// 代码生成上下文
pub struct CodegenContext {
    module: ModuleIR,
    symbol_table: SymbolTable,
    constant_pool: ConstantPool,
    bytecode: Vec<u8>,
    current_function: Option<FunctionIR>,
    register_allocator: RegisterAllocator,
    label_generator: LabelGenerator,
    config: CodegenConfig,
}

// 字节码文件格式
pub struct BytecodeFile {
    pub header: BytecodeHeader,
    pub type_table: Vec<MonoType>,
    pub const_pool: Vec<ConstValue>,
    pub code_section: CodeSection,
}

// 字节码指令
pub struct BytecodeInstruction {
    pub opcode: TypedOpcode,
    pub operands: Vec<u8>,
}

// 操作码（带类型）
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TypedOpcode {
    Nop, Mov,
    // 整数运算
    I64Add, I64Sub, I64Mul, I64Div, I64Rem, I64Neg,
    I64Eq, I64Ne, I64Lt, I64Le, I64Gt, I64Ge,
    // 内存操作
    StackAlloc, HeapAlloc, LoadConst,
    // 控制流
    Jmp, JmpIf, JmpIfNot, Return, ReturnValue,
    // 函数调用
    Call, TailCall,
    // 列表操作
    NewList, NewListWithCap, LoadElement, StoreElement,
    // 类型操作
    Cast, TypeCheck,
    // 闭包
    MakeClosure, LoadEnv,
    // 其他
    Drop, Yield,
}
```

**设计特点**：
- **类型化指令**：每条指令携带明确的类型信息
- **寄存器架构**：所有操作在寄存器上进行
- **单态化输出**：泛型已在编译期展开

#### 3.2.3 优化器 (optimizer.rs)
**职责**：对 IR 进行各种优化（单文件模块）

**优化类型**：
- **常量折叠**：编译时计算常量表达式
- **死代码消除**：移除不可达代码
- **公共子表达式消除**：避免重复计算
- **代数简化**：优化运算表达式

#### 3.2.4 单态化 (monomorphize/)
**职责**：将泛型代码转换为具体版本

**核心文件**：
- `mod.rs` - 单态化入口
- `instance.rs` - 实例管理
- `tests/` - 测试用例

#### 3.2.5 逃逸分析 (escape_analysis/)
**职责**：分析值的生命周期，优化内存分配

**核心文件**：
- `mod.rs` - 逃逸分析器

**作用**：
- 确定哪些值可以栈分配
- 识别需要堆分配的值
- 优化所有权转移

#### 3.2.6 生命周期分析 (lifetime/)
**职责**：分析值的生命周期和作用域

**核心文件**：
- `mod.rs` - 生命周期分析器

### 3.3 运行时 (src/middle/)

运行时负责程序执行时的资源管理和任务调度。

#### 3.3.1 并作图 (dag/)
**职责**：管理并作任务的依赖关系

**核心文件**：
- `mod.rs` - DAG 模块入口
- `node.rs` - 节点定义
- `node_id.rs` - 节点 ID 管理
- `graph.rs` - 图操作
- `tests/` - 测试用例

**关键数据结构**：
```rust
// 并作图节点
pub struct DagNode {
    pub id: NodeId,
    pub task_id: Option<TaskId>,
    pub deps: Vec<NodeId>,
    pub status: NodeStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeStatus {
    Pending,  // 等待依赖
    Ready,    // 可以执行
    Running,  // 执行中
    Completed,// 已完成
    Failed,   // 失败
}

// 并作图
pub struct Dag {
    nodes: HashMap<NodeId, DagNode>,
    edges: HashMap<NodeId, HashSet<NodeId>>,
    node_counter: u32,
}
```

#### 3.3.2 任务调度器 (scheduler/)
**职责**：并作任务的调度和执行

**核心文件**：
- `mod.rs` - 调度器入口
- `task.rs` - 任务定义
- `queue.rs` - 任务队列
- `work_stealer.rs` - 工作窃取算法
- `tests/` - 测试用例

**调度策略**：
- **工作窃取 (Work Stealing)**：多线程负载均衡
- **惰性求值**：按需执行
- **优先级调度**：重要任务优先

**关键数据结构**：
```rust
// 任务
pub struct Task {
    pub id: TaskId,
    pub func_idx: FuncId,
    pub args: Vec<Value>,
    pub status: TaskStatus,
    pub result: Option<Value>,
}

// 调度器
pub struct Scheduler {
    pub workers: Vec<Worker>,
    pub global_queue: TaskQueue,
    pub tasks: HashMap<TaskId, Task>,
    pub dag: Dag,
    pub config: SchedulerConfig,
}
```

#### 3.3.3 内存管理 (memory/)
**职责**：堆分配、内存布局

**核心文件**：
- `mod.rs` - 内存管理入口
- `tests/` - 内存测试

**内存策略**：
- **所有权模型**：编译时确定生命周期
- **RAII**：自动资源管理
- **可选 GC**：基于引用计数或标记清除

### 3.4 虚拟机 (src/middle/)

虚拟机负责执行字节码。

#### 3.4.1 执行器 (executor.rs)
**职责**：执行字节码

**核心功能**：
- 指令分发
- 栈管理
- 寄存器操作
- 异常处理

**关键数据结构**：
```rust
// 虚拟机
pub struct VM {
    pub stack: Vec<Value>,
    pub frames: Vec<CallFrame>,
    pub ip: usize,
    pub constant_pool: Vec<Value>,
    pub globals: HashMap<String, Value>,
    pub config: VMConfig,
}

// 调用帧
pub struct CallFrame {
    pub func_idx: u32,
    pub return_addr: u32,
    pub base_ptr: u32,
    pub closure: Option<Closure>,
}

// 值类型（使用 Runtime 提供的 RuntimeValue）
// 详见 src/middle/value/runtime_value.rs
```

#### 3.4.2 调用帧 (frames.rs)
**职责**：管理函数调用栈

**关键数据结构**：
```rust
pub struct CallFrame {
    pub func_idx: u32,          // 函数索引
    pub return_addr: u32,       // 返回地址
    pub base_ptr: u32,          // 栈基址
    pub closure: Option<Closure>, // 闭包环境
}
```

#### 3.4.3 操作码 (opcode.rs)
**职责**：定义 TypedOpcode 强类型操作码

#### 3.4.5 内联缓存 (inline_cache.rs)
**职责**：类型检查结果缓存，加速动态分发

**关键数据结构**：
```rust
pub struct InlineCache {
    pub caches: HashMap<(TypeId, String), FuncId>,
}
```

### 3.5 标准库 (src/std/)

**核心文件**：
- `mod.rs` - 标准库入口
- `io.rs` - 文件、控制台 I/O
- `string.rs` - 字符串操作
- `list.rs` - 动态数组
- `dict.rs` - 哈希表
- `math.rs` - 数学函数
- `concurrent.rs` - 并发原语（Channel, Mutex）
- `net.rs` - 网络编程（实验性）

### 3.6 工具库 (src/util/)

**核心文件**：
- `mod.rs` - 工具入口
- `span.rs` - 源码位置信息
- `diagnostic.rs` - 诊断系统
- `cache.rs` - 缓存工具

---

## 四、编译器架构

### 4.1 编译流程

```
源代码
  ↓
[词法分析] → Token 流
  ↓
[语法分析] → AST
  ↓
[类型检查] → 带类型的 AST + 类型约束
  ↓
[IR 生成] → IR (SSA 形式)
  ↓
[优化] → 优化后的 IR
  ↓
[单态化] → 具体 IR
  ↓
[逃逸分析] → 内存信息
  ↓
[代码生成] → 字节码
  ↓
[虚拟机执行] → 运行结果
```

### 4.2 关键设计决策

#### 4.2.1 双层处理策略
```
解析层（Frontend）
├─ 只验证语法结构
├─ 宽松处理，不报类型错误
└─ 生成结构化 AST

类型检查层（Typecheck）
├─ 验证语义正确性
├─ 严格的类型推断
└─ 保证运行时安全
```

#### 4.2.2 字节码 vs AOT
- **当前**：字节码 + 虚拟机（灵活性高）
- **未来**：可扩展为 AOT 编译（性能更高）
- **设计**：中间表示可切换后端

#### 4.2.3 并作模型实现
- **编译时**：构建依赖图 (DAG)
- **运行时**：惰性求值 + 自动等待
- **调度器**：工作窃取 + 优先级

---

## 五、运行时架构

### 5.1 并作执行模型

```
用户代码（同步思维）
  ↓
编译器（构建 DAG）
  ↓
运行时（并行执行）
  ├─ 节点 A（独立任务）
  ├─ 节点 B（依赖 A）
  └─ 节点 C（依赖 A 和 B）
  ↓
自动等待（在需要时）
  ↓
结果合并
```

### 5.2 内存布局

```
栈（Stack）
├─ 局部变量
├─ 函数调用帧
└─ 小对象（逃逸分析确定）

堆（Heap）
├─ 大对象
└─ 长生命周期对象

常量池（Constant Pool）
├─ 字符串字面量
└─ 数值字面量
```

### 5.3 线程模型

```
主线程
├─ 执行器（Executor）
├─ 调度器（Scheduler）
└─ 任务队列

工作线程池（可选）
├─ 工作窃取队列
└─ 负载均衡

阻塞线程池
├─ 文件 I/O
├─ 网络 I/O
└─ 其他阻塞操作
```

---

## 六、模块依赖关系

### 6.1 编译阶段依赖

```
frontend/lexer
  ↓ (被 parser 依赖)

frontend/parser
  ↓ (依赖 lexer, 输出 AST)
  ↓ (被 typecheck 依赖)

frontend/typecheck
  ↓ (依赖 parser, 输出带类型 AST)
  ↓ (被 middle 依赖)

middle/ir
  ↓ (依赖 typecheck)
  ↓ (被 optimizer, monomorphize, codegen 依赖)

middle/optimizer.rs
  ↓ (依赖 ir, 修改 IR)

middle/monomorphize
  ↓ (依赖 ir, 生成具体 IR)

middle/escape_analysis
  ↓ (依赖 ir)

middle/lifetime
  ↓ (依赖 ir)

middle/codegen
  ↓ (依赖 ir, 生成字节码)
  ↓ (输出给 vm)

vm/
  ↓ (执行字节码)

runtime/
  ↓ (提供调度器和内存管理)
  ↓ (被 vm 依赖)
```

### 6.2 运行时依赖

```
vm/executor
  ↓ (依赖 vm/frames, vm/opcode)
  ↓ (依赖 runtime/value::RuntimeValue)
  ↓ (依赖 runtime/extfunc)

runtime/scheduler
  ↓ (依赖 runtime/task, runtime/dag)

runtime/dag
  ↓ (管理并作依赖)

runtime/value
  ↓ (RuntimeValue 类型定义，被所有模块依赖)

runtime/extfunc
  ↓ (外部函数，被 vm/executor 依赖)
```

### 6.3 Cargo.toml 依赖关系

**直接依赖**：
- `parking_lot` - 高性能锁
- `crossbeam` - 并发数据结构
- `rayon` - 数据并行
- `once_cell` - 单次初始化
- `indexmap` - 有序哈希表
- `hashbrown` - 高性能哈希表
- `smallvec` - 栈上小向量
- `regex` - 正则表达式
- `serde` / `ron` - 序列化
- `anyhow` / `thiserror` - 错误处理
- `clap` - 命令行解析

---

## 七、开发工作流

### 7.1 添加新特性流程

```bash
# 1. 设计阶段
docs/plans/your-feature.md  # 设计文档

# 2. 前端实现
src/frontend/lexer/tokens.rs     # 添加 Token
src/frontend/parser/ast.rs       # 添加 AST 节点
src/frontend/parser/expr.rs      # 添加解析逻辑
src/frontend/typecheck/          # 添加类型规则

# 3. 中端实现
src/middle/ir.rs                 # 扩展 IR
src/middle/codegen/              # 生成字节码

# 4. 运行时实现
src/middle/extfunc.rs           # 添加外部函数
src/middle/executor.rs               # 添加执行逻辑
src/middle/opcode.rs                 # 添加操作码

# 5. 测试
tests/unit/codegen.rs            # 单元测试
tests/integration/               # 集成测试

# 6. 文档更新
docs/YaoXiang-book.md            # 语言指南
docs/architecture/               # 架构文档
```

### 7.2 代码质量检查

```bash
# 格式化
cargo fmt

# Clippy 检查
cargo clippy --all-targets --all-features

# 运行测试
cargo test

# 性能分析
cargo bench
```

### 7.3 版本管理

```
Git 分支策略：
├─ main          # 稳定版本
├─ develop       # 开发分支
├─ feature/*     # 功能分支
└─ hotfix/*      # 紧急修复
```

---

## 八、关键技术决策

### 8.1 为什么使用 Rust？

- **内存安全**：无 GC，无数据竞争
- **零成本抽象**：高性能
- **强类型系统**：编译时保证
- **优秀的工具链**：Cargo, Clippy, rustfmt

### 8.2 为什么选择字节码？

- **跨平台**：一次编译，到处运行
- **可移植性**：易于移植到不同架构
- **灵活性**：支持 JIT 和 AOT
- **调试友好**：易于实现调试器

### 8.3 为什么采用并作模型？

- **自然编程**：同步思维，并发执行
- **零认知负担**：无需管理线程/协程
- **自动优化**：编译器自动提取并行性
- **高性能**：充分利用多核

---

## 九、未来扩展方向

### 9.1 编译器优化
- [ ] LLVM 后端支持（AOT 编译）
- [ ] 更激进的优化（循环展开、向量化）
- [ ] 编译时间优化（增量编译）
- [ ] 错误恢复（继续编译）

### 9.2 语言特性
- [ ] 宏系统（卫生宏）
- [ ] 模块系统完善（包管理器）
- [ ] FFI 支持（C 互操作）
- [ ] 反射系统

### 9.3 工具链
- [ ] LSP 服务器（IDE 支持）
- [ ] 调试器（断点、单步）
- [ ] 性能分析工具
- [ ] 包管理器

### 9.4 运行时
- [ ] GC 优化（分代 GC）
- [ ] 实时编译 (JIT)
- [ ] 并发模型优化
- [ ] 异步 I/O 集成

---

## 附录

### A. 术语表

| 术语 | 解释 |
|------|------|
| 并作 (Spawn) | YaoXiang 的异步并发模型 |
| 并作图 (DAG) | 并发任务的依赖关系图 |
| 柯里化 | 将多参数函数转换为单参数函数链 |
| 单态化 | 泛型函数展开为具体版本 |
| 逃逸分析 | 分析值的生命周期和分配位置 |

### B. 常用命令

```bash
# 构建项目
cargo build
cargo build --release

# 运行测试
cargo test
cargo test --test integration

# 代码分析
cargo clippy
cargo doc --open

# 性能分析
cargo bench
cargo flamegraph
```

### C. 贡献指南

1. 阅读架构文档理解整体设计
2. 查看现有代码风格和模式
3. 编写测试保证代码质量
4. 更新相关文档
5. 提交 Pull Request

---

**文档维护**：此文档应随项目演进同步更新，特别是添加新模块或修改架构时。

**最后更新**：2025-01-04
