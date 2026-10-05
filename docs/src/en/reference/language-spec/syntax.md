# Syntax Specification

This document defines the syntax specification of the YaoXiang programming language, including
lexical structure, grammar rules, and operator precedence.

---

## Chapter 1: Lexical Structure

### 1.1 Source Files

YaoXiang source files must use UTF-8 encoding. Source files typically use the `.yx` extension.

### 1.2 Lexical Token Categories

| Category   | Description                      | Example                   |
| ---------- | -------------------------------- | ------------------------- |
| Identifier | Starts with letter or underscore | `x`, `_private`, `my_var` |
| Keyword    | Language-defined reserved word   | `use`, `mut`, `and`       |
| Literal    | Fixed value                      | `42`, `"hello"`, `true`   |
| Operator   | Operator symbol                  | `+`, `-`, `*`, `/`        |
| Separator  | Syntax separator                 | `(`, `)`, `{`, `}`, `,`   |

### 1.3 Keywords

YaoXiang has **18 keywords** (`keyword_from_str` in `src/frontend/core/lexer/state.rs:25-55`, each
corresponding to one `TokenKind`):

```
pub     use     spawn  ref     mut
if      else    match  while   for
in      return  break  continue
as      unsafe  and    or
```

`and` / `or` are **keywords** for logical AND / OR (Zig-style; see §2.2 level 10 for precedence);
the unary NOT is the symbol `!`, orthogonal to them.

These keywords have special meaning in any context and cannot be used as identifiers.

> **`type` is no longer a keyword** (RFC-010): use the `Name: Type = { ... }` notation to write type
> definitions. `src/frontend/core/lexer/state.rs:27` explicitly notes this, and the `TokenKind` enum
> header (`src/frontend/core/lexer/tokens.rs:82`) also reads "16 total - RFC-010: 'type' keyword
> removed"—**that 16 is an outdated comment**, and 18 are actually listed (including `and` / `or`,
> while the `Kw*` prefixed batch is still 16).
>
> **`pub` produces no visibility effect**: `pub` is still recognized by the lexer as `KwPub`
> (`src/frontend/core/lexer/state.rs:28`), and the parser skips it at declaration and import items
> (`src/frontend/core/parser/statements/declarations.rs:666-671,726`, `.../imports.rs:57-59`), but
> the module system **does not make any visibility judgment based on it**—the accepted
> [RFC-029](../../rfc/accepted/029-module-semantics.md) explicitly states "no `pub`, no `private`,
> no `export`, no visibility mechanism" (line 17 of that file). Whether you write `pub` or not makes
> no difference to visibility.

### 1.4 Reserved Words

YaoXiang's "reserved words" are divided into three layers, recognized by the parser and type checker
at different stages:

#### 1.4.1 Literal Reserved Words

Identifiers for literals with independent tokens in the parser, which cannot be used as ordinary
identifiers:

| Identifier | Belongs to Type | Description                                                                                                   |
| ---------- | --------------- | ------------------------------------------------------------------------------------------------------------- |
| `true`     | Bool            | Boolean true value                                                                                            |
| `false`    | Bool            | Boolean false value                                                                                           |
| `void`     | Void            | Void literal (Unit value). Lowercase `void` is a value literal; uppercase `Void` is a type name (see §1.4.3). |

> **`Type` is not in this layer**: it does **not** have an independent `TokenKind` (it's not in the
> `TokenKind` enum in `src/frontend/core/lexer/tokens.rs`, and `keyword_from_str` has no
> corresponding branch). The parser treats it as an ordinary identifier, and the type checker
> recognizes it as a meta type in type positions. Therefore, in expression positions, `Type` can be
> shadowed by local bindings. It is a **meta type name**, not a keyword.

#### 1.4.2 Variant Names

Variant names are recognized by the parser in **pattern** contexts (bare names are resolved
according to the scrutinee type's variant set, see §2.8); in **expression** contexts, constructing
variants requires type qualification—`Result(Int, String).ok(5)`, `Option(T).some(v)`,
`Color.green()` (RFC-010 record types and types: variant constructors are function-typed fields in
type definitions, and bare `ok(5)` is not a constructor call).

| Variant Name | Belongs to Type | Description                      |
| ------------ | --------------- | -------------------------------- |
| `some(T)`    | Option          | Option value variant constructor |
| `ok(T)`      | Result          | Result success variant           |
| `err(E)`     | Result          | Result error variant             |

#### 1.4.3 Built-in Type Names

The following type names are pre-registered by the type checker and can be used in type positions
without import. The parser treats them as ordinary identifiers—**not reserved words, can be shadowed
by local bindings (not recommended)**.

| Type Name | Logical Correspondence | Description                                                                                                                                  |
| --------- | ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `Void`    | ⊤ (true/Unit)          | Zero-field product type, exactly one inhabitant (the `void` literal, see §1.4.1)                                                             |
| `Never`   | ⊥ (false/empty type)   | Zero-variant sum type, zero inhabitants. No expression can produce a `Never` value. `Never <: T` holds for all `T` (principle of explosion). |
| `Int`     | —                      | Signed integer                                                                                                                               |
| `Float`   | —                      | Floating-point number                                                                                                                        |
| `Bool`    | —                      | Boolean value: `true` / `false`                                                                                                              |
| `Char`    | —                      | Unicode character                                                                                                                            |
| `String`  | —                      | String                                                                                                                                       |

### 1.5 Identifiers

Identifiers start with a letter or underscore, followed by letters, digits, or underscores.
Identifiers are case-sensitive.

Special identifiers:

- `_` is used as a placeholder, meaning a value is ignored
- Identifiers starting with an underscore represent private members

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
Array       ::= '[' Expr (',' Expr)* ']'   // When the target type annotation is Array(T, N), the literal lands as a fixed-length array
```

> **Dictionary literals require at least one key-value pair**: `{}` is **not** an empty
> dictionary—it is an empty block (value `Void`, see [§2.9](#_2-9-block-expressions)). For an empty
> dictionary, use the constructor `dict.new()`:
>
> ```yaoxiang
> empty = dict.new()                  // ✅ empty dictionary
> d = { "a": 1 }                       // ✅ dictionary literal
> wrong = {}                           // ❌ This is not a dictionary, it's an empty block (Void)
> ```
>
> **The criterion is content**: The `Dict` grammar requires at least one `String ':' Expr`. Since
> `{}` has no content to base a decision on, it takes the zero form of the block structure.
> Non-empty forms are self-describing by content (`{ "k": v }` has a key-value pair →
> dictionary)—this is from the same source as `f = { 5 }` being an `Int` value rather than a
> function: **the type is determined by the content**.

> Sets have no literal grammar, no runtime representation—they are planned in the collection type
> roadmap, to be completed following the Dict pattern when the need arises (std.set +
> HeapValue::Set). The landing point of List/Dict literals is determined by context type annotation:
> bare literals and `List(T)` annotations land as growable lists; `Array(T, N)` annotations acting
> directly on literals land as fixed-length arrays. Implicit List→Array conversion is forbidden.
>
> Array literal semantics:
>
> - The number of elements must equal N, otherwise compile-time E1002; an empty literal paired with
>   non-zero N is also rejected
> - Each element type must be compatible with T, otherwise compile-time E1002
> - The grammar form of N: only integer literals (possibly negative) or constant names; composite
>   expressions (e.g., `2+1`) are rejected at parse time
> - When N is a symbolic constant (function const parameter, e.g., `Array(Int, n)`), the count check
>   is deferred to the refinement type stage
> - v1 nested array literals (`Array(Array(Int,2),2) = [[1,2],[3,4]]`) are rejected at compile time,
>   requiring explicit construction layer by layer; recursive landing points are left for a later
>   version

#### 1.6.5 List Comprehensions

```
ListComp    ::= '[' Expr ( 'for' Identifier 'in' Expr ( 'if' Expr )? )+ ']'
```

> **#401 (implementation completion)**: `if` filtering and multiple generator clauses are landed
> starting with this version—the previous grammar was written as `(',' Expr)* ('if' Expr)?`, with no
> implementation backing it (neither comma-appended expressions nor multiple generators are
> supported), and `if` filtering was entirely missed by the parser (after parsing the iterable it
> directly expected `']'`, reporting `E0010 Expected RBracket, found KwIf`). The current resolution
> follows industry-standard semantics: one or more generator clauses, expanded as nested loops; each
> clause can carry at most one `if` filter, with the condition forced to `Bool` (non-Bool reports
> `E1054`); subsequent clauses' iterable/condition may reference iteration variables bound by
> previous clauses.

> **Behavior tightening (migration record)**: The iteration variable grammar has always been
> `'for' Identifier 'in'`, but the old implementation's pattern went through full pratt
> parsing—after `'in'` was registered as an infix operator, `x` would swallow `in items` as a
> membership expression. After the fix, non-identifier patterns directly fail to parse, no longer
> falling back to `_` like the old implementation (silently swallowing errors). Impact: previously
> parseable syntax like `[x for (a, b) in pairs]` now errors out—this form never had defined
> behavior (variable was always `_`), and the tightening direction is correct, with no semantic
> migration cost.

#### 1.6.6 Membership Test

```
Membership  ::= Expr 'in' Expr
```

> `in` is a binary relational operator that returns `Bool`—`true` on hit, `false` on miss, no error.
> Semantic split: `[]` asserts existence and takes a value (errors on failure), `in` asks whether it
> exists (miss is normal `false`). Right operand coverage: List / Array / Dict (key set) / Tuple /
> String (substring) / Range (interval). `in` is a first-class Hoare predicate, serving as the basis
> for compile-time provable propositions at the refinement type stage. (Set is removed from the
> right operand list—Set has no runtime representation, see §1.6.4)

### 1.7 Comments

```
// Single-line comment

/* Multi-line comment
   can span multiple lines */
```

### 1.8 Indentation Rules

Code must use 4 spaces for indentation; Tab characters are forbidden. This is a mandatory syntactic
rule.

---

## Chapter 2: Grammar Rules

### 2.1 Expression Classification

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

> **Unary prefix operators** (`!` `-` `+`) are tightly bound: only below call and member access,
> higher than all binary operators. Therefore `!a == b` ≡ `(!a) == b` (Zig-style semantics); `!` is
> a pure unary operation, does not participate in short-circuit control flow, and is orthogonal to
> the `and`/`or` keywords (short-circuit) (RFC-010 authoritative definition).

> **Range binding power**: `..` binding power (6, 7)—left 6 is lower than addition (7), right 7
> swallows addition but not same-level `..`. Before/after change comparison:
>
> | Expression   | Before change (level 1, right-assoc)                                                         | After change ((6,7), left-assoc)                                               |
> | ------------ | -------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------ |
> | `x in 1..10` | `x in 1..10` (`in` right operand level 4, `..` level 1 cannot swallow, actually unparseable) | `x in (1..10)`—the range as a whole is the right operand of `in`               |
> | `0..n+2`     | `(0..n)+2` (right-assoc trap: upper bound eaten, `for` loop directly E3004)                  | `0..(n+2)`—upper bound is an arithmetic expression                             |
> | `a == b..c`  | `a == (b..c)` (`..` level 1 < `==` level 3, naturally whole)                                 | `a == (b..c)`—**semantics unchanged**, `..` still higher than comparison level |
> | `1..2*3`     | `(1..2)*3`                                                                                   | `1..(2*3)`—upper bound is an arithmetic expression                             |
> | `a..b..c`    | `a..(b..c)` (right-assoc chained, meaningless Range nested in Range)                         | `(a..b)..c`—**step form** (`c` is the step)                                    |
>
> Net effect: composite upper bound `for i in 0..n+2` changes from "parses but E3004" to "directly
> usable"; `x in 1..10` changes from "unparseable" to "interval check"; `a..b..c` changes from
> "meaningless nesting" to "step component". Level 6 falls between `+` (level 5) and `<<` (level 7),
> matching mathematical convention: an interval is a tightly bound construct, so the upper bound is
> naturally a complete arithmetic expression.

### 2.3 Function Calls

```
FnCall      ::= Expr '(' ArgList? ')'
ArgList     ::= Expr (',' Expr)* (',' NamedArg)* | NamedArg (',' NamedArg)*
NamedArg    ::= Identifier '=' Expr
```

Named arguments use `name = value` (RFC-010 §Function Definitions, RFC-011 §Construction Forms).
Positional arguments must come before named arguments; order-specified parameters can be arranged
arbitrarily in named arguments:

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b

add(3, 5)          // positional
add(a = 3, b = 5)  // named
add(b = 5, a = 3)  // arbitrary order
add(3, b = 5)      // mixed, positional first
```

An incorrect name in a named argument reports **E1014**, the same formal parameter being specified
both positionally and by name reports **E1015**, and mismatched argument count reports **E1010**
(RFC-013).

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
> instantiation in the type body + `Grid.index` method); ③ RFC-004's `f[0]` position binding only
> exists in binding declarations, not in this grammar. Multi-dimensional index `a[0, 1]` packs keys
> as a tuple. Other types that don't implement `Index` are rejected at the type layer.

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

> **Variant set requirement for variant destructuring** (RFC-010b): `EnumPattern` (variant names
> like `ok(v)`, `some(x)` resolved by scrutinee) requires that the scrutinee's type be a **sum type
> with the variant set present**. The variant set enters the checker only through type definitions
> or `use` imports—`Result` / `Option` are defined by `std.result` / `std.option`, and
> `use std.result` / `use std.option` is required before use (whole-module and grouped
> `use std.{...}` forms have equal weight); otherwise variant destructuring reports E1002.
> Exhaustiveness checking is the same source: a default arm is exempt from exhaustiveness; without a
> default arm, all variants in the set must be covered.

### 2.9 Block Expressions

```
Block       ::= '{' Stmt* Expr? '}'
```

> **Statement termination rules**: The rules for separating Stmts and newline behavior (explicit `;`
> separator, newline termination, line continuation exceptions, leading `(`/`[` never merged) are
> defined by [RFC-038](../../rfc/accepted/038-statement-termination.md).

#### 2.9.1 Three Forms of `{`

`{` in an expression position has exactly three interpretations, determined in one pass by
**content**:

| Form                   | Notation          | Type                 | Example          |
| ---------------------- | ----------------- | -------------------- | ---------------- |
| **Empty block**        | `{}`              | `Void`               | `x: Void = {}`   |
| **Dictionary literal** | `{ "k": v, ... }` | `Dict(K, V)`         | `d = { "a": 1 }` |
| **Block**              | `{ Stmt* Expr? }` | Tail expression type | `y = { 1 + 1 }`  |

**Resolution order**:

1. `{}` (no content) → **Empty block**, value `Void`
2. First element is `String ':' Expr` key-value pair → **Dictionary literal**
3. Otherwise → **Block**, value given by the tail expression

> **Why `{}` is not an empty dictionary**: The "emptiness" of an empty dictionary cannot be
> self-describing (it could be `Dict(K, V)` or an empty block), while the dictionary grammar
> [§1.6.4](#_1-6-4-collections) requires at least one key-value pair. When there is no content to
> base a decision on, the zero form of the block structure is taken: this is consistent with
> `unsafe {}` / `spawn {}`, with no special case. For an empty dictionary, use `dict.new()`.
>
> **Why functions need annotations**: `f = { stmt }` is a **value** (tail expression type), not a
> function. To define a function, write the Fn annotation: `f: () -> Int = { 5 }`. This follows the
> same principle as dictionaries: **the type is determined by the content**, not by whether an
> annotation exists. (This rule is in
> [RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md) Appendix D.)

**Unified semantics**: The value of all `{}` blocks is given by the **tail expression**; `return` is
a non-local exit of type `Never`.

| Block Type  | Value Exit      | Empty block `{}` |
| ----------- | --------------- | ---------------- |
| Normal `{}` | Tail expression | `Void`           |
| `unsafe {}` | Tail expression | `Void`           |
| `spawn {}`  | Tail expression | `Void`           |

**Core principles** (see [RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md) for
details):

- **Block value = tail expression** (the last expression), the only exit, no exceptions
- When the last position is an **assignment statement**, the block value is `Void`; if you want
  `Void`, write `Void` explicitly
- **`return` exits the nearest function boundary** (penetrates all blocks, does not "return to the
  block"), type `Never`; `Never <: T` holds for any type (principle of explosion), so it can appear
  at any return type position
- Expression form `= expr` directly gives a value

```yaoxiang
// Normal {} block: tail expression gives the value
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

// return: penetrates the block, exits the function
f: (n: Int) -> Int = {
    if n < 0 {
        return 0     // penetrates if and the function body, exits the function
    }
    n * 2            // tail expression
}
```

#### Is `name = { ... }` a Function or a Block Value?

`name = { ... }` could be either a function definition (RFC-007 "zero-arg simplest") or a block
value binding. Resolved by **annotation first, function by default** (RFC-010a Appendix D):

| Case                              | Result          | Example                            |
| --------------------------------- | --------------- | ---------------------------------- |
| Value is a Lambda (`=>`)          | Function        | `f = () => 5` → `f()` = 5          |
| Annotation is a function type     | Function        | `f: () -> Int = { 5 }` → `f()` = 5 |
| Annotation is a non-function type | **Block value** | `x: Int = { 5 }` → `x` = 5         |
| No annotation                     | Function        | `f = { 5 }` → `f()` = 5            |

**Annotation is type**: `x: Int = ...` declares `x` to be `Int`, then `{ ... }` evaluates to `Int`;
`f: () -> Int = ...` declares `f` to be a function, then `{ ... }` is the function body.

To evaluate `{ ... }` on the spot, **just write the target type** (no new syntax needed):

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

There are two readings of the nested function type on the right side of `->`, distinguished by
**parentheses** (RFC-004):

| Notation                        | Meaning              | Call                    |
| ------------------------------- | -------------------- | ----------------------- |
| `(a: Int) -> (b: Int) -> Int`   | **Currying**         | `f(1)(2)`               |
| `(a: Int) -> ((b: Int) -> Int)` | **Returns function** | `g(1)` gives a function |

Basis: **Annotation is type**. `g: (a: Int) -> ((b: Int) -> Int)` declares `g(1) : (b: Int) -> Int`,
so `g(1)` must **be** that function, not "the next parameter segment".

```yaoxiang
// Currying: two parameter segments given layer by layer
add: (a: Int) -> (b: Int) -> Int = { a + b }
add(1)(2)        // → 3

// Returns function: outer layer one parameter segment, what is returned is the function
adder: (n: Int) -> ((x: Int) -> Int) = (x) => x + n
adder(10)(5)     // → 15
h = adder(10)    // The returned function can be stored in a variable, passed around
```

Unparenthesized nested `Fn` is always currying (including the type parameter form from RFC-011):

```yaoxiang
identity: (T: Type) -> (x: T) -> T = (x) => x   // currying
identity(5)      // → 5
```

**Type checking**: Parentheses declare the return type, and the body must produce that type.
`f: () -> (() -> Int) = { 7 }` is an error—`() -> Int` is expected, but `Int` is actually obtained
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

The `?` operator is a postfix operator with the same precedence as `.`. For `Result(T, E)` types:

- On `Ok(v)`, extract the value `v` and continue execution
- On `Err(e)`, propagate the error upward (`return Err(e)`)

```yaoxiang
process: (data: Data) -> Result(Data, Error) = {
    validated = validate(data)?     // extract value on success, propagate on failure
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

// Range is a value: binding, passing, membership test
r = 1..10
assert.assert(5 in r, "membership")
for i in r { print(i) }

// step form (third component, default 1)
for i in 0..10..2 { print(i) }  // 0, 2, 4, 6, 8
for i in 10..0..(-2) { print(i) }  // 10, 8, 6, 4, 2
```

> **Step semantics**: In `a..b..c`, `c` is the step. `c = 0` literal is rejected at compile time;
> dynamic `c` is zero-checked at runtime (E6001 family; promoted to Result after the error system
> lands). `c < 0` is legal, the interval direction is reversed by sign (`10..0..(-2)` decreases).

### 2.13 ref Expression

```
RefExpr     ::= 'ref' Expr
```

`ref` creates shared ownership. The compiler automatically selects Rc (single-task) or Arc
(cross-task), and users don't need to care about implementation details.

```yaoxiang
data = ref heavy_data
spawn { use(data) }   // cross-task: compiler automatically selects Arc
```

### 2.14 unsafe Expression

```
UnsafeExpr  ::= 'unsafe' Block
```

`unsafe` blocks are used to define opaque types and operate on raw pointers. Use `return` to return
the type definition to the enclosing scope.

**Semantics**:

- Types can be defined and raw pointers operated on within `unsafe {}`
- The returned type is available outside the `unsafe {}`
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

### 2.15 Scopes

**Basic rules**:

- Each `{}` block creates a scope
- Inner scopes can access variables in outer scopes
- Outer scopes cannot access variables in inner scopes
- Variable declarations follow the "assign first" principle

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

- `x = value`: search x outward along the scope chain; assign if found, otherwise newly declare
- `mut x = value`: explicit new mutable declaration, forbidden to share the name with the outer
  scope
- Any name in the same scope can only be declared once

> **Detailed definition**: The complete rules for scopes, variable declarations, and shadowing
> mechanisms are detailed in the [Module System Specification](modules.md#chapter-4-scopes).

---

## Chapter 3: Statements

### 3.1 Statement Classification

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

**Semantics**: `return` is a **non-local exit**, exiting the nearest **function boundary**
(penetrates all blocks—including `if` / `while` / `for` / `match` / bare block / `spawn` /
`unsafe`), passing the value to the caller. It does **not "return to the block"**.

**Type**: `return e : Never` (where `e : T`). `Never <: T'` holds for any `T'` (principle of
explosion, see [Type System §2.2](type-system.md)), so `return` can appear at any return type
position without additional rules to constrain it.

**Relationship with block evaluation**: The block's value is always the **tail expression** (see
§2.9). `{ return n }` as a block has the value `n`, of type `Never`; at the same time, the effect of
`return` is to exit the function. **Both things hold simultaneously**, coexisting through the
principle of explosion.

`return` and the tail expression together make "early return" work, without the need for additional
rules that `return` specifically refers to a function—see
[RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md).

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 {
        return 1          // penetrates if, exits the function (type Never)
    }
    n * factorial(n - 1)  // tail expression = block value
}
```

### 3.4 break Statement

```
BreakStmt   ::= 'break'
```

**Semantics**: Immediately terminate the innermost `while`/`for` loop, with control flow moving to
after the loop body.

- **Only exits the nearest level**: `break` always acts on the innermost loop containing it. When
  you need to break out multiple levels in a nested loop, extract the inner loop into a function and
  use `return`, or use a flag (break/continue take no label; if loop labels are introduced in the
  future, they will follow the loop declaration-side syntax through the RFC process, decided
  together with the proof pipeline's multi-exit design)
- **Limited to the loop body**: `break` can only appear within `while`/`for` loop bodies (including
  blocks/if/match nested in the body); appearing outside a loop is a compile error (E1102
  `'break' outside of a loop`)
- **Does not affect termination proof**: `break` does not participate in termination
  arguments—neither providing a measure nor constituting a measure's decreasing step; the loop's
  termination obligation is unrelated to `break`, triggered by refinement types (see
  [type-system §8.4](type-system.md#84-terminates-termination-measure-predicate)).
- **Borrow semantics**: The control flow edge of break participates in the structural cut of
  RFC-009a reverse BFS liveness analysis (the jumped-out iteration does not participate in back-edge
  liveness derivation)

```yaoxiang
mut i = 0
while i < 10 {
    i = i + 1
    if i == 3 {
        break              // control flow moves to after the loop, i == 3
    }
}

// Nested loop: break only exits the inner level
while j < 3 {
    while k < 10 {
        if k == 2 { break }    // only terminates the inner loop
    }
    j = j + 1                  // each outer iteration will execute here
}
```

### 3.5 continue Statement

```
ContinueStmt::= 'continue'
```

**Semantics**: Skip the remaining statements in this iteration, directly entering the next round of
the innermost loop—`while` returns to re-evaluating the condition, `for` takes the next element.

- **Only acts on the nearest level**: Same as `break`, takes no label
- **Limited to the loop body**: Appearing outside a loop is a compile error (E1102)

```yaoxiang
mut sum = 0
mut n = 0
while n < 5 {
    n = n + 1
    if n == 3 {
        continue           // skip the following accumulation, n == 3 is not counted
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

YaoXiang's for loop semantics differ from traditional languages: **each iteration is a new binding,
not modifying the same variable**.

```yaoxiang
// Example: for i in 1..5
for i in 1..5 {
    print(i)
}
```

**Execution process**:

| Iteration | Behavior of the loop variable                                                           |
| --------- | --------------------------------------------------------------------------------------- |
| 1st       | Create new binding `i = 1`, execute the loop body, print 1                              |
| 2nd       | Create new binding `i = 2` (previous binding destroyed), execute the loop body, print 2 |
| 3rd       | Create new binding `i = 3`, execute the loop body, print 3                              |
| 4th       | Create new binding `i = 4`, execute the loop body, print 4                              |
| End       | Loop body ends, bindings destroyed                                                      |

**Key point**: After each iteration ends, the binding created in that iteration is destroyed. The
next iteration is a brand-new binding, having no relationship with the binding of the previous
iteration.

#### 3.9.2 Difference Between for and for mut

| Syntax              | Loop Variable Mutability | Description                                |
| ------------------- | ------------------------ | ------------------------------------------ |
| `for i in 1..5`     | Immutable                | Cannot modify the binding in the loop body |
| `for mut i in 1..5` | Mutable                  | Can modify the binding in the loop body    |

```yaoxiang
// Legal: each iteration binds a new value, no need to modify
for i in 1..5 {
    print(i)  // read the value of i
}

// Error: immutable binding, cannot modify
for i in 1..5 {
    i = i + 1  // Error: cannot modify an immutable binding
}

// Legal: use for mut to allow modifying the binding
for mut i in 1..5 {
    i = i + 1  // modification allowed
}
```

#### 3.9.3 Shadowing Check

YaoXiang prohibits variable shadowing. The for loop variable cannot share the same name as a
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

This rule applies to all code blocks, see [4.3 Shadowing Rules](modules.md#43-shadowing-rules) for
details.

#### 3.9.4 Comparison with Other Languages

| Language | for Loop Variable Semantics                             |
| -------- | ------------------------------------------------------- |
| YaoXiang | Each iteration binds a new value                        |
| Rust     | Modifies the same variable (needs mut)                  |
| Python   | Modifies the same variable (no mut needed)              |
| C/C++    | Modifies the same variable (needs pointer or reference) |

**Design rationale**: YaoXiang adopts binding semantics because:

1. **More in line with natural semantics**. In natural language, "for every element x in the
   collection" means each x is an independent individual. YaoXiang's `for i in 1..5` reads as "for
   every i from 1 to 5", where the i in each iteration is a brand-new binding, consistent with human
   intuitive understanding.

2. **Avoid accidental modification**. The default immutable binding semantics means the loop
   variable cannot be accidentally modified within the loop body. No need to worry about writing
   `i = ...` somewhere in a complex loop body leading to hard-to-trace bugs.

3. **High-performance solutions are within reach**. When you really need to reuse a variable across
   iterations (e.g., accumulator, cache), use `for mut` to switch to mutable binding mode. This is
   clearer than implicit shared state—intent is expressed explicitly through syntax, not hidden in
   runtime behavior.

### 3.10 spawn Statement

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' 'mut'? Identifier 'in' Expr '{' Expr '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
```

**spawn block**: Explicitly declares a concurrent scope; expressions within the block execute
concurrently.

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

**spawn block captures outer variables** (RFC-024 §2.3, value capture semantics):

- The block body referencing outer variables = **Move value capture**: the value is snapshotted into
  the closure environment at the spawn creation point, and the block body reads through the env
  (LoadUpvalue)
- **Primitives** (Int/Float/Bool/Char) are value-copied, outer variables are not affected
- **Handle types** (Struct/String/List, etc.) snapshot = handle copy, sharing the underlying object;
  the Embedded runtime (default) has the same thread and heap, so the handle is valid
- Sharing between multiple tasks requires explicit `ref` (§2.13, compiler automatically selects
  Rc/Arc)
- Outer variables referenced by `return` within the block are captured the same way

```yaoxiang
t1 = 1 + 1
t2 = 2 + 2
result = spawn {
    return t1 + t2    // t1/t2 value-captured, result == 6
}
```

---

### 3.11 Program Entry and Top-Level Statements

In the compiler's eyes, a source file has two roles, **determined by the presence of
`yaoxiang.toml`**:

| Role       | Determination                              | Program Body                                                                     |
| ---------- | ------------------------------------------ | -------------------------------------------------------------------------------- |
| **Script** | Single-file direct run, no `yaoxiang.toml` | **Top-level statements** (executed in writing order); `main` is a normal binding |
| **Bin**    | `yaoxiang.toml` exists                     | **`main` function**; no executable statements allowed at the top level           |

#### Script: Top-Level Statements Are the Program

Without a manifest, the file is a "script", and **top-level statements execute in writing order**:

```yaoxiang
use std.io
io.println("hello")          // direct execution
x: Int = { 42 }              // top-level binding: runtime initialization
io.println(x)                // 42
```

In this mode, `main` is **not special**—it's just a normal binding. To make `main` run, you must
explicitly call it:

```yaoxiang
use std.io
main: () -> Void = { io.println("only runs if called") }

main()                       // ← This line must be written
```

> **Why not automatically call `main`?** Top-level statements are already the program body. If
> `main` were also implicitly called, a script that explicitly writes `main()` would execute twice.
> The two rules cannot coexist, so under Script there is only one entry point: "top-level
> statements".

#### Bin: `main` is the Entry Point

When a manifest is present, the file is an "executable target", and at this time:

- A binding named `main` must be defined, which can be either a value or a function (#388
  resolution: the entry is determined by binding existence—a function is already a value, the two
  just differ in evaluation strategy): function main is called with zero arguments at entry; value
  main is evaluated at initialization, which executes
- Executable statements are not allowed at the top level—the program body is `main`

```yaoxiang
main: () -> Void = {
    print("hello")
}
```

A value main is also legal—evaluated at initialization, which executes; evaluation happens in the
initialization sequence (topological order by global bindings, independent bindings by source
order):

```yaoxiang
main = {
    print("hello")
}
```

Missing `main` is a compile error (no `main` means all functions are unreachable). A non-callable
value main (e.g., `main: Int = 5`) is legal but has no observable effect—corresponding to Rust's
empty `fn main() {}`.

> **Library files**: Files used by other files via `use`, or files pointed to by `[lib].path` /
> `[exports]`, do not require `main`—they are not program entry points.

#### Initialization of Top-Level Bindings

The initialization value of top-level bindings is **evaluated at runtime**, not required to be a
compile-time constant:

```yaoxiang
answer: Int = { 42 }              // block value
inc: (Int) -> Int = (x) => x + 1
computed: Int = inc(41)           // function call
```

Initialization executes in **dependency order**, regardless of writing order:

```yaoxiang
derived: Int = base * 3           // references later-declared base
base: Int = 7                     // initialized first (topological sort)
```

Cyclic dependencies are a compile error (the names on the cycle are listed):

```yaoxiang
a: Int = b + 1
b: Int = a + 1                    // Error: a → b → a
```

> **Design basis**: RFC-029f (file role model), RFC-010a Appendix D (block binding resolution).

---

## Appendix: Syntax Cheat Sheet

### A.0 Keywords (18)

```
pub     use     spawn  ref     mut
if      else    match  while   for
in      return  break  continue
as      unsafe  and    or
```

`type` is no longer a keyword (RFC-010, use `Name: Type = { ... }` instead); `pub` produces no
visibility effect (RFC-029). Literal reserved words `true` / `false` / `void` are in §1.4.1; meta
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
