---
title: for Loops
---

# for Loops

When you need to do the same thing to each element in a series, the `for` loop is the right tool. YaoXiang's `for` is designed to be concise and precise.

## Basic Syntax

The `for` statement in the language specification is:

```
for 'mut'? Identifier 'in' Expr Block
```

In plain terms: `for` followed by an optional `mut`, then a loop variable name, then `in` and the expression to iterate over, and finally the loop body block.

## Iterating Over Numeric Ranges

The most common usage is creating a range with `..` and iterating with `for`:

```yaoxiang
# From 0 to 4 (not including 5)
for i in 0..5 {
    println(i)
}
# Output: 0 1 2 3 4
```

`0..5` means from 0 (inclusive) to 5 (exclusive). This is the universal convention in computer science — the half-open interval.

You can also use variables for the start and end of a range:

```yaoxiang
start = 10
end = 15
for n in start..end {
    println(n)
}
# Output: 10 11 12 13 14
```

## Iterating Over Lists

`for` can iterate over more than just numeric ranges — it works directly on lists, arrays, and other collections:

```yaoxiang
colors = ["red", "orange", "yellow", "green", "blue"]

for color in colors {
    println("Current color: " + color)
}
# Output:
# Current color: red
# Current color: orange
# ... and so on
```

## The Unique Semantic of for: Binding a New Value Each Iteration

YaoXiang's `for` loop has a design that differs from many other languages: **each iteration binds a new value, rather than modifying the same variable**.

To visualize:

| Iteration | What happens |
|-----------|-------------|
| 1st | A new binding `i = 0` is created, loop body runs, then the binding is destroyed |
| 2nd | A new binding `i = 1` (brand new) is created, loop body runs, then destroyed |
| 3rd | A new binding `i = 2` is created, loop body runs, then destroyed |
| ... | ... |
| Loop ends | The range is exhausted, loop terminates |

This means each iteration's loop variable is an independent new value. This greatly helps safety — you never need to worry about accidental modification:

```yaoxiang
for i in 1..5 {
    # i = i + 1   // Error: immutable by default, cannot modify i
    println(i)
}
```

## for mut: Declare When Mutation Is Needed

If you genuinely need to modify the loop variable inside the body (e.g., as an accumulator), use `for mut`:

```yaoxiang
# for mut allows modifying the binding inside the loop body
for mut i in 0..5 {
    i = i * 2
    println(i)
}
# Output: 0 2 4 6 8
```

Note: even with `for mut`, each iteration is still a fresh binding. `for mut` only makes the new binding mutable — it does not carry modifications from one iteration to the next.

```yaoxiang
for mut i in 1..5 {
    i = i + 100
    println(i)        # Prints 101, 102, 103, 104 each time
}
# Each iteration restarts i from the range value; previous modifications don't carry over
```

## Loop Variables Cannot Shadow Outer Variables

YaoXiang forbids variable shadowing. The `for` loop variable cannot share a name with a variable in an outer scope:

```yaoxiang
# Incorrect
i = 10
# for i in 1..5 {     // Compile error! i is already declared in outer scope
#     println(i)
# }

# Correct — use a different name
i = 10
for j in 1..5 {
    println(j)
}
```

This rule ensures you will never wonder "which variable is this code referring to?"

## Comparison with Other Languages

| Language | for loop variable semantics |
|----------|---------------------------|
| YaoXiang | Binds a new value each iteration |
| Rust | Modifies the same variable (requires mut) |
| Python | Modifies the same variable |
| C/C++ | Modifies the same variable |

YaoXiang's design aligns with human intuition — "for each element in the collection, do something" — each element is an independent entity.

## Summary

| Point | Description |
|-------|-------------|
| Iterating ranges | `for i in 0..5`, left-inclusive right-exclusive |
| Iterating collections | `for item in list`, takes elements one by one |
| Binding semantics | Each iteration creates a new binding, not modifying the same variable |
| Immutable by default | Loop variable cannot be modified, preventing accidents |
| `for mut` | Explicitly declare when mutation is needed |
| No shadowing | Loop variable cannot shadow outer variables |

Next chapter: `while` loops — the standard way to repeat based on a condition.
