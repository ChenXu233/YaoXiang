---
title: "YaoXiang Code Formatting Specification"
description: The general behavior specification for the YaoXiang code formatting tool (yaoxiang fmt), defining formatting principles and scope of application
---

# YaoXiang Code Formatting Specification

This document defines the behavior specification for the `yaoxiang fmt` code formatting tool. All formatting behavior must comply with this specification.

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

**Principle 1: Formatting is idempotent.** Formatting already formatted code must produce output identical to the input.

```rust
// Rule: format(format(code)) == format(code)
assert_eq!(format_source(input, &opts), format_source(&format_source(input, &opts).unwrap(), &opts).unwrap());
```

**Principle 2: Formatting does not change semantics.** Code before and after formatting must have the same AST (Abstract Syntax Tree).

**Principle 3: Formatting preserves all comments.** Single-line comments, multi-line comments, and documentation comments must be preserved and shall not be deleted or modified.

**Principle 4: Configuration priority.** The configuration priority chain is: CLI arguments > Project-level configuration (`yaoxiang.toml`) > User-level configuration (`~/.config/yaoxiang/config.toml`) > Default values.

## Scope of Application

This specification applies to the formatting of all `.yx` source files.