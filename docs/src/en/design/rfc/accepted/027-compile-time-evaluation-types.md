---
title: 'RFC-027: Compile-Time Predicates and Unified Static Verification'
status: 'Accepted'
author: 'ChenXu'
created: '2026-06-07'
updated: '2026-09-14'
impl_status: 'in_progress'
impl_detail:
  'Phase 1-2 complete, Phase 3 partially complete, Phase 4 partially complete. The unified
  assert/Assert solution has all 6 phases implemented (#157-#162 closed): Never type, IsTrue
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
> - [RFC-010: Unified Type Syntax — the `name: type = value` Model](../accepted/010-unified-type-syntax.md)
> - [RFC-011: Generics System Design](../accepted/011-generic-type-system.md)
> - [RFC-024: Concurrency Model Based on `spawn` Blocks](../accepted/024-concurrency-model.md)
>
> **Supersedes**:
> [RFC-022: Hoare Logic Static Verification Support (Specification Annotations and Specification Types)](../deprecated/022-hoare-logic-static-verification.md)
> — deprecated

## Abstract

This RFC proposes introducing **compile-time predicates** as first-class citizens in YaoXiang,
unifying all compile-time static verification into a single **proof pipeline**. A compile-time
predicate is not an external specification annotation—it is a function. A function that returns
`Type` can be used at type positions; the compiler invokes it at compile time and checks the return
value. Types are propositions, compile-time evaluation is proof.

**Core thesis**: The only job of the type checker at compile time is to construct and validate proof
terms. Type equality, token conflicts, dependent type reduction, compile-time predicate evaluation,
Hoare logic implications—all are different type checks in the compile-time proof pipeline, sharing
the same pipeline. The SMT solver is an accelerator module of the type checker, not a separate trust
boundary. When the compiler returns `Unproven`, the programmer writes a YaoXiang function as a
proof—the type checker validates it in exactly the same way it validates any function's return type.
Everything is YaoXiang code, everything is verified by the type checker.

## Motivation

### Why deprecate RFC-022?

RFC-022 designed specifications in the form of `//!` comments:

```yaoxiang
max: (T: Ord) -> ((arr: Array(T, n)) -> T) = {
    //! requires: NonEmpty(n) = n > 0          ← This is an annotation independent of types
    //! ensures: ExistsMax(result, arr[0..n])   ← This is an annotation independent of types
}
```

This commits a fundamental error against the Curry-Howard correspondence: **it splits specifications
and types into two layers**. Annotations are not types. Annotations do not participate in type
checking. Annotations fit an "external tool" mental model.

The white paper states clearly:

> "No `//!` annotations. No separate specification language. Everything lives within the type
> system."

### Current Problems

- RFC-022's `//!` annotations are an external syntax independent of the type system
- Specification types and ordinary types are two systems, creating conceptual redundancy
- The Debug Build verifies / Release Build ignores split pattern destroys unity
- SMT solvers are conventionally positioned as external tools—YaoXiang instead builds them in as
  accelerator modules of the type checker
- Type checking, borrow checking, compile-time predicate checking, and macro expansion each take
  different paths

### The Correct Mental Model

Type checking can be abstracted as a function:

```
verify : Program → Proved | Disproved(Model) | Unproven
```

All compile-time checks—simple type matching, borrow conflict detection, compile-time predicate
verification—are subtasks of this function. They share the same proof pipeline, differing only in
proof term complexity and construction strategy.

When the compiler returns `Unproven`, the programmer provides a proof function—whose return type
equals the proposition to be proved. The type checker validates it. This is the same operation as
ordinary type checking.

## Proposal

### 1. `{}` Is the Proof Space: Types Are Assertions, Verification Is Type Checking

YaoXiang's `{}` is the compile-time proof space. Everything inside is an assertion, and the compiler
guarantees every item is `True`—either by automatic proof or via a programmer-supplied proof
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
#          parameter in signature  only assertions inside {}
#          compiler verifies x > 0 at compile-time call

List: (T: Type) -> Type = { data: Array(T) }
#      ^^^^^^^^              ^^^^^^^^^^^^^^^
#      parameter in signature  compiler verifies type_of(T) == Type, type_of(data) == Array(T)
```

Same pattern: `name: (params) -> Type = { assertion }`. The compiler does not distinguish "type
assertions" from "value assertions"—all are evaluation targets in the proof pipeline.

**Loop invariants don't need to be written separately. Type annotations on variables are the
Floyd-Hoare invariants.**

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # annotation references i—tells compiler s's type depends on i
    mut i: UpTo(arr.len) = 0     # at initialization i=0, verify: 0 == sum(arr[0..0]) → True
    while i < arr.len {
        s += arr[i]  # compiler verifies: s_new == sum(arr[0..i+1])
        i += 1       # i changes → triggers s's dependent re-verification: s satisfies SumUpTo(arr, i_new)
    }
    return s  # s: SumUpTo(arr, arr.len) = sum(arr[0..arr.len])
}
```

The compiler generates a verification condition for the loop body once—inductive hypothesis (type
annotation) → assignment operation → whether the new value satisfies the type annotation. Once the
proof pipeline verifies the inductive step, all iterations are automatically covered. No
`: decreases`, no `: Invariant`, no inductive proof needed—the compiler decomposes induction into a
local VC for each assignment.

### 2. Pre/Postconditions: Compile-Time Predicates on Parameter and Return Types

Drop RFC-022's `//! requires`/`//! ensures`. Compile-time predicates are placed as parameter or
return type annotations.

**The parameter side is a function call.** A compile-time predicate is a function returning `Type`,
and the way to use it on the parameter side is to call it—just like `factorial(5)`. The return type
side introduces a new concept: the return value parameter.

```yaoxiang
# Precondition: explicitly invoke compile-time predicate in parameter type
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b
#                       ^^^^^^^^^^  b is the current parameter name, passed to Positive as argument
#                       compiler extracts argument value at call site, substitutes b, verifies Positive(arg)
#                       e.g.: divide(10, 2) → verifies Positive(2) = { 2 > 0 } → True
#                       e.g.: divide(10, 0) → verifies Positive(0) = { 0 > 0 } → False → compile error

# Postcondition: return value parameter + compile-time predicate
IsMax: (T: Ord, arr: Array(T), result: T) -> Type = {
    forall j in 0..arr.len: result >= arr[j]
}

NonEmpty: (arr: Array(T)) -> Type = { arr.len > 0 }

max: (T: Ord) -> ((arr: NonEmpty(arr))) -> (result: IsMax(T, arr, result)) = {
#                                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
#                                            result is the return value parameter, value supplied by return
#                                            compiler substitutes the return value at return point, verifies postcondition
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
  its value supplied by the `return` statement. `result` exists only in the type signature, is
  referenced only by predicates, does not enter the function body scope, and does not appear at the
  call site.
- **Return value parameter is optional**: omit it when there are no postconditions; the signature is
  exactly the same as an ordinary function (`-> Int`).
- **Unity**: parameters and return value parameters are the same
  concept—`paramName: predicateCall(paramName)`—the only difference being whether the value is
  supplied by the caller or by `return`.

### 3. Path Condition Propagation: Compile-Time Verification of Runtime Values

When a compile-time predicate is used at a binding position, arguments are explicitly passed by the
programmer. When runtime values flow into refined type arguments, the compiler completes
verification through path condition collection and SMT implication—without requiring the programmer
to explicitly pass a proof.

#### 3.1 Explicit Function Calls

When a compile-time predicate is used at a binding position, arguments are explicitly passed by the
programmer—it's a function call, zero implicits.

`Positive: (x: Int) -> Type = { x > 0 }` is a compile-time predicate constructor. When it appears at
a binding position (parameter declaration, variable declaration, return type), the programmer
explicitly passes already-bound variable names:

```yaoxiang
b: Positive(b)
// b is already declared as the current parameter, Positive(b) is a function call
// After normalization: b: { b > 0 }
```

The compiler does not need to implicitly fill in arguments—`b: Positive(b)` is the same as `f(5)`,
just a function call. `b` is bound as a parameter name, and its type annotation `Positive(b)`
references `b` itself—this is the standard dependent type pattern, not an implicit expansion rule.

**Unification with RFC-010's `self`**: RFC-010 establishes that `self` is not a keyword, just a
conventional parameter name ("writing it as `p`, `this`, `x` has exactly the same effect").
`b: Positive(b)` shares the same mechanism—the parameter name can be referenced in the type
annotation. `self` appears in the position `self: Point`, `b` appears in the position
`b: Positive(b)`; both annotations reference the parameter itself. The only difference is the
complexity of the annotation, the mechanism is exactly the same—once the name is bound, the type can
depend on that name.

Return types use explicit function calls as well:

```yaoxiang
Sorted: (arr: Array(T)) -> Type = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }

sort: (arr: Array(T)) -> (result: Sorted(result)) = { ... }
//                        ^^^^^^^^^^^^^^^^^^^^^^^
//                        result is the return value parameter, Sorted(result) is a function call
//                        compiler substitutes the return value into result at return point, verifies Sorted(return value)
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
conditions, forming the current scope's **assumption set**. These assumptions serve as background
knowledge for compile-time `Bool` evaluation.

```yaoxiang
if y > 0 {
    // compiler automatically acquires the assumption here: { y > 0 }
    let result = divide(x, y)
    // verification condition: (y > 0) ⇒ (y > 0)
    // proof pipeline judges implication holds → Proved
} else {
// this branch assumes: { !(y > 0) }
// if divide(x, y) were called, the verification condition would be !(y > 0) ⇒ y > 0
    // proof pipeline judges implication does not hold → Disproved
}
```

This is not a hard-coded special-case pattern in the compiler—this is the natural behavior of the
compile-time proof pipeline. Each type check call site sends to the pipeline:

```
{background assumptions} ⇒ {verification target}
```

The proof pipeline judges implication. `Proved` → pass, `Disproved` → compile error +
counterexample, `Unproven` → compile error + unresolved proposition. Background assumptions come
from the path conditions of the current program point.

#### 3.3 Assumption Stack

During control flow analysis, the compiler maintains an assumption set for each basic block:

- **if-guard**: `if y > 0` → true branch pushes `y > 0`, false branch pushes `!(y > 0)` (if `else`
  is used)
- **match pattern**: `if let Some(v) = opt` → branch pushes `opt == Some(v)`
- **logical conjunction**: `if x > 0 and y < 10` → branch pushes `x > 0` and `y < 10`
- **function precondition**: when calling `divide(a, b)`, `b` must satisfy the evidence for
  `Positive`—either from current assumptions, or from the refined type annotation on the argument
  itself (if `b` is already annotated as `Positive`, its type carries `b > 0`)
- **assignment**: when `let z = y`, the refined conditions already on `y` transfer to `z`

All assumptions enter the compile-time proof pipeline. When entering the SMT accelerator path, they
translate to SMT-LIB background assertions.

#### 3.4 No Static Evidence, Compile Error

If the programmer directly writes:

```yaoxiang
divide_user_input: (x: Int, y: Int) -> Int = divide(x, y)
```

There is no assumption `y > 0` at the current program point, and the argument `y` itself has no
`Positive` type annotation. The verification condition is:

```
{} ⇒ { y > 0 }
```

The pipeline returns `Disproved` (implication does not hold) → compile error:

> Cannot prove that parameter `b` satisfies `Positive` in the `divide` call. `y` comes from function
> input, no proven bound exists. Consider guarding the call with an `if` branch:
> `if y > 0 { divide(x, y) }`.

YaoXiang does not accept runtime values flowing directly into refined type arguments without
providing static evidence. This is not a restriction—this is the core of the hard-safety philosophy.
Any code the compiler cannot statically prove shall not pass compilation.

#### 3.5 Relationship to the Unified Pipeline

Path condition propagation is not an extra mechanism. It is the direct extension of the compile-time
proof pipeline on control flow analysis:

| Stage                             | Responsibility                                                                                                                                            |
| --------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Path condition collection         | Compiler control flow analysis phase, annotates each basic block with assumption sets                                                                     |
| Verification condition generation | When encountering type constraints to verify, merge path conditions + argument type info                                                                  |
| Proof pipeline evaluation         | Compiler kernel → SMT accelerator → derives Proved / Disproved / Unproven                                                                                 |
| Result                            | `Proved` → pass; `Disproved` → compile error + counterexample; `Unproven` → compile error + unresolved proposition (programmer may supply proof function) |

No new component. No special rules. Path conditions are background knowledge for the proof
pipeline—sharing the same pipeline and the same budget system as type equality and borrow
constraints.

### 4. The Compile-Time Proof Pipeline

All compile-time checks share the same pipeline. The core operation of the pipeline is **type
checking**—checking whether the type of a proof term equals the proposition to be proved. Everything
is type checking.

```
Compiler encounters a Bool expression needing evaluation (i.e.: needs to construct a proof term)
        │
        ├── Type equality (T1 == T2)
        │   → compiler direct judgement (structural equivalence)
        │
        ├── Token conflict condition (!conflicting(tokens))
        │   → flow-sensitive liveness analysis (Dup/Linear property tracking)
        │
        ├── Dependent type reduction (n + m simplification)
        │   → compile-time term rewriting system (βδι-reduction)
        │
        ├── Compile-time predicate (x > 0, forall...)
        │   → compiler itself + SMT accelerator module
        │
        └── Hoare logic implication (P ⇒ Q)
            → compiler + SMT accelerator module
                    │
                    ▼
             ┌──────────┐
             │ Proved   │  → compilation passes
             │ Disproved│  → compile error + counterexample
             │ Unproven │  → compile error + unresolved proposition
             └────┬─────┘
                  │
                  ▼
         Programmer writes proof function (YaoXiang code)
                  │
                  ▼
         Type checker validates ──→ Proved ──→ compilation passes
                  │
                  ▼
            Validation fails → compile error: "proof does not hold"
```

#### 4.1 Proof Results: A Three-Valued Algebra

Compile-time evaluation returns three results—this is the inevitable conclusion of the halting
problem, and the natural division of proof theory:

```
eval_compile_time : BoolExpr → Proved | Disproved(Model) | Unproven
```

- **Proved** → halts, proof term constructed, type check passes. Compilation continues.
- **Disproved(M)** → halts, counterexample M exists. Compile error + counterexample + source
  location.
- **Unproven** → proof not constructed within given resource budget. Compile error + unresolved
  proposition + budget consumption report.

**Unproven ≠ False.** The compiler saying "I cannot prove it" is not equivalent to the proposition
being false—it just exceeds current automatic proof capabilities. This is honesty, not a defect.

A hard budget limit is the engineering solution to the halting problem. No knob is provided—giving
one would be asking the user "do you think your program will halt", and neither the user nor the
compiler knows.

#### 4.2 After Unproven: The Programmer Writes the Proof

When the compiler returns `Unproven`, the programmer can write a **proof function**—a YaoXiang
function whose return type equals the proposition to be proved. The type checker validates this
function—the same mechanism by which it validates `add(a, b): Int`.

```
Proposition = Type
Proof       = Program (a value of that type)
Validation  = Type check (the only trust root)
```

The SMT solver is not an independent trust boundary—it is an **accelerator module of the type
checker**. SMT helps find proofs, but the proof is always verified by the type checker. When SMT
returns `unsat`, the compiler reconstructs its result into a proof term verifiable by the type
checker. If reconstruction fails (SMT's inference steps exceed the compiler kernel's inference
rules), it falls back to `Unproven`—the programmer can then manually write a proof function.

```yaoxiang
# Proposition: refined property the compiler cannot automatically prove
FirstIsMin: (T: Ord, arr: Sorted(T)) -> Type = {
    forall i in 0..arr.len: arr[0] <= arr[i]
}

# Proof: programmer writes a function, return type is the above proposition
# Type checker validates this function—exactly the same as validating add(a,b): Int
first_is_min: (T: Ord, arr: Sorted(T)) -> FirstIsMin(T, arr) = {
    # compiler validates here: function body's type = FirstIsMin(T, arr)
    ...
}
```

No AI needed, no export to Coq, no new concepts. **A property the compiler cannot automatically
prove at compile time → programmer writes the proof in YaoXiang code → type checker validates it.**
The whole process is a smooth gradient—the compiler does the easy proofs, leaving the hard ones for
the brain.

#### 4.3 Hierarchical Dependencies Within the Pipeline

The above evaluators share the same interface but have an evaluation order. Type equality is a
prerequisite for all subsequent analysis; ownership/token checks depend on type information; refined
predicate verification depends on the results of the first two layers. The compiler evaluates layer
by layer, expressions that fail at lower layers do not enter upper layers—avoiding wasting solver
budget on type-incorrect programs.

```
Evaluation order (same pipeline, layered scheduling)
├── Layer 0: Type equality (T1 == T2)
│   └── structural unification → if fails, subsequent is meaningless, return Disproved directly
├── Layer 1: Ownership/token conflicts
│   └── flow-sensitive liveness analysis → if fails, memory safety does not hold, return Disproved directly
└── Layer 2: Refined predicates / Hoare implications
    └── compiler itself → SMT accelerator → derive Proved / Disproved / Unproven
```

Each layer still returns `Proved/Disproved/Unproven`, sharing the same interface and the same budget
system.

### 5. Unification of Three Function Layers

| Layer                  | Execution Timing | Input      | Output | Example                                        |
| ---------------------- | ---------------- | ---------- | ------ | ---------------------------------------------- |
| Value-level function   | Runtime          | Value      | Value  | `add: (a: Int, b: Int) -> Int = a + b`         |
| Type constructor       | Compile-time     | Type/Value | Type   | `List: (T: Type) -> Type = { data: Array(T) }` |
| Compile-time predicate | Compile-time     | Value      | Type   | `Positive: (x: Int) -> Type = { x > 0 }`       |

All use the same `name: type = value` syntax. Compile-time predicates and type constructors take the
same compile-time proof pipeline—`{}` is the proof space.

### 6. Loops: Floyd-Hoare Verification Condition Generation

Loops don't need separate `: Invariant(...)` or `: decreases(...)` annotations. Compile-time
predicate type annotations on variables define Floyd-Hoare-style assertions—the compiler generates
verification conditions from the type annotations, and the proof pipeline checks whether each
assignment preserves the type.

Core mechanism: each assignment operation corresponds to a Hoare triple `{P} x := e {Q}`, with the
verification condition `P ⇒ Q[e/x]`. The compiler generates a verification condition once for the
loop body—once the proof pipeline verifies the inductive step, all iterations are automatically
covered.

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # annotation references i; at init i=0, verify: 0 == sum(arr[0..0]) → True
    mut i: UpTo(arr.len) = 0     # verify: 0 <= 0 <= arr.len → True
    while i < arr.len {
        # compiler generates one VC for the loop body. Precondition: s satisfies SumUpTo(arr, i), i satisfies UpTo(arr.len).
        #
        # s += arr[i]:
        #   verification obligation: s_new satisfies SumUpTo(arr, i) (current i unchanged)
        #   substitute s_new = s_old + arr[i]:
        #     need s_old + arr[i] == sum(arr[0..i+1])
        #     from inductive hypothesis s_old == sum(arr[0..i]), add arr[i] to both sides:
        #     sum(arr[0..i]) + arr[i] == sum(arr[0..i+1])
        #   compiler + SMT: linear arithmetic, milliseconds → Proved
        #
        # i += 1:
        #   i changes → s's type annotation in dependency graph references i → triggers re-verification
        #   new verification target: s satisfies SumUpTo(arr, i_new)
        #   i.e. s == sum(arr[0..i_new]), guaranteed by previous step → Proved
        s += arr[i]
        i += 1
    }
    return s  # at this point s: SumUpTo(arr, arr.len), i.e. s == sum(arr[0..arr.len])
}
```

Loop invariants are simply type annotations on variables—the programmer writes types, the compiler
checks the inductive step. The compiler doesn't need to "discover" invariants, nor "do induction
automatically"—it decomposes the inductive proof into local verification conditions for each
assignment operation, leaving them to the proof pipeline to conquer one by one.

#### 6.1 Dependency Tracking: Dependent Types on Mutable Variables

The prerequisite for the above mechanism is: the compiler knows that `s`'s type annotation
`SumUpTo(arr, i)` references `i`—when `i` changes, `s`'s type constraint changes accordingly. This
requires the compiler to maintain a **type dependency graph between variables**.

**Data structure**:

```
TypeDepGraph: Map<VarName, Set<VarName>>
# Key is the depended variable, value is the set of variables that reference it in their type annotations
# e.g.: { i: {s}, j: {s, t}, ... }
```

**Construction**: when the type checker processes `mut v: Pred(... x ...) = init`, it parses free
variable references in `Pred(...)`'s arguments. If the arguments reference other mutable variables
`x` in the current scope, it records `x → v` in the dependency graph.

**Trigger**: when the depended variable `x` is assigned, the compiler:

1. Looks up the set of variables `{v₁, v₂, ...}` depending on `x` in the dependency graph
2. For each `v`, generates a verification condition: does `v`'s current value satisfy the updated
   type `Pred(... x_new ...)`
3. Sends the VC into the proof pipeline

**Assignment order sensitive**: dependency tracking naturally enforces correct assignment order.
Take `SumUpTo(arr, i)` as an example:

```yaoxiang
# Correct order
s += arr[i]   # s_new satisfies SumUpTo(arr, i+1)
i += 1        # i changes → re-verify s satisfies SumUpTo(arr, i_new) → True

# Wrong order—compiler rejects
i += 1        # i changes → re-verify s satisfies SumUpTo(arr, i_new)
              # s not yet updated, s_old == sum(arr[0..i_old]) ≠ sum(arr[0..i_new])
              # → compile error: variable s does not satisfy type SumUpTo(arr, i_new)
s += arr[i]   # unreachable
```

**Combined dependencies**: a variable can depend on multiple variables. The type annotation
`{ v: Int; v == x + y }` depends on both `x` and `y`—a change in either triggers re-verification.

**Relationship to the proof pipeline**: dependency tracking is a trigger for VC generation, not an
independent verification mechanism. It answers "when is it necessary to generate a VC"—the proof
pipeline answers "does the VC hold".

### 7. Termination Checking

**Scope of application: refined types.** Termination is not an independent switch, but part of the
**verification mode**: once a type is refined (`Refined { base, constraint }`), the computation it
annotates enters verification mode, which requires termination; unrefined ordinary types do not
enter verification mode and generate no termination obligations.

This yields two corollaries:

- **Loops**: bare `while` does not enter verification mode; when a metric variable carries a refined
  annotation (e.g. `i: UpTo(n)`), it enters verification mode and must prove termination.
- **Recursion**: when a function's signature is refined (parameter refinement or return type
  contains refinement), it enters verification mode and must prove that the metric strictly
  decreases at each recursive call site.

Automatic priority within the mode: the compiler first automatically explores metrics, those
provable pass; if exploration fails and no explicit metric is given, compile error. No **annotation
syntax** loophole—metrics and termination propositions are written at the type position, no new
`decreases`-like syntax is introduced. The form of metrics and explicit fallbacks is described in
§6.9.

#### 6.1 Design Principles

The compiler automatically extracts the information needed for termination proofs from two places:

1. **Variable type annotations**: boundary constraints in refined types (e.g. `UpTo(n)` gives upper
   bound `n` and lower bound `0`)
2. **Loop body operations**: the operations applied to variables on each iteration

The compiler tries four metric synthesis strategies in priority order, stopping as soon as one is
found. The four strategies are a **restricted template sequence for metric exploration**, whose
input is refined constraints (all four strategies take "variables with bounded types" as the
starting point), not "compile-time-evaluated code"; they are the same thing as the explicit metrics
in §6.9, with one side automatic and the other manual.

> **Metric exploration is exploration, not inference.** Exploration only enumerates templates
> (linear rank, violation count, bounded pattern, multiplicative scaling), and does not guarantee a
> solution exists—general metric inference is undecidable (reduces to the halting problem).
> Therefore cases outside the templates must allow the programmer to explicitly give a metric
> (§6.9), otherwise compile error.

#### 6.2 Strategy 1: Automatic Linear Rank Function Synthesis

When variables have linear bound annotations, the compiler enumerates candidate linear metrics and
SMT-validates them.

```
Input:
  Variables v₁: UpTo(u₁), v₂: UpTo(u₂), ... (variables with upper and lower bounds)
  Loop condition cond
  Set of assignments in the loop body

Algorithm:
  1. Extract each variable's bounds from type annotations: [low_i, high_i]
  2. Enumerate candidate metrics: v_i, u_i - v_i, v_i - v_j, etc. linear combinations
  3. For each candidate metric m:
     - SMT verify m ≥ 0 (derived from type bounds)
     - For each execution path in the loop body, SMT verify m' < m (strictly decreasing)
  4. Find a linear combination that satisfies the conditions → termination proven
```

Coverage: loops where any variable is assigned a linear expression (`v = a·v + b`) with bounded type
annotations. Including `i += const`, `i -= const`, and binary-search-style interval shrinking:

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

#### 6.3 Strategy 2: Predicate Violation Count—Automatic Metric Extraction from Target Type <span style="color:orange">[Experimental Strategy]</span>

> ⚠️ **Current status: experimental strategy, feasibility to be determined during Phase 3
> implementation.** This strategy is effective for adjacent swap operations (bubble sort, insertion
> sort), but cannot automatically prove non-adjacent operations (quicksort partition, heapsort
> sift-down). Coverage boundaries are in the table below. If Phase 3 verification proves infeasible,
> this strategy will be removed or downgraded to future work.

Core insight: **the specification the user writes is raw material for compiler reasoning.** The
compiler doesn't need to know "what sorting is" internally—it reads the `Sorted` definition and
automatically extracts the metric from it.

```
Input:
  Target type: Sorted(arr) = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }
  Loop body operation: adjacent element swap

Algorithm:
  1. Parse predicate definition: forall i in range: cond(i, arr)
  2. Automatically generate metric: violation_count = |{ i | ¬cond(i, arr) }|
  3. Analyze the effect of operations on the metric:
     - adjacent swap arr[j], arr[j+1] = arr[j+1], arr[j]
     - only affects index pairs j-1, j, j+1
     - if arr[j] > arr[j+1] (violates predicate), the pair satisfies the predicate after swap
     - violation_count decreases by at least 1
  4. Upper bound: n·(n-1)/2 (maximum adjacent inversions), lower bound: 0
  → termination proven
```

**Current coverage**:

| Algorithm      | Operation Pattern | Strategy 2 Provable? | Reason                                                  |
| -------------- | ----------------- | :------------------: | ------------------------------------------------------- |
| Bubble sort    | adjacent swap     |          ✅          | violation_count strictly decreases with each swap       |
| Insertion sort | adjacent shift    |          ✅          | each shift eliminates one violating pair                |
| Selection sort | non-adjacent swap |          ❌          | a single swap may increase violation_count              |
| Quicksort      | partition         |          ❌          | non-adjacent swap, no guaranteed monotonic decrease     |
| Heapsort       | sift-down         |          ❌          | tree-shaped operation, violation_count is non-monotonic |

**Complementary strategy**: for quicksort, the `low < high` interval shrinking can be covered by
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

#### 6.4 Strategy 3: Bounded Increment/Decrement Pattern

`v += const` (positive constant), variable has upper-bound type annotation → metric
`upper_bound - v` decreases by `const` each time, lower bound 0. This is a degenerate case of
Strategy 1, handled quickly at the front by the compiler.

#### 6.5 Strategy 4: Multiplicative Scaling Metric Template

`v *= const` (const > 1), variable has upper and lower bound type annotations. The compiler has a
built-in logarithmic metric template `ceil(log_const(upper/v))`, each multiplication by const
decreases the metric by 1.

```yaoxiang
mut i: Positive(i) = 1
while i < n {
    # compiler auto-derives: metric ceil(log₂(n/i)), each multiplication by 2 decreases the metric by 1
    i *= 2
}
```

#### 6.6 Separation of Termination and Correctness

Termination proof and correctness proof are independent:

- **Termination**: the four strategies above automatically prove the loop exits in finite steps; if
  exploration fails, the programmer gives the metric at the type position (§6.9)
- **Correctness**: whether the loop body advances toward the target type, checked by the
  compile-time proof pipeline through verification conditions

Both pass → compilation passes. Termination proven but correctness fails → compile error +
counterexample. Correctness proven but termination cannot be proven → compile error pointing out the
variables or operations that cannot be analyzed. Both fail → compile error reports the two failure
causes separately.

#### 6.7 Termination Checking for Recursive Functions

For recursive functions with refined signatures, the compiler checks that the formal parameters
decrease at each recursive call site:

```yaoxiang
gcd: (a: Int, b: NonNegative(b)) -> Int = {
    if b == 0 { return a }
    return gcd(b, a % b)  // compiler explores: well-founded ordering of (b, a % b) decreases → terminates
}
```

Formal parameter decrease is the strongest automatic path (structural recursion). If exploration
fails, the programmer explicitly gives the metric at the type position (§6.9).

#### 6.8 Hard Boundary

`i = f(i)` where `f` is non-invertible, non-closed, and preserves no monotonicity—mathematically
impossible to automatically prove termination. Compile error:

> This loop cannot be automatically proven to terminate. The loop variable depends on the
> unanalyzable function `f`. Please use an iteration pattern that can be analyzed by the compiler,
> or give this loop a name and provide a metric at the type position (§6.9).

This is not a compiler failure. Any code that cannot be statically proven safe shall not pass
compilation. Even when an explicit metric is given, it must be judged true by SMT to pass—an
incorrectly written metric will be rejected by counterexample; humans can only fail to prove, not
prove wrong.

#### 6.9 Explicit Metric: `Terminates`

When automatic exploration cannot find a metric, the programmer writes the metric at the **type
position**—using the same mechanism (`predicate application`) as `Positive(b)` and
`IsMax(T, arr, result)`, zero new syntax:

```yaoxiang
// Metric: ordinary function, unit-testable, reusable, not involved at runtime
gcd_measure: (a: Int, b: Int) -> Int = { b }

// Termination component lives in the function's own type position
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// Loop: bound name is the anchor, the metric gets values from names in scope
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n {
        i = i + 1
    }
    return acc
}
```

The reason for this loop form: `Terminates(m)` refines the value type of the loop body's tail
expression, with the anchor provided by the binding name `acc`—the loop is thus denotable, with no
dead corner of "anonymous constructs being denotation-less".

**`Terminates` is a built-in predicate**, alongside `Int`, `Never`, in the core primitives. It is
the only predicate whose **function body is written by the compiler**—its assertion ("the metric
strictly decreases at each recursive call site / loop back edge") lives in the computational
structure, and user-written predicate references cannot reach the function body or loop body, so it
cannot be expressed by the predicate definition syntax in the "Syntax" section. The built-in face
converges to this single name.

**Arity**. `Terminates(FnType, m)` and `Terminates(m)` are two arities of the same predicate, not
two constructs:

| Form                    | Anchor                       | Use                                                            |
| ----------------------- | ---------------------------- | -------------------------------------------------------------- |
| `Terminates(m)`         | Name of the binding it is in | Self-recursive functions, loops—default form                   |
| `Terminates(FnType, m)` | Explicit function type       | Scenarios like mutual recursion where the anchor is not unique |

Both are essentially the same: the termination obligation always falls on "the computation annotated
by the type position where the refinement lives".

**The metric does not restrict the return type.** The metric can be an expression of any type (not
forced to be a natural number); the "strict decrease" on it is given by the well-founded ordering
available on that type. Whether the metric is well-founded (e.g. when returning `Int`, whether it is
`>= 0`) is an **independent obligation**, equally handed to refined inference or SMT; when neither
can derive it, the diagnosis **does not directly reject**, but suggests a direction to check
(whether the metric's lower bound holds, whether the recursive parameter is really moving in that
direction).

**Relationship with automatic exploration**: the explicit metric is not a separate pipeline, but the
input after exploration fails. After the metric is given, the same SMT is used to verify decrease
and well-foundedness; if it does not hold, an error with a counterexample is reported.

The implementation mechanisms for obligation generation, judgment pipeline, diagnostic direction,
and shared metrics for mutual recursion (SCC) are in
[RFC-027a: Explicit Metrics for Termination Checking](../review/027a-termination-explicit-measure.md).

### 8. SMT Solver: An Accelerator Module of the Type Checker

In traditional languages, the SMT solver is an external tool (e.g. F\* calls Z3, Dafny calls Z3). In
YaoXiang, it is an **accelerator module of the type checker**—invoked only when the compiler kernel
itself cannot directly judge. SMT helps find proofs, but it is the type checker that verifies the
proof.

**Trust model**: the type checker is the only trust root. The SMT solver is an accelerator module—it
helps find proofs, but SMT is not an independent trust boundary. The compiler trusts Z3's `unsat`
result (consistent with the F\*/Dafny route—the probability of Z3 errors is lower than the
compiler's own bug rate, which is an engineering pragmatic choice). The true unreliability is
controlled at the SMT translation layer—if the translation has a bug, the compiler will surface it
in other tests.

**Interface**: the compiler internally translates to the SMT-LIB 2.6 standard format, rather than
binding to a specific solver API. SMT-LIB is an ISO standard, and Z3, CVC5, MathSAT, Yices all
support it natively.

**Default backend**: Z3 (MIT license, most extensive documentation and community validation). CVC5
as an SMT-LIB-compatible alternative—users can switch via compiler flags at compile time.

No "general solver abstraction layer"—SMT-LIB is the abstraction layer. If CVC5 makes a breakthrough
in a particular theory in the future, switching only requires swapping the binary, no compiler code
changes.

```
Compile-time Bool expression
        │
        ├── Compiler kernel can directly judge (structural equivalence, simple arithmetic,
        │   trivial formulas after constant folding)
        │   → directly return Proved / Disproved
        │
        └── Compiler kernel cannot directly judge (quantifiers, symbolic variables)
            → dependent type pre-reduction (factorial(5) → 120)
            → translate to SMT-LIB format
            → send to Z3/CVC5 (with budget limit)
            → return value: unsat → Proved  │  sat + model → Disproved  │  unknown → Unproven
```

**Solver budget—hard limit, like stack depth**:

| Budget Dimension               | Default | Description                                                                                                                                                      |
| ------------------------------ | ------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Solver steps                   | 10,000  | Z3 usually takes within a hundred steps for linear arithmetic. 10,000 steps covers 99% of practical predicates.                                                  |
| Time                           | 100ms   | A single predicate over 100ms = user is writing a compile-time program rather than a type annotation. 100ms × 50 predicates = 5-second compile time upper limit. |
| Quantifier instantiation depth | 3       | Three layers of nested quantifiers cover practical patterns. Beyond three layers, the user is likely writing logic exercises.                                    |

Over-budget returns `Unproven`, compile error + predicate location + consumption. No degradation, no
runtime check, no silent pass.

**Why this is actually feasible**: in practice, 95% of real predicates are linear
arithmetic—`x > 0`, `arr.len > 0`, `0 <= idx < arr.len`—all within decidable fragments, and SMT
solvers return for such problems in milliseconds. For the rare complex predicates that exceed the
budget, the programmer simply writes a proof function.

Dependent type does a layer of pre-reduction before calling SMT: `factorial(5)` directly evaluates
to `120` at compile time, `append([1,2], [3])` directly evaluates to `[1,2,3]`. These deterministic
value computations do not consume SMT budget.

The programmer doesn't need to know SMT exists. The mental model is: **the compiler proves it if it
can, errors if it can't—if the compiler can't, you can write a function to prove it to it.**

### 9. Compile-Time Predicate Composition

Compile-time predicates are functions returning `Type`; composition is naturally achieved through
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

result = divide(10, 2)   # ✅ compiler verifies Positive(2) = { 2 > 0 } → True
# result = divide(10, 0)  # ❌ compiler verifies Positive(0) = { 0 > 0 } → False
```

#### 9.2 Array Access Safety

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

### 11. Dispatch Pipeline: Unified Dispatch of Compile-Time and Runtime

`assert` and `Assert` are two sides of the same refined type primitive. The dispatch pipeline
`dispatch` automatically decides whether to take compile-time proof or runtime check based on
**whether the predicate's free variables are reachable at compile time**:

| Criterion                                                                                  | Mode            | Behavior                                                                                  |
| ------------------------------------------------------------------------------------------ | --------------- | ----------------------------------------------------------------------------------------- |
| All free variables known at compile time (generics parameters, compile-time constants)     | **CompileTime** | Enters proof pipeline: Proved → erase, Disproved → compile error, Unknown → require proof |
| Some free variables come from runtime (function parameters, external input, mut variables) | **Runtime**     | Insert runtime check, and inject refined facts into the flow-sensitive assumption set Γ   |

**Key point**: "Cannot judge" ≠ "Disproved". In CompileTime mode, Unknown requires proof (no silent
degradation); in Runtime mode, the proposition is simply not true or false at compile time—no
prover, however strong, can write a tautological proof for "the user might have entered a negative
number", runtime check is the only sound choice. This is not because the prover isn't strong enough,
it is a theoretical necessity.

### 12. Flow-Sensitive Assumption Set Γ: Strongest Postcondition Propagation

The compiler maintains a flow-sensitive assumption set Γ, tracking propositions known to hold at
each control flow point.

**SP (strongest postcondition) propagation**:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
```

**kill set for `mut` variables**: after a `mut` variable is reassigned, all assumptions involving
that variable are removed from Γ:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
mut x = x - 5       // Γ = {}  ← x > 0 is killed
```

This is a hard soundness requirement—variable value changes, old assumptions are invalid.

**Branch confluence**: when IF/ELSE or match branches merge, Γ takes the intersection of each
branch's assumptions. Only propositions that hold on all paths are carried out of the branch.

### 13. Erasure Model Clarification: Witness Erasure ≠ Check Erasure

The "refined types are **completely erased** at runtime" claim in RFC-027 refers to **proof
witnesses**—proof terms already verified at compile time do not generate runtime code. But the
**runtime check** inserted by dispatch in Runtime mode is preserved—it is a Bool check executed at
the value level, not a witness at the type level.

Summary: witnesses are erased, checks are preserved. The two do not conflict, and the original
RFC-027 claim remains unchanged.

## Detailed Design

### Syntax Changes

| Before (RFC-022)                      | After (this RFC)                                                                                                                                     |
| ------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| `//! requires: NonEmpty(n) = n > 0`   | Compile-time predicate as parameter type `(b: Positive(b))`                                                                                          |
| `//! ensures: ExistsMax(result, arr)` | Return type uses return value parameter `-> (result: IsMax(T, arr, result))`                                                                         |
| `/*! invariant: ... !*/`              | Compile-time predicate type annotation on variables—Floyd-Hoare invariant                                                                            |
| `//! decreases: n`                    | Metric written at the refined type position (`Terminates`); compiler first explores automatically, only requires explicit input if exploration fails |
| Specification is annotation           | Specification is the type system                                                                                                                     |

### Syntax

**Compile-time predicates introduce no new keywords.** `{}` is the proof space, fully consistent
with existing type definition syntax. A compile-time predicate is a function returning
`Type`—`name: (params) -> Type = { assertion }`. The usage is just a function call—`Positive(b)`,
`IsMax(T, arr, result)`.

```bnf
# Compile-time predicate = function returning Type, {} contains compiler-verified assertions
# Uses existing function/type syntax, no new BNF rules needed
predicate ::= identifier ':' params '->' 'Type' '=' '{' assertions '}'
```

**Predicate application arguments must be compile-time-constant-shaped**—literals, variables (bound
by name), type applications (recursively extracted), or **compile-time-denotable function
references** (function names). Arguments whose shape cannot be converted to a constant expression
report **E1092**, mismatches between the number of arguments and the predicate's formal parameter
declarations report **E1093**. Arguments are bound positionally to the formal parameter list, with
the predicate's arity determined by the declaration—`Positive(x)` is unary, `IsMax(T, arr, result)`
is ternary, `Terminates(m)` and `Terminates(FnType, m)` are unary and binary—**refinement
constraints are never silently discarded** (previously, non-convertible arguments would cause
constraints to silently disappear, allowing bindings that violate the constraints to pass silently).

**New syntax concept: return value parameter**—in `-> (name: Type)`, `name` is the return value
parameter.

The return value parameter is the **only syntax concept** introduced by YaoXiang on top of the
existing function syntax. Its semantics:

- `name`'s value is supplied by the `return` statement
- `name` exists only in the type signature, and is referenced by postcondition predicates (e.g.
  `-> (result: IsMax(T, arr, result))`)
- `name` does not enter the function body scope, nor appear at the call site
- The return value parameter is **optional**—when there is no postcondition, the signature is
  exactly the same as an ordinary function (`-> Int`), introducing no extra burden

Reason for introducing it: postconditions need to reference "the value the function is about to
return". Without a return value parameter, the compiler can only let the predicate reference the
return value through special rules (e.g. implicit variable `$result` or `__retval__`). The return
value parameter makes this reference explicit—it is just a formal parameter, with the value supplied
by `return` rather than the caller.

**Proof function** is not a new concept—it is a YaoXiang function whose return type is the asserted
proposition. When the compiler returns `Unproven`, the programmer supplies a proof function, and the
type checker validates it in exactly the same way it validates any function's return type. No new
syntax, no new keywords, no new rules.

> **Boundary between the two.** The correctness domain (predicate Unproven) is supplemented by a
> proof in the **body**: write a function whose return type is the proposition to be proved. The
> termination domain (§6.9) is supplemented by a metric at the **type position**: write
> `Terminates(metric)` as the type of a binding or function signature. The former is "write a proof
> for the proposition you cannot prove", the latter is "explicitly declare a metric you cannot
> explore"—same mechanism (both are refined type applications), different landing point. The
> termination domain **does not need** to write a `_proof` function.

### Type System Impact

- **Type universe**: compile-time predicates are at the Type₂ level—functions taking values and
  returning Type, at the same level as type constructors
- **Generics interaction**: compile-time predicates can take generic parameters, e.g.
  `NonEmpty: (T: Type) -> (arr: Array(T)) -> Type`
- **Ownership interaction**: expressions in compile-time predicates obey ownership rules, read-only
- **Type inference**: compile-time predicate arguments participate in HM type inference

### Runtime Representation

Compile-time predicates are handled at runtime **according to the dispatch result**:

- **CompileTime mode** (all free variables known at compile time): after proof passes, the witness
  is completely erased. `Positive: (x: Int) -> Type = { x > 0 }`—parameter `b: Positive(5)`'s
  runtime representation is just `Int`. The refined condition `{ 5 > 0 }` has passed, erased.
- **Runtime mode** (some free variables are runtime): preserves the runtime check—executes a Bool
  check at the value level, injects into the flow-sensitive assumption set Γ. See §11 dispatch
  pipeline and §13 erasure model clarification for details.

Placing a compile-time predicate at the type position (e.g. `f(x: Positive(x))`) does not produce a
wrapper type, nor allocate extra memory. But when `x` comes from runtime input, **a** runtime Bool
check **will be** inserted.

**Interaction constraints with `ref`**: compile-time predicates can only reference values under
immutable borrow or with transferred ownership. Compile-time predicates referencing mutable borrows
cannot guarantee the verification result still holds at runtime—directly compile error for such
usage.

### Compiler Changes

1. **Parser**: compile-time predicates use standard function syntax, no extra parsing rules needed
2. **Compile-time proof pipeline**: unified Proved/Disproved/Unproven return interface, automatic
   strategy selection
3. **SMT accelerator module**: SMT-LIB 2.6 translation layer, default backend Z3, CVC5 as
   alternative
4. **Type checker kernel**: inference rules implementation—structural equivalence, βδι-reduction,
   universal quantifier introduction/elimination. This is the only trust root, both SMT and
   programmer proofs are validated through it
5. **Verification condition generation**: WP/SP calculus + loop invariant proof obligations
6. **Error reporting**: counterexample formatting + unresolved proposition report + source location
   association

### Backward Compatibility

- ✅ Code not using compile-time predicates is completely unchanged
- ✅ Compile-time predicates have zero runtime overhead in CompileTime mode; in Runtime mode only
  necessary Bool checks are preserved
- ⚠️ RFC-022's `//!` syntax is no longer supported—but 022 was never implemented, so no migration
  burden

## Trade-offs

### Advantages

- **Full realization of the Curry-Howard correspondence**: types are propositions, programs are
  proofs, `name: Proposition = Proof`
- **Unity**: compile-time predicates and ordinary functions use exactly the same syntax, no
  conceptual split
- **SMT transparency**: the programmer doesn't need to know SMT exists, the mental model is
  consistent with type checking
- **Progressive adoption**: start with one compile-time predicate, gradually increase coverage
- **Minimal runtime overhead**: zero overhead in CompileTime mode, only necessary Bool checks
  preserved in Runtime mode

### Disadvantages

- **Compile time**: SMT solving increases compile time, but the hard budget limit ensures a
  controllable upper bound
- **Automatic proof boundary**: complex predicates beyond first-order linear arithmetic may require
  the programmer to write proof functions. This is not a language defect—this is the inevitable
  conclusion of the halting problem. The compiler honestly reports `Unproven` rather than falsely
  reporting `True`/`False`
- **Learning curve**: writing effective compile-time predicates and proof functions requires
  understanding the basic intuition of the Curry-Howard correspondence
- **Implementation complexity**: the unification of the compile-time proof pipeline requires careful
  design

### Risk Mitigation

- SMT solver budget hard limits (steps 10,000 / time 100ms / instantiation depth 3), over-budget
  returns `Unproven`
- Dependent type pre-reduction: deterministic value computations are consumed first, SMT only chews
  the non-deterministic part
- `Unproven` is not a dead end: correctness domain writes proof functions (return type is the
  proposition), termination domain gives the metric at the type position (§6.9)—both are validated
  by the type checker
- Incremental validation: only validate changed modules
- Clear error messages + counterexample display + budget consumption report + unresolved
  proposition + suggestion (if the compiler can give one)

## Alternatives

| Alternative                                                        | Why Not                                                                                                                                                                   |
| ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| RFC-022: `//!` annotation-style specification                      | Specification and type are split, violating the Curry-Howard correspondence                                                                                               |
| Separate specification file (e.g. CVL)                             | Specification and code are separated, increasing maintenance cost                                                                                                         |
| Runtime-only assertions                                            | Cannot statically guarantee correctness                                                                                                                                   |
| External proof assistant (e.g. Coq)                                | Disconnected from the compiler, requires a separate proof language and trust boundary. YaoXiang's choice: proof is YaoXiang code, the type checker is the only trust root |
| **This proposal: compile-time predicate as a first-class citizen** | ✅                                                                                                                                                                        |

## Implementation Strategy

### Phase Division

| Phase       | Content                                                                                                                                                                |
| ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Phase 1** | Compiler kernel: structural equivalence + βδι-reduction + universal quantifier introduction/elimination. Support simple arithmetic predicates (`x > 0`, `arr.len > 0`) |
| **Phase 2** | SMT-LIB translation layer + Z3/CVC5 integration. Pipeline returns Proved/Disproved/Unproven. Unproven supports programmer writing proof functions                      |
| **Phase 3** | Loop invariant VC generation + termination checking (four-strategy metric exploration + `Terminates` explicit metric, §6, §6.9)                                        |
| **Phase 4** | Incremental validation + caching + IDE support                                                                                                                         |

### Dependencies

- RFC-010: Unified Type Syntax — compile-time predicates are based on `name: type = value`
- RFC-011: Generics System — compile-time predicates can take generic parameters
- RFC-009: Ownership Model — compile-time predicate expressions obey ownership rules

## Open Issues

- [x] **SMT solver choice**: default Z3 (MIT license, most widely validated). CVC5 as an
      SMT-LIB-compatible alternative, switched via compiler flag. The compiler internally translates
      to the SMT-LIB 2.6 standard format—SMT-LIB is the abstraction layer, no custom general solver
      interface.
- [x] **Specific values for solver budget**: steps 10,000 / time 100ms / quantifier instantiation
      depth 3. Fixed internally in the compiler, no knob. If real use cases (not "user wrote it
      wrong") prove the budget insufficient in actual use, adjust.
- [x] **Quantifier support range**: the language does not limit quantifier order. Compile-time
      predicates accept `Type` arguments—`Type` includes function types—so higher-order quantifiers
      are a natural consequence of the type system, requiring no special syntax. The SMT solver can
      automatically judge first-order quantifiers (forall/exists, with interleaved nesting allowed,
      limited by budget depth 3). Higher-order quantifiers: SMT returns `Unproven`, compiler hints
      "this predicate is outside automatic proof range, please provide a proof function". The
      programmer writes a YaoXiang function whose return type equals the proposition—the type
      checker validates the function. No external export, no AI, no interactive proof mode.
      Everything is YaoXiang code, everything is validated by the type checker.
- [x] **Counterexample formatting**: source variable names used directly as SMT variable names (with
      module prefix to avoid conflicts). When Z3 model returns, look up by variable name. Output
      format: variable name = specific value + source location + predicate definition location. No
      complex mapping layer.
- [x] ~~**Interaction between compile-time predicates and `ref` smart pointers?**~~ → Decided:
      compile-time predicates only allow values under immutable borrow or with transferred
      ownership. Values under mutable borrow cannot appear in compile-time predicates.
- [x] **Extension of `forall` predicate violation count metric to non-adjacent operations?** → No
      extension. The current coverage (adjacent swap, adjacent shift) is complementarily covered by
      Strategy 1 (linear rank function)—quicksort's outer interval shrinking is covered by Strategy
      1, and heapsort is covered by Strategy 1 (array index pattern). For loops where no strategy
      can prove termination, the compiler directly errors—this is hard-safety philosophy, not a
      defect. If in the future there are real scenarios (non-academic constructions) of algorithms
      no strategy can cover, revisit. → **Revisit triggered (2026-09-14, #318)**: non-structural
      recursion (gcd-style non-directly-decreasing, mutual recursion, merge partition) are exactly
      the real scenarios anticipated by this clause—metric exploration's template sequence cannot
      automatically handle them, and they cannot be rewritten into analyzable iteration patterns
      without sacrificing readability. Conclusion: hard-safety philosophy unchanged (incorrectly
      written metrics are still rejected by SMT counterexamples, humans can only fail to prove, not
      prove wrong), but "reject if exploration fails" is relaxed to "programmer can explicitly give
      metric at type position if exploration fails"—see §6.9.
- [x] **Linear rank function enumeration combinatorial explosion**: candidate enumeration capped at
      3 bounded variables. When ≤3, enumerate all linear combinations and SMT-verify one by one.
      When >3, only try single-variable metrics (`v_i`, `u_i - v_i`), and on failure directly report
      compile error—prompting the programmer "loop has >3 bounded variables, compiler cannot
      automatically synthesize multi-variable metrics". This is not an engineering compromise—it is
      forcing the programmer to write simpler loops.

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
│   Draft     │  ← author creates
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Reviewing  │  ← current state: community discussion
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
│ (formal     │    │  (kept in   │
│  design)    │    │   place)    │
└─────────────┘    └─────────────┘
```
