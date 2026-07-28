---
title: 'RFC-009a: Token Lifetime Analysis — Based on Hoare Proof Pipeline'
status: 'Accepted'
author: 'Chen Xu'
created: '2026-06-13'
updated: '2026-06-13'
group: 'rfc-009'

issue: '#129'

impl: 'partial'
---

# RFC-009a: Token Lifetime Analysis — Based on Hoare Proof Pipeline

> **Parent RFC**: [RFC-009: Ownership Model Design](../accepted/009-ownership-model.md)
>
> **Depends on**: [RFC-027: Compile-Time Predicates and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md)
>
> **Prerequisite**: RFC-027 has been accepted. All mechanisms in this RFC (proof pipeline, SMT fallback, path condition collection) depend on RFC-027's implementation.
>
> **This RFC amends and supersedes RFC-009 §"Token Conflict Detection: Flow-Sensitive Liveness Analysis" (lines 663-684).**

## Summary

RFC-009 line 684 claims that token conflict detection "doesn't need... NLL." The conclusion is correct; the reasoning is wrong.

It's not "because tokens are values, linear tracking is sufficient." It's because: **token liveness is a Hoare logic proposition, not a specialized flow-sensitive analysis.**

`{all conflicting_tokens dead} op {WriteToken safely acquired}` — the same `{P} op {Q}`, shared with type checking and predicate verification via the RFC-027 proof pipeline. No new analysis framework. One pipeline, multiple propositions.

---

## Motivation

### The Confusion in RFC-009

RFC-009 conflates two problems:

1. **Linear tracking** (unusable after Move) — `{v not moved} use(v) {type matches}`. Already in the type checker.
2. **Token lifetime interactions** (sub-token alive → parent token paused → sub-token dead → parent token revived) — `{all conflicting_tokens dead} write(data) {safe}`. Requires **liveness analysis**, not linear tracking.

### Current Code Reality

| Component                                 | Status                                                         |
| ----------------------------------------- | -------------------------------------------------------------- |
| `BorrowChecker`                           | Linear IR scan, passively responds to explicit `Borrow`/`Release` instructions |
| `ControlFlowAnalyzer::analyze_instruction`| Empty implementation (`control_flow.rs:145-153`)             |
| `liveness_analysis`                       | Exists but only used for Drop insertion, not connected to token conflicts |
| Release insertion                        | Hardcoded after Call instructions — pure lexical scoping (`ir_gen.rs:2734-2736`) |

**User-visible consequences**:

```yaoxiang
data = vec![1, 2, 3]
view = &data              # Create ReadToken
x = view.total_count      # Last use of view
data.push(4)              # ❌ Release(view) hasn't executed yet, ReadToken is "alive"
```

### Why Rewriting is Needed

The previous version (009a v1) used the "DAG instead of NLL" narrative, introducing unnecessary new concepts (conservative branching rules, special loop handling). The core contradiction wasn't clearly stated: **borrow checking is not an independent system — it's one kind of Hoare proposition.**

---

## Core Design

### Everything is Hoare

```
Type checking:   { x: Int }        x + 1        { result: Int }
Borrow checking: { view is dead }  data.push(4) { WriteToken acquired successfully }
Predicate verify:{ y > 0 }         divide(x, y) { result: Int }
Back-edge cut:   { i == n }        next loop     { cond == false }
```

The same form `{P} op {Q}`. The compiler generates precondition P for each operation, feeds it to the proof pipeline for verification.

**Borrow checking and user predicates share the same pipeline.** The only difference is who generates the proposition and what happens when it can't be proven.

### Two Types of Predicates, One Pipeline

|          | User Predicates         | System Predicates (Borrowing)             |
| -------- | ----------------------- | ------------------------------------------ |
| Proposition generation | Programmer (type annotations) | Compiler (brand tree + ownership rules) |
| Proof supply     | Compiler + programmer   | **Compiler fully automatic**              |
| Can't prove      | Write proof function or refactor | Refactor code (door stays but rarely needed) |
| Visibility       | Visible in signatures    | Implicit, doesn't pollute type signatures |
| Learning curve   | Learn when you want      | Zero                                       |

**System predicate proof doesn't open proof functions to programmers — fully compiler-automated.** When proof fails, users refactor code.

**Three failure modes, one verification engine.**
Type proposition can't be proven → compile error (cannot bypass). Borrow proposition can't be proven → compile error, refactor code (cannot bypass). User predicate can't be proven → compile error, can write proof function (can bypass). Failure strategies differ, but the verification engine is the same — SMT solver + compiler internal inference rules. The only difference is "who is responsible for补证明 when it can't be proven" — the compiler refuses to write borrow proofs for programmers (borrow propositions use structural analysis + SMT as proof strategy, no programmer involvement needed), but accepts programmer-written user predicate proof functions. This isn't pipeline inconsistency — it's different responsibility boundaries for different proposition categories.

This differs from Rust `'a`: `'a` is required, proof functions are elective — the vast majority of users will never touch the elective course.

### Borrow Propositions: Compiler-Generated Automatically

User writes `data.push(4)`. Compiler automatically generates proposition:

```
WriteToken(data, node) acquirable
  = forall t in conflicting_tokens(data): t is dead at node
  = forall t in brand_tree.children(data): forward_reachable(node) ∩ consumers(t) == ∅
```

**Three rules, zero special cases:**

1. **Brand tree** (RFC-009 §2.7) answers "who conflicts with whom": prefix matching, O(depth), depth ≤ 3
2. **Consumer list** (automatically collected during DAG construction) answers "who last consumed the token"
3. **Forward reachability** answers "can consumers still be reached": structural cut + logical cut

### Forward Reachability: Walking Backward from Consumers

For each consumer C of token T:

```
From C, reverse BFS DAG.
Edge is cut if:
  1. It's a break (structural cut)
  2. Path condition ⇒ !loop_cond is proven by SMT (logical cut, RFC-027 pipeline)

Reverse-propagate liveness along all uncut edges (including back-edges; back-edges propagate liveness to previous iteration).
Mark all reachable nodes → unsafe.
```

Query: write operation at node W → W ∉ unsafe → safe.

**No need to invent "conservative branching rules." No need for "loop conservative liveness." One reverse BFS + two cut rules.**

### Proof Strategy: Fast Path First, SMT Fallback

```
Each write operation needing a token
  │
  ├→ Fast path: DAG structural analysis (covers 95%+ of cases)
  │     │
  │     ├→ Brand tree prefix matching → find conflicting tokens (O(depth))
  │     ├→ Reverse BFS, break cuts back-edges
  │     └→ No back-edges traversable → directly determine Proved / Disproved
  │
  └→ Slow path: SMT logical cut (only when fast path encounters traversable back-edges)
        │
        ├→ Back-edge start has path condition → SMT check path_cond ⇒ !loop_cond
        │     ├→ Proved → logical cut → demote to fast path and continue
        │     └→ Disproved / Unproven → back-edge traversable → mark unsafe
        │
        └→ Back-edge start has no path condition → back-edge directly traversable
```

**Fast path covers**: linear code, if/else, loop + break, while without path conditions.
**Slow path covers**: while loop bodies, when path conditions imply loop will exit.
**Not covered**: runtime conditions that cannot be statically proven → back-edge traversable → unsafe → compile error (user refactors).

SMT is not the main force — it's the safety net. Unlike RFC-027 user predicates: user predicates use SMT as main force; borrow system predicates use structural analysis as main force, SMT only fills in corners structural analysis can't reach.

---

## Use Case Analysis

### Linear Code

```yaoxiang
data = vec![1, 2, 3]        # Node 1
view = &data                # Node 2: consumes data, produces ReadToken(#1)
x = view.total_count        # Node 3: consumes view (= last consumer of #1)
data.push(4)                # Node 4: needs WriteToken(data)
```

Reverse BFS from `view.total_count` (node 3) → node 3 is #1's last consumer → node 4 > node 3 → node 4 not in unsafe → ✅

### if/else: No Special Rules

```yaoxiang
view = &data
if cond {
    use(view)               # then branch consumes view
} else {
    do_something_else()     # doesn't touch view
}
data.push(4)                # view's last consumer is in if → no consumers after if → ✅
```

if/else is a composite node in the DAG. Internal consumption attributes to this node. No branch state merging. No conservative voting. **Whether there are consumers afterward, integer comparison.**

### if/else with Return Value Escape

```yaoxiang
view = &data
result = if cond {
    view                     # view escapes to result
} else {
    something_else
}
use(result)                  # indirectly consumes view
data.push(4)                 # view still has consumers (use(result))
                             # → push in unsafe → ❌ Correct error
```

view escapes via return value → `use(result)` is view's consumer → reverse walking from `push` can reach `use(result)` → unsafe.

### Loop: Break Cuts Back-Edge

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

Reverse BFS from `use(view)` → back-edge → forward to `data.push(4)` → encounter `break` → **cut** → `data.push(4)` not in unsafe → ✅

Without break:

```yaoxiang
view = &data
loop {
    use(view)
    data.push(4)             # no break cut → back-edge traversable → next iteration's use(view) reachable
                             # → push in unsafe → ❌ Correct error
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

Reverse BFS from `use(view)` → back-edge → to `data.push(4)` → check path condition `i == n` → SMT query: `i == n ⇒ !(i < n)`? → Proved → **logical cut** → `data.push(4)` not in unsafe → ✅

---

## Essence: Brand ID is Just `'a`

Not saying "we don't need `'a`." Saying "`#42` is just `'42`."

| Rust                       | YaoXiang                      | Equivalence                      |
| -------------------------- | ----------------------------- | -------------------------------- |
| `'a`                       | `#42`                         | Compile-time lifetime identifier |
| `'a: 'b` outlives constraint | `#42` is prefix of `#42.field_x` | String prefix = partial order |
| NLL liveness propagation (CFG fixpoint) | Reverse BFS (DAG)        | Both are reachability computation |
| Polonius facts             | SMT logical cut               | Both are path condition reasoning |
| Constraint system fixpoint | Brand tree prefix matching + BFS | Different encodings, same problem |

**We haven't invented new analysis. We've just moved `'a` from the type signature layer to the proof layer.** Brand ID does exactly what `'a` does — marks borrow identity, tracks derivation relationships, determines conflicts. One difference: `'a` is in user-written type signatures; `#42` is inside the compiler.

This isn't shameful. Curry-Howard says types are propositions, programs are proofs. `'a` isn't part of the proposition — it's part of the proof strategy. Rust wrote the proof strategy into the proposition signature. We've put it back where it belongs.

### What Language Design Constraints Eliminate

| Source of Complexity             | Avoided? | Reason                                        |
| -------------------------------- | -------- | --------------------------------------------- |
| Variable shadowing               | ✅       | Language forbids it — a name always refers to the same thing |
| for cross-iteration borrowing    | ✅       | Each iteration new binding — iterations naturally isolated |
| `'a` lifetime annotations        | ✅       | Brand path = `#42.field_x`, compiler-derived |
| Named lifetimes + constraint propagation | ✅ | Brand path prefix comparison replaces explicit constraint sets |
| Borrow graph constraint solving (Polonius) | ✅ | Brand tree prefix matching + DAG consumer query |
| Loop body borrow liveness propagation | ❌   | Same as Rust needs to handle — using reverse BFS + logical cut |
| Conditional branch conservatism  | ❌       | Same as Rust — SMT covers provable cases, rest conservatively rejected |

### Why DAG Works

YaoXiang's three language design constraints make DAG analysis feasible:

- **No variable shadowing** — a name always refers to the same thing, no need to track across rebindings
- **for creates new binding each iteration** — iterations naturally isolated, no cross-iteration borrowing
- **Structured concurrency** — task boundaries clear, no cross-task liveness propagation

These constraints eliminate the main source of CFG fixpoint iteration complexity in Rust. It's not that DAG is "more advanced" than CFG — simpler language design allows simpler analysis.

---

## Detailed Design

### System Predicate Checklist

Compiler automatically generates the following propositions and feeds them to RFC-027 proof pipeline:

| System Predicate      | Trigger时机           | Proposition Form                            |
| --------------------- | --------------------- | ------------------------------------------- |
| `borrow_conflict`     | Need WriteToken(v)    | `forall t ∈ conflicting(v): dead_at(t, node)` |
| `use_after_move`      | Use variable v        | `¬moved(v)`                                 |
| `use_after_drop`      | Use variable v        | `¬dropped(v)`                               |
| `double_drop`         | Drop(v)               | `¬dropped(v)`                               |
| `mut_violation`       | Write to immutable v  | `is_mut(v)`                                 |

Existing `BorrowChecker`, `MoveChecker`, `DropChecker`, `MutChecker` **become proposition generators** — not gone, changed roles. They generate propositions, pipeline verifies propositions.

### Brand Tree

The brand mechanism from RFC-009 §2.7 is formalized as a brand tree.

**Token semantics — freeze first, not copy first**:

The essential difference between `&T` and `&mut T` isn't "can it be copied", it's "does it allow concurrent writes":

```
ReadToken(T): Grants read-only permission, simultaneously freezes source data T — any WriteToken(T) is
              not acquirable during this period. Freeze is the primary semantics of ReadToken. Dup
              (copyable) is a corollary of freeze: because data is already frozen (no mutation possible),
              multiple read-only views are naturally safe.

WriteToken(T): Grants exclusive read-write permission. Because writes exist, no other tokens (read or
               write) can coexist. Not implementing Dup (linear type) is a corollary of exclusivity.
```

**Causal relationship**:

```
ReadToken exists → source data frozen → multiple read-only safe → Dup
                      ↓
              WriteToken denied (borrow_conflict system predicate enforced)
```

Not:

```
ReadToken has Dup → can have multiple → check conflicts incidentally  ← causal reversal
```

```
BrandTree:
  nodes: Map<BrandId, BrandNode>

BrandNode:
  id: BrandId               # "#42", "#42.field_x"
  kind: ReadToken | WriteToken
  source_var: Operand
  parent: Option<BrandId>   # parent in derivation relationship
  children: Set<BrandId>    # derived sub-tokens
  consumers: Set<NodeId>     # DAG nodes consuming this token
  ref_count: usize           # safe copy count during ReadToken freeze
```

**Conflict determination — enforcement mechanism for freeze**:

```rust
fn conflicts(a: &BrandId, b: &BrandId) -> bool {
    // Conflict condition: same source + at least one is write + brand path overlaps
    // This means:
    //   1. ReadToken vs ReadToken → no conflict (both read-only, no mutation)
    //   2. WriteToken vs ReadToken → conflict (write breaks read's freeze guarantee)
    //   3. WriteToken vs WriteToken → conflict (two writes cannot coexist)
    a.source() == b.source()
        && (a.is_write() || b.is_write())
        && (a.is_prefix_of(b) || b.is_prefix_of(a))
}
```

O(depth) string prefix comparison, depth ≤ 3. Constant time.

### Reverse BFS Liveness Analysis

```
Algorithm: check_borrow(token, node, dag, brand_tree)

Input:
  token: WriteToken to check
  node:  DAG node where write operation occurs

Output: Proved | Disproved

Algorithm:
  # Fast path: reverse BFS
  unsafe = empty_set
  queue = brand_tree.consumers(token)

  while queue not empty:
    cur = queue.pop()
    unsafe.add(cur)

    for each pred in dag.predecessors(cur):
      # Structural cut: break doesn't traverse
      if pred is break edge:
        continue

      # Back-edge → check if SMT fallback needed
      if pred is back-edge:
        path_cond = path condition at pred
        loop_cond = loop condition
        # First check if structurally cuttable (corresponding break already cut path → won't reach here)
        # Then check path condition
        if path_cond not empty:
          result = smt_fallback(path_cond, loop_cond)   # ← slow path
          if result == Proved:
            continue                    # logical cut
        # No path condition or SMT can't prove → traverse back-edge
        # fall through

      if pred ∉ unsafe:
        queue.push(pred)

  # Determination
  if node ∈ unsafe:
    return Disproved
  else:
    return Proved


smt_fallback(path_cond, loop_cond):
  # Called only when back-edge + path condition exists
  # Uses RFC-027 proof pipeline, shares same SMT solver and budget
  return smt.prove(path_cond ⇒ !loop_cond)
  # Proved → logical cut
  # Disproved / Unproven → don't cut, back-edge traversable
```

O(N), where SMT call count = back-edge count × proportion of back-edges with path conditions. In actual code, SMT calls are extremely rare — only triggered in while loop bodies with refined type variable path conditions.

### Path Condition Collection

Provided by existing RFC-027 §3.2-3.3 mechanisms:

- **if guard**: `if y > 0` → push `y > 0` to true branch
- **match pattern**: `if let Some(v) = opt` → push `opt == Some(v)` in branch
- **Assignment**: `i += 1`, compiler maintains variable value domain info
- **while cond**: push `cond == true` in loop body

Each DAG node carries a path condition set. When reverse BFS encounters a back-edge, it takes the path condition at the back-edge start, and SMT determines whether it excludes the next iteration's entry condition.

### Interface with RFC-027

Borrow system predicates and user predicates share the same proof pipeline — difference lies in **primary proof strategy**:

| Query Type      | Proposition Source   | Primary Strategy                     | Fallback       |
| --------------- | -------------------- | ------------------------------------ | -------------- |
| Type equality   | Type checker         | Structural equivalence               | —              |
| User predicate  | Programmer annotation| SMT                                  | Programmer proof function |
| Borrow conflict | Compiler auto-generated | **DAG structural analysis (fast path)** | SMT logical cut   |

Role of SMT solver in borrow checking: **not primary, safety net.**
Only called when while back-edge needs logical cut. The vast majority of borrow checking completes in fast path — O(N) reverse BFS, zero SMT cost.

### Relationship with Existing Code

| Existing Component             | Treatment                                              |
| ------------------------------ | ------------------------------------------------------ |
| `BorrowChecker`                | Becomes `BorrowPredicateEmitter` — generates Hoare propositions for borrowing |
| `MoveChecker`                  | Becomes `MovePredicateEmitter` — generates `¬moved(v)` propositions |
| `DropChecker`                  | Same — generates Drop-related propositions             |
| `MutChecker`                   | Same — generates `is_mut(v)` propositions             |
| `ControlFlowAnalyzer`          | No longer needed — pipeline handles uniformly          |
| `liveness_analysis`            | Kept — Drop insertion still needs variable liveness info|
| `ir_gen.rs` Release hardcode   | Deleted — Release positions driven by DAG consumer analysis |

### NLL and Iteration Boundaries

**Token death time = last use point (NLL), not end of lexical scope.**

This is the natural corollary of consumer analysis: the consumer's position defines the token's last use. `use(v)` is `v`'s consumer → `v` dies immediately after `use(v)`. No additional `{}` or `drop()` needed to end token lifetime early.

**Loop iteration boundary is the death line for token copies.** Three rules:

```
Rule 1: Variables declared in loops auto-die at end of each iteration.
        Each iteration of for is a new binding (language guarantee), same for loop.

Rule 2: Brand tree ref_count at loop head only counts copies created outside loop.
        New copies produced by Dup inside loop, ref_count resets at iteration boundary.

Rule 3: When reverse BFS traverses back-edge, it doesn't carry current iteration's liveness info.
        Only carries ref_count at loop head (i.e., copies from outside loop).
```

Example:

```yaoxiang
view = &data                          # Loop head: ref_count = 1, consumer = use(view)
loop {
    v2: &Point = view                 # Dup inside loop → ref_count = 2
    use(v2)                           # consumer: last use of v2 → v2 dies → ref_count = 1
    data.push(4)                      # ✅ Safe! v2 is dead, only view remains (ref_count = 1, not write conflict)
    # Iteration boundary: Rule 3 — don't carry v2 to next round. v2 recreated by new binding at next iteration start.
}
```

This design doesn't need additional "loop conservative liveness" rules. Reverse BFS starts from consumers, consumers are in loop body → liveness restricted to current iteration → back-edge doesn't traverse. Consistent with RFC-009a §Use Case Analysis loop examples.

### `?` Error Propagation and Scope-Driven Release

`?` is early return — one more exit path beyond the scope's normal exit. Tokens must be released on this path, wrong release order is UB.

**Release instructions are generated by scope analysis, not hardcoded after Call.**

Compiler maintains exit point list for each scope:

- `}` (scope ends normally)
- `?` (error propagation, early return)
- Explicit `return`

At each exit point, insert Release instructions for all active tokens in that scope, in reverse declaration order (LIFO). Brand tree parent-child relationships automatically handle cascading release of derived tokens:

```yaoxiang
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)    # Returns sub-token &Float + parent token &Point
}

fn use_case(p: Point) -> Result<(), Error> = {
    (x_ref, p_ref) = p.get_x()?   # If ? propagates:
    # Brand tree knows x_ref is derived from p_ref (#42.field_x is prefix of #42)
    # Release order: x_ref (child) → p_ref (parent) → LIFO automatically satisfied
    p.modify()                     # WriteToken — all ReadTokens released
    Ok(())
}
```

Implementation location: kept in `ir_gen.rs`, changed to scope-driven — no new compiler pass introduced.

| Operation                    | Complexity | Trigger Frequency                           |
| ---------------------------- | ---------- | ------------------------------------------- |
| Brand tree conflict check    | O(1)       | Every token acquisition                     |
| DAG consumer query           | O(1)       | Every token acquisition                     |
| Reverse BFS (fast path)     | O(N)       | Every token acquisition, N = nodes in block |
| SMT logical cut (fallback)  | ~1ms       | **Extremely rare** — only while + path cond  |

**SMT fallback trigger conditions are extremely stringent**: must simultaneously satisfy (1) while loop (2) write operation in loop body (3) path condition after write operation that can determine loop termination (4) compiler needs this condition to cut back-edge. Actual code proportion is far below 1%. All other borrow checking completes in fast path.

Relationship with RFC-027 user predicates: user predicates use SMT as primary, borrow system predicates use structural analysis as primary. Both share same SMT solver and budget ceiling (RFC-027 §8), but borrow system predicates barely consume SMT budget.

Linear code → no back-edges → Tier 1 O(N) instant. Loops + path conditions → SMT call, linear arithmetic millisecond-level (RFC-027 budget 100ms). One BFS result can be cached for multiple queries of the same token.

### Error Message Design

**Core principle: Error messages only contain symbols the user wrote.**

Rust and borrow-related errors fall into two categories:

**Variable-level errors**: E0597 (doesn't live long enough), E0502 (mutable and immutable borrow in scope), E0499 (cannot borrow as mutable more than once). Rust is already the benchmark — variable name + line number, no `'a` shown. YaoXiang precision matches. All info in brand tree: token creation point, consumer position, request point.

**Signature-level errors**: E0623 (lifetime mismatch), E0106 (missing lifetime specifier), E0477 (required lifetime not satisfied). Centered around `'a`. YaoXiang **doesn't have these errors** — no `'a` in signatures. Not "can't report them" — things users didn't write don't need reporting.

Function-internal conflict example:

```
Error: `data` is frozen, cannot acquire mutable permission
 --> src/main.yx:5:9
2 |     view = &data
  |            ----- `data` frozen (read-only token created here)
4 |         use(view)
  |             ---- `view` still in use here, freeze not lifted
5 |         data.push(4)
  |         ^^^^ mutable permission needed here
```

(Matches Rust E0499 precision — variable name + line number, no brand ID shown.)

Cross-function escape example:

```
Error: one of `num`'s data sources (line 4) is `default_str` (line 3),
but `default_str` is invalidated at line 6 while `num` is still being used at line 5.

Consider: moving `default_str` declaration to caller, or using `ref default_str` for shared holding.
```

(Matches Rust E0597 precision. Brand summary knows `num` has two source paths — already in compiler, error wording can use them.)

---

## RFC-009 Body Amendments

RFC-009 §"Token Conflict Detection: Flow-Sensitive Liveness Analysis" has been updated:

1. Removed "unneeded things:... NLL" — not because conclusion is wrong, but because reasoning is wrong ("tokens are values, linear tracking is sufficient")
2. Tier 1/Tier 2 transition plan kept, complete plan points to this RFC
3. Clarified: Brand ID (`#42`) is just `'a` — same information, different encoding. Not invented new analysis — moved lifetimes from type layer to proof layer

---

## Tradeoffs

### Advantages

1. **No lifetimes in type signatures**: `#42` is just `'42` — same information, encoded in brand tree, not exposed in type signatures. This point is unfalsifiable: count how many `'a` parameters a generic type with 3 reference parameters needs in Rust, versus YaoXiang. Answer: 3 vs 0.

2. **Conceptual unification**: Borrow checking and user predicates share the same proof pipeline — `{P} op {Q}`, pipeline verifies P. Curry-Howard consistent.

3. **Zero new analysis framework**: No new analysis framework introduced. Users don't perceive the existence of a "borrow checker" — just as users don't perceive the implementation details of the "type checker."

4. **Error messages only contain symbols user wrote**: One entire dimension of error categories eliminated (E0623, E0106, E0477 — all centered around `'a`). Variable-level errors match Rust precision.

5. **Algorithm not conservative**: Reverse BFS + break cut + SMT logical cut. No "loop conservative liveness" needed. No "branch conservative merge" needed.

### Disadvantages

1. **Not a new invention**: Brand ID does exactly what `'a` does — compiler-internal constraint solving complexity doesn't disappear, only encoding changes from "variable name + constraint set" to "brand path + prefix matching." Difference to end users is only that signatures don't write `'a`.

2. **Brand new implementation**: Brand tree only exists as concept in code, needs implementation from scratch. BorrowChecker, ControlFlowAnalyzer replaced.

3. **SMT dependency**: Logical cut relies on Z3 (already introduced in RFC-027, no new dependencies). But borrow checking barely triggers — only in while + path conditions.

4. **Extremely rare patterns need refactoring**: Cross-branch borrowing that compiler can't auto-prove requires user code refactoring. Unlike Rust `'a` fallback: Rust has `'a` as the pen (annotate and pass); YaoXiang's fallback (proof functions) is not MVP.

---

## Alternatives

| Alternative                  | Why Not Chosen                                                                      |
| ---------------------------- | ----------------------------------------------------------------------------------- |
| Implement full Rust NLL      | YaoXiang's design constraints (no shadowing, for new bindings) already eliminate NLL's main complexity source, CFG fixpoint not needed |
| Keep current (hardcoded Release) | Insufficient — users must manually manage token scope                                   |
| Analysis only in spawn blocks | Insufficient — non-spawn code token usage is the majority                              |
| GC instead of borrow checking| Violates language design principle — YaoXiang has no GC                               |

---

## Implementation Phases

| Phase  | Content                                           | Dependencies               |
| ------ | ------------------------------------------------- | -------------------------- |
| Phase 1 | Brand tree data structure implementation          | —                          |
| Phase 2 | System predicate generators (Borrow/Move/Drop/Mut → propositions) | Phase 1               |
| Phase 3 | Reverse BFS liveness analysis + pipeline integration (Tier 1) | Phase 2             |
| Phase 4 | Path condition collection + SMT logical cut (Tier 2) | Phase 3 + RFC-027 Phase 2 |
| Phase 5 | Release instructions changed to DAG consumer-driven | Phase 3              |
| Phase 6 | Delete ControlFlowAnalyzer, refactor BorrowChecker | Phase 4             |

---

## Open Questions

- [x] **Brand tree `ref_count` semantics across iterations during loop unrolling** — follow NLL: token dies after last use. Copies bound inside loop die at iteration boundary, reverse BFS doesn't carry cross-iteration liveness. See §NLL and Iteration Boundaries.
- [x] **Token release order on `?` error propagation paths** — Release driven by scope analysis (kept in ir_gen.rs). Each scope exit point (`}`, `?`, explicit return) releases active tokens in LIFO order. Brand tree parent-child relationships automatically handle cascading release. See §`?` Error Propagation and Scope-Driven Release.
- [ ] Proof function syntax (far-term, not MVP — doesn't block any Phase)

---

## References

- [RFC-009: Ownership Model Design](../accepted/009-ownership-model.md) — Parent RFC
- [RFC-027: Compile-Time Predicates and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md) — Proof pipeline
- [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md) — `{}` semantics
- [RFC-024: Concurrency Model Based on spawn Blocks](../accepted/024-concurrency-model.md) — spawn DAG

---

## Lifecycle and Disposition

| Status       | Location                      | Description              |
| ------------ | ----------------------------- | ------------------------ |
| **Accepted** | `docs/design/rfc/accepted/`   | Becomes formal design doc |
