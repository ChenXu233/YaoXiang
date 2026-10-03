# Module System Specification

This document defines the YaoXiang module system specification: how modules are defined, how `use`
imports work, what the export surface is, and how scope is partitioned.

Design basis: [RFC-029 Module Semantics](../../design/rfc/accepted/029-module-semantics.md),
[RFC-029g Removing the `pub` Keyword and Auto Binding](../../design/rfc/accepted/029g-remove-pub-and-auto-bind.md).

For user-facing operational guidance, see [Module System](../../guide/modules).

---

## Chapter 1: Module Definition

### 1.1 Module is a File

Modules use files as their boundary. Each `.yx` file is a module; there is no `module` / `mod`
declaration keyword.

```yaoxiang
// math/geometry.yx
Point: Type = { x: Float, y: Float }

distance: (a: Point, b: Point) -> Float = { ... }
```

### 1.2 Module = A Set of Top-Level Bindings

A module's "contents" is the set of its top-level bindings. In the example above, the contents of
the `math.geometry` module is equivalent to:

```
{ Point: Type, distance: (Point, Point) -> Float }
```

A module is therefore an **open record**—there is no export manifest to declare item by item; the
module _is_ all of its top-level bindings.

### 1.3 Path Mapping

Module paths are mapped to files segment by segment, **file first, then directory**:

| Module path | Lookup order                                   |
| ----------- | ---------------------------------------------- |
| `a.b`       | `<base>/a/b.yx` → on miss, `<base>/a/b/mod.yx` |
| `a`         | `<base>/a.yx` → on miss, `<base>/a/mod.yx`     |

`mod.yx` is a **convention** (house number) for directory entry files, not the only entry
(lock)—both `a.yx` and `a/mod.yx` can host module `a`.

::: danger Do not use index.yx The Rust habit of `index.yx` is **not recognized** in YaoXiang.
Writing `src/helper/index.yx` and then `use helper` will produce E5001. Directory entries must be
named `mod.yx`. :::

Having both forms coexist will trigger a module path ambiguity error (RFC-029 §4).

### 1.4 Lookup Starting Point

`<base>` is tried in the following order:

1. The importer's own directory
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
| `use path.{x, y}`   | Bind `x`, `y` directly into the current scope      | `use std.math.{sqrt}` → `sqrt(...)`      |
| `use path.{x as y}` | Bind `x` as `y`                                    | `use std.io.{print as say}` → `say(...)` |
| `use path as alias` | Bring in the `path` namespace and rename it        | `use helper as h` → `h.label(...)`       |

`use path.{a, b}` binds **two paths to the same set of top-level bindings**, so `use a.{Point}` and
`use a.geometry.{Point}` point to the same binding.

### 2.3 Explicitly Not Adopted Syntax

The following forms were explicitly rejected in RFC-029, and the implementation refuses them
likewise:

| Syntax                     | Disposition                   | Actual behavior                                                              |
| -------------------------- | ----------------------------- | ---------------------------------------------------------------------------- |
| `use path.*`               | Wildcard import, not used     | Parse failure                                                                |
| `from path use item`       | Python-style, not used        | Parse failure                                                                |
| `use path.{item} as alias` | Positional alias, not used    | Parse failure, with a hint to use the inline form `use path.{item as alias}` |
| `use .relative`            | Relative import, not designed | Parse failure (E0011)                                                        |

::: warning Positional alias vs. inline alias are not the same `use std.io.{print} as p` (alias
**after** the braces) is explicitly rejected; `use std.io.{print as p}` (alias **inside** the
braces) is the supported form. The compiler's error message will point this out. :::

### 2.4 Name Conflict

When two module paths point to bindings of the same name, the import fails; use inline aliases to
disambiguate.

---

## Chapter 3: Export Surface

### 3.1 No Visibility Mechanism

**This language introduces no visibility mechanism whatsoever.** No `pub`, no `private`, no
`export`.

RFC-029 made this ruling; RFC-029g further removed the `pub` keyword entirely from the lexer, AST,
type checker, dead-code exemption, formatter, and LSP.

### 3.2 All Top-Level Bindings Are Importable by Default

Every top-level binding of a module—whether or not it was ever marked with any modifier—is visible
to all code that can write out that module path.

```yaoxiang
// helper/mod.yx —— note that plain has no modifier at all
plain: () -> string = "no pub keyword at all"

// another module can import it directly
use helper

main = () => {
    print(helper.plain())
}
```

Thus **there is no such thing as "private by default."** Arguing over a binding's visibility for the
sake of export is a meaningless question in YaoXiang.

> On the history of the `pub` keyword: before RFC-029g's ruling to remove it, `pub` only affected
> dead-code exemption (the W1001 family) and **never participated in export decisions**—writing
> `pub` did not affect whether something could be imported. The "all items are private by default"
> claim in old documentation is the opposite of the implementation.

### 3.3 Method Bindings Are Explicit

Methods are not auto-bound via `pub`. The method form is **explicit composition**:

```yaoxiang
Point: Type = { x: Float, y: Float }

// explicit method: first parameter is self
Point.distance: (self: &Point, other: &Point) -> Float = {
    dx = self.x - other.x
    dy = self.y - other.y
    (dx * dx + dy * dy).sqrt()
}
```

The explicit form is strictly more expressive (it supports multi-position bindings and unit
bindings) and is the only method form.

---

## Chapter 4: Scope

### 4.1 Scope Hierarchy

- **Module scope**: bindings at the top level of a file
- **Block scope**: each `{}` establishes a layer; inner declarations do not leak outward
- **Function scope**: inside the function body

### 4.2 Declaration and Shadowing

YaoXiang has no `let` keyword. Is `x = value` a declaration or an assignment? Follow one principle:

**Assignment first.** Declaration happens only once, but assignment happens a hundred times. Let the
high-frequency operation take the shortest path.

```
x = value:
    search outward along the scope chain for x
      → find mut x         : assignment, OK (via the &mut token)
      → find x (immutable, alive) : E2010 cannot be reassigned
      → not found          : declare new in the current scope (the only declaration path)

mut x = value:
    → x already exists in current scope : E2002 duplicate definition
    → x exists in an outer scope         : E2013 shadowing forbidden (explicit new declaration cannot share a name with an outer binding)
    → no conflict                         : new mutable declaration
```

- **Same scope**: any name can be declared only once (E2002)
- **Inner without `mut`**: prefer to look up the outer scope; assign or report an error
- **Inner with `mut`**: explicit new declaration; sharing a name with an outer binding is forbidden
  (E2013)

> **Blocks are real scopes**: the premise of "looking along the scope chain" is that every `{}`
> block genuinely establishes a layer—a name newly declared in an inner scope **does not leak
> outward**.

#### Same Scope

```yaoxiang
x = 10
x = 20              // E2010: 'x' is immutable and cannot be reassigned

mut y = 10
y = 20              // OK: same binding, reassigned
mut y = 30          // E2002: 'y' is already defined in this scope (explicit new declaration collides)

z = 10
mut z = 20          // E2002: 'z' is already defined in this scope (mut cannot cover an existing declaration)
```

Note that `x = 20` reports **E2010** (cannot be reassigned) rather than E2002 (duplicate
definition): `x = value` without `mut` is semantically an **assignment** (it searches along the
scope chain), not "declaring another x." Only `mut x = value` is an explicit new declaration, and
name collision then reports E2002.

#### Rebinding After Move

If an immutable variable owns its value, after its value is moved (consumed) the original binding
enters the **moved** state—the name still occupies its slot in the scope, but the value is no longer
accessible.

**moved is not an input to "declaration judgment."** The way to regain the name is an **explicit
redeclaration**:

```yaoxiang
// pipeline-style data flow: each step consumes the old value, produces a new one
mut data = fetch()           // explicit mutable declaration
mut data = transform(data)   // E2002: data already exists in the same scope
```

> **Why not "already moved → redeclaration allowed"**:
>
> Move is a **path-sensitive** data-flow property—after `if c { move p }`, `p` is moved on one path
> and not on another; it is "possibly moved" rather than a boolean truth value. A binding-level flag
> is structurally incapable of expressing branch confluence, and treating it as a switch for "name
> can be redeclared" would produce wrong diagnostics (a single-branch move would be treated as
> moved).
>
> Real move analysis lives in `layers/ownership.rs`: it builds the function body CFG and runs a data
> flow on the lattice `Alive < Moved < Dropped`, **taking the max (conservative) at branch
> confluence**, then reports E2014/E2018 at read check points. It answers "can I read here," while
> "can this name be declared again" is independently answered by the assignment-first rule in §4.2.
>
> The two responsibilities differ and should not be coupled. This is also why the old "already-moved
> branch" in earlier docs was never reachable: it depended on a flag that is never written.

**Rebinding relies on an explicit `mut` declaration:**

```yaoxiang
// the equivalent explicit form
mut data1 = fetch()
data2 = transform(data1)  // data1 is moved and cannot be used again
mut data3 = filter(data2)
```

**Semantic separation:**

| Operation                 | Meaning                                       | Mechanism              | Syntax                             |
| ------------------------- | --------------------------------------------- | ---------------------- | ---------------------------------- |
| **Rebinding**             | The old value disappears, a new one is born   | move + new declaration | `mut x = f(x)` (new name or `mut`) |
| **In-place modification** | The value at the same memory location changes | mut assignment         | `mut x = v`; `x = w`               |

**Constraints:**

- Only values that own their data can be moved. References (`&T`, `&mut T`) are copied rather than
  moved
- Move checking happens at compile time (CFG + data flow); reading a variable in the moved state in
  any expression reports E2014
- Declarations must come with an initial value:
  `LetStmt ::= ('mut')? Identifier (':' TypeExpr)? '=' Expr`—a pure annotation declaration like
  `x: Int` is not legal syntax and reports E0012

```yaoxiang
// reading after move → error
data = fetch()
result = process(data)   // data is moved
print(data)              // E2014: 'data' has been moved and cannot be used

// references do not trigger move
ref_data = &value
copy1 = ref_data         // copy the reference, ref_data is still usable
copy2 = ref_data         // OK

// across scopes: the moved state propagates
data = fetch()
{
    data = transform(data)  // move outer data → rebind (new inner declaration)
    print(data)             // OK: uses the inner data
}
print(data)                 // E2014: outer data has been moved
```

#### Across Scopes

```yaoxiang
// outer immutable, inner assignment → immutable variable cannot be reassigned
x = 10
{
    x = 20          // E2010: 'x' is immutable and cannot be reassigned
}
{
    mut x = 20      // E2013: cannot shadow existing variable 'x' (explicit new declaration)
}

// outer mut, inner assignment → modifies the same binding
mut y = 10
{
    y = 20          // OK: same binding, modified via the &mut token
}
print(y)            // 20

// outer mut, inner cannot declare the same name
mut z = 10
{
    z = 30          // OK: same binding
}
{
    mut z = 30      // E2013: cannot shadow existing variable 'z'
}

// multi-level nesting: mut propagates through all levels
mut a = 0
{
    {
        a = 10      // OK
    }
}
print(a)            // 10

// immutable also propagates through all levels and cannot be reassigned
b = 0
{
    {
        b = 10      // E2010: 'b' is immutable and cannot be reassigned
    }
}
```

#### for Loops

```yaoxiang
// the loop variable is a new binding on each iteration, not a modification
for i in 1..5 {
    print(i)        // OK: each iteration binds a new value
    i = 10          // E2010: immutable loop variable cannot be reassigned
}

for mut i in 1..5 {
    i = 10          // OK: mutable loop variable
}

// the loop variable cannot shadow an outer one
i = 0
for i in 1..5 {     // E2013: cannot shadow existing variable 'i'
}

// a mut outer accumulator can be modified inside the loop body
mut sum = 0
for i in 1..5 {
    sum = sum + i   // OK: same binding, modified via the &mut token
}
print(sum)          // 15

// an immutable outer accumulator cannot be modified inside the loop body
sum2 = 0
for i in 1..5 {
    sum2 = sum2 + i // E2010: 'sum2' is immutable and cannot be reassigned
}
```

#### Related Error Codes

| Error code | Message                                        | Trigger scenario                                                                             |
| ---------- | ---------------------------------------------- | -------------------------------------------------------------------------------------------- |
| E2002      | `'{name}' is already defined in this scope`    | Duplicate declaration in the same scope (mut or not)                                         |
| E2010      | `Cannot assign to immutable variable '{name}'` | Assigning in an inner scope without `mut` when the outer variable is immutable and not moved |
| E2013      | `Cannot shadow existing variable '{name}'`     | Inner explicit declaration (`mut x` or `x: Type`) shares a name with an outer binding        |
| E2014      | `'{name}' has been moved and cannot be used`   | Reading a variable in the moved state                                                        |

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

`mod.yx` in a directory serves as that directory's module entry:

```yaoxiang
// math/mod.yx
use math.vector
use math.matrix

Vector = vector.Vector
Matrix = matrix.Matrix
```

Note that there is **no `pub` here**—`Vector` and `Matrix` are ordinary top-level bindings, directly
visible to importers (see §3.2).

---

## Chapter 6: Diagnostics

| Error code | Meaning                                       | Typical trigger                                                            |
| ---------- | --------------------------------------------- | -------------------------------------------------------------------------- |
| E5001      | Module not found                              | Path typo; using `index.yx` as a directory entry; dependency not installed |
| E5003      | The named export does not exist in the module | Import name typo (e.g. `read` instead of `read_line`)                      |

E5001 in project mode comes with a hint: when a dependency is missing, it suggests running
`yx install`.
