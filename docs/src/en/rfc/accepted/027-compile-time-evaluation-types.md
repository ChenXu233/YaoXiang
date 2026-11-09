---
title: 'RFC-027: Compile-Time Predicates and Unified Static Verification'
status: 'Accepted'
author: 'Chen Xu'
created: '2026-06-07'
updated: '2026-10-03'
impl_status: 'in-progress'
impl_detail:
  'Phase 1-2 complete, Phase 3 partially complete, Phase 4 partially complete. The unified
  assert/Assert approach has been fully implemented across all 6 phases (#157-#162 closed): Never
  type, IsTrue bridge, flow-sensitive Γ + kill set, type-level recursion, and weak universe
  stratification checks. The **semantics** of dispatch in §11 have been implemented through
  call-site obligations (three direct calls to `check_predicate` in `checker.rs`: binding position /
  call-site argument / return position), but the **standalone `layers/dispatch.rs` module has been
  deleted**—it had no production call sites, and its handling of Unproven (downgrading it to a W1080
  warning) conflicted with the production implementation (always reporting an error) (#377-2).'
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

## Abstract

This RFC proposes introducing **compile-time predicates** into YaoXiang as first-class citizens and
unifying all compile-time static verification into a single **proof pipeline**. A compile-time
predicate is not an add-on specification comment—it is a function. A function returning Type can be
used in a type position, where the compiler calls it at compile time and checks its result. Types
are propositions, and compile-time evaluation is proof.

**Core thesis**: the sole task of type checking at compile time is to construct and verify proof
terms. Type equalities, token conflicts, dependent type reduction, compile-time predicate
evaluation, and Hoare-logic entailment are all different kinds of type checking within the same
compile-time proof pipeline, sharing one pipeline. The SMT solver is an acceleration module for the
type checker, not an independent trust boundary. When the compiler returns Unproven, the programmer
writes a YaoXiang function as the proof—the type checker verifies it in exactly the same way that it
verifies the return type of any function. Everything is YaoXiang code, and everything is verified by
the type checker.

## Motivation

### Why Deprecate RFC-022?

RFC-022 designed specifications in the form of `//!` comments:

```yaoxiang
max: (T: Ord) -> ((arr: Array(T, n)) -> T) = {
    //! requires: NonEmpty(n) = n > 0          ← This is a comment independent of the type
    //! ensures: ExistsMax(result, arr[0..n])   ← This is a comment independent of the type
}
```

This makes a fundamental error in the Curry–Howard correspondence: **it splits specifications and
types into two layers**. A comment is not a type. Comments do not participate in type checking.
Comments embody the mental model of an “external tool.”

The white paper states this plainly:

> “There are no `//!` comments. There is no separate specification language. Everything is within
> the type system.”

### Current Problems

- RFC-022's `//!` comments are add-on syntax independent of the type system
- Specification types and ordinary types are two separate systems, creating conceptual redundancy
- The split between verification in Debug builds and its omission in Release builds undermines
  uniformity
- SMT solvers are traditionally perceived as external tools—YaoXiang instead builds them into the
  type checker as an acceleration module
- Type checking, borrow checking, compile-time predicate checking, and macro expansion all follow
  different paths

### The Correct Mental Model

Type checking can be abstracted as a function:

```
verify : Program → Proved | Disproved(Model) | Unproven
```

Every compile-time check—simple type matching, borrow-conflict detection, and compile-time predicate
verification—is a subtask of this function. They all share the same proof pipeline, differing only
in the complexity of the proof term and the construction strategy.

When the compiler returns Unproven, the programmer supplies a proof function—its return type is
equal to the proposition to be proved. The type checker verifies it. This is the same operation as
ordinary type checking.

## Proposal

### 1. `{}` Is the Proof Space: Types Are Assertions, Verification Is Type Checking

YaoXiang's `{}` is the compile-time proof space. Everything inside it is an assertion, and the
compiler guarantees that every item is True—either by proving it automatically or by accepting a
proof function supplied by the programmer.

```
Point: Type = { x: Float, y: Float }
#               ^^^^^^^^^^^^^^^^^^^^^  The compiler guarantees that x is a Float and y is a Float

List: (T: Type) -> Type = { data: Array(T) }
#                           ^^^^^^^^^^^^^^^  The compiler guarantees that data is an Array(T)
```

**Generics are a special case of compile-time predicates.**

```yaoxiang
Positive: (x: Int) -> Type = { x > 0 }
#          ^^^^^^              ^^^^^^
#          Parameter at the signature       Only assertions inside {}
#          The compiler validates x > 0 when calling it at compile time

List: (T: Type) -> Type = { data: Array(T) }
#      ^^^^^^^^              ^^^^^^^^^^^^^^^
#      Parameter at the signature           The compiler validates type_of(T) == Type and type_of(data) == Array(T)
```

The same pattern applies: `name: (params) -> Type = { assertions }`. The compiler does not
distinguish “type assertions” from “value assertions”—both are evaluation targets in the proof
pipeline.

**Loop invariants do not need to be written separately. A type annotation on a variable is the
Floyd–Hoare invariant.**

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # The annotation refers to i—it tells the compiler that s's type depends on i
    mut i: UpTo(arr.len) = 0     # During initialization i=0; validation: 0 == sum(arr[0..0]) → True
    while i < arr.len {
        s += arr[i]  # The compiler validates: s_new == sum(arr[0..i+1])
        i += 1       # i changes → triggers revalidation of s's dependency: s satisfies SumUpTo(arr, i_new)
    }
    return s  # s: SumUpTo(arr, arr.len) = sum(arr[0..arr.len])
}
```

The compiler generates one verification condition for the loop body—the inductive hypothesis (type
annotation) → assignment operation → whether the new value satisfies the type annotation. Once the
proof pipeline establishes the inductive step, all iterations are covered automatically. There is no
need for `: decreases`, `: Invariant`, or an inductive proof—the compiler decomposes induction into
a local VC for each assignment.

### 2. Preconditions/Postconditions: Compile-Time Predicates on Parameter and Return Types

Abandon RFC-022's `//! requires`/`//! ensures`. Compile-time predicates become type annotations on
parameters or return values.

**On the parameter side, this is a function call.** A compile-time predicate is a function returning
Type, and using it on the parameter side means calling it—just like `factorial(5)`. The return-value
side introduces a new concept: the return-value formal parameter.

```yaoxiang
# Precondition: explicitly call the compile-time predicate in the parameter type
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b
#                       ^^^^^^^^^^  b is the current formal parameter name and is passed to Positive
#                       The compiler extracts the actual argument at the call site, substitutes it for b, and validates Positive(actual argument)
#                       Example: divide(10, 2) → validates Positive(2) = { 2 > 0 } → True
#                       Example: divide(10, 0) → validates Positive(0) = { 0 > 0 } → False → compilation error

# Postcondition: return-value formal parameter + compile-time predicate
IsMax: (T: Ord, arr: Array(T), result: T) -> Type = {
    forall j in 0..arr.len: result >= arr[j]
}

NonEmpty: (arr: Array(T)) -> Type = { arr.len > 0 }

max: (T: Ord) -> ((arr: NonEmpty(arr))) -> (result: IsMax(T, arr, result)) = {
#                                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
#                                            result is the return-value formal parameter, and its value is supplied by return
#                                            The compiler substitutes the returned value at the return point and validates the postcondition
    candidate = arr[0]
    for i in 1..arr.len {
        if arr[i] > candidate { candidate = arr[i] }
    }
    return candidate
}
```

**Key rules**:

- **Parameter side**: `b: Positive(b)`—`b` is the current formal parameter name and is passed to
  `Positive`. This uses ordinary function-call syntax, with no implicit arguments.
- **Return side**: `-> (result: IsMax(T, arr, result))`—`result` is the return-value formal
  parameter, and its value is supplied by the `return` statement. `result` exists only in the type
  signature and is only referenced by predicates. It does not enter the function-body scope and does
  not appear at call sites.
- **The return-value formal parameter is optional**: when there is no postcondition, omit it. The
  signature is then identical to that of an ordinary function (`-> Int`).
- **Uniformity**: parameters and return-value formal parameters are the same
  concept—`formal parameter name: predicate call(actual argument)`—and differ only in whether the
  value is supplied by the caller or by `return`.

> **Implementation note (2026-10-02)**: return-position formal parameters are identified **strictly
> by their declared names**—the `r` in `-> (r: P(...))` is the sole return-value formal parameter.
> Any other free variables in the constraint that are **not in scope** are no longer substituted
> with the return value; instead, an undefined-identifier error is reported (observed:
> `g: () -> (r: Eq2(m, r)) = { return 7 }` (`m` is undefined) →
> `error [E1001] Unknown variable: 'm'`). The actual arguments of a predicate call are substituted
> into the constraint according to their **actual values**: `P(r + 100)` becomes “return value +
> 100,” not “return value.”
>
> **Known gap (2026-10-02)**: **multi-parameter predicates with symbolic arguments in return
> position are not yet supported**. Triggering form:
> `f: (b: Int) -> (r: SumUpTo(b, r)) = { return b * 2 }`
> (`SumUpTo: (n: Int, s: Int) -> Type = { s == n * 2 }`) →
> `error [E2031] … the refinement type constraint of 'r': SumUpTo(b, (b * 2)) cannot be statically proven`.
> Alternative: use a compile-time-foldable literal as the argument (for example, `SumUpTo(3, r)`),
> or supply a proof function returning `Type` for the predicate. Resolution condition: the proof
> kernel must support argument substitution for “multi-parameter predicates with symbolic
> arguments.” See [RFC-027a](../accepted/027a-termination-explicit-measure.md) §Implementation
> Record (2026-10-02, current hardening pass).

### 3. Path-Condition Propagation: Compile-Time Validation of Runtime Values

When a compile-time predicate is used in a binding position, its arguments are explicitly supplied
by the programmer. When a runtime value enters a refinement-type argument, the compiler validates it
by collecting path conditions and performing SMT entailment checking—there is no need for the
programmer to pass a proof explicitly.

#### 3.1 Explicit Function Calls

When a compile-time predicate is used in a binding position, the programmer explicitly supplies its
arguments—it is a function call, with no implicit arguments.

`Positive: (x: Int) -> Type = { x > 0 }` is a compile-time predicate constructor. When it appears in
a binding position (a parameter declaration, variable declaration, or return type), the programmer
explicitly supplies an already-bound variable name:

```yaoxiang
b: Positive(b)
// b is already declared as the current formal parameter, and Positive(b) is a function call
// After normalization: b: { b > 0 }
```

The compiler does not need to fill in arguments implicitly—`b: Positive(b)` is a function call, just
like `f(5)`. `b` is bound as a parameter name, and its type annotation `Positive(b)` refers to `b`
itself. This is the standard dependent-type pattern, not an implicit expansion rule.

**Unification with RFC-010 `self`**: RFC-010 established that `self` is not a keyword, merely a
conventional parameter name (“writing `p`, `this`, or `x` has exactly the same effect”).
`b: Positive(b)` uses the same mechanism—the parameter name may be referenced in a type annotation.
`self` appears in the position `self: Point`, while `b` appears in `b: Positive(b)`; both type
annotations refer to the parameter itself. The only difference is the complexity of the type
annotation; the mechanism is exactly the same—once a name is bound, a type may depend on that name.

Return types use explicit function calls in the same way:

```yaoxiang
Sorted: (arr: Array(T)) -> Type = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }

sort: (arr: Array(T)) -> (result: Sorted(result)) = { ... }
//                        ^^^^^^^^^^^^^^^^^^^^^^^
//                        result is the return-value formal parameter, and Sorted(result) is a function call
//                        The compiler substitutes the returned value for result at the return point and validates Sorted(returned value)
```

The same applies to local-variable declarations:

```yaoxiang
let x: Positive(x) = 5
// x is bound to 5, Positive(5) → { 5 > 0 } → True → passes

// let y: Positive(y) = 0
// y is bound to 0, Positive(0) → { 0 > 0 } → False → compilation error
```

#### 3.2 Path-Condition Collection

When a runtime value occurs in a conditional branch, the compiler automatically collects path
conditions to form the **assumption set** for the current scope. These assumptions participate in
validation as background knowledge for compile-time Bool evaluation.

```yaoxiang
if y > 0 {
    // The compiler automatically obtains the assumption in this branch: { y > 0 }
    let result = divide(x, y)
    // Verification condition: (y > 0) ⇒ (y > 0)
    // The proof pipeline finds the entailment valid → Proved
} else {
// This branch's assumption: { !(y > 0) }
// To call divide(x, y), the verification condition is !(y > 0) ⇒ y > 0
    // The proof pipeline finds that the entailment is invalid → Disproved
}
```

This is not a special pattern hard-coded into the compiler—it is the natural behavior of the
compile-time proof pipeline. At each type-check call site, the compiler sends the following to the
pipeline:

```
{背景假设} ⇒ {验证目标}
```

The proof pipeline checks entailment. Proved → pass; Disproved → compilation error plus a
counterexample; Unproven → compilation error plus the unresolved proposition. The background
assumptions come from the path conditions at the current program point.

#### 3.3 Assumption Stack

While analyzing control flow, the compiler maintains an assumption set for each basic block:

- **if-guard**: `if y > 0` → push `y > 0` into the true branch and `!(y > 0)` into the false branch
  (if an else branch is used)
- **match pattern**: `if let Some(v) = opt` → push `opt == Some(v)` into the branch
- **logical conjunction**: `if x > 0 and y < 10` → push `x > 0` and `y < 10` into the branch
- **function precondition**: when calling `divide(a, b)`, evidence that `b` satisfies `Positive`
  must come either from the current assumptions or from the actual argument's own refinement-type
  annotation (if `b` is already annotated as `Positive`, its type carries `b > 0`)
- **assignment**: when executing `let z = y`, the refinement conditions already attached to `y` are
  transferred to `z`

All assumptions enter the compile-time proof pipeline. When execution enters the SMT-acceleration
path, they are translated into SMT-LIB background assertions.

#### 3.4 A Compilation Error When No Static Evidence Exists

If the programmer directly writes:

```yaoxiang
divide_user_input: (x: Int, y: Int) -> Int = divide(x, y)
```

The current program point has no `y > 0` assumption, and the actual argument `y` itself has no
`Positive` type annotation. The verification condition is:

```
{} ⇒ { y > 0 }
```

The pipeline returns `Disproved` (the implication does not hold) → compilation error:

> Unable to prove that argument `b` satisfies `Positive` in the `divide` call. `y` comes from
> function input and has no proven bound. Consider guarding the call with an if branch:
> `if y > 0 { divide(x, y) }`.

YaoXiang does not permit a runtime value to enter a refinement-type argument without static
evidence. This is not a limitation—it is central to the hard-safety philosophy. Code that the
compiler cannot statically prove must not compile.

#### 3.5 Relationship to the Unified Pipeline

Path-condition propagation is not an additional mechanism. It is a direct extension of the
compile-time proof pipeline into control-flow analysis:

| Stage                             | Responsibility                                                                                                                                                                |
| --------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Path-condition collection         | During control-flow analysis, the compiler annotates each basic block with its assumption set                                                                                 |
| Verification-condition generation | When it encounters a type constraint that must be validated, it combines path conditions with actual-argument type information                                                |
| Proof-pipeline evaluation         | Compiler kernel → SMT acceleration → derives Proved / Disproved / Unproven                                                                                                    |
| Outcome                           | `Proved` → pass; `Disproved` → compilation error plus counterexample; `Unproven` → compilation error plus unresolved proposition (the programmer may supply a proof function) |

There are no new components and no special rules. Path conditions are background knowledge for the
proof pipeline, sharing the same pipeline and budget system as type equalities and borrow
constraints.

### 4. The Compile-Time Proof Pipeline

All compile-time checks share one pipeline. Its core operation is **type checking**—checking whether
the type of a proof term is equal to the proposition being proved. Everything is type checking.

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

#### 4.1 Proof Outcomes: A Three-Valued Algebra

Compile-time evaluation returns three outcomes—an inevitable consequence of the halting problem and
the natural partition of proof theory:

```
eval_compile_time : BoolExpr → Proved | Disproved(Model) | Unproven
```

- **Proved** → halts; the proof term has been constructed, and type checking passes. Compilation
  continues.
- **Disproved(M)** → halts; a counterexample M exists. Compilation error plus counterexample plus
  source location.
- **Unproven** → no proof could be constructed within the given resource limit. Compilation error
  plus unresolved proposition plus budget-consumption report.

**Unproven ≠ False.** When the compiler says “I cannot prove it,” that is not equivalent to the
proposition being false—it merely exceeds the compiler's current automatic-proof capability. This is
honesty, not a defect.

A hard budget limit is the engineering answer to the halting problem. No tuning knob is
provided—offering one would amount to asking the user, “Do you think your program will terminate?”
The user does not know, and neither does the compiler.

#### 4.2 After Unproven: The Programmer Writes the Proof

When the compiler returns Unproven, the programmer can write a **proof function**—a YaoXiang
function whose return type is equal to the proposition being proved. The type checker verifies this
function in exactly the same way that it verifies `add(a, b): Int`.

```
命题  = 类型
证明  = 程序（该类型的一个值）
验证  = 类型检查（唯一的信任根）
```

The SMT solver is not an independent trust boundary—it is an **acceleration module for the type
checker**. SMT helps find proofs, but the type checker always verifies them. When SMT returns
`unsat`, the compiler reconstructs the result as a proof term that the type checker can verify. If
reconstruction fails (the SMT reasoning steps exceed the compiler kernel's inference rules), it
falls back to Unproven, and the programmer can write a proof function manually.

```yaoxiang
# Proposition: a refinement property that the compiler cannot prove automatically
FirstIsMin: (T: Ord, arr: Sorted(T)) -> Type = {
    forall i in 0..arr.len: arr[0] <= arr[i]
}

# Proof: the programmer writes a function whose return type is the proposition above
# The type checker verifies this function—exactly as it verifies add(a,b): Int
first_is_min: (T: Ord, arr: Sorted(T)) -> FirstIsMin(T, arr) = {
    # The compiler validates here: the function body's type = FirstIsMin(T, arr)
    ...
}
```

No AI is needed, no export to Coq is needed, and no new concept is needed. **A property that cannot
be proved automatically at compile time → the programmer writes the proof in YaoXiang code → the
type checker verifies it.** The entire process is a smooth gradient—the compiler handles the easy
proofs and leaves the hard ones to you.

#### 4.3 Layered Dependencies Within the Pipeline

The evaluators above share the same interface but have an evaluation order. Type equality is a
prerequisite for all subsequent analyses; ownership/token checking depends on type information; and
refinement-predicate validation depends on the results of the first two layers. The compiler
evaluates layer by layer, and an expression that fails at a lower layer does not proceed to an upper
layer, avoiding wasted solver budget on programs that already contain type errors.

```
求值顺序（同一管道，分层调度）
├── 第 0 层：类型等式（T1 == T2）
│   └── 结构合一 → 失败则后续无意义，直接返回 Disproved
├── 第 1 层：所有权/令牌冲突
│   └── 流敏感活性分析 → 失败则内存安全不成立，直接返回 Disproved
└── 第 2 层：精化谓词/霍尔蕴含
    └── 编译器自身 → SMT 加速 → 得出 Proved / Disproved / Unproven
```

Each layer still returns `Proved/Disproved/Unproven`, sharing the same interface and budget system.

### 5. Unification of the Three Kinds of Function

| Layer                  | Execution Time | Input      | Output | Example                                        |
| ---------------------- | -------------- | ---------- | ------ | ---------------------------------------------- |
| Value-level function   | Runtime        | Value      | Value  | `add: (a: Int, b: Int) -> Int = a + b`         |
| Type constructor       | Compile time   | Type/value | Type   | `List: (T: Type) -> Type = { data: Array(T) }` |
| Compile-time predicate | Compile time   | Value      | Type   | `Positive: (x: Int) -> Type = { x > 0 }`       |

All use the same `name: type = value` syntax. Compile-time predicates and type constructors follow
the same compile-time proof pipeline—`{}` is the proof space.

### 6. Loops: Floyd–Hoare Verification-Condition Generation

Loops do not need separate `: Invariant(...)` or `: decreases(...)` annotations. A
compile-time-predicate type annotation on a variable defines a Floyd–Hoare-style assertion—the
compiler generates verification conditions from the type annotation, and the proof pipeline checks
whether each assignment preserves the type.

The core mechanism is that each assignment corresponds to a Hoare triple `{P} x := e {Q}`, whose
verification condition is `P ⇒ Q[e/x]`. The compiler generates one verification condition for the
loop body—once the proof pipeline establishes the inductive step, all iterations are covered
automatically.

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # The annotation refers to i; during initialization i=0; validation: 0 == sum(arr[0..0]) → True
    mut i: UpTo(arr.len) = 0     # Validation: 0 <= 0 <= arr.len → True
    while i < arr.len {
        # The compiler generates one VC for the loop body. Preconditions: s satisfies SumUpTo(arr, i), and i satisfies UpTo(arr.len).
        #
        # s += arr[i]:
        #   Verification obligation: s_new satisfies SumUpTo(arr, i) (the current i is unchanged)
        #   Substitute s_new = s_old + arr[i]:
        #     Requires s_old + arr[i] == sum(arr[0..i+1])
        #     From the inductive hypothesis s_old == sum(arr[0..i]), add arr[i] to both sides:
        #     sum(arr[0..i]) + arr[i] == sum(arr[0..i+1])
        #   Compiler + SMT: linear arithmetic, milliseconds → Proved
        #
        # i += 1:
        #   i changes → the type annotation on s refers to i in the dependency graph → triggers revalidation
        #   New verification target: s satisfies SumUpTo(arr, i_new)
        #   That is, s == sum(arr[0..i_new]), guaranteed by the previous step → Proved
        s += arr[i]
        i += 1
    }
    return s  # At this point s: SumUpTo(arr, arr.len), that is, s == sum(arr[0..arr.len])
}
```

The loop invariant is the type annotation on the variable—the programmer writes the type, and the
compiler checks the inductive step. The compiler does not need to “discover” the invariant or
“automatically perform induction”; it decomposes the inductive proof into a local verification
condition for each assignment operation and hands it to the proof pipeline to solve by divide and
conquer.

#### 6.1 Dependency Tracking: Dependent Types on Mutable Variables

The mechanism above depends on the compiler knowing that the type annotation `SumUpTo(arr, i)` on
`s` refers to `i`—the type constraint on `s` changes when `i` changes. This requires the compiler to
maintain a **type-dependency graph between variables**.

**Data structure**:

```
TypeDepGraph: Map<VarName, Set<VarName>>
# The key is the depended-on variable; the value is the set of variables whose type annotations refer to it
# Example: { i: {s}, j: {s, t}, ... }
```

**Construction**: when the type checker processes `mut v: Pred(... x ...) = init`, it resolves
free-variable references in the arguments of `Pred(...)`. If an argument refers to another mutable
variable `x` in the current scope, the dependency graph records `x → v`.

**Trigger**: when a depended-on variable `x` is assigned, the compiler:

1. Finds all variables in the dependency graph that depend on `x`: `{v₁, v₂, ...}`
2. For each `v`, generates the verification condition: does the current value of `v` satisfy the
   updated type `Pred(... x_new ...)`?
3. Sends the VC to the proof pipeline

**Assignment-order sensitivity**: dependency tracking naturally enforces the correct assignment
order. Using `SumUpTo(arr, i)` as an example:

```yaoxiang
# Correct order
s += arr[i]   # s_new satisfies SumUpTo(arr, i+1)
i += 1        # i changes → revalidate that s satisfies SumUpTo(arr, i_new) → True

# Wrong order—the compiler rejects it
i += 1        # i changes → revalidate that s satisfies SumUpTo(arr, i_new)
              # s has not yet been updated, so s_old == sum(arr[0..i_old]) ≠ sum(arr[0..i_new])
              # → Compilation error: variable s does not satisfy type SumUpTo(arr, i_new)
s += arr[i]   # Unreachable
```

**Combined dependencies**: a variable may depend on multiple variables. The type annotation
`{ v: Int; v == x + y }` depends on both `x` and `y`—a change to either triggers revalidation.

**Relationship to the proof pipeline**: dependency tracking is a trigger for VC generation, not an
independent verification mechanism. It answers “when must a VC be generated?”—the proof pipeline
answers “does the VC hold?”

### 7. Termination Checking

**Scope: refinement types.** Termination is not an independent switch; it is part of a
**verification mode**: once a type is refined (`Refined { base, constraint }`), the computation it
annotates enters verification mode, and termination is required within that mode. Ordinary types
that are not refined do not enter verification mode and generate no termination obligations.

This yields two corollaries:

- **Loops**: a bare `while` does not enter verification mode; a measure variable with a refinement
  annotation (for example, `i: UpTo(n)`) does enter verification mode and must be proved
  terminating.
- **Recursion**: when a function signature contains a refinement (a refined parameter or a refined
  return type), it enters verification mode, and the measure must be proved to decrease strictly at
  every recursive call site.

Full automation takes priority within the mode: the compiler first explores measures automatically
and accepts a loop when it can prove termination. If exploration fails and no explicit measure was
provided, compilation fails. There is no **annotation-syntax** escape hatch—the measure and
termination proposition are written in the type position, introducing no new syntax such as
`decreases`. See §6.9 for the measure and explicit fallback forms.

#### 6.1 Design Principles

The compiler automatically extracts the information needed for a termination proof from two places:

1. **Variable type annotations**: boundary constraints in refinement types (for example, `UpTo(n)`
   supplies the upper bound `n` and lower bound `0`)
2. **Loop-body operations**: the operation applied to each variable on each iteration

The compiler tries four measure-synthesis strategies in priority order and stops as soon as it finds
one. The four strategies are a **restricted template sequence for measure exploration**. Their input
is refined constraints (strategies 1–4 all start with a “variable of a bounded type”), not “code
evaluated at compile time”; they are the automatic and manual sides of the same mechanism as the
explicit measure in §6.9.

> **Measure exploration is exploration, not inference.** Exploration only enumerates templates
> (linear rank, violation count, bounded patterns, multiplicative scaling) and does not guarantee
> that a solution exists—general measure inference is undecidable (it reduces to the halting
> problem). Cases outside the templates must therefore be supplied explicitly by the programmer
> (§6.9), or compilation fails.

#### 6.2 Strategy 1: Automatic Synthesis of Linear Rank Functions

When a variable has a linear bound annotation, the compiler enumerates candidate linear measures and
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
bounded type annotation. This includes `i += const`, `i -= const`, and interval-contraction patterns
such as binary search:

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

#### 6.3 Strategy 2: Predicate Violation Count—Automatically Extracting the Measure from the Target Type <span style="color:orange">[Experimental Strategy]</span>

> ⚠️ **Current status: experimental strategy; during Phase 3 implementation, decide whether to
> include it based on practical feasibility.** This strategy works for adjacent-swap operations
> (bubble sort, insertion sort), but cannot automatically prove termination for non-adjacent
> operations (quicksort partition, heapsort sift-down). See the table below for its coverage
> boundary. If Phase 3 validation shows that it is infeasible, this strategy will be removed or
> downgraded to future work.

The central insight is: **the specifications written by the user are material for compiler
reasoning.** The compiler does not need to have “what sorting means” built in—it reads the
definition of `Sorted` and automatically extracts a measure from it.

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

| Algorithm      | Operation Pattern  | Provable by Strategy 2? | Reason                                                 |
| -------------- | ------------------ | :---------------------: | ------------------------------------------------------ |
| Bubble sort    | Adjacent swaps     |           ✅            | violation_count strictly decreases with each swap      |
| Insertion sort | Adjacent movement  |           ✅            | Each shift eliminates one violating pair               |
| Selection sort | Non-adjacent swaps |           ❌            | A single swap may increase violation_count             |
| Quicksort      | partition          |           ❌            | Non-adjacent swaps do not guarantee monotonic decrease |
| Heapsort       | sift-down          |           ❌            | Tree operations make violation_count non-monotonic     |

**Complementary strategies**: for quicksort, contraction of the `low < high` interval can be covered
by Strategy 1 (linear rank function)—the outer partition recursion halves the interval on each step.
Strategies 1 and 2 provide complementary coverage, and most real algorithms can be proved
terminating by one or the other. However, generalizing Strategy 2 to non-adjacent and tree-shaped
operations remains an open problem.

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

`v += const` (a positive constant), with a variable carrying an upper-bound type annotation → the
measure `upper_bound - v` decreases by `const` each iteration and has a lower bound of 0. This is a
degenerate case of Strategy 1, which the compiler handles first for speed.

#### 6.5 Strategy 4: Multiplicative-Scaling Measure Template

`v *= const` (`const > 1`), with a variable carrying upper- and lower-bound type annotations. The
compiler includes the logarithmic measure template `ceil(log_const(upper/v))`, which decreases by 1
each time the value is multiplied by `const`.

```yaoxiang
mut i: Positive(i) = 1
while i < n {
    # The compiler automatically derives: measure ceil(log₂(n/i)); multiplying by 2 decreases the measure by 1
    i *= 2
}
```

#### 6.6 Separation of Termination and Correctness

Termination proof and correctness proof are independent:

- **Termination**: the four strategies above automatically prove that a loop exits in finitely many
  steps; when exploration fails, the programmer supplies a measure in the type position (§6.9)
- **Correctness**: whether the loop body makes progress toward the target type is checked through
  verification conditions in the compile-time proof pipeline

If both pass, compilation succeeds. If termination is proved but correctness fails → compilation
error plus counterexample. If correctness is proved but termination cannot be proved → compilation
error identifying the variable or operation that could not be analyzed. If both fail → compilation
error reporting the two failure causes separately.

#### 6.7 Termination Checking for Recursive Functions

For recursive functions with refined signatures, the compiler checks that formal parameters decrease
at every recursive call site:

```yaoxiang
gcd: (a: Int, b: NonNegative(b)) -> Int = {
    if b == 0 { return a }
    return gcd(b, a % b)  // The compiler explores: the well-founded-order measure for (b, a % b) decreases → termination
}
```

Decreasing formal parameters are the strongest automatic path (structural recursion). When
exploration fails, the programmer explicitly supplies a measure in the type position (§6.9).

#### 6.8 Hard Boundary

`i = f(i)`, where `f` is not invertible, is not closed, and preserves no monotonicity—automatic
termination proof is mathematically impossible. Compilation fails:

> This loop cannot be automatically proved to terminate. The loop variable depends on the
> unanalyzable function `f`. Use an iteration pattern that the compiler can analyze, or bind a name
> to the loop and provide a measure in the type position (§6.9).

This is not a compiler failure. Code that cannot be statically proved safe must not compile. Even
when an explicit measure is supplied, it must still be proved true by SMT before it is accepted—an
incorrect measure is rejected with a counterexample. A human may be unable to prove a property, but
cannot prove it incorrectly.

#### 6.9 Explicit Measure: `Terminates`

When automatic exploration cannot find a measure, the programmer writes the measure in the **type
position**—through the same mechanism as `Positive(b)` and `IsMax(T, arr, result)` (predicate
application), with no new syntax:

```yaoxiang
// Measure: an ordinary function; unit-testable, reusable, and not used at runtime
gcd_measure: (a: Int, b: Int) -> Int = { b }

// The termination component is embedded in the function's own type position
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// Loop: the binding name is the anchor, and the measure obtains values in scope by name
loop: (n: Int) -> Int = {
    mut i = 0
    acc: Terminates(n - i) = while i < n {
        i = i + 1
        i
    }
    return acc
}
```

The rationale for the loop form is that `Terminates(m)` refines the value type of the loop body's
tail expression, and the binding name `acc` provides the anchor. The loop can therefore be referred
to, eliminating the “an anonymous construction cannot be referred to” corner case.

> **The body tail must provide a value**: the value of a loop = the value of its body block (§2.9),
> and the value of a block = its tail expression. If the body tail is an assignment statement (for
> example, `{ i = i + 1 }`), the block value is `Void`, and `return acc` returns `Void` from
> `-> Int`, which is ill-typed. The example therefore explicitly writes `i` as the body tail. This
> does not change the semantics of the measure—the measure `n - i` is a **state expression**,
> independent of the loop's value type.

> **Implementation note (recorded 2026-10-02; updated 2026-10-04)**: the **type side** of this
> section's loop example is valid (`check` reports 0 errors; the back-edge obligation is Proved).
> The **runtime-value defect (#409) has been fixed**: a `while` used as a value now takes the value
> of the body block's tail expression from the last iteration, consistent with the type-side
> `block_value_ty`— `loop(4) == 4` and `countdown(3) == 30` now hold at runtime (`for` received the
> same fix).
>
> **Known residual issue (zero iterations)**: when the loop body is never evaluated, there is no
> “body value from the last iteration,” so the loop value is still `Void`. Binding it to a
> non-`Void` type position and using it at runtime reports
> `E6007 Runtime error: type mismatch in comparison Eq: Void vs Int(0)` (a loud type mismatch, not a
> silently incorrect value). This is a **static-approximation boundary**: the `block_value_ty`
> criterion determines the type of the body-tail expression, but cannot determine whether the loop
> executes at least once (which would require data-flow/provability analysis), so the type side does
> not silently downgrade it (doing so would break the value type required by this section's
> `Terminates` loop binding). This residual behavior is pinned by the corpus entry
> `tests/yaoxiang/02-type-system/termination/while_zero_iteration_void_err.yx`
> (`// expect: runtime-error E6007`). **The residual behavior has been confirmed as an intentionally
> retained design boundary** (decision of 2026-10-04; not currently being fixed), so issue
> [#409](https://github.com/ChenXu233/YaoXiang/issues/409) was closed alongside the fix to the main
> form and will not be tracked separately. See the historical record of the minimal reproduction and
> affected scope in [RFC-027a](../accepted/027a-termination-explicit-measure.md) §Example “Loops,”
> “Known Defect D6.” In addition, termination Strategy 1 (linear rank function) has been restored in
> this pass, so the ordering in this section—“write an explicit measure only when automatic
> exploration fails”—remains unchanged: the explicit measure is still the fallback after exploration
> fails.

**`Terminates` is a built-in predicate**, belonging to the same set of core primitives as `Int` and
`Never`. It is the only predicate whose **function body is generated by the compiler**. Its
assertion (“the measure strictly decreases at every recursive call site/loop back edge”) is embedded
in the computation structure, and user-written predicates cannot refer to the function body or loop
body, so it cannot be expressed using the predicate-definition syntax described in the “Syntax”
section. The built-in surface is therefore limited to this single name.

**Arity**. `Terminates(FnType, m)` and `Terminates(m)` are two arities of the same predicate, not
two separate constructs:

| Form                    | Anchor                      | Use                                                           |
| ----------------------- | --------------------------- | ------------------------------------------------------------- |
| `Terminates(m)`         | Name of the current binding | Self-recursive functions and loops—the default form           |
| `Terminates(FnType, m)` | Explicit function type      | Cases such as mutual recursion where the anchor is not unique |

The two are essentially the same: the termination obligation always applies to “the computation
annotated by the type position in which the refinement resides.”

**The measure does not constrain the return type.** The measure may be an expression of any type; it
is not required to be a natural number. Its “strict decrease” is determined by the well-founded
order available for that type. Whether the measure is well-founded (for example, whether an `Int` is
`>= 0`) is an **independent obligation**, assigned to refinement inference or SMT just like the
decrease obligation. When neither can be inferred, the diagnostic **does not directly reject the
code**; instead, it suggests checking the direction (whether the measure's lower bound holds and
whether recursive arguments actually move in that direction).

**Relationship to automatic exploration**: the explicit measure is not another pipeline; it is input
supplied after exploration fails. Once a measure is provided, it still follows the same SMT pipeline
to validate decrease and well-foundedness. If it does not hold, the compiler reports an error with a
counterexample.

The shared implementation mechanisms for obligation generation, the decision pipeline, diagnostic
directions, and measures for mutual recursion (SCC) are described in
[RFC-027a: Explicit Measure for Termination Checking](../accepted/027a-termination-explicit-measure.md).

### 8. The SMT Solver: An Acceleration Module for the Type Checker

In traditional languages, the SMT solver is an external tool (for example, F\* invokes Z3, and Dafny
invokes Z3). In YaoXiang, it is **an acceleration module within the type checker** and is called
only when the compiler kernel cannot directly decide the obligation. SMT helps find proofs, but the
type checker verifies them.

**Trust model**: the type checker is the sole trust root. The SMT solver is an acceleration
module—it helps find proofs, but it is not an independent trust boundary. The compiler trusts Z3's
`unsat` result (consistent with the F\*/Dafny approach—the practical engineering choice that Z3 has
a lower error probability than the compiler itself has a bug rate). The real unreliability is
confined to the SMT translation layer—if the translation has a bug, other tests will expose it.

**Interface**: internally, the compiler translates to the SMT-LIB 2.6 standard format rather than
binding to a solver-specific API. SMT-LIB is an ISO standard supported natively by Z3, CVC5,
MathSAT, and Yices.

**Default backend**: Z3 (MIT license, the broadest documentation and community validation). CVC5 is
an SMT-LIB-compatible alternative that users can select through a compiler flag.

There is no “generic solver abstraction layer”—SMT-LIB is the abstraction layer. If CVC5 later makes
progress in a particular theory, switching backends requires only replacing the binary, not changing
compiler code.

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

| Budget Dimension               | Default | Description                                                                                                                                                                          |
| ------------------------------ | ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Solver steps                   | 10,000  | Z3 usually handles linear arithmetic in fewer than 100 steps. 10,000 steps cover 99% of real predicates.                                                                             |
| Time                           | 100ms   | A single predicate taking more than 100ms means the user is writing a compile-time program rather than a type annotation. 100ms × 50 predicates = a 5-second compilation-time limit. |
| Quantifier-instantiation depth | 3       | Three levels of nested quantifiers cover real-world patterns. Beyond three levels, the user is most likely writing a logic exercise.                                                 |

Exceeding the budget returns Unproven, a compilation error, the predicate location, and the amount
consumed. There is no downgrade, no runtime check, and no silent pass.

**Why this is practical**: 95% of real predicates are linear arithmetic—`x > 0`, `arr.len > 0`, and
`0 <= idx < arr.len`—all within a decidable fragment. SMT solvers return a result for such problems
in milliseconds. For the rare complex predicate that exceeds the budget, the programmer can write a
proof function.

Dependent types receive one layer of pre-reduction before SMT is invoked: `factorial(5)` is
evaluated directly at compile time to `120`, and `append([1,2], [3])` is evaluated directly to
`[1,2,3]`. These deterministic value computations do not consume SMT budget.

Programmers do not need to know that SMT exists. The mental model is: **the compiler passes the
proof if it can prove it and reports an error if it cannot—if the compiler cannot prove it, you can
write a function that shows it the proof.**

### 9. Composition of Compile-Time Predicates

Compile-time predicates are functions returning Type, and composition is implemented naturally
through function composition:

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

result = divide(10, 2)   # ✅ The compiler validates Positive(2) = { 2 > 0 } → True
# result = divide(10, 0)  # ❌ The compiler validates Positive(0) = { 0 > 0 } → False
```

#### 9.2 Safe Array Access

```yaoxiang
InBounds: (idx: Int, arr: Array(T)) -> Type = { 0 <= idx and idx < arr.len }

get: (arr: Array(T), idx: InBounds(idx, arr)) -> T = arr.data[idx]

arr = Array(Int)(1, 2, 3)
x = get(arr, 1)   # ✅ The compiler validates InBounds(1, arr) = { 0 <= 1 and 1 < 3 } → True
# y = get(arr, 5)  # ❌ The compiler validates InBounds(5, arr) = { 0 <= 5 and 5 < 3 } → False
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

#### 9.4 Loops: Compiler-Generated VCs

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

### 11. The dispatch Dispatch Pipeline: Unified Compile-Time and Runtime Dispatch

> **Implementation note (recorded 2026-10-03, #377-2)**: the **semantics** of this section are valid
> and integrated into production, but the **structure** is not a standalone `layers/dispatch.rs`
> module. Production dispatch is implemented through three **call-site obligations** in
> `checker.rs`—binding-position revalidation (`revalidate_refined`), call-site arguments
> (`check_call_arg_refinements`), and the return-position postcondition (`check_return_refinement`).
> Of these, there are **two direct calls** to `layers::predicate::check_predicate`: the
> call-site-argument and return-position-postcondition paths share the sole decision unit
> `judge_refined_obligation` (`checker/refinement.rs`), while `revalidate_refined` has its own E2030
> semantics. The original `layers/dispatch.rs` had zero production call sites, and its handling of
> `Unproven { ProofFunctionRequired }` was to “downgrade it to a W1080 warning and inject Γ,” which
> conflicted with the production implementation (`Unproven` always reports an error, as in
> `refined_unproven`), so it was removed in its entirety. Also note that the “insert runtime check”
> behavior in the Runtime row of the table below currently has **no** emission point in production
> (`RuntimeOutcome::InsertCheck` has no consumer); Γ injection is independently handled by the
> branch guard in `ownership.rs` and `inference/expressions.rs`.

`assert` and `Assert` are two sides of the same refinement-type primitive. The dispatch pipeline
`dispatch` automatically chooses between compile-time proof and runtime checking according to
**whether the predicate's free variables are accessible at compile time**:

| Criterion                                                                                  | Mode            | Behavior                                                                                          |
| ------------------------------------------------------------------------------------------ | --------------- | ------------------------------------------------------------------------------------------------- |
| All free variables are known at compile time (generic parameters, compile-time constants)  | **CompileTime** | Enter the proof pipeline: Proved → erase; Disproved → compilation error; Unknown → proof required |
| Some free variables come from runtime (function parameters, external input, mut variables) | **Runtime**     | Insert a runtime check and inject the refinement fact into the flow-sensitive assumption set Γ    |

**Key point**: “cannot decide” ≠ “disproved.” In CompileTime mode, Unknown requires a proof and is
never silently downgraded. In Runtime mode, the proposition has no truth value at compile time in
the first place—no matter how strong the prover is, it cannot write an always-true proof for “the
user might have entered a negative number.” A runtime check is the only sound choice. This is not a
weakness of the prover; it is a theoretical necessity.

### 12. Flow-Sensitive Assumption Set Γ: Strongest-Postcondition Propagation

The compiler maintains a flow-sensitive assumption set Γ that tracks the propositions known to hold
at each control-flow point.

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

This is a hard soundness requirement—when a variable's value changes, old assumptions no longer
hold.

**Branch merging**: when IF/ELSE or match branches are merged, Γ takes the intersection of the
branch assumptions. Only propositions that hold on every path are carried out of the branch.

**Equality bridge (automatic evidence for q=0 slots)**: constant-equality facts in Γ are a legal
source of evidence for compile-time value arguments (RFC-011 §4.1, the q=0 slot). Within the guard
branch of `if y == 42 { f(y)(y) }`, Γ records `y == 42`, meaning that y is treated as 42 in the q=0
slot, and the call is checked along the existing `f(42)` path. One side of the equality must be a
compile-time constant; a runtime value without equality evidence still produces a report at the call
site. This is not a new mechanism—the verification condition `{y == 42} ⇒ {y ≡ 42}` follows the
existing entailment decision procedure.

### 13. Clarification of the Erasure Model: Erasing Witnesses ≠ Erasing Checks

RFC-027's claim that “refinement types are **completely erased** at runtime” refers to the **proof
witness**—a proof term verified at compile time does not generate runtime code. The **runtime
check** inserted by dispatch in Runtime mode is retained—it is a Bool check executed over values,
not a type-level witness.

In summary: witnesses are erased; checks are retained. The two are not in conflict, and RFC-027's
original claim remains unchanged.

## Detailed Design

### Syntax Changes

| Before (RFC-022)                      | After (this RFC)                                                                                                                                                            |
| ------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `//! requires: NonEmpty(n) = n > 0`   | Compile-time predicate as the parameter type `(b: Positive(b))`                                                                                                             |
| `//! ensures: ExistsMax(result, arr)` | Return type uses a return-value formal parameter `-> (result: IsMax(T, arr, result))`                                                                                       |
| `/*! invariant: ... !*/`              | Compile-time-predicate type annotation on a variable—a Floyd–Hoare invariant                                                                                                |
| `//! decreases: n`                    | The measure is written in the refinement-type position (`Terminates`); the compiler explores automatically first and requires an explicit measure only if exploration fails |
| Specifications are comments           | Specifications are part of the type system                                                                                                                                  |

### Syntax

**Compile-time predicates introduce no new keywords.** `{}` is the proof space, exactly consistent
with the existing type-definition syntax. A compile-time predicate is simply a function returning
Type—`name: (params) -> Type = { assertions }`. It is used as a function call—`Positive(b)`,
`IsMax(T, arr, result)`.

```bnf
# A compile-time predicate = a function returning Type, with assertions inside {} that the compiler validates
# Uses the existing function/type syntax and requires no new BNF rule
predicate ::= identifier ':' params '->' 'Type' '=' '{' assertions '}'
```

**The arguments of a predicate application must have compile-time-constant form**—a literal, a
variable (bound by name), a type application (recursively extracted), or a **compile-time-referable
function reference** (a function name). If an argument cannot be converted to a constant expression,
the compiler reports **E1092**. If the number of arguments does not match the number of formal
parameters in the predicate declaration, it reports **E1093**. Arguments are bound positionally to
the formal-parameter list, and predicate arity is determined by the declaration—`Positive(x)` has
one argument, `IsMax(T, arr, result)` has three, and `Terminates(m)` and `Terminates(FnType, m)`
have one and two arguments, respectively. Refinement constraints are **never silently discarded**
(previously, an unconvertible argument could cause the constraint to disappear silently, allowing a
violating binding to pass silently).

**New syntactic concept: return-value formal parameter**—in `-> (name: Type)`, `name` is the
return-value formal parameter.

The return-value formal parameter is the **only syntactic concept** that YaoXiang introduces on top
of its existing function syntax. Its semantics are:

- The value of `name` is supplied by a `return` statement
- `name` exists only in the type signature and is referenced by postcondition predicates (for
  example, `-> (result: IsMax(T, arr, result))`)
- `name` does not enter the function-body scope and does not appear at call sites
- The return-value formal parameter is **optional**—when there is no postcondition, the signature is
  identical to that of an ordinary function (`-> Int`), with no additional burden

The reason for introducing it is that a postcondition needs to refer to “the value the function is
about to return.” Without a return-value formal parameter, the compiler would have to rely on
special rules—such as the implicit variable `$result` or `__retval__`—to let predicates refer to the
return value. A return-value formal parameter makes this reference explicit—it is simply a formal
parameter whose value is supplied by `return` rather than by the caller.

A **proof function** is not a new concept—it is a YaoXiang function whose return type is the
asserted proposition. When the compiler returns Unproven, the programmer supplies a proof function,
and the type checker verifies it in exactly the same way that it verifies the return type of any
function. No new syntax, keywords, or rules are required.

> **The boundary between the two.** In the correctness domain, a proof is supplied in the **body**
> for an unproved predicate: write a function whose return type is the proposition being proved. In
> the termination domain (§6.9), a measure is supplied in the **type position**: write
> `Terminates(measure)` as the type of a binding or function signature. The former means “write a
> proof for a proposition that cannot be proved,” while the latter means “explicitly declare a
> measure that exploration cannot find.” The mechanism is the same in both cases (a refinement-type
> application), but the location differs. The termination domain does **not** require writing a
> `_proof` function.

### Type-System Implications

- **Type universe**: compile-time predicates reside at the Type₂ layer—they accept functions
  returning values of type Type and occupy the same level as type constructors
- **Generics interaction**: compile-time predicates may take generic parameters, for example
  `NonEmpty: (T: Type) -> (arr: Array(T)) -> Type`
- **Ownership interaction**: expressions in compile-time predicates obey ownership rules and may
  only be read, not written
- **Type inference**: parameters of compile-time predicates participate in HM type inference

### Runtime Representation

Compile-time predicates are handled at runtime **according to the dispatch outcome**:

- **CompileTime mode** (all free variables are known at compile time): after the proof succeeds, the
  witness token is completely erased. For `Positive: (x: Int) -> Type = { x > 0 }`, the parameter
  `b: Positive(5)` has the runtime representation `Int`. The refinement condition `{ 5 > 0 }` has
  already passed and is erased.
- **Runtime mode** (some free variables are runtime values): retain the runtime check—a Bool check
  executed over values—and inject the result into the flow-sensitive assumption set Γ. See the
  dispatch pipeline in §11 and the erasure-model clarification in §13.

Placing a compile-time predicate in a type position (for example, `f(x: Positive(x))`) does not
create a wrapper type or allocate additional memory. However, when `x` comes from runtime input, a
runtime Bool check **is** inserted.

**Constraint on interaction with `ref`**: a compile-time predicate may refer only to an immutable
borrow or a value whose ownership has been transferred. The compiler cannot guarantee at compile
time that the validation result will still hold at runtime for a predicate that refers to a mutable
borrow—therefore, such usage immediately produces a compilation error.

### Compiler Changes

1. **Parser**: compile-time predicates use standard function syntax and require no additional
   parsing rules
2. **Compile-time proof pipeline**: a unified Proved/Disproved/Unproven return interface with
   automatic strategy selection
3. **SMT acceleration module**: an SMT-LIB 2.6 translation layer, with Z3 as the default backend and
   CVC5 as an alternative
4. **Type-checker kernel**: inference-rule implementation—structural equivalence, βδι-reduction, and
   universal-quantifier introduction/elimination. This is the sole trust root; both SMT results and
   programmer proofs are verified through it
5. **Verification-condition generation**: WP/SP calculi plus loop-invariant proof obligations
6. **Error reporting**: counterexample formatting, unresolved-proposition reports, and
   source-location correlation

### Backward Compatibility

- ✅ Code that does not use compile-time predicates remains completely unchanged
- ✅ Compile-time predicates have zero runtime overhead in CompileTime mode; Runtime mode retains
  only the necessary Bool checks
- ⚠️ RFC-022's `//!` syntax is no longer supported—but 022 was never implemented, so there is no
  migration burden

## Trade-offs

### Advantages

- **Complete realization of the Curry–Howard correspondence**: types are propositions, programs are
  proofs, and `name: Proposition = Proof`
- **Uniformity**: compile-time predicates use exactly the same syntax as ordinary functions, with no
  conceptual split
- **SMT transparency**: programmers do not need to know that SMT exists, and the mental model
  remains consistent with type checking
- **Incremental adoption**: start with one compile-time predicate and gradually expand coverage
- **Minimal runtime overhead**: zero overhead in CompileTime mode; Runtime mode retains only the
  necessary Bool checks

### Disadvantages

- **Compilation time**: SMT solving increases compilation time, but hard budget limits ensure that
  the upper bound remains controllable
- **Automatic-proof boundary**: complex predicates beyond first-order linear arithmetic may require
  a programmer-written proof function. This is not a language defect—it is an inevitable consequence
  of the halting problem. The compiler honestly reports Unproven instead of falsely reporting
  True/False
- **Learning curve**: writing effective compile-time predicates and proof functions requires
  understanding the basic intuition behind the Curry–Howard correspondence
- **Implementation complexity**: unifying the compile-time proof pipeline requires careful design

### Risk Mitigation

- Hard solver-budget limits (10,000 steps / 100ms / instantiation depth 3); exceeding the budget
  returns Unproven
- Dependent-type pre-reduction: deterministic value computations are handled first, and SMT only
  handles the nondeterministic remainder
- Unproven is not a dead end: in the correctness domain, write a proof function (whose return type
  is the proposition); in the termination domain, provide a measure in the type position (§6.9)—both
  are verified by the type checker
- Incremental validation: validate only changed modules
- Clear error messages + counterexample display + budget-consumption report + unresolved
  proposition + suggestions (when the compiler can provide them)

## Alternatives

| Option                                                             | Why It Was Not Chosen                                                                                                                                                                    |
| ------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| RFC-022: comment-based `//!` specifications                        | Splits specifications from types, violating the Curry–Howard correspondence                                                                                                              |
| Separate specification files (such as CVL)                         | Separates specifications from code and increases maintenance cost                                                                                                                        |
| Runtime assertions only                                            | Cannot provide static correctness guarantees                                                                                                                                             |
| External proof assistants (such as Coq)                            | Disconnects proofs from the compiler and requires a separate proof language and trust boundary. YaoXiang's choice: proofs are YaoXiang code, and the type checker is the sole trust root |
| **This proposal: compile-time predicates as first-class citizens** | ✅                                                                                                                                                                                       |

## Implementation Strategy

### Phase Breakdown

| Phase       | Content                                                                                                                                                                 |
| ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Phase 1** | Compiler kernel: structural equivalence + βδι-reduction + universal-quantifier introduction/elimination. Supports simple arithmetic predicates (`x > 0`, `arr.len > 0`) |
| **Phase 2** | SMT-LIB translation layer + Z3/CVC5 integration. The pipeline returns Proved/Disproved/Unproven. Supports programmer-written proof functions for Unproven outcomes      |
| **Phase 3** | Loop-invariant VC generation + termination checking (four measure-exploration strategies + the explicit `Terminates` measure, §6 and §6.9)                              |
| **Phase 4** | Incremental validation + caching + IDE support                                                                                                                          |

### Dependencies

- RFC-010: Unified Type Syntax—compile-time predicates are based on `name: type = value`
- RFC-011: Generic Type System—compile-time predicates may take generic parameters
- RFC-009: Ownership Model—expressions in compile-time predicates obey ownership rules

## Open Questions

- [x] **Z3 availability for the wasm32 target**: there is no Z3 under wasm (all SMT code is excluded
      by `cfg(not(target_arch = "wasm32"))`). Scope and degradation direction: - `ownership.rs`
      `smt_cut` always returns `false` → back-edge traversal → **conservative rejection** (the sound
      direction, albeit with less narrowing) - Termination-checking Strategy 1 and predicate levels
      2b/3 are skipped entirely

  **Correction (2026-10-01)**: the original text stated that these paths were “currently
  **unreachable**” (on the grounds that `predicate_defs` was never populated in production, `parser`
  did not produce `MonoType::Refined`, and production `with_solver` was never invoked). That is no
  longer true—all three have been implemented (predicate-definition registration #377-3;
  refinement-type normalization; production-pipeline solver injection). Therefore, the difference
  between wasm and native is no longer limited to the precision of `smt_cut`:

  | Path                                           | native   | wasm       | Degradation direction                |
  | ---------------------------------------------- | -------- | ---------- | ------------------------------------ |
  | Explicit-measure termination decision          | Decided  | `Unjudged` | No E4022 is reported (fewer reports) |
  | Well-foundedness decision                      | Decided  | `Unjudged` | No E4022 is reported (fewer reports) |
  | SMT entailment level for refinement predicates | Executed | Skipped    | Falls through to `Unproven`          |

  All of these are **conservative** directions (less narrowing and fewer reports) and do not affect
  soundness. To achieve identical precision, the options and costs are described in issue #376
  (conclusion: prioritize a JS-side Z3 instance rather than linking Z3 into the main wasm build; the
  latter would require replacing the emcc build system, and the artifact would grow from 3 MB to
  approximately 20 MB).

- [x] **SMT solver choice**: Z3 by default (MIT license, the broadest validation). CVC5 is an
      SMT-LIB-compatible alternative selectable through a compiler flag. The compiler's internal
      translation target is the SMT-LIB 2.6 standard format—SMT-LIB is the abstraction layer, with
      no custom generic solver interface.
- [x] **Specific solver-budget values**: 10,000 steps / 100ms / quantifier-instantiation depth 3.
      Fixed inside the compiler, with no tuning knob. Adjust them only if actual use cases
      demonstrate that these are insufficient (and not merely because “the user wrote it
      incorrectly”).
- [x] **Quantifier support scope**: the language layer does not restrict quantifier order.
      Compile-time predicates accept Type arguments—Type includes function types—so higher-order
      quantifiers are a natural consequence of the type system and require no special syntax. The
      SMT solver can automatically decide first-order quantifiers (forall/exists, including
      interleaved nesting, limited by the budget depth of 3). For higher-order quantifiers, SMT
      returns Unproven and the compiler prompts, “This predicate is outside the automatic-proof
      scope; please provide a proof function.” The programmer writes a YaoXiang function whose
      return type is equal to the proposition—the type checker verifies that function. No external
      export, AI, or interactive proof mode is required. Everything is YaoXiang code, and everything
      is verified by the type checker.
- [x] **Counterexample formatting**: source variable names are used directly as SMT variable names
      (with module prefixes to avoid collisions). When a Z3 model is returned, values are looked up
      by variable name. The output format is: variable name = concrete value + source location +
      predicate-definition location. No complex mapping layer is implemented.
- [x] ~~**How should compile-time predicates interact with the `ref` smart pointer?~~ → Decision:
      compile-time predicates permit only immutable borrows or values whose ownership has been
      transferred. Values behind mutable borrows cannot occur in compile-time predicates.
- [x] **Should the `forall` predicate violation-count measure be extended to non-adjacent
      operations?** → No extension. The current coverage (adjacent swaps and adjacent movement) is
      complemented by Strategy 1 (linear rank function): quicksort's outer interval contraction is
      covered by Strategy 1 as a fallback, while heapsort is covered by Strategy 1 (array-index
      pattern). The compiler reports an error directly for any loop that no strategy can prove
      terminating—this is the hard-safety philosophy, not a defect. If a real-world (rather than
      academic) algorithm later defeats all four strategies, this question can be revisited.→
      **Revisit triggered (2026-09-14, #318)**: non-structural recursion (gcd-style non-direct
      decrease, mutual recursion, merge partitioning) is precisely the kind of real-world scenario
      anticipated by this clause—the measure-exploration template sequence cannot handle these
      automatically, and they cannot be rewritten into analyzable iteration patterns without
      severely harming readability. Conclusion: the hard-safety philosophy remains unchanged (an
      incorrect measure is still rejected by an SMT counterexample; a human can fail to prove a
      property but cannot prove it incorrectly), but “exploration fails, therefore reject” is
      relaxed to “if exploration fails, the programmer may explicitly provide a measure in the type
      position”—see §6.9.
- [x] **Combinatorial explosion in linear-rank-function enumeration**: the candidate-enumeration
      limit is three bounded variables. For ≤3, all linear combinations are enumerated and checked
      individually with SMT. For >3, only single-variable measures are attempted (`v_i`,
      `u_i - v_i`), and failure immediately produces a compilation error telling the programmer,
      “The loop has more than three bounded variables, and the compiler cannot automatically
      synthesize a multivariable measure.” This is not an engineering compromise—it is a way to
      compel programmers to write simpler loops.

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

## Lifecycle and Disposition

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
