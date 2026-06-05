---
title: "RFC-025: 可扩展原语类型机制"
status: "草案"
author: "晨煦"
created: "2026-06-05"
updated: "2026-06-05"
---

# RFC-025: 可扩展原语类型机制

## 摘要

本文档定义 YaoXiang 编译器的**可扩展原语类型机制**（`Primitive::Extension`）。允许外部代码向编译器注册自定义原语类型，使编译器无需硬编码即可支持领域特定类型（量子位、GPU 缓冲区、SIMD 向量、硬件寄存器等）。

## 动机

### 为什么需要这个机制？

当前编译器硬编码了所有原语类型：`Int`、`Float`、`String`、`Bool`、`Unit`。每添加一种新的原语类型，都需要修改编译器源码的多个位置——类型检查器、代码生成器、所有权分析器、优化器。

这违反了开闭原则：对扩展开放，对修改关闭。

### 设计边界

```
硬编码核心类型（语言地基）：Int, Float, String, Bool, Unit
动态扩展类型（领域插件）：  通过 Primitive::Extension 注册
```

核心类型硬编码是因为编译器深度依赖它们的语义（算术运算、条件分支、哈希、比较）。扩展类型对编译器是**不透明值**——编译器只知道它们的大小、对齐和所有权属性，不知道它们的内部语义。

这不是"把所有类型统一成动态加载"。核心类型和扩展类型是两件不同的事。

## 提案

### 核心设计

#### 1. Extension 类型属性

每个扩展原语类型注册时必须声明以下属性：

```rust
pub struct PrimitiveExtension {
    /// 类型名称，如 "Qubit", "Buffer", "Vec128"
    pub name: String,

    /// 字节大小（固定大小类型）
    /// None 表示大小在编译期未知（需运行时确定）
    pub size: Option<usize>,

    /// 对齐要求
    pub align: Option<usize>,

    /// 是否允许隐式复制
    /// false = Move 语义（赋值即移动，如 Qubit）
    /// true = Copy 语义（赋值即复制，如 Vec128）
    pub is_copy: bool,

    /// 是否允许空值（zero-sized type）
    pub allow_zst: bool,
}
```

#### 2. 注册接口

```rust
// 编译器内部 API
compiler.register_primitive(PrimitiveExtension {
    name: "Qubit".to_string(),
    size: Some(0),           // 逻辑大小为0，物理状态在量子处理器上
    align: Some(1),
    is_copy: false,          // Move 语义，符合 no-cloning
    allow_zst: true,
});
```

注册后，`Qubit` 在类型系统中成为合法的原语类型，可用于变量声明、函数参数、结构体字段。

#### 3. 类型检查器行为

扩展原语类型在类型检查中遵循以下规则：

| 场景 | 行为 |
|------|------|
| 变量声明 `q: Qubit = ...` | ✅ 合法 |
| 函数参数 `fn(q: Qubit)` | ✅ 合法 |
| 结构体字段 `{ q: Qubit }` | ✅ 合法 |
| `is_copy == false` 时赋值 | Move 语义，原变量失效 |
| `is_copy == true` 时赋值 | Copy 语义，原变量保留 |
| 隐式复制（函数多处使用） | 取决于 `is_copy` |
| 比较 `==`、`!=` | ❌ 编译错误（无内置比较） |
| 算术 `+`、`-` | ❌ 编译错误（无内置运算） |
| 泛型约束 `T: Copy` | 仅当 `is_copy == true` 时满足 |

#### 4. 代码生成器行为

扩展原语类型在代码生成中作为**不透明值**处理：

- LLVM IR：生成为 `{size} x i8` 或对应大小的结构体
- 不生成任何特殊指令——语义由后端或库负责
- 如果后端需要特殊处理（如 QIR 量子门），通过后端注册机制实现（不在本 RFC 范围内）

### 示例

#### 注册一个 Move 语义类型

```rust
// 量子位：不可复制，大小为0（物理状态在 QPU 上）
compiler.register_primitive(PrimitiveExtension {
    name: "Qubit".into(),
    size: Some(0),
    align: Some(1),
    is_copy: false,
    allow_zst: true,
});
```

```yaoxiang
# 用户代码
q: Qubit = qubit(0)
q2 = q          # ❌ 编译错误：Qubit 是 Move 类型，q 已失效
q = H(q)        # ✅ 消费 q，返回新 q
```

#### 注册一个 Copy 语义类型

```rust
// SIMD 向量：可复制，固定大小
compiler.register_primitive(PrimitiveExtension {
    name: "Vec128".into(),
    size: Some(16),
    align: Some(16),
    is_copy: true,
    allow_zst: false,
});
```

```yaoxiang
# 用户代码
a: Vec128 = load_vec128(data)
b = a           # ✅ Copy 语义，a 仍然有效
c = add_vec128(a, b)  # ✅ a 和 b 都可用
```

## 详细设计

### 编译器改动

| 组件 | 改动 |
|------|------|
| 类型系统 | 新增 `Ty::Extension` 变体，存储 `PrimitiveExtension` 元数据 |
| 类型检查器 | 扩展类型不参与内置运算解析，不满足内置 trait 约束（除非显式实现） |
| 所有权分析器 | 根据 `is_copy` 决定 Move 或 Copy 语义 |
| 代码生成器 | 按 `size`/`align` 生成不透明值，无特殊指令 |
| 错误信息 | 扩展类型的错误消息引用注册时的 `name` |

### 与 FFI 的关系

`Primitive::Extension` 与 RFC-021（FFI）是正交的：

| | Primitive::Extension | FFI |
|---|---|---|
| 作用 | 注册新**类型** | 调用外部**函数** |
| 层级 | 类型系统 | 运行时 |
| 示例 | `Qubit` 是一个类型 | `native("sin")` 是一个函数调用 |

一个领域可能同时需要两者：`Qubit` 类型通过 Extension 注册，量子门函数通过 FFI 注册。

### 向后兼容性

- ✅ 完全向后兼容
- 不修改任何现有类型的语义
- 扩展类型是新增能力，不影响已有代码

## 权衡

### 优点

- ✅ 编译器无需为每个新领域修改源码
- ✅ 领域专家可以自行注册类型，不依赖编译器团队
- ✅ 核心类型保持硬编码，不牺牲编译器对基础类型的深度优化
- ✅ 接口简单，一个结构体定义所有属性

### 缺点

- ⚠️ 扩展类型不支持内置运算——需要额外的函数或后端机制来实现语义
- ⚠️ 调试时扩展类型显示为不透明值，不如核心类型直观

## 替代方案

| 方案 | 为什么不选择 |
|------|--------------|
| 所有类型动态加载 | 核心类型（Int/Float/Bool）需要编译器深度优化，动态加载会丧失这些能力 |
| 每个领域硬编码 | 每加一个领域就改编译器，不可扩展 |
| 纯库方案（不注册类型） | 无法在类型系统层面保证语义（如 no-cloning），只能运行时检查 |

## 实现策略

### 阶段 1：核心接口

- [ ] 在类型系统中添加 `Ty::Extension` 变体
- [ ] 实现 `register_primitive` API
- [ ] 扩展类型检查器处理 `is_copy` 语义
- [ ] 扩展代码生成器处理不透明值
- [ ] 单元测试

### 阶段 2：注册时机

- [ ] 支持编译器初始化时批量注册（配置文件或 builder API）
- [ ] 支持标准库预注册（`std.primitive` 模块导出扩展类型定义）

### 依赖关系

- 无硬依赖。可独立于其他 RFC 实现。

## 开放问题

- [ ] 扩展类型是否应该支持 trait 实现（如让 `Qubit` 实现自定义的 `QuantumGate` trait）？
- [ ] 是否需要生命周期钩子（如 `on_drop`）来支持 RAII 语义的扩展类型？
- [ ] 配置文件格式：TOML、YAML、还是 YaoXiang 自身的配置语法（参考 RFC-015）？

---

## 设计决策记录

| 决策 | 决定 | 原因 | 日期 |
|------|------|------|------|
| 核心类型不动态加载 | 保持硬编码 | 编译器深度依赖核心类型语义，动态化收益为零 | 2026-06-05 |
| 扩展类型为不透明值 | 不注入语义 | 语义由后端/库负责，编译器只保证类型安全和所有权 | 2026-06-05 |
| 与 FFI 正交 | 不合并 | 类型注册和函数调用是不同抽象层级 | 2026-06-05 |
