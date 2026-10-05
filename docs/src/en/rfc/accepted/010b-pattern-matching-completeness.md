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

> This document is a sub-RFC of [RFC-010](010-unified-type-syntax.md) (Unified Type Syntax). The
> declaration, construction call form, and runtime tagged union representation of variants
> (including the `variant_id` declaration order — the variant set input for exhaustiveness checking
> in this document) are defined by the RFC-010 section "Record-Style and Type's Variant Construction
> (Authoritative Definition)"; this document carries its mirror side: `match` destructuring and
> exhaustiveness checking. Construction and destructuring must come in pairs, so they belong
> together in RFC-010.
>
> Original number RFC-039 (drafted on 2026-09-03); on 2026-09-25, by owner decision, merged into
> RFC-010 as a sub-RFC and renumbered RFC-010b; tracking issue remains #330. The body was enriched
> on 2026-09-27 and landed alongside the implementation (head e368958f); on the same day, confirmed
> accepted by the owner.

## Summary

Complete `match` pattern matching from "literals + wildcard only" to full capability: variant
destructuring (including zero-payload and multi-payload), struct/tuple patterns, or-patterns and
guards, finalized identifier binding semantics, and exhaustiveness checking (E1030/E1031
transitioning from empty stub to real emission). This RFC is a prerequisite for the
`Error { kind: ErrorKind, message }` evolution path (RFC-013 "Runtime Error Values and Code Interop"
evolution section), but the motivation is independent — pattern matching is a general language
capability serving all types with variants.

## Motivation

### Current State Evidence (2026-09-03, v0.7.12 code; all resolved by 2026-09-27)

1. **Only literal patterns are real at the IR layer.** The AST defines the full pattern set, but IR
   generation only implements the `Literal` branch; other patterns fall into stubs — loading
   constant 0 for equality comparison, never matching; and when the scrutinee happens to be 0, it
   erroneously matches into the stub arm.
2. **The `match ok(v)/err(e)` example in stdlib.md is paper-only capability.**
3. **Exhaustiveness checking is empty stub.** E1030/E1031 are registered in the code table but have
   no emission point.
4. **Error handling evolution is blocked** (RFC-013's structured error modeling depends on variant
   destructuring).

### Design Goals

- **Variant destructuring usable**: `match r { ok(v) => ..., err(e) => ... }` and destructuring of
  user-defined variant sets behave semantically consistently on all execution paths.
- **Exhaustiveness is reliable**: E1030/E1031 are emitted in reality; missing branches in `match`
  are compile errors.
- **Stub cleared**: unsupported patterns report errors at compile time rather than silently going
  wrong (the #330 safety net — this transitional strategy is replaced phase by phase with real
  encoding during implementation, and all stubs are eventually cleared).

## Proposal (finalized and landed on 2026-09-27)

| Capability                             | Status | Description                                                                                                    |
| -------------------------------------- | ------ | -------------------------------------------------------------------------------------------------------------- |
| Union variant patterns                 | ✅     | `Variant` / `Variant(binding)` / multi-payload `make(a, b)` + payload binding into arm scope                   |
| Struct / Tuple patterns                | ✅     | Field patterns (shorthand binding / `name: subpattern`), tuple patterns; declared fields must be fully covered |
| Or-patterns and Guards                 | ✅     | `p1 \| p2` and `pat if cond`: alternative-level failure chain, guard failure falls through to next arm         |
| Exhaustiveness checking                | ✅     | E1030 missing branch, E1031 unreachable; scope covers the variant set                                          |
| Identifier pattern semantics confirmed | ✅     | Finalized as **binding** (see section below)                                                                   |

## Design Decisions

### Identifier Pattern: Binding, Neither Assignment nor Comparison

The bare identifier `y` in `match x { y => ... }` is finalized as **binding** — introducing a new
name to catch the matched value, always matching (equivalent to a named wildcard). Three reasons:

1. **Structural argument**: In pattern position, identifiers naturally have two roles — the
   discriminant position (the `ok` in `ok(v)`, checked against the variant table) and the leaf
   position (the `v` in `ok(v)`, binding). Parameter lists (lambdas) only have the leaf position, so
   there's no ambiguity; a bare identifier stands in the discriminant position yet looks like a leaf
   position, and the ambiguity arises from this. YaoXiang has a static variant table: discriminant
   names are adjudicated by "table lookup hit" (if the pattern head is a name or literal in the
   scrutinee's variant set → discriminant; a bare name that doesn't hit → binding), with no need for
   Rust-style case conventions.
2. **Value comparison's degeneracy**: "A name already bound in a pattern denotes equality"
   (unification semantics) is a relic of single-assignment traditions; in a mutable language, you
   can't statically determine "whether it's been bound", so this semantics doesn't hold. YaoXiang
   variables are reassignable (`mut`), so it follows Rust's legislation.
3. **The need to match existing values belongs to guards**: `n if n == threshold` (write an `if`
   chain before guards land).

**Binding is not assignment** (2026-09-27 alignment with the owner): pattern binding introduces a
**new name**, scoped within the arm, shadowing outer same-named variables, and gone when the arm
exits (probe: outer `v = 999` remains intact after `some(v)` hits); assignment rewrites an existing
name (E2010/`mut` rule). The two coexist and are conceptually distinct in this language; `=`
dispatches based on whether the name already exists (spec §4.3 assignment takes precedence), and the
pattern position belongs only to the binding side.

### `match` Is an Expression, Not an Assignment Statement

`match` produces the value of the matched arm (the value of the entire expression); `v = match ...`
is just ordinary binding syntax catching it. The teeth of exhaustiveness checking come from "arms
must produce a value": a missing shape = the assignee gets no value = compile error (E1030); a
statement-form `switch` with a missing branch can only be silent.

### Arm Semantic Model and Implementation

An arm = **a single-parameter function with an address label**: `some(v) => body` ≈ `λv.body`,
called only when the argument is of the `some` shape; binding semantics comes from "the left side is
parameter position". Shares the same source of `=>` with lambdas. In implementation, an arm is **not
a closure**: the IR recursive pattern compiler (`compile_pattern`) inlines and emits a test chain
(VariantTag/LoadField/Eq + JmpIfNot) and payload extraction (VariantPayload); on hit, falls into the
arm body; on failure, jumps to the next arm — zero call overhead.

### Or-Patterns and Guards

- `p1 | p2`: alternatives are tried in order; non-last alternative failure → next alternative; on
  hit → after payload extraction, directly into the arm body; last alternative failure → next arm.
  **The binding name sets of all alternatives must be identical** (E1033); same-named bindings share
  the arm-level pre-allocated register (the body necessarily reads the value written by the hit
  alternative).
- `pat if cond`: after the pattern hits, evaluate `cond` (**pattern bindings are visible to the
  guard**), and on false, fall through to the next arm. The guard condition must produce `Bool`.
  **Arms with guards do not count toward exhaustiveness coverage** (a guard may not hit, same rule
  as Rust), but duplicate variant arms with guards are reachable (they refine a previous arm).

### Struct Patterns

`Point { x, y: b }`: field shorthand binds the same name; `name: subpattern` recurses. **Declared
fields must be fully covered** (E1034 missing field); unknown fields are rejected (E1042); the same
field appearing twice reports E1032. `..` shorthand and generic struct patterns are not yet
supported (see Open Questions).

### Variant Destructuring (Symmetric with RFC-010 Construction)

| Construction                       | Destructuring      | Payload Representation                                                                               |
| ---------------------------------- | ------------------ | ---------------------------------------------------------------------------------------------------- |
| `Result(Int, String).ok(5)`        | `ok(v)`            | Single payload: bound directly                                                                       |
| `Pair.make(3, 4)`                  | `make(a, b)`       | Multi-payload: packed as Tuple, unpacked positionally                                                |
| `Color.red()` / `Option(T).none()` | `red()` / `none()` | Zero-payload: `pattern: None`; arity check (writing `ok()` on a payload-bearing variant is an error) |

Payload type = variant parameters with the scrutinee type's actual arguments substituted
(declaration order); nested patterns (`ok(some(v))` across sum types, `Point { pos: Tuple(a, b) }`
across structs) recurse by subtype.

### Exhaustiveness and Reachability

- **E1030 (missing branch, full exhaustiveness)**: **all types** of scrutinee — without a catch-all
  arm, coverage is checked. Sum types: the variant set must be fully covered by guardless Union arms
  (guarded arms don't count, a guard may not hit); non-sum types: the value domain is open
  (Int/String/Char etc.), literal arms cannot be exhaustive, **a catch-all arm is required**.
  Enumerable exceptions: a Bool's `true`+`false` dual literal arms are exhaustive; Void has no
  inhabitable value, vacuously exhaustive. (2026-09-27 owner decision: a `match` with no catch-all
  is an implicit assumption that "other values don't occur" — no implicit assumption where it
  shouldn't be implicit; the assumption must be written as `_`.)
- **E1031 (unreachable)**: any arm after a catch-all arm (an irrefutable pattern that always matches
  unconditionally: Identifier/Wildcard/Tuple/Struct, without a guard); guardless duplicate variant
  arms, guardless duplicate literal arms.

### Diagnostics

| Code  | Meaning                                                                                    |
| ----- | ------------------------------------------------------------------------------------------ |
| E1030 | Pattern coverage insufficient (lists missing variants)                                     |
| E1031 | Unreachable pattern (after catch-all arm / duplicate variant / duplicate literal)          |
| E1032 | Duplicate binding in pattern (same binding name appears multiple times within one pattern) |
| E1033 | Inconsistent bindings in or-pattern (alternative binding name sets differ)                 |
| E1034 | Struct pattern missing field                                                               |
| E3008 | IR-layer safety net retained (defensive branch, unreachable in normal programs)            |

### Boundaries with Assignment / Lambda (Anti-Confusion Memo)

- Pattern binding ≠ assignment: introducing a new name vs. rewriting an existing name (E2010/`mut`);
- Or-pattern ≈ extension of lambda application (semantic model), but implemented as inline jump, not
  a closure call;
- Literal patterns compile to Eq + conditional jump chain — the intuition of "an optimized form of
  if/elif" holds for literals; `match`'s increment is in shape judgment and unpacked binding.

## Implementation Landing Points

- parser: `parse_pattern` (variant call reorganization / struct pattern `Ident { .. }` branch /
  nested or-pattern restoration), `parse_match` or-pattern close + guard (`no_fat_arrow` flag
  prevents `=>` from acting as lambda infix)
- typecheck: `check_match_expr` unified arm loop + `check_pattern` recursive checker (binding
  registration, guard Bool check, exhaustiveness accounting)
- ir_gen: `compile_pattern` recursive pattern compiler + `sum_type_variants` mirror carrying payload
  types and `sum_type_param_names` (payload types substituted by scrutinee actual arguments)
- corpus: `02-type-system/pattern_destructure.yx`, `pattern_or_guard.yx`; unit tests
  E1030/E1031/E1032/E1033/E1034 (rfc011b.rs)

## Open Questions

- [x] Identifier pattern: bind a new variable, or match an existing value? → **Binding**
      (discriminant position adjudicated by variant table lookup, no case convention); matching
      existing values belongs to guards.
- [x] Exhaustiveness applicability boundary: → **Full exhaustiveness** (2026-09-27 owner decision) —
      non-sum types without a catch-all arm also yield E1030; enumerable exceptions are only Bool
      dual-literal and Void.
- [x] Does the stub loading 0's erroneous match report a compile error first during the fix's
      transition period? → Yes (the #330 safety net), all replaced with real encoding.
- [ ] Are struct pattern `..` shorthand (partial fields + catch-all) needed? (affects field coverage
      rules)
- [ ] Generic struct patterns (`Box { v, n }` over `Box(Int)` value) TBD — currently rejected by
      typecheck precondition.
- [ ] `non_exhaustive` class attribute (breakage radius of std variant set extension on user
      `match`) TBD.

## Related

- RFC-013 "Runtime Error Values and Code Interop" evolution section (`Error { kind, message }`
  depends on this RFC)
- RFC-036 test model (matching syntax when tests assert error branches)
- RFC-011b phase 2 (`?` interface-ification and Result/Option into std: this RFC's variant set
  exhaustiveness follows the `sum_types` mirror; the judgment doesn't depend on type ownership)
