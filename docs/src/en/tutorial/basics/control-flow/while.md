---
title: 'while loop'
---

# while loop

`for` is suitable for "iterating over a known collection," while `while` is for a different
case—**you don't know how many times to loop, only when to stop**.

## Basic syntax

The grammar specification defines the `while` statement as:

```
while Expr Block
```

The structure is simple: `while` is followed by a condition expression, then the loop body block. As
long as the condition is `true`, the loop body keeps executing.

```yaoxiang
mut count = 1

while count <= 5 {
    print(count)
    count = count + 1
}
// Output: 1 2 3 4 5
```

Note that we declare the variable with `mut count`—because `count` needs to be modified inside the
loop. If we wrote `count = 1` (immutable), the `count = count + 1` in the loop body would cause an
error.

## Execution flow of `while`

The execution steps of a `while` loop are as follows:

1. Check the condition expression
2. If the condition is `true`, execute the loop body, then go back to step 1
3. If the condition is `false`, end the loop and continue with the code that follows

The condition is checked **at the start of each iteration**. If the condition is `false` from the
beginning, the loop body never executes:

```yaoxiang
mut n = 0
while n > 0 {
    print("这句话永远不会被打印")
    n = n - 1
}
// The condition n > 0 is false from the start, so the loop body is skipped directly
```

## break: exiting a loop early

Sometimes you need to exit the loop early in the middle—for example, when you've found the target
you're searching for:

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

// Bool cannot be implicitly converted to String—use f-string interpolation
print(f"找到了吗？{found}")  // Output: "找到了吗？true"
```

`break` makes the program immediately jump out of the current loop and continue executing the code
after the loop.

## continue: skipping the current iteration

`continue` differs from `break`—it doesn't exit the loop, but rather skips the remainder of the
current iteration and goes directly to the next condition check:

```yaoxiang
mut n = 0
while n < 5 {
    n = n + 1
    if n == 3 {
        continue   // Skip 3, don't print
    }
    print(n)
}
// Output: 1 2 4 5
```

In this code, when `n` equals 3, `continue` skips the `print(n)` and goes directly back to
`while n < 5` to check the condition.

## Avoiding infinite loops

When using `while`, pay special attention—make sure the loop condition will eventually become
`false`, otherwise the program will hang forever:

```yaoxiang
// Danger! Infinite loop—the condition is always true
// mut x = 1
// while x > 0 {
//     x = x + 1     // x keeps growing, will never be <= 0
// }

// Correct—has a clear termination condition
mut x = 1
while x <= 5 {
    print(x)
    x = x + 1        // x gradually increases, the loop ends when x > 5
}
```

## Using `while` to read input

A classic use case for `while` is handling input of unknown length—you don't know how many times the
user will enter input, only that you should "stop when the input is empty":

<!-- docs-example: skip -->

```yaoxiang
// Pseudocode example—demonstrating a typical use of while
// read_line returns an empty string when it reads an empty line
mut line = read_line()
while line != "" {
    process(line)
    line = read_line()
}
```

This "check condition → process data → update condition" pattern is the core usage paradigm of
`while`.

## Summary

| Key Point      | Description                                                                |
| -------------- | -------------------------------------------------------------------------- |
| Use case       | Unknown number of iterations, only the termination condition               |
| Syntax         | `while condition { ... }`                                                  |
| Execution flow | Check the condition first, then execute the loop body                      |
| `break`        | Immediately exit the loop                                                  |
| `continue`     | Skip the current iteration, go back to the condition check                 |
| Note           | Make sure the condition eventually becomes `false` to avoid infinite loops |

In the next chapter, you'll learn the basics of `match`—YaoXiang's most powerful branching control
tool.
