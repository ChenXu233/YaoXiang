# Syntax Specification

This document defines the syntax specification of the YaoXiang programming language, including its
lexical structure, grammar rules, and operator precedence.

---

## Chapter 1: Lexical Structure

### 1.1 Source Files

YaoXiang source files must use UTF-8 encoding. Source files usually use the `.yx` extension.

### 1.2 Lexical Token Categories

| Category   | Description                             | Examples                  |
| ---------- | --------------------------------------- | ------------------------- |
| Identifier | Begins with a letter or underscore      | `x`, `_private`, `my_var` |
| Keyword    | Reserved and predefined by the language | `use`, `mut`, `and`       |
| Literal    | Fixed value                             | `42`, `"hello"`, `true`   |
| Operator   | Operation symbol                        | `+`, `-`, `*`, `/`        |
| Separator  | Syntactic separator                     | `(`, `)`, `{`, `}`, `,`   |

### 1.3 Keywords

YaoXiang has **17 keywords** (`keyword_from_str` in `src/frontend/core/lexer/state.rs`, with each
one corresponding to a `TokenKind`):

```
use     spawn   ref     mut
if      else    match   while   for
in      return  break   continue
as      unsafe  and     or
```

`and` / `or` are the **keywords** for logical AND / OR (Zig-style; see level 10 in §2.2); unary
negation is the symbol `!`, which is orthogonal to them.

These keywords have special meanings in every context and cannot be used as identifiers.

> **`type` is no longer a keyword** (RFC-010): use the `Name: Type = { ... }` notation to define a
> type. Both `keyword_from_str` and the `TokenKind` enum header
> (`src/frontend/core/lexer/tokens.rs`) are annotated with “RFC-010: 'type' keyword removed”.
>
> **`pub` is no longer a keyword** (RFC-029g, accepted on 2026-10-02): the language has no
> visibility mechanism. `pub` has been removed wholesale from lexing, the AST, type checking,
> dead-code exemptions, the formatter, and the LSP
> ([RFC-029g](../../rfc/accepted/029g-remove-pub-and-auto-bind.md)). It is now merely an ordinary
> identifier—the old form `pub x = 1` reports undefined name E1001. All top-level bindings are
> importable by default, and modifiers make no difference to visibility
> ([RFC-029](../../rfc/accepted/029-module-semantics.md)).

### 1.4 Reserved Words

YaoXiang's “reserved words” are divided into three layers, recognized at different stages by the
parser and type checker:

#### 1.4.1 Literal Reserved Words

These literal identifiers have dedicated tokens in the parser and cannot be used as ordinary
identifiers:

| Identifier | Owning Type | Description                                                                                                   |
| ---------- | ----------- | ------------------------------------------------------------------------------------------------------------- |
| `true`     | Bool        | Boolean true value                                                                                            |
| `false`    | Bool        | Boolean false value                                                                                           |
| `void`     | Void        | Void literal (Unit value). Lowercase `void` is a value literal; uppercase `Void` is a type name (see §1.4.3). |

> **`Type` is not in this layer**: it does **not** have a dedicated `TokenKind` (it is absent from
> the `TokenKind` enum in `src/frontend/core/lexer/tokens.rs`, and `keyword_from_str` has no
> corresponding branch). The parser treats it as an ordinary identifier, while the type checker
> recognizes it as a meta type in type positions. Therefore, `Type` can be shadowed by a local
> binding in expression positions. It is a **meta type name**, not a keyword.

#### 1.4.2 Variant Names

Variant names are recognized by the parser in **pattern** contexts (bare names are resolved against
the variant set of the scrutinee type; see §2.8). In **expression** contexts, constructing a variant
requires type qualification—`Result(Int, String).ok(5)`, `Option(T).some(v)`, or `Color.green()`
(under RFC-010's record types and types, variant constructors are function-typed fields in a type
definition; a bare name such as `ok(5)` is not a constructor call).

| Variant Name | Owning Type | Description                        |
| ------------ | ----------- | ---------------------------------- |
| `some(T)`    | Option      | Constructs an Option value variant |
| `ok(T)`      | Result      | Result success variant             |
| `err(E)`     | Result      | Result error variant               |

#### 1.4.3 Built-in Type Names

The following type names are pre-registered with the type checker and can be used in type positions
without importing them. The parser treats them as ordinary identifiers—they are **not reserved words
and can be shadowed by local bindings (not recommended)**.

| Type Name | Logical Correspondence | Description                                                                                                                                                   |
| --------- | ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Void`    | ⊤ (true/Unit)          | Zero-field product type with exactly one inhabitant (the `void` literal; see §1.4.1)                                                                          |
| `Never`   | ⊥ (false/empty type)   | A type and set with zero variants and zero inhabitants. No expression can produce a `Never` value. `Never <: T` holds for every `T` (principle of explosion). |
| `Int`     | —                      | Signed integer                                                                                                                                                |
| `Float`   | —                      | Floating-point number                                                                                                                                         |
| `Bool`    | —                      | Boolean value: `true` / `false`                                                                                                                               |
| `Char`    | —                      | Unicode character                                                                                                                                             |
| `String`  | —                      | String                                                                                                                                                        |

### 1.5 Identifiers

Identifiers begin with a letter or underscore. Subsequent characters may be letters, digits, or
underscores. Identifiers are case-sensitive.

Special identifiers:

- `_` is used as a placeholder, indicating that a value should be ignored.
- Identifiers beginning with an underscore denote private members.

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
Array       ::= '[' Expr (',' Expr)* ']'   // When the target type annotation is Array(T, N), the literal resolves to a fixed-length array
```

> **Dictionary literals require at least one key-value pair**: `{}` is **not** an empty
> dictionary—it is an empty block (value `Void`; see [§2.9](#_2-9-块表达式)). Use the constructor
> `dict.new()` for an empty dictionary:
>
> ```yaoxiang
> empty = dict.new()                  // ✅ Empty dictionary
> d = { "a": 1 }                       // ✅ Dictionary literal
> wrong = {}                           // ❌ Not a dictionary; an empty block (Void)
> ```
>
> **The criterion is the content**: the `Dict` grammar requires at least one `String ':' Expr`.
> Since `{}` has no content on which to base a decision, it takes the zero form of the block
> structure. Non-empty forms are self-describing through their content (`{ "k": v }` has a key-value
> pair → dictionary). This is the same reason that `f = { 5 }` is an `Int` value rather than a
> function: **the type is determined by the content**.

> Sets have no literal grammar and no runtime representation. When they are required during type
> planning, they will be completed following the Dict pattern (`std.set` + `HeapValue::Set`). The
> resolution of List/Dict literals is determined by the contextual type annotation: bare literals
> and `List(T)` resolve to growable lists, while an `Array(T, N)` annotation applied directly to a
> literal resolves to a fixed-length array. Implicit List→Array conversion is forbidden.
>
> Array literal semantics:
>
> - The number of elements must equal N; otherwise, compilation fails with E1002. An empty literal
>   paired with a nonzero N is likewise rejected.
> - Every element type must be compatible with T; otherwise, compilation fails with E1002.
> - The grammatical form of N is limited to an integer literal (which may be negative) or a constant
>   name; composite expressions such as `2+1` are rejected during parsing.
> - When N is a symbolic constant (such as a function `const` parameter in `Array(Int, n)`), count
>   validation is deferred to the refinement type stage.
> - In v1, nested array literals (`Array(Array(Int,2),2) = [[1,2],[3,4]]`) are rejected at compile
>   time and must be constructed explicitly layer by layer. Recursive resolution is reserved for a
>   future version.

#### 1.6.5 List Comprehensions

```
ListComp    ::= '[' Expr ( 'for' Identifier 'in' Expr ( 'if' Expr )? )+ ']'
```

> **#401 (implementation completion)**: `if` filters and multiple generator clauses are supported
> starting with this version. The previous grammar was `(',' Expr)* ('if' Expr)?`, which had no
> implementation counterpart (comma-appended expressions and multiple generators were both
> unsupported), and the parser omitted the `if` filter entirely (after parsing the iterable, it
> directly expected `']'`, reporting `E0010 Expected RBracket, found KwIf`). The final semantics
> follow common industry practice: there must be one or more generator clauses, expanded as nested
> loops; each clause may carry at most one `if` filter, whose condition must be `Bool` (a non-Bool
> value reports `E1054`); the iterable or condition of a later clause may reference iteration
> variables bound by earlier clauses.

> **Behavior tightening (migration record)**: the iteration variable grammar has always been
> `'for' Identifier 'in'`, but the old implementation parsed the pattern using the full Pratt
> parser—after `'in'` was registered as an infix operator, `x` would absorb `in items` as a
> membership expression. After the fix, a non-identifier pattern fails to parse directly and no
> longer falls back to `_` to silently swallow the error. Impact: forms such as
> `[x for (a, b) in pairs]` that previously parsed now produce an error. This form never had defined
> behavior (the variable was always `_`), so the tightening is correct and entails no semantic
> migration cost.

#### 1.6.6 Membership Tests

```
Membership  ::= Expr 'in' Expr
```

> `in` is a binary relational operator that returns `Bool`—`true` when found and `false` when not
> found, without reporting an error. The semantics are divided as follows: `[]` asserts existence
> and retrieves a value (failure is an error), whereas `in` asks whether something exists (a miss is
> a normal `false`). Right-hand operand coverage: List / Array / Dict (key set) / Tuple / String
> (substring) / Range (interval). `in` is a first-class Boolean predicate and serves as the
> foundation for compile-time provable propositions during the refinement type stage. (Sets are
> absent from the right-hand operand list—sets have no runtime representation; see §1.6.4.)

### 1.7 Comments

```
// Single-line comment

/* Multi-line comment
   can span multiple lines */
```

### 1.8 Indentation Rules

Code must use four spaces for indentation; Tab characters are forbidden. This is a mandatory syntax
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

> **Unary prefix operators** (`!` `-` `+`) bind tightly: they bind less tightly only than calls and
> member access, and more tightly than every binary operator. Therefore, `!a == b` ≡ `(!a) == b`
> (Zig-style semantics); `!` is a purely unary operation, does not participate in short-circuiting
> control flow, and is orthogonal to the `and`/`or` keywords (which short-circuit) (RFC-010 provides
> the authoritative definition).

> **Range binding strength**: `..` has binding powers (6, 7)—the left power 6 is lower than addition
> (7), while the right power 7 absorbs addition but not a same-level `..`. Before and after the
> change:
>
> | Expression   | Before the change (level 1, right-associative)                                                                      | After the change ((6,7), left-associative)                                               |
> | ------------ | ------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
> | `x in 1..10` | `x in 1..10` (`in`'s right operand has level 4, and level-1 `..` cannot absorb it, so it cannot actually be parsed) | `x in (1..10)`—the entire range is the right operand of `in`                             |
> | `0..n+2`     | `(0..n)+2` (right-associativity trap: the upper bound is consumed, causing the `for` loop to report E3004 directly) | `0..(n+2)`—the upper bound is an arithmetic expression                                   |
> | `a == b..c`  | `a == (b..c)` (`..` has level 1, lower than level-3 `==`, so it naturally forms one unit)                           | `a == (b..c)`—**semantics are unchanged**; `..` still binds more tightly than comparison |
> | `1..2*3`     | `(1..2)*3`                                                                                                          | `1..(2*3)`—the upper bound is an arithmetic expression                                   |
> | `a..b..c`    | `a..(b..c)` (right-associative chain with meaningless nested Ranges)                                                | `(a..b)..c`—**step form** (`c` is the step size)                                         |
>
> The net effect is that the composite upper bound in `for i in 0..n+2` changes from “parses
> successfully but reports E3004” to “directly usable”; `x in 1..10` changes from “cannot be parsed”
> to “range check”; and `a..b..c` changes from “meaningless nesting” to “step component”. Level 6
> falls between `+` (level 5) and `<<` (level 7), following mathematical convention: a range is a
> tightly binding construct, so its upper bound is naturally a complete arithmetic expression.

### 2.3 Function Calls

```
FnCall      ::= Expr '(' ArgList? ')'
ArgList     ::= Expr (',' Expr)* (',' NamedArg)* | NamedArg (',' NamedArg)*
NamedArg    ::= Identifier '=' Expr
```

Named arguments use `name = value` (RFC-010 §Function Definition, RFC-011 §Constructor Form).
Positional arguments must precede named arguments; parameters with explicitly specified order may
appear in any order among named arguments:

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b

add(3, 5)          // Positional
add(a = 3, b = 5)  // Named
add(b = 5, a = 3)  // Any order
add(3, b = 5)      // Mixed, positional first
```

An incorrect named-argument name reports **E1014**, specifying the same formal parameter both
positionally and by name reports **E1015**, and an incorrect argument count reports **E1010**
(RFC-013).

**Unnamed parameters and named arguments**: parameter positions in a signature may use a bare type
form (`mk: (Int, Int) -> Int`). This is an **unnamed typed parameter**—type constraints apply
positionally, while the name belongs to the implementation (provided by the lambda header; see the
shorthand rules in [RFC-007](../../rfc/accepted/007-function-syntax-unification.md)) and does not
enter the contract. An unnamed parameter cannot be referenced by a named argument (there is no name
to reference) and can only be passed positionally. To put a name in the contract, write
`mk: (a: Int, b: Int) -> Int`. A bare identifier must be a declared type; failing to resolve the
type is an error (see [Type System §3.5](type-system.md#35-函数类型)).

**Conversely, named parameters put the name in the contract, and the declaration is authoritative**.
Parameter names in the implementation header (the parameter list after `=>`) are merely local names
in the body and are bound **positionally**—in `f: (a: Int) -> Int = (b) => b`, the implementation
header uses `b`, but callers still write `f(a = 7)`. Type annotations in the implementation header
serve only for comparison; a mismatch with the declaration reports **E1011**. An implementation
header with more parameters than the declaration has runtime parameter positions reports **E1112**
(`f: () -> Int = (a: Int) => a`)—in both cases, the declaration is the authoritative source. The
reverse (an implementation that omits parameters) is not an error: the type layer checks the
declared arity, but positions omitted from the implementation header have no runtime slots, and
arguments landing in those positions are discarded on the IR side (see the gap registry in
[Type System §3.5](type-system.md#35-函数类型)).

### 2.4 Member Access

```
MemberAccess::= Expr '.' Identifier
```

### 2.5 Index Access

```
IndexAccess ::= Expr '[' Expr ']'
```

> **Three-layer semantics (RFC-011b)**: `a[i]` is dispatched according to the type of `a`—① built-in
> containers (List/Vec/Array/Dict/Tuple) use the native index instruction (fast path); ② user types
> that implement the `Index` interface are dispatched to their `index` method (instantiating
> `Index(Grid, Int, Float)` in the type body and providing a `Grid.index` method); ③ the positional
> binding `f[0]` from RFC-004 exists only in binding declarations and does not follow this grammar.
> The key of a multidimensional index such as `a[0, 1]` is packed as a tuple. Other types that do
> not implement `Index` are rejected at the type layer.

### 2.6 Type Casts

```
TypeCast    ::= Expr 'as' TypeExpr
```

> **Implementation status (pre-existing gap, not introduced in this version)**: the above defines
> only the grammar and precedence (§2.4 level 2); the semantics have not yet been implemented. The
> following facts can be verified independently: ① Parsing: the `KwAs` infix arm in
> `parser/pratt/led.rs:81` produces `ast::Expr::Cast`; ② Type checking:
> `typecheck/inference/expressions.rs:3847-3853` performs only a **silent retagging** of
> `Expr::Cast`—`let _ = self.infer_expr(expr)?;` discards the inner result and then directly returns
> the `MonoType` of `target_type`; ③ The repository contains **no** convertibility validation and no
> corresponding diagnostic code (there is no cast entry in `util/diagnostic/codes/`); ④ `ir_gen`
> emits no instructions (`middle/core/ir_gen.rs` only retrieves the span and free variables of
> `Expr::Cast` at `:5111` / `:5918` / `:9381`—the first two are the span arms of `get_expr_span` /
> `expr_span`, and the third is `collect_free_vars_expr`; none of them emits instructions); ⑤
> Therefore, `x as Int` currently neither reports an error nor produces any runtime conversion.

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

> **Variant-set requirement for variant destructuring** (RFC-010b): `EnumPattern` (variant names
> such as `ok(v)` and `some(x)` are resolved against the scrutinee) requires the scrutinee's type to
> be a **sum type whose variant set is present**. A variant set enters the checker only through a
> type definition or a `use` import—`Result`/`Option` are defined by `std.result`/`std.option` and
> must be imported with `use std.result` / `use std.option` before use (whole-module and grouped
> `use std.{...}` forms have equal standing); otherwise, variant destructuring reports E1002.
> Exhaustiveness checking follows the same source: a fallback arm is exempt from exhaustiveness,
> while without a fallback arm, the entire variant set is checked.

### 2.9 Block Expressions

```
Block       ::= '{' Stmt* Expr? '}'
```

> **Statement termination rules**: separation between Stmts and newline behavior (explicit `;`
> separators, newline termination, continuation exceptions, and leading `(`/`[` never being merged)
> are defined by [RFC-038](../../rfc/accepted/038-statement-termination.md).

#### 2.9.1 The Three Forms of `{`

In expression positions, `{` has exactly three interpretations, determined in a single pass by the
**content**:

| Form                   | Notation          | Type                 | Example          |
| ---------------------- | ----------------- | -------------------- | ---------------- |
| **Empty block**        | `{}`              | `Void`               | `x: Void = {}`   |
| **Dictionary literal** | `{ "k": v, ... }` | `Dict(K, V)`         | `d = { "a": 1 }` |
| **Block**              | `{ Stmt* Expr? }` | Tail expression type | `y = { 1 + 1 }`  |

**Decision order**:

1. `{}` (no content) → **empty block**, value `Void`
2. The first element is a `String ':' Expr` key-value pair → **dictionary literal**
3. Otherwise → **block**, with its value provided by the tail expression

> **Why `{}` is not an empty dictionary**: the “emptiness” of an empty dictionary cannot describe
> itself (it could be either `Dict(K, V)` or an empty block), while the dictionary grammar in
> [§1.6.4](#_1-6-4-集合) requires at least one key-value pair. With no content on which to base a
> decision, the zero form of the block structure is used: this is consistent with `unsafe {}` /
> `spawn {}` and introduces no special case. Use `dict.new()` for an empty dictionary.
>
> **Why functions require annotations**: `f = { stmt }` is a **value** (with the type of its tail
> expression), not a function. To define a function, state a Fn annotation: `f: () -> Int = { 5 }`.
> This follows the same principle as dictionaries: **the type is determined by the content**, not by
> whether an annotation exists. (See
> [RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md), Appendix D.)

**Unified semantics**: the values of all `{}` blocks are provided by their **tail expressions**, and
`return` is a nonlocal exit of type `Never`.

| Block Type    | Value Exit      | Empty Block `{}` |
| ------------- | --------------- | ---------------- |
| Ordinary `{}` | Tail expression | `Void`           |
| `unsafe {}`   | Tail expression | `Void`           |
| `spawn {}`    | Tail expression | `Void`           |

**Core principles** (see [RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md) for
details):

- **A block's value = its tail expression** (the final expression); it is the sole exit, with no
  exceptions
- When the final position is an **assignment statement**, the block's value is `Void`; to obtain
  `Void`, explicitly write `Void`
- **`return` exits the nearest function boundary** (passing through every block and not “returning
  to the block”), and has type `Never`; `Never <: T` holds for every type (principle of explosion),
  so it may appear in any position where a return type is expected. The module initialization layer
  (Script top level) has no function boundary, so `return` is rejected at compile time there
  (`E1109`; see §3.11)
- The expression form `= expr` directly provides a value

```yaoxiang
// Ordinary {} block: the tail expression provides the value
result = {
    x = compute()
    x                // The block's value
}

// unsafe {} block: the tail expression provides the type definition
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void
    }
    SqliteDb         // The block's value
}

// spawn {} block: the tail expression provides the result
(a, b) = spawn {
    result1 = fetch("url1"),
    result2 = fetch("url2")
    (result1, result2)   // The block's value
}

// return: passes through blocks and exits the function
f: (n: Int) -> Int = {
    if n < 0 {
        return 0     // Exits through the if and the function body, exiting the function
    }
    n * 2            // Tail expression
}
```

#### Is `name = { ... }` a Function or a Block Value?

`name = { ... }` may be either a function definition (RFC-007 “Minimal Empty Parameter Form”) or a
block-value binding. It is resolved by **annotation first, function by default** (RFC-010a, Appendix
D):

| Case                                  | Result          | Example                            |
| ------------------------------------- | --------------- | ---------------------------------- |
| The value is a Lambda (`=>`)          | Function        | `f = () => 5` → `f()` = 5          |
| The annotation is a function type     | Function        | `f: () -> Int = { 5 }` → `f()` = 5 |
| The annotation is a non-function type | **Block value** | `x: Int = { 5 }` → `x` = 5         |
| No annotation                         | Function        | `f = { 5 }` → `f()` = 5            |

**An annotation is the type**: `x: Int = ...` declares that `x` is an `Int`, so `{ ... }` evaluates
to an `Int`; `f: () -> Int = ...` declares that `f` is a function, so `{ ... }` is the function
body.

To evaluate `{ ... }` immediately, **write the target type** (no new syntax is needed):

```yaoxiang
// Block value: evaluate immediately
x: Int = {
    y = 5
    y            // x = 5
}

// Function: defaults without an annotation
f = { 5 }        // f() = 5
```

#### Nested Function Types: Currying or Returning a Function?

The nested function type on the right-hand side of `->` has two possible readings, distinguished by
**parentheses** (RFC-004):

| Form                            | Meaning                | Call                      |
| ------------------------------- | ---------------------- | ------------------------- |
| `(a: Int) -> (b: Int) -> Int`   | **Currying**           | `f(1)(2)`                 |
| `(a: Int) -> ((b: Int) -> Int)` | **Returns a function** | `g(1)` returns a function |

The basis is: **an annotation is the type**. `g: (a: Int) -> ((b: Int) -> Int)` declares that
`g(1) : (b: Int) -> Int`, so `g(1)` must **be** that function rather than “the next parameter
group”.

```yaoxiang
// Currying: supply the two parameter groups one layer at a time
add: (a: Int) -> (b: Int) -> Int = { a + b }
add(1)(2)        // → 3

// Returning a function: supply the outer parameter group; the return value is the function
adder: (n: Int) -> ((x: Int) -> Int) = (x) => x + n
adder(10)(5)     // → 15
h = adder(10)    // The returned function can be stored in a variable or passed around
```

An unparenthesized nested `Fn` is always curried (including RFC-011's type-parameter form):

```yaoxiang
identity: (T: Type) -> (x: T) -> T = (x) => x   // Currying
identity(5)      // → 5
```

**Type checking**: the parentheses declare the return type, and the body must produce that type.
`f: () -> (() -> Int) = { 7 }` is an error—the expected type is `() -> Int`, but the actual type is
`Int` (E1002).

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

- When `Ok(v)`, extract the value `v` and continue execution
- When `Err(e)`, propagate the error upward (`return Err(e)`)

```yaoxiang
process: (data: Data) -> Result(Data, Error) = {
    validated = validate(data)?     // Extract on success; propagate upward on failure
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

// Range is a value: bind it, pass it, and test membership
r = 1..10
assert.assert(5 in r, "membership")
for i in r { print(i) }

// Step form (third component, default 1)
for i in 0..10..2 { print(i) }  // 0, 2, 4, 6, 8
for i in 10..0..(-2) { print(i) }  // 10, 8, 6, 4, 2
```

> **Step semantics**: in `a..b..c`, `c` is the step size. The literal `c = 0` is rejected at compile
> time; a dynamic `c` receives a runtime zero check (the E6001 family; it will be promoted to Result
> once the error system is implemented). `c < 0` is valid, and the direction of the range is
> reversed according to the sign (`10..0..(-2)` counts downward).

### 2.13 ref Expressions

```
RefExpr     ::= 'ref' Expr
```

`ref` creates shared ownership. The compiler automatically selects Rc (single-task) or Arc
(cross-task), so users do not need to concern themselves with implementation details.

```yaoxiang
data = ref heavy_data
spawn { use(data) }   // Cross-task: the compiler automatically selects Arc
```

### 2.14 unsafe Expressions

```
UnsafeExpr  ::= 'unsafe' Block
```

`unsafe` blocks are used to define opaque types and manipulate raw pointers. Use `return` to return
a type definition to the enclosing scope.

**Semantics**:

- Types and raw pointer operations may be defined inside `unsafe {}`
- A returned type is available outside `unsafe {}`
- Accessing a type's fields requires unsafe permission

```yaoxiang
// Define an opaque type inside an unsafe block
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  // Raw pointer
    }
    return SqliteDb
}

// SqliteDb is available outside the unsafe block
db = sqlite3_open("test.db")
```

### 2.15 Scopes

**Basic rules**:

- Every `{}` block creates a scope
- An inner scope can access variables from outer scopes
- An outer scope cannot access variables from an inner scope
- Variable declarations follow the “assignment first” principle

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

- `x = value`: searches outward through the scope chain for x; if found, it assigns to it, and
  otherwise it creates a new declaration
- `mut x = value`: explicitly declares a new mutable binding and prohibits the same name in an outer
  scope
- Any name may be declared only once within the same scope

> **Detailed definition**: see [Module System Specification](modules.md#第四章作用域) for the
> complete rules on scopes, variable declarations, and shadowing.

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

**Semantics**: `return` is a **nonlocal exit** that exits the nearest **function boundary** (passing
through every block—including `if` / `while` / `for` / `match` / bare blocks / `spawn` / `unsafe`)
and hands its value to the caller. It does **not “return to the block.”**

**Type**: `return e : Never` (`e : T`). `Never <: T'` holds for every `T'` (principle of explosion;
see [Type System §2.2](type-system.md)), so `return` may appear in any position where a return type
is expected, without requiring any additional rule.

**Relationship to block evaluation**: a block's value is always its **tail expression** (see §2.9).
As a block, `{ return n }` has the value `n` and type `Never`; at the same time, `return` exits the
function. **Both facts hold simultaneously**, coexisting through the principle of explosion.

Together, `return` and tail expressions make “early return” possible, without an additional rule
specifically making `return` mean “return from the function”—see
[RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md).

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 {
        return 1          // Exits through the if and the function (type Never)
    }
    n * factorial(n - 1)  // Tail expression = the block's value
}
```

### 3.4 break Statement

```
BreakStmt   ::= 'break'
```

**Semantics**: immediately terminates the innermost containing `while`/`for` loop, transferring
control to after that loop's body.

- **Exits only the nearest level**: `break` always applies to the innermost loop containing it. When
  a nested loop must exit multiple levels at once, extract the inner loop into a function and use
  `return`, or use a flag (`break`/`continue` have no labels; if loop labels are introduced in the
  future, they will follow the loop-declaration-side syntax through the RFC process and be
  adjudicated jointly with the proof pipeline's multi-exit design)
- **Only within a loop body**: `break` may appear only in a `while`/`for` loop body (including
  nested blocks/if/match within the body); use outside a loop produces a compile error (E1102
  `'break' outside of a loop`)
- **Does not affect the termination proof**: `break` does not participate in termination
  arguments—it neither provides a measure nor constitutes a decreasing measure step. The loop's
  termination obligation is independent of `break` and is triggered by refinement types (see
  [Type System §8.5](type-system.md#85-terminates终止测度谓词)).
- **Borrow semantics**: the control-flow edge for `break` participates in the structural severing
  performed by RFC-009a's reverse BFS liveness analysis (iterations exited early do not participate
  in back-edge liveness derivation)

```yaoxiang
mut i = 0
while i < 10 {
    i = i + 1
    if i == 3 {
        break              // Control flow moves to after the loop; i == 3
    }
}

// Nested loops: break exits only the inner loop
while j < 3 {
    while k < 10 {
        if k == 2 { break }    // Terminates only the inner loop
    }
    j = j + 1                  // Executes here on every outer iteration
}
```

### 3.5 continue Statement

```
ContinueStmt::= 'continue'
```

**Semantics**: skips the remaining statements in the current iteration and proceeds directly to the
next iteration of the innermost containing loop—`while` reevaluates its condition, while `for` takes
the next element.

- **Applies only to the nearest level**: as with `break`, it has no label
- **Only within a loop body**: use outside a loop produces a compile error (E1102)

```yaoxiang
mut sum = 0
mut n = 0
while n < 5 {
    n = n + 1
    if n == 3 {
        continue           // Skips the accumulation below; n == 3 is not included
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

YaoXiang's for-loop semantics differ from those of traditional languages: **each iteration binds a
new value rather than modifying the same variable**.

```yaoxiang
// Example: for i in 1..5
for i in 1..5 {
    print(i)
}
```

**Execution process**:

| Iteration | Behavior of the Loop Variable                                                                            |
| --------- | -------------------------------------------------------------------------------------------------------- |
| 1st       | Creates a new binding `i = 1`, executes the body, and prints 1                                           |
| 2nd       | Creates a new binding `i = 2` (the previous binding has been destroyed), executes the body, and prints 2 |
| 3rd       | Creates a new binding `i = 3`, executes the body, and prints 3                                           |
| 4th       | Creates a new binding `i = 4`, executes the body, and prints 4                                           |
| End       | The body ends and the binding is destroyed                                                               |

**Key point**: after each iteration, the binding created during that iteration is destroyed. The
next iteration is an entirely new binding, unrelated to the previous iteration's binding.

#### 3.9.2 Difference between for and for mut

| Syntax              | Loop Variable Mutability | Description                                     |
| ------------------- | ------------------------ | ----------------------------------------------- |
| `for i in 1..5`     | Immutable                | The binding cannot be modified in the loop body |
| `for mut i in 1..5` | Mutable                  | The binding can be modified in the loop body    |

```yaoxiang
// Valid: each iteration binds a new value; no modification is needed
for i in 1..5 {
    print(i)  // Reads the value of i
}

// Error: an immutable binding cannot be modified
for i in 1..5 {
    i = i + 1  // Error: cannot modify an immutable binding
}

// Valid: for mut permits modifying the binding
for mut i in 1..5 {
    i = i + 1  // Mutation is allowed
}
```

#### 3.9.3 Shadowing Checks

YaoXiang prohibits variable shadowing. A for-loop variable cannot have the same name as a variable
in an outer scope:

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

This rule applies to all code blocks. See [4.3 Shadowing Rules](modules.md#43-遮蔽规则).

#### 3.9.4 Comparison with Other Languages

| Language | for-Loop Variable Semantics                                  |
| -------- | ------------------------------------------------------------ |
| YaoXiang | Binds a new value on every iteration                         |
| Rust     | Modifies the same variable (requires mut)                    |
| Python   | Modifies the same variable (does not require mut)            |
| C/C++    | Modifies the same variable (requires a pointer or reference) |

**Design rationale**: YaoXiang adopts binding semantics because:

1. **It better matches natural semantics.** In natural language, “for each element x in a set” means
   that every x is an independent entity. YaoXiang's `for i in 1..5` reads as “for each i from 1
   through 5,” where i is a new binding on every iteration. This agrees with human intuition.

2. **It prevents accidental modification.** Immutable bindings by default mean that the loop
   variable cannot be accidentally modified in the loop body. There is no need to worry about
   accidentally writing `i = ...` somewhere in a complex loop body and creating a bug that is
   difficult to track.

3. **A high-performance solution remains readily available.** When a variable genuinely must be
   reused across iterations—for example, as an accumulator or cache—`for mut` switches to mutable
   binding semantics. This is clearer than implicit shared state: the intent is expressed explicitly
   in the syntax rather than hidden in runtime behavior.

### 3.10 spawn Statement

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' 'mut'? Identifier 'in' Expr '{' Expr '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
```

**spawn block**: explicitly declares a concurrency region, and the expressions within the block
execute concurrently.

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

**A spawn block captures outer variables** (RFC-024 §2.3, value-capture semantics):

- When the block body references an outer variable, this is a **Move value capture**: the value is
  snapshotted into the closure environment at the spawn creation point, and the body reads it
  through the environment (LoadUpvalue)
- **Primitives** (Int/Float/Bool/Char) are copied, leaving the outer variable unaffected
- For **handle types** (Struct/String/List, etc.), snapshotting copies the handle while sharing the
  underlying object; in the Embedded runtime (the default), the same thread and heap are used, so
  the handle remains valid
- Sharing across multiple tasks requires an explicit `ref` (§2.13; the compiler automatically
  selects Rc/Arc)
- A `return` within the block that references outer variables captures them in the same way

```yaoxiang
t1 = 1 + 1
t2 = 2 + 2
result = spawn {
    return t1 + t2    // t1/t2 are value-captured; result == 6
}
```

---

### 3.11 Program Entry and Top-Level Statements

From the compiler's perspective, a source file has two possible roles, **determined by whether
`yaoxiang.toml` exists**:

| Role       | Determination                                       | Program Body                                                                        |
| ---------- | --------------------------------------------------- | ----------------------------------------------------------------------------------- |
| **Script** | A single file run directly, without `yaoxiang.toml` | **Top-level statements** (executed in written order); `main` is an ordinary binding |
| **Bin**    | `yaoxiang.toml` exists                              | **The `main` function**; top-level executable statements are forbidden              |

#### Script: Top-Level Statements Are the Program

Without a manifest, a file is a “script,” and **top-level statements execute in written order**
(top-level **binding initialization** as a whole takes place first; see
“[Top-Level Binding Initialization](#顶层绑定的初始化)” below):

```yaoxiang
use std.io
io.println("hello")          // Executes directly
x: Int = { 42 }              // Top-level binding: initialized at runtime
io.println(x)                // 42
```

In this mode, `main` is **not special**—it is merely an ordinary binding. To run `main`, it must be
called explicitly:

```yaoxiang
use std.io
main: () -> Void = { io.println("only runs if called") }

main()                       // ← This line is required
```

> **Why is `main` not invoked automatically?** Top-level statements are already the program body. If
> `main` were also invoked implicitly, a script that explicitly writes `main()` would execute twice.
> The two rules cannot coexist, so Script has only one execution entry: “top-level statements.”

Top-level code likewise **must not use `return`**: the Script top level is a module initialization
layer with no function boundary to exit (RFC-010a rule 2), so it reports `E1109` at compile time
rather than silently terminating initialization at runtime and skipping the remaining top-level
statements. To end a top-level sequence early, place that logic in a function and control it at the
call site.

#### Bin: `main` Is the Entry Point

When a manifest exists, the file is an “executable target.” In this case:

- A binding named `main` **must** be defined, and it may be either a value or a function (#388
  decision: entry points are determined by the existence of the binding—a function is already a
  value, so the two differ only in evaluation strategy): a function `main` is called with no
  arguments at entry; a value `main` is evaluated during initialization and thus executes
- Top-level executable statements are forbidden—the program body is `main`

```yaoxiang
main: () -> Void = {
    print("hello")
}
```

A value `main` is equally valid—it is evaluated during the initialization sequence (using the global
binding topological order, with independent bindings ordered by source order):

```yaoxiang
main = {
    print("hello")
}
```

A missing `main` is a compile error (without `main`, all functions are unreachable). A non-callable
value `main` (such as `main: Int = 5`) is valid but has no observable effect—this is analogous to
Rust's empty `fn main() {}`.

> **Library files**: files imported by other files through `use`, or files targeted by `[lib].path`
> / `[exports]`, do not require `main`—they are not program entry points.

#### Top-Level Binding Initialization

Top-level binding initializer values are **evaluated at runtime** and are not required to be
compile-time constants:

```yaoxiang
answer: Int = { 42 }              // Block value
inc: (Int) -> Int = (x) => x + 1
computed: Int = inc(41)           // Function call
```

Initialization follows **dependency order**, independently of written order (dependencies are
initialized first; bindings without dependencies follow declaration order):

```yaoxiang
derived: Int = base * 3           // Refers to base declared later
base: Int = 7                     // Initialized first (topological sort)
```

Furthermore, **all binding initialization precedes the execution of top-level statements**—side
effects within binding initializers occur before any top-level statement, regardless of how the two
are interleaved in the source:

```yaoxiang
print("a")                        // Executes later
x = { print("b") }                // Executes first (all binding initialization comes first)
print("c")                        // Output order: b, a, c
```

A cyclic dependency is a compile error (the names on the cycle are listed):

```yaoxiang
a: Int = b + 1
b: Int = a + 1                    // Error: a → b → a
```

> **Design basis**: RFC-029f (file role model) and RFC-010a, Appendix D (block binding decisions).

---

## Appendix: Syntax Quick Reference

### A.0 Keywords (17)

```
use     spawn   ref     mut
if      else    match   while   for
in      return  break   continue
as      unsafe  and     or
```

`type` is no longer a keyword (RFC-010; use `Name: Type = { ... }` instead); `pub` was removed
wholesale under RFC-029g and is once again an ordinary identifier. The literal reserved words `true`
/ `false` / `void` are described in §1.4.1; the meta type name `Type` and built-in type names `Void`
/ `Never` / `Int` / `Float` / `Bool` / `Char` / `String` are described in §1.4.3.

### A.1 Control Flow

```
if Expr Block (else if Expr Block)* (else Block)?
match Expr { MatchArm+ }
while Expr Block
for 'mut'? Identifier 'in' Expr Block
break | continue          // Only within a loop body (§3.4 / §3.5)
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
