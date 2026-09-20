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
> - [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md) — the `name: type = value` model,
>   `{}` as a dependency-driven computation unit
> - [RFC-007: Unified Function Definition Syntax](./007-function-syntax-unification.md) — block
>   return rules, early return
> - [RFC-038: Statement Termination and Newline Rules](038-statement-termination.md) — newline
>   behavior of statements and expressions
> - [Language Spec §Type System](/reference/language-spec/type-system.md) — the `Never` explosion
>   principle

## Summary

Unify the semantics of `return` and block evaluation, eliminating the statement conflicts between
RFC-007 and RFC-010.

Three **self-consistent rules** are proposed: a block's value equals its tail expression (the sole
exit); `return` is a **non-local exit** of type `Never` (exits the function, does not "return to the
block"); `if` without `else` yields `Void`.

`return` and block evaluation are **not bifurcated at the language level**—`{ return n }` as a block
has value `n` (type `Never`), while `return`'s effect is to exit the function; both facts coexist
via the **explosion principle** (`Never <: T`). RFC-007's "early return" and RFC-010's "blocks have
values" are corollaries of these three rules, not conflicting special cases.

No new syntax, no new keywords.

## Implementation Status

This RFC has been accepted. The implementation status of the three rules:

| Rule                                   | Sub-item                                                      | Status                                      |
| -------------------------------------- | ------------------------------------------------------------- | ------------------------------------------- |
| ① Block value = tail expression        | Function body tail expression                                 | ✅ Implemented                              |
| ① Block value = tail expression        | Tail-position `if` / `match`                                  | ✅ Implemented (#344)                       |
| ① Block value = tail expression        | Trailing assignment statement → `Void`                        | ✅ Implemented                              |
| ① Block value = tail expression        | Empty block `{}` → `Void`                                     | ✅ Implemented                              |
| ① Protection provided by type checking | Tail expression unified with declared return type             | ✅ Implemented (#345)                       |
| ② `return` is a non-local exit         | Piercing through `if`/`while`/`for`/bare blocks/nested blocks | ✅ Implemented                              |
| ② `Never <: T` explosion principle     | `unify` and `is_subtype` consistent                           | ✅ Implemented (resolves internal conflict) |
| ③ `if` without `else` → `Void`         | Branch value not leaked in expression position                | ✅ Implemented (#346)                       |
| ① Block value = tail expression        | Bare-block value binding `x = { ... }`                        | ✅ Implemented (#343)                       |
| ① Block value = tail expression        | `unsafe {}` value exit                                        | ✅ Implemented (#347)                       |
| ① Protection provided by type checking | Empty block / trailing statement escape check                 | ✅ Implemented (#342 open question 5)       |
| ① Block value = tail expression        | `spawn {}` value exit (tail expression)                       | ✅ Implemented (#365)                       |

Test coverage: `tests/yaoxiang/03-semantics/rfc010a_block_value.yx` (positive) +
`tests/yaoxiang/06-compile-errors/tail_expr_type_mismatch{,_with_stmts}_err.yx` (negative).

## Motivation

### Statement Conflicts

A Fibonacci example in the playground revealed a semantic divergence:

```yaoxiang
fib: (n: Int) -> Int = {
  if n <= 1 {
    return n          // Is this `return` for the "if" or for the "function"?
  }
  return fib(n - 1) + fib(n - 2)
}
```

Two already-accepted RFCs offer **opposing** derivations:

| RFC                    | Statement                                                                                                                               | Derived Semantics                                                                  |
| ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| **RFC-007** (Accepted) | Using `factorial` as the example, titled **"Early Return: Using `return`"**: `if n <= 1 { return 1 }`                                   | `return` pierces through `if` and the function body, **returning to the function** |
| **RFC-010** (Accepted) | "`{}` is a dependency-driven computation unit… use `return` to explicitly return a value"; `spawn { return c }` returns the task result | `return` gives its value **to this `{}`**                                          |

Under RFC-010, the braces of `if n <= 1 { return 1 }` are a computation unit; `return 1` makes its
value 1, so the `if` statement's value is discarded, and the next line must execute—**fib recurses
infinitely**. Under RFC-007, it is correct.

### Root Cause: `return` Has Two Jobs

- **RFC-007 uses it to mean "exit the function"**—a control-flow concept
- **RFC-010 uses it to mean "this block's value is it"**—an evaluation concept

Using a control-flow keyword to express evaluation is a category error. One word doing two jobs
means no matter how it's tuned, one side will be sacrificed.

### Downstream Documentation's Erroneous Overextension

`docs/src/reference/language-spec/syntax.md` extended RFC-010's "`{}` block" to **all braces**:

- §2.9: "`return` in `{}` **always returns its content to the enclosing scope**" (and calls it
  "unified semantics: all `{}` blocks")
- §3.3: "`return` is used to **return a value from a code block**"

But RFC-010 originally defined **three value-bearing blocks** (`= {}` / `spawn {}` / `unsafe {}`),
and **never discussed the braces of `if` / `while` / `for` / `match`**.

### Current Implementation

| Behavior                                        | Current State                                    |
| ----------------------------------------------- | ------------------------------------------------ |
| `if n == 0 { return 7 } … return 8`             | Function exits (`h(0) = 7`)                      |
| `f = { n + 1 }`                                 | Tail expression works (returns `5`)              |
| `x = { y = 5; y }` (bare-block tail expression) | `E3006` variable unresolved (see #343)           |
| `f: () -> Int = { if c {5} else {6} }`          | Returns `void`, tail expression discarded (#344) |
| `f: () -> Int = { "s" }`                        | Silently passes compilation (see #345)           |
| `x = if c { 19 }` (no `else`)                   | `19` (should be `Void`, see #346)                |
| `v = unsafe { 42 }`                             | `void` (see #347)                                |
| `y = if c { 111 } else { 222 }`                 | Works (`if` as an expression)                    |

`tests/yaoxiang/03-semantics/no_tail_expr_return.yx` once claimed "tail expressions no longer
implicitly return," but did not cover this case, so the test did not fail. That file has been
replaced by `tests/yaoxiang/03-semantics/tail_expr_and_return.yx`.

## Proposal

### Three Rules

```
① A block's value = its tail expression (the sole exit)
   An assignment statement's value is Void; the empty block {} has value Void
② return : (T) -> Never
   Non-local exit: exits the nearest function boundary
③ if without else → Void
```

**These three rules are sufficient to derive "early return"—no additional rule that `return`
specifically targets the function is needed.**

### Rule ①: A Block's Value

A block's **last statement/expression** is the block's value (the tail expression). This is **not**
"no tail expression → Void"—a non-empty block always has a tail expression (the last statement
itself is one); the empty block `{}` has value `Void`.

```yaoxiang
// The tail expression determines the block's value
a = {
    x = compute()        // Assignment statement → Void
    x * 2                // Tail expression → block's value
}

// Assignment as the tail expression → block's value is Void
b = {
    x = compute()
    log(x)               // Assignment statement → Void
}

// Be explicit when you want Void
c = {
    log(x)
    Void                 // Explicit Void
}
```

**Design criterion (separation of mechanism from protection)**:

- **The language rule only provides the mechanism**: the last statement is the block's
  value—uniquely, unambiguously
- **Protection comes from type checking**: function declared as `-> Int` but tail expression's type
  mismatches → compile error
- **The language does not guard against "wrong intent"**: if the tail expression's type happens to
  match the return type but the semantics are unintended, that's the author's oversight—the language
  cannot tell. **The language does not defend against wrong intent via language rules.**
- **Write `Void` explicitly when you don't want to return**

This replaces RFC-010's "`= { ... }` must use `return`, otherwise returns `Void`", and also replaces
its rationale that "explicit `return` is needed to eliminate the ambiguity of 'is the last
expression the return value'."

### Rule ②: `return` Is a Non-Local Exit

`return` has type `Never` (zero constructors, no inhabitable value). Its semantics:

- **Exits the nearest function boundary**, handing the value to the caller
- **Pierces through all blocks**—(if any) `if` / `while` / `for` / `match` / bare block / `spawn` /
  `unsafe`

`return` does not "return to the block." `{ return n }` as a block has value `n` (by the tail
expression rule), with **type `Never`**; meanwhile, `return`'s effect is to exit the function.
**Both facts hold simultaneously.**

### Rule ③: `if` Without `else`

```yaoxiang
x = if c { 19 }        // No else
```

When the condition is false, no branch can be evaluated, so it yields `Void`. Hence this `if`'s
value type is `Void` (or it cannot be used in a non-`Void` position).

Supply both branches explicitly when a value is needed:

```yaoxiang
x = if c { 19 } else { 20 }
```

### `Never` Is the Technical Foundation of Coexistence

`Never <: T` holds for any type `T` (the explosion principle; see Language Spec §Type System).
Therefore:

```yaoxiang
fib: (n: Int) -> Int = {
  if n <= 1 {
    return n              // Tail expression : Never
  }                       // → this if's value : Never
  fib(n - 1) + fib(n - 2) // → block's value : Int
}
```

- The branch body `{ return n }` has value `Never`
- The only branch of this `if` is `Never` ⇒ the `if`'s value is `Never`
- A statement of type `Never` means the **sequence halts here**—the next line is not "the next one
  executed in order"
- And `Never` can be reduced to any type (the explosion principle), so the whole block satisfies
  `-> Int`

**"Early return" is derived naturally**: `Never` halts the sequence + the explosion principle allows
reduction.

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
  { ...; s } (s a statement) → Void              // Assignment's value is Void
```

### Type Rule for `return`

```
return e : Never        where e : T

// Because Never <: T' holds for any T', return can appear at any return-type position
```

**No additional rule constraining `return`'s validity is needed**—the explosion principle already
covers it. This makes questions like "can `return` appear in a function returning `X`" disappear.

### Joining Multiple Branches

```
join(A, B):
  If A : Never  → B
  If B : Never  → A
  Otherwise      → requires A and B to be compatible (same type or share a common upper bound)
```

The multiple branches of `if` / `match` are joined via `join`. Branches containing `return` do not
participate in the join because their type is `Never`.

### Consistency with RFC-007

RFC-007's examples **fully conform to this RFC** without revision:

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }          // Never branch, ignored by join
    return n * factorial(n - 1)     // Function exit
}
```

"Early return" is not a special case; it is a **corollary** of rules ①, ②, and the explosion
principle.

### Differences from RFC-010 and Required Revisions

RFC-010's definition has been revised per this RFC; the differences are as follows:

| RFC-010 Original Provision                                                  | Current Definition                                                                               |
| --------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| "`= { ... }` must use `return`, otherwise returns `Void`"                   | Block's value = tail expression; empty block → `Void`                                            |
| Rationale "explicit `return` needed to eliminate tail expression ambiguity" | Does not hold—tail expressions do not produce ambiguity, and `return` does not interfere with it |
| Three `return c` / `return SqliteDb` examples                               | Tail expression form (unified value exits for `spawn` / `unsafe`)                                |

**RFC-010's core design (`{}` as a dependency-driven computation unit) is unchanged**—only the value
exit changed from `return` to tail expression.

### Unified Perspective (Design Principle)

The braces of `if` / `while` are **both imperative control bodies and declarative evaluation
units**—this is a **perspective difference, not two language constructs**. Therefore:

- **Do not bifurcate at the language level** between "control bodies" and "evaluation units"
- All blocks share the same set of evaluation rules (Rule ①)
- `return` is the sole exception mechanism, and its exceptionality comes from `Never`'s
  type-theoretic property, not a syntactic special case

This allows declarative and Pythonic styles to coexist naturally:

```yaoxiang
a = if input > 10 { 19 } else { 20 }
b = { x = f(); x * 2 }
c = spawn { fetch("a") }
```

## Trade-offs

### Advantages

- **Eliminates RFC conflict**: RFC-007 and RFC-010 shift from contradiction to a corollary
  relationship, with no need to sacrifice either side
- **Fewest rules**: three rules + one type-theoretic property (the explosion principle), no special
  cases
- **`return` has a unique meaning**: exit the function. No more judging between "block value /
  function value"
- **Unified perspective**: imperative and declarative coexist; no bifurcation at the language level
- **Consistent with mature languages**: Rust similarly coexists via "tail expression + `return : !`"
- **Zero new syntax**: no new keywords, no parser grammar changes

### Disadvantages

- **Risk of "accidental return" from the last expression**: removing the last line's return-value
  semantics silently changes
  - Mitigation: type checking intercepts type mismatches; the language does not promise to guard
    against wrong intent (see design criterion)
- **Downstream documentation must be revised in sync**: examples in already-accepted documents must
  be rewritten
- **Interaction with statement termination must be clarified**: is a newline-terminated trailing
  expression still a block value? (see open questions)

## Alternatives

### Option A: `return` Keeps Its Two Jobs, Dispatched by Block Type

In function body `= {}`, `return` belongs to the function; in `spawn {}` / `unsafe {}`, it belongs
to the block; in `if {}` / bare blocks, it belongs to ?

**Reason for rejection**: `if`'s assignment cannot be decided—this is exactly the original conflict.
And users must memorize "which blocks belong to whom," with no principled basis (why does `if`
belong to the function while a bare block belongs to itself?).

### Option B: Strict Block Return (`return` Always Belongs to the Current Block)

**Reason for rejection**: **Missing functionality**. `return` could never early-exit a function from
a nested block; guard clauses (`if err { return }`) would be entirely unwritable, forcing functions
to be written as deeply nested expressions.

### Option C: New Keyword for Block Value (e.g., `give x` / `yield x`)

**Reason for rejection**: Violates RFC-036's **zero syntax change** principle (a new keyword is
required), and users must learn two concepts (`return` for exit + `give` for evaluation). The tail
expression approach has zero new concepts.

### Option D: Retain RFC-010 As-Is (Explicit `return` Required)

**Reason for rejection**: Irreconcilable conflict with RFC-007 (see Motivation). And actual
implementation has already taken the tail expression path.

## Revisions to Downstream Documentation

The definitions in this RFC are now the authoritative semantics for downstream documentation;
related documents have all been synchronized (obsolete wording is no longer retained).

## Open Questions

- [x] Interaction between tail expressions and RFC-038 statement termination rules: does a
      newline-terminated trailing expression remain a block value? (@Chenxu: needs confirmation
      alongside RFC-038's "line-leading `(`/`[` never merges" rules, etc.)—Verified:
      newline-terminated trailing expressions (including line-leading `(` / `[` / list literals) all
      behave as block values
- [x] When does `name = { ... }` mean a function vs. a block-value binding—see Appendix D (content
      determines type)
- [x] Specific forms of `unsafe {}` / `spawn {}` after rewriting—both have been verified to work
      with tail expressions
- [x] Interaction of `match` branch `join` with exhaustiveness checking (depends on
      RFC-039)—Verified: `Never` branches do not participate in the join; the join behavior of
      multi-branch `if` / `match` is correct
- [x] Diagnostic wording for empty block `{}` as a function body with a non-`Void` return
      type—reuses the existing `E1012`, with the position pointing to the annotation

---

## Appendix A: Empirical Evidence

All of the following were reproduced on 0.8.0.

| Code                                              | Observed                          |
| ------------------------------------------------- | --------------------------------- |
| `h: (n)->Int = { if n==0 { return 7 } return 8 }` | `h(0)=7`, `h(1)=8`                |
| `f: (n)->Int = { n + 1 }`                         | `5` (tail expression works)       |
| `f: ()->Int = { if c {5} else {6} }`              | `void` (should be `5`, see #344)  |
| `f: ()->Int = if c {5} else {6}`                  | `5`                               |
| `f: ()->Int = { match ... }`                      | Correct                           |
| `f: ()->Int = { while ...; i }`                   | Correct                           |
| `{ y = 5; y }` as a binding expression            | `E3006` (see #343)                |
| `f: ()->Int = { "s" }`                            | Silently passes (see #345)        |
| `x = if c { 19 }` (no `else`)                     | `19` (should be `Void`, see #346) |
| `v = unsafe { 42 }`                               | `void` (see #347)                 |
| `y = if c { 111 } else { 222 }`                   | `111`                             |
| `while { if i==2 { return 42 } }`                 | `42` (pierces the loop)           |
| Nested `{ { return 5 } return 1 }`                | `5` (pierces the bare block)      |

## Appendix B: Design Decision Log

| Decision                                           | Resolution                                                                                 | Rationale                                                            | Date       |
| -------------------------------------------------- | ------------------------------------------------------------------------------------------ | -------------------------------------------------------------------- | ---------- |
| `return` semantics                                 | Function exit, type `Never`; does not return to the block                                  | Eliminate the category error of one word doing two jobs              | 2026-09-15 |
| Block's value exit                                 | Tail expression (the sole exit)                                                            | Fewest rules; consistent with Rust                                   | 2026-09-15 |
| No tail expression                                 | Does not exist (a non-empty block always has a tail expression; empty block `{}` → `Void`) | Write `Void` explicitly when you want `Void`                         | 2026-09-15 |
| Assignment's value                                 | `Void`                                                                                     | Assignment is a statement, not a value producer                      | 2026-09-15 |
| `if` without `else`                                | `Void`                                                                                     | When the condition is false, no branch can be evaluated              | 2026-09-15 |
| Division of labor between mechanism and protection | Language provides mechanism, type checking provides protection                             | The language does not promise to guard against "wrong intent"        | 2026-09-15 |
| Control body / evaluation unit                     | Not bifurcated at the language level; treated as a perspective difference                  | Imperative and declarative coexist; avoid unprincipled special cases | 2026-09-15 |
| Handling of erroneous examples                     | Delete directly, do not leave erroneous code                                               | Retaining seemingly usable erroneous code misleads                   | 2026-09-15 |

## Appendix C: Glossary

| Term                | Definition                                                                                                            |
| ------------------- | --------------------------------------------------------------------------------------------------------------------- |
| Tail expression     | The last expression in a block that produces a value; the block's value equals it                                     |
| Non-local exit      | A control-flow transfer that crosses enclosing evaluation units and directly affects the function boundary (`return`) |
| Explosion principle | `Never <: T` holds for any `T`, allowing `Never` to be reduced to any type                                            |
| Value-bearing block | `= {}` / `spawn {}` / `unsafe {}`—the value exit is the tail expression                                               |
| join                | Multi-branch merging rule; `Never` branches do not participate in the merge                                           |

## Appendix D: Function / Block Value Ambiguity Resolution

### The Problem

`name = { ... }` was defined differently by two already-accepted RFCs: RFC-007 (function syntax)
treated it as a function ("empty-parameter simplest" `name = { return ... }`), while RFC-010 / 010a
treated it as a block value (`= {}` is a value-bearing block whose value is the tail expression).
The same syntactic position, two sets of semantics—implementation-wise, this caused
`callable_parts()` to register the block as a 0-argument function, while `generate_block_ir` took
its value as a block value—two layers of understanding were inconsistent.

### Resolution: Content Determines Type

The same principle should apply to dictionaries and blocks:

| Case                          | Result                  | Basis                                        |
| ----------------------------- | ----------------------- | -------------------------------------------- |
| `value` is `Lambda` (`=>`)    | Function                | `=>` is an explicit function constructor     |
| Annotation is `Fn`            | Function                | Function type declared                       |
| Annotation is a non-`Fn` type | Block value             | The annotation is the type (`x: Int = {..}`) |
| **No annotation**             | **Inferred by content** | **Type determined by content**               |

```yaoxiang
x: Int = { y = 5; y }      // Block value: x = 5 (non-Fn annotation)
f: () -> Int = { 5 }       // Function: f() = 5 (Fn annotation)
f = { 5 }                  // Value: f = 5 (no annotation → inferred by content)
f = {}                     // Value: f = Void (empty block)
b = () => 5                // Function: explicit lambda
d = { "a": 1 }             // Value: Dict (self-describing by content)
```

**Why not "no annotation → default to function"**: That would make `f = { 5 }` a function, while
`d = { "a": 1 }` would be a dictionary—the same `{` in the same annotation-less position yielding
different category things. All three reasons once considered do not hold:

1. Zero changes to existing code—migration cost is bearable (211 files, 704 sites), not a semantic
   basis
2. Function definitions are high-frequency, block values are low-frequency—frequency is not a type
   rule
3. RFC-007 is already accepted; revising it costs more than revising this RFC—wrong things don't
   become right because they are "expensive" to change

**Type is determined by content**, regardless of whether an annotation is present. The annotation
still declares the type (`f: () -> Int`), but its absence does not impose a default.

### Where `{}` Lands

The `Dict` grammar requires at least one key; `{}` has no content to rely on, so it takes the zero
form of a block structure → empty block, value `Void`. Use `dict.new()` for an empty dictionary. See
spec [§2.9.1](../.../reference/language-spec/syntax.md).

### Accompanying Implementation Fixes

1. **Annotations must not determine value-taking**: three locations including `generate_function_ir`
   once used `return_type != Void` as the threshold for tail expression value-taking, causing an
   unannotated `f = { 5 }` to silently discard its tail expression and return `Void`. The annotation
   only decides whether to _check_, not whether to _evaluate_.
2. **`callable_parts()` no longer unconditionally swallows blocks**: the dispatch is unified via
   `Expr::block_binding_is_function(annotation, value)`.

## References

- [RFC-007: Unified Function Definition Syntax](./007-function-syntax-unification.md)
- [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)
- [RFC-038: Statement Termination and Newline Rules](038-statement-termination.md)
- [RFC-030: assert Mechanism](./030-assert-mechanism.md) — refined type applications of `Never`
- [Language Spec §Type System](/reference/language-spec/type-system.md) — the ⊥ / ⊤ positioning of
  `Never` / `Void`
- [Rust Reference: `!` never type](https://doc.rust-lang.org/reference/types/never.html) — an
  isomorphic application of the explosion principle
