# Module System Specification

This document defines the module system specification of the YaoXiang programming language, including module definition, import/export, and scope.

---

## Chapter 1: Module Definition

### 1.1 Module Basics

Modules use files as boundaries. Each `.yx` file is a module.

```
// File name is the module name
// Math.yx
pub pi: Float = 3.14159
pub sqrt: (x: Float) -> Float = (x) => { ... }
```

### 1.2 Module Naming Rules

- Module name is determined by the file name
- The file extension `.yx` is not part of the module name
- Module names use PascalCase

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

### 2.2 Import Methods

| Syntax | Description | Example |
|------|------|------|
| `use path;` | Import the module, access via the last part | `use std.io;` -> `io.print` |
| `use path.{a, b};` | Import specific items | `use std.io.{print};` -> `print` |
| `use path as alias;` | Import and rename | `use std.io as io;` -> `io.print` |
| `use path.{i1, i2} as a, b;` | Import specific items and rename | `use std.io.{print, read} as p, r;` -> `p`, `r` |

### 2.3 Import Examples

```yaoxiang
// Import the entire module
use std.io
io.print("Hello")

// Import specific items
use std.io.{print, read}
print("Hello")

// Import and rename
use std.io as io_module
io_module.print("Hello")

// Import specific items and rename
use std.io.{print, read} as p, r
p("Hello")
```

---

## Chapter 3: Module Export

### 3.1 The `pub` Keyword

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

### 3.3 Automatic `pub` Binding

For functions declared with `pub`, the compiler automatically binds them to the type defined in the same file:

```yaoxiang
// Declared with pub, the compiler binds automatically
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

// Compiler infers automatically:
// 1. Point is defined in the current file
// 2. Function parameters contain Point
// 3. Executes Point.distance = distance[0]

// Invocation
d = distance(p1, p2)           // Functional
d2 = p1.distance(p2)           // OOP syntactic sugar
```

---

## Chapter 4: Scope

### 4.1 Module Scope

Each module has its own scope; items within a module are not visible to the outside by default.

### 4.2 Nested Scopes

```yaoxiang
// Block scope
{
    x = 10
    // x is visible within this scope
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

YaoXiang has no `let` keyword. Is `x = value` a declaration or an assignment? One principle is followed:

**Assignment takes priority.** Declaration happens only once, but assignment happens hundreds of times. Let high-frequency operations take the shortest path.

```
x = value:
  Search outward along the scope chain for x
    → Found mut x          : assignment, OK (via &mut token)
    → Found x (moved)      : treated as "no valid binding found", re-declare in current scope
    → Found x (immutable, alive) : E2010 cannot reassign
    → Not found             : new declaration in current scope (the only declaration path)

mut x = value:
    → x already exists in current scope : E2002 duplicate definition
    → x exists in outer scope            : E2013 shadowing prohibited (explicit new declaration cannot share name with outer)
    → No conflict                         : new mutable declaration
```

- **Same scope**: any name can only be declared once (E2002)
- **Inner without `mut`**: priority lookup in outer scope, assign or error
- **Inner with `mut`**: explicit new declaration, prohibited from sharing name with outer (E2013)

#### Same Scope

```yaoxiang
x = 10
x = 20              // E2002: 'x' is already defined in this scope

mut y = 10
y = 20              // OK: same binding, reassign
mut y = 30          // E2002: 'y' is already defined in this scope

z = 10
mut z = 20          // E2002: 'z' is already defined in this scope (mut cannot overwrite existing declaration)
```

#### Rebinding After Move

If an immutable variable owns the value, when its value is moved (consumed), the original binding enters the **moved** state—the name still occupies the scope slot, but the value is no longer accessible. At this point, `x = value` is not modifying the old binding, but re-declaring `x` in the same scope.

```
The "moved" branch for assignment-priority lookup:
  x exists in the current scope, but is in the moved state
    → Compiler treats it as "no valid binding found"
    → Re-declare x in the current scope (overwrites the old moved slot)
```

**Core mechanism:** After the old value is consumed, the binding becomes invalid, and the name returns to a "declarable" state. This is not shadowing—the old binding no longer exists.

```yaoxiang
// Pipeline-style data flow: each step consumes the old value and produces a new one
data = fetch()           // Immutable, holds ownership
data = transform(data)   // move data → old data invalidated, new data rebound
data = filter(data)      // Same as above
process(data)

// Equivalent explicit writing (comparison):
data1 = fetch()
data2 = transform(data1)  // data1 is moved, cannot be used again
data3 = filter(data2)     // data2 is moved, cannot be used again
process(data3)
```

**Semantic separation:**

| Operation | Meaning | Mechanism | Syntax |
|------|------|------|------|
| **Rebinding** | Old value disappears, new value is born | move + re-declare | `x = f(x)` |
| **In-place modification** | The value at the same memory location changes | mut assignment | `mut x; x = v` |

**Why this differs from shadowing:**
- Shadowing (Rust's `let x = ...`): the old binding still exists, it's just covered by the new binding
- Rebinding after move: the old binding has been consumed, the name returns to an uninitialized state, re-declaration is the only way out

**Constraints:**
- Only values that own can be moved. References (`&T`, `&mut T`) are copied, not moved
- Move checking is done at compile-time; reading a variable in the moved state in any expression raises E2014
- IDE can display a grayed-out hint on moved variables, indicating that the name is in an uninitialized state

```yaoxiang
// Reading after move → error
data = fetch()
result = process(data)   // data is moved
print(data)              // E2014: 'data' has been moved and cannot be used

// References do not trigger move
ref_data = &value
copy1 = ref_data         // Copy the reference, ref_data is still usable
copy2 = ref_data         // OK

// Cross-scope: the moved state penetrates
data = fetch()
{
    data = transform(data)  // move outer data → rebind (new declaration in inner)
    print(data)             // OK: use the inner data
}
print(data)                 // E2014: outer data has been moved
```

#### Cross-Scope

```yaoxiang
// Outer immutable, inner assignment → immutable variable cannot be reassigned
x = 10
{
    x = 20          // E2010: 'x' is immutable and cannot be reassigned
}
{
    mut x = 20      // E2013: cannot shadow existing variable 'x' (explicit declaration of new binding)
}

// Outer mut, inner assignment → modify the same binding
mut y = 10
{
    y = 20          // OK: same binding, modify via &mut token
}
print(y)            // 20

// Outer mut, inner cannot declare the same name
mut z = 10
{
    z = 30          // OK: same binding
}
{
    mut z = 30      // E2013: cannot shadow existing variable 'z'
}

// Multi-level nesting: mut penetrates all levels
mut a = 0
{
    {
        a = 10      // OK
    }
}
print(a)            // 10

// Immutable penetration through all levels cannot be reassigned either
b = 0
{
    {
        b = 10      // E2010: 'b' is immutable and cannot be reassigned
    }
}
```

#### `for` Loops

```yaoxiang
// Loop variable is a new binding each iteration, not modification
for i in 1..5 {
    print(i)        // OK: new value bound each iteration
    i = 10          // E2010: immutable loop variable, cannot be reassigned
}

for mut i in 1..5 {
    i = 10          // OK: mutable loop variable
}

// Loop variable cannot shadow the outer
i = 0
for i in 1..5 {     // E2013: cannot shadow existing variable 'i'
}

// Mutable outer accumulator can be modified inside the loop body
mut sum = 0
for i in 1..5 {
    sum = sum + i   // OK: same binding, modify via &mut token
}
print(sum)          // 15

// Immutable outer cannot be modified inside the loop body
sum2 = 0
for i in 1..5 {
    sum2 = sum2 + i // E2010: 'sum2' is immutable and cannot be reassigned
}
```

#### Related Error Codes

| Error Code | Message | Trigger Scenario |
|--------|------|----------|
| E2002 | `'{name}' is already defined in this scope` | Duplicate declaration in the same scope (whether mut or not) |
| E2010 | `Cannot assign to immutable variable '{name}'` | Inner without `mut` assignment, outer variable is immutable and not moved |
| E2013 | `Cannot shadow existing variable '{name}'` | Inner explicit declaration (`mut x` or `x: Type`) shares name with outer |
| E2014 | `'{name}' has been moved and cannot be used` | Reading a variable that has been moved |

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
    ├── index.yx     // Utilities module entry
    └── string.yx    // String utilities
```

### 5.2 Module Entry

The `index.yx` file in a directory serves as the module entry:

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

## Appendix: Module Syntax Quick Reference

### A.1 Module Equals File

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