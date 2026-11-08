---
title: 'RFC-027: Compile-Time Predicates and Unified Static Verification'
status: 'Accepted'
author: 'Chen Xu'
created: '2026-06-07'
updated: '2026-10-03'
impl_status: 'in-progress'
impl_detail:
  'Phases 1-2 complete, Phase 3 partially complete, Phase 4 partially complete. The unified
  assert/Assert approach has been fully implemented across all 6 phases (#157-#162 closed): Never
  type, IsTrue bridge, flow-sensitive Γ + kill set, type-level recursion, weak universe
  stratification check. The **semantics** of dispatch routing in §11 have been implemented through
  call-site obligations (`checker.rs` directly calls `check_predicate` in three places: binding site
  / call-site arguments / return site), but the **standalone `layers/dispatch.rs` module has been
  removed**—it had zero production call sites, and its handling of `Unproven` (downgrading it to a
  W1080 warning) conflicted with the production implementation (always reporting an error) (#377-2).'
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
> [RFC-022: Hoare-Logic Static Verification Support (Specification Comments and Specification Types)](../deprecated/022-hoare-logic-static-verification.md)
> — Deprecated

## Abstract

This RFC proposes introducing **compile-time predicates** as first-class citizens of YaoXiang and
unifying all compile-time static verification into a single **proof pipeline**. A compile-time
predicate is not an add-on specification comment—it is a function. A function that returns Type can
be used in type position; the compiler calls it at compile time and checks its return value. Types
are propositions, and compile-time evaluation is proof.

**Core thesis**: the sole job of type checking at compile time is to construct and validate proof
terms. Type equalities, token conflicts, dependent-type reduction, compile-time predicate
evaluation, Hoare-logic implication—all are different type checks within the same compile-time proof
pipeline and share the same pipeline. The SMT solver is an accelerator module for the type checker,
not an independent trust boundary. When the compiler returns Unproven, the programmer writes a
YaoXiang function as the proof—the type checker validates it in exactly the same way it validates
the return type of any function. Everything is YaoXiang code, and everything is validated by the
type checker.

## Motivation

### Why deprecate RFC-022?

RFC-022 designed specifications in the form of `//!` comments:

```yaoxiang
max: (T: Ord) -> ((arr: Array(T, n)) -> T) = {
    //! requires: NonEmpty(n) = n > 0          ← This is a comment independent of the type
    //! ensures: ExistsMax(result, arr[0..n])   ← This is a comment independent of the type
}
```

This makes a fundamental mistake in the Curry–Howard correspondence: **it splits specifications and
types into two layers**. Comments are not types. Comments do not participate in type checking.
Comments embody the mental model of an “external tool.”

The whitepaper states this clearly:

> “There are no `//!` comments. There is no separate specification language. Everything is within
> the type system.”

### Current problems

- RFC-022's `//!` comments are add-on syntax independent of the type system
- Specification types and ordinary types constitute two separate systems, creating conceptual
  redundancy
- The split between verification in Debug builds and ignoring it in Release builds undermines
  uniformity
- In traditional understanding, the SMT solver is positioned as an external tool—YaoXiang instead
  incorporates it as an accelerator module for the type checker
- Type checking, borrow checking, compile-time predicate checking, and macro expansion each follow
  different paths

### The correct mental model

Type checking can be abstracted as a function:

```
verify : Program → Proved | Disproved(Model) | Unproven
```

All compile-time checks—simple type matching, borrow-conflict detection, and compile-time predicate
validation—are subtasks of this function. They share the same proof pipeline and differ only in
proof-term complexity and construction strategy.

When the compiler returns Unproven, the programmer supplies a proof function—whose return type
equals the proposition to be proved. The type checker validates it. This is the same operation as
ordinary type checking.

## Proposal

### 1. `{}` is the proof space: types are assertions, validation is type checking

YaoXiang's `{}` is the compile-time proof space. Everything inside it is an assertion, and the
compiler guarantees that every item is True—either by proving it automatically or by accepting a
programmer-supplied proof function.

```
Point: Type = { x: Float, y: Float }
#               ^^^^^^^^^^^^^^^^^^^^^  The compiler guarantees that x is Float and y is Float

List: (T: Type) -> Type = { data: Array(T) }
#                           ^^^^^^^^^^^^^^^  The compiler guarantees that data is Array(T)
```

**Generics are a special case of compile-time predicates.**

```yaoxiang
Positive: (x: Int) -> Type = { x > 0 }
#          ^^^^^^              ^^^^^^
#          Parameter in the signature       Only assertions inside {}
#          The compiler validates x > 0 when invoking it at compile time

List: (T: Type) -> Type = { data: Array(T) }
#      ^^^^^^^^              ^^^^^^^^^^^^^^^
#      Parameter in the signature       The compiler validates type_of(T) == Type and type_of(data) == Array(T)
```

The pattern is the same in each case: `name: (params) -> Type = { assertions }`. The compiler makes
no distinction between “type assertions” and “value assertions”—both are evaluation targets in the
proof pipeline.

**Loop invariants do not need to be written separately. Type annotations on variables are
Floyd–Hoare invariants.**

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # The annotation references i—it tells the compiler that s's type depends on i
    mut i: UpTo(arr.len) = 0     # At initialization i=0, validate: 0 == sum(arr[0..0]) → True
    while i < arr.len {
        s += arr[i]  # The compiler validates: s_new == sum(arr[0..i+1])
        i += 1       # i changes → triggers revalidation of s's dependency: s satisfies SumUpTo(arr, i_new)
    }
    return s  # s: SumUpTo(arr, arr.len) = sum(arr[0..arr.len])
}
```

The compiler generates one verification condition for the loop body—the induction hypothesis (type
annotation) → assignment operation → whether the new value satisfies the type annotation. Once the
proof pipeline establishes the inductive step, all iterations are covered automatically. There is no
need for `: decreases`, `: Invariant`, or a separate inductive proof—the compiler decomposes
induction into a local VC for each assignment.

### 2. Preconditions/postconditions: compile-time predicates in parameter and return types

Abandon RFC-022's `//! requires`/`//! ensures`. Compile-time predicates are used as type annotations
on parameters or return values.

**On the parameter side, this is a function call.** A compile-time predicate is a function that
returns Type, so using it on the parameter side is an invocation—just like `factorial(5)`. The
return-value side introduces a new concept: the return-value formal parameter.

```yaoxiang
# Precondition: explicitly invoke a compile-time predicate in the parameter type
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b
#                       ^^^^^^^^^^  b is the current formal parameter name and is passed to Positive as an argument
#                       The compiler extracts the argument value at the call site, substitutes it for b, and validates Positive(actual argument)
#                       Example: divide(10, 2) → validates Positive(2) = { 2 > 0 } → True
#                       Example: divide(10, 0) → validates Positive(0) = { 0 > 0 } → False → compile error

# Postcondition: return-value formal parameter + compile-time predicate
IsMax: (T: Ord, arr: Array(T), result: T) -> Type = {
    forall j in 0..arr.len: result >= arr[j]
}

NonEmpty: (arr: Array(T)) -> Type = { arr.len > 0 }

max: (T: Ord) -> ((arr: NonEmpty(arr))) -> (result: IsMax(T, arr, result)) = {
#                                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
#                                            result is the return-value formal parameter, and its value is supplied by return
#                                            The compiler substitutes the returned value at the return site and validates the postcondition
    candidate = arr[0]
    for i in 1..arr.len {
        if arr[i] > candidate { candidate = arr[i] }
    }
    return candidate
}
```

**Key rules**:

- **Parameter side**: `b: Positive(b)`—`b` is the current formal parameter name and is passed to
  `Positive` as an argument. This uses function-call syntax, with no implicitness.
- **Return side**: `-> (result: IsMax(T, arr, result))`—`result` is the return-value formal
  parameter, and its value is supplied by the `return` statement. `result` exists only in the type
  signature and is only referenced by predicates; it does not enter the function body's scope and
  does not appear at the call site.
- **The return-value formal parameter is optional**: when there is no postcondition, it is omitted
  and the signature is identical to an ordinary function (`-> Int`).
- **Uniformity**: parameters and return-value formal parameters are the same
  concept—`formal-name: predicate-call(actual argument)`—and differ only in whether the value is
  supplied by the caller or by `return`.

> **Implementation note (2026-10-02)**: recognition of return-position formal parameters follows
> **the declared name strictly**—the `r` in `-> (r: P(...))` is the sole return-value formal
> parameter. Other free variables in the constraint that are **not in scope** are no longer
> substituted with the return value; instead, an undefined-identifier error is reported (observed:
> `g: () -> (r: Eq2(m, r)) = { return 7 }` (`m` is undefined) →
> `error [E1001] Unknown variable: 'm'`). Predicate-call arguments are substituted into the
> constraint **according to the actual argument**: `P(r + 100)` becomes “return value + 100,” not
> merely “return value.”
>
> **Known gap (2026-10-02)**: **multi-parameter predicates at the return position + symbolic
> arguments are not yet supported**. Triggering form:
> `f: (b: Int) -> (r: SumUpTo(b, r)) = { return b * 2 }`
> (`SumUpTo: (n: Int, s: Int) -> Type = { s == n * 2 }`) →
> `error [E2031] … the refinement type constraint of 'r': SumUpTo(b, (b * 2)) cannot be statically proven`.
> Workaround: use a compile-time-foldable literal for the argument (for example, `SumUpTo(3, r)`),
> or provide a proof function returning `Type` for that predicate. Resolution condition: support
> argument substitution for “multi-parameter predicates + symbolic arguments” in the proof kernel.
> See [RFC-027a](../accepted/027a-termination-explicit-measure.md) §Implementation Notes
> (2026-10-02, this hardening round).

### 3. Path-condition propagation: compile-time validation of runtime values

When a compile-time predicate is used at a binding site, its arguments are explicitly supplied by
the programmer. When a runtime value enters a refinement-type parameter, the compiler completes
validation through path-condition collection and SMT implication checking—there is no need for the
programmer to pass proof explicitly.

#### 3.1 Explicit function invocation

When a compile-time predicate is used at a binding site, the arguments are explicitly supplied by
the programmer—this is simply a function call, with no implicitness.

`Positive: (x: Int) -> Type = { x > 0 }` is a compile-time predicate constructor. When it appears at
a binding site (parameter declaration, variable declaration, or return type), the programmer
explicitly supplies the name of an already-bound variable:

```yaoxiang
b: Positive(b)
// b has already been declared as the current formal parameter; Positive(b) is a function call
// After normalization: b: { b > 0 }
```

The compiler does not need to fill in arguments implicitly—`b: Positive(b)` is a function call, just
like `f(5)`. `b` is bound as a parameter name, and its type annotation `Positive(b)` refers to `b`
itself. This is the standard dependent-type pattern, not an implicit expansion rule.

**Unification with RFC-010's `self`**: RFC-010 established that `self` is not a keyword but merely a
naming convention for parameters (“writing it as `p`, `this`, or `x` has exactly the same effect”).
`b: Positive(b)` shares the same mechanism—a parameter name may be referenced in a type annotation.
`self` appears in `self: Point`, while `b` appears in `b: Positive(b)`; both type annotations refer
to the parameter itself. The only difference is the complexity of the annotation; the mechanism is
exactly the same—once a name is bound, the type may depend on that name.

Return types use explicit function invocation in the same way:

```yaoxiang
Sorted: (arr: Array(T)) -> Type = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }

sort: (arr: Array(T)) -> (result: Sorted(result)) = { ... }
//                        ^^^^^^^^^^^^^^^^^^^^^^^
//                        result is the return-value formal parameter; Sorted(result) is a function call
//                        The compiler substitutes the returned value for result at the return site and validates Sorted(return value)
```

The same applies to local-variable declarations:

```yaoxiang
let x: Positive(x) = 5
// x is bound to 5; Positive(5) → { 5 > 0 } → True → passes

// let y: Positive(y) = 0
// y is bound to 0; Positive(0) → { 0 > 0 } → False → compile error
```

#### 3.2 Path-condition collection

When a runtime value occurs in a conditional branch, the compiler automatically collects path
conditions and forms the current scope's **assumption set**. These assumptions participate in
validation as background knowledge for compile-time Bool evaluation.

```yaoxiang
if y > 0 {
    // The compiler automatically obtains the assumption in this branch: { y > 0 }
    let result = divide(x, y)
    // Verification condition: (y > 0) ⇒ (y > 0)
    // The proof pipeline determines that the implication holds → Proved
} else {
// Assumptions in this branch: { !(y > 0) }
// To call divide(x, y), the verification condition is !(y > 0) ⇒ y > 0
    // The proof pipeline determines that it does not hold → Disproved
}
```

This is not a hardcoded special case in the compiler—it is the natural behavior of the compile-time
proof pipeline. Every type-checking call site sends the following to the pipeline:

```
{background assumptions} ⇒ {verification target}
```

The proof pipeline determines entailment. Proved → passes, Disproved → compile error +
counterexample, Unproven → compile error + unresolved proposition. Background assumptions come from
the path conditions at the current program point.

#### 3.3 Assumption stack

While analyzing control flow, the compiler maintains an assumption set for each basic block:

- **if-guard**: `if y > 0` → the true branch pushes `y > 0`; the false branch pushes `!(y > 0)` (if
  an `else` is used)
- **match pattern**: `if let Some(v) = opt` → the branch pushes `opt == Some(v)`
- **logical conjunction**: `if x > 0 and y < 10` → the branch pushes `x > 0` and `y < 10`
- **function precondition**: when calling `divide(a, b)`, the evidence that `b` satisfies `Positive`
  must come either from the current assumptions or from the actual argument's refinement-type
  annotation (if `b` is already annotated as `Positive`, its type carries `b > 0`)
- **assignment**: when `let z = y` is executed, the refinement conditions already attached to `y`
  are transferred to `z`

All assumptions enter the compile-time proof pipeline. When execution enters the SMT-accelerated
path, they are translated into SMT-LIB background assertions.

#### 3.4 Compile error when no static evidence exists

If the programmer directly writes:

```yaoxiang
divide_user_input: (x: Int, y: Int) -> Int = divide(x, y)
```

there is no `y > 0` assumption at the current program point, and the actual argument `y` has no
`Positive` type annotation of its own. The verification condition is:

```
{} ⇒ { y > 0 }
```

The pipeline returns `Disproved` (the implication does not hold) → compile error:

> Unable to prove that parameter `b` satisfies `Positive` in the call to `divide`. `y` comes from
> function input and has no proven bound. Consider guarding the call with an if branch:
> `if y > 0 { divide(x, y) }`.

YaoXiang does not permit runtime values to enter refinement-type parameters without static evidence.
This is not a limitation—it is the core of the hard-safety philosophy. Any code the compiler cannot
prove statically must not pass compilation.

#### 3.5 Relationship to the unified pipeline

Path-condition propagation is not an additional mechanism. It is the direct extension of the
compile-time proof pipeline into control-flow analysis:

| Stage                             | Responsibility                                                                                                                                                    |
| --------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Path-condition collection         | During compiler control-flow analysis, annotate each basic block with an assumption set                                                                           |
| Verification-condition generation | When a type constraint requires validation, combine path conditions with actual-argument type information                                                         |
| Proof-pipeline evaluation         | Compiler kernel → SMT acceleration → derive Proved / Disproved / Unproven                                                                                         |
| Result                            | `Proved` → passes; `Disproved` → compile error + counterexample; `Unproven` → compile error + unresolved proposition (the programmer may supply a proof function) |

There are no new components and no special rules. Path conditions are the background knowledge of
the proof pipeline, sharing the same pipeline and budget system as type equalities and borrow
constraints.

### 4. The compile-time proof pipeline

All compile-time checks share one pipeline. The pipeline's core operation is **type
checking**—checking whether the type of a proof term equals the proposition to be proved. Everything
is type checking.

```
编译期遇到 Bool 表达式需要求值（即：需要构造一个证明项）
        │
        ├── 类型等式（T1 == T2）
        │   → 编译器直接判定（结构等价）
        │
        ├── 令牌冲突条件（!conflicting(tokens)）
        │   → 流敏感活性分析（Dup/Linear 属性追踪）
        │
        ├── 依赖类型归约（n + m 化简）
        │   → 编译期项重写系统（βδι-规约）
        │
        ├── 编译期谓词（x > 0, forall...）
        │   → 编译器自身 + SMT 加速模块
        │
        └── 霍尔逻辑蕴含式（P ⇒ Q）
            → 编译器 + SMT 加速模块
                    │
                    ▼
             ┌──────────┐
             │ Proved   │  → 编译通过
             │ Disproved│  → 编译错误 + 反例
             │ Unproven │  → 编译错误 + 未解命题
             └────┬─────┘
                  │
                  ▼
         程序员写证明函数（YaoXiang 代码）
                  │
                  ▼
         类型检查器验证 ──→ Proved ──→ 编译通过
                  │
                  ▼
            验证失败 → 编译错误："证明不成立"
```

#### 4.1 Proof outcomes: a three-valued algebra

Compile-time evaluation returns three outcomes—an inevitable consequence of the halting problem and
a natural partition of proof theory:

```
eval_compile_time : BoolExpr → Proved | Disproved(Model) | Unproven
```

- **Proved** → halts; the proof term has been constructed and passes type checking. Compilation
  continues.
- **Disproved(M)** → halts; a counterexample M exists. Compile error + counterexample + source
  location.
- **Unproven** → no proof was constructed within the given resource limit. Compile error +
  unresolved proposition + budget-consumption report.

**Unproven ≠ False.** The compiler saying “I cannot prove it” is not equivalent to the proposition
being false—it merely exceeds the current automatic proof capability. This is honesty, not a defect.

A hard budget limit is the engineering answer to the halting problem. There is no knob—offering one
would amount to asking the user, “Do you think your program will halt?” The user does not know, and
neither does the compiler.

#### 4.2 After Unproven: the programmer writes the proof

When the compiler returns Unproven, the programmer can write a **proof function**—a YaoXiang
function whose return type equals the proposition to be proved. The type checker validates this
function in exactly the same way it validates `add(a, b): Int`.

```
命题  = 类型
证明  = 程序（该类型的一个值）
验证  = 类型检查（唯一的信任根）
```

The SMT solver is not an independent trust boundary—it is an **accelerator module for the type
checker**. SMT helps find proofs, but the type checker always validates the proof. When SMT returns
`unsat`, the compiler reconstructs the result as a proof term that the type checker can validate. If
reconstruction fails (the SMT inference steps exceed the compiler kernel's inference rules), it
falls back to Unproven, and the programmer can manually write a proof function.

```yaoxiang
# Proposition: a refinement property that the compiler cannot prove automatically
FirstIsMin: (T: Ord, arr: Sorted(T)) -> Type = {
    forall i in 0..arr.len: arr[0] <= arr[i]
}

# Proof: the programmer writes a function whose return type is the proposition above
# The type checker validates this function—exactly as it validates add(a,b): Int
first_is_min: (T: Ord, arr: Sorted(T)) -> FirstIsMin(T, arr) = {
    # The compiler validates here: the function body's type = FirstIsMin(T, arr)
    ...
}
```

No AI, no export to Coq, no new concepts. **A property the compiler cannot prove automatically → the
programmer writes the proof in YaoXiang code → the type checker validates it.** The entire process
is a smooth gradient—the compiler handles the easy proofs for you, leaving the difficult ones to
you.

#### 4.3 Layered dependencies within the pipeline

The evaluators above share the same interface but have an evaluation order. Type equality is a
prerequisite for all subsequent analyses; ownership/token checking depends on type information; and
refinement-predicate validation depends on the results of the preceding two layers. The compiler
evaluates by layer, and expressions that fail at a lower layer do not enter higher layers, avoiding
wasted solver budget on programs with type errors.

```
求值顺序（同一管道，分层调度）
├── 第 0 层：类型等式（T1 == T2）
│   └── 结构合一 → 失败则后续无意义，直接返回 Disproved
├── 第 1 层：所有权/令牌冲突
│   └── 流敏感活性分析 → 失败则内存安全不成立，直接返回 Disproved
└── 第 2 层：精化谓词/霍尔蕴含
    └── 编译器自身 → SMT 加速 → 得出 Proved / Disproved / Unproven
```

Every layer still returns `Proved/Disproved/Unproven`, sharing the same interface and the same
budget system.

### 5. Unification of the three function layers

| Layer                  | Execution time | Input      | Output | Example                                        |
| ---------------------- | -------------- | ---------- | ------ | ---------------------------------------------- |
| Value-level function   | Runtime        | Value      | Value  | `add: (a: Int, b: Int) -> Int = a + b`         |
| Type constructor       | Compile time   | Type/value | Type   | `List: (T: Type) -> Type = { data: Array(T) }` |
| Compile-time predicate | Compile time   | Value      | Type   | `Positive: (x: Int) -> Type = { x > 0 }`       |

All use the same `name: type = value` syntax. Compile-time predicates and type constructors follow
the same compile-time proof pipeline—`{}` is the proof space.

### 6. Loops: Floyd–Hoare verification-condition generation

Loops do not need separate `: Invariant(...)` or `: decreases(...)` annotations. Compile-time
predicate type annotations on variables define Floyd–Hoare-style assertions—the compiler generates
verification conditions from the type annotations, and the proof pipeline checks whether each
assignment preserves the type.

The core mechanism is that every assignment corresponds to a Hoare triple `{P} x := e {Q}`, whose
verification condition is `P ⇒ Q[e/x]`. The compiler generates one verification condition for the
loop body—once the proof pipeline establishes the inductive step, all iterations are covered
automatically.

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # The annotation references i; at initialization i=0, validate: 0 == sum(arr[0..0]) → True
    mut i: UpTo(arr.len) = 0     # Validate: 0 <= 0 <= arr.len → True
    while i < arr.len {
        # The compiler generates one VC for the loop body. Premises: s satisfies SumUpTo(arr, i), and i satisfies UpTo(arr.len).
        #
        # s += arr[i]:
        #   Verification obligation: s_new satisfies SumUpTo(arr, i) (i is unchanged)
        #   Substitute s_new = s_old + arr[i]:
        #     Requires s_old + arr[i] == sum(arr[0..i+1])
        #     From the induction hypothesis s_old == sum(arr[0..i]), add arr[i] to both sides:
        #     sum(arr[0..i]) + arr[i] == sum(arr[0..i+1])
        #   Compiler + SMT: linear arithmetic, milliseconds → Proved
        #
        # i += 1:
        #   i changes → s's type annotation in the dependency graph references i → triggers re-verification
        #   New verification target: s satisfies SumUpTo(arr, i_new)
        #   That is, s == sum(arr[0..i_new]), guaranteed by the previous step → Proved
        s += arr[i]
        i += 1
    }
    return s  # At this point s: SumUpTo(arr, arr.len), that is, s == sum(arr[0..arr.len])
}
```

A loop invariant is the type annotation on a variable—the programmer writes the type, and the
compiler checks the inductive step. The compiler does not need to “discover” invariants or “perform
induction automatically”; it decomposes the inductive proof into local verification conditions for
each assignment operation and hands them to the proof pipeline to solve divide and conquer.

#### 6.1 Dependency tracking: dependent types on mutable variables

The mechanism above requires the compiler to know that `s`'s type annotation `SumUpTo(arr, i)`
references `i`: when `i` changes, `s`'s type constraint changes as well. This requires the compiler
to maintain a **type-dependency graph between variables**.

**Data structure**:

```
TypeDepGraph: Map<VarName, Set<VarName>>
# The key is the depended-on variable, and the value is the set of variables whose type annotation references it
# Example: { i: {s}, j: {s, t}, ... }
```

**Construction**: when the type checker processes `mut v: Pred(... x ...) = init`, it resolves
free-variable references in the arguments of `Pred(...)`. If those arguments reference another
mutable variable `x` in the current scope, the dependency graph records `x → v`.

**Trigger**: when the depended-on variable `x` is assigned, the compiler:

1. Looks up all variables `{v₁, v₂, ...}` in the dependency graph that depend on `x`
2. Generates a verification condition for each `v`: does `v`'s current value satisfy the updated
   type `Pred(... x_new ...)`?
3. Sends the VC into the proof pipeline

**Assignment-order sensitivity**: dependency tracking naturally enforces the correct assignment
order. Taking `SumUpTo(arr, i)` as an example:

```yaoxiang
# Correct order
s += arr[i]   # s_new satisfies SumUpTo(arr, i+1)
i += 1        # i changes → revalidate that s satisfies SumUpTo(arr, i_new) → True

# Wrong order—the compiler rejects it
i += 1        # i changes → revalidate that s satisfies SumUpTo(arr, i_new)
              # s has not yet been updated; s_old == sum(arr[0..i_old]) ≠ sum(arr[0..i_new])
              # → Compile error: variable s does not satisfy type SumUpTo(arr, i_new)
s += arr[i]   # Unreachable
```

**Combined dependencies**: one variable may depend on multiple variables. A type annotation such as
`{ v: Int; v == x + y }` depends on both `x` and `y`—a change to either triggers revalidation.

**Relationship to the proof pipeline**: dependency tracking is the trigger for VC generation, not an
independent validation mechanism. It answers “when must a VC be generated?”; the proof pipeline
answers “does the VC hold?”

### 7. Termination checking

**Applicability: refinement types.** Termination is not an independent switch; it is part of the
**validation mode**. Once a type is refined (`Refined { base, constraint }`), the computation
segment denoted by that annotation enters validation mode, and termination is required within that
mode. Ordinary types that are not refined do not enter validation mode and generate no termination
obligations.

This yields two implications:

- **Loops**: a bare `while` does not enter validation mode; when the measure variable has a
  refinement annotation (for example, `i: UpTo(n)`), the loop enters validation mode and termination
  must be proved.
- **Recursion**: when a function signature contains a refinement (a refined parameter or a return
  type containing a refinement), it enters validation mode, and the measure must strictly decrease
  at every recursive call site.

Full automation is preferred within the mode: the compiler first explores measures automatically and
accepts any it can prove; if exploration fails and no explicit measure was provided, it reports a
compile error. There is **no annotation-syntax escape hatch**—the measure and termination
proposition both live in type position, introducing no new syntax such as `decreases`. See §6.9 for
the forms of measures and explicit fallbacks.

#### 6.1 Design principles

The compiler automatically extracts the information needed for a termination proof from two places:

1. **Variable type annotations**: bounds in refinement types (for example, `UpTo(n)` provides the
   upper bound `n` and lower bound `0`)
2. **Loop-body operations**: the operations applied to variables on each iteration

The compiler tries four measure-synthesis strategies in priority order and stops as soon as it finds
one. The four strategies form a **restricted template sequence for measure exploration**; their
input is refinement constraints (strategies 1–4 all begin with “a variable of a bounded type”), not
“compile-time-evaluated code.” They are the automatic and manual sides of the same explicit measure
described in §6.9.

> **Measure exploration is exploration, not inference.** Exploration only enumerates templates
> (linear rank, violation count, bounded patterns, multiplicative scaling) and does not guarantee
> that a solution exists—general measure inference is generally undecidable (it reduces to the
> halting problem). Therefore, cases outside the templates must allow the programmer to provide an
> explicit measure (§6.9); otherwise, compilation fails.

#### 6.2 Strategy 1: Automatic synthesis of a linear ranking function

When a variable has a linear-bound annotation, the compiler enumerates candidate linear measures and
validates them with SMT.

```
输入：
  变量 v₁: UpTo(u₁), v₂: UpTo(u₂), ...（有上下界的变量）
  循环条件 cond
  循环体中的赋值集合

算法：
  1. 从类型标注提取每个变量的界：[low_i, high_i]
  2. 枚举候选度量：v_i, u_i - v_i, v_i - v_j, 等线性组合
  3. 对每个候选度量 m：
     - SMT 验证 m ≥ 0（从类型界推导）
     - 对循环体的每条执行路径，SMT 验证 m' < m（严格递减）
  4. 找到符合条件的线性组合 → 终止得证
```

Coverage: loops in which any variable is assigned a linear expression (`v = a·v + b`) and has a
bounded type annotation. This includes `i += const`, `i -= const`, and interval contraction in
binary search:

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

#### 6.3 Strategy 2: Predicate violation count—automatic measure extraction from the target type <span style="color:orange">【Experimental Strategy】</span>

> ⚠️ **Current status: an experimental strategy whose inclusion will be decided during Phase 3
> implementation according to actual feasibility.** This strategy works for adjacent-swap operations
> (bubble sort, insertion sort), but cannot automatically prove non-adjacent operations (quicksort
> partition, heap-sort sift-down). See the table below for coverage boundaries. If Phase 3
> validation proves infeasible, this strategy will be removed or deferred to future work.

Core insight: **the specifications users write are material for compiler reasoning.** The compiler
does not need a built-in definition of “what sorting means”—it reads the definition of `Sorted` and
automatically extracts a measure from it.

```
输入：
  目标类型：Sorted(arr) = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }
  循环体操作：相邻元素交换

算法：
  1. 解析谓词定义：forall i in range: cond(i, arr)
  2. 自动生成度量：violation_count = |{ i | ¬cond(i, arr) }|
  3. 分析操作对度量的影响：
     - 相邻交换 arr[j], arr[j+1] = arr[j+1], arr[j]
     - 只影响索引 j-1, j, j+1 三对
     - 如果 arr[j] > arr[j+1]（违反谓词），交换后这对满足谓词
     - violation_count 至少减 1
  4. 上界：n·(n-1)/2（最大相邻逆序数），下界：0
  → 终止得证
```

**Current coverage**:

| Algorithm      | Operation pattern | Can Strategy 2 prove? | Reason                                                 |
| -------------- | ----------------- | :-------------------: | ------------------------------------------------------ |
| Bubble sort    | Adjacent swap     |          ✅           | `violation_count` strictly decreases on each swap      |
| Insertion sort | Adjacent movement |          ✅           | Each shift eliminates one violating pair               |
| Selection sort | Non-adjacent swap |          ❌           | A single swap may increase `violation_count`           |
| Quicksort      | Partition         |          ❌           | Non-adjacent swaps do not guarantee monotonic decrease |
| Heap sort      | Sift-down         |          ❌           | Tree operations make `violation_count` non-monotonic   |

**Complementary strategy**: for quicksort, the `low < high` interval contraction can be covered by
Strategy 1 (linear ranking function)—the outer partition recursion halves the interval each time.
Strategies 1 and 2 provide complementary coverage, and most practical algorithms can be proved by
one or the other. Generalizing Strategy 2 to non-adjacent and tree operations remains an open
problem.

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

#### 6.4 Strategy 3: Bounded increment/decrement patterns

`v += const` (a positive constant), with an upper-bound type annotation on the variable → measure
`upper_bound - v` decreases by `const` each time, with lower bound 0. This is a degenerate case of
Strategy 1, and the compiler handles it first for speed.

#### 6.5 Strategy 4: Multiplicative-scaling measure template

`v *= const` (`const > 1`), with upper- and lower-bound type annotations on the variable. The
compiler includes the logarithmic measure template `ceil(log_const(upper/v))`, which decreases by 1
each time the value is multiplied by `const`.

```yaoxiang
mut i: Positive(i) = 1
while i < n {
    # The compiler automatically infers: measure ceil(log₂(n/i)); multiplying by 2 decreases the measure by 1
    i *= 2
}
```

#### 6.6 Separation of termination and correctness

Termination proofs and correctness proofs are independent:

- **Termination**: the four strategies above automatically prove that the loop exits in finitely
  many steps; if exploration fails, the programmer supplies a measure in type position (§6.9)
- **Correctness**: whether the loop body progresses toward the target type is checked by the
  compile-time proof pipeline through verification conditions

If both pass, compilation succeeds. If termination is proved but correctness fails → compile error +
counterexample. If correctness is proved but termination cannot be proved → compile error
identifying the variable or operation that could not be analyzed. If both fail → compile error
reporting the two failure causes separately.

#### 6.7 Termination checking for recursive functions

For recursive functions with refined signatures, the compiler checks that formal parameters decrease
at every recursive call site:

```yaoxiang
gcd: (a: Int, b: NonNegative(b)) -> Int = {
    if b == 0 { return a }
    return gcd(b, a % b)  // The compiler explores: the well-founded-order measure of (b, a % b) decreases → termination
}
```

Formal-parameter decrease is the strongest automatic route (structural recursion). If exploration
fails, the programmer explicitly supplies a measure in type position (§6.9).

#### 6.8 Hard boundary

For `i = f(i)`, where `f` is not invertible, is not closed, and preserves no monotonicity—automatic
termination proof is mathematically impossible. Compile error:

> This loop cannot be proved to terminate automatically. The loop variable depends on the
> unanalyzable function `f`. Please use an iteration pattern that the compiler can analyze, or bind
> a name to the loop and provide a measure in type position (§6.9).

This is not a compiler failure. Any code that cannot be proved safe statically must not pass
compilation. Even when an explicit measure is provided, it must be judged true by SMT before it is
accepted—an incorrect measure is rejected with a counterexample. Humans can fail to prove a
property, but cannot prove a false one.

#### 6.9 Explicit measure: `Terminates`

When automatic exploration cannot find a measure, the programmer writes the measure in **type
position**—through the same mechanism as `Positive(b)` and `IsMax(T, arr, result)` (predicate
application), with no new syntax:

```yaoxiang
// Measure: an ordinary function that can be unit-tested, reused, and kept out of runtime execution
gcd_measure: (a: Int, b: Int) -> Int = { b }

// The termination component lives in the function's own type position
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// Loop: the binding name is the anchor, and the measure obtains scope-local quantities by name
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n {
        i = i + 1
        i
    }
    return acc
}
```

The rationale for the loop case is that `Terminates(m)` refines the value type of the loop body's
trailing expression, with the anchor supplied by the binding name `acc`—the loop can therefore be
referred to, eliminating the “anonymous construction cannot be referred to” corner case.

> **The body tail must provide a value**: the value of a loop = the value of its body block (§2.9),
> and the value of a block = its trailing expression. If the body tail is an assignment statement
> (such as `{ i = i + 1 }`), the block value is `Void`, and `return acc` would return `Void` from a
> `-> Int` function, which is ill-typed. The example therefore explicitly writes `i` as the body
> tail. This does not change the measure's semantics—the measure `n - i` is a **state expression**,
> independent of the loop's value type.

> **Implementation note (registered 2026-10-02; updated 2026-10-04)**: the **type side** of this
> section's loop example is valid (`check` reports 0 errors and the back-edge obligation is Proved).
> The **runtime extraction defect (#409) has been fixed**: a `while` used as a value now takes the
> value of the body block's trailing expression from the last iteration, consistent with the
> type-side `block_value_ty`—`loop(4) == 4` and `countdown(3) == 30` hold in actual execution (the
> same fix applies to `for`).
>
> **Known residual issue (zero iterations)**: if the loop body is never evaluated, there is no “body
> value from the last iteration,” and the loop value remains `Void`—binding it to a non-`Void` type
> position and using it at runtime reports
> `E6007 Runtime error: type mismatch in comparison Eq: Void vs Int(0)` (a loud type mismatch, not a
> silently incorrect value). This is a **static-approximation boundary**: the `block_value_ty`
> criterion yields the type of the body's trailing expression but cannot determine whether the loop
> executes at least once (which would require dataflow/provability analysis), so the type side does
> not silently degrade—it would break the value type required by this section's `Terminates` loop
> binding. The residual issue's contract is pinned by the corpus
> `tests/yaoxiang/02-type-system/termination/while_zero_iteration_void_err.yx`
> (`// expect: runtime-error E6007`). **This residual issue has been confirmed as an intentionally
> retained design boundary** (decision of 2026-10-04; no fix for now), so issue
> [#409](https://github.com/ChenXu233/YaoXiang/issues/409) has been closed with the main form fixed,
> and no separate tracking item will be opened. See the minimal reproduction and impact history in
> [RFC-027a](../accepted/027a-termination-explicit-measure.md) §Example: Loops, “Known Defect D6.”
> In addition, termination Strategy 1 (linear ranking function) has been restored in this round, so
> this section's order—“write an explicit measure only when automatic exploration fails”—remains
> unchanged: the explicit measure is still the fallback after exploration fails.

**`Terminates` is a built-in predicate**, belonging to the same core primitives as `Int` and
`Never`. It is the only predicate **for which the compiler writes the function body**—its assertion
(“the measure strictly decreases at every recursive call site/loop back edge”) is inherent in the
computation structure. User-written predicates cannot refer to the function body or loop body, so
this cannot be expressed using the predicate-definition syntax described in the “Syntax” section.
The built-in surface is therefore limited to this one name.

**Arity**. `Terminates(FnType, m)` and `Terminates(m)` are two arities of the same predicate, not
two separate constructs:

| Form                    | Anchor                         | Use case                                                          |
| ----------------------- | ------------------------------ | ----------------------------------------------------------------- |
| `Terminates(m)`         | Name of the containing binding | Self-recursive functions and loops—the default form               |
| `Terminates(FnType, m)` | Explicit function type         | Scenarios such as mutual recursion where the anchor is not unique |

The two are fundamentally the same: the termination obligation always falls on “the computation
segment denoted by the type position containing the refinement.”

**The measure does not restrict the return type.** A measure may be an expression of any type (it
need not be a natural number); “strict decrease” is determined by the well-founded order available
on that type. Whether the measure is well-founded (for example, whether a returned `Int` is `>= 0`)
is an **independent obligation**, handled in the same way as the decrease obligation, either through
refinement inference or SMT. When neither can be derived, the diagnostic **does not directly
reject** the program; instead, it suggests what to check (whether the measure's lower bound holds
and whether recursive arguments really move in that direction).

**Relationship to automatic exploration**: an explicit measure is not another pipeline. It is the
input used after exploration fails. Once a measure is supplied, the same SMT machinery validates
decrease and well-foundedness; if either fails, an error and counterexample are reported.

The implementation mechanisms for obligation generation, the decision pipeline, diagnostic
direction, and the shared strongly connected components (SCC) of mutual-recursion measures are
described in
[RFC-027a: Explicit Measures for Termination Checking](../accepted/027a-termination-explicit-measure.md).

### 8. The SMT solver: an accelerator module for the type checker

In traditional languages, the SMT solver is an external tool (for example, F\* invokes Z3, and Dafny
invokes Z3). In YaoXiang, it is an **accelerator module for the type checker**—it is invoked only
when the compiler kernel itself cannot directly decide. SMT helps find proofs, but the type checker
validates the proof.

**Trust model**: the type checker is the sole trust root. The SMT solver is an accelerator module—it
helps find proofs, but SMT is not an independent trust boundary. The compiler trusts Z3's `unsat`
result (consistent with the F\*/Dafny approach—the practical engineering choice that Z3's error
probability is lower than the compiler's own bug rate). The real unreliability is confined to the
SMT translation layer—if the translation has a bug, the compiler will expose it in other tests.

**Interface**: the compiler translates to the SMT-LIB 2.6 standard format internally instead of
binding to a solver-specific API. SMT-LIB is an ISO standard, and Z3, CVC5, MathSAT, and Yices all
support it natively.

**Default backend**: Z3 (MIT license, with the broadest documentation and community validation).
CVC5 serves as an SMT-LIB-compatible alternative, selectable through a compiler flag at build time.

There is no “general solver abstraction layer”—SMT-LIB is the abstraction layer. If CVC5 later makes
progress in a particular theory, switching requires only replacing the binary, not changing compiler
code.

```
编译期 Bool 表达式
        │
        ├── 编译器内核可直接判定（结构等价、简单算术、
        │   常量折叠后的平凡公式）
        │   → 直接返回 Proved / Disproved
        │
        └── 编译器内核无法直接判定（量词、符号变量）
            → 依赖类型预归约（factorial(5) → 120）
            → 翻译为 SMT-LIB 格式
            → 发送给 Z3/CVC5（带预算限制）
            → 返回值：unsat → Proved  │  sat + 模型 → Disproved  │  unknown → Unproven
```

**Solver budget—a hard limit, like stack depth**:

| Budget dimension               | Default | Description                                                                                                                                                                 |
| ------------------------------ | ------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Solver step count              | 10,000  | Z3 usually completes linear arithmetic in under a hundred steps. 10,000 steps covers 99% of practical predicates.                                                           |
| Time                           | 100ms   | A single predicate exceeding 100ms means the user is writing a compile-time program rather than a type annotation. 100ms × 50 predicates = a 5-second compile-time ceiling. |
| Quantifier instantiation depth | 3       | Three levels of nested quantifiers cover practical patterns. Beyond three levels, the user is probably writing logic exercises.                                             |

Exceeding the budget returns Unproven, a compile error, the predicate location, and the amount
consumed. There is no degradation, no runtime checking, and no silent pass.

**Why this is practical in practice**: 95% of practical predicates in engineering are linear
arithmetic—`x > 0`, `arr.len > 0`, `0 <= idx < arr.len`—all within a decidable fragment, and the SMT
solver returns in milliseconds. For the rare complex predicate that exceeds the budget, the
programmer can write a proof function.

Dependent types receive one layer of pre-reduction before the SMT invocation: `factorial(5)` is
directly evaluated at compile time to `120`, and `append([1,2], [3])` is directly evaluated to
`[1,2,3]`. These deterministic value computations do not consume SMT budget.

Programmers do not need to know that SMT exists. The mental model is: **the compiler passes what it
can prove and reports an error otherwise—if the compiler cannot prove it, you can write a function
that proves it for the compiler.**

### 9. Composition of compile-time predicates

A compile-time predicate is a function that returns Type, and composition is naturally implemented
through function composition:

```yaoxiang
SortedNonEmpty: (T: Ord, arr: Array(T)) -> Type = {
    Sorted(T, arr) and NonEmpty(arr)
}
```

### 10. Code examples

#### 9.1 Safe division

```yaoxiang
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b

result = divide(10, 2)   # ✅ The compiler validates Positive(2) = { 2 > 0 } → True
# result = divide(10, 0)  # ❌ The compiler validates Positive(0) = { 0 > 0 } → False
```

#### 9.2 Safe array access

```yaoxiang
InBounds: (idx: Int, arr: Array(T)) -> Type = { 0 <= idx and idx < arr.len }

get: (arr: Array(T), idx: InBounds(idx, arr)) -> T = arr.data[idx]

arr = Array(Int)(1, 2, 3)
x = get(arr, 1)   # ✅ The compiler validates InBounds(1, arr) = { 0 <= 1 and 1 < 3 } → True
# y = get(arr, 5)  # ❌ The compiler validates InBounds(5, arr) = { 0 <= 5 and 5 < 3 } → False
```

#### 9.3 Sorting correctness

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

#### 9.4 Loops: compiler VC generation

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

### 11. The dispatch routing pipeline: unified compile-time and runtime dispatch

> **Implementation note (2026-10-03, #377-2)**: the **semantics** of this section are established
> and integrated into production, but the **structure** is not a standalone `layers/dispatch.rs`
> module. Production dispatch is implemented directly through three **call-site obligations** in
> `checker.rs` (direct calls to `layers::predicate::check_predicate`): binding-site revalidation
> (`revalidate_refined`), call-site arguments (`check_call_arg_refinements`), and return-site
> postconditions (`check_return_refinement`). The original `layers/dispatch.rs` had zero production
> call sites, and its handling of `Unproven { ProofFunctionRequired }`—downgrading it to a W1080
> warning and injecting Γ—conflicted with the production implementation (`Unproven` always reports
> an error; see `refined_unproven`), so it was removed in its entirety. Also note that the Runtime
> row's “insert runtime check” currently has **no production emission point**
> (`RuntimeOutcome::InsertCheck` has no consumer), while Γ injection is handled independently by
> branch guards in `ownership.rs` and `inference/expressions.rs`.

`assert` and `Assert` are two sides of the same refinement-type primitive. The dispatch pipeline
`dispatch` automatically chooses between compile-time proof and runtime checking based on **whether
the predicate's free variables are available at compile time**:

| Criterion                                                                                       | Mode            | Behavior                                                                                       |
| ----------------------------------------------------------------------------------------------- | --------------- | ---------------------------------------------------------------------------------------------- |
| All free variables are known at compile time (generic parameters, compile-time constants)       | **CompileTime** | Enter the proof pipeline: Proved → erase, Disproved → compile error, Unknown → require proof   |
| Some free variables come from runtime (function parameters, external inputs, mutable variables) | **Runtime**     | Insert a runtime check and inject the refinement fact into the flow-sensitive assumption set Γ |

**Key point**: “cannot decide” ≠ “disprove.” In CompileTime mode, Unknown requires proof—there is no
silent degradation. In Runtime mode, the proposition has no truth value at compile time in the first
place—no prover, however strong, can write a universally true proof for “the user might input a
negative number,” making a runtime check the only sound choice. This is not a weakness of the
prover; it is a theoretical necessity.

### 12. Flow-sensitive assumption set Γ: strongest-postcondition propagation

The compiler maintains a flow-sensitive assumption set Γ that tracks propositions known to hold at
each control-flow point.

**SP (strongest-postcondition) propagation**:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
```

**Kill set for mutable variables**: after a `mut` variable is reassigned, all assumptions involving
that variable are removed from Γ:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
mut x = x - 5       // Γ = {}  ← x > 0 is killed
```

This is a hard soundness requirement—once the variable's value changes, the old assumption is
invalid.

**Branch confluence**: when IF/ELSE or match branches are merged, Γ takes the intersection of the
branch assumptions. Only propositions that hold on every path are carried out of the branch.

**Equality bridge (automatic evidence for the q=0 slot)**: constant-equality facts in Γ are
legitimate evidence for compile-time value parameters (RFC-011 §4.1, the q=0 slot). Within the guard
branch of `if y == 42 { f(y)(y) }`, Γ records `y == 42`, so `y` is treated as 42 in the q=0 slot and
the call follows the existing path check for `f(42)`. One side of the equality must be a
compile-time constant; a runtime value without equality evidence still produces a report at the call
site. This is not a new mechanism—the verification condition `{y == 42} ⇒ {y ≡ 42}` uses the
existing implication checker.

### 13. Clarification of the erasure model: erasing witnesses ≠ erasing checks

RFC-027's claim that “refinement types are **completely erased** at runtime” refers specifically to
**proof witnesses (proof tokens)**—proof terms validated at compile time do not generate runtime
code. However, the **runtime check** inserted by dispatch in Runtime mode is retained—it is a Bool
check executed at the value layer, not a type-level witness.

In summary: witnesses are erased; checks are retained. The two do not conflict, and RFC-027's
original claim remains unchanged.

## Detailed Design

### Syntax changes

| Before (RFC-022)                      | After (this RFC)                                                                                                                                                   |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `//! requires: NonEmpty(n) = n > 0`   | Compile-time predicates as parameter types `(b: Positive(b))`                                                                                                      |
| `//! ensures: ExistsMax(result, arr)` | Return types use a return-value formal parameter `-> (result: IsMax(T, arr, result))`                                                                              |
| `/*! invariant: ... !*/`              | Compile-time predicate type annotations on variables—Floyd–Hoare invariants                                                                                        |
| `//! decreases: n`                    | The measure lives in refinement-type position (`Terminates`); the compiler explores automatically first and requires an explicit measure only if exploration fails |
| Specifications are comments           | Specifications are part of the type system                                                                                                                         |

### Syntax

**Compile-time predicates introduce no new keywords.** `{}` is the proof space and uses exactly the
same syntax as existing type definitions. A compile-time predicate is simply a function that returns
Type—`name: (params) -> Type = { assertions }`. It is used through function
invocation—`Positive(b)`, `IsMax(T, arr, result)`.

```bnf
# A compile-time predicate is a function returning Type; the compiler validates the assertions inside {}
# Uses existing function/type syntax; no new BNF rules are required
predicate ::= identifier ':' params '->' 'Type' '=' '{' assertions '}'
```

**The arguments of a predicate application must have compile-time-constant form**—literals,
variables (bound by name), type applications (recursively extracted), or **compile-time-referencable
function references** (function names). If an argument's form cannot be converted to a constant
expression, report **E1092**; if the number of arguments does not match the predicate declaration's
number of formal parameters, report **E1093**. Arguments are bound positionally to the
formal-parameter list, and the predicate's arity is determined by its declaration—`Positive(x)` is
unary, `IsMax(T, arr, result)` is ternary, and `Terminates(m)` and `Terminates(FnType, m)` are unary
and binary. Refinement constraints are **never silently discarded** (previously, an unconvertible
argument caused the constraint to disappear silently, allowing a binding that violated the
constraint to pass silently).

**New syntactic concept: the return-value formal parameter**—in `-> (name: Type)`, `name` is the
return-value formal parameter.

The return-value formal parameter is the **only syntactic concept** YaoXiang introduces on top of
the existing function syntax. Its semantics are:

- The value of `name` is supplied by a `return` statement
- `name` exists only in the type signature and is referenced by postcondition predicates (such as
  `-> (result: IsMax(T, arr, result))`)
- `name` does not enter the function body's scope and does not appear at the call site
- The return-value formal parameter is **optional**—without a postcondition, the signature is
  exactly that of an ordinary function (`-> Int`), with no additional burden

The reason for introducing it is that postconditions need to refer to “the value the function is
about to return.” Without a return-value formal parameter, the compiler would need special rules
(such as an implicit variable `$result` or `__retval__`) for predicates to refer to the return
value. The return-value formal parameter makes this reference explicit—it is simply a formal
parameter whose value is supplied by `return` rather than by the caller.

A **proof function** is not a new concept—it is a YaoXiang function whose return type is the
asserted proposition. When the compiler returns Unproven, the programmer supplies a proof function,
and the type checker validates it in exactly the same way it validates the return type of any
function. No new syntax, keywords, or rules are required.

> **The boundary between the two.** In the correctness domain (predicate Unproven), the proof is
> supplied in the **body**: write a function whose return type is the proposition to be proved. In
> the termination domain (§6.9), the measure is supplied in **type position**: write
> `Terminates(measure)` as the type of a binding or function signature. The former means “write a
> proof for a proposition that cannot be proved”; the latter means “explicitly declare a measure
> that exploration cannot find”—the mechanism is the same in both cases (refinement-type
> application), but the placement differs. The termination domain does **not** require writing a
> `_proof` function.

### Type-system implications

- **Type universe**: compile-time predicates live at the Type₂ layer—they accept functions that
  return values of type Type, at the same hierarchy level as type constructors
- **Generics interaction**: compile-time predicates may take generic parameters, such as
  `NonEmpty: (T: Type) -> (arr: Array(T)) -> Type`
- **Ownership interaction**: expressions in compile-time predicates obey ownership rules and may
  only read, not write
- **Type inference**: arguments to compile-time predicates participate in HM type inference

### Runtime representation

Compile-time predicates are handled at runtime **according to the dispatch result**:

- **CompileTime mode** (all free variables are known at compile time): the witness token is
  completely erased after the proof succeeds. `Positive: (x: Int) -> Type = { x > 0 }`—the runtime
  representation of a parameter `b: Positive(5)` is simply `Int`. The refinement condition
  `{ 5 > 0 }` has already been validated and is erased.
- **Runtime mode** (some free variables come from runtime): retain the runtime check—execute a Bool
  check at the value layer and inject into the flow-sensitive assumption set Γ. See §11's dispatch
  routing pipeline and §13's clarification of the erasure model.

Placing a compile-time predicate in a type position (such as `f(x: Positive(x))`) does not produce a
wrapper type or allocate extra memory. However, if `x` comes from runtime input, a runtime Bool
check **is** inserted.

**Interaction constraint with `ref`**: compile-time predicates may refer only to immutable borrows
or values whose ownership has been transferred. A compile-time predicate that refers to a mutable
borrow cannot guarantee at compile time that the validation result will still hold at runtime—such
use is rejected directly with a compile error.

### Compiler changes

1. **Parser**: compile-time predicates use standard function syntax and require no additional
   parsing rules
2. **Compile-time proof pipeline**: a unified Proved/Disproved/Unproven return interface with
   automatic strategy selection
3. **SMT accelerator module**: an SMT-LIB 2.6 translation layer, with Z3 as the default backend and
   CVC5 as an alternative
4. **Type-checker kernel**: inference-rule implementation—structural equivalence, βδι-reduction, and
   universal-quantifier introduction/elimination. This is the sole trust root; both SMT and
   programmer proofs are validated through it
5. **Verification-condition generation**: WP/SP calculi + loop-invariant proof obligations
6. **Error reporting**: counterexample formatting + unresolved-proposition reporting +
   source-location association

### Backward compatibility

- ✅ Code that does not use compile-time predicates remains completely unchanged
- ✅ Compile-time predicates have zero runtime overhead in CompileTime mode; Runtime mode retains
  only the necessary Bool checks
- ⚠️ RFC-022's `//!` syntax is no longer supported—but 022 was never implemented, so there is no
  migration burden

## Trade-offs

### Advantages

- **Full realization of the Curry–Howard correspondence**: types are propositions, programs are
  proofs, `name: Proposition = Proof`
- **Uniformity**: compile-time predicates and ordinary functions use exactly the same syntax, with
  no conceptual split
- **Transparent SMT**: programmers do not need to know that SMT exists; the mental model remains
  consistent with type checking
- **Gradual adoption**: start with one compile-time predicate and progressively expand coverage
- **Minimal runtime overhead**: zero overhead in CompileTime mode; Runtime mode retains only the
  necessary Bool checks

### Disadvantages

- **Compile time**: SMT solving increases compile time, but hard budget limits keep the upper bound
  manageable
- **Automatic-proof boundary**: complex predicates beyond first-order linear arithmetic may require
  the programmer to write a proof function. This is not a language defect—it is an inevitable
  consequence of the halting problem. The compiler honestly reports Unproven rather than falsely
  reporting True/False
- **Learning curve**: writing effective compile-time predicates and proof functions requires
  understanding the basic intuition of the Curry–Howard correspondence
- **Implementation complexity**: unifying the compile-time proof pipeline requires careful design

### Risk mitigation

- Hard SMT solver budget limits (10,000 steps / 100ms / instantiation depth 3); exceeding the budget
  returns Unproven
- Dependent-type pre-reduction: deterministic value computations are handled first, and SMT handles
  only the nondeterministic remainder
- Unproven is not a dead end: in the correctness domain, write a proof function (the return type is
  the proposition); in the termination domain, supply a measure in type position (§6.9)—both are
  validated by the type checker
- Incremental validation: validate only changed modules
- Clear error messages + counterexample display + budget-consumption report + unresolved
  proposition + suggestions (when the compiler can provide them)

## Alternatives

| Option                                                             | Why it was not selected                                                                                                                                                            |
| ------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| RFC-022: comment-based specifications using `//!`                  | Splits specifications and types, violating the Curry–Howard correspondence                                                                                                         |
| Standalone specification files (such as CVL)                       | Separates specifications from code and increases maintenance costs                                                                                                                 |
| Runtime assertions only                                            | Cannot statically guarantee correctness                                                                                                                                            |
| External proof assistants (such as Coq)                            | Disconnected from the compiler and requires a separate proof language and trust boundary. YaoXiang's choice: proofs are YaoXiang code, and the type checker is the sole trust root |
| **This proposal: compile-time predicates as first-class citizens** | ✅                                                                                                                                                                                 |

## Implementation Strategy

### Phase division

| Phase       | Content                                                                                                                                                                 |
| ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Phase 1** | Compiler kernel: structural equivalence + βδι-reduction + universal-quantifier introduction/elimination. Supports simple arithmetic predicates (`x > 0`, `arr.len > 0`) |
| **Phase 2** | SMT-LIB translation layer + Z3/CVC5 integration. The pipeline returns Proved/Disproved/Unproven. When Unproven is returned, programmers may write proof functions       |
| **Phase 3** | Loop-invariant VC generation + termination checking (four measure-exploration strategies + explicit `Terminates` measures, §6 and §6.9)                                 |
| **Phase 4** | Incremental validation + caching + IDE support                                                                                                                          |

### Dependencies

- RFC-010: Unified Type Syntax — compile-time predicates are based on `name: type = value`
- RFC-011: Generic Type System — compile-time predicates may take generic parameters
- RFC-009: Ownership Model — expressions in compile-time predicates obey ownership rules

## Open Questions

- [x] **Z3 availability for the wasm32 target**: there is no Z3 under wasm (all SMT code is excluded
      through `cfg(not(target_arch = "wasm32"))`). Scope and degradation directions: -
      `ownership.rs` `smt_cut` always returns `false` → back edge traversed → **conservative
      rejection** (sound direction; merely less narrowing) - Termination-checking Strategy 1 and
      predicate levels 2b/3 are skipped entirely

  **Correction (2026-10-01)**: the original text described these paths as “currently
  **unreachable**” (on the grounds that `predicate_defs` was never populated in production, `parser`
  did not produce `MonoType::Refined`, and production `with_solver` was never invoked). That is no
  longer true—all three have been implemented (predicate-definition registration #377-3;
  refinement-type normalization; production pipeline solver injection). The difference between wasm
  and native is therefore no longer limited to `smt_cut` precision:

  | Path                                            | native   | wasm       | Degradation direction                |
  | ----------------------------------------------- | -------- | ---------- | ------------------------------------ |
  | Explicit termination-measure decision           | Decided  | `Unjudged` | No E4022 is reported (fewer reports) |
  | Well-foundedness decision                       | Decided  | `Unjudged` | No E4022 is reported (fewer reports) |
  | SMT implication level for refinement predicates | Executed | Skipped    | Falls back to `Unproven`             |

  All are **conservative** directions (less narrowing and fewer reports) and do not affect
  soundness. To achieve consistent precision, see issue #376 for the options and costs (the
  conclusion is to prioritize a JS-side Z3 instance rather than linking Z3 into the main wasm
  binary—the latter would require replacing the emcc build system and increase the artifact from 3
  MB to ~20 MB).

- [x] **SMT solver choice**: Z3 by default (MIT license, with the broadest validation). CVC5 as an
      SMT-LIB-compatible alternative, selectable through a compiler flag. The compiler's internal
      translation target is the SMT-LIB 2.6 standard format—SMT-LIB is the abstraction layer, and no
      custom general solver interface is provided.
- [x] **Specific solver-budget values**: 10,000 steps / 100ms / quantifier-instantiation depth 3.
      These are fixed inside the compiler and exposed through no knob. If actual use cases
      demonstrate that they are insufficient (and not merely that “the user wrote it incorrectly”),
      they may be adjusted later.
- [x] **Quantifier support range**: the language does not restrict quantifier order. Compile-time
      predicates accept Type parameters—Type includes function types—so higher-order quantifiers are
      a natural consequence of the type system and require no special syntax. The SMT solver can
      automatically decide first-order quantifiers (forall/exists, including interleaved nesting,
      limited by budget depth 3). For higher-order quantifiers, SMT returns Unproven and the
      compiler prompts, “This predicate is outside the range of automatic proof; please provide a
      proof function.” The programmer writes a YaoXiang function whose return type equals the
      proposition, and the type checker validates that function. No external export, no AI, and no
      interactive proof mode are required. Everything is YaoXiang code, and everything is validated
      by the type checker.
- [x] **Counterexample formatting**: source variable names are used directly as SMT variable names,
      with a module prefix to avoid collisions. When a Z3 model is returned, it is looked up by
      variable name. Output format: variable name = concrete value + source location +
      predicate-definition location. No complex mapping layer is implemented.
- [x] ~~**Interaction between compile-time predicates and the `ref` smart pointer?**~~ → Decision:
      compile-time predicates may refer only to immutable borrows or values whose ownership has been
      transferred. Values under mutable borrow may not occur in compile-time predicates.
- [x] **Extending `forall` predicate violation-count measures to non-adjacent operations?** → Do not
      extend. The current coverage (adjacent swaps, adjacent movement) is complemented by Strategy 1
      (linear ranking function)—quicksort's outer interval contraction is covered by Strategy 1,
      while heap sort is covered by Strategy 1 (the array-index pattern). A loop that cannot be
      proved to terminate by any strategy causes the compiler to report an error directly—this is
      the hard-safety philosophy, not a defect. If a real-world, non-academic use case arises in
      which all four strategies fail, the question will be reconsidered.→ **Reconsideration has been
      triggered (2026-09-14, #318)**: non-structural recursion (gcd-like non-direct decrease, mutual
      recursion, merge partitioning) is exactly the kind of real-world scenario anticipated by this
      clause—the template sequence for measure exploration cannot automatically handle it, and it
      cannot be rewritten into an analyzable iteration pattern without sacrificing readability.
      Conclusion: the hard-safety philosophy remains unchanged (an incorrect measure is still
      rejected by an SMT counterexample; humans can fail to prove a property but cannot prove a
      false one), but “failure to explore means rejection” is relaxed to “failure to explore may be
      handled by the programmer explicitly providing a measure in type position”—see §6.9.
- [x] **Combinatorial explosion in linear ranking-function enumeration**: candidate enumeration is
      limited to three bounded variables. With ≤3, it enumerates every linear combination and
      validates each with SMT. With >3, it attempts only single-variable measures (`v_i`,
      `u_i - v_i`) and reports a compile error if they fail, advising the programmer, “The loop has
      more than three bounded variables, and the compiler cannot automatically synthesize a
      multivariable measure.” This is not an engineering compromise—it forces programmers to write
      simpler loops.

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

## Lifecycle and Final Destination

```
┌─────────────┐
│   草案      │  ← 作者创建
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  审核中     │  ← 当前状态：社区讨论
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  已接受     │    │  已拒绝     │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│ accepted/   │    │  rejected/  │
│ (正式设计)  │    │ (保留原位)  │
└─────────────┘    └─────────────┘
```
