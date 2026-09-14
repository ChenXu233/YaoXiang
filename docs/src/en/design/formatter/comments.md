---
title: 'Comment Preservation Rules'
description: 'Rules for preserving comments during formatting'
---

# Comment Preservation

---

## §C1 Comment Types

**§C1.1 Single-line comments.** Begins with `//` and must be preserved.

```
// 这是单行注释
let x = 1;  // 这是行末注释
```

**§C1.2 Multi-line comments.** Enclosed by `/* ... */`, supports nesting, and must be preserved.

```
/* 这是多行注释 */
/* 嵌套 /* 注释 */ */
```

**§C1.3 Documentation comments.** Begins with `///` and must be preserved.

```
/// 这是文档注释
fn foo() { ... }
```

---

## §C2 Comment Positions

**§C2.1 File header comments.** Comments at the beginning of a file must be preserved at the top of
the file.

**§C2.2 Inter-statement comments.** Comments between statements must be preserved in their original
positions.

**§C2.3 End-of-line comments.** End-of-line comments must be preserved at the end of the same line.

---

## §C3 Blank Line Preservation

**§C3.1 Blank line preservation.** Blank lines in the original code should be preserved to separate
logical blocks.
