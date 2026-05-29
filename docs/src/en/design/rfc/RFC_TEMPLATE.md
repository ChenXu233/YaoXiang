---
title: RFC Template
---

# RFC [Number]: [Proposal Title]

> **Status**: [Draft | Under Review | Accepted | Rejected]
> **Author**: [Author Name]
> **Created**: [YYYY-MM-DD]
> **Last Updated**: 2026-02-12

> **Reference**: See the [full example](EXAMPLE_full_feature_proposal.md) for how to write an RFC.

## Summary

Summarize in 1-2 sentences the problem this RFC aims to solve and the proposed solution.

## Motivation

### Why is this feature/change needed?

Explain why this RFC is necessary, what problem it solves or what need it addresses.

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

Assess whether this is backward compatible and how existing code should be handled.

## Tradeoffs

### Advantages

- List the advantages of adopting this proposal

### Disadvantages

- List the disadvantages or risks of this proposal

## Alternative Approaches

List the alternative approaches that were considered and why they were not chosen.

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

## Appendix (Optional)

### Appendix A: Design Discussion Record

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

| Option | Description | Advantages | Disadvantages |
|--------|-------------|------------|---------------|
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
| In Discussion | Keep in appendix, maintain open status |
| Resolved | Update resolution, mark status, **update main body content** |
| Abandoned | Mark reason for abandonment, keep record for reference |

> **Important**: After a discussion is concluded, the main body content must be updated! Discussion records may be retained as historical reference.

---

### Appendix B: Design Decision Record

> Records the design decisions made in the RFC and their rationale.

| Decision | Decision Made | Date | Recorder |
|----------|---------------|------|----------|
| ... | ... | ... | ... |

---

### Appendix C: Glossary

| Term | Definition |
|------|------------|
| ... | ... |

---

## References

- Links to related documentation or external resources
- References to similar implementations or other languages

---

## Lifecycle and Disposition

RFCs have the following status transitions:

```
┌─────────────┐
│   Draft     │  ← Created by author
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
│   accepted/ │    │     rfc/    │
│ (official   │    │ (kept in    │
│  design)    │    │  place)     │
└─────────────┘    └─────────────┘
```

### Status Descriptions

| Status | Location | Description |
|--------|----------|-------------|
| **Draft** | `docs/design/rfc/` | Author draft, awaiting submission for review |
| **Under Review** | `docs/design/rfc/` | Open for community discussion and feedback |
| **Accepted** | `docs/design/accepted/` | Becomes an official design document, enters implementation phase |
| **Rejected** | `docs/design/rfc/` | Kept in RFC directory, status updated |

### Post-Acceptance Actions

1. Move the RFC to the `docs/design/accepted/` directory
2. Update the filename to a descriptive name (e.g., `enhanced-pattern-matching.md`)
3. Update the status to "Official"
4. Update the status to "Accepted" and add the acceptance date

### Post-Rejection Actions

1. Keep in the `docs/design/rfc/` directory
2. Add rejection reason and date at the top of the file
3. Update the status to "Rejected"

### Post-Discussion Actions

When consensus is reached on an open question:

1. **Update Appendix A**: Fill in the "Resolution" section for the discussion topic
2. **Update Main Body**: Sync the decision to the document's main body
3. **Record Decision**: Add to "Appendix B: Design Decision Record"
4. **Mark Question**: Check the item `[x]` in the "Open Questions" list

---

> **Note**: RFC numbers are only used during the discussion phase. After acceptance, the number is removed and a descriptive filename is used instead.