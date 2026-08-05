---
title: 'RFC-027: Compile-Time Predicates and Unified Static Verification'
status: 'Accepted'
author: 'Chenxu'
created: '2026-06-07'
updated: '2026-07-05'
impl_status: 'in_progress'
impl_detail:
  'Phase 1-2 completed, Phase 3 partially completed, Phase 4 partially completed. The unified
  assert/Assert scheme has all 6 phases implemented (#157-#162 closed): Never type, IsTrue bridge,
  flow-sensitive Γ + kill set, type-level recursion, universe stratification weak check, and
  dispatch routing pipeline.'
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
> [RFC-022: Hoare Logic Static Verification Support (Specification Annotations and Specification Types)](../deprecated/022-hoare-logic-static-verification.md)
> — Deprecated

## Abstract

This RFC proposes introducing **compile-time predicates** as first-class citizens in YaoXiang,
unifying all compile-time static verification into a single **proof pipeline**. Compile-time
predicates are not external specification annotations—they are functions. A function that returns
`Type` can be used in type positions; the compiler invokes it at compile time and checks the return
value. Types are propositions, compile-time evaluation is proof.

**Core argument**: The sole job of type checking at compile time is to construct and validate proof
terms. Type equality, token conflicts, dependent type reduction, compile-time predicate evaluation,
Hoare-logic entailment—all are different type checks within the compile-time proof pipeline, sharing
the same pipeline. The SMT solver is an acceleration module for the type checker, not an independent
trust boundary. When the compiler returns `Unproven`, the programmer writes a YaoXiang function as a
proof—the type checker validates it in exactly the same way it validates any function's return type.
Everything is YaoXiang code; everything is verified by the type checker.

## Motivation

### Why Deprecate RFC-022?

RFC-022 designs specifications as `//!` annotations:

```yaoxiang
max: (T: Ord) -> ((arr: Array(T, n)) -> T) = {
    //! requires: NonEmpty(n) = n > 0          ← This is an annotation external to the type
    //! ensures: ExistsMax(result, arr[0..n])   ← This is an annotation external to the type
}
```

This commits the fundamental error of the Curry-Howard correspondence: **splitting specifications
and types into two layers**. Annotations are not types. Annotations do not participate in type
checking. Annotations reflect an "external tool" mental model.

The white paper is explicit:

> "No `//!` annotations. No separate specification language. Everything lives within the type
> system."

### Current Problems

- RFC-022's `//!` annotations are external syntax independent of the type system
- Specification types and ordinary types are two distinct systems, causing conceptual redundancy
- The Debug Build verifies / Release Build ignores split pattern breaks uniformity
- SMT solvers are positioned as external tools in conventional understanding—YaoXiang builds them in
  as acceleration modules of the type checker
- Type checking, borrow verification, compile-time predicate checking, and macro expansion each take
  different paths

### The Correct Mental Model

Type checking can be abstracted as a function:

```
verify : Program → Proved | Disproved(Model) | Unproven
```

All compile-time checks—simple type matching, borrow conflict detection, compile-time predicate
verification—are sub-tasks of this function. They share the same proof pipeline; the difference lies
only in the complexity of proof terms and construction strategies.

When the compiler returns `Unproven`, the programmer provides a proof function—whose return type
equals the proposition to be proved. The type checker validates it. This is the same operation as
ordinary type checking.

## Proposal

### 1. `{}` Is the Proof Space: Types Are Assertions, Verification Is Type Checking

YaoXiang's `{}` is the compile-time proof space. Everything inside is an assertion; the compiler
guarantees every item is `True`—either proven automatically or via a proof function supplied by the
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
#          Parameter at signature    Only assertions inside {}
#          Compiler verifies x > 0 at compile time when invoked

List: (T: Type) -> Type = { data: Array(T) }
#      ^^^^^^^^              ^^^^^^^^^^^^^^^
#      Parameter at signature    Compiler verifies type_of(T) == Type, type_of(data) == Array(T)
```

Same pattern: `name: (params) -> Type = { assertions }`. The compiler makes no distinction between
"type assertions" and "value assertions"—all are evaluation targets within the proof pipeline.

**Loop invariants need not be written separately. Type annotations on variables are Floyd-Hoare
invariants.**

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # Annotation references i—tells the compiler that s's type depends on i
    mut i: UpTo(arr.len) = 0     # At initialization, i=0; verify: 0 == sum(arr[0..0]) → True
    while i < arr.len {
        s += arr[i]  # Compiler verifies: s_new == sum(arr[0..i+1])
        i += 1       # i changes → triggers s dependency re-verification: s satisfies SumUpTo(arr, i_new)
    }
    return s  # s: SumUpTo(arr, arr.len) = sum(arr[0..arr.len])
}
```

The compiler generates one verification condition for the loop body—induction hypothesis (type
annotation) → assignment operation → whether the new value satisfies the type annotation. Once the
proof pipeline verifies the inductive step holds, all iterations are covered automatically. No
`: decreases`, no `: Invariant`, no inductive proofs needed—the compiler decomposes induction into
local VCs for each assignment.

### 2. Pre/Postconditions: Compile-Time Predicates on Parameter Types and Return Types

Abandon RFC-022's `//! requires`/`//! ensures`. Compile-time predicates serve as parameter or return
type annotations.

**The parameter side is a function call.** A compile-time predicate is a function returning `Type`;
its use on the parameter side is invoking it—just like `factorial(5)`. The return value side
introduces a new concept: the return value parameter.

```yaoxiang
# Precondition: explicitly invoke compile-time predicate in parameter types
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b
#                       ^^^^^^^^^^  b is the current parameter name, passed to Positive as argument
#                       Compiler extracts the argument value at the call site, substitutes b, verifies Positive(argument)
#                       Example: divide(10, 2) → verify Positive(2) = { 2 > 0 } → True
#                       Example: divide(10, 0) → verify Positive(0) = { 0 > 0 } → False → compile error

# Postcondition: return value parameter + compile-time predicate
IsMax: (T: Ord, arr: Array(T), result: T) -> Type = {
    forall j in 0..arr.len: result >= arr[j]
}

NonEmpty: (arr: Array(T)) -> Type = { arr.len > 0 }

max: (T: Ord) -> ((arr: NonEmpty(arr))) -> (result: IsMax(T, arr, result)) = {
#                                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
#                                            result is the return value parameter; value provided by return
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
  argument. Function call syntax, zero implicit.
- **Return side**: `-> (result: IsMax(T, arr, result))` — `result` is the return value parameter;
  value provided by the `return` statement. `result` exists only in the type signature and is only
  referenced by the predicate; it does not enter the function body scope and does not appear at the
  call site.
- **Return value parameter is optional**: when there is no postcondition, the signature is identical
  to a normal function (`-> Int`).
- **Uniformity**: parameter and return value parameters are the same
  concept—`paramname: predicate_call(paramname)`, the only difference being whether the value is
  provided by the caller or by `return`.

### 3. Path Condition Propagation: Compile-Time Verification of Runtime Values

When a compile-time predicate is used at a binding position, arguments are explicitly passed by the
programmer. When runtime values flow into refined type arguments, the compiler completes
verification through path condition collection and SMT entailment—no need for the programmer to
explicitly pass proofs.

#### 3.1 Explicit Function Calls

When a compile-time predicate is used at a binding position, arguments are explicitly passed by the
programmer—it is just a function call, zero implicit.

`Positive: (x: Int) -> Type = { x > 0 }` is a compile-time predicate constructor. When it appears at
a binding position (parameter declaration, variable declaration, return type), the programmer
explicitly passes the name of a bound variable:

```yaoxiang
b: Positive(b)
// b is already declared as the current parameter; Positive(b) is a function call
// After normalization: b: { b > 0 }
```

The compiler does not need to implicitly fill in arguments—`b: Positive(b)` is a function call just
like `f(5)`. `b` is bound as a parameter name, and its type annotation `Positive(b)` references `b`
itself—this is the standard pattern of dependent types, not an implicit expansion rule.

**Unification with `self` in RFC-010**: RFC-010 established that `self` is not a keyword, merely a
conventional parameter name ("writing `p`, `this`, or `x` has identical effect"). `b: Positive(b)`
shares the same mechanism—parameter names can be referenced in type annotations. `self` appears in
the position `self: Point`; `b` appears in the position `b: Positive(b)`; both type annotations
reference the parameter itself. The difference lies only in annotation complexity; the mechanism is
identical—once a name is bound, the type can depend on that name.

Return types also use explicit function calls:

```yaoxiang
Sorted: (arr: Array(T)) -> Type = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }

sort: (arr: Array(T)) -> (result: Sorted(result)) = { ... }
//                        ^^^^^^^^^^^^^^^^^^^^^^^
//                        result is the return value parameter; Sorted(result) is a function call
//                        Compiler substitutes return value into result at return point, verifies Sorted(return value)
```

The same applies to local variable declarations:

```yaoxiang
let x: Positive(x) = 5
// x is bound to 5; Positive(5) → { 5 > 0 } → True → pass

// let y: Positive(y) = 0
// y is bound to 0; Positive(0) → { 0 > 0 } → False → compile error
```

#### 3.2 Path Condition Collection

When runtime values appear in conditional branches, the compiler automatically collects path
conditions, forming the current scope's **assumption set**. These assumptions participate in
verification as background knowledge for compile-time `Bool` evaluation.

```yaoxiang
if y > 0 {
    // Compiler automatically acquires assumption in this branch: { y > 0 }
    let result = divide(x, y)
    // Verification condition: (y > 0) ⇒ (y > 0)
    // Proof pipeline judges entailment holds → Proved
} else {
// This branch assumes: { !(y > 0) }
// If divide(x, y) is called, the verification condition is !(y > 0) ⇒ y > 0
    // Proof pipeline judges entailment does not hold → Disproved
}
```

This is not a hard-coded special pattern in the compiler—this is the natural behavior of the
compile-time proof pipeline. At each type check call site, the pipeline sends:

```
{background assumptions} ⇒ {verification goal}
```

The proof pipeline judges entailment. `Proved` → pass; `Disproved` → compile error + counterexample;
`Unproven` → compile error + unsolved proposition. Background assumptions come from the path
condition at the current program point.

#### 3.3 Assumption Stack

When analyzing control flow, the compiler maintains an assumption set for each basic block:

- **if-guard**: `if y > 0` → true branch pushes `y > 0`; false branch pushes `!(y > 0)` (if `else`
  is used)
- **match pattern**: `if let Some(v) = opt` → branch pushes `opt == Some(v)`
- **Logical conjunction**: `if x > 0 and y < 10` → branch pushes `x > 0` and `y < 10`
- **Function precondition**: when calling `divide(a, b)`, the evidence that `b` satisfies `Positive`
  must come either from current assumptions, or from the argument's own refined type annotation (if
  `b` is annotated as `Positive`, its type carries `b > 0`)
- **Assignment**: when `let z = y`, refinement conditions already on `y` are transferred to `z`

All assumptions enter the compile-time proof pipeline. When entering the SMT acceleration path, they
are translated to SMT-LIB background assertions.

#### 3.4 No Static Evidence → Compile Error

If the programmer directly writes:

```yaoxiang
divide_user_input: (x: Int, y: Int) -> Int = divide(x, y)
```

The current program point has no `y > 0` assumption; the argument `y` itself has no `Positive` type
annotation. The verification condition is:

```
{} ⇒ { y > 0 }
```

The pipeline returns `Disproved` (no entailment) → compile error:

> Cannot prove that parameter `b` satisfies `Positive` in the `divide` call. `y` comes from function
> input, with no proven bound. Consider guarding the call with an if branch:
> `if y > 0 { divide(x, y) }`.

YaoXiang does not accept runtime values flowing directly into refined type arguments without static
evidence. This is not a restriction—this is the core of the hard-safety philosophy. Code that the
compiler cannot statically prove must not pass compilation.

#### 3.5 Relationship to the Unified Pipeline

Path condition propagation is not an additional mechanism. It is the direct extension of the
compile-time proof pipeline in control flow analysis:

| Stage                             | Responsibility                                                                                                                                             |
| --------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Path condition collection         | Compiler control flow analysis stage, annotates each basic block with an assumption set                                                                    |
| Verification condition generation | When a type constraint requiring verification is encountered, merges path conditions + argument type information                                           |
| Proof pipeline evaluation         | Compiler kernel → SMT acceleration → obtain Proved / Disproved / Unproven                                                                                  |
| Result                            | `Proved` → pass; `Disproved` → compile error + counterexample; `Unproven` → compile error + unsolved proposition (programmer may provide a proof function) |

No new components. No special rules. Path conditions are the background knowledge of the proof
pipeline—sharing the same pipeline and the same budget system as type equality and borrow
constraints.

### 4. The Compile-Time Proof Pipeline

All compile-time checks share the same pipeline. The core operation of the pipeline is **type
checking**—checking whether the type of a proof term equals the proposition to be proved. Everything
is type checking.

```
At compile time, a Bool expression requires evaluation (i.e., a proof term must be constructed)
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
        │   → Compiler itself + SMT acceleration module
        │
        └── Hoare-logic entailment (P ⇒ Q)
            → Compiler + SMT acceleration module
                    │
                    ▼
             ┌──────────┐
             │ Proved   │  → Compile passes
             │ Disproved│  → Compile error + counterexample
             │ Unproven │  → Compile error + unsolved proposition
             └────┬─────┘
                  │
                  ▼
         Programmer writes proof function (YaoXiang code)
                  │
                  ▼
         Type checker verifies ──→ Proved ──→ Compile passes
                  │
                  ▼
            Verification fails → Compile error: "proof does not hold"
```

#### 4.1 Proof Results: A Three-Valued Algebra

Compile-time evaluation returns three results—an inevitable consequence of the halting problem, and
the natural partition of proof theory:

```
eval_compile_time : BoolExpr → Proved | Disproved(Model) | Unproven
```

- **Proved** → Halts, proof term constructed, type check passes. Compilation continues.
- **Disproved(M)** → Halts, counterexample M exists. Compile error + counterexample + source
  location.
- **Unproven** → Within the given resource cap, no proof has been constructed. Compile error +
  unsolved proposition + budget consumption report.

**Unproven ≠ False.** The compiler saying "I cannot prove it" is not equivalent to the proposition
being false—only that it exceeds current automatic proof capabilities. This is honesty, not a
defect.

A hard budget cap is the engineering solution to the halting problem. No knob is provided—giving one
amounts to asking the user "do you think your program will halt", and neither the user nor the
compiler knows.

#### 4.2 After Unproven: The Programmer Writes the Proof

When the compiler returns `Unproven`, the programmer can write a **proof function**—a YaoXiang
function whose return type equals the proposition to be proved. The type checker validates this
function—using exactly the same mechanism as it validates `add(a, b): Int`.

```
Proposition = Type
Proof       = Program (a value of that type)
Verification= Type check (the sole trust root)
```

The SMT solver is not an independent trust boundary—it is an **acceleration module of the type
checker**. SMT helps find proofs, but it is the type checker that verifies them. When SMT returns
`unsat`, the compiler reconstructs its result as a proof term verifiable by the type checker. If
reconstruction fails (SMT's reasoning steps exceed the compiler kernel's inference rules), it falls
back to `Unproven`—the programmer can write a proof function manually.

```yaoxiang
# Proposition: a refined property the compiler cannot prove automatically
FirstIsMin: (T: Ord, arr: Sorted(T)) -> Type = {
    forall i in 0..arr.len: arr[0] <= arr[i]
}

# Proof: the programmer writes a function whose return type is the proposition above
# The type checker validates this function—exactly the same as validating add(a,b): Int
first_is_min: (T: Ord, arr: Sorted(T)) -> FirstIsMin(T, arr) = {
    # Compiler verifies here: function body's type = FirstIsMin(T, arr)
    ...
}
```

No AI, no export to Coq, no new concepts. **Properties that cannot be automatically proven at
compile time → the programmer writes a proof in YaoXiang code → the type checker validates it.** The
whole process is a smooth gradient—the compiler handles simple proofs, leaving the hard ones to the
brain.

#### 4.3 Layered Dependencies Within the Pipeline

The above evaluators share the same interface but have an evaluation order. Type equality is a
prerequisite for all subsequent analyses; ownership/token checks depend on type information; refined
predicate verification depends on the results of the first two layers. The compiler evaluates layer
by layer; expressions failing at lower layers do not enter upper layers—avoiding wasting solver
budget on ill-typed programs.

```
Evaluation order (same pipeline, layered scheduling)
├── Layer 0: Type equality (T1 == T2)
│   └── Structural unification → on failure subsequent analysis is meaningless, directly return Disproved
├── Layer 1: Ownership/token conflict
│   └── Flow-sensitive liveness analysis → on failure memory safety does not hold, directly return Disproved
└── Layer 2: Refined predicate / Hoare entailment
    └── Compiler itself → SMT acceleration → obtain Proved / Disproved / Unproven
```

Each layer still returns `Proved/Disproved/Unproven`, sharing the same interface and the same budget
system.

### 5. Unification of Three Function Layers

| Layer                  | Execution timing | Input      | Output | Example                                        |
| ---------------------- | ---------------- | ---------- | ------ | ---------------------------------------------- |
| Value-level function   | Runtime          | value      | value  | `add: (a: Int, b: Int) -> Int = a + b`         |
| Type constructor       | Compile-time     | type/value | Type   | `List: (T: Type) -> Type = { data: Array(T) }` |
| Compile-time predicate | Compile-time     | value      | Type   | `Positive: (x: Int) -> Type = { x > 0 }`       |

All use the same `name: type = value` syntax. Compile-time predicates and type constructors go
through the same compile-time proof pipeline—`{}` is the proof space.

### 6. Loops: Floyd-Hoare Verification Condition Generation

Loops do not need separate `: Invariant(...)` or `: decreases(...)` annotations. Compile-time
predicate type annotations on variables define Floyd-Hoare-style assertions—the compiler generates
verification conditions from type annotations, and the proof pipeline checks whether each assignment
preserves the type.

Core mechanism: each assignment operation corresponds to a Hoare triple `{P} x := e {Q}`, with
verification condition `P ⇒ Q[e/x]`. The compiler generates one verification condition for the loop
body—once the proof pipeline verifies the inductive step holds, all iterations are covered
automatically.

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
        #     Requires s_old + arr[i] == sum(arr[0..i+1])
        #     From inductive hypothesis s_old == sum(arr[0..i]), adding arr[i] to both sides:
        #     sum(arr[0..i]) + arr[i] == sum(arr[0..i+1])
        #   Compiler + SMT: linear arithmetic, millisecond-level → Proved
        #
        # i += 1:
        #   i changes → dependency graph shows s's type annotation references i → triggers re-verification
        #   New verification goal: s satisfies SumUpTo(arr, i_new)
        #   i.e., s == sum(arr[0..i_new]), guaranteed by the previous step → Proved
        s += arr[i]
        i += 1
    }
    return s  # At this point s: SumUpTo(arr, arr.len), i.e., s == sum(arr[0..arr.len])
}
```

Loop invariants are exactly the type annotations on variables—the programmer writes types, the
compiler checks the inductive step. The compiler does not need to "discover" invariants, nor
"automatically perform induction"—it decomposes the inductive proof into local verification
conditions for each assignment operation, and delegates them to the proof pipeline for
divide-and-conquer.

#### 6.1 Dependency Tracking: Dependent Types on Mutable Variables

The prerequisite of the above mechanism is: the compiler knows that `s`'s type annotation
`SumUpTo(arr, i)` references `i`—when `i` changes, the type constraint on `s` also changes
accordingly. This requires the compiler to maintain a **type dependency graph between variables**.

**Data structure**:

```
TypeDepGraph: Map<VarName, Set<VarName>>
# Key is the depended-upon variable; value is the set of variables that reference it in their type annotations
# Example: { i: {s}, j: {s, t}, ... }
```

**Construction**: When processing `mut v: Pred(... x ...) = init`, the type checker parses free
variable references in the `Pred(...)` arguments. If the arguments reference another mutable
variable `x` in the current scope, record `x → v` in the dependency graph.

**Trigger**: When the depended-upon variable `x` is assigned, the compiler:

1. Looks up all variables in the dependency graph that depend on `x`: `{v₁, v₂, ...}`
2. For each `v`, generates a verification condition: whether `v`'s current value satisfies the
   updated type `Pred(... x_new ...)`
3. Sends the VC into the proof pipeline

**Assignment order sensitivity**: dependency tracking naturally enforces the correct assignment
order. Take `SumUpTo(arr, i)` as an example:

```yaoxiang
# Correct order
s += arr[i]   # s_new satisfies SumUpTo(arr, i+1)
i += 1        # i changes → re-verify s satisfies SumUpTo(arr, i_new) → True

# Wrong order—compiler rejects
i += 1        # i changes → re-verify s satisfies SumUpTo(arr, i_new)
              # s not yet updated, s_old == sum(arr[0..i_old]) ≠ sum(arr[0..i_new])
              # → Compile error: variable s does not satisfy type SumUpTo(arr, i_new)
s += arr[i]   # unreachable
```

**Composite dependencies**: a variable can depend on multiple variables. The type annotation
`{ v: Int; v == x + y }` depends on both `x` and `y`—any change triggers re-verification.

**Relationship to the proof pipeline**: dependency tracking is the trigger for VC generation, not an
independent verification mechanism. It answers "when must a VC be generated"—the proof pipeline
answers "whether the VC holds".

### 7. Termination Checking

Fully automatic at compile time. Loops the compiler can prove pass; loops the compiler cannot prove
directly report a compile error—the programmer must make the loop analyzable automatically. No
escape hatch for semi-automatic annotations.

#### 6.1 Design Principles

The compiler automatically extracts the information needed for termination proofs from two sources:

1. **Variable type annotations**: boundary constraints in refined types (e.g., `UpTo(n)` gives upper
   bound `n` and lower bound `0`)
2. **Loop body operations**: operations applied to variables on each iteration

The compiler attempts four measure synthesis strategies in priority order, stopping once one
succeeds.

#### 6.2 Strategy 1: Automatic Linear Rank Function Synthesis

When a variable has a linear bound annotation, the compiler enumerates candidate linear measures and
validates them with SMT.

```
Input:
  Variables v₁: UpTo(u₁), v₂: UpTo(u₂), ... (variables with upper/lower bounds)
  Loop condition cond
  Set of assignments in the loop body

Algorithm:
  1. Extract each variable's bound from type annotations: [low_i, high_i]
  2. Enumerate candidate measures: v_i, u_i - v_i, v_i - v_j, etc., linear combinations
  3. For each candidate measure m:
     - SMT verifies m ≥ 0 (derived from type bounds)
     - For each execution path in the loop body, SMT verifies m' < m (strict decrease)
  4. Find a qualifying linear combination → termination proven
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

#### 6.3 Strategy 2: Predicate Violation Count—Auto-Extracting Measures from Goal Types <span style="color:orange">[Experimental Strategy]</span>

> ⚠️ **Current status: experimental strategy; inclusion in Phase 3 implementation depends on actual
> feasibility.** This strategy works for adjacent swap operations (bubble sort, insertion sort); it
> cannot automatically prove non-adjacent operations (quicksort partition, heapsort sift-down).
> Coverage boundaries are shown in the table below. If Phase 3 verification proves it infeasible,
> this strategy will be removed or downgraded to future work.

Core insight: **the user's specification is the compiler's reasoning material.** The compiler does
not need built-in knowledge of "what sorting is"—it reads the `Sorted` definition and automatically
extracts the measure from the definition.

```
Input:
  Goal type: Sorted(arr) = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }
  Loop body operation: adjacent element swap

Algorithm:
  1. Parse predicate definition: forall i in range: cond(i, arr)
  2. Automatically generate measure: violation_count = |{ i | ¬cond(i, arr) }|
  3. Analyze operation's effect on the measure:
     - Adjacent swap arr[j], arr[j+1] = arr[j+1], arr[j]
     - Only affects three pairs at indices j-1, j, j+1
     - If arr[j] > arr[j+1] (predicate violation), after the swap this pair satisfies the predicate
     - violation_count decreases by at least 1
  4. Upper bound: n·(n-1)/2 (maximum adjacent inversions); lower bound: 0
  → Termination proven
```

**Current coverage**:

| Algorithm      | Operation pattern | Strategy 2 provable? | Reason                                                  |
| -------------- | ----------------- | :------------------: | ------------------------------------------------------- |
| Bubble sort    | Adjacent swap     |          ✅          | violation_count strictly decreases with each swap       |
| Insertion sort | Adjacent move     |          ✅          | Each shift eliminates one violating pair                |
| Selection sort | Non-adjacent swap |          ❌          | A single swap may increase violation_count              |
| Quicksort      | Partitioning      |          ❌          | Non-adjacent swap; monotonic decrease not guaranteed    |
| Heapsort       | sift-down         |          ❌          | Tree-shaped operation; violation_count is not monotonic |

**Complementary strategies**: for quicksort, the `low < high` interval contraction is covered by
Strategy 1 (linear rank function)—the outer partition recursion halves the interval each time.
Strategies 1 and 2 complement each other; termination of most practical algorithms can be proven by
one of them. However, generalizing Strategy 2 (non-adjacent operations, tree-shaped operations)
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

#### 6.4 Strategy 3: Bounded Increment/Decrement Patterns

`v += const` (positive constant), the variable has an upper-bound type annotation → measure
`upper_bound - v` decreases by `const` each time, lower bound 0. This is a degenerate case of
Strategy 1, handled quickly at the front of the compiler.

#### 6.5 Strategy 4: Multiplicative Scaling Measure Templates

`v *= const` (const > 1), the variable has upper- and lower-bound type annotations. The compiler has
a built-in logarithmic measure template `ceil(log_const(upper/v))`, which decreases by 1 each time
`const` is multiplied.

```yaoxiang
mut i: Positive(i) = 1
while i < n {
    # Compiler automatically derives: measure ceil(log₂(n/i)), decreases by 1 each time it doubles
    i *= 2
}
```

#### 6.6 Termination and Correctness Separation

Termination proof and correctness proof are independent:

- **Termination**: the above four strategies automatically prove the loop exits in finite steps
- **Correctness**: whether the loop body advances toward the goal type, checked by the compile-time
  proof pipeline via verification conditions

Both pass → compile passes. Termination proven but correctness fails → compile error +
counterexample. Correctness proven but termination unprovable → compile error pointing to the
unanalyzable variable or operation. Both fail → compile error reporting each failure separately.

#### 6.7 Termination Checking for Recursive Functions

For recursive functions requiring compile-time evaluation, the compiler checks that arguments
decrease:

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)  # Compiler analysis: n-1 < n → decreasing → terminates
}

# Compile-time use—compiler guarantees factorial terminates at compile time
vec: Vec(factorial(5)) = Vec(120)()  # 5! = 120, completed at compile time
```

| Scenario                                             | Behavior                      |
| ---------------------------------------------------- | ----------------------------- |
| Compiler can analyze recursive decrease (e.g. `n-1`) | Compile-time evaluation       |
| Does not decrease / cannot determine decrease        | Compile error                 |
| Runtime call (not in type position)                  | No termination check required |

#### 6.8 Hard Boundary

`i = f(i)` where `f` is non-invertible, non-closed, and does not preserve any
monotonicity—mathematically, automatic termination proof is impossible. Compile error:

> This loop cannot be automatically proven to terminate. The loop variable depends on the
> unanalyzable function `f`. Please use an iteration pattern that can be analyzed by the compiler.

This is not a compiler failure. All code that cannot be statically proven safe must not pass
compilation.

### 8. The SMT Solver: An Acceleration Module of the Type Checker

In conventional languages, the SMT solver is an external tool (e.g., F\* calls Z3, Dafny calls Z3).
In YaoXiang, it is an **acceleration module of the type checker**—invoked only when the compiler
kernel itself cannot directly decide. SMT helps find proofs; it is the type checker that verifies
them.

**Trust model**: the type checker is the sole trust root. The SMT solver is an acceleration
module—it helps find proofs, but SMT is not an independent trust boundary. The compiler trusts Z3's
`unsat` result (consistent with the F\*/Dafny approach—the probability of Z3 errors is lower than
the bug rate of the compiler itself, an engineering-pragmatic choice). True unreliability is
controlled at the SMT translation layer—if the translation has a bug, the compiler will expose it in
other tests.

**Interface**: the compiler internally translates to the SMT-LIB 2.6 standard format, rather than
binding to a specific solver API. SMT-LIB is an ISO standard, natively supported by Z3, CVC5,
MathSAT, Yices.

**Default backend**: Z3 (MIT license, the most extensive documentation and community validation).
CVC5 as an SMT-LIB-compatible alternative—users can switch at compile time via compiler flags.

No "generic solver abstraction layer"—SMT-LIB is the abstraction layer. In the future, if CVC5
achieves a breakthrough in a particular theory, switching only requires swapping the binary, not
changing compiler code.

```
Compile-time Bool expression
        │
        ├── Compiler kernel can decide directly (structural equivalence, simple arithmetic,
        │   trivial formulas after constant folding)
        │   → Directly return Proved / Disproved
        │
        └── Compiler kernel cannot decide directly (quantifiers, symbolic variables)
            → Dependent type pre-reduction (factorial(5) → 120)
            → Translate to SMT-LIB format
            → Send to Z3/CVC5 (with budget limit)
            → Return value: unsat → Proved  │  sat + model → Disproved  │  unknown → Unproven
```

**Solver budget—hard cap, like stack depth**:

| Budget dimension               | Default | Description                                                                                                                                            |
| ------------------------------ | ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Solver steps                   | 10,000  | Z3 typically finishes linear arithmetic within a hundred steps. 10,000 steps cover 99% of practical predicates.                                        |
| Time                           | 100ms   | A single predicate exceeding 100ms = user is writing a compile-time program, not a type annotation. 100ms × 50 predicates = 5-second compile time cap. |
| Quantifier instantiation depth | 3       | Three nested quantifier layers cover practical patterns. Beyond three layers, the user is likely writing logic exercises.                              |

Exceeding budget returns `Unproven`; compile error + predicate location + consumption. No
degradation, no runtime check, no silent pass.

**Why this is practically feasible**: 95% of real-world predicates in engineering are linear
arithmetic—`x > 0`, `arr.len > 0`, `0 <= idx < arr.len`—all within decidable fragments, and SMT
solvers return in milliseconds. For the rare complex predicates that exceed the budget, the
programmer can write a proof function.

Dependent types undergo a layer of pre-reduction before SMT calls: `factorial(5)` directly
compile-time-evaluates to `120`; `append([1,2], [3])` directly evaluates to `[1,2,3]`. These
deterministic value computations do not consume SMT budget.

The programmer need not know SMT exists. The mental model is: **the compiler proves what it can and
passes it; what it cannot, it errors— if the compiler cannot, you can write a function to prove it
to the compiler.**

### 9. Compile-Time Predicate Composition

A compile-time predicate is a function returning `Type`; composition is naturally achieved through
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

### 11. The Dispatch Routing Pipeline: Unified Routing Between Compile-Time and Runtime

`assert` and `Assert` are two sides of the same refinement type primitive. The `dispatch` routing
pipeline automatically decides between compile-time proof and runtime check based on **whether the
predicate's free variables are accessible at compile time**:

| Criterion                                                                                  | Mode            | Behavior                                                                                 |
| ------------------------------------------------------------------------------------------ | --------------- | ---------------------------------------------------------------------------------------- |
| All free variables known at compile time (generic parameters, compile-time constants)      | **CompileTime** | Enter proof pipeline: Proved → erase; Disproved → compile error; Unknown → require proof |
| Some free variables come from runtime (function parameters, external input, mut variables) | **Runtime**     | Insert runtime check, and inject refined facts into flow-sensitive assumption set Γ      |

**Key**: "Cannot decide" ≠ "falsified". In CompileTime mode, `Unknown` requires a proof (no silent
degradation); in Runtime mode, the proposition has no truth value at compile time at all—no prover,
however strong, can write a tautological proof for "the user might have entered a negative number";
a runtime check is the only sound choice. This is not the prover being too weak; it is theoretical
necessity.

### 12. Flow-Sensitive Assumption Set Γ: Strongest Postcondition Propagation

The compiler maintains a flow-sensitive assumption set Γ, tracking propositions known to hold at
each control-flow point.

**SP (Strongest Postcondition) propagation**:

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

This is a hard soundness requirement—when a variable's value changes, old assumptions are invalid.

**Branch merge**: when IF/ELSE or match branches merge, Γ takes the intersection of each branch's
assumptions. Only propositions that hold on all paths are carried out of the branch.

### 13. Erasure Model Clarification: Witness Erasure ≠ Check Erasure

RFC-027's claim that "refined types are **fully erased** at runtime" refers to **proof
witnesses**—proof terms already verified at compile time produce no runtime code. But runtime checks
inserted by `dispatch` in Runtime mode are preserved—they are `Bool` checks executed at the value
layer, not witnesses at the type layer.

Summary: witnesses are erased; checks are preserved. The two do not conflict, and RFC-027's original
claim remains unchanged.

## Detailed Design

### Syntax Changes

| Before (RFC-022)                      | After (this RFC)                                                             |
| ------------------------------------- | ---------------------------------------------------------------------------- |
| `//! requires: NonEmpty(n) = n > 0`   | Compile-time predicate as parameter type `(b: Positive(b))`                  |
| `//! ensures: ExistsMax(result, arr)` | Return type uses return value parameter `-> (result: IsMax(T, arr, result))` |
| `/*! invariant: ... !*/`              | Compile-time predicate type annotation on variables—Floyd-Hoare invariant    |
| `//! decreases: n`                    | Compiler fully automatically derives measure function                        |
| Specifications are annotations        | Specifications are the type system                                           |

### Syntax

**No new keywords for compile-time predicates.** `{}` is the proof space, fully consistent with the
existing type definition syntax. A compile-time predicate is just a function returning
`Type`—`name: (params) -> Type = { assertions }`. Usage is just a function call—`Positive(b)`,
`IsMax(T, arr, result)`.

```bnf
# Compile-time predicate = function returning Type, {} contains compiler-verified assertions
# Uses existing function/type syntax; no new BNF rules required
predicate ::= identifier ':' params '->' 'Type' '=' '{' assertions '}'
```

**Predicate application arguments must be in compile-time constant form**—literals, variables (bound
by name), or single-parameter type applications (recursively extracted). Non-convertible argument
forms report **E1092**; argument count mismatches with predicate parameters report
**E1093**—refinement constraints **must never be silently dropped** (#263: previously,
non-convertible arguments would silently cause constraints to disappear, allowing violating bindings
to pass silently).

**New syntax concept: return value parameter**—in `-> (name: Type)`, `name` is the return value
parameter.

The return value parameter is the **only syntax concept** introduced by YaoXiang on top of the
existing function syntax. Its semantics:

- `name`'s value is provided by the `return` statement
- `name` exists only in the type signature and is referenced by postcondition predicates (e.g.,
  `-> (result: IsMax(T, arr, result))`)
- `name` does not enter the function body scope and does not appear at the call site
- The return value parameter is **optional**—without a postcondition, the signature is identical to
  a normal function (`-> Int`), imposing no extra burden

Rationale for introduction: postconditions need to reference "the value the function is about to
return". Without a return value parameter, the compiler can only let predicates reference the return
value through special rules (such as an implicit variable `$result` or `__retval__`). The return
value parameter makes this reference explicit—it is just a parameter, except that the value is
provided by `return` rather than by the caller.

**Proof functions** are not a new concept—they are just YaoXiang functions whose return type is the
asserted proposition. When the compiler returns `Unproven`, the programmer provides a proof
function, and the type checker validates it in exactly the same way as it validates any function's
return type. No new syntax, no new keywords, no new rules.

### Type System Impact

- **Type universe**: compile-time predicates reside in the Type₂ layer—functions taking values and
  returning `Type`, on the same level as type constructors
- **Generic interaction**: compile-time predicates may carry generic parameters, e.g.,
  `NonEmpty: (T: Type) -> (arr: Array(T)) -> Type`
- **Ownership interaction**: expressions in compile-time predicates obey ownership rules; they can
  only read, not write
- **Type inference**: compile-time predicate parameters participate in HM type inference

### Runtime Representation

Compile-time predicates are processed at runtime **according to the `dispatch` routing result**:

- **CompileTime mode** (all free variables known at compile time): after proof, the witness is fully
  erased. `Positive: (x: Int) -> Type = { x > 0 }`—the parameter `b: Positive(5)`'s runtime
  representation is simply `Int`. The refinement condition `{ 5 > 0 }` has passed; erased.
- **Runtime mode** (runtime free variables exist): runtime checks are preserved—execute `Bool`
  checks at the value layer, inject into the flow-sensitive assumption set Γ. See §11 dispatch
  routing pipeline and §13 erasure model clarification for details.

Placing compile-time predicates in type positions (e.g., `f(x: Positive(x))`) does not produce
wrapper types and does not allocate extra memory. But when `x` comes from runtime input, a runtime
`Bool` check **is** inserted.

**Interaction constraints with `ref`**: compile-time predicates may only reference values with
immutable borrows or whose ownership has been transferred. Compile-time predicates that reference
mutably borrowed values cannot be guaranteed by the compiler to remain valid at runtime—such uses
directly report a compile error.

### Compiler Changes

1. **Parser**: compile-time predicates use standard function syntax; no additional parsing rules
   needed
2. **Compile-time proof pipeline**: unified `Proved/Disproved/Unproven` return interface; automatic
   strategy selection
3. **SMT acceleration module**: SMT-LIB 2.6 translation layer; default backend Z3, CVC5 as
   alternative
4. **Type checker kernel**: inference rule implementation—structural equivalence, βδι-reduction,
   universal quantifier introduction/elimination. This is the sole trust root; both SMT and
   programmer-supplied proofs are validated through it
5. **Verification condition generation**: WP/SP calculus + loop invariant proof obligations
6. **Error reporting**: counterexample formatting + unsolved proposition reports + source location
   correlation

### Backward Compatibility

- ✅ Code not using compile-time predicates is entirely unchanged
- ✅ Compile-time predicates in CompileTime mode have zero runtime overhead; in Runtime mode only
  the necessary `Bool` checks are preserved
- ⚠️ RFC-022's `//!` syntax is no longer supported—but 022 was never implemented, so there is no
  migration burden

## Trade-offs

### Advantages

- **Curry-Howard correspondence fully realized**: types are propositions, programs are proofs,
  `name: Proposition = Proof`
- **Uniformity**: compile-time predicates and ordinary functions use exactly the same syntax; no
  conceptual split
- **SMT transparency**: programmers need not know SMT exists; the mental model matches type checking
- **Progressive adoption**: start with a single compile-time predicate and gradually increase
  coverage
- **Minimal runtime overhead**: zero overhead in CompileTime mode; only necessary `Bool` checks
  retained in Runtime mode

### Disadvantages

- **Compile time**: SMT solving increases compile time, but hard budget caps keep the upper bound
  controllable
- **Boundaries of automatic proof**: complex predicates beyond first-order linear arithmetic may
  require the programmer to write proof functions. This is not a language defect—it is the
  inevitable consequence of the halting problem. The compiler honestly reports `Unproven` rather
  than falsely reporting `True`/`False`
- **Learning curve**: writing effective compile-time predicates and proof functions requires basic
  intuition about the Curry-Howard correspondence
- **Implementation complexity**: unifying the compile-time proof pipeline requires careful design

### Risk Mitigation

- Hard SMT solver budget caps (10,000 steps / 100ms / 3 quantifier instantiation depth); exceeding
  budget returns `Unproven`
- Dependent type pre-reduction: deterministic value computations are consumed first; SMT only chews
  the non-deterministic part
- `Unproven` is not a dead end: the programmer can write a proof function, and the type checker
  validates it—same as validating any function's return type
- Incremental verification: only validate changed modules
- Clear error messages + counterexample display + budget consumption report + unsolved proposition +
  suggestions (if the compiler can provide one)

## Alternatives

| Alternative                                                        | Why not chosen                                                                                                                                                               |
| ------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| RFC-022: `//!` annotation-style specifications                     | Splits specifications from types, violating the Curry-Howard correspondence                                                                                                  |
| Separate specification files (e.g. CVL)                            | Separates specifications from code, increasing maintenance cost                                                                                                              |
| Runtime-only assertions                                            | Cannot statically guarantee correctness                                                                                                                                      |
| External proof assistants (e.g. Coq)                               | Disjoint from the compiler, requiring an independent proof language and trust boundary. YaoXiang's choice: proofs are YaoXiang code; the type checker is the sole trust root |
| **This proposal: compile-time predicates as first-class citizens** | ✅                                                                                                                                                                           |

## Implementation Strategy

### Phase Breakdown

| Phase       | Content                                                                                                                                                                 |
| ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Phase 1** | Compiler kernel: structural equivalence + βδι-reduction + universal quantifier introduction/elimination. Supports simple arithmetic predicates (`x > 0`, `arr.len > 0`) |
| **Phase 2** | SMT-LIB translation layer + Z3/CVC5 integration. Pipeline returns Proved/Disproved/Unproven. Unproven supports programmer-written proof functions                       |
| **Phase 3** | Loop invariant VC generation + termination checking (linear rank function + predicate violation count + bounded patterns + combinatorial explosion control)             |
| **Phase 4** | Incremental verification + caching + IDE support                                                                                                                        |

### Dependencies

- RFC-010: Unified Type Syntax — compile-time predicates are based on `name: type = value`
- RFC-011: Generic Type System — compile-time predicates may carry generic parameters
- RFC-009: Ownership Model — compile-time predicate expressions obey ownership rules

## Open Issues

- [x] **SMT solver choice**: default Z3 (MIT license, most widely validated). CVC5 as
      SMT-LIB-compatible alternative, switchable via compiler flag. The compiler's internal
      translation target is SMT-LIB 2.6 standard format—SMT-LIB is the abstraction layer; no custom
      generic solver interface.
- [x] **Specific solver budget values**: 10,000 steps / 100ms / 3 quantifier instantiation depth.
      Hard-coded in the compiler; no knob. If real use cases prove insufficient (not "the user wrote
      it wrong"), adjustments will be made.
- [x] **Quantifier support scope**: the language layer does not limit quantifier rank. Compile-time
      predicates accept `Type` parameters—`Type` includes function types—therefore higher-order
      quantifiers are a natural consequence of the type system, requiring no special syntax. The SMT
      solver can automatically decide first-order quantifiers (forall/exists, with interleaved
      nesting, limited by budget depth 3). Higher-order quantifiers: SMT returns `Unproven`; the
      compiler prompts "this predicate exceeds the automatic proof range; please provide a proof
      function". The programmer writes a YaoXiang function whose return type equals the
      proposition—the type checker validates the function. No external export, no AI, no interactive
      proof mode. Everything is YaoXiang code; everything is verified by the type checker.
- [x] **Counterexample formatting**: source variable names are used directly as SMT variable names
      (with module prefix to avoid conflicts). Z3 model returned and looked up by variable name.
      Output format: variable name = concrete value + source location + predicate definition
      location. No complex mapping layer.
- [x] ~~**Interaction of compile-time predicates with `ref` smart pointers?**~~ → Decided:
      compile-time predicates only allow values with immutable borrows or whose ownership has been
      transferred. Mutably borrowed values cannot appear in compile-time predicates.
- [x] **Extension of forall predicate violation-count measures to non-adjacent operations?** → Not
      extended. Current coverage (adjacent swap, adjacent move) is complemented by Strategy 1
      (linear rank function)—quicksort's outer interval contraction is covered by Strategy 1,
      heapsort by Strategy 1 (array index pattern). Loops whose termination cannot be proven by any
      strategy are directly reported as compile errors by the compiler—this is the hard-safety
      philosophy, not a defect. If in the future there are real-world scenarios (not academic
      constructions) where none of the four strategies can cover, we will revisit.
- [x] **Linear rank function enumeration combinatorial explosion**: candidate enumeration capped at
      3 bounded variables. When ≤3, enumerate all linear combinations and verify each with SMT.
      When >3, only try single-variable measures (`v_i`, `u_i - v_i`); on failure directly report a
      compile error—prompting the programmer "the loop has >3 bounded variables; the compiler cannot
      automatically synthesize a multi-variable measure". This is not an engineering compromise—it
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
│   Draft     │  ← Author creates
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Reviewing  │  ← Current state: community discussion
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
│ (Official)  │    │ (Remain in place) │
└─────────────┘    └─────────────┘
```
