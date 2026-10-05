# Repository Maintenance Mechanism and Code Placement Decision Procedure

> **The canonical rules have been migrated (2026-10-05)**: The **precise rule descriptions** for the
> three prohibitions, D0–D4, red lines, and the review checklist are in
> [coding-rules.md](../../dev/coding-rules.md) — that document is the sole long-lived authority.
> This document is preserved as the **2026-10 diagnosis and decision record**: incident evidence,
> line numbers, references to external conventions, and rationales for rejected proposals; it lives
> and dies with RFC-039. When executing, cite coding-rules.md; do not cite this document as the
> source of rules.

> **Subsidiary design document**. This document is a subsidiary of
> [RFC-039 Compiler Architecture Refactor](../rfc/draft/039-compiler-architecture.md), corresponding
> to **Phase P0**.
>
> This document is paired with [09-execution-wbs.md](09-execution-wbs.md): the procedures produced
> in P0 constrain every construction phase from P1 to P10; this phase produces no compiler code.

## Position and Scope

This document defines an executable procedure for **"where new code goes, what shape it takes, and
when one must stop and refactor rather than apply a patch"**. Three prohibitions:

1. **No invention** — before adding a new concept, you must prove it does not duplicate an existing
   one.
2. **No responsibility accumulation** — a module takes on only one class of responsibility; to add a
   new responsibility, create a new module.
3. **Should refactor but patched instead** — every change must first pass the decision procedure.

**This document covers**: the decision flow for adding/modifying code, machine-checkable rules, the
code review checklist, and references to external conventions.

**This document does not cover**: the four-layer model and directory structure (see
`01-routing.md`), stage contracts (see `02`), or specific refactoring designs (see `03`–`07`).

## Current State: Why This Mechanism Is Needed

This repository has **mature countermeasures, yet each of the same symptoms has occurred at least
once**. Verified incidents:

| Incident                                                                                                                                                                                                                                              | Scale                                                                      | Prohibition violated                                |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------- | --------------------------------------------------- |
| 3 parallel type representations (`ast::Type` / `MonoType` / `ir::Type`) + serialized `type_table`, bridged by a hand-written synonym table `"Int" \| "int" \| "i64" => ...`                                                                           | `ir.rs:3` re-exports via `pub use`; `mono.rs:618-643` synonym table        | Invention + patch concealment                       |
| 2 parallel operator enums (`ast::BinOp/UnOp` vs `const_data::BinOp/UnOp`), **no `From`/`TryFrom` at all**, distinguished only by import aliases `AstBinOp`/`CEBinOp`/`B`; a third set exists in the bytecode layer (`BinaryOp`/`UnaryOp`/`CompareOp`) | Aliases in 5+ files; `const_eval.rs:185-216` hand-written lossy conversion | Invention                                           |
| 5 compile entry points each hand-wire stages                                                                                                                                                                                                          | 11 behavioral inconsistencies                                              | Patch concealment                                   |
| 8 test subtrees never participate in compilation (parent module missing `mod tests;`)                                                                                                                                                                 | 1005 lines / 78 tests never run                                            | Responsibilities not separated (no place for tests) |
| **`ir_gen.rs` at 8448 lines, because it simultaneously bears 6 unrelated responsibilities** (symbol table / module-level globals / statement lowering / constant folding / loop iteration / call dispatch)                                            | —                                                                          | **Responsibility accumulation**                     |
| **`From<BytecodeFile>` at 1400 lines, because the entire file structure was stuffed into a single `From`**                                                                                                                                            | —                                                                          | **Responsibility accumulation**                     |
| `include!` concatenation producing 1547 lines, with the file having no module identity                                                                                                                                                                | The only `include!` in the entire repository                               | Responsibility accumulation                         |
| **At the same time**: `build.rs` enforces a build-time hard gate on error codes, `panic!` on inconsistency                                                                                                                                            | 145 codes + `--fix` remedy                                                 | — This is a positive sample                         |

**Key observation**: positive samples and incidents coexist in the same repository. The difference
is not capability, but whether a mechanism exists to turn "taste" into machine-checkable rules.
`build.rs:19-55` has already proven this path viable — **what this mechanism does is simply
generalize it**.

## Target Design

### Operational Definitions of the Three Prohibitions

Each prohibition is given **measurable criteria**, not slogans.

#### Prohibition 1: No Invention

Before adding any `pub` type / enum / constant table / concept, all four criteria must pass for
compliance:

| Criterion                               | Content                                                                                                                                                              | Repository incident it derives from                                                   |
| --------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| **A. Responsibility overlap detection** | The new item and an existing item have **≥ half of their variant names overlap** and overlapping responsibilities → violation                                        | 3 operator enums (ast / const_data / bytecode)                                        |
| **B. Insufficient call sites**          | The new `pub` item has < 2 call sites (only the definition site + a single call site) → violation. Cannot prove it is a "concept" rather than a local implementation | —                                                                                     |
| **C. Disambiguating aliases required**  | Aliases such as `use … as AstBinOp` / `CEBinOp` / `B` are needed to distinguish same-semantic items → violation                                                      | `const_eval.rs:19,1061`, `ownership.rs:19`, `termination.rs:15,226`, `ir_gen.rs:2317` |
| **D. Bridged by a synonym table**       | The new/modified item relies on hand-written string matching to bridge differences → violation; must be replaced with exhaustive matching or an explicit conversion  | `mono.rs:618-643`                                                                     |

> **External basis**: Go's official Code Review Comments explicitly states "**don't define
> interfaces before you have a real use case** — without a concrete use case, it's hard to tell
> whether an interface is necessary, let alone what methods it should have**
> (https://go.dev/wiki/CodeReviewComments). The same document's "**good package names need no
> renaming**" corresponds to criterion C.

#### Prohibition 2: No Responsibility Accumulation

> **Decision (2026-10-03, ChenXu233)**: **No line-count / size / file-size gates.** The size problem
> in this repository is not solved by numbers, but by **semantic separation** — that is, "a module
> takes on only one class of responsibility". The previously proposed line-count ratchet in this
> section is abandoned.

The correct handling of size problems is to **redraw responsibility boundaries**, not to set
upper-bound numbers on the boundaries. Reasons:

- The 8448-line `ir_gen.rs` exists because **it simultaneously bears 6 unrelated responsibilities**
  (symbol table / module-level globals / statement lowering / constant folding / loop iteration /
  call dispatch). Split into 6 modules and each is naturally only a few hundred lines — **no
  line-count rule is needed**.
- The 1400-line `From<BytecodeFile>` exists because **a single `From` stuffed in the field-by-field
  migration of the entire file structure**. Replace with 82 `decode_<op>` functions and it naturally
  shrinks.
- Conversely, setting a threshold produces two bad outcomes: either it legitimizes 8448 lines
  (taking the max), or it produces a one-shot remediation pressure (taking the median). **Both are
  treating the symptoms.**

| Criterion                                 | Content                                                                                                                                                                     | Repository incident it derives from                                   |
| ----------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- |
| **A. Responsibility class judgment**      | Does the new code belong to a **responsibility class already in this module**? If it is the 2nd class or beyond → violation; create a new module rather than stuffing it in | `ir_gen.rs` carrying 6 responsibility classes in a single file        |
| **B. Bypassing the authoritative source** | New table entries added without updating the single authoritative source (hand-written copies bypassing `code_tables`) → violation                                          | Synonym table + parallel enums                                        |
| **C. Boundary erosion**                   | The change introduces any cross-layer dependency (L2→L3, L4→L1/L2), `include!`, or `pub(crate)` cross-layer leakage → violation                                             | 2 occurrences of `operator_interfaces::spec()`, the single `include!` |

**Criterion A is a human judgment and cannot be mechanized** — which is exactly why it is more
important than line-count rules: line-count rules can be bypassed by "splitting into two files of
4000 lines each"; responsibility judgment cannot.

Criterion C can be mechanized; see the section "Machine-Checkable Rule List" below.

> **External basis**: Go's official Code Review Comments explicitly opposes line-count rules:
> _"there is no rule 'a function must not exceed N lines'... the solution is to change function
> boundaries, not to start counting lines"_
> (https://go.dev/wiki/CodeReviewComments#short-functions). **This repository adopts this stance** —
> the only difference is that we have confirmed "boundaries not drawn correctly" is a real root
> cause, so the boundary definition is written into criterion A rather than into a line-count
> threshold.

#### Prohibition 3: Should Refactor but Patched Instead

| Criterion                     | Content                                                                                                                                                               | Repository incident it derives from |
| ----------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------- |
| **A. Synonymous duplication** | The same behavior appears in ≥ 2 places (aliases / conversions / stage wiring / error mapping) → violation; must be lifted to a shared layer                          | Import aliases, stage wiring        |
| **B. Excessive fan-out**      | A single change must simultaneously modify ≥ 3 existing synonymous mappings → **classified as an architectural change**, stop work, go through the design-doc process | Synonym table + 3 representations   |
| **C. Adding an entry point**  | Adding the "Nth entry point/table item" rather than registering it into the declarative stage table → violation                                                       | 5 entry points each hand-wired      |

### Decision Procedure

**Each step is a decidable boolean condition. Stop on the first hit.**

```
D0  Gate     Does this change touch any existing table (error codes / opcodes / types / entry stages)?
              ├─ Yes → Can you point to the single authoritative implementation module of that table?
              │         ├─ Yes → Continue
              │         └─ No  → ⛔ Stop. Establish the authoritative source first.
              └─ No  → Continue
                     ↓
D1  Anti-invention   After removing the new concept, can the same semantics be expressed with "existing concept + parameters"?
              ├─ Yes → ⛔ Forbid adding it. Record the reason in the PR.
              └─ No  → Continue
                     ↓
D2  Anti-parallel   Is the new concept semantically equivalent to some existing concept (≥ half of variant names overlap)?
              ├─ Yes → Is there a From/TryFrom conversion, and is it machine-checked?
              │         ├─ Yes → Continue
              │         └─ No  → ⛔ Merge into one (forbid using import aliases to distinguish)
              └─ No  → Continue
                     ↓
D3  Patch?   Judge in order; stop on the first hit:
              1) Does the same behavior need to be duplicated in ≥ 2 places?     → Yes → ⛔ Must be lifted to a shared layer
              2) Need to add the "6th entry point"?                            → Yes → ⛔ Forbid hand-wiring; register into the stage table instead
              3) Does a single change require modifying ≥ 3 synonymous mappings? → Yes → ⛔ Classified as an architectural change; stop and go through the design doc
              4) None of the above                                              → Local patch is allowed, but must include a regression test
                     ↓
D4  Responsibility?  Does the new code belong to an existing responsibility class of the target module?
              ├─ No (2nd class or beyond) → ⛔ Create a new module, or migrate the existing code for that responsibility together
              └─ Yes → Compliant
```

**Basis for D0**: `build.rs:19-55` has already implemented this pattern — `code_tables` is the
**only** implementation location for error-code parsing/validation/comparison; if `validate` fails
it `panic!`s to refuse compilation, and offers a `--fix` remedy command. **This is not a new
invention; it is generalizing the existing pattern from "covers only error codes" to "covers all
tables".**

**External basis for D1/D2**: Go's "**don't define interfaces before you have a real use case**"
(https://go.dev/wiki/CodeReviewComments) and "**good package names need no renaming**"
(https://go.dev/wiki/CodeReviewComments#imports). D2 borrows the uniqueness idea from RFC 2451
(https://rust-lang.github.io/rfcs/2451-re-rebalancing-coherence.html).

**D4 is purely human judgment, with no machine substitute** — which is why it is more important than
line-count rules: line-count rules can be bypassed by "splitting into two files of 4000 lines each";
responsibility judgment cannot. The 8448-line `ir_gen.rs` exists because it **simultaneously bears 6
unrelated responsibilities**; split into 6 modules and each is naturally only a few hundred lines,
with no need for any numeric rule.

### Machine-Checkable Rule List

| Rule                                                                     | Implementation                                                                                                                                                                     | External reference                                                                                                       |
| ------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| **Table consistency** (error codes / opcodes / type table / test wiring) | Extend the existing `build.rs` gate: `code_tables` adds new table types, `parse` → `validate` → on failure `panic!`, with `--fix`                                                  | This repository's `build.rs:19-55` is already an example                                                                 |
| **5 entry-point stage sequence consistency**                             | Add a **declarative stage table** (`stage_table.rs`); all 5 entry points fetch the sequence from it; tests assert "the stage sequence produced by the 5 entry points == the table" | rustc's `rustc_queries!` declarative query table (https://github.com/rust-lang/rustc-dev-guide/blob/master/src/query.md) |
| **Parallel enums missing conversions**                                   | CI uses `syn` to walk all `enum`s: Jaccard ≥ 0.5 of variant names against an existing enum and no `From`/`TryFrom` in the whole repo → fail                                        | RFC 2451's uniqueness idea (https://rust-lang.github.io/rfcs/2451-re-rebalancing-coherence.html)                         |
| **Duplicate concept detection**                                          | Scan all `pub enum/struct`s in the repo for same names and responsibility overlap; on hit, report "pick one or write a reason"                                                     | Same as above                                                                                                            |
| **Uncompiled test trees**                                                | `walkdir` scan: directory contains `#[cfg(test)]` or test module files, but the parent module has no corresponding `mod` declaration → fail                                        | Same idea as `stage_table`                                                                                               |
| **Disambiguating import aliases**                                        | Count `use … as …` occurrences; above the threshold, require a `// reason:` comment                                                                                                | Go "avoid renaming imports" (https://go.dev/wiki/CodeReviewComments#imports)                                             |
| **Boundary erosion** (Prohibition 2 Criterion C)                         | Forbid `include!`; `pub(crate)` cross-layer leakage count **may only decrease, not increase**; forbid L2→L3 reverse `use`                                                          | rustc's provider single-crate ownership                                                                                  |

**The most important one is "5 entry-point stage sequence consistency"** — it turns D3-2 directly
from "relying on human memory" to "won't compile if not in the table". rustc's `rustc_queries!`
declarative query table replacing ordered passes is the most mature solution to this problem
(https://github.com/rust-lang/rustc-dev-guide/blob/master/src/query.md).

> **Parts that cannot be copied directly**: the core benefit of the rustc query system is
> **incremental compilation and dependency graph**. If YaoXiang has no incremental compilation
> requirement, copying the entire query engine is over-engineering. **Borrow only two layers**: ①
> the declarative stage table; ② each stage has a single owning module.

### External Conventions Reference

| Convention                                                                                                              | Source                                                                                | Applicability to this repository                                                                                                |
| ----------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| Compiler = a string of purpose-distinguished IRs (Token/AST/HIR/THIR/MIR/LLVM-IR), each IR with a clear "why it exists" | rustc-dev-guide https://rustc-dev-guide.rust-lang.org/overview.html                   | **High**. Directly sets the bar "adding a 4th type representation must argue what new purpose it serves"                        |
| Directory names = pipeline stage names, reusing the same construct name across stages                                   | Same as above                                                                         | **High**. The top level of `src/` should be named by stage, not by "tools/miscellaneous"                                        |
| **Don't organize the compiler as ordered passes**; use a declarative query table + single-crate provider ownership      | rustc-dev-guide https://github.com/rust-lang/rustc-dev-guide/blob/master/src/query.md | **Highest**. This is exactly the cure for "5 entry points each hand-wired"                                                      |
| A central context object holds all queries and caches                                                                   | rustc-dev-guide                                                                       | Medium. Can serve as a reference for a unified compile context; the benefit depends on whether there is incremental compilation |
| **Place interfaces in the using package, not the implementing package**                                                 | Go CodeReviewComments https://go.dev/wiki/CodeReviewComments#interfaces               | High. Suitable for judging "which layer should the new abstraction descend to"                                                  |
| **Don't define interfaces before you have a real use case**                                                             | Same as above                                                                         | **Highest**. This is the most fitting external expression of "no invention"                                                     |
| **Avoid renaming imports for disambiguation**                                                                           | https://go.dev/wiki/CodeReviewComments#imports                                        | **Highest**. Directly targets the `AstBinOp`/`CEBinOp`/`B` aliases                                                              |
| Avoid meaningless names like `util` / `common` / `misc` / `api`                                                         | https://go.dev/wiki/CodeReviewComments#package-names                                  | Medium                                                                                                                          |
| **Mechanical issues go to tools, documentation only handles non-mechanical issues**                                     | CodeReviewComments opening                                                            | High. Determines the boundary of "which rules should be mechanized"                                                             |
| Uniqueness enforced by the language: coherence guarantees "for any trait+type there is exactly one impl"                | RFC 2451 https://rust-lang.github.io/rfcs/2451-re-rebalancing-coherence.html          | **Highest** (conceptually). Turns "duplicate definitions" from a review topic into a machine invariant                          |
| One stage, one file                                                                                                     | Zig `lib/std/zig/` (https://github.com/ziglang/zig/tree/master/lib/std/zig)           | **Low**. See below                                                                                                              |
| **No line-count / size rules**; "the solution is to change function boundaries, not to start counting lines"            | Go CodeReviewComments https://go.dev/wiki/CodeReviewComments#short-functions          | **Highest**. This repository has adopted it (decision on 2026-10-03)                                                            |

### What Cannot Be Copied Directly

1. **Zig's "one stage, one file"**. Zig's stage division is clear (`Ast.zig` / `Parse.zig` /
   `AstGen.zig` / `Zir.zig`), but **file sizes are completely out of control** — `AstGen.zig` 576
   KB, `Zir.zig` 210 KB, `Ast.zig` 148 KB (https://github.com/ziglang/zig/tree/master/lib/std/zig).
   **"Organize directories by stage" can be borrowed; "one stage, one file" cannot.**
2. **coherence / orphan rules**. RFC 2451 addresses **cross-crate** impl conflicts; the 3 type
   representations and 2 operator enums in this repository are **intra-crate** duplicates, and the
   language-level coherence will not report an error, so **a self-built CI check is required**.

### Code Review Checklist

Every PR's review goes through these items one by one:

- [ ] **D0** Does it touch any table? Which is the single authoritative implementation module of
      that table?
- [ ] **D1** Can the new concept be expressed with existing concept + parameters? If not, where is
      the reason written?
- [ ] **D2** Is it semantically equivalent to an existing concept? Where is the conversion? Are
      import aliases needed?
- [ ] **D3** Does the same behavior appear in ≥ 2 places? Has an entry point been added rather than
      registered into the stage table? Does it require modifying ≥ 3 synonymous mappings?
- [ ] **D4** Does the new code belong to an **existing responsibility class** of the target module?
      Or is it the 2nd class or beyond?
- [ ] Are **boundaries** being eroded? Cross-layer dependencies / `include!` / `pub(crate)`
      cross-layer leakage added?
- [ ] Have **mechanical issues** been caught by tools (fmt / clippy / `build.rs` gates)? Does the
      review focus only on non-mechanical issues?

The last item is directly borrowed from Go's division of labor: **mechanical issues go to tools,
reviews look only at non-mechanical issues** (https://go.dev/wiki/CodeReviewComments).

## Detailed Design

### Deliverables

| File                                                              | Contents                                                                                                                    | Type           |
| ----------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- | -------------- |
| `CONTRIBUTING.md`'s "Code Placement and Change Procedure" section | D0–D4 decision procedure + three prohibitions                                                                               | Document       |
| `scripts/ci/check-concepts.py`                                    | Prohibition 1's criteria A/B/C/D (parallel representations, disambiguating aliases, synonym table, insufficient call sites) | Gate script    |
| `scripts/ci/check-fanout.py`                                      | Prohibition 3's criteria A/B/C                                                                                              | Gate script    |
| `scripts/ci/check-boundary.py`                                    | Prohibition 2's criterion C (forbid `include!`, `pub(crate)` cross-layer leakage may only decrease, forbid L2→L3)           | Gate script    |
| `tools/code-tables` extension                                     | Table type registration for D0                                                                                              | Reuse existing |

**No `baseline.toml`, no `clippy.toml` threshold configuration, no `check-scale.py`.** Size problems
are solved by responsibility separation (Prohibition 2 Criterion A, human judgment); no numeric gate
is set.

### How Responsibility Classes Are Defined

D4 Criterion A needs the fact "what responsibility classes this module already has". Approach:

1. In `01-routing.md`'s **target directory structure**, each directory has its responsibilities and
   boundary invariants written down — **this is the authoritative definition of responsibility
   classes**
2. When submitting a PR, declare in the PR description "which responsibility in `<directory name>`
   the new code belongs to"
3. If the target directory's responsibility list has no corresponding entry → this responsibility is
   a new class; **either expand the responsibility list (and explain why it should go here), or
   create a new directory**

`check-boundary.py` handles the mechanical part (cross-layer dependencies / `include!` /
`pub(crate)`); responsibility attribution itself is judged by humans — which is exactly why it needs
to be written into `CONTRIBUTING.md` rather than into a script.

## Implementation Key Points

| Step | Content                                                                                                                                          | Acceptance                                                                                                                                                                 |
| ---- | ------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1    | Write D0–D4 and the three prohibitions into `CONTRIBUTING.md`; responsibility classes reference `01-routing.md`'s directory responsibility table | Pass human review                                                                                                                                                          |
| 2    | Implement `check-concepts.py` (Prohibition 1's four criteria); first run on **unchanged** code                                                   | **Should report the existing 3 operator enums and 2 parallel type representations (with the `ir::Type` alias counted in)** — only if it reports them is the gate effective |
| 3    | Implement `check-fanout.py` (Prohibition 3)                                                                                                      | **Intentionally adding a 6th entry-point-style wiring must fail**                                                                                                          |
| 4    | Implement `check-boundary.py` (Prohibition 2 Criterion C)                                                                                        | **Intentionally adding an L2→L3 reverse `use` must fail**; record the initial value of the `pub(crate)` leakage count                                                      |
| 5    | Extend `tools/code-tables` to cover opcodes / type tables                                                                                        | Intentionally introducing a zero-arm variant must fail (depends on P6 T1)                                                                                                  |

**Step 2 is the core acceptance of this phase**: `check-concepts.py` must be able to **on the
current unchanged code** report the known 2 operator enums. If it can't, it means the criteria are
designed wrong, and the gate is a fake gate.

## Key Decisions and Rationales

- **No line-count / size gates** (2026-10-03 decision) — adopt Go's official stance
  (https://go.dev/wiki/CodeReviewComments#short-functions). The size problem in this repository
  arises because `ir_gen.rs` simultaneously bears 6 responsibility classes and `From<BytecodeFile>`
  stuffs the entire file structure into a single `From` — **tear down the responsibilities and the
  numbers naturally shrink, no numeric rule is needed**. The originally envisioned `baseline.toml`
  ratchet and `clippy.toml` threshold are both abandoned.
- **The authoritative definition of responsibility classes is in `01-routing.md`'s directory
  responsibility table**, not in this document — to avoid definition drift between two locations.
- **Borrow only two layers from rustc's query, not the engine** — declarative stage table +
  single-crate provider ownership. The core benefit of the query engine is incremental compilation;
  this repository has no such requirement.
- **The coherence idea must be self-implemented** — RFC 2451 governs cross-crate; the problem in
  this repository is intra-crate duplication, and the language will not report an error.

### Rejected Directions

- **Line-count ratchet / `baseline.toml`** — The size problem in this repository arises from
  responsibility not being separated; setting a threshold will only legitimize 8448 lines, or
  produce a one-shot remediation pressure. **Abandoned.**
- **`clippy.toml`'s `too-many-lines-threshold` / clang-tidy's `readability-function-size`** — Same
  as above, **abandoned**.
- **Copying the rustc query engine wholesale** — Over-engineering; only borrow the declarative
  table.

## Known Limitations and Risks

- **False positives of the criteria**: Prohibition 1's criterion A uses "≥ half of variant names
  overlap" as a heuristic; two truly unrelated enums may collide on names (e.g. `Field`).
  Mitigation: allow a `// reason:` comment to grant an exemption.
- **Precision of `syn` analysis**: `syn`'s ability to parse macro-expanded forms is limited;
  cross-macro-generated definitions may be missed. Mitigation: the gate only reports "suspected",
  with humans to adjudicate.
- **Responsibility judgment is not mechanizable**: Prohibition 2's A is a human judgment. This means
  **it can only be enforced through review, not by CI** — this is the biggest weakness of this
  mechanism. Mitigation: write the responsibility list into `01-routing.md` to make it checkable;
  let `check-boundary.py` catch the mechanical part (cross-layer dependencies / `include!` /
  `pub(crate)` leakage).
- **The gate may be bypassed**: The `// reason:` comment is a legitimate escape hatch. Mitigation:
  the count of exemptions itself goes into `check-boundary.py`'s report.

> **The open questions originally listed in this section have all been adjudicated.** Item-by-item
> decisions are in [RFC-039 Decision Register](../rfc/draft/039-compiler-architecture.md) (D1–D50).
> **This document leaves no open items.**

## See Also

- [01-routing.md](01-routing.md) — **Authoritative definition of responsibility classes** (directory
  responsibilities and boundary invariants table), dependency-direction specification, target
  directory structure, routing table C
- [02-stage-contract.md](02-stage-contract.md) — Declarative `Stage` table (D0/D3-2 landing point)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — Historical incidents this procedure must
  clean up
- [09-execution-wbs.md](09-execution-wbs.md) — P0's three-level task breakdown
- [RFC-009a Borrow Proof Pipeline](../rfc/accepted/009a-borrow-proof-pipeline.md)
- [RFC-013 Error Code Specification](../rfc/accepted/013-error-code-specification.md) — Source of
  the existing gate's specification
- `build.rs:19-55` — This repository's existing `panic!`-level gate example; the direct basis for
  the D0/D0 gate pattern
