---
title: Lambda Expressions
---

# Lambda Expressions

A lambda is an **anonymous function** you can define on the spot. In YaoXiang, regular functions are essentially named lambdas.

## Syntax

According to the language specification:

```
Lambda      ::= '(' ParamList? ')' '=>' Expr
            |  '(' ParamList? ')' '=>' Block
```

The simplest lambda:

```yaoxiang
# Expression-form lambda
double = (x) => x * 2

println(double(5))   # 10
println(double(10))  # 20
```

## Lambda-Function Unification

YaoXiang's core design philosophy is unified syntax. **A function is just a lambda bound to a name**:

```yaoxiang
# These two are completely equivalent:

# Lambda form
add = (a, b) => a + b

# Function form (syntactic sugar)
add: (a: Int, b: Int) -> Int = a + b
```

The first line is "assign a lambda to the variable `add`"; the second is "define a function named `add`". The compiler treats them almost identically.

## When to Use Lambdas

Lambdas shine in two scenarios:

### 1. Higher-Order Functions — passing functions as arguments

```yaoxiang
# Apply an operation to every element in a list
apply_to_all: (list: List(Int), op: (Int) -> Int) -> List(Int) = {
    mut result = []
    for item in list {
        result.append(op(item))
    }
    return result
}

numbers = [1, 2, 3, 4, 5]

# Pass lambdas
doubled = apply_to_all(numbers, (x) => x * 2)
squared = apply_to_all(numbers, (x) => x * x)

println(doubled)  # [2, 4, 6, 8, 10]
println(squared)  # [1, 4, 9, 16, 25]
```

### 2. One-off operations

No need to define a named function for logic you only use once:

```yaoxiang
# Sorting — define sort key on the fly
students = [
    {"name": "Alice", "score": 90},
    {"name": "Bob", "score": 85},
    {"name": "Charlie", "score": 92},
]

sorted_students = students.sort_by((a, b) => a["score"].compare(b["score"]))
```

## Block-Form Lambdas

When a lambda needs multiple statements, use the block form:

```yaoxiang
# Block-form lambda: can contain multiple statements
process = (data) => {
    cleaned = data.trim()
    lower = cleaned.lowercase()
    return lower
}

result = process("  Hello World  ")
println(result)  # "hello world"
```

Note that block-form lambdas require `return` to produce a value — exactly like functions.

## Multi-Parameter Lambdas

```yaoxiang
# Three parameters
add_three = (x, y, z) => x + y + z
println(add_three(1, 2, 3))  # 6

# No-parameter lambda
greet = () => "Hello, YaoXiang!"
println(greet())  # "Hello, YaoXiang!"
```

## Type Inference

Lambda parameter types are inferred from context:

```yaoxiang
# Types inferred from usage — no need for (x: Int) => x * 2
apply: (op: (Int) -> Int, value: Int) -> Int = op(value)

result = apply((x) => x + 10, 5)
println(result)  # 15
```

The compiler knows `op`'s type is `(Int) -> Int`, so `x` in the lambda is automatically inferred as `Int`.

> **Note**: Per function definition rules, parameter types must be annotated in at least one place — either the signature or the lambda head. When a lambda is passed as an argument, the type is typically provided by the receiver's signature.

## Summary

| Point | Description |
|-------|-------------|
| Syntax | `(params) => expr` or `(params) => { return ... }` |
| Essence | Function = named lambda |
| Higher-order | Lambdas can be passed as arguments |
| Block form | Multi-line logic with `{}` + `return` |
| Type inference | Parameter types inferred from context |

Lambdas are the most concise way to express "temporary logic" in YaoXiang. Master them, and your code will be more flexible and compact.
