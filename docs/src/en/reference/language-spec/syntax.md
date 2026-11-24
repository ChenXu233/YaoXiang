# Syntax Specification

This document defines the syntax specification of the YaoXiang programming language, including
lexical structure, grammar rules, and operator precedence.

---

## Chapter 1: Lexical Structure

### 1.1 Source Files

YaoXiang source files must use UTF-8 encoding. Source files usually use the `.yx` extension.

### 1.2 Lexical Token Categories

| Category   | Description                                | Example                   |
| ---------- | ------------------------------------------ | ------------------------- |
| Identifier | Starts with a letter or underscore         | `x`, `_private`, `my_var` |
| Keyword    | A reserved word predefined by the language | `use`, `mut`, `and`       |
| Literal    | A fixed value                              | `42`, `"hello"`, `true`   |
| Operator   | An operation symbol                        | `+`, `-`, `*`, `/`        |
| Separator  | A syntactic separator                      | `(`, `)`, `{`, `}`, `,`   |

### 1.3 Keywords

YaoXiang has **17 keywords** (`keyword_from_str` in `src/frontend/core/lexer/state.rs`, with one
corresponding `TokenKind` for each):

```
use     spawn   ref     mut
if      else    match   while   for
in      return  break   continue
as      unsafe  and     or
```

`and` / `or` are **keywords** for logical AND / OR (Zig-style; see level 10 in §2.2); unary negation
is the symbol `!`, which is orthogonal to them.

These keywords have special meanings in every context and cannot be used as identifiers.

> **`type` is no longer a keyword** (RFC-010): use the `Name: Type = { ... }` notation to define a
> type. Both `keyword_from_str` and the `TokenKind` enum header
> (`src/frontend/core/lexer/tokens.rs`) are annotated with “RFC-010: 'type' keyword removed”.
>
> **`pub` is no longer a keyword** (RFC-029g, accepted on 2026-10-02): the language has no
> visibility mechanism. `pub` has been removed as a whole from the lexer, AST, type checker,
> dead-code exemptions, formatter, and LSP
> ([RFC-029g](../../rfc/accepted/029g-remove-pub-and-auto-bind.md)). It is now an ordinary
> identifier—the old syntax `pub x = 1` reports E1001 as an undefined name. All top-level bindings
> are importable by default, and modifiers make no difference to visibility
> ([RFC-029](../../rfc/accepted/029-module-semantics.md)).

### 1.4 Reserved Words

YaoXiang’s “reserved words” are divided into three layers, recognized at different stages by the
parser and type checker:

#### 1.4.1 Literal Reserved Words

The parser treats the following literal identifiers as distinct tokens, so they cannot be used as
ordinary identifiers:

| Identifier | Owning Type | Description                                                                                                   |
| ---------- | ----------- | ------------------------------------------------------------------------------------------------------------- |
| `true`     | Bool        | Boolean true                                                                                                  |
| `false`    | Bool        | Boolean false                                                                                                 |
| `void`     | Void        | Void literal (Unit value). Lowercase `void` is a value literal; uppercase `Void` is a type name (see §1.4.3). |

> **`Type` is not in this layer**: it has **no** distinct `TokenKind` (it is absent from the
> `TokenKind` enum in `src/frontend/core/lexer/tokens.rs`, and `keyword_from_str` has no
> corresponding branch). The parser treats it as an ordinary identifier, while the type checker
> recognizes it as a meta type in type positions. Therefore, in expression positions, `Type` can be
> shadowed by a local binding. It is a **meta type name**, not a keyword.

#### 1.4.2 Variant Names

The parser recognizes variant names in **pattern** contexts; bare names are adjudicated according to
the scrutinee type’s variant set (see §2.8). In **expression** contexts, constructing a variant
requires type qualification—`Result(Int, String).ok(5)`, `Option(T).some(v)`, `Color.green()`
(RFC-010 record style and types: variant constructors are function-typed fields in type definitions,
so the bare name `ok(5)` is not a constructor call).

| Variant Name | Owning Type | Description                    |
| ------------ | ----------- | ------------------------------ |
| `some(T)`    | Option      | Constructs an Option value     |
| `ok(T)`      | Result      | Constructs a successful Result |
| `err(E)`     | Result      | Constructs an erroneous Result |

#### 1.4.3 Built-in Type Names

The type checker pre-registers the following type names. They can be used in type positions without
being imported. The parser treats them as ordinary identifiers—they are **not reserved words and can
be shadowed by local bindings (not recommended)**.

| Type Name | Logical Correspondence | Description                                                                                                                                  |
| --------- | ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `Void`    | ⊤ (true/Unit)          | Zero-field product type with exactly one inhabitant (the `void` literal; see §1.4.1)                                                         |
| `Never`   | ⊥ (false/empty type)   | Zero-variant type with no inhabitants. No expression can produce a `Never` value. `Never <: T` holds for every `T` (principle of explosion). |
| `Int`     | —                      | Signed integer                                                                                                                               |
| `Float`   | —                      | Floating-point number                                                                                                                        |
| `Bool`    | —                      | Boolean value: `true` / `false`                                                                                                              |
| `Char`    | —                      | Unicode character                                                                                                                            |
| `String`  | —                      | String                                                                                                                                       |

### 1.5 Identifiers

An identifier starts with a letter or underscore. Subsequent characters may be letters, digits, or
underscores. Identifiers are case-sensitive.

Special identifiers:

- `_` serves as a placeholder, indicating that a value is to be ignored.
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
> dictionary—it is an empty block (the value `Void`; see [§2.9](#_2-9-块表达式)). Use the
> constructor `dict.new()` for an empty dictionary:
>
> ```yaoxiang
> empty = dict.new()                  // ✅ Empty dictionary
> d = { "a": 1 }                       // ✅ Dictionary literal
> wrong = {}                           // ❌ Not a dictionary; it is an empty block (Void)
> ```
>
> **The criterion is the content**: the `Dict` grammar requires at least one `String ':' Expr`.
> Since `{}` has no content to inspect, it takes the zero form of a block structure. A non-empty
> form is self-describing through its content (`{ "k": v }` has a key-value pair → dictionary)—the
> same principle as `f = { 5 }` being an `Int` value rather than a function: **the type is
> determined by the content**.

> Set has no literal grammar and no runtime representation—when Set is needed in type planning,
> complete it by following the Dict pattern (std.set + HeapValue::Set). The resolution of List/Dict
> literals is determined by the contextual type annotation: a bare literal and a `List(T)`
> annotation both resolve to a growable list—the growable-list type surface has the single name
> `Vec(T)`, and a `List(T)` annotation is normalized by source name to the same type surface as
> `Vec(T)`; an `Array(T, N)` annotation applied directly to a literal resolves to a fixed-length
> array. Implicit List→Array conversion is prohibited.
>
> Array literal semantics:
>
> - The number of elements must equal N; otherwise, compilation fails with E1002. An empty literal
>   paired with a nonzero N is rejected in the same way.
> - Every element type must be compatible with T; otherwise, compilation fails with E1002.
> - The grammatical forms of N are: an integer literal (which may be negative), a constant name, or
>   a constant expression beginning with a reference to a compile-time value parameter and continued
>   with infix arithmetic, such as `2+3` or `N+1`. The folded value participates in count validation
>   (`2+3` has the same form as `5`).
> - Parse-time rejection: floating-point/string seeds (E0010) and incomplete expressions such as
>   `2+` (E0012); an identifier followed by `(` follows the constructor-type path—a function call
>   cannot serve as a type-position argument (E1003).
> - When N is a symbolic constant (such as a function const parameter in `Array(Int, n)`), count
>   validation is deferred to the refined-type stage.
> - v1 rejects nested array literals (`Array(Array(Int,2),2) = [[1,2],[3,4]]`) at compile time; each
>   level must be constructed explicitly. Recursive resolution is deferred to a later version.

#### 1.6.5 List Comprehensions

```
ListComp    ::= '[' Expr ( 'for' Identifier 'in' Expr ( 'if' Expr )? )+ ']'
```

> **#401 (implementation completion)**: `if` filters and multiple-generator clauses are implemented
> starting with this version. The previous grammar was written as `(',' Expr)* ('if' Expr)?`, with
> no implementation corresponding to either comma-appended expressions or multiple generators, while
> the parser also omitted the `if` filter entirely (after parsing the iterable, it expected `']'`
> and reported `E0010 Expected RBracket, found KwIf`). The semantics are now established according
> to common industry practice: one or more generator clauses are expanded as nested loops; each
> clause may carry at most one `if` filter, and its condition must be `Bool` (a non-Bool condition
> reports `E1054`). The iterable and condition of later clauses may refer to iteration variables
> bound by earlier clauses.

> **Behavior tightening (migration record)**: the iteration-variable grammar has always been
> `'for' Identifier 'in'`, but the old implementation ran patterns through the full Pratt
> parser—after `in` was registered as an infix operator, `x` would swallow `in items` as a
> membership expression. After the fix, non-identifier patterns fail to parse directly and no longer
> fall back to `_` as the old implementation did (silently swallowing the error). Impact: previously
> parseable forms such as `[x for (a, b) in pairs]` now report an error. This form never had defined
> behavior (the variable was always `_`), so tightening it is correct and incurs no semantic
> migration cost.

#### 1.6.6 Membership Testing

```
Membership  ::= Expr 'in' Expr
```

> `in` is a binary relational operator that returns `Bool`—`true` on a match and `false` otherwise,
> without reporting an error. The semantics are split: `[]` asserts existence and retrieves a value
> (reporting an error on failure), whereas `in` asks whether something exists (a miss is the normal
> `false` result). The right operand may be a List / Array / Dict (key set) / Tuple / String
> (substring) / Range (interval). `in` is a first-class Horn predicate and serves as the basis for
> compile-time-provable propositions during refined-type checking. (Set is omitted from the list of
> right operands—Set has no runtime representation; see §1.6.4.)

### 1.7 Comments

```
// Single-line comment

/* Multi-line comment
   Can span multiple lines */
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

> **Unary prefix operators** (`!` `-` `+`) bind tightly: they bind only less tightly than calls and
> member access, and more tightly than all binary operators. Therefore, `!a == b` ≡ `(!a) == b`
> (Zig-style semantics). `!` is a purely unary operation, does not participate in short-circuit
> control flow, and is orthogonal to the `and`/`or` keywords (which short-circuit) (RFC-010 is
> authoritative).

> **Range binding strength**: `..` has binding strength (6, 7)—the left side, 6, binds less tightly
> than addition (7), while the right side, 7, consumes addition but not another `..` at the same
> level. Comparison before and after the change:
>
> | Expression   | Before the Change (level 1, right-associative)                                                                            | After the Change ((6,7), left-associative)                                            |
> | ------------ | ------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
> | `x in 1..10` | `x in 1..10` (the right operand of `in` has level 4, and level-1 `..` cannot consume it, so it cannot actually be parsed) | `x in (1..10)`—the interval is the complete right operand of `in`                     |
> | `0..n+2`     | `(0..n)+2` (right-associativity trap: the upper bound is consumed, causing the `for` loop to report E3004 directly)       | `0..(n+2)`—the upper bound is an arithmetic expression                                |
> | `a == b..c`  | `a == (b..c)` (level-1 `..` < level-3 `==`, so it is naturally a complete unit)                                           | `a == (b..c)`—**semantics unchanged**; `..` still binds more tightly than comparisons |
> | `1..2*3`     | `(1..2)*3`                                                                                                                | `1..(2*3)`—the upper bound is an arithmetic expression                                |
> | `a..b..c`    | `a..(b..c)` (right-associative chain; nesting a Range inside a Range is meaningless)                                      | `(a..b)..c`—**step form** (`c` is the step size)                                      |
>
> Net effect: a compound upper bound such as `for i in 0..n+2` changes from “parses successfully but
> reports E3004” to “directly usable”; `x in 1..10` changes from “cannot be parsed” to “interval
> check”; and `a..b..c` changes from “meaningless nesting” to “step components”. Level 6 falls
> between `+` (level 5) and `<<` (level 7), following mathematical convention: an interval is a
> tightly bound construct, so its upper bound is naturally a complete arithmetic expression.

### 2.3 Function Calls

```
FnCall      ::= Expr '(' ArgList? ')'
ArgList     ::= Expr (',' Expr)* (',' NamedArg)* | NamedArg (',' NamedArg)*
NamedArg    ::= Identifier '=' Expr
```

Named arguments use `name = value` (RFC-010 §Function Definitions, RFC-011 §Constructor Forms).
Positional arguments must precede named arguments; positionally specified parameters may appear in
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

**Unnamed parameters and named arguments**: a signature parameter position permits a bare type form
(`mk: (Int, Int) -> Int`). This is an **unnamed typed parameter**—the type constraint applies
positionally, while the name belongs to the implementation (provided by the lambda header; see the
shorthand rules in [RFC-007](../../rfc/accepted/007-function-syntax-unification.md)) and is not part
of the contract. An unnamed parameter cannot be referenced by a named argument because it has no
name; it can only be passed positionally. To include names in the contract, write
`mk: (a: Int, b: Int) -> Int`. A bare identifier must be a declared type; failure to resolve it
reports an error (see [type-system §3.5](type-system.md#35-函数类型)).

**Named parameters are the opposite: names enter the contract, and the declaration is
authoritative**. Parameter names in an implementation header (the parameter list after `=>`) are
merely local names in the body and are bound **positionally**—if the implementation header of
`f: (a: Int) -> Int = (b) => b` uses `b`, callers still write `f(a = 7)`. Type annotations in the
implementation header serve only for comparison; a mismatch with the declaration reports **E1011**.
If the implementation header has more parameters than the declaration’s runtime parameter slots, it
reports **E1112** (`f: () -> Int = (a: Int) => a`)—in both cases, the declaration is authoritative.
The reverse case does not report an error: the type layer checks against the declared arity, but a
position omitted from the implementation header has no runtime slot, and arguments assigned to it
are discarded on the IR side (see the gap registry in
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
> containers (List/Vec/Array/Dict/Tuple) use a native indexing instruction (the fast path); ② user
> types that implement the `Index` interface are dispatched to their `index` method (instantiation
> of `Index(Grid, Int, Float)` within the type body plus a `Grid.index` method); ③ the RFC-004
> `f[0]` positional binding exists only in binding declarations and is not part of this grammar. The
> keys of a multidimensional index such as `a[0, 1]` are packed into a tuple. Other types that do
> not implement `Index` are rejected at the type layer.

### 2.6 Type Casts

```
TypeCast    ::= Expr 'as' TypeExpr
```

> **Implementation status (pre-existing gap, not introduced in this version)**: the preceding text
> defines only the grammar and precedence (§2.4, level 2); the semantics have not yet been
> implemented. The independently verifiable facts are as follows: ① Parsing: the `KwAs` infix arm in
> `parser/pratt/led.rs:81` produces `ast::Expr::Cast`; ② Type checking:
> `typecheck/inference/expressions.rs:3847-3853` performs only **silent re-annotation** on
> `Expr::Cast`—`let _ = self.infer_expr(expr)?;` discards the inner result and then directly returns
> the `target_type` `MonoType`; ③ There is **no** convertibility validation anywhere in the
> repository, nor any corresponding diagnostic code (there is no cast entry in
> `util/diagnostic/codes/`); ④ `ir_gen` generates no instructions (the only uses of `Expr::Cast` in
> `middle/core/ir_gen.rs` are at `:5111`, `:5918`, and `:9381`, where its span and free variables
> are obtained—the first two are the span arms of `get_expr_span` / `expr_span`, and the third is
> `collect_free_vars_expr`; none generates instructions); ⑤ Consequently, `x as Int` currently
> neither reports an error nor produces any runtime conversion.

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
              | RangePattern
              | StructPattern
              | TuplePattern
              | EnumPattern
              | OrPattern
RangePattern ::= ('-')? Literal '..' ('-')? Literal   // Endpoints may have a negative sign prefix; negative values are normalized—see the "Range Patterns" note below
```

> **Variant-set requirement for variant destructuring** (RFC-010b): `EnumPattern` (such as `ok(v)`
> or `some(x)`, where variant names are adjudicated according to the scrutinee) requires the
> scrutinee’s type to be a **union type with a present variant set**. The variant set enters the
> checker only through a type definition or a `use` import—`Result`/`Option` are defined by
> `std.result`/`std.option`, so `use std.result` / `use std.option` is required before use
> (whole-module and grouped `use std.{...}` forms have equal standing); otherwise, variant
> destructuring reports E1002. Exhaustiveness checking follows the same principle: a fallback arm is
> exempt from exhaustiveness, while without one, the entire variant set is checked.

> **Pattern bindings, or-patterns, and guards** (RFC-010b):
>
> - **Bindings**: a bare identifier in a pattern introduces a **new name**—it is neither a
>   comparison nor an assignment. The name is visible **within the arm**, shadows an outer variable
>   of the same name, and ceases to exist after the arm. The discriminant (`ok` in `ok(v)`) is
>   adjudicated by looking it up in the scrutinee’s variant set; a bare name that is not found is a
>   binding.
> - **Or-patterns**: `p1 | p2` tries alternatives in order; if a non-final alternative fails, the
>   next is tried, and if the final alternative fails, control proceeds to the next arm. All
>   alternatives must bind **the same set of names**, otherwise E1033 is reported. A binding with
>   the same name exists only once at the arm level, and the value read by the arm body is
>   necessarily the value written by the matching alternative.
> - **Guards**: after a pattern matches, the `if` condition is evaluated, with pattern bindings
>   visible to the condition. The condition must produce `Bool`; if it is false, control proceeds to
>   the next arm. A guarded arm **does not count toward exhaustive coverage** (the guard might not
>   match), but a repeated variant arm with a guard may still be reachable (it refines the previous
>   arm).
> - **Diagnostics**: missing branch E1030, unreachable arm E1031, duplicate binding in the same
>   pattern E1032, inconsistent or-pattern bindings E1033, and missing fields in a struct pattern
>   E1034; see the [Error Code Reference](../error-code/E1xxx.md) for the code table and messages.
>
> **Range patterns** (issue #405): `lo..hi` is a **closed-interval test**—over the Int domain,
> `x matches lo..hi ⟺ lo <= x <= hi`, so both 80 and 89 match `80..89`. Endpoints must be **integer
> literals** (a negative sign prefix is normalized to a negative value, such as `-5..-1`); other
> literal types (Float/Bool/String/Char) are syntactically valid but are rejected during type
> checking (E1002; Float ranges are unsupported in the initial version). Identifier endpoints in
> pattern positions are binding names with no test semantics and are rejected at the syntax layer
> (E0010); when both endpoints are decidable at compile time, a reversed range (`lo > hi`) is empty,
> never matches, and is reported as unreachable (E1031); duplicate range arms with identical
> endpoints are likewise reported as E1031.
>
> Exhaustiveness interaction: range patterns are checked in the same way as literal
> patterns—**checked, partially covering, and non-fallback**. Because the Int domain is open, a
> range arm cannot be exhaustive, so a match containing range arms still requires a
> wildcard/identifier arm as a fallback (E1030). A range arm may appear as an or-pattern alternative
> and may have a guard (a failed guard proceeds to the next arm, consistently with the existing
> guard rules).
>
> **Semantic separation from §2.12 Range expressions**: a pattern-position `lo..hi` does not
> construct a Range value; it only performs branch selection, with closed-interval endpoint
> semantics. An expression-position `a..b` (for `for` iteration, `in` interval checks, and
> first-class Range values) remains left-closed and right-open. The separation between the two
> meanings of `..` is intentional: a pattern position is a selection predicate over a discrete
> domain, whereas an expression position is a notation for constructing an iteration sequence.

### 2.9 Block Expressions

```
Block       ::= '{' Stmt* Expr? '}'
```

> **Statement termination rules**: separation and line-break behavior between statements (`;` as an
> explicit separator, line breaks as terminators, continuation exceptions, and never merging when a
> line begins with `(`/`[`) are defined by
> [RFC-038](../../rfc/accepted/038-statement-termination.md).

#### 2.9.1 The Three Forms of `{`

In expression position, `{` has exactly three interpretations, determined once by its **content**:

| Form                   | Notation          | Type                 | Example          |
| ---------------------- | ----------------- | -------------------- | ---------------- |
| **Empty block**        | `{}`              | `Void`               | `x: Void = {}`   |
| **Dictionary literal** | `{ "k": v, ... }` | `Dict(K, V)`         | `d = { "a": 1 }` |
| **Block**              | `{ Stmt* Expr? }` | Tail-expression type | `y = { 1 + 1 }`  |

**Decision order**:

1. `{}` (no content) → **empty block**, value `Void`
2. The first element is a `String ':' Expr` key-value pair → **dictionary literal**
3. Otherwise → **block**, with its value determined by the tail expression

> **Why `{}` is not an empty dictionary**: the “emptiness” of an empty dictionary cannot describe
> itself (it could be either `Dict(K, V)` or an empty block), while the dictionary grammar
> [§1.6.4](#_1-6-4-集合) requires at least one key-value pair. When there is no content to inspect,
> the zero form of the block structure is used: this is consistent with `unsafe {}` / `spawn {}` and
> introduces no special case. Use `dict.new()` for an empty dictionary.
>
> **Why functions require annotations**: `f = { stmt }` is a **value** (with the tail-expression
> type), not a function. To define a function, specify a Fn annotation: `f: () -> Int = { 5 }`. This
> follows the same principle as dictionaries: **the type is determined by the content**, not by the
> presence of an annotation. (See Appendix D of
> [RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md).)

**Unified semantics**: the value of every `{}` block is provided by its **tail expression**, and
`return` is a non-local exit of type `Never`.

| Block Type    | Value Exit      | Empty Block `{}` |
| ------------- | --------------- | ---------------- |
| Ordinary `{}` | Tail expression | `Void`           |
| `unsafe {}`   | Tail expression | `Void`           |
| `spawn {}`    | Tail expression | `Void`           |

**Core principles** (see [RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md) for
details):

- **A block’s value equals its tail expression** (the final expression); it is the sole exit,
  without exception.
- When the final item is an **assignment statement**, the block’s value is `Void`; to obtain `Void`,
  write `Void` explicitly.
- **`return` exits the nearest function boundary** (passing through every block rather than
  “returning to the block”), and has type `Never`; `Never <: T`

> holds for every type (principle of explosion), so it may appear in any return-type position. The
> module initialization layer (Script top level) has no function boundary, so `return` is rejected
> at compile time there (`E1109`; see §3.11).

- The expression form `= expr` directly provides the value.

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
        return 0     // Passes out of the if and function body, exiting the function
    }
    n * 2            // Tail expression
}
```

#### Is `name = { ... }` a Function or a Block Value?

`name = { ... }` may be either a function definition (RFC-007 “Minimal Empty Parameters”) or a
block-value binding. The decision is **annotation first, function by default** (RFC-010a Appendix
D):

| Case                                  | Result          | Example                            |
| ------------------------------------- | --------------- | ---------------------------------- |
| The value is a Lambda (`=>`)          | Function        | `f = () => 5` → `f()` = 5          |
| The annotation is a function type     | Function        | `f: () -> Int = { 5 }` → `f()` = 5 |
| The annotation is a non-function type | **Block value** | `x: Int = { 5 }` → `x` = 5         |
| No annotation                         | Function        | `f = { 5 }` → `f()` = 5            |

**An annotation is the type**: `x: Int = ...` declares that `x` is an `Int`, so `{ ... }` evaluates
to an `Int`; `f: () -> Int = ...` declares that `f` is a function, so `{ ... }` is a function body.

To evaluate `{ ... }` immediately, **write the target type**—no new syntax is needed:

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

A nested function type on the right-hand side of `->` has two possible readings, distinguished by
**parentheses** (RFC-004):

| Notation                        | Meaning                | Invocation               |
| ------------------------------- | ---------------------- | ------------------------ |
| `(a: Int) -> (b: Int) -> Int`   | **Curried**            | `f(1)(2)`                |
| `(a: Int) -> ((b: Int) -> Int)` | **Returns a function** | `g(1)` yields a function |

The basis is: **an annotation is the type**. `g: (a: Int) -> ((b: Int) -> Int)` declares that
`g(1) : (b: Int) -> Int`, so `g(1)` must **be** that function rather than “the next parameter
segment”.

```yaoxiang
// Currying: provide the two parameter segments in sequence
add: (a: Int) -> (b: Int) -> Int = { a + b }
add(1)(2)        // → 3

// Returning a function: the outer parameter segment returns the function
adder: (n: Int) -> ((x: Int) -> Int) = (x) => x + n
adder(10)(5)     // → 15
h = adder(10)    // The returned function can be stored in a variable and passed around
```

An unparenthesized nested `Fn` is always curried (including the type-parameter form from RFC-011):

```yaoxiang
identity: (T: Type) -> (x: T) -> T = (x) => x   // Curried
identity(5)      // → 5
```

**Type checking**: the parentheses declare the return type, and the body must produce that type.
`f: () -> (() -> Int) = { 7 }` is an error—the expected type is `() -> Int`, while the actual type
is `Int` (E1002).

### 2.10 Lambda Expressions

```
Lambda      ::= '(' ParamList? ')' '=>' Expr
            |  '(' ParamList? ')' '=>' Block
```

### 2.11 Error-Propagation Operator

```
ErrorPropagate ::= Expr '?'
```

The `?` operator is a postfix operator with the same precedence as `.`. For the `Result(T, E)` type:

- On `Ok(v)`, it extracts the value `v` and continues execution.
- On `Err(e)`, it propagates the error upward (`return Err(e)`).

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

// A Range is a value: it can be bound, passed, and tested for membership
r = 1..10
assert.assert(5 in r, "membership")
for i in r { print(i) }

// Step form (third component, default 1)
for i in 0..10..2 { print(i) }  // 0, 2, 4, 6, 8
for i in 10..0..(-2) { print(i) }  // 10, 8, 6, 4, 2
```

> **Step semantics**: in `a..b..c`, `c` is the step size. The literal `c = 0` is rejected at compile
> time; a dynamic `c` undergoes a zero check at runtime (the E6001 family; promoted to Result once
> the error system is implemented). `c < 0` is valid, and the direction of the interval is reversed
> according to the sign (`10..0..(-2)` descends).
>
> **Separation in pattern positions**: a match range pattern `lo..hi` (§2.8) does not construct a
> Range value, and its endpoint semantics are closed-interval tests; the expression-position `a..b`
> in this section remains left-closed and right-open.

### 2.13 ref Expressions

```
RefExpr     ::= 'ref' Expr
```

`ref` creates shared ownership. The compiler automatically selects Rc (single-task) or Arc
(cross-task), so users need not concern themselves with implementation details.

```yaoxiang
data = ref heavy_data
spawn { use(data) }   // Cross-task: the compiler automatically selects Arc
```

### 2.14 unsafe Expressions

```
UnsafeExpr  ::= 'unsafe' Block
```

An `unsafe` block is used to define opaque types and operate on raw pointers. Use `return` to return
a type definition to the enclosing scope.

**Semantics**:

- Types and raw-pointer operations may be defined in `unsafe {}`.
- A returned type is available outside `unsafe {}`.
- Accessing a type’s fields requires unsafe permission.

```yaoxiang
// Define an opaque type in an unsafe block
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

- `x = value`: searches outward through the scope chain for `x`; if found, assigns to it, and
  otherwise declares a new binding.
- `mut x = value`: explicitly declares a new mutable binding and prohibits the same name in an outer
  scope.
- A name may be declared only once within the same scope.

> **Detailed definition**: see the [Module System Specification](modules.md#第四章作用域) for the
> complete scope rules and the mechanisms for variable declarations and shadowing.

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
`unsafe`) and passes the value to the caller. It does **not “return to the block”**.

**Type**: `return e : Never` (`e : T`). `Never <: T'` holds for every `T'` (principle of explosion;
see [Type System §2.2](type-system.md)), so `return` may appear in any return-type position without
additional constraints.

**Relationship with block evaluation**: a block’s value is always its **tail expression** (see
§2.9). As a block, `{ return n }` has the value `n` and the type `Never`; at the same time, `return`
exits the function. **Both facts hold simultaneously**, coexisting through the principle of
explosion.

Together, `return` and tail expressions make “early return” possible without requiring a special
`return`-is-a-function rule—see [RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md).

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 {
        return 1          // Exits the if and the function (type Never)
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

- **Exits only the nearest level**: `break` always applies to the innermost loop containing it. When
  multiple loop levels must be exited in a nested loop, extract the inner loop into a function and
  use `return` to return from it, or use a flag (break/continue have no labels; if loop labels are
  introduced in the future, their syntax on the declaration side will follow the RFC process and be
  decided together with the proof pipeline’s multi-exit design).
- **Permitted only inside a loop body**: `break` may appear only in a `while`/`for` loop body
  (including nested blocks/if/match statements within the body); using it outside a loop is a
  compile-time error (E1102 `'break' outside of a loop`).
- **Does not affect termination proofs**: `break` does not participate in termination arguments—it
  provides neither a measure nor a decreasing step for one. The loop’s termination obligation is
  independent of `break` and is triggered by refined typing (see
  [type-system §8.5](type-system.md#85-terminates终止测度谓词)).
- **Borrowing semantics**: the control-flow edges of `break` participate in the structural cutoff of
  the RFC-009a reverse-BFS liveness analysis (iterations exited by the break do not participate in
  back-edge liveness derivation).

```yaoxiang
mut i = 0
while i < 10 {
    i = i + 1
    if i == 3 {
        break              // Control flows to after the loop, with i == 3
    }
}

// Nested loops: break exits only the inner loop
while j < 3 {
    while k < 10 {
        if k == 2 { break }    // Terminates only the inner loop
    }
    j = j + 1                  // This executes after every outer iteration
}
```

### 3.5 continue Statements

```
ContinueStmt::= 'continue'
```

**Semantics**: skips the remaining statements in the current iteration and proceeds directly to the
next iteration of the innermost enclosing loop—`while` re-evaluates its condition, while `for` takes
the next element.

- **Applies only to the nearest level**: like `break`, it has no label.
- **Permitted only inside a loop body**: using it outside a loop is a compile-time error (E1102).

```yaoxiang
mut sum = 0
mut n = 0
while n < 5 {
    n = n + 1
    if n == 3 {
        continue           // Skips the accumulation below, so n == 3 is not included
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

The semantics of YaoXiang’s for loop differ from those of traditional languages: **each iteration
binds a new value rather than modifying the same variable**.

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
| End       | The body ends, and the binding is destroyed                                                        |

**Key point**: after each iteration ends, the binding created during that iteration is destroyed.
The next iteration uses a completely new binding, unrelated to the previous iteration’s binding.

#### 3.9.2 Difference Between for and for mut

| Syntax              | Loop-Variable Mutability | Description                                     |
| ------------------- | ------------------------ | ----------------------------------------------- |
| `for i in 1..5`     | Immutable                | The binding cannot be modified in the loop body |
| `for mut i in 1..5` | Mutable                  | The binding can be modified in the loop body    |

```yaoxiang
// Valid: each iteration binds a new value, so no modification is needed
for i in 1..5 {
    print(i)  // Reads the value of i
}

// Error: an immutable binding cannot be modified
for i in 1..5 {
    i = i + 1  // Error: cannot modify an immutable binding
}

// Valid: use for mut to permit modifying the binding
for mut i in 1..5 {
    i = i + 1  // Modification is permitted
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

This rule applies to all code blocks. See [4.3 Shadowing Rules](modules.md#43-遮蔽规则).

#### 3.9.4 Comparison with Other Languages

| Language | Semantics of the for-Loop Variable                           |
| -------- | ------------------------------------------------------------ |
| YaoXiang | Binds a new value on each iteration                          |
| Rust     | Modifies the same variable (requires `mut`)                  |
| Python   | Modifies the same variable (does not require `mut`)          |
| C/C++    | Modifies the same variable (requires a pointer or reference) |

**Design rationale**: YaoXiang uses binding semantics because:

1. **It better matches natural semantics.** In natural language, “for each element x in a set” means
   that each x is an independent individual. YaoXiang’s `for i in 1..5` reads as “for each i from 1
   to 5,” where i in each iteration is a completely new binding, consistent with human intuition.

2. **It avoids accidental modification.** Immutable bindings by default mean that a loop variable
   cannot be accidentally modified within the loop body. There is no need to worry about
   accidentally writing `i = ...` somewhere in a complex loop body and creating a bug that is
   difficult to track.

3. **High-performance solutions remain readily available.** When a variable genuinely must be reused
   across iterations—for example, as an accumulator or cache—use a `for mut` declaration to switch
   to mutable-binding mode. This is clearer than implicit shared state: intent is expressed
   explicitly in the syntax rather than hidden in runtime behavior.

### 3.10 spawn Statements

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' 'mut'? Identifier 'in' Expr '{' Expr '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
```

**spawn block**: explicitly declares a concurrency boundary; expressions within the block execute
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

- A reference to an outer variable in the block body means a **Move value capture**: the value is
  snapshotted into the closure environment at the spawn point, and the block body reads it through
  the environment (LoadUpvalue).
- **Primitives** (Int/Float/Bool/Char) are copied, and the outer variable is unaffected.
- For **handle types** (Struct/String/List, etc.), snapshotting means copying the handle while
  sharing the underlying object; under the Embedded runtime (the default), the same thread and heap
  are used, so the handle remains valid.
- Sharing between multiple tasks requires an explicit `ref` (§2.13; the compiler automatically
  selects Rc/Arc).
- A `return` within the block also captures any referenced outer variables.

```yaoxiang
t1 = 1 + 1
t2 = 2 + 2
result = spawn {
    return t1 + t2    // t1/t2 are value-captured; result == 6
}
```

---

### 3.11 Program Entry Point and Top-Level Statements

A source file has two possible roles as viewed by the compiler, **determined by whether
`yaoxiang.toml` exists**:

| Role       | Determination                                       | Program Body                                                                       |
| ---------- | --------------------------------------------------- | ---------------------------------------------------------------------------------- |
| **Script** | A single file run directly, without `yaoxiang.toml` | **Top-level statements** (executed in source order); `main` is an ordinary binding |
| **Bin**    | `yaoxiang.toml` exists                              | **The `main` function**; top-level executable statements are prohibited            |

#### Script: Top-Level Statements Are the Program

Without a manifest, a file is a “script,” and **top-level statements execute in source order** (the
complete **top-level binding initialization** runs first; see
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

main()                       // ← This line must be written
```

> **Why is `main` not invoked automatically?** Top-level statements are already the program body. If
> `main` were also invoked implicitly, a script that explicitly calls `main()` would execute it
> twice. Both rules cannot coexist, so Script has only one execution entry point: its **top-level
> statements**.

Top-level code must likewise **not use `return`**: the Script top level is the module initialization
layer and has no function boundary to exit from (RFC-010a rule ②). Compilation reports `E1109`,
rather than silently terminating initialization at runtime and skipping the remaining top-level
statements. To end a section of top-level logic early, place it in a function and control execution
at the call site.

#### Bin: `main` Is the Entry Point

When a manifest is present, the file is an “executable target,” and:

- It must define a **binding** named `main`, whose value may be either a value or a function (#388
  establishes the final decision: the entry point is determined by the binding’s existence—functions
  are already values, and the two merely use different evaluation strategies): a `main` function is
  called with zero arguments during entry; a `main` value is evaluated during initialization.
- Top-level executable statements are prohibited—the program body is `main`.

```yaoxiang
main: () -> Void = {
    print("hello")
}
```

A value `main` is equally valid—it is evaluated during initialization as part of the initialization
sequence (in global-binding topological order, with independent bindings ordered by source order):

```yaoxiang
main = {
    print("hello")
}
```

A missing `main` is a compile-time error (without `main`, all functions are unreachable). A
non-callable value `main`, such as `main: Int = 5`, is valid but has no observable effect—analogous
to Rust’s empty `fn main() {}`.

> **Library files**: files imported by other files with `use`, or files referenced by `[lib].path` /
> `[exports]`, do not require `main`—they are not program entry points.

#### Initialization of Top-Level Bindings

The initialization values of top-level bindings are **evaluated at runtime** and are not required to
be compile-time constants:

```yaoxiang
answer: Int = { 42 }              // Block value
inc: (Int) -> Int = (x) => x + 1
computed: Int = inc(41)           // Function call
```

Initialization executes in **dependency order**, independently of source order (dependencies are
initialized first; bindings without dependencies are initialized in declaration order):

```yaoxiang
derived: Int = base * 3           // References base, which is declared later
base: Int = 7                     // Initialized first (topological sort)
```

Furthermore, **all binding initialization occurs before top-level statement execution**—side effects
in binding initializers occur before any top-level statement, regardless of how the two are
interleaved in the source:

```yaoxiang
print("a")                        // Executes later
x = { print("b") }                // Executes first (all binding initialization runs first)
print("c")                        // Output order: b, a, c
```

Circular dependencies are compile-time errors (the compiler lists the names on the cycle):

```yaoxiang
a: Int = b + 1
b: Int = a + 1                    // Error: a → b → a
```

> **Design basis**: RFC-029f (file-role model) and RFC-010a Appendix D (block-binding decisions).

---

## Appendix: Syntax Quick Reference

### A.0 Keywords (17)

```
use     spawn   ref     mut
if      else    match   while   for
in      return  break   continue
as      unsafe  and     or
```

`type` is no longer a keyword (RFC-010; use `Name: Type = { ... }` instead); `pub` was removed as a
whole under RFC-029g and is now an ordinary identifier. See §1.4.1 for the literal reserved words
`true` / `false` / `void`, and §1.4.3 for the meta type name `Type` and the built-in type names
`Void` / `Never` / `Int` / `Float` / `Bool` / `Char` / `String`.

### A.1 Control Flow

```
if Expr Block (else if Expr Block)* (else Block)?
match Expr { MatchArm+ }
while Expr Block
for 'mut'? Identifier 'in' Expr Block
break | continue          // Only inside loop bodies (§3.4 / §3.5)
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
