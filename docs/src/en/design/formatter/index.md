---
title: 'YaoXiang Code Formatting Specification'
description:
  'General specification for the behavior of the YaoXiang code formatting tool (yx format), defining
  formatting principles and scope of application'
---

# YaoXiang Code Formatting Specification

This document defines the behavior specification for the `yx format` code formatting tool. All
formatting behavior must comply with this specification.

---

## Table of Contents

- [Principles](#principles)
- [Scope of Application](#scope-of-application)
- [Formatting Rules](./formatting-rules/index.md)
- [Configuration Options](./configuration.md)
- [Comment Preservation](./comments.md)
- [Error Handling](./error-handling.md)
- [Command Line Usage](./cli.md)

---

## Principles

**Principle 1: Formatting is idempotent.** Running formatting on already-formatted code must produce
output identical to the input.

```rust
// Rule: format(format(code)) == format(code)
assert_eq!(format_source(input, &opts), format_source(&format_source(input, &opts).unwrap(), &opts).unwrap());
```

**Principle 2: Formatting does not change semantics.** The code before and after formatting must
have the same AST (Abstract Syntax Tree).

**Principle 3: Formatting preserves all comments.** Single-line comments, multi-line comments, and
documentation comments must be preserved and must not be deleted or modified.

**Principle 4: Configuration priority.** The configuration priority chain is: CLI arguments >
project-level configuration (`yaoxiang.toml`) > user-level configuration
(`~/.config/yaoxiang/config.toml`) > default values.

## Scope of Application

This specification applies to the formatting of all `.yx` source files.
