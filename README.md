# YaoXiang（爻象）编程语言

> 一门实验性的通用编程语言，融合类型论、所有权模型和自然语法的力量。
>
> 基于《并作模型：万物并作，吾以观复》

> An experimental general-purpose programming language that integrates the power of type theory, ownership model, and natural syntax.
>
> Based on "Concurrent Model: All Things Work Together, and We Observe the Return"

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Version](https://img.shields.io/badge/Version-v0.2.2--experimental-blue.svg)]()
[![Status](https://img.shields.io/badge/Status-Experiment--Validation-yellow.svg)]()

---

<!-- language-nav-start -->
🌐 **Language / 语言** | [English](#english) | [中文](#中文)
<!-- language-nav-end -->

---

<!-- bilingual-section-start -->
## <a name="english"></a>📖 Introduction

YaoXiang (爻象) is an **experimental programming language under active development**, designed to explore the fusion of type theory, ownership models, and natural syntax.

> **⚠️ Project Status: Experimental Validation**  
> This is a research project for learning compiler development. The implementation is incomplete and not production-ready. See [Project Status](#project-status-experimental-validation) for current implementation level.

### Project Status: Phase 4 - Codegen in Progress

## Current Implementation Progress (based on docs/plan phase division):

| Phase | Module | Status | Location |
|-------|------|------|------|
| P1 | Lexer | ✅ Complete | `src/frontend/lexer/` |
| P2 | Parser | ✅ Complete | `src/frontend/parser/` |
| P3 | Type Checker | ✅ Complete | `src/frontend/typecheck/` |
| P4 | Bytecode Generator | ✅ In Progress | `src/middle/codegen/` |
| P5-10 | Optimization Phase | 🔶 To Be Implemented | `src/middle/` |
| P11 | Virtual Machine | ⏳ To Be Implemented | `src/vm/` |
| P12-19 | Runtime/Toolchain | ⏳ To Be Implemented | `src/runtime/` |

## Module Details:
- ✅ **Lexer**: Complete token support, supports all literals
- ✅ **Parser**: Complete Pratt Parser, supports functions/types/control flow
- ✅ **Type Checker**: Type inference, monomorphization, specialization completed
- ✅ **Bytecode Generator**: Expression/statement generation in progress
- 🔶 **Optimizer**: Ownership system, lifetime, monomorphization to be improved
- ⏳ **Runtime**: DAG, scheduler, VM to be implemented

## Next Goals (v0.1):
- Complete P4 Bytecode Generator
- Implement P11 Virtual Machine
- End-to-end Hello World execution

See [docs/plan/IMPLEMENTATION-ROADMAP.md](docs/plan/IMPLEMENTATION-ROADMAP.md) for detailed implementation status.

### Getting Started

**⚠️ Warning: This is for experimental/educational use only**

#### Installation & Building

```bash
# Clone and build (development build)
git clone https://github.com/ChenXu233/YaoXiang.git
cd YaoXiang
cargo build

# Run tests to see current status
cargo test

# Try the examples (some may not work)
cargo run --example hello
```

#### Current Working Features

```bash
# Basic tokenization and parsing only
echo 'main: () -> Void = () => { print("Hello") }' | cargo run -- eval

# Build bytecode (partial implementation)
cargo run -- build docs/examples/hello.yx -o hello.42

# Dump bytecode for debugging
cargo run -- dump docs/examples/hello.yx
```

### Core Design Goals

| Goal | Description |
|------|-------------|
| **Everything is Type** | Values, functions, modules, generics are all types; types are first-class citizens |
| **Unified Abstraction** | Mathematical abstraction framework based on type theory |
| **Natural Syntax** | Python-like readability, close to natural language |
| **Concurrent Model Design** | Synchronous syntax, async nature (design phase, not implemented) |
| **AI-Friendly Design** | Strictly structured, clear AST (design goal) |

### Code Example

```yaoxiang
# === Basic Syntax ===

# Variable with type inference
x: Int = 42
y = 42                               # Inferred as Int
name = "YaoXiang"                    # Inferred as String

# Function definition (type signature required)
add: (Int, Int) -> Int = (a, b) => a + b
inc: Int -> Int = x => x + 1

# Type definition (only constructors)
type Point = Point(x: Float, y: Float)
type Result[T, E] = ok(T) | err(E)
type Color = red | green | blue

# Value construction
p = Point(3.0, 4.0)
r = ok("success")

# === Entry Point ===
main: () -> Void = () => {
    print("Hello, YaoXiang!")
    print("2 + 3 = " + add(2, 3).to_string())
}
```

For more examples, see [docs/examples/](docs/examples/).

---

### Getting Started

#### Installation

```bash
# Build from source
git clone https://github.com/yourusername/yaoxiang.git
cd yaoxiang
cargo build --release
```

#### Running

```bash
yaoxiang your_program.yx
```

#### Documentation

- [Quick Start](docs/guides/getting-started.md) - Get started in 5 minutes
- [Language Guide](docs/guides/YaoXiang-book.md) - Learn core concepts systematically
- [Language Specification](docs/design/language-spec.md) - Complete syntax and semantics
- [Async Whitepaper](docs/design/async-whitepaper.md) - Seamless async design
- [Tutorial](docs/tutorial/) - Step-by-step examples and best practices
- [Architecture](docs/architecture/) - Compiler and runtime design

---

### Project Structure

```
yaoxiang/
├── Cargo.toml              # Project configuration
├── README.md               # This file
├── LICENSE                 # MIT License
├── src/                    # Source code
│   ├── main.rs             # CLI entry point
│   └── lib.rs              # Library entry point
├── docs/                   # Documentation
│   ├── design/             # Design discussion area
│   │   ├── accepted/              # Accepted design proposals
│   │   ├── rfc/                   # Design proposal drafts
│   │   ├── discussion/            # Design discussion area
│   │   ├── manifesto.md           # Design manifesto
│   │   ├── manifesto-wtf.md       # Design manifesto (satirical)
│   │   ├── language-spec.md       # Language specification
│   │   └── async-whitepaper.md    # Async whitepaper
│   ├── guides/             # User guides
│   │   ├── getting-started.md     # Quick start
│   │   ├── getting-started.en.md  # Quick Start (English)
│   │   ├── YaoXiang-book.md       # Language guide
│   │   ├── YaoXiang-book.en.md    # Language Guide (English)
│   │   └── dev/                   # Developer guides
│   ├── tutorial/           # Tutorials
│   │   ├── zh/                    # Chinese tutorials
│   │   │   ├── README.md          # Tutorial index
│   │   │   ├── basics.md          # Basics
│   │   │   ├── types.md           # Type system
│   │   │   └── functions.md       # Functions and closures
│   │   └── en/                    # English tutorials
│   │       ├── README.md          # Tutorial index
│   │       ├── basics.md          # Quick Start
│   │       ├── types.md           # Type system
│   │       └── functions.md       # Functions and closures
│   ├── architecture/       # Architecture documents
│   ├── plans/              # Implementation plans
│   ├── implementation/     # Implementation tracking
│   ├── examples/           # Example code
│   └── maintenance/        # Maintenance specifications
└── tests/                  # Tests
```

---

### Design Philosophy

YaoXiang's design philosophy can be summarized in five principles:

```
Everything is Type → Unified Abstraction → Type as Data → Runtime Available
Ownership Model → Zero-Cost Abstraction → No GC → High Performance
Python Syntax → Natural Language → Readability → Beginner-Friendly
Concurrent Model → Lazy Evaluation → Auto Parallel → Seamless Concurrency
Send/Sync → Compile-Time Check → Data Race → Thread Safety
```

---

### Comparison with Existing Languages

| Feature | YaoXiang | Rust | Python | TypeScript | Go |
|---------|----------|------|--------|------------|-----|
| Everything is Type | ✅ | ❌ | ❌ | ❌ | ❌ |
| Auto Type Inference | ✅ | ✅ | ✅ | ✅ | ❌ |
| Default Immutable | ✅ | ✅ | ❌ | ❌ | ❌ |
| Ownership Model | ✅ | ✅ | ❌ | ❌ | ❌ |
| Concurrent Model | ✅ | ❌ | ❌ | ❌ | ⚠️ |
| Zero-Cost Abstraction | ✅ | ✅ | ❌ | ❌ | ❌ |
| No GC | ✅ | ✅ | ❌ | ❌ | ❌ |
| Compile-Time Thread Safety | ✅ | ✅ | ❌ | ❌ | ❌ |
| AI-Friendly Syntax | ✅ | ❌ | ✅ | ❌ | ❌ |
| Keyword Count | 18 | 51+ | 35 | 64+ | 25 |

> **Concurrent Model** = Synchronous Syntax + Lazy Evaluation + Auto Parallel + Seamless Async

---

### Roadmap

For detailed implementation status and future plans, see [Implementation Roadmap](docs/plan/IMPLEMENTATION-ROADMAP.md).

---

### Contributing

Contributions are welcome! Please read the [Contribution Guide](CONTRIBUTING.md).

### Community

- GitHub Issues: Feature suggestions, bug reports
- Discussions: Discussion and exchange

### License

This project uses the MIT License. See [LICENSE](LICENSE) for details.

### Acknowledgments

YaoXiang's design is inspired by the following projects and languages:

- **Rust** - Ownership model, zero-cost abstraction
- **Python** - Syntax style, readability
- **Idris/Agda** - Dependent types, type-driven development
- **TypeScript** - Type annotations, runtime types
- **MoonBit** - AI-friendly design

---

### Yes, It's Still an Experimental Project

Before you criticize, check this out:

- [YaoXiang Design Manifesto (Satirical Version)](docs/design/manifesto-wtf.md) - DeepSeek's Review

---

> "道生一，一生二，二生三，三生万物。"
> —— 《道德经》
>
> "The One generates two, two generates three, three generates all things."
> — Tao Te Ching
>
> Types are like the Way, all things are born from them.


## <a name="中文"></a>📖 简介

YaoXiang（爻象）是**一门正在积极开发中的实验性编程语言**，旨在探索类型论、所有权模型和自然语法的融合。

> **⚠️ 项目状态：实验验证阶段**  
> 这是一个用于学习编译器开发的研究项目。实现不完整且不适用于生产环境。当前实现进度见[项目状态](#项目状态实验验证)。

### 项目状态：Phase 4 - 字节码生成器进行中

**当前实现进度** (基于 docs/plan 阶段划分):

| Phase | 模块 | 状态 | 位置 |
|-------|------|------|------|
| P1 | 词法分析器 | ✅ 完成 | `src/frontend/lexer/` |
| P2 | 语法分析器 | ✅ 完成 | `src/frontend/parser/` |
| P3 | 类型检查器 | ✅ 完成 | `src/frontend/typecheck/` |
| P4 | 字节码生成器 | ✅ 进行中 | `src/middle/codegen/` |
| P5-10 | 优化阶段 | 🔶 待实现 | `src/middle/` |
| P11 | 虚拟机 | ⏳ 待实现 | `src/vm/` |
| P12-19 | Runtime/工具链 | ⏳ 待实现 | `src/runtime/` |

**各模块详情**:
- ✅ **词法分析器**: Token 完整，支持所有字面量
- ✅ **语法分析器**: Pratt Parser 完整，函数/类型/控制流
- ✅ **类型检查器**: 类型推断、单态化、特化完成
- ✅ **字节码生成器**: 表达式/语句生成中
- 🔶 **优化器**: 所有权系统、生命周期、单态化待完善
- ⏳ **运行时**: DAG、调度器、VM 待实现

**下一步目标 (v0.1)**:
- 完成 P4 字节码生成器
- 实现 P11 虚拟机
- 端到端运行 Hello World

详见 [docs/plan/IMPLEMENTATION-ROADMAP.md](docs/plan/IMPLEMENTATION-ROADMAP.md) 了解详细实现状态。

### 快速开始

**⚠️ 警告：仅用于实验/教育目的**

#### 安装与构建

```bash
# 克隆并构建（开发版本）
git clone https://github.com/ChenXu233/YaoXiang.git
cd YaoXiang
cargo build

# 运行测试查看当前状态
cargo test

# 尝试示例（某些可能无法工作）
cargo run --example hello
```

#### 当前可用功能

```bash
# 完整编译流程
cargo run -- build docs/examples/hello.yx -o hello.yxb    # 编译为字节码
cargo run -- run hello.yxb                                 # 运行字节码
cargo run -- dump docs/examples/hello.yx                   # 转储 AST/字节码用于调试

# 当前支持的功能：
# - 词法分析：所有字面量、关键字、标识符
# - 语法分析：函数定义、类型定义、控制流、模式匹配
# - 类型检查：类型推断、单态化、泛型特化
# - 字节码生成：表达式、语句、闭包、控制流
# - 虚拟机：指令解释执行（进行中）
```

### 核心设计目标

| 目标 | 描述 |
|------|-------------|
| **一切皆类型** | 值、函数、模块、泛型都是类型；类型是一等公民 |
| **统一抽象** | 基于类型论的数学抽象框架 |
| **自然语法** | Python 般的可读性，接近自然语言 |
| **并发模型设计** | 同步语法，异步本质（设计阶段，未实现） |
| **AI 友好设计** | 严格结构化，清晰的 AST（设计目标） |

### 代码示例

```yaoxiang
# 自动类型推断
x: Int = 42
y = 42                               # 推断为 Int
name = "YaoXiang"                    # 推断为 String

# 统一声明语法：标识符: 类型 = 表达式
add: (Int, Int) -> Int = (a, b) => a + b
inc: Int -> Int = x => x + 1

# 统一类型语法：只有构造器，没有 enum/struct/union 关键字
# 规则：用 | 分隔的都是构造器，构造器名(参数) 就是类型
type Point = Point(x: Float, y: Float)          # 单构造器（结构体风格）
type Result[T, E] = ok(T) | err(E)              # 多构造器（联合风格）
type Color = red | green | blue                  # 零参构造器（枚举风格）

# 值构造：与函数调用完全相同
p = Point(3.0, 4.0)
r = ok("success")
c = green

# === 并作模型：同步语法，异步本质 ===

# 使用 spawn 标记异步函数 - 语法与普通函数完全一致
fetch_data: (String) -> JSON spawn = (url) => {
    HTTP.get(url).json()
}

# 自动并行：多个 spawn 调用自动并行执行
process_users_and_posts: () -> Void spawn = () => {
    users = fetch_data("https://api.example.com/users")  # Async[JSON]
    posts = fetch_data("https://api.example.com/posts")  # Async[JSON]

    # users 和 posts 自动并行执行，无需 await
    print("Users: " + users.length.to_string())
    print("Posts: " + posts.length.to_string())
}

# 并发构造块：显式并行
compute_all: () -> (Int, Int, Int) spawn = () => {
    # spawn { } 内的表达式强制并行执行
    (a, b, c) = spawn {
        heavy_calc(1),    # 独立任务 1
        heavy_calc(2),    # 独立任务 2
        heavy_calc(3)     # 独立任务 3
    }
    (a, b, c)
}

# 数据并行循环
parallel_sum: (Int) -> Int spawn = (n) => {
    # spawn for 标记的循环自动并行化
    total = spawn for i in 0..n {
        fibonacci(i)  # 每次迭代并行执行
    }
    total
}

# === 线程安全：Send/Sync 约束 ===

# Arc：原子引用计数（线程安全）
type ThreadSafeCounter = ThreadSafeCounter(value: Int)

main: () -> Void = () => {
    # Arc 实现 Send + Sync
    counter: Arc[ThreadSafeCounter] = Arc.new(ThreadSafeCounter(0))

    # spawn 自动检查 Send 约束
    spawn(|| => {
        guard = counter.value.lock()  # Mutex 提供内部可变性
        guard.value = guard.value + 1
    })

    # ...
}

# === 泛型与高阶函数 ===

# 泛型函数
identity: <T> (T) -> T = x => x

# 高阶函数
apply: ((Int) -> Int, Int) -> Int = (f, x) => f(x)

# 柯里化
add_curried: Int -> Int -> Int = a => b => a + b
```

---

### 快速开始

#### 安装

```bash
# 从源码编译
git clone https://github.com/yourusername/yaoxiang.git
cd yaoxiang
cargo build --release
```

#### 运行

```bash
yaoxiang your_program.yx
```

#### 文档

- [快速入门](docs/guides/getting-started.md) - 5 分钟上手
- [语言指南](docs/guides/YaoXiang-book.md) - 系统学习核心概念
- [语言规范](docs/design/language-spec.md) - 完整语法和语义定义
- [异步白皮书](docs/design/async-whitepaper.md) - 无感异步设计
- [教程](docs/tutorial/) - 逐步示例和最佳实践
- [架构设计](docs/architecture/) - 编译器与运行时设计

---

### 项目结构

```
yaoxiang/
├── Cargo.toml              # 项目配置
├── README.md               # 本文件
├── LICENSE                 # MIT 许可证
├── src/                    # 源代码
│   ├── main.rs             # CLI 入口
│   └── lib.rs              # 库入口
├── docs/                   # 文档
│   ├── design/             # 设计讨论区
│   │   ├── accepted/              # 已接受的设计提案
│   │   ├── rfc/                   # 设计提案草案
│   │   ├── discussion/            # 设计讨论区
│   │   ├── manifesto.md           # 设计宣言
│   │   ├── manifesto-wtf.md       # 设计宣言WTF版
│   │   ├── language-spec.md       # 语言规范
│   │   └── async-whitepaper.md    # 异步白皮书
│   ├── guides/             # 使用指南
│   │   ├── getting-started.md     # 快速入门
│   │   ├── getting-started.en.md  # Quick Start (English)
│   │   ├── YaoXiang-book.md       # 语言指南
│   │   ├── YaoXiang-book.en.md    # Language Guide (English)
│   │   └── dev/                   # 开发者指南
│   ├── tutorial/           # 教程
│   │   ├── zh/                    # 中文教程
│   │   │   ├── README.md          # 教程索引
│   │   │   ├── basics.md          # 基础教程
│   │   │   ├── types.md           # 类型系统
│   │   │   └── functions.md       # 函数与闭包
│   │   └── en/                    # English tutorials
│   │       ├── README.md          # Tutorial index
│   │       ├── basics.md          # Quick Start
│   │       ├── types.md           # Type system
│   │       └── functions.md       # Functions and closures
│   ├── architecture/       # 架构文档
│   ├── plans/              # 实施计划
│   ├── implementation/     # 实现追踪
│   ├── examples/           # 示例代码
│   └── maintenance/        # 维护规范
└── tests/                  # 测试
```

---

### 设计理念

YaoXiang 的设计哲学可以用五句话概括：

```
一切皆类型 → 统一抽象 → 类型即数据 → 运行时可用
所有权模型 → 零成本抽象 → 无GC → 高性能
Python语法 → 自然语言感 → 可读性 → 新手友好
并作模型 → 惰性求值 → 自动并行 → 无感并发
Send/Sync → 编译时检查 → 数据竞争 → 线程安全
```

---

### 与现有语言的对比

| 特性 | 设计目标 | Rust | Python | TypeScript | Go |
|------|----------|------|--------|------------|-----|
| 一切皆类型 | ✅ | ❌ | ❌ | ❌ | ❌ |
| 自动类型推断 | ✅ | ✅ | ✅ | ✅ | ❌ |
| 默认不可变 | ✅ | ✅ | ❌ | ❌ | ❌ |
| 所有权模型 | ✅ | ✅ | ❌ | ❌ | ❌ |
| 并作模型 | ✅ | ❌ | ❌ | ❌ | ⚠️ |
| 零成本抽象 | ✅ | ✅ | ❌ | ❌ | ❌ |
| 无GC | ✅ | ✅ | ❌ | ❌ | ❌ |
| 编译时线程安全 | ✅ | ✅ | ❌ | ❌ | ❌ |
| AI友好语法 | ✅ | ❌ | ✅ | ❌ | ❌ |
| 关键字数量 | 17 | 51+ | 35 | 64+ | 25 |

> **并作模型** = 同步语法 + 惰性求值 + 自动并行 + 无感异步

---

### 路线图

详细实现状态和未来计划，请查看 [实现路线图](docs/plan/IMPLEMENTATION-ROADMAP.md)。

---

### 贡献

欢迎贡献！请阅读 [贡献指南](CONTRIBUTING.md)。

### 社区

- GitHub Issues: 功能建议、问题报告
- Discussions: 讨论交流

### 许可

本项目采用 MIT 许可证，详见 [LICENSE](LICENSE)。

### 致谢

YaoXiang 的设计灵感来自以下项目和语言：

- **Rust** - 所有权模型、零成本抽象
- **Python** - 语法风格、可读性
- **Idris/Agda** - 依赖类型、类型驱动开发
- **TypeScript** - 类型注解、运行时类型
- **MoonBit** - AI 友好设计

---

### 没错，目前还是个实验性项目，相当画饼，想喷之前可以看看这个玩意：

- [爻象设计宣言WTF版](docs/design/manifesto-wtf.md) - DeepSeek锐评

---

> 「道生一，一生二，二生三，三生万物。」
> —— 《道德经》
>
> 类型如道，万物皆由此生。
<!-- separator-end -->
<!-- bilingual-section-end -->