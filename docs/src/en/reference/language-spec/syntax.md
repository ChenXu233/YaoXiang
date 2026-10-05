# Syntax Specification

This document defines the syntax specification of the YaoXiang programming language, including
lexical structure, grammar rules, and operator precedence.

---

## Chapter 1: Lexical Structure

### 1.1 Source Files

YaoXiang source files must use UTF-8 encoding. Source files typically use the `.yx` extension.

### 1.2 Lexical Token Categories

| Category   | Description                  | Examples                  |
| ---------- | ---------------------------- | ------------------------- |
| Identifier | Starts with a letter or `_`  | `x`, `_private`, `my_var` |
| Keyword    | Language-predefined reserved | `use`, `mut`, `and`       |
| Literal    | Fixed values                 | `42`, `"hello"`, `true`   |
| Operator   | Operation symbols            | `+`, `-`, `*`, `/`        |
| Delimiter  | Syntax separators            | `(`, `)`, `{`, `}`, `,`   |

### 1.3 Keywords

YaoXiang has **18 keywords** (in `keyword_from_str` at `src/frontend/core/lexer/state.rs:25-55`,
each mapping one-to-one to a `TokenKind`):

```
pub     use     spawn  ref     mut
if      else    match  while   for
in      return  break  continue
as      unsafe  and    or
```

`and` / `or` are the **keywords** for logical AND / OR (Zig-style, precedence level 10; see §2.2);
unary NOT is the symbol `!`, which is orthogonal to them.

These keywords have special meaning in any context and cannot be used as identifiers.

> **`type` is no longer a keyword** (RFC-010): to write a type definition, use the
> `Name: Type = { ... }` notation. `src/frontend/core/lexer/state.rs:27` explicitly notes this, and
> the `TokenKind` enum header (`src/frontend/core/lexer/tokens.rs:82`) also says "16 total -
> RFC-010: 'type' keyword removed"—**that "16" is a stale comment**; in fact 18 are now listed
> (including `and` / `or`, while the `Kw*`-prefixed batch is still 16).
>
> **`pub` produces no visibility effect**: `pub` is still lexed as `KwPub`
> (`src/frontend/core/lexer/state.rs:28`), and the parser skips it at declarations and import items
> (`src/frontend/core/parser/statements/declarations.rs:666-671,726`, `.../imports.rs:57-59`), but
> the module system **does not make any visibility judgment based on it**— the accepted
> [RFC-029](../../rfc/accepted/029-module-semantics.md) explicitly states "no `pub`, no
> `private`, no `export`, no visibility mechanism" (line 17 of that file). Writing or omitting `pub`
> makes no difference to visibility.

### 1.4 Reserved Words

YaoXiang's "reserved words" are divided into three layers, identified by the parser and the type
checker at different stages:

#### 1.4.1 Literal Reserved Words

These are literal identifiers that have their own tokens in the parser and cannot be used as
ordinary identifiers:

| Identifier | Type | Description                                                                                                   |
| ---------- | ---- | ------------------------------------------------------------------------------------------------------------- |
| `true`     | Bool | Boolean true                                                                                                  |
| `false`    | Bool | Boolean false                                                                                                 |
| `void`     | Void | Void literal (Unit value). Lowercase `void` is a value literal; uppercase `Void` is a type name (see §1.4.3). |

> **`Type` is not in this layer**: it does **not** have its own `TokenKind` (it is absent from the
> `TokenKind` enum in `src/frontend/core/lexer/tokens.rs`, and `keyword_from_str` has no
> corresponding branch); the parser treats it as an ordinary identifier, and the type checker
> recognizes it as a meta-type in type position. Therefore, in expression position, `Type` can be
> shadowed by a local binding. It is a **meta-type name**, not a keyword.

#### 1.4.2 Variant Names

Variant names are recognized by the parser in **pattern** context (a bare name is resolved by the
variant set of the scrutinee's type; see §2.8); in **expression** context, constructing a variant
requires a type qualification—`Result(Int, String).ok(5)`, `Option(T).some(v)`, `Color.green()`
(RFC-010 record syntax and types: variant constructors are function-typed fields in the type
definition, and a bare name like `ok(5)` is not a constructor call).

| Variant   | Type   | Description                      |
| --------- | ------ | -------------------------------- |
| `some(T)` | Option | Option value-variant constructor |
| `ok(T)`   | Result | Result success variant           |
| `err(E)`  | Result | Result error variant             |

#### 1.4.3 Built-in Type Names

The following type names are pre-registered by the type checker and can be used in type position
without imports. The parser treats them as ordinary identifiers—**they are not reserved words and
can be shadowed by local bindings (not recommended)**.

| Type name | Logical mapping   | Description                                                                                                                                        |
| --------- | ----------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Void`    | ⊤ (true / Unit)   | A zero-field product type with exactly one inhabitant (the `void` literal, see §1.4.1)                                                             |
| `Never`   | ⊥ (false / empty) | A zero-variant sum type with zero inhabitants. No expression can produce a `Never` value. `Never <: T` holds for all `T` (principle of explosion). |
| `Int`     | —                 | Signed integer                                                                                                                                     |
| `Float`   | —                 | Floating-point number                                                                                                                              |
| `Bool`    | —                 | Boolean value: `true` / `false`                                                                                                                    |
| `Char`    | —                 | Unicode character                                                                                                                                  |
| `String`  | —                 | String                                                                                                                                             |

### 1.5 Identifiers

Identifiers start with a letter or underscore, followed by letters, digits, or underscores.
Identifiers are case-sensitive.

Special identifiers:

- `_` is used as a placeholder, meaning an ignored value
- Identifiers starting with an underscore denote private members

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
Array       ::= '[' Expr (',' Expr)* ']'   // When the target type is annotated as Array(T, N), the literal becomes a fixed-length array
```

> **Dictionary literals require at least one key-value pair**: `{}` is **not** an empty
> dictionary—it is an empty block (value `Void`; see [§2.9](#_2-9-block-expressions)). For an empty
> dictionary, use the constructor `dict.new()`:
>
> ```yaoxiang
> empty = dict.new()                  // ✅ empty dictionary
> d = { "a": 1 }                       // ✅ dictionary literal
> wrong = {}                           // ❌ this is not a dictionary; it is an empty block (Void)
> ```
>
> **The criterion is the content**: the `Dict` grammar requires at least one `String ':' Expr`;
> since `{}` has no content to rely on, the zero form of block structure is taken. The non-empty
> form is self-describing by content (`{ "k": v }` has a key-value pair → dictionary)—this is the
> same principle as `f = { 5 }` being an `Int` value rather than a function: **the type is
> determined by the content**.

> Set has no literal grammar and no runtime representation—collection types are under planning and
> will be completed in the Dict style when needs arise (std.set + HeapValue::Set). The landing point
> of List/Dict literals is determined by the contextual type annotation: a bare literal, or one
> annotated as `List(T)`, lands as a growable list; an `Array(T, N)` annotation applied directly to
> a literal lands as a fixed-length array. Implicit List→Array conversion is forbidden.
>
> Array literal semantics:
>
> - The number of elements must equal N; otherwise compile-time E1002; an empty literal paired with
>   a non-zero N is also rejected
> - Every element's type must be compatible with T; mismatch is compile-time E1002
> - The grammatical form of N: only an integer literal (which may be negative) or a constant name;
>   compound expressions (e.g. `2+1`) are rejected at parse time
> - When N is a symbolic constant (e.g. a `const` parameter of a function such as `Array(Int, n)`),
>   the element-count check is deferred to the refinement-typing phase
> - Nested array literals at v1 (e.g. `Array(Array(Int,2),2) = [[1,2],[3,4]]`) are rejected at
>   compile time and must be constructed explicitly layer by layer; recursive landing points are
>   deferred to a later version

#### 1.6.5 List Comprehensions

```
ListComp    ::= '[' Expr ( 'for' Identifier 'in' Expr ( 'if' Expr )? )+ ']'
```

> **#401 (implementation completion)**: `if` filters and multi-generator clauses land starting from
> this version—previously the grammar was written as `(',' Expr)* ('if' Expr)?`, which had no
> corresponding implementation (neither trailing comma-expressions nor multiple generators are
> supported), and the `if` filter was entirely missed by the parser (after parsing the iterable, it
> would directly expect `']'`, reporting `E0010 Expected RBracket, found KwIf`). The convention now
> follows industry-common semantics: one or more generator clauses, expanded as nested loops; each
> clause may carry at most one `if` filter, and the condition is forced to `Bool` (non-Bool reports
> `E1054`); later clauses' iterables/conditions may reference iteration variables bound by earlier
> clauses.
>
> **Behavior tightening (migration note)**: the grammatical form of iteration variables has always
> been `'for' Identifier 'in'`, but in the old implementation the pattern went through full Pratt
> parsing—after `'in'` was registered as an infix operator, `x` would swallow `in items` into a
> membership expression. After the fix, non-identifier patterns fail to parse directly and no longer
> fall back to `_` as the old implementation did (silently swallowing the error). Impact: code such
> as `[x for (a, b) in pairs]` that previously parsed will now report an error—that form never had
> defined behavior (the variable was always `_`), and the tightening direction is correct with no
> semantic migration cost.

#### 1.6.6 Membership Test

```
Membership  ::= Expr 'in' Expr
```

> `in` is a binary relational operator that returns `Bool`—`true` on hit, `false` on miss, no error.
> Semantic split: `[]` asserts existence and retrieves a value (fails with an error), while `in`
> asks whether something exists (a miss is a normal `false`). Right-operand coverage: List / Array /
> Dict (key set) / Tuple / String (substring) / Range (interval). `in` is a first-class Hoare
> predicate, serving as the basis of compile-time provable propositions in the refinement-typing
> phase. (Set is removed from the right-operand list—Set has no runtime representation; see §1.6.4.)

### 1.7 Comments

```
// single-line comment

/* multi-line comment
   can span multiple lines */
```

### 1.8 Indentation Rules

Code must use 4-space indentation; Tab characters are forbidden. This is a mandatory syntax rule.

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

> **Unary prefix operators** (`!` `-` `+`) bind tightly: only weaker than call and member access,
> and stronger than all binary operators. Therefore `!a == b` is equivalent to `(!a) == b`
> (Zig-style semantics); `!` is purely a unary operator and does not participate in short-circuit
> control flow, and is orthogonal to the `and` / `or` keywords (which are short-circuit)
> (authoritative definition in RFC-010).

> **Range binding strength**: `..` binds at (6, 7)—left-binding 6 is weaker than addition (7),
> right-binding 7 swallows addition but not same-level `..`. Comparison before and after the change:
>
> | Expression   | Before (level 1, right-associative)                                                                         | After ((6,7), left-associative)                                                 |
> | ------------ | ----------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
> | `x in 1..10` | `x in 1..10` (right operand of `in` is level 4, `..` at level 1 cannot swallow it; effectively unparseable) | `x in (1..10)`—the interval as a whole is the right operand of `in`             |
> | `0..n+2`     | `(0..n)+2` (right-associativity trap: the upper bound is eaten, `for` loops directly hit E3004)             | `0..(n+2)`—the upper bound is an arithmetic expression                          |
> | `a == b..c`  | `a == (b..c)` (level 1 `..` < level 3 `==`, naturally a whole)                                              | `a == (b..c)`—**semantics unchanged**, `..` still binds higher than comparisons |
> | `1..2*3`     | `(1..2)*3`                                                                                                  | `1..(2*3)`—the upper bound is an arithmetic expression                          |
> | `a..b..c`    | `a..(b..c)` (right-associative chain; meaningless Range-in-Range)                                           | `(a..b)..c`—**step form** (`c` is the step)                                     |
>
> Net effect: a compound upper bound such as `for i in 0..n+2` changes from "parses but reports
> E3004" to "directly usable"; `x in 1..10` changes from "unparseable" to "interval check";
> `a..b..c` changes from "meaningless nesting" to "step component". Level 6 falls between `+`
> (level 5) and `<<` (level 7), in line with mathematical convention: an interval is a tightly
> binding construct, so the upper bound is naturally a complete arithmetic expression.

### 2.3 Function Calls

```
FnCall      ::= Expr '(' ArgList? ')'
ArgList     ::= Expr (',' Expr)* (',' NamedArg)* | NamedArg (',' NamedArg)*
NamedArg    ::= Identifier '=' Expr
```

Named arguments use `name = value` (RFC-010 §Function Definitions, RFC-011 §Construction Form).
Positional arguments must come before named arguments; the order of parameters specified
positionally can be arranged freely among the named arguments:

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b

add(3, 5)          // positional
add(a = 3, b = 5)  // named
add(b = 5, a = 3)  // arbitrary order
add(3, b = 5)      // mixed, positional first
```

Writing a wrong name for a named argument reports **E1014**, specifying the same parameter both
positionally and by name reports **E1015**, and a count mismatch reports **E1010** (RFC-013).

### 2.4 Member Access

```
MemberAccess::= Expr '.' Identifier
```

### 2.5 Index Access

```
IndexAccess ::= Expr '[' Expr ']'
```

> **Three layers of semantics (RFC-011b)**: `a[i]` branches on the type of `a`—① built-in containers
> (List/Vec/Array/Dict/Tuple) go through native indexing instructions (fast path); ② user types that
> implement the `Index` interface dispatch to their `index` method (an `Index(Grid, Int, Float)`
> instance inside the type body, plus a `Grid.index` method); ③ the `f[0]` positional binding from
> RFC-004 exists only in binding declarations and does not flow through this grammar. The keys of
> multi-dimensional indexing `a[0, 1]` are packed as a tuple. Other types that do not implement
> `Index` are rejected at the type layer.

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

> **Variant-set requirement for variant deconstruction (RFC-010b)**: an `EnumPattern` (variant names
> like `ok(v)`, `some(x)` resolved by the scrutinee) requires the scrutinee's type to be a **sum
> type whose variant set is in scope**. The variant set only enters the checker through a type
> definition or a `use` import—`Result` / `Option` are defined by `std.result` / `std.option`, and
> `use std.result` / `use std.option` (the full module form and the grouped `use std.{...}` form are
> equally valid) is required before use; otherwise variant deconstruction reports E1002.
> Exhaustiveness checking has the same origin: the wildcard arm is exempt from exhaustiveness, and
> without a wildcard arm the check is performed against the full variant set.

### 2.9 Block Expression

```
Block       ::= '{' Stmt* Expr? '}'
```

> **Statement termination rules**: the rules for separator and line-break behavior between Stmts
> (explicit `;` separation, line-break termination, line-continuation exceptions, leading `(` / `[`
> that never merge) are defined by
> [RFC-038](../../rfc/accepted/038-statement-termination.md).

#### 2.9.1 The Three Forms of `{`

In expression position, `{` has exactly three interpretations, determined **at once by the
content**:

| Form             | Notation          | Type                 | Example          |
| ---------------- | ----------------- | -------------------- | ---------------- |
| **Empty block**  | `{}`              | `Void`               | `x: Void = {}`   |
| **Dict literal** | `{ "k": v, ... }` | `Dict(K, V)`         | `d = { "a": 1 }` |
| **Block**        | `{ Stmt* Expr? }` | Tail-expression type | `y = { 1 + 1 }`  |

**Resolution order**:

1. `{}` (no content) → **empty block**, value `Void`
2. First element is a `String ':' Expr` key-value pair → **dict literal**
3. Otherwise → **block**, value given by the tail expression

> **Why `{}` is not an empty dictionary**: the "emptiness" of an empty dictionary cannot be
> self-describing (it could be either `Dict(K, V)` or an empty block), and the dict grammar
> ([§1.6.4](#_1-6-4-collections)) requires at least one key-value pair. When there is no content to
> rely on, the zero form of the block structure is taken—this is consistent with `unsafe {}` /
> `spawn {}`, and introduces no special case. For an empty dictionary, use `dict.new()`.
>
> **Why functions need an annotation**: `f = { stmt }` is a **value** (the type of the tail
> expression), not a function. To define a function, write an Fn annotation: `f: () -> Int = { 5 }`.
> This is the same principle as the dictionary case: **the type is determined by the content**, not
> by whether an annotation exists. (This rule is in
> [RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md) Appendix D.)

**Unified semantics**: the value of every `{}` block is given by the **tail expression**; `return`
is a non-local exit of type `Never`.

| Block type  | Value exit | Empty block `{}` |
| ----------- | ---------- | ---------------- |
| Plain `{}`  | Tail expr  | `Void`           |
| `unsafe {}` | Tail expr  | `Void`           |
| `spawn {}`  | Tail expr  | `Void`           |

**Core principles** (see [RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md)
for details):

- **A block's value = its tail expression** (the last expression); the sole exit, with no exceptions
- When the final position is an **assignment statement**, the block's value is `Void`; if you want
  `Void`, write `Void` explicitly
- **`return` exits the nearest function boundary** (piercing through every block, not "returning to
  the block"); its type is `Never`; `Never <: T` holds for any type (principle of explosion), so it
  can appear at any return-type position
- The expression form `= expr` directly gives a value

```yaoxiang
// plain {} block: the tail expression gives the value
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

// return: pierces through the block, exiting the function
f: (n: Int) -> Int = {
    if n < 0 {
        return 0     // exits the if and the function body, leaving the function
    }
    n * 2            // tail expression
}
```

#### Is `name = { ... }` a Function or a Block Value?

`name = { ... }` could be either a function definition (RFC-007 "no-parameter simplest form") or a
block-value binding. The decision follows the **annotation-first, default-to-function** rule
(RFC-010a Appendix D):

| Situation                         | Result          | Example                            |
| --------------------------------- | --------------- | ---------------------------------- |
| Value is a Lambda (`=>`)          | Function        | `f = () => 5` → `f()` = 5          |
| Annotation is a function type     | Function        | `f: () -> Int = { 5 }` → `f()` = 5 |
| Annotation is a non-function type | **Block value** | `x: Int = { 5 }` → `x` = 5         |
| No annotation                     | Function        | `f = { 5 }` → `f()` = 5            |

**Annotation is the type**: `x: Int = ...` declares that `x` is `Int`, so `{ ... }` evaluates to
`Int`; `f: () -> Int = ...` declares that `f` is a function, so `{ ... }` is a function body.

To make `{ ... }` evaluate immediately, **just write the target type** (no new syntax needed):

```yaoxiang
// block value: evaluate immediately
x: Int = {
    y = 5
    y            // x = 5
}

// function: default with no annotation
f = { 5 }        // f() = 5
```

#### Nested Function Types: Currying or Returning a Function?

A nested function type to the right of `->` has two readings, distinguished by **parentheses**
(RFC-004):

| Notation                        | Meaning                | Call                      |
| ------------------------------- | ---------------------- | ------------------------- |
| `(a: Int) -> (b: Int) -> Int`   | **Curried**            | `f(1)(2)`                 |
| `(a: Int) -> ((b: Int) -> Int)` | **Returns a function** | `g(1)` returns a function |

The basis: **annotation is the type**. `g: (a: Int) -> ((b: Int) -> Int)` declares that
`g(1) : (b: Int) -> Int`, so `g(1)` must **be** that function, not the "next argument segment".

```yaoxiang
// currying: two argument segments given layer by layer
add: (a: Int) -> (b: Int) -> Int = { a + b }
add(1)(2)        // → 3

// returning a function: outer layer is one argument, what is returned is the function
adder: (n: Int) -> ((x: Int) -> Int) = (x) => x + n
adder(10)(5)     // → 15
h = adder(10)    // the returned function can be stored in a variable and passed around
```

An unparenthesized nested `Fn` is always curried (including the type-parameter form in RFC-011):

```yaoxiang
identity: (T: Type) -> (x: T) -> T = (x) => x   // curried
identity(5)      // → 5
```

**Type checking**: parentheses declare the return type, and the body must produce that type.
`f: () -> (() -> Int) = { 7 }` is wrong—it expects `() -> Int`, but actually gets `Int` (E1002).

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

- On `Ok(v)`, extract the value `v` and continue execution
- On `Err(e)`, propagate the error upward (`return Err(e)`)

```yaoxiang
process: (data: Data) -> Result(Data, Error) = {
    validated = validate(data)?     // extract value on success, propagate on failure
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

// step form (third component, defaults to 1)
for i in 0..10..2 { print(i) }  // 0, 2, 4, 6, 8
for i in 10..0..(-2) { print(i) }  // 10, 8, 6, 4, 2
```

> **Step semantics**: in `a..b..c`, `c` is the step. A literal `c = 0` is rejected at compile time;
> a dynamic `c = 0` raises a runtime zero-check (E6001 family; will be promoted to a Result once the
> error system lands). `c < 0` is valid, and the direction of the interval reverses with the sign
> (`10..0..(-2)` decreases).

### 2.13 ref Expression

```
RefExpr     ::= 'ref' Expr
```

`ref` creates shared ownership. The compiler automatically chooses Rc (single-task) or Arc
(cross-task); the user does not need to care about the implementation details.

```yaoxiang
data = ref heavy_data
spawn { use(data) }   // cross-task: the compiler automatically chooses Arc
```

### 2.14 unsafe Expression

```
UnsafeExpr  ::= 'unsafe' Block
```

The `unsafe` block is used to define opaque types and operate on raw pointers. Use `return` to
return a type definition to the enclosing scope.

**Semantics**:

- Types and raw-pointer operations are allowed inside `unsafe {}`
- The returned type is usable outside `unsafe {}`
- Accessing fields of such types requires the unsafe permission

```yaoxiang
// define an opaque type inside an unsafe block
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
- Inner scopes can access variables in outer scopes
- Outer scopes cannot access variables in inner scopes
- Variable declarations follow the "assignment-first" principle

```yaoxiang
// block scope
{
    x = 10
    // x is visible within this scope
}
// x is not visible outside this scope

// function scope
add: (a: Int, b: Int) -> Int = {
    result = a + b
    return result
}
// result is not visible outside the function
```

**Variable declaration and shadowing**:

- `x = value`: search outward along the scope chain for `x`; if found, assign, otherwise declare a
  new one
- `mut x = value`: explicitly declare a new mutable binding; disallows the same name as the outer
  scope
- Within the same scope, any name can be declared at most once

> **Detailed definition**: the complete rules of scoping, variable declaration, and shadowing
> mechanism are detailed in [Module System Specification](modules.md#chapter-4-scope).

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
(piercing through every block—including `if` / `while` / `for` / `match` / bare blocks / `spawn` /
`unsafe`) and hands the value to the caller. It does **not "return to a block"**.

**Type**: `return e : Never` (where `e : T`). `Never <: T'` holds for any `T'` (principle of
explosion; see [Type System §2.2](type-system.md)), so `return` can appear at any return-type
position without needing extra rules to constrain it.

**Relationship with block evaluation**: the value of a block is always the **tail expression** (see
§2.9). `{ return n }` as a block has value `n`, of type `Never`; meanwhile, the effect of `return`
is to exit the function. **Both are true at the same time**, coexisting by the principle of
explosion.

`return` together with the tail expression makes "early return" work, with no extra rule that
`return` specifically refers to a function—see
[RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md).

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 {
        return 1          // exits the if, leaves the function (type Never)
    }
    n * factorial(n - 1)  // tail expression = block's value
}
```

### 3.4 break Statement

```
BreakStmt   ::= 'break'
```

**Semantics**: immediately terminates the innermost enclosing `while` / `for` loop, and control
flows to after the loop body.

- **Only exits the nearest layer**: `break` always applies to the innermost loop that contains it.
  To break out of multiple layers in nested loops, extract the inner loop into a function and use
  `return`, or use a flag (break/continue carry no label; if a loop label is introduced in the
  future, it will follow the loop-declaration-side syntax through the RFC process, and be decided
  together with the multi-exit design of the proof pipeline).
- **Only valid inside a loop body**: `break` may only appear inside a `while` / `for` loop body
  (including blocks / `if` / `match` nested within the body); using it outside a loop is a compile
  error (E1102 `'break' outside of a loop`).
- **Does not affect termination proofs**: `break` does not participate in termination arguments—it
  neither provides a measure nor constitutes a decreasing step of a measure; the termination
  obligation of a loop is independent of `break` and is triggered by refinement types (see
  [type-system §8.4](type-system.md#84-terminates-termination-measure-predicate)).
- **Borrowing semantics**: the control-flow edge of `break` participates in the structural cut of
  the RFC-009a reverse-BFS liveness analysis (iterations that are jumped out of do not participate
  in the liveness derivation of back-edges).

```yaoxiang
mut i = 0
while i < 10 {
    i = i + 1
    if i == 3 {
        break              // control flows to after the loop, i == 3
    }
}

// nested loops: break only exits the inner loop
while j < 3 {
    while k < 10 {
        if k == 2 { break }    // only terminates the inner loop
    }
    j = j + 1                  // every outer iteration runs to here
}
```

### 3.5 continue Statement

```
ContinueStmt::= 'continue'
```

**Semantics**: skips the remaining statements in the current iteration, going directly to the next
iteration of the innermost enclosing loop—for `while`, it goes back to re-evaluating the condition;
for `for`, it takes the next element.

- **Only acts on the nearest layer**: same as `break`; carries no label
- **Only valid inside a loop body**: using it outside a loop is a compile error (E1102)

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

#### 3.9.1 Semantics: Each Iteration Is a New Binding

The semantics of YaoXiang's `for` loop differ from traditional languages: **each iteration is a new
binding, rather than modifying the same variable**.

```yaoxiang
// example: for i in 1..5
for i in 1..5 {
    print(i)
}
```

**Execution process**:

| Iteration | Behavior of the loop variable                                                           |
| --------- | --------------------------------------------------------------------------------------- |
| 1st       | Create a new binding `i = 1`, run the body, print 1                                     |
| 2nd       | Create a new binding `i = 2` (the previous binding is destroyed), run the body, print 2 |
| 3rd       | Create a new binding `i = 3`, run the body, print 3                                     |
| 4th       | Create a new binding `i = 4`, run the body, print 4                                     |
| End       | Body ends, binding is destroyed                                                         |

**Key point**: after each iteration, the binding created by that iteration is destroyed. The next
iteration is a brand-new binding, with no relation to the previous iteration's binding.

#### 3.9.2 Difference Between `for` and `for mut`

| Syntax              | Loop variable mutability | Description                               |
| ------------------- | ------------------------ | ----------------------------------------- |
| `for i in 1..5`     | Immutable                | Cannot modify the binding inside the body |
| `for mut i in 1..5` | Mutable                  | May modify the binding inside the body    |

```yaoxiang
// legal: each iteration binds a new value, no modification needed
for i in 1..5 {
    print(i)  // read the value of i
}

// error: immutable binding, cannot be modified
for i in 1..5 {
    i = i + 1  // error: cannot modify an immutable binding
}

// legal: use `for mut` to allow modifying the binding
for mut i in 1..5 {
    i = i + 1  // modification allowed
}
```

#### 3.9.3 Shadowing Check

YaoXiang prohibits variable shadowing. A `for` loop variable cannot have the same name as a variable
in an outer scope:

```yaoxiang
// error: i has already been declared in the outer scope
i = 10
for i in 1..5 {
    print(i)
}

// correct: use a different variable name
i = 10
for j in 1..5 {
    print(j)
}
```

This rule applies to all code blocks; see [4.3 Shadowing Rules](modules.md#43-shadowing-rules) for
details.

#### 3.9.4 Comparison with Other Languages

| Language | for loop variable semantics                                  |
| -------- | ------------------------------------------------------------ |
| YaoXiang | Each iteration is a new binding                              |
| Rust     | Modifies the same variable (requires `mut`)                  |
| Python   | Modifies the same variable (no `mut` needed)                 |
| C/C++    | Modifies the same variable (requires pointers or references) |

**Design rationale**: YaoXiang adopts binding semantics because:

1. **More aligned with natural semantics**: in natural language, "for every element x in a set"
   means each x is an independent individual. YaoXiang's `for i in 1..5` reads as "for every i from
   1 to 5", where each iteration's i is a brand-new binding—consistent with human intuition.
2. **Avoids accidental modification**: the default immutable binding semantics means the loop
   variable cannot be accidentally modified inside the body. There is no need to worry about a
   hard-to-trace bug caused by accidentally writing `i = ...` somewhere in a complex body.
3. **High-performance solutions within reach**: when it is really necessary to reuse a variable
   across iterations (e.g. an accumulator or cache), use `for mut` to switch to the mutable binding
   mode. This is clearer than implicit shared state—the intent is expressed explicitly in the syntax
   rather than hidden in runtime behavior.

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

**spawn loop**: a data-parallel loop.

```yaoxiang
results = spawn for item in items {
    process(item)
}
```

**spawn block captures outer variables** (RFC-024 §2.3, value-capture semantics):

- An outer variable referenced by the block body = **Move value capture**: the value is snapshotted
  into the closure environment at the spawn creation point, and the block body reads through the env
  (LoadUpvalue)
- **Primitives** (Int/Float/Bool/Char) are copied by value; outer variables are unaffected
- **Handle types** (Struct/String/List, etc.) snapshotting = handle copying, sharing the underlying
  object; under the Embedded runtime (default), same thread and same heap, the handle is valid
- Sharing across multiple tasks requires an explicit `ref` (§2.13; the compiler automatically picks
  Rc/Arc)
- A `return` inside the block that references an outer variable also captures that variable

```yaoxiang
t1 = 1 + 1
t2 = 2 + 2
result = spawn {
    return t1 + t2    // t1/t2 are value-captured, result == 6
}
```

---

### 3.11 Program Entry and Top-Level Statements

A source file has two roles in the compiler's view, **determined by the presence of
`yaoxiang.toml`**:

| Role       | Determination                                | Program body                                                                       |
| ---------- | -------------------------------------------- | ---------------------------------------------------------------------------------- |
| **Script** | Single file run directly, no `yaoxiang.toml` | **Top-level statements** (executed in source order); `main` is an ordinary binding |
| **Bin**    | `yaoxiang.toml` exists                       | **`main` function**; top-level executable statements are not allowed               |

#### Script: Top-Level Statements Are the Program

Without a manifest, the file is a "script", and **the top-level statements are executed in source
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

main()                       // ← this line is required
```

> **Why isn't `main` called automatically?** The top-level statements are already the program body.
> If `main` were also called implicitly, a script that explicitly wrote `main()` would execute
> twice. The two rules cannot coexist, so under Script there is only one entry point: "top-level
> statements".

#### Bin: `main` Is the Entry

When there is a manifest, the file is an "executable target", and at this point:

- A **binding** named `main` must be defined; either a value or a function works (#388 decision: the
  entry is decided by the existence of the binding—a function is already a value, the two differ
  only in evaluation strategy): a function main is called with zero arguments at the entry stage; a
  value main is evaluated during initialization, which counts as execution
- Top-level executable statements are not allowed—the program body is `main` itself

```yaoxiang
main: () -> Void = {
    print("hello")
}
```

A value main is also legal—its evaluation at initialization counts as execution, and the evaluation
happens within the initialization sequence (in topological order of the global bindings, with
independent bindings following source order):

```yaoxiang
main = {
    print("hello")
}
```

Missing `main` is a compile error (without `main`, all functions are unreachable). A value main that
cannot be called (e.g. `main: Int = 5`) is legal but has no observable effect—comparable to Rust's
empty `fn main() {}`.

> **Library files**: files that are `use`d by other files, or files pointed to by `[lib].path` /
> `[exports]`, do not require `main`—they are not program entry points.

#### Initialization of Top-Level Bindings

The initialization values of top-level bindings are **evaluated at runtime**, and are not required
to be compile-time constants:

```yaoxiang
answer: Int = { 42 }              // block value
inc: (Int) -> Int = (x) => x + 1
computed: Int = inc(41)           // function call
```

Initialization is performed in **dependency order**, regardless of source order:

```yaoxiang
derived: Int = base * 3           // refers to base declared later
base: Int = 7                     // initialized first (topological sort)
```

Cyclic dependencies are a compile error (the names on the cycle will be listed):

```yaoxiang
a: Int = b + 1
b: Int = a + 1                    // error: a → b → a
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

`type` is no longer a keyword (RFC-010; use `Name: Type = { ... }`); `pub` produces no visibility
effect (RFC-029). Literal reserved words `true` / `false` / `void` are in §1.4.1; the meta-type name
`Type` and the built-in type names `Void` / `Never` / `Int` / `Float` / `Bool` / `Char` / `String`
are in §1.4.3.

### A.1 Control Flow

```
if Expr Block (else if Expr Block)* (else Block)?
match Expr { MatchArm+ }
while Expr Block
for 'mut'? Identifier 'in' Expr Block
break | continue          // only valid inside a loop body (§3.4 / §3.5)
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
