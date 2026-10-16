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

> **Landing correction (2026-10-08, RFC-029g)**: The `pub` keyword has been removed in its entirety
> by RFC-029g. Throughout this document, every mention of "the `pub` exemption / `pub` files /
> unused-pub warnings" should be read as "**top-level binding + intra-package reference pool**": the
> role classification (Script / Bin / Lib / Internal / Test) and import surface semantics are
> unchanged — the only change is in the condition that determines the dead-code exemption (see
> [029g](029g-remove-pub-and-auto-bind.md) and the criteria in
> `docs/src/reference/warning-code/warning-codes.md`).

Fulfills the extension slot promised in RFC-029's "Sub-RFC Plan" (slots 029b–029e are taken, so 029f
follows): define a **compilation target role model** (five classes: Script / Bin / Lib / Test /
Internal) for source files, together with its **import surface semantics** — inferred by entry
reachability when no manifest exists (zero configuration), or declared explicitly via `[lib]` /
`[[bin]]` / `[exports]` (fields already defined by RFC-015) when a manifest is present. This fills
four dangling semantic gaps: how the cross-package import surface is computed; the applicable scope
of the `pub` exemption for the dead-code warning (Decision B of #321); the relationship between
`[exports]` and Registry reachability; and the disambiguation of the term "bin" between the
`[binaries]` field in RFC-014b (pre-compiled distribution artifacts) and the bin-role source files.

## Motivation

### Host and boundary basis

The entry selection and visibility decisions in RFC-029 (accepted) are the direct upstream of this
RFC:

> **Entry-file selection** priority: 1. `[run].main` 2. `[[bin]]` first entry's `path` 3.
> `src/main.yx` (conventional default). — RFC-029 § Package-Level Cycles: Revisited

> **Visibility**: does not exist. Scope + distribution boundary cover every scenario; no new keyword
> is needed. — RFC-029 § Design Decision Record (2026-07-30)

The second point is the design red line of this RFC: the import surface semantics **must not
introduce any new visibility keyword**; it can only be carried by the "distribution boundary" (file
role + declared export surface).

Four consumers each hit the same wall:

| Consumer                                           | Where it hits the wall                                                                                                                                                                                                                                 |
| -------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| #321 Dead-code warning                             | Decision B's "pub = external interface, never reported" exists because there is no role model to distinguish "truly external" from "looks external"; whether to report an unused pub inside a bin (Option A) is still pending                          |
| RFC-014c Workspace (under review, #113)            | When member packages `use` one another, what defines the import surface — `[exports]`, the `[lib]` single file, or all pub files — **there is not a single semantic clause on the subject**                                                            |
| RFC-015 Configuration system (accepted)            | The `[lib]` / `[[bin]]` / `[exports]` fields are defined; except for RFC-029's consumption of the entry priority, no one has aligned the semantics of the remaining fields (especially the relationship between `[exports]` and Registry reachability) |
| RFC-037 Packaging / RFC-029a Caching (draft, #293) | Choice of packaging artifacts and the cache-unit boundary for incremental recompilation both need the role model as a prerequisite                                                                                                                     |

### Current problems

**Four semantic gaps** (audit of 2026-09-12; confirmed by re-reading 014/014a/014b/014c/015/029 in
full):

1. **No definition of the cross-package import surface**. The 014c example of member packages
   importing each other only shows a directory structure (`src/lib.yx`); there is no clause stating
   what `use utils.helper` can resolve to. The current implicit rule is "all top-level pub files of
   the package" — internal implementation files are dragged into the namespace too, and the
   dependency boundary becomes a fiction.
2. **The `pub` exemption has no applicable scope**. Decision B of #321 — "pub is never reported" —
   is overly conservative for a single-file script (a script has no external consumers, so pub is
   meaningless), and overly loose for the entry file of a multi-file project (an unused pub on a
   root file with no importers is dead code). Without a role model, no line can be drawn between
   Options A and B.
3. **Three export concepts have not been aligned**. `[exports]` (RFC-015, path mapping), `[lib]`
   (RFC-015, single file path), and Registry reachability (RFC-029, reachable from entry along
   `use`) — is this a containment relationship, an equality relationship, or each does its own
   thing? No clause on the subject.
4. **Term collision**. RFC-014b's `[binaries]` is a **pre-compiled distribution artifact** (an
   .so/exe download-preference policy), and is on a completely different layer from the "bin target
   (source-code role)". Once 014c lands, the two will inevitably appear on screen together, and the
   confusion will persist unless disambiguated early.

**Evidence: Decision B's conservatism is already a real pain point**. The #321 M2 retrospective
records that the structural reason W1001–W1005 are all silent by default is precisely "every pub is
treated as an external interface" — this is compensatory behavior caused by a missing model, not a
terminal state.

## Proposal

### Core design: the file role model

**A role is a file-level attribute** (matching the granularity of RFC-015's fields: `[lib]` is a
single file, `[[bin]]` is a list of files, `[exports]` is a file map) — not a package-level switch.

| Role         | Determination                                                                                                                                                          | Semantics                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Bin**      | The file pointed to by `[run].main` or `[[bin]].path`; or (without a manifest) a file containing `main` that is not `use`d by any other file                           | Program entry. Requires a binding named `main`, which may be a value or a function: a function `main` is invoked at entry with zero arguments; a value `main` is evaluated at initialization time and that single evaluation is the execution (even if the evaluated result is a function value, it is not invoked again). Missing `main` is a compile error (E3020); top-level executable statements are forbidden. Its `pub` **may be reported as dead code** (no consumers outside the package); `main` is a reachability root |
| **Lib**      | Files hit by an `[exports]` mapping; or `[lib].path`; or (without a manifest) files `use`d by some other file                                                          | Distribution boundary. Its `pub` **is exempt from dead-code reporting** (consumers outside the package are invisible, so we prefer false negatives); symbols exported through it are visible across packages                                                                                                                                                                                                                                                                                                                      |
| **Test**     | Files matched by the RFC-036 test file rules (the `tests/` directory, `*_test.yx`, and other existing conventions); **no** explicit `[[test]]` declaration is provided | Test code. Does not participate in dead-code analysis; its references count as reachability roots for the code under test                                                                                                                                                                                                                                                                                                                                                                                                         |
| **Internal** | In a package with a manifest, files that are neither on the export surface nor contain `main`                                                                          | Intra-package implementation. `pub` is exempt pending Phase 2 (to be tightened to the intra-package `use`-graph reachability, contingent on the Bin role corpus as a prerequisite); `use` from outside the package **cannot reach it**                                                                                                                                                                                                                                                                                            |
| **Script**   | A file run directly as a single file (`run foo.yx`)                                                                                                                    | **No entry concept**. Top-level statements (including binding initializers) are the program body in source order; `main` is a normal binding, **not called automatically** — to run it, write `main()`. The Registry contains only std                                                                                                                                                                                                                                                                                            |

**Determination priority**: explicit manifest declaration > entry-reachability inference > RFC-036
test conventions. Explicit declarations always win — inference is only the default in the absence of
a declaration.

### Entry determination

The role model is used not only for dead-code analysis but also **drives entry determination**.
Entry rules per role:

| Role                      | Entry rule                                                                                                                                                                                                                                              |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Script** (no manifest)  | **No entry concept**. Top-level statements (including binding initializers) are the program body in source order; `main` is a normal binding, **not called automatically** — to run it, write `main()`                                                  |
| **Bin** (with manifest)   | Requires a binding named `main` (value or function: a value `main` is evaluated at initialization; a function `main` is invoked at entry with zero arguments); missing `main` is a compile error (E3020). Top-level executable statements are forbidden |
| **Lib / Internal / Test** | Do not require `main` (they are not entries)                                                                                                                                                                                                            |

**Script and Bin entries must be mutually exclusive**: "Script treats `main` as ordinary" and "Bin
treats `main` as the entry" cannot both hold — if a Script both executes its top-level statements
and implicitly calls `main`, a script that explicitly writes `main()` would be double-run (#356).
Therefore a Script has only one execution entry: its top-level statements.

**The distinction between role determination and entry determination** (implementation note): entry
determination uses "presence of a manifest" (`find_project_root().is_some()`), not
`roles::classify()`. The reason: `classify` answers "who consumes this file" (in service of
dead-code analysis); a file with a manifest but no `main` is classified as Internal by `classify`
(it isn't `use`d by any other file, which is reasonable), yet the user runs it as the entry — at
which point an entry must exist. Two different questions, hence two different criteria.

**The execution form of a value `main`** (Decision #388): functions are already values (unified at
the type layer), so the entry is determined by **the existence of the binding**, not its callability
— a value `main` and a function `main` differ only in evaluation strategy. A value `main` does not
receive an entry-time call (prevention of double-run per #356: initialization and entry call never
stack), and shares the execution form with Script; its evaluation is part of the initialization
sequence: executed in the global binding topological order, with independent bindings following
source order. Therefore migrating a Script by wrapping its top-level statements into
`main = { ... }` and moving it into a Bin **is not an order-preserving migration** — scattered
statements execute in source order, while binding initializers execute in topological order;
initializations that need to come first should make `main` depend on them explicitly. A value `main`
like `main: Int = 5` (a non-callable value `main`) is a legal but observationally empty program
(analogous to Rust's empty `fn main() {}`).

See `docs/src/reference/language-spec/syntax.md` §3.11 for details.

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
"./internal-helper" = "src/helper.yx"   # ← it is this field, not pub, that determines cross-package visibility
```

```yaoxiang
# src/cli.yx (Bin role)
pub unused_fn = (x: Int) => x    # ← W1001 may report: bin has no consumers outside the package
main: () -> Void = { ... }

# src/lib.yx (Lib role, on the export surface)
pub api_fn = ...                 # ← never reports: consumers outside the package are invisible

# src/other.yx (Internal role, not on the export surface)
pub semi_api = ...               # ← exempt pending Phase 2 (prefer false negatives); to be tightened by the use graph when that lands
```

### Import surface semantics

The resolution order for cross-package `use pkg.x` (the first hit takes effect):

1. An `[exports]` mapping exists → the import surface = the set of files listed in the mapping, and
   `x` must be a top-level binding in one of those files;
2. Otherwise `[lib].path` exists → the import surface = that single file;
3. Neither exists (a manifest-less dependency, or a path pointing at a bare directory) → **status
   quo**: all top-level files are importable (compatible with existing behavior; tightening to be
   discussed separately).

**Workspace members**: the export surface is defined solely by each member package's own manifest;
the workspace root must not override or extend a member's export surface — member self-containment
is 014c's core design, and root overrides would break encapsulation.

The relationship with Registry reachability (aligned with RFC-029): `[exports]`/`[lib]` define the
**set of legal starting points for cross-package resolution**; once the starting points are
determined, the Registry still expands per RFC-029's "reachable from entry along `use`" — internal
files brought in by the export file's own `use` enter via linking, but do not produce new
cross-package starting points.

### Demarcation from existing plans

| Neighbor                                          | Boundary                                                                                                                                                                                                                                                                  |
| ------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| RFC-029d "CLI `--entry` entry override" (planned) | 029f defines the role model; 029d consumes it — `--entry` is a CLI-level override for the Bin role                                                                                                                                                                        |
| RFC-014b `[binaries]`                             | **Term disambiguation**: `[binaries]` is a pre-compiled distribution artifact (download-preference policy), `[[bin]]` is a source-code role declaration. The former lives in a dependency's manifest, the latter in this package's manifest; they are not interchangeable |
| RFC-014c Workspace                                | The import surface for member packages' mutual imports is the resolution order defined by this RFC; workspace coordination mechanisms (members, shared lockfile) still belong to 014c                                                                                     |
| RFC-036 Testing                                   | The Test role is determined by RFC-036's existing file rules; no new convention is added                                                                                                                                                                                  |
| #321 Dead code                                    | Decision B ("pub always exempt") is explicitly downgraded to "transitional semantics when no target exists"; reporting a Bin role's `pub` is the realization of Option A promised by this RFC                                                                             |

## Detailed design

### Role determination algorithm (inside the orchestrator)

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

    # 3. Test layer: 036 rules hit → Test (overrides Lib/Internal, does not override explicit Bin)
    apply_rfc036_test_rules(files, &role)

    # Single-file direct run: entire model bypassed, behavior unchanged
    if no manifest and single_file:      all Script
```

### Compiler changes

- `frontend/config.rs`: manifest parsing supplements the `[exports]` → role-table mapping (field
  parsing already exists in RFC-015).
- `frontend/module/orchestrator.rs`: insert a role-classification phase before building the
  Registry; cross-package resolution validates `use` targets against the import-surface order, with
  out-of-surface accesses reported under the existing `module_not_found` family (no new error code;
  the message adds the "not on the export surface" hint).
- `typecheck/passes/dead_code.rs`: the entry-point set changes from "main + all pub" to "main + pub
  of Lib-role files" (Bins can be reported immediately; Internals remain exempt pending Phase 2 —
  see Implementation Strategy).
- LSP (RFC-017): completion and hover's "importable items" are filtered by the import surface; the
  "missing `main`" diagnostic is reported only for Bin-role files.

### Runtime behavior

No change. The role model is purely a compile-time / parse-time semantic; it does not affect the IR
or execution.

### Backward compatibility

- Projects without a manifest: behavior is completely unchanged (the inference layer reproduces the
  current state — a `use`d file is Lib, a file containing `main` is Bin). The dead-code-warning
  increment is confined to Bin files: an unused pub on a root file with no importers goes from
  silent to W1001 — this is the expected behavior of Option A from #321, not a break.
- Projects with a manifest: where `[exports]` is already declared, the import surface narrows from
  "all pub files" to the declared surface. **This is the only behavior-narrowing point**: code that
  depends on another package's internal files will start reporting `module_not_found`. Given that
  the package ecosystem has not yet been built (RFC-029: "no third-party package ecosystem
  currently"), the break surface is zero; a minor-version transition period is still provided
  (out-of-surface accesses warn first, then become errors).

## Trade-offs

### Advantages

- Zero new configuration surface: all fields are already defined by RFC-015; this RFC only adds
  semantics.
- Zero new keyword: the import surface is carried by the distribution boundary, isomorphic to
  RFC-029's "no visibility" decision.
- Four consumers (#321 / 014c / 029a / 037) are modeled once, each ceasing to hit its own wall.
- The manifest-less path has zero cost: the scripting language's zero-configuration DNA is fully
  preserved.

### Disadvantages

- Role inference ("being `use`d ⇒ Lib") in the common shape "the entry file `use`s a utility file"
  will classify the utility file as Lib, exempting its `pub` — coarser than the ideal granularity.
  This is a deliberate choice in the prefer-false-negative direction; tightening requires
  call-direction analysis on the use graph and is not done in the first version.
- The narrowing of the import surface by `[exports]` is a breaking semantic tightening (despite the
  transition period).

## Alternatives

| Alternative                              | Description                                                                         | Reason not adopted                                                                                                                                                                                                                          |
| ---------------------------------------- | ----------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A top-level RFC-040                      | Open a new top-level number                                                         | Module-graph semantics (import surface / reachability / entry) all live in the RFC-029 domain; splitting at the top level would produce two top-level documents for the same object; the 029b–029e slot pattern is also already established |
| Attach to 014c                           | Make it a section of the workspace RFC                                              | The dependency direction is reversed: the language layer (029x) defines semantics, and the package layer (014x) consumes them. 014c is still under review; expanding its scope mid-review would slow its landing                            |
| Option A as-is (the #321 menu)           | Force a `[[bin]]`/`[[lib]]` target in the manifest before dead-code semantics apply | Introduces a mandatory configuration for a narrow slice (unused pub on a root file with `main`); this RFC's inference layer achieves the same benefit with zero configuration                                                               |
| Pure entry inference (no manifest layer) | Do not consume RFC-015 fields                                                       | `[exports]` is an already-accepted RFC-015 field, and the cross-package import surface cannot avoid it; not defining it leaves a gap for 014c                                                                                               |

## Implementation strategy

### Phases

- **Phase 1**: role classification + import-surface resolution + Bin pub reportable (Internal/Test
  remain exempt)
- **Phase 2**: Internal pub tightened to "reportable if unreachable in the intra-package use graph"
  — gated on Phase 1's Bin role corpus verifying zero false-positive regressions; triggered when
  ready, no schedule

### Dependencies

- Prerequisites: RFC-029 orchestrator (landed), RFC-015 manifest fields (defined).
- Depended on by: #321 (Bin pub reportable), RFC-014c (import surface), RFC-029a (roles as
  cache-unit boundaries), RFC-037 (artifact selection), RFC-029d (`--entry` override).
- Landing order with RFC-014c: before 014c is approved, 029f's import-surface order should be
  finalized, and 014c should consume it by reference — to avoid expanding the scope of an in-flight
  review.

### Risks

- Divergence between inferred roles and user intuition (the first item in Trade-offs) → all
  incremental behavior surfaces as warnings (non-blocking) and can be disabled at any time.
- Managing the transition period for `[exports]` narrowing → warn first within one minor version.

## Open questions

All three draft-stage questions have been resolved (2026-09-13; decisions recorded in Appendix B and
the body); no outstanding items:

- [x] Whether to tighten Internal `pub` → **Decision**: tighten in Phase 2 to intra-package
      use-graph reachability, with Bin role corpus verification as the prerequisite (see
      Implementation Strategy Phase 2)
- [x] `[[test]]` explicit declaration → **Decision**: not introduced; the Test role is determined
      solely by RFC-036 rules; if RFC-036 ever needs explicit declarations, it will extend on its
      own
- [x] Whether the workspace root can override a member's export surface → **Decision**: not allowed;
      member self-containment is 014c's core design, and root overrides would break encapsulation

---

## Appendix B: Design Decision Record

| Decision                                               | Resolution                                                                                            | Date       | Recorder | Basis                                                                                                                                 |
| ------------------------------------------------------ | ----------------------------------------------------------------------------------------------------- | ---------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| Number                                                 | 029f (029b is reserved for hot reload, 029c was deleted, 029d/029e are taken)                         | 2026-09-12 | Chenxu   | RFC-029 § Sub-RFC Plan                                                                                                                |
| Belongs to the 029 family, not a top-level RFC         | Yes                                                                                                   | 2026-09-12 | Chenxu   | Import surface / reachability / entry all belong to module-graph semantics; RFC-029 has already consumed the `[[bin]]` entry priority |
| Visibility keyword                                     | Not introduced                                                                                        | 2026-09-12 | Chenxu   | Isomorphic with RFC-029's "no visibility" decision; the import surface is carried by the distribution boundary                        |
| Status of #321 Decision B                              | Downgraded to "transitional semantics when no target exists"                                          | 2026-09-12 | Chenxu   | Bin role's pub being reportable is the realization of #321 Option A                                                                   |
| Role granularity                                       | File-level                                                                                            | 2026-09-12 | Chenxu   | Matches the granularity of RFC-015 fields (`[lib]` single file, `[[bin]]` list, `[exports]` map)                                      |
| `[binaries]` disambiguation                            | RFC-014b's pre-compiled distribution artifacts ≠ `[[bin]]` source-code role                           | 2026-09-12 | Chenxu   | Two-layer concepts (download policy vs. compilation role); pin down before 014c lands                                                 |
| Internal pub tightening                                | Phase 2: tighten to intra-package use-graph reachability, prerequisite = Bin role corpus verification | 2026-09-13 | Chenxu   | Prefer-false-negative gradual tightening, corpus-driven to avoid premature false positives                                            |
| `[[test]]` explicit declaration                        | Not introduced; Test is determined solely by RFC-036 rules                                            | 2026-09-13 | Chenxu   | No new configuration surface until RFC-036 needs it                                                                                   |
| Workspace root overriding a member's export surface    | Not allowed                                                                                           | 2026-09-13 | Chenxu   | Member self-containment is 014c's core design; root overrides break encapsulation                                                     |
| Whether Script's `main` is auto-called                 | **Not auto-called** — top-level statements are the program, `main()` must be explicit                 | 2026-09-17 | Chenxu   | Having both rules in play would double-run an explicit `main()` (#356); a Script can only have one execution entry                    |
| Whether the role model is used for entry determination | **Yes** (previously only used for the dead-code warning)                                              | 2026-09-17 | Chenxu   | This RFC defines Bin's "main is a reachability root"; without entry, that semantic sits idle                                          |

## Appendix C: Glossary

| Term                  | Definition                                                                                                                           |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| Role                  | A source file's compilation target identity: Script / Bin / Lib / Test / Internal                                                    |
| Import Surface        | The set of files that serve as legal resolution starting points for cross-package `use`, declared by `[exports]`/`[lib]` or inferred |
| Distribution boundary | RFC-029 term: the externally exposed range of a package; this RFC refines it from the implicit "all pub files" to the import surface |
| Script state          | The bypass state of a manifest-less single-file direct run; all behavior matches the status quo                                      |

## References

- RFC-029 Module semantics (parent RFC: entry selection, visibility decision, sub-RFC plan)
- RFC-015 Configuration system (definitions of the `[lib]`/`[[bin]]`/`[exports]` fields)
- RFC-014b Build system and binary distribution (`[binaries]`, the disambiguation target)
- RFC-014c Workspace support (first consumer of the import surface, under review)
- RFC-036 Testing framework (source of the Test role rules)
- #321 M2 independent emission channel for warning codes (origin of Decision B and Option A)

## Known defects: landed (2026-10-16, WBS 4.9 round 1)

> The previous version (2026-10-09) listed the following three items as "three design flaws of the
> reference pool". All three have been landed by WBS 4.9 round 1 (commit `7e62b0454`);
> `collect_project_refs` and `orchestrator.rs` have been split out of P4 4.2.9 and moved into
> `src/driver/{state,arms,helpers}.rs`; all `file:line` references in the table below have been
> re-fetched on the referenced revision.

### Three design flaws of the reference pool — fix status

| #   | Original flaw                                                     | Current state (2026-10-16)                                                                                                                                                                                                                                                                                                                                                                                                             | Landed code (rev `7e62b0454`)                                                                                                                   |
| --- | ----------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | The scan surface is a directory, not the compilation surface      | **Pool root = package targets**: `collect_package_refs` (`helpers.rs:481`) calls `discover_with_used` per the `[[bin]]`/`[lib]`/`[exports]`/`[run].main` roots from `roles::ExplicitSurfaces` to take the compilation set; it **does not scan directories to find entries**. When none of the roots can be resolved, it falls back to the compilation set of the inspected file itself (`collect_compilation_refs` / `helpers.rs:454`) | `src/driver/helpers.rs:454-512` (fallback path); `src/driver/helpers.rs:481-512` (manifest-declared surface path)                               |
| 2   | Granularity is project-level bare names, with no module dimension | **Pool type = `HashMap<String, HashSet<String>>`**: key = module key, value = the set of reference names for that module; on the consumer side, `arms.rs:603-609` unions all "non-self-unit" module entries and hands them to `DeadCodeAnalyzer`; module self names (top-level definitions and local bindings, with the exception of `use`-imported names) no longer exempt definitions of the same name in other modules              | `src/driver/arms.rs:603-609` (consumer-side `other_module_refs`); single assembly entry at `src/driver/helpers.rs:429-449` (`collect_ref_pool`) |
| 3   | Script reuses Lib's pool (violates single-file semantics)         | **Script does not consume the pool**: the `match us.role` at `arms.rs:611-623` only assembles the pool for `Bin`/`Internal` (`set_exempt_pub`) and `Lib` (`set_cross_file_refs`); Script/Test go through the `_ => {}` default arm, which only reasons about the within-file definition-use graph                                                                                                                                      | `src/driver/arms.rs:611-623`                                                                                                                    |

**Items left for subsequent rounds of WBS 4.9 after landing** (out of scope for this round):

- **Consumption-definition refinement (4.9.4)**: distinguish "imported" from "actually used after
  import" — not in this round, depends on path resolution of `use` items (prerequisite = 4.6.1,
  delivered by `39546020`).
- **Test pool separation (4.9.5)**: references in tests/ form their own pool, and do not exempt
  unused definitions in production code (src/).

**Reason not changed in P3.5 (unchanged from the previous version)**: module-level resolution
requires the path resolution of `use` items to be complete (`use a::foo` → foo comes from module a),
which belongs to the P4-stage contract and the Driver unification scope.
