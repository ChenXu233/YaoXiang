---
title: 'RFC-027: Compile-Time Predicates and Unified Static Verification'
status: 'Accepted'
author: '晨煦'
created: '2026-06-07'
updated: '2026-07-05'
impl_status: 'in_progress'
impl_detail:
  'Phase 1-2 completed, Phase 3 partially completed, Phase 4 partially completed. Assert/Assert
  unified scheme Phase 6 fully implemented (#157-#162 closed): Never type, IsTrue bridging,
  flow-sensitive Γ + kill set, type-level recursion, universe-layered weak checks, dispatch
  pipeline.'
impl_percent: 85
issue_number: 90
issue_url: 'https://github.com/ChenXu233/YaoXiang/issues/90'

issue: '#90'
---

# RFC-027: Compile-Time Predicates and Unified Static Verification

> **References**:
>
> - [RFC-009: Ownership Model](../accepted/009-ownership-model.md)
> - [RFC-010: Unified Type Syntax - name: type = value Model](../accepted/010-unified-type-syntax.md)
> - [RFC-011: Generic Type System Design](../accepted/011-generic-type-system.md)
> - [RFC-024: Concurrency Model Based on spawn Blocks](../accepted/024-concurrency-model.md)
>
> **Supersedes**: [RFC-022: Hoare Logic Static Verification Support (Specification Comments and Specification Types)](../deprecated/022-hoare-logic-static-verification.md)
> — Deprecated

## Summary

This document proposes introducing **compile-time predicates** as first-class citizens in YaoXiang, unifying all compile-time static verification into a single **proof pipeline**. Compile-time predicates are not external specification comments — they are functions. A function that returns Type, usable in type positions, called by the compiler at compile time, with its return value checked. Types are propositions, compile-time evaluation is proof.

**Core Argument**: The sole job of type checking at compile time is to construct and verify proof terms. Type equalities, token conflicts, dependent type reduction, compile-time predicate evaluation, Hoare logic entailment — all are different forms of type checking within the compile-time proof pipeline, sharing the same pipeline. The SMT solver is a speed module for the type checker, not an independent trust boundary. When the compiler returns Unproven, the programmer writes a YaoXiang function as proof — the type checker verifies it exactly as it verifies any function's return type. Everything is YaoXiang code, everything verified by the type checker.

## Motivation

### Why Deprecate RFC-022?

RFC-022 designed specifications as `//!` comment forms:

```yaoxiang
max: (T: Ord) -> ((arr: Array(T, n)) -> T) = {
    //! requires: NonEmpty(n) = n > 0          ← This is a type-independent comment
    //! ensures: ExistsMax(result, arr[0..n])   ← This is a type-independent comment
}
```

This commits the fundamental error of Curry-Howard isomorphism: **splitting specifications and types into two layers**. Comments are not types. Comments do not participate in type checking. Comments are the mental model of an "external tool".

The whitepaper is clear:

> "No `//!` comments. No separate specification language. Everything is within the type system."

### Current Problems

- RFC-022's `//!` comments are external syntax separate from the type system
- Specification types and ordinary types are two separate systems, creating conceptual redundancy
- The split pattern of Debug Build verification / Release Build ignore breaks unity
- SMT solvers are traditionally positioned as external tools — YaoXiang integrates them as speed modules for the type checker
- Type checking, borrow verification, compile-time predicate checking, and macro expansion each take different paths

### The Correct Mental Model

Type checking can be abstracted as a function:

```
verify : Program → Proved | Disproved(Model) | Unproven
```

All compile-time checks — simple type matching, borrow conflict detection, compile-time predicate verification — are sub-tasks of this function. They share the same proof pipeline, differing only in proof term complexity and construction strategies.

When the compiler returns Unproven, the programmer provides a proof function — the function's return type equals the proposition to be proved. The type checker verifies it. This is the same operation as ordinary type checking.

## Proposal

### 1. `{}` Is the Proof Space: Types Are Assertions, Verification Is Type Checking

YaoXiang's `{}`
is the compile-time proof space. Everything inside is an assertion, guaranteed by the compiler to be True — either automatically proven or provided by the programmer as a proof function.

```
Point: Type = { x: Float, y: Float }
#               ^^^^^^^^^^^^^^^^^^^^^  The compiler guarantees x is Float, y is Float

List: (T: Type) -> Type = { data: Array(T) }
#                           ^^^^^^^^^^^^^^^  The compiler guarantees data is Array(T)
```

**Generics are a special case of compile-time predicates.**

```yaoxiang
Positive: (x: Int) -> Type = { x > 0 }
#          ^^^^^^              ^^^^^^
#          Parameter in signature    {} contains only assertions
#          Compiler verifies x > 0 when called at compile time

List: (T: Type) -> Type = { data: Array(T) }
#      ^^^^^^^^              ^^^^^^^^^^^^^^^
#      Parameter in signature    Compiler verifies type_of(T) == Type, type_of(data) == Array(T)
```

The same pattern: `name: (params) -> Type = { 断言 }`. The compiler does not distinguish between "type assertions" and "value assertions" — both are evaluation targets in the proof pipeline.

**Loop invariants don't need separate writing. Type annotations on variables are Floyd-Hoare invariants.**

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # Annotation references i — tells the compiler s's type depends on i
    mut i: UpTo(arr.len) = 0     # At initialization i=0, verify: 0 == sum(arr[0..0]) → True
    while i < arr.len {
        s += arr[i]  # Compiler verifies: s_new == sum(arr[0..i+1])
        i += 1       # i changed → triggers s dependency re-verification: s satisfies SumUpTo(arr, i_new)
    }
    return s  # s: SumUpTo(arr, arr.len) = sum(arr[0..arr.len])
}
```

The compiler generates verification conditions once for the loop body — inductive hypothesis (type annotation) → assignment operation → whether the new value satisfies the type annotation. Once the proof pipeline verifies the inductive step, all iterations are automatically covered. No
`: decreases`, no `: Invariant`, no inductive proof needed — the compiler decomposes induction into local VCs for each assignment.

### 2. Pre/Postconditions: Compile-Time Predicates on Parameter Types and Return Types

Abandon RFC-022's `//! requires`/`//! ensures`. Compile-time predicates are used as type annotations on parameters or return types.

**Parameter side is a function call.** A compile-time predicate is a function returning Type, and using it on the parameter side is calling it — just like `factorial(5)`. The return type side introduces a new concept: return value parameter.

```yaoxiang
# Precondition: Explicitly call compile-time predicate in parameter type
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b
#                       ^^^^^^^^^^  b is the current formal parameter name, passed to Positive as argument
#                       Compiler extracts actual argument value at call site, substitutes b, verifies Positive(actual)
#                       Example: divide(10, 2) → verify Positive(2) = { 2 > 0 } → True
#                       Example: divide(10, 0) → verify Positive(0) = { 0 > 0 } → False → compile error

# Postcondition: Return value parameter + compile-time predicate
IsMax: (T: Ord, arr: Array(T), result: T) -> Type = {
    forall j in 0..arr.len: result >= arr[j]
}

NonEmpty: (arr: Array(T)) -> Type = { arr.len > 0 }

max: (T: Ord) -> ((arr: NonEmpty(arr))) -> (result: IsMax(T, arr, result)) = {
#                                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
#                                            result is the return value parameter, value provided by return
#                                            Compiler at return point substitutes return value, verifies postcondition
    candidate = arr[0]
    for i in 1..arr.len {
        if arr[i] > candidate { candidate = arr[i] }
    }
    return candidate
}
```

**Key Rules**:

- **Parameter side**: `b: Positive(b)` — `b` is the current formal parameter name, passed to `Positive` as argument. Function call syntax, zero implicitness.
- **Return side**: `-> (result: IsMax(T, arr, result))` — `result` is the return value parameter, value provided by `return` statement. `result` only exists in the type signature, only referenced by predicates, does not enter function body scope, does not appear at call sites.
- **Return value parameter optional**: Omit when no postcondition, signature identical to ordinary function (`-> Int`).
- **Unity**: Parameter and return value parameters are the same concept — `param_name: predicate_call(param_name)`, differing only in whether the value is provided by the caller or by `return`.

### 3. Path Condition Propagation: Compile-Time Verification of Runtime Values

When compile-time predicates are used at binding positions, parameters are explicitly passed by the programmer. When runtime values enter refinement type parameters, the compiler completes verification through path condition collection and SMT entailment checking — no explicit proof passing required from the programmer.

#### 3.1 Explicit Function Calls

When compile-time predicates are used at binding positions, parameters are explicitly passed by the programmer — just function calls, zero implicitness.

`Positive: (x: Int) -> Type = { x > 0 }`
is a compile-time predicate constructor. When it appears at a binding position (parameter declaration, variable declaration, return type), the programmer explicitly passes the bound variable name:

```yaoxiang
b: Positive(b)
// b has already been declared as current formal parameter, Positive(b) is a function call
// After normalization: b: { b > 0 }
```

No need for the compiler to implicitly fill in the parameter — `b: Positive(b)` is just a function call, same as `f(5)`. `b`
is bound as a parameter name, and its type annotation `Positive(b)` references `b`
itself — this is the standard pattern of dependent types, not an implicit expansion rule.

**Unification with RFC-010 `self`**: RFC-010 establishes `self` is not a keyword, just a convention for parameter names ("writing `p`, `this`, or `x` has identical effect"). `b: Positive(b)`
shares the same mechanism — parameter names can be referenced in type annotations. `self` appears in `self: Point`, `b` appears in
`b: Positive(b)`. Both type annotations reference the parameter itself. The only difference is the complexity of the type annotation; the mechanism is identical — after name binding, the type can depend on that name.

Return types also use explicit function calls:

```yaoxiang
Sorted: (arr: Array(T)) -> Type = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }

sort: (arr: Array(T)) -> (result: Sorted(result)) = { ... }
//                        ^^^^^^^^^^^^^^^^^^^^^^^
//                        result is the return value parameter, Sorted(result) is a function call
//                        Compiler at return point substitutes return value for result, verifies Sorted(return value)
```

Equally applicable to local variable declarations:

```yaoxiang
let x: Positive(x) = 5
// x bound as 5, Positive(5) → { 5 > 0 } → True → pass

// let y: Positive(y) = 0
// y bound as 0, Positive(0) → { 0 > 0 } → False → compile error
```

#### 3.2 Path Condition Collection

When runtime values appear in conditional branches, the compiler automatically collects path conditions, forming a **assumption set** for the current scope. These assumptions participate in verification as background knowledge for compile-time Bool evaluation.

```yaoxiang
if y > 0 {
    // Compiler automatically gains assumption in this branch: { y > 0 }
    let result = divide(x, y)
    // Verification condition: (y > 0) ⇒ (y > 0)
    // Proof pipeline determines entailment holds → Proved
} else {
    // This branch assumption: { !(y > 0) }
    // If calling divide(x, y), verification condition is !(y > 0) ⇒ y > 0
    // Proof pipeline determines non-entailment → Disproved
}
```

This is not the compiler hardcoding special patterns — this is the natural behavior of the compile-time proof pipeline. At each type-checking call site, the pipeline receives:

```
{background assumptions} ⇒ {verification goal}
```

The proof pipeline determines entailment. Proved → pass, Disproved → compile error + counterexample, Unproven
→ compile error + unsolved proposition. Background assumptions come from path conditions at the current program point.

#### 3.3 Assumption Stack

When analyzing control flow, the compiler maintains an assumption set for each basic block:

- **if-guard**: `if y > 0` → true branch pushes `y > 0`, false branch pushes `!(y > 0)` (if else is used)
- **match pattern**: `if let Some(v) = opt` → branch pushes `opt == Some(v)`
- **logical conjunction**: `if x > 0 && y < 10` → branch pushes `x > 0` and `y < 10`
- **function preconditions**: When calling `divide(a, b)`, evidence that `b` satisfies `Positive`
  must come from either current assumptions or the actual argument's own refinement type annotation (if `b` is already annotated as `Positive`, its type carries `b > 0`)
- **assignment**: `let z = y` transfers refinement conditions from `y` to `z`

All assumptions enter the compile-time proof pipeline. When entering the SMT accelerated path, they are translated as SMT-LIB background assertions.

#### 3.4 No Static Evidence Means Compile Error

If the programmer directly writes:

```yaoxiang
divide_user_input: (x: Int, y: Int) -> Int = divide(x, y)
```

The current program point has no assumption of `y > 0`, and the actual argument `y` itself has no `Positive` type annotation. The verification condition is:

```
{} ⇒ { y > 0 }
```

Pipeline returns `Disproved` (non-entailment) → compile error:

> Cannot prove that parameter `b` satisfies `Positive` in `divide` call. `y`
> comes from function input with no proven bounds. Consider calling with if-branch guard: `if y > 0 { divide(x, y) }`.

YaoXiang does not accept runtime values directly entering refinement type parameters without providing static evidence. This is not a limitation — this is the core of the hard safety philosophy. Code that the compiler cannot statically prove correct must not compile.

#### 3.5 Relationship with the Unified Pipeline

Path condition propagation is not an additional mechanism. It is the direct extension of the compile-time proof pipeline to control flow analysis:

| Stage               | Responsibility                                                                                         |
| ------------------- | ------------------------------------------------------------------------------------------------------ |
| Path condition collection | Compiler control flow analysis stage, annotates assumption set for each basic block              |
| VC generation       | When encountering type constraints to verify, merge path conditions + actual argument type info        |
| Proof pipeline evaluation | Compiler kernel → SMT acceleration → produces Proved / Disproved / Unproven                          |
| Result              | `Proved` → pass; `Disproved` → compile error + counterexample; `Unproven` → compile error + unsolved proposition (programmer can provide proof function) |

No new parts. No special rules. Path conditions are background knowledge for the proof pipeline — sharing the same pipeline and budget system with type equalities and borrow constraints.

### 4. Compile-Time Proof Pipeline

All compile-time checks share the same pipeline. The core operation of the pipeline is **type checking** — checking whether a proof term's type equals the proposition to be proved. Everything is type checking.

```
Compile-time encounters Bool expression requiring evaluation (i.e., need to construct a proof term)
        │
        ├── Type equality (T1 == T2)
        │   → Compiler decides directly (structural equivalence)
        │
        ├── Token conflict conditions (!conflicting(tokens))
        │   → Flow-sensitive liveness analysis (Dup/Linear attribute tracking)
        │
        ├── Dependent type reduction (n + m simplification)
        │   → Compile-time term rewriting system (βδι-reduction)
        │
        ├── Compile-time predicates (x > 0, forall...)
        │   → Compiler itself + SMT acceleration module
        │
        └── Hoare logic entailment (P ⇒ Q)
            → Compiler + SMT acceleration module
                    │
                    ▼
             ┌──────────┐
             │ Proved   │  → Compile passes
             │ Disproved│  → Compile error + counterexample
             │ Unproven │  → Compile error + unsolved proposition
             └────┬─────┘
                  │
                  ▼
         Programmer writes proof function (YaoXiang code)
                  │
                  ▼
         Type checker verifies ──→ Proved ──→ Compile passes
                  │
                  ▼
            Verification failed → Compile error: "Proof does not hold"
```

#### 4.1 Proof Results: Three-Value Algebra

Compile-time evaluation returns three types of results — this is the inevitable conclusion of the halting problem, and also the natural partition of proof theory:

```
eval_compile_time : BoolExpr → Proved | Disproved(Model) | Unproven
```

- **Proved** → Halts, proof term constructed, type check passes. Compile continues.
- **Disproved(M)** → Halts, counterexample M exists. Compile error + counterexample + source location.
- **Unproven** → Proof not constructed within given resource limit. Compile error + unsolved proposition + budget consumption report.

**Unproven ≠ False.**
When the compiler says "I cannot prove it", that is not equivalent to the proposition being false — it just exceeds current automatic proving capability. This is honesty, not a defect.

Budget hard limits are the engineering solution to the halting problem. No knobs — giving knobs asks users "do you think your program will halt", and neither users nor compilers know.

#### 4.2 After Unproven: Programmer Writes Proof

When the compiler returns Unproven, the programmer can write a **proof function** — a YaoXiang function whose return type equals the proposition to be proved. The type checker verifies this function — the same mechanism as it verifies
`add(a, b): Int`.

```
Proposition  = Type
Proof        = Program (a value of that type)
Verification = Type checking (the sole trust root)
```

The SMT solver is not an independent trust boundary — it is a **speed module for the type checker**. SMT helps find proofs, but the type checker is always the one verifying proofs. When SMT returns `unsat`,
the compiler reconstructs this result as a proof term verifiable by the type checker. If reconstruction fails (SMT's reasoning steps exceed the compiler kernel's inference rules), it falls back to Unproven — the programmer can manually write a proof function.

```yaoxiang
# Proposition: refined property the compiler cannot automatically prove
FirstIsMin: (T: Ord, arr: Sorted(T)) -> Type = {
    forall i in 0..arr.len: arr[0] <= arr[i]
}

# Proof: programmer writes a function whose return type is the above proposition
# Type checker verifies this function — identical to verifying add(a,b): Int
first_is_min: (T: Ord, arr: Sorted(T)) -> FirstIsMin(T, arr) = {
    # Compiler here verifies: function body type = FirstIsMin(T, arr)
    ...
}
```

No AI needed, no export to Coq, no new concepts. **Properties the compiler cannot automatically prove at compile time → programmer writes proof in YaoXiang code → type checker verifies.**
The whole process is a smooth gradient — the compiler handles simple proofs for you, saving your mind for the hard ones.

#### 4.3 Layered Dependencies Within the Pipeline

The above evaluators share the same interface but have evaluation order. Type equality is a prerequisite for all subsequent analysis; ownership/token checking depends on type information; refined predicate verification depends on the results of the first two layers. The compiler evaluates layer by layer — expressions failing at a lower layer do not enter higher layers — avoiding wasting solving budget on programs with type errors.

```
Evaluation order (same pipeline, layered scheduling)
├── Layer 0: Type equality (T1 == T2)
│   └── Structural unification → failure means subsequent analysis is meaningless, return Disproved directly
├── Layer 1: Ownership/token conflicts
│   └── Flow-sensitive liveness analysis → failure means memory safety does not hold, return Disproved directly
└── Layer 2: Refined predicates/Hoare entailment
    └── Compiler itself → SMT acceleration → produces Proved / Disproved / Unproven
```

Each layer still returns `Proved/Disproved/Unproven`, sharing the same interface and budget system.

### 5. Three-Layer Function Unification

| Layer           | Runtime     | Input    | Output | Example                                           |
| -------------- | ----------- | -------- | ------ | ------------------------------------------------- |
| Value-level function | runtime | value    | value  | `add: (a: Int, b: Int) -> Int = a + b`             |
| Type constructor   | compile-time | type/value | Type   | `List: (T: Type) -> Type = { data: Array(T) }`    |
| Compile-time predicate | compile-time | value    | Type   | `Positive: (x: Int) -> Type = { x > 0 }`          |

All use identical `name: type = value` syntax. Compile-time predicates and type constructors go through the same compile-time proof pipeline — `{}`
is the proof space.

### 6. Loops: Floyd-Hoare Verification Condition Generation

Loops need no separate `: Invariant(...)` or `: decreases(...)`
annotations. Compile-time predicate type annotations on variables define Floyd-Hoare-style assertions — the compiler generates verification conditions from type annotations, and the proof pipeline checks whether each assignment maintains the type.

Core mechanism: Each assignment operation corresponds to a Hoare triple `{P} x := e {Q}`, and the verification condition is
`P ⇒ Q[e/x]`. The compiler generates verification conditions once for the loop body — once the proof pipeline verifies the inductive step, all iterations are automatically covered.

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # Annotation references i; at initialization i=0, verify: 0 == sum(arr[0..0]) → True
    mut i: UpTo(arr.len) = 0     # Verify: 0 <= 0 <= arr.len → True
    while i < arr.len {
        # Compiler generates VC once for loop body. Premise: s satisfies SumUpTo(arr, i), i satisfies UpTo(arr.len).
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
        #   i changed → in dependency graph, s's type annotation references i → triggers re-verification
        #   New verification goal: s satisfies SumUpTo(arr, i_new)
        #   i.e., s == sum(arr[0..i_new]), guaranteed by previous step → Proved
        s += arr[i]
        i += 1
    }
    return s  # At this point s: SumUpTo(arr, arr.len), i.e., s == sum(arr[0..arr.len])
}
```

Loop invariants are type annotations on variables — the programmer writes types, the compiler checks the inductive step. The compiler does not need to "discover" invariants or "automatically perform induction" — it decomposes the inductive proof into local verification conditions for each assignment operation, delegating to the proof pipeline for divide-and-conquer.

#### 6.1 Dependency Tracking: Dependent Types on Mutable Variables

The prerequisite for the above mechanism is: the compiler knows that `s`'s type annotation `SumUpTo(arr, i)` references `i` — when `i` changes, `s`'s type constraint also changes. This requires the compiler to maintain a **type dependency graph between variables**.

**Data structure**:

```
TypeDepGraph: Map<VarName, Set<VarName>>
# Key is the depended-upon variable, value is the set of variables whose type annotations reference it
# Example: { i: {s}, j: {s, t}, ... }
```

**Construction**: When the type checker processes `mut v: Pred(... x ...) = init`, it parses free variable references in `Pred(...)` parameters. If other mutable variables `x` in the current scope are referenced in the parameters, record `x → v` in the dependency graph.

**Trigger**: When a depended-upon variable `x` is assigned, the compiler:

1. Looks up all variables depending on `x` in the dependency graph: `{v₁, v₂, ...}`
2. For each `v`, generates verification condition: `does v's current value satisfy the updated type Pred(... x_new ...)`
3. Sends VCs to the proof pipeline

**Assignment order sensitivity**: Dependency tracking naturally enforces correct assignment order. Using `SumUpTo(arr, i)` as example:

```yaoxiang
# Correct order
s += arr[i]   # s_new satisfies SumUpTo(arr, i+1)
i += 1        # i changed → re-verify s satisfies SumUpTo(arr, i_new) → True

# Wrong order — compiler rejects
i += 1        # i changed → re-verify s satisfies SumUpTo(arr, i_new)
              # s not yet updated, s_old == sum(arr[0..i_old]) ≠ sum(arr[0..i_new])
              # → Compile error: variable s does not satisfy type SumUpTo(arr, i_new)
s += arr[i]   # Unreachable
```

**Composite dependencies**: A variable can depend on multiple variables. Type annotation `{ v: Int; v == x + y }` depends on both `x` and `y` — any change triggers re-verification.

**Relationship with the proof pipeline**: Dependency tracking is the trigger for VC generation, not an independent verification mechanism. It answers "when to generate VCs" — the proof pipeline answers "do the VCs hold".

### 7. Termination Checking

Fully automatic at compile time. Loops the compiler can prove terminate pass; loops it cannot prove terminate report compile error directly — programmers must make the compiler able to automatically analyze loop termination. No semi-automatic annotation escape routes.

#### 6.1 Design Principles

The compiler automatically extracts information needed for termination proofs from two sources:

1. **Variable type annotations**: Boundary constraints in refined types (e.g., `UpTo(n)` gives upper bound `n` and lower bound `0`)
2. **Loop body operations**: Operations applied to variables each iteration

The compiler tries four metric synthesis strategies in priority order, stopping when one succeeds.

#### 6.2 Strategy 1: Automatic Linear Ranking Function Synthesis

When variables have linear bound annotations, the compiler enumerates candidate linear measures and verifies with SMT.

```
Input:
  Variables v₁: UpTo(u₁), v₂: UpTo(u₂), ... (variables with upper and lower bounds)
  Loop condition cond
  Assignment set in loop body

Algorithm:
  1. Extract bounds for each variable from type annotations: [low_i, high_i]
  2. Enumerate candidate measures: v_i, u_i - v_i, v_i - v_j, and other linear combinations
  3. For each candidate measure m:
     - SMT verify m ≥ 0 (deduced from type bounds)
     - For each execution path in loop body, SMT verify m' < m (strictly decreasing)
  4. Find conforming linear combination → termination proved
```

Coverage: loops where arbitrary variables are assigned linear expressions (`v = a·v + b`) and have bounded type annotations. Includes
`i += const`, `i -= const`, and interval shrinking in binary search style:

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

#### 6.3 Strategy 2: Predicate Violation Count — Automatic Metric Extraction from Target Types <span style="color:orange">【Experimental Strategy】</span>

> ⚠️ **Current status: Experimental strategy, inclusion decided based on actual feasibility during Phase 3 implementation.**
> This strategy works for adjacent swap operations (bubble sort, insertion sort), but cannot automatically prove non-adjacent operations (quicksort partition, heapsort sift-down). See coverage table below. If Phase 3 verification proves infeasible, this strategy will be removed or demoted to future work.

Core insight: **User-written specifications are material for compiler reasoning.** The compiler does not need built-in knowledge of "what sorting is" — it reads the `Sorted`
definition and automatically extracts measures from it.

```
Input:
  Target type: Sorted(arr) = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }
  Loop body operations: adjacent element swaps

Algorithm:
  1. Parse predicate definition: forall i in range: cond(i, arr)
  2. Automatically generate measure: violation_count = |{ i | ¬cond(i, arr) }|
  3. Analyze operation's effect on measure:
     - Adjacent swap arr[j], arr[j+1] = arr[j+1], arr[j]
     - Only affects index pairs j-1, j, j+1
     - If arr[j] > arr[j+1] (violates predicate), swap makes this pair satisfy predicate
     - violation_count decreases by at least 1
  4. Upper bound: n·(n-1)/2 (max adjacent inversions), lower bound: 0
  → termination proved
```

**Current coverage scope**:

| Algorithm  | Operation pattern   | Strategy 2 provable? | Reason                              |
| ---------- | ------------------- | :-----------------: | ----------------------------------- |
| Bubble sort | adjacent swap       |         ✅          | violation_count strictly decreases on each swap |
| Insertion sort | adjacent move    |         ✅          | Each shift eliminates one violation |
| Selection sort | non-adjacent swap |         ❌          | Single swap may increase violation_count |
| Quicksort  | partition           |         ❌          | Non-adjacent swaps, no monotonic decrease |
| Heapsort   | sift-down           |         ❌          | Tree operations, violation_count non-monotonic |

**Complementary strategies**: For quicksort, `low < high`
interval shrinking is covered by Strategy 1 (linear ranking function) — outer partition recursion, halving interval each time. Strategy 1 and Strategy 2 complement each other's coverage; most real algorithms' termination can be proved by one of the two. But generalizing Strategy 2 (non-adjacent operations, tree operations) remains an open problem.

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

#### 6.4 Strategy 3: Bounded Increment/Decrement Pattern

`v += const` (positive constant), variable has upper bound type annotation → measure `upper_bound - v` decreases by
`const` each time, lower bound 0. This is a degenerate case of Strategy 1, handled quickly at the front.

#### 6.5 Strategy 4: Multiplicative Scaling Metric Template

`v *= const` (const > 1), variable has upper and lower bound type annotations. Compiler has built-in logarithmic measure template
`ceil(log_const(upper/v))`, measure decreases by 1 each multiply by const.

```yaoxiang
mut i: Positive(i) = 1
while i < n {
    # Compiler automatically derives: measure ceil(log₂(n/i)), measure decreases by 1 each multiply by 2
    i *= 2
}
```

#### 6.6 Separating Termination from Correctness

Termination proof and correctness proof are independent:

- **Termination**: The four strategies above automatically prove the loop exits in finite steps
- **Correctness**: Whether the loop body advances toward the target type is checked by the compile-time proof pipeline via verification conditions

Both pass → compile passes. Termination proved but correctness fails → compile error + counterexample. Correctness proved but termination cannot be proved → compile error pointing out unanalyzable variables or operations. Both fail → compile error reporting both failures separately.

#### 6.7 Termination Checking for Recursive Functions

For recursive functions needing compile-time evaluation, the compiler checks for parameter decreasing:

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)  # Compiler analyzes: n-1 < n → decreasing → terminates
}

# Compile-time usage — compiler guarantees factorial terminates at compile time
vec: Vec(factorial(5)) = Vec(120)()  # 5! = 120, completed at compile time
```

| Scenario                                     | Behavior                  |
| -------------------------------------------- | ------------------------- |
| Compiler can analyze recursive decrease (e.g., `n-1`) | Compile-time evaluation   |
| Not decreasing / cannot determine decrease   | Compile error             |
| Runtime call (not in type position)          | No termination check needed |

#### 6.8 Hard Boundary

`i = f(i)` where `f` is non-invertible, not bounded, does not maintain any monotonicity — mathematically impossible to automatically prove termination. Compile error:

> This loop cannot be automatically proven to terminate. Loop variable depends on unanalyzable function `f`. Please use an iteration pattern analyzable by the compiler.

This is not the compiler's failure. Code that cannot be statically proven safe must not compile.

### 8. SMT Solver: Speed Module for the Type Checker

In traditional languages, the SMT solver is an external tool (like F\* calling Z3, Dafny calling Z3). In YaoXiang, it is a **speed module of the type checker** — only called when the compiler kernel itself cannot decide directly. SMT helps find proofs, but the type checker verifies proofs.

**Trust model**: The type checker is the sole trust root. The SMT solver is a speed module — it helps find proofs, but SMT is not an independent trust boundary. The compiler trusts Z3's `unsat`
results (consistent with F\*/Dafny approach — probability of Z3 error is lower than probability of the compiler's own bugs, a pragmatic engineering choice). The real unreliability control is in the SMT translation layer — if the translation has bugs, the compiler will expose them in other tests.

**Interface**: The compiler internally translates to SMT-LIB
2.6 standard format, rather than binding to a specific solver API. SMT-LIB is an ISO standard, natively supported by Z3, CVC5, MathSAT, and Yices.

**Default backend**: Z3 (MIT license, most extensive documentation and community verification). CVC5 as an SMT-LIB-compatible alternative — users can switch via compiler flags at compile time.

No "general-purpose solver abstraction layer" — SMT-LIB is the abstraction layer. If CVC5 has breakthroughs in specific theories in the future, switching only requires changing the binary, not compiler code.

```
Compile-time Bool expression
        │
        ├── Compiler kernel can decide directly (structural equivalence, simple arithmetic,
        │   trivial formulas after constant folding)
        │   → Return Proved / Disproved directly
        │
        └── Compiler kernel cannot decide directly (quantifiers, symbolic variables)
            → Dependent type pre-reduction (factorial(5) → 120)
            → Translate to SMT-LIB format
            → Send to Z3/CVC5 (with budget limit)
            → Return value: unsat → Proved  │  sat + model → Disproved  │  unknown → Unproven
```

**Solving budget — hard limit, like stack depth**:

| Budget dimension        | Default value | Description                                                                                           |
| ---------------------- | ------------- | ------------------------------------------------------------------------------------------------------ |
| Solver steps           | 10,000        | Z3 typically solves linear arithmetic in under 100 steps. 10,000 covers 99% of practical predicates.   |
| Time                   | 100ms         | Single predicate > 100ms = user is writing a compile-time program, not a type annotation. 100ms × 50 predicates = 5 second compile time cap. |
| Quantifier instantiation depth | 3      | Three-level nested quantifiers cover practical patterns. Beyond three levels is likely writing logic exercises. |

Over budget returns Unproven, compile error + predicate location + consumption amount. No degradation, no runtime checks, no silent pass.

**Why this is practically feasible**: In practice, 95% of actual predicates are linear arithmetic — `x > 0`, `arr.len > 0`, `0 <= idx < arr.len` — all in the decidable fragment, SMT solvers return in milliseconds for these problems. For the rare complex predicates that exceed budget, programmers write proof functions.

Dependent types undergo a layer of pre-reduction before SMT calls: `factorial(5)` directly evaluates to `120` at compile time, `append([1,2], [3])`
directly evaluates to `[1,2,3]`. These deterministic value computations do not consume SMT budget.

Programmers do not need to know SMT exists. Mental model: **if the compiler can prove it, it passes; if it cannot, it errors — and if the compiler cannot, you can write a function to prove it**.

### 9. Compile-Time Predicate Composition

Compile-time predicates are functions returning Type, composition is naturally achieved via function composition:

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
// result = divide(10, 0)  # ❌ Compiler verifies Positive(0) = { 0 > 0 } → False
```

#### 9.2 Array Access Safety

```yaoxiang
InBounds: (idx: Int, arr: Array(T)) -> Type = { 0 <= idx && idx < arr.len }

get: (arr: Array(T), idx: InBounds(idx, arr)) -> T = arr.data[idx]

arr = Array(Int)(1, 2, 3)
x = get(arr, 1)   # ✅ Compiler verifies InBounds(1, arr) = { 0 <= 1 && 1 < 3 } → True
// y = get(arr, 5)  # ❌ Compiler verifies InBounds(5, arr) = { 0 <= 5 && 5 < 3 } → False
```

#### 9.3 Sorting Correctness

```yaoxiang
Sorted: (T: Ord, arr: Array(T)) -> Type = {
    forall i in 0..arr.len-1: arr[i] <= arr[i+1]
}

sort: (T: Ord) -> ((arr: Array(T))) -> (result: Sorted(T, result)) = {
    result = arr.clone()
    // ... sorting algorithm implementation ...
    return result
}
```

#### 9.4 Loop: Compiler VC Generation

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

### 11. Dispatch Pipeline: Unified Compile-Time and Runtime Dispatch

`assert` and `Assert` are two faces of the same refinement type primitive. The dispatch pipeline `dispatch`
automatically decides whether to go through compile-time proof or runtime check based on **whether the free variables of the predicate are accessible at compile time**:

| Criterion                                                  | Mode           | Behavior                                                                                  |
| ---------------------------------------------------------- | -------------- | ---------------------------------------------------------------------------------------- |
| All free variables known at compile time (generic parameters, compile-time constants) | **CompileTime** | Enter proof pipeline: Proved → erase, Disproved → compile error, Unknown → proof required |
| Free variables from runtime exist (function parameters, external input, mut variables) | **Runtime**    | Insert runtime check, inject refinement facts into flow-sensitive assumption set Γ        |

**Key point**: "Cannot decide" ≠ "Disproved". In CompileTime mode, Unknown requires proof (no silent degradation), and in Runtime mode the proposition has no compile-time truth value at all — even an omnipotent prover cannot write a tautology proof for "the user might have entered a negative number"; runtime check is the only sound choice. This is not the prover being insufficiently powerful; it is theoretical necessity.

### 12. Flow-Sensitive Assumption Set Γ: Strongest Postcondition Propagation

The compiler maintains a flow-sensitive (flow-sensitive) assumption set Γ, tracking propositions known to hold at each control flow point.

**SP (Strongest Postcondition) propagation**:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
```

**mut variable kill set**: After a `mut` variable is reassigned, all assumptions involving that variable are removed from Γ:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
mut x = x - 5       // Γ = {}  ← x > 0 killed
```

This is a hard requirement for soundness — when variable values change, old assumptions become invalid.

**Branch merging**: When IF/ELSE or match branches merge, Γ takes the intersection of each branch's assumptions. Only propositions holding on all paths exit the branch.

### 13. Erasure Model Clarification: Witness Erasure ≠ Check Erasure

RFC-027's assertion that "refinement types are **completely erased** at runtime" refers to **proof
witnesses** (proof tokens) — proof items verified at compile time do not generate runtime code. But runtime checks inserted by dispatch in Runtime mode are **preserved** — they are Bool checks executed at the value level, not type-level witnesses.

Summary: erase witnesses, retain checks. The two are not conflicting, RFC-027's original assertion stands.

## Detailed Design

### Syntax Changes

| Before (RFC-022)                     | After (this RFC)                                              |
| ------------------------------------ | ------------------------------------------------------------- |
| `//! requires: NonEmpty(n) = n > 0`  | Compile-time predicate as parameter type `(b: Positive(b))`  |
| `//! ensures: ExistsMax(result, arr)` | Return type uses return value parameter `-> (result: IsMax(T, arr, result))` |
| `/*! invariant: ... !*/`             | Compile-time predicate type annotations on variables — Floyd-Hoare invariants |
| `//! decreases: n`                   | Compiler automatically derives metric function               |
| Specifications are comments          | Specifications are types                                    |

### Syntax

**Compile-time predicates introduce no new keywords.** `{}`
is the proof space, completely consistent with existing type definition syntax. A compile-time predicate is a function returning Type — `name: (params) -> Type = { assertions }`. When used, it is a function call — `Positive(b)`, `IsMax(T, arr, result)`.

```bnf
# Compile-time predicate = function returning Type, assertions inside {} are verified by compiler
# Uses existing function/type syntax, no new BNF rules needed
predicate ::= identifier ':' params '->' 'Type' '=' '{' assertions '}'
```

**New syntactic concept: return value parameter** — in `-> (name: Type)`, `name` is the return value parameter.

The return value parameter is the **only syntactic concept** YaoXiang introduces on top of existing function syntax. Its semantics:

- The value of `name` is provided by the `return` statement
- `name` only exists in the type signature, referenced by postcondition predicates (e.g., `-> (result: IsMax(T, arr, result))`)
- `name` does not enter function body scope, does not appear at call sites
- Return value parameter **optional** — omit when no postcondition, signature identical to ordinary function (`-> Int`), no additional burden

Reason for introduction: postconditions need to reference "the value the function will return". Without return value parameters, the compiler can only use special rules (like implicit variables `$result` or `__retval__`) to let predicates reference the return value. Return value parameters make this reference explicit — it's just a formal parameter, with the value provided by `return` rather than by the caller.

**Proof functions** are not a new concept — they are YaoXiang functions whose return type is the asserted proposition. When the compiler returns Unproven, the programmer provides the proof function, and the type checker verifies it exactly as it verifies any function's return type. No new syntax, keywords, or rules needed.

### Type System Impact

- **Type universe**: Compile-time predicates sit at Type₂ level — functions accepting values and returning Type, same level as type constructors
- **Generic interaction**: Compile-time predicates can carry generic parameters, e.g., `NonEmpty: (T: Type) -> (arr: Array(T)) -> Type`
- **Ownership interaction**: Expressions in compile-time predicates follow ownership rules — read-only, no writes allowed
- **Type inference**: Parameters of compile-time predicates participate in HM type inference

### Runtime Representation

Compile-time predicates are handled at runtime according to dispatch results:

- **CompileTime mode** (all free variables known at compile time): After proof passes, witness tokens are completely erased. `Positive: (x: Int) -> Type = { x > 0 }` — parameter
  `b: Positive(5)` at runtime is represented as `Int`. The refinement condition `{ 5 > 0 }` has passed, erased.
- **Runtime mode** (runtime free variables exist): Runtime checks preserved — Bool checks executed at value level, injected into flow-sensitive assumption set Γ. See §11
  dispatch pipeline and §13 erasure model clarification for details.

Placing compile-time predicates in type positions (like `f(x: Positive(x))`) does not create wrapper types or allocate extra memory. But when `x`
comes from runtime input, runtime Bool checks **are** inserted.

**Interaction constraint with `ref`**: Compile-time predicates can only reference immutable borrows or values whose ownership has been transferred. Compile-time predicates referencing mutable borrows — the compiler cannot guarantee verification results remain valid at runtime — directly report compile error.

### Compiler Changes

1. **Parser**: Compile-time predicates use standard function syntax, no additional parsing rules needed
2. **Compile-time proof pipeline**: Unified Proved/Disproved/Unproven return interface, automatic strategy selection
3. **SMT acceleration module**: SMT-LIB 2.6 translation layer, default backend Z3, CVC5 alternative
4. **Type checker kernel**: Inference rule implementation — structural equivalence, βδι-reduction, universal quantifier introduction/elimination. This is the sole trust root; SMT and programmer proofs all pass through it for verification
5. **Verification condition generation**: WP/SP calculus + loop invariant proof obligations
6. **Error reporting**: Counterexample formatting + unsolved proposition reporting + source location correlation

### Backward Compatibility

- ✅ Code not using compile-time predicates is completely unchanged
- ✅ Compile-time predicates in CompileTime mode have zero runtime overhead, Runtime mode retains only necessary Bool checks
- ⚠️ RFC-022's `//!` syntax is no longer supported — but 022 was never implemented, no migration burden

## Trade-offs

### Advantages

- **Full Curry-Howard isomorphism fulfillment**: Types are propositions, programs are proofs, `name: Proposition = Proof`
- **Unity**: Compile-time predicates and ordinary functions use completely identical syntax, no conceptual split
- **SMT transparency**: Programmers do not need to know SMT exists; mental model is consistent with type checking
- **Incremental adoption**: Can start with one compile-time predicate and gradually increase coverage
- **Minimal runtime overhead**: CompileTime mode zero overhead, Runtime mode retains only necessary Bool checks

### Disadvantages

- **Compile time**: SMT solving adds compile time, but budget hard limits guarantee a controllable upper bound
- **Automatic proving boundary**: Complex predicates beyond first-order linear arithmetic may require programmers to write proof functions. This is not a language defect — this is the inevitable conclusion of the halting problem. The compiler honestly reports Unproven rather than falsely claiming True/False
- **Learning curve**: Writing effective compile-time predicates and proof functions requires understanding basic intuition of Curry-Howard isomorphism
- **Implementation complexity**: Unified compile-time proof pipeline requires careful design

### Risk Mitigation

- SMT solving budget hard limits (steps 10,000 / time 100ms / instantiation depth 3), over budget returns Unproven
- Dependent type pre-reduction: deterministic value computations consumed first, SMT only handles non-deterministic parts
- Unproven is not a dead end: programmers can write proof functions, type checker verifies — same as verifying any function return type
- Incremental verification: only verify changed modules
- Clear error messages + counterexample display + budget consumption report + unsolved propositions + suggestions (if compiler can provide)

## Alternative Approaches

| Approach                            | Why not chosen                                                                                     |
| ----------------------------------- | -------------------------------------------------------------------------------------------------- |
| RFC-022: `//!` comment-style specifications | Specifications split from types, violating Curry-Howard isomorphism                               |
| Separate specification files (like CVL) | Specifications separated from code, increasing maintenance cost                                   |
| Runtime assertions only            | Cannot statically guarantee correctness                                                            |
| External proof assistants (like Coq) | Separate from compiler, requires separate proof language and trust boundary. YaoXiang's choice: proofs are YaoXiang code, type checker is the sole trust root |
| **This approach: compile-time predicates as first-class citizens** | ✅                                                                                         |

## Implementation Strategy

### Phase Breakdown

| Phase       | Content                                                                                                        |
| ---------- | -------------------------------------------------------------------------------------------------------------- |
| **Phase 1** | Compiler kernel: structural equivalence + βδι-reduction + universal quantifier introduction/elimination. Supports simple arithmetic predicates (`x > 0`, `arr.len > 0`) |
| **Phase 2** | SMT-LIB translation layer + Z3/CVC5 integration. Pipeline returns Proved/Disproved/Unproven. Supports programmer writing proof functions on Unproven |
| **Phase 3** | Loop invariant VC generation + termination checking (linear ranking functions + predicate violation counting + bounded patterns + combinatorial explosion control) |
| **Phase 4** | Incremental verification + caching + IDE support                                                              |

### Dependencies

- RFC-010: Unified Type Syntax — compile-time predicates based on `name: type = value`
- RFC-011: Generic Type System — compile-time predicates can carry generic parameters
- RFC-009: Ownership Model — expressions in compile-time predicates follow ownership rules

## Open Questions

- [x] **SMT solver choice**: Default Z3 (MIT license, most extensively verified). CVC5 as SMT-LIB-compatible alternative, compiler flag for switching. Compiler internally translates to SMT-LIB
      2.6 standard format — SMT-LIB is the abstraction layer, no custom general-purpose solver interface.
- [x] **Specific values for solving budget**: steps 10,000 / time 100ms
      / quantifier instantiation depth 3. Fixed internally in compiler, no knobs. If real-world usage proves insufficient (not "user wrote it wrong"), adjust then.
- [x] **Quantifier support scope**: Language does not limit quantifier order. Compile-time predicates accept Type parameters — Type includes function types — so higher-order quantifiers are natural inferences of the type system, no special syntax needed. SMT solvers can automatically decide first-order quantifiers (forall/exists, supporting interleaved nesting, limited by budget depth 3). Higher-order quantifiers: SMT returns Unproven, compiler prompts "this predicate exceeds automatic proving scope, please provide proof function". Programmer writes a YaoXiang function whose return type equals the proposition — type checker verifies it. No external export, no AI, no interactive proof mode. Everything is YaoXiang code, everything verified by the type checker.
- [x] **Counterexample formatting**: Source variable names directly used as SMT variable names (with module prefix to avoid conflicts). When Z3 model returns, look up by variable name. Output format: variable name = concrete value + source location + predicate definition location. No complex mapping layer.
- [x] ~~**Compile-time predicate interaction with `ref` smart pointers?**~~
      → Decided: compile-time predicates only allow immutable borrows or transferred ownership. Mutable borrows cannot appear in compile-time predicates.
- [x] **Extension of `forall` predicate violation count measure to non-adjacent operations?**
      → Not extended. Current coverage scope (adjacent swaps, adjacent moves) is complementarily covered by Strategy 1 (linear ranking function) — quicksort outer interval shrinking covered by Strategy 1, heapsort covered by Strategy 1 (array index patterns). Loops whose termination cannot be proved by any strategy directly report compile error — this is hard safety philosophy, not a defect. If future real-world (not academic construction) algorithms cannot be covered by any of the four strategies, revisit.
- [x] **Linear ranking function enumeration combinatorial explosion**: Candidate enumeration upper limit is 3 bounded variables. ≤3 enumerates all linear combinations and SMT verifies each. >3 only tries single-variable measures (`v_i`, `u_i - v_i`), failure directly reports compile error — prompts programmer "loop has >3 bounded variables, compiler cannot automatically synthesize multi-variable measures". This is not an engineering compromise — it forces programmers to write simpler loops.

## References

- [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md)
- [RFC-011: Generic Type System Design](../accepted/011-generic-type-system.md)
- [RFC-009: Ownership Model](../accepted/009-ownership-model.md)
- Howard, W. A. (1969). The Formulae-as-Types Notion of Construction.
- Swamy, N. et al. (2016). Dependent Types and Multi-Monadic Effects in F\*. _POPL 2016_.
- Vazou, N. et al. (2014). Refinement Types for Haskell. _ICFP 2014_.
- Leino, K. R. M. (2010). Dafny: An Automatic Program Verifier for Functional Correctness. _LPAR
  2010_.
- De Moura, L. & Bjørner, N. (2008). Z3: An Efficient SMT Solver. _TACAS 2008_.

---

## Lifecycle and Disposition

```
┌─────────────┐
│   Draft     │  ← Author created
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Under Review │  ← Current status: community discussion
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
│ (official   │    │ (kept in    │
│  design)    │    │  place)     │
└─────────────┘    └─────────────┘
```
