---
title: "RFC [Number]: [Proposal Title]"
status: "Draft"
author: "[Author Name]"
created: "YYYY-MM-DD"
updated: "YYYY-MM-DD"
group: ""  # Optional: Parent RFC number (e.g., "rfc-014"), used for sub-RFC attribution
---

# RFC [Number]: [Proposal Title]

> **Reference**: See [Complete Example](EXAMPLE_full_feature_proposal.md) to learn how to write an RFC.

## Summary

Summarize in 1-2 sentences the problem this RFC aims to solve and the proposed solution.

## Motivation

### Why is this feature/change needed?

Explain why this RFC is necessary, what problem it solves, or what need it fulfills.

### Current Problems

Describe the issues with the current state, using concrete examples if available.

## Proposal

### Core Design

Describe the proposed solution, including core concepts and data structures.

### Examples

Provide code examples to illustrate the usage of the proposal:

```yaoxiang
# Example code
```

### Syntax Changes

If the proposal involves syntax changes, list the comparison before and after:

| Before | After |
|--------|-------|
| Syntax A | Syntax B |

## Detailed Design

### Type System Impact

Describe the impact on the type system, whether new types or type constraints are introduced.

### Runtime Behavior

Describe changes in runtime behavior (if any).

### Compiler Changes

List the compiler components that need to be modified.

### Backward Compatibility

Evaluate whether this is backward compatible and how to handle existing code.

## Trade-offs

### Advantages

- List the advantages of adopting this proposal

### Disadvantages

- List the disadvantages or risks of adopting this proposal

## Alternatives

List alternatives that were considered and why they were not chosen.

## Implementation Strategy

### Dependencies

List other RFCs or features that this RFC depends on.

### Risks

Identify potential risks in implementation.

## Open Questions

List questions that need further discussion.

Format:
```markdown
- [ ] Question description (@username: additional notes)
```

---

## Appendices (Optional)

### Appendix A: Design Discussion Records

> Used to record detailed discussions during the design decision-making process.

#### Discussion Template

```markdown
### [Discussion Topic]

> **Discussion Status**: [Open | Resolved]
> **Initiator**: @username
> **Date**: YYYY-MM-DD

#### Problem Description

[Describe the issue that needs to be discussed]

#### Alternatives

| Option | Description | Advantages | Disadvantages |
|--------|-------------|------------|---------------|
| A | ... | ... | ... |
| B | ... | ... | ... |

#### Discussion Records

- [Date] @username: [Opinion]
- [Date] @username: [Opinion]

#### Resolution

[Final decision and rationale]
```

#### Discussion Management Rules

| Phase | Action |
|-------|--------|
| In Discussion | Record in appendix, keep open status |
| Resolved | Update resolution, mark status, **update relevant content in main text** |
| Abandoned | Mark reason for abandonment, keep record for reference |

> **Important**: Once a discussion is concluded, the relevant content in the main text must be updated! Discussion records may be retained as historical reference.

---

### Appendix B: Design Decision Records

> Record the design decisions that have been confirmed in the RFC and their rationale.

| Decision | Determination | Date | Recorder |
|----------|---------------|------|----------|
| ... | ... | ... | ... |

---

### Appendix C: Glossary

| Term | Definition |
|------|------------|
| ... | ... |

---

## References

- Links to related documents or external resources
- Similar implementations or references from other languages

---

## Lifecycle and Destination

RFCs have the following state transitions:

```
┌─────────────┐
│   Draft     │  ← Author creates
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   Under     │  ← Community discussion
│  Review     │
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  Accepted   │    │  Rejected   │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   accepted/ │    │    rfc/     │
│  (Official │    │  (Kept in   │
│   Design)  │    │   place)    │
└─────────────┘    └─────────────┘
```

### Status Description

| Status | Location | Description |
|--------|----------|-------------|
| **Draft** | `docs/design/rfc/` | Author's draft, awaiting submission for review |
| **Under Review** | `docs/design/rfc/` | Open for community discussion and feedback |
| **Accepted** | `docs/design/accepted/` | Becomes an official design document, enters implementation phase |
| **Rejected** | `docs/design/rfc/` | Retained in the RFC directory, status updated |

### Actions After Acceptance

1. Move the RFC to the `docs/design/accepted/` directory
2. Update the filename to a descriptive name (e.g., `enhanced-pattern-matching.md`)
3. Update the status to "Official"
4. Update the status to "Accepted", and add the acceptance date

### Actions After Rejection

1. Retain in the `docs/design/rfc/` directory
2. Add the reason for rejection and the date at the top of the file
3. Update the status to "Rejected"

### Actions After Discussion Conclusion

When consensus is reached on an open question:

1. **Update Appendix A**: Fill in the "Resolution" under the discussion topic
2. **Update Main Text**: Sync the decision to the main body of the document
3. **Record Decision**: Add to "Appendix B: Design Decision Records"
4. **Mark Question**: Check `[x]` in the "Open Questions" list

---

> **Note**: RFC numbers are only used during the discussion phase. After acceptance, remove the number and use a descriptive filename.