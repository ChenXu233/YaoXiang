---
title: 'Chapter 3: The Journey of Values'
---

# Chapter 3: The Journey of Values

> **Chapter Goal**: Understand variables, assignment, scope, and the lifecycle of data in memory

## 3.1 What is a Variable?

A **variable** is a "name" for data.

Imagine you have many drawers, each drawer can hold things. You put a name on the drawer, that's a variable.

```yaoxiang
# Create a variable, name is "age", value is 25
age: Int = 25

# Create a variable, name is "name", value is "Alex"
name: String = "Alex"
```

**Understanding**:
- `age` is the **name** of the variable
- `Int` is the **type** of the variable
- `25` is the **value** of the variable

```
┌─────────────────────────────────────────┐
│                 Memory                   │
│  ┌─────────┐      ┌─────────┐        │
│  │  Name   │ ────▶ │  Value  │        │
│  │  age   │      │   25    │        │
│  └─────────┘      └─────────┘        │
│  ┌─────────┐      ┌─────────┐        │
│  │  name  │ ────▶ │ "Alex"  │        │
│  └─────────┘      └─────────┘        │
└─────────────────────────────────────────┘
```

## 3.2 Assignment

**Assignment** is the process of putting a value into a variable:

```yaoxiang
# First assignment (initialization)
age: Int = 18

# Reassignment (change the variable's value)
age = 20
age = 25
```

**Important rules**:
- First assignment is called **initialization**
- Reassignment **overwrites** the original value
- After assignment, the old value "disappears" (unless you made a copy)

## 3.3 Variable Names

Variable names can be chosen freely, but must follow rules:

| Rule | Correct Examples | Wrong Examples |
|------|------------------|----------------|
| Start with letter or underscore | `age`, `_count`, `name1` | `1age`, `$name` |
| Can be followed by letters, numbers, underscores | `age1`, `count_2` | `age!`, `name@` |
| Case sensitive | `Age` and `age` are different | - |
| Cannot use keywords | `if`, `while`, `return` | - |

**Good naming habits**:

```yaoxiang
# ✅ Good names (meaningful)
user_age = 18
total_price = 99.99
is_valid = true

# ❌ Bad names (can't see purpose)
a = 18
x1 = 99.99
flag = true
```

## 3.4 What is Scope?

**Scope** is the area where a variable "can be used".

Imagine each room is a scope, variables can only be used in the room where they were born:

```yaoxiang
# Global scope (can be used throughout the file)
global_var: Int = 100

my_function: () -> Void = {
    # Local scope (only usable in this function)
    local_var: Int = 50
    print(global_var)    # ✅ Can use
    print(local_var)     # ✅ Can use
}

# print(local_var)      # ❌ Error! Can't use outside
```

**Diagram**:

```
┌─────────────────────────────────────────┐
│              Global Scope                │
│  global_var = 100                       │
│                                         │
│  ┌─────────────────────────────────┐   │
│  │    my_function Local Scope       │   │
│  │    local_var = 50                │   │
│  │    Can use global_var and local_var│   │
│  └─────────────────────────────────┘   │
│                                         │
│  ┌─────────────────────────────────┐   │
│  │    Another Local Scope          │   │
│  │    Cannot use local_var         │   │
│  └─────────────────────────────────┘   │
└─────────────────────────────────────────┘
```

## 3.5 Value Lifecycle

**Lifecycle** is the process from a value's "birth" to "death":

```yaoxiang
create_value: () -> Void = {
    # Value is "born"
    number: Int = 42
    print(number)        # Can use

    # Value "dies" here (function ends)
}

# number no longer exists here
```

**Rules**:
- Variables "die" when their scope **ends**
- After death, the variable cannot be used anymore

## 3.6 Mutable and Immutable

Variables can be **mutable** (can change) or **immutable** (cannot change):

```yaoxiang
# Immutable variable (default)
age: Int = 18
# age = 20      # ❌ Error! Cannot reassign

# Mutable variable (needs mut)
mut counter: Int = 0
counter = counter + 1   # ✅ Can change
counter = counter + 1   # ✅ Can change
```

### 3.6.1 Field-Level Immutability

YaoXiang's immutability is **field-level**, not object-level:

```yaoxiang
# Person type
Person: Type = {
    name: String,       # Immutable field
    mut age: Int,       # Mutable field
    mut score: Float    # Mutable field
}

# Usage
person: Person = Person("Alex", 18, 95.5)

# person.name = "Bob"    # ❌ Error! name is immutable field
person.age = 19           # ✅ Can change
person.score = 98.0       # ✅ Can change
```

**Why field-level?**

| Level | Safety | Flexibility |
|-------|--------|-------------|
| Object-level mut | Weaker | Stronger |
| **Field-level mut** | **Stronger** | **Sufficient** |

- **Safer**: Only fields that need to change can be changed
- **Clearer**: You can immediately see which fields are mutable
- **Absolutely safe**: Compile-time checking, runtime guarantee

### 3.6.2 When to Use Mutable vs Immutable?

| Situation | Recommendation |
|----------|----------------|
| Value doesn't need to change | **Immutable** (safer) |
| Value needs frequent changes | **Mutable field** (mut) |
| Not sure | **Immutable** (default choice) |

## 3.7 Constants

**Constants** are values that don't change:

```yaoxiang
# Constants (once defined, never change)
PI: Float = 3.14159
MAX_SIZE: Int = 100

# Characteristics of constants:
# 1. Cannot be reassigned
# 2. Usually named with uppercase letters (convention)
```

## 3.8 Chapter Summary

| Concept | Understanding |
|---------|---------------|
| Variable | A name for a value |
| Assignment | The process of putting a value into a variable |
| Scope | Area where a variable can be used |
| Lifecycle | The process from value's birth to death |
| mut (mutable) | Variable can be reassigned |
| Field-level mut | Only fields marked with mut can be modified |
| Immutable | Variables whose values cannot be changed |
| Constants | Fixed values that don't change |

## 3.9 I Ching Introduction

> "Qian, when still, is focused; when moving, is direct, therefore great births arise.
> Kun, when still, is closed; when moving, is open, therefore vast births arise."
> — "Xici Zhuan", Book of Changes
>
> The Qian hexagram静时专一,动时刚直; the Kun hexagram静时收敛,动时开放.
> Variables are also like yin and yang — some are still and focused (immutable), some are dynamic and flexible (mutable).
> Knowing when to be "still" and when to "move" is key to writing good programs.
