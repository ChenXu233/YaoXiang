---
title: 'RFC-027: Compile-time Predicates and Unified Static Verification'
status: 'Accepted'
author: 'Chenxu'
created: '2026-06-07'
updated: '2026-10-02'
impl_status: 'in-progress'
impl_detail:
  'Phases 1–2 complete, Phase 3 partially complete, Phase 4 partially complete. The unified
  assert/Assert scheme across 6 phases is fully implemented (issues #157–#162 closed): Never type,
  IsTrue bridging, flow-sensitive Γ + kill set, type-level recursion, universe stratification weak
  check, dispatch pipeline.'
impl_percent: 85
issue_number: 90
issue_url: 'https://github.com/ChenXu233/YaoXiang/issues/90'

issue: '#90'
---

# RFC-027: Compile-time Predicates and Unified Static Verification

> **References**:
>
> - [RFC-009: Ownership Model](../accepted/009-ownership-model.md)
> - [RFC-010: Unified Type Syntax - name: type = value model](../accepted/010-unified-type-syntax.md)
> - [RFC-011: Generic Type System Design](../accepted/011-generic-type-system.md)
> - [RFC-024: Concurrency Model Based on spawn blocks](../accepted/024-concurrency-model.md)
>
> **Supersedes**:
> [RFC-022: Hoare Logic Static Verification Support (Specification Comments and Specification Types)](../deprecated/022-hoare-logic-static-verification.md)
> — deprecated

## Summary

This RFC proposes introducing **compile-time predicates** as first-class citizens in YaoXiang,
unifying all compile-time static verification into a single **proof pipeline**. Compile-time
predicates are not bolted-on specification comments — they are functions. A function that returns a
`Type` can be used in type positions; the compiler calls it at compile time and checks the return
value. Types are propositions; compile-time evaluation is proof.

**Core thesis**: The compiler's only job at compile time is to construct and verify proof terms.
Type equality, token conflicts, dependent type reduction, compile-time predicate evaluation,
Hoare-logic implication — all are different type checks in the compile-time proof pipeline, sharing
the same pipeline. The SMT solver is an accelerator module of the type checker, not an independent
trust boundary. When the compiler returns `Unproven`, the programmer writes a YaoXiang function as
proof — the type checker verifies it exactly as it verifies any function's return type. Everything
is YaoXiang code, everything is verified by the type checker.

## Motivation

### Why deprecate RFC-022?

RFC-022 designs specifications as `//!` comments:

```yaoxiang
max: (T: Ord) -> ((arr: Array(T, n)) -> T) = {
    //! requires: NonEmpty(n) = n > 0          ← Comment independent of types
    //! ensures: ExistsMax(result, arr[0..n])   ← Comment independent of types
}
```

This commits the fundamental error of violating the Curry–Howard correspondence: **splitting
specifications and types into two layers**. Comments are not types. Comments do not participate in
type checking. Comments fit the mental model of "external tools."

The white paper is clear:

> "No `//!` comments. No standalone specification language. Everything is inside the type system."

### Current Problems

- RFC-022's `//!` comments are external syntax independent of the type system
- Specification types and ordinary types are two systems, causing conceptual redundancy
- The Debug-Build-verifies / Release-Build-ignores split breaks uniformity
- SMT solvers are conventionally positioned as external tools — YaoXiang builds them in as
  accelerator modules of the type checker
- Type checking, borrow checking, compile-time predicate checking, and macro expansion each take
  different paths

### The Right Mental Model

Type checking can be abstracted as a function:

```
verify : Program → Proved | Disproved(Model) | Unproven
```

All compile-time checks — simple type matching, borrow conflict detection, compile-time predicate
verification — are subtasks of this function. They share the same proof pipeline; the only
difference is the complexity of the proof term and the construction strategy.

When the compiler returns `Unproven`, the programmer provides a proof function — the function's
return type equals the proposition to be proved. The type checker verifies it. This is the same
operation as ordinary type checking.

## Proposal

### 1. `{}` Is the Proof Space: Types as Assertions, Verification as Type Checking

YaoXiang's `{}` is the compile-time proof space. Everything inside is an assertion; the compiler
guarantees that each is `True` — either proven automatically or provided by the programmer as a
proof function.

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
#          Parameter in signature  Only assertions in {}
#          Compiler validates x > 0 at compile time when called

List: (T: Type) -> Type = { data: Array(T) }
#      ^^^^^^^^              ^^^^^^^^^^^^^^^
#      Parameter in signature  Compiler verifies type_of(T) == Type, type_of(data) == Array(T)
```

Same pattern: `name: (params) -> Type = { assertions }`. The compiler does not distinguish "type
assertions" from "value assertions" — both are evaluation targets in the proof pipeline.

**Loop invariants do not need to be written separately. Type annotations on variables are the
Floyd–Hoare invariants.**

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # Annotation references i — tells the compiler s's type depends on i
    mut i: UpTo(arr.len) = 0     # At initialization i=0, verify: 0 == sum(arr[0..0]) → True
    while i < arr.len {
        s += arr[i]  # Compiler verifies: s_new == sum(arr[0..i+1])
        i += 1       # i changes → triggers re-verification of s dependency: s satisfies SumUpTo(arr, i_new)
    }
    return s  # s: SumUpTo(arr, arr.len) = sum(arr[0..arr.len])
}
```

The compiler generates one verification condition for the loop body — induction hypothesis (type
annotation) → assignment operation → whether the new value satisfies the type annotation. Once the
proof pipeline validates the induction step, all iterations are covered automatically. No
`: decreases`, no `: Invariant`, no inductive proof needed — the compiler decomposes induction into
a local VC per assignment.

### 2. Pre/Postconditions: Compile-time Predicates on Parameter and Return Types

Drop RFC-022's `//! requires`/`//! ensures`. Compile-time predicates act as type annotations on
parameters or return values.

**On the parameter side, it's a function call.** A compile-time predicate is a function that returns
`Type`; on the parameter side, you simply call it — just like `factorial(5)`. The return side
introduces a new concept: the return-value parameter.

```yaoxiang
# Precondition: explicit call to compile-time predicate in the parameter type
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b
#                       ^^^^^^^^^^  b is the current parameter name, passed to Positive
#                       Compiler extracts the actual argument value at the call site, substitutes b, verifies Positive(arg)
#                       e.g.: divide(10, 2) → verify Positive(2) = { 2 > 0 } → True
#                       e.g.: divide(10, 0) → verify Positive(0) = { 0 > 0 } → False → compile error

# Postcondition: return-value parameter + compile-time predicate
IsMax: (T: Ord, arr: Array(T), result: T) -> Type = {
    forall j in 0..arr.len: result >= arr[j]
}

NonEmpty: (arr: Array(T)) -> Type = { arr.len > 0 }

max: (T: Ord) -> ((arr: NonEmpty(arr))) -> (result: IsMax(T, arr, result)) = {
#                                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
#                                            result is the return-value parameter, value provided by return
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
  an argument. Function-call syntax, zero implicit.
- **Return side**: `-> (result: IsMax(T, arr, result))` — `result` is the return-value parameter,
  value provided by the `return` statement. `result` only exists in the type signature, is only
  referenced by the predicate, does not enter the function body scope, does not appear at the
  caller.
- **Return-value parameter is optional**: omit it when there is no postcondition, and the signature
  is exactly like a normal function (`-> Int`).
- **Uniformity**: parameter and return-value parameters are the same concept —
  `paramName: predicateCall(args)`, the only difference being whether the value is supplied by the
  caller or by `return`.

> **Implementation note (2026-10-02)**: Recognition of return-position formal parameters is
> **strictly by the declared name** — the `r` in `-> (r: P(...))` is the unique return-value formal.
> Other **out-of-scope** free variables in the constraint are no longer substituted with the return
> value, but are reported as undefined identifiers (actual test:
> `g: () -> (r: Eq2(m, r)) = { return 7 }` (with `m` undefined) →
> `error [E1001] Unknown variable: 'm'`). The predicate call's actual arguments are truly
> substituted into the constraint: after substitution, `P(r + 100)` becomes "return value + 100",
> not "return value".
>
> **Known gap (2026-10-02)**: **Multi-parameter predicates with symbolic arguments on the return
> side are not yet supported.** Trigger form: `f: (b: Int) -> (r: SumUpTo(b, r)) = { return b * 2 }`
> (with `SumUpTo: (n: Int, s: Int) -> Type = { s == n * 2 }`) →
> `error [E2031] … the refinement type constraint of 'r': SumUpTo(b, (b * 2)) cannot be statically proven`.
> Workarounds: change the argument to a compile-time-foldable literal (e.g. `SumUpTo(3, r)`), or
> provide a proof function returning `Type` for that predicate; the gating condition is that the
> proof kernel supports "multi-parameter predicate + symbolic argument" argument substitution. See
> [RFC-027a](../review/027a-termination-explicit-measure.md) §Implementation Landing Record
> (2026-10-02, this round's hardening).

### 3. Path-Condition Propagation: Compile-time Verification of Runtime Values

When a compile-time predicate is used in a binding position, the arguments are passed explicitly by
the programmer. When runtime values enter refinement-type parameters, the compiler verifies through
path-condition collection and SMT implication — no need for the programmer to pass a proof
explicitly.

#### 3.1 Explicit Function Call

When a compile-time predicate is used in a binding position, the arguments are passed explicitly by
the programmer — it's a function call, zero implicit.

`Positive: (x: Int) -> Type = { x > 0 }` is a compile-time predicate constructor. When it appears in
a binding position (parameter declaration, variable declaration, return type), the programmer
explicitly passes an already-bound variable name:

```yaoxiang
b: Positive(b)
// b is already declared as the current formal, Positive(b) is a function call
// After normalization: b: { b > 0 }
```

The compiler does not implicitly fill in arguments — `b: Positive(b)` is just a function call, the
same as `f(5)`. `b` is bound as a parameter name, and its type annotation `Positive(b)` references
`b` itself — this is the standard dependent-type pattern, not an implicit expansion rule.

**Unified with RFC-010 `self`**: RFC-010 establishes that `self` is not a keyword, just a
conventional parameter name ("writing `p`, `this`, `x` has exactly the same effect").
`b: Positive(b)` shares the same mechanism — the parameter name can be referenced in the type
annotation. `self` appears in the position of `self: Point`, `b` appears in the position of
`b: Positive(b)`; both type annotations reference the parameter itself. The difference is only the
complexity of the annotation, the mechanism is identical — once a name is bound, the type can depend
on that name.

The return type uses explicit function calls in the same way:

```yaoxiang
Sorted: (arr: Array(T)) -> Type = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }

sort: (arr: Array(T)) -> (result: Sorted(result)) = { ... }
//                        ^^^^^^^^^^^^^^^^^^^^^^^
//                        result is the return-value formal, Sorted(result) is a function call
//                        Compiler substitutes the return value into result at the return point, verifies Sorted(return-value)
```

The same applies to local variable declarations:

```yaoxiang
let x: Positive(x) = 5
// x bound to 5, Positive(5) → { 5 > 0 } → True → pass

// let y: Positive(y) = 0
// y bound to 0, Positive(0) → { 0 > 0 } → False → compile error
```

#### 3.2 Path-Condition Collection

When a runtime value appears in a conditional branch, the compiler automatically collects path
conditions, forming the current scope's **assumption set**. These assumptions participate in
verification as background knowledge for compile-time `Bool` evaluation.

```yaoxiang
if y > 0 {
    // Compiler automatically gains the assumption in this branch: { y > 0 }
    let result = divide(x, y)
    // Verification condition: (y > 0) ⇒ (y > 0)
    // Proof pipeline judges implication holds → Proved
} else {
// Branch assumption: { !(y > 0) }
// If divide(x, y) is called, verification condition is !(y > 0) ⇒ y > 0
    // Proof pipeline judges implication does not hold → Disproved
}
```

This is not the compiler hard-coding a special pattern — this is the natural behavior of the
compile-time proof pipeline. Each type-check call site sends to the pipeline:

```
{background assumptions} ⇒ {verification target}
```

The proof pipeline judges implication. `Proved` → pass, `Disproved` → compile error +
counterexample, `Unproven` → compile error + open proposition. Background assumptions come from the
path conditions at the current program point.

#### 3.3 Assumption Stack

When the compiler analyzes control flow, it maintains an assumption set for each basic block:

- **if-guard**: `if y > 0` → true branch pushes `y > 0`, false branch pushes `!(y > 0)` (if `else`
  is used)
- **match pattern**: `if let Some(v) = opt` → branch pushes `opt == Some(v)`
- **Logical conjunction**: `if x > 0 and y < 10` → branch pushes `x > 0` and `y < 10`
- **Function precondition**: when calling `divide(a, b)`, the evidence that `b` satisfies `Positive`
  must come either from the current assumptions, or from the argument's own refinement-type
  annotation (if `b` is annotated as `Positive`, its type carries `b > 0`)
- **Assignment**: when `let z = y`, the existing refinement conditions on `y` propagate to `z`

All assumptions enter the compile-time proof pipeline. When entering the SMT-accelerated path, they
are translated into SMT-LIB background assertions.

#### 3.4 No Static Evidence Means Compile Error

If the programmer directly writes:

```yaoxiang
divide_user_input: (x: Int, y: Int) -> Int = divide(x, y)
```

The current program point has no assumption `y > 0`, and the argument `y` itself has no `Positive`
annotation. The verification condition is:

```
{} ⇒ { y > 0 }
```

The pipeline returns `Disproved` (implication does not hold) → compile error:

> Unable to prove that parameter `b` satisfies `Positive` in the call to `divide`. `y` comes from
> function input, with no proven bound. Consider guarding the call with an `if` branch:
> `if y > 0 { divide(x, y) }`.

YaoXiang does not accept runtime values directly entering refinement-type parameters without
providing static evidence. This is not a limitation — it is the core of the hard-safety philosophy.
Any code the compiler cannot statically prove must not pass compilation.

#### 3.5 Relationship to the Unified Pipeline

Path-condition propagation is not an extra mechanism. It is a direct extension of the compile-time
proof pipeline to control-flow analysis:

| Stage                             | Responsibility                                                                                                                                        |
| --------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| Path-condition collection         | Compiler's control-flow analysis stage, annotating each basic block with its assumption set                                                           |
| Verification-condition generation | When a type constraint needs verification, merge path conditions + argument type info                                                                 |
| Proof-pipeline evaluation         | Compiler kernel → SMT acceleration → produce `Proved` / `Disproved` / `Unproven`                                                                      |
| Result                            | `Proved` → pass; `Disproved` → compile error + counterexample; `Unproven` → compile error + open proposition (programmer may supply a proof function) |

No new components. No special rules. Path conditions are the background knowledge of the proof
pipeline — sharing the same pipeline and budget system as type equality and borrow constraints.

### 4. The Compile-time Proof Pipeline

All compile-time checks share the same pipeline. The core operation of the pipeline is **type
checking** — checking whether the type of a proof term equals the proposition to be proved.
Everything is type checking.

```
Compiler encounters a Bool expression needing evaluation (i.e., needs to construct a proof term)
        │
        ├── Type equality (T1 == T2)
        │   → Compiler judges directly (structural equivalence)
        │
        ├── Token-conflict condition (!conflicting(tokens))
        │   → Flow-sensitive liveness analysis (Dup/Linear property tracking)
        │
        ├── Dependent-type reduction (n + m simplification)
        │   → Compile-time term rewriting system (βδι-reduction)
        │
        ├── Compile-time predicates (x > 0, forall...)
        │   → Compiler kernel + SMT accelerator module
        │
        └── Hoare-logic implication (P ⇒ Q)
            → Compiler + SMT accelerator module
                    │
                    ▼
             ┌──────────┐
             │ Proved   │  → Compilation passes
             │ Disproved│  → Compile error + counterexample
             │ Unproven │  → Compile error + open proposition
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

Compile-time evaluation returns three results — an inevitable conclusion of the halting problem, and
a natural partition of proof theory:

```
eval_compile_time : BoolExpr → Proved | Disproved(Model) | Unproven
```

- **Proved** → Halts, proof term constructed, type check passes. Compilation continues.
- **Disproved(M)** → Halts, counterexample M exists. Compile error + counterexample + source
  location.
- **Unproven** → Within the given resource limit, no proof was constructed. Compile error + open
  proposition + budget consumption report.

**Unproven ≠ False.** The compiler saying "I cannot prove it" is not equivalent to the proposition
being false — it just exceeds the current automatic-proving capability. This is honesty, not a
defect.

Hard budget limits are the engineering answer to the halting problem. No knobs — providing a knob
would be asking the user "do you think your program will halt"; the user doesn't know, and the
compiler doesn't know either.

#### 4.2 After Unproven: The Programmer Writes a Proof

When the compiler returns `Unproven`, the programmer can write a **proof function** — a YaoXiang
function whose return type equals the proposition to be proved. The type checker verifies this
function — exactly the same mechanism as verifying `add(a, b): Int`.

```
Proposition = Type
Proof       = Program (a value of that type)
Verification= Type check (the only trust root)
```

The SMT solver is not an independent trust boundary — it is the **type checker's accelerator
module**. The SMT helps find proofs, but it is the type checker that verifies them. When SMT returns
`unsat`, the compiler reconstructs its result as a proof term verifiable by the type checker. If
reconstruction fails (the SMT's reasoning steps go beyond the compiler kernel's inference rules), it
falls back to `Unproven` — the programmer can then manually write a proof function.

```yaoxiang
# Proposition: a refinement property the compiler cannot prove automatically
FirstIsMin: (T: Ord, arr: Sorted(T)) -> Type = {
    forall i in 0..arr.len: arr[0] <= arr[i]
}

# Proof: programmer writes a function whose return type is the above proposition
# The type checker verifies this function — exactly the same as verifying add(a,b): Int
first_is_min: (T: Ord, arr: Sorted(T)) -> FirstIsMin(T, arr) = {
    # Compiler verifies here: the function body's type = FirstIsMin(T, arr)
    ...
}
```

No AI required, no exporting to Coq, no new concepts. **A property the compiler cannot automatically
prove → programmer writes a proof in YaoXiang code → type checker verifies it.** The whole process
is a smooth gradient — the compiler handles the easy proofs, leaving the hard ones for the human.

#### 4.3 Layered Dependencies within the Pipeline

The evaluators above share the same interface but have an evaluation order. Type equality is a
prerequisite for all subsequent analyses; ownership/token checking depends on type information;
refinement-predicate verification depends on the first two layers' results. The compiler evaluates
layer by layer; expressions failing at lower layers do not enter upper layers — avoiding wasting
solver budget on type-incorrect programs.

```
Evaluation order (one pipeline, layered scheduling)
├── Layer 0: Type equality (T1 == T2)
│   └── Structural unification → if fails, subsequent work is meaningless, return Disproved directly
├── Layer 1: Ownership/token conflicts
│   └── Flow-sensitive liveness analysis → if fails, memory safety does not hold, return Disproved directly
└── Layer 2: Refinement predicates / Hoare implication
    └── Compiler kernel → SMT acceleration → produce Proved / Disproved / Unproven
```

Each layer still returns `Proved / Disproved / Unproven`, sharing the same interface and the same
budget system.

### 5. Three-Layer Function Unification

| Layer                  | Timing       | Input      | Output | Example                                        |
| ---------------------- | ------------ | ---------- | ------ | ---------------------------------------------- |
| Value-level function   | Runtime      | Value      | Value  | `add: (a: Int, b: Int) -> Int = a + b`         |
| Type constructor       | Compile time | Type/Value | `Type` | `List: (T: Type) -> Type = { data: Array(T) }` |
| Compile-time predicate | Compile time | Value      | `Type` | `Positive: (x: Int) -> Type = { x > 0 }`       |

All use the same `name: type = value` syntax. Compile-time predicates and type constructors go
through the same compile-time proof pipeline — `{}` is the proof space.

### 6. Loops: Floyd–Hoare Verification-Condition Generation

Loops do not need separate `: Invariant(...)` or `: decreases(...)` annotations. Compile-time
predicate type annotations on variables define Floyd–Hoare-style assertions — the compiler generates
verification conditions from the type annotations, and the proof pipeline checks whether each
assignment preserves the type.

Core mechanism: each assignment operation corresponds to a Hoare triple `{P} x := e {Q}`, with
verification condition `P ⇒ Q[e/x]`. The compiler generates one verification condition for the loop
body — once the proof pipeline validates the induction step, all iterations are automatically
covered.

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
        #   Substitute s_new = s_old + arr[i]:
        #     Need s_old + arr[i] == sum(arr[0..i+1])
        #     From induction hypothesis s_old == sum(arr[0..i]), add arr[i] on both sides:
        #     sum(arr[0..i]) + arr[i] == sum(arr[0..i+1])
        #   Compiler + SMT: linear arithmetic, millisecond-level → Proved
        #
        # i += 1:
        #   i changes → s's type annotation references i in the dependency graph → triggers re-verification
        #   New verification target: s satisfies SumUpTo(arr, i_new)
        #   i.e. s == sum(arr[0..i_new]), guaranteed by the previous step → Proved
        s += arr[i]
        i += 1
    }
    return s  # At this point s: SumUpTo(arr, arr.len), i.e. s == sum(arr[0..arr.len])
}
```

Loop invariants are the type annotations on variables — programmers write types, the compiler checks
the induction step. The compiler does not need to "discover" invariants, nor does it need to "do
induction automatically" — it decomposes the inductive proof into a local verification condition per
assignment operation, and hands them to the proof pipeline to divide and conquer.

#### 6.1 Dependency Tracking: Dependent Types on Mutable Variables

The prerequisite for the above mechanism is that the compiler knows that `s`'s type annotation
`SumUpTo(arr, i)` references `i` — when `i` changes, `s`'s type constraint also changes. This
requires the compiler to maintain a **type-dependency graph between variables**.

**Data structure**:

```
TypeDepGraph: Map<VarName, Set<VarName>>
# Key is the depended-on variable, value is the set of variables whose type annotation references that variable
# e.g.: { i: {s}, j: {s, t}, ... }
```

**Construction**: when processing `mut v: Pred(... x ...) = init`, the type checker parses the
free-variable references in `Pred(...)`'s arguments. If the arguments reference another mutable
variable `x` in the current scope, it records `x → v` in the dependency graph.

**Trigger**: when the depended-on variable `x` is assigned, the compiler:

1. Looks up all variables in the dependency graph that depend on `x`: `{v₁, v₂, ...}`
2. For each `v`, generates a verification condition: does `v`'s current value satisfy the updated
   type `Pred(... x_new ...)`?
3. Sends the VC to the proof pipeline

**Assignment-order sensitivity**: dependency tracking naturally enforces the correct assignment
order. Take `SumUpTo(arr, i)` as an example:

```yaoxiang
# Correct order
s += arr[i]   # s_new satisfies SumUpTo(arr, i+1)
i += 1        # i changes → re-verify s satisfies SumUpTo(arr, i_new) → True

# Wrong order — compiler rejects
i += 1        # i changes → re-verify s satisfies SumUpTo(arr, i_new)
              # s not yet updated, s_old == sum(arr[0..i_old]) ≠ sum(arr[0..i_new])
              # → Compile error: variable s does not satisfy type SumUpTo(arr, i_new)
s += arr[i]   # Unreachable
```

**Composite dependencies**: a variable can depend on multiple variables. The type annotation
`{ v: Int; v == x + y }` depends on both `x` and `y` — any change to either triggers
re-verification.

**Relationship to the proof pipeline**: dependency tracking is the trigger for VC generation, not an
independent verification mechanism. It answers "when do we need to generate a VC" — the proof
pipeline answers "does the VC hold".

### 7. Termination Checking

**Scope of application: refinement types.** Termination is not an independent switch; it is part of
the **verification mode**: once a type is refined (`Refined { base, constraint }`), the computation
annotated by that type enters verification mode, which requires termination; ordinary, non-refined
types do not enter verification mode and generate no termination obligations.

This yields two corollaries:

- **Loops**: a bare `while` does not enter verification mode; when a measure variable carries a
  refinement annotation (e.g. `i: UpTo(n)`), it enters verification mode and must prove termination.
- **Recursion**: when a function's signature carries a refinement (parameter refinement or a return
  type containing a refinement), it enters verification mode, requiring each recursive call site to
  strictly decrease the measure.

Within the mode, fully automatic takes priority: the compiler first attempts to automatically
discover a measure, proving it if possible; if it cannot discover one and no explicit measure is
provided, compile error. There is no **annotation syntax** escape hatch — both the measure and the
termination proposition live in the type position, with no new syntax like `decreases`. The forms
for measures and explicit fallbacks are described in §6.9.

#### 6.1 Design Principles

The compiler automatically extracts the information needed for the termination proof from two
sources:

1. **Variable type annotations**: boundary constraints in refinement types (e.g. `UpTo(n)` gives
   upper bound `n` and lower bound `0`)
2. **Loop body operations**: the operations applied to variables on each iteration

The compiler tries four measure-synthesis strategies in priority order, stopping as soon as one
succeeds. The four strategies are a **bounded template sequence for measure discovery**, with the
input being refinement constraints (strategies 1–4 all start from "variables with bounded types"),
not "compile-time-evaluated code"; they are the automatic and manual sides of the same thing as the
explicit measure in §6.9.

> **Measure discovery is exploration, not inference.** The exploration only enumerates templates
> (linear rank, violation counting, bounded pattern, multiplicative scaling); it does not guarantee
> a solution exists — general measure inference is undecidable (reduces to the halting problem).
> Therefore, cases outside the templates must allow the programmer to provide an explicit measure
> (§6.9), otherwise compile error.

#### 6.2 Strategy 1: Automatic Linear-Rank-Function Synthesis

When variables have linear bound annotations, the compiler enumerates candidate linear measures and
verifies them with SMT.

```
Input:
  Variables v₁: UpTo(u₁), v₂: UpTo(u₂), ... (variables with upper/lower bounds)
  Loop condition cond
  Set of assignments in the loop body

Algorithm:
  1. Extract each variable's bounds from type annotations: [low_i, high_i]
  2. Enumerate candidate measures: v_i, u_i - v_i, v_i - v_j, etc., linear combinations
  3. For each candidate measure m:
     - SMT verify m ≥ 0 (derived from type bounds)
     - For each execution path in the loop body, SMT verify m' < m (strictly decreasing)
  4. Find a linear combination that satisfies the conditions → termination proven
```

Coverage: any loop where a variable is assigned a linear expression (`v = a·v + b`) and has a
bounded type annotation. Includes `i += const`, `i -= const`, and binary-search-style interval
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

#### 6.3 Strategy 2: Predicate-Violation Counting — Automatically Extracting Measures from Target Types <span style="color:orange">[Experimental Strategy]</span>

> ⚠️ **Current status: experimental strategy; whether to include it depends on actual feasibility at
> Phase 3 implementation time.** This strategy works for adjacent-swap operations (bubble sort,
> insertion sort); it cannot automatically prove non-adjacent operations (quicksort partition,
> heapsort sift-down). See the coverage boundary in the table below. If Phase 3 verification turns
> out infeasible, this strategy will be removed or downgraded to future work.

Core insight: **the specifications the user writes are material for compiler reasoning.** The
compiler does not need a built-in notion of "what sorting is" — it reads the definition of `Sorted`
and automatically extracts the measure from the definition.

```
Input:
  Target type: Sorted(arr) = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }
  Loop body operation: adjacent-element swap

Algorithm:
  1. Parse the predicate definition: forall i in range: cond(i, arr)
  2. Automatically generate measure: violation_count = |{ i | ¬cond(i, arr) }|
  3. Analyze the operation's effect on the measure:
     - Adjacent swap arr[j], arr[j+1] = arr[j+1], arr[j]
     - Affects only the three pairs at indices j-1, j, j+1
     - If arr[j] > arr[j+1] (predicate violated), after the swap this pair satisfies the predicate
     - violation_count decreases by at least 1
  4. Upper bound: n·(n-1)/2 (maximum number of adjacent inversions), lower bound: 0
  → Termination proven
```

**Current coverage**:

| Algorithm      | Operation Pattern | Strategy 2 Provable? | Reason                                               |
| -------------- | ----------------- | :------------------: | ---------------------------------------------------- |
| Bubble sort    | Adjacent swap     |          ✅          | violation_count strictly decreases per swap          |
| Insertion sort | Adjacent shift    |          ✅          | Each shift eliminates one violating pair             |
| Selection sort | Non-adjacent swap |          ❌          | A single swap may increase violation_count           |
| Quicksort      | Partition         |          ❌          | Non-adjacent swaps, no guaranteed monotonic decrease |
| Heapsort       | sift-down         |          ❌          | Tree-shaped operation, violation_count not monotonic |

**Complementary strategy**: for quicksort, the `low < high` interval contraction is covered by
Strategy 1 (linear rank function) — the outer partition recursion halves the interval each time.
Strategies 1 and 2 complement each other, and most practical algorithms' termination can be proven
by one of them. But generalizing Strategy 2 (non-adjacent operations, tree-shaped operations)
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

#### 6.4 Strategy 3: Bounded Increasing/Decreasing Pattern

`v += const` (positive constant), variable has an upper-bound type annotation → measure
`upper_bound - v` decreases by `const` each time, with lower bound 0. This is a degenerate case of
Strategy 1, handled quickly at the front.

#### 6.5 Strategy 4: Multiplicative-Scaling Measure Template

`v *= const` (const > 1), variable has upper- and lower-bound type annotations. The compiler has a
built-in logarithmic measure template `ceil(log_const(upper/v))`; each multiplication by `const`
decreases the measure by 1.

```yaoxiang
mut i: Positive(i) = 1
while i < n {
    # Compiler automatically derives: measure ceil(log₂(n/i)), each multiplication by 2 decreases the measure by 1
    i *= 2
}
```

#### 6.6 Separation of Termination and Correctness

Termination proof and correctness proof are independent:

- **Termination**: the four strategies above automatically prove the loop exits in finitely many
  steps; if discovery fails, the programmer provides a measure in the type position (§6.9)
- **Correctness**: whether the loop body progresses toward the target type, checked by the
  compile-time proof pipeline via verification conditions

Both pass → compilation passes. Termination proven but correctness fails → compile error +
counterexample. Correctness proven but termination not provable → compile error pointing to the
unanalyzable variable or operation. Both fail → compile error reporting the two failure reasons
separately.

#### 6.7 Termination Checking for Recursive Functions

For a recursive function with a refined signature, the compiler checks that the formal parameters
strictly decrease at each recursive call site:

```yaoxiang
gcd: (a: Int, b: NonNegative(b)) -> Int = {
    if b == 0 { return a }
    return gcd(b, a % b) // Compiler explores: the well-ordering measure on (b, a % b) decreases → terminates
}
```

Formal-parameter decrease is the strongest automatic path (structural recursion). If discovery
fails, the programmer explicitly provides a measure in the type position (§6.9).

#### 6.8 Hard Boundary

`i = f(i)` where `f` is non-invertible, non-closed, and preserves no monotonicity — mathematically,
automatic termination proof is impossible. Compile error:

> This loop cannot be automatically proven to terminate. The loop variable depends on the
> unanalyzable function `f`. Please use an iteration pattern that the compiler can analyze, or bind
> this loop with a name and provide a measure in the type position (§6.9).

This is not a compiler failure. Any code that cannot be statically proven safe must not pass
compilation. Even with an explicit measure, the SMT must judge it true to pass — a wrongly written
measure will be rejected by a counterexample; humans can only fail to prove, not prove wrong.

#### 6.9 Explicit Measure: `Terminates`

When automatic discovery fails to find a measure, the programmer writes the measure in the **type
position** — the same mechanism as `Positive(b)` and `IsMax(T, arr, result)` (predicate
application), with zero new syntax:

```yaoxiang
// Measure: an ordinary function, unit-testable, reusable, does not participate in runtime
gcd_measure: (a: Int, b: Int) -> Int = { b }

// The termination component lives in the function's own type position
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// Loop: the binding name is the anchor; the measure obtains values in the name's scope
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n {
        i = i + 1
        i
    }
    return acc
}
```

The reason for the loop form: `Terminates(m)` refines exactly the value type of the loop body's tail
expression; the anchor is provided by the binding name `acc` — the loop is thus denotable, with no
"anonymous construct that cannot be denoted" blind spot.

> **The body tail must yield a value**: a loop's value = the loop-body block's value (§2.9), and a
> block's value = its tail expression. If the body tail is an assignment statement (e.g.
> `{ i = i + 1 }`), the block value is `Void`, so `return acc` is returning `Void` from `-> Int`,
> which is a type error. Hence the example's body tail explicitly yields `i`. This does not change
> the measure's semantics — the measure `n - i` is a **state expression**, independent of the loop's
> value type.

> **Implementation note (2026-10-02)**: the loop example in this section is **type-side** sound
> (`check` 0 error, back-edge obligation `Proved`), but the **runtime** value-taking of `while` as a
> value is still defective: the value obtained by the binding is `Void`, and using that value
> reports `E6007 Runtime error: type mismatch in comparison Eq: Void vs Int(4)`; zero-iteration
> (`loop(0)`) hits the same issue; the impact surface is "`while` as a value always hits this", not
> limited to the `Terminates` form. For the minimal reproduction, impact surface, and the unblocking
> condition, see [RFC-027a](../review/027a-termination-explicit-measure.md) §Example Loop "Known
> Defect D6". Also: termination Strategy 1 (linear rank function) is re-enabled this round, so the
> order "automatic discovery fails before explicit measure is written" is unchanged — explicit
> measure remains the fallback after discovery fails.

**`Terminates` is a built-in predicate**, alongside `Int`, `Never`, and other core primitives. It is
the **only predicate for which the compiler writes the function body on behalf of the user** — its
assertion ("the measure strictly decreases at every recursive call site / loop back edge") lives in
the computational structure, which user-written predicates cannot reference into, so it cannot be
expressed by the predicate-definition syntax in the Syntax section. The built-in side converges to
this single name.

**Arity**. `Terminates(FnType, m)` and `Terminates(m)` are two arities of the same predicate, not
two distinct constructs:

| Form                    | Anchor                             | Use                                                             |
| ----------------------- | ---------------------------------- | --------------------------------------------------------------- |
| `Terminates(m)`         | The name of the containing binding | Self-recursive functions, loops — default form                  |
| `Terminates(FnType, m)` | Explicit function type             | Mutual recursion and other cases where the anchor is not unique |

Both are essentially the same: the termination obligation always falls on "the computation annotated
by the type position of the refinement".

**Measures do not restrict the return type.** A measure can be an expression of any type (not forced
to be a natural number); "strictly decreases" on it is given by the well-ordering available on that
type. Whether the measure is well-founded (e.g. whether it is `>= 0` when returning `Int`) is an
**independent obligation**, also handed to refinement inference or SMT, just like the decrease
obligation; when neither can deduce it, the diagnosis does **not directly reject**, but instead
suggests a direction to check (whether the measure's lower bound holds, whether the recursive
parameter actually moves in that direction).

**Relationship to automatic discovery**: the explicit measure is not a separate pipeline, but the
input after discovery fails. Once a measure is given, the same SMT verifies the decrease and
well-foundedness; if it does not hold, an error is reported with a counterexample.

For obligation generation, judgment pipeline, diagnosis direction, shared measure for mutual
recursion (SCC), and other landing mechanisms, see
[RFC-027a: Explicit Measure for Termination Checking](../review/027a-termination-explicit-measure.md).

### 8. SMT Solver: Accelerator Module of the Type Checker

In traditional languages, the SMT solver is an external tool (e.g. F\* calling Z3, Dafny calling
Z3). In YaoXiang, it is an **accelerator module of the type checker** — invoked only when the
compiler kernel itself cannot directly judge. The SMT helps find proofs, but the type checker
verifies them.

**Trust model**: the type checker is the sole trust root. The SMT solver is an accelerator module —
it helps find proofs, but SMT is not an independent trust boundary. The compiler trusts Z3's `unsat`
results (consistent with the F\*/Dafny line — the probability of Z3 being wrong is lower than the
compiler's own bug rate; this is a pragmatic engineering choice). The real unreliability control is
at the SMT-translation layer — if the translation has a bug, the compiler will be exposed in other
tests.

**Interface**: the compiler's internal translation targets the SMT-LIB 2.6 standard format, not a
specific solver API. SMT-LIB is an ISO standard, natively supported by Z3, CVC5, MathSAT, Yices.

**Default backend**: Z3 (MIT license, the most widely documented and community-validated). CVC5 as
an SMT-LIB-compatible alternative — users can switch at compile time via a compiler flag.

No "general solver abstraction layer" — SMT-LIB is the abstraction layer. If CVC5 makes a
breakthrough in a particular theory in the future, switching requires only swapping the binary, no
compiler-code changes.

```
Compile-time Bool expression
        │
        ├── Compiler kernel can directly judge (structural equivalence, simple arithmetic,
        │   trivial formulas after constant folding)
        │   → Return Proved / Disproved directly
        │
        └── Compiler kernel cannot directly judge (quantifiers, symbolic variables)
            → Dependent-type pre-reduction (factorial(5) → 120)
            → Translate into SMT-LIB format
            → Send to Z3/CVC5 (with budget limit)
            → Return value: unsat → Proved  │  sat + model → Disproved  │  unknown → Unproven
```

**Solver budget — hard limit, like stack depth**:

| Budget Dimension               | Default | Description                                                                                                                                               |
| ------------------------------ | ------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Solving steps                  | 10,000  | Z3 typically uses within a hundred steps for linear arithmetic. 10,000 steps covers 99% of real predicates.                                               |
| Time                           | 100ms   | A single predicate over 100ms means the user is writing a compile-time program, not a type annotation. 100ms × 50 predicates = 5s compilation time limit. |
| Quantifier instantiation depth | 3       | Three nested quantifier levels cover real patterns. Beyond three levels is likely a logic exercise.                                                       |

Over budget returns `Unproven`, compile error + predicate location + consumption. No degradation, no
runtime check, no silent pass.

**Why this is actually feasible**: in practice 95% of real predicates are linear arithmetic —
`x > 0`, `arr.len > 0`, `0 <= idx < arr.len` — all in decidable fragments, where SMT solvers return
in milliseconds. For the rare complex predicate that exceeds the budget, the programmer writes a
proof function.

Dependent types get a pre-reduction pass before calling SMT: `factorial(5)` directly evaluates at
compile time to `120`; `append([1,2], [3])` directly evaluates to `[1,2,3]`. These deterministic
value computations do not consume SMT budget.

The programmer does not need to know SMT exists. The mental model is: **the compiler can prove it,
it passes; the compiler cannot, it reports an error — and if the compiler doesn't know, you can
write a function to prove it to the compiler.**

### 9. Compose Compile-time Predicates

Compile-time predicates are functions returning `Type`; composition is naturally achieved through
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

#### 9.4 Loop: Compiler-Generated VCs

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

### 11. Dispatch Pipeline: Unified Dispatch for Compile-time and Runtime

`assert` and `Assert` are two sides of the same refinement-type primitive. The dispatch pipeline
`dispatch` automatically decides between compile-time proving and runtime checking based on
**whether the predicate's free variables are reachable at compile time**:

| Criterion                                                                                    | Mode            | Behavior                                                                                             |
| -------------------------------------------------------------------------------------------- | --------------- | ---------------------------------------------------------------------------------------------------- |
| All free variables known at compile time (generic parameters, compile-time constants)        | **CompileTime** | Enter the proof pipeline: `Proved` → erase, `Disproved` → compile error, `Unknown` → require a proof |
| Some free variables come from runtime (function parameters, external input, `mut` variables) | **Runtime**     | Insert a runtime check, and inject the refinement fact into the flow-sensitive assumption set Γ      |

**Key**: "cannot judge" ≠ "disproved". In CompileTime mode, `Unknown` requires a proof (no silent
degradation); in Runtime mode, the proposition is not even true-or-false at compile time — no matter
how strong the prover is, it cannot write a tautological proof for "the user may have entered a
negative number", and a runtime check is the only sound choice. This is not that the prover is not
strong enough; it is theoretically necessary.

### 12. Flow-Sensitive Assumption Set Γ: Strongest Postcondition Propagation

The compiler maintains a flow-sensitive assumption set Γ, tracking the propositions known to hold at
each control-flow point.

**SP (Strongest Postcondition) propagation**:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
```

**`mut`-variable kill set**: after a `mut` variable is reassigned, all assumptions involving that
variable are removed from Γ:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
mut x = x - 5       // Γ = {}  ← x > 0 is killed
```

This is a hard requirement of soundness — when the variable's value changes, old assumptions are
invalid.

**Branch confluence**: when IF/ELSE or match branches merge, Γ takes the intersection of each
branch's assumptions. Only propositions that hold on all paths are carried out of the branch.

### 13. Erasure Model Clarification: Witness Erasure ≠ Check Erasure

The RFC-027 statement that "refinement types are **fully erased** at runtime" refers to **proof
witnesses** — proof terms already verified at compile time generate no runtime code. However, the
**runtime checks** inserted by dispatch in Runtime mode are retained — they are `Bool` checks
executed at the value level, not type-level witnesses.

In summary: witness erasure, check retention. The two are not in conflict, and the original RFC-027
statement remains unchanged.

## Detailed Design

### Syntax Changes

| Before (RFC-022)                      | After (this RFC)                                                                                                                                                      |
| ------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `//! requires: NonEmpty(n) = n > 0`   | Compile-time predicate as a parameter type `(b: Positive(b))`                                                                                                         |
| `//! ensures: ExistsMax(result, arr)` | Return type uses a return-value parameter `-> (result: IsMax(T, arr, result))`                                                                                        |
| `/*! invariant: ... !*/`              | Compile-time predicate type annotation on variables — the Floyd–Hoare invariant                                                                                       |
| `//! decreases: n`                    | Measure written in the refinement-type position (`Terminates`); compiler first tries automatic discovery, and only requires explicit declaration when discovery fails |
| Specifications are comments           | Specifications are the type system                                                                                                                                    |

### Syntax

**Compile-time predicates have no new keyword.** `{}` is the proof space, exactly the same as the
existing type-definition syntax. A compile-time predicate is simply a function returning `Type` —
`name: (params) -> Type = { assertions }`. Usage is simply a function call — `Positive(b)`,
`IsMax(T, arr, result)`.

```bnf
# Compile-time predicate = a function returning Type, with assertions inside {} verified by the compiler
# Uses the existing function/type syntax, no new BNF rules required
predicate ::= identifier ':' params '->' 'Type' '=' '{' assertions '}'
```

**A predicate application's arguments must be in compile-time-constant form** — literals, variables
(bound by name), type applications (recursively extracted), or **compile-time-denotable function
references** (function names). Arguments that cannot be converted to constant expressions report
**E1092**; a mismatch between the number of arguments and the predicate's declared formal parameters
reports **E1093**. Arguments are positionally bound to the formal-parameter list, and the
predicate's arity is determined by its declaration — `Positive(x)` is unary, `IsMax(T, arr, result)`
is ternary, `Terminates(m)` and `Terminates(FnType, m)` are unary and binary — **refinement
constraints are never silently dropped** (previously, non-convertible arguments would cause the
constraint to silently disappear, allowing binding that violates the constraint to silently pass).

**New syntactic concept: return-value parameter** — in `-> (name: Type)`, `name` is the return-value
parameter.

The return-value parameter is the **only one syntactic concept** YaoXiang introduces on top of the
existing function syntax. Its semantics:

- The value of `name` is provided by the `return` statement
- `name` only exists in the type signature, referenced by the postcondition predicate (e.g.
  `-> (result: IsMax(T, arr, result))`)
- `name` does not enter the function body scope, does not appear at the caller
- The return-value parameter is **optional** — when there is no postcondition, the signature is
  exactly like a normal function (`-> Int`), introducing no extra burden

The reason for introducing it: postconditions need to refer to "the value the function is about to
return". Without a return-value parameter, the compiler could only let the predicate reference the
return value through special rules (such as the implicit variable `$result` or `__retval__`). The
return-value parameter makes this reference explicit — it is just a parameter, with the only
difference being that the value is provided by `return` rather than the caller.

**Proof functions** are not a new concept — they are simply YaoXiang functions whose return type is
the proposition to be asserted. When the compiler returns `Unproven`, the programmer provides a
proof function, and the type checker verifies it in exactly the same way as it verifies any
function's return type. No new syntax, no new keywords, no new rules required.

> **The boundary between the two.** The correctness domain (predicate `Unproven`) supplements the
> proof **in the body**: write a function whose return type is the proposition to be proved. The
> termination domain (§6.9) supplements the measure **in the type position**: write
> `Terminates(measure)` as the binding's or function signature's type. The former is "an unprovable
> proposition: write a proof"; the latter is "an undiscoverable measure: explicitly declare it" —
> the mechanism is the same (both are refinement-type applications), but the landing point differs.
> The termination domain **does not need** a `_proof` function.

### Type System Impact

- **Type universe**: compile-time predicates live at the Type₂ layer — functions taking values and
  returning `Type`, at the same level as type constructors
- **Generics interaction**: compile-time predicates can carry generic parameters, e.g.
  `NonEmpty: (T: Type) -> (arr: Array(T)) -> Type`
- **Ownership interaction**: expressions in compile-time predicates obey ownership rules; they can
  only read, not write
- **Type inference**: compile-time-predicate parameters participate in HM type inference

### Runtime Representation

Compile-time predicates are **handled at runtime according to the dispatch result**:

- **CompileTime mode** (all free variables known at compile time): after the proof passes, the
  witness is fully erased. `Positive: (x: Int) -> Type = { x > 0 }` — `b: Positive(5)`'s runtime
  representation is just `Int`. The refinement condition `{ 5 > 0 }` has passed, erased.
- **Runtime mode** (runtime free variables exist): keep the runtime check — a `Bool` check executed
  at the value level, injecting into the flow-sensitive assumption set Γ. See §11 dispatch pipeline
  and §13 erasure-model clarification.

Putting a compile-time predicate in a type position (e.g. `f(x: Positive(x))`) generates no wrapper
type, no extra allocation. But when `x` comes from runtime input, a **runtime `Bool` check will be
inserted**.

**Interaction constraint with `ref`**: compile-time predicates can only reference immutable borrows
or values whose ownership has been transferred. Compile-time predicates referencing a mutable borrow
cannot guarantee at compile time that the verification result still holds at runtime — such usage is
directly reported as a compile error.

### Compiler Changes

1. **Parser**: compile-time predicates use standard function syntax, no extra parsing rules
2. **Compile-time proof pipeline**: unified `Proved` / `Disproved` / `Unproven` return interface,
   automatic strategy selection
3. **SMT accelerator module**: SMT-LIB 2.6 translation layer, default backend Z3, CVC5 alternative
4. **Type-checker kernel**: inference-rule implementation — structural equivalence, βδι-reduction,
   universal quantifier introduction/elimination. This is the sole trust root, and both the SMT and
   programmer proofs are verified through it
5. **Verification-condition generation**: WP/SP calculus + loop-invariant proof obligations
6. **Error reporting**: counterexample formatting + open-proposition report + source-location
   correlation

### Backward Compatibility

- ✅ Code that does not use compile-time predicates is completely unchanged
- ✅ Compile-time predicates have zero runtime overhead in CompileTime mode; in Runtime mode only
  the necessary `Bool` check is kept
- ⚠️ RFC-022's `//!` syntax is no longer supported — but 022 was never implemented, so there is no
  migration burden

## Trade-offs

### Advantages

- **Curry–Howard correspondence fully realized**: types as propositions, programs as proofs,
  `name: Proposition = Proof`
- **Uniformity**: compile-time predicates use exactly the same syntax as ordinary functions, no
  conceptual split
- **SMT transparency**: the programmer does not need to know SMT exists; the mental model is the
  same as type checking
- **Gradual adoption**: start with a single compile-time predicate, then expand coverage gradually
- **Minimum runtime overhead**: zero overhead in CompileTime mode; only the necessary `Bool` check
  remains in Runtime mode

### Disadvantages

- **Compile time**: SMT solving increases compile time, but hard budget limits keep the upper bound
  controllable
- **Limits of automatic proving**: complex predicates beyond first-order linear arithmetic may
  require the programmer to write a proof function. This is not a language defect — it is the
  inevitable conclusion of the halting problem. The compiler honestly reports `Unproven` rather than
  falsely reporting `True` / `False`
- **Learning curve**: writing effective compile-time predicates and proof functions requires
  understanding the basic intuition of the Curry–Howard correspondence
- **Implementation complexity**: unifying the compile-time proof pipeline requires careful design

### Risk Mitigation

- SMT solving-budget hard limits (steps 10,000 / time 100ms / instantiation depth 3); over budget
  returns `Unproven`
- Dependent-type pre-reduction: deterministic value computation is consumed first; SMT only chews
  the non-deterministic part
- `Unproven` is not a dead end: the correctness domain writes a proof function (return type is the
  proposition), the termination domain provides a measure in the type position (§6.9) — both are
  verified by the type checker
- Incremental verification: only verify changed modules
- Clear error messages + counterexample display + budget-consumption report + open proposition +
  suggestions (if the compiler can give them)

## Alternatives

| Alternative                                                        | Why Not Chosen                                                                                                                                                             |
| ------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| RFC-022: `//!` comment-style specifications                        | Specifications and types are split, violating the Curry–Howard correspondence                                                                                              |
| Standalone specification files (e.g. CVL)                          | Specifications separated from code, increasing maintenance cost                                                                                                            |
| Runtime-only assertions                                            | Cannot statically guarantee correctness                                                                                                                                    |
| External proof assistants (e.g. Coq)                               | Disconnected from the compiler, requiring a separate proof language and trust boundary. YaoXiang's choice: proof is YaoXiang code; the type checker is the sole trust root |
| **This proposal: compile-time predicates as first-class citizens** | ✅                                                                                                                                                                         |

## Implementation Strategy

### Phases

| Phase       | Content                                                                                                                                                                |
| ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Phase 1** | Compiler kernel: structural equivalence + βδι-reduction + universal quantifier introduction/elimination. Support simple arithmetic predicates (`x > 0`, `arr.len > 0`) |
| **Phase 2** | SMT-LIB translation layer + Z3/CVC5 integration. Pipeline returns `Proved` / `Disproved` / `Unproven`. On `Unproven`, support the programmer writing a proof function  |
| **Phase 3** | Loop-invariant VC generation + termination checking (four-strategy measure discovery + `Terminates` explicit measure, §6, §6.9)                                        |
| **Phase 4** | Incremental verification + caching + IDE support                                                                                                                       |

### Dependencies

- RFC-010: Unified Type Syntax — compile-time predicates are based on `name: type = value`
- RFC-011: Generic Type System — compile-time predicates can carry generic parameters
- RFC-009: Ownership Model — expressions in compile-time predicates obey ownership rules

## Open Questions

- [x] **wasm32 target Z3 availability**: no Z3 under wasm (all SMT code is excluded via
      `cfg(not(target_arch = "wasm32"))`). Impact surface and degradation direction: -
      `ownership.rs`'s `smt_cut` always returns `false` → back edge crosses → **conservative
      rejection** (sound direction, just narrows less) - Termination-check Strategy 1 and predicate
      Level 2b/3 are entirely skipped

  **Correction (2026-10-01)**: the original text stated these paths are "currently **unreachable**"
  (reason: `predicate_defs` production is never populated, `parser` never produces
  `MonoType::Refined`, `with_solver` production never invoked); this no longer holds — all three
  have landed (predicate-definition registration #377-3; refinement-type normalization; production
  pipeline injects the solver). Therefore the difference between wasm and native is no longer
  limited to `smt_cut` precision:

  | Path                                        | native   | wasm       | Degradation direction               |
  | ------------------------------------------- | -------- | ---------- | ----------------------------------- |
  | Termination-check explicit-measure judgment | judgment | `Unjudged` | do not report E4022 (fewer reports) |
  | Well-foundedness judgment                   | judgment | `Unjudged` | do not report E4022 (fewer reports) |
  | Refinement-predicate SMT implication level  | executes | skipped    | falls to `Unproven`                 |

  All in a **conservative** direction (narrows less, reports less), not affecting soundness. For
  full precision, see issue #376 for the plan and cost (conclusion: prefer a JS-side Z3 instance,
  not linking Z3 into the main wasm — the latter requires switching the emcc build system and the
  artifact grows from 3 MB to ~20 MB).

- [x] **SMT solver selection**: default Z3 (MIT license, most widely validated). CVC5 as the
      SMT-LIB-compatible alternative, switched via a compiler flag. The compiler's internal
      translation targets the SMT-LIB 2.6 standard format — SMT-LIB is the abstraction layer; no
      custom general-solver interface.
- [x] **Specific budget values**: steps 10,000 / time 100ms / quantifier instantiation depth 3.
      Fixed inside the compiler, no knobs. In actual use, if a real use case proves them
      insufficient (not "the user wrote it wrong"), they will be adjusted.
- [x] **Quantifier support scope**: the language level does not limit quantifier order. Compile-time
      predicates accept `Type` parameters — `Type` includes function types — so higher-order
      quantifiers are a natural corollary of the type system, requiring no special syntax. The SMT
      solver can automatically judge first-order quantifiers (`forall` / `exists`, with alternating
      nesting, limited by budget depth 3). Higher-order quantifiers: SMT returns `Unproven`, the
      compiler prompts "this predicate is beyond the scope of automatic proving; please provide a
      proof function". The programmer writes a YaoXiang function whose return type equals the
      proposition — the type checker verifies the function. No external export, no AI, no
      interactive proof mode required. Everything is YaoXiang code; everything is verified by the
      type checker.
- [x] **Counterexample formatting**: source variable names are used directly as SMT variable names
      (with module prefixes to avoid conflicts). When the Z3 model is returned, it is
      reverse-looked-up by variable name. Output format: variable name = concrete value + source
      location + predicate-definition location. No complex mapping layer.
- [x] ~~**Interaction between compile-time predicates and `ref` smart pointers?**~~ → Decision made:
      compile-time predicates only allow immutable borrows or values whose ownership has been
      transferred. Values of mutable borrow cannot appear in compile-time predicates.
- [x] **Extending the `forall`-predicate violation-count measure to non-adjacent operations?** → Not
      extending. The current coverage (adjacent swap, adjacent shift) is complemented by Strategy 1
      (linear rank function) — quicksort's outer interval contraction is covered by Strategy 1,
      heapsort is covered by Strategy 1 (array-index pattern). Loops that cannot have their
      termination proven by any strategy are directly reported as a compile error by the compiler —
      this is the hard-safety philosophy, not a defect. If in the future a real scenario (not
      academic constructions) has an algorithm that no strategy can cover, the discussion will be
      reopened. → **Re-discussion triggered (2026-09-14, #318)**: non-structural recursion
      (gcd-style non-direct-decrease, mutual recursion, merge partition) is exactly the real
      scenario this clause anticipates — the template sequence of measure discovery cannot
      automatically take them down, and they cannot be rewritten into analyzable iteration patterns
      without destroying readability. Conclusion: the hard-safety philosophy remains unchanged (a
      wrongly written measure is still rejected by an SMT counterexample; humans can only fail to
      prove, not prove wrong), but "discovery fails → reject" is relaxed to "discovery fails → the
      programmer can explicitly provide a measure in the type position" — see §6.9.
- [x] **Combinatorial explosion in linear-rank-function enumeration**: the candidate-enumeration
      upper bound is 3 bounded variables. When ≤3, enumerate all linear combinations and verify each
      with SMT. When >3, only try single-variable measures (`v_i`, `u_i - v_i`); on failure,
      directly report a compile error — telling the programmer "the loop has >3 bounded variables;
      the compiler cannot automatically synthesize a multi-variable measure". This is not an
      engineering compromise — it forces the programmer to write simpler loops.

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
│  Under      │  ← Current state: community discussion
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
│ (formal     │    │ (retained   │
│  design)    │    │  in place)  │
└─────────────┘    └─────────────┘
```
