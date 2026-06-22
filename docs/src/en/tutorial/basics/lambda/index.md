---
title: Lambda Expressions
---

# Lambda Expressions

A Lambda is an **anonymous, inline-defined function**. In YaoXiang, a regular function is essentially a named Lambda.

## Syntax

According to the syntax specification:

```
Lambda      ::= '(' ParamList? ')' '=>' Expr
            |  '(' ParamList? ')' '=>' Block
```

The simplest Lambda:

```yaoxiang
# Expression-form Lambda
double = (x) => x * 2

println(double(5))   # 10
println(double(10))  # 20
```

## Unification of Lambdas and Functions

YaoXiang's core design philosophy is unified syntax. **A function is a Lambda bound to a name**:

```yaoxiang
# These two are completely equivalent:

# Lambda form
add = (a, b) => a + b

# Function form (syntactic sugar)
add: (a: Int, b: Int) -> Int = a + b
```

The first line is "assign a Lambda to the variable `add`", and the second line is "define a function named `add`". The compiler processes them in nearly the same way.

## When to Use Lambdas

Lambdas are best suited for two scenarios:

### 1. Higher-order functions — passing functions as arguments

```yaoxiang
# Apply an operation to every element of a list
apply_to_all: (list: List(Int), op: (Int) -> Int) -> List(Int) = {
    mut result = []
    for item in list {
        result.append(op(item))
    }
    return result
}

numbers = [1, 2, 3, 4, 5]

# Pass in a Lambda
doubled = apply_to_all(numbers, (x) => x * 2)
squared = apply_to_all(numbers, (x) => x * x)

println(doubled)  # [2, 4, 6, 8, 10]
println(squared)  # [1, 4, 9, 16, 25]
```

### 2. One-off, throwaway operations

No need to define a dedicated function for logic used only once:

```yaoxiang
# Sorting — define the sort rule inline
students = [
    {"name": "Alice", "score": 90},
    {"name": "Bob", "score": 85},
    {"name": "Charlie", "score": 92},
]

sorted_students = students.sort_by((a, b) => a["score"].compare(b["score"]))
```

## Block-form Lambdas

When a Lambda requires multiple lines of logic, use the block form:

```yaoxiang
# Block Lambda: can contain multiple statements
process = (data) => {
    cleaned = data.trim()
    lower = cleaned.lowercase()
    return lower
}

result = process("  Hello World  ")
println(result)  # "hello world"
```

Note that the block form requires `return` to produce a value, which is fully consistent with functions.

## Multi-parameter Lambdas

```yaoxiang
# Three parameters
add_three = (x, y, z) => x + y + z
println(add_three(1, 2, 3))  # 6

# Zero-parameter Lambda
greet = () => "Hello, YaoXiang!"
println(greet())  # "Hello, YaoXiang!"
```

## Type Inference

The parameter types of a Lambda can be inferred from context:

```yaoxiang
# Types are inferred from the usage site — no need to write (x: Int) => x * 2
apply: (op: (Int) -> Int, value: Int) -> Int = op(value)

result = apply((x) => x + 10, 5)
println(result)  # 15
```

The compiler knows `op`'s type is `(Int) -> Int`, so `x` in the Lambda `(x) => x + 10` is automatically inferred as `Int`.

> **Note**: According to the rules of function definition, parameter types must be annotated in at least one of the signature or the Lambda header. When a Lambda is passed as an argument, the type is usually provided by the receiver's signature.

## Summary

| Key Point | Description |
|-----------|-------------|
| Syntax | `(params) => expr` or `(params) => { return ... }` |
| Essence | A function = a named Lambda |
| Higher-order functions | Lambdas can be passed as arguments |
| Block form | Multi-line logic uses `{}` + `return` |
| Type inference | Parameter types are automatically inferred from context |

Lambdas are the most concise way to express "throwaway logic" in YaoXiang. Master them, and your code will become more flexible and compact.