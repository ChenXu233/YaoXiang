# YaoXiang Programming Language Guide

> Version: v1.2.0
> Status: Draft
> Author: Chenxu (晨煦)
> Date: 2024-12-31
> Update: 2025-01-20 - Position indexing starts from 0 (RFC-004); Unified type syntax (RFC-010)

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
9. [Centralized Type Convention](#9-centralized-type-convention-core-design-philosophy)
10. [Quick Start](#10-quick-start)

---

**Extended Documentation**:
- [Advanced Binding Features and Compiler Implementation](../works/plans/bind/YaoXiang-bind-advanced.md) - In-depth binding mechanisms, advanced features, compiler implementation, and edge case handling

---

## 1. Language Overview

### 1.1 What is YaoXiang?

YaoXiang (爻象) is an experimental general-purpose programming language whose design philosophy draws from the core concepts of "Yao" (爻) and "Xiang" (象) in the *I Ching*. "Yao" represents the basic symbols composing hexagrams, symbolizing the interplay of yin and yang; "Xiang" represents the external manifestation of the essence of things, signifying all phenomena.

YaoXiang integrates this philosophical thought into its type system, proposing the core concept of **"Everything is a Type"**. In the worldview of YaoXiang:

- **Values** are instances of types
- **Types** themselves are instances of types (metatypes)
- **Functions** are mappings from input types to output types
- **Modules** are combinations of type namespaces

### 1.2 Design Goals

YaoXiang's design goals can be summarized in the following aspects:

| Goal | Description |
|------|------|
| **Unified Type Abstraction** | Types are the highest-level abstraction units, simplifying language semantics |
| **Natural Programming Experience** | Python-style syntax, emphasizing readability |
| **Safe Memory Management** | Rust-style ownership model, no GC |
| **Transparent Asynchronous Programming** | Automatic management of async, no explicit `await` required |
| **Complete Type Reflection** | Full runtime type information available |
| **AI-Friendly Syntax** | Strictly structured, easy for AI to process |

### 1.3 Language Positioning

| Dimension | Positioning |
|------|------|
| Paradigm | Multi-paradigm (Functional + Imperative + Object-Oriented) |
| Type System | Dependent types + Parametric polymorphism |
| Memory Management | Ownership + RAII (no GC) |
| Compilation Model | AOT compilation (optional JIT) |
| Target Scenarios | Systems programming, application development, AI-assisted programming |

### 1.4 Code Example

```yaoxiang
// Automatic type inference
x: Int = 42 // Explicit type
y = 42 // Inferred as Int
name = "YaoXiang" // Inferred as String

// Immutable by default
x: Int = 10
x = 20 // ❌ Compile error! Immutable

// Unified declaration syntax: identifier: Type = expression
add: (a: Int, b: Int) -> Int = a + b // Function declaration
inc: (x: Int) -> Int = x + 1 // Single-parameter function

// Unified type syntax: constructors are types
Point: Type = { x: Float, y: Float }
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

// Transparent async (spawn function)
fetch_data: (url: String) -> JSON spawn = {
 HTTP.get(url).json()
}

main: () -> Void = {
 // Value construction: identical to function call
 p = Point(3.0, 4.0)
 r = ok("success")

 data = fetch_data("https://api.example.com")
 // Automatic wait, no await needed
 print(data.name)
}

// Generic function
identity: (T: Type) -> ((x: T) -> T) = x

// Higher-order function
apply: (f: (Int) -> Int, x: Int) -> Int = f(x)

// Currying
add_curried: (a: Int) -> (b: Int) -> Int = a + b
```

---

## 2. Core Features

### 2.1 Everything is a Type

YaoXiang's core design philosophy is **Everything is a Type**. This means in YaoXiang:

1. **Values are instances of types**: `42` is an instance of type `Int`
2. **Types are instances of types**: `Int` is an instance of the `type` metatype
3. **Functions are type mappings**: `add: (Int, Int) -> Int` is a function type
4. **Modules are combinations of types**: Modules are namespaces containing functions and types

```yaoxiang
// Values are instances of types
x: Int = 42

// Types are instances of types
MyList: type = List(Int)

// Functions are mappings between types
add: (a: Int, b: Int) -> Int = a + b

// Modules are combinations of types (files as modules)
// Math.yx
pi: Float = 3.14159
sqrt: (x: Float) -> Float = { ... }
```

### 2.2 Mathematical Abstraction

YaoXiang's type system is based on type theory and category theory, providing:

- **Dependent types**: Types can depend on values
- **Generic programming**: Type parameterization
- **Type composition**: Union types, intersection types

```yaoxiang
// Dependent type: fixed-length vector
Vector: (T: Type, n: Int) -> Type = vector(T, n)

// Type union
Number: Type = { Int | Float }

// Type intersection
Printable: Type = printable(() -> String)
Serializable: Type = serializable(() -> String)
Versatile: Type = Printable & Serializable
```

### 2.3 Zero-Cost Abstractions

YaoXiang guarantees zero-cost abstractions, meaning high-level abstractions incur no runtime overhead:

- **Monomorphization**: Generic functions are expanded into concrete versions at compile time
- **Inlining optimization**: Simple functions are automatically inlined
- **Stack allocation**: Small objects are allocated on the stack by default

```yaoxiang
// Generic expansion (monomorphization)
identity: (T: Type) -> ((x: T) -> T) = x

// Usage
int_val = identity(42) // Expanded to identity(Int) -> Int
str_val = identity("hello") // Expanded to identity(String) -> String

// No extra overhead after compilation
```

### 2.4 Natural Syntax

YaoXiang adopts Python-style syntax, pursuing readability and a natural language feel:

```yaoxiang
// Automatic type inference
x = 42
name = "YaoXiang"

// Concise function definitions
greet: (name: String) -> String = "Hello, " + name

// Pattern matching
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

YaoXiang adopts a unified declaration syntax: **identifier: Type = expression**. It also provides backward-compatible legacy syntax.

#### 2.5.1 Dual-Syntax Strategy and Centralized Type Convention

To balance innovation and compatibility, YaoXiang supports two syntax forms, but adopts a unified **centralized type annotation convention**.

**Syntax Form Comparison:**

| Syntax Type | Format | Status | Description |
|---------|------|------|------|
| **New Syntax (Standard)** | `name: Type = Lambda` | ✅ Recommended | Official standard, all new code should use this form |
| **Legacy Syntax (Compatibility)** | `name(Types) -> Ret = Lambda` | ⚠️ Compatibility only | Preserved for legacy code, not recommended for new projects |

**Core Convention: Centralized Type Annotation**

YaoXiang adopts a **"Declaration first, type centralized"** design convention:

```yaoxiang
// ✅ Correct: type information is unified in the declaration line
add: (a: Int, b: Int) -> Int = a + b
// └─────────────────┘ └─────────────┘
// Complete type signature Implementation logic

// ❌ Avoid: type information scattered in implementation
add: (a: Int, b: Int) -> Int = a + b
// └───────────────┘
// Type mixed in implementation body
```

**Benefits of the Convention:**

1. **Syntax consistency**: All declarations follow `identifier: Type = expression`
2. **Separation of declaration and implementation**: Type information is clear at a glance, implementation body focuses on logic
3. **AI-friendly**: AI only needs to read the declaration line to understand the complete function signature
4. **Safer modification**: Modifying types only requires changing the declaration, without affecting the implementation body
5. **Currying-friendly**: Supports clear currying type signatures

**Selection Recommendations**:
- **New projects**: Must use the new syntax + centralized type convention
- **Migration projects**: Gradually migrate to the new syntax and centralized type convention
- **Maintaining legacy code**: May continue to use the legacy syntax, but adopting the centralized type convention is recommended

#### 2.5.2 Basic Declaration Syntax

```yaoxiang
// === New Syntax (Recommended) ===
// All declarations follow: identifier: Type = expression

// Variable declaration
x: Int = 42
name: String = "YaoXiang"
mut counter: Int = 0

// Function declaration
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
getAnswer: () -> Int = 42
log: (msg: String) -> Void = print(msg)

// === Legacy Syntax (Compatibility) ===
// Used only for functions, format: name(Types) -> Ret = Lambda
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

# Function types are first-class citizens and can be nested
# Higher-order function type ::= '(' FunctionType ')' '->' ReturnType
```

| Example | Meaning |
|------|------|
| `Int -> Int` | Single-parameter function type |
| `(Int, Int) -> Int` | Two-parameter function type |
| `() -> Void` | No-parameter function type |
| `(Int -> Int) -> Int` | Higher-order: takes a function, returns Int |
| `Int -> Int -> Int` | Curried function (right-associative) |

#### 2.5.4 Generic Syntax (Used Only for Type Parameters)

```yaoxiang
// Generic function: <TypeParameter> prefix
identity: (T: Type) -> ((x: T) -> T) = x
map: (A: Type, B: Type) -> ((f: (A) -> B, xs: List(A)) -> List(B)) = case xs of
 [] => []
 (x :: rest) => f(x) :: map(f, rest)

// Generic type
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
|------|------|------|
| `(a, b) => a + b` | Multi-parameter Lambda | Used with declaration:<br>`add: (Int, Int) = (a, b) => a + b` |
| `x => x + 1` | Single parameter shorthand | Used with declaration:<br>`inc: Int = x => x + 1` |
| `(x: Int) => x + 1` | With type annotation | Only for temporary needs within Lambda |
| `() => 42` | No-parameter Lambda | Used with declaration:<br>`get: () = () => 42` |

**Note**: Type annotations in Lambda expressions like `(x: Int) => ...` are **temporary and local**, primarily used for:
- When type information is needed inside the Lambda
- When used together with declaration syntax (types are given in the declaration)
- Should not serve as the primary type declaration method

#### 2.5.6 Complete Examples

```yaoxiang
// === Basic Function Declarations ===

// Basic function (new syntax)
add: (a: Int, b: Int) -> Int = a + b

// Single-parameter function (two forms)
inc: (x: Int) -> Int = x + 1
inc2: (x: Int) -> Int = x + 1

// No-parameter function
getAnswer: () -> Int = 42

// No-return-value function
log: (msg: String) -> Void = print(msg)

// === Recursive Function ===
// Recursion is naturally supported in lambdas
fact: (n: Int) -> Int =
 if n <= 1 then 1 else n * fact(n - 1)

// === Higher-Order Functions and Function Type Assignment ===

// Function types as first-class citizens
IntToInt: Type = (Int) -> Int
IntBinaryOp: Type = (Int, Int) -> Int

// Higher-order function declaration
applyTwice: (f: IntToInt, x: Int) -> Int = f(f(x))

// Curried function
addCurried: (a: Int) -> (b: Int) -> Int = a + b

// Function composition
compose: (f: Int -> Int, g: Int -> Int) -> (x: Int) -> Int =
 f(g(x))

// Function that returns a function
makeAdder: (x: Int) -> (y: Int) -> Int =
 x + y

// === Generic Functions ===

// Generic function
identity: (T: Type) -> ((x: T) -> T) = x

// Generic higher-order function
map: (A: Type, B: Type) -> ((f: (A) -> B, xs: List(A)) -> List(B)) =
 case xs of
 [] => []
 (x :: rest) => f(x) :: map(f, rest)

// Generic function type
Transformer: Type = (A: Type, B: Type) -> ((A) -> B)

// Using a generic type
applyTransformer: (A: Type, B: Type) -> ((f: Transformer(A, B), x: A) -> B) =
 f(x)

// === Complex Type Examples ===

// Nested function type
higherOrder: (A: Type) -> ((f: (A) -> Int) -> (A) -> Int) =
 f => x => f(x) + 1

// Multi-parameter higher-order function
zipWith: (A: Type, B: Type, C: Type) -> ((f: (A, B) -> C, xs: List(A), ys: List(B)) -> List(C)) =
 case (xs, ys) of
 ([], _) => []
 (_, []) => []
 (x::xs', y::ys') => f(x, y) :: zipWith(f, xs', ys')

// Function type aliases
Predicate: (T: Type) -> Type = { apply: (T) -> Bool }
Mapper: Type = (A: Type, B: Type) -> ((A) -> B)
Reducer: Type = (A: Type, B: Type) -> ((B, A) -> B)

// === Legacy Syntax Examples (Backward Compatibility Only) ===
// Not recommended for new code

mul(Int, Int) -> Int = (a, b) => a * b // Multi-parameter
square(Int) -> Int = (x) => x * x // Single parameter
empty() -> Void = () => {} // No parameter
get_random() -> Int = () => 42 // With return value

// Equivalent new syntax (recommended)
mul: (a: Int, b: Int) -> Int = a * b
square: (x: Int) -> Int = x * x
empty: () -> Void = {}
get_random: () -> Int = 42
```

#### 2.5.7 Syntax Parsing Rules

**Type Parsing Precedence:**

| Precedence | Type | Description |
|--------|------|------|
| 1 (Highest) | Generic application `List(T)` | Left-associative |
| 2 | Parentheses `(T)` | Changes associativity |
| 3 | Function type `->` | Right-associative |
| 4 (Lowest) | Primitive type `Int, String` | Atomic types |

**Type Parsing Examples:**

```yaoxiang
// (A -> B) -> C -> D
// Parsed as: ((A -> B) -> (C -> D))

// A -> B -> C
// Parsed as: (A -> (B -> C)) // Right-associative

// (Int -> Int) -> Int
// Parsed as: takes a function, returns Int -> Int

// List<Int -> Int>
// Parsed as: List's element type is Int -> Int
```

**Lambda Parsing Examples:**

```yaoxiang
// a => b => a + b
// Parsed as: a => (b => (a + b)) // Right-associative, curried

// (a, b) => a + b
// Parsed as: takes two parameters, returns a + b
```

#### 2.5.8 Type Inference Rules

YaoXiang adopts a **two-layer processing** strategy: the parsing layer is lenient, while the type-checking layer strictly infers.

**Parser Layer Rules:**
- The parser only validates syntactic structure, without performing type inference
- Declarations missing type annotations have their type annotation field as `None`
- All declarations conforming to basic syntactic structure pass parsing
- **Key point**: `add: (a: Int, b: Int) -> Int = a + b` is **legal** at the parser layer

**Type Checker Layer Rules:**
- Validates semantic correctness, including type completeness
- **Parameters must have type annotations**: This is a mandatory requirement
- Return type can be inferred, but parameter types must be explicitly declared

**Complete Type Inference Rules:**

| Scenario | Parameter Inference | Return Inference | Parse Result | Type Check Result | Recommendation |
|------|---------|---------|----------|-------------|-------------|
| **Standard Function** | Uses annotated type | Uses annotated type | ✅ | ✅ | ⭐⭐⭐⭐⭐ |
| `add: (a: Int, b: Int) -> Int = a + b` | | | | | |
| **Partial Inference** | Uses annotated type | Inferred from expression | ✅ | ✅ | ⭐⭐⭐⭐ |
| `add: (Int, Int) = (a, b) => a + b` | | | | | |
| `inc: (x: Int) -> Int = x + 1` | | | | | |
| `get: () = () => 42` | | | | | |
| **Legacy Partial Inference** | Uses annotated type | Inferred from expression | ✅ | ✅ | ⭐⭐⭐ (Compatibility) |
| `add(Int, Int) = (a, b) => a + b` | | | | | |
| `square(Int) = (x) => x * x` | | | | | |
| **No Parameter Annotation** | **Cannot infer** | - | ✅ | ❌ Error | ❌ Prohibited |
| `add: (a: Int, b: Int) -> Int = a + b` | | | | | |
| `identity: (T: Type) -> ((x: T) -> T) = x` | | | | | |
| **Block Without Return Annotation** | - | Inferred from block content | ✅ | ✅ | ⭐⭐⭐⭐ |
| `main = () => {}` | | | | | |
| `get = () => { return 42; }` | | | | | |
| **Block Without return** | - | Defaults to `Void` | ✅ | ✅ Correct | ✅ Correct |
| `add: (a: Int, b: Int) -> Int = { return a + b }` | | | | | |

**Detailed Inference Rules:**

```
Parser layer: only looks at syntactic structure
├── Structure correct → Pass
└── Structure incorrect → Error

Type checker layer: validates semantics
├── Parameter type inference
│   ├── Parameter has type annotation → Use annotated type ✅
│   ├── Parameter has no type annotation → Reject ❌
│   └── Lambda parameters must be annotated → Mandatory requirement
│
├── Return type inference
│   ├── Has return expr → Infer from expr ✅
│   ├── No return, has expression → Infer from expression ✅
│   ├── Block has no return → Defaults to Void ✅
│   └── Cannot infer → Reject ❌
│
└── Completely unable to infer → Reject ❌
```

**Note**: When a code block has no `return`, it defaults to returning `Void`. For example:
- `() => { 42 }` infers to `() -> Void` (block has no return, defaults to Void)
- `() => { return 42 }` infers to `() -> Int` (has return, inferred from return)
- `() => 42` infers to `() -> Int` (expression form, returns directly)

**Inference Examples:**

```yaoxiang
// === Successful Inference ===

// Standard form
main: () -> Void = () => {} // Complete annotation
num: () -> Int = () => 42 // Complete annotation
inc: (x: Int) -> Int = x + 1 // Single parameter shorthand

// Partial inference (new syntax)
add: (Int, Int) = (a, b) => a + b // Parameters annotated, return inferred
square: (x: Int) -> Int = x * x // Parameters annotated, return inferred
get_answer: () = () => 42 // Parameters annotated (empty), return inferred

// Partial inference (legacy syntax, compatibility)
add2(Int, Int) = (a, b) => a + b // Parameters annotated, return inferred
square2(Int) = (x) => x * x // Parameters annotated, return inferred

// Inferred from return
fact: Int -> Int = (n) => {
 if n <= 1 { return 1 }
 return n * fact(n - 1)
}

// === Inference Failure ===

// Parameters cannot be inferred (parse passes, type check fails)
add: (a: Int, b: Int) -> Int = a + b // ✗ Parameters have no type
identity: (T: Type) -> ((x: T) -> T) = x // ✗ Parameters have no type

// Block without return
no_return = (x: Int) => { x } // ✓ Inferred as Void (block has no return, defaults to Void)

// Completely unable to infer
bad_fn: (T: Type) -> ((x: T) -> T) = x // ✗ Both parameters and return cannot be inferred
```

#### 2.5.9 Legacy Syntax (Backward Compatibility)

YaoXiang provides legacy syntax support for compatibility with historical code, **not recommended for new code**.

```
LegacySyntax ::= Identifier '(' [ParameterTypeList] ')' '->' ReturnType '=' Lambda
```

| Feature | Standard Syntax | Legacy Syntax |
|------|---------|--------|
| Declaration format | `name: Type = ...` | `name(Types) -> Type = ...` |
| Parameter type position | In type annotation | In parentheses after function name |
| Empty parameters | Must write `()` | Can omit `()` |
| **Recommendation** | ✅ **Officially recommended** | ⚠️ **Backward compatibility only** |
| **Use case** | All new code | Legacy code maintenance |

**Reasons Not to Recommend:**
1. **Learning cost**: Inconsistent with standard syntax, increases language complexity
2. **Consistency**: Parameter type position is not unified (one in type annotation, one after function name)
3. **Maintenance cost**: Parser needs additional handling for both forms
4. **AI-friendliness**: Increases difficulty for AI to understand and generate code

**Migration Suggestions:**
```yaoxiang
// Legacy code (not recommended)
mul(Int, Int) -> Int = (a, b) => a * b
square(Int) -> Int = (x) => x * x
empty() -> Void = () => {}

// New code (recommended)
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
│  type (metatype)                                             │
│    │                                                        │
│    ├── Primitive Types                                       │
│    │   ├── Void, Bool                                       │
│    │   ├── Int, Uint, Float                                 │
│    │   ├── Char, String, Bytes                              │
│    │                                                        │
│    ├── Constructor Types                                    │
│    │   ├── Name(args)              # Single constructor (struct) │
│    │   ├── A(T) | B(U)             # Multiple constructors (union/enum) │
│    │   ├── A | B | C               # Zero-arg constructors (enum) │
│    │   ├── tuple (T1, T2, ...)                            │
│    │   ├── list(T), dict(K, V)                           │
│    │                                                        │
│    ├── Function Types                                       │
│    │   fn (T1, T2, ...) -> R                               │
│    │                                                        │
│    ├── Generic Types                                        │
│    │   List(T), Map(K, V), etc.                            │
│    │                                                        │
│    ├── Dependent Types                                      │
│    │   (n: Int) -> Type                               │
│    │                                                        │
│    └── Module Types                                         │
│        File as module                                       │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 Type Definitions

```yaoxiang
// Unified type syntax: only constructors, no enum/struct/union keywords
// Rule: Separated by | are constructors, ConstructorName(parameters) is the type

// === Zero-argument constructors (enum style) ===
Color: Type = { red | green | blue } // Equivalent to red() | green() | blue()

// === Multi-argument constructors (struct style) ===
Point: Type = { x: Float, y: Float } // Constructor is the type

// === Generic constructors ===
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) } // Generic union

// === Mixed constructors ===
Shape: Type = { circle(Float) | rect(Float, Float) }

// === Value construction (identical to function call) ===
c: Color = green // Equivalent to green()
p: Point = Point(1.0, 2.0)
r: Result(Int, String) = ok(42)
s: Shape = circle(5.0)

// === Interface definition (record type where all fields are functions) ===
Drawable: Type = {
 draw: (Surface) -> Void,
 bounding_box: () -> Rect
}

Serializable: Type = {
 serialize: () -> String
}

// === Interface implementation (list interface names at the end of the type) ===
Point: Type = {
 x: Float,
 y: Float,
 Drawable, // Implements Drawable interface
 Serializable // Implements Serializable interface
}
```

### 3.3 Type Operations

```yaoxiang
// Types as values
MyInt = Int
MyList = List(Int)

// Type reflection (constructor pattern matching)
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
// Basic inference
x = 42 // Inferred as Int
y = 3.14 // Inferred as Float
z = "hello" // Inferred as String

// Function return value inference
add: (a: Int, b: Int) -> Int = a + b

// Generic inference
first: (T: Type) -> ((list: List(T)) -> Option(T)) = (list) => {
 if list.length > 0 { some(list[0]) } else { none }
}
```

---

## 4. Memory Management

### 4.1 Ownership Model

YaoXiang uses the **ownership model** to manage memory, with each value having a unique owner:

```yaoxiang
// === Default Move (zero-copy) ===
p: Point = Point(1.0, 2.0)
p2 = p // Move, ownership transferred, p is invalid

// === ref keyword = Arc (safe sharing) ===
shared = ref p // Arc, thread-safe

spawn(() => print(shared.x)) // ✅ Safe

// === clone() explicit copy ===
p3 = p.clone() // p and p3 are independent
```

### 4.2 Move Semantics (Default)

```yaoxiang
// Assignment = Move (zero-copy)
p: Point = Point(1.0, 2.0)
p2 = p // Move, p is invalid

// Function parameter passing = Move
process: (p: Point) -> Void = {
 // Ownership of p is transferred in
}

// Return value = Move
create: () -> Point = {
 p = Point(1.0, 2.0)
 return p // Move, ownership transferred
}
```

### 4.3 ref Keyword (Arc)

```yaoxiang
// ref keyword creates Arc (reference counting)
p: Point = Point(1.0, 2.0)
shared = ref p // Arc, thread-safe

spawn(() => print(shared.x)) // ✅ Safe

// Arc automatically manages lifetime
// When shared goes out of scope, the count reaches zero and it's automatically released
```

### 4.4 clone() Explicit Copy

```yaoxiang
// Use clone() when you need to keep the original value
p: Point = Point(1.0, 2.0)
p2 = p.clone() // p and p2 are independent

p.x = 0.0 // ✅
p2.x = 0.0 // ✅ Independent of each other
```

### 4.5 unsafe Code Block (System-Level)

```yaoxiang
// Raw pointers can only be used in unsafe blocks
p: Point = Point(1.0, 2.0)

unsafe {
 ptr: *Point = &p // Raw pointer
 (*ptr).x = 0.0 // User guarantees safety
}
```

### 4.6 RAII

```yaoxiang
// RAII automatic release
with_file: (path: String) -> String = {
 file = File.open(path) // Automatically opened
 content = file.read_all()
 // Function ends, file is automatically closed
 content
}
```

### 4.7 Send / Sync Constraints

| Constraint | Semantics | Description |
|------|------|------|
| **Send** | Can be safely transferred across threads | Value can be moved to another thread |
| **Sync** | Can be safely shared across threads | Immutable references can be shared to another thread |

```yaoxiang
// ref T automatically satisfies Send + Sync (Arc is thread-safe)
p: Point = Point(1.0, 2.0)
shared = ref p

spawn(() => print(shared.x)) // ✅ Arc is thread-safe

// Raw pointer *T does not satisfy Send/Sync
unsafe {
 ptr: *Point = &p // Can only be used in a single thread
}
```

### 4.9 Not Implemented

| Feature | Reason |
|------|------|
| Lifetime `'a` | No reference concept, no lifetime needed |
| Borrow checker | ref = Arc replacement |
| `&T` borrow syntax | Uses Move semantics |

---

## 5. Asynchronous Programming and Concurrency

> "All things operate in parallel, I observe their return." — *I Ching · Fu Hexagram*
>
> YaoXiang adopts the **spawn model**, a transparent asynchronous concurrency paradigm based on **lazy evaluation**. Its core design philosophy is: **let developers describe logic with synchronous, sequential thinking, while the language runtime makes the computational units automatically and efficiently execute concurrently like all things in operation, ultimately coordinating them in unison**.

> See [Spawn Model Whitepaper](YaoXiang-async-whitepaper.md) and [Asynchronous Implementation Plan](YaoXiang-async-implementation.md) for details.

### 5.1 Core Concepts of the Spawn Model

#### 5.1.1 Spawn Graph: The Stage for All Things Operating in Parallel

All programs are transformed into a **Directed Acyclic Graph (DAG)** at compile time, called the **spawn graph**. Nodes represent expression computations, and edges represent data dependencies. This graph is lazy, meaning nodes are only evaluated when their output is **truly needed**.

```yaoxiang
// Compiler automatically constructs the spawn graph
fetch_user: spawn () -> User = (id) => { ... }
fetch_posts: spawn (User) -> Posts = (user) => { ... }

main:() -> Void = () => {
 user = fetch_user(1) // Node A (Async(User))
 posts = fetch_posts(user) // Node B (Async(Posts)), depends on A

 // Node C needs results from A and B
 print(posts.title) // Automatic wait: ensure A and B are completed first
}
```

#### 5.1.2 Spawn Value: Async(T)

Any function call marked as `spawn fn` immediately returns a value of type `Async(T)`, called a **spawn value**. This is a lightweight proxy, not the actual result, but represents a **future value that is currently operating in parallel**.

**Core Properties**:
- **Type transparent**: `Async(T)` is a subtype of `T` in the type system and can be used in any context expecting `T`
- **Automatic wait**: When the program executes an operation that must use the concrete value of type `T`, the runtime automatically suspends the current task and waits for the computation to complete
- **Zero contagion**: Asynchronous code and synchronous code have no difference in syntax and type signatures

```yaoxiang
// Example of using spawn values
fetch_data: spawn (String) -> JSON = (url) => { ... }

main: () -> Void = () => {
 data = fetch_data("url") // Async(JSON)

 // Async(JSON) can be used directly as JSON
 // Automatic wait happens at field access
 print(data.name) // Equivalent to data.await().name
}
```

### 5.2 Spawn Syntax System

The `spawn` keyword has triple semantics and is the only bridge connecting synchronous thinking with asynchronous implementation:

| Official Term | Syntax Form | Semantics | Runtime Behavior |
|----------|----------|------|------------|
| **Spawn function** | `spawn fn` | Defines a computational unit that can participate in spawn execution | Its call returns `Async(T)` |
| **Spawn block** | `spawn { a(), b() }` | Explicitly declared concurrent domain | Tasks in the block are forced to execute in parallel |
| **Spawn loop** | `spawn for x in xs { ... }` | Data-parallel paradigm | Loop body executes spawnly on all elements |

#### 5.2.1 Spawn Function

```yaoxiang
// Use spawn to mark a spawn function
// Syntax is identical to a normal function, no extra burden

fetch_api: spawn (String) -> JSON = (url) => {
 response = HTTP.get(url)
 JSON.parse(response.body)
}

// Nested spawn calls
process_user: (Int) -> Report = (user_id) => {
 user = fetch_user(user_id) // Async(User)
 profile = fetch_profile(user) // Async(Profile), depends on user
 generate_report(user, profile) // Depends on profile
}
```

#### 5.2.2 Spawn Block

```yaoxiang
// spawn { } - explicit parallel construct
// All expressions in the block execute concurrently as independent tasks

compute_all: (Int, Int) -> (Int, Int, Int) spawn = (a, b) => {
 // Three independent computations execute in parallel
 (x, y, z) = spawn {
 heavy_calc(a), // Task 1
 heavy_calc(b), // Task 2
 another_calc(a, b) // Task 3
 }
 (x, y, z)
}
```

#### 5.2.3 Spawn Loop

```yaoxiang
// spawn for - data-parallel loop
// Each iteration executes as an independent task in parallel

parallel_sum: (Int) -> Int spawn = (n) => {
 total = spawn for i in 0..n {
 fibonacci(i) // Each iteration in parallel
 }
 total
}
```

#### 5.2.4 Data-Parallel Loop

```yaoxiang
// spawn for - data-parallel loop
// Each iteration executes as an independent task in parallel

parallel_sum: (Int) -> Int spawn = (n) => {
 total = spawn for i in 0..n {
 fibonacci(i) // Each iteration in parallel
 }
 total
}

// Matrix multiplication parallelization
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
// No explicit await needed, the compiler automatically inserts wait points

main: () -> Void = () => {
 // Automatic parallelism: two independent requests execute in parallel
 users = fetch_users() // Async(List(User))
 posts = fetch_posts() // Async(List(Post))

 // Wait point is automatically inserted at the "+" operation
 count = users.length + posts.length

 // Field access triggers a wait
 first_user = users[0] // Wait until users are ready
 print(first_user.name)
}

// Wait in conditional branches
process_data: spawn () -> Void = () => {
 data = fetch_data() // Async(Data)

 if data.is_valid { // Wait until data is ready
 process(data)
 } else {
 log("Invalid data")
 }
}
```

### 5.4 Concurrency Control Tools

```yaoxiang
// Wait for all tasks to complete
await_all: (T: Type) -> ((tasks: List(Async(T))) -> List(T)) = {
 // Barrier wait
}

// Wait for any one to complete
await_any: (T: Type) -> ((tasks: List(Async(T))) -> T) = {
 // Returns the result of the first completed one
}

// Timeout control
with_timeout: (T: Type) -> ((task: Async(T), timeout: Duration) -> Option(T)) = {
 // Returns None on timeout
}
```

### 5.5 Cross-Task Sharing: ref Keyword

YaoXiang does not need `Send`/`Sync` traits, `Mutex`, `RwLock`, or other manual synchronization primitives. Ownership + `ref` automatically handles concurrency safety.

#### 5.5.1 ref: Cross-Scope Sharing

```yaoxiang
// ref is the only way to share across scopes
// The compiler automatically chooses Rc (single-task) or Arc (cross-task)

data = load_data()
shared = ref data // Compiler automatically chooses implementation

result = spawn {
 process_a(shared), // Shared reference, cross-task → Arc
 process_b(shared) // Shared reference
}
```

#### 5.5.2 Compiler Auto-Optimization

```
ref data flow analysis:

Does not escape to other tasks → Rc (non-atomic reference counting, low overhead)
Escapes to other tasks      → Arc (atomic reference counting, thread-safe)
```

Users don't need to care whether the underlying implementation is Rc or Arc—`ref` is just shared ownership, that's enough.

#### 5.5.3 Resource Type Auto-Serialization

```yaoxiang
// The compiler tracks the usage of resource types to ensure concurrency safety
// Operations on the same resource are automatically serialized

(a, b) = spawn {
 read_file("data.txt"), // Execute first
 write_file("data.txt", x) // Wait for read to complete
}
```

---

## 6. Module System

### 6.1 Module Definition

```yaoxiang
// Modules use files as boundaries
// Math.yx file
pub pi: Float = 3.14159
pub sqrt: (x: Float) -> Float = { ... }
```

### 6.2 Module Import

```yaoxiang
// Import entire module
use std.io

// Import and rename
use std.io as IO

// Import specific functions
use std.io.{ read_file, write_file }
```

---

## 7. Method Binding and Currying

YaoXiang adopts a **pure functional design**, achieving seamless method calls and currying through advanced binding mechanisms, without introducing keywords like `struct` or `class`.

### 7.1 Core Function Definition

All operations are implemented through regular functions, with the first parameter conventionally being the subject of the operation:

```yaoxiang
// === Point.yx (module) ===

// Unified syntax: constructor is the type
Point: Type = { x: Float, y: Float }

// Core function: first parameter is the subject of the operation
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

// More complex functions
distance_with_scale: (s: Float, p1: Point, p2: Point) -> Float = {
 dx = (p1.x - p2.x) * s
 dy = (p1.y - p2.y) * s
 (dx * dx + dy * dy).sqrt()
}
```

### 7.2 Basic Method Binding

#### 7.2.1 Automatic Binding (MoonBit Style)

YaoXiang supports namespace-based automatic binding, **without any additional declarations**:

```yaoxiang
// === Point.yx ===

Point: Type = { x: Float, y: Float }

// Core function
distance: (a: Point, b: Point) -> Float = {
 dx = a.x - b.x
 dy = a.y - b.y
 (dx * dx + dy * dy).sqrt()
}

// === main.yx ===

use Point

main: () -> Void = {
 p1 = Point(3.0, 4.0)
 p2 = Point(1.0, 2.0)

 // ✅ Automatic binding: directly call the method
 result = p1.distance(p2) // Resolves to distance(p1, p2)
}
```

**Automatic Binding Rules**:
- Functions defined within a module
- If the first parameter type matches the module name
- Then method call syntax is automatically supported

#### 7.2.2 No-Binding Option (Default Behavior)

```yaoxiang
// === Vector.yx ===

Vector: Type = Vector(x: Float, y: Float, z: Float)

// Internal helper function, not intended for automatic binding
dot_product_internal: (a: Vector, b: Vector) -> Float = {
 a.x * b.x + a.y * b.y + a.z * b.z
}

// === main.yx ===

use Vector

main: () -> Void = {
 v1 = Vector(1.0, 0.0, 0.0)
 v2 = Vector(0.0, 1.0, 0.0)

 // ❌ Cannot bind: non-pub functions are not automatically bound
 // v1.dot_product_internal(v2) // Compile error!

 // ✅ Must be called directly (invisible outside the module)
}
```

### 7.3 Position-Based Binding Syntax

YaoXiang provides the **most elegant binding syntax**, using position markers `[n]` to precisely control the binding position:

#### 7.3.1 Basic Syntax

```yaoxiang
// === Point.yx ===

Point: Type = { x: Float, y: Float }

// Core functions
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

// Binding syntax: Type.method = func[position]
// Means: when calling the method, bind the caller to func's [position] parameter

Point.distance = distance[0] // Bind to the 1st parameter
Point.add = add[0] // Bind to the 1st parameter
Point.scale = scale[0] // Bind to the 1st parameter
```

**Semantic Resolution**:
- `Point.distance = distance[0]`
  - The `distance` function has two parameters: `distance(Point, Point)`
  - `[0]` means the caller is bound to the 1st parameter
  - Usage: `p1.distance(p2)` → `distance(p1, p2)`

#### 7.3.2 Multi-Position Joint Binding

```yaoxiang
// === Math.yx ===

// Function: scale, point1, point2, extra1, extra2
calculate: (scale: Float, a: Point, b: Point, x: Float, y: Float) -> Float = { ... }

// === Point.yx ===

Point: Type = { x: Float, y: Float }

// Bind multiple positions
Point.calc1 = calculate[1, 2] // Bind scale and point1
Point.calc2 = calculate[1, 3] // Bind scale and point2
Point.calc3 = calculate[2, 3] // Bind point1 and point2

// === main.yx ===

use Point

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

// 1. Bind [1,2] - remaining 3,4,5
f1 = p1.calc1(2.0) // Bind scale=2.0, point1=p1
// f1 now needs p2, x, y
result1 = f1(p2, 10.0, 20.0) // calculate(2.0, p1, p2, 10.0, 20.0)

// 2. Bind [1,3] - remaining 2,4,5
f2 = p2.calc2(2.0) // Bind scale=2.0, point2=p2
// f2 now needs point1, x, y
result2 = f2(p1, 10.0, 20.0) // calculate(2.0, p1, p2, 10.0, 20.0)

// 3. Bind [2,3] - remaining 1,4,5
f3 = p1.calc3(p2) // Bind point1=p1, point2=p2
// f3 now needs scale, x, y
result3 = f3(2.0, 10.0, 20.0) // calculate(2.0, p1, p2, 10.0, 20.0)
```

#### 7.3.3 Remaining Parameter Filling Order

**Core Rule**: After binding, the remaining parameters are filled in the **original function's order**, skipping the bound positions.

```yaoxiang
// Assume function: func(p1, p2, p3, p4, p5)

// Bind 1st and 3rd parameters
Type.method = func[1, 3]

// When calling:
method(p2_value, p4_value, p5_value)

// Maps to:
func(p1_bound, p2_value, p3_bound, p4_value, p5_value)
// Remaining parameters: 2,4,5 are filled in original order
```

#### 7.3.4 Type Checking Advantages

```yaoxiang
// ✅ Legal binding
Point.distance = distance[0] // distance(Point, Point)
Point.calc = calculate[1, 2] // calculate(scale, Point, Point, ...)

// ❌ Illegal binding (compiler error)
Point.wrong = distance[5] // 5th parameter does not exist
Point.wrong = distance[0] // Parameters start from 1
Point.wrong = distance[1, 2, 3, 4] // Exceeds function parameter count
```

### 7.4 Fine-Grained Control of Curried Binding

```yaoxiang
// === Math.yx ===

distance_with_scale: (scale: Float, a: Point, b: Point) -> Float = { ... }

// === Point.yx ===

Point: Type = { x: Float, y: Float }

// Binding strategy: flexibly control each position
Point.distance = distance[0] // Basic binding
Point.distance_scaled = distance_with_scale[2] // Bind to the 2nd parameter

// === main.yx ===

use Point
use Math

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

// 1. Basic automatic binding
d1 = p1.distance(p2) // distance(p1, p2)

// 2. Bind to different positions
f = p1.distance_scaled(2.0) // Bind 2nd parameter, remaining 1st, 3rd
result = f(p2) // distance_with_scale(2.0, p1, p2)

// 3. Chained binding
d2 = p1.distance(p2).distance_scaled(2.0) // Chained call
```

### 7.5 Complete Binding System

```yaoxiang
// === Point.yx ===

Point: Type = { x: Float, y: Float }

// Core functions
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

// Automatic binding (core)
Point.distance = distance[0]
Point.add = add[0]
Point.scale = scale[0]

// === Math.yx ===

// Global function
multiply_by_scale: (scale: Float, a: Point, b: Point) -> Float = { ... }

// === main.yx ===

use Point
use Math

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

// Usage
d = p1.distance(p2) // distance(p1, p2)
r = p1.add(p2) // add(p1, p2)
s = p1.scale(2.0) // scale(p1, 2.0)

// Global function binding
Point.multiply = multiply_by_scale[2] // Bind to the 2nd parameter
m = p1.multiply(2.0, p2) // multiply_by_scale(2.0, p1, p2)
```

### 7.6 Scope and Rules of Binding

#### 7.6.1 Role of pub

```yaoxiang
// === Point.yx ===

Point: Type = { x: Float, y: Float }

// Non-pub function
internal_distance: (a: Point, b: Point) -> Float = {
 dx = a.x - b.x
 dy = a.y - b.y
 (dx * dx + dy * dy).sqrt()
}

// pub function
pub distance: (a: Point, b: Point) -> Float = {
 dx = a.x - b.x
 dy = a.y - b.y
 (dx * dx + dy * dy).sqrt()
}

// === main.yx ===

use Point

// Automatic binding only works for pub functions
p1.distance(p2) // ✅ distance is pub, can be automatically bound
// p1.internal_distance(p2) // ❌ Not pub, cannot be bound
```

#### 7.6.2 pub Automatic Binding Mechanism

Functions declared with `pub` are automatically bound by the compiler to the types defined in the same file:

```yaoxiang
// === Point.yx ===

Point: Type = { x: Float, y: Float }

// Declared with pub, compiler automatically binds
pub distance: (p1: Point, p2: Point) -> Float = {
 dx = p1.x - p2.x
 dy = p1.y - p2.y
 (dx * dx + dy * dy).sqrt()
}

pub translate: (self: Point, dx: Float, dy: Float) -> Point = {
 Point(self.x + dx, self.y + dy)
}

// Compiler automatically infers and performs binding:
// Point.distance = distance[0]
// Point.translate = translate[0]

// === main.yx ===

use Point

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

// ✅ Functional call
d = distance(p1, p2)

// ✅ OOP syntactic sugar (automatic binding)
d2 = p1.distance(p2)
p3 = p1.translate(1.0, 1.0)
```

**Automatic Binding Rules**:
1. The function is defined in a module file (same file as the type)
2. The function parameters include the type
3. Use `pub` to export
4. The compiler automatically performs `Type.method = function[0]`

**Benefits**:
- No need to manually write binding declarations
- More concise code
- Avoid forgetting or making mistakes in binding

#### 7.6.3 Intra-Module Binding

```yaoxiang
// === Point.yx ===

Point: Type = { x: Float, y: Float }

distance: (a: Point, b: Point) -> Float = {
 dx = a.x - b.x
 dy = a.y - b.y
 (dx * dx + dy * dy).sqrt()
}

// Inside the module, all functions are visible
// But automatic binding only works externally for pub-exported functions

pub distance // Export, automatic binding available externally
```

### 7.7 Summary of Design Advantages

| Feature | Description |
|------|------|
| **Zero syntax burden** | Automatic binding requires no declarations |
| **Precise position control** | `[n]` precisely specifies the binding position |
| **Multi-position binding** | Supports `[1, 2, 3]` multi-parameter binding |
| **Type safety** | Compiler validates the validity of binding positions |
| **No keywords** | No `bind` or other keywords needed |
| **Flexible currying** | Supports binding of arbitrary position parameters |
| **pub control** | Only pub functions can be bound externally |

### 7.8 Difference from Traditional Method Binding

| Traditional Language | YaoXiang |
|---------|----------|
| `obj.method(arg)` | `obj.method(arg)` |
| Requires class/method definition | Only requires function + binding declaration |
| Syntax `class { method() {} }` | Syntax `Type.method = func[n]` |
| Inheritance, polymorphism | Purely functional, no inheritance |
| Method table lookup | Compile-time binding, no runtime overhead |

**Core Advantage**: YaoXiang's binding is a **compile-time mechanism** with zero runtime cost, while maintaining the purity and flexibility of functional programming.

---

## 8. AI-Friendly Design

YaoXiang's syntax design specifically considers the needs of AI code generation and modification:

### 8.1 Design Principles

```yaoxiang
// AI-friendly design goals:
// 1. Strictly structured, unambiguous syntax
// 2. Clear AST, easy to locate
// 3. Clear semantics, no hidden behavior
// 4. Clear code block boundaries
// 5. Complete type information
```

### 8.2 Strictly Structured Syntax

#### 8.2.1 AI-Friendly Strategies for Declaration Syntax

```yaoxiang
// === Best Practices for AI Code Generation ===

// ✅ Recommended: use complete new syntax declarations + centralized type convention
// AI can accurately understand intent and generate complete type information

add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
empty: () -> Void = {}

// ❌ Avoid: omit type annotations or scatter types
// AI cannot determine parameter types and may generate erroneous code
add: (a: Int, b: Int) -> Int = a + b // Parameters have no type
identity: (T: Type) -> ((x: T) -> T) = x // Parameters have no type
add2: (a: Int, b: Int) -> Int = a + b // Types scattered in implementation

// ⚠️ Compatibility: legacy syntax only for maintenance
// AI should prioritize generating new syntax + centralized type convention
mul(Int, Int) -> Int = (a, b) => a * b // Not recommended for new code
```

**AI Advantages of Centralized Type Convention:**

1. **Signature at a glance**: AI only needs to read the declaration line to understand the complete function signature
2. **Safer modification**: Modifying types only requires changing the declaration, without affecting the implementation body
3. **Simpler generation**: AI can generate the declaration first, then fill in the implementation
4. **Currying-friendly**: Clear currying type signatures facilitate AI processing

```yaoxiang
// AI processing example
// Input: implementation body (a, b) => a + b
// AI sees declaration: add: (Int, Int) -> Int
// Conclusion: parameter types are Int, Int, return type is Int

// Comparison: if types are scattered
// Input: implementation body (a: Int, b: Int) => a + b
// AI needs: analyze implementation body to extract type information
// Result: more complex processing logic, prone to errors
```

#### 8.2.2 Dual-Syntax Strategy and AI

| Syntax Type | AI Generation Strategy | Use Case |
|---------|-----------|---------|
| **New syntax** | ✅ Prioritize generation, complete type information | All new code generation |
| **Legacy syntax** | ⚠️ Only use when maintaining legacy code | Legacy code modification |
| **No annotation** | ❌ Avoid generation | Should not be generated in any case |

#### 8.2.3 Clear Syntax Boundaries

```yaoxiang
// AI-friendly code block boundaries

// ✅ Clear start and end markers
function_name: (Type1, Type2) -> ReturnType = (param1, param2) => {
 // Function body
 if condition {
 do_something()
 } else {
 do_other()
 }
}

// ✅ Conditional statements must have curly braces
if condition {
 // Conditional body
}

// ✅ Clear type definitions
MyType: Type = { Type1 | Type2 }

// ❌ Avoid ambiguous writing
if condition // Missing curly braces
 do_something()
```

#### 8.2.4 Unambiguous Syntax Constraints

```yaoxiang
// Constraints that must be followed when AI generates

// 1. Prohibit omitting parentheses
// ✅ Correct
foo: (T: Type) -> ((x: T) -> T) = x
my_list = [1, 2, 3]

// ❌ Error (prohibited)
foo T { T } // Parameters must have parentheses
my_list = [1 2 3] // Lists must have commas

// 2. Must have explicit return type or inferable form
// ✅ Correct
get_num: () -> Int = 42
get_num2: () = 42 // Return type can be inferred
get_void = () => { 42 } // ✓ Inferred as Void (block has no return, defaults to Void)

// 3. Parameters must have type annotations (new syntax)
// ✅ Correct
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1

// ❌ Error
add: (a: Int, b: Int) -> Int = a + b // Parameters have no type
identity: (T: Type) -> ((x: T) -> T) = x // Parameters have no type
```

#### 8.2.5 Recommended AI Generation Patterns

```yaoxiang
// Standard template when AI generates functions

// Pattern 1: Complete type annotation
function_name: (param1: ParamType1, param2: ParamType2, ...) -> ReturnType = {
 // Function body
 return expression
}

// Pattern 2: Return type inference
function_name: (param1: ParamType1, param2: ParamType2) = {
 // Function body
 return expression
}

// Pattern 3: Single parameter shorthand
function_name: (param: ParamType) -> ReturnType = expression

// Pattern 4: No-parameter function
function_name: () -> ReturnType = expression

// Pattern 5: Empty function
function_name: () -> Void = {}
```

### 8.3 AI-Friendliness of Error Messages

```yaoxiang
// Error messages should provide clear correction suggestions

// Unfriendly error
// Syntax error at token 'a'

// AI-friendly error
// Missing type annotation for parameter 'a'
// Suggestion: add ': Int' or similar type to '(a, b) => a + b'
// Correct version: add: (a: Int, b: Int) -> Int = a + b
```

---

## 9. Centralized Type Convention (Core Design Philosophy)

### 9.1 Convention Overview

YaoXiang's core design convention is **"Declaration first, type centralized"**. This convention is the cornerstone of the language's AI-friendliness and development efficiency.

```yaoxiang
// ✅ Core convention: type information is unified in the declaration line
add: (a: Int, b: Int) -> Int = a + b

// ❌ Avoid: type information scattered in implementation
add: (a: Int, b: Int) -> Int = a + b
```

### 9.2 Five Core Advantages of the Convention

#### 1. Syntax Consistency

```yaoxiang
// All declarations follow the same format
x: Int = 42 // Variable
name: String = "YaoXiang" // Variable
add: (a: Int, b: Int) -> Int = a + b // Function
inc: (x: Int) -> Int = x + 1 // Function
Point: Type = { x: Float, y: Float } // Type
```

#### 2. Separation of Declaration and Implementation

```yaoxiang
// Declaration line provides complete type information
add: (a: Int, b: Int) -> Int = a + b
// └────────────────────┘
// Complete function signature

// Implementation body focuses on business logic
// (a, b) => a + b doesn't need to care about types, only needs to implement functionality
```

#### 3. AI-Friendliness

```yaoxiang
// AI processing flow:
// 1. Read declaration line → completely understand function signature
// 2. Generate implementation → no need to analyze type inference
// 3. Modify type → only change declaration line, no impact on implementation

// Comparison: scattered type approach
add: (a: Int, b: Int) -> Int = a + b
// AI needs: analyze implementation body to extract type information → more complex, error-prone
```

#### 4. Safer Modification

```yaoxiang
// Modify parameter type
// Original: add: (a: Int, b: Int) -> Int = a + b
// Modified: add: (Float, Float) -> Float = (a, b) => a + b
// Implementation body: (a, b) => a + b no modification needed!

// If types are scattered:
// Original: add: (a: Int, b: Int) -> Int = a + b
// Modified: add: (a: Float, b: Float) -> Float = a + b // Need to change two places
```

#### 5. Currying-Friendly

```yaoxiang
// Currying types at a glance
add_curried: (a: Int) -> (b: Int) -> Int = a + b
// └─────────────┘
// Currying signature

// Function composition as a first-class citizen
compose: (Int -> Int, Int -> Int) -> Int -> Int = (f, g) => x => f(g(x))
```

### 9.3 Implementation Rules of the Convention

#### Rule 1: Parameters Must Have Types Specified in the Declaration

```yaoxiang
// ✅ Correct
add: (a: Int, b: Int) -> Int = a + b

// ❌ Error
add: (a: Int, b: Int) -> Int = a + b // Parameter types missing
identity: (T: Type) -> ((x: T) -> T) = x // Parameter types missing
```

#### Rule 2: Return Type Can Be Inferred but Annotation is Recommended

```yaoxiang
// ✅ Recommended: complete annotation
get_num: () -> Int = () => 42

// ✅ Acceptable: return type inference
get_num: () = () => 42

// ✅ Empty function inferred as Void
empty: () = () => {}
```

#### Rule 3: Type Annotations Inside Lambda are Temporary

```yaoxiang
// ✅ Correct: rely on types in the declaration
add: (a: Int, b: Int) -> Int = a + b

// ⚠️ Acceptable but not recommended: repeated annotation inside Lambda
add: (Int, Int) -> Int = (a: Int, b: Int) => a + b

// ❌ Error: missing declaration annotation
add: (a: Int, b: Int) -> Int = a + b
```

#### Rule 4: Legacy Syntax Follows the Same Philosophy

```yaoxiang
// Legacy syntax should also try to provide type information at the declaration position
// Although the format is different, the philosophy is consistent:
// - Declaration line contains main type information
// - Implementation body is relatively concise
add(Int, Int) -> Int = (a, b) => a + b
```

### 9.4 Relationship Between the Convention and Type Inference

```yaoxiang
// The convention does not prevent type inference, but guides the direction of inference

// 1. Complete annotation (no inference)
add: (a: Int, b: Int) -> Int = a + b

// 2. Partial inference (declaration provides parameter types)
add: (Int, Int) = (a, b) => a + b // Return type inferred

// 3. Empty function inference
empty: () = () => {} // Inferred as () -> Void
```

### 9.5 AI Implementation Advantages of the Convention

**AI Code Generation Flow:**

1. **Read requirements** → Generate declaration
   ```
   Requirement: addition function
   Generated: add: (Int, Int) -> Int = (a, b) => ???
   ```

2. **Fill implementation** → No type analysis needed
   ```
   Implementation: add: (a: Int, b: Int) -> Int = a + b
   ```

3. **Type modification** → Only change declaration
   ```
   Modification: add: (Float, Float) -> Float = (a, b) => a + b
   Implementation: (a, b) => a + b  remains unchanged
   ```

**Comparison with AI Processing Without the Convention:**
```
Requirement: addition function
AI needs to:
  1. Infer parameter types
  2. Infer return type
  3. Generate implementation body
  4. Verify consistency
  5. Handle complex updates when types change

Result: more complex, more error-prone
```

### 9.6 Philosophical Significance of the Convention

This convention embodies YaoXiang's core philosophy:

- **Declaration is documentation**: The declaration line is the complete function documentation
- **Type is contract**: Type information is the contract between caller and implementer
- **Logic is implementation**: The implementation body only cares about "what to do", not "what type"
- **Tool is auxiliary**: Type systems, AI tools can all work based on clear declarations

### 9.7 Practical Application Comparison

#### Complete Example: Calculator Module

```yaoxiang
// === Recommended: Centralized Type Convention ===

// Module declaration
pub add: (a: Int, b: Int) -> Int = a + b
pub multiply: (a: Int, b: Int) -> Int = a * b

// Higher-order function
pub apply_twice: (f: Int -> Int, x: Int) -> Int = f(f(x))

// Curried function
pub make_adder: (x: Int) -> (Int) -> Int = y => x + y

// Generic function
pub map: (A: Type, B: Type) -> ((f: (A) -> B, xs: List(A)) -> List(B)) = case xs of
 [] => []
 (x :: rest) => f(x) :: map(f, rest)

// Type definition
Point: Type = { x: Float, y: Float }
pub distance: (a: Point, b: Point) -> Float = {
 dx = a.x - b.x
 dy = a.y - b.y
 (dx * dx + dy * dy).sqrt()
}

// === Not Recommended: Scattered Types ===

// Parameter types in Lambda
add: (a: Int, b: Int) -> Int = a + b
multiply = (a: Int, b: Int) => a * b

// Higher-order function types scattered
apply_twice = (f: (Int) -> Int, x: Int) => f(f(x))

// Currying types scattered
make_adder = (x: Int) => (y: Int) => x + y

// Generic types scattered
map: (A: Type, B: Type) -> ((f: (A) -> B, xs: List(A)) -> List(B)) = case xs of
 [] => []
 (x :: rest) => f(x) :: map(f, rest)
```

#### Code Maintenance Comparison

```yaoxiang
// Requirement: change add from Int to Float

// === Recommended: only modify the declaration line ===
// Original
add: (a: Int, b: Int) -> Int = a + b

// After modification
add: (a: Float, b: Float) -> Float = a + b
// ↑↑↑↑↑↑↑↑↑ ↑↑↑↑↑↑↑
// Declaration line modified  Implementation body unchanged

// === Not Recommended: need to modify multiple places ===
// Original
add: (a: Int, b: Int) -> Int = a + b

// After modification
add: (a: Float, b: Float) -> Float = a + b
// ↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑
// All parameter types need to be modified
```

#### AI-Assisted Programming Comparison

```yaoxiang
// AI requirement: implement a function that computes Manhattan distance between two points

// === When AI sees the recommended writing ===
Point: Type = { x: Float, y: Float }
pub manhattan: (a: Point, b: Point) -> Float = ??? // AI directly knows the complete signature

// AI generates:
pub manhattan: (a: Point, b: Point) -> Float = {
 (a.x - b.x).abs() + (a.y - b.y).abs()
}

// === When AI sees the not-recommended writing ===
Point: Type = { x: Float, y: Float }
pub manhattan = ??? // AI needs to infer: parameter types? return type?

// AI might generate:
pub manhattan = (a: Point, b: Point) => Float => {
 (a.x - b.x).abs() + (a.y - b.y).abs()
}
// Or might error out, because type information is incomplete
```

### 9.8 Convention Implementation Checklist

When writing YaoXiang code, you can use the following checklist:

- [ ] All function declarations have complete type annotations on the declaration line
- [ ] Parameter types are specified in the declaration, not in the Lambda
- [ ] Return types are annotated in the declaration as much as possible
- [ ] Variable declarations use the `name: Type = value` format
- [ ] Lambda bodies remain concise, without repeating type information
- [ ] Use new syntax instead of legacy syntax
- [ ] Complex types use type definitions, keeping declarations clear

---

## 10. Quick Start

### 10.1 Hello World

```yaoxiang
// hello.yx
use std.io

main: () -> Void = {
 print("Hello, YaoXiang!")
}
```

Run with: `yaoxiang hello.yx`

Output:
```
Hello, YaoXiang!
```

### 10.2 Basic Syntax

```yaoxiang
// Variables and types
x = 42 // Automatically inferred as Int
name = "YaoXiang" // Automatically inferred as String
pi = 3.14159 // Automatically inferred as Float

// Function (using new syntax)
add: (a: Int, b: Int) -> Int = a + b

// Conditionals
if x > 0 {
 "positive"
} elif x == 0 {
 "zero"
} else {
 "negative"
}

// Loops
for i in 0..10 {
 print(i)
}
```

### 10.3 Method Binding Example

```yaoxiang
// === Point.yx ===

Point: Type = { x: Float, y: Float }

// Core function
distance: (a: Point, b: Point) -> Float = {
 dx = a.x - b.x
 dy = a.y - b.y
 (dx * dx + dy * dy).sqrt()
}

// Automatic binding
Point.distance = distance[0]

// === main.yx ===

use Point

main: () -> Void = {
 p1 = Point(3.0, 4.0)
 p2 = Point(1.0, 2.0)

 // Use binding
 d = p1.distance(p2) // distance(p1, p2)
 print(d)
}
```

### 10.4 Curried Binding Example

```yaoxiang
// === Math.yx ===

distance_with_scale: (scale: Float, a: Point, b: Point) -> Float = {
 dx = (p1.x - p2.x) * scale
 dy = (p1.y - p2.y) * scale
 (dx * dx + dy * dy).sqrt()
}

// === Point.yx ===

Point: Type = { x: Float, y: Float }

Point.distance_scaled = distance_with_scale[2] // Bind to the 2nd parameter

// === main.yx ===

use Point

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

// Use binding
f = p1.distance_scaled(2.0) // Bind scale and p1
result = f(p2) // Final call

// Or use directly
result2 = p1.distance_scaled(2.0, p2)
```

### 10.5 Next Steps

- Read the [Language Specification](./YaoXiang-language-specification.md) for complete syntax
- Check [Example Code](./examples/) to learn common patterns
- Refer to the [Implementation Plan](./YaoXiang-implementation.md) for technical details

---

## Appendix

### A. Keywords and Annotations

| Keyword | Purpose |
|--------|------|
| `type` | Type definition |
| `pub` | Public export |
| `use` | Import module |
| `spawn` | Asynchronous marker (function/block/loop) |
| `ref` | Immutable reference |
| `mut` | Mutable reference |
| `if/elif/else` | Conditional branches |
| `match` | Pattern matching |
| `while/for` | Loops |
| `return/break/continue` | Control flow |
| `as` | Type conversion |
| `in` | Member access |

### B. Design Inspirations

- **Rust**: Ownership model, zero-cost abstractions
- **Python**: Syntax style, readability
- **Idris/Agda**: Dependent types, type-driven development
- **TypeScript**: Type annotations, runtime types

---

## Version History

| Version | Date | Author | Change Description |
|------|------|------|---------|
| v1.0.0 | 2024-12-31 | Chenxu (晨煦) | Initial version |
| v1.1.0 | 2025-01-04 | Moyujiang (沫郁酱) | Corrected generic syntax to `[T]` (instead of `<T>`); removed `fn` keyword; updated function definition examples |
| v1.2.0 | 2025-01-06 | Chenxu (晨煦) | Unified into new syntax format: name: type -> type = lambda |
| v1.3.0 | 2025-01-20 | Chenxu (晨煦) | Added unified type syntax (RFC-010): interface definitions use curly braces `{ serialize: () -> String }`; list interface names at the end of types to implement interfaces; `pub` automatic binding mechanism |

---

> "The Dao gives birth to One, One gives birth to Two, Two gives birth to Three, Three gives birth to all things."
> — *Tao Te Ching*
>
> Types are like the Dao, from which all things are born.