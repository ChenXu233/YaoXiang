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

> **Implementation correction (2026-10-08, RFC-029g)**: The `pub` keyword has been removed entirely
> by RFC-029g. All references in this document to "`pub` exemption / `pub` files / unused pub
> warnings" should be read as "**top-level bindings + intra-package reference pool**": the role
> classification (Script / Bin / Lib / Internal / Test) and import surface semantics remain
> unchanged; the change is only in the dead code exemption judgment criteria (see
> [029g](029g-remove-pub-and-auto-bind.md) and the criteria in
> `docs/src/reference/warning-code/warning-codes.md`).

Fulfills the extension slot promised by RFC-029's "Sub-RFC Planning" (slots 029b–029e are taken,
deferred to 029f): defines a **compilation target role model** (Script / Bin / Lib / Test / Internal
— five categories) for source files and their **import surface semantics** — when there is no
manifest, roles are inferred by entry reachability (zero-config); with a manifest, roles are
explicitly declared via `[lib]` / `[[bin]]` / `[exports]` (fields already defined in RFC-015). Fills
four gaps in semantics: what defines the cross-package import surface, the applicability domain of
the pub exemption in dead code warnings (Decision B of #321), the relationship between `[exports]`
and Registry reachability, and the disambiguation between RFC-014b's `[binaries]` (precompiled
distribution artifacts) and the bin-role source file.

## Motivation

### Host and Boundary Basis

RFC-029 (accepted) is the direct upstream of this RFC for entry selection and visibility decisions:

> **Entry file selection** priority: 1. `[run].main` 2. `path` of the first `[[bin]]` 3.
> `src/main.yx` (conventional default). — RFC-029 §Inter-package cycles: revisited

> **Visibility**: does not exist. Scope + distribution boundary cover all scenarios; no new keyword
> needed. — RFC-029 §Design Decision Record (2026-07-30)

The second item is the design red line of this RFC: the import surface semantics **must not
introduce a new visibility keyword**; it can only be carried by the "distribution boundary" (file
role + declared export surface).

Four consumers each hit the same wall:

| Consumer                                         | Wall hit                                                                                                                                                                                                                                          |
| ------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| #321 dead code warning                           | Decision B "pub = external interface, never reported" stems from lacking a role model to distinguish "truly external" from "looks external"; whether unused pub inside bin should be reported (Option A) has been pending                         |
| RFC-014c workspace (under review, #113)          | When member packages `use` each other, what defines the import surface — `[exports]`, the single `[lib]` file, or all pub files — **not a single semantic clause in the entire document**                                                         |
| RFC-015 configuration system (accepted)          | `[lib]` / `[[bin]]` / `[exports]` fields are defined; aside from RFC-029's consumption of entry priority, the semantics of the remaining fields (especially the relationship between `[exports]` and Registry reachability) have not been aligned |
| RFC-037 packaging / RFC-029a cache (draft, #293) | Selection of packaging artifacts and cache unit boundaries for incremental recompilation both require the role model as a prerequisite                                                                                                            |

### Current Problem

**Four semantic gaps** (2026-09-12 inventory, confirmed by full review of
014/014a/014b/014c/015/029):

1. **Cross-package import surface undefined**. The 014c member-package mutual-reference example only
   has a directory structure (`src/lib.yx`); what `use utils.helper` can obtain has no clause. The
   current implicit rule is "all top-level pub files of that package" — internal implementation
   files are also dragged into the namespace, making the dependency boundary a dead letter.
2. **Dead code pub exemption has no applicability domain**. Decision B of #321, "pub is never
   reported," is over-conservative in single-file scripts (scripts have no external consumers, pub
   is meaningless) and over-permissive in entry files of multi-file projects (no one imports the
   root file, its pub is dead code). Without a role model, the A/B division line cannot be drawn.
3. **Three export concepts unaligned**. `[exports]` (015, path mapping), `[lib]` (015, single file
   path), Registry reachability (029, reachable from entry along use) — whether they are
   containment, equality, or independent, no clause.
4. **Term collision**. RFC-014b's `[binaries]` is the **precompiled distribution artifact** (.so/exe
   download priority policy), completely different from the "bin target (source role)". After 014c
   is landed, the two will inevitably appear together; failing to disambiguate now will continue to
   cause confusion.

**Empirical evidence: Decision B's conservatism is already a real pain point**. The #321 M2
retrospective records that the structural reason the W1001–W1005 family is silent under default
configuration is exactly that "all pub are treated as external interfaces" — this is a compensatory
behavior due to the missing model, not a terminal state.

## Proposal

### Core Design: File Role Model

**A role is a file-level attribute** (consistent with the 015 field granularity: `[lib]` is a single
file, `[[bin]]` is a file list, `[exports]` is a file mapping), not a package-level switch.

| Role         | Determination                                                                                                                                  | Semantics                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Bin**      | The file pointed to by `[run].main` / `[[bin]].path`; or (without manifest) a file containing `main` and not use'd by any other file           | Program entry. Requires a binding named `main`, value or function acceptable: function main is called with zero arguments at entry, value main is evaluated at initialization (even if the evaluated result is a function value, it is not called again). Missing `main` is a compile error (E3020); top level must not have executable statements. Its `pub` **may be reported as dead code** (no out-of-package consumer); `main` is the reachability root |
| **Lib**      | A file hit by the `[exports]` mapping; or `[lib].path`; or (without manifest) a file use'd by some other file's `use`                          | Distribution boundary. Its `pub` **is exempt from dead code** (out-of-package consumers cannot see; prefer false negatives); symbols exported through it are visible across packages                                                                                                                                                                                                                                                                         |
| **Test**     | Files matched by RFC-036 test file rules (existing conventions: `tests/` directory, `*_test.yx`, etc.); **no** `[[test]]` explicit declaration | Test code. Not subject to dead code judgment; its references count toward the reachability root of the code under test                                                                                                                                                                                                                                                                                                                                       |
| **Internal** | When the package has a manifest, files neither on the export surface nor containing main                                                       | Package-internal implementation. `pub` exemption held until phase 2 (tightened by intra-package use-graph reachability, prerequisite: Bin-role corpus verification); out-of-package `use` **unreachable**                                                                                                                                                                                                                                                    |
| **Script**   | A file run directly as a single file (`run foo.yx`)                                                                                            | **No entry concept**. Top-level statements (including binding initialization) form the program body in written order; `main` is a regular binding, **not auto-invoked** — to run it, write `main()`. Registry contains only std                                                                                                                                                                                                                              |

**Determination priority**: explicit manifest declaration > entry reachability inference > 036 test
conventions. Explicit declaration always wins — inference is only the default when there is no
declaration.

### Entry Determination

The role model is not only used for dead code analysis; it also **drives entry determination**.
Entry rules for each role:

| Role                      | Entry rule                                                                                                                                                                                                                                            |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Script** (no manifest)  | **No entry concept**. Top-level statements (including binding initialization) form the program body in written order; `main` is a regular binding, **not auto-invoked** — to run it, write `main()`                                                   |
| **Bin** (with manifest)   | Requires a binding named `main` (value/function both acceptable: value main evaluated at initialization, function main called with zero arguments at entry); missing `main` is a compile error (E3020). Top level must not have executable statements |
| **Lib / Internal / Test** | No `main` required (they are not entries)                                                                                                                                                                                                             |

**Script and Bin entries must be mutually exclusive**: "Under Script, `main` is not special" and
"Under Bin, `main` is the entry" cannot both hold — if Script both executes top-level statements and
implicitly calls `main`, a script that explicitly wrote `main()` would run twice (#356). Therefore
Script has only one execution entry: top-level statements.

**Distinction between role determination and entry determination** (implementation note): Entry
determination uses "whether there is a manifest" (`find_project_root().is_some()`) rather than
`roles::classify()`. Reason: classify answers "who consumes this file" (serves dead code analysis);
a file with a manifest but no `main` is classified as Internal by classify (reasonable, since no
other file use's it), but if the user runs it as an entry, it must have an entry. Two different
questions, two different criteria.

**Execution form of value `main`** (Decision of #388): functions are already values (unified at the
type layer), and the entry is determined by **binding existence** rather than callability — value
main and function main differ only in evaluation strategy. Value main does not set an entry call
(#356 prevents double-run: initialization and entry call never stack), sharing the same execution
form as Script; its evaluation is part of the initialization sequence: executed in topological order
of global bindings, with independent bindings following source order. Therefore wrapping Script
top-level statements into `main = { ... }` and migrating to Bin is **not an order-preserving
migration** — scattered statements execute in written order, binding initialization follows
topological order; initialization that needs to run first should make `main` explicitly depend on
it. A value main like `main: Int = 5` (a non-callable value main) is a legal but effectless program
(parallel to Rust's empty `fn main() {}`).

See `docs/src/reference/language-spec/syntax.md` §3.11.

### Example

```toml
# yaoxiang.toml (all fields are already defined in RFC-015; this RFC adds no new configuration surface)
[lib]
path = "src/lib.yx"

[[bin]]
name = "my-cli"
path = "src/cli.yx"

[exports]
"." = "src/lib.yx"
"./internal-helper" = "src/helper.yx"   # ← Cross-package visibility is determined by this, not pub
```

```yaoxiang
# src/cli.yx (Bin role)
pub unused_fn = (x: Int) => x    # ← W1001 reportable: bin has no out-of-package consumers
main: () -> Void = { ... }

# src/lib.yx (Lib role, on the export surface)
pub api_fn = ...                 # ← Never reported: out-of-package consumers are invisible

# src/other.yx (Internal role, not on the export surface)
pub semi_api = ...               # ← Exempt until phase 2 (prefer false negatives), then tightened by use graph
```

### Import Surface Semantics

The resolution order of cross-package `use pkg.x` (first match takes effect):

1. `[exports]` mapping exists → import surface = the set of files listed by the mapping, `x` must be
   among the top-level bindings of these files;
2. Otherwise `[lib].path` exists → import surface = that single file;
3. Neither exists (no-manifest dependency, path depends on bare directory) → **maintain current
   behavior**: all top-level files are importable (compatible with existing behavior, tightening to
   be discussed separately).

**Workspace members**: the export surface is defined only by the member package's own manifest; the
workspace root must not override or extend a member's export surface — member self-containment is
014c's core design, and root override would break encapsulation.

Relationship with Registry reachability (aligned with 029): `[exports]`/`[lib]` define the **set of
legitimate starting points for cross-package resolution**; once the starting points are determined,
Registry still extends per 029's "reachable from entry along `use`" — internal files use'd in by
export files come along with linking, but do not create new cross-package starting points.

### Boundary with Existing Plans

| Neighbor                                           | Boundary                                                                                                                                                                                                                                                                     |
| -------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| RFC-029d "CLI `--entry` overrides entry" (planned) | 029f defines the role model; 029d consumes it — `--entry` is the CLI-level override mechanism for the Bin role                                                                                                                                                               |
| RFC-014b `[binaries]`                              | **Term disambiguation**: `[binaries]` is the precompiled distribution artifact (download priority policy), `[[bin]]` is the source role declaration. The former exists in the dependency package's manifest, the latter in this package's manifest; they do not interchanged |
| RFC-014c workspace                                 | The import surface for member mutual references = the resolution order of this RFC; workspace coordination mechanisms (members, shared lockfile) still belong to 014c                                                                                                        |
| RFC-036 testing                                    | Test role determination directly uses 036's existing file rules, no new convention added                                                                                                                                                                                     |
| #321 dead code                                     | Decision B (pub unconditionally exempt) is explicitly downgraded to "transitional semantics when no target is present"; Bin role's reportable pub is the form in which this RFC fulfills Option A                                                                            |

## Detailed Design

### Role Determination Algorithm (in orchestrator)

```
fn classify(files, manifest, entry_reach) -> Map<File, Role>:
    # 1. Explicit layer: manifest declaration takes priority
    for f in manifest.exports.values():  role[f] = Lib
    if manifest.lib_path:                role[lib_path] = Lib
    for b in manifest.binaries:          role[b.path] = Bin
    if manifest.run_main:                role[run_main] = Bin

    # 2. Inference layer: by entry reachability where undeclared
    for f in files where role[f] undetermined:
        if f is use'd by ≥1 other file:  role[f] = Lib
        elif f contains main and no other file use's it: role[f] = Bin
        else:                             role[f] = Internal

    # 3. Test layer: 036 rules match → Test (overrides Lib/Internal, not explicit Bin)
    apply_rfc036_test_rules(files, &role)

    # Single-file direct run: entire model bypassed, behavior unchanged
    if no manifest and single_file:      all Script
```

### Compiler Changes

- `frontend/config.rs`: manifest parsing supplements the `[exports]` → role table mapping (field
  parsing already in 015).
- `frontend/module/orchestrator.rs`: insert role classification phase before Registry construction;
  cross-package resolution validates `use` targets per import surface order, out-of-bounds reports
  the existing `module_not_found` family (no new error code, message augmented with "not on export
  surface" hint).
- `typecheck/passes/dead_code.rs`: the entry point set changes from "main + all pub" to "main + pub
  of Lib-role files" (Bin can be reported immediately; Internal exempt until phase 2, see
  Implementation Strategy for Phase 2).
- LSP (RFC-017): completion/hover "importable items" filtered by import surface; `main` missing
  diagnostic reported only for Bin-role files.

### Runtime Behavior

No change. The role model is purely a compile-time/parse-time semantic, with no effect on IR and
execution.

### Backward Compatibility

- No-manifest projects: behavior completely unchanged (inference layer restores current state — used
  ones are Lib, ones with main are Bin). The dead code warning increment is only in Bin files:
  unused pub in a root file imported by no one goes from silent to W1001 — this is the expected
  behavior of #321 Option A, not a break.
- With-manifest projects: when `[exports]` is declared, the import surface is narrowed from "all pub
  files" to the declared surface. **This is the only behavioral narrowing point**: code that depends
  on another package's internal files will start reporting `module_not_found`. Given that the
  package ecosystem has not yet been established (029: "currently no third-party package
  ecosystem"), the break surface is zero; a minor-version transition period is still set
  (out-of-bounds first warning, then error).

## Trade-offs

### Advantages

- Zero new configuration surface: all fields are 015-defined; this RFC only adds semantics.
- Zero new keywords: the import surface is carried by distribution boundary, isomorphic to 029's
  "visibility does not exist" decision.
- Four consumers (#321 / 014c / 029a / 037) modeled in one go, no longer each hitting its own wall.
- Zero cost on the no-manifest path: the zero-configuration gene of scripting languages is fully
  preserved.

### Disadvantages

- Role inference (use'd means Lib) classifies a utility file as Lib in the common shape "entry file
  use's utility file", whose pub is exempt — coarser than ideal granularity. This is a deliberate
  choice in the direction of preferring false negatives; tightening requires call-direction analysis
  on the use graph, not done in v1.
- `[exports]` narrowing the import surface is a behaviorally tightening semantic with breakage
  (despite the transition period).

## Alternatives

| Alternative                              | Description                                                                 | Reason for rejection                                                                                                                                                                                                             |
| ---------------------------------------- | --------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Top-level RFC-040                        | Open a new top-level number                                                 | Module-graph semantics (import surface / reachability / entry) all live in 029's territory; splitting top-level would produce two top-level documents for the same object; and the 029b–029e slot pattern is already established |
| Attach to 014c                           | As a section of workspace                                                   | Dependency direction reversed: language layer (029x) defines semantics, package layer (014x) consumes semantics. 014c is still under review; expanding scope mid-way delays its landing                                          |
| Option A as-is (#321 menu)               | Dead code semantics only when manifest mandates `[[bin]]`/`[[lib]]` targets | Introduces mandatory configuration for a narrow slice (unused pub in a root file with main); this RFC's inference layer achieves the same benefit with zero configuration                                                        |
| Pure entry inference (no manifest layer) | Does not consume 015 fields                                                 | `[exports]` is already a 015-accepted field; cross-package import surface cannot bypass it; leaving it undefined is leaving a gap for 014c                                                                                       |

## Implementation Strategy

### Phasing

- **Phase 1**: Role classification + import surface resolution + Bin pub reportable (Internal/Test
  remain exempt)
- **Phase 2**: Tighten Internal pub to "reachable on intra-package use graph" — prerequisite is no
  false-positive callback in Bin-role warning corpus after Phase 1 lands; trigger when ready, no
  schedule set

### Dependencies

- Prerequisites: RFC-029 orchestrator (landed), RFC-015 manifest fields (defined).
- Depended on by: #321 (Bin pub reportable), RFC-014c (import surface), RFC-029a (role as cache unit
  boundary), RFC-037 (artifact selection), RFC-029d (`--entry` override).
- Landing order with RFC-014c: before 014c is approved, 029f's import surface order should be
  finalized first; 014c consumes by reference, to avoid scope expansion during review.

### Risks

- Deviation between role inference and user intuition (first item in trade-offs above) → present all
  increment behavior in warning form (non-blocking), disable-able at any time.
- Transition-period management for `[exports]` narrowing → first warning within a minor version.

## Open Questions

All three items in the draft stage have been resolved (2026-09-13, resolutions entered into Appendix
B and the main text); no outstanding items:

- [x] Whether Internal pub should be tightened → **Resolution**: tighten in phase 2 by intra-package
      use-graph reachability, with Bin-role corpus verification as prerequisite (see Implementation
      Strategy Phase 2)
- [x] `[[test]]` explicit declaration → **Resolution**: not introduced; Test role determined only by
      036 rules; if 036 needs explicit declaration in the future, it will be extended by itself
- [x] Whether workspace root can override member export surface → **Resolution**: not allowed;
      member self-containment is 014c's core design, and root override would break encapsulation

---

## Appendix B: Design Decision Record

| Decision                                           | Determination                                                                                | Date       | Recorder | Basis                                                                                                                     |
| -------------------------------------------------- | -------------------------------------------------------------------------------------------- | ---------- | -------- | ------------------------------------------------------------------------------------------------------------------------- |
| Number                                             | 029f (029b reserved hot reload, 029c deleted, 029d/029e occupied)                            | 2026-09-12 | Chenxu   | RFC-029 §Sub-RFC Planning                                                                                                 |
| Belongs to 029 family rather than top-level RFC    | Yes                                                                                          | 2026-09-12 | Chenxu   | Import surface / reachability / entry all belong to module-graph semantics; 029 already consumes `[[bin]]` entry priority |
| Visibility keyword                                 | Not introduced                                                                               | 2026-09-12 | Chenxu   | Isomorphic to RFC-029's "visibility does not exist" decision; import surface carried by distribution boundary             |
| Status of #321 Decision B                          | Downgraded to "transitional semantics when no target is present"                             | 2026-09-12 | Chenxu   | Bin-role reportable pub is the fulfillment form of #321 Option A                                                          |
| Role granularity                                   | File-level                                                                                   | 2026-09-12 | Chenxu   | 015 field granularity (`[lib]` single file, `[[bin]]` list, `[exports]` mapping)                                          |
| `[binaries]` disambiguation                        | 014b precompiled distribution artifact ≠ `[[bin]]` source role                               | 2026-09-12 | Chenxu   | Two-layer concepts (download policy vs compilation role), nailed down before 014c lands                                   |
| Internal pub tightening                            | Phase 2 by intra-package use-graph reachability, prerequisite = Bin-role corpus verification | 2026-09-13 | Chenxu   | Prefer false negatives with progressive tightening, corpus-driven to avoid premature false positives                      |
| `[[test]]` explicit declaration                    | Not introduced; Test determined only by 036 rules                                            | 2026-09-13 | Chenxu   | Don't add a configuration surface before 036 has the need                                                                 |
| Workspace root overriding member export surface    | Not allowed                                                                                  | 2026-09-13 | Chenxu   | Member self-containment is 014c's core design, root override breaks encapsulation                                         |
| Whether `main` is auto-invoked under Script        | **Not auto-invoked** — top-level statements are the program, `main()` must be explicit       | 2026-09-17 | Chenxu   | Both rules together would double-run an explicit `main()` (#356); Script can have only one execution entry                |
| Whether role model is used for entry determination | **Yes** (previously only used for dead code warnings)                                        | 2026-09-17 | Chenxu   | This RFC defines Bin's "`main` is reachability root"; without hooking entry, that semantic spins idle                     |

## Appendix C: Glossary

| Term                  | Definition                                                                                                                          |
| --------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| Role                  | A source file's compilation target identity: Script / Bin / Lib / Test / Internal                                                   |
| Import Surface        | The set of files that are legitimate resolution starting points in cross-package `use`, declared by `[exports]`/`[lib]` or inferred |
| Distribution Boundary | 029 terminology: a package's external exposure scope; this RFC refines it from the implicit (all pub files) to the import surface   |
| Script state          | The bypass form for no-manifest single-file direct run; all behavior is consistent with the current state                           |

## References

- RFC-029 Module Semantics (parent RFC: entry selection, visibility decision, sub-RFC planning)
- RFC-015 Configuration System (`[lib]`/`[[bin]]`/`[exports]` field definitions)
- RFC-014b Build System and Binary Distribution (`[binaries]`, the disambiguation target)
- RFC-014c Workspace Support (first consumer of import surface, under review)
- RFC-036 Testing Framework (source of Test role rules)
- #321 M2 warning code independent emission channel (source of Decision B and Option A)

## Known Defects: Landed (2026-10-16, WBS 4.9 round 1)

> The previous version (2026-10-09) listed the following three items as "three design defects of the
> reference pool". All three have been landed one by one by WBS 4.9 round 1 (commit `3863a9ff0`);
> `collect_project_refs` and `orchestrator.rs` have been moved into
> `src/driver/{state,arms,helpers}.rs` along with the P4 4.2.9 split; all file:line in the table
> below are re-fetched on the cited revision.

### Three Design Defects of the Reference Pool — Correction Status

| #   | Original defect                                              | Current state (2026-10-16)                                                                                                                                                                                                                                                                                                                                                                                              | Implementation code (rev `3863a9ff0`)                                                                                                   |
| --- | ------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | Scan surface is a directory, not a compilation surface       | **Pool root = package target**: `collect_package_refs` (`helpers.rs:481`) calls `discover_with_used` per the `[[bin]]`/`[lib]`/`[exports]`/`[run].main` roots of `roles::ExplicitSurfaces` to obtain the compilation set; **does not scan directories to find entries**. When all roots fail to compile, falls back to the compilation set of the inspected file itself (`collect_compilation_refs` / `helpers.rs:454`) | `src/driver/helpers.rs:454-512` (fallback path); `src/driver/helpers.rs:481-512` (manifest declared-surface path)                       |
| 2   | Granularity is project-level bare names, no module dimension | **Pool type = `HashMap<String, HashSet<String>>`**: key = module key, value = that module's set of reference names; the consumer at `arms.rs:603-609` unions all module entries except the current unit and feeds them to `DeadCodeAnalyzer`; a module's own names (top-level definitions and local bindings, `use` imported names excepted) no longer exempt same-name definitions in other modules                    | `src/driver/arms.rs:603-609` (consumer `other_module_refs`); single assembly entry `src/driver/helpers.rs:429-449` (`collect_ref_pool`) |
| 3   | Script reuses Lib's pool (violates single-file semantics)    | **Script does not connect to pool**: the `match us.role` at `arms.rs:611-623` only assembles for `Bin`/`Internal` (`set_exempt_pub`) and `Lib` (`set_cross_file_refs`); Script/Test take the `_ => {}` default arm; the latter is judged only by the in-file definition-use graph                                                                                                                                       | `src/driver/arms.rs:611-623`                                                                                                            |

**Follow-up improvements 4 / 5 — Correction status (2026-10-16, WBS 4.9 round 2)**

> The two items in this section were originally listed as "to be landed in P4 4.9". Both have been
> landed one by one in WBS 4.9 round 2; the file:line in the table below are taken on the **current
> landing worktree** (commit hash to be filled in after the batch close on push, see 09a — round 1
> lesson: local hashes before push get rewritten by `pull --rebase`).

| #   | Original improvement                                                                                                              | Current state (2026-10-16)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Implementation code                                                                                                                                                                                                                                                                            |
| --- | --------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 4   | Consumption definition refinement: distinguish "imported" from "really used after import" (only the latter counts as consumption) | **Consumption criterion = "really used after import"**: a consumption record = some module brings name n from module M into scope via `use` **and at the use site really references n**, recorded only to M (`RefPool::consumed_from`); names that only appear in use items (no use site) and other modules' references to **same-name** identifiers do not count as consumption. The target module of an import is determined by the module-key set from discovery (`resolve_import_module`, **does not start a separate path resolution** — reuses the discovery-phase resolution product delivered in 4.6.1, see `39546020`); namespace form (`use lib` + `lib.foo()`) is attributed the same way | `src/driver/helpers.rs:442-446` (`RefPool`); `src/driver/helpers.rs:449-524` (`collect_ref_pool` + `resolve_import_module`); `src/frontend/core/typecheck/passes/dead_code.rs:180-192` (`ModuleNames`), `:328-352` (`collect_module_names`), `:122-176` (`ImportFace` / `collect_import_face`) |
| 5   | Test pool separation: tests/ references do not exempt production code (src/)                                                      | **`FileRole::Test` does not contribute to consumption surface**: Test module references form a separate pool (Test itself is not subject to dead code judgment, so its references do not spill over); a production-code definition is saved only by production-code consumption. Role is determined at a single point by `roles::classify` (test discovery rules RFC-036)                                                                                                                                                                                                                                                                                                                            | `src/driver/helpers.rs:470-476` (Test short-circuit in `collect_ref_pool`); `src/driver/helpers.rs:595-601` (single-point `roles::classify`); `src/driver/arms.rs:562-580` (both fallback and declared-surface paths feed role to pool)                                                        |

**Residual surface (registered, non-blocking)**: ① Field/method position references (`obj.render()`
has only the short name) cannot be attributed to the module of the receiver type at the AST layer —
only when the base name hits a namespace import is it attributed by import; otherwise it's
conservatively recorded by name dimension into `RefPool::field_names`; ② Among same-name overloads:
**per-candidate attribution completed** (WBS 4.9.6 fix round) — reachability consumption D20
decision product (a pure-read **iter** of the `TypeCheckResult.overload_resolutions` enum, does not
touch `get`, not recorded into consumption ledger); the IR-name-to-definition pairing (same-name
k-th declaration ↔ resolved name `#k`) is completed only at L1 driver (`resolved_method_marks`); the
analyzer only receives `(base name, declaration order)` tuples (L2 must not know IR names);
declarations hit by resolution are reachable, **unhit sibling overloads are still honestly reported
as W1005** (acceptance ③ "containing those not hit by overload resolution" holds). **Fallback
boundary (registered, prefer-false-negatives direction)**: when a name group in this unit **has no
resolution record at all** (e.g., ambiguous call site E1107, or call forms not going through
overload resolution), fall back to short-name co-judgment — at this point we do **not** claim
"uncalled methods are always honestly reported"; this boundary is jointly defined by the
driver-level nail at `src/driver/tests/method_unused.rs` (mixed overload cases: some calls ⇒ the
unhit one is reported exactly).

### Reference Count Distortion (WBS 4.9.6 / issue #450) — Correction status (2026-10-16)

> Originally registered: overload methods **are called and resolution is in effect, yet still
> falsely reported as `W1005 Unused method`** (reference counting distorts overload calls). On
> 2026-10-16, user ruling A registered it as an independent level-3 task and merged it into WBS 4.9
> round 2; this item is of class D (deleting a false report changes the outward-facing surface of
> diagnostics), and has been authorized to land.

| Link                    | Original defect                                                                                                                                                                                                                                  | Current state                                                                                                                                                                                                                                                                                                     | Implementation code                                                                                     |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| Reachability probe      | Scan uses **qualified names** to look up references (`Point.render`), but reference sites collected are **short names** (`obj.render()` has only `render`) ⇒ called methods are never reachable, W1005 falsely reported in no-project-root cases | Methods use a **short-name probe** to determine call surface, and apply **per-candidate narrowing with the D20 resolution product** (`resolved_methods`; unhit sibling overloads are still reported as W1005); name groups with no resolution record fall back to short-name co-judgment (see residual surface ②) | `src/frontend/core/typecheck/passes/dead_code.rs:733-744` (probe); `:766-770` (per-definition judgment) |
| Definition table keying | `all_defs` is keyed by qualified name ⇒ **same-name overloads overwrite each other** (only one of two overloads remains, only 1 reported)                                                                                                        | Definition table is **one entry per definition**: methods appended, same-name non-method bindings deduplicated by name keeping the last (overloads are independent definitions, top-level reassignment is the same binding)                                                                                       | `src/frontend/core/typecheck/passes/dead_code.rs:304-322` (`record_definition`), `:28-29` (field)       |

**Reason for not changing in P3.5 (consistent with previous version)**: module-level resolution
requires the path resolution of `use` items to be complete (`use a::foo` → foo comes from module a),
which is in the P4-phase contract and Driver unification scope.
