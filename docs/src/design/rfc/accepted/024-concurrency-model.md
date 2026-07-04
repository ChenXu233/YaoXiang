---
title: "RFC-024：基于 spawn 的并发运行时语义"
status: "已接受（修订版）"
author: "晨煦"
created: "2026-06-05"
updated: "2026-07-04（与 RFC-032 融合修订：spawn 修饰扩展到任意表达式）"
---

# RFC-024：基于 spawn 的并发运行时语义

> **本文档定义 `spawn` 的运行时行为语义**。
> 语法正交性、AST/IR 重构、类型系统扩展见 [RFC-032](./032-spawn-unified-expression.md)。
>
> 两个 RFC 协同定义 `spawn` —— 024 回答"做什么"，032 回答"怎么表示"。

> **参考**:
> - [并发模型规范](/reference/language-spec/concurrency.md)
> - [RFC-008: Runtime 并发模型与调度器脱耦设计](./008-runtime-concurrency-model.md)
> - [RFC-009: 所有权模型设计](./009-ownership-model.md)
> - [RFC-010: 统一类型语法](./010-unified-type-syntax.md)
> - [RFC-032: spawn 统一表达式修饰 — AST/IR 重构](./032-spawn-unified-expression.md)

## 摘要

本文档定义 YaoXiang 编程语言 `spawn` 的**运行时行为语义**：`spawn <expr>` 是唯一的并行原语，可修饰任意表达式，调用方同步阻塞。表达式的形状决定任务拆解粒度，运行时按 GMP 模型调度——无依赖的任务扔进工作队列，worker 抢着跑。

**核心设计——一个原语，一组规则**：

```
spawn <expr>               ← 唯一并行原语
任务拆解由表达式形状决定    ← 唯一规则
同步阻塞等待结果            ← 唯一行为
```

**消除的复杂性**：
- ❌ 无 `@block`/`@eager`/`@auto` 注解
- ❌ 无 `Send`/`Sync` trait
- ❌ 无 `Mutex`/`RwLock`/`Atomic`
- ❌ 无 `future`/非阻塞句柄
- ❌ 无全程序 DAG 分析
- ❌ 无函数着色（async/await）

> **用户心智模型**：你写的普通代码是顺序执行的。当你希望多件事一起做时，把它们放进 `spawn <expr>` 里。没有回调，没有 `await`，没有奇怪的注解。

## 设计来源

| 文档 | 关系 |
|------|------|
| [RFC-001](/design/rfc/deprecated/001-concurrent-model-error-handling.md) | 被本文取代 |
| [RFC-008](./008-runtime-concurrency-model.md) | 运行时架构，与本文正交 |
| [RFC-009](./009-ownership-model.md) | 所有权模型，不变 |
| [RFC-010](./010-unified-type-syntax.md) | 统一类型语法 |
| [RFC-032](./032-spawn-unified-expression.md) | AST/IR 重构，与本文协同定义 spawn |

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

1. **简单**：只有一个并行原语（`spawn`），可修饰任意表达式
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

### 2. spawn 表达式语义

`spawn <expr>` 是 YaoXiang 中**唯一的并行原语**。可修饰任意表达式，表达式的形状决定任务拆解粒度。

#### 2.1 任务创建规则

| 表达式形状 | 任务拆解 | 同步语义 |
|-----------|---------|---------|
| `spawn { a, b, c }` | 直接子表达式 → N 个独立任务 | 等待全部任务完成 |
| `spawn for x in items { body }` | 每次迭代 → 1 个任务 | 等待全部迭代完成 |
| `spawn while cond { body }` | 每轮迭代 → 1 个任务（迭代间条件驱动） | 等待条件为 false |
| `spawn if c { a } else { b }` | 条件 c 顺序求值，选中分支整体 → 1 个任务 | 等待选中分支完成 |
| `spawn call(x)` | 调用本身 → 1 个任务 | 等待调用完成 |
| `spawn expr`（任意表达式） | 表达式本身 → 1 个任务 | 等待表达式完成 |

> **设计动机**：为什么 spawn 能修饰任意表达式？详见 [RFC-032 §核心设计](./032-spawn-unified-expression.md)。
>
> **控制流正交性**：`spawn <expr>`（spawn 在前）与 `<expr> spawn { body }`（spawn 在后）的语义差异，详见 [RFC-032 §控制流正交性](./032-spawn-unified-expression.md)（核心定义）。所有反着写组合（`for ... spawn { }` / `while ... spawn { }` / `if ... spawn { }`）的运行时行为——错误传播、资源类型、嵌套规则——继承本文 §2.4 / §2.5 / §2.6 的规则。

```yaoxiang
// spawn 块：直接子表达式并行
(a, b) = spawn {
    t1 = fetch("url1")   // 直接子表达式 → 并行任务 1
    t2 = fetch("url2")   // 直接子表达式 → 并行任务 2
    return (t1, t2)      // 显式返回元组
}

// spawn for：每次迭代并行
results = spawn for item in items {
    process(item)        // 每次迭代 → 独立任务
}

// spawn while：每轮迭代并行
spawn while has_next() {
    step()               // 每轮迭代 → 独立任务
}

// spawn if：选中分支整体作为任务
result = spawn if cond {
    branch_a()
} else {
    branch_b()
}
```

#### 2.2 作用域隔离

spawn 表达式创建独立的作用域，内部变量不影响外部：

```yaoxiang
x = 10
result = spawn {
    x = 20              // 这是 spawn 表达式内的局部 x
    compute(x)
}
// x 仍然是 10

result = spawn for item in items {
    item = item + 1     // 迭代局部 item，每次迭代独立副本
    process(item)
}
// 外层 item 不受影响
```

**迭代变量**（for 的 `x`）每轮独立副本，迭代结束自动销毁。

#### 2.3 所有权规则

变量进入 spawn 表达式后，外部不能再使用（Move 语义）：

```yaoxiang
data = load_data()
result = spawn {
    process(data)       // data 的所有权移入 spawn 表达式
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

**跨迭代共享**：使用 `ref` 捕获到外层，迭代间共享同一引用。

#### 2.4 错误传播规则

##### `spawn { a, b, c }`（块）

1. 等待所有任务完成（即使某些任务已失败）
2. 传播第一个遇到的错误
3. 使用 `?` 显式标记错误传播点

```yaoxiang
(a, b) = spawn {
    fetch("url1")?,     // 可能失败
    fetch("url2")?      // 可能失败
}
// 如果任一任务失败，整个 spawn 表达式传播第一个错误
```

##### `spawn for x in items { body? }`

- 等全部迭代完成再返回第一个错误
- 失败迭代后剩余迭代**继续执行**（不取消）
- 使用 `?` 显式标记错误传播点

```yaoxiang
results = spawn for item in items {
    process(item)?      // 任一迭代失败 → 等全部完成 → 传播第一个错误
}
```

##### `spawn while cond { body? }`

继承 while 自身的错误语义：

- step 用 `?` 传播错误 → 整个 spawn while 失败，不再进入下一轮
- step 不传播错误（错误被吞）→ 进入下一轮迭代

```yaoxiang
spawn while has_next() {
    item = next()       // 不传播错误时，失败也进入下一轮
    process(item)
}
```

##### `spawn if c { a } else { b }`

- 条件 c **顺序求值**
- c 求值错误 → 整体错误
- 选中分支内错误 → 整体错误

```yaoxiang
result = spawn if cond()? {  // cond 顺序求值，失败 → 整体错误
    fetch_a()?
} else {
    fetch_b()?
}
```

#### 2.5 资源类型规则

编译器追踪资源类型的使用，确保并发安全：

| 资源类型 | 说明 | 编译器行为 |
|----------|------|-----------|
| `FilePath` | 文件系统路径 | 同路径操作自动串行 |
| `HttpUrl` | HTTP 端点 | 同 URL 操作自动串行 |
| `DBUrl` | 数据库连接 | 同连接操作自动串行 |
| `Console` | 标准输出 | 所有 Console 操作自动串行 |

##### `spawn { ... }` 块内

```yaoxiang
// 同一文件的操作自动串行化
(a, b) = spawn {
    read_file("data.txt"),      // 先执行
    write_file("data.txt", x)   // 等待读取完成
}
```

##### `spawn for ... { ... }` 跨迭代同资源

所有迭代对同一资源类型操作时，编译器**自动降级为串行**（spawn 退化为顺序 for，不报错）：

```yaoxiang
// 所有迭代对同一文件路径写入 → 自动降级为串行
results = spawn for item in items {
    write_file("data.txt", item)
}
// 编译器自动串行所有迭代
```

> **设计理由**：spawn 关键字仍表达并行意图；资源冲突时编译器自动降级，比直接拒绝更符合最小惊讶原则。

##### `spawn while ... { ... }` 捕获 `&mut`

**编译期报错**：`spawn while` 不允许捕获 `&mut` 类型的外部变量：

```yaoxiang
iter = make_iter()
spawn while iter.has_next() {       // 编译期报错
    item = iter.next()              // iter 是 &mut，跨迭代共享可变 = 数据竞争
}
```

> **不重新引入 `Sync` trait**：与 RFC-024 "无 Send/Sync" 承诺一致。要求用户改用 `ref` 或非 spawn 写法。

##### `spawn if c { ... } else { ... }` 两分支同资源

**合法无警告**：if 条件互斥，最多一个分支执行，不存在并发冲突：

```yaoxiang
result = spawn if use_cache {
    load_from_cache(key)            // 分支 1：读 cache
} else {
    fetch(key)                      // 分支 2：读 URL
}
```

#### 2.6 嵌套 spawn

spawn 表达式可以嵌套，内层创建**独立的并发域**：

```yaoxiang
(a, b) = spawn {
    x = spawn {
        fetch("url1"),
        fetch("url2")
    },
    y = compute(x)
}
```

**嵌套语义**：
- 内层 spawn 是独立的并发域（独立任务队列、独立错误传播）
- 内层错误独立传播给外层（外层任务等待内层完成时收到错误）
- 内层资源类型规则独立追踪（不与外层联合检查）

```yaoxiang
// spawn for 嵌套 spawn while
results = spawn for x in items {
    inner = spawn while has_more(x) {
        step(x)
    }
    process(inner)
}
```

### 3. 与旧设计的决裂

| 旧设计（RFC-001） | 新设计（RFC-024 + RFC-032） |
|------------------|---------------------------|
| 全程序自动 DAG 分析 | 仅 spawn 表达式内分析 |
| `@block`/`@eager`/`@auto` 注解 | 无注解，依赖驱动 |
| `Send`/`Sync` trait | 无需，所有权 + ref 自动处理 |
| `future`/非阻塞句柄 | 同步阻塞，无回调 |
| `Mutex`/`RwLock`/`Atomic` | `ref` 自动选择 Rc/Arc |
| L1/L2/L3 三层心智模型 | 普通代码顺序，spawn 表达式并行 |
| 函数着色（async/await） | 无函数着色 |
| `spawn` 仅修饰 `{}` 块 | `spawn` 修饰任意表达式（见 RFC-032） |

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
> **当你希望多件事一起做时，把它们放进 `spawn <expr>` 里。**
>
> 表达式的形状决定任务怎么拆：块里的每个直接子表达式并行；for 的每个迭代并行；if 的选中分支作为一个任务。
>
> **整个 spawn 表达式同步阻塞，等待所有任务完成。**
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

// 数据并行：spawn for
results = spawn for item in items {
    process(item)
}
```

---

## 权衡

### 优点

1. **简单**：只有一个并行原语（`spawn`），可修饰任意表达式
2. **显式**：用户明确知道哪里并行，哪里顺序，没有隐式并发
3. **安全**：所有权规则自然延伸，无需 `Send`/`Sync` 等额外类型约束
4. **可控**：没有自动并行行为，避免意外的并发问题
5. **同步**：调用方同步阻塞，代码易于理解和调试
6. **无函数着色**：不存在 async/await 的函数着色问题
7. **编译高效**：DAG 分析仅限 spawn 表达式内，编译时间可控
8. **正交性**：spawn 与任意控制流结构自然组合（详见 RFC-032）

### 缺点

1. **需要显式 spawn**：不能自动并行，用户需要手动标记并行点
2. **spawn 表达式内 DAG 分析**：编译器需要在 spawn 表达式内进行依赖分析
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
| `spawn` 仅修饰 `{}` 块 | 破坏正交性，`spawn for` 成为特例（见 RFC-032） |

---

## 实现策略

### 编译期分析

1. **表达式形状识别**：根据 spawn 后表达式形状决定任务拆解（详见 RFC-032 §DAG 分析）
2. **DAG 构建**：分析 spawn 表达式内的依赖关系
3. **拓扑排序**：确定 spawn 表达式内执行顺序
4. **并行识别**：识别 spawn 表达式内无依赖的子树
5. **逃逸分析**：`ref` → Rc 还是 Arc
6. **资源冲突检测**：检测资源类型的潜在冲突

### 模块组织

spawn 相关代码统一放置在 `frontend/core/spawn/`：

```
frontend/core/spawn/
├── mod.rs           # spawn 模块入口
├── placement.rs     # spawn 出现位置合法性检查
└── analysis.rs      # 任务识别、依赖分析、资源冲突检测
```

> **迁移说明**（2026-06-11）：现有的 `frontend/core/typecheck/passes/spawn_placement.rs` 将迁移至 `frontend/core/spawn/placement.rs`。`typecheck/passes/` 目录下的 `spawn_placement` 模块声明需同步移除。

### 运行时执行

引用 [RFC-008](./008-runtime-concurrency-model.md) 的 Runtime 架构：

- **Embedded Runtime**：无 spawn 支持，即时执行
- **Standard Runtime**：支持 spawn 表达式
- **Full Runtime**：Standard + WorkStealer 负载均衡

### 依赖关系

- RFC-008（Runtime 架构）→ 已完成
- RFC-009（所有权模型）→ 已完成
- RFC-010（统一类型语法）→ 已完成
- RFC-011（泛型系统）→ 已完成
- RFC-032（AST/IR 重构）→ 与本文协同定义 spawn

---

## 设计决策记录

| 决策 | 决定 | 原因 | 日期 |
|------|------|------|------|
| 并行原语 | `spawn <expr>` | 简单、显式、可控 | 2026-06-05 |
| spawn 修饰范围 | 任意表达式 | 语法正交，消除 `spawn for` 特殊化 | 2026-07-04 |
| 任务拆解 | 由表达式形状决定 | 表达力强、规则统一 | 2026-07-04 |
| 执行模型 | 同步阻塞 | 易于理解、调试 | 2026-06-05 |
| DAG 分析范围 | 仅 spawn 表达式内 | 编译高效、行为可控 | 2026-06-05 |
| 共享机制 | `ref` 自动选 Rc/Arc | 简化用户决策 | 2026-06-05 |
| 注解 | 无 | 减少代码噪音 | 2026-06-05 |
| Send/Sync | 删除 | 所有权 + ref 足够 | 2026-06-05 |
| Mutex/RwLock | 删除 | ref 自动处理 | 2026-06-05 |
| future/句柄 | 删除 | 同步阻塞更简单 | 2026-06-05 |
| 函数着色 | 无 | 避免 async/await 问题 | 2026-06-05 |
| 资源类型 | 内置 + 用户自定义 | 自动串行化 | 2026-06-05 |
| `spawn {}` 错误 | 等全部完成，传播第一个错误 | 确定性行为 | 2026-06-05 |
| `spawn for` 错误 | 等全部完成，传播第一个错误 | 与 `spawn {}` 一致 | 2026-07-04 |
| `spawn while` 错误 | 继承 while 错误语义 | while 标准行为 | 2026-07-04 |
| `spawn if` 条件错误 | c 顺序求值，失败 → 整体错误 | 符合直觉 | 2026-07-04 |
| `spawn for` 同资源 | 自动降级为串行 | 安全降级，不粗暴拒绝 | 2026-07-04 |
| `spawn while` 捕获 `&mut` | 编译期报错 | 避免数据竞争，不引入 Sync | 2026-07-04 |
| `spawn if` 同资源 | 合法无警告 | 互斥分支不构成冲突 | 2026-07-04 |
| 嵌套 spawn | 内层独立并发域 | 独立任务队列、错误、资源 | 2026-07-04 |

---

## 参考文献

### YaoXiang 官方文档

- [并发模型规范](/reference/language-spec/concurrency.md)
- [RFC-001 并作模型（已废弃）](/design/rfc/deprecated/001-concurrent-model-error-handling.md)
- [RFC-008 Runtime 并发模型](./008-runtime-concurrency-model.md)
- [RFC-009 所有权模型](./009-ownership-model.md)
- [RFC-010 统一类型语法](./010-unified-type-syntax.md)
- [RFC-011 泛型系统](./011-generic-type-system.md)
- [RFC-032 spawn 统一表达式修饰 — AST/IR 重构](./032-spawn-unified-expression.md)

### 外部参考

- [Rust async book](https://rust-lang.github.io/async-book/)
- [Go concurrency patterns](https://go.dev/blog/pipelines)
- [Erlang concurrency](https://www.erlang.org/doc/getting_concurrency/getting_concurrency.html)
- [Structured concurrency](https://en.wikipedia.org/wiki/Structured_concurrency)

---

## 生命周期与归宿

| 状态 | 位置 | 说明 |
|------|------|------|
| **已接受（修订版）** | `docs/design/rfc/accepted/` | 与 RFC-032 协同定义 spawn（运行时语义） |