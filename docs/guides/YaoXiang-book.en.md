# YaoXiang (爻象) Programming Language Guide

> Version: v1.1.0
> Status: Draft
> Author: Chen Xu
> Date: 2024-12-31
> Update: 2025-01-04 - Fixed generic syntax to `[T]`, removed `fn` keyword

---

## Table of Contents

1. [Language Overview](#一language-overview)
2. [Core Features](#二core-features)
3. [Type System](#三type-system)
4. [Memory Management](#四memory-management)
5. [Async Programming and Concurrency](#五async-programming-and-concurrency)
6. [Module System](#六module-system)
7. [Method Binding and Currying](#七method-binding-and-currying)
8. [AI-Friendly Design](#八ai-friendly-design)
9. [Type Centralization Convention](#九type-centralization-convention-core-design-philosophy)
10. [Quick Start](#十quick-start)

---

**Extended Documentation**:
- [Advanced Binding Features and Compiler Implementation](../works/plans/bind/YaoXiang-bind-advanced.md) - In-depth binding mechanisms, advanced features, compiler implementation, and edge case handling

---

## 一、Language Overview

### 1.1 What is YaoXiang?

YaoXiang (爻象) is an experimental general-purpose programming language whose design philosophy originates from the core concepts of "爻" (yao) and "象" (xiang) in the I Ching (Book of Changes). "爻" are the basic symbols that form hexagrams, symbolizing the interplay of yin and yang; "象" is the external manifestation of things' essence, representing all phenomena.

YaoXiang integrates this philosophical thinking into the type system of the programming language, proposing the core concept of **"Everything is a Type"**. In YaoXiang's worldview:

- **Values** are instances of types
- **Types** themselves are also instances of types (meta-types)
- **Functions** are mappings from input types to output types
- **Modules** are namespace combinations of types

### 1.2 Design Goals

YaoXiang's design goals can be summarized as follows:

| Goal | Description |
|------|-------------|
| **Unified Type Abstraction** | Types are the highest-level abstraction unit, simplifying language semantics |
| **Natural Programming Experience** | Python-style syntax, emphasizing readability |
| **Safe Memory Management** | Rust-style ownership model, no GC |
| **Seamless Async Programming** | Automatic async management, no explicit await |
| **Complete Type Reflection** | Runtime type information fully available |
| **AI-Friendly Syntax** | Strictly structured, easy for AI processing |

### 1.3 Language Positioning

| Dimension | Positioning |
|-----------|-------------|
| Paradigm | Multi-paradigm (Functional + Imperative + Object-Oriented) |
| Type System | Dependent Types + Parametric Polymorphism |
| Memory Management | Ownership + RAII (No GC) |
| Compilation Model | AOT Compilation (Optional JIT) |
| Target Scenarios | System Programming, Application Development, AI-Assisted Programming |

### 1.4 Code Example

```yaoxiang
# Automatic type inference
x: Int = 42                           # Explicit type
y = 42                                # Inferred as Int
name = "YaoXiang"                     # Inferred as String

# Default immutable
x: Int = 10
x = 20                                # ❌ Compile error! Immutable

# Unified declaration syntax: identifier: Type = expression
add: (Int, Int) -> Int = (a, b) => a + b  # Function declaration
inc: Int -> Int = x => x + 1               # Single parameter function

# Unified type syntax: constructor is type
type Point = Point(x: Float, y: Float)
type Result[T, E] = ok(T) | err(E)

# Seamless async (concurrent function)
fetch_data: (String) -> JSON spawn = (url) => {
    HTTP.get(url).json()
}

main: () -> Void = () => {
    # Value construction: exactly the same as function calls
    p = Point(3.0, 4.0)
    r = ok("success")

    data = fetch_data("https://api.example.com")
    # Automatic wait, no await needed
    print(data.name)
}

# Generic function
identity: [T](T) -> T = x => x

# Higher-order function
apply: ((Int) -> Int, Int) -> Int = (f, x) => f(x)

# Currying
add_curried: Int -> Int -> Int = a => b => a + b
```

---

## 二、Core Features

### 2.1 Everything is Type

YaoXiang's core design philosophy is **Everything is Type**. This means in YaoXiang:

1. **Values are instances of types**: `42` is an instance of `Int`
2. **Types are instances of types**: `Int` is an instance of the `type` meta-type
3. **Functions are type mappings**: `fn add(Int, Int) -> Int` is a function type
4. **Modules are type combinations**: Modules are namespaces containing functions and types

```yaoxiang
# Values are instances of types
x: Int = 42

# Types are instances of types
MyList: type = List(Int)

# Functions are mappings between types
add(Int, Int) -> Int = (a, b) => a + b

# Modules are combinations of types (using files as modules)
# Math.yx
pi: Float = 3.14159
sqrt(Float) -> Float = (x) => { ... }
```

### 2.2 Mathematical Abstraction

YaoXiang's type system is based on type theory and category theory, providing:

- **Dependent Types**: Types can depend on values
- **Generic Programming**: Type parameterization
- **Type Composition**: Union types, Intersection types

```yaoxiang
# Dependent Types: fixed-length vectors
type Vector[T, n: Nat] = vector(T, n)

# Type union
type Number = Int | Float

# Type intersection
type Printable = printable(fn() -> String)
type Serializable = serializable(fn() -> String)
type Versatile = Printable & Serializable
```

### 2.3 Zero-Cost Abstraction

YaoXiang guarantees zero-cost abstraction, meaning high-level abstractions do not introduce runtime performance overhead:

- **Monomorphization**: Generic functions are expanded to concrete versions at compile time
- **Inline Optimization**: Simple functions are automatically inlined
- **Stack Allocation**: Small objects are stack-allocated by default

```yaoxiang
# Generic expansion (monomorphization)
identity[T](T) -> T = (x) => x

# Usage
int_val = identity(42)      # Expands to identity(Int) -> Int
str_val = identity("hello") # Expands to identity(String) -> String

# No additional overhead after compilation
```

### 2.4 Natural Syntax

YaoXiang adopts Python-style syntax, pursuing readability and natural language feel:

```yaoxiang
# Automatic type inference
x = 42
name = "YaoXiang"

# Concise function definition
greet: String -> String = (name) => "Hello, " + name

# Pattern matching
classify: Int -> String = (n) => {
    match n {
        0 -> "zero"
        1 -> "one"
        _ if n < 0 -> "negative"
        _ -> "many"
    }
}
```

### 2.5 Complete Syntax Specification

YaoXiang adopts unified declaration syntax: **identifier: Type = expression**. Backward-compatible legacy syntax is also provided.

#### 2.5.1 Dual Syntax Strategy and Type Centralization Convention

To balance innovation and compatibility, YaoXiang supports two syntax forms but adopts a unified **type centralization convention**.

**Syntax Form Comparison:**

| Syntax Type | Format | Status | Description |
|-------------|--------|--------|-------------|
| **New Syntax (Standard)** | `name: Type = Lambda` | ✅ Recommended | Official standard, all new code should use this form |
| **Old Syntax (Compatible)** | `name(Types) -> Ret = Lambda` | ⚠️ Compatible Only | Preserved for historical code, not recommended for new projects |

**Core Convention: Type Centralization**

YaoXiang adopts the design convention of **"Declaration First, Type Centralization"**:

```yaoxiang
# ✅ Correct: Type information unified in declaration line
add: (Int, Int) -> Int = (a, b) => a + b
#   └─────────────────┘   └─────────────┘
#       Complete type signature         Implementation logic

# ❌ Avoid: Type information scattered in implementation
add = (a: Int, b: Int) => a + b
#     └───────────────┘
#     Types mixed in implementation
```

**Benefits of the Convention:**

1. **Syntax Consistency**: All declarations follow `identifier: Type = expression`
2. **Separation of Declaration and Implementation**: Type information at a glance, implementation focuses on logic
3. **AI-Friendly**: AI can understand complete function signature just from the declaration line
4. **Safer Modifications**: Modifying types only requires changing the declaration, not the implementation body
5. **Currying-Friendly**: Supports clear currying type signatures

**Selection Suggestions:**
- **New Projects**: Must use new syntax + type centralization convention
- **Migration Projects**: Gradually migrate to new syntax and type centralization convention
- **Maintaining Legacy Code**: Can continue using old syntax, but recommend adopting type centralization convention

#### 2.5.2 Basic Declaration Syntax

```yaoxiang
# === New Syntax (Recommended) ===
# All declarations follow: identifier: Type = expression

# Variable declaration
x: Int = 42
name: String = "YaoXiang"
mut counter: Int = 0

# Function declaration
add: (Int, Int) -> Int = (a, b) => a + b
inc: Int -> Int = x => x + 1
getAnswer: () -> Int = () => 42
log: (String) -> Void = msg => print(msg)

# === Old Syntax (Compatible) ===
# Only for functions, format: name(Types) -> Ret = Lambda
add(Int, Int) -> Int = (a, b) => a + b
square(Int) -> Int = (x) => x * x
empty() -> Void = () => {}
getRandom() -> Int = () => 42
```

#### 2.5.3 Function Type Syntax

```
Function Type ::= '(' Parameter Type List ')' '->' Return Type
                | Parameter Type '->' Return Type              # Single parameter shorthand

Parameter Type List ::= [Type (',' Type)*]
Return Type ::= Type | Function Type | 'Void'

# Function types are first-class citizens, can be nested
# Higher-order Function Type ::= '(' Function Type ')' '->' Return Type
```

| Example | Meaning |
|---------|---------|
| `Int -> Int` | Single parameter function type |
| `(Int, Int) -> Int` | Two parameter function type |
| `() -> Void` | No-parameter function type |
| `(Int -> Int) -> Int` | Higher-order function: receives function, returns Int |
| `Int -> Int -> Int` | Curried function (right-associative) |

#### 2.5.4 Generic Syntax (For Type Parameters Only)

```yaoxiang
# Generic function: <type parameter> prefix
identity: [T](T) -> T = x => x
map: [A, B]((A) -> B, List[A]) -> List[B] = (f, xs) => case xs of
  [] => []
  (x :: rest) => f(x) :: map(f, rest)

# Generic type
List: Type = [T] List[T]
```

#### 2.5.5 Lambda Expression Syntax

```
Lambda ::= '(' Parameter List ')' '=>' Expression
        | Parameter '=>' Expression              # Single parameter shorthand

Parameter List ::= [Parameter (',' Parameter)*]
Parameter ::= Identifier [':' Type]               # Optional type annotation
```

| Example | Meaning | Description |
|---------|---------|-------------|
| `(a, b) => a + b` | Multi-parameter Lambda | Use with declaration:<br>`add: (Int, Int) = (a, b) => a + b` |
| `x => x + 1` | Single parameter shorthand | Use with declaration:<br>`inc: Int = x => x + 1` |
| `(x: Int) => x + 1` | With type annotation | Only for temporary needs inside Lambda |
| `() => 42` | No-parameter Lambda | Use with declaration:<br>`get: () = () => 42` |

**Note**: Type annotations in Lambda expressions `(x: Int) => ...` are **temporary and local**, mainly used for:
- When type information is needed inside Lambda
- Used with declaration syntax (type already given in declaration)
- Should not be used as the main type declaration method

#### 2.5.6 Complete Examples

```yaoxiang
# === Basic Function Declaration ===

# Basic function (new syntax)
add: (Int, Int) -> Int = (a, b) => a + b

# Single parameter function (two forms)
inc: Int -> Int = x => x + 1
inc2: (Int) -> Int = (x) => x + 1

# No-parameter function
getAnswer: () -> Int = () => 42

# No return value function
log: (String) -> Void = msg => print(msg)

# === Recursive Functions ===
# Recursion naturally supported in lambda
fact: Int -> Int = (n) =>
  if n <= 1 then 1 else n * fact(n - 1)

# === Higher-Order Functions and Function Type Assignment ===

# Function types as first-class citizens
IntToInt: Type = Int -> Int
IntBinaryOp: Type = (Int, Int) -> Int

# Higher-order function declaration
applyTwice: (IntToInt, Int) -> Int = (f, x) => f(f(x))

# Curried function
addCurried: Int -> Int -> Int = a => b => a + b

# Function composition
compose: (Int -> Int, Int -> Int) -> Int -> Int =
  (f, g) => x => f(g(x))

# Function returning function
makeAdder: Int -> (Int -> Int) =
  x => y => x + y

# === Generic Functions ===

# Generic function
identity: [T](T) -> T = x => x

# Generic higher-order function
map: [A, B]((A) -> B, List[A]) -> List[B] =
  (f, xs) => case xs of
    [] => []
    (x :: rest) => f(x) :: map(f, rest)

# Generic function type
Transformer: Type = [A, B](A) -> B

# Using generic types
applyTransformer: [A, B](Transformer[A, B], A) -> B =
  (f, x) => f(x)

# === Complex Type Examples ===

# Nested function type
higherOrder: ((Int) -> Int) -> (Int) -> Int =
  f => x => f(x) + 1

# Multi-parameter higher-order function
zipWith: [A, B, C]((A, B) -> C, List[A], List[B]) -> List[C] =
  (f, xs, ys) => case (xs, ys) of
    ([], _) => []
    (_, []) => []
    (x::xs', y::ys') => f(x, y) :: zipWith(f, xs', ys')

# Function type alias
Predicate: Type = [T] (T) -> Bool
Mapper: Type = [A, B](A) -> B
Reducer: Type = [A, B](B, A) -> B

# === Old Syntax Examples (Backward Compatible Only) ===
# Not recommended for new code

mul(Int, Int) -> Int = (a, b) => a * b    # Multi-parameter
square(Int) -> Int = (x) => x * x          # Single parameter
empty() -> Void = () => {}                  # No-parameter
get_random() -> Int = () => 42              # With return value

# Equivalent new syntax (recommended)
mul: (Int, Int) -> Int = (a, b) => a * b
square: (Int) -> Int = (x) => x * x
empty: () -> Void = () => {}
get_random: () -> Int = () => 42
```

#### 2.5.7 Syntax Parsing Rules

**Type Parsing Priority:**

| Priority | Type | Description |
|----------|------|-------------|
| 1 (Highest) | Generic Application `List[T]` | Left-associative |
| 2 | Parentheses `(T)` | Changes associativity |
| 3 | Function Type `->` | Right-associative |
| 4 (Lowest) | Base Types `Int, String` | Atomic types |

**Type Parsing Examples:**

```yaoxiang
# (A -> B) -> C -> D
# Parsed as: ((A -> B) -> (C -> D))

# A -> B -> C
# Parsed as: (A -> (B -> C))  # Right-associative

# (Int -> Int) -> Int
# Parsed as: Takes function, returns Int -> Int

# List<Int -> Int>
# Parsed as: List with element type Int -> Int
```

**Lambda Parsing Examples:**

```yaoxiang
# a => b => a + b
# Parsed as: a => (b => (a + b))  # Right-associative, currying

# (a, b) => a + b
# Parsed as: Takes two parameters, returns a + b
```

#### 2.5.8 Type Inference Rules

YaoXiang adopts a **dual-layer processing** strategy: the parsing layer is lenient, the type checking layer is strict.

**Parsing Layer Rules:**
- Parser only validates syntax structure, does not perform type inference
- Declarations without type annotations have type annotation field as `None`
- All declarations matching basic syntax structure pass parsing
- **Key Point**: `add = (a, b) => a + b` is **legal** at the parsing layer

**Type Checking Layer Rules:**
- Validates semantic correctness, including type completeness
- **Parameters must have type annotations**: This is mandatory
- Return type can be inferred, but parameter types must be explicitly declared

**Complete Type Inference Rules:**

| Scenario | Parameter Inference | Return Inference | Parse Result | Type Check Result | Recommended Level |
|----------|---------------------|------------------|--------------|-------------------|-------------------|
| **Standard Function** | Uses annotated type | Uses annotated type | ✅ | ✅ | ⭐⭐⭐⭐⭐ |
| `add: (Int, Int) -> Int = (a, b) => a + b` | | | | | |
| **Partial Inference** | Uses annotated type | Inferred from expression | ✅ | ✅ | ⭐⭐⭐⭐ |
| `add: (Int, Int) = (a, b) => a + b` | | | | | |
| `inc: Int -> Int = x => x + 1` | | | | | |
| `get: () = () => 42` | | | | | |
| **Old Syntax Partial Inference** | Uses annotated type | Inferred from expression | ✅ | ✅ | ⭐⭐⭐ (Compatible) |
| `add(Int, Int) = (a, b) => a + b` | | | | | |
| `square(Int) = (x) => x * x` | | | | | |
| **Parameters Without Annotation** | **Cannot infer** | - | ✅ | ❌ Error | ❌ Forbidden |
| `add = (a, b) => a + b` | | | | | |
| `identity = x => x` | | | | | |
| **Block Without Return Annotation** | - | Inferred from block content | ✅ | ✅ | ⭐⭐⭐⭐ |
| `main = () => {}` | | | | | |
| `get = () => { return 42; }` | | | | | |
| **Block Without Return Annotation (No Explicit Return)** | - | Inferred as `Void` | ✅ | ✅ Not Recommended | ⚠️ Avoid |
| `bad = (x: Int) => { x }` | | | | | |

**Detailed Inference Rules:**

```
Parsing Layer: Only looks at syntax structure
├── Structure correct → Pass
└── Structure incorrect → Error

Type Checking Layer: Validates semantics
├── Parameter Type Inference
│   ├── Parameter has type annotation → Use annotated type ✅
│   ├── Parameter without type annotation → Reject ❌
│   └── Lambda parameters must be annotated → Mandatory requirement
│
├── Return Type Inference
│   ├── Has return expr → Inferred from expr ✅
│   ├── No return, has expression → Inferred from expression ✅
│   ├── No return, has block `{ ... }`
│   │   ├── Block is empty `{}` → Void ✅
│   │   ├── Block has return → Inferred from return ✅
│   │   └── Block no return and no explicit return → Inferred as Void ✅ (but not recommended)
│   └── Cannot infer → Reject ❌
│
└── Completely cannot infer → Reject ❌
```

**Note**: `bad = (x: Int) => { x }` can infer return type as `Void`, but it is strongly discouraged because:
- Code intent is unclear
- Easy to cause misunderstanding
- Does not conform to functional programming expression style

**Inference Examples:**

```yaoxiang
# === Inference Success ===

# Standard form
main: () -> Void = () => {}                    # Complete annotation
num: () -> Int = () => 42                      # Complete annotation
inc: Int -> Int = x => x + 1                   # Single parameter shorthand

# Partial inference (new syntax)
add: (Int, Int) = (a, b) => a + b              # Parameters annotated, return inferred
square: Int -> Int = x => x * x                # Parameters annotated, return inferred
get_answer: () = () => 42                      # Parameters annotated (empty), return inferred

# Partial inference (old syntax, compatible)
add2(Int, Int) = (a, b) => a + b               # Parameters annotated, return inferred
square2(Int) = (x) => x * x                    # Parameters annotated, return inferred

# Inferred from return
fact: Int -> Int = (n) => {
    if n <= 1 { return 1 }
    return n * fact(n - 1)
}

# === Inference Failure ===

# Parameters cannot be inferred (passes parsing, fails type checking)
add = (a, b) => a + b                          # ✗ Parameters without type
identity = x => x                              # ✗ Parameters without type

# Block without explicit return
no_return = (x: Int) => { x }                  # ✗ Block has no return, cannot infer implicit return

# Completely cannot infer
bad_fn = x => x                                # ✗ Both parameters and return cannot be inferred
```

#### 2.5.9 Old Syntax (Backward Compatibility)

YaoXiang provides old syntax support for compatibility with historical code, **not recommended for new code**.

```
Old Syntax ::= Identifier '(' [Parameter Type List] ')' '->' Return Type '=' Lambda
```

| Feature | Standard Syntax | Old Syntax |
|---------|-----------------|------------|
| Declaration Format | `name: Type = ...` | `name(Types) -> Type = ...` |
| Parameter Type Location | In type annotation | In parentheses after function name |
| Empty Parameters | Must write `()` | Can omit `()` |
| **Recommendation Level** | ✅ **Officially Recommended** | ⚠️ **Backward Compatible Only** |
| **Usage Scenario** | All new code | Legacy code maintenance |

**Reasons for Not Recommending:**
1. **Learning Cost**: Inconsistent with standard syntax, increases language complexity
2. **Consistency**: Parameter type locations are inconsistent (one in type annotation, one after function name)
3. **Maintenance Cost**: Parser needs extra handling for both forms
4. **AI-Friendly**: Increases difficulty for AI to understand and generate code

**Migration Suggestions:**
```yaoxiang
# Old code (not recommended)
mul(Int, Int) -> Int = (a, b) => a * b
square(Int) -> Int = (x) => x * x
empty() -> Void = () => {}

# New code (recommended)
mul: (Int, Int) -> Int = (a, b) => a * b
square: (Int) -> Int = (x) => x * x
empty: () -> Void = () => {}
```

---

## 三、Type System

### 3.1 Type Hierarchy

YaoXiang's type system is hierarchical:

```
┌─────────────────────────────────────────────────────────────┐
│                    YaoXiang Type Hierarchy                  │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  type (meta-type)                                           │
│    │                                                        │
│    ├── Primitive Types                                      │
│    │   ├── Void, Bool                                       │
│    │   ├── Int, Uint, Float                                 │
│    │   ├── Char, String, Bytes                              │
│    │                                                        │
│    ├── Constructor Types                                    │
│    │   ├── Name(args)              # Single constructor (struct)  │
│    │   ├── A(T) | B(U)             # Multiple constructors (union/enum)   │
│    │   ├── A | B | C               # Zero-parameter constructors (enum)   │
│    │   ├── tuple (T1, T2, ...)                            │
│    │   ├── list [T], dict [K->V]                           │
│    │                                                        │
│    ├── Function Types                                       │
│    │   fn (T1, T2, ...) -> R                               │
│    │                                                        │
│    ├── Generic Types                                        │
│    │   List[T], Map[K, V], etc.                            │
│    │                                                        │
│    ├── Dependent Types                                      │
│    │   type [n: Nat] -> type                               │
│    │                                                        │
│    └── Module Types                                         │
│        File as module                                       │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 Type Definition

```yaoxiang
# Unified type syntax: only constructors, no enum/struct/union keywords
# Rule: Separated by | are constructors, constructor_name(parameters) is the type

# === Zero-parameter constructors (enum style) ===
type Color = red | green | blue              # Equivalent to red() | green() | blue()

# === Multi-parameter constructors (struct style) ===
type Point = Point(x: Float, y: Float)       # Constructor is the type

# === Generic constructors ===
type Result[T, E] = ok(T) | err(E)           # Generic union

# === Mixed constructors ===
type Shape = circle(Float) | rect(Float, Float)

# === Value construction (exactly the same as function calls) ===
c: Color = green                              # Equivalent to green()
p: Point = Point(1.0, 2.0)
r: Result[Int, String] = ok(42)
s: Shape = circle(5.0)
```

### 3.3 Type Operations

```yaoxiang
# Types as values
MyInt = Int
MyList = List(Int)

# Type reflection (constructor pattern matching)
describe_type(type) -> String = (t) => {
    match t {
        Point(x, y) -> "Point with x=" + x + ", y=" + y
        red -> "Red color"
        ok(value) -> "Ok value"
        _ -> "Other type"
    }
}
```

### 3.4 Type Inference

YaoXiang has powerful type inference capabilities:

```yaoxiang
# Basic inference
x = 42                    # Inferred as Int
y = 3.14                  # Inferred as Float
z = "hello"               # Inferred as String

# Function return type inference
add: (Int, Int) -> Int = (a, b) => a + b

# Generic inference
first: [T](List[T]) -> Option[T] = (list) => {
    if list.length > 0 { some(list[0]) } else { none }
}
```

---

## 四、Memory Management

### 4.1 Ownership Core Principles

YaoXiang's memory management is based on **Ownership Model**, ensuring memory safety without garbage collection. The core design is simple:

- **Default: Move (Zero-Copy)** - Ownership transfer on assignment/parameter/return
- **Explicit: `ref` = Arc** - Safe shared ownership with reference counting
- **Explicit: `clone()`** - Value copying when original must be preserved
- **System-Level: `unsafe` + `*T`** - Raw pointers for system programming

```yaoxiang
# === Default: Move (Zero-Copy) ===
p: Point = Point(1.0, 2.0)
p2 = p              # Move, p becomes invalid

# === Explicit: ref = Arc (Thread-Safe Sharing) ===
shared = ref p      # Arc, reference counting, thread-safe

spawn(() => print(shared.x))   # ✅ Safe, Arc is Send + Sync

# === Explicit: clone() (Value Copy) ===
p3 = p.clone()      # Full copy, p and p3 are independent

# === System-Level: unsafe + raw pointer ===
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0  # User guarantees safety
}
```

### 4.2 Move Semantics (Default)

**Rule**: Assignment, parameter passing, and return = Move (ownership transfer).

```yaoxiang
# === Assignment = Move ===
p: Point = Point(1.0, 2.0)
p2 = p              # Move, p is now invalid
# print(p.x)        # ❌ Error: p no longer owns the value

# === Parameter = Move ===
process(p: Point) -> Point = (p) => {
    p.transform()   # p is the parameter, Move
}

main: () -> Void = () => {
    p = Point(1.0, 2.0)
    result = process(p)
    # p is moved into process(), cannot use p here
}

# === Return = Move ===
create_point: () -> Point = () => {
    p = Point(1.0, 2.0)
    return p        # Move, p's ownership transferred to caller
}
```

**Characteristics**:
- Zero-copy (only pointer movement)
- Original owner becomes invalid
- RAII automatic cleanup when scope ends

### 4.3 ref = Arc (Explicit Shared Ownership)

**Rule**: Use `ref` keyword to create an Arc (atomic reference counting) for safe sharing.

```yaoxiang
p: Point = Point(1.0, 2.0)

# Create Arc
shared: ref Point = ref p

# Arc automatically manages lifetime
spawn(() => print(shared.x))   # ✅ Safe
spawn(() => print(shared.y))   # ✅ Safe

# Arc refcount automatically increments/decrements
# shared goes out of scope → refcount reaches zero → auto release
```

**Characteristics**:
- Thread-safe atomic reference counting
- Automatic lifetime management
- User explicitly controls sharing timing

### 4.4 clone() (Explicit Copy)

**Rule**: Use `clone()` when you need to keep the original value.

```yaoxiang
p: Point = Point(1.0, 2.0)

# Copy the value
p2 = p.clone()

# Both p and p2 are independent owners
p.x = 0.0      # ✅ OK
p2.x = 0.0     # ✅ OK, p2 is independent

# Small objects are copied efficiently (compiler optimization)
```

**When to use**:
- Need to preserve the original value
- Move is not suitable for the scenario
- Small objects (compiler optimizes copying)

### 4.5 unsafe and Raw Pointers

**Rule**: Use `unsafe` blocks for system-level programming with raw pointers. User guarantees safety.

```yaoxiang
p: Point = Point(1.0, 2.0)

# System-level programming
unsafe {
    # Get raw pointer
    ptr: *Point = &p

    # Dereference (user guarantees validity)
    (*ptr).x = 0.0

    # Pointer arithmetic
    ptr2 = ptr + 1
}

# Outside unsafe, raw pointers cannot exist
```

**Restrictions**:
- Can only be used inside `unsafe` blocks
- User must guarantee no dangling pointers, no use-after-free
- Used for system-level programming (FFI, memory operations)

### 4.6 RAII (Resource Acquisition Is Initialization)

```yaoxiang
# RAII automatic release
with_file: (String) -> String = (path) => {
    file = File.open(path)  # Auto open
    content = file.read_all()
    # Function ends, file auto closes
    content
}

# Custom resource cleanup
type Connection = Connection(handle: Handle)

# Drop called automatically when Connection goes out of scope
```

### 4.7 Send/Sync Constraints

```yaoxiang
# Basic types automatically satisfy Send + Sync
# Int, Float, Bool, Point, ...

# ref[T] automatically satisfies Send + Sync (Arc is thread-safe)
p: Point = Point(1.0, 2.0)
shared = ref p                      # Arc, thread-safe

spawn(() => print(shared.x))        # ✅ OK

# Raw pointer *T does not satisfy Send + Sync
unsafe {
    ptr: *Point = &p                 # Can only be used in single thread
}
```

### 4.9 No Lifetimes, No Borrow Checker

YaoXiang eliminates complex lifetime annotations and borrow checking:

- **No `&T` reference concept** - No need for `&` syntax
- **No `'a` lifetime annotations** - Eliminated because no references
- **No borrow checker** - Replaced by `ref` = Arc mechanism

```yaoxiang
# === No lifetime needed ===
# Rust problem:
# fn returns_ref(&Point) -> &Point { ... }  # Needs 'a annotation

# YaoXiang solution:
create_point: () -> Point = () => {
    p = Point(1.0, 2.0)
    return p                    # Move, ownership transfer, no lifetime needed
}

# === No borrow checker needed ===
# Use ref for sharing, Arc handles all lifetime management
p: Point = Point(1.0, 2.0)
shared = ref p                  # Arc, automatic lifetime management
```

**Programming Burden**: ⭐☆☆☆☆ (Almost Zero)
**Performance Guarantee**: Near Rust, no GC pauses
```

---

## 五、Async Programming and Concurrency

> "万物并作，吾以观复。" — Yi Hexagram (Book of Changes)
>
> YaoXiang adopts **Concurrent Model**, a seamless async concurrency paradigm based on **lazy evaluation**. The core design concept is: **Let developers describe logic with synchronous, sequential thinking, while the language runtime automatically and efficiently executes computation units concurrently like all things working together, and finally unifies them.**
>
> See [Concurrent Model Whitepaper](YaoXiang-async-whitepaper.md) and [Async Implementation Plan](YaoXiang-async-implementation.md).

### 5.1 Core Concepts of Concurrent Model

#### 5.1.1 Concurrent Graph: Stage for All Things Working Together

All programs are transformed into a **Directed Acyclic Computation Graph (DAG)** at compile time, called **Concurrent Graph**. Nodes represent expression computations, edges represent data dependencies. This graph is lazy, meaning nodes are only evaluated when their output is **truly needed**.

```yaoxiang
# Compiler automatically builds concurrent graph
fetch_user() -> User spawn = (id) => { ... }
fetch_posts(User) -> Posts spawn = (user) => { ... }

main() -> Void = () => {
    user = fetch_user(1)     # Node A (Async[User])
    posts = fetch_posts(user) # Node B (Async[Posts]), depends on A

    # Node C needs results from A and B
    print(posts.title)       # Auto wait: first ensure A and B complete
}
```

#### 5.1.2 Concurrent Values: Async[T]

Any function call marked with `spawn fn` immediately returns a value of type `Async[T]`, called **Concurrent Value**. This is a lightweight proxy that is not the actual result, but represents a **future value being processed concurrently**.

**Core Features:**
- **Type Transparent**: `Async[T]` is a subtype of `T` in the type system, can be used in any context expecting `T`
- **Auto Wait**: When program execution reaches an operation that must use a concrete value of type `T`, the runtime automatically suspends the current task and waits for computation to complete
- **Zero Contagion**: Async code has no syntactic or type signature differences from sync code

```yaoxiang
# Concurrent value usage example
fetch_data(String) -> JSON spawn = (url) => { ... }

main() -> Void = () => {
    data = fetch_data("url")  # Async[JSON]

    # Async[JSON] can be directly used as JSON
    # Auto wait happens at field access
    print(data.name)          # Equivalent to data.await().name
}
```

### 5.2 Concurrent Syntax System

The `spawn` keyword has三重 semantics, the only bridge connecting synchronous thinking with async implementation:

| Official Term | Syntax Form | Semantics | Runtime Behavior |
|---------------|-------------|-----------|------------------|
| **Concurrent Function** | `spawn fn` | Defines computation unit that can participate in concurrent execution | Its call returns `Async[T]` |
| **Concurrent Block** | `spawn { a(), b() }` | Explicitly declared concurrent domain | Tasks inside block forced to execute in parallel |
| **Concurrent Loop** | `spawn for x in xs { ... }` | Data parallel paradigm | Loop body executes concurrently on all elements |

#### 5.2.1 Concurrent Function

```yaoxiang
# Use spawn to mark concurrent function
# Syntax exactly the same as normal functions, no extra burden

fetch_api(String) -> JSON spawn = (url) => {
    response = HTTP.get(url)
    JSON.parse(response.body)
}

# Nested concurrent calls
process_user(Int) -> Report spawn = (user_id) => {
    user = fetch_user(user_id)     # Async[User]
    profile = fetch_profile(user)  # Async[Profile], depends on user
    generate_report(user, profile) # Depends on profile
}
```

#### 5.2.2 Concurrent Block

```yaoxiang
# spawn { } - Explicit parallel construct
# All expressions in the block execute concurrently as independent tasks

compute_all(Int, Int) -> (Int, Int, Int) spawn = (a, b) => {
    # Three independent calculations execute in parallel
    (x, y, z) = spawn {
        heavy_calc(a),        # Task 1
        heavy_calc(b),        # Task 2
        another_calc(a, b)    # Task 3
    }
    (x, y, z)
}
```

#### 5.2.3 Concurrent Loop

```yaoxiang
# spawn for - Data parallel loop
# Each iteration executes as independent task in parallel

parallel_sum(Int) -> Int spawn = (n) => {
    total = spawn for i in 0..n {
        fibonacci(i)          # Each iteration parallel
    }
    total
}
```

#### 5.2.4 Data Parallel Loop

```yaoxiang
# spawn for - Data parallel loop
# Each iteration executes as independent task in parallel

parallel_sum(Int) -> Int spawn = (n) => {
    total = spawn for i in 0..n {
        fibonacci(i)          # Each iteration parallel
    }
    total
}

# Matrix multiplication parallelization
matmul[[A: Matrix], [B: Matrix]] -> Matrix spawn = (A, B) => {
    result = spawn for i in 0..A.rows {
        row = spawn for j in 0..B.cols {
            dot_product(A.row(i), B.col(j))
        }
        row
    }
    result
}
```

### 5.3 Auto-Wait Mechanism

```yaoxiang
# No explicit await needed, compiler automatically inserts wait points

main() -> Void = () => {
    # Auto parallel: two independent requests execute in parallel
    users = fetch_users()      # Async[List[User]]
    posts = fetch_posts()      # Async[List[Post]]

    # Wait point automatically inserted at "+" operation
    count = users.length + posts.length

    # Field access triggers wait
    first_user = users[0]      # Wait for users to be ready
    print(first_user.name)
}

# Wait in conditional branches
process_data() -> Void spawn = () => {
    data = fetch_data()        # Async[Data]

    if data.is_valid {         # Wait for data to be ready
        process(data)
    } else {
        log("Invalid data")
    }
}
```

### 5.4 Concurrency Control Tools

```yaoxiang
# Wait for all tasks to complete
await_all[List[T]](List[Async[T]]) -> List[T] = (tasks) => {
    # Barrier wait
}

# Wait for any one to complete
await_any[List[T]](List[Async[T]]) -> T = (tasks) => {
    # Return first completed result
}

# Timeout control
with_timeout[T](Async[T], Duration) -> Option[T] = (task, timeout) => {
    # Return None on timeout
}
```

### 5.5 Thread Safety: Send/Sync Constraints

YaoXiang adopts **Send/Sync Type Constraints** similar to Rust to ensure thread safety, eliminating data races at compile time.

#### 5.5.1 Send Constraint

**Send**: Type can safely **transfer ownership** across threads.

```yaoxiang
# Basic types automatically satisfy Send
# Int, Float, Bool, String are all Send

# Structs automatically derive Send
type Point = Point(x: Int, y: Float)
# Point is Send because Int and Float are both Send

# Types containing non-Send fields are not Send
type NonSend = NonSend(data: Rc[Int])
# Rc is not Send (reference counting non-atomic), so NonSend is not Send
```

#### 5.5.2 Sync Constraint

**Sync**: Type can safely **share references** across threads.

```yaoxiang
# Basic types are all Sync
type Point = Point(x: Int, y: Float)
# &Point is Sync because &Int and &Float are both Sync

# Types with internal mutability
type Counter = Counter(value: Int, mutex: Mutex[Int])
# &Counter is Sync because Mutex provides internal mutability
```

#### 5.5.3 spawn and Thread Safety

```yaoxiang
# spawn requires parameters and return values to satisfy Send

# Valid: Data is Send
type Data = Data(value: Int)
task = spawn(|| => Data(42))

# Invalid: Rc is not Send
type SharedData = SharedData(rc: Rc[Int])
# task = spawn(|| => SharedData(Rc.new(42))  # Compile error!

# Solution: Use Arc (atomic reference counting)
type SafeData = SafeData(value: Arc[Int])
task = spawn(|| => SafeData(Arc.new(42)))  # Arc is Send + Sync
```

#### 5.5.4 Thread Safety Type Derivation Rules

```yaoxiang
# Struct type
type Struct[T1, T2] = Struct(f1: T1, f2: T2)

# Send derivation
Struct[T1, T2]: Send ⇐ T1: Send and T2: Send

# Sync derivation
Struct[T1, T2]: Sync ⇐ T1: Sync and T2: Sync

# Union type
type Result[T, E] = ok(T) | err(E)

# Send derivation
Result[T, E]: Send ⇐ T: Send and E: Send
```

#### 5.5.5 Standard Library Thread Safety Implementation

| Type | Send | Sync | Description |
|------|:----:|:----:|-------------|
| `Int`, `Float`, `Bool` | ✅ | ✅ | Primitive types |
| `Arc[T]` | ✅ | ✅ | T: Send + Sync |
| `Mutex[T]` | ✅ | ✅ | T: Send |
| `RwLock[T]` | ✅ | ✅ | T: Send |
| `Channel[T]` | ✅ | ❌ | Send only for sending end |
| `Rc[T]` | ❌ | ❌ | Non-atomic reference counting |
| `RefCell[T]` | ❌ | ❌ | Runtime borrow checking |


```yaoxiang
# Thread-safe counter example
type SafeCounter = SafeCounter(mutex: Mutex[Int])

main() -> Void = () => {
    counter: Arc[SafeCounter] = Arc.new(SafeCounter(Mutex.new(0)))

    # Concurrent updates
    spawn(|| => {
        guard = counter.mutex.lock()  # Mutex provides thread safety
        guard.value = guard.value + 1
    })

    spawn(|| => {
        guard = counter.mutex.lock()
        guard.value = guard.value + 1
    })
}
```

### 5.6 Blocking Operations

```yaoxiang
# Use @block annotation to mark operations that block OS threads
# Runtime will assign them to dedicated block thread pool

@block
read_large_file(String) -> String = (path) => {
    # This call will not block the core scheduler
    file = File.open(path)
    content = file.read_all()
    content
}
```

---

## 六、Module System

### 6.1 Module Definition

```yaoxiang
# Modules use files as boundaries
# Math.yx file
pub pi: Float = 3.14159
pub sqrt(Float) -> Float = (x) => { ... }
```

### 6.2 Module Import

```yaoxiang
# Import entire module
use std.io

# Import and rename
use std.io as IO

# Import specific functions
use std.io.{ read_file, write_file }
```

---

## 七、Method Binding and Currying

YaoXiang adopts **pure functional design**, achieving seamless method calling and currying through advanced binding mechanisms, without introducing `struct`, `class` and other keywords.

### 7.1 Core Function Definition

All operations are implemented through ordinary functions, with the first parameter conventionally being the subject of the operation:

```yaoxiang
# === Point.yx (module) ===

# Unified syntax: constructor is type
type Point = Point(x: Float, y: Float)

# Core function: first parameter is the subject of operation
distance(Point, Point) -> Float = (a, b) => {
    dx = a.x - b.x
    dy = a.y - b.y
    (dx * dx + dy * dy).sqrt()
}

add(Point, Point) -> Point = (a, b) => {
    Point(a.x + b.x, a.y + b.y)
}

scale(Point, Float) -> Point = (p, s) => {
    Point(p.x * s, p.y * s)
}

# More complex functions
distance_with_scale(scale: Float, a: Point, b: Point) -> Float = (s, p1, p2) => {
    dx = (p1.x - p2.x) * s
    dy = (p1.y - p2.y) * s
    (dx * dx + dy * dy).sqrt()
}
```

### 7.2 Basic Method Binding

#### 7.2.1 Auto Binding (MoonBit Style)

YaoXiang supports namespace-based automatic binding, **without any additional declarations**:

```yaoxiang
# === Point.yx ===

type Point = Point(x: Float, y: Float)

# Core function
distance(Point, Point) -> Float = (a, b) => { ... }

# === main.yx ===

use Point

main() -> Void = () => {
    p1 = Point(3.0, 4.0)
    p2 = Point(1.0, 2.0)

    # ✅ Auto binding: direct method call
    result = p1.distance(p2)  # Resolved as distance(p1, p2)
}
```

**Auto Binding Rules:**
- Functions defined within a module
- If the first parameter type matches the module name
- Then automatically supports method call syntax

#### 7.2.2 No-Binding Option (Default Behavior)

```yaoxiang
# === Vector.yx ===

type Vector = Vector(x: Float, y: Float, z: Float)

# Internal helper function, do not want auto binding
dot_product_internal(v1: Vector, v2: Vector) -> Float = (a, b) => {
    a.x * b.x + a.y * b.y + a.z * b.z
}

# === main.yx ===

use Vector

main() -> Void = () => {
    v1 = Vector(1.0, 0.0, 0.0)
    v2 = Vector(0.0, 1.0, 0.0)

    # ❌ Cannot bind: non-pub functions do not auto bind
    # v1.dot_product_internal(v2)  # Compile error!

    # ✅ Must call directly (invisible outside module)
}
```

### 7.3 Position-Based Binding Syntax

YaoXiang provides **the most elegant binding syntax**, using position marker `[n]` to precisely control binding position:

#### 7.3.1 Basic Syntax

```yaoxiang
# === Point.yx ===

type Point = Point(x: Float, y: Float)

# Core function
distance(Point, Point) -> Float = (a, b) => { ... }
add(Point, Point) -> Point = (a, b) => { ... }
scale(Point, Float) -> Point = (p, s) => { ... }

# Binding syntax: Type.method = func[position]
# Means: when calling method, bind caller to func's [position] parameter

Point.distance = distance[1]      # Bind to 1st parameter
Point.add = add[1]                 # Bind to 1st parameter
Point.scale = scale[1]             # Bind to 1st parameter
```

**Semantic Parsing:**
- `Point.distance = distance[1]`
  - `distance` function has two parameters: `distance(Point, Point)`
  - `[1]` means caller binds to 1st parameter
  - Usage: `p1.distance(p2)` → `distance(p1, p2)`

#### 7.3.2 Multi-Position Joint Binding

```yaoxiang
# === Math.yx ===

# Function: scale, point1, point2, extra1, extra2
calculate(scale: Float, a: Point, b: Point, x: Float, y: Float) -> Float = (s, p1, p2, x, y) => { ... }

# === Point.yx ===

type Point = Point(x: Float, y: Float)

# Bind multiple positions
Point.calc1 = calculate[1, 2]      # Bind scale and point1
Point.calc2 = calculate[1, 3]      # Bind scale and point2
Point.calc3 = calculate[2, 3]      # Bind point1 and point2

# === main.yx ===

use Point

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

# 1. Bind [1,2] - remaining 3,4,5
f1 = p1.calc1(2.0)  # Bind scale=2.0, point1=p1
# f1 now needs p2, x, y
result1 = f1(p2, 10.0, 20.0)  # calculate(2.0, p1, p2, 10.0, 20.0)

# 2. Bind [1,3] - remaining 2,4,5
f2 = p2.calc2(2.0)  # Bind scale=2.0, point2=p2
# f2 now needs point1, x, y
result2 = f2(p1, 10.0, 20.0)  # calculate(2.0, p1, p2, 10.0, 20.0)

# 3. Bind [2,3] - remaining 1,4,5
f3 = p1.calc3(p2)  # Bind point1=p1, point2=p2
# f3 now needs scale, x, y
result3 = f3(2.0, 10.0, 20.0)  # calculate(2.0, p1, p2, 10.0, 20.0)
```

#### 7.3.3 Remaining Parameter Fill Order

**Core Rule**: After binding, remaining parameters are filled in **original function order**, skipping already bound positions.

```yaoxiang
# Suppose function: func(p1, p2, p3, p4, p5)

# Bind 1st and 3rd parameters
Type.method = func[1, 3]

# When calling:
method(p2_value, p4_value, p5_value)

# Maps to:
func(p1_bound, p2_value, p3_bound, p4_value, p5_value)
# Remaining parameters: 2,4,5 filled in original order
```

#### 7.3.4 Type Checking Advantages

```yaoxiang
# ✅ Legal binding
Point.distance = distance[1]          # distance(Point, Point)
Point.calc = calculate[1, 2]          # calculate(scale, Point, Point, ...)

# ❌ Illegal binding (compiler error)
Point.wrong = distance[5]             # 5th parameter does not exist
Point.wrong = distance[0]             # Parameters start from 1
Point.wrong = distance[1, 2, 3, 4]    # Exceeds function parameter count
```

### 7.4 Fine-Grained Control of Curried Binding

```yaoxiang
# === Math.yx ===

distance_with_scale(scale: Float, a: Point, b: Point) -> Float = (s, p1, p2) => { ... }

# === Point.yx ===

type Point = Point(x: Float, y: Float)

# Binding strategy: flexibly control each position
Point.distance = distance[1]                    # Basic binding
Point.distance_scaled = distance_with_scale[2]  # Bind to 2nd parameter

# === main.yx ===

use Point
use Math

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

# 1. Basic auto binding
d1 = p1.distance(p2)  # distance(p1, p2)

# 2. Bind to different position
f = p1.distance_scaled(2.0)  # Bind 2nd parameter, remaining 1st, 3rd
result = f(p2)               # distance_with_scale(2.0, p1, p2)

# 3. Chained binding
d2 = p1.distance(p2).distance_scaled(2.0)  # Chained call
```

### 7.5 Complete Binding System

```yaoxiang
# === Point.yx ===

type Point = Point(x: Float, y: Float)

# Core functions
distance(Point, Point) -> Float = (a, b) => { ... }
add(Point, Point) -> Point = (a, b) => { ... }
scale(Point, Float) -> Point = (p, s) => { ... }

# Auto binding (core)
Point.distance = distance[1]
Point.add = add[1]
Point.scale = scale[1]

# === Math.yx ===

# Global functions
multiply_by_scale(scale: Float, a: Point, b: Point) -> Float = (s, p1, p2) => { ... }

# === main.yx ===

use Point
use Math

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

# Usage
d = p1.distance(p2)          # distance(p1, p2)
r = p1.add(p2)               # add(p1, p2)
s = p1.scale(2.0)            # scale(p1, 2.0)

# Global function binding
Point.multiply = multiply_by_scale[2]  # Bind to 2nd parameter
m = p1.multiply(2.0, p2)     # multiply_by_scale(2.0, p1, p2)
```

### 7.6 Binding Scope and Rules

#### 7.6.1 Effect of pub

```yaoxiang
# === Point.yx ===

type Point = Point(x: Float, y: Float)

# Non-pub functions
internal_distance(a: Point, b: Point) -> Float = (a, b) => { ... }

# pub functions
pub distance(a: Point, b: Point) -> Float = (a, b) => { ... }

# === main.yx ===

use Point

# Auto binding only works for pub functions
p1.distance(p2)      # ✅ distance is pub, can auto bind
# p1.internal_distance(p2)  # ❌ Not pub, cannot bind
```

#### 7.6.2 Binding Within Module

```yaoxiang
# === Point.yx ===

type Point = Point(x: Float, y: Float)

distance(Point, Point) -> Float = (a, b) => { ... }

# Within module, all functions are visible
# But auto binding only works for pub exported functions outside

pub distance  # Export, auto binding available outside
```

### 7.7 Design Advantage Summary

| Feature | Description |
|---------|-------------|
| **Zero Syntax Burden** | Auto binding without any declaration |
| **Precise Position Control** | `[n]` precisely specifies binding position |
| **Multi-Position Joint** | Supports `[1, 2, 3]` multi-parameter binding |
| **Type Safety** | Compiler validates binding position validity |
| **No Keywords** | No `bind` or other keywords |
| **Flexible Currying** | Supports arbitrary position parameter binding |
| **pub Control** | Only pub functions can bind externally |

### 7.8 Difference from Traditional Method Binding

| Traditional Language | YaoXiang |
|---------|----------|
| `obj.method(arg)` | `obj.method(arg)` |
| Need class/method definition | Only function + binding declaration |
| Syntax `class { method() {} }` | Syntax `Type.method = func[n]` |
| Inheritance, polymorphism | Pure functional, no inheritance |
| Method table lookup | Compile-time binding, no runtime overhead |

**Core Advantage**: YaoXiang's binding is a **compile-time mechanism**, zero runtime cost, while maintaining the purity and flexibility of functional programming.

---

## 八、AI-Friendly Design

YaoXiang's syntax design especially considers AI code generation and modification needs:

### 8.1 Design Principles

```yaoxiang
# AI-friendly design goals:
# 1. Strictly structured, unambiguous syntax
# 2. Clear AST, easy to locate
# 3. Clear semantics, no hidden behavior
# 4. Clear code block boundaries
# 5. Complete type information
```

### 8.2 Strictly Structured Syntax

#### 8.2.1 AI-Friendly Declaration Syntax Strategy

```yaoxiang
# === AI Code Generation Best Practices ===

# ✅ Recommended: Use complete new syntax declaration + type centralization convention
# AI can accurately understand intent, generate complete type information

add: (Int, Int) -> Int = (a, b) => a + b
inc: Int -> Int = x => x + 1
empty: () -> Void = () => {}

# ❌ Avoid: Omitting type annotations or scattered types
# AI cannot determine parameter types, may generate wrong code
add = (a, b) => a + b          # Parameters without type
identity = x => x              # Parameters without type
add2 = (a: Int, b: Int) => a + b  # Types scattered in implementation

# ⚠️ Compatible: Old syntax only for maintenance
# AI should prioritize generating new syntax + type centralization convention
mul(Int, Int) -> Int = (a, b) => a * b  # Not recommended in new code
```

**AI Advantages of Type Centralization Convention:**

1. **Signature at a Glance**: AI can understand complete function signature just from declaration line
2. **Safer Modifications**: Modifying types only requires changing declaration, not implementation body
3. **Simpler Generation**: AI can first generate declaration, then fill implementation
4. **Currying-Friendly**: Clear currying type signatures easy for AI to process

```yaoxiang
# AI Processing Example
# Input: Implementation body (a, b) => a + b
# AI sees declaration: add: (Int, Int) -> Int
# Conclusion: Parameter types are Int, Int, return type is Int

# Contrast: If types are scattered
# Input: Implementation body (a: Int, b: Int) => a + b
# AI needs: Analyze implementation body to extract type information
# Result: More complex processing logic, error-prone
```

#### 8.2.2 Dual Syntax Strategy and AI

| Syntax Type | AI Generation Strategy | Usage Scenario |
|-----------|-----------|---------|
| **New Syntax** | ✅ Prioritize generation, complete type information | All new code generation |
| **Old Syntax** | ⚠️ Only when maintaining old code | Historical code modification |
| **No Annotation** | ❌ Avoid generating | Should never generate in any situation |

#### 8.2.3 Clear Syntax Boundaries

```yaoxiang
# AI-friendly code block boundaries

# ✅ Clear start and end markers
function_name: (Type1, Type2) -> ReturnType = (param1, param2) => {
    # Function body
    if condition {
        do_something()
    } else {
        do_other()
    }
}

# ✅ Conditionals must have braces
if condition {
    # Condition body
}

# ✅ Type definitions are clear
type MyType = Type1 | Type2

# ❌ Avoid ambiguous写法
if condition    # Missing braces
    do_something()
```

#### 8.2.4 Unambiguous Syntax Constraints

```yaoxiang
# Constraints that must be followed when generating AI

# 1. Prohibit omitting parentheses
# ✅ Correct
foo: (T) -> T = (x) => x
my_list = [1, 2, 3]

# ❌ Error (prohibited)
foo T { T }             # Parameters must have parentheses
my_list = [1 2 3]       # Lists must have commas

# 2. Must explicitly return type or inferrable form
# ✅ Correct
get_num: () -> Int = () => 42
get_num2: () = () => 42          # Return type inferrable

# ❌ Error
get_bad = () => { 42 }           # Block has no return, cannot infer

# 3. Parameters must have type annotation (new syntax)
# ✅ Correct
add: (Int, Int) -> Int = (a, b) => a + b
inc: Int -> Int = x => x + 1

# ❌ Error
add = (a, b) => a + b            # Parameters without type
identity = x => x                # Parameters without type
```

#### 8.2.5 AI Generation Recommended Patterns

```yaoxiang
# Standard template when AI generates functions

# Pattern1: Complete type annotation
function_name: (ParamType1, ParamType2, ...) -> ReturnType = (param1, param2, ...) => {
    # Function body
    return expression
}

# Pattern2: Return type inference
function_name: (ParamType1, ParamType2) = (param1, param2) => {
    # Function body
    return expression
}

# Pattern3: Single parameter shorthand
function_name: ParamType -> ReturnType = param => expression

# Pattern4: No-parameter function
function_name: () -> ReturnType = () => expression

# Pattern5: Empty function
function_name: () -> Void = () => {}
```

### 8.3 Error Message AI-Friendly

```yaoxiang
# Error messages should provide clear correction suggestions

# Unfriendly error
# Syntax error at token 'a'

# AI-friendly error
# Missing type annotation for parameter 'a'
# Suggestion: add ': Int' or similar type to '(a, b) => a + b'
# Correct version: add: (Int, Int) -> Int = (a, b) => a + b
```

---

## 九、Type Centralization Convention (Core Design Philosophy)

### 9.1 Convention Overview

YaoXiang's core design convention is **"Declaration First, Type Centralization"**. This convention is the cornerstone of language AI-friendliness and development efficiency.

```yaoxiang
# ✅ Core convention: Type information unified in declaration line
add: (Int, Int) -> Int = (a, b) => a + b

# ❌ Avoid: Type information scattered in implementation
add = (a: Int, b: Int) => a + b
```

### 9.2 Five Core Advantages of the Convention

#### 1. Syntax Consistency
```yaoxiang
# All declarations follow same format
x: Int = 42                           # Variable
name: String = "YaoXiang"             # Variable
add: (Int, Int) -> Int = (a, b) => a + b  # Function
inc: Int -> Int = x => x + 1          # Function
type Point = Point(x: Float, y: Float) # Type
```

#### 2. Separation of Declaration and Implementation
```yaoxiang
# Declaration line provides complete type information
add: (Int, Int) -> Int = (a, b) => a + b
# └────────────────────┘
#   Complete function signature

# Implementation focuses on business logic
# (a, b) => a + b doesn't need to care about types, just implement functionality
```

#### 3. AI-Friendly
```yaoxiang
# AI processing flow:
# 1. Read declaration line → Completely understand function signature
# 2. Generate implementation → No need to analyze type inference
# 3. Modify type → Only change declaration line, not affect implementation

# Contrast: Scattered type approach
add = (a: Int, b: Int) => a + b
# AI needs: Analyze implementation body to extract type information → More complex, error-prone
```

#### 4. Safer Modifications
```yaoxiang
# Modify parameter type
# Original: add: (Int, Int) -> Int = (a, b) => a + b
# Modified: add: (Float, Float) -> Float = (a, b) => a + b
# Implementation: (a, b) => a + b  No modification needed!

# If types are scattered:
# Original: add = (a: Int, b: Int) => a + b
# Modified: add = (a: Float, b: Float) => a + b  # Need to change two places
```

#### 5. Currying-Friendly
```yaoxiang
# Currying type is clear at a glance
add_curried: Int -> Int -> Int = a => b => a + b
#              └─────────────┘
#              Currying signature

# Function composition as first-class citizen
compose: (Int -> Int, Int -> Int) -> Int -> Int = (f, g) => x => f(g(x))
```

### 9.3 Convention Implementation Rules

#### Rule1: Parameters must have type specified in declaration
```yaoxiang
# ✅ Correct
add: (Int, Int) -> Int = (a, b) => a + b

# ❌ Error
add = (a, b) => a + b            # Parameter types missing
identity = x => x                # Parameter types missing
```

#### Rule2: Return type can be inferred but recommended to annotate
```yaoxiang
# ✅ Recommended: Complete annotation
get_num: () -> Int = () => 42

# ✅ Acceptable: Return type inferred
get_num: () = () => 42

# ✅ Empty function inferred as Void
empty: () = () => {}
```

#### Rule3: Lambda internal type annotations are temporary
```yaoxiang
# ✅ Correct: Depends on types in declaration
add: (Int, Int) -> Int = (a, b) => a + b

# ⚠️ Acceptable but not recommended: Duplicate annotation in Lambda
add: (Int, Int) -> Int = (a: Int, b: Int) => a + b

# ❌ Error: Missing declaration annotation
add = (a: Int, b: Int) => a + b
```

#### Rule4: Old syntax follows same philosophy
```yaoxiang
# Old syntax should also provide type information as much as possible at declaration position
# Although format is different, philosophy is consistent:
# - Declaration line contains main type information
# - Implementation body is relatively concise
add(Int, Int) -> Int = (a, b) => a + b
```

### 9.4 Relationship Between Convention and Type Inference

```yaoxiang
# Convention does not prevent type inference, but guides inference direction

# 1. Complete annotation (no inference)
add: (Int, Int) -> Int = (a, b) => a + b

# 2. Partial inference (declaration provides parameter types)
add: (Int, Int) = (a, b) => a + b  # Return type inferred

# 3. Empty function inference
empty: () = () => {}  # Inferred as () -> Void
```

### 9.5 AI Implementation Advantages of Convention

**AI Code Generation Flow:**

1. **Read requirements** → Generate declaration
   ```
   Requirement: addition function
   Generate: add: (Int, Int) -> Int = (a, b) => ???
   ```

2. **Fill implementation** → No type analysis needed
   ```
   Implementation: add: (Int, Int) -> Int = (a, b) => a + b
   ```

3. **Type modification** → Only change declaration
   ```
   Modify: add: (Float, Float) -> Float = (a, b) => a + b
   Implementation: (a, b) => a + b  Remains unchanged
   ```

**Contrast with no convention AI processing:**
```
Requirement: addition function
AI needs:
  1. Infer parameter types
  2. Infer return type
  3. Generate implementation body
  4. Validate consistency
  5. Handle complex updates when types change

Result: More complex, more error-prone
```

### 9.6 Philosophical Significance of Convention

This convention embodies YaoXiang's core philosophy:

- **Declaration is Documentation**: Declaration line is complete function documentation
- **Type is Contract**: Type information is the contract between caller and implementer
- **Logic is Implementation**: Implementation body only focuses on "what to do", not "what type"
- **Tools are Assistance**: Type system, AI tools can all work based on clear declarations

### 9.7 Practical Application Comparison

#### Complete Example: Calculator Module

```yaoxiang
# === Recommended: Type Centralization Convention ===

# Module declarations
pub add: (Int, Int) -> Int = (a, b) => a + b
pub multiply: (Int, Int) -> Int = (a, b) => a * b

# Higher-order functions
pub apply_twice: (Int -> Int, Int) -> Int = (f, x) => f(f(x))

# Curried functions
pub make_adder: Int -> (Int -> Int) = x => y => x + y

# Generic functions
pub map: [A, B]((A) -> B, List[A]) -> List[B] = (f, xs) => case xs of
  [] => []
  (x :: rest) => f(x) :: map(f, rest)

# Type definitions
type Point = Point(x: Float, y: Float)
pub distance: (Point, Point) -> Float = (a, b) => {
    dx = a.x - b.x
    dy = a.y - b.y
    (dx * dx + dy * dy).sqrt()
}

# === Not Recommended: Scattered Types ===

# Parameter types in Lambda
add = (a: Int, b: Int) => a + b
multiply = (a: Int, b: Int) => a * b

# Higher-order function types scattered
apply_twice = (f: (Int) -> Int, x: Int) => f(f(x))

# Currying types scattered
make_adder = (x: Int) => (y: Int) => x + y

# Generic types scattered
map = [A, B](f: (A) -> B, xs: List[A]) => List[B] => case xs of
  [] => []
  (x :: rest) => f(x) :: map(f, rest)
```

#### Code Maintenance Comparison

```yaoxiang
# Requirement: Change add from Int to Float

# === Recommended: Only need to change declaration line ===
# Original
add: (Int, Int) -> Int = (a, b) => a + b

# Modified
add: (Float, Float) -> Float = (a, b) => a + b
#              ↑↑↑↑↑↑↑↑↑          ↑↑↑↑↑↑↑
#              Declaration line modified          Implementation body unchanged

# === Not Recommended: Need to change multiple places ===
# Original
add = (a: Int, b: Int) => a + b

# Modified
add = (a: Float, b: Float) => a + b
#     ↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑
#     All parameter types need modification
```

#### AI-Assisted Programming Comparison

```yaoxiang
# AI Requirement: Implement a function calculating Manhattan distance between two points

# === AI sees recommended写法 ===
type Point = Point(x: Float, y: Float)
pub manhattan: (Point, Point) -> Float = ???  # AI directly knows complete signature

# AI generates:
pub manhattan: (Point, Point) -> Float = (a, b) => {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}

# === AI sees not recommended写法 ===
type Point = Point(x: Float, y: Float)
pub manhattan = ???  # AI needs to infer: parameter types? return type?

# AI may generate:
pub manhattan = (a: Point, b: Point) => Float => {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}
# Or may make mistakes because type information is incomplete
```

### 9.8 Convention Implementation Checklist

When writing YaoXiang code, use the following checklist:

- [ ] All function declarations have complete type annotations in declaration line
- [ ] Parameter types specified in declaration, not in Lambda
- [ ] Return types annotated in declaration as much as possible
- [ ] Variable declarations use `name: Type = value` format
- [ ] Lambda body stays concise, not duplicating type information
- [ ] Use new syntax instead of old syntax
- [ ] Complex types use type definition, keep declaration clear

---

## 十、Quick Start

### 10.1 Hello World

```yaoxiang
# hello.yx
use std.io

main: () -> Void = () => {
    println("Hello, YaoXiang!")
}
```

Run: `yaoxiang hello.yx`

Output:
```
Hello, YaoXiang!
```

### 10.2 Basic Syntax

```yaoxiang
# Variables and types
x = 42                    # Auto inferred as Int
name = "YaoXiang"         # Auto inferred as String
pi = 3.14159              # Auto inferred as Float

# Functions (using new syntax)
add: (Int, Int) -> Int = (a, b) => a + b

# Conditionals
if x > 0 {
    "positive"
} elif x == 0 {
    "zero"
} else {
    "negative"
}

# Loops
for i in 0..10 {
    print(i)
}
```

### 10.3 Method Binding Example

```yaoxiang
# === Point.yx ===

type Point = Point(x: Float, y: Float)

# Core function
distance(Point, Point) -> Float = (a, b) => {
    dx = a.x - b.x
    dy = a.y - b.y
    (dx * dx + dy * dy).sqrt()
}

# Auto binding
Point.distance = distance[1]

# === main.yx ===

use Point

main() -> Void = () => {
    p1 = Point(3.0, 4.0)
    p2 = Point(1.0, 2.0)

    # Use binding
    d = p1.distance(p2)  # distance(p1, p2)
    print(d)
}
```

### 10.4 Curried Binding Example

```yaoxiang
# === Math.yx ===

distance_with_scale(scale: Float, a: Point, b: Point) -> Float = (s, p1, p2) => {
    dx = (p1.x - p2.x) * s
    dy = (p1.y - p2.y) * s
    (dx * dx + dy * dy).sqrt()
}

# === Point.yx ===

type Point = Point(x: Float, y: Float)

Point.distance_scaled = distance_with_scale[2]  # Bind to 2nd parameter

# === main.yx ===

use Point

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

# Use binding
f = p1.distance_scaled(2.0)  # Bind scale and p1
result = f(p2)               # Final call

# Or use directly
result2 = p1.distance_scaled(2.0, p2)
```

### 10.5 Next Steps

- Read [Language Specification](./YaoXiang-language-specification.md) for complete syntax
- View [Example Code](./examples/) to learn common patterns
- Reference [Implementation Plan](./YaoXiang-implementation.md) for technical details

---

## Appendix

### A. Keywords and Annotations

| Keyword | Purpose |
|---------|---------|
| `type` | Type definition |
| `pub` | Public export |
| `use` | Import module |
| `spawn` | Async marker (function/block/loop) |
| `ref` | Arc (atomic reference counting) for safe sharing |
| `mut` | Mutable binding |
| `if/elif/else` | Conditional branches |
| `match` | Pattern matching |
| `while/for` | Loops |
| `return/break/continue` | Control flow |
| `as` | Type casting |
| `in` | Member access |
| `unsafe` | System-level code block with raw pointers |

| Annotation | Purpose |
|------------|---------|
| `@block` | Mark blocking operation, assign to blocking thread pool |
| `@eager` | Mark expression needing eager evaluation |
| `@Send` | Explicitly declare satisfying Send constraint |
| `@Sync` | Explicitly declare satisfying Sync constraint |

### B. Design Inspirations

- **Rust**: Ownership model, zero-cost abstraction
- **Python**: Syntax style, readability
- **Idris/Agda**: Dependent types, type-driven development
- **TypeScript**: Type annotations, runtime types

---

## Version History

| Version | Date | Author | Change Description |
|---------|------|--------|-------------------|
| v1.0.0 | 2024-12-31 | Chen Xu | Initial version |
| v1.1.0 | 2025-01-04 | Moyu | Fixed generic syntax to `[T]` (not `<T>`); removed `fn` keyword; updated function definition examples |

---

> "道生一，一生二，二生三，三生万物。"
> —— 《道德经》
>
> Types are like the Way, all things are born from them.
