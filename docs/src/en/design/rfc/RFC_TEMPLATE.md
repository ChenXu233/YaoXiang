```yaml
---
title: "RFC [Number]: [Proposal Title]"
status: "Draft"
author: "[Author Name]"
created: "YYYY-MM-DD"
updated: "YYYY-MM-DD"
---

# RFC [Number]: [Proposal Title]

> **Reference**: See [Full Example](EXAMPLE_full_feature_proposal.md) for how to write an RFC.

## Summary

A 1-2 sentence summary of the problem this RFC solves and the proposed solution.

## Motivation

### Why is this feature/change needed?

Explain why this RFC is necessary, what problem it solves or what need it addresses.

### Current Problem

Describe the issues with the current state, using concrete examples if available.

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

Evaluate whether this is backward compatible and how to handle existing code.

## Trade-offs

### Pros

- List the advantages of adopting this proposal

### Cons

- List the disadvantages or risks of adopting this proposal

## Alternatives

List alternative solutions that were considered and why they were not chosen.

## Implementation Strategy

### Phases

Describe how to implement this feature in phases.

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

| Phase | Action |
|-------|--------|
| In Progress | Keep in appendix, maintain open status |
| Resolved | Update resolution, mark status, **update relevant body content** |
| Abandoned | Mark abandonment reason, keep record for reference |

> **Important**: After a discussion is settled, the relevant body content must be updated! Discussion logs may be kept as historical reference.

---

### Appendix B: Design Decision Record

> Records design decisions that have been made in this RFC along with their rationale.

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

RFCs follow this status transition:

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
│   Accepted  │    │   Rejected  │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   accepted/ │    │     rfc/    │
│ (official)  │    │(preserved)  │
└─────────────┘    └─────────────┘
```

### Status Descriptions

| Status | Location | Description |
|--------|----------|-------------|
| **Draft** | `docs/design/rfc/` | Author's draft, awaiting review submission |
| **Under Review** | `docs/design/rfc/` | Open for community discussion and feedback |
| **Accepted** | `docs/design/accepted/` | Becomes official design document, enters implementation phase |
| **Rejected** | `docs/design/rfc/` | Preserved in RFC directory, status updated |

### Post-Acceptance Actions

1. Move the RFC to `docs/design/accepted/` directory
2. Update the filename to a descriptive name (e.g., `enhanced-pattern-matching.md`)
3. Update the status to "Official"
4. Update the status to "Accepted", add acceptance date

### Post-Rejection Actions

1. Keep in `docs/design/rfc/` directory
2. Add rejection reason and date at the top of the file
3. Update the status to "Rejected"

### Post-Discussion Actions

When consensus is reached on an open question:

1. **Update Appendix A**: Fill in the "Resolution" section under the discussion topic
2. **Update Body**: Sync the decision to the document body
3. **Record Decision**: Add to "Appendix B: Design Decision Record"
4. **Mark Item**: Check off `[x]` in the "Open Questions" list

---

> **Note**: RFC numbers are only used during the discussion phase. After acceptance, remove the number and use a descriptive filename instead.
```