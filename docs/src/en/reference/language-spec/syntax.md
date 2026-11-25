# Syntax Specification

This document defines the syntax specification of the YaoXiang programming language, including its
lexical structure, grammar rules, and operator precedence.

---

## Chapter 1: Lexical Structure

### 1.1 Source Files

YaoXiang source files must use UTF-8 encoding. Source files usually use the `.yx` extension.

### 1.2 Lexical Token Categories

| Category   | Description                               | Examples                  |
| ---------- | ----------------------------------------- | ------------------------- |
| Identifier | Starts with a letter or underscore        | `x`, `_private`, `my_var` |
| Keyword    | Reserved words predefined by the language | `use`, `mut`, `and`       |
| Literal    | Fixed values                              | `42`, `"hello"`, `true`   |
| Operator   | Operation symbols                         | `+`, `-`, `*`, `/`        |
| Delimiter  | Syntactic delimiters                      | `(`, `)`, `{`, `}`, `,`   |

### 1.3 Keywords

YaoXiang has **17 keywords** (`keyword_from_str` in `src/frontend/core/lexer/state.rs`, with each
one corresponding to a `TokenKind`):

```
use     spawn   ref     mut
if      else    match   while   for
in      return  break   continue
as      unsafe  and     or
```

`and` / `or` are the **keywords** for logical AND / OR (Zig-style; see level 10 in §2.2 for
precedence). Unary negation is the symbol `!`, which is orthogonal to them.

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

YaoXiang’s “reserved words” fall into three layers, recognized by the parser and type checker at
different stages:

#### 1.4.1 Literal Reserved Words

The parser has separate tokens for the following literal identifiers, which cannot be used as
ordinary identifiers:

| Identifier | Type | Description                                                                                                   |
| ---------- | ---- | ------------------------------------------------------------------------------------------------------------- |
| `true`     | Bool | Boolean true value                                                                                            |
| `false`    | Bool | Boolean false value                                                                                           |
| `void`     | Void | Void literal (Unit value). Lowercase `void` is a value literal; uppercase `Void` is a type name (see §1.4.3). |

> **`Type` is not in this layer**: it does **not** have a separate `TokenKind` (it is absent from
> the `TokenKind` enum in `src/frontend/core/lexer/tokens.rs`, and `keyword_from_str` has no
> corresponding branch). The parser treats it as an ordinary identifier, while the type checker
> recognizes it as a meta type in type position. Therefore, in expression position, `Type` can be
> shadowed by a local binding. It is a **meta type name**, not a keyword.

#### 1.4.2 Variant Names

Variant names are recognized by the parser in **pattern** contexts (bare names are resolved against
the variant set of the scrutinee type; see §2.8). In **expression** contexts, constructing a variant
requires type qualification—`Result(Int, String).ok(5)`, `Option(T).some(v)`, `Color.green()`
(RFC-010 record type and type syntax: variant constructors are function-typed fields in the type
definition, and a bare name such as `ok(5)` is not a constructor call).

| Variant name | Owning type | Description                             |
| ------------ | ----------- | --------------------------------------- |
| `some(T)`    | Option      | Constructor for an Option value variant |
| `ok(T)`      | Result      | Result success variant                  |
| `err(E)`     | Result      | Result error variant                    |

#### 1.4.3 Built-in Type Names

The following type names are pre-registered by the type checker and can be used in type positions
without importing them. The parser treats them as ordinary identifiers—**they are not reserved words
and can be shadowed by local bindings (not recommended)**.

| Type name | Logical correspondence | Description                                                                                                                               |
| --------- | ---------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| `Void`    | ⊤ (true/Unit)          | Zero-field product type with exactly one inhabitant (the `void` literal; see §1.4.1)                                                      |
| `Never`   | ⊥ (false/empty type)   | Zero variants and zero inhabitants. No expression can produce a `Never` value. `Never <: T` holds for every `T` (principle of explosion). |
| `Int`     | —                      | Signed integer type                                                                                                                       |
| `Float`   | —                      | Float type                                                                                                                                |
| `Bool`    | —                      | Boolean type: `true` / `false`                                                                                                            |
| `Char`    | —                      | Unicode character type                                                                                                                    |
| `String`  | —                      | String type                                                                                                                               |

### 1.5 Identifiers

An identifier starts with a letter or underscore, and subsequent characters may be letters, digits,
or underscores. Identifiers are case-sensitive.

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
Array       ::= '[' Expr (',' Expr)* ']'   // When the target type annotation is Array(T, N), the literal produces a fixed-length array
```

> **Dictionary literals require at least one key-value pair**: `{}` is **not** an empty
> dictionary—it is an empty block (value `Void`; see [§2.9](#_2-9-块表达式)). To create an empty
> dictionary, use the constructor `dict.new()`:
>
> ```yaoxiang
> empty = dict.new()                  // ✅ Empty dictionary
> d = { "a": 1 }                       // ✅ Dictionary literal
> wrong = {}                           // ❌ This is not a dictionary; it is an empty block (Void)
> ```
>
> **The criterion is the content**: the `Dict` grammar requires at least one `String ':' Expr`.
> Since `{}` has no content on which to base a decision, it takes the zero form of the block
> structure. The non-empty form is self-describing through its content (`{ "k": v }` has a key-value
> pair → dictionary). This follows the same principle as `f = { 5 }` being an `Int` value rather
> than a function: **the type is determined by the content**.

> Set has no literal grammar or runtime representation—when requirements emerge during set type
> planning, it will be completed following the Dict model (`std.set` + `HeapValue::Set`). The
> landing point of List/Dict literals is determined by the contextual type annotation: bare literals
> and `List(T)` annotations both land on growable lists. The type surface of a growable list has the
> single name `Vec(T)`; a `List(T)` annotation is normalized through its source name and lands on
> the same type surface as `Vec(T)`. An `Array(T, N)` annotation applied directly to a literal lands
> on a fixed-length array. Implicit List→Array conversions are forbidden.
>
> Array literal semantics:
>
> - The number of elements must equal N; otherwise, compile-time E1002 is reported. An empty literal
>   with a nonzero N is rejected in the same way.
> - Every element type must be compatible with T; otherwise, compile-time E1002 is reported.
> - The grammatical form of N is: an integer literal (which may be negative), a constant name, or a
>   constant expression beginning with a reference to a compile-time value parameter (continued with
>   infix arithmetic, such as `2+3` or `N+1`). Its folded value participates in count validation
>   (`2+3` has the same form as `5`).
> - Parse-time rejection: float/string seeds (E0010) and incomplete expressions (such as `2+`,
>   E0012). An identifier followed by `(` takes the constructor type path—a function call cannot be
>   used as a type-position actual argument (E1003).
> - When N is a symbolic constant (such as a function const parameter in `Array(Int, n)`), count
>   validation is deferred to the refinement type stage.
> - v1 rejects nested array literals (`Array(Array(Int,2),2) = [[1,2],[3,4]]`) at compile time; each
>   level must be constructed explicitly. Recursive landing behavior is deferred to a later version.

#### 1.6.5 List Comprehensions

```
ListComp    ::= '[' Expr ( 'for' Identifier 'in' Expr ( 'if' Expr )? )+ ']'
```

> **#401 (implementation completion)**: `if` filters and multiple generator clauses are implemented
> as of this version. The previous grammar was written as `(',' Expr)* ('if' Expr)?`, which had no
> corresponding implementation (neither comma-appended expressions nor multiple generators were
> supported), and the parser omitted `if` filters entirely (after parsing the iterable, it expected
> `']'` and reported `E0010 Expected RBracket, found KwIf`). The semantics now follow common
> industry practice: there must be one or more generator clauses, expanded as nested loops. Each
> clause may carry at most one `if` filter, and the condition must produce a `Bool` (a non-Bool
> value reports `E1054`). The iterable or condition of a later clause may reference iteration
> variables bound by earlier clauses.

> **Tightened behavior (migration record)**: the iteration variable grammar was already
> `'for' Identifier 'in'`, but the old implementation routed patterns through the full Pratt
> parser—after `'in'` was registered as an infix operator, `x` would swallow `in items` as a
> membership expression. After the fix, a non-identifier pattern fails to parse directly and no
> longer falls back to `_` (silently swallowing the error). Impact: forms such as
> `[x for (a, b) in pairs]`, which previously parsed, now report an error. This form never had
> defined behavior (the variable was always `_`), so tightening it in this direction is correct and
> incurs no semantic migration cost.

#### 1.6.6 Membership Testing

```
Membership  ::= Expr 'in' Expr
```

> `in` is a binary relational operator that returns `Bool`—`true` on a match and `false` otherwise,
> without reporting an error. The semantic split is: `[]` asserts existence and retrieves a value
> (failing with an error), whereas `in` asks whether something exists (a miss is a normal `false`).
> Right-operand coverage: List / Array / Dict (key set) / Tuple / String (substring) / Range
> (interval). `in` is a first-class Horn predicate and serves as the foundation for compile-time
> provable propositions during the refinement type stage. (Set is excluded from the right-operand
> list because Set has no runtime representation; see §1.6.4.)

### 1.7 Comments

```
// Single-line comment

/* Multi-line comment
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

> **Unary prefix operators** (`!` `-` `+`) bind tightly: they bind only less tightly than calls and
> member access, and more tightly than every binary operator. Therefore, `!a == b` is equivalent to
> `(!a) == b` (Zig-style semantics). `!` is a pure unary operation and does not participate in
> short-circuit control flow; it is orthogonal to the `and`/`or` keywords, which do short-circuit
> (as defined authoritatively by RFC-010).

> **Range binding power**: `..` has binding power (6, 7)—left precedence 6 is lower than addition
> (7), while right precedence 7 consumes addition but not another `..` at the same level. Comparison
> before and after the change:
>
> | Expression   | Before the change (level 1, right-associative)                                                                | After the change ((6,7), left-associative)                                            |
> | ------------ | ------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
> | `x in 1..10` | `x in 1..10` (`in` right operand is level 4, and level-1 `..` cannot consume it, so parsing fails)            | `x in (1..10)`—the entire interval is the right operand of `in`                       |
> | `0..n+2`     | `(0..n)+2` (right-associativity trap: the upper bound is consumed, and the `for` loop reports E3004 directly) | `0..(n+2)`—the upper bound is an arithmetic expression                                |
> | `a == b..c`  | `a == (b..c)` (`..` level 1 < `==` level 3, so it naturally forms one unit)                                   | `a == (b..c)`—**semantics unchanged**; `..` still binds more tightly than comparisons |
> | `1..2*3`     | `(1..2)*3`                                                                                                    | `1..(2*3)`—the upper bound is an arithmetic expression                                |
> | `a..b..c`    | `a..(b..c)` (right-associative chain with meaningless nested Ranges)                                          | `(a..b)..c`—**step form** (`c` is the step size)                                      |
>
> Net effect: the compound upper bound `for i in 0..n+2` changes from “parses successfully but
> reports E3004” to “directly usable”; `x in 1..10` changes from “cannot be parsed” to “interval
> check”; and `a..b..c` changes from “meaningless nesting” to “step component”. Level 6 falls
> between `+` (level 5) and `<<` (level 7), following mathematical convention: a range is a tightly
> bound construct, so its upper bound naturally forms a complete arithmetic expression.

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

An incorrect named-argument name reports **E1014**. Specifying the same formal parameter both
positionally and by name reports **E1015**. An incorrect argument count reports **E1010** (RFC-013).

**Unnamed parameters and named arguments**: signature parameter positions permit the bare type form
(`mk: (Int, Int) -> Int`). This is an **unnamed typed parameter**—type constraints apply
positionally, while the name belongs to the implementation (the lambda header; see the shorthand
rules in [RFC-007](../../rfc/accepted/007-function-syntax-unification.md)) and is not part of the
contract. An unnamed parameter cannot be referenced by a named argument because it has no name; it
can only be passed positionally. To place a name in the contract, use `mk: (a: Int, b: Int) -> Int`.
A bare identifier must be a declared type; failure to resolve the type produces a parse error (see
[type-system §3.5](type-system.md#35-函数类型)).

**Named parameters in the other direction: the name is part of the contract, and the declaration is
authoritative**. The parameter names in the implementation header (the parameter list after `=>`)
are merely local names in the body and are bound **positionally**—for
`f: (a: Int) -> Int = (b) => b`, the implementation header uses `b`, but the caller still writes
`f(a = 7)`. Type annotations in the implementation header are used only for comparison and report
**E1011** when they differ from the declaration. If the implementation header has more parameters
than the declaration has runtime parameter slots, it reports **E1112**
(`f: () -> Int = (a: Int) => a`)—in both cases, the declaration is the authoritative source. The
reverse case, where the implementation specifies fewer parameters, does not report an error: the
type layer checks arity according to the declaration, but positions without a runtime slot discard
the corresponding argument on the IR side (see the gap recorded in
[type-system §3.5](type-system.md#35-函数类型)).

### 2.4 Member Access

```
MemberAccess::= Expr '.' Identifier
```

### 2.5 Index Access

```
IndexAccess ::= Expr '[' Expr ']'
```

> **Three-layer semantics (RFC-011b)**: `a[i]` is dispatched according to the type of `a`: ①
> built-in containers (List/Vec/Array/Dict/Tuple) use native indexing instructions (the fast path);
> ② user types that implement the `Index` interface are dispatched to their `index` method
> (instantiating `Index(Grid, Int, Float)` in the type body and providing a `Grid.index` method); ③
> RFC-004’s `f[0]` positional binding exists only in a binding declaration and does not use this
> grammar. The keys of a multidimensional index such as `a[0, 1]` are packed into a tuple. Other
> types that do not implement `Index` are rejected at the type layer.

### 2.6 Type Casts

```
TypeCast    ::= Expr 'as' TypeExpr
```

> **Current implementation status (pre-existing gap, not introduced in this version)**: the
> preceding text defines only the grammar and precedence (§2.4, level 2); the semantics have not yet
> been implemented. Independently verifiable facts: ① Parsing: the `KwAs` infix arm in
> `parser/pratt/led.rs:81` produces `ast::Expr::Cast`; ② Type checking:
> `typecheck/inference/expressions.rs:3847-3853` performs only a **silent relabeling** for
> `Expr::Cast`—`let _ = self.infer_expr(expr)?;` discards the inner result and then directly returns
> the `MonoType` of `target_type`; ③ There is **no** convertibility validation anywhere in the
> repository, and there is no corresponding diagnostic code (the `util/diagnostic/codes/` directory
> has no cast entry); ④ `ir_gen` does not generate any instructions (`middle/core/ir_gen.rs` only
> reads the span and free variables of `Expr::Cast` at `:5111` / `:5918` / `:9381`—the first two are
> span arms of `get_expr_span` / `expr_span`, and the third is `collect_free_vars_expr`; none of the
> three generates instructions); ⑤ Consequently, `x as Int` currently neither reports an error nor
> produces any runtime conversion.

### 2.7 Conditional Expressions

```
IfExpr      ::= 'if' Expr Block ('else' 'if' Expr Block)* ('else' Block)?
```

### 2.8 Pattern Matching

```
MatchExpr    ::= 'match' Expr '{' MatchArm (ArmSeparator MatchArm)* '}'
MatchArm     ::= Pattern ('|' Pattern)* ('if' Expr)? '->' Expr
ArmSeparator ::= ',' | <换行>
Pattern     ::= Literal
              | Identifier
              | Wildcard
              | RangePattern
              | StructPattern
              | TuplePattern
              | EnumPattern
              | OrPattern
RangePattern ::= ('-')? Literal '..' ('-')? Literal   // Endpoints may have a negative sign prefix; negative values are normalized—see the “Range Patterns” note below
```

> **Arm arrows and separators**: arms use `->`; `=>` is used only for lambdas. Arms are separated by
> a line break or `,`—the same rule as “line break or `;`” for statements
> ([RFC-038](../../rfc/accepted/038-statement-termination.md)).

> **Variant-set requirement for variant destructuring** (RFC-010b): `EnumPattern` (such as `ok(v)`
> or `some(x)`, whose variant names are resolved from the scrutinee) requires the scrutinee’s type
> to be a **sum type whose variant set is present**. A variant set enters the checker only through a
> type definition or a `use` import—`Result` / `Option` are defined by `std.result` / `std.option`,
> so `use std.result` / `use std.option` must be used beforehand (whole-module and grouped
> `use std.{...}` forms have equal standing); otherwise, variant destructuring reports E1002.
> Exhaustiveness checking follows the same source: a fallback arm is exempt from exhaustiveness, and
> without one, the checker searches the entire variant set.

> **Pattern bindings, or-patterns, and guards** (RFC-010b):
>
> - **Binding**: a bare identifier in a pattern introduces a **new name**—it is neither a comparison
>   nor an assignment. The name is visible **within the arm**, shadows an outer variable of the same
>   name, and expires when control leaves the arm. The discriminant (such as `ok` in `ok(v)`) is
>   resolved by looking it up in the scrutinee’s variant set; a bare name that is not found is a
>   binding.
> - **Or-pattern**: `p1 | p2` tries the alternatives in order. If a non-final alternative fails, the
>   next alternative is tried; if the final alternative fails, control falls through to the next
>   arm. All alternatives must bind **the same set of names**, or E1033 is reported. A binding with
>   the same name exists only once at the arm level, and the arm body necessarily reads the value
>   written by the matching alternative.
> - **Guard**: after a pattern matches, the `if` condition is evaluated, and the pattern bindings
>   are visible to the condition. The condition must produce a `Bool`; if it is false, control falls
>   through to the next arm. An arm with a guard **does not count toward exhaustiveness coverage**
>   (because the guard might not match), but a duplicate variant arm with a guard is reachable (it
>   refines the preceding arm).
> - **Diagnostics**: a missing branch reports E1030, an unreachable branch reports E1031, a
>   duplicate binding within the same pattern reports E1032, inconsistent or-pattern bindings report
>   E1033, and a missing field in a struct pattern reports E1034; see the
>   [Error Code Reference](../error-code/E1xxx.md) for the code table and messages.
>
> **Range patterns** (issue #405): `lo..hi` is a **closed-interval test**—on the Int domain,
> `x matches lo..hi ⟺ lo <= x <= hi`, so both 80 and 89 match `80..89`. Endpoints must be **integer
> literals** (a negative sign prefix is normalized to a negative value, such as `-5..-1`). Other
> literal types (Float/Bool/String/Char) are syntactically valid but rejected by the type checker
> (E1002; the first version does not support Float ranges). An identifier endpoint in a pattern
> position is a binding name with no matching semantics and is rejected at the syntax layer (E0010).
> When both endpoints are decidable at compile time, a reversed range (`lo > hi`) is empty, never
> matches, and reports unreachable (E1031); a repeated range arm with the same endpoints likewise
> reports E1031.
>
> Exhaustiveness interaction: range patterns are in the same category as literal patterns—**checked,
> partially covering, and non-fallback**. Because the Int domain is open, a range arm cannot be
> exhaustive. A match containing a range arm still requires a wildcard/identifier arm as a fallback
> (E1030). A range arm may appear as an alternative in an or-pattern and may have a guard (a failed
> guard falls through to the next arm, consistently with the existing guard rules).
>
> **Semantic separation from §2.12 Range expressions**: `lo..hi` in a pattern position does not
> construct a Range value; it only performs branch matching, with closed-interval endpoint
> semantics. The expression-position form `a..b` (`for` iteration, `in` interval checks, and
> first-class Range values) remains half-open. The two `..` semantics are intentionally separate:
> pattern position uses a matching predicate over a discrete value domain, while expression position
> uses notation for constructing an iteration sequence.

### 2.9 Block Expressions

```
Block       ::= '{' Stmt* Expr? '}'
```

> **Statement termination rules**: separation and line-break behavior between Stmts (`;` as an
> explicit separator, a line break as a terminator, continuation exceptions, and line-leading
> `(`/`[` never being merged) are defined by
> [RFC-038](../../rfc/accepted/038-statement-termination.md). **Adjacent statements without a
> separator are a syntax error**. The same rule (a line break or an explicit separator) applies to
> match arms (see §2.8).

#### 2.9.1 The Three Forms of `{`

In expression position, `{` has exactly three possible interpretations, determined once by its
**content**:

| Form                   | Notation          | Type                 | Example          |
| ---------------------- | ----------------- | -------------------- | ---------------- |
| **Empty block**        | `{}`              | `Void`               | `x: Void = {}`   |
| **Dictionary literal** | `{ "k": v, ... }` | `Dict(K, V)`         | `d = { "a": 1 }` |
| **Block**              | `{ Stmt* Expr? }` | Tail expression type | `y = { 1 + 1 }`  |

**Decision order**:

1. `{}` (no content) → **empty block**, value `Void`
2. The first element is a `String ':' Expr` key-value pair → **dictionary literal**
3. Otherwise → **block**, with its value determined by the tail expression

> **Why `{}` is not an empty dictionary**: the “empty” form of a dictionary cannot describe itself
> (it could be either `Dict(K, V)` or an empty block), while the dictionary grammar
> [§1.6.4](#_1-6-4-集合) requires at least one key-value pair. When there is no content on which to
> base a decision, it takes the zero form of the block structure: this is consistent with
> `unsafe {}` / `spawn {}` and introduces no special case. Use `dict.new()` for an empty dictionary.
>
> **Why functions require an annotation**: `f = { stmt }` is a **value** (with the tail expression’s
> type), not a function. To define a function, state the Fn annotation: `f: () -> Int = { 5 }`. This
> follows the same principle as dictionaries: **the type is determined by the content**, not by
> whether an annotation exists. (See
> [RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md), Appendix D.)

**Unified semantics**: the value of every `{}` block is provided by the **tail expression**, and
`return` is a `Never`-typed nonlocal exit.

| Block type    | Value source    | Empty block `{}` |
| ------------- | --------------- | ---------------- |
| Ordinary `{}` | Tail expression | `Void`           |
| `unsafe {}`   | Tail expression | `Void`           |
| `spawn {}`    | Tail expression | `Void`           |

**Core principles** (see [RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md) for
details):

- **A block’s value = its tail expression** (the final expression), with no exceptions.
- If the final element is an **assignment statement**, the block’s value is `Void`; to obtain `Void`
  explicitly, write `Void`.
- **`return` exits the nearest function boundary** (passing through every block, rather than
  “returning to the block”), and has type `Never`; `Never <: T`

> holds for any type (principle of explosion), so it may appear in any return-type position. The
> module initialization layer (Script top level) has no function boundary, and `return` is rejected
> at compile time there (`E1109`; see §3.11).

- The expression form `= expr` directly provides a value.

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
        return 0     // Exits both the if and the function body, returning from the function
    }
    n * 2            // Tail expression
}
```

#### Is `name = { ... }` a Function or a Block Value?

`name = { ... }` may be either a function definition (RFC-007’s “empty-parameter minimum”) or a
block-value binding. The decision rule is **annotation first, function by default** (RFC-010a,
Appendix D):

| Case                                  | Result          | Example                            |
| ------------------------------------- | --------------- | ---------------------------------- |
| The value is a Lambda (`=>`)          | Function        | `f = () => 5` → `f()` = 5          |
| The annotation is a function type     | Function        | `f: () -> Int = { 5 }` → `f()` = 5 |
| The annotation is a non-function type | **Block value** | `x: Int = { 5 }` → `x` = 5         |
| No annotation                         | Function        | `f = { 5 }` → `f()` = 5            |

**The annotation is the type**: `x: Int = ...` declares that `x` is an `Int`, so `{ ... }` is
evaluated as an `Int`; `f: () -> Int = ...` declares that `f` is a function, so `{ ... }` is a
function body.

To evaluate `{ ... }` immediately, **write the target type** (no new syntax is needed):

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

There are two ways to read a nested function type on the right-hand side of `->`, distinguished by
**parentheses** (RFC-004):

| Form                            | Meaning                | Call                      |
| ------------------------------- | ---------------------- | ------------------------- |
| `(a: Int) -> (b: Int) -> Int`   | **Curried**            | `f(1)(2)`                 |
| `(a: Int) -> ((b: Int) -> Int)` | **Returns a function** | `g(1)` returns a function |

The basis is: **the annotation is the type**. `g: (a: Int) -> ((b: Int) -> Int)` declares that
`g(1) : (b: Int) -> Int`, so `g(1)` must **be** that function, rather than the next parameter group.

```yaoxiang
// Curried: the two parameter groups are supplied one layer at a time
add: (a: Int) -> (b: Int) -> Int = { a + b }
add(1)(2)        // → 3

// Returns a function: the outer parameter group is supplied first, and the result is the function
adder: (n: Int) -> ((x: Int) -> Int) = (x) => x + n
adder(10)(5)     // → 15
h = adder(10)    // The returned function can be stored in a variable and passed around
```

Every unparenthesized nested `Fn` is curried (including RFC-011’s type-parameter form):

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
            |  Identifier '=>' Expr

裸单参形态（`Identifier '=>' Expr`）只允许**单个标识符、不带类型注解**；多参或带注解一律用括号式。
```

### 2.11 Error Propagation Operator

```
ErrorPropagate ::= Expr '?'
```

The `?` operator is a postfix operator with the same precedence as `.`. For the `Result(T, E)` type:

- With `Ok(v)`, it extracts the value `v` and continues execution.
- With `Err(e)`, it propagates the error upward (`return Err(e)`).

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

// A Range is a value: it can be bound, passed, and checked for membership
r = 1..10
assert.assert(5 in r, "membership")
for i in r { print(i) }

// Step form (third component, defaults to 1)
for i in 0..10..2 { print(i) }  // 0, 2, 4, 6, 8
for i in 10..0..(-2) { print(i) }  // 10, 8, 6, 4, 2
```

> **Step semantics**: in `a..b..c`, `c` is the step size. The literal `c = 0` is rejected at compile
> time; a dynamic `c` receives a runtime zero check (E6001 family; promoted to Result once the error
> system is implemented). `c < 0` is valid, and the interval direction reverses with its sign
> (`10..0..(-2)` counts downward).
>
> **Separation in pattern position**: a match range pattern `lo..hi` (§2.8) does not construct a
> Range value, and its endpoints use closed-interval matching semantics; the expression-position
> form `a..b` in this section remains half-open.

### 2.13 ref Expressions

```
RefExpr     ::= 'ref' Expr
```

`ref` creates shared ownership. The compiler automatically selects Rc (single-task) or Arc
(cross-task), and users need not concern themselves with implementation details.

```yaoxiang
data = ref heavy_data
spawn { use(data) }   // Across tasks: the compiler automatically chooses Arc
```

### 2.14 unsafe Expressions

```
UnsafeExpr  ::= 'unsafe' Block
```

An `unsafe` block is used to define opaque types and operate on raw pointers. Use `return` to return
a type definition to the enclosing scope.

**Semantics**:

- Types and raw-pointer operations may be defined inside `unsafe {}`.
- Returned types are available outside `unsafe {}`.
- Accessing a type’s fields requires unsafe permission.

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

- Every `{}` block creates a scope.
- An inner scope can access variables in an outer scope.
- An outer scope cannot access variables in an inner scope.
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

- `x = value`: searches outward through the scope chain for `x`; if found, it assigns to it, and
  otherwise it declares a new binding.
- `mut x = value`: explicitly declares a new mutable binding and prohibits the same name in an outer
  scope.
- Any name may be declared only once within the same scope.

> **Detailed definition**: see the [Module System Specification](modules.md#第四章作用域) for the
> complete scope rules and the variable declaration and shadowing mechanisms.

---

## Chapter 3: Statements

### 3.1 Statement Categories

```
StmtList    ::= Stmt (StmtSep Stmt)*
StmtSep     ::= ';' | <换行>
Stmt        ::= LetStmt
              | DestructureStmt
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

#### 3.2.1 Destructuring Assignment

```
DestructureStmt ::= '(' Pattern (',' Pattern)* ')' '=' Expr
```

The elements of the right-hand value are bound positionally to the patterns. A bare identifier in a
pattern introduces a **new name** (the same rule as match patterns); it is neither a comparison nor
an assignment to an existing name. The right-hand value’s arity must match the arity of the
destructuring pattern.

```yaoxiang
(a, b) = (1, 2)
```

### 3.3 return Statements

```
ReturnStmt  ::= 'return' Expr?
```

**Semantics**: `return` is a **nonlocal exit** from the nearest **function boundary** (passing
through every block—including `if` / `while` / `for` / `match` / bare blocks / `spawn` / `unsafe`)
and passes its value to the caller. It does **not “return to the block.”**

**Type**: `return e : Never` (`e : T`). `Never <: T'` holds for any `T'` (principle of explosion;
see [type-system §2.2](type-system.md)), so `return` may appear in any return-type position without
requiring an additional rule.

**Relationship to block evaluation**: a block’s value is always its **tail expression** (see §2.9).
As a block, `{ return n }` has the value `n` and the type `Never`; at the same time, `return` exits
the function. **Both facts hold simultaneously**, coexisting through the principle of explosion.

`return` and tail expressions together make “early return” possible, without requiring an additional
rule specifying that `return` targets functions—see
[RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md).

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 {
        return 1          // Exits the if and returns from the function (type Never)
    }
    n * factorial(n - 1)  // Tail expression = the block's value
}
```

### 3.4 break Statements

```
BreakStmt   ::= 'break'
```

**Semantics**: immediately terminates the innermost enclosing `while`/`for` loop, transferring
control to after the loop body.

- **Exits only the nearest level**: `break` always acts on the innermost loop that contains it. When
  a nested loop must be exited at multiple levels, extract the inner loop into a function and use
  `return`, or use a flag (break/continue do not carry labels; if loop labels are introduced in the
  future, they will follow the RFC process for loop declaration syntax and be decided together with
  the proof pipeline’s multi-exit design).
- **Permitted only inside a loop body**: `break` may appear only inside a `while`/`for` loop body
  (including nested blocks/if/match statements within the body). Using it outside a loop produces a
  compile error (E1102 `'break' outside of a loop`).
- **Does not affect termination proofs**: `break` does not participate in termination arguments—it
  neither provides a measure nor constitutes a decreasing measure step. The loop’s termination
  obligation is independent of `break` and is triggered by refinement types (see
  [type-system §8.5](type-system.md#85-terminates终止测度谓词)).
- **Borrowing semantics**: a break control-flow edge participates in the structural cut used by
  RFC-009a’s reverse-BFS liveness analysis (the exited iteration does not participate in back-edge
  liveness inference).

```yaoxiang
mut i = 0
while i < 10 {
    i = i + 1
    if i == 3 {
        break              // Control flow proceeds after the loop, with i == 3
    }
}

// Nested loops: break exits only the inner loop
while j < 3 {
    while k < 10 {
        if k == 2 { break }    // Terminates only the inner loop
    }
    j = j + 1                  // This is reached on every outer-loop iteration
}
```

### 3.5 continue Statements

```
ContinueStmt::= 'continue'
```

**Semantics**: skips the remaining statements in the current iteration and proceeds directly to the
next iteration of the innermost enclosing loop—`while` reevaluates its condition, and `for` takes
the next element.

- **Acts only on the nearest level**: same as `break`, without labels.
- **Permitted only inside a loop body**: using it outside a loop produces a compile error (E1102).

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

YaoXiang’s for-loop semantics differ from those of traditional languages: **each iteration binds a
new value rather than modifying the same variable**.

```yaoxiang
// Example: for i in 1..5
for i in 1..5 {
    print(i)
}
```

**Execution process**:

| Iteration | Behavior of the loop variable                                                                           |
| --------- | ------------------------------------------------------------------------------------------------------- |
| 1st       | Creates a new binding `i = 1`, executes the loop body, and prints 1                                     |
| 2nd       | Creates a new binding `i = 2` (the previous binding is destroyed), executes the loop body, and prints 2 |
| 3rd       | Creates a new binding `i = 3`, executes the loop body, and prints 3                                     |
| 4th       | Creates a new binding `i = 4`, executes the loop body, and prints 4                                     |
| End       | The loop body ends, and the binding is destroyed                                                        |

**Key point**: after each iteration ends, the binding created for that iteration is destroyed. The
next iteration is an entirely new binding, unrelated to the previous iteration’s binding.

#### 3.9.2 The Difference Between for and for mut

| Syntax              | Loop variable mutability | Description                                     |
| ------------------- | ------------------------ | ----------------------------------------------- |
| `for i in 1..5`     | Immutable                | The binding cannot be modified in the loop body |
| `for mut i in 1..5` | Mutable                  | The binding can be modified in the loop body    |

```yaoxiang
// Valid: each iteration binds a new value, so no modification is needed
for i in 1..5 {
    print(i)  // Read the value of i
}

// Error: an immutable binding cannot be modified
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

| Language | for-loop variable semantics                                  |
| -------- | ------------------------------------------------------------ |
| YaoXiang | Binds a new value on every iteration                         |
| Rust     | Modifies the same variable (requires `mut`)                  |
| Python   | Modifies the same variable (does not require `mut`)          |
| C/C++    | Modifies the same variable (requires a pointer or reference) |

**Design rationale**: YaoXiang adopts binding semantics because:

1. **It better matches natural semantics.** In natural language, “for each element x in a set” means
   that each x is an independent individual. YaoXiang’s `for i in 1..5` reads as “for each i from 1
   through 5,” with every iteration’s i being an entirely new binding. This is consistent with human
   intuition.

2. **It avoids accidental modification.** Immutable bindings are the default, so a loop variable
   cannot be accidentally modified in the loop body. There is no need to worry about accidentally
   writing `i = ...` somewhere in a complex loop body and creating a difficult-to-trace bug.

3. **High-performance solutions remain readily available.** When reusing a variable between
   iterations is genuinely necessary (for example, for an accumulator or cache), use a `for mut`
   declaration to switch to mutable binding mode. This is clearer than implicit shared state—the
   intent is expressed explicitly through syntax rather than hidden in runtime behavior.

### 3.10 spawn Statements

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' 'mut'? Identifier 'in' Expr '{' Expr '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
```

**spawn block**: explicitly declares a concurrency scope; expressions in the block execute
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

- When the block body references an outer variable, it performs a **Move value capture**: the value
  is snapshotted into the closure environment at the spawn creation point, and the block body reads
  it through the environment (`LoadUpvalue`).
- **Primitives** (Int/Float/Bool/Char) are copied, and the outer variable is unaffected.
- A **handle type** (Struct/String/List, etc.) snapshot copies the handle and shares the underlying
  object; in the Embedded runtime (the default), the same thread and heap are used, so the handle
  remains valid.
- Sharing between multiple tasks requires an explicit `ref` (§2.13; the compiler automatically
  selects Rc/Arc).
- An outer variable referenced by a `return` inside the block is captured in the same way.

```yaoxiang
t1 = 1 + 1
t2 = 2 + 2
result = spawn {
    return t1 + t2    // t1/t2 are captured by value; result == 6
}
```

---

### 3.11 Program Entry Points and Top-Level Statements

A source file has two roles from the compiler’s perspective, **determined by whether `yaoxiang.toml`
exists**:

| Role       | Determination                                       | Program body                                                                        |
| ---------- | --------------------------------------------------- | ----------------------------------------------------------------------------------- |
| **Script** | A single file run directly, without `yaoxiang.toml` | **Top-level statements** (executed in written order); `main` is an ordinary binding |
| **Bin**    | `yaoxiang.toml` exists                              | **The `main` function**; top-level executable statements are forbidden              |

#### Script: Top-Level Statements Are the Program

Without a manifest, the file is a “script,” and **top-level statements execute in written order**
(top-level **binding initialization** as a whole occurs first; see
“[Top-Level Binding Initialization](#顶层绑定的初始化)” below):

```yaoxiang
use std.io
io.println("hello")          // Executed directly
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

> **Why is `main` not called automatically?** Top-level statements already constitute the program
> body. If `main` were also called implicitly, a script that explicitly calls `main()` would execute
> twice. Both rules cannot coexist, so Script mode has only one execution entry point: top-level
> statements.

Top-level code must likewise **not use `return`**: the Script top level is the module initialization
layer and has no function boundary to exit from (RFC-010a rule ②). It reports a compile-time
`E1109`, rather than silently terminating initialization at runtime and skipping the remaining
top-level statements. To end a section of top-level logic early, place it in a function and control
it at the call site.

#### Bin: `main` Is the Entry Point

When a manifest is present, the file is an “executable target.” In this case:

- A binding named `main` must be defined, and its value may be either a value or a function (#388
  decision: entry-point detection is based on the existence of the binding—a function is already a
  value, and the two differ only in evaluation strategy): function `main` is called with no
  arguments at entry; value `main` executes when evaluated during initialization.
- Top-level executable statements are forbidden—the program body is `main`.

```yaoxiang
main: () -> Void = {
    print("hello")
}
```

Value `main` is equally valid—it executes during the initialization sequence (according to the
global binding topological order, with independent bindings following source order):

```yaoxiang
main = {
    print("hello")
}
```

A missing `main` is a compile error (without `main`, all functions are unreachable). A non-callable
value `main` (such as `main: Int = 5`) is valid but has no observable effect—this corresponds to
Rust’s empty `fn main() {}`.

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

Initialization executes in **dependency order**, independently of written order (a dependency is
initialized before its dependent; bindings without dependencies follow declaration order):

```yaoxiang
derived: Int = base * 3           // References base, which is declared later
base: Int = 7                     // Initialized first (topological sort)
```

Furthermore, **all binding initialization occurs before top-level statements execute**—side effects
in binding initializers happen before any top-level statement, regardless of how the two are
interleaved in the source:

```yaoxiang
print("a")                        // Executed later
x = { print("b") }                // Executed first (all binding initialization comes first)
print("c")                        // Output order: b, a, c
```

A circular dependency is a compile error (the names on the cycle are listed):

```yaoxiang
a: Int = b + 1
b: Int = a + 1                    // Error: a → b → a
```

> **Design basis**: RFC-029f (file-role model) and RFC-010a, Appendix D (block binding decisions).

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
entirely under RFC-029g and is now an ordinary identifier. The literal reserved words `true` /
`false` / `void` are covered in §1.4.1. The meta type name `Type` and the built-in type names `Void`
/ `Never` / `Int` / `Float` / `Bool` / `Char` / `String` are covered in §1.4.3.

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
    pattern1 -> expr1,
    pattern2 if guard -> expr2,
    _ -> default_expr,
}
```
