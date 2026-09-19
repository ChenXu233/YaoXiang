# Module System Specification

This file defines the module system specification for the YaoXiang programming language, including
module definitions, imports/exports, and scoping.

---

## Chapter 1: Module Definitions

### 1.1 Module Basics

Modules use files as boundaries. Each `.yx` file is a module.

```
// The file name is the module name
// Math.yx
pub pi: Float = 3.14159
pub sqrt: (x: Float) -> Float = (x) => { ... }
```

### 1.2 Module Naming Rules

- The module name is determined by the file name
- The `.yx` file extension does not participate in the module name
- Module names use PascalCase naming

---

## Chapter 2: Module Imports

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
| `use path.{a, b};`           | Import specific items               | `use std.io.{print};` -> `print`                |
| `use path as alias;`         | Import and rename                   | `use std.io as io;` -> `io.print`               |
| `use path.{i1, i2} as a, b;` | Import specific items and rename    | `use std.io.{print, read} as p, r;` -> `p`, `r` |

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

## Chapter 3: Module Exports

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

For functions declared with `pub`, the compiler automatically binds them to types defined in the
same file:

```yaoxiang
// Declared with pub, compiler auto-binds
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

// Compiler auto-infers:
// 1. Point is defined in the current file
// 2. Function parameters contain Point
// 3. Perform Point.distance = distance[0]

// Invocation
d = distance(p1, p2)           // Functional style
d2 = p1.distance(p2)           // OOP syntactic sugar
```

---

## Chapter 4: Scoping

### 4.1 Module Scope

Each module has its own scope; items within a module are not visible externally by default.

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

YaoXiang has no `let` keyword. Is `x = value` a declaration or an assignment? Follow one principle:

**Assignment takes precedence.** Declarations happen only once, but assignments happen hundreds of
times. Let the high-frequency operation take the shortest path.

```
x = value:
  Search outward along the scope chain for x
    → Found mut x           : assignment, OK (via &mut token)
    → Found x (immutable, alive) : E2010 cannot reassign
    → Not found              : new declaration in current scope (the only declaration path)

mut x = value:
    → x already exists in current scope : E2002 duplicate definition
    → x exists in outer scope           : E2013 shadowing forbidden (explicit new declaration cannot share name with outer)
    → No conflict              : new mutable declaration
```

- **Same scope**: Any name can only be declared once (E2002)
- **Inner without `mut`**: Search outer first, assign or error
- **Inner with `mut`**: Explicit new declaration, forbidden from sharing name with outer (E2013)

> **Blocks are real scopes**: The premise of "searching along the scope chain" is that each `{}`
> block does establish a layer—names newly declared in inner scopes **do not leak into outer
> scopes** (see §2.15).

#### Same Scope

```yaoxiang
x = 10
x = 20              // E2010: 'x' is immutable, cannot reassign

mut y = 10
y = 20              // OK: same binding, reassign
mut y = 30          // E2002: 'y' is already defined in this scope (explicit new declaration collides)

z = 10
mut z = 20          // E2002: 'z' is already defined in this scope (mut cannot override existing declaration)
```

Note that `x = 20` reports **E2010** (cannot reassign) rather than E2002 (duplicate definition):
`x = value` without `mut` is semantically an **assignment** (search along scope chain), not
"declaring another x". Only `mut x = value` is an explicit new declaration, and name collision then
reports E2002.

#### Rebinding After Move

If an immutable variable owns ownership, when its value is moved (consumed), the original binding
enters the **moved** state—the name still occupies the scope slot, but the value is no longer
accessible.

**Moved is not an input to "declaration judgment".** The way to reclaim that name is through
**explicit redeclaration**:

```yaoxiang
// Pipeline-style data flow: each step consumes old value, produces new value
mut data = fetch()           // explicit mutable declaration
mut data = transform(data)   // E2002: data already exists in same scope
```

> **Why not "already moved → can redeclare"** (revised 2026-09-19):
>
> Move is a **path-sensitive** data-flow property—after `if c { move p }`, `p` is moved on one path
> but not on the other; it is "possibly moved" rather than a boolean truth. A binding-level flag
> structurally cannot express branch confluence, and treating it as a switch for "name can be
> redeclared" produces wrong diagnostics (single-branch move treated as fully moved).
>
> The real move analysis is in `layers/ownership.rs`: build a function body CFG, perform data flow
> over the `Alive < Moved < Dropped` lattice, **take the max at branch confluence (conservative)**,
> and report E2014/E2018 at read checkpoints. It answers "can I read here", while "can this name be
> redeclared" is independently answered by §4.3's assignment-first rule.
>
> The two responsibilities differ and should not be coupled. This is also why the old text's "moved
> branch" was never reachable: it depends on a flag that is never written.

**Rebinding relies on explicit `mut` declaration:**

```yaoxiang
// Equivalent explicit form
mut data1 = fetch()
data2 = transform(data1)  // data1 is moved, cannot be used again
mut data3 = filter(data2)
```

**Semantic separation:**

| Operation                 | Meaning                                   | Mechanism              | Syntax                             |
| ------------------------- | ----------------------------------------- | ---------------------- | ---------------------------------- |
| **Rebinding**             | Old value disappears, new one born        | move + new declaration | `mut x = f(x)` (new name or `mut`) |
| **In-place modification** | Value at the same memory location changes | mut assignment         | `mut x = v; x = w`                 |

**Constraints:**

- Only values that own ownership can be moved. References (`&T`, `&mut T`) are copied, not moved
- Move checking is done at compile-time (CFG + data flow); reading a moved variable in any
  expression reports E2014
- The IDE can display grayed-out hints on moved variables, indicating the name is in an
  uninitialized state
- **Declarations must include an initial value**:
  `LetStmt ::= ('mut')? Identifier (':' TypeExpr)? '=' Expr` (§3.2)—pure annotation declarations
  like `x: Int` are not grammatical and report E0012

```yaoxiang
// Read after move → error
data = fetch()
result = process(data)   // data is moved
print(data)              // E2014: 'data' has been moved and cannot be used

// References do not trigger move
ref_data = &value
copy1 = ref_data         // copy reference, ref_data still usable
copy2 = ref_data         // OK

// Cross-scope: moved state penetrates
data = fetch()
{
    data = transform(data)  // move outer data → rebind (new declaration in inner scope)
    print(data)             // OK: use inner data
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
    mut x = 20      // E2013: cannot shadow existing variable 'x' (explicit new declaration)
}

// Outer mut, inner assignment → modify the same binding
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

#### `for` Loops

```yaoxiang
// Loop variable is a new binding each iteration, not a modification
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

// Mutable outer accumulator can be modified within loop body
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

| Error Code | Message                                        | Trigger Scenario                                                               |
| ---------- | ---------------------------------------------- | ------------------------------------------------------------------------------ |
| E2002      | `'{name}' is already defined in this scope`    | Duplicate declaration in same scope (regardless of mut)                        |
| E2010      | `Cannot assign to immutable variable '{name}'` | When inner assignment without `mut`, outer variable is immutable and not moved |
| E2013      | `Cannot shadow existing variable '{name}'`     | Inner explicit declaration (`mut x` or `x: Type`) shares name with outer       |
| E2014      | `'{name}' has been moved and cannot be used`   | Read from an already-moved variable                                            |

---

## Chapter 5: Module Organization

### 5.1 Directory Structure

```
src/
├── main.yx          // main module
├── math/
│   ├── index.yx     // math module entry
│   ├── vector.yx    // vector module
│   └── matrix.yx    // matrix module
└── utils/
    ├── index.yx     // utils module entry
    └── string.yx    // string utilities
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

### 5.3 Relative Imports

```yaoxiang
// In math/vector.yx
use math.matrix  // absolute import
use .matrix      // relative import (same directory)
```

---

## Appendix: Module Syntax Quick Reference

### A.1 Modules Are Files

```
// filename.yx is the module name
Import ::= 'use' ModuleRef
```

### A.2 Imports and Exports

```yaoxiang
// Import
use std.io
use std.io.{print, read}
use std.io as io

// Export
pub pi: Float = 3.14159
pub sqrt: (x: Float) -> Float = (x) => { ... }
```
