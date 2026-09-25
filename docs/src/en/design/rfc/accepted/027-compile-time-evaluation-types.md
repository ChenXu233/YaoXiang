---
title: 'RFC-027: Compile-Time Predicates and Unified Static Verification'
status: 'Accepted'
author: 'Chen Xu'
created: '2026-06-07'
updated: '2026-09-14'
impl_status: 'in_progress'
impl_detail:
  'Phase 1-2 complete, Phase 3 partially complete, Phase 4 partially complete. The 6 phases of the
  unified assert/Assert scheme are all implemented (#157-#162 closed): Never type, IsTrue bridging,
  flow-sensitive Γ + kill set, type-level recursion, universe layering weak check, dispatch
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
> - [RFC-010: Unified Type Syntax - name: type = value Model](../accepted/010-unified-type-syntax.md)
> - [RFC-011: Generic Type System Design](../accepted/011-generic-type-system.md)
> - [RFC-024: Concurrency Model Based on spawn Blocks](../accepted/024-concurrency-model.md)
>
> **Supersedes**:
> [RFC-022: Hoare Logic Static Verification Support (Specification Comments and Specification Types)](../deprecated/022-hoare-logic-static-verification.md)
> — Deprecated

## Abstract

This RFC proposes introducing **compile-time predicates** as first-class citizens in YaoXiang,
unifying all compile-time static verification into a single **proof pipeline**. Compile-time
predicates are not external specification annotations—they are functions. A function that returns
Type can be used at type positions; the compiler invokes it at compile time and checks the return
value. Types are propositions, compile-time evaluation is proof.

**Core argument**: The only work type checking does at compile time is constructing and validating
proof terms. Type equality, token conflicts, dependent type reduction, compile-time predicate
evaluation, Hoare logic implication—all are different type checks in the compile-time proof
pipeline, sharing the same pipeline. The SMT solver is an accelerator module of the type checker,
not an independent trust boundary. When the compiler returns Unproven, the programmer writes a
YaoXiang function as proof—the type checker validates it in exactly the same way as it validates any
function's return type. Everything is YaoXiang code, everything is verified by the type checker.

## Motivation

### Why Deprecate RFC-022?

RFC-022 designed specifications as `//!` comment form:

```yaoxiang
max: (T: Ord) -> ((arr: Array(T, n)) -> T) = {
    //! requires: NonEmpty(n) = n > 0          ← This is annotation independent of types
    //! ensures: ExistsMax(result, arr[0..n])   ← This is annotation independent of types
}
```

This commits the fundamental error of the Curry-Howard isomorphism: **splitting specifications and
types into two layers**. Annotations are not types. Annotations do not participate in type checking.
Annotations are the mental model of "external tools".

The white paper states clearly:

> "No `//!` annotations. No independent specification language. Everything is within the type
> system."

### Current Problems

- RFC-022's `//!` annotations are external syntax independent of the type system
- Specification types and ordinary types are two systems, causing conceptual redundancy
- The Debug Build verification / Release Build ignores split pattern destroys unity
- SMT solvers are traditionally positioned as external tools—YaoXiang builds them in as an
  accelerator module of the type checker
- Type checking, borrow verification, compile-time predicate checking, and macro expansion each take
  different paths

### The Correct Mental Model

Type checking can be abstracted as a function:

```
verify : Program → Proved | Disproved(Model) | Unproven
```

All compile-time checks—simple type matching, borrow conflict detection, compile-time predicate
verification—are subtasks of this function. They share the same proof pipeline, differing only in
proof term complexity and construction strategy.

When the compiler returns Unproven, the programmer provides a proof function—the function's return
type equals the proposition to be proven. The type checker validates it. This is the same operation
as ordinary type checking.

## Proposal

### 1. `{}` Is the Proof Space: Types Are Assertions, Verification Is Type Checking

YaoXiang's `{}` is the compile-time proof space. Everything inside is an assertion; the compiler
guarantees each is True—either automatically proven, or the programmer provides a proof function.

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
#          Parameter at        Only assertions in {}
#          signature           Compiler verifies x > 0 when called at compile time

List: (T: Type) -> Type = { data: Array(T) }
#      ^^^^^^^^              ^^^^^^^^^^^^^^^
#      Parameter at           Compiler verifies type_of(T) == Type, type_of(data) == Array(T)
#      signature
```

Same pattern: `name: (params) -> Type = { assertions }`. The compiler doesn't distinguish "type
assertions" from "value assertions"—both are evaluation targets in the proof pipeline.

**Loop invariants don't need to be written separately. Type annotations on variables are Floyd-Hoare
invariants.**

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # Annotation references i—tells compiler s's type depends on i
    mut i: UpTo(arr.len) = 0     # At initialization i=0, verify: 0 == sum(arr[0..0]) → True
    while i < arr.len {
        s += arr[i]  # Compiler verifies: s_new == sum(arr[0..i+1])
        i += 1       # i changes → triggers s dependency reverification: s satisfies SumUpTo(arr, i_new)
    }
    return s  # s: SumUpTo(arr, arr.len) = sum(arr[0..arr.len])
}
```

The compiler generates one verification condition for the loop body—inductive hypothesis (type
annotation) → assignment operation → whether the new value satisfies the type annotation. Once the
proof pipeline validates the inductive step holds, all iterations are automatically covered. No need
for `: decreases`, no need for `: Invariant`, no need for induction proof—the compiler decomposes
induction into local VCs for each assignment.

### 2. Pre/Postconditions: Compile-Time Predicates on Parameter and Return Types

Abandon RFC-022's `//! requires`/`//! ensures`. Compile-time predicates are type annotations on
parameters or returns.

**The parameter side is a function call.** Compile-time predicates are functions returning Type, and
the parameter-side usage is calling them—just like `factorial(5)`. The return value side introduces
a new concept: the return value parameter.

```yaoxiang
# Preconditions: explicitly call compile-time predicate in parameter type
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b
#                       ^^^^^^^^^^  b is current formal parameter name, passed to Positive as argument
#                       Compiler extracts actual argument value at call site, substitutes b, verifies Positive(actual)
#                       Ex: divide(10, 2) → verify Positive(2) = { 2 > 0 } → True
#                       Ex: divide(10, 0) → verify Positive(0) = { 0 > 0 } → False → compile error

# Postconditions: return value parameter + compile-time predicate
IsMax: (T: Ord, arr: Array(T), result: T) -> Type = {
    forall j in 0..arr.len: result >= arr[j]
}

NonEmpty: (arr: Array(T)) -> Type = { arr.len > 0 }

max: (T: Ord) -> ((arr: NonEmpty(arr))) -> (result: IsMax(T, arr, result)) = {
#                                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
#                                            result is return value parameter, value provided by return
#                                            Compiler substitutes return value at return point, verifies postcondition
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
- **Return side**: `-> (result: IsMax(T, arr, result))` — `result` is the return value parameter,
  value provided by `return` statement. `result` only exists in the type signature, only referenced
  by the predicate, does not enter the function body scope, does not appear at the caller.
- **Return value parameter is optional**: when there is no postcondition, the signature is identical
  to ordinary functions (`-> Int`).
- **Unity**: parameters and return value parameters are the same
  concept—`param_name: predicate_call(param_name)`, differing only in whether the value is provided
  by the caller or by `return`.

### 3. Path Condition Propagation: Compile-Time Verification of Runtime Values

When compile-time predicates are used at binding positions, arguments are explicitly passed by the
programmer. When runtime values enter refined type parameters, the compiler completes verification
through path condition collection and SMT implication judgment—no need for the programmer to
explicitly pass proof.

#### 3.1 Explicit Function Call

When compile-time predicates are used at binding positions, arguments are explicitly passed by the
programmer—just function calls, zero implicit.

`Positive: (x: Int) -> Type = { x > 0 }` is a compile-time predicate constructor. When it appears at
binding positions (parameter declaration, variable declaration, return type), the programmer
explicitly passes the bound variable name:

```yaoxiang
b: Positive(b)
// b is already declared as the current formal parameter, Positive(b) is a function call
// After normalization: b: { b > 0 }
```

No need for the compiler to implicitly fill in arguments—`b: Positive(b)` and `f(5)` are the same,
just function calls. `b` as a parameter name is bound, and its type annotation `Positive(b)`
references `b` itself—this is the standard pattern of dependent types, not an implicit expansion
rule.

**Unity with RFC-010's `self`**: RFC-010 establishes that `self` is not a keyword, just a
conventional parameter name ("writing it as `p`, `this`, `x` is exactly the same effect").
`b: Positive(b)` shares the same mechanism—parameter names can be referenced in type annotations.
`self` appears in `self: Point`, `b` appears in `b: Positive(b)` position, both type annotations
reference the parameter itself. The only difference is the complexity of the type annotation, the
mechanism is exactly the same—after name binding, types can depend on the name.

Return types also use explicit function calls:

```yaoxiang
Sorted: (arr: Array(T)) -> Type = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }

sort: (arr: Array(T)) -> (result: Sorted(result)) = { ... }
//                        ^^^^^^^^^^^^^^^^^^^^^^^
//                        result is return value parameter, Sorted(result) is a function call
//                        Compiler substitutes return value at return point, verifies Sorted(return value)
```

Same applies to local variable declarations:

```yaoxiang
let x: Positive(x) = 5
// x bound to 5, Positive(5) → { 5 > 0 } → True → passes

// let y: Positive(y) = 0
// y bound to 0, Positive(0) → { 0 > 0 } → False → compile error
```

#### 3.2 Path Condition Collection

When runtime values appear in conditional branches, the compiler automatically collects path
conditions, forming the **assumption set** of the current scope. These assumptions participate in
verification as background knowledge for compile-time Bool evaluation.

```yaoxiang
if y > 0 {
    // Compiler automatically obtains assumption in this branch: { y > 0 }
    let result = divide(x, y)
    // Verification condition: (y > 0) ⇒ (y > 0)
    // Proof pipeline judges implication holds → Proved
} else {
// This branch assumes: { !(y > 0) }
// If divide(x, y) is called, verification condition is !(y > 0) ⇒ y > 0
    // Proof pipeline judges implication does not hold → Disproved
}
```

This is not the compiler hard-coding special patterns—this is the natural behavior of the
compile-time proof pipeline. Each type checking call site sends to the pipeline:

```
{background assumptions} ⇒ {verification target}
```

The proof pipeline judges implication. Proved → pass, Disproved → compile error + counterexample,
Unproven → compile error + unsolved proposition. Background assumptions come from the path
conditions at the current program point.

#### 3.3 Assumption Stack

When analyzing control flow, the compiler maintains an assumption set for each basic block:

- **if-guard**: `if y > 0` → true branch pushes `y > 0`, false branch pushes `!(y > 0)` (if else is
  used)
- **match pattern**: `if let Some(v) = opt` → branch pushes `opt == Some(v)`
- **Logical conjunction**: `if x > 0 and y < 10` → branch pushes `x > 0` and `y < 10`
- **Function precondition**: when calling `divide(a, b)`, `b` must satisfy `Positive` the evidence
  either comes from the current assumption, or comes from the actual argument's own refined type
  annotation (`b` is annotated as `Positive` then its type carries `b > 0`)
- **Assignment**: when `let z = y`, the refined conditions already on `y` transfer to `z`

All assumptions enter the compile-time proof pipeline. When entering the SMT accelerator path, they
are translated to SMT-LIB background assertions.

#### 3.4 No Static Evidence Then Compile Error

If the programmer directly writes:

```yaoxiang
divide_user_input: (x: Int, y: Int) -> Int = divide(x, y)
```

There is no `y > 0` assumption at the current program point, and the actual argument `y` itself does
not have a `Positive` type annotation. The verification condition is:

```
{} ⇒ { y > 0 }
```

The pipeline returns `Disproved` (does not imply) → compile error:

> Cannot prove that parameter `b` satisfies `Positive` in the call to `divide`. `y` comes from
> function input, no proven bound. Consider guarding the call with an if branch:
> `if y > 0 { divide(x, y) }`.

YaoXiang does not accept runtime values directly entering refined type parameters without providing
static evidence. This is not a limitation—this is the core of hard safety philosophy. Any code that
the compiler cannot statically prove must not pass compilation.

#### 3.5 Relationship with the Unified Pipeline

Path condition propagation is not an additional mechanism. It is the direct extension of the
compile-time proof pipeline in control flow analysis:

| Phase                             | Responsibility                                                                                                                                           |
| --------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Path Condition Collection         | Compiler control flow analysis phase, annotates assumption set for each basic block                                                                      |
| Verification Condition Generation | When encountering type constraints to verify, merges path conditions + actual argument type information                                                  |
| Proof Pipeline Evaluation         | Compiler core → SMT accelerator → derives Proved / Disproved / Unproven                                                                                  |
| Result                            | `Proved` → pass; `Disproved` → compile error + counterexample; `Unproven` → compile error + unsolved proposition (programmer can provide proof function) |

No new components. No special rules. Path conditions are the background knowledge of the proof
pipeline—sharing the same pipeline and same budget system as type equality and borrow constraints.

### 4. Compile-Time Proof Pipeline

All compile-time checks share the same pipeline. The core operation of the pipeline is **type
checking**—checking whether a proof term's type equals the proposition to be proven. Everything is
type checking.

```
Compile-time encounters Bool expression needing evaluation (i.e.: needs to construct a proof term)
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
         Programmer writes proof function (YaoXiang code)
                  │
                  ▼
         Type checker validates ──→ Proved ──→ Compilation passes
                  │
                  ▼
            Validation failed → Compile error: "Proof does not hold"
```

#### 4.1 Proof Results: Three-Valued Algebra

Compile-time evaluation returns three results—this is the necessary conclusion of the halting
problem and the natural division of proof theory:

```
eval_compile_time : BoolExpr → Proved | Disproved(Model) | Unproven
```

- **Proved** → Halts, proof term constructed, type checking passes. Compilation continues.
- **Disproved(M)** → Halts, counterexample M exists. Compile error + counterexample + source
  location.
- **Unproven** → Within the given resource limit, no proof is constructed. Compile error + unsolved
  proposition + budget consumption report.

**Unproven ≠ False.** The compiler saying "I can't prove it" is not equivalent to the proposition
being false—just that it exceeds current automatic proof capability. This is honesty, not a flaw.

The hard budget limit is the engineering solution to the halting problem. No knob is
provided—providing one would be asking the user "do you think your program will halt", the user
doesn't know, the compiler doesn't know either.

#### 4.2 After Unproven: The Programmer Writes Proof

When the compiler returns Unproven, the programmer can write a **proof function**—a YaoXiang
function whose return type equals the proposition to be proven. The type checker validates this
function—in exactly the same way as it validates `add(a, b): Int`.

```
Proposition = Type
Proof       = Program (a value of that type)
Validation  = Type checking (the sole root of trust)
```

The SMT solver is not an independent trust boundary—it is an **accelerator module of the type
checker**. SMT helps find proofs, but it's the type checker that validates the proof. When SMT
returns `unsat` the compiler reconstructs its result into a proof term verifiable by the type
checker. If reconstruction fails (SMT's reasoning steps exceed the compiler core's inference rules),
it falls back to Unproven—the programmer can manually write a proof function.

```yaoxiang
# Proposition: refined property that the compiler cannot automatically prove
FirstIsMin: (T: Ord, arr: Sorted(T)) -> Type = {
    forall i in 0..arr.len: arr[0] <= arr[i]
}

# Proof: programmer writes a function, return type is the above proposition
# Type checker validates this function—in exactly the same way as validating add(a,b): Int
first_is_min: (T: Ord, arr: Sorted(T)) -> FirstIsMin(T, arr) = {
    # Compiler verifies here: function body's type = FirstIsMin(T, arr)
    ...
}
```

No AI needed, no export to Coq, no new concepts. **Properties that cannot be automatically proven at
compile time → programmer writes proof in YaoXiang code → type checker validates.** The entire
process is a smooth gradient—the compiler handles the simple proofs for you, leaving the hard ones
for your brain.

#### 4.3 Layered Dependencies Within the Pipeline

The above evaluators share the same interface but have an evaluation order. Type equality is the
prerequisite for all subsequent analyses; ownership/token checking depends on type information;
refined predicate verification depends on the results of the first two layers. The compiler
evaluates layer by layer, expressions failing at lower layers do not enter upper layers—avoiding
wasting solver budget on type-incorrect programs.

```
Evaluation order (same pipeline, layered scheduling)
├── Layer 0: Type equality (T1 == T2)
│   └── Structural unification → failure makes subsequent meaningless, directly return Disproved
├── Layer 1: Ownership/token conflicts
│   └── Flow-sensitive liveness analysis → failure makes memory safety not hold, directly return Disproved
└── Layer 2: Refined predicates/Hoare implication
    └── Compiler itself → SMT accelerator → derives Proved / Disproved / Unproven
```

Each layer still returns `Proved/Disproved/Unproven`, sharing the same interface and same budget
system.

### 5. Three-Layer Function Unification

| Layer                  | Execution Time | Input        | Output | Example                                        |
| ---------------------- | -------------- | ------------ | ------ | ---------------------------------------------- |
| Value-level function   | Runtime        | Values       | Value  | `add: (a: Int, b: Int) -> Int = a + b`         |
| Type constructor       | Compile time   | Types/values | Type   | `List: (T: Type) -> Type = { data: Array(T) }` |
| Compile-time predicate | Compile time   | Values       | Type   | `Positive: (x: Int) -> Type = { x > 0 }`       |

All use the same `name: type = value` syntax. Compile-time predicates and type constructors go
through the same compile-time proof pipeline—`{}` is the proof space.

### 6. Loops: Floyd-Hoare Verification Condition Generation

Loops do not need separate `: Invariant(...)` or `: decreases(...)) annotations. Compile-time
predicate type annotations on variables define Floyd-Hoare style assertions—the compiler generates
verification conditions from type annotations, and the proof pipeline checks whether each assignment
maintains the type.

Core mechanism: each assignment operation corresponds to a Hoare triple `{P} x := e {Q}`, with
verification condition `P ⇒ Q[e/x]`. The compiler generates one verification condition for the loop
body—once the proof pipeline validates the inductive step holds, all iterations are automatically
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
        #     From inductive hypothesis s_old == sum(arr[0..i]), add arr[i] to both sides:
        #     sum(arr[0..i]) + arr[i] == sum(arr[0..i+1])
        #   Compiler + SMT: linear arithmetic, millisecond level → Proved
        #
        # i += 1:
        #   i changes → s's type annotation in dependency graph references i → triggers reverification
        #   New verification target: s satisfies SumUpTo(arr, i_new)
        #   That is s == sum(arr[0..i_new]), guaranteed by previous step → Proved
        s += arr[i]
        i += 1
    }
    return s  # At this point s: SumUpTo(arr, arr.len), i.e., s == sum(arr[0..arr.len])
}
```

Loop invariants are the type annotations on variables—programmer writes the type, compiler checks
the inductive step. The compiler does not need to "discover" invariants, nor does it need to
"automatically do induction"—it decomposes induction proofs into local verification conditions for
each assignment operation, handing them to the proof pipeline to divide and conquer.

#### 6.1 Dependency Tracking: Dependent Types on Mutable Variables

The prerequisite for the above mechanism is: the compiler knows that `s`'s type annotation
`SumUpTo(arr, i)` references `i`—when `i` changes, `s`'s type constraint also changes. This requires
the compiler to maintain a **type dependency graph between variables**.

**Data structure**:

```
TypeDepGraph: Map<VarName, Set<VarName>>
# Key is the depended-on variable, value is the set of variables that reference that variable in their type annotation
# Ex: { i: {s}, j: {s, t}, ... }
```

**Construction**: when the type checker processes `mut v: Pred(... x ...) = init`, it resolves the
free variable references in `Pred(...)` arguments. If the argument references another mutable
variable `x` in the current scope, it records `x → v` in the dependency graph.

**Triggering**: when the depended-on variable `x` is assigned, the compiler:

1. Look up all variables `{v₁, v₂, ...}` in the dependency graph that depend on `x`
2. For each `v`, generate a verification condition: does `v`'s current value satisfy the updated
   type `Pred(... x_new ...)`
3. Send the VC into the proof pipeline

**Assignment order sensitivity**: dependency tracking naturally enforces correct assignment order.
Taking `SumUpTo(arr, i)` as an example:

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
`{ v: Int; v == x + y }` depends on both `x` and `y`—any change triggers reverification.

**Relationship with the proof pipeline**: dependency tracking is the trigger for VC generation, not
an independent verification mechanism. It answers "when do we need to generate VCs"—the proof
pipeline answers "whether the VC holds".

### 7. Termination Checking

**Scope of application: refined types.** Termination is not an independent switch, but part of the
**verification mode**: once a type is refined (`Refined { base, constraint }`), the computation it
annotates enters verification mode, which requires termination within the mode; ordinary types not
refined do not enter verification mode and generate no termination obligations.

This yields two corollaries:

- **Loops**: bare `while` does not enter verification mode; measure variables with refined
  annotations (such as `i: UpTo(n)`) enter verification mode, termination must be proven.
- **Recursion**: functions with refined signatures (refined parameters or return types containing
  refinements) enter verification mode, every recursive call site must prove strict decrease of the
  measure.

Automatic-first within the mode: the compiler first automatically explores the measure, proven cases
pass; cases that cannot be explored and have no explicit measure result in compile error. No
**annotation syntax** loophole—the measure and termination proposition are both written in the type
position, no new syntax like `decreases` is introduced. The form of the measure and explicit
fallback see §6.9.

#### 6.1 Design Principles

The compiler automatically extracts information needed for termination proofs from two sources:

1. **Variable type annotations**: boundary constraints in refined types (such as `UpTo(n)` giving
   upper bound `n` and lower bound `0`)
2. **Loop body operations**: operations applied to variables on each iteration

The compiler tries four measure synthesis strategies in priority order, stopping once one is found.
The four strategies are **a restricted template sequence for measure exploration**, with the input
being refinement constraints (Strategies 1-4 all start with "variables with bounded types"), not
"code for compile-time evaluation"; they are the automatic and manual two sides of the same thing as
the explicit measure in §6.9.

> **Measure exploration is exploration, not inference.** Exploration only enumerates templates
> (linear rank, violation count, bounded pattern, multiplicative scaling), and does not guarantee a
> solution exists—general measure inference is in general undecidable (reduces to the halting
> problem). Therefore cases outside the templates must be solvable by the programmer explicitly
> providing a measure (§6.9), otherwise compile error.

#### 6.2 Strategy 1: Automatic Synthesis of Linear Rank Functions

When a variable has a linear bound annotation, the compiler enumerates candidate linear measures and
verifies with SMT.

```
Input:
  Variables v₁: UpTo(u₁), v₂: UpTo(u₂), ... (variables with upper and lower bounds)
  Loop condition cond
  Set of assignments in loop body

Algorithm:
  1. Extract each variable's bounds from type annotations: [low_i, high_i]
  2. Enumerate candidate measures: v_i, u_i - v_i, v_i - v_j, etc. linear combinations
  3. For each candidate measure m:
     - SMT verify m ≥ 0 (derived from type bounds)
     - For each execution path in loop body, SMT verify m' < m (strict decrease)
  4. Find a qualifying linear combination → termination proven
```

Coverage scope: loops where any variable is assigned a linear expression (`v = a·v + b`) and has a
bounded type annotation. Including `i += const`, `i -= const`, and binary-search style interval
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

#### 6.3 Strategy 2: Predicate Violation Count—Automatically Extracting Measure from Target Type <span style="color:orange">【Experimental Strategy】</span>

> ⚠️ **Current status: experimental strategy; whether to include it is decided based on actual
> feasibility during Phase 3 implementation.** This strategy is effective for adjacent swap
> operations (bubble sort, insertion sort), and cannot automatically prove non-adjacent operations
> (quicksort partition, heapsort sift-down). Coverage boundaries see the table below. If Phase 3
> verification is infeasible, this strategy will be removed or downgraded to future work.

Core insight: **the specification the user writes is the raw material for the compiler's
reasoning.** The compiler does not need to built-in "what is sorting"—it reads the definition of
`Sorted`, and automatically extracts the measure from the definition.

```
Input:
  Target type: Sorted(arr) = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }
  Loop body operation: adjacent element swap

Algorithm:
  1. Parse predicate definition: forall i in range: cond(i, arr)
  2. Automatically generate measure: violation_count = |{ i | ¬cond(i, arr) }|
  3. Analyze operation's effect on measure:
     - Adjacent swap arr[j], arr[j+1] = arr[j+1], arr[j]
     - Only affects index pairs j-1, j, j+1
     - If arr[j] > arr[j+1] (violates predicate), after swap this pair satisfies predicate
     - violation_count decreases by at least 1
  4. Upper bound: n·(n-1)/2 (maximum adjacent inversions), lower bound: 0
  → Termination proven
```

**Current coverage**:

| Algorithm      | Operation Mode    | Can Strategy 2 Prove? | Reason                                        |
| -------------- | ----------------- | :-------------------: | --------------------------------------------- |
| Bubble sort    | Adjacent swap     |          ✅           | violation_count strictly decreases per swap   |
| Insertion sort | Adjacent move     |          ✅           | Each shift eliminates one violating pair      |
| Selection sort | Non-adjacent swap |          ❌           | Single swap may increase violation_count      |
| Quicksort      | partition         |          ❌           | Non-adjacent swap, no monotonic decrease      |
| Heapsort       | sift-down         |          ❌           | Tree operation, violation_count non-monotonic |

**Complementary strategies**: for quicksort, the `low < high` interval contraction can be covered by
Strategy 1 (linear rank function)—the outer partition recursion halves the interval each time.
Strategy 1 and Strategy 2 complementarily cover; the termination of most practical algorithms can be
proven by one of them. However, the generalization of Strategy 2 (non-adjacent operations, tree
operations) remains an open question.

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

`v += const` (positive constant), variable has upper bound type annotation → measure
`upper_bound - v` decreases by `const` each time, lower bound 0. This is a degenerate case of
Strategy 1, processed first by the compiler.

#### 6.5 Strategy 4: Multiplicative Scaling Measure Template

`v *= const` (const > 1), variable has upper and lower bound type annotations. The compiler has
built-in logarithmic measure template `ceil(log_const(upper/v))`, measure decreases by 1 per
multiplication by const.

```yaoxiang
mut i: Positive(i) = 1
while i < n {
    # Compiler automatically derives: measure ceil(log₂(n/i)), measure decreases by 1 per multiplication by 2
    i *= 2
}
```

#### 6.6 Separation of Termination and Correctness

Termination proof and correctness proof are independent:

- **Termination**: the above four strategies automatically prove that the loop terminates in a
  finite number of steps; when exploration fails, the programmer provides the measure in the type
  position (§6.9)
- **Correctness**: whether the loop body progresses toward the target type, checked by the
  compile-time proof pipeline through verification conditions

Both pass → compilation passes. Termination proven but correctness fails → compile error +
counterexample. Correctness proven but termination cannot be proven → compile error pointing out the
unanalyzable variable or operation. Both fail → compile error reports the two failure reasons
separately.

#### 6.7 Termination Checking for Recursive Functions

For recursive functions with refined signatures, the compiler checks that the formal parameters
decrease at each recursive call site:

```yaoxiang
gcd: (a: Int, b: NonNegative(b)) -> Int = {
    if b == 0 { return a }
    return gcd(b, a % b)  // Compiler explores: well-order decrease of (b, a % b) → terminates
}
```

Formal parameter decrease is the strongest automatic path (structural recursion). When exploration
fails, the programmer provides the measure in the type position (§6.9).

#### 6.8 Hard Boundary

`i = f(i)` where `f` is irreversible, not closed, does not preserve any monotonicity—mathematically
impossible to automatically prove termination. Compile error:

> This loop cannot automatically prove termination. The loop variable depends on the unanalyzable
> function `f`. Please use an iteration pattern that can be analyzed by the compiler, or bind a name
> to this loop and provide a measure in the type position (§6.9).

This is not a compiler failure. Any code that cannot be statically proven safe must not pass
compilation. Even with an explicit measure, it must be judged true by SMT to pass—measures written
incorrectly are rejected by counterexamples; humans can only fail to prove, not prove wrong.

#### 6.9 Explicit Measure: `Terminates`

When automatic exploration fails to find a measure, the programmer writes the measure in the **type
position**—the same mechanism as `Positive(b)`, `IsMax(T, arr, result)` (zero new syntax):

```yaoxiang
// Measure: ordinary function, can be unit-tested, reusable, not involved at runtime
gcd_measure: (a: Int, b: Int) -> Int = { b }

// Termination component lives on the function's own type position
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// Loop: binding name is the anchor, measure gets the scoped quantity by name
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n {
        i = i + 1
    }
    return acc
}
```

The rationale for loops: `Terminates(m)` refines exactly the value type of the loop body's tail
expression, with the anchor provided by the binding name `acc`—loops can thus be referred to, no
more "anonymous constructions without a referent" dead ends.

**`Terminates` is a built-in predicate**, belonging to the core primitives along with `Int` and
`Never`. It is the only predicate **whose function body is written by the compiler**—its assertion
("each recursive call site/loop back edge's measure strictly decreases") lives in the computational
structure; user-written predicates cannot reference function bodies or loop bodies, so they cannot
be expressed using the predicate definition syntax in the Syntax section. The built-in side
converges to this single name.

**Arity**. `Terminates(FnType, m)` and `Terminates(m)` are two arities of the same predicate, not
two kinds of construction:

| Form                    | Anchor                 | Use                                                                 |
| ----------------------- | ---------------------- | ------------------------------------------------------------------- |
| `Terminates(m)`         | Name of the binding    | Self-recursive functions, loops—default form                        |
| `Terminates(FnType, m)` | Explicit function type | Mutual recursion and other scenarios where the anchor is not unique |

Both are essentially the same: the termination obligation always falls on "the computation annotated
by the type position where the refinement sits".

**Measures do not restrict return type.** A measure can be an expression of any type (not forced to
be a natural number); the "strict decrease" on it is given by the well-order available on that type.
Whether the measure is well-founded (such as whether it is `>= 0` when returning `Int`) is an
**independent obligation**, also given to refinement derivation or SMT just like the decrease
obligation; when both cannot be derived, the diagnosis **does not directly reject**, but suggests
the direction to check (whether the lower bound of the measure holds, whether the recursive argument
is really moving in that direction).

**Relationship with automatic exploration**: the explicit measure is not another pipeline, but the
input after exploration fails. After providing the measure, it still goes through the same SMT to
verify decrease and well-foundedness; if it does not hold, an error is reported with a
counterexample.

For the implementation mechanisms of obligation generation, judgment pipeline, diagnosis direction,
and measure sharing for mutual recursion (SCC), see
[RFC-027a: Explicit Measures for Termination Checking](../review/027a-termination-explicit-measure.md).

### 8. SMT Solver: Accelerator Module of the Type Checker

In traditional languages, SMT solvers are external tools (such as F\* calling Z3, Dafny calling Z3).
In YaoXiang, it is **an accelerator module of the type checker**—only invoked when the compiler core
itself cannot directly judge. SMT helps find proofs, but it's the type checker that validates the
proof.

**Trust model**: the type checker is the sole root of trust. The SMT solver is an accelerator
module—it helps find proofs, but SMT is not an independent trust boundary. The compiler trusts Z3's
`unsat` result (consistent with the F\*/Dafny route—Z3's error rate is lower than the compiler's own
bug rate, a pragmatic engineering choice). The real unreliability is controlled at the SMT
translation layer—if the translation has bugs, the compiler will be exposed in other tests.

**Interface**: the compiler internally translates to the SMT-LIB 2.6 standard format, rather than
binding to a specific solver API. SMT-LIB is an ISO standard; Z3, CVC5, MathSAT, and Yices all
natively support it.

**Default backend**: Z3 (MIT license, most extensive documentation and community validation). CVC5
as SMT-LIB compatible fallback—users can switch via compiler flag at compile time.

No "general-purpose solver abstraction layer"—SMT-LIB is the abstraction layer. In the future, if
CVC5 has breakthroughs in specific theories, switching only requires changing the binary, no need to
modify compiler code.

```
Compile-time Bool expression
        │
        ├── Compiler core can directly judge (structural equivalence, simple arithmetic,
        │   trivial formula after constant folding)
        │   → Directly return Proved / Disproved
        │
        └── Compiler core cannot directly judge (quantifiers, symbolic variables)
            → Dependent type pre-reduction (factorial(5) → 120)
            → Translate to SMT-LIB format
            → Send to Z3/CVC5 (with budget limit)
            → Return value: unsat → Proved  │  sat + model → Disproved  │  unknown → Unproven
```

**Solver budget—hard limit, like stack depth**:

| Budget Dimension               | Default | Description                                                                                                                                                        |
| ------------------------------ | ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Solver steps                   | 10,000  | Z3 typically takes less than a hundred steps for linear arithmetic. 10,000 steps cover 99% of practical predicates.                                                |
| Time                           | 100ms   | A single predicate exceeding 100ms = the user is writing a compile-time program, not a type annotation. 100ms × 50 predicates = 5 second compile time upper limit. |
| Quantifier instantiation depth | 3       | Three levels of nested quantifiers cover practical patterns. Exceeding three levels, you're likely writing a logic exercise.                                       |

Exceeding budget returns Unproven, compile error + predicate location + consumption. No degradation,
no runtime check, no silent pass.

**Why this is actually feasible**: in practice, 95% of practical predicates are linear
arithmetic—`x > 0`, `arr.len > 0`, `0 <= idx < arr.len`—all within the decidable fragment, and SMT
solvers return in milliseconds for these problems. When encountering the rare complex predicates
exceeding the budget, the programmer writes a proof function.

Dependent types do a layer of pre-reduction before the SMT call: `factorial(5)` is directly
evaluated at compile time to `120`, and `append([1,2], [3])` is directly evaluated to `[1,2,3]`.
These deterministic value computations do not consume SMT budget.

Programmers don't need to know that SMT exists. The mental model is: **the compiler can prove it, it
passes; if it can't, it errors—if the compiler can't, you can write a function to prove it**.

### 9. Compile-Time Predicate Composition

Compile-time predicates are functions returning Type, and composition is naturally achieved through
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

### 11. Dispatch Pipeline: Unified Dispatch of Compile-Time and Runtime

`assert` and `Assert` are two sides of the same refined type primitive. The dispatch pipeline
`dispatch` automatically decides whether to go through compile-time proof or runtime check based on
**whether the predicate's free variables are reachable at compile time**:

| Criterion                                                                                  | Mode            | Behavior                                                                                     |
| ------------------------------------------------------------------------------------------ | --------------- | -------------------------------------------------------------------------------------------- |
| All free variables known at compile time (generic parameters, compile-time constants)      | **CompileTime** | Enters proof pipeline: Proved → erased, Disproved → compile error, Unknown → requires proof  |
| Some free variables come from runtime (function parameters, external input, mut variables) | **Runtime**     | Inserts runtime check, and injects refinement facts into the flow-sensitive assumption set Γ |

**Key**: "can't determine" ≠ "falsified". In CompileTime mode, Unknown requires proof (no silent
degradation); in Runtime mode, the proposition has no truth value at compile time at all—no matter
how strong the prover is, it cannot write a universally true proof for "the user may have input a
negative number"; the runtime check is the only sound choice. This is not because the prover is not
strong enough; it is a theoretical necessity.

### 12. Flow-Sensitive Assumption Set Γ: Strongest Postcondition Propagation

The compiler maintains a flow-sensitive assumption set Γ, tracking the propositions known to hold at
each control flow point.

**SP (Strongest Postcondition) propagation**:

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

This is a hard requirement of soundness—the variable value changed, old assumptions are invalid.

**Branch confluence**: when IF/ELSE or match branches merge, Γ takes the intersection of each
branch's assumptions. Only propositions holding in all paths are carried out of the branch.

### 13. Erasure Model Clarification: Witness Erasure ≠ Check Erasure

The claim in RFC-027 that "refined types are **completely erased** at runtime" refers to **proof
witnesses**—proof terms verified at compile time do not generate runtime code. But the **runtime
checks** inserted by dispatch in Runtime mode are preserved—they are Bool checks executed at the
value level, not witnesses at the type level.

Summary: witnesses are erased, checks are preserved. The two things do not conflict; the original
claim in RFC-027 remains unchanged.

## Detailed Design

### Syntax Changes

| Before (RFC-022)                      | After (This RFC)                                                                                                                             |
| ------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `//! requires: NonEmpty(n) = n > 0`   | Compile-time predicate as parameter type `(b: Positive(b))`                                                                                  |
| `//! ensures: ExistsMax(result, arr)` | Return type uses return value parameter `-> (result: IsMax(T, arr, result))`                                                                 |
| `/*! invariant: ... !*/`              | Compile-time predicate type annotation on variable—Floyd-Hoare invariant                                                                     |
| `//! decreases: n`                    | Measure written in refined type position (`Terminates`); compiler first auto-explores, only requires explicit measure when exploration fails |
| Specifications are comments           | Specifications are the type system                                                                                                           |

### Syntax

**No new keyword for compile-time predicates.** `{}` is the proof space, completely consistent with
existing type definition syntax. Compile-time predicates are functions returning
Type—`name: (params) -> Type = { assertions }`. Usage is function calls—`Positive(b)`,
`IsMax(T, arr, result)`.

```bnf
# Compile-time predicate = function returning Type, with compiler-verified assertions in {}
# Uses existing function/type syntax, no new BNF rules needed
predicate ::= identifier ':' params '->' 'Type' '=' '{' assertions '}'
```

**Predicate application arguments must be in compile-time constant form**—literals, variables (bound
by name), type applications (recursively extracted), or **compile-time referable function
references** (function names). Arguments that cannot be converted to constant expressions report
**E1092**, and argument count mismatching the predicate's declared parameter count report **E1093**.
Arguments are bound positionally to the parameter list, predicate arity is determined by
declaration—`Positive(x)` unary, `IsMax(T, arr, result)` ternary, `Terminates(m)` and
`Terminates(FnType, m)` unary and binary—refinement constraints **are never silently dropped**
(previously unconvertible arguments would cause constraints to disappear silently, and bindings
violating constraints would pass silently).

**New syntax concept: return value parameter**—in `-> (name: Type)`, `name` is the return value
parameter.

The return value parameter is the **only one syntax concept** that YaoXiang introduces on top of
existing function syntax. Its semantics:

- `name`'s value is provided by the `return` statement
- `name` only exists in the type signature, referenced by postcondition predicates (such as
  `-> (result: IsMax(T, arr, result))`)
- `name` does not enter the function body scope, does not appear at the caller
- Return value parameter is **optional**—when there is no postcondition, the signature is identical
  to ordinary functions (`-> Int`), introducing no extra burden

The reason for introducing it: postconditions need to reference "the value the function will
return". Without the return value parameter, the compiler can only let the predicate reference the
return value through special rules (such as implicit variables `$result` or `__retval__`). The
return value parameter makes this reference explicit—it is just a formal parameter, only the value
is provided by `return` rather than the caller.

**Proof functions** are not a new concept—it is a YaoXiang function whose return type is the
asserted proposition. When the compiler returns Unproven, the programmer provides a proof function,
and the type checker validates it in exactly the same way as it validates any function's return
type. No new syntax, no new keywords, no new rules.

> **The boundary between the two.** In the correctness domain (predicate Unproven), supplement proof
> in the **body**: write a function whose return type is the proposition to be proven. In the
> termination domain (§6.9), supplement the measure in the **type position**: write
> `Terminates(measure)` as the type of the binding or function signature. The former is "write a
> proof for the proposition that cannot be proven", the latter is "explicitly declare a measure that
> cannot be explored"—same mechanism (both are refined type applications), different placement. The
> termination domain **does not** require writing a `_proof` function.

### Type System Impact

- **Type universe**: compile-time predicates are at the Type₂ layer—functions accepting values and
  returning Type, at the same level as type constructors
- **Generics interaction**: compile-time predicates can take generic parameters, such as
  `NonEmpty: (T: Type) -> (arr: Array(T)) -> Type`
- **Ownership interaction**: expressions in compile-time predicates obey ownership rules, can only
  read not write
- **Type inference**: compile-time predicate arguments participate in HM type inference

### Runtime Representation

Compile-time predicates are **handled at runtime according to the dispatch pipeline result**:

- **CompileTime mode** (all free variables known at compile time): after proof, the witness is
  completely erased. `Positive: (x: Int) -> Type = { x > 0 }`—the parameter `b: Positive(5)` at
  runtime is represented as `Int`. The refinement condition `{ 5 > 0 }` has passed, erased.
- **Runtime mode** (some free variables come from runtime): preserve the runtime check—execute the
  Bool check at the value level, inject into the flow-sensitive assumption set Γ. See §11 for the
  dispatch pipeline and §13 for erasure model clarification.

Placing compile-time predicates in type positions (such as `f(x: Positive(x))`) does not produce
wrapper types, does not allocate extra memory. But when `x` comes from runtime input, a **runtime
Bool check** is inserted.

**Interaction constraint with `ref`**: Compile-time predicates can only reference values with
immutable borrows or transferred ownership. Compile-time predicates referencing mutably borrowed
values cannot guarantee at compile time that the verification result still holds at runtime—such
usage directly reports a compile error.

### Compiler Changes

1. **Parser**: compile-time predicates use standard function syntax, no additional parsing rules
2. **Compile-time proof pipeline**: unified Proved/Disproved/Unproven return interface, automatic
   strategy selection
3. **SMT accelerator module**: SMT-LIB 2.6 translation layer, default backend Z3, CVC5 fallback
4. **Type checker core**: inference rule implementation—structural equivalence, βδι-reduction,
   universal quantifier introduction/elimination. This is the sole root of trust; both SMT and
   programmer proofs are validated through this
5. **Verification condition generation**: WP/SP calculus + loop invariant proof obligations
6. **Error reporting**: counterexample formatting + unsolved proposition reports + source location
   association

### Backward Compatibility

- ✅ Code not using compile-time predicates remains completely unchanged
- ✅ Compile-time predicates have zero runtime overhead in CompileTime mode; only necessary Bool
  checks remain in Runtime mode
- ⚠️ RFC-022's `//!` syntax is no longer supported—but 022 was never implemented, so there is no
  migration burden

## Trade-offs

### Advantages

- **Curry-Howard isomorphism fully realized**: types are propositions, programs are proofs,
  `name: Proposition = Proof`
- **Unity**: compile-time predicates use exactly the same syntax as ordinary functions, no
  conceptual split
- **SMT transparency**: programmers don't need to know that SMT exists; the mental model is
  consistent with type checking
- **Incremental adoption**: can start from one compile-time predicate and gradually increase
  coverage
- **Minimal runtime overhead**: zero overhead in CompileTime mode, only necessary Bool checks in
  Runtime mode

### Disadvantages

- **Compile time**: SMT solving increases compile time, but the hard budget limit ensures the upper
  bound is controllable
- **Automatic proof boundary**: complex predicates beyond first-order linear arithmetic may require
  the programmer to write a proof function. This is not a language defect—this is the necessary
  conclusion of the halting problem. The compiler honestly reports Unproven rather than falsely
  reporting True/False
- **Learning curve**: writing effective compile-time predicates and proof functions requires
  understanding the basic intuition of the Curry-Howard isomorphism
- **Implementation complexity**: the unification of the compile-time proof pipeline requires careful
  design

### Risk Mitigation

- SMT solver budget hard limits (10,000 steps / 100ms time / 3 instantiation depth), exceeding
  budget returns Unproven
- Dependent type pre-reduction: deterministic value computation is handled first, SMT only deals
  with the non-deterministic part
- Unproven is not a dead end: in the correctness domain, write a proof function (return type is the
  proposition); in the termination domain, provide a measure in the type position (§6.9)—both are
  verified by the type checker
- Incremental verification: only verify changed modules
- Clear error messages + counterexample display + budget consumption report + unsolved proposition +
  suggestions (if the compiler can give them)

## Alternatives

| Approach                                                           | Why Not Chosen                                                                                                                                                                   |
| ------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| RFC-022: `//!` comment-style specification                         | Specification and type split, violates Curry-Howard isomorphism                                                                                                                  |
| Independent specification files (e.g., CVL)                        | Specification and code separated, increasing maintenance cost                                                                                                                    |
| Runtime-only assertions                                            | Cannot statically guarantee correctness                                                                                                                                          |
| External proof assistant (e.g., Coq)                               | Disconnected from the compiler, requires an independent proof language and trust boundary. YaoXiang's choice: proof is YaoXiang code, the type checker is the sole root of trust |
| **This proposal: compile-time predicates as first-class citizens** | ✅                                                                                                                                                                               |

## Implementation Strategy

### Phase Division

| Phase       | Content                                                                                                                                                               |
| ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Phase 1** | Compiler core: structural equivalence + βδι-reduction + universal quantifier introduction/elimination. Supports simple arithmetic predicates (`x > 0`, `arr.len > 0`) |
| **Phase 2** | SMT-LIB translation layer + Z3/CVC5 integration. Pipeline returns Proved/Disproved/Unproven. Supports programmer writing proof functions on Unproven                  |
| **Phase 3** | Loop invariant VC generation + termination check (four measure exploration strategies + `Terminates` explicit measure, §6, §6.9)                                      |
| **Phase 4** | Incremental verification + caching + IDE support                                                                                                                      |

### Dependencies

- RFC-010: Unified Type Syntax — compile-time predicates are based on `name: type = value`
- RFC-011: Generic Type System — compile-time predicates can take generic parameters
- RFC-009: Ownership Model — compile-time predicate expressions obey ownership rules

## Open Questions

- [x] **Z3 availability for wasm32 targets**: Z3 is unavailable on wasm (all SMT code is excluded
      via `cfg(not(target_arch = "wasm32"))`). Scope of impact and degradation direction: -
      `ownership.rs` `smt_cut` always returns `false` → edge crossing → **conservative rejection**
      (sound direction, just narrows less) - Termination check Strategy 1, predicate levels 2b/3
      skipped overall Currently **unreachable**: `TypeEnvironment::predicate_defs` is never
      populated in production (only in tests), `parser` does not produce `MonoType::Refined`, so the
      refined predicate path does not exist in production; the injection point for termination
      Strategy 1 (`TerminationChecker::with_solver`) is never called in production, and native also
      does not execute. Therefore the **actual behavioral difference between wasm and native is only
      in the precision of `smt_cut`**, which does not affect soundness. For subsequent plans to
      enable Z3 on wasm, the solution and cost are in issue #376 (conclusion: prioritize the JS-side
      Z3 instance rather than linking Z3 into the main wasm—the latter requires switching to the
      emcc build system and the artifact grows from 3 MB to ~20 MB).
- [x] **SMT solver selection**: default Z3 (MIT license, most widely validated). CVC5 as SMT-LIB
      compatible fallback, switched via compiler flag. The compiler internal translation target is
      SMT-LIB 2.6 standard format—SMT-LIB is the abstraction layer, no custom general-purpose solver
      interface.
- [x] **Specific values for solver budget**: steps 10,000 / time 100ms / quantifier instantiation
      depth 3. Fixed internally in the compiler, no knob. In actual use, if real use cases prove it
      not enough (not "user wrote it wrong"), then adjust.
- [x] **Quantifier support range**: the language does not limit quantifier order at the language
      level. Compile-time predicates accept Type parameters—Type includes function types—so
      higher-order quantifiers are a natural inference of the type system, requiring no special
      syntax. SMT solvers can automatically determine first-order quantifiers (forall/exists,
      supporting interleaved nesting, limited by budget depth 3). Higher-order quantifiers: SMT
      returns Unproven, compiler prompts "this predicate is outside the automatic proof range,
      please provide a proof function". The programmer writes a YaoXiang function whose return type
      equals the proposition—the type checker validates the function. No external export, no AI, no
      interactive proof mode required. Everything is YaoXiang code, everything is verified by the
      type checker.
- [x] **Counterexample formatting**: source code variable names are directly used as SMT variable
      names (with module prefix to avoid conflicts). Z3 model returns are reverse-queried by
      variable name. Output format: variable name = specific value + source location + predicate
      definition location. No complex mapping layer.
- [x] ~~**Interaction of compile-time predicates with `ref` smart pointers?**~~ → Decided:
      compile-time predicates only allow immutably borrowed values or values whose ownership has
      been transferred. Mutably borrowed values cannot appear in compile-time predicates.
- [x] **Extension of `forall` predicate violation count measure to non-adjacent operations?** → Not
      extended. Current coverage (adjacent swap, adjacent move) is complementarily covered by
      Strategy 1 (linear rank function)—the outer interval contraction of quicksort is covered by
      Strategy 1, and heapsort is covered by Strategy 1 (array index pattern). Loops that cannot be
      proven to terminate by any strategy are directly reported as errors by the compiler—this is
      the hard safety philosophy, not a defect. If in the future there are real-world scenarios (not
      academic constructions) where algorithms cannot be covered by any of the four strategies, then
      re-discuss. → **Re-discussion triggered (2026-09-14, #318)**: non-structural recursion
      (gcd-type non-direct decrease, mutual recursion, merge partition) is exactly the real-world
      scenario expected by this clause—the template sequence of measure exploration cannot
      automatically take them down, and they cannot be rewritten into analyzable iteration patterns
      without breaking readability. Conclusion: the hard safety philosophy remains unchanged
      (measures written incorrectly are still rejected by SMT counterexamples, humans can only fail
      to prove, not prove wrong), but "reject if exploration fails" is relaxed to "if exploration
      fails, the programmer can explicitly provide a measure in the type position"—see §6.9.
- [x] **Linear rank function enumeration combinatorial explosion**: candidate enumeration upper
      limit is 3 bounded variables. When ≤3, enumerate all linear combinations and verify one by one
      with SMT. When >3, only try single-variable measures (`v_i`, `u_i - v_i`), and on failure
      directly report a compile error—prompting the programmer "the loop has >3 bounded variables,
      the compiler cannot automatically synthesize multi-variable measures". This is not an
      engineering compromise—it forces programmers to write simpler loops.

## References

- [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md)
- [RFC-011: Generic Type System Design](../accepted/011-generic-type-system.md)
- [RFC-009: Ownership Model](../accepted/009-ownership-model.md)
- Howard, W. A. (1969). The Formulae-as-Types Notion of Construction.
- Swamy, N. et al. (2016). Dependent Types and Multi-Monadic Effects in F*. _POPL 2016_.
- Vazou, N. et al. (2014). Refinement Types for Haskell. _ICFP 2014_.
- Leino, K. R. M. (2010). Dafny: An Automatic Program Verifier for Functional Correctness. _LPAR
  2010_.
- De Moura, L. & Bjørner, N. (2008). Z3: An Efficient SMT Solver. _TACAS 2008_.

---

## Lifecycle and Destination

```
┌─────────────┐
│   Draft     │  ← Author created
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
│ (Formal     │    │ (Retain in  │
│  Design)    │    │   place)    │
└─────────────┘    └─────────────┘
```
