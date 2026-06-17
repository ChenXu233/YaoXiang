---
title: while Loops
---

# while Loops

`for` is for "iterate over a known collection," while `while` is for a different scenario — **you don't know how many times to loop, only when to stop**.

## Basic Syntax

The `while` statement in the language specification:

```
while Expr Block
```

The structure is simple: `while` followed by a condition expression, then the loop body block. The body keeps executing as long as the condition is `true`.

```yaoxiang
mut count = 1

while count <= 5 {
    println(count)
    count = count + 1
}
// Output: 1 2 3 4 5
```

Notice we use `mut count` — because `count` needs to be modified inside the loop. If we had written `count = 1` (immutable), the `count = count + 1` in the body would be a compile error.

## How while Executes

A `while` loop follows these steps:

1. Check the condition expression
2. If the condition is `true`, run the body, then go back to step 1
3. If the condition is `false`, exit the loop and continue with the code that follows

The condition is checked **before each iteration**. If it is `false` at the very start, the body never runs:

```yaoxiang
mut n = 0
while n > 0 {
    println("This line is never printed")
    n = n - 1
}
// The condition n > 0 is false from the start; the body is skipped entirely
```

## break: Exit the Loop Early

Sometimes you need to leave the loop mid-way — for example, when you've found what you're searching for:

```yaoxiang
numbers = [3, 7, 2, 9, 5]
mut found = false
mut index = 0

while index < 5 {
    if numbers[index] == 9 {
        found = true
        break      # Found it, no need to keep looking
    }
    index = index + 1
}

println("Found? " + found.to_string())  # "Found? true"
```

`break` immediately exits the current loop and continues with the code after it.

## continue: Skip the Current Iteration

`continue` differs from `break` — instead of exiting the loop, it skips the rest of the current iteration and moves directly to the next condition check:

```yaoxiang
mut n = 0
while n < 5 {
    n = n + 1
    if n == 3 {
        continue   # Skip 3 — don't print it
    }
    println(n)
}
// Output: 1 2 4 5
```

When `n` equals 3, `continue` skips `println(n)` and goes straight back to checking `while n < 5`.

## Avoiding Infinite Loops

Be especially careful with `while` — make sure the condition will eventually become `false`, or your program will be stuck forever:

```yaoxiang
# Dangerous! Infinite loop — condition is always true
# mut x = 1
# while x > 0 {
#     x = x + 1     # x keeps growing, will never be <= 0
# }

# Correct — a clear termination condition
mut x = 1
while x <= 5 {
    println(x)
    x = x + 1        # x grows steadily; when x > 5 the loop ends
}
```

## Using while to Read Input

A classic `while` scenario is processing input of unknown length — you don't know how many times the user will provide input, only that "stop when the input is empty":

```yaoxiang
# Pseudocode — demonstrating the typical while pattern
# read_line returns an empty string on blank input
mut line = read_line()
while line != "" {
    process(line)
    line = read_line()
}
```

This "check condition -> process data -> update condition" pattern is the core idiom of `while` loops.

## Summary

| Point | Description |
|-------|-------------|
| Use case | Unknown iteration count, known termination condition |
| Syntax | `while condition { ... }` |
| Execution | Condition checked first, then body runs |
| `break` | Immediately exit the loop |
| `continue` | Skip current iteration, go back to condition check |
| Caution | Ensure the condition eventually becomes `false` to avoid infinite loops |

Next chapter: `match` basics — YaoXiang's most powerful branching tool.
