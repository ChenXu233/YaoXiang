---
title: 'Comment Preservation Rules'
description: 'Preservation rules for comments during formatting'
---

# Comment Preservation

---

## §C1 Comment Types

**§C1.1 Single-line comments.** Use `//` to start, must be preserved.

```
// This is a single-line comment
let x = 1;  // This is an end-of-line comment
```

**§C1.2 Multi-line comments.** Enclosed by `/* ... */`, supports nesting, must be preserved.

```
/* This is a multi-line comment */
/* Nested /* comment */ */
```

**§C1.3 Documentation comments.** Use `///` to start, must be preserved.

```
/// This is a documentation comment
fn foo() { ... }
```

---

## §C2 Comment Placement

**§C2.1 File header comments.** Comments at the beginning of a file must be preserved at the start
of the file.

**§C2.2 Inter-statement comments.** Comments between statements must be preserved in their original
position.

**§C2.3 End-of-line comments.** End-of-line comments must be preserved at the end of the same line.

---

## §C3 Empty Line Preservation

**§C3.1 Empty line preservation.** Empty lines in the original code should be preserved to separate
logical blocks.
