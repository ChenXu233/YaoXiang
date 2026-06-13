---
title: "RFC-009a: Token Lifetime Analysis — Hoare-Based Proof Pipeline"
status: "Accepted"
author: "Chenxu"
created: "2026-06-13"
updated: "2026-06-13"
group: "rfc-009"
---

# RFC-009a: Token Lifetime Analysis — Hoare-Based Proof Pipeline

> **Parent RFC**: [RFC-009: Ownership Model Design](../accepted/009-ownership-model.md)
>
> **Dependency**: [RFC-027: Compile-time Predicates and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md)
>
> **Prerequisite**: RFC-027 must be accepted. All mechanisms in this RFC (the proof pipeline, SMT fallback, path condition collection) depend on RFC-027's implementation.
>
> **This RFC corrects and supersedes RFC-009 §"Token Conflict Detection: Flow-Sensitive Liveness Analysis" (lines 663-684).**

## Abstract

RFC-009 line 684 claims token conflict detection "does not need...NLL". The conclusion is correct; the argument is wrong.

It is not "because tokens are values, so linear tracking is enough." It is because: **token liveness is a Hoare logic proposition, not an ad hoc flow-sensitive analysis.**

`{conflicting_tokens all dead} op {WriteToken safely acquired}` — the same `{P} op {Q}`, sharing RFC-027's proof pipeline with type checking and predicate verification. No new analysis framework. One pipeline, many propositions.

---

## Motivation

### RFC-009's Confusion

RFC-009 conflates two problems:

1. **Linear tracking** (unavailable after Move) — `{v not moved} use(v) {types match}`. The type checker already has this.
2. **Token lifetime interaction** (child token alive → parent token suspended → child token dies → parent token revived) — `{conflicting_tokens all dead} write(data) {safe}`. Requires **liveness analysis**, not linear tracking.

### Current Code Reality

| Component | Status |
|------|------|
| `BorrowChecker` | Linearly scans IR, passively responds to explicit `Borrow`/`Release` instructions |
| `ControlFlowAnalyzer::analyze_instruction` | Empty implementation (`control_flow.rs:145-153`) |
| `liveness_analysis` | Exists but used only for Drop insertion, not wired into token conflict detection |
| Release insertion | Hardcoded after Call instructions — purely lexical scope (`ir_gen.rs:2734-2736`) |

**User-visible consequences**:

```yaoxiang
data = vec![1, 2, 3]
view = &data              # creates ReadToken
x = view.total_count      # last use of view
data.push(4)              # ❌ Release(view) not yet executed, ReadToken is "alive"
```

### Why a Rewrite Is Needed

The previous version (009a v1) used a "DAG replaces NLL" narrative, introducing unnecessary new concepts (conservative branch rules, special-case loop handling). The core contradiction was left unstated: **borrow checking is not an independent system — it is one kind of Hoare proposition.**

---

## Core Design

### Everything Is Hoare

```
Type checking:   { x: Int }        x + 1        { result: Int }
Borrow checking: { view is dead }  data.push(4) { WriteToken acquired successfully }
Predicate check: { y > 0 }         divide(x, y) { result: Int }
Backedge cutoff: { i == n }        next loop    { cond == false }
```

The same form `{P} op {Q}`. The compiler generates precondition P for each operation and feeds it into the proof pipeline for verification.

**Borrow checking and user predicates share the same pipeline.** The only differences are who generates the proposition and what happens when it fails to prove.

### Two Kinds of Predicates, One Pipeline

| | User predicate | System predicate (borrow) |
|---|---|---|
| Proposition generation | Programmer (type annotations) | Compiler (brand tree + ownership rules) |
| Proof provision | Compiler + programmer | **Compiler fully automatic** |
| Failure handling | Write proof function or refactor | Refactor code (gates exist but rarely needed) |
| Visibility | Visible in signatures | Implicit, doesn't pollute type signatures |
| Learning cost | Learn only if you use it | Zero |

**The compiler does not open proof functions to programmers for system predicates — it does them fully automatically.** When a proof fails, the user refactors the code.

**Three failure modes, one verification engine.** Type propositions unprovable → compile error (cannot bypass). Borrow propositions unprovable → compile error, refactor (cannot bypass). User predicate propositions unprovable → compile error, may write proof function (bypassable). The failure strategies differ, but the verification engine is the same — an SMT solver plus compiler-internal inference rules. The only difference is "who is responsible for supplying the proof when it fails" — the compiler refuses to write borrow proofs on behalf of the programmer (borrow-proposition proof strategy is structural analysis + SMT, needing no programmer intervention), but accepts programmer-written proof functions for user predicates. This is not pipeline inconsistency — it is a difference in responsibility boundaries across proposition categories.

This differs from Rust's `'a`: `'a` is a required course, proof functions are electives — most users never encounter the elective's door in their lifetime.

### Borrow Propositions: Compiler-Generated

The user writes `data.push(4)`. The compiler automatically generates the proposition:

```
WriteToken(data, node) acquirable
  = forall t in conflicting_tokens(data): t is dead at node
  = forall t in brand_tree.children(data): forward_reachable(node) ∩ consumers(t) == ∅
```

**Three rules, zero special cases:**

1. **Brand tree** (RFC-009 §2.7) answers "who conflicts with whom": prefix matching, O(depth), depth ≤ 3
2. **Consumer list** (auto-collected during DAG construction) answers "who last consumed the token"
3. **Forward reachability** answers "can the consumer still be executed": structural cutoffs + logical cutoffs

### Forward Reachability: Backward Walk from Consumers

For each consumer C of token T:

```
Starting from C, reverse-BFS the DAG.
An edge is cut if:
  1. It is a break (structural cutoff)
  2. The path condition ⇒ !loop_cond is proven by SMT (logical cutoff, RFC-027 pipeline)

Propagate backward along all uncut edges (including backedges; backedges carry liveness into the previous iteration).
Mark all reachable nodes → unsafe.
```

Query: a write operation at node W → W ∉ unsafe → safe.

**No need to invent "conservative branch rules." No "conservative-alive in loops." One reverse BFS plus two cutoff rules.**

### Proof Strategy: Fast Path First, SMT as Safety Net

```
Each write operation that needs a token
  │
  ├→ Fast path: DAG structural analysis (covers 95%+ of cases)
  │     │
  │     ├→ Brand tree prefix match → find conflicting tokens (O(depth))
  │     ├→ Reverse BFS, break cuts backedges
  │     └→ No backedge traversable → directly decide Proved / Disproved
  │
  └→ Slow path: SMT logical cutoff (only when fast path encounters a traversable backedge)
        │
        ├→ Backedge start has a path condition → SMT checks path_cond ⇒ !loop_cond
        │     ├→ Proved → logical cutoff → downgrade back to fast path
        │     └→ Disproved / Unproven → traverse backedge → mark unsafe
        │
        └→ Backedge start has no path condition → traverse backedge directly
```

**Fast path covers**: linear code, if/else, loop + break, while without path conditions.
**Slow path covers**: while loop bodies with path conditions implying loop exit.
**Does not cover**: runtime conditions that cannot be statically proven → backedge traversed → unsafe → compile error (user refactors).

SMT is not the main force — it is a safety net. Unlike RFC-027's user predicates (where SMT is the main force), the borrow system predicates rely on structural analysis as the main force, with SMT filling in the corners structural analysis cannot reach.

---

## Use Case Analysis

### Linear Code

```yaoxiang
data = vec![1, 2, 3]        # node 1
view = &data                # node 2: consumes data, produces ReadToken(#1)
x = view.total_count        # node 3: consumes view (= last consumer of #1)
data.push(4)                # node 4: needs WriteToken(data)
```

Reverse BFS from `view.total_count` (node 3) → node 3 is the last consumer of #1 → node 4 > node 3 → node 4 not in unsafe → ✅

### if/else: No Special Rule

```yaoxiang
view = &data
if cond {
    use(view)               # then branch consumes view
} else {
    do_something_else()     # doesn't touch view
}
data.push(4)                # view's last consumer is in the if → no consumers after if → ✅
```

An if/else is a compound node in the DAG. Internal consumption is attributed to this node. No merging of branch states. No conservative voting. **Whether a consumer exists afterward is an integer comparison.**

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
                             # → push in unsafe → ❌ correctly rejected
```

view escapes via return value → `use(result)` is a consumer of view → walking backward from `push` reaches `use(result)` → unsafe.

### Loop: break Cuts the Backedge

```yaoxiang
view = &data
loop {
    use(view)                # consumer
    if is_last {
        data.push(4)         # write
        break                # ← structural cutoff
    }
}
```

Reverse BFS from `use(view)` → backedge → walk forward to `data.push(4)` → hits `break` → **cut** → `data.push(4)` not in unsafe → ✅

Without break:

```yaoxiang
view = &data
loop {
    use(view)
    data.push(4)             # no break cutoff → backedge traversable → next iteration's use(view) reachable
                             # → push in unsafe → ❌ correctly rejected
}
```

### while: SMT Logical Cutoff

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

Reverse BFS from `use(view)` → backedge → walk to `data.push(4)` → check path condition `i == n` → SMT query: `i == n ⇒ !(i < n)`? → Proved → **logical cutoff** → `data.push(4)` not in unsafe → ✅

---

## The Essence: Brand ID Is `'a`

Not "we don't need `'a`." Rather, "`#42` *is* `'42`."

| Rust | YaoXiang | Equivalence |
|------|----------|------------|
| `'a` | `#42` | Compile-time lifetime identifier |
| `'a: 'b` outlives constraint | `#42` is a prefix of `#42.field_x` | String prefix comparison = partial order |
| NLL liveness propagation (CFG fixed point) | Reverse BFS (DAG) | Both are reachability computations |
| Polonius facts | SMT logical cutoffs | Both are path-condition reasoning |
| Constraint system fixed-point solving | Brand tree prefix match + BFS | Different encoding, same problem |

**We did not invent a new analysis. We simply lowered `'a` from the type-signature layer to the proof layer.** What a brand ID does is exactly what `'a` does — mark borrow identity, track derivation, decide conflict. The only difference: `'a` lives in the type signatures users write; `#42` lives inside the compiler.

This is nothing to be ashamed of. Curry-Howard says types are propositions, programs are proofs. `'a` is not part of the proposition — it is part of the proof strategy. Rust writes proof strategy into the proposition signature. We put it back where it belongs.

### What Language Design Constraints Eliminate

| Source of complexity | Avoided? | Why |
|---|---|---|
| Variable shadowing | ✅ | The language forbids it — one name always points to the same thing |
| Cross-iteration borrow in for | ✅ | Each iteration is a new binding — iterations are naturally isolated |
| `'a` lifetime annotations | ✅ | Brand paths = `#42.field_x`, compiler-inferred |
| Named lifetimes + constraint propagation | ✅ | Brand path prefix comparison replaces explicit constraint sets |
| Borrow graph constraint solving (Polonius) | ✅ | Brand tree prefix match + DAG consumer query |
| Loop-body borrow liveness propagation | ❌ | Same as Rust — handled via reverse BFS + logical cutoff |
| Conditional branch conservativeness | ❌ | Same as Rust — SMT covers what is provable, rest is conservatively rejected |

### Why DAG Works

Three language design constraints make DAG analysis feasible in YaoXiang:

- **No variable shadowing** — one name always points to the same thing; no need to track across re-bindings
- **for binds fresh each iteration** — iterations are naturally isolated; no cross-iteration borrows
- **Structured concurrency** — task boundaries are clear; no cross-task liveness propagation

These constraints eliminate the main sources of complexity in Rust's CFG fixed-point iteration. DAG is not "more advanced" than CFG — it is that a simpler language design permits a simpler analysis.

---

## Detailed Design

### System Predicate Catalog

The compiler automatically generates the following propositions and feeds them into the RFC-027 proof pipeline:

| System predicate | Trigger | Proposition form |
|---|---|---|
| `borrow_conflict` | WriteToken(v) needed | `forall t ∈ conflicting(v): dead_at(t, node)` |
| `use_after_move` | Using variable v | `¬moved(v)` |
| `use_after_drop` | Using variable v | `¬dropped(v)` |
| `double_drop` | Drop(v) | `¬dropped(v)` |
| `mut_violation` | Writing immutable v | `is_mut(v)` |

The existing `BorrowChecker`, `MoveChecker`, `DropChecker`, `MutChecker` **become proposition generators** — they don't disappear, they change role. They generate propositions; the pipeline verifies them.

### Brand Tree

RFC-009 §2.7's brand mechanism formalized as a brand tree.

**Token semantics — freeze-first, not duplicate-first**:

The essential difference between `&T` and `&mut T` is not "whether it can be copied," it is "whether simultaneous writes are allowed":

```
ReadToken(T):  Grants read-only permission, simultaneously freezing the source data T —
              no WriteToken(T) is acquirable during this period. Freezing is the
              primary semantics of ReadToken. Dup (copyable) is a corollary of freezing:
              because the data is already frozen (no mutation possible), multiple
              read-only views are inherently safe.

WriteToken(T): Grants exclusive read-write permission. Because writes exist, no other
              token (read or write) can coexist. Not implementing Dup (linear type)
              is a corollary of exclusivity.
```

**Causal relationship**:
```
ReadToken exists → source data frozen → multiple read-only views safe → Dup
                          ↓
                WriteToken rejected (enforced by borrow_conflict system predicate)
```

Not:
```
ReadToken has Dup → multiple allowed → check conflict on the side  ← inverted causality
```

```
BrandTree:
  nodes: Map<BrandId, BrandNode>

BrandNode:
  id: BrandId               # "#42", "#42.field_x"
  kind: ReadToken | WriteToken
  source_var: Operand
  parent: Option<BrandId>   # parent in the derivation chain
  children: Set<BrandId>    # derived child tokens
  consumers: Set<NodeId>    # DAG nodes that consume this token
  ref_count: usize          # safe copy count while ReadToken is frozen
```

**Conflict judgment — execution mechanism guaranteed by freezing**:

```rust
fn conflicts(a: &BrandId, b: &BrandId) -> bool {
    // Conflict condition: same source + at least one is a write + brand paths overlap
    // This means:
    //   1. ReadToken vs ReadToken → no conflict (both read-only, no mutation)
    //   2. WriteToken vs ReadToken → conflict (write breaks the read's freeze guarantee)
    //   3. WriteToken vs WriteToken → conflict (two writes cannot coexist)
    a.source() == b.source()
        && (a.is_write() || b.is_write())
        && (a.is_prefix_of(b) || b.is_prefix_of(a))
}
```

O(depth) string prefix comparison, depth ≤ 3. Constant.

### Reverse BFS Liveness Analysis

```
Algorithm: check_borrow(token, node, dag, brand_tree)

Input:
  token: WriteToken to check
  node:  DAG node of the write operation

Output: Proved | Disproved

Algorithm:
  # Fast path: reverse BFS
  unsafe = empty_set
  queue = brand_tree.consumers(token)

  while queue not empty:
    cur = queue.pop()
    unsafe.add(cur)

    for each pred in dag.predecessors(cur):
      # Structural cutoff: do not cross break
      if pred is a break edge:
        continue

      # Backedge → check whether SMT fallback is needed
      if pred is a backedge:
        path_cond = path condition at pred
        loop_cond = loop condition
        # First, check whether structure can cut (corresponding break already cut the path → won't reach here)
        # Then, check path condition
        if path_cond is non-empty:
          result = smt_fallback(path_cond, loop_cond)   # ← slow path
          if result == Proved:
            continue                    # logical cutoff
        # No path condition or SMT couldn't prove → traverse backedge
        # fall through

      if pred ∉ unsafe:
        queue.push(pred)

  # Decision
  if node ∈ unsafe:
    return Disproved
  else:
    return Proved


smt_fallback(path_cond, loop_cond):
  # Only called for backedges with a path condition
  # Uses RFC-027 proof pipeline, sharing the same SMT solver and the same budget
  return smt.prove(path_cond ⇒ !loop_cond)
  # Proved → logical cutoff
  # Disproved / Unproven → no cutoff, traverse backedge
```

O(N), where the number of SMT calls = backedge count × proportion of backedges with a path condition. In real code, SMT calls are extremely rare — only triggered for `while` loop bodies with refined-type path-condition variables.

### Path Condition Collection

Provided by RFC-027 §3.2-3.3 existing mechanisms:

- **if guard**: `if y > 0` → true branch pushes `y > 0`
- **match pattern**: `if let Some(v) = opt` → branch pushes `opt == Some(v)`
- **Assignment**: `i += 1`, compiler maintains variable value-range info
- **while cond**: inside loop body, pushes `cond == true`

Each DAG node carries a set of path conditions. When the reverse BFS hits a backedge, it takes the path condition at the backedge's start and asks SMT whether it rules out the next loop-entry condition.

### Interface with RFC-027

Borrow system predicates and user predicates share the same proof pipeline — the difference is the **main proof strategy**:

| Query type | Proposition source | Main strategy | Fallback |
|---|---|---|---|
| Type equality | Type checker | Structural equivalence | — |
| User predicate | Programmer type annotation | SMT | Programmer proof function |
| Borrow conflict | Compiler auto-generated | **DAG structural analysis (fast path)** | SMT logical cutoff |

The SMT solver's role in borrow checking: **not main force, but safety net.** Only invoked when a while backedge needs a logical cutoff. The vast majority of borrow checks complete on the fast path — O(N) reverse BFS, zero SMT overhead.

### Relation to Existing Code

| Existing component | Treatment |
|----------|----------|
| `BorrowChecker` | Becomes `BorrowPredicateEmitter` — generates borrow Hoare propositions |
| `MoveChecker` | Becomes `MovePredicateEmitter` — generates `¬moved(v)` propositions |
| `DropChecker` | Same — generates Drop-related propositions |
| `MutChecker` | Same — generates `is_mut(v)` propositions |
| `ControlFlowAnalyzer` | No longer needed — handled uniformly by the pipeline |
| `liveness_analysis` | Retained — Drop insertion still needs variable liveness info |
| `ir_gen.rs` Release hardcoding | Removed — Release position driven by DAG consumer analysis |

### NLL and Iteration Boundaries

**Token death time = last use point (NLL), not the end of lexical scope.**

This is a natural corollary of consumer analysis: a consumer's position defines the last use of a token. `use(v)` is a consumer of `v` → `v` dies immediately after `use(v)`. No extra `{}` or `drop()` needed to end the token's life early.

**Loop iteration boundaries are the death line of token copies.** Three rules:

```
Rule 1: Variables declared inside a loop die automatically at the end of each iteration.
        for binds fresh per iteration (guaranteed by language design); loop is the same.

Rule 2: The brand tree's ref_count at the loop header counts only copies created outside the loop.
        Copies produced by Dup inside the loop have their ref_count cleared at iteration boundaries.

Rule 3: When the reverse BFS crosses a backedge, it does not carry current-iteration liveness.
        Only the ref_count at the loop header (i.e., copies outside the loop) is carried.
```

Example:

```yaoxiang
view = &data                          # loop header: ref_count = 1, consumer = use(view)
loop {
    v2: &Point = view                 # Dup inside loop → ref_count = 2
    use(v2)                           # consumer: last use of v2 → v2 dies → ref_count = 1
    data.push(4)                      # ✅ safe! v2 is dead, only view remains (ref_count = 1, no write conflict)
    # iteration boundary: Rule 3 — do not carry v2 into the next round. Next iteration starts with v2 rebound.
}
```

This design needs no extra "conservative-alive in loops" rule. The reverse BFS starts from consumers; consumers are inside the loop body → liveness is bounded to the current iteration → backedges are not traversed. This aligns exactly with the loop examples in RFC-009a §Use Case Analysis.

### `?` Error Propagation and Scope-Driven Release

`?` is an early return — beyond the scope's normal exit, there is an additional exit path. Tokens must be released on this path; wrong release order is UB.

**Release instructions are generated by scope analysis, not hardcoded after Call.**

The compiler maintains a list of exit points for each scope:
- `}` (scope end)
- `?` (error propagation, early return)
- explicit `return`

At each exit point, Release instructions for all live tokens in the scope are inserted in reverse-declaration order (LIFO). The brand tree's parent-child relationship automatically handles cascading release of derived tokens:

```yaoxiang
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)    # returns child token &Float + parent token &Point
}

fn use_case(p: Point) -> Result<(), Error> = {
    (x_ref, p_ref) = p.get_x()?   # if ? propagates:
    # brand tree knows x_ref is derived from p_ref (#42.field_x is a prefix of #42)
    # release order: x_ref (child) → p_ref (parent) → LIFO automatically satisfied
    p.modify()                     # WriteToken — all ReadTokens released
    Ok(())
}
```

Implementation location: retained in `ir_gen.rs`, switched to scope-driven — no new compiler pass introduced.

| Operation | Complexity | Trigger frequency |
|------|--------|---------|
| Brand tree conflict check | O(1) | Each token request |
| DAG consumer query | O(1) | Each token request |
| Reverse BFS (fast path) | O(N) | Each token request, N = node count in block |
| SMT logical cutoff (fallback) | ~1ms | **Extremely rare** — only while + path conditions |

**SMT fallback's trigger conditions are extremely strict**: simultaneously (1) while loop (2) write inside the loop body (3) a path condition after the write can decide loop termination (4) the compiler needs to rely on that condition to cut the backedge. In real code, the proportion is far below 1%. The remainder of borrow checking completes on the fast path.

Relationship with RFC-027 user predicates: user predicates rely on SMT as the main force; borrow system predicates rely on structural analysis as the main force. They share the same SMT solver and budget cap (RFC-027 §8), but borrow system predicates barely consume the SMT budget.

Linear code → no backedges → tier 1 O(N) instant. Loop + path condition → SMT call, linear arithmetic millisecond-scale (RFC-027 budget 100ms). One BFS result can be cached for reuse by multiple queries on the same token.

### Error Message Design

**Core principle: error messages only show symbols the user has written.**

Rust's borrow-related errors fall into two categories:

**Variable-level errors**: E0597 (doesn't live long enough), E0502 (mutable + immutable borrow simultaneously), E0499 (multiple mutable borrows). Rust is already the gold standard — variable name + line number, no `'a` shown. YaoXiang matches the precision. All information is in the brand tree: token creation point, consumer location, request point.

**Signature-level errors**: E0623 (lifetime mismatch), E0106 (missing lifetime specifier), E0477 (required lifetime not satisfied). Centered on `'a`. YaoXiang **has no such errors** — there is no `'a` in signatures. Not "unable to report," but the user didn't write it, so we don't report it.

Function-internal conflict example:

```
error: `data` is frozen; cannot acquire mutable permission
 --> src/main.yx:5:9
2 |     view = &data
  |            ----- `data` is frozen (read-only token created here)
4 |         use(view)
  |             ---- `view` is still in use here; freeze not released
5 |         data.push(4)
  |         ^^^^ mutable permission required here
```

(Matches Rust E0499 precision — variable name + line number, no brand ID shown.)

Cross-function escape example:

```
error: `num` (line 4) holds data whose source is in part `default_str` (line 3),
but `default_str` dies at line 6, while `num` is still in use at line 5.

consider: move `default_str`'s declaration above the caller, or use `ref default_str` to share ownership.
```

(Matches Rust E0597 precision. The brand summary knows `num` has two source paths — already in the compiler; the error wording is usable.)

---

## RFC-009 Body Corrections

RFC-009 §"Token Conflict Detection: Flow-Sensitive Liveness Analysis" has been updated:

1. Remove "things not needed: ...NLL" — not because the conclusion is wrong, but because the reason is wrong ("tokens are values, linear tracking is enough")
2. The tier 1 / tier 2 transitional scheme is retained; the complete solution points to this RFC
3. Clarify: brand ID (`#42`) *is* `'a` — same information, different encoding. Not a new analysis — lifetimes lowered from the type layer to the proof layer

---

## Trade-offs

### Advantages

1. **Type signatures contain no lifetimes**: `#42` *is* `'42` — the same information encoded in the brand tree, not exposed in type signatures. This point is unfalsifiable: count how many `'a` parameters a generic type with 3 reference parameters needs in Rust, versus how many YaoXiang needs. The answer is 3 vs 0.

2. **Conceptual unification**: borrow checking and user predicates share the same proof pipeline — `{P} op {Q}`, the pipeline verifies P. Curry-Howard consistent.

3. **Zero new analysis framework**: no new analysis framework is introduced. Users are unaware of the existence of a "borrow checker" — just as users are unaware of the implementation details of a "type checker."

4. **Error messages contain only symbols the user wrote**: an entire dimension of error categories disappears (E0623, E0106, E0477 — all around `'a`). Variable-level error precision is on par with Rust.

5. **Algorithm is not conservative**: reverse BFS + break cutoff + SMT logical cutoff. No "conservative-alive inside loops." No "conservative branch merge."

### Disadvantages

1. **Not a new invention**: what brand IDs do is exactly what `'a` does — the constraint-solver complexity inside the compiler has not disappeared, only its encoding has changed from "variable name + constraint set" to "brand path + prefix matching." The difference for the end user is solely that `'a` does not appear in signatures.

2. **Brand new implementation**: the brand tree exists only as a concept in code; must be implemented from scratch. `BorrowChecker` and `ControlFlowAnalyzer` are replaced.

3. **SMT dependency**: logical cutoffs depend on Z3 (already introduced by RFC-027, no new dependency). But borrow checking almost never triggers it — only `while` + path conditions call it.

4. **A handful of patterns require refactoring**: cross-branch borrows that the compiler's automatic proof cannot cover require the user to refactor. Unlike Rust's `'a` fallback: Rust has `'a` as a pen (annotate and it passes); YaoXiang's fallback (proof function) is not MVP.

---

## Alternatives

| Alternative | Why not |
|------|-----------|
| Implement full Rust NLL | YaoXiang's design constraints (no shadowing, for rebinds) already eliminate NLL's main sources of complexity; no CFG fixed point needed |
| Stay with current (hardcoded Release) | Insufficient — users must manually manage token scopes |
| Analyze only spawn blocks | Insufficient — token usage in non-spawn code is the majority |
| GC instead of borrow checking | Violates language design principles — YaoXiang has no GC |

---

## Implementation Phases

| Phase | Content | Dependency |
|------|------|------|
| Phase 1 | Brand tree data structure implementation | — |
| Phase 2 | System predicate generators (Borrow/Move/Drop/Mut → propositions) | Phase 1 |
| Phase 3 | Reverse BFS liveness analysis + pipeline integration (tier 1) | Phase 2 |
| Phase 4 | Path condition collection + SMT logical cutoff (tier 2) | Phase 3 + RFC-027 Phase 2 |
| Phase 5 | Release instructions driven by DAG consumers | Phase 3 |
| Phase 6 | Remove `ControlFlowAnalyzer`, refactor `BorrowChecker` | Phase 4 |

---

## Open Questions

- [x] **`ref_count` cross-iteration semantics in brand tree during loop unrolling** — go NLL: token dies after last use. Copies bound inside a loop die at the iteration boundary; reverse BFS does not carry liveness across iterations. See §NLL and Iteration Boundaries.
- [x] **Token release order along `?` error propagation path** — Release driven by scope analysis (retained in `ir_gen.rs`). Each scope exit point (`}`, `?`, explicit return) releases live tokens in LIFO order. Brand tree parent-child relationship automatically handles cascading release. See §`?` Error Propagation and Scope-Driven Release.
- [ ] Proof function syntax (long-term, non-MVP — does not block any phase)

---

## References

- [RFC-009: Ownership Model Design](../accepted/009-ownership-model.md) — parent RFC
- [RFC-027: Compile-time Predicates and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md) — proof pipeline
- [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md) — `{}` semantics
- [RFC-024: spawn-block-based Concurrency Model](../accepted/024-concurrency-model.md) — spawn DAG

---

## Lifecycle and Final Status

| Status | Location | Notes |
|------|------|------|
| **Accepted** | `docs/design/rfc/accepted/` | Becomes an official design document |