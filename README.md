# YaoXiang（爻象）编程语言

> 一门实验性的通用编程语言，融合类型论、所有权模型和自然语法的力量。
>
> 基于《并作模型：万物并作，吾以观复》

> An experimental general-purpose programming language that integrates the power of type theory, ownership model, and natural syntax.
>
> Based on "Concurrent Model: All Things Work Together, and We Observe the Return"

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Version](https://img.shields.io/badge/Version-v0.5.6--experimental-blue.svg)]()
[![Status](https://img.shields.io/badge/Status-Experiment--Validation-yellow.svg)]()

<!-- language-nav-start -->
🌐 **Language / 语言** | [English](#english) | [中文](#中文)
<!-- language-nav-end -->


<!-- bilingual-section-start -->
## <a name="english"></a>📖 Introduction

YaoXiang (爻象) is an **experimental programming language under active development**, designed to explore the fusion of type theory, ownership models, and natural syntax.

> **⚠️ Project Status: Experimental Validation**
> This is a research project for learning compiler development. The implementation is incomplete and not production-ready.

### Core Design Goals

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
echo 'main: () -> Void = { print("Hello") }' | cargo run -- eval

# Build bytecode (partial implementation)
cargo run -- build docs/examples/hello.yx -o hello.42

# Dump bytecode for debugging
cargo run -- dump docs/examples/hello.yx
```

**Development hooks (pre-commit)**

We use `pre-commit` to run project checks before commits (cross-platform). The repository includes a `.pre-commit-config.yaml` that runs `cargo fmt` and `cargo clippy`.

Recommended installation (uses `pipx` to avoid polluting global site-packages):

```bash
python3 -m pip install --user pipx
python3 -m pipx ensurepath
pipx install pre-commit
pre-commit install
```

Quick install without `pipx`:

```bash
python -m pip install --user pre-commit
pre-commit install
```

Run checks locally:

```bash
pre-commit run --all-files
```

Notes:
- `pre-commit` requires Python 3.7+. On Windows ensure `pre-commit` is in your PATH (restart shell after `pipx ensurepath`).
- If you prefer not to install Python tooling locally, CI can run `pre-commit` to enforce checks centrally.
- The previous `xtasks` tooling has been removed in favor of the cross-platform `pre-commit` workflow.

### Code Example

```yaoxiang
# === Type Definitions ===

# Record types (curly braces)
Point: Type = { x: Float, y: Float }
Result: Type[T, E] = { ok(T) | err(E) }
Color: Type = { red | green | blue }

# Interface types (all fields are function types)
Serializable: Type = { serialize: () -> String }

# Value construction
p = Point(3.0, 4.0)
r = ok("success")

# === Functions ===
add: (a: Int, b: Int) -> Int = a + b

# === Entry Point ===
main: () -> Void = {
    print("Hello, YaoXiang!")
}
```

For more examples, see [docs/examples/](docs/examples/).

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

### Design Philosophy

YaoXiang's design philosophy can be summarized in five principles:

```
Everything is Type → Unified Abstraction → Type as Data → Runtime Available
Ownership Model → Zero-Cost Abstraction → No GC → High Performance
Python Syntax → Natural Language → Readability → Beginner-Friendly
Concurrent Model → Lazy Evaluation → Auto Parallel → Seamless Concurrency
Send/Sync → Compile-Time Check → Data Race → Thread Safety
```

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

### Roadmap

For detailed implementation status and future plans, see [Implementation Roadmap](docs/plan/IMPLEMENTATION-ROADMAP.md).

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


### Yes, It's Still an Experimental Project

Before you criticize, check this out:

- [YaoXiang Design Manifesto (Satirical Version)](docs/design/manifesto-wtf.md) - DeepSeek's Review


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
> 这是一个用于学习编译器开发的研究项目。实现不完整且不适用于生产环境。

### 核心设计目标

| 目标 | 描述 |
|------|-------------|
| **一切皆类型** | 值、函数、模块、泛型都是类型；类型是一等公民 |
| **统一抽象** | 基于类型论的数学抽象框架 |
| **自然语法** | Python 般的可读性，接近自然语言 |
| **并发模型设计** | 同步语法，异步本质（设计阶段，未实现） |
| **AI 友好设计** | 严格结构化，清晰的 AST（设计目标） |

**⚠️ 警告：仅用于实验/教育目的**

#### 安装与构建

```bash
# 克隆并构建（开发版本）
git clone https://github.com/ChenXu233/YaoXiang.git
cd YaoXiang
cargo build

# 运行测试查看当前状态
cargo test

# 尝试示例
cargo run --example hello
```

### 代码示例

```yaoxiang
# === 类型定义 ===

# 记录类型（花括号）
Point: Type = { x: Float, y: Float }
Result: Type[T, E] = { ok(T) | err(E) }
Color: Type = { red | green | blue }

# 接口类型（字段全为函数类型）
Serializable: Type = { serialize: () -> String }

# 值构造
p = Point(3.0, 4.0)
r = ok("success")

# === 函数 ===
add: (a: Int, b: Int) -> Int = a + b

# === 入口点 ===
main: () -> Void = {
    print("Hello, YaoXiang!")
}

# === 并作模型：同步语法，异步本质 ===

# 使用 spawn 标记异步函数
fetch_data: (url: String) -> JSON spawn = {
    return HTTP.get(url).json()
}

# 自动并行：多个 spawn 调用自动并行执行
process_users_and_posts: () -> Void spawn = {
    users = fetch_data("https://api.example.com/users")
    posts = fetch_data("https://api.example.com/posts")

    # 自动并行执行，无需 await
    print(users.length.to_string())
    print(posts.length.to_string())
}

# 并发构造块：显式并行
compute_all: () -> (Int, Int, Int) spawn = {
    (a, b, c) = spawn {
        heavy_calc(1),
        heavy_calc(2),
        heavy_calc(3)
    }
    return (a, b, c)
}

# === 泛型 ===

identity: [T](x: T) -> T = x
```

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

### 设计理念

YaoXiang 的设计哲学可以用五句话概括：

```
一切皆类型 → 统一抽象 → 类型即数据 → 运行时可用
所有权模型 → 零成本抽象 → 无GC → 高性能
Python语法 → 自然语言感 → 可读性 → 新手友好
并作模型 → 惰性求值 → 自动并行 → 无感并发
Send/Sync → 编译时检查 → 数据竞争 → 线程安全
```

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

### 路线图

详细实现状态和未来计划，请查看 [实现路线图](docs/plan/IMPLEMENTATION-ROADMAP.md)。

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

### 没错，目前还是个实验性项目，相当画饼，想喷之前可以看看这个玩意：

- [爻象设计宣言WTF版](docs/src/design/manifesto-wtf.md) - DeepSeek锐评

> 「道生一，一生二，二生三，三生万物。」
> —— 《道德经》
>
> 类型如道，万物皆由此生。
<!-- separator-end -->
<!-- bilingual-section-end -->