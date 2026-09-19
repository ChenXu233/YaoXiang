# Module System Specification

This document defines the module system specification for the YaoXiang programming language,
including module definitions, imports/exports, and scope.

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

- The module name is determined by the file name
- The file extension `.yx` does not participate in the module name
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

### 2.2 Import Methods

| Syntax                       | Description                         | Example                                         |
| ---------------------------- | ----------------------------------- | ----------------------------------------------- |
| `use path;`                  | Import module, access via last part | `use std.io;` -> `io.print`                     |
| `use path.{a, b};`           | Import specified items              | `use std.io.{print};` -> `print`                |
| `use path as alias;`         | Import and rename                   | `use std.io as io;` -> `io.print`               |
| `use path.{i1, i2} as a, b;` | Import specified items and rename   | `use std.io.{print, read} as p, r;` -> `p`, `r` |

### 2.3 Import Examples

```yaoxiang
// Import the entire module
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

### 3.1 The `pub` Keyword

Use the `pub` keyword to declare exported items:

```yaoxiang
// Math.yx
pub pi: Float = 3.14159
pub sqrt: (x: Float) -> Float = (x) => { ... }

// Private items (not exported)
internal_value: Int = 42
```

### 3.2 Export Rules

- By default, all items are private
- Items declared with `pub` can be accessed by other modules
- Private items can only be accessed within the current module

### 3.3 Automatic `pub` Binding

For functions declared with `pub`, the compiler automatically binds them to types defined in the
same file:

```yaoxiang
// Declared with pub, compiler automatically binds
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

// Compiler automatically infers:
// 1. Point is defined in the current file
// 2. Function parameters contain Point
// 3. Execute Point.distance = distance[0]

// Invocation
d = distance(p1, p2)           // Functional
d2 = p1.distance(p2)           // OOP syntactic sugar
```

---

## Chapter 4: Scope

### 4.1 Module Scope

Each module has its own scope, and items within a module are not visible to the outside by default.

### 4.2 Nested Scope

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

YaoXiang has no `let` keyword. Is `x = value` a declaration or an assignment? Follow one principle:

**Assignment first.** Declaration happens only once, but assignment happens hundreds of times. Let
high-frequency operations take the shortest path.

```
x = value:
  Search outward along the scope chain for x
    → Found mut x          : Assignment, OK (via &mut token)
    → Found x (moved)      : Treated as "no valid binding found", re-declare in current scope
    → Found x (immutable, alive) : E2010 cannot reassign
    → Not found            : New declaration in current scope (the only declaration path)

mut x = value:
    → x already exists in current scope : E2002 duplicate definition
    → x exists in outer scope           : E2013 shadowing prohibited (explicit new declaration cannot share name with outer)
    → No conflict            : New mutable declaration
```

- **Same scope**: Any name can only be declared once (E2002)
- **Inner scope without `mut`**: Search the outer scope first, assign or error
- **Inner scope with `mut`**: Explicit new declaration, prohibited from sharing name with outer
  (E2013)

> **Blocks are real scopes**: The prerequisite for "searching along the scope chain" is that each
> `{}` block truly establishes a layer—names newly declared in an inner scope **do not leak to the
> outer scope** (see §2.15).

#### Same Scope

```yaoxiang
x = 10
x = 20              // E2010: 'x' is immutable, cannot reassign

mut y = 10
y = 20              // OK: same binding, reassignment
mut y = 30          // E2002: 'y' is already defined in this scope (explicit new declaration conflicts)

z = 10
mut z = 20          // E2002: 'z' is already defined in this scope (mut cannot cover existing declaration)
```

Note that `x = 20` reports **E2010** (cannot reassign) rather than E2002 (duplicate definition):
`x = value` without `mut` is semantically an **assignment** (search along the scope chain), not
"re-declare an x". Only `mut x = value` is an explicit new declaration, and conflicts report E2002.

#### Rebinding After Move

If an immutable variable owns a value, after its value is moved (consumed), the original binding
enters the **moved** state—the name still occupies the scope slot, but the value is no longer
accessible. At this point, `x = value` is not modifying the old binding, but re-declaring `x` within
the same scope.

```
"moved" branch of assignment-first search:
  x exists in current scope, but is in moved state
    → Compiler treats it as "no valid binding found"
    → Re-declare x in current scope (overwrite the old moved slot)
```

**Core mechanism:** After the old value is consumed, the binding becomes invalid, and the name
returns to a "declarable" state. This is not shadowing—the old binding no longer exists.

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

| Operation                 | Meaning                                   | Mechanism         | Syntax         |
| ------------------------- | ----------------------------------------- | ----------------- | -------------- |
| **Rebinding**             | Old value disappears, new value is born   | move + re-declare | `x = f(x)`     |
| **In-place modification** | Value at the same memory location changes | mut assignment    | `mut x; x = v` |

**Why this differs from shadowing:**

- Shadowing (Rust's `let x = ...`): The old binding still exists, just hidden by the new binding
- Rebinding after Move: The old binding has been consumed, the name returns to an uninitialized
  state, re-declaration is the only way out

**Constraints:**

- Only values that own can be moved. References (`&T`, `&mut T`) are copied rather than moved
- Move checking is completed at compile-time; reading a moved-state variable in any expression
  reports E2014
- The IDE can show a gray hint on moved variables, indicating that the name is in an uninitialized
  state

```yaoxiang
// Read after move → error
data = fetch()
result = process(data)   // data is moved
print(data)              // E2014: 'data' has been moved and cannot be used

// References do not trigger move
ref_data = &value
copy1 = ref_data         // Copy the reference, ref_data is still usable
copy2 = ref_data         // OK

// Cross-scope: moved state propagates
data = fetch()
{
    data = transform(data)  // move outer data → rebind (new inner declaration)
    print(data)             // OK: uses inner data
}
print(data)                 // E2014: outer data has been moved
```

#### Cross-Scope

```yaoxiang
// Outer immutable, inner assignment → immutable variable cannot be reassigned
x = 10
{
    x = 20          // E2010: 'x' is immutable, cannot reassign
}
{
    mut x = 20      // E2013: cannot shadow existing variable 'x' (explicit declaration of new binding)
}

// Outer mut, inner assignment → modify the same binding
mut y = 10
{
    y = 20          // OK: same binding, modified via &mut token
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

// Immutable penetration through all levels also cannot reassign
b = 0
{
    {
        b = 10      // E2010: 'b' is immutable, cannot reassign
    }
}
```

#### for Loop

```yaoxiang
// Loop variable is a new binding each iteration, not modification
for i in 1..5 {
    print(i)        // OK: new value bound each iteration
    i = 10          // E2010: immutable loop variable, cannot reassign
}

for mut i in 1..5 {
    i = 10          // OK: mutable loop variable
}

// Loop variable cannot shadow outer
i = 0
for i in 1..5 {     // E2013: cannot shadow existing variable 'i'
}

// mut outer accumulator can be modified within loop body
mut sum = 0
for i in 1..5 {
    sum = sum + i   // OK: same binding, modified via &mut token
}
print(sum)          // 15

// Immutable outer cannot be modified within loop body
sum2 = 0
for i in 1..5 {
    sum2 = sum2 + i // E2010: 'sum2' is immutable, cannot reassign
}
```

#### Related Error Codes

| Error Code | Message                                        | Trigger Scenario                                                                       |
| ---------- | ---------------------------------------------- | -------------------------------------------------------------------------------------- |
| E2002      | `'{name}' is already defined in this scope`    | Duplicate declaration in same scope (regardless of mut)                                |
| E2010      | `Cannot assign to immutable variable '{name}'` | When assigning without `mut` in inner scope, outer variable is immutable and not moved |
| E2013      | `Cannot shadow existing variable '{name}'`     | Inner explicit declaration (`mut x` or `x: Type`) shares name with outer               |
| E2014      | `'{name}' has been moved and cannot be used`   | Reading a moved variable                                                               |

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

### A.1 Module as File

```
// File name .yx is the module name
Import ::= 'use' ModuleRef
```

### A.2 Import/Export

```yaoxiang
// Import
use std.io
use std.io.{print, read}
use std.io as io

// Export
pub pi: Float = 3.14159
pub sqrt: (x: Float) -> Float = (x) => { ... }
```
