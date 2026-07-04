---
title: "RFC-032: spawn 统一表达式修饰 — 消除 spawn for 特殊情况"
status: "审核中"
author: "晨煦"
created: "2026-06-16"
updated: "2026-07-03"
---

# RFC-032: spawn 统一表达式修饰

> **本文档定义 `spawn` 的语法、AST/IR 重构、类型系统扩展**。
> 运行时行为语义（任务拆解粒度、所有权、作用域、错误传播、资源类型、嵌套）见 [RFC-024: 基于 spawn 的并发运行时语义](./024-concurrency-model.md)。
>
> 两个 RFC 协同定义 `spawn` —— 024 回答"做什么"，032 回答"怎么表示"。

> **核心洞察**：`spawn` 不应该只修饰 `{}` 块。它可以修饰**任意表达式**。`spawn for` 不是特殊语法——它就是 `spawn` + `for` 表达式的自然组合。

## 摘要

将 `spawn` 从 `spawn { }`（仅修饰块）扩展为 `spawn <expr>`（修饰任意表达式）。`Expr::SpawnFor` 从 AST 中删除，由 `Expr::Spawn { body: Expr::For { .. } }` 自然替代。表达式结构的类型（Block、For、While、If 等）作为新 `MonoType` 变体进入类型系统，`Spawn<T>` 包装并发执行的计算结构，编译期标记，检查后擦除。


## 动机

### 为什么需要这个变更？

当前 `spawn for x in items { body }` 是独立的关键词组合，AST 中有 `Expr::SpawnFor` 专门表示它。这破坏了语言的正交性：

1. **语法不统一**：`spawn` 只能修饰 `{}` 块，`spawn for` 是硬编码的例外
2. **正交性缺失**：`spawn while`、`spawn if` 等组合无法自然表达
3. **类型系统不完整**：spawn 在类型系统中不可见，无法通过类型反射获取并发结构

### 当前的问题

```rust
// AST 中两个 spawn 变体
Spawn { body: Box<Block>, span: Span },         // spawn { ... }
SpawnFor { var, var_mut, iterable, body, span },  // spawn for x in items { ... }

// MonoType 只有值类型，没有计算结构类型
// spawn { a, b } 类型 = Tuple(T_a, T_b)  ← 丢失了"这是 spawn"的信息
// spawn for    类型 = List(T)             ← 丢失了"这是数据并行"的信息
```

## 提案

### 核心设计

`spawn <expr>`：`spawn` 修饰任意表达式。表达式的形状决定 DAG 如何分解任务。

**一切皆类型**：`MonoType` 从"值类型"扩展为"值类型 + 计算结构类型"。每个关键表达式结构在类型系统中有对应的类型变体。`Spawn<T>` 包装被并发执行的计算结构。

### 用户心智模型

`spawn` = "把这个表达式拿去做并发"。表达式的形状决定怎么拆：

| 表达式形状 | 并发行为 | 类型 |
|-----------|---------|------|
| `spawn { a, b, c }` | `a`、`b`、`c` 独立并行 | `Spawn(Block(Tuple(T_a, T_b, T_c)))` |
| `spawn for x in items { f(x) }` | N 个迭代独立并行 | `Spawn(ForExpr { body_ty: List(T) })` |
| `spawn while cond { step() }` | 每轮迭代独立任务 | `Spawn(WhileExpr { body_ty: List(T) })` |
| `spawn if c { a } else { b }` | 被选中分支整体为 spawn 域 | `Spawn(IfExpr { then_ty: T_a, else_ty: Some(T_b) })` |
| `spawn call(x)` | 调用本身作为一个任务 | `Spawn(Call { fn_ty: Fn(A→R), result_ty: R })` |
| `spawn 42` | 单独一个任务 | `Spawn(Int)` |

编译器负责 DAG 分析确定依赖关系，运行时按 GMP 模型调度——无依赖的任务扔进工作队列，worker 抢着跑。整体同步阻塞，等待所有任务完成。

**与 Go 的区别**：Go 的 `go` 是"扔出去不管"，YaoXiang 的 `spawn` 是"拆开并行执行，等全部干完再往下"。

### 控制流正交性

| 组合 | 语义 | 差异 |
|------|------|------|
| `spawn for x in items { body }` | 数据并行：每次迭代 = 独立任务 | DAG 跨迭代分析依赖 |
| `for x in items spawn { body }` | 每次迭代创建一个 spawn 域 | 不跨迭代分析 |
| `spawn while cond { body }` | 条件并行：每次迭代 = 独立任务 | 迭代间依赖由条件保证 |
| `while cond spawn { body }` | 每次迭代创建一个 spawn 域 | 与上面语义不同但不需要特殊处理 |
| `spawn if c { a } else { b }` | 整个 if-else 为一个 spawn 域 | 执行时按条件选分支 |
| `if c spawn { a } else { b }` | 仅单分支 spawn | if 表达式内部包 spawn |

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

| 写法 | AST 结构 |
|------|---------|
| `spawn if cond { a } else { b }` | `Spawn { body: Expr::If { ... } }` |
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

`spawn for` 不再作为组合关键词处理——`for` 由通用表达式解析器处理产生 `Expr::For`，`spawn` 只负责包装。

### 3. 类型系统

**新增 `MonoType` 变体：**

```rust
// ========== 计算结构类型 ==========

/// {} 块表达式
Block(Box<MonoType>),

/// for 循环表达式
ForExpr { body_ty: Box<MonoType> },

/// while 循环表达式
WhileExpr { body_ty: Box<MonoType> },

/// if-else 分支表达式
IfExpr {
    then_ty: Box<MonoType>,
    else_ty: Option<Box<MonoType>>,
},

/// 函数调用表达式
Call {
    fn_ty: Box<MonoType>,
    result_ty: Box<MonoType>,
},

/// spawn 并发包装：内部表达式被并发执行
/// 编译期标记，类型检查后擦除
Spawn(Box<MonoType>),
```

**类型推导规则**：每个表达式的类型推导返回"计算结构类型"。无 `Spawn` 包装 = 顺序执行，有 `Spawn` 包装 = 并发执行。类型检查完毕后 `Spawn` 擦除，类型降级为内部值类型。

**类型检查流程**：
1. 推导 body 表达式的类型 T（计算结构类型）
2. 若为 spawn 包裹，包装为 `Spawn(T)`
3. 赋值推导时解构：`results: List(Data) = spawn for ... {}` — 从 `Spawn(ForExpr { body_ty: List(Data) })` 提取 `List(Data)`

`Spawn<T>` 在类型检查完毕后擦除，运行时不需要知道数据来自并发还是顺序。但编译期反射（`type_of(x)`）可获取完整的并发拓扑结构。

### 4. DAG 分析层

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

| body 种类 | 如何分解为任务 |
|-----------|--------------|
| `Expr::Block` | 直接子表达式 → 任务列表 |
| `Expr::For` | 每次迭代 → 一个任务（数据并行） |
| `Expr::While` | 每次迭代 → 一个任务 |
| `Expr::If` | 被选中分支整体 → 一个任务 |
| `Expr::Call` / 其他 | 表达式本身 → 一个任务 |

DAG 分析完成后，运行时按 GMP 模型调度——无依赖的任务扔进工作队列，worker 抢着跑。

### 5. IR / Codegen 层

`Ir::SpawnFor` 删除。统一为 `Ir::Spawn`，携带 `TaskSource` 信息。

HIR → IR 翻译根据 `SpawnAnalysis.source` 生成运行时调用：
- `TaskSource::Explicit(tasks)` → 编译期已知任务列表
- `TaskSource::Iterate { .. }` → 运行时展开（编译器驱动，类似 par_iter 但零成本）

### 6. Placement 层

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

### 7. 向后兼容性

已有 `spawn for` 代码语义不变，Parser 自动将 `spawn for x in items { body }` 解析为 `Expr::Spawn { body: Expr::For }`。内部表示变化，用户可见行为不变。

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

## 权衡

### 优点

1. **语法正交**：`spawn` + 任意控制流 = 自然并发组合
2. **一切皆类型**：类型系统完整记录计算结构，编译期反射获得并发拓扑
3. **消除特殊情况**：删除 `Expr::SpawnFor` 及相关特殊处理代码
4. **可扩展**：未来新增控制流结构自动与 `spawn` 组合，无需修改 spawn 逻辑

### 缺点

1. **类型系统膨胀**：新增 6 个 `MonoType` 变体，增加类型检查复杂度
2. **破坏性变更**：内部 AST/IR 表示变化，需更新所有消费 `Expr::SpawnFor` 的代码
3. **表达式类型推导**：每个表达式现在需要返回计算结构类型，影响面大

## 替代方案

| 方案 | 为什么不选择 |
|------|-------------|
| 保持 `spawn for` 独立语法 | 破坏正交性，成为语言中唯一的关键词组合特例 |
| `spawn` 仅修饰 `{}`，数据并行走标准库 `par_iter` | 语言原始能力下沉到库，失去编译器层面的 DAG 分析和资源冲突检测 |
| 只删除 `SpawnFor` 但不在类型系统引入计算结构类型 | 类型系统失去反射能力，spawn 在类型层面不可见 |


## 与 RFC-019 的关系

本 RFC 引入的 6 个 `MonoType` 变体（Block/ForExpr/WhileExpr/IfExpr/Call/Spawn）是 [RFC-019: 类型级同像性](./019-typed-homoiconicity.md) 的**编译器内置子集**。RFC-019 的核心理念"语法结构进入类型系统"在这里实现为：6 种编译器原生理解的计算结构拥有对应的类型表示。用户不能通过 `SyntaxRule` 自定义新的计算结构类型，但编译器内置的这 6 种已覆盖所有关键控制流。

## 证明管道集成

6 个 `MonoType` 变体存在的原因：它们告诉 [RFC-027 编译期证明管道](../accepted/027-compile-time-evaluation-types.md) **要验证的命题是什么形状**。管道本身负责实际证明工作（自由变量分析、效果分类、别名分析、冲突检测），MonoType 只做一件事——提供结构化的输入接口。

### 变体 → 命题映射

| 类型 | 命题形状 | 证明策略 |
|------|---------|---------|
| `Spawn(ForExpr { body_ty })` | 数据并行：N 个迭代任务无跨迭代冲突 | 提取 body 自由变量 → 效果分类 → 检查无 Write(Shared) / `&mut`(Shared) |
| `Spawn(WhileExpr { body_ty })` | 条件并行：每轮迭代独立 + 无跨迭代因果依赖 | 同上 + 检查迭代条件是否有跨迭代副作用 |
| `Spawn(Block(T))` | 显式任务组：任务间依赖关系由 DAG 给出 | 验证 DAG 分析的依赖图——每个任务所需的输入在其开始时已就绪 |
| `Spawn(IfExpr { then_ty, else_ty })` | 分支 spawn：选中分支整体为一个 spawn 域 | 分支选择无冲突，body 内递归检查 |
| `Spawn(Call { fn_ty, result_ty })` | 调用 spawn：被调用函数作为一个独立任务 | 验证函数的纯性或隔离性 |
| `Spawn(T)`（值，如 `spawn 42`） | 单值 spawn：无并发 | 平凡通过 |

### 证明场景

**场景 1 — 纯数据并行（通过）：**

```yaoxiang
items = [1, 2, 3, 4, 5]
results = spawn for item in items { item * 2 }
// 类型：Spawn(ForExpr { body_ty: List(Int) })
```

1. 自由变量：`item`（循环局部，每次迭代独立副本）、`items`（外部，body 内只读）
2. 效果分类：全部 Read(Local) 或 Read(Shared)，无写入
3. Proved ✓

**场景 2 — 只读共享（通过）：**

```yaoxiang
config = load_config()
results = spawn for item in items { process(item, config) }
// 类型：Spawn(ForExpr { body_ty: List(Result) })
```

1. 自由变量：`item`（Read(Local)）、`config`（外部，body 内无写入路径 → Read(Shared)）
2. 效果分类：全部只读
3. Proved ✓

**场景 3 — 写冲突（拒绝）：**

```yaoxiang
mut counter = 0
spawn for item in items { counter += 1 }
```

1. 自由变量：`item`（Read(Local)）、`counter`（外部，`+=` 脱糖为写入）
2. 效果分类：`counter` 为 Write(Shared)，跨迭代写入同一内存
3. 实例化冲突：`Write(task_0, counter) ∧ Write(task_1, counter) = True`
4. Disproved ✗ → 编译错误：`错误：spawn for body 中存在跨迭代写冲突。变量 counter 被多个并发任务写入。`

**场景 4 — while + 有状态迭代器（警告/拒绝）：**

```yaoxiang
spawn while iter.has_next() {
    item = iter.next()
    process(item)
}
// 类型：Spawn(WhileExpr { body_ty: List(Processed) })
```

1. 自由变量：`iter`（外部，`next()` → `&mut self` → `&mut`(Shared)）
2. `next()` 修改迭代器状态，迭代 N+1 依赖迭代 N 的副作用
3. 这不是独立任务 → 违反 `Spawn(WhileExpr)` 的独立性约束
4. 编译器报告跨迭代因果依赖，建议改用 `spawn for`

**场景 5 — spawn if（通过）：**

```yaoxiang
result = spawn if use_cache { load(key) } else { fetch(key) }
// 类型：Spawn(IfExpr { then_ty: T, else_ty: Option(T) })
```

1. 只执行一个分支，不存在跨任务冲突
2. body 内有子 spawn 则递归检查
3. Proved ✓

**场景 6 — spawn 块任务间依赖（DAG + 管道验证）：**

```yaoxiang
spawn {
    a = fetch_user(id)
    b = fetch_orders(a.user_id)  // 依赖 a
    c = compute_stats()           // 独立
}
// 类型：Spawn(Block(Tuple(User, Orders, Stats)))
```

1. DAG 分析：`a` 和 `c` 独立（可并行），`b` 依赖 `a`（在 a 之后调度）
2. 管道验证：`b` 的输入（`a.user_id`）在 b 启动时已计算完成
3. Proved ✓

### MonoType 不做什么

| 做什么 | 不做什么 |
|--------|---------|
| 标识命题形状 | 不执行证明 |
| 在类型层面记录计算结构 | 不替代 DAG 分析 |
| 给 RFC-027 管道提供类型输入 | 不替代自由变量分析、别名分析、冲突检测 |

实际的证明工作由编译器标准分析 pass 完成。MonoType 的价值是让这些 pass 在一个统一的类型框架下被调度——证明管道不需要针对每种 AST 节点写特殊分支。
## 实现策略

### 阶段划分

1. **AST + Parser**：`Spawn { body: Box<Expr> }`，删除 `SpawnFor`
2. **类型系统**：新增 6 个 `MonoType` 变体，所有表达式类型推导返回计算结构类型
3. **DAG 分析统一**：合并入口，统一 `TaskSource` 枚举
4. **IR / Codegen 适配**：删除 `Ir::SpawnFor`，统一处理路径
5. **Placement 简化**：删除 `SpawnFor` 分支
6. **测试验证**：现有 `spawn for` 测试全部通过

### 影响范围

| 文件/目录 | 改动 |
|-----------|------|
| `frontend/core/parser/ast.rs` | `Spawn` body 改为 `Box<Expr>`，删除 `SpawnFor` |
| `frontend/core/parser/pratt/nud.rs` | `spawn` 处理器简化为通用表达式解析 |
| `frontend/core/types/mono.rs` | 新增 `Block`/`ForExpr`/`WhileExpr`/`IfExpr`/`Call`/`Spawn` 变体 |
| `frontend/core/spawn/analysis.rs` | 统一入口，`TaskSource` 合并 Explicit + Iterate |
| `frontend/core/spawn/placement.rs` | 删除 `SpawnFor` 分支 |
| `frontend/core/typecheck/` | 所有表达式节点适配计算结构类型推导 |
| `middle/core/ir.rs` | 删除 `Ir::SpawnFor` |
| `middle/` (IR gen, codegen) | 统一 spawn 路径，Spawn 类型擦除 |
| `tests/yaoxiang/04-concurrency/spawn_for.yx` | 语义不变，验证通过 |

### 依赖关系

- RFC-024（spawn 块并发模型）— 本 RFC 是其正交性扩展
- RFC-010（统一类型语法）— 类型系统变更的基础
- RFC-027（编译期证明管道）— MonoType 变体为管道提供命题形状输入
- RFC-019（类型级同像性）— MonoType 变体是其编译器内置子集

## 设计决策记录

| 决策 | 决定 | 原因 | 日期 |
|------|------|------|------|
| spawn 修饰范围 | 任意表达式 | 消除 `spawn for` 特殊情况 | 2026-06-16 |
| `spawn while` 支持 | 支持 | 语法正交，实现成本低 | 2026-06-16 |
| `spawn if` 语义 | 修饰整个 if-else | 与 `if spawn { }` 区分 | 2026-06-16 |
| 类型系统 | 引入计算结构类型 | "一切皆类型"，支持编译期反射 | 2026-06-16 |
| spawn 类型擦除 | 类型检查后擦除 | 运行时不需要并发结构信息 | 2026-06-16 |
| spawn 绑定优先级 | 最低（等同 return） | 吃掉后面整个表达式 | 2026-06-16 |
| DAG 对 for 内部 | 不展开 for 内部子表达式 | 直接子表达式规则不变，for 整体为一个任务来源 | 2026-06-16 |
| 证明管道集成 | MonoType 变体映射到 RFC-027 证明命题 | 管道需要知道验证的命题形状，MonoType 提供结构化输入 | 2026-07-03 |
| RFC-019 关系 | 编译器内置子集 | 用户不能自定义，但共享"语法即类型"理念 | 2026-07-03 |
| 证明边界 | 6 个场景覆盖：纯并行/只读共享/写冲突/while 依赖/spawn if/spawn block | 明确每个 MonoType 变体的证明义务和失败条件 | 2026-07-03 |

---

## 参考文献

- [RFC-024: 基于 spawn 块的并发模型](./024-concurrency-model.md)
- [RFC-010: 统一类型语法](./010-unified-type-syntax.md)
- [RFC-027: 编译期谓词与统一静态验证](../accepted/027-compile-time-evaluation-types.md)
- [RFC-019: 类型级同像性](./019-typed-homoiconicity.md)
- [并发模型规范](../../reference/language-spec/concurrency.md)
- [spawn for 正交性悬置（讨论稿）](../../dev/plan/ongoing/spawn-for-orthogonality.md)

---

## 生命周期与归宿

| 状态 | 位置 | 说明 |
|------|------|------|
| **审核中** | `docs/design/rfc/review/` | 开放社区讨论 |
