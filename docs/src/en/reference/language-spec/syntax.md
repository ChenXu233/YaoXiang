# Syntax Specification

This document defines the syntax specification of the YaoXiang programming language, including
lexical structure, grammar rules, and operator precedence.

---

## Chapter 1: Lexical Structure

### 1.1 Source Files

YaoXiang source files must use UTF-8 encoding. Source files typically use the `.yx` extension.

### 1.2 Lexical Token Categories

| Category   | Description                         | Examples                  |
| ---------- | ----------------------------------- | ------------------------- |
| Identifier | Starts with a letter or underscore  | `x`, `_private`, `my_var` |
| Keyword    | Language pre-defined reserved words | `Type`, `pub`, `use`      |
| Literal    | Fixed values                        | `42`, `"hello"`, `true`   |
| Operator   | Operation symbols                   | `+`, `-`, `*`, `/`        |
| Delimiter  | Syntax separators                   | `(`, `)`, `{`, `}`, `,`   |

### 1.3 Keywords

YaoXiang defines a minimal set of keywords:

```
pub    use    spawn
ref    mut    if     else
else   match  while  for    return
break  continue as     in     unsafe
```

These keywords have special meaning in any context and cannot be used as identifiers.

### 1.4 Reserved Words

YaoXiang's "reserved words" are organized into three layers, recognized at different stages by the
parser and type checker:

#### 1.4.1 Literal Reserved Words

The parser has independent tokens for these literal identifiers, which cannot be used as ordinary
identifiers:

| Identifier | Owning Type | Description                                                                                                       |
| ---------- | ----------- | ----------------------------------------------------------------------------------------------------------------- |
| `Type`     | —           | Meta type keyword                                                                                                 |
| `true`     | Bool        | Boolean true value                                                                                                |
| `false`    | Bool        | Boolean false value                                                                                               |
| `void`     | Void        | Void literal (Unit value). Lowercase `void` is the value literal; uppercase `Void` is the type name (see §1.4.3). |

#### 1.4.2 Constructor Expressions

The following constructors are recognized by the parser in pattern matching and expression contexts:

| Constructor | Owning Type | Description                      |
| ----------- | ----------- | -------------------------------- |
| `some(T)`   | Option      | Option value variant constructor |
| `ok(T)`     | Result      | Result success variant           |
| `err(E)`    | Result      | Result error variant             |

#### 1.4.3 Built-in Type Names

The following type names are pre-registered by the type checker and can be used in type positions
without import. The parser treats them as ordinary identifiers—**they are not reserved words and can
be shadowed by local bindings (not recommended)**.

| Type Name | Logical Correspondence | Description                                                                                                                                  |
| --------- | ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `Void`    | ⊤ (True/Unit)          | Zero-field product type, exactly one inhabitant (the `void` literal, see §1.4.1)                                                             |
| `Never`   | ⊥ (False/Empty type)   | Zero-variant sum type, zero inhabitants. No expression can produce a `Never` value. `Never <: T` holds for all `T` (principle of explosion). |
| `Int`     | —                      | Signed integer                                                                                                                               |
| `Float`   | —                      | Floating-point number                                                                                                                        |
| `Bool`    | —                      | Boolean value: `true` / `false`                                                                                                              |
| `Char`    | —                      | Unicode character                                                                                                                            |
| `String`  | —                      | String                                                                                                                                       |

### 1.5 Identifiers

Identifiers start with a letter or underscore, followed by letters, digits, or underscores.
Identifiers are case-sensitive.

Special identifiers:

- `_` is used as a placeholder to indicate an ignored value
- Identifiers starting with an underscore indicate private members

### 1.6 Literals

#### 1.6.1 Integers

```
Decimal     ::= [0-9][0-9_]*
Octal       ::= 0o[0-7][0-7_]*
Hex         ::= 0x[0-9a-fA-F][0-9a-fA-F_]*
Binary      ::= 0b[01][01_]*
```

#### 1.6.2 Floating-point Numbers

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
Array       ::= '[' Expr (',' Expr)* ']'   // When the target type annotation is Array(T, N), the literal becomes a fixed-length array
```

> **Dictionary literals require at least one key-value pair**: `{}` is **not** an empty
> dictionary—it is an empty block (value `Void`, see [§2.9](#_2-9-block-expression)). For an empty
> dictionary, use the constructor `dict.new()`:
>
> ```yaoxiang
> empty = dict.new()                  // ✅ Empty dictionary
> d = { "a": 1 }                       // ✅ Dictionary literal
> wrong = {}                           // ❌ This is not a dictionary; it is an empty block (Void)
> ```
>
> **The criterion is content**: The `Dict` grammar requires at least one `String ':' Expr`. Since
> `{}` has no content to rely on, it takes the zero form of block structure. The non-empty form is
> self-describing by content (`{ "k": v }` has key-value pairs → dictionary)—this is the same
> principle as `f = { 5 }` being an `Int` value rather than a function: **the type is determined by
> the content**.

> Set has no literal grammar and no runtime representation—collection types are planned; when the
> need arises, they will be completed following the Dict pattern (std.set + HeapValue::Set). The
> landing point of List/Dict literals is determined by the contextual type annotation: a bare
> literal or a `List(T)` annotation lands as a growable list; an `Array(T, N)` annotation applied
> directly to a literal lands as a fixed-length array. Implicit List→Array conversion is forbidden.
>
> Array literal semantics:
>
> - The number of elements must equal N; otherwise compile-time E1002; an empty literal paired with
>   non-zero N is likewise rejected
> - Each element type must be compatible with T; otherwise compile-time E1002
> - The grammatical form of N: only integer literals (possibly negative) or constant names;
>   composite expressions (such as `2+1`) are rejected at parse time
> - When N is a symbolic constant (function const parameter, e.g., `Array(Int, n)`), the count check
>   is deferred to the refined type stage
> - v1 nested array literals (`Array(Array(Int,2),2) = [[1,2],[3,4]]`) are rejected at compile time;
>   explicit construction layer by layer is required; recursive landing is reserved for future
>   versions

#### 1.6.5 List Comprehensions

```
ListComp    ::= '[' Expr 'for' Identifier 'in' Expr (',' Expr)* ('if' Expr)? ']'
```

> **Behavior tightening (migration record)**: The grammar for the iteration variable was always
> `'for' Identifier 'in'`, but in the old implementation the pattern went through full pratt
> parsing—after `'in'` was registered as an infix operator, `x` would swallow `in items` into a
> membership expression. After the fix, non-identifier patterns fail to parse directly, no longer
> falling back to `_` as the old implementation did (silently swallowing errors). Impact: previously
> parseable constructs like `[x for (a, b) in pairs]` now report an error—this form never had
> defined behavior (the variable was always `_`), so the tightening direction is correct with no
> semantic migration cost.

#### 1.6.6 Membership Testing

```
Membership  ::= Expr 'in' Expr
```

> `in` is a binary relational operator returning `Bool`—`true` on hit, `false` on miss, no error.
> Semantic split: `[]` asserts existence and retrieves the value (errors on failure), while `in`
> asks whether the element exists (miss is a normal `false`). Right operand coverage: List / Array /
> Dict (key set) / Tuple / String (substring) / Range (interval). `in` is a first-class Hoare
> predicate, serving as the base of compile-time provable propositions at the refined type stage.
> (Set is removed from the right operand list—Set has no runtime representation, see §1.6.4)

### 1.7 Comments

```
// Single-line comment

/* Multi-line comment
   can span multiple lines */
```

### 1.8 Indentation Rules

Code must use 4 spaces for indentation; Tab characters are forbidden. This is a mandatory syntax
rule.

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
| 1          | `()` `[]` `.` `?`           | Left to right |
| 2          | `as`                        | Left to right |
| 3          | Unary prefix `!` `-` `+`    | Right to left |
| 4          | `*` `/` `%`                 | Left to right |
| 5          | `+` `-`                     | Left to right |
| 6          | `..`                        | Left to right |
| 7          | `<<` `>>`                   | Left to right |
| 8          | `&` `\|` `^`                | Left to right |
| 9          | `==` `!=` `<` `>` `<=` `>=` | Left to right |
| 10         | `and` `or`                  | Left to right |
| 11         | `if...else`                 | Right to left |
| 12         | `=` `+=` `-=` `*=` `/=`     | Right to left |

> **Unary prefix operators** (`!` `-` `+`) bind tightly: only below call and member access, and
> above all binary operators. Hence `!a == b` ≡ `(!a) == b` (Zig-style semantics); `!` is a pure
> unary operation that does not participate in short-circuit control flow and is orthogonal to the
> `and`/`or` keywords (short-circuit) (authoritative definition in RFC-010).

> **Range binding power**: `..` has binding power (6, 7)—left 6 is below addition (7), right 7
> swallows addition but not same-level `..`. Before-and-after comparison:
>
> | Expression   | Before change (level 1, right-associative)                                                            | After change ((6,7), left-associative)                                         |
> | ------------ | ----------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------ |
> | `x in 1..10` | `x in 1..10` (right operand of `in` at level 4, `..` at level 1 cannot swallow; actually unparseable) | `x in (1..10)`—the range as a whole is the right operand of `in`               |
> | `0..n+2`     | `(0..n)+2` (right-associativity trap: the upper bound is eaten, `for` loop directly E3004)            | `0..(n+2)`—the upper bound is an arithmetic expression                         |
> | `a == b..c`  | `a == (b..c)` (`..` at level 1 < `==` at level 3, naturally whole)                                    | `a == (b..c)`—**semantics unchanged**, `..` still higher than comparison level |
> | `1..2*3`     | `(1..2)*3`                                                                                            | `1..(2*3)`—the upper bound is an arithmetic expression                         |
> | `a..b..c`    | `a..(b..c)` (right-associative chain, meaningless Range inside Range)                                 | `(a..b)..c`—**step form** (`c` is the step)                                    |
>
> Net effect: the composite upper bound `for i in 0..n+2` goes from "parses successfully but E3004"
> to "directly usable"; `x in 1..10` goes from "unparseable" to "interval check"; `a..b..c` goes
> from "meaningless nesting" to "step component". Level 6 falls between `+` (level 5) and `<<`
> (level 7), a mathematical convention: an interval is a tightly-bound construct whose upper bound
> is naturally a complete arithmetic expression.

### 2.3 Function Calls

```
FnCall      ::= Expr '(' ArgList? ')'
ArgList     ::= Expr (',' Expr)* (',' NamedArg)* | NamedArg (',' NamedArg)*
NamedArg    ::= Identifier '=' Expr
```

Named arguments use `name = value` (RFC-010 §Function Definition, RFC-011 §Construction Form).
Positional arguments must come before named arguments; order-specified parameters can appear in any
order among named arguments:

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b

add(3, 5)          // Positional
add(a = 3, b = 5)  // Named
add(b = 5, a = 3)  // Any order
add(3, b = 5)      // Mixed, positional first
```

A wrong name in a named argument reports **E1014**, specifying the same parameter both positionally
and by name reports **E1015**, and a count mismatch reports **E1010** (RFC-013).

### 2.4 Member Access

```
MemberAccess::= Expr '.' Identifier
```

### 2.5 Index Access

```
IndexAccess ::= Expr '[' Expr ']'
```

> **Three-layer semantics (RFC-011b)**: `a[i]` is dispatched based on `a`'s type—① built-in
> containers (List/Vec/Array/Dict/Tuple) go through native indexing instructions (fast path); ② user
> types that implement the `Index` interface dispatch to their `index` method
> (`Index(Grid, Int, Float)` instantiation within the type body + the `Grid.index` method); ③
> RFC-004's `f[0]` position binding only exists in binding declarations and does not go through this
> grammar. The key of multi-dimensional indexing `a[0, 1]` is packed as a tuple. Other types that do
> not implement `Index` are rejected at the type level.

### 2.6 Type Conversion

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

> **Statement termination rules**: The separator and newline behavior between Stmts (explicit `;`
> separator, newline termination, line continuation exceptions, never-merge rule for line-leading
> `(`/`[`) is defined by [RFC-038](../../design/rfc/accepted/038-statement-termination.md).

#### 2.9.1 The Three Forms of `{`

`{` has exactly three interpretations in an expression position, determined at once by **content**:

| Form                   | Notation          | Type                 | Example          |
| ---------------------- | ----------------- | -------------------- | ---------------- |
| **Empty block**        | `{}`              | `Void`               | `x: Void = {}`   |
| **Dictionary literal** | `{ "k": v, ... }` | `Dict(K, V)`         | `d = { "a": 1 }` |
| **Block**              | `{ Stmt* Expr? }` | Tail expression type | `y = { 1 + 1 }`  |

**Determination order**:

1. `{}` (no content) → **empty block**, value `Void`
2. First element is a `String ':' Expr` key-value pair → **dictionary literal**
3. Otherwise → **block**, value given by the tail expression

> **Why `{}` is not an empty dictionary**: The "empty" of an empty dictionary cannot be
> self-describing (it could be either `Dict(K, V)` or an empty block), while the dictionary grammar
> [§1.6.4](#_1-6-4-collections) requires at least one key-value pair. When there is no content to
> rely on, the zero form of block structure is taken—this is consistent with `unsafe {}` /
> `spawn {}` and introduces no special case. For an empty dictionary, use `dict.new()`.
>
> **Why functions need annotations**: `f = { stmt }` is a **value** (tail expression type), not a
> function. To define a function, write the Fn annotation explicitly: `f: () -> Int = { 5 }`. This
> follows the same principle as dictionaries: **the type is determined by the content**, not by the
> presence of an annotation. (This rule is in
> [RFC-010a](../../design/rfc/accepted/010a-tail-expression-and-return.md) Appendix D.)

**Unified semantics**: The value of every `{}` block is given by the **tail expression**; `return`
is a non-local exit of type `Never`.

| Block type  | Value exit      | Empty block `{}` |
| ----------- | --------------- | ---------------- |
| Plain `{}`  | Tail expression | `Void`           |
| `unsafe {}` | Tail expression | `Void`           |
| `spawn {}`  | Tail expression | `Void`           |

**Core principles** (see [RFC-010a](../../design/rfc/accepted/010a-tail-expression-and-return.md)
for details):

- **The value of a block = tail expression** (the last expression), the only exit, with no
  exceptions
- When the final position is an **assignment statement**, the block value is `Void`; if you want
  `Void`, write `Void` explicitly
- **`return` exits the nearest function boundary** (penetrating all blocks, not "returning to the
  block"), type `Never`; `Never <: T` holds for any type (principle of explosion), so it can appear
  at any return type position
- The expression form `= expr` directly gives the value

```yaoxiang
// Plain {} block: tail expression gives the value
result = {
    x = compute()
    x                // The value of the block
}

// unsafe {} block: tail expression gives the type definition
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void
    }
    SqliteDb         // The value of the block
}

// spawn {} block: tail expression gives the result
(a, b) = spawn {
    result1 = fetch("url1"),
    result2 = fetch("url2")
    (result1, result2)   // The value of the block
}

// return: penetrates blocks and exits the function
f: (n: Int) -> Int = {
    if n < 0 {
        return 0     // Penetrates if and function body, exits the function
    }
    n * 2            // Tail expression
}
```

#### Is `name = { ... }` a Function or a Block Value?

`name = { ... }` could be either a function definition (RFC-007 "simplest empty parameter") or a
block value binding. Adjudication is by **annotation priority, default function** (RFC-010a Appendix
D):

| Situation                         | Result          | Example                            |
| --------------------------------- | --------------- | ---------------------------------- |
| Value is a Lambda (`=>`)          | Function        | `f = () => 5` → `f()` = 5          |
| Annotation is a function type     | Function        | `f: () -> Int = { 5 }` → `f()` = 5 |
| Annotation is a non-function type | **Block value** | `x: Int = { 5 }` → `x` = 5         |
| No annotation                     | Function        | `f = { 5 }` → `f()` = 5            |

**Annotation is the type**: `x: Int = ...` declares that `x` is `Int`, so `{ ... }` evaluates to
`Int`; `f: () -> Int = ...` declares that `f` is a function, so `{ ... }` is the function body.

If you want `{ ... }` to be evaluated immediately, **just write the target type** (no new syntax
needed):

```yaoxiang
// Block value: evaluated immediately
x: Int = {
    y = 5
    y            // x = 5
}

// Function: defaults to function without annotation
f = { 5 }        // f() = 5
```

#### Nested Function Types: Currying or Returning a Function?

A nested function type on the right side of `->` has two readings, distinguished by **parentheses**
(RFC-004):

| Notation                        | Meaning                  | Call                      |
| ------------------------------- | ------------------------ | ------------------------- |
| `(a: Int) -> (b: Int) -> Int`   | **Currying**             | `f(1)(2)`                 |
| `(a: Int) -> ((b: Int) -> Int)` | **Returning a function** | `g(1)` returns a function |

Rationale: **annotation is the type**. `g: (a: Int) -> ((b: Int) -> Int)` declares that
`g(1) : (b: Int) -> Int`, so `g(1)` must **be** that function, not "the next segment of parameters".

```yaoxiang
// Currying: two parameter segments given layer by layer
add: (a: Int) -> (b: Int) -> Int = { a + b }
add(1)(2)        // → 3

// Returning a function: outer layer takes one parameter, the return value is the function
adder: (n: Int) -> ((x: Int) -> Int) = (x) => x + n
adder(10)(5)     // → 15
h = adder(10)    // The returned function can be stored in a variable and passed
```

Unparenthesized nested `Fn` is always currying (including the type parameter form in RFC-011):

```yaoxiang
identity: (T: Type) -> (x: T) -> T = (x) => x   // Currying
identity(5)      // → 5
```

**Type checking**: The parentheses declare the return type, and the body must produce that type.
`f: () -> (() -> Int) = { 7 }` is an error—the expected type is `() -> Int`, but the actual result
is `Int` (E1002).

### 2.10 Lambda Expression

```
Lambda      ::= '(' ParamList? ')' '=>' Expr
            |  '(' ParamList? ')' '=>' Block
```

### 2.11 Error Propagation Operator

```
ErrorPropagate ::= Expr '?'
```

The `?` operator is a postfix operator with the same precedence as `.`. For `Result(T, E)` types:

- On `Ok(v)`, extract the value `v` and continue execution
- On `Err(e)`, propagate the error upward (`return Err(e)`)

```yaoxiang
process: (data: Data) -> Result(Data, Error) = {
    validated = validate(data)?     // On success, extract the value; on failure, propagate upward
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

// Range is a value: bind, pass, member check
r = 1..10
assert.assert(5 in r, "membership")
for i in r { print(i) }

// step form (third component, default 1)
for i in 0..10..2 { print(i) }  // 0, 2, 4, 6, 8
for i in 10..0..(-2) { print(i) }  // 10, 8, 6, 4, 2
```

> **step semantics**: In `a..b..c`, `c` is the step. A literal `c = 0` is rejected at compile time;
> a dynamic `c` is checked at runtime to be non-zero (E6001 family; will be promoted to Result after
> the error system lands). `c < 0` is legal, and the interval direction reverses with the sign
> (`10..0..(-2)` is descending).

### 2.13 ref Expression

```
RefExpr     ::= 'ref' Expr
```

`ref` creates shared ownership. The compiler automatically chooses Rc (single task) or Arc (cross
task); users do not need to care about implementation details.

```yaoxiang
data = ref heavy_data
spawn { use(data) }   // Cross-task: the compiler automatically chooses Arc
```

### 2.14 unsafe Expression

```
UnsafeExpr  ::= 'unsafe' Block
```

`unsafe` blocks are used to define opaque types and manipulate raw pointers. Use `return` to return
the type definition to the enclosing scope.

**Semantics**:

- Types can be defined and raw pointers manipulated within `unsafe {}`
- The returned type is usable outside `unsafe {}`
- Accessing the type's fields requires unsafe permission

```yaoxiang
// Define an opaque type inside an unsafe block
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  // Raw pointer
    }
    return SqliteDb
}

// SqliteDb is usable outside the unsafe block
db = sqlite3_open("test.db")
```

### 2.15 Scope

**Basic rules**:

- Each `{}` block creates a scope
- Inner scopes can access variables in outer scopes
- Outer scopes cannot access variables in inner scopes
- Variable declarations follow the "assignment-first" principle

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

- `x = value`: look up `x` outward along the scope chain; if found, assign; if not, declare a new
  one
- `mut x = value`: explicit new mutable declaration, forbidden to share the same name as an outer
  layer
- Within the same scope, any name can be declared only once

> **Detailed definition**: The complete rules of scope, variable declaration, and shadowing
> mechanism are detailed in [Module System Specification](./modules.md#chapter-4-scope).

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
(penetrating all blocks—including `if` / `while` / `for` / `match` / bare blocks / `spawn` /
`unsafe`), delivering the value to the caller. It does **not "return to the block"**.

**Type**: `return e : Never` (where `e : T`). `Never <: T'` holds for any `T'` (principle of
explosion, see [Type System §2.2](./type-system.md)), so `return` can appear at any return type
position without additional rules.

**Relationship with block evaluation**: The value of a block is always the **tail expression** (see
§2.9). When `{ return n }` is used as a block, its value is `n` of type `Never`; meanwhile, the
effect of `return` is to exit the function. **Both are true simultaneously**, coexisting via the
principle of explosion.

`return` together with tail expressions make "early return" work, without needing extra rules where
`return` specifically refers to a function—see
[RFC-010a](../../design/rfc/accepted/010a-tail-expression-and-return.md).

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 {
        return 1          // Penetrates if, exits the function (type Never)
    }
    n * factorial(n - 1)  // Tail expression = block value
}
```

### 3.4 break Statement

```
BreakStmt   ::= 'break'
```

**Semantics**: Immediately terminates the innermost `while`/`for` loop, with control flow proceeding
to after the loop body.

- **Exits only the nearest layer**: `break` always acts on the innermost loop that contains it. When
  you need to break out of multiple layers in nested loops, extract the inner loop into a function
  and use `return` to return, or use a flag. (break/continue carry no labels; if loop labels are
  introduced in the future, they will follow the loop declaration-side syntax through the RFC
  process, decided together with the multi-exit design of the proof pipeline.)
- **Only valid within loop bodies**: `break` can only appear within a `while`/`for` loop body
  (including nested blocks/if/match inside the body); appearing outside a loop is a compile error
  (E1102 `'break' outside of a loop`).
- **Does not affect termination proofs**: `break` does not participate in termination
  arguments—neither providing a measure nor constituting a decreasing step; the loop's termination
  obligation is unrelated to `break` and is triggered by refined types (see
  [type-system §8.4](./type-system.md#84-terminates-termination-measure-predicate)).
- **Borrow semantics**: The control flow edge of break participates in the structural cut of
  RFC-009a reverse BFS liveness analysis (the iterations that are jumped out do not participate in
  the back-edge liveness derivation).

```yaoxiang
mut i = 0
while i < 10 {
    i = i + 1
    if i == 3 {
        break              // Control flow proceeds past the loop, i == 3
    }
}

// Nested loops: break only exits the inner one
while j < 3 {
    while k < 10 {
        if k == 2 { break }    // Only terminates the inner loop
    }
    j = j + 1                  // Each iteration of the outer loop reaches here
}
```

### 3.5 continue Statement

```
ContinueStmt::= 'continue'
```

**Semantics**: Skips the remaining statements in this iteration, going directly to the next round of
the innermost loop—`while` returns to the condition re-check, `for` takes the next element.

- **Only acts on the nearest layer**: Same as `break`, no labels
- **Only valid within loop bodies**: Appearing outside a loop is a compile error (E1102)

```yaoxiang
mut sum = 0
mut n = 0
while n < 5 {
    n = n + 1
    if n == 3 {
        continue           // Skips the following accumulation, n == 3 is not counted
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

#### 3.9.1 Semantics: Each Iteration is a New Binding

YaoXiang's for loop semantics differ from traditional languages: **each iteration creates a new
binding, rather than modifying the same variable**.

```yaoxiang
// Example: for i in 1..5
for i in 1..5 {
    print(i)
}
```

**Execution process**:

| Iteration | Behavior of the loop variable                                                           |
| --------- | --------------------------------------------------------------------------------------- |
| 1st       | Create new binding `i = 1`, loop body executes, print 1                                 |
| 2nd       | Create new binding `i = 2` (previous binding is destroyed), loop body executes, print 2 |
| 3rd       | Create new binding `i = 3`, loop body executes, print 3                                 |
| 4th       | Create new binding `i = 4`, loop body executes, print 4                                 |
| End       | Loop body ends, binding is destroyed                                                    |

**Key point**: After each iteration ends, the binding created in that iteration is destroyed. The
next iteration is a completely new binding, unrelated to the binding of the previous iteration.

#### 3.9.2 The Difference Between `for` and `for mut`

| Syntax              | Loop variable mutability | Description                                         |
| ------------------- | ------------------------ | --------------------------------------------------- |
| `for i in 1..5`     | Immutable                | The binding cannot be modified within the loop body |
| `for mut i in 1..5` | Mutable                  | The binding can be modified within the loop body    |

```yaoxiang
// Legal: each iteration binds a new value, no modification needed
for i in 1..5 {
    print(i)  // Read i's value
}

// Error: immutable binding, cannot modify
for i in 1..5 {
    i = i + 1  // Error: cannot modify an immutable binding
}

// Legal: use for mut to allow modification of the binding
for mut i in 1..5 {
    i = i + 1  // Modification allowed
}
```

#### 3.9.3 Shadowing Check

YaoXiang forbids variable shadowing. A for loop variable cannot share the same name as a variable in
an outer scope:

```yaoxiang
// Error: i has already been declared in the outer scope
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

This rule applies to all code blocks, see [4.3 Shadowing Rules](./modules.md#43-shadowing-rules) for
details.

#### 3.9.4 Comparison with Other Languages

| Language | for loop variable semantics                                |
| -------- | ---------------------------------------------------------- |
| YaoXiang | Each iteration creates a new binding                       |
| Rust     | Modify the same variable (requires mut)                    |
| Python   | Modify the same variable (no mut needed)                   |
| C/C++    | Modify the same variable (requires pointers or references) |

**Design rationale**: YaoXiang uses binding semantics because:

1. **More aligned with natural semantics**: In natural language, "for each element x in the
   collection" means each x is an independent individual. YaoXiang's `for i in 1..5` reads as "for
   each i from 1 to 5", and the i in each iteration is a completely new binding, consistent with
   human intuition.

2. **Avoids accidental modification**: The default immutable binding semantics means the loop
   variable cannot be accidentally modified within the loop body. No need to worry about a
   hard-to-trace bug caused by writing `i = ...` somewhere in a complex loop body.

3. **High-performance solutions are within reach**: When it is genuinely necessary to reuse a
   variable across iterations (e.g., accumulators, caches), simply use `for mut` to switch to
   mutable binding mode. This is clearer than implicit shared state—intent is explicitly expressed
   through syntax, not hidden in runtime behavior.

### 3.10 spawn Statement

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' 'mut'? Identifier 'in' Expr '{' Expr '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
```

**spawn block**: Explicitly declares a concurrent region; the expressions inside the block execute
concurrently.

```yaoxiang
(result_a, result_b) = spawn {
    parse(fetch("url1")),
    parse(fetch("url2"))
}
```

**spawn loop**: A data-parallel loop.

```yaoxiang
results = spawn for item in items {
    process(item)
}
```

**spawn block captures outer variables** (RFC-024 §2.3, value capture semantics):

- Outer variables referenced in the block body = **Move value capture**: the value is snapshotted
  into the closure environment at the spawn creation point, and the block body reads through env
  (LoadUpvalue)
- **Primitives** (Int/Float/Bool/Char) are value-copied; outer variables are not affected
- **Handle types** (Struct/String/List, etc.) snapshot = handle copy, sharing the underlying object;
  the Embedded runtime (default) uses the same thread and heap, and the handle is valid
- Sharing among multiple tasks requires explicit `ref` (§2.13, the compiler automatically chooses
  Rc/Arc)
- `return` inside the block referencing outer variables is also captured

```yaoxiang
t1 = 1 + 1
t2 = 2 + 2
result = spawn {
    return t1 + t2    // t1/t2 are value-captured, result == 6
}
```

---

### 3.11 Program Entry and Top-level Statements

A source file has two roles in the compiler's eyes, **determined by whether `yaoxiang.toml`
exists**:

| Role       | Determination                                | Program body                                                                        |
| ---------- | -------------------------------------------- | ----------------------------------------------------------------------------------- |
| **Script** | Single file run directly, no `yaoxiang.toml` | **Top-level statements** (executed in writing order); `main` is an ordinary binding |
| **Bin**    | `yaoxiang.toml` exists                       | **`main` function**; top-level must not contain executable statements               |

#### Script: Top-level Statements are the Program

Without a manifest, the file is a "script" and **top-level statements execute in writing order**:

```yaoxiang
use std.io
io.println("hello")          // Execute directly
x: Int = { 42 }              // Top-level binding: runtime initialization
io.println(x)                // 42
```

In this mode, `main` is **not special**—it is just an ordinary binding. To make `main` run, you must
call it explicitly:

```yaoxiang
use std.io
main: () -> Void = { io.println("only runs if called") }

main()                       // ← This line is required
```

> **Why not auto-call `main`?** Top-level statements are already the program body. If `main` were
> implicitly called as well, a script that explicitly wrote `main()` would execute twice. The two
> rules cannot coexist, so under Script there is only one execution entry: "top-level statements".

#### Bin: `main` is the Entry

With a manifest, the file is an "executable target". In this case:

- `main` must be defined, and it must be a function (its signature must support zero-argument calls)
- Executable statements are not allowed at the top level—the program body is `main` itself

```yaoxiang
main: () -> Void = {
    print("hello")
}
```

Missing `main` or a non-function `main` is a compile error (the former: no `main` means all
functions are unreachable; the latter: a value binding will not be called).

> **Library files**: Files `use`d by other files, or files pointed to by `[lib].path` / `[exports]`,
> do not require `main`—they are not program entry points.

#### Initialization of Top-level Bindings

The initialization value of top-level bindings is **evaluated at runtime**; it is not required to be
a compile-time constant:

```yaoxiang
answer: Int = { 42 }              // Block value
inc: (Int) -> Int = (x) => x + 1
computed: Int = inc(41)           // Function call
```

Initialization executes in **dependency order**, regardless of writing order:

```yaoxiang
derived: Int = base * 3           // References base declared later
base: Int = 7                     // Initialized first (topological sort)
```

Cyclic dependencies are a compile error (the names on the cycle will be listed):

```yaoxiang
a: Int = b + 1
b: Int = a + 1                    // Error: a → b → a
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
break | continue          // Only within loop bodies (§3.4 / §3.5)
```

### A.2 Error Handling

```
Expr '?'              // Error propagation (Result type)
```

### A.3 match Syntax

```
match value {
    pattern1 => expr1,
    pattern2 if guard => expr2,
    _ => default_expr,
}
```
