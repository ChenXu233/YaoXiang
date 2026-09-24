# Syntax Specification

This document defines the syntax specification of the YaoXiang programming language, including
lexical structure, syntax rules, and operator precedence.

---

## Chapter 1: Lexical Structure

### 1.1 Source Files

YaoXiang source files must use UTF-8 encoding. Source files typically use the `.yx` extension.

### 1.2 Lexical Token Categories

| Category   | Description                        | Examples                  |
| ---------- | ---------------------------------- | ------------------------- |
| Identifier | Begins with a letter or underscore | `x`, `_private`, `my_var` |
| Keyword    | Language predefined reserved words | `Type`, `pub`, `use`      |
| Literal    | Fixed values                       | `42`, `"hello"`, `true`   |
| Operator   | Operation symbols                  | `+`, `-`, `*`, `/`        |
| Delimiter  | Syntax separators                  | `(`, `)`, `{`, `}`, `,`   |

### 1.3 Keywords

YaoXiang defines a very small set of keywords:

```
pub    use    spawn
ref    mut    if     else
else   match  while  for    return
break  continue as     in     unsafe
```

These keywords have special meaning in any context and cannot be used as identifiers.

### 1.4 Reserved Words

YaoXiang's "reserved words" are organized into three layers, identified at different stages by the
parser and type checker:

#### 1.4.1 Literal Reserved Words

Literal identifiers with independent tokens in the parser, which cannot be used as ordinary
identifiers:

| Identifier | Type | Description                                                                                                   |
| ---------- | ---- | ------------------------------------------------------------------------------------------------------------- |
| `Type`     | —    | Meta-type keyword                                                                                             |
| `true`     | Bool | Boolean true value                                                                                            |
| `false`    | Bool | Boolean false value                                                                                           |
| `void`     | Void | Void literal (Unit value). Lowercase `void` is a value literal; uppercase `Void` is a type name (see §1.4.3). |

#### 1.4.2 Constructor Expressions

The following constructors are recognized by the parser in pattern matching and expression contexts:

| Constructor | Type   | Description                      |
| ----------- | ------ | -------------------------------- |
| `some(T)`   | Option | Option value variant constructor |
| `ok(T)`     | Result | Result success variant           |
| `err(E)`    | Result | Result error variant             |

#### 1.4.3 Built-in Type Names

The following type names are pre-registered by the type checker and can be used in type positions
without import. The parser treats them as ordinary identifiers—**not reserved words, and can be
shadowed by local bindings (not recommended)**.

| Type Name | Logical Correspondence | Description                                                                                                                                        |
| --------- | ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Void`    | ⊤ (True/Unit)          | A zero-field product type with exactly one inhabitant (the `void` literal, see §1.4.1)                                                             |
| `Never`   | ⊥ (False/Empty Type)   | A zero-variant sum type with zero inhabitants. No expression can produce a `Never` value. `Never <: T` holds for all `T` (principle of explosion). |
| `Int`     | —                      | Signed integer                                                                                                                                     |
| `Float`   | —                      | Floating-point number                                                                                                                              |
| `Bool`    | —                      | Boolean value: `true` / `false`                                                                                                                    |
| `Char`    | —                      | Unicode character                                                                                                                                  |
| `String`  | —                      | String                                                                                                                                             |

### 1.5 Identifiers

Identifiers begin with a letter or underscore, and subsequent characters can be letters, digits, or
underscores. Identifiers are case-sensitive.

Special identifiers:

- `_` is used as a placeholder to indicate a value to be ignored
- Identifiers beginning with an underscore denote private members

### 1.6 Literals

#### 1.6.1 Integers

```
Decimal     ::= [0-9][0-9_]*
Octal       ::= 0o[0-7][0-7_]*
Hex         ::= 0x[0-9a-fA-F][0-9a-fA-F_]*
Binary      ::= 0b[01][01_]*
```

#### 1.6.2 Floating-Point Numbers

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
Array       ::= '[' Expr (',' Expr)* ']'   // When target type is annotated as Array(T, N), the literal resolves to a fixed-length array
```

> **A dict literal requires at least one key-value pair**: `{}` is **not** an empty dict—it is an
> empty block (value `Void`, see [§2.9](#_2-9-block-expression)). For an empty dict, use the
> constructor `dict.new()`:
>
> ```yaoxiang
> empty = dict.new()                  // ✅ empty dict
> d = { "a": 1 }                       // ✅ dict literal
> wrong = {}                           // ❌ This is not a dict, it's an empty block (Void)
> ```
>
> **The criterion is content**: The `Dict` grammar requires at least one `String ':' Expr`. Since
> `{}` has no content to rely on, it takes the zero form of the block structure. Non-empty forms are
> self-describing by content (`{ "k": v }` has key-value pairs → dict)— this is of the same source
> as `f = { 5 }` being an `Int` value rather than a function: **the type is determined by content**.

> Set has no literal grammar and no runtime representation—set types will be completed following the
> Dict pattern (std.set + HeapValue::Set) when the need arises. The destination of List/Dict
> literals is determined by the context type annotation: bare literals and `List(T)` annotations
> resolve to growable lists; `Array(T, N)` annotations act directly on the literal to resolve to a
> fixed-length array. Implicit List→Array conversion is forbidden.
>
> Array literal semantics:
>
> - The number of elements must equal N; otherwise a compile-time E1002 error; an empty literal with
>   non-zero N is also rejected
> - Each element's type must be compatible with T; otherwise a compile-time E1002 error
> - N's grammatical form: only integer literals (possibly negative) or constant names; composite
>   expressions (e.g., `2+1`) are rejected at parse time
> - When N is a symbolic constant (function const parameter, e.g., `Array(Int, n)`), element count
>   validation is deferred to the refinement type phase
> - Nested array literals in v1 (e.g., `Array(Array(Int,2),2) = [[1,2],[3,4]]`) are rejected at
>   compile time; explicit construction layer by layer is required; recursive resolution is left for
>   a later version

#### 1.6.5 List Comprehensions

```
ListComp    ::= '[' Expr 'for' Identifier 'in' Expr (',' Expr)* ('if' Expr)? ']'
```

> **Behavior tightening (migration record)**: The iteration variable grammar was always
> `'for' Identifier 'in'`, but in the old implementation, the pattern went through full Pratt
> parsing—after `in` was registered as an infix operator, `x` would swallow `in items` into a
> membership expression. After the fix, non-identifier patterns fail to parse directly, no longer
> falling back to `_` like the old implementation (silently swallowing errors). Impact: previously
> parseable constructs like `[x for (a, b) in pairs]` now error—this form never had defined behavior
> (the variable was always `_`); the tightening direction is correct, with no semantic migration
> cost.

#### 1.6.6 Membership Test

```
Membership  ::= Expr 'in' Expr
```

> `in` is a binary relational operator returning `Bool`—returns `true` on hit, `false` on miss, and
> does not error. Semantic split: `[]` asserts existence and retrieves a value (errors on failure),
> while `in` asks whether something exists (a miss is a normal `false`). Right operand coverage:
> List / Array / Dict (key set) / Tuple / String (substring) / Range (interval). `in` is a
> first-class Hoare predicate, serving as the basis for compile-time provable propositions in the
> refinement type phase. (Set is excluded from the right operand list—Set has no runtime
> representation, see §1.6.4)

### 1.7 Comments

```
// single-line comment

/* multi-line comment
   can span multiple lines */
```

### 1.8 Indentation Rules

Code must use 4-space indentation; Tab characters are forbidden. This is a mandatory syntax rule.

---

## Chapter 2: Syntax Rules

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
| 1          | `()` `[]` `.` `?`           | Left-to-right |
| 2          | `as`                        | Left-to-right |
| 3          | Unary prefix `!` `-` `+`    | Right-to-left |
| 4          | `*` `/` `%`                 | Left-to-right |
| 5          | `+` `-`                     | Left-to-right |
| 6          | `..`                        | Left-to-right |
| 7          | `<<` `>>`                   | Left-to-right |
| 8          | `&` `\|` `^`                | Left-to-right |
| 9          | `==` `!=` `<` `>` `<=` `>=` | Left-to-right |
| 10         | `and` `or`                  | Left-to-right |
| 11         | `if...else`                 | Right-to-left |
| 12         | `=` `+=` `-=` `*=` `/=`     | Right-to-left |

> **Unary prefix operators** (`!` `-` `+`) are tightly bound: only lower than call and member
> access, higher than all binary operators. Therefore, `!a == b` ≡ `(!a) == b` (Zig-style
> semantics); `!` is a pure unary operation and does not participate in short-circuit control flow,
> orthogonal to the `and`/`or` keywords (short-circuit) (authoritative definition from RFC-010).

> **Range binding strength**: `..` has binding strength (6, 7)—left 6 is lower than addition (7),
> right 7 swallows addition but not the same level `..`. Before/after change comparison:
>
> | Expression   | Before (level 1, right-associative)                                                           | After ((6,7), left-associative)                                                |
> | ------------ | --------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------ |
> | `x in 1..10` | `x in 1..10` (`in` right operand level 4, `..` level 1 cannot swallow, actually cannot parse) | `x in (1..10)`—the entire range serves as `in`'s right operand                 |
> | `0..n+2`     | `(0..n)+2` (right-associative trap: the upper bound gets eaten, `for` loop directly E3004)    | `0..(n+2)`—the upper bound is an arithmetic expression                         |
> | `a == b..c`  | `a == (b..c)` (`..` level 1 < `==` level 3, naturally as a whole)                             | `a == (b..c)`—**semantics unchanged**, `..` still higher than comparison level |
> | `1..2*3`     | `(1..2)*3`                                                                                    | `1..(2*3)`—the upper bound is an arithmetic expression                         |
> | `a..b..c`    | `a..(b..c)` (right-associative chain, meaningless Range nested in Range)                      | `(a..b)..c`—**step form** (`c` is the step)                                    |
>
> Net effect: compound upper bound `for i in 0..n+2` changes from "parses but E3004" to "directly
> usable"; `x in 1..10` changes from "unparseable" to "interval check"; `a..b..c` changes from
> "meaningless nesting" to "step component". Level 6 falls between `+` (level 5) and `<<` (level 7),
> following mathematical convention: a range is a tightly bound construct, so the upper bound is
> naturally a complete arithmetic expression.

### 2.3 Function Calls

```
FnCall      ::= Expr '(' ArgList? ')'
ArgList     ::= Expr (',' Expr)* (',' NamedArg)* | NamedArg (',' NamedArg)*
NamedArg    ::= Identifier '=' Expr
```

Named arguments use `name = value` (RFC-010 §Function Definition, RFC-011 §Construction Form).
Positional arguments must precede named arguments; parameters specified in order can be arranged in
any order when used as named arguments:

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b

add(3, 5)          // positional
add(a = 3, b = 5)  // named
add(b = 5, a = 3)  // order arbitrary
add(3, b = 5)      // mixed, positional first
```

A wrong name in a named argument reports **E1014**, the same formal parameter specified both
positionally and by name reports **E1015**, and mismatched count reports **E1010** (RFC-013).

### 2.4 Member Access

```
MemberAccess::= Expr '.' Identifier
```

### 2.5 Index Access

```
IndexAccess ::= Expr '[' Expr ']'
```

> **Three-layer semantics (RFC-011b)**: `a[i]` dispatches based on `a`'s type—① Built-in containers
> (List/Vec/Array/Dict/Tuple) go through native index instructions (fast path); ② User types that
> implement the `Index` interface dispatch to their `index` method (`Index(Grid, Int, Float)`
> instantiation in the type body + `Grid.index` method); ③ RFC-004's `f[0]` positional binding only
> exists in binding declarations and does not go through this grammar. Multi-dimensional indices
> `a[0, 1]` have their keys packed as tuples. Other types that have not implemented `Index` are
> rejected at the type level.

### 2.6 Type Cast

```
TypeCast    ::= Expr 'as' TypeExpr
```

### 2.7 Conditional Expression

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

### 2.9 Block Expression

```
Block       ::= '{' Stmt* Expr? '}'
```

> **Statement termination rules**: The separation and newline behavior between Stmts (`;` explicit
> separation, newline termination, line continuation exceptions, line-leading `(`/`[` never merging)
> are defined by [RFC-038](../design/rfc/draft/038-statement-termination.md).

#### 2.9.1 Three Forms of `{`

`{` has exactly three interpretations in expression position, determined once by **content**:

| Form             | Notation          | Type                 | Example          |
| ---------------- | ----------------- | -------------------- | ---------------- |
| **Empty block**  | `{}`              | `Void`               | `x: Void = {}`   |
| **Dict literal** | `{ "k": v, ... }` | `Dict(K, V)`         | `d = { "a": 1 }` |
| **Block**        | `{ Stmt* Expr? }` | Tail expression type | `y = { 1 + 1 }`  |

**Determination order**:

1. `{}` (no content) → **Empty block**, value `Void`
2. First element is a `String ':' Expr` key-value pair → **Dict literal**
3. Otherwise → **Block**, value given by the tail expression

> **Why `{}` is not an empty dict**: The "emptiness" of an empty dict cannot be self-describing (it
> can be either `Dict(K, V)` or an empty block), while the dict grammar
> [§1.6.4](#_1-6-4-collections) requires at least one key-value pair. When there is no content to
> rely on, it takes the zero form of the block structure: this is consistent with `unsafe {}` /
> `spawn {}`, introducing no special cases. Use `dict.new()` for an empty dict.
>
> **Why functions need annotation**: `f = { stmt }` is a **value** (tail expression type), not a
> function. To define a function, write the Fn annotation: `f: () -> Int = { 5 }`. This follows the
> same principle as dicts: **the type is determined by content**, not by the presence of an
> annotation. (This rule is in [RFC-010a](./design/rfc/accepted/010a-tail-expression-and-return.md)
> Appendix D.)

**Unified semantics**: The value of all `{}` blocks is given by the **tail expression**, and
`return` is a non-local exit of type `Never`.

| Block type  | Value exit      | Empty block `{}` |
| ----------- | --------------- | ---------------- |
| Plain `{}`  | Tail expression | `Void`           |
| `unsafe {}` | Tail expression | `Void`           |
| `spawn {}`  | Tail expression | `Void`           |

**Core principles** (see [RFC-010a](../design/rfc/accepted/010a-tail-expression-and-return.md) for
details):

- **The value of a block = the tail expression** (the last expression), the only exit, with no
  exceptions
- When the last position is an **assignment statement**, the block value is `Void`; to get `Void`,
  write `Void` explicitly
- **`return` exits the nearest function boundary** (passes through all blocks, does not "return to a
  block"), with type `Never`; `Never <: T`

> holds for any type (principle of explosion), so it can appear in any return type position

- The expression form `= expr` directly gives a value

```yaoxiang
// Plain {} block: the tail expression gives the value
result = {
    x = compute()
    x                // the block's value
}

// unsafe {} block: the tail expression gives the type definition
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void
    }
    SqliteDb         // the block's value
}

// spawn {} block: the tail expression gives the result
(a, b) = spawn {
    result1 = fetch("url1"),
    result2 = fetch("url2")
    (result1, result2)   // the block's value
}

// return: passes through blocks, exits the function
f: (n: Int) -> Int = {
    if n < 0 {
        return 0     // passes out of if and the function body, exits the function
    }
    n * 2            // tail expression
}
```

#### Is `name = { ... }` a Function or a Block Value?

`name = { ... }` can be either a function definition (RFC-007 "empty parameter minimal form") or a
block value binding. Adjudicated by **annotation priority, default function** (RFC-010a Appendix D):

| Case                              | Result          | Example                            |
| --------------------------------- | --------------- | ---------------------------------- |
| Value is a Lambda (`=>`)          | Function        | `f = () => 5` → `f()` = 5          |
| Annotation is a function type     | Function        | `f: () -> Int = { 5 }` → `f()` = 5 |
| Annotation is a non-function type | **Block value** | `x: Int = { 5 }` → `x` = 5         |
| No annotation                     | Function        | `f = { 5 }` → `f()` = 5            |

**Annotation is the type**: `x: Int = ...` declares `x` to be `Int`, so `{ ... }` evaluates to
`Int`; `f: () -> Int = ...` declares `f` to be a function, so `{ ... }` is the function body.

To evaluate `{ ... }` on the spot, **just write the target type** (no new syntax needed):

```yaoxiang
// Block value: immediately evaluated
x: Int = {
    y = 5
    y            // x = 5
}

// Function: no annotation, defaults to function
f = { 5 }        // f() = 5
```

#### Nested Function Types: Currying or Returning a Function?

The nested function type on the right side of `->` has two readings, distinguished by
**parentheses** (RFC-004):

| Notation                        | Meaning                | Call                      |
| ------------------------------- | ---------------------- | ------------------------- |
| `(a: Int) -> (b: Int) -> Int`   | **Currying**           | `f(1)(2)`                 |
| `(a: Int) -> ((b: Int) -> Int)` | **Returns a function** | `g(1)` returns a function |

Basis: **Annotation is the type**. `g: (a: Int) -> ((b: Int) -> Int)` declares that
`g(1) : (b: Int) -> Int`, so `g(1)` must **be** that function, not "the next segment of parameters."

```yaoxiang
// Currying: two parameter segments given layer by layer
add: (a: Int) -> (b: Int) -> Int = { a + b }
add(1)(2)        // → 3

// Returning a function: the outer layer has one parameter segment, what is returned is the function
adder: (n: Int) -> ((x: Int) -> Int) = (x) => x + n
adder(10)(5)     // → 15
h = adder(10)    // the returned function can be stored in a variable, can be passed around
```

Nested `Fn` without parentheses is always currying (including RFC-011's type parameter form):

```yaoxiang
identity: (T: Type) -> (x: T) -> T = (x) => x   // currying
identity(5)      // → 5
```

**Type check**: The parentheses declare the return type, and the body must produce that type.
`f: () -> (() -> Int) = { 7 }` is an error—expected `() -> Int`, actually got `Int` (E1002).

### 2.10 Lambda Expression

```
Lambda      ::= '(' ParamList? ')' '=>' Expr
            |  '(' ParamList? ')' '=>' Block
```

### 2.11 Error Propagation Operator

```
ErrorPropagate ::= Expr '?'
```

The `?` operator is a postfix operator with the same precedence as `.`. For the `Result(T, E)` type:

- For `Ok(v)`, extract the value `v` and continue execution
- For `Err(e)`, propagate the error upward (`return Err(e)`)

```yaoxiang
process: (data: Data) -> Result(Data, Error) = {
    validated = validate(data)?     // extract value on success, propagate upward on failure
    transform(validated)
}
```

### 2.12 Range Expression

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

> **Step semantics**: In `a..b..c`, `c` is the step. `c = 0` literal is rejected at compile time;
> dynamic `c` has no runtime check (E6001 family; will be elevated to Result when the error system
> lands). `c < 0` is legal, and the interval direction reverses with the sign (`10..0..(-2)` is
> decreasing).

### 2.13 ref Expression

```
RefExpr     ::= 'ref' Expr
```

`ref` creates a shared hold. The compiler automatically chooses Rc (single-task) or Arc
(cross-task); users do not need to care about implementation details.

```yaoxiang
data = ref heavy_data
spawn { use(data) }   // cross-task: the compiler automatically picks Arc
```

### 2.14 unsafe Expression

```
UnsafeExpr  ::= 'unsafe' Block
```

`unsafe` blocks are used to define opaque types and operate on raw pointers. Use `return` to return
a type definition to the enclosing scope.

**Semantics**:

- Type definitions and raw pointer operations are allowed inside `unsafe {}`
- The returned type is available outside the `unsafe {}` block
- Field access of the type requires unsafe permission

```yaoxiang
// Define an opaque type in an unsafe block
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  // raw pointer
    }
    return SqliteDb
}

// SqliteDb is available outside the unsafe block
db = sqlite3_open("test.db")
```

### 2.15 Scope

**Basic rules**:

- Each `{}` block creates a scope
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

- `x = value`: search outward along the scope chain for x, assign if found, declare a new one if not
  found
- `mut x = value`: explicit new mutable declaration, prohibited from having the same name as an
  outer one
- Any name can only be declared once within the same scope

> **Detailed definition**: For complete scope rules, variable declaration, and shadowing mechanisms,
> see [Module System Specification](./modules.md#chapter-4-scope).

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

**Semantics**: `return` is a **non-local exit** that exits the nearest **function boundary**
(passing through all blocks—including `if` / `while` / `for` / `match` / bare blocks / `spawn` /
`unsafe`), handing the value to the caller. It does **not "return to a block."**

**Type**: `return e : Never` (with `e : T`). `Never <: T'` holds for any `T'` (principle of
explosion, see [Type System §2.2](./type-system.md)), so `return` can appear in any return type
position without needing additional rules.

**Relationship with block evaluation**: The value of a block is always the **tail expression** (see
§2.9). As a block, `{ return n }` has value `n` with type `Never`; at the same time, `return`'s
effect is to exit the function. **Both things hold simultaneously**, coexisting via the principle of
explosion.

`return` and the tail expression together enable "early return" without needing additional rules for
`return` specifically pointing to a function—see
[RFC-010a](../design/rfc/accepted/010a-tail-expression-and-return.md).

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

**Semantics**: Immediately terminates the innermost enclosing `while`/`for` loop, with control flow
transferring to after that loop's body.

- **Exits only the nearest layer**: `break`

> always acts on the innermost loop containing it. When you need to break out of multiple layers in
> nested loops, extract the inner loop into a function and use `return`, or use a flag
> (break/continue has no label; if loop labels are introduced in the future, they will follow the
> RFC process via loop declaration-side syntax, adjudicated together with the multi-exit design of
> the proof pipeline)

- **Limited to inside the loop body**: `break` can only appear inside `while`/`for`

> loop bodies (including blocks/if/match nested within the body); appearing outside a loop is a
> compile-time error (E1102 `'break' outside of a loop`)

- **Does not affect termination proof**: `while` loops must still be provably terminating (decreases
  metric); `break`

> does not participate in termination arguments; `while true { break }` will not be accepted

- **Borrow semantics**: The control-flow edge of break participates in the structural cut of
  RFC-009a's reverse BFS liveness analysis (jumped-out iterations do not participate in back-edge
  liveness derivation)

```yaoxiang
mut i = 0
while i < 10 {
    i = i + 1
    if i == 3 {
        break              // control flow transfers to after the loop, i == 3
    }
}

// Nested loops: break only exits the inner loop
while j < 3 {
    while k < 10 {
        if k == 2 { break }    // only terminates the inner loop
    }
    j = j + 1                  // each iteration of the outer loop executes up to here
}
```

### 3.5 continue Statement

```
ContinueStmt::= 'continue'
```

**Semantics**: Skips the remaining statements in the current iteration, directly entering the next
iteration of the innermost enclosing loop—`while` returns to condition re-evaluation, `for` takes
the next element.

- **Acts only on the nearest layer**: Same as `break`, no label
- **Limited to inside the loop body**: Appearing outside a loop is a compile-time error (E1102)

```yaoxiang
mut sum = 0
mut n = 0
while n < 5 {
    n = n + 1
    if n == 3 {
        continue           // skip the following accumulation, n == 3 not counted
    }
    sum = sum + n
}
// sum == 12 (1 + 2 + 4 + 5)
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

#### 3.9.1 Semantics: Each Iteration Binds a New Value

YaoXiang's for loop semantics differ from traditional languages: **each iteration binds a new value,
rather than modifying the same variable**.

```yaoxiang
// Example: for i in 1..5
for i in 1..5 {
    print(i)
}
```

**Execution process**:

| Iteration | Behavior of the loop variable                                                         |
| --------- | ------------------------------------------------------------------------------------- |
| 1st       | Create new binding `i = 1`, loop body executes, prints 1                              |
| 2nd       | Create new binding `i = 2` (previous binding destroyed), loop body executes, prints 2 |
| 3rd       | Create new binding `i = 3`, loop body executes, prints 3                              |
| 4th       | Create new binding `i = 4`, loop body executes, prints 4                              |
| End       | Loop body ends, binding destroyed                                                     |

**Key point**: After each iteration ends, the binding created by that iteration is destroyed. The
next iteration is a completely new binding, with no relationship to the previous iteration's
binding.

#### 3.9.2 Difference between `for` and `for mut`

| Syntax              | Loop variable mutability | Description                                    |
| ------------------- | ------------------------ | ---------------------------------------------- |
| `for i in 1..5`     | Immutable                | Cannot modify the binding inside the loop body |
| `for mut i in 1..5` | Mutable                  | Can modify the binding inside the loop body    |

```yaoxiang
// Legal: each iteration binds a new value, no modification needed
for i in 1..5 {
    print(i)  // read i's value
}

// Error: immutable binding, cannot modify
for i in 1..5 {
    i = i + 1  // error: cannot modify an immutable binding
}

// Legal: using for mut allows modifying the binding
for mut i in 1..5 {
    i = i + 1  // modification allowed
}
```

#### 3.9.3 Shadowing Check

YaoXiang prohibits variable shadowing. A for loop variable cannot have the same name as a variable
in an outer scope:

```yaoxiang
// Error: i is already declared outside
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

This rule applies to all code blocks; see [4.3 Shadowing Rules](./modules.md#43-shadowing-rules).

#### 3.9.4 Comparison with Other Languages

| Language | For loop variable semantics                                |
| -------- | ---------------------------------------------------------- |
| YaoXiang | Each iteration binds a new value                           |
| Rust     | Modifies the same variable (requires mut)                  |
| Python   | Modifies the same variable (no mut needed)                 |
| C/C++    | Modifies the same variable (requires pointer or reference) |

**Design rationale**: YaoXiang adopts binding semantics because:

1. **More aligned with natural semantics** In natural language, "for each element x in the
   collection" means each x is an independent individual. YaoXiang's `for i in 1..5` is read as "for
   each i from 1 to 5", and each iteration's i is a brand-new binding, consistent with human
   intuitive understanding.

2. **Avoids accidental modification** The default immutable binding semantics means the loop
   variable cannot be accidentally modified inside the loop body. No need to worry about writing
   `i = ...` somewhere in a complex loop body and causing hard-to-trace bugs.

3. **High-performance solution is within reach** When there is a genuine need to reuse a variable
   across iterations (e.g., accumulators, caches), use `for mut` to declare and switch to mutable
   binding mode. This is clearer than implicit shared state—the intent is explicitly expressed
   through syntax, not hidden in runtime behavior.

### 3.10 spawn Statement

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' 'mut'? Identifier 'in' Expr '{' Expr '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
```

**spawn block**: Explicitly declares a concurrency domain, and the expressions inside the block
execute concurrently.

```yaoxiang
(result_a, result_b) = spawn {
    parse(fetch("url1")),
    parse(fetch("url2"))
}
```

**spawn loop**: Data-parallel loop.

```yaoxiang
results = spawn for item in items {
    process(item)
}
```

**The spawn block captures outer variables** (RFC-024 §2.3, value capture semantics):

- When the block body references an outer variable, it is

> **Move value capture**: the value is snapshotted into the closure environment at the spawn
> creation point, and the block body reads via env (LoadUpvalue)

- **Primitives** (Int/Float/Bool/Char) are value-copied; the outer variable is not affected
- **Handle types** (Struct/String/List, etc.) snapshot = handle copy, sharing the underlying object;
  in the Embedded runtime (default) with same-thread same-heap, the handle is valid
- Sharing among multiple tasks requires explicit `ref` (§2.13, the compiler automatically picks
  Rc/Arc)
- Outer variables referenced by `return` inside the block are also captured

```yaoxiang
t1 = 1 + 1
t2 = 2 + 2
result = spawn {
    return t1 + t2    // t1/t2 value capture, result == 6
}
```

---

### 3.11 Program Entry and Top-Level Statements

A source file has two roles in the compiler's view, **determined by whether `yaoxiang.toml`
exists**:

| Role       | Determination                                | Program body                                                                       |
| ---------- | -------------------------------------------- | ---------------------------------------------------------------------------------- |
| **Script** | Single file run directly, no `yaoxiang.toml` | **Top-level statements** (executed in source order); `main` is an ordinary binding |
| **Bin**    | `yaoxiang.toml` exists                       | **`main` function**; top-level cannot have executable statements                   |

#### Script: Top-Level Statements Are the Program

Without a manifest, the file is a "script", and **top-level statements are executed in source
order**:

```yaoxiang
use std.io
io.println("hello")          // executed directly
x: Int = { 42 }              // top-level binding: runtime initialization
io.println(x)                // 42
```

In this mode, `main` is **not special**—it is just an ordinary binding. To make `main` run, you must
call it explicitly:

```yaoxiang
use std.io
main: () -> Void = { io.println("only runs if called") }

main()                       // ← This line must be written
```

> **Why isn't `main` called automatically?** Top-level statements are already the program body. If
> `main` were implicitly called additionally, a script that explicitly writes `main()` would execute
> twice. The two rules cannot coexist, so under Script, there is only the "top-level statements"
> execution entry point.

#### Bin: `main` is the Entry Point

When a manifest exists, the file is an "executable target", at which point:

- `main` must be defined, and it must be a function (signature must be zero-parameter-callable)
- Top-level does not allow executable statements—the program body is `main`

```yaoxiang
main: () -> Void = {
    print("hello")
}
```

Missing `main` or `main` not being a function are both compile errors (the former: without `main`
all functions are unreachable; the latter: value bindings will not be called).

> **Library files**: Files used by other files via `use`, or files pointed to by `[lib].path` /
> `[exports]` do not require `main`—they are not program entry points.

#### Top-Level Binding Initialization

The initialization value of a top-level binding **is evaluated at runtime**, and is not required to
be a compile-time constant:

```yaoxiang
answer: Int = { 42 }              // block value
inc: (Int) -> Int = (x) => x + 1
computed: Int = inc(41)           // function call
```

Initialization is executed in **dependency order**, independent of source order:

```yaoxiang
derived: Int = base * 3           // references base declared later
base: Int = 7                     // initialized first (topological sort)
```

Circular dependencies are a compile error (the names on the cycle are listed):

```yaoxiang
a: Int = b + 1
b: Int = a + 1                    // error: a → b → a
```

> **Design basis**: RFC-029f (file role model), RFC-010a Appendix D (block binding adjudication).

---

## Appendix: Syntax Quick Reference

### A.1 Control Flow

```
if Expr Block (else if Expr Block)* (else Block)?
match Expr { MatchArm+ }
while Expr Block
for 'mut'? Identifier 'in' Expr Block
break | continue          // only inside loop bodies (§3.4 / §3.5)
```

### A.2 Error Handling

```
Expr '?'              // error propagation (Result type)
```

### A.3 match Syntax

```
match value {
    pattern1 => expr1,
    pattern2 if guard => expr2,
    _ => default_expr,
}
```
