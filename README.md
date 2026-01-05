# YaoXiang（爻象）编程语言

> 一门实验性的通用编程语言，融合类型论、所有权模型和自然语法的力量。
>
> 基于《并作模型：万物并作，吾以观复》

> An experimental general-purpose programming language that integrates the power of type theory, ownership model, and natural syntax.
>
> Based on "Concurrent Model: All Things Work Together, and We Observe the Return"

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Version](https://img.shields.io/badge/Version-v1.0.0--draft-blue.svg)]()

---

<!-- language-nav-start -->
🌐 **Language / 语言** | [English](#english) | [中文](#中文)
<!-- language-nav-end -->

---

<!-- bilingual-section-start -->
## <a name="english"></a>📖 Introduction

YaoXiang (爻象) is an experimental general-purpose programming language whose design philosophy originates from the core concepts of "爻" (yao) and "象" (xiang) in the I Ching (Book of Changes).

### Core Features

| Feature | Description |
|---------|-------------|
| **Everything is Type** | Values, functions, modules, generics are all types; types are first-class citizens |
| **Mathematical Abstraction** | Unified abstraction framework based on type theory |
| **Zero-Cost Abstraction** | High performance, no GC, ownership model ensures memory safety |
| **Natural Syntax** | Python-like readability, close to natural language |
| **Concurrent Model** | Synchronous syntax, async nature; "All things work together, we observe the return" |
| **Thread Safety** | Send/Sync type constraints, compile-time concurrency safety |
| **AI-Friendly** | Strictly structured, clear AST, easy to parse and modify |

### Code Example

```yaoxiang
# Automatic type inference
x: Int = 42
y = 42                               # Inferred as Int
name = "YaoXiang"                    # Inferred as String

# Unified declaration syntax: identifier: Type = expression
add: (Int, Int) -> Int = (a, b) => a + b
inc: Int -> Int = x => x + 1

# Unified type syntax: only constructors, no enum/struct/union keywords
# Rule: Separated by | are constructors, constructor_name(parameters) is the type
type Point = Point(x: Float, y: Float)          # Single constructor (struct style)
type Result[T, E] = ok(T) | err(E)              # Multiple constructors (union style)
type Color = red | green | blue                  # Zero-parameter constructors (enum style)

# Value construction: exactly the same as function calls
p = Point(3.0, 4.0)
r = ok("success")
c = green

# === Concurrent Model: Synchronous Syntax, Async Nature ===

# Use spawn to mark async function - syntax exactly the same as normal functions
fetch_data: (String) -> JSON spawn = (url) => {
    HTTP.get(url).json()
}

# Auto parallel: multiple spawn calls automatically execute in parallel
process_users_and_posts: () -> Void spawn = () => {
    users = fetch_data("https://api.example.com/users")  # Async[JSON]
    posts = fetch_data("https://api.example.com/posts")  # Async[JSON]

    # users and posts automatically execute in parallel, no await needed
    print("Users: " + users.length.to_string())
    print("Posts: " + posts.length.to_string())
}

# Concurrent block: explicit parallelism
compute_all: () -> (Int, Int, Int) spawn = () => {
    # Expressions in spawn { } execute in parallel
    (a, b, c) = spawn {
        heavy_calc(1),    # Independent task 1
        heavy_calc(2),    # Independent task 2
        heavy_calc(3)     # Independent task 3
    }
    (a, b, c)
}

# Data parallel loop
parallel_sum: (Int) -> Int spawn = (n) => {
    # Loops marked with spawn for are automatically parallelized
    total = spawn for i in 0..n {
        fibonacci(i)  # Each iteration executes in parallel
    }
    total
}

# === Thread Safety: Send/Sync Constraints ===

# Arc: Atomic reference counting (thread-safe)
type ThreadSafeCounter = ThreadSafeCounter(value: Int)

main: () -> Void = () => {
    # Arc implements Send + Sync
    counter: Arc[ThreadSafeCounter] = Arc.new(ThreadSafeCounter(0))

    # spawn automatically checks Send constraint
    spawn(|| => {
        guard = counter.value.lock()  # Mutex provides internal mutability
        guard.value = guard.value + 1
    })

    # ...
}

# === Generics and Higher-Order Functions ===

# Generic function
identity: <T> (T) -> T = x => x

# Higher-order function
apply: ((Int) -> Int, Int) -> Int = (f, x) => f(x)

# Currying
add_curried: Int -> Int -> Int = a => b => a + b
```

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
│   │       ├── basics.md          # Basics
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
| Keyword Count | 17 | 51+ | 35 | 64+ | 25 |

> **Concurrent Model** = Synchronous Syntax + Lazy Evaluation + Auto Parallel + Seamless Async

---

### Roadmap

| Version | Goal | Time |
|---------|------|------|
| v0.1 | Interpreter Prototype | 1-2 months |
| v0.5 | Complete Interpreter | 3-4 months |
| v1.0 | AOT Compiler | 8-10 months |
| v2.0 | Self-Hosting Compiler | 14 months |

See [Implementation Plan](docs/plans/YaoXiang-implementation-plan.md) for details.

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

YaoXiang（爻象）是一门实验性的通用编程语言，其设计理念源于《易经》中「爻」与「象」的核心概念。

### 核心特性

| 特性 | 说明 |
|------|------|
| **一切皆类型** | 值、函数、模块、泛型都是类型，类型是一等公民 |
| **数学抽象** | 基于类型论的统一抽象框架 |
| **零成本抽象** | 高性能，无 GC，所有权模型保证内存安全 |
| **自然语法** | Python 般的可读性，接近自然语言 |
| **并作模型** | 同步语法，异步本质；「万物并作，吾以观复」 |
| **线程安全** | Send/Sync 类型约束，编译时保证并发安全 |
| **AI 友好** | 严格结构化，AST 清晰，易于解析和修改 |

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

| 特性 | YaoXiang | Rust | Python | TypeScript | Go |
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

| 版本 | 目标 | 时间 |
|------|------|------|
| v0.1 | 解释器原型 | 1-2 个月 |
| v0.5 | 完整解释器 | 3-4 个月 |
| v1.0 | AOT 编译器 | 8-10 个月 |
| v2.0 | 自举编译器 | 14 个月 |

详见 [实现计划](docs/plans/YaoXiang-implementation-plan.md)

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
