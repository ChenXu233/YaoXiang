---
title: while Loop
---

# while Loop

`for` is suited for "iterating over a known collection" scenarios, while `while` is suited for another situation—**when you don't know how many times to loop, only when to stop**.

## Basic Syntax

The definition of a `while` statement in the syntax specification:

```
while Expr Block
```

The structure is simple: `while` is followed by a conditional expression, then a loop body code block. As long as the condition is `true`, the loop body keeps executing.

```yaoxiang
mut count = 1

while count <= 5 {
    println(count)
    count = count + 1
}
// Output: 1 2 3 4 5
```

Note that we declare the variable with `mut count`—because `count` needs to be modified inside the loop. If we wrote `count = 1` (immutable), the `count = count + 1` inside the loop body would cause an error.

## while Execution Flow

The execution steps of a `while` loop are as follows:

1. Evaluate the conditional expression
2. If the condition is `true`, execute the loop body, then go back to step 1
3. If the condition is `false`, end the loop and continue executing the code that follows

The condition is checked **before each iteration begins**. If the condition is `false` at the start, the loop body is never executed:

```yaoxiang
mut n = 0
while n > 0 {
    println("This line will never be printed")
    n = n - 1
}
// The condition n > 0 is false from the start, so the loop body is skipped entirely
```

## break: Exiting the Loop Early

Sometimes you need to exit the loop in the middle—for example, when you've found what you were searching for:

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

`break` makes the program immediately jump out of the current loop and continue executing the code after the loop.

## continue: Skipping the Current Iteration

`continue` differs from `break`—it does not exit the loop, but skips the rest of the current iteration and goes directly to the next condition check:

```yaoxiang
mut n = 0
while n < 5 {
    n = n + 1
    if n == 3 {
        continue   # Skip 3, don't print it
    }
    println(n)
}
// Output: 1 2 4 5
```

In this code, when `n` equals 3, `continue` skips the `println(n)` and goes directly back to check `while n < 5`.

## Avoiding Infinite Loops

When using `while`, pay special attention—make sure the loop condition eventually becomes `false`, otherwise the program will get stuck forever:

```yaoxiang
# Dangerous! Infinite loop—the condition is always true
# mut x = 1
# while x > 0 {
#     x = x + 1     # x keeps growing, never <= 0
# }

# Correct—has a clear termination condition
mut x = 1
while x <= 5 {
    println(x)
    x = x + 1        # x grows gradually, loop ends when x > 5
}
```

## Using while to Read Input

A classic use case for `while` is handling input of unknown length—you don't know how many times the user will input, only that "input stops when empty":

```yaoxiang
# Pseudocode example—shows typical usage of while
# read_line returns an empty string when it reads an empty line
mut line = read_line()
while line != "" {
    process(line)
    line = read_line()
}
```

This pattern of "check condition → process data → update condition" is the core usage pattern of `while`.

## Summary

| Key Point | Description |
|------|------|
| Applicable Scenario | Unknown number of iterations, only a termination condition is known |
| Syntax | `while condition { ... }` |
| Execution Flow | Check the condition first, then execute the loop body |
| `break` | Immediately exit the loop |
| `continue` | Skip the current iteration and return to the condition check |
| Note | Ensure the condition eventually becomes `false` to avoid infinite loops |

In the next chapter you'll learn the basics of `match`—YaoXiang's most powerful branch control tool.