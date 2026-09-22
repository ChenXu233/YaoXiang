---
title: 'RFC-039: Pattern Matching Completeness'
status: 'Draft'
author: 'Chenxu'
created: '2026-09-03'
updated: '2026-09-03'
issue: '#330'
---

# RFC-039: Pattern Matching Completeness

## Summary

Extend `match` pattern matching from the current "literals + wildcard only" to full capabilities:
Union variant patterns (with payload binding), Struct/Tuple patterns, IR implementation of
or-patterns and guards, and exhaustiveness checking (E1030/E1031 transition from empty stubs to
actual emission). This RFC is a prerequisite for the `Error { kind: ErrorKind, message }` evolution
path (RFC-013 "Runtime Error Values and Code Unification" evolution section), but the motivation is
independent—pattern matching is a general-purpose language capability serving all types with
variants.

## Motivation

### Current State Evidence (2026-09-03, v0.7.12 code)

1. **In the IR layer, only literal patterns are real.** The AST defines a complete pattern set
   (`src/frontend/core/parser/ast.rs:578`: Wildcard / Identifier / Literal / Tuple / Struct / Union
   / Or / Guard), and the parser can parse `ok(v)` and `err(e)` style variant patterns
   (`Pattern::Union`); however, IR generation (`src/middle/core/ir_gen.rs:5330` onwards) only
   implements the `Literal` branch, with other patterns falling into stubs—loading constant 0 for
   equality comparison, **never matches**; and when the scrutinee happens to be 0, it will
   **incorrectly match** into the stub arm (potential erroneous behavior).
2. **The `match ok(v)/err(e)` example in stdlib.md is a documentation-only capability.** The variant
   destructuring syntax shown in the specification document (§1.3 Result) does not actually run; the
   implementation of the `?` operator does not go through match desugaring, which masks this. There
   are no variant destructuring test cases in the test corpus (`match.yx` / `pattern_matching.yx`
   only cover literals and wildcards).
3. **Exhaustiveness checking is a dead stub.** E1030 (Pattern non-exhaustive) and E1031 (Unreachable
   pattern) are registered in the code table but have no emission points—match semantics are not yet
   defined.
4. **Error handling evolution is blocked.** Runtime `Error` values currently use `{code, message}`
   string codes as the only programmatically-judgeable contract (RFC-013); structured modeling
   (`match e.kind { file_not_found(path) => ... }`) depends on this RFC's variant destructuring and
   payload binding.

### Design Goals

- **Variant destructuring usable**: `match r { ok(v) => ..., err(e) => ... }` and user-defined
  variant sets (record-style sum types) destructuring has consistent semantics across three
  execution paths.
- **Reliable exhaustiveness**: E1030/E1031 are actually emitted; missing match branches are compile
  errors.
- **Stub removal**: Remove the placeholder behavior of "load 0 never matches / incorrectly matches";
  unsupported patterns become compile-time errors rather than silently going wrong.

## Proposal (skeleton-level, to be filled in)

| Capability                                | Description                                                                                                                                                                                                        |
| ----------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Union variant patterns                    | `VariantName` / `VariantName(binding)` matching + payload binding into arm scope                                                                                                                                   |
| Struct / Tuple patterns                   | Field patterns, tuple patterns implementation (AST exists, IR to be completed)                                                                                                                                     |
| Or pattern and Guard                      | Evaluation order and binding rules for `p1 \| p2` and `p if cond`                                                                                                                                                  |
| Exhaustiveness checking                   | E1030 missing branches, E1031 unreachable; scope covers the variant set (Result/Option currently belong to core; the judgment does not depend on where the type is defined, see RFC-011b Phase 2 migration to std) |
| Identifier pattern semantics confirmation | Whether a bare identifier in the current syntax is a binding or a variant comparison, needs to be finalized (relationship with wildcard `_`)                                                                       |

### Open Questions

- [ ] Identifier pattern: bind a new variable or match an existing value? (Rust semantics vs
      existing YaoXiang corpus behavior)
- [ ] Exhaustiveness applicability boundary: how do variant sets returned by dynamic
      imports/interface methods participate in exhaustiveness judgment?
- [ ] Is a `non_exhaustive`-like attribute needed (affects the blast radius of std variant set
      extension on user matches)?
- [ ] Should the stub's load-0 mis-match report a compile error first during the transition period
      before the fix?

## Related

- RFC-013 "Runtime Error Values and Code Unification" evolution section (`Error { kind, message }`
  depends on this RFC)
- RFC-036 Test Model (matching syntax for asserting error branches in tests)
- Closed cases are not applicable; this RFC is a capability completion, not a defect fix
