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

> **Implementation amendment (2026-10-08, RFC-029g)**: The `pub` keyword has been removed in its
> entirety by RFC-029g. All statements in the body of this article about "`pub` exemption / `pub`
> files / unused-pub warnings" should be read as **"top-level bindings + in-package reference
> pool"**: the role classification (Script / Bin / Lib / Internal / Test) and the import surface
> semantics are unchanged—only the criteria for dead-code exemption have changed (see
> [029g](029g-remove-pub-and-auto-bind.md) and the determination standard in
> `docs/src/reference/warning-code/warning-codes.md`).

This fulfills the extension slot in RFC-029's "Sub-RFC Plan" (slots 029b–029e are taken, so this
extends to 029f): it defines a **compilation target role model** for source files (five kinds:
Script / Bin / Lib / Test / Internal) along with the **import surface semantics**—when there is no
manifest, the role is inferred from entry reachability (zero configuration); when a manifest is
present, the role is declared explicitly by `[lib]` / `[[bin]]` / `[exports]` (fields already
defined in RFC-015). It fills four dangling semantic gaps: what counts as the cross-package import
surface, the applicable scope of the `pub` exemption for dead-code warnings (Decision B in #321),
the relationship between `[exports]` and Registry reachability, and the disambiguation of terms
between RFC-014b's `[binaries]` (pre-compiled distribution artifacts) and the bin-role source files.

## Motivation

### Host and Boundary Basis

The entry selection and visibility decisions in RFC-029 (accepted) are the direct upstream of this
RFC:

> **Entry file selection** priority: 1. `[run].main` 2. The `path` of the first `[[bin]]` 3.
> `src/main.yx` (conventional default). — RFC-029 §Inter-package cycles: Continued

> **Visibility**: None. Scope + distribution boundary covers all scenarios; no new keyword is
> needed. — RFC-029 §Design Decision Record (2026-07-30)

The second statement is the design red line of this RFC: the import surface semantics **must not
introduce a new visibility keyword**; the "distribution boundary" (file role + declared export
surface) alone must carry the weight.

Each of the four consumers hit the same wall:

| Consumer                                           | The wall it hit                                                                                                                                                                                                                                                     |
| -------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| #321 Dead-code warning                             | Decision B "pub = external interface, never reported" exists because there is no role model to distinguish "truly external" from "looks external"; whether unused `pub` in a bin should be reported (Plan A) has been suspended until now                           |
| RFC-014c Workspace (under review, #113)            | When member packages `use` each other, what is the import surface—`[exports]`, the single `[lib]` file, or all `pub` files—**there is not a single semantic clause in the entire document**                                                                         |
| RFC-015 Configuration system (accepted)            | The `[lib]` / `[[bin]]` / `[exports]` fields are already defined; aside from RFC-029's consumption of the entry priority, the semantics of the remaining fields (especially the relationship between `[exports]` and Registry reachability) have never been aligned |
| RFC-037 Packaging / RFC-029a Caching (draft, #293) | Selecting packaging artifacts and defining cache-unit boundaries for incremental recompilation both need the role model as a prerequisite                                                                                                                           |

### The Current Problem

**Four semantic gaps** (inventory on 2026-09-12, confirmed by searching the full text of
014/014a/014b/014c/015/029):

1. **The cross-package import surface is undefined**. The 014c example of member packages importing
   one another only has a directory layout (`src/lib.yx`); there is no clause stating what
   `use utils.helper` can obtain. The current implicit rule is "all top-level `pub` files in the
   package"—internal implementation files are dragged into the namespace too, making the dependency
   boundary a fiction.
2. **The dead-code `pub` exemption has no applicable scope**. Decision B from #321, "pub is never
   reported," is too conservative in single-file scripts (scripts have no external consumers, so
   `pub` is meaningless) and too lenient in the entry files of multi-file projects (a `pub` in the
   root file that no one imports is dead code). Without a role model, the line between Plans A and B
   cannot be drawn.
3. **Three export concepts have never been aligned**. `[exports]` (015, path mapping), `[lib]` (015,
   single file path), and Registry reachability (029, reachable from the entry along `use`)—are they
   in an inclusion relationship, an equality relationship, or do they govern separate domains? No
   clause.
4. **Term collision**. RFC-014b's `[binaries]` refers to **pre-compiled distribution artifacts** (an
   .so/exe download priority policy), which is an entirely different layer from the "bin target
   (source-code role)". Once 014c lands, the two will inevitably appear on screen together, and the
   failure to disambiguate now will continue to cause confusion.

**Empirical evidence: the conservatism of Decision B is already a real pain point**. The #321 M2
retrospective notes that the structural reason W1001–W1005 are all silent under the default
configuration is precisely that "all `pub` items are treated as external interfaces"—this is
compensatory behavior caused by the missing model, not a terminal state.

## Proposal

### Core Design: The File Role Model

**The role is a file-level property** (consistent with 015's field granularity: `[lib]` is a single
file, `[[bin]]` is a list of files, `[exports]` is a file map), not a package-level switch.

| Role         | Determination                                                                                                                                                   | Semantics                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Bin**      | The file pointed to by `[run].main` / `[[bin]].path`; or (without a manifest) a file that contains `main` and is not `use`d by any other file                   | Program entry. Requires a binding named `main`, which may be a value or a function: a function `main` is called with zero arguments at the entry phase, and a value `main` is evaluated at the initialization phase and that is its execution (even if the evaluation result is a function value, it is not called again). Missing `main` is a compile error (E3020); no executable statements are allowed at the top level. Its `pub` is **reportable as dead code** (no out-of-package consumer); `main` is the reachability root |
| **Lib**      | The file hit by an `[exports]` mapping; or `[lib].path`; or (without a manifest) a file that is `use`d by other files                                           | Distribution boundary. Its `pub` is **exempt from dead code** (out-of-package consumers cannot see it; prefer false negatives); symbols exported through it are visible across packages                                                                                                                                                                                                                                                                                                                                             |
| **Test**     | Files hit by RFC-036's test-file rules (the `tests/` directory, `*_test.yx`, and other existing conventions); **no `[[test]]` explicit declaration is defined** | Test code. Does not participate in dead-code determination; its references are counted as reachability roots of the code under test                                                                                                                                                                                                                                                                                                                                                                                                 |
| **Internal** | When a package has a manifest, a file that is neither on the export surface nor contains `main`                                                                 | In-package implementation. `pub` is exempted to phase 2 (tightened by the in-package use graph, with verification against Bin-role corpus as a prerequisite); out-of-package `use` is **unreachable**                                                                                                                                                                                                                                                                                                                               |
| **Script**   | A file that is run directly as a single file (`run foo.yx`)                                                                                                     | **No entry concept**. Top-level statements (including binding initializations) are the program body in source order; `main` is an ordinary binding, **not auto-called**—to run it, write `main()`. The Registry contains only std                                                                                                                                                                                                                                                                                                   |

**Determination priority**: explicit manifest declaration > entry-reachability inference > 036 test
convention. Explicit declaration always wins—inference is only the default when there is no
declaration.

### Entry Determination

The role model not only feeds dead-code analysis, it also **drives entry determination**. The entry
rules per role:

| Role                      | Entry rule                                                                                                                                                                                                                                                                           |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Script** (no manifest)  | **No entry concept**. Top-level statements (including binding initializations) are the program body in source order; `main` is an ordinary binding, **not auto-called**—to run it, write `main()`                                                                                    |
| **Bin** (with manifest)   | Requires a binding named `main` (a value or function: a value `main` is evaluated at the initialization phase, a function `main` is called with zero arguments at the entry phase); missing `main` is a compile error (E3020). No executable statements are allowed at the top level |
| **Lib / Internal / Test** | `main` is not required (they are not entries)                                                                                                                                                                                                                                        |

**Script and Bin must be mutually exclusive at the entry**: "in Script, `main` is not special" and
"in Bin, `main` is the entry" cannot both hold—if Script both executes top-level statements and
implicitly calls `main`, a script that explicitly writes `main()` would run twice (#356). Therefore
Script has only one execution entry: the top-level statements.

**Distinguishing role determination from entry determination** (implementation note): entry
determination uses "whether a manifest exists" (`find_project_root().is_some()`) rather than
`roles::classify()`. The reason: `classify` answers "who consumes this file" (serving dead-code
analysis); a file with a manifest but no `main` is classified as Internal in `classify` (it is not
`use`d by any other file, which is correct), but the user runs it as an entry, in which case there
must be an entry. Two different questions, two different criteria.

**Execution form of a value `main`** (Decision in #388): functions are already values (unified at
the type layer), so the entry is determined by **the presence of the binding**, not by callability—a
value `main` and a function `main` only differ in their evaluation strategy. A value `main` does not
get an entry call (the #356 double-execution guard: initialization and entry call are never
stacked), and has the same execution form as Script; its evaluation is part of the initialization
sequence: executed in topological order of global bindings, with independent bindings in source
order. Therefore, wrapping Script's top-level statements into `main = { ... }` and migrating them
into Bin is **not an order-preserving migration**—scattered statements execute in source order,
while binding initializations execute in topological order; any initialization that needs to run
first should be made an explicit dependency of `main`. A non-callable value main like
`main: Int = 5` is a legal program with no observable effect (analogous to Rust's empty
`fn main() {}`).

See `docs/src/reference/language-spec/syntax.md` §3.11.

### Examples

```toml
# yaoxiang.toml (all fields are already defined in RFC-015; this RFC adds no new configuration surface)
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
pub unused_fn = (x: Int) => x    # ← W1001 reportable: bin has no out-of-package consumers
main: () -> Void = { ... }

# src/lib.yx (Lib role, on the export surface)
pub api_fn = ...                 # ← never reported: out-of-package consumers cannot see it

# src/other.yx (Internal role, not on the export surface)
pub semi_api = ...               # ← exemption until phase 2 (prefer false negatives), tightened later by use graph
```

### Import Surface Semantics

The resolution order for cross-package `use pkg.x` (the first hit takes effect):

1. If an `[exports]` mapping exists → import surface = the set of files listed in the mapping, and
   `x` must be among the top-level bindings of these files;
2. Otherwise, if `[lib].path` exists → import surface = that single file;
3. If neither exists (no manifest dependency, path-based bare directory) → **status quo**: all
   top-level files are importable (compatible with existing behavior; tightening will be discussed
   separately).

**Workspace members**: The export surface is defined solely by the member package's own manifest;
the workspace root must not override or extend a member's export surface—member self-containment is
the core design of 014c, and root override would break encapsulation.

The relationship with Registry reachability (aligned with 029): `[exports]`/`[lib]` define the **set
of legitimate starting points for cross-package resolution**; once the starting points are
determined, the Registry still expands along 029's "reachable from the entry along `use`"—internal
files `use`d by export files are pulled in through linking, but they do not create new cross-package
starting points.

### Boundaries with Existing Plans

| Neighbor                                          | Boundary                                                                                                                                                                                                                                                                                    |
| ------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| RFC-029d "CLI `--entry` override entry" (planned) | 029f defines the role model; 029d consumes it—`--entry` is a CLI-level override for the Bin role                                                                                                                                                                                            |
| RFC-014b `[binaries]`                             | **Term disambiguation**: `[binaries]` refers to pre-compiled distribution artifacts (download-priority policy), while `[[bin]]` is a source-code role declaration. The former lives in a dependency package's manifest, the latter in this package's manifest; they are not interchangeable |
| RFC-014c Workspace                                | The import surface for member-package mutual imports = this RFC's resolution order; workspace coordination mechanisms (members, shared lockfile) still belong to 014c                                                                                                                       |
| RFC-036 Testing                                   | The Test role determination directly adopts 036's existing file rules; no new convention is added                                                                                                                                                                                           |
| #321 Dead code                                    | Decision B ("all `pub` exempt") is explicitly downgraded to "transitional semantics when there is no target"; reporting `pub` in the Bin role is the form that fulfills Plan A from #321                                                                                                    |

## Detailed Design

### Role Determination Algorithm (in the Orchestrator)

```
fn classify(files, manifest, entry_reach) -> Map<File, Role>:
    # 1. Explicit layer: manifest declarations take priority
    for f in manifest.exports.values():  role[f] = Lib
    if manifest.lib_path:                role[lib_path] = Lib
    for b in manifest.binaries:          role[b.path] = Bin
    if manifest.run_main:                role[run_main] = Bin

    # 2. Inference layer: by entry reachability where no declaration exists
    for f in files where role[f] 未定:
        if f 被 ≥1 个非自身文件 use:      role[f] = Lib
        elif f 含 main 且无其他文件 use 它: role[f] = Bin
        else:                             role[f] = Internal

    # 3. Test layer: 036 rule hit → Test (overrides Lib/Internal, does not override explicit Bin)
    apply_rfc036_test_rules(files, &role)

    # Single-file direct run: entire model bypassed, behavior unchanged
    if no manifest and single_file:      all Script
```

### Compiler Changes

- `frontend/config.rs`: manifest parsing adds a mapping from `[exports]` to the role table (field
  parsing is already in 015).
- `frontend/module/orchestrator.rs`: insert a role-classification phase before Registry
  construction; cross-package resolution validates `use` targets in import-surface order, and
  out-of-bounds references report the existing `module_not_found` family (no new error code;
  messages add a "not on the export surface" hint).
- `typecheck/passes/dead_code.rs`: the entry-point set changes from "main + all `pub`" to "main +
  the `pub` of Lib-role files" (Bin is immediately reportable; Internal is exempt until phase 2, see
  implementation strategy).
- LSP (RFC-017): completion/hover "importable items" are filtered by the import surface; the `main`
  missing diagnostic is reported only for Bin-role files.

### Runtime Behavior

No change. The role model is purely a compile-time/parse-time semantic and does not affect the IR or
execution.

### Backward Compatibility

- Projects without a manifest: behavior is completely unchanged (the inference layer restores the
  status quo—what's `use`d is Lib, what contains `main` is Bin). The only dead-code warning
  increment is in Bin files: an unused `pub` in a root file that no one imports changes from silent
  to W1001—this is the expected behavior of Plan A in #321, not a break.
- Projects with a manifest: when `[exports]` is already declared, the import surface narrows from
  "all `pub` files" to the declared surface. **This is the only behavior-narrowing point**: code
  that depends on a peer package's internal files will start to report `module_not_found`. Given
  that the package ecosystem has not yet been established (029: "currently no third-party package
  ecosystem"), the break surface is zero; a minor-version transitional period is still provided
  (out-of-bounds first warns, then errors).

## Trade-offs

### Advantages

- Zero new configuration surface: all fields are already defined in 015; this RFC only adds
  semantics.
- Zero new keyword: the import surface is carried by the distribution boundary, isomorphic to 029's
  decision that "visibility does not exist."
- Four consumers (#321 / 014c / 029a / 037) are modeled at once, no longer each hitting their own
  wall.
- The zero-config path for no-manifest projects is preserved intact.

### Disadvantages

- Role inference ("being `use`d means Lib") in the common pattern of "entry file `use`s a utility
  file" will classify the utility file as Lib, whose `pub` is exempt—coarser than the ideal
  granularity. This is an intentional choice in the false-negative direction; tightening requires
  call-direction analysis on the use graph, which is not done in version 1.
- Narrowing the import surface via `[exports]` is a behavior-tightening change with break risk
  (despite the transitional period).

## Alternatives

| Alternative                              | Description                                                                        | Reason for not adopting                                                                                                                                                                                                                    |
| ---------------------------------------- | ---------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Top-level RFC-040                        | Open a new top-level number                                                        | Module-graph semantics (import surface / reachability / entry) all live in the 029 domain; splitting to a top-level would produce two top-level documents for the same object; moreover, the 029b–029e slot pattern is already established |
| Attach to 014c                           | As a section in the workspace                                                      | The dependency direction is reversed: the language layer (029x) defines the semantics, the package layer (014x) consumes them. 014c is still under review; expanding scope mid-review would delay its landing                              |
| Plan A verbatim (the #321 menu)          | Force `[[bin]]`/`[[lib]]` targets in the manifest before dead-code semantics apply | Introduces a mandatory configuration for a narrow slice (unused `pub` in a root file with `main`); this RFC's inference layer achieves the same benefit with zero configuration                                                            |
| Pure entry inference (no manifest layer) | Don't consume 015's fields                                                         | `[exports]` is already an accepted field in 015 and the cross-package import surface cannot avoid it; leaving it undefined leaves a gap for 014c                                                                                           |

## Implementation Strategy

### Phased

- **Phase 1**: Role classification + import-surface resolution + Bin-`pub` reportable (Internal/Test
  remain exempt)
- **Phase 2**: Internal `pub` is tightened to "reportable when unreachable in the in-package use
  graph"—the prerequisite is that after Phase 1 lands, the Bin-role warning corpus is verified to
  have no false-positive regressions; the work is triggered by that condition, with no fixed
  timeline

### Dependency Relationships

- Prerequisites: the RFC-029 orchestrator (already landed), RFC-015 manifest fields (already
  defined).
- Dependents: #321 (Bin-`pub` reportable), RFC-014c (import surface), RFC-029a (roles as cache-unit
  boundaries), RFC-037 (artifact selection), RFC-029d (`--entry` override).
- Landing order with RFC-014c: before 014c passes review, 029f's import-surface order should be
  finalized first; 014c consumes it by reference, avoiding scope expansion mid-review.

### Risks

- Deviation between role inference and user intuition (the first trade-off above) → present all
  incremental behavior as warnings (non-blocking), disable-able at any time.
- Transitional management of the `[exports]` narrowing → warn first within a minor version.

## Open Issues

All three draft-stage items have been decided (2026-09-13; the decisions are recorded in Appendix B
and the body); no open items remain:

- [x] Should Internal `pub` be tightened? → **Decision**: tightened in phase 2 by in-package
      use-graph reachability, with verification against Bin-role corpus as a prerequisite (see
      Implementation Strategy Phase 2)
- [x] `[[test]]` explicit declaration → **Decision**: not introduced; the Test role is determined
      solely by 036's rules; if 036 ever needs explicit declaration, it will be extended by itself
- [x] Should the workspace root override the members' export surface? → **Decision**: not allowed;
      member self-containment is the core design of 014c, and root override would break
      encapsulation

---

## Appendix B: Design Decision Record

| Decision                                               | Resolution                                                                                           | Date       | Recorder | Basis                                                                                                                      |
| ------------------------------------------------------ | ---------------------------------------------------------------------------------------------------- | ---------- | -------- | -------------------------------------------------------------------------------------------------------------------------- |
| Numbering                                              | 029f (029b was reserved for hot reload, 029c was deleted, 029d/029e are taken)                       | 2026-09-12 | Chenxu   | RFC-029 §Sub-RFC Plan                                                                                                      |
| Belongs to the 029 family, not top-level RFC           | Yes                                                                                                  | 2026-09-12 | Chenxu   | Import surface / reachability / entry all belong to module-graph semantics; 029 already consumes `[[bin]]` entry priority  |
| Visibility keyword                                     | Not introduced                                                                                       | 2026-09-12 | Chenxu   | Isomorphic with RFC-029's "visibility does not exist" decision; the import surface is carried by the distribution boundary |
| Status of #321 Decision B                              | Downgraded to "transitional semantics when there is no target"                                       | 2026-09-12 | Chenxu   | Reporting `pub` in the Bin role is the form that fulfills Plan A of #321                                                   |
| Role granularity                                       | File-level                                                                                           | 2026-09-12 | Chenxu   | 015's field granularity (`[lib]` is a single file, `[[bin]]` is a list, `[exports]` is a map)                              |
| `[binaries]` disambiguation                            | 014b's pre-compiled distribution artifacts ≠ `[[bin]]`'s source-code role                            | 2026-09-12 | Chenxu   | Two layers of concepts (download policy vs. compilation role); pin this down before 014c lands                             |
| Internal `pub` tightening                              | Phase 2: tightened by in-package use-graph reachability; prerequisite = Bin-role corpus verification | 2026-09-13 | Chenxu   | Gradual tightening in the false-negative direction; corpus-driven to avoid premature false positives                       |
| `[[test]]` explicit declaration                        | Not introduced; Test is determined solely by 036's rules                                             | 2026-09-13 | Chenxu   | Don't add a configuration surface before 036 needs it                                                                      |
| Workspace root overriding the members' export surface  | Not allowed                                                                                          | 2026-09-13 | Chenxu   | Member self-containment is the core design of 014c; root override would break encapsulation                                |
| Whether `main` is auto-called under Script             | **No auto-call**—top-level statements are the program; `main()` must be explicit                     | 2026-09-17 | Chenxu   | Having both rules would let explicit `main()` run twice (#356); Script can have only one execution entry                   |
| Whether the role model is used for entry determination | **Yes** (previously only used for dead-code warnings)                                                | 2026-09-17 | Chenxu   | This RFC defines Bin's "`main` is the reachability root"; without connecting to entry, that semantic sits idle             |

## Appendix C: Glossary

| Term                  | Definition                                                                                                                                |
| --------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| Role                  | The compilation target identity of a source file: Script / Bin / Lib / Test / Internal                                                    |
| Import Surface        | The set of files that serve as legitimate resolution starting points for cross-package `use`, declared by `[exports]`/`[lib]` or inferred |
| Distribution boundary | 029 terminology: the externally exposed range of a package; this RFC refines it from implicit (all `pub` files) to the import surface     |
| Script state          | The bypass form of a no-manifest single-file direct run; all behavior matches the status quo                                              |

## References

- RFC-029 Module semantics (parent RFC: entry selection, visibility decision, sub-RFC plan)
- RFC-015 Configuration system (definition of `[lib]`/`[[bin]]`/`[exports]` fields)
- RFC-014b Build system and binary distribution (`[binaries]`, the disambiguation target)
- RFC-014c Workspace support (the first consumer of the import surface, under review)
- RFC-036 Testing framework (source of the Test role rules)
- #321 M2 Independent emission channel for warning codes (the origin of Decision B and Plan A)

## Known Defects: Implemented (2026-10-16, WBS 4.9 Round 1)

> The previous version (2026-10-09) listed the following three items as "three design defects of the
> reference pool". All three have been landed one by one by WBS 4.9 Round 1 (commit `3863a9ff0`);
> `collect_project_refs` and `orchestrator.rs` have been split into
> `src/driver/{state,arms,helpers}.rs` along with P4 4.2.9, and all `file:line` references in the
> table below are re-fetched on the cited revision.

### The Three Design Defects of the Reference Pool — Fix Status

| #   | Original defect                                                       | Current status (2026-10-16)                                                                                                                                                                                                                                                                                                                                                                                                                 | Landing code (rev `3863a9ff0`)                                                                                                                   |
| --- | --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1   | The scan surface is a directory rather than a compilation surface     | **The pool's root = package target**: `collect_package_refs` (`helpers.rs:481`) calls `discover_with_used` on the compilation set based on the `[[bin]]`/`[lib]`/`[exports]`/`[run].main` roots of `roles::ExplicitSurfaces`; it **does not scan directories to find entries**. When none of the roots are reachable, it falls back to the compilation set of the file being checked itself (`collect_compilation_refs` / `helpers.rs:454`) | `src/driver/helpers.rs:454-512` (fallback path); `src/driver/helpers.rs:481-512` (manifest-declared-surface path)                                |
| 2   | The granularity is project-level bare names, with no module dimension | **Pool type = `HashMap<String, HashSet<String>>`**: key = module key, value = the set of reference names for that module; the consumer side in `arms.rs:603-609` unions the entries of "all modules other than the current unit" and feeds them to `DeadCodeAnalyzer`; a module's own names (top-level definitions and local bindings, with the exception of `use`-imported names) no longer exempt same-named definitions in other modules | `src/driver/arms.rs:603-609` (consumer-side `other_module_refs`); the single assembly entry `src/driver/helpers.rs:429-449` (`collect_ref_pool`) |
| 3   | Script reused the Lib pool (violating single-file semantics)          | **Script does not receive the pool**: the `match us.role` in `arms.rs:611-623` only assembles for `Bin`/`Internal` (`set_exempt_pub`) and `Lib` (`set_cross_file_refs`); Script/Test take the `_ => {}` default arm; the latter only judges by the file's internal definition-use graph                                                                                                                                                     | `src/driver/arms.rs:611-623`                                                                                                                     |

**Follow-up improvements 4 / 5 — Fix status (2026-10-16, WBS 4.9 Round 2)**

> These two items were originally listed as "to be landed in P4 4.9". Both have been landed by WBS
> 4.9 Round 2 (commit `e6cf1aeaa`, pushed 2026-10-17); the `file:line` values in the table below are
> re-fetched on revision `e9675c9dd` (the 2026-10-17 Round 3 closure supplementary note—both the fix
> round after the Round 2 landing and the closure round modified the same batch of anchor files, so
> line numbers follow the re-fetched values; Round 1 lesson: the pre-push local hash gets rewritten
> by `pull --rebase`).

| #   | Original improvement                                                                                                                    | Current status (2026-10-16)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | Landing code                                                                                                                                                                                                                                                                                            |
| --- | --------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 4   | Consumer definition refinement: distinguish between "imported" and "actually used after import" (only the latter counts as consumption) | **Consumption criterion = "actually used after import"**: a consumption record = some module uses `use` to bring a name n from module M into scope **and actually references n at a use site**; only recorded against M (`RefPool::consumed_from`); names that only appear in `use` items (no use site), and references by other modules to **same-named** identifiers, do not count as consumption. The import target module is determined by the module-key set at discovery time (`resolve_import_module`, **not a separate path-resolution pass**—reusing the 4.6.1 discovery-time resolution output, see `39546020`); the namespace form (`use lib` + `lib.foo()`) is attributed the same way | `src/driver/helpers.rs:442-447` (`RefPool`); `src/driver/helpers.rs:449-506` + `:511-524` (`collect_ref_pool` + `resolve_import_module`); `src/frontend/core/typecheck/passes/dead_code.rs:186-200` (`ModuleNames`), `:384` (`collect_module_names`), `:132-184` (`ImportFace` / `collect_import_face`) |
| 5   | Test pool separation: references in tests/ do not exempt production code (src/)                                                         | **`FileRole::Test` does not contribute to the consumption surface**: Test modules' references form a separate pool (Test itself does not participate in dead-code determination, so its references do not leak out); a production code's definition can only be rescued by a production code's consumption. The role is determined at a single point by `roles::classify` (test discovery rules from RFC-036)                                                                                                                                                                                                                                                                                      | `src/driver/helpers.rs:463-466` (Test short-circuit in `collect_ref_pool`); `src/driver/helpers.rs:707-708` (single-point determination by `roles::classify`); `src/driver/arms.rs:633-635` (Test skip shared by the fallback and declared-surface paths; see `:661-672` for the role feeding the pool) |

**Residual surfaces (recorded, non-blocking)**: ① Field/method-by-position references (only
`obj.render()` has the short name) cannot, at the AST layer, be attributed to the module of the
receiver's type—when a base name hits a namespace import it is attributed via the import,
module-qualified construction `ns.T` is attributed by the **type-name literal** to the method
attribution (see §4.9.7 below), and the rest are conservatively recorded into `RefPool::field_names`
by name dimension (in `collect_ref_pool` of `src/driver/helpers.rs`); ② Method reachability:
tightened per-candidate and per-definition-unit by the D20 resolution output
(`resolved_method_marks`)—**within the same unit**, unreached sibling overloads are reported as
W1005 as-is; **across units, uncalled same-named same-order definitions in reachable form** are
likewise reported as-is (WBS 4.9.7 / issue #464, closed; see the mechanism and empirical evidence
below; the production-side commit is `b8b3cc3f6`). **Residual boundaries (recorded, do not claim
universality; all by empirical evidence)**: (a) a standalone `check <definition file>` (the file has
no call site and the caller is not in the unit set) ⇒ no resolution entries ⇒ falls back to the
pool's **name-level exemption** (`RefPool::field_names` is globally flat) ⇒ **not reported**; (b)
the same bare type name is brought into scope by multiple modules (the source program is already
ambiguous at the name layer, with the entry additionally reporting E5006) ⇒ the entire candidate
surface is skipped by name ⇒ no resolution entries ⇒ same-named same-order groups are not reported;
(c) when the caller in this unit has made **any method call** on another unit's type name (even if
it does not hit that other side's definition), the other side's definition is merged by **possible
attribution** (rule ② of `typed_owners`) ⇒ not reported; when both sides only construct and make no
method calls at all, `typed_owners` does not take effect, and both sides report W1005 as-is (t20
empirical fixture: both sides only construct ⇒ 2 W1005 entries); (d) overall import `use M` does not
bring a bare type name into scope, and the std module's candidate surface is not connected—both
forms produce no entries, and fall back to name-level exemption. Fallback boundary: when a name
group has no resolution record in this unit (e.g., E1107 from an ambiguous call site, call forms
that bypass resolution), it falls back to short-name joint determination (false-negative
direction)—so we do not claim "all uncalled methods are reported as-is."

### Reference Count Distortion (WBS 4.9.6 / issue #450) — Fix Status (2026-10-16)

> Original record: an overloaded method **is called and the resolution takes effect, but is still
> falsely reported as `W1005 Unused method`** (the reference count is distorted for overload calls).
> On 2026-10-16, the user ruling A registered it as an independent tier-3 task and merged it into
> WBS 4.9 Round 2; this item is Type D (deleting a false positive changes the diagnostic external
> surface) and has been authorized to land.

| Link                    | Original defect                                                                                                                                                                                                                                                         | Current status                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Landing code                                                                                                               |
| ----------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| Reachability probe      | The scan looks up references by **qualified name** (`Point.render`), while what the use-site collection gets is the **short name** (`obj.render()` only has `render`) ⇒ a called method is never reachable, leading to false-positive W1005 in the no-project-root path | Methods are judged by **short-name probe** on the call surface and narrowed per candidate by the **D20 resolution output** (`resolved_methods` + `resolved_groups`; sibling overloads that are not hit by the resolution are still reported as W1005). Scope boundary: **within the same unit**, per-candidate holds; **across units (registry resolution) calls** after the production-side (`b8b3cc3f6`) and driver-side attribution land, also hold per-candidate and per-definition-unit in reachable form (see residual surface ② and §4.9.7 below) | `src/frontend/core/typecheck/passes/dead_code.rs:782-792` (probe); `:815-877` (per-definition judgment and exemption gate) |
| Definition table keying | `all_defs` is keyed by qualified name ⇒ **same-named overloads overwrite one another** (two overloads leave only one, reporting only 1 entry)                                                                                                                           | Definition table is **per-definition**: methods are appended; same-named non-method bindings are deduplicated by name and the last is kept (each overload is an independent definition, while top-level reassignment of the same name is the same binding)                                                                                                                                                                                                                                                                                               | `src/frontend/core/typecheck/passes/dead_code.rs:304-322` (`record_definition`), `:28-29` (fields)                         |

**Reason for not changing in P3.5 (consistent with the previous version)**: module-level resolution
requires the path resolution of `use` items to be complete (`use a::foo` → foo comes from module a),
which belongs to the P4-stage contract and the Driver unification scope.

### Cross-Unit Method Reachability Attribution (WBS 4.9.7 / issue #464) — Fix Status (2026-10-17)

> Original record (residual surface ②): cross-unit (registry resolution) method calls produce no D20
> resolution entries ⇒ that name group falls back to the pool's **name-level** exemption ⇒
> **cross-unit same-named same-order uncalled methods are not reported**. This item was landed in
> two installments: the **production side** (cross-unit method calls produce resolution entries,
> commit `b8b3cc3f6`; its injection surface was filtered pair-by-pair by name attribution in the fix
> round, see the table below) and the **driver-side attribution** (produces reachability marks per
> definition unit and deactivates the name-level exemption). The `file:line` in the table below are
> taken from the **driver-side attribution worktree**—commit `d5c00235a`; the production-side commit
> is `b8b3cc3f6` (see above). All `file:line` in this section are uniformly taken from revision
> `e9675c9dd` (re-fetched in the 2026-10-17 Round 3 closure, covering the injection-surface interval
> shifted by the post-`d5c00235a` fix round `d5dd1a37b`); commit hashes are the post-push truth (the
> pre-push local hash gets rewritten by `pull --rebase`, see 09a).

| Link                        | Original defect                                                                                                                                                                                                                                                  | Current status                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Landing code                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| --------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Resolution entry production | All three candidate tables are missing at cross-unit call sites (`method_overloads` / `method_overload_ir_names` / `method_overload_param_names`) ⇒ the resolution section is skipped entirely ⇒ **0 entries**                                                   | Candidate groups "candidate / mixed IR name / declared-formal-parameter name" are packaged in same order and propagated across units via `ModuleInfo.method_overloads`; the registry merges them in module-key ascending order with determinism; the Typecheck arm, based on the local import face, takes candidates **pair-by-pair**—only injecting the candidate group whose "key-providing module == the providing module of that type-name prefix in this unit's scope" (name ← module pairing, no cross-pair combination; own-key slots are not merged out, and same-name conflicts are skipped by entire name) ⇒ at the same site `cands=1`, with 1 entry (`Temp.to`). **Fix-round empirical evidence (F1)**: the previous "name set × module set" combination would fold a same-named type candidate defined in another module into the same key slot ⇒ under `use a.{Temp}` + `use b.{Other}` (a also defines Other, b also defines Temp, methods same-named same-order) `check` reports 2×E1107 and exits with 1 (a program that previously succeeded now fails to compile); after the pair-by-pair filter, both `check`/`run` exit with 0; fixture archived at `tests/yaoxiang-multifile/ref-pool-cross-unit-name-owner/` | `src/frontend/core/typecheck/environment.rs:145` (`merge_from`: same-order append at the candidate-group level), `:336` (snapshot), `:384` (injection), `:399` (`append_overload_group`: the **only** landing point for the three parallel tables, with an equal-length debug assertion before landing); `src/frontend/module/registry.rs:150` (`merge_method_overloads`); `src/frontend/module/mod.rs:155-157` (`ModuleInfo` fields); `src/driver/arms.rs:344-358` (injection surface), `:1521-1579` (`visible_method_overloads`, single injection point: name ← providing module pair-by-pair, same-name conflicts skipped by entire name, own-key slots not merged out). **Fix-round (F1, `d5dd1a37b`) of the injection surface**: the pre-fix "name set × module set" combination folded another module's same-named type candidate into the same key slot ⇒ under `use a.{Temp}` + `use b.{Other}` (a also defines Other, b also defines Temp, methods same-named same-order) `check` reports 2×E1107 and exits with 1 (a program that previously succeeded now fails to compile); after the pair-by-pair filter, both `check`/`run` exit with 0; fixture archived at `tests/yaoxiang-multifile/ref-pool-cross-unit-name-owner/`, pinned test at `src/driver/tests/overload_injection.rs` (placement commit `c2f887843`) |
| Driver-side attribution     | Resolution entries are marked globally by **name** (one set of `(base name, declared order)`) ⇒ the same-named same-order definition in another unit is exempted by the same-named sibling's call (cross-unit same-named same-order was **not reported** before) | Marks are attributed by **definition unit** (`MethodMarks { by_unit, groups }`): the `{Type}` in an entry name `{Type}.{method}#k` is the **definition-side** bare name and carries no unit ⇒ the recorder-side unit's **type-name surface** is used to compute "the set of modules that this unit can spell this name with", and the entry is placed into `by_unit` for each of those units; `groups` = the name groups (global) that have ever had an entry                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | `src/driver/helpers.rs:569-573` (`MethodMarks`), `:591-619` (`typed_owners`: brace items + inline-alias targets + module-qualified access `ns.T`, reusing `collect_module_names` + `resolve_import_module` at a single point), `:621-654` (`resolved_method_marks`, read-only via `OverloadResolutions::iter`), `src/driver/arms.rs:650-658` (per-unit `by_unit[unit_key]`), `:649-660` (injection into the analyzer)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| Judgment surface            | Name groups that have resolution records still go through the pool's **name-level** exemption (short name is globally flat)                                                                                                                                      | Name groups that have resolution records **bypass the name-level exemption**: a hit on this unit's declaration is alive, and misses are reported as W1005 as-is (`!marked_group` is the precondition for the exemption gate)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | `src/frontend/core/typecheck/passes/dead_code.rs:44` (field), `:252-257` (`set_resolved_groups`), `:262-267` (`method_group_has_marks`), `:782-795` (per-candidate reachability), `:815-877` (per-definition judgment / exemption gate / W1005 emission)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |

**Externally observable changes (Type A: previous non-success ⇒ fix along the way, registered in the
same commit)**:

- **Cross-unit overload resolution was previously ineffective**: all three candidate tables were
  missing ⇒ fell back to the cross-unit single-value binding table (`method_bindings`, last-wins on
  same-named definitions) ⇒ **multi-candidate same-named method calls report `error[E1002]`**
  (empirical: `Expected type 'string', found type 'int64'`); after the fix, resolution is
  per-candidate based on actual arguments, and the same fixture empirically reports
  `Type check passed`. The criterion is anchored in the integration case
  `test_multifile_cross_unit_method_overload_resolves_by_argument_type`
  (`tests/integration/multifile.rs`, two candidates with same name and order for Int/String, two
  calls each hit their own)—when the injection surface is turned off, this case turns red on the
  same E1002.
- **Cross-unit same-named same-order uncalled methods in reachable form start to be reported as
  W1005 as-is**: empirical three-file fixture (`main.yx` uses `use a.{Temp}` to qualify the type,
  `use b` only brings b into the compilation set)—`a.Temp.to` has a call site and is not reported,
  `a.Temp.other` is reported, **`b.Temp.to` is reported**; before the fix, the same fixture only
  reported `a.Temp.other` (b's definition was papered over by the pool's name-level exemption).
  Fixture archived at `tests/yaoxiang-multifile/ref-pool-cross-unit-method/` (CLI-reproducible);
  driver-level pin `test_cross_unit_uncalled_same_named_method_is_reported` in
  `src/driver/tests/method_unused.rs`.

**Residual boundaries (recorded, do not claim universality; item-by-item by empirical evidence)**:

1. **A standalone `check <definition file>`** (the file has no call site and the caller is not in
   the unit set) ⇒ no resolution entries ⇒ falls back to the name-level exemption ⇒ **not reported**
   (false-negative direction). Empirical: the same three-file fixture's `check b.yx` only reports
   W1002 for the unused type, and does not report `b.Temp.to`.
2. **Name-layer already ambiguous**: the same bare type name is brought into scope by multiple
   modules (`use a.{Temp}` + `use b.{Temp}`) ⇒ the candidate surface is skipped by entire name ⇒ no
   resolution entries ⇒ the same-named same-order group falls back to name-level exemption and is
   not reported; that entry additionally reports `E5006 Duplicate import`. Empirical control (only
   importing b by namespace) reports the same-kind sibling candidates as-is, proving the criterion
   is not idle.
3. **When the caller in this unit has made any method call on another unit's type name** (even if it
   does not hit that other side's definition) ⇒ the other side's definition is merged by **possible
   attribution** (rule ② of `typed_owners`) ⇒ not reported; when both sides only construct and make
   no method calls at all, `typed_owners` does not take effect, and both sides report W1005 as-is
   (empirical fixture `t20-b3-no-method-call`: both sides only construct ⇒ 2 W1005
   entries)—false-negative direction.
4. **Inline alias `use M.{T as X}` and module-qualified construction `ns.T` do not inject into the
   candidate surface** (production-side narrowing rule ②): the call-site key is still the
   definition-side `T` (empirically both forms' `X(..).to()` / `ns.T(..).to()` land on the key
   `T.to`) ⇒ the precision of `#k` for same-named types is bounded by "the set of modules that the
   recorder can spell," and **does not distinguish which unit's definition was actually selected**;
   precisely because of this, under these forms we **cannot** judge-dead another unit's same-named
   same-order definition (otherwise a real-called method would be falsely reported).
5. **Overall import `use M` does not bring a bare type name into scope** (empirically writing
   `Temp(...)` reports E1001), **and the std module's candidate surface is not connected** (its
   `ModuleInfo` is constructed via the std registration path)—both forms produce no entries, and
   fall back to name-level exemption.

**The driver-side read is read-only**: `resolved_method_marks` reads via `OverloadResolutions::iter`
(does not touch `get`, does not enter the consumption ledger) ⇒ it does not mask the W1090
"consumption must reach" settlement; the driver-level pin
`test_resolved_method_marks_read_leaves_consumption_ledger_untouched` (entries remain in the
`unconsumed()` difference set after the read).

**Closure empirical supplementary note (2026-10-17 Round 3; after the injection-surface fix round
`d5dd1a37b` + pinned test placement `c2f887843`)**: the injection-surface filter is "name ←
providing module" **pair-by-pair** candidate taking (only folding in the candidate group whose
"key-providing module == the providing module of that type-name prefix in this unit's scope")—the
single point is `visible_method_overloads` (`src/driver/arms.rs:1521-1579`, with the production call
site at `src/driver/arms.rs:354`); the downstream landing has only one place (`environment.rs:399`
`append_overload_group`: same-order append to the three parallel tables + equal-length debug
assertion; inter-group append only goes through `merge_from`, at `environment.rs:145`). Mutation
verification: when restoring the cross-pair filter, the driver-level pin turns red on
2×E1107/2×E1053, and the three assertions of the main acceptance fixture remain unchanged one by one
(proving the fix did not regress the main acceptance). Independent verification / re-review uses the
frozen revision pair (pre-fix `d5c00235a` / post-fix `e9675c9dd`) for a dual-binary side-by-side
check: the diagnostic sets for the main acceptance fixture
`tests/yaoxiang-multifile/ref-pool-cross-unit-method/` agree on both sides; the negative controls
(no same-named type / same-named type with different method names) are all green on both sides; the
existing ambiguity behavior for the same bare type name brought into scope by multiple modules
(constructor / value dimension) agrees on both sides; the reverse false-negative surface is covered
by same-named same-order **uncalled** definitions being reported as W1005 as-is (the fix no longer
swallows them).

**Criterion and drift (this batch's empirical evidence, including the closure round)**: driver-level
pin tests—5 attribution-round ones (main acceptance / inline-alias false-positive defense /
module-qualified-name false-positive defense / bidirectional key scope / read-only, in
`src/driver/tests/method_unused.rs`) + 3 injection-surface-round ones (name-attribution pair-by-pair
filter / different method names do not misfire / three tables with equal-length pairing, in
`src/driver/tests/overload_injection.rs`, placement commit `c2f887843`), with the 4 old criteria
from #450 not regressing; 204 IR snapshots with zero drift; corpus baseline 346 → 347 lines
(driver-side attribution round, new fixture `ref-pool-cross-unit-method/`) → 348 lines
(injection-surface fix round, new fixture `ref-pool-cross-unit-name-owner/`), each batch adding only
1 behavior record, with existing lines unchanged byte-by-byte (the baseline
`tests/baselines/corpus-parity.jsonl` has 348 lines at `e9675c9dd`).
