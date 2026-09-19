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
>   `{}` as dependency-driven computation units
> - [RFC-007: Unified Function Definition Syntax](./007-function-syntax-unification.md) — block
>   return rules, early return
> - [RFC-038: Statement Termination and Newline Rules](038-statement-termination.md) — newline
>   behavior of statements and expressions
> - [Language Spec §Type System](/reference/language-spec/type-system.md) — `Never` explosion
>   principle

## Summary

Unifies the semantics of `return` and block evaluation, eliminating the stated conflicts between
RFC-007 and RFC-010.

Proposes **three self-consistent rules**: a block's value equals its tail expression (the sole
outlet); `return` is a **non-local exit** of type `Never` (exits the function, does not "return to
the block"); `if` without `else` yields `Void`.

`return` and block evaluation **are not bifurcated at the language level**—`{ return n }` as a block
has value `n` (type `Never`), while `return`'s effect is to exit the function; the two coexist via
the **explosion principle** (`Never <: T`). RFC-007's "early return" and RFC-010's "blocks have
values" are corollaries of these three rules, not conflicting special cases.

No new syntax, no new keywords.

## Implementation Status

This RFC has been accepted. Implementation status of the three rules:

| Rule                                | Sub-item                                                 | Status                                   |
| ----------------------------------- | -------------------------------------------------------- | ---------------------------------------- |
| ① Block value = tail expression     | Function body tail expression                            | ✅ Implemented                           |
| ① Block value = tail expression     | Tail-position `if` / `match`                             | ✅ Implemented (#344)                    |
| ① Block value = tail expression     | Last assignment statement → `Void`                       | ✅ Implemented                           |
| ① Block value = tail expression     | Empty block `{}` → `Void`                                | ✅ Implemented                           |
| ① Protection provided by type check | Tail expression unified with declared return type        | ✅ Implemented (#345)                    |
| ② `return` non-local exit           | Exits through `if`/`while`/`for`/bare block/nested block | ✅ Implemented                           |
| ② `Never <: T` explosion principle  | `unify` and `is_subtype` consistent                      | ✅ Implemented (fixes internal conflict) |
| ③ `if` without `else` → `Void`      | Branch value does not leak in expression position        | ✅ Implemented (#346)                    |
| ① Block value = tail expression     | Bare block value binding `x = { ... }`                   | ✅ Implemented (#343)                    |
| ① Block value = tail expression     | `unsafe {}` value outlet                                 | ✅ Implemented (#347)                    |
| ① Protection provided by type check | Empty block / last-statement escape checks               | ✅ Implemented (#342 open question 5)    |
| ① Block value = tail expression     | `spawn {}` value outlet (tail expression)                | ❌ Not implemented (see #365)            |

Corpus coverage: `tests/yaoxiang/03-semantics/rfc010a_block_value.yx` (positive) +
`tests/yaoxiang/06-compile-errors/tail_expr_type_mismatch{,_with_stmts}_err.yx` (negative).

## Motivation

### Stated Conflict

The Fibonacci example in the playground revealed a semantic divergence:

```yaoxiang
fib: (n: Int) -> Int = {
  if n <= 1 {
    return n          // Is this return "the if's" or "the function's"?
  }
  return fib(n - 1) + fib(n - 2)
}
```

Two accepted RFCs gave **opposite** derivations:

| RFC                    | Statement                                                                                                                              | Derived semantics                                                       |
| ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------- |
| **RFC-007** (Accepted) | Using `factorial` as the example, titled **"Early return: using return"**: `if n <= 1 { return 1 }`                                    | `return` exits through `if` and function body, **back to the function** |
| **RFC-010** (Accepted) | "`{}` is a dependency-driven computation unit…use `return` to explicitly return a value"; `spawn { return c }` returns the task result | `return` gives **this `{}`** its value                                  |

Under RFC-010, the braces in `if n <= 1 { return 1 }` are a computation unit; `return 1` gives them
the value 1, so the `if` statement's value is discarded and the next line necessarily executes—**fib
recurses infinitely**. Under RFC-007 it works correctly.

### Root Cause: `return` Doing Double Duty

- **RFC-007 uses it to mean "exit the function"**—a control-flow concept
- **RFC-010 uses it to mean "this block's value is it"**—an evaluation concept

Using a control-flow keyword to express evaluation is a category error. One word wearing two
hats—whichever you tune, one side gets sacrificed.

### Downstream Documentation's Erroneous Expanded Interpretation

`docs/src/reference/language-spec/syntax.md` expanded RFC-010's "`{}` block" to **all braces**:

- §2.9: "`return` inside `{}` **always returns the content to the enclosing scope**" (and claims
  "unified semantics: all `{}` blocks")
- §3.3: "`return` is used to **return a value from a code block**"

While RFC-010's original text defined only **three value-bearing blocks** (`= {}` / `spawn {}` /
`unsafe {}`), and **never mentioned the braces of `if` / `while` / `for` / `match`**.

### Current Implementation

| Behavior                                        | Current state                                    |
| ----------------------------------------------- | ------------------------------------------------ |
| `if n == 0 { return 7 } … return 8`             | Function exits (`h(0) = 7`)                      |
| `f = { n + 1 }`                                 | Tail expression available (returns `5`)          |
| `x = { y = 5; y }` (bare block tail expression) | `E3006` variable unresolved (see #343)           |
| `f: () -> Int = { if c {5} else {6} }`          | Returns `void`, tail expression discarded (#344) |
| `f: () -> Int = { "s" }`                        | Silently passes compilation (see #345)           |
| `x = if c { 19 }` (no `else`)                   | `19` (should be `Void`, see #346)                |
| `v = unsafe { 42 }`                             | `void` (see #347)                                |
| `y = if c { 111 } else { 222 }`                 | Available (`if` as expression)                   |

`tests/yaoxiang/03-semantics/no_tail_expr_return.yx` once claimed "the tail expression is no longer
implicitly returned", but did not cover this case, so the test did not fail. That file has been
replaced by `tests/yaoxiang/03-semantics/tail_expr_and_return.yx`.

## Proposal

### Three Rules

```
① Block value = tail expression (the sole outlet)
   Assignment statements have value Void; empty block {} has value Void
② return : (T) -> Never
   Non-local exit: exits the nearest function boundary
③ if without else → Void
```

**These three suffice to derive "early return"—no extra rule making `return` function-specific is
needed.**

### Rule ①: Block Value

A block's **last statement/expression** is the block's value (tail expression). This is **not** "if
no tail expression then Void"—a non-empty block always has a tail expression (the last statement
itself is one), and an empty block `{}` has value `Void`.

```yaoxiang
// The tail expression determines the block's value
a = {
    x = compute()        // Assignment statement → Void
    x * 2                // Tail expression → block's value
}

// Assignment as tail expression → block value is Void
b = {
    x = compute()
    log(x)               // Assignment statement → Void
}

// Want Void? Write it explicitly
c = {
    log(x)
    Void                 // Explicit Void
}
```

**Design principle (separation of mechanism and protection)**:

- **Language rules provide only the mechanism**: the last statement is the block value—single rule,
  no ambiguity
- **Protection provided by type checking**: function declares `-> Int` but tail expression's type
  mismatches → compilation error
- **Language does not guard against "wrong intent"**: if the tail expression's type happens to match
  the return type but the semantics is not what's wanted, that's the author's oversight; the
  language has no way to tell. **Do not rely on language rules to prevent wrong intent.**
- **Don't want to return? Write `Void` explicitly.**

This replaces RFC-010's "`= { ... }` must use `return`, otherwise returns `Void`", and replaces its
design rationale "explicit `return` is needed to eliminate the ambiguity of whether the last
expression is the return value".

### Rule ②: `return` Is a Non-Local Exit

`return`'s type is `Never` (zero constructors, no value can inhabit it). Its semantics:

- **Exits the nearest function boundary**, handing the value to the caller
- **Passes through any block**—(if any) `if` / `while` / `for` / `match` / bare block / `spawn` /
  `unsafe`

`return` does not "return to the block". `{ return n }` as a block has value `n` (tail expression
rule), **type `Never`**; at the same time, `return`'s effect is to exit the function. **Both hold
simultaneously.**

### Rule ③: `if` Without `else`

```yaoxiang
x = if c { 19 }        // No else
```

When the condition is false, no branch is available to evaluate, yielding `Void`. Hence this `if`'s
value type is `Void` (or it cannot be used in non-`Void` positions).

When a value is needed, supply both branches explicitly:

```yaoxiang
x = if c { 19 } else { 20 }
```

### `Never` Is the Technical Basis of Coexistence

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
- This `if`'s only branch is `Never` ⇒ `if`'s value is `Never`
- A statement of type `Never` means **the sequence terminates here**—the next line is not "the next
  statement to execute in order"
- And `Never` can be coerced to any type (explosion principle), so the whole block satisfies
  `-> Int`

**"Early return" follows naturally**: `Never` terminates the sequence + explosion principle enables
coercion.

## Detailed Design

### Formalization of Block and Tail Expression

```
Block        ::= '{' Stmt* '}'                     // Empty block → Void
               | '{' Stmt* Expr '}'                // Value = Expr
Expr         ::= ...
               | Return                            // Type Never
Stmt         ::= Assignment | ExprStmt | ...

value(Block)：
  Empty block               → Void
  { ...; e }                → type(e)
  { ...; s } (s is stmt)    → Void          // Assignment's value is Void
```

### Type Rule for `return`

```
return e : Never        where e : T

// Because Never <: T' holds for any T', return can appear in any return-type position
```

**No extra rule constraining the validity of `return` is needed**—the explosion principle already
covers it. This makes the question "can `return` appear in a function returning `X`?" disappear.

### Multi-Branch Joining

```
join(A, B)：
  If A : Never  → B
  If B : Never  → A
  Otherwise     → require A, B compatible (same type or reachable common upper bound)
```

Multiple branches of `if` / `match` are joined via `join`. A branch containing `return` does not
participate in the join because its type is `Never`.

### Consistency with RFC-007

RFC-007's examples **fully conform to this RFC** and need no revision:

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }          // Never branch, ignored by join
    return n * factorial(n - 1)     // Function exit
}
```

"Early return" is not a special case—it's a **corollary** of rules ①, ②, and the explosion
principle.

### Differences from RFC-010 and Required Revisions

RFC-010 needs three revisions (as errata; see "Required Revisions to Downstream Documentation"):

| RFC-010 clause                                                                        | After revision                                                                        |
| ------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| "`= { ... }` must use `return`, otherwise returns `Void`"                             | Block value = tail expression; empty block → `Void`                                   |
| Design rationale "explicit `return` is needed to eliminate tail-expression ambiguity" | Deprecated—tail expressions cause no ambiguity; `return` does not interfere with them |
| Three `return c` / `return SqliteDb` examples                                         | Changed to tail expressions (unified value outlet for `spawn` / `unsafe`)             |

**RFC-010's core design (`{}` as dependency-driven computation unit) is unchanged**—only the value
outlet changes from `return` to the tail expression.

### Unifying Perspectives (Design Principle)

The braces of `if` / `while` **are both imperative control bodies and declarative evaluation
units**—a **difference in perspective, not two language constructs**. Therefore:

- **Do not bifurcate at the language level** between "control-flow body" and "evaluation unit"
- All blocks share the same evaluation rules (Rule ①)
- `return` is the only exception mechanism, and its exceptionality comes from the type-theoretic
  property of `Never`, not a syntactic special case

This lets declarative and Pythonic styles coexist naturally:

```yaoxiang
a = if input > 10 { 19 } else { 20 }
b = { x = f(); x * 2 }
c = spawn { fetch("a") }   // ⚠️ Tail-expression outlet not yet implemented, see #365; currently must write `spawn { return fetch("a") }`
```

## Trade-offs

### Advantages

- **Eliminates RFC conflict**: RFC-007 and RFC-010 go from contradictory to corollary—no need to
  sacrifice either side
- **Fewest rules**: three rules + one type-theoretic property (explosion principle), no special
  cases
- **`return` has a single meaning**: exit the function. No more judging between "block value /
  function value"
- **Unified perspective**: imperative and declarative coexist; not bifurcated at the language level
- **Consistent with mature languages**: Rust likewise uses "tail expression + `return : !`" to
  coexist both mechanisms
- **Zero new syntax**: no new keywords, no parser grammar changes

### Disadvantages

- **"Accidental return" risk from last-expression-is-value**: forgetting to delete the last line
  silently changes the return value
  - Mitigation: type checking intercepts type mismatches; language does not promise to prevent wrong
    intent (see design principle)
- **RFC-010 needs errata**: examples in already-accepted documents must be rewritten
- **Bare-block tail expression pending**: currently a compiler internal error (independent of this
  design)
- **Interaction with statement termination must be clarified**: is a newline-terminated last
  expression still the block value? (see open questions)

## Alternatives

### Option A: Keep `return`'s Double Duty, Dispatch by Containing Block Type

In function body `= {}`, `return` belongs to the function; in `spawn {}` / `unsafe {}`, it belongs
to the block; in `if {}` / bare block, it belongs to…?

**Reason for rejection**: `if`'s ownership cannot be decided—this is the original conflict. And it
requires the user to memorize "which block belongs to whom" with no principled basis (why would `if`
belong to the function but bare block belong to itself?).

### Option B: Strict Block Return (`return` Always Belongs to the Current Block)

**Reason for rejection**: **loss of functionality**. `return` can never early-exit a nested block
from a function; guard clauses (`if err { return }`) become completely unwritable, and functions
must be written as deeply nested expressions.

### Option C: New Keyword for Block Value (`give x` / `yield x`)

**Reason for rejection**: violates RFC-036's **zero-syntax-change** principle (requires a new
keyword), and users must learn two concepts (`return` for exit + `give` for value). The
tail-expression approach has zero new concepts.

### Option D: Keep RFC-010 As-Is (Explicit `return` Required)

**Reason for rejection**: irreconcilably conflicts with RFC-007 (see Motivation). And the actual
implementation has already gone down the tail-expression path.

## Required Revisions to Downstream Documentation

After this RFC is accepted, the following documents must be revised. **Erroneous examples are to be
deleted outright; do not keep seemingly-usable error code in the errata block.**

| Document                                                    | Revision content                                                                                          | Status |
| ----------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- | ------ |
| RFC-010 §Return rules                                       | Change to tail-expression rules; **delete** original "must use `return` otherwise return `Void`" examples | ✅     |
| RFC-010 design rationale section                            | **Delete** the "explicit `return` is needed to eliminate ambiguity" argument                              | ✅     |
| RFC-010 §`spawn` block / §`unsafe` block examples           | `return c` / `return SqliteDb` → tail-expression form                                                     | ✅     |
| RFC-010 §dependency-driven computation unit examples        | `return b` → tail expression `b`                                                                          | ✅     |
| `language-spec/syntax.md` §2.9                              | "`return` returns to enclosing scope" → tail-expression rules + `return` exits function                   | ✅     |
| `language-spec/syntax.md` §3.3                              | "return a value from a code block" → "exit the nearest function boundary, type `Never`"                   | ✅     |
| `language-spec/type-system.md`                              | "Function with no `return` defaults to `Void`" → tail expression decides                                  | ✅     |
| `tests/yaoxiang/03-semantics/no_tail_expr_return.yx`        | Assertions and comments contradict new rules; must be fixed                                               | ✅     |
| RFC-007 §Summary / §Lambda syntax rules / syntax rule table | "must use `return` otherwise return `Void`" → tail expression + `Never` errata block                      | ✅     |

RFC-007's "code blocks must use `return`, otherwise return `Void`" sentence conflicts with this RFC;
it has been revised via errata (RFC-007's functional form definition is unchanged)—see the last row
above.

## Open Questions

- [x] Interaction between tail expression and RFC-038 statement termination rules: is a
      newline-terminated last expression still the block value? (@Chenxu: needs to be confirmed
      together with RFC-038's "line-leading `(`/`[` never merge" etc. rules)—Tested: newline-
      terminated last expressions (including line-leading `(` / `[` / list literals) all remain
      block values
- [x] When does `name = { ... }` mean a function versus a block-value binding—see Appendix D (Ruling
      C: annotation first, default function)
- [ ] Specific form of `unsafe {}` after rewriting—`unsafe {}` tail expression tested working;
      `spawn {}` tail expression still not working (value lost or `E3006`), see #365
- [x] Interaction between `match` branch `join` and exhaustiveness check (depends on RFC-039)—
      Tested: `Never` branches do not participate in the join; multi-branch `if` / `match` join
      behavior is correct
- [x] Diagnostic wording for empty block `{}` as function body when return type is non-`Void`— reuse
      existing `E1012`, position points to annotation

---

## Appendix A: Empirical Evidence

All reproduced on 0.8.0.

| Code                                              | Observed                          |
| ------------------------------------------------- | --------------------------------- |
| `h: (n)->Int = { if n==0 { return 7 } return 8 }` | `h(0)=7`, `h(1)=8`                |
| `f: (n)->Int = { n + 1 }`                         | `5` (tail expression available)   |
| `f: ()->Int = { if c {5} else {6} }`              | `void` (should be `5`, see #344)  |
| `f: ()->Int = if c {5} else {6}`                  | `5`                               |
| `f: ()->Int = { match ... }`                      | Correct                           |
| `f: ()->Int = { while ...; i }`                   | Correct                           |
| `{ y = 5; y }` as binding expression              | `E3006` (see #343)                |
| `f: ()->Int = { "s" }`                            | Silently passes (see #345)        |
| `x = if c { 19 }` (no `else`)                     | `19` (should be `Void`, see #346) |
| `v = unsafe { 42 }`                               | `void` (see #347)                 |
| `y = if c { 111 } else { 222 }`                   | `111`                             |
| `while { if i==2 { return 42 } }`                 | `42` (exits loop)                 |
| Nested `{ { return 5 } return 1 }`                | `5` (exits bare block)            |

## Appendix B: Design Decision Record

| Decision                             | Decision                                                                   | Reason                                                               | Date       |
| ------------------------------------ | -------------------------------------------------------------------------- | -------------------------------------------------------------------- | ---------- |
| `return` semantics                   | Function exit, type `Never`; does not return to block                      | Eliminate the category error of one word with two jobs               | 2026-09-15 |
| Block value outlet                   | Tail expression (sole outlet)                                              | Fewest rules; consistent with Rust                                   | 2026-09-15 |
| No tail expression                   | Doesn't exist (non-empty block always has tail; empty block `{}` → `Void`) | Want `Void`? Write `Void` explicitly                                 | 2026-09-15 |
| Value of assignment                  | `Void`                                                                     | Assignment is a statement, not value production                      | 2026-09-15 |
| `if` without `else`                  | `Void`                                                                     | When condition is false, no branch to evaluate                       | 2026-09-15 |
| Division of mechanism and protection | Language provides mechanism, type check provides protection                | Language does not promise to prevent "wrong intent"                  | 2026-09-15 |
| Control body / evaluation unit       | Not bifurcated at language level, treated as perspective difference        | Imperative and declarative coexist; avoid unprincipled special cases | 2026-09-15 |
| Handling of erroneous examples       | Delete; do not retain in errata block                                      | Retaining seemingly-usable error code misleads                       | 2026-09-15 |

## Appendix C: Glossary

| Term                | Definition                                                                                                        |
| ------------------- | ----------------------------------------------------------------------------------------------------------------- |
| Tail expression     | The last expression in a block that produces a value; the block's value equals it                                 |
| Non-local exit      | A control-flow transfer that crosses outer evaluation units and directly acts on the function boundary (`return`) |
| Explosion principle | `Never <: T` holds for any `T`, so `Never` can be coerced to any type                                             |
| Value-bearing block | `= {}` / `spawn {}` / `unsafe {}`—value outlet is the tail expression                                             |
| join                | Multi-branch merge rule; `Never` branches do not participate in the merge                                         |

## Appendix D: Function / Block-Value Ambiguity Ruling (Ruling C)

> **Erratum (2026-09-19): The annotation-first / default-function lines of Ruling C have been
> overturned.**
>
> User ruling: **unify to "content determines type" (reading B)**, reasoning that the same principle
> should run through dicts and blocks:
>
> > Unify on B; `{}` falls into place naturally.
>
> **Current rules**
>
> | Case                        | Result                    | Basis                                     |
> | --------------------------- | ------------------------- | ----------------------------------------- |
> | `value` is `Lambda` (`=>`)  | Function                  | `=>` is the explicit function constructor |
> | Annotation is `Fn`          | Function                  | Declared function type                    |
> | Annotation is non-`Fn` type | Block value               | Annotation is the type (`x: Int = {..}`)  |
> | **No annotation**           | **Inferred from content** | **Type determined by content**            |
>
> Key change: `f = { 5 }` is now a **value** `5` (`Int`), no longer a function.
>
> ```yaoxiang
> x: Int = { y = 5; y }      // Block value: x = 5 (annotation non-Fn)
> f: () -> Int = { 5 }       // Function: f() = 5 (annotation is Fn)
> f = { 5 }                  // Value: f = 5 (no annotation → content inference)
> f = {}                     // Value: f = Void (empty block)
> b = () => 5                // Function: explicit lambda
> d = { "a": 1 }             // Value: Dict (self-describing content)
> ```
>
> **Why Ruling C Was Overturned**
>
> Ruling C's original reasoning ("default function when no annotation") was
> **implementation-convenience-driven**:
>
> 1. ~~Zero changes to existing code~~—migration cost is payable (211 files, 704 sites), not a
>    semantic argument
> 2. ~~Function definition is high-frequency, block value is low-frequency~~—frequency is not a type
>    rule
> 3. ~~RFC-007 is accepted, changing it is more expensive than changing this RFC~~—fixing something
>    wrong doesn't make it right because it's "expensive"
>
> The real problem is **inconsistency**: `f = { 5 }` is a function, but `d = { "a": 1 }` is a
> dict—the same `{` in the same annotation-less position yields different-category things. The
> "cost" Ruling C itself admitted ("in `f = { 5 }`, `f` is a function rather than `5`… counter to
> the intuition that braces are values") is exactly this crack.
>
> The current rule eliminates the crack: **type is determined by content**, regardless of whether an
> annotation is present. Annotations still declare types (`f: () -> Int`), but don't impose a
> default just because the annotation is absent.
>
> **`{}` Falling into Place**
>
> After unifying to B, `{}` falls into place naturally: `Dict` grammar requires at least one key,
> and `{}` has no content to base on, so it takes the zero form of block structure → empty block,
> value `Void`. Use `dict.new()` for an empty dict. See spec
> [§2.9.1](../.../reference/language-spec/syntax.md).
>
> **Unchanged Parts**
>
> The two subsequent items in Appendix D ("annotation must not decide value", "`callable_parts()` no
> longer unconditionally swallows blocks") still hold; the latter is now implemented by
> `Expr::block_binding_is_function`, only its annotation-less branch has changed per the table
> above.

### The Problem (Historical Context)

`name = { ... }` was defined by two already-accepted RFCs as different things:

| RFC                       | Definition                                                                       |
| ------------------------- | -------------------------------------------------------------------------------- |
| RFC-007 (Function syntax) | Function. "Zero-arg minimal" `name = { return ... }`                             |
| RFC-010 / 010a            | Block value. "`= {}` is a value-bearing block, its value is the tail expression" |

The same syntactic position, two semantics. Implementation-wise, this caused `callable_parts()` to
register a block as a 0-arg function while `generate_block_ir` took the block-value reading—two
inconsistent layers.

### Original Ruling: Annotation First, Default Function (**Deprecated**)

| Case                        | Result          | Basis                                     |
| --------------------------- | --------------- | ----------------------------------------- |
| `value` is `Lambda` (`=>`)  | Function        | `=>` is the explicit function constructor |
| Annotation is `Fn`          | Function        | Declared function type                    |
| Annotation is non-`Fn` type | **Block value** | Annotation is the type (`x: Int = {..}`)  |
| No annotation               | **Function**    | Default; RFC-007 "zero-arg minimal"       |

**Key: block-value semantics need no new syntax.** To evaluate `{ ... }` on the spot, just write the
target type:

```yaoxiang
x: Int = { y = 5; y }      # Block value: x = 5 (annotation non-Fn)
f: () -> Int = { 5 }       # Function: f() = 5 (annotation is Fn)
f = { 5 }                  # Function: f() = 5 (no annotation, default function)  # ← This line deprecated
b = () => 5                # Function: expression-body lambda
```

### Original Rationale (**Invalid**)

The annotation **already declares the type**: `x: Int = ...` says `x` is `Int`, so `{...}` is an
`Int` value; `f: () -> Int = ...` says `f` is a function, so `{...}` is the function body.
**Type-driven, not a new invented rule.**

Default function when no annotation, because:

1. Zero changes to existing code (203 unannotated `name = { ` bindings in the repo, all `main`)
2. Function definition is high-frequency, block value is low-frequency
3. RFC-007 is accepted, changing it is more expensive than changing this RFC

### Cost (Known and Accepted)

In `f = { 5 }`, `f` is a **function** rather than `5`. This runs counter to the intuition that
"braces are values"—but that is exactly RFC-007's accepted definition, and **the annotation is the
escape hatch**: write `f: Int = { 5 }` to get the value.

### Implementation Defects Fixed by the Ruling

1. **Annotation must not decide value-taking** (root cause one): three places including
   `generate_function_ir` used `return_type != Void` as the threshold for taking the tail
   expression's value, causing unannotated `f = { 5 }`'s tail expression to be silently discarded,
   returning `Void`. The annotation only decides whether to _check_, not whether to _evaluate_.
2. **`callable_parts()` no longer unconditionally swallows blocks**: dispatch is now uniformly
   decided by `Expr::block_binding_is_function(annotation, value)`.

## References

- [RFC-007: Unified Function Definition Syntax](./007-function-syntax-unification.md)
- [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)
- [RFC-038: Statement Termination and Newline Rules](038-statement-termination.md)
- [RFC-030: assert Mechanism](./030-assert-mechanism.md) — refined-type application of `Never`
- [Language Spec §Type System](/reference/language-spec/type-system.md) — ⊥ / ⊤ positioning of
  `Never` / `Void`
- [Rust Reference: `!` never type](https://doc.rust-lang.org/reference/types/never.html) —
  isomorphic application of the explosion principle
