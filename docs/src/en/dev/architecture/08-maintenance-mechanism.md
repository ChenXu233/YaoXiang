# Repository Maintenance Mechanism and Code Placement Decision Procedure

> **Subsidiary design document**. This document is a subsidiary of
> [RFC-039 Compiler Architecture Refactor](../../rfc/draft/039-compiler-architecture.md), corresponding
> to **Phase P0**.
>
> This document is paired with [09-execution-wbs.md](09-execution-wbs.md): the procedures produced
> by P0 constrain every construction phase of P1–P10; this phase produces no compiler code.

## Position and Scope

This document defines the **executable procedure for "where to place new code, what to write it
like, and when to stop and refactor rather than apply a patch"**. Three prohibitions:

1. **No fabrication**—before adding a new concept, you must prove it does not duplicate an existing
   one
2. **No responsibility accumulation**—a module carries only one class of responsibility; to add a
   new responsibility, create a new module
3. **Patching instead of refactoring**—every change must first pass the decision procedure

**This document covers**: the decision flow for adding/modifying code, machine-checkable rules, code
review checklist, external convention references.

**This document does not cover**: the four-layer model and directory structure (see
`01-routing.md`), stage contracts (see `02`), specific refactoring designs (see `03`–`07`).

## Current State: Why This Mechanism Is Needed

This repository has **mature countermeasures, and the same symptoms have each appeared at least
once**. Verified incidents:

| Incident                                                                                                                                                                                                                                         | Scale                                                                       | Prohibition Violated                                  |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------- | ----------------------------------------------------- |
| 3 parallel type representations (`ast::Type` / `MonoType` / `ir::Type`) + serialized `type_table`, bridged by handwritten synonym table `"Int" \| "int" \| "i64" => ...`                                                                         | `ir.rs:3` re-exports via `pub use`; `mono.rs:618-643` synonym table         | Fabrication + patch concealment                       |
| 2 parallel operator enums (`ast::BinOp/UnOp` vs `const_data::BinOp/UnOp`), **no `From`/`TryFrom` at all**, distinguished by import aliases `AstBinOp`/`CEBinOp`/`B`; a third set exists at the bytecode level (`BinaryOp`/`UnaryOp`/`CompareOp`) | 5+ files with aliases; `const_eval.rs:185-216` handwritten lossy conversion | Fabrication                                           |
| 5 compilation entry points each hand-wiring stages                                                                                                                                                                                               | 11 behaviorally inconsistent locations                                      | Patch concealment                                     |
| 8 test subtrees never participated in compilation (parent module missing `mod tests;`)                                                                                                                                                           | 1005 lines / 78 tests never run                                             | Responsibility not separated (nowhere to place tests) |
| **`ir_gen.rs` is 8448 lines because it simultaneously carries 6 unrelated responsibility classes** (symbol table / module-level globals / statement lowering / constant folding / loop iteration / call dispatch)                                | —                                                                           | **Responsibility accumulation**                       |
| **`From<BytecodeFile>` is 1400 lines because the entire file structure was stuffed into one `From`**                                                                                                                                             | —                                                                           | **Responsibility accumulation**                       |
| `include!` concatenates 1547 lines, file has no module identity                                                                                                                                                                                  | The only `include!` in the entire repo                                      | Responsibility accumulation                           |
| **And at the same time**: `build.rs` does build-time hard gating on error codes, panics immediately on inconsistency                                                                                                                             | 145 codes + `--fix` cure                                                    | — This is a positive sample                           |

**Key observation**: positive samples and incidents coexist in the same repository. The difference
is not in capability, but in whether a mechanism exists to turn "taste" into machine-checkable
rules. `build.rs:19-55` has already proven this path is viable—**what this mechanism does is simply
generalize it**.

## Target Design

### Operational Definitions of the Three Prohibitions

Each prohibition has **measurable criteria**, not slogans.

#### Prohibition 1: No Fabrication

Before adding any `pub` type / enum / constant table / concept, all four criteria must pass:

| Criterion                               | Content                                                                                                                                                      | Repository Incident Basis                                                             |
| --------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------- |
| **A. Responsibility overlap detection** | New item shares variant names with an existing item **≥ half** AND has overlapping responsibility → violation                                                | 3 sets of operator enums (ast / const_data / bytecode)                                |
| **B. Insufficient call sites**          | New `pub` item has < 2 call sites (only the definition site + single call site) → violation. Cannot prove it is a "concept" rather than local implementation | —                                                                                     |
| **C. Disambiguating aliases needed**    | Aliases like `use … as AstBinOp` / `CEBinOp` / `B` are needed to distinguish same-semantic items → violation                                                 | `const_eval.rs:19,1061`、`ownership.rs:19`、`termination.rs:15,226`、`ir_gen.rs:2317` |
| **D. Bridged by synonym table**         | New/modified item relies on handwritten string matching to bridge differences → violation, must be exhaustive or explicit conversion                         | `mono.rs:618-643`                                                                     |

> **External basis**: Go's official Code Review Comments explicitly state "**don't define interfaces
> before there are real use cases**—without real use cases, it's hard to judge whether an interface
> is necessary, let alone what methods it should have" (https://go.dev/wiki/CodeReviewComments ).
> The same document's "**a good package name shouldn't need renaming**" corresponds directly to
> criterion C.

#### Prohibition 2: No Responsibility Accumulation

> **Decision (2026-10-03, ChenXu233)**: **No line count / volume / file size gating.** The scale
> problem in this repository is not solved by numbers, but by **semantic separation**—that is, "a
> module carries only one class of responsibility". The previously planned line count ratchet scheme
> has been abandoned.

The correct handling of scale problems is to **redraw responsibility boundaries**, not set
upper-bound numbers on boundaries. Reasons:

- `ir_gen.rs`'s 8448 lines are caused by it **simultaneously carrying 6 unrelated responsibility
  classes** (symbol table / module-level globals / statement lowering / constant folding / loop
  iteration / call dispatch). Split into 6 modules and each naturally has only a few hundred
  lines—**no line count rules needed**.
- `From<BytecodeFile>`'s 1400 lines are caused by **one `From` stuffed with the entire file
  structure's field-by-field transport**. After changing to 82 `decode_<op>` functions, it naturally
  shrinks.
- Conversely, setting thresholds produces two bad outcomes: either legitimizing 8448 lines (taking
  the maximum), or producing one-time remediation pressure (taking the median). **Both treat the
  symptom.**

| Criterion                                 | Content                                                                                                                                                                       | Repository Incident Basis                                      |
| ----------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| **A. Responsibility class determination** | Does the new code belong to a **responsibility class the module already has**? Belongs to the 2nd class or beyond → violation, should create a new module instead of stuffing | `ir_gen.rs` carries 6 responsibility classes in one file       |
| **B. Bypassing the authoritative source** | Adding a table entry without updating the single authoritative source (bypassing `code_tables` handwritten copy) → violation                                                  | Synonym table + parallel enums                                 |
| **C. Boundary erosion**                   | This change adds any cross-layer dependency (L2→L3, L4→L1/L2), `include!`, `pub(crate)` cross-layer leak → violation                                                          | 2 locations `operator_interfaces::spec()`, the only `include!` |

**Criterion A is manual judgment and cannot be mechanized**—this is precisely why it is more
important than line count rules: line count rules can be bypassed by "splitting into two 4000-line
files", responsibility determination cannot.

Criterion C is machine-checkable; see the section "Machine-Checkable Rule List" below.

> **External basis**: Go's official Code Review Comments explicitly opposes line count rules:
> _"there is no 'functions shall not exceed N lines' rule... the solution is to change the function
> boundary, not to start counting lines"_ (https://go.dev/wiki/CodeReviewComments#short-functions ).
> **This repository adopts this position**—the only difference is that we have confirmed "boundary
> not properly drawn" is the real cause, so we write the boundary definition into criterion A, not
> into a line count threshold.

#### Prohibition 3: Patching Instead of Refactoring

| Criterion                    | Content                                                                                                                                                               | Repository Incident Basis                 |
| ---------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------- |
| **A. Synonymous repetition** | The same behavior appears in ≥2 locations (alias / conversion / stage wiring / error mapping) → violation, must be lifted to the shared layer                         | import aliases, stage wiring              |
| **B. Excessive fan-out**     | One change requires synchronously modifying ≥3 existing synonymous mappings → **judged as an architecture change**, stop work, go through the design document process | Synonym table + 3 sets of representations |
| **C. New entry point**       | Adding "the Nth entry point/table entry" rather than registering in the declarative stage table → violation                                                           | 5 entry points each hand-wiring           |

### Decision Procedure

**Each step is a decidable boolean condition. Stop on hit.**

```
D0  Gate      Does this change touch any existing table (error code/opcode/type/entry stage)?
              ├─ Yes → Can you point to the single authoritative implementation module for that table?
              │         ├─ Yes → Continue
              │         └─ No  → ⛔ Stop. First establish the authoritative source
              └─ No  → Continue
                     ↓
D1  Anti-fab  After removing the new concept, can the same semantics be expressed with
              "existing concept + parameter"?
              ├─ Yes → ⛔ Forbid adding. Reason recorded in PR
              └─ No  → Continue
                     ↓
D2  Anti-par  Is the new concept same-semantic as some existing concept
              (variant name overlap ≥ half)?
              ├─ Yes → Is there a From/TryFrom conversion with machine check?
              │         ├─ Yes → Continue
              │         └─ No  → ⛔ Merge into one (forbid distinguishing with import alias)
              └─ No  → Continue
                     ↓
D3  Patch?    Determine in order, stop on hit:
              1) Same behavior needs to be replicated in ≥2 locations?    → Yes → ⛔ Must lift to shared layer
              2) Need to add "the 6th entry point"?                       → Yes → ⛔ Forbid hand-wiring, register in stage table
              3) One change requires modifying ≥3 synonymous mappings?   → Yes → ⛔ Judge as architecture change, stop and go through design document
              4) None of the above                                        → Local patch allowed, but must include regression tests
                     ↓
D4  Resp.?    Does this newly added code belong to a responsibility class
              the target module already has?
              ├─ No (2nd class or beyond) → ⛔ Create a new module, or move the existing code of that responsibility over together
              └─ Yes                       → Compliant
```

**Basis for D0**: `build.rs:19-55` has already implemented this pattern—`code_tables` is the
**only** implementation location for error code parsing/validation/comparison, and `validate`
failure means `panic!` refusing to compile, with a `--fix` cure command provided. **This is not a
new invention; it is generalizing the existing paradigm from "only covering error codes" to
"covering all tables".**

**External basis for D1/D2**: Go's official "**don't define interfaces before there are real use
cases**" (https://go.dev/wiki/CodeReviewComments ) and "**a good package name shouldn't need
renaming**" (https://go.dev/wiki/CodeReviewComments#imports ). D2 draws on the uniqueness idea from
RFC 2451 (https://rust-lang.github.io/rfcs/2451-re-rebalancing-coherence.html ).

**D4 is pure manual judgment with no machine substitute**—this is why it is more important than line
count rules: line count rules can be bypassed by "splitting into two 4000-line files",
responsibility determination cannot. The cause of `ir_gen.rs`'s 8448 lines is that it
**simultaneously carries 6 unrelated responsibility classes**; after splitting into 6 modules, each
naturally has only a few hundred lines, with no numeric rules needed.

### Machine-Checkable Rule List

| Rule                                                                   | Implementation                                                                                                                                                    | External Reference                                                                                                        |
| ---------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| **Table consistency** (error code / opcode / type table / test wiring) | Extend existing `build.rs` gate: `code_tables` adds table types, `parse` → `validate` → fail means `panic!`, with `--fix`                                         | This repo's `build.rs:19-55` is already an example                                                                        |
| **5 entry points have consistent stage sequences**                     | Add **declarative stage table** (`stage_table.rs`), 5 entry points all take sequences from it; test asserts "stage sequences produced by 5 entry points == table" | rustc's `rustc_queries!` declarative query table (https://github.com/rust-lang/rustc-dev-guide/blob/master/src/query.md ) |
| **Parallel enums missing conversions**                                 | CI uses `syn` to traverse all `enum`: variant name Jaccard with existing enum ≥ 0.5 AND no `From`/`TryFrom` in the entire repo → fail                             | Uniqueness idea from RFC 2451 (https://rust-lang.github.io/rfcs/2451-re-rebalancing-coherence.html )                      |
| **Duplicate concept detection**                                        | Scan all `pub enum/struct` in the repo for same names and responsibility overlap, on hit report "choose one or write a reason"                                    | Same as above                                                                                                             |
| **Uncompiled test tree**                                               | `walkdir` scan: directory contains `#[cfg(test)]` or test module files, but parent module has no corresponding `mod` declaration → fail                           | Same `stage_table` idea                                                                                                   |
| **Disambiguating import aliases**                                      | Count `use … as …`, exceeding threshold requires a `// reason:` comment                                                                                           | Go "avoid renaming imports" (https://go.dev/wiki/CodeReviewComments#imports )                                             |
| **Boundary erosion** (Prohibition 2 Criterion C)                       | Forbid `include!`; `pub(crate)` cross-layer leak count **only allowed to decrease, not increase**; forbid L2→L3 reverse `use`                                     | rustc's provider unique-ownership crate                                                                                   |

**The most important one is "5 entry points have consistent stage sequences"**—it directly turns
D3-2 from "depends on people remembering" to "not in the table means compilation fails". rustc's use
of `rustc_queries!` declarative query table to replace sequential passes is the most mature solution
to this problem (https://github.com/rust-lang/rustc-dev-guide/blob/master/src/query.md ).

> **Part that cannot be copied wholesale**: the core benefit of rustc's query system is
> **incremental compilation and dependency graph**. If YaoXiang has no incremental compilation need,
> copying the entire query engine wholesale is over-engineering. **Only borrow two layers**: ①
> declarative stage table; ② each stage has a single owning module.

### External Convention Reference

| Convention                                                                                                               | Source                                                                                | Applicability to This Repo                                                                                               |
| ------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| Compiler = a chain of purpose-distinguished IRs (Token/AST/HIR/THIR/MIR/LLVM-IR), each IR clearly states "why it exists" | rustc-dev-guide https://rustc-dev-guide.rust-lang.org/overview.html                   | **High**. Directly gives the threshold "adding a 4th set of type representations must argue which new purpose it serves" |
| Directory name = pipeline stage name, cross-stage reuse of the same construct name                                       | Same as above                                                                         | **High**. Top level of `src/` should be named by stage, not by "tools/misc"                                              |
| **Don't organize compiler with sequential passes**, use declarative query table + provider unique-ownership              | rustc-dev-guide https://github.com/rust-lang/rustc-dev-guide/blob/master/src/query.md | **Highest**. This is exactly the antidote for "5 entry points each hand-wiring"                                          |
| Central context object holds all queries and caches                                                                      | rustc-dev-guide                                                                       | Medium. Can serve as unified compilation context reference, benefit depends on whether incremental compilation exists    |
| **Interfaces go in the consumer package, not the implementer package**                                                   | Go CodeReviewComments https://go.dev/wiki/CodeReviewComments#interfaces               | High. Suitable for judging "which layer should the new abstraction sink to"                                              |
| **Don't define interfaces before there are real use cases**                                                              | Same as above                                                                         | **Highest**. This is the most apt external expression of "no fabrication"                                                |
| **Avoid renaming imports for disambiguation**                                                                            | https://go.dev/wiki/CodeReviewComments#imports                                        | **Highest**. Directly hits the `AstBinOp`/`CEBinOp`/`B` aliases                                                          |
| Avoid meaningless names like `util` / `common` / `misc` / `api`                                                          | https://go.dev/wiki/CodeReviewComments#package-names                                  | Medium                                                                                                                   |
| **Mechanical problems go to tools, documentation only handles non-mechanical problems**                                  | CodeReviewComments opening                                                            | High. Determines the boundary of "which rules should be mechanized"                                                      |
| Uniqueness enforced by language: coherence guarantees "for any trait+type there is exactly one impl"                     | RFC 2451 https://rust-lang.github.io/rfcs/2451-re-rebalancing-coherence.html          | **Highest** (at the conceptual level). Turns "duplicate definitions" from a review topic into a machine invariant        |
| One stage per file                                                                                                       | Zig `lib/std/zig/` (https://github.com/ziglang/zig/tree/master/lib/std/zig )          | **Low**. See below                                                                                                       |
| **No line count / volume rules**; "the solution is to change the function boundary, not to start counting lines"         | Go CodeReviewComments https://go.dev/wiki/CodeReviewComments#short-functions          | **Highest**. This repository has adopted this (2026-10-03 decision)                                                      |

### What Cannot Be Copied Directly

1. **Zig's "one stage per file"**. Zig's stage division is clear (`Ast.zig` / `Parse.zig` /
   `AstGen.zig` / `Zir.zig`), but **file scale is completely out of control**—`AstGen.zig` is 576
   KB, `Zir.zig` is 210 KB, `Ast.zig` is 148 KB
   (https://github.com/ziglang/zig/tree/master/lib/std/zig ). **"Divide directories by stage" is
   borrowable, "one stage per file" is not.**
2. **coherence / orphan rules**. RFC 2451 solves **cross-crate** impl conflicts; this repository's 3
   type representations and 2 operator enums are **single-crate** duplicate definitions,
   language-level coherence won't report an error, **must build own CI check**.

### Code Review Checklist

Every PR review goes through these one by one:

- [ ] **D0** Does it touch any table? What is the single authoritative implementation module for
      that table?
- [ ] **D1** Can the new concept be expressed with existing concept + parameter? If not, where is
      the reason written?
- [ ] **D2** Is it same-semantic with an existing concept? Where is the conversion? Are import
      aliases needed?
- [ ] **D3** Does the same behavior appear in ≥2 locations? Is a new entry point being added rather
      than registering in the stage table? Does it require modifying ≥3 synonymous mappings?
- [ ] **D4** Does this newly added code belong to a **responsibility class the target module already
      has**? Or is it the 2nd class or beyond?
- [ ] Has the **boundary** been eroded? New cross-layer dependencies / `include!` / `pub(crate)`
      cross-layer leaks?
- [ ] Are **mechanical problems** covered by tools (fmt / clippy / `build.rs` gate)? Does review
      only spend time on non-mechanical problems?

The last item is directly borrowed from Go's division of labor principle: **mechanical problems go
to tools, review only looks at non-mechanical problems** (https://go.dev/wiki/CodeReviewComments ).

## Detailed Design

### Deliverables List

| File                                                            | Content                                                                                                                   | Type           |
| --------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- | -------------- |
| `CONTRIBUTING.md` "Code Placement and Change Procedure" section | D0–D4 decision procedure + three prohibitions                                                                             | Document       |
| `scripts/ci/check-concepts.py`                                  | Prohibition 1 criteria A/B/C/D (parallel representations, disambiguating aliases, synonym table, insufficient call sites) | Gate script    |
| `scripts/ci/check-fanout.py`                                    | Prohibition 3 criteria A/B/C                                                                                              | Gate script    |
| `scripts/ci/check-boundary.py`                                  | Prohibition 2 criterion C (forbid `include!`, `pub(crate)` cross-layer leak only allowed to decrease, forbid L2→L3)       | Gate script    |
| `tools/code-tables` extension                                   | D0's table type registration                                                                                              | Reuse existing |

**No `baseline.toml`, no `clippy.toml` threshold configuration, no `check-scale.py`.** Scale
problems are solved by responsibility separation (Prohibition 2 Criterion A, manual judgment), no
numeric gates.

### How Responsibility Classes Are Defined

D4 Criterion A requires the fact "what responsibility classes does this module already have".
Approach:

1. In `01-routing.md`'s **target directory structure**, each directory has already written down its
   responsibility and boundary invariants—**this is the authoritative definition of responsibility
   classes**
2. When submitting a PR, declare in the PR description "which responsibility of `<directory name>`
   does this newly added code belong to"
3. If the target directory's responsibility list has no corresponding entry → this responsibility is
   a new class, **either extend the responsibility list (and explain why it should go here), or
   create a new directory**

`check-boundary.py` handles the mechanical part (cross-layer dependency / `include!` /
`pub(crate)`); responsibility assignment itself is judged by humans—this is why it needs to be
written in `CONTRIBUTING.md` rather than a script.

## Implementation Points

| Step | Content                                                                                                                                      | Acceptance                                                                                                                                                                 |
| ---- | -------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1    | Write D0–D4 and three prohibitions into `CONTRIBUTING.md`; responsibility classes reference `01-routing.md`'s directory responsibility table | Pass manual review                                                                                                                                                         |
| 2    | Implement `check-concepts.py` (Prohibition 1 four criteria), first run on **unmodified** code                                                | **Should report the existing 3 sets of operator enums, 2 sets of parallel type representations (`ir::Type` alias counted in)—only counts as effective gate if it reports** |
| 3    | Implement `check-fanout.py` (Prohibition 3)                                                                                                  | **Deliberately adding a 6th entry-point style wiring must be red**                                                                                                         |
| 4    | Implement `check-boundary.py` (Prohibition 2 Criterion C)                                                                                    | **Deliberately adding an L2→L3 reverse `use` must be red**; record initial value of `pub(crate)` leak count                                                                |
| 5    | Extend `tools/code-tables` to cover opcode / type table                                                                                      | Deliberately introducing zero-constructor variants must be red (depends on P6's T1)                                                                                        |

**Step 2 is the core acceptance of this phase**: `check-concepts.py` must be able to **report the
known 2 sets of operator enums on the current unmodified code**. If it cannot report, the criterion
design is wrong and the gate is a fake gate.

## Key Decisions and Reasons

- **No line count / volume gates of any kind** (2026-10-03 decision)—adopting Go's official position
  (https://go.dev/wiki/CodeReviewComments#short-functions ). The cause of this repository's scale
  problems is that `ir_gen.rs` simultaneously carries 6 responsibility classes, `From<BytecodeFile>`
  stuffs the entire file structure into one `From`—**remove the responsibilities, numbers naturally
  shrink, no numeric rules needed**. The originally envisioned `baseline.toml` ratchet and
  `clippy.toml` thresholds have all been abandoned.
- **The authoritative definition of responsibility classes goes in `01-routing.md`'s directory
  responsibility table**, not here—to avoid definition drift between two locations.
- **Only borrow two layers from rustc query, not the engine**—declarative stage table + provider
  unique-ownership. The core benefit of the query engine is incremental compilation, which this repo
  doesn't need.
- **Coherence idea must be self-implemented**—RFC 2451 governs cross-crate; this repository's
  problem is intra-crate duplication, the language won't report an error.

### Not-Adopted Directions

- **Line count ratchet / `baseline.toml`**—the cause of this repository's scale problems is
  responsibility not separated; setting thresholds only legitimizes 8448 lines, or produces one-time
  remediation pressure. **Abandoned.**
- **`clippy.toml`'s `too-many-lines-threshold` / clang-tidy `readability-function-size`**—same as
  above, **abandoned**.
- **Copying rustc query engine wholesale**—over-engineering, only borrow declarative table.

## Known Limitations and Risks

- **False positives in criteria**: Prohibition 1 Criterion A using "variant name overlap ≥ half" is
  heuristic; two truly unrelated enums may collide on names (e.g., `Field`). Mitigation: allow
  `// reason:` comment exemption.
- **`syn` analysis precision**: `syn`'s ability to parse macro-expanded forms is limited;
  definitions generated across macros may be missed. Mitigation: gate only reports "suspected",
  humans rule.
- **Responsibility determination cannot be mechanized**: Prohibition 2 A is manual judgment. This
  means **it can only be enforced by review, not by CI**—this is the mechanism's biggest soft spot.
  Mitigation: write the responsibility list into `01-routing.md` to make it checkable;
  `check-boundary.py` covers the mechanical part (cross-layer dependency / `include!` / `pub(crate)`
  leak).
- **Gate may be bypassed**: `// reason:` comment is a legitimate escape hatch. Mitigation: exemption
  count itself goes into `check-boundary.py`'s report.

> **All open questions originally listed in this section have been decided.** Item-by-item decisions
> are in [RFC-039 Decision Register](../../rfc/draft/039-compiler-architecture.md) (D1–D50). **This
> document leaves no pending items.**

## See Also

- [01-routing.md](01-routing.md) — **Authoritative definition of responsibility classes** (directory
  responsibilities and boundary invariants table), dependency direction spec, target directory
  structure, routing table C
- [02-stage-contract.md](02-stage-contract.md) — `Stage` declarative table (landing point for
  D0/D3-2)
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — Historical incidents this procedure will
  clean up
- [09-execution-wbs.md](09-execution-wbs.md) — P0's three-level task breakdown
- [RFC-009a Borrow Proof Pipeline](../../rfc/accepted/009a-borrow-proof-pipeline.md)
- [RFC-013 Error Code Specification](../../rfc/accepted/013-error-code-specification.md) —
  Specification source for existing gates
- `build.rs:19-55` — This repo's existing `panic!`-level gate example, direct basis for D0/D0 gate
  pattern
