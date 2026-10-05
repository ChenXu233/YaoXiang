# Coding Rules

> **This document is the sole authoritative source for YaoXiang code-change rules.** It describes
> the rules precisely, without current-state diagnosis or historical justification. Historical
> incident evidence and decision rationale can be found in
> [08-maintenance-mechanism.md](architecture/08-maintenance-mechanism.md) (October 2026 diagnostic
> record, archived with RFC-039); references to external conventions (Go / rustc, etc.) are in the
> same document. Execution entry points: [HOWTO.md](HOWTO.md) (pre-flight checklist before starting
> work) → this document (rules) → PR template (mandatory at submission time).

## Part 1: Three Prohibitions

### Prohibition 1: No Spurious Additions

Before adding any new `pub` type / enum / constant table / concept, all four of the following
criteria must pass for compliance. Any hit is a violation:

| Criterion                        | Rule                                                                                                                                                                 |
| -------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A. Overlapping responsibilities  | The new item's variant name overlaps with some existing item by ≥ 50% AND the responsibilities overlap → violation                                                   |
| B. Insufficient call sites       | The new `pub` item has < 2 call sites (only the definition site + the single call site) → violation; cannot prove it is a concept rather than a local implementation |
| C. Disambiguation alias required | Requires an alias like `use … as XxxBinOp` to distinguish same-semantic items → violation                                                                            |
| D. Bridged by synonym table      | Manual string matching is used to bridge the differences → violation, must be changed to exhaustive enumeration or explicit conversion                               |

### Prohibition 2: No Responsibility Accumulation

A module has only one category of responsibility. No line-count / volume / file-size gate — scale
issues are solved by separating responsibilities, not by numbers.

| Criterion                             | Rule                                                                                                                                                                                                                                                    |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A. Responsibility category            | The new code does not belong to an **existing responsibility category** of the target module (category 2 or higher) → violation; create a new module, or move the existing code for that responsibility over. **Manual judgment, cannot be mechanized** |
| B. Bypassing the authoritative source | New table entries added but the sole authoritative source not updated (bypassing the authoritative table with hand-written copies) → violation                                                                                                          |
| C. Boundary erosion                   | Any new cross-layer dependency (L2→L3, L4→L1/L2), `include!`, or `pub(crate)` cross-layer leak → violation. **Mechanizable, enforced by CI**                                                                                                            |

### Prohibition 3: No Patches When Refactoring Is Needed

| Criterion                 | Rule                                                                                                                                                                 |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A. Synonymous duplication | The same behavior appears in ≥ 2 places (alias / conversion / stage wiring / error mapping) → violation, must be hoisted to a shared layer                           |
| B. Excessive fan-out      | A single change requires synchronously modifying ≥ 3 existing synonymous mappings → **deemed an architectural change**, stop work, go through the design-doc process |
| C. New entry point        | Adding "the Nth entry / table entry" instead of registering into the declarative stage table → violation                                                             |

## Part 2: Decision Procedure (D0–D4)

Each code change passes through five gates in order. Each gate is a decidable boolean condition,
**stop on the first hit**:

```
D0  Gate       Does this change touch any existing table (error code / opcode / type / entry stage)?
                ├─ Yes → Can you point to the sole authoritative implementation module for that table?
                │         ├─ Yes → continue
                │         └─ No  → ⛔ Stop. Establish the authoritative source first
                └─ No  → continue
                        ↓
D1  No-spurious  After removing the new concept, can "existing concept + parameters" express the same semantics?
                  ├─ Yes → ⛔ Forbid addition. Record the reason in the PR
                  └─ No  → continue
                          ↓
D2  No-parallel  Is the new concept semantically the same as some existing concept (variant name overlap ≥ 50%)?
                  ├─ Yes → Is there a From/TryFrom conversion, and is it machine-checked?
                  │         ├─ Yes → continue
                  │         └─ No  → ⛔ Merge into one (forbidden to use import alias to distinguish)
                  └─ No  → continue
                          ↓
D3  Patch?       Judge in order, stop on the first hit:
                  1) The same behavior needs to be duplicated in ≥ 2 places?     → Yes → ⛔ Must hoist to a shared layer
                  2) Need to add "the 6th entry point"?                            → Yes → ⛔ Forbid hand wiring, register in the stage table instead
                  3) A single change requires modifying ≥ 3 synonymous mappings?  → Yes → ⛔ Deem as architectural change, stop and go through the design doc
                  4) None of the above                                             → Local patch allowed, but must include a regression test
                          ↓
D4  Responsibility?  Does the new code belong to an existing responsibility category of the target module?
                       ├─ No (category 2 or higher) → ⛔ Create a new module, or move the existing code for that responsibility over
                       └─ Yes → compliant
```

## Part 3: Red Lines (mandatory rejection in review)

1. **Don't change code you haven't read** — without opening the changed file, without grepping the
   related reference points, you may not modify it
2. **No "to be implemented" leftovers in core features** — no new `todo!()`, `unimplemented!`,
   silent-discard patterns like `Vec::new() // Not implemented yet`, or indefinite "separate issue"
   tracking. Fix on discovery, or register in the current-stage task list
3. **Don't delete tests or relax criteria to pass** — inability to do so is an implementation
   defect; report honestly and re-evaluate. It is not a reason to modify criteria; the number of
   tests can only increase, not decrease
4. **Don't use import aliases to bridge same-semantic concepts**
5. **Don't add `include!`, don't add cross-layer reverse dependencies; `pub(crate)` cross-layer
   leaks can only be reduced, not added**
6. **PR required blocks must be filled in truthfully** — unable to write the D0 authoritative source
   module name = didn't check, directly reject

## Part 4: Division of Labor Between Machine and Human

**Mechanize the mechanical; review only non-mechanical issues.**

| Category                                                                                                               | Executor                                                                  |
| ---------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| Formatting, lint, table consistency, boundary erosion (Prohibition 2 C), test wiring, cross-layer dependencies         | Tools: `cargo fmt` / `clippy` / `build.rs` gate / `scripts/ci/check-*.py` |
| Responsibility category judgment (Prohibition 2 A), the D3 "patch or architectural change" judgment, exemption reasons | Human: PR required blocks + review                                        |

## Part 5: Exemption Mechanism

- All machine-criteria exemptions use inline `// reason: <reason>` comments
- The number of exemptions itself enters the gate report, and is reviewed item by item
- Exemptions are **exception credentials**, not **convention**; when the same kind of exemption
  appears for the 2nd time, the correct action is to change the rule or the criterion, not to add a
  3rd exemption

## Part 6: Code Review Checklist

Each PR is reviewed against these items one by one:

- [ ] **D0** Does it touch any table? What is the sole authoritative implementation module of that
      table?
- [ ] **D1** Can the new concept be expressed with existing concept + parameters? If not, where is
      the reason written?
- [ ] **D2** Is it semantically the same as an existing concept? Where is the conversion? Does it
      need an import alias?
- [ ] **D3** Does the same behavior appear in ≥ 2 places? Does it add an entry point rather than
      register in the stage table? Does it require modifying ≥ 3 synonymous mappings?
- [ ] **D4** Does the new code belong to an existing responsibility category of the target module?
- [ ] **Boundary** Is boundary being eroded? Any new cross-layer dependency / `include!` /
      `pub(crate)` cross-layer leak?
- [ ] **Division of Labor** Have mechanical issues been caught by tools? Does the review only spend
      time on non-mechanical issues?
