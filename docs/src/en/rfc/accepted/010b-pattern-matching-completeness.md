---
title: 'RFC-010b: Pattern Matching Completeness (Variant Destructuring and Exhaustiveness)'
status: 'Accepted'
author: 'Chenxu'
created: '2026-09-03'
updated: '2026-09-27'
issue: '#330'
parent: 'RFC-010'
---

# RFC-010b: Pattern Matching Completeness (Variant Destructuring and Exhaustiveness)

> This document is a sub-RFC of [RFC-010](010-unified-type-syntax.md) (Unified Type Syntax). Variant
> declaration, constructor call forms, and the runtime tagged-union representation (including the
> `variant_id` declaration order — the input variant set for exhaustiveness checks in this document)
> are defined by RFC-010's "Record-style and Type-style Variant Construction (Authoritative
> Definition)" section; this document carries its mirror side: match destructuring and
> exhaustiveness checks. Construction and destructuring must come in pairs, so they belong together
> in RFC-010.
>
> Originally numbered RFC-039 (drafted 2026-09-03); on 2026-09-25 it was merged into RFC-010 as a
> sub-RFC and renumbered RFC-010b by owner decision, with tracking issue still #330. The body was
> expanded on 2026-09-27 and landed with the implementation (head e368958f); on the same day the
> owner confirmed promotion to accepted.

## Summary

Flesh out `match` pattern matching from "literals + wildcard only" into a complete capability:
variant destructuring (including zero- and multi-payload), struct/tuple patterns, or-patterns and
guards, final identifier-binding semantics, and exhaustiveness checks (E1030/E1031 transitioning
from empty stubs to real emission). This RFC is a prerequisite for the
`Error { kind: ErrorKind, message }` evolution path (RFC-013 "Runtime Error Values and Code
Integration" evolution section), but its motivation is independent — pattern matching is a general
language capability serving all types with variants.

## Motivation

### Current State (2026-09-03, v0.7.12 code; all resolved by 2026-09-27)

1. **Only literal patterns are real at the IR layer.** The AST defines the full pattern set, but IR
   generation only implements the `Literal` branch; other patterns fall into a stub — loading
   constant 0 for equality comparison, never matching; and when the scrutinee happens to be 0, the
   stub arm is falsely matched.
2. **The `match ok(v)/err(e)` example in stdlib.md is paper-only capability.**
3. **Exhaustiveness checks hang empty.** E1030/E1031 are registered in the code table but have no
   emission point.
4. **Error-handling evolution is blocked** (RFC-013's structured error modeling depends on variant
   destructuring).

### Design Goals

- **Variant destructuring usable**: `match r { ok(v) => ..., err(e) => ... }` and destructuring of
  user-defined variant sets are semantically consistent across all execution paths.
- **Exhaustiveness dependable**: E1030/E1031 emit for real; a `match` with a missing branch is a
  compile error.
- **Stubs cleared**: unsupported patterns raise a compile-time error rather than silently going
  wrong (the #330 safety net — this transition strategy is progressively replaced by real encoding
  during implementation, until all stubs are cleared).

## Proposal (Finalized and landed 2026-09-27)

| Capability                             | Status | Notes                                                                                                           |
| -------------------------------------- | ------ | --------------------------------------------------------------------------------------------------------------- |
| Union variant patterns                 | ✅     | `Variant` / `Variant(binding)` / multi-payload `make(a, b)` + payload bindings enter the arm's scope            |
| Struct / Tuple patterns                | ✅     | Field patterns (shorthand binding / `name: sub-pattern`), tuple patterns; declared fields must be fully covered |
| Or-pattern and Guard                   | ✅     | `p1 \| p2` and `pat if cond`: alternative-level failure chain, guard failure falls through to next arm          |
| Exhaustiveness check                   | ✅     | E1030 missing branch, E1031 unreachable, scope covers the variant set                                           |
| Identifier-pattern semantics confirmed | ✅     | Finalized as **binding** (see section below)                                                                    |

## Design Decisions

### Identifier Pattern: Binding, Not Assignment, Nor Comparison

A bare identifier `y` in `match x { y => ... }` is finalized as **binding** — it introduces a new
name that captures the matched value and matches unconditionally (equivalent to a named wildcard).
Three reasons:

1. **Structural argument**: In pattern position, identifiers naturally play two roles — the
   discriminator slot (the `ok` in `ok(v)`, checked against the variant table) and the leaf slot
   (the `v` in `ok(v)`, bound). Parameter lists (lambda) have only the leaf slot, so there's no
   ambiguity; a bare identifier simultaneously occupies the discriminator slot while looking like
   the leaf slot — that's where the ambiguity comes from. YaoXiang has a static variant table: the
   discriminator name is resolved by "table lookup" (if the pattern head is a name or literal that
   exists in the scrutinee's variant set → discriminator; otherwise a bare name → binding), without
   needing Rust-style case conventions.
2. **Degeneracy of value comparison**: "A bound name in a pattern means equality" (unification
   semantics) is a relic of single-assignment tradition; in a language with reassignable variables
   it cannot be statically determined whether something has been bound, so that semantics does not
   hold. YaoXiang variables can be reassigned (`mut`), so we follow Rust's legislation.
3. **The need to match an existing value is delegated to guards**: `n if n == threshold` (write the
   if-chain until guards land).

**Binding is not assignment** (the position aligned with the owner on 2026-09-27): pattern binding
introduces a **new name**, scoped within the arm, shadowing same-named outer variables, gone after
the arm (probe: outer `v = 999` remains as-is after `some(v)` matches); assignment overwrites an
existing name (the E2010 / `mut` set of laws). The two coexist and are conceptually separate in this
language; `=` dispatches based on whether the name already exists (spec §4.3, assignment takes
precedence), and pattern position belongs only to the binding side.

### match Is an Expression, Not an Assignment Statement

`match` yields the value of the matched arm (the value of the whole expression); `v = match ...` is
just ordinary binding syntax capturing it. The teeth of exhaustiveness checks come from "arms must
yield a value": a missing shape = the assignee gets no value = compile error (E1030); a
statement-style switch with a missing branch can only fall silent.

### Arm Semantic Model and Implementation

An arm = **a single-parameter function with an address plate**: `some(v) => body` ≈ `λv.body`,
called only when the argument has the `some` shape; the binding semantics come from "the left side
is the parameter slot". It shares the same `=>` source as lambda. Implementation-wise, an arm **is
not a closure**: the IR recursive pattern compiler (`compile_pattern`) inlines a test chain
(VariantTag/LoadField/Eq + JmpIfNot) with payload extraction (VariantPayload); on hit, it falls into
the arm body, on miss it jumps to the next arm — zero call overhead.

### Or-pattern and Guard

- `p1 | p2`: alternatives tried in order; a non-final alternative failing → next alternative, hit →
  extract payload and go straight to the arm body; the final alternative failing → next arm. **All
  alternatives' binding name sets must be identical** (E1033); same-named bindings share an
  arm-level pre-allocated register (what the body reads must necessarily be the value written by the
  hit alternative).
- `pat if cond`: after the pattern hits, evaluate `cond` (**pattern bindings are visible to the
  guard**); if false, fall through to the next arm. The guard condition must yield `Bool`. **Arms
  with guards do not count toward exhaustiveness coverage** (a guard may not match; same rule as
  Rust), but a repeated variant arm with a guard is reachable (a refinement of the prior arm).

### Struct Pattern

`Point { x, y: b }`: shorthand binds the same name, `name: sub-pattern` recurses. **Declared fields
must be fully covered** (E1034 for missing field); unknown fields are rejected (E1042); the same
field appearing twice raises E1032. `..` rest pattern and generic struct patterns are not yet
supported (see Open Questions).

### Variant Destructuring (Symmetric with RFC-010 Construction)

| Construction                       | Destructuring      | Payload Representation                                                                                |
| ---------------------------------- | ------------------ | ----------------------------------------------------------------------------------------------------- |
| `Result(Int, String).ok(5)`        | `ok(v)`            | Single payload: bound directly                                                                        |
| `Pair.make(3, 4)`                  | `make(a, b)`       | Multi payload: Tuple-packed, unpacked by position                                                     |
| `Color.red()` / `Option(T).none()` | `red()` / `none()` | Zero payload: `pattern: None`, arity check (writing `ok()` for a payload-bearing variant is an error) |

The payload type = the variant parameters with the scrutinee's type arguments substituted in
(declaration order); nested patterns (`ok(some(v))` across sum types, `Point { pos: Tuple(a, b) }`
across structs) recurse by sub-type.

### Exhaustiveness and Reachability

- **E1030 (missing branch, full exhaustiveness)**: **all types** of scrutinee — no catch-all arm
  means check coverage. Sum types: the variant set must be fully covered by guard-less Union arms
  (arms with guards don't count; a guard may not match); non-sum types: the value domain is open
  (Int/String/Char etc.), literal arms cannot be exhaustive, **a catch-all arm is required**.
  Enumerable exceptions: `Bool`'s `true`+`false` two literal arms are exhaustive; `Void` has no
  inhabited value, vacuously exhaustive. (2026-09-27 owner finalization: a `match` with no catch-all
  arm is an implicit assumption that "the remaining values cannot occur" — don't be implicit where
  it shouldn't be; the assumption must be written as `_`.)
- **E1031 (unreachable)**: any arm after a catch-all arm (an irrefutable pattern that always matches
  unconditionally: Identifier / Wildcard / Tuple / Struct, without a guard); guard-less repeated
  variant arms; repeated literal arms.

### Diagnostics

| Code  | Meaning                                                                                    |
| ----- | ------------------------------------------------------------------------------------------ |
| E1030 | Pattern coverage insufficient (lists missing variants)                                     |
| E1031 | Unreachable pattern (after catch-all arm / repeated variant / repeated literal)            |
| E1032 | Duplicate binding in pattern (same binding name appears multiple times within one pattern) |
| E1033 | Or-pattern binding inconsistent (alternatives have different binding name sets)            |
| E1034 | Struct pattern missing field                                                               |
| E3008 | IR-layer safety net retained (defensive branch; unreachable for well-formed programs)      |

### Boundary with Assignment/lambda (Anti-Confusion Memo)

- Pattern binding ≠ assignment: introduces a new name vs. overwrites an existing name (E2010 /
  `mut`);
- Or-pattern ≈ lambda application extended (semantic model), but implemented as inlined jumps, not
  closure calls;
- Literal patterns compile to Eq + conditional jump chains — the "optimized form of if/elif"
  intuition holds for literals; `match`'s increment is in shape judgment and unpacked binding.

## Implementation Landing Points

- parser: `parse_pattern` (variant call reassembly / struct pattern `Ident { .. }` branch / nested
  or-pattern restoration), `parse_match` or-pattern closing chain + guard (`no_fat_arrow` flag so
  `=>` doesn't act as lambda infix)
- typecheck: `check_match_expr` unified arm loop + `check_pattern` recursive checker (binding
  registration, guard Bool validation, exhaustiveness bookkeeping)
- ir_gen: `compile_pattern` recursive pattern compiler + `sum_type_variants` mirror carrying payload
  types and `sum_type_param_names` (payload types substituted by scrutinee arguments)
- Corpus: `02-type-system/aggregates/pattern_destructure.yx`, `pattern_or_guard.yx`; unit tests
  E1030/E1031/E1032/E1033/E1034 (rfc011b.rs)

## Open Questions

- [x] Identifier pattern: bind a new variable or match an existing value? → **Bind** (discriminator
      slot resolved by variant-table lookup, no case convention); matching an existing value is
      delegated to guards.
- [x] Exhaustiveness applicability boundary: → **Full exhaustiveness** (2026-09-27 owner
      finalization) — non-sum types with no catch-all arm also raise E1030; enumerable exceptions
      are only `Bool`'s two literals and `Void`.
- [x] During the transition period before stub loading-0 mis-match is fixed, should it first raise a
      compile error? → Yes (#330 safety net); all replaced by real encoding.
- [ ] Struct pattern `..` rest pattern (some fields + catch-all) — is it needed? (Affects the
      field-coverage rule.)
- [ ] Generic struct pattern (`Box { v, n }` against a `Box(Int)` value) — TBD. Currently rejected
      by a typecheck pre-check.
- [ ] `non_exhaustive`-class attribute (the break-radius of std variant-set extension on user
      `match`es) — TBD.

## Related

- RFC-013 "Runtime Error Values and Code Integration" evolution section (`Error { kind, message }`
  depends on this RFC)
- RFC-036 testing model (match-when-asserting-error-branch syntax in tests)
- RFC-011b Phase 2 (`?` interfacization and Result/Option promoted to std: the variant-set
  exhaustiveness in this RFC rides on the `sum_types` mirror; the determination does not depend on
  type ownership)
