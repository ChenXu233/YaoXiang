---
title: 'RFC-010b: Pattern Matching Completeness (Variant Destructuring and Exhaustiveness)'
status: 'accepted'
author: 'Chenxu'
created: '2026-09-03'
updated: '2026-11-17'
issue: '#330'
issues_impl:
  - '#330'
  - '#389'
parent: 'RFC-010'
---

# RFC-010b: Pattern Matching Completeness (Variant Destructuring and Exhaustiveness)

> This document is a sub-RFC of [RFC-010](010-unified-type-syntax.md) (Unified Type Syntax). Variant
> declarations, constructor call forms, and the runtime tagged union representation (including the
> `variant_id` declaration order—the input variant set for this document's exhaustiveness checks)
> are defined in RFC-010's “Record-Style and Typed Variant Construction (Authoritative Definition)”
> section. This document covers the mirrored side: match destructuring and exhaustiveness checking.
> Construction and destructuring must be paired, so both belong to RFC-010.
>
> Originally numbered RFC-039 (drafted on 2026-09-03); on 2026-09-25, by decision of the owner, it
> was merged into RFC-010 as a sub-RFC and renumbered RFC-010b, while its tracking issue remains
> #330. The main text was expanded on 2026-09-27 and landed with the implementation (head e368958f);
> on the same day, the owner confirmed its formal acceptance (accepted).

## Summary

Complete the pattern matching capabilities of `match` from “literals only + wildcard” to a
full-featured facility: variant destructuring (including zero-payload and multi-payload variants),
struct/tuple patterns, or-patterns and guards, finalized identifier-binding semantics, and
exhaustiveness checking (turning E1030/E1031 from codes registered without emission points into
diagnostics that are actually emitted). This RFC is a prerequisite for the
`Error { kind: ErrorKind, message }` evolution roadmap (the evolution section of RFC-013, “Runtime
Error Values and Code Propagation”), but its motivation is independent: pattern matching is a
general-purpose language capability that serves every type with variants.

## Motivation

### Empirical Status (2026-09-03, v0.7.12 code; fully resolved on 2026-09-27)

1. **Only literal patterns work at the IR layer**. The AST already defines the complete pattern set,
   but IR generation implements only the `Literal` branch. All other patterns fall into a stub that
   loads the constant 0 for equality comparison and therefore never matches; moreover, it
   incorrectly matches the stub arm when the scrutinee is exactly 0.
2. **The `match ok(v)/err(e)` example in stdlib.md exists only on paper**.
3. **Exhaustiveness checking is hollow**. E1030/E1031 are registered in the code table but have no
   emission points.
4. **Error-handling evolution is blocked** (RFC-013's structured error model depends on variant
   destructuring).

### Design Goals

- **Variant destructuring is available**: `match r { ok(v) => ..., err(e) => ... }` and
  destructuring of user-defined variant sets have consistent semantics across all execution paths.
- **Exhaustiveness can be relied upon**: E1030/E1031 are actually emitted, and a match with a
  missing branch is a compile-time error.
- **Stubs are eliminated**: unsupported patterns produce a compile-time error instead of silently
  taking the wrong path (the #330 safety net—this transitional strategy is progressively replaced
  with real encodings during implementation, ultimately eliminating all stubs).

## Proposal (Finalized and Implemented on 2026-09-27)

| Capability                             | Status | Description                                                                                                         |
| -------------------------------------- | ------ | ------------------------------------------------------------------------------------------------------------------- |
| Union variant patterns                 | ✅     | `Variant` / `Variant(binding)` / multi-payload `make(a, b)` + payload bindings introduced into the arm's scope      |
| Struct / Tuple patterns                | ✅     | Field patterns (shorthand binding / `name: subpattern`) and tuple patterns; declared fields must be covered in full |
| Or patterns and guards                 | ✅     | `p1 \| p2` and `pat if cond`: alternative-level failure chains; a failed guard falls through to the next arm        |
| Exhaustiveness checking                | ✅     | E1030 for missing branches and E1031 for unreachable arms; coverage spans the variant set                           |
| Identifier pattern semantics confirmed | ✅     | Finalized as **binding** (see below)                                                                                |

## Design Decisions

### Identifier Patterns: Binding, Not Assignment or Comparison

In `match x { y => ... }`, the bare identifier `y` is finalized as a **binding**: it introduces a
new name that captures the matched value and always matches (equivalent to a named wildcard). This
decision has three foundations:

1. **Structural argument**: an identifier in a pattern position naturally has two roles—the
   discriminant position (the `ok` in `ok(v)`, compared against the variant table) and the leaf
   position (the `v` in `ok(v)`, which binds). Parameter lists (lambdas) have only leaf positions
   and are therefore unambiguous. A bare identifier both occupies the discriminant position and
   resembles a leaf, which is where the ambiguity arises. YaoXiang has a static variant table, so
   discriminant names are resolved by table lookup (if the pattern head is a name or literal in the
   scrutinee's variant set, it is a discriminant; if a bare name is absent from the table, it is a
   binding). No Rust-style capitalization convention is required.
2. **Degeneracy of value comparison**: the rule that “an already-bound name in a pattern means
   equality” (unification semantics) is an artifact of the single-assignment tradition. In a mutable
   language, it is impossible to statically determine whether a name “has been bound,” so this
   semantics does not apply. YaoXiang variables can be reassigned (`mut`), following Rust's rule.
3. **The need to match an existing value belongs in guards**: `n if n == threshold` (use an if-chain
   until guards are implemented).

**A binding is not an assignment** (the position agreed with the owner on 2026-09-27): a pattern
binding introduces a **new name** whose scope is the arm; it shadows an outer variable with the same
name and expires when the arm is left (probe: an outer `v = 999` remains unchanged after `some(v)`
matches). Assignment rewrites an **existing** name (the E2010/`mut` rules). Both concepts coexist in
this language and remain distinct: `=` is routed according to whether the name already exists (spec
§4.3 gives assignment precedence), while a pattern position always belongs to the binding side.

### `match` Is an Expression, Not an Assignment Statement

`match` produces the value of the arm that matched (the value of the entire expression);
`v = match ...` is merely ordinary binding syntax that captures it. The strictness of exhaustiveness
checking comes from the requirement that every arm must produce a value: a missing shape means that
the receiver cannot obtain a value, which is a compile-time error (E1030). A switch used in
statement form can only silently ignore missing branches.

### Arm Semantics and Implementation

An arm is a **name-tagged single-argument function**: `some(v) => body` ≈ `λv.body`, and it is
invoked only when the argument has the `some` shape. Its binding semantics follow from the left-hand
side being in a parameter position. It shares the same `=>` notation as lambdas. In the
implementation, an arm is **not a closure**: the IR's recursive pattern compiler (`compile_pattern`)
inlines and emits a test chain (VariantTag/LoadField/Eq + JmpIfNot) and payload extraction
(VariantPayload). A match proceeds directly to the arm body, while a failure jumps to the next
arm—there is no call overhead.

### Or Patterns and Guards

- `p1 | p2`: alternatives are attempted in order. If a non-final alternative fails, the next
  alternative is tried; if one matches, its payload is extracted and control proceeds directly to
  the arm body. If the final alternative fails, control proceeds to the next arm. **All alternatives
  must have identical binding-name sets** (E1033). Bindings with the same name share an arm-level
  preallocated register, ensuring that the arm body reads the value written by the alternative that
  matched.
- `pat if cond`: after the pattern matches, `cond` is evaluated (**pattern bindings are visible to
  the guard**). If it evaluates to false, control proceeds to the next arm. A guard condition must
  produce a `Bool`. **Arms with guards do not contribute to exhaustiveness coverage** (a guard may
  fail to match, as in Rust), but a duplicate variant arm with a guard remains reachable as a
  refinement of an earlier arm.

### Struct Patterns

In `Point { x, y: b }`, field shorthand binds the field under the same name, while
`name: subpattern` applies recursively. **All declared fields must be covered** (E1034 for missing
fields), unknown fields are rejected (E1042), and the same field appearing more than once produces
E1032. Field omission with `..` and generic struct patterns are not yet supported (see Open
Questions).

### Variant Destructuring (Symmetric with RFC-010 Construction)

| Construction                       | Destructuring      | Payload representation                                                                                 |
| ---------------------------------- | ------------------ | ------------------------------------------------------------------------------------------------------ |
| `Result(Int, String).ok(5)`        | `ok(v)`            | Single payload: direct binding                                                                         |
| `Pair.make(3, 4)`                  | `make(a, b)`       | Multiple payloads: packed into a Tuple and unpacked positionally                                       |
| `Color.red()` / `Option(T).none()` | `red()` / `none()` | Zero payload: `pattern: None`, with arity validation (`ok()` on a payload-bearing variant is an error) |

Payload type = the variant parameters after substitution with the scrutinee's type arguments (in
declaration order). Nested patterns (`ok(some(v))` across sum types, `Point { pos: Tuple(a, b) }`
across structs) recurse by subtype.

### Exhaustiveness and Reachability

- **E1030 (missing branch, fully exhaustive)**: coverage is checked for a scrutinee of **every
  type** whenever there is no fallback arm. For sum types, the variant set must be fully covered by
  unguarded union arms (guarded arms do not count because a guard may fail to match). For non-sum
  types, the value domain is open (Int/String/Char, etc.), so literal arms cannot be exhaustive and
  **a fallback arm is required**. The enumerable exceptions are Bool, for which true and false
  literal arms are exhaustive, and Void, which has no inhabiting values and is therefore vacuously
  exhaustive. (Decision by the owner on 2026-09-27: a match without a fallback arm implicitly
  assumes that “no other values will occur”—such assumptions should not be implicit where they are
  not warranted; every assumption must be written as `_`.)
- **E1031 (unreachable)**: any arm after a fallback arm (an unguarded irrefutable pattern:
  Identifier/Wildcard/Tuple/Struct), an unguarded duplicate variant arm, or a duplicate literal arm.

### Diagnostics

| Code  | Meaning                                                                                    |
| ----- | ------------------------------------------------------------------------------------------ |
| E1030 | Non-exhaustive pattern match (lists the missing variants)                                  |
| E1031 | Unreachable pattern (after a fallback arm / duplicate variant / duplicate literal)         |
| E1032 | Duplicate pattern binding (the same binding name appears more than once in one pattern)    |
| E1033 | Inconsistent bindings in an or-pattern (the alternatives have different binding-name sets) |
| E1034 | Struct pattern is missing fields                                                           |
| E3008 | IR-layer safety net retained (a defensive branch that is unreachable in normal programs)   |

### Boundaries with Assignment/Lambdas (Memo to Prevent Confusion)

- Pattern binding ≠ assignment: it introduces a new name rather than rewriting an existing name
  (E2010/`mut`);
- An or-pattern ≈ an extension of lambda application (semantic model), but it is implemented as
  inline jumps rather than a closure call;
- A literal pattern compiles to an Eq + conditional jump chain—the intuition of an “optimized form
  of if/elif” applies to literals, while what `match` adds is shape discrimination and destructuring
  bindings.

## Implementation Points

- parser: `parse_pattern` (variant-call restructuring / struct pattern `Ident { .. }` branch /
  nested or-pattern reconstruction), `parse_match` or-pattern chain closure and guards (the
  `no_fat_arrow` flag prevents `=>` from acting as an infix lambda operator)
- typecheck: `check_match_expr` unified arm loop + `check_pattern` recursive checker (binding
  registration, guard Bool validation, and exhaustiveness accounting)
- ir_gen: `compile_pattern` recursive pattern compiler + `sum_type_variants` mirror carrying payload
  types and `sum_type_param_names` (payload types are substituted according to the scrutinee's type
  arguments)
- corpus: `02-type-system/aggregates/pattern_destructure.yx`, `pattern_or_guard.yx`; unit tests for
  E1030/E1031/E1032/E1033/E1034 (rfc011b.rs)

## Open Questions

- [x] Identifier patterns: bind a new variable or match an existing value? → **Binding** (the
      discriminant position is resolved by variant-table lookup, with no capitalization convention);
      matching an existing value belongs in a guard.
- [x] Scope of exhaustiveness checking: → **Fully exhaustive** (decision by the owner on
      2026-09-27)—non-sum types likewise produce E1030 without a fallback arm; the only enumerable
      exceptions are Bool with two literal arms and Void.
- [x] Should the transition period before fixing the stub's load-0 mismatch first produce a
      compile-time error? → Yes (the #330 safety net); all such cases have been replaced with real
      encodings.
- [ ] Is struct-pattern `..` field omission (partial fields + fallback) needed? (This affects the
      field-coverage rules.)
- [ ] Generic struct patterns (e.g., `Box { v, n }` on a `Box(Int)` value) remain undecided—they are
      currently rejected by a typecheck precondition.
- [ ] A `non_exhaustive`-style attribute (the blast radius of extending the std variant set for user
      matches) remains undecided.

## Related RFCs

- RFC-013, “Runtime Error Values and Code Propagation,” evolution section (`Error { kind, message }`
  depends on this RFC)
- RFC-036, Test Model (how to write matches when asserting error branches in tests)
- RFC-011b Stage 2 (making `?` interface-based and standardizing Result/Option in std: this RFC's
  variant-set exhaustiveness follows the `sum_types` mirror and does not depend on type ownership)
