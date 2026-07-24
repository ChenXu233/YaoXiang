---
title: "RFC-027: Compile-time Predicates and Unified Static Verification"
status: "Accepted"
author: "ChenXu"
created: "2026-06-07"
updated: "2026-07-05"
impl_status: "in_progress"
impl_detail: "Phase 1-2 completed, Phase 3 partially completed, Phase 4 partially completed. The unified assert/Assert scheme's 6 phases are fully implemented (issues #157-#162 closed): Never type, IsTrue bridging, flow-sensitive Γ + kill set, type-level recursion, universe layering weak check, dispatch pipeline."
impl_percent: 85
issue_number: 90
issue_url: "https://github.com/ChenXu233/YaoXiang/issues/90"

issue: "#90"
---

# RFC-027: Compile-time Predicates and Unified Static Verification

> **References**:
>
> - [RFC-009: Ownership Model](../accepted/009-ownership-model.md)
> - [RFC-010: Unified Type Syntax - name: type = value Model](../accepted/010-unified-type-syntax.md)
> - [RFC-011: Generics System Design](../accepted/011-generic-type-system.md)
> - [RFC-024: Concurrency Model Based on `spawn` Blocks](../accepted/024-concurrency-model.md)
>
> **Supersedes**: [RFC-022: Hoare Logic Static Verification Support (Specification Annotations and Specification Types)](../deprecated/022-hoare-logic-static-verification.md) — Deprecated

## Summary

This RFC proposes introducing **compile-time predicates** as first-class citizens in YaoXiang, unifying all compile-time static verification into a single **proof pipeline**. A compile-time predicate is not an external specification annotation—it *is* a function. A function that returns `Type` can be used in type position; the compiler invokes it at compile time and checks the return value. Types are propositions; compile-time evaluation is proof.

**Core thesis**: The only work type checking does at compile time is to construct and verify proof terms. Type equality, token conflicts, dependent type reduction, compile-time predicate evaluation, Hoare logic entailment—all are different type checks in the compile-time proof pipeline, sharing the same pipeline. The SMT solver is an acceleration module for the type checker, not an independent trust boundary. When the compiler returns `Unproven`, the programmer writes a YaoXiang function as proof—the type checker verifies it in exactly the same way it verifies any function's return type. Everything is YaoXiang code; everything is verified by the type checker.

## Motivation

### Why Deprecate RFC-022?

RFC-022 designed specifications in `//!` annotation form:

```yaoxiang
max: (T: Ord) -> ((arr: Array(T, n)) -> T) = {
    //! requires: NonEmpty(n) = n > 0          ← This is annotation external to types
    //! ensures: ExistsMax(result, arr[0..n])   ← This is annotation external to types
}
```

This commits the fundamental error of the Curry-Howard correspondence: **splitting specifications and types into two layers**. Annotations are not types. Annotations do not participate in type checking. Annotations are an "external tool" mental model.

The white paper makes this clear:

> "No `//!` annotations. No independent specification language. Everything is within the type system."

### Current Problems

- RFC-022's `//!` annotations are external syntax detached from the type system
- Specification types and ordinary types are two separate systems, creating conceptual redundancy
- The Debug Build verifies / Release Build ignores split model breaks uniformity
- SMT solvers are conventionally positioned as external tools—YaoXiang makes them built-in acceleration modules of the type checker
- Type checking, borrow checking, compile-time predicate checking, and macro expansion each take different paths

### The Correct Mental Model

Type checking can be abstracted as a function:

```
verify : Program → Proved | Disproved(Model) | Unproven
```

All compile-time checks—simple type matching, borrow conflict detection, compile-time predicate verification—are sub-tasks of this function. They share the same proof pipeline; the only differences lie in proof term complexity and construction strategy.

When the compiler returns `Unproven`, the programmer provides a proof function—the function's return type equals the proposition to be proved. The type checker verifies it. This is the same operation as ordinary type checking.

## Proposal

### 1. `{}` Is Proof Space: Types Are Assertions, Verification Is Type Checking

YaoXiang's `{}` is the compile-time proof space. Everything inside is an assertion; the compiler guarantees each item is `True`—either by automatic proof or by a programmer-provided proof function.

```
Point: Type = { x: Float, y: Float }
#               ^^^^^^^^^^^^^^^^^^^^^  Compiler guarantees x is Float, y is Float

List: (T: Type) -> Type = { data: Array(T) }
#                           ^^^^^^^^^^^^^^^  Compiler guarantees data is Array(T)
```

**Generics are a special case of compile-time predicates.**

```yaoxiang
Positive: (x: Int) -> Type = { x > 0 }
#          ^^^^^^              ^^^^^^
#          parameter at signature  only assertions inside {}
#          compiler verifies x > 0 at call time

List: (T: Type) -> Type = { data: Array(T) }
#      ^^^^^^^^              ^^^^^^^^^^^^^^^
#      parameter at signature   compiler verifies type_of(T) == Type, type_of(data) == Array(T)
```

The same pattern: `name: (params) -> Type = { assertions }`. The compiler does not distinguish "type assertions" from "value assertions"—both are evaluation targets in the proof pipeline.

**Loop invariants need not be written separately. The type annotation on a variable is the Floyd-Hoare invariant.**

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # Annotation references i—tells the compiler s's type depends on i
    mut i: UpTo(arr.len) = 0     # At initialization i=0, verify: 0 == sum(arr[0..0]) → True
    while i < arr.len {
        s += arr[i]  # Compiler verifies: s_new == sum(arr[0..i+1])
        i += 1       # i changes → triggers s's dependency re-verification: s satisfies SumUpTo(arr, i_new)
    }
    return s  # s: SumUpTo(arr, arr.len) = sum(arr[0..arr.len])
}
```

The compiler generates a verification condition for the loop body—induction hypothesis (type annotation) → assignment operation → whether the new value satisfies the type annotation. Once the proof pipeline verifies the induction step, all iterations are automatically covered. No `: decreases`, no `: Invariant`, no inductive proof needed—the compiler decomposes induction into a local VC for each assignment.

### 2. Pre/Postconditions: Compile-time Predicates on Parameter Types and Return Types

Abandon RFC-022's `//! requires`/`//! ensures`. Compile-time predicates serve as annotations on parameters or return types.

**On the parameter side, it's a function call.** A compile-time predicate is a function returning `Type`; using it on the parameter side is calling it—just like `factorial(5)`. The return side introduces a new concept: the return parameter.

```yaoxiang
# Precondition: explicit compile-time predicate call in parameter type
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b
#                       ^^^^^^^^^^  b is the current parameter name, passed to Positive as argument
#                       compiler extracts argument value at call site, substitutes b, verifies Positive(arg)
#                       ex: divide(10, 2) → verify Positive(2) = { 2 > 0 } → True
#                       ex: divide(10, 0) → verify Positive(0) = { 0 > 0 } → False → compile error

# Postcondition: return parameter + compile-time predicate
IsMax: (T: Ord, arr: Array(T), result: T) -> Type = {
    forall j in 0..arr.len: result >= arr[j]
}

NonEmpty: (arr: Array(T)) -> Type = { arr.len > 0 }

max: (T: Ord) -> ((arr: NonEmpty(arr))) -> (result: IsMax(T, arr, result)) = {
#                                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
#                                            result is the return parameter, value provided by return
#                                            compiler substitutes return value at return point, verifies postcondition
    candidate = arr[0]
    for i in 1..arr.len {
        if arr[i] > candidate { candidate = arr[i] }
    }
    return candidate
}
```

**Key rules**:

- **Parameter side**: `b: Positive(b)`—`b` is the current parameter name, passed to `Positive` as an argument. Function call syntax, zero implicit behavior.
- **Return side**: `-> (result: IsMax(T, arr, result))`—`result` is the return parameter, value provided by the `return` statement. `result` exists only in the type signature, is referenced only by predicates, does not enter the function body scope, and does not appear at the call site.
- **Return parameter is optional**: when there is no postcondition, omit it; the signature is identical to an ordinary function (`-> Int`).
- **Uniformity**: parameter and return parameters are the same concept—`paramName: predicateCall(paramName)`; the only difference is whether the value is provided by the caller or by `return`.

### 3. Path Condition Propagation: Compile-time Verification of Runtime Values

When a compile-time predicate is used at a binding position, arguments are explicitly passed by the programmer. When runtime values flow into refinement type parameters, the compiler completes verification via path condition collection and SMT entailment—no explicit proof passing required.

#### 3.1 Explicit Function Call

When a compile-time predicate is used at a binding position, arguments are explicitly passed by the programmer—it is a function call, zero implicit behavior.

`Positive: (x: Int) -> Type = { x > 0 }` is a compile-time predicate constructor. When it appears at a binding position (parameter declaration, variable declaration, return type), the programmer explicitly passes already-bound variable names:

```yaoxiang
b: Positive(b)
// b is already declared as the current parameter, Positive(b) is a function call
// After normalization: b: { b > 0 }
```

No implicit argument filling by the compiler—`b: Positive(b)` is just a function call, like `f(5)`. `b` is bound as a parameter name, and its type annotation `Positive(b)` references `b` itself—this is the standard pattern of dependent types, not an implicit expansion rule.

**Unification with RFC-010 `self`**: RFC-010 establishes that `self` is not a keyword but merely a conventional parameter name ("writing it as `p`, `this`, or `x` has exactly the same effect"). `b: Positive(b)` shares the same mechanism—the parameter name can be referenced in the type annotation. `self` appears in the position `self: Point`; `b` appears in the position `b: Positive(b)`; both type annotations reference the parameter itself. The only difference is annotation complexity; the mechanism is exactly the same—after name binding, the type can depend on that name.

The return type likewise uses explicit function calls:

```yaoxiang
Sorted: (arr: Array(T)) -> Type = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }

sort: (arr: Array(T)) -> (result: Sorted(result)) = { ... }
//                        ^^^^^^^^^^^^^^^^^^^^^^^
//                        result is the return parameter, Sorted(result) is a function call
//                        compiler substitutes return value into result at return point, verifies Sorted(return_value)
```

The same applies to local variable declarations:

```yaoxiang
let x: Positive(x) = 5
// x bound to 5, Positive(5) → { 5 > 0 } → True → pass

// let y: Positive(y) = 0
// y bound to 0, Positive(0) → { 0 > 0 } → False → compile error
```

#### 3.2 Path Condition Collection

When runtime values appear in conditional branches, the compiler automatically collects path conditions, forming the **assumption set** for the current scope. These assumptions participate in verification as background knowledge for compile-time `Bool` evaluation.

```yaoxiang
if y > 0 {
    // compiler automatically has assumption: { y > 0 } in this branch
    let result = divide(x, y)
    // verification condition: (y > 0) ⇒ (y > 0)
    // proof pipeline judges entailment holds → Proved
} else {
    // this branch assumption: { !(y > 0) }
    // calling divide(x, y) here: verification condition is !(y > 0) ⇒ y > 0
    // proof pipeline judges not entailed → Disproved
}
```

This is not the compiler hard-coding a special pattern—this is the natural behavior of the compile-time proof pipeline. At each type-check call site, the pipeline sends:

```
{background assumptions} ⇒ {verification goal}
```

The proof pipeline judges entailment. `Proved` → pass; `Disproved` → compile error + counterexample; `Unproven` → compile error + unproven proposition. Background assumptions come from the path conditions at the current program point.

#### 3.3 Assumption Stack

During control flow analysis, the compiler maintains an assumption set for each basic block:

- **if-guard**: `if y > 0` → true branch pushes `y > 0`, false branch pushes `!(y > 0)` (if `else` is used)
- **match pattern**: `if let Some(v) = opt` → branch pushes `opt == Some(v)`
- **logical conjunction**: `if x > 0 && y < 10` → branch pushes `x > 0` and `y < 10`
- **function precondition**: when calling `divide(a, b)`, evidence that `b` satisfies `Positive` either comes from current assumptions or from the argument's own refinement type annotation (if `b` is annotated as `Positive`, its type carries `b > 0`)
- **assignment**: `let z = y` propagates the existing refinement condition on `y` to `z`

All assumptions enter the compile-time proof pipeline. When entering the SMT acceleration path, they are translated to SMT-LIB background assertions.

#### 3.4 No Static Evidence → Compile Error

If the programmer directly writes:

```yaoxiang
divide_user_input: (x: Int, y: Int) -> Int = divide(x, y)
```

The current program point has no assumption `y > 0`, and the argument `y` itself has no `Positive` type annotation. The verification condition is:

```
{} ⇒ { y > 0 }
```

Pipeline returns `Disproved` (not entailed) → compile error:

> Cannot prove that parameter `b` satisfies `Positive` in the call to `divide`.
> `y` comes from function input, with no proven bound.
> Consider guarding the call with an if branch: `if y > 0 { divide(x, y) }`.

YaoXiang does not accept runtime values flowing directly into refinement type parameters without static evidence. This is not a limitation—it is the core of the hard-safety philosophy. Any code the compiler cannot statically prove must not pass compilation.

#### 3.5 Relationship with the Unified Pipeline

Path condition propagation is not an additional mechanism. It is the direct extension of the compile-time proof pipeline into control flow analysis:

| Phase | Responsibility |
|-------|----------------|
| Path condition collection | Compiler control flow analysis phase, annotates each basic block with assumption set |
| Verification condition generation | When a type constraint requiring verification is encountered, merge path conditions + argument type information |
| Proof pipeline evaluation | Compiler kernel → SMT acceleration → yields Proved / Disproved / Unproven |
| Result | `Proved` → pass; `Disproved` → compile error + counterexample; `Unproven` → compile error + unproven proposition (programmer can provide proof function) |

No new components. No special rules. Path conditions are the background knowledge of the proof pipeline—sharing the same pipeline and budget system as type equality and borrow constraints.

### 4. The Compile-time Proof Pipeline

All compile-time checks share the same pipeline. The pipeline's core operation is **type checking**—checking whether a proof term's type equals the proposition to be proved. Everything is type checking.

```
At compile time, a Bool expression needs evaluation (i.e., a proof term must be constructed)
        │
        ├── Type equality (T1 == T2)
        │   → compiler direct judgment (structural equivalence)
        │
        ├── Token conflict condition (!conflicting(tokens))
        │   → flow-sensitive liveness analysis (Dup/Linear property tracking)
        │
        ├── Dependent type reduction (n + m simplification)
        │   → compile-time term rewriting system (βδι-reduction)
        │
        ├── Compile-time predicate (x > 0, forall...)
        │   → compiler itself + SMT acceleration module
        │
        └── Hoare logic entailment (P ⇒ Q)
            → compiler + SMT acceleration module
                    │
                    ▼
             ┌──────────┐
             │ Proved   │  → compilation passes
             │ Disproved│  → compile error + counterexample
             │ Unproven │  → compile error + unproven proposition
             └────┬─────┘
                  │
                  ▼
         Programmer writes proof function (YaoXiang code)
                  │
                  ▼
         Type checker verifies ──→ Proved ──→ compilation passes
                  │
                  ▼
            Verification fails → compile error: "proof does not hold"
```

#### 4.1 Proof Results: A Three-valued Algebra

Compile-time evaluation returns three outcomes—an inevitable consequence of the halting problem and a natural partition of proof theory:

```
eval_compile_time : BoolExpr → Proved | Disproved(Model) | Unproven
```

- **Proved** → halted, proof term constructed, type check passed. Compilation continues.
- **Disproved(M)** → halted, counterexample M exists. Compile error + counterexample + source location.
- **Unproven** → within the given resource bound, no proof has been constructed. Compile error + unproven proposition + budget consumption report.

**Unproven ≠ False.** The compiler saying "I cannot prove this" is not equivalent to the proposition being false—it simply exceeds the current automatic proving capability. This is honesty, not a defect.

A hard budget limit is the engineering answer to the halting problem. No knobs are given—offering one would be asking the user "do you think your program will halt," and neither the user nor the compiler knows the answer.

#### 4.2 After Unproven: The Programmer Writes Proof

When the compiler returns `Unproven`, the programmer can write a **proof function**—a YaoXiang function whose return type equals the proposition to be proved. The type checker verifies this function—using exactly the same mechanism it uses to verify `add(a, b): Int`.

```
Proposition = Type
Proof       = Program (a value of that type)
Verification= Type check (the sole root of trust)
```

The SMT solver is not an independent trust boundary—it is an **acceleration module of the type checker**. SMT helps find proofs, but it is always the type checker that verifies them. When SMT returns `unsat`, the compiler reconstructs that result as a proof term verifiable by the type checker. If reconstruction fails (SMT's inference steps exceed the compiler kernel's inference rules), it falls back to `Unproven`—the programmer can manually write a proof function.

```yaoxiang
# Proposition: refinement property the compiler cannot automatically prove
FirstIsMin: (T: Ord, arr: Sorted(T)) -> Type = {
    forall i in 0..arr.len: arr[0] <= arr[i]
}

# Proof: programmer writes a function whose return type is the above proposition
# Type checker verifies the function—exactly the same as verifying add(a,b): Int
first_is_min: (T: Ord, arr: Sorted(T)) -> FirstIsMin(T, arr) = {
    # compiler here verifies: function body's type = FirstIsMin(T, arr)
    ...
}
```

No AI needed, no export to Coq, no new concepts. **Properties the compiler cannot auto-prove → programmer writes proof in YaoXiang code → type checker verifies.** The whole process is a smooth gradient—the compiler does the easy proofs and leaves the hard ones to the programmer's brain.

#### 4.3 Layered Dependencies Within the Pipeline

The above evaluators share the same interface but have an evaluation order. Type equality is a prerequisite for all subsequent analyses; ownership/token checks depend on type information; refinement predicate verification depends on the results of the first two layers. The compiler evaluates layer by layer; expressions failing at lower layers do not enter upper layers—avoiding wasted solver budget on type-incorrect programs.

```
Evaluation order (same pipeline, layered scheduling)
├── Layer 0: type equality (T1 == T2)
│   └── structural unification → failure makes subsequent steps meaningless, directly return Disproved
├── Layer 1: ownership/token conflict
│   └── flow-sensitive liveness analysis → failure means memory safety does not hold, directly return Disproved
└── Layer 2: refinement predicate / Hoare entailment
    └── compiler itself → SMT acceleration → yields Proved / Disproved / Unproven
```

Each layer still returns `Proved/Disproved/Unproven`, sharing the same interface and the same budget system.

### 5. Unification of Three Function Layers

| Layer | Execution Time | Input | Output | Example |
|-------|----------------|-------|--------|---------|
| Value-level function | Runtime | Value | Value | `add: (a: Int, b: Int) -> Int = a + b` |
| Type constructor | Compile time | Type/Value | Type | `List: (T: Type) -> Type = { data: Array(T) }` |
| Compile-time predicate | Compile time | Value | Type | `Positive: (x: Int) -> Type = { x > 0 }` |

All use the same `name: type = value` syntax. Compile-time predicates and type constructors go through the same compile-time proof pipeline—`{}` is proof space.

### 6. Loops: Floyd-Hoare Verification Condition Generation

Loops need no separate `: Invariant(...)` or `: decreases(...)` annotations. The compile-time predicate type annotation on a variable defines a Floyd-Hoare-style assertion—the compiler generates verification conditions from the type annotation, and the proof pipeline checks whether each assignment maintains the type.

Core mechanism: each assignment operation corresponds to a Hoare triple `{P} x := e {Q}`; the verification condition is `P ⇒ Q[e/x]`. The compiler generates a single verification condition for the loop body—once the proof pipeline verifies the induction step, all iterations are automatically covered.

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # Annotation references i; at init i=0, verify: 0 == sum(arr[0..0]) → True
    mut i: UpTo(arr.len) = 0     # verify: 0 <= 0 <= arr.len → True
    while i < arr.len {
        # Compiler generates one VC for the loop body. Premise: s satisfies SumUpTo(arr, i), i satisfies UpTo(arr.len).
        #
        # s += arr[i]:
        #   Verification obligation: s_new satisfies SumUpTo(arr, i) (current i unchanged)
        #   Substituting s_new = s_old + arr[i]:
        #     Need s_old + arr[i] == sum(arr[0..i+1])
        #     From induction hypothesis s_old == sum(arr[0..i]), add arr[i] to both sides:
        #     sum(arr[0..i]) + arr[i] == sum(arr[0..i+1])
        #   Compiler + SMT: linear arithmetic, millisecond scale → Proved
        #
        # i += 1:
        #   i changes → s's type annotation references i in dependency graph → triggers re-verification
        #   New verification target: s satisfies SumUpTo(arr, i_new)
        #   i.e., s == sum(arr[0..i_new]), guaranteed by previous step → Proved
        s += arr[i]
        i += 1
    }
    return s  # At this point s: SumUpTo(arr, arr.len), i.e., s == sum(arr[0..arr.len])
}
```

Loop invariants are the type annotations on variables—the programmer writes types, the compiler checks the induction step. The compiler need not "discover" invariants or "perform induction automatically"—it decomposes the inductive proof into local verification conditions for each assignment operation, delegating divide-and-conquer to the proof pipeline.

#### 6.1 Dependency Tracking: Dependent Types on Mutable Variables

The above mechanism presupposes that the compiler knows `s`'s type annotation `SumUpTo(arr, i)` references `i`—when `i` changes, the type constraint on `s` changes accordingly. This requires the compiler to maintain a **type dependency graph between variables**.

**Data structure**:

```
TypeDepGraph: Map<VarName, Set<VarName>>
# key is the depended-on variable, value is the set of variables whose type annotations reference it
# ex: { i: {s}, j: {s, t}, ... }
```

**Construction**: When the type checker processes `mut v: Pred(... x ...) = init`, it resolves free variable references in `Pred(...)`'s arguments. If a referenced mutable variable `x` from the current scope is found in the arguments, `x → v` is recorded in the dependency graph.

**Trigger**: When depended-on variable `x` is assigned, the compiler:
1. Looks up all variables in the dependency graph depending on `x`: `{v₁, v₂, ...}`
2. For each `v`, generates a verification condition: does `v`'s current value satisfy the updated type `Pred(... x_new ...)`?
3. Sends the VC to the proof pipeline

**Assignment-order sensitive**: Dependency tracking naturally enforces correct assignment order. Taking `SumUpTo(arr, i)` as an example:

```yaoxiang
# Correct order
s += arr[i]   # s_new satisfies SumUpTo(arr, i+1)
i += 1        # i changes → re-verify s satisfies SumUpTo(arr, i_new) → True

# Wrong order—compiler rejects
i += 1        # i changes → re-verify s satisfies SumUpTo(arr, i_new)
              # s not yet updated, s_old == sum(arr[0..i_old]) ≠ sum(arr[0..i_new])
              # → compile error: variable s does not satisfy type SumUpTo(arr, i_new)
s += arr[i]   # unreachable
```

**Composite dependencies**: a variable can depend on multiple variables. The type annotation `{ v: Int; v == x + y }` depends on both `x` and `y`—either changing triggers re-verification.

**Relationship with the proof pipeline**: Dependency tracking is a trigger for VC generation, not an independent verification mechanism. It answers "when does a VC need to be generated"—the proof pipeline answers "does the VC hold".

### 7. Termination Checking

Fully automatic at compile time. Loops the compiler can prove pass; those it cannot are reported directly as compile errors—the programmer must make the compiler able to automatically analyze loop termination. No half-automatic annotation escape hatch.

#### 7.1 Design Principles

The compiler automatically extracts information needed for termination proofs from two places:

1. **Variable type annotations**: boundary constraints in refinement types (e.g., `UpTo(n)` provides upper bound `n` and lower bound `0`)
2. **Loop body operations**: operations applied to variables on each iteration

The compiler attempts four measure-synthesis strategies in priority order, stopping on the first success.

#### 7.2 Strategy 1: Automatic Linear Rank Function Synthesis

When variables have linear bound annotations, the compiler enumerates candidate linear measures and verifies via SMT.

```
Input:
  Variables v₁: UpTo(u₁), v₂: UpTo(u₂), ... (variables with bounds)
  Loop condition cond
  Set of assignments in loop body

Algorithm:
  1. Extract each variable's bounds from type annotations: [low_i, high_i]
  2. Enumerate candidate measures: v_i, u_i - v_i, v_i - v_j, etc. linear combinations
  3. For each candidate measure m:
     - SMT verify m ≥ 0 (derived from type bounds)
     - For each execution path in the loop body, SMT verify m' < m (strictly decreasing)
  4. Find a qualifying linear combination → termination proved
```

Coverage: loops where any variable is assigned a linear expression (`v = a·v + b`) with bounded type annotations. Including `i += const`, `i -= const`, and binary-search-style interval contraction:

```yaoxiang
# Binary search: low = mid + 1 or high = mid
# Measure high - low strictly decreases on both paths
binary_search: (arr: Sorted(Int, arr), key: Int) -> Option(Int) = {
    mut low: UpTo(arr.len) = 0
    mut high: UpTo(arr.len) = arr.len
    while low < high {
        let mid = (low + high) / 2
        if arr.data[mid] < key { low = mid + 1 }
        else if arr.data[mid] > key { high = mid }
        else { return Some(mid) }
    }
    return None
}
```

#### 7.3 Strategy 2: Predicate Violation Counting—Automatically Extracting Measure from Target Type <span style="color:orange">【Experimental Strategy】</span>

> ⚠️ **Current status: experimental strategy; whether to include it depends on practical feasibility during Phase 3 implementation.**
> This strategy works for adjacent-swap operations (bubble sort, insertion sort); it cannot auto-prove non-adjacent operations (quicksort partition, heap sort sift-down).
> Coverage boundaries are listed in the table below. If Phase 3 verification proves infeasible, this strategy will be removed or downgraded to future work.

Core insight: **the specifications users write are the raw material for compiler reasoning.** The compiler does not need a built-in "what is sorting"—it reads the `Sorted` definition and automatically extracts a measure from it.

```
Input:
  Target type: Sorted(arr) = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }
  Loop body operation: adjacent element swap

Algorithm:
  1. Parse predicate definition: forall i in range: cond(i, arr)
  2. Auto-generate measure: violation_count = |{ i | ¬cond(i, arr) }|
  3. Analyze the operation's effect on the measure:
     - Adjacent swap arr[j], arr[j+1] = arr[j+1], arr[j]
     - Affects only index pairs j-1, j, j+1
     - If arr[j] > arr[j+1] (predicate violation), the swap satisfies the predicate for that pair
     - violation_count decreases by at least 1
  4. Upper bound: n·(n-1)/2 (maximum adjacent inversions), lower bound: 0
  → termination proved
```

**Current coverage**:

| Algorithm | Operation Pattern | Strategy 2 Provable? | Reason |
|-----------|------------------|:---:|--------|
| Bubble sort | Adjacent swap | ✅ | violation_count strictly decreases on each swap |
| Insertion sort | Adjacent shift | ✅ | Each shift eliminates one violating pair |
| Selection sort | Non-adjacent swap | ❌ | A single swap may increase violation_count |
| Quicksort | partition | ❌ | Non-adjacent swap, not guaranteed to monotonically decrease |
| Heapsort | sift-down | ❌ | Tree-shaped operation, violation_count not monotonic |

**Complementary strategies**: For quicksort, the `low < high` interval contraction is covered by Strategy 1 (linear rank function)—the outer partition recursion halves the interval each time. Strategy 1 and Strategy 2 complement each other; termination of most practical algorithms can be proved by one of them. However, generalizing Strategy 2 (non-adjacent operations, tree-shaped operations) remains an open problem.

```yaoxiang
sort: (arr: Array(Int)) -> (result: Sorted(result)) = {
    mut i: UpTo(arr.len) = 0
    while i < arr.len - 1 {
        mut j: UpTo(arr.len - i - 1) = 0
        while j < arr.len - i - 1 {
            if arr.data[j] > arr.data[j+1] {
                arr.data[j], arr.data[j+1] = arr.data[j+1], arr.data[j]
            }
            j += 1
        }
        i += 1
    }
    return arr
}
```

#### 7.4 Strategy 3: Bounded Increment/Decrement Pattern

`v += const` (positive constant), variable has upper-bound type annotation → measure `upper_bound - v` decreases by `const` each iteration, lower bound 0. This is a degenerate case of Strategy 1, handled quickly by the compiler at the front.

#### 7.5 Strategy 4: Multiplicative Scaling Measure Template

`v *= const` (const > 1), variable has upper- and lower-bound type annotations. The compiler has a built-in logarithmic measure template `ceil(log_const(upper/v))`; measure decreases by 1 each multiplication by `const`.

```yaoxiang
mut i: Positive(i) = 1
while i < n {
    # Compiler auto-derives: measure ceil(log₂(n/i)), decreases by 1 each multiplication by 2
    i *= 2
}
```

#### 7.6 Separating Termination from Correctness

Termination proof and correctness proof are independent:
- **Termination**: the above four strategies automatically prove the loop exits in finite steps
- **Correctness**: whether the loop body progresses toward the target type, checked by the compile-time proof pipeline via verification conditions

Both pass → compilation passes. Termination proved but correctness fails → compile error + counterexample. Correctness proved but termination cannot be proved → compile error pointing out the variable or operation that cannot be analyzed. Both fail → compile error reporting both failure reasons.

#### 7.7 Termination Checking for Recursive Functions

For recursive functions that need to be evaluated at compile time, the compiler checks argument decrease:

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)  # compiler analysis: n-1 < n → decreasing → terminates
}

# Compile-time use—compiler guarantees factorial terminates at compile time
vec: Vec(factorial(5)) = Vec(120)()  # 5! = 120, completed at compile time
```

| Scenario | Behavior |
|----------|----------|
| Compiler can analyze recursive decrease (e.g., `n-1`) | Compile-time evaluation |
| Not decreasing / cannot determine decrease | Compile error |
| Runtime call (not in type position) | No termination check needed |

#### 7.8 Hard Boundary

`i = f(i)` where `f` is non-invertible, non-closed, and preserves no monotonicity—mathematically impossible to automatically prove termination. Compile error:

> This loop cannot be automatically proven to terminate. Loop variable depends on the non-analyzable function `f`. Please use an iteration pattern that the compiler can analyze.

This is not a compiler failure. Any code that cannot be statically proven safe must not pass compilation.

### 8. SMT Solver: An Acceleration Module of the Type Checker

In conventional languages the SMT solver is an external tool (e.g., F\* invokes Z3, Dafny invokes Z3). In YaoXiang, it is an **acceleration module of the type checker**—invoked only when the compiler kernel itself cannot directly judge. SMT helps find proofs, but it is the type checker that verifies them.

**Trust model**: the type checker is the sole root of trust. The SMT solver is an acceleration module—it helps find proofs, but SMT is not an independent trust boundary. The compiler trusts Z3's `unsat` results (consistent with the F\*/Dafny approach—Z3's error probability is lower than the compiler's own bug rate, which is a pragmatic engineering choice). The real unreliability is controlled at the SMT translation layer—if the translation has bugs, the compiler will expose them in other tests.

**Interface**: the compiler internally translates to SMT-LIB 2.6 standard format, rather than binding to a specific solver API. SMT-LIB is an ISO standard; Z3, CVC5, MathSAT, and Yices all support it natively.

**Default backend**: Z3 (MIT license, the most extensively documented and community-validated). CVC5 serves as an SMT-LIB-compatible alternative—users can switch via compiler flags at compile time.

No "generic solver abstraction layer"—SMT-LIB *is* the abstraction layer. If CVC5 makes breakthroughs in specific theories in the future, switching only requires changing the binary, not the compiler code.

```
Compile-time Bool expression
        │
        ├── Compiler kernel can directly judge (structural equivalence, simple arithmetic,
        │   trivial formulas after constant folding)
        │   → directly return Proved / Disproved
        │
        └── Compiler kernel cannot directly judge (quantifiers, symbolic variables)
            → dependent type pre-reduction (factorial(5) → 120)
            → translate to SMT-LIB format
            → send to Z3/CVC5 (with budget limit)
            → return: unsat → Proved  │  sat + model → Disproved  │  unknown → Unproven
```

**Solver Budget—Hard Limit, Like Stack Depth**:

| Budget Dimension | Default Value | Description |
|------------------|---------------|-------------|
| Solving steps | 10,000 | Z3 typically stays within hundreds of steps for linear arithmetic. 10,000 steps covers 99% of practical predicates. |
| Time | 100ms | A single predicate exceeding 100ms = the user is writing a compile-time program rather than a type annotation. 100ms × 50 predicates = 5-second compile time upper bound. |
| Quantifier instantiation depth | 3 | Three nested quantifier levels cover practical patterns. More than three layers is most likely logic exercise writing. |

Exceeding the budget returns `Unproven`, compile error + predicate location + consumption. No degradation, no runtime check, no silent pass.

**Why this is practically viable**: 95% of practical predicates in engineering are linear arithmetic—`x > 0`, `arr.len > 0`, `0 <= idx < arr.len`—all within decidable fragments; SMT solvers return in milliseconds on such problems. For the rare complex predicates that exceed the budget, programmers can write proof functions.

Dependent types undergo a pre-reduction layer before calling SMT: `factorial(5)` is directly evaluated at compile time to `120`; `append([1,2], [3])` is directly evaluated to `[1,2,3]`. These deterministic value computations do not consume SMT budget.

Programmers need not know SMT exists. The mental model is: **the compiler proves what it can, reports an error on what it cannot—and if the compiler cannot, you can write a function to prove it**.

### 9. Compile-time Predicate Composition

Compile-time predicates are functions returning `Type`; composition is naturally achieved through function composition:

```yaoxiang
SortedNonEmpty: (T: Ord, arr: Array(T)) -> Type = {
    Sorted(T, arr) && NonEmpty(arr)
}
```

### 10. Code Examples

#### 10.1 Division Safety

```yaoxiang
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b

result = divide(10, 2)   # ✅ Compiler verifies Positive(2) = { 2 > 0 } → True
# result = divide(10, 0)  # ❌ Compiler verifies Positive(0) = { 0 > 0 } → False
```

#### 10.2 Array Access Safety

```yaoxiang
InBounds: (idx: Int, arr: Array(T)) -> Type = { 0 <= idx && idx < arr.len }

get: (arr: Array(T), idx: InBounds(idx, arr)) -> T = arr.data[idx]

arr = Array(Int)(1, 2, 3)
x = get(arr, 1)   # ✅ Compiler verifies InBounds(1, arr) = { 0 <= 1 && 1 < 3 } → True
# y = get(arr, 5)  # ❌ Compiler verifies InBounds(5, arr) = { 0 <= 5 && 5 < 3 } → False
```

#### 10.3 Sort Correctness

```yaoxiang
Sorted: (T: Ord, arr: Array(T)) -> Type = {
    forall i in 0..arr.len-1: arr[i] <= arr[i+1]
}

sort: (T: Ord) -> ((arr: Array(T))) -> (result: Sorted(T, result)) = {
    result = arr.clone()
    # ... sort algorithm implementation ...
    return result
}
```

#### 10.4 Loops: Compiler VC Generation

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0
    mut i: UpTo(arr.len) = 0
    while i < arr.len {
        s += arr[i]
        i += 1
    }
    return s
}
```


### 11. Dispatch Pipeline: Unifying Compile-time and Runtime Dispatch

`assert` and `Assert` are two sides of the same refinement type primitive. The dispatch pipeline `dispatch` automatically decides between compile-time proof and runtime check based on **whether the predicate's free variables are reachable at compile time**:

| Criterion | Mode | Behavior |
|-----------|------|----------|
| All free variables known at compile time (generic parameters, compile-time constants) | **CompileTime** | Enter proof pipeline: Proved → erased, Disproved → compile error, Unknown → require proof |
| Some free variables come from runtime (function parameters, external inputs, mut variables) | **Runtime** | Insert runtime check, and inject refinement facts into the flow-sensitive assumption set Γ |

**Key point**: "Cannot judge" ≠ "disproved". In CompileTime mode, Unknown requires proof (no silent downgrade); in Runtime mode, the proposition has no truth value at compile time at all—no matter how strong the prover is, it cannot write a universally true proof for "the user may have input a negative number"; a runtime check is the only sound choice. This is not the prover being insufficiently strong; it is theoretical necessity.

### 12. Flow-sensitive Assumption Set Γ: Strongest Postcondition Propagation

The compiler maintains a flow-sensitive assumption set Γ, tracking propositions known to hold at each control flow point.

**SP (Strongest Postcondition) propagation**:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
```

**Kill set for `mut` variables**: after a `mut` variable is reassigned, all assumptions involving that variable are removed from Γ:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
mut x = x - 5       // Γ = {}  ← x > 0 is killed
```

This is a hard soundness requirement—when a variable's value changes, old assumptions become invalid.

**Branch confluence**: when IF/ELSE or match branches merge, Γ takes the intersection of each branch's assumptions. Only propositions holding on all paths are carried out of the branch.

### 13. Erasure Model Clarification: Witness Erasure ≠ Check Erasure

RFC-027's assertion that "refinement types are **completely erased** at runtime" refers to the **proof witness**—proof terms already verified at compile time produce no runtime code. However, **runtime checks** inserted by `dispatch` in Runtime mode are preserved—they are `Bool` checks executed at the value level, not witnesses at the type level.

Summary: witness erased, check preserved. The two are not in conflict; the original RFC-027 assertion stands.

## Detailed Design

### Syntax Changes

| Before (RFC-022) | After (This RFC) |
|---|---|
| `//! requires: NonEmpty(n) = n > 0` | Compile-time predicate as parameter type `(b: Positive(b))` |
| `//! ensures: ExistsMax(result, arr)` | Return type uses return parameter `-> (result: IsMax(T, arr, result))` |
| `/*! invariant: ... !*/` | Compile-time predicate type annotation on variable—Floyd-Hoare invariant |
| `//! decreases: n` | Compiler fully automatically derives measure function |
| Specification is annotation | Specification is type system |

### Syntax

**Compile-time predicates have no new keyword.** `{}` is the proof space, fully consistent with the existing type definition syntax. A compile-time predicate is a function returning `Type`—`name: (params) -> Type = { assertions }`. Using it is a function call—`Positive(b)`, `IsMax(T, arr, result)`.

```bnf
# Compile-time predicate = function returning Type, with assertions inside {} verified by compiler
# Uses existing function/type syntax, no new BNF rules needed
predicate ::= identifier ':' params '->' 'Type' '=' '{' assertions '}'
```

**New syntax concept: return parameter**—in `-> (name: Type)`, `name` is the return parameter.

The return parameter is the **only one syntax concept** YaoXiang introduces on top of the existing function syntax. Its semantics:
- `name`'s value is provided by the `return` statement
- `name` exists only in the type signature, referenced by postcondition predicates (e.g., `-> (result: IsMax(T, arr, result))`)
- `name` does not enter the function body scope, does not appear at the call site
- The return parameter is **optional**—when there is no postcondition, the signature is identical to an ordinary function (`-> Int`), introducing no extra burden

Rationale for introducing it: postconditions need to reference "the value the function is about to return." Without a return parameter, the compiler can only use special rules (such as the implicit variable `$result` or `__retval__`) to let the predicate reference the return value. The return parameter makes this reference explicit—it is just a parameter, except that the value is provided by `return` rather than the caller.

**Proof function** is not a new concept—it is a YaoXiang function whose return type is the proposition being asserted. When the compiler returns `Unproven`, the programmer provides a proof function, and the type checker verifies it in exactly the same way it verifies any function's return type. No new syntax, no new keywords, no new rules.

### Type System Impact

- **Type universe**: compile-time predicates reside at the Type₂ layer—functions that accept values and return Type, on the same layer as type constructors
- **Generics interaction**: compile-time predicates can take generic parameters, e.g., `NonEmpty: (T: Type) -> (arr: Array(T)) -> Type`
- **Ownership interaction**: expressions in compile-time predicates obey ownership rules; they can only read, not write
- **Type inference**: arguments of compile-time predicates participate in HM type inference

### Runtime Representation

Compile-time predicates are handled at runtime **according to the dispatch pipeline result**:
- **CompileTime mode** (all free variables known at compile time): the proof witness is completely erased after the proof passes. `Positive: (x: Int) -> Type = { x > 0 }`—parameter `b: Positive(5)`'s runtime representation is just `Int`. The refinement condition `{ 5 > 0 }` has been verified; erased.
- **Runtime mode** (runtime free variables exist): the runtime check is preserved—execute a `Bool` check at the value level, injecting into the flow-sensitive assumption set Γ. See §11 Dispatch Pipeline and §13 Erasure Model Clarification for details.

Placing a compile-time predicate at the type position (e.g., `f(x: Positive(x))`) does not produce a wrapper type or allocate extra memory. However, when `x` comes from runtime input, a **runtime `Bool` check will be inserted**.

**Interaction constraint with `ref`**: compile-time predicates may only reference immutably borrowed values or values whose ownership has been transferred. Compile-time predicates referencing mutably borrowed values cannot be guaranteed by the compiler to remain valid at runtime—such usage is directly reported as a compile error.

### Compiler Changes

1. **Parser**: compile-time predicates use standard function syntax; no extra parsing rules needed
2. **Compile-time proof pipeline**: unified Proved/Disproved/Unproven return interface, automatic strategy selection
3. **SMT acceleration module**: SMT-LIB 2.6 translation layer, default backend Z3, CVC5 alternative
4. **Type checker kernel**: implementation of inference rules—structural equivalence, βδι-reduction, universal quantifier introduction/elimination. This is the sole root of trust; both SMT and programmer proofs are verified through it
5. **Verification condition generation**: WP/SP calculus + loop invariant proof obligations
6. **Error reporting**: counterexample formatting + unproven proposition report + source location association

### Backward Compatibility

- ✅ Code that does not use compile-time predicates is completely unchanged
- ✅ Compile-time predicates have zero runtime overhead in CompileTime mode; in Runtime mode only necessary `Bool` checks are preserved
- ⚠️ RFC-022's `//!` syntax is no longer supported—but 022 was never implemented, so there is no migration burden

## Trade-offs

### Advantages

- **Full realization of the Curry-Howard correspondence**: types are propositions, programs are proofs, `name: Proposition = Proof`
- **Uniformity**: compile-time predicates and ordinary functions use exactly the same syntax; no conceptual split
- **SMT transparency**: programmers need not know SMT exists; the mental model matches type checking
- **Progressive adoption**: start with a single compile-time predicate, then expand coverage
- **Minimal runtime overhead**: zero overhead in CompileTime mode; only necessary `Bool` checks in Runtime mode

### Disadvantages

- **Compile time**: SMT solving increases compile time, but the hard budget limit keeps the upper bound controllable
- **Automatic proof boundary**: complex predicates beyond first-order linear arithmetic may require the programmer to write a proof function. This is not a language defect—it is the inevitable conclusion of the halting problem. The compiler honestly reports `Unproven` rather than falsely reporting `True`/`False`
- **Learning curve**: writing effective compile-time predicates and proof functions requires understanding the basic intuition of the Curry-Howard correspondence
- **Implementation complexity**: unifying the compile-time proof pipeline requires careful design

### Risk Mitigation

- SMT solver budget hard limit (steps 10,000 / time 100ms / instantiation depth 3); exceeding budget returns `Unproven`
- Dependent type pre-reduction: deterministic value computation is consumed first; SMT only tackles the non-deterministic part
- `Unproven` is not a dead end: programmers can write proof functions; the type checker verifies—same as verifying any function's return type
- Incremental validation: only validate changed modules
- Clear error messages + counterexample display + budget consumption report + unproven proposition + suggestions (if the compiler can provide them)

## Alternatives

| Alternative | Why Not Chosen |
| ----------- | -------------- |
| RFC-022: `//!` annotation-style specifications | Specifications and types are split; violates the Curry-Howard correspondence |
| Separate specification file (e.g., CVL) | Specifications separated from code; increases maintenance cost |
| Runtime-only assertions | Cannot statically guarantee correctness |
| External proof assistant (e.g., Coq) | Detached from the compiler; requires an independent proof language and trust boundary. YaoXiang's choice: proof is YaoXiang code; the type checker is the sole root of trust |
| **This proposal: compile-time predicates as first-class citizens** | ✅ |

## Implementation Strategy

### Phases

| Phase | Content |
| ----- | ------- |
| **Phase 1** | Compiler kernel: structural equivalence + βδι-reduction + universal quantifier introduction/elimination. Supports simple arithmetic predicates (`x > 0`, `arr.len > 0`) |
| **Phase 2** | SMT-LIB translation layer + Z3/CVC5 integration. Pipeline returns Proved/Disproved/Unproven. When Unproven, supports programmer-written proof functions |
| **Phase 3** | Loop invariant VC generation + termination checking (linear rank function + predicate violation counting + bounded pattern + combinatorial explosion control) |
| **Phase 4** | Incremental validation + caching + IDE support |

### Dependencies

- RFC-010: Unified Type Syntax — compile-time predicates are based on `name: type = value`
- RFC-011: Generics System — compile-time predicates can take generic parameters
- RFC-009: Ownership Model — expressions in compile-time predicates obey ownership rules

## Open Questions

- [x] **SMT solver choice**: default Z3 (MIT license, most extensively validated). CVC5 as SMT-LIB-compatible alternative, switched via compiler flag. The compiler's internal translation target is SMT-LIB 2.6 standard format—SMT-LIB *is* the abstraction layer; no custom generic solver interface.
- [x] **Specific budget values**: steps 10,000 / time 100ms / quantifier instantiation depth 3. Fixed inside the compiler; no knobs. If real use cases prove the values insufficient during actual use (not "user wrote it wrong"), adjust later.
- [x] **Quantifier support scope**: the language does not limit quantifier arity. Compile-time predicates accept `Type` parameters—`Type` includes function types—so higher-order quantifiers are a natural consequence of the type system and need no special syntax. The SMT solver can automatically judge first-order quantifiers (forall/exists, supporting interleaved nesting, limited by budget depth 3). Higher-order quantifiers: SMT returns `Unproven`; compiler prompts "this predicate exceeds the auto-proof range; please provide a proof function." The programmer writes a YaoXiang function whose return type equals the proposition—the type checker verifies the function. No external export, no AI, no interactive proof mode needed. Everything is YaoXiang code; everything is verified by the type checker.
- [x] **Counterexample formatting**: source variable names are used directly as SMT variable names (with module prefix to avoid conflicts). When Z3 returns a model, lookup is by variable name. Output format: variable name = concrete value + source location + predicate definition location. No complex mapping layer.
- [x] ~~**Interaction between compile-time predicates and `ref` smart pointers?**~~ → Decided: compile-time predicates only allow immutably borrowed values or values whose ownership has been transferred. Values under mutable borrow cannot appear in compile-time predicates.
- [x] **Extension of forall predicate violation counting measure to non-adjacent operations?** → No extension. Current coverage (adjacent swap, adjacent shift) is complementarily covered by Strategy 1 (linear rank function)—quicksort's outer interval contraction is covered by Strategy 1, heapsort by Strategy 1 (array index pattern). Loops whose termination cannot be proved by any strategy are reported directly by the compiler as errors—this is the hard-safety philosophy, not a defect. If future real-world scenarios (not academic constructions) have algorithms none of the four strategies can cover, revisit.
- [x] **Combinatorial explosion in linear rank function enumeration**: candidate enumeration upper limit is 3 bounded variables. ≤3 enumerates all linear combinations and verifies each via SMT one by one. >3 tries only single-variable measures (`v_i`, `u_i - v_i`); failure reports a compile error directly—prompting the programmer "the loop has >3 bounded variables; the compiler cannot auto-synthesize a multi-variable measure." This is not an engineering compromise—it forces the programmer to write simpler loops.

## References

- [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md)
- [RFC-011: Generics System Design](../accepted/011-generic-type-system.md)
- [RFC-009: Ownership Model](../accepted/009-ownership-model.md)
- Howard, W. A. (1969). The Formulae-as-Types Notion of Construction.
- Swamy, N. et al. (2016). Dependent Types and Multi-Monadic Effects in F\*. _POPL 2016_.
- Vazou, N. et al. (2014). Refinement Types for Haskell. _ICFP 2014_.
- Leino, K. R. M. (2010). Dafny: An Automatic Program Verifier for Functional Correctness. _LPAR 2010_.
- De Moura, L. & Bjørner, N. (2008). Z3: An Efficient SMT Solver. _TACAS 2008_.

---

## Lifecycle and Destination

```
┌─────────────┐
│   Draft     │  ← Author creates
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Reviewing  │  ← Current state: community discussion
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  Accepted   │    │  Rejected   │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│ accepted/   │    │  rejected/  │
│ (formal     │    │ (preserved  │
│  design)    │    │  in place)  │
└─────────────┘    └─────────────┘
```