# Coding Rules

> **This document is the sole authoritative source for YaoXiang code change rules.** It describes
> the rules precisely, without status diagnostics or historical justification. Historical incident
> evidence and decision rationale:
> [08-maintenance-mechanism.md](architecture/08-maintenance-mechanism.md) (2026-10 diagnostic
> record, archived with RFC-039); external convention references (Go / rustc, etc.) are in the same
> document. Execution entry: [HOWTO.md](HOWTO.md) (pre-work self-check) → this document (rules) → PR
> template (mandatory at submission).

## Part One: Three Prohibitions

### Prohibition 1: No Fabrication

Before adding any new `pub` type / enum / constant table / concept, **all** of the following four
criteria must pass for compliance. Any single hit is a violation:

| Criterion                        | Rule                                                                                                                                                             |
| -------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A. Overlapping responsibility    | The new item shares ≥ half of its variant names with an existing item AND responsibilities overlap → violation                                                   |
| B. Insufficient call sites       | New `pub` item has < 2 call sites (only the definition site + the single call site) → violation; cannot prove it is a concept rather than a local implementation |
| C. Requires disambiguating alias | Needs a `use … as XxxBinOp` style alias to distinguish items of the same semantics → violation                                                                   |
| D. Bridged by synonym table      | Bridges differences via hand-written string matching → violation; must switch to exhaustive matching or explicit conversion                                      |

### Prohibition 2: No Responsibility Accumulation

A module takes on only one class of responsibility. No line count / volume / file size gate — scale
issues are resolved by responsibility separation, not by numbers.

| Criterion                             | Rule                                                                                                                                                                                                                                                         |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| A. Responsibility class               | The new code does not belong to an **existing responsibility class** of the target module (class 2 and above) → violation; create a new module or move the existing code for that responsibility along with it. **Manual judgment, not machine-automatable** |
| B. Bypassing the authoritative source | Adds table entries without updating the sole authoritative source (bypassing the authoritative table with hand-written copies) → violation                                                                                                                   |
| C. Boundary erosion                   | Adds any cross-layer dependency (L2→L3, L4→L1/L2), `include!`, or `pub(crate)` cross-layer leakage → violation. **Machine-automatable, checked by CI**                                                                                                       |

### Prohibition 3: Refactor, Don't Patch

| Criterion                  | Rule                                                                                                                                                                      |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A. Synonymous duplication  | The same behavior appears in ≥ 2 places (aliasing / conversion / stage wiring / error mapping) → violation; must be lifted to a shared layer                              |
| B. Excessive fan-out       | A single change requires synchronously modifying ≥ 3 existing synonymous mappings → **deemed an architectural change**, stop work, go through the design document process |
| C. Adding new entry points | Adds "the Nth entry point / table entry" instead of registering it into the declarative stage table → violation                                                           |

## Part Two: Decision Procedure (D0–D4)

Each code change goes through five gates in order. Each is a decidable boolean condition, **stop on
first hit**:

```
D0  Gate         Does this change touch any existing table (error code / opcode / type / entry stage)?
                 ├─ Yes → Can you point to the module that is the sole authoritative implementation of that table?
                 │        ├─ Yes → Continue
                 │        └─ No  → ⛔ Stop. Establish the authoritative source first
                 └─ No  → Continue
                          ↓
D1  No Fabrication   After removing the new concept, can the same semantics be expressed with "existing concept + parameters"?
                     ├─ Yes → ⛔ Forbid adding. Record the reason in the PR
                     └─ No  → Continue
                              ↓
D2  No Parallelism   Is the new concept semantically equivalent to some existing concept (≥ half of variant names overlap)?
                     ├─ Yes → Is there a From/TryFrom conversion that is machine-checked?
                     │        ├─ Yes → Continue
                     │        └─ No  → ⛔ Merge into one (forbidden to use import aliases to distinguish)
                     └─ No  → Continue
                              ↓
D3  Patch?   Judge in order, stop on first hit:
             1) The same behavior needs to be duplicated in ≥ 2 places?            → Yes → ⛔ Must be lifted to a shared layer
             2) Need to add "the 6th entry point"?                                → Yes → ⛔ Forbid manual wiring, register in the stage table instead
             3) A single change requires modifying ≥ 3 synonymous mappings?      → Yes → ⛔ Deemed an architectural change, stop work and go through design document
             4) None of the above                                                  → Local patch is allowed, but must come with regression tests
                              ↓
D4  Responsibility?   Does the new code added this time belong to an existing responsibility class of the target module?
                       ├─ No  (class 2 and above) → ⛔ Create a new module, or move the existing code for that responsibility along with it
                       └─ Yes                       → Compliant
```

**Trade-off criteria (throughout D0–D4)**: The reasons for rejecting a proposal **may only** be
based on **correctness** and **readability**; "size of change surface", "whether a new mechanism is
introduced", and "implementation cost" **do not constitute reasons for rejection**. This runs in
parallel with Prohibition 1 — Prohibition 1 constrains "whether a concept is duplicated", while this
rule constrains "whether the reason for the trade-off stands": rejecting a proposal on the grounds
of change surface or new mechanism will be sent back directly by review.

Trade-offs requiring user adjudication (architectural choices, semantic deviations,
register/do-not-register, breaking changes, queue jumping) shall, per the repo root `AGENTS.md`
"When requesting user adjudication (mandatory)" section, provide all four items: situation +
evidence, per-option pros and cons, clear recommendation + cost, and call-out of breaking impact.

**Triage of semantic corrections (RFC-039 D59)**: D0–D4 are all retained; one additional step of
"error type" tagging — **types A/B/C** (old behavior was **non-success**: crash/stack overflow,
silent failure, silent mismatch, silent no-op, silent wrong answer) **may be fixed along the way
during refactoring**, with the conditions of **red-then-green + not changing the externally visible
behavior of programs that "previously succeeded" + registration in the same commit + baseline
regeneration in the same commit and manual review**; **types D/E/F** (changes the observable
behavior of previously successful programs, the external contract table, or RFC-level design
changes) **must first go through user adjudication**. For the full criteria, see
[07 §Triage Criteria for Semantic Corrections](architecture/07-equivalence-oracle.md) and RFC-039
decision register D59; add one line to the PR required block: "Error type (A–F)".

## Part Three: Red Lines (Review Must Send Back)

1. **Don't change code you haven't looked at** — without having opened the changed files or grepped
   for related references, you may not touch them
2. **No unimplemented leftovers for core functionality** — no adding `todo!()`, `unimplemented!()`,
   `Vec::new() // Not implemented yet` style silent drops, or indefinite "separate issues". Found
   items must be fixed immediately, or registered as current-phase tasks
3. **Don't delete tests, don't loosen criteria for a green light** — failure to comply is an
   implementation defect; report honestly and reassess, which is not a reason to modify the
   criteria; the number of tests may only increase, not decrease
4. **Don't use import aliases to bridge semantically equivalent concepts**
5. **Don't add new `include!`, don't add new reverse cross-layer dependencies, and `pub(crate)`
   cross-layer leakage may only decrease, not increase**
6. **The PR required block must be filled in truthfully** — unable to write the D0 authoritative
   source module name = didn't check, send back directly; "Error type (A–F, per D59)" missing or
   inconsistent with the actual change, send back as well

## Part Four: Division of Labor Between Machine and Human

**Hand mechanical problems to tools; review only looks at non-mechanical problems.**

| Category                                                                                                        | Executor                                                                  |
| --------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| Formatting, lint, table consistency, boundary erosion (Prohibition 2 C), test wiring, cross-layer dependencies  | Tools: `cargo fmt` / `clippy` / `build.rs` gate / `scripts/ci/check-*.py` |
| Responsibility class judgment (Prohibition 2 A), D3 "patch or architectural change" judgment, exemption reasons | Human: PR required block + review                                         |

**Corrections are written in place; no old-vs-new comparison is kept**: when you find a document or
comment that does not match reality, change the body to match the reality directly, and delete the
old wording; do not keep the old text with a "revision note" or "originally... now changed to..."
style comparison — the reader should not have to read the wrong version before reading the right
one. The **process evidence** of the correction (red→green, before/after diff, ruling number) is
written into the commit message or the WBS history record (`09a`), not into the corrected body. The
history record (`09a`) and work orders/change lists (e.g., `02` §Change List) are not bound by this
restriction.

## Part Five: Exemption Mechanism

- All exemptions from machine-checked criteria shall use the inline comment `// reason: <reason>`
- The number of exemptions itself is included in the gate report and reviewed item by item
- An exemption is an **exception voucher**, not a **convention**; when the same kind of exemption
  appears for the 2nd time, the correct action is to change the rule or the criterion, not to add a
  3rd exemption

## Part Six: Code Review Checklist

Each PR review goes through these items in order:

- [ ] **D0** Does it touch any table? Which module is the sole authoritative implementation of that
      table?
- [ ] **D1** Can the new concept be expressed with existing concepts + parameters? If not, where is
      the reason recorded?
- [ ] **D2** Is it semantically equivalent to an existing concept? Where is the conversion? Is an
      import alias needed?
- [ ] **D3** Does the same behavior appear in ≥ 2 places? Was a new entry point added instead of
      registering in the stage table? Will ≥ 3 synonymous mappings need to be changed?
- [ ] **D4** Does the new code added this time belong to an existing responsibility class of the
      target module?
- [ ] **Silent channel** For "register-consume" type mechanisms (one side registers data, the other
      side consumes, e.g., SemanticDB, proof_calls, obligation ledger), are the **direct-drive**
      tests for the producer and consumer delivered in the same batch? If consumer-side tests
      manually construct registered data while the producer side has no direct-drive evidence, the
      mechanism is superficially present but the data flow has never happened — send back (three
      existing cases: proof_calls, RFC-039 D54, RFC-039 D55)
- [ ] **Boundary** Is it being eroded? Any new cross-layer dependency / `include!` / `pub(crate)`
      cross-layer leakage?
- [ ] **Division of labor** Have mechanical problems been caught by tools? Is review being spent
      only on non-mechanical issues?
