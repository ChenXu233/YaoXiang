# Module System Specification

This document defines the module system specification for the YaoXiang programming language, including module definition, import/export, and scope.

---

## Chapter 1: Module Definition

### 1.1 Module Basics

Modules are bounded by files. Each `.yx` file is a module.

```yaoxiang
// File name is the module name
// Math.yx
pub pi: Float = 3.14159
pub sqrt(Float) -> Float = (x) => { ... }
```

### 1.2 Module Naming Rules

- Module names are determined by the file name
- The file extension `.yx` is not part of the module name
- Module names use PascalCase naming convention

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
|--------|-------------|---------|
| `use path;` | Import module, access via last segment | `use std.io;` -> `io.print` |
| `use path.{a, b};` | Import specified items | `use std.io.{print};` -> `print` |
| `use path as alias;` | Import and rename | `use std.io as io;` -> `io.print` |
| `use path.{i1, i2} as a, b;` | Import and rename specified items | `use std.io.{print, read} as p, r;` -> `p`, `r` |

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
- Private items are only accessible within the current module

### 3.3 pub Auto-Binding

For functions declared with `pub`, the compiler automatically binds them to types defined in the same file:

```yaoxiang
// Using pub declaration, compiler binds automatically
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

// Compiler inference:
// 1. Point is defined in the current file
// 2. Function parameters include Point
// 3. Execute Point.distance = distance[0]

// Invocation
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

### 4.3 Variable Declaration and Shadowing

YaoXiang has no `let` keyword. Is `x = value` a declaration or an assignment? The principle is:

**Assignment takes priority.** Declaration happens only once, but assignment happens a hundred times. Let the high-frequency operation take the shortest path.

```
x = value:
  Search for x along the scope chain
    → Found mut x          : assign, OK (via &mut token)
    → Found x (already moved) : treat as "no valid binding found", redeclare in current scope
    → Found x (immutable, alive): E2010 cannot reassign
    → Not found            : new declaration in current scope (the only declaration path)

mut x = value:
    → x already exists in current scope : E2002 duplicate definition
    → x exists in outer scope   : E2013 shadowing prohibited (explicit new declaration cannot share name with outer)
    → No conflict              : new mutable declaration
```

- **Same scope**: Any name can only be declared once (E2002)
- **Inner without `mut`**: Prioritize searching outer, assign or error
- **Inner with `mut`**: Explicit new declaration, shadowing with same name prohibited (E2013)

#### Same Scope

```yaoxiang
x = 10
x = 20              // E2002: 'x' is already defined in this scope

mut y = 10
y = 20              // OK: same binding, reassign
mut y = 30          // E2002: 'y' is already defined in this scope

z = 10
mut z = 20          // E2002: 'z' is already defined in this scope (mut cannot override existing declaration)
```

#### Re-binding After Move

If an immutable variable owns its value, when its value is moved (consumed), the original binding enters **moved** state — the name still occupies the scope slot, but the value is no longer accessible. At this point, `x = value` is not modifying the old binding, but redeclaring `x` in the same scope.

```
The "moved" branch in assignment's priority search:
  x exists in current scope but is in moved state
    → Compiler treats as "no valid binding found"
    → Redeclare x in current scope (overriding the old moved slot)
```

**Core mechanism:** After the old value is consumed, the binding is invalidated, and the name returns to an "undeclared" state. This is not shadowing — the old binding no longer exists.

```yaoxiang
// Pipeline-style data flow: each step consumes old value, produces new value
data = fetch()           // immutable, owns the value
data = transform(data)   // move data → old data invalidated, new data rebound
data = filter(data)      // same
process(data)

// Equivalent explicit style (comparison):
data1 = fetch()
data2 = transform(data1)  // data1 is moved, cannot be used again
data3 = filter(data2)    // data2 is moved, cannot be used again
process(data3)
```

**Semantic distinction:**

| Operation | Meaning | Mechanism | Syntax |
|-----------|---------|-----------|--------|
| **Re-binding** | Old value disappears, new value born | move + redeclaration | `x = f(x)` |
| **In-place modification** | Value at same memory location changes | mut assignment | `mut x; x = v` |

**Why this differs from shadowing:**
- Shadowing (Rust's `let x = ...`): old binding still exists, just obscured by new binding
- Re-binding after move: old binding has been consumed, name returns to uninitialized state, redeclaration is the only way

**Constraints:**
- Only owning values can be moved. References (`&T`, `&mut T`) are copied instead of moved
- Move checking is done at compile time; reading a variable in moved state in any expression reports E2014
- IDE can display gray hints on moved variables, indicating the name is in an uninitialized state

```yaoxiang
// Read after move → error
data = fetch()
result = process(data)   // data is moved
print(data)              // E2014: 'data' has been moved, cannot be used again

// References do not trigger move
ref_data = &value
copy1 = ref_data         // copy reference, ref_data still usable
copy2 = ref_data         // OK

// Cross-scope: moved state propagates
data = fetch()
{
    data = transform(data)  // move outer data → rebind (inner new declaration)
    print(data)             // OK: using inner data
}
print(data)                 // E2014: outer data has been moved
```

#### Cross-Scope

```yaoxiang
// Outer immutable, inner assign → immutable variable cannot be reassigned
x = 10
{
    x = 20          // E2010: 'x' is immutable, cannot reassign
}
{
    mut x = 20      // E2013: cannot shadow existing variable 'x' (explicit new binding)
}

// Outer mut, inner assign → modify same binding
mut y = 10
{
    y = 20          // OK: same binding, modify via &mut token
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

// Multi-layer nesting: mut propagates through all levels
mut a = 0
{
    {
        a = 10      // OK
    }
}
print(a)            // 10

// Immutability also propagates, cannot reassign
b = 0
{
    {
        b = 10      // E2010: 'b' is immutable, cannot reassign
    }
}
```

#### for Loops

```yaoxiang
// Loop variable is a new binding each iteration, not a modification
for i in 1..5 {
    print(i)        // OK: bind new value each iteration
    i = 10          // E2010: immutable loop variable, cannot reassign
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
    sum = sum + i   // OK: same binding, modify via &mut token
}
print(sum)          // 15

// Immutable outer cannot be modified in loop body
sum2 = 0
for i in 1..5 {
    sum2 = sum2 + i // E2010: 'sum2' is immutable, cannot reassign
}
```

#### Related Error Codes

| Error Code | Message | Trigger Scenario |
|------------|---------|------------------|
| E2002 | `'{name}' is already defined in this scope` | Duplicate declaration in same scope (regardless of mut) |
| E2010 | `Cannot assign to immutable variable '{name}'` | Inner assign without `mut`, outer variable is immutable and not moved |
| E2013 | `Cannot shadow existing variable '{name}'` | Inner explicit declaration (`mut x` or `x: Type`) shares name with outer |
| E2014 | `'{name}' has been moved and cannot be used` | Reading a moved variable |

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

## Appendix: Module Syntax Quick Reference

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