# Syntax Specification

This document defines the syntax specification of the YaoXiang programming language, including
lexical structure, grammar rules, and operator precedence.

---

## Chapter 1: Lexical Structure

### 1.1 Source Files

YaoXiang source files must use UTF-8 encoding. Source files usually use the `.yx` extension.

### 1.2 Lexical Token Categories

| Category   | Description                        | Examples                  |
| ---------- | ---------------------------------- | ------------------------- |
| Identifier | Starts with a letter or underscore | `x`, `_private`, `my_var` |
| Keyword    | Predefined reserved words          | `use`, `mut`, `and`       |
| Literal    | Fixed values                       | `42`, `"hello"`, `true`   |
| Operator   | Operation symbols                  | `+`, `-`, `*`, `/`        |
| Separator  | Syntactic separators               | `(`, `)`, `{`, `}`, `,`   |

### 1.3 Keywords

YaoXiang has **17 keywords** (`keyword_from_str` in `src/frontend/core/lexer/state.rs`, with each
one corresponding to a `TokenKind`):

```
use     spawn   ref     mut
if      else    match   while   for
in      return  break   continue
as      unsafe  and     or
```

`and` / `or` are **keywords** for logical AND / OR (Zig-style; see level 10 in §2.2 for precedence);
unary negation uses the symbol `!` and is orthogonal to them.

These keywords have special meanings in every context and cannot be used as identifiers.

> **`type` is no longer a keyword** (RFC-010): use the `Name: Type = { ... }` notation to write type
> definitions. Both `keyword_from_str` and the `TokenKind` enum header
> (`src/frontend/core/lexer/tokens.rs`) are annotated with “RFC-010: 'type' keyword removed”.
>
> **`pub` is no longer a keyword** (RFC-029g, accepted 2026-10-02): the language has no visibility
> mechanism, and `pub` has been removed wholesale from the lexer, AST, type checker, dead-code
> exemptions, formatter, and LSP ([RFC-029g](../../rfc/accepted/029g-remove-pub-and-auto-bind.md)).
> It is now an ordinary identifier—the old form `pub x = 1` reports E1001 as an undefined name. All
> top-level bindings are importable by default, and modifiers make no difference to visibility
> ([RFC-029](../../rfc/accepted/029-module-semantics.md)).

### 1.4 Reserved Words

YaoXiang's “reserved words” are divided into three layers, recognized by the parser and type checker
at different stages:

#### 1.4.1 Literal Reserved Words

The parser gives these literal identifiers dedicated tokens, so they cannot be used as ordinary
identifiers:

| Identifier | Type | Description                                                                                                   |
| ---------- | ---- | ------------------------------------------------------------------------------------------------------------- |
| `true`     | Bool | Boolean true value                                                                                            |
| `false`    | Bool | Boolean false value                                                                                           |
| `void`     | Void | Void literal (Unit value). Lowercase `void` is a value literal; uppercase `Void` is a type name (see §1.4.3). |

> **`Type` is not in this layer**: it has **no** dedicated `TokenKind` (it is absent from the
> `TokenKind` enum in `src/frontend/core/lexer/tokens.rs`, and `keyword_from_str` has no
> corresponding branch). The parser treats it as an ordinary identifier, while the type checker
> recognizes it as a meta type in type position. Therefore, `Type` can be shadowed by a local
> binding in expression position. It is a **meta type name**, not a keyword.

#### 1.4.2 Variant Names

Variant names are recognized by the parser in **pattern** contexts (a bare name is adjudicated by
the variant set of the scrutinee type; see §2.8). In **expression** contexts, constructing a variant
requires a type qualification—`Result(Int, String).ok(5)`, `Option(T).some(v)`, `Color.green()`
(RFC-010 records and types: a variant constructor is a function-typed field in the type definition;
a bare call such as `ok(5)` is not a constructor call).

| Variant name | Owning type | Description                      |
| ------------ | ----------- | -------------------------------- |
| `some(T)`    | Option      | Option value variant constructor |
| `ok(T)`      | Result      | Result success variant           |
| `err(E)`     | Result      | Result error variant             |

#### 1.4.3 Built-in Type Names

The type checker pre-registers the following type names, allowing them to be used in type positions
without importing them. The parser treats them as ordinary identifiers—**they are not reserved words
and can be shadowed by local bindings (not recommended)**.

| Type name | Logical correspondence | Description                                                                                                                                           |
| --------- | ---------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Void`    | ⊤ (true/Unit)          | A zero-field product type with exactly one inhabitant (the `void` literal; see §1.4.1).                                                               |
| `Never`   | ⊥ (false/empty type)   | A type with zero variants and zero inhabitants. No expression can produce a `Never` value. `Never <: T` holds for every `T` (principle of explosion). |
| `Int`     | —                      | Signed integer.                                                                                                                                       |
| `Float`   | —                      | Floating-point number.                                                                                                                                |
| `Bool`    | —                      | Boolean value: `true` / `false`.                                                                                                                      |
| `Char`    | —                      | Unicode character.                                                                                                                                    |
| `String`  | —                      | String.                                                                                                                                               |

### 1.5 Identifiers

Identifiers start with a letter or underscore; subsequent characters may be letters, digits, or
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
Array       ::= '[' Expr (',' Expr)* ']'   // A literal with an Array(T, N) target type annotation becomes a fixed-length array
```

> **A dictionary literal requires at least one key-value pair**: `{}` is **not** an empty
> dictionary—it is an empty block (the value `Void`; see [§2.9](#_2-9-块表达式)). To create an empty
> dictionary, use the constructor `dict.new()`:
>
> ```yaoxiang
> empty = dict.new()                  // ✅ Empty dictionary
> d = { "a": 1 }                       // ✅ Dictionary literal
> wrong = {}                           // ❌ Not a dictionary; an empty block (Void)
> ```
>
> **The criterion is the content**: The `Dict` grammar requires at least one `String ':' Expr`.
> Because `{}` has no content on which to base a decision, it takes the zero form of a block
> structure. A non-empty form is self-describing through its content (`{ "k": v }` has a key-value
> pair → dictionary). This has the same origin as `f = { 5 }` being an `Int` value rather than a
> function: **the type is determined by the content**.

> Set has no literal grammar and no runtime representation. While the set type is being planned, it
> will be completed by following the Dict pattern when the requirement arises (`std.set` +
> `HeapValue::Set`). The destination of List/Dict literals is determined by the contextual type
> annotation: a bare literal and a `List(T)` annotation resolve to the same growable list. The only
> type-surface name for a growable list is `Vec(T)`; a `List(T)` annotation is normalized by source
> name and resolves to the same type surface as `Vec(T)`. An `Array(T, N)` annotation applied
> directly to a literal resolves to a fixed-length array. Implicit List→Array conversion is
> prohibited.
>
> Array literal semantics:
>
> - The number of elements must equal N; otherwise, compilation fails with E1002. An empty literal
>   paired with a nonzero N is rejected in the same way.
> - Every element type must be compatible with T; otherwise, compilation fails with E1002.
> - The grammatical forms of N are an integer literal (possibly negative), a constant name, or a
>   constant expression beginning with a reference to a compile-time value parameter (continued with
>   infix arithmetic, such as `2+3` or `N+1`). Its folded value participates in the element-count
>   check (`2+3` has the same form as `5`).
> - Parse-time rejection: floating-point/string seeds (E0010) and incomplete expressions (such as
>   `2+`, E0012); an identifier followed by `(` takes the constructor type path—function calls
>   cannot serve as type-position actual arguments (E1003).
> - When N is a symbolic constant (such as a function `const` parameter in `Array(Int, n)`), count
>   validation is deferred to the refinement type checking stage.
> - In v1, nested array literals (such as `Array(Array(Int,2),2) = [[1,2],[3,4]]`) are rejected at
>   compile time and require explicit construction at each level; recursive destinations are
>   deferred to a later version.

#### 1.6.5 List Comprehensions

```
ListComp    ::= '[' Expr ( 'for' Identifier 'in' Expr ( 'if' Expr )? )+ ']'
```

> **#401 (implementation completion)**: `if` filters and multiple generator clauses are implemented
> as of this version. The previous grammar was written as `(',' Expr)* ('if' Expr)?`, with no
> corresponding implementation (comma-appended expressions and multiple generators were both
> unsupported), and the parser omitted `if` filters entirely (after parsing the iterable, it
> expected `']'` and reported `E0010 Expected RBracket, found KwIf`). The semantics now follow
> common industry practice: there may be one or more generator clauses, expanded as nested loops;
> each clause may carry at most one `if` filter, whose condition must be `Bool` (a non-Bool
> condition reports `E1054`); the iterable and condition of a later clause may refer to iteration
> variables bound by earlier clauses.

> **Tightened behavior (migration record)**: The iteration-variable grammar has always been
> `'for' Identifier 'in'`, but the old implementation parsed the pattern with the full Pratt
> parser—after `in` was registered as an infix operator, `x` would swallow `in items` as a
> membership expression. After the fix, a non-identifier pattern fails directly during parsing and
> no longer falls back to `_` (silently swallowing the error). Impact: forms such as
> `[x for (a, b) in pairs]`, which could previously be parsed, now produce an error. This form never
> had defined behavior—the variable was always `_`—so tightening it in this direction is correct and
> carries no semantic migration cost.

#### 1.6.6 Membership Testing

```
Membership  ::= Expr 'in' Expr
```

> `in` is a binary relational operator that returns `Bool`: `true` for a match and `false` for no
> match, without raising an error. The semantics are split: `[]` asserts existence and retrieves a
> value (failure is an error), whereas `in` asks whether it exists (a miss is the normal `false`
> result). Right-operand coverage includes List / Array / Dict (key set) / Tuple / String
> (substring) / Range (interval). `in` is a first-class HORN predicate and serves as the foundation
> for compile-time provable propositions during the refinement type checking stage. (Set is omitted
> from the right-operand list—Set has no runtime representation; see §1.6.4.)

### 1.7 Comments

```
// Single-line comment

/* Multi-line comment
   can span multiple lines */
```

### 1.8 Indentation Rules

Code must use four spaces for indentation. Tab characters are prohibited. This is a mandatory syntax
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

> **Unary prefix operators** (`!` `-` `+`) bind tightly: they are lower only than calls and member
> access, and higher than every binary operator. Therefore, `!a == b` is equivalent to `(!a) == b`
> (Zig-style semantics). `!` is a pure unary operation and does not participate in short-circuiting
> control flow; it is orthogonal to the `and`/`or` keywords (which short-circuit), as
> authoritatively defined by RFC-010.

> **Range binding power**: `..` has binding power (6, 7)—left binding power 6 is lower than addition
> (7), while right binding power 7 consumes addition but not another `..` at the same level.
> Comparison before and after the change:
>
> | Expression   | Before the change (level 1, right-associative)                                                                | After the change ((6,7), left-associative)                                     |
> | ------------ | ------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------ |
> | `x in 1..10` | `x in 1..10` (`in` has right-operand level 4, and level-1 `..` cannot consume it, so parsing fails)           | `x in (1..10)`—the entire interval is the right operand of `in`                |
> | `0..n+2`     | `(0..n)+2` (right-associativity trap: the upper bound is consumed, and the `for` loop directly reports E3004) | `0..(n+2)`—the upper bound is an arithmetic expression                         |
> | `a == b..c`  | `a == (b..c)` (level-1 `..` < level-3 `==`, so it is naturally grouped as a whole)                            | `a == (b..c)`—**semantics are unchanged**; `..` remains above comparison level |
> | `1..2*3`     | `(1..2)*3`                                                                                                    | `1..(2*3)`—the upper bound is an arithmetic expression                         |
> | `a..b..c`    | `a..(b..c)` (a right-associative chain, producing meaningless nested ranges)                                  | `(a..b)..c`—**step form** (`c` is the step size)                               |
>
> The net effect is that a compound upper bound such as `for i in 0..n+2` changes from “parses
> successfully but reports E3004” to “directly usable”; `x in 1..10` changes from “cannot be parsed”
> to “an interval check”; and `a..b..c` changes from “meaningless nesting” to “a step component”.
> Level 6 lies between `+` (level 5) and `<<` (level 7), following mathematical convention: an
> interval is a tightly bound construct, so its upper bound is naturally a complete arithmetic
> expression.

### 2.3 Function Calls

```
FnCall      ::= Expr '(' ArgList? ')'
ArgList     ::= Expr (',' Expr)* (',' NamedArg)* | NamedArg (',' NamedArg)*
NamedArg    ::= Identifier '=' Expr
```

Named arguments use `name = value` (RFC-010 §Function Definitions, RFC-011 §Constructor Forms).
Positional arguments must precede named arguments; arguments whose order is specified may appear in
any order among the named arguments:

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b

add(3, 5)          // Positional
add(a = 3, b = 5)  // Named
add(b = 5, a = 3)  // Any order
add(3, b = 5)      // Mixed; positional first
```

An incorrect named-argument name reports **E1014**, specifying the same parameter both positionally
and by name reports **E1015**, and an incorrect argument count reports **E1010** (RFC-013).

**Unnamed parameters and named arguments**: Signature parameter positions allow a bare type form
(`mk: (Int, Int) -> Int`). This is an **unnamed typed parameter**—type constraints apply by
position, while the name belongs to the implementation (the lambda header) and is not part of the
contract (see the shorthand rules in
[RFC-007](../../rfc/accepted/007-function-syntax-unification.md)). An unnamed parameter cannot be
referenced by a named argument because it has no name; it can only be passed positionally. To
include a name in the contract, use `mk: (a: Int, b: Int) -> Int`. A bare identifier must be a
declared type; if the parser cannot resolve it as a type, it reports an error (see
[type-system §3.5](type-system.md#35-函数类型)).

**Conversely, named parameters: the name enters the contract and the declaration is authoritative**.
Parameter names in an implementation header (the parameter list after `=>`) are merely local names
within the body and bind **by position**—in `f: (a: Int) -> Int = (b) => b`, the implementation
header uses `b`, but the call site still uses `f(a = 7)`. Type annotations in the implementation
header are used only for comparison and report **E1011** when they disagree with the declaration. An
implementation header with more parameters than the declaration has runtime parameter slots reports
**E1112** (`f: () -> Int = (a: Int) => a`); in both cases, the declaration is the authoritative
source. The reverse case, where the implementation omits parameters, does not report an error: the
type layer checks arity according to the declaration, but positions omitted by the implementation
header have no runtime slots, and arguments passed to those positions are discarded on the IR side
(see the gap recorded in [type-system §3.5](type-system.md#35-函数类型)).

### 2.4 Member Access

```
MemberAccess::= Expr '.' Identifier
```

### 2.5 Index Access

```
IndexAccess ::= Expr '[' Expr ']'
```

> **Three-layer semantics (RFC-011b)**: `a[i]` dispatches according to the type of `a`: ① built-in
> containers (List/Vec/Array/Dict/Tuple) use the native indexing instruction (fast path); ② user
> types that implement the `Index` interface dispatch to their `index` method (instantiating
> `Index(Grid, Int, Float)` in the type body and providing a `Grid.index` method); ③ RFC-004's
> `f[0]` positional binding exists only in binding declarations and does not use this grammar. For
> multidimensional indexing, the key in `a[0, 1]` is packed into a tuple. Other types that do not
> implement `Index` are rejected at the type layer.

### 2.6 Type Casts

```
TypeCast    ::= Expr 'as' TypeExpr
```

> **Implementation status (an existing gap, not introduced by this version)**: The above defines
> only the grammar and precedence (§2.4, level 2); the semantics have not yet been implemented.
> Independently verifiable facts: ① Parsing: the `KwAs` infix arm in `parser/pratt/led.rs:81`
> produces `ast::Expr::Cast`; ② Type checking: `typecheck/inference/expressions.rs:3847-3853`
> performs only a **silent re-labeling** for `Expr::Cast`—`let _ = self.infer_expr(expr)?;` discards
> the inner result and then directly returns the `target_type` `MonoType`; ③ There is **no**
> convertibility validation anywhere in the repository, nor is there a corresponding diagnostic code
> (`util/diagnostic/codes/` has no cast entry); ④ `ir_gen` generates no instructions (the only uses
> of `Expr::Cast` in `middle/core/ir_gen.rs` are at `:5111`, `:5918`, and `:9381`, where they obtain
> the span and free variables of `Expr::Cast`—the first two are span arms in `get_expr_span` /
> `expr_span`, and the third is `collect_free_vars_expr`; none of the three generates instructions);
> ⑤ Consequently, `x as Int` currently neither reports an error nor produces any runtime conversion.

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

> **Variant-set requirements for variant destructuring** (RFC-010b): `EnumPattern` (variant names
> such as `ok(v)` and `some(x)` are adjudicated against the scrutinee) requires the scrutinee's type
> to be a **sum type with the corresponding variant set present**. A variant set enters the checker
> only through a type definition or a `use` import—`Result` / `Option` are defined by `std.result` /
> `std.option`, so `use std.result` / `use std.option` is required before use (whole-module and
> grouped `use std.{...}` forms have equal status); otherwise, variant destructuring reports E1002.
> Exhaustiveness checking uses the same source: a fallback arm exempts the match from
> exhaustiveness; without a fallback arm, the complete variant set is checked.

> **Pattern bindings, or-patterns, and guards** (RFC-010b):
>
> - **Bindings**: A bare identifier in a pattern introduces a **new name**—it is neither a
>   comparison nor an assignment. The name is visible **within the arm**, shadows an outer variable
>   of the same name, and expires on leaving the arm. A discriminant position (such as `ok` in
>   `ok(v)`) is resolved through the scrutinee's variant set; a bare name that is not found is a
>   binding.
> - **Or-patterns**: `p1 | p2` tries the alternatives in order. If a non-final alternative fails,
>   the next alternative is tried; if the final alternative fails, control proceeds to the next arm.
>   All alternatives must bind **the same set of names**, or E1033 is reported. A binding with the
>   same name exists only once at the arm level, and the arm body necessarily reads the value
>   written by the matching alternative.
> - **Guards**: After a pattern matches, the `if` condition is evaluated, with the pattern bindings
>   visible to it. The condition must produce `Bool`; if false, control proceeds to the next arm.
>   Arms with guards **do not count toward exhaustive coverage** (a guard may fail to match), but a
>   repeated variant arm with a guard is reachable (it refines a preceding arm).
> - **Diagnostics**: a missing branch reports E1030, unreachable code reports E1031, a duplicate
>   binding in the same pattern reports E1032, inconsistent or-pattern bindings report E1033, and a
>   struct pattern with a missing field reports E1034. See the
>   [error code reference](../error-code/E1xxx.md) for the code table and messages.

### 2.9 Block Expressions

```
Block       ::= '{' Stmt* Expr? '}'
```

> **Statement termination rules**: Separation between statements and newline behavior (`;` explicit
> separators, newline termination, continuation exceptions, and line-leading `(`/`[` never merging)
> are defined by [RFC-038](../../rfc/accepted/038-statement-termination.md).

#### 2.9.1 The Three Forms of `{`

In expression position, `{` has exactly three interpretations, determined in one pass by its
**content**:

| Form                   | Notation          | Type                 | Example          |
| ---------------------- | ----------------- | -------------------- | ---------------- |
| **Empty block**        | `{}`              | `Void`               | `x: Void = {}`   |
| **Dictionary literal** | `{ "k": v, ... }` | `Dict(K, V)`         | `d = { "a": 1 }` |
| **Block**              | `{ Stmt* Expr? }` | Tail expression type | `y = { 1 + 1 }`  |

**Decision order**:

1. `{}` (no content) → **empty block**, with the value `Void`
2. The first element is a `String ':' Expr` key-value pair → **dictionary literal**
3. Otherwise → **block**, with its value supplied by the tail expression

> **Why `{}` is not an empty dictionary**: The “emptiness” of an empty dictionary cannot be
> self-describing (it could be either `Dict(K, V)` or an empty block), while the dictionary grammar
> in [§1.6.4](#_1-6-4-集合) requires at least one key-value pair. When there is no content on which
> to base a decision, the zero form of the block structure is used: this is consistent with
> `unsafe {}` / `spawn {}` and introduces no special case. To create an empty dictionary, use
> `dict.new()`.
>
> **Why functions require an annotation**: `f = { stmt }` is a **value** (with the tail expression's
> type), not a function. To define a function, specify an Fn annotation: `f: () -> Int = { 5 }`.
> This follows the same principle as dictionaries: **the type is determined by the content**, not by
> whether an annotation exists. (See Appendix D of
> [RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md).)

**Unified semantics**: The value of every `{}` block is supplied by its **tail expression**, and
`return` is a non-local exit of type `Never`.

| Block type    | Value exit      | Empty block `{}` |
| ------------- | --------------- | ---------------- |
| Ordinary `{}` | Tail expression | `Void`           |
| `unsafe {}`   | Tail expression | `Void`           |
| `spawn {}`    | Tail expression | `Void`           |

**Core principles** (see [RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md) for
details):

- **A block's value = its tail expression** (the final expression), its only exit, with no
  exceptions.
- When the final element is an **assignment statement**, the block's value is `Void`; to obtain
  `Void` explicitly, write `Void`.
- **`return` exits the nearest function boundary** (passing through every block rather than
  “returning to the block”), with type `Never`; `Never <: T` holds for any type (principle of
  explosion), so it can appear in any return type position. The module initialization layer (Script
  top level) has no function boundary, so `return` is rejected at compile time there (`E1109`; see
  §3.11).
- In expression form, `= expr` directly supplies the value.

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
        return 0     // Exits both the if and the function body
    }
    n * 2            // Tail expression
}
```

#### Is `name = { ... }` a Function or a Block Value?

`name = { ... }` may be either a function definition (RFC-007 “simplest empty-parameter form”) or a
block-value binding. The decision is **annotation first, function by default** (Appendix D of
RFC-010a):

| Situation                             | Result          | Example                            |
| ------------------------------------- | --------------- | ---------------------------------- |
| The value is a Lambda (`=>`)          | Function        | `f = () => 5` → `f()` = 5          |
| The annotation is a function type     | Function        | `f: () -> Int = { 5 }` → `f()` = 5 |
| The annotation is a non-function type | **Block value** | `x: Int = { 5 }` → `x` = 5         |
| No annotation                         | Function        | `f = { 5 }` → `f()` = 5            |

**The annotation is the type**: `x: Int = ...` declares `x` as `Int`, so `{ ... }` evaluates as
`Int`; `f: () -> Int = ...` declares `f` as a function, so `{ ... }` is the function body.

To evaluate `{ ... }` immediately, **simply specify the target type** (no new syntax is needed):

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

There are two ways to read a nested function type on the right side of `->`, distinguished by
**parentheses** (RFC-004):

| Notation                        | Meaning                | Invocation               |
| ------------------------------- | ---------------------- | ------------------------ |
| `(a: Int) -> (b: Int) -> Int`   | **Curried**            | `f(1)(2)`                |
| `(a: Int) -> ((b: Int) -> Int)` | **Returns a function** | `g(1)` yields a function |

The basis is: **the annotation is the type**. `g: (a: Int) -> ((b: Int) -> Int)` declares
`g(1) : (b: Int) -> Int`, so `g(1)` must **be** that function, rather than “the next parameter
section”.

```yaoxiang
// Curried: two parameter sections are supplied one layer at a time
add: (a: Int) -> (b: Int) -> Int = { a + b }
add(1)(2)        // → 3

// Returns a function: the outer parameter section returns the function
adder: (n: Int) -> ((x: Int) -> Int) = (x) => x + n
adder(10)(5)     // → 15
h = adder(10)    // The returned function can be stored in a variable and passed around
```

An unparenthesized nested `Fn` is always curried (including RFC-011's type-parameter form):

```yaoxiang
identity: (T: Type) -> (x: T) -> T = (x) => x   // Curried
identity(5)      // → 5
```

**Type checking**: Parentheses declare the return type, so the body must produce that type.
`f: () -> (() -> Int) = { 7 }` is an error—`() -> Int` is expected, but `Int` is produced (E1002).

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

- With `Ok(v)`, extract the value `v` and continue execution.
- With `Err(e)`, propagate the error upward (`return Err(e)`).

```yaoxiang
process: (data: Data) -> Result(Data, Error) = {
    validated = validate(data)?     // Extract the value on success; propagate upward on failure
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

> **Step semantics**: In `a..b..c`, `c` is the step size. The literal `c = 0` is rejected at compile
> time; a dynamic `c` receives a runtime zero check (E6001 family; promoted to Result once the error
> system is implemented). `c < 0` is valid, and the interval direction is reversed according to the
> sign (`10..0..(-2)` descends).

### 2.13 ref Expressions

```
RefExpr     ::= 'ref' Expr
```

`ref` creates shared ownership. The compiler automatically selects Rc (single-task) or Arc
(cross-task), so users do not need to concern themselves with implementation details.

```yaoxiang
data = ref heavy_data
spawn { use(data) }   // Across tasks: the compiler automatically selects Arc
```

### 2.14 unsafe Expressions

```
UnsafeExpr  ::= 'unsafe' Block
```

`unsafe` blocks are used to define opaque types and operate on raw pointers. Use `return` to return
a type definition to the enclosing scope.

**Semantics**:

- Types and raw-pointer operations can be defined inside `unsafe {}`.
- A returned type is available outside `unsafe {}`.
- Accessing the type's fields requires unsafe permission.

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
- An inner scope can access variables in outer scopes.
- An outer scope cannot access variables in inner scopes.
- Variable declarations follow the “assignment first” principle.

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

**Variable declarations and shadowing**:

- `x = value`: search outward through the scope chain for x; if found, assign to it; if not found,
  declare a new binding.
- `mut x = value`: explicitly declare a new mutable binding; the same name as an outer binding is
  prohibited.
- Any name may be declared only once within the same scope.

> **Detailed definition**: See [Module System Specification](modules.md#第四章作用域) for the
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
`unsafe`) and passes its value to the caller. It does **not** “return to the block”.

**Type**: `return e : Never` (`e : T`). `Never <: T'` holds for any `T'` (principle of explosion;
see [Type System §2.2](type-system.md)), so `return` can appear in any return type position without
requiring an additional rule.

**Relationship with block evaluation**: A block's value is always its **tail expression** (see
§2.9). As a block, `{ return n }` has the value `n` and type `Never`; at the same time, `return`
exits the function. **Both facts hold simultaneously**, coexisting through the principle of
explosion.

`return` and tail expressions together make “early return” possible, without requiring an additional
rule that `return` specifically target functions—see
[RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md).

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 {
        return 1          // Exits the if and exits the function (type Never)
    }
    n * factorial(n - 1)  // Tail expression = the block's value
}
```

### 3.4 break Statements

```
BreakStmt   ::= 'break'
```

**Semantics**: Immediately terminates the innermost enclosing `while`/`for` loop, transferring
control to after that loop's body.

- **Exits only the nearest level**: `break` always applies to the innermost loop containing it. When
  multiple nested loop levels must be exited, extract the inner loop into a function and use
  `return`, or use a flag (break/continue do not carry labels; if loop labels are introduced in the
  future, their declaration-side syntax will follow the RFC process and be adjudicated together with
  the multi-exit design of the proof pipeline).
- **Permitted only inside a loop**: `break` may occur only in a `while`/`for` loop body, including
  nested blocks/`if`/`match` statements within the body. Using it outside a loop produces a compile
  error (E1102 `'break' outside of a loop`).
- **Does not affect termination proofs**: `break` does not participate in termination arguments—it
  neither provides a measure nor constitutes a decreasing measure step. A loop's termination
  obligation is independent of `break` and is triggered by refinement types (see
  [type-system §8.5](type-system.md#85-terminates终止测度谓词)).
- **Borrow semantics**: The control-flow edge of `break` participates in the structural cut-off of
  RFC-009a's reverse-BFS liveness analysis (iterations exited through the break do not participate
  in back-edge liveness derivation).

```yaoxiang
mut i = 0
while i < 10 {
    i = i + 1
    if i == 3 {
        break              // Control flows to after the loop; i == 3
    }
}

// Nested loops: break exits only the inner loop
while j < 3 {
    while k < 10 {
        if k == 2 { break }    // Terminates only the inner loop
    }
    j = j + 1                  // Executed here on every outer iteration
}
```

### 3.5 continue Statements

```
ContinueStmt::= 'continue'
```

**Semantics**: Skips the remaining statements in the current iteration and proceeds directly to the
next iteration of the innermost enclosing loop—`while` re-evaluates its condition, while `for`
obtains the next element.

- **Applies only to the nearest level**: Like `break`, it carries no label.
- **Permitted only inside a loop**: Using it outside a loop produces a compile error (E1102).

```yaoxiang
mut sum = 0
mut n = 0
while n < 5 {
    n = n + 1
    if n == 3 {
        continue           // Skip the accumulation below; n == 3 is not included
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
new value rather than modifying the same variable**.

```yaoxiang
// Example: for i in 1..5
for i in 1..5 {
    print(i)
}
```

**Execution process**:

| Iteration | Loop variable behavior                                                                             |
| --------- | -------------------------------------------------------------------------------------------------- |
| First     | Creates a new binding `i = 1`, executes the body, and prints 1                                     |
| Second    | Creates a new binding `i = 2` (the previous binding is destroyed), executes the body, and prints 2 |
| Third     | Creates a new binding `i = 3`, executes the body, and prints 3                                     |
| Fourth    | Creates a new binding `i = 4`, executes the body, and prints 4                                     |
| End       | The body ends and the binding is destroyed                                                         |

**Key point**: After each iteration ends, the binding created during that iteration is destroyed.
The next iteration creates a completely new binding, unrelated to the previous one.

#### 3.9.2 Difference Between for and for mut

| Syntax              | Loop variable mutability | Description                                     |
| ------------------- | ------------------------ | ----------------------------------------------- |
| `for i in 1..5`     | Immutable                | The binding cannot be modified in the loop body |
| `for mut i in 1..5` | Mutable                  | The binding can be modified in the loop body    |

```yaoxiang
// Valid: a new value is bound each iteration; no modification is needed
for i in 1..5 {
    print(i)  // Read the value of i
}

// Invalid: an immutable binding cannot be modified
for i in 1..5 {
    i = i + 1  // Error: cannot modify an immutable binding
}

// Valid: for mut permits modifying the binding
for mut i in 1..5 {
    i = i + 1  // Modification is allowed
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

This rule applies to all code blocks. See [4.3 Shadowing Rules](modules.md#43-遮蔽规则).

#### 3.9.4 Comparison with Other Languages

| Language | for-loop variable semantics                                  |
| -------- | ------------------------------------------------------------ |
| YaoXiang | Binds a new value on each iteration                          |
| Rust     | Modifies the same variable (requires `mut`)                  |
| Python   | Modifies the same variable (does not require `mut`)          |
| C/C++    | Modifies the same variable (requires pointers or references) |

**Design rationale**: YaoXiang adopts binding semantics because:

1. **It better matches natural semantics.** In natural language, “for each element x in a
   collection” means that each x is an independent individual. YaoXiang's `for i in 1..5` reads as
   “for each i from 1 through 5,” and the i in each iteration is a completely new binding. This is
   consistent with human intuition.
2. **It avoids accidental modification.** Immutable bindings by default mean that the loop variable
   cannot be accidentally modified within the loop body. There is no need to worry about
   accidentally writing `i = ...` somewhere in a complex loop body and creating a difficult-to-trace
   bug.
3. **High-performance solutions remain readily available.** When it is genuinely necessary to reuse
   a variable across iterations—for example, as an accumulator or cache—use a `for mut` declaration
   to switch to mutable-binding mode. This is clearer than implicit shared state: the intent is
   expressed explicitly in the syntax rather than hidden in runtime behavior.

### 3.10 spawn Statements

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' 'mut'? Identifier 'in' Expr '{' Expr '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
```

**spawn block**: Explicitly declares a concurrency boundary; expressions inside the block execute
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

**A spawn block captures outer variables** (RFC-024 §2.3, value-capture semantics):

- A reference to an outer variable in the block body is a **Move value capture**: the value is
  snapshotted into the closure environment at the spawn creation point, and the block body reads it
  through the environment (LoadUpvalue).
- **Primitives** (Int/Float/Bool/Char) are copied, and the outer variable is unaffected.
- For **handle types** (Struct/String/List, etc.), a snapshot copies the handle and shares the
  underlying object; in the Embedded runtime (the default), handles are valid on the same thread and
  heap.
- Sharing between multiple tasks requires an explicit `ref` (§2.13; the compiler automatically
  selects Rc/Arc).
- A `return` inside the block also captures any referenced outer variables.

```yaoxiang
t1 = 1 + 1
t2 = 2 + 2
result = spawn {
    return t1 + t2    // t1/t2 value capture; result == 6
}
```

---

### 3.11 Program Entry Point and Top-Level Statements

From the compiler's perspective, a source file has two possible roles, **determined by whether
`yaoxiang.toml` exists**:

| Role       | Determination                                       | Program body                                                                        |
| ---------- | --------------------------------------------------- | ----------------------------------------------------------------------------------- |
| **Script** | A single file run directly, without `yaoxiang.toml` | **Top-level statements** (executed in written order); `main` is an ordinary binding |
| **Bin**    | `yaoxiang.toml` exists                              | **The `main` function**; top-level executable statements are prohibited             |

#### Script: Top-Level Statements Are the Program

Without a manifest, the file is a “script”, and **top-level statements execute in written order**
(top-level **binding initialization** as a whole occurs first; see
“[Top-Level Binding Initialization](#顶层绑定的初始化)” below):

```yaoxiang
use std.io
io.println("hello")          // Execute directly
x: Int = { 42 }              // Top-level binding: initialize at runtime
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
> `main` were also invoked implicitly, a script containing an explicit `main()` would execute twice.
> The two rules cannot coexist, so Script mode has only one execution entry point: “top-level
> statements”.

Top-level code must likewise **not use `return`**: the Script top level is the module initialization
layer and has no function boundary to exit (RFC-010a rule ②). It reports `E1109` at compile time
rather than silently terminating initialization at runtime and skipping the remaining top-level
statements. To end a section of top-level logic early, place it in a function and control it at the
call site.

#### Bin: `main` Is the Entry Point

When a manifest exists, the file is an “executable target”, in which case:

- A binding named `main` **must** be defined, and may be either a value or a function (#388
  decision: the entry point is determined by the existence of the binding—a function is already a
  value, and the two merely differ in evaluation strategy). A function `main` is called with zero
  arguments during the entry phase; a value `main` is evaluated during the initialization phase and
  therefore executes.
- Top-level executable statements are prohibited—the program body is `main`.

```yaoxiang
main: () -> Void = {
    print("hello")
}
```

A value `main` is equally valid—it is evaluated during initialization and therefore executes as part
of the initialization sequence (global bindings follow a topological order, with independent
bindings following source order):

```yaoxiang
main = {
    print("hello")
}
```

A missing `main` is a compile error (without `main`, every function is unreachable). An uncallable
value `main` (such as `main: Int = 5`) is valid but has no observable effect—analogous to Rust's
empty `fn main() {}`.

> **Library files**: Files imported by other files with `use`, or files referenced by `[lib].path` /
> `[exports]`, do not require `main`—they are not program entry points.

#### Top-Level Binding Initialization

The initialization values of top-level bindings are **evaluated at runtime** and are not required to
be compile-time constants:

```yaoxiang
answer: Int = { 42 }              // Block value
inc: (Int) -> Int = (x) => x + 1
computed: Int = inc(41)           // Function call
```

Initialization executes in **dependency order**, independently of written order (dependencies are
initialized first; bindings without dependencies follow declaration order):

```yaoxiang
derived: Int = base * 3           // References base, which is declared later
base: Int = 7                     // Initialized first (topological sorting)
```

Furthermore, **all binding initialization occurs before top-level statements execute**—side effects
in binding initializers occur before any top-level statement, regardless of how the two are
interleaved in the source:

```yaoxiang
print("a")                        // Executes later
x = { print("b") }                // Executes first (binding initialization as a whole comes first)
print("c")                        // Output order: b, a, c
```

Circular dependencies are compile errors, and the compiler lists the names on the cycle:

```yaoxiang
a: Int = b + 1
b: Int = a + 1                    // Error: a → b → a
```

> **Design basis**: RFC-029f (file-role model) and Appendix D of RFC-010a (block-binding
> adjudication).

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
entirely by RFC-029g and is now an ordinary identifier. The literal reserved words `true` / `false`
/ `void` are described in §1.4.1; the meta type name `Type` and the built-in type names `Void` /
`Never` / `Int` / `Float` / `Bool` / `Char` / `String` are described in §1.4.3.

### A.1 Control Flow

```
if Expr Block (else if Expr Block)* (else Block)?
match Expr { MatchArm+ }
while Expr Block
for 'mut'? Identifier 'in' Expr Block
break | continue          // Only inside a loop body (§3.4 / §3.5)
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
