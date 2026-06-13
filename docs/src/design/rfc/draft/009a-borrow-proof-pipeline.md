---
title: "RFC-009a: 令牌生命期分析——基于霍尔证明管道"
status: "草案"
author: "晨煦"
created: "2026-06-13"
updated: "2026-06-13"
group: "rfc-009"
---

# RFC-009a: 令牌生命期分析——基于霍尔证明管道

> **父 RFC**: [RFC-009: 所有权模型设计](../accepted/009-ownership-model.md)
>
> **依赖**: [RFC-027: 编译期谓词与统一静态验证](../accepted/027-compile-time-evaluation-types.md)
>
> **本 RFC 修正并替代 RFC-009 §"令牌冲突检测：流敏感活性分析"（第 663-684 行）。**

## 摘要

RFC-009 第 684 行声称令牌冲突检测"不需要……NLL"。结论正确，论据错误。

它不是"因为令牌是值，线性追踪就够了"。它是因为：**令牌活性是霍尔逻辑命题，不是专门的流敏感分析。**

`{conflicting_tokens 全部死亡} op {WriteToken 安全获取}` —— 同一个 `{P} op {Q}`，和类型检查、谓词验证共享 RFC-027 的证明管道。没有新分析框架。一条管道，多种命题。

---

## 动机

### RFC-009 的混淆

RFC-009 把两个问题混为一谈：

1. **线性追踪**（Move 后不可用）—— `{v 未被 move} use(v) {类型匹配}`。类型检查器已有。
2. **令牌生命期交互**（子令牌存活 → 父令牌暂停 → 子令牌死亡 → 父令牌复活）—— `{conflicting_tokens 全部死亡} write(data) {安全}`。需要**活性分析**，不是线性追踪。

### 当前代码的实际情况

| 组件 | 状态 |
|------|------|
| `BorrowChecker` | 线性扫描 IR，被动响应显式的 `Borrow`/`Release` 指令 |
| `ControlFlowAnalyzer::analyze_instruction` | 空实现（`control_flow.rs:145-153`） |
| `liveness_analysis` | 存在但仅用于 Drop 插入，未接入令牌冲突 |
| Release 插入 | 硬编码在 Call 指令之后——纯词法作用域（`ir_gen.rs:2734-2736`） |

**用户可见后果**：

```yaoxiang
data = vec![1, 2, 3]
view = &data              # 创建 ReadToken
x = view.total_count      # view 的最后使用
data.push(4)              # ❌ Release(view) 尚未执行，ReadToken "活着"
```

### 为什么需要重写

上一版（009a v1）用"DAG 替代 NLL"叙事，引入了不必要的新概念（保守分支规则、循环特殊处理）。核心矛盾没有说清楚：**借用检查不是独立系统——它是霍尔命题的一种。**

---

## 核心设计

### 一切皆霍尔

```
类型检查：  { x: Int }        x + 1        { result: Int }
借用检查：  { view 已死 }     data.push(4)  { WriteToken 获取成功 }
谓词验证：  { y > 0 }         divide(x, y)  { result: Int }
回边切断：  { i == n }        下一轮循环     { cond == false }
```

同一个形式 `{P} op {Q}`。编译器对每个操作生成前置命题 P，送入证明管道验证。

**借用检查和用户谓词共享同一条管道。** 区别仅在于命题由谁生成、证不出来时怎么处理。

### 两类谓词，一条管道

| | 用户谓词 | 系统谓词（借用） |
|---|---|---|
| 命题生成 | 程序员（类型标注） | 编译器（品牌树 + 所有权规则） |
| 证明提供 | 编译器 + 程序员 | **编译器全自动** |
| 证不出来 | 写证明函数或重构 | 重构代码（门留着但极少需要） |
| 可见性 | 签名可见 | 隐式，不污染类型签名 |
| 学习成本 | 想用才学 | 零 |

**系统谓词的证明不给程序员开证明函数——编译器全自动。** 证不出来时用户重构代码。这和 Rust `'a` 不同：`'a` 是必修课，证明函数是选修课——绝大多数用户一辈子碰不到选修课的门。

### 借用命题：编译器自动生成

用户写 `data.push(4)`。编译器自动生成命题：

```
WriteToken(data, node) 可获取
  = forall t in conflicting_tokens(data): t 在 node 处已死
  = forall t in brand_tree.children(data): forward_reachable(node) ∩ consumers(t) == ∅
```

**三条规则，零特殊情况：**

1. **品牌树**（RFC-009 §2.7）回答"谁和谁冲突"：前缀匹配，O(depth)，深度 ≤ 3
2. **消费者列表**（DAG 构建时自动收集）回答"令牌最后被谁消费"
3. **前向可达性**回答"消费者还能不能被执行到"：结构性切断 + 逻辑切断

### 前向可达性：从消费者反向走

对令牌 T 的每个消费者 C：

```
从 C 出发，反向 BFS DAG。
边被切断，如果：
  1. 它是 break（结构切断）
  2. 路径条件 ⇒ !loop_cond 被 SMT 证明为真（逻辑切断，RFC-027 管道）

沿所有未切断的边反向传播（包括回边，回边将活性传播到前一轮迭代）。
标记所有能到达的节点 → unsafe。
```

查询：写操作在节点 W → W ∉ unsafe → 安全。

**不需要发明"保守分支规则"。不需要"循环保守存活"。一条反向 BFS + 两条切断规则。**

### 证明策略：快速通道优先，SMT 兜底

```
每个需要令牌的写操作
  │
  ├→ 快速通道：DAG 结构分析（覆盖 95%+ 场景）
  │     │
  │     ├→ 品牌树前缀匹配 → 找出冲突令牌（O(depth)）
  │     ├→ 反向 BFS，break 切断回边
  │     └→ 无回边可穿越 → 直接判定 Proved / Disproved
  │
  └→ 慢速通道：SMT 逻辑切断（仅当快速通道遇到可穿越回边时）
        │
        ├→ 回边起点有路径条件 → SMT 判 path_cond ⇒ !loop_cond
        │     ├→ Proved → 逻辑切断 → 降级回快速通道继续
        │     └→ Disproved / Unproven → 回边穿越 → 标记 unsafe
        │
        └→ 回边起点无路径条件 → 回边直接穿越
```

**快速通道覆盖**：线性代码、if/else、loop + break、无路径条件的 while。
**慢速通道覆盖**：while 循环体内、有路径条件暗示循环将退出时。
**不覆盖**：运行时条件无法静态证明 → 回边穿越 → unsafe → 编译错误（用户重构）。

SMT 不是主力——是安全网。和 RFC-027 的用户谓词不同：用户谓词以 SMT 为主力；借用系统谓词以结构分析为主力，SMT 只补结构分析够不到的边角。

---

## 用例分析

### 线性代码

```yaoxiang
data = vec![1, 2, 3]        # 节点 1
view = &data                # 节点 2：消费 data，生产 ReadToken(#1)
x = view.total_count        # 节点 3：消费 view（= #1 的最后一个消费者）
data.push(4)                # 节点 4：需要 WriteToken(data)
```

反向 BFS 从 `view.total_count`（节点 3）出发 → 节点 3 是 #1 的最后一个消费者 → 节点 4 > 节点 3 → 节点 4 不在 unsafe → ✅

### if/else：无特殊规则

```yaoxiang
view = &data
if cond {
    use(view)               # then 分支消费 view
} else {
    do_something_else()     # 不碰 view
}
data.push(4)                # view 的最后消费者在 if 内 → if 之后无消费者 → ✅
```

if/else 是 DAG 的一个复合节点。内部消费归因至此节点。不合并分支状态。不保守表决。**后面有没有消费者，整数比较。**

### if/else 带返回值逃逸

```yaoxiang
view = &data
result = if cond {
    view                     # view 逃逸到 result
} else {
    something_else
}
use(result)                  # 间接消费 view
data.push(4)                 # view 仍有消费者（use(result)）
                             # → push 在 unsafe → ❌ 正确报错
```

view 通过返回值逃逸 → `use(result)` 是 view 的消费者 → 从 `push` 反向走能到达 `use(result)` → unsafe。

### 循环：break 切断回边

```yaoxiang
view = &data
loop {
    use(view)                # consumer
    if is_last {
        data.push(4)         # 写操作
        break                # ← 结构切断
    }
}
```

反向 BFS 从 `use(view)` → 回边 → 向前走到 `data.push(4)` → 碰到 `break` → **切断** → `data.push(4)` 不在 unsafe → ✅

没有 break：

```yaoxiang
view = &data
loop {
    use(view)
    data.push(4)             # 无 break 切断 → 回边可穿越 → 下一轮 use(view) 可达
                             # → push 在 unsafe → ❌ 正确报错
}
```

### while：SMT 逻辑切断

```yaoxiang
view = &data
mut i: UpTo(n) = 0
while i < n {
    use(view)                # consumer
    i += 1
    if i == n {
        data.push(4)         # 路径条件：i == n
    }
}
```

反向 BFS 从 `use(view)` → 回边 → 走到 `data.push(4)` → 检查路径条件 `i == n` → SMT 查询：`i == n ⇒ !(i < n)`？→ Proved → **逻辑切断** → `data.push(4)` 不在 unsafe → ✅

---

## 本质：品牌 ID 就是 `'a`

不说"我们不需要 `'a`"。说"`#42` 就是 `'42`"。

| Rust | YaoXiang | 等价性 |
|------|----------|--------|
| `'a` | `#42` | 编译期生命周期标识符 |
| `'a: 'b` outlives 约束 | `#42` 是 `#42.field_x` 的前缀 | 字符串前缀比较 = 偏序关系 |
| NLL 活性传播（CFG 不动点） | 反向 BFS（DAG） | 都是可达性计算 |
| Polonius 事实 | SMT 逻辑切断 | 都是路径条件推理 |
| 约束系统不动点求解 | 品牌树前缀匹配 + BFS | 不同的编码，同一个问题 |

**我们没有发明新分析。我们只是把 `'a` 从类型签名层降到了证明层。** 品牌 ID 做的事和 `'a` 完全一样——标记借用的身份、追踪派生关系、判定冲突。区别只有一个：`'a` 在用户写的类型签名里；`#42` 在编译器内部。

这不丢人。Curry-Howard 说类型是命题，程序是证明。`'a` 不是命题的一部分——它是证明策略的一部分。Rust 把证明策略写进了命题签名。我们把它放回该在的地方。

### 语言设计约束消除了什么

| 复杂度来源 | 避开了？ | 原因 |
|---|---|---|
| 变量遮蔽 | ✅ | 语言禁止——一个名字永远指向同一个东西 |
| for 跨迭代借用 | ✅ | 每次迭代新绑定——迭代间天然隔离 |
| `'a` 生命周期标注 | ✅ | 品牌路径 = `#42.field_x`，编译器推导 |
| 命名生命周期 + 约束传播 | ✅ | 品牌路径前缀比较替代显式约束集 |
| 借用图约束求解（Polonius） | ✅ | 品牌树前缀匹配 + DAG 消费者查询 |
| 循环体借用活性传播 | ❌ | 和 Rust 一样需要处理——用反向 BFS + 逻辑切断 |
| 条件分支保守性 | ❌ | 和 Rust 一样——SMT 覆盖可证明的，剩余保守拒绝 |

### DAG vs CFG：不是数据结构的胜利

> Rust 用 CFG 不是因为 Rust 蠢——Rust 的遮蔽、隐式 drop、循环语义把分析逼进了 CFG。
> YaoXiang 用 DAG 不是因为 DAG 更高级——是因为语言设计从源头消除了那些迫使 CFG 的东西。
> **核心区别是语言设计约束，不是分析数据结构。**

---

## 详细设计

### 系统谓词清单

编译器自动生成以下命题，送入 RFC-027 证明管道：

| 系统谓词 | 触发时机 | 命题形式 |
|---|---|---|
| `borrow_conflict` | 需要 WriteToken(v) | `forall t ∈ conflicting(v): dead_at(t, node)` |
| `use_after_move` | 使用变量 v | `¬moved(v)` |
| `use_after_drop` | 使用变量 v | `¬dropped(v)` |
| `double_drop` | Drop(v) | `¬dropped(v)` |
| `mut_violation` | 写不可变变量 v | `is_mut(v)` |

现有的 `BorrowChecker`、`MoveChecker`、`DropChecker`、`MutChecker` **变成命题生成器**——不是消失，换身份。它们生成命题，管道验证命题。

### 品牌树

RFC-009 §2.7 的品牌机制形式化为品牌树。

**令牌语义——冻结优先，非复制优先**：

`&T` 和 `&mut T` 的本质区别不是"能不能复制"，是"允不允许同时有写"：

```
ReadToken(T)： 授予只读权限，同时冻结源数据 T——任何 WriteToken(T) 在此期间
              不可获取。冻结是 ReadToken 的首要语义。Dup（可复制）是冻结的推论：
              因为数据已被冻结（无突变可能），多份只读视图天然安全。

WriteToken(T)：授予独占读写权限。因为存在写，任何其他令牌（读或写）都不可共存。
              不实现 Dup（线性类型）是独占的推论。
```

**因果关系**：
```
ReadToken 存在 → 源数据冻结 → 多份只读安全 → Dup
                      ↓
              WriteToken 被拒绝（borrow_conflict 系统谓词强制）
```

不是：
```
ReadToken 有 Dup → 可以有多个 → 顺便检查冲突  ← 因果倒置
```

```
BrandTree:
  nodes: Map<BrandId, BrandNode>

BrandNode:
  id: BrandId               # "#42"、"#42.field_x"
  kind: ReadToken | WriteToken
  source_var: Operand
  parent: Option<BrandId>   # 派生关系的父节点
  children: Set<BrandId>    # 派生子令牌
  consumers: Set<NodeId>    # 消费该令牌的 DAG 节点
  ref_count: usize          # ReadToken 冻结期间的安全副本数
```

**冲突判断**——冻结保证的执行机制：

```rust
fn conflicts(a: &BrandId, b: &BrandId) -> bool {
    // 冲突条件：同源 + 至少一方是写 + 品牌路径重叠
    // 这意味着：
    //   1. ReadToken vs ReadToken → 无冲突（都是只读，无突变）
    //   2. WriteToken vs ReadToken → 冲突（写破坏了读的冻结保证）
    //   3. WriteToken vs WriteToken → 冲突（两个写不可共存）
    a.source() == b.source()
        && (a.is_write() || b.is_write())
        && (a.is_prefix_of(b) || b.is_prefix_of(a))
}
```

O(depth) 字符串前缀比较，深度 ≤ 3。常数级。

### 反向 BFS 活性分析

```
算法：check_borrow(token, node, dag, brand_tree)

输入：
  token: 需要检查的 WriteToken
  node:  写操作所在的 DAG 节点

输出：Proved | Disproved

算法：
  # 快速通道：反向 BFS
  unsafe = empty_set
  queue = brand_tree.consumers(token)

  while queue not empty:
    cur = queue.pop()
    unsafe.add(cur)

    for each pred in dag.predecessors(cur):
      # 结构切断：break 不穿越
      if pred 是 break 边:
        continue

      # 回边 → 检查是否需要 SMT fallback
      if pred 是回边:
        path_cond = pred 处路径条件
        loop_cond = 循环条件
        # 先看结构上能否切断（对应的 break 已切断路径 → 不会走到这）
        # 再看路径条件
        if path_cond 非空:
          result = smt_fallback(path_cond, loop_cond)   # ← 慢速通道
          if result == Proved:
            continue                    # 逻辑切断
        # 无路径条件 或 SMT 证不出来 → 穿越回边
        # fall through

      if pred ∉ unsafe:
        queue.push(pred)

  # 判定
  if node ∈ unsafe:
    return Disproved
  else:
    return Proved


smt_fallback(path_cond, loop_cond):
  # 仅在回边 + 有路径条件时调用
  # 使用 RFC-027 证明管道，共享同一 SMT 求解器、同一预算
  return smt.prove(path_cond ⇒ !loop_cond)
  # Proved → 逻辑切断
  # Disproved / Unproven → 不切断，回边穿越
```

O(N)，其中 SMT 调用次数 = 回边数 × 有路径条件的回边比例。实际代码中 SMT 调用极为罕见——仅 `while` 循环体内、有精化类型变量的路径条件时触发。

### 路径条件收集

由 RFC-027 §3.2-3.3 已有机制提供：

- **if guard**：`if y > 0` → true 分支压入 `y > 0`
- **match 模式**：`if let Some(v) = opt` → 分支内压入 `opt == Some(v)`
- **赋值**：`i += 1`，编译器维护变量值域信息
- **while cond**：循环体内压入 `cond == true`

每个 DAG 节点携带一个路径条件集合。反向 BFS 回边时，取回边起点的路径条件，SMT 判断是否排除下一轮循环入口条件。

### 与 RFC-027 的接口

借用系统谓词和用户谓词共享同一条证明管道——区别在于**主力证明策略**不同：

| 查询类型 | 命题来源 | 主力策略 | Fallback |
|---|---|---|---|
| 类型等式 | 类型检查器 | 结构等价 | — |
| 用户谓词 | 程序员类型标注 | SMT | 程序员证明函数 |
| 借用冲突 | 编译器自动生成 | **DAG 结构分析（快速通道）** | SMT 逻辑切断 |

SMT 求解器在借用检查中的角色：**不是主力，是安全网。** 只在 while 回边需要逻辑切断时调用。绝大多数借用检查在快速通道完成——O(N) 反向 BFS，零 SMT 开销。

### 与现有代码的关系

| 现有组件 | 处理 |
|----------|------|
| `BorrowChecker` | 变为 `BorrowPredicateEmitter`——生成借用的霍尔命题 |
| `MoveChecker` | 变为 `MovePredicateEmitter`——生成 `¬moved(v)` 命题 |
| `DropChecker` | 同上——生成 Drop 相关命题 |
| `MutChecker` | 同上——生成 `is_mut(v)` 命题 |
| `ControlFlowAnalyzer` | 不再需要——管道统一处理 |
| `liveness_analysis` | 保留——Drop 插入仍需要变量活性信息 |
| `ir_gen.rs` Release 硬编码 | 删除——Release 位置由 DAG 消费者分析驱动 |

### 算法复杂度

| 操作 | 复杂度 | 触发频率 |
|------|--------|---------|
| 品牌树冲突判断 | O(1) | 每次需要令牌 |
| DAG 消费者查询 | O(1) | 每次需要令牌 |
| 反向 BFS（快速通道） | O(N) | 每次需要令牌，N = 块内节点数 |
| SMT 逻辑切断（fallback） | ~1ms | **极罕见**——仅 while + 路径条件 |

**SMT fallback 的触发条件极为苛刻**：同时满足 (1) while 循环 (2) 循环体内有写操作 (3) 写操作后有路径条件可判循环终止 (4) 编译器需要依赖该条件切断回边。实际代码中占比远低于 1%。其余的借用检查全部在快速通道完成。

和 RFC-027 用户谓词的关系：用户谓词以 SMT 为主力，借用系统谓词以结构分析为主力。两者共享同一 SMT 求解器和预算上限（RFC-027 §8），但借用系统谓词几乎不消耗 SMT 预算。

线性代码 → 无回边 → 分层 1 O(N) 秒出。循环 + 路径条件 → SMT 调用，线性算术毫秒级（RFC-027 预算 100ms）。一次 BFS 结果可缓存供同一令牌的多次查询复用。

### 错误信息设计

**核心原则：错误信息只出现用户写过的符号。**

Rust 和借用相关的错误分两类：

**变量级错误**：E0597（活得不够长）、E0502（可变+不可变同时借用）、E0499（多次可变借用）。Rust 已经是标杆——变量名+行号，不出现 `'a`。YaoXiang 精确度持平。信息全在品牌树里：令牌创建点、消费者位置、请求点。

**签名级错误**：E0623（lifetime mismatch）、E0106（missing lifetime specifier）、E0477（不满足 required lifetime）。围绕 `'a` 展开。YaoXiang **不存在这类错误**——签名里没有 `'a`。不是"报不出来"，是用户没写过的东西不用报。

函数内冲突示例：

```
错误：`data` 被冻结，不能获取可变权限
 --> src/main.yx:5:9
2 |     view = &data
  |            ----- `data` 被冻结（只读令牌创建在此处）
4 |         use(view)
  |             ---- `view` 在此处仍在使用，冻结未解除
5 |         data.push(4)
  |         ^^^^ 此处需要可变权限
```

（与 Rust E0499 精确度持平——变量名+行号，不出现品牌 ID。）

函数间逃逸示例：

```
错误：`num`（第 4 行）持有的数据来源之一是 `default_str`（第 3 行），
但 `default_str` 在第 6 行失效，`num` 在第 5 行仍在被使用。

考虑：将 `default_str` 的声明提前到调用方，或使用 `ref default_str` 共享持有。
```

（与 Rust E0597 精确度持平。品牌摘要知道 `num` 有两条来源路径——编译器中已有，错误措辞可用。）

---

## RFC-009 正文修正

RFC-009 §"令牌冲突检测：流敏感活性分析"已更新：

1. 删除"不需要的东西：……NLL"——不是因为结论错，是因为理由错（"令牌是值，线性追踪就够了"）
2. 层 1/层 2 过渡方案保留，完整方案指向本 RFC
3. 明确：品牌 ID（`#42`）就是 `'a`——信息完全一样，编码不同。不是发明了新分析——是把生命周期从类型层降到了证明层

---

## 权衡

### 优点

1. **类型签名不含生命周期**：`#42` 就是 `'42`——同一个信息，编码在品牌树里，不暴露在类型签名中。此点不可证伪：数一下 Rust 里有 3 个引用参数的泛型类型需要几个 `'a` 参数，YaoXiang 里需要几个。答案是 3 vs 0。

2. **概念统一**：借用检查和用户谓词共享同一条证明管道——`{P} op {Q}`，管道验证 P。Curry-Howard 一致。

3. **零新分析框架**：不引入新分析框架。用户不感知"借用检查器"的存在——就像用户不感知"类型检查器"的实现细节。

4. **错误信息只含用户写过的符号**：少了一整个维度的错误类别（E0623、E0106、E0477——全部围绕 `'a`）。变量级错误与 Rust 精确度持平。

5. **算法不保守**：反向 BFS + break 切断 + SMT 逻辑切断。不需要"循环内保守存活"。不需要"分支保守合并"。

### 缺点

1. **不是新发明**：品牌 ID 做的事和 `'a` 完全一样——编译器内部的约束求解复杂度没有消失，只是编码方式从"变量名+约束集"变成了"品牌路径+前缀匹配"。对终端用户的差异仅在于签名里不写 `'a`。

2. **全新实现**：品牌树在代码中仅存概念，需从头实现。BorrowChecker、ControlFlowAnalyzer 被替换。

3. **SMT 依赖**：逻辑切断依赖 Z3（RFC-027 已引入，不新增依赖）。但借用检查几乎不触发——仅 while + 路径条件时调用。

4. **极少数模式需重构**：编译器自动证明覆盖不了的跨分支借用，用户需重构代码。和 Rust `'a` 的兜底不同：Rust 有 `'a` 当笔（标注即能过）；YaoXiang 的兜底（证明函数）不是 MVP。

---

## 替代方案

| 方案 | 为什么不选 |
|------|-----------|
| 实现完整 Rust NLL | YaoXiang 的设计约束（无遮蔽、for 新绑定）已消除 NLL 主要复杂度来源，不需要 CFG 不动点 |
| 保持当前（硬编码 Release） | 不够——用户必须手动管理令牌作用域 |
| 只在 spawn 块做分析 | 不够——非 spawn 代码中的令牌使用是大多数 |
| GC 替代借用检查 | 违反语言设计原则——YaoXiang 无 GC |

---

## 实现阶段

| 阶段 | 内容 | 依赖 |
|------|------|------|
| Phase 1 | 品牌树数据结构实现 | — |
| Phase 2 | 系统谓词生成器（Borrow/Move/Drop/Mut → 命题） | Phase 1 |
| Phase 3 | 反向 BFS 活性分析 + 管道接入（分层 1） | Phase 2 |
| Phase 4 | 路径条件收集 + SMT 逻辑切断（分层 2） | Phase 3 + RFC-027 Phase 2 |
| Phase 5 | Release 指令改为 DAG 消费者驱动 | Phase 3 |
| Phase 6 | 删除 ControlFlowAnalyzer、重构 BorrowChecker | Phase 4 |

---

## 开放问题

- [ ] 品牌树在循环展开时的 `ref_count` 跨迭代语义
- [ ] `?` 错误传播路径上的令牌释放顺序（`?` 路径 = 提前 return，令牌需要级联死亡）
- [ ] 证明函数语法（远期，非 MVP）

---

## 参考文献

- [RFC-009: 所有权模型设计](../accepted/009-ownership-model.md) — 父 RFC
- [RFC-027: 编译期谓词与统一静态验证](../accepted/027-compile-time-evaluation-types.md) — 证明管道
- [RFC-010: 统一类型语法](../accepted/010-unified-type-syntax.md) — `{}` 语义
- [RFC-024: 基于 spawn 块的并发模型](../accepted/024-concurrency-model.md) — spawn DAG

---

## 生命周期与归宿

| 状态 | 位置 | 说明 |
|------|------|------|
| **草案** | `docs/design/rfc/draft/` | 等待审核 |
