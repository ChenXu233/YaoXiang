---
title: "RFC-027: Compile-time Predicates and Unified Static Verification"
status: "Under Review"
author: "Chenxu"
created: "2026-06-07"
updated: "2026-06-07"
---

# RFC-027: Compile-time Predicates and Unified Static Verification

> **References**:
>
> - [RFC-009: Ownership Model](../accepted/009-ownership-model.md)
> - [RFC-010: Unified Type Syntax — `name: type = value` Model](../accepted/010-unified-type-syntax.md)
> - [RFC-011: Generics System Design](../accepted/011-generic-type-system.md)
> - [RFC-024: Concurrency Model Based on `spawn` Blocks](../accepted/024-concurrency-model.md)
>
> **Supersedes**: [RFC-022: Hoare Logic Static Verification Support (Specification Annotations and Specification Types)](../deprecated/022-hoare-logic-static-verification.md) — deprecated

## Summary

This RFC proposes introducing **compile-time predicates** as first-class citizens in YaoXiang, unifying all compile-time static verification into a single **Bool evaluation pipeline**. A compile-time predicate is not an external specification annotation — it is a function. A function that returns Bool can be used in type positions; the compiler invokes it at compile time and checks the return value. Types are propositions; compile-time evaluation is proof. The SMT solver is built into the compiler as a backend component of the compile-time evaluator. Programmers only care about "whether the compiler can prove the condition holds" — no need to learn a separate specification language.

**Core argument**: The sole job of the type checker at compile time is Bool evaluation. Type equality, token conflict, dependent type reduction, compile-time predicate evaluation, Hoare logic implication — all are compile-time Bool evaluation, sharing the same pipeline.

## Motivation

### Why Deprecate RFC-022?

RFC-022 designed specifications as `//!` annotation comments:

```yaoxiang
max: (T: Ord) -> ((arr: Array(T, n)) -> T) = {
    //! requires: NonEmpty(n) = n > 0          ← This is an annotation independent of types
    //! ensures: ExistsMax(result, arr[0..n])   ← This is an annotation independent of types
}
```

This commits the fundamental error of the Curry-Howard correspondence: **splitting specifications and types into two layers**. Annotations are not types. Annotations do not participate in type checking. Annotations belong to the mental model of "external tools."

The white paper states clearly:

> "No `//!` annotations. No separate specification language. Everything is within the type system."

### Current Problems

- RFC-022's `//!` annotations are an external syntax detached from the type system
- Specification types and ordinary types are two systems, causing conceptual redundancy
- The Debug Build verifies / Release Build ignores split undermines unity
- The SMT solver is positioned as an external tool rather than a standard compiler component
- Type checking, borrow checking, compile-time predicate checking, and macro expansion each take different paths

### The Correct Mental Model

Type checking can be abstracted as a pure function:

```
isWellTyped : Program → Bool
```

All compile-time checks — simple type matching, borrow conflict detection, compile-time predicate verification — are subtasks of this function. They share the same evaluation pipeline; the difference lies only in expression complexity and evaluation strategy.

## Proposal

### 1. `{}` Is the Solver's Verification Space: Types as Assertions, Verification as Type Checking

YaoXiang's `{}` is the solver's verification space. Everything inside is an assertion; the solver guarantees that each item is True.

```
Point: Type = { x: Float, y: Float }
#               ^^^^^^^^^^^^^^^^^^^^^  The solver guarantees x is Float, y is Float

List: (T: Type) -> Type = { data: Array(T) }
#                           ^^^^^^^^^^^^^^^  The solver guarantees data is Array(T)
```

**Generics are a special case of compile-time predicates.**

```yaoxiang
Positive: (x: Int) -> Type = { x > 0 }
#          ^^^^^^              ^^^^^^
#          parameter in signature  inside {} are only assertions
#          solver verifies x > 0 at compile-time call site

List: (T: Type) -> Type = { data: Array(T) }
#      ^^^^^^^^              ^^^^^^^^^^^^^^^
#      parameter in signature  solver verifies type_of(T) == Type, type_of(data) == Array(T)
```

Same pattern: `name: (params) -> Type = { assertions }`. The solver does not distinguish "type assertions" from "value assertions."

**Loop invariants need not be written separately. Type annotations on variables are Floyd-Hoare invariants.**

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, 0) = 0   # Solver verifies: 0 == sum(arr[0..0])  → True
    mut i: UpTo(arr.len) = 0     # Solver verifies: 0 <= 0 <= arr.len   → True
    while i < arr.len {
        s += arr[i]  # Solver verifies: s_new == sum(arr[0..i+1])
        i += 1       # Solver verifies: i+1 <= arr.len maintained
    }
    return s  # s: SumUpTo(arr, arr.len) = sum(arr[0..arr.len])
}
```

The compiler generates one verification condition for the loop body — induction hypothesis (type annotation) → assignment operation → whether the new value satisfies the type annotation. After the SMT solver proves the induction step, all iterations are covered automatically. No `: decreases`, no `: Invariant`, no inductive proof needed — the compiler decomposes induction into per-assignment local VCs.

### 2. Pre-/Post-conditions: Compile-time Predicates on Parameter Types and Return Types

Abandon RFC-022's `//! requires`/`//! ensures`. Compile-time predicates act as parameter or return type annotations:

```yaoxiang
# Pre-condition: parameter type contains a compile-time predicate
divide: (a: Int, b: Positive) -> Int = a / b
#                       ^^^^^^^^  The solver verifies { b > 0 } at the call site

# Post-condition: return type is a compile-time predicate
IsMax: (T: Ord, arr: Array(T), result: T) -> Type = {
    forall j in 0..arr.len: result >= arr[j]
}

max: (T: Ord) -> ((arr: NonEmpty(Array(T))) -> IsMax(T, arr)) = {
    #                                           ^^^^^^^^^^^^^^
    #  IsMax(T, arr) partially applies arr, returns (result: T) -> Type
    #  Solver verifies { forall j: result >= arr[j] } at return point
    result = arr[0]
    for i in 1..arr.len {
        if arr[i] > result { result = arr[i] }
    }
    return result
}
```

### 3. Path Condition Propagation: Compile-time Verification of Runtime Values

When a compile-time predicate is used in a type annotation position, the parameter is automatically filled with the annotated variable itself. When a runtime value enters a refinement type parameter, the compiler completes verification via path condition collection and SMT implication checking — no need for the programmer to pass proofs explicitly.

#### 3.1 Implicit Self-reference

`Positive: (x: Int) -> Type = { x > 0 }` is a compile-time predicate constructor. When it appears in a binding position (parameter declaration, variable declaration, return type), the compiler automatically fills the bound variable into the parameter:

```yaoxiang
b: Positive
// Compiler expands: b: Positive(b)
// After normalization: b: { b > 0 }
```

The user does not need to write `b: Positive(b)` — that would be circular syntax. Implicit self-reference is the standard expansion rule for refinement types in binding positions. The compile-time predicate constructor itself remains a pure function that accepts `Int` and returns `Type`.

The same implicit self-reference applies to return types:

```yaoxiang
Sorted: (T: Ord, arr: Array(T)) -> Type = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }

sort: (T: Ord) -> ((arr: Array(T)) -> Sorted(T)) = { ... }
//                                            ^^^^^^^^^^
//  Sorted(T) partially applies T, return type is (arr: Array(T)) -> Type
//  Compiler fills the return value into the remaining parameter arr → verifies Sorted(T, return_value)
```

#### 3.2 Path Condition Collection

When a runtime value appears in a conditional branch, the compiler automatically collects path conditions, forming the **assumption set** for the current scope. These assumptions participate in verification as background knowledge for compile-time Bool evaluation.

```yaoxiang
if y > 0 {
    // Compiler automatically acquires assumption in this branch: { y > 0 }
    let result = divide(x, y)
    // Verification condition: (y > 0) ⇒ (y > 0)
    // SMT solver returns unsat (implication holds) → pass
} else {
    // This branch's assumption: { !(y > 0) }
    // If divide(x, y) is called, verification condition is !(y > 0) ⇒ y > 0
    // SMT returns sat (implication does not hold) → compile error
}
```

This is not the compiler hard-coding special patterns — it is the natural behavior of the compile-time Bool evaluation pipeline. At each type-check call site, the compiler sends to the SMT:

```
{background assumptions} ⇒ {verification goal}
```

The SMT solver checks implication. Implication holds → pass; implication does not hold → compile error. Background assumptions come from the current program point's path conditions.

#### 3.3 Assumption Stack

When analyzing control flow, the compiler maintains an assumption set for each basic block:

- **if-guard**: `if y > 0` → true branch pushes `y > 0`; false branch pushes `!(y > 0)` (if else is used)
- **match pattern**: `if let Some(v) = opt` → branch pushes `opt == Some(v)`
- **logical conjunction**: `if x > 0 && y < 10` → branch pushes `x > 0` and `y < 10`
- **function pre-condition**: when calling `divide(a, b)`, evidence that `b` satisfies `Positive` must come from either current assumptions or the argument's own refinement type annotation (if `b` is annotated as `Positive`, its type carries `b > 0`)
- **assignment**: when `let z = y`, the refinement condition already on `y` propagates to `z`

All assumptions enter the compile-time evaluation pipeline and are translated into SMT-LIB background assertions when sent to the SMT solver.

#### 3.4 No Static Evidence → Compile Error

If the programmer directly writes:

```yaoxiang
divide_user_input: (x: Int, y: Int) -> Int = divide(x, y)
```

There is no `y > 0` assumption at the current program point, and the argument `y` itself has no `Positive` type annotation. The verification condition is:

```
{} ⇒ { y > 0 }
```

SMT returns `sat` (implication does not hold) → compile error:

> Cannot prove that parameter `b` satisfies `Positive` in the call to `divide`.
> `y` comes from function input, with no proven bound.
> Consider guarding the call with an `if` branch: `if y > 0 { divide(x, y) }`.

YaoXiang does not accept runtime values directly entering refinement type parameters without providing static evidence. This is not a limitation — it is the core of the hard-safety philosophy. Any code the compiler cannot statically prove is not allowed to pass compilation.

#### 3.5 Relationship with the Unified Pipeline

Path condition propagation is not an additional mechanism. It is the direct extension of the compile-time Bool evaluation pipeline into control flow analysis:

| Stage | Responsibility |
|------|------|
| Path condition collection | Compiler's control flow analysis stage, annotating each basic block with its assumption set |
| Verification condition generation | When a type constraint needs verification, merge path conditions + argument type information |
| Bool evaluation | The compiler itself or the SMT solver determines implication |
| Result | `True` → pass; `False` → compile error + counterexample; `budget exhausted` → compile error + location |

No new components. No special rules. Path conditions are the background knowledge for Bool evaluation — sharing the same pipeline and the same budget system as type equality and borrow constraints.

### 4. The Compile-time Bool Evaluation Pipeline

All compile-time checks share the same pipeline:

```
Compile-time encounters a Bool expression that needs evaluation
        │
        ├── Type equality (T1 == T2)
        │   → Compiler directly determines
        │
        ├── Token conflict condition (!conflicting(tokens))
        │   → Flow-sensitive liveness analysis (Dup/Linear property tracking)
        │
        ├── Dependent type reduction (n + m simplification)
        │   → Compile-time term rewriting system
        │
        ├── Compile-time predicate (x > 0, forall...)
        │   → Compiler itself + SMT solver
        │
        └── Hoare logic implication (P ⇒ Q)
            → SMT solver
                    │
                    ▼
             ┌──────────┐
             │ True     │  → Compilation passes
             │ False    │  → Compile error + counterexample
             │ Budget   │  → Compile error + predicate location + consumption
             │ exceeded │
             └──────────┘
```

The compile-time Bool evaluation result has only three outcomes — they are **the three possible results of a single evaluation operation**:

```
eval_compile_time : BoolExpr → True | False | BudgetExhausted
```

This is not a design choice; it is the necessary conclusion of the halting problem. The compiler evaluates arbitrary Bool expressions — Rice's theorem says this is undecidable. The compile-time evaluator is also a program, facing the same halting problem. So results must take three paths:

- `True` → halts, answer is true
- `False` → halts, answer is false
- `Budget exhausted` → did not halt within the given resource limit

Hard budget limits are the engineering solution to the halting problem. No knobs — giving knobs would mean asking the user "do you think your program will halt," and neither the user nor the compiler knows.

**Key design**:
- **Three outcomes from one source**: `True`/`False`/`Budget exhausted` are three exits of a single evaluation pipeline
- **Hard budget limits**: step count, time, quantifier instantiation depth — fixed inside the compiler, no knobs
- **Automatic strategy selection**: the compiler automatically selects the evaluation strategy based on expression complexity, deciding directly when it can, calling SMT only when it cannot
- **Engineering reality**: the vast majority of predicates (`x > 0`, `arr.len > 0`, `0 <= idx < arr.len`) are linear arithmetic, and SMT returns True/False in milliseconds. Budget exhaustion occurs only when the user writes predicates that exceed decidable fragments

**Layered dependencies within the pipeline**: The above evaluators share the same interface but have an evaluation order. Type equality is the prerequisite for all subsequent analysis; ownership/token checks depend on type information; refinement predicate verification depends on the results of the first two layers. The compiler evaluates layer by layer; expressions that fail at lower layers do not enter upper layers — avoiding wasting SMT solver budget on programs with type errors.

```
Evaluation order (same pipeline, layered scheduling)
├── Layer 0: Type equality (T1 == T2)
│   └── Structural unification → if it fails, subsequent steps are meaningless, return False directly
├── Layer 1: Ownership/token conflict
│   └── Flow-sensitive liveness analysis → if it fails, memory safety does not hold, return False directly
└── Layer 2: Refinement predicate / Hoare implication
    └── Compiler + SMT → False (counterexample) or budget exhausted
```

Each layer still returns `True`/`False`/`Budget exhausted`, sharing the same interface and the same budget system.

### 5. Three-Layer Function Unification

| Layer | Execution Time | Input | Output | Example |
|------|----------|------|------|------|
| Value-level function | runtime | value | value | `add: (a: Int, b: Int) -> Int = a + b` |
| Type constructor | compile-time | type/value | Type | `List: (T: Type) -> Type = { data: Array(T) }` |
| Compile-time predicate | compile-time | value | Type | `Positive: (x: Int) -> Type = { x > 0 }` |

All use the same `name: type = value` syntax. Compile-time predicates and type constructors go through the same compile-time evaluation pipeline — `{}` is the solver's verification space.

### 6. Loops: Floyd-Hoare Verification Condition Generation

Loops do not need separate `: Invariant(...)` or `: decreases(...)` annotations. The compile-time predicate type annotations on variables define Floyd-Hoare style assertions — the compiler generates verification conditions from type annotations, and the SMT solver checks whether each assignment maintains the type.

Core mechanism: each assignment operation corresponds to a Hoare triple `{P} x := e {Q}`, with verification condition `P ⇒ Q[e/x]`. The compiler generates one verification condition for the loop body — after the SMT solver proves the induction step, all iterations are covered automatically.

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, 0) = 0   # Verify: 0 == sum(arr[0..0])  → True
    mut i: UpTo(arr.len) = 0     # Verify: 0 <= 0 <= arr.len   → True
    while i < arr.len {
        # Compiler generates one VC for the loop body:
        #
        # s += arr[i]:
        #   Precondition (induction hypothesis): s == sum(arr[0..i])
        #   Verification obligation: s_new == sum(arr[0..i+1])
        #   Substituting s_new = s_old + arr[i]:
        #     s_old + arr[i] == sum(arr[0..i]) + arr[i] == sum(arr[0..i+1])
        #   SMT solver: linear arithmetic, millisecond response → True
        #
        # i += 1:
        #   Precondition: i < arr.len (loop condition) + i >= 0 (type bound)
        #   Verification obligation: i+1 >= 0 and i+1 <= arr.len
        #   SMT: derived directly from preconditions → True
        s += arr[i]
        i += 1
    }
    return s  # At this point s: SumUpTo(arr, arr.len), i.e., s == sum(arr[0..arr.len])
}
```

Loop invariants are the type annotations on variables — the programmer writes the type, the compiler checks the induction step. The compiler does not need to "discover" invariants, nor does it need to "perform induction automatically" — it decomposes the inductive proof into local verification conditions per assignment and lets the SMT solver divide and conquer.

### 7. Termination Checking

Fully automatic at compile time. The compiler proves a loop terminates or directly reports a compile error — programmers must make loops auto-analyzable by the compiler. No half-automatic annotation escape hatch.

#### 6.1 Design Principles

The compiler automatically extracts information for termination proofs from two sources:

1. **Variable type annotations**: boundary constraints in refinement types (e.g., `UpTo(n)` provides upper bound `n` and lower bound `0`)
2. **Loop body operations**: operations applied to variables on each iteration

The compiler tries four measure synthesis strategies in priority order, stopping when one succeeds.

#### 6.2 Strategy 1: Linear Ranking Function Auto-synthesis

When a variable has a linear bound annotation, the compiler enumerates candidate linear measures and verifies with SMT.

```
Input:
  Variables v₁: UpTo(u₁), v₂: UpTo(u₂), ... (with upper and lower bounds)
  Loop condition cond
  Set of assignments in the loop body

Algorithm:
  1. Extract each variable's bounds from type annotations: [low_i, high_i]
  2. Enumerate candidate measures: v_i, u_i - v_i, v_i - v_j, and other linear combinations
  3. For each candidate measure m:
     - SMT verifies m ≥ 0 (derived from type bounds)
     - For each execution path of the loop body, SMT verifies m' < m (strictly decreasing)
  4. Find a qualifying linear combination → termination proved
```

Coverage scope: any loop where a variable is assigned to a linear expression (`v = a·v + b`) and has bounded type annotations. Including `i += const`, `i -= const`, and interval-shrinking patterns like binary search:

```yaoxiang
# Binary search: low = mid + 1 or high = mid
# The measure high - low strictly decreases on both paths
binary_search: (arr: Sorted(Array(Int)), key: Int) -> Option(Int) = {
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

#### 6.3 Strategy 2: Predicate Violation Count — Auto-extract Measures from Target Types

When the loop's return type is a `forall` refinement predicate and the loop body contains local modifications to data, the compiler auto-generates a measure from the predicate definition.

Core insight: **the user's specification is the raw material for compiler reasoning.** The compiler does not need a built-in notion of "what is sorting" — it reads the definition of `Sorted` and auto-extracts a measure from that definition.

```
Input:
  Target type: Sorted(arr) = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }
  Loop body operations: adjacent-element swaps

Algorithm:
  1. Parse the predicate definition: forall i in range: cond(i, arr)
  2. Auto-generate measure: violation_count = |{ i | ¬cond(i, arr) }|
  3. Analyze the operation's effect on the measure:
     - Adjacent swap arr[j], arr[j+1] = arr[j+1], arr[j]
     - Affects only index pairs j-1, j, j+1
     - If arr[j] > arr[j+1] (violates predicate), the pair satisfies the predicate after the swap
     - violation_count decreases by at least 1
  4. Upper bound: n·(n-1)/2 (maximum number of adjacent inversions), lower bound: 0
  → Termination proved
```

```yaoxiang
sort: (arr: Array(Int)) -> Sorted(arr) = {
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

`v += const` (positive constant), variable has an upper-bound type annotation → measure `upper_bound - v` decreases by `const` each time, lower bound 0. This is a degenerate case of Strategy 1, processed first by the compiler for speed.

#### 6.5 Strategy 4: Multiplicative Scaling Measure Template

`v *= const` (const > 1), variable has upper and lower bound type annotations. The compiler has a built-in logarithmic measure template `ceil(log_const(upper/v))`; each multiplication by `const` decreases the measure by 1.

```yaoxiang
mut i: Positive = 1
while i < n {
    # Compiler auto-derives: measure ceil(log₂(n/i)), each multiplication by 2 decreases the measure by 1
    i *= 2
}
```

#### 6.6 Separation of Termination and Correctness

Termination proofs and correctness proofs are independent:
- **Termination**: the four strategies above automatically prove the loop exits in a finite number of steps
- **Correctness**: whether the loop body advances toward the target type, checked by the SMT solver via verification conditions

Both pass → compilation passes. Termination proved but correctness fails → compile error + counterexample. Termination cannot be proved → compile error, indicating which variable or operation cannot be analyzed.

#### 6.7 Termination Checking for Recursive Functions

For recursive functions that need compile-time evaluation, the compiler checks whether the parameter decreases:

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)  # Compiler analyzes: n-1 < n → decreasing → terminates
}

# Compile-time use — compiler guarantees factorial terminates at compile time
vec: Vec(factorial(5)) = Vec(120)()  # 5! = 120, completed at compile time
```

| Scenario | Behavior |
|------|------|
| Compiler can analyze recursive decrease (e.g., `n-1`) | Compile-time evaluation |
| Does not decrease / cannot determine decrease | Compile error |
| Runtime call (not in type position) | No termination check needed |

#### 6.8 Hard Boundary

`i = f(i)` where `f` is non-invertible, non-closed, and does not preserve any monotonicity — mathematically impossible to automatically prove termination. Compile error:

> This loop cannot be automatically proved to terminate. The loop variable depends on the non-analyzable function `f`. Please use an iteration pattern analyzable by the compiler.

This is not the compiler's failure. Any code that cannot be statically proved safe is not allowed to pass compilation.

### 8. SMT Solver: A Backend Component of the Compile-time Evaluator

In traditional languages, the SMT solver is an external tool (e.g., F\* calls Z3, Dafny calls Z3). In YaoXiang, it is a backend component of the compile-time evaluation pipeline — invoked only when the compiler itself cannot directly determine the result.

```
Compile-time Bool expression
        │
        ├── Compiler can determine directly (simple comparisons, simple arithmetic,
        │   trivial formulas after constant folding)
        │   → Return True/False directly
        │
        └── Compiler cannot determine directly (quantifiers, symbolic variables)
            → Dependent type pre-reduction (factorial(5) → 120)
            → Translate to SMT-LIB format
            → Send to Z3/CVC5 (with budget limit)
            → Return: unsat (holds) │ sat (counterexample) │ budget_exceeded
```

**Solver budget — hard limit, like stack depth**:

| Budget Dimension | Description | Hard Upper Limit |
|----------|------|--------|
| Solver steps | term rewriting / resolution steps | fixed value internal to compiler |
| Time | single-predicate solver time | fixed value internal to compiler |
| Quantifier instantiation depth | forall/exists expansion layers | fixed value internal to compiler |

Exceeding budget means compile error. No degradation, no runtime check, no silent pass. Error messages are precise, pinpointing the predicate's source location and reporting how much budget was consumed.

**Why this works in practice**: In engineering, 95% of real predicates are linear arithmetic — `x > 0`, `arr.len > 0`, `0 <= idx < arr.len` — all within decidable fragments; the SMT solver returns answers for these in milliseconds. Encountering the rare complex predicates that exceed the budget, the user simply splits the predicate into simpler pieces.

Dependent types undergo a pre-reduction layer before the SMT call: `factorial(5)` is directly computed to `120` at compile time; `append([1,2], [3])` is directly computed to `[1,2,3]`. These deterministic value computations do not consume SMT budget.

Programmers do not need to know that SMT exists. The mental model is: **if the compiler can prove it, it passes; if it cannot, it errors** — exactly the same as type checking.

### 9. Compile-time Predicate Composition

A compile-time predicate is a function returning `Type`; composition is naturally achieved through function composition:

```yaoxiang
SortedNonEmpty: (T: Ord, arr: Array(T)) -> Type = {
    Sorted(T, arr) && NonEmpty(T, arr)
}
```

### 10. Code Examples

#### 9.1 Safe Division

```yaoxiang
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive) -> Int = a / b

result = divide(10, 2)   # ✅ Solver verifies { 2 > 0 } → True
# result = divide(10, 0)  # ❌ Solver verifies { 0 > 0 } → False
```

#### 9.2 Safe Array Access

```yaoxiang
InBounds: (T: Type, idx: Int, arr: Array(T)) -> Type = { 0 <= idx && idx < arr.len }

get: (T: Type) -> ((arr: Array(T), idx: InBounds(T, arr)) -> T) = arr.data[idx]

arr = Array(Int)(1, 2, 3)
x = get(arr, 1)   # ✅ Solver verifies { 0 <= 1 && 1 < 3 } → True
# y = get(arr, 5)  # ❌ Solver verifies { 0 <= 5 && 5 < 3 } → False
```

#### 9.3 Sorting Correctness

```yaoxiang
Sorted: (T: Ord, arr: Array(T)) -> Type = {
    forall i in 0..arr.len-1: arr[i] <= arr[i+1]
}

sort: (T: Ord) -> ((arr: Array(T)) -> Sorted(T)) = {
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
    mut s: SumUpTo(arr, 0) = 0
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
| `//! requires: NonEmpty(n) = n > 0` | Compile-time predicate as parameter type `(b: Positive)` |
| `//! ensures: ExistsMax(result, arr)` | Return type is a compile-time predicate `-> IsMax(T, arr)` |
| `/*! invariant: ... !*/` | Compile-time predicate type annotation on variable — Floyd-Hoare invariant |
| `//! decreases: n` | Compiler fully auto-derives the measure function |
| Specification is an annotation | Specification is the type system |

### Syntax

**Compile-time predicates have no new syntax.** `{}` is the solver's verification space, fully consistent with the existing type definition syntax. A compile-time predicate is a function returning `Type` — `name: (params) -> Type = { assertions }`.

```bnf
# Compile-time predicate = function returning Type, {} contains solver assertions
# Uses existing function/type syntax, no new BNF rules needed
predicate ::= identifier ':' params '->' 'Type' '=' '{' assertions '}'
```

### Type System Impact

- **Type universe**: Compile-time predicates live at the Type₂ layer — functions that accept values and return Type, at the same level as type constructors
- **Generics interaction**: Compile-time predicates may take generics parameters, e.g., `NonEmpty: (T: Type) -> (arr: Array(T)) -> Type`
- **Ownership interaction**: Expressions in compile-time predicates obey ownership rules — read-only, no write
- **Type inference**: Parameters of compile-time predicates participate in HM type inference

### Runtime Representation

Compile-time predicates are **completely erased** at runtime. `Positive: (x: Int) -> Type = { x > 0 }` — the parameter `b: Positive` is represented as `Int` at runtime. The refinement condition `{ x > 0 }` is verified by the SMT solver at compile time; once verified, it leaves no runtime trace.

- `Positive(5)` → runtime representation is `Int(5)`, refinement condition `{ 5 > 0 }` has passed, erased
- `SumUpTo(arr, 0)` → runtime representation is `Int(0)`, equality `0 == sum(arr[0..0])` has passed, erased
- Placing a compile-time predicate in a type position (e.g., `f(x: Positive)`) does not produce any wrapper type, does not allocate extra memory, and does not insert runtime checks

Generics erasure and refinement type erasure are the same principle: both are compile-time functions that disappear completely after compile-time evaluation. Compile-time predicates have zero runtime overhead in Release Build — this follows directly from "predicates are functions."

**Interaction constraint with `ref`**: Compile-time predicates may only reference immutable borrows or values whose ownership has been transferred. A compile-time predicate that references a mutable-borrowed value cannot be guaranteed by the compiler to remain valid at runtime — such usage directly reports a compile error.

### Compiler Changes

1. **Parser**: Compile-time predicates use standard function syntax, no additional parsing rules needed
2. **Compile-time evaluation pipeline**: Unified Bool return interface, automatic strategy selection
3. **SMT backend**: Integrate Z3/CVC5, SMT-LIB format translation
4. **Verification condition generation**: WP/SP calculus + loop invariant proof obligations
5. **Error reporting**: Counterexample formatting, linking to source locations

### Backward Compatibility

- ✅ Code that does not use compile-time predicates remains completely unchanged
- ✅ Compile-time predicates have zero runtime overhead in Release Build
- ⚠️ RFC-022's `//!` syntax is no longer supported — but 022 was never implemented, so there is no migration burden

## Trade-offs

### Advantages

- **Curry-Howard correspondence fully realized**: types are propositions, programs are proofs, `name: Proposition = Proof`
- **Unity**: Compile-time predicates and ordinary functions use exactly the same syntax, no conceptual split
- **SMT transparency**: Programmers do not need to know SMT exists; the mental model matches type checking
- **Progressive adoption**: Start with one compile-time predicate, gradually expand coverage
- **Zero runtime overhead**: All evaluation completes at compile time; Release Build inserts no runtime assertions

### Disadvantages

- **Compile time**: SMT solving increases compile time, but hard budget limits keep the upper bound controllable
- **SMT capability boundary**: Complex predicates beyond linear arithmetic may exceed the budget; users need to split predicates
- **Learning curve**: Writing effective compile-time predicates requires some experience
- **Implementation complexity**: Unifying the compile-time evaluation pipeline requires careful design

### Risk Mitigation

- Hard SMT solver budget limits (steps / time / instantiation depth); exceeding budget means compile error
- Dependent type pre-reduction: deterministic value computations are eaten first; SMT only tackles the non-deterministic part
- Incremental verification: only verify changed modules
- Clear error messages + counterexample display + budget consumption report

## Alternatives

| Approach                           | Why Not Chosen                           |
| -------------------------------- | -------------------------------------- |
| RFC-022: `//!` annotation-style specifications | Specification and type split, violates Curry-Howard correspondence |
| Separate specification files (e.g., CVL)           | Specification separated from code, increases maintenance cost           |
| Runtime assertions only                     | Cannot statically guarantee correctness                     |
| External proof assistants (e.g., Coq)           | Steep learning curve, disconnected from the compiler             |
| **This approach: Compile-time predicates as first-class citizens** | ✅                                     |

## Implementation Strategy

### Phases

| Phase       | Content                                                                 |
| ---------- | -------------------------------------------------------------------- |
| **Phase 1** | Compiler recognizes that functions returning Bool can be used in type positions. Supports simple arithmetic predicates (`x > 0`, `arr.len > 0`) |
| **Phase 2** | Compile-time Bool evaluation pipeline + Z3 integration. Supports `forall`/`exists` quantifiers          |
| **Phase 3** | Loop invariant VC generation + termination checking (linear ranking function + predicate violation count + bounded pattern) |
| **Phase 4** | Incremental verification + caching + counterexample formatting + IDE support |

### Dependencies

- RFC-010: Unified Type Syntax — compile-time predicates are built on `name: type = value`
- RFC-011: Generics System — compile-time predicates can take generics parameters
- RFC-009: Ownership Model — expressions in compile-time predicates obey ownership rules

## Open Questions

- [ ] SMT solver choice: Z3 or CVC5? Or a pluggable backend?
- [ ] Specific values for solver budget: step count upper limit, time upper limit, quantifier instantiation depth upper limit
- [ ] Scope of quantifier support: nested quantifiers? Higher-order quantifiers?
- [ ] Counterexample formatting: how to map SMT models back to source variables?
- [x] ~~Compile-time predicate interaction with `ref` smart pointers?~~ → Decided: compile-time predicates may only use immutable borrows or values whose ownership has been transferred. Mutable-borrowed values cannot appear in compile-time predicates
- [ ] Extension of the `forall` predicate violation count measure to non-adjacent operations (e.g., quicksort partition)?
- [ ] How to control the combinatorial explosion of linear ranking function enumeration with more than 3 bounded variables?

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
│ Under Review│  ← Current status: community discussion
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