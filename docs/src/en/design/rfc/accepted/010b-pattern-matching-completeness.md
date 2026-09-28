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

> This document is a sub-RFC of [RFC-010](../accepted/010-unified-type-syntax.md) (Unified Type
> Syntax). The variant declaration, construction call forms, and runtime tagged union representation
> (including the `variant_id` declaration order — the variant set input for the exhaustiveness check
> in this document) are defined by RFC-010's section [Record-style Sum Type Variant Construction
> (Authoritative Definition)]; this document carries the mirror side: match destructuring and
> exhaustiveness checking. Since construction and destructuring must come in pairs, they are grouped
> together under RFC-010.
>
> Originally numbered RFC-039 (drafted on 2026-09-03); on 2026-09-25, by owner's decision, it was
> merged into RFC-010 as a sub-RFC and renumbered RFC-010b, with the tracking issue still #330. The
> body was fleshed out on 2026-09-27 and landed with the implementation (head e368958f); on the same
> day, confirmed by the owner and promoted to accepted.

## Summary

Complete `match` pattern matching from "literals only + wildcard" to full capability: variant
destructuring (including zero-payload and multi-payload), struct/tuple patterns, or-patterns and
guards, finalized identifier binding semantics, and exhaustiveness checking (E1030/E1031 transformed
from empty stubs to real emissions). This RFC is a prerequisite for the
`Error { kind: ErrorKind, message }` evolution path (the evolution section in RFC-013 "Runtime Error
Values and Codes"), but the motivation is independent — pattern matching is a general language
capability that serves all types with variants.

## Motivation

### Current State Evidence (2026-09-03, v0.7.12 code; fully resolved on 2026-09-27)

1. **Only literal patterns are real at the IR layer**. The AST has the complete pattern set defined,
   but IR generation only implements the `Literal` branch; other patterns fall into a stub — loading
   constant 0 to participate in equality comparison, never matching; and when the scrutinee happens
   to be 0, it mismatches into the stub arm.
2. **The `match ok(v)/err(e)` examples in stdlib.md are paper-only capability**.
3. **Exhaustiveness checking is a dangling stub**. E1030/E1031 are registered in the code table but
   have no emission points.
4. **Error handling evolution is blocked** (RFC-013's structured error modeling depends on variant
   destructuring).

### Design Goals

- **Variant destructuring usable**: `match r { ok(v) => ..., err(e) => ... }` and destructuring of
  user-defined variant sets have consistent semantics across all execution paths.
- **Exhaustiveness is dependable**: E1030/E1031 are emitted in real, and a `match` with missing
  branches is a compile error.
- **Stub elimination**: Unsupported patterns report errors at compile time instead of silently going
  wrong (the #330 safety net — this transitional strategy is gradually replaced by real encoding
  during the implementation period, and all stubs are eventually eliminated).

## Proposal (Finalized and Landed on 2026-09-27)

| Capability                             | Status | Description                                                                                                     |
| -------------------------------------- | ------ | --------------------------------------------------------------------------------------------------------------- |
| Union variant patterns                 | ✅     | `Variant` / `Variant(binding)` / multi-payload `make(a, b)` + payload binding into arm scope                    |
| Struct / Tuple patterns                | ✅     | Field patterns (shorthand binding / `name: sub-pattern`), tuple patterns; declared fields must be fully covered |
| Or-patterns and Guards                 | ✅     | `p1 \| p2` and `pat if cond`: alternative-level failure chain, guard failure falls through to next arm          |
| Exhaustiveness checking                | ✅     | E1030 for missing branches, E1031 for unreachable, scope covers the variant set                                 |
| Identifier pattern semantics confirmed | ✅     | Finalized as **binding** (see section below)                                                                    |

## Design Decisions

### Identifier Pattern: Binding, Not Assignment, Not Comparison

The bare identifier `y` in `match x { y => ... }` is finalized as **binding** — introducing a new
name to capture the matched value, matching unconditionally (equivalent to a named wildcard). Three
rationales:

1. **Structural argument**: In pattern positions, an identifier naturally has two roles —
   discriminator position (the `ok` in `ok(v)`, checked against the variant table) and leaf position
   (the `v` in `ok(v)`, binding). Parameter lists (lambda) only have the leaf position, so there's
   no ambiguity; a bare identifier stands both in the discriminator position and looks like a leaf
   position, which is where the ambiguity arises. YaoXiang has a static variant table: discriminator
   names are resolved by "table lookup hit" (a pattern head is the name or literal in the
   scrutinee's variant set → discriminator; a bare name that misses the table → binding), without
   needing Rust-style case conventions.
2. **Degeneracy of value comparison**: "A previously bound name in a pattern means equality"
   (unification semantics) is a product of single-assignment tradition; in a mutable language, you
   cannot statically determine "has it been bound", so this semantics does not hold. YaoXiang
   variables can be reassigned (`mut`), so it follows Rust's legislative approach.
3. **The need to match an existing value goes to guards**: `n if n == threshold` (write an if chain
   before guards land).

**Binding is not assignment** (the alignment reached with the owner on 2026-09-27): pattern binding
introduces **new names**, scoped to the arm, shadowing outer variables with the same name, gone
after the arm (probe: outer `v = 999` remains unchanged after `some(v)` matches); assignment
rewrites an existing name (E2010 / `mut` legal framework). Both coexist in this language and are
conceptually separate; `=` is dispatched based on whether the name already exists (spec §4.3
assignment priority), and the pattern position belongs only to the binding side.

### `match` Is an Expression, Not an Assignment Statement

`match` yields the value of the matched arm (the value of the entire expression); `v = match ...` is
just ordinary binding syntax catching it. The teeth of exhaustiveness checking come from "arms must
produce a value": a missing shape = the assignee gets no value = compile error (E1030); the
statement form of switch with a missing branch can only be silent.

### Arm Semantic Model and Implementation

An arm = **a single-parameter function with an address label**: `some(v) => body` ≈ `λv.body`,
called only when the actual argument has the `some` shape; the binding semantics come from "the left
side is a parameter position". It shares the same origin `=>` as lambda. In implementation, an arm
is **not a closure**: the IR recursive pattern compiler (`compile_pattern`) inlines the test chain
(VariantTag/LoadField/Eq + JmpIfNot) and payload extraction (VariantPayload); on a hit, it falls
into the arm body; on failure, it jumps to the next arm — zero call overhead.

### Or-patterns and Guards

- `p1 | p2`: alternatives tried in order; a non-last alternative fails → next alternative, a hit →
  extract payload then go straight to the arm body; a last alternative fails → next arm. **The
  binding name sets of each alternative must be identical** (E1033); same-name bindings share the
  arm-level pre-allocated register (what the body reads must be the value written by the hit
  alternative).
- `pat if cond`: after the pattern matches, evaluate `cond` (**pattern bindings are visible to the
  guard**); if false, fall through to the next arm. The guard condition must produce `Bool`. **Arms
  with guards do not count toward exhaustiveness coverage** (the guard may not hit, same rule as
  Rust), but duplicate variant arms with guards are reachable (they refine the previous arm).

### Struct Patterns

`Point { x, y: b }`: field shorthand binds the same name, `name: sub-pattern` recurses. **Declared
fields must be fully covered** (E1034 for missing fields), unknown fields are rejected (E1042), and
the same field appearing twice reports E1032. `..` name elision and generics struct patterns are not
yet supported (see Open Questions).

### Variant Destructuring (Symmetric with RFC-010 Construction)

| Construction                       | Destructuring      | Payload Representation                                                                            |
| ---------------------------------- | ------------------ | ------------------------------------------------------------------------------------------------- |
| `Result(Int, String).ok(5)`        | `ok(v)`            | Single payload: bound directly                                                                    |
| `Pair.make(3, 4)`                  | `make(a, b)`       | Multi-payload: packed as a Tuple, unpacked positionally                                           |
| `Color.red()` / `Option(T).none()` | `red()` / `none()` | Zero payload: `pattern: None`, arity check (`ok()` on a payload-bearing variant reports an error) |

The payload type = the variant parameters with the scrutinee type's actual parameters substituted
(declaration order); nested patterns (`ok(some(v))` across sum types, `Point { pos: Tuple(a, b) }`
across structs) recurse by sub-type.

### Exhaustiveness and Reachability

- **E1030 (missing branches, full exhaustiveness)**: for scrutinees of **all types** — no catch-all
  arm means check coverage. Sum types: the variant set must be fully covered by non-guarded Union
  arms (guarded arms don't count, the guard may not hit); non-sum types: the value domain is open
  (Int/String/Char, etc.), literal arms cannot be exhaustive, **there must be a catch-all arm**.
  Enumerable exceptions: Bool with both `true` and `false` literal arms is exhaustive; Void has no
  inhabitable values and is vacuously exhaustive. (2026-09-27 owner decision: a `match` without a
  catch-all is an implicit assumption that "the other values won't occur" — where it shouldn't be
  implicit, it isn't; the assumption must be written explicitly as `_`.)
- **E1031 (unreachable)**: any arm after a catch-all arm (unconditionally matching irrefutable
  patterns: Identifier/Wildcard/Tuple/Struct, without guards); non-guarded duplicate variant arms,
  non-guarded duplicate literal arms.

### Diagnostics

| Code  | Meaning                                                                                     |
| ----- | ------------------------------------------------------------------------------------------- |
| E1030 | Pattern coverage insufficient (lists missing variants)                                      |
| E1031 | Unreachable pattern (after catch-all arm / duplicate variant / duplicate literal)           |
| E1032 | Duplicate binding in pattern (same binding name appears more than once in the same pattern) |
| E1033 | Inconsistent bindings in or-pattern (different binding name sets in each alternative)       |
| E1034 | Missing field in struct pattern                                                             |
| E3008 | IR-layer safety net retained (defensive branch, unreachable in normal programs)             |

### Boundaries with Assignment/Lambda (Anti-Confusion Memo)

- Pattern binding ≠ assignment: introducing a new name vs. rewriting an existing name (E2010 /
  `mut`);
- Or-pattern ≈ extension of lambda application (semantic model), but implemented as inline jumps,
  not closure calls;
- Literal patterns compile to Eq + conditional jump chains — the intuition of "an optimized form of
  if/elif" holds for literals; the increment of `match` is in shape judgment and unpack binding.

## Implementation Landing Points

- parser: `parse_pattern` (variant call reassembly / struct pattern `Ident { .. }` branch / nested
  or-pattern restoration), `parse_match` or-pattern closing chain + guards (`no_fat_arrow` flag so
  that `=>` does not act as a lambda infix)
- typecheck: `check_match_expr` unified arm loop + `check_pattern` recursive checker (binding
  registration, guard Bool check, exhaustiveness bookkeeping)
- ir_gen: `compile_pattern` recursive pattern compiler + `sum_type_variants` mirror carrying payload
  types and `sum_type_param_names` (payload types substituted by scrutinee's actual parameters)
- Corpora: `02-type-system/pattern_destructure.yx`, `pattern_or_guard.yx`; unit tests
  E1030/E1031/E1032/E1033/E1034 (rfc011b.rs)

## Open Questions

- [x] Identifier pattern: bind a new variable or match an existing value? → **Binding**
      (discriminator position is resolved by variant table lookup, no case convention); matching
      existing values goes to guards.
- [x] The applicable boundary of exhaustiveness: → **Full exhaustiveness** (2026-09-27 owner
      decision) — a non-sum type without a catch-all arm also reports E1030; the only enumerable
      exceptions are Bool with both literals and Void.
- [x] Does the stub loading 0 mismatching in the transitional period before the fix first report a
      compile error? → Yes (#330 safety net), fully replaced by real encoding.
- [ ] Whether struct pattern `..` name elision (partial fields + catch-all) is needed? (Affects
      field coverage rules)
- [ ] Generics struct pattern (`Box { v, n }` on a `Box(Int)` value) is TBD — currently rejected by
      typecheck upfront.
- [ ] `non_exhaustive`-like attributes (the blast radius of std variant set expansion on user
      `match`es) is TBD.

## Related

- RFC-013 "Runtime Error Values and Codes" evolution section (`Error { kind, message }` depends on
  this RFC)
- RFC-036 test model (match writing style when tests assert error branches)
- RFC-011b Phase 2 (`?` interface-ization and Result/Option into std: this RFC's variant set
  exhaustiveness walks with the `sum_types` mirror; the decision does not depend on type ownership)
