# YaoXiang（爻象）编程语言

> 一门实验性的通用编程语言，融合类型论、所有权模型和自然语法的力量。
>
> 基于《并作模型：万物并作，吾以观复》

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Version](https://img.shields.io/badge/Version-v0.7.0--experimental-blue.svg)]()

> 🌐 **Language** | [English](docs/gh/README.en.md)

---

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

---

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

---

## 所有权模型

YaoXiang 采用五级所有权梯度——无 GC，无生命周期标注：

```
&T / &mut T       Move            ref            clone()         unsafe
    |                |               |               |               |
借用令牌          默认            共享持有         深拷贝          裸指针
零成本            零拷贝          自动Rc/Arc      显式调用        系统级
```

| 操作 | 成本 | 使用场景 |
|------|------|----------|
| `&T` / `&mut T` | 零（编译期令牌，编译后消失） | 只读访问 / 独占可变访问 |
| Move | 零（指针移动） | 默认——赋值、传参、返回 |
| `ref` | 低（Rc）/ 中（Arc） | 跨作用域共享持有 |
| `clone()` | 视类型而定 | 需要独立副本 |
| `unsafe` + `*T` | 零（直接内存操作） | FFI、系统级编程 |

**核心设计决策：**
- **无生命周期标注**（`'a`）——令牌是值，由 RAII 统一管理生命周期
- **无借用检查器**——类型属性（Dup/Linear）自然推导权限
- **无 GC**——确定性资源管理
- **编译器自动选择 Rc/Arc**——`ref` 不跨任务用 Rc，跨任务用 Arc

详见 [RFC-009：所有权模型设计](docs/src/design/rfc/accepted/009-ownership-model.md)。

---

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

---

## 安装与构建

```bash
# 克隆并构建（开发版本）
git clone https://github.com/yaoxiang-lang/yaoxiang.git
cd yaoxiang
cargo build

# 运行测试查看当前状态
cargo test

# 尝试示例
cargo run --example hello
```

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

---

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

---

## 设计理念

YaoXiang 的设计哲学可以用五句话概括：

```
一切皆类型 → 统一抽象 → 类型即数据 → 运行时可用
所有权模型 → 零成本抽象 → 无GC → 高性能
Python语法 → 自然语言感 → 可读性 → 新手友好
并作模型 → 惰性求值 → 自动并行 → 无感并发
类型安全 → 编译时检查 → 数据竞争 → 线程安全
```

---

## 与现有语言的对比

| 特性 | YaoXiang | Rust | Python | TypeScript | Go |
|------|----------|------|--------|------------|-----|
| 一切皆类型 | ✅ | ❌ | ❌ | ❌ | ❌ |
| 自动类型推断 | ✅ | ✅ | ✅ | ✅ | ❌ |
| 默认不可变 | ✅ | ✅ | ❌ | ❌ | ❌ |
| 所有权模型 | ✅ | ✅ | ❌ | ❌ | ❌ |
| 并作模型 | ✅ | ❌ | ❌ | ❌ | ⚠️ |
| 零成本抽象 | ✅ | ✅ | ❌ | ❌ | ❌ |
| 无GC | ✅ | ✅ | ❌ | ❌ | ✅ |
| 编译时线程安全 | ✅ | ✅ | ❌ | ❌ | ❌ |
| 值依赖类型 | ✅ | ❌ | ❌ | ❌ | ❌ |
| 关键字数量 | 17 | 51+ | 35 | 64+ | 25 |

> **并作模型** = 同步语法 + 惰性求值 + 自动并行 + 无感异步

---

## 核心 RFC

| RFC | 标题 | 描述 |
|-----|------|------|
| [RFC-009](docs/src/design/rfc/accepted/009-ownership-model.md) | 所有权模型 | Move + 借用令牌 + ref——无 GC，无生命周期 |
| [RFC-010](docs/src/design/rfc/accepted/010-unified-type-syntax.md) | 统一类型语法 | 一切皆 `name: type = value` |
| [RFC-011](docs/src/design/rfc/accepted/011-generic-type-system.md) | 泛型系统 | 值依赖类型，零成本抽象 |

---

## 路线图

详细实现状态和未来计划，请查看 [实现路线图](docs/plan/IMPLEMENTATION-ROADMAP.md)。

---

## 贡献

欢迎贡献！请阅读 [贡献指南](CONTRIBUTING.md)。

---

## 社区

- GitHub Issues：功能建议、问题报告
- Discussions：讨论交流

---

## 许可

本项目采用 MIT 许可证，详见 [LICENSE](LICENSE)。

---

## 致谢

YaoXiang 的设计灵感来自以下项目和语言：

- **Rust** — 所有权模型、零成本抽象
- **Python** — 语法风格、可读性
- **Idris/Agda** — 依赖类型、类型驱动开发
- **TypeScript** — 类型注解、运行时类型
- **MoonBit** — AI 友好设计

---

## 没错，目前还是个实验性项目

想喷之前，可以先看看这个：

- [爻象设计宣言 WTF 版](docs/src/design/manifesto-wtf.md) — DeepSeek 锐评

> 「道生一，一生二，二生三，三生万物。」
> ——《道德经》
>
> 类型如道，万物皆由此生。

---

> 🌐 **English version**: [README.en.md](docs/gh/README.en.md)
