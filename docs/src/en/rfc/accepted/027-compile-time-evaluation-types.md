---
title: 'RFC-027: Compile-Time Predicates and Unified Static Verification'
status: 'Accepted'
author: 'Chenxu'
created: '2026-06-07'
updated: '2026-10-03'
impl_status: 'in-progress'
impl_detail:
  "Phase 1-2 complete, Phase 3 partially complete, Phase 4 partially complete. The assert/Assert
  unified 6-Phase plan is fully implemented (issues #157-#162 closed): Never type, IsTrue bridge,
  flow-sensitive Γ + kill set, type-level recursion, universe stratification weak check. The
  **semantics** of §11's dispatch has been implemented via call-site obligations (`checker.rs` calls
  `check_predicate` directly in three places: binding position / call-site argument / return
  position), but the **standalone `layers/dispatch.rs` module has been removed**—it had zero
  production call sites, and its handling of `Unproven` (downgrading to a W1080 warning) conflicts
  with the production behavior (always an error) (#377-2)."
impl_percent: 82
issue_number: 90
issue_url: 'https://github.com/ChenXu233/YaoXiang/issues/90'

issue: '#90'
---

# RFC-027: Compile-Time Predicates and Unified Static Verification

> **References**:
>
> - [RFC-009: Ownership Model](009-ownership-model.md)
> - [RFC-010: Unified Type Syntax - name: type = value Model](010-unified-type-syntax.md)
> - [RFC-011: Generic Type System Design](011-generic-type-system.md)
> - [RFC-024: Concurrency Model Based on spawn Blocks](024-concurrency-model.md)
>
> **Supersedes**:
> [RFC-022: Hoare Logic Static Verification Support (Specification Comments and Specification Types)](../deprecated/022-hoare-logic-static-verification.md)
> — deprecated

## Summary

This document proposes introducing **compile-time predicates** as first-class citizens in YaoXiang,
unifying all compile-time static verification into a single **proof pipeline**. Compile-time
predicates are not external specification comments—they _are_ functions. A function returning `Type`
can be used in a type position, and the compiler invokes it at compile time and checks the return
value. Types are propositions; compile-time evaluation is proof.

**Core argument**: The only job of type checking at compile time is to construct and verify proof
terms. Type equality, token conflicts, dependent type reduction, compile-time predicate evaluation,
and Hoare-logic entailment are all different type checks in the compile-time proof pipeline, sharing
the same pipeline. The SMT solver is an acceleration module of the type checker, not an independent
trust boundary. When the compiler returns `Unproven`, the programmer writes a YaoXiang function as a
proof—the type checker verifies it in exactly the same way it verifies any function's return type.
Everything is YaoXiang code; everything is verified by the type checker.

## Motivation

### Why deprecate RFC-022?

RFC-022 designed specifications as `//!` comments:

```yaoxiang
max: (T: Ord) -> ((arr: Array(T, n)) -> T) = {
    //! requires: NonEmpty(n) = n > 0          ← This is a comment independent of the type
    //! ensures: ExistsMax(result, arr[0..n])   ← This is a comment independent of the type
}
```

This commits a fundamental error of the Curry-Howard correspondence: **splitting specifications and
types into two layers**. Comments are not types. Comments do not participate in type checking.
Comments are the mental model of "external tools."

The white paper states clearly:

> "No `//!` comments. No separate specification language. Everything is within the type system."

### Current Problems

- RFC-022's `//!` comments are external syntax independent of the type system
- Specification types and ordinary types are two separate systems, causing conceptual redundancy
- The split between Debug Build verification / Release Build ignoring breaks unity
- The SMT solver is positioned as an external tool in conventional cognition—YaoXiang incorporates
  it as an acceleration module of the type checker
- Type checking, borrow checking, compile-time predicate checking, and macro expansion each follow
  different paths

### The Correct Mental Model

Type checking can be abstracted as a function:

```
verify : Program → Proved | Disproved(Model) | Unproven
```

All compile-time checks—simple type matching, borrow conflict detection, compile-time predicate
verification—are sub-tasks of this function. They share the same proof pipeline; the only difference
lies in the complexity of proof terms and the construction strategy.

When the compiler returns `Unproven`, the programmer provides a proof function—the function's return
type equals the proposition to be proved. The type checker verifies it. This is the same operation
as ordinary type checking.

## Proposal

### 1. `{}` Is the Proof Space: Types Are Assertions, Verification Is Type Checking

YaoXiang's `{}` is the compile-time proof space. Everything inside is an assertion, and the compiler
guarantees each item is `True`—either proven automatically or by a proof function provided by the
programmer.

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
#          parameter at        only assertions inside {}
#          signature           compiler verifies x > 0 when called at compile time

List: (T: Type) -> Type = { data: Array(T) }
#      ^^^^^^^^              ^^^^^^^^^^^^^^^
#      parameter at           compiler verifies type_of(T) == Type, type_of(data) == Array(T)
#      signature
```

The same pattern: `name: (params) -> Type = { assertions }`. The compiler does not distinguish
between "type assertions" and "value assertions"—they are all evaluation targets in the proof
pipeline.

**Loop invariants do not need to be written separately. Type annotations on variables _are_
Floyd-Hoare invariants.**

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # annotation references i—tells the compiler that s's type depends on i
    mut i: UpTo(arr.len) = 0     # at initialization i=0, verify: 0 == sum(arr[0..0]) → True
    while i < arr.len {
        s += arr[i]  # compiler verifies: s_new == sum(arr[0..i+1])
        i += 1       # i changes → triggers s's dependency re-verification: s satisfies SumUpTo(arr, i_new)
    }
    return s  # s: SumUpTo(arr, arr.len) = sum(arr[0..arr.len])
}
```

The compiler generates one verification condition for the loop body—induction hypothesis (type
annotation) → assignment operation → does the new value satisfy the type annotation. Once the proof
pipeline verifies the induction step holds, all iterations are automatically covered. No
`: decreases`, no `: Invariant`, no induction proof needed—the compiler decomposes induction into
local VCs for each assignment.

### 2. Pre/Post-Conditions: Compile-Time Predicates on Parameter Types and Return Types

Drop RFC-022's `//! requires`/`//! ensures`. Compile-time predicates appear as type annotations on
parameters or returns.

**Parameter side is a function call.** A compile-time predicate is a function returning `Type`; its
use on the parameter side is calling it—just like `factorial(5)`. The return side introduces a new
concept: the return-value formal parameter.

```yaoxiang
# Precondition: explicitly invoke a compile-time predicate in the parameter type
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b
#                       ^^^^^^^^^^  b is the current formal parameter name, passed to Positive as argument
#                       compiler extracts the actual argument value at the call site, substitutes into b, verifies Positive(actual)
#                       example: divide(10, 2) → verify Positive(2) = { 2 > 0 } → True
#                       example: divide(10, 0) → verify Positive(0) = { 0 > 0 } → False → compilation error

# Postcondition: return-value formal parameter + compile-time predicate
IsMax: (T: Ord, arr: Array(T), result: T) -> Type = {
    forall j in 0..arr.len: result >= arr[j]
}

NonEmpty: (arr: Array(T)) -> Type = { arr.len > 0 }

max: (T: Ord) -> ((arr: NonEmpty(arr))) -> (result: IsMax(T, arr, result)) = {
#                                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
#                                            result is the return-value formal parameter, value provided by return
#                                            compiler substitutes the return value at the return point, verifies the postcondition
    candidate = arr[0]
    for i in 1..arr.len {
        if arr[i] > candidate { candidate = arr[i] }
    }
    return candidate
}
```

**Key rules**:

- **Parameter side**: `b: Positive(b)` — `b` is the current formal parameter name, passed to
  `Positive` as argument. Function call syntax, zero implicit.
- **Return side**: `-> (result: IsMax(T, arr, result))` — `result` is the return-value formal
  parameter, value provided by the `return` statement. `result` exists only in the type signature,
  is only referenced by the predicate, does not enter the function body scope, does not appear on
  the caller side.
- **Return-value formal parameter is optional**: when there is no postcondition, the signature is
  identical to an ordinary function (`-> Int`).
- **Unity**: parameter and return-value formal parameters are the same
  concept—`name: predicate_call(argument)`—the only difference is whether the value is provided by
  the caller or by `return`.

> **Implementation Note (2026-10-02)**: Recognition of return-position formal parameters is
> **strictly by declared name**—`r` in `-> (r: P(...))` is the unique return formal parameter. Free
> variables in the constraint that are **not in scope** are no longer substituted with the return
> value; instead, an undefined identifier error is reported (tested:
> `g: () -> (r: Eq2(m, r)) = { return 7 }` (where `m` is undefined) →
> `error [E1001] Unknown variable: 'm'`). The arguments of the predicate call are truly substituted
> into the constraint **by argument**: `P(r + 100)` after substitution becomes "return value + 100",
> not "return value".
>
> **Known Gap (2026-10-02)**: **Multiple-parameter predicates at return position with symbolic
> arguments are not yet supported**. Triggering form:
> `f: (b: Int) -> (r: SumUpTo(b, r)) = { return b * 2 }` (where
> `SumUpTo: (n: Int, s: Int) -> Type = { s == n * 2 }`) →
> `error [E2031] … the refinement type constraint of 'r': SumUpTo(b, (b * 2)) cannot be statically proven`.
> Workaround: change the argument to a compile-time foldable literal (such as `SumUpTo(3, r)`), or
> provide a proof function returning `Type` for that predicate; unblocking condition: the proof
> kernel supports argument substitution for "multi-parameter predicate + symbolic argument". See
> [RFC-027a](../review/027a-termination-explicit-measure.md) §Implementation Records (2026-10-02,
> this round of hardening) for details.

### 3. Path Condition Propagation: Compile-Time Verification of Runtime Values

When compile-time predicates are used at binding positions, arguments are passed in explicitly by
the programmer. When runtime values enter refinement type arguments, the compiler completes
verification through path condition collection and SMT entailment—no need for the programmer to
explicitly pass a proof.

#### 3.1 Explicit Function Calls

When a compile-time predicate is used at a binding position, the arguments are passed in explicitly
by the programmer—it is simply a function call, with zero implicitness.

`Positive: (x: Int) -> Type = { x > 0 }` is a compile-time predicate constructor. When it appears in
a binding position (parameter declaration, variable declaration, return type), the programmer
explicitly passes in the already-bound variable name:

```yaoxiang
b: Positive(b)
// b is already declared as the current formal parameter, Positive(b) is a function call
// after normalization: b: { b > 0 }
```

No need for the compiler to implicitly fill in arguments—`b: Positive(b)` is the same as `f(5)`,
just a function call. `b` is bound as the parameter name, and its type annotation `Positive(b)`
references `b` itself—this is the standard pattern of dependent types, not an implicit expansion
rule.

**Unity with RFC-010's `self`**: RFC-010 establishes that `self` is not a keyword, just a
conventional name for a parameter ("writing it as `p`, `this`, or `x` has exactly the same effect").
`b: Positive(b)` shares the same mechanism—the parameter name can be referenced in type annotations.
`self` appears in the position of `self: Point`; `b` appears in the position of `b: Positive(b)`.
Both type annotations reference the parameter itself. The only difference lies in the complexity of
the type annotation; the mechanism is exactly the same—after a name is bound, the type can depend on
that name.

The return type uses explicit function calls in the same way:

```yaoxiang
Sorted: (arr: Array(T)) -> Type = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }

sort: (arr: Array(T)) -> (result: Sorted(result)) = { ... }
//                        ^^^^^^^^^^^^^^^^^^^^^^^
//                        result is the return-value formal parameter, Sorted(result) is a function call
//                        compiler substitutes the return value into result at the return point, verifies Sorted(return value)
```

The same applies to local variable declarations:

```yaoxiang
let x: Positive(x) = 5
// x is bound to 5, Positive(5) → { 5 > 0 } → True → pass

// let y: Positive(y) = 0
// y is bound to 0, Positive(0) → { 0 > 0 } → False → compilation error
```

#### 3.2 Path Condition Collection

When a runtime value appears in a conditional branch, the compiler automatically collects path
conditions to form the current scope's **assumption set**. These assumptions participate in
verification as background knowledge for compile-time `Bool` evaluation.

```yaoxiang
if y > 0 {
    // compiler automatically obtains assumption in this branch: { y > 0 }
    let result = divide(x, y)
    // verification condition: (y > 0) ⇒ (y > 0)
    // proof pipeline judges entailment holds → Proved
} else {
// this branch assumption: { !(y > 0) }
// if calling divide(x, y), verification condition is !(y > 0) ⇒ y > 0
    // proof pipeline judges non-entailment → Disproved
}
```

This is not the compiler hard-coding a special pattern—it is the natural behavior of the
compile-time proof pipeline. Each type-checking call site sends the pipeline:

```
{background assumptions} ⇒ {verification target}
```

The proof pipeline judges entailment. `Proved` → pass, `Disproved` → compilation error +
counterexample, `Unproven` → compilation error + unproven proposition. Background assumptions come
from the path conditions at the current program point.

#### 3.3 Assumption Stack

During control flow analysis, the compiler maintains an assumption set for each basic block:

- **if-guard**: `if y > 0` → true branch pushes `y > 0`, false branch pushes `!(y > 0)` (if else is
  used)
- **match pattern**: `if let Some(v) = opt` → inside the branch pushes `opt == Some(v)`
- **Logical conjunction**: `if x > 0 and y < 10` → inside the branch pushes `x > 0` and `y < 10`
- **Function precondition**: when calling `divide(a, b)`, evidence that `b` satisfies `Positive`
  must come either from the current assumptions or from the actual argument's own refinement type
  annotation (if `b` is already annotated as `Positive`, its type carries `b > 0`)
- **Assignment**: when `let z = y`, the refinement conditions already on `y` are propagated to `z`

All assumptions enter the compile-time proof pipeline. When entering the SMT acceleration path, they
are translated to SMT-LIB background assertions.

#### 3.4 No Static Evidence ⇒ Compilation Error

If the programmer writes directly:

```yaoxiang
divide_user_input: (x: Int, y: Int) -> Int = divide(x, y)
```

There is no `y > 0` assumption at the current program point, and the actual argument `y` itself does
not have a `Positive` type annotation. The verification condition is:

```
{} ⇒ { y > 0 }
```

The pipeline returns `Disproved` (not entailed) → compilation error:

> Cannot prove that parameter `b` in the `divide` call satisfies `Positive`. `y` comes from function
> input with no proven bound. Consider guarding the call with an `if` branch:
> `if y > 0 { divide(x, y) }`.

YaoXiang does not accept runtime values directly entering refinement type arguments without
providing static evidence. This is not a restriction—this is the core of the hard-safety philosophy.
Any code the compiler cannot prove statically must not pass compilation.

#### 3.5 Relationship with the Unified Pipeline

Path condition propagation is not an additional mechanism. It is a direct extension of the
compile-time proof pipeline on control flow analysis:

| Stage                             | Responsibility                                                                                                                                                     |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Path Condition Collection         | Compiler's control flow analysis phase, annotates each basic block with its assumption set                                                                         |
| Verification Condition Generation | When encountering a type constraint to be verified, merge path conditions + actual argument type information                                                       |
| Proof Pipeline Evaluation         | Compiler kernel → SMT acceleration → yields `Proved` / `Disproved` / `Unproven`                                                                                    |
| Result                            | `Proved` → pass; `Disproved` → compilation error + counterexample; `Unproven` → compilation error + unproven proposition (programmer can provide a proof function) |

No new components. No special rules. Path conditions are the background knowledge of the proof
pipeline—sharing the same pipeline and the same budget system as type equality and borrow
constraints.

### 4. Compile-Time Proof Pipeline

All compile-time checks share one pipeline. The core operation of the pipeline is **type
checking**—checking whether a proof term's type equals the proposition to be proved. Everything is
type checking.

```
Compile-time encounters a Bool expression needing evaluation (i.e., a proof term needs to be constructed)
        │
        ├── Type equality (T1 == T2)
        │   → compiler judges directly (structural equivalence)
        │
        ├── Token conflict condition (!conflicting(tokens))
        │   → flow-sensitive liveness analysis (Dup/Linear property tracking)
        │
        ├── Dependent type reduction (n + m simplification)
        │   → compile-time term rewriting system (βδι-reduction)
        │
        ├── Compile-time predicate (x > 0, forall...)
        │   → compiler itself + SMT acceleration module
        │
        └── Hoare-logic entailment (P ⇒ Q)
            → compiler + SMT acceleration module
                    │
                    ▼
             ┌──────────┐
             │ Proved   │  → compilation passes
             │ Disproved│  → compilation error + counterexample
             │ Unproven │  → compilation error + unproven proposition
             └────┬─────┘
                  │
                  ▼
         Programmer writes proof function (YaoXiang code)
                  │
                  ▼
         Type checker verifies ──→ Proved ──→ compilation passes
                  │
                  ▼
            Verification fails → compilation error: "proof does not hold"
```

#### 4.1 Proof Result: A Three-Valued Algebra

Compile-time evaluation returns three results—this is the inevitable conclusion of the halting
problem and the natural division of proof theory:

```
eval_compile_time : BoolExpr → Proved | Disproved(Model) | Unproven
```

- **Proved** → halts, proof term constructed, type check passes. Compilation continues.
- **Disproved(M)** → halts, counterexample M exists. Compilation error + counterexample + source
  location.
- **Unproven** → within the given resource limit, no proof has been constructed. Compilation error +
  unproven proposition + budget consumption report.

**Unproven ≠ False.** The compiler saying "I cannot prove it" is not equivalent to the proposition
being false—only that it exceeds the current automatic proving capability. This is honesty, not a
defect.

A hard budget limit is the engineering solution to the halting problem. No knobs are
provided—providing one would be asking the user "do you think your program will halt", and neither
the user nor the compiler knows.

#### 4.2 After Unproven: The Programmer Writes a Proof

When the compiler returns `Unproven`, the programmer can write a **proof function**—a YaoXiang
function whose return type equals the proposition to be proved. The type checker verifies this
function—exactly the same mechanism by which it verifies `add(a, b): Int`.

```
proposition = type
proof       = program (a value of that type)
verification= type check (the sole root of trust)
```

The SMT solver is not an independent trust boundary—it is an **acceleration module of the type
checker**. SMT helps find proofs, but it is always the type checker that verifies the proof. When
SMT returns `unsat`, the compiler reconstructs its result as a proof term verifiable by the type
checker. If reconstruction fails (SMT's inference steps exceed the compiler kernel's inference
rules), it falls back to `Unproven`—the programmer can then manually write a proof function.

```yaoxiang
# Proposition: a refinement attribute the compiler cannot automatically prove
FirstIsMin: (T: Ord, arr: Sorted(T)) -> Type = {
    forall i in 0..arr.len: arr[0] <= arr[i]
}

# Proof: the programmer writes a function whose return type is the proposition above
# the type checker verifies this function—exactly the same as verifying add(a,b): Int
first_is_min: (T: Ord, arr: Sorted(T)) -> FirstIsMin(T, arr) = {
    # the compiler verifies here: the function body's type = FirstIsMin(T, arr)
    ...
}
```

No AI, no export to Coq, no new concepts. **Attributes the compiler cannot automatically prove at
compile time → the programmer writes the proof in YaoXiang code → the type checker verifies.** The
entire process is a smooth gradient—the compiler handles the easy proofs, leaving the hard ones for
the brain.

#### 4.3 Layered Dependencies within the Pipeline

The above evaluators share the same interface but have an evaluation order. Type equality is a
prerequisite for all subsequent analysis; ownership/token checks depend on type information;
refinement predicate verification depends on the results of the first two layers. The compiler
evaluates layer by layer; expressions failing at a lower layer do not enter the upper layer—avoiding
wasting solver budget on a program with type errors.

```
Evaluation order (same pipeline, layered scheduling)
├── Layer 0: Type equality (T1 == T2)
│   └── structural unification → failure means subsequent is meaningless, return Disproved directly
├── Layer 1: Ownership/token conflicts
│   └── flow-sensitive liveness analysis → failure means memory safety does not hold, return Disproved directly
└── Layer 2: Refinement predicate / Hoare entailment
    └── compiler itself → SMT acceleration → yield Proved / Disproved / Unproven
```

Each layer still returns `Proved/Disproved/Unproven`, sharing the same interface and the same budget
system.

### 5. Three-Layer Function Unity

| Level                  | Execution Time | Input      | Output | Example                                        |
| ---------------------- | -------------- | ---------- | ------ | ---------------------------------------------- |
| Value-level function   | Runtime        | Value      | Value  | `add: (a: Int, b: Int) -> Int = a + b`         |
| Type constructor       | Compile-time   | Type/Value | Type   | `List: (T: Type) -> Type = { data: Array(T) }` |
| Compile-time predicate | Compile-time   | Value      | Type   | `Positive: (x: Int) -> Type = { x > 0 }`       |

All use the same `name: type = value` syntax. Compile-time predicates and type constructors go
through the same compile-time proof pipeline—`{}` is the proof space.

### 6. Loops: Floyd-Hoare Verification Condition Generation

Loops do not need separate `: Invariant(...)` or `: decreases(...)` annotations. Compile-time
predicate type annotations on variables define Floyd-Hoare-style assertions—the compiler generates
verification conditions from the type annotations, and the proof pipeline checks whether each
assignment preserves the type.

Core mechanism: each assignment operation corresponds to a Hoare triple `{P} x := e {Q}`, with the
verification condition being `P ⇒ Q[e/x]`. The compiler generates one verification condition for the
loop body—once the proof pipeline verifies the induction step holds, all iterations are
automatically covered.

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # annotation references i; at initialization i=0, verify: 0 == sum(arr[0..0]) → True
    mut i: UpTo(arr.len) = 0     # verify: 0 <= 0 <= arr.len → True
    while i < arr.len {
        # the compiler generates one VC for the loop body. Precondition: s satisfies SumUpTo(arr, i), i satisfies UpTo(arr.len).
        #
        # s += arr[i]:
        #   verification obligation: s_new satisfies SumUpTo(arr, i) (current i unchanged)
        #   substitute s_new = s_old + arr[i]:
        #     need s_old + arr[i] == sum(arr[0..i+1])
        #     from induction hypothesis s_old == sum(arr[0..i]), add arr[i] to both sides:
        #     sum(arr[0..i]) + arr[i] == sum(arr[0..i+1])
        #   compiler + SMT: linear arithmetic, millisecond-level → Proved
        #
        # i += 1:
        #   i changes → s's type annotation in the dependency graph references i → triggers re-verification
        #   new verification target: s satisfies SumUpTo(arr, i_new)
        #   i.e., s == sum(arr[0..i_new]), guaranteed by the previous step → Proved
        s += arr[i]
        i += 1
    }
    return s  # at this point s: SumUpTo(arr, arr.len), i.e., s == sum(arr[0..arr.len])
}
```

Loop invariants are the type annotations on variables—the programmer writes the type, the compiler
checks the induction step. The compiler does not need to "discover" the invariant, nor does it need
to "automatically perform induction"—it decomposes the induction proof into local verification
conditions for each assignment operation, dividing and conquering via the proof pipeline.

#### 6.1 Dependency Tracking: Dependent Types on Mutable Variables

The prerequisite of the above mechanism is that the compiler knows that `s`'s type annotation
`SumUpTo(arr, i)` references `i`—when `i` changes, `s`'s type constraint also changes. This requires
the compiler to maintain a **type dependency graph between variables**.

**Data structure**:

```
TypeDepGraph: Map<VarName, Set<VarName>>
# key is the depended-on variable, value is the set of variables that reference that variable in their type annotations
# example: { i: {s}, j: {s, t}, ... }
```

**Construction**: When the type checker processes `mut v: Pred(... x ...) = init`, it resolves free
variable references in the `Pred(...)` arguments. If the arguments reference another mutable
variable `x` in the current scope, it records `x → v` in the dependency graph.

**Triggering**: When a depended-on variable `x` is assigned, the compiler:

1. Looks up all variables in the dependency graph that depend on `x`: `{v₁, v₂, ...}`
2. For each `v`, generates a verification condition: does `v`'s current value satisfy the updated
   type `Pred(... x_new ...)`?
3. Sends the VC into the proof pipeline

**Assignment-order sensitive**: Dependency tracking naturally enforces the correct assignment order.
Take `SumUpTo(arr, i)` as an example:

```yaoxiang
# Correct order
s += arr[i]   # s_new satisfies SumUpTo(arr, i+1)
i += 1        # i changes → revalidate s satisfies SumUpTo(arr, i_new) → True

# Wrong order—compiler rejects
i += 1        # i changes → revalidate s satisfies SumUpTo(arr, i_new)
              # s not yet updated, s_old == sum(arr[0..i_old]) ≠ sum(arr[0..i_new])
              # → compilation error: variable s does not satisfy type SumUpTo(arr, i_new)
s += arr[i]   # unreachable
```

**Composite dependencies**: a variable can depend on multiple variables. The type annotation
`{ v: Int; v == x + y }` depends on both `x` and `y`—any change triggers re-verification.

**Relationship with the proof pipeline**: Dependency tracking is the trigger for VC generation, not
an independent verification mechanism. It answers "when is a VC needed"—the proof pipeline answers
"does the VC hold".

### 7. Termination Checking

**Scope of application: refinement types.** Termination is not an independent switch, but part of
the **verification mode**: once a type is refined (`Refined { base, constraint }`), the computation
it annotates enters verification mode, which requires termination; ordinary types that are not
refined do not enter verification mode and generate no termination obligations.

This yields two corollaries:

- **Loops**: a bare `while` does not enter verification mode; when a measure variable carries a
  refinement annotation (such as `i: UpTo(n)`), it enters verification mode and termination must be
  proved.
- **Recursion**: when a function's signature carries a refinement (parameter refinement or
  refinement in the return type), it enters verification mode and every recursive call site must be
  shown to have a strictly decreasing measure.

Automatic priority within the mode: the compiler first automatically explores measures, and any it
can prove passes; if it cannot explore one and no explicit measure is given, it is a compilation
error. No **annotation syntax** is provided—measures and termination propositions are both written
in the type position, no new syntax such as `decreases` is introduced. The forms of measures and
explicit fallbacks are described in §6.9.

#### 6.1 Design Principles

The compiler automatically extracts the information needed for termination proofs from two sources:

1. **Variable type annotations**: boundary constraints in refinement types (such as `UpTo(n)` giving
   upper bound `n` and lower bound `0`)
2. **Loop body operations**: the operations applied to variables on each iteration

The compiler tries four measure-synthesis strategies in priority order, stopping as soon as one
succeeds. The four strategies are a **restricted template sequence for measure exploration**, with
inputs being refinement constraints (strategies 1–4 all start with "variables of bounded types"),
not "code evaluated at compile time"; they are the automatic and manual sides of the same thing as
the explicit measures in §6.9.

> **Measure exploration is exploration, not inference.** Exploration only enumerates templates
> (linear rank, violation count, bounded pattern, multiplicative scaling); it does not guarantee a
> solution exists—general measure inference is generally undecidable (reduces to the halting
> problem). Therefore, cases outside the templates must allow the programmer to explicitly provide a
> measure (§6.9), otherwise it is a compilation error.

#### 6.2 Strategy 1: Automatic Linear Rank Function Synthesis

When a variable carries a linear bound annotation, the compiler enumerates candidate linear measures
and verifies via SMT.

```
Input:
  variables v₁: UpTo(u₁), v₂: UpTo(u₂), ... (variables with upper and lower bounds)
  loop condition cond
  set of assignments in the loop body

Algorithm:
  1. Extract the bounds of each variable from type annotations: [low_i, high_i]
  2. Enumerate candidate measures: v_i, u_i - v_i, v_i - v_j, and other linear combinations
  3. For each candidate measure m:
     - SMT verifies m ≥ 0 (derived from type bounds)
     - For each execution path of the loop body, SMT verifies m' < m (strictly decreasing)
  4. Find a linear combination that satisfies the conditions → termination is proved
```

Coverage: any loop where a variable is assigned a linear expression (`v = a·v + b`) and carries a
bounded type annotation. This includes `i += const`, `i -= const`, and binary-search-style interval
shrinking:

```yaoxiang
# Binary search: low = mid + 1 or high = mid
# measure high - low strictly decreases on both paths
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

#### 6.3 Strategy 2: Predicate Violation Count—Automatically Extract Measure from Target Type <span style="color:orange">【Experimental Strategy】</span>

> ⚠️ **Current status: experimental strategy; whether to include it is decided based on actual
> feasibility when Phase 3 is implemented.** This strategy is effective for adjacent swap operations
> (bubble sort, insertion sort); it cannot automatically prove non-adjacent operations (quicksort
> partition, heapsort sift-down). See the table below for the coverage boundary. If Phase 3
> verification proves infeasible, this strategy will be removed or downgraded to future work.

Core insight: **the specification the user writes is the raw material for the compiler's
reasoning.** The compiler does not need to have "what is sorting" built in—it reads the `Sorted`
definition and automatically extracts the measure from it.

```
Input:
  target type: Sorted(arr) = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }
  loop body operations: adjacent-element swaps

Algorithm:
  1. Parse the predicate definition: forall i in range: cond(i, arr)
  2. Automatically generate the measure: violation_count = |{ i | ¬cond(i, arr) }|
  3. Analyze the operation's effect on the measure:
     - adjacent swap arr[j], arr[j+1] = arr[j+1], arr[j]
     - only affects pairs at indices j-1, j, j+1
     - if arr[j] > arr[j+1] (violates the predicate), the swap makes the pair satisfy the predicate
     - violation_count decreases by at least 1
  4. Upper bound: n·(n-1)/2 (maximum number of adjacent inversions), lower bound: 0
  → termination is proved
```

**Current coverage**:

| Algorithm      | Operation Pattern | Provable by Strategy 2? | Reason                                                    |
| -------------- | ----------------- | :---------------------: | --------------------------------------------------------- |
| Bubble sort    | Adjacent swap     |           ✅            | `violation_count` strictly decreases on each swap         |
| Insertion sort | Adjacent move     |           ✅            | Each shift eliminates one violating pair                  |
| Selection sort | Non-adjacent swap |           ❌            | A single swap may increase `violation_count`              |
| Quicksort      | partition         |           ❌            | Non-adjacent swap, monotonic decrease not guaranteed      |
| Heapsort       | sift-down         |           ❌            | Tree-shaped operation, `violation_count` is non-monotonic |

**Complementary strategy**: for quicksort, the `low < high` interval shrinking is covered by
Strategy 1 (linear rank function)—the outer partition recursion halves the interval each time.
Strategies 1 and 2 complement each other, and termination for most practical algorithms can be
proved by one of them. But the generalization of Strategy 2 (non-adjacent operations, tree-shaped
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

#### 6.4 Strategy 3: Bounded Increment/Decrement Patterns

`v += const` (positive constant), the variable has an upper-bound type annotation → the measure
`upper_bound - v` decreases by `const` each time, lower bound 0. This is a degenerate case of
Strategy 1, which the compiler handles first and quickly.

#### 6.5 Strategy 4: Multiplicative Scaling Measure Templates

`v *= const` (const > 1), the variable has both an upper-bound and a lower-bound type annotation.
The compiler has a built-in logarithmic measure template `ceil(log_const(upper/v))`, which decreases
by 1 each time `v` is multiplied by `const`.

```yaoxiang
mut i: Positive(i) = 1
while i < n {
    # compiler automatically derives: measure ceil(log₂(n/i)), decreases by 1 each time i is doubled
    i *= 2
}
```

#### 6.6 Separation of Termination and Correctness

Termination proof and correctness proof are independent:

- **Termination**: the four strategies above automatically prove that the loop exits in a finite
  number of steps; if exploration fails, the programmer provides a measure in the type position
  (§6.9)
- **Correctness**: whether the loop body progresses toward the target type, checked by the
  compile-time proof pipeline through verification conditions

Both pass → compilation passes. Termination is proved but correctness fails → compilation error +
counterexample. Correctness is proved but termination cannot be proved → compilation error pointing
out the variable or operation that cannot be analyzed. Both fail → compilation error reporting both
failure reasons separately.

#### 6.7 Termination Checking for Recursive Functions

For recursive functions with refined signatures, the compiler checks that each recursive call site
has strictly decreasing formal parameters:

```yaoxiang
gcd: (a: Int, b: NonNegative(b)) -> Int = {
    if b == 0 { return a }
    return gcd(b, a % b)  // compiler explores: well-ordering measure of (b, a % b) decreases → terminates
}
```

Decreasing formal parameters is the strongest automatic path (structural recursion). When
exploration fails, the programmer explicitly provides a measure in the type position (§6.9).

#### 6.8 Hard Boundary

`i = f(i)` where `f` is non-invertible, not closed, and preserves no monotonicity—mathematically
impossible to automatically prove termination. Compilation error:

> This loop cannot be automatically proven to terminate. The loop variable depends on the
> non-analyzable function `f`. Please use an iteration pattern analyzable by the compiler, or bind a
> name to this loop and provide a measure in the type position (§6.9).

This is not a compiler failure. Any code that cannot be statically proven safe must not pass
compilation. Even when an explicit measure is given, it must be judged true by SMT to pass—a wrongly
written measure will be rejected by a counterexample; a human can only fail to prove, not prove
wrong.

#### 6.9 Explicit Measure: `Terminates`

When automatic exploration fails to produce a measure, the programmer writes the measure in the
**type position**—the same mechanism (`Terminates` predicate application) as `Positive(b)` and
`IsMax(T, arr, result)`, with zero new syntax:

```yaoxiang
// Measure: an ordinary function, unit-testable, reusable, not involved at runtime
gcd_measure: (a: Int, b: Int) -> Int = { b }

// The termination component lives in the function's own type position
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// Loop: the binding name is the anchor, the measure picks up in-scope quantities by name
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n {
        i = i + 1
        i
    }
    return acc
}
```

The reason this works for loops: `Terminates(m)` refines the value type of the loop body's tail
expression, the anchor is provided by the binding name `acc`—the loop is thereby denotable, with no
"anonymous construct that cannot be denoted" blind spot.

> **The body tail must yield a value**: the value of a loop = the value of the loop body block
> (§2.9), and the value of a block = its tail expression. If the body tail is an assignment
> statement (such as `{ i = i + 1 }`), the block value is `Void`, and `return acc` would be
> returning `Void` from `-> Int`, making the type inconsistent. Hence the body tail in this example
> explicitly writes `i`. This does not change the semantics of the measure—the measure `n - i` is a
> **state expression**, unrelated to the loop's value type.

> **Implementation Note (registered 2026-10-02; updated 2026-10-04)**: The **type side** of the loop
> example in this section holds (`check` 0 errors, back-edge obligation `Proved`). The **runtime
> value-taking defect (#409) has been fixed**: a `while` used as a value now takes the value of the
> body block's tail expression from the last iteration, with the same standard as the type-side
> `block_value_ty`—`loop(4) == 4`, `countdown(3) == 30` verified at runtime (same fix for `for`).
>
> **Known residual (zero iterations)**: when the loop body is never evaluated, there is no
> "last-iteration body value", and the loop value still defaults to `Void`—binding it to a
> non-`Void` type position and using it at runtime reports
> `E6007 Runtime error: type mismatch in comparison Eq: Void vs Int(0)` (a loud type mismatch, not a
> silent wrong value). This is a **static approximation boundary**: the `block_value_ty` criterion
> gives the type of the body tail expression; it has no ability to judge "whether the loop executes
> at least once" (which would require data-flow/provability analysis), so the type side does not
> unilaterally downgrade (downgrading would break the value type on which this section's
> `Terminates` loop binding depends). This residual is pinned by the corpus
> `tests/yaoxiang/02-type-system/while_zero_iteration_void_err.yx`
> (`// expect: runtime-error E6007`). **This residual has been confirmed as a deliberately retained
> design boundary** (ruling 2026-10-04, not fixed for now), so issue
> [#409](https://github.com/ChenXu233/YaoXiang/issues/409) has been closed with the main shape fixed
> and no longer tracked as a separate item. For the minimal reproduction and impact-surface history,
> see [RFC-027a](../review/027a-termination-explicit-measure.md) §Example Loop Section "Known Defect
> D6". Additionally, termination Strategy 1 (linear rank function) is re-enabled this round, so the
> order in this section—automatic exploration first, then explicit measures—remains unchanged:
> explicit measures are still the fallback after exploration fails.

**`Terminates` is a builtin predicate**, on the same level as `Int` and `Never` among the core
primitives. It is the only predicate **whose body is written by the compiler**—its assertion ("the
measure strictly decreases at every recursive call site / loop back edge") lives inside the
computation structure, and the user-written predicates cannot reference the function body or loop
body, so it cannot be expressed by the predicate definition syntax in the "Syntax" section. The
builtin side converges to this single name.

**Arity**. `Terminates(FnType, m)` and `Terminates(m)` are the two arities of the same predicate,
not two different constructs:

| Form                    | Anchor                             | Use                                                           |
| ----------------------- | ---------------------------------- | ------------------------------------------------------------- |
| `Terminates(m)`         | The name of the containing binding | Self-recursive functions, loops—the default form              |
| `Terminates(FnType, m)` | Explicit function type             | Mutual recursion and other scenarios with a non-unique anchor |

Both are essentially the same: the termination obligation always falls on "the computation annotated
by the type position where the refinement resides".

**The measure does not restrict the return type.** A measure can be an expression of any type (not
necessarily a natural number); "strictly decreases" is given by the well-ordering available on that
type. Whether the measure is well-founded (such as whether the return type `Int` is `>= 0`) is an
**independent obligation**, also handed to refinement inference or SMT; when neither can derive it,
the diagnosis does **not** directly reject, but rather suggests a direction to check (whether the
lower bound of the measure holds, whether the recursive parameter truly moves in that direction).

**Relationship with automatic exploration**: explicit measures are not another pipeline, but the
input after exploration fails. Once given, the measure still goes through the same SMT to verify
decrease and well-foundedness; if either fails, an error is reported along with a counterexample.

For obligation generation, the judgment pipeline, diagnostic direction, and shared measures for
mutual recursion (SCC), see
[RFC-027a: Explicit Measures for Termination Checking](../review/027a-termination-explicit-measure.md).

### 8. SMT Solver: An Acceleration Module of the Type Checker

The SMT solver is an external tool in traditional languages (e.g., F\* calls Z3, Dafny calls Z3). In
YaoXiang, it is **an acceleration module of the type checker**—only invoked when the compiler kernel
itself cannot directly judge. SMT helps find proofs, but the type checker is what verifies them.

**Trust model**: the type checker is the sole root of trust. The SMT solver is an acceleration
module—it helps find proofs, but SMT is not an independent trust boundary. The compiler trusts Z3's
`unsat` result (consistent with the F\*/Dafny approach—the probability of Z3 being wrong is lower
than the bug rate of the compiler itself, a pragmatic engineering choice). The real unreliability is
controlled at the SMT translation layer—if there is a bug in the translation, the compiler will
surface it in other tests.

**Interface**: the compiler internally translates to the SMT-LIB 2.6 standard format, rather than
binding to a specific solver API. SMT-LIB is an ISO standard, natively supported by Z3, CVC5,
MathSAT, and Yices.

**Default backend**: Z3 (MIT license, the most widely documented and community-validated). CVC5 as
an SMT-LIB-compatible alternative—users can switch at compile time via a compiler flag.

No "generic solver abstraction layer" is built—SMT-LIB _is_ the abstraction layer. If CVC5 has a
breakthrough in a particular theory in the future, switching only requires swapping the binary, with
no compiler code changes needed.

```
Compile-time Bool expression
        │
        ├── Compiler kernel can directly judge (structural equivalence, simple arithmetic,
        │   trivial formulas after constant folding)
        │   → return Proved / Disproved directly
        │
        └── Compiler kernel cannot directly judge (quantifiers, symbolic variables)
            → dependent type pre-reduction (factorial(5) → 120)
            → translate to SMT-LIB format
            → send to Z3/CVC5 (with budget limit)
            → return value: unsat → Proved  │  sat + model → Disproved  │  unknown → Unproven
```

**Solver budget—a hard limit, like stack depth**:

| Budget Dimension               | Default | Description                                                                                                                                                |
| ------------------------------ | ------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Solver steps                   | 10,000  | Z3 typically finishes linear arithmetic in a few hundred steps. 10,000 steps covers 99% of real predicates.                                                |
| Time                           | 100ms   | A single predicate exceeding 100ms = the user is writing a compile-time program, not a type annotation. 100ms × 50 predicates = 5-second compile time cap. |
| Quantifier instantiation depth | 3       | Three nested levels of quantifiers cover real patterns. Beyond three, the user is likely writing logic exercises.                                          |

Exceeding the budget returns `Unproven`, with a compilation error + predicate location +
consumption. No degradation, no runtime check, no silent pass.

**Why this is practical**: in practice, 95% of real predicates are linear arithmetic—`x > 0`,
`arr.len > 0`, `0 <= idx < arr.len`—all in decidable fragments, where SMT solvers return in
milliseconds. For the rare complex predicates that exceed the budget, the programmer can write a
proof function.

Dependent types go through a pre-reduction pass before SMT calls: `factorial(5)` is directly
evaluated at compile time to `120`, `append([1,2], [3])` is directly evaluated to `[1,2,3]`. These
deterministic value computations do not consume SMT budget.

The programmer does not need to know SMT exists. The mental model is: **the compiler proves what it
can, errors on what it cannot—and if the compiler cannot, you can write a function to prove it to
the compiler**.

### 9. Compile-Time Predicate Composition

Compile-time predicates are functions returning `Type`; composition is achieved naturally through
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

result = divide(10, 2)   # ✅ compiler verifies Positive(2) = { 2 > 0 } → True
# result = divide(10, 0)  # ❌ compiler verifies Positive(0) = { 0 > 0 } → False
```

#### 9.2 Safe Array Access

```yaoxiang
InBounds: (idx: Int, arr: Array(T)) -> Type = { 0 <= idx and idx < arr.len }

get: (arr: Array(T), idx: InBounds(idx, arr)) -> T = arr.data[idx]

arr = Array(Int)(1, 2, 3)
x = get(arr, 1)   # ✅ compiler verifies InBounds(1, arr) = { 0 <= 1 and 1 < 3 } → True
# y = get(arr, 5)  # ❌ compiler verifies InBounds(5, arr) = { 0 <= 5 and 5 < 3 } → False
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

### 11. The `dispatch` Dispatch Pipeline: Unified Compile-Time and Runtime Dispatch

> **Implementation Note (2026-10-03, #377-2)**: The **semantics** of this section holds and is in
> production, but the **structure** is not a standalone `layers/dispatch.rs` module. Production
> dispatch is directly implemented by three **call-site obligations** in `checker.rs` (directly
> calling `layers::predicate::check_predicate`): binding-position re-validation
> (`revalidate_refined`), call-site arguments (`check_call_arg_refinements`), and return-position
> postcondition (`check_return_refinement`). The original `layers/dispatch.rs` had zero production
> call sites, and its handling of `Unproven { ProofFunctionRequired }` was "downgrade to a W1080
> warning and inject into Γ", which conflicts with the production behavior (`Unproven` always
> errors, see `refined_unproven`), so the whole module has been deleted. Note also: the "insert
> runtime check" item in the Runtime row of the table in this section has **no** emission site in
> production yet (`RuntimeOutcome::InsertCheck` has no consumer), and Γ injection is independently
> handled by `ownership.rs`'s branch guards and `inference/expressions.rs`.

`assert` and `Assert` are the two sides of the same refinement type primitive. The dispatch pipeline
`dispatch` automatically decides between compile-time proving and runtime checking based on
**whether the predicate's free variables are reachable at compile time**:

| Criterion                                                                                    | Mode            | Behavior                                                                                               |
| -------------------------------------------------------------------------------------------- | --------------- | ------------------------------------------------------------------------------------------------------ |
| All free variables known at compile time (generic parameters, compile-time constants)        | **CompileTime** | Enter the proof pipeline: `Proved` → erase, `Disproved` → compilation error, `Unknown` → require proof |
| Some free variables come from runtime (function parameters, external input, `mut` variables) | **Runtime**     | Insert a runtime check, and inject the refinement fact into the flow-sensitive assumption set Γ        |

**Key point**: "Cannot decide" ≠ "Disproved". In CompileTime mode, `Unknown` requires a proof (no
silent degradation); in Runtime mode, the proposition has no truth value at compile time at all—no
matter how strong the prover, it cannot write a universally true proof for "the user may have
entered a negative number"; a runtime check is the only sound choice. This is not the prover being
weak, it is theoretical necessity.

### 12. Flow-Sensitive Assumption Set Γ: Strongest Postcondition Propagation

The compiler maintains a flow-sensitive assumption set Γ, tracking the propositions known to hold at
each control flow point.

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

This is a hard soundness requirement—when the variable's value changes, old assumptions are invalid.

**Branch confluence**: when an `if`/`else` or `match` branch merges, Γ takes the intersection of
each branch's assumptions. Only propositions that hold on all paths carry out of the branch.

### 13. Erasure Model Clarification: witness erasure ≠ check erasure

The RFC-027 claim that "refinement types are **completely erased** at runtime" refers to the **proof
witness**—proof terms that have been verified at compile time do not generate runtime code. But the
**runtime check** inserted by dispatch in Runtime mode is preserved—it is a `Bool` check executed at
the value level, not a witness at the type level.

Summary: witness erased, check preserved. The two do not conflict, and the original RFC-027 claim is
unchanged.

## Detailed Design

### Syntax Changes

| Before (RFC-022)                      | After (This RFC)                                                                                                                                        |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `//! requires: NonEmpty(n) = n > 0`   | Compile-time predicate as parameter type `(b: Positive(b))`                                                                                             |
| `//! ensures: ExistsMax(result, arr)` | Return type uses return-value formal parameter `-> (result: IsMax(T, arr, result))`                                                                     |
| `/*! invariant: ... !*/`              | Compile-time predicate type annotation on the variable—Floyd-Hoare invariant                                                                            |
| `//! decreases: n`                    | Measure written in the refinement type position (`Terminates`); compiler first tries automatic exploration, only requires an explicit one if that fails |
| Specification is a comment            | Specification is the type system                                                                                                                        |

### Syntax

**Compile-time predicates introduce no new keywords.** `{}` is the proof space, identical to the
existing type definition syntax. A compile-time predicate is simply a function returning
`Type`—`name: (params) -> Type = { assertions }`. To use it is to make a function
call—`Positive(b)`, `IsMax(T, arr, result)`.

```bnf
# Compile-time predicate = function returning Type, with compiler-verified assertions inside {}
# uses existing function/type syntax, no new BNF rules needed
predicate ::= identifier ':' params '->' 'Type' '=' '{' assertions '}'
```

**Predicate application arguments must be in compile-time constant form**—literals, variables (bound
by name), type applications (recursively extracted), or **compile-time-denotable function
references** (function names). An argument form that cannot be converted to a constant expression
reports **E1092**; a mismatch between the number of arguments and the predicate's declared formal
parameters reports **E1093**. Arguments are bound positionally to the formal parameter list, with
predicate arity determined by the declaration—`Positive(x)` is unary, `IsMax(T, arr, result)` is
ternary, `Terminates(m)` and `Terminates(FnType, m)` are unary and binary—**refinement constraints
are never silently dropped** (previously, non-convertible arguments caused constraints to vanish
silently, and bindings violating constraints would silently pass).

**New syntactic concept: return-value formal parameter**—`name` in `-> (name: Type)` is the
return-value formal parameter.

The return-value formal parameter is the **only one syntactic concept** YaoXiang introduces on top
of the existing function syntax. Its semantics:

- The value of `name` is provided by the `return` statement
- `name` exists only in the type signature, referenced by the postcondition predicate (such as
  `-> (result: IsMax(T, arr, result))`)
- `name` does not enter the function body scope, does not appear on the caller side
- The return-value formal parameter is **optional**—when there is no postcondition, the signature is
  identical to an ordinary function (`-> Int`), with no extra burden

The reason for introducing it: postconditions need to reference "the value the function is about to
return". Without a return-value formal parameter, the compiler could only let the predicate
reference the return value through special rules (such as an implicit variable `$result` or
`__retval__`). The return-value formal parameter makes this reference explicit—it is just a formal
parameter, with the only difference being that its value is provided by `return` rather than the
caller.

**Proof functions** are not a new concept—they are simply a YaoXiang function whose return type is
the proposition being asserted. When the compiler returns `Unproven`, the programmer provides a
proof function, and the type checker verifies it in exactly the same way it verifies any function's
return type. No new syntax, no new keywords, no new rules.

> **The boundary between the two.** The correctness domain (predicate `Unproven`) is supplemented in
> the **body**: write a function whose return type is the proposition to be proved. The termination
> domain (§6.9) is supplemented in the **type position**: write `Terminates(measure)` as the
> binding's or function signature's type. The former is "write a proof for an unprovable
> proposition", the latter is "explicitly declare a measure that exploration cannot find"—same
> mechanism (refinement type application), different landing point. The termination domain **does
> not need** to write a `_proof` function.

### Type System Impact

- **Type universe**: compile-time predicates live in the Type₂ layer—functions that take values and
  return `Type`, on the same level as type constructors
- **Generics interaction**: compile-time predicates may carry generic parameters, such as
  `NonEmpty: (T: Type) -> (arr: Array(T)) -> Type`
- **Ownership interaction**: expressions in compile-time predicates obey ownership rules and may
  only read, not write
- **Type inference**: the arguments of compile-time predicates participate in HM type inference

### Runtime Representation

Compile-time predicates are handled at runtime **according to the dispatch result**:

- **CompileTime mode** (all free variables known at compile time): after the proof passes, the
  witness is completely erased. `Positive: (x: Int) -> Type = { x > 0 }`—`b: Positive(5)` is
  represented at runtime simply as `Int`. The refinement condition `{ 5 > 0 }` has already been
  verified and is erased.
- **Runtime mode** (some free variables come from runtime): preserve the runtime check—execute a
  `Bool` check at the value level, injecting into the flow-sensitive assumption set Γ. See §11
  dispatch pipeline and §13 erasure model clarification for details.

Placing a compile-time predicate in a type position (such as `f(x: Positive(x))`) does not produce a
wrapper type or allocate extra memory. But when `x` comes from a runtime input, a runtime `Bool`
check **will** be inserted.

**Interaction constraint with `ref`**: compile-time predicates may only reference immutably borrowed
or ownership-transferred values. A compile-time predicate referencing a mutably borrowed value
cannot guarantee at compile time that the verification result still holds at runtime—such usage
directly reports a compilation error.

### Compiler Changes

1. **Parser**: compile-time predicates use standard function syntax, no additional parsing rules
2. **Compile-time proof pipeline**: unified `Proved/Disproved/Unproven` return interface, automatic
   strategy selection
3. **SMT acceleration module**: SMT-LIB 2.6 translation layer, default backend Z3, CVC5 as
   alternative
4. **Type checker kernel**: inference rule implementation—structural equivalence, βδι-reduction,
   universal quantifier introduction/elimination. This is the sole root of trust; both SMT and
   programmer proofs are verified through it
5. **Verification condition generation**: WP/SP calculus + loop invariant proof obligations
6. **Error reporting**: counterexample formatting + unproven proposition reports + source location
   correlation

### Backward Compatibility

- ✅ Code not using compile-time predicates is completely unchanged
- ✅ Compile-time predicates have zero runtime overhead in CompileTime mode; only necessary `Bool`
  checks are retained in Runtime mode
- ⚠️ RFC-022's `//!` syntax is no longer supported—but 022 was never implemented, so there is no
  migration burden

## Trade-offs

### Advantages

- **Curry-Howard correspondence fully realized**: types as propositions, programs as proofs,
  `name: Proposition = Proof`
- **Unity**: compile-time predicates and ordinary functions use exactly the same syntax, with no
  conceptual split
- **SMT transparency**: programmers do not need to know SMT exists; the mental model is consistent
  with type checking
- **Progressive adoption**: can start with a single compile-time predicate and gradually increase
  coverage
- **Minimal runtime overhead**: zero overhead in CompileTime mode; only necessary `Bool` checks in
  Runtime mode

### Disadvantages

- **Compile time**: SMT solving increases compile time, but hard budget limits keep the upper bound
  controllable
- **Automatic proof boundary**: complex predicates beyond first-order linear arithmetic may require
  the programmer to write proof functions. This is not a language defect—it is the inevitable
  conclusion of the halting problem. The compiler honestly reports `Unproven` rather than falsely
  reporting `True`/`False`
- **Learning curve**: writing effective compile-time predicates and proof functions requires an
  understanding of the basic intuition of the Curry-Howard correspondence
- **Implementation complexity**: unifying the compile-time proof pipeline requires careful design

### Risk Mitigation

- Hard limit on SMT solving budget (10,000 steps / 100ms / 3 instantiation depth), exceeding the
  budget returns `Unproven`
- Dependent type pre-reduction: deterministic value computations are consumed first; SMT only chews
  on the non-deterministic parts
- `Unproven` is not a dead end: in the correctness domain, write a proof function (return type is
  the proposition); in the termination domain, provide a measure in the type position (§6.9)—both
  are verified by the type checker
- Incremental validation: only changed modules are re-validated
- Clear error messages + counterexample display + budget consumption report + unproven proposition +
  suggestion (if the compiler can give one)

## Alternatives

| Alternative                                                        | Why Not Chosen                                                                                                                                                                   |
| ------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| RFC-022: `//!` comment-style specifications                        | Specifications and types are split, violating the Curry-Howard correspondence                                                                                                    |
| Separate specification files (such as CVL)                         | Specifications and code are separated, increasing maintenance cost                                                                                                               |
| Runtime-only assertions                                            | Cannot statically guarantee correctness                                                                                                                                          |
| External proof assistants (such as Coq)                            | Disconnected from the compiler, requires a separate proof language and trust boundary. YaoXiang's choice: the proof is YaoXiang code; the type checker is the sole root of trust |
| **This proposal: compile-time predicates as first-class citizens** | ✅                                                                                                                                                                               |

## Implementation Strategy

### Phase Breakdown

| Phase       | Content                                                                                                                                                                    |
| ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Phase 1** | Compiler kernel: structural equivalence + βδι-reduction + universal quantifier introduction/elimination. Support for simple arithmetic predicates (`x > 0`, `arr.len > 0`) |
| **Phase 2** | SMT-LIB translation layer + Z3/CVC5 integration. Pipeline returns `Proved/Disproved/Unproven`. When `Unproven`, the programmer can write proof functions                   |
| **Phase 3** | Loop invariant VC generation + termination checking (measure exploration four strategies + `Terminates` explicit measure, §6, §6.9)                                        |
| **Phase 4** | Incremental validation + caching + IDE support                                                                                                                             |

### Dependencies

- RFC-010: Unified Type Syntax — compile-time predicates are based on `name: type = value`
- RFC-011: Generic Type System — compile-time predicates may carry generic parameters
- RFC-009: Ownership Model — compile-time predicate expressions obey ownership rules

## Open Questions

- [x] **Z3 availability for wasm32 targets**: there is no Z3 under wasm (SMT code is entirely
      excluded by `cfg(not(target_arch = "wasm32"))`). Scope of impact and degradation direction: -
      `ownership.rs` `smt_cut` always returns `false` → back edge traversed → **conservative
      rejection** (sound direction, just less narrowing) - Termination check Strategy 1, predicate
      level 2b/3 skipped entirely

  **Correction (2026-10-01)**: the original text claimed these paths are "currently **unreachable**"
  (the reason being that `predicate_defs` production is never populated, `parser` does not produce
  `MonoType::Refined`, `with_solver` production is never called), but this is no longer the case—all
  three have landed (predicate definition registration #377-3; refinement type normalization;
  production pipeline injecting solver). So the difference between wasm and native is no longer
  limited to `smt_cut` precision:

  | Path                                        | native   | wasm       | Degradation Direction                 |
  | ------------------------------------------- | -------- | ---------- | ------------------------------------- |
  | Termination check explicit measure judgment | Judged   | `Unjudged` | Does not report E4022 (fewer reports) |
  | Well-foundedness judgment                   | Judged   | `Unjudged` | Does not report E4022 (fewer reports) |
  | Refinement predicate SMT entailment level   | Executed | Skipped    | Falls back to `Unproven`              |

  All are in the **conservative** direction (less narrowing, fewer reports), with no impact on
  soundness. For parity in precision, the plan and cost are in issue #376 (conclusion: prioritize a
  JS-side Z3 instance, rather than linking Z3 into the main wasm—the latter requires switching the
  emcc build system and grows the artifact from 3 MB to ~20 MB).

- [x] **SMT solver choice**: default Z3 (MIT license, most widely validated). CVC5 as an
      SMT-LIB-compatible alternative, switched via compiler flag. The compiler internally translates
      to the SMT-LIB 2.6 standard format—SMT-LIB _is_ the abstraction layer, no custom generic
      solver interface is built.
- [x] **Specific solver budget values**: 10,000 steps / 100ms / quantifier instantiation depth 3.
      Fixed inside the compiler, no knobs. If actual usage proves these insufficient (not "the user
      wrote it wrong"), adjust later.
- [x] **Quantifier support scope**: no quantifier order is limited at the language level.
      Compile-time predicates accept `Type` parameters—`Type` includes function types—so
      higher-order quantifiers are a natural consequence of the type system, requiring no special
      syntax. The SMT solver can automatically judge first-order quantifiers (`forall`/`exists`,
      supporting interleaved nesting, limited by budget depth 3). Higher-order quantifiers: SMT
      returns `Unproven`, the compiler prompts "this predicate is beyond the scope of automatic
      proof, please provide a proof function". The programmer writes a YaoXiang function whose
      return type equals that proposition—the type checker verifies that function. No external
      export, no AI, no interactive proof mode needed. Everything is YaoXiang code; everything is
      verified by the type checker.
- [x] **Counterexample formatting**: source variable names are used directly as SMT variable names
      (with module prefixes to avoid conflicts). When the Z3 model is returned, lookup is by
      variable name. Output format: variable name = concrete value + source location + predicate
      definition location. No complex mapping layer.
- [x] ~~**Interaction between compile-time predicates and `ref` smart pointers?**~~ → Decided:
      compile-time predicates only allow immutably borrowed or ownership-transferred values. A
      mutably borrowed value cannot appear in a compile-time predicate.
- [x] **Extension of `forall` predicate violation-count measure to non-adjacent operations?** → No
      extension. The current coverage (adjacent swap, adjacent move) is complemented by Strategy 1
      (linear rank function)—quicksort's outer interval shrinking is covered by Strategy 1, and
      heapsort is covered by Strategy 1 (array index pattern). For loops that cannot be proven to
      terminate by any strategy, the compiler directly reports an error—this is the hard-safety
      philosophy, not a defect. If a real (non-academic) algorithm in the future cannot be covered
      by all four strategies, discussion will be reopened. → **Re-discussion has been triggered
      (2026-09-14, #318)**: non-structural recursion (gcd-style non-directly-decreasing recursion,
      mutual recursion, merge partition) is exactly the real scenario this clause anticipates—the
      template sequence for measure exploration cannot automatically take them down, and they cannot
      be rewritten into analyzable iteration patterns without destroying readability. Conclusion:
      the hard-safety philosophy is unchanged (a wrongly written measure is still rejected by SMT
      counterexamples; a human can only fail to prove, not prove wrong), but "reject if exploration
      fails" is relaxed to "if exploration fails, the programmer can explicitly provide a measure in
      the type position"—see §6.9.
- [x] **Linear rank function enumeration combinatorial explosion**: the upper limit for candidate
      enumeration is 3 bounded variables. At ≤3, enumerate all linear combinations and verify one by
      one via SMT. When >3, only try single-variable measures (`v_i`, `u_i - v_i`); failure directly
      reports a compilation error—prompting the programmer "the loop has >3 bounded variables; the
      compiler cannot automatically synthesize a multi-variable measure". This is not an engineering
      compromise—it forces programmers to write simpler loops.

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

## Lifecycle and Destination

```
┌─────────────┐
│   Draft     │  ← Author creates
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Under Review│  ← Current state: community discussion
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
│ (formal)    │    │ (kept in place)│
└─────────────┘    └─────────────┘
```
