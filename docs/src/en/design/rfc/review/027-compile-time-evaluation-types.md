---
title: "RFC-027: Compile-Time Predicates and Unified Static Verification"
status: "Under Review"
author: "Chen Xu"
created: "2026-06-07"
updated: "2026-06-07"
---

# RFC-027: Compile-Time Predicates and Unified Static Verification

> **References**:
>
> - [RFC-009: Ownership Model](../accepted/009-ownership-model.md)
> - [RFC-010: Unified Type Syntax - name: type = value Model](../accepted/010-unified-type-syntax.md)
> - [RFC-011: Generic Type System Design](../accepted/011-generic-type-system.md)
> - [RFC-024: Concurrency Model Based on Spawn Blocks](../accepted/024-concurrency-model.md)
>
> **Supersedes**: [RFC-022: Hoare Logic Static Verification Support (Specification Comments and Refinement Types)](../deprecated/022-hoare-logic-static-verification.md) — Deprecated

## Abstract

This document proposes introducing **compile-time predicates** as first-class citizens into YaoXiang, unifying all compile-time static verification into a single **Bool evaluation pipeline**. Compile-time predicates are not external specification comments—they are functions. A function that returns Bool can be used in type positions; the compiler invokes it at compile time and checks the return value. Types are propositions, compile-time evaluation is proof. The SMT solver is built in as a backend component of the compile-time evaluator—programmers only care about "can the compiler prove this condition," without learning a separate specification language.

**Core thesis**: The sole job of type checking at compile time is Boolean evaluation. Type equality, token conflicts, dependent type reduction, compile-time predicate evaluation, and Hoare logic implication—all are compile-time Bool evaluation, sharing the same pipeline.

## Motivation

### Why Deprecate RFC-022?

RFC-022 designed specifications as `//!` comment syntax:

```yaoxiang
max: (T: Ord) -> ((arr: Array(T, n)) -> T) = {
    //! requires: NonEmpty(n) = n > 0          ← This is a comment independent of the type
    //! ensures: ExistsMax(result, arr[0..n])   ← This is a comment independent of the type
}
```

This commits the fundamental error of Curry-Howard isomorphism: **splitting specifications and types into two layers**. Comments are not types. Comments do not participate in type checking. Comments are the mental model of an "external tool."

The whitepaper states clearly:

> "No `//!` comments. No separate specification language. Everything is within the type system."

### Current Problems

- RFC-022's `//!` comments are external syntax independent of the type system
- Specification types and ordinary types are two separate systems, creating conceptual redundancy
- The split pattern of Debug Build verification / Release Build ignoring destroys uniformity
- The SMT solver is positioned as an external tool rather than a standard compiler component
- Type checking, borrow verification, compile-time predicate checking, and macro expansion each follow different paths

### The Correct Mental Model

Type checking can be abstracted as a pure function:

```
isWellTyped : Program → Bool
```

All compile-time checks—simple type matching, borrow conflict detection, compile-time predicate verification—are subtasks of this function. They share the same evaluation pipeline, differing only in expression complexity and evaluation strategy.

## Proposal

### 1. `{}` Is the Solver's Verification Space: Types Are Assertions, Verification Is Type Checking

YaoXiang's `{}` is the solver's verification space. Everything inside is an assertion, and the solver guarantees each item is True.

```
Point: Type = { x: Float, y: Float }
#               ^^^^^^^^^^^^^^^^^^^^^  Solver guarantees x is Float, y is Float

List: (T: Type) -> Type = { data: Array(T) }
#                           ^^^^^^^^^^^^^^^  Solver guarantees data is Array(T)
```

**Generics are a special case of compile-time predicates.**

```yaoxiang
Positive: (x: Int) -> Type = { x > 0 }
#          ^^^^^^              ^^^^^^
#          Parameter in signature  Assertion inside {}
#          Solver verifies x > 0 at compile-time invocation

List: (T: Type) -> Type = { data: Array(T) }
#      ^^^^^^^^              ^^^^^^^^^^^^^^^
#      Parameter in signature   Solver verifies type_of(T) == Type, type_of(data) == Array(T)
```

The same pattern: `name: (params) -> Type = { assertion }`. The solver does not distinguish between "type assertions" and "value assertions."

**Loop invariants don't need to be written separately. Type annotations on variables are Floyd-Hoare invariants.**

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

The compiler generates verification conditions for the loop body once—inductive hypothesis (type annotation) → assignment operation → whether the new value satisfies the type annotation. Once the SMT solver proves the inductive step, all iterations are automatically covered. No `: decreases`, no `: Invariant`, no inductive proof needed—the compiler decomposes induction into local VCs for each assignment.

### 2. Pre/Postconditions: Compile-Time Predicates on Parameter Types and Return Types

Abandon RFC-022's `//! requires`/`//! ensures`. Compile-time predicates serve as type annotations on parameters or return values:

```yaoxiang
# Precondition: Parameter type includes compile-time predicate
divide: (a: Int, b: Positive) -> Int = a / b
#                       ^^^^^^^^  Solver verifies { b > 0 } at call site

# Postcondition: Return type is a compile-time predicate
IsMax: (T: Ord, arr: Array(T), result: T) -> Type = {
    forall j in 0..arr.len: result >= arr[j]
}

max: (T: Ord) -> ((arr: NonEmpty(Array(T))) -> IsMax(T, arr)) = {
    #                                           ^^^^^^^^^^^^^^
    #  IsMax(T, arr) partially applies arr, returning (result: T) -> Type
    #  Solver verifies { forall j: result >= arr[j] } at return point
    result = arr[0]
    for i in 1..arr.len {
        if arr[i] > result { result = arr[i] }
    }
    return result
}
```

### 3. Compile-Time Bool Evaluation Pipeline

All compile-time checks share the same pipeline:

```
Compile-time encounters Bool expression requiring evaluation
        │
        ├── Type equality (T1 == T2)
        │   → Compiler decides directly
        │
        ├── Token conflict conditions (!conflicting(tokens))
        │   → Flow-sensitive liveness analysis (Dup/Linear attribute tracking)
        │
        ├── Dependent type reduction (n + m simplification)
        │   → Compile-time term rewriting system
        │
        ├── Compile-time predicates (x > 0, forall...)
        │   → Compiler itself + SMT solver
        │
        └── Hoare logic implications (P ⇒ Q)
            → SMT solver
                    │
                    ▼
             ┌──────────┐
             │ True     │  → Compilation passes
             │ False    │  → Compilation error + counterexample
             │ Out of budget │  → Compilation error + predicate location + consumption
             └──────────┘
```

Compile-time Bool evaluation has exactly three possible results—these are **three possible outcomes of the same evaluation operation**:

```
eval_compile_time : BoolExpr → True | False | Budget Exhausted
```

This is not a design choice; it is the inevitable conclusion of the halting problem. The compiler evaluating an arbitrary Bool expression—the compiler, as a program, faces the same halting problem. So the result must have three paths:

- `True` → Halts, answer is true
- `False` → Halts, answer is false
- `Budget exhausted` → Did not halt within given resource limits

Hard budget limits are the engineering solution to the halting problem. No knobs are given—giving knobs means asking the user "do you think your program will halt," and neither the user nor the compiler knows.

**Key design points**:
- **Three results from one source**: `True`/`False`/`Budget exhausted` are three exits from one evaluation pipeline
- **Hard budget limits**: Steps, time, quantifier instantiation depth—compiler-internal fixed values, no knobs
- **Automatic strategy selection**: Compiler automatically selects evaluation strategy based on expression complexity; decides directly if it can, delegates to SMT if it cannot
- **Engineering reality**: Vast majority of predicates (`x > 0`, `arr.len > 0`, `0 <= idx < arr.len`) are linear arithmetic, and SMT returns True/False in milliseconds. Budget exhaustion only occurs when users write complex predicates beyond the decidable fragment

**Layered dependencies within the pipeline**: The above evaluators share the same interface but have evaluation order dependencies. Type equality is a prerequisite for all subsequent analysis; ownership/token checking depends on type information; refinement predicate verification depends on results from the first two layers. The compiler evaluates layer by layer—expressions failing at lower layers do not proceed to higher layers—avoiding wasting SMT solver budget on type-erroneous programs.

```
Evaluation order (same pipeline, layered scheduling)
├── Layer 0: Type equality (T1 == T2)
│   └── Structural unification → Failure means subsequent analysis is meaningless, return False directly
├── Layer 1: Ownership/token conflicts
│   └── Flow-sensitive liveness analysis → Failure means memory safety does not hold, return False directly
└── Layer 2: Refinement predicates/Hoare implications
    └── Compiler + SMT → False (counterexample) or Budget exhausted
```

Each layer still returns `True/False/Budget exhausted`, sharing the same interface and same budget system.

### 4. Three-Level Function Unification

| Level | Execution Time | Input | Output | Example |
|-------|----------------|-------|--------|---------|
| Value-level function | Runtime | Value | Value | `add: (a: Int, b: Int) -> Int = a + b` |
| Type constructor | Compile time | Type/Value | Type | `List: (T: Type) -> Type = { data: Array(T) }` |
| Compile-time predicate | Compile time | Value | Type | `Positive: (x: Int) -> Type = { x > 0 }` |

All use the same `name: type = value` syntax. Compile-time predicates and type constructors go through the same compile-time evaluation pipeline—`{}` is the solver's verification space.

### 5. Loops: Floyd-Hoare Verification Condition Generation

Loops do not need separate `: Invariant(...)` or `: decreases(...)` annotations. Compile-time predicate type annotations on variables define Floyd-Hoare-style assertions—the compiler generates verification conditions from type annotations, and the SMT solver checks that each assignment preserves the type.

Core mechanism: Each assignment operation corresponds to a Hoare triple `{P} x := e {Q}`, and the verification condition is `P ⇒ Q[e/x]`. The compiler generates verification conditions for the loop body once—once the SMT solver proves the inductive step, all iterations are automatically covered.

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, 0) = 0   # Verify: 0 == sum(arr[0..0])  → True
    mut i: UpTo(arr.len) = 0     # Verify: 0 <= 0 <= arr.len   → True
    while i < arr.len {
        # Compiler generates VC for loop body once:
        #
        # s += arr[i]:
        #   Premise (inductive hypothesis): s == sum(arr[0..i])
        #   Verification obligation: s_new == sum(arr[0..i+1])
        #   Substitute s_new = s_old + arr[i]:
        #     s_old + arr[i] == sum(arr[0..i]) + arr[i] == sum(arr[0..i+1])
        #   SMT solver: Linear arithmetic, milliseconds → True
        #
        # i += 1:
        #   Premise: i < arr.len (loop condition) + i >= 0 (type bound)
        #   Verification obligation: i+1 >= 0 and i+1 <= arr.len
        #   SMT: Directly derived from premise → True
        s += arr[i]
        i += 1
    }
    return s  # At this point s: SumUpTo(arr, arr.len), i.e., s == sum(arr[0..arr.len])
}
```

Loop invariants are type annotations on variables—programmers write types, compiler checks inductive steps. The compiler does not need to "discover" invariants, nor "automatically perform induction"—it decomposes the inductive proof into local verification conditions for each assignment operation, delegating to the SMT solver for divide-and-conquer.

### 6. Termination Checking

Fully automatic at compile time. Loops the compiler can prove terminate pass; those it cannot prove terminate result in compilation errors—programmers must make loop termination analyzable by the compiler. No opening for semi-automatic annotations.

#### 6.1 Design Principles

The compiler automatically extracts information needed for termination proofs from two sources:

1. **Variable type annotations**: Bounding constraints in refinement types (e.g., `UpTo(n)` gives upper bound `n` and lower bound `0`)
2. **Loop body operations**: Operations applied to variables in each iteration

The compiler tries four ranking synthesis strategies in priority order, stopping upon finding one.

#### 6.2 Strategy 1: Automatic Synthesis of Linear Ranking Functions

When variables have linear bound annotations, the compiler enumerates candidate linear measures and verifies them via SMT.

```
Input:
  Variables v₁: UpTo(u₁), v₂: UpTo(u₂), ... (variables with upper and lower bounds)
  Loop condition cond
  Set of assignments in loop body

Algorithm:
  1. Extract bounds for each variable from type annotations: [low_i, high_i]
  2. Enumerate candidate measures: v_i, u_i - v_i, v_i - v_j, etc. (linear combinations)
  3. For each candidate measure m:
     - SMT verify m ≥ 0 (derived from type bounds)
     - For each execution path in loop body, SMT verify m' < m (strictly decreasing)
  4. Find a linear combination meeting the conditions → Termination proved
```

Coverage: Loops where arbitrary variables are assigned linear expressions (`v = a·v + b`) and have bounded type annotations. Includes `i += const`, `i -= const`, and binary-search-style interval shrinking:

```yaoxiang
# Binary search: low = mid + 1 or high = mid
# Measure high - low strictly decreases on both paths
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

#### 6.3 Strategy 2: Predicate Violation Counting—Measure Automatically Extracted from Target Type

When the loop's return type is a `forall` refinement predicate and the loop body contains local modifications to data, the compiler automatically generates a measure from the predicate definition.

Core insight: **User-written specifications are the compiler's reasoning material.** The compiler does not need a built-in notion of "what sorting is"—it reads the definition of `Sorted`, and automatically extracts a measure from that definition.

```
Input:
  Target type: Sorted(arr) = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }
  Loop body operations: Adjacent element swaps

Algorithm:
  1. Parse predicate definition: forall i in range: cond(i, arr)
  2. Automatically generate measure: violation_count = |{ i | ¬cond(i, arr) }|
  3. Analyze operation's impact on measure:
     - Adjacent swap arr[j], arr[j+1] = arr[j+1], arr[j]
     - Only affects index pairs j-1, j, j+1
     - If arr[j] > arr[j+1] (violating the predicate), after swap this pair satisfies it
     - violation_count decreases by at least 1
  4. Upper bound: n·(n-1)/2 (maximum adjacent inversion count), lower bound: 0
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

#### 6.4 Strategy 3: Bounded Increment/Decrement Patterns

`v += const` (positive constant), variable has upper-bound type annotation → measure `upper_bound - v` decreases by `const` each time, lower bound 0. This is a degenerate case of Strategy 1, handled quickly at the very beginning.

#### 6.5 Strategy 4: Multiplicative Scaling Measure Template

`v *= const` (const > 1), variable has upper and lower bound type annotations. Compiler has a built-in logarithmic measure template `ceil(log_const(upper/v))`, measure decreases by 1 each time `const` multiplies.

```yaoxiang
mut i: Positive = 1
while i < n {
    # Compiler automatically derives: measure ceil(log₂(n/i)), measure decreases by 1 each time multiplied by 2
    i *= 2
}
```

#### 6.6 Separation of Termination and Correctness

Termination proof and correctness proof are independent:
- **Termination**: The four strategies above automatically prove the loop exits in a finite number of steps
- **Correctness**: Whether the loop body advances toward the target type is checked by the SMT solver through verification condition checking

Both pass → Compilation passes. Termination proved but correctness fails → Compilation error + counterexample. Termination cannot be proved → Compilation error pointing out the unanalyzable variable or operation.

#### 6.7 Termination Checking for Recursive Functions

For recursive functions that need compile-time evaluation, the compiler checks that arguments decrease:

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)  # Compiler analysis: n-1 < n → decreasing → terminates
}

# Compile-time usage—compiler guarantees factorial terminates at compile time
vec: Vec(factorial(5)) = Vec(120)()  # 5! = 120, done at compile time
```

| Scenario | Behavior |
|----------|----------|
| Compiler can analyze recursive decrease (e.g., `n-1`) | Compile-time evaluation |
| Not decreasing / cannot determine decrease | Compilation error |
| Runtime call (not in type position) | No termination checking needed |

#### 6.8 Hard Boundaries

`i = f(i)` where `f` is non-invertible, not closed, does not preserve any monotonicity—mathematically impossible to automatically prove termination. Compilation error:

> This loop cannot be automatically proven to terminate. The loop variable depends on unanalyzable function `f`. Please use an iterative pattern analyzable by the compiler.

This is not the compiler's failure. Code that cannot be statically proven safe shall not compile.

### 7. SMT Solver: Backend Component of the Compile-Time Evaluator

The SMT solver is an external tool in traditional languages (e.g., F\* calls Z3, Dafny calls Z3). In YaoXiang, it is a backend component of the compile-time evaluation pipeline—only invoked when the compiler itself cannot decide directly.

```
Compile-time Bool expression
        │
        ├── Compiler can decide directly (simple comparisons, simple arithmetic,
        │   trivial formulas after constant folding)
        │   → Return True/False directly
        │
        └── Compiler cannot decide directly (quantifiers, symbolic variables)
            → Dependent type pre-reduction (factorial(5) → 120)
            → Translate to SMT-LIB format
            → Send to Z3/CVC5 (with budget limit)
            → Return value: unsat (holds) | sat (counterexample) | budget_exceeded
```

**Solver budget—hard limit, like stack depth**:

| Budget Dimension | Description | Hard Upper Bound |
|------------------|-------------|------------------|
| Solver steps | Term rewriting/resolution steps | Compiler-internal fixed value |
| Time | Single predicate solving time | Compiler-internal fixed value |
| Quantifier instantiation depth | forall/exists unfolding depth | Compiler-internal fixed value |

Over budget means compilation error. No degradation, no runtime checks, no silent pass. Error messages precisely locate the predicate in source code and report how much budget was consumed.

**Why this is practically viable**: In practice, 95% of real-world predicates are linear arithmetic—`x > 0`, `arr.len > 0`, `0 <= idx < arr.len`—all within the decidable fragment, and SMT solvers return in milliseconds for such problems. For the rare complex predicates that exceed budget, users simply break the predicate into simpler ones.

Dependent types have a pre-reduction layer before SMT invocation: `factorial(5)` directly evaluates to `120` at compile time, `append([1,2], [3])` directly evaluates to `[1,2,3]`. These deterministic value computations do not consume SMT budget.

Programmers do not need to know SMT exists. The mental model is: **If the compiler can prove it, it passes; if it cannot, it errors**—completely consistent with type checking.

### 8. Compile-Time Predicate Composition

Compile-time predicates are functions returning Type, and composition is naturally achieved through function composition:

```yaoxiang
SortedNonEmpty: (T: Ord, arr: Array(T)) -> Type = {
    Sorted(T, arr) && NonEmpty(T, arr)
}
```

### 9. Code Examples

#### 9.1 Safe Division

```yaoxiang
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive) -> Int = a / b

result = divide(10, 2)   # ✅ Solver verifies { 2 > 0 } → True
# result = divide(10, 0)  # ❌ Solver verifies { 0 > 0 } → False
```

#### 9.2 Array Access Safety

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
| `//! ensures: ExistsMax(result, arr)` | Return type is compile-time predicate `-> IsMax(T, arr)` |
| `/*! invariant: ... !*/` | Compile-time predicate type annotations on variables—Floyd-Hoare invariants |
| `//! decreases: n` | Compiler fully automatically derives measure functions |
| Specification is a comment | Specification is the type system |

### Syntax

**Compile-time predicates have no new syntax.** `{}` is the solver's verification space, identical to existing type definition syntax. Compile-time predicates are functions returning Type—`name: (params) -> Type = { assertion }`.

```bnf
# Compile-time predicate = function returning Type, {} contains solver assertions
# Uses existing function/type syntax, no new BNF rules needed
predicate ::= identifier ':' params '->' 'Type' '=' '{' assertions '}'
```

### Type System Impact

- **Type universe**: Compile-time predicates live at the Type₂ level—functions accepting values and returning Type, same tier as type constructors
- **Generic interaction**: Compile-time predicates can have generic parameters, e.g., `NonEmpty: (T: Type) -> (arr: Array(T)) -> Type`
- **Ownership interaction**: Expressions in compile-time predicates follow ownership rules and can only read, not write
- **Type inference**: Parameters of compile-time predicates participate in HM type inference

### Runtime Representation

Compile-time predicates are **completely erased at runtime**. `Positive: (x: Int) -> Type = { x > 0 }`—parameter `b: Positive` at runtime is represented as just `Int`. The refinement condition `{ x > 0 }` is verified by the SMT solver at compile time, and after verification leaves no runtime trace.

- `Positive(5)` → Runtime representation is `Int(5)`, refinement condition `{ 5 > 0 }` has passed, erased
- `SumUpTo(arr, 0)` → Runtime representation is `Int(0)`, equality `0 == sum(arr[0..0])` has passed, erased
- Placing compile-time predicates in type positions (e.g., `f(x: Positive)`) produces no wrapper types, allocates no extra memory, inserts no runtime checks

Generic erasure and refinement type erasure are the same principle: both are compile-time functions that completely disappear after compile-time evaluation. Compile-time predicates have zero runtime overhead in Release Build—this is the direct consequence of "predicates are functions."

**Interaction constraint with `ref`**: Compile-time predicates can only reference immutable borrows or values whose ownership has been transferred. Compile-time predicates referencing mutable borrows—where the compiler cannot guarantee verification results at compile time still hold at runtime—such usage directly reports a compilation error.

### Compiler Changes

1. **Parser**: Compile-time predicates use standard function syntax, no additional parsing rules needed
2. **Compile-time evaluation pipeline**: Unified Bool return interface, automatic strategy selection
3. **SMT backend**: Z3/CVC5 integration, SMT-LIB format translation
4. **Verification condition generation**: WP/SP calculus + loop invariant proof obligations
5. **Error reporting**: Counterexample formatting, source code location association

### Backward Compatibility

- ✅ Code not using compile-time predicates is completely unchanged
- ✅ Compile-time predicates have zero runtime overhead in Release Build
- ⚠️ RFC-022's `//!` syntax is no longer supported—but 022 was never implemented, so no migration burden

## Trade-offs

### Advantages

- **Complete Curry-Howard isomorphism fulfillment**: Types are propositions, programs are proofs, `name: Proposition = Proof`
- **Uniformity**: Compile-time predicates and ordinary functions use exactly the same syntax, no conceptual split
- **SMT transparency**: Programmers do not need to know SMT exists; mental model is consistent with type checking
- **Gradual adoption**: Can start with one compile-time predicate and progressively increase coverage
- **Zero runtime overhead**: All evaluation happens at compile time; Release Build inserts no runtime assertions

### Disadvantages

- **Compilation time**: SMT solving increases compilation time, but hard budget limits guarantee a controllable upper bound
- **SMT capability boundary**: Complex predicates beyond linear arithmetic may exceed budget; users need to break predicates apart
- **Learning curve**: Writing effective compile-time predicates requires some experience
- **Implementation complexity**: The unified compile-time evaluation pipeline requires careful design

### Risk Mitigation

- SMT solver budget hard limits (steps/time/instantiation depth)—over budget means compilation error
- Dependent type pre-reduction: Deterministic value computations consume first; SMT only tackles the non-deterministic parts
- Incremental verification: Only verify changed modules
- Clear error messages + counterexample display + budget consumption reporting

## Alternatives

| Alternative | Why Not Chosen |
|-------------|----------------|
| RFC-022: `//!` comment-style specifications | Specification splits from type, violates Curry-Howard isomorphism |
| Separate specification files (e.g., CVL) | Specification separated from code, increases maintenance cost |
| Runtime assertions only | Cannot statically guarantee correctness |
| External proof assistants (e.g., Coq) | Steep learning curve, disjoint from compiler |

## Implementation Strategy

### Phase Breakdown

| Phase | Content |
|-------|---------|
| **Phase 1** | Compiler recognizes functions returning Bool can be used in type positions. Support simple arithmetic predicates (`x > 0`, `arr.len > 0`) |
| **Phase 2** | Compile-time Bool evaluation pipeline + Z3 integration. Support `forall`/`exists` quantifiers |
| **Phase 3** | Loop invariant VC generation + termination checking (linear ranking functions + predicate violation counting + bounded patterns) |
| **Phase 4** | Incremental verification + caching + counterexample formatting + IDE support |

### Dependencies

- RFC-010: Unified Type Syntax — Compile-time predicates based on `name: type = value`
- RFC-011: Generic Type System — Compile-time predicates can have generic parameters
- RFC-009: Ownership Model — Expressions in compile-time predicates follow ownership rules

## Open Questions

- [ ] SMT solver choice: Z3 or CVC5? Or pluggable backend?
- [ ] Specific solver budget values: Step upper limit, time upper limit, quantifier instantiation depth upper limit
- [ ] Quantifier support scope: Nested quantifiers? Higher-order quantifiers?
- [ ] Counterexample formatting: How to map SMT models back to source code variables?
- [x] ~~Compile-time predicate interaction with `ref` smart pointers?~~ → Decided: Compile-time predicates only allow immutable borrows or values with transferred ownership. Values with mutable borrows cannot appear in compile-time predicates
- [ ] Extension of `forall` predicate violation counting measure to non-adjacent operations (e.g., quicksort partition)?
- [ ] How to control combinatorial explosion when enumerating linear ranking functions with more than 3 bounded variables?

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

## Lifecycle and Disposition

```
┌─────────────┐
│   Draft     │  ← Author creates
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Under Review │  ← Current status: Community discussion
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
│ (official   │    │ (preserved  │
│  design)    │    │  in place)  │
└─────────────┘    └─────────────┘
```