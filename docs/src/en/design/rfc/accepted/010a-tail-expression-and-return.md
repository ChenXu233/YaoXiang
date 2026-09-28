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
>   `{}` as a dependency-driven compute unit
> - [RFC-007: Unified Function Definition Syntax](./007-function-syntax-unification.md) — block
>   return rules, early return
> - [RFC-038: Statement Termination and Line Break Rules](038-statement-termination.md) — line-break
>   behavior for statements and expressions
> - [Language Spec §Type System](../../../reference/language-spec/type-system.md) — the `Never`
>   explosion principle

## Summary

Unify the semantics of `return` and block evaluation, and eliminate the description conflicts
between RFC-007 and RFC-010.

Propose **three self-consistent rules**: a block's value equals its tail expression (the sole exit);
`return` is a **non-local exit** of type `Never` (exits the function, does not "return to the
block"); an `if` without an `else` yields `Void`.

`return` and block evaluation are **not bifurcated at the language level**—`{ return n }` as a block
has value `n` (type `Never`), while at the same time `return`'s effect is to exit the function; the
two coexist via the **explosion principle** (`Never <: T`). The "early return" of RFC-007 and the
"blocks have values" of RFC-010 are corollaries of these three rules, not contradictory special
cases.

No new syntax, no new keyword.

## Implementation Status

This RFC has been accepted. Implementation status of the three rules:

| Rule                                | Sub-item                                                   | Status                                             |
| ----------------------------------- | ---------------------------------------------------------- | -------------------------------------------------- |
| ① Block's value = tail expression   | Function body tail expression                              | ✅ Implemented                                     |
| ① Block's value = tail expression   | Tail-position `if` / `match`                               | ✅ Implemented (#344)                              |
| ① Block's value = tail expression   | Trailing assignment statement → `Void`                     | ✅ Implemented                                     |
| ① Block's value = tail expression   | Empty block `{}` → `Void`                                  | ✅ Implemented                                     |
| ① Protection provided by type check | Tail expression unified with declared return type          | ✅ Implemented (#345)                              |
| ② `return` non-local exit           | Exits through `if`/`while`/`for`/bare blocks/nested blocks | ✅ Implemented                                     |
| ② `Never <: T` explosion principle  | Consistent in `unify` and `is_subtype`                     | ✅ Implemented (internal contradictions corrected) |
| ③ `if` without `else` → `Void`      | Branch values don't leak at expression position            | ✅ Implemented (#346)                              |
| ① Block's value = tail expression   | Bare-block value binding `x = { ... }`                     | ✅ Implemented (#343)                              |
| ① Block's value = tail expression   | `unsafe {}` value exit                                     | ✅ Implemented (#347)                              |
| ① Protection provided by type check | Empty block / trailing statement escape check              | ✅ Implemented (#342 open issue 5)                 |
| ① Block's value = tail expression   | `spawn {}` value exit (tail expression)                    | ✅ Implemented (#365)                              |

Corpus coverage: `tests/yaoxiang/03-semantics/rfc010a_block_value.yx` (positive) +
`tests/yaoxiang/06-compile-errors/tail_expr_type_mismatch{,_with_stmts}_err.yx` (negative).

## Motivation

### Description Conflicts

The Fibonacci example in the playground exposed a semantic divergence:

```yaoxiang
fib: (n: Int) -> Int = {
  if n <= 1 {
    return n          // Is this `return` the `if`'s or the function's?
  }
  return fib(n - 1) + fib(n - 2)
}
```

Two already-accepted RFCs give **opposite** derivations:

| RFC                    | Description                                                                                                                        | Derived Semantics                                                       |
| ---------------------- | ---------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------- |
| **RFC-007** (Accepted) | Using `factorial` as an example, the title reads **"Early Return: Using `return`"**: `if n <= 1 { return 1 }`                      | `return` exits the `if` and the function body, **back to the function** |
| **RFC-010** (Accepted) | "`{}` is a dependency-driven compute unit…use `return` to explicitly return a value"; `spawn { return c }` returns the task result | `return` gives its value **to this `{}`**                               |

According to RFC-010, the braces in `if n <= 1 { return 1 }` are a compute unit, `return 1` gives
its value 1 to it, so the `if` statement's value is discarded and the next line must execute—**fib
recurses infinitely**. According to RFC-007 it's correct.

### Root Cause: `return` Has Two Jobs

- **RFC-007 uses it to mean "exit the function"**—a control-flow concept
- **RFC-010 uses it to mean "this block's value is it"**—an evaluation concept

Using a control-flow keyword to express evaluation is a category error. One word carrying two jobs
means no matter how you adjust it, one side is sacrificed.

### Downstream Documentation's Overreach

`docs/src/reference/language-spec/syntax.md` extends RFC-010's "`{}` block" to **all braces**:

- §2.9: "`return` in `{}` **always returns the content to the enclosing scope**" (and calls it
  "unified semantics: all `{}` blocks")
- §3.3: "`return` is used to **return a value from a code block**"

But RFC-010's original text defines **three value-bearing blocks** (`= {}` / `spawn {}` /
`unsafe {}`), and **never touches the braces of `if` / `while` / `for` / `match`**.

### Current Implementation

| Behavior                                  | Current State                              |
| ----------------------------------------- | ------------------------------------------ |
| `if n == 0 { return 7 } … return 8`       | Function exits (`h(0) = 7`)                |
| `f = { n + 1 }`                           | Tail expression usable (returns `5`)       |
| `x = { y = 5; y }` (bare block tail expr) | `E3006` variable unresolved (see #343)     |
| `f: () -> Int = { if c {5} else {6} }`    | Returns `void`, tail expr discarded (#344) |
| `f: () -> Int = { "s" }`                  | Silently passes compilation (see #345)     |
| `x = if c { 19 }` (no `else`)             | `19` (should be `Void`, see #346)          |
| `v = unsafe { 42 }`                       | `void` (see #347)                          |
| `y = if c { 111 } else { 222 }`           | Usable (`if` as expression)                |

`tests/yaoxiang/03-semantics/no_tail_expr_return.yx` once claimed "tail expressions no longer
implicitly return," but didn't cover that case, so the test didn't fail. That file has been replaced
by `tests/yaoxiang/03-semantics/tail_expr_and_return.yx`.

## Proposal

### Three Rules

```
① A block's value = tail expression (the sole exit)
   Assignment statements have value Void; empty block {} has value Void
② return : (T) -> Never
   Non-local exit: exits the nearest function boundary
③ if without else → Void
```

**These three suffice to derive "early return"—no extra rule making `return` function-specific is
needed.**

### Rule ①: A Block's Value

The **last statement/expression** of a block is the block's value (tail expression). This is **not**
"no tail expression then `Void`"—a non-empty block always has a tail expression (the trailing
statement itself is one), and an empty block `{}` has value `Void`.

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

// To want Void, write it explicitly
c = {
    log(x)
    Void                 // Explicit Void
}
```

**Design criterion (separation of mechanism and protection)**:

- **The language rule provides only the mechanism**: the trailing statement is the block's value;
  the rule is unique and unambiguous
- **Protection is provided by type checking**: a function declares `-> Int` but the tail
  expression's type doesn't match → compile error
- **The language does not guard against "wrong intent"**: if the tail expression's type happens to
  match the return type but the semantics are not what was intended, that's the author's oversight;
  the language cannot judge. **Do not rely on language rules to prevent wrong intent.**
- **If you don't want to return, write `Void` explicitly**

This replaces RFC-010's "`= { ... }` must use `return`, otherwise it returns `Void`", and replaces
its rationale of "needing explicit `return` to remove the ambiguity of 'whether the last expression
is a return value'".

### Rule ②: `return` Is a Non-Local Exit

`return`'s type is `Never` (zero constructors, no inhabitable value). Its semantics:

- **Exits the nearest function boundary**, handing the value to the caller
- **Passes through all blocks**—(if any) `if` / `while` / `for` / `match` / bare blocks / `spawn` /
  `unsafe`

`return` does not "return to the block". `{ return n }` as a block has value `n` (tail expression
rule), **type `Never`**; at the same time `return`'s effect is to exit the function. **Both hold
simultaneously.**

### Rule ③: `if` Without `else`

```yaoxiang
x = if c { 19 }        // No else
```

When the condition is false there is no branch to evaluate, so the value is `Void`. Hence this
`if`'s value type is `Void` (or unusable in a non-`Void` position).

When a value is needed, supply both branches explicitly:

```yaoxiang
x = if c { 19 } else { 20 }
```

### `Never` Is the Technical Basis for Coexistence

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
- This `if`'s sole branch is `Never` ⇒ the `if`'s value is `Never`
- A statement of type `Never` means **the sequence halts here**—the following line is not "the next
  line in order of execution"
- And `Never` can be coerced to any type (explosion principle), so the entire block satisfies
  `-> Int`

**"Early return" follows naturally from this**: `Never` halts the sequence + the explosion principle
allows coercion.

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
  { ...; s } (s is a stmt) → Void          // Assignment's value is Void
```

### Type Rules for `return`

```
return e : Never        where e : T

// Because Never <: T' for any T', return may appear in any return-type position
```

**No additional rule is needed to constrain `return`'s legality**—the explosion principle already
covers it. This makes questions like "can `return` appear in a function returning `X`" disappear.

### Multi-Branch Joining

```
join(A, B):
  If A : Never  → B
  If B : Never  → A
  Otherwise     → A and B must be compatible (same type or reachable common upper bound)
```

Multiple branches of `if` / `match` are joined per `join`. Branches containing `return` have type
`Never` and don't participate in the join.

### Consistency with RFC-007

RFC-007's examples **fully conform to this RFC** without revision:

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }          // Never branch, ignored by join
    return n * factorial(n - 1)     // Function exits
}
```

"Early return" is not a special case; it's a **corollary** of rules ①, ② and the explosion
principle.

### Differences from RFC-010 and Revision Requirements

RFC-010's definitions have been revised per this RFC; the differences are as follows:

| RFC-010 Original Clause                                                  | Current Definition                                                                            |
| ------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------- |
| "`= { ... }` must use `return`, otherwise it returns `Void`"             | A block's value = tail expression; empty block → `Void`                                       |
| Rationale "needs explicit `return` to resolve tail-expression ambiguity" | No longer holds—tail expressions don't create ambiguity; `return` doesn't interfere with them |
| Three `return c` / `return SqliteDb` examples                            | Tail expression form (`spawn` / `unsafe` value exits unified)                                 |

**RFC-010's core design (`{}` as a dependency-driven compute unit) is unchanged**; only the value
exit is changed from `return` to a tail expression.

### Unifying the Perspective (Design Principle)

The braces of `if` / `while` are **both imperative control bodies and declarative evaluation
units**—this is a **difference in perspective, not two distinct language constructs**. Therefore:

- **Do not bifurcate at the language level** between "control-flow body" and "evaluation unit"
- All blocks share the same set of evaluation rules (rule ①)
- `return` is the only exception mechanism, and its exceptional nature comes from `Never`'s
  type-theoretic property, not from a syntactic special case

This lets declarative and Pythonic styles coexist naturally:

```yaoxiang
a = if input > 10 { 19 } else { 20 }
b = { x = f(); x * 2 }
c = spawn { fetch("a") }
```

## Trade-offs

### Advantages

- **Eliminates RFC conflicts**: RFC-007 and RFC-010 shift from contradiction to a corollarial
  relationship, with no need to sacrifice either
- **Fewest rules**: three rules + one type-theoretic property (explosion principle), no special
  cases
- **Single meaning for `return`**: exits the function. No more need to judge between "block value /
  function value"
- **Unified perspective**: imperative and declarative coexist; no bifurcation at the language level
- **Consistent with mature languages**: Rust likewise uses "tail expression + `return : !`" to make
  the two mechanisms coexist
- **Zero new syntax**: no new keyword, no parser grammar changes

### Disadvantages

- **"Accidental return" risk from trailing expression**: forgetting to delete the last line silently
  changes the return value
  - Mitigation: type checking intercepts type mismatches; the language does not promise to prevent
    wrong intent (see design criterion)
- **Downstream documentation must be revised synchronously**: examples in already-accepted documents
  must be rewritten
- **Interaction with statement termination must be clarified**: does a line-break-terminated
  trailing expression still count as the block's value? (see open questions)

## Alternatives

### Option A: Keep `return`'s dual role, dispatch by containing block type

`return` in function body `= {}` belongs to the function; in `spawn {}` / `unsafe {}` it belongs to
the block; in `if {}` / bare blocks it belongs to…?

**Reason for rejection**: `if`'s ownership cannot be decided—this is exactly the original conflict.
And the user must memorize "which blocks go to whom," with no principled basis (why should `if`
belong to the function while a bare block belongs to itself?).

### Option B: Strict block return (`return` always belongs to the current block)

**Reason for rejection**: **Functionality loss**. `return` can never early-exit a function from a
nested block; guard clauses (`if err { return }`) become completely unwritable, and functions can
only be written as deeply nested expressions.

### Option C: Use a new keyword for block values (`give x` / `yield x`)

**Reason for rejection**: Violates RFC-036's **zero-syntax-change** principle (requires a new
keyword), and users must learn two concepts (`return` for exit + `give` for evaluation). The tail
expression approach introduces zero new concepts.

### Option D: Keep RFC-010 as-is (explicit `return` required)

**Reason for rejection**: Irreconcilable with RFC-007 (see Motivation). And the actual
implementation has already gone down the tail expression path.

## Revisions to Downstream Documentation

The definitions in this RFC have become the authoritative semantics for downstream documentation;
relevant documents have all been synchronized (no obsolete descriptions retained).

## Open Questions

- [x] Interaction between tail expressions and RFC-038's statement termination rules: does a
      line-break-terminated trailing expression still count as the block's value? (@chenxu: needs to
      be confirmed together with RFC-038's "leading `(` / `[` never merge" etc. rules) — Verified
      empirically: line-break-terminated trailing expressions (including leading `(` / `[` / list
      literals) are all block values
- [x] When does `name = { ... }` denote a function vs. a block-value binding — see Appendix D
      (content determines the type)
- [x] Concrete forms of revised `unsafe {}` / `spawn {}` — both have been verified empirically as
      usable via tail expressions
- [x] Interaction between `match` branch `join` and exhaustiveness checking (depends on RFC-010b) —
      Verified empirically: `Never` branches don't participate in joining, and the join behavior of
      multi-branch `if` / `match` is correct
- [x] Diagnostic wording for an empty block `{}` as a function body when the return type is
      non-`Void` — reuses the existing `E1012`, with the position pointing at the annotation

---

## Appendix A: Empirical Evidence

All reproduced on 0.8.0.

| Code                                              | Empirical Result                  |
| ------------------------------------------------- | --------------------------------- |
| `h: (n)->Int = { if n==0 { return 7 } return 8 }` | `h(0)=7`, `h(1)=8`                |
| `f: (n)->Int = { n + 1 }`                         | `5` (tail expression usable)      |
| `f: ()->Int = { if c {5} else {6} }`              | `void` (should be `5`, see #344)  |
| `f: ()->Int = if c {5} else {6}`                  | `5`                               |
| `f: ()->Int = { match ... }`                      | Correct                           |
| `f: ()->Int = { while ...; i }`                   | Correct                           |
| `{ y = 5; y }` as a binding expression            | `E3006` (see #343)                |
| `f: ()->Int = { "s" }`                            | Silently passes (see #345)        |
| `x = if c { 19 }` (no `else`)                     | `19` (should be `Void`, see #346) |
| `v = unsafe { 42 }`                               | `void` (see #347)                 |
| `y = if c { 111 } else { 222 }`                   | `111`                             |
| `while { if i==2 { return 42 } }`                 | `42` (exits loop)                 |
| Nested `{ { return 5 } return 1 }`                | `5` (exits bare block)            |

## Appendix B: Design Decision Record

| Decision                             | Decision                                                                                 | Reason                                                               | Date       |
| ------------------------------------ | ---------------------------------------------------------------------------------------- | -------------------------------------------------------------------- | ---------- |
| `return` semantics                   | Function exit, type `Never`; does not return to the block                                | Eliminate the category error of one word with two jobs               | 2026-09-15 |
| Block value exit                     | Tail expression (the sole exit)                                                          | Fewest rules; consistent with Rust                                   | 2026-09-15 |
| No tail expression                   | Does not exist (non-empty block always has a tail expression; empty block `{}` → `Void`) | To want `Void`, write `Void` explicitly                              | 2026-09-15 |
| Assignment's value                   | `Void`                                                                                   | Assignment is a statement, not a value producer                      | 2026-09-15 |
| `if` without `else`                  | `Void`                                                                                   | No branch to evaluate when condition is false                        | 2026-09-15 |
| Division of mechanism and protection | Language provides mechanism; type checking provides protection                           | Language does not promise to prevent "wrong intent"                  | 2026-09-15 |
| Control-flow body / eval unit        | Not bifurcated at the language level; treated as a difference of perspective             | Imperative and declarative coexist; avoid unprincipled special cases | 2026-09-15 |
| Handling of incorrect examples       | Delete directly; do not keep incorrect code                                              | Keeping seemingly-usable incorrect code will mislead                 | 2026-09-15 |

## Appendix C: Glossary

| Term                | Definition                                                                                                        |
| ------------------- | ----------------------------------------------------------------------------------------------------------------- |
| Tail expression     | The last value-producing expression in a block; the block's value is it                                           |
| Non-local exit      | A control-flow transfer that crosses outer evaluation units and acts directly on the function boundary (`return`) |
| Explosion principle | `Never <: T` holds for any `T`, so `Never` can be coerced to any type                                             |
| Value-bearing block | `= {}` / `spawn {}` / `unsafe {}`—the value exit is the tail expression                                           |
| join                | The multi-branch merging rule; `Never` branches don't participate in the merge                                    |

## Appendix D: Function / Block-Value Ambiguity Adjudication

### The Problem

`name = { ... }` was defined as different things by two already-accepted RFCs: RFC-007 (function
syntax) treats it as a function (the "no-arg minimal" form `name = { return ... }`), while RFC-010 /
010a treats it as a block value (`= {}` is a value-bearing block, with the value being the tail
expression). For the same syntactic position, two sets of semantics caused the implementation to
register a block as a 0-arg function via `callable_parts()`, while `generate_block_ir` took the
value as a block value—the two layers were inconsistent.

### Adjudication: Content Determines the Type

The same principle should run through dictionaries and blocks:

| Situation                     | Result                  | Basis                                    |
| ----------------------------- | ----------------------- | ---------------------------------------- |
| `value` is a `Lambda` (`=>`)  | Function                | `=>` is an explicit function constructor |
| Annotation is `Fn`            | Function                | Declared a function type                 |
| Annotation is a non-`Fn` type | Block value             | Annotation is the type (`x: Int = {..}`) |
| **No annotation**             | **Inferred by content** | **Type determined by content**           |

```yaoxiang
x: Int = { y = 5; y }      // Block value: x = 5 (annotation is non-Fn)
f: () -> Int = { 5 }       // Function: f() = 5 (annotation is Fn)
f = { 5 }                  // Value: f = 5 (no annotation → inferred by content)
f = {}                     // Value: f = Void (empty block)
b = () => 5                // Function: explicit lambda
d = { "a": 1 }             // Value: Dict (self-describing content)
```

**Why not "no annotation defaults to function"**: That would make `f = { 5 }` a function, while
`d = { "a": 1 }` is a dictionary—the same `{` in the same no-annotation position yielding different
categories. The three reasons once considered all fail:

1. Zero changes to existing code—the migration cost is payable (211 files, 704 sites), not a
   semantic basis
2. Function definition is high frequency, block value is low—frequency is not a type rule
3. RFC-007 is already accepted, revising it is more costly than revising this RFC—revising the wrong
   thing does not make it right because it's "expensive"

**The type is determined by content**, not by the presence or absence of an annotation. The
annotation still declares the type (`f: () -> Int`), but does not impose a default because of
"missing annotation".

### Where `{}` Lands

The `Dict` grammar requires at least one key, so `{}` has no content to rely on, and therefore takes
the zero form of a block structure → empty block, value `Void`. Use `dict.new()` for an empty
dictionary. See spec [§2.9.1](../../../reference/language-spec/syntax.md) for details.

### Accompanying Implementation Fixes

1. **Annotations must not decide value-taking**: three places including `generate_function_ir` once
   used `return_type != Void` as the threshold for taking the tail expression, causing the tail
   expression in an unannotated `f = { 5 }` to be silently discarded and `Void` returned. The
   annotation only decides whether to _check_, not whether to _evaluate_.
2. **`callable_parts()` no longer unconditionally swallows blocks**: the dispatch is unified through
   `Expr::block_binding_is_function(annotation, value)`.

## References

- [RFC-007: Unified Function Definition Syntax](./007-function-syntax-unification.md)
- [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)
- [RFC-038: Statement Termination and Line Break Rules](038-statement-termination.md)
- [RFC-030: assert Mechanism](./030-assert-mechanism.md) — refined type application of `Never`
- [Language Spec §Type System](../../../reference/language-spec/type-system.md) — the ⊥ / ⊤
  positioning of `Never` / `Void`
- [Rust Reference: `!` never type](https://doc.rust-lang.org/reference/types/never.html) —
  isomorphic application of the explosion principle
