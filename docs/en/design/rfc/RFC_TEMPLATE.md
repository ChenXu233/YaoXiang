---
title: RFC Template
---

# RFC [ID]: [Proposal Title]

> **Status**: [Draft | Under Review | Accepted | Rejected]
> **Author**: [Author Name]
> **Created Date**: [YYYY-MM-DD]
> **Last Updated**: [YYYY-MM-DD]

> **Reference**: See [Complete Example](EXAMPLE_full_feature_proposal.md) to learn how to write an RFC.

## Summary

Summarize in 1-2 sentences the problem this RFC solves and the proposed solution.

## Motivation

### Why is this feature/change needed?

Explain why this RFC is necessary, what problems it solves or needs it fulfills.

### Current Problems

Describe the problems with the current state, with specific examples if available.

## Proposal

### Core Design

Describe the proposed solution, including core concepts and data structures.

### Examples

Provide code examples to illustrate how to use the proposal:

```yaoxiang
# Example code
```

### Syntax Changes

If the proposal involves syntax changes, list the before/after comparison:

| Before | After |
|--------|-------|
| Syntax A | Syntax B |

## Detailed Design

### Type System Impact

Describe the impact on the type system, whether new types or type constraints are introduced.

### Runtime Behavior

Describe changes in runtime behavior (if any).

### Compiler Changes

List compiler components that need to be modified.

### Backward Compatibility

Evaluate whether it is backward compatible and how to handle existing code.

## Trade-offs

### Advantages

- List the advantages of adopting this proposal

### Disadvantages

- List the disadvantages or risks of adopting this proposal

## Alternative Solutions

List the alternative solutions considered and why they were not chosen.

## Implementation Strategy

### Phase Division

Describe how to implement this feature in phases.

### Dependencies

List other RFCs or features this RFC depends on.

### Risks

Identify potential risks in implementation.

## Open Questions

List questions that need further discussion.

Format:
```markdown
- [ ] Question description (@username: additional notes)
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

#### Alternative Solutions

| Solution | Description | Advantages | Disadvantages |
|----------|-------------|------------|---------------|
| A | ... | ... | ... |
| B | ... | ... | ... |

#### Discussion Record

- [Date] @username: [Comment]
- [Date] @username: [Comment]

#### Resolution

[Final decision and reasoning]
```

#### Discussion Management Rules

| Stage | Action |
|-------|--------|
| In Discussion | Record in appendix, keep open |
| Resolved | Update resolution, mark status, **update main text content** |
| Abandoned | Mark reason for abandonment, keep record for reference |

> **Important**: After discussion is finalized, must update main text content! Discussion records can be kept as historical reference.

---

### Appendix B: Design Decision Record

> Record finalized design decisions in the RFC and their reasoning.

| Decision | Decision | Date | Recorder |
|----------|----------|------|----------|
| ... | ... | ... | ... |

---

### Appendix C: Glossary

| Term | Definition |
|------|------------|
| ... | ... |

---

## References

- Related document or external resource links
- References to similar implementations or other languages

---

## Lifecycle and Destination

RFC has the following status transitions:

```
┌─────────────┐
│   Draft     │  ← Author creates
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Under Review │  ← Community discussion
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  Accepted   │    │  Rejected  │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   accepted/ │    │    rfc/     │
│ (Final Design)|   │ (Keep Original)│
└─────────────┘    └─────────────┘
```

### Status Description

| Status | Location | Description |
|--------|-----------|-------------|
| **Draft** | `docs/design/rfc/` | Author's draft, awaiting submission for review |
| **Under Review** | `docs/design/rfc/` | Open for community discussion and feedback |
| **Accepted** | `docs/design/accepted/` | Becomes formal design document, enters implementation phase |
| **Rejected** | `docs/design/rfc/` | Keep in RFC directory, update status |

### Actions After Acceptance

1. Move RFC to `docs/design/accepted/` directory
2. Update filename to descriptive name (e.g., `enhanced-pattern-matching.md`)
3. Update status to "正式" (Formal)
4. Update status to "Accepted", add acceptance date

### Actions After Rejection

1. Keep in `docs/design/rfc/` directory
2. Add rejection reason and date at the top of the file
3. Update status to "Rejected"

### Actions After Discussion Resolution

When an open question reaches consensus:

1. **Update Appendix A**: Fill in "Resolution" under the discussion topic
2. **Update Main Text**: Sync the decision to the document body
3. **Record Decision**: Add to "Appendix B: Design Decision Record"
4. **Mark Question**: Check `[x]` in "Open Questions" list

---

> **Note**: RFC IDs are only used during the discussion phase. After acceptance, remove the ID and use descriptive filenames.
