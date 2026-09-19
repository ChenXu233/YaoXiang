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

Fulfilling RFC-029 "Sub-RFC Planning" extension slot (029b–029e are taken, deferred to 029f):
defines the **compilation target role model** (Script / Bin / Lib / Test / Internal, five
categories) for source files and their **import surface semantics** — when no manifest is present,
inferred by entry reachability (zero-config); when a manifest is present, explicitly declared by
`[lib]` / `[[bin]]` / `[exports]` (fields already defined by RFC-015). Fills four dangling
semantics: what counts as the cross-package import surface, the applicable scope of the pub
exemption for dead-code warnings (Decision B of #321), the relationship between `[exports]` and
Registry reachability, and disambiguation between RFC-014b `[binaries]` (pre-compiled distribution
artifacts) and the bin role source-file name.

## Motivation

### Host and Boundary Basis

The entry selection and visibility decisions in RFC-029 (accepted) are the direct upstream of this
RFC:

> **Entry file selection** priority: 1. `[run].main` 2. `path` of the first `[[bin]]` entry 3.
> `src/main.yx` (convention default). — RFC-029 §Inter-package cycles: More on this

> **Visibility**: does not exist. Scope + distribution boundary cover all scenarios; no new keyword
> needed. — RFC-029 §Design Decision Records (2026-07-30)

The second item is the design red line of this RFC: import surface semantics **must not introduce
new visibility keywords**; they can only be carried by the "distribution boundary" (file role +
declared export surface).

Four consumers each hit the same wall:

| Consumer                                           | Wall hit                                                                                                                                                                                                                           |
| -------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| #321 Dead-code warnings                            | Decision B "pub = never warn for external interfaces" is because no role model distinguishes "truly external" from "looks external"; whether unreferenced pub inside bin is warned (Option A) has been shelved                     |
| RFC-014c Workspaces (under review, #113)           | When member packages `use` each other, what counts as the import surface — `[exports]`, `[lib]` single file, or all pub files — **not a single semantic clause in the entire document**                                            |
| RFC-015 Configuration System (accepted)            | `[lib]` / `[[bin]]` / `[exports]` fields are defined; aside from RFC-029 consuming entry priority, the semantics of the remaining fields (especially the relationship between `[exports]` and Registry reachability) are unaligned |
| RFC-037 Packaging / RFC-029a Caching (draft, #293) | Packaging artifact selection and incremental recompilation cache-unit boundaries all need the role model as a prerequisite                                                                                                         |

### Current Problems

**Four semantic gaps** (inventory on 2026-09-12, confirmed by reading the full text of
014/014a/014b/014c/015/029):

1. **Cross-package import surface is undefined**. The member-package mutual-reference example in
   014c has only directory structure (`src/lib.yx`); there is no clause on what `use utils.helper`
   can obtain. The current implicit rule is "all top-level pub files of that package" — internal
   implementation files are dragged into the namespace, making dependency boundaries ineffective.
2. **Dead-code pub exemption has no applicable scope**. The "pub never warned" of Decision B in #321
   is too conservative in single-file scripts (scripts have no external consumers, pub is
   meaningless) and too lenient in entry files of multi-file projects (no one imports the root
   file's pub, that is dead code). Without a role model, the dividing line between Options A and B
   cannot be drawn.
3. **Three export concepts are unaligned**. `[exports]` (015, path mapping), `[lib]` (015,
   single-file path), Registry reachability (029, reachable from entry along use) — whether the
   three have a containment relationship, equality, or each handles its own, no clause exists.
4. **Name collision**. RFC-014b's `[binaries]` is a **pre-compiled distribution artifact** (.so/exe
   download-priority strategy), a completely different layer from the "bin target (source role)".
   Once 014c lands, the two will inevitably appear on the same screen; failure to disambiguate early
   will keep producing misunderstandings.

**Empirical: the conservatism of Decision B is already an actual pain point**. The #321 M2
retrospective records: the structural reason that warning codes W1001–W1005 are entirely silent
under default configuration is precisely "all pub are treated as external interfaces" — this is
compensatory behavior due to model absence, not the end state.

## Proposal

### Core Design: File Role Model

**Role is a file-level attribute** (consistent with the field granularity of 015: `[lib]` is a
single file, `[[bin]]` is a file list, `[exports]` is a file mapping), not a package-level switch.

| Role         | Determination                                                                                                                                      | Semantics                                                                                                                                                                                 |
| ------------ | -------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Bin**      | Files pointed to by `[run].main` / `[[bin]].path`; or (when no manifest) files containing `main` and not `use`d by any file                        | Program entry. Its `pub` **can be reported as dead code** (no out-of-package consumers); `main` is the reachability root                                                                  |
| **Lib**      | Files hit by the `[exports]` mapping; or `[lib].path`; or (when no manifest) files `use`d by other files                                           | Distribution boundary. Its `pub` **exempts from dead code** (out-of-package consumers are invisible; better under-report); symbols exported through it are visible across packages        |
| **Test**     | Files matched by RFC-036 test file rules (`tests/` directory, `*_test.yx`, and other existing conventions); **no** `[[test]]` explicit declaration | Test code. Does not participate in dead-code determination; its references count as reachability roots for tested code                                                                    |
| **Internal** | When the package has a manifest, files neither in the export surface nor containing `main`                                                         | In-package implementation. `pub` exempt until phase 2 (tightened by in-package use-graph reachability, prerequisite: Bin role corpus validation); out-of-package `use` is **unreachable** |
| **Script**   | File directly run as a single file (`run foo.yx`)                                                                                                  | Behavior identical to today: Registry has only std, no dead-code pub semantic changes                                                                                                     |

**Determination priority**: manifest explicit declaration > entry reachability inference > 036 test
conventions. Explicit declaration always wins — inference is just the default when no declaration
exists.

### Examples

```toml
# yaoxiang.toml (all fields are already defined by RFC-015; this RFC adds no new configuration surface)
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

# src/lib.yx (Lib role, in the export surface)
pub api_fn = ...                 # ← never reported: out-of-package consumers are invisible

# src/other.yx (Internal role, not in the export surface)
pub semi_api = ...               # ← exempt until phase 2 (better under-report), tightened by use-graph at that point
```

### Import Surface Semantics

Resolution order for cross-package `use pkg.x` (first match wins):

1. `[exports]` mapping exists → import surface = the file set listed by the mapping; `x` must be in
   the top-level bindings of these files;
2. Otherwise `[lib].path` exists → import surface = that single file;
3. Neither exists (no manifest dependency, path-depends on bare directory) → **maintain status
   quo**: all top-level files are importable (compatible with existing behavior; tightening is a
   separate discussion).

**Workspace members**: the export surface is defined solely by the member package's own manifest;
the workspace root must not override or extend member export surfaces — member self-containment is
the core design of 014c, and root override would break encapsulation.

Relationship with Registry reachability (aligned with 029): `[exports]`/`[lib]` defines the **set of
valid starting points for cross-package resolution**; once starting points are determined, the
Registry still expands according to 029's "reachable from entry along `use`" — internal files `use`d
by export files enter through linking, but do not generate new cross-package starting points.

### Boundary with Existing Plans

| Neighbor                                          | Boundary                                                                                                                                                                                                                                                                 |
| ------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| RFC-029d "CLI `--entry` Entry Override" (planned) | 029f defines the role model; 029d consumes it — `--entry` is the CLI-level override means for the Bin role                                                                                                                                                               |
| RFC-014b `[binaries]`                             | **Name disambiguation**: `[binaries]` is a pre-compiled distribution artifact (download-priority strategy), `[[bin]]` is a source-role declaration. The former exists in dependency package manifests, the latter in the current package's manifest, not interchangeable |
| RFC-014c Workspaces                               | Member packages' mutual-reference import surface = the resolution order of this RFC; workspace coordination mechanisms (members, shared lockfile) still belong to 014c                                                                                                   |
| RFC-036 Testing                                   | Test role determination directly adopts 036's existing file rules, no new conventions added                                                                                                                                                                              |
| #321 Dead code                                    | Decision B (pub exempt unconditionally) is explicitly downgraded to "transitional semantics when no target"; Bin role pub being reportable is the form in which this RFC fulfills Option A                                                                               |

## Detailed Design

### Role Determination Algorithm (inside Orchestrator)

```
fn classify(files, manifest, entry_reach) -> Map<File, Role>:
    # 1. Explicit layer: manifest declarations take precedence
    for f in manifest.exports.values():  role[f] = Lib
    if manifest.lib_path:                role[lib_path] = Lib
    for b in manifest.binaries:          role[b.path] = Bin
    if manifest.run_main:                role[run_main] = Bin

    # 2. Inference layer: where undeclared, use entry reachability
    for f in files where role[f] undefined:
        if f is use'd by ≥1 non-self file:      role[f] = Lib
        elif f contains main and no file use's it: role[f] = Bin
        else:                                     role[f] = Internal

    # 3. Test layer: 036 rules hit → Test (overrides Lib/Internal, does not override explicit Bin)
    apply_rfc036_test_rules(files, &role)

    # Single-file direct run: bypass entire model, behavior unchanged
    if no manifest and single_file:      all Script
```

### Compiler Changes

- `frontend/config.rs`: manifest parsing supplements the `[exports]` → role table mapping (field
  parsing already in 015).
- `frontend/module/orchestrator.rs`: insert a role classification phase before Registry
  construction; cross-package resolution validates `use` targets according to import surface order,
  out-of-bounds reports the existing `module_not_found` family (no new error codes added, messages
  supplemented with "not in export surface" hints).
- `typecheck/passes/dead_code.rs`: the entry point set changes from "main + all pub" to "main + Lib
  role file pub" (Bin immediately reportable; Internal exempt until phase 2, see implementation
  strategy for Phase 2).
- LSP (RFC-017): completion/hover "importable items" are filtered by import surface; `main` missing
  diagnostic is reported only for Bin role files.

### Runtime Behavior

No changes. The role model is purely a compile-time/resolution-time semantic, not affecting IR and
execution.

### Backward Compatibility

- No manifest project: behavior completely unchanged (the inference layer restores current state —
  files use'd are Lib, files containing main are Bin). The increment in dead-code warnings is only
  in Bin files: unreferenced pub in the root file with no importer goes from silent to W1001 — this
  is the expected behavior of Option A from #321, not a break.
- With manifest project: when `[exports]` is declared, the import surface narrows from "all pub
  files" to the declared surface. **This is the only behavioral narrowing point**: code that depends
  on others' packages' internal files will start reporting `module_not_found`. Given that the
  package ecosystem has not yet been established (029: "currently no third-party package
  ecosystem"), the breakage surface is zero; still set a minor-version transition period
  (out-of-bounds warns first, then errors).

## Trade-offs

### Pros

- Zero new configuration surface: all fields are defined by 015; this RFC only supplements
  semantics.
- Zero new keywords: the import surface is carried by the distribution boundary, isomorphic to 029's
  "visibility does not exist" decision.
- Four consumers (#321 / 014c / 029a / 037) modeled at once, no more each hitting their own wall.
- Zero cost for the no-manifest path: the zero-config gene of the script language is fully
  preserved.

### Cons

- Role inference (use'd = Lib) in the common shape of "entry file use's utility file" will classify
  the utility file as Lib, with its pub exempt — coarser than the ideal granularity. This is a
  deliberate choice toward under-reporting; tightening requires call-direction analysis on the use
  graph, not done in v1.
- Narrowing the import surface via `[exports]` is a semantically tight operation with breaking
  potential (though with a transition period).

## Alternatives

| Alternative                              | Description                                                                   | Reason Not Adopted                                                                                                                                                                                                                  |
| ---------------------------------------- | ----------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Top-level RFC-040                        | Open a new top-level number                                                   | Module graph semantics (import surface / reachability / entry) all lie in 029 territory; splitting into a top-level would create two top-level documents for the same object; and the 029b–029e slot pattern is already established |
| Attach to 014c                           | As a section of workspace                                                     | Dependency direction is reversed: language layer (029x) defines semantics, package layer (014x) consumes semantics. 014c is still under review; mid-way scope expansion delays its landing                                          |
| Option A as-is (#321 menu)               | Manifest requires `[[bin]]`/`[[lib]]` target before dead-code semantics apply | Introduces mandatory configuration for a narrow slice (unused pub in root files with main); this RFC's inference layer achieves the same benefit with zero configuration                                                            |
| Pure entry inference (no manifest layer) | Does not consume 015 fields                                                   | `[exports]` is an already-accepted field in 015; cross-package import surface cannot bypass it; not defining it equals leaving a gap for 014c                                                                                       |

## Implementation Strategy

### Phased

- **Phase 1**: Role classification + import surface resolution + Bin pub reportable (Internal/Test
  remains exempt)
- **Phase 2**: Internal pub tightened to "reportable if not reachable by in-package use graph" —
  prerequisite is Bin role warning corpus validation without false-positive feedback after Phase 1
  lands; triggered when ready, no schedule set

### Dependencies

- Prerequisites: RFC-029 orchestrator (landed), RFC-015 manifest fields (defined).
- Depended on by: #321 (Bin pub reportable), RFC-014c (import surface), RFC-029a (role as cache-unit
  boundary), RFC-037 (artifact selection), RFC-029d (`--entry` override).
- Landing order with RFC-014c: before 014c review passes, 029f's import surface order should be
  finalized first; 014c consumes by reference, avoiding mid-review scope expansion.

### Risks

- Divergence between role inference and user intuition (the first item in trade-offs above) →
  present all incremental behavior in warning (non-blocking) form, disable-able at any time.
- Transition-period management for `[exports]` narrowing → warn first within the minor version.

## Open Questions

All three items from the draft stage have been decided (2026-09-13, decisions entered into Appendix
B and main body), no pending items:

- [x] Whether Internal pub is tightened → **Decision**: tighten in phase 2 by in-package use-graph
      reachability, with Bin role corpus validation as prerequisite (see Implementation Strategy
      Phase 2)
- [x] `[[test]]` explicit declaration → **Decision**: not introduced, Test role determined only by
      036 rules; if 036 has explicit declaration needs in the future, extended by itself
- [x] Workspace root overriding member export surfaces → **Decision**: not allowed, member
      self-containment is the core design of 014c, root override would break encapsulation

---

## Appendix B: Design Decision Records

> **Errata (2026-09-17): Role Model Drives Entry Semantics**
>
> This RFC defines the five roles and what "who is the entry" means ("`main` is the reachability
> root" in the Bin row), but the initial implementation only used roles for **dead-code warnings**,
> and entry lookup was independent of roles — `find_entry_point` would return 0 if it couldn't find
> `main`, **silently executing the first function in the function table**. The top-level binding
> refactor (T4) wired the role model into entry determination, with the specific rules as follows:
>
> | Role                      | Entry rule                                                                                                                                                                                     |
> | ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | **Script** (no manifest)  | **No entry concept**. Top-level statements (including binding initialization) in written order are the program body; `main` is a regular binding, **not auto-called** — to run, write `main()` |
> | **Bin** (with manifest)   | Requires `main` and it must be a function; otherwise compile error (E3020 missing main / E3021 main is not a function). No executable top-level statements allowed                             |
> | **Lib / Internal / Test** | Do not require `main` (they are not entries)                                                                                                                                                   |
>
> **Two differences from the initial description in this RFC**:
>
> 1. The initial Script row stated "behavior identical to today". Today's behavior contains two
>    silent defects, both fixed in T4: ① when no `main`, execute the first function in the function
>    table; ② top-level statements report E3005 as not executable. The current Script allows
>    top-level statements to be **executable**, which is an added capability rather than maintaining
>    status quo.
> 2. "Under Script, `main` is not special" and "Under Bin, `main` is the entry" must be **mutually
>    exclusive**: if Script both executes top-level statements and implicitly calls `main`, scripts
>    with explicit `main()` would run twice (#356). Therefore Script has only one execution entry —
>    top-level statements.
>
> **Distinction between role determination and entry determination** (implementation note): entry
> determination uses "is there a manifest" (`find_project_root().is_some()`) rather than
> `roles::classify()`. Reason: classify answers "who consumes this file" (serving dead-code
> analysis); a file with a manifest but no `main` is Internal in classify (it's not use'd by other
> files, reasonable), but if the user runs it as an entry, there must be an entry. The two questions
> are different, so the criteria differ.
>
> See `docs/src/reference/language-spec/syntax.md` §3.11.

| Decision                                           | Resolution                                                                                         | Date       | Recorder | Basis                                                                                                                         |
| -------------------------------------------------- | -------------------------------------------------------------------------------------------------- | ---------- | -------- | ----------------------------------------------------------------------------------------------------------------------------- |
| Number                                             | 029f (029b reserved for hot reload, 029c deleted, 029d/029e placed)                                | 2026-09-12 | Chenxu   | RFC-029 §Sub-RFC Planning                                                                                                     |
| Belongs to 029 family rather than top-level RFC    | Yes                                                                                                | 2026-09-12 | Chenxu   | Import surface / reachability / entry all belong to module graph semantics; 029 has already consumed `[[bin]]` entry priority |
| Visibility keyword                                 | Not introduced                                                                                     | 2026-09-12 | Chenxu   | Isomorphic to RFC-029 "visibility does not exist" decision, import surface carried by distribution boundary                   |
| Status of #321 Decision B                          | Downgraded to "transitional semantics when no target"                                              | 2026-09-12 | Chenxu   | Bin role pub being reportable is the form in which #321 Option A is fulfilled                                                 |
| Role granularity                                   | File level                                                                                         | 2026-09-12 | Chenxu   | 015 field granularity (`[lib]` single file, `[[bin]]` list, `[exports]` mapping)                                              |
| `[binaries]` disambiguation                        | 014b pre-compiled distribution artifact ≠ `[[bin]]` source role                                    | 2026-09-12 | Chenxu   | Two-layer concept (download strategy vs compilation role), nailed down before 014c lands                                      |
| Internal pub tightening                            | Tighten in phase 2 by in-package use-graph reachability, prerequisite = Bin role corpus validation | 2026-09-13 | Chenxu   | Gradual tightening in the under-reporting direction, corpus-driven to avoid premature false positives                         |
| `[[test]]` explicit declaration                    | Not introduced; Test determined only by 036 rules                                                  | 2026-09-13 | Chenxu   | No configuration surface added before 036 has the need                                                                        |
| Workspace root overriding member export surfaces   | Not allowed                                                                                        | 2026-09-13 | Chenxu   | Member self-containment is the core design of 014c, root override would break encapsulation                                   |
| Whether `main` is auto-called under Script         | **No auto-call** — top-level statements are the program, `main()` must be explicit                 | 2026-09-17 | Chenxu   | Having both rules would cause explicit `main()` to run twice (#356); Script can have only one execution entry                 |
| Whether role model is used for entry determination | **Yes** (previously only for dead-code warnings)                                                   | 2026-09-17 | Chenxu   | This RFC defines Bin's "`main` is the reachability root"; without wiring into entry, that semantic is dormant                 |

## Appendix C: Glossary

| Term                  | Definition                                                                                                               |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| Role                  | A source file's compilation target identity: Script / Bin / Lib / Test / Internal                                        |
| Import Surface        | The file set of valid resolution starting points for cross-package `use`, declared by `[exports]`/`[lib]` or inferred    |
| Distribution Boundary | 029 term: the package's external exposure range; this RFC refines it from implicit (all pub files) to the import surface |
| Script state          | The bypass form of single-file direct run without a manifest, all behavior consistent with current state                 |

## References

- RFC-029 Module Semantics (parent RFC: entry selection, visibility decision, sub-RFC planning)
- RFC-015 Configuration System (`[lib]`/`[[bin]]`/`[exports]` field definitions)
- RFC-014b Build System and Binary Distribution (`[binaries]`, disambiguation target)
- RFC-014c Workspace Support (first consumer of the import surface, under review)
- RFC-036 Testing Framework (source of Test role rules)
- #321 M2 Warning Code Independent Emission Channel (origin of Decision B and Option A)
