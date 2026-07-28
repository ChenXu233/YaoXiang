---
title: Control Flow
---

# Control Flow

In the previous chapter you learned how to define functions. Now let's learn how to give code the ability to "make decisions" and "repeat execution" — this is **control flow**.

YaoXiang provides five control flow structures, each with its own purpose:

| Control Flow         | Purpose      | One-liner                     |
| -------------------- | ------------ | ----------------------------- |
| `if-else-if-else`    | Conditional  | Choose a path based on a condition |
| `for`                | Iteration    | Process each element in a collection one by one |
| `while`              | Conditional  | Loop as long as the condition holds |
| `break` / `continue` | Loop Control | Exit a loop early or skip the current iteration |
| `match`              | Pattern Matching | Branch based on the structure of a value |

A quick taste:

```yaoxiang
// if is an expression and can return a value
status = if score >= 60 { "pass" } else { "fail" }

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

// match for pattern matching
description = match number {
    0 => "zero",
    1 => "one",
    _ => "other",
}
```

All of these control flow structures can be used as **expressions** — they can compute a value. This is an important distinction between YaoXiang and many traditional languages.

The following chapters will dive deep into each control flow structure. It's recommended to read them in order, as there is a natural progression between them.
