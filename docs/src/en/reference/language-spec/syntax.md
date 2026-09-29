# Syntax Specification

This document defines the syntax specification of the YaoXiang programming language, including
lexical structure, grammar rules, and operator precedence.

---

## Chapter 1: Lexical Structure

### 1.1 Source File

YaoXiang source files must use UTF-8 encoding. Source files typically use the `.yx` extension.

### 1.2 Lexical Token Categories

| Category   | Description                        | Examples                  |
| ---------- | ---------------------------------- | ------------------------- |
| Identifier | Starts with a letter or `_`        | `x`, `_private`, `my_var` |
| Keyword    | Language-predefined reserved words | `Type`, `pub`, `use`      |
| Literal    | Fixed values                       | `42`, `"hello"`, `true`   |
| Operator   | Operation symbols                  | `+`, `-`, `*`, `/`        |
| Delimiter  | Syntax separators                  | `(`, `)`, `{`, `}`, `,`   |

### 1.3 Keywords

YaoXiang defines a very small set of keywords:

```
pub    use    spawn
ref    mut    if     else
else   match  while  for    return
break  continue as     in     unsafe
```

These keywords have special meaning in any context and cannot be used as identifiers.

### 1.4 Reserved Words

YaoXiang's "reserved words" are divided into three layers, recognized by the parser and the type
checker at different stages:

#### 1.4.1 Literal Reserved Words

The parser has independent tokens for literal identifiers, which cannot be used as ordinary
identifiers:

| Identifier | Type | Description                                                                                                   |
| ---------- | ---- | ------------------------------------------------------------------------------------------------------------- |
| `Type`     | —    | Meta type keyword                                                                                             |
| `true`     | Bool | Boolean true value                                                                                            |
| `false`    | Bool | Boolean false value                                                                                           |
| `void`     | Void | Void literal (Unit value). Lowercase `void` is a value literal; uppercase `Void` is a type name (see §1.4.3). |

#### 1.4.2 Variant Names

Variant names are recognized by the parser in **pattern** contexts (bare names are resolved by the
scrutinee type's variant set, see §2.8); in **expression** contexts, constructing a variant requires
a type qualifier — `Result(Int, String).ok(5)`, `Option(T).some(v)`, `Color.green()` (RFC-010
record-style and types: variant constructors are function-style fields in type definitions;
bare-name `ok(5)` is not a constructor call).

| Variant Name | Type   | Description                      |
| ------------ | ------ | -------------------------------- |
| `some(T)`    | Option | Option value variant constructor |
| `ok(T)`      | Result | Result success variant           |
| `err(E)`     | Result | Result error variant             |

#### 1.4.3 Builtin Type Names

The following type names are pre-registered by the type checker and can be used in type positions
without imports. The parser treats them as ordinary identifiers — **not reserved words, and can be
shadowed by local bindings (not recommended)**.

| Type Name | Logical Correspondence | Description                                                                                                                                    |
| --------- | ---------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| `Void`    | ⊤ (True/Unit)          | Zero-field product type, with exactly one inhabitant (`void` literal, see §1.4.1)                                                              |
| `Never`   | ⊥ (False/Empty type)   | Zero-variant sum type, with zero inhabitants. No expression can produce a `Never` value. `Never <: T` holds for all `T` (explosion principle). |
| `Int`     | —                      | Signed integer                                                                                                                                 |
| `Float`   | —                      | Floating-point number                                                                                                                          |
| `Bool`    | —                      | Boolean value: `true` / `false`                                                                                                                |
| `Char`    | —                      | Unicode character                                                                                                                              |
| `String`  | —                      | String                                                                                                                                         |

### 1.5 Identifiers

Identifiers start with a letter or underscore, followed by letters, digits, or underscores.
Identifiers are case-sensitive.

Special identifiers:

- `_` is used as a placeholder, indicating that a value is ignored
- Identifiers starting with an underscore denote private members

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

> **Dictionary literals require at least one key-value pair**: `{}` is **not** an empty dictionary —
> it is an empty block (value `Void`, see [§2.9](#_2-9-block-expression)). Use the constructor
> `dict.new()` for an empty dictionary:
>
> ```yaoxiang
> empty = dict.new()                  // ✅ Empty dictionary
> d = { "a": 1 }                       // ✅ Dictionary literal
> wrong = {}                           // ❌ This is not a dictionary, it's an empty block (Void)
> ```
>
> **The criterion is content**: The `Dict` grammar requires at least one `String ':' Expr`. `{}` has
> no content to fall back on, so it takes the zero form of a block structure. The non-empty form is
> self-describing by its content (`{ "k": v }` has key-value pairs → dictionary) — this is the same
> principle as `f = { 5 }` being an `Int` value rather than a function: **the type is determined by
> the content**.

> Set has no literal grammar and no runtime representation — the collection type is being planned;
> when the need arises, complete it following the Dict pattern (std.set + HeapValue::Set). The
> resolution of List/Dict literals is determined by the context type annotation: a bare literal with
> `List(T)` annotation resolves to a growable list; an `Array(T, N)` annotation directly applied to
> a literal resolves to a fixed-length array. Implicit List→Array conversion is forbidden.
>
> Array literal semantics:
>
> - The number of elements must equal N; otherwise compile-time E1002; an empty literal with a
>   non-zero N is likewise rejected
> - Each element's type must be compatible with T; otherwise compile-time E1002
> - N's grammar form: only an integer literal (possibly negative) or a constant name; compound
>   expressions (such as `2+1`) are rejected at parse time
> - When N is a symbolic constant (such as a function's const parameter, e.g., `Array(Int, n)`), the
>   element count check is deferred to the refinement type phase
> - v1 nested array literals (such as `Array(Array(Int,2),2) = [[1,2],[3,4]]`) are rejected at
>   compile time; they must be constructed explicitly layer by layer; the recursive resolution is
>   left to a later version

#### 1.6.5 List Comprehensions

```
ListComp    ::= '[' Expr 'for' Identifier 'in' Expr (',' Expr)* ('if' Expr)? ']'
```

> **Behavior tightening (migration record)**: The grammar for the iteration variable was originally
> `'for' Identifier 'in'`, but the old implementation parsed the pattern with a full Pratt parser —
> after `in` was registered as an infix operator, `x` would absorb `in items` as a membership
> expression. After the fix, non-identifier patterns fail to parse directly, no longer falling back
> to `_` (silently swallowing errors) like the old implementation. Impact: previously parseable
> forms like `[x for (a, b) in pairs]` now produce errors — this form never had defined behavior
> (the variable was always `_`); the tightening direction is correct, with no semantic migration
> cost.

#### 1.6.6 Membership Check

```
Membership  ::= Expr 'in' Expr
```

> `in` is a binary relational operator that returns `Bool` — `true` on hit, `false` on miss, no
> error. Semantic split: `[]` asserts existence and retrieves a value (fails with an error); `in`
> asks whether something exists (a miss is a normal `false`). Right-operand coverage: List / Array /
> Dict (key set) / Tuple / String (substring) / Range (interval). `in` is a first-class Hoare
> predicate, used as the base of compile-time provable propositions in the refinement type phase.
> (Set is removed from the right-operand list — Set has no runtime representation, see §1.6.4)

### 1.7 Comments

```
// Single-line comment

/* Multi-line comment
   can span multiple lines */
```

### 1.8 Indentation Rules

Code must use 4-space indentation; Tab characters are forbidden. This is a mandatory syntax rule.

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

> **Unary prefix operators** (`!` `-` `+`) bind tightly: only lower than call and member access,
> higher than all binary operators. Hence `!a == b` ≡ `(!a) == b` (Zig-style semantics); `!` is a
> pure unary operation, does not participate in short-circuit control flow, and is orthogonal to the
> `and`/`or` keywords (short-circuit) (RFC-010 authoritative definition).

> **Range binding strength**: `..` has binding strength (6, 7) — the left side 6 is lower than
> addition (7), the right side 7 swallows addition but not the same-level `..`. Before/after change
> comparison:
>
> | Expression   | Before change (level 1, right-associative)                                                     | After change ((6,7), left-associative)                                           |
> | ------------ | ---------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------- |
> | `x in 1..10` | `x in 1..10` (`in`'s right operand level 4, `..` level 1 cannot swallow, actually unparseable) | `x in (1..10)` — the entire interval is the right operand of `in`                |
> | `0..n+2`     | `(0..n)+2` (right-associative trap: upper bound is consumed, `for` loop directly E3004)        | `0..(n+2)` — the upper bound is an arithmetic expression                         |
> | `a == b..c`  | `a == (b..c)` (`..` level 1 < `==` level 3, naturally whole)                                   | `a == (b..c)` — **semantics unchanged**, `..` still higher than comparison level |
> | `1..2*3`     | `(1..2)*3`                                                                                     | `1..(2*3)` — the upper bound is an arithmetic expression                         |
> | `a..b..c`    | `a..(b..c)` (right-associative chained, meaningless Range nested in Range)                     | `(a..b)..c` — **step form** (`c` is the step)                                    |
>
> Net effect: the compound upper bound `for i in 0..n+2` goes from "parses successfully but E3004"
> to "directly usable"; `x in 1..10` goes from "unparseable" to "interval check"; `a..b..c` goes
> from "meaningless nesting" to "step component". Level 6 falls between `+` (level 5) and `<<`
> (level 7), matching mathematical convention: an interval is a tightly-bound construct, and its
> upper bound is naturally a complete arithmetic expression.

### 2.3 Function Call

```
FnCall      ::= Expr '(' ArgList? ')'
ArgList     ::= Expr (',' Expr)* (',' NamedArg)* | NamedArg (',' NamedArg)*
NamedArg    ::= Identifier '=' Expr
```

Named arguments use `name = value` (RFC-010 §Function Definition, RFC-011 §Construction Forms).
Positional arguments must come before named arguments; order-specified parameters can appear in any
order among named arguments:

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b

add(3, 5)          // Positional
add(a = 3, b = 5)  // Named
add(b = 5, a = 3)  // Order arbitrary
add(3, b = 5)      // Mixed, positional first
```

A wrong name in a named argument reports **E1014**, the same formal parameter being specified both
positionally and by name reports **E1015**, a mismatched count reports **E1010** (RFC-013).

### 2.4 Member Access

```
MemberAccess::= Expr '.' Identifier
```

### 2.5 Index Access

```
IndexAccess ::= Expr '[' Expr ']'
```

> **Three layers of semantics (RFC-011b)**: `a[i]` is dispatched based on `a`'s type — ① Builtin
> containers (List/Vec/Array/Dict/Tuple) use native indexing instructions (fast path); ② User types
> that implement the `Index` interface dispatch to their `index` method (instantiation of
> `Index(Grid, Int, Float)` in the type body + the `Grid.index` method); ③ The `f[0]` positional
> binding of RFC-004 only exists in binding declarations and is not part of this grammar. The keys
> of multi-dimensional indexing `a[0, 1]` are packaged as a tuple. Other types that do not implement
> `Index` are rejected at the type layer.

### 2.6 Type Cast

```
TypeCast    ::= Expr 'as' TypeExpr
```

### 2.7 Conditional Expression

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

> **Variant-set requirement for variant destructuring (RFC-010b)**: `EnumPattern` (variant names
> like `ok(v)`, `some(x)` resolved by the scrutinee) requires that the scrutinee's type be a **sum
> type with the variant set in scope**. The variant set enters the checker only via type definitions
> or `use` imports — `Result`/`Option` are defined by `std.result`/`std.option`, and
> `use std.result` / `use std.option` is required before use (whole-module and grouped
> `use std.{...}` forms have equal weight); otherwise variant destructuring reports E1002.
> Exhaustiveness checking shares the same source: a fallback arm waives exhaustiveness; without a
> fallback arm, the check is over the entire variant set.

### 2.9 Block Expression

```
Block       ::= '{' Stmt* Expr? '}'
```

> **Statement termination rules**: The separation and newline behavior between Stmts (`;` explicit
> separator, newline termination, line-continuation exceptions, leading `(`/`[` never merging) is
> defined by [RFC-038](../../design/rfc/accepted/038-statement-termination.md).

#### 2.9.1 The Three Forms of `{`

In expression position, `{` has exactly three interpretations, decided by the **content** in a
single pass:

| Form                   | Notation          | Type                 | Example          |
| ---------------------- | ----------------- | -------------------- | ---------------- |
| **Empty block**        | `{}`              | `Void`               | `x: Void = {}`   |
| **Dictionary literal** | `{ "k": v, ... }` | `Dict(K, V)`         | `d = { "a": 1 }` |
| **Block**              | `{ Stmt* Expr? }` | Tail expression type | `y = { 1 + 1 }`  |

**Decision order**:

1. `{}` (no content) → **Empty block**, value `Void`
2. First element is a `String ':' Expr` key-value pair → **Dictionary literal**
3. Otherwise → **Block**, value given by the tail expression

> **Why `{}` is not an empty dictionary**: The "emptiness" of an empty dictionary cannot describe
> itself (it could be either `Dict(K, V)` or an empty block), and the dictionary grammar
> [§1.6.4](#_1-6-4-collections) requires at least one key-value pair. When there is no content to
> fall back on, the zero form of a block structure is taken: this is consistent with `unsafe {}` /
> `spawn {}`, without introducing special cases. Use `dict.new()` for an empty dictionary.
>
> **Why functions need an annotation**: `f = { stmt }` is a **value** (tail expression type), not a
> function. To define a function, write the Fn annotation explicitly: `f: () -> Int = { 5 }`. This
> is the same principle as for dictionaries: **the type is determined by the content**, not by
> whether an annotation exists. (This rule is in
> [RFC-010a](../../design/rfc/accepted/010a-tail-expression-and-return.md) Appendix D.)

**Unified semantics**: The value of every `{}` block is given by the **tail expression**; `return`
is a non-local exit of type `Never`.

| Block Type  | Value Exit      | Empty Block `{}` |
| ----------- | --------------- | ---------------- |
| Plain `{}`  | Tail expression | `Void`           |
| `unsafe {}` | Tail expression | `Void`           |
| `spawn {}`  | Tail expression | `Void`           |

**Core principles** (see [RFC-010a](../../design/rfc/accepted/010a-tail-expression-and-return.md)
for details):

- **A block's value = the tail expression** (the last expression), the only exit, with no exceptions
- When the last item is an **assignment statement**, the block's value is `Void`; to get `Void`,
  write `Void` explicitly
- **`return` exits the nearest function boundary** (passes through all blocks, does not "return to
  the block"), type `Never`; `Never <: T` holds for any type (explosion principle), so it can appear
  in any return-type position
- The expression form `= expr` directly gives the value

```yaoxiang
// Plain {} block: the tail expression gives the value
result = {
    x = compute()
    x                // The block's value
}

// unsafe {} block: the tail expression gives the type definition
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void
    }
    SqliteDb         // The block's value
}

// spawn {} block: the tail expression gives the result
(a, b) = spawn {
    result1 = fetch("url1"),
    result2 = fetch("url2")
    (result1, result2)   // The block's value
}

// return: passes through blocks, exits the function
f: (n: Int) -> Int = {
    if n < 0 {
        return 0     // Passes through if and the function body, exits the function
    }
    n * 2            // Tail expression
}
```

#### Is `name = { ... }` a Function or a Block Value?

`name = { ... }` may be either a function definition (RFC-007 "zero-argument simplest") or a
block-value binding. The decision rule is **annotation-first, function-by-default** (RFC-010a
Appendix D):

| Situation                         | Result          | Example                            |
| --------------------------------- | --------------- | ---------------------------------- |
| Value is a Lambda (`=>`)          | Function        | `f = () => 5` → `f()` = 5          |
| Annotation is a function type     | Function        | `f: () -> Int = { 5 }` → `f()` = 5 |
| Annotation is a non-function type | **Block value** | `x: Int = { 5 }` → `x` = 5         |
| No annotation                     | Function        | `f = { 5 }` → `f()` = 5            |

**Annotation is the type**: `x: Int = ...` declares that `x` is `Int`, so `{ ... }` evaluates to
`Int`; `f: () -> Int = ...` declares that `f` is a function, so `{ ... }` is the function body.

To make `{ ... }` evaluate immediately, **just write the target type** (no new syntax is needed):

```yaoxiang
// Block value: evaluate immediately
x: Int = {
    y = 5
    y            // x = 5
}

// Function: no annotation, defaults to function
f = { 5 }        // f() = 5
```

#### Nested Function Types: Currying or Returning a Function?

A nested function type to the right of `->` has two readings, distinguished by **parentheses**
(RFC-004):

| Notation                        | Meaning                | Call                     |
| ------------------------------- | ---------------------- | ------------------------ |
| `(a: Int) -> (b: Int) -> Int`   | **Currying**           | `f(1)(2)`                |
| `(a: Int) -> ((b: Int) -> Int)` | **Returns a function** | `g(1)` yields a function |

The rule: **annotation is the type**. `g: (a: Int) -> ((b: Int) -> Int)` declares that
`g(1) : (b: Int) -> Int`, so `g(1)` must **be** that function, not "the next segment of arguments".

```yaoxiang
// Currying: two segments of arguments given layer by layer
add: (a: Int) -> (b: Int) -> Int = { a + b }
add(1)(2)        // → 3

// Returns a function: the outer layer takes one argument, the returned value is the function
adder: (n: Int) -> ((x: Int) -> Int) = (x) => x + n
adder(10)(5)     // → 15
h = adder(10)    // The returned function can be stored in a variable or passed around
```

Unparenthesized nested `Fn` is always currying (including the type-parameter form from RFC-011):

```yaoxiang
identity: (T: Type) -> (x: T) -> T = (x) => x   // Currying
identity(5)      // → 5
```

**Type check**: The parentheses declare the return type; the body must produce that type.
`f: () -> (() -> Int) = { 7 }` is an error — expected `() -> Int`, but actually got `Int` (E1002).

### 2.10 Lambda Expression

```
Lambda      ::= '(' ParamList? ')' '=>' Expr
            |  '(' ParamList? ')' '=>' Block
```

### 2.11 Error Propagation Operator

```
ErrorPropagate ::= Expr '?'
```

The `?` operator is a postfix operator, with the same precedence as `.`. For `Result(T, E)`:

- On `Ok(v)`, extract the value `v` and continue
- On `Err(e)`, propagate the error upward (`return Err(e)`)

```yaoxiang
process: (data: Data) -> Result(Data, Error) = {
    validated = validate(data)?     // Extract on success, propagate on failure
    transform(validated)
}
```

### 2.12 Range Expression

```
RangeExpr   ::= Expr '..' Expr ('..' Expr)?
```

`..` creates a range value (Range is a first-class value, not syntactic sugar).

```yaoxiang
for i in 0..10 { print(i) }
slice = array[0..5]

// Range is a value: bind, pass, and check membership
r = 1..10
assert.assert(5 in r, "membership")
for i in r { print(i) }

// Step form (third component, default 1)
for i in 0..10..2 { print(i) }  // 0, 2, 4, 6, 8
for i in 10..0..(-2) { print(i) }  // 10, 8, 6, 4, 2
```

> **Step semantics**: In `a..b..c`, `c` is the step. A literal `c = 0` is rejected at compile time;
> a dynamic `c` is zero-checked at runtime (E6001 family; will be promoted to Result once the error
> system lands). `c < 0` is legal; the interval direction reverses with the sign (`10..0..(-2)` is
> decreasing).

### 2.13 ref Expression

```
RefExpr     ::= 'ref' Expr
```

`ref` creates shared ownership. The compiler automatically chooses Rc (single-task) or Arc
(cross-task); the user does not need to care about the implementation details.

```yaoxiang
data = ref heavy_data
spawn { use(data) }   // Cross-task: the compiler automatically picks Arc
```

### 2.14 unsafe Expression

```
UnsafeExpr  ::= 'unsafe' Block
```

The `unsafe` block is used to define opaque types and to operate on raw pointers. Use `return` to
return a type definition to the enclosing scope.

**Semantics**:

- Types and raw pointer operations can be defined inside `unsafe {}`
- The returned type is usable outside the `unsafe {}`
- Field access on such types requires unsafe permission

```yaoxiang
// Define an opaque type inside an unsafe block
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
- An inner scope can access variables from the outer scope
- An outer scope cannot access variables from an inner scope
- Variable declarations follow the "assignment-first" principle

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

- `x = value`: search for `x` outward along the scope chain; assign if found, otherwise declare a
  new one
- `mut x = value`: explicit new mutable declaration; forbidden to share the name with an outer one
- Any name can be declared only once within the same scope

> **Detailed definition**: The complete rules for scopes, variable declaration, and shadowing are
> described in [Module System Specification](./modules.md#chapter-4-scope).

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

**Semantics**: `return` is a **non-local exit** that exits the nearest **function boundary** (passes
through all blocks — including `if` / `while` / `for` / `match` / bare blocks / `spawn` / `unsafe`),
passing the value to the caller. It does **not "return to the block"**.

**Type**: `return e : Never` (`e : T`). `Never <: T'` holds for any `T'` (explosion principle, see
[Type System §2.2](./type-system.md)), so `return` can appear in any return-type position without
any extra rule constraints.

**Relationship to block evaluation**: A block's value is always the **tail expression** (see §2.9).
`{ return n }` as a block has value `n`, of type `Never`; meanwhile, the effect of `return` is to
exit the function. **Both are true simultaneously**, coexisting via the explosion principle.

`return` and the tail expression together enable "early return" without requiring additional rules
that special-case `return` to a function — see
[RFC-010a](../../design/rfc/accepted/010a-tail-expression-and-return.md).

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

**Semantics**: Immediately terminates the innermost enclosing `while`/`for` loop, with control
transferring to after the loop body.

- **Only exits the innermost layer**: `break` always acts on the innermost loop containing it. To
  break out of multiple layers in nested loops, extract the inner loop as a function and use
  `return`, or use a flag (break/continue have no labels; if loop labels are introduced in the
  future, the syntax will follow the RFC process on the loop-declaration side, decided together with
  the proof pipeline's multi-exit design)
- **Only allowed in a loop body**: `break` can only appear within a `while`/`for` loop body
  (including nested blocks/if/match inside the body). Outside a loop, it is a compile error (E1102
  `'break' outside of a loop`)
- **Does not affect termination proofs**: `break` does not participate in termination arguments — it
  provides neither a measure nor a decreasing step in any measure; the loop's termination obligation
  is independent of `break` and is triggered by refinement types (see
  [type-system §8.4](./type-system.md#84-terminates-termination-measure-predicate))
- **Borrow semantics**: The control-flow edge of `break` participates in the structural cut of the
  reverse-BFS liveness analysis of RFC-009a (skipped iterations do not participate in back-edge
  liveness derivation)

```yaoxiang
mut i = 0
while i < 10 {
    i = i + 1
    if i == 3 {
        break              // Control transfers to after the loop, i == 3
    }
}

// Nested loops: break only exits the inner loop
while j < 3 {
    while k < 10 {
        if k == 2 { break }    // Only terminates the inner loop
    }
    j = j + 1                  // Each outer iteration reaches here
}
```

### 3.5 continue Statement

```
ContinueStmt::= 'continue'
```

**Semantics**: Skips the remaining statements in the current iteration and proceeds directly to the
next iteration of the innermost enclosing loop — `while` re-evaluates the condition, `for` takes the
next element.

- **Only affects the innermost layer**: Same as `break`; no labels
- **Only allowed in a loop body**: Outside a loop, it is a compile error (E1102)

```yaoxiang
mut sum = 0
mut n = 0
while n < 5 {
    n = n + 1
    if n == 3 {
        continue           // Skip the accumulation below, n == 3 not counted
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

YaoXiang's `for` loop semantics differ from traditional languages: **each iteration binds a new
value, rather than mutating the same variable**.

```yaoxiang
// Example: for i in 1..5
for i in 1..5 {
    print(i)
}
```

**Execution process**:

| Iteration | Behavior of the loop variable                                                           |
| --------- | --------------------------------------------------------------------------------------- |
| 1st       | Create a new binding `i = 1`, run the body, print 1                                     |
| 2nd       | Create a new binding `i = 2` (the previous binding is destroyed), run the body, print 2 |
| 3rd       | Create a new binding `i = 3`, run the body, print 3                                     |
| 4th       | Create a new binding `i = 4`, run the body, print 4                                     |
| End       | The body ends, the binding is destroyed                                                 |

**Key point**: After each iteration ends, the binding created during that iteration is destroyed.
The next iteration is a completely new binding, with no relationship to the binding from the
previous iteration.

#### 3.9.2 Difference Between `for` and `for mut`

| Syntax              | Loop Variable Mutability | Description                                    |
| ------------------- | ------------------------ | ---------------------------------------------- |
| `for i in 1..5`     | Immutable                | The binding cannot be modified inside the body |
| `for mut i in 1..5` | Mutable                  | The binding can be modified inside the body    |

```yaoxiang
// Legal: each iteration binds a new value, no modification needed
for i in 1..5 {
    print(i)  // Read i's value
}

// Error: immutable binding, cannot be modified
for i in 1..5 {
    i = i + 1  // Error: cannot modify an immutable binding
}

// Legal: use `for mut` to allow modifying the binding
for mut i in 1..5 {
    i = i + 1  // Modification allowed
}
```

#### 3.9.3 Shadowing Check

YaoXiang forbids variable shadowing. A `for` loop variable cannot share a name with a variable in an
outer scope:

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

This rule applies to all code blocks; see [4.3 Shadowing Rules](./modules.md#43-shadowing-rules).

#### 3.9.4 Comparison with Other Languages

| Language | `for` Loop Variable Semantics                              |
| -------- | ---------------------------------------------------------- |
| YaoXiang | Each iteration binds a new value                           |
| Rust     | Modifies the same variable (requires `mut`)                |
| Python   | Modifies the same variable (no `mut` needed)               |
| C/C++    | Modifies the same variable (pointer or reference required) |

**Design rationale**: YaoXiang adopts binding semantics because:

1. **More aligned with natural semantics** In natural language, "for each element x in a collection"
   means each x is an independent individual. YaoXiang's `for i in 1..5` reads as "for each i in 1
   to 5"; each iteration's i is a completely new binding, consistent with human intuition.

2. **Avoids accidental modification** The default-immutable binding semantics means the loop
   variable cannot be accidentally modified inside the body. There is no need to worry about a
   hard-to-trace bug caused by accidentally writing `i = ...` somewhere in a complex loop body.

3. **High-performance options are within reach** When reusing a variable across iterations is
   genuinely needed (e.g., accumulator, cache), declare `for mut` to switch to mutable binding mode.
   This is clearer than implicit shared state — the intent is expressed explicitly in the syntax,
   not hidden in runtime behavior.

### 3.10 spawn Statement

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' 'mut'? Identifier 'in' Expr '{' Expr '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
```

**spawn block**: Explicitly declares a concurrent region; the expressions inside the block run
concurrently.

```yaoxiang
(result_a, result_b) = spawn {
    parse(fetch("url1")),
    parse(fetch("url2"))
}
```

**spawn loop**: Data-parallel loop.

```yaoxiang
results = spawn for item in items {
    process(item)
}
```

**spawn block captures outer variables** (RFC-024 §2.3, value-capture semantics):

- The body referencing outer variables = **Move value capture**: the value is snapshotted into the
  closure environment at the spawn creation point; the body reads it via the env (LoadUpvalue)
- **Primitives** (Int/Float/Bool/Char) are value-copied; outer variables are not affected
- **Handle types** (Struct/String/List, etc.) snapshot = handle copy, sharing the underlying object;
  in the Embedded runtime (default), same-thread same-heap, the handle is valid
- Sharing between multiple tasks requires explicit `ref` (§2.13, the compiler automatically picks
  Rc/Arc)
- `return` inside the block referencing outer variables is captured the same way

```yaoxiang
t1 = 1 + 1
t2 = 2 + 2
result = spawn {
    return t1 + t2    // t1/t2 value-captured, result == 6
}
```

---

### 3.11 Program Entry and Top-Level Statements

In the compiler's view, a source file has two roles, **determined by whether `yaoxiang.toml`
exists**:

| Role       | Decision                                   | Program Body                                                                        |
| ---------- | ------------------------------------------ | ----------------------------------------------------------------------------------- |
| **Script** | Single-file direct run, no `yaoxiang.toml` | **Top-level statements** (executed in written order); `main` is an ordinary binding |
| **Bin**    | `yaoxiang.toml` exists                     | **`main` function**; no executable statements at the top level                      |

#### Script: Top-Level Statements Are the Program

Without a manifest, the file is a "script", and **top-level statements execute in written order**:

```yaoxiang
use std.io
io.println("hello")          // Directly executed
x: Int = { 42 }              // Top-level binding: runtime initialization
io.println(x)                // 42
```

In this mode, `main` is **not special** — it is just an ordinary binding. To make `main` run, you
must call it explicitly:

```yaoxiang
use std.io
main: () -> Void = { io.println("only runs if called") }

main()                       // ← This line is required
```

> **Why not call `main` automatically?** Top-level statements are already the program body. If
> `main` were called implicitly as well, a script that explicitly writes `main()` would execute it
> twice. The two rules cannot coexist, so under Script, "top-level statements" is the only execution
> entry.

#### Bin: `main` Is the Entry

When a manifest exists, the file is an "executable target", and:

- `main` must be defined, and it must be a function (the signature must be zero-argument callable)
- Executable statements are not allowed at the top level — the program body is `main` itself

```yaoxiang
main: () -> Void = {
    print("hello")
}
```

A missing `main`, or a `main` that is not a function, is a compile error (the former: no `main`
makes all functions unreachable; the latter: a value binding will not be called).

> **Library files**: Files `use`d by other files, or files pointed to by `[lib].path` / `[exports]`,
> do not require `main` — they are not program entry points.

#### Initialization of Top-Level Bindings

The initialization values of top-level bindings are **evaluated at runtime**; they need not be
compile-time constants:

```yaoxiang
answer: Int = { 42 }              // Block value
inc: (Int) -> Int = (x) => x + 1
computed: Int = inc(41)           // Function call
```

Initialization runs in **dependency order**, regardless of written order:

```yaoxiang
derived: Int = base * 3           // References base declared later
base: Int = 7                     // Initialized first (topological sort)
```

Circular dependencies are a compile error (the names on the cycle will be listed):

```yaoxiang
a: Int = b + 1
b: Int = a + 1                    // Error: a → b → a
```

> **Design basis**: RFC-029f (file role model), RFC-010a Appendix D (block binding ruling).

---

## Appendix: Syntax Quick Reference

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
