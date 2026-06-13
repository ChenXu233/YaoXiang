---
title: "RFC-009a: 令牌生命期 DAG 分析"
status: "草案"
author: "晨煦"
created: "2026-06-13"
updated: "2026-06-13"
group: "rfc-009"
---

# RFC-009a: 令牌生命期 DAG 分析

> **父 RFC**: [RFC-009: 所有权模型设计](../accepted/009-ownership-model.md)
>
> **本 RFC 修正 RFC-009 §"令牌冲突检测：流敏感活性分析"（第 663-684 行）中的错误结论。**

## 摘要

RFC-009 第 684 行声称令牌冲突检测"不需要……NLL"。这个结论不成立。借用令牌（`&T`/`&mut T`）之间的派生关系（品牌路径）要求编译器追踪令牌的生命期约束——父令牌在子令牌存活期间必须暂停。这不是"流敏感活性分析"能解决的，也不是"追踪任何线性类型值"能覆盖的。

本 RFC 提出实际方案：**全函数 `{}` 块 DAG 驱动令牌生命期分析**。利用 RFC-010 定义的 `{}` 依赖驱动语义和 RFC-024 已有的 spawn 块 DAG 分析，将令牌活性从"需要专门推断"降维为"DAG 查询"。品牌树（RFC-009 §2.7）负责冲突判断，DAG 负责时序判断。

**核心主张**：不实现 Rust 级别的 NLL（不动点控制流数据流），因为 YaoXiang 的设计约束（禁止遮蔽、显式 return、for 新绑定、`{}` DAG 语义）已消除了 NLL 要处理的歧义。但需要实现一个轻量的替代方案——本 RFC 定义它。

## 动机

### 当前问题

RFC-009 §"令牌冲突检测：流敏感活性分析" 声称：

> 不需要的东西：跨函数生命周期追踪、全局别名分析、借用图约束求解、**NLL**、`'a` 标注。因为令牌是值，值的活性分析由类型检查器统一处理——与追踪任何线性类型值完全相同。

**这个结论混淆了两个不同的问题：**

1. **线性追踪**（Move 后不可用）——类型检查器已有的能力。
2. **令牌生命期交互**（子令牌存活 → 父令牌暂停 → 子令牌死亡 → 父令牌复活）——需要额外的分析。

当前代码的实际情况验证了这个混淆的后果：

| 组件 | 状态 |
|------|------|
| `BorrowChecker`（`borrow_checker.rs`） | 线性扫描，依赖硬编码的 `Borrow`/`Release` 指令 |
| `ControlFlowAnalyzer`（`control_flow.rs`） | `analyze_instruction` 是空实现（第 145-153 行） |
| `liveness_analysis`（`mod.rs:586`） | 存在但仅用于 Drop 插入，未接入令牌冲突检测 |
| 令牌 Release 插入（`ir_gen.rs:2734-2736`） | 硬编码在 Call 指令之后——纯词法作用域 |

**用户可见后果**：以下代码当前**无法**通过编译（尽管应该通过）：

```yaoxiang
data = vec![1, 2, 3]
view = &data              # 创建 ReadToken
x = view.total_count      # view 的最后使用
# view 在此不再需要，但 Release 指令尚未被执行
data.push(4)              # ❌ 需要 WriteToken，但 ReadToken 仍活跃
```

### 为什么需要一个子 RFC

RFC-009 定义了令牌的类型属性（Dup/Linear）和品牌机制，但没有定义令牌生命期的交互规则和编译器如何推导这些规则。本 RFC 填补这个空白。

## 提案

### 核心设计：两个结构，一个分析

```
品牌树                          DAG
─────                          ───
回答"谁和谁冲突"               回答"现在冲突吗"
```

| 结构 | 职责 | 信息 |
|------|------|------|
| **品牌树** | 令牌派生关系，冲突判断 | `#1.field_x` 冲突于 `#1`，不冲突于 `#1.field_y` |
| **DAG** | 令牌时序，生命期判断 | `view` 的最后一个消费节点是节点 4 → 节点 4 之后死亡 |

一个分析——`{}` 块 DAG 构建——同时服务于并发调度（spawn）和令牌活性（所有块）。

### 1. 每个 `{}` 块构建 DAG

RFC-010 §"`{}` 语义：依赖驱动计算单元" 定义了 `{}` 的语义。本 RFC 将此语义扩展为编译器分析：

- **每个 `{}` 块**构建 DAG：节点 = 语句/表达式，边 = 数据依赖
- spawn 块的 DAG 持久化用于运行时调度（RFC-024 已有）
- 非 spawn 块的 DAG 用于令牌活性分析后即可丢弃

```yaoxiang
{
    data = vec![1, 2, 3]         # 节点 1：生产 data
    view = &data                  # 节点 2：生产 ReadToken(#N)，依赖 data
    x = view.total_count          # 节点 3：消费 view，生产 x
    # ← DAG 查询：view 的最后一个消费节点 = 节点 3
    # ← ReadToken(#N) 在节点 3 之后死亡
    data.push(4)                  # 节点 4：需要 WriteToken(data)
    # ← DAG 查询：data 的所有派生 ReadToken 是否已死？
    # ← ReadToken(#N) 已死 → ✅ 安全
}
```

**不需要"最后使用点分析"。** DAG 构建完成后，每个值的消费节点列表是现成的——最大消费节点 = 死亡点。

### 2. 品牌树：令牌派生关系

RFC-009 §2.7 定义了品牌机制。本 RFC 将其形式化为品牌树：

```
WriteToken(data, #1)             # 根
  ├── ReadToken(data.count, #1.count)      # 子令牌
  └── ReadToken(data.items, #1.items)      # 子令牌
```

**冲突规则**：品牌路径有前缀关系 → 冲突。

| 令牌 A | 令牌 B | 冲突？ |
|--------|--------|--------|
| `#1`（WriteToken） | `#1.count`（ReadToken） | ✅ 冲突——#1 是 #1.count 的前缀 |
| `#1.count` | `#1.items` | ❌ 不冲突——无前缀关系 |
| `#1.count` | `#1.count` | ⚠️ 同一个 ReadToken——Dup 允许 |

**父令牌状态**：子令牌存活期间，父令牌处于 `Suspended` 状态。所有子令牌死亡后父令牌 `Active`。

### 3. DAG + 品牌树联合查询

对每个令牌消费操作：

```
1. 品牌树查询：该操作需要的 WriteToken 与哪些令牌冲突？
2. DAG 查询：冲突令牌在此执行位置是否仍然存活？
3. 存活且冲突 → 报错。已死 → 通过。
```

**示例：WriteToken 检查**

```yaoxiang
data.push(4)                     # 执行位置：节点 4
# 1. 品牌树：需要 WriteToken(data, #1)。冲突令牌：
#    - ReadToken(data.count, #1.count)
#    - ReadToken(data.items, #1.items)
# 2. DAG：节点 4 之前，#1.count 的消费节点（节点 3）已执行
#    → #1.count 已死
#    → #1.items 的消费节点（节点 5）未执行
#    → #1.items 存活
# 3. 存活 + 冲突 → ❌ 编译错误
```

### 4. if/else：作为 DAG 节点

if/else 表达式整体是 DAG 的一个节点：

```yaoxiang
{
    data = vec![1, 2, 3]
    view = &data
    result = if cond {
        view.cached               # 子 DAG_{then}：消费 view
    } else {
        data.default()             # 子 DAG_{else}：不消费 view
    }
    # if 节点产出：result
    # view 在 then 分支被消费，在 else 分支未被消费
    # → 保守：view 可能存活
    data.push(4)                   # ❌ 保守冲突
}
```

**规则**：if 节点的令牌逃逸集合 = 各分支 return/产出值的令牌并集。任何分支持有令牌 → 汇合点令牌可能存活。

如果用户希望 view 不逃逸：

```yaoxiang
result = if cond {
    x = view.cached
    return x                       # 只返回 x，不返回 view
} else {
    return data.default()
}
# view 未在 return 中 → view 死在分支内 → 汇合点 view 已死
data.push(4)                       # ✅
```

### 5. 循环：每次迭代独立 DAG

YaoXiang for 循环（syntax.md §3.9）语义：每次迭代创建新绑定，迭代结束销毁。

```yaoxiang
loop {
    view = &data                   # DAG_{iter N}：创建 ReadToken(#N)
    token = view.next()             # 消费 view
    if token.is_empty() { break }
    # DAG_{iter N} 中 view 无更多消费者 → ReadToken(#N) 在迭代内死亡
    data.store(token)               # WriteToken(data)
    # ✅ 同一迭代内的 ReadToken 已死
}
```

**规则**：
- 每次循环迭代创建新的 DAG 实例
- 循环体内创建的令牌（`view`）在 `DAG_{iter}` 内创建，在 `DAG_{iter}` 内死亡
- 不同迭代间的令牌互不干扰（for 新绑定语义天然隔离）
- 循环外创建的令牌被循环体消费：DAG 中标记该令牌的消费节点，按正常规则处理

### 6. &mut 重借用：函数调用自动子令牌

```yaoxiang
foo: (x: &mut Vec(Int)) -> Void = {
    bar(x)                         # DAG：创建子 WriteToken(#2)，派生自 x(#1)
                                    # x(#1) → Suspended
    x.push(4)                      # DAG：子 WriteToken(#2) 无更多消费者
                                    # → 在 bar(x) 节点后死亡
                                    # x(#1) → Active
                                    # ✅
}
```

**规则**：
- 函数调用参数位置：编译器自动创建子令牌（子令牌生命期 = 调用期间）
- 调用返回后子令牌自动死亡
- 如果子令牌通过返回值逃逸：父令牌保持在 Suspended 状态直到子令牌死亡

### 7. Dup 复制：别名追踪

`&T` 是 Dup，可以复制。品牌树追踪所有副本：

```yaoxiang
view = &data                  # ReadToken(data, #1)
view2 = view                  # Dup 复制 → ReadToken(data, #1)，别名 +1
print(view2.total)            # 消费 view2
print(view.total)             # 消费 view
# view 和 view2 均无更多消费者 → ReadToken(data, #1) 死亡
data.push(4)                  # ✅
```

**规则**：品牌树中，Dup 复制不创建新品牌——同一个品牌编号，多个引用者。当所有引用者都完成最后消费，令牌死亡。

### 8. 函数间分析：签名摘要，不侵入被调用方 DAG

函数调用在调用方 DAG 中是一个节点。调用方不需要知道被调用方的内部 DAG——只需要从函数签名提取摘要：

```
函数签名 → 摘要

(&Point) -> (&Float, &Point)
  ├→ 消费：ReadToken(Point)
  ├→ 产出：ReadToken(Float) + ReadToken(Point)
  └→ 品牌派生：产出令牌的品牌 = 输入令牌品牌的子路径

(&mut Point) -> Void
  ├→ 消费：WriteToken(Point)
  └→ 产出：无
```

**品牌路径传播是纯语法的**——看返回类型表达式里引用了哪个参数：

```yaoxiang
# 被调用方
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)      # 返回类型引用 self → 品牌从 self 派生
}

# 调用方 DAG
p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()
# DAG 节点摘要：
#   - 消费 ReadToken(Point, #1)
#   - 产出 ReadToken(Float, #1.field_x)，ReadToken(Point, #1)
#   - 品牌树记录：#1.field_x 是 #1 的子令牌
```

**子令牌返回后，父令牌保持在 Suspended 状态**：

```yaoxiang
token = &mut data              # WriteToken(data, #1)
(view, token) = token.get_x()  # 返回 ReadToken(data.x, #1.field_x)
                                # WriteToken(#1) → Suspended（子令牌 #1.field_x 存活）
use(view)                      # view 的最后使用 → #1.field_x 死亡
                                # WriteToken(#1) → Active
token.y = 10                   # ✅
```

**不需要的东西**：
- ❌ 不需要内联被调用方的 DAG
- ❌ 不需要跨函数数据流不动点
- ❌ 不需要 `'a` 生命周期标注——品牌路径编码了输入→输出的连接
- ❌ 不需要全程序分析——每个函数独立分析，摘要由签名给出

**已有基础**：`OwnershipFlowAnalyzer`（`ownership_flow.rs`）已分析函数参数是否出现在返回值中（`ConsumeMode::Returns`）。在此基础上扩展：不是只回答"是否返回"，而是精确到"返回了哪个参数的哪个字段，品牌路径是什么"。

### 9. 品牌树级联死亡

子令牌派生自父令牌。父令牌被销毁时，子令牌必须级联死亡——子令牌的品牌路径以父令牌品牌为前缀，父令牌不存在了，子令牌的"派生关系"就没有了锚点。

**级联死亡规则**：

1. 父令牌被 Move（`&mut T` Linear）：所有子令牌 + 孙令牌死亡
2. 父令牌被覆盖（`mut` 重新赋值）：旧令牌的所有子令牌 + 孙令牌死亡
3. 父令牌离开作用域：所有子令牌 + 孙令牌死亡

```yaoxiang
token = &mut data              # WriteToken(data, #1)
view = token.get_view()        # ReadToken(data.field, #1.field)
# view 派生自 token，token 是 view 的父令牌

token2 = &mut data             # ❌ 想要新 WriteToken？
                                # 但 token 还活着 → 不能创建
```

情况不同：

```yaoxiang
token = &mut data              # WriteToken(data, #1)
view = token.get_view()        # ReadToken(data.field, #1.field)
token = &mut other             # Move WriteToken(#1)
                                # → 级联：ReadToken(#1.field) 死亡
                                # → WriteToken(#1) 被覆盖为 dead
                                # → WriteToken(other, #2) 创建
# view 在此不可用——父令牌死了
# ✅ 不再冲突
```

**实现**：品牌树中每个节点维护父节点指针。销毁节点时深度优先遍历子树，标记所有子节点为死亡。

### 10. break 带值逃逸

break 带值将令牌带出循环：

```yaoxiang
result = while cond {
    view = &data
    token = view.next()
    if token.is_empty() { break token }
    data.store(token)
}
# result 类型 = break 带出的值
# break 路径上 token 逃逸出循环 → 其子树令牌（如有）随之逃逸
```

**规则**：`break expr` → `expr` 中引用的令牌标记为逃逸出循环。逃逸令牌在循环外的生命期按正常 DAG 规则处理（从循环节点产出，连接到后续消费者）。

**和 if return 一致**——都是"显式语句决定什么离开子作用域"。不需要特殊处理，和 return 共享同一套逃逸标记机制。

### 11. 闭包捕获：延迟执行的 DAG 节点

闭包捕获令牌后，令牌的生命期取决于闭包的调用时机：

```yaoxiang
data = vec![1, 2, 3]
filter_fn = |x| x > threshold    # 闭包捕获 threshold（&Int 令牌）
# threshold 令牌的生命期延伸到 filter_fn 的最后一次调用
# 或 filter_fn 被丢弃时（取更晚的）
```

**与函数调用的区别**：

| | 函数调用 | 闭包捕获 |
|---|---|---|
| 消费时机 | 调用时一次 | 闭包每次调用时 |
| 令牌死亡 | 调用返回后 | 闭包最后一次调用后 / 闭包丢弃时 |
| DAG 表示 | 一个节点，消费已知 | 延迟节点，消费次数不确定 |

**保守规则**：闭包捕获的令牌活到闭包被丢弃。编译器不知道闭包会被调用多少次，只能假设闭包存活期间令牌需要保持活跃。

**spawn 块的特殊性**：闭包被传入 spawn 块 → 闭包可能被多次调用 → 令牌存活至 spawn 块结束。如果闭包是直接子表达式且无循环，保守处理；如果有循环，令牌活到循环结束。

**长远的优化**：对"创建后立即调用一次就丢弃"的闭包（`items.filter(\|p\| ...)`），编译器可优化为等效函数调用——令牌活到 filter 返回。但这是优化，不是正确性要求。保守行为永远是安全的。

## 详细设计

### 编译器架构

```
源文件
  │
  ├→ 解析 → AST
  │
  ├→ 类型检查（含 HM 推断）
  │
  ├→ 每个 {} 块 → DAG 构建
  │     │
  │     ├→ 品牌树注入：DAG 节点中识别令牌创建/消费
  │     │     ├→ Borrow 节点 → 品牌树中创建令牌节点
  │     │     ├→ 读写使用 → 品牌树中创建消费边
  │     │     └→ return 表达式 → 标记令牌逃逸
  │     │
  │     ├→ 令牌冲突检测（DAG + 品牌树联合查询）
  │     │     └→ 对每个需要 WriteToken 的节点：
  │     │           1. 品牌树：找出冲突的 ReadToken
  │     │           2. DAG：检查冲突令牌是否在当前节点之前已死亡
  │     │           3. 存活 → 报错
  │     │
  │     ├→ spawn 块：DAG 持久化 → 运行时调度（RFC-024）
  │     │
  │     └→ 非 spawn 块：DAG + 品牌树 → 令牌活性分析 → DAG 丢弃
  │
  └→ IR 生成 → 令牌 Release 指令由 DAG 分析结果驱动，不再硬编码
```

### DAG 构建规范

每个 `{}` 块内：

1. 每条语句/表达式是一个节点
2. 节点 A 消费节点 B 产出的值 → B → A 的有向边
3. 没有数据依赖的节点之间无边（可并发 / 可任意排序）
4. if/else：整体作为一个复合节点，内部分支各有子 DAG
5. loop：每次迭代一个 DAG 实例

### 品牌树规范

```
BrandTree:
  nodes: Map<BrandId, BrandNode>

BrandNode:
  id: BrandId          # 如 "#42"、"#42.field_x"
  kind: ReadToken | WriteToken
  source_var: Operand  # 原始变量
  children: Set<BrandId>  # 派生子令牌
  consumers: Set<DagNodeId>  # 消费该令牌的 DAG 节点
  ref_count: usize     # Dup 副本数
```

**冲突判断函数**：

```rust
fn conflicts(a: &BrandId, b: &BrandId) -> bool {
    // 同一品牌的多个 ReadToken 副本不冲突（Dup）
    // WriteToken 和派生 ReadToken 冲突
    // 不同来源的令牌不冲突
    a.source() == b.source()
        && (a.is_write() || b.is_write())
        && (a.is_prefix_of(b) || b.is_prefix_of(a))
}
```

### 与 RFC-009 正文的修正

本 RFC 被接受后，RFC-009 §"令牌冲突检测：流敏感活性分析"（第 663-684 行）应更新为：

1. 删除"不需要的东西：……NLL"这一行
2. 将"层 1：调用点检查"和"层 2：函数体流敏感"替换为指向本 RFC 的引用
3. 明确：令牌生命期分析使用 DAG + 品牌树，而非线性类型追踪

### 实现阶段

| 阶段 | 内容 | 依赖 |
|------|------|------|
| Phase 1 | 将 `{}` DAG 构建从 spawn 专属扩展为所有 `{}` 块可用 | RFC-024 DAG 构建器 |
| Phase 2 | 品牌树实现（当前仅在 RFC 中有概念描述） | Phase 1 |
| Phase 3 | DAG + 品牌树联合查询替代 BorrowChecker 线性扫描 | Phase 2 |
| Phase 4 | IR 生成中 Release 指令位置改为 DAG 分析驱动 | Phase 3 |
| Phase 5 | 循环迭代 DAG 实例化 | Phase 1 |

### 与现有代码的关系

| 现有组件 | 处理方式 |
|----------|---------|
| `BorrowChecker`（`borrow_checker.rs`） | Phase 3 后被 DAG + 品牌树替代 |
| `ControlFlowAnalyzer`（`control_flow.rs`） | 不再需要——DAG 中没有控制流分叉问题 |
| `liveness_analysis`（`mod.rs:586`） | 保留——Drop 插入仍然需要 |
| `ir_gen.rs` Release 硬编码 | Phase 4 后改为 DAG 驱动 |
| `emit_borrow_for_args`（`ir_gen.rs:276`） | 保留——自动借用规则不变；Release 位置改为 DAG 驱动 |

### 算法复杂度

**DAG 构建**：按变量聚合，不是按语句对逐对比较。

```
# 第一遍：收集每个变量的定义者和使用者（O(N)）
defs: Map<Var, Stmt>       # 变量 → 定义语句
uses: Map<Var, Vec<Stmt>>  # 变量 → 使用语句列表

for S in statements:
    for v in S.writes:     # 每条语句写 1-5 个变量
        defs[v] = S
    for v in S.reads:      # 每条语句读 1-5 个变量
        uses[v].push(S)

# 第二遍：按变量建依赖边（O(N)）
for v in vars:
    d = defs[v]
    for u in uses[v]:
        add_edge(d → u)
```

**O(N)**，N = 块内语句数。常数：每条语句读写 1-5 个变量，两次哈希查找。

**mut 重新赋值**（< 5% 的变量）：多个定义点。对该变量的每个使用者，找到文本顺序上最近的前驱定义——O(U)，U = 该变量的使用次数。不影响整体 O(N)。

**品牌树操作**：前缀匹配 O(depth)，深度 ≤ 3（`#1` → `#1.field` → `#1.field.sub`），常数级。

**DAG 查询**：每个令牌的最后一个消费节点在 DAG 构建时预计算。查询时比较节点序号——O(1)。

| 操作 | 复杂度 | 常数 |
|------|--------|------|
| DAG 构建 | O(N) | 哈希查找 |
| 品牌树冲突判断 | O(1) | 字符串前缀匹配 |
| DAG 令牌活性查询 | O(1) | 整数比较 |
| 函数间签名摘要 | O(1) | 品牌路径拼接 |

**与 Rust NLL 的对比**：Rust NLL 的核心开销不在数据流迭代本身，而在 Polonius 约束求解——借用之间的偏序关系被编码为代数约束系统，每一步推导都是图上的路径搜索 + 替换。本方案没有约束求解——令牌冲突是品牌路径的前缀匹配，O(depth) 字符串比较。

## 权衡

### 优点

1. **统一分析**：一个 DAG 构建同时服务并发调度和令牌活性，不引入新分析
2. **查询而非推断**：令牌活性是对已有 DAG 的查询，不需要独立的不动点迭代
3. **设计约束利用**：禁止遮蔽、显式 return、for 新绑定，每个约束都在 DAG 中自然地消除了歧义
4. **可逐步实现**：可以先扩展到所有 `{}` 块，再逐步接入令牌冲突检测
5. **复杂度可控**：不需要跨函数约束求解、不需要借用图、不需要 `'a` 标注

### 缺点

1. **全函数 DAG 构建**：每个 `{}` 块都需要，增加了编译开销（但 O(N) 实际，常数极小）
2. **品牌树需全新实现**：当前代码中品牌概念仅存在于 RFC，需要实现
3. **新代码替代旧代码**：BorrowChecker 和 ControlFlowAnalyzer 的现有工作将被替代
4. **保守行为**：if/else 返回集并集可能导致保守拒绝（用户需要显式控制 return 内容）

### 与 Rust NLL 的复杂度对比

| 复杂度来源 | Rust NLL | 本方案 |
|-----------|----------|--------|
| 控制流图不动点迭代 | 需要 | **不需要**——DAG 无环 |
| 跨函数生命周期约束 | 需要 | **不需要**——每函数独立分析 |
| 遮蔽歧义 | 需要处理 | **不存在**——禁止遮蔽 |
| 分支状态合并 | 需要不动点 | **不需要**——显式 return 并集 |
| 循环跨迭代数据流 | 需要 | **不需要**——for 新绑定语义 |
| 借用图约束求解（Polonius） | 需要 | **不需要**——品牌树前缀检查替代 |

## 替代方案

| 方案 | 为什么不选择 |
|------|-------------|
| 实现完整 Rust NLL | 过度设计——YaoXiang 的设计约束已消除了 NLL 要处理的歧义 |
| 当前状态（硬编码 Release） | 不够——用户必须手动管理令牌作用域 |
| 只在 spawn 块做 DAG | 不够——90%+ 的令牌使用在非 spawn 代码中 |
| GC 替代借用检查 | 违反语言设计原则——YaoXiang 是无 GC 语言 |

## 开放问题

- [ ] DAG 构建器当前实现是否容易扩展为所有 `{}` 块可用？（需要审查 `spawn/analysis.rs`）
- [ ] 品牌树的 `ref_count` 在循环展开时需要特殊的迭代间处理吗？
- [ ] 嵌套块之间的 DAG 信息流如何衔接？（外层 DAG 节点 = 内层 `{}` 块整体）

---

## 参考文献

- [RFC-009: 所有权模型设计](../accepted/009-ownership-model.md) — 父 RFC，§2.7 品牌机制，§"令牌冲突检测"
- [RFC-010: 统一类型语法](../accepted/010-unified-type-syntax.md) — `{}` 依赖驱动语义
- [RFC-024: 基于 spawn 块的并发模型](../accepted/024-concurrency-model.md) — spawn DAG 分析
- [语法规范](../../reference/language-spec/syntax.md) — §2.9 块表达式，§3.9 for 循环绑定语义

---

## 生命周期与归宿

| 状态 | 位置 | 说明 |
|------|------|------|
| **草案** | `docs/design/rfc/draft/` | 等待审核 |
