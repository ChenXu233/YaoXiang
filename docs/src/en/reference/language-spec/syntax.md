# Syntax Specification

This document defines the syntax specification of the YaoXiang programming language, including its
lexical structure, grammar rules, and operator precedence.

---

## Chapter 1: Lexical Structure

### 1.1 Source Files

YaoXiang source files must use UTF-8 encoding. Source files conventionally use the `.yx` extension.

### 1.2 Lexical Token Categories

| Category   | Description                        | Examples                  |
| ---------- | ---------------------------------- | ------------------------- |
| Identifier | Starts with a letter or underscore | `x`, `_private`, `my_var` |
| Keyword    | Predefined reserved word           | `use`, `mut`, `and`       |
| Literal    | Fixed value                        | `42`, `"hello"`, `true`   |
| Operator   | Operator symbol                    | `+`, `-`, `*`, `/`        |
| Delimiter  | Syntactic delimiter                | `(`, `)`, `{`, `}`, `,`   |

### 1.3 Keywords

YaoXiang has **17 keywords** (`keyword_from_str` in `src/frontend/core/lexer/state.rs` maps each one
to a corresponding `TokenKind`):

```
use     spawn   ref     mut
if      else    match   while   for
in      return  break   continue
as      unsafe  and     or
```

`and` / `or` are **keywords** for logical AND / OR (Zig-style; see level 10 in §2.2); unary NOT is
the symbol `!`, which is orthogonal to them.

These keywords have special meanings in every context and cannot be used as identifiers.

> **`type` is no longer a keyword** (RFC-010): use the `Name: Type = { ... }` notation to define a
> type. Both `keyword_from_str` and the header of the `TokenKind` enum
> (`src/frontend/core/lexer/tokens.rs`) are annotated with “RFC-010: 'type' keyword removed”.
>
> **`pub` is no longer a keyword** (RFC-029g, accepted on 2026-10-02): the language has no
> visibility mechanism. `pub` has been removed entirely from the lexer, AST, type checker, dead-code
> exemptions, formatter, and LSP ([RFC-029g](../../rfc/accepted/029g-remove-pub-and-auto-bind.md)).
> It is now an ordinary identifier—the old form `pub x = 1` reports undefined name E1001. All
> top-level bindings are importable by default, and modifiers make no difference to visibility
> ([RFC-029](../../rfc/accepted/029-module-semantics.md)).

### 1.4 Reserved Words

YaoXiang's "reserved words" are divided into three layers, recognized at different stages by the
parser and type checker:

#### 1.4.1 Literal Reserved Words

The parser has separate tokens for the following literal identifiers, which cannot be used as
ordinary identifiers:

| Identifier | Owning type | Description                                                                                                   |
| ---------- | ----------- | ------------------------------------------------------------------------------------------------------------- |
| `true`     | Bool        | Boolean true value                                                                                            |
| `false`    | Bool        | Boolean false value                                                                                           |
| `void`     | Void        | Void literal (Unit value). Lowercase `void` is a value literal; uppercase `Void` is a type name (see §1.4.3). |

> **`Type` does not belong to this layer**: it has **no** separate `TokenKind` (there is no `Type`
> in the `TokenKind` enum in `src/frontend/core/lexer/tokens.rs`, and `keyword_from_str` has no
> corresponding branch). The parser treats it as an ordinary identifier, while the type checker
> recognizes it as a metatype in type positions. Therefore, `Type` can be shadowed by a local
> binding in expression positions. It is a **metatype name**, not a keyword.

#### 1.4.2 Variant Names

Variant names are recognized by the parser in **pattern** contexts (bare names are resolved against
the variant set of the scrutinee type; see §2.8). In **expression** contexts, constructing a variant
requires a type qualification—`Result(Int, String).ok(5)`, `Option(T).some(v)`, `Color.green()`
(RFC-010 record style and types: variant constructors are function-typed fields in a type
definition, and the bare name `ok(5)` is not a constructor call).

| Variant name | Owning type | Description                        |
| ------------ | ----------- | ---------------------------------- |
| `some(T)`    | Option      | Constructs an Option value variant |
| `ok(T)`      | Result      | Result success variant             |
| `err(E)`     | Result      | Result error variant               |

#### 1.4.3 Built-in Type Names

The type checker pre-registers the following type names, allowing them to be used in type positions
without an import. The parser treats them as ordinary identifiers—**they are not reserved words and
can be shadowed by local bindings (not recommended)**.

| Type name | Logical correspondence | Description                                                                                                                               |
| --------- | ---------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| `Void`    | ⊤ (true/Unit)          | Zero-field product type with exactly one inhabitant (the `void` literal; see §1.4.1)                                                      |
| `Never`   | ⊥ (false/empty type)   | Zero variants and zero inhabitants. No expression can produce a `Never` value. `Never <: T` holds for every `T` (principle of explosion). |
| `Int`     | —                      | Signed integer                                                                                                                            |
| `Float`   | —                      | Floating-point number                                                                                                                     |
| `Bool`    | —                      | Boolean value: `true` / `false`                                                                                                           |
| `Char`    | —                      | Unicode character                                                                                                                         |
| `String`  | —                      | String                                                                                                                                    |

### 1.5 Identifiers

Identifiers start with a letter or underscore; subsequent characters may be letters, digits, or
underscores. Identifiers are case-sensitive.

Special identifiers:

- `_` is used as a placeholder to indicate that a value is ignored.
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
Array       ::= '[' Expr (',' Expr)* ']'   // When the target type is annotated as Array(T, N), the literal resolves to a fixed-length array
```

> **Dictionary literals require at least one key-value pair**: `{}` is **not** an empty
> dictionary—it is an empty block (the value `Void`; see [§2.9](#_1-6-4-集合)). To create an empty
> dictionary, use the constructor `dict.new()`:
>
> ```yaoxiang
> empty = dict.new()                  // ✅ Empty dictionary
> d = { "a": 1 }                       // ✅ Dictionary literal
> wrong = {}                           // ❌ This is not a dictionary but an empty block (Void)
> ```
>
> **The criterion is the content**: the `Dict` grammar requires at least one `String ':' Expr`.
> Since `{}` has no content on which to base a decision, it takes the zero form of the block
> structure. A nonempty form describes itself through its content (`{ "k": v }` has a key-value pair
> → dictionary), just as `f = { 5 }` is an `Int` value rather than a function: **the type is
> determined by the content**.

> Set has no literal grammar or runtime representation—when set requirements arise during planning,
> it will be completed following the Dict pattern (std.set + HeapValue::Set). The landing point of
> List/Dict literals is determined by the contextual type annotation: a bare literal and a `List(T)`
> annotation both resolve to a growable list—the sole type-level name for a growable list is
> `Vec(T)`, and a `List(T)` annotation is normalized from its source name to the same type-level
> target as `Vec(T)`; an `Array(T, N)` annotation applied directly to a literal resolves to a
> fixed-length array. Implicit List→Array conversions are forbidden.
>
> Array literal semantics:
>
> - The number of elements must equal N; otherwise, compile-time error E1002. An empty literal is
>   likewise rejected when N is nonzero.
> - Every element type must be compatible with T; otherwise, compile-time error E1002.
> - The grammatical forms of N are an integer literal (possibly negative), a constant name, or a
>   constant expression beginning with a reference to a compile-time value parameter (with infix
>   arithmetic continuation, such as `2+3` or `N+1`). Count validation uses the folded value (`2+3`
>   and `5` have the same form).
> - Rejected during parsing: floating-point/string seeds (E0010) and incomplete expressions such as
>   `2+` (E0012); an identifier followed by `(` takes the constructor type path—function calls
>   cannot be used as type-level actual arguments (E1003).
> - When N is a symbolic constant (such as a function `const` parameter in `Array(Int, n)`), count
>   validation is deferred to the refinement type phase.
> - In v1, nested array literals (`Array(Array(Int,2),2) = [[1,2],[3,4]]`) are rejected at compile
>   time; each level must be constructed explicitly. Recursive landing points are deferred to a
>   later version.

#### 1.6.5 List Comprehensions

```
ListComp    ::= '[' Expr ( 'for' Identifier 'in' Expr ( 'if' Expr )? )+ ']'
```

> **#401 (implementation completion)**: `if` filters and multiple generator clauses are implemented
> starting with this version. The previous grammar wrote `(',' Expr)* ('if' Expr)?`, which had no
> corresponding implementation (neither comma-appended expressions nor multiple generators were
> supported), and the parser omitted `if` filtering entirely (after parsing the iterable, it
> expected `']'` directly and reported `E0010 Expected RBracket, found KwIf`). The semantics now
> follow common industry practice: there are one or more generator clauses, expanded as nested
> loops; each clause may carry at most one `if` filter, whose condition must be `Bool` (a non-Bool
> condition reports `E1054`); the iterable or condition of a later clause may refer to iteration
> variables bound by earlier clauses.

> **Behavior tightening (migration record)**: the iteration-variable grammar was already
> `'for' Identifier 'in'`, but the old implementation parsed the pattern through the full Pratt
> parser—after `in` was registered as an infix operator, `x` would swallow `in items` as a
> membership expression. After the fix, a non-identifier pattern fails to parse directly and no
> longer falls back to `_` as the old implementation did (silently swallowing the error). Impact:
> forms such as `[x for (a, b) in pairs]` that could previously be parsed now produce an error. This
> form never had defined behavior (the variable was always `_`), so tightening it is correct and
> entails no semantic migration cost.

#### 1.6.6 Membership Tests

```
Membership  ::= Expr 'in' Expr
```

> `in` is a binary relational operator that returns `Bool`—`true` when found and `false` when not
> found, without raising an error. The semantic distinction is: `[]` asserts existence and retrieves
> a value (failure is an error), whereas `in` asks whether something exists (a miss is a normal
> `false`). The right operand may be a List / Array / Dict (key set) / Tuple / String (substring) /
> Range (interval). `in` is a first-class Horn predicate and serves as the foundation for
> compile-time provable propositions during the refinement type phase. (Set is excluded from the
> list of right operands—Set has no runtime representation; see §1.6.4.)

### 1.7 Comments

```
// Single-line comment

/* Multiline comment
   May span multiple lines */
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

| Priority | Operator                    | Associativity |
| -------- | --------------------------- | ------------- |
| 1        | `()` `[]` `.` `?`           | Left to right |
| 2        | `as`                        | Left to right |
| 3        | Unary prefix `!` `-` `+`    | Right to left |
| 4        | `*` `/` `%`                 | Left to right |
| 5        | `+` `-`                     | Left to right |
| 6        | `..`                        | Left to right |
| 7        | `<<` `>>`                   | Left to right |
| 8        | `&` `\|` `^`                | Left to right |
| 9        | `==` `!=` `<` `>` `<=` `>=` | Left to right |
| 10       | `and` `or`                  | Left to right |
| 11       | `if...else`                 | Right to left |
| 12       | `=` `+=` `-=` `*=` `/=`     | Right to left |

> **Unary prefix operators** (`!` `-` `+`) bind tightly: they bind less tightly only than calls and
> member access, and more tightly than every binary operator. Therefore, `!a == b` ≡ `(!a) == b`
> (Zig-style semantics); `!` is a purely unary operation, does not participate in short-circuit
> control flow, and is orthogonal to the `and`/`or` keywords (which short-circuit; RFC-010 is the
> authoritative definition).

> **Range binding power**: `..` binds at (6, 7)—its left side at 6 binds less tightly than addition
> (7), while its right side at 7 consumes addition but not another same-level `..`. Before and after
> the change:
>
> | Expression   | Before the change (level 1, right-associative)                                                                      | After the change ((6,7), left-associative)                                           |
> | ------------ | ------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
> | `x in 1..10` | `x in 1..10` (`in`'s right operand is level 4, and level-1 `..` cannot consume it, so it cannot actually be parsed) | `x in (1..10)`—the entire interval is the right operand of `in`                      |
> | `0..n+2`     | `(0..n)+2` (right-associativity trap: the upper bound is swallowed, causing a direct E3004 in a `for` loop)         | `0..(n+2)`—the upper bound is an arithmetic expression                               |
> | `a == b..c`  | `a == (b..c)` (`..` at level 1 < `==` at level 3, so it is naturally a whole)                                       | `a == (b..c)`—**semantics unchanged**; `..` still binds more tightly than comparison |
> | `1..2*3`     | `(1..2)*3`                                                                                                          | `1..(2*3)`—the upper bound is an arithmetic expression                               |
> | `a..b..c`    | `a..(b..c)` (right-associative chain with meaningless nested Ranges)                                                | `(a..b)..c`—**step form** (`c` is the step size)                                     |
>
> Net effect: a compound upper bound such as `for i in 0..n+2` changes from “parses successfully but
> reports E3004” to “directly usable”; `x in 1..10` changes from “unparseable” to “an interval
> check”; and `a..b..c` changes from “meaningless nesting” to “a step component”. Level 6 falls
> between `+` (level 5) and `<<` (level 7), following mathematical convention: an interval is a
> tightly bound construct, so its upper bound is naturally a complete arithmetic expression.

### 2.3 Function Calls

```
FnCall      ::= Expr '(' ArgList? ')'
ArgList     ::= Expr (',' Expr)* (',' NamedArg)* | NamedArg (',' NamedArg)*
NamedArg    ::= Identifier '=' Expr
```

Named arguments use `name = value` (RFC-010 §Function Definitions, RFC-011 §Constructor Forms).
Positional arguments must precede named arguments; named arguments for parameters with specified
order may appear in any order:

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b

add(3, 5)          // Positional arguments
add(a = 3, b = 5)  // Named arguments
add(b = 5, a = 3)  // Any order
add(3, b = 5)      // Mixed, with positional arguments first
```

An incorrect named argument reports **E1014**, specifying the same formal parameter both
positionally and by name reports **E1015**, and an argument-count mismatch reports **E1010**
(RFC-013).

**Unnamed parameters and named arguments**: signature parameter positions allow bare type syntax
(`mk: (Int, Int) -> Int`). This is an **unnamed typed parameter**—type constraints apply
positionally, while the name belongs to the implementation (the lambda header; see the shorthand
rules in [RFC-007](../../rfc/accepted/007-function-syntax-unification.md)) and is not part of the
contract. An unnamed parameter cannot be referenced by a named argument (there is no name to
reference) and can only be passed positionally; to include the name in the contract, write
`mk: (a: Int, b: Int) -> Int`. A bare identifier must be a declared type; failing to resolve the
type produces a parse error (see [type-system §3.5](type-system.md#35-函数类型)).

**The named-parameter alternative: names are part of the contract, and the declaration is
authoritative**. Parameter names in an implementation header (the parameter list after `=>`) are
merely local names within the body and are bound **positionally**—if the implementation header of
`f: (a: Int) -> Int = (b) => b` uses `b`, the call site still uses `f(a = 7)`. Type annotations in
the implementation header serve only for comparison; a mismatch with the declaration reports
**E1011**. If the implementation header has more parameters than the declared runtime parameter
positions, it reports **E1112** (`f: () -> Int = (a: Int) => a`)—in both cases, the declaration is
the authoritative source. The reverse case (the implementation omitting parameters) does not produce
an error: the type layer checks arity against the declaration, but a position omitted from the
implementation header has no runtime slot, and the corresponding argument is discarded on the IR
side (see the gap recorded in [type-system §3.5](type-system.md#35-函数类型)).

### 2.4 Member Access

```
MemberAccess::= Expr '.' Identifier
```

### 2.5 Index Access

```
IndexAccess ::= Expr '[' Expr ']'
```

> **Three-layer semantics (RFC-011b)**: `a[i]` dispatches according to the type of `a`—① built-in
> containers (List/Vec/Array/Dict/Tuple) use native indexing instructions (the fast path); ② user
> types that implement the `Index` interface dispatch to their `index` method (instantiating
> `Index(Grid, Int, Float)` inside the type body and providing a `Grid.index` method); ③ RFC-004's
> `f[0]` positional binding exists only in binding declarations and does not follow this grammar.
> The key of a multidimensional index such as `a[0, 1]` is packed into a tuple. Other types that do
> not implement `Index` are rejected at the type layer.

### 2.6 Type Casts

```
TypeCast    ::= Expr 'as' TypeExpr
```

> **Current implementation status (an existing gap, not introduced in this version)**: the preceding
> text defines only the grammar and precedence (§2.4, level 2); the semantics have not yet been
> implemented. Independently verifiable facts: ① Parsing: the `KwAs` infix arm in
> `parser/pratt/led.rs:81` produces `ast::Expr::Cast`; ② Type checking:
> `typecheck/inference/expressions.rs:3847-3853` performs only a **silent retagging** of
> `Expr::Cast`—`let _ = self.infer_expr(expr)?;` discards the inner result and then directly returns
> the `MonoType` for `target_type`; ③ the repository has **no** convertibility validation and no
> corresponding diagnostic code (there is no cast entry in `util/diagnostic/codes/`); ④ `ir_gen`
> generates no instructions (`middle/core/ir_gen.rs` only reads the span and free variables of
> `Expr::Cast` at `:5111` / `:5918` / `:9381`—the first two are the span arms of `get_expr_span` /
> `expr_span`, and the third is `collect_free_vars_expr`; none generates instructions); ⑤ therefore,
> `x as Int` currently neither reports an error nor performs any runtime conversion.

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
> type definition or a `use` import—`Result`/`Option` are defined by `std.result`/`std.option`, so
> `use std.result` / `use std.option` is required before use (whole-module and grouped
> `use std.{...}` forms have equal standing); otherwise, variant destructuring reports E1002.
> Exhaustiveness checking uses the same source: a fallback arm is exempt from exhaustiveness, while
> without a fallback arm, the entire variant set is checked.

### 2.9 Block Expressions

```
Block       ::= '{' Stmt* Expr? '}'
```

> **Statement termination rules**: separation between Stmts and newline behavior (`;` as an explicit
> separator, newline termination, the continuation-line exception, and line-leading `(`/`[` never
> merging) are defined by [RFC-038](../../rfc/accepted/038-statement-termination.md).

#### 2.9.1 The Three Forms of `{`

In expression position, `{` has exactly three possible interpretations, determined in a single pass
by its **content**:

| Form                   | Syntax            | Type                 | Example          |
| ---------------------- | ----------------- | -------------------- | ---------------- |
| **Empty block**        | `{}`              | `Void`               | `x: Void = {}`   |
| **Dictionary literal** | `{ "k": v, ... }` | `Dict(K, V)`         | `d = { "a": 1 }` |
| **Block**              | `{ Stmt* Expr? }` | Tail expression type | `y = { 1 + 1 }`  |

**Decision order**:

1. `{}` (no content) → **empty block**, value `Void`
2. The first element is a `String ':' Expr` key-value pair → **dictionary literal**
3. Otherwise → **block**, with its value determined by the tail expression

> **Why `{}` is not an empty dictionary**: the “emptiness” of an empty dictionary cannot describe
> itself (it could be either `Dict(K, V)` or an empty block), while the dictionary grammar in
> [§1.6.4](#_1-6-4-集合) requires at least one key-value pair. When there is no content on which to
> base a decision, the zero form of the block structure is used: this is consistent with `unsafe {}`
> / `spawn {}` and introduces no special case. To create an empty dictionary, use `dict.new()`.
>
> **Why functions require annotations**: `f = { stmt }` is a **value** (with the tail expression's
> type), not a function. To define a function, specify an Fn annotation: `f: () -> Int = { 5 }`.
> This follows the same principle as dictionaries: **the type is determined by the content**, not by
> the presence or absence of an annotation. (See Appendix D of
> [RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md).)

**Unified semantics**: the value of every `{}` block is determined by its **tail expression**, and
`return` is a `Never`-typed nonlocal exit.

| Block type    | Value outlet    | Empty block `{}` |
| ------------- | --------------- | ---------------- |
| Ordinary `{}` | Tail expression | `Void`           |
| `unsafe {}`   | Tail expression | `Void`           |
| `spawn {}`    | Tail expression | `Void`           |

**Core principles** (see [RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md) for
details):

- **A block's value = its tail expression** (the final expression), its sole exit, with no
  exceptions.
- If the final item is an **assignment statement**, the block's value is `Void`; to obtain `Void`,
  explicitly write `Void`.
- **`return` exits the nearest function boundary** (passing through every block; it does not “return
  to the block”), and has type `Never`; `Never <: T`

> holds for any type (principle of explosion), so it may appear in any return-type position. The
> module initialization layer (the Script top level) has no function boundary, so `return` is
> rejected at compile time there (`E1109`; see §3.11).

- The expression form `= expr` directly yields a value.

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

// return: pass through blocks and exit the function
f: (n: Int) -> Int = {
    if n < 0 {
        return 0     // Exits the if block and function body, exiting the function
    }
    n * 2            // Tail expression
}
```

#### Is `name = { ... }` a Function or a Block Value?

`name = { ... }` may be either a function definition (RFC-007's “zero-argument minimal form”) or a
block-value binding. The decision follows **annotation first, function by default** (RFC-010a
Appendix D):

| Situation                             | Result          | Example                            |
| ------------------------------------- | --------------- | ---------------------------------- |
| The value is a Lambda (`=>`)          | Function        | `f = () => 5` → `f()` = 5          |
| The annotation is a function type     | Function        | `f: () -> Int = { 5 }` → `f()` = 5 |
| The annotation is a non-function type | **Block value** | `x: Int = { 5 }` → `x` = 5         |
| No annotation                         | Function        | `f = { 5 }` → `f()` = 5            |

**An annotation is a type**: `x: Int = ...` declares that `x` is an `Int`, so `{ ... }` evaluates to
an `Int`; `f: () -> Int = ...` declares that `f` is a function, so `{ ... }` is a function body.

To evaluate `{ ... }` immediately, **write the target type**—no new syntax is required:

```yaoxiang
// Block value: evaluate immediately
x: Int = {
    y = 5
    y            // x = 5
}

// Function: defaults to no annotation
f = { 5 }        // f() = 5
```

#### Nested Function Types: Curried or Returning a Function?

There are two ways to read a nested function type on the right-hand side of `->`, distinguished by
**parentheses** (RFC-004):

| Syntax                          | Meaning                | Invocation               |
| ------------------------------- | ---------------------- | ------------------------ |
| `(a: Int) -> (b: Int) -> Int`   | **Curried**            | `f(1)(2)`                |
| `(a: Int) -> ((b: Int) -> Int)` | **Returns a function** | `g(1)` yields a function |

The basis is: **an annotation is a type**. `g: (a: Int) -> ((b: Int) -> Int)` declares that
`g(1) : (b: Int) -> Int`, so `g(1)` must **be** that function, rather than “the next parameter
group”.

```yaoxiang
// Curried: supply the two parameter groups one layer at a time
add: (a: Int) -> (b: Int) -> Int = { a + b }
add(1)(2)        // → 3

// Returning function: the outer parameter group returns the function
adder: (n: Int) -> ((x: Int) -> Int) = (x) => x + n
adder(10)(5)     // → 15
h = adder(10)    // The returned function can be stored in a variable or passed around
```

An unparenthesized nested `Fn` is always curried (including RFC-011's type-parameter form):

```yaoxiang
identity: (T: Type) -> (x: T) -> T = (x) => x   // Curried
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

The `?` operator is postfix and has the same precedence as `.`. For the `Result(T, E)` type:

- When `Ok(v)`, extract the value `v` and continue execution.
- When `Err(e)`, propagate the error upward (`return Err(e)`).

```yaoxiang
process: (data: Data) -> Result(Data, Error) = {
    validated = validate(data)?     // Extract the value on success, or propagate upward on failure
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

// Step form (third component, defaults to 1)
for i in 0..10..2 { print(i) }  // 0, 2, 4, 6, 8
for i in 10..0..(-2) { print(i) }  // 10, 8, 6, 4, 2
```

> **Step semantics**: in `a..b..c`, `c` is the step size. A literal `c = 0` is rejected at compile
> time; a dynamic `c` receives a runtime zero check (the E6001 family; promoted to Result once the
> error system is implemented). `c < 0` is legal, and the interval direction reverses according to
> its sign (`10..0..(-2)` is descending).

### 2.13 ref Expressions

```
RefExpr     ::= 'ref' Expr
```

`ref` creates shared ownership. The compiler automatically selects Rc (single-task) or Arc
(cross-task); users do not need to concern themselves with implementation details.

```yaoxiang
data = ref heavy_data
spawn { use(data) }   // Across tasks: the compiler automatically selects Arc
```

### 2.14 unsafe Expressions

```
UnsafeExpr  ::= 'unsafe' Block
```

`unsafe` blocks are used to define opaque types and manipulate raw pointers. Use `return` to return
a type definition to the enclosing scope.

**Semantics**:

- Types and raw-pointer operations may be defined inside `unsafe {}`.
- A returned type is available outside `unsafe {}`.
- Accessing a type's fields requires unsafe permission.

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

- Every `{}` block creates a scope.
- An inner scope can access variables from outer scopes.
- An outer scope cannot access variables from an inner scope.
- Variable declarations follow the “assignment first” principle.

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

**Variable declarations and shadowing**:

- `x = value`: searches outward through the scope chain for x; if found, it assigns to it, and if
  not found, it declares a new binding.
- `mut x = value`: explicitly declares a new mutable binding and prohibits the same name in an outer
  scope.
- Within the same scope, any name may be declared only once.

> **Detailed definition**: see the [Module System Specification](modules.md#第四章作用域) for the
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

### 3.2 Variable Declarations

```
LetStmt     ::= ('mut')? Identifier (':' TypeExpr)? '=' Expr
```

### 3.3 return Statements

```
ReturnStmt  ::= 'return' Expr?
```

**Semantics**: `return` is a **nonlocal exit** that exits the nearest **function boundary** (passing
through every block—including `if` / `while` / `for` / `match` / bare blocks / `spawn` / `unsafe`)
and hands its value to the caller. It does **not “return to the block.”**

**Type**: `return e : Never` (`e : T`). `Never <: T'` holds for any `T'` (principle of explosion;
see [Type System §2.2](type-system.md)), so `return` may appear in any return-type position without
requiring an additional rule.

**Relationship with block evaluation**: a block's value is always its **tail expression** (see
§2.9). As a block, `{ return n }` has the value `n` and the type `Never`; at the same time, `return`
exits the function. **Both properties hold simultaneously**, coexisting through the principle of
explosion.

Together, `return` and the tail expression make “early return” possible, without requiring an
additional rule in which `return` specifically refers to a function—see
[RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md).

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 {
        return 1          // Exits the if block and the function (type Never)
    }
    n * factorial(n - 1)  // Tail expression = the block's value
}
```

### 3.4 break Statements

```
BreakStmt   ::= 'break'
```

**Semantics**: immediately terminates the innermost enclosing `while`/`for` loop, transferring
control to after that loop's body.

- **Exits only the nearest level**: `break` always applies to the innermost loop containing it. When
  multiple nested loop levels must be exited at once, extract the inner loop into a function and use
  `return`, or use a flag (break/continue do not carry labels; if loop labels are introduced in the
  future, they will follow the syntax on the loop declaration side through the RFC process,
  alongside the multi-exit design of the proof pipeline).
- **Permitted only inside loop bodies**: `break` may appear only in the body of a `while`/`for` loop
  (including nested blocks/if/match statements in that body). Appearing outside a loop produces a
  compile-time error (E1102 `'break' outside of a loop`).
- **Does not affect termination proofs**: `break` does not participate in termination arguments—it
  provides neither a measure nor a decrementing step. The loop's termination obligation is
  independent of `break` and is triggered by the refinement type system (see
  [type-system §8.5](type-system.md#85-terminates终止测度谓词)).
- **Borrowing semantics**: the control-flow edge for break participates in the structural cutoff of
  RFC-009a's reverse-BFS liveness analysis (iterations exited by the break do not participate in
  back-edge liveness derivation).

```yaoxiang
mut i = 0
while i < 10 {
    i = i + 1
    if i == 3 {
        break              // Control transfers to after the loop, with i == 3
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

### 3.5 continue Statements

```
ContinueStmt::= 'continue'
```

**Semantics**: skips the remaining statements in the current iteration and proceeds directly to the
next iteration of the innermost enclosing loop—`while` reevaluates its condition, while `for` takes
the next element.

- **Affects only the nearest level**: as with `break`, it does not carry labels.
- **Permitted only inside loop bodies**: appearing outside a loop produces a compile-time error
  (E1102).

```yaoxiang
mut sum = 0
mut n = 0
while n < 5 {
    n = n + 1
    if n == 3 {
        continue           // Skip the addition below; n == 3 is not counted
    }
    sum = sum + n
}
// sum == 12 (1 + 2 + 4 + 5)
```

### 3.6 if Statements

```
IfStmt      ::= 'if' Expr Block ('else' 'if' Expr Block)* ('else' Block)?
```

### 3.7 match Statements

```
MatchStmt   ::= 'match' Expr '{' MatchArm+ '}'
```

### 3.8 while Statements

```
WhileStmt   ::= 'while' Expr Block
```

### 3.9 for Statements

```
ForStmt     ::= 'for' 'mut'? Identifier 'in' Expr Block
```

#### 3.9.1 Semantics: Each Iteration Binds a New Value

The semantics of YaoXiang's for loop differ from those of traditional languages: **each iteration
binds a new value instead of modifying the same variable**.

```yaoxiang
// Example: for i in 1..5
for i in 1..5 {
    print(i)
}
```

**Execution process**:

| Iteration | Behavior of the loop variable                                                                      |
| --------- | -------------------------------------------------------------------------------------------------- |
| First     | Creates a new binding `i = 1`, executes the body, and prints 1                                     |
| Second    | Creates a new binding `i = 2` (the previous binding is destroyed), executes the body, and prints 2 |
| Third     | Creates a new binding `i = 3`, executes the body, and prints 3                                     |
| Fourth    | Creates a new binding `i = 4`, executes the body, and prints 4                                     |
| End       | The loop body ends and the binding is destroyed                                                    |

**Key point**: after each iteration ends, the binding created during that iteration is destroyed.
The next iteration uses an entirely new binding, unrelated to the binding from the previous
iteration.

#### 3.9.2 Difference Between for and for mut

| Syntax              | Mutability of the loop variable | Description                                     |
| ------------------- | ------------------------------- | ----------------------------------------------- |
| `for i in 1..5`     | Immutable                       | The binding cannot be modified in the loop body |
| `for mut i in 1..5` | Mutable                         | The binding can be modified in the loop body    |

```yaoxiang
// Valid: each iteration binds a new value, so no modification is needed
for i in 1..5 {
    print(i)  // Reads the value of i
}

// Invalid: an immutable binding cannot be modified
for i in 1..5 {
    i = i + 1  // Error: an immutable binding cannot be modified
}

// Valid: for mut permits modifying the binding
for mut i in 1..5 {
    i = i + 1  // Modification is permitted
}
```

#### 3.9.3 Shadowing Checks

YaoXiang prohibits variable shadowing. A for-loop variable cannot have the same name as a variable
in an outer scope:

```yaoxiang
// Invalid: i has already been declared in the outer scope
i = 10
for i in 1..5 {
    print(i)
}

// Valid: use a different variable name
i = 10
for j in 1..5 {
    print(j)
}
```

This rule applies to all code blocks. See [4.3 Shadowing Rules](modules.md#43-遮蔽规则) for details.

#### 3.9.4 Comparison with Other Languages

| Language | Semantics of the for-loop variable                           |
| -------- | ------------------------------------------------------------ |
| YaoXiang | Binds a new value on each iteration                          |
| Rust     | Modifies the same variable (requires `mut`)                  |
| Python   | Modifies the same variable (no `mut` required)               |
| C/C++    | Modifies the same variable (requires a pointer or reference) |

**Design rationale**: YaoXiang adopts binding semantics because:

1. **It better matches natural semantics.** In natural language, “for each element x in a set” means
   that each x is an independent individual. YaoXiang's `for i in 1..5` reads as “for each i from 1
   through 5,” with each iteration's i being an entirely new binding, consistent with human
   intuition.

2. **It prevents accidental modification.** Immutable bindings by default mean that a loop variable
   cannot be accidentally modified within the loop body. There is no need to worry about
   accidentally writing `i = ...` somewhere in a complex loop body and creating a difficult-to-trace
   bug.

3. **A high-performance approach remains readily available.** When a variable genuinely must be
   reused across iterations (for example, as an accumulator or cache), use a `for mut` declaration
   to switch to mutable-binding mode. This is clearer than implicit shared state: intent is
   expressed explicitly through syntax rather than hidden in runtime behavior.

### 3.10 spawn Statements

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' 'mut'? Identifier 'in' Expr '{' Expr '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
```

**spawn block**: explicitly declares a concurrency region; expressions inside the block execute
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

- A block body referencing an outer variable uses **Move value capture**: the value is snapshotted
  into the closure environment at the spawn creation point, and the body reads it through the
  environment (LoadUpvalue).
- **Primitive values** (Int/Float/Bool/Char) are copied, leaving the outer variable unaffected.
- For **handle types** (Struct/String/List, etc.), the snapshot copies the handle and shares the
  underlying object; in the Embedded runtime (the default), the same thread and heap apply and the
  handle remains valid.
- Sharing across multiple tasks requires an explicit `ref` (§2.13; the compiler automatically
  selects Rc/Arc).
- A `return` within the block also captures the outer variables it references.

```yaoxiang
t1 = 1 + 1
t2 = 2 + 2
result = spawn {
    return t1 + t2    // t1/t2 are value-captured, and result == 6
}
```

---

### 3.11 Program Entry Point and Top-Level Statements

From the compiler's perspective, a source file has two possible roles, **determined by whether
`yaoxiang.toml` exists**:

| Role       | Determination                                       | Program body                                                                        |
| ---------- | --------------------------------------------------- | ----------------------------------------------------------------------------------- |
| **Script** | A single file run directly, without `yaoxiang.toml` | **Top-level statements** (executed in written order); `main` is an ordinary binding |
| **Bin**    | `yaoxiang.toml` exists                              | **The `main` function**; top-level executable statements are forbidden              |

#### Script: Top-Level Statements Are the Program

Without a manifest, the file is a “script,” and **top-level statements execute in written order**
(top-level **binding initialization** as a whole takes precedence; see
“[Top-Level Binding Initialization](#顶层绑定的初始化)” below):

```yaoxiang
use std.io
io.println("hello")          // Executes directly
x: Int = { 42 }              // Top-level binding: initialized at runtime
io.println(x)                // 42
```

In this mode, `main` is **not special**—it is merely an ordinary binding. To run `main`, it must be
invoked explicitly:

```yaoxiang
use std.io
main: () -> Void = { io.println("only runs if called") }

main()                       // ← This line must be written
```

> **Why is `main` not invoked automatically?** The top-level statements are already the program
> body. If `main` were also invoked implicitly, a script that explicitly contains `main()` would
> execute twice. The two rules cannot coexist, so under Script there is only one execution entry
> point: “top-level statements.”

The top level must likewise **not use `return`**: a Script top level is a module initialization
layer and has no function boundary to exit (RFC-010a Rule 2). It reports compile-time error `E1109`,
rather than silently terminating initialization at runtime and skipping the remaining top-level
statements. To end a top-level sequence early, place it in a function and control it at the call
site.

#### Bin: `main` Is the Entry Point

When a manifest is present, the file is an “executable target,” and:

- It must define a **binding** named `main`, whose value may be either a value or a function (#388
  final decision: the entry point is determined by the existence of the binding—a function is
  already a value, and the difference lies only in evaluation strategy): a function-valued `main` is
  invoked with no arguments at entry; a value-valued `main` is evaluated during initialization.
- Executable statements are forbidden at the top level—the program body is `main`.

```yaoxiang
main: () -> Void = {
    print("hello")
}
```

A value-valued `main` is equally valid—it is evaluated during initialization, within the
initialization sequence (using the global binding topological order, with independent bindings
ordered by source order):

```yaoxiang
main = {
    print("hello")
}
```

A missing `main` is a compile error (without `main`, all functions are unreachable). An uncallable
value-valued `main` (for example, `main: Int = 5`) is valid but has no observable effect—analogous
to Rust's empty `fn main() {}`.

> **Library files**: files imported by other files with `use`, or files referenced by `[lib].path` /
> `[exports]`, do not require `main`—they are not program entry points.

#### Top-Level Binding Initialization

Top-level binding initializers are **evaluated at runtime** and are not required to be compile-time
constants:

```yaoxiang
answer: Int = { 42 }              // Block value
inc: (Int) -> Int = (x) => x + 1
computed: Int = inc(41)           // Function call
```

Initialization executes in **dependency order**, independently of written order (dependencies are
initialized first; independent bindings follow declaration order):

```yaoxiang
derived: Int = base * 3           // References base, which is declared later
base: Int = 7                     // Initialized first (topological sorting)
```

Furthermore, **all binding initialization occurs before any top-level statement executes**—side
effects in binding initializers occur before every top-level statement, regardless of how the two
are interleaved in the source:

```yaoxiang
print("a")                        // Executes later
x = { print("b") }                // Executes first (all binding initialization takes precedence)
print("c")                        // Output order: b, a, c
```

Circular dependencies are compile errors (the names on the cycle are listed):

```yaoxiang
a: Int = b + 1
b: Int = a + 1                    // Error: a → b → a
```

> **Design basis**: RFC-029f (file role model), RFC-010a Appendix D (block binding decisions).

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
entirely under RFC-029g and has reverted to an ordinary identifier. See §1.4.1 for the literal
reserved words `true` / `false` / `void`, and §1.4.3 for the metatype name `Type` and the built-in
type names `Void` / `Never` / `Int` / `Float` / `Bool` / `Char` / `String`.

### A.1 Control Flow

```
if Expr Block (else if Expr Block)* (else Block)?
match Expr { MatchArm+ }
while Expr Block
for 'mut'? Identifier 'in' Expr Block
break | continue          // Permitted only inside loop bodies (§3.4 / §3.5)
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
