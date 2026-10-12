---
title: 'RFC-027: Compile-Time Predicates and Unified Static Verification'
status: 'Accepted'
author: 'Chenxu'
created: '2026-06-07'
updated: '2026-10-03'
impl_status: 'in-progress'
impl_detail:
  'Phase 1-2 completed, Phase 3 partially completed, Phase 4 partially completed. The assert/Assert
  unification plan (6 phases) is fully implemented (issues #157-#162 closed): Never type, IsTrue
  bridging, flow-sensitive Γ + kill set, type-level recursion, universe hierarchy weak check. The
  dispatch semantics in §11 are implemented via call-site obligations (three direct
  `check_predicate` calls in `checker.rs`: binding site / call-site argument / return position), but
  the standalone `layers/dispatch.rs` module has been deleted — it had zero production call sites,
  and its handling of `Unproven` (downgrading to W1080 warning) conflicted with the production
  implementation (always an error) (#377-2).'
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
> — Deprecated

## Summary

This RFC proposes introducing **compile-time predicates** as first-class citizens in YaoXiang,
unifying all compile-time static verification into a single **proof pipeline**. Compile-time
predicates are not external specification comments — they are simply functions. A function that
returns `Type` can be used at a type position, and the compiler calls it at compile time and checks
the return value. Types are propositions, compile-time evaluation is proof.

**Core argument**: The only job of compile-time type checking is to construct and verify proof
terms. Type equality, token conflicts, dependent type reduction, compile-time predicate evaluation,
Hoare logic implications — all are different type checks in the same compile-time proof pipeline,
sharing one pipeline. The SMT solver is an acceleration module of the type checker, not an
independent trust boundary. When the compiler returns `Unproven`, the programmer writes a YaoXiang
function as proof — the type checker verifies it exactly the same way it verifies any function's
return type. Everything is YaoXiang code, everything is verified by the type checker.

## Motivation

### Why Deprecate RFC-022?

RFC-022 designs specifications as `//!` comments:

```yaoxiang
max: (T: Ord) -> ((arr: Array(T, n)) -> T) = {
    //! requires: NonEmpty(n) = n > 0          ← This is a comment independent of types
    //! ensures: ExistsMax(result, arr[0..n])   ← This is a comment independent of types
}
```

This commits a fundamental error against the Curry-Howard correspondence: **splitting specifications
and types into two layers**. Comments are not types. Comments do not participate in type checking.
The mental model is that of an "external tool".

The white paper is clear:

> "There are no `//!` comments. There is no independent specification language. Everything is within
> the type system."

### Current Problems

- RFC-022's `//!` comments are external syntax independent of the type system
- Specification types and ordinary types are two separate systems, causing conceptual redundancy
- The Debug Build verification / Release Build ignore split model breaks unity
- The SMT solver is positioned as an external tool in conventional understanding — YaoXiang builds
  it into the type checker as an acceleration module
- Type checking, borrow checking, compile-time predicate checking, and macro expansion each take
  different paths

### The Correct Mental Model

Type checking can be abstracted as a function:

```
verify : Program → Proved | Disproved(Model) | Unproven
```

All compile-time checks — simple type matching, borrow conflict detection, compile-time predicate
verification — are subtasks of this function. They share the same proof pipeline, differing only in
proof term complexity and construction strategy.

When the compiler returns `Unproven`, the programmer provides a proof function — its return type
equals the proposition to be proved. The type checker verifies it. This is the same operation as
ordinary type checking.

## Proposal

### 1. `{}` Is the Proof Space: Types Are Assertions, Verification Is Type Checking

YaoXiang's `{}` is the compile-time proof space. Everything inside is an assertion, and the compiler
guarantees each is True — either proven automatically, or by a proof function provided by the
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
#          Parameters in         Only assertions in {}
#          signature            The compiler verifies x > 0 at compile-time call

List: (T: Type) -> Type = { data: Array(T) }
#      ^^^^^^^^              ^^^^^^^^^^^^^^^
#      Parameters in        The compiler verifies type_of(T) == Type, type_of(data) == Array(T)
#      signature
```

Same pattern: `name: (params) -> Type = { assertions }`. The compiler doesn't distinguish "type
assertions" from "value assertions" — they are all evaluation targets in the proof pipeline.

**Loop invariants don't need to be written separately. Type annotations on variables are Floyd-Hoare
invariants.**

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # Annotation references i — tells the compiler s's type depends on i
    mut i: UpTo(arr.len) = 0     # At initialization i=0, verify: 0 == sum(arr[0..0]) → True
    while i < arr.len {
        s += arr[i]  # Compiler verifies: s_new == sum(arr[0..i+1])
        i += 1       # i changes → triggers s dependency re-verification: s satisfies SumUpTo(arr, i_new)
    }
    return s  # s: SumUpTo(arr, arr.len) = sum(arr[0..arr.len])
}
```

The compiler generates one verification condition for the loop body — induction hypothesis (type
annotation) → assignment operation → whether the new value satisfies the type annotation. Once the
proof pipeline verifies the inductive step, all iterations are automatically covered. No
`: decreases`, no `: Invariant`, no inductive proof needed — the compiler decomposes induction into
a local VC for each assignment.

### 2. Pre/Postconditions: Compile-Time Predicates on Parameter Types and Return Types

Abandon RFC-022's `//! requires`/`//! ensures`. Compile-time predicates are type annotations on
parameters or returns.

**The parameter side is a function call.** A compile-time predicate is a function returning `Type`;
its use at the parameter side is calling it — just like `factorial(5)`. The return side introduces a
new concept: return value parameter.

```yaoxiang
# Precondition: explicit call of compile-time predicate in parameter type
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b
#                       ^^^^^^^^^^  b is the current parameter name, passed to Positive as argument
#                       Compiler extracts argument value at the call site, substitutes b, verifies Positive(argument)
#                       Example: divide(10, 2) → verify Positive(2) = { 2 > 0 } → True
#                       Example: divide(10, 0) → verify Positive(0) = { 0 > 0 } → False → compile error

# Postcondition: return value parameter + compile-time predicate
IsMax: (T: Ord, arr: Array(T), result: T) -> Type = {
    forall j in 0..arr.len: result >= arr[j]
}

NonEmpty: (arr: Array(T)) -> Type = { arr.len > 0 }

max: (T: Ord) -> ((arr: NonEmpty(arr))) -> (result: IsMax(T, arr, result)) = {
#                                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
#                                            result is the return value parameter, value provided by return
#                                            Compiler substitutes the return value at the return point, verifies postcondition
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
- **Return side**: `-> (result: IsMax(T, arr, result))` — `result` is the return value parameter,
  value provided by the `return` statement. `result` exists only in the type signature, is only
  referenced by predicates, does not enter the function body scope, does not appear on the caller
  side.
- **Return value parameter is optional**: omit when there's no postcondition, the signature is
  identical to an ordinary function (`-> Int`).
- **Uniformity**: Parameter and return value parameters are the same concept —
  `param_name: predicate_call(argument)`, differing only in whether the value is provided by the
  caller or by `return`.

> **Implementation note (2026-10-02)**: Identification of the return position formal parameter is
> **strictly by the declared name** — `r` in `-> (r: P(...))` is the unique return value parameter.
> Free variables in the constraint that are **not in scope** are no longer substituted with the
> return value, but reported as undefined identifiers (actual test:
> `g: () -> (r: Eq2(m, r)) = { return 7 }` (where `m` is undefined) →
> `error [E1001] Unknown variable: 'm'`). The arguments of a predicate call are substituted into the
> constraint by the **actual arguments** truly: after substituting `P(r + 100)`, it becomes "return
> value + 100", not "return value".
>
> **Known gap (2026-10-02)**: **Multi-parameter predicates with symbolic arguments at the return
> position are not currently supported**. Trigger form:
> `f: (b: Int) -> (r: SumUpTo(b, r)) = { return b * 2 }` (where
> `SumUpTo: (n: Int, s: Int) -> Type = { s == n * 2 }`) →
> `error [E2031] … the refinement type constraint of 'r': SumUpTo(b, (b * 2)) cannot be statically proven`.
> Workaround: change the argument to a compile-time foldable literal (such as `SumUpTo(3, r)`), or
> provide a proof function returning `Type` for that predicate; resolution condition: the proof
> kernel supports argument substitution for "multi-parameter predicates with symbolic arguments".
> See [RFC-027a](../accepted/027a-termination-explicit-measure.md) §Implementation Landing Record
> (2026-10-02, this round of hardening) for details.

### 3. Path Condition Propagation: Compile-Time Verification of Runtime Values

When a compile-time predicate is used at a binding position, the arguments are explicitly passed by
the programmer. When runtime values enter refinement type parameters, the compiler completes
verification through path condition collection and SMT implication judgment — no need for the
programmer to explicitly pass proof.

#### 3.1 Explicit Function Calls

When a compile-time predicate is used at a binding position, the arguments are explicitly passed by
the programmer — it's just a function call, zero implicit.

`Positive: (x: Int) -> Type = { x > 0 }` is a compile-time predicate constructor. When it appears at
a binding position (parameter declaration, variable declaration, return type), the programmer
explicitly passes the bound variable name:

```yaoxiang
b: Positive(b)
// b is already declared as the current parameter, Positive(b) is a function call
// After normalization: b: { b > 0 }
```

No need for the compiler to implicitly fill in arguments — `b: Positive(b)` is just a function call
like `f(5)`. `b` is bound as a parameter name, and its type annotation `Positive(b)` references `b`
itself — this is the standard pattern of dependent types, not an implicit expansion rule.

**Unification with RFC-010's `self`**: RFC-010 establishes that `self` is not a keyword, just a
conventional parameter name ("writing `p`, `this`, `x` has exactly the same effect").
`b: Positive(b)` shares the same mechanism — the parameter name can be referenced in type
annotations. `self` appears in the position `self: Point`, `b` appears in the position
`b: Positive(b)`, both type annotations reference the parameter itself. The difference is only the
complexity of the type annotation, the mechanism is exactly the same — once the name is bound, the
type can depend on that name.

The same applies to return types:

```yaoxiang
Sorted: (arr: Array(T)) -> Type = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }

sort: (arr: Array(T)) -> (result: Sorted(result)) = { ... }
//                        ^^^^^^^^^^^^^^^^^^^^^^^
//                        result is the return value parameter, Sorted(result) is a function call
//                        Compiler substitutes the return value into result at the return point, verifies Sorted(return value)
```

The same applies to local variable declarations:

```yaoxiang
let x: Positive(x) = 5
// x is bound to 5, Positive(5) → { 5 > 0 } → True → passes

// let y: Positive(y) = 0
// y is bound to 0, Positive(0) → { 0 > 0 } → False → compile error
```

#### 3.2 Path Condition Collection

When a runtime value appears in a conditional branch, the compiler automatically collects path
conditions, forming the **assumption set** of the current scope. These assumptions participate in
verification as background knowledge for compile-time Bool evaluation.

```yaoxiang
if y > 0 {
    // The compiler automatically obtains the assumption in this branch: { y > 0 }
    let result = divide(x, y)
    // Verification condition: (y > 0) ⇒ (y > 0)
    // Proof pipeline judges implication holds → Proved
} else {
// This branch assumption: { !(y > 0) }
// If divide(x, y) is called, the verification condition is !(y > 0) ⇒ y > 0
    // Proof pipeline judges implication does not hold → Disproved
}
```

This is not the compiler hardcoding special patterns — this is the natural behavior of the
compile-time proof pipeline. Each type check call site sends to the pipeline:

```
{background assumptions} ⇒ {verification target}
```

The proof pipeline judges implication. `Proved` → passes, `Disproved` → compile error +
counterexample, `Unproven` → compile error + unsolved proposition. The background assumptions come
from the path conditions of the current program point.

#### 3.3 Assumption Stack

When analyzing control flow, the compiler maintains an assumption set for each basic block:

- **if-guard**: `if y > 0` → true branch pushes `y > 0`, false branch pushes `!(y > 0)` (if else is
  used)
- **match pattern**: `if let Some(v) = opt` → pushes `opt == Some(v)` in the branch
- **Logical conjunction**: `if x > 0 and y < 10` → pushes `x > 0` and `y < 10` in the branch
- **Function preconditions**: when calling `divide(a, b)`, the evidence that `b` satisfies
  `Positive` either comes from the current assumption, or from the refinement type annotation of the
  argument itself (if `b` is already annotated as `Positive`, its type carries `b > 0`)
- **Assignment**: when `let z = y`, the refinement conditions already on `y` propagate to `z`

All assumptions enter the compile-time proof pipeline. When entering the SMT acceleration path, they
are translated into SMT-LIB background assertions.

#### 3.4 No Static Evidence, Then Compile Error

If the programmer writes directly:

```yaoxiang
divide_user_input: (x: Int, y: Int) -> Int = divide(x, y)
```

The current program point has no `y > 0` assumption, and the argument `y` itself has no `Positive`
type annotation. The verification condition is:

```
{} ⇒ { y > 0 }
```

The pipeline returns `Disproved` (implication does not hold) → compile error:

> Cannot prove that parameter `b` satisfies `Positive` in the call to `divide`. `y` comes from
> function input, no proven bound. Consider guarding the call with an if branch:
> `if y > 0 { divide(x, y) }`.

YaoXiang does not accept runtime values directly entering refinement type parameters without
providing static evidence. This is not a limitation — this is the core of the hard-safety
philosophy. Any code the compiler cannot statically prove shall not pass compilation.

#### 3.5 Relationship with the Unified Pipeline

Path condition propagation is not an additional mechanism. It is a direct extension of the
compile-time proof pipeline on control flow analysis:

| Stage                             | Responsibility                                                                                                                                             |
| --------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Path condition collection         | Compiler's control flow analysis stage, annotates assumption set for each basic block                                                                      |
| Verification condition generation | When encountering type constraints to verify, merge path conditions + argument type information                                                            |
| Proof pipeline evaluation         | Compiler kernel → SMT acceleration → derive Proved / Disproved / Unproven                                                                                  |
| Result                            | `Proved` → passes; `Disproved` → compile error + counterexample; `Unproven` → compile error + unsolved proposition (programmer can provide proof function) |

No new components. No special rules. Path conditions are the background knowledge of the proof
pipeline — sharing the same pipeline and the same budget system as type equality and borrow
constraints.

### 4. Compile-Time Proof Pipeline

All compile-time checks share the same pipeline. The core operation of the pipeline is **type
checking** — checking whether a proof term's type equals the proposition to be proved. Everything is
type checking.

```
Compile-time encounters a Bool expression that needs evaluation (i.e., needs to construct a proof term)
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

#### 4.1 Proof Result: Three-Valued Algebra

Compile-time evaluation returns three results — this is the inevitable conclusion of the halting
problem, and also the natural partition of proof theory:

```
eval_compile_time : BoolExpr → Proved | Disproved(Model) | Unproven
```

- **Proved** → Halts, proof term has been constructed, type check passes. Compilation continues.
- **Disproved(M)** → Halts, counterexample M exists. Compile error + counterexample + source
  location.
- **Unproven** → Within given resource upper limit, proof has not been constructed. Compile error +
  unsolved proposition + budget consumption report.

**Unproven ≠ False.** The compiler saying "I cannot prove it" is not equivalent to the proposition
being false — it just exceeds the capability of the current automatic proof. This is honesty, not a
defect.

The hard budget limit is the engineering solution to the halting problem. No knob is provided —
providing one would be asking the user "do you think your program will halt", the user doesn't know,
the compiler doesn't know either.

#### 4.2 After Unproven: Programmer Writes Proof

When the compiler returns `Unproven`, the programmer can write a **proof function** — that is, a
YaoXiang function whose return type equals the proposition to be proved. The type checker verifies
this function — exactly the same mechanism as it verifies `add(a, b): Int`.

```
Proposition = Type
Proof       = Program (a value of that type)
Verification = Type checking (the only root of trust)
```

The SMT solver is not an independent trust boundary — it is an **acceleration module of the type
checker**. SMT helps find proofs, but it's always the type checker that verifies the proof. When SMT
returns `unsat`, the compiler reconstructs its result as a proof term verifiable by the type
checker. If reconstruction fails (SMT's reasoning steps exceed the compiler kernel's inference
rules), it falls back to `Unproven` — the programmer can manually write a proof function.

```yaoxiang
# Proposition: refinement attribute the compiler cannot automatically prove
FirstIsMin: (T: Ord, arr: Sorted(T)) -> Type = {
    forall i in 0..arr.len: arr[0] <= arr[i]
}

# Proof: programmer writes a function whose return type is the above proposition
# The type checker verifies this function — exactly the same as verifying add(a,b): Int
first_is_min: (T: Ord, arr: Sorted(T)) -> FirstIsMin(T, arr) = {
    # The compiler verifies here: the function body's type = FirstIsMin(T, arr)
    ...
}
```

No AI needed, no export to Coq, no new concepts. **Compile-time attributes that cannot be
automatically proven → programmer uses YaoXiang code to write proof → type checker verifies.** The
whole process is a smooth gradient — the compiler does simple proofs for you, leaving the brain for
difficult ones.

#### 4.3 Layered Dependencies within the Pipeline

The above evaluators share the same interface but have an evaluation order. Type equality is the
prerequisite for all subsequent analyses; ownership/token checking depends on type information;
refinement predicate verification depends on the results of the first two layers. The compiler
evaluates layer by layer; expressions that fail at lower layers don't enter upper layers — avoiding
wasting solving budget on type-erroneous programs.

```
Evaluation order (same pipeline, layered scheduling)
├── Layer 0: Type equality (T1 == T2)
│   └── Structural unification → failure means subsequent is meaningless, directly return Disproved
├── Layer 1: Ownership/token conflict
│   └── Flow-sensitive liveness analysis → failure means memory safety doesn't hold, directly return Disproved
└── Layer 2: Refinement predicate / Hoare implication
    └── Compiler itself → SMT acceleration → derive Proved / Disproved / Unproven
```

Each layer still returns `Proved/Disproved/Unproven`, sharing the same interface and the same budget
system.

### 5. Three-Layer Function Unification

| Layer                  | Timing       | Input      | Output | Example                                        |
| ---------------------- | ------------ | ---------- | ------ | ---------------------------------------------- |
| Value-level function   | Runtime      | Values     | Value  | `add: (a: Int, b: Int) -> Int = a + b`         |
| Type constructor       | Compile time | Type/Value | Type   | `List: (T: Type) -> Type = { data: Array(T) }` |
| Compile-time predicate | Compile time | Value      | Type   | `Positive: (x: Int) -> Type = { x > 0 }`       |

All use the same `name: type = value` syntax. Compile-time predicates and type constructors go
through the same compile-time proof pipeline — `{}` is the proof space.

### 6. Loops: Floyd-Hoare Verification Condition Generation

Loops don't need separate `: Invariant(...)` or `: decreases(...)` annotations. Compile-time
predicate type annotations on variables define Floyd-Hoare-style assertions — the compiler generates
verification conditions from the type annotations, and the proof pipeline checks whether each
assignment maintains the type.

Core mechanism: each assignment operation corresponds to a Hoare triple `{P} x := e {Q}`, the
verification condition is `P ⇒ Q[e/x]`. The compiler generates one verification condition for the
loop body — once the proof pipeline verifies the inductive step holds, all iterations are
automatically covered.

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # Annotation references i; at initialization i=0, verify: 0 == sum(arr[0..0]) → True
    mut i: UpTo(arr.len) = 0     # Verify: 0 <= 0 <= arr.len → True
    while i < arr.len {
        # The compiler generates one VC for the loop body. Premise: s satisfies SumUpTo(arr, i), i satisfies UpTo(arr.len).
        #
        # s += arr[i]:
        #   Verification obligation: s_new satisfies SumUpTo(arr, i) (current i unchanged)
        #   Substitute s_new = s_old + arr[i]:
        #     Need s_old + arr[i] == sum(arr[0..i+1])
        #     From inductive hypothesis s_old == sum(arr[0..i]), add arr[i] to both sides:
        #     sum(arr[0..i]) + arr[i] == sum(arr[0..i+1])
        #   Compiler + SMT: linear arithmetic, millisecond-level → Proved
        #
        # i += 1:
        #   i changes → s's type annotation in dependency graph references i → triggers re-verification
        #   New verification target: s satisfies SumUpTo(arr, i_new)
        #   i.e., s == sum(arr[0..i_new]), guaranteed by the previous step → Proved
        s += arr[i]
        i += 1
    }
    return s  # At this point s: SumUpTo(arr, arr.len), i.e., s == sum(arr[0..arr.len])
}
```

Loop invariants are the type annotations on variables — the programmer writes the type, the compiler
checks the inductive step. The compiler doesn't need to "discover" invariants, nor "automatically do
induction" — it decomposes the inductive proof into local verification conditions for each
assignment operation, handed to the proof pipeline to divide and conquer.

#### 6.1 Dependency Tracking: Dependent Types on Mutable Variables

The prerequisite of the above mechanism is: the compiler knows that `s`'s type annotation
`SumUpTo(arr, i)` references `i` — when `i` changes, `s`'s type constraint also changes. This
requires the compiler to maintain a **type dependency graph between variables**.

**Data structure**:

```
TypeDepGraph: Map<VarName, Set<VarName>>
# The key is the depended-on variable, the value is the set of variables whose type annotations reference that variable
# Example: { i: {s}, j: {s, t}, ... }
```

**Construction**: When the type checker processes `mut v: Pred(... x ...) = init`, it parses the
free variable references in the `Pred(...)` parameters. If a parameter references another mutable
variable `x` in the current scope, it records `x → v` in the dependency graph.

**Trigger**: When the depended-on variable `x` is assigned, the compiler:

1. Find all variables `{v₁, v₂, ...}` in the dependency graph that depend on `x`
2. For each `v`, generate a verification condition: whether `v`'s current value satisfies the
   updated type `Pred(... x_new ...)`
3. Send the VC into the proof pipeline

**Assignment order sensitive**: Dependency tracking naturally enforces the correct assignment order.
Taking `SumUpTo(arr, i)` as an example:

```yaoxiang
# Correct order
s += arr[i]   # s_new satisfies SumUpTo(arr, i+1)
i += 1        # i changes → re-verify s satisfies SumUpTo(arr, i_new) → True

# Wrong order — compiler rejects
i += 1        # i changes → re-verify s satisfies SumUpTo(arr, i_new)
              # s has not been updated yet, s_old == sum(arr[0..i_old]) ≠ sum(arr[0..i_new])
              # → Compile error: variable s does not satisfy type SumUpTo(arr, i_new)
s += arr[i]   # Unreachable
```

**Combined dependencies**: A variable can depend on multiple variables. The type annotation
`{ v: Int; v == x + y }` depends on both `x` and `y` — any change triggers re-verification.

**Relationship with the proof pipeline**: Dependency tracking is the trigger for VC generation, not
an independent verification mechanism. It answers "when do we need to generate a VC" — the proof
pipeline answers "whether the VC holds".

### 7. Termination Check

**Scope of application: Refinement types.** Termination is not an independent switch, but part of
the **verification mode**: once a type is refined (`Refined { base, constraint }`), the computation
annotated by it enters verification mode, which requires termination within the mode; ordinary
unrefined types do not enter verification mode and generate no termination obligations.

This gives two corollaries:

- **Loops**: Bare `while` does not enter verification mode; when a measure variable carries a
  refinement annotation (e.g., `i: UpTo(n)`), it enters verification mode, requiring proof of
  termination.
- **Recursion**: When a function signature carries refinement (parameter refinement or return type
  containing refinement), it enters verification mode, requiring proof that the measure strictly
  decreases at each recursive call site.

Full automation is preferred within the mode: the compiler first automatically explores the measure;
what can be proven passes; when exploration fails to find one and no explicit measure is given, it's
a compile error. There is no **annotation syntax** loophole — the measure and termination
proposition are both written in the type position, no new `decreases` syntax is introduced. The form
of measures and explicit fallbacks is in §6.9.

#### 6.1 Design Principles

The compiler automatically extracts the information needed for the termination proof from two
places:

1. **Variable type annotations**: Boundary constraints in refinement types (e.g., `UpTo(n)` gives
   upper bound `n` and lower bound `0`)
2. **Loop body operations**: Operations applied to variables at each iteration

The compiler tries four measure synthesis strategies in priority order, stopping once one is found.
The four strategies are a **restricted template sequence for measure exploration**, taking
refinement constraints as input (strategies 1-4 all start with "variables of bounded type"), not
"code evaluated at compile time"; they are the automatic and manual sides of the same thing as the
explicit measures in §6.9.

> **Measure exploration is exploration, not inference.** Exploration only enumerates templates
> (linear rank, violation count, bounded pattern, multiplicative scaling), without guaranteeing a
> solution exists — general measure inference is generally undecidable (reduces to the halting
> problem). Therefore, situations outside the templates must be allowed to be explicitly given by
> the programmer (§6.9), otherwise compile error.

#### 6.2 Strategy 1: Automatic Linear Rank Function Synthesis

When a variable has a linear bound annotation, the compiler enumerates candidate linear measures and
verifies via SMT.

```
Input:
  Variables v₁: UpTo(u₁), v₂: UpTo(u₂), ... (variables with upper and lower bounds)
  Loop condition cond
  Assignment set in loop body

Algorithm:
  1. Extract each variable's bound from type annotations: [low_i, high_i]
  2. Enumerate candidate measures: v_i, u_i - v_i, v_i - v_j, etc., linear combinations
  3. For each candidate measure m:
     - SMT verifies m ≥ 0 (derived from type bounds)
     - For each execution path of the loop body, SMT verifies m' < m (strictly decreasing)
  4. Find a linear combination that satisfies the conditions → termination proven
```

Coverage: loops where any variable is assigned a linear expression (`v = a·v + b`) and has a bounded
type annotation. Includes `i += const`, `i -= const`, and binary-search-style interval contraction:

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

#### 6.3 Strategy 2: Predicate Violation Count — Automatically Extracting Measure from Target Type <span style="color:orange">[Experimental Strategy]</span>

> ⚠️ **Current status: Experimental strategy, the decision on whether to include it will be made in
> Phase 3 based on actual feasibility.** This strategy is effective for adjacent swap operations
> (bubble sort, insertion sort), but cannot automatically prove for non-adjacent operations
> (quicksort partition, heapsort sift-down). The coverage boundary is shown in the table below. If
> Phase 3 verification is infeasible, this strategy will be removed or downgraded to future work.

Core insight: **The specification the user writes is the material for compiler reasoning.** The
compiler doesn't need to have "what is sorting" built-in — it reads the definition of `Sorted` and
automatically extracts the measure from the definition.

```
Input:
  Target type: Sorted(arr) = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }
  Loop body operation: adjacent element swap

Algorithm:
  1. Parse the predicate definition: forall i in range: cond(i, arr)
  2. Automatically generate measure: violation_count = |{ i | ¬cond(i, arr) }|
  3. Analyze the operation's effect on the measure:
     - Adjacent swap arr[j], arr[j+1] = arr[j+1], arr[j]
     - Only affects index pairs j-1, j, j+1
     - If arr[j] > arr[j+1] (violates the predicate), the pair satisfies the predicate after swap
     - violation_count decreases by at least 1
  4. Upper bound: n·(n-1)/2 (maximum number of adjacent inversions), lower bound: 0
  → Termination proven
```

**Current coverage**:

| Algorithm      | Operation Mode    | Strategy 2 Provable? | Reason                                             |
| -------------- | ----------------- | :------------------: | -------------------------------------------------- |
| Bubble sort    | Adjacent swap     |          ✅          | violation_count strictly decreases per swap        |
| Insertion sort | Adjacent move     |          ✅          | Each shift eliminates one violating pair           |
| Selection sort | Non-adjacent swap |          ❌          | A single swap may increase violation_count         |
| Quicksort      | partition         |          ❌          | Non-adjacent swap, no monotonic decrease guarantee |
| Heapsort       | sift-down         |          ❌          | Tree operation, violation_count is not monotonic   |

**Complementary strategies**: For quicksort, the `low < high` interval contraction can be covered by
Strategy 1 (linear rank function) — the outer partition recursion halves the interval each time.
Strategies 1 and 2 complementarily cover each other; termination of most practical algorithms can be
proven by one of them. However, the generalization of Strategy 2 (non-adjacent operations, tree
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

#### 6.4 Strategy 3: Bounded Increase/Decrease Pattern

`v += const` (positive constant), variable has upper bound type annotation → measure
`upper_bound - v` decreases by `const` each time, lower bound 0. This is a degenerate case of
Strategy 1, the compiler handles it quickly at the very front.

#### 6.5 Strategy 4: Multiplicative Scaling Measure Template

`v *= const` (const > 1), variable has upper and lower bound type annotations. The compiler has a
built-in logarithmic measure template `ceil(log_const(upper/v))`, measure decreases by 1 each
multiplication by const.

```yaoxiang
mut i: Positive(i) = 1
while i < n {
    # Compiler automatically derives: measure ceil(log₂(n/i)), measure decreases by 1 each multiplication by 2
    i *= 2
}
```

#### 6.6 Termination and Correctness Separation

Termination proof and correctness proof are independent:

- **Termination**: The above four strategies automatically prove that the loop exits in finite
  steps; when exploration fails, the programmer gives the measure at the type position (§6.9)
- **Correctness**: Whether the loop body progresses toward the target type is checked by the
  compile-time proof pipeline via verification conditions

Both pass → compilation passes. Termination proven but correctness fails → compile error +
counterexample. Correctness proven but termination cannot be proven → compile error and indicate the
unanalyzable variable or operation. Both fail → compile error reports the two failure reasons
separately.

#### 6.7 Termination Check for Recursive Functions

For recursive functions with refinement signatures, the compiler checks that the formal parameters
decrease at each recursive call site:

```yaoxiang
gcd: (a: Int, b: NonNegative(b)) -> Int = {
    if b == 0 { return a }
    return gcd(b, a % b)  // Compiler explores: well-founded order measure of (b, a % b) decreases → terminates
}
```

Formal parameter decrease is the strongest automatic path (structural recursion). When exploration
fails, the programmer explicitly gives the measure at the type position (§6.9).

#### 6.8 Hard Boundary

`i = f(i)` and `f` is irreversible, not closed, does not preserve any monotonicity — mathematically
impossible to automatically prove termination. Compile error:

> This loop cannot automatically prove termination. The loop variable depends on the unanalyzable
> function `f`. Please use an iteration pattern that can be analyzed by the compiler, or bind a name
> to this loop and provide a measure at the type position (§6.9).

This is not a compiler failure. Any code that cannot be statically proven safe shall not pass
compilation. Even if an explicit measure is given, it must be judged as true by SMT to pass — an
incorrect measure will be hit back by a counterexample; humans can only fail to prove, not prove
incorrectly.

#### 6.9 Explicit Measure: `Terminates`

When automatic exploration fails to find a measure, the programmer writes the measure in the **type
position** — the same mechanism as `Positive(b)`, `IsMax(T, arr, result)` (predicate application),
zero new syntax:

```yaoxiang
// Measure: ordinary function, unit-testable, reusable, not involved at runtime
gcd_measure: (a: Int, b: Int) -> Int = { b }

// Termination component lives on the function's own type position
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// Loop: binding name is the anchor, measure acquires the in-scope quantity by name
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n {
        i = i + 1
        i
    }
    return acc
}
```

The reason for the loop here: `Terminates(m)` refines the value type of the loop body's tail
expression, the anchor is provided by the binding name `acc` — the loop can thus be referentially
named, no longer having the dead angle of "anonymous constructs cannot be referenced".

> **Body tail must give a value**: The value of a loop = the value of the loop body block (§2.9),
> and the value of a block = the tail expression. If the body tail is an assignment statement (e.g.,
> `{ i = i + 1 }`), the block value is `Void`, and `return acc` would be returning `Void` from
> `-> Int`, the type doesn't hold. Therefore the body tail of this example explicitly writes `i`.
> This does not change the semantics of the measure — the measure `n - i` is a **state expression**,
> unrelated to the loop's value type.

> **Implementation note (2026-10-02 registered; 2026-10-04 updated)**: The **type side** of the loop
> example in this section holds (`check` 0 error, back-edge obligation Proved). **The runtime
> value-taking defect (#409) has been fixed**: `while` as a value now takes the value of the body
> block's tail expression in the last iteration, on the same caliber as the type side's
> `block_value_ty` — `loop(4) == 4`, `countdown(3) == 30` actually run (`for` has the same fix).
>
> **Known residue (zero iterations)**: When the loop body has never been evaluated, there is no
> "value of the last iteration's body", the loop value still takes `Void` — binding to a non-`Void`
> type position and using it at runtime reports
> `E6007 Runtime error: type mismatch in comparison Eq: Void vs Int(0)` (a loud type mismatch, not a
> silent wrong value). This is a **static approximation boundary**: the `block_value_ty` criterion
> gives the type of the body tail expression, and has no judgment capability on "whether the loop
> executes at least once" (requires data flow/provability analysis), so the type side does not
> downgrade on its own (downgrading would break the value type that this section's `Terminates` loop
> binding depends on). The contract of this residue is nailed down by the corpus
> `tests/yaoxiang/02-type-system/while_zero_iteration_void_err.yx`
> (`// expect: runtime-error E6007`). **This residue has been confirmed as a deliberately retained
> design boundary** (2026-10-04 ruling, no fix for now), so issue
> [#409](https://github.com/ChenXu233/YaoXiang/issues/409) has been closed along with the main form
> fix, and no separate tracking item is opened. The minimal reproduction and impact history are in
> [RFC-027a](../accepted/027a-termination-explicit-measure.md) §Example Loop Section "Known Defect
> D6". Also: Termination Strategy 1 (linear rank function) has resumed effect in this round, so the
> order of "write explicit measure only after automatic exploration fails" remains unchanged —
> explicit measure is still the fallback after exploration fails.

**`Terminates` is a built-in predicate**, on the same level as `Int`, `Never` and other core
primitives. It is the only predicate **whose function body is written by the compiler on behalf of
the user** — its assertion ("the measure at each recursive call site/loop back edge strictly
decreases") lives in the computational structure, and the user-written predicate cannot reference
the function body or loop body, so it cannot be expressed with the predicate definition syntax in
the "Syntax" section. The built-in face converges to this one name.

**Arity**. `Terminates(FnType, m)` and `Terminates(m)` are two arities of the same predicate, not
two kinds of constructions:

| Form                    | Anchor                         | Use Case                                                       |
| ----------------------- | ------------------------------ | -------------------------------------------------------------- |
| `Terminates(m)`         | Name of the binding it sits on | Self-recursive functions, loops — default form                 |
| `Terminates(FnType, m)` | Explicit function type         | Scenarios like mutual recursion where the anchor is not unique |

Both are essentially the same: the termination obligation always falls on "the computation annotated
by the type position where the refinement sits".

**The measure does not restrict the return type.** The measure can be an expression of any type (not
forced to be a natural number); the "strictly decreasing" on it is given by the available
well-founded order on that type. Whether the measure is well-founded (e.g., when returning `Int`,
whether `>= 0` holds) is an **independent obligation**, also handed to refinement inference or SMT;
when both cannot be inferred, the diagnosis **does not directly reject**, but suggests directions to
check (whether the measure's lower bound holds, whether the recursive parameter really moves in that
direction).

**Relationship with automatic exploration**: Explicit measure is not another pipeline, but input
after exploration fails. After the measure is given, the same SMT still verifies the decrease and
well-foundedness; if it doesn't hold, an error is reported with a counterexample.

The obligation generation, decision pipeline, diagnosis direction, measure sharing for mutual
recursion (SCC) and other implementation mechanisms are in
[RFC-027a: Explicit Measure for Termination Check](../accepted/027a-termination-explicit-measure.md).

### 8. SMT Solver: Acceleration Module of the Type Checker

In traditional languages, the SMT solver is an external tool (e.g., F\* calls Z3, Dafny calls Z3).
In YaoXiang, it is an **acceleration module of the type checker** — only called when the compiler
kernel itself cannot directly determine. SMT helps find proofs, but the type checker verifies the
proof.

**Trust model**: The type checker is the only root of trust. The SMT solver is an acceleration
module — it helps find proofs, but SMT is not an independent trust boundary. The compiler trusts
Z3's `unsat` result (consistent with the F\*/Dafny path — the probability of Z3 error is lower than
the compiler's own bug rate, this is a pragmatic engineering choice). The real unreliability control
is at the SMT translation layer — if the translation has bugs, the compiler will expose them in
other tests.

**Interface**: The compiler internally translates to the SMT-LIB 2.6 standard format, rather than
binding to a specific solver API. SMT-LIB is an ISO standard, natively supported by Z3, CVC5,
MathSAT, Yices.

**Default backend**: Z3 (MIT license, the most extensive documentation and community validation).
CVC5 as an SMT-LIB compatible alternative — the user can switch via compiler flags at compile time.

No "general solver abstraction layer" — SMT-LIB is the abstraction layer. If CVC5 makes a
breakthrough in a specific theory in the future, switching only requires changing the binary, no
need to modify the compiler code.

```
Compile-time Bool expression
        │
        ├── Compiler kernel can directly determine (structural equivalence, simple arithmetic,
        │   trivial formulas after constant folding)
        │   → Directly return Proved / Disproved
        │
        └── Compiler kernel cannot directly determine (quantifiers, symbolic variables)
            → Dependent type pre-reduction (factorial(5) → 120)
            → Translate to SMT-LIB format
            → Send to Z3/CVC5 (with budget limit)
            → Return: unsat → Proved  │  sat + model → Disproved  │  unknown → Unproven
```

**Solving budget — hard limit, like stack depth**:

| Budget Dimension               | Default | Description                                                                                                                                                                    |
| ------------------------------ | ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Solving steps                  | 10,000  | Z3 is usually within a hundred steps for linear arithmetic. 10,000 steps cover 99% of actual predicates.                                                                       |
| Time                           | 100ms   | A single predicate exceeding 100ms = the user is writing a compile-time program rather than a type annotation. 100ms × 50 predicates = 5 seconds compilation time upper limit. |
| Quantifier instantiation depth | 3       | Three layers of nested quantifiers cover actual patterns. More than three layers is likely writing logic puzzles.                                                              |

Exceeding the budget returns `Unproven`, compile error + predicate location + consumption amount. No
degradation, no runtime check, no silent pass.

**Why this is practically feasible**: In engineering, 95% of actual predicates are linear arithmetic
— `x > 0`, `arr.len > 0`, `0 <= idx < arr.len` — all within decidable fragments, SMT solvers return
millisecond-level for these problems. Encountering the rare complex predicate that exceeds the
budget, the programmer writes a proof function.

Dependent types do a layer of pre-reduction before the SMT call: `factorial(5)` directly
compile-time evaluates to `120`, `append([1,2], [3])` directly evaluates to `[1,2,3]`. These
deterministic value computations do not consume SMT budget.

The programmer doesn't need to know SMT exists. The mental model is: **the compiler can prove and
passes, can't and errors — if the compiler doesn't know, you can write a function to prove it to
it**.

### 9. Compile-Time Predicate Composition

Compile-time predicates are functions returning `Type`, composition is naturally achieved through
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

### 11. dispatch Dispatch Pipeline: Unified Dispatch of Compile-Time and Runtime

> **Implementation note (2026-10-03, #377-2)**: The **semantics** of this section hold and are
> connected to production, but the **structure** is not a standalone `layers/dispatch.rs` module.
> The production dispatch is directly implemented by three **call-site obligations** in `checker.rs`
> (directly calling `layers::predicate::check_predicate`): binding site re-verification
> (`revalidate_refined`), call-site argument (`check_call_arg_refinements`), return position
> postcondition (`check_return_refinement`). The original `layers/dispatch.rs` had zero production
> call sites, and its handling of `Unproven { ProofFunctionRequired }` was "downgrade to W1080
> warning and inject Γ", conflicting with the production implementation (`Unproven` always errors,
> see `refined_unproven`), so it has been deleted entirely. Also note: the "insert runtime check" in
> the Runtime row of the table in this section has **no** emission point in production
> (`RuntimeOutcome::InsertCheck` has no consumer), and Γ injection is independently done by the
> branch guard in `ownership.rs` and `inference/expressions.rs`.

`assert` and `Assert` are two sides of the same refinement type primitive. The dispatch pipeline
`dispatch` automatically decides to take compile-time proof or runtime check based on **whether the
predicate's free variables are accessible at compile time**:

| Criterion                                                                                    | Mode            | Behavior                                                                                   |
| -------------------------------------------------------------------------------------------- | --------------- | ------------------------------------------------------------------------------------------ |
| All free variables known at compile time (generic parameters, compile-time constants)        | **CompileTime** | Enter proof pipeline: Proved → erase, Disproved → compile error, Unknown → require proof   |
| There exist free variables from runtime (function parameters, external input, mut variables) | **Runtime**     | Insert runtime check, and inject refinement facts into the flow-sensitive assumption set Γ |

**Key**: "Cannot decide" ≠ "Disproved". In CompileTime mode, Unknown requires proof (no silent
downgrade); in Runtime mode, the proposition has no truth value at compile time at all — no matter
how strong the prover is, it cannot write a tautological proof for "the user might have input a
negative number", runtime check is the only sound choice. This is not because the prover is not
strong enough, it is a theoretical necessity.

### 12. Flow-Sensitive Assumption Set Γ: Strongest Postcondition Propagation

The compiler maintains a flow-sensitive assumption set Γ, tracking the propositions known to hold at
each control flow point.

**SP (Strongest Postcondition) propagation**:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
```

**Kill set for mut variables**: After a `mut` variable is reassigned, all assumptions involving that
variable are removed from Γ:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
mut x = x - 5       // Γ = {}  ← x > 0 is killed
```

This is a hard requirement of soundness — the variable value has changed, old assumptions are
invalid.

**Branch confluence**: When IF/ELSE or match branches merge, Γ takes the intersection of each
branch's assumptions. Only propositions that hold on all paths are brought out of the branch.

### 13. Erasure Model Clarification: Witness Erasure ≠ Check Erasure

RFC-027's claim that "refinement types are **completely erased** at runtime" refers to the **proof
witness** — proof terms verified at compile time do not generate runtime code. But the **runtime
checks** inserted by dispatch in Runtime mode are retained — they are Bool checks executed at the
value level, not type-level witnesses.

Summary: witness is erased, check is retained. The two things don't conflict, RFC-027's original
claim remains unchanged.

## Detailed Design

### Syntax Changes

| Before (RFC-022)                      | After (This RFC)                                                                                                                        |
| ------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| `//! requires: NonEmpty(n) = n > 0`   | Compile-time predicate as parameter type `(b: Positive(b))`                                                                             |
| `//! ensures: ExistsMax(result, arr)` | Return type uses return value parameter `-> (result: IsMax(T, arr, result))`                                                            |
| `/*! invariant: ... !*/`              | Compile-time predicate type annotation on variables — Floyd-Hoare invariants                                                            |
| `//! decreases: n`                    | Measure written in refinement type position (`Terminates`); compiler first auto-explores, only requires explicit when exploration fails |
| Specification as comment              | Specification is the type system                                                                                                        |

### Syntax

**No new keywords for compile-time predicates.** `{}` is the proof space, exactly the same as the
existing type definition syntax. Compile-time predicates are functions returning `Type` —
`name: (params) -> Type = { assertions }`. Used as function calls — `Positive(b)`,
`IsMax(T, arr, result)`.

```bnf
# Compile-time predicate = function returning Type, assertions inside {} are verified by compiler
# Uses existing function/type syntax, no need to add new BNF rules
predicate ::= identifier ':' params '->' 'Type' '=' '{' assertions '}'
```

**The arguments of a predicate application must be in compile-time constant form** — literals,
variables (bound by name), type applications (recursively extracted), or **compile-time referable
function references** (function names). Arguments that cannot be converted to constant expressions
report **E1092**, mismatch between the number of arguments and the number of formal parameters
declared by the predicate reports **E1093**. Arguments are bound to the formal parameter list by
position, predicate arity is determined by declaration — `Positive(x)` unary,
`IsMax(T, arr, result)` ternary, `Terminates(m)` and `Terminates(FnType, m)` unary and binary —
refinement constraints **never silently discarded** (previously, inconvertible arguments would cause
constraints to silently disappear, and bindings violating the constraint would silently pass).

**New syntactic concept: Return value parameter** — in `-> (name: Type)`, `name` is the return value
parameter.

The return value parameter is the **only syntactic concept** introduced by YaoXiang on top of the
existing function syntax. Its semantics:

- The value of `name` is provided by the `return` statement
- `name` exists only in the type signature, referenced by postcondition predicates (e.g.,
  `-> (result: IsMax(T, arr, result))`)
- `name` does not enter the function body scope, does not appear on the caller side
- The return value parameter is **optional** — when there is no postcondition, the signature is
  exactly the same as an ordinary function (`-> Int`), with no additional burden

The reason for introducing it: postconditions need to reference "the value the function is about to
return". Without a return value parameter, the compiler can only let the predicate reference the
return value through special rules (such as implicit variable `$result` or `__retval__`). The return
value parameter makes this reference explicit — it is just a formal parameter, except that the value
is provided by `return` rather than the caller.

**Proof function** is not a new concept — it is a YaoXiang function whose return type is the
proposition being asserted. When the compiler returns `Unproven`, the programmer provides a proof
function, and the type checker verifies it exactly the same way it verifies any function's return
type. No new syntax, no new keywords, no new rules.

> **The boundary between the two.** The correctness domain (predicate Unproven) supplements the
> proof in the **body**: write a function whose return type is the proposition to be proved. The
> termination domain (§6.9) supplements the measure in the **type position**: write
> `Terminates(measure)` as the type of a binding or function signature. The former is "write a proof
> for an unprovable proposition", the latter is "explicitly declare a measure that cannot be
> explored" — the mechanism is the same (both are refinement type applications), the landing points
> differ. The termination domain **does not need** to write a `_proof` function.

### Type System Impact

- **Type universe**: Compile-time predicates are in the Type₂ layer — functions accepting values and
  returning `Type`, on the same level as type constructors
- **Generic interaction**: Compile-time predicates can take generic parameters, e.g.,
  `NonEmpty: (T: Type) -> (arr: Array(T)) -> Type`
- **Ownership interaction**: Expressions in compile-time predicates obey ownership rules, can only
  read, not write
- **Type inference**: The arguments of compile-time predicates participate in HM type inference

### Runtime Representation

Compile-time predicates are **handled according to the dispatch result at runtime**:

- **CompileTime mode** (all free variables known at compile time): The witness is completely erased
  after the proof passes. `Positive: (x: Int) -> Type = { x > 0 }` — the parameter `b: Positive(5)`
  is represented as `Int` at runtime. The refinement condition `{ 5 > 0 }` has passed, erased.
- **Runtime mode** (there exist runtime free variables): Retain the runtime check — execute a Bool
  check at the value level, inject into the flow-sensitive assumption set Γ. See §11 dispatch
  pipeline and §13 erasure model clarification for details.

Placing a compile-time predicate in a type position (e.g., `f(x: Positive(x))`) does not produce a
wrapper type, does not allocate extra memory. But when `x` comes from runtime input, **it will**
insert a runtime Bool check.

**Interaction constraints with `ref`**: Compile-time predicates can only reference immutably
borrowed values or values whose ownership has been transferred. A compile-time predicate that
references a mutably borrowed value cannot guarantee at compile time that the verification result
still holds at runtime — such usage directly reports a compile error.

### Compiler Changes

1. **Parser**: Compile-time predicates use standard function syntax, no additional parsing rules
2. **Compile-time proof pipeline**: Unified Proved/Disproved/Unproven return interface, automatic
   strategy selection
3. **SMT acceleration module**: SMT-LIB 2.6 translation layer, default backend Z3, CVC5 alternative
4. **Type checker kernel**: Inference rule implementation — structural equivalence, βδι-reduction,
   universal quantifier introduction/elimination. This is the only root of trust, SMT and programmer
   proof are both verified through this
5. **Verification condition generation**: WP/SP calculus + loop invariant proof obligations
6. **Error reporting**: Counterexample formatting + unsolved proposition report + source location
   correlation

### Backward Compatibility

- ✅ Code not using compile-time predicates is completely unchanged
- ✅ Compile-time predicates have zero runtime overhead in CompileTime mode, only retain necessary
  Bool checks in Runtime mode
- ⚠️ RFC-022's `//!` syntax is no longer supported — but 022 was never implemented, no migration
  burden

## Trade-offs

### Advantages

- **Curry-Howard correspondence fully delivered**: Types are propositions, programs are proofs,
  `name: Proposition = Proof`
- **Unity**: Compile-time predicates and ordinary functions use exactly the same syntax, no
  conceptual split
- **SMT transparency**: The programmer doesn't need to know SMT exists, the mental model is
  consistent with type checking
- **Progressive adoption**: Start with one compile-time predicate, gradually increase coverage
- **Minimal runtime overhead**: Zero overhead in CompileTime mode, only necessary Bool checks in
  Runtime mode

### Disadvantages

- **Compile time**: SMT solving increases compile time, but hard budget limit guarantees an upper
  limit
- **Automatic proof boundary**: Complex predicates beyond first-order linear arithmetic may require
  the programmer to write a proof function. This is not a language defect — this is the inevitable
  conclusion of the halting problem. The compiler honestly reports `Unproven` rather than falsely
  reporting True/False
- **Learning curve**: Writing effective compile-time predicates and proof functions requires
  understanding the basic intuition of the Curry-Howard correspondence
- **Implementation complexity**: Unifying the compile-time proof pipeline requires careful design

### Risk Mitigation

- Hard SMT solving budget limit (steps 10,000 / time 100ms / instantiation depth 3), exceeding the
  budget returns `Unproven`
- Dependent type pre-reduction: deterministic value computations are consumed first, SMT only
  tackles the non-deterministic part
- `Unproven` is not a dead end: write a proof function in the correctness domain (return type is the
  proposition), give a measure in the type position in the termination domain (§6.9) — both are
  verified by the type checker
- Incremental verification: only verify changed modules
- Clear error messages + counterexample display + budget consumption report + unsolved proposition +
  suggestion (if the compiler can give one)

## Alternatives

| Option                                                           | Why Not Chosen                                                                                                                                                         |
| ---------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| RFC-022: `//!` comment-style specification                       | Specification and type split, violates Curry-Howard correspondence                                                                                                     |
| Independent specification files (e.g., CVL)                      | Specification and code separation, increases maintenance cost                                                                                                          |
| Runtime-only assertions                                          | Cannot statically guarantee correctness                                                                                                                                |
| External proof assistant (e.g., Coq)                             | Disconnected from the compiler, needs independent proof language and trust boundary. YaoXiang's choice: proof is YaoXiang code, type checker is the only root of trust |
| **This Option: Compile-time predicates as first-class citizens** | ✅                                                                                                                                                                     |

## Implementation Strategy

### Phases

| Phase       | Content                                                                                                                                                                 |
| ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Phase 1** | Compiler kernel: structural equivalence + βδι-reduction + universal quantifier introduction/elimination. Supports simple arithmetic predicates (`x > 0`, `arr.len > 0`) |
| **Phase 2** | SMT-LIB translation layer + Z3/CVC5 integration. Pipeline returns Proved/Disproved/Unproven. Supports programmer writing proof functions when Unproven                  |
| **Phase 3** | Loop invariant VC generation + termination check (measure exploration four strategies + `Terminates` explicit measure, §6, §6.9)                                        |
| **Phase 4** | Incremental verification + cache + IDE support                                                                                                                          |

### Dependencies

- RFC-010: Unified Type Syntax — compile-time predicates are based on `name: type = value`
- RFC-011: Generic Type System — compile-time predicates can take generic parameters
- RFC-009: Ownership Model — compile-time predicate expressions obey ownership rules

## Open Questions

- [x] **Z3 availability for wasm32 target**: No Z3 under wasm (all SMT code is excluded via
      `cfg(not(target_arch = "wasm32"))`). Impact scope and degradation direction: - `ownership.rs`
      `smt_cut` always returns `false` → back-edge traversal → **conservative rejection** (sound
      direction, just less narrowing) - Termination check strategy 1, predicate layer 2b/3 entirely
      skipped

  **Correction (2026-10-01)**: The original text claims these paths are "currently **unreachable**"
  (reason: `predicate_defs` production has never been populated, `parser` does not produce
  `MonoType::Refined`, `with_solver` production is never called), which no longer holds — all three
  have been landed (predicate definition registration #377-3; refinement type normalization;
  production pipeline injects solver). Therefore, the difference between wasm and native is no
  longer limited to the precision of `smt_cut`:

  | Path                                        | native  | wasm       | Degradation Direction             |
  | ------------------------------------------- | ------- | ---------- | --------------------------------- |
  | Termination check explicit measure decision | Decide  | `Unjudged` | Do not report E4022 (less report) |
  | Well-foundedness decision                   | Decide  | `Unjudged` | Do not report E4022 (less report) |
  | Refinement predicate SMT implication level  | Execute | Skip       | Fall to `Unproven`                |

  All are **conservative** directions (less narrowing, less reporting), not affecting soundness. For
  accuracy consistency, the plan and cost are in issue #376 (conclusion: prioritize the JS-side Z3
  instance over chaining Z3 into the main wasm — the latter requires changing the emcc build system
  and the product grows from 3 MB to ~20 MB).

- [x] **SMT solver choice**: Default Z3 (MIT license, most widely verified). CVC5 as SMT-LIB
      compatible alternative, switched by compiler flag. The compiler's internal translation target
      is the SMT-LIB 2.6 standard format — SMT-LIB is the abstraction layer, no custom general
      solver interface.
- [x] **Specific values for solving budget**: Steps 10,000 / time 100ms / quantifier instantiation
      depth 3. Fixed inside the compiler, no knob. If real use cases prove it not enough (not "the
      user wrote it wrong") in actual use, adjust.
- [x] **Quantifier support scope**: The language level does not limit the order of quantifiers.
      Compile-time predicates accept `Type` parameters — `Type` includes function types — so
      higher-order quantifiers are a natural inference of the type system, no special syntax needed.
      SMT solvers can automatically decide first-order quantifiers (forall/exists, supports
      interleaved nesting, limited by budget depth 3). Higher-order quantifiers: SMT returns
      `Unproven`, compiler prompts "this predicate exceeds the automatic proof range, please provide
      a proof function". The programmer writes a YaoXiang function whose return type equals the
      proposition — the type checker verifies the function. No external export needed, no AI needed,
      no interactive proof mode needed. Everything is YaoXiang code, everything is verified by the
      type checker.
- [x] **Counterexample formatting**: Source variable names are directly used as SMT variable names
      (with module prefix to avoid conflicts). When the Z3 model returns, look up by variable name.
      Output format: variable name = specific value + source location + predicate definition
      location. No complex mapping layer.
- [x] ~~**Interaction of compile-time predicates with `ref` smart pointers?**~~ → Decided:
      Compile-time predicates only allow immutably borrowed values or values whose ownership has
      been transferred. Values that are mutably borrowed cannot appear in compile-time predicates.
- [x] **Extension of `forall` predicate violation count measure to non-adjacent operations?** → No
      extension. The current coverage (adjacent swap, adjacent move) is complemented by Strategy 1
      (linear rank function) — the outer interval contraction of quicksort is covered by Strategy 1,
      heapsort is covered by Strategy 1 (array index pattern). Loops whose termination cannot be
      proven by any strategy, the compiler directly errors — this is hard-safety philosophy, not a
      defect. If there are real-world scenarios in the future (not academic constructions) where
      algorithms cannot be covered by any of the four strategies, then discuss again. →
      **Re-discussion triggered (2026-09-14, #318)**: Non-structural recursion (gcd-like
      non-directly-decreasing, mutual recursion, merge partition) is the real scenario expected by
      this clause — the template sequence of measure exploration cannot automatically capture them,
      and they cannot be rewritten into analyzable iteration patterns without destroying
      readability. Conclusion: hard-safety philosophy unchanged (an incorrect measure is still hit
      back by SMT counterexamples, humans can only fail to prove, not prove incorrectly), but
      "rejection when exploration fails" is relaxed to "when exploration fails, the programmer can
      explicitly give the measure at the type position" — see §6.9.
- [x] **Linear rank function enumeration combination explosion**: The upper limit of candidate
      enumeration is 3 bounded variables. When ≤3, enumerate all linear combinations and SMT
      verifies one by one. When >3, only try single-variable measures (`v_i`, `u_i - v_i`), failure
      directly reports a compile error — prompting the programmer "the loop has >3 bounded
      variables, the compiler cannot automatically synthesize multi-variable measures". This is not
      an engineering compromise — it forces the programmer to write simpler loops.

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
│   Draft     │  ← Created by author
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
│ (Formal     │    │ (Retained   │
│  design)    │    │  in place)  │
└─────────────┘    └─────────────┘
```
