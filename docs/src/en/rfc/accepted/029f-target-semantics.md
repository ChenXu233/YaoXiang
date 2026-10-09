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

> **Landing revision (2026-10-08, RFC-029g)**: The `pub` keyword has been entirely removed by
> RFC-029g. All "pub exemption / pub files / unused pub warnings" wording in the body of this
> document should be read as "**top-level bindings + in-package reference pool**": role
> classification (Script / Bin / Lib / Internal / Test) and import surface semantics remain
> unchanged; the change is only in the dead-code exemption criteria (see
> [029g](029g-remove-pub-and-auto-bind.md) and the criteria in
> `docs/src/reference/warning-code/warning-codes.md`).

Fulfilling the extension slot promised in RFC-029's "Sub-RFC Plan" (slots 029b–029e are taken, 029f
follows): define a **compilation target role model** for source files (the five classes Script / Bin
/ Lib / Test / Internal) along with its **import surface semantics**—when there is no manifest,
infer from entry reachability (zero configuration); when there is a manifest, declare explicitly via
`[lib]` / `[[bin]]` / `[exports]` (fields already defined by RFC-015). This fills four outstanding
semantic gaps: what governs cross-package import surfaces, the applicability scope of the dead-code
warning's `pub` exemption (case B decided in #321), the relationship between `[exports]` and
Registry reachability, and the naming disambiguation between RFC-014b's `[binaries]` (precompiled
distribution artifacts) and the bin-role source file.

## Motivation

### Host and Boundary Basis

RFC-029 (accepted) and its entry-selection and visibility decisions are the direct upstream of this
RFC:

> **Entry file selection** priority: 1. `[run].main` 2. The first `[[bin]]`'s `path` 3.
> `src/main.yx` (convention default). — RFC-029 §Inter-package cycles: revisit

> **Visibility**: does not exist. Scope + distribution boundary cover all scenarios; no new keyword
> is needed. — RFC-029 §Decision Log (2026-07-30)

The second quote is the design red line of this RFC: the import surface semantics **must not
introduce any new visibility keyword**; it can only be carried by "distribution boundary" (file
role + declared export surface).

The four consumers each hit the same wall:

| Consumer                                         | Where it hits the wall                                                                                                                                                                                                                                    |
| ------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| #321 Dead-code warnings                          | Case B "pub = external interface, never reported" exists because there is no role model to distinguish "truly external" from "looks external"; whether unused pub inside a bin should be reported (option A) has been on hold                             |
| RFC-014c Workspace (under review, #113)          | When member packages `use` each other, what governs the import surface—`[exports]`, the `[lib]` single file, or all pub files—**there is not a single semantic clause in the entire document**                                                            |
| RFC-015 Configuration system (accepted)          | The `[lib]` / `[[bin]]` / `[exports]` fields are defined, but aside from RFC-029 consuming the entry priority, the semantics of the remaining fields (especially the relationship between `[exports]` and Registry reachability) are not aligned anywhere |
| RFC-037 Packaging / RFC-029a Cache (draft, #293) | Packaging artifact selection, cache-unit boundary for incremental recompilation—both need the role model as a prerequisite                                                                                                                                |

### Current Problem

**Four semantic gaps** (inventory taken 2026-09-12, confirmed by reviewing the full text of
014/014a/014b/014c/015/029):

1. **No definition of cross-package import surface**. The 014c example of member packages referring
   to each other only has a directory layout (`src/lib.yx`); nothing in the spec says what
   `use utils.helper` can resolve to. The current implicit rule is "all top-level pub files in the
   package"—internal implementation files are dragged into the namespace, making dependency
   boundaries meaningless.
2. **The dead-code pub exemption has no applicability scope**. Case B from #321, "pub is never
   reported", is over-conservative in a single-file script (a script has no external consumer, so
   pub is meaningless) and over-permissive in the entry file of a multi-file project (a pub on the
   root file that no one imports is dead code). Without a role model, the dividing line between
   cases A and B cannot be drawn.
3. **Three export concepts are unaligned**. `[exports]` (015, path mapping), `[lib]` (015,
   single-file path), Registry reachability (029, reachable from entry via `use`)—whether the three
   are in a containment relation, an equality relation, or each independent is not specified.
4. **Naming collision**. RFC-014b's `[binaries]` is a **precompiled distribution artifact** (.so/exe
   download-priority policy), which is an entirely different layer from "the bin target (source-code
   role)". Once 014c lands, the two will inevitably appear side by side; if we don't disambiguate
   now, the confusion will persist.

**Empirical evidence: the conservatism of case B is already a real pain point**. #321 M2
retrospective records that the structural reason W1001–W1005 are all silent under default
configuration is precisely "all pub are treated as external interfaces"—this is a compensation for a
missing model, not a terminal state.

## Proposal

### Core Design: File Role Model

**Role is a file-level attribute** (matching the granularity of 015's fields: `[lib]` is a single
file, `[[bin]]` is a list of files, `[exports]` is a file mapping), not a package-level switch.

| Role         | Determination                                                                                                                                      | Semantics                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| ------------ | -------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Bin**      | The file pointed to by `[run].main` / `[[bin]].path`; or (without a manifest) the file containing `main` that is not `use`d by any other file      | Program entry. Requires a **binding** named `main`, either a value or a function: a function main is called with zero arguments at entry; a value main is evaluated at initialization (and the result, even if it is a function value, is not called again). Missing `main` is a compile error (E3020); top-level executable statements are not allowed. Its `pub` **may be reported as dead code** (no out-of-package consumer); `main` is a reachability root |
| **Lib**      | A file hit by an `[exports]` mapping; or `[lib].path`; or (without a manifest) a file `use`d by some other file                                    | Distribution boundary. Its `pub` **is exempt from dead-code** (out-of-package consumer is invisible, prefer under-reporting); symbols exported through it are visible across packages                                                                                                                                                                                                                                                                           |
| **Test**     | Files matched by RFC-036 test file rules (`tests/` directory, `*_test.yx`, and other existing conventions); **no** `[[test]]` explicit declaration | Test code. Does not participate in dead-code determination; its references count as reachability roots of code under test                                                                                                                                                                                                                                                                                                                                       |
| **Internal** | When the package has a manifest: a file that is neither in the export surface nor contains main                                                    | In-package implementation. `pub` exemption extended to a phase 2 (tightened by in-package use-graph reachability, gated on Bin-role corpus validation); out-of-package `use` **is unreachable**                                                                                                                                                                                                                                                                 |
| **Script**   | A file run directly as a single file (`run foo.yx`)                                                                                                | **No entry concept**. Top-level statements (including binding initialization) form the program body in source order; `main` is an ordinary binding, **not auto-called**—to run, write `main()`. The Registry contains only std                                                                                                                                                                                                                                  |

**Determination priority**: explicit manifest declaration > entry-reachability inference > 036 test
convention. Explicit declaration always wins—inference is only the default when there is no
declaration.

### Entry Determination

The role model is used not only for dead-code analysis, but also to **drive entry determination**.
Entry rules per role:

| Role                      | Entry rule                                                                                                                                                                                                                                                  |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Script** (no manifest)  | **No entry concept**. Top-level statements (including binding initialization) form the program body in source order; `main` is an ordinary binding, **not auto-called**—to run, write `main()`                                                              |
| **Bin** (with manifest)   | Requires a **binding** named `main` (either value or function: value main is evaluated at initialization, function main is called with zero arguments at entry); missing `main` is a compile error (E3020). Top-level executable statements are not allowed |
| **Lib / Internal / Test** | `main` is not required (they are not entries)                                                                                                                                                                                                               |

**Script and Bin entry rules must be mutually exclusive**: "in Script, `main` is not special" and
"in Bin, `main` is the entry" cannot both hold—if Script both executes top-level statements and
implicitly calls `main`, a script that explicitly writes `main()` would run twice (#356). Therefore
Script has only one execution entry: top-level statements.

**Difference between role determination and entry determination** (implementation note): entry
determination uses "whether a manifest exists" (`find_project_root().is_some()`) rather than
`roles::classify()`. The reason is that classify answers "who consumes this file" (serving dead-code
analysis); a file with a manifest but no `main` is Internal under classify (it is not `use`d by
other files, which is reasonable), yet the user runs it as an entry— in that case there must be an
entry. The two questions differ, so the criteria differ.

**Execution form of value `main`** (decided in #388): a function is already a value (unified at the
type layer), so entry is judged by **binding existence** rather than callability—a value main and a
function main only differ in evaluation strategy. A value main has no entry invocation (#356
prevents double running: initialization and entry invocation never stack); it shares the same
execution form as Script. Its evaluation is part of the initialization sequence: executed in the
global binding topological order, with independent bindings following source order. Therefore,
wrapping Script top-level statements in `main = { ... }` to migrate into a Bin is **not an
order-preserving migration**— scattered statements execute in source order, while binding
initialization follows topological order; initialization that needs to come first should make `main`
explicitly depend on it. An uncallable value main like `main: Int = 5` is a legal program with no
observable effect (analogous to Rust's empty `fn main() {}`).

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
"./internal-helper" = "src/helper.yx"   # ← what determines cross-package visibility is this, not pub
```

```yaoxiang
# src/cli.yx (Bin role)
pub unused_fn = (x: Int) => x    # ← W1001 may be reported: the bin has no out-of-package consumer
main: () -> Void = { ... }

# src/lib.yx (Lib role, on the export surface)
pub api_fn = ...                 # ← never reported: out-of-package consumer is invisible

# src/other.yx (Internal role, not on the export surface)
pub semi_api = ...               # ← exemption extended to phase 2 (prefer under-reporting); later tightened by the use graph
```

### Import Surface Semantics

Resolution order of cross-package `use pkg.x` (the first match takes effect):

1. If an `[exports]` mapping exists → the import surface = the set of files listed in the mapping;
   `x` must be among the top-level bindings of those files;
2. Otherwise, if `[lib].path` exists → the import surface = that single file;
3. If neither exists (no-manifest dependency, raw directory dependency) → **status quo**: all
   top-level files are importable (compatibility with existing behavior; tightening is a separate
   matter).

**Workspace members**: the export surface is defined solely by each member package's own manifest;
the workspace root must not override or extend the member export surface—member self-containment is
the core design of 014c, and root override would break encapsulation.

Relationship to Registry reachability (aligned with 029): `[exports]`/`[lib]` define the **set of
valid starting points for cross-package resolution**; once the starting points are fixed, the
Registry still expands per 029's "reachable from the entry along `use`"—internal files that an
exported file itself `use`s join via linking, but do not create new cross-package starting points.

### Boundary With Existing Plans

| Neighbor                                           | Boundary                                                                                                                                                                                                                                                                         |
| -------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| RFC-029d "CLI `--entry` overrides entry" (planned) | 029f defines the role model; 029d consumes it—`--entry` is the CLI-level override means for the Bin role                                                                                                                                                                         |
| RFC-014b `[binaries]`                              | **Naming disambiguation**: `[binaries]` is a precompiled distribution artifact (download-priority policy); `[[bin]]` is a source-code role declaration. The former lives in a dependency package's manifest, the latter in this package's manifest; they are not interchangeable |
| RFC-014c Workspace                                 | Cross-member import surface = the resolution order in this RFC; workspace coordination mechanisms (members, shared lockfile) still belong to 014c                                                                                                                                |
| RFC-036 Testing                                    | Test role determination directly reuses 036's existing file rules, no new convention added                                                                                                                                                                                       |
| #321 Dead code                                     | Case B (pub always exempt) is explicitly downgraded to "transitional semantics when no target exists"; the Bin role's pub being reportable is this RFC's realization of option A                                                                                                 |

## Detailed Design

### Role Determination Algorithm (Inside the Orchestrator)

```
fn classify(files, manifest, entry_reach) -> Map<File, Role>:
    # 1. Explicit layer: manifest declarations take priority
    for f in manifest.exports.values():  role[f] = Lib
    if manifest.lib_path:                role[lib_path] = Lib
    for b in manifest.binaries:          role[b.path] = Bin
    if manifest.run_main:                role[run_main] = Bin

    # 2. Inference layer: undetermined files fall back to entry-reachability
    for f in files where role[f] undetermined:
        if f is use'd by ≥1 file other than itself:  role[f] = Lib
        elif f contains main and no other file uses it: role[f] = Bin
        else:                             role[f] = Internal

    # 3. Test layer: 036 rule hit → Test (overrides Lib/Internal, not explicit Bin)
    apply_rfc036_test_rules(files, &role)

    # Single-file direct run: the whole model is bypassed, behavior unchanged
    if no manifest and single_file:      all Script
```

### Compiler Changes

- `frontend/config.rs`: manifest parsing adds a mapping from `[exports]` to the role table (field
  parsing already exists in 015).
- `frontend/module/orchestrator.rs`: insert a role-classification stage before Registry
  construction; cross-package resolution validates `use` targets against the import surface order,
  out-of-bounds reports the existing `module_not_found` family (no new error code, message adds the
  hint "not on the export surface").
- `typecheck/passes/dead_code.rs`: the entry-point set changes from "main + all pub" to "main + pub
  of Lib-role files" (Bin is immediately reportable; Internal is exempt until phase 2; see
  Implementation Strategy for Phase 2).
- LSP (RFC-017): completion/hover's "importable items" is filtered by the import surface; the
  missing-`main` diagnostic is reported only for Bin-role files.

### Runtime Behavior

No change. The role model is purely a compile-time/parse-time semantic, and does not affect IR or
execution.

### Backward Compatibility

- Projects with no manifest: behavior is entirely unchanged (the inference layer reproduces the
  current state—what's `use`d is Lib, what contains `main` is Bin). The dead-code warning's only
  delta is in Bin files: an unused pub on the root file that no one imports goes from silent to
  W1001— this is the expected behavior of #321 option A, not a breakage.
- Projects with a manifest: once `[exports]` is declared, the import surface narrows from "all pub
  files" to the declared surface. **This is the only behavior-narrowing point**: code that depends
  on another package's internal files will start reporting `module_not_found`. Given that the
  package ecosystem is not yet established (029: "currently no third-party package ecosystem"), the
  breakage surface is zero; nevertheless, a one-minor-version transition period is reserved
  (out-of-bounds first warns, then errors).

## Trade-offs

### Advantages

- Zero new configuration surface: all fields are already defined by 015; this RFC only adds
  semantics.
- Zero new keyword: the import surface is carried by the distribution boundary, isomorphic to 029's
  "visibility does not exist" decision.
- Four consumers (#321 / 014c / 029a / 037) are modeled once, no longer each hitting their own wall.
- The no-manifest path is zero-cost: the zero-configuration gene of the scripting language is fully
  preserved.

### Disadvantages

- Role inference (anything `use`d becomes Lib) classifies utility files as Lib in the common shape
  of "entry file uses utility files", granting them pub exemption—coarser than the ideal
  granularity. This is a deliberate under-reporting choice; tightening requires call-direction
  analysis on the use graph, which is not done in v1.
- Narrowing the import surface via `[exports]` is a breaking semantic tightening (despite the
  transition period).

## Alternatives

| Alternative                              | Description                                                                                      | Reason not adopted                                                                                                                                                                                                                          |
| ---------------------------------------- | ------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Top-level RFC-040                        | Open a new top-level number                                                                      | Module-graph semantics (import surface / reachability / entry) all live in 029's territory; splitting into a top-level RFC would create two top-level documents for the same object; also the 029b–029e slot pattern is already established |
| Attach to 014c                           | As a section of workspace                                                                        | The dependency direction is reversed: the language layer (029x) defines semantics, the package layer (014x) consumes them. 014c is still under review; expanding scope mid-review would delay its landing                                   |
| Option A as-is (#321 menu)               | Dead-code semantics only for projects that declare `[[bin]]`/`[[lib]]` targets in their manifest | Introduces mandatory configuration for a narrow slice (unused pub on a `main`-containing root file); this RFC's inference layer achieves the same payoff with zero configuration                                                            |
| Pure entry inference (no manifest layer) | Don't consume 015 fields                                                                         | `[exports]` is already an accepted 015 field; cross-package import surface cannot bypass it; not defining it equals leaving a gap for 014c                                                                                                  |

## Implementation Strategy

### Phased

- **Phase 1**: role classification + import surface resolution + Bin pub reportable (Internal/Test
  remain exempt)
- **Phase 2**: Internal pub tightened to "reportable when unreachable on the in-package use
  graph"—prerequisite is that after Phase 1 lands, Bin-role warning corpus validation shows no false
  positives; triggered to proceed, no schedule set

### Dependencies

- Prerequisites: RFC-029 orchestrator (landed), RFC-015 manifest fields (defined).
- Dependents: #321 (Bin pub reportable), RFC-014c (import surface), RFC-029a (role as cache-unit
  boundary), RFC-037 (artifact selection), RFC-029d (`--entry` override).
- Landing order with RFC-014c: 014c must not be approved before 029f's import surface order is
  finalized; 014c consumes it by reference, to avoid expanding scope mid-review.

### Risks

- Deviation between role inference and user intuition (the first trade-off above) → all incremental
  behavior is presented as warnings (non-blocking), and can be disabled at any time.
- Transition-period management of the `[exports]` narrowing → within one minor version, warn first.

## Open Questions

All three draft-stage items have been decided (2026-09-13, resolutions recorded in Appendix B and in
the body), no outstanding items:

- [x] Whether to tighten Internal pub → **Decided**: phase 2, tightened by in-package use-graph
      reachability, with Bin-role corpus validation as the prerequisite (see Implementation Strategy
      Phase 2)
- [x] `[[test]]` explicit declaration → **Decided**: not introduced, the Test role is determined
      solely by 036's rules; if 036 needs an explicit declaration in the future, it will be extended
      by 036 itself
- [x] Whether the workspace root can override member export surfaces → **Decided**: not allowed;
      member self-containment is 014c's core design, and root override breaks encapsulation

---

## Appendix B: Decision Log

| Decision                                               | Resolution                                                                                         | Date       | Recorder | Basis                                                                                                                         |
| ------------------------------------------------------ | -------------------------------------------------------------------------------------------------- | ---------- | -------- | ----------------------------------------------------------------------------------------------------------------------------- |
| Number                                                 | 029f (029b reserved for hot reload, 029c deleted, 029d/029e occupied)                              | 2026-09-12 | Chenxu   | RFC-029 §Sub-RFC Plan                                                                                                         |
| Belongs to the 029 family, not a top-level RFC         | Yes                                                                                                | 2026-09-12 | Chenxu   | Import surface / reachability / entry all belong to module-graph semantics; 029 already consumes the `[[bin]]` entry priority |
| Visibility keyword                                     | Not introduced                                                                                     | 2026-09-12 | Chenxu   | Isomorphic to RFC-029's "visibility does not exist" decision; the import surface is carried by the distribution boundary      |
| Status of #321 case B                                  | Downgraded to "transitional semantics when no target exists"                                       | 2026-09-12 | Chenxu   | Bin-role pub reportable is the realization of #321 option A                                                                   |
| Role granularity                                       | File-level                                                                                         | 2026-09-12 | Chenxu   | 015 field granularity (`[lib]` single file, `[[bin]]` list, `[exports]` mapping)                                              |
| `[binaries]` disambiguation                            | 014b precompiled distribution artifact ≠ `[[bin]]` source-code role                                | 2026-09-12 | Chenxu   | Two layers of concept (download policy vs. compilation role); nail it down before 014c lands                                  |
| Tightening Internal pub                                | Phase 2, tightened by in-package use-graph reachability, prerequisite = Bin-role corpus validation | 2026-09-13 | Chenxu   | Prefer under-reporting with gradual tightening; corpus-driven to avoid premature false positives                              |
| `[[test]]` explicit declaration                        | Not introduced; Test is determined solely by 036's rules                                           | 2026-09-13 | Chenxu   | No configuration surface added before 036 has a need                                                                          |
| Workspace root overriding member export surfaces       | Not allowed                                                                                        | 2026-09-13 | Chenxu   | Member self-containment is 014c's core design; root override breaks encapsulation                                             |
| Whether `main` is auto-called in Script                | **No auto-call**—top-level statements are the program; `main()` must be explicit                   | 2026-09-17 | Chenxu   | Coexistence of the two rules would double-run an explicit `main()` (#356); Script can have only one execution entry           |
| Whether the role model is used for entry determination | **Yes** (previously only used for dead-code warnings)                                              | 2026-09-17 | Chenxu   | This RFC defines Bin's "`main` is the reachability root"; without connecting to entry, that semantic idles                    |

## Appendix C: Glossary

| Term                  | Definition                                                                                                                           |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| Role                  | A source file's compilation target identity: Script / Bin / Lib / Test / Internal                                                    |
| Import Surface        | The set of files that serve as valid resolution starting points for cross-package `use`, declared by `[exports]`/`[lib]` or inferred |
| Distribution boundary | 029 term: a package's externally exposed scope; this RFC refines it from the implicit (all pub files) form to the import surface     |
| Script state          | The bypass form for running a single file without a manifest; all behavior is identical to the current state                         |

## References

- RFC-029 Module Semantics (parent RFC: entry selection, visibility decision, sub-RFC plan)
- RFC-015 Configuration System (`[lib]`/`[[bin]]`/`[exports]` field definitions)
- RFC-014b Build System and Binary Distribution (`[binaries]`, target of naming disambiguation)
- RFC-014c Workspace Support (first consumer of import surface, under review)
- RFC-036 Testing Framework (source of Test role rules)
- #321 M2 Independent warning code emission channel (origin of case B and option A)

## Known Defects and Future Improvements (2026-10-09)

### Three Design Defects in the Reference Pool

**Current state**: the current reference pool collects identifiers via `collect_project_refs`
(orchestrator.rs:603-648) and has the following defects:

| #   | Defect                                                                | Impact                                                                                                                                                                                                   | Root cause                                                                                                                                     |
| --- | --------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | **Scan surface is a directory, not the compilation set**              | Identifiers from isolated files never `use`d, and from files under tests/, also enter the pool and exempt unused definitions in production code                                                          | `collect_yx` recursively scans `.yx` (except `.git`/`target`/`.yaoxiang`), not limited to the compilation set returned by `discover_with_used` |
| 2   | **Granularity is project-level bare names, with no module dimension** | A same-name local variable (`let x = 1`) accidentally exempts a top-level definition (`fn x()`); `foo` in a.yx and `foo` in b.yx cannot be distinguished                                                 | Pool type is `HashSet<String>`, key is the bare name                                                                                           |
| 3   | **Script reuses the Lib pool (violates single-file semantics)**       | When `yx run script.yx` runs, references from other files in the project exempt unused definitions in script.yx—Script's semantics is "single-file direct run" and should not be affected by the project | orchestrator.rs:393-396 lets Script use `cross_file_refs`                                                                                      |

**Future improvement directions** (pending P4 4.9 landing):

1. **Scan surface changed to the compilation set**: only collect references from the compilation set
   returned by `discover_with_used`
2. **Granularity upgraded to `(module, name)`**: pool type changed to
   `HashMap<ModulePath, HashSet<String>>`
3. **Script doesn't use the pool**: single-file semantics, decided only by the definition-use graph
4. **Refined consumption definition**: distinguish "imported" from "actually used after import"
   (only the latter counts as consumption)
5. **Test pool separated**: references in tests/ do not exempt production code

**Reason not changed in P3.5**: module-level resolution requires the path resolution of `use` items
to complete (`use a::foo` → foo comes from module a), which belongs to the P4 stage contract and
Driver unification scope.
