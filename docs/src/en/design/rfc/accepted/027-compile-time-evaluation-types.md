---
title: 'RFC-027: Compile-Time Predicates and Unified Static Verification'
status: 'Accepted'
author: 'Chen Xu'
created: '2026-06-07'
updated: '2026-09-14'
impl_status: 'in_progress'
impl_detail:
  'Phase 1-2 complete, Phase 3 partially complete, Phase 4 partially complete.
  Assert/Assert unified solution all 6 phases implemented (#157-#162 closed):
  Never type, IsTrue bridging, flow-sensitive Γ + kill set, type-level recursion,
  universe stratified weak checking, dispatch pipeline.'
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
> - [RFC-024: Concurrency Model Based on Spawn Blocks](../accepted/024-concurrency-model.md)
>
> **Supersedes**: [RFC-022: Hoare Logic Static Verification Support (Specification Comments and Specification Types)](../deprecated/022-hoare-logic-static-verification.md)
> — Deprecated

## Abstract

This document proposes introducing **compile-time predicates** as first-class citizens in YaoXiang, unifying all compile-time static verification into a single **proof pipeline**. Compile-time predicates are not external specification comments — they are functions. A function that returns Type, usable in type positions, called by the compiler at compile-time, with its return value checked. Types are propositions, compile-time evaluation is proof.

**Core Argument**: The only work of type checking at compile-time is constructing and verifying proof terms. Type equality, token conflicts, dependent type reduction, compile-time predicate evaluation, Hoare logic implications — all are different kinds of type checking in the compile-time proof pipeline, sharing the same pipeline. The SMT solver is an acceleration module for the type checker, not an independent trust boundary. When the compiler returns Unproven, the programmer writes a YaoXiang function as a proof — the type checker verifies it in exactly the same way it verifies any function's return type. Everything is YaoXiang code, everything is verified by the type checker.

## Motivation

### Why deprecate RFC-022?

RFC-022 designed specifications as `//!` comment forms:

```yaoxiang
max: (T: Ord) -> ((arr: Array(T, n)) -> T) = {
    //! requires: NonEmpty(n) = n > 0          ← This is a comment independent of types
    //! ensures: ExistsMax(result, arr[0..n])   ← This is a comment independent of types
}
```

This commits the fundamental error of Curry-Howard isomorphism: **splitting specifications and types into two layers**. Comments are not types. Comments do not participate in type checking. Comments are the mental model of an "external tool".

The white paper is clear:

> "No `//!` comments. No separate specification language. Everything is within the type system."

### Current Problems

- RFC-022's `//!` comments are external syntax independent of the type system
- Specification types and regular types are two separate systems, causing conceptual redundancy
- The split pattern of Debug Build verification / Release Build ignoring breaks unity
- SMT solvers are positioned as external tools in traditional understanding — YaoXiang makes them internal acceleration modules for the type checker
- Type checking, borrow verification, compile-time predicate checking, and macro expansion each follow different paths

### The Correct Mental Model

Type checking can be abstracted as a function:

```
verify : Program → Proved | Disproved(Model) | Unproven
```

All compile-time checks — simple type matching, borrow conflict detection, compile-time predicate verification — are sub-tasks of this function. They share the same proof pipeline, differing only in proof term complexity and construction strategies.

When the compiler returns Unproven, the programmer provides a proof function — the function's return type equals the proposition to be proved. The type checker verifies it. This is the same operation as ordinary type checking.

## Proposal

### 1. `{}` is the Proof Space: Types are Assertions, Verification is Type Checking

YaoXiang's `{}`
is the compile-time proof space. Everything inside is an assertion, guaranteed by the compiler to be True — either automatically proved or provided by the programmer as a proof function.

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
#          Parameters at signature  The {} contains only assertions
#          Compiler verifies x > 0 when called at compile-time

List: (T: Type) -> Type = { data: Array(T) }
#      ^^^^^^^^              ^^^^^^^^^^^^^^^
#      Parameters at signature  Compiler verifies type_of(T) == Type, type_of(data) == Array(T)
```

The same pattern: `name: (params) -> Type = { assertions }`. The compiler does not distinguish "type assertions" from "value assertions" — both are evaluation targets in the proof pipeline.

**Loop invariants don't need separate writing. Type annotations on variables are Floyd-Hoare invariants.**

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # Annotate references i — tells compiler s's type depends on i
    mut i: UpTo(arr.len) = 0     # At initialization i=0, verify: 0 == sum(arr[0..0]) → True
    while i < arr.len {
        s += arr[i]  # compiler verifies: s_new == sum(arr[0..i+1])
        i += 1       # i changes → triggers s's dependent re-verification: s satisfies SumUpTo(arr, i_new)
    }
    return s  # s: SumUpTo(arr, arr.len) = sum(arr[0..arr.len])
}
```

The compiler generates verification conditions once for the loop body — inductive hypothesis (type annotation) → assignment operation → whether new value satisfies type annotation. Once the proof pipeline verifies the inductive step, all iterations are automatically covered. No
`: decreases`, no `: Invariant`, no inductive proof — the compiler decomposes induction into local VCs for each assignment.

### 2. Pre/Postconditions: Compile-Time Predicates on Parameter and Return Types

Abandon RFC-022's `//! requires`/`//! ensures`. Compile-time predicates serve as parameter or return type annotations.

**The parameter side is a function call.** Compile-time predicates are functions returning Type, and using them on the parameter side means calling them — just like `factorial(5)`. The return side introduces a new concept: return value parameter.

```yaoxiang
# Precondition: Explicitly call compile-time predicate in parameter type
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b
#                       ^^^^^^^^^^  b is the current parameter name, passed to Positive as argument
#                       Compiler extracts actual argument at call site, substitutes b, verifies Positive(actual)
#                       Example: divide(10, 2) → verify Positive(2) = { 2 > 0 } → True
#                       Example: divide(10, 0) → verify Positive(0) = { 0 > 0 } → False → Compile error

# Postcondition: Return value parameter + compile-time predicate
IsMax: (T: Ord, arr: Array(T), result: T) -> Type = {
    forall j in 0..arr.len: result >= arr[j]
}

NonEmpty: (arr: Array(T)) -> Type = { arr.len > 0 }

max: (T: Ord) -> ((arr: NonEmpty(arr))) -> (result: IsMax(T, arr, result)) = {
#                                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
#                                            result is the return value parameter, value provided by return
#                                            Compiler substitutes return value at return point, verifies postcondition
    candidate = arr[0]
    for i in 1..arr.len {
        if arr[i] > candidate { candidate = arr[i] }
    }
    return candidate
}
```

**Key Rules**:

- **Parameter side**: `b: Positive(b)` — `b` is the current parameter name, passed to `Positive`. Function call syntax, zero implicit.
- **Return side**: `-> (result: IsMax(T, arr, result))` — `result` is the return value parameter, value provided by `return` statement. `result` only exists in the type signature, only referenced by predicates, does not enter function body scope, does not appear at call sites.
- **Return value parameter optional**: When no postcondition, omit it, signature is identical to regular function (`-> Int`).
- **Unity**: Parameter and return value parameters are the same concept — `paramName: predicateCall(paramName)`, differing only in whether value is provided by caller or `return`.

### 3. Path Condition Propagation: Compile-Time Verification of Runtime Values

When compile-time predicates are used at binding positions, parameters are explicitly provided by the programmer. When runtime values enter refined type parameters, the compiler completes verification through path condition collection and SMT implication checking — no explicit proof passing by the programmer required.

#### 3.1 Explicit Function Calls

When compile-time predicates are used at binding positions, parameters are explicitly provided by the programmer — it's just a function call, zero implicit.

`Positive: (x: Int) -> Type = { x > 0 }`
is a compile-time predicate constructor. When it appears at a binding position (parameter declaration, variable declaration, return type), the programmer explicitly passes the already-bound variable name:

```yaoxiang
b: Positive(b)
// b has been declared as current parameter, Positive(b) is a function call
// After normalization: b: { b > 0 }
```

No compiler implicit argument filling needed — `b: Positive(b)` is just like `f(5)`, it's a function call. `b`
as the parameter name is bound, its type annotation `Positive(b)` references `b` itself — this is the standard pattern of dependent types, not an implicit expansion rule.

**Unification with RFC-010 `self`**: RFC-010 establishes that `self` is not a keyword, just a conventional parameter name ("writing `p`, `this`, `x` has exactly the same effect"). `b: Positive(b)`
shares the same mechanism — parameter name can be referenced in type annotation. `self` appears in `self: Point`, `b` appears in
`b: Positive(b)`. Both type annotations reference the parameter itself. The only difference is the complexity of the type annotation — the mechanism is exactly the same: after name binding, type can depend on that name.

Return types also use explicit function calls:

```yaoxiang
Sorted: (arr: Array(T)) -> Type = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }

sort: (arr: Array(T)) -> (result: Sorted(result)) = { ... }
//                        ^^^^^^^^^^^^^^^^^^^^^^^
//                        result is the return value parameter, Sorted(result) is a function call
//                        Compiler substitutes return value into result at return point, verifies Sorted(return value)
```

Equally applicable to local variable declarations:

```yaoxiang
let x: Positive(x) = 5
// x bound to 5, Positive(5) → { 5 > 0 } → True → Pass

// let y: Positive(y) = 0
// y bound to 0, Positive(0) → { 0 > 0 } → False → Compile error
```

#### 3.2 Path Condition Collection

When runtime values appear in conditional branches, the compiler automatically collects path conditions, forming a **hypothesis set** for the current scope. These hypotheses participate in verification as background knowledge for compile-time Bool evaluation.

```yaoxiang
if y > 0 {
    // Compiler automatically gains hypothesis in this branch: { y > 0 }
    let result = divide(x, y)
    // Verification condition: (y > 0) ⇒ (y > 0)
    // Proof pipeline decides implication holds → Proved
} else {
// This branch hypothesis: { !(y > 0) }
// If calling divide(x, y), verification condition is !(y > 0) ⇒ y > 0
    // Proof pipeline decides not implied → Disproved
}
```

This is not the compiler hardcoding special patterns — it's the natural behavior of the compile-time proof pipeline. Each type checking call site sends to the pipeline:

```
{background hypotheses} ⇒ {verification goal}
```

The proof pipeline determines implicativity. Proved → Pass, Disproved → Compile error + counterexample, Unproven
→ Compile error + unproven proposition. Background hypotheses come from path conditions at the current program point.

#### 3.3 Hypothesis Stack

When analyzing control flow, the compiler maintains a hypothesis set for each basic block:

- **if-guard**: `if y > 0` → true branch pushes `y > 0`, false branch pushes `!(y > 0)` (if else is used)
- **match pattern**: `if let Some(v) = opt` → pushes `opt == Some(v)` in branch
- **logical conjunction**: `if x > 0 and y < 10` → pushes `x > 0` and `y < 10` in branch
- **function preconditions**: When calling `divide(a, b)`, evidence that `b` satisfies `Positive` must either come from current hypotheses or from the actual argument's refined type annotation (`b` already annotated as `Positive` means its type carries `b > 0`)
- **assignment**: When `let z = y`, refined conditions on `y` transfer to `z`

All hypotheses enter the compile-time proof pipeline. When entering the SMT acceleration path, they're translated to SMT-LIB background assertions.

#### 3.4 No Static Evidence Means Compile Error

If the programmer directly writes:

```yaoxiang
divide_user_input: (x: Int, y: Int) -> Int = divide(x, y)
```

There's no hypothesis of `y > 0` at the current program point, and the actual argument `y` itself has no `Positive` type annotation. The verification condition is:

```
{} ⇒ { y > 0 }
```

The pipeline returns `Disproved` (not implied) → Compile error:

> Cannot prove that parameter `b` satisfies `Positive` in `divide` call. `y`
> comes from function input with no proven bounds. Consider calling with if-branch guard: `if y > 0 { divide(x, y) }`.

YaoXiang does not accept runtime values directly entering refined type parameters without providing static evidence. This is not a limitation — it's the core of hard safety philosophy. Any code the compiler cannot statically prove must not pass compilation.

#### 3.5 Relationship to the Unified Pipeline

Path condition propagation is not an additional mechanism. It's the direct extension of the compile-time proof pipeline to control flow analysis:

| Stage | Responsibility |
| ----------- | -------------------------------------------------------------------------------------------------------- |
| Path condition collection | Compiler control flow analysis stage, annotating each basic block with hypothesis set |
| Verification condition generation | When encountering type constraints to verify, merge path conditions + actual argument type info |
| Proof pipeline evaluation | Compiler kernel → SMT acceleration → arrive at Proved / Disproved / Unproven |
| Result | `Proved` → Pass; `Disproved` → Compile error + counterexample; `Unproven` → Compile error + unproven proposition (programmer can provide proof function) |

No new components. No special rules. Path conditions are background knowledge for the proof pipeline — sharing the same pipeline and budget system with type equations and borrow constraints.

### 4. Compile-Time Proof Pipeline

All compile-time checks share the same pipeline. The pipeline's core operation is **type checking** — checking whether a proof term's type equals the proposition to be proved. Everything is type checking.

```
When compiler encounters Bool expression needing evaluation (i.e., needs to construct a proof term)
        │
        ├── Type equality (T1 == T2)
        │   → Compiler decides directly (structural equivalence)
        │
        ├── Token conflict conditions (!conflicting(tokens))
        │   → Flow-sensitive liveness analysis (Dup/Linear attribute tracking)
        │
        ├── Dependent type reduction (n + m simplification)
        │   → compile-time term rewriting system (βδι-reduction)
        │
        ├── Compile-time predicate (x > 0, forall...)
        │   → compiler itself + SMT accelerator module
        │
        └── Hoare logic implications (P ⇒ Q)
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

#### 4.1 Proof Results: Three-Value Algebra

Compile-time evaluation returns three results — this is an inevitable conclusion of the halting problem, and the natural partition of proof theory:

```
eval_compile_time : BoolExpr → Proved | Disproved(Model) | Unproven
```

- **Proved** → Halts, proof term constructed, type checking passes. Compilation continues.
- **Disproved(M)** → Halts, counterexample M exists. Compile error + counterexample + source location.
- **Unproven** → No proof constructed within given resource limit. Compile error + unproven proposition + budget consumption report.

**Unproven ≠ False.**
The compiler saying "I cannot prove it" is not equivalent to the proposition being false — it simply exceeds current automatic proof capabilities. This is honesty, not a defect.

Hard budget limits are the engineering solution to the halting problem. No knobs — giving knobs means asking users "do you think your program will halt," users don't know, compiler doesn't know either.

#### 4.2 After Unproven: The Programmer Writes the Proof

When the compiler returns Unproven, the programmer can write a **proof function** — a YaoXiang function whose return type equals the proposition to be proved. The type checker verifies this function — the same mechanism as it verifies `add(a, b): Int`.

```
Proposition = Type
Proof      = Program (a value of that type)
Verification = Type checking (the only trust root)
```

The SMT solver is not an independent trust boundary — it's an **acceleration module for the type checker**. SMT helps find proofs, but the type checker is always the one verifying proofs. When SMT returns
`sat`
with model → Disproved (counterexample exists), when SMT returns `unsat` → Proved. If SMT returns `unknown`, the compiler reconstructs SMT's result into a proof term verifiable by the type checker. If reconstruction fails (SMT's reasoning steps exceed compiler kernel's inference rules), fall back to Unproven — programmer can manually write a proof function.

```yaoxiang
# Proposition: Refined property the compiler cannot automatically prove
FirstIsMin: (T: Ord, arr: Sorted(T)) -> Type = {
    forall i in 0..arr.len: arr[0] <= arr[i]
}

# Proof: Programmer writes a function whose return type is the proposition above
# Type checker verifies this function — exactly the same as verifying add(a,b): Int
first_is_min: (T: Ord, arr: Sorted(T)) -> FirstIsMin(T, arr) = {
    # Compiler here verifies: function body's type = FirstIsMin(T, arr)
    ...
}
```

No AI, no export to Coq, no new concepts. **Properties the compiler cannot automatically prove at compile-time → programmer writes proof in YaoXiang code → type checker verifies.**
The whole process is a smooth gradient — the compiler does the easy proofs for you, leaving your mind for the hard ones.

#### 4.3 Layered Dependencies Within the Pipeline

The above evaluators share the same interface but have evaluation order. Type equality is the prerequisite for all subsequent analysis; ownership/token checking depends on type information; refined predicate verification depends on results from the first two layers. The compiler evaluates layer by layer — expressions failing at lower layers don't enter upper layers — avoiding wasted solver budget on programs with type errors.

```
Evaluation order (same pipeline, layered scheduling)
├── Layer 0: Type equality (T1 == T2)
│   └── Structural unification → If fails, subsequent analysis is meaningless, return Disproved directly
├── Layer 1: Ownership/token conflicts
│   └── Flow-sensitive liveness analysis → If fails, memory safety doesn't hold, return Disproved directly
└── Layer 2: Refined predicates/Hoare implications
    └── Compiler itself → SMT acceleration → arrive at Proved / Disproved / Unproven
```

Each layer still returns `Proved/Disproved/Unproven`, sharing the same interface and the same budget
system.

### 5. Three-Layer Function Unification

| Layer | Execution Time | Input | Output | Example |
| ---------- | -------- | ------- | ---- | ---------------------------------------------- |
| Value-level function | Runtime | Value | Value | `add: (a: Int, b: Int) -> Int = a + b` |
| Type constructor | Compile-time | Type/Value | Type | `List: (T: Type) -> Type = { data: Array(T) }` |
| Compile-time predicate | Compile-time | Value | Type | `Positive: (x: Int) -> Type = { x > 0 }` |

All use the same `name: type = value` syntax. Compile-time predicates and type constructors go through the same compile-time proof pipeline — `{}`
is the proof space.

### 6. Loops: Floyd-Hoare Verification Condition Generation

Loops don't need separate `: Invariant(...)` or `: decreases(...)`
annotations. Type annotations with compile-time predicate types on variables define Floyd-Hoare-style assertions — the compiler generates verification conditions from type annotations, and the proof pipeline checks whether each assignment maintains the type.

Core mechanism: Each assignment operation corresponds to a Hoare triple `{P} x := e {Q}`, verification condition is `P ⇒ Q[e/x]`. The compiler generates verification conditions once for the loop body — once the proof pipeline verifies the inductive step, all iterations are automatically covered.

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # Annotation references i; at initialization i=0, verify: 0 == sum(arr[0..0]) → True
    mut i: UpTo(arr.len) = 0     # Verify: 0 <= 0 <= arr.len → True
    while i < arr.len {
        # Compiler generates VC for loop body once. Premise: s satisfies SumUpTo(arr, i), i satisfies UpTo(arr.len).
        #
        # s += arr[i]:
        #   Verification obligation: s_new satisfies SumUpTo(arr, i) (current i unchanged)
        #   Substitute s_new = s_old + arr[i]:
        #     Need s_old + arr[i] == sum(arr[0..i+1])
        #     From inductive hypothesis s_old == sum(arr[0..i]), add arr[i] to both sides:
        #     sum(arr[0..i]) + arr[i] == sum(arr[0..i+1])
        #   Compiler + SMT: Linear arithmetic, millisecond-level → Proved
        #
        # i += 1:
        #   i changes → s's type annotation in dependency graph references i → triggers re-verification
        #   New verification goal: s satisfies SumUpTo(arr, i_new)
        #   i.e., s == sum(arr[0..i_new]), guaranteed by previous step → Proved
        s += arr[i]
        i += 1
    }
    return s  # at this point s: SumUpTo(arr, arr.len), i.e. s == sum(arr[0..arr.len])
}
```

Loop invariants are type annotations on variables — programmer writes types, compiler checks inductive step. The compiler doesn't need to "discover" invariants or "automatically do induction" — it decomposes the inductive proof into local verification conditions for each assignment operation, delegating to the proof pipeline for divide-and-conquer.

#### 6.1 Dependency Tracking: Dependent Types on Mutable Variables

The prerequisite for the above mechanism is: the compiler knows that `s`'s type annotation `SumUpTo(arr, i)` references `i` — when `i` changes, `s`'s type constraint also changes accordingly. This requires the compiler to maintain a **type dependency graph between variables**.

**Data structure**:

```
TypeDepGraph: Map<VarName, Set<VarName>>
# Key is the depended-upon variable, value is the set of variables whose type annotations reference the key
# Example: { i: {s}, j: {s, t}, ... }
```

**Construction**: When the type checker processes `mut v: Pred(... x ...) = init`, it parses free variable references in `Pred(...)` parameters. If parameters reference other mutable variables `x` in current scope, record `x → v` in the dependency graph.

**Trigger**: When a depended-upon variable `x` is assigned, the compiler:

1. Looks up all variables depending on `x` in the dependency graph: `{v₁, v₂, ...}`
2. For each `v`, generates verification condition: `does v's current value satisfy the updated type Pred(... x_new ...)`
3. Sends VCs to the proof pipeline

**Assignment order sensitive**: Dependency tracking naturally enforces correct assignment order. Using `SumUpTo(arr, i)` as example:

```yaoxiang
# Correct order
s += arr[i]   # s_new satisfies SumUpTo(arr, i+1)
i += 1        # i changes → re-verify s satisfies SumUpTo(arr, i_new) → True

# Wrong order — compiler rejects
i += 1        # i changes → re-verify s satisfies SumUpTo(arr, i_new)
              # s not yet updated, s_old == sum(arr[0..i_old]) ≠ sum(arr[0..i_new])
              # → compile error: variable s does not satisfy type SumUpTo(arr, i_new)
s += arr[i]   # unreachable
```

**Combined dependencies**: A variable can depend on multiple variables. Type annotation `{ v: Int; v == x + y }` depends on both `x` and `y` — either change triggers re-verification.

**Relationship with the proof pipeline**: Dependency tracking is the trigger for VC generation, not an independent verification mechanism. It answers "when do we need to generate VCs" — the proof pipeline answers "do the VCs hold".

### 7. Termination Checking

**Scope: Refined types.**
Termination is not a separate switch but part of the **verification mode**: once a type is refined (`Refined { base, constraint }`), the computation it annotates enters verification mode, which requires termination within the mode; ordinary types not refined do not enter verification mode and generate no termination obligations.

From this we derive two corollaries:

- **Loops**: Bare `while` does not enter verification mode; when a measure variable has refined annotation (e.g., `i: UpTo(n)`), it enters verification mode and must prove termination.
- **Recursion**: When a function signature is refined (parameter refinement or return type containing refinement), it enters verification mode and must prove strict decrease at each recursive call site.

Full automation priority within mode: compiler first automatically explores measures; what it can prove passes; if exploration fails and no measure is explicitly given, compile error. **No annotation syntax escape hatch** — measures and termination propositions are written in type positions, no new syntax like `decreases` introduced. Measure and explicit fallback forms are described in §6.9.

#### 6.1 Design Principles

The compiler automatically extracts information needed for termination proofs from two sources:

1. **Variable type annotations**: Boundary constraints in refined types (e.g., `UpTo(n)` gives upper bound `n` and lower bound `0`)
2. **Loop body operations**: Operations applied to variables in each iteration

The compiler tries four measure synthesis strategies in priority order, stops when it finds one. The four strategies are **restricted template sequences for measure exploration**, input is refined constraints (strategies 1–4 all start from "variables with bounded types"), not "code for compile-time evaluation"; they are the automatic and manual sides of the same thing as §6.9's explicit measures.

> **Measure exploration is exploration, not inference.**
> Exploration only enumerates templates (linear rank, violation count, bounded pattern, multiplicative scaling), doesn't guarantee existence of solution — general measure inference is generally undecidable (reduces to halting problem). Therefore cases outside templates must be explicitly given by the programmer (§6.9), otherwise compile error.

#### 6.2 Strategy 1: Automatic Synthesis of Linear Rank Functions

When variables have linear bound annotations, the compiler enumerates candidate linear measures and verifies with SMT.

```
Input:
  Variables v₁: UpTo(u₁), v₂: UpTo(u₂), ... (variables with upper and lower bounds)
  Loop condition cond
  Assignment set in loop body

Algorithm:
  1. Extract bounds for each variable from type annotations: [low_i, high_i]
  2. Enumerate candidate measures: v_i, u_i - v_i, v_i - v_j, linear combinations, etc.
  3. For each candidate measure m:
     - SMT verify m ≥ 0 (derived from type bounds)
     - For each execution path in loop body, SMT verify m' < m (strictly decreasing)
  4. Find a matching linear combination → Termination proved
```

Coverage: Any loop where variables are assigned linear expressions (`v = a·v + b`) and have bounded type annotations. Includes
`i += const`, `i -= const`, and binary-search-style interval shrinking:

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

#### 6.3 Strategy 2: Predicate Violation Counting — Measure Automatically Extracted from Target Type <span style="color:orange">【Experimental Strategy】</span>

> ⚠️ **Current status: Experimental strategy, inclusion decided during Phase 3 implementation based on actual feasibility.**
> This strategy is effective for adjacent swap operations (bubble sort, insertion sort), but cannot automatically prove non-adjacent operations (quicksort partition, heapsort sift-down). See the coverage table below. If Phase 3 verification proves infeasible, this strategy will be removed or demoted to future work.

Core insight: **User-written specifications are materials for compiler reasoning.** The compiler doesn't need to build in "what sorting is" — it reads the definition of `Sorted`, automatically extracting measures from it.

```
Input:
  Target type: Sorted(arr) = { forall i in range: cond(i, arr) }
  Loop body operations: Adjacent element swaps

Algorithm:
  1. Parse predicate definition: forall i in range: cond(i, arr)
  2. Automatically generate measure: violation_count = |{ i | ¬cond(i, arr) }|
  3. Analyze operation's effect on measure:
     - Adjacent swap arr[j], arr[j+1] = arr[j+1], arr[j]
     - Only affects pairs at indices j-1, j, j+1
     - If arr[j] > arr[j+1] (predicate violation), swap satisfies the predicate for this pair
     - violation_count decreases by at least 1
  4. Upper bound: n·(n-1)/2 (maximum adjacent inversion count), lower bound: 0
  → Termination proved
```

**Current coverage**:

| Algorithm | Operation Pattern | Strategy 2 Proves? | Reason |
| -------- | -------------- | :-------------: | -------------------------------------------- |
| Bubble sort | Adjacent swap | ✅ | violation_count strictly decreases per swap |
| Insertion sort | Adjacent move | ✅ | Each shift eliminates one violation pair |
| Selection sort | Non-adjacent swap | ❌ | Single swap may increase violation_count |
| Quicksort | partition | ❌ | Non-adjacent swap, doesn't guarantee monotonic decrease |
| Heapsort | sift-down | ❌ | Tree operations, violation_count non-monotonic |

**Complementary strategies**: For quicksort, the `low < high` interval shrinking is covered by Strategy 1 (linear rank function) — outer partition recursion, each interval halved. Strategy 1 and Strategy 2 complement each other's coverage; most real algorithms' termination can be proved by one of them. But generalizing Strategy 2 (non-adjacent operations, tree operations) remains an open problem.

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

`v += const` (normal constant), variable has upper bound type annotation → measure `upper_bound - v` decreases by `const` each time, lower bound 0. This is a degenerate case of Strategy 1, handled quickly at the front.

#### 6.5 Strategy 4: Multiplicative Scaling Metric Template

`v *= const` (const > 1), variable has upper and lower bound type annotations. Compiler has built-in logarithmic measure template
`ceil(log_const(upper/v))`, each multiply by const decreases measure by 1.

```yaoxiang
mut i: Positive(i) = 1
while i < n {
    # Compiler automatically infers: measure ceil(log₂(n/i)), each multiply by 2 decreases measure by 1
    i *= 2
}
```

#### 6.6 Separation of Termination and Correctness

Termination proof and correctness proof are independent:

- **Termination**: The four strategies above automatically prove loops exit in finite steps; when exploration fails, programmer provides measure in type position (§6.9)
- **Correctness**: Whether loop body advances toward target type, checked by compile-time proof pipeline through verification conditions

Both pass → Compilation passes. Termination proved but correctness fails → Compile error + counterexample. Correctness proved but termination cannot be proved → Compile error pointing out unanalyzable variables or operations. Both fail → Compile error reporting both failures separately.

#### 6.7 Termination Checking for Recursive Functions

For recursive functions with refined signatures, the compiler checks parameter decrease at each recursive call site:

```yaoxiang
gcd: (a: Int, b: NonNegative(b)) -> Int = {
    if b == 0 { return a }
    return gcd(b, a % b)  // Compiler explores: (b, a % b) well-founded order measure decreases → Terminates
}
```

Parameter decrease is the strongest automatic path (structural recursion). When exploration fails, programmer explicitly gives measure in type position (§6.9).

#### 6.8 Hard Boundary

`i = f(i)` where `f` is non-invertible, not closed, doesn't preserve any monotonicity — mathematically impossible to automatically prove termination. Compile error:

> This loop cannot have termination automatically proved. Loop variable depends on unanalyzable function
> `f`. Please use an iteration pattern the compiler can analyze, or bind a name to this loop and provide a measure in type position (§6.9).

This is not the compiler's failure. Any code that cannot be statically proved safe must not pass compilation. Even with explicit measures, they must be decided true by SMT to pass — if the measure is wrong, the counterexample sends it back; humans can only fail to prove, not prove incorrectly.

#### 6.9 Explicit Measure: `Terminates`

When automatic exploration fails to find a measure, the programmer writes the measure in the **type position** — same mechanism as `Positive(b)`, `IsMax(T, arr, result)` (predicate application), zero new syntax:

```yaoxiang
// Measure: ordinary function, can be unit-tested, can be reused, does not participate in runtime
gcd_measure: (a: Int, b: Int) -> Int = { b }

// Termination component lives in the function's own type position
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// Loop: binding name is the anchor, measure obtains the in-scope quantity by name
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n {
        i = i + 1
    }
    return acc
}
```

The reasoning for loops here: `Terminates(m)` refines the type of the loop body's tail expression; the anchor is provided by the binding name `acc` — this makes the loop referable, eliminating the "anonymous construct cannot be referenced" blind spot.

**`Terminates` is a built-in predicate**, alongside `Int`, `Never`
as core primitives. It's the only predicate **whose function body is written by the compiler** — its assertion ("measure strictly decreases at each recursive call site/loop back edge") lives in the computation structure, users' predicates cannot reference function bodies or loop bodies, so it cannot be expressed using the "syntax" section's predicate definition syntax. The built-in aspect converges to this one name.

**Arity**. `Terminates(FnType, m)` and `Terminates(m)` are two arities of the same predicate, not two different constructs:

| Form | Anchor | Purpose |
| ----------------------- | -------------- | -------------------------- |
| `Terminates(m)` | Name of containing binding | Self-recursive functions, loops — default form |
| `Terminates(FnType, m)` | Explicit function type | Mutual recursion and other scenarios where anchor is not unique |

Both are essentially the same: termination obligation always falls on "the computation annotated by the refined type position."

**Measure does not restrict return type.**
The measure can be an expression of any type (not forced to natural numbers); the "strictly decreasing" on it is given by a well-founded order available on that type. Whether the measure is well-founded (e.g., when returning `Int`, whether it is `>= 0`) is an **independent obligation**, like the decreasing obligation, also delegated to refined derivation or SMT; when both cannot be derived, diagnosis **does not directly reject**, but rather suggests checking direction (whether measure has lower bound, whether recursive parameters truly advance in that direction).

**Relationship with automatic exploration**: Explicit measures are not a separate pipeline, but input after exploration failure. After giving a measure, still go through the same SMT to verify decreasing and well-foundedness; if not satisfied, error with counterexample.

For implementation mechanisms of obligation generation, judgment pipeline, diagnostic directions, measure sharing for mutual recursion (SCC), etc., see
[RFC-027a: Explicit Measures for Termination Checking](../review/027a-termination-explicit-measure.md).

### 8. SMT Solver: Acceleration Module for Type Checker

In traditional languages, the SMT solver is an external tool (F\* calls Z3, Dafny calls Z3). In YaoXiang, it's an **acceleration module within the type checker** — only called when the compiler kernel itself cannot directly decide. SMT helps find proofs, but the type checker verifies proofs.

**Trust model**: The type checker is the sole trust root. The SMT solver is an acceleration module — it helps find proofs, but SMT is not an independent trust boundary. The compiler trusts Z3's
`unsat`
result (consistent with F\*/Dafny approach — probability of Z3 error is lower than probability of compiler's own bugs, pragmatic engineering choice). True unreliability control is in the SMT translation layer — if there's a bug in translation, the compiler will expose it in other tests.

**Interface**: Compiler internally translates to SMT-LIB
2.6 standard format, rather than binding specific solver APIs. SMT-LIB is an ISO standard, natively supported by Z3, CVC5, MathSAT, and Yices.

**Default backend**: Z3 (MIT license, most extensive documentation and community verification). CVC5 as SMT-LIB-compatible alternative — users can switch via compiler flags at compile time.

No "general solver abstraction layer" — SMT-LIB is the abstraction layer. If CVC5 makes breakthroughs in specific theories in the future, switching only requires swapping binaries, no compiler code changes needed.

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
            → Return: unsat → Proved │ sat + model → Disproved │ unknown → Unproven
```

**Solving budget — hard limits, like stack depth**:

| Budget Dimension | Default | Description |
| -------------- | ------ | ----------------------------------------------------------------------------------------- |
| Solving steps | 10,000 | Z3 usually finishes linear arithmetic within a few hundred steps. 10,000 steps covers 99% of practical predicates. |
| Time | 100ms | Single predicate exceeding 100ms = user is writing compile-time programs, not type annotations. 100ms × 50 predicates = 5 second compilation time limit. |
| Quantifier instantiation depth | 3 | Three levels of nested quantifiers covers practical patterns. Beyond three levels is likely writing logic exercises. |

Over budget returns Unproven, compile error + predicate location + consumption. No degradation, no runtime checks, no silent pass.

**Why this is practically feasible**: In practice, 95% of actual predicates are linear arithmetic — `x > 0`, `arr.len > 0`, `0 <= idx < arr.len` — all within the decidable fragment, SMT solvers return in milliseconds for such problems. When encountering the rare complex predicate over budget, programmer writes a proof function.

Dependent types go through a layer of pre-reduction before SMT calls: `factorial(5)` directly evaluates to `120` at compile time, `append([1,2], [3])`
directly evaluates to `[1,2,3]`. These deterministic value calculations don't consume SMT budget.

Programmers don't need to know SMT exists. Mental model: **If the compiler can prove it, it passes; if not, it errors — if the compiler can't, you can write a function to prove it.**

### 9. Compose Compile-Time Predicates

Compile-time predicates are functions returning Type, composition is natural function composition:

```yaoxiang
SortedNonEmpty: (T: Ord, arr: Array(T)) -> Type = {
    Sorted(T, arr) and NonEmpty(arr)
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

#### 9.2 Array Access Safety

```yaoxiang
InBounds: (idx: Int, arr: Array(T)) -> Type = { 0 <= idx and idx < arr.len }

get: (arr: Array(T), idx: InBounds(idx, arr)) -> T = arr.data[idx]

arr = Array(Int)(1, 2, 3)
x = get(arr, 1)   # ✅ Compiler verifies InBounds(1, arr) = { 0 <= 1 and 1 < 3 } → True
# y = get(arr, 5)  # ❌ Compiler verifies InBounds(5, arr) = { 0 <= 5 and 5 < 3 } → False
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

### 11. Dispatch Pipeline: Unified Compilation-Time and Runtime Dispatch

`assert` and `Assert` are two faces of the same refined type primitive. The dispatch pipeline `dispatch`
automatically decides whether to go through compile-time proof or runtime check based on **whether the predicate's free variables are accessible at compile-time**:

| Criterion | Mode | Behavior |
| ------------------------------------------------------ | --------------- | ------------------------------------------------------------------- |
| All free variables known at compile-time (generic parameters, compile-time constants) | **CompileTime** | Enter proof pipeline: Proved → erase, Disproved → compile error, Unknown → require proof |
| Free variables from runtime exist (function parameters, external input, mut variables) | **Runtime** | Insert runtime check, inject refined facts into flow-sensitive hypothesis set Γ |

**Key point**: "Cannot decide" ≠ "Disproved". In CompileTime mode, Unknown requires proof (no silent degradation), while in Runtime mode the proposition doesn't even have a truth value at compile-time — no prover, however strong, can write a tautology for "user may have entered a negative number"; runtime check is the only sound choice. This is not the prover being insufficiently strong, it's theoretical necessity.

### 12. Flow-Sensitive Hypothesis Set Γ: Strongest Postcondition Propagation

The compiler maintains a flow-sensitive hypothesis set Γ, tracking propositions known to hold at each control flow point.

**SP (strongest postcondition) propagation**:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
```

**Kill set for mut variables**: After a `mut` variable is reassigned, all hypotheses involving that variable are removed from Γ:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
mut x = x - 5       // Γ = {}  ← x > 0 is killed
```

This is a hard requirement for soundness — variable value changed, old hypothesis invalid.

**Branch merging**: When IF/ELSE or match branches merge, Γ takes the intersection of each branch's hypotheses. Only propositions holding across all paths carry out of the branch.

### 13. Erasure Model Clarification: Witness Erasure ≠ Check Erasure

The RFC-027 assertion that "refined types are **completely erased** at runtime" refers to **proof witnesses (proof tokens)** — proof terms verified at compile-time don't produce runtime code. But the runtime checks inserted by dispatch in Runtime mode are **preserved** — they are Bool checks executed at the value level, not witnesses at the type level.

Summary: witness erasure, check retention. These two things don't conflict, RFC-027's original assertion stands.

## Detailed Design

### Syntax Changes

| Before (RFC-022) | After (This RFC) |
| ------------------------------------- | ---------------------------------------------------------------------------- |
| `//! requires: NonEmpty(n) = n > 0` | Compile-time predicate as parameter type `(b: Positive(b))` |
| `//! ensures: ExistsMax(result, arr)` | Return type using return value parameter `-> (result: IsMax(T, arr, result))` |
| `/*! invariant: ... !*/` | Compile-time predicate type annotation on variables — Floyd-Hoare invariants |
| `//! decreases: n` | Measure written in refined type position (`Terminates`); compiler explores automatically first, requires explicit only when exploration fails |
| Specifications are comments | Specifications are type system |

### Syntax

**Compile-time predicates have no new keywords.** `{}`
is the proof space, fully consistent with existing type definition syntax. Compile-time predicates are functions returning Type — `name: (params) -> Type = { assertions }`. When used, it's a function call — `Positive(b)`, `IsMax(T, arr, result)`.

```bnf
# Compile-time predicate = function returning Type, {} contains assertions verified by compiler
# Uses existing function/type syntax, no new BNF rules needed
predicate ::= identifier ':' params '->' 'Type' '=' '{' assertions '}'
```

**Predicate application actual arguments must be compile-time constant forms** — literals, variables (by-name binding), type applications (recursive extraction), or **compile-time referable function references** (function name). Actual arguments not convertible to constant expressions report
**E1092**, actual argument count doesn't match predicate declaration parameter count report
**E1093**. Arguments bind positionally to parameter list, predicate arity is determined by declaration — `Positive(x)` is unary, `IsMax(T, arr, result)` is ternary, `Terminates(m)` and `Terminates(FnType, m)`
are unary and binary — refined constraints **are never silently dropped** (previously, non-convertible actual arguments would cause constraints to silently disappear, allowing bindings violating constraints to silently pass).

**New syntactic concept: return value parameter** — in `-> (name: Type)`, `name` is the return value parameter.

The return value parameter is the **only syntactic concept** YaoXiang introduces on top of existing function syntax. Its semantics:

- `name`'s value is provided by `return` statement
- `name` only exists in the type signature, referenced by postcondition predicates (e.g., `-> (result: IsMax(T, arr, result))`)
- `name` does not enter function body scope, doesn't appear at call sites
- Return value parameter **optional** — when no postcondition, signature is identical to regular function (`-> Int`), no extra burden introduced

The reason for introducing it: postconditions need to reference "the value the function will return". Without return value parameters, the compiler could only let predicates reference the return value through special rules (e.g., implicit variables like `$result` or `__retval__`). Return value parameters make this reference explicit — it's just a parameter, just that its value comes from
`return` rather than the caller.

**Proof function** is not a new concept — it's just a YaoXiang function whose return type is the asserted proposition. When the compiler returns Unproven, the programmer provides a proof function, and the type checker verifies it in exactly the same way it verifies any function's return type. No new syntax, new keywords, or new rules needed.

> **The boundary between the two.**
> Correctness domain (predicate Unproven) adds proof in the **body**: write a function whose return type is the proposition to be proved. Termination domain (§6.9) adds measure in the **type position**: write
> `Terminates(measure)`
> as the type of a binding or function signature. The former is "write a proof for an unprovable proposition," the latter is "declare a measure the exploration couldn't find" — the mechanism is the same (both are refined type applications), but the landing spots differ. Termination domain **does not need** to write a `_proof` function.

### Type System Impact

- **Type universe**: Compile-time predicates are at Type₂ level — functions accepting values and returning Type, same level as type constructors
- **Generic interaction**: Compile-time predicates can have generic parameters, e.g., `NonEmpty: (T: Type) -> (arr: Array(T)) -> Type`
- **Ownership interaction**: Expressions in compile-time predicates follow ownership rules, read-only, no write
- **Type inference**: Parameters of compile-time predicates participate in HM type inference

### Runtime Representation

Compile-time predicates are processed at runtime **according to dispatch result**:

- **CompileTime mode** (all free variables known at compile-time): After proof passes, witness tokens are completely erased. `Positive: (x: Int) -> Type = { x > 0 }` — parameter `b: Positive(5)` at runtime is just `Int`. Refined condition `{ 5 > 0 }` has passed, erased.
- **Runtime mode** (runtime free variables exist): Runtime check retained — Bool check executed at value level, injected into flow-sensitive hypothesis set Γ. See §11
  dispatch pipeline and §13 erasure model clarification for details.

Placing compile-time predicates in type positions (e.g., `f(x: Positive(x))`) does not create wrapper types, doesn't allocate extra memory. But when `x`
comes from runtime input, a runtime Bool check **is** inserted.

**Interaction constraint with `ref`**: Compile-time predicates can only reference immutable borrows or values whose ownership has been transferred. Compile-time predicates referencing mutable borrows — the compiler cannot guarantee at compile-time that verification results still hold at runtime — such usage directly reports compile error.

### Compiler Changes

1. **Parser**: Compile-time predicates use standard function syntax, no additional parsing rules needed
2. **Compile-time proof pipeline**: Unified Proved/Disproved/Unproven return interface, automatic strategy selection
3. **SMT acceleration module**: SMT-LIB 2.6 translation layer, default backend Z3, CVC5 as backup
4. **Type checker kernel**: Inference rules implementation — structural equivalence, βδι-reduction, universal quantifier introduction/elimination. This is the sole trust root; SMT and programmer proofs are all verified through this
5. **Verification condition generation**: WP/SP calculus + loop invariant proof obligations
6. **Error reporting**: Counterexample formatting + unproven proposition reporting + source location association

### Backward Compatibility

- ✅ Code not using compile-time predicates remains completely unchanged
- ✅ Compile-time predicates in CompileTime mode have zero runtime overhead, Runtime mode only retains necessary Bool checks
- ⚠️ RFC-022's `//!` syntax is no longer supported — but 022 was never implemented, no migration burden

## Trade-offs

### Advantages

- **Curry-Howard isomorphism fully realized**: Types are propositions, programs are proofs, `name: Proposition = Proof`
- **Unity**: Compile-time predicates use exactly the same syntax as regular functions, no conceptual split
- **SMT transparency**: Programmers don't need to know SMT exists, mental model is consistent with type checking
- **Gradual adoption**: Can start with one compile-time predicate, incrementally increase coverage
- **Minimal runtime overhead**: CompileTime mode zero overhead, Runtime mode only retains necessary Bool checks

### Disadvantages

- **Compilation time**: SMT solving increases compilation time, but hard budget limits ensure an upper bound
- **Automatic proof boundary**: Complex predicates beyond first-order linear arithmetic may require programmer-written proof functions. This is not a language defect — it's the inevitable conclusion of the halting problem. Compiler honestly reports Unproven rather than falsely claiming True/False
- **Learning curve**: Writing effective compile-time predicates and proof functions requires understanding basic intuition of Curry-Howard isomorphism
- **Implementation complexity**: Unifying the compile-time proof pipeline requires careful design

### Risk Mitigation

- SMT solving budget hard limits (steps 10,000 / time 100ms / instantiation depth 3), over budget returns Unproven
- Dependent type pre-reduction: Deterministic value calculations are consumed first, SMT only tackles non-deterministic parts
- Unproven is not a dead end: Correctness domain writes proof function (return type is proposition), termination domain gives measure in type position (§6.9) — both are verified by type checker
- Incremental verification: Only verify changed modules
- Clear error messages + counterexample display + budget consumption report + unproven propositions + suggestions (if compiler can provide)

## Alternative Approaches

| Approach | Why Not Chosen |
| ---------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| RFC-022: `//!` comment-style specifications | Specification split from types, violates Curry-Howard isomorphism |
| Separate specification files (e.g., CVL) | Specification separated from code, increases maintenance cost |
| Runtime assertions only | Cannot statically guarantee correctness |
| External proof assistants (e.g., Coq) | Disconnected from compiler, requires separate proof language and trust boundary. YaoXiang's choice: proof is YaoXiang code, type checker is the sole trust root |
| **This proposal: Compile-time predicates as first-class citizens** | ✅ |

## Implementation Strategy

### Phase Breakdown

| Phase | Content |
| ---------- | -------------------------------------------------------------------------------------------------- |
| **Phase 1** | Compiler kernel: structural equivalence + βδι-reduction + universal quantifier introduction/elimination. Support simple arithmetic predicates (`x > 0`, `arr.len > 0`) |
| **Phase 2** | SMT-LIB translation layer + Z3/CVC5 integration. Pipeline returns Proved/Disproved/Unproven. Support programmer writing proof functions when Unproven |
| **Phase 3** | Loop invariant VC generation + termination checking (four measure exploration strategies + `Terminates` explicit measure, §6, §6.9) |
| **Phase 4** | Incremental verification + caching + IDE support |

### Dependencies

- RFC-010: Unified Type Syntax — compile-time predicates based on `name: type = value`
- RFC-011: Generic Type System — compile-time predicates can have generic parameters
- RFC-009: Ownership Model — expressions in compile-time predicates follow ownership rules

## Open Issues

- [x] **SMT solver choice**: Default Z3 (MIT license, most extensively verified). CVC5 as SMT-LIB-compatible alternative, switchable via compiler flag. Compiler internally translates to SMT-LIB
      2.6 standard format — SMT-LIB is the abstraction layer, no custom general solver interface.
- [x] **Specific values for solving budget**: Steps 10,000 / time 100ms
      / quantifier instantiation depth 3. Compiler internally fixed, no knobs. If real usage proves insufficient (not "user wrote it wrong"), adjust.
- [x] **Quantifier support scope**: No language-level restriction on quantifier order. Compile-time predicates accept Type parameters — Type includes function types — therefore higher-order quantifiers are natural consequences of the type system, no special syntax needed. SMT solver can automatically decide first-order quantifiers (forall/exists, supports interleaved nesting, limited by budget depth 3). Higher-order quantifiers: SMT returns Unproven, compiler hints "this predicate exceeds automatic proof scope, please provide proof function". Programmer writes a YaoXiang function whose return type equals that proposition — type checker verifies the function. No external export, no AI, no interactive proof mode. Everything is YaoXiang code, everything is verified by the type checker.
- [x] **Counterexample formatting**: Source variable names directly used as SMT variable names (plus module prefix to avoid conflicts). When Z3 model returns, look up by variable name. Output format: variable name = concrete value + source location + predicate definition location. No complex mapping layer.
- [x] ~~**Interaction of compile-time predicates with `ref` smart pointers?**~~
      → Decided: Compile-time predicates only allow immutable borrows or values with transferred ownership. Values with mutable borrows cannot appear in compile-time predicates.
- [x] **Extension of forall predicate violation counting measure to non-adjacent operations?**
      → Not extended. Current coverage (adjacent swap, adjacent move) is complemented by Strategy 1 (linear rank function) — quicksort outer interval shrinking is backed by Strategy 1, heapsort is backed by Strategy 1 (array index pattern). Loops that cannot have termination proved by any strategy — compiler directly errors — this is hard safety philosophy, not a defect. If there are real-world algorithms (not academic constructs) that no strategy covers in the future, revisit. →
      **Revisit triggered (2026-09-14, #318)**: Non-structural recursion (gcd-style non-direct decrease, mutual recursion, merge partition) is exactly the real scenario this clause anticipated — the measure exploration template sequence cannot automatically handle them, and they cannot be rewritten into analyzable iteration patterns without breaking readability. Conclusion: Hard safety philosophy unchanged (incorrect measures still get SMT counterexamples, humans can only fail to prove, not prove incorrectly), but "exploration failure means rejection" is loosened to "exploration failure allows programmer to explicitly give measure in type position" — see §6.9.
- [x] **Linear rank function enumeration combinatorial explosion**: Candidate enumeration upper bound is 3 bounded variables. ≤3: enumerate all linear combinations and SMT verify each. >3: only try single-variable measures (`v_i`, `u_i - v_i`), on failure directly report compile error — hint "loop has >3 bounded variables, compiler cannot automatically synthesize multi-variable measure". This is not an engineering compromise — it's forcing programmers to write simpler loops.

## References

- [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md)
- [RFC-011: Generics System Design](../accepted/011-generic-type-system.md)
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
│   Draft     │  ← author creates
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Under Review│  ← Current status: Community discussion
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   Accepted  │    │   Rejected  │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│ accepted/   │    │  rejected/  │
│ (official   │    │ (preserved  │
│  design)    │    │  in place)  │
└─────────────┘    └─────────────┘
```