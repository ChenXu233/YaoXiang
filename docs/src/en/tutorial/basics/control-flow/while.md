---
title: while Loop
---

# while Loop

`for` is suited for scenarios where you **iterate over a known collection**, while `while` fits another situation—**you don't know how many times to loop, only when to stop**.

## Basic Syntax

The definition of a `while` statement in the syntax specification:

```
while Expr Block
```

The structure is simple: `while` is followed by a condition expression, then the loop body block. As long as the condition is `true`, the loop body keeps executing.

```yaoxiang
mut count = 1

while count <= 5 {
    print(count)
    count = count + 1
}
// Output: 1 2 3 4 5
```

Note that we declare the variable with `mut count`—because `count` needs to be modified inside the loop. If we wrote `count = 1` (immutable), the `count = count + 1` in the loop body would cause an error.

## while Execution Flow

The execution steps of a `while` loop are as follows:

1. Check the condition expression
2. If the condition is `true`, execute the loop body, then go back to step 1
3. If the condition is `false`, end the loop and continue executing the code after it

The condition is checked **before each iteration begins**. If the condition is `false` from the start, the loop body won't execute even once:

```yaoxiang
mut n = 0
while n > 0 {
    print("This line will never be printed")
    n = n - 1
}
// The condition n > 0 is false from the start, so the loop body is skipped
```

## break: Exit the Loop Early

Sometimes you need to exit the loop midway—for example, when you've found what you were searching for:

```yaoxiang
numbers = [3, 7, 2, 9, 5]
mut found = false
mut index = 0

while index < 5 {
    if numbers[index] == 9 {
        found = true
        break      // Found it, no need to keep searching
    }
    index = index + 1
}

print("Found it? " + found.to_string())  // "Found it? true"
```

`break` makes the program immediately jump out of the current loop and continue executing the code after the loop.

## continue: Skip the Current Iteration

`continue` is different from `break`—it doesn't exit the loop, but instead skips the rest of the current iteration and goes directly to the next condition check:

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

In this code, when `n` equals 3, `continue` skips `println(n)` and goes back to `while n < 5` to check the condition.

## Avoiding Infinite Loops

When using `while`, you must pay special attention—make sure the loop condition eventually becomes `false`, otherwise the program will hang forever:

```yaoxiang
// Danger! Infinite loop—the condition is always true
// mut x = 1
// while x > 0 {
//     x = x + 1     // x keeps growing, will never be <= 0
// }

// Correct—there is a clear termination condition
mut x = 1
while x <= 5 {
    print(x)
    x = x + 1        // x gradually increases; eventually x > 5 and the loop ends
}
```

## Using while to Read Input

A classic use case for `while` is handling input of unknown length—you don't know how many times the user will input, only that "when the input is empty, stop":

```yaoxiang
// Pseudocode example—demonstrating a typical use of while
// read_line returns an empty string when it reads an empty line
mut line = read_line()
while line != "" {
    process(line)
    line = read_line()
}
```

This pattern of "check condition → process data → update condition" is the core usage paradigm of `while`.

## Summary

| Key Point | Description |
|------|------|
| Use Case | When the number of iterations is unknown, only the termination condition is known |
| Syntax | `while condition { ... }` |
| Execution Flow | Check condition first, then execute the loop body |
| `break` | Immediately exit the loop |
| `continue` | Skip the current iteration and go back to the condition check |
| Caution | Ensure the condition eventually becomes `false` to avoid infinite loops |

In the next chapter, you'll learn the basics of `match`—YaoXiang's most powerful branching control tool.