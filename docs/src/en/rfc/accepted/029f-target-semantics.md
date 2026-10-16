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
> by RFC-029g. In the body of this document, all statements about "`pub` exemption / `pub` files /
> unused pub warnings" should be read as **"top-level bindings + in-package reference pool"**: the
> role classification (Script / Bin / Lib / Internal / Test) and the import surface semantics remain
> unchanged — only the dead-code exemption criterion has shifted (see
> [029g](029g-remove-pub-and-auto-bind.md) and the criterion in
> `docs/src/reference/warning-code/warning-codes.md`).

Fulfilling RFC-029's "Sub-RFC Planning" extension slot (029b–029e are taken; this is the next slot,
029f): defines a **compilation target role model** (Script / Bin / Lib / Test / Internal — five
roles) for source files, along with the **import surface semantics** — inferred by entry
reachability when no manifest is present (zero-configuration), or explicitly declared via `[lib]` /
`[[bin]]` / `[exports]` (fields already defined in RFC-015) when a manifest is present. Fills four
dangling semantic gaps: what counts as the cross-package import surface, the applicability scope of
the `pub` exemption for dead-code warnings (decision B of #321), the relationship between
`[exports]` and Registry reachability, and the noun disambiguation between RFC-014b `[binaries]`
(precompiled distribution artifacts) and the Bin-role source file.

## Motivation

### Host and boundary basis

RFC-029 (accepted)'s entry selection and visibility decisions are the direct upstream of this RFC:

> **Entry file selection** priority: 1. `[run].main` 2. `[[bin]]`'s first item's `path` 3.
> `src/main.yx` (conventional default). — RFC-029 § Inter-package cycles: revisit

> **Visibility**: does not exist. Scope + distribution boundary cover all scenarios; no new keyword
> needed. — RFC-029 § Design Decision Record (2026-07-30)

The second quote is the design red line for this RFC: import surface semantics **must not introduce
new visibility keywords**; it can only be carried by the "distribution boundary" (file role +
declared export surface).

Four consumers each hit the same wall:

| Consumer                                           | Hit point                                                                                                                                                                                                                                                    |
| -------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| #321 Dead-code warning                             | Decision B "pub = external interface, never reported" exists because no role model distinguishes "truly external" from "looks external"; whether unused pub inside a bin should be reported (option A) remains unresolved                                    |
| RFC-014c Workspace (under review, #113)            | When member packages `use` each other, what counts as the import surface — `[exports]`, the `[lib]` single file, or all pub files — **not a single semantic clause in the entire document**                                                                  |
| RFC-015 Configuration system (accepted)            | The `[lib]` / `[[bin]]` / `[exports]` fields are already defined; aside from RFC-029 consuming the entry priority, the semantics of the remaining fields (especially the relationship between `[exports]` and Registry reachability) have never been aligned |
| RFC-037 Packaging / RFC-029a Caching (draft, #293) | Selecting packaging artifacts and the cache unit boundary for incremental recompilation both need the role model as a prerequisite                                                                                                                           |

### Current problems

**Four semantic gaps** (inventory 2026-09-12, confirmed by reviewing the entirety of
014/014a/014b/014c/015/029):

1. **Cross-package import surface is undefined**. The 014c member-package mutual-reference example
   only has a directory structure (`src/lib.yx`); what `use utils.helper` can get is governed by no
   clause. The current implicit rule is "all top-level pub files of that package" — internal
   implementation files are also dragged into the namespace, making the dependency boundary a
   fiction.
2. **The pub exemption for dead code has no applicability scope**. Decision B's "pub is never
   reported" in #321 is too conservative in single-file scripts (scripts have no external consumers,
   so pub is meaningless) and too loose in the entry file of multi-file projects (an unused pub in a
   root file that no one imports is dead code). Without a role model, the line between options A and
   B cannot be drawn.
3. **The three export concepts are unaligned**. `[exports]` (015, path mapping), `[lib]` (015,
   single-file path), Registry reachability (029, reachable from entry along `use`) — whether they
   have an inclusion relationship, an equality relationship, or each govern their own — has no
   clause.
4. **Noun collision**. RFC-014b's `[binaries]` refers to **precompiled distribution artifacts**
   (download-priority strategy for .so/exe), which is a completely different layer from "bin target
   (source role)". Once 014c lands, the two will inevitably appear on the same screen; failing to
   disambiguate early will keep producing misunderstandings.

**Empirical evidence: the conservativeness of decision B is already an actual pain point**. The #321
M2 retrospective notes: the structural reason why the W1001–W1005 family is fully silenced under
default configuration is precisely "all pub are treated as external interfaces" — this is
compensation for a missing model, not the end state.

## Proposal

### Core design: the file role model

**Role is a file-level attribute** (consistent with 015's field granularity: `[lib]` is a single
file, `[[bin]]` is a file list, `[exports]` is a file mapping), not a package-level switch.

| Role         | Determination                                                                                                                                    | Semantics                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Bin**      | `[run].main` / `[[bin]].path` points to this file; or (without manifest) a file that contains `main` and is not `use`d by any other file         | Program entry. Requires a **binding** named `main`, value or function both work: a function main is called with zero arguments at entry phase, a value main is evaluated at initialization phase and that evaluation is the execution (even if the evaluated result is a function value, it is not called again). Missing `main` is a compile error (E3020); top level must not have executable statements. Its `pub` **may be reported as dead code** (no out-of-package consumers); `main` is a reachability root |
| **Lib**      | A file hit by an `[exports]` mapping; or `[lib].path`; or (without manifest) a file referenced by another file's `use`                           | Distribution boundary. Its `pub` **is exempt from dead code** (out-of-package consumers are invisible, prefer under-reporting); symbols exported through it are visible across packages                                                                                                                                                                                                                                                                                                                             |
| **Test**     | Files matched by RFC-036's test-file rules (existing conventions: `tests/` directory, `*_test.yx`, etc.); **no** `[[test]]` explicit declaration | Test code. Does not participate in dead-code analysis; its references count toward the reachability roots of the code under test                                                                                                                                                                                                                                                                                                                                                                                    |
| **Internal** | When a package has a manifest, a file that is neither on the export surface nor contains `main`                                                  | In-package implementation. `pub` exemption lasts to phase 2 (tightened by in-package use-graph reachability, with verification on Bin-role corpus as the prerequisite); out-of-package `use` **is unreachable**                                                                                                                                                                                                                                                                                                     |
| **Script**   | A file run directly as a single file (`run foo.yx`)                                                                                              | **No entry concept**. Top-level statements (including binding initializers) form the program body in source order; `main` is an ordinary binding, **not auto-invoked** — to run it, write `main()`. Registry is std only                                                                                                                                                                                                                                                                                            |

**Determination priority**: manifest explicit declaration > entry-reachability inference > 036 test
conventions. Explicit declaration always wins — inference is just the default when there is no
declaration.

### Entry determination

The role model not only serves dead-code analysis, it also **drives entry determination**. The entry
rules for each role:

| Role                      | Entry rule                                                                                                                                                                                                                                                            |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Script** (no manifest)  | **No entry concept**. Top-level statements (including binding initializers) form the program body in source order; `main` is an ordinary binding, **not auto-invoked** — to run it, write `main()`                                                                    |
| **Bin** (with manifest)   | Requires a **binding** named `main` (value/function both work: value main is evaluated at initialization phase, function main is called with zero arguments at entry phase); missing `main` is a compile error (E3020). Top level must not have executable statements |
| **Lib / Internal / Test** | No `main` required (they are not entries)                                                                                                                                                                                                                             |

**Script and Bin entries must be mutually exclusive**: "under Script, `main` is not special" and
"under Bin, `main` is the entry" cannot both hold — if Script both executes top-level statements and
implicitly calls `main`, a script that explicitly wrote `main()` would run twice (#356). Therefore
Script has only one execution entry: top-level statements.

**Difference between role determination and entry determination** (implementation note): entry
determination uses "whether there is a manifest" (`find_project_root().is_some()`), not
`roles::classify()`. Reason: classify answers "who consumes this file" (serves dead-code analysis);
a file with a manifest but no `main` is Internal under classify (it is not `use`d by other files,
which is reasonable), but the user runs it as an entry, so an entry must exist. The two questions
differ, so do the criteria.

**Execution form of value `main`** (decision of #388): a function is already a value (unified at the
type level); the entry is determined by **binding existence** rather than callability — value main
and function main just have different evaluation strategies. Value main has no entry-time call (#356
prevents double-running: initialization and entry-time call never stack), the same execution form as
Script; its evaluation is part of the initialization sequence: executed in topological order of
global bindings, with independent bindings in source order. Therefore wrapping Script's top-level
statements into `main = { ... }` and migrating to Bin **is not a source-order-preserving migration**
— scattered statements execute in source order, while binding initializers execute in topological
order; initializations that need to run first should have `main` explicitly depend on them. An
uncallable value main like `main: Int = 5` is a legal program with no observable effect (comparable
to Rust's empty `fn main() {}`).

See `docs/src/reference/language-spec/syntax.md` §3.11.

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
pub unused_fn = (x: Int) => x    # ← W1001 may be reported: the bin has no out-of-package consumers
main: () -> Void = { ... }

# src/lib.yx (Lib role, on the export surface)
pub api_fn = ...                 # ← never reported: out-of-package consumers are invisible

# src/other.yx (Internal role, not on the export surface)
pub semi_api = ...               # ← exempt until phase 2 (prefer under-reporting), then tightened by use graph
```

### Import surface semantics

The resolution order for cross-package `use pkg.x` (the first match wins):

1. An `[exports]` mapping exists → import surface = the set of files listed in the mapping; `x` must
   be among the top-level bindings of those files;
2. Otherwise `[lib].path` exists → import surface = that single file;
3. Neither (manifest-less dependency, path depends on bare directory) → **status quo is
   maintained**: all top-level files are importable (compatibility with existing behavior,
   tightening is a separate discussion).

**Workspace members**: the export surface is defined only by the member package's own manifest; the
workspace root must not override or extend a member's export surface — member self-containment is
014c's core design, and root override would break encapsulation.

Relationship with Registry reachability (aligned with 029): `[exports]` / `[lib]` define the **set
of legal starting points for cross-package resolution**; once the starting points are determined,
the Registry still expands per 029's "reachable from the entry along `use`" — internal files that an
exported file `use`s itself come in through linkage, but do not produce new cross-package starting
points.

### Boundary with existing plans

| Neighbor                                           | Boundary                                                                                                                                                                                                                                                                      |
| -------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| RFC-029d "CLI `--entry` overrides entry" (planned) | 029f defines the role model; 029d consumes it — `--entry` is the CLI-level override mechanism for the Bin role                                                                                                                                                                |
| RFC-014b `[binaries]`                              | **Noun disambiguation**: `[binaries]` is a precompiled distribution artifact (download-priority strategy); `[[bin]]` is a source-role declaration. The former lives in the dependency package's manifest, the latter in this package's manifest; they are not interchangeable |
| RFC-014c Workspace                                 | The import surface for member-package mutual reference = the resolution order in this RFC; the workspace coordination mechanism (members, shared lockfile) still belongs to 014c                                                                                              |
| RFC-036 Testing                                    | The Test role determination directly adopts 036's existing file rules, with no new conventions                                                                                                                                                                                |
| #321 Dead code                                     | Decision B (pub unconditionally exempt) is explicitly downgraded to "transitional semantics when no target is present"; reporting pub for the Bin role is how this RFC delivers option A                                                                                      |

## Detailed design

### Role determination algorithm (inside the orchestrator)

```
fn classify(files, manifest, entry_reach) -> Map<File, Role>:
    # 1. Explicit layer: manifest declarations take priority
    for f in manifest.exports.values():  role[f] = Lib
    if manifest.lib_path:                role[lib_path] = Lib
    for b in manifest.binaries:          role[b.path] = Bin
    if manifest.run_main:                role[run_main] = Bin

    # 2. Inference layer: entry reachability where there is no declaration
    for f in files where role[f] undetermined:
        if f is used by ≥1 other file:        role[f] = Lib
        elif f contains main and no other file uses it: role[f] = Bin
        else:                                 role[f] = Internal

    # 3. Test layer: 036 rule hit → Test (overrides Lib/Internal, not explicit Bin)
    apply_rfc036_test_rules(files, &role)

    # Single-file direct run: the whole model is bypassed, behavior unchanged
    if no manifest and single_file:      all Script
```

### Compiler changes

- `frontend/config.rs`: manifest parsing supplements the `[exports]` → role-table mapping (field
  parsing already exists in 015).
- `frontend/module/orchestrator.rs`: insert a role-classification phase before Registry
  construction; cross-package resolution validates `use` targets against the import surface order,
  and out-of-bounds targets report under the existing `module_not_found` family (no new error codes,
  message augmented with "not on the export surface" hint).
- `typecheck/passes/dead_code.rs`: the entry-point set changes from "main + all pub" to "main + pub
  of Lib-role files" (Bin can be reported immediately; Internal is exempt until phase 2, see
  Implementation Strategy for Phase 2).
- LSP (RFC-017): the "importable items" for completion/hover are filtered by the import surface; the
  `main` missing diagnostic is only reported for Bin-role files.

### Runtime behavior

No change. The role model is pure compile-time / resolution-time semantics and does not affect the
IR or execution.

### Backward compatibility

- Projects without a manifest: behavior is completely unchanged (the inference layer reproduces the
  status quo — files that are used become Lib, files containing main become Bin). The dead-code
  warning increment is only in Bin files: unused pub in a root file that no one imports goes from
  silent to W1001 — this is the expected behavior of option A from #321, not a break.
- Projects with a manifest: when `[exports]` is already declared, the import surface narrows from
  "all pub files" to the declared surface. **This is the only behavior-narrowing point**: code that
  depends on other packages' internal files will start to report `module_not_found`. Given that the
  package ecosystem has not yet been established (029: "currently no third-party package
  ecosystem"), the break surface is zero; a minor-version transition period is still provided
  (out-of-bounds warns first, then errors).

## Trade-offs

### Advantages

- Zero new configuration surface: all fields are already defined in 015; this RFC only adds
  semantics.
- Zero new keywords: the import surface is carried by the distribution boundary, isomorphic to 029's
  "visibility does not exist" decision.
- The four consumers (#321 / 014c / 029a / 037) are modeled once and stop bumping into the same
  wall.
- Zero cost for the no-manifest path: the zero-configuration gene of the scripting language is fully
  preserved.

### Disadvantages

- Role inference ("used = Lib") in the common form "entry file uses utility file" classifies the
  utility file as Lib, and its pub is exempt — coarser than ideal granularity. This is a deliberate
  choice in the prefer-under-reporting direction; tightening would require call-direction analysis
  on the use graph, which is not done in the first version.
- Narrowing the import surface via `[exports]` is a breaking semantic tightening (though a
  transition period is provided).

## Alternatives

| Option                                   | Description                                                             | Reason not adopted                                                                                                                                                                                                                        |
| ---------------------------------------- | ----------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Top-level RFC-040                        | Open a new top-level number                                             | Module-graph semantics (import surface / reachability / entry) all live in 029's territory; splitting off the top level would produce two top-level documents for the same subject; and the 029b–029e slot pattern is already established |
| Hang under 014c                          | As a section of workspace                                               | Dependency direction is reversed: the language layer (029x) defines semantics, the package layer (014x) consumes semantics. 014c is still under review; expanding scope mid-review would slow its landing                                 |
| Option A as-is (#321 menu)               | Manifest-required `[[bin]]` / `[[lib]]` targets for dead-code semantics | Introduces mandatory configuration for a narrow slice (unused pub in a root file with main); the inference layer in this RFC gets the same gain with zero configuration                                                                   |
| Pure entry inference (no manifest layer) | Does not consume 015 fields                                             | `[exports]` is already an accepted 015 field; the cross-package import surface cannot bypass it; leaving it undefined is equivalent to leaving a gap for 014c                                                                             |

## Implementation strategy

### Phasing

- **Phase 1**: role classification + import surface resolution + Bin pub may be reported
  (Internal/Test remain exempt)
- **Phase 2**: tighten Internal pub to "in-package use-graph unreachable = may be reported" —
  prerequisite is the Bin-role warning corpus verification with no false positives after Phase 1
  lands; trigger when ready, no timetable set

### Dependency relationships

- Prerequisites: RFC-029 orchestrator (landed), RFC-015 manifest fields (defined).
- Depended on by: #321 (Bin pub may be reported), RFC-014c (import surface), RFC-029a (role as cache
  unit boundary), RFC-037 (artifact selection), RFC-029d (`--entry` override).
- Landing order with RFC-014c: before 014c is approved, 029f's import surface order should be
  finalized first, and 014c should consume it by reference, to avoid scope expansion mid-review.

### Risks

- Deviation between role inference and user intuition (first trade-off above) → present all
  incremental behavior in warning form (non-blocking), can be disabled at any time.
- Managing the `[exports]` narrowing transition period → warn first within a minor version.

## Open questions

All three from the draft phase have been decided (2026-09-13, resolutions recorded in Appendix B and
the body), none outstanding:

- [x] Whether to tighten Internal pub → **Decision**: in phase 2, tighten by in-package use-graph
      reachability, with Bin-role corpus verification as the prerequisite (see Implementation
      Strategy Phase 2)
- [x] Explicit `[[test]]` declaration → **Decision**: not introduced; the Test role is determined
      solely by 036 rules; if 036 later needs an explicit declaration, it will be extended by itself
- [x] Workspace root overriding member export surfaces → **Decision**: not allowed; member
      self-containment is 014c's core design, and root override breaks encapsulation

---

## Appendix B: Design Decision Record

| Decision                                               | Resolution                                                                                           | Date       | Recorder | Basis                                                                                                                    |
| ------------------------------------------------------ | ---------------------------------------------------------------------------------------------------- | ---------- | -------- | ------------------------------------------------------------------------------------------------------------------------ |
| Number                                                 | 029f (029b reserved hot reload, 029c deleted, 029d/029e occupied)                                    | 2026-09-12 | Chenxu   | RFC-029 § Sub-RFC Planning                                                                                               |
| Belongs to 029 family, not top-level RFC               | Yes                                                                                                  | 2026-09-12 | Chenxu   | Import surface / reachability / entry are all module-graph semantics; 029 already consumes `[[bin]]` entry priority      |
| Visibility keyword                                     | Not introduced                                                                                       | 2026-09-12 | Chenxu   | Isomorphic to RFC-029's "visibility does not exist" decision; the import surface is carried by the distribution boundary |
| Status of #321 decision B                              | Downgraded to "transitional semantics when no target is present"                                     | 2026-09-12 | Chenxu   | Bin-role pub may be reported is the delivery form of #321 option A                                                       |
| Role granularity                                       | File-level                                                                                           | 2026-09-12 | Chenxu   | 015 field granularity (`[lib]` single file, `[[bin]]` list, `[exports]` mapping)                                         |
| `[binaries]` disambiguation                            | 014b precompiled distribution artifact ≠ `[[bin]]` source role                                       | 2026-09-12 | Chenxu   | Two-layer concepts (download strategy vs. compilation role); nail down before 014c lands                                 |
| Internal pub tightening                                | Phase 2 tightening by in-package use-graph reachability, prerequisite = Bin-role corpus verification | 2026-09-13 | Chenxu   | Prefer under-reporting with progressive tightening, corpus-driven to avoid premature false positives                     |
| Explicit `[[test]]` declaration                        | Not introduced; Test is determined solely by 036 rules                                               | 2026-09-13 | Chenxu   | Don't add a configuration surface before 036 has a need                                                                  |
| Workspace root overriding member export surfaces       | Not allowed                                                                                          | 2026-09-13 | Chenxu   | Member self-containment is 014c's core design; root override breaks encapsulation                                        |
| Whether Script auto-calls `main`                       | **No auto-call** — top-level statements are the program; `main()` must be explicit                   | 2026-09-17 | Chenxu   | The two rules coexisting would cause explicit `main()` to run twice (#356); Script can have only one execution entry     |
| Whether the role model is used for entry determination | **Yes** (previously only used for dead-code warnings)                                                | 2026-09-17 | Chenxu   | This RFC defines Bin's "`main` is a reachability root"; without hooking entry that semantics idles                       |

## Appendix C: Glossary

| Term                  | Definition                                                                                                                         |
| --------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| Role                  | A source file's compilation target identity: Script / Bin / Lib / Test / Internal                                                  |
| Import Surface        | The set of files that are legal resolution starting points for cross-package `use`, declared or inferred via `[exports]` / `[lib]` |
| Distribution Boundary | 029 term: the outward-exposed scope of a package; this RFC refines it from the implicit (all pub files) to the import surface      |
| Script state          | The bypass form for running a single file without a manifest; all behavior matches the status quo                                  |

## References

- RFC-029 Module Semantics (parent RFC: entry selection, visibility decision, sub-RFC planning)
- RFC-015 Configuration System (`[lib]` / `[[bin]]` / `[exports]` field definitions)
- RFC-014b Build System and Binary Distribution (`[binaries]`, the noun-disambiguation target)
- RFC-014c Workspace Support (the first consumer of the import surface, under review)
- RFC-036 Testing Framework (source of the Test role rules)
- #321 M2 Independent Emission Channel for Warning Codes (origin of decision B and option A)

## Known defects: landed (2026-10-16, WBS 4.9 round 1)

> The previous version (2026-10-09) listed the following three as "three design defects in the
> reference pool". All three were landed one by one by WBS 4.9 round 1 (commit `3863a9ff0`);
> `collect_project_refs` and `orchestrator.rs` have been moved with the P4 4.2.9 split into
> `src/driver/{state,arms,helpers}.rs`; all file:line references in the table below were re-fetched
> on the referenced revision.

### The three design defects in the reference pool — fix status

| #   | Original defect                                                   | Current state (2026-10-16)                                                                                                                                                                                                                                                                                                                                                                                                | Landing code (rev `3863a9ff0`)                                                                                                                            |
| --- | ----------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | The scan surface is the directory, not the compilation surface    | **Pool root = package target**: `collect_package_refs` (`helpers.rs:481`) calls `discover_with_used` from the roots of `roles::ExplicitSurfaces`'s `[[bin]]` / `[lib]` / `[exports]` / `[run].main` to obtain the compilation set; **does not scan the directory for entries**. When none of the roots resolve, it falls back to the inspected file's own compilation set (`collect_compilation_refs` / `helpers.rs:454`) | `src/driver/helpers.rs:454-512` (fallback path); `src/driver/helpers.rs:481-512` (manifest-declared-surface path)                                         |
| 2   | Granularity is project-level bare names, with no module dimension | **Pool type = `HashMap<String, HashSet<String>>`**: keys = module keys, values = that module's set of reference names; the consumer side at `arms.rs:603-609` unions all module entries "besides this unit" and feeds them to `DeadCodeAnalyzer`; a module's own names (top-level definitions and local bindings, `use`-imported names are an exception) no longer exempt definitions of the same name in other modules   | `src/driver/arms.rs:603-609` (consumer-side `other_module_refs`); the single assembly entry point at `src/driver/helpers.rs:429-449` (`collect_ref_pool`) |
| 3   | Script reuses Lib's pool (violates single-file semantics)         | **Script does not join the pool**: the `match us.role` at `arms.rs:611-623` only wires up `Bin` / `Internal` (`set_exempt_pub`) and `Lib` (`set_cross_file_refs`); Script/Test take the `_ => {}` default arm, which only judges by the in-file definition-use graph                                                                                                                                                      | `src/driver/arms.rs:611-623`                                                                                                                              |

**Follow-up improvements 4 / 5 — fix status (2026-10-16, WBS 4.9 round 2)**

> The two items in this section were originally listed as "awaiting P4 4.9 landing". Both were
> landed one by one by WBS 4.9 round 2; the file:line below are taken on **this landing's working
> tree** (the commit hash is filled in by the batch wrap-up after the push — see 09a for the round 1
> lesson: the local hash before push gets rewritten by `pull --rebase`).

| #   | Original improvement                                                                                                                       | Current state (2026-10-16)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Landing code                                                                                                                                                                                                                                                                                   |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 4   | Consumption definition refinement: distinguish between "imported" and "actually used after import" (only the latter counts as consumption) | **Consumption criterion = "actually used after import"**: a consumption record = some module `use`s name n from module M into scope **and really references n at a use site**; recorded only against M (`RefPool::consumed_from`); appearances only in a use item (no use site), references to **same-named** identifiers from other modules do not count as consumption. The target module of an import is determined by the discovery-phase module-key set (`resolve_import_module`, **does not start a separate path resolution** — reuses the discovery-phase resolution product delivered in 4.6.1, see `39546020`); namespace form (`use lib` + `lib.foo()`) is attributed the same way | `src/driver/helpers.rs:442-446` (`RefPool`); `src/driver/helpers.rs:449-524` (`collect_ref_pool` + `resolve_import_module`); `src/frontend/core/typecheck/passes/dead_code.rs:180-192` (`ModuleNames`), `:328-352` (`collect_module_names`), `:122-176` (`ImportFace` / `collect_import_face`) |
| 5   | Test pool separation: references in tests/ do not exempt production code (src/)                                                            | **`FileRole::Test` contributes no consumption surface**: Test modules form a separate pool on their own (Test itself does not participate in dead-code analysis, so its references do not spill out); production-code definitions are kept alive only by production-code consumers. The role is determined at the single point `roles::classify` (test discovery rules come from RFC-036)                                                                                                                                                                                                                                                                                                     | `src/driver/helpers.rs:470-476` (the Test short-circuit in `collect_ref_pool`); `src/driver/helpers.rs:595-601` (the single-point `roles::classify`); `src/driver/arms.rs:562-580` (both the degradation and declared-surface paths feed the role to the pool)                                 |

**Residual surface (registered, non-blocking)**: ① field/method positional references
(`obj.render()` only has the short name) cannot be attributed at the AST level to the receiver
type's module — only when a base name hits a namespace import does attribution follow the import;
otherwise it is conservatively counted by name dimension in `RefPool::field_names`
(`src/driver/helpers.rs:446`); ② same-named overloads are **judged together by short name** (a
called overload makes other same-named overloads reachable as well, in the prefer-under-reporting
direction) — per-candidate attribution would consume the overload-resolution table; WBS 4.9.6 (issue
#450, 2026-10-16 user decision A) only wraps up the three items "called = not reported + one line
per definition + still reported when not called"; per-candidate attribution is left for subsequent
registration.

### Reference count distortion (WBS 4.9.6 / issue #450) — fix status (2026-10-16)

> Original registration: overload methods **called and resolution effective, still falsely reported
> as `W1005 Unused method`** (the reference count distorts against overload calls). On 2026-10-16,
> user decision A registered it as an independent level-3 task and merged it into WBS 4.9 round 2;
> this item is category D (removing the false report changes the outward-facing surface of
> diagnostics), authorized to land.

| Stage                   | Original defect                                                                                                                                                                                                                                                | Current state                                                                                                                                                                                                                                               | Landing code                                                                                            |
| ----------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| Reachability probe      | Scanning looks up references by **qualified name** (`Point.render`), while reference points are collected by **short name** (`obj.render()` only has `render`) ⇒ called methods are never reachable, and W1005 is falsely reported in the no-project-root case | The method uses a **short-name probe** (`compute_reachability`); uncalled methods remain unreachable and are reported faithfully as before                                                                                                                  | `src/frontend/core/typecheck/passes/dead_code.rs:733-744` (probe); `:766-770` (per-definition decision) |
| Definition table keying | `all_defs` is keyed by qualified name ⇒ **same-named overloads overwrite each other** (two overloads leave only one, only one is reported)                                                                                                                     | The definition table is **one entry per definition**: methods are appended; same-named non-method bindings are deduped by name, keeping the last (overloads are independent definitions, while top-level reassignment of the same name is the same binding) | `src/frontend/core/typecheck/passes/dead_code.rs:304-322` (`record_definition`); `:28-29` (field)       |

**Reason not changed in P3.5 (same as the previous version)**: module-level resolution requires the
path resolution of `use` items to be complete (`use a::foo` → foo comes from module a), which falls
under the P4-stage contract and Driver unification.
