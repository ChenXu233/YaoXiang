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
> - [RFC-010: Unified Type Syntax](010-unified-type-syntax.md) — `name: type = value` model, `{}`
>   dependency-driven computation unit
> - [RFC-007: Function Definition Syntax Unification](007-function-syntax-unification.md) — Code
>   block return rules, early return
> - [RFC-038: Statement Termination and Newline Rules](038-statement-termination.md) — Newline
>   behavior of statements and expressions
> - [Language Spec §Type System](../../reference/language-spec/type-system.md) — `Never` explosion
>   principle

## Summary

Unify the semantics of `return` and block evaluation, eliminating the conflict between RFC-007 and
RFC-010.

Propose **three self-consistent rules**: the value of a block equals its tail expression (the sole
value exit); `return` is a **non-local exit** of type `Never` (exits the function, does not "return
to the block"); `if` without `else` takes `Void`.

`return` and block evaluation **are not bifurcated at the language level** — `{ return n }` as a
block has value `n` (type `Never`), while `return` exits the function; the two coexist via the
**explosion principle** (`Never <: T`). RFC-007's "early return" and RFC-010's "block has value" are
corollaries of these three rules, not contradictory special cases.

No new syntax, no new keywords.

## Implementation Status

This RFC has been accepted. The implementation status of the three rules:

| Rule                               | Sub-item                                           | Status                                               |
| ---------------------------------- | -------------------------------------------------- | ---------------------------------------------------- |
| ① Block value = tail expression    | Function body tail expression                      | ✅ Implemented                                       |
| ① Block value = tail expression    | Tail position `if` / `match`                       | ✅ Implemented (#344)                                |
| ① Block value = tail expression    | Last assignment statement → `Void`                 | ✅ Implemented                                       |
| ① Block value = tail expression    | Empty block `{}` → `Void`                          | ✅ Implemented                                       |
| ① Protection by type checking      | Tail expression unifies with declared return type  | ✅ Implemented (#345)                                |
| ② `return` non-local exit          | Exits through `if`/`while`/`for`/bare block/nested | ✅ Implemented                                       |
| ② `Never <: T` explosion principle | `unify` and `is_subtype` are consistent            | ✅ Implemented (this fix resolves internal conflict) |
| ③ `if` without `else` → `Void`     | Expression position does not leak branch value     | ✅ Implemented (#346)                                |
| ① Block value = tail expression    | Bare block value binding `x = { ... }`             | ✅ Implemented (#343)                                |
| ① Block value = tail expression    | `unsafe {}` value exit                             | ✅ Implemented (#347)                                |
| ① Protection by type checking      | Empty block / last statement escape check          | ✅ Implemented (#342 Open Question 5)                |
| ① Block value = tail expression    | `spawn {}` value exit (tail expression)            | ✅ Implemented (#365)                                |

Test coverage: `tests/yaoxiang/03-semantics/rfc010a_block_value.yx` (positive) +
`tests/yaoxiang/06-compile-errors/tail_expr_type_mismatch{,_with_stmts}_err.yx` (negative).

## Motivation

### Conflicting Statements

The Fibonacci example in the playground exposed a semantic divergence:

```yaoxiang
fib: (n: Int) -> Int = {
  if n <= 1 {
    return n          // Is this return "for the if" or "for the function"?
  }
  return fib(n - 1) + fib(n - 2)
}
```

Two accepted RFCs give **opposite** derivations:

| RFC                    | Statement                                                                                                                             | Derived semantics                                                      |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| **RFC-007** (Accepted) | Takes `factorial` as example, titled **"Early Return: Using return"**: `if n <= 1 { return 1 }`                                       | `return` exits through `if` and function body, **returns to function** |
| **RFC-010** (Accepted) | "`{}` is a dependency-driven computation unit… use `return` to explicitly return value"; `spawn { return c }` returns the task result | `return` gives the value to **this `{}`**                              |

Under RFC-010, the braces in `if n <= 1 { return 1 }` are a computation unit, and `return 1` gives
it the value 1, so the value of the `if` statement is discarded and the next line must execute —
**fib recurses infinitely**. Under RFC-007, it's correct.

### Root Cause: `return` has dual roles

- **RFC-007 uses it to express "exit function"** — a control flow concept
- **RFC-010 uses it to express "this block's value is it"** — an evaluation concept

Using a control flow keyword to express evaluation is a category error. One word carrying two roles
— no matter how you tune it, one side will be sacrificed.

### Downstream Documentation's Misinterpretations

`docs/src/reference/language-spec/syntax.md` extended RFC-010's "`{}` block" to **all braces**:

- §2.9: "`return` within `{}` **always returns its content to the enclosing scope**" (and calls it
  "unified semantics: all `{}` blocks")
- §3.3: "`return` is used to **return a value from a code block**"

Yet RFC-010 originally defined **three value-bearing blocks** (`= {}` / `spawn {}` / `unsafe {}`),
and **never mentioned the braces of `if` / `while` / `for` / `match` anywhere**.

### Current Implementation Status

| Behavior                                        | Current Status                                   |
| ----------------------------------------------- | ------------------------------------------------ |
| `if n == 0 { return 7 } … return 8`             | Function exits (`h(0) = 7`)                      |
| `f = { n + 1 }`                                 | Tail expression works (returns `5`)              |
| `x = { y = 5; y }` (bare block tail expression) | `E3006` variable unresolved (see #343)           |
| `f: () -> Int = { if c {5} else {6} }`          | Returns `void`, tail expression discarded (#344) |
| `f: () -> Int = { "s" }`                        | Silently passes compilation (see #345)           |
| `x = if c { 19 }` (no `else`)                   | `19` (should be `Void`, see #346)                |
| `v = unsafe { 42 }`                             | `void` (see #347)                                |
| `y = if c { 111 } else { 222 }`                 | Works (`if` as expression)                       |

`tests/yaoxiang/03-semantics/no_tail_expr_return.yx` once claimed "tail expression is no longer
implicitly returned", but didn't cover that case, so the test didn't fail. That file has been
replaced by `tests/yaoxiang/03-semantics/tail_expr_and_return.yx`.

## Proposal

### Three Rules

```
① Block value = tail expression (sole value exit)
   Assignment statements have value Void; empty block {} has value Void
② return : (T) -> Never
   Non-local exit: exits the nearest function boundary
③ if without else → Void
```

**These three rules are sufficient to derive "early return"; no extra rule that `return`
specifically targets the function is needed.**

### Rule ①: Block Value

The **last statement/expression** of a block is the block's value (the tail expression). This is
**not** "no tail expression then Void" — every non-empty block has a tail expression (the last
statement itself), and the value of an empty block `{}` is `Void`.

```yaoxiang
// The tail expression determines the block's value
a = {
    x = compute()        // Assignment statement → Void
    x * 2                // Tail expression → block value
}

// Assignment as tail expression → block value is Void
b = {
    x = compute()
    log(x)               // Assignment statement → Void
}

// To get Void, write it explicitly
c = {
    log(x)
    Void                 // Explicit Void
}
```

**Design Criteria (separation of mechanism and protection)**:

- **Language rules provide only the mechanism**: the last statement is the block value; the rule is
  unique and unambiguous
- **Protection is provided by type checking**: function declared `-> Int` but tail expression type
  doesn't match → compile error
- **The language doesn't prevent "wrong intent"**: if the tail expression's type happens to match
  the return type but the semantics are unintended, that's an oversight by the author; the language
  cannot judge. **Don't rely on language rules to prevent wrong intent.**
- **If you don't want to return, explicitly write `Void`**

This replaces RFC-010's "`= { ... }` must use `return`, otherwise returns `Void`", and also replaces
its design rationale "explicit `return` is needed to eliminate the ambiguity of 'whether the last
expression is a return value'".

### Rule ②: `return` is a Non-Local Exit

`return` has type `Never` (zero constructors, no inhabitable value). Its semantics:

- **Exits the nearest function boundary**, passing the value to the caller
- **Penetrates all blocks** — (if any) `if` / `while` / `for` / `match` / bare block / `spawn` /
  `unsafe`

`return` does not "return to the block". `{ return n }` as a block has value `n` (tail expression
rule), **type `Never`**; at the same time, `return` exits the function. **Both hold
simultaneously.**

### Rule ③: `if` without `else`

```yaoxiang
x = if c { 19 }        // no else
```

When the condition is false, no branch can be evaluated, so `Void` is taken. Therefore, this `if`
has value type `Void` (or cannot be used in a non-`Void` position).

When a value is needed, explicitly provide both branches:

```yaoxiang
x = if c { 19 } else { 20 }
```

### `Never` is the technical basis for coexistence

`Never <: T` holds for any type `T` (explosion principle, see Language Spec §Type System).
Therefore:

```yaoxiang
fib: (n: Int) -> Int = {
  if n <= 1 {
    return n              // tail expression : Never
  }                       // → value of this if : Never
  fib(n - 1) + fib(n - 2) // → block value : Int
}
```

- The branch body `{ return n }` has value `Never`
- This `if`'s only branch is `Never` ⇒ the `if`'s value is `Never`
- A statement of type `Never` means **the sequence terminates here** — the following statement is
  not "the next to execute sequentially"
- And `Never` can be reduced to any type (explosion principle), so the entire block satisfies
  `-> Int`

**"Early return" follows naturally**: `Never` terminates the sequence + explosion principle allows
reduction.

## Detailed Design

### Formalization of Blocks and Tail Expressions

```
Block        ::= '{' Stmt* '}'                     // Empty block → Void
               | '{' Stmt* Expr '}'                // Value = Expr
Expr         ::= ...
               | Return                            // Type Never
Stmt         ::= Assignment | ExprStmt | ...

value(Block):
  empty block             → Void
  { ...; e }              → type(e)
  { ...; s } (s is stmt)  → Void          // Assignment's value is Void
```

### Type Rules for `return`

```
return e : Never        where e : T

// Because Never <: T' holds for any T', return can appear at any return-type position
```

**No extra rule is needed to constrain `return`'s legality** — the explosion principle already
covers it. This makes questions like "can `return` appear in a function returning `X`" disappear.

### Multi-Branch Join

```
join(A, B):
  if A : Never  → B
  if B : Never  → A
  else          → require A, B compatible (same type or reachable common upper bound)
```

Multi-branch `if` / `match` are joined per `join`. Branches with `return` don't participate in join
because their type is `Never`.

### Consistency with RFC-007

The examples in RFC-007 **fully conform to this RFC** and need no revision:

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }          // Never branch, ignored by join
    return n * factorial(n - 1)     // function exit
}
```

"Early return" is not a special case; it is a **corollary** of rules ① ② and the explosion
principle.

### Differences from RFC-010 and Revision Requirements

RFC-010's definitions have been revised according to this RFC. The differences are as follows:

| RFC-010 Original Clause                                                            | Current Definition                                                                            |
| ---------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| "`= { ... }` must use `return`, otherwise returns `Void`"                          | Block value = tail expression; empty block → `Void`                                           |
| Design rationale "explicit `return` needed to eliminate tail expression ambiguity" | Doesn't hold — tail expressions don't produce ambiguity, `return` doesn't interfere with them |
| Three `return c` / `return SqliteDb` examples                                      | Tail expression form (value exits of `spawn` / `unsafe` unified)                              |

**The core design of RFC-010 (`{}` as a dependency-driven computation unit) is unchanged**; only the
value exit is changed from `return` to tail expression.

### Unification of Perspective (Design Principle)

The braces of `if` / `while` are **both imperative control bodies and declarative evaluation units**
— this is a **difference in perspective, not two language constructs**. Therefore:

- **Not bifurcated at the language level** between "control flow body" and "evaluation unit"
- All blocks share the same evaluation rules (rule ①)
- `return` is the sole exception mechanism, and its exceptional nature comes from the type-theoretic
  properties of `Never`, not from a syntactic special case

This makes declarative and Python-style writing naturally coexist:

```yaoxiang
a = if input > 10 { 19 } else { 20 }
b = { x = f(); x * 2 }
c = spawn { fetch("a") }
```

## Tradeoffs

### Advantages

- **Eliminates RFC conflict**: RFC-007 and RFC-010 change from contradiction to a corollary
  relationship, with neither side needing to be sacrificed
- **Minimum number of rules**: three rules + one type-theoretic property (explosion principle), no
  special cases
- **`return` has a single meaning**: exit the function. No more need to judge between "block value /
  function value"
- **Unified perspective**: imperative and declarative coexist, no language-level bifurcation
- **Consistent with mature languages**: Rust also uses "tail expression + `return : !`" to make both
  mechanisms coexist
- **Zero new syntax**: no new keywords, no parser grammar changes

### Disadvantages

- **Tail expression as value has "accidental return" risk**: forgetting to delete the last line
  silently changes the return value
  - Mitigation: type checking intercepts type mismatches; the language doesn't promise to prevent
    wrong intent (see Design Criteria)
- **Downstream documentation needs to be revised**: examples in accepted documents need rewriting
- **Interaction with statement termination needs clarification**: is a newline-terminated last
  expression still a block value? (see Open Questions)

## Alternatives

### Option A: `return` keeps dual roles, dispatched by block type

In function body `= {}`, `return` belongs to the function; in `spawn {}` / `unsafe {}`, it belongs
to the block; in `if {}` / bare block, it belongs to whom?

**Reason for rejection**: The attribution of `if` cannot be adjudicated — this is the original
conflict. Also, users need to memorize "which blocks belong to whom", with no principled basis (why
would `if` belong to the function but a bare block to itself?).

### Option B: Strict block return (`return` always belongs to the current block)

**Reason for rejection**: **Functionality missing**. `return` can never early-exit a function from a
nested block; guard clauses (`if err { return }`) become completely unwritable; functions can only
be written as nested expressions.

### Option C: Block value uses new keywords (`give x` / `yield x`)

**Reason for rejection**: Violates RFC-036's **zero syntax change** principle (requires adding new
keywords), and users have to learn two concepts (`return` for exit + `give` for evaluation). The
tail expression approach requires learning zero new concepts.

### Option D: Keep RFC-010 as is (explicit `return` required)

**Reason for rejection**: Conflict with RFC-007 is irreconcilable (see Motivation). And the actual
implementation has already gone down the tail expression path.

## Revisions to Downstream Documentation

This RFC's definitions have become the authoritative semantics for downstream documentation;
relevant documents have all been synchronized (obsolete wording no longer preserved).

## Open Questions

- [x] Interaction between tail expressions and RFC-038's statement termination rules: is a
      newline-terminated last expression still a block value? (@Chenxu: needs to be confirmed
      together with RFC-038's "leading `(`/`[` never merges" rules) — Verified: newline-terminated
      last expressions (including those starting with `(` / `[` / list literals) are all block
      values
- [x] When is `name = { ... }` a function vs a block value binding — see Appendix D (content
      determines type)
- [x] Specific forms after `unsafe {}` / `spawn {}` rewriting — both have been verified to work with
      tail expressions
- [x] Interaction between `match` branch `join` and exhaustiveness checking (depends on RFC-010b) —
      Verified: `Never` branches don't participate in join, multi-branch `if` / `match` join
      behavior is correct
- [x] Diagnostic message for empty block `{}` as function body with non-`Void` return type — reuse
      existing `E1012`, position points to annotation

---

## Appendix A: Empirical Evidence

All reproduced on 0.8.0.

| Code                                              | Measured                          |
| ------------------------------------------------- | --------------------------------- |
| `h: (n)->Int = { if n==0 { return 7 } return 8 }` | `h(0)=7`, `h(1)=8`                |
| `f: (n)->Int = { n + 1 }`                         | `5` (tail expression available)   |
| `f: ()->Int = { if c {5} else {6} }`              | `void` (should be `5`, see #344)  |
| `f: ()->Int = if c {5} else {6}`                  | `5`                               |
| `f: ()->Int = { match ... }`                      | correct                           |
| `f: ()->Int = { while ...; i }`                   | correct                           |
| `{ y = 5; y }` as binding expression              | `E3006` (see #343)                |
| `f: ()->Int = { "s" }`                            | Silently passes (see #345)        |
| `x = if c { 19 }` (no `else`)                     | `19` (should be `Void`, see #346) |
| `v = unsafe { 42 }`                               | `void` (see #347)                 |
| `y = if c { 111 } else { 222 }`                   | `111`                             |
| `while { if i==2 { return 42 } }`                 | `42` (exits through loop)         |
| Nested `{ { return 5 } return 1 }`                | `5` (exits through bare block)    |

## Appendix B: Design Decision Record

| Decision                             | Decision Made                                                                             | Reason                                                                  | Date       |
| ------------------------------------ | ----------------------------------------------------------------------------------------- | ----------------------------------------------------------------------- | ---------- |
| `return` semantics                   | Function exit, type `Never`; not returned to block                                        | Eliminate the category error of one word with two roles                 | 2026-09-15 |
| Block value exit                     | Tail expression (sole exit)                                                               | Minimum rules; consistent with Rust                                     | 2026-09-15 |
| No tail expression                   | Doesn't exist (non-empty blocks always have a tail expression; empty block `{}` → `Void`) | To get `Void`, explicitly write `Void`                                  | 2026-09-15 |
| Assignment value                     | `Void`                                                                                    | Assignment is a statement, not value production                         | 2026-09-15 |
| `if` without `else`                  | `Void`                                                                                    | When condition is false, no branch can be evaluated                     | 2026-09-15 |
| Division of mechanism and protection | Language provides mechanism, type checking provides protection                            | The language doesn't promise to prevent "wrong intent"                  | 2026-09-15 |
| Control flow body / evaluation unit  | Not bifurcated at the language level, treated as difference in perspective                | Imperative and declarative coexist, avoiding unprincipled special cases | 2026-09-15 |
| Handling of error examples           | Delete directly, no error code kept                                                       | Keeping seemingly usable error code would mislead                       | 2026-09-15 |

## Appendix C: Glossary

| Term                | Definition                                                                                                        |
| ------------------- | ----------------------------------------------------------------------------------------------------------------- |
| Tail expression     | The last value-producing expression in a block; the block's value is it                                           |
| Non-local exit      | Control flow transfer that traverses outer evaluation units and acts directly on the function boundary (`return`) |
| Explosion principle | `Never <: T` holds for any `T`, allowing `Never` to be reduced to any type                                        |
| Value-bearing block | `= {}` / `spawn {}` / `unsafe {}` — value exit is the tail expression                                             |
| join                | Multi-branch join rule; `Never` branches don't participate in join                                                |

## Appendix D: Function / Block Value Ambiguity Adjudication

### Problem

`name = { ... }` was defined differently by two accepted RFCs: RFC-007 (function syntax) treats it
as a function ("simplest no-arg form" `name = { return ... }`), while RFC-010 / 010a treats it as a
block value (`= {}` is a value-bearing block, value is the tail expression). Same syntactic
position, two semantics. In implementation, this led to `callable_parts()` registering the block as
a 0-arg function, while `generate_block_ir` took the block's value — two layers of inconsistent
understanding.

### Adjudication: Content Determines Type

The same principle should apply across dictionaries and blocks:

| Case                          | Result                    | Basis                                    |
| ----------------------------- | ------------------------- | ---------------------------------------- |
| `value` is `Lambda` (`=>`)    | Function                  | `=>` is an explicit function constructor |
| Annotation is `Fn`            | Function                  | Declared a function type                 |
| Annotation is a non-`Fn` type | Block value               | Annotation is the type (`x: Int = {..}`) |
| **No annotation**             | **Inferred from content** | **Type determined by content**           |

```yaoxiang
x: Int = { y = 5; y }      // Block value: x = 5 (annotation not Fn)
f: () -> Int = { 5 }       // Function: f() = 5 (annotation is Fn)
f = { 5 }                  // Value: f = 5 (no annotation → inferred from content)
f = {}                     // Value: f = Void (empty block)
b = () => 5                // Function: explicit lambda
d = { "a": 1 }             // Value: Dict (self-describing content)
```

**Why not use "no annotation defaults to function"**: That would make `f = { 5 }` a function, but
`d = { "a": 1 }` a dictionary — the same `{` in the same annotation-less position giving different
categories of things. All three previously considered reasons don't hold:

1. Zero changes to existing code — migration cost is payable (211 files, 704 sites), not a semantic
   basis
2. Function definitions are high frequency, block values are low frequency — frequency is not a type
   rule
3. RFC-007 is accepted, changing it is more expensive than changing this RFC — wrong things don't
   become right because they're "expensive"

**Type is determined by content**, regardless of whether an annotation is present. Annotations still
declare the type (`f: () -> Int`), but don't impose a default value due to "absence of annotation".

### Placement of `{}`

`Dict` grammar requires at least one key; `{}` has no content to base on, so it takes the zero form
of block structure → empty block, value `Void`. Empty dictionary uses `dict.new()`. See spec
[§2.9.1](../../reference/language-spec/syntax.md).

### Accompanying Implementation Fixes

1. **Annotation must not determine value-taking**: `generate_function_ir` and two other places once
   used `return_type != Void` as the threshold for taking the tail expression as the value, causing
   the tail expression of unannotated `f = { 5 }` to be silently discarded, returning `Void`.
   Annotation only determines whether to _check_, not whether to _evaluate_.
2. **`callable_parts()` no longer unconditionally swallows blocks**: Dispatch is unified under
   `Expr::block_binding_is_function(annotation, value)`.

## References

- [RFC-007: Function Definition Syntax Unification](007-function-syntax-unification.md)
- [RFC-010: Unified Type Syntax](010-unified-type-syntax.md)
- [RFC-038: Statement Termination and Newline Rules](038-statement-termination.md)
- [RFC-030: assert Mechanism](030-assert-mechanism.md) — Refined type application of `Never`
- [Language Spec §Type System](../../reference/language-spec/type-system.md) — ⊥ / ⊤ positioning of
  `Never` / `Void`
- [Rust Reference: `!` never type](https://doc.rust-lang.org/reference/types/never.html) —
  Isomorphic application of the explosion principle
