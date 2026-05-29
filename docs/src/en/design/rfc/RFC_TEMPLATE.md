---
title: "RFC Template"
---

# RFC [Number]: [Proposal Title]

> **Status**: [Draft | Under Review | Accepted | Rejected]
> **Author**: [Author Name]
> **Created**: [YYYY-MM-DD]
> **Last Updated**: 2026-02-12

> **Reference**: See [Full Example](EXAMPLE_full_feature_proposal.md) to learn how to write an RFC.

## Summary

A brief 1-2 sentence summary of the problem this RFC aims to solve and the proposed solution.

## Motivation

### Why is this feature/change needed?

Explain why this RFC is necessary, what problem it solves or what need it fulfills.

### Current Problem

Describe the issues with the current state, using concrete examples where applicable.

## Proposal

### Core Design

Describe the proposed solution, including core concepts and data structures.

### Examples

Provide code examples to illustrate how the proposal would be used:

```yaoxiang
# Example code
```

### Syntax Changes

If the proposal involves syntax changes, list before/after comparisons:

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

Evaluate whether this is backward compatible and how existing code will be handled.

## Trade-offs

### Advantages

- List the benefits of adopting this proposal

### Disadvantages

- List the disadvantages or risks of this proposal

## Alternatives

List the alternative solutions that were considered and why they were not chosen.

## Implementation Strategy

### Phased Approach

Describe how this feature will be implemented in phases.

### Dependencies

List other RFCs or features this RFC depends on.

### Risks

Identify potential risks in the implementation.

## Open Questions

List issues that need further discussion.

Format:
```markdown
- [ ] Problem description (@username: additional notes)
```

---

## Appendices (Optional)

### Appendix A: Design Discussion Log

> Used to record detailed discussions during the design decision-making process.

#### Discussion Template

```markdown
### [Discussion Topic]

> **Discussion Status**: [Open | Resolved]
> **Initiator**: @username
> **Date**: YYYY-MM-DD

#### Problem Description

[Describe the issue to be discussed]

#### Options

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | ... | ... | ... |
| B | ... | ... | ... |

#### Discussion Log

- [Date] @username: [comment]
- [Date] @username: [comment]

#### Resolution

[Final decision and rationale]
```

#### Discussion Management Rules

| Phase | Action |
|-------|--------|
| In Discussion | Record in appendix, keep open |
| Resolved | Update resolution, mark status, **update relevant content in body** |
| Abandoned | Mark reason for abandonment, keep record for reference |

> **Important**: Once a discussion is resolved, the relevant content in the body must be updated! Discussion logs can be kept as historical reference.

---

### Appendix B: Design Decision Records

> Records the design decisions made in this RFC and their rationale.

| Decision | Decision | Date | Recorded By |
|----------|----------|------|-------------|
| ... | ... | ... | ... |

---

### Appendix C: Glossary

| Term | Definition |
|------|------------|
| ... | ... |

---

## References

- Links to related documentation or external resources
- Similar implementations or references from other languages

---

## Lifecycle and Disposition

RFCs have the following status transitions:

```
┌─────────────┐
│   Draft     │  ← Author creates
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Under Review│  ← Community discussion
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
│(formal spec)│    │ (preserved) │
└─────────────┘    └─────────────┘
```

### Status Description

| Status | Location | Description |
|--------|----------|-------------|
| **Draft** | `docs/design/rfc/` | Author draft, awaiting review submission |
| **Under Review** | `docs/design/rfc/` | Open for community discussion and feedback |
| **Accepted** | `docs/design/accepted/` | Becomes a formal design document, enters implementation phase |
| **Rejected** | `docs/design/rfc/` | Preserved in RFC directory, status updated |

### Post-Acceptance Actions

1. Move the RFC to `docs/design/accepted/` directory
2. Update filename to a descriptive name (e.g., `enhanced-pattern-matching.md`)
3. Update status to "Formal"
4. Update status to "Accepted", add acceptance date

### Post-Rejection Actions

1. Keep in `docs/design/rfc/` directory
2. Add rejection reason and date at the top of the file
3. Update status to "Rejected"

### Post-Discussion Actions

When consensus is reached on an open question:

1. **Update Appendix A**: Fill in the "Resolution" section for the discussion topic
2. **Update Body**: Sync the decision to the document body
3. **Record Decision**: Add to "Appendix B: Design Decision Records"
4. **Mark Question**: Check `[x]` in the "Open Questions" list

---

> **Note**: RFC numbers are only used during the discussion phase. They are removed upon acceptance, and a descriptive filename is used instead.