---
title: 'RFC-009a: Token Lifetime Analysis — A Hoare Proof Pipeline'
status: 'Accepted'
author: 'Chenxu'
created: '2026-06-13'
updated: '2026-08-17'
group: 'rfc-009'

issue: '#129'

impl: 'partial'
---

# RFC-009a: Token Lifetime Analysis — A Hoare Proof Pipeline

> **Parent RFC**: [RFC-009: Ownership Model Design](009-ownership-model.md)
>
> **Dependency**:
> [RFC-027: Compile-Time Predicates and Unified Static Verification](027-compile-time-evaluation-types.md)
>
> **Prerequisite**: RFC-027 must be accepted. All mechanisms in this RFC (the proof pipeline, SMT
> fallback, path condition collection) depend on RFC-027's implementation.
>
> **This RFC corrects and replaces RFC-009 §"Token Conflict Detection: Flow-Sensitive Liveness
> Analysis" (lines 663-684).**

## Summary

RFC-009 line 684 claimed token conflict detection "does not need... NLL". The conclusion is correct;
the argument is wrong.

It is not "because tokens are values, so linear tracking is enough". It is because: **token liveness
is a Hoare logic proposition, not a specialized flow-sensitive analysis.**

`{all conflicting_tokens dead} op {WriteToken safely acquired}` — the same `{P} op {Q}`, sharing
RFC-027's proof pipeline with type checking and predicate verification. No new analysis framework.
One pipeline, many propositions.

---

## Motivation

### The Conflation in RFC-009

RFC-009 conflates two problems:

1. **Linear tracking** (unusable after Move) — `{v not moved} use(v) {type matches}`. The type
   checker already has this.
2. **Token lifetime interaction** (child token alive → parent token paused → child token dead →
   parent token revived) — `{all conflicting_tokens dead} write(data) {safe}`. This requires
   **liveness analysis**, not linear tracking.

### The Current State of the Code

| Component                                  | Status                                                                                |
| ------------------------------------------ | ------------------------------------------------------------------------------------- |
| `BorrowChecker`                            | Linearly scans the IR, passively responds to explicit `Borrow`/`Release` instructions |
| `ControlFlowAnalyzer::analyze_instruction` | Empty implementation (`control_flow.rs:145-153`)                                      |
| `liveness_analysis`                        | Exists but only used for Drop insertion, not wired into token conflicts               |
| Release insertion                          | Hardcoded after Call instructions — purely lexical scope (`ir_gen.rs:2734-2736`)      |

**User-visible consequences**:

```yaoxiang
data = vec![1, 2, 3]
view = &data              # Create ReadToken
x = view.total_count      # Final use of view
data.push(4)              # ❌ Release(view) hasn't run yet, ReadToken is "alive"
```

### Why a Rewrite is Needed

The previous version (009a v1) used a "DAG replaces NLL" narrative, which introduced unnecessary new
concepts (conservative branching rules, special loop handling). The core contradiction was not made
clear: **borrow checking is not an independent system — it is a kind of Hoare proposition.**

---

## Core Design

### Everything is Hoare

```
Type checking:   { x: Int }         x + 1         { result: Int }
Borrow checking: { view dead }      data.push(4)  { WriteToken acquired }
Predicate check: { y > 0 }          divide(x, y)  { result: Int }
Backedge cut:    { i == n }         next loop iter { cond == false }
```

The same form `{P} op {Q}`. The compiler generates a precondition P for each operation and feeds it
into the proof pipeline for verification.

**Borrow checking and user predicates share the same pipeline.** The only difference is who
generates the proposition and what happens when proof fails.

### Two Kinds of Predicates, One Pipeline

|                        | User Predicates                    | System Predicates (borrow)                   |
| ---------------------- | ---------------------------------- | -------------------------------------------- |
| Proposition generation | Programmer (type annotations)      | Compiler (brand tree + ownership rules)      |
| Proof provision        | Compiler + programmer              | **Fully automatic by compiler**              |
| On failure             | Write a proof function or refactor | Refactor code (gate stays but rarely needed) |
| Visibility             | Visible in signatures              | Implicit, does not pollute type signatures   |
| Learning cost          | Learn when you need it             | Zero                                         |

**Proofs for system predicates don't open a proof function to the programmer — the compiler does it
fully automatically.** When proof fails, the user refactors.

**Three failure modes, one verification engine.** Type propositions fail to prove → compile error
(cannot be bypassed). Borrow propositions fail to prove → compile error, refactor (cannot be
bypassed). User predicates fail to prove → compile error, can write proof function (can be
bypassed). The failure strategies differ, but the verification engine is the same — an SMT solver +
compiler kernel inference rules. The only difference is "who fills in the missing proof when proof
fails" — the compiler refuses to write borrow proofs for the programmer (the proof strategy for
borrow propositions is structural analysis + SMT, with no programmer intervention needed), but
accepts programmer-written user-predicate proof functions. This is not a pipeline inconsistency — it
is a different responsibility boundary for different proposition classes.

This differs from Rust `'a`: `'a` is the required course; proof functions are the elective — the
vast majority of users never touch the elective's gate.

### Borrow Propositions: Compiler-Generated Automatically

The user writes `data.push(4)`. The compiler automatically generates the proposition:

```
WriteToken(data, node) acquirable
  = forall t in conflicting_tokens(data): t is dead at node
  = forall t in brand_tree.children(data): forward_reachable(node) ∩ consumers(t) == ∅
```

**Three rules, zero special cases:**

1. **Brand tree** (RFC-009 §2.7) answers "who conflicts with whom": prefix matching, O(depth), depth
   ≤ 3
2. **Consumer list** (collected automatically during DAG construction) answers "who last consumed
   the token"
3. **Forward reachability** answers "can the consumer still be executed": structural cuts + logical
   cuts

### Forward Reachability: Walking Backward from the Consumer

For each consumer C of token T:

```
Starting from C, run a reverse BFS over the DAG.
An edge is cut if:
  1. It is a break (structural cut)
  2. path_condition ⇒ !loop_cond is proved by SMT (logical cut, RFC-027 pipeline)

Propagate backward along all uncut edges (including backedges; backedges propagate liveness into the previous iteration).
Mark all reachable nodes → unsafe.
```

Query: write operation at node W → W ∉ unsafe → safe.

**No need to invent "conservative branching rules". No need for "conservative loop liveness". One
reverse BFS + two cut rules.**

### Proof Strategy: Fast Path First, SMT as Fallback

```
Every write operation that needs a token
  │
  ├→ Fast path: DAG structural analysis (covers 95%+ of cases)
  │     │
  │     ├→ Brand tree prefix matching → find conflicting tokens (O(depth))
  │     ├→ Reverse BFS, break cuts backedges
  │     └→ No backedge traversal → direct Proved / Disproved
  │
  └→ Slow path: SMT logical cut (only when the fast path hits a traversable backedge)
        │
        ├→ Backedge start has a path condition → SMT checks path_cond ⇒ !loop_cond
        │     ├→ Proved → logical cut → fall back to fast path and continue
        │     └→ Disproved / Unproven → backedge traversed → mark unsafe
        │
        └→ Backedge start has no path condition → backedge traversed directly
```

**Fast path coverage**: linear code, if/else, loop + break, while without path conditions. **Slow
path coverage**: while loop bodies where a path condition implies the loop will exit. **Not
covered**: runtime conditions that cannot be statically proved → backedge traversed → unsafe →
compile error (user refactors).

SMT is not the main force — it is the safety net. Unlike RFC-027's user predicates where SMT is the
main force, borrow system predicates rely on structural analysis as the main force, and SMT only
fills the corners that structural analysis cannot reach.

**SMT is a precision layer, not a soundness dependency.** The sound judgment of borrow system
predicates is entirely carried by the fast path (interval + reverse BFS + break cuts); SMT logical
cuts only determine "whether legal programs at loop boundaries can pass". When SMT is unavailable /
times out / not implemented (RFC-027 impl: in_progress), the fallback = backedge traversal =
conservative rejection, and everything that should be rejected still is. **The conservativeness
without SMT = any borrow + write inside a loop is rejected, on par with Rust NLL** (Rust's
production-grade borrow checker also has no SMT). SMT landing is a pure precision gain and does not
block delivery of the sound mainline.

---

## Use-Case Analysis

### Linear Code

```yaoxiang
data = vec![1, 2, 3]        # Node 1
view = &data                # Node 2: consumes data, produces ReadToken(#1)
x = view.total_count        # Node 3: consumes view (= last consumer of #1)
data.push(4)                # Node 4: needs WriteToken(data)
```

Reverse BFS starts from `view.total_count` (node 3) → node 3 is the last consumer of #1 → node 4 >
node 3 → node 4 is not in unsafe → ✅

### if/else: No Special Rules

```yaoxiang
view = &data
if cond {
    use(view)               # then branch consumes view
} else {
    do_something_else()     # does not touch view
}
data.push(4)                # last consumer of view is inside if → no consumers after if → ✅
```

if/else is a compound node in the DAG. Internal consumption is attributed to this node. No merging
of branch state. No conservative voting. **Count whether there is a later consumer; integer
comparison.**

> **Clarification**: "No merging of branch state" refers only to **borrow liveness** (reverse BFS
> over brand consumers). **move state** (variable ownership) is a separate analysis: per-CFG-node
> forward data flow (NLL/Polonius style), with a **conservative meet at branch join** (any branch
> Moved → join Moved), where literally unreachable branches (`if false`) do not participate. The two
> are layered: borrow liveness asks "are there later consumers"; move analysis asks "might the
> variable have been transferred".

### if/else with Return-Value Escape

```yaoxiang
view = &data
result = if cond {
    view                     # view escapes into result
} else {
    something_else
}
use(result)                  # indirect consumption of view
data.push(4)                 # view still has a consumer (use(result))
                             # → push is in unsafe → ❌ Correctly errors
```

view escapes through the return value → `use(result)` is a consumer of view → reverse BFS from
`push` can reach `use(result)` → unsafe.

### Loops: break Cuts the Backedge

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

Reverse BFS from `use(view)` → backedge → walks forward to `data.push(4)` → hits `break` → **cut** →
`data.push(4)` is not in unsafe → ✅

Without break:

```yaoxiang
view = &data
loop {
    use(view)
    data.push(4)             # no break cut → backedge traversable → next iteration's use(view) is reachable
                             # → push is in unsafe → ❌ Correctly errors
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

Reverse BFS from `use(view)` → backedge → reaches `data.push(4)` → check path condition `i == n` →
SMT query: does `i == n ⇒ !(i < n)` hold? → Proved → **logical cut** → `data.push(4)` is not in
unsafe → ✅

Note that the judgment target is the **path condition of the write node itself** (`i == n` belongs
to `data.push(4)` inside the `if` branch), not the path condition of the backedge node.

---

## The Essence: Brand ID Is `'a`

Not "we don't need `'a`". Say "`#42` is `'42`".

| Rust                                       | YaoXiang                           | Equivalence                              |
| ------------------------------------------ | ---------------------------------- | ---------------------------------------- |
| `'a`                                       | `#42`                              | Compile-time lifetime identifier         |
| `'a: 'b` outlives constraint               | `#42` is a prefix of `#42.field_x` | String prefix comparison = partial order |
| NLL liveness propagation (CFG fixed point) | Reverse BFS (DAG)                  | Both are reachability computations       |
| Polonius facts                             | SMT logical cut                    | Both are path condition reasoning        |
| Constraint system fixed-point solving      | Brand tree prefix matching + BFS   | Different encoding, same problem         |

**We invented no new analysis. We merely lowered `'a` from the type-signature layer to the proof
layer.** What a brand ID does is exactly what `'a` does — mark borrow identity, track derivations,
determine conflicts. The only difference is: `'a` lives in the type signature the user writes; `#42`
lives inside the compiler.

This is nothing to be ashamed of. Curry-Howard says types are propositions and programs are proofs.
`'a` is not part of the proposition — it is part of the proof strategy. Rust wrote the proof
strategy into the proposition signature. We put it back where it belongs.

### What Language-Design Constraints Eliminated

| Source of Complexity                       | Avoided? | Reason                                                                        |
| ------------------------------------------ | -------- | ----------------------------------------------------------------------------- |
| Variable shadowing                         | ✅       | Language forbids it — a name always points to the same thing                  |
| Cross-iteration borrows in for             | ✅       | Each iteration creates a new binding — iterations are naturally isolated      |
| `'a` lifetime annotations                  | ✅       | Brand path = `#42.field_x`, compiler-inferred                                 |
| Named lifetimes + constraint propagation   | ✅       | Brand path prefix comparison replaces explicit constraint sets                |
| Borrow graph constraint solving (Polonius) | ✅       | Brand tree prefix matching + DAG consumer query                               |
| Loop-body borrow liveness propagation      | ❌       | Same as Rust — handled with reverse BFS + logical cuts                        |
| Conditional-branch conservativeness        | ❌       | Same as Rust — SMT covers what can be proved, rest is conservatively rejected |

### Why DAG is Feasible

Three language-design constraints of YaoXiang make DAG analysis feasible:

- **No variable shadowing** — a name always points to the same thing, no need to track across
  rebindings
- **for rebinds each iteration** — iterations are naturally isolated, no cross-iteration borrows
- **Structured concurrency** — task boundaries are clear, no cross-task liveness propagation

These constraints eliminate the main sources of complexity in Rust's CFG fixed-point iteration. It
is not that DAG is "more advanced" than CFG — it is that a simpler language design permits a simpler
analysis.

---

## Detailed Design

### System Predicate Inventory

The compiler automatically generates the following propositions and feeds them into the RFC-027
proof pipeline:

| System Predicate  | Trigger Moment       | Proposition Form                              |
| ----------------- | -------------------- | --------------------------------------------- |
| `borrow_conflict` | Needs WriteToken(v)  | `forall t ∈ conflicting(v): dead_at(t, node)` |
| `use_after_move`  | Uses variable v      | `¬moved(v)`                                   |
| `use_after_drop`  | Uses variable v      | `¬dropped(v)`                                 |
| `double_drop`     | Drop(v)              | `¬dropped(v)`                                 |
| `mut_violation`   | Write to immutable v | `is_mut(v)`                                   |

The existing `BorrowChecker`, `MoveChecker`, `DropChecker`, `MutChecker` **become proposition
generators** — they don't disappear, they change identity. They generate propositions; the pipeline
verifies them.

### Brand Tree

RFC-009 §2.7's brand mechanism formalized as a brand tree.

**Token semantics — freeze-first, not copy-first**:

The essential difference between `&T` and `&mut T` is not "can it be copied" but "does it allow
concurrent writes":

```
ReadToken(T):  grants read-only permission while freezing the source data T —
              any WriteToken(T) cannot be acquired during this period. Freezing
              is the primary semantics of ReadToken. Dup (copyable) is a corollary
              of freezing: because the data is already frozen (no mutation possible),
              multiple read-only views are naturally safe.

WriteToken(T): grants exclusive read-write permission. Because a write exists,
              no other token (read or write) can coexist. Not implementing Dup
              (linear type) is a corollary of exclusivity.
```

**Causal chain**:

```
ReadToken exists → source data frozen → multiple read-only views safe → Dup
                          ↓
              WriteToken rejected (borrow_conflict system predicate enforces)
```

Not:

```
ReadToken has Dup → can have multiple → incidentally check conflicts  ← inverted causality
```

```
BrandTree:
  nodes: Map<BrandId, BrandNode>

BrandNode:
  id: BrandId               # "#42", "#42.field_x"
  kind: ReadToken | WriteToken
  source_var: Operand
  parent: Option<BrandId>   # Parent node in derivation relationship
  children: Set<BrandId>    # Derived child tokens
  consumers: Set<NodeId>    # DAG nodes that consume this token
  ref_count: usize          # Number of safe copies during ReadToken freeze
```

**Conflict judgment — enforcement mechanism of the freeze guarantee**:

```rust
fn conflicts(a: &BrandId, b: &BrandId) -> bool {
    // Conflict conditions: same source + at least one is a write + brand paths overlap
    // This means:
    //   1. ReadToken vs ReadToken → no conflict (both read-only, no mutation)
    //   2. WriteToken vs ReadToken → conflict (write breaks the read's freeze guarantee)
    //   3. WriteToken vs WriteToken → conflict (two writes cannot coexist)
    a.source() == b.source()
        && (a.is_write() || b.is_write())
        && (a.is_prefix_of(b) || b.is_prefix_of(a))
}
```

O(depth) string prefix comparison, depth ≤ 3. Constant-level.

### Reverse BFS Liveness Analysis

This algorithm introduces a "token creation time" dimension. Token liveness is an **interval**
`[created_at, last_use]`, not a reverse-reachability set; a write operation only constitutes a
conflict within the token's liveness interval — this covers the legal "write first, borrow later"
order (§2.4 semantics: parameter tokens release when the call ends), avoiding false positives.

```
Algorithm: check_borrow(token, node, dag, brand_tree)

Input:
  token: The WriteToken to check
  node:  The DAG node where the write operation occurs

Output: Proved | Disproved

Algorithm:
  # Fast path: reverse BFS
  unsafe = empty_set
  queue = brand_tree.consumers(token)

  while queue not empty:
    cur = queue.pop()
    unsafe.add(cur)

    for each pred in dag.predecessors(cur):
      # Structural cut: break edges are not traversed
      if pred is a break edge:
        continue

      # Backedge → check whether SMT fallback is needed
      if pred is a backedge:
        path_cond = path condition of the write node 'node'  # Judgment target is the write node's own condition
        loop_cond = loop condition
        # First check whether the structure can cut (corresponding break already cut the path → won't reach here)
        # Then check the path condition
        if path_cond is non-empty:
          result = smt_fallback(path_cond, loop_cond)   # ← slow path
          if result == Proved:
            continue                    # logical cut
        # No path condition or SMT cannot prove → traverse backedge
        # fall through

      if pred ∉ unsafe:
        queue.push(pred)

  # Judgment (write first, borrow later)
  # node < created_at(token) → token does not exist when write happens → Safe
  if node ∈ unsafe and created_at(token) ≤ node:
    return Disproved
  else:
    return Proved


smt_fallback(path_cond, loop_cond):
  # Called only on backedge + with path condition
  # Uses the RFC-027 proof pipeline, sharing the same SMT solver and budget
  return smt.prove(path_cond ⇒ !loop_cond)
  # Proved → logical cut
  # Disproved / Unproven → no cut, backedge traversed (conservative rejection)
  # SMT unavailable/timeout/not implemented = Disproved branch —
  # SMT only affects precision (whether legal programs pass), not soundness (everything that should be rejected still is);
  # conservativeness without SMT = any borrow + write inside a loop is rejected, on par with Rust NLL.
```

BrandNode adds a field:

```
BrandNode:
  ...
  created_at: NodeId         # Token creation node (left endpoint of the borrow interval)
```

O(N), where the number of SMT calls = number of backedges × fraction of backedges with path
conditions. In real code, SMT calls are extremely rare — triggered only in `while` loop bodies with
refined type variables in path conditions.

### Path Condition Collection

Provided by the existing RFC-027 §3.2-3.3 mechanisms:

- **if guard**: `if y > 0` → true branch pushes `y > 0`
- **match pattern**: `if let Some(v) = opt` → branch pushes `opt == Some(v)`
- **Assignment**: `i += 1`, compiler maintains variable value-range information
- **while cond**: inside the loop body, `cond == true` is pushed

Each DAG node carries a set of path conditions. When the reverse BFS hits a backedge, take the path
condition at the backedge start, and SMT decides whether it rules out the next loop entry.

Path condition propagation rules:

1. **Path condition attaches to the write node itself**: a write W inside a branch carries that
   branch's condition (`if i == n { W }` → path_cond(W) = `i == n`). When the reverse BFS traverses
   a backedge, SMT judges `path_cond(W) ⇒ !loop_cond` (any path reaching W must exit the loop →
   next-iteration consumer unreachable → cut), not the backedge node's path condition.
2. **Conservative clearing at join**: if/else join points do not carry branch-internal path
   conditions (after disjunction, the two branch conditions are usually undecidable, so they are
   cleared directly). Write operations after a join have empty path_cond → backedge traversed.
3. **Semantic path conditions**: path_cond is a ConstExpr (RFC-027 §3.2 semantics), not source text;
   smt_cut translates it into SMT constraints and solves.
4. **No path condition → backedge traversed directly** (unsafe), no SMT call.

### Interface with RFC-027

Borrow system predicates and user predicates share the same proof pipeline — the difference lies in
the **main proof strategy**:

| Query Type      | Proposition Source    | Main Strategy                           | Fallback                  |
| --------------- | --------------------- | --------------------------------------- | ------------------------- |
| Type equality   | Type checker          | Structural equivalence                  | —                         |
| User predicate  | Programmer annotation | SMT                                     | Programmer proof function |
| Borrow conflict | Compiler-generated    | **DAG structural analysis (fast path)** | SMT logical cut           |

The SMT solver's role in borrow checking: **not the main force, the safety net.** Called only when a
while backedge needs a logical cut. The vast majority of borrow checks complete on the fast path —
O(N) reverse BFS, zero SMT overhead.

### Relationship to Existing Code

| Existing Component             | Treatment                                                                   |
| ------------------------------ | --------------------------------------------------------------------------- |
| `BorrowChecker`                | Becomes `BorrowPredicateEmitter` — generates Hoare propositions for borrows |
| `MoveChecker`                  | Becomes `MovePredicateEmitter` — generates `¬moved(v)` propositions         |
| `DropChecker`                  | Same — generates Drop-related propositions                                  |
| `MutChecker`                   | Same — generates `is_mut(v)` propositions                                   |
| `ControlFlowAnalyzer`          | No longer needed — pipeline handles it uniformly                            |
| `liveness_analysis`            | Kept — Drop insertion still needs variable liveness                         |
| `ir_gen.rs` Release hardcoding | Removed — Release positions driven by DAG consumer analysis                 |

### NLL and Iteration Boundaries

Token liveness is an **interval** `[created_at, last_use]`, not a reverse-reachability set.
`created_at` = token creation node; `last_use` = the maximum consumer node from consumer analysis. A
write operation W conflicts with token T if and only if:
`conflicts(T, W) ∧ created_at(T) ≤ node(W) ∧ node(W)` can forward-reach `last_use(T)` (judged by
reverse BFS). The legal "write first, borrow later" order (§2.4: parameter tokens release when the
call ends) is excluded directly by `created_at(T) ≤ node(W)`, with no special rules needed. This
model makes the claim "the algorithm is not conservative" from §Tradeoffs, item 5 hold in all
orderings.

**Token death moment = last use point (NLL), not the lexical scope end.**

This is a natural corollary of consumer analysis: the position of the consumer defines the last use
of the token. `use(v)` is a consumer of `v` → `v` dies immediately after `use(v)`. No extra `{}` or
`drop()` is needed to end the token's life early.

**Loop iteration boundaries are the death line of token copies.** Three rules:

```
Rule 1: Variables declared inside a loop die automatically at the end of each iteration.
        for rebinds each iteration (guaranteed by language design); loop is the same.

Rule 2: The brand tree's ref_count at the loop head counts only copies created outside the loop.
        Copies produced by Dup inside the loop have ref_count zeroed at iteration boundaries.

Rule 3: When the reverse BFS traverses a backedge, it does not carry the current iteration's liveness information.
        It carries only the ref_count at the loop head (i.e., copies from outside the loop).
```

Example:

```yaoxiang
view = &data                          # At loop head: ref_count = 1, consumer = use(view)
loop {
    v2: &Point = view                 # Dup inside loop → ref_count = 2
    use(v2)                           # consumer: last use of v2 → v2 dies → ref_count = 1
    data.push(4)                      # ✅ safe! v2 is dead, only view remains (ref_count = 1, not a write conflict)
    # Iteration boundary: Rule 3 — v2 is not carried into the next iteration. At the start of the next iteration, v2 is freshly created by the new binding.
}
```

This design needs no extra "conservative loop liveness" rules. The reverse BFS starts from
consumers; a consumer inside the loop body → liveness is confined to the current iteration → the
backedge is not traversed. Fully consistent with the loop example in RFC-009a §Use-Case Analysis.

### `?` Error Propagation and Scope-Driven Release

`?` is an early return — besides the normal scope exit, there is an additional exit path. Tokens
must be released on this path; a wrong release order is UB.

**Release instructions are generated by scope analysis, not hardcoded after Call.**

The compiler maintains an exit-point list for each scope:

- `}` (normal scope end)
- `?` (error propagation, early return)
- Explicit `return`

At each exit point, Release instructions for all active tokens in the scope are inserted in reverse
declaration order (LIFO). The brand tree's parent-child relationships automatically handle cascading
release of derived tokens:

```yaoxiang
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)    # Returns child token &Float + parent token &Point
}

fn use_case(p: Point) -> Result<(), Error> = {
    (x_ref, p_ref) = p.get_x()?   # If ? propagates:
    # The brand tree knows x_ref is derived from p_ref (#42.field_x is a prefix of #42)
    # Release order: x_ref (child) → p_ref (parent) → LIFO is automatically satisfied
    p.modify()                     # WriteToken — all ReadTokens have been released
    Ok(())
}
```

Implementation location: kept in `ir_gen.rs`, changed to be scope-driven — no new compiler pass
introduced.

| Conflict judgment | O(1) | Per token request | | DAG consumer query | O(1) | Per token request | |
Reverse BFS (fast path) | O(N) | Per token request, N = nodes in the block | | SMT logical cut
(fallback) | ~1ms | **Extremely rare** — only while + path conditions |

> The complexities in the table above are **design estimates, not measured**; "~1ms" and "extremely
> rare" should be treated as order-of-magnitude expectations rather than measured values, to be
> calibrated against observability data after implementation lands.

**The trigger conditions for SMT fallback are extremely stringent**: it must simultaneously satisfy
(1) a while loop (2) with a write operation inside the loop body (3) where the write operation has a
path condition that can decide loop termination (4) where the compiler needs to rely on that
condition to cut the backedge. In real code, this is far less than 1% of cases. All other borrow
checks complete on the fast path.

Relationship to RFC-027 user predicates: user predicates use SMT as the main force, borrow system
predicates use structural analysis as the main force. They share the same SMT solver and budget cap
(RFC-027 §8), but borrow system predicates almost never consume SMT budget.

Linear code → no backedges → layer 1 O(N) is instant. Loop + path conditions → SMT call, linear
arithmetic at the millisecond level (RFC-027 budget 100ms). The result of one BFS can be cached and
reused for multiple queries on the same token.

### Error Message Design

**Core principle: error messages only contain symbols the user has written.**

Rust's borrow-related errors fall into two categories:

**Variable-level errors**: E0597 (doesn't live long enough), E0502 (mutable + immutable borrow
simultaneously), E0499 (multiple mutable borrows). Rust is already the gold standard — variable
name + line number, never shows `'a`. YaoXiang matches this precision. All the information is in the
brand tree: token creation point, consumer position, request point.

**Signature-level errors**: E0623 (lifetime mismatch), E0106 (missing lifetime specifier), E0477
(required lifetime not satisfied). These revolve around `'a`. YaoXiang **has no such errors** —
there is no `'a` in the signature. Not "can't report them"; it's that the user never wrote them, so
there's no need to report them.

Intra-function conflict example:

```
Error: `data` is frozen; mutable permission cannot be acquired
 --> src/main.yx:5:9
2 |     view = &data
  |            ----- `data` is frozen (read-only token created here)
4 |         use(view)
  |             ---- `view` is still in use here; freeze not lifted
5 |         data.push(4)
  |         ^^^^ mutable permission needed here
```

(Same precision as Rust E0499 — variable name + line number, no brand ID.)

Cross-function escape example:

```
Error: `num` (line 4) holds data sourced partly from `default_str` (line 3),
but `default_str` becomes invalid at line 6, and `num` is still in use at line 5.

Consider: moving `default_str`'s declaration up to the caller, or using `ref default_str` to share ownership.
```

(Same precision as Rust E0597. The brand summary knows `num` has two source paths — this exists in
the compiler, and the wording is usable.)

---

## Amendments to RFC-009 Body

RFC-009 §"Token Conflict Detection: Flow-Sensitive Liveness Analysis" has been updated:

1. Delete "things not needed: ... NLL" — not because the conclusion is wrong, but because the reason
   is wrong ("tokens are values, so linear tracking is enough")
2. The Layer 1 / Layer 2 transitional scheme is preserved; the complete scheme points to this RFC
3. Clarify: a brand ID (`#42`) is `'a` — exactly the same information, different encoding. Not a new
   analysis invented — lifetimes are lowered from the type layer to the proof layer

---

## Tradeoffs

### Advantages

1. **No lifetimes in type signatures**: `#42` is `'42` — the same information, encoded in the brand
   tree, not exposed in type signatures. This is unfalsifiable: count how many `'a` parameters a
   Rust generic with 3 reference parameters needs versus YaoXiang. The answer is 3 vs 0.

2. **Conceptual unification**: borrow checking and user predicates share the same proof pipeline —
   `{P} op {Q}`, the pipeline verifies P. Curry-Howard consistent.

3. **No new analysis framework**: no new analysis framework is introduced. Users don't perceive the
   existence of a "borrow checker" — just as they don't perceive the implementation details of a
   "type checker".

4. **Error messages contain only symbols the user wrote**: an entire dimension of error categories
   is gone (E0623, E0106, E0477 — all revolving around `'a`). Variable-level errors match Rust's
   precision.

5. **The algorithm is not conservative**: reverse BFS + break cuts + SMT logical cuts. No
   "conservative liveness inside loops". No "conservative branch merging".

### Disadvantages

1. **Not a new invention**: what a brand ID does is exactly what `'a` does — the constraint-solver
   complexity inside the compiler has not disappeared; it has merely been re-encoded from "variable
   name + constraint set" to "brand path + prefix matching". The difference for end users is only
   that `'a` is not written in the signature.

2. **Completely new implementation**: the brand tree exists only as a concept in the code; it must
   be implemented from scratch. `BorrowChecker`, `ControlFlowAnalyzer` are replaced.

3. **SMT dependency**: logical cuts depend on Z3 (already introduced by RFC-027, no new dependency).
   But borrow checking almost never triggers it — only called on while + path conditions.

4. **A few patterns need refactoring**: cross-branch borrows that the compiler's automatic proof
   cannot cover require the user to refactor code. This differs from Rust's `'a` fallback: Rust has
   `'a` as a tool (annotate and it passes); YaoXiang's fallback (proof functions) is not MVP.

---

## Alternatives

| Alternative                                   | Why not chosen                                                                                                                              |
| --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| Implement full Rust NLL                       | YaoXiang's design constraints (no shadowing, for rebinds) already eliminate the main sources of NLL complexity; no need for CFG fixed point |
| Keep the current approach (hardcoded Release) | Not enough — users must manage token scopes by hand                                                                                         |
| Only analyze in spawn blocks                  | Not enough — token usage outside spawn code is the majority                                                                                 |
| GC replaces borrow checking                   | Violates language design principles — YaoXiang has no GC                                                                                    |

---

## Implementation Phases

| Phase   | Content                                                           | Dependency                |
| ------- | ----------------------------------------------------------------- | ------------------------- |
| Phase 1 | Brand tree data structure implementation                          | —                         |
| Phase 2 | System predicate generators (Borrow/Move/Drop/Mut → propositions) | Phase 1                   |
| Phase 3 | Reverse BFS liveness analysis + pipeline integration (Layer 1)    | Phase 2                   |
| Phase 4 | Path condition collection + SMT logical cut (Layer 2)             | Phase 3 + RFC-027 Phase 2 |
| Phase 5 | Release instructions driven by DAG consumers                      | Phase 3                   |
| Phase 6 | Remove `ControlFlowAnalyzer`, refactor `BorrowChecker`            | Phase 4                   |

---

## Open Questions

- [x] **Brand tree's `ref_count` cross-iteration semantics during loop unrolling** — take the NLL
      route: a token dies after its last use. Copies bound inside the loop die at iteration
      boundaries; the reverse BFS does not carry liveness across iterations. See §NLL and Iteration
      Boundaries.
- [x] **Token release order on `?` error-propagation paths** — Release is driven by scope analysis
      (kept in `ir_gen.rs`). At each scope exit point (`}`, `?`, explicit `return`), active tokens
      are released in LIFO order. The brand tree's parent-child relationships automatically handle
      cascading release. See §`?` Error Propagation and Scope-Driven Release.
- [ ] Proof function syntax (far future, not MVP — does not block any phase)

---

## References

- [RFC-009: Ownership Model Design](009-ownership-model.md) — Parent RFC
- [RFC-027: Compile-Time Predicates and Unified Static Verification](027-compile-time-evaluation-types.md)
  — Proof pipeline
- [RFC-010: Unified Type Syntax](010-unified-type-syntax.md) — `{}` semantics
- [RFC-024: spawn-block-based Concurrency Model](024-concurrency-model.md) — spawn DAG

---

## Lifecycle and Disposition

| Status       | Location                    | Description                         |
| ------------ | --------------------------- | ----------------------------------- |
| **Accepted** | `docs/design/rfc/accepted/` | Becomes an official design document |
