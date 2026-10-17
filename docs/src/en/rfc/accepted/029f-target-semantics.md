---
title: 'RFC-029f: Compilation Target Role and Import Surface Semantics'
status: 'Accepted'
author: 'Chenxu'
created: '2026-09-12'
updated: '2026-09-13'
accepted: '2026-09-13'
issue: '#334'
---

# RFC-029f: Compilation Target Role and Import Surface Semantics

## Abstract

> **Landing Correction (2026-10-08, RFC-029g)**: The `pub` keyword has been entirely removed by
> RFC-029g. All references in the body of this document to "`pub` exemption / `pub` files / unused
> pub warning" should be read as "**top-level binding + in-package reference pool**": the role
> classification (Script / Bin / Lib / Internal / Test) and import surface semantics remain
> unchanged; the change is only in the dead-code exemption criteria (see
> [029g](029g-remove-pub-and-auto-bind.md) and the criteria in
> `docs/src/reference/warning-code/warning-codes.md`).

Fulfilling the extension slot from RFC-029 "Sub-RFC Plan" (029b–029e already taken, so 029f
follows): defining a **compilation target role model** (Script / Bin / Lib / Test / Internal — five
categories) for source files and its **import surface semantics** — when no manifest, inferred by
entry reachability (zero-configuration); when a manifest is present, explicitly declared via `[lib]`
/ `[[bin]]` / `[exports]` (fields already defined by RFC-015). Filling in four dangling semantics:
what counts as the cross-package import surface, the applicable scope of the pub exemption for the
dead-code warning (Issue #321 decision B), the relationship between `[exports]` and Registry
reachability, and the term disambiguation between RFC-014b's `[binaries]` (pre-compiled distribution
artifacts) and the bin-role source file.

## Motivation

### Host and Boundary Basis

RFC-029 (accepted)'s entry selection and visibility decisions are the direct upstream of this RFC:

> **Entry file selection** priority: 1. `[run].main` 2. the first `path` of `[[bin]]` 3.
> `src/main.yx` (convention default). — RFC-029 §Cross-package Cycles: Continued

> **Visibility**: Does not exist. Scope + distribution boundary cover all scenarios; no new keywords
> needed. — RFC-029 §Design Decision Records (2026-07-30)

The second item is the design red line of this RFC: import surface semantics **must not introduce a
new visibility keyword**, and can only be carried by the "distribution boundary" (file role +
declared export surface).

Four consumers each hit the same wall:

| Consumer                                           | Wall Hit                                                                                                                                                                                                                                    |
| -------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| #321 Dead-code warning                             | Decision B "pub = external interface never reported" is because there is no role model to distinguish "truly external" from "looks external"; whether bin's unused pub is reported (Plan A) has been suspended                              |
| RFC-014c Workspace (under review, #113)            | When member packages mutually `use`, what counts as the import surface — `[exports]`, `[lib]` single file, or all pub files — **not a single semantic clause in the whole document**                                                        |
| RFC-015 Configuration System (accepted)            | `[lib]` / `[[bin]]` / `[exports]` fields are defined; besides RFC-029 consuming the entry priority, the semantics of the remaining fields (especially the relationship between `[exports]` and Registry reachability) have not been aligned |
| RFC-037 Packaging / RFC-029a Caching (draft, #293) | Packaging product selection, the cache unit boundary for incremental recompilation, all need the role model as a prerequisite                                                                                                               |

### Current Problems

**Four semantic gaps** (inventory on 2026-09-12, confirmed by reviewing 014/014a/014b/014c/015/029
in full):

1. **Cross-package import surface is undefined**. The 014c member package mutual import example only
   has a directory structure (`src/lib.yx`), and there is no clause on what `use utils.helper` can
   get. The current implicit rule is "all top-level pub files of the package" — internal
   implementation files are also dragged into the namespace, and the dependency boundary is in name
   only.
2. **The dead-code pub exemption has no applicable scope**. #321 Decision B's "pub never reported"
   is too conservative in single-file scripts (scripts have no external consumers, pub is
   meaningless) and too lenient in the entry file of multi-file projects (a root file's unused pub
   is dead code). Without a role model, the dividing line between Plans A and B cannot be drawn.
3. **Three export concepts are not aligned**. `[exports]` (015, path mapping), `[lib]` (015, single
   file path), Registry reachability (029, reachable from entry along use) — whether they are
   inclusion relationships, equality relationships, or each managing its own, there is no clause.
4. **Term collision**. RFC-014b's `[binaries]` is a **pre-compiled distribution artifact** (.so/exe
   download priority policy), which is on a completely different level from "bin target (source
   role)". After 014c lands, the two will inevitably appear on screen at the same time, and not
   disambiguating early will continue to cause misunderstandings.

**Evidence: The conservatism of Decision B is already a real pain point**. #321 M2 retrospective
records: the structural reason why W1001–W1005 are silent across the family under default
configuration is precisely "all pub are treated as external interfaces" — this is compensatory
behavior due to model absence, not the terminal state.

## Proposal

### Core Design: File Role Model

**Role is a file-level property** (consistent with the field granularity of 015: `[lib]` is a single
file, `[[bin]]` is a file list, `[exports]` is a file mapping), not a package-level switch.

| Role         | Determination                                                                                                                                               | Semantics                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| ------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Bin**      | Files pointed to by `[run].main` / `[[bin]].path`; or (when no manifest) files containing `main` and not `use`d by any file                                 | Program entry. Requires a binding named `main`, value or function acceptable: function `main` is called with zero arguments at entry stage; value `main` is evaluated at initialization stage (even if the evaluation result is a function value, it is not called again). Missing `main` is a compile error (E3020); top level must not have executable statements. Its `pub` **may be reported as dead code** (no out-of-package consumer); `main` is the reachability root |
| **Lib**      | Files matched by `[exports]` mapping; or `[lib].path`; or (when no manifest) files referenced by `use` from other files                                     | Distribution boundary. Its `pub` **is exempt from dead-code** (out-of-package consumers are not visible, prefer false negatives); symbols exported through it are visible across packages                                                                                                                                                                                                                                                                                     |
| **Test**     | Files matched by RFC-036 test file rules (`tests/` directory, `*_test.yx` and other existing conventions); **does not set** `[[test]]` explicit declaration | Test code. Does not participate in dead-code determination; its references count as the reachability root of the code under test                                                                                                                                                                                                                                                                                                                                              |
| **Internal** | When a package has a manifest, files that are neither on the export surface nor contain `main`                                                              | In-package implementation. `pub` is exempted to phase 2 (tightened by in-package use-graph reachability, with Bin-role corpus validation as prerequisite); out-of-package `use` **is unreachable**                                                                                                                                                                                                                                                                            |
| **Script**   | Files run directly as a single file (`run foo.yx`)                                                                                                          | **No entry concept**. Top-level statements (including binding initialization) are the program body in source order; `main` is a normal binding, **not auto-called** — to run it write `main()`. Registry contains only std                                                                                                                                                                                                                                                    |

**Determination priority**: manifest explicit declaration > entry-reachability inference > 036 test
convention. Explicit declaration always wins — inference is just the default when there is no
declaration.

### Entry Determination

The role model is not only used for dead-code analysis, but also **drives entry determination**.
Entry rules for each role:

| Role                      | Entry Rule                                                                                                                                                                                                                                                 |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Script** (no manifest)  | **No entry concept**. Top-level statements (including binding initialization) are the program body in source order; `main` is a normal binding, **not auto-called** — to run it write `main()`                                                             |
| **Bin** (with manifest)   | Requires a binding named `main` (value/function acceptable: value `main` is evaluated at initialization, function `main` is called with zero arguments at entry); missing `main` is a compile error (E3020). Top level must not have executable statements |
| **Lib / Internal / Test** | Does not require `main` (they are not entries)                                                                                                                                                                                                             |

**Script and Bin entries must be mutually exclusive**: "Script does not special-case `main`" and
"Bin makes `main` the entry" cannot both hold — if Script both executes top-level statements and
implicitly calls `main`, a script that explicitly writes `main()` would run twice (#356). Therefore
Script has only one execution entry: the top-level statements.

**Difference between role determination and entry determination** (implementation note): Entry
determination uses "whether there is a manifest" (`find_project_root().is_some()`) rather than
`roles::classify()`. Reason: classify answers "who consumes this file" (serving dead-code analysis);
a file with a manifest but no `main` is classified as Internal in classify (it is not `use`d by
other files, which is reasonable), but the user runs it as an entry, and an entry must exist at that
point. Two different questions, two different criteria.

**Execution form of value `main`** (Issue #388 decision): Functions are already values (unified at
the type layer), the entry is determined by **binding existence** rather than callability — value
`main` and function `main` differ only in evaluation strategy. Value `main` does not set entry call
(#356 prevents double-running: initialization and entry call never stack), using the same execution
form as Script; its evaluation is part of the initialization sequence: executed in topological order
of global bindings, with independent bindings in source order. Therefore, wrapping Script top-level
statements into `main = { ... }` and migrating them to Bin **is not an order-preserving migration**
— loose statements execute in source order, binding initialization follows topological order;
initialization that needs to run first should let `main` explicitly depend on it. A non-callable
value main like `main: Int = 5` is a legal but observation-effect-less program (analogous to Rust's
empty `fn main() {}`).

See `docs/src/reference/language-spec/syntax.md` §3.11.

### Examples

```toml
# yaoxiang.toml (all fields are already defined by RFC-015, this RFC adds no new configuration surface)
[lib]
path = "src/lib.yx"

[[bin]]
name = "my-cli"
path = "src/cli.yx"

[exports]
"." = "src/lib.yx"
"./internal-helper" = "src/helper.yx"   # ← what determines cross-package visibility is this, not pub
```

```yaoxiang
# src/cli.yx (Bin role)
pub unused_fn = (x: Int) => x    # ← W1001 may report: bin has no out-of-package consumer
main: () -> Void = { ... }

# src/lib.yx (Lib role, on the export surface)
pub api_fn = ...                 # ← never reported: out-of-package consumers are not visible

# src/other.yx (Internal role, not on the export surface)
pub semi_api = ...               # ← exempted to phase 2 (prefer false negatives), then tightened by use graph
```

### Import Surface Semantics

Resolution order for cross-package `use pkg.x` (the first hit takes effect):

1. `[exports]` mapping exists → import surface = the set of files listed in the mapping, `x` must be
   in the top-level bindings of these files;
2. Otherwise `[lib].path` exists → import surface = that single file;
3. Neither (manifest-less dependency, path-depending bare directory) → **maintain status quo**: all
   top-level files are importable (backward compatible, tightening to be discussed separately).

**Workspace members**: The export surface is defined only by the member package's own manifest; the
workspace root must not override or extend the member export surface — member self-containment is
the core design of 014c, and root override would break encapsulation.

Relationship with Registry reachability (aligned with 029): `[exports]`/`[lib]` define the **set of
legal starting points for cross-package resolution**; once the starting points are determined, the
Registry still expands according to 029's "reachable from entry along `use`" — internal files `use`d
by the exported file are brought in via linking, but do not produce new cross-package starting
points.

### Boundaries with Existing Plans

| Neighbor                                           | Boundary                                                                                                                                                                                                                                                             |
| -------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| RFC-029d "CLI `--entry` overrides entry" (planned) | 029f defines the role model; 029d consumes it — `--entry` is a CLI-level override for the Bin role                                                                                                                                                                   |
| RFC-014b `[binaries]`                              | **Term disambiguation**: `[binaries]` are pre-compiled distribution artifacts (download priority policy), `[[bin]]` is a source-role declaration. The former exists in the dependency package's manifest, the latter in this package's manifest, not interchangeable |
| RFC-014c Workspace                                 | The import surface for member package mutual references = the resolution order of this RFC; workspace coordination mechanisms (members, shared lockfile) still belong to 014c                                                                                        |
| RFC-036 Testing                                    | Test role determination directly adopts the existing file rules of 036, no new convention added                                                                                                                                                                      |
| #321 Dead-code                                     | Decision B (pub always exempted) is explicitly downgraded to "transitional semantics when there is no target"; Bin role's pub may be reported is the form in which this RFC fulfills Plan A                                                                          |

## Detailed Design

### Role Determination Algorithm (Inside the Orchestrator)

```
fn classify(files, manifest, entry_reach) -> Map<File, Role>:
    # 1. Explicit layer: manifest declarations take precedence
    for f in manifest.exports.values():  role[f] = Lib
    if manifest.lib_path:                role[lib_path] = Lib
    for b in manifest.binaries:          role[b.path] = Bin
    if manifest.run_main:                role[run_main] = Bin

    # 2. Inference layer: by entry reachability where undeclared
    for f in files where role[f] is unset:
        if f is use'd by ≥1 non-self file:      role[f] = Lib
        elif f contains main and no other file use's it: role[f] = Bin
        else:                             role[f] = Internal

    # 3. Test layer: 036 rule hit → Test (overrides Lib/Internal, does not override explicit Bin)
    apply_rfc036_test_rules(files, &role)

    # Single-file direct run: entire model bypassed, behavior unchanged
    if no manifest and single_file:      all Script
```

### Compiler Changes

- `frontend/config.rs`: manifest parsing supplements `[exports]` → role table mapping (field parsing
  already in 015).
- `frontend/module/orchestrator.rs`: Insert role classification stage before Registry construction;
  cross-package resolution validates `use` targets according to import surface order, out-of-bounds
  reports existing `module_not_found` family (no new error code, message supplemented with "not on
  export surface" hint).
- `typecheck/passes/dead_code.rs`: The entry point set changes from "main + all pub" to "main +
  Lib-role file's pub" (Bin can be reported immediately; Internal exempted to phase 2, see Phase 2
  in implementation strategy).
- LSP (RFC-017): "Importable items" for completion/hover filtered by import surface; `main` missing
  diagnostic only reported for Bin-role files.

### Runtime Behavior

No change. The role model is pure compile-time/parse-time semantics, and does not affect IR and
execution.

### Backward Compatibility

- Manifest-less projects: Behavior completely unchanged (inference layer restores status quo — files
  that are `use`d are Lib, files containing main are Bin). The increment in dead-code warnings is
  only in Bin files: the unused pub of a root file with no importer goes from silent to W1001 — this
  is the expected behavior of #321 Plan A, not a break.
- Projects with manifest: When `[exports]` is declared, the import surface narrows from "all pub
  files" to the declared surface. **This is the only behavioral narrowing point**: code that depends
  on other packages' internal files will start reporting `module_not_found`. Given that the package
  ecosystem has not yet been established (029: "currently no third-party package ecosystem"), the
  breakage surface is zero; a minor-version transition period is still set (out-of-bounds warns
  first, then errors).

## Tradeoffs

### Pros

- Zero new configuration surface: all fields are already defined by 015, this RFC only adds
  semantics.
- Zero new keywords: Import surface is carried by the distribution boundary, isomorphic to the 029
  "visibility does not exist" decision.
- Four consumers (#321 / 014c / 029a / 037) are modeled once, no longer each hitting their own wall.
- Manifest-less path is zero-cost: the zero-configuration gene of the scripting language is fully
  preserved.

### Cons

- Role inference (use'd is Lib) in the common shape of "entry file uses utility files" will classify
  the utility file as Lib, and its pub is exempted — wider than the ideal granularity. This is a
  deliberate choice in the false-negative direction; tightening requires call-direction analysis on
  the use graph, and is not done in the first version.
- Narrowing the import surface via `[exports]` is a behaviorally tightening change with breakage
  (though with a transition period).

## Alternatives

| Plan                                     | Description                                                            | Reason Not Adopted                                                                                                                                                                                                                 |
| ---------------------------------------- | ---------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Top-level RFC-040                        | Open a new top-level number                                            | Module graph semantics (import surface / reachability / entry) all belong to 029 territory, splitting into top-level would create two top-level documents for the same object; and the 029b–029e slot pattern has been established |
| Attach to 014c                           | As a section of workspace                                              | Dependency direction is reversed: language layer (029x) defines semantics, package layer (014x) consumes semantics. 014c is still under review, expanding scope mid-way delays its landing                                         |
| Plan A as-is (#321 menu)                 | Manifest forces `[[bin]]`/`[[lib]]` target to have dead-code semantics | Introduces mandatory configuration for a narrow slice (unused pub of root file with main); the inference layer of this RFC achieves the same benefit with zero configuration                                                       |
| Pure entry inference (no manifest layer) | Does not consume 015 fields                                            | `[exports]` is already an accepted field of 015, cross-package import surface cannot bypass it; not defining it is leaving a gap for 014c                                                                                          |

## Implementation Strategy

### Phased

- **Phase 1**: Role classification + import surface resolution + Bin pub reportable (Internal/Test
  remain exempted)
- **Phase 2**: Internal pub tightened to "in-package use-graph unreachable, reportable" —
  prerequisite is that after Phase 1 lands, Bin-role warning corpus validation has no false-positive
  callbacks; triggered when ready, no timeline set

### Dependency Relationships

- Prerequisites: RFC-029 orchestrator (landed), RFC-015 manifest fields (defined).
- Depended on by: #321 (Bin pub reportable), RFC-014c (import surface), RFC-029a (role as cache unit
  boundary), RFC-037 (artifact selection), RFC-029d (`--entry` override).
- Landing order with RFC-014c: Before 014c passes review, the import surface order of 029f should be
  finalized first, and 014c consumes by reference, to avoid scope expansion mid-review.

### Risks

- Deviation between role inference and user intuition (the first tradeoff above) → present all
  incremental behavior in warning (non-blocking) form, disableable at any time.
- Transition period management for `[exports]` narrowing → warn first within the minor version.

## Open Questions

All three draft-stage questions have been decided (2026-09-13, resolutions entered into Appendix B
and the body), no pending items:

- [x] Whether Internal pub is tightened → **Decision**: Phase 2 tightened by in-package use-graph
      reachability, with Bin-role corpus validation as prerequisite (see Implementation Strategy
      Phase 2)
- [x] `[[test]]` explicit declaration → **Decision**: Not introduced; Test role determined only by
      036 rules; if 036 has an explicit declaration need in the future, it will be extended by
      itself
- [x] Workspace root overriding member export surface → **Decision**: Not allowed; member
      self-containment is the core design of 014c, root override breaks encapsulation

---

## Appendix B: Design Decision Records

| Decision                                               | Determination                                                                                     | Date       | Recorder | Basis                                                                                                                         |
| ------------------------------------------------------ | ------------------------------------------------------------------------------------------------- | ---------- | -------- | ----------------------------------------------------------------------------------------------------------------------------- |
| Number                                                 | 029f (029b reserved for hot-reload, 029c deleted, 029d/029e already placed)                       | 2026-09-12 | Chenxu   | RFC-029 §Sub-RFC Plan                                                                                                         |
| Belongs to the 029 family rather than top-level RFC    | Yes                                                                                               | 2026-09-12 | Chenxu   | Import surface / reachability / entry all belong to module graph semantics; 029 has already consumed `[[bin]]` entry priority |
| Visibility keyword                                     | Not introduced                                                                                    | 2026-09-12 | Chenxu   | Isomorphic to RFC-029's "visibility does not exist" decision; import surface is carried by the distribution boundary          |
| Status of #321 Decision B                              | Downgraded to "transitional semantics when there is no target"                                    | 2026-09-12 | Chenxu   | Bin-role pub reportable is the fulfillment form of #321 Plan A                                                                |
| Role granularity                                       | File level                                                                                        | 2026-09-12 | Chenxu   | 015 field granularity (`[lib]` single file, `[[bin]]` list, `[exports]` mapping)                                              |
| `[binaries]` disambiguation                            | 014b pre-compiled distribution artifact ≠ `[[bin]]` source role                                   | 2026-09-12 | Chenxu   | Two-level concepts (download policy vs. compilation role), nailed down before 014c lands                                      |
| Internal pub tightening                                | Phase 2 tightened by in-package use-graph reachability, prerequisite = Bin-role corpus validation | 2026-09-13 | Chenxu   | Prefer false negative and tighten progressively, corpus-driven to avoid premature false positives                             |
| `[[test]]` explicit declaration                        | Not introduced; Test determined only by 036 rules                                                 | 2026-09-13 | Chenxu   | No configuration surface added before 036 needs it                                                                            |
| Workspace root overriding member export surface        | Not allowed                                                                                       | 2026-09-13 | Chenxu   | Member self-containment is the core design of 014c, root override breaks encapsulation                                        |
| Whether `main` is auto-called in Script                | **Not auto-called** — top-level statements are the program, `main()` must be explicit             | 2026-09-17 | Chenxu   | Two rules coexisting would cause explicit `main()` to run twice (#356); Script can have only one execution entry              |
| Whether the role model is used for entry determination | **Yes** (previously only used for dead-code warnings)                                             | 2026-09-17 | Chenxu   | This RFC defines Bin's "`main` is the reachability root"; not connecting entry makes that semantics idle                      |

## Appendix C: Glossary

| Term                  | Definition                                                                                                                      |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| Role                  | The compilation target identity of a source file: Script / Bin / Lib / Test / Internal                                          |
| Import Surface        | The set of files that are legal resolution starting points for cross-package `use`, declared by `[exports]`/`[lib]` or inferred |
| Distribution Boundary | 029 term: the external exposure range of a package; this RFC refines it from implicit (all pub files) to the import surface     |
| Script State          | The bypass form of manifest-less single-file direct run, all behavior consistent with status quo                                |

## References

- RFC-029 Module Semantics (parent RFC: entry selection, visibility decision, sub-RFC plan)
- RFC-015 Configuration System (`[lib]`/`[[bin]]`/`[exports]` field definitions)
- RFC-014b Build System and Binary Distribution (`[binaries]`, the disambiguation target)
- RFC-014c Workspace Support (the first consumer of the import surface, under review)
- RFC-036 Testing Framework (source of Test role rules)
- #321 M2 Warning Code Independent Emission Channel (origin of Decision B and Plan A)

## Known Defects: Landed (2026-10-16, WBS 4.9 Round 1)

> The previous version (2026-10-09) listed the following three items as "three design defects of the
> reference pool". All three have been landed item by item by WBS 4.9 Round 1 (commit `3863a9ff0`);
> `collect_project_refs` and `orchestrator.rs` have been split into
> `src/driver/{state,arms,helpers}.rs` along with P4 4.2.9, and all file:line in the table below are
> re-taken on the cited revision.

### Three Design Defects of the Reference Pool — Correction Status

| #   | Original Defect                                              | Current State (2026-10-16)                                                                                                                                                                                                                                                                                                                                                                                                          | Landed Code (rev `3863a9ff0`)                                                                                                                |
| --- | ------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | Scan surface is directory rather than compilation surface    | **Pool root = package target**: `collect_package_refs` (`helpers.rs:481`) calls `discover_with_used` for the compilation set by the `[[bin]]`/`[lib]`/`[exports]`/`[run].main` root of `roles::ExplicitSurfaces`; **does not scan the directory for entries**. When all roots fail, it degrades to the compilation set of the file being checked itself (`collect_compilation_refs` / `helpers.rs:454`)                             | `src/driver/helpers.rs:454-512` (fallback path); `src/driver/helpers.rs:481-512` (manifest declared surface path)                            |
| 2   | Granularity is project-level bare names, no module dimension | **Pool type = `HashMap<String, HashSet<String>>`**: key = module key, value = the set of reference names for that module; the consumer side, in `arms.rs:603-609`, takes the union of all module entries "other than this unit" and feeds them to `DeadCodeAnalyzer`; module's own names (top-level definitions and local bindings, `use`-imported names are an exception) no longer exempt same-named definitions in other modules | `src/driver/arms.rs:603-609` (consumer-side `other_module_refs`); single assembly entry `src/driver/helpers.rs:429-449` (`collect_ref_pool`) |
| 3   | Script reuses Lib's pool (violates single-file semantics)    | **Script does not connect to pool**: `arms.rs:611-623`'s `match us.role` only assembles for `Bin`/`Internal` (`set_exempt_pub`) and `Lib` (`set_cross_file_refs`), Script/Test go through the `_ => {}` default arm; the latter only determines by the file-internal definition-use graph                                                                                                                                           | `src/driver/arms.rs:611-623`                                                                                                                 |

**Subsequent Improvements 4 / 5 — Correction Status (2026-10-16, WBS 4.9 Round 2)**

> This section's two items were originally listed as "to be landed by P4 4.9". Both have been landed
> item by item by WBS 4.9 Round 2; the file:line in the table below are taken on the **landing
> working tree of this batch** (the commit number is filled in after the batch is closed at push
> time, see 09a — Round 1 lesson: local hashes before push are rewritten by `pull --rebase`).

| #   | Original Improvement                                                                                                               | Current State (2026-10-16)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | Landed Code                                                                                                                                                                                                                                                                                    |
| --- | ---------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 4   | Consumption definition refinement: distinguish "imported" and "actually used after import" (only the latter counts as consumption) | **Consumption criterion = "actually used after import"**: a consumption record = a module `use`s to bring the name n from module M into scope **and at the use site actually references n**, recorded only to M (`RefPool::consumed_from`); names that only appear in the use item (no use site), and references to **same-named** identifiers in other modules do not count as consumption. The target module of the import is determined by the set of module keys at the discovery stage (`resolve_import_module`, **does not start another set of path resolution** — reuses the discovery-stage resolution product delivered by 4.6.1, see `39546020`); namespace form (`use lib` + `lib.foo()`) is attributed the same way | `src/driver/helpers.rs:442-446` (`RefPool`); `src/driver/helpers.rs:449-524` (`collect_ref_pool` + `resolve_import_module`); `src/frontend/core/typecheck/passes/dead_code.rs:180-192` (`ModuleNames`), `:328-352` (`collect_module_names`), `:122-176` (`ImportFace` / `collect_import_face`) |
| 5   | Test pool separation: references in tests/ do not exempt production code (src/)                                                    | **`FileRole::Test` does not contribute to consumption surface**: Test module's references form a separate pool (Test itself does not participate in dead-code determination, so its references do not spill over); production code's definitions are only saved by production code's consumption. Role is determined by a single point `roles::classify` (test discovery rules RFC-036)                                                                                                                                                                                                                                                                                                                                          | `src/driver/helpers.rs:470-476` (Test short-circuit in `collect_ref_pool`); `src/driver/helpers.rs:595-601` (single-point determination by `roles::classify`); `src/driver/arms.rs:562-580` (both fallback/declared-surface paths feed role to the pool)                                       |

**Residual Surface (logged, non-blocking)**: ① Field/method position references (`obj.render()` only
has the short name) cannot be attributed at the AST layer to the module of the receiver type —
base-name hits on namespace imports are attributed by import, module-qualified construction `ns.T`
is attributed to the **type name face** (see 4.9.7 below), the rest are conservatively counted into
`RefPool::field_names` (`collect_ref_pool` in `src/driver/helpers.rs`) by name dimension; ② Method
reachability: per the D20 decision product, narrowed **per candidate + per definition unit**
(`resolved_method_marks`) — **within the same unit** unhit sibling overloads are reported as W1005
as-is; **across units, same-named same-ordered uncalled definitions in reachable form** are also
reported as-is (WBS 4.9.7 / issue #464, closed, mechanism and empirical results see next section).
**Residual boundary (logged, must not claim full coverage; all empirical)**: (a)
`check <definition file>` alone (the file has no call sites, callers not in the unit set) ⇒ no
decision entries ⇒ falls back to the pool's **name-level exemption** (`RefPool::field_names`
globally flat) ⇒ **not reported**; (b) the same bare type name brought into scope by multiple
modules (the source program is already ambiguous at the name layer, the entry separately reports
E5006) ⇒ candidate surface whole name skipped ⇒ no decision entries ⇒ same-named same-ordered group
not reported; (c) caller forms the type name in some form but never calls its methods (e.g., only
writes `b.Temp(..)` construction) ⇒ this unit is included by **possible attribution** ⇒ not
reported; (d) whole-package import `use M` does not let the bare type name into scope, std module's
candidate surface is not connected — neither form produces entries, falls back to name-level
exemption. Fallback boundary: when a name group has no decision record in this unit (e.g., ambiguous
call E1107, call forms not going through decision), falls back to short-name co-judgment
(false-negative direction) — therefore does not claim "uncalled methods are always reported as-is".

### Reference Count Distortion (WBS 4.9.6 / issue #450) — Correction Status (2026-10-16)

> Originally logged: overload methods **are called and the decision takes effect, yet still falsely
> reported as `W1005 Unused method`** (reference count distorts for overload calls). On 2026-10-16,
> the user decision A logged it as an independent level-3 task and merged it into WBS 4.9 Round 2;
> this item is class D (deleting the false report changes the diagnostic external surface), already
> authorized to land.

| Link                    | Original Defect                                                                                                                                                                                                                           | Current State                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | Landed Code                                                                                                                     |
| ----------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------- |
| Reachability probe      | Scan queries references by **qualified name** (`Point.render`), while the call-site collection gets **short names** (`obj.render()` only has `render`) ⇒ called methods are never reachable, false report W1005 with no project root path | Methods are determined by **short-name probe** for the call surface, with **D20 decision product** used for per-candidate narrowing (`resolved_methods` + `resolved_groups`; sibling overloads not hit by decision still report W1005). Scope boundary: **within the same unit** per-candidate holds; **across units (registry resolution) calls** after the production side (`3a97ff1f`) and driver-side attribution (this batch) land, in **reachable form** also per-candidate + per-definition unit holds (see residual surface ② and 4.9.7 below) | `src/frontend/core/typecheck/passes/dead_code.rs:782-792` (probe); `:815-877` (per-definition determination and exemption gate) |
| Definition table keying | `all_defs` keyed by qualified name ⇒ **same-named overloads overwrite each other** (two overloads leave only one, only 1 reported)                                                                                                        | Definition table **one entry per definition**: methods appended, same-named non-method bindings deduplicate by name keeping the last one (overloads are independent definitions, top-level reassignment is the same binding)                                                                                                                                                                                                                                                                                                                           | `src/frontend/core/typecheck/passes/dead_code.rs:304-322` (`record_definition`), `:28-29` (fields)                              |

**Reason not changed in P3.5 (consistent with previous version)**: Module-level resolution requires
path resolution of `use` items (`use a::foo` → foo comes from module a) to be complete, which
belongs to the P4 stage contract and Driver unification.

### Cross-Unit Method Reachability Attribution (WBS 4.9.7 / issue #464) — Correction Status (2026-10-17)

> Originally logged (residual surface ②): cross-unit (registry resolution) method calls do not
> produce D20 decision entries ⇒ the name group falls back to the pool's **name-level** exemption ⇒
> **cross-unit same-named same-ordered uncalled methods are not reported**. This item lands in two
> commits: **production side** (cross-unit method calls produce decision entries, commit `3a97ff1f`)
> and **driver-side attribution** (provide reachability marks per definition unit and disable
> name-level exemption). The file:line in the table below are taken on the **driver-side attribution
> working tree** (the commit number is filled in after the batch is closed at push time, see 09a —
> local hashes before push are rewritten by `pull --rebase`).

| Link                      | Original Defect                                                                                                                                                                                                                                                                 | Current State                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | Landed Code                                                                                                                                                                                                                                                                                                                                                                                                       |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Decision entry production | The three candidate tables for cross-unit call sites are all missing (`method_overloads` / `method_overload_ir_names` / `method_overload_param_names`) ⇒ the decision segment is entirely skipped ⇒ **0 entries**                                                               | The candidate group "candidates / mixed IR names / declaration-time parameter names" is packaged in the same order and passed across units via `ModuleInfo.method_overloads`; the registry merges in module-key ascending deterministic order; the Typecheck arm, according to this unit's import face, only injects candidate groups for "module faces that can be named by **bare type name**" (its own key slot does not merge out, same-name conflict whole name skipped) ⇒ same site `cands=1`, 1 entry (`Temp.to`) | `src/frontend/core/typecheck/environment.rs:308-360` (snapshot / injection); `src/frontend/module/registry.rs:148-170` (`merge_method_overloads`); `src/frontend/module/mod.rs:149-157` (`ModuleInfo` field); `src/driver/arms.rs:348-354` (injection surface), `:1512-1566` (`visible_method_overloads`)                                                                                                         |
| Driver-side attribution   | Decision entries are globally marked by **name** (a single set of `(base name, declaration order)`) ⇒ the same-named same-ordered definition in another unit is exempted by the call of the same-named sibling (cross-unit same-named same-ordered was **not reported** before) | Marks are attributed by **definition unit** (`MethodMarks { by_unit, groups }`): the `{Type}` of the entry name `{Type}.{method}#k` is the **definition-side** bare name without unit ⇒ use the recording unit's **type name face** to compute "the set of modules from which this unit can spell this name", entries are merged into the `by_unit` of each unit in the set; `groups` = name groups with entries (global)                                                                                                | `src/driver/helpers.rs:570-573` (`MethodMarks`), `:591-619` (`typed_owners`: brace items + inline alias targets + module-qualified access `ns.T`, reusing `collect_module_names` + `resolve_import_module` single point), `:621-654` (`resolved_method_marks`, via pure read of `OverloadResolutions::iter`), `src/driver/arms.rs:577-583` (take `by_unit[unit_key]` per unit), `:644-655` (inject into analyzer) |
| Determination surface     | Name groups with decision records still go through the pool's **name-level** exemption (short names globally flat)                                                                                                                                                              | Name groups with decision records **do not go through name-level exemption**: hit the active ones of this unit's declarations, the unhit ones are reported as W1005 as-is (`!marked_group` is the precondition of the exemption gate)                                                                                                                                                                                                                                                                                    | `src/frontend/core/typecheck/passes/dead_code.rs:44` (field), `:252-257` (`set_resolved_groups`), `:262-267` (`method_group_has_marks`), `:782-792` (reachability per candidate), `:815-877` (per-definition determination / exemption gate / W1005 emission)                                                                                                                                                     |

**Externally Observable Changes (Class A: old behavior not success ⇒ fix in passing, logged in the
same commit)**:

- **Cross-unit overload decision previously did not take effect**: three candidate tables all
  missing ⇒ fell back to the cross-unit single-value binding table (`method_bindings`, last-wins for
  same-named definitions) ⇒ **multi-candidate same-named method calls report `error[E1002]`**
  (empirically `Expected type 'string', found type 'int64'`); after the fix, decisions are made per
  candidate by actual argument, the same fixture empirically `Type check passed`. The criterion
  lands in the integration case
  `test_multifile_cross_unit_method_overload_resolves_by_argument_type`
  (`tests/integration/multifile.rs`, two candidates Int/String of the same name and order, two calls
  each hit) — when the injection surface is turned off, this case fails on the same E1002.
- **Cross-unit same-named same-ordered uncalled methods start to be reported as W1005 as-is in
  reachable form**: three-folder fixture (`main.yx` uses `use a.{Temp}` to name the type, `use b`
  only brings b into the compilation set) empirically — `a.Temp.to` has a call site and is not
  reported, `a.Temp.other` is reported, **`b.Temp.to` is reported**; before the fix, the same
  fixture only reported `a.Temp.other` (b's definition got through by the pool's name-level
  exemption). The inbound fixture is `tests/yaoxiang-multifile/ref-pool-cross-unit-method/` (CLI
  reproducible); the driver-level pin is `test_cross_unit_uncalled_same_named_method_is_reported` in
  `src/driver/tests/method_unused.rs`.

**Residual Boundary (logged, must not claim full coverage; item by item empirical)**:

1. **`check <definition file>` alone** (the file has no call sites, callers not in the unit set) ⇒
   no decision entries ⇒ falls back to name-level exemption ⇒ **not reported** (false-negative
   direction). Empirical: `check b.yx` on the same three-folder fixture only reports unused type
   W1002, does not report `b.Temp.to`.
2. **Ambiguous at the name layer**: the same bare type name brought into scope by multiple modules
   (`use a.{Temp}` + `use b.{Temp}`) ⇒ candidate surface whole name skipped ⇒ no decision entries ⇒
   same-named same-ordered group falls back to name-level exemption, not reported; the entry
   separately reports `E5006 Duplicate import`. Empirical control (only `b` is namespace-imported)
   same-class sibling candidates are reported as-is, proving the criterion is not running idle.
3. **Caller spells the type name but never calls its methods** (e.g., only writes `b.Temp(..)`
   construction) ⇒ this unit is included by **possible attribution** ⇒ not reported (false-negative
   direction).
4. **Inline alias `use M.{T as X}` and module-qualified construction `ns.T` do not inject the
   candidate surface** (production-side narrowing rule ②): the call-site key is still the
   definition-side `T` (empirically both forms `X(..).to()` / `ns.T(..).to()` land on key `T.to`) ⇒
   the `#k` precision of the same-named type is bounded by "the set of modules from which the
   recording unit can spell the name", **does not distinguish "which unit's definition is actually
   selected"**; precisely because of this, in these forms **cannot** judge a same-named same-ordered
   definition in another unit as dead (otherwise the actually called method would be falsely
   reported).
5. **Whole-package import `use M` does not let the bare type name into scope** (empirically writing
   `Temp(...)` reports E1001), **std module's candidate surface is not connected** (its `ModuleInfo`
   is constructed by the std registration path) — neither form produces entries, falls back to
   name-level exemption.

**Driver-side reading is pure read**: `resolved_method_marks` via `OverloadResolutions::iter` (does
not touch `get`, does not count into the consumption ledger) ⇒ does not mask the W1090 "consumption
must reach" settlement; the driver-level pin is
`test_resolved_method_marks_read_leaves_consumption_ledger_untouched` (after reading, the entry is
still in the `unconsumed()` difference set).

**Criteria and Drift (this batch empirical)**: Driver-level pin tests 5 (main acceptance / inline
alias false-report defense / module-qualified name false-report defense / key scope bidirectional /
pure read, plus 4 old criteria of #450 non-regression); IR snapshot 204 zero drift; corpus baseline
346 → 347 lines (only 1 new behavior record for the new fixture, the remaining 346 lines
byte-identical).
