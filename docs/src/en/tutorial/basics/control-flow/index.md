---
title: Control Flow
---

# Control Flow

In the previous chapter, you learned how to define functions. Now let's learn how to give code the ability to "make decisions" and "repeat execution"—this is **control flow**.

YaoXiang provides five control flow structures, each with its own purpose:

| Control Flow | Use | One-line Description |
|--------|------|------------|
| `if-elif-else` | Conditional Judgment | Select an execution path based on a condition |
| `for` | Iterative Loop | Process each element in a collection one by one |
| `while` | Conditional Loop | Keep looping as long as the condition holds |
| `break` / `continue` | Loop Control | Exit the loop early or skip the current iteration |
| `match` | Pattern Matching | Branch based on the structure of a value |

A quick glimpse:

```yaoxiang
# if is an expression and can return a value
status = if score >= 60 { "pass" } else { "fail" }

# for iterating over a range
for i in 0..5 {
    println(i)
}

# while conditional loop
mut n = 3
while n > 0 {
    println(n)
    n = n - 1
}

# match pattern matching
description = match number {
    0 => "zero",
    1 => "one",
    _ => "other",
}
```

These control flow structures can all be used as **expressions**—they can compute a value. This is an important difference between YaoXiang and many traditional languages.

The following chapters will delve into each control flow one by one. Reading in order is recommended, as there is a natural progressive relationship between them.