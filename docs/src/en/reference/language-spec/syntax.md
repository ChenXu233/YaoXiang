# Syntax Specification

This document defines the syntax specification of the YaoXiang programming language, including
lexical structure, grammar rules, and operator precedence.

---

## Chapter 1: Lexical Structure

### 1.1 Source Files

YaoXiang source files must use UTF-8 encoding. Source files usually have the extension `.yx`.

### 1.2 Lexical Token Categories

| Category   | Description                     | Example                   |
| ---------- | ------------------------------- | ------------------------- |
| Identifier | Begins with a letter or `_`     | `x`, `_private`, `my_var` |
| Keyword    | Language-defined reserved words | `Type`, `pub`, `use`      |
| Literal    | Fixed values                    | `42`, `"hello"`, `true`   |
| Operator   | Operation symbols               | `+`, `-`, `*`, `/`        |
| Separator  | Syntactic delimiters            | `(`, `)`, `{`, `}`, `,`   |

### 1.3 Keywords

YaoXiang defines a very small set of keywords:

```
pub    use    spawn
ref    mut    if     else
else   match  while  for    return
break  continue as     in     unsafe
```

These keywords have special meaning in all contexts and cannot be used as identifiers.

### 1.4 Reserved Words

YaoXiang's "reserved words" are organized into three layers, identified at different stages by the
parser and the type checker:

#### 1.4.1 Literal Reserved Words

Literal identifiers that the parser treats as independent tokens; they cannot be used as ordinary
identifiers:

| Identifier | Owning Type | Description                                                                                                   |
| ---------- | ----------- | ------------------------------------------------------------------------------------------------------------- |
| `Type`     | —           | Meta type keyword                                                                                             |
| `true`     | Bool        | Boolean true                                                                                                  |
| `false`    | Bool        | Boolean false                                                                                                 |
| `void`     | Void        | Void literal (Unit value). Lowercase `void` is a value literal; uppercase `Void` is a type name (see §1.4.3). |

#### 1.4.2 Variant Names

Variant names are recognized by the parser in **pattern** contexts (a bare name is decided by the
scrutinee's variant set, see §2.8); constructing a variant in an **expression** context requires
type qualification — `Result(Int, String).ok(5)`, `Option(T).some(v)`, `Color.green()` (RFC-010
record types: variant constructors are function-typed fields in a type definition; a bare name like
`ok(5)` is not a constructor call).

| Variant Name | Owning Type | Description                       |
| ------------ | ----------- | --------------------------------- |
| `some(T)`    | Option      | Option value variant construction |
| `ok(T)`      | Result      | Result success variant            |
| `err(E)`     | Result      | Result error variant              |

#### 1.4.3 Built-in Type Names

The following type names are pre-registered by the type checker and can be used in type positions
without import. The parser treats them as ordinary identifiers — **they are not reserved words and
can be shadowed by local bindings (not recommended)**.

| Type Name | Logical Correspondence | Description                                                                                                                                        |
| --------- | ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Void`    | ⊤ (true / Unit)        | A zero-field product type with exactly one inhabitant (the `void` literal, see §1.4.1)                                                             |
| `Never`   | ⊥ (false / empty type) | A zero-variant sum type with zero inhabitants. No expression can produce a `Never` value. `Never <: T` holds for all `T` (principle of explosion). |
| `Int`     | —                      | Signed integer                                                                                                                                     |
| `Float`   | —                      | Floating-point number                                                                                                                              |
| `Bool`    | —                      | Boolean value: `true` / `false`                                                                                                                    |
| `Char`    | —                      | Unicode character                                                                                                                                  |
| `String`  | —                      | String                                                                                                                                             |

### 1.5 Identifiers

An identifier begins with a letter or underscore, followed by letters, digits, or underscores.
Identifiers are case-sensitive.

Special identifiers:

- `_` is used as a placeholder to indicate an ignored value
- Identifiers beginning with an underscore represent private members

### 1.6 Literals

#### 1.6.1 Integers

```
Decimal     ::= [0-9][0-9_]*
Octal       ::= 0o[0-7][0-7_]*
Hex         ::= 0x[0-9a-fA-F][0-9a-fA-F_]*
Binary      ::= 0b[01][01_]*
```

#### 1.6.2 Floats

```
Float       ::= [0-9][0-9_]* '.' [0-9][0-9_]* ([eE][+-]?[0-9][0-9_]*)?
```

#### 1.6.3 Strings

```
String      ::= '"' ([^"\\] | EscapeSequence)* '"'
Escape      ::= '\\' ([nrt'"\\] | UnicodeEscape)
Unicode     ::= 'u' '{' HexDigit+ '}'
```

#### 1.6.4 Collections

```
List        ::= '[' Expr (',' Expr)* ']'
Dict        ::= '{' String ':' Expr (',' String ':' Expr)* '}'
Array       ::= '[' Expr (',' Expr)* ']'   // when the target type annotation is Array(T, N), the literal resolves to a fixed-length array
```

> **Dict literal requires at least one key-value pair**: `{}` is **not** an empty dict — it is an
> empty block (value `Void`, see [§2.9](#_2-9-块表达式)). Use the constructor `dict.new()` for an
> empty dict:
>
> ```yaoxiang
> empty = dict.new()                  // ✅ empty dict
> d = { "a": 1 }                       // ✅ dict literal
> wrong = {}                           // ❌ this is not a dict, it is an empty block (Void)
> ```
>
> **The criterion is content**: the `Dict` grammar requires at least one `String ':' Expr`; `{}` has
> no content to base on, so it takes the zero form of block structure. Non-empty forms are
> self-describing by content (`{ "k": v }` has a key-value pair → dict) — this is the same source as
> `f = { 5 }` being an `Int` value rather than a function: **the type is determined by content**.

> Set has no literal grammar and no runtime representation — the collection type is in the planning
> stage; when the need arises, complete it following the Dict pattern (`std.set` +
> `HeapValue::Set`). The resolution of List/Dict literals is determined by the context type
> annotation: a bare literal and a `List(T)` annotation resolve to a growable list; an `Array(T, N)`
> annotation applied directly to a literal resolves to a fixed-length array. Implicit List→Array
> conversion is prohibited.
>
> Array literal semantics:
>
> - Element count must equal N; otherwise compile-time E1002. Empty literal with non-zero N is
>   likewise rejected.
> - Each element's type must be compatible with T; otherwise compile-time E1002.
> - Grammar form of N: integer literal only (may be negative) or constant name; compound expressions
>   (e.g., `2+1`) are rejected at parse time.
> - When N is a symbolic constant (function `const` parameter, e.g., `Array(Int, n)`), the count
>   check is deferred to the refinement type stage.
> - v1 nested array literals (`Array(Array(Int,2),2) = [[1,2],[3,4]]`) are rejected at compile time;
>   explicit construction layer by layer is required. Recursive resolution is left for a later
>   version.

#### 1.6.5 List Comprehensions

```
ListComp    ::= '[' Expr 'for' Identifier 'in' Expr (',' Expr)* ('if' Expr)? ']'
```

> **Behavior tightening (migration record)**: The iteration variable grammar has always been
> `'for' Identifier 'in'`, but in the old implementation the pattern went through the full pratt
> parser — once `'in'` was registered as an infix operator, `x` would swallow `in items` as a
> membership expression. After the fix, non-identifier patterns fail to parse directly, instead of
> falling back to `_` like the old implementation did (silently swallowing the error). Impact:
> previously parseable forms like `[x for (a, b) in pairs]` now report an error — that form never
> had defined behavior (the variable was always `_`), so the tightening direction is correct with no
> semantic migration cost.

#### 1.6.6 Membership Test

```
Membership  ::= Expr 'in' Expr
```

> `in` is a binary relational operator that returns `Bool` — hit returns `true`, miss returns
> `false`, no error. Semantic split: `[]` asserts existence and retrieves a value (failure reports
> an error), while `in` asks whether something exists (a miss is a normal `false`). Right-operand
> coverage: List / Array / Dict (key set) / Tuple / String (substring) / Range (interval). `in` is a
> first-class Hoare predicate, serving as the base of compile-time-provable propositions in the
> refinement type stage. (Set is removed from the right-operand list — Set has no runtime
> representation, see §1.6.4.)

### 1.7 Comments

```
// Single-line comment

/* Multi-line comment
   can span multiple lines */
```

### 1.8 Indentation Rules

Code must use 4-space indentation; Tab characters are forbidden. This is a mandatory syntactic rule.

---

## Chapter 2: Grammar Rules

### 2.1 Expression Categories

```
Expr        ::= Literal
              | Identifier
              | FnCall
              | MemberAccess
              | IndexAccess
              | UnaryOp
              | BinaryOp
              | TypeCast
              | RangeExpr
              | ErrorPropagate
              | RefExpr
              | IfExpr
              | MatchExpr
              | Block
              | Lambda
```

### 2.2 Operator Precedence

| Precedence | Operators                   | Associativity |
| ---------- | --------------------------- | ------------- |
| 1          | `()` `[]` `.` `?`           | left-to-right |
| 2          | `as`                        | left-to-right |
| 3          | Unary prefix `!` `-` `+`    | right-to-left |
| 4          | `*` `/` `%`                 | left-to-right |
| 5          | `+` `-`                     | left-to-right |
| 6          | `..`                        | left-to-right |
| 7          | `<<` `>>`                   | left-to-right |
| 8          | `&` `\|` `^`                | left-to-right |
| 9          | `==` `!=` `<` `>` `<=` `>=` | left-to-right |
| 10         | `and` `or`                  | left-to-right |
| 11         | `if...else`                 | right-to-left |
| 12         | `=` `+=` `-=` `*=` `/=`     | right-to-left |

> **Unary prefix operators** (`!` `-` `+`) bind tightly: only lower than call and member access,
> higher than all binary operators. Therefore `!a == b` ≡ `(!a) == b` (Zig-style semantics); `!` is
> a pure unary operator and does not participate in short-circuit control flow — it is orthogonal to
> the `and`/`or` keywords (short-circuit) (authoritative definition in RFC-010).

> **Range binding strength**: `..` has binding strength (6, 7) — the left precedence (6) is lower
> than addition (7), and the right precedence (7) swallows addition but not the same-level `..`.
> Before/after comparison:
>
> | Expression   | Before (level 1, right-associative)                                                           | After ((6,7), left-associative)                                                  |
> | ------------ | --------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------- |
> | `x in 1..10` | `x in 1..10` (right-operand level of `in` is 4; level-1 `..` cannot swallow it — unparseable) | `x in (1..10)` — the range as a whole is the right operand of `in`               |
> | `0..n+2`     | `(0..n)+2` (right-associative trap: upper bound eaten; `for` loop gets E3004 directly)        | `0..(n+2)` — upper bound is an arithmetic expression                             |
> | `a == b..c`  | `a == (b..c)` (level-1 `..` < level-3 `==`; naturally whole)                                  | `a == (b..c)` — **semantics unchanged**, `..` still higher than comparison level |
> | `1..2*3`     | `(1..2)*3`                                                                                    | `1..(2*3)` — upper bound is an arithmetic expression                             |
> | `a..b..c`    | `a..(b..c)` (right-associative chain, meaningless Range in Range)                             | `(a..b)..c` — **step form** (`c` is the step)                                    |
>
> Net effect: the compound upper bound `for i in 0..n+2` goes from "parses successfully but E3004"
> to "directly usable"; `x in 1..10` goes from "unparseable" to "range check"; `a..b..c` goes from
> "meaningless nesting" to "step component". Level 6 falls between `+` (level 5) and `<<` (level 7),
> following mathematical convention: a range is a tightly-binding construct, so the upper bound is
> naturally a complete arithmetic expression.

### 2.3 Function Calls

```
FnCall      ::= Expr '(' ArgList? ')'
ArgList     ::= Expr (',' Expr)* (',' NamedArg)* | NamedArg (',' NamedArg)*
NamedArg    ::= Identifier '=' Expr
```

Named arguments use `name = value` (RFC-010 §Function Definitions, RFC-011 §Construction Forms).
Positional arguments must come before named arguments; the order of parameters specified
positionally can be arranged arbitrarily among the named arguments:

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b

add(3, 5)          // positional
add(a = 3, b = 5)  // named
add(b = 5, a = 3)  // order is free
add(3, b = 5)      // mixed, positional first
```

A wrong name in a named argument produces **E1014**; specifying the same formal parameter
positionally and by name at the same time produces **E1015**; a count mismatch produces **E1010**
(RFC-013).

### 2.4 Member Access

```
MemberAccess::= Expr '.' Identifier
```

### 2.5 Index Access

```
IndexAccess ::= Expr '[' Expr ']'
```

> **Three layers of semantics (RFC-011b)**: `a[i]` is dispatched by the type of `a` — ① built-in
> containers (List/Vec/Array/Dict/Tuple) use native indexing instructions (fast path); ② user types
> that implement the `Index` interface dispatch to their `index` method (instantiation of
> `Index(Grid, Int, Float)` in the type body + the `Grid.index` method); ③ RFC-004's `f[0]`
> positional binding exists only in binding declarations and does not go through this grammar. The
> key for multi-dimensional indexing `a[0, 1]` is packed as a tuple. Other types that do not
> implement `Index` are rejected at the type level.

### 2.6 Type Casting

```
TypeCast    ::= Expr 'as' TypeExpr
```

### 2.7 Conditional Expressions

```
IfExpr      ::= 'if' Expr Block ('else' 'if' Expr Block)* ('else' Block)?
```

### 2.8 Pattern Matching

```
MatchExpr   ::= 'match' Expr '{' MatchArm+ '}'
MatchArm    ::= Pattern ('|' Pattern)* ('if' Expr)? '=>' Expr ','
Pattern     ::= Literal
              | Identifier
              | Wildcard
              | StructPattern
              | TuplePattern
              | EnumPattern
              | OrPattern
```

> **Variant destructuring requires a variant set** (RFC-010b): `EnumPattern` (variant names like
> `ok(v)`, `some(x)` decided by the scrutinee) requires the scrutinee's type to be a **sum type
> whose variant set is in scope**. The variant set enters the checker only through type definitions
> or `use` imports — `Result`/`Option` are defined in `std.result`/`std.option`, so they must be
> imported with `use std.result` / `use std.option` before use (the whole-module form and grouped
> form `use std.{...}` are equally valid); otherwise variant destructuring reports E1002.
> Exhaustiveness checking follows the same source: a fallback arm exempts the match from
> exhaustiveness; without a fallback arm, the entire variant set is checked.

### 2.9 Block Expressions

```
Block       ::= '{' Stmt* Expr? '}'
```

> **Statement termination rules**: the separation between Stmts and the behavior of newlines
> (explicit `;` separation, newline termination, line-continuation exceptions, leading `(`/`[` that
> never merges) are defined by [RFC-038](../../design/rfc/accepted/038-statement-termination.md).

#### 2.9.1 The Three Forms of `{`

`{` in an expression position has exactly three interpretations, determined by **content** in a
single pass:

| Form             | Notation          | Type                 | Example          |
| ---------------- | ----------------- | -------------------- | ---------------- |
| **Empty block**  | `{}`              | `Void`               | `x: Void = {}`   |
| **Dict literal** | `{ "k": v, ... }` | `Dict(K, V)`         | `d = { "a": 1 }` |
| **Block**        | `{ Stmt* Expr? }` | tail-expression type | `y = { 1 + 1 }`  |

**Determination order**:

1. `{}` (no content) → **empty block**, value `Void`
2. First element is a `String ':' Expr` key-value pair → **dict literal**
3. Otherwise → **block**, value given by the tail expression

> **Why `{}` is not an empty dict**: the "empty" of an empty dict cannot be self-describing (it
> could be either `Dict(K, V)` or an empty block), while the dict grammar ([§1.6.4](#_1-6-4-集合))
> requires at least one key-value pair. When there is no content to base on, take the zero form of
> block structure — this is consistent with `unsafe {}` / `spawn {}`, without introducing a special
> case. Use `dict.new()` for an empty dict.
>
> **Why functions need an annotation**: `f = { stmt }` is a **value** (tail-expression type), not a
> function. To define a function, write the Fn annotation explicitly: `f: () -> Int = { 5 }`. This
> follows the same principle as dicts: **the type is determined by content**, not by whether an
> annotation exists. (See [RFC-010a](../../design/rfc/accepted/010a-tail-expression-and-return.md)
> Appendix D.)

**Uniform semantics**: the value of every `{}` block is given by the **tail expression**; `return`
is a non-local exit of type `Never`.

| Block Type  | Value Exit      | Empty Block `{}` |
| ----------- | --------------- | ---------------- |
| Plain `{}`  | tail expression | `Void`           |
| `unsafe {}` | tail expression | `Void`           |
| `spawn {}`  | tail expression | `Void`           |

**Core principles** (see [RFC-010a](../../design/rfc/accepted/010a-tail-expression-and-return.md)
for details):

- **Block value = tail expression** (the last expression), the only exit, no exceptions
- When the last item is an **assignment statement**, the block value is `Void`; if you want `Void`,
  write `Void` explicitly
- **`return` exits the nearest function boundary** (passes through all blocks, does not "return to
  the block"), type `Never`; `Never <: T'` holds for any `T'` (principle of explosion), so `return`
  can appear in any return-type position without additional rules
- The expression form `= expr` directly gives the value

```yaoxiang
// Plain {} block: tail expression gives the value
result = {
    x = compute()
    x                // block value
}

// unsafe {} block: tail expression gives the type definition
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void
    }
    SqliteDb         // block value
}

// spawn {} block: tail expression gives the result
(a, b) = spawn {
    result1 = fetch("url1"),
    result2 = fetch("url2")
    (result1, result2)   // block value
}

// return: passes through the block, exits the function
f: (n: Int) -> Int = {
    if n < 0 {
        return 0     // passes out of if and the function body, exits the function
    }
    n * 2            // tail expression
}
```

#### Is `name = { ... }` a Function or a Block Value?

`name = { ... }` could be either a function definition (RFC-007 "no-arg simplest form") or a
block-value binding. The rule is **annotation takes precedence, default to function** (RFC-010a
Appendix D):

| Case                              | Result          | Example                            |
| --------------------------------- | --------------- | ---------------------------------- |
| Value is a Lambda (`=>`)          | function        | `f = () => 5` → `f()` = 5          |
| Annotation is a function type     | function        | `f: () -> Int = { 5 }` → `f()` = 5 |
| Annotation is a non-function type | **block value** | `x: Int = { 5 }` → `x` = 5         |
| No annotation                     | function        | `f = { 5 }` → `f()` = 5            |

**Annotation is the type**: `x: Int = ...` declares that `x` is `Int`, so `{ ... }` evaluates to
`Int`; `f: () -> Int = ...` declares that `f` is a function, so `{ ... }` is a function body.

To make `{ ... }` evaluate immediately, **just write the target type** (no new syntax needed):

```yaoxiang
// Block value: evaluated immediately
x: Int = {
    y = 5
    y            // x = 5
}

// Function: default when no annotation
f = { 5 }        // f() = 5
```

#### Nested Function Types: Curried or Function-Returning?

A nested function type on the right of `->` has two readings, distinguished by **parentheses**
(RFC-004):

| Notation                        | Meaning                | Call                      |
| ------------------------------- | ---------------------- | ------------------------- |
| `(a: Int) -> (b: Int) -> Int`   | **curried**            | `f(1)(2)`                 |
| `(a: Int) -> ((b: Int) -> Int)` | **function-returning** | `g(1)` returns a function |

The principle: **annotation is the type**. `g: (a: Int) -> ((b: Int) -> Int)` declares that
`g(1) : (b: Int) -> Int`, so `g(1)` must **be** that function, not "the next argument segment".

```yaoxiang
// Curried: two argument segments given layer by layer
add: (a: Int) -> (b: Int) -> Int = { a + b }
add(1)(2)        // → 3

// Function-returning: one outer argument, what is returned is a function
adder: (n: Int) -> ((x: Int) -> Int) = (x) => x + n
adder(10)(5)     // → 15
h = adder(10)    // the returned function can be stored in a variable, passed around
```

Unparenthesized nested `Fn` is always curried (including the type-parameter form in RFC-011):

```yaoxiang
identity: (T: Type) -> (x: T) -> T = (x) => x   // curried
identity(5)      // → 5
```

**Type checking**: parentheses declare the return type, so the body must produce that type.
`f: () -> (() -> Int) = { 7 }` is an error — it expects `() -> Int`, but actually gets `Int`
(E1002).

### 2.10 Lambda Expressions

```
Lambda      ::= '(' ParamList? ')' '=>' Expr
            |  '(' ParamList? ')' '=>' Block
```

### 2.11 Error Propagation Operator

```
ErrorPropagate ::= Expr '?'
```

The `?` operator is a postfix operator, with the same precedence as `.`. For `Result(T, E)`:

- `Ok(v)` extracts the value `v` and continues execution
- `Err(e)` propagates the error upward (`return Err(e)`)

```yaoxiang
process: (data: Data) -> Result(Data, Error) = {
    validated = validate(data)?     // extract on success, propagate on failure
    transform(validated)
}
```

### 2.12 Range Expressions

```
RangeExpr   ::= Expr '..' Expr ('..' Expr)?
```

`..` creates a range value (Range is a first-class value, not syntactic sugar).

```yaoxiang
for i in 0..10 { print(i) }
slice = array[0..5]

// Range is a value: bind, pass, membership test
r = 1..10
assert.assert(5 in r, "membership")
for i in r { print(i) }

// step form (third component, default 1)
for i in 0..10..2 { print(i) }  // 0, 2, 4, 6, 8
for i in 10..0..(-2) { print(i) }  // 10, 8, 6, 4, 2
```

> **Step semantics**: in `a..b..c`, `c` is the step. The literal `c = 0` is rejected at compile
> time; a dynamic `c` is checked at runtime (E6001 family; promoted to `Result` after the error
> system is landed). `c < 0` is valid, and the range direction reverses with the sign (`10..0..(-2)`
> is descending).

### 2.13 ref Expressions

```
RefExpr     ::= 'ref' Expr
```

`ref` creates shared ownership. The compiler automatically selects Rc (single-task) or Arc
(cross-task); users don't need to worry about the implementation details.

```yaoxiang
data = ref heavy_data
spawn { use(data) }   // cross-task: the compiler automatically selects Arc
```

### 2.14 unsafe Expressions

```
UnsafeExpr  ::= 'unsafe' Block
```

The `unsafe` block is used to define opaque types and manipulate raw pointers. Use `return` to
return the type definition to the enclosing scope.

**Semantics**:

- `unsafe {}` can define types and manipulate raw pointers
- The returned type is usable outside `unsafe {}`
- Field access on the type requires `unsafe` permission

```yaoxiang
// Define an opaque type inside an unsafe block
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  // raw pointer
    }
    return SqliteDb
}

// SqliteDb is usable outside the unsafe block
db = sqlite3_open("test.db")
```

### 2.15 Scope

**Basic rules**:

- Every `{}` block creates a scope
- Inner scopes can access variables from outer scopes
- Outer scopes cannot access variables from inner scopes
- Variable declaration follows the "assignment-first" principle

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

**Variable declaration and shadowing**:

- `x = value`: search outward along the scope chain for `x`; if found, assign; if not found, declare
  a new one
- `mut x = value`: an explicit new mutable declaration; sharing the name with an outer binding is
  forbidden
- Any name can be declared at most once within the same scope

> **Detailed definition**: the complete rules of scope, variable declaration, and shadowing are
> detailed in [Module System Specification](./modules.md#第四章作用域).

---

## Chapter 3: Statements

### 3.1 Statement Categories

```
Stmt        ::= LetStmt
              | ExprStmt
              | ReturnStmt
              | BreakStmt
              | ContinueStmt
              | IfStmt
              | MatchStmt
              | WhileStmt
              | ForStmt
              | SpawnStmt
```

### 3.2 Variable Declaration

```
LetStmt     ::= ('mut')? Identifier (':' TypeExpr)? '=' Expr
```

### 3.3 return Statement

```
ReturnStmt  ::= 'return' Expr?
```

**Semantics**: `return` is a **non-local exit** that exits the nearest **function boundary** (passes
through all blocks — including `if` / `while` / `for` / `match` / bare blocks / `spawn` / `unsafe`),
handing the value to the caller. It does **not "return to the block"**.

**Type**: `return e : Never` (where `e : T`). `Never <: T'` holds for any `T'` (principle of
explosion, see [Type System §2.2](./type-system.md)), so `return` can appear in any return-type
position without additional constraints.

**Relationship with block evaluation**: a block's value is always the **tail expression** (see
§2.9). `{ return n }` as a block has value `n` of type `Never`; meanwhile, the effect of `return` is
to exit the function. **Both hold simultaneously**, coexisting through the principle of explosion.

`return` and the tail expression together make "early return" possible, with no special rules
designating `return` as function-specific — see
[RFC-010a](../../design/rfc/accepted/010a-tail-expression-and-return.md).

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 {
        return 1          // passes out of if, exits the function (type Never)
    }
    n * factorial(n - 1)  // tail expression = block value
}
```

### 3.4 break Statement

```
BreakStmt   ::= 'break'
```

**Semantics**: immediately terminates the innermost enclosing `while`/`for` loop, and control flows
to the statement after that loop body.

- **Exits only the innermost level**: `break` always targets the innermost loop containing it. When
  a nested loop needs to break out of multiple levels, extract the inner loop into a function and
  use `return`, or use a flag. (`break`/`continue` carry no label; if loop labels are introduced in
  the future, the syntax will follow the loop declaration side per the RFC process, decided together
  with the multi-exit design in the proof pipeline.)
- **Only valid inside a loop body**: `break` may only appear inside a `while`/`for` loop body
  (including blocks/if/match nested within); appearing outside a loop is a compile error (E1102
  `'break' outside of a loop`).
- **Does not affect termination proofs**: `break` does not participate in termination arguments — it
  neither provides a measure nor constitutes a decreasing step of a measure; the loop's termination
  obligation is independent of `break` and is triggered by refinement types (see
  [type-system §8.4](./type-system.md#84-terminates终止测度谓词)).
- **Borrow semantics**: the control-flow edge of `break` participates in the structural cut of the
  reverse BFS liveness analysis in RFC-009a (exited iterations are not part of back-edge liveness
  derivation).

```yaoxiang
mut i = 0
while i < 10 {
    i = i + 1
    if i == 3 {
        break              // control flows to after the loop, i == 3
    }
}

// Nested loop: break exits only the inner loop
while j < 3 {
    while k < 10 {
        if k == 2 { break }    // only terminates the inner loop
    }
    j = j + 1                  // each outer iteration reaches here
}
```

### 3.5 continue Statement

```
ContinueStmt::= 'continue'
```

**Semantics**: skips the remaining statements in the current iteration and proceeds directly to the
next iteration of the innermost enclosing loop — `while` returns to the condition check, `for` takes
the next element.

- **Targets only the innermost level**: same as `break`; carries no label
- **Only valid inside a loop body**: appearing outside a loop is a compile error (E1102)

```yaoxiang
mut sum = 0
mut n = 0
while n < 5 {
    n = n + 1
    if n == 3 {
        continue           // skips the accumulation below, n == 3 is not counted
    }
    sum = sum + n
}
// sum == 12（1 + 2 + 4 + 5）
```

### 3.6 if Statement

```
IfStmt      ::= 'if' Expr Block ('else' 'if' Expr Block)* ('else' Block)?
```

### 3.7 match Statement

```
MatchStmt   ::= 'match' Expr '{' MatchArm+ '}'
```

### 3.8 while Statement

```
WhileStmt   ::= 'while' Expr Block
```

### 3.9 for Statement

```
ForStmt     ::= 'for' 'mut'? Identifier 'in' Expr Block
```

#### 3.9.1 Semantics: Each Iteration is a New Binding

YaoXiang's `for` loop semantics differ from traditional languages: **each iteration is a new
binding, not a modification of the same variable**.

```yaoxiang
// Example: for i in 1..5
for i in 1..5 {
    print(i)
}
```

**Execution process**:

| Iteration | Behavior of the loop variable                                                              |
| --------- | ------------------------------------------------------------------------------------------ |
| 1st       | Creates a new binding `i = 1`, the loop body executes, prints 1                            |
| 2nd       | Creates a new binding `i = 2` (the previous binding is destroyed), body executes, prints 2 |
| 3rd       | Creates a new binding `i = 3`, the loop body executes, prints 3                            |
| 4th       | Creates a new binding `i = 4`, the loop body executes, prints 4                            |
| End       | The loop body ends, the binding is destroyed                                               |

**Key point**: after each iteration ends, the binding created in that iteration is destroyed. The
next iteration is a completely new binding, with no relationship to the previous iteration's
binding.

#### 3.9.2 Difference Between `for` and `for mut`

| Syntax              | Loop variable mutability | Description                                         |
| ------------------- | ------------------------ | --------------------------------------------------- |
| `for i in 1..5`     | immutable                | The binding cannot be modified inside the loop body |
| `for mut i in 1..5` | mutable                  | The binding can be modified inside the loop body    |

```yaoxiang
// Valid: each iteration binds a new value, no modification needed
for i in 1..5 {
    print(i)  // read the value of i
}

// Error: immutable binding, cannot be modified
for i in 1..5 {
    i = i + 1  // error: cannot modify an immutable binding
}

// Valid: use `for mut` to allow modifying the binding
for mut i in 1..5 {
    i = i + 1  // modification allowed
}
```

#### 3.9.3 Shadowing Check

YaoXiang prohibits variable shadowing. A `for` loop variable cannot share a name with a variable in
an outer scope:

```yaoxiang
// Error: i is already declared in the outer scope
i = 10
for i in 1..5 {
    print(i)
}

// Correct: use a different variable name
i = 10
for j in 1..5 {
    print(j)
}
```

This rule applies to all code blocks; see [4.3 Shadowing Rules](./modules.md#43-遮蔽规则).

#### 3.9.4 Comparison with Other Languages

| Language | For loop variable semantics                                |
| -------- | ---------------------------------------------------------- |
| YaoXiang | binds a new value each iteration                           |
| Rust     | modifies the same variable (requires `mut`)                |
| Python   | modifies the same variable (no `mut` needed)               |
| C/C++    | modifies the same variable (requires pointer or reference) |

**Design rationale**: YaoXiang adopts binding semantics because:

1. **More aligned with natural semantics** In natural language, "for each element x in the
   collection" means each x is an independent individual. YaoXiang's `for i in 1..5` reads as "for
   each i from 1 to 5", where the i in each iteration is a brand-new binding — this matches human
   intuition.

2. **Avoids accidental modification** The default immutable binding semantics means the loop
   variable cannot be accidentally modified inside the loop body. No need to worry about an
   `i = ...` somewhere in a complex loop body causing hard-to-track bugs.

3. **High-performance solutions within reach** When the same variable really does need to be reused
   across iterations (e.g., an accumulator or cache), use `for mut` to switch to mutable binding
   mode. This is clearer than implicit shared state — intent is expressed explicitly in syntax, not
   hidden in runtime behavior.

### 3.10 spawn Statement

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' 'mut'? Identifier 'in' Expr '{' Expr '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
```

**spawn block**: explicitly declares a concurrent region; expressions inside the block execute
concurrently.

```yaoxiang
(result_a, result_b) = spawn {
    parse(fetch("url1")),
    parse(fetch("url2"))
}
```

**spawn loop**: a data-parallel loop.

```yaoxiang
results = spawn for item in items {
    process(item)
}
```

**spawn blocks capture outer variables** (RFC-024 §2.3, value-capture semantics):

- References to outer variables in the block body = **Move value capture**: the value is snapshotted
  into the closure environment at the spawn creation point; the block body reads through the env
  (LoadUpvalue)
- **Primitives** (Int/Float/Bool/Char) are copied by value; outer variables are unaffected
- **Handle types** (Struct/String/List, etc.) are snapshotted = handle copy, sharing the underlying
  object; under the Embedded runtime (default), the same thread and heap make the handle valid
- Sharing between multiple tasks requires explicit `ref` (§2.13, the compiler automatically selects
  Rc/Arc)
- `return` inside the block also captures referenced outer variables

```yaoxiang
t1 = 1 + 1
t2 = 2 + 2
result = spawn {
    return t1 + t2    // t1/t2 are captured by value, result == 6
}
```

---

### 3.11 Program Entry and Top-level Statements

A source file plays one of two roles in the compiler's view, **determined by the presence of
`yaoxiang.toml`**:

| Role       | Determination                              | Program body                                                                       |
| ---------- | ------------------------------------------ | ---------------------------------------------------------------------------------- |
| **Script** | Single-file direct run, no `yaoxiang.toml` | **top-level statements** (executed in source order); `main` is an ordinary binding |
| **Bin**    | A `yaoxiang.toml` exists                   | **`main` function**; no executable statements allowed at the top level             |

#### Script: Top-level Statements Are the Program

Without a manifest, the file is a "script" — **top-level statements execute in source order**:

```yaoxiang
use std.io
io.println("hello")          // executes directly
x: Int = { 42 }              // top-level binding: initialized at runtime
io.println(x)                // 42
```

`main` is **not special** in this mode — it is just an ordinary binding. To make `main` run, you
must explicitly call it:

```yaoxiang
use std.io
main: () -> Void = { io.println("only runs if called") }

main()                       // ← this line is required
```

> **Why not auto-call `main`?** The top-level statements are already the program body. If `main`
> were also called implicitly, scripts that explicitly wrote `main()` would execute twice. The two
> rules cannot coexist, so in Script mode there is only one execution entry: "top-level statements".

#### Bin: `main` is the Entry

With a manifest, the file is an "executable target". In this case:

- A **binding** named `main` must be defined; value or function, either is acceptable (finalized in
  #388: entry is determined by the binding's existence — a function is already a value, the two only
  differ in evaluation strategy): a function `main` is called with zero arguments at entry; a value
  `main` is evaluated at initialization, which is execution
- Executable statements are not allowed at the top level — the program body is exactly `main`

```yaoxiang
main: () -> Void = {
    print("hello")
}
```

A value `main` is equally valid — it is evaluated at initialization, which is execution. Evaluation
occurs within the initialization sequence (topological order over global bindings; independent
bindings follow source order):

```yaoxiang
main = {
    print("hello")
}
```

Missing `main` is a compile error (when `main` is missing, all functions are unreachable). An
uncallable value `main` (e.g., `main: Int = 5`) is valid but has no observable effect — matching
Rust's empty `fn main() {}`.

> **Library files**: a file that is `use`d by other files, or pointed to by `[lib].path` /
> `[exports]`, does not require `main` — it is not a program entry.

#### Initialization of Top-level Bindings

The initialization value of a top-level binding is **evaluated at runtime** and is not required to
be a compile-time constant:

```yaoxiang
answer: Int = { 42 }              // block value
inc: (Int) -> Int = (x) => x + 1
computed: Int = inc(41)           // function call
```

Initialization executes in **dependency order**, independent of source order:

```yaoxiang
derived: Int = base * 3           // references base declared later
base: Int = 7                     // initialized first (topological sort)
```

Circular dependencies are a compile error (the names on the cycle will be listed):

```yaoxiang
a: Int = b + 1
b: Int = a + 1                    // error: a → b → a
```

> **Design basis**: RFC-029f (file role model), RFC-010a Appendix D (block-binding arbitration).

---

## Appendix: Syntax Quick Reference

### A.1 Control Flow

```
if Expr Block (else if Expr Block)* (else Block)?
match Expr { MatchArm+ }
while Expr Block
for 'mut'? Identifier 'in' Expr Block
break | continue          // only inside a loop body (§3.4 / §3.5)
```

### A.2 Error Handling

```
Expr '?'              // error propagation (Result types)
```

### A.3 match Syntax

```
match value {
    pattern1 => expr1,
    pattern2 if guard => expr2,
    _ => default_expr,
}
```
