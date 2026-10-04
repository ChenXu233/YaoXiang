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

This RFC fulfills the extended slot from RFC-029 "Sub-RFC Planning" (slots 029b–029e are already
occupied, deferred to 029f): it defines a **compilation target role model** (five categories: Script
/ Bin / Lib / Test / Internal) for source files, together with its **import surface semantics**—when
no manifest is present, infer by entry reachability (zero configuration); when a manifest is
present, declare explicitly via `[lib]` / `[[bin]]` / `[exports]` (fields already defined in
RFC-015). It fills four dangling semantics: how cross-package import surfaces are computed, the
applicable scope of `pub` exemption for dead-code warnings (Issue #321 Decision B), the relationship
between `[exports]` and Registry reachability, and the terminology disambiguation between RFC-014b
`[binaries]` (prebuilt distribution artifacts) and bin-role source files.

## Motivation

### Host and Boundary Foundations

RFC-029 (Accepted)'s entry-point selection and visibility decisions are the direct upstream of this
RFC:

> **Entry file selection** priority: 1. `[run].main` 2. The first `[[bin]]`'s `path` 3.
> `src/main.yx` (conventional default). — RFC-029 §Inter-package Cycles: Continued

> **Visibility**: doesn't exist. Scope + distribution boundaries cover all scenarios; no new
> keywords are needed. — RFC-029 §Design Decision Record (2026-07-30)

The second quote is the design red line of this RFC: import surface semantics **must not introduce
new visibility keywords**; they can only be carried by "distribution boundaries" (file role +
declared export surface).

Four consumers each hit the same wall:

| Consumer                                           | Pain Point                                                                                                                                                                                                                                   |
| -------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| #321 Dead-code warnings                            | Decision B "pub = external interface, never reported" is because there is no role model to distinguish "truly external" from "looks external"; whether to report unused pub inside a bin (Plan A) has been pending                           |
| RFC-014c Workspaces (under review, #113)           | When member packages `use` each other, what counts as the import surface—`[exports]`, the `[lib]` single file, or all pub files—**not a single semantic clause in the entire document**                                                      |
| RFC-015 Configuration System (Accepted)            | `[lib]` / `[[bin]]` / `[exports]` fields are defined; aside from RFC-029 consuming entry priority, the semantics of the remaining fields (especially the relationship between `[exports]` and Registry reachability) have never been aligned |
| RFC-037 Packaging / RFC-029a Caching (Draft, #293) | Selection of packaging artifacts, cache unit boundaries for incremental recompilation, all need the role model as a prerequisite                                                                                                             |

### The Current Problem

**Four semantic gaps** (inventory on 2026-09-12, confirmed by reviewing 014/014a/014b/014c/015/029
in full):

1. **No definition for cross-package import surface**. The 014c member-package mutual-reference
   example only has a directory structure (`src/lib.yx`); what `use utils.helper` can reach is
   defined by no clause. The current implicit rule is "all top-level pub files of that
   package"—internal implementation files are dragged into the namespace too, making the dependency
   boundary effectively void.
2. **No applicable scope for dead-code pub exemption**. Decision B from #321, "pub never reported,"
   is too conservative in single-file scripts (a script has no external consumer, pub is
   meaningless) and too permissive in entry files of multi-file projects (an unimported pub in the
   root file is dead code). Without a role model, the dividing line between Plans A and B cannot be
   drawn.
3. **Three export concepts left unaligned**. `[exports]` (015, path mapping), `[lib]` (015, single
   file path), Registry reachability (029, reachable from entry along `use`)—are they in inclusion,
   equal, or independent relation, with no clause.
4. **Terminology collision**. RFC-014b's `[binaries]` refers to **prebuilt distribution artifacts**
   (.so/exe download-priority policy), a completely different layer from the "bin target (source
   role)". After 014c lands, both will inevitably appear on the same screen; failing to disambiguate
   early will keep causing confusion.

**Empirical evidence: Decision B's conservatism is already a real pain point**. The #321 M2
retrospective record: the structural reason why the W1001–W1005 family defaults to silence under the
default configuration is exactly "all pub are treated as external interfaces"—this is a compensation
behavior for the missing model, not an end state.

## Proposal

### Core Design: File Role Model

**Role is a file-level property** (consistent with the granularity of 015 fields: `[lib]` is a
single file, `[[bin]]` is a list of files, `[exports]` is a file mapping), not a package-level
switch.

| Role         | Determination                                                                                                                                | Semantics                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| ------------ | -------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Bin**      | File pointed to by `[run].main` / `[[bin]].path`; or (no manifest) a file containing `main` that is not `use`d by any other file             | Program entry. Requires a **binding** named `main`; both values and functions are acceptable: a function `main` is called with zero arguments at entry, a value `main` is evaluated at initialization time and the result is the program body (if the evaluated result is itself a function value, it is not called again). Missing `main` is a compile error (E3020); top-level executable statements are forbidden. Its `pub` **may be reported as dead code** (no out-of-package consumer); `main` is a reachability root |
| **Lib**      | File hit by an `[exports]` mapping; or `[lib].path`; or (no manifest) a file `use`d by another file                                          | Distribution boundary. Its `pub` **is exempt from dead-code** (out-of-package consumer is invisible; prefer under-reporting); symbols exported through it are visible across packages                                                                                                                                                                                                                                                                                                                                        |
| **Test**     | File matched by RFC-036 test file rules (`tests/` directory, `*_test.yx`, etc. existing conventions); **no** explicit `[[test]]` declaration | Test code. Does not participate in dead-code determination; its references count as reachability roots of the code under test                                                                                                                                                                                                                                                                                                                                                                                                |
| **Internal** | When the package has a manifest: a file neither in the export surface nor containing `main`                                                  | In-package implementation. `pub` is exempted to Phase 2 (tightened by in-package `use`-graph reachability, gated on Bin role corpus verification); out-of-package `use` is **unreachable**                                                                                                                                                                                                                                                                                                                                   |
| **Script**   | A file run directly as a single file (`run foo.yx`)                                                                                          | **No entry concept**. Top-level statements (including binding initialization) in source order form the program body; `main` is a regular binding, **not auto-called**—write `main()` to run. The Registry contains only std                                                                                                                                                                                                                                                                                                  |

**Determination priority**: manifest explicit declaration > entry-reachability inference > 036 test
conventions. Explicit declaration always wins—inference is only the default when there is no
declaration.

### Entry Determination

The role model not only serves dead-code analysis, but also **drives entry determination**. Each
role's entry rules:

| Role                      | Entry Rule                                                                                                                                                                                                                                                                        |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Script** (no manifest)  | **No entry concept**. Top-level statements (including binding initialization) in source order form the program body; `main` is a regular binding, **not auto-called**—write `main()` to run                                                                                       |
| **Bin** (with manifest)   | Requires a **binding** named `main` (both values and functions are acceptable: a value `main` is evaluated at initialization, a function `main` is called with zero arguments at entry); missing `main` is a compile error (E3020). Top-level executable statements are forbidden |
| **Lib / Internal / Test** | `main` not required (they are not entry points)                                                                                                                                                                                                                                   |

**Script and Bin entries must be mutually exclusive**: "in Script, `main` is not special" and "in
Bin, `main` is the entry" cannot both hold—if Script both executes top-level statements and
implicitly calls `main`, a script that explicitly writes `main()` would run twice (#356). So Script
has only one execution entry: top-level statements.

**Distinction between role determination and entry determination** (implementation note): entry
determination uses "whether a manifest exists" (`find_project_root().is_some()`), not
`roles::classify()`. The reason: `classify` answers "who consumes this file" (serving dead-code
analysis); a file with a manifest but no `main` is Internal in `classify` (it isn't `use`d by other
files, which is reasonable), but the user runs it as the entry, so it must have an entry. Two
different questions, naturally different criteria.

**Execution form of value `main`** (#388 decision): functions are already values (unified at the
type level); the entry is judged by **binding existence**, not callability—value `main` and function
`main` differ only in evaluation strategy. Value `main` does not set an entry call (#356 double-run
guard: initialization and entry call never stack), sharing Script's execution form; its evaluation
is part of the initialization sequence: executed in global binding topological order, independent
bindings in source order. Therefore, wrapping Script top-level statements into `main = { ... }` to
migrate into Bin **is not order-preserving migration**—scattered statements execute in source order,
binding initialization in topological order; initialization that needs to come first should have
`main` explicitly depend on it. A non-callable value main like `main: Int = 5` is a legal but
observation-effect-free program (analogous to Rust's empty `fn main() {}`).

See `docs/src/reference/language-spec/syntax.md` §3.11 for details.

### Example

```toml
# yaoxiang.toml（字段全部是 RFC-015 已定义的，本 RFC 不新增配置面）
[lib]
path = "src/lib.yx"

[[bin]]
name = "my-cli"
path = "src/cli.yx"

[exports]
"." = "src/lib.yx"
"./internal-helper" = "src/helper.yx"   # ← 决定跨包可见性的是它，不是 pub
```

```yaoxiang
# src/cli.yx（Bin 角色）
pub unused_fn = (x: Int) => x    # ← W1001 可报：bin 无包外消费者
main: () -> Void = { ... }

# src/lib.yx（Lib 角色，在导出面上）
pub api_fn = ...                 # ← 永不报：包外消费者不可见

# src/other.yx（Internal 角色，不在导出面）
pub semi_api = ...               # ← 豁免至二阶段（宁漏报），届时按 use 图收紧
```

### Import Surface Semantics

The resolution order for cross-package `use pkg.x` (first hit takes effect):

1. `[exports]` mapping exists → import surface = the file set listed by the mapping; `x` must be in
   those files' top-level bindings;
2. Otherwise, `[lib].path` exists → import surface = that single file;
3. Neither (no manifest dependency; path depends on a bare directory) → **maintain current
   behavior**: all top-level files are importable (compatible with existing behavior; tightening to
   be discussed separately).

**Workspace members**: the export surface is defined only by the member package's own manifest; the
workspace root must not override or extend member export surfaces—member self-containment is 014c's
core design; root override would break encapsulation.

Relationship with Registry reachability (aligned with 029): `[exports]` / `[lib]` define the **legal
starting-point set for cross-package resolution**; once the starting points are determined, the
Registry still extends per 029's "reachable from entry along `use`"—internal files `use`d by
exported files enter via linking, but do not generate new cross-package starting points.

### Boundaries with Existing Plans

| Neighbor                                             | Boundary                                                                                                                                                                                                                                                           |
| ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| RFC-029d "CLI `--entry` overriding entry" (planning) | 029f defines the role model; 029d consumes it—`--entry` is the CLI-level override means for the Bin role                                                                                                                                                           |
| RFC-014b `[binaries]`                                | **Terminology disambiguation**: `[binaries]` is prebuilt distribution artifacts (download-priority policy); `[[bin]]` is a source-role declaration. The former exists in the dependency's manifest, the latter in this package's manifest; they do not interchange |
| RFC-014c Workspaces                                  | Member-package mutual-reference import surface = the resolution order in this RFC; workspace coordination mechanisms (members, shared lockfile) still belong to 014c                                                                                               |
| RFC-036 Tests                                        | Test role determination directly adopts 036's existing file rules; no new conventions added                                                                                                                                                                        |
| #321 Dead code                                       | Decision B (pub always exempted) is explicitly downgraded to "transitional semantics when no target is present"; Bin role's pub being reportable is the realization of Plan A in this RFC                                                                          |

## Detailed Design

### Role Determination Algorithm (Inside the Orchestrator)

```
fn classify(files, manifest, entry_reach) -> Map<File, Role>:
    # 1. 显式层：manifest 声明优先
    for f in manifest.exports.values():  role[f] = Lib
    if manifest.lib_path:                role[lib_path] = Lib
    for b in manifest.binaries:          role[b.path] = Bin
    if manifest.run_main:                role[run_main] = Bin

    # 2. 推断层：无声明处按入口可达性
    for f in files where role[f] 未定:
        if f 被 ≥1 个非自身文件 use:      role[f] = Lib
        elif f 含 main 且无其他文件 use 它: role[f] = Bin
        else:                             role[f] = Internal

    # 3. 测试层：036 规则命中 → Test（覆盖 Lib/Internal，不覆盖显式 Bin）
    apply_rfc036_test_rules(files, &role)

    # 单文件直跑：整个模型旁路，行为不变
    if no manifest and single_file:      all Script
```

### Compiler Changes

- `frontend/config.rs`: manifest parsing supplements the `[exports]` → role table mapping (field
  parsing already in 015).
- `frontend/module/orchestrator.rs`: insert a role-classification stage before Registry
  construction; cross-package resolution validates `use` targets per the import surface order;
  out-of-bound targets report the existing `module_not_found` family (no new error codes, message
  augmented with "not in export surface" hint).
- `typecheck/passes/dead_code.rs`: the entry-point set changes from "main + all pub" to "main + pub
  of Lib-role files" (Bin immediately reportable; Internal exempted until Phase 2, see
  implementation strategy).
- LSP (RFC-017): completion/hover "importable items" filtered by import surface; `main`-missing
  diagnostic reported only for Bin-role files.

### Runtime Behavior

No change. The role model is purely a compile-time/parse-time semantic, and does not affect IR or
execution.

### Backward Compatibility

- No-manifest projects: behavior completely unchanged (the inference layer restores current
  state—`use`d files are Lib, `main`-containing files are Bin). The dead-code warning increment only
  affects Bin files: an unused pub in a root file that no one imports changes from silent to
  W1001—this is the expected behavior of #321 Plan A, not a break.
- Manifest projects: when `[exports]` is declared, the import surface narrows from "all pub files"
  to the declared surface. **This is the only behavior-narrowing point**: code that depends on other
  packages' internal files will start reporting `module_not_found`. Given the package ecosystem has
  not yet been established (029: "currently no third-party package ecosystem"), the break surface is
  zero; a minor-version transition period is still set (out-of-bound first warn, then error).

## Trade-offs

### Pros

- Zero new configuration surfaces: all fields are already defined in 015; this RFC only supplements
  semantics.
- Zero new keywords: import surface is carried by distribution boundaries, isomorphic to the 029 "no
  visibility" decision.
- Four consumers (#321 / 014c / 029a / 037) modeled once, no longer each hitting their own wall.
- No-manifest path zero cost: the scripting language's zero-configuration gene is fully preserved.

### Cons

- Role inference (being `use`d = Lib) in the common form of "entry file `use`s utility file" will
  classify the utility file as Lib, and its pub is exempted—wider than the ideal granularity. This
  is a deliberate choice in the under-report direction; tightening requires call-direction analysis
  on the `use` graph, not done in the first version.
- `[exports]` narrowing the import surface is a breakable semantic narrowing (despite the transition
  period).

## Alternatives

| Alternative                              | Description                                                                | Reason Not Adopted                                                                                                                                                                                                                        |
| ---------------------------------------- | -------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Top-level RFC-040                        | Open a new top-level number                                                | Module-graph semantics (import surface / reachability / entry) all belong to the 029 domain; splitting into a top-level would create two top-level documents for the same object; also, the 029b–029e slot pattern is already established |
| Attached to 014c                         | As a section of workspaces                                                 | Dependency direction is reversed: the language layer (029x) defines semantics, the package layer (014x) consumes semantics. 014c is still under review; expanding scope midway will delay its landing                                     |
| Plan A as-is (#321 menu)                 | Manifest requires `[[bin]]` / `[[lib]]` target to have dead-code semantics | Introduces mandatory configuration for a narrow slice (unused pub in a root file with `main`); the inference layer of this RFC achieves the same benefit with zero configuration                                                          |
| Pure entry inference (no manifest layer) | Does not consume 015 fields                                                | `[exports]` is already a field accepted in 015; cross-package import surface cannot bypass it; leaving it undefined equals leaving a gap for 014c                                                                                         |

## Implementation Strategy

### Phased

- **Phase 1**: Role classification + import surface resolution + Bin pub reportable (Internal/Test
  remain exempted)
- **Phase 2**: Internal pub tightened to "in-package `use`-graph unreachable =
  reportable"—prerequisite: after Phase 1 lands, Bin role warning corpus verification with no false
  positives; triggered on completion, no schedule set

### Dependency Relationships

- Prerequisites: RFC-029 orchestrator (landed), RFC-015 manifest fields (defined).
- Depended on by: #321 (Bin pub reportable), RFC-014c (import surface), RFC-029a (roles as cache
  unit boundary), RFC-037 (artifact selection), RFC-029d (`--entry` override).
- Landing order with RFC-014c: before 014c review passes, 029f's import surface order should be
  finalized first; 014c consumes by reference, avoiding scope expansion during review.

### Risks

- Deviation between role inference and user intuition (first con in Trade-offs) → present all
  incremental behavior as warnings (non-blocking), disable-able at any time.
- Transition-period management of `[exports]` narrowing → warn first within a minor version.

## Open Questions

The three draft-stage questions have all been decided (2026-09-13, resolutions entered in Appendix B
and the body), with no open items:

- [x] Whether Internal pub is tightened → **Decision**: tightened in Phase 2 by in-package
      `use`-graph reachability, gated on Bin role corpus verification (see Implementation Strategy
      Phase 2)
- [x] `[[test]]` explicit declaration → **Decision**: not introduced; Test role determined only by
      036 rules; when 036 has future need for explicit declaration, it will extend itself
- [x] Workspace root overriding member export surface → **Decision**: not allowed; member
      self-containment is 014c's core design; root override breaks encapsulation

---

## Appendix B: Design Decision Record

| Decision                                               | Resolution                                                                                             | Date       | Recorder | Basis                                                                                                                         |
| ------------------------------------------------------ | ------------------------------------------------------------------------------------------------------ | ---------- | -------- | ----------------------------------------------------------------------------------------------------------------------------- |
| Number                                                 | 029f (029b reserved for hot-reload, 029c deleted, 029d/029e already occupied)                          | 2026-09-12 | Chenxu   | RFC-029 §Sub-RFC Planning                                                                                                     |
| Belongs to the 029 family, not a top-level RFC         | Yes                                                                                                    | 2026-09-12 | Chenxu   | Import surface / reachability / entry all belong to module-graph semantics; 029 has already consumed `[[bin]]` entry priority |
| Visibility keyword                                     | Not introduced                                                                                         | 2026-09-12 | Chenxu   | Isomorphic to the RFC-029 "no visibility" decision; import surface is carried by distribution boundaries                      |
| Status of #321 Decision B                              | Downgraded to "transitional semantics when no target is present"                                       | 2026-09-12 | Chenxu   | Bin role pub reportable is the realization of #321 Plan A                                                                     |
| Role granularity                                       | File level                                                                                             | 2026-09-12 | Chenxu   | 015 field granularity (`[lib]` single file, `[[bin]]` list, `[exports]` mapping)                                              |
| `[binaries]` disambiguation                            | 014b prebuilt distribution artifact ≠ `[[bin]]` source role                                            | 2026-09-12 | Chenxu   | Two layers of concepts (download policy vs. compilation role); nail down before 014c lands                                    |
| Internal pub tightening                                | Phase 2 tightening by in-package `use`-graph reachability, prerequisite = Bin role corpus verification | 2026-09-13 | Chenxu   | Prefer under-reporting, progressively tighten; corpus-driven to avoid premature false positives                               |
| `[[test]]` explicit declaration                        | Not introduced; Test determined only by 036 rules                                                      | 2026-09-13 | Chenxu   | No new configuration surface before 036 needs it                                                                              |
| Workspace root overriding member export surface        | Not allowed                                                                                            | 2026-09-13 | Chenxu   | Member self-containment is 014c's core design; root override breaks encapsulation                                             |
| Whether `main` is auto-called in Script                | **Not auto-called**—top-level statements are the program; `main()` must be explicit                    | 2026-09-17 | Chenxu   | Coexistence of the two rules would let an explicit `main()` run twice (#356); Script can have only one execution entry        |
| Whether the role model is used for entry determination | **Yes** (previously only for dead-code warnings)                                                       | 2026-09-17 | Chenxu   | This RFC defines Bin's "`main` is a reachability root"; not connecting to entry would leave that semantic idle                |

## Appendix C: Glossary

| Term                  | Definition                                                                                                                        |
| --------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| Role                  | A source file's compilation target identity: Script / Bin / Lib / Test / Internal                                                 |
| Import Surface        | The set of files that are legal resolution starting points for cross-package `use`; declared by `[exports]` / `[lib]` or inferred |
| Distribution Boundary | 029 term: a package's externally exposed range; this RFC refines it from implicit (all pub files) to the import surface           |
| Script State          | The bypass form for no-manifest single-file direct run; all behavior matches the current state                                    |

## References

- RFC-029 Module Semantics (parent RFC: entry selection, visibility decisions, sub-RFC planning)
- RFC-015 Configuration System (`[lib]` / `[[bin]]` / `[exports]` field definitions)
- RFC-014b Build System and Binary Distribution (`[binaries]`, terminology disambiguation target)
- RFC-014c Workspace Support (first consumer of import surface, under review)
- RFC-036 Test Framework (source of Test role rules)
- #321 M2 Warning Code Independent Emission Channel (origin of Decision B and Plan A)
