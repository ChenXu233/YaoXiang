---
title: 'RFC-027: Compile-Time Predicates and Unified Static Verification'
status: 'Accepted'
author: 'Chenxu'
created: '2026-06-07'
updated: '2026-09-14'
impl_status: 'in_progress'
impl_detail:
  'Phase 1-2 complete, Phase 3 partially complete, Phase 4 partially complete. All 6 phases of the
  assert/Assert unified scheme are implemented (issues #157-#162 closed): Never type, IsTrue bridge,
  flow-sensitive Γ + kill set, type-level recursion, weak universe stratification check, dispatch
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
> - [RFC-011: Generics System Design](../accepted/011-generic-type-system.md)
> - [RFC-024: Concurrency Model Based on spawn Blocks](../accepted/024-concurrency-model.md)
>
> **Supersedes**:
> [RFC-022: Hoare Logic Static Verification Support (Specification Comments and Specification Types)](../deprecated/022-hoare-logic-static-verification.md)
> — Deprecated

## Summary

This RFC proposes introducing **compile-time predicates** as first-class citizens to YaoXiang,
unifying all compile-time static verification into a single **proof pipeline**. Compile-time
predicates are not bolted-on specification comments—they _are_ functions. A function that returns a
`Type` can be used at type position; the compiler invokes it at compile time and checks the return
value. Types are propositions; compile-time evaluation is proof.

**Core thesis**: The sole task of type checking at compile time is to construct and verify proof
terms. Type equality, token conflicts, dependent type reduction, compile-time predicate evaluation,
Hoare logic implication—all are different type checks within the compile-time proof pipeline,
sharing a single pipeline. The SMT solver is an acceleration module of the type checker, not an
independent trust boundary. When the compiler returns `Unproven`, the programmer writes a YaoXiang
function as proof—the type checker verifies it in exactly the same way it verifies the return type
of any function. Everything is YaoXiang code; everything is verified by the type checker.

## Motivation

### Why Deprecate RFC-022?

RFC-022 designed specifications in the form of `//!` comments:

```yaoxiang
max: (T: Ord) -> ((arr: Array(T, n)) -> T) = {
    //! requires: NonEmpty(n) = n > 0          ← This is a comment independent of types
    //! ensures: ExistsMax(result, arr[0..n])   ← This is a comment independent of types
}
```

This commits a fundamental error of the Curry-Howard isomorphism: **splitting specifications and
types into two layers**. Comments are not types. Comments do not participate in type checking.
Comments are the mental model of "external tools."

The whitepaper states clearly:

> "There are no `//!` comments. There is no separate specification language. Everything is within
> the type system."

### Current Problems

- RFC-022's `//!` comments are external syntax bolted on outside the type system
- Specification types and ordinary types are two separate systems, causing conceptual redundancy
- The split pattern of Debug Build verification / Release Build ignore breaks unity
- The SMT solver is conventionally positioned as an external tool—YaoXiang embeds it as an
  acceleration module of the type checker
- Type checking, borrow verification, compile-time predicate checking, and macro expansion each take
  different paths

### The Correct Mental Model

Type checking can be abstracted as a function:

```
verify : Program → Proved | Disproved(Model) | Unproven
```

All compile-time checks—simple type matching, borrow conflict detection, compile-time predicate
verification—are subtasks of this function. They share the same proof pipeline; they differ only in
the complexity of proof terms and the construction strategy.

When the compiler returns `Unproven`, the programmer provides a proof function—whose return type
equals the proposition to be proven. The type checker verifies it. This is the same operation as
ordinary type checking.

## Proposal

### 1. `{}` Is the Proof Space: Types Are Assertions, Verification Is Type Checking

YaoXiang's `{}` is the compile-time proof space. Everything inside is an assertion; the compiler
guarantees each item is `True`—either proven automatically or with a proof function provided by the
programmer.

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
#          parameter in signature   only assertions in {}
#          compiler verifies x > 0 at compile-time invocation

List: (T: Type) -> Type = { data: Array(T) }
#      ^^^^^^^^              ^^^^^^^^^^^^^^^
#      parameter in signature  compiler verifies type_of(T) == Type, type_of(data) == Array(T)
```

The same pattern: `name: (params) -> Type = { assertions }`. The compiler makes no distinction
between "type assertions" and "value assertions"—both are evaluation targets in the proof pipeline.

**Loop invariants do not need to be written separately. The type annotation on a variable _is_ the
Floyd-Hoare invariant.**

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # annotation references i—tells the compiler s's type depends on i
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
proof pipeline verifies the inductive step holds, all iterations are automatically covered. No need
for `: decreases`, no need for `: Invariant`, no need for inductive proofs—the compiler decomposes
induction into local VCs for each assignment.

### 2. Pre/Postconditions: Compile-Time Predicates on Parameter Types and Return Types

Abandon RFC-022's `//! requires`/`//! ensures`. Compile-time predicates serve as type annotations on
parameters or return values.

**On the parameter side, it's a function call.** A compile-time predicate is a function that returns
a `Type`; using it on the parameter side is invoking it—just like `factorial(5)`. The return side
introduces a new concept: the return value parameter.

```yaoxiang
# Precondition: explicitly invoke a compile-time predicate in the parameter type
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b
#                       ^^^^^^^^^^  b is the current formal parameter name, passed to Positive
#                       compiler extracts actual argument value at call site, substitutes into b, verifies Positive(arg)
#                       example: divide(10, 2) → verify Positive(2) = { 2 > 0 } → True
#                       example: divide(10, 0) → verify Positive(0) = { 0 > 0 } → False → compile error

# Postcondition: return value parameter + compile-time predicate
IsMax: (T: Ord, arr: Array(T), result: T) -> Type = {
    forall j in 0..arr.len: result >= arr[j]
}

NonEmpty: (arr: Array(T)) -> Type = { arr.len > 0 }

max: (T: Ord) -> ((arr: NonEmpty(arr))) -> (result: IsMax(T, arr, result)) = {
#                                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
#                                            result is the return value parameter, value provided by return
#                                            compiler substitutes the return value at the return point, verifies postcondition
    candidate = arr[0]
    for i in 1..arr.len {
        if arr[i] > candidate { candidate = arr[i] }
    }
    return candidate
}
```

**Key rules**:

- **Parameter side**: `b: Positive(b)` — `b` is the current formal parameter name, passed to
  `Positive` as an argument. Function call syntax, zero implicit.
- **Return side**: `-> (result: IsMax(T, arr, result))` — `result` is the return value parameter,
  its value provided by the `return` statement. `result` exists only in the type signature, only
  referenced by predicates; it does not enter the function body scope, does not appear on the caller
  side.
- **Return value parameter optional**: when there is no postcondition, do not write it—the signature
  is identical to a normal function (`-> Int`).
- **Unity**: parameters and return value parameters are the same
  concept—`formal_name: predicate_call(formal_name)`—the only difference being whether the value is
  provided by the caller or by `return`.

### 3. Path Condition Propagation: Compile-Time Verification of Runtime Values

When a compile-time predicate is used at a binding site, arguments are explicitly passed by the
programmer. When runtime values enter refined type arguments, the compiler completes verification
through path condition collection and SMT implication—without requiring the programmer to explicitly
pass a proof.

#### 3.1 Explicit Function Call

When a compile-time predicate is used at a binding site, arguments are explicitly passed by the
programmer—it is simply a function call, zero implicit.

`Positive: (x: Int) -> Type = { x > 0 }` is a compile-time predicate constructor. When it appears at
a binding site (parameter declaration, variable declaration, return type), the programmer explicitly
passes an already-bound variable name:

```yaoxiang
b: Positive(b)
// b is already declared as the current formal parameter, Positive(b) is a function call
// after normalization: b: { b > 0 }
```

The compiler does not implicitly fill in arguments—`b: Positive(b)` is just like `f(5)`, a function
call. `b` is bound as a parameter name, and its type annotation `Positive(b)` references `b`
itself—this is the standard dependent type pattern, not an implicit expansion rule.

**Unity with RFC-010's `self`**: RFC-010 establishes that `self` is not a keyword, just a
conventional parameter name ("writing `p`, `this`, `x` is exactly the same effect").
`b: Positive(b)` shares the same mechanism—a parameter name can be referenced in a type annotation.
`self` appears in the position `self: Point`; `b` appears in the position `b: Positive(b)`. Both
type annotations reference the parameter itself. The difference lies only in the complexity of the
type annotation; the mechanism is completely the same—once the name is bound, the type can depend on
that name.

The return type also uses explicit function calls:

```yaoxiang
Sorted: (arr: Array(T)) -> Type = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }

sort: (arr: Array(T)) -> (result: Sorted(result)) = { ... }
//                        ^^^^^^^^^^^^^^^^^^^^^^^
//                        result is the return value parameter, Sorted(result) is a function call
//                        compiler substitutes the return value into result at the return point, verifies Sorted(return_value)
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
conditions to form the **assumption set** of the current scope. These assumptions participate in
verification as background knowledge for compile-time Bool evaluation.

```yaoxiang
if y > 0 {
    // the compiler automatically obtains the assumption here: { y > 0 }
    let result = divide(x, y)
    // verification condition: (y > 0) ⇒ (y > 0)
    // proof pipeline judges the implication holds → Proved
} else {
// this branch assumes: { !(y > 0) }
// if divide(x, y) is called, the verification condition is !(y > 0) ⇒ y > 0
    // proof pipeline judges implication does not hold → Disproved
}
```

This is not the compiler hard-coding special patterns—this is the natural behavior of the
compile-time proof pipeline. Each type-checked call site sends to the pipeline:

```
{background assumptions} ⇒ {verification target}
```

The proof pipeline judges implication. `Proved` → pass; `Disproved` → compile error +
counterexample; `Unproven` → compile error + unresolved proposition. Background assumptions come
from the path conditions of the current program point.

#### 3.3 Assumption Stack

When the compiler analyzes control flow, it maintains an assumption set for each basic block:

- **if-guard**: `if y > 0` → true branch pushes `y > 0`, false branch pushes `!(y > 0)` (if `else`
  is used)
- **match pattern**: `if let Some(v) = opt` → inside the branch, push `opt == Some(v)`
- **logical conjunction**: `if x > 0 and y < 10` → inside the branch, push `x > 0` and `y < 10`
- **function preconditions**: when calling `divide(a, b)`, evidence that `b` satisfies `Positive`
  must come either from the current assumptions or from the refined type annotation on the actual
  argument itself (if `b` is annotated as `Positive`, its type carries `b > 0`)
- **assignment**: when `let z = y`, refinement conditions already on `y` are propagated to `z`

All assumptions enter the compile-time proof pipeline. When entering the SMT-accelerated path, they
are translated into SMT-LIB background assertions.

#### 3.4 No Static Evidence, Then Compile Error

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

> Unable to prove parameter `b` satisfies `Positive` in the call to `divide`. `y` comes from
> function input with no proven bound. Consider guarding the call with an if branch:
> `if y > 0 { divide(x, y) }`.

YaoXiang does not accept runtime values directly entering refined type arguments without providing
static evidence. This is not a limitation—this is the core of the hard-safety philosophy. Any code
the compiler cannot statically prove shall not pass compilation.

#### 3.5 Relationship with the Unified Pipeline

Path condition propagation is not an additional mechanism. It is the direct extension of the
compile-time proof pipeline in control flow analysis:

| Stage                      | Responsibility                                                                                                                                               |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Path condition collection  | Compiler's control flow analysis phase; annotates each basic block with assumption sets                                                                      |
| Verification condition gen | When a type constraint requiring verification is encountered, merges path conditions + actual argument type information                                      |
| Proof pipeline evaluation  | Compiler kernel → SMT acceleration → obtain Proved / Disproved / Unproven                                                                                    |
| Result                     | `Proved` → pass; `Disproved` → compile error + counterexample; `Unproven` → compile error + unresolved proposition (programmer can provide a proof function) |

No new components. No special rules. Path conditions are the background knowledge of the proof
pipeline—sharing the same pipeline and the same budget system as type equality and borrow
constraints.

### 4. Compile-Time Proof Pipeline

All compile-time checks share the same pipeline. The core operation of the pipeline is **type
checking**—checking whether the type of a proof term equals the proposition to be proven. Everything
is type checking.

```
Compile-time encounters a Bool expression needing evaluation (i.e., needing to construct a proof term)
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
        └── Hoare logic implication (P ⇒ Q)
            → compiler + SMT acceleration module
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
         Type checker verifies ──→ Proved ──→ compilation passes
                  │
                  ▼
            Verification failed → compile error: "proof does not hold"
```

#### 4.1 Proof Result: A Three-Valued Algebra

Compile-time evaluation returns three results—an inevitable conclusion of the halting problem, and a
natural division of proof theory:

```
eval_compile_time : BoolExpr → Proved | Disproved(Model) | Unproven
```

- **Proved** → halts, proof term constructed, type check passes. Compilation continues.
- **Disproved(M)** → halts, counterexample M exists. Compile error + counterexample + source
  location.
- **Unproven** → proof not constructed within the given resource limit. Compile error + unresolved
  proposition + budget consumption report.

**Unproven ≠ False.** The compiler saying "I cannot prove it" is not equivalent to the proposition
being false—just that it exceeds the capability of current automatic proving. This is honesty, not a
defect.

The hard budget limit is the engineering solution to the halting problem. No knob—if we provided
one, we'd be asking the user "do you think your program will halt," and neither the user nor the
compiler knows.

#### 4.2 After Unproven: The Programmer Writes the Proof

When the compiler returns `Unproven`, the programmer can write a **proof function**—simply a
YaoXiang function whose return type equals the proposition to be proven. The type checker verifies
this function—the exact same mechanism as verifying `add(a, b): Int`.

```
Proposition = type
Proof       = program (a value of that type)
Verification = type checking (the sole trust root)
```

The SMT solver is not an independent trust boundary—it is an **acceleration module of the type
checker**. SMT helps find proofs, but the proof is always verified by the type checker. When SMT
returns `unsat`, the compiler reconstructs its result into a proof term verifiable by the type
checker. If reconstruction fails (SMT's reasoning steps exceed the compiler kernel's inference
rules), it falls back to `Unproven`—the programmer can manually write a proof function.

```yaoxiang
# Proposition: a refined property the compiler cannot automatically prove
FirstIsMin: (T: Ord, arr: Sorted(T)) -> Type = {
    forall i in 0..arr.len: arr[0] <= arr[i]
}

# Proof: the programmer writes a function whose return type is the proposition above
# The type checker verifies this function—identical to verifying add(a,b): Int
first_is_min: (T: Ord, arr: Sorted(T)) -> FirstIsMin(T, arr) = {
    # compiler verifies here: type of function body = FirstIsMin(T, arr)
    ...
}
```

No AI required, no export to Coq required, no new concepts. **Properties the compiler cannot
automatically prove at compile time → programmer writes proof in YaoXiang code → type checker
verifies.** The whole process is a smooth gradient—the compiler does the easy proofs and leaves the
hard ones to your brain.

#### 4.3 Hierarchical Dependencies within the Pipeline

The above evaluators share the same interface but have an evaluation order. Type equality is the
prerequisite for all subsequent analysis; ownership/token checking depends on type information;
refined predicate verification depends on the results of the first two layers. The compiler
evaluates layer by layer; expressions that fail at lower layers do not enter upper layers—avoiding
wasting solving budget on type-incorrect programs.

```
Evaluation order (same pipeline, layered scheduling)
├── Layer 0: type equality (T1 == T2)
│   └── structural unification → failure makes subsequent meaningless, return Disproved directly
├── Layer 1: ownership/token conflicts
│   └── flow-sensitive liveness analysis → failure makes memory safety invalid, return Disproved directly
└── Layer 2: refined predicate / Hoare implication
    └── compiler itself → SMT acceleration → obtain Proved / Disproved / Unproven
```

Each layer still returns `Proved/Disproved/Unproven`, sharing the same interface and the same budget
system.

### 5. Unification of Three-Layer Functions

| Layer                  | Timing       | Input      | Output | Example                                        |
| ---------------------- | ------------ | ---------- | ------ | ---------------------------------------------- |
| Value-level function   | Runtime      | values     | value  | `add: (a: Int, b: Int) -> Int = a + b`         |
| Type constructor       | Compile-time | type/value | Type   | `List: (T: Type) -> Type = { data: Array(T) }` |
| Compile-time predicate | Compile-time | value      | Type   | `Positive: (x: Int) -> Type = { x > 0 }`       |

All use the same `name: type = value` syntax. Compile-time predicates and type constructors go
through the same compile-time proof pipeline—`{}` is the proof space.

### 6. Loops: Floyd-Hoare Verification Condition Generation

Loops do not need separate `: Invariant(...)` or `: decreases(...)` annotations. The compile-time
predicate type annotations on variables define Floyd-Hoare-style assertions—the compiler generates
verification conditions from the type annotations, and the proof pipeline checks whether each
assignment maintains the type.

Core mechanism: each assignment operation corresponds to a Hoare triple `{P} x := e {Q}`, and the
verification condition is `P ⇒ Q[e/x]`. The compiler generates one verification condition for the
loop body—after the proof pipeline verifies the inductive step holds, all iterations are
automatically covered.

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
        #   compiler + SMT: linear arithmetic, millisecond-level → Proved
        #
        # i += 1:
        #   i changes → dependency graph shows s's type annotation references i → triggers re-verification
        #   new verification target: s satisfies SumUpTo(arr, i_new)
        #   i.e., s == sum(arr[0..i_new]), guaranteed by the previous step → Proved
        s += arr[i]
        i += 1
    }
    return s  # at this point s: SumUpTo(arr, arr.len), i.e., s == sum(arr[0..arr.len])
}
```

The loop invariant is the type annotation on the variable—the programmer writes the type, the
compiler checks the inductive step. The compiler does not need to "discover" invariants, nor
"automatically perform induction"—it decomposes the inductive proof into local verification
conditions for each assignment, and lets the proof pipeline divide and conquer.

#### 6.1 Dependency Tracking: Dependent Types on Mutable Variables

The prerequisite for the above mechanism is: the compiler knows that `s`'s type annotation
`SumUpTo(arr, i)` references `i`—when `i` changes, the type constraint of `s` changes accordingly.
This requires the compiler to maintain a **type dependency graph between variables**.

**Data structure**:

```
TypeDepGraph: Map<VarName, Set<VarName>>
# Key is the depended-on variable, value is the set of variables whose type annotations reference that variable
# Example: { i: {s}, j: {s, t}, ... }
```

**Construction**: When the type checker processes `mut v: Pred(... x ...) = init`, it parses free
variable references in the `Pred(...)` arguments. If the arguments reference another mutable
variable `x` in the current scope, the dependency graph records `x → v`.

**Triggering**: When the depended-on variable `x` is assigned, the compiler:

1. Looks up all variables in the dependency graph that depend on `x`: `{v₁, v₂, ...}`
2. For each `v`, generates a verification condition: does `v`'s current value satisfy the updated
   type `Pred(... x_new ...)`?
3. Sends the VC into the proof pipeline

**Assignment-order sensitive**: Dependency tracking naturally enforces correct assignment order.
Take `SumUpTo(arr, i)` as an example:

```yaoxiang
# correct order
s += arr[i]   # s_new satisfies SumUpTo(arr, i+1)
i += 1        # i changes → re-verify s satisfies SumUpTo(arr, i_new) → True

# wrong order—compiler rejects
i += 1        # i changes → re-verify s satisfies SumUpTo(arr, i_new)
              # s not yet updated, s_old == sum(arr[0..i_old]) ≠ sum(arr[0..i_new])
              # → compile error: variable s does not satisfy type SumUpTo(arr, i_new)
s += arr[i]   # unreachable
```

**Composed dependencies**: A variable can depend on multiple variables. The type annotation
`{ v: Int; v == x + y }` depends on both `x` and `y`—either change triggers re-verification.

**Relationship with the proof pipeline**: Dependency tracking is a trigger for VC generation, not an
independent verification mechanism. It answers "when do we need to generate a VC"—the proof pipeline
answers "does the VC hold."

### 7. Termination Checking

**Scope: compile-time evaluation.** Termination checking only targets loops and recursion involved
in compile-time evaluation (triggered by dependency tracking)—code the compiler itself executes must
be provably total; runtime code does not require termination proofs (§6.7 scope table). Within
scope, compile-time fully automatic takes priority: loops the compiler can prove pass; loops the
automatic strategy cannot prove directly cause a compile error—the programmer must let the compiler
be able to automatically analyze the loop's termination. No **annotation syntax** escape hatch (no
`decreases`-style annotation; what is unsupported is forever a syntax slot, not a function).

> **Revision (RFC-027a, 2026-09-14)**: ① The scope is clarified as compile-time evaluation—the
> original text did not limit the position of "loops," creating ambiguity with the §6.7 scope table;
> unconditional total checking at the implementation layer is narrowed per this. ② For termination
> obligations the automatic strategy cannot prove, the programmer may provide a measure function and
> a termination proof function (`Terminates`) as fallback—both are ordinary YaoXiang functions, zero
> new syntax, and the type checker verifies them. The fully-automatic-first principle is unchanged;
> fallback only occurs after `Unproven`. See
> [RFC-027a: Proof Function Fallback for Termination Checking](../draft/027a-termination-proof-fallback.md).

#### 6.1 Design Principles

The compiler automatically extracts information needed for termination proofs from two sources:

1. **Variable type annotations**: bound constraints in refined types (e.g., `UpTo(n)` provides upper
   bound `n` and lower bound `0`)
2. **Loop body operations**: operations applied to variables each iteration

The compiler tries four measure synthesis strategies in order of priority; finding one stops the
search.

#### 6.2 Strategy 1: Automatic Linear Rank Function Synthesis

When a variable has a linear bound annotation, the compiler enumerates candidate linear measures and
verifies them with SMT.

```
Input:
  variables v₁: UpTo(u₁), v₂: UpTo(u₂), ... (variables with lower and upper bounds)
  loop condition cond
  set of assignments in loop body

Algorithm:
  1. Extract bounds for each variable from type annotations: [low_i, high_i]
  2. Enumerate candidate measures: v_i, u_i - v_i, v_i - v_j, etc. linear combinations
  3. For each candidate measure m:
     - SMT verifies m ≥ 0 (derived from type bounds)
     - For each execution path in loop body, SMT verifies m' < m (strictly decreasing)
  4. Find a qualifying linear combination → termination proven
```

Coverage: any loop where a variable is assigned a linear expression (`v = a·v + b`) and has a
bounded type annotation. Including `i += const`, `i -= const`, and binary-search-style interval
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

#### 6.3 Strategy 2: Predicate Violation Count—Automatic Measure Extraction from Target Type <span style="color:orange">[Experimental Strategy]</span>

> ⚠️ **Current status: experimental strategy, whether to include it depends on actual feasibility
> when Phase 3 is implemented.** This strategy is effective for adjacent swap operations (bubble
> sort, insertion sort), but cannot automatically prove non-adjacent operations (quicksort
> partition, heapsort sift-down). See the coverage boundary table below. If Phase 3 validation
> proves infeasible, this strategy will be removed or downgraded to future work.

Core insight: **the specifications the user writes are the raw material for compiler reasoning.**
The compiler does not need to have "what is sorting" built in—it reads the `Sorted` definition and
automatically extracts the measure from it.

```
Input:
  target type: Sorted(arr) = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }
  loop body operations: adjacent element swaps

Algorithm:
  1. Parse the predicate definition: forall i in range: cond(i, arr)
  2. Auto-generate measure: violation_count = |{ i | ¬cond(i, arr) }|
  3. Analyze the effect of operations on the measure:
     - adjacent swap arr[j], arr[j+1] = arr[j+1], arr[j]
     - only affects pairs at indices j-1, j, j+1
     - if arr[j] > arr[j+1] (predicate violated), after swap this pair satisfies the predicate
     - violation_count decreases by at least 1
  4. Upper bound: n·(n-1)/2 (maximum adjacent inversions), lower bound: 0
  → termination proven
```

**Current coverage**:

| Algorithm      | Operation pattern | Strategy 2 provable? | Reason                                                |
| -------------- | ----------------- | :------------------: | ----------------------------------------------------- |
| Bubble sort    | Adjacent swap     |          ✅          | violation_count strictly decreases each swap          |
| Insertion sort | Adjacent move     |          ✅          | each shift eliminates one violated pair               |
| Selection sort | Non-adjacent swap |          ❌          | single swap may increase violation_count              |
| Quicksort      | partition         |          ❌          | non-adjacent swap, not monotonically decreasing       |
| Heapsort       | sift-down         |          ❌          | tree-shaped operations, violation_count non-monotonic |

**Complementary strategies**: For quicksort, the `low < high` interval shrinking can be covered by
Strategy 1 (linear rank function)—the outer partition recursion halves the interval each time.
Strategy 1 and Strategy 2 are complementary; the termination of most practical algorithms can be
proven by one of them. But the generalization of Strategy 2 (non-adjacent operations, tree-shaped
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

`v += const` (positive constant), variable has an upper-bound type annotation → measure
`upper_bound - v` decreases by `const` each time, lower bound 0. This is a degenerate case of
Strategy 1, handled quickly by the compiler at the front.

#### 6.5 Strategy 4: Multiplicative Scaling Measure Templates

`v *= const` (const > 1), variable has lower- and upper-bound type annotations. The compiler has
built-in logarithmic measure templates `ceil(log_const(upper/v))`; multiplying by `const` each time
decreases the measure by 1.

```yaoxiang
mut i: Positive(i) = 1
while i < n {
    # compiler auto-derives: measure ceil(log₂(n/i)), each multiplication by 2 decreases measure by 1
    i *= 2
}
```

#### 6.6 Separation of Termination and Correctness

Termination proof and correctness proof are independent:

- **Termination**: the four strategies above automatically prove that the loop exits in a finite
  number of steps
- **Correctness**: whether the loop body progresses toward the target type, checked by the
  compile-time proof pipeline through verification conditions

Both pass → compilation passes. Termination proven but correctness fails → compile error +
counterexample. Correctness proven but termination cannot be proven → compile error pointing to the
unanalyzable variable or operation. Both fail → compile error reporting both failure reasons
separately.

#### 6.7 Termination Checking of Recursive Functions

For recursive functions that need to be evaluated at compile time, the compiler checks for argument
decrease:

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)  # compiler analyzes: n-1 < n → decreases → terminates
}

# Compile-time use—the compiler guarantees factorial terminates at compile time
vec: Vec(factorial(5)) = Vec(120)()  # 5! = 120, completed at compile time
```

| Scenario                                                  | Behavior                    |
| --------------------------------------------------------- | --------------------------- |
| Compiler can analyze the recursion decrease (e.g., `n-1`) | Compile-time evaluation     |
| Not decreasing / cannot determine decrease                | Compile error               |
| Runtime call (not at type position)                       | No termination check needed |

#### 6.8 Hard Boundary

`i = f(i)` where `f` is non-invertible, non-closed, does not preserve any
monotonicity—mathematically impossible to automatically prove termination. Compile error:

> This loop cannot be automatically proven to terminate. The loop variable depends on the
> unanalyzable function `f`. Please use an iteration pattern that the compiler can analyze, or
> provide a measure function and a termination proof function (see RFC-027a).

This is not a compiler failure. Any code that cannot be statically proven safe shall not pass
compilation.

### 8. SMT Solver: An Acceleration Module of the Type Checker

The SMT solver is conventionally an external tool in traditional languages (e.g., F\* calls Z3,
Dafny calls Z3). In YaoXiang, it is an **acceleration module of the type checker**—invoked only when
the compiler kernel itself cannot directly determine. SMT helps find proofs, but the type checker
verifies the proof.

**Trust model**: The type checker is the sole trust root. The SMT solver is an acceleration
module—it helps find proofs, but SMT is not an independent trust boundary. The compiler trusts Z3's
`unsat` results (consistent with the F\*/Dafny route—the probability of Z3 being wrong is lower than
the bug rate of the compiler itself; this is a pragmatic engineering choice). The real unreliability
is controlled at the SMT translation layer—if the translation has a bug, the compiler will surface
it in other tests.

**Interface**: The compiler internally translates to the SMT-LIB 2.6 standard format, not bound to a
specific solver API. SMT-LIB is an ISO standard, natively supported by Z3, CVC5, MathSAT, Yices.

**Default backend**: Z3 (MIT license, broadest documentation and community validation). CVC5 as an
SMT-LIB-compatible alternative—users can switch at compile time via a compiler flag.

No "general solver abstraction layer"—SMT-LIB is the abstraction layer. If CVC5 makes a breakthrough
in a specific theory in the future, switching only requires swapping the binary, not changing
compiler code.

```
Compile-time Bool expression
        │
        ├── Compiler kernel can directly determine (structural equivalence, simple arithmetic,
        │   trivial formulas after constant folding)
        │   → return Proved / Disproved directly
        │
        └── Compiler kernel cannot directly determine (quantifiers, symbolic variables)
            → dependent type pre-reduction (factorial(5) → 120)
            → translate to SMT-LIB format
            → send to Z3/CVC5 (with budget limit)
            → return value: unsat → Proved  │  sat + model → Disproved  │  unknown → Unproven
```

**Solving budget—hard limit, like stack depth**:

| Budget dimension               | Default | Description                                                                                                                                                   |
| ------------------------------ | ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Solving steps                  | 10,000  | Z3 typically returns within a hundred steps for linear arithmetic. 10,000 steps cover 99% of practical predicates.                                            |
| Time                           | 100ms   | A single predicate over 100ms = the user is writing a compile-time program rather than a type annotation. 100ms × 50 predicates = 5 second compilation limit. |
| Quantifier instantiation depth | 3       | Three nested quantifiers covers practical patterns. More than three layers is likely writing logic exercises.                                                 |

Exceeding the budget returns `Unproven`; compile error + predicate location + consumption. No
degradation, no runtime check, no silent pass.

**Why this is actually feasible**: 95% of practical predicates in engineering are linear
arithmetic—`x > 0`, `arr.len > 0`, `0 <= idx < arr.len`—all within decidable fragments, and SMT
solvers return in milliseconds for such problems. For the rare complex predicates that exceed the
budget, the programmer writes a proof function.

Dependent types do a layer of pre-reduction before SMT calls: `factorial(5)` is directly evaluated
at compile time to `120`; `append([1,2], [3])` is directly evaluated to `[1,2,3]`. These
deterministic value calculations do not consume SMT budget.

Programmers do not need to know SMT exists. The mental model is: **the compiler can prove it, it
passes; if it cannot, it errors—if the compiler cannot, you can write a function to prove it to the
compiler.**

### 9. Compile-Time Predicate Composition

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

### 11. dispatch Distribution Pipeline: Unified Dispatching of Compile-Time and Runtime

`assert` and `Assert` are two sides of the same refined type primitive. The distribution pipeline
`dispatch` automatically decides between compile-time proof and runtime checking based on **whether
the predicate's free variables are reachable at compile time**:

| Criterion                                                                                    | Mode            | Behavior                                                                                 |
| -------------------------------------------------------------------------------------------- | --------------- | ---------------------------------------------------------------------------------------- |
| All free variables are compile-time known (generic parameters, compile-time constants)       | **CompileTime** | Enter proof pipeline: Proved → erase, Disproved → compile error, Unknown → require proof |
| Some free variables come from runtime (function parameters, external input, `mut` variables) | **Runtime**     | Insert runtime check, inject refined facts into flow-sensitive assumption set Γ          |

**Key**: "Cannot determine" ≠ "disproved". In CompileTime mode, `Unknown` requires a proof (no
silent downgrade); in Runtime mode, the proposition has no truth value at compile time at all—no
matter how strong a prover is, it cannot write a universally true proof for "the user might have
input a negative number"; a runtime check is the only sound choice. This is not a prover being
insufficiently strong; it is a theoretical necessity.

### 12. Flow-Sensitive Assumption Set Γ: Strongest Postcondition Propagation

The compiler maintains a flow-sensitive assumption set Γ, tracking which propositions are known to
hold at each control flow point.

**SP (Strongest Postcondition) propagation**:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
```

**`mut` variable kill set**: After a `mut` variable is reassigned, all assumptions involving that
variable are removed from Γ:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
mut x = x - 5       // Γ = {}  ← x > 0 is killed
```

This is a hard requirement of soundness—the variable's value has changed, old assumptions are
invalid.

**Branch merge**: When IF/ELSE or match branches merge, Γ takes the intersection of the assumptions
of each branch. Only propositions that hold on all paths are carried out of the branch.

### 13. Clarification of Erasure Model: witness Erasure ≠ check Erasure

RFC-027's claim that "refined types are **fully erased** at runtime" refers to the **proof witness
(proof token)**—proof terms verified at compile time generate no runtime code. But the **runtime
check** inserted by the dispatch in Runtime mode is retained—it is a Bool check executed at the
value level, not a type-level witness.

Summary: witnesses are erased, checks are retained. The two are not in conflict, and the original
RFC-027 claim stands unchanged.

## Detailed Design

### Syntax Changes

| Before (RFC-022)                      | After (This RFC)                                                             |
| ------------------------------------- | ---------------------------------------------------------------------------- |
| `//! requires: NonEmpty(n) = n > 0`   | Compile-time predicate as parameter type `(b: Positive(b))`                  |
| `//! ensures: ExistsMax(result, arr)` | Return type uses return value parameter `-> (result: IsMax(T, arr, result))` |
| `/*! invariant: ... !*/`              | Compile-time predicate type annotation on variable—Floyd-Hoare invariant     |
| `//! decreases: n`                    | Compiler fully automatically derives measure function                        |
| Specification as comment              | Specification as type system                                                 |

### Syntax

**No new keywords for compile-time predicates.** `{}` is the proof space, completely consistent with
existing type definition syntax. A compile-time predicate is a function that returns
`Type`—`name: (params) -> Type = { assertions }`. Usage is a function call—`Positive(b)`,
`IsMax(T, arr, result)`.

```bnf
# compile-time predicate = function returning Type, {} contains compiler-verified assertions
# uses existing function/type syntax, no new BNF rules needed
predicate ::= identifier ':' params '->' 'Type' '=' '{' assertions '}'
```

**The actual arguments of predicate applications must be in compile-time constant form**—literals,
variables (bound by name), or single-parameter type applications (recursively extracted). Actual
arguments in non-convertible constant form raise **E1092**; actual argument count not matching the
predicate parameters raises **E1093**—refinement constraints are **never silently dropped**
(previously, non-convertible actual arguments would cause constraints to vanish silently, allowing
bindings that violate constraints to pass silently).

**New syntactic concept: return value parameter**—in `-> (name: Type)`, `name` is the return value
parameter.

The return value parameter is the **only syntactic concept** YaoXiang introduces on top of existing
function syntax. Its semantics:

- The value of `name` is provided by the `return` statement
- `name` exists only in the type signature, referenced by postcondition predicates (e.g.,
  `-> (result: IsMax(T, arr, result))`)
- `name` does not enter the function body scope, does not appear on the caller side
- Return value parameter is **optional**—when there is no postcondition, the signature is identical
  to a normal function (`-> Int`), with no extra burden

Reason for introducing it: postconditions need to reference "the value the function is about to
return". Without a return value parameter, the compiler can only let predicates reference the return
value through special rules (such as the implicit variable `$result` or `__retval__`). The return
value parameter makes this reference explicit—it is just a formal parameter, except that the value
is provided by `return` rather than the caller.

**Proof functions** are not a new concept—they are just YaoXiang functions whose return type is the
proposition being asserted. When the compiler returns `Unproven`, the programmer provides a proof
function, and the type checker verifies it in exactly the same way as it verifies the return type of
any function. No new syntax, no new keywords, no new rules.

### Type System Impact

- **Type universe**: Compile-time predicates sit at the Type₂ layer—functions that accept values and
  return `Type`, at the same level as type constructors
- **Generics interaction**: Compile-time predicates may carry generic parameters, e.g.,
  `NonEmpty: (T: Type) -> (arr: Array(T)) -> Type`
- **Ownership interaction**: Expressions in compile-time predicates obey ownership rules, can only
  read, not write
- **Type inference**: Arguments of compile-time predicates participate in HM type inference

### Runtime Representation

Compile-time predicates are handled at runtime **according to the dispatch result**:

- **CompileTime mode** (all free variables are compile-time known): the witness is fully erased
  after proof passes. `Positive: (x: Int) -> Type = { x > 0 }`—parameter `b: Positive(5)` is
  represented as `Int` at runtime. The refinement condition `{ 5 > 0 }` has passed and is erased.
- **Runtime mode** (some free variables are runtime): retain the runtime check—execute a Bool check
  at the value level, inject into the flow-sensitive assumption set Γ. See §11 dispatch distribution
  pipeline and §13 erasure model clarification for details.

Placing a compile-time predicate at a type position (e.g., `f(x: Positive(x))`) does not generate a
wrapper type, does not allocate extra memory. But when `x` comes from runtime input, **a** runtime
Bool check **will** be inserted.

**Interaction constraint with `ref`**: Compile-time predicates can only reference values that are
immutably borrowed or whose ownership has been transferred. Compile-time predicates referencing
mutably borrowed values cannot guarantee at compile time that the verification result still holds at
runtime—such usages directly raise a compile error.

### Compiler Changes

1. **Parser**: Compile-time predicates use standard function syntax; no additional parsing rules
   needed
2. **Compile-time proof pipeline**: unified `Proved/Disproved/Unproven` return interface, automatic
   strategy selection
3. **SMT acceleration module**: SMT-LIB 2.6 translation layer, default backend Z3, CVC5 alternative
4. **Type checker kernel**: inference rule implementation—structural equivalence, βδι-reduction,
   universal quantifier introduction/elimination. This is the sole trust root; both SMT and
   programmer proofs are verified through this
5. **Verification condition generation**: WP/SP calculus + loop invariant proof obligations
6. **Error reporting**: counterexample formatting + unresolved proposition reports + source location
   association

### Backward Compatibility

- ✅ Code that does not use compile-time predicates is completely unchanged
- ✅ Compile-time predicates have zero runtime overhead in CompileTime mode; only necessary Bool
  checks are retained in Runtime mode
- ⚠️ RFC-022's `//!` syntax is no longer supported—but 022 was never implemented, so there is no
  migration burden

## Trade-offs

### Advantages

- **Full realization of the Curry-Howard isomorphism**: types are propositions, programs are proofs,
  `name: Proposition = Proof`
- **Unity**: compile-time predicates and ordinary functions use exactly the same syntax, with no
  conceptual split
- **SMT transparency**: programmers do not need to know SMT exists; the mental model is consistent
  with type checking
- **Progressive adoption**: can start from one compile-time predicate, gradually increase coverage
- **Minimum runtime overhead**: zero overhead in CompileTime mode; only necessary Bool checks
  retained in Runtime mode

### Disadvantages

- **Compile time**: SMT solving increases compile time, but the hard budget limit guarantees a
  controllable upper bound
- **Boundaries of automatic proving**: complex predicates beyond first-order linear arithmetic may
  require the programmer to write proof functions. This is not a language defect—this is an
  inevitable conclusion of the halting problem. The compiler honestly reports `Unproven` rather than
  falsely reporting `True/False`
- **Learning curve**: writing effective compile-time predicates and proof functions requires
  understanding the basic intuition of the Curry-Howard isomorphism
- **Implementation complexity**: unifying the compile-time proof pipeline requires careful design

### Risk Mitigation

- SMT solving budget hard limit (steps 10,000 / time 100ms / instantiation depth 3); exceeding
  budget returns `Unproven`
- Dependent type pre-reduction: deterministic value calculations are consumed first; SMT only chews
  on the non-deterministic parts
- `Unproven` is not a dead end: the programmer can write a proof function, verified by the type
  checker—consistent with verifying any function's return type
- Incremental verification: only modified modules are verified
- Clear error messages + counterexample display + budget consumption report + unresolved
  proposition + suggestions (if the compiler can provide them)

## Alternatives

| Alternative                                                        | Why not chosen                                                                                                                                                             |
| ------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| RFC-022: `//!` comment-style specifications                        | Specifications and types split, violating the Curry-Howard isomorphism                                                                                                     |
| Independent specification files (e.g., CVL)                        | Specifications separated from code, increasing maintenance cost                                                                                                            |
| Runtime-only assertions                                            | Cannot statically guarantee correctness                                                                                                                                    |
| External proof assistant (e.g., Coq)                               | Disconnected from the compiler, requires independent proof language and trust boundary. YaoXiang's choice: proof is YaoXiang code, the type checker is the sole trust root |
| **This proposal: compile-time predicates as first-class citizens** | ✅                                                                                                                                                                         |

## Implementation Strategy

### Phase Division

| Phase       | Content                                                                                                                                                                |
| ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Phase 1** | Compiler kernel: structural equivalence + βδι-reduction + universal quantifier introduction/elimination. Support simple arithmetic predicates (`x > 0`, `arr.len > 0`) |
| **Phase 2** | SMT-LIB translation layer + Z3/CVC5 integration. Pipeline returns `Proved/Disproved/Unproven`. Support programmer writing proof functions on `Unproven`                |
| **Phase 3** | Loop invariant VC generation + termination checking (linear rank function + predicate violation count + bounded patterns + combinatorial explosion control)            |
| **Phase 4** | Incremental verification + caching + IDE support                                                                                                                       |

### Dependencies

- RFC-010: Unified Type Syntax — compile-time predicates are based on `name: type = value`
- RFC-011: Generics System — compile-time predicates may carry generic parameters
- RFC-009: Ownership Model — compile-time predicate expressions obey ownership rules

## Open Issues

- [x] **SMT solver selection**: default Z3 (MIT license, most widely validated). CVC5 as an
      SMT-LIB-compatible alternative, switchable via a compiler flag. The compiler's internal
      translation target is the SMT-LIB 2.6 standard format—SMT-LIB is the abstraction layer; no
      custom general solver interface.
- [x] **Specific budget values**: steps 10,000 / time 100ms / quantifier instantiation depth 3.
      Fixed inside the compiler, no knob. If real use cases prove insufficient in actual practice
      (not "the user wrote it wrong"), adjust later.
- [x] **Quantifier support scope**: the language does not limit the order of quantifiers.
      Compile-time predicates accept `Type` parameters—`Type` includes function types—therefore
      higher-order quantifiers are a natural corollary of the type system and require no special
      syntax. The SMT solver can automatically determine first-order quantifiers (forall/exists,
      supports interleaved nesting, limited by the budget depth of 3). Higher-order quantifiers: SMT
      returns `Unproven`, the compiler hints "this predicate is outside the automatic proving scope,
      please provide a proof function." The programmer writes a YaoXiang function whose return type
      equals the proposition—the type checker verifies that function. No external export, no AI, no
      interactive proof mode required. Everything is YaoXiang code; everything is verified by the
      type checker.
- [x] **Counterexample formatting**: source variable names are used directly as SMT variable names
      (with module prefix to avoid conflicts). When the Z3 model returns, look up by variable name.
      Output format: variable name = concrete value + source location + predicate definition
      location. No complex mapping layer.
- [x] ~~**Interaction between `ref` smart pointers and compile-time predicates?**~~ → Decided:
      compile-time predicates only allow immutably borrowed values or values whose ownership has
      been transferred. Mutably borrowed values cannot appear in compile-time predicates.
- [x] **Extension of the `forall` predicate violation count measure to non-adjacent operations?** →
      Not extending. Current coverage (adjacent swap, adjacent move) is complemented by Strategy 1
      (linear rank function)—quicksort's outer interval shrinking is covered by Strategy 1, heapsort
      is covered by Strategy 1 (array index pattern). Loops that cannot be proven to terminate by
      any strategy cause the compiler to error directly—this is the hard-safety philosophy, not a
      defect. If in the future there is a real scenario (not an academic construction) algorithm
      that the four strategies cannot cover, we will revisit. → **Revisit triggered (2026-09-14,
      #318)**: non-structural recursion (gcd-style non-directly-decreasing, mutual recursion, merge
      partition) is exactly the real scenario anticipated by this clause—the current pipeline cannot
      automatically prove their termination, and they cannot be rewritten into analyzable iteration
      patterns without destroying readability. Conclusion: fully-automatic-first unchanged; add
      proof function fallback after `Unproven`—both measure and termination proof are ordinary
      YaoXiang functions, zero new syntax, type checker verifies. Landed in
      [RFC-027a: Proof Function Fallback for Termination Checking](../draft/027a-termination-proof-fallback.md).
- [x] **Linear rank function enumeration combinatorial explosion**: candidate enumeration limit is 3
      bounded variables. ≤3: enumerate all linear combinations and verify each with SMT. >3: only
      try single-variable measures (`v_i`, `u_i - v_i`); on failure, directly report a compile
      error—prompt the programmer "the loop has >3 bounded variables, the compiler cannot
      automatically synthesize a multi-variable measure." This is not an engineering compromise—it
      is forcing the programmer to write simpler loops.

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
│ (formal     │    │  (kept in   │
│  design)    │    │   place)    │
└─────────────┘    └─────────────┘
```
