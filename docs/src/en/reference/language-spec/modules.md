# Module System Specification

This file defines the module system specification of YaoXiang: how modules are defined, how `use`
imports them, what the export surface is, and how scopes are partitioned.

Design basis: [RFC-029 Module Semantics](../../design/rfc/accepted/029-module-semantics.md),
[RFC-029g Remove `pub` keyword and auto-binding](../../design/rfc/accepted/029g-remove-pub-and-auto-bind.md).

A user-facing operational guide is available in [Module System](../../guide/modules).

---

## Chapter 1: Module Definition

### 1.1 Modules Are Files

Modules use files as their boundary. Every `.yx` file is a module, with no `module` / `mod`
declaration keyword.

```yaoxiang
// math/geometry.yx
Point: Type = { x: Float, y: Float }

distance: (a: Point, b: Point) -> Float = { ... }
```

### 1.2 Module = A Set of Top-Level Bindings

The "content" of a module is the set of its top-level bindings. In the example above, the content of
module `math.geometry` is equivalent to:

```
{ Point: Type, distance: (Point, Point) -> Float }
```

A module is therefore an **open record**—there is no export list to declare item by item; the module
is exactly all of its top-level bindings.

### 1.3 Path Mapping

Module paths are mapped segment-by-segment to files, **file first, directory second**:

| Module path | Lookup order                                   |
| ----------- | ---------------------------------------------- |
| `a.b`       | `<base>/a/b.yx` → fallback `<base>/a/b/mod.yx` |
| `a`         | `<base>/a.yx` → fallback `<base>/a/mod.yx`     |

`mod.yx` is a **convention** (door number) for a directory entry file, not the sole entry
(lock)—both `a.yx` and `a/mod.yx` can host module `a`.

::: danger Do not use index.yx The Rust habit of `index.yx` is **not recognized** in YaoXiang. If
you write `src/helper/index.yx`, then `use helper` will report E5001. The directory entry must be
named `mod.yx`. :::

Having both forms present at the same time will trigger a module path ambiguity error (RFC-029 §4).

### 1.4 Lookup Starting Point

`<base>` is tried in the following order:

1. The importer's own directory
2. The project root

---

## Chapter 2: Module Importing

`use` is record destructuring: it brings the bindings of the target module into the current scope.

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

### 2.3 Explicitly Rejected Syntax

The following constructs are explicitly rejected in RFC-029, and the implementation rejects them as
well:

| Syntax                     | Disposition                    | Actual behavior                                                   |
| -------------------------- | ------------------------------ | ----------------------------------------------------------------- |
| `use path.*`               | Wildcard import, rejected      | Parse failure                                                     |
| `from path use item`       | Python-style, rejected         | Parse failure                                                     |
| `use path.{item} as alias` | Positional alias, rejected     | Parse failure, hint to use inline form `use path.{item as alias}` |
| `use .relative`            | Relative import, not in design | Parse failure (E0011)                                             |

::: warning Positional alias and inline alias are not the same `use std.io.{print} as p` (alias
**after** the curly braces) is explicitly rejected; `use std.io.{print as p}` (alias **inside** the
curly braces) is the supported form. The compiler's error message will point this out. :::

### 2.4 Name Conflict

When two module paths point to bindings with the same name, the import will fail; use an inline
alias to disambiguate.

---

## Chapter 3: Export Surface

### 3.1 No Visibility Mechanism

**This language introduces no visibility mechanism whatsoever.** No `pub`, no `private`, no
`export`.

RFC-029 made this ruling, and RFC-029g further removed the `pub` keyword entirely from the lexer,
AST, type checker, dead-code exemptions, formatter, and LSP.

### 3.2 All Top-Level Bindings Are Importable by Default

Every top-level binding in a module—functions, types, constants, whether or not they have type
annotations—is visible to any code that can write that module path.

```yaoxiang
// helper/mod.yx — note that plain has no modifier
plain: () -> string = "no pub keyword at all"

// Another module can import it directly
use helper

main = () => {
    print(helper.plain())
}
```

So **there is no such thing as "private by default."** Debating whether a binding should be visible
for export is a moot question in YaoXiang.

Accessing a name that does not exist on the module (or is not a top-level name) reports E1043
"Module does not export this member"; to introduce only a subset of names, use selective import with
curly braces like `use helper.{plain}`.

> History of the `pub` keyword: before the 029g ruling to delete it, `pub` only took effect for
> dead-code exemptions (the W1001 family), and **never participated in export decisions**—writing
> `pub` did not affect whether something could be imported. The old documentation's claim that "all
> items are private by default" is the opposite of what the implementation did.

### 3.3 Method Bindings Are Explicit

Methods are not auto-bound via `pub`. Method form is **explicit composition**:

```yaoxiang
Point: Type = { x: Float, y: Float }

// Explicit method: first parameter is self
Point.distance: (self: &Point, other: &Point) -> Float = {
    dx = self.x - other.x
    dy = self.y - other.y
    (dx * dx + dy * dy).sqrt()
}
```

The explicit form is strictly more expressive (it supports multi-positional bindings and unit
bindings) and is the only method form.

---

## Chapter 4: Scope

### 4.1 Scope Hierarchy

- **Module scope**: the bindings at the top of a file
- **Block scope**: every `{}` establishes a layer; declarations in inner layers do not leak to outer
  layers
- **Function scope**: inside a function body

### 4.2 Declaration and Shadowing

YaoXiang has no `let` keyword. Is `x = value` a declaration or an assignment? Follow one principle:

**Assignment wins.** A declaration happens once; an assignment happens a hundred times. Let the
high-frequency operation take the shortest path.

```
x = value:
    walk the scope chain outward looking for x
      → find mut x          : assignment, OK (via &mut token)
      → find x (immutable, alive) : E2010 cannot reassign
      → not found           : new declaration in the current scope (the only declaration path)

mut x = value:
    → x already exists in current scope : E2002 duplicate definition
    → x exists in outer scope           : E2013 shadowing forbidden (explicit new declaration cannot share the name)
    → no conflict                        : new mutable declaration
```

- **Same scope**: any name can be declared only once (E2002)
- **No `mut` in inner layer**: prefer to look up the outer layer; assign or report an error
- **`mut` in inner layer**: explicit new declaration, must not share a name with the outer layer
  (E2013)

> **Blocks are real scopes**: the prerequisite for "walking the scope chain" is that every `{}`
> block does establish a layer—a name newly declared in the inner layer **does not leak to the outer
> layer**.

#### Same Scope

```yaoxiang
x = 10
x = 20              // E2010: 'x' is immutable, cannot reassign

mut y = 10
y = 20              // OK: same binding, reassign
mut y = 30          // E2002: 'y' is already defined in this scope (explicit new declaration collides)

z = 10
mut z = 20          // E2002: 'z' is already defined in this scope (mut cannot override an existing declaration)
```

Note that `x = 20` reports **E2010** (cannot reassign) rather than E2002 (duplicate definition):
`x = value` without `mut` is semantically an **assignment** (walking the scope chain), not
"declaring another x." Only `mut x = value` is an explicit new declaration, and name collisions
there report E2002.

#### Rebinding After Move

If an immutable variable owns the value, once its value is moved (consumed), the original binding
enters the **moved** state—the name still occupies a slot in scope, but the value is no longer
accessible.

**moved is not an input to "declaration" decisions.** The way to reclaim the name is an **explicit
re-declaration**:

```yaoxiang
// Pipeline-style data flow: each step consumes the old value and produces a new one
mut data = fetch()           // explicit mutable declaration
mut data = transform(data)   // E2002: data already exists in this scope
```

> **Why not "already moved → can redeclare"**:
>
> move is a **path-sensitive** data-flow property—after `if c { move p }`, `p` is moved on one path
> and not on the other; it is "possibly moved" rather than a Boolean. A binding-level flag
> structurally cannot express branch confluence, and treating it as a switch for "can this name be
> re-declared" would give wrong diagnostics (a single-branch move treated as already moved).
>
> The real move analysis lives in `layers/ownership.rs`: it builds the function body's CFG and runs
> a data flow on the `Alive < Moved < Dropped` lattice, taking the **max at branch confluence**
> (conservative), and reports E2014/E2018 at read checkpoints. It answers "can I read here?", while
> "can this name be re-declared" is answered independently by the assignment-wins rule in §4.2.
>
> The two responsibilities are different and should not be coupled. This is also why the old
> "already-moved branch" was never reachable: it relied on a flag that was never written.

**Rebinding relies on explicit `mut` declaration:**

```yaoxiang
// The equivalent explicit form
mut data1 = fetch()
data2 = transform(data1)  // data1 is moved, can no longer be used
mut data3 = filter(data2)
```

**Semantic separation:**

| Operation                 | Meaning                                   | Mechanism              | Syntax                             |
| ------------------------- | ----------------------------------------- | ---------------------- | ---------------------------------- |
| **Rebinding**             | Old value disappears, new value is born   | move + new declaration | `mut x = f(x)` (new name or `mut`) |
| **In-place modification** | Value at the same memory location changes | mut assignment         | `mut x = v`; `x = w`               |

**Constraints:**

- Only values that own their value can be moved. References (`&T`, `&mut T`) are copied, not moved.
- Move checking is done at compile-time (CFG + data flow); reading a variable in the moved state in
  any expression reports E2014.
- Declarations must come with an initial value:
  `LetStmt ::= ('mut')? Identifier (':' TypeExpr)? '=' Expr`—a pure annotation declaration like
  `x: Int` is not grammatical and reports E0012.

```yaoxiang
// Reading after move → error
data = fetch()
result = process(data)   // data is moved
print(data)              // E2014: 'data' has been moved, cannot be used

// References do not trigger move
ref_data = &value
copy1 = ref_data         // copy the reference, ref_data is still usable
copy2 = ref_data         // OK

// Across scopes: the moved state penetrates
data = fetch()
{
    data = transform(data)  // move outer data → rebind (new inner declaration)
    print(data)             // OK: uses inner data
}
print(data)                 // E2014: outer data has been moved
```

#### Across Scopes

```yaoxiang
// Outer immutable, inner assignment → immutable variable cannot be reassigned
x = 10
{
    x = 20          // E2010: 'x' is immutable, cannot reassign
}
{
    mut x = 20      // E2013: cannot shadow existing variable 'x' (explicit new binding)
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

// Immutable penetration of all levels also cannot be reassigned
b = 0
{
    {
        b = 10      // E2010: 'b' is immutable, cannot reassign
    }
}
```

#### `for` Loop

```yaoxiang
// Loop variable is a fresh binding each iteration, not a modification
for i in 1..5 {
    print(i)        // OK: new value bound each iteration
    i = 10          // E2010: immutable loop variable, cannot reassign
}

for mut i in 1..5 {
    i = 10          // OK: mutable loop variable
}

// Loop variable cannot shadow an outer one
i = 0
for i in 1..5 {     // E2013: cannot shadow existing variable 'i'
}

// Mutable outer accumulator can be modified inside the loop body
mut sum = 0
for i in 1..5 {
    sum = sum + i   // OK: same binding, modified via &mut token
}
print(sum)          // 15

// Immutable outer accumulator cannot be modified inside the loop body
sum2 = 0
for i in 1..5 {
    sum2 = sum2 + i // E2010: 'sum2' is immutable, cannot reassign
}
```

#### Related Error Codes

| Error code | Message                                        | Trigger scenario                                                                           |
| ---------- | ---------------------------------------------- | ------------------------------------------------------------------------------------------ |
| E2002      | `'{name}' is already defined in this scope`    | Duplicate declaration in the same scope (with or without mut)                              |
| E2010      | `Cannot assign to immutable variable '{name}'` | Inner scope assigns without `mut`, outer variable is immutable and not moved               |
| E2013      | `Cannot shadow existing variable '{name}'`     | Inner scope explicit declaration (`mut x` or `x: Type`) shares a name with the outer scope |
| E2014      | `'{name}' has been moved and cannot be used`   | Reading a variable that has been moved                                                     |

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

| Error code | Meaning                                 | Typical trigger                                                          |
| ---------- | --------------------------------------- | ------------------------------------------------------------------------ |
| E5001      | Module not found                        | Wrong path; used `index.yx` as directory entry; dependency not installed |
| E5003      | The export does not exist in the module | Typo in the import name (e.g. `read` instead of `read_line`)             |

E5001 in project mode comes with a hint: if a dependency is missing, suggest running `yx install`.
