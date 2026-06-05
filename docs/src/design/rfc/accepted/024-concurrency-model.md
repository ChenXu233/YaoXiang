---
title: "RFC-024：基于 spawn 块的并发模型"
---

# RFC-024：基于 spawn 块的并发模型

> **状态**: 已接受
> **作者**: 晨煦
> **创建日期**: 2026-06-05
> **最后更新**: 2026-06-05

> **参考**:
> - [并发模型规范](/reference/language-spec/concurrency.md)
> - [RFC-008: Runtime 并发模型与调度器脱耦设计](./008-runtime-concurrency-model.md)
> - [RFC-009: 所有权模型设计](./009-ownership-model.md)
> - [RFC-010: 统一类型语法](./010-unified-type-syntax.md)

## 摘要

本文档定义 YaoXiang 编程语言的新并发模型：以 `spawn {}` 块作为唯一并行原语，依赖驱动执行，调用方同步阻塞。取代旧的基于 `@block`/`@eager`/`@auto` 注解、`Send`/`Sync` trait 和全程序 DAG 分析的并发方案。

**核心设计——一个原语，一个规则**：

```
spawn { ... }        ← 唯一并行原语
直接子表达式创建任务  ← 唯一规则
同步阻塞等待结果      ← 唯一行为
```

**消除的复杂性**：
- ❌ 无 `@block`/`@eager`/`@auto` 注解
- ❌ 无 `Send`/`Sync` trait
- ❌ 无 `Mutex`/`RwLock`/`Atomic`
- ❌ 无 `future`/非阻塞句柄
- ❌ 无全程序 DAG 分析
- ❌ 无函数着色（async/await）

> **用户心智模型**：你写的普通代码是顺序执行的。当你希望多件事一起做时，把它们放进 `spawn { ... }` 块里。没有回调，没有 `await`，没有奇怪的注解。

## 设计来源

| 文档 | 关系 |
|------|------|
| [RFC-001](/design/rfc/deprecated/001-concurrent-model-error-handling.md) | 被本文取代 |
| [RFC-008](./008-runtime-concurrency-model.md) | 运行时架构，与本文正交 |
| [RFC-009](./009-ownership-model.md) | 所有权模型，不变 |
| [RFC-010](./010-unified-type-syntax.md) | 统一类型语法，已更新返回规则 |
| [并发模型规范](/reference/language-spec/concurrency.md) | 本文的正式规范参考 |

## 动机

### 为什么需要这个设计？

当前主流语言的并发模型存在明显缺陷：

| 语言 | 并发模型 | 问题 |
|------|----------|------|
| Rust | async/await + tokio | 异步传染、函数着色、学习曲线陡峭 |
| Go | goroutine | 无类型安全、数据竞争难以检测 |
| Python | asyncio | GIL 限制、函数着色 |
| JavaScript | Promise/async | 回调地狱、函数着色 |

### 旧设计（RFC-001）的问题

RFC-001 提出的三层并发架构（L1/L2/L3）存在以下问题：

| 问题 | 描述 |
|------|------|
| 心智模型复杂 | L1/L2/L3 三层抽象增加了学习负担 |
| 注解冗余 | `@block`/`@eager`/`@auto` 注解使代码变得嘈杂 |
| 分析复杂度高 | 全程序 DAG 分析的编译时间开销大 |
| 类型约束复杂 | `Send`/`Sync` trait 增加了认知负担 |
| 不可控 | 自动并发行为难以预测和调试 |

### 设计目标

1. **简单**：只有一个并行原语（`spawn`），一条规则（直接子表达式创建任务）
2. **显式**：用户明确知道哪里并行，哪里顺序
3. **安全**：所有权规则自然延伸，无需额外类型约束
4. **可控**：没有隐式并发，没有意外的并行行为
5. **同步**：调用方同步阻塞，没有回调和 `await`

---

## 提案

### 1. {} 块的本质：依赖驱动的计算单元

在 YaoXiang 中，`{}` 是一个**依赖驱动的计算单元**。

| 属性 | 说明 |
|------|------|
| 依赖驱动 | 块在执行时检查内部所有变量是否就绪，若齐备则立即执行，否则阻塞等待 |
| 执行时机 | 由依赖决定，与"立即"或"延迟"无关 |
| 返回值 | 使用 `return` 显式返回值；无 `return` 时默认返回 `Void` |
| 语法统一 | 无论出现在函数体、变量初始化还是 `spawn` 后，语义一致 |
| 作用域隔离 | 变量严格限于 `{}` 内部，不穿透到外层作用域 |

```yaoxiang
// 依赖驱动示例
x = compute_x()        // x 就绪
y = compute_y()        // y 就绪
result = {
    // 依赖 x 和 y，两者就绪后立即执行
    return x + y
}
```

### 2. spawn 块语义

`spawn { ... }` 是 YaoXiang 中**唯一的并行原语**。

#### 2.1 核心规则

- spawn 块的**直接子表达式**创建并行任务
- 嵌套 `{}` 内的表达式不算作独立任务
- spawn 块遵循标准返回规则：必须用 `return` 显式返回值，无 `return` 时返回 `Void`
- 整个 spawn 块同步阻塞，等待所有任务完成后返回
- 没有回调、`await` 或注解

```yaoxiang
// 两个任务并行执行
(a, b) = spawn {
    t1 = fetch("url1")   // 直接子表达式 → 并行任务 1
    t2 = fetch("url2")   // 直接子表达式 → 并行任务 2
    return (t1, t2)      // 显式返回元组
}

// 嵌套 {} 内的不是直接子表达式
result = spawn {
    x = {               // 这整个块是一个直接子表达式 → 一个任务
        inner_work()    // 不是 spawn 的直接子表达式，不会成为独立任务
    },
    process(x)          // 直接子表达式 → 并行任务
    return process(x)
}

#### 2.2 作用域隔离

spawn 块创建独立的作用域，内部变量不影响外部：

```yaoxiang
x = 10
result = spawn {
    x = 20              // 这是 spawn 块内的局部 x
    compute(x)
}
// x 仍然是 10
```

#### 2.3 所有权规则

变量进入 spawn 块后，外部不能再使用（Move 语义）：

```yaoxiang
data = load_data()
result = spawn {
    process(data)       // data 的所有权移入 spawn 块
}
// data 在此处不可用（已 move）
```

如果需要在多个任务间共享，使用 `ref`：

```yaoxiang
data = load_data()
shared = ref data       // 编译器自动选择 Rc 或 Arc

result = spawn {
    process_a(shared),  // 共享引用
    process_b(shared)   // 共享引用
}
```

#### 2.4 错误处理

spawn 块内的错误传播遵循以下规则：

1. 等待所有任务完成（即使某些任务已失败）
2. 传播第一个遇到的错误
3. 使用 `?` 显式标记错误传播点

```yaoxiang
(a, b) = spawn {
    fetch("url1")?,     // 可能失败
    fetch("url2")?      // 可能失败
}
// 如果任一任务失败，整个 spawn 块传播第一个错误
```

#### 2.5 资源类型

编译器追踪资源类型的使用，确保并发安全：

| 资源类型 | 说明 | 编译器行为 |
|----------|------|-----------|
| `FilePath` | 文件系统路径 | 同路径操作自动串行 |
| `HttpUrl` | HTTP 端点 | 同 URL 操作自动串行 |
| `DBUrl` | 数据库连接 | 同连接操作自动串行 |
| `Console` | 标准输出 | 所有 Console 操作自动串行 |

```yaoxiang
// 同一文件的操作自动串行化
(a, b) = spawn {
    read_file("data.txt"),      // 先执行
    write_file("data.txt", x)   // 等待读取完成
}
```

#### 2.6 spawn for：数据并行循环

```yaoxiang
// 并行处理列表中的每个元素
results = spawn for item in items {
    result = process(item)
}
```

#### 2.7 嵌套 spawn

spawn 块可以嵌套，内层 spawn 创建新的并发域：

```yaoxiang
(a, b) = spawn {
    x = spawn {
        fetch("url1"),
        fetch("url2")
    },
    y = compute(x)
}
```

### 3. 与旧设计的决裂

| 旧设计（RFC-001） | 新设计（RFC-024） |
|------------------|------------------|
| 全程序自动 DAG 分析 | 仅 spawn 块内分析 |
| `@block`/`@eager`/`@auto` 注解 | 无注解，依赖驱动 |
| `Send`/`Sync` trait | 无需，所有权 + ref 自动处理 |
| `future`/非阻塞句柄 | 同步阻塞，无回调 |
| `Mutex`/`RwLock`/`Atomic` | `ref` 自动选择 Rc/Arc |
| L1/L2/L3 三层心智模型 | 普通代码顺序，spawn 块并行 |
| 函数着色（async/await） | 无函数着色 |

### 4. 返回规则

YaoXiang 的返回规则统一且明确：

| 写法 | 返回值 | 说明 |
|------|--------|------|
| `= expr`（无花括号） | 直接返回 `expr` | 表达式即值 |
| `= { ... }`（有花括号） | 必须用 `return`，否则返回 `Void` | 块需要显式返回 |

```yaoxiang
// 无花括号：直接返回
add: (a: Int, b: Int) -> Int = a + b

// 有花括号：必须用 return
process: (data: Data) -> Result = {
    validated = validate(data)?
    return ok(transform(validated))
}

// 有花括号但无 return：返回 Void
log: (message: String) -> Void = {
    print(message)  // 无 return，返回 Void
}
```

### 5. 用户心智模型

> **你写的普通代码是顺序执行的。**
>
> **当你希望多件事一起做时，把它们放进 `spawn { ... }` 块里。**
>
> 块里的每个直接子表达式都会立即开始（并行），用 `return` 显式返回结果。
> 整个块会等所有事做完，然后把最终结果给你。
>
> **没有回调，没有 `await`，没有奇怪的注解。**

```yaoxiang
// 普通代码：顺序执行
a = compute_a()         // 先执行
b = compute_b(a)        // 依赖 a，a 完成后执行
c = compute_c(b)        // 依赖 b，b 完成后执行

// 需要并行时：使用 spawn
(x, y, z) = spawn {
    fetch("url1"),      // 并行
    fetch("url2"),      // 并行
    fetch("url3")       // 并行
}
// 等待所有完成后继续
process(x, y, z)
```

---

## 权衡

### 优点

1. **简单**：只有一个并行原语（`spawn`），一条规则（直接子表达式创建任务）
2. **显式**：用户明确知道哪里并行，哪里顺序，没有隐式并发
3. **安全**：所有权规则自然延伸，无需 `Send`/`Sync` 等额外类型约束
4. **可控**：没有自动并行行为，避免意外的并发问题
5. **同步**：调用方同步阻塞，代码易于理解和调试
6. **无函数着色**：不存在 async/await 的函数着色问题
7. **编译高效**：DAG 分析仅限 spawn 块内，编译时间可控

### 缺点

1. **需要显式 spawn**：不能自动并行，用户需要手动标记并行点
2. **spawn 块内 DAG 分析**：编译器需要在 spawn 块内进行依赖分析
3. **与旧代码不兼容**：使用旧 RFC-001 模式的代码需要迁移

---

## 替代方案

| 方案 | 为什么不选择 |
|------|--------------|
| 全程序自动 DAG（RFC-001） | 复杂度高，编译时间长，行为不可控 |
| async/await | 函数着色，学习曲线陡峭，代码可读性差 |
| goroutine | 无类型安全，数据竞争难以检测 |
| Actor 模型 | 消息传递复杂，调试困难 |
| CSP（Go channel） | 无类型安全，死锁难以检测 |

---

## 实现策略

### 编译期分析

1. **DAG 构建**：分析 spawn 块内的依赖关系
2. **拓扑排序**：确定 spawn 块内执行顺序
3. **并行识别**：识别 spawn 块内无依赖的子树
4. **逃逸分析**：`ref` → Rc 还是 Arc
5. **资源冲突检测**：检测资源类型的潜在冲突

### 运行时执行

引用 [RFC-008](./008-runtime-concurrency-model.md) 的 Runtime 架构：

- **Embedded Runtime**：无 spawn 支持，即时执行
- **Standard Runtime**：支持 spawn 块，spawn 块内并发
- **Full Runtime**：Standard + WorkStealer 负载均衡

### 依赖关系

- RFC-008（Runtime 架构）→ 已完成
- RFC-009（所有权模型）→ 已完成
- RFC-010（统一类型语法）→ 已完成
- RFC-011（泛型系统）→ 已完成

---

## 设计决策记录

| 决策 | 决定 | 原因 | 日期 |
|------|------|------|------|
| 并行原语 | `spawn {}` 块 | 简单、显式、可控 | 2026-06-05 |
| 任务创建 | 直接子表达式 | 明确、无歧义 | 2026-06-05 |
| 执行模型 | 同步阻塞 | 易于理解、调试 | 2026-06-05 |
| DAG 分析范围 | 仅 spawn 块内 | 编译高效、行为可控 | 2026-06-05 |
| 共享机制 | `ref` 自动选 Rc/Arc | 简化用户决策 | 2026-06-05 |
| 注解 | 无 | 减少代码噪音 | 2026-06-05 |
| Send/Sync | 删除 | 所有权 + ref 足够 | 2026-06-05 |
| Mutex/RwLock | 删除 | ref 自动处理 | 2026-06-05 |
| future/句柄 | 删除 | 同步阻塞更简单 | 2026-06-05 |
| 函数着色 | 无 | 避免 async/await 问题 | 2026-06-05 |
| 错误传播 | 等待所有任务，传播第一个错误 | 确定性行为 | 2026-06-05 |
| 资源类型 | 内置 + 用户自定义 | 自动串行化 | 2026-06-05 |

---

## 参考文献

### YaoXiang 官方文档

- [并发模型规范](/reference/language-spec/concurrency.md)
- [RFC-001 并作模型（已废弃）](/design/rfc/deprecated/001-concurrent-model-error-handling.md)
- [RFC-008 Runtime 并发模型](./008-runtime-concurrency-model.md)
- [RFC-009 所有权模型](./009-ownership-model.md)
- [RFC-010 统一类型语法](./010-unified-type-syntax.md)
- [RFC-011 泛型系统](./011-generic-type-system.md)

### 外部参考

- [Rust async book](https://rust-lang.github.io/async-book/)
- [Go concurrency patterns](https://go.dev/blog/pipelines)
- [Erlang concurrency](https://www.erlang.org/doc/getting_concurrency/getting_concurrency.html)
- [Structured concurrency](https://en.wikipedia.org/wiki/Structured_concurrency)

---

## 生命周期与归宿

| 状态 | 位置 | 说明 |
|------|------|------|
| **已接受** | `docs/design/rfc/accepted/` | 正式设计文档 |
