---
title: 'while loop'
---

# while loop

`for` is suitable for "iterating over a known collection" scenarios, while `while` is suitable for
another situation—**you don't know how many times to loop, only when to stop**.

## Basic Syntax

The definition of a `while` statement in the syntax specification:

```
while Expr Block
```

The structure is simple: `while` is followed by a condition expression, then the loop body code
block. As long as the condition is `true`, the loop body keeps executing.

```yaoxiang
mut count = 1

while count <= 5 {
    print(count)
    count = count + 1
}
// 输出：1 2 3 4 5
```

Note that we declare the variable with `mut count`—because `count` needs to be modified inside the
loop. If written as `count = 1` (immutable), the `count = count + 1` in the loop body would cause an
error.

## while Execution Flow

The execution steps of a `while` loop are as follows:

1. Check the condition expression
2. If the condition is `true`, execute the loop body, then go back to step 1
3. If the condition is `false`, end the loop and continue executing the code that follows

The condition is checked **before each iteration begins**. If the condition is `false` from the
start, the loop body will not execute even once:

```yaoxiang
mut n = 0
while n > 0 {
    print("这句话永远不会被打印")
    n = n - 1
}
// 条件 n > 0 一开始就是 false，循环体直接跳过
```

## break: Exit the Loop Early

Sometimes you need to exit the loop early in the middle—for example, when you've found the target
you're searching for:

```yaoxiang
numbers = [3, 7, 2, 9, 5]
mut found = false
mut index = 0

while index < 5 {
    if numbers[index] == 9 {
        found = true
        break      // 找到了，不需要继续找
    }
    index = index + 1
}

print("找到了吗？" + found.to_string())  // "找到了吗？true"
```

`break` makes the program immediately jump out of the current loop and continue executing the code
after the loop.

## continue: Skip the Current Iteration

`continue` differs from `break`—it does not exit the loop, but skips the rest of the current
iteration and goes directly to the next condition check:

```yaoxiang
mut n = 0
while n < 5 {
    n = n + 1
    if n == 3 {
        continue   // 跳过 3，不打印
    }
    print(n)
}
// 输出：1 2 4 5
```

In this code, when `n` equals 3, `continue` skips the `println(n)` and goes directly back to the
`while n < 5` condition check.

## Avoiding Infinite Loops

When using `while`, you need to pay special attention—make sure the loop condition eventually
becomes `false`, otherwise the program will hang forever:

```yaoxiang
// 危险！死循环——条件永远为 true
// mut x = 1
// while x > 0 {
//     x = x + 1     // x 越来越大，永远不会 <= 0
// }

// 正确——有明确的终止条件
mut x = 1
while x <= 5 {
    print(x)
    x = x + 1        // x 逐渐增大，最终 x > 5 时循环结束
}
```

## Using while to Read Input

A classic use case of `while` is handling input of unknown length—you don't know how many times the
user will input, only "stop when the input is empty":

```yaoxiang
// 伪代码示例——展示 while 的典型用法
// read_line 在读到空行时返回空字符串
mut line = read_line()
while line != "" {
    process(line)
    line = read_line()
}
```

This pattern of "check condition → process data → update condition" is the core usage paradigm of
`while`.

## Summary

| Key Point      | Description                                                             |
| -------------- | ----------------------------------------------------------------------- |
| Use case       | Unknown number of iterations, only the termination condition is known   |
| Syntax         | `while condition { ... }`                                               |
| Execution flow | Check condition first, then execute the loop body                       |
| `break`        | Immediately exit the loop                                               |
| `continue`     | Skip the current iteration, return to condition check                   |
| Notes          | Ensure the condition eventually becomes `false` to avoid infinite loops |

In the next chapter, you will learn the basics of `match`—YaoXiang's most powerful branching control
tool.
