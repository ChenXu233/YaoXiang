---
title: 'RFC-010a: Tail Expression Evaluation and return Semantics'
status: 'Accepted'
author: 'Chenxu'
created: '2026-09-15'
updated: '2026-09-15 (Accepted)'
group: 'rfc-010'
issue: '#342'
---

# RFC-010a: Tail Expression Evaluation and return Semantics

> **References**:
>
> - [RFC-010: Unified Type Syntax](010-unified-type-syntax.md) — `name: type = value` model, `{}` as
>   a dependency-driven computation unit
> - [RFC-007: Unified Function Definition Syntax](007-function-syntax-unification.md) — Block return
>   rules, early return
> - [RFC-038: Statement Termination and Line Break Rules](038-statement-termination.md) — Line break
>   behavior of statements and expressions
> - [Language Spec §Type System](../../reference/language-spec/type-system.md) — `Never` explosion
>   principle

## Summary

Unify the semantics of `return` and block evaluation, and eliminate the statement conflicts between
RFC-007 and RFC-010.

Propose **three self-consistent rules**: a block's value equals its tail expression (the sole exit);
`return` is a **non-local exit** of type `Never` (exits the function, does not "return to the
block"); `if` without `else` yields `Void`.

`return` and block evaluation **are not bifurcated at the language level** — `{ return n }` as a
block has value `n` (type `Never`), while at the same time `return` serves to exit the function; the
two coexist via the **explosion principle** (`Never <: T`). RFC-007's "early return" and RFC-010's
"block has a value" are consequences of these three rules, not contradictory special cases.

No new syntax, no new keywords.

## Implementation Status

This RFC has been accepted. The implementation status of the three rules:

| Rule                                   | Subitem                                                                                                | Status                                                    |
| -------------------------------------- | ------------------------------------------------------------------------------------------------------ | --------------------------------------------------------- |
| ① Block value = tail expression        | Function body tail expression                                                                          | ✅ Implemented                                            |
| ① Block value = tail expression        | Tail-position `if` / `match`                                                                           | ✅ Implemented (#344)                                     |
| ① Block value = tail expression        | Last assignment statement → `Void`                                                                     | ✅ Implemented                                            |
| ① Block value = tail expression        | Empty block `{}` → `Void`                                                                              | ✅ Implemented                                            |
| ① Protection provided by type checking | Tail expression reconciled with declared return type                                                   | ✅ Implemented (#345)                                     |
| ② `return` is a non-local exit         | Penetrates out of `if`/`while`/`for`/bare blocks/nested blocks                                         | ✅ Implemented                                            |
| ② `return` is a non-local exit         | No function boundary at module initialization layer (Script top level); compile-time rejection (E1109) | ✅ Implemented                                            |
| ② `Never <: T` explosion principle     | Consistent in `unify` and `is_subtype`                                                                 | ✅ Implemented (this fix resolves internal contradiction) |
| ③ `if` without `else` → `Void`         | Branch value does not leak at expression position                                                      | ✅ Implemented (#346)                                     |
| ① Block value = tail expression        | Bare block value binding `x = { ... }`                                                                 | ✅ Implemented (#343)                                     |
| ① Block value = tail expression        | `unsafe {}` value exit                                                                                 | ✅ Implemented (#347)                                     |
| ① Protection provided by type checking | Empty block / last-statement escape check                                                              | ✅ Implemented (Open Issue 5 of #342)                     |
| ① Block value = tail expression        | `spawn {}` value exit (tail expression)                                                                | ✅ Implemented (#365)                                     |

Corpus coverage: `tests/yaoxiang/03-semantics/rfc010a_block_value.yx` (positive) +
`tests/yaoxiang/06-compile-errors/tail_expr_type_mismatch{,_with_stmts}_err.yx` (negative) +
`tests/yaoxiang/06-compile-errors/top_level_return*_err.yx` (Rule ② boundary negatives: with-value /
no-value / inside bound-value block / inside if block / inside unsafe block).

## Motivation

### Statement Conflict

The Fibonacci example in the playground exposed a semantic divergence:

```yaoxiang
fib: (n: Int) -> Int = {
  if n <= 1 {
    return n          // Is this return the "if's" or the "function's"?
  }
  return fib(n - 1) + fib(n - 2)
}
```

Two already-accepted RFCs give **opposite** derivations:

| RFC                    | Statement                                                                                                                               | Derived semantics                                                            |
| ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| **RFC-007** (accepted) | Using `factorial` as an example, the title reads **"Early return: using return"**: `if n <= 1 { return 1 }`                             | `return` passes through `if` and the function body, **back to the function** |
| **RFC-010** (accepted) | "`{}` is a dependency-driven computation unit… use `return` to explicitly return a value"; `spawn { return c }` returns the task result | `return` gives **this `{}`** its value                                       |

According to RFC-010, the braces of `if n <= 1 { return 1 }` are a computation unit, and `return 1`
gives it the value 1; then the value of the `if` statement is discarded, and the next line is
necessarily executed — **fib recurses infinitely**. According to RFC-007, the behavior is correct.

### Root Cause: `return` Wearing Two Hats

- **RFC-007 uses it to express "exit the function"** — a control-flow concept
- **RFC-010 uses it to express "this block's value is it"** — an evaluation concept

Using a control-flow keyword to express evaluation is a **category error**. When one word carries
two duties, no matter how you tune it, one side is sacrificed.

### Downstream Documentation's Erroneous Over-Extension

`docs/src/reference/language-spec/syntax.md` extends RFC-010's "`{}` block" to **all braces**:

- §2.9: "`return` inside `{}` **always returns its content to the enclosing scope**" (and calls it
  "unified semantics: all `{}` blocks")
- §3.3: "`return` is used to **return a value from a code block**"

Yet RFC-010's original text defines **three kinds of valued blocks** (`= {}` / `spawn {}` /
`unsafe {}`), and **never mentions the braces of `if` / `while` / `for` / `match`**.

### Implementation Status

| Behavior                                        | Current Status                                   |
| ----------------------------------------------- | ------------------------------------------------ |
| `if n == 0 { return 7 } … return 8`             | Function exits (`h(0) = 7`)                      |
| `f = { n + 1 }`                                 | Tail expression usable (returns `5`)             |
| `x = { y = 5; y }` (bare block tail expression) | `E3006` variable unresolved (see #343)           |
| `f: () -> Int = { if c {5} else {6} }`          | Returns `void`, tail expression discarded (#344) |
| `f: () -> Int = { "s" }`                        | Silently passes compilation (see #345)           |
| `x = if c { 19 }` (no `else`)                   | `19` (should be `Void`, see #346)                |
| `v = unsafe { 42 }`                             | `void` (see #347)                                |
| `y = if c { 111 } else { 222 }`                 | Usable (`if` as expression)                      |

`tests/yaoxiang/03-semantics/no_tail_expr_return.yx` once claimed that "the tail expression is no
longer implicitly returned," but it did not cover this case, so the test did not fail. That file has
been replaced by `tests/yaoxiang/03-semantics/tail_expr_and_return.yx`.

## Proposal

### The Three Rules

```
① Block value = tail expression (the sole exit)
   The value of an assignment statement is Void; the value of the empty block {} is Void
② return : (T) -> Never
   Non-local exit: exits the nearest function boundary
③ if without else → Void
```

**These three suffice to derive "early return" — no additional rule that `return` specifically
targets the function is needed.**

### Rule ①: Block Value

A block's **last statement/expression** is the block's value (the tail expression). This is **not**
"no tail expression means Void" — a non-empty block always has a tail expression (the very last
statement is one), and the value of the empty block `{}` is `Void`.

```yaoxiang
// Tail expression determines the block's value
a = {
    x = compute()        // Assignment statement → Void
    x * 2                // Tail expression → block's value
}

// Assignment as tail expression → block's value is Void
b = {
    x = compute()
    log(x)               // Assignment statement → Void
}

// Write Void explicitly when you want Void
c = {
    log(x)
    Void                 // Explicit Void
}
```

**Design criterion (separation of mechanism and protection)**:

- **The language rule provides only the mechanism**: the last statement is the block value; the rule
  is unique and unambiguous
- **Protection is provided by type checking**: function declares `-> Int` but the tail expression's
  type mismatches → compilation error
- **The language does not guard against "wrong intent"**: if the tail expression's type happens to
  match the return type but the semantics is unintended, that is the author's oversight — the
  language has no way to tell. **Do not rely on language rules to guard against wrong intent.**
- **Write `Void` explicitly when you don't want to return a value**

This replaces RFC-010's " `= { ... }` must use `return`, otherwise it returns `Void`," and also
replaces its design rationale "explicit `return` is needed to remove the ambiguity of whether the
last expression is the return value."

### Rule ②: `return` Is a Non-Local Exit

`return`'s type is `Never` (zero constructors, no inhabitable value). Its semantics:

- **Exits the nearest function boundary**, passing the value to the caller
- **Penetrates any blocks** — (if any) `if` / `while` / `for` / `match` / bare blocks / `spawn` /
  `unsafe`

**What constitutes a function boundary (the precondition for Rule ②)**: a named function body, a
lambda body, and a `spawn` body each constitute a function boundary; `return` exits the nearest one.
**The module initialization layer does not constitute a function boundary** — the top-level
statement sequence at Script's top level (single file without `yaoxiang.toml`, SPEC syntax §3.11)
has no "nearest function boundary" to exit from, so `return` there is **rejected at compile time**
(`E1109`): allowing it would silently terminate initialization at runtime and skip the remaining
top-level statements. A block value is not a function boundary, so `return` inside a top-level
bound-value block is likewise rejected.

`return` does not "return to the block." `{ return n }`, as a block, has value `n` (tail-expression
rule), of **type `Never`**; at the same time, `return`'s effect is to exit the function. **Both hold
simultaneously.**

### Rule ③: `if` Without `else`

```yaoxiang
x = if c { 19 }        // No else
```

When the condition is false there is no branch to evaluate, so it yields `Void`. Thus this `if`'s
value type is `Void` (or it cannot be used in a non-`Void` position).

When a value is needed, supply both branches explicitly:

```yaoxiang
x = if c { 19 } else { 20 }
```

### `Never` Is the Technical Foundation for Coexistence

`Never <: T` holds for any type `T` (the explosion principle; see Language Spec §Type System).
Therefore:

```yaoxiang
fib: (n: Int) -> Int = {
  if n <= 1 {
    return n              // Tail expression : Never
  }                       // → Value of this if : Never
  fib(n - 1) + fib(n - 2) // → Block value : Int
}
```

- The branch body `{ return n }` has value `Never`
- This `if`'s only branch is `Never` ⇒ the `if`'s value is `Never`
- A statement of type `Never` means **the sequence aborts here** — the following statement is not
  "the next one executed in order"
- And `Never` can be coerced to any type (explosion principle), so the entire block satisfies
  `-> Int`

**"Early return" is naturally derived from this**: `Never` aborts the sequence + the explosion
principle allows coercion.

## Detailed Design

### Formalization of Blocks and Tail Expressions

```
Block        ::= '{' Stmt* '}'                     // Empty block → Void
               | '{' Stmt* Expr '}'                // Value = Expr
Expr         ::= ...
               | Return                            // Type Never
Stmt         ::= Assignment | ExprStmt | ...

Value(Block):
  Empty block              → Void
  { ...; e }               → Type(e)
  { ...; s } (s is a statement) → Void   // The value of an assignment is Void
```

### Type Rule for `return`

```
return e : Never        where e : T

// Because Never <: T' holds for any T', return can appear in any return-type position
```

**No additional rule is needed to constrain the legality of `return`** — the explosion principle
already covers it. This makes the question "can `return` appear in a function that returns `X`"
simply disappear.

### Merging of Multiple Branches

```
join(A, B):
  If A : Never  → B
  If B : Never  → A
  Otherwise     → Require A and B to be compatible (same type or reachable common upper bound)
```

The multiple branches of `if` / `match` are merged by `join`. A branch with `return` does not
participate in the merge because its type is `Never`.

### Consistency with RFC-007

RFC-007's examples are **fully compliant with this RFC**, and require no revision:

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }          // Never branch, ignored by join
    return n * factorial(n - 1)     // Function exit
}
```

"Early return" is not a special case — it is a **consequence** of Rules ① and ② together with the
explosion principle.

### Differences from RFC-010 and Revision Requirements

RFC-010's definitions have been revised in line with this RFC. The differences are:

| RFC-010 Original Clause                                                         | Current Definition                                                                                     |
| ------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| "`= { ... }` must use `return`, otherwise it returns `Void`"                    | Block value = tail expression; empty block → `Void`                                                    |
| Design rationale "explicit `return` needed to remove tail-expression ambiguity" | No longer holds — tail expressions do not introduce ambiguity, and `return` does not interfere with it |
| Three `return c` / `return SqliteDb` examples                                   | Tail-expression form (unified value exit for `spawn` / `unsafe`)                                       |

**RFC-010's core design (`{}` as a dependency-driven computation unit) is unchanged**; only the
value exit is changed from `return` to the tail expression.

### Unified Perspective (Design Principle)

The braces of `if` / `while` are **both imperative control bodies and declarative evaluation units**
— this is a **difference of perspective, not two language constructs**. Therefore:

- **Do not bifurcate at the language level** between "control-flow body" and "evaluation unit"
- All blocks share the same evaluation rule (Rule ①)
- `return` is the sole exception mechanism, and its exceptional nature comes from the type-theoretic
  property of `Never`, not from a syntactic special case

This lets declarative and Python-style writing coexist naturally:

```yaoxiang
a = if input > 10 { 19 } else { 20 }
b = { x = f(); x * 2 }
c = spawn { fetch("a") }
```

## Trade-offs

### Advantages

- **Eliminates RFC conflict**: RFC-007 and RFC-010 go from being contradictory to a consequence
  relationship, with neither side sacrificed
- **Fewest rules**: three rules + one type-theoretic property (the explosion principle), no special
  cases
- **Single meaning of `return`**: exits the function. No more judging whether it belongs to "block
  value" or "function value"
- **Unified perspective**: imperative and declarative coexist without language-level bifurcation
- **Consistent with mature languages**: Rust likewise lets "tail expression + `return : !`" coexist
- **Zero new syntax**: no new keywords, no changes to parser grammar rules

### Disadvantages

- **"Accidental return" risk from the last expression**: forgetting to delete the last line silently
  changes the return value
  - Mitigation: type checking intercepts type mismatches; the language does not promise to guard
    against wrong intent (see Design Criterion)
- **Downstream documentation must be revised in sync**: examples in already-accepted documents must
  be rewritten
- **Interaction with statement termination must be clarified**: does a line-break-terminated last
  expression still count as a block value? (see Open Issues)

## Alternatives

### Option A: Keep `return`'s dual role; dispatch by enclosing block type

In function body `= {}`, `return` belongs to the function; in `spawn {}` / `unsafe {}`, it belongs
to the block; in `if {}` / bare blocks, it belongs to whom?

**Reason for rejection**: the ownership of `if` cannot be adjudicated — this is exactly the original
conflict. And the user must remember "which blocks assign to whom," with no principled basis (why
does `if` belong to the function while a bare block belongs to itself?).

### Option B: Strict block return (`return` always belongs to the current block)

**Reason for rejection**: **missing functionality**. `return` could never early-exit a function from
a nested block; guard clauses (`if err { return }`) would be completely unwritable, and functions
could only be written as deeply nested expressions.

### Option C: Use a new keyword for the block's value (`give x` / `yield x`)

**Reason for rejection**: violates RFC-036's **zero-syntax-change** principle (requires new
keywords), and the user must learn two concepts (`return` for exit + `give` for evaluation). The
tail-expression approach introduces zero new concepts.

### Option D: Keep RFC-010 as-is (explicit `return` required)

**Reason for rejection**: irreconcilably conflicts with RFC-007 (see Motivation). And the actual
implementation has already taken the tail-expression path.

## Revisions to Downstream Documentation

The definitions in this RFC have become the authoritative semantics for downstream documentation;
all related documents have been synchronized (obsolete wording is no longer retained).

## Open Issues

- [x] Interaction between tail expressions and RFC-038's statement-termination rules: does a
      line-break-terminated last expression still count as a block value? (@Chenxu: needs to be
      confirmed together with RFC-038's "line-leading `(` / `[` never merges" etc. rules) —
      Empirically verified: line-break-terminated last expressions (including line-leading `(` / `[`
      / list literals) all count as block values
- [x] When is `name = { ... }` a function vs. a block-value binding — see Appendix D (content
      determines type)
- [x] Specific forms after rewriting `unsafe {}` / `spawn {}` — both tail expressions empirically
      verified usable
- [x] Interaction between `match` branch `join` and exhaustiveness checking (depends on RFC-010b) —
      Empirically verified: `Never` branches do not participate in merging, and the join behavior of
      multi-branch `if` / `match` is correct
- [x] Diagnostic wording when the empty block `{}` is used as a function body whose return type is
      not `Void` — reuses the existing `E1012`, with the position pointing to the annotation

---

## Appendix A: Empirical Evidence

All reproduced on 0.8.0.

| Code                                              | Empirically Observed               |
| ------------------------------------------------- | ---------------------------------- |
| `h: (n)->Int = { if n==0 { return 7 } return 8 }` | `h(0)=7`, `h(1)=8`                 |
| `f: (n)->Int = { n + 1 }`                         | `5` (tail expression usable)       |
| `f: ()->Int = { if c {5} else {6} }`              | `void` (should be `5`, see #344)   |
| `f: ()->Int = if c {5} else {6}`                  | `5`                                |
| `f: ()->Int = { match ... }`                      | Correct                            |
| `f: ()->Int = { while ...; i }`                   | Correct                            |
| `{ y = 5; y }` as a binding expression            | `E3006` (see #343)                 |
| `f: ()->Int = { "s" }`                            | Silently passes (see #345)         |
| `x = if c { 19 }` (no `else`)                     | `19` (should be `Void`, see #346)  |
| `v = unsafe { 42 }`                               | `void` (see #347)                  |
| `y = if c { 111 } else { 222 }`                   | `111`                              |
| `while { if i==2 { return 42 } }`                 | `42` (passes out of the loop)      |
| Nested `{ { return 5 } return 1 }`                | `5` (passes out of the bare block) |

## Appendix B: Design Decision Record

| Decision                             | Decision                                                                                   | Reason                                                                  | Date       |
| ------------------------------------ | ------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------- | ---------- |
| `return` semantics                   | Function exit, type `Never`; does not return to the block                                  | Eliminate the category error of one word carrying two duties            | 2026-09-15 |
| Block's value exit                   | Tail expression (sole exit)                                                                | Fewest rules; consistent with Rust                                      | 2026-09-15 |
| No tail expression                   | Does not exist (a non-empty block always has a tail expression; empty block `{}` → `Void`) | Write `Void` explicitly when you want `Void`                            | 2026-09-15 |
| Value of assignment                  | `Void`                                                                                     | Assignment is a statement, not value production                         | 2026-09-15 |
| `if` without `else`                  | `Void`                                                                                     | When the condition is false there is no branch to evaluate              | 2026-09-15 |
| Division of mechanism and protection | Language provides mechanism, type checking provides protection                             | The language does not promise to guard against "wrong intent"           | 2026-09-15 |
| Control-flow body / evaluation unit  | Not bifurcated at the language level; treated as a perspective difference                  | Imperative and declarative coexist, avoiding unprincipled special cases | 2026-09-15 |
| Handling of wrong examples           | Delete them directly; do not keep wrong code                                               | Keeping seemingly usable wrong code would mislead                       | 2026-09-15 |

## Appendix C: Glossary

| Term                | Definition                                                                                                        |
| ------------------- | ----------------------------------------------------------------------------------------------------------------- |
| Tail expression     | The last value-producing expression in a block; the block's value is it                                           |
| Non-local exit      | A control-flow transfer that traverses outer evaluation units and acts directly on a function boundary (`return`) |
| Explosion principle | `Never <: T` holds for any `T`, allowing `Never` to be coerced to any type                                        |
| Valued block        | `= {}` / `spawn {}` / `unsafe {}` — the value exit is the tail expression                                         |
| join                | Multi-branch merging rule; `Never` branches do not participate                                                    |

## Appendix D: Function / Block-Value Ambiguity Adjudication

### The Problem

`name = { ... }` had been defined as two different things by two already-accepted RFCs: RFC-007
(function syntax) treats it as a function ("zero-arg simplest" `name = { return ... }`), while
RFC-010 / 010a treats it as a block value (`= {}` is a valued block, with the value being the tail
expression). At the same syntactic position with two semantics, the implementation caused
`callable_parts()` to register the block as a 0-arg function, while `generate_block_ir` took it as a
block value — a two-level inconsistency.

### Adjudication: Content Determines Type

The same principle should be carried through dicts and blocks:

| Situation                     | Result                    | Basis                                        |
| ----------------------------- | ------------------------- | -------------------------------------------- |
| `value` is a `Lambda` (`=>`)  | Function                  | `=>` is an explicit function constructor     |
| Annotation is `Fn`            | Function                  | Declares a function type                     |
| Annotation is a non-`Fn` type | Block value               | The annotation is the type (`x: Int = {..}`) |
| **No annotation**             | **Inferred from content** | **The type is determined by the content**    |

```yaoxiang
x: Int = { y = 5; y }      // Block value: x = 5 (annotation is not Fn)
f: () -> Int = { 5 }       // Function: f() = 5 (annotation is Fn)
f = { 5 }                  // Value: f = 5 (no annotation → inferred from content)
f = {}                     // Value: f = Void (empty block)
b = () => 5                // Function: explicit lambda
d = { "a": 1 }             // Value: Dict (content is self-describing)
```

**Why not "no annotation defaults to function"**: that would make `f = { 5 }` a function, while
`d = { "a": 1 }` would be a dict — the same `{` in the same no-annotation position would yield two
different categories of thing. None of the three reasons once considered hold up:

1. Zero changes to existing code — the migration cost is payable (211 files, 704 sites), not a
   semantic justification
2. Function definition is high-frequency, block value is low-frequency — frequency is not a type
   rule
3. RFC-007 is already accepted; changing it is more expensive than changing this RFC — wrong things
   don't become right because they are "expensive" to fix

**The type is determined by the content**, not by the presence of an annotation. The annotation
still declares the type (`f: () -> Int`), but its absence does not impose a default.

### The Placement of `{}`

The `Dict` grammar requires at least one key. `{}` has no content to rely on, so it takes the zero
form of block structure → empty block, value `Void`. Use `dict.new()` for an empty dict. See spec
[§2.9.1](../../reference/language-spec/syntax.md).

### Accompanying Implementation Fixes

1. **Annotation must not decide whether to take a value**: three places including
   `generate_function_ir` once used `return_type != Void` as the threshold for taking a tail
   expression, causing the tail expression of an unannotated `f = { 5 }` to be silently discarded,
   returning `Void`. The annotation only decides _whether to check_, not _whether to evaluate_.
2. **`callable_parts()` no longer unconditionally swallows blocks**: the dispatch is unified through
   `Expr::block_binding_is_function(annotation, value)`.

## References

- [RFC-007: Unified Function Definition Syntax](007-function-syntax-unification.md)
- [RFC-010: Unified Type Syntax](010-unified-type-syntax.md)
- [RFC-038: Statement Termination and Line Break Rules](038-statement-termination.md)
- [RFC-030: assert Mechanism](030-assert-mechanism.md) — Refined-type applications of `Never`
- [Language Spec §Type System](../../reference/language-spec/type-system.md) — `Never` / `Void` ⊥ /
  ⊤ positioning
- [Rust Reference: `!` never type](https://doc.rust-lang.org/reference/types/never.html) —
  Isomorphic application of the explosion principle
