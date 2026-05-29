# Module System Specification

This document defines the module system specification for the YaoXiang programming language, including module definition, imports/exports, and scope.

---

## Chapter 1: Module Definition

### 1.1 Module Basics

A module is bounded by a file. Each `.yx` file is a module.

```
// The filename is the module name
// Math.yx
pub pi: Float = 3.14159
pub sqrt(Float) -> Float = (x) => { ... }
```

### 1.2 Module Naming Rules

- Module names are determined by the filename
- The file extension `.yx` is not part of the module name
- Module names use PascalCase naming

---

## Chapter 2: Module Import

### 2.1 Import Syntax

```
Import       ::= 'use' ModuleRef ImportSpec?
ImportSpec   ::= ('{' ImportItems '}') ('as' AliasList)?
              |  'as' AliasList
ImportItems  ::= Identifier (',' Identifier)* ','?
AliasList    ::= Identifier (',' Identifier)*
```

### 2.2 Import Styles

| Syntax | Description | Example |
|------|------|------|
| `use path;` | Import module, access via last part | `use std.io;` -> `io.print` |
| `use path.{a, b};` | Import specified items | `use std.io.{print};` -> `print` |
| `use path as alias;` | Import and rename | `use std.io as io;` -> `io.print` |
| `use path.{i1, i2} as a, b;` | Import specified items and rename | `use std.io.{print, read} as p, r;` -> `p`, `r` |

### 2.3 Import Examples

```yaoxiang
// Import entire module
use std.io
io.print("Hello")

// Import specified items
use std.io.{print, read}
print("Hello")

// Import and rename
use std.io as io_module
io_module.print("Hello")

// Import specified items and rename
use std.io.{print, read} as p, r
p("Hello")
```

---

## Chapter 3: Module Export

### 3.1 pub Keyword

Use the `pub` keyword to declare exported items:

```yaoxiang
// Math.yx
pub pi: Float = 3.14159
pub sqrt: (x: Float) -> Float = (x) => { ... }

// Private item (not exported)
internal_value: Int = 42
```

### 3.2 Export Rules

- All items are private by default
- Items declared with `pub` can be accessed by other modules
- Private items can only be accessed within the current module

### 3.3 pub Auto Binding

When a function is declared with `pub`, the compiler automatically binds it to types defined in the same file:

```yaoxiang
// Using pub declaration, compiler auto-binds
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

// Compiler auto-infers:
// 1. Point is defined in current file
// 2. Function parameters include Point
// 3. Executes Point.distance = distance[0]

// Call
d = distance(p1, p2)           // Functional style
d2 = p1.distance(p2)           // OOP syntax sugar
```

---

## Chapter 4: Scope

### 4.1 Module Scope

Each module has its own scope; items within a module are not visible externally by default.

### 4.2 Nested Scope

```yaoxiang
// Block scope
{
    x = 10
    // x is visible in this scope
}
// x is not visible outside this scope

// Function scope
add: (a: Int, b: Int) -> Int = {
    result = a + b
    return result
}
// result is not visible outside the function
```

### 4.3 Shadowing Rules

```yaoxiang
// Variable shadowing
x = 10
x = 20  // Shadows the previous x

// Scope shadowing
x = 10
{
    x = 20  // Shadows the outer x
    // x is 20 here
}
// x is 10 here
```

---

## Chapter 5: Module Organization

### 5.1 Directory Structure

```
src/
├── main.yx          // Main module
├── math/
│   ├── index.yx     // Math module entry
│   ├── vector.yx    // Vector module
│   └── matrix.yx    // Matrix module
└── utils/
    ├── index.yx     // Utils module entry
    └── string.yx    // String utilities
```

### 5.2 Module Entry

The `index.yx` file in a directory serves as the module entry point:

```yaoxiang
// math/index.yx
use math.vector
use math.matrix

pub Vector = vector.Vector
pub Matrix = matrix.Matrix
```

### 5.3 Relative Import

```yaoxiang
// In math/vector.yx
use math.matrix  // Absolute import
use .matrix      // Relative import (same directory)
```

---

## Appendix: Module Syntax Cheatsheet

### A.1 Module Is File

```
// filename.yx is the module name
Import ::= 'use' ModuleRef
```

### A.2 Import and Export

```yaoxiang
// Import
use std.io
use std.io.{print, read}
use std.io as io

// Export
pub pi: Float = 3.14159
pub sqrt: (x: Float) -> Float = (x) => { ... }
```