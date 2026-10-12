---
title: 'RFC-029g: Remove the `pub` Keyword and Auto-Binding'
status: 'Accepted'
author: '晨煦'
created: '2026-10-02'
updated: '2026-10-02'
accepted: '2026-10-02'
issue: '#399'
---

# RFC-029g: Remove the `pub` Keyword and Auto-Binding

> Numbering note: The RFC-029 sub-RFC plan reserves 029b for "File Watching and Hot Reload"; 029c
> (re-export) has been removed and is not reused; 029a/029d/029e/029f are all taken. This RFC takes
> the next free number, 029g.

## Summary

Carry the ruling of RFC-029 "no visibility mechanism" to its conclusion: remove the `pub` keyword
and pub auto-binding, collapse the method form into explicit composition
(`Point.method: (self: &Point) -> T = {...}`), and make the final ruling at the language level that
**no visibility mechanism will be introduced**—if the ecosystem needs one in the future, the correct
seam is the **package boundary** (free within a package, declared across packages, going through the
manifest export surface), not a module-boundary keyword.

## Motivation

### The specification conflict has already been unilaterally resolved by the implementation

RFC-029 (decided 2026-07-30) explicitly states "no `pub`, no `private`, no `export`, no visibility
mechanism," and the decision table records "`pub` keyword: not wanted." Yet the language reference's
modules.md has long taught "pub exports, private by default." The implementation follows RFC-029
(`is_pub` does not participate in export judgment, non-pub top-level bindings can also be imported);
the export-rules section of modules.md has been rewritten to match the actual semantics, but the
pub-keyword and auto-binding passages remain, so the source of specification drift has not been
eliminated.

### Both real semantics of pub are unloaded

**Auto-binding** (when a pub function's first parameter is a concrete type in the same file, bind it
as a method):

- None of the 31 pub occurrences in std meet the threshold—list's first parameter is the generic
  `A: Type`, json's is a Ref shape, and the rest are type definitions;
- Across the entire repo's `.yx` corpus (tests, examples, docs), `pub` appears 0 times outside std;
- Every method form actually used in the repo is explicit composition, and the explicit form is
  strictly more expressive (RFC-004 multi-position binding, unit binding).

**Dead-code exemption** (when `is_pub` is true, the exemption switch is on, and there is no
reference pool, a pub item counts as a reachable root):

- The Bin role no longer exempts (029f Phase 1: no consumers outside the package, unused reports
  immediately);
- The Internal role's exemption has been tightened to reference-pool judgment;
- Script/Lib retain absolute exemption—Script is a single file with no "user elsewhere" to begin
  with, so the exemption protects a nonexistent reader; Lib's exemption is the "rather under-report"
  distribution-boundary policy of 029f. Once pub is removed, both fall into 029f's existing
  mechanism for non-pub items: Lib is judged by the in-package reference pool, Script by the
  single-file reference. Behavior tightens and becomes more precise.

**Dead `env.exports` table**: the typecheck environment populates an export set from "explicit
methods, pub top-level bindings, type definitions," along with the `is_exported`/`is_visible`
accessors; production code has zero readers. The real export surface is `ModuleInfo.exports` on the
module registry (read by use resolution, E1043, and LSP).

### Value ruling on the visibility mechanism

- YaoXiang types are transparent records (no private fields), so the classic value of
  visibility—"invariant protection"—does not exist;
- The value of "narrowing the API surface" only holds when there are external consumers, and the
  current ecosystem has none;
- If needed in the future, the seam is the **package boundary**, not a module-boundary
  keyword—module boundaries are bound to files, the granularity is too fine, and it conflicts with
  the "module = record" model (visibility would become a lock on a record, breaking transparency).

## Proposal

### Removal surface

| Location              | Content                                                                                                                        |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| lexer/parser          | `pub` keyword and its grammar branches (including pub-skipping inside use brace items)                                         |
| AST                   | `is_pub` field ×2 (test fixtures updated in sync)                                                                              |
| formatter             | pub prefix echo-back                                                                                                           |
| checker               | pub branch of `auto_bind_to_type` and `collect_exports` (explicit-method branch preserved)                                     |
| dead_code             | `is_exported` field and the `exempt_pub` exemption branch                                                                      |
| LSP                   | Public modifier in semantic tokens                                                                                             |
| ir_gen                | `is_pub`-ignore binding cleanup                                                                                                |
| std                   | 31 pub prefixes (list 25 / json 4 / option 1 / result 1)                                                                       |
| typecheck environment | The `exports` dead table and the `is_exported`/`is_visible` accessors are removed                                              |
| Diagnostic text       | W1xxx-series help text saying "pub is an external interface" (modify zh source, translations handled by bot)                   |
| Language reference    | modules.md §3.2–3.3 final convergence; keyword table in syntax.md; exemption wording in warning-codes.md; language-overview.md |
| RFC-029               | Add a reversal annotation to the implementation note "AST `is_pub` stays"; register 029g in the sub-RFC plan table             |

### Behavioral changes (recorded as-is)

| Role     | Current                                     | After removal                                                                                         |
| -------- | ------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| Bin      | pub not exempt                              | Unchanged (judged by reference pool)                                                                  |
| Internal | pub goes through reference pool             | Unchanged                                                                                             |
| Lib      | pub absolutely exempt (rather under-report) | Tightened: judged by in-package reference pool; unused top-level bindings in the package report W1001 |
| Script   | pub absolutely exempt                       | Tightened: unused top-level bindings report W1001 (same as ordinary bindings)                         |

Internal warnings for embedded std and vendored dependencies are already gated to "shown only for
this project's files," so removing pub from std will not bleed dead-code warnings into consumer
output.

### Preserved unchanged

- Explicit method composition (`Type.method:`) is fully preserved—this is the only method form;
- All four use forms and record destructuring semantics are unchanged;
- The E1043 module-export error path is unchanged.

## Trade-offs

**Gain**: removing a syntax path that never delivered on its design intent; convergence across five
mechanisms—keyword table, AST, checker, dead_code, and LSP; zero divergence between specification
and implementation.

**Loss**: a package author's "API not yet consumed within the package" goes from absolutely
unreported to possibly reporting W1001. This tightening is consistent with 029f's treatment of
non-pub Lib items—an extension of the existing direction rather than a new policy.

## Alternatives

- **Keep pub as pure decoration**: No—decorative keywords violate the "syntax carries semantics"
  principle, and they keep producing specification-drift surface (this very conflict is its harm).
- **Only remove auto-binding, keep dead-code exemption**: No—the exemption serves a nonexistent pub
  semantic; keeping it means keeping the keyword.
- **Introduce a real visibility mechanism**: No—the value ruling does not hold (transparent
  records + zero external consumers), and the correct seam is at the package boundary.

## Design Decision Record

| Decision             | Conclusion                             | Date       | Basis                                                                      |
| -------------------- | -------------------------------------- | ---------- | -------------------------------------------------------------------------- |
| `pub` keyword        | Remove                                 | 2026-10-02 | Both semantics are unloaded (survey data in Motivation)                    |
| Method auto-binding  | Remove                                 | 2026-10-02 | Explicit composition is strictly more expressive; zero hits in std         |
| Visibility mechanism | Never introduced at the language level | 2026-10-02 | Transparent records have no private fields; future seam = package boundary |
| Numbering            | 029g                                   | 2026-10-02 | 029b is reserved for hot reload; 029c is removed and not reused            |

## References

- RFC-029: Module Semantics System (visibility ruling and sub-RFC plan)
- RFC-029f: Compilation Target Roles and Import-Surface Semantics (role-aware dead-code model)
- RFC-004: Currying and Multi-Position Binding (basis for the expressiveness of explicit method
  composition)
- Language reference modules.md (actual semantics of export rules)
