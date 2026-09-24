---
title: 'RFC-010a: Tail Expression Evaluation and return Semantics'
status: 'Accepted'
author: 'MorningX'
created: '2026-09-15'
updated: '2026-09-15 (Accepted)'
group: 'rfc-010'
issue: '#342'
---

# RFC-010a: Tail Expression Evaluation and return Semantics

> **References**:
>
> - [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md) — `name: type = value` model, `{}`
>   as dependency-driven computation unit
> - [RFC-007: Function Definition Syntax Unification](./007-function-syntax-unification.md) — block return rules, early return
> - [RFC-038: Statement Termination and Newline Rules](038-statement-termination.md) — newline behavior for statements and expressions
> - [Language Spec §Type System](../../../../reference/language-spec/type-system.md) — `Never` explosion principle

## Summary

Unify the semantics of `return` and block evaluation, eliminating the discrepancies between RFC-007 and RFC-010.

Propose **three self-consistent rules**: the value of a block equals its tail expression (single exit point); `return`
is a **non-local exit** of type `Never` (exits the function, does not "return to the block"); `if` without `else` evaluates to `Void`.

`return` and block evaluation **are not bifurcated at the language level** — `{ return n }` as a block has value `n` (type `Never`), while
`return`'s effect is to exit the function. The two coexist through the **explosion principle** (`Never <: T`). RFC-007's "early return" and RFC-010's "blocks have values" are corollaries of these three rules, not conflicting special cases.

No new syntax, no new keywords.

## Implementation Status

This RFC has been accepted. The implementation status of the three rules:

| Rule                       | Sub-item                               | Status                                |
| -------------------------- | -------------------------------------- | ------------------------------------- |
| ① Block value = tail expr  | Function body tail expression          | ✅ Implemented                        |
| ① Block value = tail expr  | Tail-position `if`/`match`            | ✅ Implemented (#344)                 |
| ① Block value = tail expr  | Last statement assignment → `Void`     | ✅ Implemented                        |
| ① Block value = tail expr  | Empty block `{}` → `Void`             | ✅ Implemented                        |
| ① Protection via type check | Tail expression matches declared return | ✅ Implemented (#345)                 |
| ② `return` non-local exit  | Exits `if`/`while`/`for`/bare/nested   | ✅ Implemented                        |
| ② `Never <: T` explosion   | `unify` and `is_subtype` consistency   | ✅ Implemented (internal fix)         |
| ③ `if` no `else` → `Void`  | No branch value leaking at expr position | ✅ Implemented (#346)                |
| ① Block value = tail expr  | Bare block value binding `x = { ... }` | ✅ Implemented (#343)                 |
| ① Block value = tail expr  | `unsafe {}` value exit                 | ✅ Implemented (#347)                 |
| ① Protection via type check | Empty block / trailing statement escape check | ✅ Implemented (#342 Issue 5)      |
| ① Block value = tail expr  | `spawn {}` value exit (tail expression) | ✅ Implemented (#365)                 |

Corpus coverage: `tests/yaoxiang/03-semantics/rfc010a_block_value.yx` (positive) +
`tests/yaoxiang/06-compile-errors/tail_expr_type_mismatch{,_with_stmts}_err.yx` (negative).

## Motivation

### Semantic Discrepancy

The playground Fibonacci example exposed a semantic divergence:

```yaoxiang
fib: (n: Int) -> Int = {
  if n <= 1 {
    return n          // Is this return "if's" or "function's"?
  }
  return fib(n - 1) + fib(n - 2)
}
```

Two accepted RFCs give **opposite** derivations:

| RFC                   | Statement                                                                              | Derived semantics                         |
| --------------------- | -------------------------------------------------------------------------------------- | ----------------------------------------- |
| **RFC-007** (accepted) | Using `factorial` as example, title says **"Early Return: Using return"**: `if n <= 1 { return 1 }` | `return` exits both `if` and function body, **returns to function** |
| **RFC-010** (accepted) | "`{}` is a dependency-driven computation unit... use `return` to explicitly return value"; `spawn { return c }` returns task result | `return` gives value **to this `{}`**    |

According to RFC-010, `if n <= 1 { return 1 }`'s braces are a computation unit, `return 1` gives it value 1, so the `if`
statement's value is discarded and the next line must execute — **fib infinite recursion**. RFC-007 gives correct behavior.

### Root Cause: `return` Serves Two Roles

- **RFC-007 uses it to mean "exit function"** — control flow concept
- **RFC-010 uses it to mean "this block's value is this"** — evaluation concept

Using a control flow keyword for evaluation is a category error. One word carrying two roles means one side is always compromised.

### Downstream Documentation's Expanded Interpretation

`docs/src/reference/language-spec/syntax.md` extends RFC-010's "`{}` block" to **all braces**:

- §2.9: "`return` in `{}` **always returns content to the enclosing scope**" (calling it "unified semantics: all `{}` blocks")
- §3.3: "`return` is used to **return a value from a code block**"

But RFC-010 defines only **three kinds of value-bearing blocks** (`= {}` / `spawn {}` / `unsafe {}`), **never mentioning the braces of `if` /
`while` / `for` / `match`**.

### Implementation Status

| Behavior                                  | Current Status                       |
| ----------------------------------------- | ------------------------------------ |
| `if n == 0 { return 7 } … return 8`       | Function exits (`h(0) = 7`)          |
| `f = { n + 1 }`                           | Tail expression usable (returns `5`) |
| `x = { y = 5; y }` (bare block tail expr) | `E3006` variable unresolved (see #343) |
| `f: () -> Int = { if c {5} else {6} }`   | Returns `void`, tail expression dropped (#344) |
| `f: () -> Int = { "s" }`                  | Silently passes compilation (see #345) |
| `x = if c { 19 }` (no `else`)             | `19` (should be `Void`, see #346)    |
| `v = unsafe { 42 }`                       | `void` (see #347)                    |
| `y = if c { 111 } else { 222 }`           | Usable (`if` as expression)          |

`tests/yaoxiang/03-semantics/no_tail_expr_return.yx`
claimed "tail expressions no longer implicitly return" but didn't cover that case, so the test didn't fail. The file has been replaced with
`tests/yaoxiang/03-semantics/tail_expr_and_return.yx`.

## Proposal

### Three Rules

```
① Block value = tail expression (single exit point)
   Assignment statement's value is Void; empty block {} value is Void
② return : (T) -> Never
   Non-local exit: exits the nearest function boundary
③ if without else → Void
```

**These three are sufficient to derive "early return"; no additional rule about `return` specifically targeting functions is needed.**

### Rule ①: Block Value

A block's **last statement/expression** is the block's value (tail expression). This **is not** "no tail expression means Void" — non-empty blocks always have a tail expression (the last statement itself is it), and an empty block
`{}` has value `Void`.

```yaoxiang
// Tail expression determines block value
a = {
    x = compute()        // Assignment statement → Void
    x * 2                // Tail expression → block value
}

// Assignment as tail expression → block value is Void
b = {
    x = compute()
    log(x)               // Assignment statement → Void
}

// Explicit Void when you want it
c = {
    log(x)
    Void                 // Explicit Void
}
```

**Design Criterion (mechanism and protection separated)**:

- **Language rules only provide mechanism**: last statement is block value, rules are unique and unambiguous
- **Protection provided by type checking**: function declaration `-> Int` but tail expression type mismatch → compile error
- **Language does not prevent "intent errors"**: if the tail expression type happens to match the return type but the semantics are unintended, that's author oversight; the language cannot judge intent. **Language rules don't protect against intent errors.**
- **Explicitly write `Void` if you don't want to return**

This replaces RFC-010's "`= { ... }` must use `return`, otherwise returns `Void`", and its design rationale "need explicit `return` to eliminate ambiguity about whether the last expression is a return value".

### Rule ②: `return` is Non-local Exit

`return` has type `Never` (zero constructors, no value can inhabit it). Its semantics:

- **Exits the nearest function boundary**, passing the value to the caller
- **Pierces through all blocks** — (if present) `if` / `while` / `for` / `match` / bare blocks / `spawn` / `unsafe`

`return` does not "return to the block". `{ return n }` as a block has value `n` (tail expression rule), **type `Never`**; simultaneously,
`return`'s effect is to exit the function. **Both hold simultaneously.**

### Rule ③: `if` without `else`

```yaoxiang
x = if c { 19 }        // No else
```

When the condition is false, no branch can be evaluated, so `Void` is taken. Hence this `if`'s value type is `Void` (or cannot be used in non-`Void` positions).

When a value is needed, explicitly complete both branches:

```yaoxiang
x = if c { 19 } else { 20 }
```

### `Never` is the Technical Foundation for Coexistence

`Never <: T` holds for any type `T` (explosion principle, see Language Spec §Type System). Therefore:

```yaoxiang
fib: (n: Int) -> Int = {
  if n <= 1 {
    return n              // Tail expression: Never
  }                       // → This if's value: Never
  fib(n - 1) + fib(n - 2) // → Block value: Int
}
```

- The branch body `{ return n }` has value `Never`
- The `if`'s only branch is `Never` ⇒ the `if`'s value is `Never`
- A statement of type `Never` means **sequence terminates here** — the next line is not "the next to execute sequentially"
- And `Never` can reduce to any type (explosion principle), so the whole block satisfies `-> Int`

**"Early return" naturally follows from this**: `Never` terminates the sequence + explosion principle allows reduction.

## Detailed Design

### Formalization of Blocks and Tail Expressions

```
Block        ::= '{' Stmt* '}'                     // Empty block → Void
               | '{' Stmt* Expr '}'                // Value = Expr
Expr         ::= ...
               | Return                            // Type Never
Stmt         ::= Assignment | ExprStmt | ...

value(Block):
  Empty block               → Void
  { ...; e }                → type(e)
  { ...; s } (s is statement) → Void          // Assignment's value is Void
```

### `return` Type Rules

```
return e : Never        where e : T

// Because Never <: T' holds for any T', return can appear in any return type position
```

**No additional rules constraining `return` legality are needed** — the explosion principle covers it. This makes the question "can `return` appear in a function returning `X`" disappear.

### Multi-branch Merging

```
join(A, B):
  If A: Never → B
  If B: Never → A
  Otherwise → A and B must be compatible (same type or have a common supertype)
```

Multi-branches of `if` / `match` merge according to `join`. Branches with `return` don't participate in merging due to their `Never` type.

### Consistency with RFC-007

RFC-007's examples **fully comply with this RFC**, no revisions needed:

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }          // Never branch, ignored by join
    return n * factorial(n - 1)     // Function exit
}
```

"Early return" is not a special case; it's a **corollary** of rules ①, ② and the explosion principle.

### Differences from RFC-010 and Revision Requirements

RFC-010's definitions have been revised per this RFC, differences as follows:

| RFC-010 Original Provision                            | Current Definition                                   |
| ----------------------------------------------------- | ---------------------------------------------------- |
| "`= { ... }` must use `return`, otherwise returns `Void`"` | Block value = tail expression; empty block → `Void`  |
| Design rationale "need explicit `return` to eliminate tail expression ambiguity" | Invalid — tail expressions don't create ambiguity, `return` doesn't interfere |
| Three `return c` / `return SqliteDb` examples        | Tail expression syntax (value exits for `spawn` / `unsafe` unified) |

**RFC-010's core design (`{}` as dependency-driven computation unit) remains unchanged**; only the value exit changed from `return` to tail expression.

### Unified Perspective (Design Principle)

The braces of `if` / `while`
**serve both as imperative control bodies and declarative evaluation units** — this is **a difference in perspective, not two different language constructs**. Therefore:

- **Do not bifurcate** "control flow body" vs "evaluation unit" at the language level
- All blocks share the same evaluation rules (rule ①)
- `return` is the sole exceptional mechanism, and its exceptionality derives from `Never`'s type-theoretic properties, not syntactic special-casing

This allows declarative and Python-style syntax to coexist naturally:

```yaoxiang
a = if input > 10 { 19 } else { 20 }
b = { x = f(); x * 2 }
c = spawn { fetch("a") }
```

## Trade-offs

### Advantages

- **Eliminates RFC conflict**: RFC-007 and RFC-010 transition from contradiction to corollary relationship, no sacrifice required on either side
- **Minimum rules**: Three rules + one type-theoretic property (explosion principle), no special cases
- **Unique `return` meaning**: Exit function. No longer need to judge ownership between "block value / function value"
- **Unified perspective**: Imperative and declarative coexist without language-level bifurcation
- **Consistent with mature languages**: Rust similarly coexists with "tail expression + `return : !`"
- **Zero new syntax**: No keywords added, parser syntax rules unchanged

### Disadvantages

- **"Accidental return" risk with last expression as value**: Forgetting to delete the last line silently changes the return value
  - Mitigation: Type checking catches type mismatches; language doesn't promise to prevent intent errors (see design criterion)
- **Downstream documentation needs syncing**: Accepted document examples need rewriting
- **Interaction with statement termination needs clarification**: Whether a newline-terminated last expression is still the block value (see open issues)

## Alternative Approaches

### Option A: `return` keeps dual role, dispatch by block type

In function body `= {}`, `return` belongs to function; in `spawn {}` / `unsafe {}`, belongs to block; in `if {}` / bare blocks, belongs to?

**Rejection reason**: `if`'s ownership cannot be adjudicated — this is the original conflict. And users would need to memorize "which block belongs to whom", with no principled basis (why should `if` belong to function but bare blocks belong to themselves?).

### Option B: Strict block return (`return` always belongs to current block)

**Rejection reason**: **Feature missing**. `return` can never early-exit a function from nested blocks; guard clauses (`if err { return }`) become completely unwritable, forcing functions to be written as nested expressions.

### Option C: Block value with new keyword (`give x` / `yield x`)

**Rejection reason**: Violates RFC-036's **zero syntax change** principle (new keywords required), and users would learn two concepts (`return` for exit + `give` for evaluation). Tail expression approach introduces zero new concepts.

### Option D: Keep RFC-010 as-is (must use explicit `return`)

**Rejection reason**: Conflict with RFC-007 is irreconcilable (see motivation). And implementation has already moved toward tail expression path.

## Downstream Documentation Revisions

This RFC's definitions have become the authoritative semantics for downstream documentation; all related documents have been synced (obsolete statements no longer retained).

## Open Issues

- [x] Interaction between tail expression and RFC-038 statement termination rules: is a newline-terminated last expression still the block value? (@MorningX: needs confirmation together with RFC-038's "line-start `(`/`[` never joins" rules) —实测：换行终止的末位表达式（含行首 `(` / `[` / 列表字面量）均为块值
- [x] When is `name = { ... }` a function vs block value binding — see Appendix D (content determines type)
- [x] Specific shape after rewriting `unsafe {}` / `spawn {}` — tail expressions for both have been tested
- [x] Interaction between `match` branch `join` and exhaustiveness checking (depends on RFC-039) —实测：`Never` 分支不参与合并，多分支 `if` / `match` 的 join 行为正确
- [x] Diagnostic message when empty block `{}` is function body and return type is non-`Void` — reuse existing `E1012`, point to annotation

---

## Appendix A: Empirical Evidence

All reproduced on 0.8.0.

| Code                                               | Observed Behavior              |
| -------------------------------------------------- | ------------------------------ |
| `h: (n)->Int = { if n==0 { return 7 } return 8 }` | `h(0)=7`, `h(1)=8`             |
| `f: (n)->Int = { n + 1 }`                          | `5` (tail expression usable)   |
| `f: ()->Int = { if c {5} else {6} }`              | `void` (should be `5`, see #344) |
| `f: ()->Int = if c {5} else {6}`                   | `5`                            |
| `f: ()->Int = { match ... }`                       | Correct                        |
| `f: ()->Int = { while ...; i }`                    | Correct                        |
| `{ y = 5; y }` as binding expression               | `E3006` (see #343)             |
| `f: ()->Int = { "s" }`                             | Silently passes (see #345)     |
| `x = if c { 19 }` (no `else`)                     | `19` (should be `Void`, see #346) |
| `v = unsafe { 42 }`                                | `void` (see #347)              |
| `y = if c { 111 } else { 222 }`                    | `111`                          |
| `while { if i==2 { return 42 } }`                  | `42` (exits loop)              |
| Nested `{ { return 5 } return 1 }`                 | `5` (exits bare block)         |

## Appendix B: Design Decision Log

| Decision                      | Decision                                             | Rationale                              | Date        |
| ----------------------------- | ---------------------------------------------------- | -------------------------------------- | ----------- |
| `return` semantics            | Function exit, type `Never`; does not return to block | Eliminates category error of one word serving two roles | 2026-09-15 |
| Block value exit              | Tail expression (single exit point)                  | Minimum rules; consistent with Rust    | 2026-09-15 |
| No tail expression            | Does not exist (non-empty block always has tail expr; empty `{}` → `Void`) | Explicitly write `Void` when you want it | 2026-09-15 |
| Assignment value              | `Void`                                               | Assignment is a statement, not value-producing | 2026-09-15 |
| `if` without `else`           | `Void`                                               | No branch evaluable when condition is false | 2026-09-15 |
| Mechanism vs protection split  | Language provides mechanism, type checking provides protection | Language doesn't promise to prevent "intent errors" | 2026-09-15 |
| Control flow body / eval unit | Not bifurcated at language level, treated as perspective difference | Imperative and declarative coexist, avoids unprincipled special cases | 2026-09-15 |
| Handling of error examples    | Directly deleted, not kept with error code           | Keeping seemingly usable erroneous code misleads | 2026-09-15 |

## Appendix C: Glossary

| Term              | Definition                                                                                              |
| ----------------- | ------------------------------------------------------------------------------------------------------- |
| Tail expression   | The last value-producing expression in a block; the block's value is this expression                  |
| Non-local exit    | Control flow transfer that crosses outer evaluation units and directly acts on function boundaries (`return`) |
| Explosion principle | `Never <: T` holds for any `T`, allowing `Never` to reduce to any type                                 |
| Value-bearing block | `= {}` / `spawn {}` / `unsafe {}` — value exit is tail expression                                       |
| join              | Multi-branch merge rule; `Never` branches don't participate in merging                                  |

## Appendix D: Function / Block Value Ambiguity Resolution

### Problem

`name = { ... }` was defined differently by two accepted RFCs: RFC-007 (function syntax) treated it as a function
("simplest no-argument form" `name = { return ... }`), while RFC-010 / 010a treated it as a block value (`= {}` is a value-bearing block,
value is tail expression). Same syntactic position, two sets of semantics; implementation had `callable_parts()` registering blocks as 0-arg functions while `generate_block_ir` evaluated them as block values — two layers of understanding inconsistent.

### Resolution: Content Determines Type

The same principle should apply consistently to dict literals and blocks:

| Situation                    | Result      | Basis                              |
| ---------------------------- | ----------- | ---------------------------------- |
| `value` is `Lambda` (`=>`)  | Function    | `=>` is explicit function constructor |
| Annotation is `Fn`           | Function    | Function type declared             |
| Annotation is non-`Fn` type  | Block value | Annotation is the type (`x: Int = {..}`) |
| **No annotation**           | **Inferred from content** | **Type determined by content** |

```yaoxiang
x: Int = { y = 5; y }      // Block value: x = 5 (annotation is non-Fn)
f: () -> Int = { 5 }       // Function: f() = 5 (annotation is Fn)
f = { 5 }                  // Value: f = 5 (no annotation → content inference)
f = {}                     // Value: f = Void (empty block)
b = () => 5                // Function: explicit lambda
d = { "a": 1 }             // Value: Dict (content self-describes)
```

**Why not "no annotation defaults to function"**: That would make `f = { 5 }` a function while `d = { "a": 1 }` a dict —
the same `{` in the same no-annotation position yields different kinds of things. Three once-considered rationales are all invalid:

1. Zero changes to existing code — migration cost is payable (211 files, 704 locations), not a semantic rationale
2. Function definitions are high-frequency, block values are low-frequency — frequency is not a type rule
3. RFC-007 is accepted; changing it costs more than changing this RFC — wrong things don't become right because changing them is expensive

**Type is determined by content**, not by presence or absence of annotation. Annotations still declare types (`f: () -> Int`),
but don't impose a default just because "annotation is absent".

### Placement of `{}`

Dict grammar requires at least one key; `{}` has no content to go by, so block structure's zero form is taken → empty block, value `Void`.
Empty dict uses `dict.new()`. See spec [§2.9.1](../../../reference/language-spec/syntax.md).

### Accompanying Implementation Fixes

1. **Annotations must not determine whether to evaluate**: Three places like `generate_function_ir` previously used `return_type != Void`
   as threshold for whether to evaluate tail expression, causing unannotated `f = { 5 }` to silently drop the tail expression and return
   `Void`. Annotations only determine whether to *check*, not whether to *evaluate*.
2. **`callable_parts()` no longer unconditionally swallows blocks**: Dispatch is now unified via `Expr::block_binding_is_function(annotation, value)`.

## References

- [RFC-007: Function Definition Syntax Unification](./007-function-syntax-unification.md)
- [RFC-010: Unified Type Syntax](./010-unified-type-syntax.md)
- [RFC-038: Statement Termination and Newline Rules](038-statement-termination.md)
- [RFC-030: assert Mechanism](./030-assert-mechanism.md) — `Never` refined type application
- [Language Spec §Type System](../../../../reference/language-spec/type-system.md) — `Never` / `Void` ⊥ / ⊤ positioning
- [Rust Reference: `!` never type](https://doc.rust-lang.org/reference/types/never.html)
  — Isomorphic explosion principle application