# Syntax Specification

This document defines the syntax specification of the YaoXiang programming language, including lexical structure, grammar rules, and operator precedence.

---

## Chapter 1: Lexical Structure

### 1.1 Source Files

YaoXiang source files must use UTF-8 encoding. Source files typically use the `.yx` extension.

### 1.2 Token Classification

| Category   | Description           | Examples                    |
| ---------- | ---------------------- | --------------------------- |
| Identifier | Starts with letter or underscore | `x`, `_private`, `my_var` |
| Keyword    | Language-reserved words | `Type`, `pub`, `use`      |
| Literal    | Fixed values           | `42`, `"hello"`, `true`     |
| Operator   | Operation symbols      | `+`, `-`, `*`, `/`          |
| Delimiter  | Syntax delimiters      | `(`, `)`, `{`, `}`, `,`     |

### 1.3 Keywords

YaoXiang defines a minimal set of keywords:

```
pub    use    spawn
ref    mut    if     else
else   match  while  for    return
break  continue as     in     unsafe
```

These keywords have special meaning in any context and cannot be used as identifiers.

### 1.4 Reserved Words

YaoXiang's "reserved words" are divided into three levels, recognized by the parser and type checker at different stages:

#### 1.4.1 Literal Reserved Words

These are literal identifiers with independent tokens in the parser and cannot be used as regular identifiers:

| Identifier | Type      | Description                                                                              |
| ---------- | --------- | ---------------------------------------------------------------------------------------- |
| `Type`     | —         | Meta type keyword                                                                        |
| `true`     | Bool      | Boolean true value                                                                       |
| `false`    | Bool      | Boolean false value                                                                      |
| `void`     | Void      | Void literal (Unit value). Lowercase `void` is a value literal; uppercase `Void` is a type name (see §1.4.3). |

#### 1.4.2 Constructor Expressions

The following constructors are recognized by the parser in pattern matching and expression contexts:

| Constructor | Type    | Description              |
| ---------- | ------- | ------------------------ |
| `some(T)`  | Option  | Option value variant constructor |
| `ok(T)`    | Result  | Result success variant   |
| `err(E)`   | Result  | Result error variant     |

#### 1.4.3 Built-in Type Names

The following type names are pre-registered by the type checker and can be used in type positions without importing. The parser treats them as regular identifiers—they are **not reserved words and can be shadowed by local bindings (not recommended)**.

| Type Name | Logical Correspondence | Description                                                                                                      |
| --------- | ---------------------- | ---------------------------------------------------------------------------------------------------------------- |
| `Void`    | ⊤ (True/Unit)          | Zero-field product type with exactly one inhabitant (`void` literal, see §1.4.1)                               |
| `Never`   | ⊥ (False/Empty type)   | Zero-variant enum type with zero inhabitants. No expression can produce a `Never` value. `Never <: T` holds for all `T` (ex falsum principle). |
| `Int`     | —                      | Signed integer                                                                                                   |
| `Float`   | —                      | Floating point number                                                                                             |
| `Bool`    | —                      | Boolean value: `true` / `false`                                                                                   |
| `Char`    | —                      | Unicode character                                                                                                 |
| `String`  | —                      | String                                                                                                           |

### 1.5 Identifiers

Identifiers start with a letter or underscore, followed by letters, digits, or underscores. Identifiers are case-sensitive.

Special identifiers:

- `_` is used as a placeholder to ignore a value
- Identifiers starting with underscore denote private members

### 1.6 Literals

#### 1.6.1 Integers

```
Decimal     ::= [0-9][0-9_]*
Octal       ::= 0o[0-7][0-7_]*
Hex         ::= 0x[0-9a-fA-F][0-9a-fA-F_]*
Binary      ::= 0b[01][01_]*
```

#### 1.6.2 Floating Point Numbers

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
Array       ::= '[' Expr (',' Expr)* ']'   // When target type annotation is Array(T, N), literal resolves to fixed-length array
```

> **Dictionary literals require at least one key-value pair**: `{}` **is not** an empty dictionary—it is an empty block (value `Void`, see [§2.9](#_2-9-block-expressions)). For empty dictionaries, use the constructor `dict.new()`:
>
> ```yaoxiang
> empty = dict.new()                  // ✅ Empty dictionary
> d = { "a": 1 }                       // ✅ Dictionary literal
> wrong = {}                           // ❌ This is not a dictionary, it's an empty block (Void)
> ```
>
> **The criterion is content**: The `Dict` grammar requires at least one `String ':' Expr`. `{}` has no content to match against, so it takes the zero form of the block structure. Non-empty forms are self-describing by content (`{ "k": v }` has key-value pairs → dictionary)—this is homologous to `f = { 5 }` being an `Int` value rather than a function: **type is determined by content**.

> Set has no literal grammar and no runtime representation—set types are planned; when needed, complete following the Dict pattern (std.set + HeapValue::Set). List/Dict literals' resolution is determined by context type annotation: bare literals with `List(T)` annotation resolve to growable list; `Array(T, N)` annotation directly on literals resolves to fixed-length array. Implicit List→Array conversion is forbidden.
>
> Array literal semantics:
>
> - Number of elements must equal N; otherwise compile-time E1002; empty literal with non-zero N also rejected
> - Each element type must be compatible with T; otherwise compile-time E1002
> - N's grammar form: integer literals only (can be negative) or constant names; compound expressions (like `2+1`) rejected at parse time
> - When N is a symbolic constant (function const parameter, like `Array(Int, n)`), element count validation is deferred to refinement type stage
> - v1 nested array literals (`Array(Array(Int,2),2) = [[1,2],[3,4]]`) rejected at compile time; requires explicit layer-by-layer construction; recursive resolution left for future version

#### 1.6.5 List Comprehensions

```
ListComp    ::= '[' Expr 'for' Identifier 'in' Expr (',' Expr)* ('if' Expr)? ']'
```

> **Behavior tightening (migration record)**: The iterator variable grammar was always `'for' Identifier 'in'`, but the old implementation ran patterns through full Pratt parsing—after `'in'` was registered as an infix operator, `x` would swallow `in items` as a membership expression. After the fix, non-identifier patterns now fail directly during parsing instead of falling back to `_` in the old implementation (silent error swallowing). Impact: code like `[x for (a, b) in pairs]` that previously parsed will now error—this form never had defined behavior (variable was always `_`), the tightening direction is correct, with no semantic migration cost.

#### 1.6.6 Membership Testing

```
Membership  ::= Expr 'in' Expr
```

> `in` is a binary relational operator returning `Bool`—hit returns `true`, miss returns `false`, no error. Semantic distinction: `[]` asserts existence and retrieves (failure errors), `in` asks existence (miss is normal `false`). Right operand coverage: List / Array / Dict (key set) / Tuple / String (substring) / Range (interval). `in` is a first-class Hoare predicate, serving as the base for compile-time provable propositions during refinement typing. (Set is removed from the right operand list—Set has no runtime representation, see §1.6.4)

### 1.7 Comments

```
// Single-line comment

/* Multi-line comment
   Can span multiple lines */
```

### 1.8 Indentation Rules

Code must use 4 spaces for indentation. Tab characters are forbidden. This is a mandatory syntax rule.

---

## Chapter 2: Grammar Rules

### 2.1 Expression Classification

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
| 3          | Unary prefix `!` `-` `+`   | Right to left |
| 4          | `*` `/` `%`                 | Left to right |
| 5          | `+` `-`                     | Left to right |
| 6          | `..`                        | Left to right |
| 7          | `<<` `>>`                   | Left to right |
| 8          | `&` `\|` `^`                | Left to right |
| 9          | `==` `!=` `<` `>` `<=` `>=` | Left to right |
| 10         | `and` `or`                  | Left to right |
| 11         | `if...else`                 | Right to left |
| 12         | `=` `+=` `-=` `*=` `/=`     | Right to left |

> **Unary prefix operators** (`!` `-` `+`) bind tightly: only below call and member access, above all binary operators. Therefore `!a == b` ≡ `(!a) == b` (Zig-style semantics); `!` is a pure unary operation and does not participate in short-circuit control flow, orthogonal to `and`/`or` keywords (short-circuit) (RFC-010 authoritative definition).

> **Range binding power**: `..` has binding power (6, 7)—left 6 is below addition (7), right 7 consumes addition without consuming same-level `..`. Before/after comparison:
>
> | Expression       | Before (level 1, right associative)                                              | After ((6,7), left associative)                            |
> | ---------------- | -------------------------------------------------------------------------------- | ---------------------------------------------------------- |
> | `x in 1..10`     | `x in 1..10` (`in` right operand level 4, `..` level 1 can't consume, actually can't parse) | `x in (1..10)`—range as whole `in` right operand           |
> | `0..n+2`         | `(0..n)+2` (right-associative trap: upper bound eaten, `for` loop directly E3004) | `0..(n+2)`—upper bound is arithmetic expression           |
> | `a == b..c`      | `a == (b..c)` (`..` level 1 < `==` level 3, naturally whole)                    | `a == (b..c)`—**semantics unchanged**, `..` still higher than comparison |
> | `1..2*3`         | `(1..2)*3`                                                                       | `1..(2*3)`—upper bound is arithmetic expression           |
> | `a..b..c`        | `a..(b..c)` (right-associative chain, meaningless Range nesting)               | `(a..b)..c`—**step form** (`c` is step)                   |
>
> Net effect: composite upper bound `for i in 0..n+2` changes from "parses successfully but E3004" to "directly usable"; `x in 1..10` changes from "can't parse" to "range check"; `a..b..c` changes from "meaningless nesting" to "step component". Level 6 falls between `+` (level 5) and `<<` (level 7), following mathematical convention: range is a tightly binding construct, upper bound naturally is a complete arithmetic expression.

### 2.3 Function Calls

```
FnCall      ::= Expr '(' ArgList? ')'
ArgList     ::= Expr (',' Expr)* (',' NamedArg)* | NamedArg (',' NamedArg)*
NamedArg    ::= Identifier '=' Expr
```

Named arguments use `name = value` (RFC-010 §Function Definition, RFC-011 §Constructor Forms). Positional arguments must come before named arguments; order-specified parameters can be arranged arbitrarily within named arguments:

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b

add(3, 5)          // Positional
add(a = 3, b = 5)  // Named
add(b = 5, a = 3)  // Arbitrary order
add(3, b = 5)      // Mixed, positional first
```

Misspelled named arguments report **E1014**, same parameter specified by both positional and named reports **E1015**, incorrect count reports **E1010** (RFC-013).

### 2.4 Member Access

```
MemberAccess::= Expr '.' Identifier
```

### 2.5 Index Access

```
IndexAccess ::= Expr '[' Expr ']'
```

> **Three-layer semantics (RFC-011b)**: `a[i]` dispatches based on `a`'s type—① Built-in containers (List/Vec/Array/Dict/Tuple) use native index instructions (fast path); ② User types implementing `Index` dispatch to their `index` method (type body `Index(Grid, Int, Float)` instantiation + `Grid.index` method); ③ RFC-004 positional binding for `f[0]` only exists in binding declarations, not in this grammar. Multi-dimensional index `a[0, 1]` keys are packed as tuples. Other types not implementing `Index` are rejected at type level.

### 2.6 Type Conversion

```
TypeCast    ::= Expr 'as' TypeExpr
```

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

### 2.9 Block Expressions

```
Block       ::= '{' Stmt* Expr? '}'
```

> **Statement termination rules**: Separator and newline behavior between Stmts (`;` explicit separator, newline terminates, line continuation exception, line-start `(`/`[` never merges) are defined by [RFC-038](../../design/rfc/accepted/038-statement-termination.md).

#### 2.9.1 The Three Forms of `{`

`{` in expression position has exactly three interpretations, determined once by **content**:

| Form             | Notation               | Type         | Example              |
| ---------------- | ---------------------- | ------------ | -------------------- |
| **Empty block**  | `{}`                   | `Void`       | `x: Void = {}`       |
| **Dict literal** | `{ "k": v, ... }`      | `Dict(K, V)` | `d = { "a": 1 }`     |
| **Block**        | `{ Stmt* Expr? }`      | Tail expression type | `y = { 1 + 1 }` |

**Decision order**:

1. `{}` (no content) → **Empty block**, value `Void`
2. First element is `String ':' Expr` key-value pair → **Dict literal**
3. Everything else → **Block**, value given by tail expression

> **Why `{}` is not an empty dictionary**: An empty dictionary's "emptiness" cannot self-describe (it could be `Dict(K, V)` or an empty block), and the Dict grammar [§1.6.4](#_1-6-4-collections) requires at least one key-value pair. When there's no content to match against, the zero form of the block structure is taken: consistent with `unsafe {}` / `spawn {}`, no special case introduced. Empty dictionary uses `dict.new()`.
>
> **Why functions need annotations**: `f = { stmt }` is a **value** (tail expression type), not a function. To define a function, write the Fn annotation explicitly: `f: () -> Int = { 5 }`. This follows the same principle as dict: **type is determined by content**, not by presence or absence of annotations. (This rule is from [RFC-010a](../../design/rfc/accepted/010a-tail-expression-and-return.md) Appendix D.)

**Unified semantics**: All `{}` blocks' values are given by **tail expression**, `return` is a non-local exit of type `Never`.

| Block type       | Value exit | Empty `{}` |
| ---------------- | ---------- | ---------- |
| Regular `{}`     | Tail expr  | `Void`     |
| `unsafe {}`      | Tail expr  | `Void`     |
| `spawn {}`       | Tail expr  | `Void`     |

**Core principles** (details in [RFC-010a](../../design/rfc/accepted/010a-tail-expression-and-return.md)):

- **Block value = tail expression** (last expression), sole exit point, no exceptions
- When the last position is an **assignment statement**, block value is `Void`; if you want `Void`, write it explicitly
- **`return` exits the nearest function boundary** (pierces through all blocks, does not "return to a block"), type `Never`; `Never <: T` holds for any type (ex falsum principle), so `return` can appear in any return type position
- Expression form `= expr` directly gives the value

```yaoxiang
// Regular {} block: tail expression gives value
result = {
    x = compute()
    x                // block value
}

// unsafe {} block: tail expression gives type definition
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void
    }
    SqliteDb         // block value
}

// spawn {} block: tail expression gives result
(a, b) = spawn {
    result1 = fetch("url1"),
    result2 = fetch("url2")
    (result1, result2)   // block value
}

// return: pierces through blocks, exits function
f: (n: Int) -> Int = {
    if n < 0 {
        return 0     // pierces through if and function body, exits function
    }
    n * 2            // Tail expression
}
```

#### Is `name = { ... }` a function or a block value?

`name = { ... }` can be either a function definition (RFC-007 "zero-parameter simplest form") or a block value binding. Decision follows **annotation first, default to function** (RFC-010a Appendix D):

| Situation              | Result       | Example                                 |
| ---------------------- | ------------ | --------------------------------------- |
| Value is Lambda (`=>`) | Function     | `f = () => 5` → `f()` = 5              |
| Annotation is function type | Function | `f: () -> Int = { 5 }` → `f()` = 5 |
| Annotation is non-function type | **Block value** | `x: Int = { 5 }` → `x` = 5 |
| No annotation          | Function     | `f = { 5 }` → `f()` = 5                |

**Annotation is type**: `x: Int = ...` declares `x` is `Int`, so `{ ... }` evaluates to `Int`; `f: () -> Int = ...` declares `f` is a function, so `{ ... }` is the function body.

If you want `{ ... }` to evaluate immediately, **just write the target type** (no new syntax needed):

```yaoxiang
// Block value: evaluate immediately
x: Int = {
    y = 5
    y            // x = 5
}

// Function: no annotation defaults to function
f = { 5 }        // f() = 5
```

#### Nested function types: currying or returning functions?

Nested function types to the right of `->` have two readings, distinguished by **parentheses** (RFC-004):

| Syntax                              | Meaning            | Call           |
| ----------------------------------- | ------------------ | -------------- |
| `(a: Int) -> (b: Int) -> Int`       | **Curried**        | `f(1)(2)`      |
| `(a: Int) -> ((b: Int) -> Int)`     | **Returning function** | `g(1)` returns a function |

Basis: **annotation is type**. `g: (a: Int) -> ((b: Int) -> Int)` declares `g(1) : (b: Int) -> Int`, so `g(1)` must **be** that function, not "the next segment of parameters".

```yaoxiang
// Curried: two segments of parameters given layer by layer
add: (a: Int) -> (b: Int) -> Int = { a + b }
add(1)(2)        // → 3

// Returning function: outer segment returns the inner function
adder: (n: Int) -> ((x: Int) -> Int) = (x) => x + n
adder(10)(5)     // → 15
h = adder(10)    // Returned function can be stored in variables, passed around
```

Unparenthesized nested `Fn` is always curried (including RFC-011 type parameter forms):

```yaoxiang
identity: (T: Type) -> (x: T) -> T = (x) => x   // Curried
identity(5)      // → 5
```

**Type checking**: Parentheses declare return type, body must produce that type. `f: () -> (() -> Int) = { 7 }` is an error—expects `() -> Int`, actually gets `Int` (E1002).

### 2.10 Lambda Expressions

```
Lambda      ::= '(' ParamList? ')' '=>' Expr
            |  '(' ParamList? ')' '=>' Block
```

### 2.11 Error Propagation Operator

```
ErrorPropagate ::= Expr '?'
```

`?` is a postfix operator with the same precedence as `.`. For `Result(T, E)` type:

- `Ok(v)` extracts value `v` and continues execution
- `Err(e)` propagates the error upward (`return Err(e)`)

```yaoxiang
process: (data: Data) -> Result(Data, Error) = {
    validated = validate(data)?     // Extract value on success, propagate on failure
    transform(validated)
}
```

### 2.12 Range Expressions

```
RangeExpr   ::= Expr '..' Expr ('..' Expr)?
```

`..` creates range values (Range is a first-class value, not syntax sugar).

```yaoxiang
for i in 0..10 { print(i) }
slice = array[0..5]

// Range is a value: binding, passing, membership test
r = 1..10
assert.assert(5 in r, "membership")
for i in r { print(i) }

// step form (third component, default 1)
for i in 0..10..2 { print(i) }  // 0, 2, 4, 6, 8
for i in 10..0..(-2) { print(i) }  // 10, 8, 6, 4, 2
```

> **Step semantics**: In `a..b..c`, `c` is the step. `c = 0` literal rejected at compile time; dynamic `c` runtime zero check (E6001 family; promoted to Result when error system is landed). `c < 0` is valid, range direction reverses with sign (`10..0..(-2)` decrements).

### 2.13 ref Expression

```
RefExpr     ::= 'ref' Expr
```

`ref` creates shared ownership. The compiler automatically chooses Rc (single-task) or Arc (cross-task); users don't need to care about implementation details.

```yaoxiang
data = ref heavy_data
spawn { use(data) }   // Cross-task: compiler automatically chooses Arc
```

### 2.14 unsafe Expression

```
UnsafeExpr  ::= 'unsafe' Block
```

`unsafe` blocks are used to define opaque types and operate on raw pointers. Use `return` to pass type definitions to the outer scope.

**Semantics**:

- Types and raw pointers can be defined inside `unsafe {}`
- Returned types are usable outside `unsafe {}`
- Field access on types requires unsafe permission

```yaoxiang
// Define opaque type inside unsafe block
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  // Raw pointer
    }
    return SqliteDb
}

// SqliteDb usable outside unsafe block
db = sqlite3_open("test.db")
```

### 2.15 Scopes

**Basic rules**:

- Each `{}` block creates a scope
- Inner scopes can access variables from outer scopes
- Outer scopes cannot access variables from inner scopes
- Variable declaration follows the "assignment preference" principle

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

**Variable declaration and shadowing**:

- `x = value`: searches outward along scope chain for x; assigns if found, declares new if not
- `mut x = value`: explicitly new mutable declaration, forbidden to shadow outer same name
- Any name in the same scope can only be declared once

> **Full definition**: Complete rules for scopes, variable declaration, and shadowing are detailed in [Module System Specification](./modules.md#chapter-4-scopes).

---

## Chapter 3: Statements

### 3.1 Statement Classification

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

**Semantics**: `return` is a **non-local exit** that exits the nearest **function boundary** (piercing through all blocks—including `if` / `while` / `for` / `match` / bare blocks / `spawn` / `unsafe`) and delivers the value to the caller. It does **not "return to a block"**.

**Type**: `return e : Never` (where `e : T`). `Never <: T'` holds for any `T'` (ex falsum principle, see [type system §2.2](./type-system.md)), so `return` can appear in any return type position without additional rule constraints.

**Relation to block evaluation**: Block value is always the **tail expression** (see §2.9). `{ return n }` as a block, its value is `n`, type `Never`; simultaneously `return` exits the function. **Both are true simultaneously**, coexisting through the ex falsum principle.

`return` and tail expression together enable "early return" without needing special rules for `return` to reference the function specifically—see [RFC-010a](../../design/rfc/accepted/010a-tail-expression-and-return.md).

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 {
        return 1          // Pierces through if, exits function (type Never)
    }
    n * factorial(n - 1)  // Tail expression = block value
}
```

### 3.4 break Statement

```
BreakStmt   ::= 'break'
```

**Semantics**: Immediately terminates the innermost enclosing `while`/`for` loop; control flow moves to after that loop body.

- **Exits only one level**: `break` always applies to the innermost loop containing it. When needing to exit multiple levels in nested loops, extract the inner loop into a function and use `return`, or use a flag (break/continue have no labels; if loop labels are introduced in the future, syntax will follow RFC process, decided together with multi-exit design for proof pipeline)
- **Only in loop bodies**: `break` can only appear inside `while`/`for` loop bodies (including nested blocks/if/match within the body); appearing outside a loop is a compile error (E1102 `'break' outside of a loop`)
- **Does not affect termination proofs**: `break` does not participate in termination arguments—it neither provides a measure nor forms a decreasing step of a measure; the loop's termination obligation is unrelated to `break`, triggered by refinement types (see [type-system §8.4](./type-system.md#84-terminates-termination-measure-predicate))
- **Borrow semantics**: break's control flow edge participates in structural cut of RFC-009a reverse BFS liveness analysis (the iterated iteration being broken does not participate in back-edge liveness derivation)

```yaoxiang
mut i = 0
while i < 10 {
    i = i + 1
    if i == 3 {
        break              // Control flows to after the loop, i == 3
    }
}

// Nested loops: break exits only inner loop
while j < 3 {
    while k < 10 {
        if k == 2 { break }    // Terminates only inner loop
    }
    j = j + 1                  // Each outer iteration executes this
}
```

### 3.5 continue Statement

```
ContinueStmt::= 'continue'
```

**Semantics**: Skips remaining statements in the current iteration and proceeds directly to the next round of the innermost enclosing loop—`while` returns to condition re-evaluation, `for` takes the next element.

- **Applies only to nearest level**: Same as `break`, no labels
- **Only in loop bodies**: Appearing outside a loop is a compile error (E1102)

```yaoxiang
mut sum = 0
mut n = 0
while n < 5 {
    n = n + 1
    if n == 3 {
        continue           // Skip the addition below, n == 3 not counted
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

#### 3.9.1 Semantics: Each iteration binds a new value

YaoXiang's for loop semantics differ from traditional languages: **each iteration binds a new value, rather than modifying the same variable**.

```yaoxiang
// Example: for i in 1..5
for i in 1..5 {
    print(i)
}
```

**Execution process**:

| Iteration | Behavior of loop variable                                            |
| --------- | -------------------------------------------------------------------- |
| First     | Creates new binding `i = 1`, executes loop body, prints 1           |
| Second    | Creates new binding `i = 2` (previous binding destroyed), executes body, prints 2 |
| Third     | Creates new binding `i = 3`, executes body, prints 3               |
| Fourth    | Creates new binding `i = 4`, executes body, prints 4                 |
| End       | Loop body ends, binding destroyed                                    |

**Key point**: After each iteration ends, the binding created during that iteration is destroyed. The next iteration is a completely fresh binding with no relation to the previous iteration's binding.

#### 3.9.2 Difference between for and for mut

| Syntax                | Loop variable mutability | Description                 |
| --------------------- | ------------------------ | --------------------------- |
| `for i in 1..5`       | Immutable                | Cannot modify binding in loop body |
| `for mut i in 1..5`  | Mutable                  | Can modify binding in loop body |

```yaoxiang
// Valid: each iteration binds new value, no need to modify
for i in 1..5 {
    print(i)  // Read i's value
}

// Invalid: immutable binding, cannot modify
for i in 1..5 {
    i = i + 1  // Error: cannot modify immutable binding
}

// Valid: using for mut allows modification
for mut i in 1..5 {
    i = i + 1  // Allowed
}
```

#### 3.9.3 Shadowing Check

YaoXiang forbids variable shadowing. Loop variables cannot have the same name as variables in outer scopes:

```yaoxiang
// Invalid: i already declared outside
i = 10
for i in 1..5 {
    print(i)
}

// Correct: use different variable name
i = 10
for j in 1..5 {
    print(j)
}
```

This rule applies to all code blocks, see [4.3 Shadowing Rules](./modules.md#43-shadowing-rules).

#### 3.9.4 Comparison with Other Languages

| Language   | for loop variable semantics                 |
| ---------- | ------------------------------------------- |
| YaoXiang   | Each iteration binds a new value            |
| Rust       | Modifies the same variable (requires mut)   |
| Python     | Modifies the same variable (no mut needed)  |
| C/C++      | Modifies the same variable (requires pointer or reference) |

**Design rationale**: YaoXiang uses binding semantics because:

1. **More natural semantics** In natural language, "for each element x in a set" means each x is an independent entity. YaoXiang's `for i in 1..5` reads as "for each i in 1 to 5", and each iteration's i is a completely new binding—this aligns with human intuitive understanding.

2. **Avoids accidental modification** Default immutable binding semantics mean the loop body cannot accidentally modify the loop variable. No need to worry about accidentally writing `i = ...` somewhere in a complex loop body causing hard-to-trace bugs.

3. **High-performance solutions are straightforward** When truly needing to reuse a variable across iterations (e.g., accumulator, cache), simply use `for mut` to switch to mutable binding mode. This is clearer than implicit shared state—intent is expressed explicitly through syntax rather than hidden in runtime behavior.

### 3.10 spawn Statement

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' 'mut'? Identifier 'in' Expr '{' Expr '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
```

**spawn block**: Explicitly declares concurrency boundary; expressions inside the block execute concurrently.

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

**spawn block captures outer variables** (RFC-024 §2.3, value capture semantics):

- Block body referencing outer variables = **Move value capture**: value is snapshotted into closure environment at spawn creation point, block body reads via env (LoadUpvalue)
- **Primitives** (Int/Float/Bool/Char) are copied by value, outer variables unaffected
- **Handle types** (Struct/String/List etc.) snapshot = handle copy, sharing underlying object; Embedded runtime (default) is same thread and heap, handle valid
- Sharing between multiple tasks requires explicit `ref` (§2.13, compiler auto-chooses Rc/Arc)
- Outer variables referenced by `return` inside block are also captured

```yaoxiang
t1 = 1 + 1
t2 = 2 + 2
result = spawn {
    return t1 + t2    // t1/t2 value captured, result == 6
}
```

---

### 3.11 Program Entry and Top-level Statements

A source file has two roles from the compiler's perspective, **determined by whether `yaoxiang.toml` exists**:

| Role        | Determination                        | Program body                                             |
| ----------- | ------------------------------------ | -------------------------------------------------------- |
| **Script**  | Single file runs directly, no `yaoxiang.toml` | **Top-level statements** (executed in writing order); `main` is a regular binding |
| **Bin**     | `yaoxiang.toml` exists               | **`main` function**; top level must not have executable statements |

#### Script: Top-level statements are the program

Without a manifest, the file is a "script", **top-level statements execute in writing order**:

```yaoxiang
use std.io
io.println("hello")          // Execute directly
x: Int = { 42 }              // Top-level binding: runtime initialization
io.println(x)                // 42
```

`main` in this mode is **not special**—it's just a regular binding. To make `main` run, you must call it explicitly:

```yaoxiang
use std.io
main: () -> Void = { io.println("only runs if called") }

main()                       // ← Must write this line
```

> **Why not auto-call `main`?** Top-level statements are already the program body. If `main` were also implicitly called, scripts that explicitly write `main()` would execute twice. The two rules cannot coexist, so in Script mode there's only one execution entry: "top-level statements".

#### Bin: `main` is the entry point

With a manifest, the file is an "executable target", and:

- `main` must be defined, and it must be a function (signature must allow zero-argument call)
- Top level allows no executable statements—the program body is `main`

```yaoxiang
main: () -> Void = {
    print("hello")
}
```

Missing `main` or `main` not being a function are compile errors (former: all functions unreachable when no `main`; latter: value bindings are not called).

> **Library files**: Files that are `use`d by other files, or files pointed to by `[lib].path` / `[exports]`, do not require `main`—they are not program entry points.

#### Top-level binding initialization

Top-level binding initializers are **evaluated at runtime**, not required to be compile-time constants:

```yaoxiang
answer: Int = { 42 }              // Block value
inc: (Int) -> Int = (x) => x + 1
computed: Int = inc(41)           // Function call
```

Initialization executes in **dependency order**, unrelated to writing order:

```yaoxiang
derived: Int = base * 3           // References base declared later
base: Int = 7                     // Initialized first (topological sort)
```

Circular dependency is a compile error (lists names in the cycle):

```yaoxiang
a: Int = b + 1
b: Int = a + 1                    // Error: a → b → a
```

> **Design rationale**: RFC-029f (file role model), RFC-010a Appendix D (block binding arbitration).

---

## Appendix: Syntax Quick Reference

### A.1 Control Flow

```
if Expr Block (else if Expr Block)* (else Block)?
match Expr { MatchArm+ }
while Expr Block
for 'mut'? Identifier 'in' Expr Block
break | continue          // Only inside loops (§3.4 / §3.5)
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