---
title: 'RFC-039: Pattern Matching Completion'
status: 'Draft'
author: 'Chenxu'
created: '2026-09-03'
updated: '2026-09-03'
issue: '#330'
---

# RFC-039: Pattern Matching Completion

## Abstract

Complete `match` pattern matching from its current "literals + wildcards only" state into a full
capability: Union variant patterns (with payload binding), struct/tuple patterns, IR landing for
or-patterns and guards, and exhaustiveness checking (turning E1030/E1031 from empty-registered codes
into actually-emitted diagnostics). This RFC is a prerequisite for the
`Error { kind: ErrorKind, message }` evolution path (RFC-013 "Runtime Error Value and Code
End-to-End Integration" evolution section), but the motivation is independent—pattern matching is a
general-purpose language feature that serves all types with variants.

## Motivation

### Current State Evidence (2026-09-03, v0.7.12 code)

1. **Only literal patterns are real at the IR level**. The AST already defines a complete pattern
   set (`src/frontend/core/parser/ast.rs:578`: Wildcard / Identifier / Literal / Tuple / Struct /
   Union / Or / Guard), and the parser can parse variant patterns like `ok(v)` and `err(e)`
   (`Pattern::Union`); however, IR generation (from `src/middle/core/ir_gen.rs:5330`) only
   implements the `Literal` branch—the other patterns fall into stubs that load constant 0 and
   participate in equality comparison, **never matching**; and when the scrutinee happens to be 0,
   they **mistakenly match** into the stub arm (potential incorrect behavior).
2. **The `match ok(v)/err(e)` examples in `stdlib.md` are paper features**. The variant
   destructuring forms shown in the specification (§1.3 Result) cannot actually run; the `?`
   operator's implementation does not desugar through `match`, masking this fact. The test corpus
   has zero variant destructuring cases (`match.yx` / `pattern_matching.yx` only cover literals and
   wildcards).
3. **Exhaustiveness checking is registered-only**. E1030 (Pattern non-exhaustive) and E1031
   (Unreachable pattern) are registered in the code table but have no emission sites—match semantics
   are not yet finalized.
4. **Error handling evolution is blocked**. The runtime `Error` value currently uses
   `{code, message}` string codes as its sole programmable judgment contract (RFC-013); structured
   modeling (`match e.kind { file_not_found(path) => ... }`) depends on this RFC's variant
   destructuring and payload binding.

### Design Goals

- **Variant destructuring available**: `match r { ok(v) => ..., err(e) => ... }` and destructuring
  of user-defined variant sets (record-style sum type) have consistent semantics across all three
  execution paths.
- **Exhaustiveness is reliable**: E1030/E1031 actually emit; a missing match branch is a compile
  error.
- **Stub removal**: Eliminate the "load 0 and never match / mis-match" placeholder
  behavior—unsupported patterns become compile errors rather than silently going wrong.

## Proposal (Skeleton Level, to be Filled Out in Body)

| Capability                                | Description                                                                                                                      |
| ----------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| Union variant pattern                     | `VariantName` / `VariantName(binding)` matching + payload binding into the arm scope                                             |
| Struct / Tuple pattern                    | Field patterns and tuple patterns landed (AST already has them; IR completes)                                                    |
| Or pattern and Guard                      | Evaluation order and binding rules for `p1 \| p2` and `p if cond`                                                                |
| Exhaustiveness check                      | E1030 missing branch, E1031 unreachable; scope covers the variant set (including builtin Result/Option)                          |
| Identifier pattern semantics confirmation | Whether a bare identifier in current syntax means binding or variant comparison must be decided (relationship with wildcard `_`) |

### Open Questions

- [ ] Identifier pattern: bind a new variable, or match against an existing value? (Rust semantics
      vs. existing YaoXiang corpus behavior)
- [ ] Exhaustiveness applicability boundary: how do variant sets returned by dynamic imports /
      interface methods participate in exhaustiveness checking?
- [ ] Is a `non_exhaustive`-like attribute needed (affecting the breakage radius of std variant set
      extension on user matches)?
- [ ] Should the stub's load-0 mis-match report a compile error first during the transition period
      before the fix?

## Related

- RFC-013 "Runtime Error Value and Code End-to-End Integration" evolution section
  (`Error { kind, message }` depends on this RFC)
- RFC-036 Test Model (match forms used when tests assert error branches)
- Closed cases are not applicable; this RFC is a capability completion, not a defect fix
