# Repository Maintenance Mechanism and Code Placement Decision Procedure

> **Rule body has been migrated (2026-10-05)**: For precise rule descriptions of the three
> prohibitions, D0–D4, red lines, and review checklist, see
> [coding-rules.md](../coding-rules.md)—that is the long-term authoritative source. This document is
> retained as a **2026-10 diagnostics and decision record**: incident evidence, line numbers,
> external convention references, and rejected-proposal arguments, living and dying with RFC-039.
> When enforcing, please reference coding-rules.md, not this document, as the source of rules.

> **Companion design document**. This document is a companion to
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md),
> corresponding to the **P0 phase**.
>
> This document is paired with [09-execution-wbs.md](09-execution-wbs.md): the procedure produced in
> P0 constrains every construction phase of P1–P10; this phase does not produce any compiler code.

## Position and Scope

This document defines the **executable procedure** for "where to place new code, what to write it
as, and under what circumstances one must stop and refactor instead of patching." The three
prohibitions:

1. **No fabrication**—before adding a new concept, one must prove it does not duplicate an existing
   concept
2. **No responsibility accumulation**—a module only takes on one class of responsibility; to add new
   responsibility, create a new module
3. **Patching when refactoring is required**—every change must first pass the decision procedure

**This document covers**: the decision flow for adding/modifying code, machine-checkable rules, code
review checklist, external convention references.

**This document does not cover**: the four-layer model and directory structure (see
`01-routing.md`), phase contracts (see `02`), specific refactoring designs (see `03`–`07`).

## Current State: Why This Mechanism Is Needed

This repository has **mature countermeasures, but the same symptoms have appeared at least once
each**. Verified incidents:

| Incident                                                                                                                                                                                                                                      | Scale                                                                       | Prohibition Violated                              |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------- | ------------------------------------------------- |
| 3 parallel type representations (`ast::Type` / `MonoType` / `ir::Type`) + serialized `type_table`, bridged by hand-written synonym table `"Int" \| "int" \| "i64" => ...`                                                                     | `ir.rs:3` re-exports via `pub use`; `mono.rs:618-643` synonym table         | Fabrication + patch covering                      |
| 2 parallel operator enums (`ast::BinOp/UnOp` vs `const_data::BinOp/UnOp`), **without any `From`/`TryFrom`**, distinguished only by import aliases `AstBinOp`/`CEBinOp`/`B`; bytecode layer has a third set (`BinaryOp`/`UnaryOp`/`CompareOp`) | 5+ files use aliases; `const_eval.rs:185-216` hand-written lossy conversion | Fabrication                                       |
| 5 compilation entries each hand-wiring phases                                                                                                                                                                                                 | 11 places of inconsistent behavior                                          | Patch covering                                    |
| 8 test subtrees never participate in compilation (parent module missing `mod tests;`)                                                                                                                                                         | 1005 lines / 78 tests never run                                             | Responsibility not separated (no place for tests) |
| **`ir_gen.rs` is 8448 lines because it simultaneously shoulders 6 unrelated responsibilities** (symbol table / module-level globals / statement lowering / constant folding / loop iteration / call dispatch)                                 | —                                                                           | **Responsibility accumulation**                   |
| **`From<BytecodeFile>` is 1400 lines because the entire file structure is stuffed into one `From`**                                                                                                                                           | —                                                                           | **Responsibility accumulation**                   |
| `include!` concatenates 1547 lines; the file has no module identity                                                                                                                                                                           | The only `include!` in the entire repo                                      | Responsibility accumulation                       |
| **And at the same time**: `build.rs` has a build-time hard gate on error codes, with `panic!` on inconsistency                                                                                                                                | 145 codes + `--fix` healing                                                 | — This is a positive sample                       |

**Key observation**: the positive sample and the incidents live in the same repository. The
difference is not in capability, but in whether there is a mechanism to turn "taste" into
machine-checkable rules. `build.rs:19-55` has already proven this path works—**this mechanism only
needs to generalize it**.

## Target Design

### Operational Definitions of the Three Prohibitions

Each one provides **measurable criteria**, not slogans.

#### Prohibition 1: No Fabrication

Before adding any `pub` type / enum / constant table / concept, all four criteria must pass to be
compliant:

| Criterion                                  | Content                                                                                                                                                              | Repository Incident Basis                                                             |
| ------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| **A. Responsibility overlap detection**    | New item has **variant name overlap ≥ half** with some existing item AND responsibility overlap → violation                                                          | 3 operator enums (ast / const_data / bytecode)                                        |
| **B. Insufficient call sites**             | The new `pub` item has < 2 call sites (only the definition site + the sole call site) → violation. Cannot prove it is a "concept" rather than a local implementation | —                                                                                     |
| **C. Need disambiguation aliases**         | Need `use … as AstBinOp` / `CEBinOp` / `B` etc. aliases to distinguish same-semantic items → violation                                                               | `const_eval.rs:19,1061`, `ownership.rs:19`, `termination.rs:15,226`, `ir_gen.rs:2317` |
| **D. Relying on synonym tables to bridge** | New/modified items rely on hand-written string matching to bridge differences → violation, must be changed to exhaustive match or explicit conversion                | `mono.rs:618-643`                                                                     |

> **External basis**: Go's official Code Review Comments explicitly state "**don't define interfaces
> before you have real use cases**—without real use cases, it's hard to tell whether the interface
> is necessary or what methods it should have" (https://go.dev/wiki/CodeReviewComments). Another
> line in the same document, "**a good package name shouldn't need renaming**," corresponds exactly
> to criterion C.

#### Prohibition 2: No Responsibility Accumulation

> **Decision (2026-10-03, ChenXu233)**: **No line count / volume / file size gate.** This
> repository's scale problem is not solved by numbers, only by **semantic separation**—that is, "a
> module only takes on one class of responsibility." The line-count ratchet scheme originally in
> this section has been abandoned.

The correct handling of scale problems is to **redraw responsibility boundaries**, not to set
upper-bound numbers on boundaries. Reason:

- The cause of `ir_gen.rs` being 8448 lines is that it **simultaneously shoulders 6 unrelated
  responsibilities** (symbol table / module-level globals / statement lowering / constant folding /
  loop iteration / call dispatch). After splitting into 6 modules each will naturally only have a
  few hundred lines—**no line-count rule needed**.
- The cause of `From<BytecodeFile>` being 1400 lines is that **one `From` stuffed in the
  field-by-field transport of the entire file structure**. After changing to 82 `decode_<op>`
  functions it naturally becomes small.
- Conversely, setting thresholds produces two bad outcomes: either legitimizing the 8448 lines (take
  the maximum), or producing one-shot remediation pressure (take the median). **Both treat the
  symptoms.**

| Criterion                             | Content                                                                                                                                                                                         | Repository Incident Basis                                     |
| ------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| **A. Responsibility class judgment**  | Does the new code belong to a **responsibility class already held by the module**? Belongs to the 2nd class or above → violation, should create a new module rather than continuing to stuff in | `ir_gen.rs` single file shoulders 6 classes of responsibility |
| **B. Bypassing authoritative source** | New table entries but the unique authoritative source is not updated (bypassing `code_tables` with hand-written copy) → violation                                                               | Synonym table + parallel enums                                |
| **C. Boundary erosion**               | This change adds any cross-layer dependency (L2→L3, L4→L1/L2), `include!`, `pub(crate)` cross-layer leakage → violation                                                                         | 2 `operator_interfaces::spec()` sites, the only `include!`    |

**Criterion A is a human judgment that cannot be mechanized**—this is exactly why it is more
important than line-count rules: line-count rules can be circumvented by "splitting into two
4000-line files", responsibility judgment cannot.

Criterion C can be mechanized, see the "Machine-Checkable Rule List" section below.

> **External basis**: Go's official Code Review Comments explicitly opposes line-count rules:
> _"There is no 'function shall not exceed N lines' rule … the solution is to change function
> boundaries, not to start counting lines"_
> (https://go.dev/wiki/CodeReviewComments#short-functions). **This repository adopts this
> stance**—the only difference is we have confirmed "boundaries were not drawn well" as the real
> cause, so we write the boundary definition into criterion A, not into a line-count threshold.

#### Prohibition 3: Patching When Refactoring Is Required

| Criterion                     | Content                                                                                                                                                 | Repository Incident Basis         |
| ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------- |
| **A. Synonymous duplication** | The same behavior appears in ≥2 places (alias / conversion / phase wiring / error mapping) → violation, must be raised to the shared layer              | import aliases, phase wiring      |
| **B. Excessive fan-out**      | One change requires synchronously modifying ≥3 existing synonymous mappings → **judged as architectural change**, stop work, go to design document flow | Synonym table + 3 representations |
| **C. New entry**              | Adding "the Nth entry/table item" rather than registering into the declarative phase table → violation                                                  | 5 entries each wire up            |

### Decision Procedure

**Each step is a decidable boolean condition. Stop on hit.**

```
D0  Gate    Did this change touch any existing table (error codes/opcode/types/entry phases)?
             ├─ Yes → Can you point to that table's single authoritative implementation module?
             │        ├─ Yes → Continue
             │        └─ No  → ⛔ Stop. First establish the authoritative source
             └─ No  → Continue
                    ↓
D1  No fabrication   After removing the new concept, can the same semantics be expressed as "existing concept + parameters"?
             ├─ Yes → ⛔ Forbid new addition. Record reason in PR
             └─ No  → Continue
                    ↓
D2  No parallel     Is the new concept semantically equivalent to some existing concept (variant name overlap ≥ half)?
             ├─ Yes → Is there a From/TryFrom conversion that is machine-checked?
             │        ├─ Yes → Continue
             │        └─ No  → ⛔ Merge into one (forbid using import aliases to distinguish)
             └─ No  → Continue
                    ↓
D3  Patch?         Decide in order, stop on first hit:
             1) Same behavior duplicated in ≥2 places?                 → Yes → ⛔ Must be raised to shared layer
             2) Need to add a "6th entry"?                             → Yes → ⛔ Forbid manual wiring, register in stage table instead
             3) One change requires modifying ≥3 synonym mappings?      → Yes → ⛔ Treated as architectural change, stop and go to design doc
             4) All above No                                           → Allow local patch, but must include regression test
                    ↓
D4  Responsibility?  Does the new code belong to a responsibility class already held by the target module?
             ├─ No (2nd class or above) → ⛔ Create a new module, or move the existing code for that responsibility together
             └─ Yes → Compliant
```

**D0's basis**: `build.rs:19-55` has already implemented this pattern—`code_tables` is the **sole**
implementation location for error code parsing/validation/comparison, `validate` failure means
`panic!` refusing compilation, and a `--fix` healing command is provided. **This is not a new
invention, but generalizing the existing paradigm from "only covering error codes" to "all
tables."**

**External basis for D1/D2**: Go's official "**don't define interfaces before you have real use
cases**" (https://go.dev/wiki/CodeReviewComments) and "**a good package name shouldn't need
renaming**" (https://go.dev/wiki/CodeReviewComments#imports). D2 borrows the uniqueness idea from
RFC 2451 (https://rust-lang.github.io/rfcs/2451-re-rebalancing-coherence.html).

**D4 is a purely human judgment with no machine substitute**—this is why it is more important than
line-count rules: line-count rules can be circumvented by "splitting into two 4000-line files",
responsibility judgment cannot. The cause of `ir_gen.rs` being 8448 lines is that it
**simultaneously shoulders 6 unrelated responsibilities**; after splitting into 6 modules each will
naturally only have a few hundred lines, no numerical rules needed.

### Machine-Checkable Rule List

| Rule                                                                    | Implementation                                                                                                                                            | External Reference                                                                                                       |
| ----------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| **Table consistency** (error codes / opcode / type table / test wiring) | Extend the existing `build.rs` gate: `code_tables` adds table types, `parse` → `validate` → fail `panic!`, with `--fix`                                   | This repo's `build.rs:19-55` is already an example                                                                       |
| **5 entry phase sequences consistent**                                  | Add a **declarative phase table** (`stage_table.rs`), all 5 entries take sequences from it; test asserts "phase sequences produced by 5 entries == table" | rustc's `rustc_queries!` declarative query table (https://github.com/rust-lang/rustc-dev-guide/blob/master/src/query.md) |
| **Parallel enum missing conversion**                                    | CI uses `syn` to traverse all `enum`: variant name Jaccard with existing enum ≥ 0.5 and no `From`/`TryFrom` anywhere in the codebase → fail               | Uniqueness idea from RFC 2451 (https://rust-lang.github.io/rfcs/2451-re-rebalancing-coherence.html)                      |
| **Duplicate concept detection**                                         | Scan the entire codebase for `pub enum/struct` with the same name and overlapping responsibility; on hit, report "choose one or document the reason"      | Same as above                                                                                                            |
| **Uncompiled test tree**                                                | `walkdir` scan: directory contains `#[cfg(test)]` or test module file, but parent module has no corresponding `mod` declaration → fail                    | Same idea as `stage_table`                                                                                               |
| **Disambiguation import alias**                                         | Count `use … as …` occurrences; exceeding threshold requires an attached `// reason:` comment                                                             | Go "avoid renaming imports" (https://go.dev/wiki/CodeReviewComments#imports)                                             |
| **Boundary erosion** (Prohibition 2 criterion C)                        | Forbid `include!`; `pub(crate)` cross-layer leakage count **may only decrease, not increase**; forbid L2→L3 reverse `use`                                 | rustc's provider uniquely belongs to a crate                                                                             |

**The most important one is "5 entry phase sequences consistent"**—it directly turns D3-2 from
"relying on human memory" to "the table doesn't have it and the build fails". rustc uses
`rustc_queries!` declarative query tables to replace sequential passes, which is the most mature
solution to this problem (https://github.com/rust-lang/rustc-dev-guide/blob/master/src/query.md).

> **Parts that cannot be copied verbatim**: the core benefit of rustc's query system is
> **incremental compilation and dependency graphs**. If YaoXiang has no need for incremental
> compilation, copying the entire query engine is over-engineering. **Only borrow two layers**: ①
> declarative phase table; ② each phase has a unique owning module.

### External Convention References

| Convention                                                                                                                  | Source                                                                                | Applicability to This Repo                                                                                            |
| --------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| Compiler = a chain of IRs distinguished by purpose (Token/AST/HIR/THIR/MIR/LLVM-IR), each IR clearly states "why it exists" | rustc-dev-guide https://rustc-dev-guide.rust-lang.org/overview.html                   | **High**. Directly gives the threshold of "adding a 4th type representation must justify what new purpose it serves"  |
| Directory name = pipeline phase name, cross-phase reuse of the same construction name                                       | Same as above                                                                         | **High**. `src/` top level should be named by phase, not "tools/misc"                                                 |
| **Do not organize the compiler with sequential passes**, use declarative query tables + provider unique ownership           | rustc-dev-guide https://github.com/rust-lang/rustc-dev-guide/blob/master/src/query.md | **Highest**. This is the symptomatic cure for "5 entries each hand-wired"                                             |
| Central context object holds all queries and caches                                                                         | rustc-dev-guide                                                                       | Medium. Can serve as unified compilation context reference, benefit depends on whether incremental compilation exists |
| **Interface in the consumer package, not the implementer package**                                                          | Go CodeReviewComments https://go.dev/wiki/CodeReviewComments#interfaces               | High. Suitable for judging "to which layer the new abstraction should sink"                                           |
| **Don't define interfaces before you have real use cases**                                                                  | Same as above                                                                         | **Highest**. This is the most fitting external expression of "no fabrication"                                         |
| **Avoid renaming imports for disambiguation**                                                                               | https://go.dev/wiki/CodeReviewComments#imports                                        | **Highest**. Directly hits the `AstBinOp`/`CEBinOp`/`B` aliases                                                       |
| Avoid meaningless names like `util` / `common` / `misc` / `api`                                                             | https://go.dev/wiki/CodeReviewComments#package-names                                  | Medium                                                                                                                |
| **Mechanical problems go to tools, docs only handle non-mechanical problems**                                               | Beginning of CodeReviewComments                                                       | High. Decides the boundary of "which rules should be mechanized"                                                      |
| Uniqueness enforced by language: coherence guarantees "for any trait+type there is exactly one impl"                        | RFC 2451 https://rust-lang.github.io/rfcs/2451-re-rebalancing-coherence.html          | **Highest** (idea level). Turns "duplicate definition" from a review topic into a machine invariant                   |
| One phase one file                                                                                                          | Zig `lib/std/zig/` (https://github.com/ziglang/zig/tree/master/lib/std/zig)           | **Low**. See below                                                                                                    |
| **No line-count / volume rules**; "the solution is to change function boundaries, not to start counting lines"              | Go CodeReviewComments https://go.dev/wiki/CodeReviewComments#short-functions          | **Highest**. This repo has adopted (2026-10-03 decision)                                                              |

### Cannot Be Directly Copied

1. **Zig's "one phase one file"**. Zig's phase division is clear (`Ast.zig` / `Parse.zig` /
   `AstGen.zig` / `Zir.zig`), but **file scale is completely out of control**—`AstGen.zig` 576 KB,
   `Zir.zig` 210 KB, `Ast.zig` 148 KB (https://github.com/ziglang/zig/tree/master/lib/std/zig).
   **"Divided by phase in directories" can be borrowed, "one phase one file" cannot.**
2. **Coherence / orphan rules**. RFC 2451 solves **cross-crate** impl conflicts; this repo's 3 type
   representations and 2 operator enums are **within a single crate** duplicate definitions,
   language-level coherence will not report an error, **a self-built CI check is required**.

### Code Review Checklist

Each PR's review goes through these one by one:

- [ ] **D0** Does it touch any table? Which is that table's sole authoritative implementation
      module?
- [ ] **D1** Can the new concept be expressed as existing concept + parameters? If not, where is the
      reason documented?
- [ ] **D2** Is it semantically equivalent to an existing concept? Where is the conversion? Does it
      require import aliases?
- [ ] **D3** Does the same behavior appear in ≥2 places? Does it add an entry instead of registering
      in the stage table? Does it require modifying ≥3 synonym mappings?
- [ ] **D4** Does the new code belong to a **responsibility class already held** by the target
      module? Or the 2nd class or above?
- [ ] Have **boundaries** been eroded? New cross-layer dependencies / `include!` / `pub(crate)`
      cross-layer leakage?
- [ ] Have **mechanical problems** been caught by tools (fmt / clippy / `build.rs` gate)? Does the
      review only spend time on non-mechanical problems?

The last one is directly copied from Go's division-of-labor principle: **mechanical problems go to
tools, review only looks at non-mechanical problems** (https://go.dev/wiki/CodeReviewComments).

## Detailed Design

### Deliverables List

| File                                                              | Content                                                                                                                     | Type           |
| ----------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- | -------------- |
| `CONTRIBUTING.md`'s "Code Placement and Change Procedure" section | D0–D4 decision procedure + three prohibitions                                                                               | Documentation  |
| `scripts/ci/check-concepts.py`                                    | Prohibition 1's A/B/C/D criteria (parallel representations, disambiguation aliases, synonym table, insufficient call sites) | Gate script    |
| `scripts/ci/check-fanout.py`                                      | Prohibition 3's A/B/C criteria                                                                                              | Gate script    |
| `scripts/ci/check-boundary.py`                                    | Prohibition 2's criterion C (forbid `include!`, `pub(crate)` cross-layer leakage may only decrease, forbid L2→L3)           | Gate script    |
| `tools/code-tables` extension                                     | D0's table type registration                                                                                                | Reuse existing |

**No `baseline.toml`, no `clippy.toml` threshold configuration, no `check-scale.py`.** Scale
problems are solved by responsibility separation (Prohibition 2 criterion A, human judgment), no
numerical gates.

### How Responsibility Classes Are Defined

D4 criterion A needs the fact "what responsibility classes does this module already have". Approach:

1. In the **target directory structure** of `01-routing.md`, each directory already documents its
   responsibility and boundary invariants—**this is the authoritative definition of responsibility
   classes**
2. When submitting a PR, declare in the PR description "which responsibility of `<directory name>`
   does this new code belong to"
3. If there is no corresponding entry in the target directory's responsibility list → this
   responsibility is a new class, **either extend the responsibility list (and explain why it
   belongs here), or create a new directory**

`check-boundary.py` is responsible for the mechanical part (cross-layer dependencies / `include!` /
`pub(crate)`); responsibility attribution itself is judged by humans—this is exactly why it needs to
be written in `CONTRIBUTING.md` rather than a script.

## Implementation Essentials

| Step | Content                                                                                                                                              | Acceptance                                                                                                                                                         |
| ---- | ---------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1    | Write D0–D4 and the three prohibitions into `CONTRIBUTING.md`; responsibility class references the directory responsibility table in `01-routing.md` | Human review pass                                                                                                                                                  |
| 2    | Implement `check-concepts.py` (Prohibition 1 four criteria), first run on **unchanged** code                                                         | **Should report the existing 3 operator enums and 2 parallel type representations (count the `ir::Type` alias together)—reporting it means the gate is effective** |
| 3    | Implement `check-fanout.py` (Prohibition 3)                                                                                                          | **Deliberately adding a 6th entry-style wiring must turn red**                                                                                                     |
| 4    | Implement `check-boundary.py` (Prohibition 2 criterion C)                                                                                            | **Deliberately adding an L2→L3 reverse `use` must turn red**; `pub(crate)` leakage count records the initial value                                                 |
| 5    | Extend `tools/code-tables` to cover opcode / type tables                                                                                             | Deliberately introducing zero-construction variants must turn red (depends on P6's T1)                                                                             |

**Step 2 is the core acceptance of this phase**: `check-concepts.py` must be able to **on currently
unchanged code** report the known 2 operator enums. If it cannot report them, the criterion design
is wrong and the gate is a fake gate.

## Key Decisions and Reasons

- **No line-count / volume gate of any kind** (2026-10-03 decision)—adopt Go's official stance
  (https://go.dev/wiki/CodeReviewComments#short-functions). The cause of this repo's scale problem
  is `ir_gen.rs` simultaneously shouldering 6 responsibilities, and `From<BytecodeFile>` stuffing
  the entire file structure into one `From`—**remove the responsibility, the number naturally
  shrinks, no numerical rule needed**. The originally envisioned `baseline.toml` ratchet and
  `clippy.toml` thresholds are both abandoned.
- **The authoritative definition of responsibility class goes in the directory responsibility table
  of `01-routing.md`**, not here—avoid two-place definition drift.
- **Only borrow two layers of rustc queries, don't move the engine**—declarative phase table +
  provider unique ownership. The core benefit of the query engine is incremental compilation, which
  this repo does not need.
- **The coherence idea must be self-implemented**—RFC 2451 governs cross-crate, this repo's problem
  is within-crate duplication, the language will not report an error.

### Rejected Directions

- **Line-count ratchet / `baseline.toml`**—the cause of this repo's scale problem is un-separated
  responsibility, setting thresholds only legitimizes 8448 lines, or produces one-shot remediation
  pressure. **Abandoned.**
- **`clippy.toml`'s `too-many-lines-threshold` / clang-tidy `readability-function-size`**—same as
  above, **abandoned**.
- **Copying the rustc query engine**—over-engineered, only borrow the declarative table.

## Known Limitations and Risks

- **False positives in criteria**: Prohibition 1 criterion A's "variant name overlap ≥ half" is a
  heuristic; two truly unrelated enums may collide on names (e.g., `Field`). Mitigation: allow
  `// reason:` comments to exempt.
- **Precision of `syn` analysis**: `syn`'s ability to parse post-macro-expansion forms is limited;
  definitions generated across macros may be missed. Mitigation: the gate only reports "suspected",
  humans adjudicate.
- **Responsibility judgment cannot be mechanized**: Prohibition 2 A is a human judgment. This means
  **it can only be enforced by review, not by CI**—this is this mechanism's biggest weakness.
  Mitigation: write the responsibility list into `01-routing.md` so it is checkable;
  `check-boundary.py` catches the mechanical part (cross-layer dependencies / `include!` /
  `pub(crate)` leakage).
- **Gates may be bypassed**: `// reason:` comments are a legitimate escape hatch. Mitigation: the
  exemption count itself goes into `check-boundary.py`'s report.

> **All open questions originally listed in this section have been adjudicated.** Item-by-item
> decisions see [RFC-039 decision log](../../rfc/accepted/039-compiler-architecture.md) (D1–D50).
> **This document leaves no pending items.**

## See Also

- [01-routing.md](01-routing.md) — **Authoritative definition of responsibility class** (directory
  responsibility and boundary invariant table), dependency direction specification, target directory
  structure, routing table C
- [02-stage-contract.md](02-stage-contract.md) — `Stage` declarative table (landing point for
  D0/D3-2)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — Historical incidents to be cleaned up by this
  procedure
- [09-execution-wbs.md](09-execution-wbs.md) — P0's three-level task breakdown
- [RFC-009a Borrow Proof Pipeline](../../rfc/accepted/009a-borrow-proof-pipeline.md)
- [RFC-013 Error Code Specification](../../rfc/accepted/013-error-code-specification.md) —
  Specification source for the existing gate
- `build.rs:19-55` — The repo's existing `panic!`-level gate example, the direct basis for the D0/D0
  gate pattern
