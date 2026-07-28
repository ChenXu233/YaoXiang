---
title: 'RFC [Number]: [Proposal Title]'
author: '[Author Name]'
created: 'YYYY-MM-DD'
updated: 'YYYY-MM-DD'
issue: '#123' # Required: Linked initial idea Issue
issues_impl: # Required after acceptance: Implementation Issue list
  - '#456'
pr_impl: # Required after PR merge: Implementation PR list
  - '#789'
---

# RFC [Number]: [Proposal Title]

> **Reference**: See [Full Example](EXAMPLE_full_feature_proposal.md) for how to write an RFC.

## Summary

Summarize the problem this RFC aims to solve and the proposed solution in 1-2 sentences.

## Motivation

### Why is this feature/change needed?

Explain why this RFC is necessary, what problem it solves or what need it fulfills.

### Current Problem

Describe the issues with the current state, with specific examples if available.

## Proposal

### Core Design

Describe the proposed solution, including core concepts and data structures.

### Examples

Provide code examples to illustrate how the proposal works:

```yaoxiang
# Example code
```

### Syntax Changes

If the proposal involves syntax changes, list before/after comparisons:

| Before   | After   |
| -------- | ------- |
| Syntax A | Syntax B |

## Detailed Design

### Type System Impact

Describe the impact on the type system, whether new types or type constraints are introduced.

### Runtime Behavior

Describe changes in runtime behavior (if any).

### Compiler Changes

List compiler components that need to be modified.

### Backward Compatibility

Evaluate whether this is backward compatible and how to handle existing code.

## Trade-offs

### Advantages

- List the benefits of adopting this proposal

### Disadvantages

- List the drawbacks or risks of this proposal

## Alternative Solutions

List considered alternatives and why they were not chosen.

## Implementation Strategy

### Dependencies

List other RFCs or features this RFC depends on.

### Risks

Identify potential risks in the implementation.

## Open Questions

List issues that need further discussion.

Format:

```markdown
- [ ] Issue description (@username: additional notes)
```

---

## Appendix (Optional)

### Appendix A: Design Discussion Log

> Used to record detailed discussions during the design decision process.

#### Discussion Template

```markdown
### [Discussion Topic]

> **Status**: [Open | Resolved] **Initiator**: @username **Date**: YYYY-MM-DD

#### Problem Description

[Describe the issue to be discussed]

#### Options

| Option | Description | Pros | Cons |
| ------ | ----------- | ---- | ---- |
| A      | ...         | ...  | ...  |
| B      | ...         | ...  | ...  |

#### Discussion Log

- [Date] @username: [comment]
- [Date] @username: [comment]

#### Resolution

[Final decision and rationale]
```

#### Discussion Management Rules

| Stage     | Action                                              |
| --------- | --------------------------------------------------- |
| In Progress | Record in appendix, keep open                     |
| Resolved  | Update resolution, mark status, **update main content** |
| Abandoned | Mark reason, keep record for reference             |

> **Important**: After discussion is concluded, the main content must be updated! Discussion records can be kept as historical reference.

---

### Appendix B: Design Decision Record

> Records established design decisions and their rationale.

| Decision | Resolution | Date | Recorder |
| -------- | ---------- | ---- | -------- |
| ...      | ...        | ...  | ...      |

---

### Appendix C: Glossary

| Term   | Definition |
| ------ | ---------- |
| ...    | ...        |

---

## References

- Links to related documentation or external resources
- Similar implementations or references from other languages
