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

## Summary

Fulfills the extension slot promised in RFC-029's "Sub-RFC Planning" section (slots 029b–029e are
taken, so the next available is 029f): defines a **compilation target role model** for source files
(five categories: Script / Bin / Lib / Test / Internal) and its **import surface semantics** —
inferred from entry-point reachability when no manifest is present (zero-configuration), or
explicitly declared via `[lib]` / `[[bin]]` / `[exports]` (fields already defined by RFC-015) when a
manifest exists. This fills four pending semantic gaps: what determines the cross-package import
surface, the scope of pub exemption for dead-code warnings (resolved as plan B in #321), the
relationship between `[exports]` and Registry reachability, and disambiguation between RFC-014b's
`[binaries]` (precompiled distribution artifacts) and bin-role source files.

## Motivation

### Host and Boundary Rationale

The entry selection and visibility decisions of RFC-029 (accepted) are the direct upstream of this
RFC:

> **Entry file selection** priority: 1. `[run].main` 2. The `path` of the first `[[bin]]` 3.
> `src/main.yx` (convention default). — RFC-029 §Inter-package Cycles: Revisited

> **Visibility**: does not exist. Scope + distribution boundary cover all scenarios; no new keyword
> is needed. — RFC-029 §Design Decision Records (2026-07-30)

The second quote is the design red line of this RFC: import surface semantics **must not introduce
new visibility keywords**; it can only be carried by the "distribution boundary" (file role +
declared export surface).

Four consumers each hit the same wall:

| Consumer                                           | Wall-hit point                                                                                                                                                                                                                                   |
| -------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| #321 Dead-code warnings                            | The rationale for plan B ("pub = external interface, never reported") is the lack of a role model to distinguish "truly external" from "looks external"; whether to report unused pub in bin (plan A) has been pending                           |
| RFC-014c Workspaces (under review, #113)           | When member packages `use` each other, what defines the import surface — `[exports]`, the single `[lib]` file, or all pub files — **not a single semantic clause in the entire document**                                                        |
| RFC-015 Configuration system (accepted)            | The `[lib]` / `[[bin]]` / `[exports]` fields are defined; aside from RFC-029 consuming the entry-point priority, the semantics of the remaining fields (especially the relationship between `[exports]` and Registry reachability) are unaligned |
| RFC-037 Packaging / RFC-029a Caching (draft, #293) | Packaging artifact selection and incremental recompilation cache unit boundaries both need the role model as a prerequisite                                                                                                                      |

### Current Problems

**Four semantic gaps** (inventory on 2026-09-12, confirmed by reviewing the entirety of
014/014a/014b/014c/015/029):

1. **Cross-package import surface is undefined**. The inter-member-package reference examples in
   014c only show directory structure (`src/lib.yx`); there is no clause stating what
   `use utils.helper` can obtain. The current implicit rule is "all top-level pub files of the
   package" — internal implementation files are dragged into the namespace too, making the
   dependency boundary nominal.
2. **No scope for the pub dead-code exemption**. #321's plan B "pub is never reported" is too
   conservative in single-file scripts (scripts have no external consumer, pub is meaningless) and
   too lenient in entry files of multi-file projects (unimported pub in the root file is dead code).
   Without a role model, the dividing line between plans A and B cannot be drawn.
3. **Three export concepts are unaligned**. `[exports]` (015, path mapping), `[lib]` (015, single
   file path), Registry reachability (029, reachable from entry along use) — whether they stand in
   subset, equality, or independent relations is unstated.
4. **Terminology collision**. RFC-014b's `[binaries]` refers to **precompiled distribution
   artifacts** (.so/exe download-priority strategy), which is a completely different layer from the
   "bin target (source role)". Once 014c lands, the two will inevitably appear on screen together;
   failure to disambiguate early will keep producing misunderstandings.

**Empirical evidence: the conservatism of plan B is already a real pain point**. The #321 M2
retrospective records: the structural reason for the silent W1001–W1005 family under default
configuration is exactly "all pub treated as external interfaces" — this is compensating behavior
for a missing model, not the end state.

## Proposal

### Core Design: File Role Model

**Roles are file-level attributes** (matching the field granularity of 015: `[lib]` is a single
file, `[[bin]]` is a file list, `[exports]` is a file mapping), not package-level switches.

| Role         | Determination                                                                                                                                                    | Semantics                                                                                                                                                                                                    |
| ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Bin**      | Files pointed to by `[run].main` / `[[bin]].path`; or (when no manifest) files containing `main` that are not `use`d by any other file                           | Program entry. Its `pub` **may be reported as dead code** (no external consumer); `main` is a reachability root                                                                                              |
| **Lib**      | Files hit by `[exports]` mappings; or `[lib].path`; or (when no manifest) files referenced by `use` from other files                                             | Distribution boundary. Its `pub` is **exempt from dead-code reporting** (external consumer is invisible; better to under-report); symbols exported through it are visible across packages                    |
| **Test**     | Files matched by RFC-036 test file rules (the `tests/` directory, `*_test.yx` and other existing conventions); **no** `[[test]]` explicit declaration is defined | Test code. Does not participate in dead-code judgment; its references count as reachability roots for the code under test                                                                                    |
| **Internal** | When the package has a manifest, files neither on the export surface nor containing `main`                                                                       | In-package implementation. `pub` exempt in phase 1 (tightened in phase 2 by in-package use-graph reachability, with Bin-role corpus verification as prerequisite); **not reachable** by out-of-package `use` |
| **Script**   | A file run directly as a single file (`run foo.yx`)                                                                                                              | Behavior identical to today: Registry only contains std, no dead-code pub semantic changes                                                                                                                   |

**Determination priority**: manifest explicit declaration > entry-reachability inference > 036 test
convention. Explicit declaration always wins — inference is only the default when nothing is
declared.

### Example

```toml
# yaoxiang.toml (all fields are already defined by RFC-015; this RFC adds no new configuration surface)
[lib]
path = "src/lib.yx"

[[bin]]
name = "my-cli"
path = "src/cli.yx"

[exports]
"." = "src/lib.yx"
"./internal-helper" = "src/helper.yx"   # ← cross-package visibility is determined by this, not pub
```

```yaoxiang
# src/cli.yx (Bin role)
pub unused_fn = (x: Int) => x    # ← W1001 may be reported: bin has no external consumer
main: () -> Void = { ... }

# src/lib.yx (Lib role, on the export surface)
pub api_fn = ...                 # ← never reported: external consumers are invisible

# src/other.yx (Internal role, not on the export surface)
pub semi_api = ...               # ← exempt in phase 1 (better to under-report), tightened in phase 2 by use-graph reachability
```

### Import Surface Semantics

Resolution order for cross-package `use pkg.x` (first match wins):

1. If an `[exports]` mapping exists → import surface = the set of files listed in the mapping; `x`
   must be among the top-level bindings of those files.
2. Otherwise if `[lib].path` exists → import surface = that single file.
3. If neither exists (no-manifest dependency, path depends on bare directory) → **status quo
   preserved**: all top-level files are importable (compatible with existing behavior; tightening to
   be discussed separately).

**Workspace members**: the export surface is defined solely by the member package's own manifest;
the workspace root must not override or extend the member's export surface — member self-containment
is the core design of 014c, and root overrides would break encapsulation.

Relationship to Registry reachability (aligned with 029): `[exports]`/`[lib]` define the **set of
legal starting points for cross-package resolution**; once the starting points are determined, the
Registry still expands per 029's "reachable from entry along `use`" — internal files `use`d by the
exported files enter through linking, but do not generate new cross-package starting points.

### Boundary with Existing Plans

| Neighbor                                          | Boundary                                                                                                                                                                                                                                                                         |
| ------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| RFC-029d "CLI `--entry` override entry" (planned) | 029f defines the role model; 029d consumes it — `--entry` is the CLI-level override for the Bin role                                                                                                                                                                             |
| RFC-014b `[binaries]`                             | **Terminology disambiguation**: `[binaries]` is precompiled distribution artifacts (download-priority strategy); `[[bin]]` is the source role declaration. The former lives in the dependency package's manifest, the latter in this package's manifest; they do not interchange |
| RFC-014c Workspaces                               | Inter-member-package import surface = the resolution order in this RFC; workspace coordination mechanisms (members, shared lockfile) still belong to 014c                                                                                                                        |
| RFC-036 Testing                                   | Test role determination directly adopts 036's existing file rules; no new conventions added                                                                                                                                                                                      |
| #321 Dead code                                    | Plan B (pub unconditionally exempt) is explicitly downgraded to "transitional semantics when no target exists"; Bin role's pub being reportable is the form in which this RFC delivers plan A                                                                                    |

## Detailed Design

### Role Determination Algorithm (in the orchestrator)

```
fn classify(files, manifest, entry_reach) -> Map<File, Role>:
    # 1. Explicit layer: manifest declarations take precedence
    for f in manifest.exports.values():  role[f] = Lib
    if manifest.lib_path:                role[lib_path] = Lib
    for b in manifest.binaries:          role[b.path] = Bin
    if manifest.run_main:                role[run_main] = Bin

    # 2. Inference layer: entry reachability for undeclared files
    for f in files where role[f] 未定:
        if f 被 ≥1 个非自身文件 use:      role[f] = Lib
        elif f 含 main 且无其他文件 use 它: role[f] = Bin
        else:                             role[f] = Internal

    # 3. Test layer: 036 rule match → Test (overrides Lib/Internal, not explicit Bin)
    apply_rfc036_test_rules(files, &role)

    # Single-file direct run: entire model bypassed, behavior unchanged
    if no manifest and single_file:      all Script
```

### Compiler Changes

- `frontend/config.rs`: manifest parsing adds the `[exports]` → role-table mapping (field parsing
  already exists in 015).
- `frontend/module/orchestrator.rs`: insert the role classification phase before Registry
  construction; cross-package resolution validates `use` targets per the import surface order;
  out-of-bounds reports the existing `module_not_found` family (no new error code; messages get a
  "not on the export surface" hint).
- `typecheck/passes/dead_code.rs`: the entry-point set changes from "main + all pub" to "main + pub
  of Lib-role files" (Bin is reportable immediately; Internal exempt in phase 1, see Implementation
  Strategy for Phase 2).
- LSP (RFC-017): completion/hover's "importable items" filtered by import surface; the `main`
  missing diagnostic is reported only for Bin-role files.

### Runtime Behavior

No changes. The role model is pure compile-time/parse-time semantics; it does not affect IR or
execution.

### Backward Compatibility

- No-manifest projects: behavior completely unchanged (the inference layer reproduces the status quo
  — `use`d files are Lib, files containing main are Bin). The increment in dead-code warnings is
  only in Bin files: unused pub in the root file with no importer changes from silent to W1001 —
  this is the expected behavior of #321's plan A, not a break.
- With-manifest projects: when `[exports]` is declared, the import surface narrows from "all pub
  files" to the declared surface. **This is the only behavior-narrowing point**: code depending on
  other packages' internal files will start reporting `module_not_found`. Given that the package
  ecosystem is not yet established (029: "currently no third-party package ecosystem"), the breakage
  surface is zero; nonetheless, a minor-version transition period is set (out-of-bounds first warns,
  then errors).

## Trade-offs

### Advantages

- Zero new configuration surface: all fields are defined by 015; this RFC only adds semantics.
- Zero new keywords: the import surface is carried by the distribution boundary, isomorphic to 029's
  "visibility does not exist" decision.
- Four consumers (#321 / 014c / 029a / 037) are modeled in one shot, no more walls to hit
  individually.
- Zero-cost no-manifest path: the zero-configuration gene of a scripting language is fully
  preserved.

### Disadvantages

- Role inference (use'd = Lib) classifies a utility file as Lib in the common shape of "entry file
  uses utility file", whose pub is then exempt — coarser than the ideal granularity. This is a
  deliberate choice in the under-report direction; tightening requires call-direction analysis on
  the use graph, which the first version does not do.
- Narrowing the import surface via `[exports]` is a breaking semantic tightening (despite the
  transition period).

## Alternatives

| Alternative                              | Description                                                                 | Reason not adopted                                                                                                                                                                                                                   |
| ---------------------------------------- | --------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Top-level RFC-040                        | Open a new top-level number                                                 | Module-graph semantics (import surface / reachability / entry) all live in 029's territory; splitting into top-level would create two top-level documents for the same object; and the 029b–029e slot pattern is already established |
| Attach to 014c                           | As a section of workspace                                                   | Dependency direction is reversed: language layer (029x) defines semantics, package layer (014x) consumes semantics. 014c is still under review; expanding its scope mid-review would slow its landing                                |
| Plan A as-is (#321 menu)                 | manifest requires `[[bin]]`/`[[lib]]` targets to enable dead-code semantics | Forces configuration for a narrow slice (unused pub in main-bearing root files); this RFC's inference layer achieves equal benefit with zero configuration                                                                           |
| Pure entry inference (no manifest layer) | Does not consume 015 fields                                                 | `[exports]` is an already-accepted field in 015, and the cross-package import surface cannot bypass it; not defining it would leave a gap for 014c                                                                                   |

## Implementation Strategy

### Phased

- **Phase 1**: role classification + import surface resolution + Bin pub reportable (Internal/Test
  stay exempt)
- **Phase 2**: Internal pub tightened to "reportable when unreachable in the in-package use graph" —
  prerequisite is post-Phase-1 Bin-role warning corpus verification with no false-positive
  regressions; triggered when ready, no fixed timeline

### Dependencies

- Prerequisites: RFC-029 orchestrator (landed), RFC-015 manifest fields (defined).
- Dependents: #321 (Bin pub reportable), RFC-014c (import surface), RFC-029a (roles as cache unit
  boundary), RFC-037 (artifact selection), RFC-029d (`--entry` override).
- Landing order with RFC-014c: before 014c review completes, 029f's import surface order should be
  finalized; 014c consumes it by reference, avoiding mid-review scope expansion.

### Risks

- Deviation between role inference and user intuition (trade-off disadvantage #1 above) → all
  incremental behavior is presented as warnings (non-blocking) and can be disabled at any time.
- Transition period management for `[exports]` narrowing → first warn within the minor version.

## Open Questions

All three draft-stage items are settled (2026-09-13, decisions recorded in Appendix B and the main
text); no outstanding items:

- [x] Whether to tighten Internal pub → **Decision**: tighten in phase 2 by in-package use-graph
      reachability, with Bin-role corpus verification as prerequisite (see Implementation Strategy
      Phase 2)
- [x] `[[test]]` explicit declaration → **Decision**: not introduced; Test role is determined solely
      by 036 rules; any future explicit-declaration need is left to 036's own extension
- [x] Workspace root overriding member export surface → **Decision**: not allowed; member
      self-containment is the core design of 014c, root override would break encapsulation

---

## Appendix B: Design Decision Records

> **Erratum (2026-09-17): The role model drives entry semantics**
>
> This RFC defines the five roles and the meaning of "who is the entry" (the Bin row's "`main` is
> the reachability root"), but the initial implementation only used roles for **dead-code
> warnings**; entry lookup remained independent of roles — when `find_entry_point` couldn't find
> `main`, it returned 0 and **silently executed the first function in the function table**. The
> top-level binding refactor (T4) wired the role model into entry determination, with the following
> specific rules:
>
> | Role                      | Entry rule                                                                                                                                                                                       |
> | ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
> | **Script** (no manifest)  | **No entry concept**. Top-level statements (including binding initializers) in source order form the program body; `main` is a regular binding, **not auto-invoked** — to run it, write `main()` |
> | **Bin** (with manifest)   | Requires `main` and it must be a function; otherwise compile error (E3020 missing main / E3021 main not a function). No executable statements allowed at the top level                           |
> | **Lib / Internal / Test** | No `main` required (they are not entries)                                                                                                                                                        |
>
> **Two differences from the initial description in this RFC**:
>
> 1. The initial Script row said "behavior identical to today". Today's behavior contained two
>    silent defects, both fixed in T4: ① when there's no `main`, execute the first function in the
>    function table; ② top-level statements reported E3005 as not executable. The current Script's
>    top-level statements **are executable** — this is a new capability, not a status-quo
>    preservation.
> 2. "Script does not special-case `main`" and "Bin treats `main` as the entry" must be **mutually
>    exclusive**: if Script both executes top-level statements and implicitly invokes `main`, a
>    script that explicitly writes `main()` would double-run (#356). Hence Script has only one
>    execution entry — top-level statements.
>
> **Distinction between role determination and entry determination** (implementation note): entry
> determination uses "whether a manifest exists" (`find_project_root().is_some()`) rather than
> `roles::classify()`. Reason: classify answers "who consumes this file" (serving dead-code
> analysis); a file with a manifest but no `main` is Internal in classify (it is not `use`d by other
> files, which is correct), but if the user runs it as an entry, there must be an entry. The two
> questions are different, so the criteria differ.
>
> See `docs/src/reference/language-spec/syntax.md` §3.11 for details.

| Decision                                               | Determination                                                                                        | Date       | Recorder | Rationale                                                                                                           |
| ------------------------------------------------------ | ---------------------------------------------------------------------------------------------------- | ---------- | -------- | ------------------------------------------------------------------------------------------------------------------- |
| Numbering                                              | 029f (029b reserved for hot reload, 029c deleted, 029d/029e occupied)                                | 2026-09-12 | Chenxu   | RFC-029 §Sub-RFC Planning                                                                                           |
| Belongs to 029 family, not a top-level RFC             | Yes                                                                                                  | 2026-09-12 | Chenxu   | Import surface / reachability / entry are all module-graph semantics; 029 already consumes `[[bin]]` entry priority |
| Visibility keyword                                     | Not introduced                                                                                       | 2026-09-12 | Chenxu   | Isomorphic to RFC-029's "visibility does not exist" decision; import surface carried by distribution boundary       |
| Status of #321 plan B                                  | Downgraded to "transitional semantics when no target exists"                                         | 2026-09-12 | Chenxu   | Bin role's pub being reportable is the delivery form of #321 plan A                                                 |
| Role granularity                                       | File-level                                                                                           | 2026-09-12 | Chenxu   | Matches 015's field granularity (`[lib]` single file, `[[bin]]` list, `[exports]` mapping)                          |
| `[binaries]` disambiguation                            | 014b precompiled distribution artifact ≠ `[[bin]]` source role                                       | 2026-09-12 | Chenxu   | Two distinct concepts (download strategy vs. compilation role); nail down before 014c lands                         |
| Internal pub tightening                                | Tighten in phase 2 by in-package use-graph reachability; prerequisite = Bin-role corpus verification | 2026-09-13 | Chenxu   | Better to under-report, then tighten gradually; corpus-driven to avoid premature false positives                    |
| `[[test]]` explicit declaration                        | Not introduced; Test determined solely by 036 rules                                                  | 2026-09-13 | Chenxu   | Don't add configuration surface before 036 has a need                                                               |
| Workspace root overriding member export surface        | Not allowed                                                                                          | 2026-09-13 | Chenxu   | Member self-containment is the core design of 014c; root override breaks encapsulation                              |
| Whether `main` is auto-invoked under Script            | **Not auto-invoked** — top-level statements are the program, `main()` must be explicit               | 2026-09-17 | Chenxu   | Having both rules would double-run explicit `main()` (#356); Script can only have one execution entry               |
| Whether the role model is used for entry determination | **Yes** (previously only used for dead-code warnings)                                                | 2026-09-17 | Chenxu   | This RFC defines Bin's "`main` is the reachability root"; without wiring it to entry, that semantics idles          |

## Appendix C: Glossary

| Term                  | Definition                                                                                                                                       |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| Role                  | The compilation target identity of a source file: Script / Bin / Lib / Test / Internal                                                           |
| Import Surface        | The set of files that are legal resolution starting points for cross-package `use`, declared by `[exports]`/`[lib]` or inferred                  |
| Distribution Boundary | 029 terminology: the externally exposed scope of a package; this RFC refines it from the implicit "all pub files" to the explicit import surface |
| Script State          | The bypass form for no-manifest single-file direct runs; all behavior matches the status quo                                                     |

## References

- RFC-029 Module Semantics (parent RFC: entry selection, visibility decision, sub-RFC planning)
- RFC-015 Configuration system (`[lib]`/`[[bin]]`/`[exports]` field definitions)
- RFC-014b Build system and binary distribution (`[binaries]`, the disambiguation target)
- RFC-014c Workspace support (first consumer of import surface, under review)
- RFC-036 Testing framework (source of Test role rules)
- #321 M2 Independent emission channels for warning codes (origin of plan B and plan A)
