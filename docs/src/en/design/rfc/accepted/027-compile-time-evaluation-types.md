---
title: "RFC-027: Compile-time Predicates and Unified Static Verification"
status: "Accepted"
author: "Chenxu"
created: "2026-06-07"
updated: "2026-06-10"
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

This RFC proposes introducing **compile-time predicates** as first-class citizens in YaoXiang, unifying all compile-time static verification into a single **proof pipeline**. Compile-time predicates are not external specification annotations—they are functions. A function that returns `Type` can be used in a type position; the compiler invokes it at compile-time and checks the return value. Types are propositions; compile-time evaluation is proof.

**Core thesis**: The sole job of type checking at compile-time is to construct and verify proof terms. Type equality, token conflicts, dependent type reduction, compile-time predicate evaluation, Hoare logic implication—all are different type checks in the same compile-time proof pipeline, sharing the same pipeline. The SMT solver is an acceleration module of the type checker, not an independent trust boundary. When the compiler returns `Unproven`, the programmer writes a YaoXiang function as proof—the type checker verifies it in exactly the same way it verifies any function's return type. Everything is YaoXiang code, everything is verified by the type checker.

## Motivation

### Why deprecate RFC-022?

RFC-022 designs specifications as `//!` annotation form:

```yaoxiang
max: (T: Ord) -> ((arr: Array(T, n)) -> T) = {
    //! requires: NonEmpty(n) = n > 0          ← This is an annotation independent of types
    //! ensures: ExistsMax(result, arr[0..n])   ← This is an annotation independent of types
}
```

This commits a fundamental error in the Curry-Howard correspondence: **splitting specifications and types into two layers**. Annotations are not types. Annotations do not participate in type checking. Annotations are the mental model of "external tools."

The white paper states it clearly:

> "No `//!` annotations. No independent specification language. Everything is within the type system."

### Current Problems

- RFC-022's `//!` annotations are an external syntax independent of the type system
- Specification types and ordinary types are two separate systems, causing conceptual redundancy
- The split mode of "verify in Debug Build / ignore in Release Build" breaks unity
- The SMT solver is traditionally positioned as an external tool—YaoXiang builds it in as an acceleration module of the type checker
- Type checking, borrow checking, compile-time predicate checking, and macro expansion each take different paths

### The Correct Mental Model

Type checking can be abstracted as a function:

```
verify : Program → Proved | Disproved(Model) | Unproven
```

All compile-time checks—simple type matching, borrow conflict detection, compile-time predicate verification—are subtasks of this function. They share the same proof pipeline, differing only in proof term complexity and construction strategy.

When the compiler returns `Unproven`, the programmer provides a proof function—whose return type equals the proposition to be proven. The type checker verifies it. This is the same operation as ordinary type checking.

## Proposal

### 1. `{}` Is the Proof Space: Types Are Assertions, Verification Is Type Checking

YaoXiang's `{}` is the compile-time proof space. Everything inside is an assertion; the compiler guarantees every item is `True`—either proven automatically, or with a proof function provided by the programmer.

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
#          Parameters in signature  Only assertions inside {}
#          Compiler verifies x > 0 when invoked at compile-time

List: (T: Type) -> Type = { data: Array(T) }
#      ^^^^^^^^              ^^^^^^^^^^^^^^^
#      Parameters in signature   Compiler verifies type_of(T) == Type, type_of(data) == Array(T)
```

The same pattern: `name: (params) -> Type = { assertion }`. The compiler does not distinguish "type assertions" from "value assertions"—both are evaluation targets in the proof pipeline.

**Loop invariants do not need to be written separately. Type annotations on variables are Floyd-Hoare invariants.**

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # Annotation references i—tells the compiler s's type depends on i
    mut i: UpTo(arr.len) = 0     # At initialization i=0, verify: 0 == sum(arr[0..0]) → True
    while i < arr.len {
        s += arr[i]  # Compiler verifies: s_new == sum(arr[0..i+1])
        i += 1       # i changes → triggers re-verification of s's dependency: s satisfies SumUpTo(arr, i_new)
    }
    return s  # s: SumUpTo(arr, arr.len) = sum(arr[0..arr.len])
}
```

The compiler generates one verification condition for the loop body—inductive hypothesis (type annotation) → assignment operation → does the new value satisfy the type annotation. Once the proof pipeline verifies the inductive step, all iterations are covered automatically. No `: decreases`, no `: Invariant`, no induction proof needed—the compiler decomposes induction into local VCs for each assignment.

### 2. Pre/Postconditions: Compile-time Predicates on Parameter Types and Return Types

Abandon RFC-022's `//! requires`/`//! ensures`. Compile-time predicates serve as type annotations on parameters or returns.

**The parameter side is a function call.** A compile-time predicate is a function that returns `Type`; using it on the parameter side is invoking it—just like `factorial(5)`. The return value side introduces a new concept: the return value parameter.

```yaoxiang
# Precondition: explicit invocation of compile-time predicate in parameter type
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b
#                       ^^^^^^^^^^  b is the current parameter name, passed to Positive as argument
#                       Compiler extracts the argument value at call site, substitutes into b, verifies Positive(arg)
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
#                                            Compiler substitutes return value into result at the return point, verifies postcondition
    candidate = arr[0]
    for i in 1..arr.len {
        if arr[i] > candidate { candidate = arr[i] }
    }
    return candidate
}
```

**Key rules**:

- **Parameter side**: `b: Positive(b)` — `b` is the current parameter name, passed to `Positive` as an argument. Function call syntax, zero implicitness.
- **Return side**: `-> (result: IsMax(T, arr, result))` — `result` is the return value parameter, with its value provided by the `return` statement. `result` exists only in the type signature, is referenced only by predicates, does not enter the function body scope, and does not appear at the call site.
- **Return value parameter is optional**: when there is no postcondition, omit it; the signature is identical to an ordinary function (`-> Int`).
- **Unity**: parameters and return value parameters are the same concept—`param_name: predicate_call(param_name)`—the difference is only whether the value is provided by the caller or by `return`.

### 3. Path Condition Propagation: Compile-time Verification of Runtime Values

When compile-time predicates are used at binding positions, the arguments are passed explicitly by the programmer. When runtime values enter refinement type parameters, the compiler completes verification through path condition collection and SMT implication—without requiring the programmer to explicitly pass a proof.

#### 3.1 Explicit Function Call

When a compile-time predicate is used at a binding position, the arguments are passed explicitly by the programmer—it is a function call, zero implicitness.

`Positive: (x: Int) -> Type = { x > 0 }` is a compile-time predicate constructor. When it appears at a binding position (parameter declaration, variable declaration, return type), the programmer explicitly passes the already-bound variable name:

```yaoxiang
b: Positive(b)
// b is already declared as the current parameter, Positive(b) is a function call
// After normalization: b: { b > 0 }
```

The compiler does not need to implicitly fill in arguments—`b: Positive(b)` is a function call just like `f(5)`. `b` is bound as a parameter name, and its type annotation `Positive(b)` references `b` itself—this is the standard pattern of dependent types, not an implicit expansion rule.

**Unity with RFC-010's `self`**: RFC-010 establishes that `self` is not a keyword, just a conventional parameter name ("writing it as `p`, `this`, or `x` has exactly the same effect"). `b: Positive(b)` shares the same mechanism—the parameter name can be referenced in the type annotation. `self` appears in the position `self: Point`, `b` appears in the position `b: Positive(b)`, and both type annotations reference the parameter itself. The difference is only the complexity of the type annotation; the mechanism is exactly the same—after the name is bound, the type can depend on that name.

The return type likewise uses explicit function calls:

```yaoxiang
Sorted: (arr: Array(T)) -> Type = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }

sort: (arr: Array(T)) -> (result: Sorted(result)) = { ... }
//                        ^^^^^^^^^^^^^^^^^^^^^^^
//                        result is the return value parameter, Sorted(result) is a function call
//                        Compiler substitutes the return value into result at the return point, verifies Sorted(return_value)
```

The same applies to local variable declarations:

```yaoxiang
let x: Positive(x) = 5
// x is bound to 5, Positive(5) → { 5 > 0 } → True → passes

// let y: Positive(y) = 0
// y is bound to 0, Positive(0) → { 0 > 0 } → False → compile error
```

#### 3.2 Path Condition Collection

When runtime values appear in conditional branches, the compiler automatically collects path conditions, forming the **assumption set** of the current scope. These assumptions participate in verification as background knowledge for compile-time `Bool` evaluation.

```yaoxiang
if y > 0 {
    // Compiler automatically acquires assumption here: { y > 0 }
    let result = divide(x, y)
    // Verification condition: (y > 0) ⇒ (y > 0)
    // Proof pipeline judges implication holds → Proved
} else {
    // Assumption here: { !(y > 0) }
    // If divide(x, y) is called, verification condition is !(y > 0) ⇒ y > 0
    // Proof pipeline judges implication does not hold → Disproved
}
```

This is not the compiler hard-coding a special pattern—this is the natural behavior of the compile-time proof pipeline. Each type check call site sends to the pipeline:

```
{background assumptions} ⇒ {verification target}
```

The proof pipeline judges implication. `Proved` → pass, `Disproved` → compile error + counterexample, `Unproven` → compile error + unproven proposition. Background assumptions come from the path conditions at the current program point.

#### 3.3 Assumption Stack

When analyzing control flow, the compiler maintains an assumption set for each basic block:

- **if-guard**: `if y > 0` → true branch pushes `y > 0`, false branch pushes `!(y > 0)` (if `else` is used)
- **match pattern**: `if let Some(v) = opt` → branch pushes `opt == Some(v)`
- **logical conjunction**: `if x > 0 && y < 10` → branch pushes `x > 0` and `y < 10`
- **function precondition**: when calling `divide(a, b)`, evidence that `b` satisfies `Positive` must come either from current assumptions or from the argument's own refinement type annotation (if `b` is annotated as `Positive`, its type carries `b > 0`)
- **assignment**: when `let z = y`, existing refinement conditions on `y` are transferred to `z`

All assumptions enter the compile-time proof pipeline. When entering the SMT acceleration path, they are translated to SMT-LIB background assertions.

#### 3.4 No Static Evidence, Then Compile Error

If the programmer writes directly:

```yaoxiang
divide_user_input: (x: Int, y: Int) -> Int = divide(x, y)
```

The current program point has no assumption of `y > 0`, and the argument `y` itself has no `Positive` type annotation. The verification condition is:

```
{} ⇒ { y > 0 }
```

The pipeline returns `Disproved` (implication does not hold) → compile error:

> Cannot prove that parameter `b` satisfies `Positive` in the call to `divide`.
> `y` comes from function input, with no proven bound.
> Consider guarding the call with an `if` branch: `if y > 0 { divide(x, y) }`.

YaoXiang does not accept runtime values entering refinement type parameters without providing static evidence. This is not a restriction—this is the core of the hard-safety philosophy. Code that the compiler cannot statically prove must not pass compilation.

#### 3.5 Relationship to the Unified Pipeline

Path condition propagation is not an additional mechanism. It is the direct extension of the compile-time proof pipeline in control flow analysis:

| Phase | Responsibility |
|------|------|
| Path condition collection | Compiler's control flow analysis phase, annotating assumption sets for each basic block |
| Verification condition generation | When encountering a type constraint to verify, merge path conditions + argument type information |
| Proof pipeline evaluation | Compiler kernel → SMT acceleration → derive `Proved` / `Disproved` / `Unproven` |
| Result | `Proved` → pass; `Disproved` → compile error + counterexample; `Unproven` → compile error + unproven proposition (programmer may provide a proof function) |

No new components. No special rules. Path conditions are the background knowledge of the proof pipeline—sharing the same pipeline and budget system as type equality and borrow constraints.

### 4. Compile-time Proof Pipeline

All compile-time checks share one pipeline. The core operation of the pipeline is **type checking**—checking whether the type of a proof term equals the proposition to be proven. Everything is type checking.

```
Compile-time encounters a Bool expression needing evaluation (i.e., a proof term needs to be constructed)
        │
        ├── Type equality (T1 == T2)
        │   → Compiler directly judges (structural equivalence)
        │
        ├── Token conflict conditions (!conflicting(tokens))
        │   → Flow-sensitive liveness analysis (Dup/Linear attribute tracking)
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
             │ Unproven │  → Compile error + unproven proposition
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

#### 4.1 Proof Result: A Three-Valued Algebra

Compile-time evaluation returns three results—this is the inevitable conclusion of the halting problem, and the natural division of proof theory:

```
eval_compile_time : BoolExpr → Proved | Disproved(Model) | Unproven
```

- **Proved** → halts, proof term constructed, type check passes. Compilation continues.
- **Disproved(M)** → halts, counterexample M exists. Compile error + counterexample + source location.
- **Unproven** → within the given resource limit, no proof was constructed. Compile error + unproven proposition + budget consumption report.

**Unproven ≠ False.** The compiler saying "I cannot prove it" is not equivalent to the proposition being false—it's just beyond the current automatic proof capability. This is honesty, not a defect.

A hard budget limit is the engineering solution to the halting problem. No knobs are provided—giving one would be asking the user "do you think your program halts"; the user doesn't know, and neither does the compiler.

#### 4.2 After Unproven: The Programmer Writes the Proof

When the compiler returns `Unproven`, the programmer can write a **proof function**—a YaoXiang function whose return type equals the proposition to be proven. The type checker verifies this function—in exactly the same way it verifies `add(a, b): Int`.

```
Proposition = Type
Proof       = Program (a value of that type)
Verification = Type checking (the sole trust root)
```

The SMT solver is not an independent trust boundary—it is an **acceleration module of the type checker**. SMT helps find proofs, but the proof is always verified by the type checker. When SMT returns `unsat`, the compiler reconstructs its result into a proof term verifiable by the type checker. If reconstruction fails (SMT's inference steps exceed the compiler kernel's inference rules), it falls back to `Unproven`—the programmer can manually write a proof function.

```yaoxiang
# Proposition: a refinement attribute the compiler cannot automatically prove
FirstIsMin: (T: Ord, arr: Sorted(T)) -> Type = {
    forall i in 0..arr.len: arr[0] <= arr[i]
}

# Proof: programmer writes a function whose return type is the above proposition
# Type checker verifies this function—exactly the same as verifying add(a,b): Int
first_is_min: (T: Ord, arr: Sorted(T)) -> FirstIsMin(T, arr) = {
    # Compiler verifies here: type of function body = FirstIsMin(T, arr)
    ...
}
```

No AI needed, no export to Coq, no new concepts. **Attributes the compiler cannot automatically prove at compile-time → programmer uses YaoXiang code to write the proof → type checker verifies.** The whole process is a smooth gradient—the compiler does the simple proofs for you, leaving the hard ones to you.

#### 4.3 Layered Dependencies Within the Pipeline

The above evaluators share the same interface but have an evaluation order. Type equality is the prerequisite for all subsequent analysis; ownership/token checking depends on type information; refinement predicate verification depends on the results of the first two layers. The compiler evaluates layer by layer; expressions that fail at a lower layer do not enter upper layers—avoiding wasting solver budget on type-incorrect programs.

```
Evaluation order (same pipeline, layered scheduling)
├── Layer 0: Type equality (T1 == T2)
│   └── Structural unification → failure means the rest is meaningless, directly return Disproved
├── Layer 1: Ownership/token conflict
│   └── Flow-sensitive liveness analysis → failure means memory safety does not hold, directly return Disproved
└── Layer 2: Refinement predicate / Hoare implication
    └── Compiler itself → SMT acceleration → derive Proved / Disproved / Unproven
```

Each layer still returns `Proved/Disproved/Unproven`, sharing the same interface and the same budget system.

### 5. Three-Layer Function Unity

| Layer | When Executed | Input | Output | Example |
|------|----------|------|------|------|
| Value-level function | Runtime | Value | Value | `add: (a: Int, b: Int) -> Int = a + b` |
| Type constructor | Compile-time | Type/Value | Type | `List: (T: Type) -> Type = { data: Array(T) }` |
| Compile-time predicate | Compile-time | Value | Type | `Positive: (x: Int) -> Type = { x > 0 }` |

All use the same `name: type = value` syntax. Compile-time predicates and type constructors go through the same compile-time proof pipeline—`{}` is the proof space.

### 6. Loops: Floyd-Hoare Verification Condition Generation

Loops do not need separate `: Invariant(...)` or `: decreases(...)` annotations. The compile-time predicate type annotation on variables defines Floyd-Hoare-style assertions—the compiler generates verification conditions from type annotations, and the proof pipeline checks whether each assignment preserves the type.

Core mechanism: each assignment operation corresponds to a Hoare triple `{P} x := e {Q}`, with verification condition `P ⇒ Q[e/x]`. The compiler generates one verification condition for the loop body—once the proof pipeline verifies the inductive step, all iterations are covered automatically.

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
        #     From inductive hypothesis s_old == sum(arr[0..i]), add arr[i] to both sides:
        #     sum(arr[0..i]) + arr[i] == sum(arr[0..i+1])
        #   Compiler + SMT: linear arithmetic, millisecond level → Proved
        #
        # i += 1:
        #   i changes → s's type annotation in dependency graph references i → triggers re-verification
        #   New verification target: s satisfies SumUpTo(arr, i_new)
        #   i.e., s == sum(arr[0..i_new]), guaranteed by the previous step → Proved
        s += arr[i]
        i += 1
    }
    return s  # At this point s: SumUpTo(arr, arr.len), i.e., s == sum(arr[0..arr.len])
}
```

Loop invariants are type annotations on variables—programmer writes the type, compiler checks the inductive step. The compiler does not need to "discover" invariants, nor does it need to "automatically do induction"—it decomposes the inductive proof into local verification conditions for each assignment operation, and delegates to the proof pipeline to divide and conquer.

#### 6.1 Dependency Tracking: Dependent Types on Mutable Variables

The prerequisite for the above mechanism is: the compiler knows that the type annotation `SumUpTo(arr, i)` of `s` references `i`—when `i` changes, the type constraint of `s` changes accordingly. This requires the compiler to maintain a **type dependency graph between variables**.

**Data structure**:

```
TypeDepGraph: Map<VarName, Set<VarName>>
# Key is the depended-on variable, value is the set of variables that reference this variable in their type annotations
# Example: { i: {s}, j: {s, t}, ... }
```

**Construction**: When the type checker processes `mut v: Pred(... x ...) = init`, it parses the free variable references in `Pred(...)`'s arguments. If the arguments reference another mutable variable `x` in the current scope, it records `x → v` in the dependency graph.

**Trigger**: When a depended-on variable `x` is assigned, the compiler:
1. Looks up all variables `{v₁, v₂, ...}` in the dependency graph that depend on `x`
2. For each `v`, generates a verification condition: does `v`'s current value satisfy the updated type `Pred(... x_new ...)`?
3. Sends the VC to the proof pipeline

**Assignment-order sensitive**: dependency tracking naturally enforces the correct assignment order. Take `SumUpTo(arr, i)` as an example:

```yaoxiang
# Correct order
s += arr[i]   # s_new satisfies SumUpTo(arr, i+1)
i += 1        # i changes → re-verify s satisfies SumUpTo(arr, i_new) → True

# Wrong order—compiler rejects
i += 1        # i changes → re-verify s satisfies SumUpTo(arr, i_new)
              # s not yet updated, s_old == sum(arr[0..i_old]) ≠ sum(arr[0..i_new])
              # → Compile error: variable s does not satisfy type SumUpTo(arr, i_new)
s += arr[i]   # unreachable
```

**Combined dependencies**: a variable can depend on multiple variables. The type annotation `{ v: Int; v == x + y }` depends on both `x` and `y`—change of either triggers re-verification.

**Relationship to the proof pipeline**: dependency tracking is the trigger for VC generation, not an independent verification mechanism. It answers "when VCs need to be generated"—the proof pipeline answers "whether the VC holds."

### 7. Termination Checking

Fully automatic at compile-time. The compiler proves the loop passes, or cannot prove it and directly reports a compile error—the programmer must let the compiler automatically analyze the loop's termination. No semi-automatic annotation escape hatches are provided.

#### 7.1 Design Principles

The compiler automatically extracts the information needed for termination proofs from two places:

1. **Variable type annotations**: boundary constraints in refinement types (e.g., `UpTo(n)` gives upper bound `n` and lower bound `0`)
2. **Loop body operations**: operations applied to variables per iteration

The compiler tries four metric synthesis strategies in priority order, stopping once one succeeds.

#### 7.2 Strategy 1: Linear Ranking Function Automatic Synthesis

When variables have linear bound annotations, the compiler enumerates candidate linear metrics and verifies them via SMT.

```
Input:
  Variables v₁: UpTo(u₁), v₂: UpTo(u₂), ... (variables with lower and upper bounds)
  Loop condition cond
  Set of assignments in the loop body

Algorithm:
  1. Extract each variable's bounds from type annotations: [low_i, high_i]
  2. Enumerate candidate metrics: v_i, u_i - v_i, v_i - v_j, etc. linear combinations
  3. For each candidate metric m:
     - SMT verifies m ≥ 0 (derived from type bounds)
     - For each execution path in the loop body, SMT verifies m' < m (strictly decreasing)
  4. Find a linear combination that satisfies the conditions → termination proven
```

Coverage: any loop where a variable is assigned a linear expression (`v = a·v + b`) and has a bounded type annotation. Includes `i += const`, `i -= const`, and binary-search-style interval contraction:

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

#### 7.3 Strategy 2: Predicate Violation Counting — Automatically Extracting Metrics from Target Types <span style="color:orange">【Experimental Strategy】</span>

> ⚠️ **Current status: experimental strategy, inclusion decided at stage 3 implementation based on practical feasibility.**
> This strategy is effective for adjacent swap operations (bubble sort, insertion sort), but cannot automatically prove for non-adjacent operations (quicksort partition, heapsort sift-down).
> See the coverage boundary table below. If stage 3 verification proves infeasible, this strategy will be removed or downgraded to future work.

Core insight: **the specifications written by the user are the compiler's inference material.** The compiler does not need to hard-code "what is sorting"—it reads the definition of `Sorted` and automatically extracts the metric from the definition.

```
Input:
  Target type: Sorted(arr) = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }
  Loop body operation: adjacent element swap

Algorithm:
  1. Parse predicate definition: forall i in range: cond(i, arr)
  2. Automatically generate metric: violation_count = |{ i | ¬cond(i, arr) }|
  3. Analyze the operation's effect on the metric:
     - Adjacent swap arr[j], arr[j+1] = arr[j+1], arr[j]
     - Only affects index pairs j-1, j, j+1
     - If arr[j] > arr[j+1] (violates predicate), after swap this pair satisfies the predicate
     - violation_count decreases by at least 1
  4. Upper bound: n·(n-1)/2 (maximum adjacent inversions), lower bound: 0
  → Termination proven
```

**Current coverage**:

| Algorithm | Operation Pattern | Strategy 2 Provable? | Reason |
|------|----------|:---:|------|
| Bubble sort | Adjacent swap | ✅ | violation_count strictly decreases with each swap |
| Insertion sort | Adjacent movement | ✅ | Each shift eliminates one violation pair |
| Selection sort | Non-adjacent swap | ❌ | A single swap may increase violation_count |
| Quicksort | partition division | ❌ | Non-adjacent swap, no guarantee of monotonic decrease |
| Heapsort | sift-down | ❌ | Tree-structured operations, violation_count is non-monotonic |

**Complementary strategies**: for quicksort, the `low < high` interval contraction can be covered by Strategy 1 (linear ranking function)—the outer partition recursion halves the interval each time. Strategy 1 and Strategy 2 complementarily cover each other, and the termination of most practical algorithms can be proven by one of them. But the generalization of Strategy 2 (non-adjacent operations, tree operations) remains an open problem.

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

`v += const` (positive constant), variable has an upper bound type annotation → metric `upper_bound - v` decreases by `const` each time, lower bound 0. This is a degenerate case of Strategy 1, processed quickly at the front by the compiler.

#### 7.5 Strategy 4: Multiplicative Scaling Metric Template

`v *= const` (const > 1), variable has lower and upper bound type annotations. The compiler has a built-in logarithmic metric template `ceil(log_const(upper/v))`; each multiplication by `const` decreases the metric by 1.

```yaoxiang
mut i: Positive(i) = 1
while i < n {
    # Compiler automatically derives: metric ceil(log₂(n/i)), each *2 decreases metric by 1
    i *= 2
}
```

#### 7.6 Separation of Termination and Correctness

Termination proofs and correctness proofs are independent:
- **Termination**: the four strategies above automatically prove the loop exits in finite steps
- **Correctness**: whether the loop body progresses toward the target type, checked by the compile-time proof pipeline through verification conditions

Both pass → compilation passes. Termination proven but correctness fails → compile error + counterexample. Correctness proven but termination cannot be proven → compile error pointing to the unanalyzable variable or operation. Both fail → compile error reports both failure reasons separately.

#### 7.7 Termination Checking for Recursive Functions

For recursive functions that need to be evaluated at compile-time, the compiler checks argument decrease:

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)  # Compiler analysis: n-1 < n → decreasing → terminates
}

# Compile-time use—compiler guarantees factorial terminates at compile-time
vec: Vec(factorial(5)) = Vec(120)()  # 5! = 120, completed at compile-time
```

| Scenario | Behavior |
|------|------|
| Compiler can analyze recursive decrease (e.g., `n-1`) | Compile-time evaluation |
| Does not decrease / cannot determine decrease | Compile error |
| Runtime call (not in a type position) | No termination check needed |

#### 7.8 Hard Boundary

`i = f(i)` where `f` is non-invertible, non-closed, and does not preserve any monotonicity—mathematically impossible to automatically prove termination. Compile error:

> This loop cannot be automatically proven to terminate. The loop variable depends on the unanalyzable function `f`. Please use an iteration pattern that the compiler can analyze.

This is not a compiler failure. Code that cannot be statically proven safe must not pass compilation.

### 8. SMT Solver: An Acceleration Module of the Type Checker

In traditional languages, the SMT solver is an external tool (e.g., F\* calls Z3, Dafny calls Z3). In YaoXiang, it is **an acceleration module of the type checker**—invoked only when the compiler kernel itself cannot directly judge. SMT helps find proofs, but the proof is verified by the type checker.

**Trust model**: the type checker is the sole trust root. The SMT solver is an acceleration module—it helps find proofs, but SMT is not an independent trust boundary. The compiler trusts Z3's `unsat` result (consistent with the F\*/Dafny line—Z3's error probability is lower than the compiler's own bug rate, which is a pragmatic engineering choice). The real unreliability is controlled at the SMT translation layer—if the translation has bugs, the compiler will be exposed by other tests.

**Interface**: the compiler internally translates to SMT-LIB 2.6 standard format, not bound to a specific solver API. SMT-LIB is an ISO standard, natively supported by Z3, CVC5, MathSAT, Yices.

**Default backend**: Z3 (MIT license, broadest documentation and community validation). CVC5 as an SMT-LIB-compatible alternative—users can switch via a compiler flag at compile-time.

No "generic solver abstraction layer"—SMT-LIB is the abstraction layer. If CVC5 makes a breakthrough in a particular theory in the future, switching requires only swapping binaries, not changing compiler code.

```
Compile-time Bool expression
        │
        ├── Compiler kernel can directly judge (structural equivalence, simple arithmetic,
        │   trivial formulas after constant folding)
        │   → Directly return Proved / Disproved
        │
        └── Compiler kernel cannot directly judge (quantifiers, symbolic variables)
            → Dependent type pre-reduction (factorial(5) → 120)
            → Translate to SMT-LIB format
            → Send to Z3/CVC5 (with budget limit)
            → Return value: unsat → Proved  │  sat + model → Disproved  │  unknown → Unproven
```

**Solver budget—hard limit, like stack depth**:

| Budget Dimension | Default Value | Description |
|----------|--------|------|
| Solver steps | 10,000 | Z3 typically solves linear arithmetic within hundreds of steps. 10,000 steps covers 99% of practical predicates. |
| Time | 100ms | A single predicate exceeding 100ms = the user is writing a compile-time program, not a type annotation. 100ms × 50 predicates = 5 second compile time cap. |
| Quantifier instantiation depth | 3 | Three levels of nested quantifiers cover practical patterns. Beyond three levels is most likely a logic exercise. |

Exceeding the budget returns `Unproven`, with a compile error + predicate location + consumption. No degradation, no runtime check, no silent pass.

**Why this is actually feasible**: in practice, 95% of real predicates are linear arithmetic—`x > 0`, `arr.len > 0`, `0 <= idx < arr.len`—all in decidable fragments, with the SMT solver returning in milliseconds. For the rare complex predicate that exceeds the budget, the programmer writes a proof function.

Dependent types are pre-reduced before the SMT call: `factorial(5)` is directly evaluated at compile-time to `120`, `append([1,2], [3])` is directly evaluated to `[1,2,3]`. These deterministic value computations do not consume SMT budget.

The programmer does not need to know SMT exists. Mental model: **if the compiler can prove it, it passes; if not, it reports an error—and if the compiler can't, you can write a function to prove it.**

### 9. Compile-time Predicate Composition

Compile-time predicates are functions that return `Type`; composition is naturally achieved through function composition:

```yaoxiang
SortedNonEmpty: (T: Ord, arr: Array(T)) -> Type = {
    Sorted(T, arr) && NonEmpty(arr)
}
```

### 10. Code Examples

#### 10.1 Safe Division

```yaoxiang
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b

result = divide(10, 2)   # ✅ Compiler verifies Positive(2) = { 2 > 0 } → True
# result = divide(10, 0)  # ❌ Compiler verifies Positive(0) = { 0 > 0 } → False
```

#### 10.2 Safe Array Access

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

## Detailed Design

### Syntax Changes

| Before (RFC-022) | After (This RFC) |
|---|---|
| `//! requires: NonEmpty(n) = n > 0` | Compile-time predicate as parameter type `(b: Positive(b))` |
| `//! ensures: ExistsMax(result, arr)` | Return type uses return value parameter `-> (result: IsMax(T, arr, result))` |
| `/*! invariant: ... !*/` | Compile-time predicate type annotation on variables—Floyd-Hoare invariant |
| `//! decreases: n` | Compiler fully automatically derives the metric function |
| Specifications are annotations | Specifications are the type system |

### Syntax

**Compile-time predicates have no new keywords.** `{}` is the proof space, identical in syntax to existing type definitions. A compile-time predicate is a function that returns `Type`—`name: (params) -> Type = { assertion }`. Usage is a function call—`Positive(b)`, `IsMax(T, arr, result)`.

```bnf
# Compile-time predicate = function returning Type, {} contains compiler-verified assertions
# Uses existing function/type syntax, no new BNF rules needed
predicate ::= identifier ':' params '->' 'Type' '=' '{' assertions '}'
```

**New syntax concept: return value parameter**—in `-> (name: Type)`, `name` is the return value parameter.

The return value parameter is the **only syntax concept** YaoXiang introduces on top of existing function syntax. Its semantics:
- The value of `name` is provided by the `return` statement
- `name` exists only in the type signature, is referenced by postcondition predicates (e.g., `-> (result: IsMax(T, arr, result))`)
- `name` does not enter the function body scope, and does not appear at the call site
- The return value parameter is **optional**—when there is no postcondition, the signature is identical to an ordinary function (`-> Int`), introducing no extra burden

The reason for introducing it: postconditions need to reference "the value the function is about to return." Without a return value parameter, the compiler can only use special rules (such as the implicit variable `$result` or `__retval__`) to let predicates reference the return value. The return value parameter makes this reference explicit—it is a parameter, except that the value is provided by `return` rather than by the caller.

**Proof functions** are not a new concept—they are simply YaoXiang functions whose return type is the asserted proposition. When the compiler returns `Unproven`, the programmer provides a proof function, and the type checker verifies it in exactly the same way it verifies any function's return type. No new syntax, no new keywords, no new rules needed.

### Type System Impact

- **Type universe**: compile-time predicates live in the `Type₂` layer—functions that accept values and return `Type`, at the same level as type constructors
- **Generics interaction**: compile-time predicates can take generic parameters, e.g., `NonEmpty: (T: Type) -> (arr: Array(T)) -> Type`
- **Ownership interaction**: expressions in compile-time predicates obey ownership rules; they can only read, not write
- **Type inference**: arguments of compile-time predicates participate in HM type inference

### Runtime Representation

Compile-time predicates are **completely erased** at runtime. `Positive: (x: Int) -> Type = { x > 0 }`—the parameter `b: Positive(b)` has runtime representation of simply `Int`. The refinement condition `{ x > 0 }` is verified by the proof pipeline at compile-time; after verification, no runtime trace remains.

- `Positive(5)` → runtime representation is `Int(5)`, refinement condition `{ 5 > 0 }` has passed, erased
- `SumUpTo(arr, 0)` → runtime representation is `Int(0)`, equality `0 == sum(arr[0..0])` has passed, erased
- Placing a compile-time predicate in a type position (e.g., `f(x: Positive(x))`) produces no wrapper type, no extra allocation, no runtime check insertion

Generic erasure and refinement type erasure follow the same principle: both are compile-time functions that completely disappear after evaluation at compile-time. Compile-time predicates have zero runtime overhead in Release Build—this is a direct corollary of "predicates are functions."

**Interaction constraint with `ref`**: compile-time predicates can only reference values with immutable borrows or values whose ownership has been transferred. Compile-time predicates that reference mutable borrows cannot be guaranteed by the compiler to still hold at runtime—such usage directly reports a compile error.

### Compiler Changes

1. **Parser**: compile-time predicates use standard function syntax, no additional parsing rules needed
2. **Compile-time proof pipeline**: unified `Proved/Disproved/Unproven` return interface, automatic strategy selection
3. **SMT acceleration module**: SMT-LIB 2.6 translation layer, default backend Z3, CVC5 as alternative
4. **Type checker kernel**: inference rule implementation—structural equivalence, βδι-reduction, universal quantifier introduction/elimination. This is the sole trust root; both SMT and programmer proofs are verified through it
5. **Verification condition generation**: WP/SP calculus + loop invariant proof obligations
6. **Error reporting**: counterexample formatting + unproven proposition reports + source location association

### Backward Compatibility

- ✅ Code not using compile-time predicates is completely unchanged
- ✅ Compile-time predicates have zero runtime overhead in Release Build
- ⚠️ RFC-022's `//!` syntax is no longer supported—but 022 was never implemented, with no migration burden

## Trade-offs

### Advantages

- **Full realization of the Curry-Howard correspondence**: types are propositions, programs are proofs, `name: Proposition = Proof`
- **Unity**: compile-time predicates and ordinary functions use exactly the same syntax, with no conceptual split
- **SMT transparency**: programmers don't need to know SMT exists; the mental model is consistent with type checking
- **Progressive adoption**: start with one compile-time predicate, gradually increase coverage
- **Zero runtime overhead**: all evaluation is completed at compile-time; Release Build inserts no runtime assertions

### Disadvantages

- **Compile time**: SMT solving increases compile time, but hard budget limits guarantee a controllable upper bound
- **Automatic proof boundary**: complex predicates beyond first-order linear arithmetic may require the programmer to write proof functions. This is not a language defect—this is the inevitable conclusion of the halting problem. The compiler honestly reports `Unproven` rather than falsely reporting `True`/`False`
- **Learning curve**: writing effective compile-time predicates and proof functions requires understanding the basic intuition of the Curry-Howard correspondence
- **Implementation complexity**: unifying the compile-time proof pipeline requires careful design

### Risk Mitigation

- SMT solver budget hard limit (steps 10,000 / time 100ms / instantiation depth 3); exceeding the budget returns `Unproven`
- Dependent type pre-reduction: deterministic value computation is consumed first; SMT only tackles the non-deterministic part
- `Unproven` is not a dead end: programmers can write proof functions, verified by the type checker—consistent with verifying any function's return type
- Incremental verification: only validate changed modules
- Clear error messages + counterexample display + budget consumption report + unproven proposition + suggestions (if the compiler can provide them)

## Alternatives

| Option | Why Not Chosen |
| -------------------------------- | -------------------------------------- |
| RFC-022: `//!` annotation-style specifications | Specifications and types are split, violating the Curry-Howard correspondence |
| Independent specification files (e.g., CVL) | Separation of specifications and code, increasing maintenance cost |
| Runtime assertions only | Cannot statically guarantee correctness |
| External proof assistants (e.g., Coq) | Disconnected from the compiler, requiring an independent proof language and trust boundary. YaoXiang's choice: proofs are YaoXiang code, the type checker is the sole trust root |
| **This plan: compile-time predicates as first-class citizens** | ✅ |

## Implementation Strategy

### Phasing

| Phase | Content |
| ---------- | -------------------------------------------------------------------- |
| **Phase 1** | Compiler kernel: structural equivalence + βδι-reduction + universal quantifier introduction/elimination. Support simple arithmetic predicates (`x > 0`, `arr.len > 0`) |
| **Phase 2** | SMT-LIB translation layer + Z3/CVC5 integration. Pipeline returns `Proved/Disproved/Unproven`. On `Unproven`, support programmer writing proof functions |
| **Phase 3** | Loop invariant VC generation + termination checking (linear ranking function + predicate violation counting + bounded pattern + combinatorial explosion control) |
| **Phase 4** | Incremental verification + caching + IDE support |

### Dependencies

- RFC-010: Unified Type Syntax — compile-time predicates are based on `name: type = value`
- RFC-011: Generics System — compile-time predicates can take generic parameters
- RFC-009: Ownership Model — expressions in compile-time predicates obey ownership rules

## Open Questions

- [x] **SMT solver selection**: default Z3 (MIT license, most widely validated). CVC5 as an SMT-LIB-compatible alternative, switched via compiler flag. The compiler's internal translation target is the SMT-LIB 2.6 standard format—SMT-LIB is the abstraction layer; no custom generic solver interface.
- [x] **Specific solver budget values**: steps 10,000 / time 100ms / quantifier instantiation depth 3. Fixed inside the compiler, no knobs. If real use cases prove insufficient in actual use (not "the user wrote it wrong"), then adjust.
- [x] **Quantifier support scope**: the language does not limit the quantifier order. Compile-time predicates accept `Type` arguments—`Type` includes function types—so higher-order quantifiers are a natural consequence of the type system, requiring no special syntax. The SMT solver can automatically decide first-order quantifiers (forall/exists, with interleaved nesting, limited by budget depth 3). Higher-order quantifiers: SMT returns `Unproven`, the compiler prompts "this predicate exceeds the automatic proof range, please provide a proof function." The programmer writes a YaoXiang function whose return type equals the proposition—the type checker verifies the function. No external export, no AI, no interactive proof mode needed. Everything is YaoXiang code, everything is verified by the type checker.
- [x] **Counterexample formatting**: source variable names are used directly as SMT variable names (with module prefix to avoid conflicts). Z3 model returns are looked up by variable name. Output format: variable name = concrete value + source location + predicate definition location. No complex mapping layer.
- [x] ~~**Interaction of compile-time predicates with `ref` smart pointers?**~~ → Decided: compile-time predicates only allow values with immutable borrows or transferred ownership. Values with mutable borrows cannot appear in compile-time predicates.
- [x] **Extension of forall predicate violation count metric to non-adjacent operations?** → No extension. Current coverage (adjacent swap, adjacent movement) is complementarily covered by Strategy 1 (linear ranking function)—quicksort's outer interval contraction is covered by Strategy 1, and heapsort is covered by Strategy 1 (array index pattern). Loops whose termination cannot be proven by any strategy directly report a compiler error—this is the hard-safety philosophy, not a defect. If in the future there is a real scenario (non-academic construction) where an algorithm cannot be covered by any of the four strategies, then rediscuss.
- [x] **Linear ranking function enumeration combinatorial explosion**: candidate enumeration upper limit is 3 bounded variables. ≤3 enumerate all linear combinations and verify each via SMT. >3 only try single-variable metrics (`v_i`, `u_i - v_i`), failure directly reports a compile error—prompting the programmer "the loop has >3 bounded variables, the compiler cannot automatically synthesize multi-variable metrics." This is not an engineering compromise—it forces programmers to write simpler loops.

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
│ (Formal     │    │ (Remains    │
│  design)    │    │  in place)  │
└─────────────┘    └─────────────┘
```