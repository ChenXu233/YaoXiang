---
title: 'RFC-027: Compile-Time Predicates and Unified Static Verification'
status: 'Accepted'
author: 'Chen Xu'
created: '2026-06-07'
updated: '2026-09-14'
impl_status: 'in-progress'
impl_detail:
  'Phase 1-2 complete, Phase 3 partially complete, Phase 4 partially complete. All 6 phases of the
  unified assert/Assert plan implemented (issues #157-#162 closed): Never type, IsTrue bridging,
  flow-sensitive Γ + kill set, type-level recursion, universe stratification weak check, and the
  dispatch pipeline.'
impl_percent: 85
issue_number: 90
issue_url: 'https://github.com/ChenXu233/YaoXiang/issues/90'

issue: '#90'
---

# RFC-027: Compile-Time Predicates and Unified Static Verification

> **References**:
>
> - [RFC-009: Ownership Model](../accepted/009-ownership-model.md)
> - [RFC-010: Unified Type Syntax — `name: type = value` Model](../accepted/010-unified-type-syntax.md)
> - [RFC-011: Generic Type System Design](../accepted/011-generic-type-system.md)
> - [RFC-024: Concurrency Model Based on `spawn` Blocks](../accepted/024-concurrency-model.md)
>
> **Supersedes**:
> [RFC-022: Hoare-Logic Static Verification Support (Specification Comments and Specification Types)](../deprecated/022-hoare-logic-static-verification.md)
> — deprecated

## Summary

This RFC proposes introducing **compile-time predicates** as first-class citizens in YaoXiang,
unifying all compile-time static verification into a single **proof pipeline**. A compile-time
predicate is not an external specification comment — it is a function. A function that returns
`Type` can be used at type positions, and the compiler invokes it at compile time and checks the
return value. Types are propositions; compile-time evaluation is proof.

**Core argument**: The only thing type checking does at compile time is construct and verify proof
terms. Type equality, token conflicts, dependent-type reduction, compile-time predicate evaluation,
Hoare-logic implication — all are different type checks within a single compile-time proof pipeline,
sharing the same pipeline. The SMT solver is an accelerator module for the type checker, not a
separate trust boundary. When the compiler returns `Unproven`, the programmer writes a YaoXiang
function as proof — the type checker verifies it exactly the same way it verifies any function's
return type. Everything is YaoXiang code, and everything is verified by the type checker.

## Motivation

### Why deprecate RFC-022?

RFC-022 designed specifications as `//!` comments:

```yaoxiang
max: (T: Ord) -> ((arr: Array(T, n)) -> T) = {
    //! requires: NonEmpty(n) = n > 0          ← This is a comment independent of types
    //! ensures: ExistsMax(result, arr[0..n])   ← This is a comment independent of types
}
```

This makes a fundamental error against the Curry-Howard correspondence: **splitting specifications
and types into two layers**. Comments are not types. Comments do not participate in type checking.
Comments fit the mental model of "external tools."

The white paper says it clearly:

> "There are no `//!` comments. There is no separate specification language. Everything is within
> the type system."

### Current Problems

- RFC-022's `//!` comments are external syntax independent of the type system
- Specification types and regular types are two different systems, causing conceptual redundancy
- The Debug Build verifies / Release Build ignores split model breaks unity
- The SMT solver is conventionally positioned as an external tool — YaoXiang instead builds it in as
  an accelerator module of the type checker
- Type checking, borrow checking, compile-time predicate checking, and macro expansion all take
  different paths

### The Correct Mental Model

Type checking can be abstracted as a function:

```
verify : Program → Proved | Disproved(Model) | Unproven
```

All compile-time checks — simple type matching, borrow-conflict detection, compile-time predicate
verification — are subtasks of this function. They share the same proof pipeline; the only
difference is the complexity of proof terms and the strategy used to construct them.

When the compiler returns `Unproven`, the programmer provides a proof function — whose return type
equals the proposition to be proven. The type checker verifies it. This is the same operation as
ordinary type checking.

## Proposal

### 1. `{}` Is the Proof Space: Types Are Assertions, Verification Is Type Checking

YaoXiang's `{}` is the compile-time proof space. Everything inside is an assertion, and the compiler
guarantees every item is `True` — either proven automatically or supplied with a proof function by
the programmer.

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
#          parameter in signature   only assertions inside {}
#          compiler verifies x > 0 when called at compile time

List: (T: Type) -> Type = { data: Array(T) }
#      ^^^^^^^^              ^^^^^^^^^^^^^^^
#      parameter in signature   compiler verifies type_of(T) == Type, type_of(data) == Array(T)
```

The same pattern: `name: (params) -> Type = { assertions }`. The compiler does not distinguish "type
assertions" from "value assertions" — both are evaluation targets within the proof pipeline.

**Loop invariants need not be written separately. Type annotations on variables are the Floyd-Hoare
invariants.**

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # Annotation references i — tells the compiler that s's type depends on i
    mut i: UpTo(arr.len) = 0     # At initialization i=0, verify: 0 == sum(arr[0..0]) → True
    while i < arr.len {
        s += arr[i]  # Compiler verifies: s_new == sum(arr[0..i+1])
        i += 1       # i changes → triggers s dependency re-verification: s satisfies SumUpTo(arr, i_new)
    }
    return s  # s: SumUpTo(arr, arr.len) = sum(arr[0..arr.len])
}
```

The compiler generates one verification condition for the loop body — inductive hypothesis (the type
annotation) → assignment operation → whether the new value satisfies the type annotation. After the
proof pipeline verifies the inductive step, all iterations are covered automatically. No
`: decreases`, no `: Invariant`, no inductive proof needed — the compiler decomposes induction into
a local VC for each assignment.

### 2. Pre/Postconditions: Compile-Time Predicates on Parameter Types and Return Types

Discard RFC-022's `//! requires`/`//! ensures`. Compile-time predicates are type annotations on
parameters or return values.

**On the parameter side it is a function call.** A compile-time predicate is a function that returns
`Type`; its use on the parameter side is exactly calling it — same as `factorial(5)`. On the return
side, a new concept is introduced: the return-value parameter.

```yaoxiang
# Precondition: explicitly invoke compile-time predicate in the parameter type
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b
#                       ^^^^^^^^^^  b is the current formal parameter name, passed to Positive
#                       The compiler extracts the actual argument at the call site, substitutes into b, and verifies Positive(arg)
#                       e.g. divide(10, 2) → verify Positive(2) = { 2 > 0 } → True
#                       e.g. divide(10, 0) → verify Positive(0) = { 0 > 0 } → False → compile error

# Postcondition: return-value parameter + compile-time predicate
IsMax: (T: Ord, arr: Array(T), result: T) -> Type = {
    forall j in 0..arr.len: result >= arr[j]
}

NonEmpty: (arr: Array(T)) -> Type = { arr.len > 0 }

max: (T: Ord) -> ((arr: NonEmpty(arr))) -> (result: IsMax(T, arr, result)) = {
#                                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
#                                            result is the return-value parameter, value provided by return
#                                            The compiler substitutes the return value at the return point, verifies the postcondition
    candidate = arr[0]
    for i in 1..arr.len {
        if arr[i] > candidate { candidate = arr[i] }
    }
    return candidate
}
```

**Key rules**:

- **Parameter side**: `b: Positive(b)` — `b` is the current formal parameter name, passed to
  `Positive`. Function-call syntax, zero implicit machinery.
- **Return side**: `-> (result: IsMax(T, arr, result))` — `result` is the return-value parameter;
  its value is provided by the `return` statement. `result` exists only in the type signature and is
  referenced only by predicates; it does not enter the function-body scope and does not appear at
  the call site.
- **Return-value parameter is optional**: when there is no postcondition, omit it and the signature
  looks exactly like a regular function (`-> Int`).
- **Unity**: parameter and return-value parameters are the same concept —
  `formal-name: predicate(formal-name)` — the only difference is whether the value is provided by
  the caller or by `return`.

### 3. Path-Condition Propagation: Compile-Time Verification of Runtime Values

When a compile-time predicate is used at a binding position, the arguments are explicitly supplied
by the programmer. When a runtime value enters a refined-type argument, the compiler completes
verification via path-condition collection and SMT implication — no need for the programmer to
supply the proof explicitly.

#### 3.1 Explicit Function Calls

When a compile-time predicate is used at a binding position, the arguments are explicitly supplied
by the programmer — it's a function call, zero implicit machinery.

`Positive: (x: Int) -> Type = { x > 0 }` is a compile-time predicate constructor. When it appears in
a binding position (parameter declaration, variable declaration, return type), the programmer
explicitly passes the already-bound variable name:

```yaoxiang
b: Positive(b)
// b is already declared as the current formal parameter; Positive(b) is a function call
// After normalization: b: { b > 0 }
```

The compiler does not implicitly fill in arguments — `b: Positive(b)` is just a function call, like
`f(5)`. The name `b` is bound as a parameter name, and its type annotation `Positive(b)` references
`b` itself — this is the standard pattern of dependent types, not an implicit expansion rule.

**Unification with RFC-010's `self`**: RFC-010 establishes that `self` is not a keyword, just a
conventional parameter name ("writing `p`, `this`, or `x` works exactly the same"). `b: Positive(b)`
shares the same mechanism — the parameter name can be referenced in the type annotation. `self`
appears in the position `self: Point`, `b` appears in the position `b: Positive(b)`; both type
annotations reference the parameter itself. The difference lies only in the complexity of the type
annotation; the mechanism is identical — once a name is bound, the type can depend on that name.

The return type uses explicit function calls in the same way:

```yaoxiang
Sorted: (arr: Array(T)) -> Type = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }

sort: (arr: Array(T)) -> (result: Sorted(result)) = { ... }
//                        ^^^^^^^^^^^^^^^^^^^^^^^
//                        result is the return-value parameter; Sorted(result) is a function call
//                        The compiler substitutes the return value into result at the return point, verifies Sorted(return value)
```

The same applies to local variable declarations:

```yaoxiang
let x: Positive(x) = 5
// x is bound to 5, Positive(5) → { 5 > 0 } → True → pass

// let y: Positive(y) = 0
// y is bound to 0, Positive(0) → { 0 > 0 } → False → compile error
```

#### 3.2 Path-Condition Collection

When a runtime value appears in a conditional branch, the compiler automatically collects path
conditions, forming the **assumption set** for the current scope. These assumptions participate in
verification as background knowledge for compile-time `Bool` evaluation.

```yaoxiang
if y > 0 {
    // The compiler automatically obtains the assumption in this branch: { y > 0 }
    let result = divide(x, y)
    // Verification condition: (y > 0) ⇒ (y > 0)
    // The proof pipeline judges the implication to hold → Proved
} else {
// This branch assumes: { !(y > 0) }
// If you want to call divide(x, y), the verification condition is !(y > 0) ⇒ y > 0
    // The proof pipeline judges the implication as not holding → Disproved
}
```

This is not a hard-coded special pattern in the compiler — it is the natural behavior of the
compile-time proof pipeline. Each type-check call site sends the following to the pipeline:

```
{background assumptions} ⇒ {verification goal}
```

The proof pipeline judges implication. `Proved` → pass, `Disproved` → compile error +
counterexample, `Unproven` → compile error + unsolved proposition. Background assumptions come from
the path conditions of the current program point.

#### 3.3 Assumption Stack

As the compiler analyzes control flow, it maintains an assumption set for each basic block:

- **if-guard**: `if y > 0` → true branch pushes `y > 0`, false branch pushes `!(y > 0)` (if `else`
  is used)
- **match patterns**: `if let Some(v) = opt` → inside the branch, push `opt == Some(v)`
- **Logical conjunction**: `if x > 0 and y < 10` → inside the branch, push `x > 0` and `y < 10`
- **Function preconditions**: when calling `divide(a, b)`, `b` must satisfy `Positive`; the evidence
  either comes from current assumptions or from the actual argument's own refined type annotation
  (if `b` is annotated as `Positive`, its type carries `b > 0`)
- **Assignment**: when `let z = y`, the refined conditions already on `y` propagate to `z`

All assumptions enter the compile-time proof pipeline. When the SMT-accelerated path is taken, they
are translated into SMT-LIB background assertions.

#### 3.4 No Static Evidence → Compile Error

If the programmer writes directly:

```yaoxiang
divide_user_input: (x: Int, y: Int) -> Int = divide(x, y)
```

At the current program point there is no `y > 0` assumption, and the actual argument `y` itself has
no `Positive` annotation. The verification condition is:

```
{} ⇒ { y > 0 }
```

The pipeline returns `Disproved` (implication does not hold) → compile error:

> Cannot prove that parameter `b` satisfies `Positive` in the call to `divide`. `y` comes from
> function input with no proven bound. Consider guarding the call with an `if` branch:
> `if y > 0 { divide(x, y) }`.

YaoXiang does not accept runtime values entering refined-type arguments without supplying static
evidence. This is not a restriction — it is the core of the hard-safety philosophy. Code the
compiler cannot prove statically must not pass compilation.

#### 3.5 Relationship with the Unified Pipeline

Path-condition propagation is not an extra mechanism. It is a direct extension of the compile-time
proof pipeline over control-flow analysis:

| Stage                             | Responsibility                                                                                                                                            |
| --------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Path-condition collection         | The compiler's control-flow analysis stage annotates each basic block with an assumption set                                                              |
| Verification-condition generation | When a type constraint requiring verification is encountered, merge path conditions + actual-argument type info                                           |
| Proof-pipeline evaluation         | Compiler kernel → SMT acceleration → derive `Proved` / `Disproved` / `Unproven`                                                                           |
| Result                            | `Proved` → pass; `Disproved` → compile error + counterexample; `Unproven` → compile error + unsolved proposition (programmer can supply a proof function) |

No new component. No special rule. Path conditions are the background knowledge of the proof
pipeline — sharing the same pipeline and the same budget system as type equality and borrow
constraints.

### 4. Compile-Time Proof Pipeline

All compile-time checks share the same pipeline. The core operation of the pipeline is **type
checking** — checking whether a proof term's type equals the proposition to be proven. Everything is
type checking.

```
The compiler encounters a Bool expression requiring evaluation at compile time (i.e. needs to construct a proof term)
        │
        ├── Type equality (T1 == T2)
        │   → The compiler judges directly (structural equivalence)
        │
        ├── Token conflict condition (!conflicting(tokens))
        │   → Flow-sensitive liveness analysis (Dup/Linear property tracking)
        │
        ├── Dependent-type reduction (n + m simplification)
        │   → Compile-time term rewriting system (βδι-reduction)
        │
        ├── Compile-time predicate (x > 0, forall...)
        │   → The compiler itself + SMT acceleration module
        │
        └── Hoare-logic implication (P ⇒ Q)
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
         The programmer writes a proof function (YaoXiang code)
                  │
                  ▼
         The type checker verifies ──→ Proved ──→ Compilation passes
                  │
                  ▼
            Verification fails → Compile error: "Proof does not hold"
```

#### 4.1 Three-Valued Algebra of Proof Results

Compile-time evaluation returns three results — this is a necessary consequence of the halting
problem and a natural division of proof theory:

```
eval_compile_time : BoolExpr → Proved | Disproved(Model) | Unproven
```

- **Proved** → terminates, the proof term is constructed, type checking passes. Compilation
  continues.
- **Disproved(M)** → terminates, a counterexample M exists. Compile error + counterexample + source
  location.
- **Unproven** → within the given resource upper bound, no proof was constructed. Compile error +
  unsolved proposition + budget-consumption report.

**Unproven ≠ False.** The compiler saying "I cannot prove it" is not equivalent to the proposition
being false — it only means it exceeds the current automatic-proof capability. This is honesty, not
a defect.

The hard budget upper bound is the engineering solution to the halting problem. There is no knob —
giving a knob would be asking the user "do you think your program will halt," and neither the user
nor the compiler knows the answer.

#### 4.2 After `Unproven`: The Programmer Writes the Proof

When the compiler returns `Unproven`, the programmer can write a **proof function** — a YaoXiang
function whose return type equals the proposition to be proven. The type checker verifies this
function — exactly the same mechanism as verifying that `add(a, b): Int`.

```
Proposition = Type
Proof       = Program (a value of that type)
Verification = Type check (the sole root of trust)
```

The SMT solver is not a separate trust boundary — it is an **accelerator module of the type
checker**. SMT helps find proofs, but it is always the type checker that verifies the proofs. When
SMT returns `unsat`, the compiler reconstructs its result as a proof term verifiable by the type
checker. If the reconstruction fails (the SMT reasoning steps exceed the compiler kernel's inference
rules), it falls back to `Unproven` — the programmer can write a proof function manually.

```yaoxiang
# Proposition: a refined attribute the compiler cannot prove automatically
FirstIsMin: (T: Ord, arr: Sorted(T)) -> Type = {
    forall i in 0..arr.len: arr[0] <= arr[i]
}

# Proof: the programmer writes a function whose return type is the proposition above
# The type checker verifies this function — exactly the same as verifying add(a,b): Int
first_is_min: (T: Ord, arr: Sorted(T)) -> FirstIsMin(T, arr) = {
    # The compiler verifies here: the function body's type = FirstIsMin(T, arr)
    ...
}
```

No AI needed, no export to Coq, no new concept. **An attribute that cannot be proven automatically
at compile time → the programmer writes the proof in YaoXiang code → the type checker verifies it.**
The whole process is a smooth gradient — the compiler does the easy proofs for you, leaving the hard
ones for your brain.

#### 4.3 Layered Dependencies Within the Pipeline

The evaluators above share the same interface but have an evaluation order. Type equality is the
prerequisite for all subsequent analyses; ownership/token checks depend on type information;
refinement-predicate verification depends on the results of the first two layers. The compiler
evaluates layer by layer; expressions failing at lower layers do not enter upper layers — avoiding
wasted solving budget on type-incorrect programs.

```
Evaluation order (same pipeline, layered dispatch)
├── Layer 0: Type equality (T1 == T2)
│   └── Structural unification → if it fails, subsequent steps are meaningless; return Disproved directly
├── Layer 1: Ownership/token conflicts
│   └── Flow-sensitive liveness analysis → if it fails, memory safety does not hold; return Disproved directly
└── Layer 2: Refinement predicate / Hoare implication
    └── Compiler itself → SMT acceleration → derive Proved / Disproved / Unproven
```

Each layer still returns `Proved/Disproved/Unproven`, sharing the same interface and the same budget
system.

### 5. Unification of Three Function Layers

| Layer                  | Evaluation time | Input      | Output | Example                                        |
| ---------------------- | --------------- | ---------- | ------ | ---------------------------------------------- |
| Value-level function   | Runtime         | Value      | Value  | `add: (a: Int, b: Int) -> Int = a + b`         |
| Type constructor       | Compile time    | Type/Value | Type   | `List: (T: Type) -> Type = { data: Array(T) }` |
| Compile-time predicate | Compile time    | Value      | Type   | `Positive: (x: Int) -> Type = { x > 0 }`       |

All use the same `name: type = value` syntax. Compile-time predicates and type constructors go
through the same compile-time proof pipeline — `{}` is the proof space.

### 6. Loops: Floyd-Hoare Verification-Condition Generation

Loops do not need separate `: Invariant(...)` or `: decreases(...)` annotations. Compile-time
predicate type annotations on variables define Floyd-Hoare-style assertions — the compiler generates
verification conditions from the type annotations, and the proof pipeline checks whether each
assignment preserves the type.

Core mechanism: each assignment operation corresponds to a Hoare triple `{P} x := e {Q}`, with
verification condition `P ⇒ Q[e/x]`. The compiler generates one verification condition for the loop
body — once the proof pipeline verifies the inductive step, all iterations are covered
automatically.

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # Annotation references i; at initialization i=0, verify: 0 == sum(arr[0..0]) → True
    mut i: UpTo(arr.len) = 0     # Verify: 0 <= 0 <= arr.len → True
    while i < arr.len {
        # The compiler generates one VC for the loop body. Pre: s satisfies SumUpTo(arr, i), i satisfies UpTo(arr.len).
        #
        # s += arr[i]:
        #   Verification obligation: s_new satisfies SumUpTo(arr, i) (current i unchanged)
        #   Substituting s_new = s_old + arr[i]:
        #     Need s_old + arr[i] == sum(arr[0..i+1])
        #     From inductive hypothesis s_old == sum(arr[0..i]), add arr[i] to both sides:
        #     sum(arr[0..i]) + arr[i] == sum(arr[0..i+1])
        #   Compiler + SMT: linear arithmetic, millisecond-level → Proved
        #
        # i += 1:
        #   i changes → in the dependency graph, s's type annotation references i → triggers re-verification
        #   New verification goal: s satisfies SumUpTo(arr, i_new)
        #   i.e. s == sum(arr[0..i_new]), guaranteed by the previous step → Proved
        s += arr[i]
        i += 1
    }
    return s  # At this point s: SumUpTo(arr, arr.len), i.e. s == sum(arr[0..arr.len])
}
```

Loop invariants are simply the type annotations on variables — the programmer writes the type, the
compiler checks the inductive step. The compiler does not need to "discover" invariants, nor does it
need to "automatically perform induction" — it decomposes the inductive proof into local
verification conditions for each assignment operation and dispatches them to the proof pipeline,
divide and conquer.

#### 6.1 Dependency Tracking: Dependent Types on Mutable Variables

The prerequisite for the above mechanism is that the compiler knows `s`'s type annotation
`SumUpTo(arr, i)` references `i` — when `i` changes, `s`'s type constraint also changes. This
requires the compiler to maintain a **type dependency graph between variables**.

**Data structure**:

```
TypeDepGraph: Map<VarName, Set<VarName>>
# Key is the depended-on variable, value is the set of variables whose type annotation references that variable
# Example: { i: {s}, j: {s, t}, ... }
```

**Construction**: when the type checker processes `mut v: Pred(... x ...) = init`, it parses the
free-variable references in the `Pred(...)` arguments. If a referenced variable `x` is another
mutable variable in the current scope, it records `x → v` in the dependency graph.

**Trigger**: when the depended-on variable `x` is assigned, the compiler:

1. Looks up all variables in the dependency graph that depend on `x`, namely `{v₁, v₂, ...}`
2. For each `v`, generates a verification condition: does `v`'s current value satisfy the updated
   type `Pred(... x_new ...)`?
3. Sends the VC into the proof pipeline

**Assignment-order sensitivity**: dependency tracking naturally enforces the correct assignment
order. Taking `SumUpTo(arr, i)` as an example:

```yaoxiang
# Correct order
s += arr[i]   # s_new satisfies SumUpTo(arr, i+1)
i += 1        # i changes → re-verify s satisfies SumUpTo(arr, i_new) → True

# Wrong order — the compiler rejects it
i += 1        # i changes → re-verify s satisfies SumUpTo(arr, i_new)
              # s has not yet been updated; s_old == sum(arr[0..i_old]) ≠ sum(arr[0..i_new])
              # → Compile error: variable s does not satisfy type SumUpTo(arr, i_new)
s += arr[i]   # unreachable
```

**Combined dependencies**: a variable can depend on multiple variables. The type annotation
`{ v: Int; v == x + y }` depends on both `x` and `y` — a change in either triggers re-verification.

**Relationship with the proof pipeline**: dependency tracking is the trigger for VC generation, not
an independent verification mechanism. It answers "when do we need to generate VCs" — the proof
pipeline answers "does the VC hold."

### 7. Termination Checking

**Scope of application: refined types.** Termination is not an independent switch, but part of the
**verification mode**: once a type is refined (`Refined { base, constraint }`), the computation
annotated by it enters verification mode, which requires termination within the mode; ordinary types
not refined do not enter verification mode and generate no termination obligation.

From this, two corollaries follow:

- **Loops**: a bare `while` does not enter verification mode; when a measure variable carries a
  refined annotation (e.g. `i: UpTo(n)`), it enters verification mode and termination must be
  proven.
- **Recursion**: when a function signature is refined (parameter refined or return type contains
  refinement), it enters verification mode and each recursive call site must be shown to have a
  strictly decreasing measure.

Fully automatic priority within the mode: the compiler first automatically explores measures; if it
can prove termination, pass; if it cannot explore one and the programmer has not given an explicit
measure, compile error. There is **no annotation-syntax escape hatch** — measures and termination
propositions are written at the type position, introducing no new `decreases`-like syntax. For the
forms of measures and explicit fallback, see §6.9.

#### 6.1 Design Principles

The compiler automatically extracts the information needed for termination proofs from two sources:

1. **Variable type annotations**: boundary constraints in refined types (e.g. `UpTo(n)` gives an
   upper bound `n` and a lower bound `0`)
2. **Loop-body operations**: the operation the variable undergoes on each iteration

The compiler tries four measure-synthesis strategies in priority order; it stops at the first
success. The four strategies are a **bounded template sequence for measure exploration**, taking
refined constraints (all four strategies start from "a variable with a bounded type") as input, not
"code evaluated at compile time"; they are the automatic and manual sides of the same thing as the
explicit measure in §6.9.

> **Measure exploration is exploration, not inference.** Exploration only enumerates templates
> (linear rank, violation count, bounded pattern, multiplicative scaling) and does not guarantee a
> solution exists — general measure inference is undecidable (reduces to the halting problem).
> Therefore, cases outside the templates must be given an explicit measure by the programmer (§6.9),
> otherwise compile error.

#### 6.2 Strategy 1: Automatic Linear Rank-Function Synthesis

When a variable has a linear bound annotation, the compiler enumerates candidate linear measures and
verifies them with SMT.

```
Input:
  Variables v₁: UpTo(u₁), v₂: UpTo(u₂), ... (variables with upper and lower bounds)
  Loop condition cond
  Set of assignments in the loop body

Algorithm:
  1. Extract each variable's bounds from the type annotation: [low_i, high_i]
  2. Enumerate candidate measures: v_i, u_i - v_i, v_i - v_j, and other linear combinations
  3. For each candidate measure m:
     - SMT verify m ≥ 0 (derived from the type bounds)
     - For each execution path of the loop body, SMT verify m' < m (strictly decreasing)
  4. Find a linear combination that satisfies the conditions → termination proven
```

Coverage: any loop where a variable is assigned a linear expression (`v = a·v + b`) with a bounded
type annotation. This includes `i += const`, `i -= const`, and binary-search-style interval
shrinking:

```yaoxiang
# Binary search: low = mid + 1 or high = mid
# The measure high - low strictly decreases on both paths
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

#### 6.3 Strategy 2: Predicate-Violation Counting — Automatically Extracting Measures from Target Types <span style="color:orange">【Experimental Strategy】</span>

> ⚠️ **Current status: experimental strategy; the Phase 3 implementation will decide whether to
> include it based on practical feasibility.** This strategy is effective for adjacent-swap
> operations (bubble sort, insertion sort) and cannot automatically prove non-adjacent operations
> (quicksort partition, heapsort sift-down). See the table below for coverage boundaries. If Phase 3
> verification proves infeasible, this strategy will be removed or downgraded to future work.

Core insight: **the specification written by the user is material for the compiler to reason
about.** The compiler does not need to hard-code "what sorting is" — it reads the definition of
`Sorted` and automatically extracts a measure from it.

```
Input:
  Target type: Sorted(arr) = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }
  Loop-body operation: adjacent-element swap

Algorithm:
  1. Parse the predicate definition: forall i in range: cond(i, arr)
  2. Automatically generate the measure: violation_count = |{ i | ¬cond(i, arr) }|
  3. Analyze the operation's effect on the measure:
     - Adjacent swap arr[j], arr[j+1] = arr[j+1], arr[j]
     - Only affects the three pairs at indices j-1, j, j+1
     - If arr[j] > arr[j+1] (predicate violated), after the swap this pair satisfies the predicate
     - violation_count decreases by at least 1
  4. Upper bound: n·(n-1)/2 (max number of adjacent inversions), lower bound: 0
  → Termination proven
```

**Current coverage**:

| Algorithm      | Operation pattern | Strategy 2 provable? | Reason                                                      |
| -------------- | ----------------- | :------------------: | ----------------------------------------------------------- |
| Bubble sort    | Adjacent swap     |          ✅          | violation_count strictly decreases on each swap             |
| Insertion sort | Adjacent move     |          ✅          | Each shift eliminates one violated pair                     |
| Selection sort | Non-adjacent swap |          ❌          | A single swap may increase violation_count                  |
| Quicksort      | partition         |          ❌          | Non-adjacent swap; not guaranteed to decrease monotonically |
| Heapsort       | sift-down         |          ❌          | Tree-shaped operations; violation_count is non-monotonic    |

**Complementary strategies**: for quicksort, the `low < high` interval shrink can be covered by
Strategy 1 (linear rank function) — the outer partition recursion halves the interval each time.
Strategies 1 and 2 complement each other; for most practical algorithms, termination is provable by
at least one. However, generalizing Strategy 2 (non-adjacent operations, tree operations) remains an
open problem.

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

`v += const` (positive constant) with the variable having an upper-bound type annotation → the
measure `upper_bound - v` decreases by `const` each time, with a lower bound of 0. This is a
degenerate case of Strategy 1, which the compiler handles first.

#### 6.5 Strategy 4: Multiplicative-Scaling Measure Template

`v *= const` (const > 1) with the variable having upper- and lower-bound type annotations. The
compiler has a built-in logarithmic measure template `ceil(log_const(upper/v))`, with the measure
decreasing by 1 on each multiplication by `const`.

```yaoxiang
mut i: Positive(i) = 1
while i < n {
    # The compiler automatically derives: measure ceil(log₂(n/i)), decreasing by 1 on each multiplication by 2
    i *= 2
}
```

#### 6.6 Separation of Termination and Correctness

Termination proofs and correctness proofs are independent:

- **Termination**: the four strategies above automatically prove that the loop exits in finite
  steps; when exploration fails, the programmer gives the measure at the type position (§6.9)
- **Correctness**: whether the loop body advances toward the target type, checked by the
  compile-time proof pipeline via verification conditions

Both pass → compilation passes. Termination proven but correctness fails → compile error +
counterexample. Correctness proven but termination cannot be proven → compile error and points out
the variable or operation that could not be analyzed. Both fail → compile error reports both
failures separately.

#### 6.7 Termination Checking for Recursive Functions

For recursive functions with refined signatures, the compiler checks parameter strict decrease at
every recursive call site:

```yaoxiang
gcd: (a: Int, b: NonNegative(b)) -> Int = {
    if b == 0 { return a }
    return gcd(b, a % b)  // The compiler explores: well-order measure on (b, a % b) strictly decreases → terminates
}
```

Parameter strict decrease is the strongest automatic path (structural recursion). When exploration
fails, the programmer gives the measure explicitly at the type position (§6.9).

#### 6.8 Hard Boundary

`i = f(i)` where `f` is non-invertible, non-closed, and preserves no monotonicity — mathematically
impossible to prove termination automatically. Compile error:

> This loop cannot be proven to terminate automatically. The loop variable depends on a
> non-analyzable function `f`. Please use an iteration pattern that can be analyzed by the compiler,
> or give this loop a name and supply a measure at the type position (§6.9).

This is not a compiler failure. Code that cannot be proven safe statically must not pass
compilation. Even when an explicit measure is given, the measure must be judged true by SMT to pass
— a wrongly written measure is rejected by counterexample; humans can only fail to prove, never
prove wrong.

#### 6.9 Explicit Measure: `Terminates`

When automatic exploration cannot find a measure, the programmer writes the measure at the **type
position** — using the same mechanism as `Positive(b)` and `IsMax(T, arr, result)` (predicate
application), with zero new syntax:

```yaoxiang
// Measure: an ordinary function, unit-testable, reusable, does not participate in runtime
gcd_measure: (a: Int, b: Int) -> Int = { b }

// The termination component lives at the function's own type position
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// Loops: the binding name is the anchor; the measure acquires variables from the scope by name
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n {
        i = i + 1
        i
    }
    return acc
}
```

The reason for the loop: `Terminates(m)` refines exactly the value type of the loop body's tail
expression, and the anchor is provided by the binding name `acc` — the loop is therefore nameable,
and there is no longer a "denotational dead end" of "anonymous constructs that cannot be referred
to."

> **The body tail must yield a value**: a loop's value = its body block's value (§2.15), and a
> block's value = its tail expression. If the body tail is an assignment statement (e.g.
> `{ i = i + 1 }`), the block's value is `Void`, and `return acc` is returning `Void` from `-> Int`,
> which is a type mismatch. Hence the body tail in this example explicitly yields `i`. This does not
> change the semantics of the measure — the measure `n - i` is a **state expression** and is
> independent of the loop's value type.

**`Terminates` is a built-in predicate**, on the same level as `Int` and `Never` among the core
primitives. It is the only predicate for which the compiler writes the function body — its assertion
("the measure strictly decreases at every recursive call site / loop back-edge") lives in the
computation structure, which is why the user-written predicate-definition syntax cannot refer to
function bodies or loop bodies. The built-in face converges to this single name.

**Arity**. `Terminates(FnType, m)` and `Terminates(m)` are two arities of the same predicate, not
two different constructs:

| Form                    | Anchor                             | Use                                                             |
| ----------------------- | ---------------------------------- | --------------------------------------------------------------- |
| `Terminates(m)`         | The name of the binding it sits on | Self-recursive functions, loops — the default form              |
| `Terminates(FnType, m)` | Explicit function type             | Scenarios where the anchor is not unique, e.g. mutual recursion |

Both are essentially the same: the termination obligation always falls on "the piece of computation
annotated by the type position where the refinement lives."

**The measure does not restrict the return type.** A measure can be an expression of any type (not
necessarily a natural number); the "strictly decreases" requirement comes from the well-order
available on that type. Whether the measure is well-founded (e.g. when returning `Int`, whether
`>= 0` holds) is an **independent obligation**, equally handled by refinement inference or SMT; if
neither can derive it, the diagnostic does **not** directly reject, but suggests a direction to
check (whether the lower bound holds, whether the recursive parameters really move in that
direction).

**Relationship with automatic exploration**: the explicit measure is not a separate pipeline, but
the input to it after exploration fails. Once given, the measure still goes through the same SMT to
verify strict decrease and well-foundedness; if not, an error is reported with a counterexample.

Obligation generation, the judgment pipeline, diagnostic directions, shared measures for mutual
recursion (SCCs), and other implementation mechanisms are detailed in
[RFC-027a: Explicit Measures for Termination Checking](../review/027a-termination-explicit-measure.md).

### 8. SMT Solver: Accelerator Module of the Type Checker

In traditional languages, the SMT solver is an external tool (e.g. F\* calls Z3, Dafny calls Z3). In
YaoXiang, it is an **accelerator module of the type checker** — invoked only when the compiler
kernel itself cannot directly decide. SMT helps find proofs, but it is the type checker that
verifies them.

**Trust model**: the type checker is the sole root of trust. The SMT solver is an accelerator module
— it helps find proofs, but SMT is not a separate trust boundary. The compiler trusts Z3's `unsat`
result (in line with the F\*/Dafny approach — the probability of Z3 being wrong is lower than that
of the compiler's own bugs, which is a pragmatic engineering choice). The real unreliability control
is in the SMT translation layer — if the translation has a bug, the compiler will expose it in other
tests.

**Interface**: the compiler internally translates to SMT-LIB 2.6 standard format, rather than
binding to a specific solver API. SMT-LIB is an ISO standard, natively supported by Z3, CVC5,
MathSAT, Yices.

**Default backend**: Z3 (MIT license, the most widely documented and community-validated). CVC5 as
an SMT-LIB-compatible alternative — users can switch at compile time via a compiler flag.

No "generic solver abstraction layer" — SMT-LIB is the abstraction layer. If CVC5 makes a
breakthrough in a specific theory in the future, switching requires only swapping the binary, not
modifying the compiler code.

```
Compile-time Bool expression
        │
        ├── The compiler kernel can directly decide (structural equivalence, simple arithmetic,
        │   trivial formulas after constant folding)
        │   → Return Proved / Disproved directly
        │
        └── The compiler kernel cannot directly decide (quantifiers, symbolic variables)
            → Dependent-type pre-reduction (factorial(5) → 120)
            → Translate to SMT-LIB format
            → Send to Z3/CVC5 (with budget limit)
            → Return value: unsat → Proved  │  sat + model → Disproved  │  unknown → Unproven
```

**Solver budget — hard limit, like stack depth**:

| Budget dimension               | Default | Description                                                                                                                                                        |
| ------------------------------ | ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Solving steps                  | 10,000  | Z3 typically finishes linear arithmetic in under a hundred steps. 10,000 steps cover 99% of practical predicates.                                                  |
| Time                           | 100ms   | A single predicate exceeding 100ms = the user is writing a compile-time program, not a type annotation. 100ms × 50 predicates = 5-second compile-time upper bound. |
| Quantifier instantiation depth | 3       | Three nested quantifier layers cover practical patterns. More than three layers is likely a logic exercise.                                                        |

Exceeding the budget returns `Unproven`, compile error + predicate location + consumption. No
degradation, no runtime checks, no silent pass.

**Why this is practical in practice**: in engineering, 95% of practical predicates are linear
arithmetic — `x > 0`, `arr.len > 0`, `0 <= idx < arr.len` — all within decidable fragments, and SMT
solvers return on these in milliseconds. For the few complex predicates that exceed the budget, the
programmer writes a proof function.

Dependent types have a pre-reduction layer before the SMT call: `factorial(5)` directly evaluates at
compile time to `120`, `append([1,2], [3])` directly evaluates to `[1,2,3]`. These deterministic
value computations do not consume SMT budget.

Programmers do not need to know SMT exists. The mental model is: **the compiler proves it if it can,
errors if it cannot — if the compiler does not know how, you can write a function to prove it.**

### 9. Compile-Time Predicate Composition

Compile-time predicates are functions that return `Type`; composition is achieved naturally through
function composition:

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

#### 9.2 Safe Array Access

```yaoxiang
InBounds: (idx: Int, arr: Array(T)) -> Type = { 0 <= idx and idx < arr.len }

get: (arr: Array(T), idx: InBounds(idx, arr)) -> T = arr.data[idx]

arr = Array(Int)(1, 2, 3)
x = get(arr, 1)   # ✅ Compiler verifies InBounds(1, arr) = { 0 <= 1 and 1 < 3 } → True
# y = get(arr, 5)  # ❌ Compiler verifies InBounds(5, arr) = { 0 <= 5 and 5 < 3 } → False
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

### 11. `dispatch` Pipeline: Unified Dispatch for Compile Time and Runtime

`assert` and `Assert` are the two faces of the same refinement-type primitive. The dispatch pipeline
`dispatch` automatically decides whether to go through compile-time proof or runtime check based on
**whether the predicate's free variables are reachable at compile time**:

| Criterion                                                                                    | Mode            | Behavior                                                                                            |
| -------------------------------------------------------------------------------------------- | --------------- | --------------------------------------------------------------------------------------------------- |
| All free variables known at compile time (generic parameters, compile-time constants)        | **CompileTime** | Enter the proof pipeline: `Proved` → erased, `Disproved` → compile error, `Unknown` → require proof |
| Some free variables come from runtime (function parameters, external input, `mut` variables) | **Runtime**     | Insert runtime check, and inject refinement facts into the flow-sensitive assumption set Γ          |

**Key point**: "cannot decide" ≠ "disproved". In `CompileTime` mode, `Unknown` requires proof (no
silent degradation); in `Runtime` mode the proposition has no truth value at compile time at all —
no prover, however strong, can write a tautological proof for "the user might have entered a
negative number"; a runtime check is the only sound choice. This is not the prover being too weak —
it is a theoretical necessity.

### 12. Flow-Sensitive Assumption Set Γ: Strongest-Postcondition Propagation

The compiler maintains a flow-sensitive assumption set Γ that tracks which propositions are known to
hold at each control-flow point.

**SP (strongest postcondition) propagation**:

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

This is a hard requirement of soundness — when the variable's value changes, old assumptions are
invalid.

**Branch merge**: when `IF/ELSE` or match branches merge, Γ is the intersection of each branch's
assumptions. Only propositions that hold on all paths propagate out of the branch.

### 13. Erasure Model Clarification: Witness Erasure ≠ Check Erasure

The assertion in RFC-027 that "refinement types are **completely erased** at runtime" refers to the
**proof witness** — proof terms that have been verified at compile time do not generate runtime
code. However, the **runtime checks** inserted by `dispatch` in `Runtime` mode are retained — they
are `Bool` checks executed at the value layer, not witnesses at the type layer.

Summary: witnesses are erased, checks are retained. The two do not conflict, and the original
assertion of RFC-027 remains unchanged.

## Detailed Design

### Syntax Changes

| Before (RFC-022)                      | After (this RFC)                                                                                                                                       |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `//! requires: NonEmpty(n) = n > 0`   | Compile-time predicate as a parameter type `(b: Positive(b))`                                                                                          |
| `//! ensures: ExistsMax(result, arr)` | Return type uses a return-value parameter `-> (result: IsMax(T, arr, result))`                                                                         |
| `/*! invariant: ... !*/`              | Compile-time predicate type annotation on the variable — Floyd-Hoare invariant                                                                         |
| `//! decreases: n`                    | Measure written at the refined type position (`Terminates`); the compiler first auto-explores, then asks for an explicit one only if exploration fails |
| Specifications are comments           | Specifications are the type system                                                                                                                     |

### Syntax

**No new keyword for compile-time predicates.** `{}` is the proof space, identical to the existing
type-definition syntax. A compile-time predicate is just a function returning `Type` —
`name: (params) -> Type = { assertions }`. Its use is a function call — `Positive(b)`,
`IsMax(T, arr, result)`.

```bnf
# Compile-time predicate = a function returning Type, with the compiler-verified assertions inside {}
# Uses the existing function/type syntax; no new BNF rules required
predicate ::= identifier ':' params '->' 'Type' '=' '{' assertions '}'
```

**Predicate-application arguments must be in compile-time-constant form** — literals, variables
(bound by name), type applications (extracted recursively), or **compile-time nameable function
references** (function names). Argument forms that cannot be converted into a constant expression
report **E1092**, and mismatches between the argument count and the predicate's declared
formal-parameter count report **E1093**. Arguments are bound positionally to the formal-parameter
list; the predicate's arity is determined by the declaration — `Positive(x)` is unary,
`IsMax(T, arr, result)` is ternary, `Terminates(m)` and `Terminates(FnType, m)` are unary and binary
— refinement constraints are **never silently dropped** (previously, non-convertible arguments
caused constraints to vanish silently, and bindings that violated constraints would pass silently).

**New syntactic concept: return-value parameter** — in `-> (name: Type)`, `name` is the return-value
parameter.

The return-value parameter is the **only one syntactic concept** YaoXiang introduces on top of the
existing function syntax. Its semantics:

- The value of `name` is provided by the `return` statement
- `name` exists only in the type signature and is referenced by postcondition predicates (e.g.
  `-> (result: IsMax(T, arr, result))`)
- `name` does not enter the function-body scope and does not appear at the call site
- The return-value parameter is **optional** — when there is no postcondition, the signature is
  exactly like a regular function (`-> Int`), imposing no extra burden

The reason for introducing it: postconditions need to refer to "the value the function is about to
return." Without the return-value parameter, the compiler could only let predicates reference the
return value through special rules (such as implicit variables like `$result` or `__retval__`). The
return-value parameter makes this reference explicit — it is just a formal parameter, only its value
is provided by `return` rather than by the caller.

**Proof functions** are not a new concept — they are simply YaoXiang functions whose return type is
the proposition being asserted. When the compiler returns `Unproven`, the programmer supplies a
proof function, and the type checker verifies it exactly the same way it verifies any function's
return type. No new syntax, no new keyword, no new rule.

> **The boundary between the two.** The correctness domain (predicate `Unproven`) supplies a proof
> **in the body**: write a function whose return type is the proposition to be proven. The
> termination domain (§6.9) supplies a measure **at the type position**: write `Terminates(measure)`
> as the type of a binding or function signature. The former is "write a proof for a proposition you
> cannot prove"; the latter is "explicitly declare a measure the explorer cannot find" — the
> mechanism is the same (both are refined-type applications), only the landing point differs. The
> termination domain **does not** need the programmer to write a `_proof` function.

### Type-System Impact

- **Type universe**: compile-time predicates sit in the Type₂ layer — functions that take values and
  return `Type`, at the same level as type constructors
- **Generic interaction**: compile-time predicates can take generic parameters, e.g.
  `NonEmpty: (T: Type) -> (arr: Array(T)) -> Type`
- **Ownership interaction**: expressions inside compile-time predicates obey the ownership rules;
  they can only read, not write
- **Type inference**: arguments to compile-time predicates participate in HM type inference

### Runtime Representation

Compile-time predicates are handled at runtime **according to the dispatch result**:

- **CompileTime mode** (all free variables known at compile time): once the proof passes, the
  witness is completely erased. `Positive: (x: Int) -> Type = { x > 0 }` — the argument
  `b: Positive(5)` at runtime is just an `Int`. The refinement condition `{ 5 > 0 }` has been
  verified, and is erased.
- **Runtime mode** (some free variables are runtime): retain the runtime check — a `Bool` check
  executed at the value layer, injecting into the flow-sensitive assumption set Γ. See §11 for the
  dispatch pipeline and §13 for the erasure-model clarification.

Putting a compile-time predicate at the type position (e.g. `f(x: Positive(x))`) does not introduce
a wrapper type or extra allocation. But when `x` comes from runtime input, a runtime `Bool` check
**is** inserted.

**Interaction constraint with `ref`**: compile-time predicates can only reference immutably borrowed
values or values whose ownership has been transferred. A compile-time predicate that references a
mutably borrowed value cannot guarantee at compile time that the verification result will still hold
at runtime — such uses directly report a compile error.

### Compiler Changes

1. **Parser**: compile-time predicates use standard function syntax; no additional parsing rules
   needed
2. **Compile-time proof pipeline**: unified `Proved/Disproved/Unproven` return interface with
   automatic strategy selection
3. **SMT acceleration module**: SMT-LIB 2.6 translation layer, default backend Z3, CVC5 as
   alternative
4. **Type-checker kernel**: implementation of inference rules — structural equivalence,
   βδι-reduction, universal-quantifier introduction/elimination. This is the sole root of trust;
   both SMT and programmer-supplied proofs are verified through it
5. **Verification-condition generation**: WP/SP calculus + loop-invariant proof obligations
6. **Error reporting**: counterexample formatting + unsolved-proposition report + source-location
   association

### Backward Compatibility

- ✅ Code that does not use compile-time predicates is completely unchanged
- ✅ Compile-time predicates in `CompileTime` mode have zero runtime overhead; in `Runtime` mode
  only the necessary `Bool` checks are retained
- ⚠️ RFC-022's `//!` syntax is no longer supported — but RFC-022 was never implemented, so there is
  no migration burden

## Trade-offs

### Advantages

- **Full realization of the Curry-Howard correspondence**: types are propositions, programs are
  proofs, `name: Proposition = Proof`
- **Unity**: compile-time predicates and ordinary functions use exactly the same syntax; no
  conceptual split
- **SMT transparency**: programmers do not need to know SMT exists; the mental model matches type
  checking
- **Incremental adoption**: start with a single compile-time predicate and progressively expand
  coverage
- **Minimal runtime overhead**: zero overhead in `CompileTime` mode; in `Runtime` mode only the
  necessary `Bool` checks are retained

### Disadvantages

- **Compile time**: SMT solving increases compile time, but the hard budget upper bound keeps the
  cap controllable
- **Boundary of automatic proof**: predicates more complex than first-order linear arithmetic may
  require the programmer to write a proof function. This is not a language defect — it is the
  necessary consequence of the halting problem. The compiler honestly reports `Unproven` rather than
  falsely reporting `True`/`False`
- **Learning curve**: writing effective compile-time predicates and proof functions requires
  understanding the basic intuition of the Curry-Howard correspondence
- **Implementation complexity**: unifying the compile-time proof pipeline requires careful design

### Risk Mitigation

- Hard SMT-solver budget upper bound (10,000 steps / 100ms / instantiation depth 3); exceeding the
  budget returns `Unproven`
- Dependent-type pre-reduction: deterministic value computations are eaten first; SMT only chews on
  the non-deterministic parts
- `Unproven` is not a dead end: in the correctness domain, write a proof function (return type is
  the proposition); in the termination domain, give a measure at the type position (§6.9) — both are
  verified by the type checker
- Incremental verification: verify only changed modules
- Clear error messages + counterexample display + budget-consumption report + unsolved proposition +
  suggestion (if the compiler can give one)

## Alternatives

| Approach                                                      | Why not chosen                                                                                                                                                             |
| ------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| RFC-022: `//!` comment-style specifications                   | Specifications and types are split, violating the Curry-Howard correspondence                                                                                              |
| Separate specification files (e.g. CVL)                       | Specifications separated from code increases maintenance cost                                                                                                              |
| Runtime-only assertions                                       | Cannot guarantee correctness statically                                                                                                                                    |
| External proof assistant (e.g. Coq)                           | Decoupled from the compiler, requiring a separate proof language and trust boundary. YaoXiang's choice: proof is YaoXiang code, the type checker is the sole root of trust |
| **This RFC: compile-time predicates as first-class citizens** | ✅                                                                                                                                                                         |

## Implementation Strategy

### Phase Breakdown

| Phase       | Content                                                                                                                                                                |
| ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Phase 1** | Compiler kernel: structural equivalence + βδι-reduction + universal-quantifier introduction/elimination. Support simple arithmetic predicates (`x > 0`, `arr.len > 0`) |
| **Phase 2** | SMT-LIB translation layer + Z3/CVC5 integration. Pipeline returns `Proved/Disproved/Unproven`. When `Unproven`, the programmer can write a proof function              |
| **Phase 3** | Loop-invariant VC generation + termination checking (four measure-exploration strategies + explicit `Terminates` measure, §6, §6.9)                                    |
| **Phase 4** | Incremental verification + caching + IDE support                                                                                                                       |

### Dependencies

- RFC-010: Unified Type Syntax — compile-time predicates are based on `name: type = value`
- RFC-011: Generic Type System — compile-time predicates can take generic parameters
- RFC-009: Ownership Model — expressions inside compile-time predicates obey the ownership rules

## Open Questions

- [x] **Availability of Z3 for the wasm32 target**: Z3 is not available under wasm (all SMT code is
      gated with `cfg(not(target_arch = "wasm32"))`). Scope of impact and degradation direction: -
      `ownership.rs` `smt_cut` always returns `false` → back-edge traversal → **conservative
      rejection** (sound direction, just narrows less) - Termination-check Strategy 1 and predicate
      level 2b/3 are entirely skipped

  **Correction (2026-10-01)**: The original text claimed these paths were "currently
  **unreachable**" (reasoning: the `predicate_defs` production is never populated, the `parser` does
  not produce `MonoType::Refined`, and the `with_solver` production is never called), but this is no
  longer true — all three are in place (predicate definitions registered in #377-3; refined-type
  normalization; the production pipeline injects the solver). So the difference between wasm and
  native is no longer limited to the precision of `smt_cut`:

  | Path                                        | native   | wasm       | Degradation direction    |
  | ------------------------------------------- | -------- | ---------- | ------------------------ |
  | Termination-check explicit measure judgment | Decided  | `Unjudged` | No E4022 (under-reports) |
  | Well-foundedness judgment                   | Decided  | `Unjudged` | No E4022 (under-reports) |
  | SMT implication level of refined predicates | Executed | Skipped    | Falls into `Unproven`    |

  All are **conservative** directions (less narrowing, less reporting), and do not affect soundness.
  To achieve the same precision, the solution and cost are discussed in issue #376 (conclusion:
  prefer a Z3 instance on the JS side, rather than linking Z3 into the main wasm — the latter
  requires switching the emcc build system and the artifact grows from 3 MB to ~20 MB).

- [x] **SMT solver choice**: default Z3 (MIT license, most widely validated). CVC5 as an
      SMT-LIB-compatible alternative, switched via a compiler flag. The compiler's internal
      translation target is SMT-LIB 2.6 standard format — SMT-LIB is the abstraction layer; no
      custom generic solver interface.
- [x] **Specific values of solver budget**: 10,000 steps / 100ms / quantifier instantiation depth 3.
      Hard-coded in the compiler, no knob. If real use cases prove them insufficient (not "the user
      wrote it wrong"), they can be adjusted later.
- [x] **Quantifier support range**: the language itself does not restrict quantifier order.
      Compile-time predicates accept `Type` arguments — `Type` includes function types — so
      higher-order quantifiers are a natural consequence of the type system, requiring no special
      syntax. The SMT solver can automatically decide first-order quantifiers (forall/exists,
      supporting interleaved nesting, bounded by the budget depth of 3). Higher-order quantifiers:
      SMT returns `Unproven`; the compiler hints "this predicate is beyond the automatic-proof
      range, please provide a proof function." The programmer writes a YaoXiang function whose
      return type equals the proposition — the type checker verifies the function. No external
      export, no AI, no interactive proof mode. Everything is YaoXiang code; everything is verified
      by the type checker.
- [x] **Counterexample formatting**: source variable names are used directly as SMT variable names
      (with a module prefix to avoid collisions). When a Z3 model is returned, lookup is by variable
      name. Output format: `variable name = concrete value` + source location + predicate-definition
      location. No complex mapping layer.
- [x] ~~**Interaction of compile-time predicates with the `ref` smart pointer?**~~ → Decided:
      compile-time predicates allow only immutably borrowed values or values whose ownership has
      been transferred. Values under mutable borrow cannot appear in a compile-time predicate.
- [x] **Extension of the `forall` predicate violation-count measure to non-adjacent operations?** →
      Not extended. Current coverage (adjacent swap, adjacent move) is complemented by Strategy 1
      (linear rank function) — quicksort's outer interval shrink is covered by Strategy 1, heapsort
      is covered by Strategy 1 (array index pattern). For loops that no strategy can prove
      termination, the compiler directly errors out — this is the hard-safety philosophy, not a
      defect. If in the future there is a real-world (non-academic) algorithm that none of the four
      strategies can cover, the discussion can be reopened. → **Rediscussion triggered (2026-09-14,
      #318)**: non-structural recursion (gcd-style non-directly-decreasing recursion, mutual
      recursion, merge partition) is exactly the real-world scenario this clause anticipated — the
      template sequence of measure exploration cannot automatically handle them, and they cannot be
      rewritten into analyzable iteration patterns without sacrificing readability. Conclusion: the
      hard-safety philosophy stands (a wrongly written measure is still rejected by SMT
      counterexample; humans can only fail to prove, never prove wrong), but "exploration fails →
      reject" is relaxed to "exploration fails → the programmer can give an explicit measure at the
      type position" — see §6.9.
- [x] **Combinatorial explosion when enumerating linear rank functions**: the upper bound for
      candidate enumeration is 3 bounded variables. When ≤ 3, enumerate all linear combinations and
      verify each with SMT. When > 3, only try single-variable measures (`v_i`, `u_i - v_i`); on
      failure, report a compile error directly — prompting the programmer "the loop has > 3 bounded
      variables; the compiler cannot automatically synthesize a multi-variable measure." This is not
      an engineering compromise — it forces programmers to write simpler loops.

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

## Lifecycle and Destination

```
┌─────────────┐
│   Draft     │  ← Author creates
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  In Review  │  ← Current state: community discussion
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
│ (formal     │    │ (kept in    │
│  design)    │    │  place)     │
└─────────────┘    └─────────────┘
```
