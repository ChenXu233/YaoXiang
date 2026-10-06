# Syntax Specification

This document defines the syntax specification of the YaoXiang programming language, including
lexical structure, grammar rules, and operator precedence.

---

## Chapter 1: Lexical Structure

### 1.1 Source Files

YaoXiang source files must use UTF-8 encoding. Source files typically use the `.yx` extension.

### 1.2 Lexical Token Categories

| Category   | Description                   | Examples                  |
| ---------- | ----------------------------- | ------------------------- |
| Identifier | Starts with letter/underscore | `x`, `_private`, `my_var` |
| Keyword    | Language reserved words       | `use`, `mut`, `and`       |
| Literal    | Fixed value                   | `42`, `"hello"`, `true`   |
| Operator   | Operation symbols             | `+`, `-`, `*`, `/`        |
| Delimiter  | Syntax separators             | `(`, `)`, `{`, `}`, `,`   |

### 1.3 Keywords

YaoXiang has **18 keywords** (`keyword_from_str` in `src/frontend/core/lexer/state.rs:25-55`, each
corresponding to a `TokenKind`):

```
pub     use     spawn  ref     mut
if      else    match  while   for
in      return  break  continue
as      unsafe  and    or
```

`and` / `or` are **keywords** for logical and / or (Zig-style, see precedence level 10 in §2.2);
unary not is the symbol `!`, orthogonal to them.

These keywords have special meaning in any context and cannot be used as identifiers.

> **`type` is no longer a keyword** (RFC-010): use the `Name: Type = { ... }` notation to write type
> definitions. `src/frontend/core/lexer/state.rs:27` explicitly comments on this, and the
> `TokenKind` enum header (`src/frontend/core/lexer/tokens.rs:82`) also reads "16 total - RFC-010:
> 'type' keyword removed"—**that 16 is an outdated comment**; in reality 18 are listed (including
> `and` / `or`, while the `Kw*` prefixed batch is still 16).
>
> **`pub` has no visibility effect**: `pub` is still lexically recognized as `KwPub`
> (`src/frontend/core/lexer/state.rs:28`), and the parser skips it at declaration and import items
> (`src/frontend/core/parser/statements/declarations.rs:666-671,726`, `.../imports.rs:57-59`), but
> the module system **does not make any visibility decisions based on it**—the accepted
> [RFC-029](../../rfc/accepted/029-module-semantics.md) explicitly states "no `pub`, no `private`,
> no `export`, no visibility mechanism" (line 17 of that document). Whether or not `pub` is written
> has no effect on visibility.

### 1.4 Reserved Words

YaoXiang's "reserved words" are organized into three layers, recognized by the parser and type
checker at different stages:

#### 1.4.1 Literal Reserved Words

The parser has independent tokens for these literal identifiers and they cannot be used as regular
identifiers:

| Identifier | Type | Description                                                                                                   |
| ---------- | ---- | ------------------------------------------------------------------------------------------------------------- |
| `true`     | Bool | Boolean true value                                                                                            |
| `false`    | Bool | Boolean false value                                                                                           |
| `void`     | Void | Void literal (Unit value). Lowercase `void` is a value literal; uppercase `Void` is a type name (see §1.4.3). |

> **`Type` is not in this layer**: it has **no** independent `TokenKind` (it is not in the
> `TokenKind` enum in `src/frontend/core/lexer/tokens.rs`, and `keyword_from_str` has no
> corresponding branch), the parser treats it as a regular identifier, and the type checker
> recognizes it as a meta type in type positions. Therefore in expression positions `Type` can be
> shadowed by local bindings. It is a **meta type name**, not a keyword.

#### 1.4.2 Variant Names

Variant names are recognized by the parser in **pattern** context (bare names are resolved by the
variant set of the scrutinee's type, see §2.8); in **expression** context, constructing a variant
requires type qualification—`Result(Int, String).ok(5)`, `Option(T).some(v)`, `Color.green()`
(RFC-010 record syntax and types: variant constructors are function-typed fields in type
definitions; bare `ok(5)` is not a constructor call).

| Variant   | Type   | Description                      |
| --------- | ------ | -------------------------------- |
| `some(T)` | Option | Option value variant constructor |
| `ok(T)`   | Result | Result success variant           |
| `err(E)`  | Result | Result error variant             |

#### 1.4.3 Built-in Type Names

The following type names are pre-registered by the type checker and can be used in type positions
without import. The parser treats them as regular identifiers—**they are not reserved words and can
be shadowed by local bindings (not recommended)**.

| Type Name | Logical Correspondence | Description                                                                                                                                  |
| --------- | ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `Void`    | ⊤ (true/Unit)          | Zero-field product type, with exactly one inhabitant (`void` literal, see §1.4.1)                                                            |
| `Never`   | ⊥ (false/empty type)   | Zero-variant sum type, zero inhabitants. No expression can produce a `Never` value. `Never <: T` holds for all `T` (principle of explosion). |
| `Int`     | —                      | Signed integer                                                                                                                               |
| `Float`   | —                      | Floating-point number                                                                                                                        |
| `Bool`    | —                      | Boolean value: `true` / `false`                                                                                                              |
| `Char`    | —                      | Unicode character                                                                                                                            |
| `String`  | —                      | String                                                                                                                                       |

### 1.5 Identifiers

Identifiers start with a letter or underscore, and subsequent characters can be letters, digits, or
underscores. Identifiers are case-sensitive.

Special identifiers:

- `_` is used as a placeholder to indicate that a value should be ignored
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

#### 1.6.4 Collection

```
List        ::= '[' Expr (',' Expr)* ']'
Dict        ::= '{' String ':' Expr (',' String ':' Expr)* '}'
Array       ::= '[' Expr (',' Expr)* ']'   // When target type annotated as Array(T, N), literal lands as fixed-length array
```

> **Dict literal requires at least one key-value pair**: `{}` is **not** an empty dict—it is an
> empty block (value `Void`, see [§2.9](#_2-9-block-expression)). For an empty dict, use the
> constructor `dict.new()`:
>
> ```yaoxiang
> empty = dict.new()                  // ✅ empty dict
> d = { "a": 1 }                       // ✅ dict literal
> wrong = {}                           // ❌ this is not a dict, it's an empty block (Void)
> ```
>
> **The criterion is the content**: The `Dict` grammar requires at least one `String ':' Expr`; `{}`
> has no content to rely on, so it takes the zero form of block structure. Non-empty forms are
> self-describing by content (`{ "k": v }` has a key-value pair → dict)—this is the same source as
> why `f = { 5 }` is an `Int` value, not a function: **the type is determined by the content**.

> Set has no literal grammar and no runtime representation—it's part of the collection type
> planning, and will be added following the Dict pattern when requirements emerge (std.set +
> HeapValue::Set). The landing point of List/Dict literals is determined by context type annotation:
> bare literals and `List(T)` annotations land as growable lists; `Array(T, N)` annotations acting
> directly on literals land as fixed-length arrays. Implicit List→Array conversion is forbidden.
>
> Array literal semantics:
>
> - The number of elements must equal N; otherwise compile-time E1002; an empty literal paired with
>   non-zero N is also rejected
> - Each element's type must be compatible with T; otherwise compile-time E1002
> - Grammar form of N: only integer literals (may be negative) or constant names; compound
>   expressions (e.g., `2+1`) are rejected at parse time
> - When N is a symbolic constant (function const parameter, e.g., `Array(Int, n)`), the count check
>   is deferred to the refined type stage
> - Nested array literals (e.g., `Array(Array(Int,2),2) = [[1,2],[3,4]]`) are rejected at compile
>   time in v1, requiring explicit construction layer by layer; recursive landing is left to future
>   versions

#### 1.6.5 List Comprehension

```
ListComp    ::= '[' Expr ( 'for' Identifier 'in' Expr ( 'if' Expr )? )+ ']'
```

> **#401 (implementation completion)**: `if` filtering and multiple generator clauses take effect
> from this version—previously the grammar was written as `(',' Expr)* ('if' Expr)?`, with no
> implementation support (neither comma-appended expressions nor multiple generators were
> supported), and `if` filtering was also entirely missed by the parser (after parsing the iterable
> it directly expected `']'`, reporting `E0010 Expected RBracket, found KwIf`). The current decision
> follows industry-standard semantics: one or more generator clauses, expanded as nested loops; each
> clause can carry at most one `if` filter, and the condition is forced to be `Bool` (non-Bool
> reports `E1054`); subsequent clauses' iterable/condition can reference iteration variables bound
> by previous clauses.

> **Behavior tightening (migration record)**: The iteration variable grammar has always been
> `'for' Identifier 'in'`, but in the old implementation, patterns went through full Pratt
> parsing—after `'in'` was registered as an infix operator, `x` would swallow `in items` into a
> membership expression. After the fix, non-identifier patterns fail to parse directly, and no
> longer fall back to `_` like the old implementation (silently swallowing errors). Impact:
> previously parsable syntax like `[x for (a, b) in pairs]` now reports an error—this form never had
> defined behavior (the variable was always `_`), so the tightening direction is correct and incurs
> no semantic migration cost.

#### 1.6.6 Membership Test

```
Membership  ::= Expr 'in' Expr
```

> `in` is a binary relational operator that returns `Bool`—returns `true` on hit, `false` on miss,
> and does not error. Semantic split: `[]` asserts existence and takes a value (errors on failure),
> while `in` asks whether something exists (a miss is a normal `false`). Right operand coverage:
> List / Array / Dict(key set) / Tuple / String(substring) / Range(interval). `in` is a first-class
> Hoare predicate, serving as the basis for compile-time-provable propositions in the refined type
> stage. (Set is removed from the right operand list—Set has no runtime representation, see §1.6.4)

### 1.7 Comments

```
// Single-line comment

/* Multi-line comment
   can span multiple lines */
```

### 1.8 Indentation Rules

Code must use 4 spaces for indentation; tab characters are forbidden. This is a mandatory syntax
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

> **Unary prefix operators** (`!` `-` `+`) bind tightly: only below call and member access, and
> higher than all binary operators. Therefore `!a == b` ≡ `(!a) == b` (Zig-style semantics); `!` is
> pure unary operation, not participating in short-circuit control flow, and is orthogonal to the
> `and`/`or` keywords (short-circuit) (authoritative definition in RFC-010).

> **Range binding strength**: `..` has binding strength (6, 7)—left 6 is lower than addition (7),
> right 7 swallows addition but not the same level `..`. Before and after the change:
>
> | Expression   | Before (level 1, right-associative)                                                             | After ((6,7), left-associative)                                                   |
> | ------------ | ----------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
> | `x in 1..10` | `x in 1..10` (`in` right operand level 4, `..` level 1 cannot swallow, unparseable in practice) | `x in (1..10)`—the interval is the right operand of `in`                          |
> | `0..n+2`     | `(0..n)+2` (right-associative trap: upper bound is eaten, `for` loop directly E3004)            | `0..(n+2)`—upper bound is an arithmetic expression                                |
> | `a == b..c`  | `a == (b..c)` (`..` level 1 < `==` level 3, naturally whole)                                    | `a == (b..c)`—**semantics unchanged**, `..` is still higher than comparison level |
> | `1..2*3`     | `(1..2)*3`                                                                                      | `1..(2*3)`—upper bound is an arithmetic expression                                |
> | `a..b..c`    | `a..(b..c)` (right-associative chaining, meaningless Range nesting)                             | `(a..b)..c`—**step form** (`c` is the step)                                       |
>
> Net effect: the composite upper bound `for i in 0..n+2` changes from "parses successfully but
> E3004" to "directly usable"; `x in 1..10` changes from "unparseable" to "interval check";
> `a..b..c` changes from "meaningless nesting" to "step component". Level 6 falls between `+`
> (level 5) and `<<` (level 7), mathematical convention: intervals are tightly-bound constructs, and
> the upper bound is naturally a complete arithmetic expression.

### 2.3 Function Calls

```
FnCall      ::= Expr '(' ArgList? ')'
ArgList     ::= Expr (',' Expr)* (',' NamedArg)* | NamedArg (',' NamedArg)*
NamedArg    ::= Identifier '=' Expr
```

Named arguments use `name = value` (RFC-010 §Function Definition, RFC-011 §Construction Forms).
Positional arguments must precede named arguments; the order of named arguments can be arbitrary:

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b

add(3, 5)          // Positional
add(a = 3, b = 5)  // Named
add(b = 5, a = 3)  // Arbitrary order
add(3, b = 5)      // Mixed, positional first
```

A wrong name in a named argument reports **E1014**, the same parameter being specified both
positionally and by name reports **E1015**, and a count mismatch reports **E1010** (RFC-013).

**Unnamed parameters and named arguments**: The parameter position in the signature allows a bare
type form (`mk: (Int, Int) -> Int`), which is an **unnamed typed parameter**—type constraints apply
positionally, and the name belongs to the implementation (lambda header has its own name, see
[RFC-007](../../rfc/accepted/007-function-syntax-unification.md) shorthand rules), not part of the
contract. Unnamed parameters cannot be referenced by named arguments (there is no name to reference)
and can only be passed positionally; the form with the name in the contract is
`mk: (a: Int, b: Int) -> Int`. A bare identifier must be a declared type; if the type cannot be
resolved, an error is reported (see [type-system §3.5](type-system.md#35-function-type)).

### 2.4 Member Access

```
MemberAccess::= Expr '.' Identifier
```

### 2.5 Index Access

```
IndexAccess ::= Expr '[' Expr ']'
```

> **Three-layer semantics (RFC-011b)**: `a[i]` is dispatched by the type of `a`—① built-in
> containers (List/Vec/Array/Dict/Tuple) go through the native indexing instruction (fast path); ②
> user types that implement the `Index` interface dispatch to their `index` method (instantiation of
> `Index(Grid, Int, Float)` + `Grid.index` method in the type body); ③ the `f[0]` positional binding
> in RFC-004 only exists in binding declarations and does not use this grammar. The keys of
> multi-dimensional indexing `a[0, 1]` are packaged as a tuple. Other types that don't implement
> `Index` are rejected at the type level.

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
> like `ok(v)`, `some(x)` resolved by the scrutinee) requires the scrutinee's type to be a **sum
> type whose variant set is in scope**. The variant set only enters the checker through type
> definitions or `use` imports—`Result`/`Option` are defined in `std.result`/`std.option`, and
> `use std.result` / `use std.option` must precede their use (whole module and grouped
> `use std.{...}` forms are equally valid); otherwise variant destructuring reports E1002.
> Exhaustiveness checking shares the same source: the catch-all arm exempts exhaustiveness; without
> a catch-all arm, all variants in the set are checked.

### 2.9 Block Expression

```
Block       ::= '{' Stmt* Expr? '}'
```

> **Statement termination rules**: The rules for separation between Stmts and line break behavior
> (explicit `;` separation, line break termination, line continuation exceptions, line-leading
> `(`/`[` never merging) are defined in [RFC-038](../../rfc/accepted/038-statement-termination.md).

#### 2.9.1 Three Forms of `{`

`{` has exactly three interpretations in expression position, decided at once by **content**:

| Form             | Notation          | Type                 | Example          |
| ---------------- | ----------------- | -------------------- | ---------------- |
| **Empty block**  | `{}`              | `Void`               | `x: Void = {}`   |
| **Dict literal** | `{ "k": v, ... }` | `Dict(K, V)`         | `d = { "a": 1 }` |
| **Block**        | `{ Stmt* Expr? }` | Tail expression type | `y = { 1 + 1 }`  |

**Determination order**:

1. `{}` (no content) → **empty block**, value `Void`
2. First element is `String ':' Expr` key-value pair → **dict literal**
3. Otherwise → **block**, value given by the tail expression

> **Why `{}` is not an empty dict**: The "emptiness" of an empty dict cannot be self-describing (it
> could be either `Dict(K, V)` or an empty block), while the dict grammar
> [§1.6.4](#_1-6-4-collection) requires at least one key-value pair. When there is no content to
> rely on, the zero form of the block structure is taken: this is consistent with `unsafe {}` /
> `spawn {}`, introducing no special cases. Use `dict.new()` for an empty dict.
>
> **Why functions need annotations**: `f = { stmt }` is a **value** (tail expression type), not a
> function. To define a function, specify the Fn annotation: `f: () -> Int = { 5 }`. This is the
> same principle as for dicts: **the type is determined by the content**, not by the presence of an
> annotation. (This rule is in [RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md)
> Appendix D.)

**Unified semantics**: The value of all `{}` blocks is given by the **tail expression**, and
`return` is a non-local exit of type `Never`.

| Block Type    | Value exit      | Empty block `{}` |
| ------------- | --------------- | ---------------- |
| Ordinary `{}` | Tail expression | `Void`           |
| `unsafe {}`   | Tail expression | `Void`           |
| `spawn {}`    | Tail expression | `Void`           |

**Core principles** (see [RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md) for
details):

- **Block value = tail expression** (the last expression), the only exit, with no exceptions
- When the last item is an **assignment statement**, the block value is `Void`; if you want `Void`,
  write `Void` explicitly
- **`return` exits the nearest function boundary** (penetrating all blocks, not "returning to the
  block"), type `Never`; `Never <: T`

> holds for any type (principle of explosion), so it can appear in any return type position

- The expression form `= expr` directly gives the value

```yaoxiang
// Ordinary {} block: tail expression gives the value
result = {
    x = compute()
    x                // Block value
}

// unsafe {} block: tail expression gives the type definition
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void
    }
    SqliteDb         // Block value
}

// spawn {} block: tail expression gives the result
(a, b) = spawn {
    result1 = fetch("url1"),
    result2 = fetch("url2")
    (result1, result2)   // Block value
}

// return: penetrates the block and exits the function
f: (n: Int) -> Int = {
    if n < 0 {
        return 0     // Penetrates if and the function body, exits the function
    }
    n * 2            // Tail expression
}
```

#### Is `name = { ... }` a Function or a Block Value?

`name = { ... }` could be either a function definition (RFC-007 "empty parameter simplest") or a
block value binding. The decision is **annotation first, function by default** (RFC-010a Appendix
D):

| Case                              | Result          | Example                            |
| --------------------------------- | --------------- | ---------------------------------- |
| Value is Lambda (`=>`)            | Function        | `f = () => 5` → `f()` = 5          |
| Annotation is a function type     | Function        | `f: () -> Int = { 5 }` → `f()` = 5 |
| Annotation is a non-function type | **Block value** | `x: Int = { 5 }` → `x` = 5         |
| No annotation                     | Function        | `f = { 5 }` → `f()` = 5            |

**Annotation is the type**: `x: Int = ...` declares `x` as `Int`, so `{ ... }` evaluates to `Int`;
`f: () -> Int = ...` declares `f` as a function, so `{ ... }` is the function body.

If you want `{ ... }` to be evaluated immediately, **just write the target type** (no new syntax
needed):

```yaoxiang
// Block value: immediate evaluation
x: Int = {
    y = 5
    y            // x = 5
}

// Function: no annotation, default
f = { 5 }        // f() = 5
```

#### Nested Function Types: Currying or Returning a Function?

The nested function type on the right side of `->` has two readings, distinguished by
**parentheses** (RFC-004):

| Notation                        | Meaning              | Call                    |
| ------------------------------- | -------------------- | ----------------------- |
| `(a: Int) -> (b: Int) -> Int`   | **Curried**          | `f(1)(2)`               |
| `(a: Int) -> ((b: Int) -> Int)` | **Returns function** | `g(1)` gives a function |

The basis: **Annotation is the type**. `g: (a: Int) -> ((b: Int) -> Int)` declares that
`g(1) : (b: Int) -> Int`, so `g(1)` must **be** that function, not "the next segment of arguments".

```yaoxiang
// Currying: two segments of arguments given layer by layer
add: (a: Int) -> (b: Int) -> Int = { a + b }
add(1)(2)        // → 3

// Returns function: outer layer one segment of arguments, the returned value is the function
adder: (n: Int) -> ((x: Int) -> Int) = (x) => x + n
adder(10)(5)     // → 15
h = adder(10)    // The returned function can be stored in a variable and passed
```

Unparenthesized nested `Fn` is always curried (including the type parameter form in RFC-011):

```yaoxiang
identity: (T: Type) -> (x: T) -> T = (x) => x   // Curried
identity(5)      // → 5
```

**Type checking**: The parentheses declare the return type, and the body must produce that type.
`f: () -> (() -> Int) = { 7 }` is an error—expects `() -> Int`, actually gets `Int` (E1002).

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

- Extract value `v` on `Ok(v)` and continue execution
- Propagate the error upward on `Err(e)` (`return Err(e)`)

```yaoxiang
process: (data: Data) -> Result(Data, Error) = {
    validated = validate(data)?     // Extract value on success, propagate on failure
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

// Range is a value: can be bound, passed, and tested for membership
r = 1..10
assert.assert(5 in r, "membership")
for i in r { print(i) }

// Step form (third component, default 1)
for i in 0..10..2 { print(i) }  // 0, 2, 4, 6, 8
for i in 10..0..(-2) { print(i) }  // 10, 8, 6, 4, 2
```

> **Step semantics**: In `a..b..c`, `c` is the step. A literal `c = 0` is rejected at compile time;
> a dynamic `c` is zero-checked at runtime (E6001 family; will be upgraded to Result after the error
> system is implemented). `c < 0` is valid, and the interval direction is reversed by the sign
> (`10..0..(-2)` decreases).

### 2.13 ref Expression

```
RefExpr     ::= 'ref' Expr
```

`ref` creates shared ownership. The compiler automatically selects Rc (single task) or Arc
(cross-task), and the user does not need to worry about implementation details.

```yaoxiang
data = ref heavy_data
spawn { use(data) }   // Cross-task: compiler automatically selects Arc
```

### 2.14 unsafe Expression

```
UnsafeExpr  ::= 'unsafe' Block
```

The `unsafe` block is used to define opaque types and operate on raw pointers. Use `return` to
return the type definition to the parent scope.

**Semantics**:

- Opaque types and raw pointer operations can be defined in `unsafe {}`
- The returned type is available outside the `unsafe {}` block
- Accessing the type's fields requires unsafe permission

```yaoxiang
// Define opaque type in unsafe block
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

- `x = value`: look up `x` outward along the scope chain, assign if found, otherwise declare new
- `mut x = value`: explicitly declares a new mutable binding, disallowing the same name as the outer
- Any name can only be declared once in the same scope

> **Detailed definition**: For the complete rules of scope, variable declaration, and shadowing
> mechanism, see [Module System Specification](modules.md#chapter-4-scope).

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

**Semantics**: `return` is a **non-local exit** that exits the nearest **function boundary**
(penetrating all blocks—including `if` / `while` / `for` / `match` / bare blocks / `spawn` /
`unsafe`), handing the value to the caller. It does **not "return to the block"**.

**Type**: `return e : Never` (`e : T`). `Never <: T'` holds for any `T'` (principle of explosion,
see [Type System §2.2](type-system.md)), so `return` can appear in any return type position without
additional constraints.

**Relationship with block evaluation**: The block's value is always the **tail expression** (see
§2.9). `{ return n }` as a block has value `n`, type `Never`; meanwhile, the effect of `return` is
to exit the function. **Both are true simultaneously**, coexisting by the principle of explosion.

`return` and the tail expression together make "early return" work, with no need for additional
rules specifying `return` to the function—see
[RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md).

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

**Semantics**: Immediately terminates the innermost enclosing `while`/`for` loop, transferring
control to after the loop body.

- **Exits only the innermost level**: `break`

> always acts on the innermost loop that contains it. In nested loops, when needing to exit multiple
> levels at once, extract the inner loop as a function and use `return` to return, or use a flag
> (break/continue have no labels; if loop labels are introduced in the future, they will follow the
> loop declaration-side syntax through the RFC process, decided together with the multi-exit design
> of the proof pipeline)

- **Only within loop bodies**: `break` can only appear within `while`/`for` loop bodies (including
  blocks/if/match nested within), and appearing outside a loop is a compile error (E1102
  `'break' outside of a loop`)
- **Does not affect termination proofs**: `break`

> does not participate in termination arguments—it neither provides a measure nor constitutes a
> decreasing step in a measure; the loop's termination obligation is independent of `break` and is
> triggered by refined types (see
> [type-system §8.4](type-system.md#84-terminates-termination-measure-predicate)).

- **Borrowing semantics**: The control flow edge of break participates in the structural cut of the
  reverse BFS liveness analysis of RFC-009a (the skipped iteration does not participate in the
  back-edge liveness derivation)

```yaoxiang
mut i = 0
while i < 10 {
    i = i + 1
    if i == 3 {
        break              // Control flows to after the loop, i == 3
    }
}

// Nested loops: break only exits the inner level
while j < 3 {
    while k < 10 {
        if k == 2 { break }    // Only terminates the inner loop
    }
    j = j + 1                  // Every iteration of the outer loop reaches here
}
```

### 3.5 continue Statement

```
ContinueStmt::= 'continue'
```

**Semantics**: Skips the remaining statements in the current iteration and directly proceeds to the
next round of the innermost enclosing loop—`while` returns to re-evaluate the condition, `for` takes
the next element.

- **Only acts on the innermost level**: Same as `break`, without labels
- **Only within loop bodies**: Appearing outside a loop is a compile error (E1102)

```yaoxiang
mut sum = 0
mut n = 0
while n < 5 {
    n = n + 1
    if n == 3 {
        continue           // Skips the accumulation below, n == 3 is not counted
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

YaoXiang's for loop semantics differ from traditional languages: **each iteration binds a new value,
rather than modifying the same variable**.

```yaoxiang
// Example: for i in 1..5
for i in 1..5 {
    print(i)
}
```

**Execution process**:

| Iteration | Behavior of the loop variable                                                          |
| --------- | -------------------------------------------------------------------------------------- |
| 1st       | Creates new binding `i = 1`, loop body executes, prints 1                              |
| 2nd       | Creates new binding `i = 2` (previous binding destroyed), loop body executes, prints 2 |
| 3rd       | Creates new binding `i = 3`, loop body executes, prints 3                              |
| 4th       | Creates new binding `i = 4`, loop body executes, prints 4                              |
| End       | Loop body ends, binding destroyed                                                      |

**Key point**: After each iteration ends, the binding created in that iteration is destroyed. The
next iteration is an entirely new binding, with no relation to the binding of the previous
iteration.

#### 3.9.2 Difference Between for and for mut

| Syntax              | Loop variable mutability | Description                        |
| ------------------- | ------------------------ | ---------------------------------- |
| `for i in 1..5`     | Immutable                | Cannot modify binding in loop body |
| `for mut i in 1..5` | Mutable                  | Can modify binding in loop body    |

```yaoxiang
// Valid: each iteration binds a new value, no need to modify
for i in 1..5 {
    print(i)  // Read value of i
}

// Error: immutable binding, cannot modify
for i in 1..5 {
    i = i + 1  // Error: cannot modify immutable binding
}

// Valid: use for mut to allow modifying the binding
for mut i in 1..5 {
    i = i + 1  // Modification allowed
}
```

#### 3.9.3 Shadowing Check

YaoXiang prohibits variable shadowing. The for loop variable cannot have the same name as a variable
in an outer scope:

```yaoxiang
// Error: i is already declared externally
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

This rule applies to all code blocks, see [4.3 Shadowing Rules](modules.md#43-shadowing-rules).

#### 3.9.4 Comparison with Other Languages

| Language | for loop variable semantics                             |
| -------- | ------------------------------------------------------- |
| YaoXiang | Each iteration binds a new value                        |
| Rust     | Modifies the same variable (needs mut)                  |
| Python   | Modifies the same variable (no mut needed)              |
| C/C++    | Modifies the same variable (needs pointer or reference) |

**Design rationale**: YaoXiang uses binding semantics because:

1. **More aligned with natural semantics** In natural language, "for each element x in the
   collection" means each x is an independent individual. YaoXiang's `for i in 1..5` reads as "for
   each i from 1 to 5", and the i in each iteration is an entirely new binding, which is consistent
   with human intuition.

2. **Avoid accidental modification** The default immutable binding semantics means the loop variable
   cannot be accidentally modified in the loop body. No need to worry about writing `i = ...`
   somewhere in a complex loop body, leading to hard-to-trace bugs.

3. **High-performance solutions within reach** When there is a real need to reuse variables between
   iterations (e.g., accumulators, caches), use `for mut` to switch to the mutable binding mode.
   This is clearer than implicit shared state—intent is explicitly expressed through syntax, not
   hidden in runtime behavior.

### 3.10 spawn Statement

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' 'mut'? Identifier 'in' Expr '{' Expr '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
```

**spawn block**: Explicitly declares a concurrent region, with expressions in the block executing
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

- The block body references outer variables = **Move value capture**: The value is snapshotted into
  the closure environment at the spawn creation point, and the block body reads it via env
  (LoadUpvalue)
- **Primitives** (Int/Float/Bool/Char) are copied by value, outer variables are unaffected
- **Handle types** (Struct/String/List etc.) snapshot = handle copy, sharing the underlying object;
  in the Embedded runtime (default) with the same thread and same heap, handles are valid
- Sharing between multiple tasks requires explicit `ref` (§2.13, compiler automatically selects
  Rc/Arc)
- Outer variables referenced by `return` within the block are captured in the same way

```yaoxiang
t1 = 1 + 1
t2 = 2 + 2
result = spawn {
    return t1 + t2    // t1/t2 value capture, result == 6
}
```

---

### 3.11 Program Entry and Top-level Statements

A source file has two roles in the compiler's view, **determined by the presence of
`yaoxiang.toml`**:

| Role       | Determination                              | Program body                                                                        |
| ---------- | ------------------------------------------ | ----------------------------------------------------------------------------------- |
| **Script** | Single-file direct run, no `yaoxiang.toml` | **Top-level statements** (executed in written order); `main` is an ordinary binding |
| **Bin**    | `yaoxiang.toml` exists                     | **`main` function**; top-level cannot have executable statements                    |

#### Script: Top-level Statements Are the Program

Without a manifest, the file is a "script", and **top-level statements are executed in written
order**:

```yaoxiang
use std.io
io.println("hello")          // Execute directly
x: Int = { 42 }              // Top-level binding: runtime initialization
io.println(x)                // 42
```

`main` is **not special** in this mode—it is just a normal binding. To make `main` run, you must
call it explicitly:

```yaoxiang
use std.io
main: () -> Void = { io.println("only runs if called") }

main()                       // ← Must write this line
```

> **Why is `main` not called automatically?** Top-level statements are already the program body. If
> `main` were implicitly called, a script that explicitly writes `main()` would execute twice. The
> two rules cannot coexist, so in Script mode there is only "top-level statements" as the single
> execution entry point.

#### Bin: `main` is the Entry Point

When there is a manifest, the file is an "executable target", in which case:

- A **binding** named `main` must be defined; either a value or a function is acceptable (#388
  decision: entry is determined by binding existence—a function is already a value, the two differ
  only in evaluation strategy): the function main is called with zero parameters at entry time; the
  value main is evaluated at initialization time, which executes it
- Top-level cannot have executable statements—the program body is `main`

```yaoxiang
main: () -> Void = {
    print("hello")
}
```

A value main is also legal—it executes at initialization time, which happens in the initialization
sequence (according to the global binding topological order, independent bindings in source order):

```yaoxiang
main = {
    print("hello")
}
```

Missing `main` is a compile error (without `main` all functions are unreachable). A non-callable
value main (e.g., `main: Int = 5`) is legal but has no observable effect—comparable to Rust's empty
`fn main() {}`.

> **Library files**: Files that are `use`d by other files, or files pointed to by `[lib].path` /
> `[exports]`, do not require `main`—they are not program entry points.

#### Initialization of Top-level Bindings

The initialization value of a top-level binding **is evaluated at runtime** and is not required to
be a compile-time constant:

```yaoxiang
answer: Int = { 42 }              // Block value
inc: (Int) -> Int = (x) => x + 1
computed: Int = inc(41)           // Function call
```

Initialization executes in **dependency order**, independent of written order:

```yaoxiang
derived: Int = base * 3           // References later-declared base
base: Int = 7                     // Initialized first (topological sort)
```

Circular dependencies are compile errors (the names in the cycle are listed):

```yaoxiang
a: Int = b + 1
b: Int = a + 1                    // Error: a → b → a
```

> **Design basis**: RFC-029f (file role model), RFC-010a Appendix D (block binding decision).

---

## Appendix: Syntax Quick Reference

### A.0 Keywords (18)

```
pub     use     spawn  ref     mut
if      else    match  while   for
in      return  break  continue
as      unsafe  and    or
```

`type` is no longer a keyword (RFC-010, use `Name: Type = { ... }` instead); `pub` has no visibility
effect (RFC-029). Literal reserved words `true` / `false` / `void` see §1.4.1, meta type name `Type`
and built-in type names `Void` / `Never` / `Int` / `Float` / `Bool` / `Char` / `String` see §1.4.3.

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
