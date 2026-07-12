---
title: "RFC-027: Compile-Time Predicates and Unified Static Verification"
status: "Accepted"
author: "Chen Xu"
created: "2026-06-07"
updated: "2026-07-05"
impl_status: "in_progress"
impl_detail: "Phase 1-2 completed, Phase 3 partially completed, Phase 4 partially completed"
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
> - [RFC-024: Concurrency Model Based on spawn Blocks](../accepted/024-concurrency-model.md)
>
> **Supersedes**: [RFC-022: Hoare Logic Static Verification Support (Specification Comments and Specification Types)](../deprecated/022-hoare-logic-static-verification.md) — Deprecated

## Summary

This document proposes introducing **compile-time predicates** as first-class citizens to YaoXiang, unifying all compile-time static verification into a single **proof pipeline**. Compile-time predicates are not external specification comments—they are functions. A function that returns Type can be used at type position; the compiler invokes it at compile time and checks the return value. Types are propositions; compile-time evaluation is proof.

**Core argument**: The only job of type checking at compile time is to construct and verify proof terms. Type equality, token conflicts, dependent type reduction, compile-time predicate evaluation, Hoare logic implications—all are different type checks within the compile-time proof pipeline, sharing the same pipeline. The SMT solver is an acceleration module for the type checker, not an independent trust boundary. When the compiler returns Unproven, the programmer writes a YaoXiang function as proof—the type checker verifies it in exactly the same way it verifies any function's return type. Everything is YaoXiang code; everything is verified by the type checker.

## Motivation

### Why deprecate RFC-022?

RFC-022 designed specifications as `//!` comments:

```yaoxiang
max: (T: Ord) -> ((arr: Array(T, n)) -> T) = {
    //! requires: NonEmpty(n) = n > 0          ← Independent of types
    //! ensures: ExistsMax(result, arr[0..n])   ← Independent of types
}
```

This commits a fundamental error against the Curry-Howard isomorphism: **splitting specifications and types into two layers**. Comments are not types. Comments do not participate in type checking. Comments are the mental model of "external tools".

The white paper states clearly:

> "No `//!` comments. No independent specification language. Everything is within the type system."

### Current Problems

- RFC-022's `//!` comments are external syntax independent of the type system
- Specification types and ordinary types are two separate systems, causing conceptual redundancy
- The split mode of "Debug Build verification / Release Build ignore" breaks unity
- The SMT solver is positioned as an external tool in traditional understanding—YaoXiang builds it in as an acceleration module of the type checker
- Type checking, borrow checking, compile-time predicate checking, and macro expansion each follow different paths

### The Correct Mental Model

Type checking can be abstracted as a function:

```
verify : Program → Proved | Disproved(Model) | Unproven
```

All compile-time checks—simple type matching, borrow conflict detection, compile-time predicate verification—are sub-tasks of this function. They share the same proof pipeline; the only difference is the complexity of proof terms and construction strategies.

When the compiler returns Unproven, the programmer provides a proof function—the return type of that function equals the proposition to be proved. The type checker verifies it. This is the same operation as ordinary type checking.

## Proposal

### 1. `{}` is the Proof Space: Types are Assertions, Verification is Type Checking

YaoXiang's `{}` is the compile-time proof space. Everything inside is an assertion, and the compiler guarantees each item is True—either automatically proven or proven by a programmer-supplied proof function.

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
#          Parameters in signature    Only assertions in {}
#          Compiler verifies x > 0 at compile time

List: (T: Type) -> Type = { data: Array(T) }
#      ^^^^^^^^              ^^^^^^^^^^^^^^^
#      Parameters in signature    Compiler verifies type_of(T) == Type, type_of(data) == Array(T)
```

The same pattern: `name: (params) -> Type = { assertions }`. The compiler does not distinguish "type assertions" from "value assertions"—both are evaluation targets in the proof pipeline.

**Loop invariants don't need to be written separately. Type annotations on variables are Floyd-Hoare invariants.**

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # Annotation references i—tells compiler s's type depends on i
    mut i: UpTo(arr.len) = 0     # At init i=0, verify: 0 == sum(arr[0..0]) → True
    while i < arr.len {
        s += arr[i]  # Compiler verifies: s_new == sum(arr[0..i+1])
        i += 1       # i changes → triggers re-verification of s dependency: s satisfies SumUpTo(arr, i_new)
    }
    return s  # s: SumUpTo(arr, arr.len) = sum(arr[0..arr.len])
}
```

The compiler generates one verification condition for the loop body—induction hypothesis (type annotation) → assignment operation → whether the new value satisfies the type annotation. Once the proof pipeline verifies the inductive step, all iterations are automatically covered. No `: decreases`, no `: Invariant`, no inductive proof needed—the compiler decomposes induction into local VCs for each assignment.

### 2. Pre/Postconditions: Compile-Time Predicates on Parameter Types and Return Types

Abandon RFC-022's `//! requires`/`//! ensures`. Compile-time predicates serve as type annotations on parameters or returns.

**Parameter side is function call.** Compile-time predicates are functions that return Type; their use at parameter position is calling them—just like `factorial(5)`. The return side introduces a new concept: return value parameters.

```yaoxiang
# Precondition: explicit invocation of compile-time predicate in parameter type
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b
#                       ^^^^^^^^^^  b is the current parameter name, passed to Positive
#                       Compiler extracts the actual argument value at call site, substitutes b, verifies Positive(actual)
#                       Example: divide(10, 2) → verify Positive(2) = { 2 > 0 } → True
#                       Example: divide(10, 0) → verify Positive(0) = { 0 > 0 } → False → compilation error

# Postcondition: return value parameter + compile-time predicate
IsMax: (T: Ord, arr: Array(T), result: T) -> Type = {
    forall j in 0..arr.len: result >= arr[j]
}

NonEmpty: (arr: Array(T)) -> Type = { arr.len > 0 }

max: (T: Ord) -> ((arr: NonEmpty(arr))) -> (result: IsMax(T, arr, result)) = {
#                                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
#                                            result is the return value parameter, value provided by return
#                                            Compiler substitutes the return value at return point, verifies postcondition
    candidate = arr[0]
    for i in 1..arr.len {
        if arr[i] > candidate { candidate = arr[i] }
    }
    return candidate
}
```

**Key rules**:

- **Parameter side**: `b: Positive(b)`—`b` is the current parameter name, passed to `Positive` as an argument. Function call syntax, zero implicits.
- **Return side**: `-> (result: IsMax(T, arr, result))`—`result` is the return value parameter, its value provided by the `return` statement. `result` exists only in the type signature, is only referenced by predicates, and does not enter the function body scope or appear at the call site.
- **Return value parameter is optional**: When there is no postcondition, it is not written; the signature is identical to an ordinary function (`-> Int`).
- **Unity**: Parameters and return value parameters are the same concept—`paramName: predicateCall(paramName)`—the only difference is whether the value is provided by the caller or by `return`.

### 3. Path Condition Propagation: Compile-Time Verification of Runtime Values

When compile-time predicates are used at binding positions, arguments are explicitly passed by the programmer. When runtime values enter refinement type parameters, the compiler completes verification through path condition collection and SMT implication—without the programmer explicitly providing a proof.

#### 3.1 Explicit Function Call

When compile-time predicates are used at binding positions, arguments are explicitly passed by the programmer—it is a function call, with zero implicits.

`Positive: (x: Int) -> Type = { x > 0 }` is a compile-time predicate constructor. When it appears at a binding position (parameter declaration, variable declaration, return type), the programmer explicitly passes the bound variable name:

```yaoxiang
b: Positive(b)
// b is already declared as the current parameter; Positive(b) is a function call
// After normalization: b: { b > 0 }
```

The compiler does not implicitly fill in arguments—`b: Positive(b)` is the same as `f(5)`, just a function call. `b` is bound as a parameter name, and its type annotation `Positive(b)` references `b` itself—this is a standard dependent type pattern, not an implicit expansion rule.

**Unified with RFC-010 `self`**: RFC-010 establishes that `self` is not a keyword, just a conventional parameter name ("writing `p`, `this`, `x` has exactly the same effect"). `b: Positive(b)` shares the same mechanism—the parameter name can be referenced in type annotations. `self` appears at the `self: Point` position, `b` appears at the `b: Positive(b)` position; both type annotations reference the parameter itself. The only difference is the complexity of the type annotation; the mechanism is identical—after name binding, the type can depend on this name.

The return type likewise uses explicit function calls:

```yaoxiang
Sorted: (arr: Array(T)) -> Type = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }

sort: (arr: Array(T)) -> (result: Sorted(result)) = { ... }
//                        ^^^^^^^^^^^^^^^^^^^^^^^
//                        result is the return value parameter; Sorted(result) is a function call
//                        Compiler substitutes the return value into result at return point, verifies Sorted(return value)
```

The same applies to local variable declarations:

```yaoxiang
let x: Positive(x) = 5
// x is bound to 5, Positive(5) → { 5 > 0 } → True → pass

// let y: Positive(y) = 0
// y is bound to 0, Positive(0) → { 0 > 0 } → False → compilation error
```

#### 3.2 Path Condition Collection

When runtime values appear in conditional branches, the compiler automatically collects path conditions, forming the current scope's **assumption set**. These assumptions participate in verification as background knowledge for compile-time Bool evaluation.

```yaoxiang
if y > 0 {
    // Compiler automatically acquires assumption in this branch: { y > 0 }
    let result = divide(x, y)
    // Verification condition: (y > 0) ⇒ (y > 0)
    // Proof pipeline determines implication holds → Proved
} else {
    // This branch's assumption: { !(y > 0) }
    // If divide(x, y) is called, verification condition is !(y > 0) ⇒ y > 0
    // Proof pipeline determines not implied → Disproved
}
```

This is not a hard-coded special pattern by the compiler—this is the natural behavior of the compile-time proof pipeline. Each type-checking call site sends to the pipeline:

```
{background assumptions} ⇒ {verification target}
```

The proof pipeline determines implication. Proved → pass; Disproved → compilation error + counterexample; Unproven → compilation error + unresolved proposition. Background assumptions come from the current program point's path conditions.

#### 3.3 Assumption Stack

During control flow analysis, the compiler maintains an assumption set for each basic block:

- **if-guard**: `if y > 0` → true branch pushes `y > 0`, false branch pushes `!(y > 0)` (if else is used)
- **match patterns**: `if let Some(v) = opt` → branch pushes `opt == Some(v)`
- **Logical conjunction**: `if x > 0 && y < 10` → branch pushes `x > 0` and `y < 10`
- **Function preconditions**: When calling `divide(a, b)`, `b` must satisfy `Positive`'s evidence either from the current assumption or from the actual argument's own refinement type annotation (`b` is annotated as `Positive`, its type carries `b > 0`)
- **Assignment**: `let z = y` → existing refinement conditions on `y` transfer to `z`

All assumptions enter the compile-time proof pipeline. When entering the SMT acceleration path, they are translated into SMT-LIB background assertions.

#### 3.4 No Static Evidence Equals Compilation Error

If the programmer directly writes:

```yaoxiang
divide_user_input: (x: Int, y: Int) -> Int = divide(x, y)
```

The current program point has no assumption `y > 0`; the actual argument `y` itself has no `Positive` type annotation. The verification condition is:

```
{} ⇒ { y > 0 }
```

Pipeline returns `Disproved` (not implied) → compilation error:

> Cannot prove parameter `b` satisfies `Positive` in `divide` call.
> `y` comes from function input with no proven bound.
> Consider guarding the call with an if branch: `if y > 0 { divide(x, y) }`.

YaoXiang does not accept runtime values directly entering refinement type parameters without providing static evidence. This is not a restriction—this is the core of the hard-safety philosophy. Whatever the compiler cannot statically prove shall not pass compilation.

#### 3.5 Relationship with the Unified Pipeline

Path condition propagation is not an additional mechanism. It is the direct extension of the compile-time proof pipeline into control flow analysis:

| Stage | Responsibility |
|------|------|
| Path condition collection | Compiler's control flow analysis stage, annotates assumption set for each basic block |
| Verification condition generation | When encountering type constraints to be verified, merge path conditions + actual argument type information |
| Proof pipeline evaluation | Compiler kernel → SMT acceleration → derive Proved / Disproved / Unproven |
| Result | `Proved` → pass; `Disproved` → compilation error + counterexample; `Unproven` → compilation error + unresolved proposition (programmer can provide proof function) |

No new components. No special rules. Path conditions are the background knowledge of the proof pipeline—sharing the same pipeline and budget system as type equality and borrow constraints.

### 4. Compile-Time Proof Pipeline

All compile-time checks share the same pipeline. The core operation of the pipeline is **type checking**—checking whether a proof term's type equals the proposition to be proved. Everything is type checking.

```
Compile-time encounters Bool expression requiring evaluation (i.e., needs to construct a proof term)
        │
        ├── Type equality (T1 == T2)
        │   → Compiler direct judgment (structural equivalence)
        │
        ├── Token conflict condition (!conflicting(tokens))
        │   → Flow-sensitive liveness analysis (Dup/Linear property tracking)
        │
        ├── Dependent type reduction (n + m simplification)
        │   → Compile-time term rewriting system (βδι-reduction)
        │
        ├── Compile-time predicates (x > 0, forall...)
        │   → Compiler itself + SMT acceleration module
        │
        └── Hoare logic implication (P ⇒ Q)
            → Compiler + SMT acceleration module
                    │
                    ▼
             ┌──────────┐
             │ Proved   │  → Compilation passes
             │ Disproved│  → Compilation error + counterexample
             │ Unproven │  → Compilation error + unresolved proposition
             └────┬─────┘
                  │
                  ▼
         Programmer writes proof function (YaoXiang code)
                  │
                  ▼
         Type checker verifies ──→ Proved ──→ Compilation passes
                  │
                  ▼
            Verification fails → Compilation error: "proof does not hold"
```

#### 4.1 Proof Result: Three-Value Algebra

Compile-time evaluation returns three results—this is the inevitable conclusion of the halting problem and the natural division of proof theory:

```
eval_compile_time : BoolExpr → Proved | Disproved(Model) | Unproven
```

- **Proved** → Halts, proof term constructed, type checking passes. Compilation continues.
- **Disproved(M)** → Halts, counterexample M exists. Compilation error + counterexample + source location.
- **Unproven** → Within given resource limit, no proof constructed. Compilation error + unresolved proposition + budget consumption report.

**Unproven ≠ False.** The compiler saying "I cannot prove it" is not equivalent to the proposition being false—it's just beyond the current automatic proof capability. This is honesty, not a defect.

Budget hard limits are the engineering solution to the halting problem. No knobs provided—giving a knob is asking the user "do you think your program will halt"; the user doesn't know, and the compiler doesn't know either.

#### 4.2 After Unproven: The Programmer Writes the Proof

When the compiler returns Unproven, the programmer can write a **proof function**—a YaoXiang function whose return type equals the proposition to be proved. The type checker verifies this function—in exactly the same way it verifies `add(a, b): Int`.

```
Proposition = Type
Proof       = Program (a value of that type)
Verification= Type checking (the only trust root)
```

The SMT solver is not an independent trust boundary—it is an **acceleration module of the type checker**. SMT helps find proofs, but the verifier of proofs is always the type checker. When SMT returns `unsat`, the compiler reconstructs its result as a proof term verifiable by the type checker. If reconstruction fails (SMT's reasoning steps exceed the compiler kernel's reasoning rules), it falls back to Unproven—the programmer can manually write a proof function.

```yaoxiang
# Proposition: refinement property the compiler cannot automatically prove
FirstIsMin: (T: Ord, arr: Sorted(T)) -> Type = {
    forall i in 0..arr.len: arr[0] <= arr[i]
}

# Proof: programmer writes a function whose return type is the above proposition
# Type checker verifies this function—in exactly the same way as verifying add(a,b): Int
first_is_min: (T: Ord, arr: Sorted(T)) -> FirstIsMin(T, arr) = {
    # Compiler verifies here: function body's type = FirstIsMin(T, arr)
    ...
}
```

No AI needed, no exporting to Coq, no new concepts. **Properties the compiler cannot automatically prove → programmer writes proof in YaoXiang code → type checker verifies.** The whole process is a smooth gradient—the compiler handles simple proofs, leaving the brain for hard ones.

#### 4.3 Layered Dependencies Within the Pipeline

The above evaluators share the same interface but have evaluation order. Type equality is the prerequisite for all subsequent analysis; ownership/token checks depend on type information; refinement predicate verification depends on the results of the first two layers. The compiler evaluates layer by layer; expressions that fail at lower layers do not enter upper layers—avoiding wasting solver budget on type-incorrect programs.

```
Evaluation order (same pipeline, layered scheduling)
├── Layer 0: Type equality (T1 == T2)
│   └── Structural unification → fail means subsequent is meaningless, return Disproved directly
├── Layer 1: Ownership/token conflict
│   └── Flow-sensitive liveness analysis → fail means memory safety does not hold, return Disproved directly
└── Layer 2: Refinement predicate / Hoare implication
    └── Compiler itself → SMT acceleration → derive Proved / Disproved / Unproven
```

Each layer still returns `Proved/Disproved/Unproven`, sharing the same interface and the same budget system.

### 5. Three-Layer Function Unification

| Layer | Execution Time | Input | Output | Example |
|------|----------|------|------|------|
| Value-level function | Runtime | Value | Value | `add: (a: Int, b: Int) -> Int = a + b` |
| Type constructor | Compile-time | Type/Value | Type | `List: (T: Type) -> Type = { data: Array(T) }` |
| Compile-time predicate | Compile-time | Value | Type | `Positive: (x: Int) -> Type = { x > 0 }` |

All use the same `name: type = value` syntax. Compile-time predicates and type constructors go through the same compile-time proof pipeline—`{}` is the proof space.

### 6. Loops: Floyd-Hoare Verification Condition Generation

Loops do not need separate `: Invariant(...)` or `: decreases(...)` annotations. Compile-time predicate type annotations on variables define Floyd-Hoare-style assertions—the compiler generates verification conditions from type annotations, and the proof pipeline checks whether each assignment maintains the type.

Core mechanism: each assignment operation corresponds to a Hoare triple `{P} x := e {Q}`; the verification condition is `P ⇒ Q[e/x]`. The compiler generates one verification condition for the loop body—once the proof pipeline verifies the inductive step, all iterations are automatically covered.

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # Annotation references i; at init i=0, verify: 0 == sum(arr[0..0]) → True
    mut i: UpTo(arr.len) = 0     # Verify: 0 <= 0 <= arr.len → True
    while i < arr.len {
        # Compiler generates one VC for the loop body. Premise: s satisfies SumUpTo(arr, i), i satisfies UpTo(arr.len).
        #
        # s += arr[i]:
        #   Verification obligation: s_new satisfies SumUpTo(arr, i) (current i unchanged)
        #   Substitute s_new = s_old + arr[i]:
        #     Need s_old + arr[i] == sum(arr[0..i+1])
        #     From inductive hypothesis s_old == sum(arr[0..i]), add arr[i] to both sides:
        #     sum(arr[0..i]) + arr[i] == sum(arr[0..i+1])
        #   Compiler + SMT: linear arithmetic, millisecond-level → Proved
        #
        # i += 1:
        #   i changes → s's type annotation in dependency graph references i → trigger re-verification
        #   New verification target: s satisfies SumUpTo(arr, i_new)
        #   That is, s == sum(arr[0..i_new]), guaranteed by the previous step → Proved
        s += arr[i]
        i += 1
    }
    return s  # At this point s: SumUpTo(arr, arr.len), i.e., s == sum(arr[0..arr.len])
}
```

Loop invariants are the type annotations on variables—programmer writes types, compiler checks inductive steps. The compiler doesn't need to "discover" invariants, nor does it need to "automatically perform induction"—it decomposes inductive proofs into local verification conditions for each assignment operation, and hands them to the proof pipeline to divide and conquer.

#### 6.1 Dependency Tracking: Dependent Types on Mutable Variables

The premise of the above mechanism is: the compiler knows that the type annotation `SumUpTo(arr, i)` of `s` references `i`—when `i` changes, the type constraint on `s` also changes. This requires the compiler to maintain a **type dependency graph between variables**.

**Data structure**:

```
TypeDepGraph: Map<VarName, Set<VarName>>
# Keys are depended-on variables, values are the set of variables that reference this variable in their type annotations
# Example: { i: {s}, j: {s, t}, ... }
```

**Construction**: When the type checker processes `mut v: Pred(... x ...) = init`, it parses free variable references in `Pred(...)` arguments. If the argument references another mutable variable `x` in the current scope, record `x → v` in the dependency graph.

**Trigger**: When the depended-on variable `x` is assigned, the compiler:
1. Looks up all variables in the dependency graph that depend on `x`: `{v₁, v₂, ...}`
2. For each `v`, generates a verification condition: whether `v`'s current value satisfies the updated type `Pred(... x_new ...)`
3. Sends the VC into the proof pipeline

**Assignment order sensitivity**: Dependency tracking naturally enforces correct assignment order. Take `SumUpTo(arr, i)` as an example:

```yaoxiang
# Correct order
s += arr[i]   # s_new satisfies SumUpTo(arr, i+1)
i += 1        # i changes → re-verify s satisfies SumUpTo(arr, i_new) → True

# Wrong order—compiler rejects
i += 1        # i changes → re-verify s satisfies SumUpTo(arr, i_new)
              # s not yet updated, s_old == sum(arr[0..i_old]) ≠ sum(arr[0..i_new])
              # → Compilation error: variable s does not satisfy type SumUpTo(arr, i_new)
s += arr[i]   # Unreachable
```

**Composite dependencies**: A variable can depend on multiple variables. The type annotation `{ v: Int; v == x + y }` depends on both `x` and `y`—either changing triggers re-verification.

**Relationship with the proof pipeline**: Dependency tracking is the trigger for VC generation, not an independent verification mechanism. It answers "when to generate VCs"—the proof pipeline answers "whether VCs hold".

### 7. Termination Checking

Fully automatic at compile time. Loops the compiler can prove pass; loops it cannot prove are reported directly as compilation errors—the programmer must make the loop amenable to automatic termination analysis. No half-automatic annotation escape hatch.

#### 7.1 Design Principles

The compiler automatically extracts information needed for termination proofs from two sources:

1. **Variable type annotations**: Boundary constraints in refinement types (e.g., `UpTo(n)` gives upper bound `n` and lower bound `0`)
2. **Loop body operations**: Operations applied to variables per iteration

The compiler tries four measure synthesis strategies by priority, stopping when one is found.

#### 7.2 Strategy 1: Automatic Linear Rank Function Synthesis

When variables have linear bound annotations, the compiler enumerates candidate linear measures and verifies via SMT.

```
Input:
  Variables v₁: UpTo(u₁), v₂: UpTo(u₂), ... (variables with upper and lower bounds)
  Loop condition cond
  Set of assignments in the loop body

Algorithm:
  1. Extract each variable's bounds from type annotations: [low_i, high_i]
  2. Enumerate candidate measures: v_i, u_i - v_i, v_i - v_j, etc., linear combinations
  3. For each candidate measure m:
     - SMT verifies m ≥ 0 (derived from type bounds)
     - For each execution path in the loop body, SMT verifies m' < m (strictly decreasing)
  4. Find a qualifying linear combination → termination proven
```

Coverage: any loop where a variable is assigned a linear expression (`v = a·v + b`) and has a bounded type annotation. Including `i += const`, `i -= const`, and interval-narrowing patterns like binary search:

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

#### 7.3 Strategy 2: Predicate Violation Count—Automatically Extracting Measures from Target Types <span style="color:orange">[Experimental Strategy]</span>

> ⚠️ **Current status: experimental strategy; whether to include depends on actual feasibility during Phase 3 implementation.**
> This strategy works for adjacent swap operations (bubble sort, insertion sort); cannot automatically prove for non-adjacent operations (quicksort partition, heapsort sift-down).
> See coverage boundaries in the table below. If Phase 3 verification proves infeasible, this strategy will be removed or downgraded to future work.

Core insight: **user-written specifications are material for compiler reasoning.** The compiler doesn't need to hardcode "what is sorted"—it reads the definition of `Sorted` and automatically extracts measures from the definition.

```
Input:
  Target type: Sorted(arr) = { forall i in range: cond(i, arr) }
  Loop body operations: adjacent element swaps

Algorithm:
  1. Parse predicate definition: forall i in range: cond(i, arr)
  2. Automatically generate measure: violation_count = |{ i | ¬cond(i, arr) }|
  3. Analyze operation's impact on measure:
     - Adjacent swap arr[j], arr[j+1] = arr[j+1], arr[j]
     - Only affects pairs at indices j-1, j, j+1
     - If arr[j] > arr[j+1] (predicate violation), after swap this pair satisfies the predicate
     - violation_count decreases by at least 1
  4. Upper bound: n·(n-1)/2 (max adjacent inversions), lower bound: 0
  → Termination proven
```

**Current coverage**:

| Algorithm | Operation Pattern | Strategy 2 Provable? | Reason |
|------|----------|:---:|------|
| Bubble sort | Adjacent swap | ✅ | violation_count strictly decreases per swap |
| Insertion sort | Adjacent shift | ✅ | Each shift eliminates one violation pair |
| Selection sort | Non-adjacent swap | ❌ | A single swap may increase violation_count |
| Quicksort | Partition | ❌ | Non-adjacent swap, no monotonic decrease guarantee |
| Heapsort | sift-down | ❌ | Tree-shaped operations, violation_count non-monotonic |

**Complementary strategies**: For quicksort, the `low < high` interval narrowing is covered by Strategy 1 (linear rank function)—the outer partition recursion halves the interval each time. Strategies 1 and 2 complementarily cover most practical algorithms' termination. However, the generalization of Strategy 2 (non-adjacent operations, tree-shaped operations) remains an open problem.

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

#### 7.4 Strategy 3: Bounded Increment/Decrement Patterns

`v += const` (positive number), variable has upper-bound type annotation → measure `upper_bound - v` decreases by `const` each time, lower bound 0. This is a degenerate case of Strategy 1, handled first by the compiler for quick processing.

#### 7.5 Strategy 4: Multiplicative Scaling Measure Templates

`v *= const` (const > 1), variable has upper and lower bound type annotations. The compiler has built-in logarithmic measure template `ceil(log_const(upper/v))`, which decreases by 1 on each multiplication by const.

```yaoxiang
mut i: Positive(i) = 1
while i < n {
    # Compiler automatically derives: measure ceil(log₂(n/i)), decreases by 1 on each multiplication by 2
    i *= 2
}
```

#### 7.6 Termination and Correctness Separation

Termination proofs and correctness proofs are independent:
- **Termination**: The four strategies above automatically prove loops exit in finite steps
- **Correctness**: Whether the loop body advances toward the target type, checked by the compile-time proof pipeline through verification conditions

Both pass → compilation passes. Termination proven but correctness fails → compilation error + counterexample. Correctness proven but termination cannot be proven → compilation error pointing to the unanalyzable variable or operation. Both fail → compilation error reports both failure reasons separately.

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
| Compiler can analyze recursion decrease (e.g., `n-1`) | Compile-time evaluation |
| Not decreasing / cannot determine decrease | Compilation error |
| Runtime call (not at type position) | No termination check needed |

#### 7.8 Hard Boundary

`i = f(i)` where `f` is non-invertible, non-closed, does not preserve any monotonicity—mathematically impossible to automatically prove termination. Compilation error:

> This loop cannot automatically prove termination. The loop variable depends on the unanalyzable function `f`. Please use an iteration pattern that can be analyzed by the compiler.

This is not a compiler failure. Whatever cannot be statically proven safe shall not pass compilation.

### 8. SMT Solver: Acceleration Module of the Type Checker

The SMT solver is an external tool in traditional languages (e.g., F\* calls Z3, Dafny calls Z3). In YaoXiang, it is **an acceleration module of the type checker**—invoked only when the compiler kernel itself cannot directly determine. SMT helps find proofs, but the verifier of proofs is the type checker.

**Trust model**: The type checker is the only trust root. The SMT solver is an acceleration module—it helps find proofs, but SMT is not an independent trust boundary. The compiler trusts Z3's `unsat` results (aligned with the F\*/Dafny route—the probability of Z3 errors is lower than the compiler's own bug rate, an engineering pragmatic choice). True unreliability control lies in the SMT translation layer—if the translation has bugs, the compiler will be exposed in other tests.

**Interface**: The compiler internally translates to SMT-LIB 2.6 standard format, not bound to a specific solver API. SMT-LIB is an ISO standard; Z3, CVC5, MathSAT, Yices all natively support it.

**Default backend**: Z3 (MIT license, most extensive documentation and community validation). CVC5 as SMT-LIB-compatible alternative—users can switch via compiler flags at compile time.

No "generic solver abstraction layer"—SMT-LIB is the abstraction layer. In the future, if CVC5 makes breakthroughs in specific theories, switching requires only changing the binary, no compiler code changes.

```
Compile-time Bool expression
        │
        ├── Compiler kernel can directly determine (structural equivalence, simple arithmetic,
        │   trivial formulas after constant folding)
        │   → Return Proved / Disproved directly
        │
        └── Compiler kernel cannot directly determine (quantifiers, symbolic variables)
            → Dependent type pre-reduction (factorial(5) → 120)
            → Translate to SMT-LIB format
            → Send to Z3/CVC5 (with budget limits)
            → Return value: unsat → Proved  │  sat + model → Disproved  │  unknown → Unproven
```

**Solver budget—hard limits, like stack depth**:

| Budget Dimension | Default | Description |
|----------|--------|------|
| Solving steps | 10,000 | Z3 for linear arithmetic usually within a hundred steps. 10,000 steps covers 99% of practical predicates. |
| Time | 100ms | A single predicate exceeding 100ms = the user is writing a compile-time program rather than a type annotation. 100ms × 50 predicates = 5 second compile time upper limit. |
| Quantifier instantiation depth | 3 | Three levels of nested quantifiers cover practical patterns. More than three layers likely means writing logic exercises. |

Exceeding the budget returns Unproven, compilation error + predicate location + consumption. No degradation, no runtime checks, no silent pass.

**Why this is practically viable**: In engineering, 95% of practical predicates are linear arithmetic—`x > 0`, `arr.len > 0`, `0 <= idx < arr.len`—all within the decidable fragment, and SMT solvers return in milliseconds for such problems. For the rare complex predicates that exceed the budget, the programmer writes a proof function.

Dependent types have a layer of pre-reduction before SMT calls: `factorial(5)` directly evaluates to `120` at compile time, `append([1,2], [3])` directly evaluates to `[1,2,3]`. These deterministic value computations do not consume SMT budget.

Programmers don't need to know SMT exists. Mental model: **the compiler can prove → passes; cannot prove → error—if the compiler doesn't know, you can write a function to prove it to it.**

### 9. Compile-Time Predicate Composition

Compile-time predicates are functions returning Type; composition is naturally implemented through function composition:

```yaoxiang
SortedNonEmpty: (T: Ord, arr: Array(T)) -> Type = {
    Sorted(T, arr) && NonEmpty(arr)
}
```

### 10. Code Examples

#### 9.1 Division Safety

```yaoxiang
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b

result = divide(10, 2)   # ✅ Compiler verifies Positive(2) = { 2 > 0 } → True
# result = divide(10, 0)  # ❌ Compiler verifies Positive(0) = { 0 > 0 } → False
```

#### 9.2 Array Access Safety

```yaoxiang
InBounds: (idx: Int, arr: Array(T)) -> Type = { 0 <= idx && idx < arr.len }

get: (arr: Array(T), idx: InBounds(idx, arr)) -> T = arr.data[idx]

arr = Array(Int)(1, 2, 3)
x = get(arr, 1)   # ✅ Compiler verifies InBounds(1, arr) = { 0 <= 1 && 1 < 3 } → True
# y = get(arr, 5)  # ❌ Compiler verifies InBounds(5, arr) = { 0 <= 5 && 5 < 3 } → False
```

#### 9.3 Sort Correctness

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


### 11. Dispatch Pipeline: Unifying Compile-Time and Runtime Dispatch

`assert` and `Assert` are two sides of the same refinement type primitive. The `dispatch` pipeline automatically decides between compile-time proof and runtime check based on **whether the predicate's free variables are accessible at compile time**:

| Criterion | Mode | Behavior |
|------|------|------|
| All free variables known at compile time (generic parameters, compile-time constants) | **CompileTime** | Enter proof pipeline: Proved → erased; Disproved → compilation error; Unknown → require proof |
| Some free variables come from runtime (function parameters, external input, mut variables) | **Runtime** | Insert runtime check, inject refinement facts into flow-sensitive assumption set Γ |

**Key point**: "Cannot determine" ≠ "disproved". In CompileTime mode, Unknown requires proof (no silent downgrade); in Runtime mode, the proposition has no truth value at compile time at all—no prover, however strong, can write a universally true proof for "the user might have entered a negative number"; runtime check is the only sound choice. This is not the prover being too weak; it is a theoretical inevitability.

### 12. Flow-Sensitive Assumption Set Γ: Strongest Postcondition Propagation

The compiler maintains a flow-sensitive assumption set Γ, tracking propositions known to hold at each control flow point.

**SP (strongest postcondition) propagation**:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
```

**mut variable kill set**: After a `mut` variable is reassigned, all assumptions involving that variable are removed from Γ:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
mut x = x - 5       // Γ = {}  ← x > 0 killed
```

This is a hard requirement of soundness—the variable's value changed, old assumptions are invalid.

**Branch confluence**: When IF/ELSE or match branches merge, Γ takes the intersection of each branch's assumptions. Only propositions that hold in all paths are carried out of the branch.

### 13. Erasure Model Clarification: Witness Erasure ≠ Check Erasure

RFC-027's claim that "refinement types are **completely erased** at runtime" refers to the **proof witness (proof token)**—proof terms verified at compile time do not generate runtime code. But the **runtime checks** inserted by dispatch in Runtime mode are retained—they are Bool checks executed at the value level, not type-level witnesses.

Summary: witness erased, check retained. The two things don't conflict, and RFC-027's original claim stands.

## Detailed Design

### Syntax Changes

| Before (RFC-022) | After (this RFC) |
|---|---|
| `//! requires: NonEmpty(n) = n > 0` | Compile-time predicate as parameter type `(b: Positive(b))` |
| `//! ensures: ExistsMax(result, arr)` | Return type uses return value parameter `-> (result: IsMax(T, arr, result))` |
| `/*! invariant: ... !*/` | Compile-time predicate type annotation on variables—Floyd-Hoare invariant |
| `//! decreases: n` | Compiler fully automatic measure function derivation |
| Specifications are comments | Specifications are the type system |

### Syntax

**Compile-time predicates have no new keywords.** `{}` is the proof space, fully consistent with existing type definition syntax. Compile-time predicates are functions returning Type—`name: (params) -> Type = { assertions }`. Usage is function calls—`Positive(b)`, `IsMax(T, arr, result)`.

```bnf
# Compile-time predicate = function returning Type, assertions in {} are verified by the compiler
# Uses existing function/type syntax, no new BNF rules needed
predicate ::= identifier ':' params '->' 'Type' '=' '{' assertions '}'
```

**New syntax concept: return value parameter**—in `-> (name: Type)`, `name` is the return value parameter.

The return value parameter is the **only new syntax concept** YaoXiang introduces on top of existing function syntax. Its semantics:
- `name`'s value is provided by the `return` statement
- `name` exists only in the type signature, referenced by postcondition predicates (e.g., `-> (result: IsMax(T, arr, result))`)
- `name` does not enter the function body scope and does not appear at the call site
- The return value parameter is **optional**—when there is no postcondition, the signature is identical to an ordinary function (`-> Int`), introducing no extra burden

The reason for introducing it: postconditions need to reference "the value the function is about to return". Without return value parameters, the compiler can only let predicates reference the return value through special rules (like implicit variables `$result` or `__retval__`). Return value parameters make this reference explicit—it is just a parameter, but its value is provided by `return` rather than the caller.

**Proof functions** are not a new concept—they are YaoXiang functions whose return type is the proposition being asserted. When the compiler returns Unproven, the programmer provides a proof function, and the type checker verifies it in exactly the same way as verifying any function's return type. No new syntax, new keywords, or new rules.

### Type System Impact

- **Type universe**: Compile-time predicates are at the Type₂ layer—functions that accept values and return Type, at the same level as type constructors
- **Generic interaction**: Compile-time predicates can take generic parameters, e.g., `NonEmpty: (T: Type) -> (arr: Array(T)) -> Type`
- **Ownership interaction**: Expressions in compile-time predicates obey ownership rules; can only read, not write
- **Type inference**: Compile-time predicate parameters participate in HM type inference

### Runtime Representation

Compile-time predicates are processed at runtime **according to dispatch results**:
- **CompileTime mode** (all free variables known at compile time): After proof, the witness token is completely erased. `Positive: (x: Int) -> Type = { x > 0 }`—the parameter `b: Positive(5)`'s runtime representation is just `Int`. The refinement condition `{ 5 > 0 }` has passed, erased.
- **Runtime mode** (runtime free variables exist): Retain runtime check—execute Bool check at the value level, inject into flow-sensitive assumption set Γ. See §11 dispatch pipeline and §13 erasure model clarification for details.

Putting compile-time predicates at type position (e.g., `f(x: Positive(x))`) does not generate wrapper types or allocate extra memory. But when `x` comes from runtime input, a **runtime Bool check will be inserted**.

**Interaction constraints with `ref`**: Compile-time predicates can only reference immutably borrowed values or values with transferred ownership. Compile-time predicates referencing mutably borrowed values cannot guarantee the verification result still holds at runtime—the compiler directly reports a compilation error for such usage.

### Compiler Changes

1. **Parser**: Compile-time predicates use standard function syntax, no additional parsing rules needed
2. **Compile-time proof pipeline**: Unified Proved/Disproved/Unproven return interface, automatic strategy selection
3. **SMT acceleration module**: SMT-LIB 2.6 translation layer, default backend Z3, CVC5 alternative
4. **Type checker kernel**: Reasoning rule implementation—structural equivalence, βδι-reduction, universal quantifier introduction/elimination. This is the only trust root; SMT and programmer proofs are verified through this
5. **Verification condition generation**: WP/SP calculus + loop invariant proof obligations
6. **Error reporting**: Counterexample formatting + unresolved proposition report + source location association

### Backward Compatibility

- ✅ Code that does not use compile-time predicates is completely unchanged
- ✅ Compile-time predicates have zero runtime overhead in CompileTime mode; only necessary Bool checks are retained in Runtime mode
- ⚠️ RFC-022's `//!` syntax is no longer supported—but 022 was never implemented, no migration burden

## Trade-offs

### Advantages

- **Curry-Howard isomorphism fully realized**: Types are propositions, programs are proofs, `name: Proposition = Proof`
- **Unity**: Compile-time predicates and ordinary functions use exactly the same syntax, no conceptual split
- **SMT transparency**: Programmers don't need to know SMT exists; mental model consistent with type checking
- **Progressive adoption**: Start with one compile-time predicate and gradually expand coverage
- **Minimal runtime overhead**: Zero overhead in CompileTime mode; only necessary Bool checks in Runtime mode

### Disadvantages

- **Compile time**: SMT solving increases compile time, but budget hard limits guarantee upper bound is controllable
- **Automatic proof boundaries**: Complex predicates beyond first-order linear arithmetic may require the programmer to write proof functions. This is not a language defect—this is the inevitable conclusion of the halting problem. The compiler honestly reports Unproven rather than falsely reporting True/False
- **Learning curve**: Writing effective compile-time predicates and proof functions requires understanding the basic intuition of Curry-Howard isomorphism
- **Implementation complexity**: Unifying the compile-time proof pipeline requires careful design

### Risk Mitigation

- SMT solver budget hard limits (steps 10,000 / time 100ms / instantiation depth 3); exceeding budget returns Unproven
- Dependent type pre-reduction: deterministic value computation consumed first, SMT only tackles the non-deterministic part
- Unproven is not a dead end: programmers can write proof functions, type checker verifies—consistent with verifying any function's return type
- Incremental verification: only verifies changed modules
- Clear error messages + counterexample display + budget consumption report + unresolved proposition + suggestions (if the compiler can give them)

## Alternatives

| Alternative | Why Not Chosen |
| -------------------------------- | -------------------------------------- |
| RFC-022: `//!` comment-style specifications | Specifications and types split, violates Curry-Howard isomorphism |
| Independent specification files (e.g., CVL) | Specifications separated from code, increases maintenance cost |
| Runtime-only assertions | Cannot statically guarantee correctness |
| External proof assistants (e.g., Coq) | Disconnected from the compiler, requires independent proof language and trust boundary. YaoXiang's choice: proof is YaoXiang code, type checker is the only trust root |
| **This proposal: compile-time predicates as first-class citizens** | ✅ |

## Implementation Strategy

### Phase Division

| Phase | Content |
| ---------- | -------------------------------------------------------------------- |
| **Phase 1** | Compiler kernel: structural equivalence + βδι-reduction + universal quantifier introduction/elimination. Supports simple arithmetic predicates (`x > 0`, `arr.len > 0`) |
| **Phase 2** | SMT-LIB translation layer + Z3/CVC5 integration. Pipeline returns Proved/Disproved/Unproven. Unproven supports programmer-written proof functions |
| **Phase 3** | Loop invariant VC generation + termination checking (linear rank function + predicate violation count + bounded patterns + combinatorial explosion control) |
| **Phase 4** | Incremental verification + caching + IDE support |

### Dependencies

- RFC-010: Unified Type Syntax — compile-time predicates based on `name: type = value`
- RFC-011: Generic Type System — compile-time predicates can take generic parameters
- RFC-009: Ownership Model — compile-time predicate expressions obey ownership rules

## Open Questions

- [x] **SMT solver selection**: Default Z3 (MIT license, most extensively validated). CVC5 as SMT-LIB-compatible alternative, switchable via compiler flag. The compiler internally translates to SMT-LIB 2.6 standard format—SMT-LIB is the abstraction layer, no custom generic solver interface.
- [x] **Specific budget values**: Steps 10,000 / time 100ms / quantifier instantiation depth 3. Fixed inside the compiler, no knob. If real use cases prove insufficient in actual use (not "user wrote incorrectly"), adjust.
- [x] **Quantifier support scope**: The language level does not limit quantifier order. Compile-time predicates accept Type parameters—Type includes function types—therefore higher-order quantifiers are a natural inference of the type system, requiring no special syntax. SMT solver can automatically determine first-order quantifiers (forall/exists, supports interleaved nesting, limited by budget depth 3). Higher-order quantifiers: SMT returns Unproven, compiler prompts "this predicate exceeds automatic proof range, please provide proof function". The programmer writes a YaoXiang function whose return type equals the proposition—the type checker verifies this function. No external export needed, no AI needed, no interactive proof mode needed. Everything is YaoXiang code, everything is verified by the type checker.
- [x] **Counterexample formatting**: Source variable names used directly as SMT variable names (with module prefix to avoid conflicts). When Z3 model is returned, look up by variable name. Output format: variable name = concrete value + source location + predicate definition location. No complex mapping layer.
- [x] ~~**Interaction between `ref` smart pointers and compile-time predicates?**~~ → Decided: compile-time predicates only allow immutably borrowed values or values with transferred ownership. Mutably borrowed values cannot appear in compile-time predicates.
- [x] **Extension of forall predicate violation count measure to non-adjacent operations?** → No extension. Current coverage (adjacent swap, adjacent shift) is complementarily covered by Strategy 1 (linear rank function)—quicksort outer interval narrowing is covered by Strategy 1, heapsort is covered by Strategy 1 (array index pattern). Loops that cannot be proven terminating by any strategy are reported directly by the compiler—this is hard-safety philosophy, not a defect. If in the future there are real scenarios (not academic constructions) where the four strategies cannot cover an algorithm, then it will be revisited.
- [x] **Linear rank function enumeration combinatorial explosion**: Candidate enumeration upper limit is 3 bounded variables. ≤3 enumerates all linear combinations and SMT verifies one by one. >3 only tries single-variable measures (`v_i`, `u_i - v_i`), failure directly reports compilation error—prompting the programmer "loop has >3 bounded variables, compiler cannot automatically synthesize multi-variable measures". This is not an engineering compromise—it forces the programmer to write simpler loops.

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
│ Under Review│  ← Current state: community discussion
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
│ (Official   │    │ (Retained   │
│  Design)    │    │   in Place) │
└─────────────┘    └─────────────┘
```