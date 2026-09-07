---
title: 'RFC-009a: Token Lifetime Analysis—Based on the Hoare Proof Pipeline'
status: 'Accepted'
author: '晨煦'
created: '2026-06-13'
updated: '2026-08-17'
group: 'rfc-009'

issue: '#129'

impl: 'partial'
---

# RFC-009a: Token Lifetime Analysis—Based on the Hoare Proof Pipeline

> **Parent RFC**: [RFC-009: Ownership Model Design](../accepted/009-ownership-model.md)
>
> **Dependencies**:
> [RFC-027: Compile-Time Predicates and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md)
>
> **Prerequisites**: RFC-027 has been accepted. All mechanisms in this RFC (proof pipeline, SMT
> fallback, path condition collection) depend on the implementation of RFC-027.
>
> **This RFC corrects and replaces RFC-009 §"Token Conflict Detection: Flow-Sensitive Liveness
> Analysis" (lines 663-684).**

## Summary

Line 684 of RFC-009 claims that token conflict detection "does not require... NLL". The conclusion
is correct, but the argument is wrong.

It is not "because tokens are values, linear tracking is enough". It is because: **token liveness is
a Hoare logic proposition, not a dedicated flow-sensitive analysis.**

`{conflicting_tokens all dead} op {WriteToken safely acquired}` — the same `{P} op {Q}`, sharing the
proof pipeline of RFC-027 with type checking and predicate verification. No new analysis framework.
One pipeline, many kinds of propositions.

---

## Motivation

### The Confusion in RFC-009

RFC-009 conflates two problems:

1. **Linear tracking** (unavailable after Move) — `{v not moved} use(v) {type matches}`. The type
   checker already has this.
2. **Token lifetime interaction** (child token alive → parent token paused → child token dead →
   parent token revived) — `{conflicting_tokens all dead} write(data) {safe}`. Requires **liveness
   analysis**, not linear tracking.

### Current State of the Code

| Component                                  | Status                                                                            |
| ------------------------------------------ | --------------------------------------------------------------------------------- |
| `BorrowChecker`                            | Linearly scans IR, passively responds to explicit `Borrow`/`Release` instructions |
| `ControlFlowAnalyzer::analyze_instruction` | Empty implementation (`control_flow.rs:145-153`)                                  |
| `liveness_analysis`                        | Exists but only used for Drop insertion, not connected to token conflicts         |
| Release insertion                          | Hardcoded after Call instructions—pure lexical scope (`ir_gen.rs:2734-2736`)      |

**User-visible consequences**:

```yaoxiang
data = vec![1, 2, 3]
view = &data              # 创建 ReadToken
x = view.total_count      # view 的最后使用
data.push(4)              # ❌ Release(view) 尚未执行，ReadToken "活着"
```

### Why a Rewrite Is Needed

The previous version (009a v1) used a "DAG replaces NLL" narrative, introducing unnecessary new
concepts (conservative branching rules, special loop handling). The core contradiction was not made
clear: **borrow checking is not a standalone system—it is a kind of Hoare proposition.**

---

## Core Design

### Everything Is Hoare

```
类型检查：  { x: Int }        x + 1        { result: Int }
借用检查：  { view 已死 }     data.push(4)  { WriteToken 获取成功 }
谓词验证：  { y > 0 }         divide(x, y)  { result: Int }
回边切断：  { i == n }        下一轮循环     { cond == false }
```

Same `{P} op {Q}` form. The compiler generates a precondition P for each operation and sends it to
the proof pipeline for verification.

**Borrow checking and user predicates share the same pipeline.** The only difference is who
generates the proposition and what happens when it cannot be proved.

### Two Kinds of Predicates, One Pipeline

|                        | User Predicate                   | System Predicate (Borrow)                        |
| ---------------------- | -------------------------------- | ------------------------------------------------ |
| Proposition Generation | Programmer (type annotation)     | Compiler (brand tree + ownership rules)          |
| Proof Provision        | Compiler + Programmer            | **Compiler fully automatic**                     |
| Cannot Prove           | Write proof function or refactor | Refactor code (door left open but rarely needed) |
| Visibility             | Visible in signature             | Implicit, doesn't pollute type signature         |
| Learning Cost          | Learn only if you want to use    | Zero                                             |

**Proofs for system predicates do not open proof functions for the programmer—the compiler is fully
automatic.** When unable to prove, the user refactors the code.

**Three failure modes, one verification engine.** Type proposition cannot be proved → compile error
(unbypassable). Borrow proposition cannot be proved → compile error, refactor code (unbypassable).
User predicate cannot be proved → compile error, can write proof function (bypassable). The failure
strategies differ, but the verification engine is the same—SMT solver + compiler core inference
rules. The only difference is "who is responsible for supplying the proof when it cannot be
proved"—the compiler refuses to write borrow proofs for the programmer (the proof strategy for
borrow propositions is structural analysis + SMT, no programmer intervention needed), but accepts
programmer-written proof functions for user predicates. This is not pipeline inconsistency—it is the
responsibility boundary between different proposition categories being different.

This differs from Rust `'a`: `'a` is a required course, proof functions are an elective—most users
never touch the door of the elective course in their lifetime.

### Borrow Propositions: Compiler-Generated

The user writes `data.push(4)`. The compiler automatically generates the proposition:

```
WriteToken(data, node) 可获取
  = forall t in conflicting_tokens(data): t 在 node 处已死
  = forall t in brand_tree.children(data): forward_reachable(node) ∩ consumers(t) == ∅
```

**Three rules, zero special cases:**

1. **Brand tree** (RFC-009 §2.7) answers "who conflicts with whom": prefix matching, O(depth), depth
   ≤ 3
2. **Consumer list** (automatically collected during DAG construction) answers "who last consumed
   the token"
3. **Forward reachability** answers "can the consumer still be executed": structural cut + logical
   cut

### Forward Reachability: Walk Backwards from Consumers

For each consumer C of token T:

```
从 C 出发，反向 BFS DAG。
边被切断，如果：
  1. 它是 break（结构切断）
  2. 路径条件 ⇒ !loop_cond 被 SMT 证明为真（逻辑切断，RFC-027 管道）

沿所有未切断的边反向传播（包括回边，回边将活性传播到前一轮迭代）。
标记所有能到达的节点 → unsafe。
```

Query: write operation at node W → W ∉ unsafe → safe.

**No need to invent "conservative branching rules". No "conservative loop liveness". One reverse
BFS + two cut rules.**

### Proof Strategy: Fast Path First, SMT as Fallback

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

**Fast path coverage**: linear code, if/else, loop + break, while without path conditions. **Slow
path coverage**: inside while loops, when a path condition implies the loop will exit. **Not
covered**: runtime conditions cannot be statically proved → back edge traversed → unsafe → compile
error (user refactors).

SMT is not the main force—it is the safety net. Unlike RFC-027's user predicates, which use SMT as
the main force; borrow system predicates use structural analysis as the main force, with SMT only
filling in the corners that structural analysis cannot reach.

> **Errata (2026-08-17, SMT positioning correction)**: SMT is a **precision layer, not a soundness
> dependency**. The sound judgment of borrow system predicates is entirely carried by the fast path
> (interval + reverse BFS + break cut); SMT logical cuts only determine "whether a legitimate
> program at a loop boundary can pass through". When SMT is unavailable / times out / not
> implemented (RFC-027 impl: in_progress), the fallback = back edge traversed = conservative
> rejection, everything that should be rejected still must be rejected. **The conservatism without
> SMT = reject any borrow+write inside a loop, on par with Rust NLL** (Rust's production borrow
> checking likewise has no SMT). SMT implementation is pure precision gain, and does not block the
> delivery of the sound main line.

---

## Use Case Analysis

### Linear Code

```yaoxiang
data = vec![1, 2, 3]        # 节点 1
view = &data                # 节点 2：消费 data，生产 ReadToken(#1)
x = view.total_count        # 节点 3：消费 view（= #1 的最后一个消费者）
data.push(4)                # 节点 4：需要 WriteToken(data)
```

Reverse BFS from `view.total_count` (node 3) → node 3 is the last consumer of #1 → node 4 > node 3 →
node 4 not in unsafe → ✅

### if/else: No Special Rules

```yaoxiang
view = &data
if cond {
    use(view)               # then 分支消费 view
} else {
    do_something_else()     # 不碰 view
}
data.push(4)                # view 的最后消费者在 if 内 → if 之后无消费者 → ✅
```

if/else is a composite node in the DAG. Internal consumption is attributed to this node. Branch
states are not merged. No conservative voting. **Whether there is a consumer afterwards, integer
comparison.**

> **Clarification**: "Not merging branch states" only refers to **borrow liveness** (brand consumer
> reverse BFS). **move state** (variable ownership) is a separate analysis: per-CFG-node forward
> data flow (NLL/Polonius style), with a **conservative meet** at branch confluence (any branch
> Moved → confluence Moved), and literally unreachable branches (`if false`) do not participate. The
> two are layered: borrow liveness looks at "is there a subsequent consumer", move analysis looks at
> "whether the variable may have been moved".

### if/else with Return Value Escape

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

view escapes through the return value → `use(result)` is a consumer of view → reverse walking from
`push` can reach `use(result)` → unsafe.

### Loop: break Cuts the Back Edge

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

Reverse BFS from `use(view)` → back edge → walk forward to `data.push(4)` → hit `break` → **cut** →
`data.push(4)` not in unsafe → ✅

Without break:

```yaoxiang
view = &data
loop {
    use(view)
    data.push(4)             # 无 break 切断 → 回边可穿越 → 下一轮 use(view) 可达
                             # → push 在 unsafe → ❌ 正确报错
}
```

### while: SMT Logical Cut

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

Reverse BFS from `use(view)` → back edge → walk to `data.push(4)` → check path condition `i == n` →
SMT query: `i == n ⇒ !(i < n)`? → Proved → **logical cut** → `data.push(4)` not in unsafe → ✅

> Errata (2026-08-17): `i == n` is the path condition of the write node itself (`data.push(4)`
> inside the if branch), and the judgment target is the write node (see §Path Condition Collection
> Errata Rule 1).

---

## Essence: Brand ID Is `'a`

We don't say "we don't need `'a`". We say "`#42` is `'42`".

| Rust                                    | YaoXiang                         | Equivalence                              |
| --------------------------------------- | -------------------------------- | ---------------------------------------- |
| `'a`                                    | `#42`                            | Compile-time lifetime identifier         |
| `'a: 'b` outlives constraint            | `#42` 是 `#42.field_x` 的前缀    | String prefix comparison = partial order |
| NLL liveness propagation (CFG fixpoint) | Reverse BFS (DAG)                | Both are reachability computations       |
| Polonius facts                          | SMT logical cut                  | Both are path condition reasoning        |
| Constraint system fixpoint solving      | Brand tree prefix matching + BFS | Different encoding, same problem         |

**We have not invented new analysis. We have just lowered `'a` from the type signature layer to the
proof layer.** What brand ID does is exactly the same as `'a`—marking borrow identity, tracking
derivation relationships, judging conflicts. The only difference is: `'a` is in the user-written
type signature; `#42` is inside the compiler.

This is not embarrassing. Curry-Howard says types are propositions, programs are proofs. `'a` is not
part of the proposition—it is part of the proof strategy. Rust writes the proof strategy into the
proposition signature. We put it back where it belongs.

### What Language Design Constraints Eliminate

| Source of Complexity                       | Avoided? | Reason                                                               |
| ------------------------------------------ | -------- | -------------------------------------------------------------------- |
| Variable shadowing                         | ✅       | Language forbids—one name always points to the same thing            |
| for cross-iteration borrow                 | ✅       | Each iteration is a new binding—iterations are naturally isolated    |
| `'a` lifetime annotation                   | ✅       | Brand path = `#42.field_x`, compiler-inferred                        |
| Named lifetime + constraint propagation    | ✅       | Brand path prefix comparison replaces explicit constraint set        |
| Borrow graph constraint solving (Polonius) | ✅       | Brand tree prefix matching + DAG consumer query                      |
| Loop body borrow liveness propagation      | ❌       | Needs to be handled like in Rust—using reverse BFS + logical cut     |
| Conditional branch conservatism            | ❌       | Same as Rust—SMT covers provable cases, rest conservatively rejected |

### Why DAG Is Feasible

Three language design constraints of YaoXiang make DAG analysis feasible:

- **No variable shadowing**—one name always points to the same thing, no need to track across
  re-bindings
- **for each iteration is a new binding**—iterations are naturally isolated, no cross-iteration
  borrows
- **Structured concurrency**—task boundaries are clear, no cross-task liveness propagation

These constraints eliminate the main sources of complexity in Rust's CFG fixpoint iteration. It is
not that DAG is "more advanced" than CFG—rather, simpler language design allows simpler analysis.

---

## Detailed Design

### System Predicate List

The compiler automatically generates the following propositions and sends them to the RFC-027 proof
pipeline:

| System Predicate  | Trigger Timing                | Proposition Form                              |
| ----------------- | ----------------------------- | --------------------------------------------- |
| `borrow_conflict` | Needs WriteToken(v)           | `forall t ∈ conflicting(v): dead_at(t, node)` |
| `use_after_move`  | Use variable v                | `¬moved(v)`                                   |
| `use_after_drop`  | Use variable v                | `¬dropped(v)`                                 |
| `double_drop`     | Drop(v)                       | `¬dropped(v)`                                 |
| `mut_violation`   | Write to immutable variable v | `is_mut(v)`                                   |

Existing `BorrowChecker`, `MoveChecker`, `DropChecker`, `MutChecker` **become proposition
generators**—not disappearing, changing identity. They generate propositions, and the pipeline
verifies them.

### Brand Tree

The brand mechanism from RFC-009 §2.7 is formalized as the brand tree.

**Token semantics—freeze-first, not copy-first**:

The essential difference between `&T` and `&mut T` is not "whether it can be copied", but "whether
concurrent writes are allowed":

```
ReadToken(T)： 授予只读权限，同时冻结源数据 T——任何 WriteToken(T) 在此期间
              不可获取。冻结是 ReadToken 的首要语义。Dup（可复制）是冻结的推论：
              因为数据已被冻结（无突变可能），多份只读视图天然安全。

WriteToken(T)：授予独占读写权限。因为存在写，任何其他令牌（读或写）都不可共存。
              不实现 Dup（线性类型）是独占的推论。
```

**Causal relationship**:

```
ReadToken 存在 → 源数据冻结 → 多份只读安全 → Dup
                      ↓
              WriteToken 被拒绝（borrow_conflict 系统谓词强制）
```

Not:

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

**Conflict judgment**—enforcement mechanism of the freeze guarantee:

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

O(depth) string prefix comparison, depth ≤ 3. Constant order.

### Reverse BFS Liveness Analysis

> **Errata (2026-08-17 audit)**: The following algorithm adds the "token creation time" dimension.
> The original relied on the DAG node total order implicitly assuming "borrow before write", and did
> not cover the legal order of "write first, borrow later" (§2.4 semantics: parameter token released
> when call ends)—audit testing showed this scenario was misreported. Token liveness is an
> **interval** `[created_at, last_use]`, not a reverse reachable set; a write operation only
> constitutes a conflict within the liveness interval of the token.

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
        path_cond = 写节点 node 的路径条件   # 勘误：判定目标为写节点自身条件
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

  # 判定（勘误：加入创建时间区间）
  # 写先、借用后：node < created_at(token) → 写发生时令牌尚不存在 → Safe
  if node ∈ unsafe and created_at(token) ≤ node:
    return Disproved
  else:
    return Proved


smt_fallback(path_cond, loop_cond):
  # 仅在回边 + 有路径条件时调用
  # 使用 RFC-027 证明管道，共享同一 SMT 求解器、同一预算
  return smt.prove(path_cond ⇒ !loop_cond)
  # Proved → 逻辑切断
  # Disproved / Unproven → 不切断，回边穿越（保守拒绝）
  # 勘误（2026-08-17）：SMT 不可用/超时/未实现 = Disproved 分支——
  # SMT 只影响精度（合法程序能否过），不影响 soundness（该拒的必拒）；
  # 无 SMT 时的保守度 = 循环内借用+写一律拒，与 Rust NLL 同级。
```

BrandNode adds a field (errata):

```
BrandNode:
  ...
  created_at: NodeId         # 令牌创建节点（勘误：borrow 区间的左端点）
```

O(N), where the number of SMT calls = number of back edges × proportion of back edges with path
conditions. In actual code, SMT calls are extremely rare—only triggered inside `while` loops when
there are path conditions with refined type variables.

### Path Condition Collection

Provided by existing mechanisms in RFC-027 §3.2-3.3:

- **if guard**: `if y > 0` → true branch pushes `y > 0`
- **match pattern**: `if let Some(v) = opt` → inside branch pushes `opt == Some(v)`
- **Assignment**: `i += 1`, compiler maintains variable value range information
- **while cond**: inside loop body pushes `cond == true`

Each DAG node carries a set of path conditions. When the reverse BFS traverses a back edge, take the
path conditions at the back edge's starting point, and let SMT judge whether the next loop
iteration's entry condition is excluded.

> **Errata (2026-08-17, propagation rules completion)**: The original text did not define how path
> conditions propagate to the back edge's starting point, and the use case "while + SMT logical cut"
> with `i == n` cannot be reproduced by the original algorithm. Supplementary rules:
>
> 1. **Path conditions attached to the write node itself**: a write operation W inside a branch
>    carries its branch condition (`if i == n { W }` → path_cond(W) = `i == n`). When the reverse
>    BFS traverses the back edge, SMT judges `path_cond(W) ⇒ !loop_cond` (the path reaching W must
>    exit the loop → next iteration's consumer is unreachable → cut), not the path conditions of the
>    back edge node.
> 2. **join conservatively cleared**: if/else confluence points do not carry path conditions inside
>    branches (the disjunction of the two branch conditions is usually not decidable, just clear
>    it). Write operations after the confluence point have empty path_cond → back edge traversed.
> 3. **Path conditions semanticized**: path_cond is a ConstExpr (RFC-027 §3.2 semantics), not source
>    text; smt_cut translates it into SMT constraints before solving.
> 4. **No path conditions → back edge directly traversed** (unsafe), no SMT call.

### Interface with RFC-027

Borrow system predicates and user predicates share the same proof pipeline—the difference lies in
the **main proof strategy**:

| Query Type      | Proposition Source         | Main Strategy                           | Fallback                  |
| --------------- | -------------------------- | --------------------------------------- | ------------------------- |
| Type equality   | Type checker               | Structural equivalence                  | —                         |
| User predicate  | Programmer type annotation | SMT                                     | Programmer proof function |
| Borrow conflict | Compiler auto-generated    | **DAG structural analysis (fast path)** | SMT logical cut           |

The role of the SMT solver in borrow checking: **not the main force, but the safety net.**

Only called when a while back edge needs a logical cut. The vast majority of borrow checks are
completed in the fast path—O(N) reverse BFS, zero SMT overhead.

### Relationship with Existing Code

| Existing Component             | Treatment                                                                 |
| ------------------------------ | ------------------------------------------------------------------------- |
| `BorrowChecker`                | Becomes `BorrowPredicateEmitter`—generates Hoare propositions for borrows |
| `MoveChecker`                  | Becomes `MovePredicateEmitter`—generates `¬moved(v)` propositions         |
| `DropChecker`                  | Same as above—generates Drop-related propositions                         |
| `MutChecker`                   | Same as above—generates `is_mut(v)` propositions                          |
| `ControlFlowAnalyzer`          | No longer needed—pipeline handles uniformly                               |
| `liveness_analysis`            | Retained—Drop insertion still needs variable liveness information         |
| `ir_gen.rs` Release hardcoding | Removed—Release position is driven by DAG consumer analysis               |

### NLL and Iteration Boundaries

> **Errata (2026-08-17, interval model completion)**: Token liveness is an **interval**
> `[created_at, last_use]`, not a reverse reachable set. `created_at` = the token's creation node;
> `last_use` = the maximum consumption node from consumer analysis. The necessary and sufficient
> condition for write operation W to conflict with token T:
> `conflicts(T, W) ∧ created_at(T) ≤ node(W)` `∧ node(W) can forward-reach last_use(T)` (judged by
> reverse BFS). The legal order of "write first, borrow later" (§2.4: parameter token released when
> call ends) is directly excluded by `created_at(T) ≤ node(W)`, without any special rules. This
> model makes the §Trade-off Advantage 5 "algorithm not conservative" claim hold in all orders.

**Token death time = last use point (NLL), not the end of the lexical scope.**

This is a natural consequence of consumer analysis: the consumer's position defines the token's last
use. `use(v)` is a consumer of `v` → `v` dies immediately after `use(v)`. No additional `{}` or
`drop()` is needed to end the token's life early.

**Loop iteration boundaries are the death line of token copies.** Three rules:

```
规则 1：循环内声明的变量在每次迭代结束时自动死亡。
        for 的每次迭代是新绑定（语言设计保证），loop 同理。

规则 2：品牌树 ref_count 在循环头只计入循环外创建的副本。
        循环内 Dup 产生的新副本，ref_count 在迭代边界清零。

规则 3：反向 BFS 穿越回边时，不携带当前迭代的活性信息。
        只携带循环头处的 ref_count（即：循环外的副本）。
```

Example:

```yaoxiang
view = &data                          # 循环头：ref_count = 1，consumer = use(view)
loop {
    v2: &Point = view                 # 循环内 Dup → ref_count = 2
    use(v2)                           # consumer：v2 的最后使用 → v2 死亡 → ref_count = 1
    data.push(4)                      # ✅ 安全！v2 已死，只剩 view（ref_count = 1，非写冲突）
    # 迭代边界：规则 3——不携带 v2 进入下一轮。下一轮迭代开始时 v2 被新绑定重新创建。
}
```

This design does not need additional "conservative loop liveness" rules. Reverse BFS starts from
consumers; consumers are inside the loop body → liveness is confined within the current iteration →
back edges are not traversed. This is fully consistent with the loop example in RFC-009a §Use Case
Analysis.

### `?` Error Propagation and Scope-Driven Release

`?` is an early return—an additional exit path beyond the normal exit of a scope. Tokens must be
released on this path, and wrong release order is UB.

**Release instructions are generated by scope analysis, not hardcoded after Call.**

The compiler maintains a list of exit points for each scope:

- `}` (normal scope end)
- `?` (error propagation, early return)
- explicit `return`

At each exit point, insert Release instructions for all active tokens in that scope in declaration
reverse order (LIFO). The parent-child relationship of the brand tree automatically handles the
cascade release of derived tokens:

```yaoxiang
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)    # 返回子令牌 &Float + 父令牌 &Point
}

fn use_case(p: Point) -> Result<(), Error> = {
    (x_ref, p_ref) = p.get_x()?   # 如果 ? 传播：
    # 品牌树知道 x_ref 是 p_ref 的派生（#42.field_x 是 #42 的前缀）
    # 释放顺序：x_ref（子）→ p_ref（父）→ LIFO 自动满足
    p.modify()                     # WriteToken——所有 ReadToken 已释放
    Ok(())
}
```

Implementation location: kept in `ir_gen.rs`, changed to be scope-driven—no new compiler pass is
introduced.

| Conflict Judgment | O(1) | Each time a token is needed | | DAG Consumer Query | O(1) | Each time a
token is needed | | Reverse BFS (fast path) | O(N) | Each time a token is needed, N = number of
nodes in the block | | SMT logical cut (fallback) | ~1ms | **Extremely rare**—only while + path
conditions |

> Errata (2026-08-17): the complexity figures in the above table are design estimates, not actually
> measured; "~1ms" and "extremely rare" should be regarded as order-of-magnitude expectations rather
> than measured values, and should be calibrated with observability data after implementation lands.

**The trigger conditions for SMT fallback are extremely strict**: simultaneously satisfying (1)
while loop (2) there is a write operation inside the loop body (3) after the write operation there
is a path condition that can judge loop termination (4) the compiler needs to rely on this condition
to cut the back edge. The proportion in actual code is far below 1%. All other borrow checks are
completed in the fast path.

Relationship with RFC-027 user predicates: user predicates use SMT as the main force, borrow system
predicates use structural analysis as the main force. Both share the same SMT solver and budget cap
(RFC-027 §8), but borrow system predicates hardly consume any SMT budget.

Linear code → no back edges → tier 1 O(N) instant. Loop + path conditions → SMT call, linear
arithmetic millisecond-level (RFC-027 budget 100ms). One BFS result can be cached for multiple
queries on the same token.

### Error Message Design

**Core principle: error messages only contain symbols the user has written.**

Rust has two categories of borrow-related errors:

**Variable-level errors**: E0597 (doesn't live long enough), E0502 (mutable+immutable simultaneous
borrow), E0499 (multiple mutable borrows). Rust is already the benchmark—variable name + line
number, no `'a` appears. YaoXiang matches in precision. All information is in the brand tree: token
creation point, consumer location, request point.

**Signature-level errors**: E0623 (lifetime mismatch), E0106 (missing lifetime specifier), E0477
(doesn't satisfy required lifetime). Centered on `'a`. YaoXiang **does not have this kind of
error**—no `'a` in the signature. It's not "can't be reported", it's that things the user hasn't
written don't need to be reported.

In-function conflict example:

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

(Precision matches Rust E0499—variable name + line number, no brand ID appears.)

Inter-function escape example:

```
错误：`num`（第 4 行）持有的数据来源之一是 `default_str`（第 3 行），
但 `default_str` 在第 6 行失效，`num` 在第 5 行仍在被使用。

考虑：将 `default_str` 的声明提前到调用方，或使用 `ref default_str` 共享持有。
```

(Precision matches Rust E0597. The brand digest knows that `num` has two source paths—already in the
compiler, error wording is available.)

---

## RFC-009 Body Corrections

RFC-009 §"Token Conflict Detection: Flow-Sensitive Liveness Analysis" has been updated:

1. Delete "Things not needed:... NLL"—not because the conclusion is wrong, but because the reason is
   wrong ("tokens are values, linear tracking is enough")
2. The tier 1/tier 2 transitional scheme is retained, the complete scheme points to this RFC
3. Clarify: brand ID (`#42`) is `'a`—same information, different encoding. Not inventing a new
   analysis—lowering lifetime from the type layer to the proof layer

---

## Trade-offs

### Advantages

1. **Type signature does not contain lifetime**: `#42` is `'42`—the same information, encoded in the
   brand tree, not exposed in the type signature. This point is unfalsifiable: count how many `'a`
   parameters a Rust generic type with 3 reference parameters needs, vs how many YaoXiang needs. The
   answer is 3 vs 0.

2. **Conceptual unification**: borrow checking and user predicates share the same proof
   pipeline—`{P} op {Q}`, the pipeline verifies P. Curry-Howard consistency.

3. **Zero new analysis framework**: no new analysis framework is introduced. Users don't perceive
   the existence of a "borrow checker"—just as they don't perceive the implementation details of the
   "type checker".

4. **Error messages only contain symbols the user has written**: an entire dimension of error
   categories is gone (E0623, E0106, E0477—all centered on `'a`). Variable-level errors match Rust
   in precision.

5. **Algorithm not conservative**: reverse BFS + break cut + SMT logical cut. No need for
   "conservative liveness inside loops". No need for "conservative branch merging".

### Disadvantages

1. **Not a new invention**: what brand ID does is exactly the same as `'a`—the constraint solving
   complexity inside the compiler hasn't disappeared, only the encoding has changed from "variable
   name + constraint set" to "brand path + prefix matching". The only difference for end users is
   that `'a` is not written in the signature.

2. **Brand new implementation**: the brand tree only exists as a concept in code, it needs to be
   implemented from scratch. BorrowChecker and ControlFlowAnalyzer are replaced.

3. **SMT dependency**: logical cut depends on Z3 (already introduced by RFC-027, no new dependency).
   But borrow checking almost never triggers it—only called for while + path conditions.

4. **A very small number of patterns require refactoring**: cross-branch borrows that the compiler's
   automatic proof cannot cover require the user to refactor code. Different from Rust `'a`'s safety
   net: Rust has `'a` as a pen (annotate and it passes); YaoXiang's safety net (proof functions) is
   not MVP.

---

## Alternatives

| Plan                             | Why not chosen                                                                                                                              |
| -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| Implement complete Rust NLL      | YaoXiang's design constraints (no shadowing, for new bindings) have eliminated the main sources of NLL complexity, no need for CFG fixpoint |
| Keep current (hardcoded Release) | Not enough—users must manually manage token scope                                                                                           |
| Only do analysis in spawn blocks | Not enough—token usage in non-spawn code is the majority                                                                                    |
| GC replaces borrow checking      | Violates language design principles—YaoXiang has no GC                                                                                      |

---

## Implementation Phases

| Phase   | Content                                                           | Dependencies              |
| ------- | ----------------------------------------------------------------- | ------------------------- |
| Phase 1 | Brand tree data structure implementation                          | —                         |
| Phase 2 | System predicate generators (Borrow/Move/Drop/Mut → propositions) | Phase 1                   |
| Phase 3 | Reverse BFS liveness analysis + pipeline integration (tier 1)     | Phase 2                   |
| Phase 4 | Path condition collection + SMT logical cut (tier 2)              | Phase 3 + RFC-027 Phase 2 |
| Phase 5 | Release instructions changed to DAG consumer-driven               | Phase 3                   |
| Phase 6 | Remove ControlFlowAnalyzer, refactor BorrowChecker                | Phase 4                   |

---

## Open Questions

- [x] **`ref_count` cross-iteration semantics of the brand tree during loop unrolling**—follow NLL:
      tokens die after last use. Copies bound inside loops die at iteration boundaries, reverse BFS
      does not carry cross-iteration liveness. See §NLL and Iteration Boundaries.
- [x] **Token release order on `?` error propagation path**—Release is driven by scope analysis
      (kept in ir_gen.rs). At each scope exit point (`}`, `?`, explicit return), active tokens are
      released in LIFO order. Brand tree parent-child relationships automatically handle cascade
      release. See §`?` Error Propagation and Scope-Driven Release.
- [ ] Proof function syntax (long-term, not MVP—doesn't block any Phase)

---

## References

- [RFC-009: Ownership Model Design](../accepted/009-ownership-model.md) — Parent RFC
- [RFC-027: Compile-Time Predicates and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md)
  — Proof Pipeline
- [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md) — `{}` Semantics
- [RFC-024: Concurrency Model Based on spawn Blocks](../accepted/024-concurrency-model.md) — spawn
  DAG

---

## Lifecycle and Destination

| Status       | Location                    | Description                      |
| ------------ | --------------------------- | -------------------------------- |
| **Accepted** | `docs/design/rfc/accepted/` | Becomes official design document |
