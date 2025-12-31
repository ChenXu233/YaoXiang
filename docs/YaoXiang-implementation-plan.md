# YaoXiang（爻象）实现计划

> 版本：v1.0.0
> 状态：实现规划
> 作者：沫郁酱
> 日期：2024-12-31

---

## 一、概述

### 1.1 文档目的

本文档详细规划了 YaoXiang 编程语言的高性能解释器实现方案，包括核心架构设计、关键技术选型、优化策略，以及未来编译器和自举的演进路线图。

### 1.2 设计目标

| 目标 | 要求 |
|------|------|
| 性能 | 解释器运行速度达到原生代码的 50% 以上 |
| 启动 | 启动时间控制在 100 毫秒以内 |
| 内存 | 内存占用低于同功能 Rust 程序的 2 倍 |
| 兼容性 | 支持 YaoXiang 语言规范的全部特性 |

### 1.3 整体架构

```
┌─────────────────────────────────────────────────────────────────────┐
│                         YaoXiang 实现架构                           │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────────────────┐ │
│  │   前端层    │ →  │   中间层    │ →  │        后端层           │ │
│  ├─────────────┤    ├─────────────┤    ├─────────────────────────┤ │
│  │ • 词法分析  │    │ • AST 优化  │    │ • 字节码虚拟机          │ │
│  │ • 语法分析  │    │ • 类型检查  │    │ • JIT 编译器            │ │
│  │ • 解析器   │    │ • IR 转换   │    │ • 本地代码生成          │ │
│  └─────────────┘    └─────────────┘    └─────────────────────────┘ │
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                      运行时系统                              │   │
│  ├─────────────────────────────────────────────────────────────┤   │
│  │ • 内存管理 • 垃圾回收 • 并发调度 • 标准库                   │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 二、技术选型

### 2.1 核心技术决策

**字节码 vs 直接解释**：采用字节码虚拟机方案。

| 字节码优势 | 说明 |
|------------|------|
| 指令紧凑 | 占用空间小 |
| 可缓存 | 避免重复解析 |
| JIT 友好 | 便于渐进式优化 |
| 跨平台 | 便于移植 |

**垃圾回收策略**：增量式分代 GC。

- 小对象栈分配
- 大对象堆分配
- 分代回收
- 增量标记、并发清除

**并发模型**：M:N 线程模型。

- 绿色线程映射到系统线程
- 工作窃取负载均衡
- 协程栈按需增长

---

## 三、模块设计

### 3.1 词法分析器

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    pub literal: Option<Literal>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // 关键字
    KwType, KwFn, KwPub, KwMod, KwUse,
    KwSpawn, KwRef, KwMut,
    KwIf, KwElif, KwElse, KwMatch,
    KwWhile, KwFor, KwReturn, KwBreak, KwContinue, KwAs,

    // 标识符和字面量
    Identifier(String),
    IntLiteral(i128),
    FloatLiteral(f64),
    BoolLiteral(bool),
    CharLiteral(char),
    StringLiteral(String),

    // 运算符和分隔符
    Plus, Minus, Star, Slash,
    Eq, Neq, Lt, Le, Gt, Ge,
    And, Or, Not,
    LParen, RParen, LBracket, RBracket, LBrace, RBrace,
    Comma, Colon, Semicolon, Pipe,
    Arrow, FatArrow,

    // 特殊
    Eof,
    Error(String),
}
```

### 3.2 语法分析器

采用 LL(1) 递归下降解析器结合 Pratt Parser 处理表达式：

```rust
#[derive(Debug, Clone)]
pub enum Expr {
    Lit(Literal),
    Var(String),
    BinOp(BinOp, Box<Expr>, Box<Expr>),
    UnOp(UnOp, Box<Expr>),
    FnCall { func: Box<Expr>, args: Vec<Expr> },
    FnDef {
        name: String,
        params: Vec<Param>,
        return_type: Option<Type>,
        body: Box<Block>,
        is_async: bool,
    },
    If {
        condition: Box<Expr>,
        then_branch: Box<Block>,
        elif_branches: Vec<(Box<Expr>, Box<Block>)>,
        else_branch: Option<Box<Block>>,
    },
    Match {
        expr: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    For {
        var: String,
        iterable: Box<Expr>,
        body: Box<Block>,
    },
    While {
        condition: Box<Expr>,
        body: Box<Block>,
    },
    Block(Vec<Stmt>),
    Return(Option<Box<Expr>>),
    // ...
}
```

### 3.3 类型检查器

核心算法采用 Hindley-Milner 类型推断的扩展版本：

```rust
#[derive(Debug, Clone)]
pub struct TypeEnvironment {
    variables: HashMap<String, Type>,
    constraints: Vec<TypeConstraint>,
    generics: HashSet<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Void, Bool,
    Int(usize), Float(usize),
    Char, String, Bytes,
    Struct(StructType),
    Union(UnionType),
    Enum(Vec<String>),
    Tuple(Vec<Type>),
    List(Box<Type>),
    Dict(Box<Type>, Box<Type>),
    Fn {
        params: Vec<Type>,
        return_type: Box<Type>,
        is_async: bool,
    },
    TypeVar(usize),
    TypeRef(String),
}
```

### 3.4 中间表示（IR）

```rust
#[derive(Debug)]
pub enum Value {
    Const(ConstValue),
    Local(usize),        // 局部变量索引
    Arg(usize),          // 参数索引
    Temp(usize),         // 临时变量
    StaticAddr(usize),   // 静态数据地址
}

#[derive(Debug)]
pub enum Instruction {
    // 移动指令
    Move { dst: Value, src: Value },

    // 算术运算
    Add { dst: Value, lhs: Value, rhs: Value },
    Sub { dst: Value, lhs: Value, rhs: Value },
    Mul { dst: Value, lhs: Value, rhs: Value },
    Div { dst: Value, lhs: Value, rhs: Value },

    // 比较跳转
    Cmp { dst: Value, lhs: Value, rhs: Value },
    Jmp(usize),
    JmpIf(Value, usize),
    JmpIfNot(Value, usize),

    // 函数调用
    Call { dst: Option<Value>, func: Value, args: Vec<Value> },
    CallAsync { dst: Value, func: Value, args: Vec<Value> },
    Ret(Option<Value>),

    // 内存操作
    Alloc { dst: Value, size: Value },
    Free(Value),
    LoadField { dst: Value, src: Value, field: usize },
    StoreField { dst: Value, field: usize, src: Value },

    // 并发操作
    Spawn { func: Value },
    Await(Value),
    Yield,
}
```

### 3.5 字节码虚拟机

```rust
pub struct VM {
    // 寄存器
    regs: Vec<Value>,
    ip: usize,
    sp: usize,
    fp: usize,

    // 运行时
    stack: Vec<Value>,
    frames: Vec<Frame>,
    constants: Vec<ConstValue>,
    globals: Vec<Value>,

    // 内存管理
    heap: Heap,
    gc: GC,

    // 并发
    scheduler: Scheduler,
}
```

### 3.6 垃圾回收器

采用增量式分代收集器：

```rust
pub struct GC {
    heaps: Vec<HeapSpace>,     // 分代堆空间
    large_objects: Heap,       // 大对象堆
    global_root: Vec<GCRoot>,  // 全局根集合
    threads: Vec<GCRoot>,      // 线程根集合
    state: GCState,
    collector: Collector,
}

pub struct HeapSpace {
    young: Heap,               // 年轻代
    old: Heap,                 // 老年代
    allocation_buffer: Vec<u8>,
    card_table: Vec<u8>,
}
```

### 3.7 并发调度器

采用 M:N 线程模型，工作窃取负载均衡：

```rust
pub struct Scheduler {
    runqueues: Vec<Arc<RunQueue>>,
    global_queue: Arc<GlobalQueue>,
    workers: Vec<Worker>,
    task_counter: AtomicUsize,
}

pub struct Task {
    id: TaskId,
    state: AtomicTaskState,
    stack: TaskStack,
    context: Context,
    future: Option<BoxFuture>,
    spawned_at: Instant,
}
```

---

## 四、性能优化策略

### 4.1 解释器优化

| 优化技术 | 说明 |
|----------|------|
| 热点检测 | 采样分析识别热点函数 |
| 类型特化 | 针对常见类型生成特化字节码 |
| 内联缓存 | 缓存已知类型的实现 |
| 字节码缓存 | 避免重复解析相同代码 |

### 4.2 内存优化

| 优化技术 | 说明 |
|----------|------|
| 栈分配优先 | 小对象分配在栈上 |
| 小对象优化 | bump allocator 快速分配 |
| 内存布局优化 | 结构体字段重排减少填充 |

### 4.3 缓存优化

| 优化技术 | 说明 |
|----------|------|
| 指令缓存优化 | 线性代码布局 |
| 数据缓存优化 | SoA 布局提高向量化 |

---

## 五、JIT 编译器

### 5.1 分层编译

| 层级 | 说明 |
|------|------|
| 解释器 | 立即开始执行，收集类型信息 |
| 基线编译 | 快速生成机器码 |
| 优化编译 | 基于 profiling 优化 |

### 5.2 代码生成

```rust
pub trait CodeGenerator {
    fn emit_prologue(&mut self, frame: &Frame);
    fn emit_epilogue(&mut self, frame: &Frame);
    fn emit_add(&mut self, dst: Reg, lhs: Reg, rhs: Reg);
    fn emit_call(&mut self, func: Reg, args: Vec<Reg>, dst: Reg);
}
```

---

## 六、路线图

### 6.1 实现阶段

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              实现路线图                                      │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  v0.1: Rust 解释器 ────────→ v0.5: Rust JIT 编译器 ────────→ v1.0: Rust AOT │
│        (当前阶段)                        │                      编译器      │
│                                           │                                   │
│                                           ▼                                   │
│  v0.6: YaoXiang 解释器 ←─────── v1.0: YaoXiang JIT 编译器 ←──── v2.0:       │
│        （自举）                     （自举）                      YaoXiang AOT │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 6.2 里程碑

| 里程碑 | 时间 | 交付物 |
|--------|------|--------|
| M1: 解释器原型 | 第 1-2 个月 | 基本解释器 |
| M2: 完整解释器 | 第 3-4 个月 | 功能完整的解释器 |
| M3: JIT 编译器 | 第 5-7 个月 | 支持 JIT 的运行时 |
| M4: AOT 编译器 | 第 8-10 个月 | 原生编译器 |
| M5: 自举 | 第 11-14 个月 | 自举编译器 |

---

## 七、测试策略

### 7.1 测试层次

| 层级 | 说明 |
|------|------|
| 单元测试 | 测试各个模块的独立功能 |
| 集成测试 | 测试模块间的协作 |
| 端到端测试 | 测试完整程序的执行 |
| 模糊测试 | 随机测试发现边界问题 |

### 7.2 性能测试

| 测试类型 | 说明 |
|----------|------|
| 微基准测试 | 单个操作的性能 |
| 宏基准测试 | 完整程序的性能 |
| 并发测试 | 并发性能测试 |

---

## 八、风险与应对

| 风险 | 可能性 | 影响 | 应对措施 |
|------|--------|------|----------|
| 性能不达预期 | 中 | 高 | 渐进优化、JIT 升级 |
| 类型系统复杂度过高 | 中 | 中 | 简化实现、迭代完善 |
| JIT 实现困难 | 中 | 高 | 先完成解释器 |
| 自举困难 | 低 | 高 | Rust 实现兜底 |

---

## 附录A：指令集

### A.1 指令编码

| 类别 | 指令前缀 | 数量 |
|------|----------|------|
| 栈操作 | 0x00-0x0F | 16 |
| 加载存储 | 0x10-0x2F | 32 |
| 算术运算 | 0x30-0x5F | 48 |
| 比较跳转 | 0x60-0x7F | 32 |
| 函数调用 | 0x80-0x8F | 16 |
| 内存分配 | 0x90-0x9F | 16 |
| 类型操作 | 0xA0-0xAF | 16 |
| 并发操作 | 0xB0-0xBF | 16 |

### A.2 核心指令

| 指令 | 操作数 | 说明 |
|------|--------|------|
| NOP | - | 空操作 |
| PUSH | const | 将常量压栈 |
| POP | reg | 弹栈到寄存器 |
| DUP | - | 复制栈顶 |
| ADD | - | 加法 |
| SUB | - | 减法 |
| MUL | - | 乘法 |
| DIV | - | 除法 |
| CALL | func | 函数调用 |
| RET | - | 返回 |
| SPAWN | func | 创建异步任务 |
| AWAIT | - | 等待异步任务 |

---

> 「道生之，德畜之，物形之，势成之。」
>
> 编程语言之道，在于设计之完善、实现之精进、性能之优化。
