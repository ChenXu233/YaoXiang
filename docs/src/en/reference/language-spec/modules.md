# Module System Specification

This document defines the module system specification for YaoXiang: how modules are defined, how
`use` imports them, what the export surface is, and how scope is partitioned.

Design basis: [RFC-029 Module Semantics](../../rfc/accepted/029-module-semantics.md),
[RFC-029g Removing the `pub` keyword and auto-binding](../../rfc/accepted/029g-remove-pub-and-auto-bind.md).

For a user-oriented operational guide, see [Module System](../../guide/modules).

---

## Chapter 1: Module Definition

### 1.1 A Module is a File

A module uses a file as its boundary. Every `.yx` file is a module; there are no `module` / `mod`
declaration keywords.

```yaoxiang
// math/geometry.yx
Point: Type = { x: Float, y: Float }

distance: (a: Point, b: Point) -> Float = { ... }
```

### 1.2 A Module Equals a Set of Top-level Bindings

A module's "contents" is its set of top-level bindings. In the example above, the contents of module
`math.geometry` are equivalent to:

```
{ Point: Type, distance: (Point, Point) -> Float }
```

A module is therefore an **open record**—there is no export list to be declared item by item; the
module is all of its top-level bindings.

### 1.3 Path Mapping

Module paths are mapped to files segment by segment, **file before directory**:

| Module path | Lookup order                                      |
| ----------- | ------------------------------------------------- |
| `a.b`       | `<base>/a/b.yx` → if not hit, `<base>/a/b/mod.yx` |
| `a`         | `<base>/a.yx` → if not hit, `<base>/a/mod.yx`     |

`mod.yx` is a **convention** (a house number) for the directory entry file, not the sole entry (a
lock)—both `a.yx` and `a/mod.yx` can host module `a`.

::: danger Do not use index.yx The Rust habit of `index.yx` is **not recognized** in YaoXiang.
Writing `src/helper/index.yx` will cause `use helper` to report E5001. The directory entry must be
named `mod.yx`. :::

Having both forms present at the same time triggers an ambiguous module path error (RFC-029 §4).

### 1.4 Lookup Starting Point

`<base>` is tried in the following order:

1. The importer's directory
2. The project root

---

## Chapter 2: Module Import

`use` is record destructuring: it brings the target module's bindings into the current scope.

### 2.1 Grammar

```
Import       ::= 'use' ModuleRef ImportSpec?
ImportSpec   ::= 'as' Identifier
              |  '{' ImportItems '}'
ImportItems  ::= ImportItem (',' ImportItem)* ','?
ImportItem   ::= Identifier ('as' Identifier)?
ModuleRef    ::= Identifier ('.' Identifier)*
```

### 2.2 Forms

| Syntax              | Semantics                                          | Example                                  |
| ------------------- | -------------------------------------------------- | ---------------------------------------- |
| `use path`          | Bring in the `path` namespace; access via `path.x` | `use std.io` → `io.print(...)`           |
| `use path.{x, y}`   | Bind `x` and `y` directly into the current scope   | `use std.math.{sqrt}` → `sqrt(...)`      |
| `use path.{x as y}` | Bind `x` as `y`                                    | `use std.io.{print as say}` → `say(...)` |
| `use path as alias` | Bring in the `path` namespace and rename it        | `use helper as h` → `h.label(...)`       |

`use path.{a, b}` binds two paths to the **same batch of top-level bindings**, so `use a.{Point}`
and `use a.geometry.{Point}` refer to the same binding.

### 2.3 Explicitly Rejected Syntax

The following forms were explicitly rejected in RFC-029, and the implementation rejects them as
well:

| Syntax                     | Disposition                    | Actual behavior                                                     |
| -------------------------- | ------------------------------ | ------------------------------------------------------------------- |
| `use path.*`               | Wildcard import, not adopted   | Parse failure                                                       |
| `from path use item`       | Python-style, not adopted      | Parse failure                                                       |
| `use path.{item} as alias` | Positional alias, not adopted  | Parse failure, prompting the inline form `use path.{item as alias}` |
| `use .relative`            | Relative import, not in design | Parse failure (E0011)                                               |

::: warning Positional alias and inline alias are not the same thing `use std.io.{print} as p`
(alias **after** the curly braces) is explicitly rejected; `use std.io.{print as p}` (alias
**inside** the curly braces) is a supported form. The compiler's error message will point this out.
:::

### 2.4 Name Conflicts

When two module paths point to bindings sharing the same name, the import fails; you must use an
inline alias to disambiguate.

---

## Chapter 3: Export Surface

### 3.1 No Visibility Mechanism

**This language does not introduce any visibility mechanism.** No `pub`, no `private`, no `export`.

RFC-029 made this ruling, and RFC-029g further removed the `pub` keyword wholesale from the lexer,
AST, type checker, dead code exemption, formatter, and LSP.

### 3.2 All Top-level Bindings are Importable by Default

Every top-level binding in a module—function, type, constant, with or without type annotations—is
visible to any code that can write the module's path.

```yaoxiang
// helper/mod.yx -- note that plain has no modifiers at all
plain: () -> string = "no pub keyword at all"

// another module can import it directly
use helper

main = () => {
    print(helper.plain())
}
```

Therefore **there is no such thing as "private by default"**. Debating whether a binding's
visibility is exported is a meaningless question in YaoXiang.

Accessing a name that does not exist on a module (or is not top-level) is reported as E1043 "module
does not export this member"; to import only some names, use the curly-brace form
`use helper.{plain}` to selectively bring them in.

> About the history of the `pub` keyword: before 029g's ruling to remove it, `pub` only acted on
> dead code exemption (the W1001 family), **never participating in export judgment**—writing `pub`
> did not affect whether it could be imported. The old documentation's claim that "all items are
> private by default" was the opposite of the implementation.

### 3.3 Method Binding is Explicit

Methods are not auto-bound via `pub`. The method form is an **explicit composition**:

```yaoxiang
Point: Type = { x: Float, y: Float }

// Explicit method: first parameter is self
Point.distance: (self: &Point, other: &Point) -> Float = {
    dx = self.x - other.x
    dy = self.y - other.y
    (dx * dx + dy * dy).sqrt()
}
```

The explicit form is strictly more expressive (it supports multi-position bindings, unit bindings)
and is the only method form.

---

## Chapter 4: Scope

### 4.1 Scope Hierarchy

- **Module scope**: bindings at the top level of a file
- **Block scope**: each `{}` establishes a level; inner declarations do not leak outward
- **Function scope**: within a function body

### 4.2 Declaration and Shadowing

YaoXiang has no `let` keyword. Is `x = value` a declaration or an assignment? Follow one principle:

**Assignment first.** Declarations happen only once, but assignments happen hundreds of times. Let
the high-frequency operation take the shortest path.

```
x = value:
    Search outward along the scope chain for x
      → Found mut x            : Assignment, OK (via &mut token)
      → Found x (immutable, alive) : E2010 cannot reassign
      → Not found              : Declare new in current scope (the only declaration path)

mut x = value:
    → x exists in current scope : E2002 duplicate definition
    → x exists in outer scope   : E2013 shadowing forbidden (explicit new declaration cannot share a name with outer)
    → No conflict               : New mutable declaration
```

- **Same scope**: any name can be declared only once (E2002)
- **Inner without `mut`**: prefer searching the outer scope; assign or error
- **Inner with `mut`**: explicit new declaration; cannot share a name with the outer scope (E2013)

> **Blocks are real scopes**: the premise of "searching along the scope chain" is that each `{}`
> block actually establishes a level—names newly declared in the inner scope **do not leak to the
> outer scope**.

#### Same Scope

```yaoxiang
x = 10
x = 20              // E2010: 'x' is immutable, cannot reassign

mut y = 10
y = 20              // OK: same binding, reassign
mut y = 30          // E2002: 'y' is already defined in this scope (explicit new declaration collides)

z = 10
mut z = 20          // E2002: 'z' is already defined in this scope (mut cannot overwrite an existing declaration)
```

Note that `x = 20` reports **E2010** (cannot reassign), not E2002 (duplicate definition):
`x = value` without `mut` is semantically an **assignment** (searching the scope chain), not
"declare another x". Only `mut x = value` is an explicit new declaration, and a name collision
reports E2002.

#### Re-binding After Move

If an immutable variable owns data, once its value is moved (consumed), the original binding enters
the **moved** state—the name still occupies the scope slot, but the value is no longer accessible.

**moved is not an input to "declaration judgment".** The way to reclaim that name is through an
**explicit re-declaration**:

```yaoxiang
// Pipelined data flow: each step consumes the old value and produces a new one
mut data = fetch()           // Explicit mutable declaration
mut data = transform(data)   // E2002: data already exists in the same scope
```

> **Why "already moved → re-declarable" is not used**:
>
> Move is a **path-dependent** data-flow property—after `if c { move p }`, `p` is moved on one path
> and not yet on another, which is "possibly moved" rather than a Boolean. A binding-level flag is
> structurally incapable of expressing branch merging; treating it as a switch for "the name can be
> re-declared" would give wrong diagnostics (a single-branch move being treated as already moved).
>
> The real move analysis lives in `layers/ownership.rs`: it builds the function-body CFG, performs
> data flow on the `Alive < Moved < Dropped` lattice, **taking max at branch merging
> (conservative)**, and reports E2014/E2018 at read checkpoints. It answers "can we read here",
> while "can this name be re-declared" is independently answered by the assignment-first rule in
> 4.2.
>
> The two responsibilities differ and should not be coupled. This is also why the old "already-moved
> branch" in old documents was never reachable: it depended on a flag that was never written.

**Re-binding relies on an explicit `mut` declaration:**

```yaoxiang
// Equivalent explicit form
mut data1 = fetch()
data2 = transform(data1)  // data1 has been moved, cannot be used again
mut data3 = filter(data2)
```

**Semantic separation:**

| Operation                 | Meaning                                  | Mechanism              | Syntax                             |
| ------------------------- | ---------------------------------------- | ---------------------- | ---------------------------------- |
| **Re-binding**            | Old value disappears, new value is born  | move + new declaration | `mut x = f(x)` (new name or `mut`) |
| **In-place modification** | The same memory location's value changes | mut assignment         | `mut x = v`; `x = w`               |

**Constraints:**

- Only values that own data can be moved. References (`&T`, `&mut T`) are copied, not moved
- Move checking is done at compile time (CFG + data flow); a variable in the moved state is reported
  as E2014 when read in any expression
- Declarations must carry an initial value:
  `LetStmt ::= ('mut')? Identifier (':' TypeExpr)? '=' Expr`—a pure annotation declaration like
  `x: Int` is ungrammatical and is reported as E0012

```yaoxiang
// Reading after move → error
data = fetch()
result = process(data)   // data is moved
print(data)              // E2014: 'data' has been moved and cannot be used

// References don't trigger move
ref_data = &value
copy1 = ref_data         // Copy the reference, ref_data is still usable
copy2 = ref_data         // OK

// Cross-scope: moved state propagates
data = fetch()
{
    data = transform(data)  // move outer data → re-bind (new declaration in inner scope)
    print(data)             // OK: use inner data
}
print(data)                 // E2014: outer data has been moved
```

#### Cross-scope

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

// Outer mut, inner cannot declare the same name
mut z = 10
{
    z = 30          // OK: same binding
}
{
    mut z = 30      // E2013: cannot shadow existing variable 'z'
}

// Multi-level nesting: mut propagates through all levels
mut a = 0
{
    {
        a = 10      // OK
    }
}
print(a)            // 10

// Immutable propagates through all levels and still cannot be reassigned
b = 0
{
    {
        b = 10      // E2010: 'b' is immutable, cannot reassign
    }
}
```

#### `for` Loop

```yaoxiang
// Loop variable is a new binding each iteration, not a modification
for i in 1..5 {
    print(i)        // OK: each iteration binds a new value
    i = 10          // E2010: immutable loop variable, cannot reassign
}

for mut i in 1..5 {
    i = 10          // OK: mutable loop variable
}

// Loop variable cannot shadow the outer scope
i = 0
for i in 1..5 {     // E2013: cannot shadow existing variable 'i'
}

// Outer mut accumulator can be modified inside the loop body
mut sum = 0
for i in 1..5 {
    sum = sum + i   // OK: same binding, modified via &mut token
}
print(sum)          // 15

// Outer immutable cannot be modified inside the loop body
sum2 = 0
for i in 1..5 {
    sum2 = sum2 + i // E2010: 'sum2' is immutable, cannot reassign
}
```

#### Related Error Codes

| Error code | Message                                        | Trigger scenario                                                          |
| ---------- | ---------------------------------------------- | ------------------------------------------------------------------------- |
| E2002      | `'{name}' is already defined in this scope`    | Same-scope duplicate declaration (regardless of mut)                      |
| E2010      | `Cannot assign to immutable variable '{name}'` | Inner assignment without `mut`; outer variable is immutable and not moved |
| E2013      | `Cannot shadow existing variable '{name}'`     | Inner explicit declaration (`mut x` or `x: Type`) shares name with outer  |
| E2014      | `'{name}' has been moved and cannot be used`   | Reading a moved variable                                                  |

---

## Chapter 5: Directory Organization

### 5.1 Structure

```
src/
├── main.yx          // top-level file
├── math/
│   ├── mod.yx       // directory entry (module math)
│   ├── vector.yx    // module math.vector
│   └── matrix.yx    // module math.matrix
└── utils/
    ├── mod.yx       // directory entry (module utils)
    └── string.yx    // module utils.string
```

### 5.2 Directory Entry

The `mod.yx` in a directory serves as that directory's module entry:

```yaoxiang
// math/mod.yx
use math.vector
use math.matrix

Vector = vector.Vector
Matrix = matrix.Matrix
```

Note the **absence of `pub`** here—`Vector` and `Matrix` are ordinary top-level bindings and are
directly visible to importers (see §3.2).

---

## Chapter 6: Diagnostics

| Error code | Meaning                       | Typical trigger                                                           |
| ---------- | ----------------------------- | ------------------------------------------------------------------------- |
| E5001      | Module not found              | Path typo; used `index.yx` as a directory entry; dependency not installed |
| E5003      | Member not exported in module | Import name typo (e.g., `read` instead of `read_line`)                    |

In project mode, E5001 will include a hint: when a dependency is missing, it suggests running
`yx install`.
