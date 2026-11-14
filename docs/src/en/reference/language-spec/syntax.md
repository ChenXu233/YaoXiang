# Syntax Specification

This document defines the syntax specification of the YaoXiang programming language, including
lexical structure, grammar rules, and operator precedence.

---

## Chapter 1: Lexical Structure

### 1.1 Source Files

YaoXiang source files must use UTF-8 encoding. Source files conventionally use the `.yx` extension.

### 1.2 Lexical Token Categories

| Category   | Description                 | Examples                  |
| ---------- | --------------------------- | ------------------------- |
| Identifier | Begins with a letter or `_` | `x`, `_private`, `my_var` |
| Keyword    | Reserved by the language    | `use`, `mut`, `and`       |
| Literal    | Fixed value                 | `42`, `"hello"`, `true`   |
| Operator   | Operator symbol             | `+`, `-`, `*`, `/`        |
| Separator  | Grammar separator           | `(`, `)`, `{`, `}`, `,`   |

### 1.3 Keywords

YaoXiang has **17 keywords** (`keyword_from_str` in `src/frontend/core/lexer/state.rs`, with one
corresponding `TokenKind` for each):

```
use     spawn   ref     mut
if      else    match   while   for
in      return  break   continue
as      unsafe  and     or
```

`and` and `or` are **keywords** for logical AND and OR (Zig-style; see level 10 in §2.2). The unary
NOT operator is the symbol `!`, which is orthogonal to them.

These keywords have special meanings in every context and cannot be used as identifiers.

> **`type` is no longer a keyword** (RFC-010): use the `Name: Type = { ... }` notation to define a
> type. Both `keyword_from_str` and the `TokenKind` enum header
> (`src/frontend/core/lexer/tokens.rs`) are marked with “RFC-010: 'type' keyword removed”.
>
> **`pub` is no longer a keyword** (RFC-029g, accepted on 2026-10-02): the language has no
> visibility mechanism. `pub` has been removed wholesale from lexing, the AST, type checking,
> dead-code exemptions, the formatter, and the LSP
> ([RFC-029g](../../rfc/accepted/029g-remove-pub-and-auto-bind.md)). It is now an ordinary
> identifier—the legacy form `pub x = 1` reports E1001 as an undefined name. All top-level bindings
> are importable by default, and modifiers make no difference to visibility
> ([RFC-029](../../rfc/accepted/029-module-semantics.md)).

### 1.4 Reserved Words

YaoXiang's “reserved words” are divided into three layers, recognized by the parser and type checker
at different stages:

#### 1.4.1 Literal Reserved Words

The parser has separate tokens for the following literal identifiers, which cannot be used as
ordinary identifiers:

| Identifier | Owning Type | Description                                                                                                   |
| ---------- | ----------- | ------------------------------------------------------------------------------------------------------------- |
| `true`     | Bool        | Boolean true value                                                                                            |
| `false`    | Bool        | Boolean false value                                                                                           |
| `void`     | Void        | Void literal (Unit value). Lowercase `void` is a value literal; uppercase `Void` is a type name (see §1.4.3). |

> **`Type` is not in this layer**: it does **not** have a dedicated `TokenKind` (there is no `Type`
> entry in the `TokenKind` enum in `src/frontend/core/lexer/tokens.rs`, and `keyword_from_str` has
> no corresponding branch). The parser treats it as an ordinary identifier, while the type checker
> recognizes it as a metatype in type position. Therefore, in expression position, `Type` may be
> shadowed by a local binding. It is a **metatype name**, not a keyword.

#### 1.4.2 Variant Names

Variant names are recognized by the parser in **pattern** contexts (a bare name is resolved
according to the variant set of the scrutinee type; see §2.8). In **expression** contexts,
constructing a variant requires type qualification—`Result(Int, String).ok(5)`, `Option(T).some(v)`,
`Color.green()` (under RFC-010 record types and types, variant constructors are function-typed
fields in type definitions, so a bare name such as `ok(5)` is not a constructor call).

| Variant Name | Owning Type | Description              |
| ------------ | ----------- | ------------------------ |
| `some(T)`    | Option      | Option value constructor |
| `ok(T)`      | Result      | Result success variant   |
| `err(E)`     | Result      | Result error variant     |

#### 1.4.3 Built-in Type Names

The following type names are pre-registered by the type checker and can be used in type positions
without an import. The parser treats them as ordinary identifiers—they are **not reserved words and
can be shadowed by local bindings (not recommended)**.

| Type Name | Logical Correspondence | Description                                                                                                                                 |
| --------- | ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| `Void`    | ⊤ (true/Unit)          | A zero-field product type with exactly one inhabitant (the `void` literal; see §1.4.1)                                                      |
| `Never`   | ⊥ (false/empty type)   | A zero-variant, zero-inhabitant type. No expression can produce a `Never` value. `Never <: T` holds for every `T` (principle of explosion). |
| `Int`     | —                      | Signed integer                                                                                                                              |
| `Float`   | —                      | Floating-point number                                                                                                                       |
| `Bool`    | —                      | Boolean value: `true` / `false`                                                                                                             |
| `Char`    | —                      | Unicode character                                                                                                                           |
| `String`  | —                      | String                                                                                                                                      |

### 1.5 Identifiers

Identifiers begin with a letter or an underscore. Subsequent characters may be letters, digits, or
underscores. Identifiers are case-sensitive.

Special identifiers:

- `_` is used as a placeholder to indicate that a value should be ignored.
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
Array       ::= '[' Expr (',' Expr)* ']'   // The literal resolves to a fixed-length array when the target type is annotated as Array(T, N)
```

> **A dictionary literal requires at least one key-value pair**: `{}` is **not** an empty
> dictionary—it is an empty block (the value `Void`; see [§2.9](#_2-9-块表达式)). Use the
> `dict.new()` constructor for an empty dictionary:
>
> ```yaoxiang
> empty = dict.new()                  // ✅ Empty dictionary
> d = { "a": 1 }                       // ✅ Dictionary literal
> wrong = {}                           // ❌ Not a dictionary; this is an empty block (Void)
> ```
>
> **The criterion is the content**: the `Dict` grammar requires at least one `String ':' Expr`.
> Since `{}` has no content on which to base a decision, it takes the zero form of the block
> structure. Non-empty forms are self-describing through their content (`{ "k": v }` has a key-value
> pair → dictionary). This has the same origin as `f = { 5 }` being an `Int` value rather than a
> function: **the content determines the type**.

> Set has no literal grammar and no runtime representation—set types are still being planned, and
> when the requirement arises they should be completed following the Dict model (std.set +
> HeapValue::Set). The destination of List/Dict literals is determined by the contextual type
> annotation: bare literals and `List(T)` annotations resolve to growable lists; an `Array(T, N)`
> annotation applied directly to a literal resolves to a fixed-length array. Implicit List→Array
> conversion is forbidden.
>
> Array literal semantics:
>
> - The number of elements must equal N; otherwise, compilation fails with E1002. An empty literal
>   with a nonzero N is likewise rejected
> - Every element type must be compatible with T; otherwise, compilation fails with E1002
> - The grammatical forms of N are an integer literal (possibly negative), a constant name, or a
>   constant expression beginning with a reference to a compile-time value parameter (continued with
>   infix arithmetic, such as `2+3` or `N+1`). The folded value participates in count validation
>   (`2+3` and `5` have the same form)
> - Parser-time rejection: floating-point/string seeds (E0010) and incomplete expressions (such as
>   `2+`, E0012); an identifier followed by `(` follows the constructor-type path—a function call
>   cannot be used as a type-position actual argument (E1003)
> - When N is a symbolic constant (such as a function `const` parameter in `Array(Int, n)`), count
>   validation is deferred to the refinement-type phase
> - Nested array literals are rejected at compile time in v1
>   (`Array(Array(Int,2),2) = [[1,2],[3,4]]`); they must be constructed explicitly, one layer at a
>   time. Recursive destinations are deferred to a later version

#### 1.6.5 List Comprehensions

```
ListComp    ::= '[' Expr ( 'for' Identifier 'in' Expr ( 'if' Expr )? )+ ']'
```

> **#401 (implementation completion)**: `if` filters and multiple generator clauses are implemented
> as of this version. The previous grammar was `(',' Expr)* ('if' Expr)?`, which had no
> implementation corresponding to it (neither comma-appended expressions nor multiple generators
> were supported), and the parser omitted the `if` filter entirely (after parsing the iterable, it
> expected `']'` directly and reported `E0010 Expected RBracket, found KwIf`). The final semantics
> follow common industry practice: one or more generator clauses, expanded as nested loops; each
> clause may carry at most one `if` filter, and the condition is required to be `Bool` (a non-Bool
> condition reports `E1054`); the iterable and condition of later clauses may refer to iteration
> variables bound by earlier clauses.

> **Tightened behavior (migration record)**: the iteration-variable grammar has always been
> `'for' Identifier 'in'`, but the old implementation parsed the pattern using the full Pratt
> parser—after `in` was registered as an infix operator, `x` would consume `in items` as a
> membership expression. After the fix, a non-identifier pattern fails to parse directly and no
> longer falls back to `_` as the old implementation did (silently swallowing the error). Affected
> code: forms such as `[x for (a, b) in pairs]` that previously parsed now report an error. This
> form never had defined behavior (the variable was always `_`), so tightening it is correct and
> incurs no semantic migration cost.

#### 1.6.6 Membership Tests

```
Membership  ::= Expr 'in' Expr
```

> `in` is a binary relational operator that returns `Bool`—`true` on a match and `false` otherwise,
> without reporting an error. The semantic split is: `[]` asserts existence and retrieves a value
> (failure reports an error), whereas `in` asks whether something exists (a non-match is a normal
> `false`). The right operand may be a List / Array / Dict (key set) / Tuple / String (substring) /
> Range (interval). `in` is a first-class Horn predicate and serves as the foundation for
> compile-time provable propositions during the refinement-type phase. (Set is omitted from the list
> of right operands—Set has no runtime representation; see §1.6.4)

### 1.7 Comments

```
// Single-line comment

/* Multi-line comment
   can span multiple lines */
```

### 1.8 Indentation Rules

Code must use four spaces for indentation; tab characters are forbidden. This is a mandatory syntax
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

> **Unary prefix operators** (`!` `-` `+`) bind tightly: they bind less tightly only than calls and
> member access, and more tightly than every binary operator. Therefore, `!a == b` ≡ `(!a) == b`
> (Zig-style semantics). `!` is a purely unary operation, does not participate in short-circuit
> control flow, and is orthogonal to the `and`/`or` keywords (which short-circuit); RFC-010 provides
> the authoritative definition.

> **Range binding strength**: `..` has binding strengths `(6, 7)`—the left side 6 is lower than
> addition (7), while the right side 7 consumes addition but not another same-level `..`. Comparison
> before and after the change:
>
> | Expression   | Before the Change (level 1, right-associative)                                                               | After the Change (`(6,7)`, left-associative)                                          |
> | ------------ | ------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------- |
> | `x in 1..10` | `x in 1..10` (the right operand of `in` is level 4, and level-1 `..` cannot consume it, so parsing fails)    | `x in (1..10)`—the entire interval is the right operand of `in`                       |
> | `0..n+2`     | `(0..n)+2` (right-associativity trap: the upper bound is consumed, causing E3004 directly in the `for` loop) | `0..(n+2)`—the upper bound is an arithmetic expression                                |
> | `a == b..c`  | `a == (b..c)` (level-1 `..` < level-3 `==`, so it is naturally grouped as a whole)                           | `a == (b..c)`—**semantics unchanged**; `..` still binds more tightly than comparisons |
> | `1..2*3`     | `(1..2)*3`                                                                                                   | `1..(2*3)`—the upper bound is an arithmetic expression                                |
> | `a..b..c`    | `a..(b..c)` (a right-associative chain with meaningless nested Ranges)                                       | `(a..b)..c`—**step form** (`c` is the step size)                                      |
>
> Net effect: a compound upper bound such as `for i in 0..n+2` changes from “parses successfully but
> reports E3004” to “works directly”; `x in 1..10` changes from “cannot be parsed” to “interval
> check”; and `a..b..c` changes from “meaningless nesting” to “step components”. Level 6 falls
> between `+` (level 5) and `<<` (level 7), following mathematical convention: an interval is a
> tightly binding construct, so its upper bound is naturally a complete arithmetic expression.

### 2.3 Function Calls

```
FnCall      ::= Expr '(' ArgList? ')'
ArgList     ::= Expr (',' Expr)* (',' NamedArg)* | NamedArg (',' NamedArg)*
NamedArg    ::= Identifier '=' Expr
```

Named arguments use `name = value` (RFC-010 §Function Definitions and RFC-011 §Constructor Forms).
Positional arguments must precede named arguments; arguments whose order is specified may be
arranged in any order among the named arguments:

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b

add(3, 5)          // Positional
add(a = 3, b = 5)  // Named
add(b = 5, a = 3)  // Any order
add(3, b = 5)      // Mixed, with positional arguments first
```

An incorrect named-argument name reports **E1014**, specifying the same parameter both positionally
and by name reports **E1015**, and a mismatched argument count reports **E1010** (RFC-013).

**Unnamed parameters and named arguments**: signature parameter positions permit a bare type form
(`mk: (Int, Int) -> Int`). This is an **unnamed typed parameter**—type constraints apply
positionally, while the name belongs to the implementation (the lambda header; see the shorthand
rules in [RFC-007](../../rfc/accepted/007-function-syntax-unification.md)) and is not part of the
contract. An unnamed parameter cannot be referenced by a named argument because it has no name; it
can only be passed positionally. To place a name in the contract, write
`mk: (a: Int, b: Int) -> Int`. A bare identifier must be a declared type; if no type can be
resolved, parsing reports an error (see [type-system §3.5](type-system.md#35-函数类型)).

**Conversely, named parameters place the name in the contract, and the declaration is
authoritative**. The parameter names in the implementation header (the parameter list after `=>`)
are merely local names within the body and are bound **positionally**—for
`f: (a: Int) -> Int = (b) => b`, the implementation header uses `b`, but callers still write
`f(a = 7)`. Type annotations in the implementation header serve only for comparison; a mismatch with
the declaration reports **E1011**. If the implementation header has more runtime parameter positions
than the declaration, it reports **E1112** (`f: () -> Int = (a: Int) => a`)—in both cases, the
declaration is authoritative. The reverse case, where the implementation omits parameters, does not
report an error: the type layer checks arity against the declaration, but positions omitted from the
implementation header have no runtime slots, and arguments placed in those positions are discarded
on the IR side (see the gap recorded in [type-system §3.5](type-system.md#35-函数类型)).

### 2.4 Member Access

```
MemberAccess::= Expr '.' Identifier
```

### 2.5 Index Access

```
IndexAccess ::= Expr '[' Expr ']'
```

> **Three-layer semantics (RFC-011b)**: `a[i]` is dispatched according to the type of `a`—① built-in
> containers (List/Vec/Array/Dict/Tuple) use the native indexing instruction (the fast path); ② user
> types that implement the `Index` interface are dispatched to their `index` method (instantiating
> `Index(Grid, Int, Float)` within the type body and providing a `Grid.index` method); ③ RFC-004's
> `f[0]` positional binding exists only in binding declarations and is not part of this grammar. For
> a multidimensional index such as `a[0, 1]`, the key is packed into a tuple. Other types that do
> not implement `Index` are rejected at the type layer.

### 2.6 Type Casts

```
TypeCast    ::= Expr 'as' TypeExpr
```

> **Implementation status (an existing gap, not introduced in this version)**: the preceding text
> defines only the grammar and precedence (§2.4, level 2); the semantics have not yet been
> implemented. The following facts can be independently verified: ① Parsing: the `KwAs` infix arm in
> `parser/pratt/led.rs:81` produces `ast::Expr::Cast`; ② Type checking:
> `typecheck/inference/expressions.rs:3847-3853` performs only a **silent re-annotation** of
> `Expr::Cast`—`let _ = self.infer_expr(expr)?;` discards the inner result, after which it directly
> returns the `target_type` `MonoType`; ③ There is **no** convertibility validation anywhere in the
> repository, and there is no corresponding diagnostic code (`util/diagnostic/codes/` has no cast
> entry); ④ `ir_gen` generates no instructions (`middle/core/ir_gen.rs` only obtains the span and
> free variables of `Expr::Cast` at `:5111` / `:5918` / `:9381`—the first two are span arms in
> `get_expr_span` / `expr_span`, and the third is `collect_free_vars_expr`; none of the three
> generates instructions); ⑤ Consequently, `x as Int` currently reports no error and performs no
> runtime conversion.

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
> or `some(x)`, where the variant name is resolved according to the scrutinee) requires the
> scrutinee's type to be a **sum type whose variant set is present**. A variant set enters the
> checker only through a type definition or a `use` import—`Result`/`Option` are defined by
> `std.result`/`std.option`, so `use std.result` / `use std.option` is required before use
> (whole-module and grouped `use std.{...}` forms have equal status); otherwise, variant
> destructuring reports E1002. Exhaustiveness checking uses the same source: a fallback arm exempts
> the match from exhaustiveness requirements; without one, the entire variant set is checked.

### 2.9 Block Expressions

```
Block       ::= '{' Stmt* Expr? '}'
```

> **Statement termination rules**: separation between `Stmt`s and newline behavior (`;` as an
> explicit separator, newline termination, the continuation exception, and line-leading `(`/`[`
> never merging) are defined by [RFC-038](../../rfc/accepted/038-statement-termination.md).

#### 2.9.1 The Three Forms of `{`

In expression position, `{` has exactly three interpretations, determined once by its **content**:

| Form                   | Notation          | Type                 | Example          |
| ---------------------- | ----------------- | -------------------- | ---------------- |
| **Empty block**        | `{}`              | `Void`               | `x: Void = {}`   |
| **Dictionary literal** | `{ "k": v, ... }` | `Dict(K, V)`         | `d = { "a": 1 }` |
| **Block**              | `{ Stmt* Expr? }` | Tail-expression type | `y = { 1 + 1 }`  |

**Decision order**:

1. `{}` (no content) → **empty block**, with value `Void`
2. The first element is a `String ':' Expr` key-value pair → **dictionary literal**
3. Otherwise → **block**, with the value determined by the tail expression

> **Why `{}` is not an empty dictionary**: the “emptiness” of an empty dictionary cannot describe
> itself (it could be either `Dict(K, V)` or an empty block), while the dictionary grammar
> [§1.6.4](#_1-6-4-集合) requires at least one key-value pair. When there is no content on which to
> base a decision, the zero form of the block structure is used. This is consistent with `unsafe {}`
> / `spawn {}` and introduces no special case. Use `dict.new()` for an empty dictionary.
>
> **Why functions require an annotation**: `f = { stmt }` is a **value** (with the tail expression's
> type), not a function. To define a function, state the Fn annotation: `f: () -> Int = { 5 }`. This
> follows the same principle as dictionaries: **the content determines the type**, not the presence
> of an annotation. (See Appendix D of
> [RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md).)

**Unified semantics**: the value of every `{}` block is supplied by its **tail expression**, and
`return` is a nonlocal exit of type `Never`.

| Block Type    | Value Exit      | Empty Block `{}` |
| ------------- | --------------- | ---------------- |
| Ordinary `{}` | Tail expression | `Void`           |
| `unsafe {}`   | Tail expression | `Void`           |
| `spawn {}`    | Tail expression | `Void`           |

**Core principles** (see [RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md) for
details):

- **A block's value = its tail expression** (the final expression); it has one exit and no
  exceptions
- If the final item is an **assignment statement**, the block's value is `Void`; to obtain `Void`
  explicitly, write `Void`
- **`return` exits the nearest function boundary** (passing through every block, rather than
  “returning to the block”), and has type `Never`; `Never <: T`

> holds for any type (principle of explosion), so it may occur wherever a return type is expected.
> The module-initialization layer (the Script top level) has no function boundary, so `return` is
> rejected at compile time there (`E1109`; see §3.11)

- The expression form `= expr` directly supplies a value

```yaoxiang
// Ordinary {} block: the tail expression supplies the value
result = {
    x = compute()
    x                // The block's value
}

// unsafe {} block: the tail expression supplies the type definition
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void
    }
    SqliteDb         // The block's value
}

// spawn {} block: the tail expression supplies the result
(a, b) = spawn {
    result1 = fetch("url1"),
    result2 = fetch("url2")
    (result1, result2)   // The block's value
}

// return: passes through blocks and exits the function
f: (n: Int) -> Int = {
    if n < 0 {
        return 0     // Exits the if and function body, exiting the function
    }
    n * 2            // Tail expression
}
```

#### Is `name = { ... }` a Function or a Block Value?

`name = { ... }` may be either a function definition (RFC-007 “empty-parameter minimal form”) or a
block-value binding. The decision is made by **annotation first, with function as the default**
(RFC-010a Appendix D):

| Case                                  | Result          | Example                            |
| ------------------------------------- | --------------- | ---------------------------------- |
| The value is a Lambda (`=>`)          | Function        | `f = () => 5` → `f()` = 5          |
| The annotation is a function type     | Function        | `f: () -> Int = { 5 }` → `f()` = 5 |
| The annotation is a non-function type | **Block value** | `x: Int = { 5 }` → `x` = 5         |
| No annotation                         | Function        | `f = { 5 }` → `f()` = 5            |

**An annotation is a type**: `x: Int = ...` declares that `x` is an `Int`, so `{ ... }` evaluates to
an `Int`; `f: () -> Int = ...` declares that `f` is a function, so `{ ... }` is the function body.

To evaluate `{ ... }` immediately, **simply specify the target type**; no new syntax is needed:

```yaoxiang
// Block value: evaluated immediately
x: Int = {
    y = 5
    y            // x = 5
}

// Function: defaults to a function without an annotation
f = { 5 }        // f() = 5
```

#### Nested Function Types: Currying or Returning a Function?

The nested function type on the right-hand side of `->` has two possible readings, distinguished by
**parentheses** (RFC-004):

| Notation                        | Meaning                | Invocation                |
| ------------------------------- | ---------------------- | ------------------------- |
| `(a: Int) -> (b: Int) -> Int`   | **Currying**           | `f(1)(2)`                 |
| `(a: Int) -> ((b: Int) -> Int)` | **Returns a function** | `g(1)` returns a function |

The basis is: **an annotation is a type**. `g: (a: Int) -> ((b: Int) -> Int)` declares that
`g(1) : (b: Int) -> Int`, so `g(1)` must **be** that function, rather than “the next set of
parameters”.

```yaoxiang
// Currying: the two parameter groups are supplied in stages
add: (a: Int) -> (b: Int) -> Int = { a + b }
add(1)(2)        // → 3

// Returns a function: one outer parameter is supplied, and the returned value is the function
adder: (n: Int) -> ((x: Int) -> Int) = (x) => x + n
adder(10)(5)     // → 15
h = adder(10)    // The returned function can be stored in a variable and passed around
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

The `?` operator is postfix and has the same precedence as `.`. For the `Result(T, E)` type:

- On `Ok(v)`, extract the value `v` and continue execution
- On `Err(e)`, propagate the error upward (`return Err(e)`)

```yaoxiang
process: (data: Data) -> Result(Data, Error) = {
    validated = validate(data)?     // Extracts the value on success and propagates the error on failure
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

// Range is a value: it can be bound, passed, and tested for membership
r = 1..10
assert.assert(5 in r, "membership")
for i in r { print(i) }

// Step form (third component, default 1)
for i in 0..10..2 { print(i) }  // 0, 2, 4, 6, 8
for i in 10..0..(-2) { print(i) }  // 10, 8, 6, 4, 2
```

> **Step semantics**: in `a..b..c`, `c` is the step size. A literal `c = 0` is rejected at compile
> time; a dynamic `c` receives a runtime zero check (the E6001 family; it will be promoted to Result
> once the error system is implemented). `c < 0` is valid, and the interval direction reverses with
> its sign (`10..0..(-2)` counts downward).

### 2.13 ref Expressions

```
RefExpr     ::= 'ref' Expr
```

`ref` creates shared ownership. The compiler automatically chooses Rc (single-task) or Arc
(cross-task); users do not need to concern themselves with implementation details.

```yaoxiang
data = ref heavy_data
spawn { use(data) }   // Cross-task: the compiler automatically chooses Arc
```

### 2.14 unsafe Expressions

```
UnsafeExpr  ::= 'unsafe' Block
```

`unsafe` blocks are used to define opaque types and operate on raw pointers. Use `return` to return
a type definition to the enclosing scope.

**Semantics**:

- Types and raw-pointer operations may be defined inside `unsafe {}`
- A returned type is available outside `unsafe {}`
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

### 2.15 Scopes

**Basic rules**:

- Every `{}` block creates a scope
- An inner scope can access variables from an outer scope
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

- `x = value`: searches outward along the scope chain for `x`; if found, it assigns to it, and if
  not found, it declares a new binding
- `mut x = value`: explicitly declares a new mutable binding and prohibits the same name in an outer
  scope
- Within the same scope, any name may be declared only once

> **Detailed definition**: see [Module System Specification](modules.md#第四章作用域) for the
> complete scope rules and the mechanisms for variable declaration and shadowing.

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

**Semantics**: `return` is a **nonlocal exit** that exits the nearest **function boundary**, passing
through every block—including `if` / `while` / `for` / `match` / bare blocks / `spawn` /
`unsafe`—and hands the value to the caller. It does **not “return to the block.”**

**Type**: `return e : Never` (where `e : T`). `Never <: T'` holds for any `T'` (principle of
explosion; see [Type System §2.2](type-system.md)), so `return` may occur wherever a return type is
expected, without requiring any additional rule.

**Relationship to block evaluation**: a block's value always comes from its **tail expression** (see
§2.9). As a block, `{ return n }` has the value `n` and the type `Never`; at the same time, `return`
exits the function. **Both facts hold simultaneously**, coexisting through the principle of
explosion.

`return` and tail expressions together make “early return” possible, without requiring a special
rule for `return` to target functions—see
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

**Semantics**: immediately terminates the innermost enclosing `while`/`for` loop, transferring
control to after that loop's body.

- **Exits only one level**: `break` always applies to the innermost enclosing loop. When a nested
  loop must be exited by more than one level, extract the inner loop into a function and use
  `return` to exit it, or use a flag (break/continue have no labels; if loop labels are introduced
  in the future, their declaration-side syntax will follow the RFC process and be adjudicated
  jointly with the proof pipeline's multi-exit design)
- **Permitted only inside a loop body**: `break` may appear only in a `while`/`for` loop body
  (including nested blocks/if/match statements within it); using it outside a loop reports a
  compile-time error (E1102 `'break' outside of a loop`)
- **Does not affect termination proofs**: `break` does not participate in a termination argument—it
  provides neither a measure nor a decrement step for one. The loop's termination obligation is
  independent of `break` and is triggered by refinement types (see
  [type-system §8.5](type-system.md#85-terminates终止测度谓词)).
- **Borrowing semantics**: the control-flow edge of break participates in the structural cutoff of
  RFC-009a's reverse-BFS liveness analysis (the escaping iteration does not participate in back-edge
  liveness inference)

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
    j = j + 1                  // This is executed after every outer iteration
}
```

### 3.5 continue Statements

```
ContinueStmt::= 'continue'
```

**Semantics**: skips the remaining statements in the current iteration and proceeds directly to the
next iteration of the innermost enclosing loop—for `while`, the condition is evaluated again; for
`for`, the next element is obtained.

- **Applies only to the nearest level**: the same as `break`; there are no labels
- **Permitted only inside a loop body**: using it outside a loop reports a compile-time error
  (E1102)

```yaoxiang
mut sum = 0
mut n = 0
while n < 5 {
    n = n + 1
    if n == 3 {
        continue           // Skips the addition below; n == 3 is not included
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

YaoXiang's `for` loop semantics differ from those of traditional languages: **each iteration binds a
new value rather than modifying the same variable**.

```yaoxiang
// Example: for i in 1..5
for i in 1..5 {
    print(i)
}
```

**Execution process**:

| Iteration | Loop Variable Behavior                                                                             |
| --------- | -------------------------------------------------------------------------------------------------- |
| 1st       | Creates a new binding `i = 1`, executes the body, and prints 1                                     |
| 2nd       | Creates a new binding `i = 2` (the previous binding is destroyed), executes the body, and prints 2 |
| 3rd       | Creates a new binding `i = 3`, executes the body, and prints 3                                     |
| 4th       | Creates a new binding `i = 4`, executes the body, and prints 4                                     |
| End       | The loop body ends, and the binding is destroyed                                                   |

**Key point**: after each iteration ends, the binding created during that iteration is destroyed.
The next iteration uses a completely new binding, unrelated to the binding from the previous
iteration.

#### 3.9.2 Difference Between `for` and `for mut`

| Syntax              | Loop Variable Mutability | Description                                     |
| ------------------- | ------------------------ | ----------------------------------------------- |
| `for i in 1..5`     | Immutable                | The binding cannot be modified in the loop body |
| `for mut i in 1..5` | Mutable                  | The binding may be modified in the loop body    |

```yaoxiang
// Valid: each iteration binds a new value, so no modification is needed
for i in 1..5 {
    print(i)  // Reads the value of i
}

// Error: an immutable binding cannot be modified
for i in 1..5 {
    i = i + 1  // Error: an immutable binding cannot be modified
}

// Valid: for mut permits modifying the binding
for mut i in 1..5 {
    i = i + 1  // Modification is permitted
}
```

#### 3.9.3 Shadowing Checks

YaoXiang prohibits variable shadowing. A `for` loop variable cannot have the same name as a variable
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

This rule applies to every code block. See [4.3 Shadowing Rules](modules.md#43-遮蔽规则) for
details.

#### 3.9.4 Comparison with Other Languages

| Language | `for` Loop Variable Semantics                                |
| -------- | ------------------------------------------------------------ |
| YaoXiang | Binds a new value on every iteration                         |
| Rust     | Modifies the same variable (requires `mut`)                  |
| Python   | Modifies the same variable (does not require `mut`)          |
| C/C++    | Modifies the same variable (requires a pointer or reference) |

**Design rationale**: YaoXiang adopts binding semantics because:

1. **It aligns more closely with natural semantics.** In natural language, “for each element x in a
   set” means that each x is an independent entity. YaoXiang's `for i in 1..5` reads as “for each i
   from 1 through 5,” and i is a completely new binding on every iteration, which is consistent with
   human intuition.

2. **It prevents accidental modification.** Immutable bindings by default mean that the loop
   variable cannot be accidentally modified in the loop body. There is no need to worry that some
   part of a complex loop body inadvertently contains `i = ...`, making the bug difficult to track
   down.

3. **A high-performance solution remains readily available.** When a variable genuinely must be
   reused across iterations—for example, as an accumulator or cache—use a `for mut` declaration to
   switch to mutable-binding mode. This is clearer than implicit shared state: the intent is
   expressed explicitly in the syntax rather than hidden in runtime behavior.

### 3.10 spawn Statements

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' 'mut'? Identifier 'in' Expr '{' Expr '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
```

**spawn block**: explicitly declares a concurrency boundary, and the expressions in the block
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

- A reference from the block body to an outer variable is a **Move value capture**: the value is
  snapshotted into the closure environment at the spawn creation point, and the block body reads it
  through the environment (LoadUpvalue)
- For **primitives** (Int/Float/Bool/Char), the value is copied, and the outer variable is
  unaffected
- For **handle types** (Struct/String/List, etc.), snapshotting means copying the handle while
  sharing the underlying object; in the Embedded runtime (the default), the same thread and heap are
  used, so the handle remains valid
- Sharing between multiple tasks requires an explicit `ref` (§2.13; the compiler automatically
  chooses Rc/Arc)
- A `return` within the block likewise captures any referenced outer variables

```yaoxiang
t1 = 1 + 1
t2 = 2 + 2
result = spawn {
    return t1 + t2    // t1/t2 are value-captured; result == 6
}
```

---

### 3.11 Program Entry Point and Top-Level Statements

A source file has two roles from the compiler's perspective, **determined by the presence of
`yaoxiang.toml`**:

| Role       | Determination                                       | Program Body                                                                       |
| ---------- | --------------------------------------------------- | ---------------------------------------------------------------------------------- |
| **Script** | A single file run directly, without `yaoxiang.toml` | **Top-level statements** (executed in source order); `main` is an ordinary binding |
| **Bin**    | `yaoxiang.toml` is present                          | **The `main` function**; top-level executable statements are forbidden             |

#### Script: Top-Level Statements Are the Program

Without a manifest, the file is a “script,” and **top-level statements execute in source order**
(top-level **binding initialization** runs as a whole beforehand; see
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

> **Why `main` is not invoked automatically**: top-level statements are already the program body. If
> `main` were also invoked implicitly, a script that explicitly calls `main()` would execute it
> twice. The two rules cannot coexist, so Script mode has only one execution entry point: top-level
> statements.

The top level likewise **must not use `return`**: the Script top level is the module-initialization
layer and has no function boundary to exit (RFC-010a rule ②). The compiler reports `E1109`, rather
than silently terminating initialization at runtime and skipping the remaining top-level statements.
To end a section of top-level logic early, place it in a function and control it at the call site.

#### Bin: `main` Is the Entry Point

When a manifest is present, the file is an “executable target,” and:

- It must define a **binding** named `main`, whose value may be either a value or a function (#388's
  final decision: entry is determined by the existence of the binding—a function is already a value,
  and the two differ only in evaluation strategy): a function `main` is called with zero arguments
  during entry; a value `main` is evaluated during initialization and thus executes
- Top-level executable statements are forbidden—the program body is `main`

```yaoxiang
main: () -> Void = {
    print("hello")
}
```

A value `main` is equally valid—it is evaluated during initialization, within the initialization
sequence (according to the global binding topological order, with independent bindings following
source order):

```yaoxiang
main = {
    print("hello")
}
```

A missing `main` is a compile-time error (without `main`, all functions are unreachable). An
uncallable value `main` (such as `main: Int = 5`) is valid but has no observable effect—the
equivalent of Rust's empty `fn main() {}`.

> **Library files**: files imported by other files with `use`, or files referenced by `[lib].path` /
> `[exports]`, do not require `main`—they are not program entry points.

#### Top-Level Binding Initialization

The initialization values of top-level bindings are **evaluated at runtime** and are not required to
be compile-time constants:

```yaoxiang
answer: Int = { 42 }              // Block value
inc: (Int) -> Int = (x) => x + 1
computed: Int = inc(41)           // Function call
```

Initialization executes in **dependency order**, independently of source order (dependencies are
initialized first; bindings without dependencies follow declaration order):

```yaoxiang
derived: Int = base * 3           // References base, which is declared later
base: Int = 7                     // Initialized first (topological sort)
```

Moreover, **all binding initialization occurs before any top-level statement executes**—side effects
in binding initializers happen before any top-level statement, regardless of how the two are
interleaved in the source:

```yaoxiang
print("a")                        // Executes afterward
x = { print("b") }                // Executes first (binding initialization runs as a whole first)
print("c")                        // Output order: b, a, c
```

Circular dependencies are compile-time errors (the names on the cycle are listed):

```yaoxiang
a: Int = b + 1
b: Int = a + 1                    // Error: a → b → a
```

> **Design basis**: RFC-029f (file role model) and RFC-010a Appendix D (block binding decisions).

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
wholesale under RFC-029g and is now an ordinary identifier. For literal reserved words `true` /
`false` / `void`, see §1.4.1. For the metatype name `Type` and the built-in type names `Void` /
`Never` / `Int` / `Float` / `Bool` / `Char` / `String`, see §1.4.3.

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
