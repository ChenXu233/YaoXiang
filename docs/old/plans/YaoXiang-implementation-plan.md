# YaoXiang（爻象）实现计划

> 版本：v2.0.0
> 状态：实现规划
> 作者：晨煦
> 日期：2025-01-02

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
    // 关键字（16个核心关键字，不可覆盖）
    KwType, KwPub, KwUse,
    KwSpawn, KwRef, KwMut,
    KwIf, KwElif, KwElse, KwMatch,
    KwWhile, KwFor, KwIn, KwReturn, KwBreak, KwContinue, KwAs,

    // 标识符和字面量
    Identifier(String),
    Underscore,
    IntLiteral(i128),
    FloatLiteral(f64),
    BoolLiteral(bool),
    CharLiteral(char),
    StringLiteral(String),

    // 运算符和分隔符
    Plus, Minus, Star, Slash, Percent,
    Eq, Neq, Lt, Le, Gt, Ge,
    And, Or, Not,
    ColonColon, DotDotDot,
    LParen, RParen, LBracket, RBracket, LBrace, RBrace,
    Comma, Colon, Semicolon, Pipe,
    Dot, Arrow, FatArrow,

    // 特殊
    Eof,
    Error(String),
}
```

> **📝 设计说明**：
> - **模块设计**：采用多文件模块系统，通过 `use` 导入文件。每个 `.yx` 文件就是一个模块
> - **函数定义语法**：移除 `fn` 关键字，采用简洁的赋值语法
>   ```yaoxiang
>   add(Int, Int) -> Int = (a, b) => a + b
>   fact(Int) -> Int = (n) => {
>       if n == 0 { 1 } else { n * fact(n - 1) }
>   }
>   identity<T>(T) -> T = (x) => x
>   ```
> - **类型关键字**（可覆盖）：`void`, `bool`, `char`, `string`, `bytes`, `int`, `float` - 当前作为普通 `Identifier` 处理，解析器在类型上下文中识别
> - **布尔字面量**：`true`, `false` - 当前作为普通 `Identifier` 处理，解析器在表达式上下文中识别为 `BoolLiteral`
> - **`in` 关键字**：支持 Python 风格的列表推导式语法糖
>   ```yaoxiang
>   [x * 2 for x in range(10) if x % 2 == 0]  // 生成 [0, 4, 8, 12, 16]
>   ```

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

## 七、项目结构

### 7.1 代码组织

```
yaoxiang/
├── Cargo.toml                    # 根配置文件
├── src/
│   ├── main.rs                   # CLI 入口
│   ├── lib.rs                    # 库入口
│   │
│   ├── frontend/                 # 前端模块
│   │   ├── mod.rs
│   │   ├── lexer/                # 词法分析器
│   │   │   ├── mod.rs
│   │   │   ├── tokens.rs
│   │   │   └── tokenizer.rs
│   │   ├── parser/               # 语法分析器
│   │   │   ├── mod.rs
│   │   │   ├── ast.rs
│   │   │   ├── parser.rs
│   │   │   └── pratt.rs
│   │   └── typecheck/            # 类型检查器
│   │       ├── mod.rs
│   │       ├── infer.rs
│   │       └── check.rs
│   │
│   ├── middle/                   # 中间层模块
│   │   ├── mod.rs
│   │   ├── ir.rs                 # 中间表示
│   │   ├── optimizer/            # 优化器
│   │   │   ├── mod.rs
│   │   │   ├── inliner.rs
│   │   │   └── dce.rs
│   │   └── codegen/              # 代码生成
│   │       ├── mod.rs
│   │       └── bytecode.rs
│   │
│   ├── backends/                # 后端模块
│   │   └── dev/
│   │       └── tui_repl/        # TUI REPL 开发工具
│   │
│   ├── std/                      # 标准库
│   │   └── mod.rs
│   │
│   └── util/                     # 工具模块
│       ├── mod.rs
│       ├── span.rs               # 源代码位置
│       ├── diagnostic.rs         # 错误诊断
│       └── cache.rs              # 缓存管理
│
├── tests/                        # 测试套件
│   ├── unit/
│   ├── integration/
│   └── e2e/
│
├── examples/                     # 示例程序
│   ├── hello.yx
│   ├── fib.yx
│   └── async_example.yx
│
├── docs/                         # 文档
│   ├── YaoXiang-concept-validation.md
│   ├── YaoXiang-language-specification.md
│   └── YaoXiang-implementation-plan.md
│
└── benchmarks/                   # 性能基准
    ├── basic.rs
    ├── loops.rs
    └── concurrent.rs
```

### 7.2 关键文件说明

**Cargo.toml 配置项目元数据和依赖**：

```toml
[package]
name = "yaoxiang"
version = "0.1.0"
edition = "2024"
authors = ["YaoXiang Team"]

[lib]
path = "src/lib.rs"

[[bin]]
name = "yaoxiang"
path = "src/main.rs"

[features]
debug = []
jit = ["cranelift", "dynasm"]
wasm = ["wasm-bindgen"]

[dependencies]
logos = "0.14"
parking_lot = "0.12"
crossbeam = "0.8"
rayon = "1.9"
# ... 更多依赖
```

---

## 八、实现阶段详细规划

### 8.1 阶段划分总览

```
┌──────────────────────────────────────────────────────────────────────────────────────┐
│                              YaoXiang 实现阶段划分                                   │
├──────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                      │
│  阶段一          阶段二          阶段三          阶段四          阶段五              │
│  ────────       ────────       ────────       ────────       ────────              │
│  项目初始化     词法分析器      语法分析器      类型检查器      字节码生成           │
│  第1周          第2周           第3-4周         第5-6周         第6-7周              │
│                                                                                      │
│  阶段六          阶段七          阶段八                                             │
│  ────────       ────────       ────────                                           │
│  字节码虚拟机    运行时系统      测试与优化                                           │
│  第8-10周       第10-12周       第13-14周                                          │
│                                                                                      │
└──────────────────────────────────────────────────────────────────────────────────────┘
```

### 8.2 阶段一：项目初始化（第 1 周）

**目标**：搭建项目基础架构

| 任务 | 详细说明 | 输出文件 | 验收标准 |
|------|----------|----------|----------|
| 创建 Cargo 项目 | 初始化 workspace，配置依赖 | `Cargo.toml` | `cargo build` 成功 |
| 创建目录结构 | 按模块创建 src 子目录 | `src/frontend/`, `src/middle/`, `src/middle/`, `src/middle/`, `src/std/` | 目录结构符合设计 |
| 配置依赖 | 添加必要依赖（logos 可选、parking_lot、crossbeam、rayon） | `Cargo.toml` | 依赖解析成功 |
| 配置 CI/CD | GitHub Actions 自动构建测试 | `.github/workflows/ci.yml` | CI 通过 |
| 配置代码风格 | rustfmt.toml、.clippy.toml | 配置文件 | `cargo fmt` + `cargo clippy` 通过 |

**详细任务分解**：

```bash
# 1. 创建项目结构
mkdir -p yaoxiang/src/{frontend/{lexer,parser,typecheck},middle/{ir,optimizer,codegen},vm,runtime/{gc,scheduler,memory},std,util}
mkdir -p yaoxiang/{tests,examples,benchmarks,.github/workflows}

# 2. 初始化 Cargo
cd yaoxiang
cargo init --name yaoxiang
```

### 8.3 阶段二：词法分析器（第 2 周）

**目标**：完成词法分析功能

| 任务 | 详细说明 | 输出文件 | 验收标准 |
|------|----------|----------|----------|
| 定义 Token 类型 | 16 个关键字、字面量、运算符、分隔符 | `src/frontend/lexer/tokens.rs` | TokenKind 枚举完整 |
| 实现 Tokenizer | 状态机驱动的词法分析 | `src/frontend/lexer/mod.rs` | 通过所有词法测试 |
| Unicode 支持 | UTF-8 直接处理，支持 Unicode 标识符和字符串 | 同上 | 支持中文字符等 |
| 位置追踪 | Spanned trait、Span 类型 | `src/util/span.rs` | 错误定位准确 |
| 单元测试 | 各类型 token 测试、边界测试 | `tests/unit/lexer.rs` | 100% 分支覆盖 |

**词法分析器的优化策略**：零拷贝词法分析（避免字符串复制）、查表法状态机（O(1) 状态转移）、Unicode 支持（UTF-8 直接处理）、增量扫描（大文件分块处理）。

**核心数据结构**：

```rust
// src/frontend/lexer/tokens.rs
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    pub literal: Option<Literal>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // 关键字（16个核心关键字）
    KwType, KwPub, KwUse,
    KwSpawn, KwRef, KwMut,
    KwIf, KwElif, KwElse, KwMatch,
    KwWhile, KwFor, KwIn, KwReturn, KwBreak, KwContinue, KwAs,

    // 标识符和字面量
    Identifier(String),
    Underscore,
    IntLiteral(i128),
    FloatLiteral(f64),
    BoolLiteral(bool),
    CharLiteral(char),
    StringLiteral(String),

    // 运算符和分隔符
    Plus, Minus, Star, Slash, Percent,
    Eq, Neq, Lt, Le, Gt, Ge,
    And, Or, Not,
    ColonColon, DotDotDot,
    LParen, RParen, LBracket, RBracket, LBrace, RBrace,
    Comma, Colon, Semicolon, Pipe,
    Dot, Arrow, FatArrow,

    // 特殊
    Eof,
    Error(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i128),
    Float(f64),
    Bool(bool),
    Char(char),
    String(String),
}
```

**测试用例**：

```rust
// tests/unit/lexer.rs
#[test]
fn test_keywords() {
    let tokens = tokenize("type spawn ref mut if else");
    assert!(matches!(tokens[0].kind, TokenKind::KwType));
    assert!(matches!(tokens[1].kind, TokenKind::KwSpawn));
}

#[test]
fn test_string_literals() {
    let tokens = tokenize(r#""hello world" "#);
    assert!(matches!(
        tokens[0].kind,
        TokenKind::StringLiteral(s) if s == "hello world"
    ));
}

#[test]
fn test_unicode_identifiers() {
    let tokens = tokenize("姓名 年龄 地址");
    assert!(matches!(tokens[0].kind, TokenKind::Identifier(s) if s == "姓名"));
}
```

### 8.4 阶段三：语法分析器（第 3-4 周）

**目标**：完成语法分析和 AST 生成

| 任务 | 详细说明 | 输出文件 | 验收标准 |
|------|----------|----------|----------|
| 定义 AST 节点 | Expression、Statement、Type、Module | `src/frontend/parser/ast.rs` | AST 节点完整 |
| 递归下降解析 | 解析函数定义、类型定义、控制流 | `src/frontend/parser/mod.rs` | 通过所有语法测试 |
| Pratt Parser | 处理运算符优先级和结合性 | `src/frontend/parser/mod.rs` | 表达式解析正确 |
| 错误恢复 | 错误位置记录、恢复解析 | 同上 | 友好的错误信息 |
| 单元测试 | 各语法结构测试、错误测试 | `tests/unit/parser.rs` | 95%+ 覆盖 |

**语法分析器的优化策略**：零成本抽象（使用枚举而非 trait 对象）、避免重复解析（缓存解析结果）、增量解析（修改局部重解析）、错误恢复（容错解析）。

**核心数据结构**：

```rust
// src/frontend/parser/ast.rs

// 表达式
#[derive(Debug, Clone)]
pub enum Expr {
    // 字面量
    Lit(Literal),

    // 标识符
    Var { name: String, span: Span },

    // 二元运算
    BinOp {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },

    // 一元运算
    UnOp {
        op: UnOp,
        expr: Box<Expr>,
        span: Span,
    },

    // 函数调用
    Call {
        func: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },

    // 函数定义
    FnDef {
        name: String,
        params: Vec<Param>,
        return_type: Option<Type>,
        body: Box<Block>,
        is_async: bool,
        span: Span,
    },

    // 条件表达式
    If {
        condition: Box<Expr>,
        then_branch: Box<Block>,
        elif_branches: Vec<(Box<Expr>, Box<Block>)>,
        else_branch: Option<Box<Block>>,
        span: Span,
    },

    // 模式匹配
    Match {
        expr: Box<Expr>,
        arms: Vec<MatchArm>,
        span: Span,
    },

    // 循环
    While {
        condition: Box<Expr>,
        body: Box<Block>,
        label: Option<String>,
        span: Span,
    },

    For {
        var: String,
        iterable: Box<Expr>,
        body: Box<Block>,
        label: Option<String>,
        span: Span,
    },

    // 代码块
    Block(Block),

    // 返回
    Return(Option<Box<Expr>>),

    // 中断
    Break(Option<String>),

    // 继续
    Continue(Option<String>),

    // 类型转换
    Cast {
        expr: Box<Expr>,
        target_type: Type,
        span: Span,
    },

    // 元组
    Tuple(Vec<Expr>, Span),

    // 列表
    List(Vec<Expr>, Span),

    // 字典
    Dict(Vec<(Expr, Expr)>, Span),
}

// 语句
#[derive(Debug, Clone)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum StmtKind {
    Expr(Box<Expr>),
    Let {
        name: String,
        type_annotation: Option<Type>,
        initializer: Option<Box<Expr>>,
        is_mut: bool,
    },
    TypeDef {
        name: String,
        definition: Type,
    },
    Module {
        name: String,
        items: Vec<Stmt>,
    },
    Use {
        path: String,
        items: Option<Vec<String>>,
        alias: Option<String>,
    },
}

// 代码块
#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub expr: Option<Box<Expr>>,  // 块表达式的值
    pub span: Span,
}

// 函数参数
#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: Option<Type>,
    pub span: Span,
}

// Match 臂
#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Expr,
    pub span: Span,
}

// 模式
#[derive(Debug, Clone)]
pub enum Pattern {
    Wildcard,
    Identifier(String),
    Literal(Literal),
    Tuple(Vec<Pattern>),
    Struct {
        name: String,
        fields: Vec<(String, Pattern)>,
    },
    Union {
        name: String,
        variant: String,
        pattern: Option<Box<Pattern>>,
    },
    Or(Vec<Pattern>),
    Guard {
        pattern: Box<Pattern>,
        condition: Expr,
    },
}
```

**Pratt Parser 实现要点**：

```rust
// src/frontend/parser/pratt.rs

pub struct PrattParser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> PrattParser<'a> {
    pub fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, pos: 0 }
    }

    // 运算符优先级定义
    fn prefix_binding_power(op: &TokenKind) -> Option<((), ())> {
        match op {
            TokenKind::Minus | TokenKind::Not => Some(((), (7,))),
            _ => None,
        }
    }

    fn infix_binding_power(op: &TokenKind) -> Option<((u8, u8), (u8, u8))> {
        match op {
            TokenKind::As => Some(((1,), (2,))),
            TokenKind::Star | TokenKind::Slash | TokenKind::Percent => Some(((5,), (6,))),
            TokenKind::Plus | TokenKind::Minus => Some(((3,), (4,))),
            TokenKind::Lt | TokenKind::Le | TokenKind::Gt | TokenKind::Ge => Some(((1,), (2,))),
            TokenKind::Eq | TokenKind::Neq => Some(((1,), (2,))),
            TokenKind::And => Some(((1,), (2,))),
            TokenKind::Or => Some(((1,), (2,))),
            _ => None,
        }
    }

    pub fn parse_expression(&mut self, min_bp: u8) -> Option<Expr> {
        // 前缀解析
        let mut lhs = self.parse_prefix()?;

        // 中缀解析循环
        while let Some((l_bp, r_bp)) = self.infix_binding_power(&self.current().kind) {
            if l_bp.0 < min_bp {
                break;
            }

            let op = self.consume();
            let rhs = self.parse_expression(r_bp.0);

            lhs = Expr::BinOp {
                op: op.try_into()?,
                left: Box::new(lhs),
                right: Box::new(rhs.unwrap()),
                span: self.span_from(lhs.span, rhs.span),
            };
        }

        Some(lhs)
    }
}
```

### 8.5 阶段四：类型检查器（第 5-6 周）

**目标**：完成类型推断和类型检查

| 任务 | 详细说明 | 输出文件 | 验收标准 |
|------|----------|----------|----------|
| 定义类型系统 | Type 枚举、类型环境、约束 | `src/frontend/typecheck/types.rs` | 类型表示完整 |
| 类型推断 | Hindley-Milner 算法扩展 | `src/frontend/typecheck/infer.rs` | 自动推断正确 |
| 类型检查 | 约束收集和求解 | `src/frontend/typecheck/check.rs` | 类型检查正确 |
| 泛型支持 | 类型参数实例化 | 同上 | 泛型函数和类型工作 |
| 单元测试 | 类型推断测试、错误测试 | `tests/unit/typecheck.rs` | 90%+ 覆盖 |

**核心数据结构**：

```rust
// src/frontend/typecheck/types.rs

// 类型表示
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    // 原类型
    Void,
    Bool,
    Int(usize),      // 位宽: 8, 16, 32, 64, 128
    Float(usize),    // 位宽: 32, 64
    Char,
    String,
    Bytes,

    // 复合类型
    Struct {
        name: String,
        fields: Vec<(String, Type)>,
    },
    Enum {
        name: String,
        variants: Vec<String>,
    },
    Union {
        name: String,
        variants: Vec<(String, Option<Type>)>,
    },
    Tuple(Vec<Type>),
    List(Box<Type>),
    Dict(Box<Type>, Box<Type>),
    Set(Box<Type>),

    // 函数类型
    Fn {
        params: Vec<Type>,
        return_type: Box<Type>,
        is_async: bool,
    },

    // 泛型
    Generic {
        name: String,
        params: Vec<Type>,
    },

    // 类型变量（用于推断）
    TypeVar(usize),

    // 类型引用
    TypeRef(String),
}

// 类型环境
#[derive(Debug, Default)]
pub struct TypeEnvironment {
    // 变量类型绑定
    vars: HashMap<String, Type>,
    // 类型变量（用于推断）
    type_vars: Vec<Type>,
    // 泛型参数
    generics: HashSet<String>,
    // 当前作用域级别
    scope_level: usize,
    // 作用域链
    scopes: Vec<HashMap<String, Type>>,
}

// 约束
#[derive(Debug, Clone)]
pub struct TypeConstraint {
    pub left: Type,
    pub right: Type,
    pub span: Span,
}

// 泛型约束
#[derive(Debug, Clone)]
pub enum TypeBound {
    Eq(Type),        // 等于
    Sub(Type),       // 子类型
}
```

**类型推断算法**：

```rust
// src/frontend/typecheck/infer.rs

impl TypeChecker {
    // 表达式类型推断
    pub fn infer_expr(&mut self, expr: &Expr) -> Result<Type, TypeError> {
        match expr {
            Expr::Lit(lit) => self.infer_literal(lit),
            Expr::Var { name, .. } => self.infer_variable(name),
            Expr::BinOp { op, left, right, .. } => {
                let left_ty = self.infer_expr(left)?;
                let right_ty = self.infer_expr(right)?;
                self.infer_binop(op, &left_ty, &right_ty)
            }
            Expr::Call { func, args, .. } => {
                let func_ty = self.infer_expr(func)?;
                let arg_tys: Vec<Type> = args.iter()
                    .map(|a| self.infer_expr(a))
                    .collect::<Result<_, _>>()?;
                self.infer_call(&func_ty, &arg_tys)
            }
            Expr::FnDef { params, return_type, body, is_async, .. } => {
                self.infer_fn_def(params, return_type.as_ref(), body, *is_async)
            }
            Expr::If { condition, then_branch, elif_branches, else_branch, .. } => {
                self.infer_if(condition, then_branch, elif_branches, else_branch)
            }
            Expr::Match { expr, arms, .. } => {
                self.infer_match(expr, arms)
            }
            // ... 其他表达式
        }
    }

    // 合一算法
    fn unify(&mut self, t1: &Type, t2: &Type) -> Result<(), TypeError> {
        // 类型变量处理
        if let Type::TypeVar(id1) = t1 {
            if let Type::TypeVar(id2) = t2 {
                if id1 != id2 {
                    self.type_vars[*id2] = t1.clone();
                }
                return Ok(());
            }
            return self.bind_type_var(*id1, t2);
        }
        if let Type::TypeVar(id) = t2 {
            return self.bind_type_var(id, t1);
        }

        // 递归合一
        match (t1, t2) {
            (Type::Int(w1), Type::Int(w2)) if w1 == w2 => Ok(()),
            (Type::Float(w1), Type::Float(w2)) if w1 == w2 => Ok(()),
            (Type::List(t1), Type::List(t2)) => self.unify(t1, t2),
            (Type::Tuple(ts1), Type::Tuple(ts2)) if ts1.len() == ts2.len() => {
                ts1.iter().zip(ts2.iter())
                    .try_for_each(|(a, b)| self.unify(a, b))
            }
            (Type::Fn { params: p1, return_type: r1, .. },
             Type::Fn { params: p2, return_type: r2, .. }) => {
                if p1.len() != p2.len() {
                    return Err(TypeError::ArityMismatch);
                }
                p1.iter().zip(p2.iter())
                    .try_for_each(|(a, b)| self.unify(a, b))?;
                self.unify(r1, r2)
            }
            // ... 其他情况
            _ => Err(TypeError::Mismatch(t1.clone(), t2.clone()))
        }
    }

    fn bind_type_var(&mut self, var_id: usize, ty: &Type) -> Result<(), TypeError> {
        if let Type::TypeVar(id) = ty {
            if id != var_id {
                self.type_vars[var_id] = self.type_vars[id].clone();
            }
            return Ok(());
        }

        // 检查循环引用
        if self.occurs_check(var_id, ty) {
            return Err(TypeError::RecursiveType);
        }

        self.type_vars[var_id] = ty.clone();
        Ok(())
    }

    fn occurs_check(&self, var_id: usize, ty: &Type) -> bool {
        match ty {
            Type::TypeVar(id) => *id == var_id,
            Type::List(t) => self.occurs_check(var_id, t),
            Type::Tuple(ts) => ts.iter().any(|t| self.occurs_check(var_id, t)),
            Type::Fn { params, return_type, .. } => {
                params.iter().any(|t| self.occurs_check(var_id, t)) ||
                self.occurs_check(var_id, return_type)
            }
            _ => false,
        }
    }
}
```

### 8.6 阶段五：字节码生成（第 6-7 周）

**目标**：完成 AST 到字节码的转换

| 任务 | 详细说明 | 输出文件 | 验收标准 |
|------|----------|----------|----------|
| 定义 IR | 中间表示指令集 | `src/middle/ir.rs` | IR 指令集完整 |
| 代码生成器 | AST → IR → 字节码 | `src/middle/codegen/mod.rs` | 正确生成字节码 |
| 字节码序列化 | 编码/解码、验证 | `src/middle/codegen/bytecode.rs` | 可反序列化 |
| 基础优化 | 常量折叠、死代码消除 | `src/middle/optimizer/mod.rs` | 优化生效 |

**核心数据结构**：

```rust
// src/middle/ir.rs

// 指令操作数
#[derive(Debug, Clone)]
pub enum Operand {
    Const(ConstValue),   // 常量
    Local(usize),        // 局部变量索引
    Arg(usize),          // 参数索引
    Temp(usize),         // 临时变量
    Global(usize),       // 全局变量
    Label(usize),        // 标签
}

// 指令
#[derive(Debug, Clone)]
pub enum Instruction {
    // 移动
    Move { dst: Operand, src: Operand },

    // 加载存储
    Load { dst: Operand, src: Operand },
    Store { dst: Operand, src: Operand },
    Push(Operand),
    Pop(Operand),

    // 栈操作
    Dup,
    Swap,
    Rot2,   // 交换栈顶3个值

    // 算术运算
    Add { dst: Operand, lhs: Operand, rhs: Operand },
    Sub { dst: Operand, lhs: Operand, rhs: Operand },
    Mul { dst: Operand, lhs: Operand, rhs: Operand },
    Div { dst: Operand, lhs: Operand, rhs: Operand },
    Mod { dst: Operand, lhs: Operand, rhs: Operand },
    Neg { dst: Operand, src: Operand },

    // 比较
    Cmp { dst: Operand, lhs: Operand, rhs: Operand },

    // 跳转
    Jmp(usize),                   // 无条件跳转
    JmpIf(Operand, usize),        // 条件跳转
    JmpIfNot(Operand, usize),

    // 函数调用
    Call { dst: Option<Operand>, func: Operand, args: Vec<Operand> },
    CallAsync { dst: Operand, func: Operand, args: Vec<Operand> },
    TailCall { func: Operand, args: Vec<Operand> },
    Ret(Option<Operand>),

    // 内存
    Alloc { dst: Operand, size: Operand },
    Free(Operand),
    AllocArray { dst: Operand, size: Operand, elem_size: Operand },
    LoadField { dst: Operand, src: Operand, field: usize },
    StoreField { dst: Operand, field: usize, src: Operand },
    LoadIndex { dst: Operand, src: Operand, index: Operand },
    StoreIndex { dst: Operand, index: Operand, src: Operand },

    // 类型
    Cast { dst: Operand, src: Operand, target_type: Type },
    TypeTest(Operand, Type),

    // 并发
    Spawn { func: Operand },
    Await(Operand),
    Yield,
}

// 基本块
#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub label: usize,
    pub instructions: Vec<Instruction>,
    pub successors: Vec<usize>,  // 后继块
}

// 函数 IR
#[derive(Debug, Clone)]
pub struct FunctionIR {
    pub name: String,
    pub params: Vec<Type>,
    pub return_type: Type,
    pub is_async: bool,
    pub locals: Vec<Type>,           // 局部变量类型
    pub blocks: Vec<BasicBlock>,
    pub entry: usize,
}

// 模块 IR
#[derive(Debug, Clone)]
pub struct ModuleIR {
    pub types: Vec<Type>,             // 类型表
    pub constants: Vec<ConstValue>,   // 常量池
    pub globals: Vec<(String, Type, Option<ConstValue>)>,  // 全局变量
    pub functions: Vec<FunctionIR>,
}
```

### 8.7 阶段六：字节码虚拟机（第 8-10 周）

**目标**：完成字节码解释器

| 任务 | 详细说明 | 输出文件 | 验收标准 |
|------|----------|----------|----------|
| 虚拟机核心 | 解释器循环、寄存器管理 | `src/middle/executor.rs` | 解释器运行正确 |
| 指令实现 | 60+ 指令实现 | `src/middle/instructions.rs` | 所有指令工作 |
| 调用栈 | Frame 管理 | `src/middle/frames.rs` | 函数调用正确 |
| 错误处理 | 错误类型、错误传播 | `src/middle/errors.rs` | 错误信息友好 |

**核心数据结构**：

```rust
// src/middle/executor.rs

pub struct VM {
    // 寄存器
    regs: Vec<Value>,

    // 栈
    stack: Vec<Value>,
    sp: usize,      // 栈指针
    fp: usize,      // 帧指针

    // 调用帧
    frames: Vec<Frame>,

    // 常量池
    constants: Vec<ConstValue>,

    // 全局变量
    globals: Vec<Value>,

    // 字节码
    code: Vec<Opcode>,
    ip: usize,

    // 运行时
    heap: Heap,
    gc: GC,
    scheduler: Scheduler,

    // 状态
    status: VMStatus,
    error: Option<VMError>,
}

pub struct Frame {
    pub function: Function,
    pub ip: usize,
    pub fp: usize,
    pub locals: Vec<Value>,
}

// 运行时值
#[derive(Debug, Clone)]
pub enum Value {
    Void,
    Bool(bool),
    Int(i128),
    Float(f64),
    Char(char),
    String(Handle<StringValue>),
    Bytes(Handle<BytesValue>),
    List(Handle<ListValue>),
    Dict(Handle<DictValue>),
    Tuple(Vec<Value>),
    Fn {
        func: Handle<FunctionValue>,
        env: Vec<Value>,  // 闭包环境
    },
    Object(Handle<ObjectValue>),
    Type(Type),
    TypeVar(usize),
}

// 句柄（用于 GC）
pub struct Handle<T> {
    ptr: NonNull<T>,
    generation: u8,
}
```

**解释器循环优化**：采用直接线程化（Direct Threading）替代 switch-case，使用字节码缓存避免重复解析，内联缓存（Inline Caching）优化热点调用，特化解释器处理常见类型。

```rust
// 直接线程化解释器循环
macro_rules! dispatch {
    ($vm:ident) => {
        loop {
            let opcode = unsafe { *$vm.ip };
            $vm.ip += 1;
            match Opcode::from_u8(opcode) {
                Opcode::Add => { /* ... */ }
                Opcode::Sub => { /* ... */ }
                Opcode::Call => { /* ... */ }
                // ...
                Opcode::Invalid => return Err(VMError::InvalidOpcode),
            }
        }
    };
}
```

### 8.8 阶段七：运行时系统（第 10-12 周）

**目标**：完成内存管理和并发调度

| 任务 | 详细说明 | 输出文件 | 验收标准 |
|------|----------|----------|----------|
| 内存管理 | 堆分配器、内存池 | `src/middle/memory/mod.rs` | 分配正确 |
| 垃圾回收 | 分代 GC、增量收集 | `src/middle/gc/mod.rs` | GC 正常工作 |
| 并发调度 | M:N 线程、工作窃取 | `src/middle/scheduler/mod.rs` | 并发正确 |
| 标准库核心 | IO、List、String | `src/std/mod.rs` | 标准库可用 |

**垃圾回收器设计**：

```rust
// src/middle/gc/mod.rs

pub struct GC {
    // 堆空间
    young_space: HeapSpace,
    old_space: HeapSpace,
    large_object_space: HeapSpace,

    // 根集合
    global_roots: Vec<GCRoot>,
    thread_roots: Mutex<Vec<GCRoot>>,

    // 状态
    state: GCState,
    pause_time: Duration,

    // 配置
    config: GCConfig,
}

pub struct HeapSpace {
    start: *mut u8,
    end: *mut u8,
    cursor: *mut u8,
    bump_size: usize,
    objects: Vec<GcObjectHeader>,
}

#[derive(Debug, Clone)]
pub struct GcObjectHeader {
    pub size: usize,
    pub color: Color,
    pub mark_bits: usize,
    pub next: Option<NonNull<GcObjectHeader>>,
}

pub enum Color {
    White,
    Gray,
    Black,
}

impl GC {
    pub fn collect(&mut self) {
        match self.state {
            GCState::Idle => self.start_mark_and_sweep(),
            GCState::Marking => self.continue_marking(),
            GCState::Sweeping => self.continue_sweeping(),
            _ => {}
        }
    }

    fn start_mark_and_sweep(&mut self) {
        self.state = GCState::Marking;
        self.mark_roots();
        self.state = GCState::Sweeping;
        self.sweep();
        self.state = GCState::Idle;
    }

    fn mark_roots(&mut self) {
        let mut queue = VecDeque::new();

        // 标记全局根
        for root in &self.global_roots {
            if let Some(obj) = root.get() {
                self.mark_object(obj, &mut queue);
            }
        }

        // 标记线程根
        for root in &*self.thread_roots.lock().unwrap() {
            if let Some(obj) = root.get() {
                self.mark_object(obj, &mut queue);
            }
        }

        // 三色标记
        while let Some(obj) = queue.pop_front() {
            self.mark_children(obj, &mut queue);
        }
    }
}
```

**工作窃取算法**：

```rust
fn steal_work(worker_id: usize) -> Option<Task> {
    let mut rng = thread_rng();
    let num_workers = workers.len();

    // 随机选择 victim
    let mut attempts = 0;
    while attempts < num_workers {
        let victim_id = rng.gen_range(0..num_workers);
        if victim_id != worker_id {
            if let Some(task) = runqueues[victim_id].steal() {
                return Some(task);
            }
        }
        attempts += 1;
    }

    None
}
```

### 8.9 阶段八：测试与优化（第 13-14 周）

**目标**：确保质量和性能

| 任务 | 详细说明 | 输出文件 | 验收标准 |
|------|----------|----------|----------|
| 单元测试 | 各模块测试 | `tests/unit/*.rs` | 90%+ 覆盖 |
| 集成测试 | 端到端测试 | `tests/integration/*.rs` | 所有测试通过 |
| 性能基准 | benchmark suite | `benches/*.rs` | 性能数据可用 |
| Bug 修复 | 修复发现的问题 | - | 稳定版本 |

**测试用例**：

```rust
// tests/integration/hello_world.rs

#[test]
fn test_hello_world() {
    let code = r#"
        main() -> Void = () => {
            print("Hello, World!")
        }
    "#;
    let output = run_yx(code);
    assert_eq!(output.trim(), "Hello, World!");
}

// tests/integration/fibonacci.rs

#[test]
fn test_fibonacci() {
    let code = r#"
        fib(Int) -> Int = (n) => {
            if n < 2 { n } else { fib(n - 1) + fib(n - 2) }
        }
        main() -> Void = () => {
            print(fib(10))
        }
    "#;
    let output = run_yx(code);
    assert_eq!(output.trim(), "55");
}

// benchmarks/fibonacci.rs

#[bench]
fn bench_fibonacci(b: &mut Bencher) {
    b.iter(|| {
        let code = r#"
            fib(Int) -> Int = (n) => {
                if n < 2 { n } else { fib(n - 1) + fib(n - 2) }
            }
            fib(30)
        "#;
        run_yx(code);
    });
}
```

### 8.10 每周检查点

| 周次 | 主要交付物 | 风险点 | 应对措施 |
|------|-----------|--------|----------|
| 1 | 项目结构、CI | 依赖选择不当 | 提前验证依赖 |
| 2 | 词法分析器 | Unicode 复杂 | 参考现有实现 |
| 3-4 | 语法分析器 | 语法二义性 | 详细语法设计 |
| 5-6 | 类型检查器 | 推断算法复杂 | 分步实现、测试驱动 |
| 6-7 | 字节码生成 | IR 设计缺陷 | 参考现有 VM 设计 |
| 8-10 | 虚拟机 | 性能问题 | 渐进优化 |
| 10-12 | 运行时 | GC 正确性 | 充分测试 |
| 13-14 | 测试优化 | Bug 过多 | 预留缓冲时间 |

---

## 九、测试策略

### 9.1 测试层次

**单元测试**测试各个模块的独立功能，包括词法分析器、语法分析器、类型检查器、虚拟机指令、运行时组件。单元测试使用 Rust 的 `#[test]` 属性编写，位于源码同文件或 tests/unit 目录。

**集成测试**测试模块间的协作，包括前端到后端的完整流程、错误处理流程、边界条件处理。集成测试位于 tests/integration 目录。

**端到端测试**测试完整程序的执行，包括标准库功能、语言特性验证、性能基准测试。端到端测试位于 tests/e2e 目录。

**模糊测试**使用 libFuzzer 或 cargo-fuzz 进行随机测试，发现解析器崩溃、类型检查错误、虚拟机崩溃等问题。

### 9.2 测试覆盖

测试覆盖目标包括：关键路径 100% 覆盖、边界条件 95% 覆盖、分支覆盖 90% 以上、函数覆盖 95% 以上。使用 tarpaulin 或 cargo-kcov 生成覆盖率报告。

### 9.3 性能测试

**微基准测试**测量单个操作的性能，如指令执行时间、函数调用开销、内存分配开销。

**宏基准测试**测量完整程序的性能，如标准 benchmark suite、真实程序移植、常见算法实现。

**并发测试**测量并发性能，包括吞吐量测试、延迟测试、扩展性测试。

### 9.4 测试类型

| 层级 | 说明 |
|------|------|
| 单元测试 | 测试各个模块的独立功能 |
| 集成测试 | 测试模块间的协作 |
| 端到端测试 | 测试完整程序的执行 |
| 模糊测试 | 随机测试发现边界问题 |

| 测试类型 | 说明 |
|----------|------|
| 微基准测试 | 单个操作的性能 |
| 宏基准测试 | 完整程序的性能 |
| 并发测试 | 并发性能测试 |

---

## 十、风险与应对

### 10.1 技术风险

| 风险 | 可能性 | 影响 | 应对措施 |
|------|--------|------|----------|
| 性能不达预期 | 中 | 高 | 渐进优化、JIT 升级、原生编译 |
| 类型系统复杂度过高 | 中 | 中 | 简化实现、迭代完善 |
| JIT 实现困难 | 中 | 高 | 先完成解释器、JIT 作为可选特性 |
| 自举困难 | 低 | 高 | Rust 实现兜底、分阶段自举 |

### 10.2 项目风险

| 风险 | 可能性 | 影响 | 应对措施 |
|------|--------|------|----------|
| 开发进度延迟 | 中 | 中 | 敏捷开发、持续集成 |
| 核心人员流失 | 低 | 高 | 文档完善、知识共享 |
| 需求变更 | 中 | 中 | 需求评审、版本规划 |

### 10.3 进度规划

**里程碑与资源估算**：

**M1: 解释器原型（第 1-2 个月）**交付物为可运行的基本解释器，功能支持包括词法分析、语法分析、基础类型、基础控制流、标准库 IO。验收标准为运行 Hello World、简单计算器程序。

**M2: 完整解释器（第 3-4 个月）**交付物为功能完整的解释器，功能支持包括完整类型系统、模式匹配、模块系统、错误处理、垃圾回收。验收标准为通过标准测试套件、无内存泄漏。

**M3: JIT 编译器（第 5-7 个月）**交付物为支持 JIT 的运行时，性能提升 2-5 倍，功能支持包括基线编译、优化编译、profiling。验收标准为性能 benchmark 达标。

**M4: AOT 编译器（第 8-10 个月）**交付物为原生编译器，性能达到 Rust 的 50%，功能支持包括完整优化、静态链接、代码布局。验收标准为性能 benchmark 达标、自包含可执行文件。

**M5: 自举（第 11-14 个月）**交付物为自举编译器，YaoXiang 编写的新功能，验证自举正确性。验收标准为自举编译器可编译自身。

**人力估算**核心开发者 2-3 人，全职投入，预计 14 个月完成。部分贡献者参与测试、文档、周边工具开发。

**基础设施**开发机器（16GB RAM + SSD）、CI/CD 服务器（GitHub Actions 或自建）、性能测试服务器。

---

## 十一、开发环境配置

### 11.1 必需工具

- Rust 1.75+（2024 edition）
- cargo-edit, cargo-expand, cargo-outdated
- rustfmt, clippy
- Git, GitHub CLI

### 11.2 推荐工具

- IDE: VS Code + rust-analyzer / IntelliJ Rust
- Debugger: LLDB / WinDbg
- Profiler: perf / VTune
- Memory: valgrind / drmemory

### 11.3 开发命令

```bash
# 编译和运行
cargo build
cargo run --bin yaoxiang -- examples/hello.yx

# 测试
cargo test
cargo test --release
cargo nextest run  # 使用 nextest

# 代码质量
cargo fmt
cargo clippy
cargo clippy --fix

# 性能基准
cargo bench
cargo flamegraph

# 文档
cargo doc --open
cargo mdbook serve
```

---

## 十二、待实现特性提醒 ⚠️

> ⚠️ **重要提醒**：以下特性尚未实现，实现时请参考本提醒避免遗漏！

### 12.1 不可变性检查

**功能描述**：编译时检查，确保不可变变量（默认）不会被重新赋值。

```yaoxiang
// ✅ 正确 - 可变变量
mut x = 10
x = 20  // 允许

// ❌ 错误 - 不可变变量
x = 10
x = 20  // 应该报错：cannot assign to immutable variable
```

**实现位置**：`src/frontend/typecheck/`

**实现要求**：
1. 在 `TypeEnvironment` 中存储变量的可变性信息
2. 在赋值表达式检查时，验证目标变量是否可变
3. 错误类型建议：`ImmutableAssignError { name: String, span: Span }`

### 12.2 其他待实现特性

**参考代码位置**：
- AST 中已有 `StmtKind::Var { is_mut: bool }` 字段
- `src/frontend/typecheck/check.rs` 中的 `check_var` 函数
- `src/frontend/typecheck/infer.rs` 中的 `infer_var_decl` 函数

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
