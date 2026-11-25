---
title: 'Control Flow'
---

# Control Flow

In the previous chapter, you learned how to define functions. Now, let's learn how to give code the
ability to "make decisions" and "execute repeatedly"—this is **control flow**.

YaoXiang provides five control-flow constructs, each serving a different purpose:

| Control flow         | Purpose               | One-sentence description                           |
| -------------------- | --------------------- | -------------------------------------------------- |
| `if-else-if-else`    | Conditional branching | Select an execution path based on a condition      |
| `for`                | Iteration loop        | Process each element in a collection one at a time |
| `while`              | Conditional loop      | Keep looping as long as the condition holds        |
| `break` / `continue` | Loop control          | Exit the loop early or skip the current iteration  |
| `match`              | Pattern matching      | Branch based on the structure of a value           |

Get a quick feel for them:

```yaoxiang
score = 72
number = 3

// if is an expression and can return a value
status = if score >= 60 { "及格" } else { "不及格" }
print(status)

// for iterates over a range
for i in 0..5 {
    print(i)
}

// while is a conditional loop
mut n = 3
while n > 0 {
    print(n)
    n = n - 1
}

// match performs pattern matching
description = match number {
    0 -> "零",
    1 -> "一",
    _ -> "其他",
}
print(description)
```

All of these control-flow constructs can be used as **expressions**—they can compute a value. This
is an important distinction between YaoXiang and many traditional languages.

The following chapters will take a closer look at each control-flow construct. We recommend reading
them in order, because there is a natural progression from one to the next.

## Chapters

- [Conditional branching if-elif-else](if-elif-else.md) — Condition checks and branch selection
- [for loop](for.md) — Iterating over ranges and collections
- [while loop](while.md) — Conditional loops and accumulation
- [match pattern matching](match.md) — Branching according to the structure of a value
