# YaoXiang（爻象）编程语言

> AI辅助的编译器开发探索。

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Version](https://img.shields.io/badge/Version-v0.7.6--patch1-blue.svg)]()

> 🌐 **Language** | [English](docs/gh/README.en.md)
>
> ❤️ **Docs** | [Docs Website](https://chenxu233.github.io/YaoXiang/)

## 简介

YaoXiang（爻象）是**一门正在积极开发中的实验性编程语言**，旨在探索类型论、所有权模型和自然语法的融合。

> **项目状态：实验验证阶段**
> 这是一个用于学习编译器开发的研究项目。实现不完整且不适用于生产环境。


### 核心设计目标

| 目标 | 描述 |
|------|------|
| **一切皆类型** | 值、函数、模块、泛型都是类型；类型是一等公民 |
| **统一语法** | 一切皆 `name: type = value`——一个规则覆盖所有声明 |
| **自然语法** | Python 般的可读性，接近自然语言 |
| **所有权模型** | Move 语义 + 借用令牌 + ref 共享——无 GC，无生命周期标注 |
| **并发模型** | 同步语法，异步本质（设计阶段，未实现） |
| **值依赖类型** | 类型可依赖值，支持编译期维度验证 |


## 代码示例

```yaoxiang
# ═══════════ 类型定义（统一语法：name: type = value）═══════════

# 记录类型
Point: Type = {
    x: Float,
    y: Float,
}

# 泛型类型
Option: (T: Type) -> Type = {
    some: (T) -> Self,
    none: () -> Self,
}

# 接口（字段全为函数类型的记录）
Drawable: Type = {
    draw: (Surface) -> Void,
}

# ═══════════ 函数 ═══════════

add: (a: Int, b: Int) -> Int = a + b

# 泛型函数
map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R)) = ...

# ═══════════ 所有权模型 ═══════════

# Move（默认）：零拷贝所有权转移
p1 = Point(1.0, 2.0)
p2 = p1              # Move，p1 不可再读

# &T / &mut T 借用令牌：零成本编译期访问权限
p2.print()           # 编译器自动创建 &Point 令牌
p2.shift(1.0, 1.0)  # 编译器自动创建 &mut Point 令牌

# ref：共享持有（编译器自动选 Rc 或 Arc）
shared = ref p2      # 跨作用域共享

# clone()：显式深拷贝
backup = p2.clone()

# ═══════════ 方法定义 ═══════════

Point.draw: (self: &Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}

Point.shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
    self.x = self.x + dx
    self.y = self.y + dy
}

# ═══════════ 入口点 ═══════════

main: () -> Void = {
    print("Hello, YaoXiang!")
}
```

## 类型系统

### 统一语法模型

YaoXiang 只有一种声明形式：**`identifier : type = expression`**

| 概念 | 写法 |
|------|------|
| 变量 | `x: Int = 42` |
| 函数 | `add: (a: Int, b: Int) -> Int = a + b` |
| 记录类型 | `Point: Type = { x: Float, y: Float }` |
| 接口 | `Drawable: Type = { draw: (Surface) -> Void }` |
| 泛型类型 | `List: (T: Type) -> Type = { data: Array(T), length: Int }` |
| 方法 | `Point.draw: (self: &Point, s: Surface) -> Void = ...` |

**没有 `fn`、`struct`、`trait`、`impl` 关键字。** `Type` 是语言中唯一的元类型关键字。

详见 [RFC-010：统一类型语法](docs/src/design/rfc/accepted/010-unified-type-syntax.md)。

### 泛型与值依赖类型

YaoXiang 的泛型系统支持**类型依赖值**，允许在编译期进行维度验证：

```yaoxiang
# 矩阵类型：维度在编译期确定
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),
}

# 编译期计算：factorial(3) = 6
vec: Vec(factorial(3)) = Vec(6)()

# 编译期维度验证：维度不匹配在编译期捕获
# multiply(matrix_2x3, matrix_4x2)  # 编译错误：3 != 4
```

详见 [RFC-011：泛型系统设计](docs/src/design/rfc/accepted/011-generic-type-system.md)。


## 安装与构建

```bash
# 克隆项目
git clone https://github.com/yaoxiang-lang/yaoxiang.git
cd yaoxiang

# 一键安装 Z3 依赖（纯 Rust，自动下载预编译包）
cd tools/setup-z3 && cargo run && cd ../..

# 构建
cargo build

# 运行测试
cargo test

# 尝试示例
cargo run --example hello
```

> Z3 是编译器的 SMT 求解模块，用于编译期谓词证明（RFC-027）。`tools/setup-z3` 自动从 GitHub Releases 下载对应平台的预编译包到 `.z3/`，写入 `.cargo/config.toml`。首次运行后 `cargo build` 即可直接构建。详见 [RFC-027](docs/src/design/rfc/accepted/027-compile-time-evaluation-types.md)。

### 开发环境配置

我们使用 `pre-commit` 在提交前运行项目检查（跨平台）。仓库包含 `.pre-commit-config.yaml`，运行 `cargo fmt` 和 `cargo clippy`。

推荐安装方式（使用 `pipx`，避免污染全局 site-packages）：

```bash
python3 -m pip install --user pipx
python3 -m pipx ensurepath
pipx install pre-commit
pre-commit install
```

快速安装（不使用 `pipx`）：

```bash
python -m pip install --user pre-commit
pre-commit install
```

本地运行检查：

```bash
pre-commit run --all-files
```


## 项目结构

```
yaoxiang/
├── Cargo.toml                  # 项目配置
├── README.md                   # 本文件
├── LICENSE                     # MIT 许可证
├── src/                        # 源代码
│   ├── main.rs                 # CLI 入口
│   └── lib.rs                  # 库入口
├── docs/                       # 文档
│   ├── src/
│   │   ├── design/             # 设计文档
│   │   │   ├── rfc/            # RFC 提案
│   │   │   │   ├── accepted/   # 已接受的 RFC
│   │   │   │   └── draft/      # RFC 草案
│   │   │   └── manifesto.md    # 设计宣言
│   │   ├── reference/          # 语言参考
│   │   │   └── language-spec/  # 语言规范
│   │   ├── guide/              # 用户指南
│   │   │   └── YaoXiang-book.md
│   │   ├── tutorial/           # 教程（zh/en）
│   │   ├── blog/               # 博客
│   │   └── dev/                # 开发者文档
│   ├── examples/               # 示例代码
│   └── gh/                     # GitHub 文档（英文 README 等）
└── tests/                      # 测试
```


## 与现有语言的对比

> ⚠️ YaoXiang 处于实验阶段，以下"当前"列反映真实状态，"目标"列是设计方向。

| 维度 | YaoXiang 当前 | YaoXiang 目标 | Rust | Python | Go | Java | TypeScript |
|------|------------|------------|------|--------|----|------|------------|
| 生产就绪 | ❌ 实验阶段 | ✅ 稳定发布 | ✅ | ✅ | ✅ | ✅ | ✅ |
| 类型安全 | 🚧 精化类型 编译期证明 | ✅ 精化类型 编译期证明 | ✅ | ❌ 动态 | ❌ 弱 | ✅ 强 | 🚧 有逃逸 |
| 运行时性能 | 🚧 未测量 | ✅ 接近 Rust | ✅ | ❌ 慢 | ✅ 快 | ✅ 快 | ❌ 慢 |
| 学习曲线 | 🚧 待验证 | ✅ 平滑 | ❌ 陡峭 | ✅ 平滑 | ✅ 平滑 | ✅ 中等 | ✅ 中等 |
| 开发体验 | 🚧 基础 LSP | ✅ 完善 | ✅ rust-analyzer | ✅ 成熟 | ✅ 成熟 | ✅ 成熟 | ✅ 成熟 |
| 包管理/生态 | ❌ 无 | ✅ 统一包管理中心 | ✅ crates.io | ✅ PyPI | ✅ 标准库 | ✅ Maven | ✅ npm |
| 内存管理 | ✅ 所有权模型 | ✅ 低成本所有权 | ✅ 所有权 | ❌ GC | ❌ GC | ❌ GC | ❌ GC |
| 并发编程 | 🚧 设计中 | ✅ 安全并发 | ✅ Send+Sync | ❌ GIL | ✅ goroutine | 🚧 线程模型 | ❌ 单线程 |
| 编译速度 | 🚧 快速检查/new | 💥极其慢（编译期证明）/增量编译优化 | 💥慢/增量编译优化 | ✅ 无编译 | ✅ 快速 | ✅ 快速 | 🚧 tsc慢 |
| 启动速度 | 🚧 解释器初始化+JIT等待 | ✅ 解释器模式/AOT双模式 | ✅ 无预热 | 🚧 解释器初始化+JIT等待 | ✅ 无预热 | ❌ JVM预热 | 🚧 解释器初始化+JIT等待 |
| 泛型/多态 | 🚧 基础实现 | ✅ 完整泛型 | ✅ trait 系统 | ✅ 鸭子类型 | ❌ 无 | ✅ 擦除式 | ✅ 结构化 |

## 核心 RFC

| RFC | 标题 | 描述 |
|-----|------|------|
| [RFC-009](docs/src/design/rfc/accepted/009-ownership-model.md) | 所有权模型 | Move + 借用令牌 + ref——无 GC，无生命周期 |
| [RFC-010](docs/src/design/rfc/accepted/010-unified-type-syntax.md) | 统一类型语法 | 一切皆 `name: type = value` |
| [RFC-011](docs/src/design/rfc/accepted/011-generic-type-system.md) | 泛型系统 | 值依赖类型，零成本抽象 |

---

## 贡献

欢迎贡献！请阅读 [贡献指南](CONTRIBUTING.md)。


## 社区

- GitHub Issues：功能建议、问题报告
- Discussions：讨论交流


## 许可

本项目采用 MIT 许可证，详见 [LICENSE](LICENSE)。


## 致谢

YaoXiang 的设计灵感来自以下项目和语言：

- **Rust** — 所有权模型、零成本抽象
- **Python** — 语法风格、可读性
- **Idris/Agda** — 依赖类型、类型驱动开发
- **TypeScript** — 类型注解、运行时类型
- **MoonBit** — AI 友好设计


## 没错，目前还是个实验性项目

想喷之前，可以先看看这个：

- [爻象设计宣言 WTF 版](docs/src/design/manifesto-wtf.md) — DeepSeek 锐评

> 「道生一，一生二，二生三，三生万物。」
> ——《道德经》
>
> 类型如道，万物皆由此生。


## 🌟 Star History

<div align="center">
<a href="https://www.star-history.com/?type=date&repos=ChenXu233%2FYaoXiang">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/chart?repos=ChenXu233/YaoXiang&type=date&theme=dark&legend=top-left&sealed_token=wdeU56ITEYJrILAq17aZ5ciE-iqMUTIMhwkf3fvcrGbRz5Ejbm8pRO_Ef8EYVh8vrEGjwcPvDatnTcyNTSetcCPA88yg8Eia_OTa9dNHUVCTeIamCziUCE25ckxdpmGdLjKsS8ZZc2HWXvqhWAezVmpPtMLtc5p92_PX1MFCCtqppFmAndlJV-Ml8Q_C" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/chart?repos=ChenXu233/YaoXiang&type=date&legend=top-left&sealed_token=wdeU56ITEYJrILAq17aZ5ciE-iqMUTIMhwkf3fvcrGbRz5Ejbm8pRO_Ef8EYVh8vrEGjwcPvDatnTcyNTSetcCPA88yg8Eia_OTa9dNHUVCTeIamCziUCE25ckxdpmGdLjKsS8ZZc2HWXvqhWAezVmpPtMLtc5p92_PX1MFCCtqppFmAndlJV-Ml8Q_C" />
   <img alt="Star History Chart" src="https://api.star-history.com/chart?repos=ChenXu233/YaoXiang&type=date&legend=top-left&sealed_token=wdeU56ITEYJrILAq17aZ5ciE-iqMUTIMhwkf3fvcrGbRz5Ejbm8pRO_Ef8EYVh8vrEGjwcPvDatnTcyNTSetcCPA88yg8Eia_OTa9dNHUVCTeIamCziUCE25ckxdpmGdLjKsS8ZZc2HWXvqhWAezVmpPtMLtc5p92_PX1MFCCtqppFmAndlJV-Ml8Q_C" />
 </picture>
</a>
</div>

