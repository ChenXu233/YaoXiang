# Repository Maintenance Mechanism and Code Placement Decision Procedure

> **The rule body has been migrated (2026-10-05)**: For the precise rule descriptions of the three
> prohibitions, D0–D4, red lines, and review checklist, see
> [coding-rules.md](../coding-rules.md)—that is the sole long-lived authority. This document is
> retained as the **October 2026 diagnosis and decision record**: incident evidence, line numbers,
> references to external conventions, and the reasoning for rejected options. It lives and dies
> together with RFC-039. When enforcing, cite `coding-rules.md`; do not cite this document as the
> source of rules.

> **Subsidiary design document**. This document is an appendix to
> [RFC-039 Compiler Architecture Refactor](../../rfc/draft/039-compiler-architecture.md),
> corresponding to **Stage P0**.
>
> This document pairs with [09-execution-wbs.md](09-execution-wbs.md): the procedure produced by P0
> constrains every construction stage of P1–P10; no compiler code is produced at this stage.

## Positioning and Scope

This document defines the **executable procedure** for "where new code goes, what shape it takes,
and under what circumstances one must stop and refactor rather than patch." The three prohibitions:

1. **No fabrication**—Before adding a new concept, you must prove it does not duplicate an existing
   one
2. **No responsibility accumulation**—A module bears only one class of responsibility; to add a new
   responsibility, create a new module
3. **Refactor needed, but a patch was applied**—Every change must first pass the decision procedure

**This document covers**: decision flow for new/modified code, machine-checkable rules, code review
checklist, references to external conventions.

**This document does not cover**: the four-layer model and directory structure (see
`01-routing.md`), stage contracts (see `02`), or specific refactor designs (see `03`–`07`).

## Current State: Why This Mechanism Is Needed

This repository has **mature countermeasures, but the same symptoms have each appeared at least
once**. Confirmed incidents:

| Incident                                                                                                                                                                                                                                         | Scale                                                                    | Prohibition Violated                              |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------ | ------------------------------------------------- |
| 3 parallel type representations (`ast::Type` / `MonoType` / `ir::Type`) + serialization `type_table`, bridged by hand-written synonym tables `"Int" \| "int" \| "i64" => ...`                                                                    | `ir.rs:3` re-exports via `pub use`; `mono.rs:618-643` synonym table      | Fabrication + patch concealment                   |
| 2 parallel operator enums (`ast::BinOp/UnOp` vs `const_data::BinOp/UnOp`), with **no `From`/`TryFrom` at all**, distinguished via import aliases `AstBinOp`/`CEBinOp`/`B`; the bytecode layer has a third set (`BinaryOp`/`UnaryOp`/`CompareOp`) | 5+ files aliasing; `const_eval.rs:185-216` hand-written lossy conversion | Fabrication                                       |
| 5 compilation entry points each hand-wired with stages                                                                                                                                                                                           | 11 inconsistent behaviors                                                | Patch concealment                                 |
| 8 test subtrees never participating in compilation (parent module missing `mod tests;`)                                                                                                                                                          | 1005 lines / 78 tests never run                                          | Responsibility not separated (no place for tests) |
| **`ir_gen.rs` 8448 lines, because it simultaneously bears 6 unrelated classes of responsibility** (symbol table / module-level globals / statement lowering / constant folding / loop iteration / call dispatch)                                 | —                                                                        | **Responsibility accumulation**                   |
| **`From<BytecodeFile>` 1400 lines, because the entire file structure was stuffed into one `From`**                                                                                                                                               | —                                                                        | **Responsibility accumulation**                   |
| `include!` concatenation of 1547 lines, file has no module identity                                                                                                                                                                              | The only `include!` in the whole repo                                    | Responsibility accumulation                       |
| **And simultaneously**: `build.rs` enforces a build-time hard gate on error codes, panicking on inconsistency                                                                                                                                    | 145 codes + `--fix` cure                                                 | — This is a positive sample                       |

**Key observation**: positive samples and incidents coexist in the same repository. The difference
is not capability, but whether a mechanism exists to turn "taste" into machine-checkable rules.
`build.rs:19-55` has already proven this path feasible—**all this mechanism does is generalize it**.

## Target Design

### Operational Definitions of the Three Prohibitions

Each prohibition comes with **measurable criteria**, not slogans.

#### Prohibition 1: No Fabrication

Before adding any new `pub` type / enum / constant table / concept, all four criteria must pass for
compliance:

| Criterion                               | Content                                                                                                                                                                | Repo Incident Basis                                                                   |
| --------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| **A. Responsibility Overlap Detection** | If the new item's **variant names overlap ≥ half** with some existing item and responsibilities overlap → violation                                                    | 3 operator enums (ast / const_data / bytecode)                                        |
| **B. Insufficient Call Sites**          | The new `pub` item has < 2 call sites (only the definition site + the single call site) → violation. Cannot prove it is a "concept" rather than a local implementation | —                                                                                     |
| **C. Disambiguation Alias Required**    | Aliases like `use … as AstBinOp` / `CEBinOp` / `B` are needed to distinguish semantically equivalent items → violation                                                 | `const_eval.rs:19,1061`, `ownership.rs:19`, `termination.rs:15,226`, `ir_gen.rs:2317` |
| **D. Bridged by Synonym Table**         | Adding/modifying items relies on hand-written string matching to bridge differences → violation; must be changed to exhaustive matching or explicit conversion         | `mono.rs:618-643`                                                                     |

> **External reference**: Go's official Code Review Comments explicitly state "**don't define
> interfaces before you have a real use case**—without a real use case, it's hard to judge whether
> the interface is necessary and which methods it should have"
> (https://go.dev/wiki/CodeReviewComments ). Another item in the same document, "**good package
> names need not be renamed**," corresponds directly to criterion C.

#### Prohibition 2: No Responsibility Accumulation

> **Decision (2026-10-03, ChenXu233)**: **No line count / volume / file size gates.** The scale
> problem in this repository is not solved by numbers, but by **semantic separation**—that is, "a
> module bears only one class of responsibility." The originally proposed line-count ratchet scheme
> is abandoned.

The correct handling of scale problems is to **redraw responsibility boundaries**, not to set
upper-bound numbers on boundaries. Reasons:

- `ir_gen.rs` at 8448 lines is caused by its **simultaneous bearing of 6 unrelated classes of
  responsibility** (symbol table / module-level globals / statement lowering / constant folding /
  loop iteration / call dispatch). After splitting into 6 modules, each is naturally only a few
  hundred lines—**no line-count rule needed**.
- `From<BytecodeFile>` at 1400 lines is caused by **one `From` stuffing the entire file structure's
  per-field transfer**. After changing to 82 `decode_<op>` functions, it naturally becomes small.
- Conversely, setting thresholds produces two bad outcomes: either legitimizing 8448 lines (taking
  the maximum), or producing one-shot remediation pressure (taking the median). **Both treat the
  symptom.**

| Criterion                             | Content                                                                                                                                                                                | Repo Incident Basis                                                 |
| ------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------- |
| **A. Responsibility Class Judgment**  | Does the new code belong to an **existing responsibility class** of this module? If it is the 2nd class or above → violation; should create a new module rather than continue stuffing | `ir_gen.rs` single file bearing 6 classes of responsibility         |
| **B. Bypassing the Authority Source** | Adding table entries without updating the sole authority source (bypassing `code_tables` with hand-written copies) → violation                                                         | Synonym table + parallel enums                                      |
| **C. Boundary Erosion**               | The change introduced any cross-layer dependency (L2→L3, L4→L1/L2), `include!`, or `pub(crate)` cross-layer leakage → violation                                                        | 2 occurrences of `operator_interfaces::spec()`, the only `include!` |

**Criterion A is a human judgment, not machine-checkable**—this is precisely why it is more
important than line-count rules: line-count rules can be bypassed by "splitting into two 4000-line
files"; responsibility judgment cannot.

Criterion C is machine-checkable; see the section "Machine-Checkable Rule List" below.

> **External reference**: Go's official Code Review Comments explicitly oppose line-count rules:
> _"There is no rule that 'a function must not exceed N lines'... the solution is to change function
> boundaries, not to start counting lines"_ (https://go.dev/wiki/CodeReviewComments#short-functions
> ). **This repository adopts that position**—the only difference is that we have already confirmed
> "poorly drawn boundaries" as the true cause, so the boundary definition is written into criterion
> A rather than a line-count threshold.

#### Prohibition 3: Refactor Needed, but a Patch Was Applied

| Criterion                     | Content                                                                                                                                                                       | Repo Incident Basis               |
| ----------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------- |
| **A. Synonymous Duplication** | The same behavior appears in ≥ 2 places (aliasing / conversion / stage wiring / error mapping) → violation; must be lifted to a shared layer                                  | Import aliases, stage wiring      |
| **B. Excessive Fan-out**      | A single change requires synchronously modifying ≥ 3 existing synonymous mappings → **classified as an architectural change**, stop work, go through the design document flow | Synonym table + 3 representations |
| **C. New Entry Point**        | Adding "the Nth entry/table item" instead of registering it in the declarative stage table → violation                                                                        | 5 entry points each hand-wired    |

### Decision Procedure

**Each step is a decidable boolean condition. Stop on first hit.**

```
D0  Gate     Does this change touch any existing table (error codes / opcodes / types / entry stages)?
              ├─ Yes → Can you point to the sole authoritative implementation module of that table?
              │         ├─ Yes → Continue
              │         └─ No  → ⛔ Stop. First establish the authority source
              └─ No  → Continue
                     ↓
D1  No-fabricate  After removing the new concept, can the same semantics be expressed
                  with "existing concept + parameters"?
                  ├─ Yes → ⛔ Forbidden to add. Reason recorded in PR
                  └─ No  → Continue
                          ↓
D2  No-parallel   Is the new concept semantically equivalent to some existing concept
                  (variant name overlap ≥ half)?
                  ├─ Yes → Is there a From/TryFrom conversion with machine check?
                  │         ├─ Yes → Continue
                  │         └─ No  → ⛔ Merge into one (forbidden to use import aliases to distinguish)
                  └─ No  → Continue
                          ↓
D3  Patch?        Judge in order, stop on first hit:
                  1) Same behavior must be duplicated in ≥ 2 places?   → Yes → ⛔ Must lift to shared layer
                  2) Need to add "the 6th entry point"?               → Yes → ⛔ Forbidden to hand-wire, must register in stage table
                  3) A single change must modify ≥ 3 synonymous mappings? → Yes → ⛔ Classified as architectural change, stop and go through design document
                  4) None of the above                                → Local patch allowed, but must include regression tests
                          ↓
D4  Responsibility?  Does the new code added this time belong to an existing responsibility
                     class of the target module?
                     ├─ No (2nd class or above) → ⛔ Create a new module, or move the existing
                     │                             code for that responsibility over together
                     └─ Yes → Compliant
```

**Basis for D0**: `build.rs:19-55` has already implemented this pattern—`code_tables` is the
**sole** implementation location for error code parsing/validation/comparison; if `validate` fails,
the build panics, and a `--fix` cure command is provided. **This is not a new invention, but the
generalization of an existing pattern from "covering only error codes" to "covering all tables."**

**External basis for D1/D2**: Go's official "**don't define interfaces before you have a real use
case**" (https://go.dev/wiki/CodeReviewComments ) and "**good package names need not be renamed**"
(https://go.dev/wiki/CodeReviewComments#imports ). D2 borrows the uniqueness idea from RFC 2451
(https://rust-lang.github.io/rfcs/2451-re-rebalancing-coherence.html ).

**D4 is purely human judgment with no machine substitute**—this is why it is more important than
line-count rules: line-count rules can be bypassed by "splitting into two 4000-line files";
responsibility judgment cannot. The cause of `ir_gen.rs` at 8448 lines is that it **simultaneously
bears 6 unrelated classes of responsibility**; after splitting into 6 modules, each is naturally
only a few hundred lines, requiring no numerical rule.

### Machine-Checkable Rule List

| Rule                                                                     | Implementation                                                                                                                                                                        | External Reference                                                                                                        |
| ------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| **Table Consistency** (error codes / opcodes / type table / test wiring) | Extend existing `build.rs` gate: add table types to `code_tables`, `parse` → `validate` → panic on failure, with `--fix`                                                              | This repo's `build.rs:19-55` is already an example                                                                        |
| **5 entry-point stage sequence consistency**                             | Add **declarative stage table** (`stage_table.rs`); all 5 entry points uniformly take their sequences from it; test asserts "the stage sequences produced by 5 entry points == table" | rustc's `rustc_queries!` declarative query table (https://github.com/rust-lang/rustc-dev-guide/blob/master/src/query.md ) |
| **Parallel enums missing conversions**                                   | CI uses `syn` to traverse all `enum`s: if Jaccard similarity of variant names with an existing enum ≥ 0.5 and no `From`/`TryFrom` exists repo-wide → fail                             | Uniqueness idea from RFC 2451 (https://rust-lang.github.io/rfcs/2451-re-rebalancing-coherence.html )                      |
| **Duplicate concept detection**                                          | Scan all repo `pub enum/struct`s for same names and responsibility overlap; on hit, report "choose one or write a reason"                                                             | Same as above                                                                                                             |
| **Uncompiled test trees**                                                | `walkdir` scan: directory contains `#[cfg(test)]` or test module files, but the parent module has no corresponding `mod` declaration → fail                                           | Same approach as `stage_table`                                                                                            |
| **Disambiguating import aliases**                                        | Count `use … as …`; exceeding the threshold requires a `// reason:` comment                                                                                                           | Go "avoid renaming imports" (https://go.dev/wiki/CodeReviewComments#imports )                                             |
| **Boundary Erosion** (Prohibition 2, Criterion C)                        | Forbid `include!`; `pub(crate)` cross-layer leakage count **may only decrease, never increase**; forbid L2→L3 reverse `use`                                                           | rustc's provider sole-ownership crate                                                                                     |

**The most important one is "5 entry-point stage sequence consistency"**—it directly turns D3-2 from
"depends on people remembering" into "not in the table, won't compile." rustc uses `rustc_queries!`
declarative query table to replace sequential passes, the most mature solution to this problem
(https://github.com/rust-lang/rustc-dev-guide/blob/master/src/query.md ).

> **Parts that cannot be directly copied**: The core benefit of the rustc query system is
> **incremental compilation and dependency graphs**. Without incremental compilation needs, copying
> the entire query engine would be over-engineering for YaoXiang. **Only borrow two layers**: ①
> declarative stage table; ② each stage has a sole-ownership module.

### External Convention References

| Convention                                                                                                                  | Source                                                                                | Applicability to This Repo                                                                                            |
| --------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| Compiler = a sequence of IRs distinguished by purpose (Token/AST/HIR/THIR/MIR/LLVM-IR), each IR has a clear "why it exists" | rustc-dev-guide https://rustc-dev-guide.rust-lang.org/overview.html                   | **High**. Directly provides the threshold "a 4th type representation must justify which new purpose it serves"        |
| Directory names = pipeline stage names; reuse the same constructor name across stages                                       | Same as above                                                                         | **High**. The `src/` top level should be named by stage, not "tools/miscellaneous"                                    |
| **Don't organize a compiler with sequential passes**; use declarative query table + sole-ownership providers                | rustc-dev-guide https://github.com/rust-lang/rustc-dev-guide/blob/master/src/query.md | **Highest**. This is exactly the right remedy for "5 entry points each hand-wired"                                    |
| Central context object holds all queries and caches                                                                         | rustc-dev-guide                                                                       | Medium. Can serve as unified compilation context reference; benefit depends on whether incremental compilation exists |
| **Interfaces go in the consumer package, not the implementer package**                                                      | Go CodeReviewComments https://go.dev/wiki/CodeReviewComments#interfaces               | High. Suitable for judging "which layer a new abstraction should sink to"                                             |
| **Don't define interfaces before you have a real use case**                                                                 | Same as above                                                                         | **Highest**. This is the most fitting external expression of "no fabrication"                                         |
| **Avoid renaming imports for disambiguation**                                                                               | https://go.dev/wiki/CodeReviewComments#imports                                        | **Highest**. Directly targets the `AstBinOp`/`CEBinOp`/`B` aliases                                                    |
| Avoid meaningless names like `util` / `common` / `misc` / `api`                                                             | https://go.dev/wiki/CodeReviewComments#package-names                                  | Medium                                                                                                                |
| **Mechanical issues go to tools, documentation only handles non-mechanical issues**                                         | CodeReviewComments opening                                                            | High. Determines the boundary of "which rules should be machine-checked"                                              |
| Uniqueness enforced by language: coherence guarantees "for any trait+type, there is exactly one impl"                       | RFC 2451 https://rust-lang.github.io/rfcs/2451-re-rebalancing-coherence.html          | **Highest** (conceptual level). Turn "duplicate definitions" from a review topic into a machine invariant             |
| One stage, one file                                                                                                         | Zig `lib/std/zig/` (https://github.com/ziglang/zig/tree/master/lib/std/zig )          | **Low**. See below                                                                                                    |
| **No line count / volume rules**; "the solution is to change function boundaries, not to start counting lines"              | Go CodeReviewComments https://go.dev/wiki/CodeReviewComments#short-functions          | **Highest**. This repo has adopted (2026-10-03 decision)                                                              |

### What Cannot Be Directly Copied

1. **Zig's "one stage, one file"**. Zig's stage division is clear (`Ast.zig` / `Parse.zig` /
   `AstGen.zig` / `Zir.zig`), but **file size is completely out of control**—`AstGen.zig` 576 KB,
   `Zir.zig` 210 KB, `Ast.zig` 148 KB (https://github.com/ziglang/zig/tree/master/lib/std/zig ).
   **"Divide directories by stage" can be borrowed; "one stage, one file" cannot.**
2. **coherence / orphan rules**. RFC 2451 solves **cross-crate** impl conflicts; the 3 type
   representations and 2 operator enums in this repo are **single-crate** duplications;
   language-level coherence will not error—**a self-built CI check is required**.

### Code Review Checklist

Each PR's review goes through these one by one:

- [ ] **D0**: Does it touch any table? Which is the sole authoritative implementation module of that
      table?
- [ ] **D1**: Can the new concept be expressed with existing concept + parameters? If not, where is
      the reason written?
- [ ] **D2**: Is it semantically equivalent to an existing concept? Where is the conversion? Are
      import aliases required?
- [ ] **D3**: Does the same behavior appear in ≥ 2 places? Did you add an entry point instead of
      registering in the stage table? Do you need to modify ≥ 3 synonymous mappings?
- [ ] **D4**: Does the new code added this time belong to an **existing responsibility class** of
      the target module? Or is it the 2nd class or above?
- [ ] **Boundary** erosion? New cross-layer dependencies / `include!` / `pub(crate)` cross-layer
      leakage?
- [ ] **Mechanical issues** already covered by tools (fmt / clippy / `build.rs` gates)? Does the
      review only spend time on non-mechanical issues?

The last item directly borrows Go's division of labor: **mechanical issues go to tools, review only
looks at non-mechanical issues** (https://go.dev/wiki/CodeReviewComments ).

## Detailed Design

### Deliverables List

| File                                                              | Content                                                                                                                  | Type           |
| ----------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ | -------------- |
| `CONTRIBUTING.md`'s "Code Placement and Change Procedure" section | D0–D4 decision procedure + three prohibitions                                                                            | Documentation  |
| `scripts/ci/check-concepts.py`                                    | Prohibition 1's A/B/C/D criteria (parallel representation, disambiguation alias, synonym table, insufficient call sites) | Gate script    |
| `scripts/ci/check-fanout.py`                                      | Prohibition 3's A/B/C criteria                                                                                           | Gate script    |
| `scripts/ci/check-boundary.py`                                    | Prohibition 2's Criterion C (forbid `include!`, `pub(crate)` cross-layer leakage may only decrease, forbid L2→L3)        | Gate script    |
| `tools/code-tables` extension                                     | D0's table type registration                                                                                             | Reuse existing |

**No `baseline.toml`, no `clippy.toml` threshold configuration, no `check-scale.py`.** Scale
problems are solved by responsibility separation (Prohibition 2, Criterion A, human judgment); no
numerical gates.

### How Responsibility Classes Are Defined

D4 Criterion A requires the fact "what responsibility classes does this module already have."
Approach:

1. In `01-routing.md`'s **target directory structure**, each directory has already written its
   responsibility and boundary invariants—**this is the authoritative definition of responsibility
   classes**
2. When submitting a PR, declare in the PR description "which responsibility of `<directory name>`
   does the new code added this time belong to"
3. If the target directory's responsibility list has no corresponding entry → this responsibility is
   a new class; **either expand the responsibility list (and explain why it belongs here), or create
   a new directory**

`check-boundary.py` handles the mechanical part (cross-layer dependencies / `include!` /
`pub(crate)`); responsibility attribution itself is judged by humans—this is precisely why it needs
to be written into `CONTRIBUTING.md` rather than a script.

## Implementation Key Points

| Step | Content                                                                                                                                          | Acceptance                                                                                                                                                      |
| ---- | ------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1    | Write D0–D4 and the three prohibitions into `CONTRIBUTING.md`; responsibility classes reference `01-routing.md`'s directory responsibility table | Manual review passed                                                                                                                                            |
| 2    | Implement `check-concepts.py` (Prohibition 1's four criteria); first run on **unchanged** code                                                   | **Should report the existing 3 operator enums and 2 parallel type representations (`ir::Type` aliases counted in)—reporting them proves the gate is effective** |
| 3    | Implement `check-fanout.py` (Prohibition 3)                                                                                                      | **Deliberately adding a 6th entry-point-style wiring must turn red**                                                                                            |
| 4    | Implement `check-boundary.py` (Prohibition 2, Criterion C)                                                                                       | **Deliberately adding a reverse L2→L3 `use` must turn red**; `pub(crate)` leakage count records initial value                                                   |
| 5    | Extend `tools/code-tables` to cover opcodes / type tables                                                                                        | Deliberately introducing a zero-constructor variant must turn red (depends on P6's T1)                                                                          |

**Step 2 is the core acceptance for this stage**: `check-concepts.py` must be able to **report the
known 2 operator enums on the current unchanged code**. If it cannot report them, the criteria are
wrongly designed, and the gate is a false gate.

## Key Decisions and Reasons

- **No line count / volume gates whatsoever** (2026-10-03 decision)—adopting Go's official position
  (https://go.dev/wiki/CodeReviewComments#short-functions ). The cause of this repo's scale problem
  is `ir_gen.rs` simultaneously bearing 6 classes of responsibility, and `From<BytecodeFile>`
  stuffing the entire file structure into one `From`—**remove the responsibilities, numbers
  naturally shrink, no numerical rule needed**. The originally envisioned `baseline.toml` ratchet
  and `clippy.toml` thresholds are abandoned.
- **The authoritative definition of responsibility classes is placed in `01-routing.md`'s directory
  responsibility table**, not here—to avoid definition drift in two places.
- **Only borrow two layers from rustc queries, not the engine**—declarative stage table +
  sole-ownership providers. The core benefit of the query engine is incremental compilation; this
  repo has no such need.
- **coherence idea requires self-built implementation**—RFC 2451 governs cross-crate; this repo's
  problem is single-crate duplication, the language will not error.

### Directions Not Adopted

- **Line count ratchet / `baseline.toml`**—The cause of this repo's scale problem is responsibility
  non-separation; setting a threshold would only legitimize 8448 lines or produce one-shot
  remediation pressure. **Abandoned.**
- **`clippy.toml`'s `too-many-lines-threshold` / clang-tidy `readability-function-size`**—Same as
  above, **abandoned**.
- **Copying the rustc query engine directly**—Over-engineering; only borrow the declarative table.

## Known Limitations and Risks

- **False positives of criteria**: Prohibition 1 Criterion A uses "variant name overlap ≥ half" as a
  heuristic; two truly unrelated enums may collide on names (e.g., `Field`). Mitigation: allow
  `// reason:` comments for exemption.
- **Precision of `syn` analysis**: `syn`'s parsing of macro-expanded forms is limited; definitions
  generated across macros may be missed. Mitigation: the gate only reports "suspected," judged by
  humans.
- **Responsibility judgment is not machine-checkable**: Prohibition 2 A is a human judgment. This
  means **it can only be enforced by review, not by CI**—this is the mechanism's biggest soft spot.
  Mitigation: write the responsibility list into `01-routing.md` to make it verifiable;
  `check-boundary.py` covers the mechanical part (cross-layer dependencies / `include!` /
  `pub(crate)` leakage).
- **The gate can be bypassed**: `// reason:` comments are a legitimate escape hatch. Mitigation: the
  number of exemptions itself goes into `check-boundary.py`'s report.

> **The open questions originally listed in this section have all been adjudicated.** Item-by-item
> decisions are in [RFC-039 Decision Registry](../../rfc/draft/039-compiler-architecture.md)
> (D1–D50). **This document leaves no pending items.**

## See Also

- [01-routing.md](01-routing.md) — **Authoritative definition of responsibility classes** (directory
  responsibilities and boundary invariant table), dependency direction specification, target
  directory structure, routing table C
- [02-stage-contract.md](02-stage-contract.md) — `Stage` declarative table (landing point of
  D0/D3-2)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — Historical incidents this procedure is meant
  to clean up
- [09-execution-wbs.md](09-execution-wbs.md) — P0's three-level task breakdown
- [RFC-009a Borrow Proof Pipeline](../../rfc/accepted/009a-borrow-proof-pipeline.md)
- [RFC-013 Error Code Specification](../../rfc/accepted/013-error-code-specification.md) — Source
  specification for the existing gate
- `build.rs:19-55` — This repo's existing `panic!`-level gate example, direct basis for the D0 gate
  pattern
