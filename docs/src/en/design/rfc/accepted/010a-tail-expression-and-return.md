---
title: 'RFC-010a: Tail Expression Evaluation and `return` Semantics'
status: 'Accepted'
author: 'Chenxu'
created: '2026-09-15'
updated: '2026-09-15 (Accepted)'
group: 'rfc-010'
issue: '#342'
---

# RFC-010a: Tail Expression Evaluation and `return` Semantics

> **References**:
>
> - [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md) — `name: type = value` model, `{}`
>   dependency-driven computation unit
> - [RFC-007: Function Definition Syntax Unification](./007-function-syntax-unification.md) — code
>   block return rules, early return
> - [RFC-038: Statement Termination and Line Break Rules](038-statement-termination.md) — line break
>   behavior of statements and expressions
> - [Language Spec §Type System](/reference/language-spec/type-system.md) — `Never` explosion
>   principle

## Summary

Unify the semantics of `return` and block evaluation, and eliminate the conflicting statements
between RFC-007 and RFC-010.

This RFC proposes **three self-consistent rules**: a block's value equals its tail expression (the
sole exit); `return` is a **non-local exit** of type `Never` (exits the function, not "returning to
the block"); and `if` without `else` evaluates to `Void`.

`return` and block evaluation **are not bifurcated at the language level** — `{ return n }` as a
block has value `n` (type `Never`), while `return` acts to exit the function. These two facts
coexist via the **explosion principle** (`Never <: T`). The "early return" of RFC-007 and the "block
has a value" of RFC-010 are consequences of these three rules, not contradictory special cases.

No new syntax, no new keywords.

## Implementation Status

This RFC has been accepted. Implementation status of the three rules:

| Rule                           | Sub-item                                              | Status                                            |
| ------------------------------ | ----------------------------------------------------- | ------------------------------------------------- |
| ① Block's value = tail expr    | Function body tail expression                         | ✅ Implemented                                    |
| ① Block's value = tail expr    | Tail-position `if` / `match`                          | ✅ Implemented (#344)                             |
| ① Block's value = tail expr    | Trailing assignment statement → `Void`                | ✅ Implemented                                    |
| ① Block's value = tail expr    | Empty block `{}` → `Void`                             | ✅ Implemented                                    |
| ① Safeguard by type checking   | Tail expression matches declared return type          | ✅ Implemented (#345)                             |
| ② `return` is non-local exit   | Pierces through `if`/`while`/`for`/bare/nested blocks | ✅ Implemented                                    |
| ② `Never <: T` explosion       | `unify` and `is_subtype` consistent                   | ✅ Implemented (this RFC fixes internal conflict) |
| ③ `if` without `else` → `Void` | Branch value not leaked in expression position        | ✅ Implemented (#346)                             |
| ① Block's value = tail expr    | Bare block value binding `x = { ... }`                | ❌ Not implemented (#343, pending design ruling)  |
| ① Block's value = tail expr    | `unsafe {}` value exit                                | ❌ Not implemented (#347)                         |

Test coverage: `tests/yaoxiang/03-semantics/rfc010a_block_value.yx` (positive) +
`tests/yaoxiang/06-compile-errors/tail_expr_type_mismatch{,_with_stmts}_err.yx` (negative).

## Motivation

### Conflicting Statements

The Fibonacci example in the playground exposed a semantic divergence:

```yaoxiang
fib: (n: Int) -> Int = {
  if n <= 1 {
    return n          // Does this return belong to "if" or to "the function"?
  }
  return fib(n - 1) + fib(n - 2)
}
```

The two already-accepted RFCs give **opposite** derivations:

| RFC                    | Statement                                                                                                                              | Derived Semantics                                                             |
| ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| **RFC-007** (accepted) | Uses `factorial` as example, titled **"Early Return: Using `return`"**: `if n <= 1 { return 1 }`                                       | `return` pierces through `if` and the function body, **back to the function** |
| **RFC-010** (accepted) | "`{}` is a dependency-driven computation unit…use `return` to explicitly return a value"; `spawn { return c }` returns the task result | `return` gives its value to **this `{}`**                                     |

Per RFC-010, the braces in `if n <= 1 { return 1 }` are a computation unit, `return 1` gives its
value to 1, so the value of the `if` statement is discarded and the next line inevitably executes —
**`fib` recurses infinitely**. Per RFC-007, it is correct.

### Root Cause: The Word `return` Has Two Jobs

- **RFC-007 uses it to express "exit the function"** — a control flow concept
- **RFC-010 uses it to express "this block's value is it"** — an evaluation concept

Using a control flow keyword to express evaluation is a category error. One word carrying two jobs —
no matter how you tune it, one side is sacrificed.

### Downstream Documentation's Erroneous Over-Extension

`docs/src/reference/language-spec/syntax.md` extends RFC-010's "`{}` block" to **all curly braces**:

- §2.9: "`return` in `{}` **always returns the contents to the enclosing scope**" (and claims
  "unified semantics: all `{}` blocks")
- §3.3: "`return` is used to **return a value from a code block**"

Yet RFC-010 originally defined only **three valued blocks** (`= {}` / `spawn {}` / `unsafe {}`), and
**never mentioned the braces of `if` / `while` / `for` / `match`**.

### Current Implementation State

| Behavior                                        | Current State                                    |
| ----------------------------------------------- | ------------------------------------------------ |
| `if n == 0 { return 7 } … return 8`             | Function exits (`h(0) = 7`)                      |
| `f = { n + 1 }`                                 | Tail expression usable (returns `5`)             |
| `x = { y = 5; y }` (bare block tail expression) | `E3006` variable unresolved (see #343)           |
| `f: () -> Int = { if c {5} else {6} }`          | Returns `void`, tail expression discarded (#344) |
| `f: () -> Int = { "s" }`                        | Silently passes compilation (see #345)           |
| `x = if c { 19 }` (no `else`)                   | `19` (should be `Void`, see #346)                |
| `v = unsafe { 42 }`                             | `void` (see #347)                                |
| `y = if c { 111 } else { 222 }`                 | Usable (`if` as expression)                      |

`tests/yaoxiang/03-semantics/no_tail_expr_return.yx` once claimed "tail expressions are no longer
implicitly returned", but did not cover that case, so the test stayed green. That file has been
replaced by `tests/yaoxiang/03-semantics/tail_expr_and_return.yx`.

## Proposal

### Three Rules

```
① Block's value = tail expression (the sole exit)
   Assignment statements have value Void; empty block {} has value Void
② return : (T) -> Never
   Non-local exit: exits the nearest function boundary
③ if without else → Void
```

**These three rules suffice to derive "early return"; no extra rule is needed to make `return`
specific to the function.**

### Rule ①: Block's Value

The block's **last statement/expression** is the block's value (the tail expression). This is
**not** "no tail expression means `Void`" — a non-empty block always has a tail expression (the last
statement itself is one), and an empty block `{}` has value `Void`.

```yaoxiang
// Tail expression determines block's value
a = {
    x = compute()        // assignment statement → Void
    x * 2                // tail expression → block's value
}

// Assignment as tail expression → block's value is Void
b = {
    x = compute()
    log(x)               // assignment statement → Void
}

// Explicitly write Void when you want Void
c = {
    log(x)
    Void                 // explicit Void
}
```

**Design Criterion (separation of mechanism and safeguard)**:

- **Language rules provide only the mechanism**: the last statement is the block's value, with one
  unambiguous rule
- **The safeguard is provided by type checking**: function declared as `-> Int` but tail expression
  type mismatches → compile error
- **The language does not defend against "wrong intent"**: if the tail expression's type happens to
  match the return type but the semantics is unintended, that is the author's oversight; the
  language cannot judge. **Do not rely on language rules to defend against wrong intent.**
- **Write `Void` explicitly when you do not want to return**

This replaces RFC-010's "`= { ... }` must use `return`, otherwise returns `Void`", and also replaces
its design rationale of "needing explicit `return` to eliminate the ambiguity of 'whether the last
expression is the return value'".

### Rule ②: `return` is Non-Local Exit

`return` has type `Never` (zero constructors, no value can inhabit it). Its semantics:

- **Exits the nearest function boundary**, handing the value to the caller
- **Pierces through all blocks** — (any) `if` / `while` / `for` / `match` / bare blocks / `spawn` /
  `unsafe`

`return` does not "return to the block". `{ return n }` as a block has value `n` (by the tail
expression rule), **type `Never`**; at the same time, `return`'s action is to exit the function.
**Both hold simultaneously.**

### Rule ③: `if` Without `else`

```yaoxiang
x = if c { 19 }        // no else
```

When the condition is false there is no branch to evaluate, so the value is `Void`. Thus this `if`'s
value type is `Void` (and cannot be used in non-`Void` positions).

Provide both branches explicitly when a value is needed:

```yaoxiang
x = if c { 19 } else { 20 }
```

### `Never` is the Technical Foundation of Coexistence

`Never <: T` holds for any type `T` (explosion principle, see Language Spec §Type System). Hence:

```yaoxiang
fib: (n: Int) -> Int = {
  if n <= 1 {
    return n              // tail expression : Never
  }                       // → this if's value : Never
  fib(n - 1) + fib(n - 2) // → block's value : Int
}
```

- The branch body `{ return n }` has value `Never`
- This `if`'s only branch is `Never` ⇒ the `if`'s value is `Never`
- A statement of type `Never` means **the sequence terminates here** — the following statement is
  not "the next sequentially executed one"
- And `Never` can be coerced to any type (explosion principle), so the whole block satisfies
  `-> Int`

**"Early return" follows naturally**: the `Never` type terminates the sequence + the explosion
principle permits coercion.

## Detailed Design

### Formalization of Blocks and Tail Expressions

```
Block        ::= '{' Stmt* '}'                     // empty block → Void
               | '{' Stmt* Expr '}'                // value = Expr
Expr         ::= ...
               | Return                            // type Never
Stmt         ::= Assignment | ExprStmt | ...

value(Block):
  empty block               → Void
  { ...; e }                → type(e)
  { ...; s } (s is a stmt)  → Void          // assignment's value is Void
```

### Type Rule for `return`

```
return e : Never        where e : T

// Since Never <: T' for any T', return may appear in any return-type position
```

**No extra rule is needed to constrain `return`'s legality** — the explosion principle already
covers it. This makes the question "can `return` appear in a function returning `X`" simply
disappear.

### Multi-Branch Joining

```
join(A, B):
  If A : Never  → B
  If B : Never  → A
  Otherwise     → require A, B compatible (same type or reachable common upper bound)
```

Multiple branches of `if` / `match` are joined by `join`. Branches containing `return` have type
`Never` and do not participate in the join.

### Consistency with RFC-007

RFC-007's examples **fully conform to this RFC**, needing no revision:

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }          // Never branch, ignored by join
    return n * factorial(n - 1)     // function exits
}
```

"Early return" is not a special case, but a **derivation** of rules ① ② plus the explosion
principle.

### Differences from RFC-010 and Revision Requirements

RFC-010 needs three revisions (as errata, see "Revision Requirements for Downstream Documents"):

| RFC-010 Clause                                                                     | After Revision                                                                          |
| ---------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| "`= { ... }` must use `return`, otherwise returns `Void`"                          | Block's value = tail expression; empty block → `Void`                                   |
| Design rationale "explicit `return` needed to eliminate tail expression ambiguity" | Voided — tail expressions produce no ambiguity, and `return` does not interfere with it |
| Three examples `return c` / `return SqliteDb`                                      | Changed to tail expressions (unified value exit for `spawn` / `unsafe`)                 |

**RFC-010's core design (`{}` as a dependency-driven computation unit) remains unchanged**; only the
value exit is switched from `return` to a tail expression.

### Unified Perspective (Design Principle)

The braces of `if` / `while` are **both an imperative control body and a declarative evaluation
unit** — this is **a difference of perspective, not two language constructs**. Therefore:

- **Do not bifurcate at the language level** between "control body" and "evaluation unit"
- All blocks share one set of evaluation rules (rule ①)
- `return` is the sole exception mechanism, and its exceptionality comes from the type-theoretic
  property of `Never`, not a syntactic special case

This allows declarative and Pythonic styles to coexist naturally:

```yaoxiang
a = if input > 10 { 19 } else { 20 }
b = { x = f(); x * 2 }
c = spawn { fetch("a") }
```

## Trade-offs

### Advantages

- **Eliminates RFC conflict**: RFC-007 and RFC-010 go from contradiction to derivation, no need to
  sacrifice either side
- **Minimal rule count**: three rules + one type-theoretic property (explosion principle), no
  special cases
- **`return` has a single meaning**: exits the function. No more need to judge whether it belongs to
  "block value / function value"
- **Unified perspective**: imperative and declarative coexist, no language-level bifurcation
- **Consistent with mature languages**: Rust similarly uses "tail expression + `return : !`" to let
  both mechanisms coexist
- **Zero new syntax**: no new keywords, no parser grammar changes

### Disadvantages

- **"Accidental return" risk from last-expression-is-value**: forgetting to remove the last line
  silently changes the return value
  - Mitigation: type checking intercepts type mismatches; the language does not promise to defend
    against wrong intent (see design criterion)
- **RFC-010 needs errata**: examples in the already-accepted document must be rewritten
- **Bare block tail expressions pending implementation**: currently a compiler internal error
  (independent of this design)
- **Interaction with statement termination needs clarification**: whether a line-break-terminated
  final expression still counts as the block's value (see Open Questions)

## Alternatives

### Option A: Keep `return`'s dual roles, dispatch by containing block type

In function body `= {}`, `return` belongs to the function; in `spawn {}` / `unsafe {}`, it belongs
to the block; in `if {}` / bare blocks, it belongs to…?

**Reason for rejection**: `if`'s ownership cannot be decided — this is the original conflict. Also
requires the user to remember "which blocks go where", with no principled basis (why would `if` go
to the function but a bare block to itself?).

### Option B: Strict block return (`return` always belongs to the current block)

**Reason for rejection**: **Functionality loss**. `return` can never early-exit the function from a
nested block; guard clauses (`if err { return }`) become completely unwriteable, and one can only
write functions as deeply nested expressions.

### Option C: New keyword for block value (`give x` / `yield x`)

**Reason for rejection**: Violates RFC-036's **zero syntax change** principle (requires new
keywords), and the user must learn two concepts (`return` for exit + `give` for evaluation). The
tail expression approach has zero new concepts.

### Option D: Keep RFC-010 as-is (must use explicit `return`)

**Reason for rejection**: Irreconcilably conflicts with RFC-007 (see Motivation). And the actual
implementation has already taken the tail expression path.

## Revision Requirements for Downstream Documents

The following documents must be revised after this RFC is accepted. **Erroneous examples are to be
deleted outright, not retained as seemingly-usable incorrect code in an errata block.**

| Document                                                    | Revision Content                                                                                              | Status |
| ----------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- | ------ |
| RFC-010 §Return Rules                                       | Switch to tail expression rules; **delete** the original "must use `return`, otherwise return `Void`" example | ✅     |
| RFC-010 Design Rationale Section                            | **Delete** the argument "explicit `return` needed to eliminate ambiguity"                                     | ✅     |
| RFC-010 §`spawn` block / §`unsafe` block Examples           | `return c` / `return SqliteDb` → tail expression form                                                         | ✅     |
| RFC-010 §Dependency-Driven Computation Unit Example         | `return b` → tail expression `b`                                                                              | ✅     |
| `language-spec/syntax.md` §2.9                              | "`return` returns to enclosing scope" → tail expression rule + `return` exits function                        | ✅     |
| `language-spec/syntax.md` §3.3                              | "Return value from a code block" → "Exits the nearest function boundary, type `Never`"                        | ✅     |
| `language-spec/type-system.md`                              | "Function with no `return` returns `Void`" → tail expression decides                                          | ✅     |
| `tests/yaoxiang/03-semantics/no_tail_expr_return.yx`        | Assertions and comments contradict the new rules, must be fixed                                               | ✅     |
| RFC-007 §Summary / §Lambda Syntax Rules / Syntax Rule Table | "Must use `return`, otherwise return `Void`" → tail expression + `Never` errata block                         | ✅     |

RFC-007's statement "must use `return` inside a code block, otherwise return `Void`" conflicts with
this RFC and has been revised via errata (RFC-007's formal function definition is unchanged) — see
the last row of the table above.

## Open Questions

- [ ] Interaction between tail expressions and RFC-038's statement termination rules: does a
      line-break-terminated final expression still count as the block's value? (@Chenxu: needs
      confirmation together with RFC-038's "line-leading `(`/`[` never merges" and other rules)
- [x] When is `name = { ... }` a function vs. a block value binding — see Appendix D (Ruling C:
      annotation takes priority, function by default)
- [ ] Specific form of `unsafe {}` after revision — currently entirely unusable (see #347)
- [ ] Interaction between `match` branch `join` and exhaustiveness checking (depends on RFC-039)
- [ ] Diagnostic message when empty block `{}` is used as a function body with non-`Void` return
      type

---

## Appendix A: Empirical Evidence

All reproduced on 0.8.0.

| Code                                              | Measured                          |
| ------------------------------------------------- | --------------------------------- |
| `h: (n)->Int = { if n==0 { return 7 } return 8 }` | `h(0)=7`, `h(1)=8`                |
| `f: (n)->Int = { n + 1 }`                         | `5` (tail expression usable)      |
| `f: ()->Int = { if c {5} else {6} }`              | `void` (should be `5`, see #344)  |
| `f: ()->Int = if c {5} else {6}`                  | `5`                               |
| `f: ()->Int = { match ... }`                      | Correct                           |
| `f: ()->Int = { while ...; i }`                   | Correct                           |
| `{ y = 5; y }` as binding expression              | `E3006` (see #343)                |
| `f: ()->Int = { "s" }`                            | Silently passes (see #345)        |
| `x = if c { 19 }` (no `else`)                     | `19` (should be `Void`, see #346) |
| `v = unsafe { 42 }`                               | `void` (see #347)                 |
| `y = if c { 111 } else { 222 }`                   | `111`                             |
| `while { if i==2 { return 42 } }`                 | `42` (pierces out of loop)        |
| Nested `{ { return 5 } return 1 }`                | `5` (pierces out of bare block)   |

## Appendix B: Design Decision Record

| Decision                          | Determination                                                                | Reason                                                               | Date       |
| --------------------------------- | ---------------------------------------------------------------------------- | -------------------------------------------------------------------- | ---------- |
| `return` semantics                | Function exit, type `Never`; does not return to the block                    | Eliminate the category error of one word doing two jobs              | 2026-09-15 |
| Block's value exit                | Tail expression (the sole exit)                                              | Minimum rule count; consistent with Rust                             | 2026-09-15 |
| No tail expression                | Does not exist (non-empty block always has a tail expr; empty `{}` → `Void`) | Write `Void` explicitly when you want `Void`                         | 2026-09-15 |
| Assignment's value                | `Void`                                                                       | Assignment is a statement, not a value producer                      | 2026-09-15 |
| `if` without `else`               | `Void`                                                                       | No branch to evaluate when condition is false                        | 2026-09-15 |
| Division of mechanism & safeguard | Language provides mechanism, type checking provides safeguard                | The language does not promise to defend "wrong intent"               | 2026-09-15 |
| Control body / evaluation unit    | Not bifurcated at the language level; treat as difference of perspective     | Imperative and declarative coexist; avoid unprincipled special cases | 2026-09-15 |
| Handling of erroneous examples    | Delete; do not retain in the errata block                                    | Retaining seemingly-usable wrong code would mislead                  | 2026-09-15 |

## Appendix C: Glossary

| Term                | Definition                                                                                                        |
| ------------------- | ----------------------------------------------------------------------------------------------------------------- |
| Tail expression     | The last value-producing expression in a block; the block's value is it                                           |
| Non-local exit      | A control-flow transfer that crosses outer evaluation units and acts directly on the function boundary (`return`) |
| Explosion principle | `Never <: T` holds for any `T`, allowing `Never` to be coerced to any type                                        |
| Valued block        | `= {}` / `spawn {}` / `unsafe {}` — value exit is the tail expression                                             |
| join                | Multi-branch merging rule; `Never` branches do not participate in the join                                        |

## Appendix D: Function / Block Value Ambiguity Ruling (Ruling C)

### Problem

`name = { ... }` is defined differently by two already-accepted RFCs:

| RFC                       | Definition                                                        |
| ------------------------- | ----------------------------------------------------------------- |
| RFC-007 (function syntax) | Function. "Zero-arg minimal" `name = { return ... }`              |
| RFC-010 / 010a            | Block value. "`= {}` is a valued block, value is tail expression" |

The same syntactic position with two semantics caused `callable_parts()` to register the block as a
0-arg function while `generate_block_ir` took the block value — the two layers' understandings were
inconsistent.

### Ruling: Annotation Takes Priority, Function by Default

| Case                          | Result          | Basis                                    |
| ----------------------------- | --------------- | ---------------------------------------- |
| `value` is a `Lambda` (`=>`)  | Function        | `=>` is an explicit function constructor |
| Annotation is `Fn`            | Function        | Declared a function type                 |
| Annotation is a non-`Fn` type | **Block value** | Annotation is the type (`x: Int = {..}`) |
| No annotation                 | **Function**    | Default; RFC-007 "zero-arg minimal"      |

**Key: block-value semantics does not require new syntax.** To evaluate `{ ... }` on the spot, just
write the target type:

```yaoxiang
x: Int = { y = 5; y }      # block value: x = 5 (annotation is not Fn)
f: () -> Int = { 5 }       # function: f() = 5 (annotation is Fn)
f = { 5 }                  # function: f() = 5 (no annotation, default function)
b = () => 5                # function: expression-body lambda
```

### Reasoning

The annotation **already declares the type**: `x: Int = ...` says `x` is `Int`, so `{...}` is an
`Int` value; `f: () -> Int = ...` says `f` is a function, so `{...}` is a function body.
**Type-directed, not an invented new rule.**

Default to function when no annotation, because:

1. Zero changes to existing code (there are 203 unannotated `name = { ` bindings in the repo, all
   `main`)
2. Function definitions are high-frequency; block values are low-frequency
3. RFC-007 is already accepted; changing it is more expensive than changing this RFC

### Cost (Known and Accepted)

In `f = { 5 }`, `f` is a **function**, not `5`. This contradicts the intuition that "curly braces
are values" — but that is exactly RFC-007's already-accepted definition, and **the annotation is the
escape hatch**: write `f: Int = { 5 }` to get a value.

### Implementation Defects Simultaneously Fixed by This Ruling

1. **Annotation must not decide value-taking** (root cause one): three places, including
   `generate_function_ir`, used `return_type != Void` as the threshold for taking a tail
   expression's value, causing the tail expression of unannotated `f = { 5 }` to be silently
   discarded and return `Void`. The annotation decides whether to _check_, not whether to
   _evaluate_.
2. **`callable_parts()` no longer unconditionally swallows the block**: dispatch is unified by
   `Expr::block_binding_is_function(annotation, value)`.

## References

- [RFC-007: Function Definition Syntax Unification](./007-function-syntax-unification.md)
- [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)
- [RFC-038: Statement Termination and Line Break Rules](038-statement-termination.md)
- [RFC-030: assert Mechanism](./030-assert-mechanism.md) — Refined type application of `Never`
- [Language Spec §Type System](/reference/language-spec/type-system.md) — `Never` / `Void` ⊥ / ⊤
  positioning
- [Rust Reference: `!` never type](https://doc.rust-lang.org/reference/types/never.html) —
  Isomorphic application of the explosion principle
