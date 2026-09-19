---
title: 'RFC-009a: Token Lifetime Analysis—A Hoare Proof Pipeline'
status: 'Accepted'
author: 'Chenxu'
created: '2026-06-13'
updated: '2026-08-17'
group: 'rfc-009'

issue: '#129'

impl: 'partial'
---

# RFC-009a: Token Lifetime Analysis—A Hoare Proof Pipeline

> **Parent RFC**: [RFC-009: Ownership Model Design](../accepted/009-ownership-model.md)
>
> **Depends on**:
> [RFC-027: Compile-time Predicates and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md)
>
> **Prerequisite**: RFC-027 must be accepted. All mechanisms in this RFC (the proof pipeline, SMT
> fallback, path condition collection) depend on the RFC-027 implementation.
>
> **This RFC revises and replaces RFC-009 §"Token Conflict Detection: Flow-Sensitive Liveness
> Analysis" (lines 663-684).**

## Summary

RFC-009 line 684 claims token conflict detection "does not need...NLL". The conclusion is correct;
the argument is wrong.

It is not "because tokens are values, linear tracking is enough". It is because: **token liveness is
a Hoare logic proposition, not a special-purpose flow-sensitive analysis.**

`{all conflicting_tokens dead} op {WriteToken safely acquired}` — the same `{P} op {Q}` that shares
RFC-027's proof pipeline with type checking and predicate verification. No new analysis framework.
One pipeline, many propositions.

---

## Motivation

### The Confusion in RFC-009

RFC-009 conflates two problems:

1. **Linear tracking** (unusable after Move) — `{v not moved} use(v) {types match}`. The type
   checker already has this.
2. **Token lifetime interaction** (child token alive → parent token suspended → child token dead →
   parent token revived) — `{all conflicting_tokens dead} write(data) {safe}`. Requires **liveness
   analysis**, not linear tracking.

### The Actual State of the Code

| Component                                  | Status                                                                              |
| ------------------------------------------ | ----------------------------------------------------------------------------------- |
| `BorrowChecker`                            | Linearly scans IR, passively responding to explicit `Borrow`/`Release` instructions |
| `ControlFlowAnalyzer::analyze_instruction` | Empty implementation (`control_flow.rs:145-153`)                                    |
| `liveness_analysis`                        | Exists but only used for Drop insertion, not wired into token conflicts             |
| Release insertion                          | Hardcoded after Call instructions—pure lexical scope (`ir_gen.rs:2734-2736`)        |

**User-visible consequences**:

```yaoxiang
data = vec![1, 2, 3]
view = &data              # create ReadToken
x = view.total_count      # last use of view
data.push(4)              # ❌ Release(view) has not yet executed, ReadToken is "alive"
```

### Why a Rewrite Is Needed

The previous version (009a v1) used a "DAG replaces NLL" narrative, introducing unnecessary new
concepts (conservative branch rules, special-case loop handling). The core contradiction was not
articulated clearly: **borrow checking is not an independent system — it is one species of Hoare
proposition.**

---

## Core Design

### Everything Is Hoare

```
Type check:    { x: Int }        x + 1        { result: Int }
Borrow check:  { view is dead }  data.push(4) { WriteToken acquired }
Predicate:     { y > 0 }         divide(x, y) { result: Int }
Backedge cut:  { i == n }        next loop    { cond == false }
```

The same form `{P} op {Q}`. The compiler generates a precondition P for each operation and feeds it
into the proof pipeline for verification.

**Borrow checking and user predicates share the same pipeline.** The only difference is who
generates the proposition and what happens when it cannot be proved.

### Two Kinds of Predicates, One Pipeline

|                        | User predicate                   | System predicate (borrow)                     |
| ---------------------- | -------------------------------- | --------------------------------------------- |
| Proposition generation | Programmer (type annotation)     | Compiler (brand tree + ownership rules)       |
| Proof provider         | Compiler + programmer            | **Fully automatic by compiler**               |
| Cannot prove           | Write proof function or refactor | Refactor code (gate exists but rarely needed) |
| Visibility             | Visible in signature             | Implicit, does not pollute type signatures    |
| Learning cost          | Learn only if you want to use it | Zero                                          |

**Proofs of system predicates do not open proof functions to the programmer — the compiler handles
them fully automatically.** When it cannot prove, the user refactors the code.

**Three failure modes, one verification engine.** A type proposition cannot be proved → compile
error (cannot bypass). A borrow proposition cannot be proved → compile error, refactor code (cannot
bypass). A user predicate cannot be proved → compile error, may write proof function (can bypass).
The failure policies differ, but the verification engine is the same — an SMT solver + compiler
kernel inference rules. The only difference is "who is responsible for filling the gap when proof
fails" — the compiler refuses to write borrow proofs for the programmer (borrow propositions' proof
strategy is structural analysis + SMT, requiring no programmer intervention), but accepts
programmer-written user-predicate proof functions. This is not pipeline inconsistency — it is a
difference in the responsibility boundary between proposition categories.

This differs from Rust `'a`: `'a` is a required course, and proof functions are an elective — the
vast majority of users will never touch the door of the elective.

### Borrow Propositions: Compiler-Generated

The user writes `data.push(4)`. The compiler automatically generates the proposition:

```
WriteToken(data, node) acquirable
  = forall t in conflicting_tokens(data): t is dead at node
  = forall t in brand_tree.children(data): forward_reachable(node) ∩ consumers(t) == ∅
```

**Three rules, zero special cases:**

1. **Brand tree** (RFC-009 §2.7) answers "who conflicts with whom": prefix match, O(depth), depth ≤
   3
2. **Consumer list** (auto-collected during DAG construction) answers "who last consumed the token"
3. **Forward reachability** answers "can the consumer still be executed": structural cuts + logical
   cuts

### Forward Reachability: Walk Backwards from Consumers

For each consumer C of token T:

```
From C, do a reverse BFS over the DAG.
An edge is cut if:
  1. it is a break (structural cut)
  2. path condition ⇒ !loop_cond is proved true by SMT (logical cut, RFC-027 pipeline)

Propagate backwards along all uncut edges (including backedges; backedges propagate liveness to the previous iteration).
Mark all reachable nodes → unsafe.
```

Query: write operation at node W → W ∉ unsafe → safe.

**No need to invent "conservative branch rules". No need for "conservative loop liveness". One
reverse BFS + two cut rules.**

### Proof Strategy: Fast Path First, SMT as Fallback

```
Every write needing a token
  │
  ├→ Fast path: DAG structural analysis (covers 95%+ of cases)
  │     │
  │     ├→ Brand tree prefix match → find conflicting tokens (O(depth))
  │     ├→ Reverse BFS, break cuts backedges
  │     └→ No backedge to traverse → directly decide Proved / Disproved
  │
  └→ Slow path: SMT logical cut (only when fast path encounters a traversable backedge)
        │
        ├→ Backedge source has path condition → SMT check path_cond ⇒ !loop_cond
        │     ├→ Proved → logical cut → degrade back to fast path
        │     └→ Disproved / Unproven → traverse backedge → mark unsafe
        │
        └→ Backedge source has no path condition → traverse backedge directly
```

**Fast path covers**: linear code, if/else, loop + break, while without path conditions. **Slow path
covers**: while loop bodies where path conditions imply the loop will exit. **Does not cover**:
runtime conditions that cannot be statically proved → traverse backedge → unsafe → compile error
(user refactors).

SMT is not the main force — it is the safety net. Unlike RFC-027 user predicates where SMT is the
main force, borrow system predicates use structural analysis as the main force, with SMT only
filling corners that structural analysis cannot reach.

**SMT is a precision layer, not a soundness dependency.** Sound judgment of borrow system predicates
is fully borne by the fast path (interval + reverse BFS + break cuts); SMT logical cuts only
determine "whether a legal program with a loop boundary can be admitted". When SMT is unavailable /
times out / unimplemented (RFC-027 impl: in_progress), fallback = traverse backedge = conservative
rejection, what should be rejected is still always rejected. **Conservatism without SMT = borrow +
write inside a loop is always rejected, on par with Rust NLL** (production-grade Rust borrow
checking likewise has no SMT). SMT landing is pure precision gain, not blocking the delivery of the
sound main line.

---

## Use Case Analysis

### Linear Code

```yaoxiang
data = vec![1, 2, 3]        # node 1
view = &data                # node 2: consumes data, produces ReadToken(#1)
x = view.total_count        # node 3: consumes view (= last consumer of #1)
data.push(4)                # node 4: needs WriteToken(data)
```

Reverse BFS from `view.total_count` (node 3) → node 3 is #1's last consumer → node 4 > node 3 → node
4 not in unsafe → ✅

### if/else: No Special Rules

```yaoxiang
view = &data
if cond {
    use(view)               # then branch consumes view
} else {
    do_something_else()     # does not touch view
}
data.push(4)                # view's last consumer is inside if → no consumer after if → ✅
```

if/else is a composite node of the DAG. Internal consumption is attributed to this node. Branch
states are not merged. No conservative voting. **Whether there is a consumer later — integer
comparison.**

> **Clarification**: "branch states are not merged" refers only to **borrow liveness** (reverse BFS
> of brand consumers). **Move state** (variable ownership) is a separate analysis: per-CFG-node
> forward dataflow (NLL/Polonius style), with **conservative meet** at branch join (any branch Moved
> → join Moved), unreachable literal branches (`if false`) do not participate. The two are layered:
> borrow liveness looks at "are there subsequent consumers", move analysis looks at "has the
> variable possibly been transferred".

### if/else with Return Value Escape

```yaoxiang
view = &data
result = if cond {
    view                     # view escapes into result
} else {
    something_else
}
use(result)                  # indirectly consumes view
data.push(4)                 # view still has a consumer (use(result))
                             # → push is in unsafe → ❌ correct error
```

view escapes via return value → `use(result)` is view's consumer → walking backwards from `push`
reaches `use(result)` → unsafe.

### Loop: break Cuts the Backedge

```yaoxiang
view = &data
loop {
    use(view)                # consumer
    if is_last {
        data.push(4)         # write
        break                # ← structural cut
    }
}
```

Reverse BFS from `use(view)` → backedge → forward walk to `data.push(4)` → hit `break` → **cut** →
`data.push(4)` not in unsafe → ✅

Without break:

```yaoxiang
view = &data
loop {
    use(view)
    data.push(4)             # no break cut → backedge traversable → next iteration's use(view) reachable
                             # → push in unsafe → ❌ correct error
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

Note the judgment target is the **path condition of the write node itself** (`i == n` belongs to
`data.push(4)` inside the `if` branch), not the path condition of the backedge node.

---

## Essence: Brand ID Is `'a`

Don't say "we don't need `'a`". Say "`#42` _is_ `'42`".

| Rust                                    | YaoXiang                         | Equivalence                              |
| --------------------------------------- | -------------------------------- | ---------------------------------------- |
| `'a`                                    | `#42`                            | Compile-time lifetime identifier         |
| `'a: 'b` outlives constraint            | `#42` is prefix of `#42.field_x` | String prefix comparison = partial order |
| NLL liveness propagation (CFG fixpoint) | Reverse BFS (DAG)                | Both are reachability computations       |
| Polonius facts                          | SMT logical cut                  | Both are path condition reasoning        |
| Constraint system fixpoint solving      | Brand tree prefix match + BFS    | Different encoding, same problem         |

**We did not invent a new analysis. We merely moved `'a` from the type signature layer to the proof
layer.** What brand IDs do is exactly what `'a` does — mark borrow identity, track derivation, judge
conflict. There is only one difference: `'a` is in the user-written type signature; `#42` is inside
the compiler.

This is nothing to be ashamed of. Curry-Howard says types are propositions, programs are proofs.
`'a` is not part of the proposition — it is part of the proof strategy. Rust wrote the proof
strategy into the proposition signature. We put it back where it belongs.

### What Language Design Constraints Eliminate

| Source of complexity                       | Avoided? | Reason                                                                      |
| ------------------------------------------ | -------- | --------------------------------------------------------------------------- |
| Variable shadowing                         | ✅       | Language prohibits — one name always refers to the same thing               |
| Borrow across for iterations               | ✅       | Each iteration is a new binding — iterations are naturally isolated         |
| `'a` lifetime annotation                   | ✅       | Brand path = `#42.field_x`, compiler-inferred                               |
| Named lifetime + constraint propagation    | ✅       | Brand path prefix comparison replaces explicit constraint sets              |
| Borrow graph constraint solving (Polonius) | ✅       | Brand tree prefix match + DAG consumer query                                |
| Loop body borrow liveness propagation      | ❌       | Same as Rust, needs to be handled — via reverse BFS + logical cut           |
| Conditional branch conservatism            | ❌       | Same as Rust — SMT covers what is provable, rest is conservatively rejected |

### Why DAG Is Feasible

Three language design constraints of YaoXiang make DAG analysis feasible:

- **No variable shadowing** — one name always refers to the same thing, no need to track across
  rebindings
- **Each for iteration is a new binding** — iterations are naturally isolated, no cross-iteration
  borrow
- **Structured concurrency** — task boundaries are clear, no cross-task liveness propagation

These constraints eliminate the main sources of complexity of Rust's CFG fixpoint iteration. DAG is
not "more advanced" than CFG — it is a simpler language design that permits a simpler analysis.

---

## Detailed Design

### System Predicate List

The compiler automatically generates the following propositions and feeds them into the RFC-027
proof pipeline:

| System predicate  | Trigger              | Proposition form                              |
| ----------------- | -------------------- | --------------------------------------------- |
| `borrow_conflict` | Needs WriteToken(v)  | `forall t ∈ conflicting(v): dead_at(t, node)` |
| `use_after_move`  | Use variable v       | `¬moved(v)`                                   |
| `use_after_drop`  | Use variable v       | `¬dropped(v)`                                 |
| `double_drop`     | Drop(v)              | `¬dropped(v)`                                 |
| `mut_violation`   | Write to immutable v | `is_mut(v)`                                   |

The existing `BorrowChecker`, `MoveChecker`, `DropChecker`, `MutChecker` **become proposition
generators** — not deleted, repurposed. They generate propositions; the pipeline verifies them.

### Brand Tree

The brand mechanism of RFC-009 §2.7 is formalized as a brand tree.

**Token semantics — freeze first, not copy first**:

The essential difference between `&T` and `&mut T` is not "can it be copied" but "is simultaneous
write allowed":

```
ReadToken(T):  grants read-only permission, while freezing the source data T — any WriteToken(T) is
              unacquirable during this period. Freezing is ReadToken's primary semantics. Dup (copyable) is a corollary of freezing:
              since data is already frozen (no mutation possible), multiple read-only views are naturally safe.

WriteToken(T): grants exclusive read-write permission. Since write exists, no other token (read or write) can coexist.
              Dup is not implemented (linear type) is a corollary of exclusivity.
```

**Causal relation**:

```
ReadToken exists → source data frozen → multiple read-only safe → Dup
                      ↓
              WriteToken is rejected (enforced by borrow_conflict system predicate)
```

Not:

```
ReadToken has Dup → can have multiple → incidentally check conflict  ← reversed causation
```

```
BrandTree:
  nodes: Map<BrandId, BrandNode>

BrandNode:
  id: BrandId               # "#42", "#42.field_x"
  kind: ReadToken | WriteToken
  source_var: Operand
  parent: Option<BrandId>   # parent node of derivation relation
  children: Set<BrandId>    # derived child tokens
  consumers: Set<NodeId>    # DAG nodes that consume this token
  ref_count: usize          # number of safe copies during ReadToken freeze
```

**Conflict judgment** — execution mechanism guaranteed by freezing:

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

This algorithm introduces the "token creation time" dimension. Token liveness is an **interval**
`[created_at, last_use]`, not a reverse reachability set; a write operation only constitutes a
conflict within the token's liveness interval — this covers the legal order of "write first, borrow
after" (§2.4 semantics: parameter token released at call end), avoiding false positives.

```
Algorithm: check_borrow(token, node, dag, brand_tree)

Input:
  token: WriteToken to check
  node:  DAG node where the write occurs

Output: Proved | Disproved

Algorithm:
  # Fast path: reverse BFS
  unsafe = empty_set
  queue = brand_tree.consumers(token)

  while queue not empty:
    cur = queue.pop()
    unsafe.add(cur)

    for each pred in dag.predecessors(cur):
      # Structural cut: break does not traverse
      if pred is a break edge:
        continue

      # Backedge → check if SMT fallback is needed
      if pred is a backedge:
        path_cond = path condition of write node node   # judgment target is write node's own condition
        loop_cond = loop condition
        # First check whether it can be structurally cut (the corresponding break has cut the path → won't reach here)
        # Then check the path condition
        if path_cond not empty:
          result = smt_fallback(path_cond, loop_cond)   # ← slow path
          if result == Proved:
            continue                    # logical cut
        # No path condition or SMT cannot prove → traverse backedge
        # fall through

      if pred ∉ unsafe:
        queue.push(pred)

  # Judgment (write first, borrow after)
  # node < created_at(token) → token did not exist yet when write occurred → Safe
  if node ∈ unsafe and created_at(token) ≤ node:
    return Disproved
  else:
    return Proved


smt_fallback(path_cond, loop_cond):
  # Only called when there is a backedge + path condition
  # Uses RFC-027 proof pipeline, sharing the same SMT solver and the same budget
  return smt.prove(path_cond ⇒ !loop_cond)
  # Proved → logical cut
  # Disproved / Unproven → no cut, traverse backedge (conservative rejection)
  # SMT unavailable / times out / unimplemented = Disproved branch —
  # SMT only affects precision (whether legal programs pass), not soundness (what should be rejected is always rejected);
  # conservatism without SMT = borrow + write inside a loop is always rejected, on par with Rust NLL.
```

BrandNode adds the field:

```
BrandNode:
  ...
  created_at: NodeId         # token creation node (left endpoint of borrow interval)
```

O(N), where SMT call count = number of backedges × proportion of backedges with path conditions. In
real code, SMT calls are extremely rare — only triggered when a `while` loop body has path
conditions with refined type variables.

### Path Condition Collection

Provided by existing mechanisms in RFC-027 §3.2-3.3:

- **if guard**: `if y > 0` → true branch pushes `y > 0`
- **match pattern**: `if let Some(v) = opt` → branch pushes `opt == Some(v)`
- **Assignment**: `i += 1`, compiler maintains variable value range info
- **while cond**: loop body pushes `cond == true`

Each DAG node carries a path condition set. When reverse BFS hits a backedge, it takes the path
condition of the backedge's source and SMT judges whether the next loop entry condition is excluded.

Path condition propagation rules:

1. **Path condition attached to the write node itself**: a write W inside a branch carries its
   branch condition (`if i == n { W }` → path_cond(W) = `i == n`). When reverse BFS traverses a
   backedge, SMT judges `path_cond(W) ⇒ !loop_cond` (the path reaching W must exit the loop → next
   iteration's consumer unreachable → cut), not the path condition of the backedge node.
2. **Join conservatively clears**: if/else join points do not carry intra-branch path conditions
   (the disjunction of two branch conditions is usually undecidable, just clear it directly). Writes
   after the join have empty path_cond → backedge traverses.
3. **Path condition semantization**: path_cond is ConstExpr (RFC-027 §3.2 semantics), not source
   text; smt_cut translates it into SMT constraints before solving.
4. **No path condition → backedge traverses directly** (unsafe), SMT not called.

### Interface with RFC-027

Borrow system predicates and user predicates share the same proof pipeline — the difference is in
the **main proof strategy**:

| Query type      | Proposition source           | Main strategy                           | Fallback                    |
| --------------- | ---------------------------- | --------------------------------------- | --------------------------- |
| Type equality   | Type checker                 | Structural equivalence                  | —                           |
| User predicate  | Programmer's type annotation | SMT                                     | Programmer's proof function |
| Borrow conflict | Compiler-generated           | **DAG structural analysis (fast path)** | SMT logical cut             |

The SMT solver's role in borrow checking: **not the main force, but the safety net.** Called only
when a while backedge needs a logical cut. The vast majority of borrow checks complete on the fast
path — O(N) reverse BFS, zero SMT overhead.

### Relation to Existing Code

| Existing component             | Treatment                                                                   |
| ------------------------------ | --------------------------------------------------------------------------- |
| `BorrowChecker`                | Becomes `BorrowPredicateEmitter` — generates Hoare propositions for borrows |
| `MoveChecker`                  | Becomes `MovePredicateEmitter` — generates `¬moved(v)` propositions         |
| `DropChecker`                  | Same — generates Drop-related propositions                                  |
| `MutChecker`                   | Same — generates `is_mut(v)` propositions                                   |
| `ControlFlowAnalyzer`          | No longer needed — pipeline handles everything uniformly                    |
| `liveness_analysis`            | Kept — Drop insertion still needs variable liveness info                    |
| `ir_gen.rs` Release hardcoding | Removed — Release positions driven by DAG consumer analysis                 |

### NLL and Iteration Boundaries

Token liveness is an **interval** `[created_at, last_use]`, not a reverse reachability set.
`created_at` = token creation node; `last_use` = max consumer node from consumer analysis. The
necessary and sufficient condition for write W to conflict with token T:
`conflicts(T, W) ∧ created_at(T) ≤ node(W) ∧ node(W) can forward-reach last_use(T)` (judged by
reverse BFS). The legal order of "write first, borrow after" (§2.4: parameter token released at call
end) is directly excluded by `created_at(T) ≤ node(W)`, without any special rules. This model makes
the §Trade-offs benefit 5 "algorithm is not conservative" claim hold in all orderings.

**Token death time = last use point (NLL), not lexical scope end.**

This is a natural corollary of consumer analysis: the consumer's position defines the token's last
use. `use(v)` is a consumer of `v` → `v` dies immediately after `use(v)`. No extra `{}` or `drop()`
is needed to end the token's life early.

**Loop iteration boundary is the death line of token copies.** Three rules:

```
Rule 1: Variables declared inside a loop automatically die at the end of each iteration.
        Each for iteration is a new binding (guaranteed by language design), same for loop.

Rule 2: The brand tree ref_count at the loop header only counts copies created outside the loop.
        New copies produced by Dup inside the loop have ref_count cleared at iteration boundary.

Rule 3: When reverse BFS traverses a backedge, it does not carry the liveness info of the current iteration.
        Only carries ref_count at the loop header (i.e., copies outside the loop).
```

Example:

```yaoxiang
view = &data                          # loop header: ref_count = 1, consumer = use(view)
loop {
    v2: &Point = view                 # Dup inside loop → ref_count = 2
    use(v2)                           # consumer: v2's last use → v2 dies → ref_count = 1
    data.push(4)                      # ✅ safe! v2 is dead, only view remains (ref_count = 1, no write conflict)
    # Iteration boundary: Rule 3 — does not carry v2 into next round. At next iteration's start v2 is recreated by new binding.
}
```

This design needs no additional "conservative loop liveness" rule. Reverse BFS starts from
consumers, consumers are inside the loop body → liveness is confined to the current iteration →
backedge does not traverse. This is fully consistent with the loop example in RFC-009a §Use Case
Analysis.

### `?` Error Propagation and Scope-Driven Release

`?` is early return — beyond the normal scope exit, there is one more exit path. Tokens must be
released along this path; incorrect release order is UB.

**Release instructions are generated by scope analysis, not hardcoded after Call.**

The compiler maintains an exit point list for each scope:

- `}` (normal end of scope)
- `?` (error propagation, early return)
- explicit `return`

At each exit point, Release instructions for all live tokens in the scope are inserted in
declaration reverse order (LIFO). Parent-child relations in the brand tree automatically handle
cascading release of derived tokens:

```yaoxiang
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)    # returns child token &Float + parent token &Point
}

fn use_case(p: Point) -> Result<(), Error> = {
    (x_ref, p_ref) = p.get_x()?   # if ? propagates:
    # Brand tree knows x_ref is derived from p_ref (#42.field_x is prefix of #42)
    # Release order: x_ref (child) → p_ref (parent) → LIFO automatically satisfied
    p.modify()                     # WriteToken — all ReadTokens released
    Ok(())
}
```

Implementation location: kept in `ir_gen.rs`, changed to scope-driven — no new compiler pass
introduced.

| Conflict judgment | O(1) | Per token needed | | DAG consumer query | O(1) | Per token needed | |
Reverse BFS (fast path) | O(N) | Per token needed, N = number of nodes in block | | SMT logical cut
(fallback) | ~1ms | **Extremely rare** — only while + path condition |

> The complexity figures in the table above are **design estimates, not measured**; "~1ms" and
> "extremely rare" should be read as order-of-magnitude expectations rather than measurements, to be
> calibrated against observability data after implementation lands.

**The trigger conditions for SMT fallback are extremely strict**: simultaneously satisfying (1)
while loop (2) write operation inside loop body (3) write followed by a path condition that decides
loop termination (4) compiler needs to depend on that condition to cut the backedge. In real code,
the proportion is far less than 1%. The rest of borrow checking all completes on the fast path.

Relation to RFC-027 user predicates: user predicates have SMT as the main force, borrow system
predicates have structural analysis as the main force. They share the same SMT solver and budget cap
(RFC-027 §8), but borrow system predicates barely consume SMT budget.

Linear code → no backedge → tier 1 O(N) finishes in an instant. Loop + path condition → SMT call,
linear arithmetic at millisecond level (RFC-027 budget 100ms). One BFS result can be cached and
reused for multiple queries of the same token.

### Error Message Design

**Core principle: error messages only show symbols the user has written.**

Rust's borrow-related errors fall into two categories:

**Variable-level errors**: E0597 (does not live long enough), E0502 (mutable + immutable
simultaneous borrow), E0499 (multiple mutable borrows). Rust is already the benchmark — variable
name + line number, no `'a` appears. YaoXiang matches the precision. All info is in the brand tree:
token creation point, consumer location, request point.

**Signature-level errors**: E0623 (lifetime mismatch), E0106 (missing lifetime specifier), E0477
(required lifetime not satisfied). All revolve around `'a`. YaoXiang **does not have this category
of error** — there is no `'a` in signatures. Not "cannot be reported", but the user never wrote it
so it does not need to be reported.

In-function conflict example:

```
Error: `data` is frozen, mutable permission cannot be acquired
 --> src/main.yx:5:9
2 |     view = &data
  |            ----- `data` is frozen (read-only token created here)
4 |         use(view)
  |             ---- `view` is still in use here, freeze not lifted
5 |         data.push(4)
  |         ^^^^ mutable permission needed here
```

(On par with Rust E0499 precision — variable name + line number, no brand ID appears.)

Cross-function escape example:

```
Error: one of the data sources held by `num` (line 4) is `default_str` (line 3),
but `default_str` becomes invalid at line 6, and `num` is still in use at line 5.

Consider: moving the declaration of `default_str` up to the caller, or using `ref default_str` to share holding.
```

(On par with Rust E0597 precision. The brand summary knows `num` has two source paths — already in
the compiler, wording is available.)

---

## Corrections to RFC-009 Body

RFC-009 §"Token Conflict Detection: Flow-Sensitive Liveness Analysis" has been updated:

1. Remove "things not needed: ...NLL" — not because the conclusion is wrong, but because the reason
   is wrong ("tokens are values, linear tracking is enough")
2. The tier 1 / tier 2 transition plan is kept; the full plan points to this RFC
3. Clarify: brand ID (`#42`) is `'a` — exactly the same information, different encoding. Not a new
   analysis — moving lifetime from the type layer to the proof layer

---

## Trade-offs

### Benefits

1. **Type signatures contain no lifetime**: `#42` is `'42` — same information, encoded in the brand
   tree, not exposed in type signatures. This is unfalsifiable: count how many `'a` parameters a
   Rust generic with 3 reference parameters needs vs YaoXiang. The answer is 3 vs 0.

2. **Conceptual unification**: borrow checking and user predicates share the same proof pipeline —
   `{P} op {Q}`, pipeline verifies P. Curry-Howard consistent.

3. **Zero new analysis framework**: no new analysis framework introduced. The user does not perceive
   the existence of a "borrow checker" — just as the user does not perceive the implementation
   details of a "type checker".

4. **Error messages only contain user-written symbols**: one whole dimension of error categories is
   gone (E0623, E0106, E0477 — all around `'a`). Variable-level errors are on par with Rust
   precision.

5. **Algorithm is not conservative**: reverse BFS + break cuts + SMT logical cuts. No "conservative
   survival inside loops". No "conservative branch merging".

### Drawbacks

1. **Not a new invention**: what brand IDs do is exactly the same as `'a` — the constraint-solver
   complexity inside the compiler has not disappeared, only the encoding has changed from "variable
   name + constraint set" to "brand path + prefix match". The difference for end users is only that
   `'a` is not written in signatures.

2. **Brand new implementation**: brand tree only exists as a concept in code, needs to be built from
   scratch. BorrowChecker, ControlFlowAnalyzer are replaced.

3. **SMT dependency**: logical cuts depend on Z3 (already introduced by RFC-027, no new dependency).
   But borrow checking barely triggers it — only called for while + path conditions.

4. **A very few patterns require refactoring**: borrow conflicts across branches that the compiler's
   automatic proof cannot cover, users must refactor code. Different from Rust's `'a` fallback: Rust
   has `'a` as a tool (annotation passes); YaoXiang's fallback (proof functions) is not MVP.

---

## Alternatives

| Alternative                      | Why not                                                                                                                                       |
| -------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| Implement full Rust NLL          | YaoXiang's design constraints (no shadowing, for rebinding) have already eliminated NLL's main complexity sources; CFG fixpoint is not needed |
| Keep current (hardcoded Release) | Not enough — users must manually manage token scopes                                                                                          |
| Only do analysis in spawn blocks | Not enough — token use outside spawn code is the majority                                                                                     |
| GC instead of borrow checking    | Violates language design principles — YaoXiang has no GC                                                                                      |

---

## Implementation Phases

| Phase   | Content                                                           | Depends on                |
| ------- | ----------------------------------------------------------------- | ------------------------- |
| Phase 1 | Brand tree data structure implementation                          | —                         |
| Phase 2 | System predicate generators (Borrow/Move/Drop/Mut → propositions) | Phase 1                   |
| Phase 3 | Reverse BFS liveness analysis + pipeline integration (tier 1)     | Phase 2                   |
| Phase 4 | Path condition collection + SMT logical cut (tier 2)              | Phase 3 + RFC-027 Phase 2 |
| Phase 5 | Release instructions changed to be driven by DAG consumers        | Phase 3                   |
| Phase 6 | Delete ControlFlowAnalyzer, refactor BorrowChecker                | Phase 4                   |

---

## Open Questions

- [x] **ref_count** cross-iteration semantics during loop unrolling — NLL path: token dies after
      last use. Copies bound inside the loop die at iteration boundary; reverse BFS does not carry
      liveness across iterations. See §NLL and Iteration Boundaries.
- [x] **Token release order on `?`** error propagation paths — Release driven by scope analysis
      (kept in ir_gen.rs). Each scope exit point (`}`, `?`, explicit return) releases live tokens in
      LIFO order. Brand tree parent-child relations automatically handle cascading release. See §`?`
      Error Propagation and Scope-Driven Release.
- [ ] Proof function syntax (far future, not MVP — does not block any Phase)

---

## References

- [RFC-009: Ownership Model Design](../accepted/009-ownership-model.md) — Parent RFC
- [RFC-027: Compile-time Predicates and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md)
  — Proof pipeline
- [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md) — `{}` semantics
- [RFC-024: spawn-block-based Concurrency Model](../accepted/024-concurrency-model.md) — spawn DAG

---

## Lifecycle and Destination

| Status       | Location                    | Description                      |
| ------------ | --------------------------- | -------------------------------- |
| **Accepted** | `docs/design/rfc/accepted/` | Becomes official design document |
