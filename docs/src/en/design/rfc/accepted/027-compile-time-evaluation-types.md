---
title: 'RFC-027: Compile-Time Predicates and Unified Static Verification'
status: 'Accepted'
author: 'Chen Xu'
created: '2026-06-07'
updated: '2026-07-05'
impl_status: 'in_progress'
impl_detail:
  'Phase 1-2 complete, Phase 3 partially complete, Phase 4 partially complete. All 6 Phases of the
  assert/Assert unified plan implemented (#157-#162 closed): Never type, IsTrue bridge,
  flow-sensitive Γ + kill set, type-level recursion, universe layering weak check, dispatch
  distribution pipeline.'
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
> - [RFC-011: Generic System Design](../accepted/011-generic-type-system.md)
> - [RFC-024: Concurrency Model Based on spawn Blocks](../accepted/024-concurrency-model.md)
>
> **Supersedes**:
> [RFC-022: Hoare Logic Static Verification Support (Specification Annotations and Specification Types)](../deprecated/022-hoare-logic-static-verification.md)
> — Deprecated

## Summary

This document proposes introducing **compile-time predicates** as first-class citizens to YaoXiang,
unifying all compile-time static verification into a single **proof pipeline**. Compile-time
predicates are not bolt-on specification annotations—they are functions. A function returning Type,
usable at type positions, called by the compiler at compile time to check its return value. Types
are propositions, compile-time evaluation is proof.

**Core argument**: The sole job of type checking at compile time is to construct and verify proof
terms. Type equality, token conflicts, dependent type reduction, compile-time predicate evaluation,
Hoare logic implications—all are different type checks in the same compile-time proof pipeline. The
SMT solver is an accelerator module of the type checker, not an independent trust boundary. When the
compiler returns Unproven, the programmer writes a YaoXiang function as proof—the type checker
verifies it in exactly the same way as verifying any function's return type. Everything is YaoXiang
code, everything is verified by the type checker.

## Motivation

### Why Deprecate RFC-022?

RFC-022 designed specifications as `//!` annotation form:

```yaoxiang
max: (T: Ord) -> ((arr: Array(T, n)) -> T) = {
    //! requires: NonEmpty(n) = n > 0          ← This is annotation independent of types
    //! ensures: ExistsMax(result, arr[0..n])   ← This is annotation independent of types
}
```

This commits the fundamental error of the Curry-Howard isomorphism: **splitting specifications and
types into two layers**. Annotations are not types. Annotations don't participate in type checking.
Annotations are the mental model of "external tools."

The white paper makes it clear:

> "No `//!` annotations. No independent specification language. Everything is within the type
> system."

### Current Problems

- RFC-022's `//!` annotations are bolt-on syntax independent of the type system
- Specification types and ordinary types are two systems, causing conceptual redundancy
- The Debug Build verifies / Release Build ignores split pattern breaks unity
- In traditional understanding, the SMT solver is positioned as an external tool—YaoXiang builds it
  in as an accelerator module of the type checker
- Type checking, borrow verification, compile-time predicate checking, and macro expansion each take
  different paths

### The Correct Mental Model

Type checking can be abstracted as a function:

```
verify : Program → Proved | Disproved(Model) | Unproven
```

All compile-time checks—simple type matching, borrow conflict detection, compile-time predicate
verification—are sub-tasks of this function. They share the same proof pipeline; the only difference
is proof term complexity and construction strategy.

When the compiler returns Unproven, the programmer provides a proof function—the function's return
type equals the proposition to be proved. The type checker verifies it. This is the same operation
as ordinary type checking.

## Proposal

### 1. `{}` Is the Proof Space: Types as Assertions, Verification as Type Checking

YaoXiang's `{}` is the compile-time proof space. Everything in it is an assertion, and the compiler
guarantees each is True—either proved automatically, or provided by the programmer as a proof
function.

```
Point: Type = { x: Float, y: Float }
#               ^^^^^^^^^^^^^^^^^^^^^  Compiler guarantees x is Float, y is Float

List: (T: Type) -> Type = { data: Array(T) }
#                           ^^^^^^^^^^^^^^^  Compiler guarantees data is Array(T)
```

**Generics are special cases of compile-time predicates.**

```yaoxiang
Positive: (x: Int) -> Type = { x > 0 }
#          ^^^^^^              ^^^^^^
#          Parameters at       Only assertions in {}
#          signature           Compiler verifies x > 0 at compile-time call

List: (T: Type) -> Type = { data: Array(T) }
#      ^^^^^^^^              ^^^^^^^^^^^^^^^
#      Parameters at         Compiler verifies type_of(T) == Type, type_of(data) == Array(T)
#      signature
```

Same pattern: `name: (params) -> Type = { assertions }`. The compiler does not distinguish between
"type assertions" and "value assertions"—both are evaluation targets in the proof pipeline.

**Loop invariants don't need to be written separately. Type annotations on variables ARE Floyd-Hoare
invariants.**

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # Annotation references i—tells the compiler s's type depends on i
    mut i: UpTo(arr.len) = 0     # At initialization i=0, verify: 0 == sum(arr[0..0]) → True
    while i < arr.len {
        s += arr[i]  # Compiler verifies: s_new == sum(arr[0..i+1])
        i += 1       # i changes → triggers s dependency re-verification: s satisfies SumUpTo(arr, i_new)
    }
    return s  # s: SumUpTo(arr, arr.len) = sum(arr[0..arr.len])
}
```

The compiler generates one verification condition for the loop body—induction hypothesis (type
annotation) → assignment operation → whether new value satisfies the type annotation. After the
proof pipeline verifies the induction step, all iterations are automatically covered. No
`: decreases`, no `: Invariant`, no inductive proof needed—the compiler decomposes induction into
local VCs per assignment.

### 2. Pre/Postconditions: Compile-Time Predicates on Parameter and Return Types

Abandon RFC-022's `//! requires`/`//! ensures`. Compile-time predicates serve as type annotations on
parameters or returns.

**Parameter side is a function call.** Compile-time predicates are functions returning Type; their
use on the parameter side is just calling them—just like `factorial(5)`. The return side introduces
a new concept: the return value parameter.

```yaoxiang
# Precondition: Explicit compile-time predicate call in parameter type
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b
#                       ^^^^^^^^^^  b is current parameter name, passed to Positive as argument
#                       Compiler extracts argument value at call site, substitutes for b, verifies Positive(arg)
#                       Ex: divide(10, 2) → verify Positive(2) = { 2 > 0 } → True
#                       Ex: divide(10, 0) → verify Positive(0) = { 0 > 0 } → False → compile error

# Postcondition: Return value parameter + compile-time predicate
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
  its value provided by the `return` statement. `result` only exists in the type signature, only
  referenced by the predicate, doesn't enter the function body scope, doesn't appear at the caller.
- **Return value parameter is optional**: omitted when there's no postcondition, the signature is
  identical to a normal function (`-> Int`).
- **Uniformity**: parameters and return value parameters are the same
  concept—`param_name: predicate_call(param_name)`, the only difference is whether the value is
  provided by the caller or by `return`.

### 3. Path Condition Propagation: Compile-Time Verification of Runtime Values

When compile-time predicates are used at binding positions, parameters are explicitly passed in by
the programmer. When runtime values enter refined type parameters, the compiler completes
verification through path condition collection and SMT implication judgment—no need for programmers
to explicitly pass proofs.

#### 3.1 Explicit Function Call

When compile-time predicates are used at binding positions, parameters are explicitly passed in by
the programmer—just function calls, zero implicit.

`Positive: (x: Int) -> Type = { x > 0 }` is a compile-time predicate constructor. When it appears at
a binding position (parameter declaration, variable declaration, return type), the programmer
explicitly passes in the already-bound variable name:

```yaoxiang
b: Positive(b)
// b is already declared as the current parameter, Positive(b) is a function call
// After normalization: b: { b > 0 }
```

The compiler doesn't need to implicitly fill in parameters—`b: Positive(b)` is the same as `f(5)`,
just a function call. `b` is bound as a parameter name, and its type annotation `Positive(b)`
references `b` itself—this is the standard pattern of dependent types, not an implicit expansion
rule.

**Unification with RFC-010's `self`**: RFC-010 establishes that `self` is not a keyword, just a
conventional parameter name ("writing `p`, `this`, `x` has exactly the same effect").
`b: Positive(b)` shares the same mechanism—parameter names can be referenced in type annotations.
`self` appears in the position `self: Point`, `b` appears in the position `b: Positive(b)`; both
type annotations reference the parameter itself. The only difference is the complexity of the type
annotation, the mechanism is exactly the same—after name binding, the type can depend on this name.

Return types use explicit function calls the same way:

```yaoxiang
Sorted: (arr: Array(T)) -> Type = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }

sort: (arr: Array(T)) -> (result: Sorted(result)) = { ... }
//                        ^^^^^^^^^^^^^^^^^^^^^^^
//                        result is the return value parameter, Sorted(result) is a function call
//                        Compiler substitutes the return value into result at return point, verifies Sorted(return value)
```

Applies equally to local variable declarations:

```yaoxiang
let x: Positive(x) = 5
// x bound to 5, Positive(5) → { 5 > 0 } → True → pass

// let y: Positive(y) = 0
// y bound to 0, Positive(0) → { 0 > 0 } → False → compile error
```

#### 3.2 Path Condition Collection

When runtime values appear in conditional branches, the compiler automatically collects path
conditions to form the **assumption set** for the current scope. These assumptions participate in
verification as background knowledge for compile-time Bool evaluation.

```yaoxiang
if y > 0 {
    // Compiler automatically obtains assumption in this branch: { y > 0 }
    let result = divide(x, y)
    // Verification condition: (y > 0) ⇒ (y > 0)
    // Proof pipeline determines implication holds → Proved
} else {
// This branch's assumption: { !(y > 0) }
// If divide(x, y) is called, verification condition is !(y > 0) ⇒ y > 0
    // Proof pipeline determines not implied → Disproved
}
```

This is not the compiler hardcoding special patterns—this is the natural behavior of the
compile-time proof pipeline. At each type check call site, the pipeline receives:

```
{background assumptions} ⇒ {verification target}
```

The proof pipeline determines the implication. Proved → pass, Disproved → compile error +
counterexample, Unproven → compile error + unproven proposition. Background assumptions come from
the current program point's path conditions.

#### 3.3 Assumption Stack

When analyzing control flow, the compiler maintains an assumption set for each basic block:

- **if-guard**: `if y > 0` → true branch pushes `y > 0`, false branch pushes `!(y > 0)` (if else is
  used)
- **match pattern**: `if let Some(v) = opt` → push `opt == Some(v)` inside the branch
- **logical conjunction**: `if x > 0 and y < 10` → push `x > 0` and `y < 10` inside the branch
- **function precondition**: when calling `divide(a, b)`, `b` must satisfy `Positive`'s evidence
  either from current assumptions or from the argument's own refined type annotation (if `b` is
  already annotated as `Positive`, its type carries `b > 0`)
- **assignment**: when `let z = y`, existing refined conditions on `y` propagate to `z`

All assumptions enter the compile-time proof pipeline. When entering the SMT acceleration path, they
are translated into SMT-LIB background assertions.

#### 3.4 No Static Evidence Means Compile Error

If the programmer directly writes:

```yaoxiang
divide_user_input: (x: Int, y: Int) -> Int = divide(x, y)
```

There's no assumption of `y > 0` at the current program point, and the argument `y` itself has no
`Positive` type annotation. The verification condition is:

```
{} ⇒ { y > 0 }
```

Pipeline returns `Disproved` (not implied) → compile error:

> Cannot prove that parameter `b` in `divide` call satisfies `Positive`. `y` comes from function
> input, with no proven bound. Consider guarding the call with an if branch:
> `if y > 0 { divide(x, y) }`.

YaoXiang does not accept runtime values directly entering refined type parameters without static
evidence. This is not a restriction—this is the core of the hard security philosophy. Any code the
compiler cannot statically prove must not pass compilation.

#### 3.5 Relationship with the Unified Pipeline

Path condition propagation is not an additional mechanism. It is a direct extension of the
compile-time proof pipeline in control flow analysis:

| Stage                             | Responsibility                                                                                                                                           |
| --------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Path condition collection         | Compiler's control flow analysis phase, annotates assumption set for each basic block                                                                    |
| Verification condition generation | When encountering type constraints to verify, merge path conditions + argument type information                                                          |
| Proof pipeline evaluation         | Compiler kernel → SMT acceleration → yields Proved / Disproved / Unproven                                                                                |
| Result                            | `Proved` → pass; `Disproved` → compile error + counterexample; `Unproven` → compile error + unproven proposition (programmer can provide proof function) |

No new components. No special rules. Path conditions are the proof pipeline's background
knowledge—shared with type equality and borrow constraints on the same pipeline, the same budget
system.

### 4. Compile-Time Proof Pipeline

All compile-time checks share the same pipeline. The pipeline's core operation is **type
checking**—checking whether a proof term's type equals the proposition to be proved. Everything is
type checking.

```
Compile-time encounters Bool expression needing evaluation (i.e., needs to construct a proof term)
        │
        ├── Type equality (T1 == T2)
        │   → Compiler decides directly (structural equivalence)
        │
        ├── Token conflict condition (!conflicting(tokens))
        │   → Flow-sensitive liveness analysis (Dup/Linear property tracking)
        │
        ├── Dependent type reduction (n + m simplification)
        │   → Compile-time term rewriting system (βδι-reduction)
        │
        ├── Compile-time predicate (x > 0, forall...)
        │   → Compiler itself + SMT accelerator
        │
        └── Hoare logic implication (P ⇒ Q)
            → Compiler + SMT accelerator
                    │
                    ▼
             ┌──────────┐
             │ Proved   │  → Compilation passes
             │ Disproved│  → Compile error + counterexample
             │ Unproven │  → Compile error + unproven proposition
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

Compile-time evaluation returns three results—this is a necessary conclusion of the halting problem,
and the natural division of proof theory:

```
eval_compile_time : BoolExpr → Proved | Disproved(Model) | Unproven
```

- **Proved** → Halts, proof term constructed, type check passes. Compilation continues.
- **Disproved(M)** → Halts, counterexample M exists. Compile error + counterexample + source
  location.
- **Unproven** → Within the given resource bound, proof was not constructed. Compile error +
  unproven proposition + budget consumption report.

**Unproven ≠ False.** The compiler saying "I cannot prove it" does not equal the proposition being
false—it's just beyond the capability of current automatic proving. This is honest, not a defect.

The hard budget limit is the engineering solution to the halting problem. No knob—giving one would
be asking the user "do you think your program will halt", the user doesn't know, the compiler
doesn't know.

#### 4.2 After Unproven: The Programmer Writes Proof

When the compiler returns Unproven, the programmer can write a **proof function**—just a YaoXiang
function whose return type equals the proposition to be proved. The type checker verifies this
function—exactly the same mechanism as verifying `add(a, b): Int`.

```
Proposition = Type
Proof       = Program (a value of that type)
Verification= Type check (the sole trust root)
```

The SMT solver is not an independent trust boundary—it is an **accelerator module of the type
checker**. SMT helps find proofs, but the one that verifies proofs is always the type checker. When
SMT returns `unsat`, the compiler reconstructs its result as a proof term verifiable by the type
checker. If reconstruction fails (SMT's inference steps exceed the compiler kernel's inference
rules), it falls back to Unproven—the programmer can manually write a proof function.

```yaoxiang
# Proposition: refinement property the compiler cannot automatically prove
FirstIsMin: (T: Ord, arr: Sorted(T)) -> Type = {
    forall i in 0..arr.len: arr[0] <= arr[i]
}

# Proof: programmer writes a function, return type is the proposition above
# Type checker verifies this function—exactly the same as verifying add(a,b): Int
first_is_min: (T: Ord, arr: Sorted(T)) -> FirstIsMin(T, arr) = {
    # Compiler verifies here: function body's type = FirstIsMin(T, arr)
    ...
}
```

No AI needed, no export to Coq, no new concept. **Properties the compiler cannot automatically prove
at compile time → programmer uses YaoXiang code to write proof → type checker verifies.** The whole
process is a smooth gradient—the compiler does the simple proofs for you, leaving the brain for the
hard ones.

#### 4.3 Layered Dependencies Within the Pipeline

The above evaluators share the same interface but have an evaluation order. Type equality is a
prerequisite for all subsequent analyses; ownership/token check depends on type information;
refinement predicate verification depends on the results of the first two layers. The compiler
evaluates layer by layer; failed expressions in lower layers don't enter upper layers—avoiding
wasting solver budget on type-incorrect programs.

```
Evaluation order (same pipeline, layered scheduling)
├── Layer 0: Type equality (T1 == T2)
│   └── Structural unification → if fails, subsequent is meaningless, directly return Disproved
├── Layer 1: Ownership/token conflict
│   └── Flow-sensitive liveness analysis → if fails, memory safety doesn't hold, directly return Disproved
└── Layer 2: Refinement predicate/Hoare implication
    └── Compiler itself → SMT acceleration → yields Proved / Disproved / Unproven
```

Each layer still returns `Proved/Disproved/Unproven`, sharing the same interface and the same budget
system.

### 5. Three-Layer Function Unification

| Layer                  | Runtime      | Input        | Output | Example                                        |
| ---------------------- | ------------ | ------------ | ------ | ---------------------------------------------- |
| Value-level function   | Runtime      | Values       | Values | `add: (a: Int, b: Int) -> Int = a + b`         |
| Type constructor       | Compile-time | Types/Values | Type   | `List: (T: Type) -> Type = { data: Array(T) }` |
| Compile-time predicate | Compile-time | Values       | Type   | `Positive: (x: Int) -> Type = { x > 0 }`       |

All use the same `name: type = value` syntax. Compile-time predicates and type constructors go
through the same compile-time proof pipeline—`{}` is the proof space.

### 6. Loops: Floyd-Hoare Verification Condition Generation

Loops don't need separate `: Invariant(...)` or `: decreases(...)` annotations. Compile-time
predicate type annotations on variables define Floyd-Hoare style assertions—the compiler generates
verification conditions from type annotations, the proof pipeline checks whether each assignment
maintains the type.

Core mechanism: each assignment operation corresponds to a Hoare triple `{P} x := e {Q}`, the
verification condition is `P ⇒ Q[e/x]`. The compiler generates one verification condition for the
loop body—after the proof pipeline verifies the induction step holds, all iterations are
automatically covered.

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # Annotation references i; at initialization i=0, verify: 0 == sum(arr[0..0]) → True
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
        #   Compiler + SMT: linear arithmetic, milliseconds → Proved
        #
        # i += 1:
        #   i changes → s's type annotation in dependency graph references i → trigger re-verification
        #   New verification target: s satisfies SumUpTo(arr, i_new)
        #   i.e., s == sum(arr[0..i_new]), guaranteed by previous step → Proved
        s += arr[i]
        i += 1
    }
    return s  # At this point s: SumUpTo(arr, arr.len), i.e., s == sum(arr[0..arr.len])
}
```

Loop invariants are type annotations on variables—programmer writes the type, compiler checks the
induction step. The compiler doesn't need to "discover" invariants, nor "automatically do
induction"—it decomposes inductive proofs into local verification conditions for each assignment
operation, handed to the proof pipeline divide-and-conquer.

#### 6.1 Dependency Tracking: Dependent Types on Mutable Variables

The premise of the above mechanism is: the compiler knows `s`'s type annotation `SumUpTo(arr, i)`
references `i`—when `i` changes, `s`'s type constraint also changes. This requires the compiler to
maintain a **type dependency graph between variables**.

**Data structure**:

```
TypeDepGraph: Map<VarName, Set<VarName>>
# Key is the depended variable, value is the set of variables that reference this variable in type annotations
# Ex: { i: {s}, j: {s, t}, ... }
```

**Construction**: When the type checker processes `mut v: Pred(... x ...) = init`, it parses free
variable references in `Pred(...)`'s parameters. If the parameters reference other mutable variables
`x` in the current scope, record `x → v` in the dependency graph.

**Trigger**: When the depended variable `x` is assigned, the compiler:

1. Find all variables in the dependency graph that depend on `x` `{v₁, v₂, ...}`
2. For each `v`, generate verification condition:
   `whether v's current value satisfies the updated type Pred(... x_new ...)`
3. Send the VC to the proof pipeline

**Assignment order sensitive**: Dependency tracking naturally enforces the correct assignment order.
Taking `SumUpTo(arr, i)` as example:

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

**Composed dependencies**: a variable can depend on multiple variables. Type annotation
`{ v: Int; v == x + y }` depends on both `x` and `y`—any change triggers re-verification.

**Relationship with the proof pipeline**: dependency tracking is the trigger for VC generation, not
an independent verification mechanism. It answers "when to generate VCs"—the proof pipeline answers
"whether the VCs hold."

### 7. Termination Check

Fully automatic at compile time. Loops the compiler can prove pass; those it cannot prove directly
report compile error—the programmer must allow the compiler to automatically analyze loop
termination. No half-automatic annotation escape hatch.

#### 6.1 Design Principles

The compiler automatically extracts the information needed for termination proof from two places:

1. **Variable type annotations**: boundary constraints in refinement types (e.g., `UpTo(n)` gives
   upper bound `n` and lower bound `0`)
2. **Loop body operations**: operations applied to variables per iteration

The compiler tries four metric synthesis strategies in priority order, stopping as soon as one is
found.

#### 6.2 Strategy 1: Linear Rank Function Auto-Synthesis

When variables have linear bound annotations, the compiler enumerates candidate linear metrics and
verifies with SMT.

```
Input:
  Variables v₁: UpTo(u₁), v₂: UpTo(u₂), ... (variables with upper and lower bounds)
  Loop condition cond
  Set of assignments in loop body

Algorithm:
  1. Extract each variable's bounds from type annotations: [low_i, high_i]
  2. Enumerate candidate metrics: v_i, u_i - v_i, v_i - v_j, etc. linear combinations
  3. For each candidate metric m:
     - SMT verify m ≥ 0 (derived from type bounds)
     - For each execution path in loop body, SMT verify m' < m (strictly decreasing)
  4. Find a linear combination meeting the condition → termination proved
```

Coverage: any variable assigned to a linear expression (`v = a·v + b`) in a loop with bound type
annotations. Including `i += const`, `i -= const`, and binary search style interval contraction:

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

#### 6.3 Strategy 2: Predicate Violation Count—Automatically Extract Metric from Target Type <span style="color:orange">[Experimental Strategy]</span>

> ⚠️ **Current status: experimental strategy, included in Phase 3 implementation based on actual
> feasibility.** This strategy is effective for adjacent swap operations (bubble sort, insertion
> sort), but cannot automatically prove non-adjacent operations (quicksort partition, heap sort
> sift-down). Coverage boundary shown in the table below. If Phase 3 verification is infeasible,
> this strategy will be removed or downgraded to future work.

Core insight: **user-written specifications are the material for compiler reasoning.** The compiler
doesn't need to bake in "what is sorting"—it reads the definition of `Sorted`, automatically
extracts metrics from the definition.

```
Input:
  Target type: Sorted(arr) = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }
  Loop body operation: adjacent element swap

Algorithm:
  1. Parse predicate definition: forall i in range: cond(i, arr)
  2. Auto-generate metric: violation_count = |{ i | ¬cond(i, arr) }|
  3. Analyze impact of operations on metric:
     - Adjacent swap arr[j], arr[j+1] = arr[j+1], arr[j]
     - Only affects three pairs of indices j-1, j, j+1
     - If arr[j] > arr[j+1] (predicate violation), this pair satisfies predicate after swap
     - violation_count decreases by at least 1
  4. Upper bound: n·(n-1)/2 (maximum number of adjacent inversions), lower bound: 0
  → Termination proved
```

**Current coverage**:

| Algorithm      | Operation Pattern   | Strategy 2 Proves? | Reason                                                  |
| -------------- | ------------------- | :----------------: | ------------------------------------------------------- |
| Bubble Sort    | Adjacent Swap       |         ✅         | violation_count strictly decreases per swap             |
| Insertion Sort | Adjacent Move       |         ✅         | Each shift eliminates one violation pair                |
| Selection Sort | Non-adjacent Swap   |         ❌         | A single swap may increase violation_count              |
| Quicksort      | Partition Splitting |         ❌         | Non-adjacent swap, doesn't guarantee monotonic decrease |
| Heap Sort      | sift-down           |         ❌         | Tree operation, violation_count non-monotonic           |

**Complementary strategy**: For quicksort, the `low < high` interval contraction is covered by
Strategy 1 (linear rank function)—the outer partition recursion halves the interval each time.
Strategy 1 and Strategy 2 complement each other in coverage; most practical algorithm termination
can be proved by one of them. But generalizing Strategy 2 (non-adjacent operations, tree operations)
is still an open problem.

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

`v += const` (positive constant), variable has upper bound type annotation → metric
`upper_bound - v` decreases by `const` each time, lower bound 0. This is a degenerate case of
Strategy 1, handled quickly by the compiler at the very front.

#### 6.5 Strategy 4: Multiplicative Scaling Metric Template

`v *= const` (const > 1), variable has upper and lower bound type annotations. Compiler built-in
logarithmic metric template `ceil(log_const(upper/v))`, each multiplication by const reduces metric
by 1.

```yaoxiang
mut i: Positive(i) = 1
while i < n {
    # Compiler automatically derives: metric ceil(log₂(n/i)), each multiplication by 2 reduces metric by 1
    i *= 2
}
```

#### 6.6 Separation of Termination and Correctness

Termination proof and correctness proof are independent:

- **Termination**: the four strategies above automatically prove the loop exits in finite steps
- **Correctness**: whether the loop body progresses toward the target type, checked by the
  compile-time proof pipeline through verification conditions

Both pass → compilation passes. Termination proved but correctness fails → compile error +
counterexample. Correctness proved but termination cannot be proved → compile error and point out
the variable or operation that cannot be analyzed. Both fail → compile error reports the two failure
reasons separately.

#### 6.7 Termination Check for Recursive Functions

For recursive functions that need to be evaluated at compile time, the compiler checks parameter
decrease:

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)  # Compiler analyzes: n-1 < n → decreasing → terminates
}

# Compile-time use—compiler guarantees factorial terminates at compile time
vec: Vec(factorial(5)) = Vec(120)()  # 5! = 120, completed at compile time
```

| Scenario                                              | Behavior                    |
| ----------------------------------------------------- | --------------------------- |
| Compiler can analyze recursive decrease (e.g., `n-1`) | Compile-time evaluation     |
| Doesn't decrease / cannot determine decrease          | Compile error               |
| Runtime call (not in type position)                   | No termination check needed |

#### 6.8 Hard Boundary

`i = f(i)` where `f` is irreversible, non-closed, doesn't preserve any monotonicity—mathematically
impossible to automatically prove termination. Compile error:

> This loop cannot be automatically proved to terminate. The loop variable depends on the
> unanalyzable function `f`. Please use iteration patterns that can be analyzed by the compiler.

This is not a failure of the compiler. Any code that cannot be statically proven safe must not pass
compilation.

### 8. SMT Solver: Accelerator Module of the Type Checker

In traditional languages, the SMT solver is an external tool (e.g., F\* calls Z3, Dafny calls Z3).
In YaoXiang, it is an **accelerator module of the type checker**—only invoked when the compiler
kernel itself cannot directly decide. SMT helps find proofs, but the one that verifies proofs is the
type checker.

**Trust model**: The type checker is the sole trust root. The SMT solver is an accelerator module—it
helps find proofs, but SMT is not an independent trust boundary. The compiler trusts Z3's `unsat`
result (consistent with the F\*/Dafny line—the probability of Z3 error is lower than the compiler's
own bug rate, a pragmatic engineering choice). The real unreliability is controlled at the SMT
translation layer—if the translation has bugs, the compiler will expose them in other tests.

**Interface**: The compiler internally translates to SMT-LIB 2.6 standard format, rather than
binding to a specific solver API. SMT-LIB is an ISO standard; Z3, CVC5, MathSAT, Yices all natively
support it.

**Default backend**: Z3 (MIT license, most extensive documentation and community validation). CVC5
as SMT-LIB compatible alternative—users can switch via compiler flag at compile time.

No "generic solver abstraction layer"—SMT-LIB is the abstraction layer. In the future if CVC5 has
breakthroughs in specific theories, switching just requires changing the binary, no need to change
compiler code.

```
Compile-time Bool expression
        │
        ├── Compiler kernel can directly decide (structural equivalence, simple arithmetic,
        │   trivial formulas after constant folding)
        │   → Directly return Proved / Disproved
        │
        └── Compiler kernel cannot directly decide (quantifiers, symbolic variables)
            → Dependent type pre-reduction (factorial(5) → 120)
            → Translate to SMT-LIB format
            → Send to Z3/CVC5 (with budget limit)
            → Return value: unsat → Proved  │  sat + model → Disproved  │  unknown → Unproven
```

**Solver budget—hard limit, like stack depth**:

| Budget Dimension               | Default Value | Description                                                                                                                                         |
| ------------------------------ | ------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| Solver steps                   | 10,000        | Z3 usually within a hundred steps for linear arithmetic. 10,000 steps cover 99% of practical predicates.                                            |
| Time                           | 100ms         | Single predicate over 100ms = user is writing a compile-time program, not a type annotation. 100ms × 50 predicates = 5 second compilation time cap. |
| Quantifier instantiation depth | 3             | Three levels of nested quantifiers cover practical patterns. Beyond three levels is most likely a logic exercise.                                   |

Over budget returns Unproven, compile error + predicate location + consumption. No degradation, no
runtime check, no silent pass.

**Why this is actually feasible**: 95% of practical predicates in engineering are linear
arithmetic—`x > 0`, `arr.len > 0`, `0 <= idx < arr.len`—all within decidable fragments; SMT solver
returns in milliseconds for these problems. For the rare complex predicate that exceeds the budget,
the programmer can just write a proof function.

Dependent types do a pre-reduction before SMT calls: `factorial(5)` directly evaluates at compile
time to `120`, `append([1,2], [3])` directly evaluates to `[1,2,3]`. These deterministic value
calculations don't consume SMT budget.

Programmers don't need to know SMT exists. Mental model is: **the compiler can prove it, pass;
can't, error—if the compiler can't, you can write a function to prove it.**

### 9. Compile-Time Predicate Composition

Compile-time predicates are functions returning Type; composition is naturally implemented through
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

### 11. Dispatch Distribution Pipeline: Unified Compile-Time and Runtime Distribution

`assert` and `Assert` are two sides of the same refinement type primitive. The distribution pipeline
`dispatch` automatically decides whether to take compile-time proof or runtime check based on
**whether the predicate's free variables are reachable at compile time**:

| Criterion                                                                             | Mode            | Behavior                                                                                   |
| ------------------------------------------------------------------------------------- | --------------- | ------------------------------------------------------------------------------------------ |
| All free variables known at compile time (generic parameters, compile-time constants) | **CompileTime** | Enter proof pipeline: Proved → erase, Disproved → compile error, Unknown → require proof   |
| Free variable comes from runtime (function parameters, external input, mut variable)  | **Runtime**     | Insert runtime check, and inject refinement facts into the flow-sensitive assumption set Γ |

**Key**: "Cannot determine" ≠ "Disproved". Unknown in CompileTime mode requires proof (no silent
degradation); in Runtime mode, the proposition has no truth value at compile time—no matter how
strong the prover, it cannot write a perpetually true proof for "the user may have input a negative
number"; runtime check is the only sound choice. This is not the prover being too weak, it's a
theoretical necessity.

### 12. Flow-Sensitive Assumption Set Γ: Strongest Postcondition Propagation

The compiler maintains a flow-sensitive assumption set Γ, tracking which propositions are known to
hold at each control flow point.

**SP (strongest postcondition) propagation**:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
```

**Kill set of mut variable**: after a `mut` variable is reassigned, all assumptions involving that
variable are removed from Γ:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
mut x = x - 5       // Γ = {}  ← x > 0 is killed
```

This is a hard requirement of soundness—the variable value changed, old assumptions are invalid.

**Branch confluence**: when IF/ELSE or match branches merge, Γ takes the intersection of each
branch's assumptions. Only propositions that hold on all paths are carried out of the branch.

### 13. Erasure Model Clarification: Witness Erasure ≠ Check Erasure

The "refinement types are completely erased at runtime" claim in RFC-027 refers to **proof witness
(proof token)**—already verified proof terms at compile time don't produce runtime code. But runtime
checks inserted by dispatch in Runtime mode are preserved—they are Bool checks executed at the value
level, not type-level witnesses.

Summary: witness erasure, check retention. The two don't conflict, the original RFC-027 claim
remains unchanged.

## Detailed Design

### Syntax Changes

| Before (RFC-022)                      | After (This RFC)                                                              |
| ------------------------------------- | ----------------------------------------------------------------------------- |
| `//! requires: NonEmpty(n) = n > 0`   | Compile-time predicate as parameter type `(b: Positive(b))`                   |
| `//! ensures: ExistsMax(result, arr)` | Return type using return value parameter `-> (result: IsMax(T, arr, result))` |
| `/*! invariant: ... !*/`              | Compile-time predicate type annotation on variables—Floyd-Hoare invariant     |
| `//! decreases: n`                    | Compiler fully auto-derives metric function                                   |
| Specifications are annotations        | Specifications are the type system                                            |

### Syntax

**Compile-time predicates have no new keywords.** `{}` is the proof space, identical to existing
type definition syntax. Compile-time predicates are functions returning
Type—`name: (params) -> Type = { assertions }`. Usage is just function calls—`Positive(b)`,
`IsMax(T, arr, result)`.

```bnf
# 编译期谓词 = 返回 Type 的函数，{} 内为编译器验证的断言
# 使用现有函数/类型语法，无需新增 BNF 规则
predicate ::= identifier ':' params '->' 'Type' '=' '{' assertions '}'
```

**Arguments to predicate applications must be in compile-time constant form**—literals, variables
(bound by name), or single-parameter type applications (recursive extraction). When the argument
form cannot be converted to a constant expression, or the argument count doesn't match the predicate
parameters, report an **E1092** compile error—refinement constraints are **never silently
discarded** (#263: previously, non-convertible arguments would cause constraints to silently
disappear, with constraint-violating bindings silently passing).

**New syntax concept: return value parameter**—in `-> (name: Type)`, `name` is the return value
parameter.

The return value parameter is the **only syntax concept** introduced by YaoXiang in the existing
function syntax. Its semantics:

- `name`'s value is provided by the `return` statement
- `name` only exists in the type signature, referenced by postcondition predicates (e.g.,
  `-> (result: IsMax(T, arr, result))`)
- `name` doesn't enter the function body scope, doesn't appear at the caller
- **Return value parameter is optional**—without postcondition the signature is identical to a
  normal function (`-> Int`), introducing no additional burden

Reason for introducing it: postconditions need to reference "the value the function will return".
Without a return value parameter, the compiler can only let predicates reference the return value
through special rules (like implicit variables `$result` or `__retval__`). The return value
parameter makes this reference explicit—it's just a parameter, just the value is provided by
`return` rather than the caller.

**Proof functions are not a new concept**—they are just YaoXiang functions whose return type is the
asserted proposition. When the compiler returns Unproven, the programmer provides a proof function;
the type checker verifies it in exactly the same way as verifying any function's return type. No new
syntax, keywords, or rules needed.

### Type System Impact

- **Type universe**: compile-time predicates are at the Type₂ level—functions that accept values and
  return Type, same level as type constructors
- **Generic interaction**: compile-time predicates can take generic parameters, e.g.,
  `NonEmpty: (T: Type) -> (arr: Array(T)) -> Type`
- **Ownership interaction**: expressions in compile-time predicates follow ownership rules, can only
  read, not write
- **Type inference**: compile-time predicate parameters participate in HM type inference

### Runtime Representation

Compile-time predicates are processed at runtime according to dispatch distribution results:

- **CompileTime mode** (all free variables known at compile time): after proof passes, the witness
  token is completely erased. `Positive: (x: Int) -> Type = { x > 0 }`—parameter `b: Positive(5)`'s
  runtime representation is just `Int`. The refinement condition `{ 5 > 0 }` has passed, erased.
- **Runtime mode** (runtime free variables exist): preserve runtime check—execute Bool check at the
  value level, inject into the flow-sensitive assumption set Γ. See §11 dispatch distribution
  pipeline and §13 erasure model clarification for details.

Putting compile-time predicates in type position (e.g., `f(x: Positive(x))`) doesn't create wrapper
types or allocate extra memory. But when `x` comes from runtime input, **runtime Bool check will be
inserted**.

**Interaction constraints with `ref`**: compile-time predicates can only reference immutably
borrowed or ownership-transferred values. Compile-time predicates referencing mutably borrowed
values—where the compiler cannot guarantee at compile time that verification results still hold at
runtime—directly report a compile error.

### Compiler Changes

1. **Parser**: compile-time predicates use standard function syntax, no additional parsing rules
   needed
2. **Compile-time proof pipeline**: unified Proved/Disproved/Unproven return interface, automatic
   strategy selection
3. **SMT accelerator module**: SMT-LIB 2.6 translation layer, default backend Z3, CVC5 alternative
4. **Type checker kernel**: inference rule implementation—structural equivalence, βδι-reduction,
   universal quantifier introduction/elimination. This is the sole trust root; SMT and programmer
   proofs are both verified through this
5. **Verification condition generation**: WP/SP calculus + loop invariant proof obligations
6. **Error reporting**: counterexample formatting + unproven proposition report + source location
   association

### Backward Compatibility

- ✅ Code that doesn't use compile-time predicates is completely unchanged
- ✅ Compile-time predicates have zero runtime overhead in CompileTime mode; in Runtime mode only
  retain necessary Bool checks
- ⚠️ RFC-022's `//!` syntax is no longer supported—but 022 was never implemented, no migration
  burden

## Tradeoffs

### Advantages

- **Curry-Howard isomorphism fully realized**: types as propositions, programs as proofs,
  `name: Proposition = Proof`
- **Unity**: compile-time predicates and regular functions use exactly the same syntax, no
  conceptual split
- **SMT transparent**: programmers don't need to know SMT exists, mental model consistent with type
  checking
- **Progressive adoption**: can start from one compile-time predicate, gradually increase coverage
- **Minimal runtime overhead**: zero overhead in CompileTime mode, only retain necessary Bool checks
  in Runtime mode

### Disadvantages

- **Compilation time**: SMT solving increases compilation time, but hard budget limit ensures the
  cap is controllable
- **Automatic proof boundary**: complex predicates beyond first-order linear arithmetic may require
  programmers to write proof functions. This is not a language defect—this is a necessary conclusion
  of the halting problem. The compiler honestly reports Unproven rather than falsely reporting
  True/False
- **Learning curve**: writing effective compile-time predicates and proof functions requires
  understanding the basic intuition of the Curry-Howard isomorphism
- **Implementation complexity**: unifying the compile-time proof pipeline requires careful design

### Risk Mitigation

- SMT solver budget hard limit (steps 10,000 / time 100ms / instantiation depth 3), over budget
  returns Unproven
- Dependent type pre-reduction: deterministic value calculations eat first, SMT only gnaws on
  non-deterministic parts
- Unproven is not a dead end: programmer can write a proof function, type checker
  verifies—consistent with verifying any function return type
- Incremental verification: only verify changed modules
- Clear error messages + counterexample display + budget consumption report + unproven proposition +
  suggestions (if compiler can give them)

## Alternatives

| Plan                                                           | Why Not Choose                                                                                                                                                     |
| -------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| RFC-022: `//!` annotation-style specification                  | Specifications split from types, violates Curry-Howard isomorphism                                                                                                 |
| Independent specification file (e.g., CVL)                     | Specifications separated from code, increases maintenance cost                                                                                                     |
| Runtime-only assertions                                        | Cannot statically guarantee correctness                                                                                                                            |
| External proof assistant (e.g., Coq)                           | Disconnected from compiler, requires independent proof language and trust boundary. YaoXiang's choice: proof is YaoXiang code, type checker is the sole trust root |
| **This plan: compile-time predicates as first-class citizens** | ✅                                                                                                                                                                 |

## Implementation Strategy

### Phase Division

| Phase       | Content                                                                                                                                                                |
| ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Phase 1** | Compiler kernel: structural equivalence + βδι-reduction + universal quantifier introduction/elimination. Support simple arithmetic predicates (`x > 0`, `arr.len > 0`) |
| **Phase 2** | SMT-LIB translation layer + Z3/CVC5 integration. Pipeline returns Proved/Disproved/Unproven. Support programmer writing proof functions when Unproven                  |
| **Phase 3** | Loop invariant VC generation + termination check (linear rank function + predicate violation counting + bounded pattern + combinatorial explosion control)             |
| **Phase 4** | Incremental verification + caching + IDE support                                                                                                                       |

### Dependencies

- RFC-010: Unified Type Syntax — compile-time predicates based on `name: type = value`
- RFC-011: Generic System — compile-time predicates can take generic parameters
- RFC-009: Ownership Model — expressions in compile-time predicates follow ownership rules

## Open Questions

- [x] **SMT solver selection**: default Z3 (MIT license, most widely verified). CVC5 as SMT-LIB
      compatible alternative, switch via compiler flag. Compiler internal translation target is
      SMT-LIB 2.6 standard format—SMT-LIB is the abstraction layer, no custom generic solver
      interface.
- [x] **Specific values of solver budget**: steps 10,000 / time 100ms / quantifier instantiation
      depth 3. Fixed in the compiler, no knob. In actual use, if there are real use cases proving
      insufficient (not "user wrote it wrong"), then adjust.
- [x] **Quantifier support range**: no limit on quantifier order at the language level. Compile-time
      predicates take Type parameters—Type includes function types—therefore higher-order
      quantifiers are natural implications of the type system, no special syntax needed. SMT solver
      can automatically decide first-order quantifiers (forall/exists, supports interleaved nesting,
      limited by budget depth 3). Higher-order quantifiers: SMT returns Unproven, compiler prompts
      "this predicate is beyond the range of automatic proving, please provide a proof function".
      Programmer writes a YaoXiang function whose return type equals the proposition—type checker
      verifies the function. No external export, no AI, no interactive proof mode. Everything is
      YaoXiang code, everything verified by the type checker.
- [x] **Counterexample formatting**: source variable names used directly as SMT variable names (with
      module prefix to avoid conflicts). When Z3 model returns, lookup by variable name. Output
      format: variable name = specific value + source location + predicate definition location. No
      complex mapping layer.
- [x] ~~**Interaction of compile-time predicates with `ref` smart pointers?**~~ → Decided:
      compile-time predicates only allow immutably borrowed or ownership-transferred values. Mutably
      borrowed values cannot appear in compile-time predicates.
- [x] **Extension of forall predicate violation count metric to non-adjacent operations?** → No
      extension. Current coverage (adjacent swap, adjacent move) is complementarily covered by
      Strategy 1 (linear rank function)—quicksort outer interval contraction is covered by Strategy
      1, heap sort is covered by Strategy 1 (array index pattern). Loops that cannot have
      termination proved by any strategy, the compiler directly errors—this is hard security
      philosophy, not a defect. If in the future there are real scenarios (not academic
      constructions) where algorithms cannot be covered by all four strategies, then discuss again.
- [x] **Linear rank function enumeration combinatorial explosion**: candidate enumeration limit is 3
      bounded variables. ≤3 enumerate all linear combinations and verify one by one with SMT. >3
      only try single variable metrics (`v_i`, `u_i - v_i`), fail directly report compile error—hint
      programmer "loop has >3 bounded variables, compiler cannot automatically synthesize
      multi-variable metrics". This is not an engineering compromise—it forces programmers to write
      simpler loops.

## References

- [RFC-010: Unified Type Syntax](../accepted/010-unified-type-syntax.md)
- [RFC-011: Generic System Design](../accepted/011-generic-type-system.md)
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
│ Under Review│  ← Current status: community discussion
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
│(Formal      │    │ (Preserved  │
│ Design)     │    │  in place)  │
└─────────────┘    └─────────────┘
```
