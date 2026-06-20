---
title: for Loop
---

# for Loop

When you need to perform the same operation on a series of elements one by one, the `for` loop is the right tool. YaoXiang's `for` loop is designed to be concise and precise.

## Basic Syntax

The formal definition of the `for` statement in the language specification:

```
for 'mut'? Identifier 'in' Expr Block
```

In plain words: it starts with `for`, optionally followed by `mut`, then the loop variable name, then `in` and the expression to iterate over, and finally the loop body block.

## Iterating Over Numeric Ranges

The most common usage is to create a range with `..` and then iterate over it with `for`:

```yaoxiang
# From 0 to 4 (excluding 5)
for i in 0..5 {
    println(i)
}
# Output: 0 1 2 3 4
```

`0..5` represents a range starting at 0 (inclusive) and ending at 5 (exclusive). This follows the universal convention in computer science—a half-open interval (left-closed, right-open).

You can also use variables to define the start and end of the range:

```yaoxiang
start = 10
end = 15
for n in start..end {
    println(n)
}
# Output: 10 11 12 13 14
```

## Iterating Over Lists

`for` is not limited to iterating over numeric ranges—you can also iterate directly over collections like lists and arrays:

```yaoxiang
colors = ["红", "橙", "黄", "绿", "蓝"]

for color in colors {
    println("当前颜色: " + color)
}
# Output:
# 当前颜色: 红
# 当前颜色: 橙
# ... and so on
```

## for's Unique Semantics: A New Binding Per Iteration

YaoXiang's `for` loop has a design that differs from other languages: **each iteration creates a new binding, rather than modifying the same variable**.

Use this table to understand it:

| Iteration | What Happens |
|-----------|--------------|
| 1st | A new binding `i = 0` is created, the loop body executes, then the binding is destroyed |
| 2nd | A new binding `i = 1` is created (a brand-new binding), the loop body executes, then it's destroyed |
| 3rd | A new binding `i = 2` is created, the loop body executes, then it's destroyed |
| ... | ... |
| Loop ends | The range is exhausted and the loop terminates |

This means the loop variable in each iteration is an independent new value. This is a huge boost for safety—you never have to worry about the loop variable being accidentally modified:

```yaoxiang
for i in 1..5 {
    # i = i + 1   // Error: immutable by default, cannot modify i
    println(i)
}
```

## for mut: Declare Mutability Explicitly When You Need It

If you genuinely need to modify the loop variable inside the loop body (for example, using it as an accumulator), use `for mut`:

```yaoxiang
# Use for mut to allow modifying the binding inside the loop body
for mut i in 0..5 {
    i = i * 2
    println(i)
}
# Output: 0 2 4 6 8
```

Note: even with `for mut`, each iteration is still a new binding. `for mut` only makes the new binding itself mutable—it does not carry modifications from the previous iteration into the next one.

```yaoxiang
for mut i in 1..5 {
    i = i + 100
    println(i)        # Prints 101, 102, 103, 104 each time
}
# Each iteration i restarts from the range value; the previous iteration's modification does not affect the next
```

## Loop Variables Cannot Shadow Outer Variables

YaoXiang forbids variable shadowing. The loop variable in `for` cannot share the same name as a variable in an outer scope:

```yaoxiang
# Wrong example
i = 10
# for i in 1..5 {     # Compilation error! i is already declared in the outer scope
#     println(i)
# }

# Correct way—use a different name
i = 10
for j in 1..5 {
    println(j)
}
```

This rule ensures you never get confused about "which variable does the current code refer to."

## Comparison With Other Languages

| Language | for Loop Variable Semantics |
|----------|------------------------------|
| YaoXiang | A new binding is created per iteration |
| Rust | Modifies the same variable (requires mut) |
| Python | Modifies the same variable |
| C/C++ | Modifies the same variable |

YaoXiang's design aligns more closely with human intuition—"do something with each element in the collection"—where each element is an independent individual.

## Summary

| Key Point | Description |
|-----------|-------------|
| Iterating over a range | `for i in 0..5`, left-closed right-open |
| Iterating over a collection | `for item in list`, takes elements one by one |
| Binding semantics | A new binding is created per iteration, not a modification of the same variable |
| Immutable by default | Loop variables cannot be modified, preventing accidents |
| `for mut` | Declare explicitly when modification is needed |
| Shadowing prohibited | Loop variables cannot share a name with outer variables |

In the next chapter you'll learn about the `while` loop—the standard way to repeat based on a condition.