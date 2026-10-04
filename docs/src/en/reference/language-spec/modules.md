# Module System Specification

This document defines YaoXiang's module system specification: how modules are defined, how `use`
imports them, what the export surface is, and how scopes are partitioned.

Design basis: [RFC-029 Module Semantics](../../design/rfc/accepted/029-module-semantics.md),
[RFC-029g Removing the `pub` Keyword and Auto-Binding](../../design/rfc/accepted/029g-remove-pub-and-auto-bind.md).

For the user-facing operation guide, see [Module System](../../guide/modules).

---

## Chapter 1: Module Definition

### 1.1 A Module is a File

Modules use files as boundaries. Every `.yx` file is a module; there are no `module` / `mod`
declaration keywords.

```yaoxiang
// math/geometry.yx
Point: Type = { x: Float, y: Float }

distance: (a: Point, b: Point) -> Float = { ... }
```

### 1.2 A Module = a Set of Top-Level Bindings

A module's "content" is its set of top-level bindings. In the example above, the `math.geometry`
module's content is equivalent to:

```
{ Point: Type, distance: (Point, Point) -> Float }
```

A module is therefore an **open record**—there is no export list to declare one by one; the module
is all of its top-level bindings.

### 1.3 Path Mapping

Module paths are mapped segment-by-segment to files, **file first, then directory**:

| Module path | Lookup order                                        |
| ----------- | --------------------------------------------------- |
| `a.b`       | `<base>/a/b.yx` → if not found, `<base>/a/b/mod.yx` |
| `a`         | `<base>/a.yx` → if not found, `<base>/a/mod.yx`     |

`mod.yx` is a **convention** (house number) for a directory entry file, not the sole entry
(lock)—both `a.yx` and `a/mod.yx` can host module `a`.

::: danger Do Not Use index.yx The Rust convention of `index.yx` is **not recognized** in YaoXiang.
Writing `src/helper/index.yx` and then `use helper` will trigger E5001. Directory entries must be
named `mod.yx`. :::

If both forms exist simultaneously, a module path ambiguity is triggered and an error is reported
(RFC-029 §4).

### 1.4 Lookup Starting Point

`<base>` is tried in the following order:

1. The directory of the importer
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

`use path.{a, b}` binds **two paths to the same batch of top-level bindings**, so `use a.{Point}`
and `use a.geometry.{Point}` point to the same binding.

### 2.3 Explicitly Rejected Syntax

The following writings were explicitly rejected in RFC-029, and the implementation rejects them as
well:

| Syntax                     | Disposition                                 | Actual behavior                                                      |
| -------------------------- | ------------------------------------------- | -------------------------------------------------------------------- |
| `use path.*`               | Wildcard import, not adopted                | Parse failure                                                        |
| `from path use item`       | Python-style, not adopted                   | Parse failure                                                        |
| `use path.{item} as alias` | Positional alias, not adopted               | Parse failure, suggesting the inline form `use path.{item as alias}` |
| `use .relative`            | Relative import, not included in the design | Parse failure (E0011)                                                |

::: warning Positional Aliases and Inline Aliases Are Not the Same Thing `use std.io.{print} as p`
(alias after the curly braces) is explicitly rejected; `use std.io.{print as p}` (inside the curly
braces) is a supported form. The compiler's error message will point this out. :::

### 2.4 Name Conflicts

When two module paths point to bindings with the same name, the import fails; you must use inline
aliases to disambiguate.

---

## Chapter 3: Export Surface

<<<<<<< HEAD

### 3.1 No Visibility Mechanism

**This language introduces no visibility mechanism.** No `pub`, no `private`, no `export`.

RFC-029 made this ruling, and RFC-029g went further to remove the `pub` keyword entirely from the
lexer, AST, type checker, dead-code exemptions, formatter, and LSP.

### 3.2 All Top-Level Bindings are Importable by Default

Every top-level binding of a module—functions, types, constants, with or without type annotations—is
visible to any code that can write that module path.

```yaoxiang
// helper/mod.yx — note that plain has no modifier at all
plain: () -> string = "no pub keyword at all"

// another module can import it directly
use helper

main = () => {
    print(helper.plain())
}
```

Therefore **there is no such thing as "private by default"**. Arguing about the visibility of a
binding in YaoXiang is an invalid question.

Accessing a name that does not exist (or is not top-level) on a module reports E1043 "Module does
not export this member"; if you only want to bring in some names, use the `use helper.{plain}`
curly-brace form to selectively import.

> Regarding the history of the `pub` keyword: Before the 029g ruling to remove it, `pub` only
> affected dead-code exemptions (the W1001 family), and **never participated in export
> decisions**—writing `pub` does not affect whether something can be imported. The description in
> old documentation that "all items are private by default" is the opposite of the implementation.
> =======

### 3.1 Export Rules

Modules are bounded by files, and **all top-level bindings can be imported by other modules**—no
visibility mechanism is set (RFC-029 ruling of 2026-07-30: "If you don't want others to use it,
don't put it at the top level / don't publish that package"). There are two access forms:

```yaoxiang
// Math.yx
pi: Float = 3.14159
sqrt: (x: Float) -> Float = (x) => { ... }

// main.yx — after whole-module import, access via the module name (functions/constants both work)
use Math
Math.sqrt(4.0)
Math.pi
```

- Top-level bindings (functions, types, constants, with or without type annotations) all enter the
  export surface
- Accessing a name not exported by the module (non-existent or non-top-level) reports E1043 "Module
  does not export this member"
- If you only want to export some names, use the `use Math.{pi}` curly-brace form to selectively
  bring them in

### 3.2 The `pub` Keyword

`pub` currently only affects two things, and **does not affect export visibility** (top-level
bindings without `pub` can still be imported):

- Triggers automatic method binding (see 3.3)
- Dead-code check exemption: `pub` bindings do not participate in "defined but unused" warnings

> > > > > > > origin/fix/namespace-data-access

### 3.3 Method Binding is Explicit

Methods are not auto-bound via `pub`. The method form is **explicit composition**:

```yaoxiang
Point: Type = { x: Float, y: Float }

// Explicit method: first parameter is self
Point.distance: (self: &Point, other: &Point) -> Float = {
    dx = self.x - other.x
    dy = self.y - other.y
    (dx * dx + dy * dy).sqrt()
}
```

The explicit form is strictly more expressive (supports multiple positional bindings, unit bindings)
and is the only method form.

---

## Chapter 4: Scopes

### 4.1 Scope Layers

- **Module scope**: bindings at the top level of a file
- **Block scope**: each `{}` establishes a layer; declarations in inner layers do not leak to outer
  layers
- **Function scope**: within a function body

### 4.2 Declaration and Shadowing

YaoXiang has no `let` keyword. Is `x = value` a declaration or an assignment? Follow one principle:

**Assignment first.** A declaration happens only once, but an assignment happens a hundred times.
Let the high-frequency operation take the shortest path.

```
x = value:
    Search outward along the scope chain for x
      → found mut x          : assignment, OK (via &mut token)
      → found x (immutable, alive): E2010 cannot reassign
      → not found            : declare in current scope (the only declaration path)

mut x = value:
    → x already exists in current scope : E2002 duplicate definition
    → x exists in outer scope           : E2013 shadowing forbidden (explicit new declaration cannot share a name with an outer one)
    → no conflict                       : new mutable declaration
```

- **Same scope**: any name can be declared only once (E2002)
- **No `mut` in inner layer**: first look in the outer layer, then assign or error
- **`mut` in inner layer**: explicit new declaration, no name sharing with the outer layer (E2013)

> **Blocks are real scopes**: the prerequisite for "search along the scope chain" is that each `{}`
> block actually establishes a layer—names newly declared in inner layers **do not leak to outer
> layers**.

#### Same Scope

```yaoxiang
x = 10
x = 20              // E2010: 'x' is immutable, cannot be reassigned

mut y = 10
y = 20              // OK: same binding, reassigned
mut y = 30          // E2002: 'y' is already defined in this scope (explicit new declaration collides)

z = 10
mut z = 20          // E2002: 'z' is already defined in this scope (mut cannot overwrite an existing declaration)
```

Note that `x = 20` reports **E2010** (cannot reassign) rather than E2002 (duplicate definition):
`x = value` without `mut` is semantically an **assignment** (searches along the scope chain), not
"declaring another x". Only `mut x = value` is an explicit new declaration, and name collision is
reported as E2002.

#### Re-binding After Move

If an immutable variable owns a value, once its value is moved (consumed), the original binding
enters a **moved** state—the name still occupies a scope slot, but the value is no longer
accessible.

**Moved is not input to "declaration judgment."** The way to reclaim that name is an **explicit
redeclaration**:

```yaoxiang
// Pipeline-style data flow: each step consumes the old value, producing a new one
mut data = fetch()           // Explicit mutable declaration
mut data = transform(data)   // E2002: 'data' already exists in this scope
```

> **Why not "already moved → can redeclare"**:
>
> Move is a **path-dependent** data-flow property—after `if c { move p }`, `p` is moved on one path
> and not yet on the other, i.e. "may have been moved" rather than a boolean. A binding-level flag
> structurally cannot express branch confluence; treating it as a switch for "the name can be
> redeclared" would yield incorrect diagnostics (a single-branch move would be treated as already
> moved).
>
> The real move analysis lives in `layers/ownership.rs`: it builds a function-body CFG, performs
> data flow over a `Alive < Moved < Dropped` lattice, and **takes the max (conservative) at branch
> confluence**, reporting E2014/E2018 at read checkpoints. It answers "can we read here," whereas
> "can this name be declared again" is answered independently by the assignment-first rule in 4.2.
>
> The two responsibilities differ and should not be coupled. This is also why the old "already-moved
> branch" was never reachable: it depended on a flag that is never written.

**Re-binding relies on an explicit `mut` declaration:**

```yaoxiang
// Equivalent explicit form
mut data1 = fetch()
data2 = transform(data1)  // data1 is moved, cannot be used again
mut data3 = filter(data2)
```

**Semantic separation:**

| Operation                 | Meaning                                   | Mechanism              | Syntax                             |
| ------------------------- | ----------------------------------------- | ---------------------- | ---------------------------------- |
| **Re-binding**            | Old value disappears, new value is born   | move + new declaration | `mut x = f(x)` (new name or `mut`) |
| **In-place modification** | Value at the same memory location changes | mut assignment         | `mut x = v`; `x = w`               |

**Constraints:**

- Only values that own ownership can be moved. References (`&T`, `&mut T`) are copied rather than
  moved
- Move checking is done at compile-time (CFG + data flow); a variable in the moved state being read
  in any expression reports E2014
- Declarations must have an initial value:
  `LetStmt ::= ('mut')? Identifier (':' TypeExpr)? '=' Expr`—a pure-annotation declaration like
  `x: Int` is not grammatical and reports E0012

```yaoxiang
// Read after move → error
data = fetch()
result = process(data)   // data is moved
print(data)              // E2014: 'data' has been moved and cannot be used

// References do not trigger move
ref_data = &value
copy1 = ref_data         // Copy the reference, ref_data is still usable
copy2 = ref_data         // OK

// Across scopes: the moved state penetrates
data = fetch()
{
    data = transform(data)  // move outer data → re-bind (new declaration in inner scope)
    print(data)             // OK: uses the inner data
}
print(data)                 // E2014: outer data has been moved
```

#### Cross-Scope

```yaoxiang
// Outer immutable, inner assignment → immutable variable cannot be reassigned
x = 10
{
    x = 20          // E2010: 'x' is immutable, cannot be reassigned
}
{
    mut x = 20      // E2013: cannot shadow existing variable 'x' (explicit declaration of a new binding)
}

// Outer mut, inner assignment → modifies the same binding
mut y = 10
{
    y = 20          // OK: same binding, modified via &mut token
}
print(y)            // 20

// Outer mut, inner cannot declare a same-named binding
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

// Immutable penetration across all levels also cannot be reassigned
b = 0
{
    {
        b = 10      // E2010: 'b' is immutable, cannot be reassigned
    }
}
```

#### for Loop

```yaoxiang
// The loop variable is a fresh binding each iteration, not a modification
for i in 1..5 {
    print(i)        // OK: each iteration binds a new value
    i = 10          // E2010: immutable loop variable, cannot be reassigned
}

for mut i in 1..5 {
    i = 10          // OK: mutable loop variable
}

// Loop variables cannot shadow the outer scope
i = 0
for i in 1..5 {     // E2013: cannot shadow existing variable 'i'
}

// A mutable outer accumulator can be modified inside the loop body
mut sum = 0
for i in 1..5 {
    sum = sum + i   // OK: same binding, modified via &mut token
}
print(sum)          // 15

// An immutable outer cannot be modified inside the loop body
sum2 = 0
for i in 1..5 {
    sum2 = sum2 + i // E2010: 'sum2' is immutable, cannot be reassigned
}
```

#### Related Error Codes

| Error code | Message                                        | Trigger scenario                                                                           |
| ---------- | ---------------------------------------------- | ------------------------------------------------------------------------------------------ |
| E2002      | `'{name}' is already defined in this scope`    | Duplicate declaration in the same scope (mut or not)                                       |
| E2010      | `Cannot assign to immutable variable '{name}'` | When the inner layer assigns without `mut`, the outer variable is immutable and not moved  |
| E2013      | `Cannot shadow existing variable '{name}'`     | Inner layer explicit declaration (`mut x` or `x: Type`) shares a name with the outer layer |
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

Note the **absence of `pub`** here—`Vector` and `Matrix` are ordinary top-level bindings, directly
visible to importers (see §3.2).

---

## Chapter 6: Diagnostics

| Error code | Meaning                              | Typical trigger                                                            |
| ---------- | ------------------------------------ | -------------------------------------------------------------------------- |
| E5001      | Module not found                     | Path typo; using `index.yx` as a directory entry; dependency not installed |
| E5003      | The module does not export this item | Imported name typo (e.g. `read` instead of `read_line`)                    |

E5001 in project mode comes with a hint: when a dependency is missing, it suggests running
`yx install`.
