---
title: 'RFC-029f: Compilation Target Roles and Import Surface Semantics'
status: 'Accepted'
author: 'Chen Xu'
created: '2026-09-12'
updated: '2026-09-13'
accepted: '2026-09-13'
issue: '#334'
---

# RFC-029f: Compilation Target Roles and Import Surface Semantics

## Abstract

> **Implementation correction (2026-10-08, RFC-029g)**: The `pub` keyword has been removed entirely
> by RFC-029g. All expressions in the body of this document such as “`pub` exemption / `pub` files /
> unused pub warning” should be read as “**top-level bindings + in-package reference pool**”: role
> classification (Script / Bin / Lib / Internal / Test) and import surface semantics remain
> unchanged; only the criterion for dead-code exemption changes (see
> [029g](029g-remove-pub-and-auto-bind.md) and the criteria used in
> `docs/src/reference/warning-code/warning-codes.md`).

Fulfill the extension slot of RFC-029 “sub-RFC planning” (029b–029e are occupied, so 029f comes
next): define a **compilation target role model** for source files (Script / Bin / Lib / Test /
Internal) and its **import surface semantics**—when there is no manifest, infer them from entry
reachability (zero configuration); when there is a manifest, declare them explicitly through `[lib]`
/ `[[bin]]` / `[exports]` (fields already defined by RFC-015). Four dangling areas of semantics are
filled in: how the cross-package import surface is calculated, the applicable scope of the pub
exemption for dead-code warnings (Decision B from #321), the relationship between `[exports]` and
Registry reachability, and the terminological disambiguation between RFC-014b `[binaries]`
(precompiled distribution artifacts) and source files with the bin role.

## Motivation

### Host and Boundary Basis

The entry selection and visibility decisions in RFC-029 (accepted) are the direct upstream of this
RFC:

> **Entry file selection** priority: 1. `[run].main` 2. the `path` of the first `[[bin]]` 3.
> `src/main.yx` (conventional default).— RFC-029 §Inter-package Cycles: Once More

> **Visibility**: nonexistent. Scope + distribution boundaries cover all scenarios; no new keyword
> is needed. — RFC-029 §Design Decision Record (2026-07-30)

The second statement is this RFC's design red line: import surface semantics **must not introduce a
new visibility keyword**. They can only be carried by the “distribution boundary” (file role +
declared export surface).

Four consumers have each encountered the same wall:

| Consumer                                           | Point of collision                                                                                                                                                                                                                                       |
| -------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| #321 Dead-code warnings                            | Decision B, “pub = a public interface is never reported,” exists because there was no role model to distinguish “genuinely public” from “appears public”; whether an unused pub in a bin should be reported (Option A) has remained unresolved           |
| RFC-014c Workspaces (under review, #113)           | When member packages import one another, how the import surface is calculated—`[exports]`, a single `[lib]` file, or every pub file—has **not a single semantic clause anywhere**                                                                        |
| RFC-015 Configuration system (accepted)            | The `[lib]` / `[[bin]]` / `[exports]` fields have been defined, but aside from RFC-029 consuming the entry priority, nobody has aligned the semantics of the remaining fields, especially the relationship between `[exports]` and Registry reachability |
| RFC-037 Packaging / RFC-029a Caching (draft, #293) | Both package artifact selection and the cache-unit boundary for incremental recompilation require a role model as a prerequisite                                                                                                                         |

### Current Problems

**Four semantic gaps** (inventory on 2026-09-12, confirmed by reading the full text of
014/014a/014b/014c/015/029):

1. **The cross-package import surface is undefined.** The example of mutual imports between member
   packages in 014c contains only a directory structure (`src/lib.yx`); there is no clause defining
   what `use utils.helper` can access. The current implicit rule is “all top-level pub files in the
   package”—which drags internal implementation files into the namespace and makes the dependency
   boundary effectively meaningless.
2. **The applicable scope of the dead-code pub exemption does not exist.** Decision B from #321,
   “pub is never reported,” is overly conservative in a single-file script (a script has no external
   consumers, so pub is meaningless), but overly permissive in the entry file of a multi-file
   project (a pub in the root file that nobody imports is dead code). Without a role model, the
   dividing line between Options A and B cannot be drawn.
3. **The three export concepts have not been aligned.** `[exports]` (015, path mapping), `[lib]`
   (015, single-file path), and Registry reachability (029, reachable from the entry along
   use)—there is no clause stating whether these form a containment relationship, an equality
   relationship, or are independent.
4. **Terminological collision.** RFC-014b's `[binaries]` refers to **precompiled distribution
   artifacts** (the .so/exe download-priority strategy), which is an entirely different layer from a
   “bin target (source-code role).” Once 014c is implemented, both will inevitably appear on the
   same screen. Failure to disambiguate them now will continue to cause misunderstandings.

**Empirical evidence: the conservatism of Decision B is already a practical pain point.** The #321
M2 retrospective records that the structural reason why the entire W1001–W1005 family is silent
under the default configuration is precisely that “all pub bindings are treated as public
interfaces”—a compensating behavior caused by the missing model, not the terminal state.

## Proposal

### Core Design: File Role Model

A **role is a file-level attribute** (consistent with the field granularity of 015: `[lib]` is a
single file, `[[bin]]` is a list of files, and `[exports]` is a file mapping), not a package-level
switch.

| Role         | Determination                                                                                                                                                        | Semantics                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| ------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Bin**      | The file targeted by `[run].main` / `[[bin]].path`; or, when there is no manifest, a file that contains `main` and is not used by any file                           | Program entry point. A **binding** named `main` must exist, and may be either a value or a function: a function `main` is called with zero arguments at entry, while a value `main` is evaluated during initialization and that evaluation constitutes execution (if the result is itself a function value, it is not called again). A missing `main` is a compilation error (E3020); top-level executable statements are forbidden. Its `pub` **may be reported as dead code** (there are no out-of-package consumers); `main` is a reachability root |
| **Lib**      | A file matched by an `[exports]` mapping; or `[lib].path`; or, when there is no manifest, a file referenced by another file's `use`                                  | Distribution boundary. Its `pub` is **exempt from dead-code reporting** (out-of-package consumers are not visible, so prefer under-reporting); symbols exported through it are visible across packages                                                                                                                                                                                                                                                                                                                                                 |
| **Test**     | A file matched by the RFC-036 test-file rules (the `tests/` directory, `*_test.yx`, and other existing conventions); there is **no** explicit `[[test]]` declaration | Test code. It does not participate in dead-code determination; its references count as reachability roots for the code under test                                                                                                                                                                                                                                                                                                                                                                                                                      |
| **Internal** | When the package has a manifest, a file that is neither on the export surface nor contains `main`                                                                    | Internal implementation. `pub` is exempt until phase 2 (then it is tightened according to reachability in the in-package use graph, with validation against Bin-role corpus as a prerequisite); out-of-package `use` is **unreachable**                                                                                                                                                                                                                                                                                                                |
| **Script**   | A file run directly on its own (`run foo.yx`)                                                                                                                        | **No entry-point concept.** Top-level statements (including binding initialization) constitute the program body in source order; `main` is an ordinary binding and is **not called automatically**—write `main()` if you want to run it. Registry contains only std                                                                                                                                                                                                                                                                                    |

**Determination priority**: explicit manifest declaration > entry-reachability inference > the 036
test convention. An explicit declaration always wins—inference is only the default when no
declaration exists.

### Entry Point Determination

The role model is not only used for dead-code analysis; it also **drives entry point
determination**. The entry rules for each role are:

| Role                       | Entry rule                                                                                                                                                                                                                                                                              |
| -------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Script** (no manifest)   | **No entry-point concept.** Top-level statements (including binding initialization) constitute the program body in source order; `main` is an ordinary binding and is **not called automatically**—write `main()` if you want to run it                                                 |
| **Bin** (manifest present) | A **binding** named `main` must exist (either a value or a function: a value `main` is evaluated during initialization, while a function `main` is called with zero arguments at entry); a missing `main` is a compilation error (E3020). Top-level executable statements are forbidden |
| **Lib / Internal / Test**  | `main` is not required (they are not entry points)                                                                                                                                                                                                                                      |

**The entry points of Script and Bin must be mutually exclusive**: “under Script, `main` is not
special” and “under Bin, `main` is the entry point” cannot both hold. If Script both executed
top-level statements and implicitly called `main`, a script that explicitly calls `main()` would run
twice (#356). Script therefore has exactly one execution entry point: its top-level statements.

**The distinction between role determination and entry point determination** (implementation note):
entry point determination uses the presence or absence of a manifest
(`find_project_root().is_some()`), not `roles::classify()`. The reason is that classify answers
“which files consume this file?” (serving dead-code analysis). A file that has a manifest but no
`main` is Internal under classify (it is reasonably not used by any other file), but when a user
runs it as the entry point, an entry point must still exist. These are two different questions and
therefore use different criteria.

**Execution form of a value `main`** (Decision from #388): functions are already values (unified at
the type level), and the entry point is determined by **the existence of the binding**, not by
callability—a value `main` and a function `main` differ only in their evaluation strategy. A value
`main` does not establish an entry call (#356 prevents double execution: initialization and entry
invocation are never combined), and has the same execution form as Script. Its evaluation is part of
the initialization sequence: bindings execute in global binding topological order, while independent
bindings follow source order. Therefore, moving Script top-level statements into `main = { ... }`
and then moving that into Bin is **not an order-preserving migration**—scattered statements execute
in source order, whereas binding initialization follows topological order. Initialization that must
occur first should be made an explicit dependency of `main`. A non-callable value `main` such as
`main: Int = 5` is legal but has no observable effect (analogous to an empty `fn main() {}` in
Rust).

See `docs/src/reference/language-spec/syntax.md` §3.11.

### Examples

```toml
# yaoxiang.toml (all fields were defined by RFC-015; this RFC adds no configuration surface)
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
pub unused_fn = (x: Int) => x    # ← W1001 may be reported: a bin has no out-of-package consumers
main: () -> Void = { ... }

# src/lib.yx (Lib role, on the export surface)
pub api_fn = ...                 # ← Never reported: out-of-package consumers cannot see it

# src/other.yx (Internal role, not on the export surface)
pub semi_api = ...               # ← Exempt until phase 2 (prefer under-reporting), then tightened by the use graph
```

### Import Surface Semantics

The resolution order for a cross-package `use pkg.x` is as follows (the first match takes effect):

1. An `[exports]` mapping exists → the import surface = the set of files listed by the mapping, and
   `x` must be among the top-level bindings in those files;
2. Otherwise, `[lib].path` exists → the import surface = that single file;
3. Otherwise (a dependency without a manifest, or a path dependency on a bare directory) →
   **preserve the current behavior**: every top-level file is importable (for compatibility with
   existing behavior; tightening will be considered separately).

**Workspace members**: an export surface is defined only by the member package's own manifest; the
workspace root must not override or extend a member's export surface—member self-containment is the
core design of 014c, and root overrides would break encapsulation.

Relationship to Registry reachability (aligned with 029): `[exports]` / `[lib]` define the **legal
starting-point set for cross-package resolution**; once the starting point has been determined,
Registry still expands according to 029's rule of “reachable from the entry along `use`”—internal
files used by an exported file enter the link, but do not create new cross-package starting points.

### Boundaries with Existing Plans

| Neighbor                                                     | Boundary                                                                                                                                                                                                                                                                                                                |
| ------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| RFC-029d “CLI `--entry` overrides the entry point” (planned) | 029f defines the role model; 029d consumes it—`--entry` is the CLI-level override mechanism for the Bin role                                                                                                                                                                                                            |
| RFC-014b `[binaries]`                                        | **Terminological disambiguation**: `[binaries]` refers to precompiled distribution artifacts (download-priority strategy), while `[[bin]]` declares a source-code role. The former exists in a dependency package's manifest, whereas the latter exists in the current package's manifest; they are not interchangeable |
| RFC-014c Workspaces                                          | The import surface for mutual imports between member packages = the resolution order in this RFC; workspace coordination mechanisms (members, shared lockfile) remain under 014c                                                                                                                                        |
| RFC-036 Testing                                              | Test-role determination directly adopts 036's existing file rules and introduces no new convention                                                                                                                                                                                                                      |
| #321 Dead code                                               | Decision B (pub is always exempt) is explicitly downgraded to “transitional semantics when there is no target”; reporting pub for the Bin role is the form in which this RFC implements Option A                                                                                                                        |

## Detailed Design

### Role Determination Algorithm (Inside the Orchestrator)

```
fn classify(files, manifest, entry_reach) -> Map<File, Role>:
    # 1. Explicit layer: manifest declarations take priority
    for f in manifest.exports.values():  role[f] = Lib
    if manifest.lib_path:                role[lib_path] = Lib
    for b in manifest.binaries:          role[b.path] = Bin
    if manifest.run_main:                role[run_main] = Bin

    # 2. Inference layer: undeclared files are classified by entry reachability
    for f in files where role[f] 未定:
        if f 被 ≥1 个非自身文件 use:      role[f] = Lib
        elif f 含 main 且无其他文件 use 它: role[f] = Bin
        else:                             role[f] = Internal

    # 3. Test layer: a match under the 036 rules → Test (overrides Lib/Internal, but not an explicit Bin)
    apply_rfc036_test_rules(files, &role)

    # Single-file direct run: bypass the entire model; behavior remains unchanged
    if no manifest and single_file:      all Script
```

### Compiler Changes

- `frontend/config.rs`: add the `[exports]` → role-table mapping to manifest parsing (field parsing
  already exists in 015).
- `frontend/module/orchestrator.rs`: insert the role classification stage before Registry
  construction; validate `use` targets according to the import-surface order for cross-package
  resolution, reporting the existing `module_not_found` family when a target is out of bounds (no
  new error code; add a “not on the export surface” hint to the message).
- `typecheck/passes/dead_code.rs`: change the entry point set from “main + all pub” to “main + pub
  in Lib-role files” (Bin may be reported immediately; Internal is exempt until phase 2; see the
  implementation strategy for Phase 2).
- LSP (RFC-017): filter “importable items” for completion/hover by the import surface; report the
  missing-`main` diagnostic only for Bin-role files.

### Runtime Behavior

No change. The role model is purely compile-time/parse-time semantics and does not affect the IR or
execution.

### Backward Compatibility

- Projects without a manifest: behavior remains completely unchanged (the inference layer restores
  the current behavior—a used file is Lib, and a file containing `main` is Bin). The only increment
  in dead-code warnings is in Bin files: an unused pub in a root file that nobody imports changes
  from silent to W1001. This is the expected behavior of Option A from #321, not a breaking change.
- Projects with a manifest: for declared `[exports]`, the import surface narrows from “all pub
  files” to the declared surface. **This is the only point at which behavior is narrowed**: code
  that depends on another package's internal files will begin to report `module_not_found`. Given
  that the package ecosystem has not yet been established (029: “there is currently no third-party
  package ecosystem”), the impact is zero; nevertheless, a one-minor-version transition period is
  provided (out-of-bounds access produces a warning before becoming an error).

## Trade-offs

### Advantages

- Zero new configuration surface: all fields were already defined by 015; this RFC only supplies
  semantics.
- Zero new keywords: the import surface is carried by the distribution boundary, matching 029's
  “visibility does not exist” decision.
- Four consumers (#321 / 014c / 029a / 037) are modeled once and no longer encounter the same wall
  independently.
- Zero cost for the no-manifest path: the zero-configuration nature of the scripting language is
  fully preserved.

### Disadvantages

- Role inference (a used file becomes Lib) classifies a utility file as Lib in the common shape
  where an entry file uses a utility file, thereby exempting its pub bindings. This is wider than
  the ideal granularity. It is an intentional preference for under-reporting; tightening would
  require call-direction analysis on the use graph and will not be done in the first version.
- Narrowing the import surface through `[exports]` is a breaking semantic tightening (even though a
  transition period is provided).

## Alternatives

| Alternative                              | Description                                                                             | Reason not adopted                                                                                                                                                                                                                                               |
| ---------------------------------------- | --------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Top-level RFC-040                        | Open a new top-level RFC number                                                         | Module-graph semantics (import surface / reachability / entry point) all belong to the 029 territory. Splitting them into a top-level RFC would produce two top-level documents for the same object; moreover, the 029b–029e slot pattern is already established |
| Incorporate into 014c                    | Make it a section of the workspace RFC                                                  | The dependency direction is reversed: the language layer (029x) defines semantics, while the package layer (014x) consumes them. 014c is still under review, and expanding its scope midway would impede its implementation                                      |
| Option A as written (the #321 menu)      | Require a `[[bin]]` / `[[lib]]` target in the manifest before dead-code semantics apply | Mandatory configuration would be introduced merely for a narrow slice (unused pub in a root file containing main); this RFC's inference layer obtains the same benefit with zero configuration                                                                   |
| Pure entry inference (no manifest layer) | Do not consume the 015 fields                                                           | `[exports]` is already an accepted 015 field, and the cross-package import surface cannot avoid it; leaving it undefined would create another gap for 014c                                                                                                       |

## Implementation Strategy

### Phased Implementation

- **Phase 1**: role classification + import surface resolution + ability to report pub for Bin
  (Internal/Test remain exempt)
- **Phase 2**: tighten Internal pub to “report when unreachable in the in-package use graph”—the
  prerequisite is validation of the Bin-role warning corpus after Phase 1 lands, with no
  false-positive regressions. Implement as soon as the prerequisite is met; no timetable is set.

### Dependencies

- Prerequisites: the RFC-029 orchestrator (implemented) and the RFC-015 manifest fields (defined).
- Dependents: #321 (reporting Bin pub), RFC-014c (import surface), RFC-029a (roles as cache-unit
  boundaries), RFC-037 (artifact selection), and RFC-029d (`--entry` override).
- Landing order with RFC-014c: before 014c is approved, 029f's import-surface order should be
  finalized, and 014c should consume it by reference, avoiding a mid-review expansion of scope.

### Risks

- Divergence between role inference and user intuition (the first trade-off above) → present all
  incremental behavior as warnings (non-blocking), which can be disabled at any time.
- Managing the transition period for the narrowing performed by `[exports]` → warn first during the
  next minor version.

## Open Questions

All three draft-stage questions were decided (2026-09-13, recorded in Appendix B and the body of the
document), with no unresolved items:

- [x] Should Internal pub be tightened? → **Decision**: tighten it in phase 2 according to
      reachability in the in-package use graph, with validation against the Bin-role corpus as a
      prerequisite (see implementation strategy Phase 2)
- [x] Explicit `[[test]]` declaration → **Decision**: do not introduce it; the Test role is
      determined solely by the 036 rules. If 036 later requires an explicit declaration, it will be
      extended by 036 itself
- [x] Can the workspace root override a member's export surface? → **Decision**: no; member
      self-containment is the core design of 014c, and root overrides break encapsulation

---

## Appendix B: Design Decision Record

| Decision                                              | Resolution                                                                                                          | Date       | Recorder | Basis                                                                                                                                   |
| ----------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- | ---------- | -------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| Number                                                | 029f (029b was reserved for hot reload, 029c was deleted, and 029d/029e are occupied)                               | 2026-09-12 | Chen Xu  | RFC-029 §Sub-RFC Planning                                                                                                               |
| Belongs to the 029 family rather than a top-level RFC | Yes                                                                                                                 | 2026-09-12 | Chen Xu  | Import surface / reachability / entry point all belong to module-graph semantics; 029 already consumes the `[[bin]]` entry priority     |
| Visibility keyword                                    | Do not introduce one                                                                                                | 2026-09-12 | Chen Xu  | Matches RFC-029's “visibility does not exist” decision; the import surface is carried by the distribution boundary                      |
| Status of Decision B from #321                        | Downgrade it to “transitional semantics when there is no target”                                                    | 2026-09-12 | Chen Xu  | Reporting pub for the Bin role is the form in which Option A from #321 is implemented                                                   |
| Role granularity                                      | File level                                                                                                          | 2026-09-12 | Chen Xu  | 015 field granularity (`[lib]` single file, `[[bin]]` list, `[exports]` mapping)                                                        |
| Disambiguation of `[binaries]`                        | 014b precompiled distribution artifacts ≠ `[[bin]]` source-code role                                                | 2026-09-12 | Chen Xu  | Two layers of concepts (download strategy vs. compilation role), fixed before 014c lands                                                |
| Tightening Internal pub                               | Tighten in phase 2 according to reachability in the in-package use graph; prerequisite = Bin-role corpus validation | 2026-09-13 | Chen Xu  | Gradual tightening that prefers under-reporting; corpus-driven validation avoids premature false positives                              |
| Explicit `[[test]]` declaration                       | Do not introduce it; Test is determined solely by the 036 rules                                                     | 2026-09-13 | Chen Xu  | No configuration surface is added until 036 has a need for it                                                                           |
| Workspace root overriding a member's export surface   | Not permitted                                                                                                       | 2026-09-13 | Chen Xu  | Member self-containment is the core design of 014c; root overrides break encapsulation                                                  |
| Is `main` called automatically under Script?          | **No**—top-level statements are the program, and `main()` must be explicit                                          | 2026-09-17 | Chen Xu  | Coexistence of the two rules would cause an explicit `main()` to run twice (#356); Script can have only one execution entry point       |
| Is the role model used for entry point determination? | **Yes** (previously it was used only for dead-code warnings)                                                        | 2026-09-17 | Chen Xu  | This RFC defines Bin's “`main` is a reachability root”; without connecting this to the entry point, that semantics would have no effect |

## Appendix C: Glossary

| Term                  | Definition                                                                                                                                  |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| Role                  | The compilation-target identity of a source file: Script / Bin / Lib / Test / Internal                                                      |
| Import Surface        | The set of files that are legal resolution starting points for cross-package `use`, declared or inferred through `[exports]` / `[lib]`      |
| Distribution Boundary | 029 terminology: the package's externally exposed scope; this RFC refines it from the implicit form (all pub files) into the import surface |
| Script Mode           | The bypass form used when a single file is run directly without a manifest; all behavior remains identical to the current behavior          |

## References

- RFC-029 Module Semantics (parent RFC: entry selection, visibility decision, sub-RFC planning)
- RFC-015 Configuration System (definition of the `[lib]` / `[[bin]]` / `[exports]` fields)
- RFC-014b Build System and Binary Distribution (`[binaries]`, the target of terminological
  disambiguation)
- RFC-014c Workspace Support (the first consumer of the import surface, under review)
- RFC-036 Test Framework (source of the Test-role rules)
- #321 M2 Independent Warning-Code Emission Channel (the source of Decision B and Option A)

## Known Defects: Implemented (2026-10-16, WBS 4.9 Round 1)

> The previous version (2026-10-09) listed the following three items as “three design defects in the
> reference pool.” All three have now been implemented one by one in WBS 4.9 Round 1 (commit
> `3863a9ff0`); `collect_project_refs` and `orchestrator.rs` were split and moved into
> `src/driver/{state,arms,helpers}.rs` with P4 4.2.9. All file:line references in the table below
> were re-obtained from the referenced revision.

### The Three Design Defects in the Reference Pool — Correction Status

| #   | Original Defect                                                        | Current State (2026-10-16)                                                                                                                                                                                                                                                                                                                                                                                                                                | Implementation (rev `3863a9ff0`)                                                                                                                 |
| --- | ---------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1   | The scan surface was the directory rather than the compilation surface | **The pool's roots = package targets**: `collect_package_refs` (`helpers.rs:481`) uses the `[[bin]]` / `[lib]` / `[exports]` / `[run].main` roots from `roles::ExplicitSurfaces` to invoke `discover_with_used` and obtain the compilation set; **it does not scan the directory for entry points**. When all roots fail to run, it falls back to the compilation set of the file under inspection itself (`collect_compilation_refs` / `helpers.rs:454`) | `src/driver/helpers.rs:454-512` (fallback path); `src/driver/helpers.rs:481-512` (manifest-declared surface paths)                               |
| 2   | The granularity was project-level bare names, with no module dimension | **Pool type = `HashMap<String, HashSet<String>>`**: keys = module keys, values = the set of reference names for that module; the consumer merges the entries of all modules other than the current unit and passes their union to `DeadCodeAnalyzer` (`arms.rs:603-609`). A module's own name (top-level definitions and local bindings, except for `use`-imported names) no longer exempts same-named definitions in other modules                       | `src/driver/arms.rs:603-609` (consumer-side `other_module_refs`); sole assembly entry point `src/driver/helpers.rs:429-449` (`collect_ref_pool`) |
| 3   | Script reused Lib's pool, violating single-file semantics              | **Script does not connect to the pool**: the `match us.role` in `arms.rs:611-623` assembles the pool only for `Bin` / `Internal` (`set_exempt_pub`) and `Lib` (`set_cross_file_refs`); Script and Test take the `_ => {}` default arm. The latter determines reachability solely from the within-file definition-use graph                                                                                                                                | `src/driver/arms.rs:611-623`                                                                                                                     |

**Follow-up Improvements 4 / 5 — Correction Status (2026-10-16, WBS 4.9 Round 2)**

> These two items were originally listed as “pending P4 4.9 implementation” in this section. Both
> have now been implemented in WBS 4.9 Round 2 (commit `e6cf1aeaa`, pushed on 2026-10-17). All
> file:line references in the table below were re-obtained from revision `e9675c9dd` (re-obtained
> during the 2026-10-17 Round 3 closure; the repair round and closure round after Round 2 changed
> the same anchor files, so the line numbers use the re-obtained values. Lesson from Round 1: a
> local hash before push is rewritten by `pull --rebase`).

| #   | Original Improvement                                                                                                 | Current State (2026-10-16)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | Implementation                                                                                                                                                                                                                                                                                                 |
| --- | -------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 4   | Refine consumption: distinguish “imported” from “actually used after import” (only the latter counts as consumption) | **Consumption criterion = “actually used after import”**: a consumption record means that a module uses `use` to bring the name n from module M into scope **and actually references n at the use site**, and is recorded only against M (`RefPool::consumed_from`). Merely appearing in a use item (with no use site), or another module referencing a name with the **same spelling**, does not count as consumption. The target module of an import is determined by the set of module keys discovered during the discovery phase (`resolve_import_module`; **no separate path-resolution mechanism is introduced**—the discovery-phase resolution result delivered in 4.6.1 is reused, see `39546020`). Namespace-form imports (`use lib` + `lib.foo()`) receive the same attribution | `src/driver/helpers.rs:442-447` (`RefPool`); `src/driver/helpers.rs:449-506` + `:511-524` (`collect_ref_pool` + `resolve_import_module`); `src/frontend/core/typecheck/passes/dead_code.rs:186-200` (`ModuleNames`), `:384` (`collect_module_names`), `:132-184` (`ImportFace` / `collect_import_face`)        |
| 5   | Separate the test pool: references from tests/ do not exempt production code (src/)                                  | **`FileRole::Test` does not contribute to the consumption surface**: Test-module references form a separate pool (Test itself does not participate in dead-code determination, so its references do not leak outward), and production-code definitions are kept alive only by consumption from production code. The role is determined at a single point by `roles::classify` (the RFC-036 test discovery rules)                                                                                                                                                                                                                                                                                                                                                                          | `src/driver/helpers.rs:463-466` (the Test short-circuit in `collect_ref_pool`); `src/driver/helpers.rs:707-708` (single-point role determination through `roles::classify`); `src/driver/arms.rs:633-635` (the Test skip shared by the fallback and declared-surface paths); role-to-pool wiring at `:661-672` |

**Residual Surface (registered; non-blocking)**: ① Field/member references (`obj.render()` has only
a short name) cannot be attributed at the AST level to the module that owns the receiver's type—when
a base name matches a namespace import, attribution follows the import; for module-qualified
construction `ns.T`, it is assigned to method attribution according to the **type-name surface**
(see §4.9.7 below); all other cases are conservatively counted by name dimension in
`RefPool::field_names` (`collect_ref_pool` in `src/driver/helpers.rs`); ② Method reachability is
narrowed **candidate by candidate and by defining unit** according to the D20 resolution artifacts
(`resolved_method_marks`)—unmatched sibling overloads **within the same unit** are truthfully
reported as W1005; uncalled definitions with the same name and order **across units** are likewise
truthfully reported in a reachable form (WBS 4.9.7 / issue #464, closed; mechanism and measured
behavior are described in the following section; the reference-producing side was delivered in
commit `b8b3cc3f6`). **Residual boundaries (registered; global coverage must not be claimed; all are
based on measured behavior)**: (a) Running `check <definition file>` alone (the file has no call
site, and the caller is not in the unit set) ⇒ there is no resolution entry ⇒ it falls back to the
pool's **name-level exemption** (the global flattening of `RefPool::field_names`) ⇒ **nothing is
reported**; (b) The same bare type name is brought into scope by multiple modules (the source
program is already ambiguous at the name layer, and the entry reports E5006 separately) ⇒ the entire
name is skipped in the candidate surface ⇒ there is no resolution entry ⇒ same-name, same-order
groups are not reported; (c) When the caller in this unit has made **any method call** on another
unit's type name (even if it does not match that side's definition), that side's definition is
merged in as a **possible owner** (the `typed_owners` rule ②) ⇒ nothing is reported; when both sides
merely construct values and make no method calls, `typed_owners` does not apply, and both sides are
truthfully reported as W1005 (t20 measured fixture: construction on both sides only ⇒ 2 W1005
entries); (d) Whole-module import `use M` does not bring a bare type name into scope, and the
candidate surface for the std module has not been integrated— neither form produces entries, so they
fall back to the name-level exemption. Fallback boundary: when a name group has no resolution record
in the current unit (for example, an ambiguous call point E1107 or a call form that does not go
through resolution), it falls back to short-name co-decision (in the direction of preferring
under-reporting)—therefore, do not claim that “uncalled methods are always reported truthfully.”

### Reference-Count Distortion (WBS 4.9.6 / issue #450) — Correction Status (2026-10-16)

> Original registration: **an overloaded method that was called and whose resolution took effect was
> still falsely reported as `W1005 Unused method`** (reference counting was distorted for overloaded
> calls). On 2026-10-16, the user's Decision A registered this as an independent level-three task
> and incorporated it into WBS 4.9 Round 2. This item belongs to class D (removing a false positive
> changes the diagnostic's external behavior) and has been authorized for implementation.

| Stage                   | Original Defect                                                                                                                                                                                                                                                                   | Current State                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Implementation                                                                                                                          |
| ----------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| Reachability probe      | The scan looked up references by **qualified name** (`Point.render`), whereas the reference collector obtains only the **short name** (`obj.render()` has only `render`) ⇒ a called method was always unreachable, producing false-positive W1005 in projects without a root path | Method call surfaces are determined by a **short-name probe**, then narrowed **candidate by candidate** using the **D20 resolution artifacts** (`resolved_methods` + `resolved_groups`; sibling overloads not matched by a resolution are still reported as W1005). Scope boundary: candidate-by-candidate determination holds **within the same unit**; **across units** (registry resolution), once the producing side (commit `b8b3cc3f6`) and driver-side attribution (this batch) have landed, it likewise holds candidate by candidate and by defining unit in a reachable form (see residual surface ② and §4.9.7 below) | `src/frontend/core/typecheck/passes/dead_code.rs:782-792` (probe); `:815-877` (candidate-by-candidate determination and exemption gate) |
| Definition-table keying | `all_defs` was keyed by qualified name ⇒ same-named overloads overwrote one another (only two overloads remained, and only one was reported)                                                                                                                                      | The definition table contains **one entry per definition**: methods are appended, while same-named non-method bindings are deduplicated by name, retaining the last occurrence (overloads are independent definitions, whereas a repeated top-level assignment is the same binding)                                                                                                                                                                                                                                                                                                                                             | `src/frontend/core/typecheck/passes/dead_code.rs:304-322` (`record_definition`), `:28-29` (fields)                                      |

**Reason not to change this in P3.5** (unchanged from the previous version): module-level resolution
requires `use` path resolution to be completed (`use a::foo` → `foo` comes from module `a`), which
belongs to the P4-stage contract and the unified Driver.

### Cross-Unit Method Reachability Attribution (WBS 4.9.7 / issue #464) — Correction Status (2026-10-17)

> Original registration (residual surface ②): method calls across units (registry resolution) do not
> produce D20 resolution entries ⇒ the name group falls back to the pool's **name-level** exemption
> ⇒ **uncalled methods with the same name and order across units are not reported**. This item was
> implemented through two separate changes: the **producing side** (cross-unit method calls produce
> resolution entries, commit `b8b3cc3f6`; its injection surface was filtered pair by pair by name
> attribution during the repair round, as shown below) and the **driver-side attribution** (provide
> reachability marks by defining unit and disable the name-level exemption). The file:line
> references in the table below are taken from the **driver-side attribution working tree**—commit
> `d5c00235a`; producing-side commit `b8b3cc3f6` (see above). All file:line references in this
> section are uniformly taken from revision `e9675c9dd` (re-obtained during the 2026-10-17 Round 3
> closure, covering the shifting of the injection surface range after the `d5c00235a` change was
> followed by repair-round commit `d5dd1a37b`); commit numbers always use their post-push true
> values (a local hash before push is rewritten by `pull --rebase`; see 09a).

| Stage                       | Original Defect                                                                                                                                                                                                                                                                        | Current State                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Implementation                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| --------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Resolution entry generation | All three candidate tables for a cross-unit call site are missing (`method_overloads` / `method_overload_ir_names` / `method_overload_param_names`) ⇒ the entire resolution section is skipped ⇒ **0 entries**                                                                         | Candidate groups (“candidate / mixed IR name / declaration-time formal-parameter name”) are packaged in matching order and passed across units through `ModuleInfo.method_overloads`; the registry merges them deterministically in ascending module-key order. The Typecheck arm takes candidates **pair by pair** according to the current unit's import face—only candidate groups are injected where the provider module of the key equals the module providing that **type-name prefix** in the current unit's scope (name ← module pairing, with no cross-pair combinations; an owned key slot is not merged in, and a same-name conflict skips the entire name) ⇒ at the same call site, `cands=1`, with 1 entry (`Temp.to`). **Repair-round measurement (F1)**: the previous “name set × module set” combination merged same-named type candidates from another module into the same key slot ⇒ under `use a.{Temp}` + `use b.{Other}` (where a also defines Other, b also defines Temp, and the methods have the same name and order), `check` reported 2×E1107 and exited with status 1 (a program that previously compiled successfully now failed); after pair-by-pair filtering, both `check` and `run` exit with status 0. The fixture is checked in as `tests/yaoxiang-multifile/ref-pool-cross-unit-name-owner/` | `src/frontend/core/typecheck/environment.rs:145` (`merge_from`: matching-order append at candidate-group granularity), `:336` (snapshot), `:384` (injection), `:399` (`append_overload_group`: the **sole** insertion point for the three parallel tables, validating equal lengths before insertion); `src/frontend/module/registry.rs:150` (`merge_method_overloads`); `src/frontend/module/mod.rs:155-157` (`ModuleInfo` field); `src/driver/arms.rs:344-358` (injection surface), `:1521-1579` (`visible_method_overloads`, the single injection point: name ← providing module pair by pair, same-name conflict skips the entire name, owned key slot is not merged in). **Injection-surface repair round (F1, `d5dd1a37b`)**: the previous “name set × module set” combination merged same-named type candidates from another module into the same key slot ⇒ under `use a.{Temp}` + `use b.{Other}` (where a also defines Other, b also defines Temp, and the methods have the same name and order), `check` reported 2×E1107 and exited with status 1 (a program that previously compiled successfully now failed); after pair-by-pair filtering, both `check` and `run` exit with status 0—the fixture is checked in as `tests/yaoxiang-multifile/ref-pool-cross-unit-name-owner/`, and the pinned test is `src/driver/tests/overload_injection.rs` (relocated in commit `c2f887843`) |
| Driver-side attribution     | Resolution entries were marked globally by **name** (a single set of `(base name, declaration order)`) ⇒ another unit's same-named, same-ordered definition was exempted by a call to its same-named sibling (cross-unit same-name, same-order cases were previously **not reported**) | Marks are attributed by **defining unit** (`MethodMarks { by_unit, groups }`): for an entry named `{Type}.{method}#k`, `{Type}` is the **bare name on the defining side** and does not carry a unit; the set of modules that can construct that name is calculated from the recording unit's **type-name surface**, and the entry is assigned to `by_unit` for each unit in that set; `groups` = the set of name groups for which an entry appeared (global)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | `src/driver/helpers.rs:569-573` (`MethodMarks`), `:591-619` (`typed_owners`: brace-list items + inline alias targets + module-qualified access `ns.T`, reusing the single points `collect_module_names` + `resolve_import_module`), `:621-654` (`resolved_method_marks`, a pure read through `OverloadResolutions::iter`); `src/driver/arms.rs:650-658` (retrieve `by_unit[unit_key]` per unit), `:649-660` (inject into analyzer)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| Decision surface            | A name group with resolution records still used the pool's **name-level exemption** (globally flattened short names)                                                                                                                                                                   | A name group with resolution records **does not use the name-level exemption**: definitions declared in the current unit that are matched are live, and unmatched definitions are truthfully reported as W1005 (`!marked_group` is the prerequisite of the exemption gate)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | `src/frontend/core/typecheck/passes/dead_code.rs:44` (field), `:252-257` (`set_resolved_groups`), `:262-267` (`method_group_has_marks`), `:782-795` (candidate-by-candidate reachability), `:815-877` (per-definition determination / exemption gate / W1005 emission)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |

**Externally Observable Changes (Class A: old behavior was unsuccessful ⇒ fix in passing, with a
separate commit record)**:

- **Cross-unit overload resolution previously did not take effect**: all three candidate tables were
  missing ⇒ it fell back to the cross-unit single-value binding table (`method_bindings`, where the
  last definition with a given name wins) ⇒ **a call to a same-named method with multiple candidates
  reported `error[E1002]`** (measured: `Expected type 'string', found type 'int64'`); after the fix,
  candidates are resolved according to the actual argument types, and the same fixture reports
  `Type check passed`. The criterion is implemented by the integration case
  `test_multifile_cross_unit_method_overload_resolves_by_argument_type`
  (`tests/integration/multifile.rs`, with same-named, same-ordered Int/String candidates and two
  calls that each match one)—turning off the injection surface makes this case fail with the same
  E1102.
- **Uncalled methods with the same name and order across units now begin to be truthfully reported
  as W1005**: a three-file fixture (`main.yx` imports the named type with `use a.{Temp}`, while
  `use b` only brings b into the compilation set) was measured—`a.Temp.to` has a call site and is
  not reported, `a.Temp.other` is reported, and **`b.Temp.to` is reported**; before the fix, the
  same fixture only reported `a.Temp.other` (b's definition had escaped through the pool's
  name-level exemption). The fixture is checked in as
  `tests/yaoxiang-multifile/ref-pool-cross-unit-method/` (reproducible from the CLI); the
  driver-level pinned test is `src/driver/tests/method_unused.rs` ::
  `test_cross_unit_uncalled_same_named_method_is_reported`.

**Residual Boundaries (registered; global coverage must not be claimed; each is based on measured
behavior)**:

1. Running `check <definition file>` alone (the file has no call site, and the caller is not in the
   unit set) ⇒ there is no resolution entry ⇒ it falls back to the name-level exemption ⇒ **nothing
   is reported** (in the direction of preferring under-reporting). Measured: with the same
   three-file fixture, `check b.yx` reports only the unused type W1002 and does not report
   `b.Temp.to`.
2. **Ambiguity at the name layer**: the same bare type name is brought into scope by multiple
   modules (`use a.{Temp}` + `use b.{Temp}`) ⇒ the entire name is skipped in the candidate surface ⇒
   there is no resolution entry ⇒ same-name, same-order groups fall back to the name-level exemption
   and are not reported; this entry separately reports `E5006 Duplicate import`. Measured
   comparison: importing only b in namespace form truthfully reports the same-named sibling
   candidates, proving that the criterion is not inert.
3. When the caller in this unit has made **any method call** on another unit's type name (even if it
   does not match that side's definition) ⇒ that side's definition is merged in as a **possible
   owner** (the `typed_owners` rule ②) ⇒ nothing is reported; when both sides merely construct
   values and make no method calls, `typed_owners` does not apply, and both sides are truthfully
   reported as W1005 (measured fixture `t20-b3-no-method-call`: construction on both sides only ⇒ 2
   W1005 entries)—in the direction of preferring under-reporting.
4. Inline aliases `use M.{T as X}` and module-qualified construction `ns.T` do not inject into the
   candidate surface (producing-side narrowing rule ②): the call-site key remains the defining
   side's `T` (measured: both `X(..).to()` and `ns.T(..).to()` are keyed as `T.to`) ⇒ the precision
   of `#k` for same-named types is bounded by “the set of modules from which the recording unit can
   construct the name”; in particular, these forms **cannot** cause a same-named, same-ordered
   definition on the other side to be declared dead (otherwise, a method that was genuinely called
   would be falsely reported).
5. Whole-module import `use M` does not bring a bare type name into scope (measured: writing
   `Temp(...)` reports E1001), and the **candidate surface for the std module has not been
   integrated** (its `ModuleInfo` is constructed through the std registration path)—neither form
   produces entries, so both fall back to the name-level exemption.

**Driver-side reading is a pure read**: `resolved_method_marks` is accessed through
`OverloadResolutions::iter` (without touching `get` and without being counted in the consumption
ledger) ⇒ it does not conceal the W1090 “consumption must be reachable” settlement; the driver-level
pinned test `test_resolved_method_marks_read_leaves_consumption_ledger_untouched` confirms that the
entry remains in the `unconsumed()` difference after the read.

**Closure Measurement Addendum (2026-10-17 Round 3; after injection-surface repair round
`d5dd1a37b` + relocation of the pinned test to `c2f887843`)**:

The injection surface filters by “name ← providing module” **pair by pair**, taking candidate groups
only when the provider module of the key equals the module providing that type-name prefix in the
current unit's scope. The single point is `visible_method_overloads`
(`src/driver/arms.rs:1521-1579`; production call site `src/driver/arms.rs:354`); downstream
insertion has only one point (`environment.rs:399` `append_overload_group`: matching-order append to
the three parallel tables + an equal-length debug assertion; the sole merge path between groups is
`merge_from`, `environment.rs:145`). Mutation verification: restoring cross-pair filtering flips the
driver-level pinned test red with 2×E1107/2×E1053, and all three assertions in the primary
acceptance fixture remain unchanged (proving that the fix did not regress the primary acceptance).
Independent validation / review used a frozen revision pair (before the fix `d5c00235a` / after the
fix `e9675c9dd`) for a dual-binary comparison: the diagnostic sets on both sides are identical for
the primary acceptance fixture `tests/yaoxiang-multifile/ref-pool-cross-unit-method/`; both negative
controls (no same-named type / same-named type but a different method name) are green on both sides;
the existing ambiguity behavior is identical on both sides when the same bare type name is brought
into scope by multiple modules (constructor / value dimensions); the reverse false-negative surface
is covered by truthfully reporting same-name, same-order **uncalled** definitions as W1005 (the fix
no longer swallows them).

**Criteria and Drift (measured in this batch, including the closure round)**: driver-level pinned
tests—5 attribution-round cases (primary acceptance / inline-alias false-positive defense /
module-qualified-name false-positive defense / bidirectional key scope / pure read,
`src/driver/tests/method_unused.rs`) + 3 injection-surface-round cases (pair-by-pair
name-attribution filtering / no false positives when method names differ / three-table equal-length
pairing, `src/driver/tests/overload_injection.rs`, relocated in commit `c2f887843`), plus 4 old #450
criteria with no regression; 204 IR snapshots with zero drift; corpus baseline increased from 346 →
347 lines (driver-side attribution round, new fixture `ref-pool-cross-unit-method/`) → 348 lines
(injection-surface repair round, new fixture `ref-pool-cross-unit-name-owner/`), with each change
adding only one `behavior` record and leaving every existing line byte-for-byte unchanged (the
baseline `tests/baselines/corpus-parity.jsonl` has 348 lines at `e9675c9dd`).

### Method Dispatch Attribution for Same-Named Types Across Units (WBS 4.9.8 / issue #465) — Correction Status (2026-10-25)

> Symptom: two compilation units each define a **same-named type** `Temp` and a same-named method
> `Temp.tag` (the implementations can be distinguished: a = `v + 1.0`, b = `v * 10.0`). The entry
> point imports a by bare name (`use a.{Temp}`), then imports b using an **inline alias**
> (`use b.{Temp as TempB}`) or **module-qualified construction** (`b.Temp(..)`). Both constructions
> are correct (the field values are correct), but `u.tag()` is dispatched to a's implementation;
> `run` exits with status 1 (assertion failure)— a silently incorrect value (D59 class C). The
> warnings produced by `check` depend on whether the manifest is readable: the manifest form in the
> issue (`[package]` contains only `name` and lacks `version`) actually produces **2
> `W1002 unused type` warnings** (`a.yx:1:1` / `b.yx:1:1`); adding `version` produces 0 warnings.
> Manifest parsing failure / missing required fields causes a **silent fallback** (residual boundary
> 4), which is unrelated to the dead-code criterion itself.

| Stage                       | Original Defect                                                                                                                                                                                                                                                                                                              | Current State                                                                                                                                                                                                                                                                                                                 | Implementation                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| --------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source of the dispatch name | The dispatch name = the **bare type name on the defining side** + the method name (`Temp.tag`). `MonoType` in typecheck retains only the defining side's bare name (`Temp`) for imported types, so same-named types in two units are indistinguishable at the type layer                                                     | The dispatch name is first attributed to the defining unit according to the **type name written by the receiver**: `Temp.tag` → `b.Temp.tag` (written name → import face → qualified name of the defining unit). **Method calls and operator interfaces (Index / Equal) use the same criterion**                              | `src/middle/core/ir_gen.rs:1265` (`method_dispatch_name`, the **sole criterion**), `:1232` (`imported_type_owner`: `use_aliases` + `Resolver::resolve_namespace` + the registry's single source, `ExportKind::Type`; **return immediately on an exact authoritative-qualified-name match—do not resolve again**), `:1188` (`constructor_written_type_name`, pure syntactic extraction), `:1169` (`written_type_name_of`, annotation written name) |
| Recording the written name  | This concept did not exist: `ir_gen` dispatched only by the bare name in `MonoType`                                                                                                                                                                                                                                          | Local variables are recorded in symbol-table entries (`SymbolEntry.written_type`, with the scope carried by the symbol table); top-level bindings are recorded in a per-unit table (`global_written_types`; in Script form, `t = Temp(1.0)` is a global slot); formal-parameter annotations are recorded along the same chain | `src/middle/core/ir_gen.rs:1115` (`note_local_written_type`), `:1129` (`note_param_written_types`), `:1159` (`binding_written_type`: local before top-level), `:109` (`SymbolEntry.written_type` field), `:3255` (top-level registration point)                                                                                                                                                                                                   |
| Five production paths       | The three method-naming paths (RFC-004 binding / D20 overload resolution / bare-name inference) each produced their own names; the two operator-interface arms (Index → `Type.index`, Equal → `Type.equal`) constructed the short type name separately and completely bypassed attribution (measured during repair round R1) | All five paths use `method_dispatch_name` to produce the final call name (one criterion only, D3)                                                                                                                                                                                                                             | `src/middle/core/ir_gen.rs:8318` (binding arm), `:8379` (resolution arm), `:8532` (bare-name arm), `:6444` (Index arm), `:7858` (**shared binary-interface arm**: Equal + Add/Subtract/Multiply/Divide/Modulo)                                                                                                                                                                                                                                    |

**Mechanism Discrimination (discriminating experiment, performed before the fix)**: issue #465
listed two candidate mechanisms—(a) `type_bindings` is keyed by **short type name**, so same-named
units overwrite one another; (b) D20 resolution overrides `binding.function`. Only (b) is active in
this form: the entry unit's `type_bindings` is empty (the type bodies contain no inline external
bindings), both method calls match `overload_resolutions`, and the resolution name is the **defining
side's bare name** `Temp.tag`; `qualify_names` then resolves the bare-name prefix `Temp` to the
**bare-name provider** a (`use a.{Temp}`), so both call sites land on `a.Temp.tag`—consistent with
the issue's eight comparison groups (the winner = the owner of the bare name; independent of import
order and of whether there is a call site on a's side). Mechanism (a) is inactive in this form
(`seed_cross_file_types` records only the short names of bindings **inline within type bodies**,
unrelated to the form of the method statements).

**Externally Observable Change (Class C: old behavior was unsuccessful ⇒ fix in passing)**: before
the fix, a program that silently produced the wrong value now executes the implementation from each
respective defining unit; cross-unit method calls without same-named types and the two
negative-control groups (different type names / two types in the same file) remain unchanged. All
criteria are **value-level**: after changing the two method bodies to be distinguishable (`+1.0` /
`*10.0`), the assertions are `2.0` / `20.0`—before the fix, the same fixture produced `2.0` / `3.0`
(`CROSS_UNIT_A/B` previously had identical implementations in both units and was therefore blind to
this defect).

**Criteria and Drift (measured in this batch, including the repair round)**: driver-level pinned
tests—**10 cases** (inline alias / module-qualified construction / both operator arms / local-type
comparison / five of the eight comparison groups / negative control,
`src/driver/tests/method_dispatch.rs`, with the structural layer reading the **merged IR call name**
and the behavioral layer reading the value in the interpreter's global slot) + 5 integration
value-level assertions (`tests/integration/multifile.rs`, including the R1 operator form and the R2
`p.q` + alias form); mutation verification: disabling `method_dispatch_name` attribution (returning
the defining-side name unchanged) flips the two new same-named-type cases red (the call name returns
to `a.Temp.tag` ×2), while the negative controls and all 159 existing driver-level cases show no
regression; the patch is mechanically reversible (after `git apply -R`, it is byte-for-byte
identical to the mutated file). The corpus baseline `tests/baselines/corpus-parity.jsonl` has **356
lines with zero drift**, and the 216 IR snapshots have zero drift (this change only alters call-name
attribution, does not alter the IR shape, and does not add diagnostic codes / warnings).

**Repair Rounds (R1–R4, 2026-10-25; source = needs_revision from an independent-family review of the
frozen baseline)**:

- **R1, the two operator-interface arms**: the Index / Equal dispatch previously constructed the
  call name using the **short type name** supplied by typecheck (`Temp.index` / `Temp.equal`)
  without passing through attribution ⇒ operator implementations for same-named types across units
  were dispatched to the wrong unit. Measured against the frozen baseline: `u[0]` produced `3.0` and
  `u == t` produced `true` (the expected values are `20.0` / `false`), and `check` exited with
  status 0. Both arms now pass through `method_dispatch_name` (`:6444` / `:7858`), producing `20.0`
  / `false` after the fix; the **two local-type forms remain byte-for-byte unchanged** (pinned by
  `test_local_type_operator_arms_keep_own_dispatch` + the corpus fixtures `operator_index.yx` /
  `struct_equality.yx`, all green). During the review round, measurement showed that `:7858` is the
  **shared arm for all “interface-driven binary operations”** (Equal +
  Add/Subtract/Multiply/Divide/Modulo)—this section originally described it as the “Equal arm,”
  which understated its coverage; the record has been corrected accordingly.
- **R2, single-point attribution and no secondary resolution**: the previous
  `constructor_written_type_name` produced an **authoritative qualified name** (`p.q.Temp`), and
  `imported_type_owner` then passed it through `resolve_namespace` again; the first segment was
  rewritten by the namespace alias table (`use other as p` ⇒ `other.q.Temp`) ⇒ type-export
  determination failed ⇒ it fell back to the bare name (measured: `y = 3.0`). The constructor now
  performs **pure syntactic extraction** (the written name is preserved unchanged),
  `imported_type_owner` is the sole determination point, and an **exact authoritative qualified-name
  match returns immediately without further resolution** (`:1232`). Pinned by
  `test_multifile_qualified_module_key_not_rewritten_by_namespace_alias` (module key `p.q` + an
  unused `use other as p`; red before the fix, green after).
- **R3, documentation correction**: the previous statement in this section that “`check` produces 0
  errors / 0 warnings” did not match the measured behavior—when the issue's manifest lacks
  `version`, `check` reports 2 `W1002` entries. It has been rewritten to reflect the actual behavior
  and registered as residual boundary 4; the **silent fallback** in the manifest-parsing-failure /
  missing-field path is itself another defect (not fixed in this batch).
- **R4, pinned tests for the eight comparison groups**: the driver now includes five new value-level
  cases—swapping the `use` order, swapping the two units' method bodies (the value follows its unit:
  `10.0` / `3.0`), **changing the bare-name owner to the lexicographically later `z.yx`** (proving
  that the winner comes from the `use` face rather than sorting order), calling only the alias side,
  and using a negative control with two types in the same file; together with the original three,
  all eight comparison groups are now checked in and reproducible.
- **Mutation verification** (mechanically reversible; byte-for-byte identical after `git apply -R`):
  M1, disabling `method_dispatch_name` attribution ⇒ seven cross-unit cases flip red (including both
  operator arms and four comparison groups), while the three local-type / negative-control cases
  remain green; M3, restoring the “old constructor + no exact match” ⇒ R2's case flips red.

**Residual Boundaries (registered; prefer omissions over guesses; each is either measured or
directly readable from the code)**:

1. **Fallback to the bare name when the written name is unavailable** (unchanged from before the
   fix): a receiver whose type flows in through a function return value, a field, or a container
   element has no written name to rely on—for example, `u = make_b()` (which returns a cross-unit
   function's b-unit type) followed by `u.tag()` still resolves by the bare-name prefix.
2. **Inline alias × same-named overload**: typecheck's candidate injection surface still follows the
   4.9.7 narrowing rules (rule ②: inline-alias / module-qualified forms are not injected); dispatch
   only replaces the type prefix and retains the resolution's `#k` mixed-IR order—the cross-unit
   precision of `#k` is therefore unchanged (same as residual surface ④ in 4.9.7).
3. **Cross-unit value binding passed through another binding**: an instance imported by `use M.{v}`
   (whose type is recorded by the defining unit) likewise has no local written name and falls back
   to the bare name.
4. **Silent fallback when manifest parsing fails / required fields are missing** (measured): when
   `[package]` lacks `version` (the manifest form given in issue #465), `check` reports 2
   `W1002 unused type` entries (`a.yx:1:1` / `b.yx:1:1`)—both types are referenced by the entry
   point and are therefore **false positives**; adding `version` produces 0 entries, and an
   **illegal TOML** manifest produces the same silent fallback. The root cause is the
   silent-fallback path in manifest parsing, not the dead-code criterion itself (registered; to be
   handled by a separate WBS task / issue, and not fixed in this batch).
