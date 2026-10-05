---
title: 'Control Flow'
---

# Control Flow

In the previous chapter you learned how to define functions. Now let's learn how to give code the
ability to "make decisions" and "repeat execution"—this is **control flow**.

YaoXiang provides five control flow structures, each with its own purpose:

| Control Flow         | Purpose           | One-line Description                              |
| -------------------- | ----------------- | ------------------------------------------------- |
| `if-else-if-else`    | Conditional check | Choose execution path based on the condition      |
| `for`                | Iteration loop    | Process each element in a collection one by one   |
| `while`              | Conditional loop  | Keep looping as long as the condition holds       |
| `break` / `continue` | Loop control      | Break out of the loop early or skip the iteration |
| `match`              | Pattern matching  | Branch based on the structure of a value          |

A quick taste:

```yaoxiang
score = 72
number = 3

// if is an expression, it can return a value
status = if score >= 60 { "及格" } else { "不及格" }
print(status)

// for iterates over a range
for i in 0..5 {
    print(i)
}

// while conditional loop
mut n = 3
while n > 0 {
    print(n)
    n = n - 1
}

// match pattern matching
description = match number {
    0 => "零",
    1 => "一",
    _ => "其他",
}
print(description)
```

All of these control flow structures can be used as **expressions**—they compute a value. This is an
important difference between YaoXiang and many traditional languages.

The following chapters will dive into each kind of control flow in depth. We recommend reading them
in order, since they follow a natural progression.

## Chapters

- [Conditional Branching if-elif-else](if-elif-else.md) — Conditional checks and branch selection
- [for Loop](for.md) — Iterating over ranges and collections
- [while Loop](while.md) — Conditional looping and accumulation
- [match Pattern Matching](match.md) — Branching by the structure of a value
