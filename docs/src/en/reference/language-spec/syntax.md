# Syntax Specification

This document defines the syntax specification of the YaoXiang programming language, including its
lexical structure, syntax rules, and operator precedence.

---

## Chapter 1: Lexical Structure

### 1.1 Source Files

YaoXiang source files must use UTF-8 encoding. Source files usually use the `.yx` extension.

### 1.2 Lexical Token Categories

| Category   | Description                        | Examples                  |
| ---------- | ---------------------------------- | ------------------------- |
| Identifier | Begins with a letter or underscore | `x`, `_private`, `my_var` |
| Keyword    | Predefined reserved word           | `use`, `mut`, `and`       |
| Literal    | Fixed value                        | `42`, `"hello"`, `true`   |
| Operator   | Operator symbol                    | `+`, `-`, `*`, `/`        |
| Separator  | Syntax separator                   | `(`, `)`, `{`, `}`, `,`   |

### 1.3 Keywords

YaoXiang has **17 keywords** (`keyword_from_str` in `src/frontend/core/lexer/state.rs`, each
corresponding to one `TokenKind`):

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
> type. Both `keyword_from_str` and the header of the `TokenKind` enum
> (`src/frontend/core/lexer/tokens.rs`) are marked with “RFC-010: 'type' keyword removed”.
>
> **`pub` is no longer a keyword** (RFC-029g, accepted on 2026-10-02): the language has no
> visibility mechanism, and `pub` has been removed as a whole from the lexer, AST, type checker,
> dead-code exemptions, formatter, and LSP
> ([RFC-029g](../../rfc/accepted/029g-remove-pub-and-auto-bind.md)). It is now an ordinary
> identifier—the old form `pub x = 1` reports E1001 as an undefined name. All top-level bindings are
> importable by default, and modifiers make no difference to visibility
> ([RFC-029](../../rfc/accepted/029-module-semantics.md)).

### 1.4 Reserved Words

YaoXiang’s “reserved words” are divided into three layers, recognized at different stages by the
parser and type checker:

#### 1.4.1 Literal Reserved Words

The parser has separate tokens for literal identifiers, which cannot be used as ordinary
identifiers:

| Identifier | Type | Description                                                                                                   |
| ---------- | ---- | ------------------------------------------------------------------------------------------------------------- |
| `true`     | Bool | Boolean true value                                                                                            |
| `false`    | Bool | Boolean false value                                                                                           |
| `void`     | Void | Void literal (Unit value). Lowercase `void` is a value literal; uppercase `Void` is a type name (see §1.4.3). |

> **`Type` is not at this layer**: it has **no** dedicated `TokenKind` (the `TokenKind` enum in
> `src/frontend/core/lexer/tokens.rs` does not contain it, and `keyword_from_str` has no
> corresponding branch). The parser treats it as an ordinary identifier, while the type checker
> recognizes it in type positions as a meta type. Therefore, in expression positions, `Type` may be
> shadowed by a local binding. It is a **meta-type name**, not a keyword.

#### 1.4.2 Variant Names

Variant names are recognized by the parser in **pattern** contexts (bare names are resolved
according to the scrutinee type’s variant set; see §2.8). In **expression** contexts, constructing a
variant requires type qualification—`Result(Int, String).ok(5)`, `Option(T).some(v)`,
`Color.green()` (under RFC-010 record types and types, variant constructors are function-typed
fields in type definitions; a bare name such as `ok(5)` is not a constructor call).

| Variant name | Owning type | Description                             |
| ------------ | ----------- | --------------------------------------- |
| `some(T)`    | Option      | Construction of an Option value variant |
| `ok(T)`      | Result      | Result success variant                  |
| `err(E)`     | Result      | Result error variant                    |

#### 1.4.3 Built-in Type Names

The following type names are pre-registered by the type checker and may be used in type positions
without an import. The parser treats them as ordinary identifiers—**they are not reserved words and
may be shadowed by local bindings (not recommended)**.

| Type name | Logical correspondence | Description                                                                                                                                           |
| --------- | ---------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Void`    | ⊤ (true/Unit)          | Zero-field product type with exactly one inhabitant (the `void` literal; see §1.4.1)                                                                  |
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

- `_` is a placeholder indicating that a value should be ignored
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
> Since `{}` has no content to inspect, it takes the zero form of the block structure. Non-empty
> forms are self-describing through their content (`{ "k": v }` has a key-value pair → dictionary).
> This has the same source as `f = { 5 }` being an `Int` value rather than a function: **the type is
> determined by the content**.

> Set has no literal grammar and no runtime representation—when set types are planned, it will be
> completed following the Dict model (std.set + HeapValue::Set). The resolution of List/Dict
> literals is determined by the contextual type annotation: a bare literal and a `List(T)`
> annotation both resolve to a growable list—the growable-list type surface has the single name
> `Vec(T)`, and a `List(T)` annotation is normalized from its source name and resolves to the same
> type surface as `Vec(T)`; an `Array(T, N)` annotation applied directly to a literal resolves to a
> fixed-length array. Implicit List→Array conversion is prohibited.
>
> Array literal semantics:
>
> - The number of elements must equal N; otherwise, compilation fails with E1002. An empty literal
>   with a nonzero N is likewise rejected
> - Every element type must be compatible with T; otherwise, compilation fails with E1002
> - N’s grammar form: an integer literal (possibly negative), a constant name, or a constant
>   expression beginning with a reference to a compile-time value parameter (continued with infix
>   arithmetic, such as `2+3` or `N+1`). It participates in the element-count check by its folded
>   value (`2+3` has the same form as `5`)
> - Parse-time rejection: floating-point/string seeds (E0010) and incomplete expressions (such as
>   `2+`, E0012); an identifier followed by `(` follows the constructor type path—a function call
>   cannot serve as a type-position argument (E1003)
> - When N is a symbolic constant (such as a function `const` parameter, as in `Array(Int, n)`), the
>   element-count check is deferred to the refinement type stage
> - v1 rejects nested array literals (`Array(Array(Int,2),2) = [[1,2],[3,4]]`) at compile time; each
>   layer must be constructed explicitly. Recursive resolution is deferred to a later version

#### 1.6.5 List Comprehensions

```
ListComp    ::= '[' Expr ( 'for' Identifier 'in' Expr ( 'if' Expr )? )+ ']'
```

> **#401 (implementation completion)**: starting with this version, `if` filters and multiple
> generator clauses are implemented. The previous grammar was `(',' Expr)* ('if' Expr)?`, which had
> no implementation counterpart (neither comma-separated additional expressions nor multiple
> generators were supported), and the parser omitted `if` filters altogether (after parsing the
> iterable, it expected `']'` and reported `E0010 Expected RBracket, found KwIf`). The standard
> industry semantics are now established: there may be one or more generator clauses, expanded as
> nested loops; each clause may carry at most one `if` filter, whose condition must have type `Bool`
> (a non-Bool condition reports `E1054`); the iterable and condition of later clauses may reference
> iteration variables bound by earlier clauses.

> **Tightened behavior (migration record)**: the iteration-variable grammar has always been
> `'for' Identifier 'in'`, but the old implementation parsed the pattern using the complete Pratt
> parser—after `'in'` was registered as an infix operator, `x` would consume `in items` as a
> membership expression. After the fix, non-identifier patterns fail to parse directly and no longer
> fall back to `_` as the old implementation did (silently swallowing the error). Impact: forms such
> as `[x for (a, b) in pairs]` that could previously be parsed now report an error. This form never
> had defined behavior (the variable was always `_`), so tightening it is correct and entails no
> semantic migration cost.

#### 1.6.6 Membership Testing

```
Membership  ::= Expr 'in' Expr
```

> `in` is a binary relational operator that returns `Bool`—`true` when matched and `false` when not
> matched; it does not report an error. The semantic split is: `[]` asserts existence and retrieves
> a value (and fails with an error), whereas `in` asks whether something exists (a non-match is a
> normal `false`). Right-hand operands covered: List / Array / Dict (key set) / Tuple / String
> (substring) / Range (interval). `in` is a first-class Horn predicate and serves as the basis for
> compile-time provable propositions during refinement typing. (Set is omitted from the list of
> right-hand operands—Set has no runtime representation; see §1.6.4.)

### 1.7 Comments

```
// Single-line comment

/* Multi-line comment
   may span multiple lines */
```

### 1.8 Indentation Rules

Code must use four spaces for indentation. Tab characters are prohibited. This is a mandatory syntax
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

> **Unary prefix operators** (`!` `-` `+`) bind tightly: they rank below only call and member access
> and above all binary operators. Therefore, `!a == b` ≡ `(!a) == b` (Zig-style semantics); `!` is a
> pure unary operation, does not participate in short-circuit control flow, and is orthogonal to the
> `and`/`or` keywords, which perform short-circuiting (as authoritatively defined by RFC-010).

> **Range binding power**: `..` has binding powers (6, 7)—the left binding power 6 is lower than
> addition (7), while the right binding power 7 absorbs addition but not a same-level `..`.
> Comparison before and after the change:
>
> | Expression   | Before the change (level 1, right-associative)                                                              | After the change ((6,7), left-associative)                                           |
> | ------------ | ----------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
> | `x in 1..10` | `x in 1..10` (`in`’s right operand is level 4, and level-1 `..` cannot absorb it, so parsing fails)         | `x in (1..10)`—the entire interval is the right operand of `in`                      |
> | `0..n+2`     | `(0..n)+2` (right-associativity trap: the upper bound is consumed, and a `for` loop reports E3004 directly) | `0..(n+2)`—the upper bound is an arithmetic expression                               |
> | `a == b..c`  | `a == (b..c)` (level-1 `..` < level-3 `==`, naturally covering the whole expression)                        | `a == (b..c)`—**semantics unchanged**; `..` still binds more tightly than comparison |
> | `1..2*3`     | `(1..2)*3`                                                                                                  | `1..(2*3)`—the upper bound is an arithmetic expression                               |
> | `a..b..c`    | `a..(b..c)` (right-associative chain, with meaningless nesting of Ranges)                                   | `(a..b)..c`—**step form** (`c` is the step size)                                     |
>
> Net effect: the compound upper bound `for i in 0..n+2` changes from “parses successfully but
> reports E3004” to “directly usable”; `x in 1..10` changes from “cannot be parsed” to “interval
> check”; and `a..b..c` changes from “meaningless nesting” to “step component”. Level 6 falls
> between `+` (level 5) and `<<` (level 7), following mathematical convention: an interval is a
> tightly bound construct, so its upper bound is naturally a complete arithmetic expression.

### 2.3 Function Calls

```
FnCall      ::= Expr '(' ArgList? ')'
ArgList     ::= Expr (',' Expr)* (',' NamedArg)* | NamedArg (',' NamedArg)*
NamedArg    ::= Identifier '=' Expr
```

Named arguments use `name = value` (RFC-010 §Function Definitions, RFC-011 §Constructor Forms).
Positional arguments must precede named arguments; parameters whose order is specified may appear in
any order among the named arguments:

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b

add(3, 5)          // Positional
add(a = 3, b = 5)  // Named
add(b = 5, a = 3)  // Any order
add(3, b = 5)      // Mixed, with positional arguments first
```

An incorrect named-argument name reports **E1014**, specifying the same formal parameter both
positionally and by name reports **E1015**, and an incorrect argument count reports **E1010**
(RFC-013).

**Unnamed parameters and named arguments**: signature parameter positions permit the bare type form
(`mk: (Int, Int) -> Int`). This is an **unnamed typed parameter**—type constraints apply by
position, while the name belongs to the implementation (as shown in the lambda header; see the
shorthand rules in [RFC-007](../../rfc/accepted/007-function-syntax-unification.md)) and is not part
of the contract. An unnamed parameter cannot be referenced by a named argument because it has no
name; it can only be passed positionally. To place a name in the contract, use
`mk: (a: Int, b: Int) -> Int`. A bare identifier must be a declared type; failure to resolve the
type reports an error (see [type-system §3.5](type-system.md#35-函数类型)).

**Named parameters conversely: the name enters the contract, and the declaration is authoritative**.
Parameter names in the implementation header (the parameter list after `=>`) are merely local names
in the body and are bound **by position**—if the implementation header of
`f: (a: Int) -> Int = (b) => b` uses `b`, callers still write `f(a = 7)`. Type annotations in the
implementation header are used only for comparison; a mismatch with the declaration reports
**E1011**. If the implementation header has more formal parameters than the declaration has runtime
parameter positions, the result is **E1112** (`f: () -> Int = (a: Int) => a`)—in both cases, the
declaration is the authoritative source. The reverse case (the implementation specifying fewer
parameters) does not report an error: the type layer checks arity according to the declaration, but
positions omitted by the implementation header have no runtime slots, and arguments that would
occupy them are discarded on the IR side (see the gap recorded in
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
> containers (List/Vec/Array/Dict/Tuple) use native indexing instructions (the fast path); ② user
> types that implement the `Index` interface are dispatched to their `index` method (instantiation
> of `Index(Grid, Int, Float)` in the type body and the `Grid.index` method); ③ RFC-004’s `f[0]`
> positional binding exists only in binding declarations and does not enter this grammar. For
> multidimensional indexing, the keys in `a[0, 1]` are packaged as a tuple. Other types that do not
> implement `Index` are rejected at the type level.

### 2.6 Type Casting

```
TypeCast    ::= Expr 'as' TypeExpr
```

> **Implementation status (an existing gap, not introduced in this version)**: the preceding text
> defines only the grammar and precedence (§2.4, level 2); the semantics have not yet been
> implemented. Independently verifiable facts: ① Parsing: the `KwAs` infix arm in
> `parser/pratt/led.rs:81` produces `ast::Expr::Cast`; ② Type checking:
> `typecheck/inference/expressions.rs:3847-3853` performs only a **silent retagging** for
> `Expr::Cast`—`let _ = self.infer_expr(expr)?;` discards the inner result, after which it directly
> returns the `target_type` `MonoType`; ③ The repository has **no** convertibility validation and no
> corresponding diagnostic code (`util/diagnostic/codes/` has no cast entry); ④ `ir_gen` does not
> generate any instructions (`middle/core/ir_gen.rs` only obtains the span and free variables of
> `Expr::Cast` at `:5111` / `:5918` / `:9381`—the first two are the span arms of `get_expr_span` /
> `expr_span`, and the third is `collect_free_vars_expr`; none of the three emits instructions); ⑤
> Consequently, `x as Int` currently reports neither an error nor any runtime conversion.

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
RangePattern ::= Literal '..' Literal
```

> **Variant-set requirement for variant destructuring (RFC-010b)**: `EnumPattern` (such as `ok(v)`
> or `some(x)`, whose variant name is resolved according to the scrutinee) requires the scrutinee’s
> type to be a **sum type whose variant set is present**. A variant set enters the checker only
> through a type definition or a `use` import—`Result`/`Option` are defined by
> `std.result`/`std.option`, so `use std.result` / `use std.option` must appear before use
> (whole-module and grouped `use std.{...}` forms are equivalent); otherwise, variant destructuring
> reports E1002. Exhaustiveness checking has the same source: a fallback arm is exempt from
> exhaustiveness, while without a fallback arm, all variants are checked.

> **Pattern bindings, or-patterns, and guards (RFC-010b)**:
>
> - **Bindings**: a bare identifier in a pattern introduces a **new name**—it is neither a
>   comparison nor an assignment. The name is visible **within the arm**, shadows an outer variable
>   of the same name, and becomes invalid after leaving the arm. Discriminants (such as `ok` in
>   `ok(v)`) are resolved according to the scrutinee’s variant set; a bare name that cannot be found
>   is a binding.
> - **Or-patterns**: `p1 | p2` tries the alternatives in order; if a non-final alternative fails,
>   the next alternative is tried, and if the final alternative fails, control proceeds to the next
>   arm. All alternatives must bind **the same set of names**, or E1033 is reported; a binding with
>   the same name exists only once at arm level, and the arm body necessarily reads the value
>   written by the matching alternative.
> - **Guards**: after a pattern matches, the `if` condition is evaluated, with pattern bindings
>   visible to the condition; the condition must produce `Bool`; if false, control proceeds to the
>   next arm. An arm with a guard **does not count toward exhaustive coverage** (the guard may not
>   match), but a duplicate variant arm with a guard may still be reachable (it refines a previous
>   arm).
> - **Diagnostics**: a missing branch reports E1030, an unreachable arm reports E1031, a duplicate
>   binding within the same pattern reports E1032, inconsistent bindings across or-patterns report
>   E1033, and a missing field in a struct pattern reports E1034; see the
>   [error-code reference](../error-code/E1xxx.md) for the code table and messages.
>
> **Range patterns (issue #405)**: `lo..hi` is a **closed-interval test**—over the Int domain,
> `x matches lo..hi ⟺ lo <= x <= hi`, so both 80 and 89 match `80..89`. Endpoints must be **integer
> literals** (a leading minus sign is normalized to a negative value, such as `-5..-1`); other
> literal types (Float/Bool/String/Char) are syntactically valid but rejected during type checking
> (E1002; Float ranges are not supported in the first version). An identifier endpoint in a pattern
> position is a binding name with no matching semantics and is rejected by the syntax layer (E0010).
> When both endpoints are decidable at compile time, a reversed range (`lo > hi`) is an empty range
> that never matches and reports unreachable (E1031); repeated range arms with the same endpoints
> likewise report E1031.
>
> Exhaustiveness interaction: range patterns belong to the same category as literal patterns—they
> are **checked, partially covering, and non-fallback**. Because the Int domain is open, a range arm
> cannot be exhaustive, and a match containing a range arm still requires a wildcard/identifier arm
> as a fallback (E1030); range arms may serve as or-pattern alternatives and may have guards (a
> failed guard proceeds to the next arm, consistent with the existing guard rules).
>
> **Semantic separation from the Range expression in §2.12**: `lo..hi` in a pattern position does
> not construct a Range value; it only performs branch matching, with endpoints having
> closed-interval semantics. The expression form `a..b` (`for` iteration, `in` interval checks, and
> first-class Range values) remains half-open. These two `..` semantics are intentionally separate:
> a pattern is a predicate over a discrete value domain, whereas an expression is notation for
> constructing an iteration sequence.

### 2.9 Block Expressions

```
Block       ::= '{' Stmt* Expr? '}'
```

> **Statement termination rules**: separation and newline behavior between statements (`;` as an
> explicit separator, newline termination, continuation-line exceptions, and line-leading `(`/`[`
> never merging) are defined by [RFC-038](../../rfc/accepted/038-statement-termination.md).

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
3. Otherwise → **block**, whose value is provided by the tail expression

> **Why `{}` is not an empty dictionary**: the “emptiness” of an empty dictionary cannot describe
> itself (it could be either a `Dict(K, V)` or an empty block), whereas the dictionary grammar
> [§1.6.4](#_1-6-4-集合) requires at least one key-value pair. With no content to inspect, the zero
> form of the block structure is used: this is consistent with `unsafe {}` / `spawn {}` and
> introduces no special case. Use `dict.new()` for an empty dictionary.
>
> **Why functions require annotations**: `f = { stmt }` is a **value** (with the tail-expression
> type), not a function. To define a function, specify an Fn annotation: `f: () -> Int = { 5 }`.
> This follows the same principle as dictionaries: **the type is determined by the content**, not by
> whether an annotation is present. (See
> [RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md), Appendix D.)

**Unified semantics**: the values of all `{}` blocks are provided by the **tail expression**, and
`return` is a nonlocal exit of type `Never`.

| Block type    | Value exit      | Empty block `{}` |
| ------------- | --------------- | ---------------- |
| Ordinary `{}` | Tail expression | `Void`           |
| `unsafe {}`   | Tail expression | `Void`           |
| `spawn {}`    | Tail expression | `Void`           |

**Core principles** (see [RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md)):

- **A block’s value = the tail expression** (the final expression), its sole exit, with no
  exceptions
- When the final item is an **assignment statement**, the block value is `Void`; to obtain `Void`,
  explicitly write `Void`
- **`return` exits the nearest function boundary** (passing through every block, rather than
  “returning to the block”), and has type `Never`; `Never <: T`

> holds for any type (principle of explosion), so it may appear at any position where a return type
> is expected. The module initialization layer (Script top level) has no function boundary, so
> `return` is rejected at compile time there (`E1109`; see §3.11)

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
        return 0     // Passes out of if and the function body, exiting the function
    }
    n * 2            // Tail expression
}
```

#### Is `name = { ... }` a Function or a Block Value?

`name = { ... }` may be either a function definition (RFC-007’s “empty-parameter minimal form”) or a
block-value binding. The decision rule is **annotation first, function by default** (RFC-010a,
Appendix D):

| Case                                  | Result          | Example                            |
| ------------------------------------- | --------------- | ---------------------------------- |
| The value is a Lambda (`=>`)          | Function        | `f = () => 5` → `f()` = 5          |
| The annotation is a function type     | Function        | `f: () -> Int = { 5 }` → `f()` = 5 |
| The annotation is a non-function type | **Block value** | `x: Int = { 5 }` → `x` = 5         |
| No annotation                         | Function        | `f = { 5 }` → `f()` = 5            |

**The annotation is the type**: `x: Int = ...` declares that `x` is an `Int`, so `{ ... }` evaluates
to an `Int`; `f: () -> Int = ...` declares that `f` is a function, so `{ ... }` is a function body.

To evaluate `{ ... }` immediately, **write the target type** (no new syntax is required):

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

There are two ways to read a nested function type on the right-hand side of `->`, distinguished by
**parentheses** (RFC-004):

| Form                            | Meaning                | Invocation               |
| ------------------------------- | ---------------------- | ------------------------ |
| `(a: Int) -> (b: Int) -> Int`   | **Curried**            | `f(1)(2)`                |
| `(a: Int) -> ((b: Int) -> Int)` | **Returns a function** | `g(1)` yields a function |

The basis is: **the annotation is the type**. `g: (a: Int) -> ((b: Int) -> Int)` declares that
`g(1) : (b: Int) -> Int`, so `g(1)` must **be** that function, rather than the next parameter
segment.

```yaoxiang
// Curried: provide two parameter groups in stages
add: (a: Int) -> (b: Int) -> Int = { a + b }
add(1)(2)        // → 3

// Returned function: one outer parameter; only the returned value is a function
adder: (n: Int) -> ((x: Int) -> Int) = (x) => x + n
adder(10)(5)     // → 15
h = adder(10)    // The returned function can be stored in a variable and passed around
```

Without parentheses, every nested `Fn` is curried (including RFC-011’s type-parameter form):

```yaoxiang
identity: (T: Type) -> (x: T) -> T = (x) => x   // Curried
identity(5)      // → 5
```

**Type checking**: the parentheses declare the return type, so the body must produce that type.
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

- When `Ok(v)`, extract the value `v` and continue execution
- When `Err(e)`, propagate the error upward (`return Err(e)`)

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

// Range is a value: bind it, pass it, and perform membership checks
r = 1..10
assert.assert(5 in r, "membership")
for i in r { print(i) }

// Step form (third component, default 1)
for i in 0..10..2 { print(i) }  // 0, 2, 4, 6, 8
for i in 10..0..(-2) { print(i) }  // 10, 8, 6, 4, 2
```

> **Step semantics**: in `a..b..c`, `c` is the step size. The literal `c = 0` is rejected at compile
> time; a dynamic `c` is checked for zero at runtime (E6001 family; it will be promoted to Result
> once the error system is implemented). `c < 0` is valid, and the interval direction reverses with
> its sign (`10..0..(-2)` descends).
>
> **Separation in pattern position**: the match range pattern `lo..hi` (§2.8) does not construct a
> Range value, and its endpoints use closed-interval matching semantics; the expression form `a..b`
> in this section remains half-open.

### 2.13 ref Expressions

```
RefExpr     ::= 'ref' Expr
```

`ref` creates shared ownership. The compiler automatically selects Rc (single-task) or Arc
(cross-task); users need not concern themselves with implementation details.

```yaoxiang
data = ref heavy_data
spawn { use(data) }   // Cross-task: the compiler automatically selects Arc
```

### 2.14 unsafe Expressions

```
UnsafeExpr  ::= 'unsafe' Block
```

`unsafe` blocks are used to define opaque types and manipulate raw pointers. Use `return` to return
the type definition to the enclosing scope.

**Semantics**:

- Types and raw-pointer operations may be defined within `unsafe {}`
- The returned type is available outside `unsafe {}`
- Accessing the type’s fields requires unsafe permission

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

- `x = value`: searches outward along the scope chain for x; if found, assigns to it, and if not
  found, declares a new binding
- `mut x = value`: explicitly declares a new mutable binding and prohibits the same name in an outer
  scope
- Within the same scope, any name may be declared only once

> **Detailed definition**: see the [module system specification](modules.md#第四章作用域) for the
> complete scope rules and the variable declaration and shadowing mechanisms.

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
and passes the value to the caller. It does **not “return to the block”**.

**Type**: `return e : Never` (`e : T`). `Never <: T'` holds for any `T'` (principle of explosion;
see [type-system §2.2](type-system.md)), so `return` may appear at any position where a return type
is expected without requiring additional rules.

**Relationship with block evaluation**: a block’s value is always the **tail expression** (see
§2.9). As a block, `{ return n }` has the value `n` and the type `Never`; at the same time, `return`
exits the function. **Both facts hold simultaneously**, coexisting by virtue of the principle of
explosion.

Together, `return` and tail expressions make “early return” possible without requiring an additional
rule specifying that `return` applies to functions—see
[RFC-010a](../../rfc/accepted/010a-tail-expression-and-return.md).

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 {
        return 1          // Exits if and exits the function (type Never)
    }
    n * factorial(n - 1)  // Tail expression = the block's value
}
```

### 3.4 break Statements

```
BreakStmt   ::= 'break'
```

**Semantics**: immediately terminates the innermost enclosing `while`/`for` loop, transferring
control to after that loop’s body.

- **Exits only one level**: `break` always applies to the innermost enclosing loop. To exit multiple
  levels of nested loops, extract the inner loop into a function and use `return`, or use a flag
  (break/continue have no labels; if loop labels are introduced in the future, their
  declaration-side syntax will follow the RFC process and be adjudicated together with the proof
  pipeline’s multi-exit design)
- **Permitted only inside a loop body**: `break` may appear only in a `while`/`for` loop body
  (including nested blocks/if/match statements within the body); use outside a loop is a
  compile-time error (E1102 `'break' outside of a loop`)
- **Does not affect termination proofs**: `break` does not participate in termination reasoning—it
  neither provides a measure nor constitutes a decreasing measure step. The loop’s termination
  obligation is independent of `break` and is triggered by refinement types (see
  [type-system §8.5](type-system.md#85-terminates终止测度谓词)).
- **Borrow semantics**: the control-flow edge of `break` participates in the structural cutoff of
  RFC-009a’s reverse-BFS liveness analysis (the iteration exited by the break does not participate
  in deriving back-edge liveness)

```yaoxiang
mut i = 0
while i < 10 {
    i = i + 1
    if i == 3 {
        break              // Control transfers to after the loop; i == 3
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
next iteration of the innermost enclosing loop—`while` reevaluates the condition, while `for` takes
the next element.

- **Applies only to the nearest level**: like `break`, it has no label
- **Permitted only inside a loop body**: use outside a loop is a compile-time error (E1102)

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

YaoXiang’s for-loop semantics differ from those of traditional languages: **each iteration binds a
new value rather than modifying the same variable**.

```yaoxiang
// Example: for i in 1..5
for i in 1..5 {
    print(i)
}
```

**Execution process**:

| Iteration | Behavior of the loop variable                                                                                 |
| --------- | ------------------------------------------------------------------------------------------------------------- |
| 1st       | Creates a new binding `i = 1`, executes the loop body, and prints 1                                           |
| 2nd       | Creates a new binding `i = 2` (the previous binding has been destroyed), executes the loop body, and prints 2 |
| 3rd       | Creates a new binding `i = 3`, executes the loop body, and prints 3                                           |
| 4th       | Creates a new binding `i = 4`, executes the loop body, and prints 4                                           |
| End       | The loop body ends, and the binding is destroyed                                                              |

**Key point**: after each iteration ends, the binding created during that iteration is destroyed.
The next iteration is a completely new binding, with no relationship to the previous iteration’s
binding.

#### 3.9.2 Difference Between for and for mut

| Syntax              | Mutability of the loop variable | Description                                         |
| ------------------- | ------------------------------- | --------------------------------------------------- |
| `for i in 1..5`     | Immutable                       | The binding cannot be modified inside the loop body |
| `for mut i in 1..5` | Mutable                         | The binding can be modified inside the loop body    |

```yaoxiang
// Valid: each iteration binds a new value; no modification is needed
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
| YaoXiang | Binds a new value on every iteration                         |
| Rust     | Modifies the same variable (requires `mut`)                  |
| Python   | Modifies the same variable (does not require `mut`)          |
| C/C++    | Modifies the same variable (requires a pointer or reference) |

**Design rationale**: YaoXiang adopts binding semantics because:

1. **It better matches natural semantics.** In natural language, “for each element x in a set” means
   that each x is an independent individual. YaoXiang’s `for i in 1..5` reads as “for each i from 1
   through 5,” and the i in each iteration is a completely new binding, consistent with human
   intuition.

2. **It avoids accidental modification.** Immutable binding semantics by default mean that a loop
   variable cannot be accidentally modified inside the loop body. There is no need to worry that
   somewhere in a complex loop body, an accidental `i = ...` causes a difficult-to-track bug.

3. **A high-performance solution is readily available.** When it is genuinely necessary to reuse a
   variable between iterations (for example, as an accumulator or cache), declare it with `for mut`
   to switch to mutable-binding mode. This is clearer than implicit shared state—the intent is
   expressed explicitly in the syntax rather than hidden in runtime behavior.

### 3.10 spawn Statements

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' 'mut'? Identifier 'in' Expr '{' Expr '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
```

**spawn block**: explicitly declares a concurrent region; expressions inside the block execute
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

- A reference from the block body to an outer variable is a **Move value capture**: the value is
  snapshotted into the closure environment at the spawn creation point, and the block body reads it
  through the environment (LoadUpvalue)
- **Primitive types** (Int/Float/Bool/Char) have their values copied, leaving outer variables
  unaffected
- **Handle types** (Struct/String/List, etc.) are snapshotted by copying the handle and sharing the
  underlying object; under the Embedded runtime (the default), the same thread and heap mean that
  the handle remains valid
- Sharing between multiple tasks requires an explicit `ref` (§2.13; the compiler automatically
  selects Rc/Arc)
- A `return` inside the block that references an outer variable captures that variable in the same
  way

```yaoxiang
t1 = 1 + 1
t2 = 2 + 2
result = spawn {
    return t1 + t2    // t1/t2 are value-captured; result == 6
}
```

---

### 3.11 Program Entry Point and Top-Level Statements

A source file has two roles from the compiler’s perspective, **determined by whether `yaoxiang.toml`
exists**:

| Role       | Determination                                       | Program body                                                                        |
| ---------- | --------------------------------------------------- | ----------------------------------------------------------------------------------- |
| **Script** | A single file run directly, with no `yaoxiang.toml` | **Top-level statements** (executed in written order); `main` is an ordinary binding |
| **Bin**    | `yaoxiang.toml` exists                              | **The `main` function**; top-level executable statements are prohibited             |

#### Script: Top-Level Statements Are the Program

Without a manifest, a file is a “script,” and **top-level statements execute in written order**
(top-level **binding initialization** runs as a whole first; see
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

> **Why `main` is not invoked automatically**: top-level statements are already the program body. If
> `main` were also invoked implicitly, a script explicitly containing `main()` would execute twice.
> The two rules cannot coexist, so Script mode has only one execution entry point: **top-level
> statements**.

Top-level code must likewise **not use `return`**: the Script top level is the module initialization
layer and has no function boundary to exit (RFC-010a rule ②). This is a compile-time `E1109`
error—not a silent runtime termination of initialization that skips the remaining top-level
statements. To end a section of top-level logic early, put it in a function and control it at the
call site.

#### Bin: `main` Is the Entry Point

When a manifest exists, the file is an “executable target,” and:

- A binding named `main` must be defined, and its value may be either a function or a non-function
  value (#388 established: the entry point is determined by the existence of the binding—a function
  is already a value, and the two merely use different evaluation strategies): a function `main` is
  called with zero arguments at entry; a value `main` is evaluated during initialization
- Top-level executable statements are prohibited—the program body is `main`

```yaoxiang
main: () -> Void = {
    print("hello")
}
```

A value `main` is equally valid—evaluating it during initialization executes it, and this evaluation
occurs within the initialization sequence (in global binding topological order, with independent
bindings following source order):

```yaoxiang
main = {
    print("hello")
}
```

A missing `main` is a compile-time error (without `main`, all functions are unreachable). An
uncallable value `main` (such as `main: Int = 5`) is valid but has no observable effect—this
corresponds to Rust’s empty `fn main() {}`.

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

Initialization executes in **dependency order**, independently of written order (dependencies are
initialized first; bindings without dependencies follow declaration order):

```yaoxiang
derived: Int = base * 3           // References base, which is declared later
base: Int = 7                     // Initialized first (topological sort)
```

Furthermore, **all binding initialization precedes the execution of top-level statements**—side
effects in binding initializers occur before any top-level statement, regardless of how the two are
interleaved in the source:

```yaoxiang
print("a")                        // Executes later
x = { print("b") }                // Executes first (all binding initialization comes first)
print("c")                        // Output order: b, a, c
```

Circular dependencies are compile-time errors (the compiler lists the names in the cycle):

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

`type` is no longer a keyword (RFC-010; use `Name: Type = { ... }` instead); `pub` was removed as a
whole under RFC-029g and reverted to an ordinary identifier. For literal reserved words `true` /
`false` / `void`, see §1.4.1; for the meta-type name `Type` and built-in type names `Void` / `Never`
/ `Int` / `Float` / `Bool` / `Char` / `String`, see §1.4.3.

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
