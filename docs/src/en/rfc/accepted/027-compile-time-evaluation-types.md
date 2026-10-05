---
title: 'RFC-027: Compile-Time Predicates and Unified Static Verification'
status: 'Accepted'
author: 'Chen Xu'
created: '2026-06-07'
updated: '2026-10-03'
impl_status: 'in-progress'
impl_detail:
  'Phase 1-2 complete, Phase 3 partially complete, Phase 4 partially complete. All 6 phases of the
  unified assert/Assert plan implemented (#157-#162 closed): Never type, IsTrue bridging,
  flow-sensitive Γ + kill set, type-level recursion, universe tiering weak check. The **semantics**
  of dispatch in §11 is already implemented via call-site obligations (three direct calls to
  `check_predicate` in `checker.rs`: binding position / call-site arguments / return position), but
  the **standalone `layers/dispatch.rs` module has been deleted** — it had zero production call
  sites, and its handling of `Unproven` (downgrading to W1080 warning) conflicted with the
  production implementation (always erroring out) (#377-2).'
impl_percent: 82
issue_number: 90
issue_url: 'https://github.com/ChenXu233/YaoXiang/issues/90'

issue: '#90'
---

# RFC-027: Compile-Time Predicates and Unified Static Verification

> **References**:
>
> - [RFC-009: Ownership Model](009-ownership-model.md)
> - [RFC-010: Unified Type Syntax — name: type = value Model](010-unified-type-syntax.md)
> - [RFC-011: Generic Type System Design](011-generic-type-system.md)
> - [RFC-024: Concurrency Model Based on spawn Blocks](024-concurrency-model.md)
>
> **Supersedes**:
> [RFC-022: Hoare Logic Static Verification Support (Specification Comments and Specification Types)](../deprecated/022-hoare-logic-static-verification.md)
> — Deprecated

## Abstract

This RFC proposes introducing **compile-time predicates** as first-class citizens to YaoXiang,
unifying all compile-time static verification into a single **proof pipeline**. A compile-time
predicate is not an external specification comment — it _is_ a function. A function that returns a
`Type` can be used at type position; the compiler calls it at compile time and checks the return
value. Types are propositions, compile-time evaluation is proof.

**Core thesis**: The only work of type checking at compile time is constructing and verifying proof
terms. Type equality, token conflicts, dependent-type reduction, compile-time predicate evaluation,
Hoare-logic implications — all are different type checks in the compile-time proof pipeline, sharing
the same pipeline. The SMT solver is an acceleration module of the type checker, not a separate
trust boundary. When the compiler returns `Unproven`, the programmer writes a YaoXiang function as
proof — the type checker verifies it in exactly the same way it verifies any function's return type.
Everything is YaoXiang code, everything is verified by the type checker.

## Motivation

### Why deprecate RFC-022?

RFC-022 designed specifications as `//!` comments:

```yaoxiang
max: (T: Ord) -> ((arr: Array(T, n)) -> T) = {
    //! requires: NonEmpty(n) = n > 0          ← This is a comment separate from the type
    //! ensures: ExistsMax(result, arr[0..n])   ← This is a comment separate from the type
}
```

This commits a fundamental error against the Curry-Howard correspondence: **splitting specifications
and types into two layers**. Comments are not types. Comments do not participate in type checking.
The comment is a mental model of an "external tool".

The white paper makes it clear:

> "There are no `//!` comments. There is no separate specification language. Everything is inside
> the type system."

### Current Problems

- RFC-022's `//!` comments are external syntax separate from the type system
- Specification types and ordinary types are two systems, causing conceptual redundancy
- The split pattern of "Debug Build verifies / Release Build ignores" breaks unity
- In the traditional view, the SMT solver is positioned as an external tool — YaoXiang builds it
  into the type checker as an acceleration module
- Type checking, borrow checking, compile-time predicate checking, and macro expansion each take
  different paths

### The Correct Mental Model

Type checking can be abstracted as a function:

```
verify : Program → Proved | Disproved(Model) | Unproven
```

All compile-time checks — simple type matching, borrow-conflict detection, compile-time predicate
verification — are subtasks of this function. They share the same proof pipeline; they differ only
in proof-term complexity and construction strategy.

When the compiler returns `Unproven`, the programmer provides a proof function — the return type of
which equals the proposition to be proved. The type checker verifies it. This is the same operation
as ordinary type checking.

## Proposal

### 1. `{}` Is the Proof Space: Types Are Assertions, Verification Is Type Checking

YaoXiang's `{}` is the compile-time proof space. Everything inside is an assertion; the compiler
guarantees each item to be `True` — either proved automatically, or by a proof function provided by
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
#          Parameter at signature    Only assertions inside {}
#          Compiler verifies x > 0 at the compile-time call

List: (T: Type) -> Type = { data: Array(T) }
#      ^^^^^^^^              ^^^^^^^^^^^^^^^
#      Parameter at signature   Compiler verifies type_of(T) == Type, type_of(data) == Array(T)
```

The same pattern: `name: (params) -> Type = { assertions }`. The compiler does not distinguish "type
assertion" from "value assertion" — both are evaluation targets in the proof pipeline.

**Loop invariants do not need to be written separately. The type annotation on a variable _is_ the
Floyd-Hoare invariant.**

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # Annotation references i — tells the compiler s's type depends on i
    mut i: UpTo(arr.len) = 0     # At initialization i=0, verify: 0 == sum(arr[0..0]) → True
    while i < arr.len {
        s += arr[i]  # Compiler verifies: s_new == sum(arr[0..i+1])
        i += 1       # i changes → triggers s dependency re-verification: s satisfies SumUpTo(arr, i_new)
    }
    return s  # s: SumUpTo(arr, arr.len) = sum(arr[0..arr.len])
}
```

The compiler generates one verification condition for the loop body — inductive hypothesis (type
annotation) → assignment operation → whether the new value satisfies the type annotation. After the
proof pipeline verifies the inductive step, all iterations are covered automatically. No need for
`: decreases`, no need for `: Invariant`, no need for inductive proofs — the compiler decomposes
induction into a local VC for each assignment.

### 2. Pre-/Post-conditions: Compile-Time Predicates on Parameter and Return Types

Abandon RFC-022's `//! requires`/`//! ensures`. Compile-time predicates act as type annotations on
parameters or return values.

**The parameter side is a function call.** A compile-time predicate is a function returning `Type`;
using it on the parameter side is calling it — just like `factorial(5)`. The return-value side
introduces a new concept: the return-value formal parameter.

```yaoxiang
# Pre-condition: explicit call to compile-time predicate in parameter type
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b
#                       ^^^^^^^^^^  b is the current formal parameter name, passed to Positive
#                       The compiler extracts the actual argument value at the call site, substitutes b, and verifies Positive(actual)
#                       Example: divide(10, 2) → verify Positive(2) = { 2 > 0 } → True
#                       Example: divide(10, 0) → verify Positive(0) = { 0 > 0 } → False → compile error

# Post-condition: return-value formal parameter + compile-time predicate
IsMax: (T: Ord, arr: Array(T), result: T) -> Type = {
    forall j in 0..arr.len: result >= arr[j]
}

NonEmpty: (arr: Array(T)) -> Type = { arr.len > 0 }

max: (T: Ord) -> ((arr: NonEmpty(arr))) -> (result: IsMax(T, arr, result)) = {
#                                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
#                                            result is the return-value formal parameter; value provided by return
#                                            The compiler substitutes the return value at the return point and verifies the post-condition
    candidate = arr[0]
    for i in 1..arr.len {
        if arr[i] > candidate { candidate = arr[i] }
    }
    return candidate
}
```

**Key rules**:

- **Parameter side**: `b: Positive(b)` — `b` is the current formal parameter name, passed to
  `Positive`. Function-call syntax, zero implicit magic.
- **Return side**: `-> (result: IsMax(T, arr, result))` — `result` is the return-value formal
  parameter; value provided by the `return` statement. `result` only exists inside the type
  signature, is only referenced by the predicate, does not enter the function body scope, and does
  not appear at the caller.
- **Return-value formal parameter is optional**: omit it when there is no post-condition; the
  signature is identical to an ordinary function (`-> Int`).
- **Unity**: parameter and return-value formal parameters are the same concept —
  `formal_name: predicate_call(args)`, differing only in whether the value comes from the caller or
  from `return`.

> **Implementation note (2026-10-02)**: Identification of the return-position formal parameter is
> **strictly by its declared name** — the `r` in `-> (r: P(...))` is the unique return-value formal
> parameter. Free variables in the constraint that are **not in scope** are no longer substituted
> with the return value; instead an undefined-identifier error is reported (verified:
> `g: () -> (r: Eq2(m, r)) = { return 7 }` where `m` is undefined →
> `error [E1001] Unknown variable: 'm'`). The arguments of a predicate call are substituted into the
> constraint as **arguments**: `P(r + 100)` after substitution becomes "return value + 100", not
> "return value".
>
> **Known gap (2026-10-02)**: **Return-position multi-argument predicates + symbolic arguments are
> not yet supported**. Triggering form: `f: (b: Int) -> (r: SumUpTo(b, r)) = { return b * 2 }`
> (where `SumUpTo: (n: Int, s: Int) -> Type = { s == n * 2 }`) →
> `error [E2031] … the refinement type constraint of 'r': SumUpTo(b, (b * 2)) cannot be statically proven`.
> Workaround: change arguments to compile-time-foldable literals (e.g. `SumUpTo(3, r)`), or provide
> a proof function returning `Type` for that predicate; unblocking condition: the proof kernel
> supports argument substitution for "multi-argument predicate + symbolic arguments". See
> [RFC-027a](../review/027a-termination-explicit-measure.md) §Implementation Landing Record
> (2026-10-02, this round of hardening).

### 3. Path-Condition Propagation: Compile-Time Verification of Runtime Values

When a compile-time predicate is used at a binding position, the arguments are passed explicitly by
the programmer. When a runtime value enters a refinement-type argument, the compiler completes
verification via path-condition collection and SMT implication — no explicit proof pass required
from the programmer.

#### 3.1 Explicit Function Call

When a compile-time predicate is used at a binding position, arguments are passed explicitly by the
programmer — it's just a function call, with zero implicit magic.

`Positive: (x: Int) -> Type = { x > 0 }` is a compile-time predicate constructor. When it appears at
a binding position (parameter declaration, variable declaration, return type), the programmer passes
the already-bound variable name explicitly:

```yaoxiang
b: Positive(b)
// b has been declared as the current formal parameter; Positive(b) is a function call
// After normalization: b: { b > 0 }
```

No implicit filling by the compiler — `b: Positive(b)` is just like `f(5)`, it's a function call.
`b` is bound as a parameter name; its type annotation `Positive(b)` references `b` itself — this is
the standard pattern of dependent types, not an implicit expansion rule.

**Unification with RFC-010's `self`**: RFC-010 establishes that `self` is not a keyword, just a
conventional name for a parameter ("writing it as `p`, `this`, or `x` has exactly the same effect").
`b: Positive(b)` shares the same mechanism — a parameter name can be referenced in its type
annotation. `self` appears in the position `self: Point`; `b` appears in the position
`b: Positive(b)`; both type annotations reference the parameter itself. The difference is only the
complexity of the type annotation; the mechanism is identical — once a name is bound, the type can
depend on that name.

The return type also uses explicit function call:

```yaoxiang
Sorted: (arr: Array(T)) -> Type = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }

sort: (arr: Array(T)) -> (result: Sorted(result)) = { ... }
//                        ^^^^^^^^^^^^^^^^^^^^^^^
//                        result is the return-value formal parameter; Sorted(result) is a function call
//                        The compiler substitutes the return value into result at the return point and verifies Sorted(return value)
```

The same applies to local variable declarations:

```yaoxiang
let x: Positive(x) = 5
// x is bound to 5; Positive(5) → { 5 > 0 } → True → pass

// let y: Positive(y) = 0
// y is bound to 0; Positive(0) → { 0 > 0 } → False → compile error
```

#### 3.2 Path-Condition Collection

When a runtime value appears in a conditional branch, the compiler automatically collects path
conditions to form the current scope's **assumption set**. These assumptions participate in
verification as background knowledge for compile-time `Bool` evaluation.

```yaoxiang
if y > 0 {
    // The compiler automatically acquires the assumption here: { y > 0 }
    let result = divide(x, y)
    // Verification condition: (y > 0) ⇒ (y > 0)
    // The proof pipeline decides the implication holds → Proved
} else {
// This branch's assumption: { !(y > 0) }
// If we want to call divide(x, y), the verification condition is !(y > 0) ⇒ y > 0
    // The proof pipeline decides the implication does not hold → Disproved
}
```

This is not a hard-coded special pattern in the compiler — this is the natural behavior of the
compile-time proof pipeline. At every type-checking call site, the pipeline is sent:

```
{background assumptions} ⇒ {verification target}
```

The proof pipeline decides implication. `Proved` → pass; `Disproved` → compile error +
counterexample; `Unproven` → compile error + unsolved proposition. Background assumptions come from
the path condition at the current program point.

#### 3.3 Assumption Stack

When analyzing control flow, the compiler maintains an assumption set for each basic block:

- **if-guard**: `if y > 0` → push `y > 0` for the true branch, push `!(y > 0)` for the false branch
  (if `else` is used)
- **match pattern**: `if let Some(v) = opt` → push `opt == Some(v)` inside the branch
- **Logical conjunction**: `if x > 0 and y < 10` → push `x > 0` and `y < 10` inside the branch
- **Function pre-condition**: when calling `divide(a, b)`, the evidence that `b` satisfies
  `Positive` must come either from the current assumption, or from the refinement-type annotation on
  the argument itself (if `b` is annotated as `Positive`, its type carries `b > 0`)
- **Assignment**: on `let z = y`, the refinement conditions already on `y` are propagated to `z`

All assumptions enter the compile-time proof pipeline. When taking the SMT-acceleration path, they
are translated into SMT-LIB background assertions.

#### 3.4 No Static Evidence Means Compile Error

If the programmer directly writes:

```yaoxiang
divide_user_input: (x: Int, y: Int) -> Int = divide(x, y)
```

There is no `y > 0` assumption at the current program point, and the argument `y` itself has no
`Positive` type annotation. The verification condition is:

```
{} ⇒ { y > 0 }
```

The pipeline returns `Disproved` (not implied) → compile error:

> Unable to prove that argument `b` satisfies `Positive` in the call to `divide`. `y` comes from
> function input, with no proven bound. Consider guarding the call with an if-branch:
> `if y > 0 { divide(x, y) }`.

YaoXiang does not accept runtime values entering refinement-type arguments without static evidence.
This is not a restriction — this is the core of the hard-safety philosophy. Whatever the compiler
cannot statically prove, may not pass compilation.

#### 3.5 Relation to the Unified Pipeline

Path-condition propagation is not an additional mechanism. It is the direct extension of the
compile-time proof pipeline into control-flow analysis:

| Phase                      | Responsibility                                                                                                                                           |
| -------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Path-condition collection  | Compiler control-flow analysis phase; tag each basic block with its assumption set                                                                       |
| Verification-condition gen | When a type constraint to be verified is encountered, merge path conditions + argument type info                                                         |
| Proof-pipeline evaluation  | Compiler kernel → SMT acceleration → produce `Proved` / `Disproved` / `Unproven`                                                                         |
| Result                     | `Proved` → pass; `Disproved` → compile error + counterexample; `Unproven` → compile error + unsolved proposition (programmer can provide proof function) |

No new components. No special rules. Path conditions are the background knowledge of the proof
pipeline — sharing the same pipeline and the same budget system as type equality and borrow
constraints.

### 4. The Compile-Time Proof Pipeline

All compile-time checks share the same pipeline. The core operation of the pipeline is **type
checking** — checking that the type of a proof term equals the proposition to be proved. Everything
is type checking.

```
Compile-time encounters a Bool expression to evaluate (i.e., to construct a proof term)
        │
        ├── Type equality (T1 == T2)
        │   → Compiler decides directly (structural equivalence)
        │
        ├── Token-conflict conditions (!conflicting(tokens))
        │   → Flow-sensitive liveness analysis (Dup/Linear property tracking)
        │
        ├── Dependent-type reduction (n + m simplification)
        │   → Compile-time term-rewriting system (βδι-reduction)
        │
        ├── Compile-time predicates (x > 0, forall...)
        │   → Compiler itself + SMT acceleration module
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
         Programmer writes proof function (YaoXiang code)
                  │
                  ▼
         Type checker verifies ──→ Proved ──→ Compilation passes
                  │
                  ▼
            Verification fails → Compile error: "proof does not hold"
```

#### 4.1 Proof Results: A Three-Valued Algebra

Compile-time evaluation returns three results — this is the inevitable conclusion of the halting
problem, and the natural partition of proof theory:

```
eval_compile_time : BoolExpr → Proved | Disproved(Model) | Unproven
```

- **Proved** → Halt, proof term constructed, type check passes. Compilation continues.
- **Disproved(M)** → Halt, counterexample M exists. Compile error + counterexample + source
  location.
- **Unproven** → Within the given resource bound, no proof is constructed. Compile error + unsolved
  proposition + budget consumption report.

**Unproven ≠ False.** When the compiler says "I cannot prove it", that is not equivalent to the
proposition being false — it just exceeds the current auto-proving capability. This is honesty, not
a defect.

Hard budget limits are the engineering solution to the halting problem. No knob is provided —
providing one would be asking the user "do you think your program will halt", which neither the user
nor the compiler knows.

#### 4.2 After Unproven: The Programmer Writes the Proof

When the compiler returns `Unproven`, the programmer can write a **proof function** — a YaoXiang
function whose return type equals the proposition to be proved. The type checker verifies this
function — exactly the same mechanism as verifying that `add(a, b): Int`.

```
Proposition = Type
Proof       = Program (a value of that type)
Verification= Type check (the sole trust root)
```

The SMT solver is not a separate trust boundary — it is an **acceleration module of the type
checker**. SMT helps find proofs, but it is the type checker that verifies them. When SMT returns
`unsat` , the compiler reconstructs that result as a proof term verifiable by the type checker. If
reconstruction fails (the SMT reasoning step exceeds the compiler kernel's inference rules), it
falls back to `Unproven` — the programmer can then manually write a proof function.

```yaoxiang
# Proposition: a refinement attribute the compiler cannot auto-prove
FirstIsMin: (T: Ord, arr: Sorted(T)) -> Type = {
    forall i in 0..arr.len: arr[0] <= arr[i]
}

# Proof: programmer writes a function whose return type is the proposition above
# The type checker verifies this function — exactly the same as verifying add(a, b): Int
first_is_min: (T: Ord, arr: Sorted(T)) -> FirstIsMin(T, arr) = {
    # The compiler verifies here: function body's type = FirstIsMin(T, arr)
    ...
}
```

No AI, no export to Coq, no new concepts. **An attribute that cannot be auto-proved at compile time
→ programmer writes the proof in YaoXiang code → type checker verifies it.** The whole process is a
smooth gradient — the compiler does the easy proofs, leaving the hard ones to the programmer.

#### 4.3 Stratified Dependencies Within the Pipeline

The evaluators above share the same interface but have an evaluation order. Type equality is the
prerequisite for all subsequent analysis; ownership/token checking depends on type information;
refinement-predicate verification depends on the results of the first two layers. The compiler
evaluates layer by layer; expressions that fail in a lower layer do not enter higher layers —
avoiding wasting solver budget on type-erroneous programs.

```
Evaluation order (same pipeline, layered dispatch)
├── Layer 0: Type equality (T1 == T2)
│   └── Structural unification → on failure, downstream is meaningless, return Disproved directly
├── Layer 1: Ownership/token conflicts
│   └── Flow-sensitive liveness analysis → on failure, memory safety does not hold, return Disproved directly
└── Layer 2: Refinement predicate / Hoare implication
    └── Compiler itself → SMT acceleration → produce Proved / Disproved / Unproven
```

Each layer still returns `Proved/Disproved/Unproven`, sharing the same interface and the same budget
system.

### 5. Unification of Three Function Layers

| Layer                  | When run     | Input        | Output | Example                                        |
| ---------------------- | ------------ | ------------ | ------ | ---------------------------------------------- |
| Value-level function   | Runtime      | Values       | Value  | `add: (a: Int, b: Int) -> Int = a + b`         |
| Type constructor       | Compile-time | Types/Values | Type   | `List: (T: Type) -> Type = { data: Array(T) }` |
| Compile-time predicate | Compile-time | Values       | Type   | `Positive: (x: Int) -> Type = { x > 0 }`       |

All use the same `name: type = value` syntax. Compile-time predicates and type constructors take the
same compile-time proof pipeline — `{}` is the proof space.

### 6. Loops: Floyd-Hoare Verification-Condition Generation

Loops do not need a separate `: Invariant(...)` or `: decreases(...)` annotation. The compile-time
predicate type annotation on a variable defines a Floyd-Hoare-style assertion — the compiler
generates verification conditions from the type annotation, and the proof pipeline checks whether
each assignment preserves the type.

Core mechanism: each assignment corresponds to a Hoare triple `{P} x := e {Q}`, with verification
condition `P ⇒ Q[e/x]`. The compiler generates one verification condition for the loop body — after
the proof pipeline verifies the inductive step, all iterations are automatically covered.

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # Annotation references i; at initialization i=0, verify: 0 == sum(arr[0..0]) → True
    mut i: UpTo(arr.len) = 0     # Verify: 0 <= 0 <= arr.len → True
    while i < arr.len {
        # The compiler generates one VC for the loop body. Precondition: s satisfies SumUpTo(arr, i), i satisfies UpTo(arr.len).
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
        #   i changes → dependency graph says s's type annotation references i → triggers re-verification
        #   New verification target: s satisfies SumUpTo(arr, i_new)
        #   i.e. s == sum(arr[0..i_new]), guaranteed by the previous step → Proved
        s += arr[i]
        i += 1
    }
    return s  # At this point s: SumUpTo(arr, arr.len), i.e. s == sum(arr[0..arr.len])
}
```

The loop invariant is the type annotation on the variable — the programmer writes the type, the
compiler checks the inductive step. The compiler does not need to "discover" the invariant, nor to
"do induction automatically" — it decomposes the inductive proof into a local verification condition
for each assignment and lets the proof pipeline divide and conquer.

#### 6.1 Dependency Tracking: Dependent Types on Mutable Variables

The premise of the mechanism above is: the compiler knows that the type annotation `SumUpTo(arr, i)`
on `s` references `i` — when `i` changes, the type constraint on `s` changes too. This requires the
compiler to maintain a **type-dependency graph between variables**.

**Data structure**:

```
TypeDepGraph: Map<VarName, Set<VarName>>
# Key is the depended-on variable; value is the set of variables whose type annotation references that variable
# Example: { i: {s}, j: {s, t}, ... }
```

**Construction**: When the type checker handles `mut v: Pred(... x ...) = init`, it resolves
free-variable references in the `Pred(...)` arguments. If the arguments reference another mutable
variable `x` in the current scope, it records `x → v` in the dependency graph.

**Trigger**: When the depended-on variable `x` is assigned, the compiler:

1. Looks up all variables `{v₁, v₂, ...}` in the dependency graph that depend on `x`
2. For each `v`, generates a verification condition: does `v`'s current value satisfy the updated
   type `Pred(... x_new ...)`?
3. Sends the VC into the proof pipeline

**Assignment-order sensitive**: Dependency tracking naturally enforces the correct assignment order.
Take `SumUpTo(arr, i)` as an example:

```yaoxiang
# Correct order
s += arr[i]   # s_new satisfies SumUpTo(arr, i+1)
i += 1        # i changes → re-verify s satisfies SumUpTo(arr, i_new) → True

# Wrong order — compiler rejects
i += 1        # i changes → re-verify s satisfies SumUpTo(arr, i_new)
              # s has not been updated yet; s_old == sum(arr[0..i_old]) ≠ sum(arr[0..i_new])
              # → Compile error: variable s does not satisfy type SumUpTo(arr, i_new)
s += arr[i]   # unreachable
```

**Composite dependency**: A variable can depend on multiple variables. The type annotation
`{ v: Int; v == x + y }` depends on both `x` and `y` — a change in either triggers re-verification.

**Relation to the proof pipeline**: Dependency tracking is a trigger for VC generation, not an
independent verification mechanism. It answers "when do we need to generate a VC" — the proof
pipeline answers "does the VC hold".

### 7. Termination Checking

**Scope: refinement types.** Termination is not an independent switch, but part of the
**verification mode**: once a type is refined (`Refined { base, constraint }`), the computation
annotated by it enters verification mode, which requires termination; ordinary unrefined types do
not enter verification mode and generate no termination obligations.

Two corollaries:

- **Loops**: A bare `while` does not enter verification mode; a measure variable with a refinement
  annotation (e.g. `i: UpTo(n)`) does, and must prove termination.
- **Recursion**: When a function signature carries a refinement (parameter refinement or a
  refinement in the return type), it enters verification mode, and must prove that the measure
  strictly decreases at every recursive call site.

Fully automatic first inside the mode: the compiler first automatically explores the measure; if
provable, it passes; if neither explorable nor explicitly given, it's a compile error. No
**annotation syntax** escape hatch — both the measure and the termination proposition are written at
the type position; no new `decreases`-style syntax is introduced. The form of the measure and the
explicit fallback is in §6.9.

#### 6.1 Design Principles

The compiler automatically extracts the information needed for a termination proof from two places:

1. **Variable type annotations**: boundary constraints in the refinement type (e.g. `UpTo(n)` gives
   upper bound `n` and lower bound `0`)
2. **Loop-body operations**: the operations applied to the variable on each iteration

The compiler attempts four measure-synthesis strategies in priority order, stopping when one
succeeds. The four strategies are a **bounded template sequence for measure exploration**; the input
is the refinement constraint (all of strategies 1–4 start from a "variable with a bounded type"),
not "code to be compile-time-evaluated"; they are the automatic and manual sides of the same thing,
alongside the explicit measures in §6.9.

> **Measure exploration is exploration, not inference.** Exploration only enumerates templates
> (linear rank, violation count, bounded pattern, multiplicative scaling); it does not guarantee a
> solution exists — general measure inference is undecidable (reduces to the halting problem).
> Therefore, situations outside the templates must be solvable by the programmer giving an explicit
> measure (§6.9), otherwise a compile error.

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
  2. Enumerate candidate measures: v_i, u_i - v_i, v_i - v_j, etc., linear combinations
  3. For each candidate measure m:
     - SMT verifies m ≥ 0 (derived from type bounds)
     - For each path in the loop body, SMT verifies m' < m (strictly decreasing)
  4. Find a linear combination that satisfies the conditions → termination proved
```

Coverage: loops where any variable is assigned a linear expression (`v = a·v + b`) and has a bounded
type annotation. Includes `i += const`, `i -= const`, and binary-search-style interval shrinking:

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

#### 6.3 Strategy 2: Predicate-Violation Count — Auto-Extracting the Measure from the Goal Type <span style="color:orange">【Experimental Strategy】</span>

> ⚠️ **Current status: experimental strategy; whether to include it depends on actual feasibility
> during Phase 3 implementation.** This strategy works for adjacent-swap operations (bubble sort,
> insertion sort); it cannot auto-prove non-adjacent operations (quicksort partition, heapsort
> sift-down). See the coverage boundary table below. If Phase 3 verification is not feasible, this
> strategy will be removed or downgraded to future work.

Core insight: **The user-written specification is material for the compiler's reasoning.** The
compiler does not need to hard-code "what sorting is" — it reads the `Sorted` definition, and
automatically extracts the measure from it.

```
Input:
  Goal type: Sorted(arr) = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }
  Loop-body operations: adjacent-element swaps

Algorithm:
  1. Parse the predicate definition: forall i in range: cond(i, arr)
  2. Automatically generate measure: violation_count = |{ i | ¬cond(i, arr) }|
  3. Analyze the operation's effect on the measure:
     - Adjacent swap arr[j], arr[j+1] = arr[j+1], arr[j]
     - Only affects the three pairs at indices j-1, j, j+1
     - If arr[j] > arr[j+1] (violates the predicate), the pair satisfies the predicate after the swap
     - violation_count decreases by at least 1
  4. Upper bound: n·(n-1)/2 (max adjacent inversions), lower bound: 0
  → Termination proved
```

**Current coverage**:

| Algorithm      | Operation pattern | Strategy 2 provable? | Reason                                                  |
| -------------- | ----------------- | :------------------: | ------------------------------------------------------- |
| Bubble sort    | Adjacent swap     |          ✅          | violation_count strictly decreases each swap            |
| Insertion sort | Adjacent move     |          ✅          | Each shift eliminates one violating pair                |
| Selection sort | Non-adjacent swap |          ❌          | A single swap may increase violation_count              |
| Quicksort      | Partition         |          ❌          | Non-adjacent swap; no guarantee of monotonic decrease   |
| Heapsort       | sift-down         |          ❌          | Tree-shaped operation; violation_count is non-monotonic |

**Complementary strategy**: For quicksort, the `low < high` interval shrinking is covered by
Strategy 1 (linear rank function) — the outer partition recursion halves the interval each time.
Strategies 1 and 2 cover each other complementarily; termination of most real algorithms can be
proved by one of the two. But generalizing Strategy 2 (non-adjacent operations, tree-shaped
operations) remains an open problem.

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

`v += const` (positive constant) with a variable that has an upper-bound type annotation → the
measure `upper_bound - v` decreases by `const` each step, lower bound 0. This is a degenerate case
of Strategy 1; the compiler handles it first as a fast path.

#### 6.5 Strategy 4: Multiplicative-Scaling Measure Template

`v *= const` (const > 1) with a variable that has upper- and lower-bound type annotations. The
compiler has a built-in logarithmic measure template `ceil(log_const(upper/v))`; each multiplication
by `const` decreases the measure by 1.

```yaoxiang
mut i: Positive(i) = 1
while i < n {
    # Compiler auto-derives: measure ceil(log₂(n/i)); each multiplication by 2 decreases the measure by 1
    i *= 2
}
```

#### 6.6 Separation of Termination and Correctness

Termination proof and correctness proof are independent:

- **Termination**: the four strategies above auto-prove that the loop exits in finite steps; when
  none is found, the programmer gives the measure at the type position (§6.9)
- **Correctness**: whether the loop body progresses toward the goal type, checked by the
  compile-time proof pipeline through verification conditions

Both pass → compile passes. Termination proved but correctness fails → compile error +
counterexample. Correctness proved but termination not provable → compile error pointing to the
un-analyzable variable or operation. Both fail → compile error reporting each failure separately.

#### 6.7 Termination Checking for Recursive Functions

For a recursive function with a refinement signature, the compiler checks that the formal parameters
decrease at every recursive call site:

```yaoxiang
gcd: (a: Int, b: NonNegative(b)) -> Int = {
    if b == 0 { return a }
    return gcd(b, a % b)  // Compiler explores: well-founded order of (b, a % b) decreases → termination
}
```

Decrease in formal parameters is the strongest automatic path (structural recursion). When it cannot
be found, the programmer gives the measure explicitly at the type position (§6.9).

#### 6.8 Hard Boundary

`i = f(i)` where `f` is non-invertible, non-closed, and does not preserve any monotonicity —
mathematically, termination cannot be auto-proved. Compile error:

> This loop cannot be auto-proved to terminate. The loop variable depends on the non-analyzable
> function `f`. Please use an iteration pattern that the compiler can analyze, or bind a name to
> this loop and provide a measure at the type position (§6.9).

This is not a compiler failure. Whatever cannot be statically proved safe may not pass compilation.
Even with an explicit measure, it must be judged true by SMT to pass — a wrong measure will be
turned back by a counterexample; humans can fail to prove, but cannot prove wrong.

#### 6.9 Explicit Measure: `Terminates`

When automatic exploration finds no measure, the programmer writes the measure at the **type
position** — the same mechanism as `Positive(b)` and `IsMax(T, arr, result)` (predicate
application), with zero new syntax:

```yaoxiang
// Measure: an ordinary function; unit-testable, reusable, not participating at runtime
gcd_measure: (a: Int, b: Int) -> Int = { b }

// The termination component lives at the function's own type position
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// Loop: the binding name is the anchor; the measure takes scope-local values by name
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n {
        i = i + 1
        i
    }
    return acc
}
```

Reason for the loop form: `Terminates(m)` refines the value type of the loop body's tail expression,
and the anchor is provided by the binding name `acc` — the loop is therefore nameable, with no dead
corner of "anonymous construct, no name to refer to".

> **The body tail must yield a value**: the value of a loop = the value of the loop body block
> (§2.9), and the value of a block = the tail expression. If the body tail is an assignment
> statement (e.g. `{ i = i + 1 }`), the block value is `Void`, and `return acc` would be returning
> `Void` from `-> Int`, a type error. Hence in this example the body tail is written explicitly as
> `i`. This does not change the semantics of the measure — the measure `n - i` is a **state
> expression**, unrelated to the loop's value type.

> **Implementation note (registered 2026-10-02; updated 2026-10-04)**: This section's loop example
> holds on the **type side** (0 errors from `check`, back-edge obligation `Proved`). The **runtime
> value-fetch defect (#409) has been fixed**: a `while` used as a value now takes the value of the
> tail expression of the last iteration's body block, on par with the type-side `block_value_ty` —
> `loop(4) == 4`, `countdown(3) == 30` actually run true (same fix for `for`).
>
> **Known residual (zero iterations)**: when the loop body never executes, there is no "value of the
> last iteration's body", and the loop value still defaults to `Void` — binding it to a non-`Void`
> type position and using it at runtime produces
> `E6007 Runtime error: type mismatch in comparison Eq: Void vs Int(0)` (a loud type mismatch, not a
> silent wrong value). This is a **deliberate static-approximation boundary**: the `block_value_ty`
> criterion yields the type of the body's tail expression, with no ability to decide "whether the
> loop executes at least once" (that would need data-flow / provability analysis); therefore the
> type side does not silently downgrade (downgrading would break the value type on which this
> section's `Terminates` loop binding depends). The contract for this residual is pinned by the
> corpus `tests/yaoxiang/02-type-system/while_zero_iteration_void_err.yx`
> (`// expect: runtime-error E6007`). **This residual has been confirmed as a deliberately retained
> design boundary** (ruled on 2026-10-04, not to be fixed for now), so issue
> [#409](https://github.com/ChenXu233/YaoXiang/issues/409) was closed together with the main-form
> fix, and no separate tracking item is opened. For historical records of the minimal repro and the
> impact surface, see [RFC-027a](../review/027a-termination-explicit-measure.md) §Examples / Loop
> section "Known defect D6". Also: termination Strategy 1 (linear rank function) is restored in this
> round, so the order "write an explicit measure only after automatic exploration fails" in this
> section is unchanged — explicit measure remains the fallback after exploration fails.

**`Terminates` is a built-in predicate**, belonging to the same core primitives as `Int` and
`Never`. It is the only predicate for which the **compiler writes the function body** — its
assertion ("the measure strictly decreases at every recursive call site / loop back edge") is rooted
in the computation structure, and predicates the user writes cannot refer to a function body or loop
body, so it cannot be expressed by the predicate-definition syntax in the "Syntax" section. The
built-in side converges to this single name.

**Arity**. `Terminates(FnType, m)` and `Terminates(m)` are the two arities of the same predicate,
not two constructions:

| Form                    | Anchor          | Use case                                               |
| ----------------------- | --------------- | ------------------------------------------------------ |
| `Terminates(m)`         | The bound name  | Self-recursive functions, loops — the default form     |
| `Terminates(FnType, m)` | Explicit FnType | Mutual recursion, etc., where the anchor is not unique |

Both are the same in essence: the termination obligation always falls on "the computation annotated
by the type position where the refinement lives".

**The measure does not restrict the return type.** The measure can be an expression of any type (not
forced to be a natural number); "strictly decreasing" is given by the well-founded order available
on that type. Whether the measure is well-founded (e.g. when it returns `Int`, whether `>= 0`) is an
**independent obligation**, handed to refinement inference or SMT in the same way as the decrease
obligation; when neither can infer it, the diagnosis does **not reject directly** but instead
suggests a direction to check (whether the measure's lower bound holds, whether the recursive
argument really goes in that direction).

**Relation to automatic exploration**: the explicit measure is not a separate pipeline, but input
after exploration fails. After the measure is given, the same SMT verifies decrease and
well-foundedness; if it does not hold, an error is reported with a counterexample.

For the implementation mechanisms of obligation generation, decision pipeline, diagnostic direction,
and shared measures for mutual recursion (SCC), see
[RFC-027a: Explicit Measures for Termination Checking](../review/027a-termination-explicit-measure.md).

### 8. The SMT Solver: An Acceleration Module of the Type Checker

The SMT solver is an external tool in traditional languages (e.g. F\* calls Z3, Dafny calls Z3). In
YaoXiang, it is an **acceleration module of the type checker** — only invoked when the compiler
kernel itself cannot decide directly. SMT helps find proofs, but the type checker is what verifies
them.

**Trust model**: the type checker is the sole trust root. The SMT solver is an acceleration module —
it helps find proofs, but SMT is not a separate trust boundary. The compiler trusts Z3's `unsat`
result (consistent with the F\*/Dafny approach — the probability of Z3 making an error is lower than
the bug rate of the compiler itself, an engineering pragmatic choice). The real unreliability is
controlled at the SMT-translation layer — if the translation has a bug, the compiler will surface it
in other tests.

**Interface**: the compiler internally translates to the SMT-LIB 2.6 standard format, rather than
binding a specific solver API. SMT-LIB is an ISO standard, natively supported by Z3, CVC5, MathSAT,
and Yices.

**Default backend**: Z3 (MIT license, the most widely documented and community-verified). CVC5 as an
SMT-LIB-compatible alternative — users can switch at compile time via a compiler flag.

No "generic-solver abstraction layer" — SMT-LIB _is_ the abstraction layer. If CVC5 ever breaks
through in a particular theory, switching is just replacing the binary, no compiler code change
required.

```
Compile-time Bool expression
        │
        ├── The compiler kernel can decide directly (structural equivalence, simple arithmetic,
        │   trivial formulas after constant folding)
        │   → Return Proved / Disproved directly
        │
        └── The compiler kernel cannot decide directly (quantifiers, symbolic variables)
            → Dependent-type pre-reduction (factorial(5) → 120)
            → Translate to SMT-LIB format
            → Send to Z3/CVC5 (with a budget limit)
            → Return: unsat → Proved  │  sat + model → Disproved  │  unknown → Unproven
```

**Solver budget — a hard limit, like stack depth**:

| Budget dimension               | Default | Description                                                                                                                                                |
| ------------------------------ | ------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Solver steps                   | 10,000  | Z3 is usually within a hundred steps for linear arithmetic. 10,000 steps cover 99% of real predicates.                                                     |
| Time                           | 100ms   | If a single predicate exceeds 100ms, the user is writing a compile-time program, not a type annotation. 100ms × 50 predicates = 5-second compile-time cap. |
| Quantifier instantiation depth | 3       | Three levels of nested quantifiers cover real patterns. More than three layers and you're probably writing logic exercises.                                |

Over budget returns `Unproven`; compile error + predicate location + consumption. No downgrade, no
runtime check, no silent pass.

**Why this is actually viable**: in practice, 95% of real predicates are linear arithmetic —
`x > 0`, `arr.len > 0`, `0 <= idx < arr.len` — all inside decidable fragments, and SMT solvers
return in milliseconds for such questions. For the rare few complex predicates that exceed the
budget, the programmer can write a proof function.

Dependent types have a pre-reduction layer before the SMT call: `factorial(5)` directly yields `120`
at compile time, `append([1,2], [3])` directly yields `[1,2,3]`. These deterministic value
computations consume no SMT budget.

The programmer does not need to know that SMT exists. The mental model is: **the compiler can prove
it or it errors — if the compiler can't, you can write a function to prove it to it.**

### 9. Compile-Time Predicate Composition

A compile-time predicate is a function returning `Type`; composition is naturally implemented by
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

### 11. The dispatch Pipeline: Unified Compile-Time and Runtime Dispatch

> **Implementation note (2026-10-03, #377-2)**: This section's **semantics** holds and has been
> integrated into production, but the **structure** is not a standalone `layers/dispatch.rs` module.
> Production dispatch is directly implemented by three **call-site obligations** in `checker.rs`
> (direct calls to `layers::predicate::check_predicate`): binding-position re-verification
> (`revalidate_refined`), call-site arguments (`check_call_arg_refinements`), and return-position
> post-condition (`check_return_refinement`). The original `layers/dispatch.rs` had zero production
> call sites, and its handling of `Unproven { ProofFunctionRequired }` (downgrading to a W1080
> warning and injecting Γ) conflicted with the production implementation (`Unproven` always errors;
> see `refined_unproven`), so it was deleted as a whole. Also note: the "insert runtime check" item
> in the Runtime row of this section's table has **no emission point** in production yet (no
> consumer for `RuntimeOutcome::InsertCheck`); Γ injection is independently done by `ownership.rs`
> branch-guard logic and `inference/expressions.rs`.

`assert` and `Assert` are two sides of the same refinement-type primitive. The dispatch pipeline
`dispatch` automatically decides between compile-time proof and runtime check based on **whether the
predicate's free variables are reachable at compile time**:

| Criterion                                                                             | Mode            | Behavior                                                                                        |
| ------------------------------------------------------------------------------------- | --------------- | ----------------------------------------------------------------------------------------------- |
| All free variables are known at compile time (generic params, compile-time constants) | **CompileTime** | Enter the proof pipeline: Proved → erase, Disproved → compile error, Unknown → require proof    |
| Some free variables come from runtime (function args, external inputs, mut variables) | **Runtime**     | Insert a runtime check, and inject the refinement fact into the flow-sensitive assumption set Γ |

**Key**: "can't decide" ≠ "disproved". In `CompileTime` mode, `Unknown` requires a proof (no silent
downgrade); in `Runtime` mode, the proposition is not even true-or-false at compile time — no
prover, however strong, can write a universally-true proof for "the user may have input a negative
number"; a runtime check is the only sound choice. This is not the prover being too weak; it is a
theoretical inevitability.

### 12. Flow-Sensitive Assumption Set Γ: Strongest Post-Condition Propagation

The compiler maintains a flow-sensitive assumption set Γ, tracking the propositions known to hold at
each control-flow point.

**SP (Strongest Post-condition) propagation**:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
```

**Kill set for `mut` variables**: after a `mut` variable is reassigned, all assumptions involving
that variable are removed from Γ:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
mut x = x - 5       // Γ = {}  ← x > 0 is killed
```

This is a hard requirement of soundness — once a variable's value changes, old assumptions are
invalid.

**Branch confluence**: when IF/ELSE or match branches merge, Γ takes the intersection of each
branch's assumptions. Only propositions that hold on all paths are carried out of the branch.

### 13. Erasure Model Clarification: Witness Erasure ≠ Check Erasure

RFC-027's assertion that "refinement types are **fully erased** at runtime" refers to the **proof
witness** — proof terms already verified at compile time produce no runtime code. But the **runtime
check** inserted by `dispatch` in `Runtime` mode is retained — it is a `Bool` check executed at the
value layer, not a type-layer witness.

Summary: witness erased, check retained. The two are not in conflict; the original RFC-027 assertion
stands.

## Detailed Design

### Syntax Changes

| Before (RFC-022)                      | After (this RFC)                                                                                                                                     |
| ------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| `//! requires: NonEmpty(n) = n > 0`   | Compile-time predicate as parameter type `(b: Positive(b))`                                                                                          |
| `//! ensures: ExistsMax(result, arr)` | Return type uses return-value formal parameter `-> (result: IsMax(T, arr, result))`                                                                  |
| `/*! invariant: ... !*/`              | Compile-time predicate type annotation on a variable — a Floyd-Hoare invariant                                                                       |
| `//! decreases: n`                    | Measure written at the refinement type position (`Terminates`); compiler explores automatically first, requires explicit only when exploration fails |
| Specifications as comments            | Specifications as the type system                                                                                                                    |

### Syntax

**No new keyword for compile-time predicates.** `{}` is the proof space, identical to the existing
type-definition syntax. A compile-time predicate is a function returning `Type` —
`name: (params) -> Type = { assertions }`. Use is just a function call — `Positive(b)`,
`IsMax(T, arr, result)`.

```bnf
# Compile-time predicate = a function returning Type, with compiler-verified assertions inside {}
# Reuses the existing function/type syntax; no new BNF rule is needed
predicate ::= identifier ':' params '->' 'Type' '=' '{' assertions '}'
```

**Predicate-application arguments must be in a compile-time-constant form** — literals, variables
(bound by name), type applications (recursively extracted), or **compile-time-nameable function
references** (function names). Arguments whose form cannot be converted into a constant expression
report **E1092**; mismatches between the number of arguments and the predicate's declared formals
report **E1093**. Arguments are bound positionally to the formal-parameter list; predicate arity is
determined by the declaration — `Positive(x)` unary, `IsMax(T, arr, result)` ternary,
`Terminates(m)` and `Terminates(FnType, m)` unary and binary — **refinement constraints are never
silently dropped** (previously, non-convertible arguments would cause constraints to silently
disappear, and bindings that violated the constraint would silently pass).

**New syntax concept: the return-value formal parameter** — in `-> (name: Type)`, `name` is the
return-value formal parameter.

The return-value formal parameter is the **only syntax concept** YaoXiang introduces on top of the
existing function syntax. Its semantics:

- The value of `name` is provided by the `return` statement
- `name` exists only inside the type signature, referenced by the post-condition predicate (e.g.
  `-> (result: IsMax(T, arr, result))`)
- `name` does not enter the function body scope, and does not appear at the caller
- The return-value formal parameter is **optional** — when there is no post-condition, the signature
  is exactly the same as an ordinary function (`-> Int`), and no extra burden is introduced

The reason for introducing it: post-conditions need to refer to "the value the function is about to
return". Without a return-value formal parameter, the compiler could only use a special rule (like
an implicit `$result` or `__retval__` variable) to let the predicate reference the return value. The
return-value formal parameter makes this reference explicit — it is just a formal parameter, except
that the value is provided by `return` rather than the caller.

**Proof function** is not a new concept — it is a YaoXiang function whose return type is the
asserted proposition. When the compiler returns `Unproven`, the programmer provides a proof
function; the type checker verifies it in exactly the same way as verifying any function's return
type. No new syntax, no new keyword, no new rule.

> **The boundary between the two.** For the correctness domain (predicate `Unproven`), supplement
> the proof in the **body**: write a function whose return type is the proposition to be proved. For
> the termination domain (§6.9), supplement the measure at the **type position**: write
> `Terminates(measure)` as the binding's or function signature's type. The former is "write a proof
> for the proposition that can't be auto-proved", the latter is "explicitly declare the measure that
> can't be auto-explored" — same mechanism (refinement-type application), different landing point.
> The termination domain **does not need** to write a `_proof` function.

### Type-System Impact

- **Type universe**: compile-time predicates sit at the Type₂ layer — functions accepting values and
  returning `Type`, the same layer as type constructors
- **Generic interaction**: compile-time predicates can carry generic parameters, e.g.
  `NonEmpty: (T: Type) -> (arr: Array(T)) -> Type`
- **Ownership interaction**: expressions inside compile-time predicates obey the ownership rules —
  read-only, no writes
- **Type inference**: arguments of compile-time predicates participate in HM type inference

### Runtime Representation

A compile-time predicate is handled at runtime **according to the dispatch result**:

- **CompileTime mode** (all free variables are known at compile time): the proof, once passed, has
  its witness **fully erased**. `Positive: (x: Int) -> Type = { x > 0 }` — `b: Positive(5)` at
  runtime is just `Int`. The refinement condition `{ 5 > 0 }` has already passed; erase it.
- **Runtime mode** (some free variables are runtime): retain the runtime check — execute a `Bool`
  check at the value layer, and inject into the flow-sensitive assumption set Γ. See §11 the
  dispatch pipeline and §13 the erasure-model clarification.

Putting a compile-time predicate at a type position (e.g. `f(x: Positive(x))`) produces no wrapper
type, no extra allocation. But when `x` comes from runtime input, a **runtime `Bool` check will be
inserted**.

**Constraint on interaction with `ref`**: compile-time predicates may only reference
immutably-borrowed or ownership-transferred values. A compile-time predicate referencing a
mutably-borrowed value cannot be guaranteed by the compiler to have its verification result still
hold at runtime — such usage directly reports a compile error.

### Compiler Changes

1. **Parser**: compile-time predicates use standard function syntax; no extra parsing rule needed
2. **Compile-time proof pipeline**: unified `Proved/Disproved/Unproven` return interface, automatic
   strategy selection
3. **SMT acceleration module**: SMT-LIB 2.6 translation layer; default backend Z3, CVC5 as
   alternative
4. **Type-checker kernel**: inference-rule implementation — structural equivalence, βδι-reduction,
   forall introduction/elimination. This is the sole trust root; both SMT and programmer proofs are
   validated through it
5. **Verification-condition generation**: WP/SP calculus + loop-invariant proof obligations
6. **Error reporting**: counterexample formatting + unsolved-proposition report + source-location
   linking

### Backward Compatibility

- ✅ Code that does not use compile-time predicates is completely unchanged
- ✅ Compile-time predicates have zero runtime cost in `CompileTime` mode; in `Runtime` mode only
  the necessary `Bool` check is retained
- ⚠️ RFC-022's `//!` syntax is no longer supported — but 022 was never implemented, so there is no
  migration burden

## Trade-offs

### Advantages

- **Curry-Howard correspondence fully realized**: types are propositions, programs are proofs,
  `name: Proposition = Proof`
- **Unity**: compile-time predicates and ordinary functions use exactly the same syntax, with no
  conceptual split
- **SMT transparency**: programmers don't need to know SMT exists; the mental model matches type
  checking
- **Progressive adoption**: start from a single compile-time predicate, expand coverage gradually
- **Minimal runtime cost**: zero cost in `CompileTime` mode; only the necessary `Bool` checks
  retained in `Runtime` mode

### Disadvantages

- **Compile time**: SMT solving increases compile time, but hard budget limits keep the upper bound
  controlled
- **Auto-proving boundary**: complex predicates beyond first-order linear arithmetic may require the
  programmer to write a proof function. This is not a language defect — it is the inevitable
  conclusion of the halting problem. The compiler honestly reports `Unproven` rather than falsely
  reporting `True`/`False`
- **Learning curve**: writing effective compile-time predicates and proof functions requires an
  understanding of the basic intuition of the Curry-Howard correspondence
- **Implementation complexity**: the unification of the compile-time proof pipeline requires careful
  design

### Risk Mitigation

- Hard SMT-solver budget limits (10,000 steps / 100ms / instantiation depth 3); over-budget returns
  `Unproven`
- Dependent-type pre-reduction: deterministic value computations are consumed first; SMT only chews
  on the non-deterministic part
- `Unproven` is not a dead end: for the correctness domain, write a proof function (return type =
  proposition); for the termination domain, give the measure at the type position (§6.9) — both are
  verified by the type checker
- Incremental verification: only changed modules are re-verified
- Clear error messages + counterexample display + budget consumption report + unsolved proposition +
  suggestions (if the compiler can offer them)

## Alternatives

| Alternative                                                        | Why not chosen                                                                                                                                                                 |
| ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| RFC-022: `//!` comment-style specification                         | Specifications and types split, violating the Curry-Howard correspondence                                                                                                      |
| Independent specification file (e.g. CVL)                          | Specifications separated from code, increasing maintenance cost                                                                                                                |
| Runtime-only assertions                                            | Cannot statically guarantee correctness                                                                                                                                        |
| External proof assistant (e.g. Coq)                                | Disconnected from the compiler, requiring a separate proof language and trust boundary. YaoXiang's choice: the proof is YaoXiang code, the type checker is the sole trust root |
| **This proposal: compile-time predicate as a first-class citizen** | ✅                                                                                                                                                                             |

## Implementation Strategy

### Phases

| Phase       | Content                                                                                                                                                   |
| ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Phase 1** | Compiler kernel: structural equivalence + βδι-reduction + forall introduction/elimination. Supports simple arithmetic predicates (`x > 0`, `arr.len > 0`) |
| **Phase 2** | SMT-LIB translation layer + Z3/CVC5 integration. Pipeline returns `Proved/Disproved/Unproven`. `Unproven` supports programmer-written proof functions     |
| **Phase 3** | Loop-invariant VC generation + termination checking (four measure-exploration strategies + `Terminates` explicit measure, §6, §6.9)                       |
| **Phase 4** | Incremental verification + caching + IDE support                                                                                                          |

### Dependencies

- RFC-010: Unified Type Syntax — compile-time predicates are based on `name: type = value`
- RFC-011: Generic Type System — compile-time predicates can carry generic parameters
- RFC-009: Ownership Model — expressions inside compile-time predicates obey the ownership rules

## Open Questions

- [x] **Z3 availability on wasm32 target**: no Z3 on wasm (all SMT code is excluded by
      `cfg(not(target_arch = "wasm32"))`). Scope of impact and downgrade direction: - `ownership.rs`
      `smt_cut` always returns `false` → back edge passes through → **conservative rejection**
      (sound direction, just less narrowing) - Termination-checking Strategy 1 and predicate levels
      2b/3 are skipped as a whole

  **Correction (2026-10-01)**: the original text claimed these paths were "currently
  **unreachable**" (on the grounds that `predicate_defs` production was never populated, `parser`
  never produces `MonoType::Refined`, and `with_solver` production was never called), no longer
  holds — all three are now landed (predicate-definition registration #377-3; refinement-type
  normalization; production pipeline injecting the solver). So the difference between wasm and
  native is no longer limited to `smt_cut` precision:

  | Path                                           | native   | wasm       | Downgrade direction                  |
  | ---------------------------------------------- | -------- | ---------- | ------------------------------------ |
  | Termination-checking explicit-measure judgment | Decided  | `Unjudged` | Don't report E4022 (under-reporting) |
  | Well-foundedness judgment                      | Decided  | `Unjudged` | Don't report E4022 (under-reporting) |
  | Refinement-predicate SMT implication level     | Executed | Skipped    | Falls to `Unproven`                  |

  All are **conservative** directions (less narrowing, under-reporting), not affecting soundness. If
  precision parity is desired, the plan and the cost are in issue #376 (conclusion: prefer a JS-side
  Z3 instance, rather than linking Z3 into the main wasm — the latter requires changing the emcc
  build system and grows the artifact from 3 MB to ~20 MB).

- [x] **SMT solver choice**: default Z3 (MIT license, most widely verified). CVC5 as an
      SMT-LIB-compatible alternative, switched via a compiler flag. The compiler's internal
      translation target is the SMT-LIB 2.6 standard format — SMT-LIB _is_ the abstraction layer; no
      custom generic-solver interface.
- [x] **Specific solver-budget values**: 10,000 steps / 100ms / quantifier-instantiation depth 3.
      Fixed inside the compiler, no knob. In practice, if a real use case proves this is not enough
      (not "the user wrote it wrong"), it will be adjusted.
- [x] **Quantifier support scope**: the language layer does not limit the quantifier order.
      Compile-time predicates accept `Type` arguments — `Type` includes function types — so
      higher-order quantifiers are a natural corollary of the type system, requiring no special
      syntax. The SMT solver can automatically decide first-order quantifiers (forall/exists, with
      interleaved nesting allowed, capped by budget depth 3). Higher-order quantifiers: SMT returns
      `Unproven`; the compiler prompts "this predicate is beyond the auto-proving range, please
      provide a proof function". The programmer writes a YaoXiang function whose return type equals
      the proposition — the type checker verifies that function. No external export, no AI, no
      interactive proof mode. Everything is YaoXiang code, everything is verified by the type
      checker.
- [x] **Counterexample formatting**: source variable names are used directly as SMT variable names
      (with a module prefix to avoid collisions). When the Z3 model is returned, the variable name
      is reverse-looked-up. Output format: variable name = concrete value + source location +
      predicate-definition location. No complex mapping layer.
- [x] ~~**Compile-time predicate interaction with `ref` smart pointers?**~~ → Decided: compile-time
      predicates only allow immutably-borrowed or ownership-transferred values. Mutably-borrowed
      values may not appear in compile-time predicates.
- [x] **Extending the `forall` predicate violation-count measure to non-adjacent operations?** → No
      extension. The current coverage (adjacent swap, adjacent move) is complemented by Strategy 1
      (linear rank function) — quicksort's outer interval shrinking is covered by Strategy 1, and
      heapsort is covered by Strategy 1 (array-index pattern). Loops whose termination cannot be
      proved by any strategy are directly rejected by the compiler — this is the hard-safety
      philosophy, not a defect. If in the future some real-world algorithm (not an academic
      construction) emerges that all four strategies cannot cover, it will be revisited. →
      **Re-discussion triggered (2026-09-14, #318)**: non-structural recursion (gcd-style
      non-directly-decreasing, mutual recursion, merge partition) are exactly the real-world
      scenarios this clause anticipated — the template sequence of measure exploration cannot
      auto-handle them, and they cannot be rewritten into analyzable iteration patterns without
      sacrificing readability. Conclusion: the hard-safety philosophy stands (a wrong measure is
      still turned back by SMT counterexamples; humans can fail to prove, but cannot prove wrong),
      but "rejected if exploration fails" is relaxed to "the programmer can give an explicit measure
      at the type position when exploration fails" — see §6.9.
- [x] **Linear-rank-function enumeration combinatorial explosion**: candidate-enumeration cap is 3
      bounded variables. ≤3 enumerates all linear combinations and verifies them with SMT one by
      one. >3 only tries single-variable measures (`v_i`, `u_i - v_i`); failure directly reports a
      compile error — telling the programmer "this loop has more than 3 bounded variables; the
      compiler cannot auto-synthesize a multi-variable measure". This is not an engineering
      compromise — it forces the programmer to write simpler loops.

## References

- [RFC-010: Unified Type Syntax](010-unified-type-syntax.md)
- [RFC-011: Generic Type System Design](011-generic-type-system.md)
- [RFC-009: Ownership Model](009-ownership-model.md)
- Howard, W. A. (1969). The Formulae-as-Types Notion of Construction.
- Swamy, N. et al. (2016). Dependent Types and Multi-Monadic Effects in F\*. _POPL 2016_.
- Vazou, N. et al. (2014). Refinement Types for Haskell. _ICFP 2014_.
- Leino, K. R. M. (2010). Dafny: An Automatic Program Verifier for Functional Correctness. _LPAR
  2010_.
- De Moura, L. & Bjørner, N. (2008). Z3: An Efficient SMT Solver. _TACAS 2008_.

---

## Lifecycle and Outcome

```
┌─────────────┐
│   Draft     │  ← Created by the author
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
