# Coding Rules

> **This document is the single authoritative source for YaoXiang code change rules.** It describes
> the rules precisely, without status diagnostics or historical justifications. Historical incident
> evidence and decision rationale are in
> [08-maintenance-mechanism.md](architecture/08-maintenance-mechanism.md) (2026-10 diagnostic
> record, archived with RFC-039); references to external conventions (Go / rustc, etc.) are also in
> that document. Execution entry: [HOWTO.md](HOWTO.md) (pre-work self-check) → this document (rules)
> → PR template (mandatory at submission).

## Part One: Three Prohibitions

### Prohibition 1: No Inventing

Before adding any `pub` type / enum / constant table / concept, the following four criteria must
**all pass** to be compliant. Hitting any one is a violation:

| Criterion                          | Rule                                                                                                                                                                    |
| ---------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A. Overlapping responsibility      | The new item's variant names overlap with an existing item by ≥ half, and responsibilities overlap → violation                                                          |
| B. Insufficient call sites         | The new `pub` item has < 2 call sites (only the definition site + single call site) → violation; it cannot be proven to be a concept rather than a local implementation |
| C. Disambiguating aliases required | Needs a `use … as XxxBinOp`-style alias to distinguish items of the same semantics → violation                                                                          |
| D. Bridged by synonym table        | Bridges differences via hand-written string matching → violation; must be replaced with exhaustive matching or explicit conversion                                      |

### Prohibition 2: No Accumulating Responsibilities

A module takes on only one class of responsibility. No line count / volume / file size
threshold—scale problems are solved by separating responsibilities, not by numbers.

| Criterion                             | Rule                                                                                                                                                                                                                                                     |
| ------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A. Responsibility class               | The new code does not belong to the target module's **existing responsibility class** (2nd class or beyond) → violation; create a new module or move the existing code for that responsibility along with it. **Manual judgment, not machine-checkable** |
| B. Bypassing the authoritative source | Adding a table entry without updating the single authoritative source (hand-written copies bypassing the authoritative table) → violation                                                                                                                |
| C. Boundary erosion                   | Adding any cross-layer dependency (L2→L3, L4→L1/L2), `include!`, or `pub(crate)` cross-layer leakage → violation. **Machine-checkable, enforced by CI**                                                                                                  |

### Prohibition 3: No Patching Where Refactoring is Required

| Criterion                 | Rule                                                                                                                                                                      |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A. Synonymous duplication | The same behavior appears at ≥ 2 places (aliasing / conversion / phase wiring / error mapping) → violation; must be lifted to a shared layer                              |
| B. Excessive fan-out      | A single change must synchronously modify ≥ 3 existing synonymous mappings → **determined as an architectural change**, stop work, go through the design document process |
| C. New entry point        | Adding "the Nth entry point / table entry" instead of registering it in the declarative phase table → violation                                                           |

## Part Two: Decision Procedure (D0–D4)

Every code change goes through five gates in order. Each gate is a decidable boolean condition,
**stop on the first hit**:

```
D0  Gate     Does this change touch any existing table (error code / opcode / type / entry phase)?
              ├─ Yes → Can you point to that table's single authoritative implementation module?
              │         ├─ Yes → Continue
              │         └─ No  → ⛔ Stop. Establish the authoritative source first
              └─ No  → Continue
                     ↓
D1  Anti-invent  After removing the new concept, can the same semantics be expressed with
                  "existing concept + parameter"?
                  ├─ Yes → ⛔ Prohibited from adding. Reason recorded in PR
                  └─ No  → Continue
                          ↓
D2  Anti-parallel Is the new concept semantically identical to some existing concept
                   (variant name overlap ≥ half)?
                   ├─ Yes → Is there a From/TryFrom conversion and is it machine-checked?
                   │          ├─ Yes → Continue
                   │          └─ No  → ⛔ Merge into one (no using import alias to distinguish)
                   └─ No  → Continue
                            ↓
D3  Patch?        Judge in order, stop on first hit:
                   1) Same behavior needs to be copied at ≥ 2 places?     → Yes → ⛔ Must lift to shared layer
                   2) Need to add "the 6th entry point"?                 → Yes → ⛔ No hand-wiring, register in the phase table instead
                   3) One change needs to modify ≥ 3 synonymous mappings? → Yes → ⛔ Determined as architectural change, stop and go through design document
                   4) None of the above                                  → Local patch allowed, but must come with regression tests
                            ↓
D4  Responsibility?  Does the new code belong to an existing responsibility class of the target module?
                        ├─ No (2nd class or beyond) → ⛔ Create a new module, or move the existing code for that responsibility over as well
                        └─ Yes → Compliant
```

**Trade-off criteria (throughout D0–D4)**: Reasons for rejecting a proposal **may only** be based on
**correctness** and **readability**; "size of the change", "whether it introduces a new mechanism",
and "implementation cost" **do not constitute reasons for rejection**. This runs parallel to
Prohibition 1—Prohibition 1 constrains "whether a concept is duplicated", while this rule constrains
"whether the rejection reasoning is valid": anyone who rejects a proposal on the grounds of change
size or new mechanism will be sent back at review.

Trade-offs that need the user to make the call (architectural choices, semantic deviations, register
or not, breaking changes, queue jumping) must, per the repository root `AGENTS.md` "When Requesting
User Adjudication (mandatory)", provide all four items: situation + evidence, per-option pros and
cons, clear recommendation + cost, and explicit call-out of breaking changes.

## Part Three: Red Lines (review will send these back)

1. **Don't change code you haven't looked at**—without having opened the changed file or grepped the
   relevant reference points, you are not allowed to touch it.
2. **Core functionality must not be left as TODO**—no adding `todo!()`, `unimplemented!()`,
   `Vec::new() // Not implemented yet`-style silent discarding, or indefinite "separate issue". On
   discovery, fix it, or register it in the current phase's tasks.
3. **Don't delete tests or loosen criteria for a green light**—inability to do so is an
   implementation defect; report it honestly and reassess; that is not a reason to modify criteria;
   the number of tests may only increase, not decrease.
4. **Don't use import aliases to bridge same-semantic concepts**.
5. **Don't add `include!`, don't add cross-layer reverse dependencies, `pub(crate)` cross-layer
   leakage may only decrease, never increase**.
6. **Required PR blocks must be filled in truthfully**—not being able to write down the D0
   authoritative source module name means you didn't check; send it back directly.

## Part Four: Division of Labor Between Machine and Human

**Let tools handle mechanical problems; review only looks at non-mechanical problems.**

| Category                                                                                                               | Executor                                                                   |
| ---------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| Formatting, lint, table consistency, boundary erosion (Prohibition 2 C), test wiring, cross-layer dependencies         | Tools: `cargo fmt` / `clippy` / `build.rs` gates / `scripts/ci/check-*.py` |
| Responsibility class determination (Prohibition 2 A), D3 "patch or architectural change" judgment, exemption rationale | Human: required PR blocks + review                                         |

## Part Five: Exemption Mechanism

- Exemptions for machine-checkable criteria uniformly use `// reason: <reason>` inline comments.
- The number of exemptions themselves enters the gate report; review goes through them one by one.
- Exemptions are **exception credentials**, not **convention**; when the same kind of exemption
  appears a 2nd time, the correct action is to change the rule or the criterion, not to add a 3rd
  exemption.

## Part Six: Code Review Checklist

Each PR's review goes through the following items one by one:

- [ ] **D0** Does this touch any table? Which module is that table's single authoritative
      implementation?
- [ ] **D1** Can the new concept be expressed with existing concept + parameter? If not, where is
      the reason written?
- [ ] **D2** Is it semantically identical to an existing concept? Where is the conversion? Does it
      need an import alias?
- [ ] **D3** Does the same behavior appear at ≥ 2 places? Is an entry point added rather than
      registered in the phase table? Does it require modifying ≥ 3 synonymous mappings?
- [ ] **D4** Does the new code belong to an existing responsibility class of the target module?
- [ ] **Silent channel** For "register-consume"-type mechanisms (one side registers data, the other
      side consumes, e.g. SemanticDB, proof_calls, obligation ledgers), are the producer and
      consumer **direct-drive** tests delivered in the same batch? Consumer tests that
      hand-construct registered data while the producer has no direct-drive evidence = mechanism
      appears to exist but the data flow never actually happened, send it back (three prior cases:
      proof_calls, RFC-039 D54, RFC-039 D55)
- [ ] **Boundary** Has it been eroded? New cross-layer dependency / `include!` / `pub(crate)`
      cross-layer leakage?
- [ ] **Division of labor** Have mechanical problems been caught by tools? Does the review only
      spend time on non-mechanical problems?
