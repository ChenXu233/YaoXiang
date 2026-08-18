---
title: 'RFC-009a: Token Lifetime Analysis — Hoare Proof Pipeline'
status: 'Accepted'
author: 'Chenxu'
created: '2026-06-13'
updated: '2026-08-17'
group: 'rfc-009'

issue: '#129'

impl: 'partial'
---

# RFC-009a: Token Lifetime Analysis — Hoare Proof Pipeline

> **Parent RFC**: [RFC-009: Ownership Model Design](../accepted/009-ownership-model.md)
>
> **Dependency**:
> [RFC-027: Compile-time Predicates and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md)
>
> **Prerequisite**: RFC-027 must be accepted. All mechanisms in this RFC (proof pipeline, SMT
> fallback, path condition collection) depend on RFC-027's implementation.
>
> **This RFC revises and supersedes RFC-009 §"Token Conflict Detection: Flow-Sensitive Liveness
> Analysis" (lines 663-684).**

## Summary

Line 684 of RFC-009 claims that token conflict detection "does not require...NLL." The conclusion is
correct, the reasoning is wrong.

It is not "because tokens are values, linear tracking suffices." It is because: **token liveness is
a Hoare logic proposition, not a specialized flow-sensitive analysis.**

`{all conflicting_tokens dead} op {WriteToken safely acquired}` — the same `{P} op {Q}`, sharing
RFC-027's proof pipeline with type checking and predicate verification. No new analysis framework.
One pipeline, many propositions.

---

## Motivation

### The Confusion in RFC-009

RFC-009 conflates two problems:

1. **Linear tracking** (unavailable after Move) — `{v not moved} use(v) {type matches}`. The type
   checker already has it.
2. **Token lifetime interaction** (child token alive → parent token paused → child token dies →
   parent token revived) — `{all conflicting_tokens dead} write(data) {safe}`. This requires
   **liveness analysis**, not linear tracking.

### Current Code Reality

| Component                                  | Status                                                                            |
| ------------------------------------------ | --------------------------------------------------------------------------------- |
| `BorrowChecker`                            | Linearly scans IR, passively responds to explicit `Borrow`/`Release` instructions |
| `ControlFlowAnalyzer::analyze_instruction` | Empty implementation (`control_flow.rs:145-153`)                                  |
| `liveness_analysis`                        | Exists but only used for Drop insertion, not connected to token conflicts         |
| Release insertion                          | Hardcoded after Call instructions — purely lexical scope (`ir_gen.rs:2734-2736`)  |

**User-visible consequences**:

```yaoxiang
data = vec![1, 2, 3]
view = &data              # Create ReadToken
x = view.total_count      # Last use of view
data.push(4)              # ❌ Release(view) not yet executed, ReadToken "alive"
```

### Why a Rewrite is Needed

The previous version (009a v1) used a "DAG replacing NLL" narrative, introducing unnecessary new
concepts (conservative branching rules, special loop handling). The core contradiction was not made
clear: **borrow checking is not an independent system — it is a kind of Hoare proposition.**

---

## Core Design

### Everything is Hoare

```
Type checking:    { x: Int }        x + 1        { result: Int }
Borrow checking:  { view dead }     data.push(4)  { WriteToken acquired }
Predicate check:  { y > 0 }         divide(x, y)  { result: Int }
Backedge cut:     { i == n }        next loop     { cond == false }
```

The same form `{P} op {Q}`. The compiler generates a precondition P for each operation and feeds it
into the proof pipeline for verification.

**Borrow checking and user predicates share the same pipeline.** The difference is only in who
generates the proposition and what happens when it fails to prove.

### Two Kinds of Predicates, One Pipeline

|                        | User predicate                     | System predicate (borrow)                      |
| ---------------------- | ---------------------------------- | ---------------------------------------------- |
| Proposition generation | Programmer (type annotations)      | Compiler (brand tree + ownership rules)        |
| Proof provision        | Compiler + programmer              | **Fully automatic compiler**                   |
| Failure to prove       | Write proof function or refactor   | Refactor code (gates remain but rarely needed) |
| Visibility             | Visible in signature               | Implicit, does not pollute type signatures     |
| Learning cost          | Learned only if you want to use it | Zero                                           |

**Proofs for system predicates don't open proof functions for the programmer — the compiler is fully
automatic.** When a proof fails, the user refactors code.

**Three failure modes, one verification engine.** Type propositions fail to prove → compile error
(cannot bypass). Borrow propositions fail to prove → compile error, refactor (cannot bypass). User
predicates fail to prove → compile error, may write proof function (bypassable). Failure strategies
differ, but the verification engine is the same — SMT solver + compiler core inference rules. The
only difference is "who is responsible for filling the proof when it fails to prove" — the compiler
refuses to write borrow proofs for the programmer (the proof strategy for borrow propositions is
structural analysis + SMT, no programmer intervention needed), but accepts programmer-written proof
functions for user predicates. This is not pipeline inconsistency — it is a different responsibility
boundary for different proposition categories.

This differs from Rust `'a`: `'a` is required coursework, proof functions are elective — the vast
majority of users never touch the elective door in their lifetime.

### Borrow Propositions: Auto-Generated by the Compiler

The user writes `data.push(4)`. The compiler auto-generates the proposition:

```
WriteToken(data, node) acquirable
  = forall t in conflicting_tokens(data): t dead at node
  = forall t in brand_tree.children(data): forward_reachable(node) ∩ consumers(t) == ∅
```

**Three rules, zero special cases:**

1. **Brand tree** (RFC-009 §2.7) answers "who conflicts with whom": prefix matching, O(depth), depth
   ≤ 3
2. **Consumer list** (auto-collected during DAG construction) answers "who was the last consumer of
   the token"
3. **Forward reachability** answers "can the consumer still be executed": structural cut + logical
   cut

### Forward Reachability: Reverse-Walk from the Consumer

For each consumer C of token T:

```
Start from C, reverse BFS the DAG.
An edge is cut if:
  1. It is a break (structural cut)
  2. path_condition ⇒ !loop_cond is proven true by SMT (logical cut, RFC-027 pipeline)

Propagate backwards along all uncut edges (including backedges; backedges propagate liveness to the previous iteration).
Mark all reachable nodes → unsafe.
```

Query: write operation at node W → W ∉ unsafe → safe.

**No need to invent "conservative branching rules." No "conservative loop survival." One reverse
BFS + two cut rules.**

### Proof Strategy: Fast Path First, SMT as Safety Net

```
Every write operation that requires a token
  │
  ├→ Fast path: DAG structural analysis (covers 95%+ of cases)
  │     │
  │     ├→ Brand tree prefix match → find conflicting tokens (O(depth))
  │     ├→ Reverse BFS, break cuts backedges
  │     └→ No backedge traversable → directly determine Proved / Disproved
  │
  └→ Slow path: SMT logical cut (only when fast path encounters a traversable backedge)
        │
        ├→ Backedge source has path condition → SMT judges path_cond ⇒ !loop_cond
        │     ├→ Proved → logical cut → downgrade to fast path to continue
        │     └→ Disproved / Unproven → backedge traversed → mark unsafe
        │
        └→ Backedge source has no path condition → backedge traversed directly
```

**Fast path coverage**: linear code, if/else, loop + break, while without path conditions. **Slow
path coverage**: while loop bodies, cases where path conditions imply loop exit. **Not covered**:
runtime conditions not statically provable → backedge traversed → unsafe → compile error (user
refactors).

SMT is not the main force — it is the safety net. Unlike RFC-027's user predicates, where SMT is the
main force, the borrow system predicates use structural analysis as the main force; SMT only fills
the corners where structural analysis is insufficient.

> **Errata (2026-08-17, SMT positioning correction)**: SMT is a **precision layer, not a soundness
> dependency**. The sound judgment of borrow system predicates is fully carried by the fast path
> (interval + reverse BFS + break cut); SMT logical cut only determines "whether legal programs at
> loop boundaries can pass." When SMT is unavailable / times out / not yet implemented (RFC-027
> impl: in_progress), fallback = backedge traversed = conservative rejection, and what should be
> rejected is still rejected. **The conservatism without SMT = reject all borrow+write inside loops,
> at the same level as Rust NLL** (Rust's production-grade borrow checker also has no SMT). SMT
> landing is pure precision gain, not blocking the delivery of the sound mainline.

---

## Use Case Analysis

### Linear Code

```yaoxiang
data = vec![1, 2, 3]        # Node 1
view = &data                # Node 2: consume data, produce ReadToken(#1)
x = view.total_count        # Node 3: consume view (= last consumer of #1)
data.push(4)                # Node 4: requires WriteToken(data)
```

Reverse BFS starts from `view.total_count` (node 3) → node 3 is the last consumer of #1 → node 4 >
node 3 → node 4 not in unsafe → ✅

### if/else: No Special Rules

```yaoxiang
view = &data
if cond {
    use(view)               # then branch consumes view
} else {
    do_something_else()     # doesn't touch view
}
data.push(4)                # last consumer of view is inside if → no consumer after if → ✅
```

if/else is a compound node in the DAG. Internal consumption is attributed to that node. No branch
state merging. No conservative vote. **Whether there is a subsequent consumer, integer comparison.**

> **Clarification (after #264)**: "No branch state merging" only refers to **borrow liveness**
> (reverse BFS over brand consumers). **move state** (variable ownership) is a separate analysis:
> per-CFG-node forward dataflow (#264, NLL/Polonius style), branch join performs **conservative
> meet** (any branch Moved → join Moved), unreachable branches (literal `if false`) do not
> participate. The two are layered: borrow liveness asks "are there subsequent consumers", move
> analysis asks "could the variable have been transferred".

### if/else with Return Value Escape

```yaoxiang
view = &data
result = if cond {
    view                     # view escapes to result
} else {
    something_else
}
use(result)                  # indirect consumption of view
data.push(4)                 # view still has a consumer (use(result))
                             # → push is in unsafe → ❌ correct error
```

view escapes through the return value → `use(result)` is a consumer of view → reverse-walking from
`push` can reach `use(result)` → unsafe.

### Loop: break Cuts Backedge

```yaoxiang
view = &data
loop {
    use(view)                # consumer
    if is_last {
        data.push(4)         # write operation
        break                # ← structural cut
    }
}
```

Reverse BFS from `use(view)` → backedge → walk forward to `data.push(4)` → hit `break` → **cut** →
`data.push(4)` not in unsafe → ✅

Without break:

```yaoxiang
view = &data
loop {
    use(view)
    data.push(4)             # no break cut → backedge traversable → next round's use(view) reachable
                             # → push is in unsafe → ❌ correct error
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
        data.push(4)         # path condition: i == n
    }
}
```

Reverse BFS from `use(view)` → backedge → walk to `data.push(4)` → check path condition `i == n` →
SMT query: `i == n ⇒ !(i < n)`? → Proved → **logical cut** → `data.push(4)` not in unsafe → ✅

> Errata (2026-08-17): `i == n` is the path condition of the write node itself (the `data.push(4)`
> inside the if branch), the judgment target is the write node (see §Path Condition Collection
> Errata Rule 1).

---

## Essence: Brand ID is `'a`

Don't say "we don't need `'a`." Say "`#42` is `'42`."

| Rust                                       | YaoXiang                         | Equivalence                              |
| ------------------------------------------ | -------------------------------- | ---------------------------------------- |
| `'a`                                       | `#42`                            | Compile-time lifetime identifier         |
| `'a: 'b` outlives constraint               | `#42` is prefix of `#42.field_x` | String prefix comparison = partial order |
| NLL liveness propagation (CFG fixed point) | Reverse BFS (DAG)                | Both are reachability computation        |
| Polonius facts                             | SMT logical cut                  | Both are path condition reasoning        |
| Constraint system fixed point solver       | Brand tree prefix match + BFS    | Different encoding, same problem         |

**We didn't invent a new analysis. We just moved `'a` from the type signature layer to the proof
layer.** Brand ID does exactly the same thing as `'a` — marking borrow identity, tracking derivation
relationships, judging conflicts. There is only one difference: `'a` is in the type signature the
user writes; `#42` is inside the compiler.

This is nothing to be ashamed of. Curry-Howard says types are propositions, programs are proofs.
`'a` is not part of the proposition — it is part of the proof strategy. Rust writes the proof
strategy into the proposition signature. We put it back where it belongs.

### What Language Design Constraints Eliminate

| Source of complexity                       | Avoided? | Reason                                                                 |
| ------------------------------------------ | -------- | ---------------------------------------------------------------------- |
| Variable shadowing                         | ✅       | Language forbids it — one name always points to the same thing         |
| Cross-iteration borrowing in for           | ✅       | Each iteration is a new binding — iterations are naturally isolated    |
| `'a` lifetime annotation                   | ✅       | Brand path = `#42.field_x`, compiler-inferred                          |
| Named lifetime + constraint propagation    | ✅       | Brand path prefix comparison replaces explicit constraint sets         |
| Borrow graph constraint solving (Polonius) | ✅       | Brand tree prefix match + DAG consumer query                           |
| Loop body borrow liveness propagation      | ❌       | Same as Rust — handled by reverse BFS + logical cut                    |
| Conditional branch conservatism            | ❌       | Same as Rust — SMT covers provable cases, rest conservatively rejected |

### Why DAG is Feasible

Three language design constraints of YaoXiang make DAG analysis feasible:

- **No variable shadowing** — one name always points to the same thing, no need to track across
  rebindings
- **for creates new bindings each iteration** — iterations are naturally isolated, no
  cross-iteration borrowing
- **Structured concurrency** — task boundaries are clear, no cross-task liveness propagation

These constraints eliminate the main sources of complexity in Rust's CFG fixed-point iteration. It's
not that DAG is "more advanced" than CFG — it's that a simpler language design allows a simpler
analysis.

---

## Detailed Design

### System Predicate List

The compiler auto-generates the following propositions and feeds them into the RFC-027 proof
pipeline:

| System predicate  | Trigger timing                 | Proposition form                              |
| ----------------- | ------------------------------ | --------------------------------------------- |
| `borrow_conflict` | Requires WriteToken(v)         | `forall t ∈ conflicting(v): dead_at(t, node)` |
| `use_after_move`  | Uses variable v                | `¬moved(v)`                                   |
| `use_after_drop`  | Uses variable v                | `¬dropped(v)`                                 |
| `double_drop`     | Drop(v)                        | `¬dropped(v)`                                 |
| `mut_violation`   | Writes to immutable variable v | `is_mut(v)`                                   |

The existing `BorrowChecker`, `MoveChecker`, `DropChecker`, `MutChecker` **become proposition
generators** — not deleted, but change identity. They generate propositions, the pipeline verifies
propositions.

### Brand Tree

The brand mechanism from RFC-009 §2.7 is formalized as a brand tree.

**Token semantics — freeze first, not copy first**:

The essential difference between `&T` and `&mut T` is not "whether it can be copied", but "whether
concurrent writes are allowed":

```
ReadToken(T):  Grants read-only permission, while freezing the source data T — any
              WriteToken(T) is unacquirable during this period. Freezing is the
              primary semantics of ReadToken. Dup (copyable) is a corollary of freezing:
              because the data is frozen (no mutation possible), multiple read-only
              views are naturally safe.

WriteToken(T): Grants exclusive read-write permission. Because writes exist, no other
              token (read or write) can coexist. No Dup (linear type) is a corollary
              of exclusivity.
```

**Causal relationship**:

```
ReadToken exists → source data frozen → multiple read-only copies safe → Dup
                      ↓
              WriteToken rejected (enforced by borrow_conflict system predicate)
```

Not:

```
ReadToken has Dup → can have multiple → incidentally check conflict  ← Causal inversion
```

```
BrandTree:
  nodes: Map<BrandId, BrandNode>

BrandNode:
  id: BrandId               # "#42", "#42.field_x"
  kind: ReadToken | WriteToken
  source_var: Operand
  parent: Option<BrandId>   # Parent node of the derivation relationship
  children: Set<BrandId>    # Derived child tokens
  consumers: Set<NodeId>    # DAG nodes that consume this token
  ref_count: usize          # Safe copy count during ReadToken freeze
```

**Conflict judgment** — the execution mechanism guaranteed by freezing:

```rust
fn conflicts(a: &BrandId, b: &BrandId) -> bool {
    // Conflict conditions: same source + at least one is a write + brand paths overlap
    // This means:
    //   1. ReadToken vs ReadToken → no conflict (both read-only, no mutation)
    //   2. WriteToken vs ReadToken → conflict (write breaks read's freeze guarantee)
    //   3. WriteToken vs WriteToken → conflict (two writes cannot coexist)
    a.source() == b.source()
        && (a.is_write() || b.is_write())
        && (a.is_prefix_of(b) || b.is_prefix_of(a))
}
```

O(depth) string prefix comparison, depth ≤ 3. Constant level.

### Reverse BFS Liveness Analysis

> **Errata (2026-08-17, #251 P0-6 audit)**: The following algorithm adds the "token creation time"
> dimension. The original text relied on the DAG node total order implicitly assuming "borrow before
> write", not covering the legal order of "write first, borrow after" (§2.4 semantics: parameter
> token released when call ends) — the audit measured this scenario as falsely reported (#290).
> Token liveness is an **interval** `[created_at, last_use]`, not a reverse reachability set; a
> write operation only constitutes a conflict within the token's liveness interval.

```
Algorithm: check_borrow(token, node, dag, brand_tree)

Input:
  token: WriteToken to check
  node:  DAG node where the write operation resides

Output: Proved | Disproved

Algorithm:
  # Fast path: reverse BFS
  unsafe = empty_set
  queue = brand_tree.consumers(token)

  while queue not empty:
    cur = queue.pop()
    unsafe.add(cur)

    for each pred in dag.predecessors(cur):
      # Structural cut: break is not traversed
      if pred is a break edge:
        continue

      # Backedge → check if SMT fallback is needed
      if pred is a backedge:
        path_cond = path condition of write node node   # Errata: judgment target is write node's own condition
        loop_cond = loop condition
        # First check structurally whether it can be cut (the corresponding break has already cut the path → won't reach here)
        # Then check path condition
        if path_cond is non-empty:
          result = smt_fallback(path_cond, loop_cond)   # ← slow path
          if result == Proved:
            continue                    # logical cut
        # No path condition or SMT fails to prove → traverse backedge
        # fall through

      if pred ∉ unsafe:
        queue.push(pred)

  # Judgment (errata: add creation time interval)
  # Write first, borrow after: node < created_at(token) → token doesn't exist when write happens → Safe
  if node ∈ unsafe and created_at(token) ≤ node:
    return Disproved
  else:
    return Proved


smt_fallback(path_cond, loop_cond):
  # Only called on backedge + with path condition
  # Uses the RFC-027 proof pipeline, sharing the same SMT solver and budget
  return smt.prove(path_cond ⇒ !loop_cond)
  # Proved → logical cut
  # Disproved / Unproven → no cut, backedge traversed (conservative rejection)
  # Errata (2026-08-17): SMT unavailable/timeout/not implemented = Disproved branch —
  # SMT only affects precision (whether legal programs can pass), not soundness (what should be rejected is rejected);
  # conservatism without SMT = reject all borrow+write inside loops, at the same level as Rust NLL.
```

BrandNode adds a field (errata):

```
BrandNode:
  ...
  created_at: NodeId         # Token creation node (errata: left endpoint of borrow interval)
```

O(N), where SMT call count = number of backedges × proportion of backedges with path conditions. In
actual code, SMT calls are extremely rare — only triggered when a `while` loop body has a refined
type variable's path condition.

### Path Condition Collection

Provided by RFC-027 §3.2-3.3 existing mechanisms:

- **if guard**: `if y > 0` → true branch pushes `y > 0`
- **match pattern**: `if let Some(v) = opt` → branch pushes `opt == Some(v)`
- **Assignment**: `i += 1`, compiler maintains variable value range information
- **while cond**: loop body pushes `cond == true`

Each DAG node carries a path condition set. When reverse BFS hits a backedge, take the path
condition of the backedge source, SMT determines whether it excludes the next loop entry condition.

> **Errata (2026-08-17, propagation rule completion)**: The original text didn't define how path
> conditions propagate to the backedge source, and the use case "while + SMT logical cut"'s `i == n`
> is not reproducible under the original algorithm. Added rules:
>
> 1. **Path condition attaches to the write node itself**: a write operation W inside a branch
>    carries its branch condition (`if i == n { W }` → path_cond(W) = `i == n`). When reverse BFS
>    traverses a backedge, SMT judges `path_cond(W) ⇒ !loop_cond` (the path to W must exit the loop
>    → next round's consumer is unreachable → cut), not the path condition of the backedge node.
> 2. **join conservatively empties**: if/else join points do not carry inside-branch path conditions
>    (after disjunction the two branch conditions usually can't be determined, so empty directly).
>    Write operations after the join have empty path_cond → backedge traversed.
> 3. **Path condition semantization**: path_cond is a ConstExpr (RFC-027 §3.2 semantics), not source
>    text; smt_cut translates it into SMT constraints before solving.
> 4. **No path condition → backedge traversed directly** (unsafe), no SMT call.

### Interface with RFC-027

Borrow system predicates and user predicates share the same proof pipeline — the difference is in
the **main proof strategy**:

| Query type      | Proposition source         | Main strategy                           | Fallback                  |
| --------------- | -------------------------- | --------------------------------------- | ------------------------- |
| Type equality   | Type checker               | Structural equivalence                  | —                         |
| User predicate  | Programmer type annotation | SMT                                     | Programmer proof function |
| Borrow conflict | Compiler auto-generated    | **DAG structural analysis (fast path)** | SMT logical cut           |

The SMT solver's role in borrow checking: **not the main force, but the safety net.** Only called
when a while backedge needs logical cut. The vast majority of borrow checking completes on the fast
path — O(N) reverse BFS, zero SMT overhead.

### Relationship to Existing Code

| Existing component             | Handling                                                                    |
| ------------------------------ | --------------------------------------------------------------------------- |
| `BorrowChecker`                | Becomes `BorrowPredicateEmitter` — generates Hoare propositions for borrows |
| `MoveChecker`                  | Becomes `MovePredicateEmitter` — generates `¬moved(v)` propositions         |
| `DropChecker`                  | Same — generates Drop-related propositions                                  |
| `MutChecker`                   | Same — generates `is_mut(v)` propositions                                   |
| `ControlFlowAnalyzer`          | No longer needed — pipeline handles uniformly                               |
| `liveness_analysis`            | Kept — Drop insertion still needs variable liveness information             |
| `ir_gen.rs` Release hardcoding | Removed — Release position is driven by DAG consumer analysis               |

### NLL and Iteration Boundaries

> **Errata (2026-08-17, interval model completion)**: Token liveness is an **interval**
> `[created_at, last_use]`, not a reverse reachability set. `created_at` = token creation node;
> `last_use` = maximum consumer node from consumer analysis. Sufficient and necessary condition for
> write operation W conflicting with token T: `conflicts(T, W) ∧ created_at(T) ≤ node(W)`
> `∧ node(W) can forward-reach last_use(T)` (judged by reverse BFS). The legal order of "write
> first, borrow after" (§2.4: parameter token released when call ends) is directly excluded by
> `created_at(T) ≤ node(W)`, no special rules needed. This model makes the §Trade-offs Advantage 5
> "algorithm not conservative" claim hold under all orderings.

**Token death moment = last use point (NLL), not lexical scope end.**

This is a natural consequence of consumer analysis: the consumer's position defines the last use of
the token. `use(v)` is a consumer of `v` → `v` dies immediately after `use(v)`. No additional `{}`
or `drop()` needed to end token life early.

**Loop iteration boundary is the death line for token copies.** Three rules:

```
Rule 1: Variables declared inside a loop die automatically at the end of each iteration.
        Each for iteration is a new binding (guaranteed by language design), loop is the same.

Rule 2: Brand tree ref_count at loop header only counts copies created outside the loop.
        New copies produced by Dup inside the loop have ref_count cleared at iteration boundaries.

Rule 3: When reverse BFS traverses a backedge, it does not carry the current iteration's liveness information.
        It only carries the ref_count at the loop header (i.e.: copies from outside the loop).
```

Example:

```yaoxiang
view = &data                          # loop header: ref_count = 1, consumer = use(view)
loop {
    v2: &Point = view                 # Dup inside loop → ref_count = 2
    use(v2)                           # consumer: last use of v2 → v2 dies → ref_count = 1
    data.push(4)                      # ✅ safe! v2 is dead, only view remains (ref_count = 1, not a write conflict)
    # iteration boundary: rule 3 — v2 is not carried into the next round. At the start of the next iteration v2 is recreated by new binding.
}
```

This design doesn't need additional "conservative loop survival" rules. Reverse BFS starts from
consumers; consumers inside the loop body → liveness is confined to the current iteration → backedge
not traversed. Fully consistent with the loop examples in RFC-009a §Use Case Analysis.

### `?` Error Propagation and Scope-Driven Release

`?` is an early return — beyond the normal exit of the scope, there's an additional exit path. The
token must be released on this path; incorrect release order is UB.

**Release instructions are generated by scope analysis, not hardcoded after Call.**

The compiler maintains an exit point list for each scope:

- `}` (normal scope end)
- `?` (error propagation, early return)
- Explicit `return`

At each exit point, Release instructions are inserted in declaration reverse order (LIFO) for all
active tokens in that scope. The parent-child relationship of the brand tree automatically handles
cascading release of derived tokens:

```yaoxiang
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)    # return child token &Float + parent token &Point
}

fn use_case(p: Point) -> Result<(), Error> = {
    (x_ref, p_ref) = p.get_x()?   # If ? propagates:
    # Brand tree knows x_ref is derived from p_ref (#42.field_x is prefix of #42)
    # Release order: x_ref (child) → p_ref (parent) → LIFO automatically satisfied
    p.modify()                     # WriteToken — all ReadTokens released
    Ok(())
}
```

Implementation location: kept in `ir_gen.rs`, changed to scope-driven — no new compiler pass
introduced.

| Conflict judgment | O(1) | Each token request | | DAG consumer query | O(1) | Each token request |
| Reverse BFS (fast path) | O(N) | Each token request, N = nodes in block | | SMT logical cut
(fallback) | ~1ms | **Extremely rare** — only while + path conditions |

> Errata (2026-08-17): The complexity in the table above is a design estimate, not measured; "~1ms"
> and "extremely rare" should be treated as order-of-magnitude expectations rather than measured
> values, to be calibrated with observability data after implementation lands.

**The trigger conditions for SMT fallback are extremely stringent**: must simultaneously satisfy (1)
while loop (2) write operation inside the loop body (3) write operation has a path condition that
can judge loop termination (4) compiler needs to rely on that condition to cut the backedge. In
actual code, the proportion is far less than 1%. All other borrow checking completes on the fast
path.

Relationship to RFC-027 user predicates: user predicates use SMT as the main force, borrow system
predicates use structural analysis as the main force. Both share the same SMT solver and budget cap
(RFC-027 §8), but borrow system predicates almost don't consume SMT budget.

Linear code → no backedge → layer 1 O(N) in seconds. Loop + path condition → SMT call, linear
arithmetic millisecond level (RFC-027 budget 100ms). One BFS result is cacheable for reuse across
multiple queries for the same token.

### Error Message Design

**Core principle: error messages only contain symbols the user has written.**

Rust has two categories of borrow-related errors:

**Variable-level errors**: E0597 (doesn't live long enough), E0502 (mutable + immutable borrow
simultaneously), E0499 (multiple mutable borrows). Rust is already the benchmark — variable name +
line number, no `'a` appears. YaoXiang precision is on par. All information is in the brand tree:
token creation point, consumer location, request point.

**Signature-level errors**: E0623 (lifetime mismatch), E0106 (missing lifetime specifier), E0477
(required lifetime not satisfied). Centered on `'a`. YaoXiang **has no such errors** — no `'a` in
signatures. Not "can't report", but don't report things the user hasn't written.

Example: conflict within a function:

```
Error: `data` is frozen, cannot acquire mutable permission
 --> src/main.yx:5:9
2 |     view = &data
  |            ----- `data` is frozen (read-only token created here)
4 |         use(view)
  |             ---- `view` is still in use here, freeze not released
5 |         data.push(4)
  |         ^^^^ mutable permission required here
```

(On par with Rust E0499 precision — variable name + line number, no brand ID appears.)

Example: escape between functions:

```
Error: `num` (line 4) holds data from one source being `default_str` (line 3),
but `default_str` becomes invalid at line 6, and `num` is still in use at line 5.

Consider: move the `default_str` declaration up to the caller, or use `ref default_str` to share holding.
```

(On par with Rust E0597 precision. The brand digest knows `num` has two source paths — already in
the compiler, the wording can be used.)

---

## RFC-009 Main Text Revision

RFC-009 §"Token Conflict Detection: Flow-Sensitive Liveness Analysis" has been updated:

1. Delete "Things not needed: ...NLL" — not because the conclusion is wrong, but because the
   reasoning is wrong ("tokens are values, linear tracking is enough")
2. Layer 1 / Layer 2 transition plan kept, complete plan points to this RFC
3. Clarify: brand ID (`#42`) is `'a` — completely the same information, different encoding. Not
   inventing a new analysis — moving lifetime from the type layer to the proof layer

---

## Trade-offs

### Advantages

1. **Type signatures contain no lifetime**: `#42` is `'42` — the same information, encoded in the
   brand tree, not exposed in the type signature. This is unfalsifiable: count how many `'a`
   parameters a generic type with 3 reference parameters needs in Rust vs. how many YaoXiang needs.
   The answer is 3 vs 0.

2. **Conceptual unity**: borrow checking and user predicates share the same proof pipeline —
   `{P} op {Q}`, pipeline verifies P. Curry-Howard consistent.

3. **Zero new analysis framework**: no new analysis framework introduced. The user doesn't perceive
   the existence of a "borrow checker" — just as the user doesn't perceive implementation details of
   the "type checker".

4. **Error messages only contain symbols the user has written**: an entire dimension of error
   categories removed (E0623, E0106, E0477 — all around `'a`). Variable-level errors on par with
   Rust precision.

5. **Algorithm not conservative**: reverse BFS + break cut + SMT logical cut. No need for
   "conservative survival inside loops". No need for "conservative branch merging".

### Disadvantages

1. **Not a new invention**: brand ID does exactly the same thing as `'a` — the constraint solving
   complexity inside the compiler hasn't disappeared, only the encoding has changed from "variable
   name + constraint set" to "brand path + prefix match". The difference to the end user is only
   that `'a` is not written in the signature.

2. **Brand new implementation**: the brand tree only exists as a concept in the code, needs to be
   implemented from scratch. BorrowChecker, ControlFlowAnalyzer are replaced.

3. **SMT dependency**: logical cut depends on Z3 (already introduced by RFC-027, no new dependency).
   But borrow checking almost never triggers it — only called on while + path conditions.

4. **A very small number of patterns need refactoring**: borrow system predicates the compiler
   cannot auto-prove across branches, the user needs to refactor code. Different from Rust `'a`'s
   fallback: Rust has `'a` as a tool (annotate and it passes); YaoXiang's fallback (proof function)
   is not MVP.

---

## Alternatives

| Alternative                      | Why not chosen                                                                                                                               |
| -------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| Implement full Rust NLL          | YaoXiang's design constraints (no shadowing, for rebinds) have already eliminated NLL's main complexity sources, no need for CFG fixed point |
| Keep current (hardcoded Release) | Insufficient — users must manually manage token scopes                                                                                       |
| Only analyze inside spawn blocks | Insufficient — token use in non-spawn code is the majority                                                                                   |
| GC instead of borrow checking    | Violates language design principles — YaoXiang has no GC                                                                                     |

---

## Implementation Phases

| Phase   | Content                                                           | Dependency                |
| ------- | ----------------------------------------------------------------- | ------------------------- |
| Phase 1 | Brand tree data structure implementation                          | —                         |
| Phase 2 | System predicate generators (Borrow/Move/Drop/Mut → propositions) | Phase 1                   |
| Phase 3 | Reverse BFS liveness analysis + pipeline integration (layer 1)    | Phase 2                   |
| Phase 4 | Path condition collection + SMT logical cut (layer 2)             | Phase 3 + RFC-027 Phase 2 |
| Phase 5 | Release instructions changed to DAG-consumer-driven               | Phase 3                   |
| Phase 6 | Remove ControlFlowAnalyzer, refactor BorrowChecker                | Phase 4                   |

---

## Open Questions

- [x] **Brand tree `ref_count` cross-iteration semantics during loop unrolling** — Go NLL: the token
      dies after last use. Copies bound inside the loop die at iteration boundaries, reverse BFS
      does not carry liveness across iterations. See §NLL and Iteration Boundaries.
- [x] **Token release order on `?` error propagation path** — Release is driven by scope analysis
      (kept in ir_gen.rs). Each scope exit point (`}`, `?`, explicit return) releases active tokens
      in LIFO. Brand tree parent-child relationships automatically handle cascading release. See
      §`?` Error Propagation and Scope-Driven Release.
- [ ] Proof function syntax (long-term, not MVP — does not block any Phase)

---

## References

- [RFC-009: Ownership Model Design](../accepted/009-ownership-model.md) — Parent RFC
- [RFC-027: Compile-time Predicates and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md)
  — Proof pipeline
- [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md) — `{}` semantics
- [RFC-024: spawn Block-Based Concurrency Model](../accepted/024-concurrency-model.md) — spawn DAG

---

## Lifecycle and Destination

| Status       | Location                    | Description                      |
| ------------ | --------------------------- | -------------------------------- |
| **Accepted** | `docs/design/rfc/accepted/` | Becomes a formal design document |
