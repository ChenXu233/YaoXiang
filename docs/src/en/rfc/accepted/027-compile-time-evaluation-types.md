---
title: 'RFC-027: Compile-Time Predicates and Unified Static Verification'
status: 'Accepted'
author: 'Chen Xu'
created: '2026-06-07'
updated: '2026-10-03'
impl_status: 'in-progress'
impl_detail:
  'Phases 1-2 complete, Phase 3 partial, Phase 4 partial. The unified assert/Assert scheme across
  all 6 phases is fully implemented (#157-#162 closed): Never type, IsTrue bridge, flow-sensitive Γ
  + kill set, type-level recursion, universe stratification weak check. The **semantics** of §11
  dispatch is implemented via call-site obligations (three direct calls to `check_predicate` in
  `checker.rs`: binding site / call-site arg / return site), but the **standalone
  `layers/dispatch.rs` module was removed**—it had zero production call sites, and its handling of
  `Unproven` (downgrade to W1080 warning) conflicted with the production implementation (always
  error) (#377-2).'
impl_percent: 82
issue_number: 90
issue_url: 'https://github.com/ChenXu233/YaoXiang/issues/90'

issue: '#90'
---

# RFC-027: Compile-Time Predicates and Unified Static Verification

> **References**:
>
> - [RFC-009: Ownership Model](009-ownership-model.md)
> - [RFC-010: Unified Type Syntax — name: type = value model](010-unified-type-syntax.md)
> - [RFC-011: Generics System Design](011-generic-type-system.md)
> - [RFC-024: Concurrency Model Based on spawn Blocks](024-concurrency-model.md)
>
> **Supersedes**:
> [RFC-022: Hoare Logic Static Verification Support (Specification Comments and Specification Types)](../deprecated/022-hoare-logic-static-verification.md)
> — deprecated

## Summary

This RFC proposes introducing **compile-time predicates** as first-class citizens to YaoXiang,
unifying all compile-time static verification into a single **proof pipeline**. A compile-time
predicate is not an externalized specification comment — it is a function. A function that returns a
Type can be used at a type position; the compiler calls it at compile time and checks the return
value. Types are propositions, and compile-time evaluation is proof.

**Core argument**: the only job of the type checker at compile time is to construct and verify proof
terms. Type equality, token conflicts, dependent type reduction, compile-time predicate evaluation,
and Hoare logic implication are all different type checks within the compile-time proof pipeline,
sharing a single pipeline. The SMT solver is an acceleration module of the type checker, not a
separate trust boundary. When the compiler returns `Unproven`, the programmer writes a YaoXiang
function as proof — the type checker verifies it in exactly the same way it verifies any function's
return type. Everything is YaoXiang code, and everything is verified by the type checker.

## Motivation

### Why deprecate RFC-022?

RFC-022 designed specifications in the form of `//!` comments:

```yaoxiang
max: (T: Ord) -> ((arr: Array(T, n)) -> T) = {
    //! requires: NonEmpty(n) = n > 0          ← This is a comment independent of types
    //! ensures: ExistsMax(result, arr[0..n])   ← This is a comment independent of types
}
```

This commits a fundamental error of the Curry-Howard isomorphism: **splitting specifications and
types into two layers**. Comments are not types. Comments do not participate in type checking.
Comments belong to the mental model of "external tools."

The white paper states this clearly:

> "No `//!` comments. No separate specification language. Everything stays within the type system."

### Current problems

- RFC-022's `//!` comments are externalized syntax independent of the type system
- Specification types and ordinary types form two systems, causing conceptual redundancy
- The Debug Build verifies / Release Build ignores split pattern breaks unification
- In conventional understanding, the SMT solver is positioned as an external tool — YaoXiang treats
  it as a built-in acceleration module of the type checker
- Type checking, borrow verification, compile-time predicate checks, and macro expansion each take
  different paths

### The correct mental model

Type checking can be abstracted as a function:

```
verify : Program → Proved | Disproved(Model) | Unproven
```

All compile-time checks — simple type matching, borrow conflict detection, compile-time predicate
verification — are sub-tasks of this function. They share a single proof pipeline, differing only in
proof term complexity and construction strategy.

When the compiler returns `Unproven`, the programmer provides a proof function — its return type
equals the proposition to be proven. The type checker verifies it. This is the same operation as
ordinary type checking.

## Proposal

### 1. `{}` is the proof space: types are assertions, verification is type checking

YaoXiang's `{}` is the compile-time proof space. Everything inside is an assertion, and the compiler
guarantees each is True — either proven automatically or by a programmer-supplied proof function.

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
#          parameter in the   only assertions
#          signature          inside {}
#          The compiler verifies x > 0 at the call site

List: (T: Type) -> Type = { data: Array(T) }
#      ^^^^^^^^              ^^^^^^^^^^^^^^^
#      parameter in the     The compiler verifies
#      signature            type_of(T) == Type, type_of(data) == Array(T)
```

The same pattern: `name: (params) -> Type = { assertions }`. The compiler does not distinguish
between "type assertions" and "value assertions" — both are evaluation targets in the proof
pipeline.

**Loop invariants need not be written separately. Type annotations on variables are Floyd-Hoare
invariants.**

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # The annotation references i — tells the compiler s's type depends on i
    mut i: UpTo(arr.len) = 0     # At initialization i=0, verify: 0 == sum(arr[0..0]) → True
    while i < arr.len {
        s += arr[i]  # The compiler verifies: s_new == sum(arr[0..i+1])
        i += 1       # i changes → triggers re-verification of s's dependency: s satisfies SumUpTo(arr, i_new)
    }
    return s  # s: SumUpTo(arr, arr.len) = sum(arr[0..arr.len])
}
```

The compiler generates one verification condition for the loop body — the inductive hypothesis (type
annotation) → the assignment operation → whether the new value satisfies the type annotation. Once
the proof pipeline verifies the inductive step, all iterations are covered automatically. No
`: decreases`, no `: Invariant`, no inductive proof is needed — the compiler decomposes induction
into local VCs for each assignment.

### 2. Pre/post-conditions: compile-time predicates on parameter and return types

Drop RFC-022's `//! requires`/`//! ensures`. Compile-time predicates serve as annotations on
parameter or return types.

**The parameter side is a function call.** A compile-time predicate is a function that returns Type,
and its use at the parameter side is simply calling it — just like `factorial(5)`. The return side
introduces a new concept: the return value parameter.

```yaoxiang
# Pre-condition: explicitly invoke a compile-time predicate in a parameter type
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b
#                       ^^^^^^^^^^  b is the current parameter name, passed to Positive as an argument
#                       At the call site, the compiler extracts the argument value, substitutes into b, and verifies Positive(arg)
#                       Example: divide(10, 2) → verify Positive(2) = { 2 > 0 } → True
#                       Example: divide(10, 0) → verify Positive(0) = { 0 > 0 } → False → compile error

# Post-condition: return value parameter + compile-time predicate
IsMax: (T: Ord, arr: Array(T), result: T) -> Type = {
    forall j in 0..arr.len: result >= arr[j]
}

NonEmpty: (arr: Array(T)) -> Type = { arr.len > 0 }

max: (T: Ord) -> ((arr: NonEmpty(arr))) -> (result: IsMax(T, arr, result)) = {
#                                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
#                                            result is the return value parameter; its value is provided by return
#                                            The compiler substitutes the return value at the return point and verifies the post-condition
    candidate = arr[0]
    for i in 1..arr.len {
        if arr[i] > candidate { candidate = arr[i] }
    }
    return candidate
}
```

**Key rules**:

- **Parameter side**: `b: Positive(b)` — `b` is the current parameter name, passed to `Positive` as
  an argument. Function call syntax, zero implicits.
- **Return side**: `-> (result: IsMax(T, arr, result))` — `result` is the return value parameter,
  whose value is provided by the `return` statement. `result` only exists in the type signature and
  is only referenced by predicates; it does not enter the function body scope and does not appear at
  the caller.
- **Return value parameters are optional**: when there is no post-condition, omit them — the
  signature is identical to a normal function (`-> Int`).
- **Unification**: parameter and return value parameters are the same concept —
  `param_name: predicate_call(args)` — the only difference is whether the value is provided by the
  caller or by `return`.

> **Implementation note (2026-10-02)**: The identification of the return-site formal parameter is
> **strictly by the declared name** — the `r` in `-> (r: P(...))` is the unique return value
> parameter. Free variables in the constraint that are **not in scope** are no longer substituted
> with the return value; instead, they are reported as undefined identifiers (verified:
> `g: () -> (r: Eq2(m, r)) = { return 7 }` (`m` undefined) → `error [E1001] Unknown variable: 'm'`).
> The arguments of a predicate call are **really** substituted into the constraint by the argument:
> after substitution, `P(r + 100)` becomes "return value + 100", not "return value".
>
> **Known gap (2026-10-02)**: **multi-argument predicates with symbolic arguments at the return site
> are not yet supported**. Trigger form: `f: (b: Int) -> (r: SumUpTo(b, r)) = { return b * 2 }`
> (`SumUpTo: (n: Int, s: Int) -> Type = { s == n * 2 }`) →
> `error [E2031] … the refinement type constraint of 'r': SumUpTo(b, (b * 2)) cannot be statically proven`.
> Workarounds: change the argument to a compile-time-foldable literal (e.g. `SumUpTo(3, r)`), or
> provide a proof function returning `Type` for that predicate; lifted when the proof kernel
> supports "multi-argument predicate + symbolic argument" argument substitution. See
> [RFC-027a](../accepted/027a-termination-explicit-measure.md) §Implementation Landing Record
> (2026-10-02, this hardening pass).

### 3. Path condition propagation: compile-time verification of runtime values

When a compile-time predicate is used at a binding site, the parameters are passed explicitly by the
programmer. When a runtime value enters a refinement type parameter, the compiler completes
verification through path condition collection and SMT implication judgment — without requiring the
programmer to pass a proof explicitly.

#### 3.1 Explicit function calls

When a compile-time predicate is used at a binding site, the parameters are passed explicitly by the
programmer — it is a function call, with zero implicits.

`Positive: (x: Int) -> Type = { x > 0 }` is a compile-time predicate constructor. When it appears at
a binding site (parameter declaration, variable declaration, return type), the programmer explicitly
passes a bound variable name:

```yaoxiang
b: Positive(b)
// b is already declared as the current parameter, Positive(b) is a function call
// After normalization: b: { b > 0 }
```

The compiler does not need to implicitly fill in parameters — `b: Positive(b)`, like `f(5)`, is
simply a function call. `b` is bound as a parameter name, and its type annotation `Positive(b)`
references `b` itself — this is the standard pattern of dependent types, not an implicit expansion
rule.

**Unification with RFC-010's `self`**: RFC-010 establishes that `self` is not a keyword, just a
conventional name for a parameter ("writing it as `p`, `this`, or `x` works exactly the same").
`b: Positive(b)` shares the same mechanism — the parameter name can be referenced in the type
annotation. `self` appears at the position `self: Point`; `b` appears at the position
`b: Positive(b)`; both have type annotations referencing the parameter itself. The difference is
only the complexity of the type annotation; the mechanism is exactly the same — once the name is
bound, the type may depend on that name.

The return type uses explicit function calls in the same way:

```yaoxiang
Sorted: (arr: Array(T)) -> Type = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }

sort: (arr: Array(T)) -> (result: Sorted(result)) = { ... }
//                        ^^^^^^^^^^^^^^^^^^^^^^^
//                        result is the return value parameter; Sorted(result) is a function call
//                        The compiler substitutes the return value into result at the return point and verifies Sorted(return value)
```

The same applies to local variable declarations:

```yaoxiang
let x: Positive(x) = 5
// x is bound to 5, Positive(5) → { 5 > 0 } → True → passes

// let y: Positive(y) = 0
// y is bound to 0, Positive(0) → { 0 > 0 } → False → compile error
```

#### 3.2 Path condition collection

When a runtime value appears in a conditional branch, the compiler automatically collects path
conditions to form the **assumption set** for the current scope. These assumptions participate in
verification as background knowledge for compile-time Bool evaluation.

```yaoxiang
if y > 0 {
    // The compiler automatically acquires the assumption in this branch: { y > 0 }
    let result = divide(x, y)
    // Verification condition: (y > 0) ⇒ (y > 0)
    // The proof pipeline judges the implication holds → Proved
} else {
// This branch's assumption: { !(y > 0) }
// If divide(x, y) were to be called, the verification condition is !(y > 0) ⇒ y > 0
    // The proof pipeline judges it does not hold → Disproved
}
```

This is not a hard-coded special pattern in the compiler — it is the natural behavior of the
compile-time proof pipeline. Each type-check call site sends the following to the pipeline:

```
{background assumptions} ⇒ {verification goal}
```

The proof pipeline judges the implication. `Proved` → pass; `Disproved` → compile error +
counterexample; `Unproven` → compile error + unsolved proposition. The background assumptions come
from the path conditions of the current program point.

#### 3.3 Assumption stack

When analyzing control flow, the compiler maintains an assumption set for each basic block:

- **if-guard**: `if y > 0` → push `y > 0` into the true branch, push `!(y > 0)` into the false
  branch (if `else` is used)
- **match patterns**: `if let Some(v) = opt` → push `opt == Some(v)` into the branch
- **Logical conjunction**: `if x > 0 and y < 10` → push `x > 0` and `y < 10` into the branch
- **Function pre-conditions**: when calling `divide(a, b)`, the evidence that `b` satisfies
  `Positive` must come either from current assumptions or from the refinement type annotation on the
  argument itself (if `b` is annotated as `Positive`, its type carries `b > 0`)
- **Assignment**: when `let z = y`, the refinement conditions already on `y` are transferred to `z`

All assumptions enter the compile-time proof pipeline. When they enter the SMT-accelerated path,
they are translated to SMT-LIB background assertions.

#### 3.4 No static evidence → compile error

If the programmer writes directly:

```yaoxiang
divide_user_input: (x: Int, y: Int) -> Int = divide(x, y)
```

There is no `y > 0` assumption at the current program point, and the argument `y` itself has no
`Positive` type annotation. The verification condition is:

```
{} ⇒ { y > 0 }
```

The pipeline returns `Disproved` (implication does not hold) → compile error:

> Cannot prove that argument `b` satisfies `Positive` in the call to `divide`. `y` comes from
> function input with no proven bound. Consider guarding the call with an if branch:
> `if y > 0 { divide(x, y) }`.

YaoXiang does not accept a runtime value flowing directly into a refinement type parameter without
static evidence. This is not a restriction — it is at the heart of the hard-safety philosophy. Code
the compiler cannot statically prove must not pass compilation.

#### 3.5 Relationship to the unified pipeline

Path condition propagation is not an extra mechanism. It is the direct extension of the compile-time
proof pipeline in control flow analysis:

| Stage                             | Responsibility                                                                                                                                          |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Path condition collection         | Compiler's control flow analysis stage, annotating each basic block with its assumption set                                                             |
| Verification condition generation | When a type constraint requiring verification is encountered, merge path conditions with argument type info                                             |
| Proof pipeline evaluation         | Compiler kernel → SMT acceleration → conclude Proved / Disproved / Unproven                                                                             |
| Result                            | `Proved` → pass; `Disproved` → compile error + counterexample; `Unproven` → compile error + unsolved proposition (programmer may supply proof function) |

No new components. No special rules. Path conditions are background knowledge for the proof pipeline
— sharing the same pipeline and budget system as type equality and borrow constraints.

### 4. The compile-time proof pipeline

All compile-time checks share a single pipeline. The core operation of the pipeline is **type
checking** — checking whether a proof term's type equals the proposition to be proven. Everything is
type checking.

```
The compiler encounters a Bool expression to evaluate at compile time (i.e., needs to construct a proof term)
        │
        ├── Type equality (T1 == T2)
        │   → The compiler judges directly (structural equivalence)
        │
        ├── Token conflict condition (!conflicting(tokens))
        │   → Flow-sensitive liveness analysis (Dup/Linear property tracking)
        │
        ├── Dependent type reduction (n + m simplification)
        │   → Compile-time term rewriting system (βδι-reduction)
        │
        ├── Compile-time predicate (x > 0, forall...)
        │   → Compiler kernel + SMT acceleration module
        │
        └── Hoare logic implication (P ⇒ Q)
            → Compiler + SMT acceleration module
                    │
                    ▼
             ┌──────────┐
             │ Proved   │  → compilation passes
             │ Disproved│  → compile error + counterexample
             │ Unproven │  → compile error + unsolved proposition
             └────┬─────┘
                  │
                  ▼
         Programmer writes a proof function (YaoXiang code)
                  │
                  ▼
         Type checker verifies ──→ Proved ──→ compilation passes
                  │
                  ▼
            Verification failed → compile error: "proof does not hold"
```

#### 4.1 Proof results: a three-valued algebra

Compile-time evaluation returns three results — this is the necessary conclusion of the halting
problem and a natural partition of proof theory:

```
eval_compile_time : BoolExpr → Proved | Disproved(Model) | Unproven
```

- **Proved** → halts, the proof term is constructed, type check passes. Compilation continues.
- **Disproved(M)** → halts, there exists a counterexample M. Compile error + counterexample + source
  location.
- **Unproven** → the proof is not constructed within the given resource cap. Compile error +
  unsolved proposition + budget consumption report.

**Unproven ≠ False.** The compiler saying "I cannot prove it" is not equivalent to the proposition
being false — it is simply beyond the current automatic proof capability. This is honesty, not a
defect.

A hard budget limit is the engineering solution to the halting problem. No knob — offering one would
amount to asking the user "do you think your program will halt," and neither the user nor the
compiler knows.

#### 4.2 After Unproven: the programmer writes a proof

When the compiler returns `Unproven`, the programmer may write a **proof function** — a YaoXiang
function whose return type equals the proposition to be proven. The type checker verifies this
function — using the exact same mechanism as it verifies `add(a, b): Int`.

```
Proposition = Type
Proof       = Program (a value of that type)
Verification = Type checking (the only root of trust)
```

The SMT solver is not a separate trust boundary — it is an **acceleration module of the type
checker**. SMT helps find proofs, but the one that verifies them is always the type checker. When
SMT returns `unsat`, the compiler restructures its result as a proof term verifiable by the type
checker. If the restructuring fails (the SMT's reasoning steps exceed the compiler kernel's
inference rules), it falls back to `Unproven` — the programmer can manually write a proof function.

```yaoxiang
# Proposition: a refinement property the compiler cannot prove automatically
FirstIsMin: (T: Ord, arr: Sorted(T)) -> Type = {
    forall i in 0..arr.len: arr[0] <= arr[i]
}

# Proof: the programmer writes a function whose return type is the proposition above
# The type checker verifies this function — exactly as it verifies add(a,b): Int
first_is_min: (T: Ord, arr: Sorted(T)) -> FirstIsMin(T, arr) = {
    # The compiler verifies here: the function body's type = FirstIsMin(T, arr)
    ...
}
```

No AI, no export to Coq, no new concepts. **Properties that the compiler cannot prove automatically
at compile time → the programmer writes a proof in YaoXiang code → the type checker verifies it.**
The entire process is a smooth gradient — the compiler handles simple proofs and leaves the brain
for the hard ones.

#### 4.3 Hierarchical dependencies within the pipeline

The evaluators above share the same interface but have an evaluation order. Type equality is the
prerequisite for all subsequent analyses; ownership/token checks depend on type information;
refinement predicate verification depends on the results of the first two layers. The compiler
evaluates layer by layer, and expressions that fail at a lower layer do not enter the upper layer —
avoiding wasting solver budget on type-erroneous programs.

```
Evaluation order (single pipeline, layered scheduling)
├── Layer 0: Type equality (T1 == T2)
│   └── Structural unification → failure makes further analysis meaningless, return Disproved directly
├── Layer 1: Ownership/token conflicts
│   └── Flow-sensitive liveness analysis → failure means memory safety does not hold, return Disproved directly
└── Layer 2: Refinement predicate / Hoare implication
    └── Compiler kernel → SMT acceleration → conclude Proved / Disproved / Unproven
```

Each layer still returns `Proved/Disproved/Unproven`, sharing the same interface and the same budget
system.

### 5. Unification of three function layers

| Layer                  | Execution time | Input        | Output | Example                                        |
| ---------------------- | -------------- | ------------ | ------ | ---------------------------------------------- |
| Value-level function   | Runtime        | Values       | Value  | `add: (a: Int, b: Int) -> Int = a + b`         |
| Type constructor       | Compile time   | Types/values | Type   | `List: (T: Type) -> Type = { data: Array(T) }` |
| Compile-time predicate | Compile time   | Values       | Type   | `Positive: (x: Int) -> Type = { x > 0 }`       |

All use the same `name: type = value` syntax. Compile-time predicates and type constructors go
through the same compile-time proof pipeline — `{}` is the proof space.

### 6. Loops: Floyd-Hoare verification condition generation

Loops do not need a separate `: Invariant(...)` or `: decreases(...)` annotation. The compile-time
predicate type annotations on variables define Floyd-Hoare-style assertions — the compiler generates
verification conditions from the type annotations, and the proof pipeline checks whether each
assignment preserves the type.

Core mechanism: each assignment operation corresponds to a Hoare triple `{P} x := e {Q}`, and the
verification condition is `P ⇒ Q[e/x]`. The compiler generates one verification condition for the
loop body — once the proof pipeline verifies the inductive step, all iterations are covered
automatically.

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # Annotation references i; at initialization i=0, verify: 0 == sum(arr[0..0]) → True
    mut i: UpTo(arr.len) = 0     # Verify: 0 <= 0 <= arr.len → True
    while i < arr.len {
        # The compiler generates one VC for the loop body. Precondition: s satisfies SumUpTo(arr, i); i satisfies UpTo(arr.len).
        #
        # s += arr[i]:
        #   Verification obligation: s_new satisfies SumUpTo(arr, i) (current i unchanged)
        #   Substituting s_new = s_old + arr[i]:
        #     Need s_old + arr[i] == sum(arr[0..i+1])
        #     From the inductive hypothesis s_old == sum(arr[0..i]), add arr[i] to both sides:
        #     sum(arr[0..i]) + arr[i] == sum(arr[0..i+1])
        #   Compiler + SMT: linear arithmetic, millisecond response → Proved
        #
        # i += 1:
        #   i changes → s's type annotation in the dependency graph references i → triggers re-verification
        #   New verification target: s satisfies SumUpTo(arr, i_new)
        #   i.e. s == sum(arr[0..i_new]), guaranteed by the previous step → Proved
        s += arr[i]
        i += 1
    }
    return s  # At this point s: SumUpTo(arr, arr.len), i.e. s == sum(arr[0..arr.len])
}
```

Loop invariants are simply type annotations on variables — the programmer writes the type, and the
compiler checks the inductive step. The compiler does not need to "discover" invariants, nor to
"perform induction automatically" — it decomposes the inductive proof into local verification
conditions for each assignment, and the proof pipeline handles them divide and conquer.

#### 6.1 Dependency tracking: dependent types on mutable variables

The prerequisite for the above mechanism is: the compiler knows that the type annotation
`SumUpTo(arr, i)` on `s` references `i` — when `i` changes, the type constraint of `s` also changes
accordingly. This requires the compiler to maintain a **type dependency graph between variables**.

**Data structure**:

```
TypeDepGraph: Map<VarName, Set<VarName>>
# The key is the depended-upon variable, the value is the set of variables whose type annotation references that variable
# Example: { i: {s}, j: {s, t}, ... }
```

**Construction**: when the type checker processes `mut v: Pred(... x ...) = init`, it parses the
free variable references in the `Pred(...)` arguments. If a reference targets another mutable
variable `x` in the current scope, it records `x → v` in the dependency graph.

**Trigger**: when the depended-upon variable `x` is reassigned, the compiler:

1. Looks up the set of all variables `{v₁, v₂, ...}` in the dependency graph that depend on `x`
2. For each `v`, generates a verification condition: does `v`'s current value satisfy the updated
   type `Pred(... x_new ...)`?
3. Sends the VC into the proof pipeline

**Assignment order is sensitive**: dependency tracking naturally enforces the correct assignment
order. Using `SumUpTo(arr, i)` as an example:

```yaoxiang
# Correct order
s += arr[i]   # s_new satisfies SumUpTo(arr, i+1)
i += 1        # i changes → re-verify s satisfies SumUpTo(arr, i_new) → True

# Wrong order — compiler rejects
i += 1        # i changes → re-verify s satisfies SumUpTo(arr, i_new)
              # s has not been updated yet, s_old == sum(arr[0..i_old]) ≠ sum(arr[0..i_new])
              # → compile error: variable s does not satisfy type SumUpTo(arr, i_new)
s += arr[i]   # unreachable
```

**Combined dependencies**: a variable can depend on multiple variables. The type annotation
`{ v: Int; v == x + y }` depends on both `x` and `y` — a change in either triggers re-verification.

**Relationship to the proof pipeline**: dependency tracking is a trigger for VC generation, not an
independent verification mechanism. It answers "when does a VC need to be generated" — the proof
pipeline answers "whether the VC holds".

### 7. Termination check

**Scope: refinement types.** Termination is not an independent switch, but a part of the
**verification mode**: once a type is refined (`Refined { base, constraint }`), the computation it
annotates enters verification mode, and termination is required within the mode; ordinary types that
are not refined do not enter verification mode and generate no termination obligations.

This yields two corollaries:

- **Loops**: a bare `while` does not enter verification mode; when a measure variable carries a
  refinement annotation (e.g. `i: UpTo(n)`), it enters verification mode, and termination must be
  proven.
- **Recursion**: when a function signature carries refinement (refined parameter or refined return
  type), it enters verification mode, and the strictly decreasing measure of every recursive call
  site must be proven.

Full automation is preferred within the mode: the compiler first automatically explores the measure;
if it can prove termination, the program passes; if it cannot explore a measure and none is given
explicitly, it is a compile error. There is **no annotation-syntax hole** — measures and termination
propositions are written at the type position, and no new syntax like `decreases` is introduced. The
forms of measures and explicit fallbacks are given in §6.9.

#### 6.1 Design principle

The compiler automatically extracts the information needed for termination proofs from two places:

1. **Variable type annotations**: boundary constraints in refinement types (e.g. `UpTo(n)` gives an
   upper bound `n` and a lower bound `0`)
2. **Loop body operations**: the operation applied to the variable at each iteration

The compiler tries four measure synthesis strategies in priority order, and stops when one succeeds.
The four strategies are a **restricted template sequence for measure exploration**; their input is
the refinement constraint (strategies 1–4 all start with "variables with bounded types"), not "code
evaluated at compile time"; they and the explicit measures in §6.9 are the automatic and manual
sides of the same thing.

> **Measure exploration is exploration, not inference.** Exploration only enumerates templates
> (linear rank, violation count, bounded pattern, multiplicative scaling); it does not guarantee a
> solution exists — general measure inference is generally undecidable (reduces to the halting
> problem). Therefore, situations outside the templates must be expressible by the programmer
> explicitly giving the measure (§6.9); otherwise it is a compile error.

#### 6.2 Strategy 1: automatic synthesis of linear rank functions

When a variable has a linear bound annotation, the compiler enumerates candidate linear measures and
verifies them via SMT.

```
Input:
  Variables v₁: UpTo(u₁), v₂: UpTo(u₂), ... (variables with upper and lower bounds)
  Loop condition cond
  Set of assignments in the loop body

Algorithm:
  1. Extract the bounds of each variable from the type annotations: [low_i, high_i]
  2. Enumerate candidate measures: v_i, u_i - v_i, v_i - v_j, and other linear combinations
  3. For each candidate measure m:
     - SMT verifies m ≥ 0 (derived from the type bounds)
     - For every execution path of the loop body, SMT verifies m' < m (strictly decreasing)
  4. Find a linear combination that satisfies the conditions → termination is proven
```

Coverage: loops where any variable is assigned a linear expression (`v = a·v + b`) and has a bounded
type annotation. Includes `i += const`, `i -= const`, and binary-search-style interval contraction:

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

#### 6.3 Strategy 2: predicate violation count — automatic measure extraction from the goal type <span style="color:orange">【Experimental strategy】</span>

> ⚠️ **Current status: experimental strategy, decided in Phase 3 implementation based on actual
> feasibility.** This strategy is effective for adjacent swap operations (bubble sort, insertion
> sort), but cannot automatically prove for non-adjacent operations (quicksort partition, heapsort
> sift-down). See the table below for coverage boundaries. If Phase 3 verification is infeasible,
> this strategy will be removed or downgraded to future work.

Core insight: **the specifications the user writes are the material for the compiler's reasoning.**
The compiler does not need a built-in notion of "what is sorting" — it reads the definition of
`Sorted` and automatically extracts the measure from it.

```
Input:
  Goal type: Sorted(arr) = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }
  Loop body operation: adjacent element swap

Algorithm:
  1. Parse the predicate definition: forall i in range: cond(i, arr)
  2. Automatically generate the measure: violation_count = |{ i | ¬cond(i, arr) }|
  3. Analyze the impact of the operation on the measure:
     - Adjacent swap arr[j], arr[j+1] = arr[j+1], arr[j]
     - Affects only the three pairs with index j-1, j, j+1
     - If arr[j] > arr[j+1] (predicate violated), this pair satisfies the predicate after the swap
     - violation_count decreases by at least 1
  4. Upper bound: n·(n-1)/2 (max number of adjacent inversions), lower bound: 0
  → Termination is proven
```

**Current coverage**:

| Algorithm      | Operation mode    | Provable by Strategy 2? | Reason                                                  |
| -------------- | ----------------- | :---------------------: | ------------------------------------------------------- |
| Bubble sort    | Adjacent swap     |           ✅            | violation_count strictly decreases per swap             |
| Insertion sort | Adjacent move     |           ✅            | Each move eliminates one violating pair                 |
| Selection sort | Non-adjacent swap |           ❌            | A single swap may increase violation_count              |
| Quicksort      | partition         |           ❌            | Non-adjacent swap, no monotonic decrease                |
| Heapsort       | sift-down         |           ❌            | Tree-shaped operation, violation_count is non-monotonic |

**Complementary strategy**: for quicksort, `low < high` interval contraction can be covered by
Strategy 1 (linear rank function) — the outer partition recursion halves the interval each time.
Strategies 1 and 2 complement each other, and most practical algorithms' termination can be proven
by one of them. But the generalization of Strategy 2 (non-adjacent operations, tree-shaped
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

#### 6.4 Strategy 3: bounded increment/decrement pattern

`v += const` (positive constant), and the variable has an upper-bound type annotation → the measure
`upper_bound - v` decreases by `const` each time, with lower bound 0. This is a degenerate case of
Strategy 1; the compiler handles it first and quickly.

#### 6.5 Strategy 4: multiplicative scaling measure template

`v *= const` (const > 1), and the variable has upper- and lower-bound type annotations. The compiler
has a built-in logarithmic measure template `ceil(log_const(upper/v))`, which decreases by 1 on each
multiplication by const.

```yaoxiang
mut i: Positive(i) = 1
while i < n {
    # The compiler automatically derives: measure ceil(log₂(n/i)), decreases by 1 on each multiplication by 2
    i *= 2
}
```

#### 6.6 Separating termination and correctness

Termination proofs and correctness proofs are independent:

- **Termination**: the four strategies above automatically prove that the loop exits in a finite
  number of steps; when exploration fails, the programmer gives the measure at the type position
  (§6.9)
- **Correctness**: whether the loop body advances toward the goal type, checked by the compile-time
  proof pipeline through verification conditions

If both pass → compilation passes. Termination proven but correctness fails → compile error +
counterexample. Correctness proven but termination cannot be proven → compile error pointing out the
variable or operation that cannot be analyzed. Both fail → compile error reports both failure
reasons separately.

#### 6.7 Termination check for recursive functions

For recursive functions with refined signatures, the compiler checks the formal parameter decrease
at every recursive call site:

```yaoxiang
gcd: (a: Int, b: NonNegative(b)) -> Int = {
    if b == 0 { return a }
    return gcd(b, a % b)  // The compiler explores: the well-founded ordinal measure of (b, a % b) decreases → termination
}
```

Formal parameter decrease is the strongest automatic path (structural recursion). When exploration
fails, the programmer gives the measure explicitly at the type position (§6.9).

#### 6.8 Hard boundary

`i = f(i)` where `f` is non-invertible, non-closed, and preserves no monotonicity — mathematically,
termination cannot be automatically proven. Compile error:

> This loop cannot be automatically proven to terminate. The loop variable depends on the
> unanalyzable function `f`. Please use an iterative pattern that the compiler can analyze, or bind
> a name to this loop and provide a measure at the type position (§6.9).

This is not a compiler failure. Code that cannot be statically proven safe must not pass
compilation. Even when an explicit measure is given, it must be judged true by the SMT to pass — a
wrongly written measure is rejected by counterexample; humans can only fail to prove, never prove
wrong.

#### 6.9 Explicit measure: `Terminates`

When automatic exploration fails to find a measure, the programmer writes the measure at the **type
position** — the same mechanism as `Positive(b)` and `IsMax(T, arr, result)` (predicate
application), with zero new syntax:

```yaoxiang
// Measure: an ordinary function, unit-testable, reusable, runtime-invisible
gcd_measure: (a: Int, b: Int) -> Int = { b }

// The termination component lives on the function's own type position
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// Loop: the binding name is the anchor; the measure is given variables in the binding's scope
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n {
        i = i + 1
        i
    }
    return acc
}
```

The rationale for the loop here: `Terminates(m)` refines the value type of the loop body's tail
expression; the anchor is provided by the binding name `acc` — so the loop is denotable, and there
is no longer a "dead corner where anonymous constructs have no denotation".

> **The body tail must yield a value**: the value of a loop = the value of the loop body block
> (§2.9), and the value of a block = the tail expression. If the body tail is an assignment
> statement (e.g. `{ i = i + 1 }`), the block's value is `Void`, and `return acc` would return
> `Void` from `-> Int`, which fails the type. So this example's body tail explicitly yields `i`.
> This does not change the measure's semantics — the measure `n - i` is a **state expression**,
> independent of the loop's value type.

> **Implementation note (recorded 2026-10-02; updated 2026-10-04)**: this section's loop example is
> **type-side** sound (0 errors from `check`, back-edge obligations Proved). The **runtime
> value-fetch defect (#409) has been fixed**: `while` as a value now takes the value of the loop
> body's tail expression on the last iteration, matching the type-side `block_value_ty` —
> `loop(4) == 4`, `countdown(3) == 30` verified at runtime (same fix as `for`).
>
> **Known residual (zero iterations)**: when the loop body is never evaluated, there is no "value on
> the last iteration", and the loop value is still `Void` — binding to a non-`Void` type position
> and using it at runtime yields
> `E6007 Runtime error: type mismatch in comparison Eq: Void vs Int(0)` (loud type mismatch, not a
> silent wrong value). This is a **static-approximation boundary**: the `block_value_ty` judgment
> gives the type of the tail expression, but has no way to determine "whether the loop executes at
> least once" (that requires data flow / provability analysis), so the type side does not silently
> downgrade (downgrading would break the value type that this section's `Terminates` loop binding
> relies on). This residual is pinned by the corpus
> `tests/yaoxiang/02-type-system/termination/while_zero_iteration_void_err.yx`
> (`// expect: runtime-error E6007`). **This residual is a deliberately retained design boundary**
> (decided 2026-10-04, not fixed for now), so issue
> [#409](https://github.com/ChenXu233/YaoXiang/issues/409) is closed along with the main-form fix,
> and no separate tracker is opened. The minimum reproduction and impact-surface history are
> recorded under [RFC-027a](../accepted/027a-termination-explicit-measure.md) §Example Loop Section
> "Known Defect D6". Additionally: termination Strategy 1 (linear rank function) is re-enabled in
> this round, so the order in this section — "write an explicit measure only after automatic
> exploration fails" — is unchanged — explicit measures are still the fallback after exploration
> fails.

**`Terminates` is a built-in predicate**, on the same level as `Int` and `Never` among the core
primitives. It is the only predicate **whose body is written by the compiler** — its assertion ("the
measure strictly decreases at every recursive call site / loop back-edge") lives in the
computational structure, and user-written predicates cannot reference function bodies or loop
bodies, so it cannot be expressed by the predicate-definition syntax in the "Syntax" section. The
built-in surface converges on this single name.

**Arity**. `Terminates(FnType, m)` and `Terminates(m)` are two arities of the same predicate, not
two constructs:

| Form                    | Anchor                 | Usage                                                       |
| ----------------------- | ---------------------- | ----------------------------------------------------------- |
| `Terminates(m)`         | Name of the binding    | Self-recursive functions, loops — the default form          |
| `Terminates(FnType, m)` | Explicit function type | Mutually recursive scenarios where the anchor is not unique |

Both are essentially the same: the termination obligation always falls on "the computation annotated
by the type position where the refinement sits".

**Measures do not restrict the return type.** A measure can be an expression of any type (not
required to be a natural number); "strictly decreasing" is given by the well-founded order available
on that type. Whether the measure is well-founded (e.g. whether it is `>= 0` when returning `Int`)
is an **independent obligation**, also submitted to refinement inference or SMT just like the
decrease obligation; when neither can be inferred, the diagnosis **does not directly reject**, but
suggests a direction to check (whether the measure's lower bound holds, whether the recursive
argument is actually moving in that direction).

**Relationship with automatic exploration**: the explicit measure is not a separate pipeline, but
input after exploration fails. After the measure is given, the same SMT verifies decrease and
well-foundedness; if it does not hold, an error is reported along with a counterexample.

The obligation generation, decision pipeline, diagnostic direction, measure sharing for mutual
recursion (SCC), and other implementation mechanisms are detailed in
[RFC-027a: Explicit Measure for Termination Checking](../accepted/027a-termination-explicit-measure.md).

### 8. The SMT solver: an acceleration module of the type checker

In traditional languages, the SMT solver is an external tool (e.g. F\* calls Z3, Dafny calls Z3). In
YaoXiang, it is an **acceleration module of the type checker** — invoked only when the compiler
kernel cannot judge on its own. SMT helps find proofs, but the one that verifies the proofs is the
type checker.

**Trust model**: the type checker is the sole root of trust. The SMT solver is an acceleration
module — it helps find proofs, but SMT is not a separate trust boundary. The compiler trusts Z3's
`unsat` results (consistent with the F\*/Dafny line — the probability of Z3 error is lower than that
of compiler bugs, which is a pragmatic engineering choice). The real unreliability is controlled at
the SMT translation layer — if there is a bug in the translation, the compiler will be exposed in
other tests.

**Interface**: the compiler internally translates to SMT-LIB 2.6 standard format rather than binding
to a specific solver API. SMT-LIB is an ISO standard, natively supported by Z3, CVC5, MathSAT,
Yices.

**Default backend**: Z3 (MIT license, with the broadest documentation and community validation).
CVC5 as an SMT-LIB-compatible alternative — users can switch at compile time via a compiler flag.

No "generic solver abstraction layer" — SMT-LIB is the abstraction layer. In the future, if CVC5
makes a breakthrough in a particular theory, switching only requires changing the binary, not the
compiler code.

```
Compile-time Bool expression
        │
        ├── The compiler kernel can judge directly (structural equivalence, simple arithmetic,
        │   trivial formulas after constant folding)
        │   → Return Proved / Disproved directly
        │
        └── The compiler kernel cannot judge directly (quantifiers, symbolic variables)
            → Dependent-type pre-reduction (factorial(5) → 120)
            → Translate to SMT-LIB format
            → Send to Z3/CVC5 (with budget limit)
            → Return value: unsat → Proved  │  sat + model → Disproved  │  unknown → Unproven
```

**Solver budget — hard limit, like stack depth**:

| Budget dimension               | Default | Description                                                                                                                                                               |
| ------------------------------ | ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Solving steps                  | 10,000  | Z3 usually finishes linear arithmetic within a hundred steps. 10,000 steps cover 99% of practical predicates.                                                             |
| Time                           | 100ms   | A single predicate over 100ms means the user is writing a compile-time program rather than a type annotation. 100ms × 50 predicates = 5 seconds compile-time upper bound. |
| Quantifier instantiation depth | 3       | Three nested quantifier levels cover practical patterns. Beyond three levels, the user is likely writing logic exercises.                                                 |

Exceeding the budget returns `Unproven`, with a compile error + predicate location + consumption
amount. No degradation, no runtime check, no silent pass.

**Why this is practical**: 95% of practical predicates in engineering are linear arithmetic —
`x > 0`, `arr.len > 0`, `0 <= idx < arr.len` — all within the decidable fragment, and the SMT solver
returns in milliseconds for such problems. For the rare complex predicate that exceeds the budget,
the programmer can write a proof function.

Dependent types go through a pre-reduction before the SMT call: `factorial(5)` is directly evaluated
at compile time to `120`; `append([1,2], [3])` is directly evaluated to `[1,2,3]`. These
deterministic value calculations do not consume SMT budget.

The programmer does not need to know SMT exists. The mental model is: **the compiler can prove it,
then it passes; if it cannot, it reports an error — if the compiler cannot, you can write a function
to prove it to the compiler.**

### 9. Composition of compile-time predicates

Compile-time predicates are functions returning `Type`, and composition is naturally achieved
through function composition:

```yaoxiang
SortedNonEmpty: (T: Ord, arr: Array(T)) -> Type = {
    Sorted(T, arr) and NonEmpty(arr)
}
```

### 10. Code examples

#### 9.1 Safe division

```yaoxiang
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b

result = divide(10, 2)   # ✅ The compiler verifies Positive(2) = { 2 > 0 } → True
# result = divide(10, 0)  # ❌ The compiler verifies Positive(0) = { 0 > 0 } → False
```

#### 9.2 Safe array access

```yaoxiang
InBounds: (idx: Int, arr: Array(T)) -> Type = { 0 <= idx and idx < arr.len }

get: (arr: Array(T), idx: InBounds(idx, arr)) -> T = arr.data[idx]

arr = Array(Int)(1, 2, 3)
x = get(arr, 1)   # ✅ The compiler verifies InBounds(1, arr) = { 0 <= 1 and 1 < 3 } → True
# y = get(arr, 5)  # ❌ The compiler verifies InBounds(5, arr) = { 0 <= 5 and 5 < 3 } → False
```

#### 9.3 Sort correctness

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

#### 9.4 Loop: compiler VC generation

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

### 11. dispatch pipeline: unified compile-time and runtime dispatch

> **Implementation note (2026-10-03, #377-2)**: the **semantics** of this section is sound and wired
> into production, but the **structure** is not a standalone `layers/dispatch.rs` module. The
> production dispatch is implemented by three **call-site obligations** in `checker.rs` (directly
> calling `layers::predicate::check_predicate`): binding-site re-validation (`revalidate_refined`),
> call-site argument (`check_call_arg_refinements`), and return-site post-condition
> (`check_return_refinement`). The original `layers/dispatch.rs` had zero production call sites, and
> its handling of `Unproven { ProofFunctionRequired }` (downgrade to W1080 warning and inject into
> Γ) conflicted with the production implementation (`Unproven` always errors, see
> `refined_unproven`), so it was deleted in its entirety. Also note: the table's Runtime row "insert
> runtime check" has **no** emission site in production yet (`RuntimeOutcome::InsertCheck` has no
> consumer), and Γ injection is independently handled by `ownership.rs`'s branch guards and
> `inference/expressions.rs`.

`assert` and `Assert` are two sides of the same refinement type primitive. The dispatch pipeline
`dispatch` decides automatically between compile-time proof and runtime check based on **whether the
predicate's free variables are reachable at compile time**:

| Criterion                                                                                  | Mode            | Behavior                                                                                     |
| ------------------------------------------------------------------------------------------ | --------------- | -------------------------------------------------------------------------------------------- |
| All free variables are known at compile time (generics, compile-time constants)            | **CompileTime** | Enter the proof pipeline: Proved → erase, Disproved → compile error, Unknown → require proof |
| Some free variables come from runtime (function parameters, external input, mut variables) | **Runtime**     | Insert a runtime check, and inject refinement facts into the flow-sensitive assumption set Γ |

**Key point**: "cannot decide" ≠ "false". In CompileTime mode, `Unknown` requires a proof (no silent
downgrade); in Runtime mode, the proposition is not even true or false at compile time — no prover,
however strong, can write a tautological proof for "the user might have entered a negative number",
so runtime checks are the only sound choice. This is not because the prover is too weak, but a
theoretical necessity.

### 12. Flow-sensitive assumption set Γ: strongest post-condition propagation

The compiler maintains a flow-sensitive assumption set Γ, tracking the propositions known to hold at
each control-flow point.

**SP (strongest post-condition) propagation**:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
```

**Kill set for mut variables**: after a `mut` variable is reassigned, all assumptions involving that
variable are removed from Γ:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
mut x = x - 5       // Γ = {}  ← x > 0 is killed
```

This is a hard requirement of soundness — once the variable's value changes, the old assumption is
invalid.

**Branch merging**: when an `IF/ELSE` or `match` branch is merged, Γ takes the intersection of each
branch's assumptions. Only propositions that hold on all paths are carried out of the branch.

### 13. Erasure model clarification: witness erasure ≠ check erasure

RFC-027's claim that "refinement types are **completely erased** at runtime" refers to the **proof
witness** (proof token) — proof terms already verified at compile time produce no runtime code. But
the **runtime checks** inserted by dispatch in Runtime mode are retained — they are Bool checks
executed at the value level, not type-level witnesses.

Summary: witnesses are erased, checks are retained. The two do not conflict, and RFC-027's original
claim stands.

## Detailed design

### Syntax changes

| Before (RFC-022)                      | After (this RFC)                                                                                                                                                   |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `//! requires: NonEmpty(n) = n > 0`   | Compile-time predicate as a parameter type `(b: Positive(b))`                                                                                                      |
| `//! ensures: ExistsMax(result, arr)` | Return type uses a return value parameter `-> (result: IsMax(T, arr, result))`                                                                                     |
| `/*! invariant: ... !*/`              | Compile-time predicate type annotation on variables — Floyd-Hoare invariants                                                                                       |
| `//! decreases: n`                    | Measure written at the refinement type position (`Terminates`); the compiler first explores automatically, only requiring an explicit measure if exploration fails |
| Specifications are comments           | Specifications are the type system                                                                                                                                 |

### Syntax

**No new keyword for compile-time predicates.** `{}` is the proof space, fully consistent with the
existing type definition syntax. A compile-time predicate is simply a function returning `Type` —
`name: (params) -> Type = { assertions }`. Usage is a function call — `Positive(b)`,
`IsMax(T, arr, result)`.

```bnf
# Compile-time predicate = a function returning Type, with compiler-verified assertions in {}
# Uses existing function/type syntax; no new BNF rule needed
predicate ::= identifier ':' params '->' 'Type' '=' '{' assertions '}'
```

**Predicate application arguments must be compile-time-constant forms** — literals, variables (bound
by name), type applications (recursively extracted), or **compile-time-denotable function
references** (function names). An argument whose form is not convertible to a constant expression
reports **E1092**; an argument count that does not match the predicate's formal parameter count
reports **E1093**. Arguments are bound positionally to the formal parameter list; predicate arity is
determined by the declaration — `Positive(x)` is unary, `IsMax(T, arr, result)` is ternary,
`Terminates(m)` and `Terminates(FnType, m)` are unary and binary — refinement constraints are
**never silently discarded** (previously, non-convertible arguments would cause constraints to
vanish silently, and bindings that violated the constraint would pass silently).

**New syntax concept: return value parameter** — in `-> (name: Type)`, `name` is the return value
parameter.

The return value parameter is the **only syntax concept** YaoXiang introduces on top of the existing
function syntax. Its semantics:

- The value of `name` is provided by the `return` statement
- `name` exists only in the type signature and is referenced by post-condition predicates (e.g.
  `-> (result: IsMax(T, arr, result))`)
- `name` does not enter the function body scope and does not appear at the caller
- The return value parameter is **optional** — when there is no post-condition, the signature is
  identical to a normal function (`-> Int`), introducing no extra burden

The reason for introducing it: post-conditions need to reference "the value the function is about to
return". Without a return value parameter, the compiler can only let predicates reference the return
value through special rules (such as implicit variable `$result` or `__retval__`). The return value
parameter makes this reference explicit — it is simply a parameter, except its value is provided by
`return` rather than the caller.

**Proof function** is not a new concept — it is simply a YaoXiang function whose return type is the
asserted proposition. When the compiler returns `Unproven`, the programmer provides a proof
function, and the type checker verifies it in exactly the same way as it verifies any function's
return type. No new syntax, no new keyword, no new rules.

> **The boundary between the two.** The correctness domain (predicate `Unproven`) supplements the
> proof **in the body**: write a function whose return type is the proposition to be proven. The
> termination domain (§6.9) supplements the measure **at the type position**: write
> `Terminates(measure)` as the type of the binding or function signature. The former is "write a
> proof for a proposition you cannot prove"; the latter is "declare explicitly a measure you cannot
> explore" — same mechanism (both are refinement type applications), different landing points. The
> termination domain does **not** require writing a `_proof` function.

### Type system impact

- **Type universe**: compile-time predicates live at the Type₂ layer — functions taking values and
  returning Type, at the same level as type constructors
- **Generics interaction**: compile-time predicates may take generic parameters, e.g.
  `NonEmpty: (T: Type) -> (arr: Array(T)) -> Type`
- **Ownership interaction**: expressions in compile-time predicates follow the ownership rules and
  may only read, not write
- **Type inference**: parameters of compile-time predicates participate in HM type inference

### Runtime representation

Compile-time predicates are **handled at runtime according to the dispatch result**:

- **CompileTime mode** (all free variables known at compile time): the witness is completely erased
  after the proof passes. `Positive: (x: Int) -> Type = { x > 0 }` — the parameter
  `b: Positive(5)`'s runtime representation is just `Int`. The refinement condition `{ 5 > 0 }` has
  passed; it is erased.
- **Runtime mode** (some free variables come from runtime): a runtime check is retained — a Bool
  check executed at the value level, with the result injected into the flow-sensitive assumption set
  Γ. See §11 dispatch pipeline and §13 erasure model clarification for details.

Placing a compile-time predicate at a type position (e.g. `f(x: Positive(x))`) produces no wrapper
type, no extra memory allocation. But when `x` comes from runtime input, a **runtime Bool check is
inserted**.

**Interaction constraint with `ref`**: a compile-time predicate can only reference values that are
immutably borrowed or whose ownership has been transferred. A compile-time predicate referencing a
mutably borrowed value cannot guarantee at compile time that the verification result still holds at
runtime — such usage directly reports a compile error.

### Compiler changes

1. **Parser**: compile-time predicates use standard function syntax and need no additional parsing
   rules
2. **Compile-time proof pipeline**: unified `Proved/Disproved/Unproven` return interface, with
   automatic strategy selection
3. **SMT acceleration module**: SMT-LIB 2.6 translation layer, default backend Z3, CVC5 as
   alternative
4. **Type checker kernel**: implementation of inference rules — structural equivalence,
   βδι-reduction, universal quantifier introduction/elimination. This is the sole root of trust;
   both SMT and programmer proofs are verified through it
5. **Verification condition generation**: WP/SP calculus + loop invariant proof obligations
6. **Error reporting**: counterexample formatting + unsolved-proposition report + source location
   association

### Backward compatibility

- ✅ Code that does not use compile-time predicates is completely unchanged
- ✅ Compile-time predicates incur zero runtime overhead in CompileTime mode; in Runtime mode, only
  the necessary Bool checks are retained
- ⚠️ RFC-022's `//!` syntax is no longer supported — but 022 was never implemented, so there is no
  migration burden

## Trade-offs

### Advantages

- **The Curry-Howard isomorphism is fully realized**: types are propositions, programs are proofs,
  `name: Proposition = Proof`
- **Unification**: compile-time predicates and ordinary functions use exactly the same syntax, with
  no conceptual split
- **SMT transparency**: the programmer does not need to know SMT exists, the mental model matches
  type checking
- **Progressive adoption**: start with one compile-time predicate and gradually expand coverage
- **Minimal runtime overhead**: zero overhead in CompileTime mode, only the necessary Bool checks in
  Runtime mode

### Disadvantages

- **Compile time**: SMT solving increases compile time, but the hard budget limit keeps the upper
  bound controllable
- **Automatic proof boundary**: complex predicates beyond first-order linear arithmetic may require
  the programmer to write a proof function. This is not a language defect — it is the necessary
  conclusion of the halting problem. The compiler honestly reports `Unproven` rather than falsely
  reporting True/False
- **Learning curve**: writing effective compile-time predicates and proof functions requires
  understanding the basic intuition of the Curry-Howard isomorphism
- **Implementation complexity**: unifying the compile-time proof pipeline requires careful design

### Risk mitigation

- Hard SMT solver budget limit (10,000 steps / 100ms / instantiation depth 3), exceeding the budget
  returns `Unproven`
- Dependent-type pre-reduction: deterministic value calculations are consumed first; SMT only chews
  the non-deterministic part
- `Unproven` is not a dead end: correctness domain writes proof functions (return type is the
  proposition), termination domain gives the measure at the type position (§6.9) — both are verified
  by the type checker
- Incremental verification: only changed modules are verified
- Clear error messages + counterexample display + budget consumption report + unsolved proposition +
  suggestions (if the compiler can give them)

## Alternatives

| Option                                                        | Why not                                                                                                                                                                           |
| ------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| RFC-022: `//!` comment-style specifications                   | Specifications and types are split, violating the Curry-Howard isomorphism                                                                                                        |
| Standalone specification files (e.g. CVL)                     | Specifications separated from code, increasing maintenance cost                                                                                                                   |
| Runtime-only assertions                                       | Cannot statically guarantee correctness                                                                                                                                           |
| External proof assistants (e.g. Coq)                          | Disconnected from the compiler, requiring a separate proof language and trust boundary. YaoXiang's choice: the proof is YaoXiang code; the type checker is the sole root of trust |
| **This RFC: compile-time predicates as first-class citizens** | ✅                                                                                                                                                                                |

## Implementation strategy

### Phases

| Phase       | Content                                                                                                                                                                |
| ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Phase 1** | Compiler kernel: structural equivalence + βδι-reduction + universal quantifier introduction/elimination. Support simple arithmetic predicates (`x > 0`, `arr.len > 0`) |
| **Phase 2** | SMT-LIB translation layer + Z3/CVC5 integration. Pipeline returns `Proved/Disproved/Unproven`. Support programmer-written proof functions on `Unproven`                |
| **Phase 3** | Loop invariant VC generation + termination check (measure exploration four strategies + explicit `Terminates` measure, §6, §6.9)                                       |
| **Phase 4** | Incremental verification + caching + IDE support                                                                                                                       |

### Dependencies

- RFC-010: Unified Type Syntax — compile-time predicates are based on `name: type = value`
- RFC-011: Generics System — compile-time predicates may take generic parameters
- RFC-009: Ownership Model — expressions in compile-time predicates follow the ownership rules

## Open questions

- [x] **Z3 availability for the wasm32 target**: no Z3 on wasm (all SMT code is excluded by
      `cfg(not(target_arch = "wasm32"))`). Impact scope and degradation direction: - `ownership.rs`
      `smt_cut` always returns `false` → back-edge passes through → **conservative rejection**
      (sound direction, just less narrowing) - Termination check Strategy 1 and predicate level 2b/3
      are skipped entirely

  **Correction (2026-10-01)**: the original text stated that these paths are "currently
  **unreachable**" (on the grounds that `predicate_defs` was never populated in production, the
  `parser` does not produce `MonoType::Refined`, and `with_solver` is never called in production),
  which is no longer true — all three have landed (predicate definition registration #377-3;
  refinement type normalization; solver injection in the production pipeline). So the difference
  between wasm and native is no longer limited to the precision of `smt_cut`:

  | Path                                        | native   | wasm       | Degradation direction             |
  | ------------------------------------------- | -------- | ---------- | --------------------------------- |
  | Termination check explicit measure judgment | Judgment | `Unjudged` | Don't report E4022 (under-report) |
  | Well-foundedness judgment                   | Judgment | `Unjudged` | Don't report E4022 (under-report) |
  | Refinement predicate SMT implication level  | Executed | Skipped    | Falls back to `Unproven`          |

  All are in the **conservative** direction (less narrowing, less reporting), with no impact on
  soundness. For accuracy parity, see the plan and cost in issue #376 (conclusion: prefer a JS-side
  Z3 instance rather than linking Z3 into the main wasm — the latter requires switching to the emcc
  build system and the artifact grows from 3 MB to ~20 MB).

- [x] **SMT solver choice**: default Z3 (MIT license, most widely validated). CVC5 as an
      SMT-LIB-compatible alternative, switchable via a compiler flag. The compiler internally
      translates to the SMT-LIB 2.6 standard format — SMT-LIB is the abstraction layer, no custom
      generic solver interface.
- [x] **Specific values of the solver budget**: 10,000 steps / 100ms / quantifier instantiation
      depth 3. Fixed in the compiler, no knob. In practice, if there are real use cases proving
      these are insufficient (not "the user wrote it wrong"), they will be adjusted.
- [x] **Quantifier support scope**: the language does not restrict quantifier rank. Compile-time
      predicates accept Type parameters — Type includes function types — so higher-order quantifiers
      are a natural consequence of the type system, requiring no special syntax. The SMT solver can
      automatically decide first-order quantifiers (forall/exists, supporting interleaved nesting,
      limited by the budget depth of 3). For higher-order quantifiers: SMT returns `Unproven`, and
      the compiler suggests "this predicate is beyond the automatic proof range, please supply a
      proof function". The programmer writes a YaoXiang function whose return type equals the
      proposition — the type checker verifies this function. No external export, no AI, no
      interactive proof mode. Everything is YaoXiang code, and everything is verified by the type
      checker.
- [x] **Counterexample formatting**: source variable names are used directly as SMT variable names
      (with module prefixes to avoid conflicts). When the Z3 model returns, look up by variable
      name. Output format: variable name = concrete value + source location + predicate definition
      location. No complex mapping layer.
- [x] ~~**Interaction of compile-time predicates with `ref` smart pointers?**~~ → Decided:
      compile-time predicates only allow immutably borrowed values or values whose ownership has
      been transferred. Mutably borrowed values cannot appear in compile-time predicates.
- [x] **Extension of the forall predicate violation-count measure to non-adjacent operations?** →
      Not extended. The current coverage (adjacent swap, adjacent move) is complemented by Strategy
      1 (linear rank function) — quicksort's outer interval contraction is covered by Strategy 1,
      and heapsort is covered by Strategy 1 (array-index pattern). Loops that cannot be proven
      terminating by any strategy are directly reported as errors by the compiler — this is
      hard-safety philosophy, not a defect. If in the future there are real (non-academic)
      algorithms that none of the four strategies can cover, the topic will be revisited. →
      **Revisit triggered (2026-09-14, #318)**: non-structural recursion (gcd-style non-direct
      decrease, mutual recursion, merge partition) are exactly the real scenarios envisioned by this
      clause — the template sequence of measure exploration cannot automatically handle them, and
      they cannot be rewritten into analyzable iterative patterns without sacrificing readability.
      Conclusion: hard-safety philosophy is unchanged (a wrongly written measure is still rejected
      by SMT counterexample; humans can only fail to prove, never prove wrong), but "exploration
      fails → reject" is relaxed to "exploration fails → the programmer can give the measure
      explicitly at the type position" — see §6.9.
- [x] **Combinatorial explosion in linear rank function enumeration**: the candidate enumeration
      upper bound is 3 bounded variables. When ≤ 3, enumerate all linear combinations and verify
      each with SMT. When > 3, only try single-variable measures (`v_i`, `u_i - v_i`); if it fails,
      directly report a compile error — telling the programmer "the loop has > 3 bounded variables,
      the compiler cannot automatically synthesize a multi-variable measure". This is not an
      engineering compromise — it forces the programmer to write simpler loops.

## References

- [RFC-010: Unified Type Syntax](010-unified-type-syntax.md)
- [RFC-011: Generics System Design](011-generic-type-system.md)
- [RFC-009: Ownership Model](009-ownership-model.md)
- Howard, W. A. (1969). The Formulae-as-Types Notion of Construction.
- Swamy, N. et al. (2016). Dependent Types and Multi-Monadic Effects in F\*. _POPL 2016_.
- Vazou, N. et al. (2014). Refinement Types for Haskell. _ICFP 2014_.
- Leino, K. R. M. (2010). Dafny: An Automatic Program Verifier for Functional Correctness. _LPAR
  2010_.
- De Moura, L. & Bjørner, N. (2008). Z3: An Efficient SMT Solver. _TACAS 2008_.

---

## Lifecycle and destination

```
┌─────────────┐
│   Draft     │  ← Author creates
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Under      │  ← Current status: community discussion
│  Review     │
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
