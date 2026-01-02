# YaoXiang（爻象）编程语言

> 一门实验性的通用编程语言，融合类型论、所有权模型和自然语法的力量。

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
| **无感异步** | 无需显式 await，编译器自动处理 |
| **AI 友好** | 严格结构化，AST 清晰，易于解析和修改 |

### 代码示例

```yaoxiang
# 自动类型推断
x = 42
name = "YaoXiang"

# 函数定义
add(Int, Int) -> Int = (a, b) => a + b

# 统一类型语法：只有构造器，没有 enum/struct/union 关键字
# 规则：用 | 分隔的都是构造器，构造器名(参数) 就是类型
type Point = Point(x: Float, y: Float)          # 单构造器（结构体风格）
type Result[T, E] = ok(T) | err(E)              # 多构造器（联合风格）
type Color = red | green | blue                  # 零参构造器（枚举风格）

# 值构造：与函数调用完全相同
p = Point(3.0, 4.0)
r = ok("success")
c = green

# 无感异步
fetch_data(String) -> JSON spawn = (url) => {
    HTTP.get(url).json()
}

main() -> Void = () => {
    data = fetch_data("https://api.example.com")
    print(data.name)
}
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
- [语言指南](docs/YaoXiang-book.md) - 系统学习
- [语言规范](docs/YaoXiang-language-specification.md) - 完整参考
- [设计宣言](docs/YaoXiang-design-manifesto.md) - 核心理念与路线图
- [实现计划](docs/YaoXiang-implementation-plan.md) - 技术细节

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
│   ├── YaoXiang-book.md                    # 语言指南
│   ├── YaoXiang-language-specification.md  # 语言规范
│   ├── YaoXiang-implementation-plan.md     # 实现计划
│   ├── guides/                               # 指南文档
│   ├── reference/                            # 参考文档
│   ├── architecture/                         # 架构文档
│   └── examples/                             # 示例代码
└── tests/                    # 测试
```

## 设计理念

YaoXiang 的设计哲学可以用四句话概括：

```
一切皆类型 → 统一抽象 → 类型即数据 → 运行时可用
所有权模型 → 零成本抽象 → 无GC → 高性能
Python语法 → 自然语言感 → 可读性 → 新手友好
自动推断 → 极简关键字 → 简洁表达 → AI友好
```

## 与现有语言的对比

| 特性 | YaoXiang | Rust | Python | TypeScript |
|------|----------|------|--------|------------|
| 一切皆类型 | ✅ | ❌ | ❌ | ❌ |
| 自动类型推断 | ✅ | ✅ | ✅ | ✅ |
| 默认不可变 | ✅ | ✅ | ❌ | ❌ |
| 所有权模型 | ✅ | ✅ | ❌ | ❌ |
| 无感异步 | ✅ | ❌ | ❌ | ❌ |
| 依赖类型 | ✅ | ❌ | ❌ | ❌ |
| 零成本抽象 | ✅ | ✅ | ❌ | ❌ |
| 无GC | ✅ | ✅ | ❌ | ❌ |
| AI友好语法 | ✅ | ❌ | ✅ | ❌ |
| 关键字数量 | 17 | 51+ | 35 | 64+ |

## 路线图

| 版本 | 目标 | 时间 |
|------|------|------|
| v0.1 | 解释器原型 | 1-2 个月 |
| v0.5 | 完整解释器 | 3-4 个月 |
| v1.0 | AOT 编译器 | 8-10 个月 |
| v2.0 | 自举编译器 | 14 个月 |

详见 [实现计划](docs/YaoXiang-implementation-plan.md)

## 贡献

欢迎贡献！请阅读 [贡献指南](CONTRIBUTING.md)（待添加）。

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

---

> 「道生一，一生二，二生三，三生万物。」
> —— 《道德经》
>
> 类型如道，万物皆由此生。
