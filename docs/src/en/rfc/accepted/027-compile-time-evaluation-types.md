---
title: 'RFC-027: Compile-Time Predicates and Unified Static Verification'
status: 'Accepted'
author: 'Chenxu'
created: '2026-06-07'
updated: '2026-10-03'
impl_status: 'in-progress'
impl_detail:
  'Phases 1-2 are complete, Phase 3 is partially complete, and Phase 4 is partially complete. The
  unified assert/Assert proposal has been fully implemented across all 6 phases (#157-#162 closed):
  Never type, IsTrue bridge, flow-sensitive Γ + kill set, type-level recursion, and
  universe-stratification weak checking. The dispatch **semantics** in §11 have been implemented
  through call-site obligations (`checker.rs` directly calls `check_predicate` in three places:
  binding position / call-site argument / return position), but the **standalone
  `layers/dispatch.rs` module has been deleted**—it had zero production call sites, and its handling
  of `Unproven` (downgrading it to a W1080 warning) conflicted with the production implementation
  (always reporting an error) (#377-2).'
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
> - [RFC-024: Concurrency Model Based on `spawn` Blocks](024-concurrency-model.md)
>
> **Supersedes**:
> [RFC-022: Hoare Logic Static Verification Support (Specification Comments and Specification Types)](../deprecated/022-hoare-logic-static-verification.md)
> — Deprecated

## Abstract

This RFC proposes introducing **compile-time predicates** into YaoXiang as first-class citizens,
unifying all compile-time static verification into a single **proof pipeline**. A compile-time
predicate is not an externally attached specification comment—it is a function. A function that
returns a Type can be used in a type position; the compiler calls it at compile time and checks its
return value. Types are propositions, and compile-time evaluation is proof.

**Core thesis**: the sole job of type checking at compile time is to construct and verify proof
terms. Type equality, token conflicts, dependent type reduction, compile-time predicate evaluation,
and Hoare-logic implication are all different type-checking tasks within the same compile-time proof
pipeline, sharing one pipeline. The SMT solver is an acceleration module for the type checker, not
an independent trust boundary. When the compiler returns Unproven, the programmer writes a YaoXiang
function as the proof—the type checker verifies it in exactly the same way that it verifies the
return type of any function. Everything is YaoXiang code, and everything is verified by the type
checker.

## Motivation

### Why deprecate RFC-022?

RFC-022 designed specifications as `//!` comments:

```yaoxiang
max: (T: Ord) -> ((arr: Array(T, n)) -> T) = {
    //! requires: NonEmpty(n) = n > 0          ← This is a comment independent of the type
    //! ensures: ExistsMax(result, arr[0..n])   ← This is a comment independent of the type
}
```

This makes a fundamental mistake in the Curry–Howard correspondence: **it splits specifications and
types into two layers**. Comments are not types. Comments do not participate in type checking.
Comments embody the mental model of an “external tool.”

The whitepaper states this plainly:

> “No `//!` comments. No separate specification language. Everything is within the type system.”

### Current Problems

- RFC-022's `//!` comments are an external syntax layer independent of the type system
- Specification types and ordinary types form two separate systems, creating conceptual redundancy
- The split between verification in Debug builds and ignoring it in Release builds undermines
  uniformity
- SMT solvers are traditionally perceived as external tools—YaoXiang instead builds one into the
  type checker as an acceleration module
- Type checking, borrow checking, compile-time predicate checking, and macro expansion each follow
  different paths

### The Correct Mental Model

Type checking can be abstracted as a function:

```
verify : Program → Proved | Disproved(Model) | Unproven
```

All compile-time checks—simple type matching, borrow-conflict detection, and compile-time predicate
verification—are subtasks of this function. They share one proof pipeline and differ only in the
complexity of their proof terms and construction strategies.

When the compiler returns Unproven, the programmer provides a proof function whose return type is
the proposition to be proved. The type checker verifies it. This is the same operation as ordinary
type checking.

## Proposal

### 1. `{}` Is the Proof Space: Types Are Assertions, Verification Is Type Checking

A YaoXiang `{}` is the compile-time proof space. Everything inside it is an assertion, and the
compiler guarantees that every item is True—either by proving it automatically or by accepting a
proof function supplied by the programmer.

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
#          Parameter in the signature              Only assertions inside {}
#          The compiler verifies x > 0 when it calls it at compile time

List: (T: Type) -> Type = { data: Array(T) }
#      ^^^^^^^^              ^^^^^^^^^^^^^^^
#      Parameter in the signature           The compiler verifies type_of(T) == Type and type_of(data) == Array(T)
```

The same pattern applies: `name: (params) -> Type = { assertions }`. The compiler makes no
distinction between “type assertions” and “value assertions”—both are evaluation targets in the
proof pipeline.

**Loop invariants do not need to be written separately. Type annotations on variables are
Floyd–Hoare invariants.**

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # The annotation references i—telling the compiler that s's type depends on i
    mut i: UpTo(arr.len) = 0     # During initialization i=0, verification: 0 == sum(arr[0..0]) → True
    while i < arr.len {
        s += arr[i]  # The compiler verifies: s_new == sum(arr[0..i+1])
        i += 1       # Changing i triggers re-verification of s's dependency: s satisfies SumUpTo(arr, i_new)
    }
    return s  # s: SumUpTo(arr, arr.len) = sum(arr[0..arr.len])
}
```

The compiler generates one verification condition for the loop body—the inductive hypothesis (type
annotation) → assignment operation → whether the new value satisfies the type annotation. Once the
proof pipeline establishes the inductive step, all iterations are covered automatically. No
`: decreases` is needed, no `: Invariant` is needed, and no inductive proof is needed—the compiler
decomposes induction into local VCs for each assignment.

### 2. Preconditions/Postconditions: Compile-Time Predicates on Parameter and Return Types

RFC-022's `//! requires`/`//! ensures` are abandoned. Compile-time predicates become type
annotations on parameters or return values.

**The parameter side is a function call.** A compile-time predicate is a function that returns Type,
so its use on the parameter side is a call—just like `factorial(5)`. The return-value side
introduces a new concept: the return-value formal parameter.

```yaoxiang
# Precondition: explicitly call a compile-time predicate in the parameter type
Positive: (x: Int) -> Type = { x > 0 }

divide: (a: Int, b: Positive(b)) -> Int = a / b
#                       ^^^^^^^^^^  b is the current formal parameter name, passed to Positive
#                       The compiler extracts the actual argument at the call site, substitutes it for b, and verifies Positive(actual argument)
#                       Example: divide(10, 2) → verify Positive(2) = { 2 > 0 } → True
#                       Example: divide(10, 0) → verify Positive(0) = { 0 > 0 } → False → compilation error

# Postcondition: return-value formal parameter + compile-time predicate
IsMax: (T: Ord, arr: Array(T), result: T) -> Type = {
    forall j in 0..arr.len: result >= arr[j]
}

NonEmpty: (arr: Array(T)) -> Type = { arr.len > 0 }

max: (T: Ord) -> ((arr: NonEmpty(arr))) -> (result: IsMax(T, arr, result)) = {
#                                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
#                                            result is the return-value formal parameter, and its value is supplied by return
#                                            The compiler substitutes the returned value at the return point and verifies the postcondition
    candidate = arr[0]
    for i in 1..arr.len {
        if arr[i] > candidate { candidate = arr[i] }
    }
    return candidate
}
```

**Key rules**:

- **Parameter side**: `b: Positive(b)` — `b` is the current formal parameter name and is passed to
  `Positive` as an argument. This uses ordinary function-call syntax, with no implicit behavior.
- **Return side**: `-> (result: IsMax(T, arr, result))` — `result` is the return-value formal
  parameter, and its value is supplied by a `return` statement. `result` exists only in the type
  signature and is referenced only by predicates; it does not enter the function-body scope and does
  not appear at the caller.
- **The return-value formal parameter is optional**: without a postcondition, it is omitted and the
  signature is identical to that of an ordinary function (`-> Int`).
- **Uniformity**: parameter and return-value formal parameters are the same
  concept—`formal parameter name: predicate call(actual argument)`—and differ only in whether the
  value is supplied by the caller or by `return`.

> **Implementation note (2026-10-02)**: recognition of a return-position formal parameter follows
> **the declared name strictly**—the `r` in `-> (r: P(...))` is the sole return-value formal
> parameter. Other free variables in the constraint that are **not in scope** are no longer
> substituted with the return value; instead, an undefined-identifier error is reported (observed:
> `g: () -> (r: Eq2(m, r)) = { return 7 }` (`m` is undefined) →
> `error [E1001] Unknown variable: 'm'`). Predicate-call arguments are substituted into the
> constraint **as actual arguments**: `P(r + 100)` becomes “return value + 100,” not “return value.”
>
> **Known gap (2026-10-02)**: **multi-parameter predicates with symbolic arguments in return
> position are not yet supported**. Triggering form:
> `f: (b: Int) -> (r: SumUpTo(b, r)) = { return b * 2 }`
> (`SumUpTo: (n: Int, s: Int) -> Type = { s == n * 2 }`) →
> `error [E2031] … the refinement type constraint of 'r': SumUpTo(b, (b * 2)) cannot be statically proven`.
> Workaround: change the argument to a compile-time-foldable literal, such as `SumUpTo(3, r)`, or
> supply a proof function for the predicate that returns `Type`. Resolution condition: the proof
> kernel must support argument substitution for “multi-parameter predicates + symbolic arguments.”
> See [RFC-027a](../accepted/027a-termination-explicit-measure.md) §Implementation Record
> (2026-10-02, this hardening round).

### 3. Path-Condition Propagation: Compile-Time Verification of Runtime Values

When a compile-time predicate is used in a binding position, its arguments are explicitly supplied
by the programmer. When a runtime value enters a refinement-type parameter, the compiler completes
verification through path-condition collection and SMT implication checking—there is no need for the
programmer to pass a proof explicitly.

#### 3.1 Explicit Function Calls

When a compile-time predicate is used in a binding position, its arguments are explicitly supplied
by the programmer—this is an ordinary function call, with no implicit behavior.

`Positive: (x: Int) -> Type = { x > 0 }` is a compile-time predicate constructor. When it appears in
a binding position (parameter declaration, variable declaration, or return type), the programmer
explicitly supplies the name of an already-bound variable:

```yaoxiang
b: Positive(b)
// b has already been declared as the current formal parameter, and Positive(b) is a function call
// After normalization: b: { b > 0 }
```

The compiler does not need to fill in arguments implicitly—`b: Positive(b)` is a function call, just
like `f(5)`. `b` is bound as a parameter name, and its type annotation `Positive(b)` references `b`
itself. This is the standard dependent-type pattern, not an implicit expansion rule.

**Unification with RFC-010 `self`**: RFC-010 establishes that `self` is not a keyword, merely a
conventional parameter name (“writing `p`, `this`, or `x` has exactly the same effect”).
`b: Positive(b)` uses the same mechanism—the parameter name can be referenced in a type annotation.
`self` appears in `self: Point`, while `b` appears in `b: Positive(b)`; in both cases, the type
annotation references the parameter itself. The only difference is the complexity of the annotation;
the mechanism is identical—after a name is bound, the type may depend on that name.

Return types use explicit function calls in the same way:

```yaoxiang
Sorted: (arr: Array(T)) -> Type = { forall i in 0..arr.len-1: arr[i] <= arr[i+1] }

sort: (arr: Array(T)) -> (result: Sorted(result)) = { ... }
//                        ^^^^^^^^^^^^^^^^^^^^^^^
//                        result is the return-value formal parameter, and Sorted(result) is a function call
//                        The compiler substitutes the returned value for result at the return point and verifies Sorted(return value)
```

The same applies to local-variable declarations:

```yaoxiang
let x: Positive(x) = 5
// x is bound to 5, Positive(5) → { 5 > 0 } → True → passes

// let y: Positive(y) = 0
// y is bound to 0, Positive(0) → { 0 > 0 } → False → compilation error
```

#### 3.2 Path-Condition Collection

When a runtime value appears in a conditional branch, the compiler automatically collects path
conditions to form an **assumption set** for the current scope. These assumptions participate in
verification as background knowledge for compile-time Boolean evaluation.

```yaoxiang
if y > 0 {
    // The compiler automatically obtains the assumption in this branch: { y > 0 }
    let result = divide(x, y)
    // Verification condition: (y > 0) ⇒ (y > 0)
    // The proof pipeline determines that the implication holds → Proved
} else {
// Assumptions for this branch: { !(y > 0) }
// To call divide(x, y), the verification condition is !(y > 0) ⇒ y > 0
    // The proof pipeline determines that the implication does not hold → Disproved
}
```

This is not a special pattern hard-coded into the compiler—it is the natural behavior of the
compile-time proof pipeline. At every type-checking call site, the pipeline receives:

```
{background assumptions} ⇒ {verification goal}
```

The proof pipeline determines implication. Proved → pass; Disproved → compilation error with a
counterexample; Unproven → compilation error with the unresolved proposition. Background assumptions
come from the path conditions at the current program point.

#### 3.3 Assumption Stack

While analyzing control flow, the compiler maintains an assumption set for each basic block:

- **if-guard**: `if y > 0` → push `y > 0` onto the true branch, and push `!(y > 0)` onto the false
  branch (if an else is present)
- **match pattern**: `if let Some(v) = opt` → push `opt == Some(v)` within the branch
- **Logical conjunction**: `if x > 0 and y < 10` → push `x > 0` and `y < 10` within the branch
- **Function precondition**: when calling `divide(a, b)`, evidence that `b` satisfies `Positive`
  must come either from the current assumptions or from the actual argument's own refinement-type
  annotation (`b` is already annotated as `Positive`, so its type carries `b > 0`)
- **Assignment**: when evaluating `let z = y`, existing refinement conditions on `y` are propagated
  to `z`

All assumptions enter the compile-time proof pipeline. When execution takes the SMT acceleration
path, they are translated into SMT-LIB background assertions.

#### 3.4 Compilation Error When Static Evidence Is Missing

If the programmer directly writes:

```yaoxiang
divide_user_input: (x: Int, y: Int) -> Int = divide(x, y)
```

there is no `y > 0` assumption at the current program point, and the actual argument `y` itself has
no `Positive` type annotation. The verification condition is:

```
{} ⇒ { y > 0 }
```

The pipeline returns `Disproved` (the implication does not hold) → compilation error:

> Unable to prove that argument `b` satisfies `Positive` in the call to `divide`. `y` comes from the
> function input and has no proven bound. Consider guarding the call with an if branch:
> `if y > 0 { divide(x, y) }`.

YaoXiang does not permit a runtime value to enter a refinement-type parameter without static
evidence. This is not a limitation—it is the core of the hard-safety philosophy. Code that the
compiler cannot prove statically must not compile.

#### 3.5 Relationship to the Unified Pipeline

Path-condition propagation is not an additional mechanism. It is a direct extension of the
compile-time proof pipeline into control-flow analysis:

| Stage                             | Responsibility                                                                                                                                                          |
| --------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Path-condition collection         | During control-flow analysis, the compiler annotates each basic block with an assumption set                                                                            |
| Verification-condition generation | When it encounters a type constraint that must be verified, it combines path conditions with actual-argument type information                                           |
| Proof-pipeline evaluation         | Compiler kernel → SMT acceleration → produces Proved / Disproved / Unproven                                                                                             |
| Result                            | `Proved` → pass; `Disproved` → compilation error + counterexample; `Unproven` → compilation error + unresolved proposition (the programmer may supply a proof function) |

There are no new components and no special rules. Path conditions are background knowledge for the
proof pipeline—sharing the same pipeline and budget system as type equalities and borrow
constraints.

### 4. The Compile-Time Proof Pipeline

All compile-time checks share one pipeline. Its central operation is **type checking**—checking
whether the type of a proof term is equal to the proposition being proved. Everything is type
checking.

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

#### 4.1 Proof Results: A Three-Valued Algebra

Compile-time evaluation returns three possible results—an inevitable consequence of the halting
problem and a natural partition of proof theory:

```
eval_compile_time : BoolExpr → Proved | Disproved(Model) | Unproven
```

- **Proved** → halts; the proof term has been constructed and passes type checking. Compilation
  continues.
- **Disproved(M)** → halts; a counterexample M exists. Compilation error + counterexample + source
  location.
- **Unproven** → no proof was constructed within the given resource limit. Compilation error +
  unresolved proposition + budget-consumption report.

**Unproven ≠ False.** When the compiler says “I cannot prove it,” that is not equivalent to the
proposition being false—it only means the proposition exceeds the compiler's current ability to
prove it automatically. This is honesty, not a defect.

A hard budget limit is the engineering solution to the halting problem. No knob is provided—exposing
one would amount to asking the user “Do you think your program will halt?” The user does not know,
and neither does the compiler.

#### 4.2 After Unproven: The Programmer Writes the Proof

When the compiler returns Unproven, the programmer can write a **proof function**—a YaoXiang
function whose return type is the proposition to be proved. The type checker verifies this function
through exactly the same mechanism it uses to verify `add(a, b): Int`.

```
命题  = 类型
证明  = 程序（该类型的一个值）
验证  = 类型检查（唯一的信任根）
```

The SMT solver is not an independent trust boundary—it is an **acceleration module for the type
checker**. SMT helps find proofs, but the type checker always verifies the proof. When SMT returns
`unsat`, the compiler reconstructs the result as a proof term that the type checker can verify. If
reconstruction fails (because the SMT reasoning steps exceed the compiler kernel's inference rules),
it falls back to Unproven, and the programmer can write a proof function manually.

```yaoxiang
# Proposition: a refinement property the compiler cannot prove automatically
FirstIsMin: (T: Ord, arr: Sorted(T)) -> Type = {
    forall i in 0..arr.len: arr[0] <= arr[i]
}

# Proof: the programmer writes a function whose return type is the proposition above
# The type checker verifies this function—exactly as it verifies add(a,b): Int
first_is_min: (T: Ord, arr: Sorted(T)) -> FirstIsMin(T, arr) = {
    # The compiler verifies here: the type of the function body = FirstIsMin(T, arr)
    ...
}
```

No AI is needed, no export to Coq is needed, and no new concepts are needed. **A property that
cannot be proved automatically at compile time → the programmer writes the proof in YaoXiang code →
the type checker verifies it.** The entire process is a smooth continuum—the compiler handles the
easy proofs, leaving the difficult ones to the programmer.

#### 4.3 Layered Dependencies Within the Pipeline

The evaluators above share one interface but have an evaluation order. Type equality is a
prerequisite for all subsequent analysis; ownership/token checking depends on type information; and
refinement-predicate verification depends on the results of the preceding two layers. The compiler
evaluates layer by layer, and expressions that fail at a lower layer do not proceed to higher
layers—avoiding wasted solver budget on programs with type errors.

```
求值顺序（同一管道，分层调度）
├── 第 0 层：类型等式（T1 == T2）
│   └── 结构合一 → 失败则后续无意义，直接返回 Disproved
├── 第 1 层：所有权/令牌冲突
│   └── 流敏感活性分析 → 失败则内存安全不成立，直接返回 Disproved
└── 第 2 层：精化谓词/霍尔蕴含
    └── 编译器自身 → SMT 加速 → 得出 Proved / Disproved / Unproven
```

Every layer still returns `Proved/Disproved/Unproven`, sharing one interface and one budget system.

### 5. Unification of the Three Function Layers

| Layer                  | Execution Time | Input        | Output | Example                                        |
| ---------------------- | -------------- | ------------ | ------ | ---------------------------------------------- |
| Value-level function   | runtime        | Values       | Value  | `add: (a: Int, b: Int) -> Int = a + b`         |
| Type constructor       | compile-time   | Types/values | Type   | `List: (T: Type) -> Type = { data: Array(T) }` |
| Compile-time predicate | compile-time   | Values       | Type   | `Positive: (x: Int) -> Type = { x > 0 }`       |

All use the same `name: type = value` syntax. Compile-time predicates and type constructors follow
the same compile-time proof pipeline—`{}` is the proof space.

### 6. Loops: Floyd–Hoare Verification-Condition Generation

Loops do not need a separate `: Invariant(...)` or `: decreases(...)` annotation. Compile-time
predicate type annotations on variables define Floyd–Hoare-style assertions—the compiler derives
verification conditions from those annotations and asks the proof pipeline to check whether every
assignment preserves the type.

The central mechanism is that each assignment corresponds to a Hoare triple `{P} x := e {Q}`, whose
verification condition is `P ⇒ Q[e/x]`. The compiler generates one verification condition for the
loop body—once the proof pipeline establishes the inductive step, all iterations are covered
automatically.

```yaoxiang
SumUpTo: (arr: Array(Int), i: Int) -> Type = { s: Int; s == sum(arr[0..i]) }
UpTo: (n: Int) -> Type = { i: Int; 0 <= i <= n }

sum: (arr: Array(Int)) -> Int = {
    mut s: SumUpTo(arr, i) = 0   # The annotation references i; during initialization i=0, verification: 0 == sum(arr[0..0]) → True
    mut i: UpTo(arr.len) = 0     # Verification: 0 <= 0 <= arr.len → True
    while i < arr.len {
        # The compiler generates one VC for the loop body. Premises: s satisfies SumUpTo(arr, i), and i satisfies UpTo(arr.len).
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
        #   Changing i → s's type annotation references i in the dependency graph → triggers re-verification
        #   New verification target: s satisfies SumUpTo(arr, i_new)
        #   That is, s == sum(arr[0..i_new]), which is guaranteed by the preceding step → Proved
        s += arr[i]
        i += 1
    }
    return s  # At this point s: SumUpTo(arr, arr.len), that is, s == sum(arr[0..arr.len])
}
```

The loop invariant is the type annotation on the variable—the programmer writes types, and the
compiler checks the inductive step. The compiler does not need to “discover” invariants or “perform
induction automatically”; it decomposes an inductive proof into local verification conditions for
each assignment operation and hands them to the proof pipeline divide and conquer.

#### 6.1 Dependency Tracking: Dependent Types on Mutable Variables

The mechanism above assumes that the compiler knows that `s`'s type annotation `SumUpTo(arr, i)`
references `i`—when `i` changes, `s`'s type constraint changes with it. This requires the compiler
to maintain a **type-dependency graph between variables**.

**Data structure**:

```
TypeDepGraph: Map<VarName, Set<VarName>>
# The key is the variable being depended on, and the value is the set of variables whose type annotations reference it
# Example: { i: {s}, j: {s, t}, ... }
```

**Construction**: while processing `mut v: Pred(... x ...) = init`, the type checker parses
free-variable references in the arguments of `Pred(...)`. If an argument references another mutable
variable `x` in the current scope, the dependency graph records `x → v`.

**Trigger**: when the depended-on variable `x` is assigned, the compiler:

1. Looks up all variables `{v₁, v₂, ...}` in the dependency graph that depend on `x`
2. For each `v`, generates a verification condition:
   `does v's current value satisfy the updated type Pred(... x_new ...)?`
3. Sends the VC into the proof pipeline

**Assignment-order sensitivity**: dependency tracking naturally enforces the correct assignment
order. Using `SumUpTo(arr, i)` as an example:

```yaoxiang
# Correct order
s += arr[i]   # s_new satisfies SumUpTo(arr, i+1)
i += 1        # Changing i → re-verify that s satisfies SumUpTo(arr, i_new) → True

# Wrong order—the compiler rejects it
i += 1        # Changing i → re-verify that s satisfies SumUpTo(arr, i_new)
              # s has not yet been updated, so s_old == sum(arr[0..i_old]) ≠ sum(arr[0..i_new])
              # → Compilation error: variable s does not satisfy type SumUpTo(arr, i_new)
s += arr[i]   # Unreachable
```

**Combined dependencies**: one variable may depend on several variables. The type annotation
`{ v: Int; v == x + y }` depends on both `x` and `y`—a change to either triggers re-verification.

**Relationship to the proof pipeline**: dependency tracking is the trigger for VC generation, not an
independent verification mechanism. It answers “When must a VC be generated?”—the proof pipeline
answers “Does the VC hold?”

### 7. Termination Checking

**Scope: refinement types.** Termination is not an independent switch; it is part of a
**verification mode**: once a type is refined (`Refined { base, constraint }`), the computation
annotated by that type enters verification mode, and that mode requires termination. Ordinary types
without refinement do not enter verification mode and generate no termination obligations.

This yields two corollaries:

- **Loops**: a bare `while` does not enter verification mode; a loop enters verification mode when
  its measure variable has a refinement annotation, such as `i: UpTo(n)`, and termination must then
  be proved.
- **Recursion**: a function enters verification mode when its signature contains a refinement (a
  refined parameter or a return type containing a refinement), and the measure must strictly
  decrease at every recursive call site within that mode.

The mode is fully automatic by default: the compiler first explores for a measure automatically, and
passes when it can prove termination. If exploration fails and no explicit measure has been
supplied, compilation fails. There is no **annotation-syntax** escape hatch—the measure and
termination proposition are both written in type position, introducing no new syntax such as
`decreases`. See §6.9 for the forms of measures and explicit fallbacks.

#### 6.1 Design Principles

The compiler automatically extracts the information required for a termination proof from two
places:

1. **Variable type annotations**: bounds in refinement types (for example, `UpTo(n)` provides the
   upper bound `n` and lower bound `0`)
2. **Loop-body operations**: the operations applied to variables on each iteration

The compiler tries four measure-synthesis strategies in priority order and stops when it finds one.
The four strategies form a **restricted template sequence for measure exploration**. Their input
consists of refinement constraints (strategies 1–4 all start from “a variable of bounded type”),
rather than “compile-time-evaluated code.” They are the automatic and manual sides of the same
mechanism as the explicit measures in §6.9.

> **Measure exploration is exploration, not inference.** Exploration enumerates only templates
> (linear rank, violation count, bounded patterns, multiplicative scaling) and does not guarantee
> that a solution exists—general measure inference is generally undecidable (it reduces to the
> halting problem). Cases outside the templates must therefore be handled by an explicit measure
> supplied by the programmer (§6.9); otherwise compilation fails.

#### 6.2 Strategy 1: Automatic Synthesis of Linear Rank Functions

When a variable has a linear-bound annotation, the compiler enumerates candidate linear measures and
verifies them with SMT.

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

> ⚠️ **Current status: experimental strategy; whether to include it will be determined during Phase
> 3 implementation according to practical feasibility.** This strategy works for adjacent-swap
> operations (bubble sort, insertion sort), but cannot automatically prove termination for
> non-adjacent operations (quick-sort partition, heap-sort sift-down). See the coverage boundary in
> the table below. If Phase 3 verification proves infeasible, this strategy will be removed or
> deferred to future work.

Core insight: **the specification written by the user is material for compiler inference.** The
compiler does not need a built-in definition of “what it means to be sorted”—it reads the definition
of `Sorted` and automatically extracts a measure from it.

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

| Algorithm      | Operation Pattern | Provable by Strategy 2? | Reason                                                      |
| -------------- | ----------------- | :---------------------: | ----------------------------------------------------------- |
| Bubble sort    | Adjacent swap     |           ✅            | violation_count strictly decreases on every swap            |
| Insertion sort | Adjacent movement |           ✅            | Every shift eliminates one violating pair                   |
| Selection sort | Non-adjacent swap |           ❌            | A single swap may increase violation_count                  |
| Quicksort      | partition         |           ❌            | Non-adjacent swaps do not guarantee monotonic decrease      |
| Heap sort      | sift-down         |           ❌            | Tree-shaped operations are not monotonic in violation_count |

**Complementary strategies**: for quicksort, contraction of the `low < high` interval can be covered
by Strategy 1 (linear rank function)—the outer partition recursion halves the interval on each
level. Strategies 1 and 2 provide complementary coverage, and most practical algorithms can be
proved by one or the other. Generalizing Strategy 2 to non-adjacent and tree-shaped operations
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

`v += const` (a positive constant), with a bounded type annotation on the variable → the measure
`upper_bound - v` decreases by `const` each time, with lower bound 0. This is a degenerate case of
Strategy 1, which the compiler handles first as a fast path.

#### 6.5 Strategy 4: Multiplicative Scaling Measure Template

`v *= const` (`const > 1`), with a bounded type annotation on the variable. The compiler provides
the logarithmic measure template `ceil(log_const(upper/v))`, which decreases by 1 each time the
value is multiplied by `const`.

```yaoxiang
mut i: Positive(i) = 1
while i < n {
    # The compiler automatically derives the measure ceil(log₂(n/i)), which decreases by 1 each time the value is multiplied by 2
    i *= 2
}
```

#### 6.6 Separation of Termination and Correctness

Termination proof and correctness proof are independent:

- **Termination**: the four strategies above automatically prove that the loop exits in finitely
  many steps; when exploration fails, the programmer supplies the measure in type position (§6.9)
- **Correctness**: whether the loop body progresses toward the target type is checked by the
  compile-time proof pipeline through verification conditions

If both pass, compilation succeeds. If termination is proved but correctness fails → compilation
error + counterexample. If correctness is proved but termination cannot be proved → compilation
error identifying the variable or operation that could not be analyzed. If both fail → compilation
error reporting the two failure causes separately.

#### 6.7 Termination Checking for Recursive Functions

For recursive functions with refined signatures, the compiler checks that the formal parameters
decrease at every recursive call site:

```yaoxiang
gcd: (a: Int, b: NonNegative(b)) -> Int = {
    if b == 0 { return a }
    return gcd(b, a % b)  # The compiler explores: the well-founded-order measure of (b, a % b) decreases → termination
}
```

Formal-parameter decrease is the strongest automatic path (structural recursion). When exploration
fails, the programmer explicitly supplies the measure in type position (§6.9).

#### 6.8 Hard Boundary

For `i = f(i)`, where `f` is not invertible, not closed, and does not preserve any monotonicity,
automatic termination proof is mathematically impossible. Compilation fails:

> This loop cannot be proved to terminate automatically. The loop variable depends on the
> unanalyzable function `f`. Use an iteration pattern that the compiler can analyze, or bind a name
> to the loop and provide a measure in type position (§6.9).

This is not a compiler failure. Code that cannot be proved safe statically must not compile. Even
when an explicit measure is provided, it must pass SMT verification—an incorrect measure is rejected
with a counterexample. A human may fail to prove termination, but cannot prove it incorrectly.

#### 6.9 Explicit Measures: `Terminates`

When automatic exploration cannot find a measure, the programmer writes the measure in **type
position**—through the same mechanism as `Positive(b)` and `IsMax(T, arr, result)` (predicate
application), with no new syntax:

```yaoxiang
// Measure: an ordinary function, unit-testable, reusable, and absent at runtime
gcd_measure: (a: Int, b: Int) -> Int = { b }

// The termination component is attached to the function's own type position
gcd: Terminates((a: Int, b: Int) -> Int, gcd_measure) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}

// Loop: the binding name is the anchor, and the measure obtains scoped values by name
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
tail expression, and the binding name `acc` provides the anchor. The loop can therefore be named,
eliminating the “anonymous construct cannot be named” corner case.

> **The body tail must provide a value**: the loop's value = the loop-body block's value (§2.9), and
> the block's value = the tail expression. If the body tail is an assignment statement, such as
> `{ i = i + 1 }`, the block value is `Void`, and `return acc` therefore returns `Void` from
> `-> Int`, which is ill-typed. This example therefore explicitly writes `i` as the body tail. This
> does not change the measure's semantics—the measure `n - i` is a **state expression**, independent
> of the loop's value type.

> **Implementation note (registered 2026-10-02; updated 2026-10-04)**: the loop example in this
> section is valid on the **type side** (`check` reports 0 errors; back-edge obligations are
> Proved). The **runtime value defect (#409) has been fixed**: a `while` used as a value now takes
> the value of the loop body's tail expression on its final iteration, matching the type-side
> `block_value_ty` convention— `loop(4) == 4` and `countdown(3) == 30` both hold in actual execution
> (the same fix applies to `for`).
>
> **Known residual issue (zero iterations)**: when the loop body is never evaluated, there is no
> “final-iteration body value,” and the loop value remains `Void`. Binding it to a non-`Void` type
> position and using it at runtime therefore reports
> `E6007 Runtime error: type mismatch in comparison Eq: Void vs Int(0)` (a conspicuous type mismatch
> rather than a silently incorrect value). This is a **static-approximation boundary**: the
> `block_value_ty` criterion yields the type of the tail expression, but cannot determine “whether
> the loop executes at least once” (which requires data-flow/provability analysis). The type side
> therefore does not silently degrade the result, because doing so would break the value type
> required by the `Terminates` loop binding in this section. This residual behavior is pinned by the
> corpus `tests/yaoxiang/02-type-system/termination/while_zero_iteration_void_err.yx`
> (`// expect: runtime-error E6007`). **This residual issue has been confirmed as an intentionally
> retained design boundary** (decision of 2026-10-04; no fix for now), so issue
> [#409](https://github.com/ChenXu233/YaoXiang/issues/409) was closed along with the main-form fix
> and is no longer tracked separately. See the minimum reproduction and historical impact in
> [RFC-027a](../accepted/027a-termination-explicit-measure.md) §Example, Loop Section, “Known Defect
> D6.” In addition, termination Strategy 1 (linear rank function) has been restored and is effective
> again, so the order “write an explicit measure only when automatic exploration fails” remains
> unchanged— an explicit measure remains the fallback after exploration fails.

**`Terminates` is a built-in predicate**, belonging to the same class of core primitives as `Int`
and `Never`. It is the only predicate for which **the compiler writes the function body**. Its
assertions (“the measure strictly decreases at every recursive call site/loop back edge”) are
embedded in the computational structure. User-written predicates cannot reference a function body or
loop body, so this predicate cannot be expressed using the predicate-definition syntax in the
“Syntax” section. The built-in surface is therefore limited to this one name.

**Arity**. `Terminates(FnType, m)` and `Terminates(m)` are two arities of the same predicate, not
two different constructs:

| Form                    | Anchor                         | Use Case                                                          |
| ----------------------- | ------------------------------ | ----------------------------------------------------------------- |
| `Terminates(m)`         | Name of the containing binding | Self-recursive functions, loops—the default form                  |
| `Terminates(FnType, m)` | Explicit function type         | Scenarios such as mutual recursion where the anchor is not unique |

The two are fundamentally the same: a termination obligation always applies to “the computation
annotated at the type position where the refinement resides.”

**The measure does not constrain the return type.** A measure may be an expression of any type (it
is not required to be a natural number); its “strict decrease” is determined by the well-founded
order available for that type. Whether the measure is well-founded—for example, whether it is `>= 0`
when it returns `Int`—is an **independent obligation**, handled by refinement inference or SMT just
like the decrease obligation. When neither can be inferred, the diagnostic **does not directly
reject** the program; instead, it suggests checking the direction (whether the measure's lower bound
holds and whether recursive arguments actually move in that direction).

**Relationship to automatic exploration**: an explicit measure is not a separate pipeline; it is an
input supplied after exploration fails. Once the measure is supplied, the same SMT engine verifies
decrease and well-foundedness. If they do not hold, an error with a counterexample is reported.

The obligation-generation, decision-pipeline, diagnostic-direction, and shared termination-measure
(SCC) mechanisms are described in
[RFC-027a: Explicit Measures for Termination Checking](../accepted/027a-termination-explicit-measure.md).

### 8. The SMT Solver: An Acceleration Module for the Type Checker

In traditional languages, the SMT solver is an external tool (for example, F\* invokes Z3, and Dafny
invokes Z3). In YaoXiang, it is an **acceleration module within the type checker**—called only when
the compiler kernel itself cannot decide directly. SMT helps find proofs, but the type checker
verifies them.

**Trust model**: the type checker is the sole trust root. The SMT solver is an acceleration
module—it helps find proofs, but SMT is not an independent trust boundary. The compiler trusts Z3's
`unsat` result (consistent with the F\*/Dafny approach—the probability of a Z3 error is lower than
the compiler's own bug rate, a pragmatic engineering choice). The real unreliability is controlled
at the SMT-translation layer—if translation has a bug, other tests will expose it.

**Interface**: internally, the compiler translates to the SMT-LIB 2.6 standard format rather than
binding to a particular solver API. SMT-LIB is an ISO standard, natively supported by Z3, CVC5,
MathSAT, and Yices.

**Default backend**: Z3 (MIT license, the broadest documentation and community validation). CVC5 is
an SMT-LIB-compatible alternative, selectable through a compiler flag at build time.

There is no “generic solver abstraction layer”—SMT-LIB is the abstraction layer. If CVC5 later gains
an advantage in a particular theory, switching requires only replacing the binary, not modifying
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

| Budget Dimension               | Default | Description                                                                                                                                                                     |
| ------------------------------ | ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Solver steps                   | 10,000  | Z3 usually takes fewer than one hundred steps for linear arithmetic. 10,000 steps cover 99% of practical predicates.                                                            |
| Time                           | 100ms   | A single predicate exceeding 100ms means the user is writing a compile-time program rather than a type annotation. 100ms × 50 predicates = a 5-second compilation-time ceiling. |
| Quantifier-instantiation depth | 3       | Three levels of nested quantifiers cover practical patterns. Beyond three levels, the user is probably writing logic exercises.                                                 |

Exceeding the budget returns Unproven, followed by a compilation error + predicate location +
consumption amount. There is no degradation, no runtime check, and no silent pass.

**Why this is practical in practice**: 95% of practical predicates in engineering are linear
arithmetic—`x > 0`, `arr.len > 0`, `0 <= idx < arr.len`—all within the decidable fragment, where the
SMT solver returns in milliseconds. For the rare complex predicate that exceeds the budget, the
programmer can write a proof function.

Dependent types undergo one layer of pre-reduction before the SMT call: `factorial(5)` is evaluated
directly at compile time to `120`, and `append([1,2], [3])` is evaluated directly to `[1,2,3]`.
These deterministic value computations do not consume SMT budget.

Programmers do not need to know that SMT exists. The mental model is: **the compiler passes what it
can prove and reports an error when it cannot—if the compiler cannot prove it, you can write a
function that proves it to the compiler.**

### 9. Composition of Compile-Time Predicates

A compile-time predicate is a function that returns Type, and composition is naturally implemented
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

result = divide(10, 2)   # ✅ The compiler verifies Positive(2) = { 2 > 0 } → True
# result = divide(10, 0)  # ❌ The compiler verifies Positive(0) = { 0 > 0 } → False
```

#### 9.2 Array-Access Safety

```yaoxiang
InBounds: (idx: Int, arr: Array(T)) -> Type = { 0 <= idx and idx < arr.len }

get: (arr: Array(T), idx: InBounds(idx, arr)) -> T = arr.data[idx]

arr = Array(Int)(1, 2, 3)
x = get(arr, 1)   # ✅ The compiler verifies InBounds(1, arr) = { 0 <= 1 and 1 < 3 } → True
# y = get(arr, 5)  # ❌ The compiler verifies InBounds(5, arr) = { 0 <= 5 and 5 < 3 } → False
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

### 11. The `dispatch` Dispatch Pipeline: Unified Compile-Time and Runtime Dispatch

> **Implementation note (registered 2026-10-03, #377-2)**: the **semantics** in this section are
> established and integrated into production, but the **structure** is not a standalone
> `layers/dispatch.rs` module. Production dispatch is implemented through three **call-site
> obligations** in `checker.rs`—binding-position re-verification (`revalidate_refined`), call-site
> actual arguments (`check_call_arg_refinements`), and return-position postconditions
> (`check_return_refinement`). Of these, the **direct calls** to
> `layers::predicate::check_predicate` occur in **two** places: the call-site actual argument and
> return-position postcondition share the single decision unit `judge_refined_obligation`
> (`checker/refinement.rs`), while `revalidate_refined` uses its own E2030 convention. A **fourth
> obligation surface exists in the proof executor** (`proof_execution.rs`, wired on 2026-11-14,
> issue #537): when executing a proof call, it first expands the **called proof function's own
> formal-parameter refinements** (RFC-027 §3.3 call-site precondition nesting obligations—a
> breadth-first worklist, deduplication set + §8 budget closure; closure of Bool constraints judged
> false reports E4018, while Call forms are enqueued for continued execution). The original
> `layers/dispatch.rs` had zero production call sites, and its handling of
> `Unproven { ProofFunctionRequired }` was “downgrade to a W1080 warning and inject Γ,” which
> conflicted with the production implementation (`Unproven` always reports an error; see
> `refined_unproven`), so the entire module was deleted. Also note that the “insert runtime check”
> action in the Runtime row of the table below has **no** emission point in production
> (`RuntimeOutcome::InsertCheck` has no consumer), while Γ injection is completed independently by
> the branch guards in `ownership.rs` and by `inference/expressions.rs`.

`assert` and `Assert` are two sides of the same refinement-type primitive. The `dispatch` pipeline
automatically chooses between compile-time proof and runtime checking according to **whether the
predicate's free variables are accessible at compile time**:

| Criterion                                                                                       | Mode            | Behavior                                                                                           |
| ----------------------------------------------------------------------------------------------- | --------------- | -------------------------------------------------------------------------------------------------- |
| All free variables are known at compile time (generic parameters, compile-time constants)       | **CompileTime** | Enter the proof pipeline: Proved → erase; Disproved → compilation error; Unknown → require a proof |
| Some free variables come from runtime (function parameters, external inputs, mutable variables) | **Runtime**     | Insert a runtime check and inject the refinement fact into the flow-sensitive assumption set Γ     |

**Key point**: “cannot decide” ≠ “disproved.” In CompileTime mode, Unknown requires a proof (no
silent degradation). In Runtime mode, the proposition has no notion of true or false at compile time
at all—no prover, however powerful, can produce a universally true proof for “the user might provide
a negative number.” A runtime check is the only sound choice. This is not a weakness of the prover;
it is a theoretical necessity.

### 12. Flow-Sensitive Assumption Set Γ: Strongest-Postcondition Propagation

The compiler maintains a flow-sensitive assumption set Γ that tracks propositions known to hold at
each control-flow point.

**Strongest-postcondition (SP) propagation**:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
y = x + 1           // Γ = {x > 0, y > 1}  ← SP propagation
```

**Kill set for mutable variables**: after a mutable variable is reassigned, every assumption
involving that variable is removed from Γ:

```yaoxiang
assert(x > 0)       // Γ = {x > 0}
mut x = x - 5       // Γ = {}  ← x > 0 is killed
```

This is a hard soundness requirement—when a variable's value changes, old assumptions are invalid.

**Branch confluence**: when IF/ELSE or match branches are merged, Γ contains the intersection of the
branches' assumptions. Only propositions that hold on every path are carried out of the branch.

**Equality bridge (automatic evidence for the q=0 slot)**: constant-equality facts in Γ are
legitimate evidence for compile-time value parameters (RFC-011 §4.1, the q=0 slot). Within the guard
branch of `if y == 42 { f(y)(y) }`, Γ records `y == 42`, so y is treated as 42 in the q=0 slot and
the call follows the existing path for `f(42)`. One side of the equality must be a compile-time
constant; a runtime value without equality evidence still produces a call-site report when placed in
the q=0 slot. This is not a new mechanism—the verification condition `{y == 42} ⇒ {y ≡ 42}` uses the
existing implication decision procedure.

#### 12.1 Definition of Definitional Equality (defeq)

This section was added by RFC-039 Decision D62: the equality bridge above provides an evidence
source for the q=0 slot, while this section completes the definition of equality itself, the role of
Unproven in type equality, and the diagnostic shape.

Definitional equality is the unique equivalence relation on types. It is defined by exactly one
rule: after normalizing both sides, they are **the same**.

- **Normalization** consists of three kinds of reduction: δ unfolds type aliases and definitions; β
  reduces type families and dependent-type applications; ι canonicalizes literal types—the identity
  of a literal type is determined only by its value, not by its source spelling (`03` and `3` are
  the same type).
- **Identity** is fully structural: type constructors have the same name, and actual arguments are
  identical one by one. Definitional equality does not inspect cache keys or rendered text, and
  performs no reasoning about refinement predicates.
- **Subtyping is an independent relation**: `A <: B` is decided by the subtyping rules. The fact
  that `A <: B` and `B <: A` both hold is not the definition of equality. Two-way subtyping would
  mix covariance and interface duck-typing rules into equivalence, contaminating the notions of
  “assignable” and “equivalent.”
- **Different `Refined` predicates ⇒ Unproven**: when the base types are the same but the predicates
  differ, the compiler returns Unproven rather than Disproved. Predicate equivalence belongs to the
  assumption-set Γ layer (the equality bridge above and the subsequent Γ equality rules). Until that
  layer is implemented, the compiler can only say “cannot decide.”

The failure surface is consistent with the three-valued algebra in §4.1: if both sides normalize
successfully and are identical ⇒ `Proved`; if both normalize successfully but differ ⇒ `Disproved`;
if either side fails to normalize ⇒ `Unproven` (budget exhausted or capability boundary).
**Reduction failure never produces `Disproved`**—inability to compute does not imply inequality.

Definitional equality is a pure decision: it **does not bind** type variables, accepts a successful
result without leaving instantiations, and is used at decision points (assignment, actual arguments,
return positions) after the unified procedure fails. Before switching to equality, however, any
partial bindings produced by the unified attempt must be rolled back. Without rollback, a failed
attempt that was judged non-equivalent would leave bindings behind and pollute subsequent inference.

#### 12.2 Unproven ≠ False: The Role of Type Equality

The three-valued algebra in §4.1 applies to every obligation point, including type equality. This
section applies “Unproven ≠ False” at that layer.

- When an obligation point returns `Unproven`, the compiler **must** report a compilation error and
  must not accept the program. The programmer's remedy is to supply evidence: write a proof function
  in the refinement domain (§4.2), or provide a normalizable form in the equality domain.
- `Unproven` and `Disproved` must be distinguishable: `Disproved` carries a counterexample (a
  concrete assignment or two unequal sides), while `Unproven` carries the unresolved proposition and
  its reason. Rendering `Unproven` as “does not hold” is an incorrect report.
- `Unproven` **must not** be downgraded to an internal compiler error (E8001). E8001 denotes only an
  internal consistency failure of the compiler; budget exhaustion and capability boundaries belong
  to user-domain diagnostics (RFC-013, “Message Quality Specification,” Section 5).
- `Unproven` **must not** be silently accepted. When a decision point ends in failure, the
  diagnostic shown to the user must come from an existing failure path; no second diagnostic path is
  introduced, and inability to decide is not grounds for acceptance.

#### 12.3 Diagnostic Shape

An `Unproven` diagnostic must carry all three of the following; if any is missing, it is incomplete:

- **Unresolved proposition**: renderable proposition text (a refinement constraint, the types on
  both sides of an equality, or a termination measure);
- **Reason**: capability boundary (outside the kernel's inference rules), budget exhaustion, or an
  unbound symbolic name—one of these;
- **Location**: source location. If a decision point truly has no location, use the default location
  and document the reason in an implementation comment.

Budget details are presented as a **one-line summary** (for example, “budget 10000 steps, 10000
steps used”). The full step count, solver-internal trace, and failure shape belong in the verbose
channel or logs, not in default output.

This section is a **formal relaxation** of “Unproven → compilation error + unresolved proposition +
budget-consumption report” in §4.1: the substance is unchanged—users must still be able to
distinguish “cannot compute it” from “does not hold.” Only the granularity of the budget-consumption
report is relaxed. The reason is that the full step count provides no actionable value to the user
(who cannot modify the program based on it) and drifts with implementation details. Limiting it to a
one-line summary improves diagnostic stability and readability, while distinguishability is
guaranteed by the “Reason” field.

The division of responsibility between general `Unproven` and refinement-specific error codes is
described in RFC-013's draft error-code table (E4023 / E2031).

### 13. Clarification of the Erasure Model: Erasing Witnesses ≠ Erasing Checks

RFC-027's assertion that refinement types are **completely erased at runtime** refers specifically
to **proof witnesses**—proof terms verified at compile time do not generate runtime code. However,
the **runtime checks** inserted by dispatch in Runtime mode are retained—they are Boolean checks
executed over values, not type-level witnesses.

In summary: witnesses are erased; checks are retained. The two are not conflicting, and RFC-027's
original assertion remains unchanged.

## Detailed Design

### Syntax Changes

| Before (RFC-022)                      | After (This RFC)                                                                                                                                                        |
| ------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `//! requires: NonEmpty(n) = n > 0`   | Compile-time predicate as a parameter type `(b: Positive(b))`                                                                                                           |
| `//! ensures: ExistsMax(result, arr)` | Return type uses a return-value formal parameter `-> (result: IsMax(T, arr, result))`                                                                                   |
| `/*! invariant: ... !*/`              | Compile-time predicate type annotations on variables—Floyd–Hoare invariants                                                                                             |
| `//! decreases: n`                    | The measure is written in refinement-type position (`Terminates`); the compiler explores automatically first and requires an explicit measure only if exploration fails |
| Specifications are comments           | Specifications are part of the type system                                                                                                                              |

### Syntax

**Compile-time predicates introduce no new keywords.** `{}` is the proof space and is exactly
consistent with the existing type-definition syntax. A compile-time predicate is simply a function
that returns Type—`name: (params) -> Type = { assertions }`. At a use site, it is an ordinary
function call—`Positive(b)`, `IsMax(T, arr, result)`.

```bnf
# Compile-time predicate = a function returning Type, with assertions inside {} verified by the compiler
# Uses the existing function/type syntax and requires no new BNF rule
predicate ::= identifier ':' params '->' 'Type' '=' '{' assertions '}'
```

**The actual arguments of a predicate application must have compile-time constant forms**—literals,
variables (bound by name), type applications (extracted recursively), or **compile-time referencable
function references** (function names). If an argument form cannot be converted into a constant
expression, report **E1092**; if the number of arguments does not match the predicate declaration's
number of formal parameters, report **E1093**. Arguments are bound positionally to the
formal-parameter list, and the predicate's arity is determined by its declaration—`Positive(x)` is
unary, `IsMax(T, arr, result)` is ternary, and `Terminates(m)` and `Terminates(FnType, m)` are unary
and binary. Refinement constraints **must never be silently discarded** (previously, an argument
that could not be converted caused the constraint to disappear silently, allowing an unsatisfied
binding to pass).

**New syntactic concept: the return-value formal parameter**—in `-> (name: Type)`, `name` is the
return-value formal parameter.

The return-value formal parameter is the **only syntactic concept** that YaoXiang introduces on top
of its existing function syntax. Its semantics are:

- The value of `name` is supplied by a `return` statement
- `name` exists only in the type signature and is referenced by postcondition predicates (such as
  `-> (result: IsMax(T, arr, result))`)
- `name` does not enter the function-body scope and does not appear at the caller
- The return-value formal parameter is **optional**—without a postcondition, the signature is
  identical to that of an ordinary function (`-> Int`), with no additional burden

The reason for introducing it is that a postcondition needs to refer to “the value the function is
about to return.” Without a return-value formal parameter, the compiler could only make the
predicate refer to the return value through special rules, such as an implicit variable `$result` or
`__retval__`. The return-value formal parameter makes this reference explicit—it is merely a formal
parameter whose value is supplied by `return` rather than by the caller.

A **proof function** is not a new concept—it is simply a YaoXiang function whose return type is the
proposition being asserted. When the compiler returns Unproven, the programmer supplies a proof
function, and the type checker verifies it in exactly the same way it verifies the return type of
any function. No new syntax, keywords, or rules are required.

> **The boundary between the two.** In the correctness domain, a predicate's Unproven result is
> addressed by supplying a proof in the **body**: write a function whose return type is the
> proposition to be proved. In the termination domain (§6.9), a measure is supplied in **type
> position**: write `Terminates(measure)` as the type of a binding or function signature. The former
> means “write a proof for a proposition that cannot be proved,” while the latter means “explicitly
> declare a measure that cannot be discovered by exploration.” The mechanism is the same in both
> cases (a refinement-type application), but the location differs. The termination domain **does not
> require** a `_proof` function.

### Type-System Impact

- **Type universe**: compile-time predicates reside at the Type₂ level—functions that accept values
  and return Type, at the same level as type constructors
- **Generic interaction**: compile-time predicates may carry generic parameters, such as
  `NonEmpty: (T: Type) -> (arr: Array(T)) -> Type`
- **Ownership interaction**: expressions in compile-time predicates obey ownership rules and may
  only be read, not written
- **Type inference**: compile-time predicate parameters participate in HM type inference

### Runtime Representation

Compile-time predicates are handled at runtime **according to the dispatch result**:

- **CompileTime mode** (all free variables are known at compile time): once the proof succeeds, the
  witness token is completely erased. `Positive: (x: Int) -> Type = { x > 0 }`—a parameter
  `b: Positive(5)` is represented as `Int` at runtime. The refinement condition `{ 5 > 0 }` has
  passed and is erased.
- **Runtime mode** (some free variables come from runtime): retain the runtime check—a Boolean check
  executed over values, injecting the result into the flow-sensitive assumption set Γ. See §11 for
  the `dispatch` pipeline and §13 for clarification of the erasure model.

Placing a compile-time predicate in a type position, such as `f(x: Positive(x))`, creates no wrapper
type and allocates no additional memory. However, if `x` comes from runtime input, a runtime Boolean
check **will** be inserted.

**Constraint on interaction with `ref`**: a compile-time predicate may refer only to an immutable
borrow or a value whose ownership has been transferred. A compile-time predicate that refers to a
mutable borrow cannot guarantee at compile time that the verification result will still hold at
runtime—such usage is a direct compilation error.

### Compiler Changes

1. **Parser**: compile-time predicates use standard function syntax and require no additional
   parsing rules
2. **Compile-time proof pipeline**: a unified Proved/Disproved/Unproven return interface with
   automatic strategy selection
3. **SMT acceleration module**: an SMT-LIB 2.6 translation layer, with Z3 as the default backend and
   CVC5 as an alternative
4. **Type-checker kernel**: implementation of inference rules—structural equivalence, βδι reduction,
   and universal-quantifier introduction/elimination. This is the sole trust root, and both SMT
   results and programmer proofs are verified through it
5. **Verification-condition generation**: WP/SP calculi + loop-invariant proof obligations
6. **Error reporting**: counterexample formatting + unresolved-proposition reports + source-location
   correlation

### Backward Compatibility

- ✅ Code that does not use compile-time predicates remains completely unchanged
- ✅ Compile-time predicates have zero runtime overhead in CompileTime mode; Runtime mode retains
  only the necessary Boolean checks
- ⚠️ RFC-022's `//!` syntax is no longer supported—but 022 was never implemented, so there is no
  migration burden

## Trade-offs

### Advantages

- **Complete realization of the Curry–Howard correspondence**: types are propositions, programs are
  proofs, `name: Proposition = Proof`
- **Uniformity**: compile-time predicates use exactly the same syntax as ordinary functions, with no
  conceptual split
- **SMT transparency**: programmers do not need to know that SMT exists, and the mental model
  remains consistent with type checking
- **Incremental adoption**: adoption can begin with one compile-time predicate and expand gradually
- **Minimal runtime overhead**: zero overhead in CompileTime mode; Runtime mode retains only the
  necessary Boolean checks

### Disadvantages

- **Compilation time**: SMT solving increases compilation time, but hard budget limits ensure that
  the upper bound is controllable
- **Automatic-proof boundary**: complex predicates beyond first-order linear arithmetic may require
  the programmer to write a proof function. This is not a language defect—it is an inevitable
  consequence of the halting problem. The compiler honestly reports Unproven rather than falsely
  reporting True/False
- **Learning curve**: writing effective compile-time predicates and proof functions requires
  understanding the basic intuition behind the Curry–Howard correspondence
- **Implementation complexity**: unifying the compile-time proof pipeline requires careful design

### Risk Mitigation

- Hard limits on the SMT-solving budget (10,000 steps / 100ms / instantiation depth 3), returning
  Unproven when exceeded
- Dependent-type pre-reduction: deterministic value computations are handled first, and SMT handles
  only the nondeterministic portion
- Unproven is not a dead end: in the correctness domain, write a proof function (the return type is
  the proposition); in the termination domain, supply a measure in type position (§6.9)—both are
  verified by the type checker
- Incremental verification: verify only changed modules
- Clear error messages + counterexample display + budget-consumption reports + unresolved
  propositions + suggestions (when the compiler can provide them)

## Alternatives

| Option                                                             | Why It Was Not Chosen                                                                                                                                                                            |
| ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| RFC-022: `//!` comment-style specifications                        | Splits specifications from types, violating the Curry–Howard correspondence                                                                                                                      |
| Separate specification files, such as CVL                          | Separates specifications from code and increases maintenance costs                                                                                                                               |
| Runtime assertions only                                            | Cannot provide static correctness guarantees                                                                                                                                                     |
| External proof assistants, such as Coq                             | Disconnects proofs from the compiler and requires a separate proof language and trust boundary. YaoXiang's choice is that the proof is YaoXiang code and the type checker is the sole trust root |
| **This proposal: compile-time predicates as first-class citizens** | ✅                                                                                                                                                                                               |

## Implementation Strategy

### Phase Division

| Phase       | Content                                                                                                                                                                  |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Phase 1** | Compiler kernel: structural equivalence + βδι reduction + universal-quantifier introduction/elimination. Supports simple arithmetic predicates (`x > 0`, `arr.len > 0`)  |
| **Phase 2** | SMT-LIB translation layer + Z3/CVC5 integration. The pipeline returns Proved/Disproved/Unproven. Supports programmer-written proof functions when the result is Unproven |
| **Phase 3** | Loop-invariant VC generation + termination checking (four measure-exploration strategies + explicit `Terminates` measures, §6 and §6.9)                                  |
| **Phase 4** | Incremental verification + caching + IDE support                                                                                                                         |

### Dependencies

- RFC-010: Unified Type Syntax — compile-time predicates are based on `name: type = value`
- RFC-011: Generic System — compile-time predicates may carry generic parameters
- RFC-009: Ownership Model — expressions in compile-time predicates obey ownership rules

## Open Questions

- [x] **Z3 availability for the wasm32 target**: Z3 is unavailable under wasm (all SMT code is
      excluded by `cfg(not(target_arch = "wasm32"))`). Scope and degradation direction: -
      `ownership.rs` `smt_cut` always returns `false` → back-edge traversal → **conservative
      rejection** (the sound direction, though it narrows less) - Termination-checking Strategy 1
      and predicate levels 2b/3 are skipped entirely

  **Correction (2026-10-01)**: the original text claimed that these paths were “currently
  **unreachable**” (on the grounds that `predicate_defs` was never populated in production, `parser`
  did not produce `MonoType::Refined`, and production `with_solver` was never called). That is no
  longer true—all three have been implemented (predicate-definition registration #377-3;
  refinement-type normalization; solver injection into the production pipeline). The difference
  between wasm and native is therefore no longer limited to the precision of `smt_cut`:

  | Path                                            | native     | wasm       | Degradation Direction                |
  | ----------------------------------------------- | ---------- | ---------- | ------------------------------------ |
  | Explicit termination-measure determination      | Determined | `Unjudged` | No E4022 is reported (fewer reports) |
  | Well-foundedness determination                  | Determined | `Unjudged` | No E4022 is reported (fewer reports) |
  | SMT implication level for refinement predicates | Executed   | Skipped    | Falls through to `Unproven`          |

  All of these are **conservative** directions (less narrowing and fewer reports), with no impact on
  soundness. To achieve identical precision, the options and costs are described in issue #376
  (conclusion: prefer a JS-side Z3 instance rather than linking Z3 into the main wasm binary— the
  latter requires replacing the emcc build system and grows the artifact from 3 MB to ~20 MB).

- [x] **SMT-solver choice**: Z3 by default (MIT license, the most broadly validated). CVC5 is an
      SMT-LIB-compatible alternative, selectable through a compiler flag. The compiler's internal
      translation target is the SMT-LIB 2.6 standard format—SMT-LIB is the abstraction layer, and
      there is no custom generic solver interface.
- [x] **Specific solver-budget values**: 10,000 steps / 100ms / quantifier-instantiation depth 3.
      These are fixed inside the compiler, with no knob exposed. If practical use cases demonstrate
      that they are insufficient (rather than “the user made a mistake”), they may be adjusted in
      the future.
- [x] **Quantifier-support scope**: the language does not restrict quantifier order. Compile-time
      predicates accept Type parameters—Type includes function types—so higher-order quantifiers are
      a natural consequence of the type system and require no special syntax. The SMT solver can
      automatically decide first-order quantifiers (forall/exists, including alternating nesting,
      limited by the budget depth of 3). For higher-order quantifiers, SMT returns Unproven and the
      compiler prompts, “This predicate is outside the range of automatic proof; please provide a
      proof function.” The programmer writes a YaoXiang function whose return type is the
      proposition, and the type checker verifies that function. No external export, AI, or
      interactive proof mode is required. Everything is YaoXiang code, and everything is verified by
      the type checker.
- [x] **Counterexample formatting**: source variable names are used directly as SMT variable names,
      with module prefixes added to avoid collisions. When the Z3 model is returned, variables are
      looked up by name. Output format: variable name = concrete value + source location +
      predicate-definition location. No complex mapping layer is implemented.
- [x] ~~**Interaction between compile-time predicates and the `ref` smart pointer?**~~ → Decision:
      compile-time predicates may refer only to immutable borrows or values whose ownership has been
      transferred. Values held through mutable borrows may not appear in compile-time predicates.
- [x] **Extending the `forall` predicate violation-count measure to non-adjacent operations?** → Do
      not extend it. The current coverage (adjacent swaps, adjacent movement) is complemented by
      Strategy 1 (linear rank function)—the outer interval contraction of quicksort is covered by
      Strategy 1, while heap sort is covered by Strategy 1 (array-index pattern). A loop that no
      strategy can prove terminates is rejected directly by the compiler—this is the hard-safety
      philosophy, not a defect. If a practical, non-academic use case arises in which all four
      strategies fail, the matter will be reconsidered.→ **Reconsideration was triggered
      (2026-09-14, #318)**: non-structural recursion (gcd-like non-direct decreases, mutual
      recursion, merge partitioning) is precisely the kind of practical scenario anticipated by this
      clause—the template sequence for measure exploration cannot handle it automatically, and it
      cannot be rewritten as an analyzable iterative pattern without sacrificing readability.
      Conclusion: the hard-safety philosophy remains unchanged (an incorrect measure is still
      rejected by an SMT counterexample; a human may fail to prove termination but cannot prove it
      incorrectly), but “failure to explore ⇒ rejection” is relaxed to “failure to explore ⇒ the
      programmer may explicitly supply a measure in type position”—see §6.9.
- [x] **Combinatorial explosion in linear-rank-function enumeration**: the candidate-enumeration
      limit is three bounded variables. With ≤3, all linear combinations are enumerated and verified
      individually by SMT. With >3, only single-variable measures are attempted (`v_i`,
      `u_i - v_i`), and failure produces a compilation error prompting the programmer, “This loop
      has more than three bounded variables, and the compiler cannot automatically synthesize a
      multivariable measure.” This is not an engineering compromise—it forces programmers to write
      simpler loops.

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

## Lifecycle and Destination

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
