---
title: 'RFC-027: Compile-time Predicates and Unified Static Verification'
status: 'Accepted'
author: 'Chen Xu'
created: '2026-06-07'
updated: '2026-07-05'
impl_status: 'in_progress'
impl_detail:
  'Phase 1-2 completed, Phase 3 partially completed, Phase 4 partially completed. The unified
  assert/Assert 6-Phase plan is fully implemented (#157-#162 closed): Never type, IsTrue bridging,
  flow-sensitive Γ + kill set, type-level recursion, universe stratification weak check, and
  dispatch pipeline.'
impl_percent: 85
issue_number: 90
issue_url: 'https://github.com/ChenXu233/YaoXiang/issues/90'

issue: '#90'
---

# RFC-027: Compile-time Predicates and Unified Static Verification

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

## Summary

This RFC proposes introducing **compile-time predicates** as first-class citizens in YaoXiang,
unifying all compile-time static verification into a single **proof pipeline**. Compile-time
predicates are not external specification annotations—they are functions. A function that returns a
Type can be used at type positions, and the compiler invokes it at compile time and checks the
return value. Types are propositions; compile-time evaluation is proof.

**Core argument**: The only work of type checking at compile time is to construct and verify proof
terms. Type equality, token conflicts, dependent type reduction, compile-time predicate evaluation,
Hoare logic implication—all are different type checks within the compile-time proof pipeline,
sharing the same pipeline. The SMT solver is an acceleration module of the type checker, not an
independent trust boundary. When the compiler returns Unproven, the programmer writes a YaoXiang
function as a proof—the type checker verifies it in exactly the same way it verifies any function's
return type. Everything is YaoXiang code; everything is verified by the type checker.

## Motivation

### Why Deprecate RFC-022?

RFC-022 designed specifications as `//!` annotation form:

```yaoxiang
max: (T: Ord) -> ((arr: Array(T, n)) -> T) = {
    //! requires: NonEmpty(n) = n > 0          ← This is an annotation independent of types
    //! ensures: ExistsMax(result, arr[0..n])   ← This is an annotation independent of types
}
```

This commits the fundamental error of the Curry-Howard isomorphism: **splitting specifications and
types into two layers**. Annotations are not types. Annotations do not participate in type checking.
Annotations belong to the "external tool" mental model.

The white paper is clear:

> "There are no `//!` annotations. There is no separate specification language. Everything is within
> the type system."

### Current Problems

- RFC-022's `//!` annotations are external syntax independent of the type system
- Specification types and ordinary types are two separate systems, causing conceptual redundancy
- The split mode of "Debug Build verifies / Release Build ignores" breaks unity
- The SMT solver is conventionally positioned as an external tool—YaoXiang embeds it as an
  acceleration module of the type checker
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

When the compiler returns Unproven, the programmer provides a proof function—whose return type
equals the proposition to be proven. The type checker verifies it. This is the same operation as
ordinary type checking.

## Proposal

### 1. `{}` Is Proof Space: Types Are Assertions, Verification Is Type Checking

YaoXiang's `{}` is the compile-time proof space. Everything within is an assertion, and the compiler
guarantees each item is True—either proven automatically or by a proof function provided by the
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
#          parameters in signature  only assertions in {}
#          compiler verifies x > 0 at call site

List: (T: Type) -> Type = { data: Array(T) }
#      ^^^^^^^^              ^^^^^^^^^^^^^^^
#      parameters in signature   compiler verifies type_of(T) == Type, type_of(data) == Array(T)
```

The same pattern: `name: (params) -> Type = { assertion }`. The compiler does not distinguish "type
assertions" from "value assertions"—both are evaluation targets in the proof pipeline.

**Loop invariants do not need to be written separately. Type annotations on variables are
Floyd-Hoare invariants.**

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # annotation references i—tells compiler s's type depends on i
    mut i: UpTo(arr.len) = 0     # at initialization i=0, verify: 0 == sum(arr[0..0]) → True
    while i < arr.len {
        s += arr[i]  # compiler verifies: s_new == sum(arr[0..i+1])
        i += 1       # i changes → triggers s dependency re-verification: s satisfies SumUpTo(arr, i_new)
    }
    return s  # s: SumUpTo(arr, arr.len) = sum(arr[0..arr.len])
}
```

The compiler generates one verification condition for the loop body—inductive hypothesis (type
annotation) → assignment operation → whether the new value satisfies the type annotation. After the
proof pipeline verifies the inductive step, all iterations are covered automatically. No
`: decreases`, no `: Invariant`, no inductive proof needed—the compiler decomposes induction into
local VCs for each assignment.

### 2. Pre/Postconditions: Compile-time Predicates on Parameter and Return Types

Abandon RFC-022's `//! requires`/`//! ensures`. Compile-time predicates are used as parameter or
return type annotations.

**Parameter side is a function call.** A compile-time predicate is a function that returns Type, and
the parameter-side usage is calling it—just like `factorial(5)`. The return side introduces a new
concept: return value parameter.

```yaoxiang
# Precondition: explicitly invoke compile-time predicate in parameter type
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b
#                       ^^^^^^^^^^  b is current parameter name, passed to Positive as argument
#                       compiler extracts actual value at call site, substitutes into b, verifies Positive(actual)
#                       e.g.: divide(10, 2) → verify Positive(2) = { 2 > 0 } → True
#                       e.g.: divide(10, 0) → verify Positive(0) = { 0 > 0 } → False → compile error

# Postcondition: return value parameter + compile-time predicate
IsMax: (T: Ord, arr: Array(T), result: T) -> Type = {
    forall j in 0..arr.len: result >= arr[j]
}

NonEmpty: (arr: Array(T)) -> Type = { arr.len > 0 }

max: (T: Ord) -> ((arr: NonEmpty(arr))) -> (result: IsMax(T, arr, result)) = {
#                                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
#                                            result is return value parameter, value provided by return
#                                            compiler substitutes return value at return point, verifies postcondition
    candidate = arr[0]
    for i in 1..arr.len {
        if arr[i] > candidate { candidate = arr[i] }
    }
    return candidate
}
```

**Key rules**:

- **Parameter side**: `b: Positive(b)` — `b` is the current parameter name, passed to `Positive` as
  argument. Function call syntax, zero implicits.
- **Return side**: `-> (result: IsMax(T, arr, result))` — `result` is a return value parameter,
  value provided by the `return` statement. `result` exists only in the type signature, is only
  referenced by predicates, and does not enter the function body scope or appear at the call site.
- **Return value parameter is optional**: not written without postconditions, signature is identical
  to ordinary functions (`-> Int`).
- **Unity**: parameters and return value parameters are the same
  concept—`param_name: predicate_call(param_name)`, differing only in whether the value is provided
  by the caller or by `return`.

### 3. Path Condition Propagation: Compile-time Verification of Runtime Values

When compile-time predicates are used at binding positions, parameters are explicitly passed by the
programmer. When runtime values enter refinement type parameters, the compiler completes
verification through path condition collection and SMT implication judgment—no need for the
programmer to explicitly pass proofs.

#### 3.1 Explicit Function Call

When compile-time predicates are used at binding positions, parameters are explicitly passed by the
programmer—just function calls, zero implicits.

`Positive: (x: Int) -> Type = { x > 0 }` is a compile-time predicate constructor. When it appears at
binding positions (parameter declarations, variable declarations, return types), the programmer
explicitly passes already-bound variable names:

```yaoxiang
b: Positive(b)
// b is already declared as current parameter, Positive(b) is a function call
// after strictification: b: { b > 0 }
```

The compiler does not need to implicitly fill in parameters—`b: Positive(b)` is the same as `f(5)`,
just a function call. `b` is bound as a parameter name, and its type annotation `Positive(b)`
references `b` itself—this is the standard dependent type pattern, not an implicit expansion rule.

**Unified with RFC-010's `self`**: RFC-010 establishes that `self` is not a keyword, just a
conventional parameter name ("writing `p`, `this`, or `x` has exactly the same effect").
`b: Positive(b)` shares the same mechanism—parameter names can be referenced in type annotations.
`self` appears at positions like `self: Point`, `b` appears at positions like `b: Positive(b)`, both
annotations reference the parameter itself. The difference is only in the complexity of the type
annotation; the mechanism is identical—after name binding, types can depend on this name.

Return types use the same explicit function call:

```yaoxiang
Sorted: (arr: Array(T)) -> Type = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }

sort: (arr: Array(T)) -> (result: Sorted(result)) = { ... }
//                        ^^^^^^^^^^^^^^^^^^^^^^^
//                        result is return value parameter, Sorted(result) is function call
//                        compiler substitutes return value at return point, verifies Sorted(return value)
```

The same applies to local variable declarations:

```yaoxiang
let x: Positive(x) = 5
// x bound to 5, Positive(5) → { 5 > 0 } → True → pass

// let y: Positive(y) = 0
// y bound to 0, Positive(0) → { 0 > 0 } → False → compile error
```

#### 3.2 Path Condition Collection

When runtime values appear in conditional branches, the compiler automatically collects path
conditions, forming the **assumption set** of the current scope. These assumptions participate in
verification as background knowledge for compile-time Bool evaluation.

```yaoxiang
if y > 0 {
    // compiler automatically obtains assumption: { y > 0 } in this branch
    let result = divide(x, y)
    // verification condition: (y > 0) ⇒ (y > 0)
    // proof pipeline judges implication holds → Proved
} else {
// this branch assumption: { !(y > 0) }
// if divide(x, y) is called, verification condition is !(y > 0) ⇒ y > 0
    // proof pipeline judges implication does not hold → Disproved
}
```

This is not the compiler hardcoding special patterns—this is the natural behavior of the
compile-time proof pipeline. Each type check call site sends to the pipeline:

```
{background assumptions} ⇒ {verification target}
```

The proof pipeline judges implication. Proved → pass, Disproved → compile error + counterexample,
Unproven → compile error + unproven proposition. Background assumptions come from the path
conditions of the current program point.

#### 3.3 Assumption Stack

When analyzing control flow, the compiler maintains an assumption set for each basic block:

- **if-guard**: `if y > 0` → true branch pushes `y > 0`, false branch pushes `!(y > 0)` (if else is
  used)
- **match pattern**: `if let Some(v) = opt` → branch pushes `opt == Some(v)`
- **logical conjunction**: `if x > 0 and y < 10` → branch pushes `x > 0` and `y < 10`
- **function precondition**: when calling `divide(a, b)`, `b` must satisfy evidence of `Positive`
  either from current assumptions, or from the actual argument's own refinement type annotation (if
  `b` is annotated as `Positive`, its type carries `b > 0`)
- **assignment**: when `let z = y`, the existing refinement conditions on `y` are transferred to `z`

All assumptions enter the compile-time proof pipeline. When entering the SMT acceleration path, they
are translated into SMT-LIB background assertions.

#### 3.4 No Static Evidence Means Compile Error

If the programmer writes directly:

```yaoxiang
divide_user_input: (x: Int, y: Int) -> Int = divide(x, y)
```

There is no assumption of `y > 0` at the current program point, and the actual argument `y` itself
has no `Positive` type annotation. The verification condition is:

```
{} ⇒ { y > 0 }
```

The pipeline returns `Disproved` (implication does not hold) → compile error:

> Cannot prove parameter `b` satisfies `Positive` in call to `divide`. `y` comes from function
> input, no proven bound. Consider guarding the call with an if branch: `if y > 0 { divide(x, y) }`.

YaoXiang does not accept runtime values directly entering refinement type parameters without
providing static evidence. This is not a limitation—this is the core of the hard-safety philosophy.
Any code the compiler cannot statically prove is not allowed to compile.

#### 3.5 Relationship with the Unified Pipeline

Path condition propagation is not an additional mechanism. It is the direct extension of the
compile-time proof pipeline in control flow analysis:

| Stage                             | Responsibility                                                                                                                                           |
| --------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Path condition collection         | Compiler's control flow analysis phase, annotates assumption sets for each basic block                                                                   |
| Verification condition generation | When encountering type constraints to verify, merge path conditions + actual argument type information                                                   |
| Proof pipeline evaluation         | Compiler kernel → SMT acceleration → obtain Proved / Disproved / Unproven                                                                                |
| Result                            | `Proved` → pass; `Disproved` → compile error + counterexample; `Unproven` → compile error + unproven proposition (programmer can provide proof function) |

No new components. No special rules. Path conditions are the background knowledge of the proof
pipeline—sharing the same pipeline and budget system as type equality and borrow constraints.

### 4. Compile-time Proof Pipeline

All compile-time checks share the same pipeline. The core operation of the pipeline is **type
checking**—checking whether a proof term's type equals the proposition to be proven. Everything is
type checking.

```
Compile-time encounters Bool expression needing evaluation (i.e., needs to construct a proof term)
        │
        ├── Type equality (T1 == T2)
        │   → Compiler directly judges (structural equivalence)
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

#### 4.1 Proof Result: Three-valued Algebra

Compile-time evaluation returns three results—this is the inevitable conclusion of the halting
problem, and the natural partition of proof theory:

```
eval_compile_time : BoolExpr → Proved | Disproved(Model) | Unproven
```

- **Proved** → Halts, proof term constructed, type check passes. Compilation continues.
- **Disproved(M)** → Halts, counterexample M exists. Compile error + counterexample + source
  location.
- **Unproven** → Within given resource upper bound, no proof was constructed. Compile error +
  unproven proposition + budget consumption report.

**Unproven ≠ False.** The compiler saying "I cannot prove it" is not equivalent to the proposition
being false—it only exceeds the capability of the current automatic proof. This is honesty, not a
defect.

A hard budget limit is the engineering solution to the halting problem. No knob given—giving one is
asking the user "do you think your program will halt", and the user doesn't know, nor does the
compiler.

#### 4.2 After Unproven: Programmer Writes Proof

When the compiler returns Unproven, the programmer can write a **proof function**—just a YaoXiang
function whose return type equals the proposition to be proven. The type checker verifies this
function—in exactly the same way it verifies `add(a, b): Int`.

```
Proposition  = Type
Proof        = Program (a value of that type)
Verification = Type checking (the only trust root)
```

The SMT solver is not an independent trust boundary—it is an **acceleration module of the type
checker**. SMT helps find proofs, but the type checker is what verifies proofs. When SMT returns
`unsat`, the compiler restructures its result into a proof term verifiable by the type checker. If
restructuring fails (SMT's reasoning steps exceed the compiler kernel's inference rules), it falls
back to Unproven—the programmer can manually write a proof function.

```yaoxiang
# Proposition: refinement attribute the compiler cannot automatically prove
FirstIsMin: (T: Ord, arr: Sorted(T)) -> Type = {
    forall i in 0..arr.len: arr[0] <= arr[i]
}

# Proof: programmer writes a function, return type is the above proposition
# Type checker verifies this function—exactly the same as verifying add(a,b): Int
first_is_min: (T: Ord, arr: Sorted(T)) -> FirstIsMin(T, arr) = {
    # compiler verifies here: function body's type = FirstIsMin(T, arr)
    ...
}
```

No AI needed, no export to Coq, no new concepts. **Compile-time attributes that cannot be
automatically proven → programmer writes proof in YaoXiang code → type checker verifies.** The
entire process is a smooth gradient—the compiler does the simple proofs for you, leaving the hard
ones to your brain.

#### 4.3 Hierarchical Dependencies Within the Pipeline

The above evaluators share the same interface but have an evaluation order. Type equality is the
prerequisite for all subsequent analysis; ownership/token checks depend on type information;
refinement predicate verification depends on the results of the first two layers. The compiler
evaluates layer by layer; expressions that fail at lower layers do not enter upper layers—avoiding
wasting solver budget on type-incorrect programs.

```
Evaluation order (same pipeline, layered scheduling)
├── Layer 0: Type equality (T1 == T2)
│   └── Structural unification → failure makes subsequent meaningless, directly return Disproved
├── Layer 1: Ownership/token conflicts
│   └── Flow-sensitive liveness analysis → failure means memory safety does not hold, directly return Disproved
└── Layer 2: Refinement predicates/Hoare implications
    └── Compiler itself → SMT acceleration → obtain Proved / Disproved / Unproven
```

Each layer still returns `Proved/Disproved/Unproven`, sharing the same interface and the same budget
system.

### 5. Three-layer Function Unity

| Layer                  | Execution Timing | Input        | Output | Example                                        |
| ---------------------- | ---------------- | ------------ | ------ | ---------------------------------------------- |
| Value-level function   | Runtime          | Values       | Values | `add: (a: Int, b: Int) -> Int = a + b`         |
| Type constructor       | Compile-time     | Types/values | Type   | `List: (T: Type) -> Type = { data: Array(T) }` |
| Compile-time predicate | Compile-time     | Values       | Type   | `Positive: (x: Int) -> Type = { x > 0 }`       |

All use the same `name: type = value` syntax. Compile-time predicates and type constructors go
through the same compile-time proof pipeline—`{}` is proof space.

### 6. Loops: Floyd-Hoare Verification Condition Generation

Loops do not need separate `: Invariant(...)` or `: decreases(...)` annotations. The compile-time
predicate type annotations on variables define Floyd-Hoare-style assertions—the compiler generates
verification conditions from the type annotations, and the proof pipeline checks whether each
assignment maintains the type.

Core mechanism: each assignment operation corresponds to a Hoare triple `{P} x := e {Q}`, the
verification condition is `P ⇒ Q[e/x]`. The compiler generates one verification condition for the
loop body—after the proof pipeline verifies the inductive step, all iterations are covered
automatically.

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # annotation references i; at initialization i=0, verify: 0 == sum(arr[0..0]) → True
    mut i: UpTo(arr.len) = 0     # verify: 0 <= 0 <= arr.len → True
    while i < arr.len {
        # compiler generates one VC for the loop body. Premise: s satisfies SumUpTo(arr, i), i satisfies UpTo(arr.len).
        #
        # s += arr[i]:
        #   verification obligation: s_new satisfies SumUpTo(arr, i) (current i unchanged)
        #   substitute s_new = s_old + arr[i]:
        #     need s_old + arr[i] == sum(arr[0..i+1])
        #     from inductive hypothesis s_old == sum(arr[0..i]), add arr[i] to both sides:
        #     sum(arr[0..i]) + arr[i] == sum(arr[0..i+1])
        #   compiler + SMT: linear arithmetic, millisecond level → Proved
        #
        # i += 1:
        #   i changes → s's type annotation in dependency graph references i → triggers re-verification
        #   new verification target: s satisfies SumUpTo(arr, i_new)
        #   i.e., s == sum(arr[0..i_new]), guaranteed by previous step → Proved
        s += arr[i]
        i += 1
    }
    return s  # at this point s: SumUpTo(arr, arr.len), i.e., s == sum(arr[0..arr.len])
}
```

Loop invariants are the type annotations on variables—programmer writes types, compiler checks the
inductive step. The compiler does not need to "discover" invariants, nor "automatically do
induction"—it decomposes the inductive proof into local verification conditions for each assignment
operation, delegating to the proof pipeline to divide and conquer.

#### 6.1 Dependency Tracking: Dependent Types on Mutable Variables

The premise of the above mechanism is: the compiler knows that `s`'s type annotation
`SumUpTo(arr, i)` references `i`—when `i` changes, `s`'s type constraint also changes. This requires
the compiler to maintain a **type dependency graph between variables**.

**Data structure**:

```
TypeDepGraph: Map<VarName, Set<VarName>>
# key is the depended variable, value is the set of variables referencing it in type annotations
# e.g.: { i: {s}, j: {s, t}, ... }
```

**Construction**: When the type checker processes `mut v: Pred(... x ...) = init`, it parses free
variable references in `Pred(...)` parameters. If the parameters reference other mutable variable
`x` in the current scope, it records `x → v` in the dependency graph.

**Triggering**: When the depended variable `x` is assigned, the compiler:

1. Finds all variables `{v₁, v₂, ...}` in the dependency graph that depend on `x`
2. For each `v`, generates a verification condition: whether `v`'s current value satisfies the
   updated type `Pred(... x_new ...)`
3. Sends the VC into the proof pipeline

**Assignment order sensitivity**: Dependency tracking naturally enforces correct assignment order.
Taking `SumUpTo(arr, i)` as an example:

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

**Composite dependencies**: a variable can depend on multiple variables. The type annotation
`{ v: Int; v == x + y }` depends on both `x` and `y`—any change triggers re-verification.

**Relationship with the proof pipeline**: dependency tracking is the trigger for VC generation, not
an independent verification mechanism. It answers "when is a VC needed to be generated"—the proof
pipeline answers "does the VC hold".

### 7. Termination Check

Fully automatic at compile time. Loops the compiler can prove pass; loops the compiler cannot prove
directly report compile errors—the programmer must make the loop's termination automatically
analyzable by the compiler. No half-automatic annotation escape hatches.

#### 7.1 Design Principles

The compiler automatically extracts information needed for termination proof from two places:

1. **Variable type annotations**: boundary constraints in refinement types (e.g., `UpTo(n)` gives
   upper bound `n` and lower bound `0`)
2. **Loop body operations**: operations applied to variables on each iteration

The compiler tries four metric synthesis strategies in priority order, stops as soon as one is
found.

#### 7.2 Strategy 1: Linear Rank Function Automatic Synthesis

When a variable has a linear bound annotation, the compiler enumerates candidate linear metrics and
verifies with SMT.

```
Input:
  Variables v₁: UpTo(u₁), v₂: UpTo(u₂), ... (variables with upper and lower bounds)
  Loop condition cond
  Set of assignments in loop body

Algorithm:
  1. Extract each variable's bound from type annotation: [low_i, high_i]
  2. Enumerate candidate metrics: v_i, u_i - v_i, v_i - v_j, etc. linear combinations
  3. For each candidate metric m:
     - SMT verifies m ≥ 0 (derived from type bounds)
     - For each execution path in loop body, SMT verifies m' < m (strictly decreasing)
  4. Find a linear combination that meets conditions → termination proven
```

Coverage: loops where any variable is assigned a linear expression (`v = a·v + b`) and has bounded
type annotation. Includes `i += const`, `i -= const`, and binary-search-style interval contraction:

```yaoxiang
# Binary search: low = mid + 1 or high = mid
# metric high - low strictly decreases on both paths
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

#### 7.3 Strategy 2: Predicate Violation Counting—Automatically Extract Metric from Target Type <span style="color:orange">【Experimental Strategy】</span>

> ⚠️ **Current status: experimental strategy, whether to include will be determined during Phase 3
> implementation based on actual feasibility.** This strategy works for adjacent swap operations
> (bubble sort, insertion sort), but cannot automatically prove non-adjacent operations (quicksort
> partition, heapsort sift-down). Coverage boundaries are shown in the table below. If Phase 3
> verification proves infeasible, this strategy will be removed or downgraded to future work.

Core insight: **the user-written specification is material for the compiler's reasoning.** The
compiler does not need to build in "what is sorting"—it reads the `Sorted` definition and
automatically extracts the metric from it.

```
Input:
  Target type: Sorted(arr) = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }
  Loop body operation: adjacent element swap

Algorithm:
  1. Parse predicate definition: forall i in range: cond(i, arr)
  2. Automatically generate metric: violation_count = |{ i | ¬cond(i, arr) }|
  3. Analyze operation's effect on metric:
     - adjacent swap arr[j], arr[j+1] = arr[j+1], arr[j]
     - only affects index pairs j-1, j, j+1
     - if arr[j] > arr[j+1] (violates predicate), after swap this pair satisfies predicate
     - violation_count decreases by at least 1
  4. Upper bound: n·(n-1)/2 (max adjacent inversions), lower bound: 0
  → termination proven
```

**Current coverage**:

| Algorithm      | Operation Pattern | Strategy 2 Provable? | Reason                                         |
| -------------- | ----------------- | :------------------: | ---------------------------------------------- |
| Bubble sort    | Adjacent swap     |          ✅          | violation_count strictly decreases per swap    |
| Insertion sort | Adjacent move     |          ✅          | each shift eliminates one violation pair       |
| Selection sort | Non-adjacent swap |          ❌          | single swap may increase violation_count       |
| Quicksort      | partition         |          ❌          | non-adjacent swap, no monotonic decrease       |
| Heapsort       | sift-down         |          ❌          | tree operations, violation_count not monotonic |

**Complementary strategy**: for quicksort, `low < high` interval contraction is covered by Strategy
1 (linear rank function)—outer partition recursion halves the interval each time. Strategies 1 and 2
complementarily cover each other; most practical algorithms' termination can be proven by one of the
two. But generalizing Strategy 2 (non-adjacent operations, tree operations) remains an open problem.

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

#### 7.4 Strategy 3: Bounded Increase/Decrease Pattern

`v += const` (positive constant), variable has upper bound type annotation → metric
`upper_bound - v` decreases by `const` each time, lower bound 0. This is a degenerate case of
Strategy 1, which the compiler handles quickly at the front.

#### 7.5 Strategy 4: Multiplicative Scaling Metric Template

`v *= const` (const > 1), variable has upper and lower bound type annotations. The compiler has a
built-in logarithmic metric template `ceil(log_const(upper/v))`, metric decreases by 1 per
multiplication by const.

```yaoxiang
mut i: Positive(i) = 1
while i < n {
    # compiler automatically derives: metric ceil(log₂(n/i)), metric decreases by 1 per multiplication by 2
    i *= 2
}
```

#### 7.6 Separation of Termination and Correctness

Termination proof and correctness proof are independent:

- **Termination**: the above four strategies automatically prove the loop exits in finite steps
- **Correctness**: whether the loop body advances toward the target type, checked by the
  compile-time proof pipeline via verification conditions

Both pass → compilation passes. Termination proven but correctness fails → compile error +
counterexample. Correctness proven but termination unprovable → compile error pointing out
unanalyzable variable or operation. Both fail → compile error reports both failure reasons
separately.

#### 7.7 Termination Check for Recursive Functions

For recursive functions that need to be evaluated at compile time, the compiler checks for parameter
decrease:

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)  # compiler analyzes: n-1 < n → decreases → terminates
}

# Compile-time use—compiler guarantees factorial terminates at compile time
vec: Vec(factorial(5)) = Vec(120)()  # 5! = 120, completed at compile time
```

| Scenario                                              | Behavior                    |
| ----------------------------------------------------- | --------------------------- |
| Compiler can analyze recursive decrease (e.g., `n-1`) | Compile-time evaluation     |
| Does not decrease/cannot determine decrease           | Compile error               |
| Runtime call (not at type position)                   | No termination check needed |

#### 7.8 Hard Boundary

`i = f(i)` where `f` is non-invertible, not closed, and does not preserve any
monotonicity—mathematically impossible to automatically prove termination. Compile error:

> This loop cannot automatically prove termination. Loop variable depends on unanalyzable function
> `f`. Please use an iteration pattern that can be analyzed by the compiler.

This is not a compiler failure. Any code that cannot be statically proven safe is not allowed to
compile.

### 8. SMT Solver: Acceleration Module of the Type Checker

The SMT solver is an external tool in traditional languages (e.g., F\* calls Z3, Dafny calls Z3). In
YaoXiang, it is **an acceleration module of the type checker**—only invoked when the compiler kernel
itself cannot directly judge. SMT helps find proofs, but the type checker is what verifies them.

**Trust model**: the type checker is the only trust root. The SMT solver is an acceleration
module—it helps find proofs, but SMT is not an independent trust boundary. The compiler trusts Z3's
`unsat` result (consistent with the F\*/Dafny line—the probability of Z3 error is lower than the
compiler's own bug rate, an engineering pragmatic choice). The real unreliability control is at the
SMT translation layer—if the translation has bugs, the compiler will expose them in other tests.

**Interface**: the compiler internally translates to SMT-LIB 2.6 standard format, not bound to a
specific solver API. SMT-LIB is an ISO standard, natively supported by Z3, CVC5, MathSAT, Yices.

**Default backend**: Z3 (MIT license, most extensive documentation and community validation). CVC5
as an SMT-LIB-compatible alternative—users can switch via compiler flags at compile time.

No "universal solver abstraction layer"—SMT-LIB is the abstraction layer. In the future if CVC5 has
breakthroughs in specific theories, switching only requires changing the binary, no compiler code
changes.

```
Compile-time Bool expression
        │
        ├── Compiler kernel can directly judge (structural equivalence, simple arithmetic,
        │   trivial formulas after constant folding)
        │   → directly return Proved / Disproved
        │
        └── Compiler kernel cannot directly judge (quantifiers, symbolic variables)
            → Dependent type pre-reduction (factorial(5) → 120)
            → Translate to SMT-LIB format
            → Send to Z3/CVC5 (with budget limit)
            → Return value: unsat → Proved  │  sat + model → Disproved  │  unknown → Unproven
```

**Solver budget—hard limit, like stack depth**:

| Budget Dimension               | Default | Description                                                                                                                                            |
| ------------------------------ | ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Solving steps                  | 10,000  | Z3 for linear arithmetic usually within hundreds of steps. 10,000 steps covers 99% of practical predicates.                                            |
| Time                           | 100ms   | Single predicate exceeding 100ms = user is writing a compile-time program, not a type annotation. 100ms × 50 predicates = 5-second compile time limit. |
| Quantifier instantiation depth | 3       | Three layers of nested quantifiers cover practical patterns. Exceeding three layers is likely writing logic exercises.                                 |

Exceeding budget returns Unproven, compile error + predicate location + consumption. No degradation,
no runtime checks, no silent pass.

**Why this is actually feasible**: 95% of practical predicates in engineering are linear
arithmetic—`x > 0`, `arr.len > 0`, `0 <= idx < arr.len`—all within decidable fragments, and SMT
solvers return millisecond-level for such problems. Encountering the rare complex predicate
exceeding budget, the programmer can write a proof function.

Dependent types have a layer of pre-reduction before SMT calls: `factorial(5)` is directly evaluated
at compile time to get `120`, `append([1,2], [3])` is directly evaluated to get `[1,2,3]`. These
deterministic value computations do not consume SMT budget.

The programmer does not need to know SMT exists. The mental model is: **the compiler can prove it
then it passes, cannot then it errors—if the compiler won't, you can write a function to prove it.**

### 9. Compile-time Predicate Composition

Compile-time predicates are functions returning Type, composition is naturally implemented through
function composition:

```yaoxiang
SortedNonEmpty: (T: Ord, arr: Array(T)) -> Type = {
    Sorted(T, arr) and NonEmpty(arr)
}
```

### 10. Code Examples

#### 10.1 Division Safety

```yaoxiang
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b

result = divide(10, 2)   # ✅ compiler verifies Positive(2) = { 2 > 0 } → True
# result = divide(10, 0)  # ❌ compiler verifies Positive(0) = { 0 > 0 } → False
```

#### 10.2 Array Access Safety

```yaoxiang
InBounds: (idx: Int, arr: Array(T)) -> Type = { 0 <= idx and idx < arr.len }

get: (arr: Array(T), idx: InBounds(idx, arr)) -> T = arr.data[idx]

arr = Array(Int)(1, 2, 3)
x = get(arr, 1)   # ✅ compiler verifies InBounds(1, arr) = { 0 <= 1 and 1 < 3 } → True
# y = get(arr, 5)  # ❌ compiler verifies InBounds(5, arr) = { 0 <= 5 and 5 < 3 } → False
```

#### 10.3 Sort Correctness

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

#### 10.4 Loop: Compiler VC Generation

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

### 11. Dispatch Pipeline: Unified Dispatching of Compile-time and Runtime

`assert` and `Assert` are two sides of the same refinement type primitive. The dispatch pipeline
`dispatch` automatically decides whether to take the compile-time proof or runtime check based on
**whether the predicate's free variables are accessible at compile time**:

| Criterion                                                                              | Mode            | Behavior                                                                                 |
| -------------------------------------------------------------------------------------- | --------------- | ---------------------------------------------------------------------------------------- |
| All free variables known at compile time (generic parameters, compile-time constants)  | **CompileTime** | Enter proof pipeline: Proved → erase, Disproved → compile error, Unknown → require proof |
| Free variables exist from runtime (function parameters, external input, mut variables) | **Runtime**     | Insert runtime check, inject refinement facts into flow-sensitive assumption set Γ       |

**Key**: "Cannot judge" ≠ "Refuted". In CompileTime mode, Unknown requires proof (no silent
degradation), in Runtime mode the proposition has no truth value at compile time at all—no matter
how strong the prover is, it cannot write a universally true proof for "user may have entered a
negative number", runtime check is the only sound choice. This is not because the prover is not
strong enough, it is theoretically necessary.

### 12. Flow-sensitive Assumption Set Γ: Strongest Postcondition Propagation

The compiler maintains a flow-sensitive assumption set Γ, tracking propositions known to hold at
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
mut x = x - 5       // Γ = {}  ← x > 0 killed
```

This is a hard requirement of soundness—variable value changed, old assumptions invalid.

**Branch confluence**: when IF/ELSE or match branches merge, Γ takes the intersection of each
branch's assumptions. Only propositions that hold in all paths are carried out of the branch.

### 13. Erasure Model Clarification: Witness Erasure ≠ Check Erasure

RFC-027's assertion that "refinement types are **completely erased** at runtime" refers to the
**proof witness**—proof terms verified at compile time do not produce runtime code. But runtime
checks inserted by dispatch in Runtime mode are retained—they are Bool checks executed at the value
level, not witnesses at the type level.

Summary: witnesses erased, checks retained. The two things do not conflict, RFC-027's original
assertion remains unchanged.

## Detailed Design

### Syntax Changes

| Before (RFC-022)                      | After (This RFC)                                                             |
| ------------------------------------- | ---------------------------------------------------------------------------- |
| `//! requires: NonEmpty(n) = n > 0`   | Compile-time predicate as parameter type `(b: Positive(b))`                  |
| `//! ensures: ExistsMax(result, arr)` | Return type uses return value parameter `-> (result: IsMax(T, arr, result))` |
| `/*! invariant: ... !*/`              | Compile-time predicate type annotation on variable—Floyd-Hoare invariant     |
| `//! decreases: n`                    | Compiler fully automatically derives metric function                         |
| Specification as annotation           | Specification as type system                                                 |

### Syntax

**Compile-time predicates have no new keywords.** `{}` is proof space, completely consistent with
existing type definition syntax. A compile-time predicate is a function returning
Type—`name: (params) -> Type = { assertion }`. Usage is function call—`Positive(b)`,
`IsMax(T, arr, result)`.

```bnf
# Compile-time predicate = function returning Type, compiler-verified assertions in {}
# Uses existing function/type syntax, no new BNF rules needed
predicate ::= identifier ':' params '->' 'Type' '=' '{' assertions '}'
```

**New syntax concept: return value parameter**—in `-> (name: Type)`, `name` is the return value
parameter.

The return value parameter is the **only syntax concept** YaoXiang introduces on top of existing
function syntax. Its semantics:

- `name`'s value is provided by the `return` statement
- `name` exists only in the type signature, referenced by postcondition predicates (e.g.,
  `-> (result: IsMax(T, arr, result))`)
- `name` does not enter the function body scope, does not appear at the call site
- Return value parameter is **optional**—without postconditions, the signature is identical to
  ordinary functions (`-> Int`), introducing no additional burden

Reason for introduction: postconditions need to reference "the value the function is about to
return". Without return value parameters, the compiler can only let predicates reference return
values through special rules (such as implicit variable `$result` or `__retval__`). Return value
parameters make this reference explicit—it is just a parameter, except the value is provided by
`return` rather than the caller.

**Proof functions** are not a new concept—they are simply a YaoXiang function whose return type is
the asserted proposition. When the compiler returns Unproven, the programmer provides a proof
function, and the type checker verifies it in exactly the same way it verifies any function's return
type. No new syntax, no new keywords, no new rules.

### Type System Impact

- **Type universe**: compile-time predicates reside at the Type₂ layer—functions accepting values
  and returning Type, at the same level as type constructors
- **Generics interaction**: compile-time predicates can carry generic parameters, e.g.,
  `NonEmpty: (T: Type) -> (arr: Array(T)) -> Type`
- **Ownership interaction**: expressions in compile-time predicates follow ownership rules, only
  read, not write
- **Type inference**: parameters of compile-time predicates participate in HM type inference

### Runtime Representation

Compile-time predicates are processed at runtime **according to dispatch results**:

- **CompileTime mode** (all free variables known at compile time): proof passes then witness is
  completely erased. `Positive: (x: Int) -> Type = { x > 0 }`—parameter `b: Positive(5)`'s runtime
  representation is just `Int`. Refinement condition `{ 5 > 0 }` is verified, erased.
- **Runtime mode** (runtime free variables exist): retain runtime check—execute Bool check at value
  level, inject into flow-sensitive assumption set Γ. See §11 dispatch pipeline and §13 erasure
  model clarification for details.

Placing compile-time predicates at type positions (e.g., `f(x: Positive(x))`) does not produce
wrapper types, does not allocate extra memory. But when `x` comes from runtime input, **it will**
insert runtime Bool check.

**Interaction constraint with `ref`**: compile-time predicates can only reference immutably borrowed
or ownership-transferred values. Compile-time predicates referencing mutably borrowed values cannot
be guaranteed by the compiler at compile time to remain valid at runtime—such usage directly reports
compile error.

### Compiler Changes

1. **Parser**: compile-time predicates use standard function syntax, no additional parsing rules
2. **Compile-time proof pipeline**: unified Proved/Disproved/Unproven return interface, automatic
   strategy selection
3. **SMT acceleration module**: SMT-LIB 2.6 translation layer, default backend Z3, CVC5 alternative
4. **Type checker kernel**: inference rule implementation—structural equivalence, βδι-reduction,
   universal quantifier introduction/elimination. This is the only trust root, both SMT and
   programmer proofs are verified through this
5. **Verification condition generation**: WP/SP calculus + loop invariant proof obligations
6. **Error reporting**: counterexample formatting + unproven proposition report + source location
   correlation

### Backward Compatibility

- ✅ Code not using compile-time predicates is completely unchanged
- ✅ Compile-time predicates have zero runtime overhead in CompileTime mode, only retain necessary
  Bool checks in Runtime mode
- ⚠️ RFC-022's `//!` syntax is no longer supported—but 022 was never implemented, no migration
  burden

## Trade-offs

### Advantages

- **Curry-Howard isomorphism fully realized**: types are propositions, programs are proofs,
  `name: Proposition = Proof`
- **Unity**: compile-time predicates and ordinary functions use completely identical syntax, no
  conceptual split
- **SMT transparency**: programmers do not need to know SMT exists, mental model consistent with
  type checking
- **Progressive adoption**: can start from one compile-time predicate, gradually increase coverage
- **Minimal runtime overhead**: zero overhead in CompileTime mode, only necessary Bool checks in
  Runtime mode

### Disadvantages

- **Compile time**: SMT solving increases compile time, but hard budget limits guarantee upper bound
  is controllable
- **Automatic proof boundary**: complex predicates beyond first-order linear arithmetic may require
  programmer to write proof functions. This is not a language defect—this is the inevitable
  conclusion of the halting problem. The compiler honestly reports Unproven rather than falsely
  reporting True/False
- **Learning curve**: writing effective compile-time predicates and proof functions requires
  understanding basic intuition of the Curry-Howard isomorphism
- **Implementation complexity**: unifying the compile-time proof pipeline requires careful design

### Risk Mitigation

- SMT solver budget hard limit (steps 10,000 / time 100ms / instantiation depth 3), exceeding budget
  returns Unproven
- Dependent type pre-reduction: deterministic value computations are consumed first, SMT only chews
  the non-deterministic part
- Unproven is not a dead end: programmer can write proof function, type checker verifies—consistent
  with verifying any function return type
- Incremental verification: only verify changed modules
- Clear error messages + counterexample display + budget consumption report + unproven proposition +
  suggestion (if compiler can give)

## Alternatives

| Alternative                                                        | Why Not Chosen                                                                                                                                                     |
| ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| RFC-022: `//!` annotation-style specifications                     | Specifications and types split, violates Curry-Howard isomorphism                                                                                                  |
| Independent specification files (e.g., CVL)                        | Specifications separated from code, increases maintenance cost                                                                                                     |
| Runtime-only assertions                                            | Cannot statically guarantee correctness                                                                                                                            |
| External proof assistant (e.g., Coq)                               | Disconnected from compiler, requires independent proof language and trust boundary. YaoXiang's choice: proof is YaoXiang code, type checker is the only trust root |
| **This proposal: compile-time predicates as first-class citizens** | ✅                                                                                                                                                                 |

## Implementation Strategy

### Phase Division

| Phase       | Content                                                                                                                                                                |
| ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Phase 1** | Compiler kernel: structural equivalence + βδι-reduction + universal quantifier introduction/elimination. Support simple arithmetic predicates (`x > 0`, `arr.len > 0`) |
| **Phase 2** | SMT-LIB translation layer + Z3/CVC5 integration. Pipeline returns Proved/Disproved/Unproven. Support programmer-written proof functions when Unproven                  |
| **Phase 3** | Loop invariant VC generation + termination check (linear rank function + predicate violation counting + bounded pattern + combinatorial explosion control)             |
| **Phase 4** | Incremental verification + caching + IDE support                                                                                                                       |

### Dependencies

- RFC-010: Unified Type Syntax — compile-time predicates based on `name: type = value`
- RFC-011: Generic Type System — compile-time predicates can carry generic parameters
- RFC-009: Ownership Model — compile-time predicate expressions follow ownership rules

## Open Questions

- [x] **SMT solver choice**: default Z3 (MIT license, most extensively validated). CVC5 as SMT-LIB
      compatible alternative, switched via compiler flag. Compiler internal translation target is
      SMT-LIB 2.6 standard format—SMT-LIB is the abstraction layer, no custom universal solver
      interface.
- [x] **Specific values of solver budget**: steps 10,000 / time 100ms / quantifier instantiation
      depth 3. Fixed inside compiler, no knob. If real use cases prove it is insufficient in actual
      use (not "user wrote wrong"), adjust then.
- [x] **Quantifier support scope**: language does not limit quantifier order. Compile-time
      predicates accept Type parameters—Type includes function types—therefore higher-order
      quantifiers are a natural consequence of the type system, no special syntax needed. SMT solver
      can automatically judge first-order quantifiers (forall/exists, supports interleaved nesting,
      limited by budget depth 3). Higher-order quantifiers: SMT returns Unproven, compiler prompts
      "this predicate exceeds automatic proof range, please provide proof function". Programmer
      writes a YaoXiang function whose return type equals the proposition—type checker verifies the
      function. No external export needed, no AI needed, no interactive proof mode needed.
      Everything is YaoXiang code, everything is verified by the type checker.
- [x] **Counterexample formatting**: source variable names directly used as SMT variable names (with
      module prefix to avoid conflicts). Z3 model returned is looked up by variable name. Output
      format: variable name = specific value + source location + predicate definition location. No
      complex mapping layer.
- [x] ~~**Interaction between `ref` smart pointer and compile-time predicates?**~~ → Decided:
      compile-time predicates only allow immutably borrowed or ownership-transferred values. Mutably
      borrowed values cannot appear in compile-time predicates.
- [x] **Extension of `forall` predicate violation counting metric to non-adjacent operations?** → No
      extension. Current coverage (adjacent swap, adjacent move) is complementarily covered by
      Strategy 1 (linear rank function)—quicksort outer interval contraction is covered by Strategy
      1, heapsort is covered by Strategy 1 (array index pattern). Loops whose termination cannot be
      proven by any strategy, compiler directly reports error—this is hard-safety philosophy, not a
      defect. If there are real scenarios in the future (not academic constructions) where
      algorithms cannot be covered by all four strategies, rediscuss then.
- [x] **Linear rank function enumeration combinatorial explosion**: candidate enumeration upper
      limit is 3 bounded variables. ≤3 enumerate all linear combinations and SMT verify one by
      one. >3 only try single variable metrics (`v_i`, `u_i - v_i`), failure directly reports
      compile error—prompting programmer "loop has >3 bounded variables, compiler cannot
      automatically synthesize multi-variable metrics". This is not an engineering compromise—it is
      forcing the programmer to write simpler loops.

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
│ (formal     │    │ (remains in │
│ design)     │    │  place)     │
└─────────────┘    └─────────────┘
```
