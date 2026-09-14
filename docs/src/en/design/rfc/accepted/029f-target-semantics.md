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

Fulfills the extension slot of RFC-029 "Sub-RFC Planning" (slots 029b–029e are already taken,
extending to 029f): defines for source files a **compilation target role model** (five types: Script
/ Bin / Lib / Test / Internal) and its **import surface semantics**—when no manifest exists,
inferred by entry reachability (zero configuration); when a manifest exists, explicitly declared via
`[lib]` / `[[bin]]` / `[exports]` (fields already defined by RFC-015). Fills four dangling semantic
gaps: how the cross-package import surface is computed, the applicability scope of the pub exemption
for dead code warnings (Final Decision B from #321), the relationship between `[exports]` and
Registry reachability, and the noun disambiguation between RFC-014b `[binaries]` (precompiled
distribution artifacts) and the bin-role source files.

## Motivation

### Host and Boundary Basis

The entry selection and visibility decisions of RFC-029 (accepted) are the direct upstream of this
RFC:

> **Entry file selection** priority: 1. `[run].main` 2. The first `[[bin]]`'s `path` 3.
> `src/main.yx` (default convention). — RFC-029 §Inter-package Cycles: Revisted

> **Visibility**: does not exist. Scope + distribution boundary cover all scenarios; no new keyword
> needed. — RFC-029 §Design Decision Records (2026-07-30)

The second item is the design red line of this RFC: import surface semantics **must not introduce
new visibility keywords**; it can only be carried by "distribution boundary" (file role + declared
export surface).

Four consumers each ran into the same wall:

| Consumer                                         | The wall                                                                                                                                                                                                                                      |
| ------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| #321 Dead code warning                           | Final Decision B "pub = external interface, never reported" is because there is no role model to distinguish "truly external" from "looks external"; whether unused pub in bin should be reported (Option A) is left pending                  |
| RFC-014c Workspace (under review, #113)          | When member packages `use` each other, what counts as the import surface—`[exports]`, the single file in `[lib]`, or all pub files—**not a single semantic clause in the entire text**                                                        |
| RFC-015 Configuration system (accepted)          | `[lib]` / `[[bin]]` / `[exports]` fields already defined; aside from RFC-029's consumption of entry priority, the semantics of the remaining fields (especially the relationship between `[exports]` and Registry reachability) are unaligned |
| RFC-037 Packaging / RFC-029a Cache (draft, #293) | Packaging artifact selection and incremental recompilation cache unit boundaries both require the role model as a prerequisite                                                                                                                |

### Current Problems

**Four semantic gaps** (inventory on 2026-09-12, verified by reviewing 014/014a/014b/014c/015/029 in
full):

1. **Cross-package import surface undefined**. The mutual-reference example in 014c only has the
   directory structure (`src/lib.yx`); there is no clause on what `use utils.helper` can get. The
   current implicit rule is "all top-level pub files in that package"—internal implementation files
   are also dragged into the namespace, and the dependency boundary is effectively nonexistent.
2. **Dead code pub exemption has no applicability scope**. The "pub never reported" of #321's Final
   Decision B is overly conservative in single-file scripts (scripts have no external consumers, pub
   is meaningless), yet overly permissive in entry files of multi-file projects (unused pub in the
   root file with no importer is dead code). Without a role model, the dividing line between Options
   A and B cannot be drawn.
3. **Three export concepts are unaligned**. `[exports]` (015, path mapping), `[lib]` (015,
   single-file path), Registry reachability (029, reachable from entry along use)—whether they are
   in a containment relation, equality relation, or each handle their own, there is no clause.
4. **Noun collision**. RFC-014b's `[binaries]` is the **precompiled distribution artifact** (.so/exe
   download-priority strategy), completely a different layer from "bin target (source role)". After
   014c lands, the two will inevitably appear on the same screen; not disambiguating early will
   continue to cause misunderstanding.

**Empirical evidence: the conservatism of Final Decision B is already a real pain point**. #321 M2
retrospective record: under default configuration, the structural reason that the W1001–W1005 entire
family is silent is precisely that "all pub are treated as external interfaces"—this is compensation
for a missing model, not the end state.

## Proposal

### Core Design: File Role Model

**Roles are file-level properties** (consistent with the field granularity of 015: `[lib]` is a
single file, `[[bin]]` is a file list, `[exports]` is a file map), not a package-level switch.

| Role         | Determination                                                                                                                                            | Semantics                                                                                                                                                                                          |
| ------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Bin**      | File pointed to by `[run].main` / `[[bin]].path`; or (when no manifest) a file containing `main` and not `use`d by any other file                        | Program entry. Its `pub` **may be reported as dead code** (no out-of-package consumers); `main` is a reachability root                                                                             |
| **Lib**      | Files hit by the `[exports]` mapping; or `[lib].path`; or (when no manifest) files referenced by `use` in other files                                    | Distribution boundary. Its `pub` **exempted from dead code** (out-of-package consumers are not visible, better to under-report); symbols exported through it are visible across packages           |
| **Test**     | Files hit by RFC-036 test file rules (the `tests/` directory, `*_test.yx` and other existing conventions); **no** `[[test]]` explicit declaration is set | Test code. Does not participate in dead code judgment; its references count toward the reachability roots of the code under test                                                                   |
| **Internal** | When the package has a manifest, files that are neither on the export surface nor contain `main`                                                         | In-package implementation. `pub` exempted until Phase 2 (tightened by in-package use-graph reachability, with Bin-role corpus validation as prerequisite); out-of-package `use` is **unreachable** |
| **Script**   | A file run directly as a single file (`run foo.yx`)                                                                                                      | Behavior is exactly as today: Registry contains only std, no change to dead code pub semantics                                                                                                     |

**Determination priority**: explicit manifest declaration > entry-reachability inference > 036 test
convention. Explicit declaration always wins—inference is only the default when no declaration
exists.

### Examples

```toml
# yaoxiang.toml (all fields already defined by RFC-015, this RFC adds no new configuration surface)
[lib]
path = "src/lib.yx"

[[bin]]
name = "my-cli"
path = "src/cli.yx"

[exports]
"." = "src/lib.yx"
"./internal-helper" = "src/helper.yx"   # ← Determines cross-package visibility, not pub
```

```yaoxiang
# src/cli.yx (Bin role)
pub unused_fn = (x: Int) => x    # ← W1001 can be reported: bin has no out-of-package consumer
main = { ... }

# src/lib.yx (Lib role, on the export surface)
pub api_fn = ...                 # ← Never reported: out-of-package consumers are not visible

# src/other.yx (Internal role, not on the export surface)
pub semi_api = ...               # ← Exempted until Phase 2 (better to under-report), later tightened by use graph
```

### Import Surface Semantics

Resolution order for cross-package `use pkg.x` (the first match takes effect):

1. If `[exports]` mapping exists → import surface = the file set listed in the mapping, `x` must be
   in the top-level bindings of these files;
2. Otherwise, if `[lib].path` exists → import surface = that single file;
3. If neither (no manifest dependency, path depends on bare directory) → **maintain status quo**:
   all top-level files are importable (compatible with existing behavior, tightening is a separate
   discussion).

**Workspace members**: the export surface is defined only by each member package's own manifest; the
workspace root may not override or extend the member's export surface—member self-containment is the
core design of 014c, and root override would break encapsulation.

Relationship with Registry reachability (aligned with 029): `[exports]`/`[lib]` define the **legal
starting set for cross-package resolution**; once the starting set is determined, the Registry still
expands according to 029's "reachable from entry along `use`"—internal files that the export files
themselves `use` enter via linking, but do not generate new cross-package starting points.

### Boundaries with Existing Plans

| Neighbor                                           | Boundary                                                                                                                                                                                                                                                                         |
| -------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| RFC-029d "CLI `--entry` overrides entry" (planned) | 029f defines the role model; 029d consumes it—`--entry` is the CLI-level override for the Bin role                                                                                                                                                                               |
| RFC-014b `[binaries]`                              | **Noun disambiguation**: `[binaries]` is the precompiled distribution artifact (download-priority strategy), `[[bin]]` is the source role declaration. The former exists in the dependency package's manifest, the latter exists in this package's manifest; not interchangeable |
| RFC-014c Workspace                                 | The import surface for member packages mutually referencing each other = this RFC's resolution order; the workspace coordination mechanism (members, shared lockfile) still belongs to 014c                                                                                      |
| RFC-036 Test                                       | Test role determination directly adopts 036's existing file rules, no new conventions added                                                                                                                                                                                      |
| #321 Dead code                                     | Final Decision B (pub always exempted) is explicitly downgraded to "transitional semantics when there is no target"; the Bin role's pub being reportable is the form in which this RFC delivers Option A                                                                         |

## Detailed Design

### Role Determination Algorithm (in the Orchestrator)

```
fn classify(files, manifest, entry_reach) -> Map<File, Role>:
    # 1. Explicit layer: manifest declarations take priority
    for f in manifest.exports.values():  role[f] = Lib
    if manifest.lib_path:                role[lib_path] = Lib
    for b in manifest.binaries:          role[b.path] = Bin
    if manifest.run_main:                role[run_main] = Bin

    # 2. Inference layer: by entry reachability where undeclared
    for f in files where role[f] unset:
        if f is use'd by ≥1 non-self file: role[f] = Lib
        elif f contains main and no other file use's it: role[f] = Bin
        else:                            role[f] = Internal

    # 3. Test layer: 036 rule hits → Test (overrides Lib/Internal, does not override explicit Bin)
    apply_rfc036_test_rules(files, &role)

    # Single-file direct run: entire model bypassed, behavior unchanged
    if no manifest and single_file:      all Script
```

### Compiler Changes

- `frontend/config.rs`: manifest parsing adds `[exports]` → role table mapping (field parsing
  already exists in 015).
- `frontend/module/orchestrator.rs`: insert role classification phase before Registry construction;
  cross-package resolution validates `use` targets according to the import surface order, and
  out-of-bounds cases report the existing `module_not_found` family (no new error code, message adds
  "not on the export surface" hint).
- `typecheck/passes/dead_code.rs`: the entry set changes from "main + all pub" to "main + pub of
  Lib-role files" (Bin immediately reportable; Internal exempted until Phase 2, see Implementation
  Strategy for Phase 2).
- LSP (RFC-017): completion/hover "importable items" are filtered by the import surface; `main`
  missing diagnostics are reported only for Bin-role files.

### Runtime Behavior

No change. The role model is purely compile-time/parse-time semantics and does not affect the IR or
execution.

### Backward Compatibility

- Projects without a manifest: behavior is completely unchanged (the inference layer restores the
  current state—`use`d files are Lib, files containing `main` are Bin). The increment in dead code
  warnings is only in Bin files: unused pub in root files with no importer goes from silent to
  W1001—this is the expected behavior of #321's Option A, not a break.
- Projects with a manifest: once `[exports]` is declared, the import surface narrows from "all pub
  files" to the declared surface. **This is the only behavior-narrowing point**: code that depends
  on others' internal package files will start reporting `module_not_found`. Given that the package
  ecosystem has not yet been established (029: "currently no third-party package ecosystem"), the
  break surface is zero; nevertheless, a minor version transition period is set (out-of-bounds warns
  first, then errors).

## Trade-offs

### Advantages

- Zero new configuration surface: all fields are already defined in 015; this RFC only adds
  semantics.
- Zero new keywords: the import surface is carried by the distribution boundary, isomorphic to 029's
  "visibility does not exist" decision.
- Four consumers (#321 / 014c / 029a / 037) are modeled in one go, no more each hitting their own
  wall.
- The zero-configuration gene of the scripting language is fully preserved for the no-manifest path.

### Disadvantages

- Role inference (`use`d means Lib) in the common form of "entry file `use`s utility file" will
  judge utility files as Lib, and their pub is exempted—broader than the ideal granularity. This is
  an intentional choice in the better-to-under-report direction; tightening requires call-direction
  analysis on the use graph and is not done in the first version.
- Narrowing the import surface via `[exports]` is a semantically narrowing change with breakage
  (although a transition period is provided).

## Alternatives

| Alternative                              | Description                                                               | Reason for not adopting                                                                                                                                                                                                                |
| ---------------------------------------- | ------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Top-level RFC-040                        | Open a new top-level number                                               | Module graph semantics (import surface / reachability / entry) all live in the 029 territory; splitting into top-level would create two top-level documents for the same object; and the 029b–029e slot pattern is already established |
| Attach to 014c                           | As a section of workspace                                                 | The dependency direction is reversed: the language layer (029x) defines semantics, the package layer (014x) consumes semantics. 014c is still under review, expanding scope mid-way would drag down its landing                        |
| Option A as-is (#321 menu)               | Manifest mandates `[[bin]]`/`[[lib]]` targets to have dead code semantics | Introduces mandatory configuration for a narrow slice (unused pub in the root file with main); the inference layer of this RFC achieves the same benefit with zero configuration                                                       |
| Pure entry inference (no manifest layer) | Does not consume 015 fields                                               | `[exports]` is already an accepted field in 015; cross-package import surface cannot bypass it; not defining it equals leaving a gap for 014c                                                                                          |

## Implementation Strategy

### Phased

- **Phase 1**: Role classification + import surface resolution + Bin pub reportable (Internal/Test
  remain exempted)
- **Phase 2**: Internal pub tightened to "reportable if unreachable in the in-package use
  graph"—prerequisite is that after Phase 1 lands, Bin-role warning corpus validation shows no false
  positive callbacks; triggered when ready, no timeline set

### Dependencies

- Prerequisites: RFC-029 orchestrator (landed), RFC-015 manifest fields (defined).
- Dependents: #321 (Bin pub reportable), RFC-014c (import surface), RFC-029a (role as cache unit
  boundary), RFC-037 (artifact selection), RFC-029d (`--entry` override).
- Landing order with RFC-014c: before 014c is approved, the import surface order of 029f should be
  finalized first; 014c consumes by reference, to avoid scope expansion mid-review.

### Risks

- Deviation between role inference and user intuition (the first disadvantage above) → all
  incremental behavior is presented as warnings (non-blocking) and can be disabled at any time.
- Transition period management for the narrowing of `[exports]` → warn first within a minor version.

## Open Questions

All three draft-stage items have been finalized (2026-09-13, decisions recorded in Appendix B and
the main text), with no outstanding items:

- [x] Whether to tighten Internal pub → **Finalized**: tighten by in-package use-graph reachability
      in Phase 2, with Bin-role corpus validation as prerequisite (see Implementation Strategy
      Phase 2)
- [x] Explicit `[[test]]` declaration → **Finalized**: not introduced; Test role is determined only
      by 036 rules; if 036 has a future need for explicit declaration, it will extend itself
- [x] Workspace root overriding member export surface → **Finalized**: not allowed; member
      self-containment is the core design of 014c, and root override would break encapsulation

---

## Appendix B: Design Decision Records

| Decision                                              | Decision                                                                                           | Date       | Recorder | Basis                                                                                                                         |
| ----------------------------------------------------- | -------------------------------------------------------------------------------------------------- | ---------- | -------- | ----------------------------------------------------------------------------------------------------------------------------- |
| Numbering                                             | 029f (029b reserved for hot reload, 029c deleted, 029d/029e already placeholdered)                 | 2026-09-12 | Chenxu   | RFC-029 §Sub-RFC Planning                                                                                                     |
| Belongs to the 029 family rather than a top-level RFC | Yes                                                                                                | 2026-09-12 | Chenxu   | Import surface / reachability / entry all belong to module graph semantics; 029 has already consumed `[[bin]]` entry priority |
| Visibility keyword                                    | Not introduced                                                                                     | 2026-09-12 | Chenxu   | Isomorphic to RFC-029's "visibility does not exist" decision; the import surface is carried by the distribution boundary      |
| Status of #321 Final Decision B                       | Downgraded to "transitional semantics when there is no target"                                     | 2026-09-12 | Chenxu   | Bin-role pub being reportable is the form in which #321 Option A is delivered                                                 |
| Role granularity                                      | File level                                                                                         | 2026-09-12 | Chenxu   | 015 field granularity (`[lib]` single file, `[[bin]]` list, `[exports]` mapping)                                              |
| `[binaries]` disambiguation                           | 014b precompiled distribution artifact ≠ `[[bin]]` source role                                     | 2026-09-12 | Chenxu   | Two layers of concept (download strategy vs. compilation role); nail it down before 014c lands                                |
| Internal pub tightening                               | Tighten by in-package use-graph reachability in Phase 2, prerequisite = Bin-role corpus validation | 2026-09-13 | Chenxu   | Better-to-under-report progressive tightening, corpus-driven to avoid premature false positives                               |
| Explicit `[[test]]` declaration                       | Not introduced; Test determined only by 036 rules                                                  | 2026-09-13 | Chenxu   | No configuration surface added before 036 has the need                                                                        |
| Workspace root overriding member export surface       | Not allowed                                                                                        | 2026-09-13 | Chenxu   | Member self-containment is the core design of 014c, and root override would break encapsulation                               |

## Appendix C: Glossary

| Term                  | Definition                                                                                                                      |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| Role                  | The compilation target identity of a source file: Script / Bin / Lib / Test / Internal                                          |
| Import Surface        | The set of files that are legal resolution starting points for cross-package `use`, declared by `[exports]`/`[lib]` or inferred |
| Distribution Boundary | 029 term: the external exposure range of a package; this RFC refines it from the implicit (all pub files) to the import surface |
| Script State          | The bypass form of running a single file directly without a manifest; all behavior is consistent with the current state         |

## References

- RFC-029 Module Semantics (parent RFC: entry selection, visibility decision, sub-RFC planning)
- RFC-015 Configuration System (`[lib]`/`[[bin]]`/`[exports]` field definitions)
- RFC-014b Build System and Binary Distribution (`[binaries]`, the noun disambiguation target)
- RFC-014c Workspace Support (the first consumer of the import surface, under review)
- RFC-036 Test Framework (source of Test role rules)
- #321 M2 Warning Code Independent Emission Channel (origin of Final Decision B and Option A)
