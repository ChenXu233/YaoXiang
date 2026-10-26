# Repository Maintenance Mechanism and Code Placement Decision Procedure

> **The rules themselves have been migrated (2026-10-05)**: the exact descriptions of the three
> prohibitions, D0–D4, the red lines, and the review checklist are in
> [coding-rules.md](../coding-rules.md)—the sole long-term authority. This document is retained as
> the **October 2026 diagnostic and decision record**: incident evidence, line numbers, external
> convention references, and arguments for rejected approaches live and die with RFC-039. When
> enforcing these rules, cite coding-rules.md, not this document as the source of the rules.

> **Ancillary design document**. This document accompanies
> [RFC-039 Compiler Architecture Restructuring](../../rfc/accepted/039-compiler-architecture.md) and
> corresponds to the **P0 phase**.
>
> This document is paired with [09-execution-wbs.md](09-execution-wbs.md): the procedures produced
> in P0 constrain every implementation phase from P1–P10; this phase produces no compiler code.

## Purpose and Scope

This document defines an executable procedure for determining **“where new code belongs, what form
it should take, and when work must stop for refactoring rather than continue with a patch.”** The
three prohibitions are:

1. **No ad hoc invention**—before introducing a concept, prove that it does not duplicate an
   existing concept.
2. **No accumulation of responsibilities**—a module has only one kind of responsibility; create a
   new module when adding another.
3. **No patching when refactoring is required**—every change must first pass through the decision
   procedure.

**Covered by this document**: the decision process for adding and modifying code, machine-checkable
rules, the code review checklist, and external convention references.

**Not covered by this document**: the four-layer model and directory structure (see
`01-routing.md`), stage contracts (see `02`), and specific refactoring designs (see `03`–`07`).

## Current State: Why This Mechanism Is Needed

This repository has **both mature countermeasures and the same syndrome, which has occurred at least
once each**. The following incidents have been confirmed:

| Incident                                                                                                                                                                                                                                                 | Scale                                                                                         | Violated prohibition                                          |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| 3 parallel type representations (`ast::Type` / `MonoType` / `ir::Type`) + serialized `type_table`, bridged by a hand-written synonym table `"Int" \| "int" \| "i64" => ...`                                                                              | `ir.rs:3` re-exports with `pub use`; `mono.rs:618-643` contains the synonym table             | Ad hoc invention + patch concealment                          |
| 2 parallel operator enums (`ast::BinOp/UnOp` vs `const_data::BinOp/UnOp`), with **no `From`/`TryFrom` at all**, distinguished through the import aliases `AstBinOp`/`CEBinOp`/`B`; the bytecode layer has a third set (`BinaryOp`/`UnaryOp`/`CompareOp`) | Aliasing starts in 5+ files; `const_eval.rs:185-216` contains a hand-written lossy conversion | Ad hoc invention                                              |
| 5 compilation entry points each manually wire together the stages                                                                                                                                                                                        | Behavior is inconsistent in 11 places                                                         | Patch concealment                                             |
| 8 test subtrees have never participated in compilation because the parent module omitted `mod tests;`                                                                                                                                                    | 1005 lines / 78 tests have never run                                                          | Responsibilities were not separated (tests had nowhere to go) |
| **`ir_gen.rs` is 8448 lines because it simultaneously handles 6 unrelated responsibilities** (symbol table / module-level globals / statement lowering / constant folding / loop iteration / call dispatch)                                              | —                                                                                             | **Accumulation of responsibilities**                          |
| **`From<BytecodeFile>` is 1400 lines because the entire file structure was stuffed into a single `From`**                                                                                                                                                | —                                                                                             | **Accumulation of responsibilities**                          |
| `include!` concatenates 1547 lines, and the file has no module identity (**resolved (P5 5.1)**)                                                                                                                                                          | The repository’s only `include!` has been eliminated                                          | Accumulation of responsibilities                              |
| **At the same time**, `build.rs` enforces error codes through a build-time hard gate and directly calls `panic!` on any inconsistency                                                                                                                    | 145 codes + a `--fix` remedy                                                                  | — This is a positive example                                  |

**Key observation**: positive examples and incidents coexist in the same repository. The difference
is not capability, but whether a mechanism exists to turn “taste” into machine-checkable rules.
`build.rs:19-55` has already proven that this path is feasible—**all this mechanism needs to do is
extend it**.

## Target Design

### Operational Definitions of the Three Prohibitions

Each prohibition is defined by **measurable criteria**, not slogans.

#### Prohibition One: No Ad Hoc Invention

Before adding any `pub` type, enum, constant table, or concept, all four criteria must pass:

| Criterion                                     | Content                                                                                                                                                                                       | Repository incident that motivated it                                                 |
| --------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| **A. Responsibility-overlap detection**       | The new item and an existing item have **at least half their variant names in common** and overlapping responsibilities → violation                                                           | 3 operator enums (ast / const_data / bytecode)                                        |
| **B. Insufficient call sites**                | The new `pub` item has < 2 call sites (only the definition site and one unique call site) → violation. There is no way to establish that it is a “concept” rather than a local implementation | —                                                                                     |
| **C. Disambiguating aliases required**        | Disambiguating semantically equivalent items requires aliases such as `use … as AstBinOp` / `CEBinOp` / `B` → violation                                                                       | `const_eval.rs:19,1061`, `ownership.rs:19`, `termination.rs:15,226`, `ir_gen.rs:2317` |
| **D. Differences bridged by a synonym table** | A new or modified item relies on hand-written string matching to bridge differences → violation; exhaustive matching or an explicit conversion is required                                    | `mono.rs:618-643`                                                                     |

> **External basis**: Go’s official Code Review Comments explicitly states: “**Do not define an
> interface before there is a real use case**—without a real use case, it is difficult to determine
> whether the interface is necessary, and even harder to determine which methods it should have”
> (https://go.dev/wiki/CodeReviewComments ). Another statement in the same document, “**Good package
> names should not require renaming**,” corresponds directly to criterion C.

#### Prohibition Two: No Accumulation of Responsibilities

> **Decision (2026-10-03, ChenXu233)**: **Do not impose line-count, size, or file-size gates.** The
> repository’s scale problems are not solved through numbers, but solely through **semantic
> separation**—that is, “one module has only one kind of responsibility.” The line-count ratchet
> proposal previously described in this section has been discarded.

The correct way to address scale problems is to **redraw responsibility boundaries**, not to place
numerical limits on those boundaries. Rationale:

- The reason `ir_gen.rs` reached 8448 lines is that it **simultaneously handled 6 unrelated
  responsibilities** (symbol table / module-level globals / statement lowering / constant folding /
  loop iteration / call dispatch). After splitting it into 6 modules, each naturally has only a few
  hundred lines—**without any line-count rule**.
- The reason `From<BytecodeFile>` reached 1400 lines is that **a single `From` was used to transfer
  the entire file structure field by field**. After changing it to 82 `decode_<op>` functions, it
  naturally becomes smaller.
- Conversely, setting thresholds produces two bad outcomes: either legitimizing 8448 lines by taking
  the maximum, or creating pressure for a one-time remediation by taking the median. **Both
  approaches treat the symptoms.**

| Criterion                                    | Content                                                                                                                                                                                                                          | Repository incident that motivated it                                                        |
| -------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| **A. Responsibility-category determination** | Does the new code belong to one of the target module’s **existing responsibility categories**? If it belongs to the second or a later category → violation; create a new module instead of continuing to add to the existing one | `ir_gen.rs` handles 6 responsibility categories in a single file                             |
| **B. Bypassing the authoritative source**    | A table entry is added without updating the sole authoritative source, by maintaining a hand-written copy that bypasses `code_tables` → violation                                                                                | Synonym table + parallel enums                                                               |
| **C. Boundary erosion**                      | The change introduces any cross-layer dependency (L2→L3, L4→L1/L2), an `include!`, or a cross-layer `pub(crate)` leak → violation                                                                                                | 2 instances of `operator_interfaces::spec()` and the sole `include!` (**resolved (P5 5.1)**) |

**Criterion A requires human judgment and cannot be automated**—which is exactly why it matters more
than a line-count rule: a line-count rule can be circumvented by “splitting it into two files of
4000 lines each,” whereas responsibility determination cannot.

Criterion C can be automated; see the section “Machine-Checkable Rules Checklist” below.

> **External basis**: Go’s official Code Review Comments explicitly rejects line-count rules:
> _“There is no rule saying ‘a function must not exceed N lines’… the solution is to change function
> boundaries, not start counting lines”_ (https://go.dev/wiki/CodeReviewComments#short-functions ).
> **This repository adopts that position**—the only difference is that we have confirmed that “the
> boundaries were drawn incorrectly” is the actual cause, so criterion A defines the boundaries
> rather than imposing a numerical threshold.

#### Prohibition Three: No Patching When Refactoring Is Required

| Criterion                     | Content                                                                                                                                                                       | Repository incident that motivated it |
| ----------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------- |
| **A. Synonymous duplication** | The same behavior occurs in ≥ 2 places (aliases / conversions / stage wiring / error mapping) → violation; it must be promoted to a shared layer                              | Import aliases, stage wiring          |
| **B. Excessive fan-out**      | A single modification must synchronously update ≥ 3 existing synonymous mappings → **classified as an architecture change**; stop work and follow the design-document process | Synonym table + 3 representations     |
| **C. New entry point**        | Adding an “Nth entry point/table entry” instead of registering it in the declarative stage table → violation                                                                  | 5 entry points with separate wiring   |

### Decision Procedure

**Every step is a decidable Boolean condition. Stop as soon as one matches.**

```
D0  门禁    本次改动是否触碰任何一张已存在的表（错误码/opcode/类型/入口阶段）？
             ├─ 是 → 能指出该表的唯一权威实现模块吗？
             │        ├─ 能 → 继续
             │        └─ 不能 → ⛔ 停止。先建立权威源
             └─ 否 → 继续
                    ↓
D1  防生造   去掉新概念后，能否用「既有概念 + 参数」表达同样的语义？
             ├─ 能 → ⛔ 禁止新增。理由记入 PR
             └─ 不能 → 继续
                    ↓
D2  防并行   新概念是否与某既有概念同语义（变体名重合 ≥ 半数）？
             ├─ 是 → 有 From/TryFrom 转换且机检吗？
             │        ├─ 有 → 继续
             │        └─ 无 → ⛔ 合并为一份（禁止用 import 别名区分）
             └─ 否 → 继续
                    ↓
D3  补丁?    依次判定，命中即止：
             1) 同一行为需在 ≥2 处复制？        → 是 → ⛔ 必须提到共享层
             2) 需要新增"第 6 个入口"？          → 是 → ⛔ 禁止手工接线，改为登记进阶段表
             3) 一次修改要改 ≥3 处同义映射？    → 是 → ⛔ 判定为架构变更，停工走设计文档
             4) 以上皆否                        → 允许局部补丁，但必须附带回归测试
                    ↓
D4  职责?    本次新增代码属于目标模块已有的职责类别吗？
             ├─ 否（第 2 类及以上）→ ⛔ 新建模块，或把该职责的现有代码一并搬过去
             └─ 是 → 合规
```

**Basis for D0**: `build.rs:19-55` has already implemented this pattern—`code_tables` is the
**sole** implementation point for parsing, validating, and comparing error codes. If `validate`
fails, compilation is rejected with `panic!`, accompanied by a `--fix` remediation command. **This
is not a new invention; it extends an existing pattern from “error codes only” to “all tables.”**

**External basis for D1/D2**: Go’s official “**Do not define an interface before there is a real use
case**” (https://go.dev/wiki/CodeReviewComments ) and “**Good package names should not require
renaming**” (https://go.dev/wiki/CodeReviewComments#imports ). D2 draws on RFC 2451’s uniqueness
principle (https://rust-lang.github.io/rfcs/2451-re-rebalancing-coherence.html ).

**D4 is pure human judgment and has no machine substitute**—which is exactly why it matters more
than a line-count rule: a line-count rule can be circumvented by “splitting it into two files of
4000 lines each,” whereas responsibility determination cannot. The reason `ir_gen.rs` reached 8448
lines is that it **simultaneously handled 6 unrelated responsibilities**; after splitting it into 6
modules, each naturally has only a few hundred lines, with no numerical rule required.

### Machine-Checkable Rules Checklist

| Rule                                                                      | Implementation                                                                                                                                                                                     | External reference                                                                                                        |
| ------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| **Table consistency** (error codes / opcodes / type tables / test wiring) | Extend the existing `build.rs` gate: add table types to `code_tables`; `parse` → `validate` → `panic!` on failure, with a `--fix` remedy                                                           | This repository’s `build.rs:19-55` is already an example                                                                  |
| **Consistent stage sequences for 5 entry points**                         | Add a **declarative stage table** (`stage_table.rs`); all 5 entry points obtain their sequence from it; tests assert that “the stage sequence produced by each of the 5 entry points == the table” | rustc’s declarative `rustc_queries!` query table (https://github.com/rust-lang/rustc-dev-guide/blob/master/src/query.md ) |
| **Parallel enums without conversions**                                    | CI uses `syn` to traverse every `enum`: if the Jaccard similarity of variant names with an existing enum is ≥ 0.5 and there is no `From`/`TryFrom` anywhere in the repository → fail               | RFC 2451’s uniqueness principle (https://rust-lang.github.io/rfcs/2451-re-rebalancing-coherence.html )                    |
| **Duplicate-concept detection**                                           | Scan the repository for `pub enum/struct` declarations with the same name and overlapping responsibilities; on a match, report “choose one or provide a rationale”                                 | Same as above                                                                                                             |
| **Uncompiled test trees**                                                 | Scan with `walkdir`: if a directory contains `#[cfg(test)]` or a test-module file but the parent module has no corresponding `mod` declaration → fail                                              | Same `stage_table` idea                                                                                                   |
| **Disambiguating import aliases**                                         | Count `use … as …` declarations; if the count exceeds the threshold, require a `// reason:` comment                                                                                                | Go’s “avoid renaming imports” (https://go.dev/wiki/CodeReviewComments#imports )                                           |
| **Boundary erosion** (prohibition two, criterion C)                       | Ban `include!`; the count of cross-layer `pub(crate)` leaks may **only decrease, never increase**; ban reverse L2→L3 `use` declarations                                                            | rustc’s rule that each provider has a unique owning crate                                                                 |

**The most important rule is “consistent stage sequences for 5 entry points”**—it directly turns
D3-2 from something that “people must remember” into something that “does not compile if the entry
is absent from the table.” rustc’s use of a declarative `rustc_queries!` query table in place of
sequential passes is the most mature solution to this problem
(https://github.com/rust-lang/rustc-dev-guide/blob/master/src/query.md ).

> **What must not be copied wholesale**: the core benefit of rustc’s query system is **incremental
> compilation and a dependency graph**. If YaoXiang has no need for incremental compilation, copying
> the entire query engine would be overengineering. **Borrow only two layers**: ① a declarative
> stage table; and ② a unique owning module for each stage.

### External Convention References

| Convention                                                                                                                                          | Source                                                                                | Applicability to this repository                                                                                                            |
| --------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| A compiler consists of a sequence of IRs distinguished by purpose (Token/AST/HIR/THIR/MIR/LLVM-IR), with each IR explicitly stating “why it exists” | rustc-dev-guide https://rustc-dev-guide.rust-lang.org/overview.html                   | **High**. It directly establishes the threshold for justifying “a fourth type representation must serve a new purpose”                      |
| Directory names correspond to pipeline stage names, and the same construct names are reused across stages                                           | Same as above                                                                         | **High**. The top level of `src/` should be named by stage, not by “tools/misc”                                                             |
| **Organize a compiler without sequential passes**, using a declarative query table + unique provider ownership                                      | rustc-dev-guide https://github.com/rust-lang/rustc-dev-guide/blob/master/src/query.md | **Highest**. This is the direct remedy for “5 entry points with separate hand-written wiring”                                               |
| A central context object holds all queries and caches                                                                                               | rustc-dev-guide                                                                       | Medium. It can serve as a reference for a unified compilation context, but the benefit depends on whether incremental compilation is needed |
| **Place interfaces in the consumer package, not the provider package**                                                                              | Go CodeReviewComments https://go.dev/wiki/CodeReviewComments#interfaces               | High. Suitable for determining which layer should own a new abstraction                                                                     |
| **Do not define an interface before there is a real use case**                                                                                      | Same as above                                                                         | **Highest**. This is the closest external expression of “no ad hoc invention”                                                               |
| **Avoid renaming imports for disambiguation**                                                                                                       | https://go.dev/wiki/CodeReviewComments#imports                                        | **Highest**. Directly addresses the `AstBinOp`/`CEBinOp`/`B` aliases                                                                        |
| Avoid meaningless names such as `util` / `common` / `misc` / `api`                                                                                  | https://go.dev/wiki/CodeReviewComments#package-names                                  | Medium                                                                                                                                      |
| **Assign mechanical problems to tools; documentation should cover only non-mechanical problems**                                                    | Opening of CodeReviewComments                                                         | High. Determines the boundary of which rules should be automated                                                                            |
| The language enforces uniqueness: coherence guarantees “exactly one impl for any given trait + type”                                                | RFC 2451 https://rust-lang.github.io/rfcs/2451-re-rebalancing-coherence.html          | **Highest** at the conceptual level. It turns “duplicate definition” from a review topic into a machine invariant                           |
| One stage per file                                                                                                                                  | Zig `lib/std/zig/` (https://github.com/ziglang/zig/tree/master/lib/std/zig )          | **Low**. See below                                                                                                                          |
| **Do not impose line-count or size rules**; “the solution is to change function boundaries, not start counting lines”                               | Go CodeReviewComments https://go.dev/wiki/CodeReviewComments#short-functions          | **Highest**. Adopted by this repository (2026-10-03 decision)                                                                               |

### What Must Not Be Copied Directly

1. **Zig’s “one stage per file.”** Zig’s stage division is clear (`Ast.zig` / `Parse.zig` /
   `AstGen.zig` / `Zir.zig`), but its **file sizes are completely out of control**—`AstGen.zig` is
   576 KB, `Zir.zig` is 210 KB, and `Ast.zig` is 148 KB
   (https://github.com/ziglang/zig/tree/master/lib/std/zig ). **“Organize by stage directory” can be
   borrowed, but “one stage per file” must not be.**
2. **Coherence / orphan rules.** RFC 2451 addresses **cross-crate** impl conflicts; this
   repository’s 3 type representations and 2 operator enums are duplicate definitions **within a
   single crate**, which language-level coherence will not report. **Custom CI checks are
   required.**

### Code Review Checklist

Each PR’s review must go through every item:

- [ ] **D0**: Does the change touch any existing table? Which module is that table’s sole
      authoritative implementation?
- [ ] **D1**: Can the new concept be expressed using an existing concept + parameters? If not, where
      is the rationale recorded?
- [ ] **D2**: Is the new concept semantically equivalent to an existing concept? Where is the
      conversion? Is an import alias required?
- [ ] **D3**: Does the same behavior occur in ≥ 2 places? Is a new entry point being added instead
      of registering it in the stage table? Must ≥ 3 synonymous mappings be changed?
- [ ] **D4**: Does the new code belong to one of the target module’s **existing responsibility
      categories**, or to the second or a later category?
- [ ] Have the **boundaries** been eroded? Have any cross-layer dependencies, `include!`
      declarations, or cross-layer `pub(crate)` leaks been added?
- [ ] Are **mechanical issues** already covered by tools (fmt / clippy / `build.rs` gates)? Is
      review effort confined to non-mechanical issues?

The final item directly follows Go’s division-of-labor principle: **assign mechanical problems to
tools and have reviews focus only on non-mechanical problems**
(https://go.dev/wiki/CodeReviewComments ).

## Detailed Design

### Deliverables

| File                                                                   | Content                                                                                                                         | Type                          |
| ---------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- | ----------------------------- |
| The “Code Placement and Change Procedure” section of `CONTRIBUTING.md` | D0–D4 decision procedure + the three prohibitions                                                                               | Documentation                 |
| `scripts/ci/check-concepts.py`                                         | Criteria A/B/C/D of prohibition one (parallel representations, disambiguating aliases, synonym tables, insufficient call sites) | Gate script                   |
| `scripts/ci/check-fanout.py`                                           | Criteria A/B/C of prohibition three                                                                                             | Gate script                   |
| `scripts/ci/check-boundary.py`                                         | Criterion C of prohibition two (ban `include!`, cross-layer `pub(crate)` leaks may only decrease, ban L2→L3)                    | Gate script                   |
| Extension of `tools/code-tables`                                       | D0 table-type registration                                                                                                      | Reuse existing infrastructure |

**There is no `baseline.toml`, no `clippy.toml` threshold configuration, and no `check-scale.py`.**
Scale problems are solved through responsibility separation (prohibition two, criterion A, human
judgment), not numerical gates.

### How Responsibility Categories Are Defined

Criterion D4-A requires the fact of “which responsibility categories this module already has.” The
approach is:

1. In the **target directory structure** in `01-routing.md`, each directory already specifies its
   responsibilities and boundary invariants—**this is the authoritative definition of responsibility
   categories**.
2. When submitting a PR, declare in the PR description which responsibility under the
   `<directory name>` the new code belongs to.
3. If the target directory’s responsibility list has no corresponding entry → this responsibility is
   a new category. **Either expand the responsibility list, with an explanation of why the code
   belongs there, or create a new directory.**

`check-boundary.py` handles the mechanical parts (cross-layer dependencies / `include!` /
`pub(crate)`), while responsibility assignment itself requires human judgment—which is why it must
be written into `CONTRIBUTING.md` rather than implemented in a script.

## Implementation Essentials

| Step | Content                                                                                                                                                      | Acceptance criterion                                                                                                                                                 |
| ---- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1    | Write D0–D4 and the three prohibitions into `CONTRIBUTING.md`; reference the directory responsibility table in `01-routing.md` for responsibility categories | Manual review approval                                                                                                                                               |
| 2    | Implement `check-concepts.py` (the four criteria of prohibition one), then run it against the **unmodified** code first                                      | **It should report the existing 3 operator enums and 2 parallel type representations, including the `ir::Type` alias**—the gate is effective only if it reports them |
| 3    | Implement `check-fanout.py` (prohibition three)                                                                                                              | **Deliberately adding a sixth entry-point-style wiring must fail**                                                                                                   |
| 4    | Implement `check-boundary.py` (prohibition two, criterion C)                                                                                                 | **Deliberately adding a reverse L2→L3 `use` declaration must fail**; record the initial `pub(crate)` leak count                                                      |
| 5    | Extend `tools/code-tables` to cover opcodes / type tables                                                                                                    | Deliberately introducing a zero-field variant must fail (depends on T1 in P6)                                                                                        |

**Step 2 is the core acceptance criterion for this phase**: `check-concepts.py` must report the
known 2 operator enum sets **against the current unmodified code**. If it cannot detect them, the
criteria were designed incorrectly and the gate is a false gate.

## Key Decisions and Rationale

- **Do not impose any line-count or size gates** (2026-10-03 decision)—adopt Go’s official position
  (https://go.dev/wiki/CodeReviewComments#short-functions ). The repository’s scale problems arise
  because `ir_gen.rs` simultaneously handles 6 responsibilities and `From<BytecodeFile>` stuffs the
  entire file structure into a single `From`—**remove the responsibilities and the numbers naturally
  become smaller; no numerical rule is needed**. The original `baseline.toml` ratchet and
  `clippy.toml` threshold ideas have both been discarded.
- **The authoritative definition of responsibility categories belongs in the directory
  responsibility table in `01-routing.md`, not in this document**—to prevent the two definitions
  from drifting apart.
- **Borrow only two layers of rustc queries, not the engine**—a declarative stage table + unique
  provider ownership. The core benefit of the query engine is incremental compilation, which this
  repository does not need.
- **The coherence principle requires a custom implementation**—RFC 2451 governs cross-crate cases,
  whereas this repository’s problem is duplication within a single crate, which the language will
  not report.

### Rejected Directions

- **Line-count ratchet / `baseline.toml`**—the repository’s scale problems arise from inadequate
  responsibility separation; setting thresholds would merely legitimize 8448 lines or create
  pressure for a one-time remediation. **Discarded.**
- **The `clippy.toml` `too-many-lines-threshold` / clang-tidy `readability-function-size`**—same as
  above. **Discarded.**
- **Copying the rustc query engine**—overengineering; borrow only the declarative table.

## Known Limitations and Risks

- **False positives in the criteria**: criterion A of prohibition one uses “at least half of variant
  names overlap,” which is heuristic; two genuinely unrelated enums may have colliding names, such
  as `Field`. Mitigation: allow exemptions through `// reason:` comments.
- **Accuracy of `syn` analysis**: `syn` has limited ability to analyze forms after macro expansion,
  so macro-generated definitions may be missed. Mitigation: the gate reports only “suspected” cases
  for human adjudication.
- **Responsibility determination cannot be automated**: criterion A of prohibition two requires
  human judgment. This means that **it can only be enforced through review and cannot be enforced by
  CI**—the mechanism’s greatest weakness. Mitigation: write the responsibility list into
  `01-routing.md` so it can be checked; have `check-boundary.py` cover the mechanical parts
  (cross-layer dependencies / `include!` / `pub(crate)` leaks).
- **The gate may be circumvented**: a `// reason:` comment is a legitimate escape hatch. Mitigation:
  include the number of exemptions in the report from `check-boundary.py`.

> **All open questions originally listed in this section have been adjudicated.** The item-by-item
> decisions are recorded in the
> [RFC-039 Decision Registry](../../rfc/accepted/039-compiler-architecture.md) (D1–D50). **No
> pending items remain in this document.**

## See Also

- [01-routing.md](01-routing.md) — **Authoritative definition of responsibility categories**
  (directory responsibilities and boundary-invariant table), dependency-direction rules, target
  directory structure, and routing table C
- [02-stage-contract.md](02-stage-contract.md) — Declarative `Stage` table (landing point for
  D0/D3-2)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — Historical incidents to be cleaned up by this
  procedure
- [09-execution-wbs.md](09-execution-wbs.md) — Three-level task breakdown for P0
- [RFC-009a Borrow Proof Pipeline](../../rfc/accepted/009a-borrow-proof-pipeline.md)
- [RFC-013 Error Code Specification](../../rfc/accepted/013-error-code-specification.md) — Normative
  source for the existing gates
- `build.rs:19-55` — The repository’s existing `panic!`-level gate example and the direct basis for
  the D0/D0 gate pattern
