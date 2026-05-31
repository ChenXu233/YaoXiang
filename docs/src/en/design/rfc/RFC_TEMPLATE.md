---
title: "RFC Template"
---

# RFC [Number]: [Proposal Title]

> **Status**: [Draft | Under Review | Accepted | Rejected]
> **Author**: [Author Name]
> **Created**: [YYYY-MM-DD]
> **Last Updated**: 2026-02-12

> **Reference**: See [Full Example](EXAMPLE_full_feature_proposal.md) for how to write an RFC.

## Summary

A 1-2 sentence summary of the problem this RFC solves and the proposed solution.

## Motivation

### Why is this feature/change needed?

Explain why this RFC is necessary, what problem it solves or what need it fulfills.

### Current Problem

Describe issues with the current state, with concrete examples if available.

## Proposal

### Core Design

Describe the proposed solution, including core concepts and data structures.

### Examples

Provide code examples illustrating how the proposal would be used:

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

Describe impacts on the type system, whether new types or type constraints are introduced.

### Runtime Behavior

Describe changes to runtime behavior (if any).

### Compiler Changes

List compiler components that need modification.

### Backward Compatibility

Assess whether this is backward compatible and how existing code should be handled.

## Trade-offs

### Pros

- List advantages of adopting this proposal

### Cons

- List disadvantages or risks of this proposal

## Alternatives

List considered alternatives and why they were not chosen.

## Implementation Strategy

### Phased Approach

Describe how this feature will be implemented in phases.

### Dependencies

List other RFCs or features this RFC depends on.

### Risks

Identify potential implementation risks.

## Open Questions

List issues that need further discussion.

Format:
```markdown
- [ ] Problem description (@username: additional notes)
```

---

## Appendices (Optional)

### Appendix A: Design Discussion Log

> Used to record detailed discussions during the design decision process.

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

| Stage | Action |
|-------|--------|
| In Discussion | Record in appendix, keep open |
| Resolved | Update resolution, mark status, **update main body content** |
| Abandoned | Mark abandonment reason, keep record for reference |

> **Important**: After a discussion is concluded, the main body content must be updated! Discussion logs can be kept as historical reference.

---

### Appendix B: Design Decision Record

> Record finalized design decisions and their rationale.

| Decision | Resolution | Date | Recorder |
|----------|------------|------|----------|
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

RFCs follow this status flow:

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
│ (Official   │    │ (Kept in    │
│  Design)    │    │  place)     │
└─────────────┘    └─────────────┘
```

### Status Descriptions

| Status | Location | Description |
|--------|----------|-------------|
| **Draft** | `docs/design/rfc/` | Author's draft, awaiting review submission |
| **Under Review** | `docs/design/rfc/` | Open for community discussion and feedback |
| **Accepted** | `docs/design/accepted/` | Becomes official design document, enters implementation phase |
| **Rejected** | `docs/design/rfc/` | Remains in RFC directory, status updated |

### Post-Acceptance Actions

1. Move the RFC to the `docs/design/accepted/` directory
2. Update filename to a descriptive name (e.g., `enhanced-pattern-matching.md`)
3. Update status to "Official"
4. Update status to "Accepted", add acceptance date

### Post-Rejection Actions

1. Keep in the `docs/design/rfc/` directory
2. Add rejection reason and date at the top of the file
3. Update status to "Rejected"

### Post-Discussion Actions

When consensus is reached on an open question:

1. **Update Appendix A**: Fill in the "Resolution" for the discussion topic
2. **Update Main Body**: Sync the decision to the document body
3. **Record Decision**: Add to "Appendix B: Design Decision Record"
4. **Mark Issue**: Check off `[x]` in the "Open Questions" list

---

> **Note**: RFC numbers are only used during the discussion phase. Upon acceptance, the number is removed and a descriptive filename is used instead.