---
title: 'RFC-027: Compile-Time Predicates and Unified Static Verification'
status: 'Accepted'
author: 'ChenXu'
created: '2026-06-07'
updated: '2026-07-05'
impl_status: 'in_progress'
impl_detail:
  'Phase 1-2 complete, Phase 3 partially complete, Phase 4 partially complete. All 6 phases of the
  assert/Assert unification plan implemented (#157-#162 closed): Never type, IsTrue bridging,
  flow-sensitive Γ + kill set, type-level recursion, universe hierarchy weak check, dispatch
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
> - [RFC-010: Unified Type Syntax — the `name: type = value` Model](../accepted/010-unified-type-syntax.md)
> - [RFC-011: Generics System Design](../accepted/011-generic-type-system.md)
> - [RFC-024: Concurrency Model Based on `spawn` Blocks](../accepted/024-concurrency-model.md)
>
> **Supersedes**:
> [RFC-022: Hoare-Logic Static Verification Support (Specification Comments and Specification Types)](../deprecated/022-hoare-logic-static-verification.md)
> — deprecated

## Summary

This RFC proposes introducing **compile-time predicates** as first-class citizens to YaoXiang,
unifying all compile-time static verification into a single **proof pipeline**. Compile-time
predicates are not external specification comments—they _are_ functions. A function that returns
`Type` can be used in a type position; the compiler calls it at compile time and checks the return
value. Types are propositions; compile-time evaluation is proof.

**Core thesis**: The type checker's sole job at compile time is to construct and verify proof terms.
Type equality, token conflicts, dependent type reduction, compile-time predicate evaluation,
Hoare-logic implication—all are different type checks within the compile-time proof pipeline,
sharing the same pipeline. The SMT solver is an accelerator module for the type checker, not a
separate trust boundary. When the compiler returns `Unproven`, the programmer writes a YaoXiang
function as a proof—the type checker verifies it in exactly the same way as it verifies any
function's return type. Everything is YaoXiang code; everything is verified by the type checker.

## Motivation

### Why Deprecate RFC-022?

RFC-022 designed specifications as `//!` comments:

```yaoxiang
max: (T: Ord) -> ((arr: Array(T, n)) -> T) = {
    //! requires: NonEmpty(n) = n > 0          ← This is a type-independent comment
    //! ensures: ExistsMax(result, arr[0..n])   ← This is a type-independent comment
}
```

This commits the fundamental sin of violating the Curry–Howard correspondence: **it splits
specifications and types into two layers**. Comments are not types. Comments do not participate in
type checking. Comments reflect an "external tool" mental model.

The white paper says it clearly:

> "No `//!` comments. No standalone specification language. Everything lives within the type
> system."

### Current Problems

- RFC-022's `//!` comments are external syntax outside the type system
- Specification types and ordinary types are two systems, creating conceptual redundancy
- The Debug Build verification / Release Build ignore split undermines unity
- SMT solvers are positioned as external tools in conventional wisdom—YaoXiang builds them in as
  accelerator modules of the type checker
- Type checking, borrow verification, compile-time predicate checking, and macro expansion each take
  different paths

### The Correct Mental Model

Type checking can be abstracted as a function:

```
verify : Program → Proved | Disproved(Model) | Unproven
```

All compile-time checks—simple type matching, borrow conflict detection, compile-time predicate
verification—are subtasks of this function. They share the same proof pipeline, differing only in
proof-term complexity and construction strategy.

When the compiler returns `Unproven`, the programmer supplies a proof function—its return type
equals the proposition to be proven. The type checker verifies it. This is the same operation as
ordinary type checking.

## Proposal

### 1. `{}` Is Proof Space: Types Are Assertions, Verification Is Type Checking

YaoXiang's `{}` is the compile-time proof space. Everything inside is an assertion; the compiler
guarantees each item is `True`—either by automatic proof or by programmer-supplied proof functions.

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
#          parameter in signature  only assertions inside {}
#          compiler verifies x > 0 at call site

List: (T: Type) -> Type = { data: Array(T) }
#      ^^^^^^^^              ^^^^^^^^^^^^^^^
#      parameter in signature   compiler verifies type_of(T) == Type, type_of(data) == Array(T)
```

Same pattern: `name: (params) -> Type = { assertions }`. The compiler does not distinguish "type
assertions" from "value assertions"—both are evaluation targets within the proof pipeline.

**Loop invariants need not be written separately. Type annotations on variables _are_ Floyd–Hoare
invariants.**

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # Annotation references i—tells compiler s's type depends on i
    mut i: UpTo(arr.len) = 0     # At init i=0, verify: 0 == sum(arr[0..0]) → True
    while i < arr.len {
        s += arr[i]  # Compiler verifies: s_new == sum(arr[0..i+1])
        i += 1       # i changes → triggers s dependency reverification: s satisfies SumUpTo(arr, i_new)
    }
    return s  # s: SumUpTo(arr, arr.len) = sum(arr[0..arr.len])
}
```

The compiler generates one verification condition for the loop body—inductive hypothesis (type
annotation) → assignment operation → whether the new value satisfies the type annotation. Once the
proof pipeline validates the inductive step, all iterations are automatically covered. No
`: decreases`, no `: Invariant`, no inductive proof needed—the compiler decomposes induction into a
local VC per assignment.

### 2. Pre/Postconditions: Compile-Time Predicates on Parameter Types and Return Types

Abandoning RFC-022's `//! requires`/`//! ensures`. Compile-time predicates serve as type annotations
on parameters or returns.

**The parameter side is a function call.** A compile-time predicate is a function returning `Type`;
its use on the parameter side is simply calling it—just like `factorial(5)`. The return value side
introduces a new concept: the return value parameter.

```yaoxiang
# Precondition: explicit compile-time predicate call in parameter type
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b
#                       ^^^^^^^^^^  b is the current parameter name, passed to Positive
#                       Compiler extracts argument value at call site, substitutes into b, verifies Positive(arg)
#                       Ex: divide(10, 2) → verify Positive(2) = { 2 > 0 } → True
#                       Ex: divide(10, 0) → verify Positive(0) = { 0 > 0 } → False → compile error

# Postcondition: return value parameter + compile-time predicate
IsMax: (T: Ord, arr: Array(T), result: T) -> Type = {
    forall j in 0..arr.len: result >= arr[j]
}

NonEmpty: (arr: Array(T)) -> Type = { arr.len > 0 }

max: (T: Ord) -> ((arr: NonEmpty(arr))) -> (result: IsMax(T, arr, result)) = {
#                                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
#                                            result is return value parameter, value supplied by return
#                                            Compiler substitutes return value at return point, verifies postcondition
    candidate = arr[0]
    for i in 1..arr.len {
        if arr[i] > candidate { candidate = arr[i] }
    }
    return candidate
}
```

**Key rules**:

- **Parameter side**: `b: Positive(b)` — `b` is the current parameter name, passed to `Positive` as
  an argument. Function call syntax, zero implicit behavior.
- **Return side**: `-> (result: IsMax(T, arr, result))` — `result` is the return value parameter,
  its value supplied by the `return` statement. `result` exists only in the type signature, only
  referenced by the predicate, and does not enter the function body scope nor appear at the call
  site.
- **Return value parameter is optional**: omit it when there is no postcondition; the signature is
  identical to an ordinary function (`-> Int`).
- **Unity**: parameters and return value parameters are the same
  concept—`paramName: predicateCall(paramName)`—the only difference being whether the value is
  supplied by the caller or by `return`.

### 3. Path Condition Propagation: Compile-Time Verification of Runtime Values

When a compile-time predicate is used in a binding position, the parameter is explicitly supplied by
the programmer. When a runtime value enters a refinement-type parameter, the compiler completes
verification through path-condition collection and SMT implication judgment—no explicit proof from
the programmer needed.

#### 3.1 Explicit Function Call

When a compile-time predicate is used in a binding position, the parameter is explicitly supplied by
the programmer—just a function call, zero implicit behavior.

`Positive: (x: Int) -> Type = { x > 0 }` is a compile-time predicate constructor. When it appears in
a binding position (parameter declaration, variable declaration, return type), the programmer
explicitly passes the already-bound variable name:

```yaoxiang
b: Positive(b)
// b has been declared as the current parameter, Positive(b) is a function call
// After normalization: b: { b > 0 }
```

No implicit parameter filling by the compiler—`b: Positive(b)` is just a function call, the same as
`f(5)`. `b` is bound as a parameter name, and its type annotation `Positive(b)` references `b`
itself—this is the standard dependent-types pattern, not an implicit expansion rule.

**Unified with RFC-010's `self`**: RFC-010 established that `self` is not a keyword, just a
conventional parameter name ("writing it as `p`, `this`, or `x` has exactly the same effect").
`b: Positive(b)` shares the same mechanism—the parameter name is referenceable in type annotations.
`self` appears in `self: Point`, `b` appears in `b: Positive(b)`; both type annotations reference
the parameter itself. The only difference is annotation complexity; the mechanism is identical—once
the name is bound, the type can depend on that name.

The return type also uses explicit function calls:

```yaoxiang
Sorted: (arr: Array(T)) -> Type = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }

sort: (arr: Array(T)) -> (result: Sorted(result)) = { ... }
//                        ^^^^^^^^^^^^^^^^^^^^^^^
//                        result is return value parameter, Sorted(result) is a function call
//                        Compiler substitutes return value at return point, verifies Sorted(return value)
```

The same applies to local variable declarations:

```yaoxiang
let x: Positive(x) = 5
// x bound to 5, Positive(5) → { 5 > 0 } → True → pass

// let y: Positive(y) = 0
// y bound to 0, Positive(0) → { 0 > 0 } → False → compile error
```

#### 3.2 Path Condition Collection

When a runtime value appears in a conditional branch, the compiler automatically collects path
conditions to form the **assumption set** for the current scope. These assumptions participate in
verification as background knowledge for compile-time `Bool` evaluation.

```yaoxiang
if y > 0 {
    // Compiler automatically acquires the assumption in this branch: { y > 0 }
    let result = divide(x, y)
    // Verification condition: (y > 0) ⇒ (y > 0)
    // Proof pipeline judges implication holds → Proved
} else {
// This branch assumes: { !(y > 0) }
// If we want to call divide(x, y), verification condition is !(y > 0) ⇒ y > 0
    // Proof pipeline judges implication does not hold → Disproved
}
```

This is not the compiler hardcoding special patterns—this is the natural behavior of the
compile-time proof pipeline. At each type-check call site, the pipeline sends:

```
{background assumptions} ⇒ {verification target}
```

The proof pipeline judges implication. `Proved` → pass, `Disproved` → compile error +
counterexample, `Unproven` → compile error + unsolved proposition. Background assumptions come from
the path conditions at the current program point.

#### 3.3 Assumption Stack

During control-flow analysis, the compiler maintains an assumption set for each basic block:

- **if-guard**: `if y > 0` → true branch pushes `y > 0`, false branch pushes `!(y > 0)` (when `else`
  is used)
- **match patterns**: `if let Some(v) = opt` → branch pushes `opt == Some(v)`
- **Logical conjunction**: `if x > 0 and y < 10` → branch pushes `x > 0` and `y < 10`
- **Function preconditions**: when calling `divide(a, b)`, evidence that `b` satisfies `Positive`
  must come either from current assumptions or from the argument's own refinement type annotation
  (if `b` is annotated as `Positive`, its type carries `b > 0`)
- **Assignment**: when `let z = y`, refinement conditions already on `y` propagate to `z`

All assumptions enter the compile-time proof pipeline. When entering the SMT acceleration path, they
are translated to SMT-LIB background assertions.

#### 3.4 No Static Evidence → Compile Error

If the programmer writes directly:

```yaoxiang
divide_user_input: (x: Int, y: Int) -> Int = divide(x, y)
```

There is no `y > 0` assumption at the current program point, and the argument `y` itself carries no
`Positive` annotation. The verification condition is:

```
{} ⇒ { y > 0 }
```

The pipeline returns `Disproved` (implication does not hold) → compile error:

> Cannot prove that argument `b` satisfies `Positive` in the `divide` call. `y` comes from function
> input, with no proven bound. Consider guarding the call with an if branch:
> `if y > 0 { divide(x, y) }`.

YaoXiang does not allow runtime values to enter refinement-type parameters without static evidence.
This is not a limitation—this is the core of the hard-safety philosophy. Any code the compiler
cannot statically prove is not allowed to pass compilation.

#### 3.5 Relationship to the Unified Pipeline

Path condition propagation is not an additional mechanism. It is the direct extension of the
compile-time proof pipeline in control-flow analysis:

| Stage                             | Responsibility                                                                                                                                           |
| --------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Path condition collection         | Compiler's control-flow analysis stage; annotates each basic block with an assumption set                                                                |
| Verification condition generation | When a type constraint requiring verification is encountered, merges path conditions + argument type info                                                |
| Proof pipeline evaluation         | Compiler kernel → SMT acceleration → derives Proved / Disproved / Unproven                                                                               |
| Result                            | `Proved` → pass; `Disproved` → compile error + counterexample; `Unproven` → compile error + unsolved proposition (programmer may provide proof function) |

No new components. No special rules. Path conditions are the background knowledge of the proof
pipeline—sharing the same pipeline and budget system with type equality and borrow constraints.

### 4. The Compile-Time Proof Pipeline

All compile-time checks share a single pipeline. The pipeline's core operation is **type
checking**—checking whether a proof term's type equals the proposition to be proven. Everything is
type checking.

```
Compile-time encounters Bool expression needing evaluation (i.e., a proof term must be constructed)
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
        │   → Compiler itself + SMT accelerator module
        │
        └── Hoare-logic implication (P ⇒ Q)
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
         Programmer writes proof function (YaoXiang code)
                  │
                  ▼
         Type checker verifies ──→ Proved ──→ Compilation passes
                  │
                  ▼
            Verification fails → Compile error: "proof does not hold"
```

#### 4.1 Proof Result: A Three-Valued Algebra

Compile-time evaluation returns three results—an inevitable conclusion of the halting problem and a
natural partition of proof theory:

```
eval_compile_time : BoolExpr → Proved | Disproved(Model) | Unproven
```

- **Proved** → halted, proof term constructed, type check passed. Compilation continues.
- **Disproved(M)** → halted, counterexample M exists. Compile error + counterexample + source
  location.
- **Unproven** → proof not constructed within the given resource limit. Compile error + unsolved
  proposition + budget consumption report.

**Unproven ≠ False.** The compiler saying "I cannot prove it" is not equivalent to the proposition
being false—it's just beyond the current automatic prover's capability. This is honesty, not a
defect.

A hard budget limit is the engineering solution to the halting problem. No knobs—providing them
amounts to asking the user "do you think your program will halt", and neither the user nor the
compiler knows.

#### 4.2 After Unproven: The Programmer Writes the Proof

When the compiler returns `Unproven`, the programmer can write a **proof function**—a YaoXiang
function whose return type equals the proposition to be proven. The type checker verifies this
function—using exactly the same mechanism as it verifies `add(a, b): Int`.

```
Proposition = Type
Proof       = Program (a value of that type)
Verification = Type check (the sole trust root)
```

The SMT solver is not an independent trust boundary—it is an **accelerator module of the type
checker**. SMT helps find proofs, but the proof verifier is always the type checker. When SMT
returns `unsat`, the compiler reconstructs its result as a proof term verifiable by the type
checker. If reconstruction fails (SMT's reasoning steps exceed the compiler kernel's inference
rules), it falls back to `Unproven`—the programmer can manually write the proof function.

```yaoxiang
# Proposition: a refinement property the compiler cannot automatically prove
FirstIsMin: (T: Ord, arr: Sorted(T)) -> Type = {
    forall i in 0..arr.len: arr[0] <= arr[i]
}

# Proof: programmer writes a function whose return type is the proposition above
# Type checker verifies this function—exactly as it verifies add(a,b): Int
first_is_min: (T: Ord, arr: Sorted(T)) -> FirstIsMin(T, arr) = {
    # Compiler verifies here: the function body's type = FirstIsMin(T, arr)
    ...
}
```

No AI needed, no exporting to Coq, no new concepts. **Compile-time properties that cannot be
auto-proven → programmer writes proof in YaoXiang code → type checker verifies.** The whole process
is a smooth gradient—the compiler handles simple proofs, leaving the brain for the hard ones.

#### 4.3 Layered Dependencies Within the Pipeline

The evaluators above share the same interface but have an evaluation order. Type equality is the
prerequisite for all subsequent analysis; ownership/token checks depend on type information;
refinement predicate verification depends on the first two layers' results. The compiler evaluates
layer by layer; expressions failing at lower layers do not enter higher layers—avoiding wasted
solver budget on type-erroneous programs.

```
Evaluation order (same pipeline, layered scheduling)
├── Layer 0: Type equality (T1 == T2)
│   └── Structural unification → failure makes the rest meaningless; return Disproved directly
├── Layer 1: Ownership/token conflict
│   └── Flow-sensitive liveness analysis → failure means memory safety fails; return Disproved directly
└── Layer 2: Refinement predicate / Hoare implication
    └── Compiler itself → SMT acceleration → derive Proved / Disproved / Unproven
```

Each layer still returns `Proved/Disproved/Unproven`, sharing the same interface and the same budget
system.

### 5. Unification of the Three Function Layers

| Layer                  | Timing       | Input      | Output | Example                                        |
| ---------------------- | ------------ | ---------- | ------ | ---------------------------------------------- |
| Value-level function   | Runtime      | Value      | Value  | `add: (a: Int, b: Int) -> Int = a + b`         |
| Type constructor       | Compile-time | Type/value | Type   | `List: (T: Type) -> Type = { data: Array(T) }` |
| Compile-time predicate | Compile-time | Value      | Type   | `Positive: (x: Int) -> Type = { x > 0 }`       |

All use the same `name: type = value` syntax. Compile-time predicates and type constructors take the
same compile-time proof pipeline—`{}` is proof space.

### 6. Loops: Floyd–Hoare Verification Condition Generation

Loops do not need separate `: Invariant(...)` or `: decreases(...)` annotations. Compile-time
predicate type annotations on variables define Floyd–Hoare-style assertions—the compiler generates
verification conditions from type annotations, and the proof pipeline checks whether each assignment
preserves the type.

Core mechanism: each assignment operation corresponds to a Hoare triple `{P} x := e {Q}`; the
verification condition is `P ⇒ Q[e/x]`. The compiler generates one verification condition for the
loop body—once the proof pipeline validates the inductive step, all iterations are automatically
covered.

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
        #   Substituting s_new = s_old + arr[i]:
        #     Need s_old + arr[i] == sum(arr[0..i+1])
        #     From inductive hypothesis s_old == sum(arr[0..i]), adding arr[i] to both sides:
        #     sum(arr[0..i]) + arr[i] == sum(arr[0..i+1])
        #   Compiler + SMT: linear arithmetic, millisecond-level → Proved
        #
        # i += 1:
        #   i changes → s's type annotation in dependency graph references i → trigger reverification
        #   New verification target: s satisfies SumUpTo(arr, i_new)
        #   That is, s == sum(arr[0..i_new]), guaranteed by the previous step → Proved
        s += arr[i]
        i += 1
    }
    return s  # At this point s: SumUpTo(arr, arr.len), i.e., s == sum(arr[0..arr.len])
}
```

The loop invariant is the type annotation on the variable—the programmer writes the type, the
compiler checks the inductive step. The compiler need not "discover" invariants, nor "perform
induction automatically"—it decomposes the inductive proof into local verification conditions per
assignment and delegates them to the proof pipeline to divide and conquer.

#### 6.1 Dependency Tracking: Dependent Types on Mutable Variables

The prerequisite of the above mechanism is: the compiler knows `s`'s type annotation
`SumUpTo(arr, i)` references `i`—when `i` changes, `s`'s type constraint changes accordingly. This
requires the compiler to maintain a **type-dependency graph between variables**.

**Data structure**:

```
TypeDepGraph: Map<VarName, Set<VarName>>
# Key is the depended-on variable, value is the set of variables whose type annotations reference that variable
# Example: { i: {s}, j: {s, t}, ... }
```

**Construction**: when the type checker processes `mut v: Pred(... x ...) = init`, it parses free
variable references in the `Pred(...)` arguments. If an argument references another mutable variable
`x` in the current scope, the dependency graph records `x → v`.

**Trigger**: when the depended-on variable `x` is assigned, the compiler:

1. Looks up all variables in the dependency graph that depend on `x`: `{v₁, v₂, ...}`
2. For each `v`, generates a verification condition: does `v`'s current value satisfy the updated
   type `Pred(... x_new ...)`
3. Sends the VC to the proof pipeline

**Assignment order sensitivity**: dependency tracking naturally enforces correct assignment order.
Take `SumUpTo(arr, i)` as an example:

```yaoxiang
# Correct order
s += arr[i]   # s_new satisfies SumUpTo(arr, i+1)
i += 1        # i changes → reverify s satisfies SumUpTo(arr, i_new) → True

# Wrong order—compiler rejects
i += 1        # i changes → reverify s satisfies SumUpTo(arr, i_new)
              # s not yet updated, s_old == sum(arr[0..i_old]) ≠ sum(arr[0..i_new])
              # → Compile error: variable s does not satisfy type SumUpTo(arr, i_new)
s += arr[i]   # Unreachable
```

**Composite dependencies**: a variable can depend on multiple variables. The type annotation
`{ v: Int; v == x + y }` depends on both `x` and `y`—either change triggers reverification.

**Relationship to the proof pipeline**: dependency tracking is the trigger for VC generation, not an
independent verification mechanism. It answers "when do we need to generate a VC"—the proof pipeline
answers "does the VC hold".

### 7. Termination Checking

Fully automatic at compile time. Loops the compiler can prove pass; loops it cannot prove trigger a
direct compile error—the programmer must make loops amenable to the compiler's automatic analysis.
No half-automatic annotation escape hatch.

#### 7.1 Design Principles

The compiler automatically extracts information needed for termination proofs from two sources:

1. **Variable type annotations**: bound constraints in refinement types (e.g., `UpTo(n)` gives upper
   bound `n` and lower bound `0`)
2. **Loop body operations**: operations applied to variables at each iteration

The compiler tries four measure-synthesis strategies in priority order, stopping when one succeeds.

#### 7.2 Strategy 1: Automatic Linear Rank Function Synthesis

When variables have linear bound annotations, the compiler enumerates candidate linear measures and
verifies them via SMT.

```
Input:
  Variables v₁: UpTo(u₁), v₂: UpTo(u₂), ... (variables with upper/lower bounds)
  Loop condition cond
  Set of assignments in the loop body

Algorithm:
  1. Extract each variable's bounds from type annotations: [low_i, high_i]
  2. Enumerate candidate measures: v_i, u_i - v_i, v_i - v_j, etc. linear combinations
  3. For each candidate measure m:
     - SMT verify m ≥ 0 (derived from type bounds)
     - For each execution path of the loop body, SMT verify m' < m (strictly decreasing)
  4. Find a qualifying linear combination → termination proven
```

Coverage: any loop where a variable is assigned a linear expression (`v = a·v + b`) with a bounded
type annotation. This includes `i += const`, `i -= const`, and binary-search-style interval
contraction:

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

#### 7.3 Strategy 2: Predicate Violation Counting—Automatically Extract Measure from Target Type <span style="color:orange">[Experimental Strategy]</span>

> ⚠️ **Current status: experimental strategy; inclusion in Phase 3 implementation depends on actual
> feasibility.** This strategy works for adjacent-swap operations (bubble sort, insertion sort); it
> cannot auto-prove non-adjacent operations (quicksort partition, heapsort sift-down). Coverage
> boundaries are shown in the table below. If Phase 3 verification proves infeasible, this strategy
> will be removed or downgraded to future work.

Core insight: **user-written specifications are material for the compiler to reason about.** The
compiler need not hardcode "what sorting is"—it reads the `Sorted` definition and automatically
extracts a measure from it.

```
Input:
  Target type: Sorted(arr) = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }
  Loop body operation: adjacent element swap

Algorithm:
  1. Parse predicate definition: forall i in range: cond(i, arr)
  2. Auto-generate measure: violation_count = |{ i | ¬cond(i, arr) }|
  3. Analyze the operation's effect on the measure:
     - Adjacent swap arr[j], arr[j+1] = arr[j+1], arr[j]
     - Only affects index pairs j-1, j, j+1
     - If arr[j] > arr[j+1] (predicate violation), the swap satisfies the predicate
     - violation_count decreases by at least 1
  4. Upper bound: n·(n-1)/2 (max adjacent inversions), lower bound: 0
  → Termination proven
```

**Current coverage**:

| Algorithm      | Operation pattern | Strategy 2 provable? | Reason                                               |
| -------------- | ----------------- | :------------------: | ---------------------------------------------------- |
| Bubble sort    | Adjacent swap     |          ✅          | violation_count strictly decreases per swap          |
| Insertion sort | Adjacent move     |          ✅          | Each shift eliminates one violating pair             |
| Selection sort | Non-adjacent swap |          ❌          | A single swap may increase violation_count           |
| Quicksort      | Partition         |          ❌          | Non-adjacent swap, no monotonic decrease             |
| Heapsort       | sift-down         |          ❌          | Tree-shaped operation, violation_count non-monotonic |

**Complementary strategy**: for quicksort, the `low < high` interval contraction is covered by
Strategy 1 (linear rank function)—the outer partition recursion halves the interval each time.
Strategies 1 and 2 complementarily cover most practical algorithms' termination. But generalizing
Strategy 2 (non-adjacent operations, tree-shaped operations) remains an open problem.

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

`v += const` (positive constant) with an upper-bound type annotation on the variable → measure
`upper_bound - v` decreases by `const` per iteration, lower bound 0. This is a degenerate case of
Strategy 1, which the compiler handles first.

#### 7.5 Strategy 4: Multiplicative Scaling Measure Template

`v *= const` (const > 1), with upper/lower-bound type annotations on the variable. The compiler has
a built-in logarithmic measure template `ceil(log_const(upper/v))`; each multiplication by const
decreases the measure by 1.

```yaoxiang
mut i: Positive(i) = 1
while i < n {
    # Compiler auto-derives: measure ceil(log₂(n/i)), each ×2 decreases measure by 1
    i *= 2
}
```

#### 7.6 Separation of Termination and Correctness

Termination proof and correctness proof are independent:

- **Termination**: the four strategies above automatically prove the loop exits in finite steps
- **Correctness**: whether the loop body advances toward the target type, checked by the
  compile-time proof pipeline through verification conditions

Both pass → compilation passes. Termination proven but correctness fails → compile error +
counterexample. Correctness proven but termination cannot be proven → compile error pointing out the
unanalyzable variable or operation. Both fail → compile error reporting both failure reasons.

#### 7.7 Recursive Function Termination Checking

For recursive functions that need to be evaluated at compile time, the compiler checks parameter
decrease:

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)  # Compiler analysis: n-1 < n → decreasing → terminates
}

# Compile-time use—compiler guarantees factorial terminates at compile time
vec: Vec(factorial(5)) = Vec(120)()  # 5! = 120, completed at compile time
```

| Scenario                                             | Behavior                    |
| ---------------------------------------------------- | --------------------------- |
| Compiler can analyze recursive decrease (e.g. `n-1`) | Compile-time evaluation     |
| Not decreasing / decrease cannot be determined       | Compile error               |
| Runtime call (not in type position)                  | No termination check needed |

#### 7.8 Hard Boundary

`i = f(i)` where `f` is non-invertible, non-closed, and preserves no monotonicity—mathematically
impossible to automatically prove termination. Compile error:

> This loop cannot be automatically proven to terminate. The loop variable depends on the
> unanalyzable function `f`. Please use iteration patterns amenable to compiler analysis.

This is not a compiler failure. Any code that cannot be statically proven safe is not allowed to
pass compilation.

### 8. The SMT Solver: Accelerator Module of the Type Checker

In conventional languages, the SMT solver is an external tool (e.g., F\* invokes Z3, Dafny invokes
Z3). In YaoXiang, it is an **accelerator module of the type checker**—invoked only when the compiler
kernel itself cannot directly decide. SMT helps find proofs, but the proof verifier is the type
checker.

**Trust model**: the type checker is the sole trust root. The SMT solver is an accelerator—it helps
find proofs, but SMT is not an independent trust boundary. The compiler trusts Z3's `unsat` result
(consistent with the F\*/Dafny route—the probability of Z3 error is lower than the compiler's own
bug rate, an engineering pragmatic choice). True unreliability control lies in the SMT translation
layer—if the translation has a bug, the compiler will expose it in other tests.

**Interface**: the compiler internally translates to SMT-LIB 2.6 standard format, rather than
binding to a specific solver API. SMT-LIB is an ISO standard, natively supported by Z3, CVC5,
MathSAT, and Yices.

**Default backend**: Z3 (MIT license, most extensive documentation and community validation). CVC5
as an SMT-LIB-compatible alternative—users can switch via compiler flag at compile time.

No "generic solver abstraction layer"—SMT-LIB _is_ the abstraction layer. If CVC5 breaks through in
specific theories in the future, switching only requires swapping the binary, not changing compiler
code.

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

**Solver budget—hard limit, like stack depth**:

| Budget dimension               | Default | Description                                                                                                                                                |
| ------------------------------ | ------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Solver steps                   | 10,000  | Z3 handles linear arithmetic typically within a hundred steps. 10,000 steps cover 99% of real predicates.                                                  |
| Time                           | 100ms   | A single predicate exceeding 100ms = the user is writing a compile-time program, not a type annotation. 100ms × 50 predicates = 5-second compile time cap. |
| Quantifier instantiation depth | 3       | Three-level nested quantifiers cover real patterns. Beyond three levels is likely a logic exercise.                                                        |

Exceeding the budget returns `Unproven`, compile error + predicate location + consumption. No
degradation, no runtime check, no silent pass.

**Why this is actually viable**: in practice, 95% of real predicates are linear arithmetic—`x > 0`,
`arr.len > 0`, `0 <= idx < arr.len`—all within decidable fragments, and SMT solvers return in
milliseconds. For the rare complex predicate that exceeds the budget, the programmer can write a
proof function.

Dependent types get a pre-reduction before the SMT call: `factorial(5)` directly evaluates to `120`
at compile time, `append([1,2], [3])` directly evaluates to `[1,2,3]`. These deterministic value
computations do not consume SMT budget.

The programmer does not need to know SMT exists. Mental model: **the compiler can prove it, it
passes; it can't, it errors—if the compiler doesn't know, you can write a function to prove it to
it.**

### 9. Compile-Time Predicate Composition

Compile-time predicates are functions returning `Type`; composition is naturally achieved through
function composition:

```yaoxiang
SortedNonEmpty: (T: Ord, arr: Array(T)) -> Type = {
    Sorted(T, arr) and NonEmpty(arr)
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
InBounds: (idx: Int, arr: Array(T)) -> Type = { 0 <= idx and idx < arr.len }

get: (arr: Array(T), idx: InBounds(idx, arr)) -> T = arr.data[idx]

arr = Array(Int)(1, 2, 3)
x = get(arr, 1)   # ✅ Compiler verifies InBounds(1, arr) = { 0 <= 1 and 1 < 3 } → True
# y = get(arr, 5)  # ❌ Compiler verifies InBounds(5, arr) = { 0 <= 5 and 5 < 3 } → False
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

### 11. The `dispatch` Pipeline: Unified Dispatch for Compile-Time and Runtime

`assert` and `Assert` are two sides of the same refinement-type primitive. The `dispatch` pipeline
automatically decides between compile-time proof and runtime check based on **whether the
predicate's free variables are reachable at compile time**:

| Criterion                                                                                    | Mode            | Behavior                                                                                  |
| -------------------------------------------------------------------------------------------- | --------------- | ----------------------------------------------------------------------------------------- |
| All free variables known at compile time (generic parameters, compile-time constants)        | **CompileTime** | Enters proof pipeline: Proved → erase, Disproved → compile error, Unknown → require proof |
| Some free variables come from runtime (function parameters, external input, `mut` variables) | **Runtime**     | Insert runtime check, inject refinement facts into flow-sensitive assumption set Γ        |

**Key point**: "Cannot decide" ≠ "disproved". In CompileTime mode, `Unknown` requires proof (no
silent degradation); in Runtime mode, the proposition has no truth value at compile time at all—no
matter how strong the prover is, it cannot write a tautological proof for "the user might have
entered a negative number"; a runtime check is the only sound choice. This is not the prover being
weak—it is theoretical necessity.

### 12. Flow-Sensitive Assumption Set Γ: Strongest Postcondition Propagation

The compiler maintains a flow-sensitive assumption set Γ that tracks propositions known to hold at
each control-flow point.

**SP (Strongest Postcondition) propagation**:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
```

**`mut` variable kill set**: after a `mut` variable is reassigned, all assumptions involving that
variable are removed from Γ:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
mut x = x - 5       // Γ = {}  ← x > 0 is killed
```

This is a hard soundness requirement—the variable's value changed, so old assumptions are invalid.

**Branch confluence**: when IF/ELSE or match branches merge, Γ takes the intersection of each
branch's assumptions. Only propositions holding on all paths propagate out of the branch.

### 13. Clarification of the Erasure Model: Witness Erasure ≠ Check Erasure

RFC-027's claim that "refinement types are **fully erased** at runtime" refers to the **proof
witness**—proof terms already verified at compile time do not produce runtime code. But the
**runtime check** inserted by the `dispatch` in Runtime mode is preserved—it is a `Bool` check
executed at the value level, not a type-level witness.

Summary: witness is erased, check is preserved. The two things do not conflict; RFC-027's original
claim stands.

## Detailed Design

### Syntax Changes

| Before (RFC-022)                      | After (this RFC)                                                             |
| ------------------------------------- | ---------------------------------------------------------------------------- |
| `//! requires: NonEmpty(n) = n > 0`   | Compile-time predicate as parameter type `(b: Positive(b))`                  |
| `//! ensures: ExistsMax(result, arr)` | Return type uses return value parameter `-> (result: IsMax(T, arr, result))` |
| `/*! invariant: ... !*/`              | Compile-time predicate type annotation on variables—Floyd–Hoare invariants   |
| `//! decreases: n`                    | Compiler fully auto-derives measure function                                 |
| Specifications are comments           | Specifications are the type system                                           |

### Syntax

**No new keywords for compile-time predicates.** `{}` is proof space, exactly consistent with the
existing type-definition syntax. A compile-time predicate _is_ a function returning
`Type`—`name: (params) -> Type = { assertions }`. Usage is just a function call—`Positive(b)`,
`IsMax(T, arr, result)`.

```bnf
# Compile-time predicate = function returning Type, {} contains compiler-verified assertions
# Uses existing function/type syntax, no new BNF rules needed
predicate ::= identifier ':' params '->' 'Type' '=' '{' assertions '}'
```

**Predicate application arguments must be in compile-time-constant form**—literals, variables (bound
by name), or single-argument type applications (recursive extraction). Arguments not convertible to
a constant expression report **E1092**; argument count mismatch with predicate parameters reports
**E1093**—refinement constraints are **never silently dropped** (previously, non-convertible
arguments caused constraints to vanish silently, allowing bindings that violated the constraint to
pass quietly).

**New syntax concept: return value parameter**—in `-> (name: Type)`, `name` is the return value
parameter.

The return value parameter is the **single new syntax concept** YaoXiang introduces on top of the
existing function syntax. Its semantics:

- `name`'s value is supplied by the `return` statement
- `name` exists only in the type signature, referenced by the postcondition predicate (e.g.,
  `-> (result: IsMax(T, arr, result))`)
- `name` does not enter the function body scope, nor appear at the call site
- Return value parameter is **optional**—when there is no postcondition, the signature is identical
  to an ordinary function (`-> Int`), introducing no additional burden

The reason for introducing it: postconditions need to reference "the value the function is about to
return". Without a return value parameter, the compiler could only let the predicate reference the
return value through special rules (e.g., implicit variable `$result` or `__retval__`). The return
value parameter makes this reference explicit—it is just a parameter, only the value is supplied by
`return` rather than the caller.

**Proof functions** are not a new concept—they are just YaoXiang functions whose return type is the
proposition being asserted. When the compiler returns `Unproven`, the programmer provides a proof
function, and the type checker verifies it in exactly the same way as it verifies any function's
return type. No new syntax, no new keywords, no new rules.

### Type System Impact

- **Type universe**: compile-time predicates live at the Type₂ layer—functions taking values and
  returning Type, at the same level as type constructors
- **Generics interaction**: compile-time predicates can carry generic parameters, e.g.
  `NonEmpty: (T: Type) -> (arr: Array(T)) -> Type`
- **Ownership interaction**: expressions in compile-time predicates obey ownership rules; they can
  only read, not write
- **Type inference**: compile-time predicate arguments participate in HM type inference

### Runtime Representation

Compile-time predicates are **handled at runtime according to the `dispatch` pipeline result**:

- **CompileTime mode** (all free variables known at compile time): once the proof passes, the
  witness is completely erased. `Positive: (x: Int) -> Type = { x > 0 }`—parameter `b: Positive(5)`
  at runtime is represented simply as `Int`. The refinement condition `{ 5 > 0 }` has passed;
  erased.
- **Runtime mode** (runtime free variables exist): preserve runtime check—execute a `Bool` check at
  the value level, inject into flow-sensitive assumption set Γ. See §11 `dispatch` pipeline and §13
  erasure model clarification for details.

Placing a compile-time predicate in a type position (e.g., `f(x: Positive(x))`) does not produce a
wrapper type, does not allocate extra memory. But when `x` comes from runtime input, a **runtime
`Bool` check is inserted**.

**Interaction constraint with `ref`**: compile-time predicates may only reference immutable borrows
or values with ownership transferred. Compile-time predicates referencing mutable borrows cannot be
guaranteed by the compiler to remain valid at runtime—direct compile error for such uses.

### Compiler Changes

1. **Parser**: compile-time predicates use standard function syntax; no additional parsing rules
   needed
2. **Compile-time proof pipeline**: unified Proved/Disproved/Unproven return interface, automatic
   strategy selection
3. **SMT accelerator module**: SMT-LIB 2.6 translation layer, default backend Z3, CVC5 alternative
4. **Type checker kernel**: inference rule implementation—structural equivalence, βδι-reduction,
   universal quantifier introduction/elimination. This is the sole trust root; SMT and programmer
   proofs are both verified through this
5. **Verification condition generation**: WP/SP calculus + loop invariant proof obligations
6. **Error reporting**: counterexample formatting + unsolved proposition report + source location
   linkage

### Backward Compatibility

- ✅ Code not using compile-time predicates is completely unchanged
- ✅ Compile-time predicates have zero runtime overhead in CompileTime mode; only necessary `Bool`
  checks remain in Runtime mode
- ⚠️ RFC-022's `//!` syntax is no longer supported—but 022 was never implemented, so there is no
  migration burden

## Trade-offs

### Advantages

- **Curry–Howard correspondence fully realized**: types are propositions, programs are proofs,
  `name: Proposition = Proof`
- **Unity**: compile-time predicates use exactly the same syntax as ordinary functions; no
  conceptual split
- **SMT transparency**: programmers need not know SMT exists; mental model matches type checking
- **Progressive adoption**: start with a single compile-time predicate, expand coverage gradually
- **Minimal runtime overhead**: zero overhead in CompileTime mode; only necessary `Bool` checks in
  Runtime mode

### Disadvantages

- **Compile time**: SMT solving increases compile time, but the hard budget limit keeps the upper
  bound controllable
- **Automatic proof boundary**: complex predicates beyond first-order linear arithmetic may require
  the programmer to write a proof function. This is not a language defect—it is the inevitable
  conclusion of the halting problem. The compiler honestly reports `Unproven` rather than falsely
  reporting `True`/`False`
- **Learning curve**: writing effective compile-time predicates and proof functions requires
  understanding the basic intuition of the Curry–Howard correspondence
- **Implementation complexity**: unifying the compile-time proof pipeline requires careful design

### Risk Mitigation

- SMT solver budget hard limit (steps 10,000 / time 100ms / instantiation depth 3); exceeding the
  budget returns `Unproven`
- Dependent type pre-reduction: deterministic value computation is consumed first; SMT only chews on
  the non-deterministic part
- `Unproven` is not a dead end: the programmer can write a proof function; the type checker verifies
  it—exactly as it verifies any function's return type
- Incremental verification: only changed modules are verified
- Clear error messages + counterexample display + budget consumption report + unsolved proposition +
  suggestions (when the compiler can provide them)

## Alternatives

| Alternative                                                        | Why not chosen                                                                                                                                                          |
| ------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| RFC-022: `//!` comment-style specifications                        | Splits specifications from types, violating the Curry–Howard correspondence                                                                                             |
| Standalone specification files (e.g., CVL)                         | Specifications separated from code increases maintenance cost                                                                                                           |
| Runtime-only assertions                                            | Cannot statically guarantee correctness                                                                                                                                 |
| External proof assistants (e.g., Coq)                              | Disjoint from the compiler, requires a separate proof language and trust boundary. YaoXiang's choice: proofs are YaoXiang code; the type checker is the sole trust root |
| **This proposal: compile-time predicates as first-class citizens** | ✅                                                                                                                                                                      |

## Implementation Strategy

### Phased Plan

| Phase       | Content                                                                                                                                                                |
| ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Phase 1** | Compiler kernel: structural equivalence + βδι-reduction + universal quantifier introduction/elimination. Support simple arithmetic predicates (`x > 0`, `arr.len > 0`) |
| **Phase 2** | SMT-LIB translation layer + Z3/CVC5 integration. Pipeline returns Proved/Disproved/Unproven. `Unproven` supports programmer-supplied proof functions                   |
| **Phase 3** | Loop invariant VC generation + termination checking (linear rank function + predicate violation counting + bounded patterns + combinatorial explosion control)         |
| **Phase 4** | Incremental verification + caching + IDE support                                                                                                                       |

### Dependencies

- RFC-010: Unified Type Syntax — compile-time predicates are based on `name: type = value`
- RFC-011: Generics System — compile-time predicates can carry generic parameters
- RFC-009: Ownership Model — compile-time predicate expressions obey ownership rules

## Open Questions

- [x] **SMT solver choice**: default Z3 (MIT license, most extensively validated). CVC5 as
      SMT-LIB-compatible alternative, switched via compiler flag. The compiler's internal
      translation target is SMT-LIB 2.6 standard format—SMT-LIB _is_ the abstraction layer; no
      custom generic-solver interface.
- [x] **Specific solver budget values**: steps 10,000 / time 100ms / quantifier instantiation
      depth 3. Fixed internally in the compiler, no knob. If real use cases prove insufficient (not
      "the user wrote it wrong"), adjust later.
- [x] **Quantifier support scope**: the language imposes no quantifier-order limit. Compile-time
      predicates accept Type parameters—Type includes function types—so higher-order quantifiers are
      a natural corollary of the type system, requiring no special syntax. SMT solvers can
      automatically decide first-order quantifiers (forall/exists, with interleaved nesting, bounded
      by budget depth 3). Higher-order quantifiers: SMT returns `Unproven`; the compiler prompts
      "this predicate exceeds the automatic proof range; please provide a proof function". The
      programmer writes a YaoXiang function whose return type equals the proposition—the type
      checker verifies the function. No external export, no AI, no interactive proof mode.
      Everything is YaoXiang code; everything is verified by the type checker.
- [x] **Counterexample formatting**: source variable names are used directly as SMT variable names
      (with module prefix to avoid conflicts). When Z3's model is returned, look up by variable
      name. Output format: variable name = concrete value + source location + predicate definition
      location. No complex mapping layer.
- [x] ~~**Interaction between compile-time predicates and `ref` smart pointers?**~~ → Decided:
      compile-time predicates only allow immutable borrows or values with ownership transferred.
      Values with mutable borrows cannot appear in compile-time predicates.
- [x] **Extending the `forall` predicate violation-count measure to non-adjacent operations?** → No
      extension. Current coverage (adjacent swap, adjacent move) is complementarily covered by
      Strategy 1 (linear rank function)—quicksort's outer interval contraction is backed by Strategy
      1, heapsort by Strategy 1 (array-index pattern). Loops that cannot be proven terminating by
      any strategy are directly errored by the compiler—this is the hard-safety philosophy, not a
      defect. If future real scenarios (non-academic constructions) have algorithms that all four
      strategies cannot cover, we will revisit.
- [x] **Combinatorial explosion in linear rank function enumeration**: candidate enumeration is
      capped at 3 bounded variables. ≤3: enumerate all linear combinations and verify each via
      SMT. >3: try only single-variable measures (`v_i`, `u_i - v_i`); failure triggers a direct
      compile error—prompting the programmer "the loop has >3 bounded variables; the compiler cannot
      auto-synthesize a multi-variable measure". This is not an engineering compromise—it forces
      programmers to write simpler loops.

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
│  Reviewing  │  ← Current status: community discussion
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
│ (formal design) │  (remains in place) │
└─────────────┘    └─────────────┘
```
