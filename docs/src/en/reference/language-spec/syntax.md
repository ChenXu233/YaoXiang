# Syntax Specification

This document defines the syntax specification of the YaoXiang programming language, including
lexical structure, syntax rules, and operator precedence.

---

## Chapter 1: Lexical Structure

### 1.1 Source Files

YaoXiang source files must use UTF-8 encoding. Source files usually use the `.yx` extension.

### 1.2 Lexical Token Categories

| Category   | Description                               | Examples                  |
| ---------- | ----------------------------------------- | ------------------------- |
| Identifier | Starts with a letter or underscore        | `x`, `_private`, `my_var` |
| Keyword    | Reserved words predefined by the language | `use`, `mut`, `and`       |
| Literal    | Fixed value                               | `42`, `"hello"`, `true`   |
| Operator   | Operator symbol                           | `+`, `-`, `*`, `/`        |
| Delimiter  | Syntax delimiter                          | `(`, `)`, `{`, `}`, `,`   |

### 1.3 Keywords

YaoXiang has **17 keywords** (`keyword_from_str` in `src/frontend/core/lexer/state.rs`, with a
one-to-one correspondence to `TokenKind`):

```
use     spawn   ref     mut
if      else    match   while   for
in      return  break   continue
as      unsafe  and     or
```

`and` / `or` are **keywords** for logical AND / OR (Zig-style; see level 10 in §2.2). Unary negation
uses the symbol `!` and is orthogonal to them.

These keywords have special meanings in every context and cannot be used as identifiers.

> **`type` is no longer a keyword** (RFC-010): use the `Name: Type = { ... }` notation to define a
> type. Both `keyword_from_str` and the `TokenKind` enum header
> (`src/frontend/core/lexer/tokens.rs`) are marked with “RFC-010: 'type' keyword removed”.
>
> **`pub` is no longer a keyword** (RFC-029g, accepted on 2026-10-02): the language has no
> visibility mechanism. `pub` has been removed entirely from the lexer, AST, type checker, dead-code
> exemptions, formatter, and LSP ([RFC-029g](../../rfc/accepted/029g-remove-pub-and-auto-bind.md)).
> It is now an ordinary identifier—the old form `pub x = 1` reports undefined name E1001. All
> top-level bindings are importable by default, and modifiers make no difference to visibility
> ([RFC-029](../../rfc/accepted/029-module-semantics.md)).

### 1.4 Reserved Words

YaoXiang's “reserved words” are divided into three layers, recognized at different stages by the
parser and type checker:

#### 1.4.1 Literal Reserved Words

The parser has separate tokens for the following literal identifiers, which cannot be used as
ordinary identifiers:

| Identifier | Type | Description                                                                                                   |
| ---------- | ---- | ------------------------------------------------------------------------------------------------------------- |
| `true`     | Bool | Boolean true value                                                                                            |
| `false`    | Bool | Boolean false value                                                                                           |
| `void`     | Void | Void literal (Unit value). Lowercase `void` is a value literal; uppercase `Void` is a type name (see §1.4.3). |

> **`Type` is not in this layer**: it has **no** dedicated `TokenKind` (there is no `Type` in the
> `TokenKind` enum in `src/frontend/core/lexer/tokens.rs`, and `keyword_from_str` has no
> corresponding branch). The parser treats it as an ordinary identifier, while the type checker
> recognizes it as a meta type in type position. Therefore, in expression position, `Type` may be
> shadowed by a local binding. It is a **meta type name**, not a keyword.

#### 1.4.2 Variant Names

Variant names are recognized by the parser in **pattern** contexts (bare names are resolved
according to the scrutinee type's variant set; see §2.8). In **expression** contexts, constructing a
variant requires a type qualification—`Result(Int, String).ok(5)`, `Option(T).some(v)`,
`Color.green()` (RFC-010 record types and types: variant constructors are function-typed fields in
type definitions, so a bare name such as `ok(5)` is not a constructor call).

| Variant Name | Owning Type | Description                       |
| ------------ | ----------- | --------------------------------- |
| `some(T)`    | Option      | Option value variant construction |
| `ok(T)`      | Result      | Result success variant            |
| `err(E)`     | Result      | Result error variant              |

#### 1.4.3 Built-in Type Names

The type checker pre-registers the following type names, allowing them to be used in type positions
without imports. The parser treats them as ordinary identifiers—**they are not reserved words and
may be shadowed by local bindings (not recommended)**.

| Type Name | Logical Correspondence | Description                                                                                                                               |
| --------- | ---------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| `Void`    | ⊤ (true/Unit)          | A product type with zero fields and exactly one inhabitant (the `void` literal; see §1.4.1)                                               |
| `Never`   | ⊥ (false/empty type)   | Zero variants and zero inhabitants. No expression can produce a `Never` value. `Never <: T` holds for every `T` (principle of explosion). |
| `Int`     | —                      | Signed integer                                                                                                                            |
| `Float`   | —                      | Floating-point number                                                                                                                     |
| `Bool`    | —                      | Boolean value: `true` / `false`                                                                                                           |
| `Char`    | —                      | Unicode character                                                                                                                         |
| `String`  | —                      | String                                                                                                                                    |

### 1.5 Identifiers

Identifiers start with a letter or underscore, and subsequent characters may be letters, digits, or
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
Array       ::= '[' Expr (',' Expr)* ']'   // When the target type annotation is Array(T, N), the literal becomes a fixed-length array
```

> **A dictionary literal requires at least one key-value pair**: `{}` is **not** an empty
> dictionary—it is an empty block (value `Void`; see [§2.9](#_2-9-块表达式)). To create an empty
> dictionary, use the constructor `dict.new()`:
>
> ```yaoxiang
> empty = dict.new()                  // ✅ Empty dictionary
> d = { "a": 1 }                       // ✅ Dictionary literal
> wrong = {}                           // ❌ Not a dictionary; an empty block (Void)
> ```
>
> **The criterion is content**: the `Dict` grammar requires at least one `String ':' Expr`. Since
> `{}` has no content on which to base a decision, it takes the zero form of the block structure.
> The non-empty form is self-describing through its content (`{ "k": v }` has a key-value pair → a
> dictionary). This comes from the same principle as `f = { 5 }` being an `Int` value rather than a
> function: **the type is determined by the content**.

> Set has no literal grammar or runtime representation—when a Set is required in future planning, it
> will be completed following the Dict pattern (`std.set` + `HeapValue::Set`). The destination of
> List/Dict literals is determined by the contextual type annotation: a bare literal and a `List(T)`
> annotation resolve to a growable list, while an `Array(T, N)` annotation applied directly to a
> literal resolves to a fixed-length array. Implicit List→Array conversion is forbidden.
>
> Array literal semantics:
>
> - The number of elements must equal N; otherwise, compilation fails with E1002. An empty literal
>   paired with a nonzero N is rejected in the same way
> - Every element type must be compatible with T; otherwise, compilation fails with E1002
> - The grammatical form of N is limited to an integer literal (which may be negative) or a constant
>   name; compound expressions such as `2+1` are rejected during parsing
> - When N is a signed constant (such as a function const parameter in `Array(Int, n)`),
>   element-count validation is deferred to the refinement type stage
> - v1 rejects nested array literals (`Array(Array(Int,2),2) = [[1,2],[3,4]]`) at compile time; each
>   level must be constructed explicitly. Recursive destinations are deferred to a later version

#### 1.6.5 List Comprehensions

```
ListComp    ::= '[' Expr ( 'for' Identifier 'in' Expr ( 'if' Expr )? )+ ']'
```

> **#401 (implementation completion)**: `if` filters and multiple generator clauses are implemented
> as of this release. The previous grammar was `(',' Expr)* ('if' Expr)?`, which had no
> corresponding implementation (neither additional comma-separated expressions nor multiple
> generators were supported), and the parser omitted the `if` filter entirely (after parsing the
> iterable, it expected `']'` and reported `E0010 Expected RBracket, found KwIf`). The semantics now
> follow common industry practice: there may be one or more generator clauses, expanded as nested
> loops; each clause may carry at most one `if` filter, and the condition must be a `Bool` (a
> non-Bool condition reports `E1054`). The iterable or condition of a later clause may refer to
> iteration variables bound by an earlier clause.

> **Behavior tightening (migration record)**: the iteration-variable grammar was already
> `'for' Identifier 'in'`, but the old implementation parsed the pattern using the full Pratt
> parser—after `'in'` was registered as an infix operator, `x` would swallow `in items` as a
> membership expression. After the fix, non-identifier patterns fail to parse directly and no longer
> fall back to `_` (silently swallowing the error). Impact: forms such as `[x for (a, b) in pairs]`,
> which previously parsed, now report an error. This form never had defined behavior (the variable
> was always `_`), so tightening it is correct and entails no semantic migration cost.

#### 1.6.6 Membership Tests

```
Membership  ::= Expr 'in' Expr
```

> `in` is a binary relation operator that returns `Bool`—`true` for a match and `false` for no
> match, without reporting an error. The semantic distinction is that `[]` asserts existence and
> retrieves a value (reporting an error on failure), whereas `in` asks whether something exists (a
> miss is a normal `false`). Right-operand coverage: List / Array / Dict (key set) / Tuple / String
> (substring) / Range (interval). `in` is a first-class Horn predicate and serves as the basis for
> compile-time provable propositions in the refinement type stage. (Set is omitted from the
> right-operand list—Set has no runtime representation; see §1.6.4.)

### 1.7 Comments

```
// Single-line comment

/* Multi-line comment
   Can span multiple lines */
```

### 1.8 Indentation Rules

Code must use four spaces for indentation. Tab characters are forbidden. This is a mandatory syntax
rule.

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

> **Unary prefix operators** (`!` `-` `+`) bind tightly: they rank only below calls and member
> access, and above all binary operators. Therefore, `!a == b` ≡ `(!a) == b` (Zig-style semantics).
> `!` is a purely unary operation, does not participate in short-circuit control flow, and is
> orthogonal to the `and`/`or` keywords (which short-circuit; RFC-010 provides the authoritative
> definition).

> **Range binding strength**: `..` has binding strengths `(6, 7)`—a left strength of 6, below
> addition (7), and a right strength of 7 that consumes addition but not another same-level `..`.
> Comparison before and after the change:
>
> | Expression   | Before the Change (level 1, right-associative)                                                                   | After the Change (`(6,7)`, left-associative)                               |
> | ------------ | ---------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------- |
> | `x in 1..10` | `x in 1..10` (`in` has right-operand level 4; level-1 `..` cannot consume it, so parsing actually fails)         | `x in (1..10)`—the entire interval is the right operand of `in`            |
> | `0..n+2`     | `(0..n)+2` (right-associativity trap: the upper bound is consumed, causing `for` loops to report E3004 directly) | `0..(n+2)`—the upper bound is an arithmetic expression                     |
> | `a == b..c`  | `a == (b..c)` (level-1 `..` < level-3 `==`, so it is naturally grouped as a whole)                               | `a == (b..c)`—**semantics unchanged**; `..` remains above comparison level |
> | `1..2*3`     | `(1..2)*3`                                                                                                       | `1..(2*3)`—the upper bound is an arithmetic expression                     |
> | `a..b..c`    | `a..(b..c)` (right-associative chain, a meaningless Range nested within a Range)                                 | `(a..b)..c`—**step form** (`c` is the step size)                           |
>
> Net effect: a compound upper bound such as `for i in 0..n+2` changes from “parses successfully but
> reports E3004” to “works directly”; `x in 1..10` changes from “cannot be parsed” to “interval
> check”; and `a..b..c` changes from “meaningless nesting” to “step component”. Level 6 falls
> between `+` (level 5) and `<<` (level 7), following mathematical convention: an interval is a
> tightly binding construct, so its upper bound is naturally a complete arithmetic expression.

### 2.3 Function Calls

```
FnCall      ::= Expr '(' ArgList? ')'
ArgList     ::= Expr (',' Expr)* (',' NamedArg)* | NamedArg (',' NamedArg)*
NamedArg    ::= Identifier '=' Expr
```

Named arguments use `name = value` (RFC-010 §Function Definitions, RFC-011 §Constructor Forms).
Positional arguments must precede named arguments. Parameters whose order is specified may appear in
any order among named arguments:

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b

add(3, 5)          // Positional
add(a = 3, b = 5)  // Named
add(b = 5, a = 3)  // Any order
add(3, b = 5)      // Mixed, with positional arguments first
```

An incorrect named argument reports **E1014**, specifying the same formal parameter both
positionally and by name reports **E1015**, and an incorrect argument count reports **E1010**
(RFC-013).

**Unnamed parameters and named arguments**: signature parameter positions may use a bare type form
(`mk: (Int, Int) -> Int`). This is an **unnamed typed parameter**—type constraints apply
positionally, while the name belongs to the implementation (the lambda header itself; see the
shorthand rules in [RFC-007](../../rfc/accepted/007-function-syntax-unification.md)) and is not part
of the contract. An unnamed parameter cannot be referenced by a named argument because it has no
name to reference; it can only be passed positionally. To include names in the contract, write
`mk: (a: Int, b: Int) -> Int`. A bare identifier must be a declared type; if the type cannot be
resolved, parsing reports an error (see [type-system §3.5](type-system.md#35-函数类型)).

**Named parameters in reverse: the name is part of the contract, and the declaration is
authoritative**. Parameter names in an implementation header (the parameter list after `=>`) are
merely local names within the body and are bound **positionally**—in
`f: (a: Int) -> Int = (b) => b`, the implementation header uses `b`, while callers still write
`f(a = 7)`. Type annotations in the implementation header serve only for comparison; a mismatch with
the declaration reports **E1011**. An implementation header with more parameters than the declared
runtime parameter positions reports **E1112** (`f: () -> Int = (a: Int) => a`)—in both cases, the
declaration is the authoritative source. The reverse case, where the implementation omits a
parameter, does not report an error: the type layer checks arity against the declaration, but a
parameter position omitted from the implementation header has no runtime slot, and arguments
corresponding to it are discarded on the IR side (see the gap registry in
[type-system §3.5](type-system.md#35-函数类型)).

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
> that implement the `Index` interface are dispatched to their `index` method
> (`Index(Grid, Int, Float)` instantiated in the type body, plus a `Grid.index` method); ③ RFC-004's
> `f[0]` positional binding exists only in binding declarations and does not use this grammar. In a
> multidimensional index such as `a[0, 1]`, the key is packed as a tuple. Other types that do not
> implement `Index` are rejected at the type layer.

### 2.6 Type Casts

```
TypeCast    ::= Expr 'as' TypeExpr
```

> **Implementation status (an existing gap, not introduced in this version)**: the preceding text
> defines only the grammar and precedence (level 2 in §2.4); the semantics have not yet been
> implemented. Independently verifiable facts: ① Parsing: the `KwAs` infix arm in
> `parser/pratt/led.rs:81` produces `ast::Expr::Cast`; ② Type checking:
> `typecheck/inference/expressions.rs:3847-3853` performs only a **silent reannotation** of
> `Expr::Cast`—`let _ = self.infer_expr(expr)?;` discards the inner result before directly returning
> the `target_type` `MonoType`; ③ The repository has **no** convertibility validation and no
> corresponding diagnostic code (there is no cast entry in `util/diagnostic/codes/`); ④ `ir_gen`
> generates no instructions (`middle/core/ir_gen.rs` only obtains the span and free variables of
> `Expr::Cast` at `:5111`, `:5918`, and `:9381`—the first two are `get_expr_span` / `expr_span` span
> arms, and the third is `collect_free_vars_expr`; none of the three generates instructions); ⑤
> Consequently, `x as Int` currently reports no error and performs no runtime conversion.

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

> **Variant-set requirement for variant destructuring** (RFC-010b): `EnumPattern` (such as `ok(v)`
> or `some(x)`, with variant names resolved according to the scrutinee) requires the scrutinee's
> type to be a **sum type with a present variant set**. A variant set enters the checker only
> through a type definition or a `use` import—`Result`/`Option` are defined by
> `std.result`/`std.option` and must be imported with `use std.result` / `use std.option` before use
> (whole-module and grouped `use std.{...}` forms have equal status); otherwise, variant
> destructuring reports E1002. Exhaustiveness checking uses the same source: a fallback arm is
> exempt from exhaustiveness, while without a fallback arm, the entire variant set is checked.

### 2.9 Block Expressions

```
Block       ::= '{' Stmt* Expr? '}'
```

> **Statement termination rules**: separation between statements and newline behavior (`;` explicit
> separators, newline termination, continuation exceptions, and line-leading `(`/`[` never merging)
> are defined by [RFC-038](../../rfc/accepted/038-statement-termination.md).

#### 2.9.1 The Three Forms of `{`

In expression position, `{` has exactly three interpretations, determined in a single pass by its
**content**:

| Form                   | Notation          | Type                 | Example          |
| ---------------------- | ----------------- | -------------------- | ---------------- |
| **Empty block**        | `{}`              | `Void`               | `x: Void = {}`   |
| **Dictionary literal** | `{ "k": v, ... }` | `Dict(K, V)`         | `d = { "a": 1 }` |
| **Block**              | `{ Stmt* Expr? }` | Tail expression type | `y = { 1 + 1 }`  |

**Decision order**:

1. `{}` (no content) → **empty block**, value `Void`
2. The first element is a `String ':' Expr` key-value pair → **dictionary literal**
3. Otherwise → **block**, with the value determined by the tail expression

> **Why `{}` is not an empty dictionary**: the “emptiness” of an empty dictionary cannot be
> self-describing (it could be either `Dict(K, V)` or an empty block), and the dictionary grammar
> [§1.6.4](#_1-6-4-集合) requires at least one key-value pair. When there is no content on which to
> base a decision, the zero form of the block structure is used. This is consistent with `unsafe {}`
> / `spawn {}` and introduces no special case. Use `dict.new()` for an empty dictionary.
>
> **Why functions require an annotation**: `f = { stmt }` is a **value** (with the tail expression's
> type), not a function. To define a function, specify an Fn annotation: `f: () -> Int = { 5 }`.
> This follows the same principle as dictionaries: **the type is determined by the content**, not by
> whether an annotation exists. (See
> [RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md), Appendix D.)

**Unified semantics**: the value of every `{}` block is determined by its **tail expression**, and
`return` is a non-local exit of type `Never`.

| Block Type    | Value Exit      | Empty Block `{}` |
| ------------- | --------------- | ---------------- |
| Ordinary `{}` | Tail expression | `Void`           |
| `unsafe {}`   | Tail expression | `Void`           |
| `spawn {}`    | Tail expression | `Void`           |

**Core principles** (see [RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md) for
details):

- **A block's value = its tail expression** (the last expression), its sole exit, with no exceptions
- When the final element is an **assignment statement**, the block's value is `Void`; to obtain
  `Void` explicitly, write `Void`
- **`return` exits the nearest function boundary** (passing through every block, rather than
  “returning to the block”), with type `Never`; `Never <: T` holds for any type (principle of
  explosion), so it may appear in any return type position. The module initialization layer (Script
  top level) has no function boundary, and `return` is rejected at compile time there (`E1109`; see
  §3.11)
- The expression form `= expr` directly yields a value

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

// return: passes through the block and exits the function
f: (n: Int) -> Int = {
    if n < 0 {
        return 0     // Passes out of if and the function body, exiting the function
    }
    n * 2            // Tail expression
}
```

#### Is `name = { ... }` a Function or a Block Value?

`name = { ... }` may be either a function definition (RFC-007 “Empty-Parameter Minimum”) or a
block-value binding. The decision follows **annotation first, function by default** (RFC-010a,
Appendix D):

| Case                                  | Result          | Example                            |
| ------------------------------------- | --------------- | ---------------------------------- |
| The value is a Lambda (`=>`)          | Function        | `f = () => 5` → `f()` = 5          |
| The annotation is a function type     | Function        | `f: () -> Int = { 5 }` → `f()` = 5 |
| The annotation is a non-function type | **Block value** | `x: Int = { 5 }` → `x` = 5         |
| No annotation                         | Function        | `f = { 5 }` → `f()` = 5            |

**The annotation is the type**: `x: Int = ...` declares that `x` is an `Int`, so `{ ... }` evaluates
to an `Int`; `f: () -> Int = ...` declares that `f` is a function, so `{ ... }` is the function
body.

To evaluate `{ ... }` immediately, **write the target type** (no new syntax is required):

```yaoxiang
// Block value: evaluated immediately
x: Int = {
    y = 5
    y            // x = 5
}

// Function: defaults when no annotation is present
f = { 5 }        // f() = 5
```

#### Nested Function Types: Currying or Returning a Function?

There are two ways to interpret a nested function type on the right-hand side of `->`, distinguished
by **parentheses** (RFC-004):

| Notation                        | Meaning                  | Call                     |
| ------------------------------- | ------------------------ | ------------------------ |
| `(a: Int) -> (b: Int) -> Int`   | **Currying**             | `f(1)(2)`                |
| `(a: Int) -> ((b: Int) -> Int)` | **Returning a function** | `g(1)` yields a function |

The basis is: **the annotation is the type**. `g: (a: Int) -> ((b: Int) -> Int)` declares that
`g(1) : (b: Int) -> Int`, so `g(1)` must **be** that function, rather than “the next set of
parameters”.

```yaoxiang
// Currying: provide two sets of parameters one layer at a time
add: (a: Int) -> (b: Int) -> Int = { a + b }
add(1)(2)        // → 3

// Returning a function: the outer call takes one parameter and returns a function
adder: (n: Int) -> ((x: Int) -> Int) = (x) => x + n
adder(10)(5)     // → 15
h = adder(10)    // The returned function can be stored in a variable or passed around
```

An unparenthesized nested `Fn` is always curried (including RFC-011's type-parameter forms):

```yaoxiang
identity: (T: Type) -> (x: T) -> T = (x) => x   // Curried
identity(5)      // → 5
```

**Type checking**: parentheses declare the return type, and the body must produce that type.
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

- On `Ok(v)`, it extracts the value `v` and continues execution
- On `Err(e)`, it propagates the error upward (`return Err(e)`)

```yaoxiang
process: (data: Data) -> Result(Data, Error) = {
    validated = validate(data)?     // Extracts the value on success; propagates on failure
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

// Range is a value: it can be bound, passed, and checked for membership
r = 1..10
assert.assert(5 in r, "membership")
for i in r { print(i) }

// Step form (third component, default 1)
for i in 0..10..2 { print(i) }  // 0, 2, 4, 6, 8
for i in 10..0..(-2) { print(i) }  // 10, 8, 6, 4, 2
```

> **Step semantics**: in `a..b..c`, `c` is the step size. The literal `c = 0` is rejected at compile
> time; a dynamic `c` receives a runtime zero check (the E6001 family; promoted to Result once the
> error system is implemented). `c < 0` is valid, and the interval direction reverses with the sign
> (`10..0..(-2)` counts downward).

### 2.13 ref Expressions

```
RefExpr     ::= 'ref' Expr
```

`ref` creates shared ownership. The compiler automatically selects Rc (single-task) or Arc
(cross-task), and users do not need to concern themselves with implementation details.

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

- Types and raw-pointer operations may be defined inside an `unsafe {}` block
- A returned type is available outside the `unsafe {}` block
- Accessing the type's fields requires unsafe permission

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

### 2.15 Scope

**Basic rules**:

- Every `{}` block creates a scope
- An inner scope may access variables from an outer scope
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

**Variable declarations and shadowing**:

- `x = value`: searches outward through the scope chain for x; if found, it assigns to it, and if
  not found, it creates a new declaration
- `mut x = value`: explicitly declares a new mutable binding and forbids the same name in an outer
  scope
- Within the same scope, any name may be declared only once

> **Detailed definitions**: see the [Module System Specification](modules.md#第四章作用域) for the
> complete scope rules, variable declarations, and shadowing mechanisms.

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

**Semantics**: `return` is a **non-local exit** that exits the nearest **function boundary**
(passing through every block—including `if` / `while` / `for` / `match` / bare blocks / `spawn` /
`unsafe`) and hands its value to the caller. It does **not “return to the block.”**

**Type**: `return e : Never` (`e : T`). `Never <: T'` holds for any `T'` (principle of explosion;
see [Type System §2.2](type-system.md)), so `return` may appear in any return type position without
requiring an additional rule.

**Relationship with block evaluation**: a block's value is always its **tail expression** (see
§2.9). As a block, `{ return n }` has the value `n` and type `Never`; at the same time, `return`
exits the function. **Both facts hold simultaneously**, coexisting through the principle of
explosion.

Together, `return` and tail expressions make “early return” possible without requiring a special
rule for `return` to target functions—see
[RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md).

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 {
        return 1          // Passes out of if and exits the function (type Never)
    }
    n * factorial(n - 1)  // Tail expression = the block's value
}
```

### 3.4 break Statements

```
BreakStmt   ::= 'break'
```

**Semantics**: immediately terminates the innermost containing `while`/`for` loop, transferring
control to after that loop's body.

- **Exits only the nearest level**: `break` always applies to the innermost containing loop. When a
  nested loop must be exited at multiple levels, extract the inner loop into a function and use
  `return`, or use a flag (break/continue have no labels; if loop labels are introduced in the
  future, they will follow the loop-declaration syntax through the RFC process and be decided
  together with the proof pipeline's multi-exit design)
- **Permitted only inside loops**: `break` may appear only in the body of a `while`/`for` loop
  (including nested blocks/`if`/`match` in the body). Appearing outside a loop is a compile error
  (E1102 `'break' outside of a loop`)
- **Does not affect termination proofs**: `break` does not participate in termination reasoning—it
  provides neither a measure nor a decreasing step in that measure. The loop's termination
  obligation is unrelated to `break` and is triggered by refinement types (see
  [type-system §8.4](type-system.md#84-terminates终止测度谓词)).
- **Borrow semantics**: the control-flow edge of `break` participates in the structural cutoff of
  RFC-009a's reverse BFS liveness analysis (iterations exited through it do not participate in
  back-edge liveness derivation)

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
next iteration of the innermost containing loop—`while` re-evaluates its condition, while `for`
takes the next element.

- **Applies only to the nearest level**: the same as `break`, with no labels
- **Permitted only inside loops**: appearing outside a loop is a compile error (E1102)

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

YaoXiang's for-loop semantics differ from those of traditional languages: **each iteration binds a
new value instead of modifying the same variable**.

```yaoxiang
// Example: for i in 1..5
for i in 1..5 {
    print(i)
}
```

**Execution process**:

| Iteration | Behavior of the Loop Variable                                                                      |
| --------- | -------------------------------------------------------------------------------------------------- |
| 1st       | Creates a new binding `i = 1`, executes the body, and prints 1                                     |
| 2nd       | Creates a new binding `i = 2` (the previous binding is destroyed), executes the body, and prints 2 |
| 3rd       | Creates a new binding `i = 3`, executes the body, and prints 3                                     |
| 4th       | Creates a new binding `i = 4`, executes the body, and prints 4                                     |
| End       | The loop body ends, and the binding is destroyed                                                   |

**Key point**: after each iteration ends, the binding created during that iteration is destroyed.
The next iteration is a completely new binding, unrelated to the binding from the previous
iteration.

#### 3.9.2 Difference Between for and for mut

| Syntax              | Loop Variable Mutability | Description                                     |
| ------------------- | ------------------------ | ----------------------------------------------- |
| `for i in 1..5`     | Immutable                | The binding cannot be modified in the loop body |
| `for mut i in 1..5` | Mutable                  | The binding can be modified in the loop body    |

```yaoxiang
// Valid: each iteration binds a new value, so no modification is needed
for i in 1..5 {
    print(i)  // Reads i's value
}

// Invalid: an immutable binding cannot be modified
for i in 1..5 {
    i = i + 1  // Error: an immutable binding cannot be modified
}

// Valid: for mut allows the binding to be modified
for mut i in 1..5 {
    i = i + 1  // Modification is allowed
}
```

#### 3.9.3 Shadowing Checks

YaoXiang prohibits variable shadowing. A for-loop variable cannot have the same name as a variable
in an outer scope:

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

This rule applies to all code blocks. See [4.3 Shadowing Rules](modules.md#43-遮蔽规则) for details.

#### 3.9.4 Comparison with Other Languages

| Language | for-Loop Variable Semantics                                  |
| -------- | ------------------------------------------------------------ |
| YaoXiang | Binds a new value on each iteration                          |
| Rust     | Modifies the same variable (requires `mut`)                  |
| Python   | Modifies the same variable (does not require `mut`)          |
| C/C++    | Modifies the same variable (requires a pointer or reference) |

**Design rationale**: YaoXiang uses binding semantics because:

1. **It better matches natural semantics.** In natural language, “for each element x in a set” means
   that each x is an independent entity. YaoXiang's `for i in 1..5` reads as “for each i from 1 to
   5,” where the i in each iteration is a completely new binding, consistent with human intuition.

2. **It prevents accidental modification.** Immutable bindings by default mean that the loop
   variable cannot be accidentally modified in the loop body. There is no need to worry about
   accidentally writing `i = ...` somewhere in a complex loop body and creating a bug that is
   difficult to track.

3. **High-performance solutions remain accessible.** When a variable genuinely must be reused across
   iterations (for example, an accumulator or cache), use a `for mut` declaration to switch to
   mutable-binding mode. This is clearer than implicit shared state—the intent is expressed
   explicitly through syntax rather than hidden in runtime behavior.

### 3.10 spawn Statements

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' 'mut'? Identifier 'in' Expr '{' Expr '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
```

**spawn block**: explicitly declares a concurrency realm, and expressions inside the block execute
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

**A spawn block captures outer variables** (RFC-024 §2.3, value-capture semantics):

- When the block body references an outer variable, it uses **Move value capture**: the value is
  snapshotted into the closure environment at the spawn creation point, and the block body reads it
  through the environment (LoadUpvalue)
- **Primitive types** (Int/Float/Bool/Char) are copied, and the outer variable is unaffected
- For **handle types** (Struct/String/List, etc.), snapshotting copies the handle and shares the
  underlying object. In the Embedded runtime (the default), the same thread and heap are used, and
  the handle remains valid
- Sharing across multiple tasks requires an explicit `ref` (§2.13; the compiler automatically
  selects Rc/Arc)
- An outer variable referenced by `return` inside the block is captured in the same way

```yaoxiang
t1 = 1 + 1
t2 = 2 + 2
result = spawn {
    return t1 + t2    // t1/t2 are captured by value; result == 6
}
```

---

### 3.11 Program Entry Point and Top-Level Statements

From the compiler's perspective, a source file has two possible roles, **determined by whether
`yaoxiang.toml` exists**:

| Role       | Determination                                       | Program Body                                                                        |
| ---------- | --------------------------------------------------- | ----------------------------------------------------------------------------------- |
| **Script** | A single file run directly, without `yaoxiang.toml` | **Top-level statements** (executed in written order); `main` is an ordinary binding |
| **Bin**    | `yaoxiang.toml` exists                              | **The `main` function**; top-level executable statements are forbidden              |

#### Script: Top-Level Statements Are the Program

Without a manifest, the file is a “script,” and **top-level statements execute in written order**
(all top-level **binding initialization** runs first; see
“[Initialization of Top-Level Bindings](#顶层绑定的初始化)” below):

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

main()                       // ← This line must be included
```

> **Why is `main` not called automatically?** The top-level statements are already the program body.
> If `main` were also called implicitly, a script that explicitly contains `main()` would execute
> twice. Both rules cannot coexist, so Script mode has only one execution entry point: the
> **top-level statements**.

The top level likewise **must not use `return`**: the Script top level is the module initialization
layer and has no function boundary to exit (RFC-010a rule ②). Such use is rejected at compile time
with `E1109`, rather than silently terminating initialization at runtime and skipping the remaining
top-level statements. To end a section of top-level logic early, place it in a function and control
it at the call site.

#### Bin: `main` Is the Entry Point

When a manifest exists, the file is an “executable target,” and:

- It must define a **binding** named `main`, whose value may be either a value or a function (#388
  establishes the decision: entry is determined by the existence of the binding—a function is
  already a value, and the two merely use different evaluation strategies): a function `main` is
  called with no arguments at entry; a value `main` is evaluated during initialization
- Top-level executable statements are forbidden—the program body is `main`

```yaoxiang
main: () -> Void = {
    print("hello")
}
```

A value `main` is equally valid—it is evaluated during initialization, within the initialization
sequence (global bindings follow topological order, while independent bindings follow source order):

```yaoxiang
main = {
    print("hello")
}
```

A missing `main` is a compile error (without `main`, all functions are unreachable). A non-callable
value `main` (such as `main: Int = 5`) is valid but has no observable effect—this is analogous to
Rust's empty `fn main() {}`.

> **Library files**: files imported by other files with `use`, or files referenced by `[lib].path` /
> `[exports]`, do not require `main`—they are not program entry points.

#### Initialization of Top-Level Bindings

Top-level binding initializers are **evaluated at runtime** and are not required to be compile-time
constants:

```yaoxiang
answer: Int = { 42 }              // Block value
inc: (Int) -> Int = (x) => x + 1
computed: Int = inc(41)           // Function call
```

Initialization executes in **dependency order**, independently of written order (dependencies
initialize first; bindings without dependencies initialize in declaration order):

```yaoxiang
derived: Int = base * 3           // References base, which is declared later
base: Int = 7                     // Initializes first (topological sorting)
```

Also, **all binding initialization precedes top-level statement execution**—side effects in binding
initializers occur before any top-level statement, regardless of how the two are interleaved in the
source:

```yaoxiang
print("a")                        // Executes later
x = { print("b") }                // Executes first (all binding initialization precedes the rest)
print("c")                        // Output order: b, a, c
```

Circular dependencies are compile errors (the compiler lists the names in the cycle):

```yaoxiang
a: Int = b + 1
b: Int = a + 1                    // Error: a → b → a
```

> **Design basis**: RFC-029f (file-role model), RFC-010a Appendix D (block-binding decision).

---

## Appendix: Syntax Quick Reference

### A.0 Keywords (17)

```
use     spawn   ref     mut
if      else    match   while   for
in      return  break   continue
as      unsafe  and     or
```

`type` is no longer a keyword (RFC-010; use `Name: Type = { ... }` instead), and `pub` was removed
entirely under RFC-029g and is now an ordinary identifier. Literal reserved words `true` / `false` /
`void` are covered in §1.4.1; the meta type name `Type` and built-in type names `Void` / `Never` /
`Int` / `Float` / `Bool` / `Char` / `String` are covered in §1.4.3.

### A.1 Control Flow

```
if Expr Block (else if Expr Block)* (else Block)?
match Expr { MatchArm+ }
while Expr Block
for 'mut'? Identifier 'in' Expr Block
break | continue          // Permitted only inside loops (§3.4 / §3.5)
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
