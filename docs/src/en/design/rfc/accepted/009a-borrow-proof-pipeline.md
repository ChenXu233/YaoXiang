---
title: "RFC-009a: Token Lifetime Analysis — Hoare Proof Pipeline Based"
status: "Accepted"
author: "Chenxu"
created: "2026-06-13"
updated: "2026-06-13"
group: "rfc-009"

issue: "#129"

impl: "partial"
---

# RFC-009a: Token Lifetime Analysis — Hoare Proof Pipeline Based

> **Parent RFC**: [RFC-009: Ownership Model Design](../accepted/009-ownership-model.md)
>
> **Dependency**: [RFC-027: Compile-time Predicates and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md)
>
> **Prerequisite**: RFC-027 must be accepted. All mechanisms in this RFC (proof pipeline, SMT fallback, path condition collection) depend on RFC-027's implementation.
>
> **This RFC corrects and replaces RFC-009 §"Token Conflict Detection: Flow-Sensitive Liveness Analysis" (lines 663-684).**

## Summary

RFC-009 line 684 claims token conflict detection "does not need...NLL." The conclusion is correct; the reasoning is wrong.

It is not "because tokens are values, linear tracking is enough." It is because: **token liveness is a Hoare logic proposition, not a dedicated flow-sensitive analysis.**

`{conflicting_tokens all dead} op {WriteToken safely acquired}` — the same `{P} op {Q}`, sharing the proof pipeline with type checking and predicate verification. No new analysis framework. One pipeline, multiple propositions.

---

## Motivation

### RFC-009's Confusion

RFC-009 conflates two problems:

1. **Linear tracking** (unavailable after Move) — `{v not moved} use(v) {type matches}`. The type checker already has this.
2. **Token lifetime interaction** (child token alive → parent token paused → child token dead → parent token revived) — `{conflicting_tokens all dead} write(data) {safe}`. Requires **liveness analysis**, not linear tracking.

### The Reality of Current Code

| Component | Status |
|------|------|
| `BorrowChecker` | Linearly scans IR, passively responding to explicit `Borrow`/`Release` instructions |
| `ControlFlowAnalyzer::analyze_instruction` | Empty implementation (`control_flow.rs:145-153`) |
| `liveness_analysis` | Exists but only used for Drop insertion, not connected to token conflicts |
| Release insertion | Hardcoded after Call instruction — purely lexical scope (`ir_gen.rs:2734-2736`) |

**User-visible consequences**:

```yaoxiang
data = vec![1, 2, 3]
view = &data              # Create ReadToken
x = view.total_count      # Last use of view
data.push(4)              # ❌ Release(view) not yet executed, ReadToken "alive"
```

### Why a Rewrite Is Needed

The previous version (009a v1) used the "DAG replaces NLL" narrative, introducing unnecessary new concepts (conservative branching rules, special loop handling). The core contradiction was not made clear: **borrow checking is not an independent system — it is a kind of Hoare proposition.**

---

## Core Design

### Everything Is Hoare

```
Type check:     { x: Int }        x + 1        { result: Int }
Borrow check:   { view is dead }  data.push(4) { WriteToken acquired successfully }
Predicate check: { y > 0 }         divide(x, y) { result: Int }
Backedge cut:   { i == n }        next loop    { cond == false }
```

The same form `{P} op {Q}`. The compiler generates a precondition P for each operation and sends it to the proof pipeline for verification.

**Borrow checking and user predicates share the same pipeline.** The only difference is who generates the proposition and what happens when it fails to prove.

### Two Types of Predicates, One Pipeline

| | User Predicate | System Predicate (Borrow) |
|---|---|---|
| Proposition generation | Programmer (type annotations) | Compiler (brand tree + ownership rules) |
| Proof provider | Compiler + programmer | **Fully automatic by compiler** |
| When proof fails | Write proof function or refactor | Refactor code (escape hatch exists but rarely needed) |
| Visibility | Visible in signature | Implicit, does not pollute type signatures |
| Learning cost | Learn only if you want to use it | Zero |

**The proof of system predicates does not open a proof function for the programmer — it is fully automatic by the compiler.** When the proof fails, the user refactors the code.

**Three failure modes, one verification engine.** Type proposition fails to prove → compile error (cannot bypass). Borrow proposition fails to prove → compile error, refactor (cannot bypass). User predicate fails to prove → compile error, can write proof function (can bypass). Failure strategies differ, but the verification engine is the same — SMT solver + compiler kernel inference rules. The only difference is "who is responsible for providing the proof when it fails to prove" — the compiler refuses to write borrow proofs for the programmer (the proof strategy for borrow propositions is structural analysis + SMT, requiring no programmer involvement), but accepts programmer-written user predicate proof functions. This is not pipeline inconsistency — it is different responsibility boundaries for different proposition categories.

This differs from Rust `'a`: `'a` is a required course, proof functions are elective — the vast majority of users will never touch the door of the elective course.

### Borrow Propositions: Automatically Generated by the Compiler

The user writes `data.push(4)`. The compiler automatically generates the proposition:

```
WriteToken(data, node) is acquirable
  = forall t in conflicting_tokens(data): t is dead at node
  = forall t in brand_tree.children(data): forward_reachable(node) ∩ consumers(t) == ∅
```

**Three rules, zero special cases:**

1. **Brand tree** (RFC-009 §2.7) answers "who conflicts with whom": prefix matching, O(depth), depth ≤ 3
2. **Consumer list** (automatically collected during DAG construction) answers "who was the last consumer of the token"
3. **Forward reachability** answers "can the consumer still be executed": structural cut + logical cut

### Forward Reachability: Reverse from Consumers

For each consumer C of token T:

```
Starting from C, reverse BFS the DAG.
An edge is cut if:
  1. It is a break (structural cut)
  2. The path condition ⇒ !loop_cond is proven true by SMT (logical cut, RFC-027 pipeline)

Propagate backward along all uncut edges (including backedges; backedges propagate liveness to the previous iteration).
Mark all reachable nodes → unsafe.
```

Query: write operation at node W → W ∉ unsafe → safe.

**No need to invent "conservative branching rules." No "conservative loop survival." One reverse BFS + two cut rules.**

### Proof Strategy: Fast Lane First, SMT as Fallback

```
Every write operation that needs a token
  │
  ├→ Fast lane: DAG structural analysis (covers 95%+ scenarios)
  │     │
  │     ├→ Brand tree prefix matching → find conflicting tokens (O(depth))
  │     ├→ Reverse BFS, break cuts backedges
  │     └→ No backedge can be crossed → directly determine Proved / Disproved
  │
  └→ Slow lane: SMT logical cut (only when fast lane encounters crossable backedges)
        │
        ├→ Backedge start has path condition → SMT judges path_cond ⇒ !loop_cond
        │     ├→ Proved → logical cut → downgrade to fast lane to continue
        │     └→ Disproved / Unproven → backedge crossed → mark unsafe
        │
        └→ Backedge start has no path condition → backedge crossed directly
```

**Fast lane coverage**: linear code, if/else, loop + break, while without path conditions.
**Slow lane coverage**: while loop body, when path conditions suggest the loop will exit.
**Not covered**: runtime conditions cannot be statically proven → backedge crossed → unsafe → compile error (user refactors).

SMT is not the main force — it is a safety net. Unlike RFC-027's user predicates: user predicates rely on SMT as the main force; borrow system predicates rely on structural analysis as the main force, with SMT only patching edges that structural analysis cannot reach.

---

## Use Case Analysis

### Linear Code

```yaoxiang
data = vec![1, 2, 3]        # Node 1
view = &data                # Node 2: consumes data, produces ReadToken(#1)
x = view.total_count        # Node 3: consumes view (= last consumer of #1)
data.push(4)                # Node 4: needs WriteToken(data)
```

Reverse BFS from `view.total_count` (node 3) → node 3 is the last consumer of #1 → node 4 > node 3 → node 4 is not in unsafe → ✅

### if/else: No Special Rules

```yaoxiang
view = &data
if cond {
    use(view)               # then branch consumes view
} else {
    do_something_else()     # does not touch view
}
data.push(4)                # Last consumer of view is inside if → no consumer after if → ✅
```

if/else is a composite node in the DAG. Internal consumption is attributed to this node. Branch states are not merged. No conservative voting. **Whether there is a consumer afterward, an integer comparison.**

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

view escapes via return value → `use(result)` is a consumer of view → reverse walking from `push` can reach `use(result)` → unsafe.

### Loop: break Cuts the Backedge

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

Reverse BFS from `use(view)` → backedge → forward walk to `data.push(4)` → encounters `break` → **cut** → `data.push(4)` is not in unsafe → ✅

Without break:

```yaoxiang
view = &data
loop {
    use(view)
    data.push(4)             # no break cut → backedge can be crossed → next iteration use(view) is reachable
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

Reverse BFS from `use(view)` → backedge → walks to `data.push(4)` → check path condition `i == n` → SMT query: `i == n ⇒ !(i < n)`? → Proved → **logical cut** → `data.push(4)` is not in unsafe → ✅

---

## Essence: Brand ID Is `'a`

Do not say "we don't need `'a`." Say "`#42` is `'42`."

| Rust | YaoXiang | Equivalence |
|------|----------|--------|
| `'a` | `#42` | Compile-time lifetime identifier |
| `'a: 'b` outlives constraint | `#42` is the prefix of `#42.field_x` | String prefix comparison = partial order |
| NLL liveness propagation (CFG fixed point) | Reverse BFS (DAG) | Both are reachability computations |
| Polonius facts | SMT logical cut | Both are path condition reasoning |
| Constraint system fixed-point solving | Brand tree prefix matching + BFS | Different encoding, same problem |

**We did not invent a new analysis. We just lowered `'a` from the type signature layer to the proof layer.** Brand ID does exactly what `'a` does — mark borrow identities, track derivation relationships, determine conflicts. The only difference: `'a` is in the user-written type signature; `#42` is inside the compiler.

This is not shameful. Curry-Howard says types are propositions, programs are proofs. `'a` is not part of the proposition — it is part of the proof strategy. Rust wrote the proof strategy into the proposition signature. We put it back where it belongs.

### What Language Design Constraints Eliminate

| Source of Complexity | Avoided? | Reason |
|---|---|---|
| Variable shadowing | ✅ | Language prohibits — one name always points to the same thing |
| Cross-iteration borrowing in for | ✅ | Each iteration is a new binding — natural isolation between iterations |
| `'a` lifetime annotation | ✅ | Brand path = `#42.field_x`, inferred by compiler |
| Named lifetime + constraint propagation | ✅ | Brand path prefix comparison replaces explicit constraint sets |
| Borrow graph constraint solving (Polonius) | ✅ | Brand tree prefix matching + DAG consumer query |
| Liveness propagation of borrows in loop bodies | ❌ | Same as Rust — handled with reverse BFS + logical cut |
| Conditional branch conservativeness | ❌ | Same as Rust — SMT covers provable, remainder conservatively rejected |

### Why DAG Is Feasible

Three language design constraints of YaoXiang make DAG analysis feasible:

- **No variable shadowing** — one name always points to the same thing, no need to track across rebinding
- **for creates new binding per iteration** — natural isolation between iterations, no cross-iteration borrowing
- **Structured concurrency** — clear task boundaries, no cross-task liveness propagation

These constraints eliminate the main sources of complexity in Rust's CFG fixed-point iteration. It is not that DAG is "more advanced" than CFG — it is that a simpler language design allows a simpler analysis.

---

## Detailed Design

### System Predicate List

The compiler automatically generates the following propositions and sends them to the RFC-027 proof pipeline:

| System Predicate | Trigger Timing | Proposition Form |
|---|---|---|
| `borrow_conflict` | Need WriteToken(v) | `forall t ∈ conflicting(v): dead_at(t, node)` |
| `use_after_move` | Use variable v | `¬moved(v)` |
| `use_after_drop` | Use variable v | `¬dropped(v)` |
| `double_drop` | Drop(v) | `¬dropped(v)` |
| `mut_violation` | Write immutable variable v | `is_mut(v)` |

The existing `BorrowChecker`, `MoveChecker`, `DropChecker`, `MutChecker` **become proposition generators** — they don't disappear, they change identity. They generate propositions, and the pipeline verifies them.

### Brand Tree

The brand mechanism from RFC-009 §2.7 is formalized as a brand tree.

**Token semantics — freeze-first, not copy-first**:

The essential difference between `&T` and `&mut T` is not "can it be copied" but "is simultaneous writing allowed":

```
ReadToken(T):  Grants read-only permission, simultaneously freezing the source data T — any
              WriteToken(T) cannot be acquired during this period. Freezing is ReadToken's
              primary semantics. Dup (copiable) is a corollary of freezing: since the data
              is already frozen (no mutation possible), multiple read-only views are
              naturally safe.

WriteToken(T): Grants exclusive read-write permission. Because writes exist, no other
              token (read or write) can coexist. It does not implement Dup (linear type)
              is a corollary of exclusivity.
```

**Causal relationship**:
```
ReadToken exists → source data frozen → multiple read-only copies safe → Dup
                            ↓
              WriteToken rejected (enforced by borrow_conflict system predicate)
```

Not:
```
ReadToken has Dup → can have multiple → check conflicts as a side note  ← inverted causality
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
  ref_count: usize          # Number of safe copies during ReadToken freezing
```

**Conflict determination** — execution mechanism guaranteed by freezing:

```rust
fn conflicts(a: &BrandId, b: &BrandId) -> bool {
    // Conflict condition: same source + at least one is a write + brand path overlap
    // This means:
    //   1. ReadToken vs ReadToken → no conflict (both read-only, no mutation)
    //   2. WriteToken vs ReadToken → conflict (write breaks read's freezing guarantee)
    //   3. WriteToken vs WriteToken → conflict (two writes cannot coexist)
    a.source() == b.source()
        && (a.is_write() || b.is_write())
        && (a.is_prefix_of(b) || b.is_prefix_of(a))
}
```

O(depth) string prefix comparison, depth ≤ 3. Constant level.

### Reverse BFS Liveness Analysis

```
Algorithm: check_borrow(token, node, dag, brand_tree)

Inputs:
  token: WriteToken that needs to be checked
  node:  DAG node where the write operation is located

Output: Proved | Disproved

Algorithm:
  # Fast lane: reverse BFS
  unsafe = empty_set
  queue = brand_tree.consumers(token)

  while queue not empty:
    cur = queue.pop()
    unsafe.add(cur)

    for each pred in dag.predecessors(cur):
      # Structural cut: break does not cross
      if pred is a break edge:
        continue

      # Backedge → check whether SMT fallback is needed
      if pred is a backedge:
        path_cond = path condition at pred
        loop_cond = loop condition
        # First see if it can be cut structurally (corresponding break has cut the path → won't reach here)
        # Then see the path condition
        if path_cond is non-empty:
          result = smt_fallback(path_cond, loop_cond)   # ← slow lane
          if result == Proved:
            continue                    # logical cut
        # No path condition or SMT failed to prove → cross the backedge
        # fall through

      if pred ∉ unsafe:
        queue.push(pred)

  # Determination
  if node ∈ unsafe:
    return Disproved
  else:
    return Proved


smt_fallback(path_cond, loop_cond):
  # Only called when there is a backedge + a path condition
  # Uses the RFC-027 proof pipeline, sharing the same SMT solver and the same budget
  return smt.prove(path_cond ⇒ !loop_cond)
  # Proved → logical cut
  # Disproved / Unproven → no cut, backedge crossed
```

O(N), where the number of SMT calls = number of backedges × proportion of backedges with path conditions. In actual code, SMT calls are extremely rare — only triggered in `while` loop bodies with refined type variable path conditions.

### Path Condition Collection

Provided by existing RFC-027 §3.2-3.3 mechanisms:

- **if guard**: `if y > 0` → push `y > 0` into the true branch
- **match pattern**: `if let Some(v) = opt` → push `opt == Some(v)` into the branch
- **Assignment**: `i += 1`, compiler maintains variable value domain information
- **while cond**: push `cond == true` into the loop body

Each DAG node carries a set of path conditions. When reverse BFS encounters a backedge, take the path condition at the backedge start, and let SMT determine whether the next loop entry condition is excluded.

### Interface with RFC-027

Borrow system predicates and user predicates share the same proof pipeline — the difference lies in the **main proof strategy**:

| Query Type | Proposition Source | Main Strategy | Fallback |
|---|---|---|---|
| Type equality | Type checker | Structural equivalence | — |
| User predicate | Programmer type annotation | SMT | Programmer proof function |
| Borrow conflict | Compiler auto-generated | **DAG structural analysis (fast lane)** | SMT logical cut |

SMT solver's role in borrow checking: **not the main force, but a safety net.** Only called when while backedges need logical cuts. The vast majority of borrow checks are completed in the fast lane — O(N) reverse BFS, zero SMT overhead.

### Relationship with Existing Code

| Existing Component | Treatment |
|----------|------|
| `BorrowChecker` | Becomes `BorrowPredicateEmitter` — generates Hoare propositions for borrows |
| `MoveChecker` | Becomes `MovePredicateEmitter` — generates `¬moved(v)` propositions |
| `DropChecker` | Same — generates Drop-related propositions |
| `MutChecker` | Same — generates `is_mut(v)` propositions |
| `ControlFlowAnalyzer` | No longer needed — handled uniformly by the pipeline |
| `liveness_analysis` | Retained — Drop insertion still needs variable liveness information |
| `ir_gen.rs` Release hardcoding | Removed — Release positions driven by DAG consumer analysis |

### NLL and Iteration Boundaries

**Token death time = last use point (NLL), not lexical scope end.**

This is a natural corollary of consumer analysis: the position of the consumer defines the last use of the token. `use(v)` is a consumer of `v` → `v` dies immediately after `use(v)`. No additional `{}` or `drop()` is needed to terminate the token lifetime early.

**Loop iteration boundaries are the death line of token copies.** Three rules:

```
Rule 1: Variables declared inside a loop die automatically at the end of each iteration.
        Each iteration of for is a new binding (guaranteed by language design), same for loop.

Rule 2: The brand tree ref_count at the loop header only counts copies created outside the loop.
        New copies produced by Dup inside the loop have ref_count cleared at iteration boundaries.

Rule 3: When reverse BFS crosses a backedge, it does not carry the current iteration's liveness information.
        Only the ref_count at the loop header is carried (i.e., copies from outside the loop).
```

Example:

```yaoxiang
view = &data                          # Loop header: ref_count = 1, consumer = use(view)
loop {
    v2: &Point = view                 # Dup inside loop → ref_count = 2
    use(v2)                           # consumer: last use of v2 → v2 dies → ref_count = 1
    data.push(4)                      # ✅ Safe! v2 is dead, only view remains (ref_count = 1, not a write conflict)
    # Iteration boundary: Rule 3 — v2 does not carry into the next iteration. At the start of the next iteration, v2 is recreated by a new binding.
}
```

This design does not require an additional "conservative loop survival" rule. Reverse BFS starts from the consumer, the consumer is in the loop body → liveness is confined to the current iteration → backedge is not crossed. This is completely consistent with the loop examples in RFC-009a §Use Case Analysis.

### `?` Error Propagation and Scope-Driven Release

`?` is an early return — there is an additional exit path beyond the scope's normal exit. The token must be released on this path; an incorrect release order results in UB.

**Release instructions are generated by scope analysis, not hardcoded after Call.**

The compiler maintains a list of exit points for each scope:
- `}` (normal end of scope)
- `?` (error propagation, early return)
- Explicit `return`

At each exit point, insert Release instructions for all active tokens within that scope in declaration reverse order (LIFO). The parent-child relationship of the brand tree automatically handles cascading release of derived tokens:

```yaoxiang
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)    # Return child token &Float + parent token &Point
}

fn use_case(p: Point) -> Result<(), Error> = {
    (x_ref, p_ref) = p.get_x()?   # If ? propagates:
    # The brand tree knows x_ref is derived from p_ref (#42.field_x is a prefix of #42)
    # Release order: x_ref (child) → p_ref (parent) → LIFO automatically satisfied
    p.modify()                     # WriteToken — all ReadTokens have been released
    Ok(())
}
```

Implementation location: kept in `ir_gen.rs`, changed to scope-driven — no new compiler pass introduced.

| Operation | Complexity | Trigger Frequency |
|------|--------|---------|
| Brand tree conflict determination | O(1) | Every token needed |
| DAG consumer query | O(1) | Every token needed |
| Reverse BFS (fast lane) | O(N) | Every token needed, N = nodes in block |
| SMT logical cut (fallback) | ~1ms | **Extremely rare** — only while + path condition |

**The trigger conditions for SMT fallback are extremely stringent**: must simultaneously satisfy (1) while loop (2) write operation inside the loop body (3) path condition after the write operation that can determine loop termination (4) the compiler needs to rely on this condition to cut the backedge. In actual code, the proportion is far less than 1%. All other borrow checks are completed in the fast lane.

Relationship with RFC-027 user predicates: user predicates rely on SMT as the main force, borrow system predicates rely on structural analysis as the main force. Both share the same SMT solver and budget cap (RFC-027 §8), but borrow system predicates almost never consume SMT budget.

Linear code → no backedges → O(N) at tier 1, instant. Loop + path condition → SMT call, linear arithmetic millisecond level (RFC-027 budget 100ms). One BFS result can be cached for reuse by multiple queries for the same token.

### Error Message Design

**Core principle: error messages only contain symbols the user has written.**

Rust's borrow-related errors fall into two categories:

**Variable-level errors**: E0597 (does not live long enough), E0502 (mutable + immutable simultaneous borrow), E0499 (multiple mutable borrows). Rust is already the benchmark — variable name + line number, no `'a` appears. YaoXiang matches in precision. All information is in the brand tree: token creation point, consumer position, request point.

**Signature-level errors**: E0623 (lifetime mismatch), E0106 (missing lifetime specifier), E0477 (required lifetime not satisfied). Centered on `'a`. YaoXiang **does not have this class of errors** — there is no `'a` in the signature. Not "cannot report," but things the user did not write do not need to be reported.

In-function conflict example:

```
Error: `data` is frozen; mutable permission cannot be acquired
 --> src/main.yx:5:9
2 |     view = &data
  |            ----- `data` is frozen (read-only token created here)
4 |         use(view)
  |             ---- `view` is still in use here; freezing not lifted
5 |         data.push(4)
  |         ^^^^ mutable permission needed here
```

(Matches Rust E0499 in precision — variable name + line number, no brand ID appears.)

Cross-function escape example:

```
Error: `num` (line 4) holds data with one source being `default_str` (line 3),
but `default_str` is invalidated at line 6, while `num` is still being used at line 5.

Consider: move `default_str`'s declaration up to the caller, or use `ref default_str` to share ownership.
```

(Matches Rust E0597 in precision. The brand summary knows `num` has two source paths — already in the compiler, wording available for the error message.)

---

## RFC-009 Body Corrections

RFC-009 §"Token Conflict Detection: Flow-Sensitive Liveness Analysis" has been updated:

1. Remove "things not needed:...NLL" — not because the conclusion is wrong, but because the reasoning is wrong ("tokens are values, linear tracking is enough")
2. Tier 1/Tier 2 transition plan retained, complete solution points to this RFC
3. Clarify: Brand ID (`#42`) is `'a` — exactly the same information, different encoding. Not a new analysis invented — the lifetime is lowered from the type layer to the proof layer

---

## Trade-offs

### Advantages

1. **Type signatures contain no lifetimes**: `#42` is `'42` — the same information, encoded in the brand tree, not exposed in the type signature. This point is unfalsifiable: count how many `'a` parameters a generic type with 3 reference parameters needs in Rust, versus how many in YaoXiang. The answer is 3 vs 0.

2. **Conceptual unification**: Borrow checking and user predicates share the same proof pipeline — `{P} op {Q}`, the pipeline verifies P. Curry-Howard consistent.

3. **Zero new analysis framework**: No new analysis framework introduced. Users are unaware of the existence of a "borrow checker" — just as users are unaware of the implementation details of a "type checker."

4. **Error messages only contain symbols the user has written**: An entire dimension of error categories is eliminated (E0623, E0106, E0477 — all centered on `'a`). Variable-level errors match Rust in precision.

5. **Algorithm is not conservative**: Reverse BFS + break cut + SMT logical cut. No need for "conservative survival within loops." No need for "conservative branch merging."

### Disadvantages

1. **Not a new invention**: Brand ID does exactly what `'a` does — the constraint solving complexity inside the compiler has not disappeared, only the encoding has changed from "variable name + constraint set" to "brand path + prefix matching." The difference for end users is only that `'a` is not written in the signature.

2. **Brand new implementation**: The brand tree only exists as a concept in the code; it needs to be implemented from scratch. BorrowChecker, ControlFlowAnalyzer are replaced.

3. **SMT dependency**: Logical cut depends on Z3 (RFC-027 has already introduced it, no new dependency). But borrow checking almost never triggers it — only when there is while + path condition.

4. **Very few patterns require refactoring**: Cross-branch borrows that the compiler's automatic proof cannot cover, the user needs to refactor the code. Different from Rust `'a`'s fallback: Rust has `'a` as a pen (annotate and it passes); YaoXiang's fallback (proof function) is not MVP.

---

## Alternative Solutions

| Solution | Why Not Chosen |
|------|--------|
| Implement complete Rust NLL | YaoXiang's design constraints (no shadowing, for new bindings) have already eliminated the main sources of NLL complexity; CFG fixed point is not needed |
| Keep the current (hardcoded Release) | Not enough — users must manually manage token scopes |
| Only do analysis in spawn blocks | Not enough — token usage in non-spawn code is the majority |
| GC replaces borrow checking | Violates language design principles — YaoXiang has no GC |

---

## Implementation Phases

| Phase | Content | Dependency |
|------|------|------|
| Phase 1 | Brand tree data structure implementation | — |
| Phase 2 | System predicate generators (Borrow/Move/Drop/Mut → propositions) | Phase 1 |
| Phase 3 | Reverse BFS liveness analysis + pipeline integration (tier 1) | Phase 2 |
| Phase 4 | Path condition collection + SMT logical cut (tier 2) | Phase 3 + RFC-027 Phase 2 |
| Phase 5 | Release instructions changed to DAG consumer-driven | Phase 3 |
| Phase 6 | Remove ControlFlowAnalyzer, refactor BorrowChecker | Phase 4 |

---

## Open Questions

- [x] **Cross-iteration semantics of `ref_count` in brand tree during loop unrolling** — adopt NLL: the token dies after the last use. Copies bound inside the loop die at the iteration boundary; reverse BFS does not carry liveness across iterations. See §NLL and Iteration Boundaries.
- [x] **Token release order on `?` error propagation paths** — Release is driven by scope analysis (kept in ir_gen.rs). At each scope exit point (`}`, `?`, explicit return), active tokens are released in LIFO order. Brand tree parent-child relationship automatically handles cascading release. See §`?` Error Propagation and Scope-Driven Release.
- [ ] Proof function syntax (far term, not MVP — does not block any Phase)

---

## References

- [RFC-009: Ownership Model Design](../accepted/009-ownership-model.md) — Parent RFC
- [RFC-027: Compile-time Predicates and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md) — Proof pipeline
- [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md) — `{}` semantics
- [RFC-024: spawn Block Based Concurrency Model](../accepted/024-concurrency-model.md) — spawn DAG

---

## Lifecycle and Destination

| Status | Location | Description |
|------|------|------|
| **Accepted** | `docs/design/rfc/accepted/` | Becomes formal design document |