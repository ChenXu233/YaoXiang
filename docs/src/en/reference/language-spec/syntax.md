# Syntax Specification

This document defines the syntax specification of the YaoXiang programming language, including
lexical structure, grammar rules, and operator precedence.

---

## Chapter 1: Lexical Structure

### 1.1 Source File

YaoXiang source files must use UTF-8 encoding. Source files typically have the `.yx` extension.

### 1.2 Lexical Token Categories

| Category   | Description                      | Examples                  |
| ---------- | -------------------------------- | ------------------------- |
| Identifier | Starts with letter or underscore | `x`, `_private`, `my_var` |
| Keyword    | Language-defined reserved word   | `use`, `mut`, `and`       |
| Literal    | Fixed value                      | `42`, `"hello"`, `true`   |
| Operator   | Operation symbol                 | `+`, `-`, `*`, `/`        |
| Separator  | Syntactic separator              | `(`, `)`, `{`, `}`, `,`   |

### 1.3 Keywords

YaoXiang has **18 keywords** (`src/frontend/core/lexer/state.rs:25-55` `keyword_from_str`, each
corresponds to a `TokenKind`):

```
pub     use     spawn  ref     mut
if      else    match  while   for
in      return  break  continue
as      unsafe  and    or
```

`and` / `or` are the **keywords** for logical and / or (Zig-style, precedence see §2.2 level 10);
the unary not is the symbol `!`, orthogonal to them.

These keywords have special meaning in any context and cannot be used as identifiers.

> **`type` is no longer a keyword** (RFC-010): use the `Name: Type = { ... }` notation to write type
> definitions. `src/frontend/core/lexer/state.rs:27` explicitly notes this, and the `TokenKind` enum
> header (`src/frontend/core/lexer/tokens.rs:82`) also says "16 total - RFC-010: 'type' keyword
> removed" — **that 16 is an obsolete comment**, and 18 are actually listed (including `and` / `or`,
> while the `Kw*` prefixed batch is still 16).
>
> **`pub` produces no visibility effect**: `pub` is still lexically recognized as `KwPub`
> (`src/frontend/core/lexer/state.rs:28`), the parser skips it at declarations and import items
> (`src/frontend/core/parser/statements/declarations.rs:666-671,726`, `.../imports.rs:57-59`), but
> the module system **makes no visibility decisions based on it** — the accepted
> [RFC-029](../../rfc/accepted/029-module-semantics.md) explicitly states "no `pub`, no `private`,
> no `export`, no visibility mechanism" (line 17 of that file). Whether you write `pub` or not makes
> no difference to visibility.

### 1.4 Reserved Words

YaoXiang's "reserved words" are divided into three layers, recognized by the parser and type checker
at different stages:

#### 1.4.1 Literal Reserved Words

The parser has independent tokens for literal identifiers and cannot be used as ordinary
identifiers:

| Identifier | Belongs to Type | Description                                                                                                   |
| ---------- | --------------- | ------------------------------------------------------------------------------------------------------------- |
| `true`     | Bool            | Boolean true value                                                                                            |
| `false`    | Bool            | Boolean false value                                                                                           |
| `void`     | Void            | Void literal (Unit value). Lowercase `void` is a value literal; uppercase `Void` is a type name (see §1.4.3). |

> **`Type` is not in this layer**: it has **no** independent `TokenKind` (it is not in the
> `TokenKind` enum in `src/frontend/core/lexer/tokens.rs`, nor is there a corresponding branch in
> `keyword_from_str`), the parser treats it as an ordinary identifier, and the type checker
> recognizes it as a meta-type in type position. Therefore in expression position `Type` can be
> shadowed by local bindings. It is a **meta-type name**, not a keyword.

#### 1.4.2 Variant Names

Variant names are recognized by the parser in **pattern** context (bare names are resolved by the
scrutinee type's variant set, see §2.8); in **expression** context, constructing a variant requires
type qualification — `Result(Int, String).ok(5)`, `Option(T).some(v)`, `Color.green()` (RFC-010
record and types: variant constructors are function-typed fields in type definitions, bare name
`ok(5)` is not a constructor call).

| Variant Name | Belongs to Type | Description                       |
| ------------ | --------------- | --------------------------------- |
| `some(T)`    | Option          | Option value variant construction |
| `ok(T)`      | Result          | Result success variant            |
| `err(E)`     | Result          | Result error variant              |

#### 1.4.3 Built-in Type Names

The following type names are pre-registered by the type checker and can be used in type position
without import. The parser treats them as ordinary identifiers — **not reserved words, can be
shadowed by local bindings (not recommended)**.

| Type Name | Logical Correspondence | Description                                                                                                                                  |
| --------- | ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `Void`    | ⊤ (true/Unit)          | Zero-field product type, with exactly one inhabitant (`void` literal, see §1.4.1)                                                            |
| `Never`   | ⊥ (false/empty type)   | Zero-variant sum type, zero inhabitants. No expression can produce a `Never` value. `Never <: T` holds for all `T` (principle of explosion). |
| `Int`     | —                      | Signed integer                                                                                                                               |
| `Float`   | —                      | Floating point number                                                                                                                        |
| `Bool`    | —                      | Boolean value: `true` / `false`                                                                                                              |
| `Char`    | —                      | Unicode character                                                                                                                            |
| `String`  | —                      | String                                                                                                                                       |

### 1.5 Identifiers

Identifiers start with a letter or underscore, followed by letters, digits, or underscores.
Identifiers are case-sensitive.

Special identifiers:

- `_` is used as a placeholder to indicate ignoring a value
- Identifiers starting with an underscore indicate private members

### 1.6 Literals

#### 1.6.1 Integer

```
Decimal     ::= [0-9][0-9_]*
Octal       ::= 0o[0-7][0-7_]*
Hex         ::= 0x[0-9a-fA-F][0-9a-fA-F_]*
Binary      ::= 0b[01][01_]*
```

#### 1.6.2 Floating Point

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
Array       ::= '[' Expr (',' Expr)* ']'   // When the target type annotation is Array(T, N), the literal becomes a fixed-length array
```

> **Dictionary literals require at least one key-value pair**: `{}` is **not** an empty dictionary —
> it is an empty block (value `Void`, see [§2.9](#_2-9-block-expression)). For an empty dictionary,
> use the constructor `dict.new()`:
>
> ```yaoxiang
> empty = dict.new()                  // ✅ empty dictionary
> d = { "a": 1 }                       // ✅ dictionary literal
> wrong = {}                           // ❌ this is not a dictionary, it's an empty block (Void)
> ```
>
> **The criterion is the content**: The `Dict` grammar requires at least one `String ':' Expr`, and
> `{}` has no content to refer to, so it takes the zero form of block structure. Non-empty forms are
> self-describing by their content (`{ "k": v }` has key-value pairs → dictionary) — this is the
> same principle as `f = { 5 }` being an `Int` value rather than a function: **type is determined by
> content**.
>
> Set has no literal grammar and no runtime representation — collection types are in planning, and
> when the requirement arises, follow the Dict pattern (std.set + HeapValue::Set). The landing point
> of List/Dict literals is determined by the context type annotation: bare literals and `List(T)`
> annotations land as growable lists; `Array(T, N)` annotations directly act on literals to land as
> fixed-length arrays. Implicit List→Array conversion is forbidden.
>
> Array literal semantics:
>
> - The number of elements must equal N, otherwise compile-time E1002; an empty literal paired with
>   non-zero N is also rejected
> - Each element type must be compatible with T, otherwise compile-time E1002
> - The grammar form of N: only integer literals (possibly negative) or constant names; compound
>   expressions (e.g. `2+1`) are rejected at parse time
> - When N is a symbolic constant (function const parameter, e.g. `Array(Int, n)`), the count check
>   is deferred to the refined type phase
> - Nested array literals in v1 (`Array(Array(Int,2),2) = [[1,2],[3,4]]`) are rejected at compile
>   time, requiring explicit construction layer by layer; recursive landing points will be addressed
>   in future versions

#### 1.6.5 List Comprehension

```
ListComp    ::= '[' Expr ( 'for' Identifier 'in' Expr ( 'if' Expr )? )+ ']'
```

> **#401 (implementation completion)**: `if` filtering and multiple generator clauses are landed
> from this version — previously the grammar was written as `(',' Expr)* ('if' Expr)?`, with no
> implementation corresponding (comma-appended expressions and multiple generators are both
> unsupported), and `if` filtering was entirely missed by the parser (after parsing the iterable, it
> directly expected `']'`, reporting `E0010 Expected RBracket, found KwIf`). Now, following industry
> common semantics: one or more generator clauses, expanded as nested loops; each clause can carry
> at most one `if` filter, with the condition forced to `Bool` (non-Bool reports `E1054`);
> subsequent clauses' iterable/condition can reference the iteration variables bound by earlier
> clauses.
>
> **Behavior tightening (migration record)**: The iteration variable grammar was originally
> `'for' Identifier 'in'`, but in the old implementation, the pattern went through the full pratt
> parsing — after `'in'` was registered as an infix operator, `x` would swallow `in items` as a
> membership expression. After the fix, non-identifier patterns fail to parse directly, no longer
> like the old implementation falling back to `_` (silently swallowing the error). Impact:
> previously parseable `[x for (a, b) in pairs]` now reports an error — this form never had defined
> behavior (the variable was always `_`), the tightening direction is correct, with no semantic
> migration cost.

#### 1.6.6 Membership Test

```
Membership  ::= Expr 'in' Expr
```

> `in` is a binary relational operator returning `Bool` — `true` on hit, `false` on miss, no error.
> Semantic split: `[]` asserts existence and takes the value (fails with error), `in` asks whether
> it exists (miss is normal `false`). Right operand coverage: List / Array / Dict(key set) / Tuple /
> String(substring) / Range(interval). `in` is a first-class Hoare predicate, serving as the base of
> compile-time provable propositions in the refined type phase. (Set is removed from the right
> operand list — Set has no runtime representation, see §1.6.4)

### 1.7 Comments

```
// Single-line comment

/* Multi-line comment
   can span multiple lines */
```

### 1.8 Indentation Rules

Code must use 4-space indentation, and Tab characters are forbidden. This is a mandatory syntax
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

| Precedence | Operator                    | Associativity |
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

> **Unary prefix operators** (`!` `-` `+`) bind tightly: only below call and member access, above
> all binary operators. Therefore `!a == b` ≡ `(!a) == b` (Zig-style semantics); `!` is a pure unary
> operation, does not participate in short-circuit control flow, orthogonal to `and`/`or` keywords
> (short-circuit) (RFC-010 authoritative definition).
>
> **Range binding strength**: `..` binding strength (6, 7) — left 6 is lower than addition (7),
> right 7 swallows addition but not sibling `..`. Before and after change comparison:
>
> | Expression   | Before (level 1, right-assoc)                                                               | After ((6,7), left-assoc)                                                        |
> | ------------ | ------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------- |
> | `x in 1..10` | `x in 1..10` (`in` right operand level 4, `..` level 1 can't swallow, actually unparseable) | `x in (1..10)` — interval as a whole as `in` right operand                       |
> | `0..n+2`     | `(0..n)+2` (right-assoc trap: upper bound eaten, `for` loop directly E3004)                 | `0..(n+2)` — upper bound is arithmetic expression                                |
> | `a == b..c`  | `a == (b..c)` (`..` level 1 < `==` level 3, naturally whole)                                | `a == (b..c)` — **semantics unchanged**, `..` still higher than comparison level |
> | `1..2*3`     | `(1..2)*3`                                                                                  | `1..(2*3)` — upper bound is arithmetic expression                                |
> | `a..b..c`    | `a..(b..c)` (right-assoc chained, meaningless Range inside Range)                           | `(a..b)..c` — **step form** (`c` is the step)                                    |
>
> Net effect: composite upper bound `for i in 0..n+2` goes from "parses successfully but E3004" to
> "directly usable"; `x in 1..10` goes from "unparseable" to "interval check"; `a..b..c` goes from
> "meaningless nesting" to "step component". Level 6 falls between `+` (level 5) and `<<` (level 7),
> mathematical convention: the interval is a tightly bound construct, with the upper bound naturally
> being a complete arithmetic expression.

### 2.3 Function Call

```
FnCall      ::= Expr '(' ArgList? ')'
ArgList     ::= Expr (',' Expr)* (',' NamedArg)* | NamedArg (',' NamedArg)*
NamedArg    ::= Identifier '=' Expr
```

Named arguments use `name = value` (RFC-010 §Function Definition, RFC-011 §Construction Form).
Positional arguments must come before named arguments; parameters specified by order can be in any
order in named arguments:

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b

add(3, 5)          // Positional
add(a = 3, b = 5)  // Named
add(b = 5, a = 3)  // Any order
add(3, b = 5)      // Mixed, positional first
```

Naming a parameter incorrectly reports **E1014**, specifying the same parameter both positionally
and by name reports **E1015**, mismatched count reports **E1010** (RFC-013).

**Unnamed parameters and named arguments**: The signature parameter position allows the bare type
form (`mk: (Int, Int) -> Int`), which is an **unnamed typed parameter** — type constraints take
effect by position, the name belongs to the implementation (lambda header carries it, see
[RFC-007](../../rfc/accepted/007-function-syntax-unification.md) shorthand rules), and does not
enter the contract. Unnamed parameters cannot be referenced by named arguments (no name to
reference), and can only be passed by position; the way to put a name in the contract is
`mk: (a: Int, b: Int) -> Int`. The bare identifier must be a declared type, otherwise an error is
reported (see [type-system §3.5](type-system.md#35-function-type)).

### 2.4 Member Access

```
MemberAccess::= Expr '.' Identifier
```

### 2.5 Index Access

```
IndexAccess ::= Expr '[' Expr ']'
```

> **Three-layer semantics (RFC-011b)**: `a[i]` is split by the type of `a` — ① built-in containers
> (List/Vec/Array/Dict/Tuple) go through native indexing instructions (fast path); ② user types
> implementing the `Index` interface are dispatched to their `index` method (instantiation of
> `Index(Grid, Int, Float)` in the type body + `Grid.index` method); ③ the `f[0]` positional binding
> from RFC-004 only exists in binding declarations, not in this grammar. The key for
> multi-dimensional index `a[0, 1]` is packaged as a tuple. Other types not implementing `Index` are
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

> **Variant set requirement for variant destructuring** (RFC-010b): `EnumPattern` (`ok(v)`,
> `some(x)`, etc. variant names resolved by scrutinee) requires that the scrutinee's type is a **sum
> type whose variant set is in scope**. The variant set only enters the checker through type
> definitions or `use` imports — `Result`/`Option` are defined by `std.result`/`std.option`, and you
> must `use std.result` / `use std.option` before use (whole module and grouped `use std.{...}`
> forms are equivalent); otherwise variant destructuring reports E1002. Exhaustiveness determination
> is the same source: the catch-all arm is exempt from exhaustiveness, and without a catch-all arm,
> the full variant set is checked.

### 2.9 Block Expression

```
Block       ::= '{' Stmt* Expr? '}'
```

> **Statement termination rules**: The separation between Stmts and newline behavior (explicit `;`
> separation, newline termination, continuation exceptions, leading `(`/`[` never merged) are
> defined by [RFC-038](../../rfc/accepted/038-statement-termination.md).

#### 2.9.1 Three Forms of `{`

`{` has exactly three interpretations in expression position, decided by **content** in one pass:

| Form             | Notation          | Type                 | Example          |
| ---------------- | ----------------- | -------------------- | ---------------- |
| **Empty block**  | `{}`              | `Void`               | `x: Void = {}`   |
| **Dict literal** | `{ "k": v, ... }` | `Dict(K, V)`         | `d = { "a": 1 }` |
| **Block**        | `{ Stmt* Expr? }` | Tail expression type | `y = { 1 + 1 }`  |

**Decision order**:

1. `{}` (no content) → **Empty block**, value `Void`
2. First element is `String ':' Expr` key-value pair → **Dict literal**
3. Otherwise → **Block**, value given by tail expression

> **Why `{}` is not an empty dictionary**: The "emptiness" of an empty dictionary cannot be
> self-describing (it could be either `Dict(K, V)` or an empty block), and the dict grammar
> [§1.6.4](#_1-6-4-collections) requires at least one key-value pair. When there is no content to
> refer to, the zero form of block structure is taken: this is consistent with `unsafe {}` /
> `spawn {}`, no special case is introduced. For an empty dictionary, use `dict.new()`.
>
> **Why functions need annotations**: `f = { stmt }` is a **value** (tail expression type), not a
> function. To define a function, write the Fn annotation: `f: () -> Int = { 5 }`. This is the same
> principle as for dict: **type is determined by content**, not by the presence of an annotation.
> (This rule is in [RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md) Appendix D.)

**Unified semantics**: The value of all `{}` blocks is given by the **tail expression**, `return` is
a non-local exit of type `Never`.

| Block Type    | Value Outlet    | Empty Block `{}` |
| ------------- | --------------- | ---------------- |
| Ordinary `{}` | Tail expression | `Void`           |
| `unsafe {}`   | Tail expression | `Void`           |
| `spawn {}`    | Tail expression | `Void`           |

**Core principles** (see [RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md) for
details):

- **Block value = tail expression** (the last expression), the only outlet, no exceptions
- When the last position is an **assignment statement**, the block value is `Void`; to get `Void`,
  write `Void` explicitly
- **`return` exits the nearest function boundary** (passes through all blocks, does not "return to
  the block"), type `Never`; `Never <: T` holds for any type (principle of explosion), so it can
  appear at any return type position. The module initialization layer (Script top level) has no
  function boundary, `return` is rejected at compile time here (`E1109`, see §3.11)
- The expression form `= expr` directly gives the value

```yaoxiang
// Ordinary {} block: tail expression gives the value
result = {
    x = compute()
    x                // value of the block
}

// unsafe {} block: tail expression gives the type definition
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void
    }
    SqliteDb         // value of the block
}

// spawn {} block: tail expression gives the result
(a, b) = spawn {
    result1 = fetch("url1"),
    result2 = fetch("url2")
    (result1, result2)   // value of the block
}

// return: passes through the block, exits the function
f: (n: Int) -> Int = {
    if n < 0 {
        return 0     // passes through if and the function body, exits the function
    }
    n * 2            // tail expression
}
```

#### Is `name = { ... }` a Function or a Block Value?

`name = { ... }` could be either a function definition (RFC-007 "no-arg simplest") or a block value
binding. Resolved by **annotation first, function by default** (RFC-010a Appendix D):

| Case                            | Result          | Example                            |
| ------------------------------- | --------------- | ---------------------------------- |
| Value is Lambda (`=>`)          | Function        | `f = () => 5` → `f()` = 5          |
| Annotation is function type     | Function        | `f: () -> Int = { 5 }` → `f()` = 5 |
| Annotation is non-function type | **Block value** | `x: Int = { 5 }` → `x` = 5         |
| No annotation                   | Function        | `f = { 5 }` → `f()` = 5            |

**Annotation is the type**: `x: Int = ...` declares `x` to be `Int`, so `{ ... }` evaluates to
`Int`; `f: () -> Int = ...` declares `f` to be a function, so `{ ... }` is the function body.

To have `{ ... }` evaluate immediately, **just write the target type** (no new syntax needed):

```yaoxiang
// Block value: evaluate immediately
x: Int = {
    y = 5
    y            // x = 5
}

// Function: no annotation, defaults to function
f = { 5 }        // f() = 5
```

#### Nested Function Types: Currying or Returning a Function?

The nested function type on the right of `->` has two readings, distinguished by **parentheses**
(RFC-004):

| Writing                         | Meaning              | Call                  |
| ------------------------------- | -------------------- | --------------------- |
| `(a: Int) -> (b: Int) -> Int`   | **Currying**         | `f(1)(2)`             |
| `(a: Int) -> ((b: Int) -> Int)` | **Returns function** | `g(1)` gives function |

Basis: **Annotation is the type**. `g: (a: Int) -> ((b: Int) -> Int)` declares
`g(1) : (b: Int) -> Int`, so `g(1)` must **be** that function, not "the next segment of parameters".

```yaoxiang
// Currying: two segments of parameters given layer by layer
add: (a: Int) -> (b: Int) -> Int = { a + b }
add(1)(2)        // → 3

// Returns function: outer layer one segment of parameter, what is returned is the function
adder: (n: Int) -> ((x: Int) -> Int) = (x) => x + n
adder(10)(5)     // → 15
h = adder(10)    // The returned function can be stored in a variable, passed
```

Unparenthesized nested `Fn` is always currying (including the type parameter form in RFC-011):

```yaoxiang
identity: (T: Type) -> (x: T) -> T = (x) => x   // Currying
identity(5)      // → 5
```

**Type checking**: The parentheses declare the return type, the body must produce that type.
`f: () -> (() -> Int) = { 7 }` is an error — expected `() -> Int`, actually got `Int` (E1002).

### 2.10 Lambda Expression

```
Lambda      ::= '(' ParamList? ')' '=>' Expr
            |  '(' ParamList? ')' '=>' Block
```

### 2.11 Error Propagation Operator

```
ErrorPropagate ::= Expr '?'
```

The `?` operator is a postfix operator with the same precedence as `.`. For `Result(T, E)` type:

- Extracts value `v` on `Ok(v)` and continues execution
- On `Err(e)`, propagates the error upward (`return Err(e)`)

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

// Range is a value: bind, pass, membership test
r = 1..10
assert.assert(5 in r, "membership")
for i in r { print(i) }

// step form (third component, default 1)
for i in 0..10..2 { print(i) }  // 0, 2, 4, 6, 8
for i in 10..0..(-2) { print(i) }  // 10, 8, 6, 4, 2
```

> **step semantics**: In `a..b..c`, `c` is the step. Literal `c = 0` is rejected at compile time;
> dynamic `c` is zero-checked at runtime (E6001 family; will be promoted to Result after the error
> system lands). `c < 0` is legal, the range direction reverses with the sign (`10..0..(-2)` is
> decreasing).

### 2.13 ref Expression

```
RefExpr     ::= 'ref' Expr
```

`ref` creates shared ownership. The compiler automatically chooses Rc (single task) or Arc
(cross-task), and the user does not need to care about implementation details.

```yaoxiang
data = ref heavy_data
spawn { use(data) }   // Cross-task: the compiler automatically chooses Arc
```

### 2.14 unsafe Expression

```
UnsafeExpr  ::= 'unsafe' Block
```

`unsafe` blocks are used to define opaque types and operate on raw pointers. Use `return` to return
the type definition to the outer scope.

**Semantics**:

- Types and raw pointer operations can be defined in `unsafe {}`
- The returned type is available outside `unsafe {}`
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
- Variable declarations follow the "assign-first" principle

```yaoxiang
// Block scope
{
    x = 10
    // x is visible in this scope
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

- `x = value`: look up `x` along the scope chain outward, assign if found, otherwise declare new
- `mut x = value`: explicit new mutable declaration, forbidden to share a name with the outer scope
- Any name can only be declared once in the same scope

> **Detailed definition**: For complete rules of scope, variable declaration and shadowing
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

**Semantics**: `return` is a **non-local exit**, exiting the nearest **function boundary** (passes
through all blocks — including `if` / `while` / `for` / `match` / bare block / `spawn` / `unsafe`),
giving the value to the caller. It **does not "return to the block"**.

**Type**: `return e : Never` (`e : T`). `Never <: T'` holds for any `T'` (principle of explosion,
see [Type System §2.2](type-system.md)), so `return` can appear at any return type position without
additional rule constraints.

**Relationship with block evaluation**: The value of a block is always the **tail expression** (see
§2.9). As a block, `{ return n }` has value `n`, type `Never`; at the same time, `return` acts to
exit the function. **Both things hold at the same time**, coexisting through the principle of
explosion.

`return` and tail expression together make "early return" possible, without the need for additional
rules that specially designate `return` for functions — see
[RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md).

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 {
        return 1          // passes through if, exits the function (type Never)
    }
    n * factorial(n - 1)  // tail expression = value of the block
}
```

### 3.4 break Statement

```
BreakStmt   ::= 'break'
```

**Semantics**: Immediately terminates the innermost `while`/`for` loop, with control flow
transferred to after the loop body.

- **Only exits the nearest layer**: `break` always acts on the innermost loop containing it. In
  nested loops, to jump out of multiple layers at once, extract the inner loop as a function and use
  `return` to return, or use a flag (break/continue carry no label; if loop labels are introduced in
  the future, they will follow the loop declaration side syntax through the RFC process, decided
  together with the multi-exit design of the proof pipeline)
- **Limited to loop body only**: `break` can only appear in `while`/`for` loop body (including
  blocks/if/match nested in the body), appearing outside a loop reports a compile error (E1102
  `'break' outside of a loop`)
- **Does not affect termination proof**: `break` does not participate in the termination argument —
  it neither provides a measure nor constitutes a decreasing step of the measure; the termination
  obligation of the loop is independent of `break`, triggered by the refined type (see
  [type-system §8.4](type-system.md#84-terminates-termination-measure-predicate))
- **Borrow semantics**: The control flow edge of break participates in the structural cut of the
  reverse BFS liveness analysis of RFC-009a (the iterations jumped out do not participate in
  back-edge liveness derivation)

```yaoxiang
mut i = 0
while i < 10 {
    i = i + 1
    if i == 3 {
        break              // Control flow transfers to after the loop, i == 3
    }
}

// Nested loops: break only exits the inner layer
while j < 3 {
    while k < 10 {
        if k == 2 { break }    // only terminates the inner loop
    }
    j = j + 1                  // every outer iteration will execute to here
}
```

### 3.5 continue Statement

```
ContinueStmt::= 'continue'
```

**Semantics**: Skips the remaining statements in this iteration, directly entering the next round of
the innermost loop — `while` returns to re-evaluate the condition, `for` takes the next element.

- **Only acts on the nearest layer**: same as `break`, carries no label
- **Limited to loop body only**: appearing outside a loop reports a compile error (E1102)

```yaoxiang
mut sum = 0
mut n = 0
while n < 5 {
    n = n + 1
    if n == 3 {
        continue           // Skip the following accumulation, n == 3 is not counted
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
not modifying the same variable**.

```yaoxiang
// Example: for i in 1..5
for i in 1..5 {
    print(i)
}
```

**Execution process**:

| Iteration | Behavior of the Loop Variable                                                         |
| --------- | ------------------------------------------------------------------------------------- |
| 1st       | Create new binding `i = 1`, loop body executes, prints 1                              |
| 2nd       | Create new binding `i = 2` (previous binding destroyed), loop body executes, prints 2 |
| 3rd       | Create new binding `i = 3`, loop body executes, prints 3                              |
| 4th       | Create new binding `i = 4`, loop body executes, prints 4                              |
| End       | Loop body ends, binding destroyed                                                     |

**Key point**: After each iteration ends, the binding created in that iteration is destroyed. The
next iteration is a completely new binding, with no relation to the previous iteration's binding.

#### 3.9.2 Difference Between for and for mut

| Syntax              | Loop Variable Mutability | Description                                |
| ------------------- | ------------------------ | ------------------------------------------ |
| `for i in 1..5`     | Immutable                | Cannot modify the binding in the loop body |
| `for mut i in 1..5` | Mutable                  | Can modify the binding in the loop body    |

```yaoxiang
// Legal: each iteration binds a new value, no modification needed
for i in 1..5 {
    print(i)  // read value of i
}

// Error: immutable binding, cannot modify
for i in 1..5 {
    i = i + 1  // Error: cannot modify immutable binding
}

// Legal: use for mut to allow modification of the binding
for mut i in 1..5 {
    i = i + 1  // modification allowed
}
```

#### 3.9.3 Shadowing Check

YaoXiang forbids variable shadowing. The for loop variable cannot share a name with a variable in
the outer scope:

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

This rule applies to all code blocks, see [4.3 Shadowing Rules](modules.md#43-shadowing-rules).

#### 3.9.4 Comparison with Other Languages

| Language | for Loop Variable Semantics                             |
| -------- | ------------------------------------------------------- |
| YaoXiang | Each iteration binds a new value                        |
| Rust     | Modifies the same variable (needs mut)                  |
| Python   | Modifies the same variable (no mut needed)              |
| C/C++    | Modifies the same variable (needs pointer or reference) |

**Design rationale**: YaoXiang adopts binding semantics because:

1. **More natural semantics** In natural language, "for each element x in the collection" means each
   x is an independent individual. YaoXiang's `for i in 1..5` is read as "for each i from 1 to 5",
   each iteration's i is a completely new binding, which is consistent with human intuitive
   understanding.

2. **Avoids accidental modification** The default immutable binding semantics means the loop
   variable cannot be accidentally modified in the loop body. No need to worry about writing
   `i = ...` somewhere in a complex loop body causing hard-to-trace bugs.

3. **High-performance solutions are within reach** When you really need to reuse variables between
   iterations (e.g. accumulators, caches), use `for mut` to declare and switch to mutable binding
   mode. This is clearer than implicit shared state — intent is expressed explicitly through syntax,
   not hidden in runtime behavior.

### 3.10 spawn Statement

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' 'mut'? Identifier 'in' Expr '{' Expr '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
```

**spawn block**: Explicitly declares a concurrent region, expressions in the block execute
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

**spawn blocks capture outer variables** (RFC-024 §2.3, value capture semantics):

- Block body references outer variable = **Move value capture**: the value is snapshotted into the
  closure environment at the spawn creation point, the block body reads it through env (LoadUpvalue)
- **Primitives** (Int/Float/Bool/Char) copy values, outer variables are not affected
- **Handle types** (Struct/String/List etc.) snapshot = handle copy, sharing the underlying object;
  Embedded runtime (default) same thread same heap, handle is valid
- Sharing between multiple tasks requires explicit `ref` (§2.13, compiler automatically chooses
  Rc/Arc)
- `return` in the block references the same captured outer variables

```yaoxiang
t1 = 1 + 1
t2 = 2 + 2
result = spawn {
    return t1 + t2    // t1/t2 value capture, result == 6
}
```

---

### 3.11 Program Entry and Top-Level Statements

A source file has two roles in the compiler's view, **determined by the presence of
`yaoxiang.toml`**:

| Role       | Determination                                | Program Body                                                                        |
| ---------- | -------------------------------------------- | ----------------------------------------------------------------------------------- |
| **Script** | Single file run directly, no `yaoxiang.toml` | **Top-level statements** (executed in written order); `main` is an ordinary binding |
| **Bin**    | `yaoxiang.toml` exists                       | **`main` function**; top level cannot have executable statements                    |

#### Script: Top-Level Statements are the Program

Without a manifest, the file is a "script", **top-level statements execute in written order**
(top-level **binding initialization** runs first overall, see "Top-Level Binding Initialization"
below):

```yaoxiang
use std.io
io.println("hello")          // executed directly
x: Int = { 42 }              // top-level binding: runtime initialization
io.println(x)                // 42
```

`main` in this mode is **not special** — it is just an ordinary binding. To make `main` run, you
must call it explicitly:

```yaoxiang
use std.io
main: () -> Void = { io.println("only runs if called") }

main()                       // ← must write this line
```

> **Why not auto-call `main`?** The top-level statements are already the program body. If `main`
> were called implicitly, a script with an explicit `main()` would execute twice. The two rules
> cannot coexist, so under Script there is only one execution entry: "top-level statements".

Top level also **cannot use `return`**: Script top level is the module initialization layer, there
is no function boundary to exit (RFC-010a rule ②), compile-time reports `E1109` — rather than
silently terminating initialization at runtime and skipping the remaining top-level statements. To
finish a piece of top-level logic early, put it in a function and control it at the call site.

#### Bin: `main` is the Entry

With a manifest, the file is an "executable target", in which case:

- A binding named `main` must be defined, value or function both work (#388 final decision: entry is
  determined by binding existence — function is already a value, the two are just different
  evaluation strategies): function main is called with zero args at entry; value main is evaluated
  at initialization, which executes
- Top level does not allow executable statements — the program body is `main`

```yaoxiang
main: () -> Void = {
    print("hello")
}
```

Value main is also legal — evaluation at initialization is execution, evaluation occurs in the
initialization sequence (in global binding topological order, independent bindings in source order):

```yaoxiang
main = {
    print("hello")
}
```

Missing `main` is a compile error (without `main`, all functions are unreachable). A non-callable
value main (e.g. `main: Int = 5`) is legal but has no observable effect — comparable to Rust's empty
`fn main() {}`.

> **Library files**: Files that are `use`d by other files, or files pointed to by `[lib].path` /
> `[exports]`, do not require `main` — they are not program entries.

#### Top-Level Binding Initialization

The initialization value of top-level bindings **is evaluated at runtime**, not required to be a
compile-time constant:

```yaoxiang
answer: Int = { 42 }              // block value
inc: (Int) -> Int = (x) => x + 1
computed: Int = inc(41)           // function call
```

Initialization executes in **dependency order**, independent of written order (dependencies are
initialized first; bindings without dependencies are in declaration order):

```yaoxiang
derived: Int = base * 3           // references base declared later
base: Int = 7                     // initialized first (topological sort)
```

And **all binding initialization runs before top-level statements** — side effects in binding
initializers occur before any top-level statement, regardless of how the two are interleaved in
writing:

```yaoxiang
print("a")                        // executes later
x = { print("b") }                // executes first (binding initialization runs first overall)
print("c")                        // output order: b, a, c
```

Circular dependencies are compile errors (the names on the cycle are listed):

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

`type` is no longer a keyword (RFC-010, using `Name: Type = { ... }` instead); `pub` has no
visibility effect (RFC-029). Literal reserved words `true` / `false` / `void` see §1.4.1, meta-type
name `Type` and built-in type names `Void` / `Never` / `Int` / `Float` / `Bool` / `Char` / `String`
see §1.4.3.

### A.1 Control Flow

```
if Expr Block (else if Expr Block)* (else Block)?
match Expr { MatchArm+ }
while Expr Block
for 'mut'? Identifier 'in' Expr Block
break | continue          // only in loop body (§3.4 / §3.5)
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
