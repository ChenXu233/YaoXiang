---
title: "RFC-027: Compile-Time Predicates and Unified Static Verification"
status: "Accepted"
author: "ChenXu"
created: "2026-06-07"
updated: "2026-07-05"
impl_status: "in_progress"
impl_detail: "Phase 1-2 complete, Phase 3 partial, Phase 4 partial"
impl_percent: 70
issue_number: 90
issue_url: "https://github.com/ChenXu233/YaoXiang/issues/90"

issue: "#90"
---

# RFC-027: Compile-Time Predicates and Unified Static Verification

> **References**:
>
> - [RFC-009: Ownership Model](../accepted/009-ownership-model.md)
> - [RFC-010: Unified Type Syntax - name: type = value Model](../accepted/010-unified-type-syntax.md)
> - [RFC-011: Generic Type System Design](../accepted/011-generic-type-system.md)
> - [RFC-024: spawn block-based Concurrency Model](../accepted/024-concurrency-model.md)
>
> **Supersedes**: [RFC-022: Hoare Logic Static Verification Support (Specification Annotations and Specification Types)](../deprecated/022-hoare-logic-static-verification.md) — Deprecated

## Abstract

This RFC proposes introducing **compile-time predicates** as first-class citizens in YaoXiang, unifying all compile-time static verification into a single **proof pipeline**. Compile-time predicates are not external specification annotations—they are functions. A function that returns `Type` can be used at type positions, and the compiler calls it at compile time and checks the return value. Types are propositions; compile-time evaluation is proof.

**Core thesis**: The only work of type checking at compile time is to construct and verify proof terms. Type equality, token conflicts, dependent type reduction, compile-time predicate evaluation, Hoare logic implications—all are different type checks within the compile-time proof pipeline, sharing the same pipeline. The SMT solver is an acceleration module of the type checker, not an independent trust boundary. When the compiler returns `Unproven`, the programmer writes a YaoXiang function as a proof—the type checker verifies it in exactly the same way it verifies any function's return type. Everything is YaoXiang code; everything is verified by the type checker.

## Motivation

### Why deprecate RFC-022?

RFC-022 designed specifications as `//!` annotations:

```yaoxiang
max: (T: Ord) -> ((arr: Array(T, n)) -> T) = {
    //! requires: NonEmpty(n) = n > 0          ← This is an annotation independent of types
    //! ensures: ExistsMax(result, arr[0..n])   ← This is an annotation independent of types
}
```

This commits the fundamental mistake against the Curry-Howard isomorphism: **splitting specifications and types into two layers**. Annotations are not types. Annotations do not participate in type checking. Annotations are mental models of "external tools."

The white paper states clearly:

> "No `//!` annotations. No independent specification language. Everything is within the type system."

### Current Problems

- RFC-022's `//!` annotations are external syntax independent of the type system
- Specification types and regular types are two separate systems, causing conceptual redundancy
- The Debug Build verifies / Release Build ignores split pattern breaks unity
- SMT solvers are positioned as external tools in conventional understanding—YaoXiang builds them in as acceleration modules of the type checker
- Type checking, borrow verification, compile-time predicate checking, and macro expansion each take different paths

### The Correct Mental Model

Type checking can be abstracted as a function:

```
verify : Program → Proved | Disproved(Model) | Unproven
```

All compile-time checks—simple type matching, borrow conflict detection, compile-time predicate verification—are subtasks of this function. They share the same proof pipeline; the only difference is the complexity of proof terms and construction strategy.

When the compiler returns `Unproven`, the programmer provides a proof function—whose return type equals the proposition to be proven. The type checker verifies it. This is the same operation as ordinary type checking.

## Proposal

### 1. `{}` Is the Proof Space: Types Are Assertions, Verification Is Type Checking

YaoXiang's `{}` is the compile-time proof space. Everything within it is an assertion, and the compiler guarantees each item is True—either by automatic proof or by programmer-provided proof functions.

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
#          Parameter in signature    {} contains only assertions
#          Compiler verifies x > 0 at compile-time call site

List: (T: Type) -> Type = { data: Array(T) }
#      ^^^^^^^^              ^^^^^^^^^^^^^^^
#      Parameter in signature   Compiler verifies type_of(T) == Type, type_of(data) == Array(T)
```

The same pattern: `name: (params) -> Type = { assertions }`. The compiler does not distinguish "type assertions" from "value assertions"—both are evaluation targets in the proof pipeline.

**Loop invariants do not need to be written separately. Type annotations on variables are Floyd-Hoare invariants.**

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # Annotation references i—tells the compiler s's type depends on i
    mut i: UpTo(arr.len) = 0     # At initialization i=0, verify: 0 == sum(arr[0..0]) → True
    while i < arr.len {
        s += arr[i]  # Compiler verifies: s_new == sum(arr[0..i+1])
        i += 1       # i changes → triggers s dependency reverification: s satisfies SumUpTo(arr, i_new)
    }
    return s  # s: SumUpTo(arr, arr.len) = sum(arr[0..arr.len])
}
```

The compiler generates one verification condition for the loop body—inductive hypothesis (type annotation) → assignment operation → whether the new value satisfies the type annotation. Once the proof pipeline verifies the inductive step holds, all iterations are automatically covered. No `: decreases`, no `: Invariant`, no inductive proof needed—the compiler decomposes induction into local VCs for each assignment.

### 2. Pre/Postconditions: Compile-Time Predicates on Parameter Types and Return Types

Abandon RFC-022's `//! requires`/`//! ensures`. Compile-time predicates serve as type annotations on parameters or return values.

**The parameter side is a function call.** Compile-time predicates are functions that return `Type`, and their usage on the parameter side is simply calling them—just like `factorial(5)`. The return value side introduces a new concept: return value parameters.

```yaoxiang
# Precondition: explicitly call compile-time predicate in parameter type
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b
#                       ^^^^^^^^^^  b is the current parameter name, passed to Positive as argument
#                       Compiler extracts actual argument at call site, substitutes for b, verifies Positive(actual)
#                       Example: divide(10, 2) → verify Positive(2) = { 2 > 0 } → True
#                       Example: divide(10, 0) → verify Positive(0) = { 0 > 0 } → False → compile error

# Postcondition: return value parameter + compile-time predicate
IsMax: (T: Ord, arr: Array(T), result: T) -> Type = {
    forall j in 0..arr.len: result >= arr[j]
}

NonEmpty: (arr: Array(T)) -> Type = { arr.len > 0 }

max: (T: Ord) -> ((arr: NonEmpty(arr))) -> (result: IsMax(T, arr, result)) = {
#                                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
#                                            result is the return value parameter, value provided by return
#                                            Compiler substitutes return value for result at return point, verifies postcondition
    candidate = arr[0]
    for i in 1..arr.len {
        if arr[i] > candidate { candidate = arr[i] }
    }
    return candidate
}
```

**Key rules**:

- **Parameter side**: `b: Positive(b)` — `b` is the current parameter name, passed to `Positive` as an argument. Function call syntax, zero implicits.
- **Return side**: `-> (result: IsMax(T, arr, result))` — `result` is the return value parameter, value provided by the `return` statement. `result` only exists in the type signature, only referenced by predicates, does not enter the function body scope, does not appear at the call site.
- **Return value parameter is optional**: omit it when there's no postcondition; the signature is exactly the same as a regular function (`-> Int`).
- **Unity**: Parameter and return value parameters are the same concept—`paramName: predicateCall(paramName)`—the only difference is whether the value is provided by the caller or by `return`.

### 3. Path Condition Propagation: Compile-Time Verification of Runtime Values

When compile-time predicates are used at binding positions, parameters are passed explicitly by the programmer. When runtime values enter refinement type parameters, the compiler completes verification through path condition collection and SMT implication judgment—no explicit proof passing required.

#### 3.1 Explicit Function Call

When compile-time predicates are used at binding positions, parameters are passed explicitly by the programmer—it is a function call, zero implicits.

`Positive: (x: Int) -> Type = { x > 0 }` is a compile-time predicate constructor. When it appears at a binding position (parameter declaration, variable declaration, return type), the programmer explicitly passes the already-bound variable name:

```yaoxiang
b: Positive(b)
// b has been declared as the current parameter, Positive(b) is a function call
// After normalization: b: { b > 0 }
```

The compiler does not need to implicitly fill in parameters—`b: Positive(b)` is a function call, just like `f(5)`. `b` is bound as a parameter name, and its type annotation `Positive(b)` references `b` itself—this is the standard pattern of dependent types, not an implicit expansion rule.

**Unification with RFC-010's `self`**: RFC-010 establishes that `self` is not a keyword, just a conventional name for a parameter ("writing `p`, `this`, `x` has exactly the same effect"). `b: Positive(b)` shares the same mechanism—the parameter name can be referenced in type annotations. `self` appears at the position of `self: Point`; `b` appears at the position of `b: Positive(b)`; both have type annotations that reference the parameter itself. The only difference is the complexity of the type annotation; the mechanism is exactly the same—once a name is bound, the type can depend on that name.

The return type also uses explicit function calls:

```yaoxiang
Sorted: (arr: Array(T)) -> Type = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }

sort: (arr: Array(T)) -> (result: Sorted(result)) = { ... }
//                        ^^^^^^^^^^^^^^^^^^^^^^^
//                        result is the return value parameter, Sorted(result) is a function call
//                        Compiler substitutes return value for result at return point, verifies Sorted(return_value)
```

The same applies to local variable declarations:

```yaoxiang
let x: Positive(x) = 5
// x is bound to 5, Positive(5) → { 5 > 0 } → True → Pass

// let y: Positive(y) = 0
// y is bound to 0, Positive(0) → { 0 > 0 } → False → compile error
```

#### 3.2 Path Condition Collection

When runtime values appear in conditional branches, the compiler automatically collects path conditions, forming the current scope's **assumption set**. These assumptions participate in verification as background knowledge for compile-time `Bool` evaluation.

```yaoxiang
if y > 0 {
    // Compiler automatically gains assumption in this branch: { y > 0 }
    let result = divide(x, y)
    // Verification condition: (y > 0) ⇒ (y > 0)
    // Proof pipeline judges implication holds → Proved
} else {
    // This branch assumes: { !(y > 0) }
    // If divide(x, y) is called, verification condition is !(y > 0) ⇒ y > 0
    // Proof pipeline judges not implied → Disproved
}
```

This is not the compiler hard-coding special patterns—this is the natural behavior of the compile-time proof pipeline. Each type check call site sends to the pipeline:

```
{background assumptions} ⇒ {verification target}
```

The proof pipeline judges implication. `Proved` → pass; `Disproved` → compile error + counterexample; `Unproven` → compile error + unresolved proposition. Background assumptions come from the path conditions of the current program point.

#### 3.3 Assumption Stack

The compiler maintains an assumption set for each basic block during control flow analysis:

- **if-guard**: `if y > 0` → true branch pushes `y > 0`, false branch pushes `!(y > 0)` (if else is used)
- **match pattern**: `if let Some(v) = opt` → branch pushes `opt == Some(v)`
- **logical conjunction**: `if x > 0 && y < 10` → branch pushes `x > 0` and `y < 10`
- **function precondition**: when calling `divide(a, b)`, the evidence that `b` satisfies `Positive` must come either from the current assumption, or from `b`'s own refinement type annotation (`b` is annotated as `Positive`, then its type carries `b > 0`)
- **assignment**: when `let z = y`, the refinement condition on `y` propagates to `z`

All assumptions enter the compile-time proof pipeline. When entering the SMT acceleration path, they are translated to SMT-LIB background assertions.

#### 3.4 No Static Evidence Means Compile Error

If the programmer directly writes:

```yaoxiang
divide_user_input: (x: Int, y: Int) -> Int = divide(x, y)
```

The current program point has no `y > 0` assumption, and the actual argument `y` itself has no `Positive` type annotation. The verification condition is:

```
{} ⇒ { y > 0 }
```

The pipeline returns `Disproved` (not implied) → compile error:

> Cannot prove that argument `b` satisfies `Positive` in the `divide` call.
> `y` comes from function input, with no proven bound.
> Consider guarding the call with an if branch: `if y > 0 { divide(x, y) }`.

YaoXiang does not accept runtime values entering refinement type parameters without providing static evidence. This is not a restriction—this is the core of the hard-safety philosophy. Any code the compiler cannot statically prove shall not pass compilation.

#### 3.5 Relationship with the Unified Pipeline

Path condition propagation is not an additional mechanism. It is a direct extension of the compile-time proof pipeline in control flow analysis:

| Phase | Responsibility |
|------|---------|
| Path condition collection | Compiler control flow analysis phase, annotates assumption set for each basic block |
| Verification condition generation | When encountering type constraints to verify, merge path conditions + actual argument type information |
| Proof pipeline evaluation | Compiler kernel → SMT acceleration → derive Proved / Disproved / Unproven |
| Result | `Proved` → pass; `Disproved` → compile error + counterexample; `Unproven` → compile error + unresolved proposition (programmer can provide proof function) |

No new components. No special rules. Path conditions are the background knowledge of the proof pipeline—sharing the same pipeline and budget system as type equality and borrow constraints.

### 4. Compile-Time Proof Pipeline

All compile-time checks share the same pipeline. The core operation of the pipeline is **type checking**—checking whether a proof term's type equals the proposition to be proven. Everything is type checking.

```
Compile-time encounters Bool expression needing evaluation (i.e., needs to construct a proof term)
        │
        ├── Type equality (T1 == T2)
        │   → Compiler judges directly (structural equivalence)
        │
        ├── Token conflict condition (!conflicting(tokens))
        │   → Flow-sensitive liveness analysis (Dup/Linear property tracking)
        │
        ├── Dependent type reduction (n + m simplification)
        │   → Compile-time term rewriting system (βδι-reduction)
        │
        ├── Compile-time predicate (x > 0, forall...)
        │   → Compiler itself + SMT acceleration module
        │
        └── Hoare logic implication (P ⇒ Q)
            → Compiler + SMT acceleration module
                    │
                    ▼
             ┌──────────┐
             │ Proved   │  → Compilation passes
             │ Disproved│  → Compile error + counterexample
             │ Unproven │  → Compile error + unresolved proposition
             └────┬─────┘
                  │
                  ▼
         Programmer writes proof function (YaoXiang code)
                  │
                  ▼
         Type checker verifies ──→ Proved ──→ Compilation passes
                  │
                  ▼
            Verification failed → Compile error: "Proof does not hold"
```

#### 4.1 Proof Results: Three-Valued Algebra

Compile-time evaluation returns three results—this is a necessary conclusion from the halting problem and a natural division of proof theory:

```
eval_compile_time : BoolExpr → Proved | Disproved(Model) | Unproven
```

- **Proved** → Halts, proof term constructed, type check passes. Compilation continues.
- **Disproved(M)** → Halts, counterexample M exists. Compile error + counterexample + source location.
- **Unproven** → Within given resource bound, no proof constructed. Compile error + unresolved proposition + budget consumption report.

**Unproven ≠ False.** The compiler saying "I cannot prove it" is not equivalent to the proposition being false—it just exceeds current automatic proof capability. This is honesty, not a defect.

The budget hard limit is the engineering solution to the halting problem. No knob is given—giving one would be asking the user "do you think your program will halt," and neither the user nor the compiler knows.

#### 4.2 After Unproven: Programmer Writes Proof

When the compiler returns `Unproven`, the programmer can write a **proof function**—a YaoXiang function whose return type equals the proposition to be proven. The type checker verifies this function—in exactly the same way it verifies `add(a, b): Int`.

```
Proposition = Type
Proof       = Program (a value of that type)
Verification = Type checking (the only trust root)
```

The SMT solver is not an independent trust boundary—it is an **acceleration module of the type checker**. SMT helps find proofs, but the proof is always verified by the type checker. When SMT returns `unsat`, the compiler reconstructs its result as a proof term verifiable by the type checker. If reconstruction fails (SMT's reasoning steps exceed the compiler kernel's inference rules), it falls back to `Unproven`—the programmer can manually write a proof function.

```yaoxiang
# Proposition: a refinement property the compiler cannot automatically prove
FirstIsMin: (T: Ord, arr: Sorted(T)) -> Type = {
    forall i in 0..arr.len: arr[0] <= arr[i]
}

# Proof: programmer writes a function whose return type is the above proposition
# Type checker verifies this function—exactly the same as verifying add(a,b): Int
first_is_min: (T: Ord, arr: Sorted(T)) -> FirstIsMin(T, arr) = {
    # Compiler verifies here: function body's type = FirstIsMin(T, arr)
    ...
}
```

No AI required, no export to Coq, no new concepts. **Properties the compiler cannot automatically prove at compile time → programmer writes proof in YaoXiang code → type checker verifies.** The whole process is a smooth gradient—the compiler handles simple proofs, leaving the hard ones for the brain.

#### 4.3 Layered Dependencies Within the Pipeline

The evaluators above share the same interface but have an evaluation order. Type equality is the prerequisite for all subsequent analyses; ownership/token checks depend on type information; refinement predicate verification depends on the results of the first two layers. The compiler evaluates layer by layer; expressions failing at lower layers do not enter upper layers—avoiding wasting solver budget on type-incorrect programs.

```
Evaluation order (same pipeline, layered scheduling)
├── Layer 0: Type equality (T1 == T2)
│   └── Structural unification → failure means subsequent is meaningless, return Disproved directly
├── Layer 1: Ownership/token conflicts
│   └── Flow-sensitive liveness analysis → failure means memory safety does not hold, return Disproved directly
└── Layer 2: Refinement predicate / Hoare implication
    └── Compiler itself → SMT acceleration → derive Proved / Disproved / Unproven
```

Each layer still returns `Proved/Disproved/Unproven`, sharing the same interface and the same budget system.

### 5. Three-Layer Function Unification

| Layer | Timing | Input | Output | Example |
|------|----------|------|------|------|
| Value-level function | Runtime | Value | Value | `add: (a: Int, b: Int) -> Int = a + b` |
| Type constructor | Compile-time | Type/Value | Type | `List: (T: Type) -> Type = { data: Array(T) }` |
| Compile-time predicate | Compile-time | Value | Type | `Positive: (x: Int) -> Type = { x > 0 }` |

All use the same `name: type = value` syntax. Compile-time predicates and type constructors go through the same compile-time proof pipeline—`{}` is the proof space.

### 6. Loops: Floyd-Hoare Verification Condition Generation

Loops do not need separate `: Invariant(...)` or `: decreases(...)` annotations. Compile-time predicate type annotations on variables define Floyd-Hoare-style assertions—the compiler generates verification conditions from type annotations, and the proof pipeline checks whether each assignment maintains the type.

Core mechanism: each assignment operation corresponds to a Hoare triple `{P} x := e {Q}`, where the verification condition is `P ⇒ Q[e/x]`. The compiler generates one verification condition for the loop body—once the proof pipeline verifies the inductive step holds, all iterations are automatically covered.

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # Annotation references i; at initialization i=0, verify: 0 == sum(arr[0..0]) → True
    mut i: UpTo(arr.len) = 0     # Verify: 0 <= 0 <= arr.len → True
    while i < arr.len {
        # Compiler generates one VC for the loop body. Premise: s satisfies SumUpTo(arr, i), i satisfies UpTo(arr.len).
        #
        # s += arr[i]:
        #   Verification obligation: s_new satisfies SumUpTo(arr, i) (current i unchanged)
        #   Substituting s_new = s_old + arr[i]:
        #     Need s_old + arr[i] == sum(arr[0..i+1])
        #     From inductive hypothesis s_old == sum(arr[0..i]), adding arr[i] to both sides:
        #     sum(arr[0..i]) + arr[i] == sum(arr[0..i+1])
        #   Compiler + SMT: linear arithmetic, milliseconds → Proved
        #
        # i += 1:
        #   i changes → dependency graph shows s's type annotation references i → triggers reverification
        #   New verification target: s satisfies SumUpTo(arr, i_new)
        #   i.e. s == sum(arr[0..i_new]), guaranteed by previous step → Proved
        s += arr[i]
        i += 1
    }
    return s  # At this point s: SumUpTo(arr, arr.len), i.e. s == sum(arr[0..arr.len])
}
```

Loop invariants are the type annotations on variables—the programmer writes types, the compiler checks the inductive step. The compiler does not need to "discover" invariants, nor "automatically perform induction"—it decomposes the inductive proof into local verification conditions for each assignment operation, handed to the proof pipeline for divide-and-conquer.

#### 6.1 Dependency Tracking: Dependent Types on Mutable Variables

The prerequisite for the above mechanism is: the compiler knows that `s`'s type annotation `SumUpTo(arr, i)` references `i`—when `i` changes, the type constraint on `s` also changes. This requires the compiler to maintain a **type dependency graph between variables**.

**Data structure**:

```
TypeDepGraph: Map<VarName, Set<VarName>>
# Key is the depended-on variable, value is the set of variables whose type annotations reference it
# Example: { i: {s}, j: {s, t}, ... }
```

**Construction**: When the type checker processes `mut v: Pred(... x ...) = init`, it parses free variable references in `Pred(...)` arguments. If the arguments reference another mutable variable `x` in the current scope, it records `x → v` in the dependency graph.

**Trigger**: When the depended-on variable `x` is assigned, the compiler:
1. Looks up all variables `{v₁, v₂, ...}` in the dependency graph that depend on `x`
2. For each `v`, generates a verification condition: does `v`'s current value satisfy the updated type `Pred(... x_new ...)`
3. Sends the VC into the proof pipeline

**Assignment order sensitive**: Dependency tracking naturally enforces the correct assignment order. Taking `SumUpTo(arr, i)` as an example:

```yaoxiang
# Correct order
s += arr[i]   # s_new satisfies SumUpTo(arr, i+1)
i += 1        # i changes → reverify s satisfies SumUpTo(arr, i_new) → True

# Wrong order—compiler rejects
i += 1        # i changes → reverify s satisfies SumUpTo(arr, i_new)
              # s not yet updated, s_old == sum(arr[0..i_old]) ≠ sum(arr[0..i_new])
              # → compile error: variable s does not satisfy type SumUpTo(arr, i_new)
s += arr[i]   # unreachable
```

**Combined dependencies**: A variable can depend on multiple variables. A type annotation `{ v: Int; v == x + y }` depends on both `x` and `y`—either changing triggers reverification.

**Relationship with proof pipeline**: Dependency tracking is the trigger for VC generation, not an independent verification mechanism. It answers "when does a VC need to be generated"—the proof pipeline answers "does the VC hold."

### 7. Termination Checking

Fully automatic at compile time. Loops the compiler can prove pass; loops the compiler cannot prove directly report compile errors—the programmer must make the loop's termination automatically analyzable by the compiler. No half-automatic annotation loopholes.

#### 7.1 Design Principles

The compiler automatically extracts the information needed for termination proof from two sources:

1. **Variable type annotations**: Boundary constraints in refinement types (e.g., `UpTo(n)` gives upper bound `n` and lower bound `0`)
2. **Loop body operations**: Operations applied to variables on each iteration

The compiler tries four metric synthesis strategies in priority order, stopping when one is found.

#### 7.2 Strategy 1: Linear Ranking Function Automatic Synthesis

When variables have linear bound annotations, the compiler enumerates candidate linear metrics and verifies via SMT.

```
Input:
  Variables v₁: UpTo(u₁), v₂: UpTo(u₂), ... (variables with upper/lower bounds)
  Loop condition cond
  Set of assignments in loop body

Algorithm:
  1. Extract each variable's bound from type annotations: [low_i, high_i]
  2. Enumerate candidate metrics: v_i, u_i - v_i, v_i - v_j, etc. linear combinations
  3. For each candidate metric m:
     - SMT verify m ≥ 0 (derived from type bounds)
     - For each execution path in the loop body, SMT verify m' < m (strictly decreasing)
  4. Find a qualifying linear combination → termination proven
```

Coverage scope: any loop where variables are assigned linear expressions (`v = a·v + b`) and have bounded type annotations. Includes `i += const`, `i -= const`, and binary-search-style interval contraction:

```yaoxiang
# Binary search: low = mid + 1 or high = mid
# Metric high - low strictly decreases on both paths
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

#### 7.3 Strategy 2: Predicate Violation Counting — Automatically Extract Metrics from Target Type <span style="color:orange">【Experimental Strategy】</span>

> ⚠️ **Current status: Experimental strategy; whether to include it in Phase 3 implementation will be decided based on actual feasibility.**
> This strategy is effective for adjacent-swap operations (bubble sort, insertion sort), but cannot automatically prove non-adjacent operations (quicksort partition, heapsort sift-down).
> See the coverage boundary table below. If Phase 3 validation proves infeasible, this strategy will be removed or downgraded to future work.

Core insight: **the user-written specification is the compiler's reasoning material.** The compiler does not need to build in "what is sorting"—it reads the `Sorted` definition and automatically extracts metrics from that definition.

```
Input:
  Target type: Sorted(arr) = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }
  Loop body operation: adjacent element swap

Algorithm:
  1. Parse predicate definition: forall i in range: cond(i, arr)
  2. Automatically generate metric: violation_count = |{ i | ¬cond(i, arr) }|
  3. Analyze operation's effect on the metric:
     - Adjacent swap arr[j], arr[j+1] = arr[j+1], arr[j]
     - Only affects three pairs at indices j-1, j, j+1
     - If arr[j] > arr[j+1] (violates predicate), after swap this pair satisfies the predicate
     - violation_count decreases by at least 1
  4. Upper bound: n·(n-1)/2 (max adjacent inversions), lower bound: 0
  → Termination proven
```

**Current coverage scope**:

| Algorithm | Operation pattern | Strategy 2 provable? | Reason |
|------|----------|:---:|------|
| Bubble sort | Adjacent swap | ✅ | violation_count strictly decreases with each swap |
| Insertion sort | Adjacent move | ✅ | Each shift eliminates one violating pair |
| Selection sort | Non-adjacent swap | ❌ | A single swap may increase violation_count |
| Quicksort | partition | ❌ | Non-adjacent swap, no monotonic decrease guarantee |
| Heapsort | sift-down | ❌ | Tree-shaped operation, violation_count non-monotonic |

**Complementary strategies**: For quicksort, the `low < high` interval contraction is covered by Strategy 1 (linear ranking function)—each partition recursion halves the interval. Strategy 1 and Strategy 2 complement each other, covering termination of most practical algorithms. However, generalizing Strategy 2 (non-adjacent operations, tree-shaped operations) remains an open problem.

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

`v += const` (positive constant), variable has upper bound type annotation → metric `upper_bound - v` decreases by `const` each time, lower bound 0. This is a degenerate case of Strategy 1; the compiler handles it first as a fast path.

#### 7.5 Strategy 4: Multiplicative Scaling Metric Template

`v *= const` (const > 1), variable has upper/lower bound type annotation. The compiler has a built-in logarithmic metric template `ceil(log_const(upper/v))`; multiplying by const each time decreases the metric by 1.

```yaoxiang
mut i: Positive(i) = 1
while i < n {
    # Compiler auto-derives: metric ceil(log₂(n/i)), each multiplication by 2 decreases metric by 1
    i *= 2
}
```

#### 7.6 Separation of Termination and Correctness

Termination proof and correctness proof are independent:
- **Termination**: The four strategies above automatically prove the loop exits in finite steps
- **Correctness**: Whether the loop body advances toward the target type is checked by the compile-time proof pipeline via verification conditions

Both pass → compilation passes. Termination proven but correctness fails → compile error + counterexample. Correctness proven but termination cannot be proven → compile error pointing out the variable or operation that cannot be analyzed. Both fail → compile error reporting both failure reasons separately.

#### 7.7 Termination Checking for Recursive Functions

For recursive functions that need to be evaluated at compile time, the compiler checks argument decrease:

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)  # Compiler analyzes: n-1 < n → decreasing → terminates
}

# Compile-time use—compiler guarantees factorial terminates at compile time
vec: Vec(factorial(5)) = Vec(120)()  # 5! = 120, completed at compile time
```

| Scenario | Behavior |
|------|------|
| Compiler can analyze recursive decrease (e.g., `n-1`) | Compile-time evaluation |
| Does not decrease / cannot determine decrease | Compile error |
| Runtime call (not at type position) | No termination check needed |

#### 7.8 Hard Boundary

`i = f(i)` where `f` is irreversible, non-closed, and preserves no monotonicity—mathematically impossible to automatically prove termination. Compile error:

> This loop cannot automatically prove termination. The loop variable depends on the unanalyzable function `f`. Please use an iteration pattern that can be analyzed by the compiler.

This is not a compiler failure. Any code that cannot be statically proven safe shall not pass compilation.

### 8. SMT Solver: Acceleration Module of the Type Checker

SMT solvers are external tools in traditional languages (e.g., F\* calls Z3, Dafny calls Z3). In YaoXiang, it is **an acceleration module of the type checker**—only invoked when the compiler kernel itself cannot directly decide. SMT helps find proofs, but the proofs are verified by the type checker.

**Trust model**: The type checker is the only trust root. The SMT solver is an acceleration module—it helps find proofs, but SMT is not an independent trust boundary. The compiler trusts Z3's `unsat` result (consistent with the F\*/Dafny approach—Z3's error probability is lower than the compiler's own bug rate, a pragmatic engineering choice). The true unreliability control is in the SMT translation layer—if the translation has bugs, the compiler will expose them in other tests.

**Interface**: The compiler internally translates to the SMT-LIB 2.6 standard format, not bound to a specific solver API. SMT-LIB is an ISO standard; Z3, CVC5, MathSAT, Yices all support it natively.

**Default backend**: Z3 (MIT license, broadest documentation and community validation). CVC5 as an SMT-LIB-compatible alternative—users can switch via compiler flags at compile time.

No "universal solver abstraction layer"—SMT-LIB is the abstraction layer. If CVC5 breaks through in specific theories in the future, switching requires only swapping the binary, not changing compiler code.

```
Compile-time Bool expression
        │
        ├── Compiler kernel can directly decide (structural equivalence, simple arithmetic,
        │   trivial formulas after constant folding)
        │   → Return Proved / Disproved directly
        │
        └── Compiler kernel cannot directly decide (quantifiers, symbolic variables)
            → Dependent type pre-reduction (factorial(5) → 120)
            → Translate to SMT-LIB format
            → Send to Z3/CVC5 (with budget limits)
            → Return: unsat → Proved  │  sat + model → Disproved  │  unknown → Unproven
```

**Solver budget—hard limit, like stack depth**:

| Budget dimension | Default | Description |
|----------|--------|------|
| Solve steps | 10,000 | Z3 typically finishes linear arithmetic in hundreds of steps. 10,000 steps covers 99% of practical predicates. |
| Time | 100ms | A single predicate exceeding 100ms = the user is writing a compile-time program, not a type annotation. 100ms × 50 predicates = 5 second compile time upper bound. |
| Quantifier instantiation depth | 3 | Three nested quantifier levels cover practical patterns. Beyond three levels, the user is most likely writing a logic exercise. |

Exceeding budget returns `Unproven`, compile error + predicate location + consumption. No degradation, no runtime check, no silent pass.

**Why this is practical**: 95% of practical predicates in engineering are linear arithmetic—`x > 0`, `arr.len > 0`, `0 <= idx < arr.len`—all within the decidable fragment, and SMT solvers return in milliseconds. Encountering the rare complex predicate exceeding budget, the programmer writes a proof function.

Dependent types have a pre-reduction layer before SMT calls: `factorial(5)` is directly computed at compile time to `120`; `append([1,2], [3])` is directly computed to `[1,2,3]`. These deterministic value computations do not consume SMT budget.

The programmer does not need to know SMT exists. Mental model: **the compiler proves it if it can, reports an error if it cannot—if the compiler doesn't know, you can write a function to prove it for it to see.**

### 9. Compile-Time Predicate Composition

Compile-time predicates are functions returning `Type`; composition is naturally achieved through function composition:

```yaoxiang
SortedNonEmpty: (T: Ord, arr: Array(T)) -> Type = {
    Sorted(T, arr) && NonEmpty(arr)
}
```

### 10. Code Examples

#### 9.1 Safe Division

```yaoxiang
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b

result = divide(10, 2)   # ✅ Compiler verifies Positive(2) = { 2 > 0 } → True
# result = divide(10, 0)  # ❌ Compiler verifies Positive(0) = { 0 > 0 } → False
```

#### 9.2 Safe Array Access

```yaoxiang
InBounds: (idx: Int, arr: Array(T)) -> Type = { 0 <= idx && idx < arr.len }

get: (arr: Array(T), idx: InBounds(idx, arr)) -> T = arr.data[idx]

arr = Array(Int)(1, 2, 3)
x = get(arr, 1)   # ✅ Compiler verifies InBounds(1, arr) = { 0 <= 1 && 1 < 3 } → True
# y = get(arr, 5)  # ❌ Compiler verifies InBounds(5, arr) = { 0 <= 5 && 5 < 3 } → False
```

#### 9.3 Sorting Correctness

```yaoxiang
Sorted: (T: Ord, arr: Array(T)) -> Type = {
    forall i in 0..arr.len-1: arr[i] <= arr[i+1]
}

sort: (T: Ord) -> ((arr: Array(T))) -> (result: Sorted(T, result)) = {
    result = arr.clone()
    # ... sorting algorithm implementation ...
    return result
}
```

#### 9.4 Loops: Compiler VC Generation

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

## Detailed Design

### Syntax Changes

| Before (RFC-022) | After (this RFC) |
|---|---|
| `//! requires: NonEmpty(n) = n > 0` | Compile-time predicate as parameter type `(b: Positive(b))` |
| `//! ensures: ExistsMax(result, arr)` | Return type uses return value parameter `-> (result: IsMax(T, arr, result))` |
| `/*! invariant: ... !*/` | Compile-time predicate type annotation on variable—Floyd-Hoare invariant |
| `//! decreases: n` | Compiler fully automatically derives metric function |
| Specifications are annotations | Specifications are the type system |

### Syntax

**Compile-time predicates have no new keywords.** `{}` is the proof space, exactly matching the existing type definition syntax. A compile-time predicate is a function returning `Type`—`name: (params) -> Type = { assertions }`. Usage is a function call—`Positive(b)`, `IsMax(T, arr, result)`.

```bnf
# Compile-time predicate = function returning Type, with compiler-verified assertions in {}
# Uses existing function/type syntax, no new BNF rules needed
predicate ::= identifier ':' params '->' 'Type' '=' '{' assertions '}'
```

**New syntax concept: return value parameter**—`name` in `-> (name: Type)` is a return value parameter.

The return value parameter is the **only new syntax concept** YaoXiang introduces on top of the existing function syntax. Its semantics:
- `name`'s value is provided by the `return` statement
- `name` only exists in the type signature, referenced by postcondition predicates (e.g., `-> (result: IsMax(T, arr, result))`)
- `name` does not enter the function body scope, does not appear at the call site
- Return value parameter is **optional**—when there is no postcondition, the signature is exactly the same as a regular function (`-> Int`), introducing no extra burden

The reason for introducing it: postconditions need to reference "the value the function is about to return." Without return value parameters, the compiler can only let the predicate reference the return value through special rules (such as implicit variable `$result` or `__retval__`). The return value parameter makes this reference explicit—it is a parameter, just with the value provided by `return` rather than by the caller.

**Proof functions** are not a new concept—they are a YaoXiang function whose return type is the proposition being asserted. When the compiler returns `Unproven`, the programmer provides a proof function, and the type checker verifies it in exactly the same way it verifies any function's return type. No new syntax, no new keywords, no new rules.

### Type System Impact

- **Type universe**: Compile-time predicates reside at the Type₂ layer—functions that accept values and return Type, at the same level as type constructors
- **Generic interaction**: Compile-time predicates can carry generic parameters, e.g., `NonEmpty: (T: Type) -> (arr: Array(T)) -> Type`
- **Ownership interaction**: Expressions in compile-time predicates obey ownership rules; they can only read, not write
- **Type inference**: Compile-time predicate parameters participate in HM type inference

### Runtime Representation

Compile-time predicates are **completely erased** at runtime. `Positive: (x: Int) -> Type = { x > 0 }`—the parameter `b: Positive(b)`'s runtime representation is just `Int`. The refinement condition `{ x > 0 }` is verified by the proof pipeline at compile time, leaving no runtime trace after verification.

- `Positive(5)` → runtime representation is `Int(5)`, refinement condition `{ 5 > 0 }` passed, erased
- `SumUpTo(arr, 0)` → runtime representation is `Int(0)`, equation `0 == sum(arr[0..0])` passed, erased
- Placing compile-time predicates at type positions (e.g., `f(x: Positive(x))`) produces no wrapper type, allocates no extra memory, inserts no runtime check

Generic erasure and refinement type erasure are the same principle: both are compile-time functions that completely disappear after compile-time evaluation. Compile-time predicates have zero runtime overhead in Release Build—this is a direct consequence of "predicates are functions."

**Interaction constraint with `ref`**: Compile-time predicates can only reference immutably borrowed or ownership-transferred values. Compile-time predicates referencing mutably borrowed values cannot guarantee the verification result still holds at runtime at compile time—such usage directly reports a compile error.

### Compiler Changes

1. **Parser**: Compile-time predicates use standard function syntax, no extra parsing rules needed
2. **Compile-time proof pipeline**: Unified `Proved/Disproved/Unproven` return interface, automatic strategy selection
3. **SMT acceleration module**: SMT-LIB 2.6 translation layer, default backend Z3, CVC5 as alternative
4. **Type checker kernel**: Inference rule implementation—structural equivalence, βδι-reduction, universal quantifier introduction/elimination. This is the only trust root; SMT and programmer proofs are both verified through it
5. **Verification condition generation**: WP/SP calculus + loop invariant proof obligations
6. **Error reporting**: Counterexample formatting + unresolved proposition reports + source location correlation

### Backward Compatibility

- ✅ Code not using compile-time predicates is completely unchanged
- ✅ Compile-time predicates have zero runtime overhead in Release Build
- ⚠️ RFC-022's `//!` syntax is no longer supported—but 022 was never implemented, so there is no migration burden

## Trade-offs

### Advantages

- **Curry-Howard isomorphism fully realized**: Types are propositions, programs are proofs, `name: Proposition = Proof`
- **Unity**: Compile-time predicates and regular functions use exactly the same syntax, no conceptual split
- **SMT transparency**: Programmers do not need to know SMT exists; the mental model is consistent with type checking
- **Progressive adoption**: Start with a single compile-time predicate, gradually expand coverage
- **Zero runtime overhead**: All evaluation completes at compile time; Release Build inserts no runtime assertions

### Disadvantages

- **Compile time**: SMT solving increases compile time, but budget hard limits ensure the upper bound is controllable
- **Automatic proof boundary**: Complex predicates beyond first-order linear arithmetic may require programmer-written proof functions. This is not a language defect—this is a necessary conclusion from the halting problem. The compiler honestly reports `Unproven` rather than falsely reporting `True/False`
- **Learning curve**: Writing effective compile-time predicates and proof functions requires understanding the basic intuition of the Curry-Howard isomorphism
- **Implementation complexity**: Unifying the compile-time proof pipeline requires careful design

### Risk Mitigation

- SMT solver budget hard limits (10,000 steps / 100ms time / 3 instantiation depth); exceeding budget returns `Unproven`
- Dependent type pre-reduction: deterministic value computation is consumed first, SMT only tackles the non-deterministic part
- `Unproven` is not a dead end: programmer can write a proof function, type checker verifies—consistent with verifying any function's return type
- Incremental verification: only validate changed modules
- Clear error messages + counterexample display + budget consumption report + unresolved proposition + suggestions (if the compiler can provide them)

## Alternatives

| Approach | Why Not Chosen |
| -------------------------------- | -------------------------------------- |
| RFC-022: `//!` annotation-style specifications | Splits specifications and types, violates Curry-Howard isomorphism |
| Independent specification files (e.g., CVL) | Separates specifications from code, increases maintenance cost |
| Runtime assertions only | Cannot statically guarantee correctness |
| External proof assistants (e.g., Coq) | Disconnected from the compiler, requiring a separate proof language and trust boundary. YaoXiang's choice: proof is YaoXiang code, the type checker is the only trust root |
| **This approach: compile-time predicates as first-class citizens** | ✅ |

## Implementation Strategy

### Phase Division

| Phase | Content |
| ---------- | -------------------------------------------------------------------- |
| **Phase 1** | Compiler kernel: structural equivalence + βδι-reduction + universal quantifier introduction/elimination. Support simple arithmetic predicates (`x > 0`, `arr.len > 0`) |
| **Phase 2** | SMT-LIB translation layer + Z3/CVC5 integration. Pipeline returns `Proved/Disproved/Unproven`. Support programmer-written proof functions when `Unproven` |
| **Phase 3** | Loop invariant VC generation + termination checking (linear ranking function + predicate violation counting + bounded pattern + combinatorial explosion control) |
| **Phase 4** | Incremental verification + caching + IDE support |

### Dependencies

- RFC-010: Unified Type Syntax — compile-time predicates are based on `name: type = value`
- RFC-011: Generic Type System — compile-time predicates can carry generic parameters
- RFC-009: Ownership Model — compile-time predicate expressions obey ownership rules

## Open Questions

- [x] **SMT solver choice**: Default Z3 (MIT license, most widely validated). CVC5 as SMT-LIB-compatible alternative, switchable via compiler flag. The compiler internally translates to the SMT-LIB 2.6 standard format—SMT-LIB is the abstraction layer, no custom universal solver interface.
- [x] **Specific solver budget values**: 10,000 steps / 100ms time / 3 quantifier instantiation depth. Fixed internally in the compiler, no knob. If real use cases prove it insufficient (not "user wrote it wrong") in actual use, adjust then.
- [x] **Quantifier support scope**: No limit on quantifier arity at the language level. Compile-time predicates accept `Type` parameters—`Type` includes function types—therefore higher-order quantifiers are a natural consequence of the type system, requiring no special syntax. SMT solver can automatically decide first-order quantifiers (forall/exists, supports interleaved nesting, limited by budget depth 3). Higher-order quantifiers: SMT returns `Unproven`, compiler prompts "this predicate exceeds the automatic proof scope, please provide a proof function." The programmer writes a YaoXiang function whose return type equals the proposition—the type checker verifies that function. No external export, no AI, no interactive proof mode. Everything is YaoXiang code; everything is verified by the type checker.
- [x] **Counterexample formatting**: Source variable names used directly as SMT variable names (with module prefix to avoid conflicts). Z3 model returns looked up by variable name. Output format: variable name = specific value + source location + predicate definition location. No complex mapping layer.
- [x] ~~**Interaction of compile-time predicates with `ref` smart pointers?**~~ → Decided: compile-time predicates only allow immutably borrowed or ownership-transferred values. Mutably borrowed values cannot appear in compile-time predicates.
- [x] **Extension of forall predicate violation counting metric to non-adjacent operations?** → No extension. Current coverage (adjacent swap, adjacent move) is complementarily covered by Strategy 1 (linear ranking function)—quicksort's outer interval contraction is covered by Strategy 1, heapsort by Strategy 1 (array index pattern). For loops that cannot be proven to terminate by any strategy, the compiler directly reports an error—this is the hard-safety philosophy, not a defect. If real-world scenarios (not academic constructions) arise in the future that none of the four strategies can cover, we will revisit.
- [x] **Combinatorial explosion in linear ranking function enumeration**: Upper limit of candidate enumeration is 3 bounded variables. When ≤3, enumerate all linear combinations and verify one by one via SMT. When >3, only try single-variable metrics (`v_i`, `u_i - v_i`); on failure, directly report a compile error—prompting the programmer "the loop has >3 bounded variables, the compiler cannot automatically synthesize multi-variable metrics." This is not an engineering compromise—it forces the programmer to write simpler loops.

## References

- [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md)
- [RFC-011: Generic Type System Design](../accepted/011-generic-type-system.md)
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
│ (Official)  │    │ (Kept in place) │
└─────────────┘    └─────────────┘
```