---
title: 'RFC-027: Compile-time Predicates and Unified Static Verification'
status: 'Accepted'
author: 'ChenXu'
created: '2026-06-07'
updated: '2026-09-14'
impl_status: 'in_progress'
impl_detail:
  'Phase 1-2 completed, Phase 3 partially completed, Phase 4 partially completed. All 6 phases of
  the unified assert/Assert scheme implemented (#157-#162 closed): Never type, IsTrue bridging,
  flow-sensitive Γ + kill set, type-level recursion, universe stratification weak check, dispatch
  pipeline.'
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
> [RFC-022: Hoare Logic Static Verification Support (Specification Comments and Specification Types)](../deprecated/022-hoare-logic-static-verification.md)
> — Deprecated

## Summary

This RFC proposes introducing **compile-time predicates** as first-class citizens in YaoXiang,
unifying all compile-time static verification into a single **proof pipeline**. Compile-time
predicates are not external specification annotations—they are functions. A function that returns
Type, usable in type positions, called and checked by the compiler at compile time. Types are
propositions, compile-time evaluation is proof.

**Core argument**: The sole job of type checking at compile time is to construct and verify proof
terms. Type equality, token conflicts, dependent type reduction, compile-time predicate evaluation,
Hoare logic entailment—all are different type checks within the compile-time proof pipeline, sharing
the same pipeline. The SMT solver is an acceleration module of the type checker, not an independent
trust boundary. When the compiler returns Unproven, the programmer writes a YaoXiang function as the
proof—the type checker validates it exactly the same way it validates any function's return type.
Everything is YaoXiang code, everything is verified by the type checker.

## Motivation

### Why Deprecate RFC-022?

RFC-022 designed specifications as `//!` comments:

```yaoxiang
max: (T: Ord) -> ((arr: Array(T, n)) -> T) = {
    //! requires: NonEmpty(n) = n > 0          ← This is an annotation independent of types
    //! ensures: ExistsMax(result, arr[0..n])   ← This is an annotation independent of types
}
```

This commits the fundamental error of the Curry-Howard isomorphism: **splitting specifications and
types into two layers**. Annotations are not types. Annotations do not participate in type checking.
The "external tool" mental model.

The white paper makes it clear:

> "No `//!` annotations. No independent specification language. Everything is within the type
> system."

### Current Problems

- RFC-022's `//!` annotations are external syntax independent of the type system
- Specification types and ordinary types are two systems, causing conceptual redundancy
- The Debug Build verification / Release Build ignore split pattern breaks unity
- SMT solvers are traditionally positioned as external tools—YaoXiang builds them in as acceleration
  modules of the type checker
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
equals the proposition to be proved. The type checker validates it. This is the same operation as
ordinary type checking.

## Proposal

### 1. `{}` Is the Proof Space: Types Are Assertions, Verification Is Type Checking

YaoXiang's `{}` is the compile-time proof space. Everything inside it is an assertion, and the
compiler guarantees each item is True—either automatically proved or provided by the programmer as a
proof function.

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
#          Parameter in signature  Only assertions in {}
#          Compiler validates x > 0 at compile-time call

List: (T: Type) -> Type = { data: Array(T) }
#      ^^^^^^^^              ^^^^^^^^^^^^^^^
#      Parameter in signature  Compiler validates type_of(T) == Type, type_of(data) == Array(T)
```

The same pattern: `name: (params) -> Type = { assertions }`. The compiler does not distinguish "type
assertions" from "value assertions"—both are evaluation targets in the proof pipeline.

**Loop invariants need not be written separately. Type annotations on variables are Floyd-Hoare
invariants.**

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # Annotation references i—tells the compiler s's type depends on i
    mut i: UpTo(arr.len) = 0     # At init i=0, verify: 0 == sum(arr[0..0]) → True
    while i < arr.len {
        s += arr[i]  # Compiler verifies: s_new == sum(arr[0..i+1])
        i += 1       # i changes → triggers s dependency re-verification: s satisfies SumUpTo(arr, i_new)
    }
    return s  # s: SumUpTo(arr, arr.len) = sum(arr[0..arr.len])
}
```

The compiler generates a verification condition once for the loop body—inductive hypothesis (type
annotation) → assignment operation → whether the new value satisfies the type annotation. After the
proof pipeline verifies the inductive step holds, all iterations are automatically covered. No
`: decreases`, no `: Invariant`, no inductive proof needed—the compiler breaks induction into local
VCs for each assignment.

### 2. Pre/Postconditions: Compile-time Predicates on Parameter and Return Types

Abandon RFC-022's `//! requires`/`//! ensures`. Compile-time predicates serve as annotations on
parameter or return types.

**The parameter side is a function call.** A compile-time predicate is a function that returns Type,
and its use on the parameter side is calling it—just like `factorial(5)`. The return side introduces
a new concept: the return value parameter.

```yaoxiang
# Precondition: explicitly call compile-time predicate in parameter type
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b
#                       ^^^^^^^^^^  b is the current parameter name, passed to Positive as argument
#                       Compiler extracts argument value at call site, substitutes b, validates Positive(arg)
#                       Ex: divide(10, 2) → validate Positive(2) = { 2 > 0 } → True
#                       Ex: divide(10, 0) → validate Positive(0) = { 0 > 0 } → False → Compile error

# Postcondition: return value parameter + compile-time predicate
IsMax: (T: Ord, arr: Array(T), result: T) -> Type = {
    forall j in 0..arr.len: result >= arr[j]
}

NonEmpty: (arr: Array(T)) -> Type = { arr.len > 0 }

max: (T: Ord) -> ((arr: NonEmpty(arr))) -> (result: IsMax(T, arr, result)) = {
#                                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
#                                            result is the return value parameter, value provided by return
#                                            Compiler substitutes the return value at return point, validates postcondition
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
  value provided by the `return` statement. `result` only exists in the type signature, only
  referenced by predicates, does not enter the function body scope, does not appear at the caller.
- **Return value parameter is optional**: when no postcondition is needed, signature is identical to
  an ordinary function (`-> Int`).
- **Unity**: parameter and return value parameters are the same
  concept—`param_name: predicate_call(param_name)`, differing only in whether the value is provided
  by the caller or by `return`.

### 3. Path Condition Propagation: Compile-time Verification of Runtime Values

When a compile-time predicate is used at a binding position, arguments are explicitly passed by the
programmer. When a runtime value enters a refinement type parameter, the compiler completes
verification through path condition collection and SMT entailment judgment—no explicit proof passing
required.

#### 3.1 Explicit Function Call

When a compile-time predicate is used at a binding position, arguments are explicitly passed by the
programmer—it is a function call, zero implicit.

`Positive: (x: Int) -> Type = { x > 0 }` is a compile-time predicate constructor. When it appears in
a binding position (parameter declaration, variable declaration, return type), the programmer
explicitly passes an already-bound variable name:

```yaoxiang
b: Positive(b)
// b is already declared as the current parameter, Positive(b) is a function call
// After normalization: b: { b > 0 }
```

No need for the compiler to implicitly fill in arguments—`b: Positive(b)` is just like `f(5)`, a
function call. `b` is bound as a parameter name, and its type annotation `Positive(b)` references
`b` itself—this is the standard pattern of dependent types, not an implicit expansion rule.

**Unity with RFC-010 `self`**: RFC-010 establishes that `self` is not a keyword, just a conventional
parameter name ("writing `p`, `this`, `x` is exactly the same effect"). `b: Positive(b)` shares the
same mechanism—parameter names can be referenced in type annotations. `self` appears in the position
of `self: Point`, `b` appears in the position of `b: Positive(b)`, both type annotations reference
the parameter itself. The difference is only in the complexity of the type annotation, the mechanism
is exactly the same—after a name is bound, the type can depend on that name.

The return type also uses explicit function calls:

```yaoxiang
Sorted: (arr: Array(T)) -> Type = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }

sort: (arr: Array(T)) -> (result: Sorted(result)) = { ... }
//                        ^^^^^^^^^^^^^^^^^^^^^^^
//                        result is the return value parameter, Sorted(result) is a function call
//                        Compiler substitutes the return value into result at return point, validates Sorted(return value)
```

The same applies to local variable declarations:

```yaoxiang
let x: Positive(x) = 5
// x is bound to 5, Positive(5) → { 5 > 0 } → True → Pass

// let y: Positive(y) = 0
// y is bound to 0, Positive(0) → { 0 > 0 } → False → Compile error
```

#### 3.2 Path Condition Collection

When a runtime value appears in a conditional branch, the compiler automatically collects path
conditions, forming the **assumption set** for the current scope. These assumptions participate in
verification as background knowledge for compile-time Bool evaluation.

```yaoxiang
if y > 0 {
    // Compiler automatically obtains the assumption in this branch: { y > 0 }
    let result = divide(x, y)
    // Validation condition: (y > 0) ⇒ (y > 0)
    // Proof pipeline judges entailment holds → Proved
} else {
// This branch assumes: { !(y > 0) }
// If divide(x, y) is called, validation condition is !(y > 0) ⇒ y > 0
    // Proof pipeline judges not entailed → Disproved
}
```

This is not a hard-coded special pattern in the compiler—this is the natural behavior of the
compile-time proof pipeline. Each type check call site sends to the pipeline:

```
{background assumptions} ⇒ {validation target}
```

The proof pipeline judges entailment. Proved → pass, Disproved → compile error + counterexample,
Unproven → compile error + unsolved proposition. Background assumptions come from the path condition
at the current program point.

#### 3.3 Assumption Stack

When the compiler analyzes control flow, it maintains an assumption set for each basic block:

- **if-guard**: `if y > 0` → true branch pushes `y > 0`, false branch pushes `!(y > 0)` (if else is
  used)
- **match pattern**: `if let Some(v) = opt` → push `opt == Some(v)` inside the branch
- **Logical conjunction**: `if x > 0 and y < 10` → push `x > 0` and `y < 10` inside the branch
- **Function preconditions**: when calling `divide(a, b)`, evidence that `b` satisfies `Positive`
  must come either from current assumptions or from the argument's own refinement type annotation
  (`b` is annotated as `Positive` then its type carries `b > 0`)
- **Assignment**: `let z = y`, refinement conditions already on `y` are transferred to `z`

All assumptions enter the compile-time proof pipeline. When entering the SMT acceleration path, they
are translated into SMT-LIB background assertions.

#### 3.4 No Static Evidence Then Compile Error

If the programmer directly writes:

```yaoxiang
divide_user_input: (x: Int, y: Int) -> Int = divide(x, y)
```

The current program point has no assumption of `y > 0`, and the argument `y` itself has no
`Positive` type annotation. The validation condition is:

```
{} ⇒ { y > 0 }
```

The pipeline returns `Disproved` (not entailed) → compile error:

> Cannot prove that parameter `b` satisfies `Positive` in the `divide` call. `y` comes from function
> input, with no proven bound. Consider guarding the call with an if branch:
> `if y > 0 { divide(x, y) }`.

YaoXiang does not accept runtime values directly entering refinement type parameters without
providing static evidence. This is not a restriction—this is the core of hard safety philosophy.
Whatever the compiler cannot statically prove shall not pass compilation.

#### 3.5 Relationship with the Unified Pipeline

Path condition propagation is not an additional mechanism. It is the direct extension of the
compile-time proof pipeline over control flow analysis:

| Stage                           | Responsibility                                                                                                                                           |
| ------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Path condition collection       | Compiler control flow analysis stage, annotates assumption set for each basic block                                                                      |
| Validation condition generation | When encountering type constraints to validate, merge path conditions + argument type information                                                        |
| Proof pipeline evaluation       | Compiler kernel → SMT acceleration → Proved / Disproved / Unproven                                                                                       |
| Result                          | `Proved` → pass; `Disproved` → compile error + counterexample; `Unproven` → compile error + unsolved proposition (programmer can provide proof function) |

No new components. No special rules. Path conditions are background knowledge of the proof
pipeline—sharing the same pipeline and budget system as type equality and borrow constraints.

### 4. Compile-time Proof Pipeline

All compile-time checks share the same pipeline. The core operation of the pipeline is **type
checking**—checking whether the type of a proof term equals the proposition to be proved. Everything
is type checking.

```
Compile-time encounters Bool expression to evaluate (i.e.: needs to construct a proof term)
        │
        ├── Type equality (T1 == T2)
        │   → Compiler judges directly (structural equivalence)
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
        └── Hoare logic entailment (P ⇒ Q)
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
            Validation failed → Compile error: "Proof does not hold"
```

#### 4.1 Proof Results: A Three-valued Algebra

Compile-time evaluation returns three results—this is the inevitable conclusion of the halting
problem, and the natural partition of proof theory:

```
eval_compile_time : BoolExpr → Proved | Disproved(Model) | Unproven
```

- **Proved** → Halts, proof term constructed, type check passes. Compilation continues.
- **Disproved(M)** → Halts, counterexample M exists. Compile error + counterexample + source
  location.
- **Unproven** → Within given resource upper bound, no proof was constructed. Compile error +
  unsolved proposition + budget consumption report.

**Unproven ≠ False.** The compiler saying "I cannot prove it" is not equivalent to the proposition
being false—just beyond current automatic proof capability. This is honesty, not a defect.

The hard budget limit is the engineering solution to the halting problem. No knob—giving one would
be asking the user "do you think your program will halt," and the user doesn't know, and neither
does the compiler.

#### 4.2 After Unproven: Programmer Writes Proof

When the compiler returns Unproven, the programmer can write a **proof function**—which is a
YaoXiang function whose return type equals the proposition to be proved. The type checker validates
this function—the same mechanism as validating `add(a, b): Int`.

```
Proposition  = Type
Proof        = Program (a value of that type)
Validation   = Type checking (the only trust root)
```

The SMT solver is not an independent trust boundary—it is an **acceleration module of the type
checker**. SMT helps find proofs, but validating the proof is always the type checker. When SMT
returns `unsat`, the compiler reconstructs its result as a proof term verifiable by the type
checker. If reconstruction fails (SMT's reasoning steps exceed the compiler kernel's inference
rules), fall back to Unproven—the programmer can manually write a proof function.

```yaoxiang
# Proposition: refinement property the compiler cannot automatically prove
FirstIsMin: (T: Ord, arr: Sorted(T)) -> Type = {
    forall i in 0..arr.len: arr[0] <= arr[i]
}

# Proof: programmer writes a function, return type is the above proposition
# Type checker validates this function—exactly the same as validating add(a,b): Int
first_is_min: (T: Ord, arr: Sorted(T)) -> FirstIsMin(T, arr) = {
    # Compiler validates here: type of function body = FirstIsMin(T, arr)
    ...
}
```

No AI, no export to Coq, no new concepts. **Compile-time cannot automatically prove property →
programmer writes proof in YaoXiang code → type checker validates.** The entire process is a smooth
gradient—the compiler does simple proofs for you, leaving the brain for the hard ones.

#### 4.3 Layered Dependencies within the Pipeline

The above evaluators share the same interface but have an evaluation order. Type equality is the
prerequisite for all subsequent analyses; ownership/token checks depend on type information;
refinement predicate validation depends on the first two layers' results. The compiler evaluates
layer by layer; expressions failing in lower layers do not enter upper layers—avoiding wasting
solver budget on type-error programs.

```
Evaluation order (same pipeline, layered scheduling)
├── Layer 0: Type equality (T1 == T2)
│   └── Structural unification → failure means subsequent is meaningless, directly return Disproved
├── Layer 1: Ownership/token conflicts
│   └── Flow-sensitive liveness analysis → failure means memory safety doesn't hold, directly return Disproved
└── Layer 2: Refinement predicates / Hoare entailment
    └── Compiler itself → SMT acceleration → Proved / Disproved / Unproven
```

Each layer still returns `Proved/Disproved/Unproven`, sharing the same interface and budget system.

### 5. Three-layer Function Unity

| Layer                  | Timing       | Input        | Output | Example                                        |
| ---------------------- | ------------ | ------------ | ------ | ---------------------------------------------- |
| Value-level function   | Runtime      | Values       | Value  | `add: (a: Int, b: Int) -> Int = a + b`         |
| Type constructor       | Compile time | Types/Values | Type   | `List: (T: Type) -> Type = { data: Array(T) }` |
| Compile-time predicate | Compile time | Values       | Type   | `Positive: (x: Int) -> Type = { x > 0 }`       |

All use the same `name: type = value` syntax. Compile-time predicates and type constructors go
through the same compile-time proof pipeline—`{}` is the proof space.

### 6. Loops: Floyd-Hoare Verification Condition Generation

Loops do not need separate `: Invariant(...)` or `: decreases(...)` annotations. Compile-time
predicate type annotations on variables define Floyd-Hoare style assertions—the compiler generates
verification conditions from type annotations, and the proof pipeline checks whether each assignment
preserves the type.

Core mechanism: each assignment operation corresponds to a Hoare triple `{P} x := e {Q}`, the
validation condition is `P ⇒ Q[e/x]`. The compiler generates a verification condition once for the
loop body—after the proof pipeline validates the inductive step holds, all iterations are
automatically covered.

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # Annotation references i; at init i=0, validate: 0 == sum(arr[0..0]) → True
    mut i: UpTo(arr.len) = 0     # Validate: 0 <= 0 <= arr.len → True
    while i < arr.len {
        # Compiler generates one VC for the loop body. Precondition: s satisfies SumUpTo(arr, i), i satisfies UpTo(arr.len).
        #
        # s += arr[i]:
        #   Validation obligation: s_new satisfies SumUpTo(arr, i) (current i unchanged)
        #   Substitute s_new = s_old + arr[i]:
        #     Need s_old + arr[i] == sum(arr[0..i+1])
        #     From inductive hypothesis s_old == sum(arr[0..i]), add arr[i] to both sides:
        #     sum(arr[0..i]) + arr[i] == sum(arr[0..i+1])
        #   Compiler + SMT: linear arithmetic, millisecond-level → Proved
        #
        # i += 1:
        #   i changes → dependency graph shows s's type annotation references i → trigger re-validation
        #   New validation target: s satisfies SumUpTo(arr, i_new)
        #   i.e., s == sum(arr[0..i_new]), guaranteed by previous step → Proved
        s += arr[i]
        i += 1
    }
    return s  # At this point s: SumUpTo(arr, arr.len), i.e., s == sum(arr[0..arr.len])
}
```

Loop invariants are type annotations on variables—programmer writes types, compiler checks inductive
steps. The compiler doesn't need to "discover" invariants, nor "auto-induct"—it breaks inductive
proofs into local verification conditions for each assignment, divide-and-conquer handed to the
proof pipeline.

#### 6.1 Dependency Tracking: Dependent Types on Mutable Variables

The premise of the above mechanism is: the compiler knows that the type annotation `SumUpTo(arr, i)`
of `s` references `i`—when `i` changes, `s`'s type constraint changes accordingly. This requires the
compiler to maintain a **type dependency graph between variables**.

**Data structure**:

```
TypeDepGraph: Map<VarName, Set<VarName>>
# Key is the depended-on variable, value is the set of variables that reference it in type annotations
# Ex: { i: {s}, j: {s, t}, ... }
```

**Construction**: when the type checker processes `mut v: Pred(... x ...) = init`, it parses free
variable references in the `Pred(...)` arguments. If the arguments reference another mutable
variable `x` in the current scope, it records `x → v` in the dependency graph.

**Trigger**: when a depended-on variable `x` is assigned, the compiler:

1. Looks up all variables `{v₁, v₂, ...}` in the dependency graph that depend on `x`
2. For each `v`, generates a validation condition:
   `does v's current value satisfy the updated type Pred(... x_new ...)`
3. Sends the VC to the proof pipeline

**Assignment order sensitivity**: dependency tracking naturally enforces the correct assignment
order. Take `SumUpTo(arr, i)` as an example:

```yaoxiang
# Correct order
s += arr[i]   # s_new satisfies SumUpTo(arr, i+1)
i += 1        # i changes → re-validate s satisfies SumUpTo(arr, i_new) → True

# Wrong order—compiler rejects
i += 1        # i changes → re-validate s satisfies SumUpTo(arr, i_new)
              # s not yet updated, s_old == sum(arr[0..i_old]) ≠ sum(arr[0..i_new])
              # → Compile error: variable s does not satisfy type SumUpTo(arr, i_new)
s += arr[i]   # Unreachable
```

**Combined dependencies**: a variable can depend on multiple variables. The type annotation
`{ v: Int; v == x + y }` depends on both `x` and `y`—either change triggers re-validation.

**Relationship with proof pipeline**: dependency tracking is the trigger for VC generation, not an
independent validation mechanism. It answers "when is VC generation needed"—the proof pipeline
answers "does the VC hold".

### 7. Termination Checking

**Scope of application: compile-time evaluation.** Termination checking only targets loops and
recursion involved in compile-time evaluation (triggered by dependency tracking)—the code the
compiler itself must execute must be provably total; runtime code does not require termination
proofs (§6.7 scope table). Within scope, compile-time full automation is preferred: loops the
compiler can prove pass through, those automatic strategies cannot prove directly report compile
errors—the programmer must make the compiler able to automatically analyze loop termination. No
**annotation syntax** escape hatch (not introducing `decreases`-like annotations, what is
unsupported is always a syntax position, not a function).

> **Revision (RFC-027a, 2026-09-14)**: ① Scope explicitly limited to compile-time evaluation—the
> original text didn't qualify "loops" by position, creating ambiguity with the §6.7 scope table;
> the implementation layer's unconditional full check is narrowed by this. ② For termination
> obligations that automatic strategies cannot prove, programmers can provide measure functions and
> termination proof functions (`Terminates`) as fallback—all ordinary YaoXiang functions, zero new
> syntax, type checker validates. Full-automation preference unchanged, fallback only after
> Unproven. See
> [RFC-027a: Termination Proof Function Fallback](../draft/027a-termination-proof-fallback.md).

#### 6.1 Design Principles

The compiler automatically extracts the information needed for termination proofs from two places:

1. **Variable type annotations**: boundary constraints in refinement types (e.g., `UpTo(n)` gives
   upper bound `n` and lower bound `0`)
2. **Loop body operations**: operations applied to variables on each iteration

The compiler tries four measure synthesis strategies in order of priority, stopping when one is
found.

#### 6.2 Strategy 1: Linear Rank Function Automatic Synthesis

When variables have linear bound annotations, the compiler enumerates candidate linear measures and
verifies with SMT.

```
Input:
  Variables v₁: UpTo(u₁), v₂: UpTo(u₂), ... (variables with upper/lower bounds)
  Loop condition cond
  Set of assignments in loop body

Algorithm:
  1. Extract each variable's bounds from type annotations: [low_i, high_i]
  2. Enumerate candidate measures: v_i, u_i - v_i, v_i - v_j, etc. linear combinations
  3. For each candidate measure m:
     - SMT validates m ≥ 0 (derived from type bounds)
     - For each execution path in loop body, SMT validates m' < m (strictly decreasing)
  4. Find a satisfying linear combination → termination proved
```

Coverage scope: any loop where a variable is assigned a linear expression (`v = a·v + b`) and has
bounded type annotation. Includes `i += const`, `i -= const`, and binary search-style interval
shrinking:

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

#### 6.3 Strategy 2: Predicate Violation Counting—Automatic Measure Extraction from Target Type <span style="color:orange">【Experimental Strategy】</span>

> ⚠️ **Current status: experimental strategy, decide whether to include based on actual feasibility
> when implementing Phase 3.** This strategy is effective for adjacent swap operations (bubble sort,
> insertion sort), but cannot automatically prove non-adjacent operations (quicksort partition,
> heapsort sift-down). Coverage boundary see table below. If Phase 3 validation proves infeasible,
> this strategy will be removed or downgraded to future work.

Core insight: **the specifications users write are material for compiler reasoning.** The compiler
doesn't need to have built-in "what is sorting"—it reads the definition of `Sorted` and
automatically extracts measures from it.

```
Input:
  Target type: Sorted(arr) = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }
  Loop body operations: adjacent element swaps

Algorithm:
  1. Parse predicate definition: forall i in range: cond(i, arr)
  2. Automatically generate measure: violation_count = |{ i | ¬cond(i, arr) }|
  3. Analyze operation's effect on measure:
     - Adjacent swap arr[j], arr[j+1] = arr[j+1], arr[j]
     - Only affects index pairs j-1, j, j+1
     - If arr[j] > arr[j+1] (violates predicate), after swap this pair satisfies predicate
     - violation_count decreases by at least 1
  4. Upper bound: n·(n-1)/2 (max adjacent inversions), lower bound: 0
  → Termination proved
```

**Current coverage scope**:

| Algorithm      | Operation Pattern | Strategy 2 Proves? | Reason                                               |
| -------------- | ----------------- | :----------------: | ---------------------------------------------------- |
| Bubble sort    | Adjacent swap     |         ✅         | violation_count strictly decreases per swap          |
| Insertion sort | Adjacent shift    |         ✅         | Each shift eliminates one violation pair             |
| Selection sort | Non-adjacent swap |         ❌         | Single swap may increase violation_count             |
| Quicksort      | Partition         |         ❌         | Non-adjacent swap, no monotonic decrease             |
| Heapsort       | sift-down         |         ❌         | Tree-shaped operation, violation_count non-monotonic |

**Complementary strategy**: for quicksort, `low < high` interval shrinking can be covered by
Strategy 1 (linear rank function)—outer partition recursion, interval halves each time. Strategy 1
and Strategy 2 complement each other, covering most practical algorithms' termination. But
generalizing Strategy 2 (non-adjacent operations, tree-shaped operations) remains an open problem.

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
Strategy 1, which the compiler handles first as a fast path.

#### 6.5 Strategy 4: Multiplicative Scaling Measure Template

`v *= const` (const > 1), variable has upper and lower bound type annotation. The compiler has
built-in logarithmic measure template `ceil(log_const(upper/v))`, each multiply by const decreases
measure by 1.

```yaoxiang
mut i: Positive(i) = 1
while i < n {
    # Compiler automatically derives: measure ceil(log₂(n/i)), each multiply by 2 decreases measure by 1
    i *= 2
}
```

#### 6.6 Termination and Correctness Separation

Termination proof and correctness proof are independent:

- **Termination**: the above four strategies automatically prove the loop exits in finite steps
- **Correctness**: whether the loop body advances toward the target type, checked by the
  compile-time proof pipeline through verification conditions

Both pass → compilation passes. Termination proved but correctness fails → compile error +
counterexample. Correctness proved but termination unprovable → compile error pointing out the
variables or operations that cannot be analyzed. Both fail → compile error reporting both failure
reasons separately.

#### 6.7 Recursive Function Termination Checking

For recursive functions that need to be evaluated at compile time, the compiler checks argument
decrease:

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)  # Compiler analyzes: n-1 < n → decreases → terminates
}

# Compile-time use—compiler guarantees factorial terminates at compile time
arr: Array(Int, factorial(5)) = Array(Int, 120)()  # 5! = 120, done at compile time
```

| Scenario                                              | Behavior                    |
| ----------------------------------------------------- | --------------------------- |
| Compiler can analyze recursive decrease (e.g., `n-1`) | Compile-time evaluation     |
| No decrease / cannot determine decrease               | Compile error               |
| Runtime call (not in type position)                   | No termination check needed |

#### 6.8 Hard Boundary

`i = f(i)` where `f` is irreversible, not closed, and doesn't preserve any
monotonicity—mathematically impossible to automatically prove termination. Compile error:

> This loop cannot be automatically proven to terminate. The loop variable depends on the
> unanalyzable function `f`. Please use an iteration pattern analyzable by the compiler, or provide
> a measure function and termination proof function (see RFC-027a).

This is not a compiler failure. Whatever cannot be statically proven safe shall not pass
compilation.

### 8. SMT Solver: Type Checker's Acceleration Module

The SMT solver is an external tool in traditional languages (e.g., F\* calls Z3, Dafny calls Z3). In
YaoXiang, it is an **acceleration module of the type checker**—only called when the compiler kernel
itself cannot directly determine. SMT helps find proofs, but validating the proof is the type
checker.

**Trust model**: the type checker is the sole trust root. The SMT solver is an acceleration
module—it helps find proofs, but SMT is not an independent trust boundary. The compiler trusts Z3's
`unsat` result (consistent with the F\*/Dafny route—Z3's error rate is lower than the compiler's own
bug rate, an engineering pragmatic choice). The true unreliability control is in the SMT translation
layer—if the translation has bugs, the compiler will be exposed in other tests.

**Interface**: the compiler internally translates to SMT-LIB 2.6 standard format, rather than
binding to specific solver APIs. SMT-LIB is an ISO standard, natively supported by Z3, CVC5,
MathSAT, Yices.

**Default backend**: Z3 (MIT license, most extensive documentation and community validation). CVC5
as SMT-LIB compatible alternative—users can switch via compiler flags at compile time.

No "general solver abstraction layer"—SMT-LIB is the abstraction layer. If CVC5 makes breakthroughs
in specific theories in the future, switching only requires changing the binary, no compiler code
changes needed.

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
            → Send to Z3/CVC5 (with budget limits)
            → Return: unsat → Proved  │  sat + model → Disproved  │  unknown → Unproven
```

**Solver budget—hard limit, like stack depth**:

| Budget dimension               | Default | Description                                                                                                                                              |
| ------------------------------ | ------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Solving steps                  | 10,000  | Z3 typically within a hundred steps for linear arithmetic. 10,000 steps covers 99% of practical predicates.                                              |
| Time                           | 100ms   | Single predicate exceeding 100ms = user is writing compile-time program, not type annotation. 100ms × 50 predicates = 5 second compile time upper bound. |
| Quantifier instantiation depth | 3       | Three-level nested quantifiers cover practical patterns. Beyond three levels is likely a logic exercise.                                                 |

Over budget returns Unproven, compile error + predicate location + consumption. No degradation, no
runtime check, no silent pass.

**Why this is actually feasible**: 95% of practical predicates in engineering are linear
arithmetic—`x > 0`, `arr.len > 0`, `0 <= idx < arr.len`—all within the decidable fragment, SMT
solvers return millisecond-level for these problems. For the rare complex predicates that exceed
budget, the programmer writes a proof function.

Dependent types do a layer of pre-reduction before SMT calls: `factorial(5)` directly evaluates to
`120` at compile time, `append([1,2], [3])` directly evaluates to `[1,2,3]`. These deterministic
value calculations don't consume SMT budget.

Programmers don't need to know SMT exists. Mental model is: **the compiler can prove → pass, cannot
→ error—if the compiler doesn't, you can write a function to prove to it**.

### 9. Compile-time Predicate Composition

Compile-time predicates are functions that return Type, composition is naturally achieved through
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

### 11. Dispatch Pipeline: Unified Dispatch for Compile-time and Runtime

`assert` and `Assert` are two sides of the same refinement type primitive. The dispatch pipeline
`dispatch` automatically decides between compile-time proof and runtime check based on **whether the
predicate's free variables are accessible at compile time**:

| Criterion                                                                                  | Mode            | Behavior                                                                                 |
| ------------------------------------------------------------------------------------------ | --------------- | ---------------------------------------------------------------------------------------- |
| All free variables known at compile time (generic parameters, compile-time constants)      | **CompileTime** | Enter proof pipeline: Proved → erase, Disproved → compile error, Unknown → require proof |
| Some free variables come from runtime (function parameters, external input, mut variables) | **Runtime**     | Insert runtime check, inject refinement fact into flow-sensitive assumption set Γ        |

**Key**: "Cannot determine" ≠ "disproved". In CompileTime mode, Unknown requires proof (no silent
degradation); in Runtime mode, the proposition has no truth value at compile time at all—no matter
how strong the prover is, it cannot write a always-true proof for "user might input a negative
number", runtime check is the only sound choice. This is not the prover being too weak, it is
theoretical necessity.

### 12. Flow-sensitive Assumption Set Γ: Strongest Postcondition Propagation

The compiler maintains a flow-sensitive assumption set Γ, tracking propositions known to hold at
each control flow point.

**SP (Strongest Postcondition) propagation**:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
```

**Kill set for `mut` variables**: when a `mut` variable is reassigned, all assumptions involving
that variable are removed from Γ:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
mut x = x - 5       // Γ = {}  ← x > 0 is killed
```

This is a hard requirement of soundness—variable value changed, old assumptions invalid.

**Branch confluence**: when IF/ELSE or match branches merge, Γ takes the intersection of each
branch's assumptions. Only propositions that hold on all paths are carried out of the branch.

### 13. Erasure Model Clarification: Witness Erasure ≠ Check Erasure

RFC-027's assertion that "refinement types are **completely erased** at runtime" refers to **proof
witnesses**—proof terms already verified at compile time produce no runtime code. But runtime checks
inserted by Runtime mode dispatch **are retained**—they are Bool checks executed at the value level,
not witnesses at the type level.

Summary: witnesses are erased, checks are retained. The two things don't conflict, RFC-027's
original assertion is unchanged.

## Detailed Design

### Syntax Changes

| Before (RFC-022)                      | After (This RFC)                                                             |
| ------------------------------------- | ---------------------------------------------------------------------------- |
| `//! requires: NonEmpty(n) = n > 0`   | Compile-time predicate as parameter type `(b: Positive(b))`                  |
| `//! ensures: ExistsMax(result, arr)` | Return type uses return value parameter `-> (result: IsMax(T, arr, result))` |
| `/*! invariant: ... !*/`              | Compile-time predicate type annotation on variable—Floyd-Hoare invariant     |
| `//! decreases: n`                    | Compiler fully automatic measure function derivation                         |
| Specification is annotation           | Specification is type system                                                 |

### Syntax

**Compile-time predicates have no new keywords.** `{}` is the proof space, exactly consistent with
existing type definition syntax. A compile-time predicate is a function that returns
Type—`name: (params) -> Type = { assertions }`. Usage is a function call—`Positive(b)`,
`IsMax(T, arr, result)`.

```bnf
# Compile-time predicate = function returning Type, {} contains compiler-validated assertions
# Uses existing function/type syntax, no new BNF rules needed
predicate ::= identifier ':' params '->' 'Type' '=' '{' assertions '}'
```

**Predicate application arguments must be compile-time constant forms**—literals, variables (bound
by name), or single-argument type applications (recursive extraction). Arguments that cannot be
converted to constant expressions report **E1092**, argument count mismatching predicate parameters
report **E1093**—refinement constraints are **never silently discarded** (previously, inconvertible
arguments caused constraints to silently disappear, and bindings violating constraints silently
passed).

**New syntax concept: return value parameter**—`name` in `-> (name: Type)` is a return value
parameter.

The return value parameter is the **only syntax concept** YaoXiang introduces on existing function
syntax. Its semantics:

- `name`'s value is provided by the `return` statement
- `name` only exists in the type signature, referenced by postcondition predicates (e.g.,
  `-> (result: IsMax(T, arr, result))`)
- `name` does not enter the function body scope, does not appear at the caller
- Return value parameter **is optional**—when no postcondition, signature is identical to ordinary
  function (`-> Int`), introducing no extra burden

Reason for introducing it: postconditions need to reference "the value the function will return".
Without return value parameters, the compiler can only use special rules (such as implicit variable
`$result` or `__retval__`) to let the predicate reference the return value. The return value
parameter makes this reference explicit—it is just a parameter, just that the value is provided by
`return` rather than the caller.

**Proof function** is not a new concept—it is a YaoXiang function whose return type is the asserted
proposition. When the compiler returns Unproven, the programmer provides a proof function, and the
type checker validates it the same way it validates any function's return type. No new syntax, no
new keywords, no new rules needed.

### Type System Impact

- **Type universe**: compile-time predicates are at the Type₂ layer—functions taking values and
  returning Type, same layer as type constructors
- **Generic interaction**: compile-time predicates can take generic parameters, e.g.,
  `NonEmpty: (T: Type) -> (arr: Array(T)) -> Type`
- **Ownership interaction**: expressions in compile-time predicates obey ownership rules, only
  readable, not writable
- **Type inference**: compile-time predicate arguments participate in HM type inference

### Runtime Representation

Compile-time predicates are handled at runtime **according to dispatch pipeline results**:

- **CompileTime mode** (all free variables known at compile time): after proof passes, the witness
  is completely erased. `Positive: (x: Int) -> Type = { x > 0 }`—the parameter `b: Positive(5)`
  runtime representation is just `Int`. The refinement condition `{ 5 > 0 }` has passed, erased.
- **Runtime mode** (runtime free variables exist): retain runtime check—execute Bool check at the
  value level, inject into flow-sensitive assumption set Γ. See §11 dispatch pipeline and §13
  erasure model clarification for details.

Placing compile-time predicates in type position (e.g., `f(x: Positive(x))`) produces no wrapper
type, no extra memory allocation. But when `x` comes from runtime input, **it will** insert a
runtime Bool check.

**Interaction constraint with `ref`**: compile-time predicates can only reference immutable borrows
or values with transferred ownership. Compile-time predicates referencing mutable borrows, the
compiler cannot guarantee at compile time that the validation result still holds at runtime—such
usage directly reports a compile error.

### Compiler Changes

1. **Parser**: compile-time predicates use standard function syntax, no extra parsing rules needed
2. **Compile-time proof pipeline**: unified Proved/Disproved/Unproven return interface, automatic
   strategy selection
3. **SMT acceleration module**: SMT-LIB 2.6 translation layer, default backend Z3, CVC5 alternative
4. **Type checker kernel**: inference rule implementation—structural equivalence, βδι-reduction,
   universal quantifier introduction/elimination. This is the sole trust root, SMT and programmer
   proofs are both validated through this
5. **Verification condition generation**: WP/SP calculus + loop invariant proof obligations
6. **Error reporting**: counterexample formatting + unsolved proposition report + source location
   association

### Backward Compatibility

- ✅ Code not using compile-time predicates is completely unchanged
- ✅ Compile-time predicates have zero runtime overhead in CompileTime mode, Runtime mode only
  retains necessary Bool checks
- ⚠️ RFC-022's `//!` syntax is no longer supported—but 022 was never implemented, no migration
  burden

## Trade-offs

### Advantages

- **Full realization of Curry-Howard isomorphism**: types are propositions, programs are proofs,
  `name: Proposition = Proof`
- **Unity**: compile-time predicates and ordinary functions use exactly the same syntax, no
  conceptual split
- **SMT transparency**: programmers don't need to know SMT exists, mental model consistent with type
  checking
- **Progressive adoption**: can start with one compile-time predicate, gradually increase coverage
- **Minimal runtime overhead**: zero overhead in CompileTime mode, Runtime mode only retains
  necessary Bool checks

### Disadvantages

- **Compile time**: SMT solving increases compile time, but hard budget limits keep the upper bound
  controllable
- **Automatic proof boundary**: complex predicates beyond first-order linear arithmetic may require
  programmers to write proof functions. This is not a language defect—this is the inevitable
  conclusion of the halting problem. The compiler honestly reports Unproven rather than falsely
  claiming True/False
- **Learning curve**: writing effective compile-time predicates and proof functions requires
  understanding the basic intuition of Curry-Howard isomorphism
- **Implementation complexity**: unifying the compile-time proof pipeline requires careful design

### Risk Mitigation

- SMT solver budget hard limits (10,000 steps / 100ms / instantiation depth 3), over budget returns
  Unproven
- Dependent type pre-reduction: deterministic value calculations are consumed first, SMT only chews
  on the non-deterministic part
- Unproven is not a dead end: programmers can write proof functions, type checker
  validates—consistent with validating any function return type
- Incremental validation: only validate changed modules
- Clear error messages + counterexample display + budget consumption report + unsolved proposition +
  suggestions (if compiler can give them)

## Alternatives

| Alternative                                                    | Why Not                                                                                                                                                             |
| -------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| RFC-022: `//!` annotation-style specifications                 | Specifications and types are split, violating Curry-Howard isomorphism                                                                                              |
| Independent specification files (e.g., CVL)                    | Specifications and code separated, increasing maintenance cost                                                                                                      |
| Runtime assertions only                                        | Cannot statically guarantee correctness                                                                                                                             |
| External proof assistant (e.g., Coq)                           | Disconnected from compiler, requiring independent proof language and trust boundary. YaoXiang's choice: proof is YaoXiang code, type checker is the sole trust root |
| **This plan: compile-time predicates as first-class citizens** | ✅                                                                                                                                                                  |

## Implementation Strategy

### Phase Division

| Phase       | Content                                                                                                                                                                 |
| ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Phase 1** | Compiler kernel: structural equivalence + βδι-reduction + universal quantifier introduction/elimination. Supports simple arithmetic predicates (`x > 0`, `arr.len > 0`) |
| **Phase 2** | SMT-LIB translation layer + Z3/CVC5 integration. Pipeline returns Proved/Disproved/Unproven. Unproven supports programmer writing proof functions                       |
| **Phase 3** | Loop invariant VC generation + termination checking (linear rank function + predicate violation counting + bounded pattern + combinatorial explosion control)           |
| **Phase 4** | Incremental validation + caching + IDE support                                                                                                                          |

### Dependencies

- RFC-010: Unified Type Syntax — compile-time predicates based on `name: type = value`
- RFC-011: Generic Type System — compile-time predicates can take generic parameters
- RFC-009: Ownership Model — expressions in compile-time predicates obey ownership rules

## Open Questions

- [x] **SMT solver choice**: default Z3 (MIT license, most extensively validated). CVC5 as SMT-LIB
      compatible alternative, switched via compiler flag. Compiler internal translation target is
      SMT-LIB 2.6 standard format—SMT-LIB is the abstraction layer, no custom general solver
      interface.
- [x] **Specific solver budget values**: 10,000 steps / 100ms / quantifier instantiation depth 3.
      Fixed inside compiler, no knob. If actual use proves insufficient (not "user wrote wrong"),
      adjust then.
- [x] **Quantifier support range**: language level doesn't limit quantifier order. Compile-time
      predicates accept Type parameters—Type includes function types—therefore higher-order
      quantifiers are natural inference of the type system, no special syntax needed. SMT solver can
      automatically determine first-order quantifiers (forall/exists, supports interleaved nesting,
      limited by budget depth 3). Higher-order quantifiers: SMT returns Unproven, compiler hints
      "this predicate exceeds automatic proof range, please provide proof function". Programmer
      writes a YaoXiang function whose return type equals the proposition—type checker validates
      that function. No external export, no AI, no interactive proof mode needed. Everything is
      YaoXiang code, everything is validated by the type checker.
- [x] **Counterexample formatting**: source variable names directly used as SMT variable names (with
      module prefix to avoid conflicts). Z3 model returns query by variable name. Output format:
      variable name = specific value + source location + predicate definition location. No complex
      mapping layer.
- [x] ~~**Interaction of compile-time predicates with `ref` smart pointers?**~~ → Decided:
      compile-time predicates only allow immutable borrows or values with transferred ownership.
      Values with mutable borrows cannot appear in compile-time predicates.
- [x] **Extension of `forall` predicate violation count measure to non-adjacent operations?** → Not
      extended. Current coverage scope (adjacent swap, adjacent shift) is complemented by Strategy 1
      (linear rank function)—quicksort outer interval shrinking backed by Strategy 1, heapsort
      backed by Strategy 1 (array index pattern). Loops where no strategy can prove termination, the
      compiler directly errors—this is hard safety philosophy, not a defect. If future has real
      scenarios (not academic constructions) where algorithms cannot be covered by any of the four
      strategies, discuss again. → **Re-discussion triggered (2026-09-14, #318)**: non-structural
      recursion (gcd-style non-direct decrease, mutual recursion, merge partition) is the real
      scenario anticipated by this clause—current pipeline cannot automatically prove their
      termination, and they cannot be rewritten into analyzable iteration patterns without breaking
      readability. Conclusion: full-automation priority unchanged; after Unproven add proof function
      fallback—measure and termination proof are ordinary YaoXiang functions, zero new syntax, type
      checker validates. Landed in
      [RFC-027a: Termination Proof Function Fallback](../draft/027a-termination-proof-fallback.md).
- [x] **Linear rank function enumeration combinatorial explosion**: candidate enumeration upper
      limit is 3 bounded variables. ≤3 enumerate all linear combinations and SMT validates each. >3
      only try single-variable measures (`v_i`, `u_i - v_i`), failure directly reports compile
      error—hints programmer "loop has >3 bounded variables, compiler cannot automatically
      synthesize multi-variable measure". This is not an engineering compromise—it forces
      programmers to write simpler loops.

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
│ (Formal     │    │ (Kept in    │
│  design)    │    │  place)     │
└─────────────┘    └─────────────┘
```
