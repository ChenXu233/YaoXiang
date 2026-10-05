# Code Writing Rules (Coding Rules)

> **This document is the sole authoritative body of YaoXiang code change rules.** It precisely
> describes the rules, without status diagnosis and historical argumentation. Historical incident
> evidence and decision rationale see
> [08-maintenance-mechanism.md](../design/compiler-architecture/08-maintenance-mechanism.md)
> (2026-10 diagnostic record, archived with RFC-039); external convention references (Go / rustc
> etc.) see the same document. Execution entry: [HOWTO.md](../design/compiler-architecture/HOWTO.md)
> (start-work self-check) → This document (rules) → PR template (mandatory at submission).

## Part One: Three Prohibitions

### Prohibition One: No Fabrication

Before adding any new `pub` type / enum / constant table / concept, the following four criteria
**must all pass** to be compliant. Any single hit is a violation:

| Criterion                        | Rule                                                                                                                                                               |
| -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| A. Responsibility Overlap        | The new item has variant-name overlap ≥ half with some existing item AND has overlapping responsibilities → Violation                                              |
| B. Insufficient Call Sites       | The new `pub` item has call sites < 2 (only the definition site + the sole call site) → Violation; cannot prove it is a concept rather than a local implementation |
| C. Disambiguation Alias Required | Needs a `use … as XxxBinOp` style alias to distinguish same-semantic items → Violation                                                                             |
| D. Bridged by Synonym Table      | Relies on hand-written string matching to bridge differences → Violation; must change to exhaustive enumeration or explicit conversion                             |

### Prohibition Two: No Responsibility Accumulation

A module only takes on one class of responsibility. No line count / volume / file size threshold —
scale problems are solved by responsibility separation, not by numbers.

| Criterion                             | Rule                                                                                                                                                                                                                                                     |
| ------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A. Responsibility Category            | New code does not belong to the target module's **existing responsibility category** (2nd class or above) → Violation; create a new module, or move the existing code of that responsibility over together. **Manual judgment, not machine-automatable** |
| B. Bypassing the Authoritative Source | A new table entry is added but the unique authoritative source is not updated (hand-written copy that bypasses the authoritative table) → Violation                                                                                                      |
| C. Boundary Erosion                   | Any new cross-layer dependency (L2→L3, L4→L1/L2), `include!`, or `pub(crate)` cross-layer leak → Violation. **Machine-automatable, checked by CI**                                                                                                       |

### Prohibition Three: This Refactor, Not Patches

| Criterion                 | Rule                                                                                                                                                                            |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A. Synonymous Duplication | The same behavior appears in ≥2 places (alias / conversion / stage wiring / error mapping) → Violation; must be lifted to the shared layer                                      |
| B. Excessive Fan-out      | One change requires synchronized modification of ≥3 existing synonymous mappings → **Determined as an architectural change**, stop work, go through the design document process |
| C. New Entry Point        | Adding "the Nth entry / table item" instead of registering into the declarative stage table → Violation                                                                         |

## Part Two: Decision Procedure (D0–D4)

Each code change goes through five gates in order. Each gate is a decidable boolean condition,
**stop on hit**:

```
D0  Gate       Does this change touch any existing table (error code / opcode / type / entry stage)?
               ├─ Yes → Can you point out the unique authoritative implementation module of that table?
               │         ├─ Yes → Continue
               │         └─ No  → ⛔ Stop. First establish the authoritative source
               └─ No  → Continue
                      ↓
D1  Anti-Fab.  After removing the new concept, can "existing concept + parameters" express the same semantics?
               ├─ Yes → ⛔ Prohibit the new addition. Reason recorded in the PR
               └─ No  → Continue
                      ↓
D2  Anti-Parallel  Is the new concept semantically identical to some existing concept (variant-name overlap ≥ half)?
                   ├─ Yes → Is there a From/TryFrom conversion with machine check?
                   │         ├─ Yes → Continue
                   │         └─ No  → ⛔ Merge into one (prohibit using import alias to distinguish)
                   └─ No  → Continue
                          ↓
D3  Patch?      Judge in order, stop on hit:
                1) Same behavior needs to be duplicated in ≥2 places?         → Yes → ⛔ Must be lifted to the shared layer
                2) Need to add "the 6th entry point"?                          → Yes → ⛔ Prohibit manual wiring, change to register in the stage table
                3) One change needs to modify ≥3 synonymous mappings?          → Yes → ⛔ Determined as architectural change, stop work and go through the design document
                4) None of the above                                           → Allow local patches, but must attach regression tests
                          ↓
D4  Responsibility?  Does this new code belong to the target module's existing responsibility category?
                       ├─ No  (2nd class or above) → ⛔ Create a new module, or move the existing code of that responsibility over together
                       └─ Yes → Compliant
```

## Part Three: Red Lines (review must reject)

1. **Don't change code you haven't looked at** — without having opened the changed file, without
   having grepped the relevant reference points, you must not touch it
2. **No core functionality left as TODO** — do not add `todo!()`, `unimplemented!`, or
   `Vec::new() // Not implemented yet`-style silent discarding, or indefinite "independent issue".
   Found it, fix it, or register it into the current phase task
3. **Don't delete tests, don't relax criteria to get a green light** — inability to do it is an
   implementation defect; report truthfully and re-evaluate, not a reason to modify the criteria;
   the test count may only increase, never decrease
4. **Do not use import aliases to bridge same-semantic concepts**
5. **Do not add new `include!`, do not add new cross-layer reverse dependencies, and `pub(crate)`
   cross-layer leaks may only decrease, never increase**
6. **PR required fields must be filled in truthfully** — not being able to write down the D0
   authoritative source module name = didn't check, directly reject

## Part Four: Division of Labor Between Machine and Human

**Mechanical problems go to tools; review only looks at non-mechanical problems.**

| Category                                                                                                             | Executor                                                                  |
| -------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| Formatting, lint, table consistency, boundary erosion (Prohibition Two C), test wiring, cross-layer dependencies     | Tools: `cargo fmt` / `clippy` / `build.rs` gate / `scripts/ci/check-*.py` |
| Responsibility category judgment (Prohibition Two A), D3 "patch or architectural change" judgment, exemption reasons | Human: PR required fields + review                                        |

## Part Five: Exemption Mechanism

- Machine-criterion exemptions all use `// reason: <reason>` inline comments
- The exemption count itself enters the gate report, reviewed item by item during review
- Exemptions are **exception credentials**, not **convention**; when the same kind of exemption
  appears a 2nd time, the correct action is to change the rule or criterion, not to add a 3rd
  exemption

## Part Six: Code Review Checklist

Each PR's review goes through these item by item:

- [ ] **D0** Did it touch any table? What is the unique authoritative implementation module of that
      table?
- [ ] **D1** Can the new concept be expressed with existing concept + parameters? If not, where is
      the reason written?
- [ ] **D2** Is it semantically identical to an existing concept? Where is the conversion? Does it
      need an import alias?
- [ ] **D3** Does the same behavior appear in ≥2 places? Was a new entry point added instead of
      registering in the stage table? Does it require modifying ≥3 synonymous mappings?
- [ ] **D4** Does this new code belong to the target module's existing responsibility category?
- [ ] **Boundary** Is the boundary eroded? New cross-layer dependency / `include!` / `pub(crate)`
      cross-layer leak?
- [ ] **Division of Labor** Are mechanical problems already caught by tools? Is the review only
      spent on non-mechanical problems?
