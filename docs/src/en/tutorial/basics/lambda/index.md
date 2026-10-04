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

The first line is 'assigning a Lambda to the variable `add`', and the second line is 'defining a
function named `add`'. The compiler handles them in almost the same way.

## When to Use Lambda

Lambda is best suited for two scenarios:

### 1. Higher-Order Functions — Passing Functions as Arguments

```yaoxiang
use std.list

// Apply an operation to every element of the list
// Note: do not name the parameter `list` (it would shadow the std.list module);
// write the type as `Vec(Int)` — values annotated as `List(T)` are currently
// not accepted by `for` (E1002)
apply_to_all: (xs: Vec(Int), op: (Int) -> Int) -> Vec(Int) = {
    mut result = []
    for item in xs {
        result = list.push(result, op(item))
    }
    return result
}

main = () => {
    // Pass in a Lambda
    doubled = apply_to_all([1, 2, 3, 4, 5], (x) => x * 2)
    squared = apply_to_all([1, 2, 3, 4, 5], (x) => x * x)

    print(doubled)  // [2, 4, 6, 8, 10]
    print(squared)  // [4, 16, 36, 64, 100]
}
```

### 2. Ad-Hoc One-Off Operations

No need to define a dedicated function for logic that is only used once:

```yaoxiang
use std.list

// Note: std.list in 0.8.2 has no sort / sort_by (verified to produce E1042),
// so sorting has to be hand-written; here we demonstrate the same
// "define rules on the fly" pattern with map / filter
main = () => {
    scores = [90, 85, 92, 78]

    passed = list.filter(scores, (s) => s >= 85)   // [90, 85, 92]
    boosted = list.map(scores, (s) => s + 5)       // [95, 90, 97, 83]

    print(passed)
    print(boosted)
}
```

## Block-Form Lambda

When a Lambda needs multi-line logic, use the block form:

```yaoxiang
use std.string

// Block-form Lambda: can contain multiple statements
// Note: String has no instance methods — `data.trim()` reports E1053;
// use std.string's function form
process = (data: String) => {
    cleaned = string.trim(data)
    lower = string.lower(cleaned)
    return lower
}

main = () => {
    result = process("  Hello World  ")
    print(result)  // "hello world"
}
```

Note that the block form requires `return` to return a value, which is exactly the same as a regular
function.

## Multi-Parameter Lambda

```yaoxiang
// Three parameters
add_three = (x, y, z) => x + y + z
print(add_three(1, 2, 3))  // 6

// Zero-argument Lambda
greet = () => "Hello, YaoXiang!"
print(greet())  // "Hello, YaoXiang!"
```

## Type Inference

The parameter types of a Lambda can be inferred from context:

```yaoxiang
// Type inferred from the call site — no need to write (x: Int) => x * 2
apply: (op: (Int) -> Int, value: Int) -> Int = op(value)

result = apply((x) => x + 10, 5)
print(result)  // 15
```

The compiler knows that `op`'s type is `(Int) -> Int`, so `x` in the Lambda `(x) => x + 10` is
automatically inferred as `Int`.

> **Note**: According to the rules of function definitions, parameter types must be annotated in at
> least one of either the signature or the Lambda header. When a Lambda is passed as an argument,
> the type is usually provided by the receiver's signature.

## Summary

| Point              | Description                                             |
| ------------------ | ------------------------------------------------------- |
| Syntax             | `(params) => expr` or `(params) => { return ... }`      |
| Essence            | Function = named Lambda                                 |
| Higher-Order Funcs | Lambdas can be passed as arguments                      |
| Block Form         | Use `{}` + `return` for multi-line logic                |
| Type Inference     | Parameter types are automatically inferred from context |

Lambda is the most concise way to express "ad-hoc logic" in YaoXiang. Master it, and your code will
become more flexible and compact.
