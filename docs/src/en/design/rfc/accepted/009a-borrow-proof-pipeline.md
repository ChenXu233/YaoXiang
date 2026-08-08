---
title: 'RFC-009a: Token Lifetime Analysis - Hoare Proof Pipeline'
status: 'Accepted'
author: 'Chenxu'
created: '2026-06-13'
updated: '2026-06-13'
group: 'rfc-009'

issue: '#129'

impl: 'partial'
---

# RFC-009a: Token Lifetime Analysis - Hoare Proof Pipeline

> **Parent RFC**: [RFC-009: Ownership Model Design](../accepted/009-ownership-model.md)
>
> **Dependency**:
> [RFC-027: Compile-time Predicates and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md)
>
> **Prerequisite**: RFC-027 has been accepted. All mechanisms in this RFC (proof pipeline, SMT
> fallback, path condition collection) depend on the implementation of RFC-027.
>
> **This RFC corrects and replaces RFC-009 §"Token Conflict Detection: Flow-Sensitive Liveness
> Analysis" (lines 663-684).**

## Summary

Line 684 of RFC-009 claims that token conflict detection "does not need... NLL". The conclusion is
correct, but the argument is wrong.

It is not "because tokens are values, linear tracking is enough". It is because: **token liveness is
a Hoare logic proposition, not a special flow-sensitive analysis.**

`{conflicting_tokens all dead} op {WriteToken safely acquired}` -- the same `{P} op {Q}`, sharing
RFC-027's proof pipeline with type checking and predicate verification. No new analysis framework.
One pipeline, multiple propositions.

---

## Motivation

### The Confusion in RFC-009

RFC-009 conflates two issues:

1. **Linear Tracking** (unavailable after Move) -- `{v not moved} use(v) {type matches}`. The type
   checker already has this.
2. **Token Lifetime Interaction** (child token alive → parent token paused → child token dead →
   parent token revived) -- `{conflicting_tokens all dead} write(data) {safe}`. Requires **liveness
   analysis**, not linear tracking.

### Current Code State

| Component                                  | Status                                                                              |
| ------------------------------------------ | ----------------------------------------------------------------------------------- |
| `BorrowChecker`                            | Linearly scans IR, passively responding to explicit `Borrow`/`Release` instructions |
| `ControlFlowAnalyzer::analyze_instruction` | Empty implementation (`control_flow.rs:145-153`)                                    |
| `liveness_analysis`                        | Exists but only used for Drop insertion, not connected to token conflicts           |
| Release insertion                          | Hardcoded after Call instructions -- purely lexical scope (`ir_gen.rs:2734-2736`)   |

**User-visible consequences**:

```yaoxiang
data = vec![1, 2, 3]
view = &data              # 创建 ReadToken
x = view.total_count      # view 的最后使用
data.push(4)              # ❌ Release(view) 尚未执行，ReadToken "活着"
```

### Why a Rewrite is Needed

The previous version (009a v1) used the "DAG replaces NLL" narrative, introducing unnecessary new
concepts (conservative branch rules, loop special handling). The core contradiction was not made
clear: **borrow checking is not an independent system -- it is a kind of Hoare proposition.**

---

## Core Design

### Everything is Hoare

```
类型检查：  { x: Int }        x + 1        { result: Int }
借用检查：  { view 已死 }     data.push(4)  { WriteToken 获取成功 }
谓词验证：  { y > 0 }         divide(x, y)  { result: Int }
回边切断：  { i == n }        下一轮循环     { cond == false }
```

The same form `{P} op {Q}`. The compiler generates a precondition proposition P for each operation
and sends it to the proof pipeline for verification.

**Borrow checking and user predicates share the same pipeline.** The only differences are who
generates the propositions and how to handle when the proof fails.

### Two Kinds of Predicates, One Pipeline

|                        | User Predicate                   | System Predicate (Borrow)                        |
| ---------------------- | -------------------------------- | ------------------------------------------------ |
| Proposition generation | Programmer (type annotation)     | Compiler (brand tree + ownership rules)          |
| Proof provision        | Compiler + Programmer            | **Compiler fully automatic**                     |
| Cannot be proven       | Write proof function or refactor | Refactor code (door left open but rarely needed) |
| Visibility             | Visible in signature             | Implicit, does not pollute type signature        |
| Learning cost          | Learn only if you want to use it | Zero                                             |

**Proofs for system predicates do not open proof functions to programmers -- the compiler is fully
automatic.** When proof fails, the user refactors the code.

**Three failure modes, one verification engine.** Type proposition cannot be proven → compile error
(cannot be bypassed). Borrow proposition cannot be proven → compile error, refactor code (cannot be
bypassed). User predicate cannot be proven → compile error, can write proof function (can be
bypassed). The failure strategies differ, but the verification engine is the same -- SMT solver +
compiler core inference rules. The only difference is "who is responsible for filling in the proof
when it cannot be proven" -- the compiler refuses to write borrow proofs for the programmer (the
proof strategy for borrow propositions is structural analysis + SMT, no programmer intervention
needed), but accepts programmer-written user predicate proof functions. This is not pipeline
inconsistency -- it is the responsibility boundary between different proposition categories.

This is different from Rust `'a`: `'a` is a required course, proof functions are elective -- the
vast majority of users never touch the elective.

### Borrow Propositions: Compiler Auto-generated

The user writes `data.push(4)`. The compiler automatically generates the proposition:

```
WriteToken(data, node) 可获取
  = forall t in conflicting_tokens(data): t 在 node 处已死
  = forall t in brand_tree.children(data): forward_reachable(node) ∩ consumers(t) == ∅
```

**Three rules, zero special cases:**

1. **Brand Tree** (RFC-009 §2.7) answers "who conflicts with whom": prefix matching, O(depth), depth
   ≤ 3
2. **Consumer List** (automatically collected during DAG construction) answers "who consumed the
   token last"
3. **Forward Reachability** answers "can the consumer still be executed": structural cut + logical
   cut

### Forward Reachability: Reverse from Consumer

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

**No need to invent "conservative branch rules". No "conservative loop liveness". One reverse BFS +
two cut rules.**

### Proof Strategy: Fast Path First, SMT Fallback

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

**Fast Path Coverage**: linear code, if/else, loop + break, while without path conditions. **Slow
Path Coverage**: inside while loops, when there is a path condition suggesting the loop will exit.
**Not Covered**: runtime conditions cannot be statically proven → cross back edge → unsafe → compile
error (user refactors).

SMT is not the main force -- it is a safety net. Different from RFC-027's user predicates: user
predicates use SMT as the main force; borrow system predicates use structural analysis as the main
force, SMT only fills the corners that structural analysis cannot reach.

---

## Use Case Analysis

### Linear Code

```yaoxiang
data = vec![1, 2, 3]        # 节点 1
view = &data                # 节点 2：消费 data，生产 ReadToken(#1)
x = view.total_count        # 节点 3：消费 view（= #1 的最后一个消费者）
data.push(4)                # 节点 4：需要 WriteToken(data)
```

Reverse BFS starting from `view.total_count` (Node 3) → Node 3 is the last consumer of #1 → Node 4 >
Node 3 → Node 4 is not in unsafe → ✅

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

if/else is a composite node of the DAG. Internal consumption is attributed to this node. No merging
of branch states. No conservative voting. **Whether there is a consumer after, integer comparison.**

> **Clarification (after #264)**: "No merging of branch states" only refers to **borrow liveness**
> (brand consumer reverse BFS). **Move state** (variable ownership) is another analysis:
> per-CFG-node forward data flow (#264, NLL/Polonius style), at branch merge **conservative meet**
> (any branch Moved → merge Moved), literal unreachable branches (`if false`) do not participate.
> The two are layered: borrow liveness looks at "is there a subsequent consumer", move analysis
> looks at "may the variable have been transferred".

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

view escapes through return value → `use(result)` is a consumer of view → reverse walk from `push`
can reach `use(result)` → unsafe.

### Loop: break Cuts Back Edge

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

Reverse BFS from `use(view)` → back edge → forward walk to `data.push(4)` → encounter `break` →
**cut** → `data.push(4)` is not in unsafe → ✅

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
SMT query: `i == n ⇒ !(i < n)`? → Proved → **logical cut** → `data.push(4)` is not in unsafe → ✅

---

## Essence: Brand ID is `'a`

Not "we don't need `'a`". Say "`#42` is `'42`".

| Rust                                       | YaoXiang                             | Equivalence                              |
| ------------------------------------------ | ------------------------------------ | ---------------------------------------- |
| `'a`                                       | `#42`                                | Compile-time lifetime identifier         |
| `'a: 'b` outlives constraint               | `#42` is the prefix of `#42.field_x` | String prefix comparison = partial order |
| NLL liveness propagation (CFG fixed point) | Reverse BFS (DAG)                    | Both are reachability computations       |
| Polonius facts                             | SMT logical cut                      | Both are path condition reasoning        |
| Constraint system fixed point solving      | Brand tree prefix matching + BFS     | Different encoding, same problem         |

**We did not invent new analysis. We just moved `'a` from the type signature layer to the proof
layer.** What brand ID does is exactly the same as `'a` -- marking borrow identity, tracking
derivation relationships, judging conflicts. There is only one difference: `'a` is in the
user-written type signature; `#42` is inside the compiler.

This is not embarrassing. Curry-Howard says types are propositions, programs are proofs. `'a` is not
part of the proposition -- it is part of the proof strategy. Rust wrote the proof strategy into the
proposition signature. We put it back where it should be.

### What the Language Design Constraints Eliminate

| Source of Complexity                       | Avoided? | Reason                                                             |
| ------------------------------------------ | -------- | ------------------------------------------------------------------ |
| Variable shadowing                         | ✅       | Language forbids -- a name always points to the same thing         |
| for cross-iteration borrow                 | ✅       | New binding each iteration -- natural isolation between iterations |
| `'a` lifetime annotation                   | ✅       | Brand path = `#42.field_x`, compiler infers                        |
| Named lifetime + constraint propagation    | ✅       | Brand path prefix comparison replaces explicit constraint set      |
| Borrow graph constraint solving (Polonius) | ✅       | Brand tree prefix matching + DAG consumer query                    |
| Loop body borrow liveness propagation      | ❌       | Same as Rust, needs to be handled -- use reverse BFS + logical cut |
| Conditional branch conservatism            | ❌       | Same as Rust -- SMT covers provable, rest conservatively rejected  |

### Why DAG is Feasible

YaoXiang's three language design constraints make DAG analysis feasible:

- **No variable shadowing** -- a name always points to the same thing, no need to track across
  rebinding
- **for new binding each iteration** -- natural isolation between iterations, no cross-iteration
  borrow
- **Structured concurrency** -- clear task boundaries, no cross-task liveness propagation

These constraints eliminate the main sources of complexity in Rust's CFG fixed point iteration. It
is not that DAG is "more advanced" than CFG -- it is that a simpler language design allows simpler
analysis.

---

## Detailed Design

### System Predicate List

The compiler automatically generates the following propositions and sends them to the RFC-027 proof
pipeline:

| System Predicate  | Trigger Timing              | Proposition Form                              |
| ----------------- | --------------------------- | --------------------------------------------- |
| `borrow_conflict` | Needs WriteToken(v)         | `forall t ∈ conflicting(v): dead_at(t, node)` |
| `use_after_move`  | Uses variable v             | `¬moved(v)`                                   |
| `use_after_drop`  | Uses variable v             | `¬dropped(v)`                                 |
| `double_drop`     | Drop(v)                     | `¬dropped(v)`                                 |
| `mut_violation`   | Writes immutable variable v | `is_mut(v)`                                   |

The existing `BorrowChecker`, `MoveChecker`, `DropChecker`, `MutChecker` **become proposition
generators** -- not disappearing, changing identity. They generate propositions, the pipeline
verifies them.

### Brand Tree

The brand mechanism of RFC-009 §2.7 is formalized as a brand tree.

**Token Semantics -- Freeze First, Not Copy First**:

The essential difference between `&T` and `&mut T` is not "can they be copied", but "is simultaneous
write allowed":

```
ReadToken(T)： 授予只读权限，同时冻结源数据 T——任何 WriteToken(T) 在此期间
              不可获取。冻结是 ReadToken 的首要语义。Dup（可复制）是冻结的推论：
              因为数据已被冻结（无突变可能），多份只读视图天然安全。

WriteToken(T)：授予独占读写权限。因为存在写，任何其他令牌（读或写）都不可共存。
              不实现 Dup（线性类型）是独占的推论。
```

**Causal Relationship**:

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

**Conflict Judgment -- Enforcement Mechanism of Freeze Guarantee**:

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

O(depth) string prefix comparison, depth ≤ 3. Constant level.

### Reverse BFS Liveness Analysis

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

O(N), where the number of SMT calls = back edge count × proportion of back edges with path
conditions. SMT calls are very rare in actual code -- only triggered inside `while` loops with path
conditions on refined type variables.

### Path Condition Collection

Provided by existing mechanisms in RFC-027 §3.2-3.3:

- **if guard**: `if y > 0` → true branch pushes `y > 0`
- **match pattern**: `if let Some(v) = opt` → branch pushes `opt == Some(v)`
- **Assignment**: `i += 1`, compiler maintains variable value range information
- **while cond**: inside loop body pushes `cond == true`

Each DAG node carries a path condition set. When reverse BFS encounters a back edge, take the path
condition at the back edge's start, SMT judges whether to exclude the next loop entry condition.

### Interface with RFC-027

Borrow system predicates and user predicates share the same proof pipeline -- the difference is in
**main proof strategy**:

| Query Type      | Proposition Source         | Main Strategy                           | Fallback                  |
| --------------- | -------------------------- | --------------------------------------- | ------------------------- |
| Type equality   | Type checker               | Structural equivalence                  | —                         |
| User predicate  | Programmer type annotation | SMT                                     | Programmer proof function |
| Borrow conflict | Compiler auto-generated    | **DAG structural analysis (fast path)** | SMT logical cut           |

SMT solver's role in borrow checking: **not the main force, is the safety net.** Only called when
while back edge requires logical cut. The vast majority of borrow checking is done on the fast path
-- O(N) reverse BFS, zero SMT overhead.

### Relationship with Existing Code

| Existing Component             | Handling                                                                       |
| ------------------------------ | ------------------------------------------------------------------------------ |
| `BorrowChecker`                | Becomes `BorrowPredicateEmitter` -- generates Hoare propositions for borrowing |
| `MoveChecker`                  | Becomes `MovePredicateEmitter` -- generates `¬moved(v)` proposition            |
| `DropChecker`                  | Same as above -- generates Drop-related propositions                           |
| `MutChecker`                   | Same as above -- generates `is_mut(v)` proposition                             |
| `ControlFlowAnalyzer`          | No longer needed -- pipeline handles uniformly                                 |
| `liveness_analysis`            | Retained -- Drop insertion still needs variable liveness information           |
| `ir_gen.rs` Release hardcoding | Removed -- Release position is driven by DAG consumer analysis                 |

### NLL and Iteration Boundaries

**Token death time = last use point (NLL), not end of lexical scope.**

This is a natural inference of consumer analysis: the consumer's position defines the token's last
use. `use(v)` is a consumer of `v` → `v` dies immediately after `use(v)`. No need for additional
`{}` or `drop()` to end the token's life early.

**Loop iteration boundary is the death line of token copies.** Three rules:

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

This design does not need additional "conservative loop liveness" rules. Reverse BFS starts from the
consumer, the consumer is inside the loop body → liveness is restricted to the current iteration →
back edge does not cross. Consistent with the loop example in RFC-009a §Use Case Analysis.

### `?` Error Propagation and Scope-Driven Release

`?` is an early return -- an extra exit path beyond the scope's normal exit. Tokens must be released
on this path, and incorrect release order is UB.

**Release instructions are generated by scope analysis, not hardcoded after Call.**

The compiler maintains a list of exit points for each scope:

- `}` (scope normal end)
- `?` (error propagation, early return)
- Explicit `return`

At each exit point, insert Release instructions for all active tokens in the scope in declaration
reverse order (LIFO). The parent-child relationship of the brand tree automatically handles cascade
release of derived tokens:

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

Implementation location: retained in `ir_gen.rs`, changed to scope-driven -- no new compiler pass
introduced.

| Operation                    | Complexity | Trigger Frequency                                         |
| ---------------------------- | ---------- | --------------------------------------------------------- |
| Brand tree conflict judgment | O(1)       | Each time a token is needed                               |
| DAG consumer query           | O(1)       | Each time a token is needed                               |
| Reverse BFS (fast path)      | O(N)       | Each time a token is needed, N = number of nodes in block |
| SMT logical cut (fallback)   | ~1ms       | **Very rare** -- only while + path conditions             |

**The trigger conditions for SMT fallback are extremely strict**: simultaneously satisfy (1) while
loop (2) write operation inside the loop body (3) path condition after the write operation that can
determine loop termination (4) compiler needs to rely on this condition to cut the back edge. The
proportion in actual code is far less than 1%. The rest of the borrow checking is all done on the
fast path.

Relationship with RFC-027 user predicates: user predicates use SMT as the main force, borrow system
predicates use structural analysis as the main force. The two share the same SMT solver and budget
limit (RFC-027 §8), but borrow system predicates hardly consume SMT budget.

Linear code → no back edge → tier 1 O(N) instant. Loop + path condition → SMT call, linear
arithmetic in milliseconds (RFC-027 budget 100ms). One BFS result can be cached for multiple queries
on the same token.

### Error Message Design

**Core principle: error messages only contain symbols the user has written.**

Rust's borrow-related errors are divided into two categories:

**Variable-level errors**: E0597 (doesn't live long enough), E0502 (mutable+immutable simultaneous
borrow), E0499 (multiple mutable borrows). Rust is already the benchmark -- variable name + line
number, no `'a` appears. YaoXiang has equal precision. All information is in the brand tree: token
creation point, consumer location, request point.

**Signature-level errors**: E0623 (lifetime mismatch), E0106 (missing lifetime specifier), E0477
(required lifetime not satisfied). Built around `'a`. YaoXiang **does not have such errors** -- no
`'a` in signatures. Not "cannot report", but things the user did not write are not reported.

Intra-function conflict example:

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

(Equal to Rust E0499 precision -- variable name + line number, no brand ID appears.)

Inter-function escape example:

```
错误：`num`（第 4 行）持有的数据来源之一是 `default_str`（第 3 行），
但 `default_str` 在第 6 行失效，`num` 在第 5 行仍在被使用。

考虑：将 `default_str` 的声明提前到调用方，或使用 `ref default_str` 共享持有。
```

(Equal to Rust E0597 precision. The brand summary knows that `num` has two source paths -- already
in the compiler, error wording is available.)

---

## RFC-009 Main Text Correction

RFC-009 §"Token Conflict Detection: Flow-Sensitive Liveness Analysis" has been updated:

1. Delete "things not needed: ... NLL" -- not because the conclusion is wrong, but because the
   reason is wrong ("tokens are values, linear tracking is enough")
2. Tier 1/tier 2 transition plan retained, complete plan points to this RFC
3. Clarify: brand ID (`#42`) is `'a` -- the information is exactly the same, the encoding is
   different. Not inventing a new analysis -- moving lifetimes from the type layer to the proof
   layer

---

## Trade-offs

### Advantages

1. **Type signatures do not contain lifetimes**: `#42` is `'42` -- the same information, encoded in
   the brand tree, not exposed in the type signature. This point is irrefutable: count how many `'a`
   parameters a Rust generic type with 3 reference parameters needs, and how many YaoXiang needs.
   The answer is 3 vs 0.
2. **Conceptual unification**: borrow checking and user predicates share the same proof pipeline --
   `{P} op {Q}`, pipeline verifies P. Curry-Howard consistent.
3. **Zero new analysis framework**: no new analysis framework introduced. Users do not perceive the
   existence of the "borrow checker" -- just as users do not perceive the implementation details of
   the "type checker".
4. **Error messages only contain symbols the user wrote**: one entire dimension of error categories
   is missing (E0623, E0106, E0477 -- all around `'a`). Variable-level errors have the same
   precision as Rust.
5. **Algorithm is not conservative**: reverse BFS + break cut + SMT logical cut. No need for
   "conservative liveness inside loops". No need for "conservative branch merge".

### Disadvantages

1. **Not a new invention**: what brand ID does is exactly the same as `'a` -- the constraint solving
   complexity inside the compiler has not disappeared, only the encoding method has changed from
   "variable name + constraint set" to "brand path + prefix matching". The difference for end users
   is only that `'a` is not written in signatures.
2. **Brand new implementation**: the brand tree only exists as a concept in the code, needs to be
   implemented from scratch. BorrowChecker and ControlFlowAnalyzer are replaced.
3. **SMT dependency**: logical cut depends on Z3 (already introduced in RFC-027, no new dependency).
   But borrow checking is rarely triggered -- only called when while + path conditions.
4. **Very few patterns require refactoring**: cross-branch borrows that the compiler's automatic
   proof cannot cover require users to refactor code. Different from Rust `'a` fallback: Rust has
   `'a` as a pen (annotate and pass); YaoXiang's fallback (proof functions) is not MVP.

---

## Alternative Plans

| Plan                             | Why not chosen                                                                                                                         |
| -------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| Implement complete Rust NLL      | YaoXiang's design constraints (no shadowing, for new binding) have eliminated main NLL complexity sources, no need for CFG fixed point |
| Keep current (hardcoded Release) | Not enough -- users must manually manage token scopes                                                                                  |
| Only analyze in spawn blocks     | Not enough -- token usage in non-spawn code is the majority                                                                            |
| GC replaces borrow checking      | Violates language design principles -- YaoXiang has no GC                                                                              |

---

## Implementation Phases

| Phase   | Content                                                          | Dependency                |
| ------- | ---------------------------------------------------------------- | ------------------------- |
| Phase 1 | Brand tree data structure implementation                         | —                         |
| Phase 2 | System predicate generator (Borrow/Move/Drop/Mut → propositions) | Phase 1                   |
| Phase 3 | Reverse BFS liveness analysis + pipeline integration (tier 1)    | Phase 2                   |
| Phase 4 | Path condition collection + SMT logical cut (tier 2)             | Phase 3 + RFC-027 Phase 2 |
| Phase 5 | Release instructions changed to DAG consumer-driven              | Phase 3                   |
| Phase 6 | Delete ControlFlowAnalyzer, refactor BorrowChecker               | Phase 4                   |

---

## Open Questions

- [x] **Brand tree's `ref_count` cross-iteration semantics during loop unrolling** -- take NLL: the
      token dies after the last use. Copies bound inside the loop die at the iteration boundary,
      reverse BFS does not carry cross-iteration liveness. See §NLL and Iteration Boundaries.
- [x] **Token release order on `?` error propagation path** -- Release is driven by scope analysis
      (retained in ir_gen.rs). Each scope exit point (`}`, `?`, explicit return) releases active
      tokens in LIFO. The brand tree parent-child relationship automatically handles cascade
      release. See §`?` Error Propagation and Scope-Driven Release.
- [ ] Proof function syntax (long-term, not MVP -- does not block any Phase)

---

## References

- [RFC-009: 所有权模型设计](../accepted/009-ownership-model.md) — Parent RFC
- [RFC-027: 编译期谓词与统一静态验证](../accepted/027-compile-time-evaluation-types.md) — Proof
  pipeline
- [RFC-010: 统一类型语法](../accepted/010-unified-type-syntax.md) — `{}` semantics
- [RFC-024: 基于 spawn 块的并发模型](../accepted/024-concurrency-model.md) — spawn DAG

---

## Lifecycle and Destination

| Status       | Location                    | Description                      |
| ------------ | --------------------------- | -------------------------------- |
| **Accepted** | `docs/design/rfc/accepted/` | Becomes official design document |
