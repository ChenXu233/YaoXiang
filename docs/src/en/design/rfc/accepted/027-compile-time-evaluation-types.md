---
title: "RFC-027: Compile-Time Predicates and Unified Static Verification"
status: "Accepted"
author: "Chenxu"
created: "2026-06-07"
updated: "2026-06-10"
---

# RFC-027: Compile-Time Predicates and Unified Static Verification

> **References**:
>
> - [RFC-009: Ownership Model](../accepted/009-ownership-model.md)
> - [RFC-010: Unified Type Syntax - name: type = value Model](../accepted/010-unified-type-syntax.md)
> - [RFC-011: Generics System Design](../accepted/011-generic-type-system.md)
> - [RFC-024: Concurrency Model Based on spawn Blocks](../accepted/024-concurrency-model.md)
>
> **Supersedes**: [RFC-022: Hoare Logic Static Verification Support (Specification Comments and Specification Types)](../deprecated/022-hoare-logic-static-verification.md) — Deprecated

## Summary

This document proposes introducing **compile-time predicates** as first-class citizens in YaoXiang, unifying all compile-time static verification into a single **proof pipeline**. A compile-time predicate is not an external specification comment—it is a function. A function that returns Type, usable at type positions. The compiler invokes it at compile time and checks the return value. Types are propositions; compile-time evaluation is proof.

**Core thesis**: The type checker's sole task at compile time is to construct and verify proof terms. Type equality, token conflicts, dependent type reduction, compile-time predicate evaluation, Hoare logic implication—all are different type checks within the compile-time proof pipeline, sharing the same pipeline. The SMT solver is an acceleration module of the type checker, not a separate trust boundary. When the compiler returns Unproven, the programmer writes a YaoXiang function as proof—the type checker verifies it in exactly the same way it verifies any function's return type. Everything is YaoXiang code, everything is verified by the type checker.

## Motivation

### Why deprecate RFC-022?

RFC-022 designed specifications in `//!` comment form:

```yaoxiang
max: (T: Ord) -> ((arr: Array(T, n)) -> T) = {
    //! requires: NonEmpty(n) = n > 0          ← This is a comment independent of the type
    //! ensures: ExistsMax(result, arr[0..n])   ← This is a comment independent of the type
}
```

This commits a fundamental error of the Curry-Howard correspondence: **splitting specifications and types into two layers**. Comments are not types. Comments do not participate in type checking. Comments reflect the mental model of "external tools."

The whitepaper states clearly:

> "There are no `//!` comments. There is no separate specification language. Everything is within the type system."

### Current Problems

- RFC-022's `//!` comments are external syntax independent of the type system
- Specification types and ordinary types are two separate systems, causing conceptual redundancy
- The Debug Build verifies / Release Build ignores split pattern breaks unity
- The SMT solver is positioned as an external tool in conventional understanding—YaoXiang builds it in as an acceleration module of the type checker
- Type checking, borrow verification, compile-time predicate checking, and macro expansion each follow different paths

### The Correct Mental Model

Type checking can be abstracted as a function:

```
verify : Program → Proved | Disproved(Model) | Unproven
```

All compile-time checks—simple type matching, borrow conflict detection, compile-time predicate verification—are subtasks of this function. They share the same proof pipeline; the only difference lies in proof term complexity and construction strategy.

When the compiler returns Unproven, the programmer provides a proof function—the function's return type equals the proposition to be proven. The type checker verifies it. This is the same operation as ordinary type checking.

## Proposal

### 1. `{}` Is the Proof Space: Types Are Assertions, Verification Is Type Checking

YaoXiang's `{}` is the compile-time proof space. Everything inside is an assertion, and the compiler guarantees each item is True—either proven automatically or with a proof function provided by the programmer.

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
#          Parameters at signature   Only assertions inside {}
#          Compiler verifies x > 0 at call time

List: (T: Type) -> Type = { data: Array(T) }
#      ^^^^^^^^              ^^^^^^^^^^^^^^^
#      Parameters at signature   Compiler verifies type_of(T) == Type, type_of(data) == Array(T)
```

Same pattern: `name: (params) -> Type = { assertions }`. The compiler makes no distinction between "type assertions" and "value assertions"—both are evaluation targets in the proof pipeline.

**Loop invariants need not be written separately. Type annotations on variables are Floyd-Hoare invariants.**

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # Annotation references i—tells compiler s's type depends on i
    mut i: UpTo(arr.len) = 0     # At initialization i=0, verify: 0 == sum(arr[0..0]) → True
    while i < arr.len {
        s += arr[i]  # Compiler verifies: s_new == sum(arr[0..i+1])
        i += 1       # i changes → triggers re-verification of s dependency: s satisfies SumUpTo(arr, i_new)
    }
    return s  # s: SumUpTo(arr, arr.len) = sum(arr[0..arr.len])
}
```

The compiler generates a verification condition for the loop body—inductive hypothesis (type annotation) → assignment operation → whether the new value satisfies the type annotation. Once the proof pipeline verifies the inductive step, all iterations are automatically covered. No need for `: decreases`, no need for `: Invariant`, no need for inductive proofs—the compiler decomposes induction into local VCs for each assignment.

### 2. Pre/Postconditions: Compile-Time Predicates on Parameter Types and Return Types

Abandon RFC-022's `//! requires`/`//! ensures`. Compile-time predicates serve as annotations on parameters or return types.

**The parameter side is function call.** A compile-time predicate is a function returning Type, and its use on the parameter side is calling it—just like `factorial(5)`. The return value side introduces a new concept: the return value parameter.

```yaoxiang
# Precondition: Explicitly invoke the compile-time predicate in the parameter type
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b
#                       ^^^^^^^^^^  b is the current parameter name, passed to Positive as argument
#                       The compiler extracts the argument value at the call site, substitutes b, and verifies Positive(arg)
#                       Example: divide(10, 2) → verifies Positive(2) = { 2 > 0 } → True
#                       Example: divide(10, 0) → verifies Positive(0) = { 0 > 0 } → False → compile error

# Postcondition: return value parameter + compile-time predicate
IsMax: (T: Ord, arr: Array(T), result: T) -> Type = {
    forall j in 0..arr.len: result >= arr[j]
}

NonEmpty: (arr: Array(T)) -> Type = { arr.len > 0 }

max: (T: Ord) -> ((arr: NonEmpty(arr))) -> (result: IsMax(T, arr, result)) = {
#                                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
#                                            result is the return value parameter, value provided by return
#                                            The compiler substitutes the return value at the return point, verifies the postcondition
    candidate = arr[0]
    for i in 1..arr.len {
        if arr[i] > candidate { candidate = arr[i] }
    }
    return candidate
}
```

**Key rules**:

- **Parameter side**: `b: Positive(b)`—`b` is the current parameter name, passed to `Positive` as argument. Function call syntax, zero implicit.
- **Return side**: `-> (result: IsMax(T, arr, result))`—`result` is the return value parameter, its value provided by the `return` statement. `result` exists only in the type signature, only referenced by the predicate; it does not enter the function body scope, and does not appear at the caller.
- **Return value parameter is optional**: When there is no postcondition, omit it; the signature is identical to a normal function (`-> Int`).
- **Unity**: Parameters and return value parameters are the same concept—`param_name: predicate_call(param_name)`—the only difference is whether the value is provided by the caller or by `return`.

### 3. Path Condition Propagation: Compile-Time Verification of Runtime Values

When a compile-time predicate is used at a binding position, the parameters are explicitly passed in by the programmer. When a runtime value enters a refinement type parameter, the compiler completes verification through path condition collection and SMT implication judgment—no need for the programmer to explicitly pass a proof.

#### 3.1 Explicit Function Calls

When a compile-time predicate is used at a binding position, the parameters are explicitly passed in by the programmer—it is a function call, with zero implicit.

`Positive: (x: Int) -> Type = { x > 0 }` is a compile-time predicate constructor. When it appears at a binding position (parameter declaration, variable declaration, return type), the programmer explicitly passes in a bound variable name:

```yaoxiang
b: Positive(b)
// b is already declared as the current parameter, Positive(b) is a function call
// After normalization: b: { b > 0 }
```

The compiler does not need to implicitly fill in parameters—`b: Positive(b)` is the same as `f(5)`, just a function call. `b` is bound as a parameter name, and its type annotation `Positive(b)` references `b` itself—this is the standard pattern of dependent types, not an implicit expansion rule.

**Unification with RFC-010's `self`**: RFC-010 establishes that `self` is not a keyword, just a conventional parameter name ("writing it as `p`, `this`, `x` has exactly the same effect"). `b: Positive(b)` shares the same mechanism—the parameter name can be referenced in the type annotation. `self` appears in the position of `self: Point`; `b` appears in the position of `b: Positive(b)`. Both type annotations reference the parameter itself. The difference lies only in the complexity of the type annotation; the mechanism is identical—once the name is bound, the type can depend on this name.

The return type also uses explicit function calls:

```yaoxiang
Sorted: (arr: Array(T)) -> Type = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }

sort: (arr: Array(T)) -> (result: Sorted(result)) = { ... }
//                        ^^^^^^^^^^^^^^^^^^^^^^^
//                        result is the return value parameter, Sorted(result) is a function call
//                        The compiler substitutes the return value at the return point, verifies Sorted(return value)
```

The same applies to local variable declarations:

```yaoxiang
let x: Positive(x) = 5
// x bound to 5, Positive(5) → { 5 > 0 } → True → passes

// let y: Positive(y) = 0
// y bound to 0, Positive(0) → { 0 > 0 } → False → compile error
```

#### 3.2 Path Condition Collection

When a runtime value appears in a conditional branch, the compiler automatically collects path conditions, forming the current scope's **assumption set**. These assumptions participate in verification as background knowledge for compile-time Bool evaluation.

```yaoxiang
if y > 0 {
    // The compiler automatically obtains assumptions in this branch: { y > 0 }
    let result = divide(x, y)
    // Verification condition: (y > 0) ⇒ (y > 0)
    // Proof pipeline judges implication holds → Proved
} else {
    // This branch's assumption: { !(y > 0) }
    // If we want to call divide(x, y), the verification condition is !(y > 0) ⇒ y > 0
    // Proof pipeline judges implication does not hold → Disproved
}
```

This is not a hard-coded special pattern by the compiler—this is the natural behavior of the compile-time proof pipeline. Each type check call site sends to the pipeline:

```
{background assumptions} ⇒ {verification target}
```

The proof pipeline judges implication. Proved → pass, Disproved → compile error + counterexample, Unproven → compile error + unsolved proposition. Background assumptions come from the path conditions at the current program point.

#### 3.3 Assumption Stack

When analyzing control flow, the compiler maintains an assumption set for each basic block:

- **if-guard**: `if y > 0` → true branch pushes `y > 0`, false branch pushes `!(y > 0)` (if else is used)
- **match patterns**: `if let Some(v) = opt` → branch pushes `opt == Some(v)`
- **Logical conjunction**: `if x > 0 && y < 10` → branch pushes `x > 0` and `y < 10`
- **Function preconditions**: When calling `divide(a, b)`, `b` must satisfy `Positive`'s evidence—either from the current assumption, or from the argument's own refinement type annotation (if `b` is already annotated as `Positive`, its type carries `b > 0`)
- **Assignment**: When `let z = y`, the refinement conditions already on `y` propagate to `z`

All assumptions enter the compile-time proof pipeline. When entering the SMT acceleration path, they are translated to SMT-LIB background assertions.

#### 3.4 No Static Evidence Means Compile Error

If the programmer directly writes:

```yaoxiang
divide_user_input: (x: Int, y: Int) -> Int = divide(x, y)
```

There is no assumption of `y > 0` at the current program point, and the argument `y` itself has no `Positive` type annotation. The verification condition is:

```
{} ⇒ { y > 0 }
```

The pipeline returns `Disproved` (implication does not hold) → compile error:

> Cannot prove that parameter `b` satisfies `Positive` in the `divide` call.
> `y` comes from function input, with no proven bound.
> Consider guarding the call with an if branch: `if y > 0 { divide(x, y) }`.

YaoXiang does not accept runtime values entering refinement type parameters without providing static evidence. This is not a restriction—this is the core of the hard-safety philosophy. Whatever the compiler cannot statically prove cannot pass compilation.

#### 3.5 Relationship to the Unified Pipeline

Path condition propagation is not an additional mechanism. It is the direct extension of the compile-time proof pipeline over control flow analysis:

| Stage | Responsibility |
|------|------|
| Path condition collection | Compiler control flow analysis stage, annotates assumption set for each basic block |
| Verification condition generation | Upon encountering type constraints to verify, merge path conditions + argument type information |
| Proof pipeline evaluation | Compiler kernel → SMT acceleration → derive Proved / Disproved / Unproven |
| Result | `Proved` → pass; `Disproved` → compile error + counterexample; `Unproven` → compile error + unsolved proposition (programmer can provide proof function) |

No new components. No special rules. Path conditions are background knowledge for the proof pipeline—sharing the same pipeline, the same budget system, with type equality and borrow constraints.

### 4. The Compile-Time Proof Pipeline

All compile-time checks share a single pipeline. The pipeline's core operation is **type checking**—checking whether a proof term's type equals the proposition to be proven. Everything is type checking.

```
Compile-time encounters a Bool expression requiring evaluation (i.e., need to construct a proof term)
        │
        ├── Type equality (T1 == T2)
        │   → Compiler directly judges (structural equivalence)
        │
        ├── Token conflict conditions (!conflicting(tokens))
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
             │ Disproved│  → Compile error + counterexample
             │ Unproven │  → Compile error + unsolved proposition
             └────┬─────┘
                  │
                  ▼
         Programmer writes proof function (YaoXiang code)
                  │
                  ▼
         Type checker verifies ──→ Proved ──→ Compilation passes
                  │
                  ▼
            Verification fails → Compile error: "Proof does not hold"
```

#### 4.1 Proof Results: A Three-Valued Algebra

Compile-time evaluation returns three results—this is the inevitable conclusion of the halting problem, and the natural division of proof theory:

```
eval_compile_time : BoolExpr → Proved | Disproved(Model) | Unproven
```

- **Proved** → Halts, proof term constructed, type check passes. Compilation continues.
- **Disproved(M)** → Halts, counterexample M exists. Compile error + counterexample + source location.
- **Unproven** → Within given resource budget, no proof constructed. Compile error + unsolved proposition + budget consumption report.

**Unproven ≠ False.** The compiler saying "I cannot prove it" is not equivalent to the proposition being false—it merely exceeds the current automatic proof capability. This is honesty, not a defect.

The hard budget limit is the engineering solution to the halting problem. No knob is given—giving one would be asking the user "do you think your program will halt," and neither user nor compiler knows.

#### 4.2 After Unproven: The Programmer Writes the Proof

When the compiler returns Unproven, the programmer can write a **proof function**—which is a YaoXiang function whose return type equals the proposition to be proven. The type checker verifies this function—in the same way it verifies `add(a, b): Int`.

```
Proposition = Type
Proof       = Program (a value of that type)
Verification = Type checking (the sole trust root)
```

The SMT solver is not a separate trust boundary—it is an **acceleration module of the type checker**. SMT helps find proofs, but the proof is always verified by the type checker. When SMT returns `unsat`, the compiler reconstructs the result as a proof term verifiable by the type checker. If reconstruction fails (SMT's reasoning steps exceed the compiler kernel's inference rules), it falls back to Unproven—the programmer can manually write a proof function.

```yaoxiang
# Proposition: A refinement property the compiler cannot automatically prove
FirstIsMin: (T: Ord, arr: Sorted(T)) -> Type = {
    forall i in 0..arr.len: arr[0] <= arr[i]
}

# Proof: The programmer writes a function whose return type is the proposition above
# The type checker verifies this function—exactly the same as verifying add(a,b): Int
first_is_min: (T: Ord, arr: Sorted(T)) -> FirstIsMin(T, arr) = {
    # The compiler verifies here: the function body's type = FirstIsMin(T, arr)
    ...
}
```

No AI required, no export to Coq, no new concepts. **Properties the compiler cannot automatically prove → programmer writes proof in YaoXiang code → type checker verifies.** The entire process is a smooth gradient—the compiler handles simple proofs, leaving the hard ones to the brain.

#### 4.3 Layered Dependencies Within the Pipeline

The above evaluators share the same interface but have an evaluation order. Type equality is the prerequisite for all subsequent analyses; ownership/token checks depend on type information; refinement predicate verification depends on the results of the first two layers. The compiler evaluates layer by layer; expressions failing at lower layers do not enter upper layers—avoiding wasting solver budget on type-incorrect programs.

```
Evaluation order (same pipeline, layered scheduling)
├── Layer 0: Type equality (T1 == T2)
│   └── Structural unification → failure makes subsequent meaningless, return Disproved directly
├── Layer 1: Ownership/token conflicts
│   └── Flow-sensitive liveness analysis → failure means memory safety does not hold, return Disproved directly
└── Layer 2: Refinement predicates / Hoare implication
    └── Compiler itself → SMT acceleration → derive Proved / Disproved / Unproven
```

Each layer still returns `Proved/Disproved/Unproven`, sharing the same interface and the same budget system.

### 5. Unification of Three-Layer Functions

| Layer | When | Input | Output | Example |
|------|----------|------|------|------|
| Value-level function | runtime | value | value | `add: (a: Int, b: Int) -> Int = a + b` |
| Type constructor | compile-time | type/value | Type | `List: (T: Type) -> Type = { data: Array(T) }` |
| Compile-time predicate | compile-time | value | Type | `Positive: (x: Int) -> Type = { x > 0 }` |

All use the same `name: type = value` syntax. Compile-time predicates and type constructors follow the same compile-time proof pipeline—`{}` is the proof space.

### 6. Loops: Floyd-Hoare Verification Condition Generation

Loops do not need separate `: Invariant(...)` or `: decreases(...)` annotations. Compile-time predicate type annotations on variables define Floyd-Hoare-style assertions—the compiler generates verification conditions from type annotations, and the proof pipeline checks whether each assignment preserves the type.

Core mechanism: each assignment operation corresponds to a Hoare triple `{P} x := e {Q}`, with the verification condition being `P ⇒ Q[e/x]`. The compiler generates a verification condition for the loop body—once the proof pipeline verifies the inductive step, all iterations are automatically covered.

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # Annotation references i; at initialization i=0, verify: 0 == sum(arr[0..0]) → True
    mut i: UpTo(arr.len) = 0     # Verify: 0 <= 0 <= arr.len → True
    while i < arr.len {
        # The compiler generates a VC for the loop body. Premise: s satisfies SumUpTo(arr, i), i satisfies UpTo(arr.len).
        #
        # s += arr[i]:
        #   Verification obligation: s_new satisfies SumUpTo(arr, i) (current i unchanged)
        #   Substituting s_new = s_old + arr[i]:
        #     Need s_old + arr[i] == sum(arr[0..i+1])
        #     From the inductive hypothesis s_old == sum(arr[0..i]), add arr[i] to both sides:
        #     sum(arr[0..i]) + arr[i] == sum(arr[0..i+1])
        #   Compiler + SMT: linear arithmetic, millisecond-level → Proved
        #
        # i += 1:
        #   i changes → s's type annotation in the dependency graph references i → triggers re-verification
        #   New verification target: s satisfies SumUpTo(arr, i_new)
        #   i.e., s == sum(arr[0..i_new]), guaranteed by the previous step → Proved
        s += arr[i]
        i += 1
    }
    return s  # At this point s: SumUpTo(arr, arr.len), i.e., s == sum(arr[0..arr.len])
}
```

Loop invariants are the type annotations on variables—the programmer writes types, the compiler checks the inductive step. The compiler need not "discover" invariants, nor "automatically perform induction"—it decomposes inductive proofs into local verification conditions for each assignment operation, leaving them to the proof pipeline's divide-and-conquer.

#### 6.1 Dependency Tracking: Dependent Types on Mutable Variables

The prerequisite of the above mechanism is: the compiler knows that `s`'s type annotation `SumUpTo(arr, i)` references `i`—when `i` changes, `s`'s type constraint also changes. This requires the compiler to maintain a **type dependency graph between variables**.

**Data structure**:

```
TypeDepGraph: Map<VarName, Set<VarName>>
# Key is the depended-on variable, value is the set of variables whose type annotation references that variable
# Example: { i: {s}, j: {s, t}, ... }
```

**Construction**: When the type checker processes `mut v: Pred(... x ...) = init`, it parses the free variable references in `Pred(...)`'s arguments. If a reference points to another mutable variable `x` in the current scope, record `x → v` in the dependency graph.

**Triggering**: When the depended-on variable `x` is assigned, the compiler:
1. Looks up all variables in the dependency graph depending on `x`: `{v₁, v₂, ...}`
2. For each `v`, generates a verification condition: `does v's current value satisfy the updated type Pred(... x_new ...)`
3. Sends the VC into the proof pipeline

**Assignment order sensitivity**: Dependency tracking naturally enforces correct assignment order. Taking `SumUpTo(arr, i)` as an example:

```yaoxiang
# Correct order
s += arr[i]   # s_new satisfies SumUpTo(arr, i+1)
i += 1        # i changes → re-verify s satisfies SumUpTo(arr, i_new) → True

# Wrong order—compiler rejects
i += 1        # i changes → re-verify s satisfies SumUpTo(arr, i_new)
              # s not yet updated, s_old == sum(arr[0..i_old]) ≠ sum(arr[0..i_new])
              # → Compile error: variable s does not satisfy type SumUpTo(arr, i_new)
s += arr[i]   # Unreachable
```

**Combined dependencies**: A variable can depend on multiple variables. The type annotation `{ v: Int; v == x + y }` depends on both `x` and `y`—either changing triggers re-verification.

**Relationship to the proof pipeline**: Dependency tracking is the trigger for VC generation, not an independent verification mechanism. It answers "when should a VC be generated"—the proof pipeline answers "whether the VC holds."

### 7. Termination Checking

Fully automatic at compile time. Loops the compiler can prove pass; loops the compiler cannot prove directly report a compile error—the programmer must allow the compiler to automatically analyze the loop's termination. No half-automatic annotation escape hatches are provided.

#### 6.1 Design Principles

The compiler automatically extracts the information needed for termination proofs from two sources:

1. **Variable type annotations**: Boundary constraints in refinement types (e.g., `UpTo(n)` gives upper bound `n` and lower bound `0`)
2. **Loop body operations**: Operations applied to variables per iteration

The compiler attempts four metric synthesis strategies by priority; finding one stops the search.

#### 6.2 Strategy 1: Linear Ranking Function Automatic Synthesis

When variables have linear bound annotations, the compiler enumerates candidate linear metrics and verifies with SMT.

```
Input:
  Variables v₁: UpTo(u₁), v₂: UpTo(u₂), ... (variables with upper/lower bounds)
  Loop condition cond
  Set of assignments in the loop body

Algorithm:
  1. Extract each variable's bounds from type annotations: [low_i, high_i]
  2. Enumerate candidate metrics: v_i, u_i - v_i, v_i - v_j, etc. linear combinations
  3. For each candidate metric m:
     - SMT verifies m ≥ 0 (derived from type bounds)
     - For each execution path in the loop body, SMT verifies m' < m (strictly decreasing)
  4. Find a satisfying linear combination → termination proven
```

Coverage scope: Loops where arbitrary variables are assigned linear expressions (`v = a·v + b`) and have bounded type annotations. Including `i += const`, `i -= const`, and binary-search-style interval contraction:

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

#### 6.3 Strategy 2: Predicate Violation Counting—Automatically Extracting Metrics from Target Types <span style="color:orange">【Experimental Strategy】</span>

> ⚠️ **Current status: Experimental strategy, to be determined at Phase 3 implementation based on actual feasibility.**
> This strategy is effective for adjacent swap operations (bubble sort, insertion sort); it cannot automatically prove non-adjacent operations (quicksort partition, heapsort sift-down).
> Coverage boundaries are shown in the table below. If Phase 3 verification is infeasible, this strategy will be removed or downgraded to future work.

Core insight: **The specifications users write are the raw material for the compiler's reasoning.** The compiler need not have "what is sorting" built in—it reads the definition of `Sorted`, and automatically extracts metrics from the definition.

```
Input:
  Target type: Sorted(arr) = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }
  Loop body operations: adjacent element swaps

Algorithm:
  1. Parse the predicate definition: forall i in range: cond(i, arr)
  2. Automatically generate metric: violation_count = |{ i | ¬cond(i, arr) }|
  3. Analyze the operation's effect on the metric:
     - Adjacent swap arr[j], arr[j+1] = arr[j+1], arr[j]
     - Only affects the three pairs at indices j-1, j, j+1
     - If arr[j] > arr[j+1] (predicate violation), after swap this pair satisfies the predicate
     - violation_count decreases by at least 1
  4. Upper bound: n·(n-1)/2 (maximum adjacent inversions), lower bound: 0
  → Termination proven
```

**Current coverage scope**:

| Algorithm | Operation Pattern | Strategy 2 Provable? | Reason |
|------|----------|:---:|------|
| Bubble sort | Adjacent swap | ✅ | violation_count strictly decreases per swap |
| Insertion sort | Adjacent move | ✅ | Each shift eliminates one violating pair |
| Selection sort | Non-adjacent swap | ❌ | Single swap may increase violation_count |
| Quicksort | partition | ❌ | Non-adjacent swap, no monotonic decrease guarantee |
| Heapsort | sift-down | ❌ | Tree-shaped operation, violation_count non-monotonic |

**Complementary strategies**: For quicksort, the `low < high` interval contraction can be covered by Strategy 1 (linear ranking function)—the outer partition recursion halves the interval each time. Strategies 1 and 2 complementarily cover most practical algorithms' termination. However, generalizing Strategy 2 (non-adjacent operations, tree-shaped operations) remains an open problem.

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

#### 6.4 Strategy 3: Bounded Increasing/Decreasing Pattern

`v += const` (positive constant), variable has upper-bound type annotation → metric `upper_bound - v` decreases by `const` per iteration, lower bound 0. This is a degenerate case of Strategy 1, handled first by the compiler for quick processing.

#### 6.5 Strategy 4: Multiplicative Scaling Metric Template

`v *= const` (const > 1), variable has upper/lower bound type annotations. The compiler has a built-in logarithmic metric template `ceil(log_const(upper/v))`, which decreases by 1 per multiplication by const.

```yaoxiang
mut i: Positive(i) = 1
while i < n {
    # Compiler automatically derives: metric ceil(log₂(n/i)), decreases by 1 per multiplication by 2
    i *= 2
}
```

#### 6.6 Separation of Termination and Correctness

Termination proofs and correctness proofs are independent:
- **Termination**: The four strategies above automatically prove the loop exits in finite steps
- **Correctness**: Whether the loop body advances toward the target type, checked by the compile-time proof pipeline through verification conditions

Both pass → compilation passes. Termination proven but correctness fails → compile error + counterexample. Correctness proven but termination cannot be proven → compile error pointing out the unanalyzable variables or operations. Both fail → compile error reporting both failure reasons separately.

#### 6.7 Termination Checking for Recursive Functions

For recursive functions requiring compile-time evaluation, the compiler checks parameter decrease:

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)  # Compiler analysis: n-1 < n → decreasing → terminates
}

# Compile-time usage—the compiler guarantees factorial terminates at compile time
vec: Vec(factorial(5)) = Vec(120)()  # 5! = 120, completed at compile time
```

| Scenario | Behavior |
|------|------|
| Compiler can analyze recursive decrease (e.g., `n-1`) | Compile-time evaluation |
| Does not decrease / cannot determine decrease | Compile error |
| Runtime call (non-type position) | No termination check needed |

#### 6.8 Hard Boundary

`i = f(i)` where `f` is non-invertible, non-closed, and preserves no monotonicity—it is mathematically impossible to automatically prove termination. Compile error:

> This loop cannot be automatically proven to terminate. The loop variable depends on the unanalyzable function `f`. Please use an iteration pattern analyzable by the compiler.

This is not a compiler failure. Whatever cannot be statically proven safe cannot pass compilation.

### 8. SMT Solver: The Type Checker's Acceleration Module

The SMT solver is an external tool in traditional languages (e.g., F\* calls Z3, Dafny calls Z3). In YaoXiang, it is **an acceleration module of the type checker**—invoked only when the compiler kernel itself cannot directly judge. SMT helps find proofs, but the proof is verified by the type checker.

**Trust model**:
- The type checker is the sole trust root. When SMT returns `unsat`, the compiler reconstructs its reasoning trace as a proof term verifiable by the type checker kernel. If reconstruction fails (SMT's reasoning steps exceed the compiler kernel's inference rules), fall back to Unproven—the programmer can manually write a proof function.
- In Phase 1/2, as a pragmatic start, SMT `unsat` results are trusted by the compiler (this is F\*/Dafny's path—the probability of Z3 error is lower than the compiler's own bug rate). Phases 3/4 add full proof reconstruction as stronger guarantee.

**Interface**: The compiler internally translates to SMT-LIB 2.6 standard format, not bound to a specific solver API. SMT-LIB is an ISO standard, natively supported by Z3, CVC5, MathSAT, Yices.

**Default backend**: Z3 (MIT license, most extensive documentation and community validation). CVC5 as an SMT-LIB-compatible alternative—users can switch via compiler flag at compile time.

No "universal solver abstraction layer"—SMT-LIB is the abstraction layer. If CVC5 makes breakthroughs in specific theories in the future, switching requires only changing the binary, not the compiler code.

```
Compile-time Bool expression
        │
        ├── Compiler kernel can directly judge (structural equivalence, simple arithmetic,
        │   trivial formulas after constant folding)
        │   → Return Proved / Disproved directly
        │
        └── Compiler kernel cannot directly judge (quantifiers, symbolic variables)
            → Dependent type pre-reduction (factorial(5) → 120)
            → Translate to SMT-LIB format
            → Send to Z3/CVC5 (with budget limit)
            → Return value: unsat → Proved  │  sat + model → Disproved  │  unknown → Unproven
```

**Solver budget—hard limit, like stack depth**:

| Budget Dimension | Default | Description |
|----------|--------|------|
| Solving steps | 10,000 | Z3 typically completes linear arithmetic within a few hundred steps. 10,000 steps covers 99% of actual predicates. |
| Time | 100ms | A single predicate exceeding 100ms = the user is writing a compile-time program, not a type annotation. 100ms × 50 predicates = 5 second compilation time upper limit. |
| Quantifier instantiation depth | 3 | Three levels of nested quantifiers cover actual patterns. More than three levels is likely a logic exercise. |

Exceeding budget returns Unproven, compile error + predicate location + consumption. No degradation, no runtime check, no silent pass.

**Why this is actually feasible**: 95% of practical predicates in engineering are linear arithmetic—`x > 0`, `arr.len > 0`, `0 <= idx < arr.len`—all within decidable fragments, and SMT solvers return in milliseconds for such problems. For the rare complex predicates exceeding budget, the programmer writes a proof function.

Dependent types undergo a pre-reduction layer before SMT calls: `factorial(5)` directly evaluates at compile time to `120`, `append([1,2], [3])` directly evaluates to `[1,2,3]`. These deterministic value computations do not consume SMT budget.

The programmer need not know SMT exists. Mental model: **The compiler proves it or it errors—if the compiler can't, you can write a function to prove it to the compiler.**

### 9. Compile-Time Predicate Composition

Compile-time predicates are functions returning Type; composition is naturally achieved through function composition:

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

## Detailed Design

### Syntax Changes

| Before (RFC-022) | After (This RFC) |
|---|---|
| `//! requires: NonEmpty(n) = n > 0` | Compile-time predicate as parameter type `(b: Positive(b))` |
| `//! ensures: ExistsMax(result, arr)` | Return type uses return value parameter `-> (result: IsMax(T, arr, result))` |
| `/*! invariant: ... !*/` | Compile-time predicate type annotation on variables—Floyd-Hoare invariant |
| `//! decreases: n` | Compiler fully automatically derives metric function |
| Specifications are comments | Specifications are the type system |

### Syntax

**Compile-time predicates have no new keywords.** `{}` is the proof space, fully consistent with existing type definition syntax. A compile-time predicate is a function returning Type—`name: (params) -> Type = { assertions }`. Usage is a function call—`Positive(b)`, `IsMax(T, arr, result)`.

```bnf
# Compile-time predicate = function returning Type, with compiler-verified assertions inside {}
# Uses existing function/type syntax, no new BNF rules needed
predicate ::= identifier ':' params '->' 'Type' '=' '{' assertions '}'
```

**New syntactic concept: return value parameter**—in `-> (name: Type)`, `name` is the return value parameter.

The return value parameter is the **only syntactic concept** YaoXiang introduces on top of existing function syntax. Its semantics:
- `name`'s value is provided by the `return` statement
- `name` exists only in the type signature, referenced by postcondition predicates (e.g., `-> (result: IsMax(T, arr, result))`)
- `name` does not enter the function body scope, and does not appear at the caller
- The return value parameter is **optional**—when there is no postcondition, the signature is identical to a normal function (`-> Int`), introducing no extra burden

Reason for introduction: Postconditions need to reference "the value the function will return." Without a return value parameter, the compiler could only use special rules (such as the implicit variable `$result` or `__retval__`) to let predicates reference the return value. The return value parameter makes this reference explicit—it is a parameter, just with its value provided by `return` rather than the caller.

**Proof functions** are not a new concept—they are YaoXiang functions whose return type is the asserted proposition. When the compiler returns Unproven, the programmer provides a proof function, and the type checker verifies it in exactly the same way it verifies any function's return type. No new syntax, keywords, or rules required.

### Type System Impact

- **Type universe**: Compile-time predicates reside at the Type₂ layer—functions taking values and returning Type, at the same level as type constructors
- **Generics interaction**: Compile-time predicates can take generics parameters, e.g., `NonEmpty: (T: Type) -> (arr: Array(T)) -> Type`
- **Ownership interaction**: Expressions in compile-time predicates obey ownership rules; can only read, not write
- **Type inference**: Compile-time predicate parameters participate in HM type inference

### Runtime Representation

Compile-time predicates are **completely erased** at runtime. `Positive: (x: Int) -> Type = { x > 0 }`—parameter `b: Positive(b)`'s runtime representation is just `Int`. The refinement condition `{ x > 0 }` is verified by the proof pipeline at compile time, leaving no runtime trace after verification.

- `Positive(5)` → Runtime representation is `Int(5)`, refinement condition `{ 5 > 0 }` has passed, erased
- `SumUpTo(arr, 0)` → Runtime representation is `Int(0)`, equation `0 == sum(arr[0..0])` has passed, erased
- Placing compile-time predicates at type positions (e.g., `f(x: Positive(x))`) produces no wrapper types, allocates no extra memory, inserts no runtime checks

Generics erasure and refinement type erasure are the same principle: both are compile-time functions, completely disappearing after compile-time evaluation. Compile-time predicates have zero runtime overhead in Release Build—this is a direct corollary of "predicates are functions."

**Interaction constraint with `ref`**: Compile-time predicates can only reference immutably borrowed or ownership-transferred values. Compile-time predicates referencing mutably borrowed values cannot have their verification results guaranteed to still hold at runtime—directly reporting compile error for such usage.

### Compiler Changes

1. **Parser**: Compile-time predicates use standard function syntax, no extra parsing rules needed
2. **Compile-time proof pipeline**: Unified Proved/Disproved/Unproven return interface, automatic strategy selection
3. **SMT acceleration module**: SMT-LIB 2.6 translation layer, default backend Z3, CVC5 alternative
4. **Type checker kernel**: Inference rule implementation—structural equivalence, βδι-reduction, universal quantifier introduction/elimination. This is the sole trust root; SMT and programmer proofs are both verified through it
5. **Verification condition generation**: WP/SP calculus + loop invariant proof obligations
6. **Error reporting**: Counterexample formatting + unsolved proposition report + source location association

### Backward Compatibility

- ✅ Code not using compile-time predicates is completely unchanged
- ✅ Compile-time predicates have zero runtime overhead in Release Build
- ⚠️ RFC-022's `//!` syntax is no longer supported—but 022 was never implemented, with no migration burden

## Trade-offs

### Advantages

- **Curry-Howard correspondence fully realized**: Types are propositions, programs are proofs, `name: Proposition = Proof`
- **Unity**: Compile-time predicates and normal functions use completely identical syntax, no conceptual split
- **SMT transparency**: Programmers need not know SMT exists; mental model is consistent with type checking
- **Progressive adoption**: Start with one compile-time predicate, gradually increase coverage
- **Zero runtime overhead**: All evaluation completes at compile time; Release Build inserts no runtime assertions

### Disadvantages

- **Compilation time**: SMT solving increases compilation time, but hard budget limits keep the upper bound controllable
- **Automatic proof boundary**: Complex predicates beyond first-order linear arithmetic may require the programmer to write proof functions. This is not a language defect—it is the inevitable conclusion of the halting problem. The compiler honestly reports Unproven rather than falsely reporting True/False
- **Learning curve**: Writing effective compile-time predicates and proof functions requires understanding the basic intuition of the Curry-Howard correspondence
- **Implementation complexity**: Unifying the compile-time proof pipeline requires careful design

### Risk Mitigation

- SMT solving budget hard limit (steps 10,000 / time 100ms / instantiation depth 3), exceeding budget returns Unproven
- Dependent type pre-reduction: deterministic value computation consumed first, SMT only chews the non-deterministic part
- Unproven is not a dead end: programmer can write proof function, type checker verifies—consistent with verifying any function's return type
- Incremental validation: only verify changed modules
- Clear error messages + counterexample display + budget consumption report + unsolved proposition + suggestions (if the compiler can provide them)

## Alternatives

| Alternative | Why Not Chosen |
| -------------------------------- | -------------------------------------- |
| RFC-022: `//!` comment-style specifications | Specifications split from types, violates Curry-Howard correspondence |
| Separate specification files (e.g., CVL) | Specifications separate from code, increases maintenance cost |
| Runtime-only assertions | Cannot statically guarantee correctness |
| External proof assistant (e.g., Coq) | Disconnected from compiler, requires separate proof language and trust boundary. YaoXiang's choice: proof is YaoXiang code, type checker is the sole trust root |
| **This proposal: compile-time predicates as first-class citizens** | ✅ |

## Implementation Strategy

### Phase Division

| Phase | Content |
| ---------- | -------------------------------------------------------------------- |
| **Phase 1** | Compiler kernel: structural equivalence + βδι-reduction + universal quantifier introduction/elimination. Supports simple arithmetic predicates (`x > 0`, `arr.len > 0`) |
| **Phase 2** | SMT-LIB translation layer + Z3/CVC5 integration. Pipeline returns Proved/Disproved/Unproven. When Unproven, supports programmer writing proof functions |
| **Phase 3** | Loop invariant VC generation + termination checking (linear ranking function + predicate violation counting + bounded pattern + combinatorial explosion control) |
| **Phase 4** | Proof reconstruction (SMT unsat → kernel-verifiable proof term) + incremental validation + caching + counterexample formatting + IDE support |

### Dependencies

- RFC-010: Unified Type Syntax — compile-time predicates are based on `name: type = value`
- RFC-011: Generics System — compile-time predicates can take generics parameters
- RFC-009: Ownership Model — compile-time predicate expressions obey ownership rules

## Open Questions

- [x] **SMT solver choice**: Default Z3 (MIT license, most extensively validated). CVC5 as SMT-LIB-compatible alternative, switched via compiler flag. The compiler's internal translation target is SMT-LIB 2.6 standard format—SMT-LIB is the abstraction layer, no custom universal solver interface.
- [x] **Specific budget values**: Steps 10,000 / time 100ms / quantifier instantiation depth 3. Fixed internally by the compiler, no knob provided. If actual usage cases prove insufficient in practice (not "user wrote it wrong"), adjust.
- [x] **Quantifier support scope**: The language does not limit quantifier order at the language level. Compile-time predicates accept Type parameters—Type includes function types—therefore higher-order quantifiers are a natural corollary of the type system, requiring no special syntax. The SMT solver can automatically judge first-order quantifiers (forall/exists, supporting interleaved nesting, limited by budget depth 3). Higher-order quantifiers: SMT returns Unproven, compiler prompts "this predicate is beyond the scope of automatic proof, please provide a proof function." The programmer writes a YaoXiang function whose return type equals the proposition—the type checker verifies the function. No external export needed, no AI needed, no interactive proof mode needed. Everything is YaoXiang code, everything is verified by the type checker.
- [x] **Counterexample formatting**: Source variable names used directly as SMT variable names (with module prefix to avoid conflicts). When Z3 model is returned, look up by variable name. Output format: variable name = specific value + source location + predicate definition location. No complex mapping layer.
- [x] ~~**Interaction of compile-time predicates with `ref` smart pointers?**~~ → Decided: compile-time predicates only allow immutably borrowed or ownership-transferred values. Mutably borrowed values cannot appear in compile-time predicates.
- [x] **Extension of `forall` predicate violation counting metric to non-adjacent operations?** → No extension. Current coverage (adjacent swap, adjacent move) is complementarily covered by Strategy 1 (linear ranking function)—quicksort outer interval contraction is covered by Strategy 1, heapsort is covered by Strategy 1 (array index pattern). Loops that cannot have termination proven by any strategy, the compiler directly reports an error—this is the hard-safety philosophy, not a defect. If real-world scenarios (not academic constructions) emerge in the future where no strategy can cover, revisit.
- [x] **Combinatorial explosion in linear ranking function enumeration**: Candidate enumeration upper limit is 3 bounded variables. ≤3 enumerates all linear combinations and verifies one by one with SMT. >3 attempts only single-variable metrics (`v_i`, `u_i - v_i`), failure directly reports compile error—prompting the programmer "the loop has >3 bounded variables, the compiler cannot automatically synthesize multi-variable metrics." This is not an engineering compromise—it is forcing the programmer to write simpler loops.

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
│ (official)  │    │ (preserved) │
└─────────────┘    └─────────────┘
```