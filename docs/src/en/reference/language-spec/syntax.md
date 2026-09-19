# Syntax Specification

This document defines the syntax specification of the YaoXiang programming language, including
lexical structure, grammar rules, and operator precedence.

---

## Chapter 1: Lexical Structure

### 1.1 Source Files

YaoXiang source files must use UTF-8 encoding. Source files typically have the extension `.yx`.

### 1.2 Lexical Token Categories

| Category   | Description                        | Examples                  |
| ---------- | ---------------------------------- | ------------------------- |
| Identifier | Starts with letter or underscore   | `x`, `_private`, `my_var` |
| Keyword    | Language predefined reserved words | `Type`, `pub`, `use`      |
| Literal    | Fixed value                        | `42`, `"hello"`, `true`   |
| Operator   | Operation symbols                  | `+`, `-`, `*`, `/`        |
| Delimiter  | Syntax separators                  | `(`, `)`, `{`, `}`, `,`   |

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

YaoXiang's "reserved words" are organized into three layers, identified at different stages by the
parser and the type checker:

#### 1.4.1 Literal Reserved Words

The parser has independent tokens for literal identifiers, which cannot be used as ordinary
identifiers:

| Identifier | Belongs to Type | Description                                                                                                   |
| ---------- | --------------- | ------------------------------------------------------------------------------------------------------------- |
| `Type`     | —               | Meta-type keyword                                                                                             |
| `true`     | Bool            | Boolean true value                                                                                            |
| `false`    | Bool            | Boolean false value                                                                                           |
| `void`     | Void            | Void literal (Unit value). Lowercase `void` is a value literal; uppercase `Void` is a type name (see §1.4.3). |

#### 1.4.2 Constructor Expressions

The following constructors are recognized by the parser in pattern matching and expression contexts:

| Constructor | Belongs to Type | Description                       |
| ----------- | --------------- | --------------------------------- |
| `some(T)`   | Option          | Option value variant construction |
| `ok(T)`     | Result          | Result success variant            |
| `err(E)`    | Result          | Result error variant              |

#### 1.4.3 Built-in Type Names

The following type names are pre-registered by the type checker and can be used in type positions
without import. The parser treats them as ordinary identifiers—**not reserved words, can be shadowed
by local bindings (not recommended)**.

| Type Name | Logical Correspondence | Description                                                                                                                                      |
| --------- | ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| `Void`    | ⊤ (true/Unit)          | Zero-field product type with exactly one inhabitant (`void` literal, see §1.4.1)                                                                 |
| `Never`   | ⊥ (false/empty type)   | Zero-variant sum type with zero inhabitants. No expression can produce a `Never` value. `Never <: T` holds for all `T` (principle of explosion). |
| `Int`     | —                      | Signed integer                                                                                                                                   |
| `Float`   | —                      | Floating-point number                                                                                                                            |
| `Bool`    | —                      | Boolean value: `true` / `false`                                                                                                                  |
| `Char`    | —                      | Unicode character                                                                                                                                |
| `String`  | —                      | String                                                                                                                                           |

### 1.5 Identifiers

Identifiers start with a letter or underscore, and subsequent characters can be letters, digits, or
underscores. Identifiers are case-sensitive.

Special identifiers:

- `_` is used as a placeholder, indicating an ignored value
- Identifiers starting with an underscore indicate private members

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
Array       ::= '[' Expr (',' Expr)* ']'   // When target type annotation is Array(T, N), the literal resolves to a fixed-length array
```

> **Dictionary literals require at least one key-value pair**: `{}` is **not** an empty
> dictionary—it is an empty block (value `Void`, see [§2.9](#_2-9-block-expressions)). For an empty
> dictionary, use the constructor `dict.new()`:
>
> ```yaoxiang
> empty = dict.new()                  // ✅ empty dictionary
> d = { "a": 1 }                       // ✅ dictionary literal
> wrong = {}                           // ❌ this is not a dictionary, but an empty block (Void)
> ```
>
> **The criterion is the content**: The `Dict` grammar requires at least one `String ':' Expr`, and
> `{}` has no content to rely on, so it takes the zero form of block structure. Non-empty forms are
> self-describing by content (`{ "k": v }` has key-value pairs → dictionary)—this is consistent with
> `f = { 5 }` being an `Int` value rather than a function: **type is determined by content**.

> Set has no literal grammar and no runtime representation—in the collection type planning, when the
> need arises, it will be completed following the Dict pattern (std.set + HeapValue::Set). The
> resolution of List/Dict literals is determined by the context type annotation: bare literals and
> `List(T)` annotations resolve to growable lists; `Array(T, N)` annotations directly applied to
> literals resolve to fixed-length arrays. Implicit List→Array conversion is forbidden.
>
> Array literal semantics:
>
> - The number of elements must equal N; otherwise compile-time E1002; an empty literal with
>   non-zero N is also rejected
> - Each element's type must be compatible with T; otherwise compile-time E1002
> - The grammar form of N: only integer literals (possibly negative) or constant names; compound
>   expressions (such as `2+1`) are rejected at parse time
> - When N is a symbolic constant (function const parameter, such as `Array(Int, n)`), the count
>   check is deferred to the refinement type stage
> - v1 nested array literals (`Array(Array(Int,2),2) = [[1,2],[3,4]]`) are rejected at compile time
>   and require explicit construction layer by layer; recursive resolution is reserved for future
>   versions

#### 1.6.5 List Comprehension

```
ListComp    ::= '[' Expr 'for' Identifier 'in' Expr (',' Expr)* ('if' Expr)? ']'
```

> **Behavior tightening (migration record)**: The grammar of the iteration variable was originally
> `'for' Identifier 'in'`, but in the old implementation, patterns went through full pratt
> parsing—after `'in'` was registered as an infix operator, `x` would consume `in items` as a
> membership expression. After the fix, non-identifier patterns fail to parse directly, no longer
> falling back to `_` (silently swallowing errors) like the old implementation. Impact: the previous
> parseable form `[x for (a, b) in pairs]` now reports an error—this form never had defined behavior
> (the variable was always `_`), the tightening direction is correct, and there is no semantic
> migration cost.

#### 1.6.6 Membership Test

```
Membership  ::= Expr 'in' Expr
```

> `in` is a binary relational operator that returns `Bool`—hit returns `true`, miss returns `false`,
> no error. Semantic division: `[]` asserts existence and retrieves a value (errors on failure),
> `in` asks whether it exists (miss is a normal `false`). Right operand coverage: List / Array /
> Dict (key set) / Tuple / String (substring) / Range (interval). `in` is a first-class Hoare
> predicate, serving as the base for compile-time provable propositions during the refinement type
> stage. (Set is removed from the right operand list—Set has no runtime representation, see §1.6.4)

### 1.7 Comments

```
// Single-line comment

/* Multi-line comment
   can span multiple lines */
```

### 1.8 Indentation Rules

Code must use 4 spaces for indentation; Tab characters are forbidden. This is a mandatory syntax
rule.

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
> higher than all binary operators. Therefore `!a == b` ≡ `(!a) == b` (Zig-style semantics); `!` is
> a pure unary operation that does not participate in short-circuit control flow, and is orthogonal
> to the `and`/`or` keywords (short-circuit) (authoritative definition in RFC-010).

> **Range binding force**: `..` has binding force (6, 7)—left 6 is lower than addition (7), right 7
> consumes addition but not the same-level `..`. Comparison before and after the change:
>
> | Expression   | Before change (level 1, right-associative)                                                   | After change ((6,7), left-associative)                                            |
> | ------------ | -------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
> | `x in 1..10` | `x in 1..10` (`in` right operand level 4, `..` level 1 cannot consume, actually unparseable) | `x in (1..10)`—the interval as a whole is the right operand of `in`               |
> | `0..n+2`     | `(0..n)+2` (right-associative trap: upper bound is consumed, `for` loop directly E3004)      | `0..(n+2)`—upper bound is an arithmetic expression                                |
> | `a == b..c`  | `a == (b..c)` (`..` level 1 < `==` level 3, naturally as a whole)                            | `a == (b..c)`—**semantics unchanged**, `..` is still higher than comparison level |
> | `1..2*3`     | `(1..2)*3`                                                                                   | `1..(2*3)`—upper bound is an arithmetic expression                                |
> | `a..b..c`    | `a..(b..c)` (right-associative chain, meaningless Range nested in Range)                     | `(a..b)..c`—**step form** (`c` is the step)                                       |
>
> Net effect: the composite upper bound `for i in 0..n+2` changed from "parses successfully but
> E3004" to "directly usable"; `x in 1..10` changed from "unparseable" to "interval check";
> `a..b..c` changed from "meaningless nesting" to "step component". Level 6 falls between `+`
> (level 5) and `<<` (level 7), mathematical convention: an interval is a tightly-bound construct,
> and the upper bound is naturally a complete arithmetic expression.

### 2.3 Function Calls

```
FnCall      ::= Expr '(' ArgList? ')'
ArgList     ::= Expr (',' Expr)* (',' NamedArg)* | NamedArg (',' NamedArg)*
NamedArg    ::= Identifier ':' Expr
```

### 2.4 Member Access

```
MemberAccess::= Expr '.' Identifier
```

### 2.5 Index Access

```
IndexAccess ::= Expr '[' Expr ']'
```

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

### 2.9 Block Expression

```
Block       ::= '{' Stmt* Expr? '}'
```

> **Statement termination rules**: The separation and newline behavior between Stmts (explicit `;`
> separation, newline termination, line continuation exceptions, leading `(`/`[` never merging) is
> defined by [RFC-038](../design/rfc/draft/038-statement-termination.md).

#### 2.9.1 Three Forms of `{`

`{` in an expression position has exactly three interpretations, determined once by the **content**:

| Form                   | Notation          | Type                 | Example          |
| ---------------------- | ----------------- | -------------------- | ---------------- |
| **Empty block**        | `{}`              | `Void`               | `x: Void = {}`   |
| **Dictionary literal** | `{ "k": v, ... }` | `Dict(K, V)`         | `d = { "a": 1 }` |
| **Block**              | `{ Stmt* Expr? }` | Tail expression type | `y = { 1 + 1 }`  |

**Determination order**:

1. `{}` (no content) → **Empty block**, value `Void`
2. The first element is a `String ':' Expr` key-value pair → **Dictionary literal**
3. Otherwise → **Block**, value given by the tail expression

> **Why `{}` is not an empty dictionary**: The "emptiness" of an empty dictionary cannot
> self-describe (it can be either `Dict(K, V)` or an empty block), and the dictionary grammar
> [§1.6.4](#_1-6-4-collections) requires at least one key-value pair. When there is no content to
> rely on, the zero form of block structure is taken: this is consistent with `unsafe {}` /
> `spawn {}`, without introducing special cases. For an empty dictionary, use `dict.new()`.
>
> **Why functions need annotations**: `f = { stmt }` is a **value** (tail expression type), not a
> function. To define a function, write the Fn annotation explicitly: `f: () -> Int = { 5 }`. This
> is the same principle as dictionaries: **type is determined by content**, not by the existence of
> an annotation. (This rule supersedes the ruling C of Appendix D of RFC-010a "no annotation →
> default function", see its errata.)

**Unified semantics**: The value of all `{}` blocks is given by the **tail expression**, and
`return` is a non-local exit of type `Never`.

| Block Type   | Value Outlet    | Empty block `{}` |
| ------------ | --------------- | ---------------- |
| Regular `{}` | Tail expression | `Void`           |
| `unsafe {}`  | Tail expression | `Void`           |
| `spawn {}`   | Tail expression | `Void`           |

**Core principles** (see [RFC-010a](../design/rfc/accepted/010a-tail-expression-and-return.md) for
details):

- **Block value = tail expression** (the last expression), the only outlet, no exceptions
- When the last position is an **assignment statement**, the block value is `Void`; to get `Void`,
  write `Void` explicitly
- **`return` exits the nearest function boundary** (passing through all blocks, does not "return to
  the block"), type `Never`; `Never <: T` holds for any type (principle of explosion), so it can
  appear at any return type position
- The expression form `= expr` directly gives the value

```yaoxiang
// Regular {} block: tail expression gives the value
result = {
    x = compute()
    x                // block value
}

// unsafe {} block: tail expression gives the type definition
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void
    }
    SqliteDb         // block value
}

// spawn {} block: tail expression gives the result
(a, b) = spawn {
    result1 = fetch("url1"),
    result2 = fetch("url2")
    (result1, result2)   // block value
}

// return: passes through blocks, exits the function
f: (n: Int) -> Int = {
    if n < 0 {
        return 0     // passes out of if and function body, exits the function
    }
    n * 2            // tail expression
}
```

#### Is `name = { ... }` a function or a block value?

`name = { ... }` could be either a function definition (RFC-007 "zero-arg simplest") or a block
value binding. According to the ruling "annotation priority, default function" (RFC-010a Appendix
D):

| Situation                       | Result          | Example                            |
| ------------------------------- | --------------- | ---------------------------------- |
| Value is Lambda (`=>`)          | Function        | `f = () => 5` → `f()` = 5          |
| Annotation is function type     | Function        | `f: () -> Int = { 5 }` → `f()` = 5 |
| Annotation is non-function type | **Block value** | `x: Int = { 5 }` → `x` = 5         |
| No annotation                   | Function        | `f = { 5 }` → `f()` = 5            |

**Annotation is type**: `x: Int = ...` declares `x` to be `Int`, so `{ ... }` evaluates to `Int`;
`f: () -> Int = ...` declares `f` to be a function, so `{ ... }` is the function body.

To evaluate `{ ... }` immediately, **just write the target type** (no new syntax needed):

```yaoxiang
// Block value: evaluate immediately
x: Int = {
    y = 5
    y            // x = 5
}

// Function: no annotation defaults to function
f = { 5 }        // f() = 5
```

#### Nested Function Types: Currying or Returning a Function?

The nested function type on the right side of `->` has two readings, distinguished by
**parentheses** (RFC-004):

| Writing                         | Meaning                  | Call                   |
| ------------------------------- | ------------------------ | ---------------------- |
| `(a: Int) -> (b: Int) -> Int`   | **Currying**             | `f(1)(2)`              |
| `(a: Int) -> ((b: Int) -> Int)` | **Returning a function** | `g(1)` gets a function |

Basis: **annotation is type**. `g: (a: Int) -> ((b: Int) -> Int)` declares `g(1) : (b: Int) -> Int`,
so `g(1)` must **be** that function, not "the next segment of arguments".

```yaoxiang
// Currying: two segments of arguments given layer by layer
add: (a: Int) -> (b: Int) -> Int = { a + b }
add(1)(2)        // → 3

// Returning a function: outer segment of arguments, the returned one is the function
adder: (n: Int) -> ((x: Int) -> Int) = (x) => x + n
adder(10)(5)     // → 15
h = adder(10)    // the returned function can be stored in a variable, passed around
```

Unparenthesized nested `Fn` is always currying (including the type parameter form of RFC-011):

```yaoxiang
identity: (T: Type) -> (x: T) -> T = (x) => x   // currying
identity(5)      // → 5
```

**Type checking**: The parentheses declare the return type, the body must produce that type.
`f: () -> (() -> Int) = { 7 }` is an error—expected `() -> Int`, actually got `Int` (E1002).

### 2.10 Lambda Expression

```
Lambda      ::= '(' ParamList? ')' '=>' Expr
            |  '(' ParamList? ')' '=>' Block
```

### 2.11 Error Propagation Operator

```
ErrorPropagate ::= Expr '?'
```

The `?` operator is a postfix operator with the same precedence as `.`. For `Result(T, E)` type:

- When `Ok(v)`, extract value `v` and continue execution
- When `Err(e)`, propagate the error upward (`return Err(e)`)

```yaoxiang
process: (data: Data) -> Result(Data, Error) = {
    validated = validate(data)?     // extract value on success, propagate on failure
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

// Range is a value: bind, pass, membership check
r = 1..10
assert.assert(5 in r, "membership")
for i in r { print(i) }

// step form (third component, default 1)
for i in 0..10..2 { print(i) }  // 0, 2, 4, 6, 8
for i in 10..0..(-2) { print(i) }  // 10, 8, 6, 4, 2
```

> **step semantics**: In `a..b..c`, `c` is the step. `c = 0` literal is rejected at compile time;
> dynamic `c` has zero runtime check (E6001 family; will be upgraded to Result after the error
> system lands). `c < 0` is legal, and the interval direction is reversed by sign (`10..0..(-2)` is
> descending).

### 2.13 ref Expression

```
RefExpr     ::= 'ref' Expr
```

`ref` creates a shared holding. The compiler automatically selects Rc (single-task) or Arc
(cross-task), users do not need to care about implementation details.

```yaoxiang
data = ref heavy_data
spawn { use(data) }   // cross-task: compiler automatically selects Arc
```

### 2.14 unsafe Expression

```
UnsafeExpr  ::= 'unsafe' Block
```

`unsafe` blocks are used to define opaque types and operate on raw pointers. Use `return` to return
the type definition to the outer scope.

**Semantics**:

- Types and raw pointer operations can be defined in `unsafe {}`
- The returned type is available outside `unsafe {}`
- Field access of the type requires unsafe permission

```yaoxiang
// Define an opaque type in an unsafe block
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  // raw pointer
    }
    return SqliteDb
}

// SqliteDb is available outside the unsafe block
db = sqlite3_open("test.db")
```

### 2.15 Scope

**Basic rules**:

- Each `{}` block creates a scope
- Inner scopes can access variables from outer scopes
- Outer scopes cannot access variables from inner scopes
- Variable declarations follow the "assignment priority" principle

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

- `x = value`: search outward along the scope chain for x, assign if found, declare new if not found
- `mut x = value`: explicit new mutable declaration, forbidden to have the same name as the outer
  scope
- Any name can only be declared once within the same scope

> **Detailed definition**: The complete rules of scope, variable declaration, and shadowing
> mechanism are detailed in [Module System Specification](./modules.md#chapter-4-scope).

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

**Semantics**: `return` is a **non-local exit**, exiting the nearest **function boundary** (passing
through all blocks—including `if` / `while` / `for` / `match` / bare blocks / `spawn` / `unsafe`),
handing the value to the caller. It does **not "return to the block"**.

**Type**: `return e : Never` (where `e : T`). `Never <: T'` holds for any `T'` (principle of
explosion, see [Type System §2.2](./type-system.md)), so `return` can appear at any return type
position without additional rule constraints.

**Relationship with block evaluation**: A block's value is always the **tail expression** (see
§2.9). `{ return n }` as a block has value `n`, type `Never`; at the same time, the effect of
`return` is to exit the function. **Both things hold simultaneously**, coexisting through the
principle of explosion.

`return` together with the tail expression make "early return" possible, without additional rules
that `return` specifically refers to the function—see
[RFC-010a](../design/rfc/accepted/010a-tail-expression-and-return.md).

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 {
        return 1          // passes out of if, exits function (type Never)
    }
    n * factorial(n - 1)  // tail expression = block value
}
```

### 3.4 break Statement

```
BreakStmt   ::= 'break'
```

**Semantics**: Immediately terminates the innermost `while`/`for` loop, with control flow
transferring to after the loop body.

- **Only exits the nearest layer**: `break` always acts on the innermost loop containing it. When
  nested loops need to jump out of multiple layers at once, extract the inner loop as a function and
  use `return`, or use a flag (break/continue has no label; if loop labels are introduced in the
  future, they will follow the loop declaration side syntax and go through the RFC process, decided
  together with the multi-exit design of the proof pipeline)
- **Only inside loop body**: `break` can only appear inside `while`/`for` loop bodies (including
  blocks/if/match nested in the body), appearing outside a loop is a compile error (E1102
  `'break' outside of a loop`)
- **Does not affect termination proof**: `while` loops must still be provably terminating (decreases
  metric); `break` does not participate in the termination argument, `while true { break }` will not
  be accepted
- **Borrow semantics**: The control flow edge of break participates in the structural cutting of the
  reverse BFS liveness analysis of RFC-009a (the iterations that jump out do not participate in the
  back-edge liveness derivation)

```yaoxiang
mut i = 0
while i < 10 {
    i = i + 1
    if i == 3 {
        break              // control flow transfers to after the loop, i == 3
    }
}

// Nested loops: break only exits the inner loop
while j < 3 {
    while k < 10 {
        if k == 2 { break }    // only terminates the inner loop
    }
    j = j + 1                  // each outer iteration will execute here
}
```

### 3.5 continue Statement

```
ContinueStmt::= 'continue'
```

**Semantics**: Skips the remaining statements in the current iteration and directly enters the next
round of the innermost loop—`while` returns to the condition recheck, `for` takes the next element.

- **Only acts on the nearest layer**: same as `break`, no label
- **Only inside loop body**: appearing outside a loop is a compile error (E1102)

```yaoxiang
mut sum = 0
mut n = 0
while n < 5 {
    n = n + 1
    if n == 3 {
        continue           // skip the accumulation below, n == 3 is not counted
    }
    sum = sum + n
}
// sum == 12（1 + 2 + 4 + 5）
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

#### 3.9.1 Semantics: Each Iteration is a New Value Binding

YaoXiang's for loop semantics differ from traditional languages: **each iteration is a new value
binding, not a modification of the same variable**.

```yaoxiang
// Example: for i in 1..5
for i in 1..5 {
    print(i)
}
```

**Execution process**:

| Iteration | Behavior of the loop variable                                                           |
| --------- | --------------------------------------------------------------------------------------- |
| 1st       | Create new binding `i = 1`, loop body executes, print 1                                 |
| 2nd       | Create new binding `i = 2` (previous binding is destroyed), loop body executes, print 2 |
| 3rd       | Create new binding `i = 3`, loop body executes, print 3                                 |
| 4th       | Create new binding `i = 4`, loop body executes, print 4                                 |
| End       | Loop body ends, binding is destroyed                                                    |

**Key point**: After each iteration ends, the binding created by that iteration is destroyed. The
next iteration is an entirely new binding, with no relationship to the previous iteration's binding.

#### 3.9.2 Difference between for and for mut

| Syntax              | Loop Variable Mutability | Description                                |
| ------------------- | ------------------------ | ------------------------------------------ |
| `for i in 1..5`     | Immutable                | Cannot modify the binding in the loop body |
| `for mut i in 1..5` | Mutable                  | Can modify the binding in the loop body    |

```yaoxiang
// Legal: each iteration binds a new value, no modification needed
for i in 1..5 {
    print(i)  // read the value of i
}

// Error: immutable binding, cannot modify
for i in 1..5 {
    i = i + 1  // error: cannot modify immutable binding
}

// Legal: use for mut to allow modifying the binding
for mut i in 1..5 {
    i = i + 1  // modification allowed
}
```

#### 3.9.3 Shadowing Check

YaoXiang forbids variable shadowing. The for loop variable cannot have the same name as a variable
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

This rule applies to all code blocks, see [4.3 Shadowing Rules](./modules.md#43-shadowing-rules) for
details.

#### 3.9.4 Comparison with Other Languages

| Language | for Loop Variable Semantics                           |
| -------- | ----------------------------------------------------- |
| YaoXiang | Each iteration binds a new value                      |
| Rust     | Modify the same variable (needs mut)                  |
| Python   | Modify the same variable (no mut needed)              |
| C/C++    | Modify the same variable (needs pointer or reference) |

**Design rationale**: YaoXiang adopts binding semantics because:

1. **More in line with natural semantics** In natural language, "for each element x in the set"
   means each x is an independent individual. YaoXiang's `for i in 1..5` reads as "for each i from 1
   to 5", and each iteration's i is an entirely new binding, which is consistent with human
   intuitive understanding.

2. **Avoid accidental modification** The default immutable binding semantics mean that the loop
   variable cannot be accidentally modified in the loop body. There is no need to worry about
   writing `i = ...` somewhere in a complex loop body and causing a hard-to-trace bug.

3. **High-performance solutions are within reach** When it is indeed necessary to reuse variables
   between iterations (such as accumulators, caches), use `for mut` to switch to mutable binding
   mode. This is clearer than implicit shared state—intent is explicitly expressed through syntax,
   not hidden in runtime behavior.

### 3.10 spawn Statement

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' 'mut'? Identifier 'in' Expr '{' Expr '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
```

**spawn block**: Explicitly declares a concurrent domain, expressions within the block execute
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

**spawn block captures outer variables** (RFC-024 §2.3, value capture semantics):

- Block body references outer variables = **Move value capture**: the value is snapshotted into the
  closure environment at the spawn creation point, and the block body reads through env
  (LoadUpvalue)
- **Primitives** (Int/Float/Bool/Char) are value-copied, outer variables are not affected
- **Handle types** (Struct/String/List etc.) snapshot = handle copy, sharing the underlying object;
  Embedded runtime (default) same thread same heap, handles are valid
- Sharing between multiple tasks requires explicit `ref` (§2.13, compiler automatically selects
  Rc/Arc)
- Outer variables referenced by `return` within the block are also captured

```yaoxiang
t1 = 1 + 1
t2 = 2 + 2
result = spawn {
    return t1 + t2    // t1/t2 value-captured, result == 6
}
```

---

### 3.11 Program Entry and Top-Level Statements

A source file has two roles in the compiler's eyes, **determined by whether `yaoxiang.toml`
exists**:

| Role       | Determination                                | Program Body                                                                     |
| ---------- | -------------------------------------------- | -------------------------------------------------------------------------------- |
| **Script** | Single file run directly, no `yaoxiang.toml` | **Top-level statements** (executed in writing order); `main` is a normal binding |
| **Bin**    | `yaoxiang.toml` exists                       | **`main` function**; top-level cannot have executable statements                 |

#### Script: Top-Level Statements are the Program

Without a manifest, the file is a "script", and **top-level statements are executed in writing
order**:

```yaoxiang
use std.io
io.println("hello")          // executes directly
x: Int = { 42 }              // top-level binding: initialized at runtime
io.println(x)                // 42
```

`main` in this mode **is not special**—it is just a normal binding. To make `main` run, you must
call it explicitly:

```yaoxiang
use std.io
main: () -> Void = { io.println("only runs if called") }

main()                       // ← must write this line
```

> **Why is `main` not called automatically?** The top-level statements are already the program body.
> If `main` were implicitly called, a script that explicitly writes `main()` would execute twice.
> The two rules cannot coexist, so under Script there is only "top-level statements" as the single
> execution entry.

#### Bin: `main` is the Entry

With a manifest, the file is an "executable target", in which case:

- `main` must be defined, and it must be a function (signature must be callable with zero arguments)
- Top-level does not allow executable statements—the program body is `main`

```yaoxiang
main: () -> Void = {
    print("hello")
}
```

Missing `main` or `main` not being a function is a compile error (the former: all functions are
unreachable when `main` is missing; the latter: value bindings will not be called).

> **Library files**: Files used by other files via `use`, or files pointed to by `[lib].path` /
> `[exports]` do not require `main`—they are not program entries.

#### Initialization of Top-Level Bindings

The initialization value of top-level bindings is **evaluated at runtime**, not required to be
compile-time constants:

```yaoxiang
answer: Int = { 42 }              // block value
inc: (Int) -> Int = (x) => x + 1
computed: Int = inc(41)           // function call
```

Initialization is executed in **dependency order**, independent of writing order:

```yaoxiang
derived: Int = base * 3           // references base declared later
base: Int = 7                     // initialized first (topological sort)
```

Circular dependencies are compile errors (the names on the cycle will be listed):

```yaoxiang
a: Int = b + 1
b: Int = a + 1                    // error: a → b → a
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
break | continue          // only inside loop body (§3.4 / §3.5)
```

### A.2 Error Handling

```
Expr '?'              // error propagation (Result type)
```

### A.3 match Syntax

```
match value {
    pattern1 => expr1,
    pattern2 if guard => expr2,
    _ => default_expr,
}
```
