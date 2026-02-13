---
title: 'Chapter 7: The Magic of Generics'
---

# Chapter 7: The Magic of Generics

> **Chapter Goal**: Understand generics, learn to write universal code with type parameters

## 7.1 The Problem

Suppose we want to write a function that "gets the first element of a list":

```yaoxiang
# Integer list
first_int: (list: List[Int]) -> Option[Int] = {
    # ...
}

# String list
first_string: (list: List[String]) -> Option[String] = {
    # ...
}

# Float list
first_float: (list: List[Float]) -> Option[Float] = {
    # ...
}
```

**Problem**: Writing a function for each type is too troublesome!

## 7.2 Solution: Generics

**Generics** is writing code with **type parameters**, write once, use with multiple types:

```yaoxiang
# Generic function: one function, works for all types
first: [T](list: List[T]) -> Option[T] = {
    # T is a "type parameter", will be replaced with concrete type when called
    if list.length > 0 {
        return Option.some(list[0])
    } else {
        return Option.none
    }
}
```

**Usage**:

```yaoxiang
# Integer list
int_list: List[Int] = List(1, 2, 3)
first_int: Option[Int] = first(int_list)           # Option.some(1)

# String list
str_list: List[String] = List("a", "b", "c")
first_str: Option[String] = first(str_list)         # Option.some("a")

# Float list
float_list: List[Float] = List(1.1, 2.2, 3.3)
first_float: Option[Float] = first(float_list)       # Option.some(1.1)
```

## 7.3 Generic Syntax

```
[name: ] [generic parameters] (parameters) -> return type = implementation

# Generic parameters: [T] or [T, U, ...]
```

```yaoxiang
# Single generic parameter
map: [T, R](list: List[T], f: Fn(T) -> R) -> List[R] = {
    # T: Type of input elements
    # R: Type of output elements
}

# Multiple generic parameters
combine: [T, U](a: T, b: U) -> (T, U) = {
    return (a, b)
}

# Generic parameters with constraints
clone: [T: Clone](value: T) -> T = {
    # T must implement Clone interface
    return value.clone()
}
```

## 7.4 Generic Types

Not only functions, types can also be generic:

```yaoxiang
# Option type (may have value, may not)
Option: Type[T] = {
    some: (T) -> Self,
    none: () -> Self
}

# Using Option
maybe_number: Option[Int] = Option.some(42)
maybe_string: Option[String] = Option.none
```

```
Option Type
┌─────────────────────────────────────────┐
│            Option[T]                      │
├─────────────────────────────────────────┤
│  ┌───────────────────────────────────┐  │
│  │  Option.some(value: T)             │  │
│  │  Option.none()                    │  │
│  └───────────────────────────────────┘  │
└─────────────────────────────────────────┘
```

## 7.5 List Type

Lists in YaoXiang are generic:

```yaoxiang
# List type definition
List: Type[T] = {
    data: Array[T],      # Array storing type T
    length: Int,        # List length

    push: [T](self: List[T], item: T) -> Void,
    get: [T](self: List[T], index: Int) -> Option[T],
    map: [T, R](self: List[T], f: Fn(T) -> R) -> List[R],
    filter: [T](self: List[T], f: Fn(T) -> Bool) -> List[T],
}

# Usage
numbers: List[Int] = List(1, 2, 3, 4, 5)
numbers.push(6)           # Add element

# map: transform each element
doubled: List[Int] = numbers.map((x) => x * 2)  # 2, 4, 6, 8, 10, 12

# filter: filter elements
evens: List[Int] = numbers.filter((x) => x % 2 == 0)  # 2, 4, 6
```

## 7.6 Why Generics?

| Benefit | Description |
|---------|-------------|
| **Code reuse** | Write once, works for multiple types |
| **Type safety** | Compiler checks type correctness |
| **Avoid duplication** | Don't need to write similar code for each type |
| **Abstraction** | Write universal algorithms |

**Comparison**:

```yaoxiang
# ❌ No generics: write one for each type
first_int: (List[Int]) -> Option[Int] = ...
first_string: (List[String]) -> Option[String] = ...
first_float: (List[Float]) -> Option[Float] = ...
first_person: (List[Person]) -> Option[Person] = ...

# ✅ With generics: one function, all types
first: [T](List[T]) -> Option[T] = ...
```

## 7.7 Type Inference

Compiler can automatically infer generic parameters:

```yaoxiang
# Complete写法
numbers: List[Int] = List[Int](1, 2, 3)

# Infer写法 (recommended)
numbers = List(1, 2, 3)     # Compiler infers as List[Int]
```

## 7.8 Chapter Summary

| Concept | Description | Example |
|---------|-------------|---------|
| Generics | Write universal code with type parameters | `first: [T](List[T]) -> Option[T]` |
| Type parameter | Placeholder type in generics | `T`, `R`, `U` |
| Generic type | Template type with type parameters | `Option[T]`, `List[T]` |
| Generic function | Function with type parameters | `map: [T, R](...) -> ...` |

## 7.9 I Ching Introduction

> "Heaven and earth are not benevolent, they treat all things as straw dogs; saints are not benevolent, they treat people as straw dogs."
> — Tao Te Ching
>
> The way of generics is the same:
> - **Tao** (algorithm) is universal, doesn't favor any type
> - **Utensil** (concrete type) is determined at use time
>
> One `first[T]`, can take first of Int, first of String, first of Person...
> This is the programming interpretation of "something born from nothing, one born from many".
>
> **Generics is the "Tao" of algorithms; types are the concrete "utensils".**
