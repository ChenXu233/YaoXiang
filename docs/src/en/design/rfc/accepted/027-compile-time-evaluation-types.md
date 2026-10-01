---
title: 'RFC-027: Compile-time Predicates and Unified Static Verification'
status: 'Accepted'
author: 'ChenXu'
created: '2026-06-07'
updated: '2026-09-14'
impl_status: 'in_progress'
impl_detail:
  'Phase 1-2 complete, Phase 3 partial, Phase 4 partial. The 6-phase assert/Assert unified scheme is
  fully implemented (issues #157-#162 closed): Never type, IsTrue bridging, flow-sensitive Γ + kill
  set, type-level recursion, universe hierarchy weak check, dispatch routing pipeline.'
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
> - [RFC-024: Concurrency Model Based on spawn Blocks](../accepted/024-concurrency-model.md)
>
> **Supersedes**:
> [RFC-022: Hoare Logic Static Verification Support (Specification Annotations and Specification Types)](../deprecated/022-hoare-logic-static-verification.md)
> — deprecated

## Summary

This proposal introduces **compile-time predicates** as first-class citizens in YaoXiang, unifying
all compile-time static verification into a single **proof pipeline**. A compile-time predicate is
not an external specification annotation — it is a function. A function that returns Type, usable at
type positions; the compiler invokes it at compile time and checks the return value. Types are
propositions; compile-time evaluation is proof.

**Core argument**: The only job of type checking at compile time is to construct and verify proof
terms. Type equality, token conflicts, dependent type reduction, compile-time predicate evaluation,
Hoare logic implication — all are different type checks within the compile-time proof pipeline,
sharing the same pipeline. The SMT solver is an acceleration module of the type checker, not a
separate trust boundary. When the compiler returns Unproven, the programmer writes a YaoXiang
function as proof — the type checker verifies it in exactly the same way it verifies any function's
return type. Everything is YaoXiang code; everything is verified by the type checker.

## Motivation

### Why deprecate RFC-022?

RFC-022 designed specifications as `//!` annotation form:

```yaoxiang
max: (T: Ord) -> ((arr: Array(T, n)) -> T) = {
    //! requires: NonEmpty(n) = n > 0          ← This is an annotation independent of types
    //! ensures: ExistsMax(result, arr[0..n])   ← This is an annotation independent of types
}
```

This commits the fundamental error of the Curry-Howard correspondence: **splitting specifications
and types into two layers**. Annotations are not types. Annotations do not participate in type
checking. Annotations are the mental model of "external tools."

The white paper states clearly:

> "No `//!` annotations. No separate specification language. Everything is within the type system."

### Current Problems

- RFC-022's `//!` annotations are external syntax independent of the type system
- Specification types and ordinary types are two systems, causing conceptual redundancy
- The split mode of "verify in Debug Build / ignore in Release Build" breaks unity
- In conventional understanding, the SMT solver is positioned as an external tool — YaoXiang builds
  it in as an acceleration module of the type checker
- Type checking, borrow verification, compile-time predicate checking, and macro expansion each take
  different paths

### The Correct Mental Model

Type checking can be abstracted as a function:

```
verify : Program → Proved | Disproved(Model) | Unproven
```

All compile-time checks — simple type matching, borrow conflict detection, compile-time predicate
verification — are subtasks of this function. They share the same proof pipeline; they differ only
in proof term complexity and construction strategy.

When the compiler returns Unproven, the programmer provides a proof function — its return type
equals the proposition to be proved. The type checker verifies it. This is the same operation as
normal type checking.

## Proposal

### 1. `{}` Is the Proof Space: Types Are Assertions, Verification Is Type Checking

YaoXiang's `{}` is the compile-time proof space. Everything inside is an assertion; the compiler
guarantees each item is True — either proved automatically, or the programmer provides a proof
function.

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
#          Parameters at the   Only assertions in {}
#          signature           Compiler verifies x > 0 at compile time

List: (T: Type) -> Type = { data: Array(T) }
#      ^^^^^^^^              ^^^^^^^^^^^^^^^
#      Parameters at the    Compiler verifies type_of(T) == Type, type_of(data) == Array(T)
#      signature
```

The same pattern: `name: (params) -> Type = { assertions }`. The compiler does not distinguish "type
assertions" from "value assertions" — both are evaluation targets in the proof pipeline.

**Loop invariants don't need to be written separately. Type annotations on variables are Floyd-Hoare
invariants.**

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # Annotation references i — tells the compiler s's type depends on i
    mut i: UpTo(arr.len) = 0     # On initialization, i=0, verify: 0 == sum(arr[0..0]) → True
    while i < arr.len {
        s += arr[i]  # Compiler verifies: s_new == sum(arr[0..i+1])
        i += 1       # i changes → triggers s dependency reverification: s satisfies SumUpTo(arr, i_new)
    }
    return s  # s: SumUpTo(arr, arr.len) = sum(arr[0..arr.len])
}
```

The compiler generates a verification condition for the loop body once — induction hypothesis (type
annotation) → assignment operation → whether the new value satisfies the type annotation. Once the
proof pipeline verifies the inductive step holds, all iterations are covered automatically. No need
for `: decreases`, no need for `: Invariant`, no need for an induction proof — the compiler
decomposes induction into local VCs for each assignment.

### 2. Pre/Postconditions: Compile-time Predicates on Parameter Types and Return Types

Abandon RFC-022's `//! requires`/`//! ensures`. Compile-time predicates serve as annotations on
parameters or returns.

**On the parameter side it's a function call.** A compile-time predicate is a function returning
Type; its use on the parameter side is simply calling it — just like `factorial(5)`. On the return
side, a new concept is introduced: the return value parameter.

```yaoxiang
# Precondition: explicitly call the compile-time predicate in the parameter type
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b
#                       ^^^^^^^^^^  b is the current parameter name, passed to Positive as argument
#                       The compiler extracts the actual argument value at the call site,
#                       substitutes b, verifies Positive(actual argument)
#                       Example: divide(10, 2) → verifies Positive(2) = { 2 > 0 } → True
#                       Example: divide(10, 0) → verifies Positive(0) = { 0 > 0 } → False → compile error

# Postcondition: return value parameter + compile-time predicate
IsMax: (T: Ord, arr: Array(T), result: T) -> Type = {
    forall j in 0..arr.len: result >= arr[j]
}

NonEmpty: (arr: Array(T)) -> Type = { arr.len > 0 }

max: (T: Ord) -> ((arr: NonEmpty(arr))) -> (result: IsMax(T, arr, result)) = {
#                                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
#                                            result is the return value parameter; value is provided by return
#                                            The compiler substitutes the return value at the return point and verifies the postcondition
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
- **Return side**: `-> (result: IsMax(T, arr, result))` — `result` is the return value parameter;
  the value is provided by the `return` statement. `result` exists only in the type signature, only
  referenced by the predicate; it does not enter the function body scope and does not appear at the
  caller.
- **Return value parameter is optional**: when there is no postcondition, do not write it; the
  signature is identical to an ordinary function (`-> Int`).
- **Unity**: parameter and return value parameters are the same concept —
  `param_name: predicate_call(param_name)`, differing only in whether the value is provided by the
  caller or by `return`.

### 3. Path Condition Propagation: Compile-time Verification of Runtime Values

When compile-time predicates are used at binding positions, arguments are passed explicitly by the
programmer. When runtime values enter refined type parameters, the compiler completes verification
through path condition collection and SMT implication judgment — no need for the programmer to
explicitly pass proof.

#### 3.1 Explicit Function Calls

When compile-time predicates are used at binding positions, arguments are passed explicitly by the
programmer — it's just a function call, zero implicits.

`Positive: (x: Int) -> Type = { x > 0 }` is a compile-time predicate constructor. When it appears at
a binding position (parameter declaration, variable declaration, return type), the programmer
explicitly passes an already-bound variable name:

```yaoxiang
b: Positive(b)
// b has been declared as the current parameter; Positive(b) is a function call
// After normalization: b: { b > 0 }
```

The compiler does not need to implicitly fill in arguments — `b: Positive(b)` is just a function
call, same as `f(5)`. `b` is bound as a parameter name, and its type annotation `Positive(b)`
references `b` itself — this is the standard pattern of dependent types, not an implicit expansion
rule.

**Unification with RFC-010 `self`**: RFC-010 establishes that `self` is not a keyword, just a
conventional name for a parameter ("writing `p`, `this`, or `x` has exactly the same effect").
`b: Positive(b)` shares the same mechanism — a parameter name can be referenced in a type
annotation. `self` appears in the position of `self: Point`; `b` appears in the position of
`b: Positive(b)`; both annotations reference the parameter itself. The difference lies only in the
complexity of the annotation; the mechanism is exactly the same — once a name is bound, the type can
depend on that name.

Return types also use explicit function calls:

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
// x is bound to 5, Positive(5) → { 5 > 0 } → True → pass

// let y: Positive(y) = 0
// y is bound to 0, Positive(0) → { 0 > 0 } → False → compile error
```

#### 3.2 Path Condition Collection

When a runtime value appears in a conditional branch, the compiler automatically collects path
conditions, forming the **assumption set** of the current scope. These assumptions participate in
verification as background knowledge for compile-time Bool evaluation.

```yaoxiang
if y > 0 {
    // The compiler automatically obtains the assumption within this branch: { y > 0 }
    let result = divide(x, y)
    // Verification condition: (y > 0) ⇒ (y > 0)
    // The proof pipeline judges the implication holds → Proved
} else {
// This branch assumes: { !(y > 0) }
// If we want to call divide(x, y), the verification condition is !(y > 0) ⇒ y > 0
    // The proof pipeline judges it does not hold → Disproved
}
```

This is not the compiler hard-coding a special pattern — this is the natural behavior of the
compile-time proof pipeline. At each type-check call point, the pipeline receives:

```
{Background Assumptions} ⇒ {Verification Goal}
```

The proof pipeline judges implication. Proved → pass; Disproved → compile error + counterexample;
Unproven → compile error + unsolved proposition. Background assumptions come from the path
conditions of the current program point.

#### 3.3 Assumption Stack

When analyzing control flow, the compiler maintains an assumption set for each basic block:

- **if-guard**: `if y > 0` → true branch pushes `y > 0`, false branch pushes `!(y > 0)` (if else is
  used)
- **match patterns**: `if let Some(v) = opt` → branch pushes `opt == Some(v)`
- **Logical conjunction**: `if x > 0 and y < 10` → branch pushes `x > 0` and `y < 10`
- **Function preconditions**: when calling `divide(a, b)`, evidence that `b` satisfies `Positive`
  must come either from the current assumption, or from the refined type annotation of the argument
  itself (if `b` is already annotated as `Positive`, its type carries `b > 0`)
- **Assignment**: when `let z = y`, the existing refined condition on `y` is transferred to `z`

All assumptions enter the compile-time proof pipeline. When entering the SMT acceleration path, they
are translated into SMT-LIB background assertions.

#### 3.4 Compile Error When There Is No Static Evidence

If the programmer directly writes:

```yaoxiang
divide_user_input: (x: Int, y: Int) -> Int = divide(x, y)
```

There is no `y > 0` assumption at the current program point, and the argument `y` itself has no
`Positive` type annotation. The verification condition is:

```
{} ⇒ { y > 0 }
```

The pipeline returns `Disproved` (does not hold) → compile error:

> Cannot prove that argument `b` satisfies `Positive` in the call to `divide`. `y` comes from
> function input with no proven bound. Consider guarding the call with an if branch:
> `if y > 0 { divide(x, y) }`.

YaoXiang does not accept runtime values directly entering refined type parameters without providing
static evidence. This is not a limitation — it is the core of the hard safety philosophy. Any code
the compiler cannot statically prove is not allowed to pass compilation.

#### 3.5 Relationship with the Unified Pipeline

Path condition propagation is not an additional mechanism. It is a direct extension of the
compile-time proof pipeline in control flow analysis:

| Stage                             | Responsibility                                                                                                                                             |
| --------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Path condition collection         | Compiler's control flow analysis stage; annotates the assumption set for each basic block                                                                  |
| Verification condition generation | When a type constraint requiring verification is encountered, merge path conditions + argument type information                                            |
| Proof pipeline evaluation         | Compiler kernel → SMT acceleration → arrive at Proved / Disproved / Unproven                                                                               |
| Result                            | `Proved` → pass; `Disproved` → compile error + counterexample; `Unproven` → compile error + unsolved proposition (programmer can provide a proof function) |

No new components. No special rules. Path conditions are the background knowledge of the proof
pipeline — sharing the same pipeline and the same budget system with type equality and borrow
constraints.

### 4. Compile-time Proof Pipeline

All compile-time checks share one pipeline. The core operation of the pipeline is **type checking**
— checking whether a proof term's type equals the proposition to be proved. Everything is type
checking.

```
Compile-time encounter with Bool expression requiring evaluation
(i.e., need to construct a proof term)
        │
        ├── Type equality (T1 == T2)
        │   → Compiler directly judges (structural equivalence)
        │
        ├── Token conflict condition (!conflicting(tokens))
        │   → Flow-sensitive liveness analysis (Dup/Linear property tracking)
        │
        ├── Dependent type reduction (n + m simplification)
        │   → Compile-time term rewriting system (βδι-reduction)
        │
        ├── Compile-time predicates (x > 0, forall...)
        │   → Compiler itself + SMT acceleration module
        │
        └── Hoare logic implication (P ⇒ Q)
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
            Verification fails → Compile error: "Proof does not hold"
```

#### 4.1 Proof Result: Three-valued Algebra

Compile-time evaluation returns three results — this is the inevitable conclusion of the halting
problem and the natural division of proof theory:

```
eval_compile_time : BoolExpr → Proved | Disproved(Model) | Unproven
```

- **Proved** → Halts, proof term constructed, type check passes. Compilation continues.
- **Disproved(M)** → Halts, counterexample M exists. Compile error + counterexample + source
  location.
- **Unproven** → Within the given resource limit, no proof was constructed. Compile error + unsolved
  proposition + budget consumption report.

**Unproven ≠ False.** The compiler saying "I cannot prove it" is not equivalent to the proposition
being false — it simply exceeds the current automatic proof capability. This is honesty, not a
defect.

A hard limit on budget is the engineering solution to the halting problem. No knob — giving one is
asking the user "do you think your program will halt", the user doesn't know, and the compiler
doesn't either.

#### 4.2 After Unproven: The Programmer Writes the Proof

When the compiler returns Unproven, the programmer can write a **proof function** — a YaoXiang
function whose return type equals the proposition to be proved. The type checker verifies this
function — exactly the same mechanism as it verifies `add(a, b): Int`.

```
Proposition = Type
Proof       = Program (a value of that type)
Verification= Type check (the sole trust root)
```

The SMT solver is not a separate trust boundary — it is an **acceleration module of the type
checker**. SMT helps find proofs, but the proof is always verified by the type checker. When SMT
returns `unsat`, the compiler restructures its result as a proof term verifiable by the type
checker. If the restructuring fails (SMT's inference steps exceed the compiler kernel's inference
rules), it falls back to Unproven — the programmer can manually write a proof function.

```yaoxiang
# Proposition: a refined property the compiler cannot prove automatically
FirstIsMin: (T: Ord, arr: Sorted(T)) -> Type = {
    forall i in 0..arr.len: arr[0] <= arr[i]
}

# Proof: the programmer writes a function whose return type is the proposition above
# The type checker verifies this function — exactly the same as verifying add(a,b): Int
first_is_min: (T: Ord, arr: Sorted(T)) -> FirstIsMin(T, arr) = {
    # The compiler verifies here: function body's type = FirstIsMin(T, arr)
    ...
}
```

No AI needed, no export to Coq, no new concepts. **Properties the compiler cannot prove
automatically at compile time → the programmer writes proof in YaoXiang code → the type checker
verifies.** The whole process is a smooth gradient — the compiler handles easy proofs, leaving the
hard ones for your brain.

#### 4.3 Hierarchical Dependencies within the Pipeline

The evaluators above share the same interface but have an evaluation order. Type equality is a
prerequisite for all subsequent analysis; ownership/token checking depends on type information;
refined predicate verification depends on the results of the first two layers. The compiler
evaluates layer by layer; expressions that fail at lower layers do not enter upper layers — avoiding
wasting solving budget on type-erroneous programs.

```
Evaluation order (same pipeline, layered scheduling)
├── Layer 0: Type equality (T1 == T2)
│   └── Structural unification → failure makes subsequent steps meaningless, return Disproved directly
├── Layer 1: Ownership/token conflict
│   └── Flow-sensitive liveness analysis → failure makes memory safety fail, return Disproved directly
└── Layer 2: Refined predicate / Hoare implication
    └── Compiler itself → SMT acceleration → arrive at Proved / Disproved / Unproven
```

Each layer still returns `Proved/Disproved/Unproven`, sharing the same interface and the same budget
system.

### 5. Three-layer Function Unification

| Layer                  | When it runs | Input      | Output | Example                                        |
| ---------------------- | ------------ | ---------- | ------ | ---------------------------------------------- |
| Value-level function   | Runtime      | Value      | Value  | `add: (a: Int, b: Int) -> Int = a + b`         |
| Type constructor       | Compile time | Type/value | Type   | `List: (T: Type) -> Type = { data: Array(T) }` |
| Compile-time predicate | Compile time | Value      | Type   | `Positive: (x: Int) -> Type = { x > 0 }`       |

All use the same `name: type = value` syntax. Compile-time predicates and type constructors go
through the same compile-time proof pipeline — `{}` is the proof space.

### 6. Loops: Floyd-Hoare Verification Condition Generation

Loops do not need a separate `: Invariant(...)` or `: decreases(...)` annotation. The compile-time
predicate type annotation on a variable defines a Floyd-Hoare-style assertion — the compiler
generates verification conditions from the type annotation, and the proof pipeline checks whether
each assignment maintains the type.

Core mechanism: each assignment corresponds to a Hoare triple `{P} x := e {Q}`, where the
verification condition is `P ⇒ Q[e/x]`. The compiler generates a verification condition for the loop
body once — once the proof pipeline verifies the inductive step holds, all iterations are covered
automatically.

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # Annotation references i; on initialization, i=0, verify: 0 == sum(arr[0..0]) → True
    mut i: UpTo(arr.len) = 0     # Verify: 0 <= 0 <= arr.len → True
    while i < arr.len {
        # The compiler generates one VC for the loop body. Premise: s satisfies SumUpTo(arr, i), i satisfies UpTo(arr.len).
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
        #   i changes → in the dependency graph, s's type annotation references i → triggers reverification
        #   New verification goal: s satisfies SumUpTo(arr, i_new)
        #   i.e. s == sum(arr[0..i_new]), guaranteed by the previous step → Proved
        s += arr[i]
        i += 1
    }
    return s  # At this point s: SumUpTo(arr, arr.len), i.e. s == sum(arr[0..arr.len])
}
```

Loop invariants are the type annotations on variables — the programmer writes the type, the compiler
checks the inductive step. The compiler does not need to "discover" invariants, nor "do induction
automatically" — it decomposes the induction proof into local verification conditions for each
assignment operation, leaving them to the proof pipeline to be solved divide and conquer.

#### 6.1 Dependency Tracking: Dependent Types on Mutable Variables

The prerequisite for the mechanism above is: the compiler knows that `s`'s type annotation
`SumUpTo(arr, i)` references `i` — when `i` changes, the type constraint of `s` changes accordingly.
This requires the compiler to maintain a **type dependency graph between variables**.

**Data structure**:

```
TypeDepGraph: Map<VarName, Set<VarName>>
# Key is the depended-on variable; value is the set of variables that reference that variable in type annotations
# Example: { i: {s}, j: {s, t}, ... }
```

**Construction**: when processing `mut v: Pred(... x ...) = init`, the type checker parses free
variable references in the `Pred(...)` arguments. If the arguments reference another mutable
variable `x` in the current scope, `x → v` is recorded in the dependency graph.

**Triggering**: when a depended-on variable `x` is assigned, the compiler:

1. Looks up all variables in the dependency graph that depend on `x` — `{v₁, v₂, ...}`
2. For each `v`, generates a verification condition: whether `v`'s current value satisfies the
   updated type `Pred(... x_new ...)`
3. Sends the VC to the proof pipeline

**Assignment order sensitive**: dependency tracking naturally enforces the correct assignment order.
Take `SumUpTo(arr, i)` as an example:

```yaoxiang
# Correct order
s += arr[i]   # s_new satisfies SumUpTo(arr, i+1)
i += 1        # i changes → reverify s satisfies SumUpTo(arr, i_new) → True

# Wrong order — compiler rejects
i += 1        # i changes → reverify s satisfies SumUpTo(arr, i_new)
              # s not yet updated, s_old == sum(arr[0..i_old]) ≠ sum(arr[0..i_new])
              # → Compile error: variable s does not satisfy type SumUpTo(arr, i_new)
s += arr[i]   # Unreachable
```

**Composite dependencies**: a variable can depend on multiple variables. The type annotation
`{ v: Int; v == x + y }` depends on both `x` and `y` — any change to either triggers reverification.

**Relationship with the proof pipeline**: dependency tracking is the trigger for VC generation, not
an independent verification mechanism. It answers "when does a VC need to be generated" — the proof
pipeline answers "does the VC hold".

### 7. Termination Checking

**Scope of application: refined types.** Termination is not a separate switch, but a part of the
**verification mode**: once a type is refined (`Refined { base, constraint }`), the computation
annotated by it enters verification mode, which requires termination; unrefined ordinary types do
not enter verification mode and generate no termination obligations.

From this, two corollaries:

- **Loops**: a bare `while` does not enter verification mode; when the measure variable carries a
  refined annotation (e.g. `i: UpTo(n)`), it enters verification mode and must prove termination.
- **Recursion**: when a function signature carries refinements (refined parameters or return types
  containing refinements), it enters verification mode and must prove that the measure strictly
  decreases at each recursive call point.

Fully automatic takes priority within the mode: the compiler first automatically explores measures,
proving what it can; if nothing can be proved and no explicit measure is given, compile error. No
**annotation syntax** escape hatch — measures and termination propositions are written at the type
position, no new `decreases`-like syntax. The form of measures and explicit fallbacks is given in
§6.9.

#### 6.1 Design Principles

The compiler automatically extracts the information needed for termination proofs from two sources:

1. **Variable type annotations**: boundary constraints in refined types (e.g. `UpTo(n)` gives upper
   bound `n` and lower bound `0`)
2. **Loop body operations**: the operations applied to variables on each iteration

The compiler tries four measure synthesis strategies in priority order, stopping as soon as one
succeeds. The four strategies are a **bounded template sequence for measure exploration**, with
input being refined constraints (all four strategies start from "variables of bounded types"), not
"code evaluated at compile time"; they are the automatic and manual sides of the same thing as the
explicit measure in §6.9.

> **Measure exploration is exploration, not inference.** Exploration only enumerates templates
> (linear rank, violation count, bounded patterns, multiplicative scaling) and does not guarantee a
> solution — general measure inference is undecidable (reduces to the halting problem). Therefore,
> cases outside the templates must allow the programmer to explicitly give a measure (§6.9),
> otherwise compile error.

#### 6.2 Strategy 1: Automatic Synthesis of Linear Rank Functions

When a variable has a linear bound annotation, the compiler enumerates candidate linear measures and
verifies them with SMT.

```
Input:
  Variables v₁: UpTo(u₁), v₂: UpTo(u₂), ... (variables with lower and upper bounds)
  Loop condition cond
  Set of assignments in the loop body

Algorithm:
  1. Extract each variable's bounds from type annotations: [low_i, high_i]
  2. Enumerate candidate measures: v_i, u_i - v_i, v_i - v_j, etc. — linear combinations
  3. For each candidate measure m:
     - SMT verifies m ≥ 0 (derived from type bounds)
     - For each execution path of the loop body, SMT verifies m' < m (strictly decreasing)
  4. Find a qualifying linear combination → termination proved
```

Coverage: any loop where a variable is assigned a linear expression (`v = a·v + b`) and has a
bounded type annotation. Includes `i += const`, `i -= const`, and interval contraction in binary
search:

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

#### 6.3 Strategy 2: Predicate Violation Count — Automatic Measure Extraction from Target Types <span style="color:orange">[Experimental Strategy]</span>

> ⚠️ **Current status: experimental strategy; whether to include it in Phase 3 implementation will
> be determined based on actual feasibility.** This strategy works for adjacent swap operations
> (bubble sort, insertion sort) and cannot automatically prove non-adjacent operations (quicksort
> partition, heapsort sift-down). Coverage boundaries are shown in the table below. If Phase 3
> verification is infeasible, this strategy will be removed or downgraded to future work.

Core insight: **the specification the user writes is material for the compiler's reasoning.** The
compiler does not need to have "what is sorting" built in — it reads the definition of `Sorted` and
automatically extracts the measure from that definition.

```
Input:
  Target type: Sorted(arr) = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }
  Loop body operation: adjacent element swap

Algorithm:
  1. Parse predicate definition: forall i in range: cond(i, arr)
  2. Automatically generate measure: violation_count = |{ i | ¬cond(i, arr) }|
  3. Analyze the operation's effect on the measure:
     - Adjacent swap arr[j], arr[j+1] = arr[j+1], arr[j]
     - Only affects the three pairs at indices j-1, j, j+1
     - If arr[j] > arr[j+1] (violates predicate), this pair satisfies the predicate after the swap
     - violation_count decreases by at least 1
  4. Upper bound: n·(n-1)/2 (maximum adjacent inversions), lower bound: 0
  → Termination proved
```

**Current coverage**:

| Algorithm      | Operation pattern | Strategy 2 provable? | Reason                                               |
| -------------- | ----------------- | :------------------: | ---------------------------------------------------- |
| Bubble sort    | Adjacent swap     |          ✅          | violation_count strictly decreases per swap          |
| Insertion sort | Adjacent shift    |          ✅          | Each shift eliminates one violation pair             |
| Selection sort | Non-adjacent swap |          ❌          | A single swap may increase violation_count           |
| Quicksort      | partition         |          ❌          | Non-adjacent swap, no monotonic decrease             |
| Heapsort       | sift-down         |          ❌          | Tree-shaped operation, violation_count non-monotonic |

**Complementary strategies**: for quicksort, the `low < high` interval contraction can be covered by
Strategy 1 (linear rank function) — the outer partition recursion halves the interval each time.
Strategies 1 and 2 complement each other; the termination of most practical algorithms can be proved
by one of them. But the generalization of Strategy 2 (non-adjacent operations, tree operations)
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

#### 6.4 Strategy 3: Bounded Increment/Decrement Pattern

`v += const` (positive constant), variable has upper-bound type annotation → measure
`upper_bound - v` decreases by `const` each iteration, lower bound 0. This is a degenerate case of
Strategy 1, which the compiler handles first.

#### 6.5 Strategy 4: Multiplicative Scaling Measure Template

`v *= const` (const > 1), variable has lower- and upper-bound type annotations. The compiler has a
built-in logarithmic measure template `ceil(log_const(upper/v))`; each multiplication by const
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

- **Termination**: the four strategies above automatically prove the loop exits in a finite number
  of steps; when exploration fails, the programmer gives a measure at the type position (§6.9)
- **Correctness**: whether the loop body progresses toward the target type is checked by the
  compile-time proof pipeline through verification conditions

Both pass → compilation passes. Termination proved but correctness fails → compile error +
counterexample. Correctness proved but termination cannot be proved → compile error, pointing to the
variable or operation that cannot be analyzed. Both fail → compile error reporting both failure
causes separately.

#### 6.7 Termination Checking for Recursive Functions

For recursive functions with refined signatures, the compiler checks that the parameters decrease at
each recursive call point:

```yaoxiang
gcd: (a: Int, b: NonNegative(b)) -> Int = {
    if b == 0 { return a }
    return gcd(b, a % b)  // Compiler explores: well-order measure on (b, a % b) decreases → termination
}
```

Parameter decrease is the strongest automatic path (structural recursion). When exploration fails,
the programmer explicitly gives the measure at the type position (§6.9).

#### 6.8 Hard Boundary

`i = f(i)` where `f` is non-invertible, not closed, and does not preserve any monotonicity —
automatic termination proof is mathematically impossible. Compile error:

> This loop cannot have its termination proved automatically. The loop variable depends on the
> unanalyzable function `f`. Please use an iteration pattern that can be analyzed by the compiler,
> or bind a name to the loop and provide a measure at the type position (§6.9).

This is not a compiler failure. Any code that cannot be statically proved safe is not allowed to
pass compilation. Even when an explicit measure is given, it must be judged true by SMT to pass — if
the measure is written incorrectly, it will be rejected by a counterexample. A person can only fail
to prove, never prove incorrectly.

#### 6.9 Explicit Measure: `Terminates`

When automatic exploration fails to produce a measure, the programmer writes the measure at the
**type position** — the same mechanism as `Positive(b)`, `IsMax(T, arr, result)` (predicate
application), with zero new syntax:

```yaoxiang
// Measure: an ordinary function, can be unit-tested, reusable, not involved at runtime
gcd_measure: (a: Int, b: Int) -> Int = { b }

// The termination component lives on the function's own type position
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// Loop: the binding name is the anchor; the measure picks up the relevant quantities by name within its scope
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n {
        i = i + 1
        i
    }
    return acc
}
```

The reasoning for the loop here: `Terminates(m)` refines the value type of the loop body's tail
expression, and the anchor is provided by the binding name `acc` — the loop can therefore be
referred to, and there is no longer any dead corner of "anonymous construct with no way to refer to
it".

> **The body tail must yield a value**: the value of a loop = the value of its body block (§2.15),
> and the value of a block = its tail expression. If the body tail is an assignment statement (e.g.
> `{ i = i + 1 }`), the block value is `Void`, and `return acc` returns `Void` from `-> Int`, which
> does not type-check. Therefore, the body tail in this example explicitly writes `i`. This does not
> change the semantics of the measure — the measure `n - i` is a **state expression**, unrelated to
> the loop's value type.

**`Terminates` is a built-in predicate**, in the same core primitive class as `Int`, `Never`. It is
the only predicate for which the **compiler writes the function body** — its assertion ("the measure
strictly decreases at each recursive call point / loop back edge") lives in the computation
structure, and the user's predicate cannot reference function bodies or loop bodies, so it cannot be
expressed using the predicate definition syntax in the "Syntax" section. The built-in face converges
on this single name.

**Arity**. `Terminates(FnType, m)` and `Terminates(m)` are two arities of the same predicate, not
two distinct constructs:

| Form                    | Anchor                          | Use case                                                        |
| ----------------------- | ------------------------------- | --------------------------------------------------------------- |
| `Terminates(m)`         | Name of the binding it lives in | Self-recursive functions, loops — the default form              |
| `Terminates(FnType, m)` | Explicit function type          | Mutual recursion and other cases where the anchor is not unique |

Both are essentially the same: the termination obligation always falls on "the computation annotated
by the type position where the refinement lives."

**The measure does not constrain the return type.** The measure can be an expression of any type
(not necessarily natural numbers); "strict decrease" on it is given by the well-order available on
that type. Whether the measure is well-founded (e.g. when it returns `Int`, whether it is `>= 0`) is
an **independent obligation**, and is also handed to refined inference or SMT just like the decrease
obligation; when neither can derive it, the diagnosis does **not directly reject**, but suggests a
direction to check (whether the lower bound on the measure holds, whether the recursive parameters
truly move in that direction).

**Relationship with automatic exploration**: the explicit measure is not a separate pipeline, but
the input after exploration fails. After the measure is given, it still goes through the same SMT to
verify decrease and well-foundedness; if it fails, an error is reported along with a counterexample.

The implementation mechanisms for obligation generation, judgment pipeline, diagnosis direction, and
measure sharing for mutual recursion (SCC) are described in
[RFC-027a: Explicit Measures for Termination Checking](../review/027a-termination-explicit-measure.md).

### 8. SMT Solver: An Acceleration Module of the Type Checker

In traditional languages, the SMT solver is an external tool (e.g. F\* calls Z3, Dafny calls Z3). In
YaoXiang, it is an **acceleration module of the type checker** — invoked only when the compiler
kernel itself cannot directly judge. SMT helps find proofs, but the proof is verified by the type
checker.

**Trust model**: the type checker is the sole trust root. The SMT solver is an acceleration module —
it helps find proofs, but SMT is not a separate trust boundary. The compiler trusts Z3's `unsat`
results (consistent with the F\*/Dafny approach — Z3's error rate is lower than the compiler's own
bug rate, an engineering-practical choice). The real unreliability is controlled in the SMT
translation layer — if the translation has a bug, the compiler will surface it in other tests.

**Interface**: the compiler internally translates to the SMT-LIB 2.6 standard format, rather than
binding to a specific solver API. SMT-LIB is an ISO standard, natively supported by Z3, CVC5,
MathSAT, and Yices.

**Default backend**: Z3 (MIT license, the broadest documentation and community validation). CVC5 is
an SMT-LIB-compatible alternative — users can switch at compile time via compiler flags.

No "generic solver abstraction layer" — SMT-LIB is the abstraction layer. If CVC5 ever has a
breakthrough in a particular theory, switching only requires swapping the binary, no compiler code
changes.

```
Compile-time Bool expression
        │
        ├── Compiler kernel can directly judge (structural equivalence, simple arithmetic,
        │   trivially true formulas after constant folding)
        │   → Return Proved / Disproved directly
        │
        └── Compiler kernel cannot directly judge (quantifiers, symbolic variables)
            → Dependent type pre-reduction (factorial(5) → 120)
            → Translate to SMT-LIB format
            → Send to Z3/CVC5 (with budget limit)
            → Return value: unsat → Proved  │  sat + model → Disproved  │  unknown → Unproven
```

**Solving budget — hard limit, like stack depth**:

| Budget dimension               | Default | Description                                                                                                                                                               |
| ------------------------------ | ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Solving steps                  | 10,000  | Z3 is usually within a hundred steps for linear arithmetic. 10,000 steps covers 99% of practical predicates.                                                              |
| Time                           | 100ms   | A single predicate exceeding 100ms = the user is writing a compile-time program rather than a type annotation. 100ms × 50 predicates = 5-second compile time upper bound. |
| Quantifier instantiation depth | 3       | Three levels of nested quantifiers cover practical patterns. Beyond three layers, the user is most likely writing logic exercises.                                        |

Exceeding the budget returns Unproven, compile error + predicate location + consumption. No
degradation, no runtime check, no silent pass.

**Why this is actually feasible**: in practice, 95% of real predicates are linear arithmetic —
`x > 0`, `arr.len > 0`, `0 <= idx < arr.len` — all within the decidable fragment, and SMT solvers
return in milliseconds. For the rare complex predicates that exceed the budget, the programmer
writes a proof function.

Dependent type pre-reduction happens before the SMT call: `factorial(5)` directly evaluates at
compile time to `120`; `append([1,2], [3])` directly evaluates to `[1,2,3]`. These deterministic
value computations do not consume SMT budget.

The programmer does not need to know SMT exists. Mental model: **the compiler can prove it and pass;
cannot prove it and error — if the compiler cannot, you can write a function to prove it.**

### 9. Compile-time Predicate Composition

Compile-time predicates are functions returning Type; composition is naturally achieved through
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

### 11. Dispatch Routing Pipeline: Unified Routing for Compile-time and Runtime

`assert` and `Assert` are two sides of the same refined type primitive. The routing pipeline
`dispatch` automatically decides whether to go through compile-time proof or runtime check based on
**whether the predicate's free variables are accessible at compile time**:

| Criterion                                                                                   | Mode            | Behavior                                                                                     |
| ------------------------------------------------------------------------------------------- | --------------- | -------------------------------------------------------------------------------------------- |
| All free variables known at compile time (generic parameters, compile-time constants)       | **CompileTime** | Enter the proof pipeline: Proved → erase, Disproved → compile error, Unknown → require proof |
| Some free variables come from runtime (function parameters, external inputs, mut variables) | **Runtime**     | Insert runtime check and inject refined facts into the flow-sensitive assumption set Γ       |

**Key point**: "Cannot decide" ≠ "Disproved". In CompileTime mode, Unknown requires proof (no silent
degradation); in Runtime mode, the proposition has no truth value at compile time in the first place
— no matter how strong the prover is, it cannot write a universally true proof for "the user might
have entered a negative number"; a runtime check is the only sound choice. This is not because the
prover isn't strong enough; it is a theoretical necessity.

### 12. Flow-sensitive Assumption Set Γ: Strongest Postcondition Propagation

The compiler maintains a flow-sensitive assumption set Γ, tracking the propositions known to hold at
each control flow point.

**SP (Strongest Postcondition) propagation**:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
```

**mut variable kill set**: after a `mut` variable is reassigned, all assumptions involving that
variable are removed from Γ:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
mut x = x - 5       // Γ = {}  ← x > 0 is killed
```

This is a hard requirement of soundness — when a variable's value changes, the old assumption is
invalid.

**Branch confluence**: when IF/ELSE or match branches merge, Γ takes the intersection of each
branch's assumptions. Only propositions that hold on all paths carry out of the branch.

### 13. Clarification of the Erasure Model: Witness Erasure ≠ Check Erasure

RFC-027's claim that "refined types are **fully erased** at runtime" refers to the **proof witness**
— the proof term verified at compile time produces no runtime code. But the **runtime check**
inserted by dispatch in Runtime mode is retained — it is a Bool check performed at the value level,
not a witness at the type level.

Summary: witnesses are erased, checks are retained. These two things don't conflict, and the
original claim of RFC-027 is unchanged.

## Detailed Design

### Syntax Changes

| Before (RFC-022)                      | After (This RFC)                                                                                                                                                    |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `//! requires: NonEmpty(n) = n > 0`   | Compile-time predicate as parameter type `(b: Positive(b))`                                                                                                         |
| `//! ensures: ExistsMax(result, arr)` | Return type uses return value parameter `-> (result: IsMax(T, arr, result))`                                                                                        |
| `/*! invariant: ... !*/`              | Compile-time predicate type annotation on variables — Floyd-Hoare invariants                                                                                        |
| `//! decreases: n`                    | Measure written at the refined type position (`Terminates`); compiler tries automatic exploration first, and only requires explicit measures when exploration fails |
| Specifications are annotations        | Specifications are the type system                                                                                                                                  |

### Syntax

**No new keywords for compile-time predicates.** `{}` is the proof space, identical to the existing
type definition syntax. A compile-time predicate is a function returning Type —
`name: (params) -> Type = { assertions }`. Its use is just a function call — `Positive(b)`,
`IsMax(T, arr, result)`.

```bnf
# Compile-time predicate = a function returning Type; contents of {} are compiler-verified assertions
# Uses existing function/type syntax; no new BNF rules needed
predicate ::= identifier ':' params '->' 'Type' '=' '{' assertions '}'
```

**Predicate application arguments must be in compile-time constant form** — literals, variables
(bound by name), type applications (recursively extracted), or **compile-time referable function
references** (function names). Arguments that cannot be converted to a constant expression report
**E1092**; argument count mismatching the predicate's declared parameter count reports **E1093**.
Arguments are bound positionally to the parameter list; predicate arity is determined by its
declaration — `Positive(x)` unary, `IsMax(T, arr, result)` ternary, `Terminates(m)` and
`Terminates(FnType, m)` unary and binary — **refinement constraints are never silently dropped**
(previously, non-convertible arguments caused constraints to silently disappear, and bindings
violating constraints would silently pass).

**New syntax concept: return value parameter** — in `-> (name: Type)`, `name` is the return value
parameter.

The return value parameter is the **only syntax concept** YaoXiang introduces on top of the existing
function syntax. Its semantics:

- `name`'s value is provided by the `return` statement
- `name` exists only in the type signature, referenced by the postcondition predicate (e.g.
  `-> (result: IsMax(T, arr, result))`)
- `name` does not enter the function body scope, nor appear at the caller
- The return value parameter is **optional** — when there is no postcondition, the signature is
  identical to an ordinary function (`-> Int`), introducing no extra burden

The reason for introducing it: postconditions need to reference "the value the function is about to
return". Without a return value parameter, the compiler can only make the predicate reference the
return value through special rules (such as implicit variables `$result` or `__retval__`). The
return value parameter makes this reference explicit — it is just a parameter; the value is provided
by `return` rather than the caller.

**Proof functions** are not a new concept — they are just YaoXiang functions whose return type is
the proposition being asserted. When the compiler returns Unproven, the programmer provides a proof
function; the type checker verifies it in exactly the same way as it verifies any function's return
type. No new syntax, no new keywords, no new rules.

> **The boundary between the two.** The correctness domain (predicate Unproven) supplements the
> proof in the **body**: write a function whose return type is the proposition to be proved. The
> termination domain (§6.9) supplements the measure at the **type position**: write
> `Terminates(measure)` as the type of a binding or function signature. The former is "write a proof
> for a proposition that cannot be proved", the latter is "explicitly declare a measure that cannot
> be explored" — the mechanism is the same (both are refined type applications), but the location
> differs. The termination domain **does not need** to write a `_proof` function.

### Type System Impact

- **Type universe**: compile-time predicates live at the Type₂ level — functions taking values and
  returning Type, at the same level as type constructors
- **Generics interaction**: compile-time predicates can take generic parameters, e.g.
  `NonEmpty: (T: Type) -> (arr: Array(T)) -> Type`
- **Ownership interaction**: expressions inside compile-time predicates obey ownership rules; they
  can only read, not write
- **Type inference**: compile-time predicate arguments participate in HM type inference

### Runtime Representation

Compile-time predicates are processed at runtime **according to the dispatch routing result**:

- **CompileTime mode** (all free variables known at compile time): after proof passes, the proof
  witness is fully erased. `Positive: (x: Int) -> Type = { x > 0 }` — the argument `b: Positive(5)`
  at runtime is just represented as `Int`. The refinement condition `{ 5 > 0 }` has passed and is
  erased.
- **Runtime mode** (runtime free variables present): retain the runtime check — perform a Bool check
  at the value level and inject it into the flow-sensitive assumption set Γ. See §11 dispatch
  routing pipeline and §13 erasure model clarification for details.

Placing compile-time predicates at type positions (e.g. `f(x: Positive(x))`) does not produce
wrapper types or allocate extra memory. But when `x` comes from runtime input, **a runtime Bool
check is inserted**.

**Interaction constraint with `ref`**: compile-time predicates can only reference values with
immutable borrows or already-transferred ownership. Compile-time predicates referencing
mutable-borrowed values cannot guarantee the verification result still holds at runtime — such uses
directly report a compile error.

### Compiler Changes

1. **Parser**: compile-time predicates use standard function syntax; no additional parsing rules
   required
2. **Compile-time proof pipeline**: unified Proved/Disproved/Unproven return interface with
   automatic strategy selection
3. **SMT acceleration module**: SMT-LIB 2.6 translation layer, default backend Z3, CVC5 alternative
4. **Type checker kernel**: implementation of inference rules — structural equivalence,
   βδι-reduction, universal quantifier introduction/elimination. This is the sole trust root; both
   SMT and programmer proofs are verified through this
5. **Verification condition generation**: WP/SP calculus + loop invariant proof obligations
6. **Error reporting**: counterexample formatting + unsolved proposition reports + source location
   association

### Backward Compatibility

- ✅ Code not using compile-time predicates is completely unchanged
- ✅ Compile-time predicates have zero runtime overhead in CompileTime mode; only necessary Bool
  checks are retained in Runtime mode
- ⚠️ RFC-022's `//!` syntax is no longer supported — but 022 was never implemented, so there is no
  migration burden

## Trade-offs

### Advantages

- **Curry-Howard correspondence fully realized**: types are propositions, programs are proofs,
  `name: Proposition = Proof`
- **Unity**: compile-time predicates and ordinary functions use exactly the same syntax, with no
  conceptual split
- **SMT transparency**: the programmer doesn't need to know SMT exists; the mental model is
  consistent with type checking
- **Progressive adoption**: start with one compile-time predicate and gradually expand coverage
- **Minimum runtime overhead**: zero overhead in CompileTime mode; only necessary Bool checks
  retained in Runtime mode

### Disadvantages

- **Compile time**: SMT solving increases compile time, but the hard budget limit guarantees an
  upper bound
- **Automatic proof boundary**: complex predicates beyond first-order linear arithmetic may require
  the programmer to write proof functions. This is not a language defect — it is the inevitable
  conclusion of the halting problem. The compiler honestly reports Unproven rather than falsely
  reporting True/False
- **Learning curve**: writing effective compile-time predicates and proof functions requires
  understanding the basic intuition of the Curry-Howard correspondence
- **Implementation complexity**: unifying the compile-time proof pipeline requires careful design

### Risk Mitigation

- SMT solving budget hard limit (10,000 steps / 100ms / instantiation depth 3); exceeding the budget
  returns Unproven
- Dependent type pre-reduction: deterministic value computation is consumed first; SMT only chews on
  the non-deterministic part
- Unproven is not a dead end: correctness domain writes a proof function (return type is the
  proposition), termination domain gives a measure at the type position (§6.9) — both are verified
  by the type checker
- Incremental verification: only verify changed modules
- Clear error messages + counterexample display + budget consumption report + unsolved proposition +
  suggestions (if the compiler can give them)

## Alternatives

| Alternative                                                        | Why not chosen                                                                                                                                                                |
| ------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| RFC-022: `//!` annotation-style specifications                     | Specifications and types are split, violating the Curry-Howard correspondence                                                                                                 |
| Separate specification files (e.g. CVL)                            | Specifications separated from code, increasing maintenance cost                                                                                                               |
| Runtime-only assertions                                            | Cannot statically guarantee correctness                                                                                                                                       |
| External proof assistants (e.g. Coq)                               | Disconnected from the compiler, requires an independent proof language and trust boundary. YaoXiang's choice: proof is YaoXiang code, the type checker is the sole trust root |
| **This proposal: compile-time predicates as first-class citizens** | ✅                                                                                                                                                                            |

## Implementation Strategy

### Phases

| Phase       | Content                                                                                                                                                                |
| ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Phase 1** | Compiler kernel: structural equivalence + βδι-reduction + universal quantifier introduction/elimination. Support simple arithmetic predicates (`x > 0`, `arr.len > 0`) |
| **Phase 2** | SMT-LIB translation layer + Z3/CVC5 integration. Pipeline returns Proved/Disproved/Unproven. Support programmer-written proof functions when Unproven                  |
| **Phase 3** | Loop invariant VC generation + termination checking (four measure exploration strategies + `Terminates` explicit measure, §6, §6.9)                                    |
| **Phase 4** | Incremental verification + caching + IDE support                                                                                                                       |

### Dependencies

- RFC-010: Unified Type Syntax — compile-time predicates are based on `name: type = value`
- RFC-011: Generic Type System — compile-time predicates can take generic parameters
- RFC-009: Ownership Model — compile-time predicate expressions obey ownership rules

## Open Questions

- [x] **Z3 availability for wasm32 target**: no Z3 on wasm (SMT code is entirely excluded by
      `cfg(not(target_arch = "wasm32"))`). Impact scope and degradation direction: -
      `ownership.rs`'s `smt_cut` always returns `false` → back edge traversal → **conservative
      rejection** (sound direction, just less narrowing) - Termination checking Strategy 1,
      predicate level 2b/3 entirely skipped

  **Correction (2026-10-01)**: the original text claimed these paths are "currently **unreachable**"
  (because `predicate_defs` production was never populated, `parser` did not produce
  `MonoType::Refined`, `with_solver` production was never called) — this is no longer the case; all
  three have landed (predicate definition registration #377-3; refined type normalization;
  production pipeline injecting the solver). Therefore, the difference between wasm and native no
  longer lies only in the precision of `smt_cut`:

  | Path                                           | native   | wasm       | Degradation direction              |
  | ---------------------------------------------- | -------- | ---------- | ---------------------------------- |
  | Termination checking explicit measure judgment | Judgment | `Unjudged` | Don't report E4022 (less reported) |
  | Well-foundedness judgment                      | Judgment | `Unjudged` | Don't report E4022 (less reported) |
  | Refined predicate SMT implication level        | Execute  | Skip       | Falls to `Unproven`                |

  All are **conservative** directions (less narrowing, less reported), not affecting soundness. If
  precision consistency is required, the solution and cost are in issue #376 (conclusion: prefer
  JS-side Z3 instance, rather than linking Z3 into the main wasm — the latter requires changing the
  emcc build system and the artifact grows from 3 MB to ~20 MB).

- [x] **SMT solver choice**: default Z3 (MIT license, most widely validated). CVC5 as an
      SMT-LIB-compatible alternative, switchable via compiler flags. The compiler's internal
      translation target is the SMT-LIB 2.6 standard format — SMT-LIB is the abstraction layer; no
      custom generic solver interface.
- [x] **Specific budget values**: 10,000 steps / 100ms / quantifier instantiation depth 3. Fixed
      internally in the compiler, no knob. If real use cases prove these are insufficient in actual
      use (not "user wrote it wrong"), adjust.
- [x] **Quantifier support range**: the language does not limit quantifier order. Compile-time
      predicates accept Type arguments — Type includes function types — so higher-order quantifiers
      are a natural consequence of the type system and require no special syntax. The SMT solver can
      automatically judge first-order quantifiers (forall/exists, supports interleaved nesting,
      limited by budget depth 3). Higher-order quantifiers: SMT returns Unproven; the compiler
      prompts "this predicate exceeds the automatic proof range; please provide a proof function".
      The programmer writes a YaoXiang function whose return type equals the proposition — the type
      checker verifies that function. No external export needed, no AI needed, no interactive proof
      mode needed. Everything is YaoXiang code; everything is verified by the type checker.
- [x] **Counterexample formatting**: source variable names are used directly as SMT variable names
      (with module prefixes to avoid conflicts). When the Z3 model is returned, look up by variable
      name. Output format: variable name = concrete value + source location + predicate definition
      location. No complex mapping layer.
- [x] ~~**Interaction between compile-time predicates and `ref` smart pointers?**~~ → Decided:
      compile-time predicates only allow immutable borrows or values with already-transferred
      ownership. Values with mutable borrows cannot appear in compile-time predicates.
- [x] **Extension of `forall` predicate violation count measure to non-adjacent operations?** → No
      extension. Current coverage (adjacent swap, adjacent shift) is complemented by Strategy 1
      (linear rank function) — quicksort's outer interval contraction is covered by Strategy 1, and
      heapsort is covered by Strategy 1 (array index pattern). Loops whose termination cannot be
      proved by any strategy are directly rejected by the compiler — this is the hard safety
      philosophy, not a defect. If future real-world scenarios (non-academic constructions) have
      algorithms none of the four strategies can cover, we will revisit. → **Revisit triggered
      (2026-09-14, #318)**: non-structural recursion (gcd-like non-directly-decreasing cases, mutual
      recursion, merge partitions) — exactly the real-world scenario this clause anticipated —
      cannot be automatically handled by the template sequence of measure exploration, and cannot be
      rewritten into analyzable iteration patterns without breaking readability. Conclusion: the
      hard safety philosophy is unchanged (a measure written incorrectly is still rejected by SMT
      counterexamples; a person can only fail to prove, never prove incorrectly), but "reject if
      exploration fails" is relaxed to "if exploration fails, the programmer can give a measure
      explicitly at the type position" — see §6.9.
- [x] **Linear rank function enumeration combinatorial explosion**: candidate enumeration is capped
      at 3 bounded variables. ≤3: enumerate all linear combinations and verify with SMT one by
      one. >3: try only single-variable measures (`v_i`, `u_i - v_i`); failure reports a compile
      error directly — prompting the programmer "the loop has >3 bounded variables; the compiler
      cannot automatically synthesize a multi-variable measure". This is not an engineering
      compromise — it forces the programmer to write simpler loops.

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
│ Under       │  ← Current state: community discussion
│ Review      │
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
