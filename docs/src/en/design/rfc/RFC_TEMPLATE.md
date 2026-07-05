---
title: "RFC [Number]: [Proposal Title]"
author: "[Author Name]"
created: "YYYY-MM-DD"
updated: "YYYY-MM-DD"
issue: "#123"           # Required: associated initial idea Issue
issues_impl:            # Required after acceptance: list of implementation Issues split out
  - "#456"
pr_impl:                # Required after PR merge: list of implementation PRs
  - "#789"
---

# RFC [Number]: [Proposal Title]

> **Reference**: See the [full example](EXAMPLE_full_feature_proposal.md) to learn how to write an RFC.

## Summary

Summarize the problem this RFC addresses and the proposed solution in 1-2 sentences.

## Motivation

### Why is this feature/change needed?

Explain why this RFC is necessary, what problem it solves, or what need it fulfills.

### Current Problem

Describe the issues with the current state, using concrete examples if available.

## Proposal

### Core Design

Describe the proposed solution, including core concepts and data structures.

### Examples

Provide code examples to illustrate the use of the proposal:

```yaoxiang
# Example code
```

### Syntax Changes

If the proposal involves syntax changes, list a before/after comparison:

| Before | After |
|--------|-------|
| Syntax A | Syntax B |

## Detailed Design

### Type System Impact

Describe the impact on the type system, whether new types or type constraints are introduced.

### Runtime Behavior

Describe changes in runtime behavior, if any.

### Compiler Changes

List compiler components that need to be modified.

### Backward Compatibility

Evaluate backward compatibility and how to handle existing code.

## Trade-offs

### Advantages

- List the advantages of adopting this proposal

### Disadvantages

- List the disadvantages or risks of adopting this proposal

## Alternatives

List alternative solutions that were considered and why they were not chosen.

## Implementation Strategy

### Dependencies

List other RFCs or features this RFC depends on.

### Risks

Identify potential risks in implementation.

## Open Questions

List questions that require further discussion.

Format:
```markdown
- [ ] Question description (@username: supplementary explanation)
```

---

## Appendices (Optional)

### Appendix A: Design Discussion Records

> Used to record detailed discussions during the design decision process.

#### Discussion Template

```markdown
### [Discussion Topic]

> **Discussion Status**: [Open | Resolved]
> **Initiator**: @username
> **Date**: YYYY-MM-DD

#### Problem Description

[Describe the issue to be discussed]

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

| Stage | Action |
|-------|--------|
| In Discussion | Record in appendix, keep status open |
| Resolved | Update resolution, mark status, **update relevant content in the main body** |
| Abandoned | Mark the reason for abandonment, keep records for reference |

> **Important**: After a discussion is finalized, the relevant content in the main body must be updated! Discussion records may be retained as historical reference.

---

### Appendix B: Design Decision Records

> Record the design decisions made in the RFC and their rationale.

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
- References to similar implementations in other languages

---