---
title: 'RFC-027: Compile-Time Predicates and Unified Static Verification'
status: 'Accepted'
author: 'Chen Xu'
created: '2026-06-07'
updated: '2026-10-03'
impl_status: 'in-progress'
impl_detail:
  'Phase 1-2 complete, Phase 3 partially complete, Phase 4 partially complete. The unified
  assert/Assert scheme is fully implemented in all 6 Phases (#157-#162 closed): Never type, IsTrue
  bridge, flow-sensitive Γ + kill set, type-level recursion, and universe-stratification weak
  checks. The dispatch-routing semantics in §11 have been implemented through call-site obligations
  (`checker.rs` makes three direct calls to `check_predicate`: binding site / call-site argument /
  return site), but the standalone `layers/dispatch.rs` module has been removed—it had zero
  production call sites, and its handling of `Unproven` (downgrading it to a W1080 warning)
  conflicted with the production implementation (always reporting an error) (#377-2).'
impl_percent: 82
issue_number: 90
issue_url: 'https://github.com/ChenXu233/YaoXiang/issues/90'

issue: '#90'
---

# RFC-027: Compile-Time Predicates and Unified Static Verification

> **Reference**:
>
> - [RFC-009: Ownership Model](009-ownership-model.md)
> - [RFC-010: Unified Type Syntax — The `name: type = value` Model](010-unified-type-syntax.md)
> - [RFC-011: Generic System Design](011-generic-type-system.md)
> - [RFC-024: Concurrency Model Based on spawn Blocks](024-concurrency-model.md)
>
> **Replaces**:
> [RFC-022: Hoare Logic Static Verification Support (Specification Comments and Specification Types)](../deprecated/022-hoare-logic-static-verification.md)
> — Deprecated

## Abstract

This RFC proposes introducing **compile-time predicates** as first-class citizens in YaoXiang and
unifying all compile-time static verification under a single **proof pipeline**. A compile-time
predicate is not an add-on specification comment—it is a function. A function that returns Type can
be used in type position; the compiler invokes it at compile time and checks its return value. Types
are propositions, and compile-time evaluation is proof.

**Core thesis**: the sole task of type checking during compilation is to construct and verify proof
terms. Type equality, token conflicts, dependent type reduction, compile-time predicate evaluation,
and Hoare-logic entailment are all different kinds of type checking within the same compile-time
proof pipeline, sharing one pipeline. The SMT solver is an acceleration module for the type checker,
not an independent trust boundary. When the compiler returns Unproven, the programmer writes a
YaoXiang function as the proof—the type checker verifies it in exactly the same way it verifies the
return type of any function. Everything is YaoXiang code, and everything is verified by the type
checker.

## Motivation

### Why Deprecate RFC-022?

RFC-022 designed specifications in the form of `//!` comments:

```yaoxiang
max: (T: Ord) -> ((arr: Array(T, n)) -> T) = {
    //! requires: NonEmpty(n) = n > 0          ← This is a comment independent of the type
    //! ensures: ExistsMax(result, arr[0..n])   ← This is a comment independent of the type
}
```

This commits a fundamental error in the Curry-Howard correspondence: **it splits specifications and
types into two layers**. Comments are not types. Comments do not participate in type checking.
Comments embody the mental model of an “external tool.”

The whitepaper states this unambiguously:

> “There are no `//!` comments. There is no separate specification language. Everything is within
> the type system.”

### Current Problems

- RFC-022's `//!` comments are add-on syntax outside the type system
- Specification types and ordinary types are two separate systems, causing conceptual redundancy
- The split between validating Debug Builds and ignoring them in Release Builds undermines
  uniformity
- SMT solvers are traditionally perceived as external tools—YaoXiang instead builds one into the
  type checker as an acceleration module
- Type checking, borrow verification, compile-time predicate checking, and macro expansion each
  follow a separate path

### The Correct Mental Model

Type checking can be abstracted as a function:

```
verify : Program → Proved | Disproved(Model) | Unproven
```

All compile-time checks—simple type matching, borrow-conflict detection, and compile-time predicate
verification—are subtasks of this function. They share the same proof pipeline and differ only in
proof-term complexity and construction strategy.

When the compiler returns Unproven, the programmer supplies a proof function whose return type
equals the proposition to be proved. The type checker verifies it. This is the same operation as
ordinary type checking.

## Proposal

### 1. `{}` Is the Proof Space: Types Are Assertions, and Verification Is Type Checking

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
#          Parameter in the signature       {} contains only assertions
#          The compiler validates x > 0 when invoking it at compile time

List: (T: Type) -> Type = { data: Array(T) }
#      ^^^^^^^^              ^^^^^^^^^^^^^^^
#      Parameter in the signature           The compiler validates type_of(T) == Type and type_of(data) == Array(T)
```

The pattern is always the same: `name: (params) -> Type = { assertions }`. The compiler does not
distinguish between “type assertions” and “value assertions”—both are evaluation targets in the
proof pipeline.

**Loop invariants do not need to be written separately. A type annotation on a variable is the
Floyd-Hoare invariant.**

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # The annotation references i—it tells the compiler that the type of s depends on i
    mut i: UpTo(arr.len) = 0     # During initialization i=0; validation: 0 == sum(arr[0..0]) → True
    while i < arr.len {
        s += arr[i]  # The compiler validates: s_new == sum(arr[0..i+1])
        i += 1       # i changes → triggers revalidation of the dependency of s: s satisfies SumUpTo(arr, i_new)
    }
    return s  # s: SumUpTo(arr, arr.len) = sum(arr[0..arr.len])
}
```

The compiler generates one verification condition for the loop body: inductive hypothesis (the type
annotation) → assignment operation → whether the new value satisfies the annotation. Once the proof
pipeline establishes the inductive step, all iterations are covered automatically. No `: decreases`
is needed, no `: Invariant` is needed, and no inductive proof is needed—the compiler decomposes
induction into local VCs for each assignment.

### 2. Preconditions and Postconditions: Compile-Time Predicates on Parameter and Return Types

RFC-022's `//! requires` and `//! ensures` are abandoned. Compile-time predicates become annotations
on parameter or return types.

**On the parameter side, this is a function call.** A compile-time predicate is a function that
returns Type, and using it on the parameter side means invoking it—just like `factorial(5)`. The
return-value side introduces a new concept: the return-value formal parameter.

```yaoxiang
# Precondition: explicitly invoke a compile-time predicate in the parameter type
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b
#                       ^^^^^^^^^^  b is the current formal parameter name; pass b to Positive as an argument
#                       The compiler extracts the actual argument value at the call site, substitutes it for b, and validates Positive(actual argument)
#                       Example: divide(10, 2) → validate Positive(2) = { 2 > 0 } → True
#                       Example: divide(10, 0) → validate Positive(0) = { 0 > 0 } → False → compile error

# Postcondition: return-value formal parameter + compile-time predicate
IsMax: (T: Ord, arr: Array(T), result: T) -> Type = {
    forall j in 0..arr.len: result >= arr[j]
}

NonEmpty: (arr: Array(T)) -> Type = { arr.len > 0 }

max: (T: Ord) -> ((arr: NonEmpty(arr))) -> (result: IsMax(T, arr, result)) = {
#                                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
#                                            result is the return-value formal parameter; its value is provided by return
#                                            The compiler substitutes the returned value for result at the return site and validates the postcondition
    candidate = arr[0]
    for i in 1..arr.len {
        if arr[i] > candidate { candidate = arr[i] }
    }
    return candidate
}
```

**Key rules**:

- **Parameter side**: `b: Positive(b)`—`b` is the current formal parameter name and is passed to
  `Positive` as an argument. This uses function-call syntax, with zero implicit behavior.
- **Return side**: `-> (result: IsMax(T, arr, result))`—`result` is the return-value formal
  parameter, and its value is supplied by a `return` statement. `result` exists only in the type
  signature and is referenced only by the predicate; it does not enter the function body's scope and
  does not appear at the call site.
- **The return-value formal parameter is optional**: without a postcondition, it is omitted and the
  signature is identical to that of an ordinary function (`-> Int`).
- **Uniformity**: parameter and return-value formal parameters are the same
  concept—`formal parameter name: predicate call(actual argument)`—and differ only in whether the
  value is supplied by the caller or by `return`.

> **Implementation note (2026-10-02)**: return-position formal parameters are recognized **strictly
> by their declared names**—the `r` in `-> (r: P(...))` is the sole return-value formal parameter.
> Any remaining free variables in the constraint that are **not in scope** are no longer substituted
> with the return value; instead, an undefined-identifier error is reported (observed:
> `g: () -> (r: Eq2(m, r)) = { return 7 }` (`m` is undefined) →
> `error [E1001] Unknown variable: 'm'`). Predicate-call arguments are substituted into the
> constraint according to the **actual argument**: `P(r + 100)` becomes “return value + 100,” not
> merely “return value.”
>
> **Known gap (2026-10-02)**: **multi-argument predicates with symbolic arguments in return position
> are not yet supported**. Triggering form: `f: (b: Int) -> (r: SumUpTo(b, r)) = { return b * 2 }`
> (`SumUpTo: (n: Int, s: Int) -> Type = { s == n * 2 }`) →
> `error [E2031] … the refinement type constraint of 'r': SumUpTo(b, (b * 2)) cannot be statically proven`.
> Workaround: change the argument to a compile-time-foldable literal such as `SumUpTo(3, r)`, or
> provide the predicate with a proof function returning `Type`; unblock condition: the proof kernel
> must support actual-argument substitution for “multi-argument predicate + symbolic argument.” See
> [RFC-027a](../accepted/027a-termination-explicit-measure.md) §Implementation Notes (2026-10-02,
> current hardening pass).

### 3. Path-Condition Propagation: Compile-Time Verification of Runtime Values

When a compile-time predicate is used at a binding site, its arguments are explicitly supplied by
the programmer. When a runtime value enters a refinement-type argument, the compiler completes
verification by collecting path conditions and applying SMT entailment—no explicit proof passing is
required.

#### 3.1 Explicit Function Calls

When a compile-time predicate is used at a binding site, its arguments are explicitly supplied by
the programmer—it is a function call, with zero implicit behavior.

`Positive: (x: Int) -> Type = { x > 0 }` is a compile-time predicate constructor. When it appears in
a binding position (a parameter declaration, variable declaration, or return type), the programmer
explicitly supplies an already-bound variable name:

```yaoxiang
b: Positive(b)
// b is already declared as the current formal parameter; Positive(b) is a function call
// After normalization: b: { b > 0 }
```

The compiler need not implicitly fill in arguments—`b: Positive(b)` is a function call, just like
`f(5)`. The name `b` is bound as a parameter, and its type annotation `Positive(b)` references `b`
itself. This is the standard dependent-type pattern, not an implicit-expansion rule.

**Unification with RFC-010 `self`**: RFC-010 establishes that `self` is not a keyword, but merely a
conventional parameter name (“writing `p`, `this`, or `x` has exactly the same effect”).
`b: Positive(b)` uses the same mechanism—a parameter name may be referenced in a type annotation.
`self` appears in `self: Point`, while `b` appears in `b: Positive(b)`; both annotations reference
the parameter itself. The only difference is the complexity of the type annotation; the mechanism is
identical—once a name is bound, a type may depend on that name.

Return types likewise use explicit function calls:

```yaoxiang
Sorted: (arr: Array(T)) -> Type = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }

sort: (arr: Array(T)) -> (result: Sorted(result)) = { ... }
//                        ^^^^^^^^^^^^^^^^^^^^^^^
//                        result is the return-value formal parameter; Sorted(result) is a function call
//                        The compiler substitutes the returned value for result at the return site and validates Sorted(returned value)
```

The same applies to local-variable declarations:

```yaoxiang
let x: Positive(x) = 5
// x is bound to 5; Positive(5) → { 5 > 0 } → True → passes

// let y: Positive(y) = 0
// y is bound to 0; Positive(0) → { 0 > 0 } → False → compile error
```

#### 3.2 Path-Condition Collection

When a runtime value appears in a conditional branch, the compiler automatically collects path
conditions to form an **assumption set** for the current scope. These assumptions serve as
background knowledge during compile-time Bool evaluation.

```yaoxiang
if y > 0 {
    // The compiler automatically obtains the assumption here: { y > 0 }
    let result = divide(x, y)
    // Verification condition: (y > 0) ⇒ (y > 0)
    // The proof pipeline finds that the entailment holds → Proved
} else {
// This branch assumes: { !(y > 0) }
// To call divide(x, y), the verification condition is !(y > 0) ⇒ y > 0
    // The proof pipeline finds that the entailment does not hold → Disproved
}
```

This is not a hardcoded special case in the compiler—it is the natural behavior of the compile-time
proof pipeline. Every type-check call site sends the following to the pipeline:

```
{背景假设} ⇒ {验证目标}
```

The proof pipeline determines entailment. Proved → pass; Disproved → compile error + counterexample;
Unproven → compile error + unresolved proposition. The background assumptions come from the path
conditions at the current program point.

#### 3.3 Assumption Stack

While analyzing control flow, the compiler maintains an assumption set for each basic block:

- **if-guard**: `if y > 0` → push `y > 0` in the true branch; push `!(y > 0)` in the false branch
  (if an else branch is used)
- **match pattern**: `if let Some(v) = opt` → push `opt == Some(v)` within the branch
- **Logical conjunction**: `if x > 0 and y < 10` → push `x > 0` and `y < 10` within the branch
- **Function precondition**: when calling `divide(a, b)`, evidence that `b` satisfies `Positive`
  must come either from the current assumptions or from the actual argument's refinement-type
  annotation (if `b` is already annotated as `Positive`, its type carries `b > 0`)
- **Assignment**: when executing `let z = y`, existing refinement conditions on `y` are propagated
  to `z`

All assumptions enter the compile-time proof pipeline. When the SMT-accelerated path is taken, they
are translated into SMT-LIB background assertions.

#### 3.4 A Compile Error Without Static Evidence

Suppose the programmer directly writes:

```yaoxiang
divide_user_input: (x: Int, y: Int) -> Int = divide(x, y)
```

There is no `y > 0` assumption at the current program point, and the actual argument `y` itself has
no `Positive` type annotation. The verification condition is:

```
{} ⇒ { y > 0 }
```

The pipeline returns `Disproved` (the implication does not hold) → compile error:

> Unable to prove that parameter `b` satisfies `Positive` in the call to `divide`. `y` comes from
> the function input and has no proven bound. Consider guarding the call with an if branch:
> `if y > 0 { divide(x, y) }`.

YaoXiang does not permit a runtime value to enter a refinement-type argument without static
evidence. This is not a restriction—it is the core of the hard-safety philosophy. No code that the
compiler cannot statically prove safe may pass compilation.

#### 3.5 Relationship to the Unified Pipeline

Path-condition propagation is not an additional mechanism. It is a direct extension of the
compile-time proof pipeline into control-flow analysis:

| Stage                             | Responsibility                                                                                                                                                  |
| --------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Path-condition collection         | The compiler's control-flow analysis stage annotates each basic block with an assumption set                                                                    |
| Verification-condition generation | When a type constraint requires validation, merge path conditions with actual-argument type information                                                         |
| Proof-pipeline evaluation         | Compiler kernel → SMT acceleration → derive Proved / Disproved / Unproven                                                                                       |
| Outcome                           | `Proved` → pass; `Disproved` → compile error + counterexample; `Unproven` → compile error + unresolved proposition (the programmer may supply a proof function) |

There are no new components and no special rules. Path conditions are background knowledge for the
proof pipeline, shared with type equality and borrow constraints on the same pipeline and under the
same budget system.

### 4. The Compile-Time Proof Pipeline

All compile-time checks share one pipeline. Its core operation is **type checking**—checking whether
the type of a proof term equals the proposition to be proved. Everything is type checking.

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
a natural partition of proof theory:

```
eval_compile_time : BoolExpr → Proved | Disproved(Model) | Unproven
```

- **Proved** → Halts; the proof term has been constructed and type checking passes. Compilation
  continues.
- **Disproved(M)** → Halts; a counterexample M exists. Compile error + counterexample + source
  location.
- **Unproven** → No proof was constructed within the given resource limit. Compile error +
  unresolved proposition + budget-consumption report.

**Unproven ≠ False.** The compiler saying “I cannot prove it” is not equivalent to the proposition
being false—it only means the proposition exceeds the current automatic proving capability. This is
honesty, not a defect.

Hard budget limits are the engineering solution to the halting problem. Do not provide a knob—doing
so asks the user, “Do you think your program will halt?” Neither the user nor the compiler knows the
answer.

#### 4.2 After Unproven: The Programmer Writes the Proof

When the compiler returns Unproven, the programmer can write a **proof function**—a YaoXiang
function whose return type equals the proposition to be proved. The type checker verifies this
function in exactly the same way it verifies `add(a, b): Int`.

```
命题  = 类型
证明  = 程序（该类型的一个值）
验证  = 类型检查（唯一的信任根）
```

The SMT solver is not an independent trust boundary—it is an **acceleration module for the type
checker**. SMT helps find proofs, but the proof is always verified by the type checker. When SMT
returns `unsat`, the compiler reconstructs the result as a proof term that the type checker can
verify. If reconstruction fails (because the SMT reasoning steps exceed the compiler kernel's
inference rules), it falls back to Unproven, and the programmer can manually write a proof function.

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

No AI is required, no export to Coq is required, and no new concepts are required. **A property that
cannot be proved automatically at compile time → the programmer writes the proof in YaoXiang code →
the type checker verifies it.** The entire process is a smooth gradient—the compiler handles the
easy proofs and leaves the hard ones to the programmer.

#### 4.3 Layered Dependencies Within the Pipeline

The evaluators above share one interface but have an evaluation order. Type equality is a
prerequisite for all subsequent analyses; ownership/token checking depends on type information; and
refinement-predicate validation depends on the results of the preceding two layers. The compiler
evaluates layer by layer, and expressions that fail at a lower layer do not proceed to higher
layers—avoiding wasting solver budget on programs that already contain type errors.

```
求值顺序（同一管道，分层调度）
├── 第 0 层：类型等式（T1 == T2）
│   └── 结构合一 → 失败则后续无意义，直接返回 Disproved
├── 第 1 层：所有权/令牌冲突
│   └── 流敏感活性分析 → 失败则内存安全不成立，直接返回 Disproved
└── 第 2 层：精化谓词/霍尔蕴含
    └── 编译器自身 → SMT 加速 → 得出 Proved / Disproved / Unproven
```

Each layer still returns `Proved/Disproved/Unproven`, sharing the same interface and the same budget
system.

### 5. Unification of the Three Function Layers

| Layer                  | Execution time | Input      | Output | Example                                        |
| ---------------------- | -------------- | ---------- | ------ | ---------------------------------------------- |
| Value-level function   | Runtime        | Value      | Value  | `add: (a: Int, b: Int) -> Int = a + b`         |
| Type constructor       | Compile time   | Type/value | Type   | `List: (T: Type) -> Type = { data: Array(T) }` |
| Compile-time predicate | Compile time   | Value      | Type   | `Positive: (x: Int) -> Type = { x > 0 }`       |

All three use the same `name: type = value` syntax. Compile-time predicates and type constructors
use the same compile-time proof pipeline—`{}` is the proof space.

### 6. Loops: Floyd-Hoare Verification-Condition Generation

Loops do not require a separate `: Invariant(...)` or `: decreases(...)` annotation. Compile-time
predicate annotations on variables define Floyd-Hoare-style assertions—the compiler derives
verification conditions from the type annotations and asks the proof pipeline to check whether each
assignment preserves the type.

The core mechanism is that every assignment corresponds to a Hoare triple `{P} x := e {Q}`, whose
verification condition is `P ⇒ Q[e/x]`. The compiler generates one verification condition for the
loop body—once the proof pipeline establishes the inductive step, all iterations are covered
automatically.

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # The annotation references i; during initialization i=0, validation: 0 == sum(arr[0..0]) → True
    mut i: UpTo(arr.len) = 0     # Validation: 0 <= 0 <= arr.len → True
    while i < arr.len {
        # The compiler generates one VC for the loop body. Preconditions: s satisfies SumUpTo(arr, i), and i satisfies UpTo(arr.len).
        #
        # s += arr[i]:
        #   Verification obligation: s_new satisfies SumUpTo(arr, i) (i is unchanged for now)
        #   Substitute s_new = s_old + arr[i]:
        #     Need s_old + arr[i] == sum(arr[0..i+1])
        #     From the inductive hypothesis s_old == sum(arr[0..i]), add arr[i] to both sides:
        #     sum(arr[0..i]) + arr[i] == sum(arr[0..i+1])
        #   Compiler + SMT: linear arithmetic, milliseconds → Proved
        #
        # i += 1:
        #   i changes → the dependency graph shows that s's type annotation references i → triggers revalidation
        #   New verification target: s satisfies SumUpTo(arr, i_new)
        #   That is, s == sum(arr[0..i_new]), guaranteed by the previous step → Proved
        s += arr[i]
        i += 1
    }
    return s  # At this point s: SumUpTo(arr, arr.len), i.e. s == sum(arr[0..arr.len])
}
```

The loop invariant is the type annotation on the variable—the programmer writes a type, and the
compiler checks the inductive step. The compiler does not need to “discover” invariants or “perform
induction automatically”; it decomposes an inductive proof into local verification conditions for
each assignment and hands them to the proof pipeline to solve by divide-and-conquer.

#### 6.1 Dependency Tracking: Dependent Types on Mutable Variables

The mechanism above presupposes that the compiler knows that the type annotation of `s`,
`SumUpTo(arr, i)`, references `i`—when `i` changes, the type constraint on `s` changes as well. This
requires the compiler to maintain a **type-dependency graph between variables**.

**Data structure**:

```
TypeDepGraph: Map<VarName, Set<VarName>>
# Keys are the variables being depended on; values are the sets of variables whose type annotations reference that variable
# Example: { i: {s}, j: {s, t}, ... }
```

**Construction**: when the type checker processes `mut v: Pred(... x ...) = init`, it resolves
free-variable references in the arguments of `Pred(...)`. If an argument references another mutable
variable `x` in the current scope, the dependency graph records `x → v`.

**Trigger**: when a depended-on variable `x` is assigned, the compiler:

1. Finds all variables `{v₁, v₂, ...}` in the dependency graph that depend on `x`
2. For each `v`, generates a verification condition: does the current value of `v` satisfy the
   updated type Pred(... x_new ...)
3. Sends the VC into the proof pipeline

**Assignment-order sensitivity**: dependency tracking naturally enforces the correct assignment
order. Using `SumUpTo(arr, i)` as an example:

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
independent verification mechanism. It answers “when must a VC be generated?”; the proof pipeline
answers “does the VC hold?”

### 7. Termination Checking

**Scope: refinement types.** Termination is not an independent switch; it is part of the
**verification mode**. Once a type is refined (`Refined { base, constraint }`), the computation it
annotates enters verification mode, and termination is required within that mode. Ordinary types
that are not refined do not enter verification mode and generate no termination obligations.

This yields two corollaries:

- **Loops**: a bare `while` does not enter verification mode; when a measure variable has a
  refinement annotation such as `i: UpTo(n)`, the loop enters verification mode and termination must
  be proved.
- **Recursion**: when a function signature contains a refinement (a refined parameter or a refined
  return type), the function enters verification mode, and the measure must strictly decrease at
  every recursive call site.

Automation has priority within the mode: the compiler first explores for a measure automatically,
and passes if it can prove one; if exploration fails and no explicit measure has been supplied,
compilation fails. There is no **annotation-syntax** escape hatch—the measure and termination
proposition are both written in type position, introducing no new `decreases`-style syntax. See §6.9
for the forms of measures and explicit fallbacks.

#### 6.1 Design Principles

The compiler automatically extracts the information needed for termination proofs from two sources:

1. **Variable type annotations**: boundary constraints in refinement types (for example, `UpTo(n)`
   provides upper bound `n` and lower bound `0`)
2. **Loop-body operations**: the operation applied to a variable on each iteration

The compiler tries four measure-synthesis strategies in priority order and stops as soon as it finds
one. The four strategies form a **restricted template sequence for measure exploration**. Their
input consists of refinement constraints (strategies 1–4 all begin with a variable of bounded type),
not “compile-time-evaluated code”; they are the automatic and manual sides of the same mechanism as
the explicit measures in §6.9.

> **Measure exploration is exploration, not inference.** Exploration merely enumerates templates
> (linear rank, violation count, bounded patterns, multiplicative scaling); it does not guarantee
> that a solution exists—general measure inference is generally undecidable (it reduces to the
> halting problem). Cases outside the templates must therefore be given an explicit measure by the
> programmer (§6.9), or compilation fails.

#### 6.2 Strategy 1: Automatic Synthesis of a Linear Rank Function

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

#### 6.3 Strategy 2: Predicate Violation Count—Automatically Extracting a Measure from the Target Type <span style="color:orange">[Experimental Strategy]</span>

> ⚠️ **Current status: an experimental strategy whose inclusion will be decided during Phase 3
> implementation based on practical feasibility.** This strategy works for adjacent-swap operations
> (bubble sort, insertion sort), but cannot automatically prove termination for non-adjacent
> operations (quicksort partition, heap-sort sift-down). See the table below for the coverage
> boundary. If Phase 3 validation proves infeasible, this strategy will be removed or deferred to
> future work.

Core insight: **the specifications written by the user are material for compiler reasoning.** The
compiler does not need to hard-code “what sorting means”—it reads the definition of `Sorted` and
automatically extracts a measure from that definition.

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

| Algorithm      | Operation pattern | Can Strategy 2 prove it? | Reason                                                    |
| -------------- | ----------------- | :----------------------: | --------------------------------------------------------- |
| Bubble sort    | Adjacent swap     |            ✅            | violation_count strictly decreases with each swap         |
| Insertion sort | Adjacent move     |            ✅            | Each shift eliminates one violating pair                  |
| Selection sort | Non-adjacent swap |            ❌            | A single swap may increase violation_count                |
| Quicksort      | partition         |            ❌            | Non-adjacent swaps do not guarantee monotonic decrease    |
| Heap sort      | sift-down         |            ❌            | Tree-shaped operations make violation_count non-monotonic |

**Complementary strategies**: for quicksort, contraction of the `low < high` interval can be covered
by Strategy 1 (a linear rank function)—the outer partition recursion halves the interval each time.
Strategies 1 and 2 provide complementary coverage, and one or the other can prove termination for
most practical algorithms. However, generalizing Strategy 2 to non-adjacent and tree-shaped
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

For `v += const` (a positive constant), the variable has an upper-bound type annotation → the
measure `upper_bound - v` decreases by `const` each iteration, with lower bound 0. This is a
degenerate case of Strategy 1, which the compiler handles first for speed.

#### 6.5 Strategy 4: Multiplicative-Scaling Measure Templates

For `v *= const` (`const > 1`), the variable has upper- and lower-bound type annotations. The
compiler includes the logarithmic measure template `ceil(log_const(upper/v))`; each multiplication
by `const` decreases the measure by 1.

```yaoxiang
mut i: Positive(i) = 1
while i < n {
    # The compiler automatically derives: measure ceil(log₂(n/i)); multiplying by 2 decreases it by 1
    i *= 2
}
```

#### 6.6 Separation of Termination and Correctness

Termination proofs and correctness proofs are independent:

- **Termination**: the four strategies above automatically prove that a loop exits in finitely many
  steps; if exploration fails, the programmer supplies a measure in type position (§6.9)
- **Correctness**: whether the loop body advances toward the target type is checked through
  verification conditions in the compile-time proof pipeline

If both pass, compilation succeeds. If termination is proved but correctness fails → compile error +
counterexample. If correctness is proved but termination cannot be proved → compile error
identifying the variable or operation that could not be analyzed. If both fail → compile error
reporting both failure causes separately.

#### 6.7 Termination Checking for Recursive Functions

For a recursive function with a refined signature, the compiler checks that formal parameters
decrease at each recursive call site:

```yaoxiang
gcd: (a: Int, b: NonNegative(b)) -> Int = {
    if b == 0 { return a }
    return gcd(b, a % b)  // The compiler explores: the well-founded measure of (b, a % b) decreases → terminates
}
```

Formal-parameter decrease is the strongest automatic path (structural recursion). If exploration
fails, the programmer explicitly supplies a measure in type position (§6.9).

#### 6.8 Hard Boundary

For `i = f(i)`, where `f` is neither invertible nor closure-preserving nor monotone in any
respect—automatic termination proof is mathematically impossible. Compilation fails:

> This loop cannot be proved to terminate automatically. The loop variable depends on the
> unanalyzable function `f`. Use an iteration pattern that the compiler can analyze, or bind a name
> to this loop and provide a measure in type position (§6.9).

This is not a compiler failure. No code that cannot be statically proved safe may pass compilation.
Even when an explicit measure is supplied, it must be judged true by SMT before it can pass—an
incorrect measure is rejected with a counterexample. Humans can fail to prove termination, but
cannot prove a false claim.

#### 6.9 Explicit Measures: `Terminates`

When automatic exploration cannot find a measure, the programmer writes the measure in **type
position**—through the same mechanism as `Positive(b)` and `IsMax(T, arr, result)` (predicate
application), with zero new syntax:

```yaoxiang
// Measure: an ordinary function that is unit-testable, reusable, and absent at runtime
gcd_measure: (a: Int, b: Int) -> Int = { b }

// The termination component lives in the function's own type position
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// Loop: the binding name is the anchor; the measure obtains values from names in scope
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
tail expression, and the binding name `acc` provides the anchor. The loop therefore becomes
referable, eliminating the “anonymous construct with no referent” blind spot.

> **The body tail must yield a value**: the loop's value = the value of the loop-body block (§2.9),
> and the block's value = its tail expression. If the tail is an assignment statement such as
> `{ i = i + 1 }`, the block's value is `Void`, so `return acc` returns `Void` from `-> Int`, and
> the type does not hold. The example therefore explicitly writes `i` as the body tail. This does
> not change the measure's semantics—the measure `n - i` is a **state expression**, independent of
> the loop's value type.

> **Implementation note (recorded 2026-10-02; updated 2026-10-04)**: this section's loop example is
> valid on the **type side** (`check` reports 0 errors and the back-edge obligation is Proved). The
> **runtime-value defect (#409) has been fixed**: a `while` used as a value now takes the value of
> the body block's tail expression on the final iteration, matching the type-side `block_value_ty`—
> `loop(4) == 4` and `countdown(3) == 30` now execute correctly (`for` has the same fix).
>
> **Known residual issue (zero iterations)**: when the loop body is never evaluated, there is no
> “body value from the final iteration,” so the loop value remains `Void`; binding it to a
> non-`Void` type position and using it at runtime reports
> `E6007 Runtime error: type mismatch in comparison Eq: Void vs Int(0)` (a loud type mismatch, not a
> silently incorrect value). This is a **static-approximation boundary**: the `block_value_ty`
> criterion yields the type of the tail expression, but cannot determine whether the loop executes
> at least once (which would require dataflow/provability analysis), so the type side deliberately
> does not degrade (degrading would break the value type required by the `Terminates` loop binding
> in this section). The residual behavior is pinned by the corpus
> `tests/yaoxiang/02-type-system/termination/while_zero_iteration_void_err.yx`
> (`// expect: runtime-error E6007`). **This residual issue has been confirmed as an intentionally
> retained design boundary** (ruling of 2026-10-04, no fix for now), so issue
> [#409](https://github.com/ChenXu233/YaoXiang/issues/409) has been closed along with the main-form
> fix and will not be tracked separately. Historical records of the minimal reproduction and
> affected areas appear in [RFC-027a](../accepted/027a-termination-explicit-measure.md) §Example,
> Loop section, “Known Defect D6.” In addition: termination Strategy 1 (linear rank function) is
> effective again in this round, so the ordering in this section—write an explicit measure only
> after automatic exploration fails—remains unchanged: an explicit measure is still the fallback
> after exploration fails.

**`Terminates` is a builtin predicate**, belonging to the same core primitives as `Int` and `Never`.
It is the only predicate **whose function body is written by the compiler**—its assertion (“the
measure strictly decreases at every recursive call site/loop back edge”) is embedded in the
computation structure. A user-written predicate cannot refer to the function body or loop body and
therefore cannot express this using the predicate-definition syntax in the “Syntax” section. The
builtin surface is narrowed to this single name.

**Arity**. `Terminates(FnType, m)` and `Terminates(m)` are two arities of the same predicate, not
two separate constructs:

| Form                    | Anchor                        | Use case                                                          |
| ----------------------- | ----------------------------- | ----------------------------------------------------------------- |
| `Terminates(m)`         | Name of the enclosing binding | Self-recursive functions and loops—the default form               |
| `Terminates(FnType, m)` | Explicit function type        | Scenarios such as mutual recursion where the anchor is not unique |

The two are essentially identical: a termination obligation always attaches to “the computation
annotated by the type position in which the refinement resides.”

**Measures do not restrict the return type.** A measure may be an expression of any type (it need
not be a natural number); its “strict decrease” is determined by the well-founded ordering available
on that type. Whether the measure is well-founded—for example, whether it is `>= 0` when it returns
`Int`—is an **independent obligation**, handled by the same refinement derivation or SMT machinery.
If neither can be inferred, the diagnostic **does not directly reject** the program, but instead
suggests checking the direction (whether the measure's lower bound holds and whether recursive
arguments actually move in that direction).

**Relationship to automatic exploration**: an explicit measure is not a separate pipeline; it is an
input supplied after exploration fails. Once a measure is supplied, the same SMT machinery still
validates decrease and well-foundedness; if either does not hold, compilation fails with a
counterexample.

The implementation mechanisms for obligation generation, the decision pipeline, diagnostic
direction, and the measure-sharing graph for mutual recursion (SCC) are described in
[RFC-027a: Explicit Measures for Termination Checking](../accepted/027a-termination-explicit-measure.md).

### 8. SMT Solver: An Acceleration Module for the Type Checker

In traditional languages, the SMT solver is an external tool—for example, F\* invokes Z3 and Dafny
invokes Z3. In YaoXiang, it is an **acceleration module within the type checker** and is invoked
only when the compiler kernel cannot decide directly. SMT helps find proofs, but the proof is
verified by the type checker.

**Trust model**: the type checker is the sole trust root. The SMT solver is an acceleration
module—it helps find proofs, but SMT is not an independent trust boundary. The compiler trusts Z3's
`unsat` result (consistent with the F\*/Dafny approach—the probability of a Z3 error is lower than
the compiler's own bug rate, which is a pragmatic engineering choice). The real unreliability is
confined to the SMT translation layer; if that layer has a bug, other tests will expose it.

**Interface**: internally, the compiler translates to the SMT-LIB 2.6 standard format rather than
binding to a solver-specific API. SMT-LIB is an ISO standard, supported natively by Z3, CVC5,
MathSAT, and Yices.

**Default backend**: Z3 (MIT license, with the broadest documentation and community validation).
CVC5 is an SMT-LIB-compatible alternative, and users may switch to it through a compiler flag at
build time.

Do not build a “generic solver abstraction layer”—SMT-LIB is the abstraction layer. If CVC5 later
gains an advantage in a particular theory, switching requires only replacing the binary, not
modifying compiler code.

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

**Solver budget—hard limits, like stack depth**:

| Budget dimension               | Default | Description                                                                                                                                                                        |
| ------------------------------ | ------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Solver steps                   | 10,000  | Z3 usually handles linear arithmetic within hundreds of steps. 10,000 steps covers 99% of practical predicates.                                                                    |
| Time                           | 100ms   | A single predicate taking more than 100ms means the user is writing a compile-time program rather than a type annotation. 100ms × 50 predicates = a 5-second compilation-time cap. |
| Quantifier instantiation depth | 3       | Three nested quantifier levels cover practical patterns. Beyond three levels, the user is most likely writing a logic exercise.                                                    |

Exceeding the budget returns Unproven, producing a compile error + predicate location + amount
consumed. There is no degradation, no runtime check, and no silent pass.

**Why this is practical**: in practice, 95% of predicates are linear arithmetic—`x > 0`,
`arr.len > 0`, `0 <= idx < arr.len`—all within a decidable fragment, for which the SMT solver
returns in milliseconds. For the rare complex predicate that exceeds the budget, the programmer can
write a proof function.

Dependent types undergo one layer of pre-reduction before the SMT call: `factorial(5)` is evaluated
directly at compile time to `120`, and `append([1,2], [3])` is evaluated directly to `[1,2,3]`.
These deterministic value computations do not consume SMT budget.

Programmers do not need to know that SMT exists. The mental model is: **the compiler passes what it
can prove and reports an error otherwise—if the compiler cannot prove it, write a function that
proves it for the compiler.**

### 9. Composition of Compile-Time Predicates

A compile-time predicate is a function that returns Type, and composition is implemented naturally
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

result = divide(10, 2)   # ✅ The compiler validates Positive(2) = { 2 > 0 } → True
# result = divide(10, 0)  # ❌ The compiler validates Positive(0) = { 0 > 0 } → False
```

#### 9.2 Array-Access Safety

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

### 11. The `dispatch` Routing Pipeline: Unified Compile-Time and Runtime Dispatch

> **Implementation note (recorded 2026-10-03, #377-2)**: the **semantics** of this section are
> established and integrated into production, but the **structure** is not a standalone
> `layers/dispatch.rs` module. Production dispatch is implemented through three call-site
> obligations in `checker.rs`—binding-site revalidation (`revalidate_refined`), call-site arguments
> (`check_call_arg_refinements`), and the return-site postcondition (`check_return_refinement`). Of
> these, the ones that **directly call** `layers::predicate::check_predicate` are **two**: call-site
> arguments and the return-site postcondition share the sole decision unit
> `judge_refined_obligation` (`checker/refinement.rs`), while `revalidate_refined` has its own E2030
> behavior. The original `layers/dispatch.rs` had zero production call sites, and its handling of
> `Unproven { ProofFunctionRequired }`—degrading it to a W1080 warning and injecting Γ—conflicted
> with the production implementation (`Unproven` always reports an error; see `refined_unproven`),
> so it was removed in its entirety. Note also that the Runtime row's “insert runtime check” has
> **no** emission point in production (`RuntimeOutcome::InsertCheck` has no consumer); Γ injection
> is independently handled by branch guards in `ownership.rs` and `inference/expressions.rs`.

`assert` and `Assert` are two sides of the same refinement-type primitive. The `dispatch` routing
pipeline automatically chooses between compile-time proof and runtime checking according to
**whether the predicate's free variables are accessible at compile time**:

| Criterion                                                                                  | Mode            | Behavior                                                                                       |
| ------------------------------------------------------------------------------------------ | --------------- | ---------------------------------------------------------------------------------------------- |
| All free variables are known at compile time (generic parameters, compile-time constants)  | **CompileTime** | Enter the proof pipeline: Proved → erase; Disproved → compile error; Unknown → require a proof |
| Some free variables come from runtime (function parameters, external input, mut variables) | **Runtime**     | Insert a runtime check and inject the refinement fact into the flow-sensitive assumption set Γ |

**Key point**: “cannot decide” ≠ “disprove.” In CompileTime mode, Unknown requires a proof (there is
no silent degradation). In Runtime mode, the proposition has no truth value at compile time in the
first place—no matter how strong the prover is, it cannot produce a universally true proof for “the
user may enter a negative number,” so a runtime check is the only sound choice. This is not a
weakness of the prover; it is a theoretical necessity.

### 12. Flow-Sensitive Assumption Set Γ: Strongest-Postcondition Propagation

The compiler maintains a flow-sensitive assumption set Γ that tracks the propositions known to hold
at each control-flow point.

**SP (strongest-postcondition) propagation**:

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

This is a hard soundness requirement—after a variable's value changes, old assumptions are invalid.

**Branch confluence**: when IF/ELSE or match branches are merged, Γ is the intersection of the
branch assumption sets. Only propositions that hold on every path are carried out of the branch.

**Equality bridge (automatic evidence for the q=0 slot)**: constant-equality facts in Γ are
legitimate evidence for compile-time value arguments (RFC-011 §4.1, the q=0 slot). Within the guard
branch of `if y == 42 { f(y)(y) }`, Γ records `y == 42` ⇒ y is treated as 42 in the q=0 slot, and
the call is checked through the existing path as `f(42)`. One side of the equality must be a
compile-time constant; a runtime value without equality evidence still reports an error at the call
site. This is not a new mechanism—the verification condition `{y == 42} ⇒ {y ≡ 42}` uses the
existing entailment decision.

#### 12.1 Definition of Definitional Equality (defeq)

This section was added by RFC-039 Decision D62: the equality bridge above supplies the evidence
source for the q=0 slot, while this section completes the definition of definitional equality
itself, the role of Unproven in type equality, and the diagnostic shape.

Definitional equality is the sole equivalence relation on types. It is given by exactly one rule:
after normalization, the two sides are **the same**.

- **Normalization** consists of three reductions: δ unfolds type aliases and definitions; β reduces
  type families and dependent type applications; and ι normalizes literal types—the identity of a
  literal type is determined solely by its value, and source spelling does not participate (`03` and
  `3` are the same type).
- **Identity** is fully structural: type constructors have the same name, and actual arguments are
  identical one by one. Definitional equality does not inspect cache keys or rendered text, nor does
  it perform any reasoning about refinement predicates.
- **Subtyping is an independent relation**: `A <: B` is decided by the subtyping rules; the fact
  that `A <: B` and `B <: A` both hold does not define definitional equality. Combining
  bidirectional subtyping with equality would mix covariance and interface duck-typing rules into
  equivalence, polluting the distinction between “assignable” and “equivalent.”
- **Different `Refined` predicates ⇒ Unproven**: when the base types are the same but the predicates
  differ, the compiler returns Unproven, not Disproved. Predicate equivalence belongs to the
  assumption-set Γ layer (the equality bridge above and the subsequent Γ equality rules); until that
  layer is implemented, the compiler can only say “cannot decide.”

The failure modes of definitional equality align with the three-valued algebra of §4.1: both sides
normalize and are identical ⇒ `Proved`; both sides normalize but differ ⇒ `Disproved`; either side
fails to normalize ⇒ `Unproven` (budget exhausted or capability boundary). **Reduction failure never
produces `Disproved`**—failure to compute is not inequality.

Definitional equality is a pure decision: it **does not bind** type variables, and success accepts
without leaving an instantiation. Decision sites (assignment, actual argument, return position)
first use unification; after unification fails, definitional equality may be used, but any partial
bindings produced by the unification attempt must first be rolled back. Without rollback, an attempt
judged non-equivalent leaves bindings behind that contaminate subsequent inference.

#### 12.2 Unproven ≠ False: Type Equality

The three-valued algebra of §4.1 applies to every obligation point, including type equality. This
section grounds “Unproven ≠ False” at this layer.

- When an obligation point returns `Unproven`, the compiler **must** report a compile error and must
  not let the program pass. The programmer's remedy is to supply evidence: write a proof function in
  the refinement domain (§4.2), or provide a normalizable form in the equality domain.
- `Unproven` and `Disproved` must be distinguishable: `Disproved` carries a counterexample (a
  concrete assignment or two non-equivalent sides), while `Unproven` carries an unresolved
  proposition and its reason. Rendering `Unproven` as “does not hold” is an incorrect report.
- `Unproven` **must not** be degraded to an internal compiler error (E8001). E8001 indicates only a
  consistency failure within the compiler itself; budget exhaustion and capability boundaries are
  limits of the compiler's reasoning ability and belong to user-facing diagnostics (RFC-013,
  “Message Quality Specification,” section 5).
- `Unproven` **must not** silently pass. When a decision point ends in failure, the diagnostic shown
  to the user must come only from an existing failure path; no second diagnostic is added, and
  “cannot decide” never results in acceptance.

#### 12.3 Diagnostic Shape

An `Unproven` diagnostic must carry three items; it is incomplete if any is missing:

- **Unresolved proposition**: renderable proposition text (a refinement constraint, the two sides of
  an equality, or a termination measure)
- **Reason**: a capability boundary (beyond the kernel's inference rules), budget exhaustion, or an
  unbound symbol name
- **Location**: source location. A decision point with genuinely no location uses the default
  location, with the reason documented in an implementation comment

Budget details are presented as a **one-line summary** (for example, “Budget 10000 steps, 10000
steps used”). The complete step count, solver-internal trace, and failure shape belong in the
verbose channel or logs, not in default output.

This section formally relaxes §4.1's “Unproven → compile error + unresolved proposition +
budget-consumption report”: the substance is unchanged—the user must still distinguish “cannot
compute it” from “does not hold”; only the granularity of the budget-consumption report is relaxed.
The reason is that the complete step count provides no actionable value to the user—the user cannot
modify the program based on it—and it drifts with implementation details. After restricting it to a
one-line summary, diagnostic stability and readability improve, while distinguishability is
guaranteed by the “Reason” field.

The division of responsibility between general `Unproven` and refinement-specific codes is described
in RFC-013's draft code table (E4023 / E2031).

### 13. Erasure Model Clarification: Erasing witness ≠ Erasing check

RFC-027's claim that “refinement types are **completely erased** at runtime” refers specifically to
the **proof witness (proof token)**—a proof term verified at compile time produces no runtime code.
However, the **runtime check** inserted by dispatch in Runtime mode is retained—it is a Bool check
executed at the value level, not a type-level witness.

In summary: witnesses are erased; checks are retained. The two are not in conflict, and RFC-027's
original claim remains unchanged.

## Detailed Design

### Syntax Changes

| Before (RFC-022)                      | After (this RFC)                                                                                                                                                        |
| ------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `//! requires: NonEmpty(n) = n > 0`   | Compile-time predicate as a parameter type `(b: Positive(b))`                                                                                                           |
| `//! ensures: ExistsMax(result, arr)` | Return type uses a return-value formal parameter `-> (result: IsMax(T, arr, result))`                                                                                   |
| `/*! invariant: ... !*/`              | Compile-time predicate type annotation on a variable—Floyd-Hoare invariant                                                                                              |
| `//! decreases: n`                    | Measure is written in the refinement-type position (`Terminates`); the compiler explores automatically first and requires an explicit measure only if exploration fails |
| Specifications are comments           | Specifications are part of the type system                                                                                                                              |

### Syntax

**Compile-time predicates introduce no new keywords.** `{}` is the proof space and uses exactly the
same syntax as existing type definitions. A compile-time predicate is a function that returns
Type—`name: (params) -> Type = { assertions }`. It is used through a function call—`Positive(b)`,
`IsMax(T, arr, result)`.

```bnf
# Compile-time predicate = a function returning Type; {} contains assertions validated by the compiler
# Uses existing function/type syntax; no new BNF rules
predicate ::= identifier ':' params '->' 'Type' '=' '{' assertions '}'
```

**The arguments of a predicate application must have a compile-time constant shape**—a literal, a
variable (bound by name), a type application (recursively extracted), or a
**compile-time-referencable function reference** (a function name). If an argument cannot be
converted to a constant expression, report **E1092**; if the number of arguments does not match the
predicate declaration's number of formal parameters, report **E1093**. Arguments are bound
positionally to the formal-parameter list, and predicate arity is determined by the
declaration—`Positive(x)` is unary, `IsMax(T, arr, result)` is ternary, and `Terminates(m)` and
`Terminates(FnType, m)` are unary and binary, respectively. Refinement constraints are **never
silently discarded** (previously, an unconvertible argument caused the constraint to disappear
silently, allowing a binding that violated the constraint to pass silently).

**New syntactic concept: return-value formal parameter**—in `-> (name: Type)`, `name` is the
return-value formal parameter.

The return-value formal parameter is the **only syntactic concept** that YaoXiang introduces on top
of its existing function syntax. Its semantics are:

- The value of `name` is supplied by a `return` statement
- `name` exists only in the type signature and is referenced by postcondition predicates such as
  `-> (result: IsMax(T, arr, result))`
- `name` does not enter the function body's scope and does not appear at the call site
- The return-value formal parameter is **optional**—without a postcondition, the signature is
  identical to that of an ordinary function (`-> Int`), with no additional burden

The reason for introducing it is that a postcondition must refer to “the value the function is about
to return.” Without a return-value formal parameter, the compiler could only expose the return value
through a special rule, such as an implicit variable `$result` or `__retval__`, allowing predicates
to refer to it. A return-value formal parameter makes this reference explicit—it is simply a formal
parameter whose value is supplied by `return` rather than by the caller.

A **proof function** is not a new concept—it is simply a YaoXiang function whose return type is the
asserted proposition. When the compiler returns Unproven, the programmer supplies a proof function,
and the type checker verifies it in exactly the same way it verifies the return type of any
function. No new syntax, keywords, or rules are needed.

> **The boundary between the two.** In the correctness domain (predicate Unproven), the proof is
> supplied in the **body**: write a function whose return type is the proposition to be proved. In
> the termination domain (§6.9), the measure is supplied in **type position**: write
> `Terminates(measure)` as the type of a binding or function signature. The former means “write a
> proof for a proposition that cannot be proved,” while the latter means “explicitly declare a
> measure that exploration cannot discover”—the mechanism is the same in both cases (a
> refinement-type application), but the location differs. The termination domain **does not
> require** a `_proof` function.

### Type-System Impact

- **Type universe**: compile-time predicates reside at the Type₂ layer—functions accepting values
  and returning Type, at the same level as type constructors
- **Generics interaction**: compile-time predicates may take generic parameters, such as
  `NonEmpty: (T: Type) -> (arr: Array(T)) -> Type`
- **Ownership interaction**: expressions in compile-time predicates obey ownership rules and may
  only be read, not written
- **Type inference**: compile-time predicate parameters participate in HM type inference

### Runtime Representation

Compile-time predicates are handled at runtime according to the `dispatch` outcome:

- **CompileTime mode** (all free variables are known at compile time): after the proof succeeds, the
  witness token is completely erased. For `Positive: (x: Int) -> Type = { x > 0 }`, the runtime
  representation of the parameter `b: Positive(5)` is simply `Int`. The refinement condition
  `{ 5 > 0 }` has already been validated and is erased.
- **Runtime mode** (some free variables come from runtime): retain the runtime check—execute a Bool
  check at the value level and inject the result into the flow-sensitive assumption set Γ. See §11,
  the `dispatch` routing pipeline, and §13, the erasure-model clarification.

Placing a compile-time predicate in a type position such as `f(x: Positive(x))` does not create a
wrapper type or allocate additional memory. However, if `x` comes from runtime input, a runtime Bool
check **is** inserted.

**Interaction constraint with `ref`**: compile-time predicates may refer only to immutable borrows
or values whose ownership has already been transferred. A compile-time predicate that refers to a
mutable borrow cannot guarantee at compile time that the validation result will still hold at
runtime—such usage directly produces a compile error.

### Compiler Changes

1. **Parser**: compile-time predicates use standard function syntax and require no additional
   parsing rules
2. **Compile-time proof pipeline**: a unified Proved/Disproved/Unproven return interface with
   automatic strategy selection
3. **SMT acceleration module**: an SMT-LIB 2.6 translation layer, with Z3 as the default backend and
   CVC5 as an alternative
4. **Type-checker kernel**: implementation of inference rules—structural equivalence, βδι-reduction,
   and universal-quantifier introduction/elimination. This is the sole trust root; both SMT and
   programmer proofs are verified through it
5. **Verification-condition generation**: WP/SP calculus + loop-invariant proof obligations
6. **Error reporting**: counterexample formatting + unresolved-proposition reports + source-location
   correlation

### Backward Compatibility

- ✅ Code that does not use compile-time predicates remains completely unchanged
- ✅ Compile-time predicates have zero runtime overhead in CompileTime mode; Runtime mode retains
  only the necessary Bool checks
- ⚠️ RFC-022's `//!` syntax is no longer supported—but 022 was never implemented, so there is no
  migration burden

## Trade-offs

### Advantages

- **Full realization of the Curry-Howard correspondence**: types are propositions, programs are
  proofs, `name: Proposition = Proof`
- **Uniformity**: compile-time predicates and ordinary functions use exactly the same syntax, with
  no conceptual split
- **SMT transparency**: programmers do not need to know SMT exists; the mental model remains
  consistent with type checking
- **Incremental adoption**: adoption can begin with a single compile-time predicate and expand
  gradually
- **Minimal runtime overhead**: zero overhead in CompileTime mode; Runtime mode retains only the
  necessary Bool checks

### Disadvantages

- **Compilation time**: SMT solving increases compilation time, but hard budget limits keep the
  upper bound manageable
- **Automatic-proof boundary**: complex predicates beyond first-order linear arithmetic may require
  the programmer to write a proof function. This is not a language defect—it is an inevitable
  consequence of the halting problem. The compiler honestly reports Unproven rather than falsely
  reporting True/False
- **Learning curve**: writing effective compile-time predicates and proof functions requires
  understanding the basic intuition of the Curry-Howard correspondence
- **Implementation complexity**: unifying the compile-time proof pipeline requires careful design

### Risk Mitigation

- Hard limits on the SMT-solving budget (10,000 steps / 100ms / instantiation depth 3); exceeding
  the budget returns Unproven
- Dependent-type pre-reduction: deterministic value computations are handled first, and SMT handles
  only the nondeterministic remainder
- Unproven is not a dead end: write a proof function in the correctness domain (its return type is
  the proposition), or supply a measure in type position in the termination domain (§6.9)—both are
  verified by the type checker
- Incremental verification: only changed modules are verified
- Clear error messages + counterexample display + budget-consumption report + unresolved
  proposition + suggestions (when the compiler can provide them)

## Alternatives

| Option                                                             | Why It Was Not Chosen                                                                                                                                                                              |
| ------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| RFC-022: comment-based specifications with `//!`                   | Splits specifications and types, violating the Curry-Howard correspondence                                                                                                                         |
| Separate specification files, such as CVL                          | Separates specifications from code and increases maintenance cost                                                                                                                                  |
| Runtime assertions only                                            | Cannot provide static correctness guarantees                                                                                                                                                       |
| External proof assistants, such as Coq                             | Creates a disconnect from the compiler and requires a separate proof language and trust boundary. YaoXiang's choice is that the proof is YaoXiang code and the type checker is the sole trust root |
| **This proposal: compile-time predicates as first-class citizens** | ✅                                                                                                                                                                                                 |

## Implementation Strategy

### Phase Breakdown

| Phase       | Contents                                                                                                                                                                 |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Phase 1** | Compiler kernel: structural equivalence + βδι-reduction + universal-quantifier introduction/elimination. Supports simple arithmetic predicates (`x > 0`, `arr.len > 0`)  |
| **Phase 2** | SMT-LIB translation layer + Z3/CVC5 integration. The pipeline returns Proved/Disproved/Unproven. Supports programmer-written proof functions when the result is Unproven |
| **Phase 3** | Loop-invariant VC generation + termination checking (four measure-exploration strategies + explicit `Terminates` measures, §6 and §6.9)                                  |
| **Phase 4** | Incremental verification + caching + IDE support                                                                                                                         |

### Dependencies

- RFC-010: Unified Type Syntax—compile-time predicates are based on `name: type = value`
- RFC-011: Generic System—compile-time predicates may take generic parameters
- RFC-009: Ownership Model—expressions in compile-time predicates obey ownership rules

## Open Questions

- [x] **Z3 availability for the wasm32 target**: there is no Z3 under wasm (all SMT code is excluded
      with `cfg(not(target_arch = "wasm32"))`). Scope and degradation direction: - `ownership.rs`'s
      `smt_cut` always returns `false` → back-edge crossing → **conservative rejection** (the sound
      direction, merely less narrowing) - Termination-checking Strategy 1 and predicate levels 2b/3
      are skipped entirely

  **Correction (2026-10-01)**: the original text stated that these paths were “currently
  **unreachable**” (on the grounds that production never populated `predicate_defs`, the parser
  never produced `MonoType::Refined`, and production `with_solver` was never invoked). That is no
  longer true—all three have been implemented (predicate-definition registration #377-3;
  refinement-type normalization; solver injection into the production pipeline). Therefore, the
  difference between wasm and native is no longer limited to the precision of `smt_cut`:

  | Path                                           | native   | wasm       | Degradation direction                 |
  | ---------------------------------------------- | -------- | ---------- | ------------------------------------- |
  | Termination-checking explicit-measure decision | Decided  | `Unjudged` | E4022 is not reported (fewer reports) |
  | Well-foundedness decision                      | Decided  | `Unjudged` | E4022 is not reported (fewer reports) |
  | SMT entailment level for refinement predicates | Executed | Skipped    | Falls back to `Unproven`              |

  All of these are **conservative** directions (less narrowing and fewer reports) and do not affect
  soundness. To achieve identical precision, the options and costs are described in issue #376
  (conclusion: prioritize a JS-side Z3 instance rather than linking Z3 into the main wasm build—the
  latter would require replacing the emcc build system and would grow the artifact from 3 MB to ~20
  MB).

- [x] **SMT solver selection**: Z3 by default (MIT license, the broadest validation). CVC5 is an
      SMT-LIB-compatible alternative, switchable through a compiler flag. The compiler's internal
      translation target is the SMT-LIB 2.6 standard format—SMT-LIB is the abstraction layer, with
      no custom generic solver interface.
- [x] **Specific solver-budget values**: 10,000 steps / 100ms / quantifier instantiation depth 3.
      Fixed inside the compiler, with no knob. If practical use cases demonstrate that these values
      are insufficient (rather than the user having made an error), they may be adjusted later.
- [x] **Quantifier support range**: quantifier order is not restricted at the language level.
      Compile-time predicates accept Type arguments—Type includes function types—so higher-order
      quantifiers are a natural consequence of the type system and require no special syntax. The
      SMT solver can automatically decide first-order quantifiers (forall/exists, including
      alternating nesting, limited by budget depth 3). For higher-order quantifiers, SMT returns
      Unproven and the compiler prompts, “This predicate is outside the automatic proving range;
      please provide a proof function.” The programmer writes a YaoXiang function whose return type
      equals the proposition—the type checker verifies that function. No external export, AI, or
      interactive proof mode is required. Everything is YaoXiang code, and everything is verified by
      the type checker.
- [x] **Counterexample formatting**: source variable names are used directly as SMT variable names,
      with a module prefix to avoid collisions. When a Z3 model is returned, variables are looked up
      by name. Output format: variable name = concrete value + source location +
      predicate-definition location. No complex mapping layer is implemented.
- [x] ~~**Interaction between compile-time predicates and the `ref` smart pointer?**~~ → Decision:
      compile-time predicates permit only immutable borrows or values whose ownership has already
      been transferred. A value behind a mutable borrow may not appear in a compile-time predicate.
- [x] **Extending the `forall` predicate violation-count measure to non-adjacent operations?** → No
      extension. The current coverage (adjacent swaps, adjacent moves) is complemented by Strategy 1
      (a linear rank function)—quicksort's outer interval contraction is covered by Strategy 1, and
      heap sort is covered by Strategy 1 (the array-index pattern). A loop that no strategy can
      prove terminates produces a direct compile error—this is the hard-safety philosophy, not a
      defect. If a practical, non-academic scenario arises in which none of the four strategies can
      cover the algorithm, the issue will be reconsidered. → **Reconsideration triggered
      (2026-09-14, #318)**: non-structural recursion (gcd-like non-direct decrease, mutual
      recursion, merge partitioning) is precisely the kind of practical scenario anticipated by this
      clause—the template sequence for measure exploration cannot handle it automatically, and it
      cannot be rewritten into an analyzable iteration pattern without harming readability.
      Conclusion: the hard-safety philosophy remains unchanged (an incorrect measure is still
      rejected by an SMT counterexample; humans can only fail to prove termination, not prove it
      falsely), but “reject when exploration fails” is relaxed to “allow the programmer to supply an
      explicit measure in type position when exploration fails”—see §6.9.
- [x] **Combinatorial explosion in linear-rank-function enumeration**: the candidate-enumeration
      limit is 3 bounded variables. At ≤3, enumerate all linear combinations and validate each with
      SMT. At >3, try only single-variable measures (`v_i`, `u_i - v_i`), and report a compile error
      if they fail—prompting the programmer, “This loop has more than 3 bounded variables, so the
      compiler cannot automatically synthesize a multivariable measure.” This is not an engineering
      compromise—it forces programmers to write simpler loops.

## References

- [RFC-010: Unified Type Syntax](010-unified-type-syntax.md)
- [RFC-011: Generic System Design](011-generic-type-system.md)
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
