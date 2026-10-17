---
title: 'RFC-029f: Compilation Target Roles and Import Surface Semantics'
status: 'Accepted'
author: 'Chenxu'
created: '2026-09-12'
updated: '2026-09-13'
accepted: '2026-09-13'
issue: '#334'
---

# RFC-029f: Compilation Target Roles and Import Surface Semantics

## Abstract

> **Implementation Amendment (2026-10-08, RFC-029g)**: The `pub` keyword has been entirely removed
> by RFC-029g. Throughout the body of this document, all expressions of "`pub` exemption / `pub`
> files / unused-pub warning" should be read as "**top-level bindings + intra-package reference
> pool**": the role classification (Script / Bin / Lib / Internal / Test) and import surface
> semantics are unchanged; only the dead-code exemption criteria differ (see
> [029g](029g-remove-pub-and-auto-bind.md) and the criteria in
> `docs/src/reference/warning-code/warning-codes.md`).

Fulfills the extension slot of RFC-029's "sub-RFC planning" (029b–029e already taken, extended to
029f): defines a **compilation target role model** (five categories: Script / Bin / Lib / Test /
Internal) for source files and its **import surface semantics** — when no manifest is present,
inferred from entry-point reachability (zero-config); when a manifest is present, declared
explicitly via `[lib]` / `[[bin]]` / `[exports]` (fields already defined by RFC-015). Fills four
open semantic gaps: how the cross-package import surface is computed, the applicability domain of
the pub exemption for dead-code warnings (Issue #321 Decision B), the relationship between
`[exports]` and Registry reachability, and the terminology disambiguation between RFC-014b's
`[binaries]` (precompiled distribution artifacts) and bin-role source files.

## Motivation

### Host and Boundary Basis

RFC-029 (Accepted)'s entry-point selection and visibility decisions are the direct upstream of this
RFC:

> **Entry-file selection** priority: 1. `[run].main` 2. The `path` of the first `[[bin]]` 3.
> `src/main.yx` (conventional default). — RFC-029 §Cross-package Cycles: Revisted

> **Visibility**: does not exist. Scope + distribution boundary cover all scenarios; no new keyword
> needed. — RFC-029 §Design Decision Log (2026-07-30)

The second quote is this RFC's design red line: the import surface semantics **must not introduce
new visibility keywords**; it can only be borne by the "distribution boundary" (file role + declared
export surface).

The four consumers each hit the same wall:

| Consumer                                           | Wall                                                                                                                                                                                                                                        |
| -------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| #321 Dead Code Warning                             | Decision B "pub = external interface, never reported" exists because there is no role model to distinguish "truly external" from "looks external"; whether to report unused pub inside bin (Plan A) has been pending                        |
| RFC-014c Workspaces (Under Review, #113)           | What counts as the import surface when member packages `use` each other — `[exports]`, single-file `[lib]`, or all pub files — **not a single semantic clause in the entire document**                                                      |
| RFC-015 Configuration System (Accepted)            | `[lib]` / `[[bin]]` / `[exports]` fields are defined; apart from RFC-029 consuming the entry priority, no one has aligned the semantics of the remaining fields (especially the relationship between `[exports]` and Registry reachability) |
| RFC-037 Packaging / RFC-029a Caching (Draft, #293) | Choice of packaging artifacts, cache unit boundaries for incremental recompilation, all need the role model as a prerequisite                                                                                                               |

### Current Problems

**Four Semantic Gaps** (inventory on 2026-09-12, confirmed by reviewing the full texts of
014/014a/014b/014c/015/029):

1. **Cross-package import surface undefined**. 014c's member-package mutual reference example only
   has a directory structure (`src/lib.yx`); there is no clause stating what `use utils.helper` can
   access. The current implicit rule is "all top-level pub files of that package" — internal
   implementation files are dragged into the namespace too, making the dependency boundary
   effectively non-existent.
2. **Dead-code pub exemption has no applicability domain**. #321 Decision B's "pub never reported"
   is overly conservative in single-file scripts (scripts have no external consumers, pub is
   meaningless) and overly permissive in the entry file of multi-file projects (a root file's pub
   with no one importing it is dead code). Without a role model, the dividing line between Plans A
   and B cannot be drawn.
3. **Three export concepts unaligned**. `[exports]` (015, path mapping), `[lib]` (015, single-file
   path), and Registry reachability (029, reachable from entry along `use`) — whether these three
   are in inclusion relation, equality relation, or each governs its own domain, there is no clause.
4. **Terminology collision**. RFC-014b's `[binaries]` is **precompiled distribution artifacts**
   (.so/exe download priority strategy), which is an entirely different level from "bin target
   (source code role)". Once 014c lands, the two will inevitably appear on screen together; failure
   to disambiguate early will keep producing misunderstandings.

**Empirical evidence: the conservatism of Decision B is already a real pain point**. #321 M2
retrospective records: the structural reason that the entire W1001–W1005 family is silent under
default configuration is precisely that "all pub are treated as external interfaces" — this is a
compensating behavior for a missing model, not the end state.

## Proposal

### Core Design: File Role Model

**Roles are file-level properties** (matching the field granularity of 015: `[lib]` is a single
file, `[[bin]]` is a file list, `[exports]` is a file mapping), not package-level switches.

| Role         | Determination                                                                                                                                              | Semantics                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Bin**      | The file pointed to by `[run].main` / `[[bin]].path`; or, when no manifest is present, a file containing `main` and not used by any other file             | Program entry. Requires a binding named `main`, either a value or a function: a function main is called with zero arguments at entry; a value main is evaluated during initialization and executed (the evaluation result, even if it is a function value, is not called again). Missing `main` is a compile error (E3020); top-level executable statements are forbidden. Its `pub` **may be reported as dead code** (no consumers outside the package); `main` is the reachability root |
| **Lib**      | Files hit by the `[exports]` mapping; or `[lib].path`; or, when no manifest is present, files referenced by `use` in other files                           | Distribution boundary. Its `pub` **is exempt from dead-code reporting** (consumers outside the package are invisible, prefer under-reporting); symbols exported through it are visible across packages                                                                                                                                                                                                                                                                                    |
| **Test**     | Files hit by RFC-036 test file rules (the `tests/` directory, `*_test.yx` and other existing conventions); does not set an explicit `[[test]]` declaration | Test code. Does not participate in dead-code determination; its references count as reachability roots for the code under test                                                                                                                                                                                                                                                                                                                                                            |
| **Internal** | When the package has a manifest, files that are neither on the export surface nor contain `main`                                                           | Intra-package implementation. `pub` exempt until Phase 2 (tightened by intra-package `use` graph reachability, contingent on Bin-role corpus validation); not reachable by `use` from outside the package                                                                                                                                                                                                                                                                                 |
| **Script**   | A file run directly as a single file (`run foo.yx`)                                                                                                        | **No entry concept**. Top-level statements (including binding initialization) are the program body in source order; `main` is an ordinary binding, **not auto-called** — to run, write `main()`. Registry contains only std                                                                                                                                                                                                                                                               |

**Determination priority**: explicit manifest declarations > entry-reachability inference > RFC-036
test conventions. Explicit declarations always win — inference is only the default when nothing is
declared.

### Entry Determination

The role model is not only used for dead-code analysis, but also **drives entry-point
determination**. The entry rules for each role:

| Role                      | Entry rule                                                                                                                                                                                                                                                        |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Script** (no manifest)  | **No entry concept**. Top-level statements (including binding initialization) are the program body in source order; `main` is an ordinary binding, **not auto-called** — to run, write `main()`                                                                   |
| **Bin** (with manifest)   | Requires a binding named `main` (either a value or a function: a value main is evaluated during initialization, a function main is called with zero arguments at entry); missing `main` is a compile error (E3020). Top-level executable statements are forbidden |
| **Lib / Internal / Test** | Does not require `main` (they are not entry points)                                                                                                                                                                                                               |

**Script and Bin entries must be mutually exclusive**: "Under Script, `main` is not special" and
"Under Bin, `main` is the entry" cannot both hold — if a Script both executes top-level statements
and implicitly calls `main`, a script that explicitly writes `main()` would run twice (#356).
Therefore Script has only one execution entry: the top-level statements.

**The difference between role determination and entry determination** (implementation note): entry
determination uses "whether a manifest exists" (`find_project_root().is_some()`) rather than
`roles::classify()`. Reason: classify answers "who consumes this file" (serving dead-code analysis);
a file with a manifest but no `main` is classified as Internal (it is not used by other files, which
is reasonable), but the user runs it as an entry, so an entry must exist. The two questions are
different, so the criteria differ accordingly.

**Execution form of value `main`** (#388 Decision): a function is already a value (unified at the
type layer); entry is determined by **binding existence** rather than callability — value main and
function main differ only in evaluation strategy. Value main does not set an entry call (#356
anti-double-run: initialization and entry call never stack), sharing Script's execution form; its
evaluation is part of the initialization sequence: executed in global binding topological order,
independent bindings in source order. Therefore, wrapping Script's top-level statements in
`main = { ... }` and migrating to Bin is **not an order-preserving migration** — loose statements
execute in source order, binding initialization in topological order; initializations that need to
come first should have `main` explicitly depend on them. A value main of a non-callable form like
`main: Int = 5` is a legal but observationally-empty program (analogous to Rust's empty
`fn main() {}`).

See `docs/src/reference/language-spec/syntax.md` §3.11 for details.

### Example

```toml
# yaoxiang.toml (all fields are defined by RFC-015; this RFC adds no new configuration surface)
[lib]
path = "src/lib.yx"

[[bin]]
name = "my-cli"
path = "src/cli.yx"

[exports]
"." = "src/lib.yx"
"./internal-helper" = "src/helper.yx"   # ← it is this, not pub, that determines cross-package visibility
```

```yaoxiang
# src/cli.yx (Bin role)
pub unused_fn = (x: Int) => x    # ← W1001 may be reported: bin has no consumers outside the package
main: () -> Void = { ... }

# src/lib.yx (Lib role, on the export surface)
pub api_fn = ...                 # ← never reported: consumers outside the package are invisible

# src/other.yx (Internal role, not on the export surface)
pub semi_api = ...               # ← exempt until Phase 2 (prefer under-reporting); tightened by use graph at that time
```

### Import Surface Semantics

Cross-package `use pkg.x` resolution order (first hit takes effect):

1. `[exports]` mapping exists → import surface = the file set listed in the mapping; `x` must be
   among the top-level bindings of those files;
2. Otherwise, `[lib].path` exists → import surface = that single file;
3. Neither exists (no-manifest dependency, path-depends-on-bare-directory) → **Maintain status
   quo**: all top-level files are importable (compatible with existing behavior; tightening is a
   separate discussion).

**Workspace members**: the export surface is defined only by the member package's own manifest; the
workspace root must not override or extend the member's export surface — member self-containment is
014c's core design, and root override would break encapsulation.

Relationship to Registry reachability (aligned with 029): `[exports]`/`[lib]` define the **set of
legal starting points for cross-package resolution**; once the starting points are determined, the
Registry still expands per 029's "reachable from entry along `use`" — internal files `use`d by the
export file itself enter via linking, but do not produce new cross-package starting points.

### Boundaries with Existing Plans

| Neighbor                                          | Boundary                                                                                                                                                                                                                                                                          |
| ------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| RFC-029d "CLI `--entry` entry override" (planned) | 029f defines the role model; 029d consumes it — `--entry` is a CLI-level override for the Bin role                                                                                                                                                                                |
| RFC-014b `[binaries]`                             | **Terminology disambiguation**: `[binaries]` is precompiled distribution artifacts (download priority strategy), `[[bin]]` is a source code role declaration. The former exists in the dependency package's manifest, the latter in this package's manifest — not interchangeable |
| RFC-014c Workspaces                               | The import surface for member-package mutual reference = this RFC's resolution order; workspace coordination mechanisms (members, shared lockfile) still belong to 014c                                                                                                           |
| RFC-036 Testing                                   | Test role determination directly adopts 036's existing file rules; no new convention is added                                                                                                                                                                                     |
| #321 Dead Code                                    | Decision B (pub uniformly exempt) is explicitly downgraded to "transitional semantics when there is no target"; Bin role's pub being reportable is this RFC's fulfillment of Plan A                                                                                               |

## Detailed Design

### Role Determination Algorithm (Inside the Orchestrator)

```
fn classify(files, manifest, entry_reach) -> Map<File, Role>:
    # 1. Explicit layer: manifest declarations take priority
    for f in manifest.exports.values():  role[f] = Lib
    if manifest.lib_path:                role[lib_path] = Lib
    for b in manifest.binaries:          role[b.path] = Bin
    if manifest.run_main:                role[run_main] = Bin

    # 2. Inference layer: entry reachability for undeclared spots
    for f in files where role[f] is unset:
        if f is used by ≥1 non-self file:                  role[f] = Lib
        elif f contains main and no other file uses it:    role[f] = Bin
        else:                                              role[f] = Internal

    # 3. Test layer: 036 rules hit → Test (overrides Lib/Internal, not explicit Bin)
    apply_rfc036_test_rules(files, &role)

    # Single-file direct run: the whole model is bypassed, behavior unchanged
    if no manifest and single_file:       all Script
```

### Compiler Changes

- `frontend/config.rs`: manifest parsing adds the `[exports]` → role-table mapping (field parsing
  already in 015).
- `frontend/module/orchestrator.rs`: insert a role classification phase before Registry
  construction; cross-package resolution validates `use` targets per the import-surface order;
  out-of-bound accesses report the existing `module_not_found` family (no new error code; the
  message gains a "not on the export surface" hint).
- `typecheck/passes/dead_code.rs`: the entry-point set changes from "main + all pub" to "main + Lib
  role files' pub" (Bin is reportable immediately; Internal exempt until Phase 2, see Implementation
  Strategy).
- LSP (RFC-017): completion/hover's "importable items" is filtered by the import surface; the
  `main`-missing diagnostic is reported only for Bin-role files.

### Runtime Behavior

No change. The role model is purely compile-time/parse-time semantics; it does not affect the IR or
execution.

### Backward Compatibility

- Projects without a manifest: behavior is completely unchanged (the inference layer restores the
  status quo — files used by others are Lib, files containing main are Bin). The dead-code warning
  increment is only in Bin files: unused pub in the root file that no one imports goes from silent
  to W1001 — this is the expected behavior of #321 Plan A, not a breaking change.
- Projects with a manifest: when `[exports]` is already declared, the import surface narrows from
  "all pub files" to the declared surface. **This is the only behavior narrowing point**: code that
  depends on other packages' internal files will begin to report `module_not_found`. Given that the
  package ecosystem has not yet been established (029: "currently no third-party package
  ecosystem"), the breaking surface is zero; a minor-version transition period is still set
  (out-of-bound accesses first warn, then error).

## Trade-offs

### Pros

- Zero new configuration surface: all fields are already defined by 015; this RFC only adds
  semantics.
- Zero new keywords: the import surface is borne by the distribution boundary, isomorphic to 029's
  "visibility does not exist" decision.
- The four consumers (#321 / 014c / 029a / 037) are modeled in one pass, no longer each hitting
  their own wall.
- Zero cost on the no-manifest path: the zero-config gene of a scripting language is fully
  preserved.

### Cons

- Role inference (used by others = Lib) classifies utility files as Lib under the common form of
  "entry file uses utility files", whose pub is exempt — wider than ideal granularity. This is a
  deliberate choice in the under-reporting direction; tightening requires call-direction analysis on
  the use graph, not done in v1.
- Narrowing the import surface via `[exports]` is a destructive semantic tightening (though with a
  transition period).

## Alternatives

| Plan                                     | Description                                                                 | Reason not adopted                                                                                                                                                                                                                      |
| ---------------------------------------- | --------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Top-level RFC-040                        | Open a new top-level number                                                 | Module-graph semantics (import surface / reachability / entry) all live in 029's territory; splitting to the top level would produce two top-level documents for the same object; and the 029b–029e slot pattern is already established |
| Attach to 014c                           | As a section of the workspace document                                      | Dependency direction is reversed: the language layer (029x) defines semantics, the package layer (014x) consumes them. 014c is still under review; expanding scope mid-review delays its landing                                        |
| Plan A as-is (#321 menu)                 | Manifest mandatorily has `[[bin]]`/`[[lib]]` target for dead-code semantics | Introduces mandatory configuration for a narrow slice (unused pub in the main-bearing root file); this RFC's inference layer achieves the same gain with zero configuration                                                             |
| Pure entry inference (no manifest layer) | Does not consume 015 fields                                                 | `[exports]` is already an accepted field in 015; cross-package import surfaces cannot bypass it; leaving it undefined is leaving a gap for 014c                                                                                         |

## Implementation Strategy

### Phased Rollout

- **Phase 1**: role classification + import surface resolution + Bin pub reportable (Internal/Test
  remain exempt)
- **Phase 2**: Internal pub tightened to "reportable when not reachable in the intra-package use
  graph" — precondition is the Bin-role warning corpus validation post Phase 1 showing no
  false-positive callbacks; triggered when ready, no fixed schedule

### Dependencies

- Prerequisite: RFC-029 orchestrator (landed), RFC-015 manifest fields (defined).
- Depended on by: #321 (Bin pub reportable), RFC-014c (import surface), RFC-029a (role as cache unit
  boundary), RFC-037 (artifact selection), RFC-029d (`--entry` override).
- Landing order with RFC-014c: before 014c is approved, 029f's import-surface order should be
  finalized; 014c consumes it by reference, avoiding mid-review scope expansion.

### Risks

- Divergence between role inference and user intuition (first drawback above) → present all
  incremental behavior as warnings (non-blocking); can be disabled at any time.
- Transition period management for `[exports]` narrowing → warnings first within the minor version.

## Open Questions

All three items from the draft stage have been settled (2026-09-13, decisions entered into Appendix
B and the body); no open items:

- [x] Whether Internal pub should be tightened → **Decision**: tightened in Phase 2 by intra-package
      use-graph reachability, with Bin-role corpus validation as the prerequisite (see
      Implementation Strategy Phase 2)
- [x] Explicit `[[test]]` declaration → **Decision**: not introduced; the Test role is determined
      solely by 036 rules; if 036 later needs explicit declaration, it will extend itself
- [x] Whether the workspace root can override member export surfaces → **Decision**: not allowed;
      member self-containment is 014c's core design, and root override would break encapsulation

---

## Appendix B: Design Decision Log

| Decision                                               | Resolution                                                                                         | Date       | Recorder | Basis                                                                                                                     |
| ------------------------------------------------------ | -------------------------------------------------------------------------------------------------- | ---------- | -------- | ------------------------------------------------------------------------------------------------------------------------- |
| Numbering                                              | 029f (029b reserved for hot reload, 029c deleted, 029d/029e already placed)                        | 2026-09-12 | Chenxu   | RFC-029 §Sub-RFC Planning                                                                                                 |
| Belongs to the 029 family, not a top-level RFC         | Yes                                                                                                | 2026-09-12 | Chenxu   | Import surface / reachability / entry all belong to module-graph semantics; 029 already consumes `[[bin]]` entry priority |
| Visibility keyword                                     | Not introduced                                                                                     | 2026-09-12 | Chenxu   | Isomorphic with RFC-029's "visibility does not exist" decision; import surface is borne by the distribution boundary      |
| Status of #321 Decision B                              | Downgraded to "transitional semantics when there is no target"                                     | 2026-09-12 | Chenxu   | Bin-role pub being reportable is the fulfillment of #321 Plan A                                                           |
| Role granularity                                       | File-level                                                                                         | 2026-09-12 | Chenxu   | 015 field granularity (`[lib]` single file, `[[bin]]` list, `[exports]` mapping)                                          |
| `[binaries]` disambiguation                            | 014b precompiled distribution artifacts ≠ `[[bin]]` source code role                               | 2026-09-12 | Chenxu   | Two-layer concept (download strategy vs compilation role); pinned down before 014c lands                                  |
| Internal pub tightening                                | Phase 2 tighten by intra-package use-graph reachability, precondition = Bin-role corpus validation | 2026-09-13 | Chenxu   | Prefer under-reporting, gradual tightening; corpus-driven to avoid premature false positives                              |
| Explicit `[[test]]` declaration                        | Not introduced; Test is determined solely by 036 rules                                             | 2026-09-13 | Chenxu   | No configuration surface added until 036 needs it                                                                         |
| Workspace root overriding member export surface        | Not allowed                                                                                        | 2026-09-13 | Chenxu   | Member self-containment is 014c's core design; root override breaks encapsulation                                         |
| Whether `main` is auto-called under Script             | **No** — top-level statements are the program; `main()` must be explicit                           | 2026-09-17 | Chenxu   | Coexistence of the two rules would cause explicit `main()` to run twice (#356); Script can have only one execution entry  |
| Whether the role model is used for entry determination | **Yes** (previously only for dead-code warnings)                                                   | 2026-09-17 | Chenxu   | This RFC defines Bin's "`main` as reachability root"; without entry handling that semantics would be idle                 |

## Appendix C: Glossary

| Term                  | Definition                                                                                                                           |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| Role                  | A source file's compilation target identity: Script / Bin / Lib / Test / Internal                                                    |
| Import Surface        | The file set of legal resolution starting points for cross-package `use`, declared or inferred via `[exports]`/`[lib]`               |
| Distribution Boundary | 029 terminology: the package's externally exposed scope; this RFC refines it from the implicit (all pub files) to the import surface |
| Script State          | The bypass form of single-file direct run without a manifest; all behavior matches the status quo                                    |

## References

- RFC-029 Module Semantics (parent RFC: entry selection, visibility decision, sub-RFC planning)
- RFC-015 Configuration System (definitions of `[lib]`/`[[bin]]`/`[exports]` fields)
- RFC-014b Build System and Binary Distribution (`[binaries]`, terminology disambiguation target)
- RFC-014c Workspace Support (first consumer of the import surface, under review)
- RFC-036 Testing Framework (source of the Test role rules)
- #321 M2 Independent Emission Channel for Warning Codes (origin of Decision B and Plan A)

## Known Defects: Landed (2026-10-16, WBS 4.9 Round 1)

> The previous version (2026-10-09) listed the following three items as "Three Design Defects in the
> Reference Pool". All three have been landed item by item in WBS 4.9 Round 1 (commit `3863a9ff0`);
> `collect_project_refs` and `orchestrator.rs` have been split and moved into
> `src/driver/{state,arms,helpers}.rs` along with P4 4.2.9; all file:line values in the table below
> are retaken on the referenced revision.

### The Three Design Defects of the Reference Pool — Correction Status

| #   | Original Defect                                                 | Current Status (2026-10-16)                                                                                                                                                                                                                                                                                                                                                                                      | Landed Code (rev `3863a9ff0`)                                                                                                           |
| --- | --------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | Scan surface is a directory rather than the compilation surface | **Pool root = package target**: `collect_package_refs` (`helpers.rs:481`) invokes `discover_with_used` based on the `[[bin]]`/`[lib]`/`[exports]`/`[run].main` roots from `roles::ExplicitSurfaces` to obtain the compilation set; **does not scan directories to find entries**. When all roots fail, it falls back to the inspected file's own compilation set (`collect_compilation_refs` / `helpers.rs:454`) | `src/driver/helpers.rs:454-512` (fallback path); `src/driver/helpers.rs:481-512` (manifest-declared surface path)                       |
| 2   | Granularity is project-level bare names, no module dimension    | **Pool type = `HashMap<String, HashSet<String>>`**: key = module key, value = that module's reference name set; the consumer in `arms.rs:603-609` unions all module entries except the current unit and passes them to `DeadCodeAnalyzer`; module's own names (top-level definitions and local bindings; `use`-imported names are exceptions) no longer exempt other modules' same-name definitions              | `src/driver/arms.rs:603-609` (consumer `other_module_refs`); single assembly entry `src/driver/helpers.rs:429-449` (`collect_ref_pool`) |
| 3   | Script reuses the Lib pool (violates single-file semantics)     | **Script does not attach to the pool**: the `match us.role` at `arms.rs:611-623` only assembles for `Bin`/`Internal` (`set_exempt_pub`) and `Lib` (`set_cross_file_refs`); Script/Test take the `_ => {}` default arm; the latter only judges per the file-internal definition-use graph                                                                                                                         | `src/driver/arms.rs:611-623`                                                                                                            |

**Follow-up Improvements 4 / 5 — Correction Status (2026-10-16, WBS 4.9 Round 2)**

> The two items in this section were originally listed as "pending P4 4.9 landing". Both have been
> landed item by item in WBS 4.9 Round 2; the file:line values in the table below are taken on
> **this landing's working tree** (the commit hash will be supplemented after the batch is closed
> and pushed, see 09a — Round 1 lesson: local hashes before push get rewritten by `pull --rebase`).

| #   | Original Improvement                                                                                                                | Current Status (2026-10-16)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Landed Code                                                                                                                                                                                                                                                                                    |
| --- | ----------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 4   | Consumption definition refinement: distinguish "imported" from "actually used after import" (only the latter counts as consumption) | **Consumption criterion = "actually used after import"**: a consumption record = a module uses `use` to bring name n from module M into scope **and actually references n at a use site**, recorded only against M (`RefPool::consumed_from`); appearing only in the use item (no use site), or other modules' references to a **same-name** identifier do not count as consumption. The target module of an import is determined by the module key set at discovery time (`resolve_import_module`, **does not spin up a separate path resolution** — reuses the discovery-time resolution artifacts delivered by 4.6.1, see `39546020`); namespace form (`use lib` + `lib.foo()`) is attributed the same way | `src/driver/helpers.rs:442-446` (`RefPool`); `src/driver/helpers.rs:449-524` (`collect_ref_pool` + `resolve_import_module`); `src/frontend/core/typecheck/passes/dead_code.rs:180-192` (`ModuleNames`), `:328-352` (`collect_module_names`), `:122-176` (`ImportFace` / `collect_import_face`) |
| 5   | Test pool separation: tests/ references do not exempt production code (src/)                                                        | **`FileRole::Test` does not contribute to the consumption surface**: test module references form a separate pool (the test itself does not participate in dead-code judgment, so its references do not leak out); production code definitions are only saved by production code's consumption. The role is determined by a single point `roles::classify` (test discovery rules per RFC-036)                                                                                                                                                                                                                                                                                                                  | `src/driver/helpers.rs:470-476` (Test short-circuit in `collect_ref_pool`); `src/driver/helpers.rs:595-601` (single-point `roles::classify`); `src/driver/arms.rs:562-580` (both fallback/declared-surface paths feed the role into the pool)                                                  |

**Residual Surface (logged, not blocking)**: ① Field/method-position references (`obj.render()` only
has the short name) cannot be attributed at the AST layer to the module owning the receiver type —
only when the base name hits a namespace import is it attributed by import; the rest is
conservatively entered under the name dimension into `RefPool::field_names` (`collect_ref_pool` in
`src/driver/helpers.rs`); ② Method reachability: **within the same unit** narrows per candidate by
D20 resolution artifacts (`resolved_method_marks`; sibling overloads not hit still honestly report
W1005); **method calls across units (registry resolution) produce no resolution entries** (WBS 4.9.7
/ issue #464) ⇒ that name group falls back to the pool's **name-level exemption**
(`RefPool::field_names` flat global) ⇒ same-name same-order methods across units were previously not
reported; this gap is logged as WBS 4.9.7 / issue #464 (capability extension, not in this round).
Fallback boundary: when a name group has no resolution record in the current unit (such as call-site
ambiguity E1107, call forms bypassing resolution), fall back to short-name co-judgment (prefer
under-reporting direction) — therefore we do not claim "all uncalled methods are reported honestly".

### Reference Count Distortion (WBS 4.9.6 / issue #450) — Correction Status (2026-10-16)

> Originally logged: overloaded methods **are called and resolution is effective, yet still falsely
> reported as `W1005 Unused method`** (the reference count is distorted for overloaded calls). The
> 2026-10-16 user ruling A logged it as an independent tier-3 task and merged it into WBS 4.9 Round
> 2; this item is category D (removing false positives changes the diagnostic surface), authorized
> for landing.

| Stage                   | Original Defect                                                                                                                                                                                                                                               | Current Status                                                                                                                                                                                                                                                                                                                                                                                                           | Landed Code                                                                                             |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------- |
| Reachability probe      | The scan looks up references by **qualified name** (`Point.render`), while the reference points collected are **short names** (`obj.render()` only has `render`) ⇒ called methods are never reachable; in the no-project-root case, W1005 is falsely reported | Methods use a **short-name probe** to determine the call surface, and **D20 resolution artifacts** narrow it per candidate (`resolved_methods`; sibling overloads not hit by resolution still report W1005). Scope boundary: per-candidate holds within the same unit; **cross-unit (registry resolution) calls produce no resolution entries** ⇒ fall back to name-level exemption (issue #464, see Residual Surface ②) | `src/frontend/core/typecheck/passes/dead_code.rs:733-744` (probe); `:766-770` (per-definition judgment) |
| Definition-table keying | `all_defs` is keyed by qualified name ⇒ **same-name overloads shadow each other** (only one of two overloads remains, only 1 reported)                                                                                                                        | Definition table is **one entry per definition**: methods are appended; same-name non-method bindings are deduplicated by name, keeping the last (overloads are independent definitions; top-level reassignment is the same binding)                                                                                                                                                                                     | `src/frontend/core/typecheck/passes/dead_code.rs:304-322` (`record_definition`), `:28-29` (fields)      |

**Reason for not changing in P3.5 (consistent with the previous version)**: module-level resolution
requires path resolution for `use` items to be complete (`use a::foo` → foo comes from module a),
which falls under the P4-phase contract and Driver unification scope.
