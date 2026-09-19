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
> - [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md) — `name: type = value` model, `{}`
>   dependency-driven computation unit
> - [RFC-007: Unified Function Definition Syntax](./007-function-syntax-unification.md) — code block
>   return rules, early return
> - [RFC-038: Statement Termination and Newline Rules](038-statement-termination.md) — newline
>   behavior of statements and expressions
> - [Language Spec §Type System](/reference/language-spec/type-system.md) — `Never` bottom type
>   principle

## Summary

Unify the semantics of `return` and block evaluation, eliminating the statement conflict between
RFC-007 and RFC-010.

Propose **three self-consistent rules**: a block's value equals its tail expression (the unique
outlet); `return` is a **non-local exit** of type `Never` (exits the function, does not "return to
the block"); `if` without `else` resolves to `Void`.

`return` and block evaluation are **not bifurcated at the language level** — `{ return n }` as a
block has value `n` (type `Never`), while `return`'s effect is to exit the function; both coexist
via the **bottom principle** (`Never <: T`). RFC-007's "early return" and RFC-010's "block has
value" are consequences of these three rules, not contradictory special cases.

No new syntax, no new keywords.

## Implementation Status

This RFC has been accepted. Implementation status of the three rules:

| Rule                                   | Subitem                                                 | Status                                            |
| -------------------------------------- | ------------------------------------------------------- | ------------------------------------------------- |
| ① Block's value = tail expression      | Function body tail expression                           | ✅ Implemented                                    |
| ① Block's value = tail expression      | Tail-position `if` / `match`                            | ✅ Implemented (#344)                             |
| ① Block's value = tail expression      | Last assignment statement → `Void`                      | ✅ Implemented                                    |
| ① Block's value = tail expression      | Empty block `{}` → `Void`                               | ✅ Implemented                                    |
| ① Protection provided by type checking | Tail expression unified with declared return type       | ✅ Implemented (#345)                             |
| ② `return` non-local exit              | Pass through `if`/`while`/`for`/bare block/nested block | ✅ Implemented                                    |
| ② `Never <: T` bottom principle        | `unify` and `is_subtype` consistent                     | ✅ Implemented (corrected internal inconsistency) |
| ③ `if` without `else` → `Void`         | Branch value does not leak in expression position       | ✅ Implemented (#346)                             |
| ① Block's value = tail expression      | Bare block value binding `x = { ... }`                  | ❌ Not implemented (#343, awaiting design ruling) |
| ① Block's value = tail expression      | `unsafe {}` value outlet                                | ❌ Not implemented (#347)                         |

Corpus coverage: `tests/yaoxiang/03-semantics/rfc010a_block_value.yx` (positive) +
`tests/yaoxiang/06-compile-errors/tail_expr_type_mismatch{,_with_stmts}_err.yx` (negative).

## Motivation

### Statement Conflict

A fibonacci example from the playground exposes a semantic divergence:

```yaoxiang
fib: (n: Int) -> Int = {
  if n <= 1 {
    return n          // Is this return "if's" or "function's"?
  }
  return fib(n - 1) + fib(n - 2)
}
```

Two accepted RFCs give **opposite** derivations:

| RFC                    | Statement                                                                                                                              | Derived Semantics                                                       |
| ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------- |
| **RFC-007** (Accepted) | Using `factorial` as example, the title is **"Early Return: Using return"**: `if n <= 1 { return 1 }`                                  | `return` passes through `if` and function body, **returns to function** |
| **RFC-010** (Accepted) | "`{}` is a dependency-driven computation unit…use `return` to explicitly return a value"; `spawn { return c }` returns the task result | `return` gives the value to **this `{}`**                               |

Per RFC-010, the braces in `if n <= 1 { return 1 }` are a computation unit, `return 1` gives its
value as 1, so the `if` statement's value is discarded, the next line must execute—**fib recurses
infinitely**. Per RFC-007, the result is correct.

### Root Cause: `return` Plays Two Roles

- **RFC-007 uses it to express "exit function"**—a control flow concept
- **RFC-010 uses it to express "the value of this block"**—an evaluation concept

Using a control flow keyword to express evaluation is a category error. One word playing two
roles—no matter how you adjust, one side gets sacrificed.

### Downstream Documentation's Misleading Amplification

`docs/src/reference/language-spec/syntax.md` extends RFC-010's "`{}` block" to **all braces**:

- §2.9: "`return` in `{}` **always returns the content to the enclosing scope**" (and claims
  "unified semantics: all `{}` blocks")
- §3.3: "`return` is used to **return a value from a code block**"

Whereas RFC-010's original text defines only **three value-producing blocks** (`= {}` / `spawn {}` /
`unsafe {}`), and **never mentions `if` / `while` / `for` / `match` braces**.

### Current Implementation

| Behavior                                        | Current State                                    |
| ----------------------------------------------- | ------------------------------------------------ |
| `if n == 0 { return 7 } … return 8`             | Function exits (`h(0) = 7`)                      |
| `f = { n + 1 }`                                 | Tail expression works (returns `5`)              |
| `x = { y = 5; y }` (bare block tail expression) | `E3006` variable unresolved (see #343)           |
| `f: () -> Int = { if c {5} else {6} }`          | Returns `void`, tail expression discarded (#344) |
| `f: () -> Int = { "s" }`                        | Silently passes compilation (see #345)           |
| `x = if c { 19 }` (no `else`)                   | `19` (should be `Void`, see #346)                |
| `v = unsafe { 42 }`                             | `void` (see #347)                                |
| `y = if c { 111 } else { 222 }`                 | Works (`if` as expression)                       |

`tests/yaoxiang/03-semantics/no_tail_expr_return.yx` previously claimed "tail expression no longer
implicitly returns" but didn't cover this case, so the test didn't fail. That file has been replaced
by `tests/yaoxiang/03-semantics/tail_expr_and_return.yx`.

## Proposal

### Three Rules

```
① Block's value = tail expression (unique outlet)
   Assignment statement's value is Void; empty block {}'s value is Void
② return : (T) -> Never
   Non-local exit: exits the nearest function boundary
③ if without else → Void
```

**These three suffice to derive "early return" — no need for an extra rule where `return`
specifically targets the function.**

### Rule ①: Block's Value

A block's **last statement/expression** is the block's value (the tail expression). This is **not**
"no tail expression means Void"—a non-empty block always has a tail expression (the last statement
itself), and the empty block `{}` has value `Void`.

```yaoxiang
// Tail expression determines block's value
a = {
    x = compute()        // Assignment statement → Void
    x * 2                // Tail expression → block's value
}

// Assignment as tail expression → block's value is Void
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

**Design rationale (separation of mechanism and protection)**:

- **Language rules only provide mechanism**: the last statement is the block's value, unique and
  unambiguous
- **Protection provided by type checking**: function declared `-> Int` but tail expression type
  doesn't match → compile error
- **Language does not prevent "intent errors"**: if the tail expression's type happens to match the
  return type but the semantics is unintended, that's the author's oversight—language cannot judge.
  **Don't rely on language rules to prevent intent errors.**
- **Don't want to return? Explicitly write `Void`**

This replaces RFC-010's "`= { ... }` must use `return`, otherwise returns `Void`" and its design
rationale "explicit `return` is needed to eliminate the ambiguity of 'whether the last expression is
a return value'".

### Rule ②: `return` is Non-Local Exit

`return` has type `Never` (zero constructors, no value to inhabit). Its semantics:

- **Exits the nearest function boundary**, handing the value to the caller
- **Passes through all blocks**—(if any) `if` / `while` / `for` / `match` / bare block / `spawn` /
  `unsafe`

`return` does not "return to the block". `{ return n }` as a block has value `n` (tail expression
rule), **type `Never`**; simultaneously, `return`'s effect is to exit the function. **Both hold at
the same time.**

### Rule ③: `if` Without `else`

```yaoxiang
x = if c { 19 }        // No else
```

When the condition is false, no branch is available to evaluate, so it takes `Void`. Thus this
`if`'s value type is `Void` (or unusable in non-`Void` positions).

When a value is needed, explicitly complete both branches:

```yaoxiang
x = if c { 19 } else { 20 }
```

### `Never` is the Technical Foundation for Coexistence

`Never <: T` holds for any type `T` (bottom principle, see Language Spec §Type System). Therefore:

```yaoxiang
fib: (n: Int) -> Int = {
  if n <= 1 {
    return n              // Tail expression : Never
  }                       // → this if's value : Never
  fib(n - 1) + fib(n - 2) // → block's value : Int
}
```

- The branch body `{ return n }`'s value is `Never`
- This `if` has a single branch of type `Never` ⇒ `if`'s value is `Never`
- A `Never`-typed statement means **the sequence terminates here**—the following line is not "the
  next to execute in sequence"
- And `Never` can be reduced to any type (bottom principle), so the entire block satisfies `-> Int`

**"Early return" follows naturally from this**: `Never` terminates the sequence + bottom principle
allows reduction.

## Detailed Design

### Formalization of Block and Tail Expression

```
Block        ::= '{' Stmt* '}'                     // empty block → Void
               | '{' Stmt* Expr '}'                // value = Expr
Expr         ::= ...
               | Return                            // type Never
Stmt         ::= Assignment | ExprStmt | ...

Value(Block)：
  Empty block                    → Void
  { ...; e }                     → Type(e)
  { ...; s } (s is a statement)  → Void          // Assignment's value is Void
```

### Type Rule of `return`

```
return e : Never        where e : T

// Since Never <: T' holds for any T', return can appear in any return-type position
```

**No additional rule needed to constrain `return`'s legality**—the bottom principle already covers
it. This makes the question "can `return` appear in a function returning `X`" disappear.

### Multi-Branch Joining

```
join(A, B)：
  If A : Never  → B
  If B : Never  → A
  Otherwise     → require A, B to be compatible (same type or have a common upper bound)
```

`if` / `match`'s multi-branches merge per `join`. Branches with `return` don't participate in
merging because their type is `Never`.

### Consistency with RFC-007

RFC-007's examples **fully comply with this RFC** and need no revision:

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }          // Never branch, ignored by join
    return n * factorial(n - 1)     // Function exits
}
```

"Early return" is not a special case—it's a **consequence** of Rules ①② and the bottom principle.

### Differences from RFC-010 and Revision Requirements

RFC-010 needs three corrections (as errata, see "Revision Requirements for Downstream
Documentation"):

| RFC-010 Clause                                                                        | After Revision                                                                        |
| ------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| "`= { ... }` must use `return`, otherwise returns `Void`"                             | Block's value = tail expression; empty block → `Void`                                 |
| Design rationale "explicit `return` is needed to eliminate tail expression ambiguity" | Invalidated—tail expression produces no ambiguity, `return` doesn't interfere with it |
| Three `return c` / `return SqliteDb` examples                                         | Change to tail expression (`spawn` / `unsafe`'s value outlet unified)                 |

**RFC-010's core design (`{}` as a dependency-driven computation unit) remains unchanged**; only the
value outlet changes from `return` to tail expression.

### Perspective Unification (Design Principle)

`if` / `while`'s braces are **both imperative control bodies and declarative evaluation units**—this
is a **perspective difference, not two language constructs**. Therefore:

- **Don't bifurcate at the language level** between "control flow body" and "evaluation unit"
- All blocks share the same evaluation rule (Rule ①)
- `return` is the sole exception mechanism, and its exceptionality comes from `Never`'s
  type-theoretic property, not a syntactic special case

This allows declarative and Pythonic writing styles to coexist naturally:

```yaoxiang
a = if input > 10 { 19 } else { 20 }
b = { x = f(); x * 2 }
c = spawn { fetch("a") }
```

## Trade-offs

### Advantages

- **Eliminates RFC conflict**: RFC-007 and RFC-010 go from contradiction to consequence—no need to
  sacrifice either
- **Fewest rules**: three rules + one type-theoretic property (bottom principle), no special cases
- **`return` has unique meaning**: exits the function. No more need to judge between "block value /
  function value"
- **Perspective unification**: imperative and declarative coexist, no language-level bifurcation
- **Consistent with mature languages**: Rust also uses "tail expression + `return : !`" to make both
  mechanisms coexist
- **Zero new syntax**: no new keywords, no changes to parser grammar rules

### Disadvantages

- **Last expression as value has "accidental return" risk**: forgetting to remove the last line
  silently changes the return value
  - Mitigation: type checking intercepts type mismatches; language doesn't promise to prevent intent
    errors (see design rationale)
- **RFC-010 needs errata**: examples in the accepted document need rewriting
- **Bare block tail expression pending implementation**: currently a compiler internal error
  (independent of this design)
- **Interaction with statement termination needs clarification**: does the last expression
  terminated by newline still count as the block's value? (see Open Questions)

## Alternatives

### Option A: `return` Keeps Dual Roles, Dispatched by Block Type

`return` in function body `= {}` belongs to function; in `spawn {}` / `unsafe {}` belongs to block;
in `if {}` / bare block, belongs to?

**Reason for rejection**: `if`'s ownership cannot be ruled—this is the original conflict. Also
requires users to remember "which blocks belong to whom," with no principled basis (why does `if`
belong to function but bare block belong to itself?).

### Option B: Strict Block Return (`return` Always Belongs to Current Block)

**Reason for rejection**: **Feature loss**. `return` can never early-exit a function from a nested
block; guard clauses (`if err { return }`) become completely unwritable, and functions can only be
written as deeply nested expressions.

### Option C: Block's Value Uses a New Keyword (`give x` / `yield x`)

**Reason for rejection**: Violates RFC-036's **zero syntax change** principle (requires new
keywords), and users must learn two concepts (`return` exit + `give` evaluation). The tail
expression solution has zero new concepts.

### Option D: Keep RFC-010 as Is (Must Explicitly `return`)

**Reason for rejection**: Conflicts with RFC-007 irreconcilably (see Motivation). Also, the actual
implementation has already taken the tail expression path.

## Revision Requirements for Downstream Documentation

After this RFC is passed, the following documents need to be revised. **Error examples should be
deleted entirely, not retained in erratum blocks as if they were usable.**

| Document                                                   | Revision Content                                                                                             | Status |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ | ------ |
| RFC-010 §Return Rule                                       | Change to tail expression rule; **delete** the original "must use `return` otherwise returns `Void`" example | ✅     |
| RFC-010 Design Rationale Paragraph                         | **Delete** the "explicit `return` is needed to eliminate ambiguity" argument                                 | ✅     |
| RFC-010 §`spawn` Block / §`unsafe` Block Examples          | `return c` / `return SqliteDb` → tail expression form                                                        | ✅     |
| RFC-010 §Dependency-Driven Computation Unit Example        | `return b` → tail expression `b`                                                                             | ✅     |
| `language-spec/syntax.md` §2.9                             | "`return` returns to the enclosing scope" → tail expression rule + `return` exits function                   | ✅     |
| `language-spec/syntax.md` §3.3                             | "return a value from a code block" → "exit the nearest function boundary, type `Never`"                      | ✅     |
| `language-spec/type-system.md`                             | "function without `return` defaults to `Void`" → tail expression determines                                  | ✅     |
| `tests/yaoxiang/03-semantics/no_tail_expr_return.yx`       | Assertions and comments contradict the new rules, need correction                                            | ✅     |
| RFC-007 §Summary / §Lambda Syntax Rule / Syntax Rule Table | "must use `return` otherwise returns `Void`" → tail expression + `Never` erratum block                       | ✅     |

RFC-007's "code block must use `return`, otherwise returns `Void`" sentence conflicts with this RFC
and has been revised by erratum (RFC-007's function form definition is unchanged)—see the last row
in the table above.

## Open Questions

- [ ] Interaction between tail expression and RFC-038's statement termination rules: is the last
      expression terminated by newline still the block's value? (@Chenxu: needs to be confirmed
      together with RFC-038's "line-start `(`/`[` never merge" rules)
- [x] When is `name = { ... }` a function vs. a block value binding—see Appendix D (Ruling C:
      annotation priority, default function)
- [ ] Specific form of `unsafe {}` after rewrite—currently entirely unusable (see #347)
- [ ] Interaction between `match` branch `join` and exhaustiveness check (depends on RFC-039)
- [ ] Diagnostic message when empty block `{}` is a function body and the return type is not `Void`

---

## Appendix A: Empirical Evidence

All reproduced on 0.8.0.

| Code                                              | Observed                          |
| ------------------------------------------------- | --------------------------------- |
| `h: (n)->Int = { if n==0 { return 7 } return 8 }` | `h(0)=7`, `h(1)=8`                |
| `f: (n)->Int = { n + 1 }`                         | `5` (tail expression works)       |
| `f: ()->Int = { if c {5} else {6} }`              | `void` (should be `5`, see #344)  |
| `f: ()->Int = if c {5} else {6}`                  | `5`                               |
| `f: ()->Int = { match ... }`                      | Correct                           |
| `f: ()->Int = { while ...; i }`                   | Correct                           |
| `{ y = 5; y }` as binding expression              | `E3006` (see #343)                |
| `f: ()->Int = { "s" }`                            | Silently passes (see #345)        |
| `x = if c { 19 }` (no `else`)                     | `19` (should be `Void`, see #346) |
| `v = unsafe { 42 }`                               | `void` (see #347)                 |
| `y = if c { 111 } else { 222 }`                   | `111`                             |
| `while { if i==2 { return 42 } }`                 | `42` (passes through loop)        |
| Nested `{ { return 5 } return 1 }`                | `5` (passes through bare block)   |

## Appendix B: Design Decision Log

| Decision                             | Decision                                                                                | Reason                                                               | Date       |
| ------------------------------------ | --------------------------------------------------------------------------------------- | -------------------------------------------------------------------- | ---------- |
| `return` semantics                   | Function exit, type `Never`; does not return to block                                   | Eliminate the category error of one word playing two roles           | 2026-09-15 |
| Block's value outlet                 | Tail expression (unique outlet)                                                         | Fewest rules; consistent with Rust                                   | 2026-09-15 |
| No tail expression                   | Doesn't exist (non-empty block always has a tail expression; empty block `{}` → `Void`) | Want `Void`? Write `Void` explicitly                                 | 2026-09-15 |
| Assignment's value                   | `Void`                                                                                  | Assignment is a statement, not value production                      | 2026-09-15 |
| `if` without `else`                  | `Void`                                                                                  | No branch to evaluate when condition is false                        | 2026-09-15 |
| Division of mechanism and protection | Language provides mechanism, type checking provides protection                          | Language doesn't promise to prevent "intent errors"                  | 2026-09-15 |
| Control flow body / evaluation unit  | Don't bifurcate at language level, treat as perspective difference                      | Imperative and declarative coexist, avoid unprincipled special cases | 2026-09-15 |
| Handling of error examples           | Delete, don't keep in erratum block                                                     | Keeping error code that looks usable will mislead                    | 2026-09-15 |

## Appendix C: Glossary

| Term                  | Definition                                                                                                               |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| Tail expression       | The last value-producing expression in a block; the block's value is this one                                            |
| Non-local exit        | A control flow transfer that passes through outer evaluation units and directly affects the function boundary (`return`) |
| Bottom principle      | `Never <: T` holds for any `T`, allowing `Never` to reduce to any type                                                   |
| Value-producing block | `= {}` / `spawn {}` / `unsafe {}`—value outlet is the tail expression                                                    |
| join                  | Multi-branch merge rule; `Never` branches don't participate in merging                                                   |

## Appendix D: Function / Block Value Ambiguity Ruling (Ruling C)

> **Erratum (2026-09-19): Ruling C's unannotated line has been overturned.**
>
> User ruling: **unify to "content determines type" (Reading B)**, with the reason being that the
> same principle should run through dictionaries and blocks:
>
> > Unify to B, `{}` naturally falls into place.
>
> **Current Rule**
>
> | Case                        | Result                    | Basis                                    |
> | --------------------------- | ------------------------- | ---------------------------------------- |
> | `value` is `Lambda` (`=>`)  | Function                  | `=>` is an explicit function constructor |
> | Annotation is `Fn`          | Function                  | Declared function type                   |
> | Annotation is non-`Fn` type | Block value               | Annotation is the type (`x: Int = {..}`) |
> | **No annotation**           | **Inferred from content** | **Type determined by content**           |
>
> Key change: `f = { 5 }` is now the **value** `5` (`Int`), no longer a function.
>
> ```yaoxiang
> x: Int = { y = 5; y }      // Block value: x = 5 (non-Fn annotation)
> f: () -> Int = { 5 }       // Function: f() = 5 (Fn annotation)
> f = { 5 }                  // Value: f = 5 (no annotation → content inference)
> f = {}                     // Value: f = Void (empty block)
> b = () => 5                // Function: explicit lambda
> d = { "a": 1 }             // Value: Dict (content self-describes)
> ```
>
> **Why Ruling C Was Overturned**
>
> Ruling C's original reasoning ("default to function when no annotation") was
> **implementation-convenience-oriented**:
>
> 1. ~~No changes to existing code~~—migration cost is payable (211 files, 704 sites), not a
>    semantic justification
> 2. ~~Function definitions are high-frequency, block values are low-frequency~~—frequency is not a
>    type rule
> 3. ~~RFC-007 has been accepted; changing it is more expensive than changing this RFC~~—changing
>    what's wrong doesn't make it right because it's "expensive"
>
> The real problem is **inconsistency**: `f = { 5 }` is a function, but `d = { "a": 1 }` is a
> dictionary—the same `{` at the same unannotated position yields different categories of things.
> The "cost" Ruling C itself admitted is precisely this crack: "In `f = { 5 }`, `f` is a function,
> not `5`…contrary to the intuition of 'braces mean value'".
>
> The current rule eliminates the crack: **type is determined by content**, regardless of whether an
> annotation is present. Annotation still declares the type (`f: () -> Int`), but does not impose a
> default value because the "annotation is missing."
>
> **Where `{}` Falls**
>
> After unifying to B, `{}` naturally falls into place: `Dict` syntax requires at least one key,
> `{}` has no content to rely on, so it takes the zero form of block structure → empty block, value
> `Void`. For empty dictionary, use `dict.new()`. See spec
> [§2.9.1](../.../reference/language-spec/syntax.md) for details.
>
> **Unchanged Parts**
>
> The two subsequent points in Appendix D ("annotation must not determine value taking,"
> "`callable_parts()` no longer unconditionally swallows blocks") remain valid; the latter is now
> implemented by `Expr::block_binding_is_function`, except its no-annotation branch has changed with
> the table above.

### Problem (Historical Background)

`name = { ... }` is simultaneously defined as different things by two accepted RFCs:

| RFC                       | Definition                                                                     |
| ------------------------- | ------------------------------------------------------------------------------ |
| RFC-007 (Function Syntax) | Function. "Zero-arg simplest" `name = { return ... }`                          |
| RFC-010 / 010a            | Block value. "`= {}` is a value-producing block, value is the tail expression" |

The same syntactic position, two sets of semantics. At the implementation level, this caused
`callable_parts()` to register a block as a 0-arg function, while `generate_block_ir` again took the
block value—two layers of understanding inconsistent.

### Original Ruling: Annotation Priority, Default Function (**Deprecated**)

| Case                        | Result          | Basis                                    |
| --------------------------- | --------------- | ---------------------------------------- |
| `value` is `Lambda` (`=>`)  | Function        | `=>` is an explicit function constructor |
| Annotation is `Fn`          | Function        | Declared function type                   |
| Annotation is non-`Fn` type | **Block value** | Annotation is the type (`x: Int = {..}`) |
| No annotation               | **Function**    | Default; RFC-007 "zero-arg simplest"     |

**Key: block value semantics does not rely on new syntax.** To evaluate `{ ... }` on the spot, write
the target type:

```yaoxiang
x: Int = { y = 5; y }      # Block value: x = 5 (non-Fn annotation)
f: () -> Int = { 5 }       # Function: f() = 5 (Fn annotation)
f = { 5 }                  # Function: f() = 5 (no annotation, default function)  # ← this line deprecated
b = () => 5                # Function: expression-body lambda
```

### Original Reason (**No Longer Valid**)

Annotation **already declares the type**: `x: Int = ...` says `x` is `Int`, then `{...}` is an `Int`
value; `f: () -> Int = ...` says `f` is a function, then `{...}` is the function body.
**Type-oriented, not inventing a new rule.**

Default to function when no annotation, because:

1. No changes to existing code (203 `name = { ` bindings without annotation in the repo, all of
   which are `main`)
2. Function definitions are high-frequency, block values are low-frequency
3. RFC-007 has been accepted; changing it is more expensive than changing this RFC

### Cost (Known and Accepted)

In `f = { 5 }`, `f` is a **function** rather than `5`. This is contrary to the intuition that
"braces mean value"—but that's the definition already accepted in RFC-007, and **the annotation is
the escape hatch**: write `f: Int = { 5 }` to get the value.

### Implementation Defects Also Corrected by This Ruling

1. **Annotation must not determine value taking** (root cause one): three places including
   `generate_function_ir` used `return_type != Void` as the threshold for value taking of the tail
   expression, causing the unannotated `f = { 5 }`'s tail expression to be silently discarded,
   returning `Void`. Annotation only determines whether to _check_, not whether to _evaluate_.
2. **`callable_parts()` no longer unconditionally swallows blocks**: dispatching is unified by
   `Expr::block_binding_is_function(annotation, value)`.

## References

- [RFC-007: Unified Function Definition Syntax](./007-function-syntax-unification.md)
- [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)
- [RFC-038: Statement Termination and Newline Rules](038-statement-termination.md)
- [RFC-030: assert Mechanism](./030-assert-mechanism.md) — `Never`'s refinement type application
- [Language Spec §Type System](/reference/language-spec/type-system.md) — `Never` / `Void`'s ⊥ / ⊤
  positioning
- [Rust Reference: `!` never type](https://doc.rust-lang.org/reference/types/never.html) —
  isomorphic bottom principle application
