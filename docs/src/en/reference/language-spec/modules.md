# Module System Specification

This document defines the module system specification for the YaoXiang programming language, including module definitions, imports, exports, and more.

---

## Chapter 1: Module Definition

### 1.1 Module Basics

Modules use files as boundaries. Each `.yx` file is a module.

```
// File name is the module name
// Math.yx
pub pi: Float = 3.14159
pub sqrt(Float) -> Float = (x) => { ... }
```

### 1.2 Module Naming Rules

- Module names are determined by file names
- File extension `.yx` is not part of the module name
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
|--------|-------------|---------|
| `use path;` | Import module, access via last segment | `use std.io;` -> `io.print` |
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

- By default, all items are private
- Items declared with `pub` can be accessed by other modules
- Private items can only be accessed within the current module

### 3.3 pub Automatic Binding

For functions declared with `pub`, the compiler automatically binds them to types defined in the same file:

```yaoxiang
// Declared with pub, compiler binds automatically
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

// Compiler automatically infers:
// 1. Point is defined in the current file
// 2. Function parameters include Point
// 3. Execute Point.distance = distance[0]

// Invocation
d = distance(p1, p2)           // Functional style
d2 = p1.distance(p2)          // OOP syntax sugar
```

---

## Chapter 4: Scopes

### 4.1 Module Scope

Each module has its own scope. Items within a module are not visible externally by default.

### 4.2 Nested Scopes

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

### 4.3 Variable Declaration and Shadowing

YaoXiang has no `let` keyword. For `x = value`, is it a declaration or an assignment? Following one principle:

**Assignment takes priority.** Declaration happens only once, while assignment happens a hundred times. Let the high-frequency operation take the shortest path.

```
x = value:
  Search for x along the scope chain
    → Found mut x    : assignment, OK (via &mut token)
    → Found x (immutable): E2010 cannot reassign
    → Not found       : new declaration in current scope (the only declaration path)

mut x = value:
    → x already exists in current scope : E2002 duplicate definition
    → x exists in outer scope           : E2013 shadowing prohibited (explicit new declaration cannot have same name as outer)
    → No conflict                       : new mutable declaration
```

- **Same scope**: Any name can only be declared once (E2002)
- **Inner scope without `mut`**: Prioritize searching outer scope, assign or error
- **Inner scope with `mut`**: Explicit new declaration, cannot share name with outer scope (E2013)

#### Same Scope

```yaoxiang
x = 10
x = 20              // E2002: 'x' is already defined in this scope

mut y = 10
y = 20              // OK: same binding, reassignment
mut y = 30          // E2002: 'y' is already defined in this scope

z = 10
mut z = 20          // E2002: 'z' is already defined in this scope (mut cannot override existing declaration)
```

#### Cross Scope

```yaoxiang
// Outer immutable, inner assignment → immutable variable cannot be reassigned
x = 10
{
    x = 20          // E2010: 'x' is immutable, cannot be reassigned
}
{
    mut x = 20      // E2013: cannot shadow existing variable 'x' (explicit declaration creates new binding)
}

// Outer mut, inner assignment → modify same binding
mut y = 10
{
    y = 20          // OK: same binding, modified via &mut token
}
print(y)            // 20

// Outer mut, inner cannot declare same name
mut z = 10
{
    z = 30          // OK: same binding
}
{
    mut z = 30      // E2013: cannot shadow existing variable 'z'
}

// Multiple nesting: mut penetrates all levels
mut a = 0
{
    {
        a = 10      // OK
    }
}
print(a)            // 10

// Immutability penetrates all levels, still cannot reassign
b = 0
{
    {
        b = 10      // E2010: 'b' is immutable, cannot be reassigned
    }
}
```

#### for Loops

```yaoxiang
// Loop variable is a new binding each iteration, not a modification
for i in 1..5 {
    print(i)        // OK: binding new value each iteration
    i = 10          // E2010: immutable loop variable, cannot be reassigned
}

for mut i in 1..5 {
    i = 10          // OK: mutable loop variable
}

// Loop variable cannot shadow outer
i = 0
for i in 1..5 {     // E2013: cannot shadow existing variable 'i'
}

// mut outer accumulator can be modified in loop body
mut sum = 0
for i in 1..5 {
    sum = sum + i   // OK: same binding, modified via &mut token
}
print(sum)          // 15

// Immutable outer cannot be modified in loop body
sum2 = 0
for i in 1..5 {
    sum2 = sum2 + i // E2010: 'sum2' is immutable, cannot be reassigned
}
```

#### Related Error Codes

| Error Code | Message | Trigger Scenario |
|------------|---------|------------------|
| E2002 | `'{name}' is already defined in this scope` | Duplicate declaration in same scope (regardless of mut) |
| E2010 | `Cannot assign to immutable variable '{name}'` | Inner scope without `mut` assigns to outer immutable variable |
| E2013 | `Cannot shadow existing variable '{name}'` | Inner explicit declaration (`mut x` or `x: Type`) shares name with outer |

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

### 5.2 Module Entry Point

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

## Appendix: Module Syntax Cheat Sheet

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