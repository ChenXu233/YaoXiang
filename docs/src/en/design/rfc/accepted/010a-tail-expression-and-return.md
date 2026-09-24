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
>   as a dependency-driven computation unit
> - [RFC-007: Function Definition Syntax Unification](./007-function-syntax-unification.md) — code
>   block return rules, early return
> - [RFC-038: Statement Termination and Line-Break Rules](038-statement-termination.md) — line-break
>   behavior of statements and expressions
> - [Language Specification §Type System](/reference/language-spec/type-system.md) — the `Never`
>   explosion principle

## Summary

Unify the semantics of `return` and block evaluation, eliminating the wording conflict between
RFC-007 and RFC-010.

This RFC proposes **three self-consistent rules**: a block's value equals its tail expression (the
sole exit); `return` is a **non-local exit** of type `Never` (exits the function, not "returned to
the block"); and an `if` without `else` takes `Void`.

`return` and block evaluation **are not bifurcated at the language level**—`{ return n }` as a block
has value `n` (of type `Never`), while at the same time the effect of `return` is to exit the
function. The two coexist via the **explosion principle** (`Never <: T`). RFC-007's "early return"
and RFC-010's "blocks have values" are corollaries of these three rules, not contradictory special
cases.

No new syntax, no new keywords.

## Implementation Status

This RFC is accepted. Implementation status of the three rules:

| Rule                               | Subitem                                                      | Status                                                     |
| ---------------------------------- | ------------------------------------------------------------ | ---------------------------------------------------------- |
| ① Block's value = tail expression  | Function body tail expression                                | ✅ Implemented                                             |
| ① Block's value = tail expression  | Tail-position `if` / `match`                                 | ✅ Implemented (#344)                                      |
| ① Block's value = tail expression  | Trailing assignment statement → `Void`                       | ✅ Implemented                                             |
| ① Block's value = tail expression  | Empty block `{}` → `Void`                                    | ✅ Implemented                                             |
| ① Safeguard via type checking      | Tail expression agrees with declared return type             | ✅ Implemented (#345)                                      |
| ② `return` is non-local exit       | Pierces `if` / `while` / `for` / bare blocks / nested blocks | ✅ Implemented                                             |
| ② `Never <: T` explosion principle | Consistent across `unify` and `is_subtype`                   | ✅ Implemented (this RFC fixes the internal contradiction) |
| ③ `if` without `else` → `Void`     | Branch value does not leak into expression position          | ✅ Implemented (#346)                                      |
| ① Block's value = tail expression  | Bare-block value binding `x = { ... }`                       | ✅ Implemented (#343)                                      |
| ① Block's value = tail expression  | `unsafe {}` value exit                                       | ✅ Implemented (#347)                                      |
| ① Safeguard via type checking      | Empty block / trailing-statement escape validation           | ✅ Implemented (#342 Open Question 5)                      |
| ① Block's value = tail expression  | `spawn {}` value exit (tail expression)                      | ✅ Implemented (#365)                                      |

Corpus coverage: `tests/yaoxiang/03-semantics/rfc010a_block_value.yx` (positive) +
`tests/yaoxiang/06-compile-errors/tail_expr_type_mismatch{,_with_stmts}_err.yx` (negative).

## Motivation

### Wording Conflict

The Fibonacci example in the playground exposed a semantic divergence:

```yaoxiang
fib: (n: Int) -> Int = {
  if n <= 1 {
    return n          // Is this `return` for the `if` or for the function?
  }
  return fib(n - 1) + fib(n - 2)
}
```

Two accepted RFCs derive **opposite** conclusions:

| RFC                    | Statement                                                                                                                              | Derived Semantics                                                               |
| ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| **RFC-007** (Accepted) | Uses `factorial` as the example, with the heading **"Early return: using `return`"**: `if n <= 1 { return 1 }`                         | `return` pierces both `if` and the function body, **returning to the function** |
| **RFC-010** (Accepted) | "`{}` is a dependency-driven computation unit…use `return` to return a value explicitly"; `spawn { return c }` returns the task result | `return` gives the value to **this `{}`**                                       |

Per RFC-010, the braces of `if n <= 1 { return 1 }` form a computation unit; `return 1` gives the
block the value 1, so the value of the `if` statement is discarded and the next line must
execute—**infinite recursion in fib**. Per RFC-007, the result is correct.

### Root Cause: The Double Duty of `return`

- **RFC-007 uses it to express "exit the function"**—a control-flow concept
- **RFC-010 uses it to express "this block's value is it"**—an evaluation concept

Using a control-flow keyword to express evaluation is a **category error**. A single word bearing
two responsibilities: however you adjust, one side gets sacrificed.

### Erroneous Extended Interpretation in Downstream Documents

`docs/src/reference/language-spec/syntax.md` has extended RFC-010's "`{}` block" to **all braces**:

- §2.9: "`return` in `{}` **always returns its content to the enclosing scope**" (also calling it
  "unified semantics: all `{}` blocks")
- §3.3: "`return` is used to **return a value from a code block**"

But RFC-010's original text defines only **three value-bearing block forms** (`= {}` / `spawn {}` /
`unsafe {}`), and **nowhere in it discusses the braces of `if` / `while` / `for` / `match`**.

### Current Implementation Status

| Behavior                                        | Current State                                  |
| ----------------------------------------------- | ---------------------------------------------- |
| `if n == 0 { return 7 } … return 8`             | Function exit (`h(0) = 7`)                     |
| `f = { n + 1 }`                                 | Tail expression works (returns `5`)            |
| `x = { y = 5; y }` (bare block tail expression) | `E3006` variable unresolved (see #343)         |
| `f: () -> Int = { if c {5} else {6} }`          | Returns `void`, tail expression dropped (#344) |
| `f: () -> Int = { "s" }`                        | Silently passes compilation (see #345)         |
| `x = if c { 19 }` (without `else`)              | `19` (should be `Void`, see #346)              |
| `v = unsafe { 42 }`                             | `void` (see #347)                              |
| `y = if c { 111 } else { 222 }`                 | Works (`if` as expression)                     |

`tests/yaoxiang/03-semantics/no_tail_expr_return.yx` once claimed "tail expressions no longer
implicitly return," but didn't cover this case, so the test didn't fail. That file has been replaced
by `tests/yaoxiang/03-semantics/tail_expr_and_return.yx`.

## Proposal

### The Three Rules

```
① The block's value = the tail expression (sole exit)
   Assignment statements have value Void; the empty block {} has value Void
② return : (T) -> Never
   Non-local exit: exits the nearest function boundary
③ if without else → Void
```

**These three suffice to derive "early return" without needing any additional rule that singles out
`return` for functions.**

### Rule ①: The Block's Value

The **last statement/expression** of a block is the block's value (the tail expression). This is
**not** "no tail expression ⇒ Void"—a non-empty block always has a tail expression (the last
statement _is_ one); the empty block `{}` has value `Void`.

```yaoxiang
// The tail expression determines the block's value
a = {
    x = compute()        // Assignment statement → Void
    x * 2                // Tail expression → block's value
}

// An assignment as tail expression → block's value is Void
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

**Design criterion (separating mechanism from safeguard)**:

- **The language rule only provides the mechanism**: the last statement is the block's value—single
  rule, unambiguous
- **Safeguard is provided by type checking**: a function declared `-> Int` with a tail expression of
  an incompatible type → compile error
- **The language does not guard against "wrong intent"**: if the tail expression's type happens to
  match the return type but its semantics isn't what was intended, that's the author's oversight—the
  language has no way to tell. **We do not rely on language rules to prevent intent errors.**
- **If you don't want to return, write `Void` explicitly**

This replaces RFC-010's "`= { ... }` must use `return`, otherwise returns `Void`," as well as its
design rationale "an explicit `return` is needed to eliminate the ambiguity of 'whether the last
expression is the return value'."

### Rule ②: `return` Is a Non-Local Exit

`return` has type `Never` (zero constructors, no inhabitable value). Its semantics:

- **Exits the nearest function boundary**, handing the value to the caller
- **Pierces all blocks**—(any) `if` / `while` / `for` / `match` / bare block / `spawn` / `unsafe`

`return` does not "return to the block." `{ return n }` as a block has value `n` (by the tail
expression rule), of type `Never`; and at the same time the effect of `return` is to exit the
function. **Both hold simultaneously.**

### Rule ③: `if` Without `else`

```yaoxiang
x = if c { 19 }        // no else
```

When the condition is false there is no branch to evaluate, so the value is `Void`. Hence this `if`
has value type `Void` (and cannot be used in non-`Void` positions).

When a value is needed, supply both branches explicitly:

```yaoxiang
x = if c { 19 } else { 20 }
```

### `Never` Is the Technical Foundation for Coexistence

`Never <: T` holds for any type `T` (the explosion principle; see Language Specification §Type
System). Therefore:

```yaoxiang
fib: (n: Int) -> Int = {
  if n <= 1 {
    return n              // tail expression : Never
  }                       // → the if's value : Never
  fib(n - 1) + fib(n - 2) // → the block's value : Int
}
```

- The branch body `{ return n }` has value `Never`
- The `if`'s only branch is `Never` ⇒ the `if`'s value is `Never`
- A statement of type `Never` means **the sequence ends here**—the following statement is not "the
  next to execute sequentially"
- And since `Never` can be coerced to any type (the explosion principle), the whole block satisfies
  `-> Int`

**"Early return" follows naturally from this**: `Never` ends the sequence + the explosion principle
enables coercion.

## Detailed Design

### Formalization of Block and Tail Expression

```
Block        ::= '{' Stmt* '}'                     // empty block → Void
               | '{' Stmt* Expr '}'                // value = Expr
Expr         ::= ...
               | Return                            // type Never
Stmt         ::= Assignment | ExprStmt | ...

Value(Block):
  empty block             → Void
  { ...; e }              → type(e)
  { ...; s } (s a stmt)   → Void          // assignment's value is Void
```

### Typing Rule for `return`

```
return e : Never        where e : T

// Because Never <: T' holds for any T', return can appear in any return-type position
```

**No additional rule is needed to constrain the legality of `return`**—the explosion principle
already covers it. This makes questions like "can `return` appear in a function that returns `X`"
simply disappear.

### Multi-Branch Join

```
join(A, B):
  if A : Never  → B
  if B : Never  → A
  otherwise     → require A and B to be compatible (same type or with a common upper bound)
```

The multiple branches of `if` / `match` are joined via `join`. Branches containing `return` have
type `Never` and do not participate in the join.

### Consistency with RFC-007

RFC-007's example fits this RFC exactly, with no revision needed:

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }          // Never branch, ignored by join
    return n * factorial(n - 1)     // function exit
}
```

"Early return" is not a special case; it is a **corollary** of rules ① ② and the explosion
principle.

### Differences from RFC-010 and Revision Requirements

RFC-010's definitions have been revised per this RFC. The differences:

| RFC-010 Original Clause                                                           | Current Definition                                                                        |
| --------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| "`= { ... }` must use `return`, otherwise returns `Void`"                         | The block's value = tail expression; empty block → `Void`                                 |
| Design rationale: "explicit `return` needed to resolve tail-expression ambiguity" | Does not hold—the tail expression is unambiguous, and `return` does not interfere with it |
| The three `return c` / `return SqliteDb` examples                                 | Tail-expression form (unified value exit for `spawn` / `unsafe`)                          |

**RFC-010's core design (`{}` as a dependency-driven computation unit) is unchanged**; only the
value exit changes from `return` to the tail expression.

### Unified Perspective (Design Principle)

The braces of `if` / `while` are **both an imperative control body and a declarative evaluation
unit**—this is a **difference in perspective, not two language constructs**. Therefore:

- **Do not bifurcate at the language level** between "control-flow body" and "evaluation unit"
- All blocks share the same evaluation rules (Rule ①)
- `return` is the sole exception mechanism, and its exceptional nature comes from the type-theoretic
  property of `Never`, not from a syntactic special case

This lets declarative and Python-style writing naturally coexist:

```yaoxiang
a = if input > 10 { 19 } else { 20 }
b = { x = f(); x * 2 }
c = spawn { fetch("a") }
```

## Trade-offs

### Advantages

- **Eliminates the RFC conflict**: RFC-007 and RFC-010 go from contradictory to corollary-relation,
  with neither side sacrificed
- **Minimum number of rules**: three rules + one type-theoretic property (the explosion principle),
  no special cases
- **Single meaning of `return`**: exits the function. No more need to judge between "block value /
  function value"
- **Unified perspective**: imperative and declarative coexist, not bifurcated at the language level
- **Consistent with mature languages**: Rust also lets the two mechanisms ("tail expression" +
  `return : !`) coexist
- **Zero new syntax**: no new keywords, no parser grammar changes

### Drawbacks

- **Last-expression-as-value carries a risk of "accidental return"**: forgetting to delete the last
  line silently changes the return value
  - Mitigation: type checking intercepts type mismatches; the language does not promise to prevent
    intent errors (see Design Criterion)
- **Downstream documents need synchronized revision**: examples in already-accepted documents must
  be rewritten
- **Interaction with statement termination needs to be clarified**: is a newline-terminated last
  expression still the block's value (see Open Questions)

## Alternatives

### Alternative A: `return` Retains Double Duty, Dispatched by Containing Block Type

In a function body `= {}`, `return` belongs to the function; in `spawn {}` / `unsafe {}`, to the
block; in `if {}` / bare blocks, to what?

**Reason for rejection**: The belonging of `if` cannot be adjudicated—this is precisely the original
conflict. And the user would have to remember "which blocks belong to whom," with no principled
basis (why should `if` belong to the function while a bare block belongs to itself?).

### Alternative B: Strict Block Return (`return` Always Belongs to the Current Block)

**Reason for rejection**: **Missing functionality**. `return` can never exit a function early from a
nested block; guard clauses (`if err { return }`) become impossible to write, and the function can
only be written as a stack of nested expressions.

### Alternative C: Block Value via a New Keyword (`give x` / `yield x`)

**Reason for rejection**: Violates the **zero syntax change** principle of RFC-036 (would require a
new keyword), and users have to learn two concepts (`return` for exit + `give` for evaluation). The
tail-expression approach has zero new concepts.

### Alternative D: Keep RFC-010 As-Is (Explicit `return` Required)

**Reason for rejection**: Irreconcilable conflict with RFC-007 (see Motivation). And the actual
implementation has already taken the tail-expression path.

## Revisions to Downstream Documents

This RFC's definitions have become the authoritative semantics for downstream documents; the related
documents have all been synchronized (no deprecated wording is retained).

## Open Questions

- [x] Interaction between tail expressions and RFC-038 statement-termination rules: does a
      newline-terminated last expression remain the block's value? (@chenxu: needs to be confirmed
      together with RFC-038's "leading `(` / `[` on a new line never merge" rule). Empirically
      verified: newline-terminated last expressions (including leading `(` / `[` / list literals)
      all serve as block values.
- [x] When is `name = { ... }` a function vs. a block value binding—see Appendix D (content
      determines the type).
- [x] Specific forms of `unsafe {}` / `spawn {}` after revision—both have empirically verified
      working tail expressions.
- [x] Interaction of `match` branch `join` with exhaustiveness checking (depends on RFC-039).
      Empirically verified: `Never` branches do not participate in the join; the join behavior of
      multi-branch `if` / `match` is correct.
- [x] Diagnostic message for empty block `{}` as a function body when the return type is not
      `Void`—reuses the existing `E1012`, with the location pointing at the annotation.

---

## Appendix A: Empirical Evidence

All reproduced below on 0.8.0.

| Code                                              | Empirically Verified              |
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

## Appendix B: Design Decision Record

| Decision                            | Decision                                                                                   | Reason                                                               | Date       |
| ----------------------------------- | ------------------------------------------------------------------------------------------ | -------------------------------------------------------------------- | ---------- |
| `return` semantics                  | Function exit, type `Never`; does not return to the block                                  | Eliminate the category error of double duty                          | 2026-09-15 |
| Block value exit                    | Tail expression (sole exit)                                                                | Minimum number of rules; consistent with Rust                        | 2026-09-15 |
| No tail expression                  | Does not occur (a non-empty block always has a tail expression; empty block `{}` → `Void`) | If you want `Void`, write `Void` explicitly                          | 2026-09-15 |
| Assignment's value                  | `Void`                                                                                     | Assignment is a statement, not value production                      | 2026-09-15 |
| `if` without `else`                 | `Void`                                                                                     | When the condition is false, no branch can be evaluated              | 2026-09-15 |
| Division of mechanism and safeguard | Language provides the mechanism, type checking provides the safeguard                      | The language does not promise to prevent "intent errors"             | 2026-09-15 |
| Control-flow body / evaluation unit | Not bifurcated at the language level; treated as a difference in perspective               | Imperative and declarative coexist; avoid unprincipled special cases | 2026-09-15 |
| Handling of error examples          | Delete directly; do not keep erroneous code                                                | Keeping seemingly usable erroneous code is misleading                | 2026-09-15 |

## Appendix C: Glossary

| Term                | Definition                                                                                                         |
| ------------------- | ------------------------------------------------------------------------------------------------------------------ |
| Tail expression     | The last expression in a block that produces a value; the block's value is it.                                     |
| Non-local exit      | A control-flow transfer that crosses outer evaluation units and acts directly on the function boundary (`return`). |
| Explosion principle | `Never <: T` holds for any `T`, enabling `Never` to be coerced to any type.                                        |
| Value-bearing block | `= {}` / `spawn {}` / `unsafe {}`—the value exit is the tail expression.                                           |
| join                | The multi-branch merge rule; `Never` branches do not participate in the join.                                      |

## Appendix D: Function / Block Value Ambiguity Adjudication

### Problem

`name = { ... }` had been defined as two different things by two accepted RFCs: RFC-007 (function
syntax) treated it as a function (the "simplest no-arg" form, `name = { return ... }`); RFC-010 /
010a treats it as a block value (`= {}` is a value-bearing block, whose value is the tail
expression). The same syntactic position with two semantics caused, implementationally,
`callable_parts()` to register the block as a 0-arg function while `generate_block_ir` took it as a
block value—the two layers of understanding are inconsistent.

### Ruling: Content Determines the Type

The same principle should span dicts and blocks:

| Case                          | Result                    | Basis                                        |
| ----------------------------- | ------------------------- | -------------------------------------------- |
| `value` is a `Lambda` (`=>`)  | Function                  | `=>` is an explicit function constructor     |
| Annotation is `Fn`            | Function                  | A function type has been declared            |
| Annotation is a non-`Fn` type | Block value               | The annotation is the type (`x: Int = {..}`) |
| **No annotation**             | **Inferred from content** | **The type is determined by the content**    |

```yaoxiang
x: Int = { y = 5; y }      // Block value: x = 5 (annotation is not Fn)
f: () -> Int = { 5 }       // Function: f() = 5 (annotation is Fn)
f = { 5 }                  // Value: f = 5 (no annotation → content inference)
f = {}                     // Value: f = Void (empty block)
b = () => 5                // Function: explicit lambda
d = { "a": 1 }             // Value: Dict (content self-describing)
```

**Why not "no annotation defaults to function"**: that would make `f = { 5 }` a function while
`d = { "a": 1 }` is a dict—the same `{` at the same no-annotation position would give categorically
different things. All three previously considered reasons fail:

1. Zero changes to existing code—migration cost is payable (211 files, 704 sites), not a semantic
   basis
2. Function definition is high-frequency, block value is low-frequency—frequency is not a type rule
3. RFC-007 has been accepted; modifying it is more expensive than modifying this RFC—wrong things
   don't become right because they're "expensive" to change

**The type is determined by the content**, regardless of whether an annotation is present. The
annotation still declares the type (`f: () -> Int`), but its absence does not impose a default
value.

### Placement of `{}`

The `Dict` grammar requires at least one key; `{}` has no content to rely on, so it takes the zero
form of block structure → empty block, value `Void`. Use `dict.new()` for an empty dict. See spec
[§2.9.1](../../../reference/language-spec/syntax.md) for details.

### Accompanying Implementation Fixes

1. **The annotation must not determine value-taking**: `generate_function_ir` and two other places
   used `return_type != Void` as a threshold for taking the tail expression's value, causing the
   tail expression of an unannotated `f = { 5 }` to be silently discarded and the function to return
   `Void`. The annotation only determines whether to _check_, not whether to _evaluate_.
2. **`callable_parts()` no longer unconditionally swallows blocks**: the dispatch is now uniformly
   determined by `Expr::block_binding_is_function(annotation, value)`.

## References

- [RFC-007: Function Definition Syntax Unification](./007-function-syntax-unification.md)
- [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)
- [RFC-038: Statement Termination and Line-Break Rules](038-statement-termination.md)
- [RFC-030: assert Mechanism](./030-assert-mechanism.md) — refined-type application of `Never`
- [Language Specification §Type System](/reference/language-spec/type-system.md) — `Never` / `Void`
  as ⊥ / ⊤
- [Rust Reference: `!` never type](https://doc.rust-lang.org/reference/types/never.html) —
  homologous application of the explosion principle
