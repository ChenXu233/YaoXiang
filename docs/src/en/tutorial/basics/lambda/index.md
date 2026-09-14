---
title: 'Lambda Expressions'
---

# Lambda Expressions

A Lambda is an **anonymous function that can be defined on the fly**. In YaoXiang, regular functions
are essentially named Lambdas.

## Syntax

According to the syntax specification:

```
Lambda      ::= '(' ParamList? ')' '=>' Expr
            |  '(' ParamList? ')' '=>' Block
```

The simplest Lambda:

```yaoxiang
// Expression-form Lambda
double = (x) => x * 2

print(double(5))   // 10
print(double(10))  // 20
```

## Unification of Lambda and Functions

YaoXiang's core design philosophy is unified syntax. **A function is a Lambda bound to a name**:

```yaoxiang
// These two are completely equivalent:

// Lambda form
add = (a, b) => a + b

// Function form (syntactic sugar)
add: (a: Int, b: Int) -> Int = a + b
```

The first line is "assigning a Lambda to the variable `add`", and the second is "defining a function
named `add`". The compiler handles them in almost the same way.

## When to Use Lambda

Lambda is best suited for two scenarios:

### 1. Higher-Order Functions — Passing Functions as Arguments

```yaoxiang
// Apply an operation to each element of a list
apply_to_all: (list: List(Int), op: (Int) -> Int) -> List(Int) = {
    mut result = []
    for item in list {
        result.append(op(item))
    }
    return result
}

numbers = [1, 2, 3, 4, 5]

// Pass in a Lambda
doubled = apply_to_all(numbers, (x) => x * 2)
squared = apply_to_all(numbers, (x) => x * x)

print(doubled)  // [2, 4, 6, 8, 10]
print(squared)  // [1, 4, 9, 16, 25]
```

### 2. Temporary One-Off Operations

No need to define a separate function for logic used only once:

```yaoxiang
// Sorting — define sorting rules on the fly
students = [
    {"name": "Alice", "score": 90},
    {"name": "Bob", "score": 85},
    {"name": "Charlie", "score": 92},
]

sorted_students = students.sort_by((a, b) => a["score"].compare(b["score"]))
```

## Block-Form Lambda

When a Lambda requires multi-line logic, use the block form:

```yaoxiang
// Block-form Lambda: can contain multiple statements
process = (data) => {
    cleaned = data.trim()
    lower = cleaned.lowercase()
    return lower
}

result = process("  Hello World  ")
print(result)  // "hello world"
```

Note that the block form requires `return` to return a value, which is exactly the same as
functions.

## Multi-Parameter Lambda

```yaoxiang
// Three parameters
add_three = (x, y, z) => x + y + z
print(add_three(1, 2, 3))  // 6

// No-parameter Lambda
greet = () => "Hello, YaoXiang!"
print(greet())  // "Hello, YaoXiang!"
```

## Type Inference

Lambda's parameter types can be inferred from context:

```yaoxiang
// Types are inferred from usage — no need to write `(x: Int) => x * 2`
apply: (op: (Int) -> Int, value: Int) -> Int = op(value)

result = apply((x) => x + 10, 5)
print(result)  // 15
```

The compiler knows `op`'s type is `(Int) -> Int`, so `x` in the Lambda `(x) => x + 10` is
automatically inferred as `Int`.

> **Note**: According to the rules of function definition, parameter types must be annotated in at
> least one place: the signature or the Lambda header. When a Lambda is passed as an argument, the
> type is usually provided by the receiver's signature.

## Summary

| Key Point              | Description                                             |
| ---------------------- | ------------------------------------------------------- |
| Syntax                 | `(params) => expr` or `(params) => { return ... }`      |
| Essence                | Function = Named Lambda                                 |
| Higher-Order Functions | Lambdas can be passed as arguments                      |
| Block Form             | Multi-line logic uses `{}` + `return`                   |
| Type Inference         | Parameter types are automatically inferred from context |

Lambda is the most concise way to express "temporary logic" in YaoXiang. Master it, and your code
will be more flexible and compact.
