---
title: 'RFC-027: Compile-Time Predicates and Unified Static Verification'
status: 'Accepted'
author: 'Chen Xu'
created: '2026-06-07'
updated: '2026-07-05'
impl_status: 'in_progress'
impl_detail:
  'Phase 1-2 complete, Phase 3 partially complete, Phase 4 partially complete. The unified
  assert/Assert scheme (all 6 phases) is implemented (#157-#162 closed): Never type, IsTrue
  bridging, flow-sensitive Γ + kill set, type-level recursion, universe stratification weak check,
  and dispatch pipeline.'
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
> - [RFC-011: Generics System Design](../accepted/011-generic-type-system.md)
> - [RFC-024: Concurrency Model Based on spawn Blocks](../accepted/024-concurrency-model.md)
>
> **Supersedes**:
> [RFC-022: Hoare Logic Static Verification Support (Specification Comments and Specification Types)](../deprecated/022-hoare-logic-static-verification.md)
> — Deprecated

## Abstract

This document proposes introducing **compile-time predicates** as first-class citizens in YaoXiang,
unifying all compile-time static verification into a single **proof pipeline**. Compile-time
predicates are not bolted-on specification comments—they are functions. A function that returns
`Type` can be used in type position; the compiler invokes it at compile time and checks the return
value. Types are propositions; compile-time evaluation is proof.

**Core thesis**: The only job of type checking at compile time is to construct and verify proof
terms. Type equality, token conflicts, dependent type reduction, compile-time predicate evaluation,
Hoare-logic implication—all are different type checks within the same compile-time proof pipeline,
sharing one pipeline. The SMT solver is an accelerator module of the type checker, not a separate
trust boundary. When the compiler returns `Unproven`, the programmer writes a YaoXiang function as a
proof—the type checker verifies it in exactly the same way as it verifies any function's return
type. Everything is YaoXiang code, everything is verified by the type checker.

## Motivation

### Why deprecate RFC-022?

RFC-022 designed specifications as `//!` comments:

```yaoxiang
max: (T: Ord) -> ((arr: Array(T, n)) -> T) = {
    //! requires: NonEmpty(n) = n > 0          ← A comment independent of the type
    //! ensures: ExistsMax(result, arr[0..n])   ← A comment independent of the type
}
```

This commits a fundamental error of the Curry-Howard correspondence: **splitting specifications and
types into two layers**. Comments are not types. Comments do not participate in type checking.
Comments belong to the "external tool" mental model.

The white paper is unambiguous:

> "There are no `//!` comments. There is no independent specification language. Everything is within
> the type system."

### Current Problems

- RFC-022's `//!` comments are external syntax bolted on alongside the type system
- Specification types and ordinary types are two separate systems, causing conceptual redundancy
- The Debug Build verification / Release Build ignore split pattern breaks unity
- SMT solvers are traditionally positioned as external tools—YaoXiang builds them in as accelerator
  modules of the type checker
- Type checking, borrow verification, compile-time predicate checking, and macro expansion each take
  different paths

### The Correct Mental Model

Type checking can be abstracted as a function:

```
verify : Program → Proved | Disproved(Model) | Unproven
```

All compile-time checks—simple type matching, borrow conflict detection, compile-time predicate
verification—are subtasks of this function. They share one proof pipeline; the only differences are
proof term complexity and construction strategy.

When the compiler returns `Unproven`, the programmer provides a proof function—whose return type
equals the proposition to be proved. The type checker verifies it. This is the same operation as
ordinary type checking.

## Proposal

### 1. `{}` Is the Proof Space: Types Are Assertions, Verification Is Type Checking

YaoXiang's `{}` is the compile-time proof space. Everything inside is an assertion; the compiler
guarantees each is `True`—either proved automatically, or the programmer provides a proof function.

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
#          Compiler verifies x > 0 when invoked at compile time

List: (T: Type) -> Type = { data: Array(T) }
#      ^^^^^^^^              ^^^^^^^^^^^^^^^
#      Parameters in signature     Compiler verifies type_of(T) == Type, type_of(data) == Array(T)
```

The same pattern: `name: (params) -> Type = { assertions }`. The compiler does not distinguish "type
assertions" from "value assertions"—both are evaluation targets in the proof pipeline.

**Loop invariants need not be written separately. Type annotations on variables are Floyd-Hoare
invariants.**

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # Annotation references i—tells the compiler s's type depends on i
    mut i: UpTo(arr.len) = 0     # At initialization i=0, verify: 0 == sum(arr[0..0]) → True
    while i < arr.len {
        s += arr[i]  # Compiler verifies: s_new == sum(arr[0..i+1])
        i += 1       # i changes → triggers s dependency re-verification: s satisfies SumUpTo(arr, i_new)
    }
    return s  # s: SumUpTo(arr, arr.len) = sum(arr[0..arr.len])
}
```

The compiler generates one verification condition for the loop body—induction hypothesis (type
annotation) → assignment operation → does the new value satisfy the type annotation. Once the proof
pipeline verifies the inductive step, all iterations are automatically covered. No `: decreases`, no
`: Invariant`, no inductive proofs needed—the compiler decomposes induction into a local VC for each
assignment.

### 2. Pre/Postconditions: Compile-Time Predicates on Parameter and Return Types

Abandon RFC-022's `//! requires`/`//! ensures`. Compile-time predicates are type annotations on
parameters or returns.

**The parameter side is a function call.** A compile-time predicate is a function returning `Type`;
its use in parameter position is just calling it—exactly like `factorial(5)`. The return value side
introduces a new concept: the return value parameter.

```yaoxiang
# Precondition: explicit call to a compile-time predicate in parameter type
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b
#                       ^^^^^^^^^^  b is the current parameter name, passed to Positive as argument
#                       Compiler extracts the actual argument value at the call site, substitutes b, verifies Positive(actual)
#                       E.g.: divide(10, 2) → verifies Positive(2) = { 2 > 0 } → True
#                       E.g.: divide(10, 0) → verifies Positive(0) = { 0 > 0 } → False → compile error

# Postcondition: return value parameter + compile-time predicate
IsMax: (T: Ord, arr: Array(T), result: T) -> Type = {
    forall j in 0..arr.len: result >= arr[j]
}

NonEmpty: (arr: Array(T)) -> Type = { arr.len > 0 }

max: (T: Ord) -> ((arr: NonEmpty(arr))) -> (result: IsMax(T, arr, result)) = {
#                                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
#                                            result is the return value parameter, value provided by return
#                                            Compiler substitutes the return value at the return point, verifies the postcondition
    candidate = arr[0]
    for i in 1..arr.len {
        if arr[i] > candidate { candidate = arr[i] }
    }
    return candidate
}
```

**Key rules**:

- **Parameter side**: `b: Positive(b)` — `b` is the current parameter name, passed to `Positive` as
  argument. Function call syntax, zero implicits.
- **Return side**: `-> (result: IsMax(T, arr, result))` — `result` is the return value parameter,
  value provided by the `return` statement. `result` exists only in the type signature, is
  referenced only by predicates, does not enter the function body scope, and does not appear at the
  caller.
- **Return value parameter is optional**: When there is no postcondition, do not write it; the
  signature is identical to a normal function (`-> Int`).
- **Unity**: Parameters and return value parameters are the same
  concept—`param_name: predicate_call(param_name)`—the difference is only whether the value is
  provided by the caller or by `return`.

### 3. Path Condition Propagation: Compile-Time Verification of Runtime Values

When compile-time predicates are used in binding position, parameters are passed in explicitly by
the programmer. When runtime values enter refinement type parameters, the compiler performs
verification through path condition collection and SMT implication—no need for the programmer to
explicitly pass a proof.

#### 3.1 Explicit Function Call

When a compile-time predicate is used in binding position, parameters are passed in explicitly by
the programmer—it is just a function call, zero implicits.

`Positive: (x: Int) -> Type = { x > 0 }` is a compile-time predicate constructor. When it appears in
binding position (parameter declaration, variable declaration, return type), the programmer
explicitly passes in a bound variable name:

```yaoxiang
b: Positive(b)
// b is already declared as the current parameter, Positive(b) is a function call
// After normalization: b: { b > 0 }
```

No need for the compiler to implicitly fill in parameters—`b: Positive(b)` is the same as `f(5)`, it
is just a function call. `b` is bound as a parameter name, and its type annotation `Positive(b)`
references `b` itself—this is the standard pattern of dependent types, not an implicit expansion
rule.

**Unification with RFC-010's `self`**: RFC-010 established that `self` is not a keyword, just a
conventional parameter name ("writing `p`, `this`, or `x` has exactly the same effect").
`b: Positive(b)` shares the same mechanism—parameter names can be referenced in type annotations.
`self` appears in `self: Point`; `b` appears in `b: Positive(b)`; both type annotations reference
the parameter itself. The difference is only the complexity of the type annotation; the mechanism is
identical—once a name is bound, the type can depend on that name.

The return type likewise uses explicit function calls:

```yaoxiang
Sorted: (arr: Array(T)) -> Type = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }

sort: (arr: Array(T)) -> (result: Sorted(result)) = { ... }
//                        ^^^^^^^^^^^^^^^^^^^^^^^
//                        result is the return value parameter, Sorted(result) is a function call
//                        Compiler substitutes the return value at the return point, verifies Sorted(return value)
```

The same applies to local variable declarations:

```yaoxiang
let x: Positive(x) = 5
// x binds to 5, Positive(5) → { 5 > 0 } → True → pass

// let y: Positive(y) = 0
// y binds to 0, Positive(0) → { 0 > 0 } → False → compile error
```

#### 3.2 Path Condition Collection

When runtime values appear in conditional branches, the compiler automatically collects path
conditions, forming the current scope's **assumption set**. These assumptions participate in
verification as background knowledge for compile-time `Bool` evaluation.

```yaoxiang
if y > 0 {
    // Compiler automatically acquires assumption in this branch: { y > 0 }
    let result = divide(x, y)
    // Verification condition: (y > 0) ⇒ (y > 0)
    // Proof pipeline judges implication holds → Proved
} else {
// This branch assumes: { not (y > 0) }
// If divide(x, y) is called here, the verification condition is not (y > 0) ⇒ y > 0
    // Proof pipeline judges implication does not hold → Disproved
}
```

This is not the compiler hard-coding a special pattern—it is the natural behavior of the
compile-time proof pipeline. Each type-check call site sends to the pipeline:

```
{background assumptions} ⇒ {verification target}
```

The proof pipeline judges implication. `Proved` → pass, `Disproved` → compile error +
counterexample, `Unproven` → compile error + unsolved proposition. Background assumptions come from
the path conditions of the current program point.

#### 3.3 Assumption Stack

When analyzing control flow, the compiler maintains an assumption set for each basic block:

- **if-guard**: `if y > 0` → true branch pushes `y > 0`, false branch pushes `not (y > 0)` (if else
  is used)
- **match pattern**: `if let Some(v) = opt` → inside branch, push `opt == Some(v)`
- **Logical conjunction**: `if x > 0 and y < 10` → inside branch, push `x > 0` and `y < 10`
- **Function preconditions**: When calling `divide(a, b)`, evidence that `b` satisfies `Positive`
  must come either from current assumptions, or from the actual argument's own refinement type
  annotation (if `b` is annotated as `Positive`, its type carries `b > 0`)
- **Assignment**: When `let z = y`, the refinement conditions already on `y` transfer to `z`

All assumptions enter the compile-time proof pipeline. When entering the SMT acceleration path, they
are translated into SMT-LIB background assertions.

#### 3.4 No Static Evidence Means Compile Error

If the programmer writes directly:

```yaoxiang
divide_user_input: (x: Int, y: Int) -> Int = divide(x, y)
```

The current program point has no assumption of `y > 0`, and the actual argument `y` itself has no
`Positive` type annotation. The verification condition is:

```
{} ⇒ { y > 0 }
```

Pipeline returns `Disproved` (implication does not hold) → compile error:

> Cannot prove that parameter `b` in the call to `divide` satisfies `Positive`. `y` comes from
> function input with no proven bound. Consider guarding the call with an if branch:
> `if y > 0 { divide(x, y) }`.

YaoXiang does not accept runtime values directly entering refinement type parameters without
providing static evidence. This is not a restriction—it is the core of the hard-safety philosophy.
Any code the compiler cannot statically prove must not pass compilation.

#### 3.5 Relationship to the Unified Pipeline

Path condition propagation is not an additional mechanism. It is the direct extension of the
compile-time proof pipeline in control-flow analysis:

| Phase                             | Responsibility                                                                                                                                           |
| --------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Path condition collection         | Compiler control-flow analysis phase, annotates each basic block with an assumption set                                                                  |
| Verification condition generation | When encountering a type constraint to verify, merges path conditions + actual argument type info                                                        |
| Proof pipeline evaluation         | Compiler kernel → SMT acceleration → yields Proved / Disproved / Unproven                                                                                |
| Result                            | `Proved` → pass; `Disproved` → compile error + counterexample; `Unproven` → compile error + unsolved proposition (programmer can provide proof function) |

No new components. No special rules. Path conditions are the background knowledge of the proof
pipeline—sharing the same pipeline and the same budget system as type equality and borrow
constraints.

### 4. Compile-Time Proof Pipeline

All compile-time checks share one pipeline. The pipeline's core operation is **type
checking**—checking whether the type of a proof term equals the proposition to be proved. Everything
is type checking.

```
Compile time encounters a Bool expression requiring evaluation (i.e., requiring construction of a proof term)
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
        ├── Compile-time predicate (x > 0, forall...)
        │   → Compiler itself + SMT accelerator module
        │
        └── Hoare logic implication (P ⇒ Q)
            → Compiler + SMT accelerator module
                    │
                    ▼
             ┌──────────┐
             │ Proved   │  → Compilation passes
             │ Disproved│  → Compile error + counterexample
             │ Unproven │  → Compile error + unsolved proposition
             └────┬─────┘
                  │
                  ▼
         Programmer writes a proof function (YaoXiang code)
                  │
                  ▼
         Type checker verifies ──→ Proved ──→ Compilation passes
                  │
                  ▼
            Verification failed → Compile error: "proof does not hold"
```

#### 4.1 Proof Results: Three-Valued Algebra

Compile-time evaluation returns three results—this is the necessary conclusion of the halting
problem and the natural division of proof theory:

```
eval_compile_time : BoolExpr → Proved | Disproved(Model) | Unproven
```

- **Proved** → halts, proof term constructed, type check passes. Compilation continues.
- **Disproved(M)** → halts, counterexample M exists. Compile error + counterexample + source
  location.
- **Unproven** → within the given resource limit, no proof was constructed. Compile error + unsolved
  proposition + budget consumption report.

**Unproven ≠ False.** The compiler saying "I cannot prove it" is not equivalent to the proposition
being false—it is only beyond the current automatic proof capability. This is honesty, not a defect.

A hard budget limit is the engineering solution to the halting problem. No knob—giving one would be
asking the user "do you think your program will halt," and neither user nor compiler knows.

#### 4.2 After Unproven: The Programmer Writes the Proof

When the compiler returns `Unproven`, the programmer can write a **proof function**—a YaoXiang
function whose return type equals the proposition to be proved. The type checker verifies this
function—using the same mechanism as verifying `add(a, b): Int`.

```
Proposition = Type
Proof       = Program (a value of that type)
Verification = Type checking (the only trust root)
```

The SMT solver is not a separate trust boundary—it is an **accelerator module of the type checker**.
SMT helps find proofs, but the proof is always verified by the type checker. When SMT returns
`unsat`, the compiler reconstructs the result into a proof term verifiable by the type checker. If
reconstruction fails (SMT's inference steps exceed the compiler kernel's inference rules), it falls
back to `Unproven`—the programmer can manually write a proof function.

```yaoxiang
# Proposition: refinement property the compiler cannot automatically prove
FirstIsMin: (T: Ord, arr: Sorted(T)) -> Type = {
    forall i in 0..arr.len: arr[0] <= arr[i]
}

# Proof: programmer writes a function whose return type is the proposition above
# Type checker verifies this function—exactly as it verifies add(a,b): Int
first_is_min: (T: Ord, arr: Sorted(T)) -> FirstIsMin(T, arr) = {
    # Compiler verifies here: function body type = FirstIsMin(T, arr)
    ...
}
```

No AI needed, no export to Coq, no new concepts. **Properties the compiler cannot prove
automatically at compile time → programmer writes the proof in YaoXiang code → type checker
verifies.** The whole process is a smooth gradient—the compiler handles easy proofs for you, saving
your brain for the hard ones.

#### 4.3 Layered Dependencies Within the Pipeline

The evaluators above share the same interface but have an evaluation order. Type equality is a
prerequisite for all subsequent analysis; ownership/token checks depend on type information;
refinement predicate verification depends on the results of the first two layers. The compiler
evaluates layer by layer; expressions failing at lower layers do not enter upper layers—avoiding
wasting solver budget on type-incorrect programs.

```
Evaluation order (one pipeline, layered dispatch)
├── Layer 0: Type equality (T1 == T2)
│   └── Structural unification → failure makes the rest meaningless, directly return Disproved
├── Layer 1: Ownership/token conflicts
│   └── Flow-sensitive liveness analysis → failure means memory safety does not hold, directly return Disproved
└── Layer 2: Refinement predicates / Hoare implications
    └── Compiler itself → SMT acceleration → yields Proved / Disproved / Unproven
```

Each layer still returns `Proved/Disproved/Unproven`, sharing the same interface and the same budget
system.

### 5. Unification of Three Function Layers

| Layer                  | When Run     | Input      | Output | Example                                        |
| ---------------------- | ------------ | ---------- | ------ | ---------------------------------------------- |
| Value-level function   | Runtime      | Value      | Value  | `add: (a: Int, b: Int) -> Int = a + b`         |
| Type constructor       | Compile time | Type/Value | Type   | `List: (T: Type) -> Type = { data: Array(T) }` |
| Compile-time predicate | Compile time | Value      | Type   | `Positive: (x: Int) -> Type = { x > 0 }`       |

All use the same `name: type = value` syntax. Compile-time predicates and type constructors go
through the same compile-time proof pipeline—`{}` is the proof space.

### 6. Loops: Floyd-Hoare Verification Condition Generation

Loops do not need separate `: Invariant(...)` or `: decreases(...)` annotations. Compile-time
predicate type annotations on variables define Floyd-Hoare-style assertions—the compiler generates
verification conditions from type annotations, and the proof pipeline checks whether each assignment
maintains the type.

Core mechanism: each assignment corresponds to a Hoare triple `{P} x := e {Q}`; the verification
condition is `P ⇒ Q[e/x]`. The compiler generates one verification condition for the loop body—once
the proof pipeline verifies the inductive step, all iterations are automatically covered.

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # Annotation references i; at initialization i=0, verify: 0 == sum(arr[0..0]) → True
    mut i: UpTo(arr.len) = 0     # Verify: 0 <= 0 <= arr.len → True
    while i < arr.len {
        # Compiler generates one VC for the loop body. Precondition: s satisfies SumUpTo(arr, i), i satisfies UpTo(arr.len).
        #
        # s += arr[i]:
        #   Verification obligation: s_new satisfies SumUpTo(arr, i) (current i unchanged)
        #   Substituting s_new = s_old + arr[i]:
        #     Need s_old + arr[i] == sum(arr[0..i+1])
        #     From induction hypothesis s_old == sum(arr[0..i]), add arr[i] to both sides:
        #     sum(arr[0..i]) + arr[i] == sum(arr[0..i+1])
        #   Compiler + SMT: linear arithmetic, millisecond-level → Proved
        #
        # i += 1:
        #   i changes → in dependency graph, s's type annotation references i → triggers re-verification
        #   New verification target: s satisfies SumUpTo(arr, i_new)
        #   That is, s == sum(arr[0..i_new]), guaranteed by the previous step → Proved
        s += arr[i]
        i += 1
    }
    return s  # At this point s: SumUpTo(arr, arr.len), i.e., s == sum(arr[0..arr.len])
}
```

Loop invariants are simply type annotations on variables—programmer writes the type, compiler checks
the inductive step. The compiler does not need to "discover" invariants, nor does it need to "do
induction automatically"—it decomposes the inductive proof into local verification conditions for
each assignment, handing them over to the proof pipeline to divide and conquer.

#### 6.1 Dependency Tracking: Dependent Types on Mutable Variables

The prerequisite of the above mechanism is: the compiler knows that the type annotation of `s`,
`SumUpTo(arr, i)`, references `i`—when `i` changes, `s`'s type constraint also changes. This
requires the compiler to maintain a **type dependency graph between variables**.

**Data structure**:

```
TypeDepGraph: Map<VarName, Set<VarName>>
# Key is the depended-on variable, value is the set of variables whose type annotations reference that variable
# E.g.: { i: {s}, j: {s, t}, ... }
```

**Construction**: When the type checker processes `mut v: Pred(... x ...) = init`, it resolves free
variable references in the `Pred(...)` arguments. If the arguments reference another mutable
variable `x` in the current scope, it records `x → v` in the dependency graph.

**Trigger**: When a depended-on variable `x` is assigned, the compiler:

1. Looks up all variables `{v₁, v₂, ...}` in the dependency graph that depend on `x`
2. For each `v`, generates the verification condition: does `v`'s current value satisfy the updated
   type `Pred(... x_new ...)`?
3. Sends the VC into the proof pipeline

**Assignment order sensitive**: Dependency tracking naturally enforces the correct assignment order.
Taking `SumUpTo(arr, i)` as an example:

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

**Composite dependencies**: A variable can depend on multiple variables. The type annotation
`{ v: Int; v == x + y }` depends on both `x` and `y`—changes to either trigger re-verification.

**Relationship to the proof pipeline**: Dependency tracking is the trigger for VC generation, not an
independent verification mechanism. It answers "when do VCs need to be generated"—the proof pipeline
answers "does the VC hold".

### 7. Termination Checking

Fully automatic at compile time. Loops the compiler can prove terminate pass; loops it cannot prove
terminate directly report compile errors—the programmer must let the compiler automatically analyze
loop termination. No half-automatic annotation escape hatch.

#### 6.1 Design Principles

The compiler automatically extracts information needed for termination proofs from two sources:

1. **Variable type annotations**: Boundary constraints in refinement types (e.g., `UpTo(n)` provides
   upper bound `n` and lower bound `0`)
2. **Loop body operations**: Operations applied to variables each iteration

The compiler attempts four metric synthesis strategies in priority order; once one succeeds, it
stops.

#### 6.2 Strategy 1: Automatic Linear Rank Function Synthesis

When variables have linear bound annotations, the compiler enumerates candidate linear metrics and
verifies them via SMT.

```
Input:
  Variables v₁: UpTo(u₁), v₂: UpTo(u₂), ... (variables with upper and lower bounds)
  Loop condition cond
  Set of assignments in the loop body

Algorithm:
  1. Extract each variable's bounds from type annotations: [low_i, high_i]
  2. Enumerate candidate metrics: v_i, u_i - v_i, v_i - v_j, etc., linear combinations
  3. For each candidate metric m:
     - SMT verifies m ≥ 0 (derived from type bounds)
     - For each execution path in the loop body, SMT verifies m' < m (strictly decreasing)
  4. Find a linear combination satisfying the conditions → termination proved
```

Coverage scope: loops where any variable is assigned a linear expression (`v = a·v + b`) and has a
bounded type annotation. Includes `i += const`, `i -= const`, and binary-search-style interval
contraction:

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

#### 6.3 Strategy 2: Predicate Violation Counting—Automatically Extract Metric from Target Type <span style="color:orange">[Experimental Strategy]</span>

> ⚠️ **Current status: experimental strategy; inclusion decided based on actual feasibility during
> Phase 3 implementation.** This strategy works for adjacent swap operations (bubble sort, insertion
> sort), but cannot automatically prove non-adjacent operations (quicksort partition, heapsort
> sift-down). Coverage boundaries are shown in the table below. If Phase 3 verification proves
> infeasible, this strategy will be removed or downgraded to future work.

Core insight: **user-written specifications are material for compiler reasoning.** The compiler does
not need to hard-code "what is sorting"—it reads the definition of `Sorted` and automatically
extracts the metric from it.

```
Input:
  Target type: Sorted(arr) = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }
  Loop body operation: adjacent element swap

Algorithm:
  1. Parse the predicate definition: forall i in range: cond(i, arr)
  2. Automatically generate metric: violation_count = |{ i | ¬cond(i, arr) }|
  3. Analyze the operation's effect on the metric:
     - Adjacent swap arr[j], arr[j+1] = arr[j+1], arr[j]
     - Only affects pairs at indices j-1, j, j+1
     - If arr[j] > arr[j+1] (violates predicate), after swap the pair satisfies the predicate
     - violation_count decreases by at least 1
  4. Upper bound: n·(n-1)/2 (maximum adjacent inversions), lower bound: 0
  → Termination proved
```

**Current coverage scope**:

| Algorithm      | Operation pattern  | Strategy 2 provable? | Reason                                               |
| -------------- | ------------------ | :------------------: | ---------------------------------------------------- |
| Bubble sort    | Adjacent swap      |          ✅          | violation_count strictly decreases per swap          |
| Insertion sort | Adjacent shift     |          ✅          | Each shift eliminates one violation pair             |
| Selection sort | Non-adjacent swap  |          ❌          | Single swap may increase violation_count             |
| Quicksort      | Partition division |          ❌          | Non-adjacent swap, no monotonic decrease guarantee   |
| Heapsort       | Sift-down          |          ❌          | Tree-shaped operation, violation_count non-monotonic |

**Complementary strategies**: For quicksort, the `low < high` interval contraction is covered by
Strategy 1 (linear rank function)—each outer partition recursion halves the interval. Strategies 1
and 2 complementarily cover; the termination of most practical algorithms can be proved by one or
the other. But generalization of Strategy 2 (non-adjacent operations, tree-shaped operations)
remains an open problem.

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

#### 6.4 Strategy 3: Bounded Increment/Decrement Patterns

`v += const` (positive constant), with upper bound type annotation on the variable → metric
`upper_bound - v` decreases by `const` each time, lower bound 0. This is a degenerate case of
Strategy 1; the compiler handles it first and quickly.

#### 6.5 Strategy 4: Multiplicative Scaling Metric Template

`v *= const` (const > 1), with upper and lower bound type annotations on the variable. The compiler
has a built-in logarithmic metric template `ceil(log_const(upper/v))`; each multiplication by
`const` decreases the metric by 1.

```yaoxiang
mut i: Positive(i) = 1
while i < n {
    # Compiler automatically derives: metric ceil(log₂(n/i)), each multiplication by 2 decreases the metric by 1
    i *= 2
}
```

#### 6.6 Separation of Termination and Correctness

Termination proof and correctness proof are independent:

- **Termination**: The four strategies above automatically prove the loop exits in finite steps
- **Correctness**: Whether the loop body progresses toward the target type, checked by the
  compile-time proof pipeline through verification conditions

Both pass → compilation passes. Termination proved but correctness fails → compile error +
counterexample. Correctness proved but termination cannot be proved → compile error pointing out the
variable or operation that cannot be analyzed. Both fail → compile error reporting both failure
reasons separately.

#### 6.7 Termination Checking for Recursive Functions

For recursive functions that need to be evaluated at compile time, the compiler checks parameter
decrease:

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)  # Compiler analyzes: n-1 < n → decreasing → terminates
}

# Compile-time use—compiler guarantees factorial terminates at compile time
vec: Vec(factorial(5)) = Vec(120)()  # 5! = 120, done at compile time
```

| Scenario                                              | Behavior                    |
| ----------------------------------------------------- | --------------------------- |
| Compiler can analyze recursive decrease (e.g., `n-1`) | Compile-time evaluation     |
| Not decreasing / cannot determine decrease            | Compile error               |
| Runtime call (non-type position)                      | No termination check needed |

#### 6.8 Hard Boundary

`i = f(i)` where `f` is non-invertible, non-closed, and preserves no monotonicity—mathematically
impossible to automatically prove termination. Compile error:

> This loop cannot be automatically proved to terminate. The loop variable depends on the
> unanalyzable function `f`. Please use iteration patterns that can be analyzed by the compiler.

This is not the compiler's failure. Any code that cannot be statically proved safe must not pass
compilation.

### 8. SMT Solver: Accelerator Module of the Type Checker

In traditional languages, SMT solvers are external tools (e.g., F\* calls Z3, Dafny calls Z3). In
YaoXiang, it is an **accelerator module of the type checker**—invoked only when the compiler kernel
itself cannot directly decide. SMT helps find proofs, but the proof is verified by the type checker.

**Trust model**: The type checker is the only trust root. The SMT solver is an accelerator module—it
helps find proofs, but SMT is not a separate trust boundary. The compiler trusts Z3's `unsat` result
(consistent with the F\*/Dafny line—the probability of Z3 going wrong is lower than the compiler's
own bug rate; an engineering pragmatic choice). The real unreliability control lies in the SMT
translation layer—if the translation has a bug, the compiler will be exposed in other tests.

**Interface**: The compiler internally translates to the SMT-LIB 2.6 standard format, rather than
binding to a specific solver API. SMT-LIB is an ISO standard; Z3, CVC5, MathSAT, and Yices all
support it natively.

**Default backend**: Z3 (MIT license, most extensive documentation and community validation). CVC5
as an SMT-LIB-compatible alternative—users can switch via compiler flags at compile time.

No "universal solver abstraction layer"—SMT-LIB is the abstraction layer. In the future, if CVC5
makes breakthroughs in specific theories, switching only requires swapping the binary, not changing
compiler code.

```
Compile-time Bool expression
        │
        ├── Compiler kernel can directly decide (structural equivalence, simple arithmetic,
        │   trivially-folded formulas)
        │   → Directly return Proved / Disproved
        │
        └── Compiler kernel cannot directly decide (quantifiers, symbolic variables)
            → Dependent type pre-reduction (factorial(5) → 120)
            → Translate to SMT-LIB format
            → Send to Z3/CVC5 (with budget limit)
            → Return value: unsat → Proved  │  sat + model → Disproved  │  unknown → Unproven
```

**Solver budget—hard limit, like stack depth**:

| Budget dimension               | Default | Description                                                                                                                                                         |
| ------------------------------ | ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Solver steps                   | 10,000  | Z3 typically handles linear arithmetic in hundreds of steps. 10,000 covers 99% of practical predicates.                                                             |
| Time                           | 100ms   | A single predicate exceeding 100ms = the user is writing a compile-time program, not a type annotation. 100ms × 50 predicates = 5 seconds compile time upper limit. |
| Quantifier instantiation depth | 3       | Three-level nested quantifiers cover practical patterns. Beyond three levels, the user is likely writing logic puzzles.                                             |

Exceeding the budget returns `Unproven`, compile error + predicate location + consumption. No
degradation, no runtime check, no silent pass.

**Why this is actually feasible**: In engineering, 95% of practical predicates are linear
arithmetic—`x > 0`, `arr.len > 0`, `0 <= idx < arr.len`—all within decidable fragments; SMT solvers
return these in milliseconds. For the rare complex predicates that exceed the budget, the programmer
can write proof functions.

Dependent types have a pre-reduction layer before the SMT call: `factorial(5)` directly evaluates at
compile time to `120`; `append([1,2], [3])` directly evaluates to `[1,2,3]`. These deterministic
value computations do not consume SMT budget.

Programmers do not need to know SMT exists. The mental model is: **the compiler can prove it, then
pass; cannot prove it, then report an error—if the compiler does not know, you can write a function
to prove it to it.**

### 9. Compile-Time Predicate Composition

Compile-time predicates are functions returning `Type`; composition is naturally implemented through
function composition:

```yaoxiang
SortedNonEmpty: (T: Ord, arr: Array(T)) -> Type = {
    Sorted(T, arr) and NonEmpty(arr)
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

### 11. dispatch Dispatch Pipeline: Unified Dispatch of Compile-Time and Runtime

`assert` and `Assert` are two sides of the same refinement type primitive. The dispatch pipeline
`dispatch` automatically decides between compile-time proof and runtime check based on **whether the
predicate's free variables are accessible at compile time**:

| Criterion                                                                              | Mode            | Behavior                                                                                 |
| -------------------------------------------------------------------------------------- | --------------- | ---------------------------------------------------------------------------------------- |
| All free variables known at compile time (generic parameters, compile-time constants)  | **CompileTime** | Enter proof pipeline: Proved → erase, Disproved → compile error, Unknown → require proof |
| Free variables exist from runtime (function parameters, external input, mut variables) | **Runtime**     | Insert runtime check, and inject refinement facts into flow-sensitive assumption set Γ   |

**Key**: "Cannot determine" ≠ "Disproved". In CompileTime mode, Unknown requires proof (no silent
degradation); in Runtime mode, the proposition is not even true or false at compile time—no matter
how strong the prover is, it cannot write a universally true proof for "the user may have entered a
negative number"; a runtime check is the only sound choice. This is not the prover being
insufficiently strong; it is a theoretical necessity.

### 12. Flow-Sensitive Assumption Set Γ: Strongest Postcondition Propagation

The compiler maintains a flow-sensitive assumption set Γ, tracking propositions known to hold at
each control-flow point.

**SP (strongest postcondition) propagation**:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
```

**Kill set for mut variables**: After a `mut` variable is reassigned, all assumptions involving that
variable are removed from Γ:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
mut x = x - 5       // Γ = {}  ← x > 0 killed
```

This is a hard requirement of soundness—the variable's value has changed, and old assumptions are
invalid.

**Branch confluence**: When IF/ELSE or match branches merge, Γ takes the intersection of each
branch's assumptions. Only propositions that hold on all paths are carried out of the branch.

### 13. Erasure Model Clarification: Witness Erasure ≠ Check Erasure

The RFC-027 assertion that "refinement types are **fully erased** at runtime" refers to the **proof
witness (proof token)**—proof terms verified at compile time do not generate runtime code. However,
the **runtime check** inserted by the dispatch in Runtime mode is preserved—it is a `Bool` check
executed at the value level, not a witness at the type level.

Summary: witness is erased, check is preserved. The two are not in conflict; the original RFC-027
assertion is unchanged.

## Detailed Design

### Syntax Changes

| Before (RFC-022)                      | After (this RFC)                                                             |
| ------------------------------------- | ---------------------------------------------------------------------------- |
| `//! requires: NonEmpty(n) = n > 0`   | Compile-time predicate as parameter type `(b: Positive(b))`                  |
| `//! ensures: ExistsMax(result, arr)` | Return type uses return value parameter `-> (result: IsMax(T, arr, result))` |
| `/*! invariant: ... !*/`              | Compile-time predicate type annotation on variables—Floyd-Hoare invariant    |
| `//! decreases: n`                    | Compiler fully automatic metric function derivation                          |
| Specifications are comments           | Specifications are the type system                                           |

### Syntax

**Compile-time predicates have no new keyword.** `{}` is the proof space, identical to the existing
type definition syntax. A compile-time predicate is simply a function returning
`Type`—`name: (params) -> Type = { assertions }`. Usage is just a function call—`Positive(b)`,
`IsMax(T, arr, result)`.

```bnf
# Compile-time predicate = function returning Type, {} contains compiler-verified assertions
# Uses existing function/type syntax, no new BNF rules needed
predicate ::= identifier ':' params '->' 'Type' '=' '{' assertions '}'
```

**New syntax concept: return value parameter**—`name` in `-> (name: Type)` is a return value
parameter.

The return value parameter is the **only syntax concept** YaoXiang introduces on top of the existing
function syntax. Its semantics:

- The value of `name` is provided by the `return` statement
- `name` exists only in the type signature, referenced by postcondition predicates (e.g.,
  `-> (result: IsMax(T, arr, result))`)
- `name` does not enter the function body scope, and does not appear at the caller
- The return value parameter is **optional**—when there is no postcondition, the signature is
  identical to a normal function (`-> Int`), with no extra burden

The reason for introducing it: postconditions need to reference "the value the function will
return". Without a return value parameter, the compiler could only let predicates reference the
return value through special rules (such as implicit variables `$result` or `__retval__`). The
return value parameter makes this reference explicit—it is just a parameter, only the value is
provided by `return` rather than by the caller.

**Proof function** is not a new concept—it is just a YaoXiang function whose return type is the
proposition being asserted. When the compiler returns `Unproven`, the programmer provides a proof
function, and the type checker verifies it in exactly the same way as it verifies any function's
return type. No new syntax, no new keywords, no new rules.

### Type System Impact

- **Type universe**: Compile-time predicates sit at the `Type₂` layer—functions taking values and
  returning `Type`, at the same level as type constructors
- **Generics interaction**: Compile-time predicates can carry generic parameters, e.g.,
  `NonEmpty: (T: Type) -> (arr: Array(T)) -> Type`
- **Ownership interaction**: Expressions in compile-time predicates obey ownership rules, read-only,
  no write
- **Type inference**: Parameters of compile-time predicates participate in HM type inference

### Runtime Representation

Compile-time predicates are handled at runtime **according to the dispatch result**:

- **CompileTime mode** (all free variables known at compile time): After proof passes, the witness
  is completely erased. `Positive: (x: Int) -> Type = { x > 0 }`—parameter `b: Positive(5)` is
  represented at runtime as just `Int`. The refinement condition `{ 5 > 0 }` has passed, erased.
- **Runtime mode** (free variables from runtime exist): Preserve the runtime check—execute a `Bool`
  check at the value level, inject into flow-sensitive assumption set Γ. See §11 dispatch pipeline
  and §13 erasure model clarification for details.

Putting a compile-time predicate in type position (e.g., `f(x: Positive(x))`) does not produce a
wrapper type, does not allocate extra memory. But when `x` comes from runtime input, a runtime
`Bool` check **will** be inserted.

**Interaction constraint with `ref`**: Compile-time predicates can only reference values with an
immutable borrow or whose ownership has been transferred. A compile-time predicate referencing a
value with a mutable borrow cannot guarantee at compile time that the verification result still
holds at runtime—such usage directly reports a compile error.

### Compiler Changes

1. **Parser**: Compile-time predicates use standard function syntax, no additional parsing rules
   needed
2. **Compile-time proof pipeline**: Unified `Proved/Disproved/Unproven` return interface, automatic
   strategy selection
3. **SMT accelerator module**: SMT-LIB 2.6 translation layer, default backend Z3, CVC5 as
   alternative
4. **Type checker kernel**: Inference rule implementation—structural equivalence, βδι-reduction,
   universal quantifier introduction/elimination. This is the only trust root; SMT and programmer
   proofs are all verified through this
5. **Verification condition generation**: WP/SP calculus + loop invariant proof obligations
6. **Error reporting**: Counterexample formatting + unsolved proposition report + source location
   correlation

### Backward Compatibility

- ✅ Code that does not use compile-time predicates is completely unchanged
- ✅ Compile-time predicates in CompileTime mode have zero runtime overhead; Runtime mode only
  retains necessary `Bool` checks
- ⚠️ RFC-022's `//!` syntax is no longer supported—but 022 was never implemented, so there is no
  migration burden

## Trade-offs

### Advantages

- **Curry-Howard correspondence fully realized**: Types are propositions, programs are proofs,
  `name: Proposition = Proof`
- **Unity**: Compile-time predicates and ordinary functions use exactly the same syntax, no
  conceptual split
- **SMT transparency**: Programmers do not need to know SMT exists; the mental model is consistent
  with type checking
- **Progressive adoption**: Start with one compile-time predicate, gradually increase coverage
- **Minimal runtime overhead**: CompileTime mode has zero overhead; Runtime mode only retains
  necessary `Bool` checks

### Disadvantages

- **Compile time**: SMT solving increases compile time, but hard budget limits guarantee the upper
  bound
- **Automatic proof boundaries**: Complex predicates beyond first-order linear arithmetic may
  require the programmer to write proof functions. This is not a language defect—this is the
  necessary conclusion of the halting problem. The compiler honestly reports `Unproven` rather than
  falsely reporting `True/False`
- **Learning curve**: Writing effective compile-time predicates and proof functions requires
  understanding the basic intuition of the Curry-Howard correspondence
- **Implementation complexity**: Unification of the compile-time proof pipeline requires careful
  design

### Risk Mitigation

- SMT solver budget hard limit (10,000 steps / 100ms / instantiation depth 3), exceeding the budget
  returns `Unproven`
- Dependent type pre-reduction: deterministic value computation is consumed first, SMT only chews on
  the non-deterministic part
- `Unproven` is not a dead end: the programmer can write a proof function, and the type checker
  verifies it—consistent with verifying any function's return type
- Incremental verification: only changed modules are verified
- Clear error messages + counterexample display + budget consumption report + unsolved proposition +
  suggestions (if the compiler can provide them)

## Alternatives

| Option                                                             | Why not chosen                                                                                                                                                                  |
| ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| RFC-022: `//!` comment-style specifications                        | Specifications and types are split, violating the Curry-Howard correspondence                                                                                                   |
| Independent specification files (e.g., CVL)                        | Specifications and code separated, increasing maintenance cost                                                                                                                  |
| Runtime-only assertions                                            | Cannot statically guarantee correctness                                                                                                                                         |
| External proof assistants (e.g., Coq)                              | Disconnected from the compiler, requires an independent proof language and trust boundary. YaoXiang's choice: proofs are YaoXiang code, the type checker is the only trust root |
| **This proposal: compile-time predicates as first-class citizens** | ✅                                                                                                                                                                              |

## Implementation Strategy

### Phasing

| Phase       | Content                                                                                                                                                                 |
| ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Phase 1** | Compiler kernel: structural equivalence + βδι-reduction + universal quantifier introduction/elimination. Supports simple arithmetic predicates (`x > 0`, `arr.len > 0`) |
| **Phase 2** | SMT-LIB translation layer + Z3/CVC5 integration. Pipeline returns Proved/Disproved/Unproven. When Unproven, support programmer writing proof functions                  |
| **Phase 3** | Loop invariant VC generation + termination checking (linear rank function + predicate violation counting + bounded patterns + combinatorial explosion control)          |
| **Phase 4** | Incremental verification + caching + IDE support                                                                                                                        |

### Dependencies

- RFC-010: Unified Type Syntax — compile-time predicates are based on `name: type = value`
- RFC-011: Generics System — compile-time predicates can carry generic parameters
- RFC-009: Ownership Model — expressions in compile-time predicates obey ownership rules

## Open Questions

- [x] **SMT solver choice**: Default Z3 (MIT license, most extensively validated). CVC5 as an
      SMT-LIB-compatible alternative, switchable via compiler flag. The compiler's internal
      translation target is the SMT-LIB 2.6 standard format—SMT-LIB is the abstraction layer, no
      custom universal solver interface.
- [x] **Specific solver budget values**: 10,000 steps / 100ms / quantifier instantiation depth 3.
      Hard-coded inside the compiler, no knob. If actual use proves it insufficient (not "the user
      wrote it wrong") in real cases, it will be adjusted.
- [x] **Quantifier support scope**: The language layer does not limit the quantifier rank.
      Compile-time predicates accept `Type` parameters—`Type` includes function types—therefore
      higher-order quantifiers are a natural inference of the type system, no special syntax needed.
      SMT solver can automatically decide first-order quantifiers (forall/exists, supports
      interleaved nesting, limited by budget depth 3). Higher-order quantifiers: SMT returns
      `Unproven`, compiler prompts "this predicate exceeds the automatic proof range, please provide
      a proof function". Programmer writes a YaoXiang function whose return type equals the
      proposition—type checker verifies that function. No external export needed, no AI needed, no
      interactive proof mode needed. Everything is YaoXiang code, everything is verified by the type
      checker.
- [x] **Counterexample formatting**: Source variable names are used directly as SMT variable names
      (with module prefixes to avoid conflicts). When Z3 model returns, look up by variable name.
      Output format: variable name = specific value + source location + predicate definition
      location. No complex mapping layer.
- [x] ~~**Interaction between compile-time predicates and `ref` smart pointers?**~~ → Decided:
      compile-time predicates only allow immutable borrows or values with transferred ownership.
      Values with mutable borrows cannot appear in compile-time predicates.
- [x] **Extension of forall predicate violation counting metric to non-adjacent operations?** → Not
      extended. Current coverage (adjacent swap, adjacent shift) is complementarily covered by
      Strategy 1 (linear rank function)—quicksort's outer interval contraction is backed by Strategy
      1, heapsort is backed by Strategy 1 (array index pattern). Loops whose termination cannot be
      proved by any strategy are directly reported as errors by the compiler—this is the hard-safety
      philosophy, not a defect. If in the future there are real-world scenarios (not academic
      constructions) where no strategy can cover the algorithm, it will be revisited.
- [x] **Linear rank function enumeration combinatorial explosion**: The candidate enumeration limit
      is 3 bounded variables. ≤3: enumerate all linear combinations and verify each via SMT. >3:
      only attempt single-variable metrics (`v_i`, `u_i - v_i`), failure directly reports a compile
      error—prompting the programmer "the loop has >3 bounded variables, the compiler cannot
      automatically synthesize multi-variable metrics". This is not an engineering compromise—it
      forces the programmer to write simpler loops.

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
│ (formal design) │ │ (kept in place) │
└─────────────┘    └─────────────┘
```
