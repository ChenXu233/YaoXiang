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
>   as dependency-driven computation units
> - [RFC-007: Function Definition Syntax Unification](./007-function-syntax-unification.md) — Code
>   block return rules, early return
> - [RFC-038: Statement Termination and Line Break Rules](038-statement-termination.md) — Line break
>   behavior of statements and expressions
> - [Language Specification §Type System](/reference/language-spec/type-system.md) — `Never`
>   bottoms-out principle

## Summary

Unify the semantics of `return` and block evaluation, eliminating the conflicting statements between
RFC-007 and RFC-010.

This RFC proposes **three self-consistent rules**: a block's value equals its tail expression (the
sole exit point); `return` is a **non-local exit** of type `Never` (it exits the function, not
"returning to the block"); and an `if` without `else` takes `Void`.

`return` and block evaluation **are not split into two by the language**—`{ return n }` as a block
has value `n` (type `Never`), while at the same time `return`'s effect is to exit the function.
These two things coexist via the **bottoms-out principle** (`Never <: T`). RFC-007's "early return"
and RFC-010's "blocks have values" are corollaries of these three rules, not contradictory special
cases.

No new syntax, no new keywords.

## Implementation Status

This RFC is accepted. Implementation status of the three rules:

| Rule                                   | Sub-item                                                   | Status                                           |
| -------------------------------------- | ---------------------------------------------------------- | ------------------------------------------------ |
| ① Block value = tail expr              | Function body tail expression                              | ✅ Implemented                                   |
| ① Block value = tail expr              | Tail-position `if` / `match`                               | ✅ Implemented (#344)                            |
| ① Block value = tail expr              | Trailing assignment statement → `Void`                     | ✅ Implemented                                   |
| ① Block value = tail expr              | Empty block `{}` → `Void`                                  | ✅ Implemented                                   |
| ① Protection provided by type checking | Tail expression unified with declared return type          | ✅ Implemented (#345)                            |
| ② `return` non-local exit              | Pierces through `if`/`while`/`for`/bare block/nested block | ✅ Implemented                                   |
| ② `Never <: T` bottoms-out             | `unify` consistent with `is_subtype`                       | ✅ Implemented (resolves internal contradiction) |
| ③ `if` without `else` → `Void`         | Branch value does not leak at expression position          | ✅ Implemented (#346)                            |
| ① Block value = tail expr              | Bare block value binding `x = { ... }`                     | ✅ Implemented (#343)                            |
| ① Block value = tail expr              | `unsafe {}` value exit                                     | ✅ Implemented (#347)                            |
| ① Protection provided by type checking | Empty block / trailing statement escape check              | ✅ Implemented (#342 open issue 5)               |
| ① Block value = tail expr              | `spawn {}` value exit (tail expression)                    | ✅ Implemented (#365)                            |

Test coverage: `tests/yaoxiang/03-semantics/rfc010a_block_value.yx` (positive) +
`tests/yaoxiang/06-compile-errors/tail_expr_type_mismatch{,_with_stmts}_err.yx` (negative).

## Motivation

### Conflicting Statements

A Fibonacci example in the playground exposed a semantic divergence:

```yaoxiang
fib: (n: Int) -> Int = {
  if n <= 1 {
    return n          // Is this return "the if's" or "the function's"?
  }
  return fib(n - 1) + fib(n - 2)
}
```

Two already-accepted RFCs give **opposite** derivations:

| RFC                    | Statement                                                                                                                               | Derived semantics                                                             |
| ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| **RFC-007** (Accepted) | Using `factorial` as example, the title reads **"Early return: using return"**: `if n <= 1 { return 1 }`                                | `return` pierces through `if` and the function body, **back to the function** |
| **RFC-010** (Accepted) | "`{}` is a dependency-driven computation unit…uses `return` to explicitly return a value"; `spawn { return c }` returns the task result | `return` gives **that `{}`** a value                                          |

According to RFC-010, in `if n <= 1 { return 1 }` the braces form a computation unit, `return 1`
gives it value 1, so the `if` statement's value is discarded and the next line will inevitably
execute—**fib recurses infinitely**. According to RFC-007, it is correct.

### Root Cause: `return` Carries Two Jobs

- **RFC-007 uses it to express "exiting the function"**—a control-flow concept
- **RFC-010 uses it to express "this block's value is it"**—an evaluation concept

Using a control-flow keyword to express evaluation is a **category error**. When one word bears two
jobs, no matter how you adjust it, one side gets sacrificed.

### Downstream Documentation's Mis-Expanded Interpretation

`docs/src/reference/language-spec/syntax.md` expanded RFC-010's "`{}` blocks" to **all braces**:

- §2.9: "`return` inside `{}` **always returns the content to the enclosing scope**" (and calls it
  "unified semantics: all `{}` blocks")
- §3.3: "`return` is used to **return a value from a code block**"

But RFC-010 originally defined only **three value-bearing blocks** (`= {}` / `spawn {}` /
`unsafe {}`), and **never mentioned the braces of `if` / `while` / `for` / `match`**.

### Current Implementation State

| Behavior                                        | Current state                                    |
| ----------------------------------------------- | ------------------------------------------------ |
| `if n == 0 { return 7 } … return 8`             | Function exits (`h(0) = 7`)                      |
| `f = { n + 1 }`                                 | Tail expression usable (returns `5`)             |
| `x = { y = 5; y }` (bare block tail expression) | `E3006` variable unresolved (see #343)           |
| `f: () -> Int = { if c {5} else {6} }`          | Returns `void`, tail expression discarded (#344) |
| `f: () -> Int = { "s" }`                        | Silently passes compilation (see #345)           |
| `x = if c { 19 }` (no `else`)                   | `19` (should be `Void`, see #346)                |
| `v = unsafe { 42 }`                             | `void` (see #347)                                |
| `y = if c { 111 } else { 222 }`                 | Usable (`if` as expression)                      |

`tests/yaoxiang/03-semantics/no_tail_expr_return.yx` once claimed "tail expressions no longer
implicitly return," but it didn't cover that case, so the test didn't fail. That file has been
replaced by `tests/yaoxiang/03-semantics/tail_expr_and_return.yx`.

## Proposal

### Three Rules

```
① Block value = tail expression (sole exit point)
   Assignment statement's value is Void; empty block {}'s value is Void
② return : (T) -> Never
   Non-local exit: exits the nearest function boundary
③ if without else → Void
```

**These three suffice to derive "early return"—no extra rule making `return` specifically belong to
a function is needed.**

### Rule ①: Block's Value

The **last statement/expression of a block** is the block's value (the tail expression). This is
**not** "no tail expression means Void"—a non-empty block always has a tail expression (the very
last statement is one), and an empty block `{}` has value `Void`.

```yaoxiang
// The tail expression determines the block's value
a = {
    x = compute()        // Assignment statement → Void
    x * 2                // Tail expression → block's value
}

// Assignment as tail expression → block's value is Void
b = {
    x = compute()
    log(x)               // Assignment statement → Void
}

// If you want Void, write it explicitly
c = {
    log(x)
    Void                 // Explicit Void
}
```

**Design rationale (separation of mechanism and protection)**:

- **The language rule only provides the mechanism**: the last statement is the block's value—single
  rule, no ambiguity
- **Protection is provided by type checking**: function declared `-> Int` but tail expression's type
  mismatches → compile error
- **The language does not guard against "wrong intent"**: if the tail expression's type happens to
  match the return type but the semantics is unintended, that's the author's carelessness—the
  language cannot tell. **The language does not rely on rules to prevent intent errors.**
- **If you don't want to return, write `Void` explicitly**

This replaces RFC-010's "= { ... } must use `return`, otherwise returns `Void`," and also replaces
its design rationale that "explicit `return` is needed to disambiguate whether the last expression
is the return value."

### Rule ②: `return` Is a Non-Local Exit

`return` has type `Never` (zero constructors, no value can inhabit it). Its semantics:

- **Exits the nearest function boundary**, delivering the value to the caller
- **Pierces through all blocks**—(if any) `if` / `while` / `for` / `match` / bare block / `spawn` /
  `unsafe`

`return` does not "return to a block." As a block, `{ return n }` has value `n` (tail expression
rule), **of type `Never`**; at the same time, `return`'s effect is to exit the function. **Both hold
simultaneously.**

### Rule ③: `if` Without `else`

```yaoxiang
x = if c { 19 }        // no else
```

When the condition is false there is no branch to evaluate, so take `Void`. Hence this `if`'s value
type is `Void` (or it cannot be used at a non-`Void` position).

When a value is needed, supply both branches explicitly:

```yaoxiang
x = if c { 19 } else { 20 }
```

### `Never` Is the Technical Foundation of Coexistence

`Never <: T` holds for any type `T` (the bottoms-out principle; see Language Specification §Type
System). Therefore:

```yaoxiang
fib: (n: Int) -> Int = {
  if n <= 1 {
    return n              // Tail expression : Never
  }                       // → this if's value : Never
  fib(n - 1) + fib(n - 2) // → block's value : Int
}
```

- The branch body `{ return n }`'s value is `Never`
- This `if`'s only branch is `Never` ⇒ `if`'s value is `Never`
- A statement of type `Never` means **the sequence terminates here**—the following statement is not
  "the next one to execute in order"
- And `Never` can be coerced to any type (bottoms-out principle), so the entire block satisfies
  `-> Int`

**"Early return" naturally follows from this**: `Never` terminates the sequence + the bottoms-out
principle allows coercion.

## Detailed Design

### Formalizing Blocks and Tail Expressions

```
Block        ::= '{' Stmt* '}'                     // Empty block → Void
               | '{' Stmt* Expr '}'                // Value = Expr
Expr         ::= ...
               | Return                            // Type Never
Stmt         ::= Assignment | ExprStmt | ...

Value(Block):
  Empty block                → Void
  { ...; e }                 → type(e)
  { ...; s } (s is a stmt)   → Void          // Assignment's value is Void
```

### Type Rule for `return`

```
return e : Never        where e : T

// Because Never <: T' for any T', return can appear at any return-type position
```

**No extra rule is needed to constrain `return`'s legality**—the bottoms-out principle already
covers it. This makes questions like "can `return` appear in a function returning `X`" disappear.

### Joining Multiple Branches

```
join(A, B):
  If A : Never  → B
  If B : Never  → A
  Otherwise     → require A, B to be compatible (same type or reach a common upper bound)
```

`if` / `match`'s multiple branches are joined by `join`. Branches with `return` don't participate in
joining because their type is `Never`.

### Consistency with RFC-007

RFC-007's example **fully conforms to this RFC**—no revision needed:

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }          // Never branch, ignored by join
    return n * factorial(n - 1)     // Function exit
}
```

"Early return" is not a special case; it's a **corollary** of rules ①, ②, and the bottoms-out
principle.

### Differences from RFC-010 and Revision Requirements

RFC-010's definitions have been revised per this RFC; the differences are as follows:

| RFC-010 original clause                                                     | Current definition                                                                      |
| --------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| "= { ... } must use `return`, otherwise returns `Void`"                     | Block's value = tail expression; empty block → `Void`                                   |
| Design rationale "explicit `return` needed to disambiguate tail expression" | No longer holds—tail expression causes no ambiguity, `return` doesn't interfere with it |
| Three examples with `return c` / `return SqliteDb`                          | Tail expression form (value exits of `spawn` / `unsafe` unified)                        |

**RFC-010's core design (`{}` as a dependency-driven computation unit) is unchanged**—only the value
exit changed from `return` to tail expression.

### Unifying Perspectives (Design Principle)

The braces of `if` / `while` are **both imperative control bodies and declarative evaluation
units**—this is a **difference in perspective, not two different language constructs**. Therefore:

- **Do not split at the language level** between "control-flow bodies" and "evaluation units"
- All blocks share the same set of evaluation rules (rule ①)
- `return` is the only exceptional mechanism, and its exceptional nature comes from `Never`'s
  type-theoretic property, not a syntactic special case

This allows declarative and Python-style writing to coexist naturally:

```yaoxiang
a = if input > 10 { 19 } else { 20 }
b = { x = f(); x * 2 }
c = spawn { fetch("a") }
```

## Trade-offs

### Advantages

- **Eliminates RFC conflicts**: RFC-007 and RFC-010 shift from contradiction to a corollary
  relationship, without sacrificing either
- **Fewest rules**: three rules + one type-theoretic property (bottoms-out principle), no special
  cases
- **Single meaning for `return`**: exits the function. No more judging whether it belongs to "block
  value" or "function value"
- **Unified perspective**: imperative and declarative coexist, no language-level split
- **Consistent with mature languages**: Rust likewise coexists via "tail expression + `return : !`"
- **Zero new syntax**: no new keywords, no parser grammar changes

### Disadvantages

- **"Accidental return" risk from last-expression-is-value**: when the last line is left by
  accident, the return value silently changes
  - Mitigation: type checking intercepts type mismatches; the language does not promise to prevent
    intent errors (see design rationale)
- **Downstream documentation needs synchronized revision**: examples in accepted documents must be
  rewritten
- **Interaction with statement termination must be clarified**: does a newline-terminated last
  expression still count as the block's value (see open issues)

## Alternatives

### Option A: `return` Retains Dual Jobs, Dispatched by Containing Block Type

In function body `= {}`, `return` belongs to the function; in `spawn {}` / `unsafe {}`, it belongs
to the block; in `if {}` / bare blocks, it belongs to…?

**Reason for rejection**: `if`'s assignment is unresolvable—this is the original conflict. And users
must remember "which blocks belong to whom," with no principled basis (why should `if` belong to the
function but a bare block to itself?).

### Option B: Strict Block Return (`return` Always Belongs to the Current Block)

**Reason for rejection**: **Missing functionality**. `return` can never early-exit from a nested
block, so guard clauses (`if err { return }`) are completely unwritable, and functions can only be
written as deeply nested expressions.

### Option C: New Keyword for Block's Value (`give x` / `yield x`)

**Reason for rejection**: Violates RFC-036's **zero-syntax-change** principle (must add new
keywords), and users must learn two concepts (`return` to exit + `give` to evaluate). The tail
expression approach needs zero new concepts.

### Option D: Keep RFC-010 as Originally Written (Explicit `return` Required)

**Reason for rejection**: Irreconcilably conflicts with RFC-007 (see Motivation). And the actual
implementation has already gone down the tail expression path.

## Revisions to Downstream Documentation

This RFC's definition has become the authoritative semantics for downstream documentation; the
relevant documents have all been synchronized (no deprecated wording is retained).

## Open Issues

- [x] Interaction between tail expression and RFC-038's statement termination rules: does a
      newline-terminated last expression still count as the block's value? (@Chenxu: needs to be
      confirmed together with RFC-038's "line-start `(`/`[` never merges" rules) — Verified by test:
      newline-terminated last expressions (including line-start `(` / `[` / list literals) all count
      as block values
- [x] When is `name = { ... }` a function vs. a block value binding—see Appendix D (content
      determines the type)
- [x] Specific forms after rewriting `unsafe {}` / `spawn {}`—both verified usable with tail
      expressions
- [x] Interaction between `match` branch `join` and exhaustiveness checking (depends on RFC-010b) —
      Verified: `Never` branches don't participate in joining; multi-branch `if` / `match` join
      behavior is correct
- [x] Diagnostic message for empty block `{}` as function body with non-`Void` return type—reuse
      existing `E1012`, position points to the annotation

---

## Appendix A: Empirical Evidence

The following were all reproduced on 0.8.0.

| Code                                              | Empirical result                  |
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
| `while { if i==2 { return 42 } }`                 | `42` (pierces through loop)       |
| Nested `{ { return 5 } return 1 }`                | `5` (pierces through bare block)  |

## Appendix B: Design Decision Log

| Decision                             | Determination                                                                     | Reason                                                            | Date       |
| ------------------------------------ | --------------------------------------------------------------------------------- | ----------------------------------------------------------------- | ---------- |
| `return` semantics                   | Function exit, type `Never`; does not return to a block                           | Eliminates the category error of one word bearing two jobs        | 2026-09-15 |
| Block's value exit                   | Tail expression (sole exit)                                                       | Fewest rules; consistent with Rust                                | 2026-09-15 |
| No tail expression                   | Doesn't exist (non-empty block always has a tail expression; empty `{}` → `Void`) | If you want `Void`, write `Void` explicitly                       | 2026-09-15 |
| Assignment's value                   | `Void`                                                                            | Assignment is a statement, not a value producer                   | 2026-09-15 |
| `if` without `else`                  | `Void`                                                                            | No branch to evaluate when the condition is false                 | 2026-09-15 |
| Division of mechanism and protection | Language provides mechanism, type checking provides protection                    | Language does not promise to prevent "wrong intent"               | 2026-09-15 |
| Control body / evaluation unit       | Not split at the language level; treat as difference in perspective               | Imperative and declarative coexist; avoid unprincipled exceptions | 2026-09-15 |
| Handling of erroneous examples       | Delete directly; do not retain erroneous code                                     | Retaining seemingly usable erroneous code will mislead            | 2026-09-15 |

## Appendix C: Glossary

| Term                  | Definition                                                                                                          |
| --------------------- | ------------------------------------------------------------------------------------------------------------------- |
| Tail expression       | The last value-producing expression in a block; the block's value equals it                                         |
| Non-local exit        | A control-flow transfer that traverses outer evaluation units and acts directly on the function boundary (`return`) |
| Bottoms-out principle | `Never <: T` holds for any `T`, so `Never` can be coerced to any type                                               |
| Value-bearing block   | `= {}` / `spawn {}` / `unsafe {}`—the value exit is the tail expression                                             |
| join                  | Multi-branch merge rule; `Never` branches don't participate in joining                                              |

## Appendix D: Function / Block Value Disambiguation

### The Problem

`name = { ... }` was once defined differently by two accepted RFCs: RFC-007 (function syntax)
treated it as a function ("zero-arg minimal" `name = { return ... }`), while RFC-010 / 010a treated
it as a block value (`= {}` is a value-bearing block, whose value is the tail expression). The same
syntactic position, two sets of semantics, led in the implementation to `callable_parts()`
registering the block as a 0-arg function while `generate_block_ir` takes it as a block value—the
two levels of understanding are inconsistent.

### Resolution: Content Determines the Type

The same principle should pervade dictionaries and blocks:

| Case                          | Result                    | Basis                                    |
| ----------------------------- | ------------------------- | ---------------------------------------- |
| `value` is a `Lambda` (`=>`)  | Function                  | `=>` is an explicit function constructor |
| Annotation is `Fn`            | Function                  | Declared a function type                 |
| Annotation is a non-`Fn` type | Block value               | Annotation is the type (`x: Int = {..}`) |
| **No annotation**             | **Inferred from content** | **Type determined by content**           |

```yaoxiang
x: Int = { y = 5; y }      // Block value: x = 5 (annotation is non-Fn)
f: () -> Int = { 5 }       // Function: f() = 5 (annotation is Fn)
f = { 5 }                  // Value: f = 5 (no annotation → content inference)
f = {}                     // Value: f = Void (empty block)
b = () => 5                // Function: explicit lambda
d = { "a": 1 }             // Value: Dict (content self-describing)
```

**Why not "no annotation defaults to function"**: that would make `f = { 5 }` a function, while
`d = { "a": 1 }` is a dictionary—the same `{` in the same no-annotation position yielding different
categories of things. The three reasons once considered all fail:

1. Zero changes to existing code—migration cost is payable (211 files, 704 sites), not a semantic
   basis
2. Function definition is high-frequency, block value is low-frequency—frequency is not a type rule
3. RFC-007 is already accepted; changing it costs more than changing this RFC—changing a wrong thing
   doesn't make it right because it's "expensive"

**Type is determined by content**, not by the presence or absence of an annotation. The annotation
still declares the type (`f: () -> Int`), but does not impose a default value just because the
annotation is absent.

### Where `{}` Falls

`Dict`'s grammar requires at least one key; `{}` has no content to base on, so it takes the zero
form of block structure → empty block, value `Void`. Use `dict.new()` for an empty dictionary. See
spec [§2.9.1](../../../reference/language-spec/syntax.md).

### Accompanying Implementation Fixes

1. **Annotation must not determine value-taking**: three places in `generate_function_ir` once used
   `return_type != Void` as the threshold for taking the tail expression's value, causing the
   no-annotation `f = { 5 }` to silently discard the tail expression and return `Void`. The
   annotation only decides whether to _check_, not whether to _evaluate_.
2. **`callable_parts()` no longer unconditionally swallows blocks**: dispatch is now uniformly
   determined by `Expr::block_binding_is_function(annotation, value)`.

## References

- [RFC-007: Function Definition Syntax Unification](./007-function-syntax-unification.md)
- [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)
- [RFC-038: Statement Termination and Line Break Rules](038-statement-termination.md)
- [RFC-030: assert Mechanism](./030-assert-mechanism.md) — Refined type application of `Never`
- [Language Specification §Type System](/reference/language-spec/type-system.md) — ⊥ / ⊤ positioning
  of `Never` / `Void`
- [Rust Reference: `!` never type](https://doc.rust-lang.org/reference/types/never.html) —
  Isomorphic application of the bottoms-out principle
