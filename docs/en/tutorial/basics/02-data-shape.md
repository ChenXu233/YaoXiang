---
title: Chapter 2: The Shape of Data
---

# Chapter 2: The Shape of Data

> **Chapter Goal**: Understand what types are, and why we need to "label" data

## 2.1 Data in Computers

Computers store all kinds of **data**:

| Data | Examples |
|------|----------|
| Numbers | `42`, `3.14`, `-7` |
| Text | `"你好"`, `"Hello"` |
| Yes/No | `true`, `false` |
| Colors | Red, Blue |

Programs are all about processing this data.

## 2.2 What is a Type?

**Type** is a "kind label" for data.

Imagine you're at a supermarket:
- Fruits go in the **fruit section**
- Vegetables go in the **vegetable section**
- Snacks go in the **snack section**

Types are like **section labels** in a supermarket, telling the computer "what kind of data this is".

```yaoxiang
# Data of different types
age: Int = 25              # Integer type
price: Float = 19.99       # Decimal type
name: String = "小明"      # Text type
is_student: Bool = true    # Yes/No type
```

## 2.3 Why Use Types?

**Question**: What happens without types?

Imagine a supermarket without sections:
- Customers want to buy apples but search through a pile of socks
- Store clerks don't know where to put new goods

**The role of types**:

| Role | Description | Example |
|------|-------------|---------|
| **Organize** | Put similar data together | All Int together |
| **Restrict** | Prevent wrong operations | Can't "add" text |
| **Communicate** | Make code easier to understand | See type, know purpose |
| **Optimize** | Computer can store efficiently | Int takes 8 bytes |

## 2.4 YaoXiang's Basic Types

YaoXiang provides various basic types for us:

| Type | Name | Example | Use |
|------|------|---------|-----|
| `Int` | Integer | `42`, `-7`, `0` | Counting, numbering |
| `Float` | Decimal | `3.14`, `-0.5` | Money, scientific calculation |
| `Bool` | Boolean | `true`, `false` | Yes/No judgment |
| `String` | String | `"你好"`, `"Hi"` | Text |
| `Char` | Character | `'A'`, `'中'` | Single character |
| `Void` | Void | `null` | Represents "no value" |

## 2.5 Basic Operations for Types

Each type has its own "operations" it can do:

```yaoxiang
# Integer operations
a: Int = 10
b: Int = 3
c: Int = a + b      # Addition: 13
d: Int = a * b      # Multiplication: 30

# Decimal operations
x: Float = 3.14
y: Float = x * 2    # Multiplication: 6.28

# Text operations
greeting: String = "你好"
name: String = "小明"
message: String = greeting + " " + name  # Concatenation: "你好 小明"

# Yes/No operations
is_adult: Bool = age >= 18
is_weekend: Bool = day == "Saturday"
```

## 2.6 Type Inference

When writing code, sometimes types can be **automatically inferred**:

```yaoxiang
# Complete写法 (recommended for beginners)
age: Int = 18

# Short写法 (compiler automatically infers type)
age = 18           # Inferred as Int
price = 19.99      # Inferred as Float
name = "小明"      # Inferred as String
is_done = true     # Inferred as Bool
```

**Suggestion**: Beginners should **explicitly write out types**, it's easier to understand the code!

## 2.7 Type Conversion

Different types can be converted to each other:

```yaoxiang
# Integer to decimal
num: Int = 42
num_float: Float = num.as_float()  # 42.0

# Decimal to integer (fractional part is discarded)
pi: Float = 3.14
pi_int: Int = pi.as_int()          # 3

# Number to text
age: Int = 25
age_str: String = age.as_string()  # "25"
```

## 2.8 Chapter Summary

| Concept | Understanding |
|---------|----------------|
| Data | Information processed by programs (numbers, text, yes/no, etc.) |
| Type | "Kind label" for data, telling the computer what the data is |
| Role of types | Organize, restrict, communicate, optimize |
| Basic types | Int, Float, Bool, String, Char, Void |

## 2.9 I Ching Introduction

> "The great virtue of heaven and earth is called life; the great treasure of saints is called position."
> — "Xici Zhuan", Book of Changes
>
> Heaven and earth give birth to all things, each has its own nature and returns to its own position.
> The division of types is like all things returning to their positions — integers return to integers, text returns to text.
> With "position", all things can fulfill their uses; with types, data can each perform their duties.
