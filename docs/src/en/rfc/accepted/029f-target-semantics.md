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

## Summary

> **Implementation correction (2026-10-08, RFC-029g)**: The `pub` keyword has been entirely removed
> by RFC-029g. In this document body, all statements about "pub exemption / pub file / unused pub
> warning" should be read as "**top-level binding + in-package reference pool**": role
> classification (Script / Bin / Lib / Internal / Test) and import surface semantics remain
> unchanged; the change is only in the dead code exemption determination conditions (see
> [029g](029g-remove-pub-and-auto-bind.md) and `docs/src/reference/warning-code/warning-codes.md`
> for the determination criteria).

Fulfills the extension slot in RFC-029 "Sub-RFC Planning" (029b–029e are already taken, extended to
029f): defines a **compilation target role model** (five categories: Script / Bin / Lib / Test /
Internal) for source files and their **import surface semantics**—when there is no manifest,
inferred by entry reachability (zero configuration); when there is a manifest, explicitly declared
via `[lib]` / `[[bin]]` / `[exports]` (fields already defined in RFC-015). Fills four semantic gaps:
what counts as the cross-package import surface, the applicable scope of the pub exemption for dead
code warnings (#321 Decision B), the relationship between `[exports]` and Registry reachability, and
the disambiguation between RFC-014b `[binaries]` (precompiled distribution artifacts) and the source
files of the bin role.

## Motivation

### Host and Boundary Basis

The entry selection and visibility decisions of RFC-029 (accepted) are the direct upstream of this
RFC:

> **Entry file selection** priority: 1. `[run].main` 2. The `path` of the first `[[bin]]` 3.
> `src/main.yx` (convention default). — RFC-029 §Inter-package Cycles: A Re-look

> **Visibility**: Does not exist. Scope + distribution boundary cover all scenarios, no new keyword
> needed. — RFC-029 §Design Decision Record (2026-07-30)

The second is the design red line of this RFC: import surface semantics **must not introduce new
visibility keywords**, and can only be carried by "distribution boundary" (file role + declared
export surface).

Four consumers each hit the same wall:

| Consumer                                         | Sticking point                                                                                                                                                                                                                                     |
| ------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| #321 Dead code warning                           | Decision B "pub = external interface, never reported" is because there is no role model to distinguish "truly external" from "looks external"; whether unused pub inside bin is reported (Plan A) is suspended to date                             |
| RFC-014c Workspace (under review, #113)          | When member packages `use` each other, what is the import surface—`[exports]`, `[lib]` single file, or all pub files—**the entire document has not a single semantic provision**                                                                   |
| RFC-015 Configuration system (accepted)          | `[lib]` / `[[bin]]` / `[exports]` fields are defined; aside from RFC-029 consuming the entry priority, the semantics of the remaining fields (especially the relationship between `[exports]` and Registry reachability) are not aligned by anyone |
| RFC-037 Packaging / RFC-029a Cache (draft, #293) | Both packaging artifact selection and incremental recompilation cache unit boundaries need the role model as a prerequisite                                                                                                                        |

### Current Problems

**Four semantic gaps** (2026-09-12 inventory, confirmed by reading the full text of
014/014a/014b/014c/015/029):

1. **Cross-package import surface undefined**. The member package cross-reference example in 014c
   only has directory structure (`src/lib.yx`), and there is no provision for what
   `use utils.helper` can get. The current implicit rule is "all top-level pub files of the
   package"—internal implementation files are also dragged into the namespace, and the dependency
   boundary exists in name only.
2. **Dead code pub exemption has no applicable scope**. #321 Decision B's "pub never reported" is
   over-conservative in single-file scripts (scripts have no external consumers, pub is
   meaningless), and over-permissive in the entry file of multi-file projects (unused pub in the
   root file that no one imports is dead code). Without a role model, the dividing line between
   Plans A and B cannot be drawn.
3. **Three export concepts unaligned**. `[exports]` (015, path mapping), `[lib]` (015, single file
   path), Registry reachability (029, reachable from the entry along use)—whether the three are in a
   containment relationship, equality relationship, or each manages its own, there is no provision.
4. **Term collision**. RFC-014b's `[binaries]` are **precompiled distribution artifacts** (.so/exe
   download priority strategy), completely different layer from "bin target (source code role)". The
   two will inevitably appear on the same screen after 014c lands; without early disambiguation,
   misunderstandings will continue to occur.

**Empirical evidence: The conservativeness of Decision B is already a real pain point**. The #321 M2
retrospective records: the structural reason for the W1001–W1005 family being silenced by default is
exactly that "all pub is treated as external interface"—this is compensatory behavior for a missing
model, not the end state.

## Proposal

### Core Design: File Role Model

**Role is a file-level attribute** (consistent with the field granularity of 015: `[lib]` is a
single file, `[[bin]]` is a file list, `[exports]` is a file mapping), not a package-level switch.

| Role         | Determination                                                                                                                                  | Semantics                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Bin**      | Files pointed to by `[run].main` / `[[bin]].path`; or (when there is no manifest) files containing `main` and not `use`d by any other file     | Program entry. Requires a **binding** named `main` to exist, either a value or a function: function `main` is called with zero arguments at entry, value `main` is evaluated at initialization and that evaluation executes (the evaluation result is not called again even if it is a function value). Missing `main` is a compile error (E3020); top-level must not have executable statements. Its `pub` is **reportable as dead code** (no out-of-package consumers); `main` is a reachability root |
| **Lib**      | Files hit by `[exports]` mapping; or `[lib].path`; or (when there is no manifest) files referenced by `use` from other files                   | Distribution boundary. Its `pub` is **exempted from dead code** (out-of-package consumers cannot see it, prefer under-reporting); symbols exported through it are visible across packages                                                                                                                                                                                                                                                                                                               |
| **Test**     | Files hit by RFC-036 test file rules (`tests/` directory, `*_test.yx`, and other existing conventions); **no** explicit `[[test]]` declaration | Test code. Does not participate in dead code determination; its references are counted as reachability roots of the tested code                                                                                                                                                                                                                                                                                                                                                                         |
| **Internal** | When the package has a manifest, files neither on the export surface nor containing main                                                       | In-package implementation. `pub` exempted until phase 2 (tightened by in-package use graph reachability, with Bin role corpus validation as prerequisite); `use` from outside the package is **unreachable**                                                                                                                                                                                                                                                                                            |
| **Script**   | Single-file direct-run (`run foo.yx`) files                                                                                                    | **No entry concept**. Top-level statements (including binding initialization) form the program body in source order; `main` is a regular binding and is **not called automatically**—to run it, write `main()`. Registry contains only std                                                                                                                                                                                                                                                              |

**Determination priority**: manifest explicit declaration > entry reachability inference > 036 test
convention. Explicit declaration always wins—inference is only the default when there is no
declaration.

### Entry Determination

The role model is not only used for dead code analysis, but also **drives entry determination**.
Entry rules for each role:

| Role                      | Entry rule                                                                                                                                                                                                                                                    |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Script** (no manifest)  | **No entry concept**. Top-level statements (including binding initialization) form the program body in source order; `main` is a regular binding and is **not called automatically**—to run it, write `main()`                                                |
| **Bin** (has manifest)    | Requires a **binding** named `main` (either value or function: value `main` is evaluated at initialization, function `main` is called with zero arguments at entry); missing `main` is a compile error (E3020). Top-level must not have executable statements |
| **Lib / Internal / Test** | Does not require `main` (they are not entries)                                                                                                                                                                                                                |

**The entry points of Script and Bin must be mutually exclusive**: "under Script, `main` is not
special" and "under Bin, `main` is the entry" cannot both hold—if Script both executes top-level
statements and implicitly calls `main`, scripts that explicitly write `main()` would run twice
(#356). Therefore, Script has only one execution entry: top-level statements.

**Differences between role determination and entry determination** (implementation note): Entry
determination uses "presence of manifest" (`find_project_root().is_some()`) rather than
`roles::classify()`. Reason: classify answers "who consumes this file" (serves dead code analysis);
a file that has a manifest but no `main` is classified as Internal (it isn't `use`d by other files,
which is reasonable), but if the user runs it as an entry, an entry must exist at that point. The
two questions differ, so the criteria differ as well.

**Execution form of value `main`** (#388 Decision): Functions are already values (unified at the
type level), so the entry is determined by **binding existence** rather than callability—value
`main` and function `main` only differ in evaluation strategy. Value `main` does not have an entry
call (#356 to prevent double-run: initialization and entry call never stack), the same execution
form as Script; its evaluation is part of the initialization sequence: executed in global binding
topological order, with independent bindings in source order. Therefore, wrapping Script top-level
statements into `main = { ... }` and migrating to Bin is **not an order-preserving
migration**—scattered statements are executed in source order, while binding initialization is in
topological order; initialization that needs to run first should have `main` explicitly depend on
it. A non-callable value `main` like `main: Int = 5` is a legal program with no observable effect
(analogous to Rust's empty `fn main() {}`).

See `docs/src/reference/language-spec/syntax.md` §3.11.

### Examples

```toml
# yaoxiang.toml (all fields are already defined in RFC-015, this RFC adds no new configuration surface)
[lib]
path = "src/lib.yx"

[[bin]]
name = "my-cli"
path = "src/cli.yx"

[exports]
"." = "src/lib.yx"
"./internal-helper" = "src/helper.yx"   # ← Determines cross-package visibility is this, not pub
```

```yaoxiang
# src/cli.yx (Bin role)
pub unused_fn = (x: Int) => x    # ← W1001 reportable: bin has no out-of-package consumers
main: () -> Void = { ... }

# src/lib.yx (Lib role, on export surface)
pub api_fn = ...                 # ← Never reported: out-of-package consumers cannot see

# src/other.yx (Internal role, not on export surface)
pub semi_api = ...               # ← Exempted until phase 2 (prefer under-reporting), then tightened by use graph
```

### Import Surface Semantics

Cross-package `use pkg.x` resolution order (the first hit takes effect):

1. `[exports]` mapping exists → import surface = the set of files listed in the mapping, `x` must be
   among the top-level bindings of these files;
2. Otherwise, `[lib].path` exists → import surface = that single file;
3. Neither (no manifest dependency, path dependency on bare directory) → **maintain current
   behavior**: all top-level files are importable (compatible with existing behavior, tightening
   discussed separately).

**Workspace members**: The export surface is defined only by the member package's own manifest; the
workspace root must not override or extend the member's export surface—member self-containment is
the core design of 014c, and root override would break encapsulation.

Relationship with Registry reachability (aligned with 029): `[exports]`/`[lib]` define the **set of
legal starting points for cross-package resolution**; once the starting points are determined, the
Registry still expands according to 029's "reachable from the entry along `use`"—internal files
brought in by the export file's own `use` enter with linking, but do not generate new cross-package
starting points.

### Boundaries with Existing Plans

| Neighbor                                           | Boundary                                                                                                                                                                                                                                                                            |
| -------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| RFC-029d "CLI `--entry` overrides entry" (planned) | 029f defines the role model; 029d consumes it—`--entry` is the CLI-level override means for the Bin role                                                                                                                                                                            |
| RFC-014b `[binaries]`                              | **Term disambiguation**: `[binaries]` are precompiled distribution artifacts (download priority strategy), `[[bin]]` are source role declarations. The former exists in the dependency package's manifest, the latter exists in the current package's manifest, not interchangeable |
| RFC-014c Workspace                                 | Cross-references between member packages use the import surface = this RFC's resolution order; workspace coordination mechanisms (members, shared lockfile) still belong to 014c                                                                                                    |
| RFC-036 Testing                                    | Test role determination directly uses 036's existing file rules, no new conventions added                                                                                                                                                                                           |
| #321 Dead code                                     | Decision B (pub unconditionally exempted) is explicitly downgraded to "transitional semantics when there is no target"; Bin role's pub being reportable is this RFC's fulfillment of Plan A                                                                                         |

## Detailed Design

### Role Determination Algorithm (inside the orchestrator)

```
fn classify(files, manifest, entry_reach) -> Map<File, Role>:
    # 1. Explicit layer: manifest declarations take priority
    for f in manifest.exports.values():  role[f] = Lib
    if manifest.lib_path:                role[lib_path] = Lib
    for b in manifest.binaries:          role[b.path] = Bin
    if manifest.run_main:                role[run_main] = Bin

    # 2. Inferred layer: by entry reachability where undeclared
    for f in files where role[f] is undecided:
        if f is use'd by ≥1 file other than itself:  role[f] = Lib
        elif f contains main and no other file uses it: role[f] = Bin
        else:                                        role[f] = Internal

    # 3. Test layer: 036 rule hit → Test (overrides Lib/Internal, not explicit Bin)
    apply_rfc036_test_rules(files, &role)

    # Single-file direct run: entire model bypassed, behavior unchanged
    if no manifest and single_file:      all Script
```

### Compiler Changes

- `frontend/config.rs`: manifest parsing supplements the `[exports]` → role table mapping (field
  parsing already exists in 015).
- `frontend/module/orchestrator.rs`: insert the role classification stage before Registry
  construction; cross-package resolution validates `use` targets by import surface order,
  out-of-bounds reported as the existing `module_not_found` family (no new error code, message
  supplements "not on export surface" hint).
- `typecheck/passes/dead_code.rs`: the entry point set changes from "main + all pub" to "main + Lib
  role file's pub" (Bin immediately reportable; Internal exempted until phase 2, see implementation
  strategy for Phase 2).
- LSP (RFC-017): completion/hover "importable items" filtered by import surface; `main` missing
  diagnosis reported only for Bin role files.

### Runtime Behavior

No change. The role model is pure compile-time/parse-time semantics and does not affect IR or
execution.

### Backward Compatibility

- No-manifest projects: behavior completely unchanged (the inference layer restores the current
  state—used files are Lib, files containing main are Bin). The incremental dead code warnings are
  only in Bin files: unused pub in the root file that no one imports changes from silent to
  W1001—this is the expected behavior of #321 Plan A, not a breaking change.
- Manifest projects: when `[exports]` is already declared, the import surface narrows from "all pub
  files" to the declared surface. **This is the only behavior narrowing point**: code that depends
  on others' packages' internal files will start reporting `module_not_found`. Given that the
  package ecosystem has not yet been established (029: "currently no third-party package
  ecosystem"), the breaking surface is zero; still set a minor version transition period
  (out-of-bounds warnings first, then errors).

## Trade-offs

### Advantages

- Zero new configuration surface: all fields are already defined in 015; this RFC only supplements
  semantics.
- Zero new keywords: import surface carried by distribution boundary, isomorphic to 029's
  "visibility does not exist" decision.
- Four consumers (#321 / 014c / 029a / 037) modeled in one shot, no longer each hitting their own
  wall.
- No-manifest path is zero-cost: the zero-configuration gene of scripting language is fully
  preserved.

### Disadvantages

- Role inference (used = Lib) in the common form "entry file uses utility file" will classify the
  utility file as Lib, and its pub is exempted—wider granularity than ideal. This is an intentional
  choice in the prefer-under-reporting direction; tightening requires call-direction analysis on the
  use graph, not done in the first version.
- Narrowing the import surface via `[exports]` is a breaking semantic tightening (although with a
  transition period).

## Alternatives

| Plan                                     | Description                                                                    | Reason not adopted                                                                                                                                                                                                          |
| ---------------------------------------- | ------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Top-level RFC-040                        | New top-level number                                                           | Module graph semantics (import surface / reachability / entry) are all in 029 territory; splitting top-level would cause two top-level documents for the same object; and the 029b–029e slot pattern is already established |
| Attach to 014c                           | As a section of workspace                                                      | Dependency direction is reversed: language layer (029x) defines semantics, package layer (014x) consumes semantics. 014c is still under review; expanding scope mid-review delays its landing                               |
| Plan A as-is (#321 menu)                 | Dead code semantics only with mandatory `[[bin]]`/`[[lib]]` target in manifest | Introduces mandatory configuration for a narrow slice (unused pub in root file with main); the inference layer of this RFC achieves equivalent gains with zero configuration                                                |
| Pure entry inference (no manifest layer) | Does not consume 015 fields                                                    | `[exports]` is already an accepted field in 015; cross-package import surface cannot bypass it; not defining it is leaving a gap for 014c                                                                                   |

## Implementation Strategy

### Phased

- **Phase 1**: Role classification + import surface resolution + Bin pub reportable (Internal/Test
  remain exempted)
- **Phase 2**: Internal pub tightened to "reportable when unreachable in the in-package use
  graph"—prerequisite is that after Phase 1 lands, Bin role warning corpus validation has no false
  positive callbacks; trigger and execute, no schedule set

### Dependency Relationships

- Prerequisites: RFC-029 orchestrator (landed), RFC-015 manifest fields (defined).
- Depended on by: #321 (Bin pub reportable), RFC-014c (import surface), RFC-029a (role as cache unit
  boundary), RFC-037 (artifact selection), RFC-029d (`--entry` override).
- Landing order with RFC-014c: before 014c review passes, the import surface order of 029f should be
  finalized first; 014c consumes it by reference, avoiding mid-review scope expansion.

### Risks

- Deviation between role inference and user intuition (the first trade-off above) → all incremental
  behavior presented as warnings (non-blocking), disable-able at any time.
- Transition period management for `[exports]` narrowing → warn first within the minor version.

## Open Questions

All three items in the draft stage have been decided (2026-09-13, resolutions recorded in Appendix B
and the body); no open items:

- [x] Whether to tighten Internal pub → **Decision**: tighten in phase 2 by in-package use graph
      reachability, with Bin role corpus validation as prerequisite (see implementation strategy
      Phase 2)
- [x] `[[test]]` explicit declaration → **Decision**: not introduced; Test role is determined only
      by 036 rules; when 036 has explicit declaration needs in the future, it will be extended by
      itself
- [x] Workspace root overrides member export surface → **Decision**: not allowed; member
      self-containment is the core design of 014c, root override would break encapsulation

---

## Appendix B: Design Decision Record

| Decision                                               | Resolution                                                                                         | Date       | Recorder | Basis                                                                                                               |
| ------------------------------------------------------ | -------------------------------------------------------------------------------------------------- | ---------- | -------- | ------------------------------------------------------------------------------------------------------------------- |
| Number                                                 | 029f (029b reserved for hot reload, 029c deleted, 029d/029e occupied)                              | 2026-09-12 | Chenxu   | RFC-029 §Sub-RFC Planning                                                                                           |
| Belongs to 029 family, not top-level RFC               | Yes                                                                                                | 2026-09-12 | Chenxu   | Import surface / reachability / entry are all module graph semantics; 029 already consumes `[[bin]]` entry priority |
| Visibility keyword                                     | Not introduced                                                                                     | 2026-09-12 | Chenxu   | Isomorphic with RFC-029 "visibility does not exist" decision, import surface carried by distribution boundary       |
| Status of #321 Decision B                              | Downgraded to "transitional semantics when there is no target"                                     | 2026-09-12 | Chenxu   | Bin role pub reportable is the fulfillment form of #321 Plan A                                                      |
| Role granularity                                       | File-level                                                                                         | 2026-09-12 | Chenxu   | 015 field granularity (`[lib]` single file, `[[bin]]` list, `[exports]` mapping)                                    |
| `[binaries]` disambiguation                            | 014b precompiled distribution artifact ≠ `[[bin]]` source role                                     | 2026-09-12 | Chenxu   | Two-layer concept (download strategy vs compilation role), nailed down before 014c lands                            |
| Internal pub tightening                                | Tighten in phase 2 by in-package use graph reachability, prerequisite = Bin role corpus validation | 2026-09-13 | Chenxu   | Prefer-under-reporting gradual tightening, corpus-driven to avoid premature false positives                         |
| `[[test]]` explicit declaration                        | Not introduced; Test determined only by 036 rules                                                  | 2026-09-13 | Chenxu   | No new configuration surface until 036 has the need                                                                 |
| Workspace root overrides member export surface         | Not allowed                                                                                        | 2026-09-13 | Chenxu   | Member self-containment is the core design of 014c, root override would break encapsulation                         |
| Whether `main` is auto-called under Script             | **Not auto**—top-level statements are the program, `main()` must be explicit                       | 2026-09-17 | Chenxu   | Two coexisting rules would make explicit `main()` run twice (#356); Script can have only one execution entry        |
| Whether the role model is used for entry determination | **Yes** (previously only for dead code warnings)                                                   | 2026-09-17 | Chenxu   | This RFC defines Bin's "main is the reachability root"; without connecting entry, that semantics spins in idle      |

## Appendix C: Glossary

| Term                  | Definition                                                                                                                      |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| Role                  | The compilation target identity of a source file: Script / Bin / Lib / Test / Internal                                          |
| Import Surface        | The set of files that are legal resolution starting points for cross-package `use`, declared by `[exports]`/`[lib]` or inferred |
| Distribution Boundary | 029 term: the external exposure range of a package; this RFC refines it from implicit (all pub files) to import surface         |
| Script State          | The bypass form of single-file direct run without a manifest; all behavior is consistent with the current state                 |

## References

- RFC-029 Module Semantics (parent RFC: entry selection, visibility decisions, sub-RFC planning)
- RFC-015 Configuration System (`[lib]`/`[[bin]]`/`[exports]` field definitions)
- RFC-014b Build System and Binary Distribution (`[binaries]`, the disambiguation target)
- RFC-014c Workspace Support (the first consumer of import surface, under review)
- RFC-036 Testing Framework (source of Test role rules)
- #321 M2 Warning Code Independent Emission Channel (origin of Decision B and Plan A)

## Known Defects: Landed (2026-10-16, WBS 4.9 Round 1)

> The previous version (2026-10-09) listed the following three items as "three design defects of the
> reference pool". All three have been landed one by one by WBS 4.9 Round 1 (commit `3863a9ff0`);
> `collect_project_refs` and `orchestrator.rs` have been moved into
> `src/driver/{state,arms,helpers}.rs` along with the P4 4.2.9 split; all file:line in the table
> below are re-fetched on the referenced revision.

### Three Design Defects of the Reference Pool — Correction Status

| #   | Original Defect                                             | Current Status (2026-10-16)                                                                                                                                                                                                                                                                                                                                                                             | Landed Code (rev `3863a9ff0`)                                                                                                                |
| --- | ----------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | Scan surface is directory instead of compilation surface    | **Pool root = package target**: `collect_package_refs` (`helpers.rs:481`) calls `discover_with_used` by `roles::ExplicitSurfaces`'s `[[bin]]`/`[lib]`/`[exports]`/`[run].main` roots to take the compilation set; **does not scan directories to find entries**. When all roots fail, it falls back to the compilation set of the inspected file itself (`collect_compilation_refs` / `helpers.rs:454`) | `src/driver/helpers.rs:454-512` (fallback path); `src/driver/helpers.rs:481-512` (manifest declared surface path)                            |
| 2   | Granularity is project-level bare name, no module dimension | **Pool type = `HashMap<String, HashSet<String>>`**: key = module key, value = the set of reference names of that module; the consumer at `arms.rs:603-609` unions all module entries except the current unit and feeds them to `DeadCodeAnalyzer`; a module's own name (top-level definition and local binding, `use` imported names excepted) no longer exempts other modules' same-name definitions   | `src/driver/arms.rs:603-609` (consumer side `other_module_refs`); assembly single entry `src/driver/helpers.rs:429-449` (`collect_ref_pool`) |
| 3   | Script reuses Lib's pool (violates single-file semantics)   | **Script does not connect to the pool**: `match us.role` at `arms.rs:611-623` only assembles for `Bin`/`Internal` (`set_exempt_pub`) and `Lib` (`set_cross_file_refs`); Script/Test goes to the `_ => {}` default arm; the latter only judges by the file's internal definition-use graph                                                                                                               | `src/driver/arms.rs:611-623`                                                                                                                 |

**Items left for subsequent rounds of WBS 4.9** (outside this round's inScope):

- **Consumption definition refinement (4.9.4)**: distinguish "imported" from "really used after
  import"—not in this round, depends on `use` item path resolution (prerequisite = 4.6.1, already
  delivered by `39546020`).
- **Test pool separation (4.9.5)**: tests/ references form a separate pool, do not exempt unused
  definitions of production code (src/).

**Reason for not changing in P3.5** (consistent with the previous version): module-level resolution
requires `use` item path resolution completion (`use a::foo` → foo comes from module a), which
belongs to the P4-stage contract and Driver unification scope.
