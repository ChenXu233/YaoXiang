---
title: for Loop
---

# for Loop

When you need to do the same thing to a series of elements one by one, the `for` loop is the right
tool. YaoXiang's `for` loop is designed to be clean and precise.

## Basic Syntax

The formal definition of the `for` statement in the syntax specification:

```
for 'mut'? Identifier 'in' Expr Block
```

In plain terms: start with `for`, optionally followed by `mut`, then the loop variable name, then
`in` and the expression to iterate over, and finally the loop body code block.

## Iterating Over Number Ranges

The most common usage is to create a range with `..`, then iterate with `for`:

```yaoxiang
// from 0 to 4 (excluding 5)
for i in 0..5 {
    print(i)
}
// Output: 0 1 2 3 4
```

`0..5` represents a range starting from 0 (inclusive) and ending at 5 (exclusive). This is a common
convention in computer science—the half-open interval [start, end).

You can also use variables to define the start and end of a range:

```yaoxiang
start = 10
end = 15
for n in start..end {
    print(n)
}
// Output: 10 11 12 13 14
```

## Iterating Over Lists

`for` can not only iterate over number ranges, but also directly iterate over lists, arrays, and
other collections:

```yaoxiang
colors = ["Red", "Orange", "Yellow", "Green", "Blue"]

for color in colors {
    print("Current color: " + color)
}
// Output:
// Current color: Red
// Current color: Orange
// ... and so on
```

## The Unique Semantics of for: Binding a New Value on Each Iteration

YaoXiang's `for` loop has a design different from other languages: **each iteration creates a new
binding rather than modifying the same variable**.

Understanding through a table:

| Iteration | What happens                                                                    |
| --------- | ------------------------------------------------------------------------------- |
| 1st       | Create new binding `i = 0`, execute loop body, then destroy binding             |
| 2nd       | Create new binding `i = 1` (brand new binding), execute loop body, then destroy |
| 3rd       | Create new binding `i = 2`, execute loop body, then destroy                     |
| ...       | ...                                                                             |
| Loop ends | Range exhausted, loop terminates                                                |

This means the loop variable on each iteration is an independent new value. This is very helpful for
safety—you don't have to worry about the loop variable being accidentally modified:

```yaoxiang
for i in 1..5 {
    // i = i + 1   // Error: immutable by default, cannot modify i
    print(i)
}
```

## for mut: Explicit Declaration When Modification Is Needed

If you really need to modify the loop variable inside the loop body (for example, as an
accumulator), use `for mut`:

```yaoxiang
// Using for mut allows you to modify the binding inside the loop body
for mut i in 0..5 {
    i = i * 2
    print(i)
}
// Output: 0 2 4 6 8
```

Note: Even with `for mut`, each iteration is still a new binding. `for mut` only makes the new
binding itself mutable, it does not pass modifications from the previous iteration to the next
iteration.

```yaoxiang
for mut i in 1..5 {
    i = i + 100
    print(i)        // Each time prints 101, 102, 103, 104
}
// Each iteration i starts anew from the range value, the previous modification doesn't affect the next iteration
```

## Loop Variables Cannot Shadow Outer Variables

YaoXiang prohibits variable shadowing. The loop variable of `for` cannot have the same name as a
variable in the outer scope:

```yaoxiang
// Wrong example
i = 10
// for i in 1..5 {     // Compilation error! i is already declared in the outer scope
//     print(i)
// }

// Correct approach—use a different name
i = 10
for j in 1..5 {
    print(j)
}
```

This rule ensures you never wonder "which variable does the current code refer to".

## Comparison with Other Languages

| Language | for loop variable semantics             |
| -------- | --------------------------------------- |
| YaoXiang | Bind a new value on each iteration      |
| Rust     | Modify the same variable (requires mut) |
| Python   | Modify the same variable                |
| C/C++    | Modify the same variable                |

YaoXiang's design is closer to human intuition—"do something for each element in a collection"—each
element is an independent individual.

## Summary

| Key Point            | Description                                                             |
| -------------------- | ----------------------------------------------------------------------- |
| Iterate ranges       | `for i in 0..5`, half-open interval [start, end)                        |
| Iterate collections  | `for item in list`, take elements one by one                            |
| Binding semantics    | Create a new binding on each iteration, not modifying the same variable |
| Immutable by default | Loop variables cannot be modified, prevents accidents                   |
| `for mut`            | Explicit declaration when modification is needed                        |
| Prohibit shadowing   | Loop variables cannot have the same name as outer variables             |

In the next chapter you will learn about `while` loops—the standard way to repeat execution based on
a condition.
