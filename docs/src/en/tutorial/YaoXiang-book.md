# YaoXiang Programming Language Guide

> Version: v1.2.0
> Status: Draft
> Author: Chen Xu
> Date: 2024-12-31
> Updated: 2025-01-20 - Position indices start from 0 (RFC-004); Unified type syntax (RFC-010)

---

## Table of Contents

1. [Language Overview](#1-language-overview)
2. [Core Features](#2-core-features)
3. [Type System](#3-type-system)
4. [Memory Management](#4-memory-management)
5. [Asynchronous Programming and Concurrency](#5-asynchronous-programming-and-concurrency)
6. [Module System](#6-module-system)
7. [Method Binding and Currying](#7-method-binding-and-currying)
8. [AI-Friendly Design](#8-ai-friendly-design)
9. [Type-Centralized Convention (Core Design Philosophy)](#9-type-centralized-convention-core-design-philosophy)
10. [Quick Start](#10-quick-start)

---

**Extended Documentation**:
- [Advanced Binding Features and Compiler Implementation](../works/plans/bind/YaoXiang-bind-advanced.md) - In-depth binding mechanisms, advanced features, compiler implementation, and edge case handling

---

## 1. Language Overview

### 1.1 What is YaoXiang?

YaoXiang is an experimental general-purpose programming language whose design philosophy originates from the core concepts of "Yao" (爻) and "Xiang" (象) in the I Ching (Book of Changes). "Yao" is the fundamental symbol that forms hexagrams, symbolizing the interplay of yin and yang; "Xiang" is the external manifestation of the essence of things, representing all phenomena in the universe.

YaoXiang integrates this philosophical thinking into the type system of a programming language, proposing the core idea that **"everything is a type"**. In YaoXiang's worldview:

- **Values** are instances of types
- **Types** themselves are also instances of types (meta types)
- **Functions** are mappings from input types to output types
- **Modules** are combinations of type namespaces

### 1.2 Design Goals

The design goals of YaoXiang can be summarized as follows:

| Goal | Description |
|------|-------------|
| **Unified type abstraction** | Types are the highest-level abstraction unit, simplifying language semantics |
| **Natural programming experience** | Python-style syntax, emphasizing readability |
| **Safe memory management** | Rust-style ownership model, no GC |
| **Seamless asynchronous programming** | Automatic async management, no explicit await |
| **Complete type reflection** | Runtime type information fully available |
| **AI-friendly syntax** | Strictly structured, easy for AI to process |

### 1.3 Language Positioning

| Dimension | Positioning |
|-----------|-------------|
| Paradigm | Multi-paradigm (functional + imperative + object-oriented) |
| Type system | Dependent types + parametric polymorphism |
| Memory management | Ownership + RAII (no GC) |
| Compilation model | AOT compilation (optional JIT) |
| Target scenarios | Systems programming, application development, AI-assisted programming |

### 1.4 Code Examples

```yaoxiang
# Automatic type inference
x: Int = 42                           # Explicit type
y = 42                                # Inferred as Int
name = "YaoXiang"                     # Inferred as String

# Default immutable
x: Int = 10
x = 20                                # ❌ Compile error! Immutable

# Unified declaration syntax: identifier: type = expression
add: (a: Int, b: Int) -> Int = a + b  # Function declaration
inc: (x: Int) -> Int = x + 1               # Single parameter function

# Unified type syntax: constructors are types
type Point = { x: Float, y: Float }
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

# Seamless async (spawn function)
fetch_data: (url: String) -> JSON spawn = {
    HTTP.get(url).json()
}

main: () -> Void = {
    # Value construction: identical to function calls
    p = Point(3.0, 4.0)
    r = ok("success")

    data = fetch_data("https://api.example.com")
    # Automatic wait, no await needed
    print(data.name)
}

# Generic function
identity: (T: Type) -> ((x: T) -> T) = x

# Higher-order function
apply: (f: (Int) -> Int, x: Int) -> Int = f(x)

# Currying
add_curried: (a: Int) -> (b: Int) -> Int = a + b
```

---

## 2. Core Features

### 2.1 Everything is a Type

The core design philosophy of YaoXiang is **everything is a type**. This means in YaoXiang:

1. **Values are instances of types**: `42` is an instance of the `Int` type
2. **Types are instances of types**: `Int` is an instance of the `type` meta type
3. **Functions are type mappings**: `add: (Int, Int) -> Int` is a function type
4. **Modules are type combinations**: Modules are namespaces containing functions and types

```yaoxiang
# Values are instances of types
x: Int = 42

# Types are instances of types
MyList: type = List(Int)

# Functions are mappings between types
add: (a: Int, b: Int) -> Int = a + b

# Modules are combinations of types (using files as modules)
# Math.yx
pi: Float = 3.14159
sqrt: (x: Float) -> Float = { ... }
```

### 2.2 Mathematical Abstraction

YaoXiang's type system is based on type theory and category theory, providing:

- **Dependent types**: Types can depend on values
- **Generic programming**: Type parameterization
- **Type composition**: Union types, intersection types

```yaoxiang
# Dependent type: fixed-length vector
Vector: (T: Type, n: Int) -> Type = vector(T, n)

# Type union
type Number = Int | Float

# Type intersection
type Printable = printable(fn() -> String)
type Serializable = serializable(fn() -> String)
type Versatile = Printable & Serializable
```

### 2.3 Zero-Cost Abstraction

YaoXiang guarantees zero-cost abstraction, meaning high-level abstractions do not incur runtime performance overhead:

- **Monomorphization**: Generic functions are expanded to concrete versions at compile time
- **Inlining optimization**: Simple functions are automatically inlined
- **Stack allocation**: Small objects are allocated on the stack by default

```yaoxiang
# Generic expansion (monomorphization)
identity: (T: Type) -> ((x: T) -> T) = x

# Usage
int_val = identity(42)      # Expanded to identity(Int) -> Int
str_val = identity("hello") # Expanded to identity(String) -> String

# No additional overhead after compilation
```

### 2.4 Natural Syntax

YaoXiang adopts Python-style syntax, pursuing readability and natural language feel:

```yaoxiang
# Automatic type inference
x = 42
name = "YaoXiang"

# Concise function definition
greet: (name: String) -> String = "Hello, " + name

# Pattern matching
classify: (n: Int) -> String = {
    match n {
        0 -> "zero"
        1 -> "one"
        _ if n < 0 -> "negative"
        _ -> "many"
    }
}
```

### 2.5 Complete Syntax Specification

YaoXiang adopts a unified declaration syntax: **identifier: type = expression**. It also provides backward-compatible old syntax.

#### 2.5.1 Dual Syntax Strategy and Type-Centralized Convention

To balance innovation and compatibility, YaoXiang supports two syntax forms but adopts a unified **type-centralized annotation convention**.

**Syntax form comparison:**

| Syntax Type | Format | Status | Description |
|-------------|--------|--------|-------------|
| **New syntax (standard)** | `name: Type = Lambda` | ✅ Recommended | Official standard, all new code should use this form |
| **Old syntax (compatible)** | `name(Types) -> Ret = Lambda` | ⚠️ Compatibility only | Retained for historical code, not recommended for new projects |

**Core convention: Type-Centralized Annotation**

YaoXiang adopts the design convention of **"declaration first, types centralized"**:

```yaoxiang
# ✅ Correct: Type information unified on the declaration line
add: (a: Int, b: Int) -> Int = a + b
#   └─────────────────┘   └─────────────┘
#       Complete type signature         Implementation logic

# ❌ Avoid: Type information scattered in the implementation
add: (a: Int, b: Int) -> Int = a + b
#     └───────────────┘
#     Types mixed in the implementation body
```

**Benefits of the convention:**

1. **Syntax consistency**: All declarations follow `identifier: type = expression`
2. **Separation of declaration and implementation**: Type information is clear at a glance, implementation body focuses on logic
3. **AI friendliness**: AI only needs to read the declaration line to understand the complete function signature
4. **Safer modifications**: Modifying types only requires changing the declaration, not affecting the implementation body
5. **Currying friendly**: Supports clear curried type signatures

**Selection advice**:
- **New projects**: Must use new syntax + type-centralized convention
- **Migrating projects**: Gradually migrate to new syntax and type-centralized convention
- **Maintaining old code**: Can continue using old syntax, but recommended to adopt type-centralized convention

#### 2.5.2 Basic Declaration Syntax

```yaoxiang
# === New syntax (recommended) ===
# All declarations follow: identifier: type = expression

# Variable declaration
x: Int = 42
name: String = "YaoXiang"
mut counter: Int = 0

# Function declaration
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
getAnswer: () -> Int = 42
log: (msg: String) -> Void = print(msg)

# === Old syntax (compatible) ===
# Only used for functions, format: name(Types) -> Ret = Lambda
add(Int, Int) -> Int = (a, b) => a + b
square(Int) -> Int = (x) => x * x
empty() -> Void = () => {}
getRandom() -> Int = () => 42
```

#### 2.5.3 Function Type Syntax

```
FunctionType ::= '(' ParameterTypeList ')' '->' ReturnType
               | ParameterType '->' ReturnType              # Single parameter shorthand

ParameterTypeList ::= [Type (',' Type)*]
ReturnType ::= Type | FunctionType | 'Void'

# Function types are first-class citizens, can be nested
# HigherOrderFunctionType ::= '(' FunctionType ')' '->' ReturnType
```

| Example | Meaning |
|---------|---------|
| `Int -> Int` | Single parameter function type |
| `(Int, Int) -> Int` | Two parameter function type |
| `() -> Void` | No-parameter function type |
| `(Int -> Int) -> Int` | Higher-order function: accepts function, returns Int |
| `Int -> Int -> Int` | Curried function (right-associative) |

#### 2.5.4 Generic Syntax (only for type parameters)

```yaoxiang
# Generic function: <type parameter> prefix
identity: (T: Type) -> ((x: T) -> T) = x
map: (A: Type, B: Type) -> ((f: (A) -> B, xs: List(A)) -> List(B)) = case xs of
  [] => []
  (x :: rest) => f(x) :: map(f, rest)

# Generic type
List: (T: Type) -> Type
```

#### 2.5.5 Lambda Expression Syntax

```
Lambda ::= '(' ParameterList ')' '=>' Expression
         | Parameter '=>' Expression              # Single parameter shorthand

ParameterList ::= [Parameter (',' Parameter)*]
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
- When used with declaration syntax (types already given in declaration)
- Should not be used as the main type declaration method

#### 2.5.6 Complete Examples

```yaoxiang
# === Basic Function Declarations ===

# Basic function (new syntax)
add: (a: Int, b: Int) -> Int = a + b

# Single parameter function (two forms)
inc: (x: Int) -> Int = x + 1
inc2: (x: Int) -> Int = x + 1

# No-parameter function
getAnswer: () -> Int = 42

# No return value function
log: (msg: String) -> Void = print(msg)

# === Recursive Functions ===
# Recursion naturally supported in lambda
fact: (n: Int) -> Int =
  if n <= 1 then 1 else n * fact(n - 1)

# === Higher-Order Functions and Function Type Assignment ===

# Function types as first-class citizens
IntToInt: Type = (Int) -> Int
IntBinaryOp: Type = (Int, Int) -> Int

# Higher-order function declaration
applyTwice: (f: IntToInt, x: Int) -> Int = f(f(x))

# Curried function
addCurried: (a: Int) -> (b: Int) -> Int = a + b

# Function composition
compose: (f: Int -> Int, g: Int -> Int) -> (x: Int) -> Int =
  f(g(x))

# Function that returns a function
makeAdder: (x: Int) -> (y: Int) -> Int =
  x + y

# === Generic Functions ===

# Generic function
identity: (T: Type) -> ((x: T) -> T) = x

# Generic higher-order function
map: (A: Type, B: Type) -> ((f: (A) -> B, xs: List(A)) -> List(B)) =
  case xs of
    [] => []
    (x :: rest) => f(x) :: map(f, rest)

# Generic function type
Transformer: Type = (A: Type, B: Type) -> ((A) -> B)

# Using generic types
applyTransformer: (A: Type, B: Type) -> ((f: Transformer(A, B), x: A) -> B) =
  f(x)

# === Complex Type Examples ===

# Nested function types
higherOrder: (A: Type) -> ((f: (A) -> Int) -> (A) -> Int) =
  f => x => f(x) + 1

# Multi-parameter higher-order function
zipWith: (A: Type, B: Type, C: Type) -> ((f: (A, B) -> C, xs: List(A), ys: List(B)) -> List(C)) =
  case (xs, ys) of
    ([], _) => []
    (_, []) => []
    (x::xs', y::ys') => f(x, y) :: zipWith(f, xs', ys')

# Function type aliases
Predicate: (T: Type) -> Type = { apply: (T) -> Bool }
Mapper: Type = (A: Type, B: Type) -> ((A) -> B)
Reducer: Type = (A: Type, B: Type) -> ((B, A) -> B)

# === Old Syntax Examples (backward compatibility only) ===
# Not recommended for new code

mul(Int, Int) -> Int = (a, b) => a * b    # Multi-parameter
square(Int) -> Int = (x) => x * x          # Single parameter
empty() -> Void = () => {}                  # No-parameter
get_random() -> Int = () => 42              # With return value

# Equivalent new syntax (recommended)
mul: (a: Int, b: Int) -> Int = a * b
square: (x: Int) -> Int = x * x
empty: () -> Void = {}
get_random: () -> Int = 42
```

#### 2.5.7 Syntax Parsing Rules

**Type parsing precedence:**

| Precedence | Type | Description |
|------------|------|-------------|
| 1 (highest) | Generic application `List(T)` | Left-associative |
| 2 | Parentheses `(T)` | Changes associativity |
| 3 | Function type `->` | Right-associative |
| 4 (lowest) | Base types `Int, String` | Atomic types |

**Type parsing examples:**

```yaoxiang
# (A -> B) -> C -> D
# Parsed as: ((A -> B) -> (C -> D))

# A -> B -> C
# Parsed as: (A -> (B -> C))  # Right-associative

# (Int -> Int) -> Int
# Parsed as: accepts function, returns Int -> Int

# List<Int -> Int>
# Parsed as: List's element type is Int -> Int
```

**Lambda parsing examples:**

```yaoxiang
# a => b => a + b
# Parsed as: a => (b => (a + b))  # Right-associative, curried

# (a, b) => a + b
# Parsed as: accepts two parameters, returns a + b
```

#### 2.5.8 Type Inference Rules

YaoXiang adopts a **two-layer processing** strategy: the parser is lenient, the type checker is strict.

**Parser layer rules:**
- The parser only validates syntax structure, does not perform type inference
- Declarations missing type annotations have `None` for type annotation field
- All declarations conforming to basic syntax structure pass parsing
- **Key point**: `add: (a: Int, b: Int) -> Int = a + b` is **valid** at the parser layer

**Type checker layer rules:**
- Validates semantic correctness, including type completeness
- **Parameters must have type annotations**: This is mandatory
- Return types can be inferred, but parameter types must be explicitly declared

**Complete type inference rules:**

| Scenario | Parameter inference | Return inference | Parser result | Type checker result | Recommended level |
|----------|---------------------|------------------|---------------|---------------------|-------------------|
| **Standard function** | Use annotated types | Use annotated types | ✅ | ✅ | ⭐⭐⭐⭐⭐ |
| `add: (a: Int, b: Int) -> Int = a + b` | | | | | |
| **Partial inference** | Use annotated types | Inferred from expression | ✅ | ✅ | ⭐⭐⭐⭐ |
| `add: (Int, Int) = (a, b) => a + b` | | | | | |
| `inc: (x: Int) -> Int = x + 1` | | | | | |
| `get: () = () => 42` | | | | | |
| **Old syntax partial inference** | Use annotated types | Inferred from expression | ✅ | ✅ | ⭐⭐⭐ (compatibility) |
| `add(Int, Int) = (a, b) => a + b` | | | | | |
| `square(Int) = (x) => x * x` | | | | | |
| **Parameters without annotation** | **Cannot infer** | - | ✅ | ❌ Error | ❌ Forbidden |
| `add: (a: Int, b: Int) -> Int = a + b` | | | | | |
| `identity: (T: Type) -> ((x: T) -> T) = x` | | | | | |
| **Block without return annotation** | - | Inferred from block content | ✅ | ✅ | ⭐⭐⭐⭐ |
| `main = () => {}` | | | | | |
| `get = () => { return 42; }` | | | | | |
| **Code block without return** | - | Defaults to `Void` | ✅ | ✅ Correct | ✅ Correct |
| `add: (a: Int, b: Int) -> Int = { return a + b }` | | | | | |

**Detailed inference rules:**

```
Parser layer: only checks syntax structure
├── Correct structure → Pass
└── Incorrect structure → Error

Type checker layer: validates semantics
├── Parameter type inference
│   ├── Parameter has type annotation → Use annotated type ✅
│   ├── Parameter without type annotation → Reject ❌
│   └── Lambda parameters must be annotated → Mandatory requirement
│
├── Return type inference
│   ├── Has return expr → Inferred from expr ✅
│   ├── No return, has expression → Inferred from expression ✅
│   ├── Code block without return → Defaults to Void ✅
│   └── Cannot infer → Reject ❌
│
└── Completely cannot infer → Reject ❌
```

**Note**: When there's no `return` in a code block, it defaults to returning `Void`. For example:
- `() => { 42 }` infers to `() -> Void` (block has no return, defaults to Void)
- `() => { return 42 }` infers to `() -> Int` (has return, inferred from return)
- `() => 42` infers to `() -> Int` (expression form, directly returns value)

**Inference examples:**

```yaoxiang
# === Inference success ===

# Standard form
main: () -> Void = () => {}                    # Complete annotation
num: () -> Int = () => 42                      # Complete annotation
inc: (x: Int) -> Int = x + 1                   # Single parameter shorthand

# Partial inference (new syntax)
add: (Int, Int) = (a, b) => a + b              # Parameters annotated, return inferred
square: (x: Int) -> Int = x * x                # Parameters annotated, return inferred
get_answer: () = () => 42                      # Parameters annotated (empty), return inferred

# Partial inference (old syntax, compatibility)
add2(Int, Int) = (a, b) => a + b               # Parameters annotated, return inferred
square2(Int) = (x) => x * x                    # Parameters annotated, return inferred

# Inferred from return
fact: Int -> Int = (n) => {
    if n <= 1 { return 1 }
    return n * fact(n - 1)
}

# === Inference failure ===

# Parameters cannot be inferred (passes parsing, fails type checking)
add: (a: Int, b: Int) -> Int = a + b                          # ✗ Parameters without type
identity: (T: Type) -> ((x: T) -> T) = x                              # ✗ Parameters without type

# Code block without return
no_return = (x: Int) => { x }                  # ✓ Inferred as Void (block without return defaults to Void)

# Completely cannot infer
bad_fn: (T: Type) -> ((x: T) -> T) = x                                # ✗ Cannot infer parameters or return
```

#### 2.5.9 Old Syntax (Backward Compatibility)

YaoXiang provides old syntax support for backward compatibility with historical code, **not recommended for use in new code**.

```
OldSyntax ::= Identifier '(' [ParameterTypeList] ')' '->' ReturnType '=' Lambda
```

| Feature | Standard syntax | Old syntax |
|---------|----------------|------------|
| Declaration format | `name: Type = ...` | `name(Types) -> Type = ...` |
| Parameter type location | In type annotation | After function name in parentheses |
| Empty parameters | Must write `()` | Can omit `()` |
| **Recommended level** | ✅ **Official recommendation** | ⚠️ **Backward compatibility only** |
| **Use case** | All new code | Historical code maintenance |

**Reasons for not recommending:**
1. **Learning cost**: Inconsistent with standard syntax, increases language complexity
2. **Consistency**: Parameter type location is inconsistent (one in type annotation, one after function name)
3. **Maintenance cost**: Parser needs extra handling for both forms
4. **AI friendliness**: Increases difficulty for AI to understand and generate code

**Migration suggestions:**
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

## 3. Type System

### 3.1 Type Hierarchy

YaoXiang's type system is hierarchical:

```
┌─────────────────────────────────────────────────────────────┐
│                    YaoXiang Type Hierarchy                  │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  type (meta type)                                           │
│    │                                                        │
│    ├── Primitive Types                                     │
│    │   ├── Void, Bool                                       │
│    │   ├── Int, Uint, Float                                 │
│    │   ├── Char, String, Bytes                              │
│    │                                                        │
│    ├── Constructor Types                                    │
│    │   ├── Name(args)              # Single constructor (struct)      │
│    │   ├── A(T) | B(U)             # Multiple constructors (union/enum)   │
│    │   ├── A | B | C               # Zero-parameter constructors (enum)      │
│    │   ├── tuple (T1, T2, ...)                            │
│    │   ├── list(T), dict(K, V)                           │
│    │                                                        │
│    ├── Function Types                                      │
│    │   fn (T1, T2, ...) -> R                               │
│    │                                                        │
│    ├── Generic Types                                       │
│    │   List(T), Map(K, V), etc.                            │
│    │                                                        │
│    ├── Dependent Types                                     │
│    │   (n: Int) -> Type                               │
│    │                                                        │
│    └── Module Types                                        │
│        Files as modules                                            │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 Type Definitions

```yaoxiang
# Unified type syntax: only constructors, no enum/struct/union keywords
# Rule: Everything separated by | is a constructor, constructor name(args) is a type

# === Zero-parameter constructors (enum style)===
type Color = { red | green | blue }              # Equivalent to red() | green() | blue()

# === Multi-parameter constructors (struct style)===
type Point = { x: Float, y: Float }       # Constructor is the type

# === Generic constructors ===
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }           # Generic union

# === Mixed constructors ===
type Shape = circle(Float) | rect(Float, Float)

# === Value construction (identical to function calls)===
c: Color = green                              # Equivalent to green()
p: Point = Point(1.0, 2.0)
r: Result(Int, String) = ok(42)
s: Shape = circle(5.0)

# === Interface definition (record type with all function fields)===
type Drawable = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

type Serializable = {
    serialize: () -> String
}

# === Interface implementation (list interface names at end of type)===
type Point = {
    x: Float,
    y: Float,
    Drawable,        # Implements Drawable interface
    Serializable     # Implements Serializable interface
}
```

### 3.3 Type Operations

```yaoxiang
# Types as values
MyInt = Int
MyList = List(Int)

# Type reflection (constructor pattern matching)
describe_type: (type) -> String = (t) => {
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
add: (a: Int, b: Int) -> Int = a + b

# Generic inference
first: (T: Type) -> ((list: List(T)) -> Option(T)) = (list) => {
    if list.length > 0 { some(list[0]) } else { none }
}
```

---

## 4. Memory Management

### 4.1 Ownership Model

YaoXiang uses the **ownership model** for memory management, where each value has a unique owner:

```yaoxiang
# === Default Move (zero-copy) ===
p: Point = Point(1.0, 2.0)
p2 = p              # Move, ownership transferred, p invalidated

# === ref keyword = Arc (safe sharing) ===
shared = ref p      # Arc, thread-safe

spawn(() => print(shared.x))   # ✅ Safe

# === clone() explicit copy ===
p3 = p.clone()      # p and p3 are independent
```

### 4.2 Move Semantics (Default)

```yaoxiang
# Assignment = Move (zero-copy)
p: Point = Point(1.0, 2.0)
p2 = p              # Move, p invalidated

# Function parameter passing = Move
process: (p: Point) -> Void = {
    # Ownership of p transferred in
}

# Return value = Move
create: () -> Point = {
    p = Point(1.0, 2.0)
    return p        # Move, ownership transferred
}
```

### 4.3 ref Keyword (Arc)

```yaoxiang
# ref keyword creates Arc (reference counting)
p: Point = Point(1.0, 2.0)
shared = ref p      # Arc, thread-safe

spawn(() => print(shared.x))   # ✅ Safe

# Arc automatically manages lifecycle
# When shared goes out of scope, count reaches zero and is automatically freed
```

### 4.4 clone() Explicit Copy

```yaoxiang
# Use clone() when you need to keep the original value
p: Point = Point(1.0, 2.0)
p2 = p.clone()   # p and p2 are independent

p.x = 0.0        # ✅
p2.x = 0.0       # ✅ Independent
```

### 4.5 unsafe Code Block (Systems Level)

```yaoxiang
# Raw pointers can only be used in unsafe blocks
p: Point = Point(1.0, 2.0)

unsafe {
    ptr: *Point = &p     # Raw pointer
    (*ptr).x = 0.0       # User guarantees safety
}
```

### 4.6 RAII

```yaoxiang
# RAII automatic cleanup
with_file: (path: String) -> String = {
    file = File.open(path)  # Auto open
    content = file.read_all()
    # Function ends, file auto closes
    content
}
```

### 4.7 Send / Sync Constraints

| Constraint | Semantics | Description |
|------------|-----------|-------------|
| **Send** | Can be safely transferred across threads | Value can be moved to another thread |
| **Sync** | Can be safely shared across threads | Immutable references can be shared to another thread |

```yaoxiang
# ref T automatically satisfies Send + Sync (Arc is thread-safe)
p: Point = Point(1.0, 2.0)
shared = ref p

spawn(() => print(shared.x))   # ✅ Arc is thread-safe

# Raw pointers *T do not satisfy Send/Sync
unsafe {
    ptr: *Point = &p         # Only usable in single thread
}
```

### 4.9 Not Implemented

| Feature | Reason |
|---------|--------|
| Lifetime `'a` | No reference concept, no need for lifetimes |
| Borrow checker | ref = Arc as replacement |
| `&T` borrow syntax | Uses Move semantics instead |

---

## 5. Asynchronous Programming and Concurrency

> "All things act together, and I observe their return." — I Ching, Hexagram Fu (复卦)
>
> YaoXiang adopts the **spawn model**, a seamless async concurrency paradigm based on **lazy evaluation**. Its core design philosophy is: **let developers describe logic with synchronous, sequential thinking, while the language runtime makes the computational units within it automatically and efficiently execute concurrently like all things acting together, and finally unify and coordinate**.

> See [Spawn Model Whitepaper](YaoXiang-async-whitepaper.md) and [Async Implementation Plan](YaoXiang-async-implementation.md) for details.

### 5.1 Spawn Model Core Concepts

#### 5.1.1 Spawn Graph: The Stage for All Things Acting Together

All programs are transformed at compile time into a **directed acyclic computation graph (DAG)**, called the **spawn graph**. Nodes represent expression computations, edges represent data dependencies. This graph is lazy, meaning nodes are only evaluated when their output is **truly needed**.

```yaoxiang
# Compiler automatically constructs spawn graph
fetch_user: spawn () -> User = (id) => { ... }
fetch_posts: spawn (User) -> Posts = (user) => { ... }

main:() -> Void = () => {
    user = fetch_user(1)     # Node A (Async(User))
    posts = fetch_posts(user) # Node B (Async(Posts)), depends on A

    # Node C needs results from A and B
    print(posts.title)       # Auto wait: first ensure A and B complete
}
```

#### 5.1.2 Spawn Values: Async(T)

Any function call marked with `spawn fn` immediately returns a value of type `Async(T)`, called a **spawn value**. This is a lightweight proxy that represents not the actual result, but a **future value being spawned**.

**Core characteristics**:
- **Type transparent**: `Async(T)` is a subtype of `T` in the type system, usable in any context expecting `T`
- **Auto wait**: When program execution reaches an operation that must use the concrete value of type `T`, the runtime automatically suspends the current task and waits for computation to complete
- **Zero contagion**: Async code and sync code have no differences in syntax or type signatures

```yaoxiang
# Spawn value usage example
fetch_data: spawn (String) -> JSON = (url) => { ... }

main: () -> Void = () => {
    data = fetch_data("url")  # Async(JSON)

    # Async(JSON) can be used directly as JSON
    # Auto wait occurs on field access
    print(data.name)          # Equivalent to data.await().name
}
```

### 5.2 Spawn Syntax System

The `spawn` keyword has triple semantics, the sole bridge connecting synchronous thinking with async implementation:

| Official Term | Syntax Form | Semantics | Runtime Behavior |
|--------------|-------------|-----------|------------------|
| **Spawn function** | `spawn fn` | Defines computational units that can participate in spawn execution | Its call returns `Async(T)` |
| **Spawn block** | `spawn { a(), b() }` | Explicitly declared concurrency domain | Tasks within block forced to execute in parallel |
| **Spawn loop** | `spawn for x in xs { ... }` | Data parallelism paradigm | Loop body executes spawnly on all elements |

#### 5.2.1 Spawn Functions

```yaoxiang
# Use spawn to mark spawn functions
# Syntax is identical to regular functions, no extra burden

fetch_api: spawn (String) -> JSON = (url) => {
    response = HTTP.get(url)
    JSON.parse(response.body)
}

# Nested spawn calls
process_user: (Int) -> Report = (user_id) => {
    user = fetch_user(user_id)     # Async(User)
    profile = fetch_profile(user)  # Async(Profile), depends on user
    generate_report(user, profile) # Depends on profile
}
```

#### 5.2.2 Spawn Blocks

```yaoxiang
# spawn { } - Explicit parallel construction
# All expressions in the block execute as independent tasks concurrently

compute_all: (Int, Int) -> (Int, Int, Int) spawn = (a, b) => {
    # Three independent computations execute in parallel
    (x, y, z) = spawn {
        heavy_calc(a),        # Task 1
        heavy_calc(b),        # Task 2
        another_calc(a, b)    # Task 3
    }
    (x, y, z)
}
```

#### 5.2.3 Spawn Loops

```yaoxiang
# spawn for - Data parallel loop
# Each iteration executes as an independent task in parallel

parallel_sum: (Int) -> Int spawn = (n) => {
    total = spawn for i in 0..n {
        fibonacci(i)          # Each iteration in parallel
    }
    total
}
```

#### 5.2.4 Data Parallel Loops

```yaoxiang
# spawn for - Data parallel loop
# Each iteration executes as an independent task in parallel

parallel_sum: (Int) -> Int spawn = (n) => {
    total = spawn for i in 0..n {
        fibonacci(i)          # Each iteration in parallel
    }
    total
}

# Matrix multiplication parallelization
matmul: spawn [[A: Matrix], [B: Matrix]] -> Matrix = (A, B) => {
    result = spawn for i in 0..A.rows {
        row = spawn for j in 0..B.cols {
            dot_product(A.row(i), B.col(j))
        }
        row
    }
    result
}
```

### 5.3 Automatic Wait Mechanism

```yaoxiang
# No explicit await needed, compiler automatically inserts wait points

main: () -> Void = () => {
    # Auto parallel: two independent requests execute in parallel
    users = fetch_users()      # Async(List(User))
    posts = fetch_posts()      # Async(List(Post))

    # Wait point automatically inserted at "+" operation
    count = users.length + posts.length

    # Field access triggers wait
    first_user = users[0]      # Wait for users to be ready
    print(first_user.name)
}

# Wait in conditional branches
process_data: spawn () -> Void = () => {
    data = fetch_data()        # Async(Data)

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
await_all: (T: Type) -> ((tasks: List(Async(T))) -> List(T)) = {
    # Barrier wait
}

# Wait for any one to complete
await_any: (T: Type) -> ((tasks: List(Async(T))) -> T) = {
    # Returns first completed result
}

# Timeout control
with_timeout: (T: Type) -> ((task: Async(T), timeout: Duration) -> Option(T)) = {
    # Returns None on timeout
}
```

### 5.5 Thread Safety: Send/Sync Constraints

YaoXiang uses **Send/Sync type constraints** similar to Rust to ensure thread safety, eliminating data races at compile time.

#### 5.5.1 Send Constraint

**Send**: A type can safely **transfer ownership** across threads.

```yaoxiang
# Basic types automatically satisfy Send
# Int, Float, Bool, String are all Send

# Structs automatically derive Send
type Point = { x: Int, y: Float }
# Point is Send because Int and Float are both Send

# Types containing non-Send fields are not Send
type NonSend = NonSend(data: Rc(Int))
# Rc is not Send (reference counting is non-atomic), so NonSend is not Send
```

#### 5.5.2 Sync Constraint

**Sync**: A type can safely **share references** across threads.

```yaoxiang
# Basic types are all Sync
type Point = { x: Int, y: Float }
# &Point is Sync because &Int and &Float are both Sync

# Types with internal mutability
type Counter = Counter(value: Int, mutex: Mutex(Int))
# &Counter is Sync because Mutex provides internal mutability
```

#### 5.5.3 spawn and Thread Safety

```yaoxiang
# spawn requires parameters and return values to satisfy Send

# Valid: Data is Send
type Data = Data(value: Int)
task = spawn(|| => Data(42))

# Invalid: Rc is not Send
type SharedData = SharedData(rc: Rc(Int))
# task = spawn(|| => SharedData(Rc.new(42))  # Compile error!

# Solution: Use Arc (atomic reference counting)
type SafeData = SafeData(value: Arc(Int))
task = spawn(|| => SafeData(Arc.new(42)))  # Arc is Send + Sync
```

#### 5.5.4 Thread Safety Type Derivation Rules

```yaoxiang
# Struct types
type Struct(T1, T2) = Struct(f1: T1, f2: T2)

# Send derivation
Struct(T1, T2): Send ⇐ T1: Send 且 T2: Send

# Sync derivation
Struct(T1, T2): Sync ⇐ T1: Sync 且 T2: Sync

# Union types
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

# Send derivation
Result(T, E): Send ⇐ T: Send 且 E: Send
```

#### 5.5.5 Standard Library Thread Safety Implementation

| Type | Send | Sync | Description |
|------|:----:|:----:|-------------|
| `Int`, `Float`, `Bool` | ✅ | ✅ | Primitive types |
| `Arc(T)` | ✅ | ✅ | T: Send + Sync |
| `Mutex(T)` | ✅ | ✅ | T: Send |
| `RwLock(T)` | ✅ | ✅ | T: Send |
| `Channel(T)` | ✅ | ❌ | Only send end is Send |
| `Rc(T)` | ❌ | ❌ | Non-atomic reference counting |
| `RefCell(T)` | ❌ | ❌ | Runtime borrow checking |


```yaoxiang
# Thread-safe counter example
type SafeCounter = SafeCounter(mutex: Mutex(Int))

main: () -> Void = () => {
    counter: Arc(SafeCounter) = Arc.new(SafeCounter(Mutex.new(0)))

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
# Use @block annotation to mark operations that block the OS thread
# Runtime will assign them to a dedicated blocking thread pool

@block
read_large_file: (path: String) -> String = {
    # This call will not block the core scheduler
    file = File.open(path)
    content = file.read_all()
    content
}
```

---

## 6. Module System

### 6.1 Module Definition

```yaoxiang
# Modules use files as boundaries
# Math.yx file
pub pi: Float = 3.14159
pub sqrt: (x: Float) -> Float = { ... }
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

## 7. Method Binding and Currying

YaoXiang adopts a **pure functional design**, achieving seamless method calls and currying through an advanced binding mechanism, without introducing keywords like `struct` or `class`.

### 7.1 Core Function Definitions

All operations are implemented through regular functions, with the first parameter conventionally being the subject of the operation:

```yaoxiang
# === Point.yx (module) ===

# Unified syntax: constructors are types
type Point = { x: Float, y: Float }

# Core functions: first parameter is the subject of operation
distance: (a: Point, b: Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    (dx * dx + dy * dy).sqrt()
}

add: (a: Point, b: Point) -> Point = {
    Point(a.x + b.x, a.y + b.y)
}

scale: (p: Point, s: Float) -> Point = {
    Point(p.x * s, p.y * s)
}

# More complex function
distance_with_scale: (s: Float, p1: Point, p2: Point) -> Float = {
    dx = (p1.x - p2.x) * s
    dy = (p1.y - p2.y) * s
    (dx * dx + dy * dy).sqrt()
}
```

### 7.2 Basic Method Binding

#### 7.2.1 Automatic Binding (MoonBit Style)

YaoXiang supports automatic binding based on namespace, **without any extra declarations**:

```yaoxiang
# === Point.yx ===

type Point = { x: Float, y: Float }

# Core functions
distance: (a: Point, b: Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    (dx * dx + dy * dy).sqrt()
}

# === main.yx ===

use Point

main: () -> Void = {
    p1 = Point(3.0, 4.0)
    p2 = Point(1.0, 2.0)

    # ✅ Auto binding: direct method call
    result = p1.distance(p2)  # Resolved to distance(p1, p2)
}
```

**Auto binding rules**:
- Functions defined within a module
- If the first parameter type matches the module name
- Then automatically supports method call syntax

#### 7.2.2 No-Binding Option (Default Behavior)

```yaoxiang
# === Vector.yx ===

type Vector = Vector(x: Float, y: Float, z: Float)

# Internal helper function, not meant for auto binding
dot_product_internal: (a: Vector, b: Vector) -> Float = {
    a.x * b.x + a.y * b.y + a.z * b.z
}

# === main.yx ===

use Vector

main: () -> Void = {
    v1 = Vector(1.0, 0.0, 0.0)
    v2 = Vector(0.0, 1.0, 0.0)

    # ❌ Cannot bind: non-pub functions don't auto bind
    # v1.dot_product_internal(v2)  # Compile error!

    # ✅ Must call directly (not visible outside module)
}
```

### 7.3 Position-Based Binding Syntax

YaoXiang provides the **most elegant binding syntax**, using position markers `[n]` to precisely control binding positions:

#### 7.3.1 Basic Syntax

```yaoxiang
# === Point.yx ===

type Point = { x: Float, y: Float }

# Core functions
distance: (a: Point, b: Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    (dx * dx + dy * dy).sqrt()
}
add: (a: Point, b: Point) -> Point = {
    Point(a.x + b.x, a.y + b.y)
}
scale: (p: Point, s: Float) -> Point = {
    Point(p.x * s, p.y * s)
}

# Binding syntax: Type.method = func[position]
# Meaning: when calling the method, bind the caller to func's [position] parameter

Point.distance = distance[0]      # Bind to 1st parameter
Point.add = add[0]                 # Bind to 1st parameter
Point.scale = scale[0]             # Bind to 1st parameter
```

**Semantic parsing**:
- `Point.distance = distance[0]`
  - `distance` function has two parameters: `distance(Point, Point)`
  - `[0]` means the caller is bound to the 1st parameter
  - Usage: `p1.distance(p2)` → `distance(p1, p2)`

#### 7.3.2 Multi-Position Combined Binding

```yaoxiang
# === Math.yx ===

# Function: scale, point1, point2, extra1, extra2
calculate: (scale: Float, a: Point, b: Point, x: Float, y: Float) -> Float = { ... }

# === Point.yx ===

type Point = { x: Float, y: Float }

# Bind multiple positions
Point.calc1 = calculate[1, 2]      # Bind scale and point1
Point.calc2 = calculate[1, 3]      # Bind scale and point2
Point.calc3 = calculate[2, 3]      # Bind point1 and point2

# === main.yx ===

use Point

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

# 1. Bind[1,2] - remaining 3,4,5
f1 = p1.calc1(2.0)  # Bind scale=2.0, point1=p1
# f1 now needs p2, x, y
result1 = f1(p2, 10.0, 20.0)  # calculate(2.0, p1, p2, 10.0, 20.0)

# 2. Bind[1,3] - remaining 2,4,5
f2 = p2.calc2(2.0)  # Bind scale=2.0, point2=p2
# f2 now needs point1, x, y
result2 = f2(p1, 10.0, 20.0)  # calculate(2.0, p1, p2, 10.0, 20.0)

# 3. Bind[2,3] - remaining 1,4,5
f3 = p1.calc3(p2)  # Bind point1=p1, point2=p2
# f3 now needs scale, x, y
result3 = f3(2.0, 10.0, 20.0)  # calculate(2.0, p1, p2, 10.0, 20.0)
```

#### 7.3.3 Remaining Parameter Fill Order

**Core rule**: After binding, remaining parameters are filled in **the original function's order**, skipping already bound positions.

```yaoxiang
# Suppose function: func(p1, p2, p3, p4, p5)

# Bind 1st and 3rd parameters
Type.method = func[1, 3]

# When called:
method(p2_value, p4_value, p5_value)

# Maps to:
func(p1_bound, p2_value, p3_bound, p4_value, p5_value)
# Remaining parameters: 2,4,5 filled in original order
```

#### 7.3.4 Type Checking Advantages

```yaoxiang
# ✅ Valid binding
Point.distance = distance[0]          # distance(Point, Point)
Point.calc = calculate[1, 2]          # calculate(scale, Point, Point, ...)

# ❌ Invalid binding (compiler error)
Point.wrong = distance[5]             # 5th parameter doesn't exist
Point.wrong = distance[0]             # Parameters start from 1
Point.wrong = distance[1, 2, 3, 4]    # Exceeds function parameter count
```

### 7.4 Fine-Grained Control of Curried Binding

```yaoxiang
# === Math.yx ===

distance_with_scale: (scale: Float, a: Point, b: Point) -> Float = { ... }

# === Point.yx ===

type Point = { x: Float, y: Float }

# Binding strategy: flexible control over each position
Point.distance = distance[0]                    # Basic binding
Point.distance_scaled = distance_with_scale[2]  # Bind to 2nd parameter

# === main.yx ===

use Point
use Math

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

# 1. Basic auto binding
d1 = p1.distance(p2)  # distance(p1, p2)

# 2. Bind to different position
f = p1.distance_scaled(2.0)  # Bind 2nd parameter, remaining 1st,3rd
result = f(p2)               # distance_with_scale(2.0, p1, p2)

# 3. Chained binding
d2 = p1.distance(p2).distance_scaled(2.0)  # Chained call
```

### 7.5 Complete Binding System

```yaoxiang
# === Point.yx ===

type Point = { x: Float, y: Float }

# Core functions
distance: (a: Point, b: Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    (dx * dx + dy * dy).sqrt()
}
add: (a: Point, b: Point) -> Point = {
    Point(a.x + b.x, a.y + b.y)
}
scale: (p: Point, s: Float) -> Point = {
    Point(p.x * s, p.y * s)
}

# Auto binding (core)
Point.distance = distance[0]
Point.add = add[0]
Point.scale = scale[0]

# === Math.yx ===

# Global functions
multiply_by_scale: (scale: Float, a: Point, b: Point) -> Float = { ... }

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

type Point = { x: Float, y: Float }

# Non-pub functions
internal_distance: (a: Point, b: Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    (dx * dx + dy * dy).sqrt()
}

# pub functions
pub distance: (a: Point, b: Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    (dx * dx + dy * dy).sqrt()
}

# === main.yx ===

use Point

# Auto binding only works for pub functions
p1.distance(p2)      # ✅ distance is pub, can auto bind
# p1.internal_distance(p2)  # ❌ Not pub, cannot bind
```

#### 7.6.2 pub Auto Binding Mechanism

Functions declared with `pub`, the compiler automatically binds to types defined in the same file:

```yaoxiang
# === Point.yx ===

type Point = { x: Float, y: Float }

# Using pub declaration, compiler auto binds
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

pub translate: (self: Point, dx: Float, dy: Float) -> Point = {
    Point(self.x + dx, self.y + dy)
}

# Compiler auto infers and executes bindings:
# Point.distance = distance[0]
# Point.translate = translate[0]

# === main.yx ===

use Point

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

# ✅ Functional call
d = distance(p1, p2)

# ✅ OOP syntax sugar (auto binding)
d2 = p1.distance(p2)
p3 = p1.translate(1.0, 1.0)
```

**Auto binding rules**:
1. Function is defined in module file (same file as type)
2. Function parameter contains that type
3. Exported with `pub`
4. Compiler automatically executes `Type.method = function[0]`

**Benefits**:
- No need to manually write binding declarations
- More concise code
- Avoids binding omissions or errors

#### 7.6.3 Module-Level Binding

```yaoxiang
# === Point.yx ===

type Point = { x: Float, y: Float }

distance: (a: Point, b: Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    (dx * dx + dy * dy).sqrt()
}

# Inside module, all functions are visible
# But auto binding only works externally for pub exported functions

pub distance  # Export, usable externally with auto binding
```

### 7.7 Design Advantages Summary

| Feature | Description |
|---------|-------------|
| **Zero syntax burden** | Auto binding requires no declarations |
| **Precise position control** | `[n]` precisely specifies binding position |
| **Multi-position combination** | Supports `[1, 2, 3]` multi-parameter binding |
| **Type safe** | Compiler validates binding position validity |
| **No keywords** | No need for `bind` or other keywords |
| **Flexible currying** | Supports arbitrary position parameter binding |
| **pub control** | Only pub functions can be bound externally |

### 7.8 Differences from Traditional Method Binding

| Traditional language | YaoXiang |
|----------------------|----------|
| `obj.method(arg)` | `obj.method(arg)` |
| Need class/method definition | Only function + binding declaration needed |
| Syntax `class { method() {} }` | Syntax `Type.method = func[n]` |
| Inheritance, polymorphism | Pure functional, no inheritance |
| Method table lookup | Compile-time binding, no runtime overhead |

**Core advantage**: YaoXiang's binding is a **compile-time mechanism**, zero runtime cost, while maintaining the purity and flexibility of functional programming.

---

## 8. AI-Friendly Design

YaoXiang's syntax design particularly considers the needs of AI code generation and modification:

### 8.1 Design Principles

```yaoxiang
# AI-friendly design goals:
# 1. Strictly structured, unambiguous syntax
# 2. Clear AST, easy to locate
# 3. Explicit semantics, no hidden behavior
# 4. Clear code block boundaries
# 5. Complete type information
```

### 8.2 Strictly Structured Syntax

#### 8.2.1 AI-Friendly Strategy for Declaration Syntax

```yaoxiang
# === AI Code Generation Best Practices ===

# ✅ Recommended: Use complete new syntax declaration + type-centralized convention
# AI can accurately understand intent, generate complete type information

add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
empty: () -> Void = {}

# ❌ Avoid: Omitting type annotations or scattering types
# AI cannot determine parameter types, may generate incorrect code
add: (a: Int, b: Int) -> Int = a + b          # Parameters without type
identity: (T: Type) -> ((x: T) -> T) = x              # Parameters without type
add2: (a: Int, b: Int) -> Int = a + b  # Types scattered in implementation

# ⚠️ Compatibility: Old syntax only for maintenance
# AI should prefer generating new syntax + type-centralized convention
mul(Int, Int) -> Int = (a, b) => a * b  # Not recommended for new code
```

**AI advantages of type-centralized convention:**

1. **Signature at a glance**: AI only needs to read the declaration line to understand the complete function signature
2. **Safer modifications**: Modifying types only requires changing the declaration, not affecting the implementation body
3. **Easier generation**: AI can first generate the declaration, then fill in the implementation
4. **Currying friendly**: Clear curried type signatures are easy for AI to process

```yaoxiang
# AI processing example
# Input: implementation body (a, b) => a + b
# AI sees declaration: add: (Int, Int) -> Int
# Conclusion: parameter types are Int, Int, return type is Int

# Contrast: scattered types
# Input: implementation body (a: Int, b: Int) => a + b
# AI needs: analyze implementation body to extract type information
# Result: more complex processing logic, error-prone
```

#### 8.2.2 Dual Syntax Strategy and AI

| Syntax type | AI generation strategy | Use case |
|-------------|----------------------|----------|
| **New syntax** | ✅ Preferred generation, complete type information | All new code generation |
| **Old syntax** | ⚠️ Only when maintaining old code | Historical code modification |
| **No annotation** | ❌ Avoid generating | Should never be generated |

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

# ✅ Condition statements must have braces
if condition {
    # Condition body
}

# ✅ Type definitions are clear
type MyType = Type1 | Type2

# ❌ Ambiguous写法 to avoid
if condition    # Missing braces
    do_something()
```

#### 8.2.4 Unambiguous Syntax Constraints

```yaoxiang
# Constraints AI must follow when generating

# 1. Parentheses cannot be omitted
# ✅ Correct
foo: (T: Type) -> ((x: T) -> T) = x
my_list = [1, 2, 3]

# ❌ Forbidden (incorrect)
foo T { T }             # Parameters must have parentheses
my_list = [1 2 3]       # Lists must have commas

# 2. Return type must be explicit or inferrable
# ✅ Correct
get_num: () -> Int = 42
get_num2: () = 42          # Return type inferrable
get_void = () => { 42 }    # ✓ Inferred as Void (block without return defaults to Void)

# 3. Parameters must have type annotations (new syntax)
# ✅ Correct
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1

# ❌ Incorrect
add: (a: Int, b: Int) -> Int = a + b            # Parameters without type
identity: (T: Type) -> ((x: T) -> T) = x                # Parameters without type
```

#### 8.2.5 AI Generation Recommended Patterns

```yaoxiang
# Standard template when AI generates functions

# Pattern 1: Complete type annotation
function_name: (param1: ParamType1, param2: ParamType2, ...) -> ReturnType = {
    # Function body
    return expression
}

# Pattern 2: Return type inference
function_name: (param1: ParamType1, param2: ParamType2) = {
    # Function body
    return expression
}

# Pattern 3: Single parameter shorthand
function_name: (param: ParamType) -> ReturnType = expression

# Pattern 4: No-parameter function
function_name: () -> ReturnType = expression

# Pattern 5: Empty function
function_name: () -> Void = {}
```

### 8.3 AI Friendliness of Error Messages

```yaoxiang
# Error messages should provide clear fix suggestions

# Unfriendly error
# Syntax error at token 'a'

# AI-friendly error
# Missing type annotation for parameter 'a'
# Suggestion: add ': Int' or similar type to '(a, b) => a + b'
# Correct version: add: (a: Int, b: Int) -> Int = a + b
```

---

## 9. Type-Centralized Convention (Core Design Philosophy)

### 9.1 Convention Overview

The core design convention of YaoXiang is **"declaration first, types centralized"**. This convention is the foundation of the language's AI friendliness and development efficiency.

```yaoxiang
# ✅ Core convention: Type information unified on the declaration line
add: (a: Int, b: Int) -> Int = a + b

# ❌ Avoid: Type information scattered in the implementation
add: (a: Int, b: Int) -> Int = a + b
```

### 9.2 Five Core Advantages of the Convention

#### 1. Syntax Consistency
```yaoxiang
# All declarations follow the same format
x: Int = 42                           # Variable
name: String = "YaoXiang"             # Variable
add: (a: Int, b: Int) -> Int = a + b  # Function
inc: (x: Int) -> Int = x + 1          # Function
type Point = { x: Float, y: Float } # Type
```

#### 2. Declaration and Implementation Separation
```yaoxiang
# Declaration line provides complete type information
add: (a: Int, b: Int) -> Int = a + b
# └────────────────────┘
#   Complete function signature

# Implementation body focuses on business logic
# (a, b) => a + b  No need to worry about types, just implement functionality
```

#### 3. AI Friendliness
```yaoxiang
# AI processing flow:
# 1. Read declaration line → Completely understand function signature
# 2. Generate implementation → No need for type analysis
# 3. Modify types → Only change declaration line, not implementation

# Contrast: scattered types approach
add: (a: Int, b: Int) -> Int = a + b
# AI needs: analyze implementation body to extract type information → More complex, error-prone
```

#### 4. Safer Modifications
```yaoxiang
# Modifying parameter types
# Original: add: (a: Int, b: Int) -> Int = a + b
# Modified: add: (Float, Float) -> Float = (a, b) => a + b
# Implementation: (a, b) => a + b  No need to change!

# If types are scattered:
# Original: add: (a: Int, b: Int) -> Int = a + b
# Modified: add: (a: Float, b: Float) -> Float = a + b  # Need to change in two places
```

#### 5. Currying Friendly
```yaoxiang
# Curried types are clear at a glance
add_curried: (a: Int) -> (b: Int) -> Int = a + b
#              └─────────────┘
#              Curried signature

# Function composition as first-class citizen
compose: (Int -> Int, Int -> Int) -> Int -> Int = (f, g) => x => f(g(x))
```

### 9.3 Implementation Rules of the Convention

#### Rule 1: Parameters must have type annotations in the declaration
```yaoxiang
# ✅ Correct
add: (a: Int, b: Int) -> Int = a + b

# ❌ Incorrect
add: (a: Int, b: Int) -> Int = a + b            # Parameter types missing
identity: (T: Type) -> ((x: T) -> T) = x                # Parameter types missing
```

#### Rule 2: Return types can be inferred but recommended to annotate
```yaoxiang
# ✅ Recommended: Complete annotation
get_num: () -> Int = () => 42

# ✅ Acceptable: Return type inferred
get_num: () = () => 42

# ✅ Empty function inferred as Void
empty: () = () => {}
```

#### Rule 3: Lambda internal type annotations are temporary
```yaoxiang
# ✅ Correct: Rely on types in declaration
add: (a: Int, b: Int) -> Int = a + b

# ⚠️ Acceptable but not recommended: Repeated annotation in Lambda
add: (Int, Int) -> Int = (a: Int, b: Int) => a + b

# ❌ Incorrect: Missing declaration annotation
add: (a: Int, b: Int) -> Int = a + b
```

#### Rule 4: Old syntax follows the same philosophy
```yaoxiang
# Old syntax should also provide type information at declaration position as much as possible
# Although the format is different, the philosophy is consistent:
# - Declaration line contains main type information
# - Implementation body is relatively concise
add(Int, Int) -> Int = (a, b) => a + b
```

### 9.4 Convention and Type Inference Relationship

```yaoxiang
# Convention doesn't block type inference, but guides inference direction

# 1. Complete annotation (no inference)
add: (a: Int, b: Int) -> Int = a + b

# 2. Partial inference (declaration provides parameter types)
add: (Int, Int) = (a, b) => a + b  # Return type inferred

# 3. Empty function inference
empty: () = () => {}  # Inferred as () -> Void
```

### 9.5 AI Implementation Advantages of the Convention

**AI code generation flow:**

1. **Read requirements** → Generate declaration
   ```
   Requirement: addition function
   Generate: add: (Int, Int) -> Int = (a, b) => ???
   ```

2. **Fill implementation** → No type analysis needed
   ```
   Implementation: add: (a: Int, b: Int) -> Int = a + b
   ```

3. **Type modification** → Only change declaration
   ```
   Modify: add: (Float, Float) -> Float = (a, b) => a + b
   Implementation: (a, b) => a + b  Remains unchanged
   ```

**Contrast with AI processing without convention:**
```
Requirement: addition function
AI needs:
  1. Infer parameter types
  2. Infer return type
  3. Generate implementation body
  4. Verify consistency
  5. Handle complex updates when types change

Result: More complex, more error-prone
```

### 9.6 Philosophical Significance of the Convention

This convention embodies the core philosophy of YaoXiang:

- **Declaration is documentation**: The declaration line is the complete function documentation
- **Type is contract**: Type information is the contract between caller and implementer
- **Logic is implementation**: Implementation body only focuses on "what to do", not "what type"
- **Tools are assistants**: Type system, AI tools can all work based on clear declarations

### 9.7 Practical Application Comparison

#### Complete example: Calculator module

```yaoxiang
# === Recommended approach: Type-centralized convention ===

# Module declarations
pub add: (a: Int, b: Int) -> Int = a + b
pub multiply: (a: Int, b: Int) -> Int = a * b

# Higher-order functions
pub apply_twice: (f: Int -> Int, x: Int) -> Int = f(f(x))

# Curried functions
pub make_adder: (x: Int) -> (Int) -> Int = y => x + y

# Generic functions
pub map: (A: Type, B: Type) -> ((f: (A) -> B, xs: List(A)) -> List(B)) = case xs of
  [] => []
  (x :: rest) => f(x) :: map(f, rest)

# Type definitions
type Point = { x: Float, y: Float }
pub distance: (a: Point, b: Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    (dx * dx + dy * dy).sqrt()
}

# === Not recommended: Scattered types ===

# Parameter types in Lambda
add: (a: Int, b: Int) -> Int = a + b
multiply = (a: Int, b: Int) => a * b

# Higher-order function types scattered
apply_twice = (f: (Int) -> Int, x: Int) => f(f(x))

# Curried function types scattered
make_adder = (x: Int) => (y: Int) => x + y

# Generic function types scattered
map: (A: Type, B: Type) -> ((f: (A) -> B, xs: List(A)) -> List(B)) = case xs of
  [] => []
  (x :: rest) => f(x) :: map(f, rest)
```

#### Code Maintenance Comparison

```yaoxiang
# Requirement: Change add from Int to Float

# === Recommended approach: Only need to change declaration line ===
# Original
add: (a: Int, b: Int) -> Int = a + b

# After modification
add: (a: Float, b: Float) -> Float = a + b
#              ↑↑↑↑↑↑↑↑↑          ↑↑↑↑↑↑↑
#              Declaration line changed          Implementation remains unchanged

# === Not recommended: Need to change multiple places ===
# Original
add: (a: Int, b: Int) -> Int = a + b

# After modification
add: (a: Float, b: Float) -> Float = a + b
#     ↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑
#     All parameter types need to be changed
```

#### AI-Assisted Programming Comparison

```yaoxiang
# AI requirement: Implement a function calculating Manhattan distance between two points

# === When AI sees recommended写法 ===
type Point = { x: Float, y: Float }
pub manhattan: (a: Point, b: Point) -> Float = ???  # AI directly knows complete signature

# AI generates:
pub manhattan: (a: Point, b: Point) -> Float = {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}

# === When AI sees not recommended写法 ===
type Point = { x: Float, y: Float }
pub manhattan = ???  # AI needs to infer: parameter types? return type?

# AI might generate:
pub manhattan = (a: Point, b: Point) => Float => {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}
# Or might make mistakes because type information is incomplete
```

### 9.8 Convention Implementation Checklist

When writing YaoXiang code, you can use the following checklist:

- [ ] All function declarations have complete type annotations on the declaration line
- [ ] Parameter types are specified in declaration, not in Lambda
- [ ] Return types are annotated in declaration as much as possible
- [ ] Variable declarations use `name: Type = value` format
- [ ] Lambda bodies remain concise, not duplicating type information
- [ ] Use new syntax instead of old syntax
- [ ] Complex types use type definitions, keeping declarations clear

---

## 10. Quick Start

### 10.1 Hello World

```yaoxiang
# hello.yx
use std.io

main: () -> Void = {
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
add: (a: Int, b: Int) -> Int = a + b

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

type Point = { x: Float, y: Float }

# Core functions
distance: (a: Point, b: Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    (dx * dx + dy * dy).sqrt()
}

# Auto binding
Point.distance = distance[0]

# === main.yx ===

use Point

main: () -> Void = {
    p1 = Point(3.0, 4.0)
    p2 = Point(1.0, 2.0)

    # Using binding
    d = p1.distance(p2)  # distance(p1, p2)
    print(d)
}
```

### 10.4 Curried Binding Example

```yaoxiang
# === Math.yx ===

distance_with_scale: (scale: Float, a: Point, b: Point) -> Float = {
    dx = (p1.x - p2.x) * scale
    dy = (p1.y - p2.y) * scale
    (dx * dx + dy * dy).sqrt()
}

# === Point.yx ===

type Point = { x: Float, y: Float }

Point.distance_scaled = distance_with_scale[2]  # Bind to 2nd parameter

# === main.yx ===

use Point

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

# Using binding
f = p1.distance_scaled(2.0)  # Bind scale and p1
result = f(p2)               # Final call

# Or use directly
result2 = p1.distance_scaled(2.0, p2)
```

### 10.5 Next Steps

- Read the [Language Specification](./YaoXiang-language-specification.md) for complete syntax
- Check out [Example Code](./examples/) to learn common patterns
- Reference the [Implementation Plan](./YaoXiang-implementation.md) for technical details

---

## Appendix

### A. Keywords and Annotations

| Keyword | Purpose |
|---------|---------|
| `type` | Type definition |
| `pub` | Public export |
| `use` | Import module |
| `spawn` | Async marker (function/block/loop) |
| `ref` | Immutable reference |
| `mut` | Mutable reference |
| `if/elif/else` | Conditional branching |
| `match` | Pattern matching |
| `while/for` | Loops |
| `return/break/continue` | Control flow |
| `as` | Type casting |
| `in` | Member access |

| Annotation | Purpose |
|------------|---------|
| `@block` | Mark as fully synchronous code |
| `@eager` | Mark expressions that need eager evaluation |
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
| v1.1.0 | 2025-01-04 | Mo Yu Jiang | Fixed generic syntax to `[T]` (not `<T>`); Removed `fn` keyword; Updated function definition examples |
| v1.2.0 | 2025-01-06 | Chen Xu | Unified to new syntax format: name: type -> type = lambda |
| v1.3.0 | 2025-01-20 | Chen Xu | Added unified type syntax (RFC-010): interface definitions use braces `{ serialize: () -> String }`; interface names listed at end of type to implement interfaces; `pub` auto binding mechanism |

---

> "The Tao gives birth to the One, the One gives birth to the Two, the Two gives birth to the Three, the Three gives birth to the Ten Thousand Things."
> — Dao De Jing
>
> Types are like the Tao, from which all things are born.