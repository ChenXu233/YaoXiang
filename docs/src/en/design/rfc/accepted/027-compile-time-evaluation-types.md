---
title: 'RFC-027: Compile-time Predicates and Unified Static Verification'
status: 'Accepted'
author: 'Chen Xu'
created: '2026-06-07'
updated: '2026-10-03'
impl_status: 'in-progress'
impl_detail:
  'Phase 1-2 complete, Phase 3 partial, Phase 4 partial. All 6 phases of the assert/Assert unified
  plan implemented (#157-#162 closed): Never type, IsTrue bridging, flow-sensitive Γ + kill set,
  type-level recursion, universe stratification weak check. The **semantics** of dispatch in §11 has
  been implemented via call-site obligations (`checker.rs` calls `check_predicate` directly in three
  places: binding site / call-site argument / return position), but the **standalone
  `layers/dispatch.rs` module has been deleted**—it has zero production call sites, and its handling
  of `Unproven` (downgrade to W1080 warning) conflicts with the production implementation (always
  report error) (#377-2).'
impl_percent: 82
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
> [RFC-022: Hoare Logic Static Verification Support (Specification Comments and Specification Types)](../deprecated/022-hoare-logic-static-verification.md)
> — deprecated

## Summary

This RFC proposes introducing **compile-time predicates** as first-class citizens into YaoXiang,
unifying all compile-time static verification into a single **proof pipeline**. Compile-time
predicates are not external specification comments—they are functions. A function that returns Type
can be used at type positions, and the compiler calls it at compile time and checks the return
value. Types are propositions, compile-time evaluation is proof.

**Core thesis**: The only job of type checking at compile time is to construct and verify proof
terms. Type equality, token conflicts, dependent type reduction, compile-time predicate evaluation,
Hoare logic implications—all are different type checks in the compile-time proof pipeline, sharing
the same pipeline. The SMT solver is an acceleration module of the type checker, not an independent
trust boundary. When the compiler returns Unproven, the programmer writes a YaoXiang function as the
proof—the type checker validates it in exactly the same way as it validates any function's return
type. Everything is YaoXiang code, everything is verified by the type checker.

## Motivation

### Why deprecate RFC-022?

RFC-022 designed specifications as `//!` comment form:

```yaoxiang
max: (T: Ord) -> ((arr: Array(T, n)) -> T) = {
    //! requires: NonEmpty(n) = n > 0          ← this is a comment independent of types
    //! ensures: ExistsMax(result, arr[0..n])   ← this is a comment independent of types
}
```

This commits a fundamental error of the Curry-Howard correspondence: **splitting specifications and
types into two layers**. Comments are not types. Comments do not participate in type checking.
Comments are the mental model of "external tools".

The white paper states clearly:

> "No `//!` comments. No independent specification language. Everything is within the type system."

### Current Problems

- RFC-022's `//!` comments are external syntax independent of the type system
- Specification types and ordinary types are two separate systems, causing conceptual redundancy
- The Debug Build verification / Release Build ignore split pattern breaks unity
- The SMT solver is conventionally positioned as an external tool—YaoXiang includes it as an
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

When the compiler returns Unproven, the programmer provides a proof function—the function's return
type equals the proposition to be proven. The type checker validates it. This is the same operation
as normal type checking.

## Proposal

### 1. `{}` Is the Proof Space: Types Are Assertions, Verification Is Type Checking

YaoXiang's `{}` is the compile-time proof space. Everything inside is an assertion, and the compiler
guarantees each item is True—either proven automatically, or the programmer provides a proof
function.

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
#          parameters at signature     only assertions inside {}
#          compiler validates x > 0 at compile time when called

List: (T: Type) -> Type = { data: Array(T) }
#      ^^^^^^^^              ^^^^^^^^^^^^^^^
#      parameters at signature     compiler validates type_of(T) == Type, type_of(data) == Array(T)
```

The same pattern: `name: (params) -> Type = { assertions }`. The compiler does not distinguish "type
assertions" from "value assertions"—both are evaluation targets in the proof pipeline.

**Loop invariants don't need to be written separately. The type annotation on a variable is the
Floyd-Hoare invariant.**

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # Annotation references i—tells the compiler s's type depends on i
    mut i: UpTo(arr.len) = 0     # At initialization i=0, validate: 0 == sum(arr[0..0]) → True
    while i < arr.len {
        s += arr[i]  # Compiler validates: s_new == sum(arr[0..i+1])
        i += 1       # i changes → triggers s's dependent re-validation: s satisfies SumUpTo(arr, i_new)
    }
    return s  # s: SumUpTo(arr, arr.len) = sum(arr[0..arr.len])
}
```

The compiler generates one verification condition for the loop body—induction hypothesis (type
annotation) → assignment operation → whether the new value satisfies the type annotation. After the
proof pipeline validates the inductive step, all iterations are automatically covered. No
`: decreases`, no `: Invariant`, no inductive proof needed—the compiler decomposes induction into
local VCs for each assignment.

### 2. Pre/Post Conditions: Compile-time Predicates on Parameter Types and Return Types

Abandon RFC-022's `//! requires`/`//! ensures`. Compile-time predicates serve as type annotations on
parameters or return values.

**The parameter side is a function call.** A compile-time predicate is a function that returns Type,
and its use on the parameter side is just calling it—just like `factorial(5)`. The return value side
introduces a new concept: the return value parameter.

```yaoxiang
# Pre-condition: explicit call to compile-time predicate in parameter type
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b
#                       ^^^^^^^^^^  b is the current formal parameter name, passed to Positive as argument
#                       The compiler extracts the actual argument value at the call site, substitutes b, validates Positive(arg)
#                       Example: divide(10, 2) → validate Positive(2) = { 2 > 0 } → True
#                       Example: divide(10, 0) → validate Positive(0) = { 0 > 0 } → False → compile error

# Post-condition: return value parameter + compile-time predicate
IsMax: (T: Ord, arr: Array(T), result: T) -> Type = {
    forall j in 0..arr.len: result >= arr[j]
}

NonEmpty: (arr: Array(T)) -> Type = { arr.len > 0 }

max: (T: Ord) -> ((arr: NonEmpty(arr))) -> (result: IsMax(T, arr, result)) = {
#                                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
#                                            result is the return value parameter, value provided by return
#                                            The compiler substitutes the return value at the return point, validates the post-condition
    candidate = arr[0]
    for i in 1..arr.len {
        if arr[i] > candidate { candidate = arr[i] }
    }
    return candidate
}
```

**Key rules**:

- **Parameter side**: `b: Positive(b)` — `b` is the current formal parameter name, passed as
  argument to `Positive`. Function call syntax, zero implicit.
- **Return side**: `-> (result: IsMax(T, arr, result))` — `result` is the return value parameter,
  value provided by the `return` statement. `result` only exists in the type signature, is only
  referenced by the predicate, does not enter the function body scope, and does not appear on the
  caller side.
- **Return value parameter is optional**: not written when there is no post-condition, the signature
  is completely identical to a normal function (`-> Int`).
- **Unity**: parameters and return value parameters are the same
  concept—`formal_param_name: predicate_call(args)`, the only difference is whether the value is
  provided by the caller or by `return`.

> **Implementation note (2026-10-02)**: The return position formal parameter is recognized
> **strictly by the declared name**—`r` in `-> (r: P(...))` is the unique return value parameter.
> Free variables in the constraint that are **not in scope** are no longer substituted with the
> return value, but instead report an undefined identifier (verified:
> `g: () -> (r: Eq2(m, r)) = { return 7 }` (`m` undefined) → `error [E1001] Unknown variable: 'm'`).
> The arguments of the predicate call are substituted into the constraint according to the
> **arguments** themselves: `P(r + 100)` after substitution is "return value + 100", not "return
> value".
>
> **Known gap (2026-10-02)**: **Multi-parameter predicates with symbolic arguments at the return
> position are not yet supported**. Trigger form:
> `f: (b: Int) -> (r: SumUpTo(b, r)) = { return b * 2 }`
> (`SumUpTo: (n: Int, s: Int) -> Type = { s == n * 2 }`) →
> `error [E2031] … the refinement type constraint of 'r': SumUpTo(b, (b * 2)) cannot be statically proven`.
> Workaround: change the arguments to compile-time foldable literals (e.g. `SumUpTo(3, r)`), or
> provide a proof function returning `Type` for that predicate; unblock condition: the proof kernel
> supports "multi-parameter predicate + symbolic argument" argument substitution. See
> [RFC-027a](../review/027a-termination-explicit-measure.md) §Implementation Landing Record
> (2026-10-02, this hardening round) for details.

### 3. Path Condition Propagation: Compile-time Verification of Runtime Values

When a compile-time predicate is used at a binding position, arguments are passed explicitly by the
programmer. When runtime values enter refinement type arguments, the compiler completes verification
through path condition collection and SMT implication judgment—no need for the programmer to pass
proofs explicitly.

#### 3.1 Explicit Function Call

When a compile-time predicate is used at a binding position, the arguments are passed explicitly by
the programmer—just a function call, zero implicit.

`Positive: (x: Int) -> Type = { x > 0 }` is a compile-time predicate constructor. When it appears at
a binding position (parameter declaration, variable declaration, return type), the programmer
explicitly passes the already-bound variable name:

```yaoxiang
b: Positive(b)
// b is already declared as the current formal parameter, Positive(b) is a function call
// After normalization: b: { b > 0 }
```

The compiler does not need to implicitly fill in arguments—`b: Positive(b)` is the same as `f(5)`,
just a function call. `b` is bound as a parameter name, and its type annotation `Positive(b)`
references `b` itself—this is the standard pattern of dependent types, not an implicit expansion
rule.

**Unity with RFC-010's `self`**: RFC-010 establishes that `self` is not a keyword, just a
conventional name for parameters ("written as `p`, `this`, or `x` has exactly the same effect").
`b: Positive(b)` shares the same mechanism—the parameter name can be referenced in the type
annotation. `self` appears at the position `self: Point`, `b` appears at the position
`b: Positive(b)`, both type annotations reference the parameter itself. The difference is only in
the complexity of the type annotation, the mechanism is completely the same—after a name is bound,
the type can depend on that name.

The return type similarly uses explicit function calls:

```yaoxiang
Sorted: (arr: Array(T)) -> Type = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }

sort: (arr: Array(T)) -> (result: Sorted(result)) = { ... }
//                        ^^^^^^^^^^^^^^^^^^^^^^^
//                        result is the return value parameter, Sorted(result) is a function call
//                        The compiler substitutes the return value into result at the return point, validates Sorted(return value)
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
    // The compiler automatically obtains the assumption in this branch: { y > 0 }
    let result = divide(x, y)
    // Validation condition: (y > 0) ⇒ (y > 0)
    // Proof pipeline judges the implication holds → Proved
} else {
// This branch assumes: { !(y > 0) }
// If divide(x, y) is called, the validation condition is !(y > 0) ⇒ y > 0
    // Proof pipeline judges the implication does not hold → Disproved
}
```

This is not the compiler hardcoding special patterns—this is the natural behavior of the
compile-time proof pipeline. Each type-checking call site sends to the pipeline:

```
{background assumptions} ⇒ {validation target}
```

The proof pipeline judges the implication. Proved → pass, Disproved → compile error +
counterexample, Unproven → compile error + unsolved proposition. Background assumptions come from
the path conditions of the current program point.

#### 3.3 Assumption Stack

When analyzing control flow, the compiler maintains an assumption set for each basic block:

- **if-guard**: `if y > 0` → true branch pushes `y > 0`, false branch pushes `!(y > 0)` (if else is
  used)
- **match patterns**: `if let Some(v) = opt` → inside the branch, push `opt == Some(v)`
- **Logical conjunction**: `if x > 0 and y < 10` → inside the branch, push `x > 0` and `y < 10`
- **Function preconditions**: when calling `divide(a, b)`, `b` must satisfy the evidence of
  `Positive` either from the current assumption, or from the refinement type annotation of the
  argument itself (if `b` is already annotated as `Positive`, its type carries `b > 0`)
- **Assignment**: when `let z = y`, the refinement conditions already on `y` are propagated to `z`

All assumptions enter the compile-time proof pipeline. When entering the SMT acceleration path, they
are translated into SMT-LIB background assertions.

#### 3.4 No Static Evidence Means Compile Error

If the programmer directly writes:

```yaoxiang
divide_user_input: (x: Int, y: Int) -> Int = divide(x, y)
```

There is no assumption of `y > 0` at the current program point, and the argument `y` itself has no
`Positive` type annotation. The validation condition is:

```
{} ⇒ { y > 0 }
```

The pipeline returns `Disproved` (does not hold) → compile error:

> Cannot prove that parameter `b` satisfies `Positive` in the `divide` call. `y` comes from function
> input, with no proven bound. Consider guarding the call with an if branch:
> `if y > 0 { divide(x, y) }`.

YaoXiang does not accept runtime values directly entering refinement type arguments without
providing static evidence. This is not a limitation—this is the core of the hard safety philosophy.
Whatever the compiler cannot statically prove may not pass compilation.

#### 3.5 Relationship with the Unified Pipeline

Path condition propagation is not an additional mechanism. It is a direct extension of the
compile-time proof pipeline in control flow analysis:

| Phase                           | Responsibility                                                                                                                                             |
| ------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Path condition collection       | The compiler's control flow analysis phase annotates the assumption set for each basic block                                                               |
| Validation condition generation | When a type constraint requiring validation is encountered, merge path conditions + argument type info                                                     |
| Proof pipeline evaluation       | Compiler kernel → SMT acceleration → obtain Proved / Disproved / Unproven                                                                                  |
| Result                          | `Proved` → pass; `Disproved` → compile error + counterexample; `Unproven` → compile error + unsolved proposition (programmer can provide a proof function) |

No new components. No special rules. Path conditions are the background knowledge of the proof
pipeline—sharing the same pipeline and the same budget system as type equality and borrow
constraints.

### 4. Compile-time Proof Pipeline

All compile-time checks share the same pipeline. The core operation of the pipeline is **type
checking**—checking whether the type of a proof term equals the proposition to be proven. Everything
is type checking.

```
Encountering a Bool expression needing evaluation at compile time (i.e.: needing to construct a proof term)
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
         Type checker validates ──→ Proved ──→ Compilation passes
                  │
                  ▼
            Validation fails → Compile error: "proof does not hold"
```

#### 4.1 Proof Results: Three-Valued Algebra

Compile-time evaluation returns three results—this is the inevitable conclusion of the halting
problem, and the natural division of proof theory:

```
eval_compile_time : BoolExpr → Proved | Disproved(Model) | Unproven
```

- **Proved** → halted, proof term constructed, type check passes. Compilation continues.
- **Disproved(M)** → halted, counterexample M exists. Compile error + counterexample + source
  location.
- **Unproven** → proof not constructed within the given resource cap. Compile error + unsolved
  proposition + budget consumption report.

**Unproven ≠ False.** The compiler saying "I cannot prove it" is not equivalent to the proposition
being false—it's just beyond the current automatic proving capability. This is honesty, not a
defect.

The hard budget limit is the engineering solution to the halting problem. No knob given—giving one
would be asking the user "do you think your program will halt", the user doesn't know, and the
compiler doesn't know either.

#### 4.2 After Unproven: Programmer Writes Proof

When the compiler returns Unproven, the programmer can write a **proof function**—just a YaoXiang
function whose return type equals the proposition to be proven. The type checker validates this
function—exactly the same mechanism as it validates `add(a, b): Int`.

```
Proposition = type
Proof       = program (a value of that type)
Validation  = type checking (the only trust root)
```

The SMT solver is not an independent trust boundary—it is an **acceleration module of the type
checker**. The SMT helps find proofs, but it is the type checker that validates the proof. When SMT
returns `unsat`, the compiler reconstructs its result as a proof term verifiable by the type
checker. If reconstruction fails (the SMT's inference steps exceed the compiler kernel's inference
rules), it falls back to Unproven—the programmer can manually write a proof function.

```yaoxiang
# Proposition: a refinement property the compiler cannot automatically prove
FirstIsMin: (T: Ord, arr: Sorted(T)) -> Type = {
    forall i in 0..arr.len: arr[0] <= arr[i]
}

# Proof: the programmer writes a function whose return type is the proposition above
# The type checker validates this function—exactly the same as validating add(a,b): Int
first_is_min: (T: Ord, arr: Sorted(T)) -> FirstIsMin(T, arr) = {
    # The compiler validates here: the type of the function body = FirstIsMin(T, arr)
    ...
}
```

No AI, no export to Coq, no new concepts needed. **Properties that cannot be automatically proven at
compile time → programmer writes the proof in YaoXiang code → type checker validates.** The whole
process is a smooth gradient—the compiler does the simple proofs for you, leaving the brain for the
hard ones.

#### 4.3 Layered Dependencies within the Pipeline

The evaluators above share the same interface but have an evaluation order. Type equality is the
prerequisite for all subsequent analyses; ownership/token checks depend on type information;
refinement predicate validation depends on the results of the first two layers. The compiler
evaluates by layer, and expressions that fail at lower layers do not enter the upper layers—avoiding
wasting solving budget on type-erroneous programs.

```
Evaluation order (same pipeline, layered scheduling)
├── Layer 0: Type equality (T1 == T2)
│   └── Structural unification → if fails, subsequent is meaningless, directly return Disproved
├── Layer 1: Ownership/token conflicts
│   └── Flow-sensitive liveness analysis → if fails, memory safety does not hold, directly return Disproved
└── Layer 2: Refinement predicate / Hoare implication
    └── Compiler itself → SMT acceleration → obtain Proved / Disproved / Unproven
```

Each layer still returns `Proved/Disproved/Unproven`, sharing the same interface and the same budget
system.

### 5. Three-Layer Function Unity

| Layer                  | Execution Timing | Input      | Output | Example                                        |
| ---------------------- | ---------------- | ---------- | ------ | ---------------------------------------------- |
| Value-level function   | Runtime          | Value      | Value  | `add: (a: Int, b: Int) -> Int = a + b`         |
| Type constructor       | Compile-time     | Type/Value | Type   | `List: (T: Type) -> Type = { data: Array(T) }` |
| Compile-time predicate | Compile-time     | Value      | Type   | `Positive: (x: Int) -> Type = { x > 0 }`       |

All use the same `name: type = value` syntax. Compile-time predicates and type constructors go
through the same compile-time proof pipeline—`{}` is the proof space.

### 6. Loops: Floyd-Hoare Verification Condition Generation

Loops do not need separate `: Invariant(...)` or `: decreases(...))` annotations. The compile-time
predicate type annotation on a variable defines a Floyd-Hoare-style assertion—the compiler generates
verification conditions from the type annotation, and the proof pipeline checks whether each
assignment preserves the type.

Core mechanism: each assignment operation corresponds to a Hoare triple `{P} x := e {Q}`, with the
validation condition being `P ⇒ Q[e/x]`. The compiler generates one verification condition for the
loop body—after the proof pipeline validates the inductive step, all iterations are automatically
covered.

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # Annotation references i; at initialization i=0, validate: 0 == sum(arr[0..0]) → True
    mut i: UpTo(arr.len) = 0     # Validate: 0 <= 0 <= arr.len → True
    while i < arr.len {
        # The compiler generates one VC for the loop body. Premise: s satisfies SumUpTo(arr, i), i satisfies UpTo(arr.len).
        #
        # s += arr[i]:
        #   Validation obligation: s_new satisfies SumUpTo(arr, i) (current i unchanged)
        #   Substituting s_new = s_old + arr[i]:
        #     Need s_old + arr[i] == sum(arr[0..i+1])
        #     From induction hypothesis s_old == sum(arr[0..i]), add arr[i] to both sides:
        #     sum(arr[0..i]) + arr[i] == sum(arr[0..i+1])
        #   Compiler + SMT: linear arithmetic, millisecond-level → Proved
        #
        # i += 1:
        #   i changes → in the dependency graph, s's type annotation references i → triggers re-validation
        #   New validation target: s satisfies SumUpTo(arr, i_new)
        #   i.e. s == sum(arr[0..i_new]), guaranteed by the previous step → Proved
        s += arr[i]
        i += 1
    }
    return s  # At this point s: SumUpTo(arr, arr.len), i.e. s == sum(arr[0..arr.len])
}
```

The loop invariant is the type annotation on the variable—the programmer writes the type, the
compiler checks the inductive step. The compiler does not need to "discover" invariants, nor
"automatically do induction"—it decomposes the inductive proof into local validation conditions for
each assignment operation, and hands them over to the proof pipeline to divide and conquer.

#### 6.1 Dependency Tracking: Dependent Types on Mutable Variables

The prerequisite for the above mechanism is: the compiler knows that the type annotation
`SumUpTo(arr, i)` of `s` references `i`—when `i` changes, the type constraint of `s` also changes
accordingly. This requires the compiler to maintain a **type dependency graph between variables**.

**Data structure**:

```
TypeDepGraph: Map<VarName, Set<VarName>>
# Keys are depended variables, values are the set of variables that reference that variable in their type annotations
# Example: { i: {s}, j: {s, t}, ... }
```

**Construction**: The type checker, when processing `mut v: Pred(... x ...) = init`, parses the free
variable references in `Pred(...)`'s arguments. If the arguments reference other mutable variables
`x` in the current scope, record `x → v` in the dependency graph.

**Trigger**: When a depended variable `x` is assigned, the compiler:

1. Look up all variables `{v₁, v₂, ...}` in the dependency graph that depend on `x`
2. For each `v`, generate a validation condition: does `v`'s current value satisfy the updated type
   `Pred(... x_new ...)`
3. Send the VC into the proof pipeline

**Assignment order sensitive**: Dependency tracking naturally enforces the correct assignment order.
Taking `SumUpTo(arr, i)` as an example:

```yaoxiang
# Correct order
s += arr[i]   # s_new satisfies SumUpTo(arr, i+1)
i += 1        # i changes → re-validate s satisfies SumUpTo(arr, i_new) → True

# Wrong order—the compiler rejects
i += 1        # i changes → re-validate s satisfies SumUpTo(arr, i_new)
              # s has not yet been updated, s_old == sum(arr[0..i_old]) ≠ sum(arr[0..i_new])
              # → compile error: variable s does not satisfy type SumUpTo(arr, i_new)
s += arr[i]   # unreachable
```

**Composite dependencies**: a variable can depend on multiple variables. A type annotation
`{ v: Int; v == x + y }` depends on both `x` and `y`—any change triggers re-validation.

**Relationship with the proof pipeline**: Dependency tracking is the trigger for VC generation, not
an independent validation mechanism. It answers "when do we need to generate a VC"—the proof
pipeline answers "whether the VC holds".

### 7. Termination Check

**Scope of application: refinement types.** Termination is not an independent switch, but part of
the **verification mode**: once a type is refined (`Refined { base, constraint }), the computation
annotated by it enters verification mode, requiring termination within the mode; ordinary types that
are not refined do not enter verification mode, and do not generate any termination obligations.

Two corollaries follow:

- **Loops**: bare `while` does not enter verification mode; when a measure variable has a refinement
  annotation (e.g. `i: UpTo(n)`), it enters verification mode and must prove termination.
- **Recursion**: when a function signature has refinement (parameter refinement or return type
  contains refinement), it enters verification mode, requiring proof that the measure strictly
  decreases at each recursive call site.

Full automatic priority within the mode: the compiler first automatically explores the measure,
proving what can be proven; if it cannot be explored and no explicit measure is given, compile
error. No **annotation syntax** loophole—measures and termination propositions are written at the
type position, no new syntax like `decreases` is introduced. The form of the measure and explicit
fallback is in §6.9.

#### 6.1 Design Principles

The compiler automatically extracts information needed for termination proofs from two places:

1. **Variable type annotation**: boundary constraints in the refinement type (e.g. `UpTo(n)` gives
   upper bound `n` and lower bound `0`)
2. **Loop body operations**: operations applied to variables on each iteration

The compiler tries four measure synthesis strategies in priority order, stopping when one is found.
The four strategies are a **limited template sequence for measure exploration**, with input being
refinement constraints (strategies 1–4 all start with "variables of bounded types"), not "code
evaluated at compile time"; they are the automatic and manual sides of the same thing as the
explicit measure in §6.9.

> **Measure exploration is exploration, not inference.** Exploration only enumerates templates
> (linear rank, violation count, bounded pattern, multiplicative scaling), and does not guarantee a
> solution exists—general measure inference is generally undecidable (reduces to the halting
> problem). Therefore, cases outside the templates must be able to be given explicitly by the
> programmer (§6.9), otherwise it is a compile error.

#### 6.2 Strategy 1: Automatic Synthesis of Linear Rank Functions

When a variable has a linear bound annotation, the compiler enumerates candidate linear measures and
verifies via SMT.

```
Input:
  Variables v₁: UpTo(u₁), v₂: UpTo(u₂), ... (variables with upper and lower bounds)
  Loop condition cond
  Set of assignments in the loop body

Algorithm:
  1. Extract each variable's bounds from the type annotation: [low_i, high_i]
  2. Enumerate candidate measures: v_i, u_i - v_i, v_i - v_j, etc. linear combinations
  3. For each candidate measure m:
     - SMT verify m ≥ 0 (derived from type bounds)
     - For each execution path of the loop body, SMT verify m' < m (strictly decreasing)
  4. Find a linear combination that satisfies the conditions → termination proven
```

Coverage: any loop where a variable is assigned a linear expression (`v = a·v + b`) and has a
bounded type annotation. Includes `i += const`, `i -= const`, and binary-search-style interval
contraction:

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

#### 6.3 Strategy 2: Predicate Violation Count—Automatically Extracting Measures from Target Types <span style="color:orange">[Experimental Strategy]</span>

> ⚠️ **Current status: experimental strategy, the actual feasibility is to be determined during
> Phase 3 implementation.** This strategy is effective for adjacent swap operations (bubble sort,
> insertion sort), but cannot automatically prove for non-adjacent operations (quick sort partition,
> heap sort sift-down). See the table below for coverage boundaries. If Phase 3 verification is not
> feasible, this strategy will be removed or downgraded to future work.

Core insight: **the specifications written by the user are material for the compiler to reason
about.** The compiler does not need to have "what is sorting" built in—it reads the definition of
`Sorted` and automatically extracts the measure from the definition.

```
Input:
  Target type: Sorted(arr) = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }
  Loop body operation: adjacent element swap

Algorithm:
  1. Parse the predicate definition: forall i in range: cond(i, arr)
  2. Automatically generate the measure: violation_count = |{ i | ¬cond(i, arr) }|
  3. Analyze the effect of operations on the measure:
     - Adjacent swap arr[j], arr[j+1] = arr[j+1], arr[j]
     - Only affects index pairs j-1, j, j+1
     - If arr[j] > arr[j+1] (violates the predicate), the pair satisfies the predicate after swapping
     - violation_count decreases by at least 1
  4. Upper bound: n·(n-1)/2 (maximum adjacent inversions), lower bound: 0
  → Termination proven
```

**Current coverage**:

| Algorithm      | Operation Mode    | Strategy 2 Provable? | Reason                                                      |
| -------------- | ----------------- | :------------------: | ----------------------------------------------------------- |
| Bubble sort    | Adjacent swap     |          ✅          | violation_count strictly decreases on each swap             |
| Insertion sort | Adjacent shift    |          ✅          | Each shift eliminates one violation pair                    |
| Selection sort | Non-adjacent swap |          ❌          | A single swap may increase violation_count                  |
| Quick sort     | partition         |          ❌          | Non-adjacent swap, not guaranteed to monotonically decrease |
| Heap sort      | sift-down         |          ❌          | Tree operation, violation_count is non-monotonic            |

**Complementary strategies**: for quick sort, `low < high` interval contraction can be covered by
Strategy 1 (linear rank function)—the outer partition recursion, with intervals halving each time.
Strategy 1 and Strategy 2 cover each other complementarily, and the termination of most practical
algorithms can be proven by one of the two. But the generalization of Strategy 2 (non-adjacent
operations, tree operations) is still an open problem.

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

#### 6.4 Strategy 3: Bounded Increasing/Decreasing Patterns

`v += const` (positive constant), variable has an upper bound type annotation → measure
`upper_bound - v` decreases by `const` each time, lower bound 0. This is a degenerate case of
Strategy 1, and the compiler processes it quickly at the front.

#### 6.5 Strategy 4: Multiplicative Scaling Measure Template

`v *= const` (const > 1), variable has upper and lower bound type annotations. The compiler has a
built-in logarithmic measure template `ceil(log_const(upper/v))`, which decreases by 1 each time
multiplying by const.

```yaoxiang
mut i: Positive(i) = 1
while i < n {
    # Compiler automatically derives: measure ceil(log₂(n/i)), multiplying by 2 decreases the measure by 1 each time
    i *= 2
}
```

#### 6.6 Separation of Termination and Correctness

Termination proof and correctness proof are independent:

- **Termination**: the four strategies above automatically prove the loop exits in finite steps;
  when exploration fails, the programmer gives the measure at the type position (§6.9)
- **Correctness**: whether the loop body advances toward the target type is checked by the
  compile-time proof pipeline through validation conditions

Both pass → compilation passes. Termination proven but correctness fails → compile error +
counterexample. Correctness proven but termination cannot be proven → compile error and point out
the unanalyzable variable or operation. Both fail → compile error reports both failure reasons
separately.

#### 6.7 Termination Check for Recursive Functions

For recursive functions with refined signatures, the compiler checks that the formal parameters
decrease at each recursive call site:

```yaoxiang
gcd: (a: Int, b: NonNegative(b)) -> Int = {
    if b == 0 { return a }
    return gcd(b, a % b)  // Compiler explores: the well-founded order measure of (b, a % b) decreases → terminates
}
```

Formal parameter decrease is the strongest automatic path (structural recursion). When exploration
fails, the programmer gives the measure explicitly at the type position (§6.9).

#### 6.8 Hard Boundary

`i = f(i)` where `f` is non-invertible, non-closed, and does not preserve any
monotonicity—mathematically impossible to automatically prove termination. Compile error:

> This loop cannot automatically prove termination. The loop variable depends on the unanalyzable
> function `f`. Please use an iteration pattern that can be analyzed by the compiler, or bind a name
> to the loop and provide a measure at the type position (§6.9).

This is not a compiler failure. Whatever cannot be statically proven safe may not pass compilation.
Even if an explicit measure is given, it must be judged true by SMT to pass—a measure written
incorrectly will be sent back by a counterexample; humans can only fail to prove, not prove wrong.

#### 6.9 Explicit Measure: `Terminates`

When automatic exploration cannot find a measure, the programmer writes the measure at the **type
position**—using the same mechanism as `Positive(b)`, `IsMax(T, arr, result)` (zero new syntax):

```yaoxiang
// Measure: a normal function, unit-testable, reusable, not participating in runtime
gcd_measure: (a: Int, b: Int) -> Int = { b }

// Termination component is at the function's own type position
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// Loop: the binding name is the anchor, the measure acquires quantities within the scope by name
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
expression, the anchor is provided by the binding name `acc`—the loop can therefore be referred to,
no longer having the dead corner of "anonymous construction without referent".

> **The body tail must give a value**: the loop's value = the loop body block's value (§2.9), and
> the block's value = the tail expression. If the body tail is an assignment statement (e.g.
> `{ i = i + 1 }`), the block value is `Void`, `return acc` would be returning `Void` from `-> Int`,
> the type does not hold. Therefore the body tail in this example explicitly writes `i`. This does
> not change the semantics of the measure—the measure `n - i` is a **state expression**, independent
> of the loop's value type.

> **Implementation note (2026-10-02 logged; 2026-10-04 updated)**: The loop example in this
> section's **type side** holds (`check` 0 error, back-edge obligation Proved). **Runtime
> value-taking defect (#409) has been fixed**: `while` as a value now takes the value of the last
> iteration's body block tail expression, with the same calibration as the type side
> `block_value_ty`— `loop(4) == 4`, `countdown(3) == 30` actually run hold (`for` has the same fix).
>
> **Known remaining (zero iteration)**: when the loop body is never evaluated, there is no "last
> iteration's body value", and the loop value still takes `Void`—binding to a non-`Void` type
> position and using it at runtime would report
> `E6007 Runtime error: type mismatch in comparison Eq: Void vs Int(0)` (loud type mismatch, not
> silent wrong value). This is a **static approximation boundary**: the `block_value_ty` judgment
> gives the type of the body tail expression, and has no judgment ability for "whether the loop
> executes at least once" (which requires data flow/provability analysis), so the type side does not
> downgrade on its own initiative (downgrading would break the value type that this section's
> `Terminates` loop binding depends on). The contract for this remaining is pinned by the test
> `tests/yaoxiang/02-type-system/while_zero_iteration_void_err.yx`
> (`// expect: runtime-error E6007`); the remaining is tracked by issue
> [#409](https://github.com/ChenXu233/YaoXiang/issues/409). For the historical record of minimal
> reproduction and impact surface, see [RFC-027a](../review/027a-termination-explicit-measure.md)
> §Example Loop Section "Known Defect D6". In addition: termination Strategy 1 (linear rank
> function) takes effect again this round, so the order of "only write explicit measure when
> automatic exploration fails" in this section remains unchanged— the explicit measure is still the
> fallback after exploration failure.

**`Terminates` is a built-in predicate**, belonging to the same core primitives as `Int`, `Never`.
It is the only predicate **whose function body is written by the compiler**—its assertion ("the
measure strictly decreases at each recursive call point/loop back edge") lives in the computation
structure, and predicates written by the user cannot reference the function body or loop body, so it
cannot be expressed using the "syntactic" section's predicate definition syntax. The built-in
surface converges to this one name.

**Arity**. `Terminates(FnType, m)` and `Terminates(m)` are two arities of the same predicate, not
two constructs:

| Form                    | Anchor                 | Use case                                                         |
| ----------------------- | ---------------------- | ---------------------------------------------------------------- |
| `Terminates(m)`         | Name of the binding    | Self-recursive functions, loops—default form                     |
| `Terminates(FnType, m)` | Explicit function type | Scenarios where anchors are not unique, such as mutual recursion |

Both are essentially the same: the termination obligation always falls on "the computation annotated
by the type position where the refinement is located".

**The measure does not restrict the return type.** The measure can be an expression of any type (not
limited to natural numbers); the "strictly decreasing" on it is given by the well-founded order
available on that type. Whether the measure is well-founded (e.g. whether it is `>= 0` when
returning `Int`) is an **independent obligation**, handed to the refinement derivation or SMT the
same way as the decrease obligation; when both cannot be derived, the diagnosis **does not directly
reject**, but suggests the direction to check (whether the measure's lower bound holds, whether the
recursive parameters really go in that direction).

**Relationship with automatic exploration**: the explicit measure is not a separate pipeline, but
the input after exploration fails. After giving the measure, it still goes through the same SMT to
verify the decrease and well-foundedness; if it does not hold, an error is reported with a
counterexample.

For landing mechanisms of obligation generation, judgment pipeline, diagnosis direction, mutual
recursion measure sharing (SCC), see
[RFC-027a: Explicit Measure for Termination Check](../review/027a-termination-explicit-measure.md).

### 8. SMT Solver: Acceleration Module of the Type Checker

In traditional languages, the SMT solver is an external tool (e.g. F\* calls Z3, Dafny calls Z3). In
YaoXiang, it is an **acceleration module of the type checker**—only invoked when the compiler kernel
itself cannot directly judge. The SMT helps find proofs, but it is the type checker that validates
the proof.

**Trust model**: the type checker is the only trust root. The SMT solver is an acceleration
module—it helps find proofs, but SMT is not an independent trust boundary. The compiler trusts Z3's
`unsat` result (consistent with the F\*/Dafny route—the probability of Z3 errors is lower than the
compiler's own bug rate, which is a pragmatic engineering choice). The real unreliability is
controlled at the SMT translation layer—if the translation has bugs, the compiler will be exposed in
other tests.

**Interface**: the compiler internally translates to the SMT-LIB 2.6 standard format, rather than
binding to a specific solver API. SMT-LIB is an ISO standard, natively supported by Z3, CVC5,
MathSAT, Yices.

**Default backend**: Z3 (MIT license, the most extensive documentation and community validation).
CVC5 as an SMT-LIB-compatible alternative—users can switch via a compiler flag.

No "general solver abstraction layer"—SMT-LIB is the abstraction layer. In the future, if CVC5 has a
breakthrough in a specific theory, switching only requires swapping the binary, without changing
compiler code.

```
Compile-time Bool expression
        │
        ├── Compiler kernel can directly judge (structural equivalence, simple arithmetic,
        │   trivial formulas after constant folding)
        │   → Directly return Proved / Disproved
        │
        └── Compiler kernel cannot directly judge (quantifiers, symbolic variables)
            → Dependent type pre-reduction (factorial(5) → 120)
            → Translate to SMT-LIB format
            → Send to Z3/CVC5 (with budget limit)
            → Return value: unsat → Proved  │  sat + model → Disproved  │  unknown → Unproven
```

**Solving budget—hard limit, like stack depth**:

| Budget Dimension               | Default | Description                                                                                                                                                      |
| ------------------------------ | ------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Solving steps                  | 10,000  | Z3 is usually within a hundred steps for linear arithmetic. 10,000 steps cover 99% of practical predicates.                                                      |
| Time                           | 100ms   | A single predicate over 100ms = the user is writing a compile-time program rather than a type annotation. 100ms × 50 predicates = 5 second compilation time cap. |
| Quantifier instantiation depth | 3       | Three layers of nested quantifiers cover practical patterns. More than three layers are likely writing logic exercises.                                          |

Over budget returns Unproven, compile error + predicate location + consumption. No downgrade, no
runtime check, no silent pass.

**Why this is actually feasible**: 95% of practical predicates in engineering are linear
arithmetic—`x > 0`, `arr.len > 0`, `0 <= idx < arr.len`—all within the decidable fragment, and SMT
solvers return within milliseconds for these problems. When encountering the rare complex predicates
that exceed the budget, the programmer writes a proof function.

Dependent types do a layer of pre-reduction before SMT is called: `factorial(5)` is directly
compile-time evaluated to `120`, `append([1,2], [3])` is directly evaluated to `[1,2,3]`. These
deterministic value calculations do not consume SMT budget.

Programmers do not need to know that SMT exists. The mental model is: **the compiler can prove it,
it passes; cannot, it errors—if the compiler doesn't, you can write a function to prove it to it.**

### 9. Compile-time Predicate Composition

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

result = divide(10, 2)   # ✅ Compiler validates Positive(2) = { 2 > 0 } → True
# result = divide(10, 0)  # ❌ Compiler validates Positive(0) = { 0 > 0 } → False
```

#### 9.2 Array Access Safety

```yaoxiang
InBounds: (idx: Int, arr: Array(T)) -> Type = { 0 <= idx and idx < arr.len }

get: (arr: Array(T), idx: InBounds(idx, arr)) -> T = arr.data[idx]

arr = Array(Int)(1, 2, 3)
x = get(arr, 1)   # ✅ Compiler validates InBounds(1, arr) = { 0 <= 1 and 1 < 3 } → True
# y = get(arr, 5)  # ❌ Compiler validates InBounds(5, arr) = { 0 <= 5 and 5 < 3 } → False
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

### 11. Dispatch Pipeline: Unified Dispatch of Compile-time and Runtime

> **Implementation note (2026-10-03, #377-2)**: The **semantics** in this section holds and is
> integrated into production, but the **structure** is not a standalone `layers/dispatch.rs` module.
> Production dispatch is directly implemented by **call-site obligations** in three places of
> `checker.rs` (directly calling `layers::predicate::check_predicate`): binding site re-validation
> (`revalidate_refined`), call-site argument (`check_call_arg_refinements`), return position
> post-condition (`check_return_refinement`). The original `layers/dispatch.rs` has zero production
> call sites, and its handling of `Unproven { ProofFunctionRequired }` is "downgrade to W1080
> warning and inject Γ", which conflicts with the production implementation (`Unproven` always
> reports error, see `refined_unproven`), so it has been deleted as a whole. Note also: the "insert
> runtime check" in the Runtime row of the table in this section has **no** emission point in
> production (`RuntimeOutcome::InsertCheck` has no consumer), and Γ injection is done independently
> by the branch guard in `ownership.rs` and `inference/expressions.rs`.

`assert` and `Assert` are two sides of the same refinement type primitive. The dispatch pipeline
`dispatch` automatically decides whether to go through compile-time proof or runtime check based on
**whether the free variables of the predicate are reachable at compile time**:

| Criterion                                                                                  | Mode            | Behavior                                                                                   |
| ------------------------------------------------------------------------------------------ | --------------- | ------------------------------------------------------------------------------------------ |
| All free variables are compile-time known (generic parameters, compile-time constants)     | **CompileTime** | Enter proof pipeline: Proved → erase, Disproved → compile error, Unknown → proof required  |
| There are free variables from runtime (function parameters, external input, mut variables) | **Runtime**     | Insert runtime check, and inject refinement facts into the flow-sensitive assumption set Γ |

**Key**: "Cannot judge" ≠ "falsified". In CompileTime mode, Unknown requires proof (no silent
downgrade); in Runtime mode, the proposition is not even true or false at compile time—no matter how
strong the prover is, it cannot write a universally true proof for "the user may have entered a
negative number", and runtime check is the only sound choice. This is not because the prover is not
strong enough, it is theoretically inevitable.

### 12. Flow-sensitive Assumption Set Γ: Strongest Postcondition Propagation

The compiler maintains a flow-sensitive assumption set Γ, tracking the propositions known to hold at
each control flow point.

**SP (strongest postcondition) propagation**:

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

This is a hard requirement of soundness—the variable value has changed, and old assumptions are
invalid.

**Branch confluence**: when IF/ELSE or match branches merge, Γ takes the intersection of the
assumptions of each branch. Only propositions that hold on all paths are brought out of the branch.

### 13. Erasure Model Clarification: Witness Erasure ≠ Check Erasure

The "refinement types are **completely erased** at runtime" claim in RFC-027 refers to the **proof
witness**—proof terms verified at compile time do not generate runtime code. But the **runtime
check** inserted by dispatch in Runtime mode is retained—it is a Bool check executed at the value
level, not a type-level witness.

Summary: witness erased, check retained. The two do not conflict, and the original claim in RFC-027
remains unchanged.

## Detailed Design

### Syntax Changes

| Before (RFC-022)                      | After (This RFC)                                                                                                                                            |
| ------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `//! requires: NonEmpty(n) = n > 0`   | Compile-time predicate as parameter type `(b: Positive(b))`                                                                                                 |
| `//! ensures: ExistsMax(result, arr)` | Return type uses return value parameter `-> (result: IsMax(T, arr, result))`                                                                                |
| `/*! invariant: ... !*/`              | Compile-time predicate type annotation on variable—Floyd-Hoare invariant                                                                                    |
| `//! decreases: n`                    | Measure is written at the refinement type position (`Terminates`); the compiler first automatically explores, only requires explicit when exploration fails |
| Specification is a comment            | Specification is the type system                                                                                                                            |

### Syntax

**No new keywords for compile-time predicates.** `{}` is the proof space, completely consistent with
the existing type definition syntax. A compile-time predicate is just a function that returns
Type—`name: (params) -> Type = { assertions }`. When used, it is a function call—`Positive(b)`,
`IsMax(T, arr, result)`.

```bnf
# Compile-time predicate = function returning Type, assertions inside {} are verified by the compiler
# Uses existing function/type syntax, no new BNF rules needed
predicate ::= identifier ':' params '->' 'Type' '=' '{' assertions '}'
```

**Arguments of a predicate application must be in compile-time constant form**—literals, variables
(bound by name), type applications (recursively extracted), or **compile-time referable function
references** (function names). Arguments that cannot be converted to constant expressions report
**E1092**, mismatches between the number of arguments and the predicate's declared formal parameters
report **E1093**. Arguments are bound to the formal parameter list by position, the predicate's
arity is determined by the declaration—`Positive(x)` is unary, `IsMax(T, arr, result)` is ternary,
`Terminates(m)` and `Terminates(FnType, m)` are unary and binary—**refinement constraints are never
silently discarded** (previously, non-convertible arguments would cause constraints to silently
disappear, allowing bindings that violated the constraint to pass silently).

**New syntax concept: return value parameter**—`name` in `-> (name: Type)` is the return value
parameter.

The return value parameter is the **only one syntax concept** YaoXiang introduces on top of the
existing function syntax. Its semantics:

- The value of `name` is provided by the `return` statement
- `name` only exists in the type signature, and is referenced by post-condition predicates (e.g.
  `-> (result: IsMax(T, arr, result))`)
- `name` does not enter the function body scope, and does not appear on the caller side
- Return value parameter is **optional**—when there is no post-condition, the signature is
  completely identical to a normal function (`-> Int`), and no extra burden is introduced

The reason for introducing it: post-conditions need to reference "the value the function is about to
return". Without return value parameters, the compiler can only let the predicate reference the
return value through special rules (such as implicit variables `$result` or `__retval__`). The
return value parameter makes this reference explicit—it is just a formal parameter, except that the
value is provided by `return` rather than the caller.

**Proof functions** are not a new concept—it is just a YaoXiang function whose return type is the
proposition being asserted. When the compiler returns Unproven, the programmer provides a proof
function, and the type checker validates it in exactly the same way as it validates any function's
return type. No new syntax, no new keywords, no new rules.

> **The boundary between the two.** In the correctness domain (predicate Unproven), the proof is
> supplemented in the **body**: write a function whose return type is the proposition to be proven.
> In the termination domain (§6.9), the measure is supplemented at the **type position**: write
> `Terminates(measure)` as the type of the binding or function signature. The former is "write a
> proof for a proposition that cannot be proven", the latter is "explicitly declare a measure that
> cannot be explored"—the mechanism is the same (both are refinement type applications), but the
> location is different. The termination domain **does not** need to write a `_proof` function.

### Type System Impact

- **Type universe**: compile-time predicates are at the Type₂ layer—functions that accept values and
  return Type, the same level as type constructors
- **Generic interaction**: compile-time predicates can have generic parameters, e.g.
  `NonEmpty: (T: Type) -> (arr: Array(T)) -> Type`
- **Ownership interaction**: expressions in compile-time predicates obey ownership rules, read-only
- **Type inference**: arguments of compile-time predicates participate in HM type inference

### Runtime Representation

Compile-time predicates are **handled at runtime according to the dispatch result**:

- **CompileTime mode** (all free variables are compile-time known): the witness is completely erased
  after the proof passes. `Positive: (x: Int) -> Type = { x > 0 }`—the parameter `b: Positive(5)`'s
  runtime representation is just `Int`. The refinement condition `{ 5 > 0 }` has passed and is
  erased.
- **Runtime mode** (there are runtime free variables): retain the runtime check—execute a Bool check
  at the value level, and inject it into the flow-sensitive assumption set Γ. See §11 dispatch
  pipeline and §13 erasure model clarification for details.

Placing a compile-time predicate at a type position (e.g. `f(x: Positive(x))`) does not produce a
wrapper type, and does not allocate extra memory. But when `x` comes from runtime input, a runtime
Bool check **will** be inserted.

**Interaction constraints with `ref`**: compile-time predicates can only reference values with
immutable borrows or whose ownership has been transferred. Compile-time predicates that reference
mutably borrowed values—the compiler cannot guarantee at compile time that the validation result
still holds at runtime—such usages directly report a compile error.

### Compiler Changes

1. **Parser**: compile-time predicates use standard function syntax, no additional parsing rules
   needed
2. **Compile-time proof pipeline**: unified Proved/Disproved/Unproven return interface, automatic
   strategy selection
3. **SMT acceleration module**: SMT-LIB 2.6 translation layer, default backend Z3, CVC5 as
   alternative
4. **Type checker kernel**: inference rule implementation—structural equivalence, βδι-reduction,
   universal quantifier introduction/elimination. This is the only trust root, both SMT and
   programmer proofs are validated through this
5. **Verification condition generation**: WP/SP calculus + loop invariant proof obligations
6. **Error reporting**: counterexample formatting + unsolved proposition report + source location
   correlation

### Backward Compatibility

- ✅ Code that does not use compile-time predicates is completely unchanged
- ✅ Compile-time predicates have zero runtime overhead in CompileTime mode, and only retain
  necessary Bool checks in Runtime mode
- ⚠️ RFC-022's `//!` syntax is no longer supported—but 022 was never implemented, so there is no
  migration burden

## Trade-offs

### Advantages

- **Curry-Howard correspondence fully realized**: types are propositions, programs are proofs,
  `name: Proposition = Proof`
- **Unity**: compile-time predicates and normal functions use completely identical syntax, no
  concept splitting
- **SMT transparent**: programmers do not need to know that SMT exists, the mental model is
  consistent with type checking
- **Progressive adoption**: can start from one compile-time predicate and gradually increase
  coverage
- **Minimal runtime overhead**: zero overhead in CompileTime mode, only retain necessary Bool checks
  in Runtime mode

### Disadvantages

- **Compilation time**: SMT solving increases compilation time, but the hard budget limit guarantees
  a controllable upper bound
- **Automatic proof boundary**: complex predicates beyond first-order linear arithmetic may require
  the programmer to write proof functions. This is not a language defect—this is the inevitable
  conclusion of the halting problem. The compiler honestly reports Unproven rather than falsely
  reporting True/False
- **Learning curve**: writing effective compile-time predicates and proof functions requires
  understanding the basic intuition of the Curry-Howard correspondence
- **Implementation complexity**: the unification of the compile-time proof pipeline requires careful
  design

### Risk Mitigation

- Hard SMT solving budget limit (10,000 steps / 100ms / instantiation depth 3), over budget returns
  Unproven
- Dependent type pre-reduction: deterministic value calculations are eaten first, SMT only chews on
  the non-deterministic part
- Unproven is not a dead end: in the correctness domain, write a proof function (return type is the
  proposition), in the termination domain, give the measure at the type position (§6.9)—both are
  validated by the type checker
- Incremental validation: only validate changed modules
- Clear error messages + counterexample display + budget consumption report + unsolved proposition +
  suggestions (if the compiler can give them)

## Alternatives

| Option                                                             | Why not choose                                                                                                                                                                 |
| ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| RFC-022: `//!` comment-style specification                         | Specification and type are split, violating the Curry-Howard correspondence                                                                                                    |
| Standalone specification file (e.g. CVL)                           | Specification and code are separated, increasing maintenance cost                                                                                                              |
| Runtime-only assertions                                            | Cannot statically guarantee correctness                                                                                                                                        |
| External proof assistant (e.g. Coq)                                | Disconnected from the compiler, requires independent proof language and trust boundary. YaoXiang's choice: the proof is YaoXiang code, the type checker is the only trust root |
| **This proposal: compile-time predicates as first-class citizens** | ✅                                                                                                                                                                             |

## Implementation Strategy

### Phase Division

| Phase       | Content                                                                                                                                                                 |
| ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Phase 1** | Compiler kernel: structural equivalence + βδι-reduction + universal quantifier introduction/elimination. Supports simple arithmetic predicates (`x > 0`, `arr.len > 0`) |
| **Phase 2** | SMT-LIB translation layer + Z3/CVC5 integration. Pipeline returns Proved/Disproved/Unproven. Supports programmer-written proof functions when Unproven                  |
| **Phase 3** | Loop invariant VC generation + termination check (four measure exploration strategies + `Terminates` explicit measure, §6, §6.9)                                        |
| **Phase 4** | Incremental validation + caching + IDE support                                                                                                                          |

### Dependencies

- RFC-010: Unified Type Syntax — compile-time predicates are based on `name: type = value`
- RFC-011: Generic Type System — compile-time predicates can have generic parameters
- RFC-009: Ownership Model — compile-time predicate expressions obey ownership rules

## Open Questions

- [x] **Z3 availability for wasm32 target**: no Z3 under wasm (all SMT code is excluded by
      `cfg(not(target_arch = "wasm32"))`). Impact scope and downgrade direction: - `ownership.rs`
      `smt_cut` always returns `false` → back edge traversed → **conservative rejection** (sound
      direction, just less narrowing) - Termination check Strategy 1, predicate layer 2b/3 skipped
      as a whole

  **Correction (2026-10-01)**: the original text said these paths are "currently **unreachable**"
  (because `predicate_defs` production is never populated, `parser` does not produce
  `MonoType::Refined`, `with_solver` production is never called), which no longer holds—all three
  have landed (predicate definition registration #377-3; refinement type normalization; production
  pipeline injects solver). Therefore, the difference between wasm and native is no longer limited
  to the precision of `smt_cut`:

  | Path                                        | native   | wasm       | Downgrade direction                 |
  | ------------------------------------------- | -------- | ---------- | ----------------------------------- |
  | Termination check explicit measure judgment | Judgment | `Unjudged` | Do not report E4022 (less reported) |
  | Well-foundedness judgment                   | Judgment | `Unjudged` | Do not report E4022 (less reported) |
  | Refinement predicate SMT implication layer  | Executed | Skipped    | Falls to `Unproven`                 |

  All are **conservative** directions (less narrowing, less reported), not affecting soundness. If
  precision consistency is required, see issue #376 for the plan and cost (conclusion: prefer Z3
  instance on the JS side, rather than linking Z3 into the main wasm— the latter requires changing
  the emcc build system and the product grows from 3 MB to ~20 MB).

- [x] **SMT solver selection**: default Z3 (MIT license, the most extensive validation). CVC5 as an
      SMT-LIB-compatible alternative, switched via a compiler flag. The compiler internally
      translates to the SMT-LIB 2.6 standard format—SMT-LIB is the abstraction layer, no custom
      general solver interface.
- [x] **Specific values of the solving budget**: 10,000 steps / 100ms / quantifier instantiation
      depth 3. Fixed inside the compiler, no knob. If actual use cases prove insufficient in real
      practice (not "the user wrote it wrong"), adjust later.
- [x] **Quantifier support scope**: the language does not restrict the quantifier order at the
      language level. Compile-time predicates accept Type arguments—Type includes function types—so
      higher-order quantifiers are a natural corollary of the type system, no special syntax
      required. The SMT solver can automatically judge first-order quantifiers (forall/exists,
      supports interleaved nesting, limited by the budget depth of 3). Higher-order quantifiers: SMT
      returns Unproven, the compiler prompts "this predicate is beyond the scope of automatic proof,
      please provide a proof function". The programmer writes a YaoXiang function whose return type
      equals the proposition—the type checker validates the function. No external export, no AI, no
      interactive proof mode required. Everything is YaoXiang code, everything is validated by the
      type checker.
- [x] **Counterexample formatting**: source variable names are used directly as SMT variable names
      (with module prefixes to avoid conflicts). When Z3 model returns, look up by variable name.
      Output format: variable name = specific value + source location + predicate definition
      location. No complex mapping layer.
- [x] ~~**Interaction between `ref` smart pointers and compile-time predicates?**~~ → Decided:
      compile-time predicates only allow values with immutable borrows or whose ownership has been
      transferred. Values with mutable borrows cannot appear in compile-time predicates.
- [x] **Extension of the forall predicate violation count measure to non-adjacent operations?** →
      Not extended. The current coverage (adjacent swap, adjacent shift) is complementarily covered
      by Strategy 1 (linear rank function)—the outer interval contraction of quick sort is backed up
      by Strategy 1, and heap sort is backed up by Strategy 1 (array index pattern). Loops whose
      termination cannot be proven by any strategy are directly reported as errors by the
      compiler—this is the hard safety philosophy, not a defect. If there is a real scenario (not
      academic construction) in the future where all four strategies cannot cover an algorithm, then
      reconsider. → **Re-discussion triggered (2026-09-14, #318)**: non-structural recursion
      (gcd-like non-directly decreasing, mutual recursion, merge partition) are exactly the real
      scenarios anticipated by this clause—the template sequence of measure exploration cannot
      automatically take them, and they cannot be rewritten into analyzable iteration patterns
      without sacrificing readability. Conclusion: the hard safety philosophy remains unchanged (a
      measure written incorrectly is still sent back by SMT counterexamples; humans can only fail to
      prove, not prove wrong), but "exploration failure means rejection" is relaxed to "exploration
      failure allows the programmer to explicitly give the measure at the type position"—see §6.9.
- [x] **Combinatorial explosion of linear rank function enumeration**: the candidate enumeration
      upper limit is 3 bounded variables. When ≤3, enumerate all linear combinations and verify one
      by one via SMT. When >3, only try single-variable measures (`v_i`, `u_i - v_i`), and if it
      fails, directly report a compile error—prompting the programmer that "the loop has >3 bounded
      variables, and the compiler cannot automatically synthesize multi-variable measures". This is
      not an engineering compromise—it is forcing the programmer to write simpler loops.

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
│ (formal     │    │ (kept in    │
│  design)    │    │  place)     │
└─────────────┘    └─────────────┘
```
