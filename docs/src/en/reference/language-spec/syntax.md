# Syntax Specification

This document defines the syntax specification of the YaoXiang programming language, including
lexical structure, syntax rules, and operator precedence.

---

## Chapter 1: Lexical Structure

### 1.1 Source Files

YaoXiang source files must use UTF-8 encoding. Source files generally use the `.yx` extension.

### 1.2 Lexical Token Categories

| Category   | Description                                | Examples                  |
| ---------- | ------------------------------------------ | ------------------------- |
| Identifier | Starts with a letter or underscore         | `x`, `_private`, `my_var` |
| Keyword    | A reserved word predefined by the language | `use`, `mut`, `and`       |
| Literal    | A fixed value                              | `42`, `"hello"`, `true`   |
| Operator   | An operational symbol                      | `+`, `-`, `*`, `/`        |
| Delimiter  | A syntactic delimiter                      | `(`, `)`, `{`, `}`, `,`   |

### 1.3 Keywords

YaoXiang has **17 keywords** (`keyword_from_str` in `src/frontend/core/lexer/state.rs`, with each
one corresponding to a `TokenKind`):

```
use     spawn   ref     mut
if      else    match   while   for
in      return  break   continue
as      unsafe  and     or
```

`and` / `or` are **keywords** for logical AND / OR (Zig-style; `and` has higher precedence than
`or`, see levels 10–11 in §2.2). Unary negation is the symbol `!` and is orthogonal to them.

These keywords have special meanings in every context and cannot be used as identifiers.

> **`type` is no longer a keyword** (RFC-010): use the `Name: Type = { ... }` notation to define a
> type. Both `keyword_from_str` and the header of the `TokenKind` enum
> (`src/frontend/core/lexer/tokens.rs`) are marked “RFC-010: 'type' keyword removed”.
>
> **`pub` is no longer a keyword** (RFC-029g, accepted on 2026-10-02): the language has no
> visibility mechanism. `pub` has been removed entirely from the lexer, AST, type checker, dead-code
> exemptions, formatter, and LSP ([RFC-029g](../../rfc/accepted/029g-remove-pub-and-auto-bind.md)).
> It is now merely an ordinary identifier—the old form `pub x = 1` reports undefined name E1001. All
> top-level bindings are importable by default, and modifiers make no difference to visibility
> ([RFC-029](../../rfc/accepted/029-module-semantics.md)).

### 1.4 Reserved Words

YaoXiang's “reserved words” are divided into three layers, recognized by the parser and type checker
at different stages:

#### 1.4.1 Literal Reserved Words

The parser has separate tokens for the following literal identifiers, which cannot be used as
ordinary identifiers:

| Identifier | Type | Description                                                                                                       |
| ---------- | ---- | ----------------------------------------------------------------------------------------------------------------- |
| `true`     | Bool | The Boolean true value                                                                                            |
| `false`    | Bool | The Boolean false value                                                                                           |
| `void`     | Void | The Void literal (Unit value). Lowercase `void` is a value literal; uppercase `Void` is a type name (see §1.4.3). |

> **`Type` is not at this layer**: it has **no** separate `TokenKind` (there is no `Type` in the
> `TokenKind` enum in `src/frontend/core/lexer/tokens.rs`, and `keyword_from_str` has no
> corresponding branch). The parser treats it as an ordinary identifier, while the type checker
> recognizes it as a meta type in type positions. Consequently, `Type` may be shadowed by a local
> binding in expression positions. It is a **meta type name**, not a keyword.

#### 1.4.2 Variant Names

Variant names are recognized by the parser in **pattern** contexts (bare names are adjudicated
according to the scrutinee type's variant set; see §2.8). In **expression** contexts, variants must
be type-qualified: `Result(Int, String).ok(5)`, `Option(T).some(v)`, or `Color.green()` (RFC-010
record types and types: variant constructors are function-typed fields in type definitions, so a
bare `ok(5)` is not a constructor call).

| Variant Name | Owning Type | Description                      |
| ------------ | ----------- | -------------------------------- |
| `some(T)`    | Option      | Option value variant constructor |
| `ok(T)`      | Result      | Result success variant           |
| `err(E)`     | Result      | Result error variant             |

#### 1.4.3 Built-in Type Names

The following type names are pre-registered by the type checker and can be used in type positions
without being imported. The parser treats them as ordinary identifiers—**they are not reserved words
and may be shadowed by local bindings (not recommended)**.

| Type Name | Logical Correspondence | Description                                                                                                                                           |
| --------- | ---------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Void`    | ⊤ (true/Unit)          | A zero-field product type with exactly one inhabitant (the `void` literal; see §1.4.1)                                                                |
| `Never`   | ⊥ (false/empty type)   | A type with zero variants and zero inhabitants. No expression can produce a `Never` value. `Never <: T` holds for every `T` (principle of explosion). |
| `Int`     | —                      | Signed integer                                                                                                                                        |
| `Float`   | —                      | Floating-point number                                                                                                                                 |
| `Bool`    | —                      | Boolean value: `true` / `false`                                                                                                                       |
| `Char`    | —                      | Unicode character                                                                                                                                     |
| `String`  | —                      | String                                                                                                                                                |

### 1.5 Identifiers

Identifiers begin with a letter or underscore; subsequent characters may be letters, digits, or
underscores. Identifiers are case-sensitive.

Special identifiers:

- `_` is used as a placeholder, indicating that a value is to be ignored
- Identifiers beginning with an underscore denote private members

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

> **A dictionary literal requires at least one key-value pair**: `{}` is **not** an empty
> dictionary—it is an empty block (the value `Void`; see [§2.9](#_2-9-块表达式)). Use the
> constructor `dict.new()` for an empty dictionary:
>
> ```yaoxiang
> empty = dict.new()                  // ✅ Empty dictionary
> d = { "a": 1 }                       // ✅ Dictionary literal
> wrong = {}                           // ❌ Not a dictionary; an empty block (Void)
> ```
>
> **The criterion is the content**: the `Dict` grammar requires at least one `String ':' Expr`.
> Since `{}` has no content on which to base a decision, it takes the zero form of the block
> structure. A non-empty form is self-describing (`{ "k": v }` has a key-value pair →
> dictionary)—the same principle as `f = { 5 }` being an `Int` value rather than a function: **the
> content determines the type**.

> Set has no literal grammar and no runtime representation—when a set type is planned, its
> requirements will be filled in following the Dict pattern (std.set + HeapValue::Set). The
> resolution of List/Dict literals is determined by the contextual type annotation: a bare literal
> and a `List(T)` annotation both resolve to a growable list. The type face of a growable list has
> the single name `Vec(T)`; a `List(T)` annotation is normalized by source name and resolves to the
> same type face as `Vec(T)`. An `Array(T, N)` annotation applied directly to a literal resolves to
> a fixed-length array. Implicit List→Array conversions are prohibited.
>
> Array literal semantics:
>
> - The number of elements must equal N; otherwise, compilation fails with E1002. An empty literal
>   paired with a nonzero N is likewise rejected
> - Every element type must be compatible with T; otherwise, compilation fails with E1002
> - Grammar forms for N: an integer literal (possibly negative), a constant name, or a compile-time
>   constant expression beginning with a reference to a value parameter and continued with infix
>   arithmetic (such as `2+3` or `N+1`), participating in count validation by its folded value
>   (`2+3` and `5` have the same form)
> - Parser rejection: floating-point/string seeds (E0010) and incomplete expressions (such as `2+`,
>   E0012); an identifier followed by `(` takes the constructor type path—function calls cannot be
>   used as type-level arguments (E1003)
> - When N is a symbolic constant (such as a function const parameter in `Array(Int, n)`), count
>   validation is deferred to the refinement type stage
> - Nested array literals are rejected at compile time in v1
>   (`Array(Array(Int,2),2) = [[1,2],[3,4]]`); each layer must be constructed explicitly. Recursive
>   resolution is deferred to a later version

#### 1.6.5 List Comprehensions

```
ListComp    ::= '[' Expr ( 'for' Identifier 'in' Expr ( 'if' Expr )? )+ ']'
```

> **#401 (implementation completion)**: `if` filters and multiple generator clauses are implemented
> as of this version. The previous grammar was `(',' Expr)* ('if' Expr)?`, for which there was no
> implementation (neither comma-appended expressions nor multiple generators were supported), and
> the parser omitted the `if` filter entirely (after parsing the iterable, it expected `']'` and
> reported `E0010 Expected RBracket, found KwIf`). The semantics now follow common industry
> practice: one or more generator clauses, expanded as nested loops; each clause may carry at most
> one `if` filter, whose condition must be a `Bool` (otherwise E1054 is reported); the iterable or
> condition of a later clause may refer to iteration variables bound by earlier clauses.

> **Behavior tightening (migration record)**: the iteration-variable grammar has always been
> `'for' Identifier 'in'`, but the old implementation parsed the pattern using the full Pratt
> parser—after `'in'` was registered as an infix operator, `x` would swallow `in items` as a
> membership expression. After the fix, a non-identifier pattern fails to parse directly and no
> longer falls back to `_` (silently swallowing the error). Impact: forms such as
> `[x for (a, b) in pairs]` that previously parsed now produce an error. This form never had defined
> behavior (the variable was always `_`), so the tightening is correct and incurs no semantic
> migration cost.

#### 1.6.6 Membership Tests

```
Membership  ::= Expr 'in' Expr
```

> `in` is a binary relational operator that returns `Bool`—`true` on a match and `false` otherwise,
> without reporting an error. The semantic split is: `[]` asserts existence and retrieves a value
> (failure reports an error), whereas `in` asks whether something exists (a miss is a normal
> `false`). Right-operand coverage: List / Array / Dict (key set) / Tuple / String (substring) /
> Range (interval). `in` is a first-class total predicate and serves as the basis for compile-time
> provable propositions during the refinement type stage. (Set is excluded from the list of right
> operands—Set has no runtime representation; see §1.6.4.)

### 1.7 Comments

```
// Single-line comment

/* Multi-line comment
   Can span multiple lines */
```

### 1.8 Indentation Rules

Code must use four-space indentation. Tab characters are prohibited. This is a mandatory syntax
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
| 10         | `and`                       | Left to right |
| 11         | `or`                        | Left to right |
| 12         | `if...else`                 | Right to left |
| 13         | `=` `+=` `-=` `*=` `/=`     | Right to left |

> **Unary prefix operators** (`!` `-` `+`) bind tightly: they bind only less tightly than calls and
> member access, and more tightly than every binary operator. Therefore, `!a == b` ≡ `(!a) == b`
> (Zig-style semantics). `!` is a purely unary operation, does not participate in short-circuit
> control flow, and is orthogonal to the `and`/`or` keywords (which short-circuit; RFC-010 is the
> authoritative definition).

> **Range binding strength**: `..` has binding strength (6, 7)—the left value 6 is lower than
> addition (7), while the right value 7 consumes addition but not a same-level `..`. Before and
> after the change:
>
> | Expression   | Before the Change (level 1, right-associative)                                                                                   | After the Change ((6,7), left-associative)                                               |
> | ------------ | -------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
> | `x in 1..10` | `x in 1..10` (the right operand of `in` is at level 4, while `..` at level 1 cannot consume it, so this was actually unparsable) | `x in (1..10)`—the entire interval is the right operand of `in`                          |
> | `0..n+2`     | `(0..n)+2` (right-associativity trap: the upper bound is swallowed, causing the `for` loop to report E3004 directly)             | `0..(n+2)`—the upper bound is an arithmetic expression                                   |
> | `a == b..c`  | `a == (b..c)` (`..` at level 1 < `==` at level 3, so it is naturally grouped as a whole)                                         | `a == (b..c)`—**semantics are unchanged**; `..` still binds more tightly than comparison |
> | `1..2*3`     | `(1..2)*3`                                                                                                                       | `1..(2*3)`—the upper bound is an arithmetic expression                                   |
> | `a..b..c`    | `a..(b..c)` (right-associative chain, resulting in meaningless nested Ranges)                                                    | `(a..b)..c`—**step form** (`c` is the step)                                              |
>
> Net effect: the compound upper bound in `for i in 0..n+2` changes from “parses successfully but
> reports E3004” to “directly usable”; `x in 1..10` changes from “unparsable” to “interval check”;
> and `a..b..c` changes from “meaningless nesting” to “step components”. Level 6 falls between `+`
> (level 5) and `<<` (level 7), following mathematical convention: an interval is a tightly bound
> construct, so its upper bound is naturally a complete arithmetic expression.

### 2.3 Function Calls

```
FnCall      ::= Expr '(' ArgList? ')'
ArgList     ::= Expr (',' Expr)* (',' NamedArg)* | NamedArg (',' NamedArg)*
NamedArg    ::= Identifier '=' Expr
```

Named arguments use `name = value` (RFC-010 §Function Definitions, RFC-011 §Construction Forms).
Positional arguments must precede named arguments; named arguments for positionally specified
parameters may appear in any order:

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b

add(3, 5)          // Positional
add(a = 3, b = 5)  // Named
add(b = 5, a = 3)  // Any order
add(3, b = 5)      // Mixed, with positional arguments first
```

An incorrect argument name reports **E1014**, specifying the same formal parameter both positionally
and by name reports **E1015**, and an incorrect argument count reports **E1010** (RFC-013).

**Unnamed parameters and named arguments**: signature parameter positions allow a bare type form
(`mk: (Int, Int) -> Int`). This is an **unnamed typed parameter**—the type constraint applies
positionally, while the name belongs to the implementation (provided by the lambda header; see the
shorthand rules in [RFC-007](../../rfc/accepted/007-function-syntax-unification.md)) and does not
enter the contract. An unnamed parameter cannot be referenced by a named argument (there is no name
to reference) and may only be passed positionally. To place a name in the contract, write
`mk: (a: Int, b: Int) -> Int`. A bare identifier must be a declared type; if the type cannot be
resolved, parsing reports an error (see [type-system §3.5](type-system.md#35-函数类型)).

**Named parameters in reverse: the name enters the contract, and the declaration is authoritative**.
Parameter names in an implementation header (the parameter list after `=>`) are merely local names
within the body and are bound **positionally**—for `f: (a: Int) -> Int = (b) => b`, the
implementation header uses `b`, but callers still write `f(a = 7)`. Type annotations in the
implementation header are used only for comparison; a mismatch with the declaration reports
**E1011**. If the implementation header has more parameters than the declaration's runtime parameter
positions, **E1112** is reported (`f: () -> Int = (a: Int) => a`)—in both cases, the declaration is
the authoritative source. The reverse case, where the implementation omits parameters, does not
report an error: the type layer checks against the declaration's arity, but a position without a
corresponding implementation-header parameter has no runtime slot, and arguments placed there are
discarded on the IR side (see the gap recorded in [type-system §3.5](type-system.md#35-函数类型)).

### 2.4 Member Access

```
MemberAccess::= Expr '.' Identifier
```

### 2.5 Index Access

```
IndexAccess ::= Expr '[' Expr ']'
```

> **Three-layer semantics (RFC-011b)**: `a[i]` is dispatched according to the type of `a`: ①
> built-in containers (List/Vec/Array/Dict/Tuple) use native index instructions (the fast path); ②
> user types that implement the `Index` interface are dispatched to their `index` method
> (instantiating `Index(Grid, Int, Float)` in the type body and adding a `Grid.index` method); ③
> RFC-004's `f[0]` positional binding exists only in binding declarations and does not follow this
> grammar. For multidimensional indexing, the keys in `a[0, 1]` are packed into a tuple. Other types
> that do not implement `Index` are rejected at the type layer.

### 2.6 Type Casting

```
TypeCast    ::= Expr 'as' TypeExpr
```

> **Implementation status (pre-existing gap, not introduced in this version)**: the preceding text
> defines only the grammar and precedence (§2.4, level 2); the semantics have not yet been
> implemented. Independently verifiable facts: ① Parsing: the infix arm for `KwAs` in
> `parser/pratt/led.rs:81` produces `ast::Expr::Cast`; ② Type checking:
> `typecheck/inference/expressions.rs:3847-3853` performs only a **silent reannotation** for
> `Expr::Cast`—`let _ = self.infer_expr(expr)?;` discards the inner result and then directly returns
> the `target_type` `MonoType`; ③ The repository has **no** convertibility validation and no
> corresponding diagnostic code (`util/diagnostic/codes/` has no cast entry); ④ `ir_gen` generates
> no instructions (`middle/core/ir_gen.rs` only obtains the span and free variables of `Expr::Cast`
> at `:5111` / `:5918` / `:9381`—the first two are the span arms of `get_expr_span` / `expr_span`,
> and the third is `collect_free_vars_expr`; none of the three generates instructions); ⑤
> Consequently, `x as Int` currently neither reports an error nor produces any runtime conversion.

### 2.7 Conditional Expressions

```
IfExpr      ::= 'if' Expr Block ('else' 'if' Expr Block)* ('else' Block)?
```

### 2.8 Pattern Matching

```
MatchExpr    ::= 'match' Expr '{' MatchArm (ArmSeparator MatchArm)* '}'
MatchArm     ::= Pattern ('|' Pattern)* ('if' Expr)? '->' Expr
ArmSeparator ::= ',' | <newline>
Pattern     ::= Literal
              | Identifier
              | Wildcard
              | RangePattern
              | StructPattern
              | TuplePattern
              | EnumPattern
              | OrPattern
RangePattern ::= ('-')? Literal '..' ('-')? Literal   // Endpoints may have a negative prefix, and negative values are normalized—see the “Range Patterns” note below
```

> **Branch arrows and separators**: branches use `->`; `=>` is used only for lambdas. Branches are
> separated by a newline or `,`—the same rule as “newline or `;`” for statements
> ([RFC-038](../../rfc/accepted/038-statement-termination.md)).

> **Variant-set requirement for variant destructuring** (RFC-010b): `EnumPattern` (such as `ok(v)`
> or `some(x)`, with variant names adjudicated by the scrutinee) requires the scrutinee type to be a
> **sum type with a present variant set**. Variant sets enter the checker only through a type
> definition or a `use` import—`Result`/`Option` are defined by `std.result`/`std.option` and
> require `use std.result` / `use std.option` before use (whole-module and grouped `use std.{...}`
> forms are equivalent); otherwise, variant destructuring reports E1002. Exhaustiveness checking
> uses the same source: a fallback arm is exempt from exhaustiveness, and without one, all variants
> are checked.

> **Pattern bindings, or-patterns, and guards** (RFC-010b):
>
> - **Bindings**: a bare identifier in a pattern introduces a **new name**—it is neither a
>   comparison nor an assignment. The name is visible **within the arm**, shadows an outer variable
>   of the same name, and ceases to exist after the arm. Discriminants (such as `ok` in `ok(v)`) are
>   adjudicated using the scrutinee's variant set; a bare name that does not match is a binding.
> - **Or-patterns**: `p1 | p2` tries alternatives in order; if a non-final alternative fails, the
>   next is tried, and if the final alternative fails, control falls to the next arm. All
>   alternatives must bind **the same set of names**, or E1033 is reported. A binding with the same
>   name exists only once at arm level, and the arm body necessarily reads the value written by the
>   matching alternative.
> - **Guards**: after a pattern matches, the `if` condition is evaluated, with pattern bindings
>   visible to the condition. The condition must produce a `Bool`; if false, control falls to the
>   next arm. A guarded arm **does not count toward exhaustive coverage** (the guard may not match),
>   but a guarded duplicate-variant arm is reachable (it is a refinement of the preceding arm).
> - **Diagnostics**: missing branch E1030, unreachable branch E1031, duplicate binding within the
>   same pattern E1032, inconsistent or-pattern bindings E1033, and missing fields in a struct
>   pattern E1034; see the [error code reference](../error-code/E1xxx.md) for the code table and
>   messages.
>
> **Range patterns** (issue #405): `lo..hi` is a **closed-interval test**—over the Int domain,
> `x matches lo..hi ⟺ lo <= x <= hi`, so both 80 and 89 match `80..89`. Endpoints must be **integer
> literals** (a negative prefix is normalized to a negative value, such as `-5..-1`); all other
> literal types (Float/Bool/String/Char) are syntactically valid but rejected by the type checker
> (E1002; the first version does not support Float ranges); identifier endpoints in pattern
> positions are binding names with no test semantics and are rejected by the parser (E0010). When
> both endpoints are decidable at compile time, a reversed range (`lo > hi`) is empty, never
> matches, and reports unreachable (E1031); duplicate range arms with the same endpoints likewise
> report E1031.
>
> Exhaustiveness interaction: range patterns are in the same category as literal patterns—**checked,
> partially covering, and non-fallback**. Because the Int domain is open, a range arm cannot be
> exhaustive, so a match containing a range arm still requires a wildcard/identifier arm as a
> fallback (E1030); range arms may be used as or-pattern alternatives and may have guards (a failed
> guard falls to the next arm, consistent with the existing guard rules).
>
> **Semantic separation from the §2.12 Range expression**: `lo..hi` in a pattern position does not
> construct a Range value; it only performs branch selection, with closed-interval endpoint
> semantics. In expression positions, `a..b` (for `for` iteration, `in` interval checks, and Range
> as a first-class value) remains half-open, left-closed and right-open. The separation of the two
> `..` semantics is intentional: a pattern position is a predicate over a discrete value domain,
> whereas an expression position is notation for constructing an iteration sequence.

### 2.9 Block Expressions

```
Block       ::= '{' Stmt* Expr? '}'
```

> **Statement termination rules**: separation between statements and newline behavior (explicit `;`
> separation, newline termination, continuation-line exceptions, and never merging when a line
> starts with `(`/`[`) are defined by [RFC-038](../../rfc/accepted/038-statement-termination.md);
> **adjacent statements without a separator are a syntax error**. The same rule (newline or explicit
> separator) applies to match branches (see §2.8).

#### 2.9.1 The Three Forms of `{`

In expression positions, `{` has exactly three interpretations, determined once by its **content**:

| Form                   | Notation          | Type                 | Example          |
| ---------------------- | ----------------- | -------------------- | ---------------- |
| **Empty block**        | `{}`              | `Void`               | `x: Void = {}`   |
| **Dictionary literal** | `{ "k": v, ... }` | `Dict(K, V)`         | `d = { "a": 1 }` |
| **Block**              | `{ Stmt* Expr? }` | Tail expression type | `y = { 1 + 1 }`  |

**Decision order**:

1. `{}` (no content) → **empty block**, value `Void`
2. The first element is a `String ':' Expr` key-value pair → **dictionary literal**
3. Otherwise → **block**, with the value provided by the tail expression

> **Why `{}` is not an empty dictionary**: the “emptiness” of an empty dictionary cannot be
> self-describing (it could be either `Dict(K, V)` or an empty block), while the dictionary grammar
> [§1.6.4](#_1-6-4-集合) requires at least one key-value pair. When there is no content on which to
> base a decision, the zero form of the block structure is used: this is consistent with `unsafe {}`
> / `spawn {}` and introduces no special case. Use `dict.new()` for an empty dictionary.
>
> **Why functions require an annotation**: `f = { stmt }` is a **value** (with the tail expression's
> type), not a function. To define a function, state the Fn annotation: `f: () -> Int = { 5 }`. This
> follows the same principle as dictionaries: **the content determines the type**, not whether an
> annotation exists. (See [RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md),
> Appendix D.)

**Unified semantics**: the value of every `{}` block is provided by its **tail expression**, and
`return` is a `Never`-typed nonlocal exit.

| Block Type    | Value Exit      | Empty Block `{}` |
| ------------- | --------------- | ---------------- |
| Ordinary `{}` | Tail expression | `Void`           |
| `unsafe {}`   | Tail expression | `Void`           |
| `spawn {}`    | Tail expression | `Void`           |

**Core principles** (see [RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md)):

- **A block's value = its tail expression** (the final expression), the sole exit, with no
  exceptions
- When the final statement is an **assignment statement**, the block's value is `Void`; write `Void`
  explicitly if that is the desired value
- **`return` exits the nearest function boundary** (passing through every block, rather than
  “returning to the block”), and has type `Never`; `Never <: T`

> holds for every type (principle of explosion), so it may appear in any return-type position. The
> module initialization layer (the Script top level) has no function boundary, so `return` is
> rejected at compile time there (`E1109`; see §3.11)

- The expression form `= expr` directly provides the value

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
        return 0     // Passes out of if and the function body, exiting the function
    }
    n * 2            // Tail expression
}
```

#### Is `name = { ... }` a Function or a Block Value?

`name = { ... }` may be either a function definition (RFC-007's “empty-parameter shorthand”) or a
block-value binding. The decision is made by **annotation priority, with function as the default**
(RFC-010a Appendix D):

| Case                                  | Result          | Example                            |
| ------------------------------------- | --------------- | ---------------------------------- |
| The value is a Lambda (`=>`)          | Function        | `f = () => 5` → `f()` = 5          |
| The annotation is a function type     | Function        | `f: () -> Int = { 5 }` → `f()` = 5 |
| The annotation is a non-function type | **Block value** | `x: Int = { 5 }` → `x` = 5         |
| No annotation                         | Function        | `f = { 5 }` → `f()` = 5            |

**The annotation is the type**: `x: Int = ...` declares `x` to be an `Int`, so `{ ... }` evaluates
to an `Int`; `f: () -> Int = ...` declares `f` to be a function, so `{ ... }` is the function body.

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

#### Nested Function Types: Curried or Returning a Function?

There are two interpretations for a nested function type on the right side of `->`, distinguished by
**parentheses** (RFC-004):

| Notation                        | Meaning                | Invocation               |
| ------------------------------- | ---------------------- | ------------------------ |
| `(a: Int) -> (b: Int) -> Int`   | **Curried**            | `f(1)(2)`                |
| `(a: Int) -> ((b: Int) -> Int)` | **Returns a function** | `g(1)` yields a function |

The basis is: **the annotation is the type**. `g: (a: Int) -> ((b: Int) -> Int)` declares that
`g(1) : (b: Int) -> Int`, so `g(1)` must **be** that function, rather than “the next set of
arguments”.

```yaoxiang
// Curried: the two sets of arguments are supplied in successive layers
add: (a: Int) -> (b: Int) -> Int = { a + b }
add(1)(2)        // → 3

// Returns a function: the outer call takes one argument and returns the function
adder: (n: Int) -> ((x: Int) -> Int) = (x) => x + n
adder(10)(5)     // → 15
h = adder(10)    // The returned function can be stored in a variable and passed around
```

An unparenthesized nested `Fn` is always curried (including RFC-011's type-parameter form):

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
            |  Identifier '=>' Expr

裸单参形态（`Identifier '=>' Expr`）只允许**单个标识符、不带类型注解**；多参或带注解一律用括号式。
```

The bare single-parameter form (`Identifier '=>' Expr`) permits **only a single identifier without a
type annotation**; multiple parameters or parameters with annotations must always use the
parenthesized form.

Parameters in a lambda header must be **named**: the name is the only way to refer to the parameter
within the body. A bare type item belongs only to the signature position, while the name is provided
by the lambda header—see [type-system §3.5](type-system.md#35-函数类型) for the two signature
parameter forms and the ownership of names.

### 2.11 Error Propagation Operator

```
ErrorPropagate ::= Expr '?'
```

The `?` operator is postfix and has the same precedence as `.`. For the `Result(T, E)` type:

- On `Ok(v)`, extract the value `v` and continue execution
- On `Err(e)`, propagate the error upward (`return Err(e)`)

```yaoxiang
process: (data: Data) -> Result(Data, Error) = {
    validated = validate(data)?     // Extract the value on success; propagate on failure
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

// Step form (the third component, default 1)
for i in 0..10..2 { print(i) }  // 0, 2, 4, 6, 8
for i in 10..0..(-2) { print(i) }  // 10, 8, 6, 4, 2
```

> **Step semantics**: in `a..b..c`, `c` is the step. The literal `c = 0` is rejected at compile
> time; a dynamic `c` undergoes a runtime zero check (E6001 family; promoted to Result once the
> error system is implemented). `c < 0` is valid, and the interval direction reverses with the sign
> (`10..0..(-2)` descends).
>
> **Separation in pattern positions**: the match range pattern `lo..hi` (§2.8) does not construct a
> Range value; its endpoint semantics are closed-interval selection. The expression-position `a..b`
> in this section remains left-closed and right-open.

### 2.13 ref Expressions

```
RefExpr     ::= 'ref' Expr
```

`ref` creates shared ownership. The compiler automatically selects Rc (single-task) or Arc
(cross-task), and users need not concern themselves with implementation details.

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

- Types and raw-pointer operations may be defined inside `unsafe {}`
- The returned type is available outside `unsafe {}`
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
- An inner scope may access variables from an outer scope
- An outer scope cannot access variables from an inner scope
- Variable declarations follow the “assignment-first” principle

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

- `x = value`: searches outward through the scope chain for x; if found, assigns to it; otherwise,
  declares it anew
- `mut x = value`: explicitly declares a new mutable binding and forbids the same name in an outer
  scope
- Any name may be declared only once within the same scope

> **Detailed definition**: see the [Module System Specification](modules.md#第四章作用域) for the
> complete scope rules, variable declarations, and shadowing mechanism.

---

## Chapter 3: Statements

### 3.1 Statement Categories

```
StmtList    ::= Stmt (StmtSep Stmt)*
StmtSep     ::= ';' | <newline>
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

The right-hand value's elements are bound to the patterns positionally; a bare identifier in a
pattern introduces a **new name** (following the same rule as match patterns), rather than
performing a comparison or assigning to an existing name. The right-hand value's arity must equal
that of the destructuring pattern.

```yaoxiang
(a, b) = (1, 2)
```

### 3.3 return Statements

```
ReturnStmt  ::= 'return' Expr?
```

**Semantics**: `return` is a **nonlocal exit** that exits the nearest **function boundary** (passing
through every block—including `if` / `while` / `for` / `match` / bare blocks / `spawn` / `unsafe`)
and hands its value to the caller. It does **not** “return to the block”.

**Type**: `return e : Never` (`e : T`). `Never <: T'` holds for every `T'` (principle of explosion;
see [Type System §2.2](type-system.md)), so `return` may appear in any return-type position without
requiring an additional constraint.

**Relationship to block evaluation**: a block's value is always its **tail expression** (see §2.9).
As a block, `{ return n }` has the value `n` and the type `Never`; at the same time, `return` exits
the function. **Both facts hold simultaneously**, coexisting through the principle of explosion.

`return` and tail expressions together make “early return” possible, without requiring a special
rule for `return` targeting a function—see
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

**Semantics**: immediately terminates the innermost enclosing `while`/`for` loop, transferring
control to after that loop's body.

- **Exits only the nearest level**: `break` always applies to the innermost enclosing loop. If
  multiple nested-loop levels must be exited, extract the inner loop into a function and use
  `return`, or use a flag (break/continue have no labels; if loop labels are introduced in the
  future, their loop-declaration-side syntax will follow the RFC process and be adjudicated together
  with the multi-exit design of the proof pipeline)
- **Permitted only inside a loop**: `break` may appear only in the body of a `while`/`for` loop
  (including nested blocks/if/match statements in that body); use outside a loop reports a compile
  error (E1102 `'break' outside of a loop`)
- **Does not affect termination proofs**: `break` does not participate in termination reasoning—it
  provides neither a measure nor a decreasing step in one. The loop's termination obligation is
  independent of `break` and is triggered by the refinement type system (see
  [type-system §8.5](type-system.md#85-terminates终止测度谓词)).
- **Borrow semantics**: the control-flow edge for break participates in the structural cut performed
  by RFC-009a's reverse BFS liveness analysis (the exited iteration does not participate in
  back-edge liveness inference)

```yaoxiang
mut i = 0
while i < 10 {
    i = i + 1
    if i == 3 {
        break              // Control transfers to after the loop, where i == 3
    }
}

// Nested loops: break exits only the inner loop
while j < 3 {
    while k < 10 {
        if k == 2 { break }    // Terminates only the inner loop
    }
    j = j + 1                  // Executed after every outer iteration
}
```

### 3.5 continue Statements

```
ContinueStmt::= 'continue'
```

**Semantics**: skips the remaining statements in the current iteration and proceeds directly to the
next iteration of the innermost enclosing loop—`while` returns to condition evaluation, while `for`
takes the next element.

- **Applies only to the nearest level**: as with `break`, it has no label
- **Permitted only inside a loop**: use outside a loop reports a compile error (E1102)

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

YaoXiang's for-loop semantics differ from those of traditional languages: **each iteration binds a
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
| End       | The body ends, and the binding is destroyed                                                        |

**Key point**: after each iteration ends, the binding created in that iteration is destroyed. The
next iteration uses an entirely new binding, unrelated to the previous iteration's binding.

#### 3.9.2 Difference between for and for mut

| Syntax              | Loop Variable Mutability | Description                                         |
| ------------------- | ------------------------ | --------------------------------------------------- |
| `for i in 1..5`     | Immutable                | The binding cannot be modified within the loop body |
| `for mut i in 1..5` | Mutable                  | The binding may be modified within the loop body    |

```yaoxiang
// Valid: each iteration binds a new value, so no modification is needed
for i in 1..5 {
    print(i)  // Reads the value of i
}

// Invalid: an immutable binding cannot be modified
for i in 1..5 {
    i = i + 1  // Error: cannot modify an immutable binding
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
| YaoXiang | Binds a new value on every iteration                         |
| Rust     | Modifies the same variable (requires mut)                    |
| Python   | Modifies the same variable (does not require mut)            |
| C/C++    | Modifies the same variable (requires a pointer or reference) |

**Design rationale**: YaoXiang adopts binding semantics because:

1. **It better matches natural semantics.** In natural language, “for each element x in a set” means
   that each x is an independent individual. YaoXiang's `for i in 1..5` reads as “for each i from 1
   through 5”, with each iteration's i being an entirely new binding. This is consistent with
   intuitive human understanding.

2. **It avoids accidental modification.** Immutable binding semantics by default mean that the loop
   variable cannot be accidentally modified within the loop body. There is no need to worry about
   accidentally writing `i = ...` somewhere in a complex loop body and creating a difficult-to-trace
   bug.

3. **High-performance solutions remain readily accessible.** When reuse of a variable across
   iterations is genuinely needed (for example, an accumulator or cache), use a `for mut`
   declaration to switch to mutable binding mode. This is clearer than implicit shared state—the
   intent is expressed explicitly in the syntax rather than hidden in runtime behavior.

### 3.10 spawn Statements

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' 'mut'? Identifier 'in' Expr '{' Expr '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
```

**spawn block**: explicitly declares a concurrency region in which the expressions execute
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

- A block-body reference to an outer variable is a **Move value capture**: the value is snapshotted
  into the closure environment at the spawn creation point, and the body reads it through the
  environment (LoadUpvalue)
- **Primitives** (Int/Float/Bool/Char) are copied, and the outer variable is unaffected
- **Handle types** (Struct/String/List, etc.) are snapshotted as handle copies that share the
  underlying object; under the Embedded runtime (the default), the heap is shared within the same
  thread and the handle remains valid
- Sharing between multiple tasks requires an explicit `ref` (§2.13; the compiler automatically
  selects Rc/Arc)
- A `return` within the block also captures referenced outer variables

```yaoxiang
t1 = 1 + 1
t2 = 2 + 2
result = spawn {
    return t1 + t2    // t1/t2 are captured by value; result == 6
}
```

---

### 3.11 Program Entry Point and Top-Level Statements

A source file has two roles from the compiler's perspective, **determined by whether `yaoxiang.toml`
exists**:

| Role       | Determination                                       | Program Body                                                                       |
| ---------- | --------------------------------------------------- | ---------------------------------------------------------------------------------- |
| **Script** | A single file run directly, with no `yaoxiang.toml` | **Top-level statements** (executed in source order); `main` is an ordinary binding |
| **Bin**    | `yaoxiang.toml` exists                              | **The `main` function**; executable statements are not permitted at the top level  |

#### Script: Top-Level Statements Are the Program

Without a manifest, the file is a “script,” and **top-level statements execute in source order**
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

main()                       // ← This call is required
```

> **Why is `main` not called automatically?** The top-level statements are already the program body.
> If `main` were also invoked implicitly, a script containing an explicit `main()` would execute
> twice. The two rules cannot coexist, so Script mode has only one execution entry point: the
> top-level statements.

The top level must likewise **not use `return`**: the Script top level is the module initialization
layer and has no function boundary to exit (RFC-010a rule ②). Compilation reports `E1109`—rather
than silently terminating initialization at runtime and skipping the remaining top-level statements.
To terminate a section of top-level logic early, place it in a function and control execution at the
call site.

#### Bin: `main` Is the Entry Point

When a manifest exists, the file is an “executable target,” and:

- A binding named `main` **must** be defined, with either a value or a function (the decision
  finalized in #388: entry-point detection is based on the binding's existence—a function is already
  a value, and the two merely use different evaluation strategies): a function-valued `main` is
  called with no arguments at entry; a value-valued `main` is evaluated during initialization
- Executable statements are not permitted at the top level—the program body is `main`

```yaoxiang
main: () -> Void = {
    print("hello")
}
```

A value-valued `main` is equally valid—it is evaluated during initialization, within the
initialization sequence (following the global binding topological order, with independent bindings
ordered by source order):

```yaoxiang
main = {
    print("hello")
}
```

A missing `main` is a compile error (without `main`, all functions are unreachable). A non-callable
value-valued `main` (such as `main: Int = 5`) is valid but has no observable effect—the counterpart
to Rust's empty `fn main() {}`.

> **Library files**: files imported by other files through `use`, or files referenced by
> `[lib].path` / `[exports]`, do not require `main`—they are not program entry points.

#### Top-Level Binding Initialization

Top-level binding initializer values are **evaluated at runtime** and are not required to be
compile-time constants:

```yaoxiang
answer: Int = { 42 }              // Block value
inc: (Int) -> Int = (x) => x + 1
computed: Int = inc(41)           // Function call
```

Initialization executes in **dependency order**, independently of source order (dependencies
initialize first; independent bindings follow declaration order):

```yaoxiang
derived: Int = base * 3           // References base, which is declared later
base: Int = 7                     // Initializes first (topological sort)
```

Furthermore, **all binding initialization precedes all top-level statement execution**—side effects
in binding initializers occur before any top-level statement, regardless of how the two are
interleaved in the source:

```yaoxiang
print("a")                        // Executes later
x = { print("b") }                // Executes first (binding initialization as a whole takes precedence)
print("c")                        // Output order: b, a, c
```

A cyclic dependency is a compile error (the names on the cycle are listed):

```yaoxiang
a: Int = b + 1
b: Int = a + 1                    // Error: a → b → a
```

> **Design basis**: RFC-029f (file-role model), RFC-010a Appendix D (block-binding adjudication).

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
entirely under RFC-029g and reverted to an ordinary identifier. The literal reserved words `true` /
`false` / `void` are described in §1.4.1. The meta type name `Type` and built-in type names `Void` /
`Never` / `Int` / `Float` / `Bool` / `Char` / `String` are described in §1.4.3.

### A.1 Control Flow

```
if Expr Block (else if Expr Block)* (else Block)?
match Expr { MatchArm+ }
while Expr Block
for 'mut'? Identifier 'in' Expr Block
break | continue          // Only inside a loop (§3.4 / §3.5)
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
