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

> **Implementation correction (2026-10-08, RFC-029g)**: The `pub` keyword was removed in its
> entirety by RFC-029g. All statements in the body of this document concerning a "`pub` exemption /
> `pub` file / unused pub warning" should be read as referring to "**top-level bindings + the
> in-package reference pool**": the role classification (Script / Bin / Lib / Internal / Test) and
> import-surface semantics remain unchanged; only the criterion for dead-code exemptions changes
> (see [029g](029g-remove-pub-and-auto-bind.md) and the criteria in
> `docs/src/reference/warning-code/warning-codes.md`).

This RFC fulfills the extension slot in RFC-029's “Sub-RFC Plan” (029b–029e are already occupied, so
029f is deferred to this slot). It defines a **compilation target role model** for source files
(five categories: Script / Bin / Lib / Test / Internal) and their **import-surface semantics**: with
no manifest, roles are inferred from entry-point reachability (zero configuration); with a manifest,
they are explicitly declared through `[lib]` / `[[bin]]` / `[exports]` (fields already defined by
RFC-015). It fills four semantic gaps: how the cross-package import surface is calculated, the scope
in which the pub exemption for dead-code warnings applies (decision B of #321), the relationship
between `[exports]` and Registry reachability, and the terminology disambiguation between RFC-014b's
`[binaries]` (precompiled distribution artifacts) and source files with the bin role.

## Motivation

### Host and Boundary Basis

RFC-029 (accepted) is the direct upstream RFC for this document's entry-point selection and
visibility decisions:

> **Entry-file selection** priority: 1. `[run].main` 2. The `path` of the first `[[bin]]` entry 3.
> `src/main.yx` (conventional default). — RFC-029 §Inter-package cycles: one more time

> **Visibility**: nonexistent. Scope + distribution boundaries cover all scenarios; no new keywords
> are needed. — RFC-029 §Design Decision Record (2026-07-30)

The second statement is this RFC's design red line: import-surface semantics **must not introduce
any new visibility keywords**. The boundary must be carried solely by the “distribution boundary”
(file role + declared export surface).

Four consumers have independently encountered the same wall:

| Consumer                                           | Point of blockage                                                                                                                                                                                                                                                    |
| -------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| #321 dead-code warnings                            | Decision B, “pub = an external interface is never reported,” exists because there was no role model to distinguish “genuinely external” from “merely appears external”; whether an unused pub in a bin should be reported (option A) has remained unresolved         |
| RFC-014c workspace (under review, #113)            | When member packages `use` one another, it is unclear how the import surface is calculated—`[exports]`, a single `[lib]` file, or all pub files—and **there is not a single semantic provision covering this**                                                       |
| RFC-015 configuration system (accepted)            | The `[lib]` / `[[bin]]` / `[exports]` fields have already been defined. Apart from RFC-029 consuming the entry-point priority, the semantics of the remaining fields—especially the relationship between `[exports]` and Registry reachability—have not been aligned |
| RFC-037 packaging / RFC-029a caching (draft, #293) | Both package-artifact selection and the cache-unit boundaries for incremental recompilation require the role model as a prerequisite                                                                                                                                 |

### Current Problems

**Four semantic gaps** (inventory completed on 2026-09-12 after reviewing the full text of
014/014a/014b/014c/015/029):

1. **The cross-package import surface is undefined.** The example of mutual imports between member
   packages in 014c contains only a directory structure (`src/lib.yx`), with no provision stating
   what `use utils.helper` can access. The current implicit rule is “all top-level pub files in the
   package”—internal implementation files are also pulled into the namespace, making the dependency
   boundary effectively meaningless.
2. **The dead-code pub exemption has no defined scope.** Decision B of #321, “pub is never
   reported,” is overly conservative in a single-file script (a script has no external consumers, so
   pub is meaningless) and overly permissive for the entry file of a multi-file project (a pub in
   the root file that no one imports is dead code). Without a role model, there is no way to draw
   the boundary between options A and B.
3. **Three export concepts have not been aligned.** `[exports]` (015, path mapping), `[lib]` (015,
   single-file path), and Registry reachability (029, reachable from the entry point along
   `use`)—there is no provision stating whether these are containment relationships, equality
   relationships, or independently managed concepts.
4. **Terminology collision.** RFC-014b's `[binaries]` denotes **precompiled distribution artifacts**
   (the .so/exe download-priority strategy), which is an entirely different layer from a “bin target
   (source role).” Once 014c is implemented, the two will inevitably appear together; failing to
   disambiguate them now will continue to cause misunderstanding.

**Empirical evidence: the conservatism of decision B is already causing practical pain.** The #321
M2 retrospective records that the structural reason the entire W1001–W1005 family is silent under
the default configuration is precisely that “all pub bindings are treated as external interfaces.”
This is compensating behavior caused by the absence of a model, not the final state.

## Proposal

### Core Design: File Role Model

A **role is a file-level attribute** (consistent with the field granularity of 015: `[lib]` is a
single file, `[[bin]]` is a list of files, and `[exports]` is a file mapping), not a package-level
switch.

| Role         | Classification                                                                                                                                                          | Semantics                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| ------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Bin**      | The file targeted by `[run].main` / `[[bin]].path`; or, when there is no manifest, a file that contains `main` and is not `use`d by any file                            | Program entry point. It must contain a **binding** named `main`, either a value or a function: a function `main` is called with zero arguments at entry, while a value `main` executes when evaluated during initialization (if the evaluated result is itself a function value, it is not called). A missing `main` is a compile-time error (E3020); executable statements are not permitted at the top level. Its `pub` bindings **may be reported as dead code** (there are no out-of-package consumers); `main` is a reachability root |
| **Lib**      | A file matched by the `[exports]` mapping; or `[lib].path`; or, when there is no manifest, a file referenced by another file's `use`                                    | Distribution boundary. Its `pub` bindings are **exempt from dead-code reporting** (out-of-package consumers cannot be seen, so under-reporting is preferred); symbols exported through it are visible across packages                                                                                                                                                                                                                                                                                                                      |
| **Test**     | A file matched by RFC-036's test-file rules (existing conventions such as the `tests/` directory and `*_test.yx`); **no** explicit `[[test]]` declaration is introduced | Test code. It does not participate in dead-code determination; its references count as reachability roots for the code under test                                                                                                                                                                                                                                                                                                                                                                                                          |
| **Internal** | When the package has a manifest, a file that is neither on the export surface nor contains `main`                                                                       | In-package implementation. `pub` bindings are exempt until phase two (when they are tightened according to reachability in the in-package `use` graph, contingent on first validating the Bin-role warning corpus); out-of-package `use` is **unreachable**                                                                                                                                                                                                                                                                                |
| **Script**   | A file executed directly as a single file (`run foo.yx`)                                                                                                                | **No entry-point concept**. Top-level statements (including binding initialization) constitute the program body in source order; `main` is an ordinary binding and is **not called automatically**—write `main()` if it should run. The Registry contains only std                                                                                                                                                                                                                                                                         |

**Classification priority**: explicit manifest declaration > entry-point reachability inference >
the RFC-036 test convention. An explicit declaration always wins—inference is only the default when
no declaration exists.

### Entry-Point Determination

The role model is used not only for dead-code analysis but also to **drive entry-point
determination**. Entry-point rules differ by role:

| Role                      | Entry-point rule                                                                                                                                                                                                                                                                                   |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Script** (no manifest)  | **No entry-point concept**. Top-level statements (including binding initialization) constitute the program body in source order; `main` is an ordinary binding and is **not called automatically**—write `main()` if it should run                                                                 |
| **Bin** (with a manifest) | A **binding** named `main` is required (either a value or a function: a value `main` is evaluated during initialization, and a function `main` is called with zero arguments at entry); a missing `main` is a compile-time error (E3020). Executable statements are not permitted at the top level |
| **Lib / Internal / Test** | `main` is not required (they are not entry points)                                                                                                                                                                                                                                                 |

**The Script and Bin entry points must be mutually exclusive**: “`main` is not special under Script”
and “`main` is the entry point under Bin” cannot both hold. If a Script both executes top-level
statements and implicitly calls `main`, a script that explicitly calls `main()` would run twice
(#356). A Script therefore has only one execution entry point: its top-level statements.

**The distinction between role classification and entry-point determination** (implementation note):
entry-point determination uses the presence or absence of a manifest
(`find_project_root().is_some()`), rather than `roles::classify()`. This is because classify answers
“who consumes this file?” (serving dead-code analysis). A file in a manifested project that has no
`main` is classified as Internal by classify (because no other file uses it), but if the user runs
it as an entry point, it must still be treated as one. These are different questions and therefore
require different criteria.

**Execution form of a value `main`** (decision of #388): functions are already values (unified at
the type layer), and entry points are determined by **binding existence** rather than callability—a
value `main` and a function `main` differ only in their evaluation strategy. A value `main` does not
receive an entry-point call (#356 prevents double execution: initialization and entry-point
invocation never overlap), and uses the same execution form as Script. Its evaluation is part of the
initialization sequence: global bindings execute in topological order, while independent bindings
execute in source order. Consequently, migrating Script top-level statements into `main = { ... }`
when moving to a Bin **does not preserve ordering**: scattered statements execute in source order,
whereas binding initialization executes in topological order. Initialization that must run first
should be made an explicit dependency of `main`. A non-callable value `main` such as `main: Int = 5`
is legal but has no observable effect (analogous to an empty Rust `fn main() {}`).

See `docs/src/reference/language-spec/syntax.md` §3.11.

### Examples

```toml
# yaoxiang.toml (all fields are already defined by RFC-015; this RFC adds no configuration surface)
[lib]
path = "src/lib.yx"

[[bin]]
name = "my-cli"
path = "src/cli.yx"

[exports]
"." = "src/lib.yx"
"./internal-helper" = "src/helper.yx"   # ← This, not pub, determines cross-package visibility
```

```yaoxiang
# src/cli.yx (Bin role)
pub unused_fn = (x: Int) => x    # ← W1001 may be reported: bin has no out-of-package consumers
main: () -> Void = { ... }

# src/lib.yx (Lib role, on the export surface)
pub api_fn = ...                 # ← Never reported: out-of-package consumers are not visible

# src/other.yx (Internal role, not on the export surface)
pub semi_api = ...               # ← Exempt until phase two (prefer under-reporting); it will be tightened using the use graph then
```

### Import-Surface Semantics

The resolution order for a cross-package `use pkg.x` is as follows (the first match takes effect):

1. If an `[exports]` mapping exists → the import surface is the set of files listed by the mapping,
   and `x` must be among the top-level bindings in those files;
2. Otherwise, if `[lib].path` exists → the import surface is that single file;
3. If neither exists (a path dependency on a bare directory) → **preserve the current behavior**:
   all top-level files are importable (for compatibility with existing behavior; narrowing will be
   considered separately).

**Workspace members**: an export surface is defined only by the member package's own manifest. The
workspace root must not override or extend a member's export surface—member self-containment is the
core design of 014c, and root overrides would break encapsulation.

Relationship to Registry reachability (aligned with 029): `[exports]`/`[lib]` define the **set of
legal starting points for cross-package resolution**. Once the starting points have been determined,
the Registry still expands from them according to 029's rule of “reachable from the entry point
along `use`”—internal files imported by an exported file enter through linkage, but do not create
new cross-package starting points.

### Boundaries with Existing Plans

| Neighbor                                                | Boundary                                                                                                                                                                                                                                                                                        |
| ------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| RFC-029d “CLI `--entry` entry-point override” (planned) | 029f defines the role model; 029d consumes it—`--entry` is a CLI-level override mechanism for the Bin role                                                                                                                                                                                      |
| RFC-014b `[binaries]`                                   | **Terminology disambiguation**: `[binaries]` denotes precompiled distribution artifacts (download-priority strategy), whereas `[[bin]]` declares source roles. The former exists in a dependency package's manifest, the latter in the current package's manifest; they are not interchangeable |
| RFC-014c workspace                                      | The import surface for imports between member packages follows this RFC's resolution order; workspace coordination mechanisms (members, shared lockfile) remain under 014c                                                                                                                      |
| RFC-036 testing                                         | Test-role classification directly adopts 036's existing file rules and introduces no new conventions                                                                                                                                                                                            |
| #321 dead code                                          | Decision B (pub is always exempt) is explicitly downgraded to “transitional semantics when there is no target”; reporting pub bindings in the Bin role is this RFC's implementation of option A                                                                                                 |

## Detailed Design

### Role Classification Algorithm (Within the Orchestrator)

```
fn classify(files, manifest, entry_reach) -> Map<File, Role>:
    # 1. Explicit layer: manifest declarations take priority
    for f in manifest.exports.values():  role[f] = Lib
    if manifest.lib_path:                role[lib_path] = Lib
    for b in manifest.binaries:          role[b.path] = Bin
    if manifest.run_main:                role[run_main] = Bin

    # 2. Inference layer: undeclared files are classified by entry-point reachability
    for f in files where role[f] 未定:
        if f 被 ≥1 个非自身文件 use:      role[f] = Lib
        elif f 含 main 且无其他文件 use 它: role[f] = Bin
        else:                             role[f] = Internal

    # 3. Test layer: RFC-036 rule match → Test (overrides Lib/Internal, but not an explicit Bin)
    apply_rfc036_test_rules(files, &role)

    # Single-file direct execution: bypass the entire model; behavior remains unchanged
    if no manifest and single_file:      all Script
```

### Compiler Changes

- `frontend/config.rs`: add the `[exports]` → role-table mapping to manifest parsing (field parsing
  is already provided by 015).
- `frontend/module/orchestrator.rs`: insert a role-classification stage before constructing the
  Registry; validate `use` targets according to the import-surface resolution order, and report
  out-of-surface access through the existing `module_not_found` family (no new error code; the
  message gains a “not in export surface” hint).
- `typecheck/passes/dead_code.rs`: change the entry-point set from “main + all pub” to “main + pub
  in Lib-role files” (Bin may be reported immediately; Internal remains exempt until phase two; see
  the implementation strategy for Phase 2).
- LSP (RFC-017): filter “importable items” for completion and hover by the import surface; report a
  missing-`main` diagnostic only for Bin-role files.

### Runtime Behavior

There is no change. The role model consists purely of compile-time and resolution-time semantics and
does not affect the IR or execution.

### Backward Compatibility

- Projects without a manifest: behavior remains completely unchanged (the inference layer restores
  the current behavior—files that are used are Lib, and files that contain `main` are Bin). The only
  incremental change to dead-code warnings occurs in Bin files: an unused pub in a root file that no
  one imports changes from silent to W1001. This is the expected behavior of #321 option A, not a
  breaking change.
- Projects with a manifest: when `[exports]` is declared, the import surface narrows from “all pub
  files” to the declared surface. **This is the only behavioral narrowing point**: code that
  accesses another package's internal files will begin to report `module_not_found`. Given that the
  package ecosystem has not yet been established (029: “there is currently no third-party package
  ecosystem”), the breakage surface is zero; nevertheless, a one-minor-version transition period
  will be provided (out-of-surface access warns before it becomes an error).

## Trade-offs

### Advantages

- Zero new configuration surface: every field is already defined by 015; this RFC only supplies
  semantics.
- Zero new keywords: the import surface is carried by distribution boundaries, isomorphic to 029's
  “visibility does not exist” decision.
- One model serves four consumers (#321 / 014c / 029a / 037), eliminating the need for each to
  define its own semantics.
- Zero-cost path for projects without a manifest: the script language's zero-configuration nature is
  fully preserved.

### Disadvantages

- Role inference (being used implies Lib) classifies a utility file as Lib in the common pattern
  where the entry file uses a utility file, making its pub exemption broader than the ideal
  granularity. This is an intentional choice in the under-reporting direction; tightening it
  requires call-direction analysis on the `use` graph, which is out of scope for the first version.
- Narrowing the import surface through `[exports]` is a breaking semantic tightening (although a
  transition period is provided).

## Alternatives

| Alternative                                    | Description                                                                       | Reason not adopted                                                                                                                                                                                                                                 |
| ---------------------------------------------- | --------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Top-level RFC-040                              | Open a new top-level RFC number                                                   | Module-graph semantics (import surface/reachability/entry points) all belong to 029's domain; splitting them into a top-level RFC would produce two top-level documents for the same object, and the 029b–029e slot pattern is already established |
| Incorporate it into 014c                       | Add it as a section of the workspace RFC                                          | The dependency direction is reversed: the language layer (029x) defines the semantics, while the package layer (014x) consumes them. 014c is still under review, and expanding its scope midway would impede implementation                        |
| Option A as originally written (#321 menu)     | Dead-code semantics require a manifest with mandatory `[[bin]]`/`[[lib]]` targets | This introduces mandatory configuration for a narrow slice (an unused pub in a root file with `main`); this RFC's inference layer obtains the same benefit with zero configuration                                                                 |
| Pure entry-point inference (no manifest layer) | Do not consume the RFC-015 fields                                                 | `[exports]` is already an accepted RFC-015 field, and the cross-package import surface cannot avoid it; leaving it undefined would create a gap for 014c                                                                                           |

## Implementation Strategy

### Phases

- **Phase 1**: role classification + import-surface resolution + reporting of pub bindings in Bin
  files (Internal/Test remain exempt)
- **Phase 2**: tighten Internal pub bindings so that those unreachable in the in-package `use` graph
  may be reported—the prerequisite is validation of the Bin-role warning corpus after Phase 1 lands,
  with no false positives; implement as soon as this condition is met, with no fixed timetable

### Dependencies

- Prerequisites: RFC-029 orchestrator (already implemented), RFC-015 manifest fields (already
  defined).
- Dependents: #321 (reporting pub bindings in Bin files), RFC-014c (import surface), RFC-029a (roles
  as cache-unit boundaries), RFC-037 (artifact selection), RFC-029d (`--entry` override).
- Implementation order relative to RFC-014c: before 014c is approved, 029f's import-surface
  resolution order should be finalized, and 014c should consume it by reference so that its scope
  does not expand midway through review.

### Risks

- Deviation between role inference and user intuition (the first trade-off above) → present all
  incremental behavior as non-blocking warnings so it can be disabled at any time.
- Managing the transition period for the `[exports]` narrowing → warn first within the next minor
  version.

## Open Questions

All three draft-stage questions have been decided (2026-09-13; resolutions recorded in Appendix B
and the body of this document). There are no unresolved items:

- [x] Whether Internal pub bindings should be tightened → **Decision**: tighten them in phase two
      according to reachability in the in-package `use` graph, contingent on first validating the
      Bin-role warning corpus (see Phase 2 of the implementation strategy)
- [x] Explicit `[[test]]` declaration → **Decision**: do not introduce it; the Test role is
      classified solely by the RFC-036 rules. If 036 later needs an explicit declaration, it will be
      extended by 036 itself
- [x] Workspace root overriding a member's export surface → **Decision**: not permitted; member
      self-containment is a core principle of 014c, and a root override would break encapsulation

---

## Appendix B: Design Decision Record

| Decision                                                     | Resolution                                                                                                       | Date       | Recorder | Basis                                                                                                                                           |
| ------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------- | ---------- | -------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| Number                                                       | 029f (029b was reserved for hot reload, 029c was deleted, and 029d/029e are occupied)                            | 2026-09-12 | Chenxu   | RFC-029 §Sub-RFC Plan                                                                                                                           |
| Belongs to the 029 family rather than being a top-level RFC  | Yes                                                                                                              | 2026-09-12 | Chenxu   | Import surface/reachability/entry points all belong to module-graph semantics; 029 has already consumed the `[[bin]]` entry-point priority      |
| Visibility keyword                                           | None introduced                                                                                                  | 2026-09-12 | Chenxu   | Isomorphic to RFC-029's “visibility does not exist” decision; the import surface is carried by distribution boundaries                          |
| Status of decision B from #321                               | Downgraded to “transitional semantics when there is no target”                                                   | 2026-09-12 | Chenxu   | Reporting pub bindings in the Bin role is the implementation of #321 option A                                                                   |
| Role granularity                                             | File-level                                                                                                       | 2026-09-12 | Chenxu   | RFC-015 field granularity (`[lib]` single file, `[[bin]]` list, `[exports]` mapping)                                                            |
| Disambiguating `[binaries]`                                  | 014b precompiled distribution artifacts ≠ `[[bin]]` source roles                                                 | 2026-09-12 | Chenxu   | Two separate layers of concepts (download strategy vs. compilation role); fixed before 014c is implemented                                      |
| Tightening Internal pub bindings                             | Tighten in phase two according to in-package `use`-graph reachability; prerequisite = Bin-role corpus validation | 2026-09-13 | Chenxu   | Progressive tightening in the under-reporting direction; corpus-driven to avoid premature false positives                                       |
| Explicit `[[test]]` declaration                              | None introduced; Test is classified solely by RFC-036 rules                                                      | 2026-09-13 | Chenxu   | Do not add configuration surface before RFC-036 needs it                                                                                        |
| Workspace root overriding a member's export surface          | Not permitted                                                                                                    | 2026-09-13 | Chenxu   | Member self-containment is a core principle of 014c; a root override would break encapsulation                                                  |
| Whether `main` is called automatically under Script          | **No**—top-level statements are the program; `main()` must be explicit                                           | 2026-09-17 | Chenxu   | Coexistence of both rules would cause an explicit `main()` to run twice (#356); Script can have only one execution entry point                  |
| Whether the role model is used for entry-point determination | **Yes** (previously it was used only for dead-code warnings)                                                     | 2026-09-17 | Chenxu   | This RFC defines Bin's “`main` is a reachability root”; without connecting it to entry-point determination, that semantics would have no effect |

## Appendix C: Glossary

| Term                  | Definition                                                                                                                                    |
| --------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| Role                  | The compilation-target identity of a source file: Script / Bin / Lib / Test / Internal                                                        |
| Import Surface        | The set of files that may serve as legal resolution starting points for cross-package `use`, declared or inferred through `[exports]`/`[lib]` |
| Distribution Boundary | RFC-029 term: a package's externally exposed scope; this RFC refines it from the implicit form (all pub files) to the import surface          |
| Script Form           | The bypass form for directly running a single file without a manifest; all behavior remains identical to the current implementation           |

## References

- RFC-029, Module Semantics (parent RFC: entry-point selection, visibility decisions, sub-RFC plan)
- RFC-015, Configuration System (definitions for the `[lib]`/`[[bin]]`/`[exports]` fields)
- RFC-014b, Build System and Binary Distribution (`[binaries]`; the subject of terminology
  disambiguation)
- RFC-014c, Workspace Support (the first consumer of the import surface; under review)
- RFC-036, Testing Framework (source of the Test-role rules)
- #321 M2, Independent Warning-Code Emission Channel (origin of decision B and option A)

## Known Defects: Implemented (2026-10-16, WBS 4.9 Round 1)

> The previous version (2026-10-09) listed the following three items as “three design defects in the
> reference pool.” All three were implemented in WBS 4.9 Round 1 (commit `3863a9ff0`), one by one.
> `collect_project_refs` and `orchestrator.rs` were moved into `src/driver/{state,arms,helpers}.rs`
> as part of the P4 4.2.9 split; all file:line references in the table below were retaken at the
> cited revision.

### Three Design Defects in the Reference Pool — Correction Status

| #   | Original defect                                                        | Current state (2026-10-16)                                                                                                                                                                                                                                                                                                                                                                                                                                  | Implemented code (revision `3863a9ff0`)                                                                                                                 |
| --- | ---------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | The scan surface was the directory rather than the compilation surface | **The pool's root = the package target**: `collect_package_refs` (`helpers.rs:481`) uses the `[[bin]]`/`[lib]`/`[exports]`/`[run].main` roots from `roles::ExplicitSurfaces` to invoke `discover_with_used` and obtain the compilation set; **it does not scan the directory to find an entry point**. When none of the roots can be run, it falls back to the compilation set of the file under inspection (`collect_compilation_refs` / `helpers.rs:454`) | `src/driver/helpers.rs:454-512` (fallback path); `src/driver/helpers.rs:481-512` (manifest-declared surface path)                                       |
| 2   | The granularity was project-level bare names, with no module dimension | **Pool type = `HashMap<String, HashSet<String>>`**: keys are module keys, and values are the set of reference names for that module; the consumer unions the entries of every module except the current unit and passes them to `DeadCodeAnalyzer`. A module's own names (top-level definitions and local bindings, except names imported through `use`) no longer exempt identically named definitions in other modules                                    | `src/driver/arms.rs:603-609` (consumer-side `other_module_refs`); the sole assembly entry point is `src/driver/helpers.rs:429-449` (`collect_ref_pool`) |
| 3   | Script reused Lib's pool, violating single-file semantics              | **Script does not connect to the pool**: the `match us.role` in `arms.rs:611-623` only assembles `Bin`/`Internal` through `set_exempt_pub` and `Lib` through `set_cross_file_refs`; Script/Test take the `_ => {}` default arm. The default arm judges only by the file-local definition-use graph                                                                                                                                                          | `src/driver/arms.rs:611-623`                                                                                                                            |

**Subsequent Improvements 4 / 5 — Correction Status (2026-10-16, WBS 4.9 Round 2)**

> These two items were previously listed as “pending P4 4.9 implementation” in this section. Both
> were implemented in WBS 4.9 Round 2 (commit `e6cf1aeaa`, pushed on 2026-10-17); all file:line
> references in the table below were retaken at revision `e9675c9dd` (a note added during the
> 2026-10-17 Round 3 closure after the Round 2 implementation—the fix round and closure round
> changed the same set of anchor files, so the line numbers use the retaken values; the lesson from
> Round 1 is that a local hash before push can be rewritten by `pull --rebase`).

| #   | Original improvement                                                                                                 | Current state (2026-10-16)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Implemented code                                                                                                                                                                                                                                                                                        |
| --- | -------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 4   | Refine consumption: distinguish “imported” from “actually used after import” (only the latter counts as consumption) | **Consumption criterion = “actually used after import”**: a consumption record means that a module uses `use` to bring the name n from module M into scope **and actually references n at the use site**, and it is recorded only against M; a name that appears only in a `use` item, with no use site, and a reference to an identically named identifier in another module do not count as consumption. The target module of an import is determined from the discovery-phase module-key set (`resolve_import_module`, **without introducing a separate path-resolution mechanism**—it reuses the discovery-phase resolution output delivered in 4.6.1; see `39546020`). The namespace form (`use lib` + `lib.foo()`) is attributed in the same way | `src/driver/helpers.rs:442-447` (`RefPool`); `src/driver/helpers.rs:449-506` + `:511-524` (`collect_ref_pool` + `resolve_import_module`); `src/frontend/core/typecheck/passes/dead_code.rs:186-200` (`ModuleNames`), `:384` (`collect_module_names`), `:132-184` (`ImportFace` / `collect_import_face`) |
| 5   | Separate the test pool: references from tests/ do not exempt production code (src/)                                  | **`FileRole::Test` contributes no consumption surface**: references from a Test module form a separate pool (Test itself does not participate in dead-code determination, so its references do not spill out), and a production definition is kept alive only by production consumption. The role is determined at the single point `roles::classify` (test discovery rule RFC-036)                                                                                                                                                                                                                                                                                                                                                                    | `src/driver/helpers.rs:463-466` (Test short-circuit in `collect_ref_pool`); `src/driver/helpers.rs:707-708` (single-point `roles::classify`); `src/driver/arms.rs:633-635` (Test skip shared by the fallback and declared-surface paths; role-to-pool feeding at `:661-672`)                            |

**Residual surface (registered, non-blocking)**: ① Field/method member references (`obj.render()`
contains only a short name) cannot be attributed at the AST level to the module that owns the
receiver's type. When the base name matches a namespace import, attribution follows the import;
module-qualified construction `ns.T` is attributed as a method through the **type-name surface**
(see §4.9.7 below); all remaining cases are conservatively counted at the name dimension
(`RefPool::field_names` in `src/driver/helpers.rs`'s `collect_ref_pool`). ② Method reachability is
narrowed **per candidate and per definition unit** according to the D20 decision output
(`resolved_method_marks`)—an unmatched sibling overload within the same unit is faithfully reported
as W1005; a same-name, same-order uncalled definition across units in a reachable form is likewise
faithfully reported (WBS 4.9.7 / issue #464, closed; mechanism and measurements below;
reference-producing-side commit `b8b3cc3f6`). **Residual boundaries (registered; do not claim global
coverage; all are based on measurements)**: (a) running `check <definition file>` alone (the file
has no call site, and its caller is not in the unit set) ⇒ no decision entry ⇒ fallback to the
pool's **name-level exemption** (`RefPool::field_names` globally flattened) ⇒ **no report**; (b) the
same bare type name is brought into scope by multiple modules (the source program is already
ambiguous at the name layer, and the entry point separately reports E5006) ⇒ the candidate surface
skips the entire name ⇒ no decision entry ⇒ a same-name, same-order group falls back to the
name-level exemption and is not reported; (c) the caller makes **any** method call on another unit's
type name in the current unit (even if it does not hit that unit's definition) ⇒ that other
definition is merged under **possible ownership** by the `typed_owners` rule ② ⇒ no report. If both
sides only construct values and make no method calls, `typed_owners` does not apply, and both sides
are faithfully reported as W1005 (t20 measurement fixture: both sides only construct ⇒ 2 W1005); (d)
whole-module import `use M` does not bring a bare type name into scope, and the std module's
candidate surface is not connected—both forms produce no entry and fall back to the name-level
exemption. Fallback boundary: when a name group has no decision record in the current unit (for
example, call-site ambiguity E1107 or a call form that does not go through resolution), judgment
falls back to collective short-name matching (in the under-reporting direction)—therefore, do not
claim that “uncalled methods are always faithfully reported.”

### Reference Count Distortion (WBS 4.9.6 / issue #450) — Correction Status

> Original registration: an **overloaded method that was called and whose resolution took effect was
> still falsely reported as `W1005 Unused method`** (reference-count distortion for overloaded
> calls). On 2026-10-16, user decision A registered this as an independent tier-three task and
> merged it into WBS 4.9 Round 2. This is a category D item (removing a false positive changes the
> externally observable diagnostic behavior) and was authorized for implementation.

| Stage                        | Original defect                                                                                                                                                                                                                                                              | Current state                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Implemented code                                                                                                                |
| ---------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| Reachability probe           | The scan looked up references by **qualified name** (`Point.render`), whereas the collected reference sites use **short names** (`obj.render()` contains only `render`) ⇒ a called method was never reachable, causing a false W1005 when no project-root path was available | Methods are now judged by a **short-name probe**, with the **D20 decision output** used to narrow them per candidate (`resolved_methods` + `resolved_groups`; sibling overloads not matched by resolution are still reported as W1005). Scope boundary: **within the same unit**, the per-candidate rule holds; **across units** (registry resolution), after the producing side (`b8b3cc3f6`) and driver-side attribution (this batch) were implemented, the same per-candidate and per-definition-unit rule holds in a **reachable form** (see residual surface ② and §4.9.7 below) | `src/frontend/core/typecheck/passes/dead_code.rs:782-792` (probe); `:815-877` (per-definition determination and exemption gate) |
| Definition-table key control | `all_defs` was keyed by qualified name ⇒ same-name overloads overwrote one another (only one of two remained, so only one was reported)                                                                                                                                      | The definition table now contains **one entry per definition**: methods are appended, while same-name non-method bindings are deduplicated by name and the last is retained (overloads are independent definitions, whereas a top-level reassignment is the same binding)                                                                                                                                                                                                                                                                                                             | `src/frontend/core/typecheck/passes/dead_code.rs:304-322` (`record_definition`), `:28-29` (fields)                              |

**Reason not to address this in P3.5 (unchanged from the previous version)**: module-level
resolution requires `use`-item path resolution to have completed (`use a::foo` → foo comes from
module a), which belongs to the P4 stage contract and the unified Driver scope.

### Cross-Unit Method Reachability Attribution (WBS 4.9.7 / issue #464) — Correction Status

> Original registration (residual surface ②): cross-unit method calls do not produce D20 decision
> entries ⇒ the name group falls back to the pool's **name-level** exemption ⇒ **same-name,
> same-order uncalled methods across units are not reported**. This item was split into two
> implementations: the **producing side** (cross-unit method calls produce decision entries, commit
> `b8b3cc3f6`; its injection surface is filtered pairwise by name ownership in the fix round; see
> the table below) and the **driver-side attribution** (provide reachability marks per definition
> unit and disable the name-level exemption). The table's file:line references are taken from the
> **driver-side attribution worktree**—commit `d5c00235a`; the producing-side commit is `b8b3cc3f6`
> (above). All file:line references in this section are uniformly taken from revision `e9675c9dd`
> (retaken during the 2026-10-17 Round 3 closure, covering the shift in the injection range caused
> by fix-round commit `d5dd1a37b` after `d5c00235a` landed); commit numbers always use their
> post-push values (a local hash before push can be rewritten by `pull --rebase`, see.

| Stage                     | Original defect                                                                                                                                                                                                                                                           | Current state                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | Implemented code                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Decision-entry production | All three candidate tables were missing for cross-unit call sites (`method_overloads` / `method_overload_ir_names` / `method_overload_param_names`) ⇒ the resolution stage was skipped entirely ⇒ **zero entries**                                                        | The candidate group's “candidate / mixed IR name / declaration-order formal-parameter names” are packaged in the same order and passed across units through `ModuleInfo.method_overloads`; the Registry merges them deterministically in ascending module-key order. The Typecheck arm takes candidate groups pairwise according to the current unit's import face—only a group is injected when the key's provider module is the same as the provider module of that **type-name prefix** in the current unit's scope (name ← module pairing, with no cross-pair combinations; an owned key slot is not merged outward, and a same-name conflict skips the entire name) ⇒ at one call site, `cands=1`, with one entry (`Temp.to`). **Fix-round measurement (F1)**: the previous “name set × module set” combination included another module's same-name type candidate in the same key slot ⇒ `check` reported 2×E1107 and exited with 1 (a program that had previously compiled successfully now failed to compile); after pairwise filtering, both `check` and `run` exit with 0, and the fixture is stored at `tests/yaoxiang-multifile/ref-pool-cross-unit-name-owner/` | `src/frontend/core/typecheck/environment.rs:145` (`merge_from`: same-order group-level append), `:336` (snapshot), `:384` (injection), `:399` (`append_overload_group`: the **only** insertion point for the three parallel tables, with an equal-length debug assertion before insertion); `src/frontend/module/registry.rs:150` (`merge_method_overloads`); `src/frontend/module/mod.rs:155-157` (`ModuleInfo` fields); `src/driver/arms.rs:344-358` (injection surface), `:1521-1579` (`visible_method_overloads`, the single injection point: name ← provider module, pairwise; same-name conflicts skip the entire name; owned key slots are not merged outward). **Fix-round injection surface (F1, `d5dd1a37b`)**: the previous “name set × module set” combination included another module's same-name type candidate in the same key slot ⇒ `check` reported 2×E1107 and exited with 1 (a program that had previously compiled successfully now failed to compile); after pairwise filtering, both `check` and `run` exit with 0—the fixture is stored at `tests/yaoxiang-multifile/ref-pool-cross-unit-name-owner/`, with a pinned test at `src/driver/tests/overload_injection.rs` (relocated in commit `c2f887843`) |
| Driver-side attribution   | Decision entries were globally marked by **name** (a set of `(base name, declaration order)`) ⇒ a same-name, same-order definition in another unit was exempted by a call to its sibling (same-name, same-order cross-unit uncalled methods were previously not reported) | Marks are now attributed by **definition unit** (`MethodMarks { by_unit, groups }`). For an entry named `{Type}.{method}#k`, `{Type}` is the **definition-side** bare name and does not carry a unit; the recording unit's **type-name surface** is used to calculate the set of modules from which that unit can construct the name, and the entry is assigned to each unit in that set through `by_unit`; `groups` is the global set of name groups for which entries have occurred                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | `src/driver/helpers.rs:569-573` (`MethodMarks`), `:591-619` (`typed_owners`: brace-list items + inline alias targets + module-qualified access `ns.T`, reusing `collect_module_names` + `resolve_import_module` at a single point), `:621-654` (`resolved_method_marks`, a pure read through `OverloadResolutions::iter`); `src/driver/arms.rs:650-658` (per-unit retrieval of `by_unit[unit_key]`), `:649-660` (injecting the analyzer)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| Judgment surface          | A name group with decision records still used the pool's **name-level** exemption (short names flattened globally)                                                                                                                                                        | A name group with decision records **does not use the name-level exemption**: definitions declared by the current unit that are matched are retained, and unmatched definitions are faithfully reported as W1005 (`!marked_group` is a precondition of the exemption gate)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | `src/frontend/core/typecheck/passes/dead_code.rs:44` (fields), `:252-257` (`set_resolved_groups`), `:262-267` (`method_group_has_marks`), `:782-795` (per-candidate reachability), `:815-877` (per-definition determination / exemption gate / W1005 emission)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |

**Externally Observable Changes (Category A: old behavior was unsuccessful ⇒ fix it as part of the
same change)**:

- **Cross-unit overload resolution previously did not take effect**: all three candidate tables were
  missing ⇒ it fell back to the cross-unit single-value binding table (`method_bindings`, where
  same-name definitions use last-wins) ⇒ a call to a same-name overloaded method reported
  **`error[E1002]`** (measured: `Expected type 'string', found type 'int64'`). After the fix,
  candidates are resolved according to the argument type, and the same fixture reports
  `Type check passed`. The criterion is implemented by the integration case
  `test_multifile_cross_unit_method_overload_resolves_by_argument_type`
  (`tests/integration/multifile.rs`, with same-name, same-order Int/String candidates and two calls
  that respectively match them)—turning off the injection surface makes this case fail with the same
  E1002.
- **Same-name, same-order uncalled methods across units now begin to be faithfully reported as
  W1005**: measurements with three fixtures (`main.yx` imports type `Temp` through `use a.{Temp}`,
  and `use b` only brings b into the compilation set) show that `a.Temp.to`, which has a call site,
  is not reported; `a.Temp.other` is reported; and **`b.Temp.to` is reported**. Before the fix, the
  same fixture reported only `a.Temp.other` (b's definition survived through the pool's name-level
  exemption). The fixture is stored at `tests/yaoxiang-multifile/ref-pool-cross-unit-method/` and
  can be rerun from the CLI; the driver-level pin is `src/driver/tests/method_unused.rs`'s
  `test_cross_unit_uncalled_same_named_method_is_reported`.

**Residual Boundaries (registered; do not claim global coverage; each is based on measurements)**:

1. Running **`check <definition file>` alone** (the file has no call site, and its caller is not in
   the unit set) ⇒ no decision entry ⇒ fallback to the name-level exemption ⇒ **no report** (in the
   under-reporting direction). Measured: running `check b.yx` with the same three-file fixture
   reports only an unused type W1002 and does not report `b.Temp.to`.
2. **Ambiguity at the name layer**: the same bare type name is brought into scope by multiple
   modules (`use a.{Temp}` + `use b.{Temp}`) ⇒ the candidate surface skips the entire name ⇒ no
   decision entry ⇒ a same-name, same-order group falls back to the name-level exemption and is not
   reported; this entry point separately reports `E5006 Duplicate import`. Comparative measurement
   (importing only b as a namespace) faithfully reports the sibling candidate, proving that the
   criterion is not a no-op.
3. The caller makes **any method call** on another unit's type name in the current unit (even if it
   does not hit the other side's definition) ⇒ that other definition is merged under **possible
   ownership** ⇒ no report. If both sides only construct values and make no method calls,
   `typed_owners` does not take effect, and both sides are faithfully reported as W1005 (measurement
   fixture `t20-b3-no-method-call`: both sides only construct ⇒ 2 W1005)—in the under-reporting
   direction.
4. **Inline alias `use M.{T as X}` and module-qualified construction `ns.T` do not inject the
   candidate surface** (producing-side narrowing rule ②): the call-site key remains the
   definition-side `T` (measurements of both `X(..).to()` and `ns.T(..).to()` produce the key
   `T.to`) ⇒ the precision of `#k` for same-name types remains bounded by “the set of modules from
   which the recording unit can construct the name.” It **does not distinguish which unit's
   definition was actually selected**, and for this reason these forms **cannot** cause another
   same-name, same-order definition to be declared dead (doing so would falsely report a method that
   was genuinely called).
5. **Whole-module import `use M` does not bring a bare type name into scope** (measured: writing
   `Temp(...)` reports E1001), and the **std module's candidate surface is not connected** (its
   `ModuleInfo` is constructed through the std registration path)—neither form produces an entry, so
   both fall back to the name-level exemption.

**The driver-side read is a pure read**: `resolved_method_marks` traverses
`OverloadResolutions::iter` (it does not call `get` and does not enter the consumption ledger) ⇒ it
does not conceal settlement of W1090 “consumption must be reachable”; the driver-level pin
`test_resolved_method_marks_read_leaves_consumption_ledger_untouched` verifies that the entry
remains in the `unconsumed()` difference set after the read.

**Closure Measurement Addendum (2026-10-17 Round 3; after injection-surface fix round `d5dd1a37b`
and relocation of the pinned test to `c2f887843`)**: Injection-surface filtering takes candidates
pairwise under “name ← provider module” (only groups whose “key's provider module == the provider
module of that type-name prefix in the current unit's scope” are merged). The single point is
`visible_method_overloads` (`src/driver/arms.rs:1521-1579`; production call site
`src/driver/arms.rs:354`); downstream, there is only one insertion point: `environment.rs:399`'s
`append_overload_group` (the three parallel tables are appended in the same order with an
equal-length debug assertion; the only group-level merge is `merge_from` at `environment.rs:145`).
Mutation verification: restoring cross-pair filtering makes the driver-level pins fail with
2×E1107/2×E1053, while all three assertions in the main acceptance fixture remain unchanged (proving
that the fix did not regress the main acceptance path). Independent verification/review used a
paired comparison of two binaries built from frozen revisions (before the fix: `d5c00235a`; after
the fix: `e9675c9dd`): the diagnostic sets on both sides are identical for the main acceptance
fixture `tests/yaoxiang-multifile/ref-pool-cross-unit-method/`; the negative controls (no same-name
type / same-name type but a different method name) are green on both sides; the existing ambiguity
behavior for a same bare type name brought into scope by multiple modules (constructor/value
dimension) is identical on both sides; the reverse false-negative surface is covered by faithfully
reporting a same-name, same-order **uncalled** definition as W1005 (the fix no longer swallows it).

**Criteria and Drift (Measurements for this Batch, Including the Closure Round)**: driver-level
pinned tests—5 for attribution (main acceptance / inline-alias false-positive defense /
module-qualified-name false-positive defense / bidirectional key scope / pure read,
`src/driver/tests/method_unused.rs`) + 3 for injection surface (name-ownership pairwise filtering /
no false positive when method names differ / equal-length pairing of the three tables,
`src/driver/tests/overload_injection.rs`, relocated in commit `c2f887843`), plus 4 older #450
criteria that do not regress; 204 IR snapshots with zero drift; corpus baseline 346 → 347 rows
(driver-side attribution round, new fixture `ref-pool-cross-unit-method/`) → 348 rows
(injection-surface fix round, new fixture `ref-pool-cross-unit-name-owner/`), with each round adding
exactly one behavior record and every existing row remaining byte-for-byte unchanged (the baseline
`tests/baselines/corpus-parity.jsonl` contains 348 rows at `e9675c9dd`).

### Cross-Unit Same-Name Type Method Dispatch Attribution (WBS 4.9.8 / issue #465) — Correction Status

> Phenomenon: two compilation units each define a **same-name type** `Temp` and the same method name
> `Temp.tag` (the implementations can be distinguished: a = `v + 1.0`, b = `v * 10.0`). The entry
> point introduces a through a bare name (`use a.{Temp}`), then introduces b through an **inline
> alias** (`use b.{Temp as TempB}`) or **module-qualified construction** (`b.Temp(..)`). Both sides
> **construct correctly** (the field values are correct), but `u.tag()` dispatches to a's
> implementation; `run` exits with 1 (assertion failure)—a silent incorrect value (D59 category C).
> The `check` warning surface depends on whether the manifest is readable: the manifest form given
> in the issue (`[package]` containing only `name`, with no `version`) actually produces **2
> `W1002 unused type` warnings** (`a.yx:1:1` / `b.yx:1:1`); only adding `version` produces 0
> warnings. Manifest parse failure / missing required fields silently degrades (residual boundary
> 4), which is unrelated to the dead-code criterion itself.

| Stage                      | Original defect                                                                                                                                                                                                                                                                                               | Current state                                                                                                                                                                                                                                                                                                         | Implemented code                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Dispatch-name source       | The dispatch name is the **definition-side bare type name** + method name (`Temp.tag`). Typecheck's `MonoType` retains only the definition-side bare name for imported types (`Temp`), so same-name types in different units cannot be distinguished at the type layer                                        | The dispatch name is first attributed to the definition unit according to the **type name written by the receiver**: `Temp.tag` → `b.Temp.tag` (written name → import surface → definition-unit qualified name). **Method calls and operator interfaces (Index / Equal) use the same criterion**                      | `src/middle/core/ir_gen.rs:1265` (`method_dispatch_name`, the **sole criterion**), `:1232` (`imported_type_owner`: `use_aliases` + `Resolver::resolve_namespace` + the Registry's single source, `ExportKind::Type`; **an exact authoritative qualified-name match returns immediately and is not resolved again**), `:1188` (`constructor_written_type_name`, pure syntactic extraction), `:1169` (`written_type_name_of`, extracts the name as written) |
| Recording the written name | No such concept existed: `ir_gen` dispatched only by the bare name in `MonoType`                                                                                                                                                                                                                              | Local variables are recorded in symbol-table entries (`SymbolEntry.written_type`, with scope tracked automatically); top-level bindings are recorded in a per-unit table (`global_written_types`; the Script form `t = Temp(1.0)` is a global slot); formal-parameter annotations are recorded through the same chain | `src/middle/core/ir_gen.rs:1115` (`note_local_written_type`), `:1129` (`note_param_written_types`), `:1159` (`binding_written_type`: local first, then top-level), `:109` (`SymbolEntry.written_type` field), `:3255` (top-level registration point)                                                                                                                                                                                                      |
| Five production paths      | Three method-naming paths (RFC-004 binding / D20 overload resolution / bare-name inference) each produced their own names; the two operator-interface arms (Index → `Type.index`, Equal → `Type.equal`) independently assembled short type names, completely bypassing attribution (observed in fix round R1) | All five paths now produce the final call name through `method_dispatch_name` (one and only one copy of the criterion, D3)                                                                                                                                                                                            | `src/middle/core/ir_gen.rs:8318` (binding arm), `:8379` (resolution arm), `:8532` (bare-name arm), `:6444` (Index arm), `:7858` (**shared binary-interface arm**: Equal + Add/Subtract/Multiply/Divide/Modulo)                                                                                                                                                                                                                                            |

**Mechanism Discrimination (Discriminating Experiment, Before the Fix)**: issue #465 listed two
candidate mechanisms: (a) `type_bindings` is keyed by **short type name**, so same-name units
overwrite one another; (b) D20 resolution overrides `binding.function`. In this form, only (b)
applies: the entry unit's `type_bindings` is empty (the type body contains no inline external
binding), and both method calls hit `overload_resolutions`. The resolution names are the
definition-side bare names `Temp.tag`; `qualify_names` then uses the alias prefix to resolve the
bare `Temp` prefix to the bare-name provider a (`use a.{Temp}`), causing both call sites to land at
`a.Temp.tag`. This agrees with the issue's eight comparison groups (the winner is the bare-name
owner, independent of import order and of whether a has a call site). Mechanism (a) is inactive
(`seed_cross_file_types` records only the short names of inline bindings within a type body, not
method statements).

**Externally Observable Changes (Category C: old behavior was unsuccessful ⇒ fix it as part of the
same change)**: programs that previously produced a silently incorrect value now execute their
respective definition-unit implementations; cross-unit method calls without same-name types and the
two negative controls (different type names / two types in one file) remain unchanged. All criteria
are **value-level**: after changing the two units' method bodies to be distinguishable (`+1.0` /
`*10.0`), the assertions produce **`2.0` / `20.0`** (before the fix, the same fixture produced `2.0`
/ `3.0`; `CROSS_UNIT_A/B`, the two original unit implementations, were identical and therefore blind
to this defect).

**Criteria and Drift (Measurements for This Batch, Including the Fix Round)**: driver-level pinned
tests—**10** (inline alias / module-qualified construction / both operator arms / local-type
comparison / five of the eight comparison groups / negative control,
`src/driver/tests/method_dispatch.rs`), inspecting the **merged IR call name** at the structural
layer and the interpreter's global-slot value at the behavioral layer—plus integration value-level
assertions—**5** (`tests/integration/multifile.rs`, including the R1 operator form and the R2
`p.q` + alias form). Mutation verification: disabling the attribution in `method_dispatch_name`
(returning the original definition-side name) causes the two new same-name-type cases to fail (the
call names return to `a.Temp.tag` ×2), while the negative controls and 159 existing driver-level
cases do not regress; the patch is mechanically reversible (`git apply -R` reproduces the mutated
file byte-for-byte). Corpus baseline `tests/baselines/corpus-parity.jsonl` contains **356 rows with
zero drift**, and there are 216 IR snapshots with zero drift (this change only alters call-name
attribution, not IR shape, and adds no diagnostic code or warning).

**Fix Rounds (R1–R4, 2026-10-25; source = needs_revision from a cross-family review of the frozen
baseline)**:

- **R1, the two operator-interface arms**: Index / Equal dispatch previously assembled the call name
  from typecheck's **short type name** (`Temp.index` / `Temp.equal`) without attribution, causing
  the operator implementations of same-name types in two units to dispatch to the wrong unit.
  Measured on the frozen baseline: `u[0]` produced `3.0` and `u == t` produced `true` (expected
  `20.0` / `false`), and `check` exited with 0. Both arms now pass through `method_dispatch_name`
  (`:6444` / `:7858`), producing `20.0` / `false`; the **two local-type forms remain byte-for-byte
  unchanged** (pinned by `test_local_type_operator_arms_keep_own_dispatch` and the green
  `operator_index.yx` / `struct_equality.yx` corpus cases). The review round established that
  `:7858` is the **shared arm for all “interface-driven binary operations”** (Equal +
  Add/Subtract/Multiply/Divide/Modulo)—this section originally described it as only the “Equal arm,”
  understating its coverage; the text has now been corrected accordingly. The arithmetic families
  therefore follow the same corrected criterion, and `u + u` now produces `20.0` rather than `3.0`.
- **R2, single-point attribution and no second resolution**: the previous
  `constructor_written_type_name` extracted an **authoritative qualified name** (`p.q.Temp`), after
  which `imported_type_owner` resolved it again. The namespace-alias table rewrote the first segment
  (`use other as p` ⇒ `other.q.Temp`), causing the type-export determination to fail and fall back
  to the bare name (measured: `y = 3.0`). The constructor now performs only **pure syntactic
  extraction** (retaining the written name as written), `imported_type_owner` is the sole
  determination point, and an **exact authoritative qualified-name match returns immediately without
  further resolution** (`:1232`). Pinned by
  `test_multifile_qualified_module_key_not_rewritten_by_namespace_alias` (module key `p.q` + an
  unused `use other as p`; red before the fix, green afterward).
- **R3, documentation correction**: the section previously stated that `check` produced 0 errors / 0
  warnings, which does not agree with measurements—when the manifest lacks `version` (the form given
  in issue #465), `check` reports 2 `W1002` warnings. The text has now been rewritten to match the
  actual behavior and registered as residual boundary 4; silent degradation on manifest parse
  failure / missing fields is a separate defect (not addressed in this batch).
- **R4, pinning the eight comparison groups**: the driver adds five value-level cases—swapping the
  `use` order, swapping the method bodies between units (the value follows the unit: `10.0` /
  `3.0`), changing the **bare-name owner to the lexicographically later `z.yx`** (proving that the
  winner comes from the `use` surface, not sorting), calling only the alias side, and the same-file
  two-type negative control; together with the three existing cases, all eight comparison groups are
  now stored in the repository and can be rerun.
- Mutation verification (mechanically reversible; byte-for-byte identical after `git apply -R`): M1
  disables attribution in `method_dispatch_name` ⇒ seven cross-unit cases fail (including the two
  operator arms and four comparison groups), while the three local/negative-control cases do not
  regress; M3 restores the “old constructor + no exact match” behavior ⇒ the R2 case fails.
  **Residual Boundaries (registered; prefer omission over guessing; each is based on measurement or
  directly readable from the code)**:

1. **Fallback to the bare name when the written name is unavailable** (unchanged from before the
   fix): a receiver whose type flows in through a function return value, field, or container element
   has no written name to rely on—for example, after `u = make_b()` (a cross-unit function returning
   b's type), `u.tag()` still resolves by the bare-name prefix.
2. **Inline alias × same-name overload**: typecheck's candidate-injection surface remains narrowed
   according to the rule from 4.9.7 (rule ②: inline alias / module-qualified forms are not
   injected). Dispatch replaces only the type prefix and retains the resolution's `#k`
   mixed-instruction order—the cross-unit precision of `#k` is unchanged (residual ④ in 4.9.7).
3. **Cross-unit value binding passed through another binding**: an instance introduced by
   `use M.{v}` has its type recorded by the definition unit but no local written name, so it
   likewise falls back to the bare name.
4. **Manifest parse failure / missing required fields silently degrades** (measured): when
   `[package]` lacks `version` (the manifest form given in issue #465), `check` reports 2
   `W1002 unused type` warnings (`a.yx:1:1` / `b.yx:1:1`) even though both types are referenced by
   the entry point—these are **false positives**; adding `version` produces 0 warnings, and an
   invalid TOML manifest degrades in the same way. The root cause is the silent-degradation path in
   manifest parsing, not the dead-code criterion itself (registered; a separate WBS task / issue is
   to be handled by the team lead and is not fixed in this batch).
