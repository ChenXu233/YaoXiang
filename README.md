# YaoXiang（爻象）编程语言

> 一门实验性的通用编程语言，融合类型论、所有权模型和自然语法的力量。
>
> 基于《并作模型：万物并作，吾以观复》

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Version](https://img.shields.io/badge/Version-v1.0.0--draft-blue.svg)]()

## 简介

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

## 快速开始

### 安装

```bash
# 从源码编译
git clone https://github.com/yourusername/yaoxiang.git
cd yaoxiang
cargo build --release
```

### 运行

```bash
yaoxiang your_program.yx
```

### 文档

- [快速入门](docs/guides/getting-started.md) - 5 分钟上手
- [语言指南](docs/guides/YaoXiang-book.md) - 系统学习核心概念
- [语言规范](docs/design/language-spec.md) - 完整语法和语义定义
- [异步白皮书](docs/design/async-whitepaper.md) - 无感异步设计
- [教程](docs/tutorial/) - 逐步示例和最佳实践
- [架构设计](docs/architecture/) - 编译器与运行时设计

## 项目结构

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
|   │   ├── accepted/              # 已接受的设计提案
|   │   ├── rfc/                   # 设计提案草案
|   │   ├── discussion/            # 设计讨论区
|   |   ├── manifesto.md           # 设计宣言
│   |   ├── manifesto-wtf.md       # 设计宣言WTF版
│   │   ├── language-spec.md       # 语言规范
│   │   └── async-whitepaper.md    # 异步白皮书
│   ├── guides/             # 使用指南
│   │   ├── getting-started.md     # 快速入门
│   │   ├── YaoXiang-book.md       # 语言指南
│   │   └── dev/                   # 开发者指南
│   ├── tutorial/           # 详细教程
│   │   ├── basics.md               # 基础教程
│   │   ├── types.md                # 类型系统
│   │   └── functions.md            # 函数与闭包
│   ├── architecture/       # 架构文档
│   ├── plans/              # 实施计划
│   ├── implementation/     # 实现追踪
│   ├── examples/           # 示例代码
│   └── maintenance/        # 维护规范
└── tests/                  # 测试
```

## 设计理念

YaoXiang 的设计哲学可以用五句话概括：

```
一切皆类型 → 统一抽象 → 类型即数据 → 运行时可用
所有权模型 → 零成本抽象 → 无GC → 高性能
Python语法 → 自然语言感 → 可读性 → 新手友好
并作模型 → 惰性求值 → 自动并行 → 无感并发
Send/Sync → 编译时检查 → 数据竞争 → 线程安全
```

## 与现有语言的对比

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

## 路线图

| 版本 | 目标 | 时间 |
|------|------|------|
| v0.1 | 解释器原型 | 1-2 个月 |
| v0.5 | 完整解释器 | 3-4 个月 |
| v1.0 | AOT 编译器 | 8-10 个月 |
| v2.0 | 自举编译器 | 14 个月 |

详见 [实现计划](docs/plans/YaoXiang-implementation-plan.md)

## 贡献

欢迎贡献！请阅读 [贡献指南](CONTRIBUTING.md)。

## 社区

- GitHub Issues: 功能建议、问题报告
- Discussions: 讨论交流

## 许可

本项目采用 MIT 许可证，详见 [LICENSE](LICENSE)。

## 致谢

YaoXiang 的设计灵感来自以下项目和语言：

- **Rust** - 所有权模型、零成本抽象
- **Python** - 语法风格、可读性
- **Idris/Agda** - 依赖类型、类型驱动开发
- **TypeScript** - 类型注解、运行时类型
- **MoonBit** - AI 友好设计


## 没错，目前还是个实验性项目，相当画饼，想喷之前可以看看这个玩意：

- [爻象设计宣言WTF版](docs/design/manifesto-wtf.md) - DeepSeek锐评


---

> 「道生一，一生二，二生三，三生万物。」
> —— 《道德经》
>
> 类型如道，万物皆由此生。
