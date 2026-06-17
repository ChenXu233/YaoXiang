---
title: Control Flow
---

# Control Flow

In the previous chapter, you learned how to define functions. Now let's give your code the ability to "make decisions" and "repeat actions" — this is **control flow**.

YaoXiang provides five control flow structures, each with a clear purpose:

| Structure | Purpose | In one sentence |
|-----------|---------|-----------------|
| `if-elif-else` | Conditional branching | Choose a path based on a condition |
| `for` | Iteration | Process each element in a collection one by one |
| `while` | Conditional loop | Keep looping as long as a condition holds |
| `break` / `continue` | Loop control | Exit a loop early or skip the current iteration |
| `match` | Pattern matching | Branch based on the structure of a value |

A quick taste:

```yaoxiang
# if is an expression — it returns a value
status = if score >= 60 { "pass" } else { "fail" }

# for iterates over a range
for i in 0..5 {
    println(i)
}

# while loops while a condition holds
mut n = 3
while n > 0 {
    println(n)
    n = n - 1
}

# match does pattern matching
description = match number {
    0 => "zero",
    1 => "one",
    _ => "other",
}
```

All of these control flow structures can be used as **expressions** — they compute a value. This is one important way YaoXiang differs from many traditional languages.

The following chapters explain each structure in depth. Reading them in order is recommended, as they progress naturally from one to the next.
