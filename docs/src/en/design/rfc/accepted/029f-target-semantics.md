---
title: 'RFC-029f: Compile Target Roles and Import Surface Semantics'
status: 'Accepted'
author: 'Chenxu'
created: '2026-09-12'
updated: '2026-09-13'
accepted: '2026-09-13'
issue: '#334'
---

# RFC-029f: Compile Target Roles and Import Surface Semantics

## Summary

Fulfilling RFC-029's "Sub-RFC Planning" extension slot (029b–029e already taken, deferred to 029f):
defines a **Compile Target Role Model** for source files (five types: Script / Bin / Lib / Test /
Internal) and its **Import Surface Semantics** — inferred by entry reachability when no manifest
(zero-config), or explicitly declared via `[lib]` / `[[bin]]` / `[exports]` (fields defined by
RFC-015) when a manifest exists. This fills four dangling semantic gaps: what counts as the
cross-package import surface, the applicable scope of `pub` exemption in dead code warnings (RFC
#321 Decision B), the relationship between `[exports]` and Registry reachability, and the term
disambiguation between RFC-014b's `[binaries]` (precompiled distribution artifacts) and bin-role
source files.

## Motivation

### Host and Boundary Basis

RFC-029 (Accepted)'s entry selection and visibility decisions are the direct upstream of this RFC:

> **Entry file selection** priority: 1. `[run].main` 2. First item of `[[bin]]` `path` 3.
> `src/main.yx` (conventional default). — RFC-029 §Inter-package Cycles: Revisited

> **Visibility**: does not exist. Scope + distribution boundary cover all scenarios; no new keyword
> needed. — RFC-029 §Design Decision Record (2026-07-30)

The second statement is the design red line of this RFC: Import surface semantics **must not
introduce a new visibility keyword**; it can only be borne by the "distribution boundary" (file
role + declared export surface).

Four consumers each hit the same wall:

| Consumer                                         | Wall they hit                                                                                                                                                                                                                                       |
| ------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| #321 Dead code warning                           | Decision B "pub = external interface, never reported" exists only because no role model distinguishes "truly external" from "looks external"; whether to report unused `pub` inside bin (Plan A) has been pending to date                           |
| RFC-014c Workspace (under review, #113)          | When member packages `use` each other, what counts as the import surface — `[exports]`, the single `[lib]` file, or all pub files — **not a single semantic provision in the entire document**                                                      |
| RFC-015 Configuration System (Accepted)          | `[lib]` / `[[bin]]` / `[exports]` fields are defined; apart from RFC-029's consumption of entry priority, the semantics of the remaining fields (especially the relationship between `[exports]` and Registry reachability) have never been aligned |
| RFC-037 Packaging / RFC-029a Cache (Draft, #293) | Packaging artifact selection, incremental recompilation cache unit boundaries — all need the role model as a prerequisite                                                                                                                           |

### Current Problem

**Four semantic gaps** (inventoried 2026-09-12, full-text confirmed across
014/014a/014b/014c/015/029):

1. **Cross-package import surface undefined**. 014c's member inter-reference example contains only a
   directory structure (`src/lib.yx`); what `use utils.helper` gets has no provision. The current
   implicit rule is "all top-level pub files of the package" — internal implementation files are
   also dragged into the namespace, and the dependency boundary exists in name only.
2. **Dead code `pub` exemption has no applicable scope**. #321's Decision B "pub never reported" is
   over-conservative in single-file scripts (scripts have no external consumers, `pub` is
   meaningless), yet over-lenient in the entry file of multi-file projects (an unimported `pub` on
   the root file is dead code). Without a role model, the dividing line between Plans A/B cannot be
   drawn.
3. **Three export concepts unaligned**. `[exports]` (015, path mapping), `[lib]` (015, single file
   path), Registry reachability (029, reachable from entry along `use`) — whether the three are in
   inclusion, equality, or independent relationships has no provision.
4. **Term collision**. RFC-014b's `[binaries]` is **precompiled distribution artifacts** (.so/exe
   download-priority policy), a completely different layer from "bin target (source role)". Once
   014c lands, the two will appear on the same screen; without early disambiguation,
   misunderstandings will persist.

**Evidence: Decision B's conservatism is already a real pain point**. The #321 M2 retrospective
records that the structural reason the W1001–W1005 family is silent under default config is
precisely "all `pub` treated as external interface" — this is compensatory behavior due to a missing
model, not the end state.

## Proposal

### Core Design: File Role Model

**Roles are file-level attributes** (consistent with 015's field granularity: `[lib]` is a single
file, `[[bin]]` is a file list, `[exports]` is a file mapping), not package-level switches.

| Role         | Classification                                                                                                                            | Semantics                                                                                                                                                                                                                                                                                     |
| ------------ | ----------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Bin**      | File pointed to by `[run].main` / `[[bin]].path`; or (no manifest) a file containing `main` and not `use`d by any other file              | Program entry. Requires `main` and it must be a function; otherwise compile error (E3020 missing main / E3021 main not a function), and top-level may not have executable statements. Its `pub` **may be reported as dead code** (no extra-package consumer); `main` is the reachability root |
| **Lib**      | File hit by `[exports]` mapping; or `[lib].path`; or (no manifest) a file `use`d by another file                                          | Distribution boundary. Its `pub` **exempted from dead code** (extra-package consumers not visible, prefer under-reporting); symbols exported through it are visible across packages                                                                                                           |
| **Test**     | File hit by RFC-036 test file rules (`tests/` directory, `*_test.yx`, etc., existing conventions); **no** `[[test]]` explicit declaration | Test code. Not part of dead code judgment; its references count as reachability roots of the tested code                                                                                                                                                                                      |
| **Internal** | In a package with manifest: a file neither in the export surface nor containing `main`                                                    | Intra-package implementation. `pub` exempted to Phase 2 (tightened by intra-package use-graph reachability, with Bin role corpus validation as prerequisite); extra-package `use` **unreachable**                                                                                             |
| **Script**   | A file run directly as a single file (`run foo.yx`)                                                                                       | **No entry concept**. Top-level statements (including binding initializers) are the program body in source order; `main` is an ordinary binding, **not auto-invoked** — to run, write `main()`. Registry has only std                                                                         |

**Classification priority**: manifest explicit declaration > entry reachability inference > 036 test
conventions. Explicit declaration always wins — inference is only the default when no declaration
exists.

### Entry Judgment

The role model is used not only for dead code analysis, but also **drives entry judgment**. Entry
rules for each role:

| Role                      | Entry rule                                                                                                                                                                                     |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Script** (no manifest)  | **No entry concept**. Top-level statements (including binding initializers) are the program body in source order; `main` is an ordinary binding, **not auto-invoked** — to run, write `main()` |
| **Bin** (with manifest)   | Requires `main` and it must be a function; otherwise compile error (E3020 missing main / E3021 main not a function). Top-level may not have executable statements                              |
| **Lib / Internal / Test** | Do not require `main` (they are not entries)                                                                                                                                                   |

**Script and Bin entries must be mutually exclusive**: "Under Script, `main` is not special" and
"Under Bin, `main` is the entry" cannot both hold — if Script both executes top-level statements and
implicitly calls `main`, a script that explicitly writes `main()` would run twice (#356). Therefore
Script has only one execution entry: top-level statements.

**Distinction between role classification and entry judgment** (implementation note): entry judgment
uses "whether a manifest exists" (`find_project_root().is_some()`) rather than `roles::classify()`.
Reason: `classify` answers "who consumes this file" (serving dead code analysis); a file with a
manifest but no `main` is Internal under `classify` (it isn't `use`d by other files, which is
reasonable), but if the user runs it as an entry, it must have an entry. Two different problems,
different criteria.

See `docs/src/reference/language-spec/syntax.md` §3.11 for details.

### Example

```toml
# yaoxiang.toml（字段全部是 RFC-015 已定义的，本 RFC 不新增配置面）
[lib]
path = "src/lib.yx"

[[bin]]
name = "my-cli"
path = "src/cli.yx"

[exports]
"." = "src/lib.yx"
"./internal-helper" = "src/helper.yx"   # ← 决定跨包可见性的是它，不是 pub
```

```yaoxiang
# src/cli.yx（Bin 角色）
pub unused_fn = (x: Int) => x    # ← W1001 可报：bin 无包外消费者
main: () -> Void = { ... }

# src/lib.yx（Lib 角色，在导出面上）
pub api_fn = ...                 # ← 永不报：包外消费者不可见

# src/other.yx（Internal 角色，不在导出面）
pub semi_api = ...               # ← 豁免至二阶段（宁漏报），届时按 use 图收紧
```

### Import Surface Semantics

Cross-package `use pkg.x` resolution order (first hit takes effect):

1. `[exports]` mapping exists → import surface = the set of files listed by the mapping; `x` must be
   among the top-level bindings of these files;
2. Otherwise `[lib].path` exists → import surface = that single file;
3. Neither (no manifest dependency, path dependency on bare directory) → **maintain status quo**:
   all top-level files are importable (compatible with existing behavior; tightening to be discussed
   separately).

**Workspace members**: The export surface is defined only by the member package's own manifest; the
workspace root must not override or extend a member's export surface — member self-containment is
014c's core design, and root override would break encapsulation.

Relationship with Registry reachability (aligned with 029): `[exports]`/`[lib]` define the **set of
legal starting points for cross-package resolution**; once the starting points are determined, the
Registry still extends per 029's "from entry along `use` reachable" — internal files `use`d in by
exported files are linked in, but do not generate new cross-package starting points.

### Boundary with Existing Plans

| Neighbor                                           | Boundary                                                                                                                                                                                                                                                         |
| -------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| RFC-029d "CLI `--entry` override entry" (planning) | 029f defines the role model; 029d consumes it — `--entry` is the CLI-level override for the Bin role                                                                                                                                                             |
| RFC-014b `[binaries]`                              | **Term disambiguation**: `[binaries]` is precompiled distribution artifact (download-priority policy), `[[bin]]` is source role declaration. The former exists in a dependency package's manifest, the latter in the own package's manifest; not interchangeable |
| RFC-014c Workspace                                 | Inter-member import surface = this RFC's resolution order; workspace coordination mechanism (members, shared lockfile) still belongs to 014c                                                                                                                     |
| RFC-036 Test                                       | Test role classification directly uses 036's existing file rules, no new conventions                                                                                                                                                                             |
| #321 Dead code                                     | Decision B ("pub always exempt") explicitly downgraded to "transitional semantics when no target"; Bin-role `pub` being reportable is the form in which this RFC fulfills Plan A                                                                                 |

## Detailed Design

### Role Classification Algorithm (in Orchestrator)

```
fn classify(files, manifest, entry_reach) -> Map<File, Role>:
    # 1. 显式层：manifest 声明优先
    for f in manifest.exports.values():  role[f] = Lib
    if manifest.lib_path:                role[lib_path] = Lib
    for b in manifest.binaries:          role[b.path] = Bin
    if manifest.run_main:                role[run_main] = Bin

    # 2. 推断层：无声明处按入口可达性
    for f in files where role[f] 未定:
        if f 被 ≥1 个非自身文件 use:      role[f] = Lib
        elif f 含 main 且无其他文件 use 它: role[f] = Bin
        else:                             role[f] = Internal

    # 3. 测试层：036 规则命中 → Test（覆盖 Lib/Internal，不覆盖显式 Bin）
    apply_rfc036_test_rules(files, &role)

    # 单文件直跑：整个模型旁路，行为不变
    if no manifest and single_file:      all Script
```

### Compiler Changes

- `frontend/config.rs`: Manifest parsing supplements the `[exports]` → role table mapping (field
  parsing already in 015).
- `frontend/module/orchestrator.rs`: Insert a role classification phase before Registry
  construction; cross-package resolution validates `use` targets per import surface order,
  out-of-bounds reports the existing `module_not_found` family (no new error code, message
  supplemented with "not in export surface" hint).
- `typecheck/passes/dead_code.rs`: Entry point set changed from "main + all pub" to "main + Lib-role
  file's pub" (Bin immediately reportable; Internal exempted to Phase 2, see Implementation Strategy
  for Phase 2).
- LSP (RFC-017): Completion/hover "importable items" filtered by import surface; `main` missing
  diagnostic only reported for Bin-role files.

### Runtime Behavior

No change. The role model is purely compile-time/parse-time semantics, affecting neither IR nor
execution.

### Backward Compatibility

- No-manifest projects: Behavior is completely unchanged (the inference layer restores the current
  state — `use`d is Lib, contains `main` is Bin). The dead code warning increment occurs only in Bin
  files: a root file's unused `pub` with no importers changes from silent to W1001 — this is the
  expected behavior of #321 Plan A, not a break.
- Projects with manifest: When `[exports]` is already declared, the import surface narrows from "all
  pub files" to the declared surface. **This is the only behavior narrowing point**: code that
  depends on other packages' internal files will start reporting `module_not_found`. Given the
  package ecosystem is not yet established (029: "currently no third-party package ecosystem"), the
  breaking surface is zero; still, a one-minor-version transition period is set (out-of-bounds first
  warned, then errored).

## Trade-offs

### Advantages

- Zero new configuration surface: all fields are already defined by 015; this RFC only supplements
  semantics.
- Zero new keyword: import surface is borne by the distribution boundary, isomorphic with 029's
  "visibility doesn't exist" decision.
- Four consumers (#321 / 014c / 029a / 037) modeled once, no longer each hitting their own wall.
- The no-manifest path costs nothing: the script language's zero-config DNA is fully preserved.

### Disadvantages

- Role inference (`use`d becomes Lib), in the common pattern of "entry file `use`s utility file",
  will classify the utility file as Lib, with its `pub` exempt — wider than ideal granularity. This
  is an intentional choice in the prefer-under-report direction; tightening requires call-direction
  analysis on the use graph, which is not done in the first version.
- `[exports]` narrowing the import surface is a breaking semantic tightening (despite the transition
  period).

## Alternatives

| Option                                   | Description                                                                   | Reason not adopted                                                                                                                                                                                                                     |
| ---------------------------------------- | ----------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Top-level RFC-040                        | Open a new top-level number                                                   | Module graph semantics (import surface / reachability / entry) all live in 029's territory; splitting into a top-level would create two top-level documents for the same object; and the 029b–029e slot pattern is already established |
| Attach to 014c                           | As a section of the workspace RFC                                             | Dependency direction reversed: language layer (029x) defines semantics, package layer (014x) consumes semantics. 014c is still under review; mid-review scope expansion would delay its landing                                        |
| Plan A as-is (#321 menu)                 | Manifest forced to declare `[[bin]]`/`[[lib]]` target for dead code semantics | Forced configuration for a narrow slice (root file with `main` and unused `pub`); this RFC's inference layer achieves the same benefit with zero configuration                                                                         |
| Pure entry inference (no manifest layer) | Do not consume 015 fields                                                     | `[exports]` is already a 015-accepted field, and the cross-package import surface cannot bypass it; not defining it equals leaving a gap for 014c                                                                                      |

## Implementation Strategy

### Phased Plan

- **Phase 1**: Role classification + import surface resolution + Bin `pub` reportable (Internal/Test
  remain exempt)
- **Phase 2**: Internal `pub` tightened to "reportable when intra-package use graph is unreachable"
  — prerequisite is Phase 1 lands, then Bin-role warning corpus validation shows no false-positive
  callback; triggered immediately, no timeline set

### Dependencies

- Prerequisites: RFC-029 orchestrator (landed), RFC-015 manifest fields (defined).
- Depended on by: #321 (Bin `pub` reportable), RFC-014c (import surface), RFC-029a (role as cache
  unit boundary), RFC-037 (artifact selection), RFC-029d (`--entry` override).
- Landing order with RFC-014c: Before 014c is approved, 029f's import surface order should be
  finalized first; 014c consumes it by reference, avoiding mid-review scope expansion.

### Risks

- Deviation of role inference from user intuition (first item in trade-offs above) → present all
  increment behavior as warnings (non-blocking), disable at any time.
- Transition-period management for the `[exports]` narrowing → warn first within the minor version.

## Open Questions

All three draft-stage items have been decided (2026-09-13, recorded in Appendix B and the body); no
open items remain:

- [x] Whether Internal `pub` should tighten → **Decision**: Phase 2 tightened by intra-package
      use-graph reachability, with Bin role corpus validation as prerequisite (see Implementation
      Strategy Phase 2)
- [x] `[[test]]` explicit declaration → **Decision**: Not introduced; Test role is judged solely by
      036 rules; if 036 later needs explicit declaration, it will extend itself
- [x] Workspace root overriding member export surface → **Decision**: Not allowed; member
      self-containment is 014c's core design, and root override would break encapsulation

---

## Appendix B: Design Decision Record

| Decision                                          | Determination                                                                                        | Date       | Recorder | Basis                                                                                                                         |
| ------------------------------------------------- | ---------------------------------------------------------------------------------------------------- | ---------- | -------- | ----------------------------------------------------------------------------------------------------------------------------- |
| Number                                            | 029f (029b reserved for hot reload, 029c deleted, 029d/029e placeholder)                             | 2026-09-12 | Chenxu   | RFC-029 §Sub-RFC Planning                                                                                                     |
| Belongs to 029 family rather than top-level RFC   | Yes                                                                                                  | 2026-09-12 | Chenxu   | Import surface / reachability / entry all belong to module graph semantics; 029 has already consumed `[[bin]]` entry priority |
| Visibility keyword                                | Not introduced                                                                                       | 2026-09-12 | Chenxu   | Isomorphic with RFC-029 "visibility doesn't exist" decision; import surface is borne by the distribution boundary             |
| Status of #321 Decision B                         | Downgraded to "transitional semantics when no target"                                                | 2026-09-12 | Chenxu   | Bin-role `pub` reportable is the form in which #321 Plan A is fulfilled                                                       |
| Role granularity                                  | File-level                                                                                           | 2026-09-12 | Chenxu   | 015 field granularity (`[lib]` single file, `[[bin]]` list, `[exports]` mapping)                                              |
| `[binaries]` disambiguation                       | 014b precompiled distribution artifact ≠ `[[bin]]` source role                                       | 2026-09-12 | Chenxu   | Two-layer concept (download policy vs compile role); nailed down before 014c lands                                            |
| Internal `pub` tightening                         | Phase 2 tightened by intra-package use-graph reachability; prerequisite = Bin role corpus validation | 2026-09-13 | Chenxu   | Prefer-under-report with progressive tightening; corpus-driven to avoid premature false positives                             |
| `[[test]]` explicit declaration                   | Not introduced; Test judged solely by 036 rules                                                      | 2026-09-13 | Chenxu   | No new configuration surface before 036 has the need                                                                          |
| Workspace root overriding member export surface   | Not allowed                                                                                          | 2026-09-13 | Chenxu   | Member self-containment is 014c's core design; root override breaks encapsulation                                             |
| Whether `main` is auto-invoked under Script       | **Not auto** — top-level statements are the program; `main()` must be explicit                       | 2026-09-17 | Chenxu   | Two rules coexisting would make an explicit `main()` run twice (#356); Script can have only one execution entry               |
| Whether the role model is used for entry judgment | **Yes** (previously used only for dead code warning)                                                 | 2026-09-17 | Chenxu   | This RFC defines Bin's "`main` is the reachability root"; without connecting to entry, that semantics would spin idle         |

## Appendix C: Glossary

| Term                  | Definition                                                                                                                          |
| --------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| Role                  | A source file's compile-target identity: Script / Bin / Lib / Test / Internal                                                       |
| Import Surface        | The set of files that serve as legal resolution starting points in cross-package `use`, declared by `[exports]`/`[lib]` or inferred |
| Distribution Boundary | 029 terminology: a package's external exposure range; this RFC refines it from implicit (all pub files) to the import surface       |
| Script State          | The bypass form of running a single file without a manifest; all behavior matches the current state                                 |

## References

- RFC-029 Module Semantics (parent RFC: entry selection, visibility decision, sub-RFC planning)
- RFC-015 Configuration System (definitions of `[lib]` / `[[bin]]` / `[exports]` fields)
- RFC-014b Build System and Binary Distribution (`[binaries]`, the term-disambiguation target)
- RFC-014c Workspace Support (the first consumer of the import surface; under review)
- RFC-036 Test Framework (source of Test role rules)
- #321 M2 Warning Code Independent Emission Channel (origin of Decision B and Plan A)
