# Syntax Specification

This file defines the syntax specification of the YaoXiang programming language, including lexical
structure, grammar rules, and operator precedence.

---

## Chapter 1: Lexical Structure

### 1.1 Source Files

YaoXiang source files must be encoded in UTF-8. Source files typically use the `.yx` extension.

### 1.2 Lexical Token Categories

| Category   | Description                        | Example                   |
| ---------- | ---------------------------------- | ------------------------- |
| Identifier | Starts with a letter or underscore | `x`, `_private`, `my_var` |
| Keyword    | Language predefined reserved words | `use`, `mut`, `and`       |
| Literal    | Fixed values                       | `42`, `"hello"`, `true`   |
| Operator   | Operation symbols                  | `+`, `-`, `*`, `/`        |
| Delimiter  | Syntax separators                  | `(`, `)`, `{`, `}`, `,`   |

### 1.3 Keywords

YaoXiang has **18 keywords** (the `keyword_from_str` in `src/frontend/core/lexer/state.rs:25-55`,
each corresponding to a `TokenKind`):

```
pub     use     spawn  ref     mut
if      else    match  while   for
in      return  break  continue
as      unsafe  and    or
```

`and` / `or` are **keywords** for logical AND / OR (Zig-style; see precedence in §2.2 level 10);
unary NOT is the symbol `!`, orthogonal to them.

These keywords have special meaning in any context and cannot be used as identifiers.

> **`type` is no longer a keyword** (RFC-010): write type definitions using the
> `Name: Type = { ... }` notation. `src/frontend/core/lexer/state.rs:27` explicitly comments on
> this, and the `TokenKind` enum header (`src/frontend/core/lexer/tokens.rs:82`) also states "16
> total - RFC-010: 'type' keyword removed" — **that "16" is an outdated comment**, and 18 are
> actually listed (including `and` / `or`, while the `Kw*`-prefixed batch remains 16).
>
> **`pub` does not produce visibility effects**: `pub` is still lexed as `KwPub`
> (`src/frontend/core/lexer/state.rs:28`), and the parser skips it at declarations and imports
> (`src/frontend/core/parser/statements/declarations.rs:666-671,726`, `.../imports.rs:57-59`), but
> the **module system does not perform any visibility judgment based on it** — the accepted
> [RFC-029](../../design/rfc/accepted/029-module-semantics.md) explicitly states "no `pub`, no
> `private`, no `export`, no visibility mechanism" (line 17 of that file). Whether you write `pub`
> or not makes no difference to visibility.

### 1.4 Reserved Words

YaoXiang's "reserved words" are split into three layers, recognized at different stages by the
parser and type checker:

#### 1.4.1 Literal Reserved Words

The parser has independent tokens for these literal identifiers and cannot use them as ordinary
identifiers:

| Identifier | Belongs to Type | Description                                                                                                   |
| ---------- | --------------- | ------------------------------------------------------------------------------------------------------------- |
| `true`     | Bool            | Boolean truth value                                                                                           |
| `false`    | Bool            | Boolean false value                                                                                           |
| `void`     | Void            | Void literal (Unit value). Lowercase `void` is a value literal; uppercase `Void` is a type name (see §1.4.3). |

> **`Type` is not in this layer**: it has **no** independent `TokenKind` (it is not in the
> `TokenKind` enum in `src/frontend/core/lexer/tokens.rs`, and there is no corresponding branch in
> `keyword_from_str`). The parser treats it as an ordinary identifier, and the type checker
> recognizes it as a meta type in type positions. Therefore `Type` can be shadowed by a local
> binding in expression positions. It is a **meta type name**, not a keyword.

#### 1.4.2 Variant Names

Variant names are recognized by the parser in **pattern** contexts (bare names are adjudicated by
the scrutinee type's variant set, see §2.8); in **expression** contexts, constructing variants
requires type qualification — `Result(Int, String).ok(5)`, `Option(T).some(v)`, `Color.green()`
(RFC-010 record types: variant constructors are function-type fields in type definitions; a bare
name `ok(5)` is not a constructor call).

| Variant Name | Belongs to Type | Description                      |
| ------------ | --------------- | -------------------------------- |
| `some(T)`    | Option          | Option value variant constructor |
| `ok(T)`      | Result          | Result success variant           |
| `err(E)`     | Result          | Result error variant             |

#### 1.4.3 Built-in Type Names

The following type names are pre-registered by the type checker and can be used in type positions
without imports. The parser treats them as ordinary identifiers — **not reserved words, and can be
shadowed by local bindings (not recommended)**.

| Type Name | Logical Correspondence | Description                                                                                                                                        |
| --------- | ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Void`    | ⊤ (true / Unit)        | A zero-field product type with exactly one inhabitant (the `void` literal, see §1.4.1)                                                             |
| `Never`   | ⊥ (false / empty type) | A zero-variant sum type with zero inhabitants. No expression can produce a `Never` value. `Never <: T` holds for all `T` (principle of explosion). |
| `Int`     | —                      | Signed integer                                                                                                                                     |
| `Float`   | —                      | Float                                                                                                                                              |
| `Bool`    | —                      | Boolean value: `true` / `false`                                                                                                                    |
| `Char`    | —                      | Unicode character                                                                                                                                  |
| `String`  | —                      | String                                                                                                                                             |

### 1.5 Identifiers

Identifiers start with a letter or underscore, and subsequent characters can be letters, digits, or
underscores. Identifiers are case-sensitive.

Special identifiers:

- `_` is used as a placeholder, indicating that a value is ignored
- Identifiers starting with an underscore represent private members

### 1.6 Literals

#### 1.6.1 Integer

```
Decimal     ::= [0-9][0-9_]*
Octal       ::= 0o[0-7][0-7_]*
Hex         ::= 0x[0-9a-fA-F][0-9a-fA-F_]*
Binary      ::= 0b[01][01_]*
```

#### 1.6.2 Float

```
Float       ::= [0-9][0-9_]* '.' [0-9][0-9_]* ([eE][+-]?[0-9][0-9_]*)?
```

#### 1.6.3 String

```
String      ::= '"' ([^"\\] | EscapeSequence)* '"'
Escape      ::= '\\' ([nrt'"\\] | UnicodeEscape)
Unicode     ::= 'u' '{' HexDigit+ '}'
```

#### 1.6.4 Collections

```
List        ::= '[' Expr (',' Expr)* ']'
Dict        ::= '{' String ':' Expr (',' String ':' Expr)* '}'
Array       ::= '[' Expr (',' Expr)* ']'   // when the target type annotation is Array(T, N), the literal lands as a fixed-length array
```

> **Dict literals require at least one key-value pair**: `{}` is **not** an empty dict — it is an
> empty block (value `Void`, see [§2.9](#_2-9-block-expressions)). For an empty dict, use the
> constructor `dict.new()`:
>
> ```yaoxiang
> empty = dict.new()                  // ✅ empty dict
> d = { "a": 1 }                       // ✅ dict literal
> wrong = {}                           // ❌ this is not a dict, it's an empty block (Void)
> ```
>
> **The criterion is content**: the `Dict` grammar requires at least one `String ':' Expr`, and `{}`
> has no content to rely on, so it takes the zero form of the block structure. Non-empty forms are
> self-describing by content (`{ "k": v }` has key-value pairs → dict) — this is the same source as
> `f = { 5 }` being an `Int` value rather than a function: **the type is determined by content**.

> Set has no literal grammar and no runtime representation — set types are in planning and will be
> filled in following the Dict pattern when the need arises (std.set + HeapValue::Set). The landing
> of List/Dict literals is determined by the context type annotation: a bare literal and a `List(T)`
> annotation land as a growable list; an `Array(T, N)` annotation acting directly on a literal lands
> as a fixed-length array. Implicit List→Array conversion is prohibited.
>
> Array literal semantics:
>
> - The number of elements must equal N; otherwise E1002 at compile-time; an empty literal paired
>   with a non-zero N is likewise rejected
> - Each element's type must be compatible with T; otherwise E1002 at compile-time
> - N's grammar form: only integer literals (possibly negative) or constant names; compound
>   expressions (e.g., `2+1`) are rejected at parse time
> - When N is a symbolic constant (function const parameter, e.g., `Array(Int, n)`), the count check
>   is deferred to the refined type stage
> - Nested array literals in v1 (e.g., `Array(Array(Int,2),2) = [[1,2],[3,4]]`) are rejected at
>   compile-time and must be constructed layer by layer; recursive landing is reserved for a later
>   version

#### 1.6.5 List Comprehension

```
ListComp    ::= '[' Expr 'for' Identifier 'in' Expr (',' Expr)* ('if' Expr)? ']'
```

> **Behavior tightening (migration record)**: the iteration variable grammar is already
> `'for' Identifier 'in'`, but the old implementation parsed pattern via full pratt — after `in` was
> registered as an infix operator, `x` would swallow `in items` into a membership expression. After
> the fix, a non-identifier pattern directly fails to parse and no longer falls back to `_` like the
> old implementation (silently swallowing errors). Impact: previously parseable forms like
> `[x for (a, b) in pairs]` now error — this form never had defined behavior (the variable was
> always `_`), the tightening direction is correct, and there is no semantic migration cost.

#### 1.6.6 Membership Test

```
Membership  ::= Expr 'in' Expr
```

> `in` is a binary relational operator that returns `Bool` — hit is `true`, miss is `false`, no
> error. Semantic split: `[]` asserts existence and takes a value (fails with error), while `in`
> asks whether something exists (a miss is a normal `false`). Right operand coverage: List / Array /
> Dict (key set) / Tuple / String (substring) / Range (interval). `in` is a first-class Hoare
> predicate, and at the refined type stage it serves as the basis for compile-time provable
> propositions. (Set is removed from the right operand list — Set has no runtime representation, see
> §1.6.4)

### 1.7 Comments

```
// Single-line comment

/* Multi-line comment
   can span multiple lines */
```

### 1.8 Indentation Rules

Code must use 4 spaces for indentation; Tab characters are prohibited. This is a mandatory syntax
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

> **Unary prefix operators** (`!` `-` `+`) bind tightly: only below call and member access, above
> all binary operators. Therefore `!a == b` ≡ `(!a) == b` (Zig-style semantics); `!` is a pure unary
> operation, does not participate in short-circuit control flow, and is orthogonal to the `and` /
> `or` keywords (short-circuit) (authoritative definition in RFC-010).
>
> **Range binding strength**: `..` has binding strength (6, 7) — left 6 is below addition (7), right
> 7 swallows addition but not the same-level `..`. Before/after comparison:
>
> | Expression   | Before change (level 1, right-assoc)                                                            | After change ((6,7), left-assoc)                                           |
> | ------------ | ----------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------- |
> | `x in 1..10` | `x in 1..10` (`in` right operand level 4, `..` level 1 cannot swallow, unparseable in practice) | `x in (1..10)` — the interval as a whole serves as the `in` right operand  |
> | `0..n+2`     | `(0..n)+2` (right-assoc trap: upper bound swallowed, `for` loop directly E3004)                 | `0..(n+2)` — upper bound is an arithmetic expression                       |
> | `a == b..c`  | `a == (b..c)` (`..` level 1 < `==` level 3, naturally the whole)                                | `a == (b..c)` — **semantics unchanged**, `..` still above comparison level |
> | `1..2*3`     | `(1..2)*3`                                                                                      | `1..(2*3)` — upper bound is an arithmetic expression                       |
> | `a..b..c`    | `a..(b..c)` (right-assoc chained, meaningless Range inside Range)                               | `(a..b)..c` — **step form** (`c` is the step)                              |
>
> Net effect: composite upper bound `for i in 0..n+2` goes from "parses successfully but E3004" to
> "directly usable"; `x in 1..10` goes from "unparseable" to "interval check"; `a..b..c` goes from
> "meaningless nesting" to "step component". Level 6 falls between `+` (level 5) and `<<` (level 7),
> a mathematical convention: intervals are tightly binding constructs, and the upper bound is
> naturally a full arithmetic expression.

### 2.3 Function Calls

```
FnCall      ::= Expr '(' ArgList? ')'
ArgList     ::= Expr (',' Expr)* (',' NamedArg)* | NamedArg (',' NamedArg)*
NamedArg    ::= Identifier '=' Expr
```

Named arguments use `name = value` (RFC-010 §Function Definitions, RFC-011 §Construction Forms).
Positional arguments must come before named arguments; order-specified parameters can be arranged in
any order among named arguments:

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b

add(3, 5)          // positional
add(a = 3, b = 5)  // named
add(b = 5, a = 3)  // arbitrary order
add(3, b = 5)      // mixed, positional first
```

A wrong name in a named argument reports **E1014**; the same parameter being specified both
positionally and by name reports **E1015**; a count mismatch reports **E1010** (RFC-013).

### 2.4 Member Access

```
MemberAccess::= Expr '.' Identifier
```

### 2.5 Index Access

```
IndexAccess ::= Expr '[' Expr ']'
```

> **Three-layer semantics (RFC-011b)**: `a[i]` dispatches by `a`'s type — ① built-in containers
> (List/Vec/Array/Dict/Tuple) go through the native index instruction (fast path); ② user types
> implementing the `Index` interface dispatch to their `index` method (an `Index(Grid, Int, Float)`
> instantiation inside the type body + the `Grid.index` method); ③ RFC-004's `f[0]` positional
> binding only exists in binding declarations, not via this grammar. Multi-dimensional index
> `a[0, 1]` packs the keys as a tuple. Other types that have not implemented `Index` are rejected at
> the type level.

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

> **Variant destructuring's variant set requirement** (RFC-010b): `EnumPattern` (variant names such
> as `ok(v)`, `some(x)` adjudicated by the scrutinee) requires the scrutinee's type to be a **sum
> type whose variant set is present**. The variant set only enters the checker through type
> definitions or `use` imports — `Result` / `Option` are defined by `std.result` / `std.option`, and
> `use std.result` / `use std.option` must appear before use (whole-module and grouped
> `use std.{...}` forms have equal weight); otherwise variant destructuring reports E1002.
> Exhaustiveness adjudication is the same source: a catch-all arm is exempt from exhaustiveness;
> without a catch-all arm, the full variant set is checked.

### 2.9 Block Expression

```
Block       ::= '{' Stmt* Expr? '}'
```

> **Statement termination rules**: the separation between Stmts and newline behavior (explicit `;`
> separation, newline termination, line-continuation exceptions, leading `(` / `[` never merging)
> are defined by [RFC-038](../../design/rfc/accepted/038-statement-termination.md).

#### 2.9.1 Three Forms of `{`

`{` has exactly three interpretations in expression position, determined in one pass by **content**:

| Form             | Notation          | Type                 | Example          |
| ---------------- | ----------------- | -------------------- | ---------------- |
| **Empty block**  | `{}`              | `Void`               | `x: Void = {}`   |
| **Dict literal** | `{ "k": v, ... }` | `Dict(K, V)`         | `d = { "a": 1 }` |
| **Block**        | `{ Stmt* Expr? }` | Tail expression type | `y = { 1 + 1 }`  |

**Adjudication order**:

1. `{}` (no content) → **empty block**, value `Void`
2. The first element is a `String ':' Expr` key-value pair → **dict literal**
3. Otherwise → **block**, value given by the tail expression

> **Why `{}` is not an empty dict**: the "empty" of an empty dict cannot be self-describing (it can
> be either `Dict(K, V)` or an empty block), and the dict grammar [§1.6.4](#_1-6-4-collections)
> requires at least one key-value pair. When there is no content to rely on, the zero form of the
> block structure is taken: this is consistent with `unsafe {}` / `spawn {}`, with no special cases
> introduced. Use `dict.new()` for an empty dict.
>
> **Why functions need annotations**: `f = { stmt }` is a **value** (the tail expression's type),
> not a function. To define a function, write the Fn annotation explicitly: `f: () -> Int = { 5 }`.
> This is the same principle as dicts: **the type is determined by content**, not by the presence or
> absence of an annotation. (This rule is in
> [RFC-010a](../../design/rfc/accepted/010a-tail-expression-and-return.md) Appendix D.)

**Unified semantics**: the value of all `{}` blocks is given by the **tail expression**; `return` is
a non-local exit of type `Never`.

| Block Type   | Value Outlet    | Empty Block `{}` |
| ------------ | --------------- | ---------------- |
| Regular `{}` | Tail expression | `Void`           |
| `unsafe {}`  | Tail expression | `Void`           |
| `spawn {}`   | Tail expression | `Void`           |

**Core principles** (see [RFC-010a](../../design/rfc/accepted/010a-tail-expression-and-return.md)
for details):

- **The block's value = the tail expression** (the last expression), the only outlet, with no
  exceptions
- When the last position is an **assignment statement**, the block's value is `Void`; if you want
  `Void`, write `Void` explicitly
- **`return` exits the nearest function boundary** (pierces through all blocks — it does not "return
  to the block"), of type `Never`; `Never <: T` holds for any type (principle of explosion), so it
  can appear at any return type position
- The expression form `= expr` directly gives the value

```yaoxiang
// Regular {} block: the tail expression gives the value
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

// return: pierces through the block and exits the function
f: (n: Int) -> Int = {
    if n < 0 {
        return 0     // pierces out of if and the function body, exits the function
    }
    n * 2            // tail expression
}
```

#### Is `name = { ... }` a Function or a Block Value?

`name = { ... }` may be either a function definition (RFC-007 "zero-argument simplest form") or a
block-value binding. Adjudicated by **annotation priority, function by default** (RFC-010a Appendix
D):

| Case                              | Result          | Example                            |
| --------------------------------- | --------------- | ---------------------------------- |
| Value is a Lambda (`=>`)          | Function        | `f = () => 5` → `f()` = 5          |
| Annotation is a function type     | Function        | `f: () -> Int = { 5 }` → `f()` = 5 |
| Annotation is a non-function type | **Block value** | `x: Int = { 5 }` → `x` = 5         |
| No annotation                     | Function        | `f = { 5 }` → `f()` = 5            |

**Annotation is the type**: `x: Int = ...` declares `x` to be `Int`, so `{ ... }` evaluates to
`Int`; `f: () -> Int = ...` declares `f` to be a function, so `{ ... }` is the function body.

To make `{ ... }` evaluate on the spot, **just write the target type** (no new syntax needed):

```yaoxiang
// Block value: evaluated immediately
x: Int = {
    y = 5
    y            // x = 5
}

// Function: no annotation, default
f = { 5 }        // f() = 5
```

#### Nested Function Types: Currying or Returning a Function?

A nested function type on the right side of `->` has two readings, distinguished by **parentheses**
(RFC-004):

| Notation                        | Meaning                  | Call                    |
| ------------------------------- | ------------------------ | ----------------------- |
| `(a: Int) -> (b: Int) -> Int`   | **Currying**             | `f(1)(2)`               |
| `(a: Int) -> ((b: Int) -> Int)` | **Returning a function** | `g(1)` gives a function |

Basis: **annotation is the type**. `g: (a: Int) -> ((b: Int) -> Int)` declares
`g(1) : (b: Int) -> Int`, so `g(1)` must **be** that function, not the "next segment of parameters".

```yaoxiang
// Currying: two parameter segments given layer by layer
add: (a: Int) -> (b: Int) -> Int = { a + b }
add(1)(2)        // → 3

// Returning a function: outer one parameter segment, the returned value is the function
adder: (n: Int) -> ((x: Int) -> Int) = (x) => x + n
adder(10)(5)     // → 15
h = adder(10)    // the returned function can be stored in a variable, passed around
```

Nested `Fn` without parentheses is always currying (including the type-parameter form from RFC-011):

```yaoxiang
identity: (T: Type) -> (x: T) -> T = (x) => x   // currying
identity(5)      // → 5
```

**Type check**: the parentheses declare the return type, and the body must produce that type.
`f: () -> (() -> Int) = { 7 }` is an error — `() -> Int` is expected, but `Int` is given (E1002).

### 2.10 Lambda Expression

```
Lambda      ::= '(' ParamList? ')' '=>' Expr
            |  '(' ParamList? ')' '=>' Block
```

### 2.11 Error Propagation Operator

```
ErrorPropagate ::= Expr '?'
```

The `?` operator is a postfix operator with the same precedence as `.`. For a `Result(T, E)` type:

- On `Ok(v)`, extract the value `v` and continue execution
- On `Err(e)`, propagate the error upward (`return Err(e)`)

```yaoxiang
process: (data: Data) -> Result(Data, Error) = {
    validated = validate(data)?     // on success, extract the value; on failure, propagate upward
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

> **Step semantics**: in `a..b..c`, `c` is the step. The literal `c = 0` is rejected at
> compile-time; a dynamic `c` is zero-checked at runtime (E6001 family; after the error system
> lands, it is upgraded to Result). `c < 0` is legal, and the interval direction reverses with the
> sign (`10..0..(-2)` is decreasing).

### 2.13 ref Expression

```
RefExpr     ::= 'ref' Expr
```

`ref` creates shared ownership. The compiler automatically chooses Rc (single-task) or Arc
(cross-task), and the user does not need to care about the implementation details.

```yaoxiang
data = ref heavy_data
spawn { use(data) }   // cross-task: the compiler automatically chooses Arc
```

### 2.14 unsafe Expression

```
UnsafeExpr  ::= 'unsafe' Block
```

The `unsafe` block is used to define opaque types and operate on raw pointers. Use `return` to
return the type definition to the outer scope.

**Semantics**:

- Inside `unsafe {}` you can define types and operate on raw pointers
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

- Each `{}` block creates a scope
- An inner scope can access variables in an outer scope
- An outer scope cannot access variables in an inner scope
- Variable declarations follow the "assignment first" principle

```yaoxiang
// Block scope
{
    x = 10
    // x is visible inside this scope
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

- `x = value`: search for `x` outward along the scope chain; if found, assign; if not found, declare
  a new one
- `mut x = value`: explicit new mutable declaration, disallows the same name as the outer scope
- Any name can only be declared once in the same scope

> **Detailed definition**: the complete rules of scopes, variable declaration, and shadowing
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

**Semantics**: `return` is a **non-local exit**, exiting the nearest **function boundary** (pierces
through all blocks — including `if` / `while` / `for` / `match` / bare blocks / `spawn` / `unsafe`),
handing the value to the caller. It does **not "return to the block"**.

**Type**: `return e : Never` (`e : T`). `Never <: T'` holds for any `T'` (principle of explosion,
see [Type System §2.2](./type-system.md)), so `return` can appear at any return type position with
no extra rules.

**Relationship with block evaluation**: a block's value is always the **tail expression** (see
§2.9). `{ return n }` as a block has the value `n`, of type `Never`; at the same time, the effect of
`return` is to exit the function. **Both hold simultaneously**, coexisting via the principle of
explosion.

`return` together with the tail expression makes "early return" possible, with no extra rule that
`return` specifically targets a function — see
[RFC-010a](../../design/rfc/accepted/010a-tail-expression-and-return.md).

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 {
        return 1          // pierces out of if and exits the function (type Never)
    }
    n * factorial(n - 1)  // tail expression = block's value
}
```

### 3.4 break Statement

```
BreakStmt   ::= 'break'
```

**Semantics**: immediately terminates the innermost `while` / `for` loop containing it; control flow
moves to after the loop body.

- **Only exits the nearest layer**: `break` always acts on the innermost loop containing it. In
  nested loops, when you need to jump out of multiple layers in one go, extract the inner loop into
  a function and use `return` to return, or use a flag (break / continue do not carry labels; if
  loop labels are introduced in the future, they will follow the loop declaration-side syntax
  through the RFC process, adjudicated together with the proof pipeline's multi-exit design)
- **Restricted to the loop body**: `break` can only appear inside a `while` / `for` loop body
  (including blocks / if / match nested within the body); appearing outside a loop is a compile-time
  error (E1102 `'break' outside of a loop`)
- **Does not affect termination proofs**: `break` does not participate in termination arguments — it
  neither provides a measure nor constitutes a decreasing step of a measure; the loop's termination
  obligation has nothing to do with `break`, and is triggered by refined types (see
  [type-system §8.4](./type-system.md#84-terminates-termination-measure-predicate))
- **Borrowing semantics**: the control-flow edge of `break` participates in the structural cut of
  the RFC-009a reverse BFS liveness analysis (skipped iterations do not participate in back-edge
  liveness derivation)

```yaoxiang
mut i = 0
while i < 10 {
    i = i + 1
    if i == 3 {
        break              // control flow moves to after the loop, i == 3
    }
}

// Nested loop: break only exits the inner one
while j < 3 {
    while k < 10 {
        if k == 2 { break }    // only terminates the inner loop
    }
    j = j + 1                  // every outer iteration reaches here
}
```

### 3.5 continue Statement

```
ContinueStmt::= 'continue'
```

**Semantics**: skips the remaining statements in the current iteration and directly enters the next
round of the innermost loop containing it — `while` goes back to re-evaluating the condition, `for`
takes the next element.

- **Only acts on the nearest layer**: same as `break`, carries no label
- **Restricted to the loop body**: appearing outside a loop is a compile-time error (E1102)

```yaoxiang
mut sum = 0
mut n = 0
while n < 5 {
    n = n + 1
    if n == 3 {
        continue           // skip the accumulation below, n == 3 is not counted
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

#### 3.9.1 Semantics: Each Iteration Binds a New Value

YaoXiang's `for` loop semantics differ from traditional languages: **each iteration binds a new
value, rather than modifying the same variable**.

```yaoxiang
// Example: for i in 1..5
for i in 1..5 {
    print(i)
}
```

**Execution process**:

| Iteration | Behavior of the loop variable                                                                   |
| --------- | ----------------------------------------------------------------------------------------------- |
| 1st       | Create a new binding `i = 1`, the loop body executes, prints 1                                  |
| 2nd       | Create a new binding `i = 2` (the previous binding has been destroyed), body executes, prints 2 |
| 3rd       | Create a new binding `i = 3`, the loop body executes, prints 3                                  |
| 4th       | Create a new binding `i = 4`, the loop body executes, prints 4                                  |
| End       | Loop body ends, bindings are destroyed                                                          |

**Key point**: after each iteration ends, the binding created in that iteration is destroyed. The
next iteration is an entirely new binding, with no relation to the previous iteration's binding.

#### 3.9.2 The Difference Between `for` and `for mut`

| Syntax              | Loop variable mutability | Description                                     |
| ------------------- | ------------------------ | ----------------------------------------------- |
| `for i in 1..5`     | Immutable                | The binding cannot be modified in the loop body |
| `for mut i in 1..5` | Mutable                  | The binding can be modified in the loop body    |

```yaoxiang
// Legal: each iteration binds a new value; no modification needed
for i in 1..5 {
    print(i)  // read the value of i
}

// Error: immutable binding, cannot be modified
for i in 1..5 {
    i = i + 1  // error: cannot modify an immutable binding
}

// Legal: use `for mut` to allow modifying the binding
for mut i in 1..5 {
    i = i + 1  // modification allowed
}
```

#### 3.9.3 Shadowing Check

YaoXiang prohibits variable shadowing. The `for` loop variable cannot have the same name as a
variable in the outer scope:

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

This rule applies to all code blocks; see [4.3 Shadowing Rules](./modules.md#43-shadowing-rules).

#### 3.9.4 Comparison with Other Languages

| Language | for loop variable semantics                             |
| -------- | ------------------------------------------------------- |
| YaoXiang | Each iteration binds a new value                        |
| Rust     | Modifies the same variable (needs `mut`)                |
| Python   | Modifies the same variable (no `mut` needed)            |
| C/C++    | Modifies the same variable (needs pointer or reference) |

**Design rationale**: YaoXiang adopts binding semantics because:

1. **Closer to natural semantics** In natural language, "for each element x in the collection" means
   each x is an independent individual. YaoXiang's `for i in 1..5` reads as "for each i in 1 through
   5", and each iteration's i is an entirely new binding, consistent with human intuition.

2. **Avoids accidental modification** The default-immutable binding semantics mean the loop variable
   cannot be accidentally modified inside the loop body. No need to worry about some complex part of
   the loop body accidentally writing `i = ...` causing a hard-to-trace bug.

3. **High-performance solutions within reach** When you really need to reuse a variable across
   iterations (e.g., accumulators, caches), use `for mut` to switch to mutable binding mode. This is
   clearer than implicit shared state — intent is expressed explicitly through syntax, not hidden in
   runtime behavior.

### 3.10 spawn Statement

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' 'mut'? Identifier 'in' Expr '{' Expr '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
```

**spawn block**: explicitly declares a concurrent region; the expressions inside the block run
concurrently.

```yaoxiang
(result_a, result_b) = spawn {
    parse(fetch("url1")),
    parse(fetch("url2"))
}
```

**spawn loop**: data-parallel loop.

```yaoxiang
results = spawn for item in items {
    process(item)
}
```

**spawn block captures outer variables** (RFC-024 §2.3, value-capture semantics):

- When the block body references an outer variable = **Move value capture**: the value is
  snapshotted into the closure environment at the spawn creation point, and the block body reads it
  via env (LoadUpvalue)
- **Primitives** (Int / Float / Bool / Char) are value-copied; the outer variable is unaffected
- **Handle types** (Struct / String / List, etc.) snapshot = handle copy, sharing the underlying
  object; the Embedded runtime (default) uses a single thread and single heap, so the handle is
  valid
- Sharing across multiple tasks requires explicit `ref` (§2.13, compiler automatically chooses Rc /
  Arc)
- The outer variable referenced by `return` inside the block is captured the same way

```yaoxiang
t1 = 1 + 1
t2 = 2 + 2
result = spawn {
    return t1 + t2    // t1 / t2 are value-captured, result == 6
}
```

---

### 3.11 Program Entry and Top-level Statements

A source file has two roles in the compiler's eyes, **determined by whether a `yaoxiang.toml`
exists**:

| Role       | Adjudication                                 | Program Body                                                                    |
| ---------- | -------------------------------------------- | ------------------------------------------------------------------------------- |
| **Script** | Single file run directly, no `yaoxiang.toml` | **Top-level statements** (executed in source order); `main` is a normal binding |
| **Bin**    | A `yaoxiang.toml` exists                     | **`main` function**; top-level may not have executable statements               |

#### Script: Top-level Statements are the Program

Without a manifest, the file is a "script", and **top-level statements execute in source order**:

```yaoxiang
use std.io
io.println("hello")          // executed directly
x: Int = { 42 }              // top-level binding: runtime initialization
io.println(x)                // 42
```

In this mode, `main` is **not special** — it is just a normal binding. To make `main` run, you must
call it explicitly:

```yaoxiang
use std.io
main: () -> Void = { io.println("only runs if called") }

main()                       // ← this line must be written
```

> **Why not auto-call `main`?** The top-level statements are already the program body. If `main`
> were also implicitly called, a script with an explicit `main()` would execute twice. The two rules
> cannot coexist, so in Script mode there is only one execution entry: "top-level statements".

#### Bin: `main` is the Entry

When a manifest exists, the file is an "executable target", and at this point:

- A binding named `main` must be defined; a value or function are both acceptable (#388
  finalization: the entry is adjudicated by the existence of the binding — a function is already a
  value, the two differ only in evaluation strategy): a function `main` is zero-arg called at the
  entry stage; a value `main` is evaluated at the initialization stage, which is itself execution
- Executable statements are not allowed at the top level — the program body is `main`

```yaoxiang
main: () -> Void = {
    print("hello")
}
```

A value `main` is also legal — evaluation at the initialization stage is itself execution, and
evaluation happens in the initialization sequence (in topological order of global bindings;
independent bindings are in source order):

```yaoxiang
main = {
    print("hello")
}
```

A missing `main` is a compile error (when there is no `main`, all functions are unreachable). A
non-callable value `main` (e.g., `main: Int = 5`) is legal but has no observable effect — analogous
to Rust's empty `fn main() {}`.

> **Library files**: files used by other files via `use`, or files pointed to by `[lib].path` /
> `[exports]`, do not require `main` — they are not program entries.

#### Initialization of Top-level Bindings

The initialization value of a top-level binding **is evaluated at runtime**; it is not required to
be a compile-time constant:

```yaoxiang
answer: Int = { 42 }              // block value
inc: (Int) -> Int = (x) => x + 1
computed: Int = inc(41)           // function call
```

Initialization executes in **dependency order**, regardless of source order:

```yaoxiang
derived: Int = base * 3           // references base declared later
base: Int = 7                     // initialized first (topological sort)
```

Cyclic dependencies are a compile error (the names in the cycle are listed):

```yaoxiang
a: Int = b + 1
b: Int = a + 1                    // error: a → b → a
```

> **Design basis**: RFC-029f (file role model), RFC-010a Appendix D (block binding adjudication).

---

## Appendix: Syntax Quick Reference

### A.0 Keywords (18)

```
pub     use     spawn  ref     mut
if      else    match  while   for
in      return  break  continue
as      unsafe  and    or
```

`type` is no longer a keyword (RFC-010; use `Name: Type = { ... }` instead); `pub` produces no
visibility effect (RFC-029). Literal reserved words `true` / `false` / `void` are in §1.4.1, meta
type name `Type` and built-in type names `Void` / `Never` / `Int` / `Float` / `Bool` / `Char` /
`String` are in §1.4.3.

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
