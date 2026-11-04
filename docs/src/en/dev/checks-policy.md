# Check Admission and Retirement Policy (Draft, 2026-11-04)

> Status: **Proposed**. Not yet submitted or integrated into any gate. Before adoption: ① the user
> must resolve the four questions in Section 11; ② a sidebar entry and `check-docs-orphan`
> registration must be added; ③ every item must be aligned with the “red lines” in
> [coding-rules](coding-rules.md).

## 0. In One Sentence

**Checks exist for only two reasons: to block, or to catch.** If neither applies, a check merely
taxes people who follow the rules.

## 1. What Counts as a “Check”

This document treats each of the following as a check, subject to the same gate:

- Mandatory validations in CI jobs, commit hooks, and build scripts;
- Regression tests, unit tests, and snapshot tests;
- Corpus differential baselines and IR snapshot baselines;
- Mechanical items in review checklists (for example, “reference points must be grepped” when
  performed manually) count as human-review criteria, not checks.

**Human-review criteria** (written into documentation and judged by reviewers) are not governed by
this document—they have no execution cost, but they also provide no automatic interception.

## 2. Admission Gate: Two Questions

Every new check must answer the following questions before it is introduced:

**Question 1: Can it be bypassed?**  
Can it block at an **unbypassable** point—through CI enforcement, a commit hook, the type system, a
compilation failure, or a test failure? If bypassing it takes only one line of code or one comment,
the answer is no.

**Question 2: Has it caught any real issues?**  
Register a specific error that could recur: which change, which defect, and what would happen if the
check did not block it.

| Question 1   | Question 2        | Action                                                                                       |
| ------------ | ----------------- | -------------------------------------------------------------------------------------------- |
| Unbypassable | Has detections    | **Keep**, and add it to CI                                                                   |
| Unbypassable | No detections yet | **Cold start**: first prove that it can turn red (see §2.1), then observe it for 3 months    |
| Bypassable   | Has detections    | **Rework**: move it to an observable location, or downgrade it to a human-review criterion   |
| Bypassable   | No detections     | **Do not write it**. Turn it into a sentence in the documentation, or do not write it at all |

### 2.1 Cold-Start Clause (a Supplement to the “Two Questions”)

The requirement for “real detections” creates a circular dependency: a new check has not yet had an
opportunity to detect anything and would therefore be rejected under the “no detections” rule.

Every new check must therefore first pass a **proof that it can turn red**: use a mechanically
reversible mutation (break the code under test → the check must turn red → restore it → compare each
file against its original state) to prove that it has detection capability. A check that cannot
prove that it can turn red is not a check; it is a ritual.

> Repository example: in the 6.8.3 batch of 2026-11-03, two pieces of “defensive alignment” code
> were deleted while all 2950 tests were green, and no criterion turned red. The unit criterion
> added afterward had to use a mutation to prove that “deleting it turns it red” before it was
> accepted.

## 3. The Four Types of Checks and Their Gate Status

| Category             | What it asserts                                                                                  | During implementation refactoring             | Gate status                                                                   |
| -------------------- | ------------------------------------------------------------------------------------------------ | --------------------------------------------- | ----------------------------------------------------------------------------- |
| **Invariant**        | A property that must hold regardless of how the implementation is written                        | Should not turn red                           | **Gate**                                                                      |
| **Differential**     | Agreement with expectations derived from SPEC/RFC                                                | Should not turn red (unless the SPEC changes) | **Gate**, but it must cite a SPEC clause (§6)                                 |
| **Characterization** | A choice made by the current implementation (rendered text, message wording, exact line numbers) | Will turn red                                 | **Advisory only; non-blocking**                                               |
| **Frozen**           | Existing observable behavior (what users can see)                                                | Turns red                                     | Gate, but it may be reclassified by a formal ruling and the change registered |

**Criterion**: Which category a check belongs to depends on “whether a correct alternative
implementation would cause it to turn red.” If it turns red, it is characterization.

## 4. Four Questions for Regression Tests

Tests are also checks. Answer these four questions before introducing one:

1. **What user-visible breakage does it prevent?** If you cannot say, it is not a regression test.
2. **What is the minimal reproduction?** One command, one file, or a few lines of code.
3. **Why can a cheaper check not catch it?** If the type system, a compiler warning, or a one-line
   grep can block it, do not write the test.
4. **Is the cost of running it every day worthwhile?** Starting three processes and running for ten
   minutes to guard against a once-a-year failure taxes everyone.

Write the answers in a header comment in the test file or in the commit message.

## 5. Two Common Misjudgments

**Misjudgment 1: Treating “low probability” as a reason to delete.**  
Even a once-a-year failure may be irreversible (credential leakage, concurrent message loss), so a
cheap guard should remain. Judge cost and benefit, not a probability threshold.

**Misjudgment 2: Treating an assertion that “guards against honest people, not bad actors” as a
guard.**  
“The code calls a certain function,” “a certain string does not appear in the file,” “this helper
function behaves as expected”—an honest implementation passes naturally, and bypassing the assertion
takes one line. It provides a sense of ritual, not safety. If you want to block it, block it where
the breakage is observable.

## 6. The SPEC Takes Priority

**The oracle for a differential check must be the SPEC/RFC, not yesterday’s compiler.**

If yesterday’s implementation was itself wrong, freezing it as a baseline promotes the error to gate
status. In the repository’s 2026-11-02/04 6.8.x batch, several decision points (which layer the
implementation header of a curried declaration binds to, the denominator of `E1112`, dependent-type
value parameters, and borrow/value parameter alignment) had no corresponding clauses in the
specification. Disputes were ultimately settled based on “the existing tests are green”—using
history as the specification.

This leads to three constraints:

1. **Undefined behavior must not be frozen**. Constructs not covered by a clause may only be marked
   `characterization` and must not enter the differential gate.
2. **Corpus entries must cite clauses**. Each fixture must state which specification clause it
   proves; every clause must have at least one fixture. Both empty clauses and empty fixtures must
   be listable.
3. **Criterion for changing a criterion**: If changing a criterion reveals that no clause supports
   it, then it was characterization rather than an invariant from the beginning—it should be
   **downgraded**, rather than “made green and called done.”

## 7. Cost Accounting and Retirement

Every check must record three things: **number of detections in the last 12 months**, **cost per
run**, and **bypassability**.

A check may be retired only if all of the following conditions hold:

- Zero detections in the last 12 months;
- It is bypassable (an honest implementation passes naturally, and bypassing it takes one line);
- Its per-run cost is significant.

Retirement methods: downgrade it to an advisory (not included in CI), convert it into a human-review
criterion, or delete it. **The rationale and alternative must be recorded.**

### 7.1 Relationship to the “Red Lines”

The red lines in [coding-rules](coding-rules.md) prohibit “relaxing an assertion because it turned
red”—that is, **changing a criterion to make it green**.

This document permits “deleting a check with no detections on the basis of an argument”—the criteria
are different:

- The red line targets the **motive** (to make CI green);
- This document targets **cost and benefit** (whether the check is worth its cost).

When the two conflict, use this order of adjudication: first ask, “Is it red now because the
implementation is wrong, or because the criterion is wrong?” If that question cannot be answered,
**do not delete it**.

## 8. Repository Measurements (Baseline, 2026-11-02/04 Batch)

| Method                                    | Scale                 | Real issues caught  | Notes                                                     |
| ----------------------------------------- | --------------------- | ------------------- | --------------------------------------------------------- |
| IR snapshots (zero-drift gate)            | 224                   | 0                   | Instead, it caused an entire batch to be rolled back once |
| Structural/unit tests                     | Approximately 2950    | 2                   | const-generics regression, curry-refinement regression    |
| Corpus differential + baseline comparison | 375 rows + 63 cells   | 1 category, 9 cells | It was the only thing that exposed a silent wrong answer  |
| Adversarial review                        | 4 rounds, 15 findings | 3 Class A findings  | Silent wrong answer and false positive                    |

Conclusion: **snapshot checks had a detection rate of 0** in this batch; the checks that actually
found problems were differential checks and human review.

## 9. Migration Path (Four Independently Reversible Stages)

1. **Establish real gates**: `check` passes ⇒ `run` contains no type errors (a soundness criterion).
   This does not depend on a new mechanism and can be run directly against the existing corpus.
2. **Split fake gates**: Downgrade IR snapshots from a “zero-drift gate” to a “change advisory”; tag
   characterization tests with their category.
3. **Make differential checks cite specifications**: Add SPEC clause references to corpus entries;
   add bidirectional coverage checks.
4. **Codify the rules**: Incorporate this document into [coding-rules](coding-rules.md) and
   [test-specification](test-specification.md).

## 10. Risks and Counterarguments

- **“No detections today ≠ no detections tomorrow”**: This is why there is a cold-start period and a
  proof that a check can turn red. Retiring a check with zero detections requires 12 months of
  observation, not a single review decision.
- **No one will watch for rendering regressions after snapshots are downgraded**: Accept this.
  Rendering regressions will be surfaced to humans in the PR through a change advisory and will not
  block the merge. The cost is that regressions may be missed; the benefit is that unrelated
  refactoring no longer incurs rework.
- **Limits of formal verification**: The language’s support for formal verification applies to
  **user programs**, which can be proven; the compiler implementation (Rust, approximately 160,000
  lines) is outside its scope. Formal verification can prevent “a stated property from being
  violated,” but it cannot prevent “a property no one thought to state”—the latter can only be
  discovered by writing a SPEC and conducting an adversarial review.
- **Psychological cost of deleting checks**: Deleting a check that has stayed green for a long time
  can make people uneasy. Offset this with the three recorded facts in §7: zero detections,
  bypassability, and high cost are facts, not feelings.

## 11. Pending Decisions

1. Should this document be incorporated into `coding-rules` as formal policy? (Your review of the
   wording is needed.)
2. Should the soundness criterion from migration stage 1 be implemented now?
3. If IR snapshots are downgraded to advisories, do we accept the cost of possibly missing rendering
   regressions?
4. Should SPEC references for corpus entries be **mandatory** (red if missing) or **recommended**
   (with the coverage check providing advisory warnings only)?
