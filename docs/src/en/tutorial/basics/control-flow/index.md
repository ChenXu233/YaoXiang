---
title: Control Flow
---

# Control Flow

In the previous chapter you learned how to define functions. Now let's learn how to give code the ability to "make decisions" and "execute repeatedly"—this is **control flow**.

YaoXiang provides five control flow structures, each with its own purpose:

| Control Flow | Purpose | One-line Description |
|--------|------|------------|
| `if-elif-else` | Conditional judgment | Choose an execution path based on a condition |
| `for` | Traversal loop | Process each element in a collection one by one |
| `while` | Conditional loop | Keep looping as long as the condition holds |
| `break` / `continue` | Loop control | Exit the loop early or skip the current iteration |
| `match` | Pattern matching | Branch based on the structure of a value |

A quick taste:

```yaoxiang
// if is an expression that can return a value
status = if score >= 60 { "pass" } else { "fail" }

// for traverses a range
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
    0 => "zero",
    1 => "one",
    _ => "other",
}
```

All of these control flow structures can be used as **expressions**—they can compute a value. This is an important difference between YaoXiang and many traditional languages.

The following sections will dive into each kind of control flow one by one. It is recommended to read them in order, since there is a natural progressive relationship among them.