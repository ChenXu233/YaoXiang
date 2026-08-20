---
title: 'RFC-032: spawn 统一表达式修饰 — 消除 spawn for 特殊情况'
status: '审核中'
author: '晨煦'
created: '2026-06-16'
updated: '2026-08-19'
issue: '#98'
---

# RFC-032: spawn 统一表达式修饰

> **本文档定义 `spawn`
> 的语法、AST/IR 重构**。运行时行为语义（任务拆解粒度、所有权、作用域、错误传播、资源类型、嵌套）见
> [RFC-024: 基于 spawn 的并发运行时语义](./024-concurrency-model.md)。
>
> 两个 RFC 协同定义 `spawn` —— 024 回答"做什么"，032 回答"怎么表示"。

> **核心洞察**：`spawn` 不应该只修饰 `{}` 块。它可以修饰**任意表达式**。`spawn for`
> 不是特殊语法——它就是 `spawn` + `for` 表达式的自然组合。

## 摘要

将 `spawn` 从 `spawn { }`（仅修饰块）扩展为 `spawn <expr>`（修饰任意表达式）。`Expr::SpawnFor`
从 AST 中删除，由 `Expr::Spawn { body: Expr::For { .. } }`
自然替代。本 RFC 仅做 AST/IR/Parser 清理，不涉及类型系统变更。

> **计算结构类型（`MonoType` 扩展）推迟到独立 RFC。** 本 RFC 删除 `SpawnFor` 特殊情况后，
> `spawn` 的证明管道集成需要类型系统感知计算结构——这是通用机制，不限于 spawn，值得独立设计。

## 动机

### 为什么需要这个变更？

当前 `spawn for x in items { body }` 是独立的关键词组合，AST 中有 `Expr::SpawnFor`
专门表示它。这破坏了语言的正交性：

1. **语法不统一**：`spawn` 只能修饰 `{}` 块，`spawn for` 是硬编码的例外
2. **正交性缺失**：`spawn while`、`spawn if` 等组合无法自然表达

### 当前的问题

```rust
// AST 中两个 spawn 变体
Spawn { body: Box<Block>, span: Span },         // spawn { ... }
SpawnFor { var, var_mut, iterable, body, span },  // spawn for x in items { ... }
```

## 提案

### 核心设计

`spawn <expr>`：`spawn` 修饰任意表达式。表达式的形状决定 DAG 如何分解任务。



### 用户心智模型

`spawn` = "把这个表达式拿去做并发"。表达式的形状决定怎么拆：

| 表达式形状                      | 并发行为                  |
| ------------------------------- | ------------------------- |
| `spawn { a, b, c }`             | `a`、`b`、`c` 独立并行    |
| `spawn for x in items { f(x) }` | N 个迭代独立并行          |
| `spawn while cond { step() }`   | 每轮迭代独立任务          |
| `spawn if c { a } else { b }`   | 被选中分支整体为 spawn 域 |
| `spawn call(x)`                 | 调用本身作为一个任务      |
| `spawn 42`                      | 单独一个任务              |

编译器负责 DAG 分析确定依赖关系，运行时按 GMP 模型调度——无依赖的任务扔进工作队列，worker 抢着跑。整体同步阻塞，等待所有任务完成。

**与 Go 的区别**：Go 的 `go` 是"扔出去不管"，YaoXiang 的 `spawn`
是"拆开并行执行，等全部干完再往下"。

### 控制流正交性

| 组合                            | 语义                          | 差异                           |
| ------------------------------- | ----------------------------- | ------------------------------ |
| `spawn for x in items { body }` | 数据并行：每次迭代 = 独立任务 | DAG 跨迭代分析依赖             |
| `for x in items spawn { body }` | 每次迭代创建一个 spawn 域     | 不跨迭代分析                   |
| `spawn while cond { body }`     | 条件并行：每次迭代 = 独立任务 | 迭代间依赖由条件保证           |
| `while cond spawn { body }`     | 每次迭代创建一个 spawn 域     | 与上面语义不同但不需要特殊处理 |
| `spawn if c { a } else { b }`   | 整个 if-else 为一个 spawn 域  | 执行时按条件选分支             |
| `if c spawn { a } else { b }`   | 仅单分支 spawn                | if 表达式内部包 spawn          |

### 消除的复杂度

- ❌ `Expr::SpawnFor` 从 AST 中删除
- ❌ `SpawnForAnalysis` 从 DAG 分析中删除
- ❌ `spawn for` 不再作为组合关键词在 Parser 中特殊处理
- ❌ `Ir::SpawnFor` 从 IR 中删除

## 详细设计

### 1. AST 层

**之前：**

```rust
Spawn { body: Box<Block>, span: Span },         // spawn { ... }
SpawnFor { var, var_mut, iterable, body, span },  // spawn for x in items { ... }
```

**之后：**

```rust
Spawn { body: Box<Expr>, span: Span },           // spawn <任意表达式>
```

`Expr::SpawnFor` 删除。`spawn for x in items { body }` 的 AST 表示：

```rust
Expr::Spawn {
    body: Box::new(Expr::For {
        var: "x",
        iterable: items,
        body: body_block,
        ..
    })
}
```

**IF 特殊情况**：

| 写法                             | AST 结构                                            |
| -------------------------------- | --------------------------------------------------- |
| `spawn if cond { a } else { b }` | `Spawn { body: Expr::If { ... } }`                  |
| `if cond spawn { a } else { b }` | `Expr::If { then: Spawn { body: {a} }, else: {b} }` |

两者语义不同但都是自然组合，不需要特殊规则。

### 2. Parser 层

`spawn` 绑定优先级最低（等同 `return`），吃掉后续整个表达式：

```
spawn a + b        →  spawn (a + b)         ≠  (spawn a) + b
spawn f(x).y       →  spawn (f(x).y)
```

Parser 改动：`pratt/nud.rs` 中 `spawn` 不再要求 `{`，而是调用通用表达式解析：

```
token spawn → parse_expr(min_precedence) → Expr::Spawn { body: expr }
```

`spawn for` 不再作为组合关键词处理——`for` 由通用表达式解析器处理产生 `Expr::For`，`spawn`
只负责包装。

### 3. DAG 分析层

当前两个入口合并为一个：

```rust
/// 统一入口：根据 body 表达式种类分发
fn analyze_spawn_expr(body: &Expr, ...) -> SpawnAnalysis {
    match body {
        Expr::Block(block)       => analyze_block_tasks(block, ...),
        Expr::For { .. }         => analyze_iter_tasks(IterKind::For, body, ...),
        Expr::While { .. }       => analyze_iter_tasks(IterKind::While, body, ...),
        Expr::If { .. }          => analyze_if_task(body, ...),
        _                        => single_task(body, ...),
    }
}
```

**统一结果结构**：

```rust
struct SpawnAnalysis {
    source: TaskSource,
    plan: ExecutionPlan,
}

enum TaskSource {
    /// spawn { a, b, c } — 编译期已知的 N 个直接子表达式
    Explicit(Vec<TaskInfo>),
    /// spawn for/while — N 个任务由运行时迭代产生
    Iterate {
        kind: IterKind,
        iter_var: String,
        iterable: Option<Expr>,      // for 有，while 无
        condition: Option<Expr>,     // while 有，for 无
        body: Block,
        reads: HashSet<String>,
        writes: HashSet<String>,
        resource_vars: HashSet<String>,
    },
}

enum IterKind { For, While }
```

`SpawnForAnalysis` 结构体删除。

| body 种类           | 如何分解为任务                  |
| ------------------- | ------------------------------- |
| `Expr::Block`       | 直接子表达式 → 任务列表         |
| `Expr::For`         | 每次迭代 → 一个任务（数据并行） |
| `Expr::While`       | 每次迭代 → 一个任务             |
| `Expr::If`          | 被选中分支整体 → 一个任务       |
| `Expr::Call` / 其他 | 表达式本身 → 一个任务           |

DAG 分析完成后，运行时按 GMP 模型调度——无依赖的任务扔进工作队列，worker 抢着跑。

### 4. IR / Codegen 层

`Ir::SpawnFor` 删除。统一为 `Ir::Spawn`，携带 `TaskSource` 信息。

HIR → IR 翻译根据 `SpawnAnalysis.source` 生成运行时调用：

- `TaskSource::Explicit(tasks)` → 编译期已知任务列表
- `TaskSource::Iterate { .. }` → 运行时展开（编译器驱动，类似 par_iter 但零成本）

### 5. Placement 层

当前两个分支合并为一个：

```rust
// 之前
Expr::Spawn { body, .. } => self.check_block(body),
Expr::SpawnFor { body, iterable, .. } => {
    self.check_expr(iterable);
    self.check_block(body);
}

// 之后
Expr::Spawn { body, .. } => self.check_expr(body),   // body 是 Expr，递归即可
```

### 6. 向后兼容性

已有 `spawn for` 代码语义不变，Parser 自动将 `spawn for x in items { body }` 解析为
`Expr::Spawn { body: Expr::For }`。内部表示变化，用户可见行为不变。

新语法自然获得：

```yx
spawn while has_next() {
    item = next()
    process(item)
}

spawn if use_cache {
    load_from_cache(key)
} else {
    fetch(key)
}
```

**单任务 spawn 警告**：`spawn call(x)` 和 `spawn 42` 等修饰单个表达式时，DAG 分析产生编译警告：
"spawn 修饰单个表达式没有并发效果"。语法合法，但提醒用户检查意图。

## 权衡

### 优点

1. **语法正交**：`spawn` + 任意控制流 = 自然并发组合
2. **消除特殊情况**：删除 `Expr::SpawnFor` 及相关特殊处理代码
3. **可扩展**：未来新增控制流结构自动与 `spawn` 组合，无需修改 spawn 逻辑

### 缺点

1. **破坏性变更**：内部 AST/IR 表示变化，需更新所有消费 `Expr::SpawnFor` 的代码
2. **证明管道需适配**：删除 `SpawnFor` 后，证明管道通过 AST 分发（`match body { Expr::For => ..., Expr::While => ... }`）——该适配在本 RFC 范围内通过 DAG 统一入口完成

## 替代方案

| 方案                                             | 为什么不选择                                                  |
| ------------------------------------------------ | ------------------------------------------------------------- |
| 保持 `spawn for` 独立语法                        | 破坏正交性，成为语言中唯一的关键词组合特例                    |
| `spawn` 仅修饰 `{}`，数据并行走标准库 `par_iter` | 语言原始能力下沉到库，失去编译器层面的 DAG 分析和资源冲突检测 |

## 计算结构类型（推迟到独立 RFC）

本 RFC 删除 `SpawnFor` 后，`spawn` 的证明管道集成面临一个架构问题：证明管道工作在类型层，需要知道 spawn 内部的计算结构（For/While/Block/If/Call）才能选择正确的证明策略。当前证明管道通过 AST 分发，但长期方向是将计算结构编码为 `MonoType` 变体（`Block`/`ForExpr`/`WhileExpr`/`IfExpr`/`Call`/`Spawn`），使管道完全在类型层工作。

这是 [RFC-019: 类型级同像性](./019-typed-homoiconicity.md) 的弱化实用版——编译器内置的计算结构进入类型系统，但不开放用户自定义语法。理论基础为 ECMTT（Contextual Modal Types for Algebraic Effects and Handlers, ICFP 2021）：`Spawn<T>` 对应模态算子 `□`，证明管道对应 handler。

该机制不限于 spawn——未来任何 effect（纯计算、IO、fallible）都可以通过同一种模式进入类型系统。spawn 是第一个消费者，不是唯一消费者。

> **独立 RFC 将定义**：6 个 MonoType 变体的完整语义、类型检查器适配策略、证明管道按类型分发的统一接口、与 RFC-027 的集成方案。

## 实现策略

### 阶段划分

1. **AST + Parser**：`Spawn { body: Box<Expr> }`，删除 `SpawnFor`
2. **DAG 分析统一**：合并入口，统一 `TaskSource` 枚举。单任务 spawn（`spawn call(x)`、`spawn 42`）产生编译警告
3. **IR / Codegen 适配**：删除 `Ir::SpawnFor`，统一处理路径
4. **Placement 简化**：删除 `SpawnFor` 分支
5. **测试验证**：现有 `spawn for` 测试全部通过

### 影响范围

| 文件/目录                                    | 改动                                                            |
| -------------------------------------------- | --------------------------------------------------------------- |
| `frontend/core/parser/ast.rs`                | `Spawn` body 改为 `Box<Expr>`，删除 `SpawnFor`                  |
| `frontend/core/parser/pratt/nud.rs`          | `spawn` 处理器简化为通用表达式解析                              |
| `frontend/core/spawn/analysis.rs`            | 统一入口，`TaskSource` 合并 Explicit + Iterate                  |
| `frontend/core/spawn/placement.rs`           | 删除 `SpawnFor` 分支                                            |
| `middle/core/ir.rs`                          | 删除 `Ir::SpawnFor`                                             |
| `middle/` (IR gen, codegen)                  | 统一 spawn 路径                                                 |
| `tests/yaoxiang/04-concurrency/spawn_for.yx` | 语义不变，验证通过                                              |

### 依赖关系

- RFC-024（spawn 块并发模型）— 本 RFC 是其正交性扩展
- RFC-010（统一类型语法）— 语法统一的基础

## 设计决策记录

| 决策               | 决定                                                                 | 原因                                                | 日期       |
| ------------------ | -------------------------------------------------------------------- | --------------------------------------------------- | ---------- |
| spawn 修饰范围     | 任意表达式                                                           | 消除 `spawn for` 特殊情况                           | 2026-06-16 |
| `spawn while` 支持 | 支持                                                                 | 语法正交，实现成本低。证明管道可能拒绝跨迭代依赖用例 | 2026-06-16 |
| `spawn if` 语义    | 修饰整个 if-else                                                     | 与 `if spawn { }` 区分                              | 2026-06-16 |
| spawn 绑定优先级   | 最低（等同 return）                                                  | 吃掉后面整个表达式                                  | 2026-06-16 |
| DAG 对 for 内部    | 不展开 for 内部子表达式                                              | 直接子表达式规则不变，for 整体为一个任务来源        | 2026-06-16 |
| 单任务 spawn 警告  | `spawn call(x)` / `spawn 42` 产生编译警告                            | 无并发效果，提醒用户检查意图                        | 2026-08-19 |
| 计算结构类型       | 推迟到独立 RFC                                                       | 通用机制，不限于 spawn。ECMTT 理论基础              | 2026-08-19 |

---

## 参考文献

- [RFC-024: 基于 spawn 块的并发模型](./024-concurrency-model.md)
- [RFC-010: 统一类型语法](./010-unified-type-syntax.md)
- [ECMTT: Contextual Modal Types for Algebraic Effects and Handlers (ICFP 2021)](https://arxiv.org/abs/2103.02976) — 计算结构类型的理论基础
- [并发模型规范](../../reference/language-spec/concurrency.md)
- [spawn for 正交性悬置（讨论稿）](../../dev/plan/ongoing/spawn-for-orthogonality.md)

---

## 生命周期与归宿

| 状态       | 位置                      | 说明         |
| ---------- | ------------------------- | ------------ |
| **审核中** | `docs/design/rfc/review/` | 开放社区讨论 |
