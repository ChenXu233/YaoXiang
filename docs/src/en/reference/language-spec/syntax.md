# Syntax Specification

This document defines the syntax specification of the YaoXiang programming language, including
lexical structure, grammar rules, and operator precedence.

---

## Chapter 1: Lexical Structure

### 1.1 Source Files

YaoXiang source files must use UTF-8 encoding. Source files typically use the `.yx` extension.

### 1.2 Lexical Token Categories

| Category   | Description               | Examples                  |
| ---------- | ------------------------- | ------------------------- |
| Identifier | Begins with letter or `_` | `x`, `_private`, `my_var` |
| Keyword    | Language-defined reserved | `use`, `mut`, `and`       |
| Literal    | Fixed value               | `42`, `"hello"`, `true`   |
| Operator   | Operation symbol          | `+`, `-`, `*`, `/`        |
| Delimiter  | Syntax delimiter          | `(`, `)`, `{`, `}`, `,`   |

### 1.3 Keywords

YaoXiang has **17 keywords** (the `keyword_from_str` in `src/frontend/core/lexer/state.rs`, each
corresponding one-to-one to a `TokenKind`):

```
use     spawn   ref     mut
if      else    match   while   for
in      return  break   continue
as      unsafe  and     or
```

`and` / `or` are **keywords** for logical AND / OR (Zig-style; see precedence level 10 in §2.2);
unary NOT is the symbol `!`, orthogonal to them.

These keywords have special meaning in any context and cannot be used as identifiers.

> **`type` is no longer a keyword** (RFC-010): use the `Name: Type = { ... }` notation to write type
> definitions. Both `keyword_from_str` and the `TokenKind` enum header
> (`src/frontend/core/lexer/tokens.rs`) are annotated with "RFC-010: 'type' keyword removed".
>
> **`pub` is no longer a keyword** (RFC-029g, accepted 2026-10-02): the language has no visibility
> mechanism, and `pub` has been removed entirely from the lexer, AST, type checker, dead-code
> exemptions, formatter, and LSP ([RFC-029g](../../rfc/accepted/029g-remove-pub-and-auto-bind.md)).
> It is now just an ordinary identifier—the old form `pub x = 1` will be reported as E1001
> (undefined name). All top-level bindings are importable by default; modifiers make no difference
> to visibility ([RFC-029](../../rfc/accepted/029-module-semantics.md)).

### 1.4 Reserved Words

YaoXiang's "reserved words" are organized in three layers, recognized by the parser and type checker
at different stages:

#### 1.4.1 Literal Reserved Words

The parser has dedicated tokens for these literal identifiers, and they cannot be used as ordinary
identifiers:

| Identifier | Type | Description                                                                                                   |
| ---------- | ---- | ------------------------------------------------------------------------------------------------------------- |
| `true`     | Bool | Boolean truth value                                                                                           |
| `false`    | Bool | Boolean false value                                                                                           |
| `void`     | Void | Void literal (Unit value). Lowercase `void` is a value literal; uppercase `Void` is a type name (see §1.4.3). |

> **`Type` is not in this layer**: it has **no** dedicated `TokenKind` (it does not appear in the
> `TokenKind` enum in `src/frontend/core/lexer/tokens.rs`, and `keyword_from_str` has no
> corresponding branch); the parser treats it as an ordinary identifier, and the type checker
> recognizes it as a meta-type in type positions. Therefore, `Type` can be shadowed by a local
> binding in expression position. It is a **meta-type name**, not a keyword.

#### 1.4.2 Variant Names

Variant names are recognized by the parser in **pattern** contexts (bare names are resolved against
the scrutinee's variant set, see §2.8); in **expression** contexts, constructing a variant requires
a type qualifier—`Result(Int, String).ok(5)`, `Option(T).some(v)`, `Color.green()` (RFC-010 record
and types: variant constructors are function-typed fields in the type definition, and a bare name
like `ok(5)` is not a constructor call).

| Variant Name | Type   | Description                       |
| ------------ | ------ | --------------------------------- |
| `some(T)`    | Option | Option value variant construction |
| `ok(T)`      | Result | Result success variant            |
| `err(E)`     | Result | Result error variant              |

#### 1.4.3 Built-in Type Names

The following type names are pre-registered by the type checker and can be used in type positions
without import. The parser treats them as ordinary identifiers—**they are not reserved words, and
can be shadowed by local bindings (not recommended)**.

| Type Name | Logical Correspondence | Description                                                                                                                               |
| --------- | ---------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| `Void`    | ⊤ (true/Unit)          | Zero-field product type, exactly one inhabitant (the `void` literal, see §1.4.1)                                                          |
| `Never`   | ⊥ (false/empty type)   | Zero-variant sum type, zero inhabitants. No expression can produce a `Never` value. `Never <: T` holds for all `T` (explosion principle). |
| `Int`     | —                      | Signed integer                                                                                                                            |
| `Float`   | —                      | Floating-point number                                                                                                                     |
| `Bool`    | —                      | Boolean value: `true` / `false`                                                                                                           |
| `Char`    | —                      | Unicode character                                                                                                                         |
| `String`  | —                      | String                                                                                                                                    |

### 1.5 Identifiers

Identifiers begin with a letter or underscore; subsequent characters may be letters, digits, or
underscores. Identifiers are case-sensitive.

Special identifiers:

- `_` is used as a placeholder to indicate that a value is ignored
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
Array       ::= '[' Expr (',' Expr)* ']'   // When the target type is annotated as Array(T, N), the literal becomes a fixed-size array
```

> **Dictionary literals require at least one key-value pair**: `{}` is **not** an empty
> dictionary—it is an empty block (value `Void`, see [§2.9](#_2-9-块表达式)). For an empty
> dictionary, use the constructor `dict.new()`:
>
> ```yaoxiang
> empty = dict.new()                  // ✅ Empty dictionary
> d = { "a": 1 }                       // ✅ Dictionary literal
> wrong = {}                           // ❌ This is not a dictionary; it is an empty block (Void)
> ```
>
> **The criterion is content**: the `Dict` grammar requires at least one `String ':' Expr`; `{}` has
> no content to be based on, so it takes the zero form of block structure. Non-empty forms are
> self-describing by content (`{ "k": v }` has a key-value pair → dictionary)—this is in the same
> vein as `f = { 5 }` being an `Int` value rather than a function: **the type is determined by
> content**.
>
> Set has no literal grammar, no runtime representation—collection types are planned and will be
> completed following the Dict pattern when needs arise (std.set + HeapValue::Set). The landing
> point of List/Dict literals is determined by the contextual type annotation: a bare literal or one
> annotated as `List(T)` lands as a growable list; an `Array(T, N)` annotation acting directly on a
> literal lands as a fixed-size array. Implicit List→Array conversion is forbidden.
>
> Array literal semantics:
>
> - The number of elements must equal N; otherwise E1002 is raised at compile time; an empty literal
>   paired with a non-zero N is also rejected
> - Each element's type must be compatible with T; otherwise E1002 is raised at compile time
> - N's grammatical form: integer literal only (may be negative) or constant name; composite
>   expressions (e.g., `2+1`) are rejected at parse time
> - When N is a symbolic constant (function const parameter, e.g., `Array(Int, n)`), the count check
>   is deferred to the refinement type phase
> - v1 nested array literals (`Array(Array(Int,2),2) = [[1,2],[3,4]]`) are rejected at compile time;
>   per-layer explicit construction is required; recursive landing points are deferred to a later
>   version

#### 1.6.5 List Comprehensions

```
ListComp    ::= '[' Expr ( 'for' Identifier 'in' Expr ( 'if' Expr )? )+ ']'
```

> **#401 (implementation completion)**: `if` filtering and multi-generator clauses are implemented
> starting from this version—the previous grammar was written as `(',' Expr)* ('if' Expr)?`, with no
> implementation corresponding (neither comma-appended expressions nor multiple generators were
> supported), and `if` filtering was entirely missed by the parser (after parsing the iterable, it
> directly expected `']'`, raising `E0010 Expected RBracket, found KwIf`). The convention now
> follows industry-standard semantics: one or more generator clauses, expanding as nested loops;
> each clause may carry at most one `if` filter, whose condition is forced to `Bool` (non-Bool
> raises `E1054`); later clauses' iterable/condition may reference iteration variables bound by
> earlier clauses.
>
> **Behavior tightening (migration record)**: the iteration variable grammar was always
> `'for' Identifier 'in'`, but the old implementation had the pattern go through full pratt
> parsing—after `'in'` was registered as an infix operator, `x` would swallow `in items` as a
> membership expression. After the fix, non-identifier patterns fail to parse directly, no longer
> falling back to `_` like the old implementation did (silently swallowing errors). Impact: forms
> like `[x for (a, b) in pairs]` that could be parsed before now raise errors—that form never had
> defined behavior (the variable was always `_`), so the tightening direction is correct with no
> semantic migration cost.

#### 1.6.6 Membership Test

```
Membership  ::= Expr 'in' Expr
```

> `in` is a binary relational operator that returns `Bool`—`true` on hit, `false` on miss, and does
> not raise an error. The semantic split: `[]` asserts existence and retrieves the value (raising an
> error on failure), while `in` asks whether it exists (miss is a normal `false`). Right-operand
> coverage: List / Array / Dict (key set) / Tuple / String (substring) / Range (interval). `in` is a
> first-class Hoare predicate, and serves as the basis of compile-time provable propositions in the
> refinement type phase. (Set is removed from the right-operand list—Set has no runtime
> representation, see §1.6.4)

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

> **Unary prefix operators** (`!` `-` `+`) bind tightly: only lower than call and member access, and
> higher than all binary operators. Therefore `!a == b` ≡ `(!a) == b` (Zig-style semantics); `!` is
> a pure unary operation that does not participate in short-circuit control flow, and is orthogonal
> to the `and`/`or` keywords (short-circuit) (authoritatively defined by RFC-010).
>
> **Range binding power**: `..` has binding power (6, 7)—left 6 is lower than addition (7), right 7
> swallows addition but not sibling-level `..`. Comparison before and after the change:
>
> | Expression   | Before change (level 1, right-associative)                                                   | After change ((6,7), left-associative)                                   |
> | ------------ | -------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
> | `x in 1..10` | `x in 1..10` (`in` right-operand level 4, `..` level 1 cannot swallow, actually unparseable) | `x in (1..10)`—the interval is the `in` right operand as a whole         |
> | `0..n+2`     | `(0..n)+2` (right-associative trap: upper bound is eaten, `for` loop directly E3004)         | `0..(n+2)`—upper bound is an arithmetic expression                       |
> | `a == b..c`  | `a == (b..c)` (`..` level 1 < `==` level 3, naturally whole)                                 | `a == (b..c)`—**semantics unchanged**, `..` still above comparison level |
> | `1..2*3`     | `(1..2)*3`                                                                                   | `1..(2*3)`—upper bound is an arithmetic expression                       |
> | `a..b..c`    | `a..(b..c)` (right-associative chain, meaningless Range nesting)                             | `(a..b)..c`—**step form** (`c` is the step)                              |
>
> Net effect: compound upper bounds like `for i in 0..n+2` change from "parses successfully but
> E3004" to "directly usable"; `x in 1..10` changes from "unparseable" to "interval check";
> `a..b..c` changes from "meaningless nesting" to "step component". Level 6 falls between `+`
> (level 5) and `<<` (level 7), following the mathematical convention: an interval is a tightly
> bound construct, so the upper bound is naturally a complete arithmetic expression.

### 2.3 Function Calls

```
FnCall      ::= Expr '(' ArgList? ')'
ArgList     ::= Expr (',' Expr)* (',' NamedArg)* | NamedArg (',' NamedArg)*
NamedArg    ::= Identifier '=' Expr
```

Named arguments use `name = value` (RFC-010 §Function Definitions, RFC-011 §Construction Forms).
Positional arguments must come before named arguments; order-specified parameters may appear in any
order among named arguments:

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b

add(3, 5)          // Positional
add(a = 3, b = 5)  // Named
add(b = 5, a = 3)  // Any order
add(3, b = 5)      // Mixed, positional first
```

A wrong name in a named argument raises **E1014**; the same formal parameter specified both
positionally and by name raises **E1015**; mismatched count raises **E1010** (RFC-013).

**Unnamed parameters and named arguments**: signature parameter positions allow a bare type form
(`mk: (Int, Int) -> Int`), which is an **unnamed typed parameter**—type constraints take effect by
position, the name belongs to the implementation (the lambda header provides one, see
[RFC-007](../../rfc/accepted/007-function-syntax-unification.md) shorthand rules), and does not
enter the contract. Unnamed parameters cannot be referenced by named arguments (there is no name to
reference), and can only be passed by position; the way to put a name into the contract is
`mk: (a: Int, b: Int) -> Int`. A bare identifier must be a declared type; failing to parse a type
raises an error (see [type-system §3.5](type-system.md#35-函数类型)).

### 2.4 Member Access

```
MemberAccess::= Expr '.' Identifier
```

### 2.5 Index Access

```
IndexAccess ::= Expr '[' Expr ']'
```

> **Three-layer semantics (RFC-011b)**: `a[i]` branches by the type of `a`—① built-in containers
> (List/Vec/Array/Dict/Tuple) take the native index instruction (fast path); ② user types
> implementing the `Index` interface dispatch to their `index` method (`Index(Grid, Int, Float)`
> instantiation in the type body + `Grid.index` method); ③ RFC-004's `f[0]` positional binding only
> exists in binding declarations, not in this grammar. The key of multi-dimensional indexing
> `a[0, 1]` is packed as a tuple. Other types that do not implement `Index` are rejected at the type
> layer.

### 2.6 Type Cast

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

> **Variant set requirement for variant destructuring** (RFC-010b): `EnumPattern` (variant names
> like `ok(v)`, `some(x)` resolved against the scrutinee) requires the scrutinee's type to be a
> **sum type whose variant set is in scope**. The variant set only enters the checker through a type
> definition or a `use` import—`Result`/`Option` are defined in `std.result`/`std.option`, and
> `use std.result` / `use std.option` must be issued before use (the entire module form and the
> grouped `use std.{...}` form are equivalent); otherwise, variant destructuring raises E1002.
> Exhaustiveness checking is from the same source: a default arm is exempt from exhaustiveness;
> without a default arm, the check is done against the full variant set.

### 2.9 Block Expressions

```
Block       ::= '{' Stmt* Expr? '}'
```

> **Statement termination rules**: the separator and line-break behavior between Stmts (explicit `;`
> separator, newline termination, line-continuation exceptions, line-leading `(`/`[` never merging)
> are defined by [RFC-038](../../rfc/accepted/038-statement-termination.md).

#### 2.9.1 Three Forms of `{`

`{` has exactly three interpretations in expression position, decided in a single pass based on
**content**:

| Form             | Notation          | Type                 | Example          |
| ---------------- | ----------------- | -------------------- | ---------------- |
| **Empty block**  | `{}`              | `Void`               | `x: Void = {}`   |
| **Dict literal** | `{ "k": v, ... }` | `Dict(K, V)`         | `d = { "a": 1 }` |
| **Block**        | `{ Stmt* Expr? }` | Tail expression type | `y = { 1 + 1 }`  |

**Decision order**:

1. `{}` (no content) → **Empty block**, value `Void`
2. First element is a `String ':' Expr` key-value pair → **Dict literal**
3. Otherwise → **Block**, value given by the tail expression

> **Why `{}` is not an empty dictionary**: the "emptiness" of an empty dictionary cannot describe
> itself (it could be either `Dict(K, V)` or an empty block), and the dict grammar
> [§1.6.4](#_1-6-4-集合) requires at least one key-value pair. When there is no content to base on,
> the zero form of block structure is taken: this is consistent with `unsafe {}` / `spawn {}`, and
> introduces no special case. For an empty dictionary, use `dict.new()`.
>
> **Why functions need an annotation**: `f = { stmt }` is a **value** (tail expression type), not a
> function. To define a function, write the Fn annotation explicitly: `f: () -> Int = { 5 }`. This
> follows the same principle as the dictionary: **the type is determined by content**, not by the
> presence of an annotation. (This rule is in
> [RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md) Appendix D.)
>
> **Unified semantics**: the value of all `{}` blocks is given by the **tail expression**; `return`
> is a non-local exit of type `Never`.

| Block type    | Value exit      | Empty block `{}` |
| ------------- | --------------- | ---------------- |
| Ordinary `{}` | Tail expression | `Void`           |
| `unsafe {}`   | Tail expression | `Void`           |
| `spawn {}`    | Tail expression | `Void`           |

**Core principles** (see [RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md) for
details):

- **The value of a block = the tail expression** (the last expression), the only exit, no exception
- When the last item is an **assignment statement**, the block value is `Void`; if you want `Void`,
  write it explicitly
- **`return` exits the nearest function boundary** (passing through any block, not "returning to the
  block"), with type `Never`; `Never <: T` holds for any type (explosion principle), so it may
  appear at any return type position. The module initialization layer (Script top level) has no
  function boundary, and `return` is rejected at compile time there (`E1109`, see §3.11)
- Expression form `= expr` directly gives the value

```yaoxiang
// Ordinary {} block: value given by the tail expression
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

// return: passes through the block, exits the function
f: (n: Int) -> Int = {
    if n < 0 {
        return 0     // Passes through if and the function body, exits the function
    }
    n * 2            // Tail expression
}
```

#### Is `name = { ... }` a Function or a Block Value?

`name = { ... }` may be either a function definition (RFC-007 "Empty-Parameter Shortest Form") or a
block-value binding. It is decided by **annotation priority, function as default** (RFC-010a
Appendix D):

| Case                              | Result          | Example                            |
| --------------------------------- | --------------- | ---------------------------------- |
| Value is a Lambda (`=>`)          | Function        | `f = () => 5` → `f()` = 5          |
| Annotation is a function type     | Function        | `f: () -> Int = { 5 }` → `f()` = 5 |
| Annotation is a non-function type | **Block value** | `x: Int = { 5 }` → `x` = 5         |
| No annotation                     | Function        | `f = { 5 }` → `f()` = 5            |

**The annotation is the type**: `x: Int = ...` declares that `x` is `Int`, so `{ ... }` evaluates to
`Int`; `f: () -> Int = ...` declares that `f` is a function, so `{ ... }` is the function body.

To evaluate `{ ... }` on the spot, **just write the target type** (no new syntax needed):

```yaoxiang
// Block value: evaluated immediately
x: Int = {
    y = 5
    y            // x = 5
}

// Function: default without annotation
f = { 5 }        // f() = 5
```

#### Nested Function Types: Currying or Returning a Function?

A nested function type on the right of `->` has two readings, distinguished by **parentheses**
(RFC-004):

| Notation                        | Meaning                  | Call                    |
| ------------------------------- | ------------------------ | ----------------------- |
| `(a: Int) -> (b: Int) -> Int`   | **Currying**             | `f(1)(2)`               |
| `(a: Int) -> ((b: Int) -> Int)` | **Returning a function** | `g(1)` gives a function |

The principle: **the annotation is the type**. `g: (a: Int) -> ((b: Int) -> Int)` declares that
`g(1) : (b: Int) -> Int`, so `g(1)` must **be** that function, not "the next argument segment."

```yaoxiang
// Currying: two argument segments given one at a time
add: (a: Int) -> (b: Int) -> Int = { a + b }
add(1)(2)        // → 3

// Returning a function: the outer one segment of arguments, the returned is the function
adder: (n: Int) -> ((x: Int) -> Int) = (x) => x + n
adder(10)(5)     // → 15
h = adder(10)    // The returned function can be stored in a variable, passed around
```

Unparenthesized nested `Fn` is always curried (including RFC-011's type-parameter form):

```yaoxiang
identity: (T: Type) -> (x: T) -> T = (x) => x   // Currying
identity(5)      // → 5
```

**Type checking**: parentheses declare the return type, and the body must produce that type.
`f: () -> (() -> Int) = { 7 }` is an error—`() -> Int` is expected, but `Int` is obtained (E1002).

### 2.10 Lambda Expressions

```
Lambda      ::= '(' ParamList? ')' '=>' Expr
            |  '(' ParamList? ')' '=>' Block
```

### 2.11 Error Propagation Operator

```
ErrorPropagate ::= Expr '?'
```

The `?` operator is a postfix operator with the same precedence as `.`. For the `Result(T, E)` type:

- For `Ok(v)`, extracts the value `v` and continues
- For `Err(e)`, propagates the error upward (`return Err(e)`)

```yaoxiang
process: (data: Data) -> Result(Data, Error) = {
    validated = validate(data)?     // Extract value on success, propagate on failure
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

// Step form (third component, default 1)
for i in 0..10..2 { print(i) }  // 0, 2, 4, 6, 8
for i in 10..0..(-2) { print(i) }  // 10, 8, 6, 4, 2
```

> **Step semantics**: in `a..b..c`, `c` is the step. A literal `c = 0` is rejected at compile time;
> a dynamic `c` is zero-checked at runtime (E6001 family; will be promoted to Result after the error
> system lands). `c < 0` is legal, and the interval direction is reversed with the sign
> (`10..0..(-2)` is descending).

### 2.13 ref Expressions

```
RefExpr     ::= 'ref' Expr
```

`ref` creates shared ownership. The compiler automatically chooses Rc (single-task) or Arc
(cross-task); users do not need to care about the implementation details.

```yaoxiang
data = ref heavy_data
spawn { use(data) }   // Cross-task: the compiler automatically picks Arc
```

### 2.14 unsafe Expressions

```
UnsafeExpr  ::= 'unsafe' Block
```

The `unsafe` block is used to define opaque types and operate on raw pointers. Use `return` to
return the type definition to the enclosing scope.

**Semantics**:

- Types can be defined and raw pointers operated on within `unsafe {}`
- The returned type is usable outside the `unsafe {}` block
- Accessing fields of the type requires unsafe permission

```yaoxiang
// Define an opaque type in an unsafe block
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
- An inner scope can access variables from an outer scope
- An outer scope cannot access variables from an inner scope
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

- `x = value`: search `x` outward along the scope chain; if found, assign; if not, declare anew
- `mut x = value`: explicit new mutable declaration, prohibited from having the same name as an
  outer binding
- Within the same scope, any name can be declared only once

> **Detailed definition**: the complete rules for scopes, variable declaration, and shadowing
> mechanisms are detailed in the [Module System Specification](modules.md#第四章作用域).

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

**Semantics**: `return` is a **non-local exit**, exiting the nearest **function boundary** (passing
through any block—including `if` / `while` / `for` / `match` / bare block / `spawn` / `unsafe`),
handing the value to the caller. It does **not "return to the block."**

**Type**: `return e : Never` (`e : T`). `Never <: T'` holds for any `T'` (explosion principle, see
[Type System §2.2](type-system.md)), so `return` can appear at any return type position without
additional rules to constrain it.

**Relationship with block evaluation**: a block's value is always the **tail expression** (see
§2.9). `{ return n }` as a block has value `n`, type `Never`; at the same time, `return`'s effect is
to exit the function. **Both hold simultaneously**, coexisting through the explosion principle.

`return` and the tail expression together enable "early return," without any extra rule requiring
`return` to specifically designate the function—see
[RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md).

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 {
        return 1          // Passes through if, exits the function (type Never)
    }
    n * factorial(n - 1)  // Tail expression = block value
}
```

### 3.4 break Statement

```
BreakStmt   ::= 'break'
```

**Semantics**: immediately terminates the innermost `while`/`for` loop in which it appears; control
transfers to after the loop body.

- **Exits only the nearest level**: `break` always acts on the innermost loop containing it. In
  nested loops, when one needs to break out of multiple levels at once, extract the inner loop as a
  function and use `return` to return, or use a flag (break/continue carries no label; if loop
  labels are introduced in the future, the syntax will follow the loop declaration side through the
  RFC process, and be decided together with the proof pipeline's multi-exit design)
- **Only within a loop body**: `break` can only appear within a `while`/`for` loop body (including
  blocks/if/match nested within the body); appearing outside a loop is a compile-time error (E1102
  `'break' outside of a loop`)
- **Does not affect termination proofs**: `break` does not participate in the termination
  argument—it provides no measure, nor does it constitute a decreasing step of a measure; the loop's
  termination obligation is unrelated to `break` and is triggered by refinement types (see
  [type-system §8.4](type-system.md#84-terminates终止测度谓词))
- **Borrowing semantics**: the control-flow edge of break participates in the structural cut of the
  reverse BFS liveness analysis of RFC-009a (the skipped iteration does not participate in back-edge
  liveness derivation)

```yaoxiang
mut i = 0
while i < 10 {
    i = i + 1
    if i == 3 {
        break              // Control transfers to after the loop, i == 3
    }
}

// Nested loop: break only exits the inner one
while j < 3 {
    while k < 10 {
        if k == 2 { break }    // Only terminates the inner loop
    }
    j = j + 1                  // Each outer iteration executes up to here
}
```

### 3.5 continue Statement

```
ContinueStmt::= 'continue'
```

**Semantics**: skips the remaining statements in the current iteration and directly enters the next
round of the innermost loop in which it appears—`while` returns to the condition re-check, `for`
takes the next element.

- **Acts only on the nearest level**: same as `break`; carries no label
- **Only within a loop body**: appearing outside a loop is a compile-time error (E1102)

```yaoxiang
mut sum = 0
mut n = 0
while n < 5 {
    n = n + 1
    if n == 3 {
        continue           // Skips the accumulation below; n == 3 is not counted
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

**Key point**: after each iteration ends, the binding created in that iteration is destroyed. The
next iteration is a brand new binding, with no relation to the previous iteration's binding.

#### 3.9.2 Difference Between `for` and `for mut`

| Syntax              | Loop variable mutability | Description                                    |
| ------------------- | ------------------------ | ---------------------------------------------- |
| `for i in 1..5`     | Immutable                | Cannot modify the binding within the loop body |
| `for mut i in 1..5` | Mutable                  | May modify the binding within the loop body    |

```yaoxiang
// Legal: each iteration binds a new value; modification not needed
for i in 1..5 {
    print(i)  // Read the value of i
}

// Error: immutable binding; cannot modify
for i in 1..5 {
    i = i + 1  // Error: cannot modify an immutable binding
}

// Legal: use for mut to allow modifying the binding
for mut i in 1..5 {
    i = i + 1  // Modification allowed
}
```

#### 3.9.3 Shadowing Check

YaoXiang forbids variable shadowing. The for loop variable cannot share a name with a variable in an
outer scope:

```yaoxiang
// Error: i has already been declared outside
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

This rule applies to all code blocks; see [4.3 Shadowing Rules](modules.md#43-遮蔽规则) for details.

#### 3.9.4 Comparison with Other Languages

| Language | For loop variable semantics                                |
| -------- | ---------------------------------------------------------- |
| YaoXiang | Each iteration binds a new value                           |
| Rust     | Modifies the same variable (requires mut)                  |
| Python   | Modifies the same variable (no mut needed)                 |
| C/C++    | Modifies the same variable (requires pointer or reference) |

**Design rationale**: YaoXiang adopts the binding semantics because:

1. **More aligned with natural semantics** In natural language, "for every element x in the
   collection" means that each x is an independent individual. YaoXiang's `for i in 1..5` is read as
   "for every i in 1 to 5," and the i of each iteration is a brand new binding, consistent with
   human intuitive understanding.
2. **Avoids accidental modification** The default immutable binding semantics means the loop
   variable cannot be accidentally modified within the loop body. There is no need to worry about
   writing `i = ...` somewhere in a complex loop body and causing a hard-to-trace bug.
3. **High-performance solutions are within reach** When you really need to reuse a variable across
   iterations (e.g., accumulator, cache), use `for mut` to switch to mutable binding mode. This is
   clearer than implicit shared state—the intent is expressed explicitly through syntax, rather than
   hidden in runtime behavior.

### 3.10 spawn Statement

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' 'mut'? Identifier 'in' Expr '{' Expr '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
```

**spawn block**: explicitly declares a concurrent region; expressions within the block execute
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

- Block body references an outer variable = **Move value capture**: the value is snapshotted into
  the closure environment at the spawn creation point, and the block body reads it through env
  (LoadUpvalue)
- **Primitives** (Int/Float/Bool/Char) are value-copied; outer variables are not affected
- **Handle types** (Struct/String/List, etc.) snapshot = handle copy, sharing the underlying object;
  the Embedded runtime (default) has the same thread and heap, so handles are valid
- Sharing between multiple tasks requires explicit `ref` (§2.13, the compiler automatically picks
  Rc/Arc)
- `return` in the block references the outer variable and captures it as well

```yaoxiang
t1 = 1 + 1
t2 = 2 + 2
result = spawn {
    return t1 + t2    // t1/t2 value-captured, result == 6
}
```

---

### 3.11 Program Entry and Top-Level Statements

In the compiler's eyes, a source file plays one of two roles, **determined by the presence of
`yaoxiang.toml`**:

| Role       | Determination                              | Program body                                                                        |
| ---------- | ------------------------------------------ | ----------------------------------------------------------------------------------- |
| **Script** | Single-file direct run, no `yaoxiang.toml` | **Top-level statements** (executed in written order); `main` is an ordinary binding |
| **Bin**    | `yaoxiang.toml` exists                     | **`main` function**; top level may not contain executable statements                |

#### Script: Top-Level Statements Are the Program

Without a manifest, the file is a "script" and **top-level statements are executed in written
order** (the top-level **binding initialization** runs first as a whole, see
"[Top-Level Binding Initialization](#top-level-binding-initialization)" below):

```yaoxiang
use std.io
io.println("hello")          // Execute directly
x: Int = { 42 }              // Top-level binding: runtime initialization
io.println(x)                // 42
```

`main` is **not special** in this mode—it is just an ordinary binding. To make `main` run, you must
call it explicitly:

```yaoxiang
use std.io
main: () -> Void = { io.println("only runs if called") }

main()                       // ← This line must be written
```

> **Why not auto-call `main`?** Top-level statements are already the program body. If `main` were
> also called implicitly, a script that explicitly wrote `main()` would execute twice. The two rules
> cannot coexist, so under Script, there is only one execution entry: "top-level statements."
>
> The top level also **must not use `return`**: the Script top level is the module initialization
> layer, with no function boundary to exit (RFC-010a rule ②); the compile-time error is
> `E1109`—rather than silently terminating initialization at runtime and skipping the remaining
> top-level statements. To end a piece of top-level logic early, put it inside a function and
> control it at the call site.

#### Bin: `main` Is the Entry

With a manifest, the file is an "executable target," in which case:

- A binding named `main` must be defined, value or function both allowed (decision in #388: the
  entry is determined by the existence of the binding—a function is already a value, and the two
  only differ in evaluation strategy): a function main is called with zero arguments at the entry
  phase; a value main is evaluated during the initialization phase, which is execution itself
- The top level may not contain executable statements—the program body is `main`

```yaoxiang
main: () -> Void = {
    print("hello")
}
```

A value main is also legal—evaluation during the initialization phase is execution itself, and
evaluation happens in the initialization sequence (in global binding topological order, independent
bindings in source order):

```yaoxiang
main = {
    print("hello")
}
```

Missing `main` is a compile error (without `main`, all functions are unreachable). A non-callable
value main (e.g., `main: Int = 5`) is legal but has no observable effect—analogous to Rust's empty
`fn main() {}`.

> **Library files**: a file used by other files via `use`, or pointed to by `[lib].path` /
> `[exports]`, does not require `main`—it is not a program entry.

#### Top-Level Binding Initialization

The initialization value of a top-level binding **is evaluated at runtime**, and need not be a
compile-time constant:

```yaoxiang
answer: Int = { 42 }              // Block value
inc: (Int) -> Int = (x) => x + 1
computed: Int = inc(41)           // Function call
```

Initialization runs in **dependency order**, independent of written order (dependencies initialize
first; independent bindings in declaration order):

```yaoxiang
derived: Int = base * 3           // References base declared later
base: Int = 7                     // Initialized first (topological sort)
```

And **all binding initialization happens before top-level statements execute**—side effects inside
binding initializers occur before any top-level statement, regardless of how the two are interleaved
in writing:

```yaoxiang
print("a")                        // Executes later
x = { print("b") }                // Executes first (binding initialization as a whole runs first)
print("c")                        // Output order: b, a, c
```

Cyclic dependencies are a compile error (the names on the cycle are listed):

```yaoxiang
a: Int = b + 1
b: Int = a + 1                    // Error: a → b → a
```

> **Design basis**: RFC-029f (file role model), RFC-010a Appendix D (block binding decision).

---

## Appendix: Syntax Quick Reference

### A.0 Keywords (17)

```
use     spawn   ref     mut
if      else    match   while   for
in      return  break   continue
as      unsafe  and     or
```

`type` is no longer a keyword (RFC-010, use `Name: Type = { ... }` instead); `pub` has been entirely
removed with RFC-029g, falling back to an ordinary identifier. Literal reserved words `true` /
`false` / `void` are in §1.4.1; the meta-type name `Type` and built-in type names `Void` / `Never` /
`Int` / `Float` / `Bool` / `Char` / `String` are in §1.4.3.

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
