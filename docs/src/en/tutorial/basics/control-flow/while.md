---
title: while Loop
---

# while Loop

`for` is suitable for "iterating over a known collection", while `while`
is appropriate for another scenario—**you don't know how many times to loop, only when to stop**.

## Basic Syntax

The definition of a `while` statement in the grammar specification:

```
while Expr Block
```

The structure is simple: `while` is followed by a condition expression, then a loop body code block. As long as the condition is `true`, the loop body keeps executing.

```yaoxiang
mut count = 1

while count <= 5 {
    print(count)
    count = count + 1
}
// Output: 1 2 3 4 5
```

Note that we declare the variable with `mut count`—because `count` needs to be modified within the loop. If you write
`count = 1` (immutable), then `count = count + 1` in the loop body will cause an error.

## Execution Flow of while

The `while` loop executes in the following steps:

1. Check the condition expression
2. If the condition is `true`, execute the loop body, then go back to step 1
3. If the condition is `false`, end the loop and continue executing the code after it

The condition is checked **at the start of each iteration**. If the condition is `false` from the beginning, the loop body won't execute at all:

```yaoxiang
mut n = 0
while n > 0 {
    print("This sentence will never be printed")
    n = n - 1
}
// Condition n > 0 is false from the start, so the loop body is skipped
```

## break: Exit Loop Early

Sometimes you need to exit the loop early in the middle—for example, when you've found the target you're searching for:

```yaoxiang
numbers = [3, 7, 2, 9, 5]
mut found = false
mut index = 0

while index < 5 {
    if numbers[index] == 9 {
        found = true
        break      // Found it, no need to continue searching
    }
    index = index + 1
}

print("Found? " + found.to_string())  // "Found? true"
```

`break` makes the program immediately exit the current loop and continue executing the code after the loop.

## continue: Skip Current Iteration

`continue` differs from `break`—it doesn't exit the loop, but skips the rest of the current iteration and goes directly to the next condition check:

```yaoxiang
mut n = 0
while n < 5 {
    n = n + 1
    if n == 3 {
        continue   // Skip 3, don't print it
    }
    print(n)
}
// Output: 1 2 4 5
```

In this code, when `n` equals 3, `continue` skips `println(n)` and goes directly back to checking the condition `while n < 5`.

## Avoiding Infinite Loops

When using `while`, be especially careful—make sure the loop condition will eventually become `false`, otherwise the program will get stuck forever:

```yaoxiang
// Dangerous! Infinite loop—the condition is always true
// mut x = 1
// while x > 0 {
//     x = x + 1     // x keeps growing, never becomes <= 0
// }

// Correct—there is a clear termination condition
mut x = 1
while x <= 5 {
    print(x)
    x = x + 1        // x gradually increases, loop ends when x > 5
}
```

## Using while to Read Input

A classic use case for `while` is handling input of unknown length—you don't know how many times the user will input, only that "stop when input is empty":

```yaoxiang
// Pseudocode example—demonstrating typical while usage
// read_line returns an empty string when it encounters an empty line
mut line = read_line()
while line != "" {
    process(line)
    line = read_line()
}
```

This "check condition → process data → update condition" pattern is the core usage paradigm of `while`.

## Summary

| Key Point    | Description                                          |
| ------------ | ---------------------------------------------------- |
| Use Case     | Don't know iteration count, only termination condition |
| Syntax       | `while condition { ... }`                            |
| Flow         | Check condition first, then execute loop body         |
| `break`      | Exit loop immediately                                |
| `continue`   | Skip current iteration, return to condition check     |
| Caution      | Make sure condition eventually becomes `false`, avoid infinite loops |

In the next chapter you'll learn the basics of `match`—YaoXiang's most powerful branching control tool.
