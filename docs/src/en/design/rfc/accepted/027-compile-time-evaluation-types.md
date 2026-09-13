---
title: 'RFC-027: Compile-Time Predicates and Unified Static Verification'
status: 'Accepted'
author: 'Chen Xu'
created: '2026-06-07'
updated: '2026-09-14'
impl_status: 'in_progress'
impl_detail:
  'Phase 1-2 complete, Phase 3 partially complete, Phase 4 partially complete. All 6 phases of the
  unified assert/Assert scheme implemented (#157-#162 closed): Never type, IsTrue bridge,
  flow-sensitive Γ + kill set, type-level recursion, universe stratification weak check, dispatch
  routing pipeline.'
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
predicates are not external specification comments—they are functions. A function that returns Type
can be used in type positions; the compiler invokes it at compile time and checks the return value.
Types are propositions, compile-time evaluation is proof.

**Core argument**: The only job of type checking at compile time is to construct and verify proof
terms. Type equality, token conflicts, dependent type reduction, compile-time predicate evaluation,
Hoare logic implication—all are different type checks within the compile-time proof pipeline,
sharing the same pipeline. The SMT solver is an acceleration module of the type checker, not an
independent trust boundary. When the compiler returns Unproven, the programmer writes a YaoXiang
function as a proof—the type checker validates it in exactly the same way it validates any
function's return type. Everything is YaoXiang code, everything is verified by the type checker.

## Motivation

### Why Deprecate RFC-022?

RFC-022 designed specifications as `//!` comments:

```yaoxiang
max: (T: Ord) -> ((arr: Array(T, n)) -> T) = {
    //! requires: NonEmpty(n) = n > 0          ← This is a comment independent of types
    //! ensures: ExistsMax(result, arr[0..n])   ← This is a comment independent of types
}
```

This commits a fundamental error of the Curry-Howard correspondence: **splitting specifications and
types into two layers**. Comments are not types. Comments do not participate in type checking.
Comments are the mental model of "external tools."

The white paper states it clearly:

> "No `//!` comments. No separate specification language. Everything is within the type system."

### Current Problems

- RFC-022's `//!` comments are external syntax independent of the type system
- Specification types and ordinary types are two separate systems, creating conceptual redundancy
- The Debug Build verification / Release Build ignoring split pattern breaks unity
- The SMT solver is positioned in traditional cognition as an external tool—YaoXiang builds it in as
  an acceleration module of the type checker
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

When the compiler returns Unproven, the programmer provides a proof function—whose return type
equals the proposition to be proven. The type checker validates it. This is the same operation as
ordinary type checking.

## Proposal

### 1. `{}` is Proof Space: Types are Assertions, Verification is Type Checking

YaoXiang's `{}` is the compile-time proof space. Everything inside is an assertion, and the compiler
guarantees each item is True—either proven automatically or provided by the programmer as a proof
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
#          Params in signature  Only assertions inside {}
#          Compiler verifies x > 0 at call site

List: (T: Type) -> Type = { data: Array(T) }
#      ^^^^^^^^              ^^^^^^^^^^^^^^^
#      Params in signature   Compiler verifies type_of(T) == Type, type_of(data) == Array(T)
```

The same pattern: `name: (params) -> Type = { assertion }`. The compiler does not distinguish "type
assertions" from "value assertions"—both are evaluation targets in the proof pipeline.

**Loop invariants need not be written separately. Type annotations on variables are the Floyd-Hoare
invariants.**

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # Annotation references i—tells the compiler s's type depends on i
    mut i: UpTo(arr.len) = 0     # At init i=0, verify: 0 == sum(arr[0..0]) → True
    while i < arr.len {
        s += arr[i]  # Compiler verifies: s_new == sum(arr[0..i+1])
        i += 1       # i changes → triggers re-verification of s's dependency: s satisfies SumUpTo(arr, i_new)
    }
    return s  # s: SumUpTo(arr, arr.len) = sum(arr[0..arr.len])
}
```

The compiler generates a single verification condition for the loop body—induction hypothesis (type
annotation) → assignment operation → whether the new value satisfies the type annotation. Once the
proof pipeline verifies the inductive step, all iterations are automatically covered. No
`: decreases`, no `: Invariant`, no inductive proof needed—the compiler decomposes induction into
local VCs for each assignment.

### 2. Pre/Postconditions: Compile-Time Predicates on Parameter Types and Return Types

Abandon RFC-022's `//! requires`/`//! ensures`. Compile-time predicates are used as annotations on
parameters or return types.

**The parameter side is a function call.** A compile-time predicate is a function that returns Type,
and using it on the parameter side is just calling it—just like `factorial(5)`. The return value
side introduces a new concept: return value parameters.

```yaoxiang
# Precondition: explicitly call compile-time predicate in parameter type
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b
#                       ^^^^^^^^^^  b is the current parameter name, passed to Positive
#                       Compiler extracts argument value at call site, substitutes b, verifies Positive(arg)
#                       Ex: divide(10, 2) → verify Positive(2) = { 2 > 0 } → True
#                       Ex: divide(10, 0) → verify Positive(0) = { 0 > 0 } → False → compile error

# Postcondition: return value parameter + compile-time predicate
IsMax: (T: Ord, arr: Array(T), result: T) -> Type = {
    forall j in 0..arr.len: result >= arr[j]
}

NonEmpty: (arr: Array(T)) -> Type = { arr.len > 0 }

max: (T: Ord) -> ((arr: NonEmpty(arr))) -> (result: IsMax(T, arr, result)) = {
#                                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
#                                            result is the return value parameter, value provided by return
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
  an argument. Function call syntax, zero implicit.
- **Return side**: `-> (result: IsMax(T, arr, result))` — `result` is the return value parameter,
  with its value provided by the `return` statement. `result` exists only in the type signature, is
  referenced only by predicates, does not enter the function body scope, and does not appear at the
  call site.
- **Return value parameter is optional**: when there is no postcondition, don't write it, and the
  signature is exactly the same as a normal function (`-> Int`).
- **Unity**: Parameter and return value parameters are the same
  concept—`paramName: predicateCall(paramName)`, differing only in whether the value is provided by
  the caller or by `return`.

### 3. Path Condition Propagation: Compile-Time Verification of Runtime Values

When compile-time predicates are used at binding positions, arguments are explicitly passed by the
programmer. When runtime values enter refinement type arguments, the compiler performs verification
through path condition collection and SMT implication—without requiring the programmer to explicitly
pass proofs.

#### 3.1 Explicit Function Calls

When compile-time predicates are used at binding positions, arguments are explicitly passed by the
programmer—just function calls, zero implicit.

`Positive: (x: Int) -> Type = { x > 0 }` is a compile-time predicate constructor. When it appears at
a binding position (parameter declaration, variable declaration, return type), the programmer
explicitly passes a bound variable name:

```yaoxiang
b: Positive(b)
// b has been declared as the current parameter, Positive(b) is a function call
// After strict normalization: b: { b > 0 }
```

No implicit parameter filling by the compiler needed—`b: Positive(b)` is just a function call, same
as `f(5)`. `b` is bound as a parameter name, and its type annotation `Positive(b)` references `b`
itself—this is the standard pattern of dependent types, not an implicit expansion rule.

**Unity with RFC-010's `self`**: RFC-010 establishes that `self` is not a keyword, just a
conventional parameter name ("writing `p`, `this`, `x` is exactly the same effect").
`b: Positive(b)` shares the same mechanism—the parameter name can be referenced in type annotations.
`self` appears in the position of `self: Point`, `b` appears in the position of `b: Positive(b)`,
and both type annotations reference the parameter itself. The difference is only in the complexity
of the type annotation, the mechanism is completely the same—after the name is bound, the type can
depend on this name.

Return types also use explicit function calls:

```yaoxiang
Sorted: (arr: Array(T)) -> Type = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }

sort: (arr: Array(T)) -> (result: Sorted(result)) = { ... }
//                        ^^^^^^^^^^^^^^^^^^^^^^^
//                        result is the return value parameter, Sorted(result) is a function call
//                        Compiler substitutes the return value at the return point, verifies Sorted(return value)
```

Same applies to local variable declarations:

```yaoxiang
let x: Positive(x) = 5
// x bound to 5, Positive(5) → { 5 > 0 } → True → pass

// let y: Positive(y) = 0
// y bound to 0, Positive(0) → { 0 > 0 } → False → compile error
```

#### 3.2 Path Condition Collection

When runtime values appear in conditional branches, the compiler automatically collects path
conditions, forming the **assumption set** for the current scope. These assumptions participate in
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

This is not the compiler hardcoding special patterns—this is the natural behavior of the
compile-time proof pipeline. At each type check call site, the pipeline sends:

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
- **match patterns**: `if let Some(v) = opt` → branch pushes `opt == Some(v)`
- **logical conjunction**: `if x > 0 and y < 10` → branch pushes `x > 0` and `y < 10`
- **function preconditions**: when calling `divide(a, b)`, the evidence that `b` satisfies
  `Positive` either comes from current assumptions, or from the refinement type annotation on the
  argument itself (`b` is already annotated as `Positive`, so its type carries `b > 0`)
- **assignment**: when `let z = y`, the existing refinement condition on `y` is propagated to `z`

All assumptions enter the compile-time proof pipeline. When entering the SMT acceleration path, they
are translated to SMT-LIB background assertions.

#### 3.4 No Static Evidence Means Compile Error

If the programmer writes directly:

```yaoxiang
divide_user_input: (x: Int, y: Int) -> Int = divide(x, y)
```

There is no `y > 0` assumption at the current program point, and the argument `y` itself has no
`Positive` type annotation. The verification condition is:

```
{} ⇒ { y > 0 }
```

Pipeline returns `Disproved` (implication does not hold) → compile error:

> Cannot prove that parameter `b` satisfies `Positive` in the `divide` call. `y` comes from function
> input, with no proven bound. Consider guarding the call with an if branch:
> `if y > 0 { divide(x, y) }`.

YaoXiang does not accept runtime values entering refinement type arguments directly without
providing static evidence. This is not a restriction—this is the core of the hard safety philosophy.
Any code that the compiler cannot statically prove must not pass compilation.

#### 3.5 Relationship with the Unified Pipeline

Path condition propagation is not an additional mechanism. It is the direct extension of the
compile-time proof pipeline on control flow analysis:

| Stage                             | Responsibility                                                                                                                                           |
| --------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Path condition collection         | Compiler control flow analysis phase, annotating each basic block with assumption set                                                                    |
| Verification condition generation | When encountering type constraints to be verified, merge path conditions + argument type information                                                     |
| Proof pipeline evaluation         | Compiler kernel → SMT acceleration → get Proved / Disproved / Unproven                                                                                   |
| Result                            | `Proved` → pass; `Disproved` → compile error + counterexample; `Unproven` → compile error + unsolved proposition (programmer can provide proof function) |

No new components. No special rules. Path conditions are the background knowledge of the proof
pipeline—sharing the same pipeline and the same budget system with type equality and borrow
constraints.

### 4. Compile-Time Proof Pipeline

All compile-time checks share the same pipeline. The core operation of the pipeline is **type
checking**—checking whether a proof term's type equals the proposition to be proven. Everything is
type checking.

```
Compile-time encounters Bool expression needing evaluation (i.e.: needs to construct a proof term)
        │
        ├── Type equality (T1 == T2)
        │   → Compiler direct judgment (structural equivalence)
        │
        ├── Token conflict conditions (!conflicting(tokens))
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
            Verification failed → Compile error: "Proof does not hold"
```

#### 4.1 Proof Results: Three-Valued Algebra

Compile-time evaluation returns three results—this is the inevitable conclusion of the halting
problem, and the natural division of proof theory:

```
eval_compile_time : BoolExpr → Proved | Disproved(Model) | Unproven
```

- **Proved** → Halts, proof term constructed, type check passed. Compilation continues.
- **Disproved(M)** → Halts, counterexample M exists. Compile error + counterexample + source
  location.
- **Unproven** → Within the given resource limit, no proof was constructed. Compile error + unsolved
  proposition + budget consumption report.

**Unproven ≠ False.** The compiler saying "I can't prove it" is not equivalent to the proposition
being false—just beyond current automatic proof capabilities. This is honesty, not a defect.

Hard budget limits are the engineering solution to the halting problem. No knob—giving one would be
asking the user "do you think your program will halt," the user doesn't know, and the compiler
doesn't know either.

#### 4.2 After Unproven: The Programmer Writes the Proof

When the compiler returns Unproven, the programmer can write a **proof function**—just a YaoXiang
function whose return type equals the proposition to be proven. The type checker validates this
function—in exactly the same way it validates `add(a, b): Int`.

```
Proposition = Type
Proof       = Program (a value of that type)
Verification = Type check (the only root of trust)
```

The SMT solver is not an independent trust boundary—it is an **acceleration module of the type
checker**. SMT helps find proofs, but it is always the type checker that verifies proofs. When SMT
returns `unsat`, the compiler reconstructs the result as a proof term verifiable by the type
checker. If reconstruction fails (SMT's reasoning steps exceed the compiler kernel's inference
rules), it falls back to Unproven—the programmer can manually write a proof function.

```yaoxiang
# Proposition: refinement property that the compiler cannot automatically prove
FirstIsMin: (T: Ord, arr: Sorted(T)) -> Type = {
    forall i in 0..arr.len: arr[0] <= arr[i]
}

# Proof: programmer writes a function whose return type is the above proposition
# Type checker verifies this function—exactly the same as verifying add(a,b): Int
first_is_min: (T: Ord, arr: Sorted(T)) -> FirstIsMin(T, arr) = {
    # Compiler verifies here: the function body's type = FirstIsMin(T, arr)
    ...
}
```

No AI needed, no export to Coq, no new concepts. **Properties the compiler cannot automatically
prove → programmer writes proof in YaoXiang code → type checker verifies.** The whole process is a
smooth gradient—the compiler does the simple proofs, leaving the brain for the hard ones.

#### 4.3 Layered Dependencies Within the Pipeline

The above evaluators share the same interface but have an evaluation order. Type equality is the
prerequisite for all subsequent analyses; ownership/token checks depend on type information;
refinement predicate verification depends on the results of the first two layers. The compiler
evaluates layer by layer; expressions failing at lower layers do not enter upper layers—avoiding
wasting solving budget on type-erroneous programs.

```
Evaluation order (same pipeline, layered scheduling)
├── Layer 0: Type equality (T1 == T2)
│   └── Structural unification → failure makes subsequent meaningless, return Disproved directly
├── Layer 1: Ownership/token conflicts
│   └── Flow-sensitive liveness analysis → failure makes memory safety not hold, return Disproved directly
└── Layer 2: Refinement predicates / Hoare implications
    └── Compiler itself → SMT acceleration → get Proved / Disproved / Unproven
```

Each layer still returns `Proved/Disproved/Unproven`, sharing the same interface and the same budget
system.

### 5. Three-Layer Function Unification

| Layer                  | Timing       | Input      | Output | Example                                        |
| ---------------------- | ------------ | ---------- | ------ | ---------------------------------------------- |
| Value-level function   | Runtime      | Value      | Value  | `add: (a: Int, b: Int) -> Int = a + b`         |
| Type constructor       | Compile-time | Type/Value | Type   | `List: (T: Type) -> Type = { data: Array(T) }` |
| Compile-time predicate | Compile-time | Value      | Type   | `Positive: (x: Int) -> Type = { x > 0 }`       |

All use the same `name: type = value` syntax. Compile-time predicates and type constructors go
through the same compile-time proof pipeline—`{}` is proof space.

### 6. Loops: Floyd-Hoare Verification Condition Generation

Loops do not need separate `: Invariant(...)` or `: decreases(...)` annotations. The compile-time
predicate type annotations on variables define Floyd-Hoare style assertions—the compiler generates
verification conditions from type annotations, and the proof pipeline checks whether each assignment
maintains the type.

Core mechanism: each assignment operation corresponds to a Hoare triple `{P} x := e {Q}`, with
verification condition `P ⇒ Q[e/x]`. The compiler generates a single verification condition for the
loop body—once the proof pipeline verifies the inductive step holds, all iterations are
automatically covered.

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
        #     From induction hypothesis s_old == sum(arr[0..i]), add arr[i] to both sides:
        #     sum(arr[0..i]) + arr[i] == sum(arr[0..i+1])
        #   Compiler + SMT: linear arithmetic, millisecond-level → Proved
        #
        # i += 1:
        #   i changes → in dependency graph s's type annotation references i → triggers re-verification
        #   New verification target: s satisfies SumUpTo(arr, i_new)
        #   i.e. s == sum(arr[0..i_new]), guaranteed by previous step → Proved
        s += arr[i]
        i += 1
    }
    return s  # At this point s: SumUpTo(arr, arr.len), i.e. s == sum(arr[0..arr.len])
}
```

Loop invariants are the type annotations on variables—programmer writes types, compiler checks the
inductive step. The compiler doesn't need to "discover" invariants, nor does it need to
"automatically do induction"—it decomposes the inductive proof into local verification conditions
for each assignment operation, handing them over to the proof pipeline for divide and conquer.

#### 6.1 Dependency Tracking: Dependent Types on Mutable Variables

The premise of the above mechanism is: the compiler knows that `s`'s type annotation
`SumUpTo(arr, i)` references `i`—when `i` changes, `s`'s type constraint also changes. This requires
the compiler to maintain a **type dependency graph between variables**.

**Data structure**:

```
TypeDepGraph: Map<VarName, Set<VarName>>
# Key is the depended-on variable, value is the set of variables whose type annotation references that variable
# E.g.: { i: {s}, j: {s, t}, ... }
```

**Construction**: When the type checker processes `mut v: Pred(... x ...) = init`, it parses the
free variable references in `Pred(...)` arguments. If an argument references another mutable
variable `x` in the current scope, it records `x → v` in the dependency graph.

**Trigger**: When a depended-on variable `x` is assigned, the compiler:

1. Looks up all variables `{v₁, v₂, ...}` in the dependency graph that depend on `x`
2. For each `v`, generates a verification condition:
   `does v's current value satisfy the updated type Pred(... x_new ...)`
3. Sends VC to the proof pipeline

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

**Combined dependencies**: A variable can depend on multiple variables. The type annotation
`{ v: Int; v == x + y }` depends on both `x` and `y`—any change triggers re-verification.

**Relationship with the proof pipeline**: Dependency tracking is the trigger for VC generation, not
an independent verification mechanism. It answers "when to generate VCs"—the proof pipeline answers
"whether the VC holds."

### 7. Termination Checking

Compile-time full automation is prioritized. If the compiler can prove the loop passes, it does; if
automatic strategy cannot prove it, a compile error is reported directly—the programmer must let the
compiler automatically analyze the loop's termination. No **annotation syntax** escape hatch (no
`decreases`-style annotations introduced; unsupported things are always syntax positions, not
functions).

> **Revision (RFC-027a, 2026-09-14)**: For termination obligations that the automatic strategy
> cannot prove, the programmer can provide measure functions and termination proof functions
> (`Terminates`) as fallback—both are ordinary YaoXiang functions with zero new syntax, verified by
> the type checker. The full-automation-first principle is unchanged; fallback only occurs after
> Unproven. See
> [RFC-027a: Proof Function Fallback for Termination Checking](../draft/027a-termination-proof-fallback.md).

#### 6.1 Design Principles

The compiler automatically extracts the information needed for termination proofs from two places:

1. **Variable type annotations**: Boundary constraints in refinement types (such as `UpTo(n)`
   providing upper bound `n` and lower bound `0`)
2. **Loop body operations**: Operations applied to variables each iteration

The compiler tries four measure synthesis strategies in order of priority, stopping when one is
found.

#### 6.2 Strategy 1: Automatic Linear Rank Function Synthesis

When variables have linear bound annotations, the compiler enumerates candidate linear measures and
verifies them with SMT.

```
Input:
  Variables v₁: UpTo(u₁), v₂: UpTo(u₂), ... (variables with bounds)
  Loop condition cond
  Set of assignments in loop body

Algorithm:
  1. Extract each variable's bounds from type annotations: [low_i, high_i]
  2. Enumerate candidate measures: v_i, u_i - v_i, v_i - v_j, etc. linear combinations
  3. For each candidate measure m:
     - SMT verify m ≥ 0 (derived from type bounds)
     - For each execution path in loop body, SMT verify m' < m (strictly decreasing)
  4. Find a qualifying linear combination → termination proven
```

Coverage: any loop where a variable is assigned a linear expression (`v = a·v + b`) and has bounded
type annotation. Including `i += const`, `i -= const`, and binary search-style interval shrinking:

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

#### 6.3 Strategy 2: Predicate Violation Counting—Auto-Extracting Measures from Target Type <span style="color:orange">【Experimental Strategy】</span>

> ⚠️ **Current status: Experimental strategy, feasibility to be determined during Phase 3
> implementation.** This strategy works for adjacent swap operations (bubble sort, insertion sort),
> but cannot automatically prove for non-adjacent operations (quicksort partition, heapsort
> sift-down). Coverage boundaries are shown in the table below. If Phase 3 verification proves
> infeasible, this strategy will be removed or downgraded to future work.

Core insight: **The specifications written by the user are material for compiler reasoning.** The
compiler doesn't need to have "what is sorting" built in—it reads the definition of `Sorted` and
automatically extracts measures from the definition.

```
Input:
  Target type: Sorted(arr) = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }
  Loop body operations: adjacent element swaps

Algorithm:
  1. Parse predicate definition: forall i in range: cond(i, arr)
  2. Auto-generate measure: violation_count = |{ i | ¬cond(i, arr) }|
  3. Analyze operation's impact on the measure:
     - Adjacent swap arr[j], arr[j+1] = arr[j+1], arr[j]
     - Only affects index pairs j-1, j, j+1
     - If arr[j] > arr[j+1] (violates predicate), after swap this pair satisfies predicate
     - violation_count decreases by at least 1
  4. Upper bound: n·(n-1)/2 (max adjacent inversions), lower bound: 0
  → Termination proven
```

**Current coverage**:

| Algorithm      | Operation Pattern  | Strategy 2 Provable? | Reason                                             |
| -------------- | ------------------ | :------------------: | -------------------------------------------------- |
| Bubble sort    | Adjacent swap      |          ✅          | violation_count strictly decreases per swap        |
| Insertion sort | Adjacent shift     |          ✅          | Each shift eliminates one violating pair           |
| Selection sort | Non-adjacent swap  |          ❌          | Single swap may increase violation_count           |
| Quicksort      | partition division |          ❌          | Non-adjacent swap, no monotonic decrease guarantee |
| Heapsort       | sift-down          |          ❌          | Tree operation, violation_count not monotonic      |

**Complementary strategies**: For quicksort, the `low < high` interval shrinking is covered by
Strategy 1 (linear rank function)—outer partition recursion, each interval halved. Strategy 1 and
Strategy 2 complement each other, and most practical algorithms' termination can be proven by one of
them. However, the generalization of Strategy 2 (non-adjacent operations, tree operations) remains
an open problem.

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
Strategy 1, handled first by the compiler.

#### 6.5 Strategy 4: Multiplicative Scaling Measure Template

`v *= const` (const > 1), variable has upper and lower bound type annotations. Compiler has built-in
logarithmic measure template `ceil(log_const(upper/v))`, each multiplication by const decreases the
measure by 1.

```yaoxiang
mut i: Positive(i) = 1
while i < n {
    # Compiler auto-derives: measure ceil(log₂(n/i)), each *2 decreases measure by 1
    i *= 2
}
```

#### 6.6 Separation of Termination and Correctness

Termination proofs and correctness proofs are independent:

- **Termination**: The above four strategies automatically prove the loop exits in finite steps
- **Correctness**: Whether the loop body progresses toward the target type, checked by the
  compile-time proof pipeline through verification conditions

Both pass → compilation passes. Termination proven but correctness fails → compile error +
counterexample. Correctness proven but termination not provable → compile error pointing out
unanalyzable variables or operations. Both fail → compile error reporting both failure reasons
separately.

#### 6.7 Termination Checking of Recursive Functions

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

| Scenario                                              | Behavior                    |
| ----------------------------------------------------- | --------------------------- |
| Compiler can analyze recursive decrease (e.g., `n-1`) | Compile-time evaluation     |
| Not decreasing / cannot determine decrease            | Compile error               |
| Runtime call (non-type position)                      | No termination check needed |

#### 6.8 Hard Boundary

`i = f(i)` where `f` is non-invertible, non-closed, and preserves no monotonicity—mathematically
impossible to automatically prove termination. Compile error:

> This loop cannot automatically prove termination. The loop variable depends on the unanalyzable
> function `f`. Please use iteration patterns that can be analyzed by the compiler, or provide a
> measure function and termination proof function (see RFC-027a).

This is not the compiler's failure. Any code that cannot be statically proven safe must not pass
compilation.

### 8. SMT Solver: An Acceleration Module of the Type Checker

The SMT solver is an external tool in traditional languages (e.g., F\* calls Z3, Dafny calls Z3). In
YaoXiang, it is an **acceleration module of the type checker**—only called when the compiler kernel
itself cannot directly determine. SMT helps find proofs, but it is the type checker that verifies
proofs.

**Trust model**: The type checker is the only root of trust. The SMT solver is an acceleration
module—it helps find proofs, but SMT is not an independent trust boundary. The compiler trusts Z3's
`unsat` results (consistent with the F\*/Dafny route—the probability of Z3 making an error is lower
than the compiler's own bug rate, this is a pragmatic engineering choice). The true unreliability
control is in the SMT translation layer—if the translation has bugs, the compiler will expose them
in other tests.

**Interface**: The compiler internally translates to SMT-LIB 2.6 standard format, rather than
binding to a specific solver API. SMT-LIB is an ISO standard, natively supported by Z3, CVC5,
MathSAT, Yices.

**Default backend**: Z3 (MIT license, most extensive documentation and community validation). CVC5
as an SMT-LIB compatible alternative—users can switch at compile time through compiler flags.

No "universal solver abstraction layer"—SMT-LIB is the abstraction layer. In the future, if CVC5
makes breakthroughs in specific theories, switching only requires swapping binaries, no compiler
code changes.

```
Compile-time Bool expression
        │
        ├── Compiler kernel can directly determine (structural equivalence, simple arithmetic,
        │   trivial formulas after constant folding)
        │   → Return Proved / Disproved directly
        │
        └── Compiler kernel cannot directly determine (quantifiers, symbolic variables)
            → Dependent type pre-reduction (factorial(5) → 120)
            → Translate to SMT-LIB format
            → Send to Z3/CVC5 (with budget limit)
            → Return value: unsat → Proved  │  sat + model → Disproved  │  unknown → Unproven
```

**Solving budget—hard limit, like stack depth**:

| Budget Dimension               | Default | Description                                                                                                                                                        |
| ------------------------------ | ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Solving steps                  | 10,000  | Z3 is usually within a hundred steps for linear arithmetic. 10,000 steps covers 99% of practical predicates.                                                       |
| Time                           | 100ms   | A single predicate over 100ms = the user is writing a compile-time program rather than type annotations. 100ms × 50 predicates = 5 seconds compilation time limit. |
| Quantifier instantiation depth | 3       | Three layers of nested quantifiers cover practical patterns. More than three layers is most likely writing logic exercises.                                        |

Exceeding budget returns Unproven, compile error + predicate location + consumption. No degradation,
no runtime check, no silent pass.

**Why this is practical**: 95% of practical predicates in engineering are linear arithmetic—`x > 0`,
`arr.len > 0`, `0 <= idx < arr.len`—all within decidable fragments, and SMT solvers return
millisecond-level for such problems. For the rare complex predicates that exceed budget, the
programmer can write a proof function.

Dependent types have a layer of pre-reduction before SMT calls: `factorial(5)` directly evaluates to
`120` at compile time, `append([1,2], [3])` directly evaluates to `[1,2,3]`. These deterministic
value computations do not consume SMT budget.

Programmers don't need to know SMT exists. Mental model: **The compiler can prove it → passes;
cannot → errors—if the compiler can't, you can write a function to prove it to the compiler.**

### 9. Compile-Time Predicate Composition

Compile-time predicates are functions that return Type, and composition is naturally achieved
through function composition:

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

### 11. Dispatch Routing Pipeline: Unified Dispatch for Compile-Time and Runtime

`assert` and `Assert` are two sides of the same refinement type primitive. The dispatch routing
pipeline `dispatch` automatically decides between compile-time proof and runtime check based on
**whether the free variables of the predicate are accessible at compile time**:

| Criterion                                                                              | Mode            | Behavior                                                                                 |
| -------------------------------------------------------------------------------------- | --------------- | ---------------------------------------------------------------------------------------- |
| All free variables known at compile time (generic params, compile-time constants)      | **CompileTime** | Enter proof pipeline: Proved → erase, Disproved → compile error, Unknown → require proof |
| Some free variables come from runtime (function params, external input, mut variables) | **Runtime**     | Insert runtime check, and inject refinement facts into flow-sensitive assumption set Γ   |

**Key**: "Cannot determine" ≠ "Disproven". In CompileTime mode, Unknown requires proof (no silent
degradation); in Runtime mode, the proposition has no truth value at compile time at all—no matter
how strong the prover is, it cannot write a universally true proof for "the user might have entered
a negative number"; runtime check is the only sound choice. This is not the prover being weak, it is
theoretical necessity.

### 12. Flow-Sensitive Assumption Set Γ: Strongest Postcondition Propagation

The compiler maintains a flow-sensitive assumption set Γ, tracking propositions known to hold at
each control flow point.

**SP (strongest postcondition) propagation**:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
```

**mut variable kill set**: After a `mut` variable is reassigned, all assumptions involving that
variable are removed from Γ:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
mut x = x - 5       // Γ = {}  ← x > 0 is killed
```

This is the hard requirement of soundness—the variable's value changed, old assumptions are invalid.

**Branch confluence**: When IF/ELSE or match branches merge, Γ takes the intersection of each
branch's assumptions. Only propositions that hold in all paths carry out of the branch.

### 13. Clarification of Erasure Model: witness erasure ≠ check erasure

RFC-027's assertion that "refinement types are **completely erased** at runtime" refers to **proof
witnesses**—proof terms already verified at compile time do not produce runtime code. But the
**runtime checks** inserted by dispatch in Runtime mode are retained—they are Bool checks executed
at the value level, not type-level witnesses.

Summary: witnesses erased, checks retained. The two don't conflict, the original RFC-027 assertion
remains unchanged.

## Detailed Design

### Syntax Changes

| Before (RFC-022)                      | After (This RFC)                                                             |
| ------------------------------------- | ---------------------------------------------------------------------------- |
| `//! requires: NonEmpty(n) = n > 0`   | Compile-time predicate as parameter type `(b: Positive(b))`                  |
| `//! ensures: ExistsMax(result, arr)` | Return type uses return value parameter `-> (result: IsMax(T, arr, result))` |
| `/*! invariant: ... !*/`              | Compile-time predicate type annotations on variables—Floyd-Hoare invariants  |
| `//! decreases: n`                    | Compiler fully automatic measure function derivation                         |
| Specifications are comments           | Specifications are the type system                                           |

### Syntax

**Compile-time predicates have no new keywords.** `{}` is proof space, completely consistent with
existing type definition syntax. Compile-time predicates are functions that return
Type—`name: (params) -> Type = { assertion }`. Usage is function calls—`Positive(b)`,
`IsMax(T, arr, result)`.

```bnf
# Compile-time predicate = function returning Type, {} contains compiler-verified assertions
# Uses existing function/type syntax, no new BNF rules needed
predicate ::= identifier ':' params '->' 'Type' '=' '{' assertions '}'
```

**Predicate application arguments must be in compile-time constant form**—literals, variables (bound
by name), or single-parameter type applications (recursive extraction). Arguments in non-convertible
constant form report **E1092**, argument count mismatch with predicate params reports
**E1093**—refinement constraints are **never silently dropped** (previously, non-convertible
arguments would cause constraints to silently disappear, and violating bindings would silently
pass).

**New syntax concept: return value parameter**—`-> (name: Type)` where `name` is the return value
parameter.

The return value parameter is the **only syntax concept** YaoXiang introduces on top of the existing
function syntax. Its semantics:

- The value of `name` is provided by the `return` statement
- `name` exists only in the type signature, referenced by postcondition predicates (such as
  `-> (result: IsMax(T, arr, result))`)
- `name` does not enter the function body scope, does not appear at the call site
- Return value parameter is **optional**—when there is no postcondition, the signature is exactly
  the same as a normal function (`-> Int`), introducing no extra burden

Reason for introduction: postconditions need to reference "the value the function is about to
return." Without return value parameters, the compiler could only let predicates reference the
return value through special rules (such as implicit variable `$result` or `__retval__`). The return
value parameter makes this reference explicit—it is just a parameter, except the value is provided
by `return` rather than the caller.

**Proof functions** are not a new concept—they are just a YaoXiang function whose return type is the
asserted proposition. When the compiler returns Unproven, the programmer provides a proof function,
and the type checker validates it in exactly the same way it validates any function's return type.
No new syntax, no new keywords, no new rules needed.

### Type System Impact

- **Type universe**: Compile-time predicates are at the Type₂ layer—functions that take values and
  return Type, at the same hierarchy level as type constructors
- **Generics interaction**: Compile-time predicates can carry generic parameters, such as
  `NonEmpty: (T: Type) -> (arr: Array(T)) -> Type`
- **Ownership interaction**: Expressions in compile-time predicates obey ownership rules, can only
  read, not write
- **Type inference**: Compile-time predicate parameters participate in HM type inference

### Runtime Representation

Compile-time predicates are handled at runtime **according to dispatch routing results**:

- **CompileTime mode** (all free variables known at compile time): After proof passes, the witness
  is completely erased. `Positive: (x: Int) -> Type = { x > 0 }`—parameter `b: Positive(5)`'s
  runtime representation is just `Int`. The refinement condition `{ 5 > 0 }` has passed, erased.
- **Runtime mode** (runtime free variables exist): Runtime check is retained—Bool check executed at
  the value level, injected into flow-sensitive assumption set Γ. See §11 dispatch routing pipeline
  and §13 erasure model clarification for details.

Placing compile-time predicates in type positions (such as `f(x: Positive(x))`) does not produce
wrapper types or allocate extra memory. But when `x` comes from runtime input, a **runtime Bool
check will be inserted**.

**Interaction constraints with `ref`**: Compile-time predicates can only reference immutably
borrowed values or values whose ownership has been transferred. Compile-time predicates that
reference mutably borrowed values—where the compiler cannot guarantee at compile time that the
verification result still holds at runtime—such usages directly report a compile error.

### Compiler Changes

1. **Parser**: Compile-time predicates use standard function syntax, no additional parsing rules
   needed
2. **Compile-time proof pipeline**: Unified Proved/Disproved/Unproven return interface, automatic
   strategy selection
3. **SMT acceleration module**: SMT-LIB 2.6 translation layer, default backend Z3, CVC5 alternative
4. **Type checker kernel**: Inference rule implementation—structural equivalence, βδι-reduction,
   universal quantifier introduction/elimination. This is the only root of trust; both SMT and
   programmer proofs are verified through it
5. **Verification condition generation**: WP/SP calculus + loop invariant proof obligations
6. **Error reporting**: Counterexample formatting + unsolved proposition reports + source location
   association

### Backward Compatibility

- ✅ Code not using compile-time predicates is completely unchanged
- ✅ Compile-time predicates have zero runtime overhead in CompileTime mode, only necessary Bool
  checks retained in Runtime mode
- ⚠️ RFC-022's `//!` syntax is no longer supported—but 022 was never implemented, no migration
  burden

## Trade-offs

### Advantages

- **Curry-Howard correspondence fully realized**: Types are propositions, programs are proofs,
  `name: Proposition = Proof`
- **Unity**: Compile-time predicates and ordinary functions use exactly the same syntax, no
  conceptual split
- **SMT transparency**: Programmers don't need to know SMT exists, mental model is consistent with
  type checking
- **Progressive adoption**: Can start with one compile-time predicate, gradually expand coverage
- **Minimal runtime overhead**: Zero overhead in CompileTime mode, only necessary Bool checks
  retained in Runtime mode

### Disadvantages

- **Compilation time**: SMT solving increases compilation time, but hard budget limits guarantee
  controllable upper bound
- **Automatic proof boundaries**: Complex predicates beyond first-order linear arithmetic may
  require programmers to write proof functions. This is not a language defect—this is the inevitable
  conclusion of the halting problem. The compiler honestly reports Unproven rather than falsely
  reporting True/False
- **Learning curve**: Writing effective compile-time predicates and proof functions requires
  understanding the basic intuition of the Curry-Howard correspondence
- **Implementation complexity**: Unifying the compile-time proof pipeline requires careful design

### Risk Mitigation

- Hard SMT solving budget limits (10,000 steps / 100ms / instantiation depth 3), exceeding budget
  returns Unproven
- Dependent type pre-reduction: deterministic value computation is consumed first, SMT only chews on
  the non-deterministic part
- Unproven is not a dead end: programmer can write proof function, type checker verifies—just like
  verifying any function return type
- Incremental verification: only validate changed modules
- Clear error messages + counterexample display + budget consumption report + unsolved proposition +
  suggestions (if the compiler can give them)

## Alternatives

| Alternative                                                        | Why Not Chosen                                                                                                                                                        |
| ------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| RFC-022: `//!` comment-style specifications                        | Specifications and types split, violates Curry-Howard correspondence                                                                                                  |
| Independent specification files (e.g., CVL)                        | Specifications separated from code, increases maintenance cost                                                                                                        |
| Runtime-only assertions                                            | Cannot statically guarantee correctness                                                                                                                               |
| External proof assistants (e.g., Coq)                              | Disconnected from compiler, requires independent proof language and trust boundary. YaoXiang's choice: proof is YaoXiang code, type checker is the only root of trust |
| **This proposal: compile-time predicates as first-class citizens** | ✅                                                                                                                                                                    |

## Implementation Strategy

### Phase Division

| Phase       | Content                                                                                                                                                                |
| ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Phase 1** | Compiler kernel: structural equivalence + βδι-reduction + universal quantifier introduction/elimination. Support simple arithmetic predicates (`x > 0`, `arr.len > 0`) |
| **Phase 2** | SMT-LIB translation layer + Z3/CVC5 integration. Pipeline returns Proved/Disproved/Unproven. Unproven supports programmer writing proof functions                      |
| **Phase 3** | Loop invariant VC generation + termination checking (linear rank function + predicate violation counting + bounded pattern + combinatorial explosion control)          |
| **Phase 4** | Incremental verification + caching + IDE support                                                                                                                       |

### Dependencies

- RFC-010: Unified Type Syntax — compile-time predicates based on `name: type = value`
- RFC-011: Generic Type System — compile-time predicates can carry generic parameters
- RFC-009: Ownership Model — expressions in compile-time predicates obey ownership rules

## Open Questions

- [x] **SMT solver selection**: Default Z3 (MIT license, most extensively validated). CVC5 as
      SMT-LIB compatible alternative, switched via compiler flag. The compiler internally translates
      to SMT-LIB 2.6 standard format—SMT-LIB is the abstraction layer, no custom universal solver
      interface.
- [x] **Specific solving budget values**: 10,000 steps / 100ms / quantifier instantiation depth 3.
      Hard-coded in the compiler, no knob. If actual use proves insufficient (not "user wrote it
      wrong"), adjust later.
- [x] **Quantifier support range**: The language doesn't limit quantifier order. Compile-time
      predicates accept Type parameters—Type includes function types—so higher-order quantifiers are
      a natural inference of the type system, no special syntax needed. SMT solver can automatically
      determine first-order quantifiers (forall/exists, supports interleaved nesting, limited by
      budget depth 3). Higher-order quantifiers: SMT returns Unproven, compiler hints "this
      predicate exceeds the automatic proof range, please provide a proof function." Programmer
      writes a YaoXiang function whose return type equals the proposition—type checker verifies the
      function. No external export, no AI, no interactive proof mode needed. Everything is YaoXiang
      code, everything is verified by the type checker.
- [x] **Counterexample formatting**: Source variable names used directly as SMT variable names (with
      module prefix to avoid conflicts). Z3 model returns are reverse-queried by variable name.
      Output format: variable name = specific value + source location + predicate definition
      location. No complex mapping layer.
- [x] ~~**Interaction of compile-time predicates with `ref` smart pointers?**~~ → Decided:
      compile-time predicates only allow immutably borrowed values or values whose ownership has
      been transferred. Mutably borrowed values cannot appear in compile-time predicates.
- [x] **Extension of `forall` predicate violation count measure to non-adjacent operations?** → No
      extension. Current coverage (adjacent swap, adjacent shift) is complemented by Strategy 1
      (linear rank function)—quicksort's outer interval shrinking is covered by Strategy 1, heapsort
      by Strategy 1 (array index pattern). Loops whose termination cannot be proven by any strategy,
      the compiler directly errors—this is hard safety philosophy, not a defect. If in the future
      there are real scenarios (non-academic constructs) where algorithms cannot be covered by any
      of the four strategies, revisit. → **Rediscussion triggered (2026-09-14, #318)**:
      Non-structural recursion (gcd-style non-direct decrease, mutual recursion, merge partition)
      are exactly the real scenarios anticipated by this clause—the current pipeline cannot
      automatically prove their termination, and they cannot be rewritten into analyzable iteration
      patterns without sacrificing readability. Conclusion: full-automation-first unchanged; after
      Unproven, add proof function fallback—measure and termination proof are both ordinary YaoXiang
      functions, zero new syntax, verified by the type checker. Implemented in
      [RFC-027a: Proof Function Fallback for Termination Checking](../draft/027a-termination-proof-fallback.md).
- [x] **Linear rank function enumeration combinatorial explosion**: Candidate enumeration upper
      limit is 3 bounded variables. ≤3 enumerate all linear combinations and verify with SMT one by
      one. >3 only try single-variable measures (`v_i`, `u_i - v_i`), failure directly reports
      compile error—prompting programmer "loop has >3 bounded variables, compiler cannot
      automatically synthesize multi-variable measures." This is not an engineering compromise—it
      forces programmers to write simpler loops.

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
│   Draft     │  ← Author created
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
│ (official)  │    │ (kept in place) │
└─────────────┘    └─────────────┘
```
