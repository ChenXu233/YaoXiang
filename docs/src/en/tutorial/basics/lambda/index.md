---
title: Lambda Expressions
---

# Lambda Expressions

A Lambda is **an anonymous function that you can define on the fly**. In YaoXiang, regular functions are essentially named Lambdas.

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

## Lambda and Function Unification

YaoXiang's core design philosophy is unified syntax. **Functions are Lambdas bound to a name**:

```yaoxiang
// These two are completely equivalent:

// Lambda form
add = (a, b) => a + b

// Function form (syntactic sugar)
add: (a: Int, b: Int) -> Int = a + b
```

The first line is "assigning a Lambda to the variable `add`", and the second line is "defining a function named `add`". The compiler processes them in nearly the same way.

## When to Use Lambda

Lambda is most suitable for two scenarios:

### 1. Higher-order functions — passing functions as arguments

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

// Pass a Lambda
doubled = apply_to_all(numbers, (x) => x * 2)
squared = apply_to_all(numbers, (x) => x * x)

print(doubled)  // [2, 4, 6, 8, 10]
print(squared)  // [1, 4, 9, 16, 25]
```

### 2. Temporary one-off operations

No need to define a separate function for logic used only once:

```yaoxiang
// Sorting — temporarily define sorting rules
students = [
    {"name": "Alice", "score": 90},
    {"name": "Bob", "score": 85},
    {"name": "Charlie", "score": 92},
]

sorted_students = students.sort_by((a, b) => a["score"].compare(b["score"]))
```

## Block-form Lambda

When a Lambda needs multiple lines of logic, use the block form:

```yaoxiang
// Block Lambda: can contain multiple statements
process = (data) => {
    cleaned = data.trim()
    lower = cleaned.lowercase()
    return lower
}

result = process("  Hello World  ")
print(result)  // "hello world"
```

Note that the block form requires `return` to return a value, which is exactly the same as functions.

## Multi-parameter Lambda

```yaoxiang
// Three parameters
add_three = (x, y, z) => x + y + z
print(add_three(1, 2, 3))  // 6

// No-parameter Lambda
greet = () => "Hello, YaoXiang!"
print(greet())  // "Hello, YaoXiang!"
```

## Type Inference

Lambda parameter types can be inferred from context:

```yaoxiang
// Types are inferred from the call site — no need to write (x: Int) => x * 2
apply: (op: (Int) -> Int, value: Int) -> Int = op(value)

result = apply((x) => x + 10, 5)
print(result)  // 15
```

The compiler knows that the type of `op` is `(Int) -> Int`, so `x` in the Lambda `(x) => x + 10` is automatically inferred to be `Int`.

> **Note**: According to the function definition rules, parameter types must be annotated in at least one place — either in the signature or in the Lambda header. When a Lambda is passed as an argument, the type is usually provided by the receiver's signature.

## Summary

| Key Point | Description |
|------|------|
| Syntax | `(params) => expr` or `(params) => { return ... }` |
| Essence | Function = named Lambda |
| Higher-order functions | Lambdas can be passed as arguments |
| Block form | Use `{}` + `return` for multi-line logic |
| Type inference | Parameter types are automatically inferred from context |

Lambda is the most concise way to express "temporary logic" in YaoXiang. Master it, and your code will be more flexible and compact.