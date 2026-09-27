---
title: 'RFC-010b: Pattern Matching Completeness (Variant Destructure and Exhaustiveness)'
status: 'accepted'
author: 'Chenxu'
created: '2026-09-03'
updated: '2026-09-27'
issue: '#330'
parent: 'RFC-010'
---

# RFC-010b: Pattern Matching Completeness (Variant Destructure and Exhaustiveness)

> This document is a sub-RFC of [RFC-010](../accepted/010-unified-type-syntax.md) (Unified Type
> Syntax). The declaration of variants, the forms of constructor calls, and the runtime tagged-union
> representation (including the `variant_id` declaration order — the input of the exhaustiveness
> check's variant set in this document) are defined in the section "Record-Style and Type-Style
> Variant Construction (Authoritative Definition)" of RFC-010. This document carries the mirror
> side: match destructure and exhaustiveness checking. Construction and destructure must come in
> pairs, so they belong to the same RFC-010.
>
> Originally numbered RFC-039 (drafted 2026-09-03); on 2026-09-25, by the owner's decision, it was
> merged as a sub-RFC of RFC-010 and renumbered RFC-010b; the tracking issue remains #330. The body
> was expanded on 2026-09-27 and landed with the implementation (head e368958f); on the same day,
> the owner confirmed it as accepted.

## Summary

Extend `match`'s pattern matching from "literals only + wildcard" to full capability: variant
destructure (including zero-payload and multi-payload), struct/tuple patterns, or-patterns and
guards, the finalization of identifier binding semantics, and exhaustiveness checking (E1030/E1031
transitions from dangling codes to actual emission). This RFC is a prerequisite for the
`Error { kind: ErrorKind, message }` evolution path (evolution section of RFC-013 "Runtime Error
Values and Code Unification"), but its motivation is independent — pattern matching is a general
language capability that serves all types with variants.

## Motivation

### Status Quo Evidence (2026-09-03, v0.7.12 code; all resolved by 2026-09-27)

1. **Only literal patterns are real at the IR layer.** The AST has a complete pattern set defined,
   but IR generation only implements the `Literal` branch; other patterns fall into stubs — loading
   the constant 0 for equality comparison, never matching; and when the scrutinee happens to be 0,
   it mismatches into the stub arm.
2. **The `match ok(v)/err(e)` examples in stdlib.md are paper-only capability.**
3. **Exhaustiveness checking is dangling.** E1030/E1031 are registered in the code table but have no
   emission site.
4. **The error-handling evolution is blocked** (the structured error modeling of RFC-013 depends on
   variant destructure).

### Design Goals

- **Variant destructure is usable:** `match r { ok(v) => ..., err(e) => ... }` and the destructure
  of user-defined variant sets have consistent semantics across all execution paths.
- **Exhaustiveness is reliable:** E1030/E1031 are actually emitted, and missing match branches are
  compile errors.
- **Stub removal:** unsupported patterns report errors at compile time rather than silently going
  wrong (the #330 safety net — this transitional strategy is replaced phase by phase during
  implementation with real encoding, and finally all stubs are cleared).

## Proposal (Finalized and Landed 2026-09-27)

| Capability                             | Status | Description                                                                                                     |
| -------------------------------------- | ------ | --------------------------------------------------------------------------------------------------------------- |
| Union variant patterns                 | ✅     | `Variant` / `Variant(binding)` / multi-payload `make(a, b)` + payload bindings enter arm scope                  |
| Struct / Tuple patterns                | ✅     | Field patterns (shorthand binding / `name: sub-pattern`), tuple patterns; declared fields must be fully covered |
| Or-patterns and Guards                 | ✅     | `p1 \| p2` and `pat if cond`: alternative-level failure chain, guard failure falls through to the next arm      |
| Exhaustiveness checking                | ✅     | E1030 missing branch, E1031 unreachable, scope covers the variant set                                           |
| Identifier pattern semantics confirmed | ✅     | Finalized as **binding** (see the section below)                                                                |

## Design Decisions

### Identifier Pattern: Binding, Not Assignment, Not Comparison

The bare identifier `y` in `match x { y => ... }` is finalized as **binding** — introducing a new
name to receive the matched value, matching unconditionally (equivalent to a named wildcard). Three
justifications:

1. **Structural argument:** An identifier in pattern position naturally has two roles — the
   discriminant position (`ok` in `ok(v)`, checked against the variant table) and the leaf position
   (`v` in `ok(v)`, binding). In parameter lists (lambda), only the leaf position exists, so there
   is no ambiguity; the bare identifier stands both in the discriminant position and looks like a
   leaf position — that is where the ambiguity comes from. YaoXiang has a static variant table:
   discriminant names are adjudicated by "table lookup hit" (if the pattern head is a name or
   literal in the scrutinee's variant set → discriminant; if a bare name fails the lookup →
   binding), without needing a Rust-style case convention.
2. **Degeneracy of value comparison:** "A name already bound in the pattern means equality"
   (unification semantics) is a legacy of the single-assignment tradition; in a mutable language it
   is impossible to statically determine "whether it has been bound," and this semantics does not
   hold. YaoXiang variables are reassignable (`mut`), so it follows Rust's legislation.
3. **The need to match an existing value falls under guards:** `n if n == threshold` (write an if
   chain before the guard lands).

**Binding is not assignment** (the alignment with the owner on 2026-09-27): Pattern binding
introduces a **new name**, which lives within the arm's scope, shadows same-named outer variables,
and vanishes when the arm exits (probe: outer `v = 999` remains unchanged after `some(v)` is hit);
assignment overwrites an existing name (the E2010/`mut` law). The two coexist in this language with
distinct concepts, and `=` diverges based on whether the name already exists (spec §4.3, assignment
takes priority), while the pattern position belongs only to the binding side.

### match Is an Expression, Not an Assignment Statement

`match` produces the value of the hit arm (the value of the entire expression), and `v = match ...`
is just ordinary binding syntax receiving it. The teeth of exhaustiveness checking come from "the
arm must produce a value": missing a shape = the assigner gets no value = compile error (E1030); the
statement-form switch silently swallows missing branches.

### Arm's Semantic Model and Implementation

An arm = **a single-parameter function with an address plate**: `some(v) => body` ≈ `λv.body`, only
called when the actual argument is of the `some` shape; the binding semantics come from "the left
side is the parameter position". It shares the same source of `=>` as lambda. In implementation, the
arm is **not a closure**: the IR recursive pattern compiler (`compile_pattern`) inlines the emission
of the test chain (VariantTag/LoadField/Eq + JmpIfNot) and payload extraction (VariantPayload); on a
hit it falls into the arm body, on a failure it jumps to the next arm — zero call overhead.

### Or-patterns and Guards

- `p1 | p2`: alternatives tried in order; a non-final alternative fails → next alternative, on a hit
  → extract the payload and go straight to the arm body; the final alternative fails → next arm.
  **The binding name sets of each alternative must be identical** (E1033), and bindings with the
  same name share the arm-level pre-allocated register (the body necessarily reads the value written
  by the hit alternative).
- `pat if cond`: after the pattern hits, evaluate `cond` (**pattern bindings are visible to the
  guard**), and if false fall through to the next arm. The guard condition must produce `Bool`.
  **Arms with guards are not counted toward exhaustiveness coverage** (guards may not hit; Rust has
  the same rule), but duplicate-variant arms with guards are reachable (a refinement of the previous
  arm).

### Struct Patterns

`Point { x, y: b }`: field shorthand binds the same name; `name: sub-pattern` recurses. **Declared
fields must be fully covered** (E1034 missing fields), unknown fields are rejected (E1042), and a
same field appearing repeatedly reports E1032. The `..` shorthand and generic struct patterns are
not yet supported (see Open Questions).

### Variant Destructure (Symmetric with RFC-010 Construction)

| Construction                       | Destructure        | Payload representation                                                                             |
| ---------------------------------- | ------------------ | -------------------------------------------------------------------------------------------------- |
| `Result(Int, String).ok(5)`        | `ok(v)`            | Single payload: bound directly                                                                     |
| `Pair.make(3, 4)`                  | `make(a, b)`       | Multi-payload: packed as Tuple, unpacked positionally                                              |
| `Color.red()` / `Option(T).none()` | `red()` / `none()` | Zero payload: `pattern: None`, arity check (`ok()` on a payload-carrying variant reports an error) |

The payload type = the variant's parameters after the scrutinee type's actual arguments are
substituted (declaration order); nested patterns (`ok(some(v))` crossing sum types,
`Point { pos: Tuple(a, b) }` crossing structs) recurse by subtype.

### Exhaustiveness and Reachability

- **E1030 (missing branch):** only for sum-type scrutinees. When there is no catch-all arm, the
  variant set must be fully covered by guardless Union arms. Arms with guards do not count (guards
  may not hit).
- **E1031 (unreachable):** any arm after a catch-all arm (an irrefutable pattern that hits
  unconditionally: Identifier/Wildcard/Tuple/Struct, without a guard); guardless duplicate-variant
  arms; guardless duplicate-literal arms.
- **Non-sum-type scrutinees:** the value domain is open, so E1030 is not performed (the lenient
  semantics of allowing all-literal arms with no catch-all is preserved); E1031 is applied as usual.

### Diagnostics

| Code  | Meaning                                                                                      |
| ----- | -------------------------------------------------------------------------------------------- |
| E1030 | Pattern coverage insufficient (list the missing variants)                                    |
| E1031 | Unreachable pattern (after a catch-all arm / duplicate variant / duplicate literal)          |
| E1032 | Pattern duplicate binding (the same binding name appears multiple times in the same pattern) |
| E1033 | Or-pattern binding inconsistent (the binding name sets of each alternative differ)           |
| E1034 | Struct pattern missing field                                                                 |
| E3008 | IR-layer safety net retained (defensive branch; unreachable in normal programs)              |

### Boundary with Assignment/Lambda (Anti-Confusion Memo)

- Pattern binding ≠ assignment: introducing a new name vs. overwriting an existing name
  (E2010/`mut`);
- Or-pattern ≈ extension of lambda application (semantic model), but implemented as inline jumps,
  not closure calls;
- Literal patterns compile to Eq + conditional jump chains — the intuition of "an optimized form of
  if/elif" holds for literals, and match's increment lies in shape judgment and destructure binding.

## Implementation Landing Points

- parser: `parse_pattern` (variant-call reassembly / struct pattern `Ident { .. }` branch / nested
  or-pattern restoration), `parse_match` or-pattern chain closing + guard (`no_fat_arrow` flag so
  that `=>` is not a lambda infix)
- typecheck: `check_match_expr` unified arm loop + `check_pattern` recursive checker (binding
  registration, guard Bool validation, exhaustiveness accounting)
- ir_gen: `compile_pattern` recursive pattern compiler + `sum_type_variants` mirror carrying the
  payload type and `sum_type_param_names` (payload type substituted by the scrutinee's actual
  arguments)
- corpus: `02-type-system/pattern_destructure.yx`, `pattern_or_guard.yx`; unit tests
  E1030/E1031/E1032/E1033/E1034 (rfc011b.rs)

## Open Questions

- [x] Identifier pattern: bind a new variable or match an existing value? → **Binding** (the
      discriminant position is adjudicated by table lookup against the variant table, no case
      convention); matching an existing value falls under guards.
- [x] The applicable boundary of exhaustiveness: → variant set exhaustiveness applies only to sum
      types; non-sum-type value domains are open and E1030 is not performed.
- [x] Should the stub loading 0 mismatching in the transitional period before the fix first report a
      compile error? → Yes (the #330 safety net), and all have been replaced with real encoding.
- [ ] Is the struct pattern `..` shorthand (partial fields + catch-all) needed? (affects field
      coverage rules)
- [ ] Generic struct patterns (`Box { v, n }` on a `Box(Int)` value) are TBD — currently rejected
      up-front by typecheck.
- [ ] non_exhaustive-like attributes (the damage radius of std variant set extension on user
      matches) are TBD.

## Related

- RFC-013 "Runtime Error Values and Code Unification" evolution section (`Error { kind, message }`
  depends on this RFC)
- RFC-036 Test Model (match syntax when assertions test error branches)
- RFC-011b Phase 2 (`?` interface-ization and Result/Option return to std: this RFC's variant set
  exhaustiveness goes with the `sum_types` mirror, and the judgment does not depend on type
  ownership)
