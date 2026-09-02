---
title: 'RFC-039: Pattern Matching Completion'
status: 'Draft'
author: 'Chenxu'
created: '2026-09-03'
updated: '2026-09-03'
---

# RFC-039: Pattern Matching Completion

## Summary

Extend `match` pattern matching from the current "literals + wildcard only" to full capabilities:
Union variant patterns (including payload binding), struct/tuple patterns, IR lowering of Or
patterns and guards, and exhaustiveness checking (E1030/E1031 transitioning from empty placeholder
codes to real emission). This RFC is a prerequisite for the `Error { kind: ErrorKind, message }`
evolution path (RFC-013 "Runtime Error Values and Code Unification" evolution section), but the
motivation is independent—pattern matching is a general language capability that serves all types
with variants.

## Motivation

### Current State Evidence (2026-09-03, v0.7.12 code)

1. **Only literal patterns are real in the IR layer**. The AST has defined a complete pattern set
   (`src/frontend/core/parser/ast.rs:578`: Wildcard / Identifier / Literal / Tuple / Struct / Union
   / Or / Guard), and the parser can parse `ok(v)`, `err(e)` style variant patterns
   (`Pattern::Union`); but IR generation (starting at `src/middle/core/ir_gen.rs:5330`) only
   implements the `Literal` branch, and other patterns fall into a stub—loading the constant 0 to
   participate in equality comparison, **never matches**; and when the scrutinee happens to be 0, it
   will **mistakenly match** into the stub arm (potential erroneous behavior).
2. **The `match ok(v)/err(e)` examples in stdlib.md are paper capabilities**. The variant
   destructuring syntax given in the specification document (§1.3 Result) does not actually run; the
   `?` operator implementation does not go through match desugaring, which masks this. There are
   zero variant destructuring test cases in the test corpus (`match.yx` / `pattern_matching.yx` only
   cover literals and wildcards).
3. **Exhaustiveness checking is a placeholder**. E1030 (Pattern non-exhaustive) and E1031
   (Unreachable pattern) are registered in the code table but have no emission points—match
   semantics are not finalized.
4. **Error handling evolution is blocked**. The runtime `Error` value currently uses
   `{code, message}` string code as the only programmable judgment contract (RFC-013); structured
   modeling (`match e.kind { file_not_found(path) => ... }`) depends on this RFC's variant
   destructuring and payload binding.

### Design Goals

- **Variant destructuring is usable**: `match r { ok(v) => ..., err(e) => ... }` and destructuring
  of user-defined variant sets (record-style sum type) have consistent semantics across the three
  execution paths.
- **Exhaustiveness is reliable**: E1030/E1031 are actually emitted; missing branches in match are
  compilation errors.
- **Stub removal**: Delete the placeholder behavior of "loading 0 never matches / mistakenly
  matches"; unsupported patterns report errors at compile time rather than silently going wrong.

## Proposal (Skeleton-level, to be expanded)

| Capability                                | Description                                                                                                                             |
| ----------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| Union variant pattern                     | `VariantName` / `VariantName(binding)` match + payload binding into arm scope                                                           |
| Struct / Tuple pattern                    | Field patterns, tuple patterns lowered (AST exists, IR to be completed)                                                                 |
| Or pattern and Guard                      | Evaluation order and binding rules for `p1 \| p2` and `p if cond`                                                                       |
| Exhaustiveness checking                   | E1030 missing branches, E1031 unreachable; coverage scope includes variant sets (including built-in Result/Option)                      |
| Identifier pattern semantics confirmation | Whether a bare identifier in current syntax is a binding or a variant comparison needs to be finalized (relationship with wildcard `_`) |

### Open Questions

- [ ] Identifier pattern: bind a new variable or match an existing value? (Rust semantics vs
      existing YaoXiang corpus behavior)
- [ ] Boundaries of exhaustiveness applicability: how do variant sets returned by dynamic
      imports/interface methods participate in exhaustiveness checking?
- [ ] Is a non_exhaustive-like attribute needed (affects the damage radius of std variant set
      extensions on user match)?
- [ ] Should the stub's "loading 0" mismatching behavior first report a compile error during the
      transition period before the fix?

## References

- RFC-013 "Runtime Error Values and Code Unification" evolution section (`Error { kind, message }`
  depends on this RFC)
- RFC-036 Test Model (match syntax for asserting error branches in tests)
- Closed cases do not apply; this RFC is a capability completion, not a defect fix
