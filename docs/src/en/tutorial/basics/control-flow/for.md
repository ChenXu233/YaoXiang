---
title: 'for loop'
---

# for loop

When you need to do the same thing to each element in a series, the `for` loop is the right tool.
YaoXiang's `for` loop is designed to be concise and precise.

## Basic syntax

The formal definition of the `for` statement in the language specification:

```
for 'mut'? Identifier 'in' Expr Block
```

In plain English: it starts with `for`, optionally followed by `mut`, then the loop variable name,
followed by `in` and the expression being iterated over, and finally the loop body block.

## Iterating over number ranges

The most common usage is to create a range with `..` and then iterate over it with `for`:

```yaoxiang
// From 0 to 4 (not including 5)
for i in 0..5 {
    print(i)
}
// Output: 0 1 2 3 4
```

`0..5` represents a range starting at 0 (inclusive) and ending at 5 (exclusive). This is the
universal convention in computer science—half-open interval.

You can also use variables to define the start and end of the range:

```yaoxiang
start = 10
end = 15
for n in start..end {
    print(n)
}
// Output: 10 11 12 13 14
```

## Iterating over lists

`for` can not only iterate over number ranges, but also directly over lists, arrays, and other
collections:

```yaoxiang
colors = ["红", "橙", "黄", "绿", "蓝"]

for color in colors {
    print("当前颜色: " + color)
}
// Output:
// 当前颜色: 红
// 当前颜色: 橙
// ... and so on
```

## for's unique semantics: a new value is bound in each iteration

YaoXiang's `for` loop has a design different from other languages: **each iteration creates a new
binding rather than modifying the same variable**.

To understand this with a table:

| Iteration | What happens                                                                          |
| --------- | ------------------------------------------------------------------------------------- |
| 1st       | Create new binding `i = 0`, execute the loop body, then destroy the binding           |
| 2nd       | Create new binding `i = 1` (a brand new binding), execute the loop body, then destroy |
| 3rd       | Create new binding `i = 2`, execute the loop body, then destroy                       |
| ...       | ...                                                                                   |
| Loop ends | Range is exhausted, loop terminates                                                   |

This means the loop variable in each iteration is an independent new value. This is very helpful for
safety—you don't have to worry about the loop variable being accidentally modified:

```yaoxiang
for i in 1..5 {
    // i = i + 1   // Error: immutable by default, cannot modify i
    print(i)
}
```

## for mut: explicit declaration when modification is needed

If you really need to modify the loop variable inside the loop body (for example, as an
accumulator), use `for mut`:

```yaoxiang
// Use for mut to allow modifying the binding inside the loop body
for mut i in 0..5 {
    i = i * 2
    print(i)
}
// Output: 0 2 4 6 8
```

Note: even with `for mut`, each iteration still creates a new binding. `for mut` only makes the new
binding itself mutable—it does not let modifications from one iteration carry over to the next.

```yaoxiang
for mut i in 1..5 {
    i = i + 100
    print(i)        // Each time prints 101, 102, 103, 104
}
// Each iteration's i restarts from the range value; the previous iteration's modification does not affect the next
```

## The loop variable cannot shadow an outer variable

YaoXiang prohibits variable shadowing. The `for` loop variable cannot have the same name as a
variable in the outer scope:

```yaoxiang
// Wrong example
i = 10
// for i in 1..5 {     // Compile error! i is already declared in the outer scope
//     print(i)
// }

// Correct写法——use a different name
i = 10
for j in 1..5 {
    print(j)
}
```

This rule ensures you'll never be confused about "which variable is this code referring to right
now".

## Comparison with other languages

| Language | for loop variable semantics                 |
| -------- | ------------------------------------------- |
| YaoXiang | A new value is bound in each iteration      |
| Rust     | Modifies the same variable (requires `mut`) |
| Python   | Modifies the same variable                  |
| C/C++    | Modifies the same variable                  |

YaoXiang's design is closer to human intuition—"for each element in the collection, do
something"—each element is an independent individual.

## Summary

| Key point                   | Description                                                                   |
| --------------------------- | ----------------------------------------------------------------------------- |
| Iterating over a range      | `for i in 0..5`, half-open                                                    |
| Iterating over a collection | `for item in list`, takes each element one by one                             |
| Binding semantics           | Each iteration creates a new binding, not a modification of the same variable |
| Immutable by default        | Loop variable cannot be modified, preventing accidents                        |
| `for mut`                   | Declare explicitly when modification is needed                                |
| No shadowing                | Loop variable cannot share a name with an outer variable                      |

In the next chapter you'll learn about the `while` loop—the standard way to repeat based on a
condition.
