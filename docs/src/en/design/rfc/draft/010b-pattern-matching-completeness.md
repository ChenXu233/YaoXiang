---
title: 'RFC-010b: Pattern Matching Completion (Variant Destructuring and Exhaustiveness)'
status: 'Draft'
author: 'Chenxu'
created: '2026-09-03'
updated: '2026-09-25'
issue: '#330'
parent: 'RFC-010'
---

# RFC-010b: Pattern Matching Completion (Variant Destructuring and Exhaustiveness)

> This document is a sub-RFC of [RFC-010](../accepted/010-unified-type-syntax.md) (Unified Type
> Syntax). The variant declaration, constructor call form, and runtime tagged union representation
> (including the `variant_id` declaration order—the input of the variant set for exhaustiveness
> checking in this document) are defined in the RFC-010 section "Record-Style and Variant
> Construction of Types (Authoritative Definition)"; this document carries its mirror side: match
> destructuring and exhaustiveness checking. Since construction and destructuring must come in
> pairs, they belong to RFC-010 together.
>
> Original number RFC-039 (drafted on 2026-09-03); on 2026-09-25, by the owner's decision, it was
> merged into RFC-010 as a sub-RFC and renumbered to RFC-010b; the tracking issue remains #330.

## Summary

Complete the pattern matching of `match` from the current "literals + wildcard only" to full
capabilities: Union variant patterns (including payload binding), struct/tuple patterns, the IR
landing of or-patterns and guards, and exhaustiveness checking (E1030/E1031 transitioning from empty
placeholders to real emission). This RFC is a prerequisite for the
`Error { kind: ErrorKind, message }` evolution path (RFC-013 "Runtime Error Value and Code
Unification" evolution section), but the motivation is independent—pattern matching is a general
language capability that serves all types with variants.

## Motivation

### Current State Evidence (2026-09-03, v0.7.12 code)

1. **Only literal patterns are real at the IR layer**. The AST has defined a complete pattern set
   (`src/frontend/core/parser/ast.rs:578`: Wildcard / Identifier / Literal / Tuple / Struct / Union
   / Or / Guard), and the parser can parse variant patterns like `ok(v)` and `err(e)`
   (`Pattern::Union`); however, IR generation (from `src/middle/core/ir_gen.rs:5330`) only
   implements the `Literal` branch, and the remaining patterns fall into a stub—loading the constant
   0 for equality comparison, **which never matches**; and when the scrutinee happens to be 0, it
   will **incorrectly match** into the stub arm (potential erroneous behavior).
2. **The `match ok(v)/err(e)` examples in stdlib.md are paper capabilities**. The variant
   destructuring syntax given in the specification document (§1.3 Result) cannot actually run; the
   implementation of the `?` operator does not go through match desugaring, which masks this. There
   are zero variant destructuring use cases in the test corpus (`match.yx` / `pattern_matching.yx`
   only covers literals and wildcards).
3. **Exhaustiveness checking is empty**. E1030 (Pattern non-exhaustive), E1031 (Unreachable pattern)
   are registered in the code table but have no emission points—match semantics are not yet defined.
4. **Error handling evolution is blocked**. The runtime `Error` value currently uses
   `{code, message}` string codes as the sole programmable judgment contract (RFC-013); structured
   modeling (`match e.kind { file_not_found(path) => ... }`) depends on the variant destructuring
   and payload binding of this RFC.

### Design Goals

- **Variant destructuring usable**: `match r { ok(v) => ..., err(e) => ... }` and destructuring of
  user-defined variant sets (record-style sum type) have consistent semantics across all three
  execution paths.
- **Exhaustiveness dependable**: E1030/E1031 emit for real; missing match branches are compile
  errors.
- **Stub removal**: Delete the placeholder behavior of "loading 0 never matches/incorrectly
  matches"; unsupported patterns should report errors at compile time rather than silently going
  wrong.

## Proposal (Skeleton Level, Subject to Elaboration)

| Capability                               | Description                                                                                                                                                                                   |
| ---------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Union variant patterns                   | `VariantName` / `VariantName(binding)` matching + payload binding into arm scope                                                                                                              |
| Struct / Tuple patterns                  | Landing of field patterns and tuple patterns (AST already exists, IR completion)                                                                                                              |
| Or-patterns and Guard                    | Evaluation order and binding rules for `p1 \| p2` and `p if cond`                                                                                                                             |
| Exhaustiveness checking                  | E1030 missing branches, E1031 unreachable; scope covers the variant set (Result/Option currently belong to core; judgment does not depend on attribution, see RFC-011b Phase 2 std migration) |
| Identifier pattern semantic confirmation | Whether a bare identifier in current syntax is a binding or a variant comparison, needs to be finalized (relationship with wildcard `_`)                                                      |

### Open Questions

- [ ] Identifier pattern: bind a new variable or match an existing value? (Rust semantics vs.
      existing YaoXiang corpus behavior)
- [ ] Exhaustiveness applicability boundary: how do variant sets returned by dynamic
      imports/interface methods participate in exhaustiveness judgment?
- [ ] Is an attribute like `non_exhaustive` needed (affects the break radius of std variant set
      extensions on user matches)?
- [ ] Should the stub's "load 0, incorrect match" report a compile error first during the transition
      period before the fix?

## Related

- RFC-013 "Runtime Error Value and Code Unification" evolution section (`Error { kind, message }`
  depends on this RFC)
- RFC-036 Test Model (matching syntax when tests assert error branches)
- Closed individual cases do not apply; this RFC is capability completion rather than defect fix
