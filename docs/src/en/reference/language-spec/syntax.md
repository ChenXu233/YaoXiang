# Syntax Specification

This document defines the syntax specification of the YaoXiang programming language, including
lexical structure, grammar rules, and operator precedence.

---

## Chapter 1: Lexical Structure

### 1.1 Source Files

YaoXiang source files must be encoded in UTF-8. Source files typically use the `.yx` extension.

### 1.2 Lexical Token Categories

| Category   | Description                        | Examples                  |
| ---------- | ---------------------------------- | ------------------------- |
| Identifier | Starts with a letter or underscore | `x`, `_private`, `my_var` |
| Keyword    | Language-defined reserved words    | `Type`, `pub`, `use`      |
| Literal    | Fixed value                        | `42`, `"hello"`, `true`   |
| Operator   | Operation symbols                  | `+`, `-`, `*`, `/`        |
| Separator  | Syntactic delimiters               | `(`, `)`, `{`, `}`, `,`   |

### 1.3 Keywords

YaoXiang defines a very small set of keywords:

```
pub    use    spawn
ref    mut    if     else
else   match  while  for    return
break  continue as     in     unsafe
```

These keywords have special meaning in all contexts and cannot be used as identifiers.

### 1.4 Reserved Words

YaoXiang's "reserved words" are organized in three layers, recognized at different stages by the
parser and type checker:

#### 1.4.1 Literal Reserved Words

Literal identifiers that the parser treats as independent tokens, and cannot be used as ordinary
identifiers:

| Identifier | Belongs To | Description                                                                                                       |
| ---------- | ---------- | ----------------------------------------------------------------------------------------------------------------- |
| `Type`     | —          | Meta-type keyword                                                                                                 |
| `true`     | Bool       | Boolean true value                                                                                                |
| `false`    | Bool       | Boolean false value                                                                                               |
| `void`     | Void       | Void literal (Unit value). Lowercase `void` is the value literal; uppercase `Void` is the type name (see §1.4.3). |

#### 1.4.2 Constructor Expressions

The following constructors are recognized by the parser in pattern matching and expression contexts:

| Constructor | Belongs To | Description                      |
| ----------- | ---------- | -------------------------------- |
| `some(T)`   | Option     | Option value variant constructor |
| `ok(T)`     | Result     | Result success variant           |
| `err(E)`    | Result     | Result error variant             |

#### 1.4.3 Built-in Type Names

The following type names are pre-registered by the type checker and can be used in type positions
without import. The parser treats them as ordinary identifiers—**they are not reserved words and can
be shadowed by local bindings (not recommended)**.

| Type Name | Logical Correspondence | Description                                                                                                                                       |
| --------- | ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Void`    | ⊤ (true/Unit)          | Zero-field product type, with exactly one inhabitant (the `void` literal, see §1.4.1)                                                             |
| `Never`   | ⊥ (false/empty type)   | Zero-variant sum type, with zero inhabitants. No expression can produce a `Never` value. `Never <: T` holds for all `T` (principle of explosion). |
| `Int`     | —                      | Signed integer                                                                                                                                    |
| `Float`   | —                      | Floating-point number                                                                                                                             |
| `Bool`    | —                      | Boolean value: `true` / `false`                                                                                                                   |
| `Char`    | —                      | Unicode character                                                                                                                                 |
| `String`  | —                      | String                                                                                                                                            |

### 1.5 Identifiers

Identifiers start with a letter or underscore; subsequent characters may be letters, digits, or
underscores. Identifiers are case-sensitive.

Special identifiers:

- `_` is used as a placeholder, meaning to ignore a value
- Identifiers starting with an underscore represent private members

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
Array       ::= '[' Expr (',' Expr)* ']'   // When target type is annotated as Array(T, N), the literal becomes a fixed-length array
```

> **Dictionary literals require at least one key-value pair**: `{}` is **not** an empty
> dictionary—it is an empty block (value `Void`, see [§2.9](#_2-9-block-expressions)). For an empty
> dictionary, use the constructor `dict.new()`:
>
> ```yaoxiang
> empty = dict.new()                  // ✅ empty dictionary
> d = { "a": 1 }                       // ✅ dictionary literal
> wrong = {}                           // ❌ This is not a dictionary; it is an empty block (Void)
> ```
>
> **The criterion is content**: The `Dict` grammar requires at least one `String ':' Expr`. Since
> `{}` has no content to go by, it takes the zero form of block structure. The non-empty form is
> self-describing by content (`{ "k": v }` has a key-value pair → dictionary)— this is in the same
> spirit as `f = { 5 }` being an `Int` value rather than a function: **the type is determined by the
> content**.

> Set has no literal grammar and no runtime representation—set types are planned; when the need
> arises, follow the Dict pattern to complete them (std.set + HeapValue::Set). The landing point of
> List/Dict literals is determined by the context type annotation: a bare literal and a `List(T)`
> annotation land as a growable list; an `Array(T, N)` annotation, when applied directly to the
> literal, lands as a fixed-length array. Implicit List→Array conversion is forbidden.
>
> Array literal semantics:
>
> - The number of elements must equal N; otherwise, compile-time E1002; an empty literal paired with
>   non-zero N is also rejected
> - The type of each element must be compatible with T; otherwise, compile-time E1002
> - Grammar form of N: integer literal only (may be negative) or constant name; compound expressions
>   (such as `2+1`) are rejected at parse time
> - When N is a symbolic constant (e.g., `Array(Int, n)` with `n` as a const parameter), the count
>   check is deferred to the refined type phase
> - Nested array literals in v1 (e.g., `Array(Array(Int,2),2) = [[1,2],[3,4]]`) are rejected at
>   compile time; explicit construction layer by layer is required; recursive landing is left to a
>   later version

#### 1.6.5 List Comprehensions

```
ListComp    ::= '[' Expr 'for' Identifier 'in' Expr (',' Expr)* ('if' Expr)? ']'
```

> **Behavior tightening (migration record)**: The iteration variable grammar was originally
> `'for' Identifier 'in'`, but in the old implementation the pattern went through full pratt
> parsing—after `'in'` was registered as an infix operator, `x` would swallow `in items` into a
> membership expression. After the fix, non-identifier patterns fail to parse directly, and no
> longer fall back to `_` (silently swallowing errors) as in the old implementation. Impact:
> expressions like `[x for (a, b) in pairs]` that could be parsed before now report errors—this form
> never had defined behavior (the variable was always `_`), so tightening is the right direction
> with no semantic migration cost.

#### 1.6.6 Membership Test

```
Membership  ::= Expr 'in' Expr
```

> `in` is a binary relational operator that returns `Bool`—`true` on hit, `false` on miss, no error.
> Semantic split: `[]` asserts existence and retrieves a value (error on failure), `in` asks whether
> something exists (miss is a normal `false`). Right operand coverage: List / Array / Dict (key set)
> / Tuple / String (substring) / Range (interval). `in` is a first-class Hoare predicate, serving as
> the base of compile-time provable propositions in the refined type phase. (Set is removed from the
> right operand list—Set has no runtime representation, see §1.6.4)

### 1.7 Comments

```
// single-line comment

/* multi-line comment
   can span multiple lines */
```

### 1.8 Indentation Rules

Code must use 4 spaces for indentation; Tab characters are forbidden. This is a mandatory syntactic
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
| 1          | `()` `[]` `.` `?`           | Left-to-right |
| 2          | `as`                        | Left-to-right |
| 3          | Unary prefix `!` `-` `+`    | Right-to-left |
| 4          | `*` `/` `%`                 | Left-to-right |
| 5          | `+` `-`                     | Left-to-right |
| 6          | `..`                        | Left-to-right |
| 7          | `<<` `>>`                   | Left-to-right |
| 8          | `&` `\|` `^`                | Left-to-right |
| 9          | `==` `!=` `<` `>` `<=` `>=` | Left-to-right |
| 10         | `and` `or`                  | Left-to-right |
| 11         | `if...else`                 | Right-to-left |
| 12         | `=` `+=` `-=` `*=` `/=`     | Right-to-left |

> **Unary prefix operators** (`!` `-` `+`) bind tightly: only weaker than call and member access,
> and stronger than all binary operators. Therefore `!a == b` ≡ `(!a) == b` (Zig-style semantics);
> `!` is a pure unary operation, does not participate in short-circuit control flow, and is
> orthogonal to the `and`/`or` keywords (short-circuit) (authoritative definition in RFC-010).

> **Range binding power**: `..` has binding power (6, 7)—left 6 is weaker than addition (7), right 7
> swallows addition but not the same-level `..`. Before/after comparison:
>
> | Expression   | Before (level 1, right-associative)                                                 | After ((6,7), left-associative)                                          |
> | ------------ | ----------------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
> | `x in 1..10` | `x in 1..10` (`in` right operand level 4, `..` level 1 can't swallow, unparseable)  | `x in (1..10)`—the interval as a whole is `in`'s right operand           |
> | `0..n+2`     | `(0..n)+2` (right-associativity trap: upper bound eaten, `for` loop directly E3004) | `0..(n+2)`—upper bound is an arithmetic expression                       |
> | `a == b..c`  | `a == (b..c)` (`..` level 1 < `==` level 3, naturally whole)                        | `a == (b..c)`—**semantics unchanged**, `..` still above comparison level |
> | `1..2*3`     | `(1..2)*3`                                                                          | `1..(2*3)`—upper bound is an arithmetic expression                       |
> | `a..b..c`    | `a..(b..c)` (right-associative chaining, meaningless Range nesting)                 | `(a..b)..c`—**step form** (`c` is the step)                              |
>
> Net effect: complex upper bounds like `for i in 0..n+2` go from "parses successfully but E3004" to
> "directly usable"; `x in 1..10` goes from "unparseable" to "interval check"; `a..b..c` goes from
> "meaningless nesting" to "step component". Level 6 falls between `+` (level 5) and `<<` (level 7),
> following mathematical convention: intervals are tightly bound constructs, so the upper bound is
> naturally a complete arithmetic expression.

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

> **Statement termination rules**: The separation and line-break behavior between Stmts (explicit
> `;` separation, line-break termination, line-continuation exceptions, line-leading `(`/`[` never
> merging) are defined by [RFC-038](../design/rfc/draft/038-statement-termination.md).

#### 2.9.1 The Three Forms of `{`

`{` has exactly three interpretations in expression position, determined at once by the **content**:

| Form                   | Notation          | Type                 | Example          |
| ---------------------- | ----------------- | -------------------- | ---------------- |
| **Empty block**        | `{}`              | `Void`               | `x: Void = {}`   |
| **Dictionary literal** | `{ "k": v, ... }` | `Dict(K, V)`         | `d = { "a": 1 }` |
| **Block**              | `{ Stmt* Expr? }` | Tail expression type | `y = { 1 + 1 }`  |

**Determination order**:

1. `{}` (no content) → **Empty block**, value `Void`
2. First element is a `String ':' Expr` key-value pair → **Dictionary literal**
3. Otherwise → **Block**, value given by tail expression

> **Why `{}` is not an empty dictionary**: The "emptiness" of an empty dictionary cannot be
> self-describing (it could be `Dict(K, V)` or an empty block), and the dictionary grammar
> [§1.6.4](#_1-6-4-collections) requires at least one key-value pair. When there is no content to go
> by, the zero form of block structure is taken: this is consistent with `unsafe {}` / `spawn {}`,
> without introducing special cases. For an empty dictionary, use `dict.new()`.
>
> **Why functions need an annotation**: `f = { stmt }` is a **value** (tail expression type), not a
> function. To define a function, write the Fn annotation explicitly: `f: () -> Int = { 5 }`. This
> follows the same principle as dictionaries: **the type is determined by content**, not by the
> presence of an annotation. (This rule supersedes RFC-010a Appendix D ruling C "no annotation →
> default to function", see its errata.)

**Unified semantics**: The value of all `{}` blocks is given by the **tail expression**; `return` is
a non-local exit of type `Never`.

| Block type  | Value exit      | Empty block `{}` |
| ----------- | --------------- | ---------------- |
| Plain `{}`  | Tail expression | `Void`           |
| `unsafe {}` | Tail expression | `Void`           |
| `spawn {}`  | Tail expression | `Void`           |

**Core principles** (see [RFC-010a](../design/rfc/accepted/010a-tail-expression-and-return.md) for
details):

- **Block value = tail expression** (the last expression), the only exit, no exceptions
- When the last item is an **assignment statement**, the block value is `Void`; if you want `Void`,
  write `Void` explicitly
- **`return` exits the nearest function boundary** (passing through all blocks, not "returning to a
  block"), type `Never`; `Never <: T`

> holds for any type (principle of explosion), so it can appear at any return-type position

- The expression form `= expr` directly gives a value

```yaoxiang
// Plain {} block: tail expression gives the value
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

// spawn {} block: tail expression gives the result (⚠️ not yet implemented, see #365; currently you must write return)
(a, b) = spawn {
    result1 = fetch("url1"),
    result2 = fetch("url2")
    return (result1, result2)   // block value
}

// return: passes through the block, exits the function
f: (n: Int) -> Int = {
    if n < 0 {
        return 0     // passes through if and the function body, exits the function
    }
    n * 2            // tail expression
}
```

#### Is `name = { ... }` a Function or a Block Value?

`name = { ... }` could be either a function definition (RFC-007 "no-parameter simplest form") or a
block value binding. Per the **annotation-first, default to function** ruling (RFC-010a Appendix D):

| Case                              | Result          | Example                            |
| --------------------------------- | --------------- | ---------------------------------- |
| Value is a Lambda (`=>`)          | Function        | `f = () => 5` → `f()` = 5          |
| Annotation is a function type     | Function        | `f: () -> Int = { 5 }` → `f()` = 5 |
| Annotation is a non-function type | **Block value** | `x: Int = { 5 }` → `x` = 5         |
| No annotation                     | Function        | `f = { 5 }` → `f()` = 5            |

**Annotation is type**: `x: Int = ...` declares `x` to be `Int`, so `{ ... }` evaluates to `Int`;
`f: () -> Int = ...` declares `f` to be a function, so `{ ... }` is a function body.

To have `{ ... }` evaluated immediately, **just write the target type** (no new syntax needed):

```yaoxiang
// Block value: evaluate immediately
x: Int = {
    y = 5
    y            // x = 5
}

// Function: default when no annotation
f = { 5 }        // f() = 5
```

#### Nested Function Types: Currying or Returning a Function?

There are two readings of a nested function type to the right of `->`, distinguished by
**parentheses** (RFC-004):

| Notation                        | Meaning                  | Call                    |
| ------------------------------- | ------------------------ | ----------------------- |
| `(a: Int) -> (b: Int) -> Int`   | **Currying**             | `f(1)(2)`               |
| `(a: Int) -> ((b: Int) -> Int)` | **Returning a function** | `g(1)` gives a function |

Rationale: **Annotation is type**. `g: (a: Int) -> ((b: Int) -> Int)` declares that
`g(1) : (b: Int) -> Int`, so `g(1)` must **be** that function, not "the next segment of parameters".

```yaoxiang
// Currying: two parameter segments supplied layer by layer
add: (a: Int) -> (b: Int) -> Int = { a + b }
add(1)(2)        // → 3

// Returning a function: outer layer one parameter, the returned value is the function
adder: (n: Int) -> ((x: Int) -> Int) = (x) => x + n
adder(10)(5)     // → 15
h = adder(10)    // The returned function can be stored in a variable or passed around
```

Unparenthesized nested `Fn` is always currying (including the type-parameter form in RFC-011):

```yaoxiang
identity: (T: Type) -> (x: T) -> T = (x) => x   // currying
identity(5)      // → 5
```

**Type check**: Parentheses declare the return type, the body must produce that type.
`f: () -> (() -> Int) = { 7 }` is an error—expected `() -> Int`, but actually got `Int` (E1002).

### 2.10 Lambda Expression

```
Lambda      ::= '(' ParamList? ')' '=>' Expr
            |  '(' ParamList? ')' '=>' Block
```

### 2.11 Error Propagation Operator

```
ErrorPropagate ::= Expr '?'
```

The `?` operator is a postfix operator with the same precedence as `.`. For `Result(T, E)`:

- `Ok(v)` extracts the value `v` and continues execution
- `Err(e)` propagates the error upward (`return Err(e)`)

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

// Range is a value: can be bound, passed, used in membership tests
r = 1..10
assert.assert(5 in r, "membership")
for i in r { print(i) }

// step form (third component, default 1)
for i in 0..10..2 { print(i) }  // 0, 2, 4, 6, 8
for i in 10..0..(-2) { print(i) }  // 10, 8, 6, 4, 2
```

> **step semantics**: In `a..b..c`, `c` is the step. `c = 0` literal is rejected at compile time;
> dynamic `c` has a zero check at runtime (E6001 family; will be elevated to Result once the error
> system lands). `c < 0` is legal, the interval direction is reversed by the sign (`10..0..(-2)`
> decreases).

### 2.13 ref Expression

```
RefExpr     ::= 'ref' Expr
```

`ref` creates a shared holder. The compiler automatically selects Rc (single-task) or Arc
(cross-task); users do not need to care about the implementation details.

```yaoxiang
data = ref heavy_data
spawn { use(data) }   // cross-task: compiler automatically selects Arc
```

### 2.14 unsafe Expression

```
UnsafeExpr  ::= 'unsafe' Block
```

The `unsafe` block is used to define opaque types and operate on raw pointers. Use `return` to
return the type definition to the enclosing scope.

**Semantics**:

- Types can be defined and raw pointers operated on within `unsafe {}`
- Returned types are usable outside `unsafe {}`
- Field access on the type requires unsafe permission

```yaoxiang
// Define an opaque type inside an unsafe block
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  // raw pointer
    }
    return SqliteDb
}

// SqliteDb is usable outside the unsafe block
db = sqlite3_open("test.db")
```

### 2.15 Scope

**Basic rules**:

- Each `{}` block creates a scope
- Inner scopes can access variables from outer scopes
- Outer scopes cannot access variables from inner scopes
- Variable declarations follow the "assignment first" principle

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

- `x = value`: search outward along the scope chain for `x`; if found, assign; if not, declare a new
  one
- `mut x = value`: explicit new mutable declaration, cannot share a name with the outer scope
- Within the same scope, any name can only be declared once

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

**Semantics**: `return` is a **non-local exit** that exits the nearest **function boundary**
(passing through all blocks—including `if` / `while` / `for` / `match` / bare blocks / `spawn` /
`unsafe`), passing the value to the caller. It does **not "return to a block"**.

**Type**: `return e : Never` (with `e : T`). `Never <: T'` holds for any `T'` (principle of
explosion, see [Type System §2.2](./type-system.md)), so `return` can appear at any return-type
position without additional rule constraints.

**Relationship with block evaluation**: A block's value is always the **tail expression** (see
§2.9). `{ return n }` as a block has value `n`, type `Never`; at the same time, `return`'s effect is
to exit the function. **Both things are true simultaneously**, and they coexist via the principle of
explosion.

`return` together with tail expression makes "early return" work, without requiring additional rules
to specifically bind `return` to a function—see
[RFC-010a](../design/rfc/accepted/010a-tail-expression-and-return.md).

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 {
        return 1          // passes through if, exits the function (type Never)
    }
    n * factorial(n - 1)  // tail expression = block value
}
```

### 3.4 break Statement

```
BreakStmt   ::= 'break'
```

**Semantics**: Immediately terminates the innermost `while`/`for` loop that contains it; control
flow continues after the loop body.

- **Exits only the nearest layer**: `break`

> always acts on the innermost loop that contains it. When a nested loop needs to jump out multiple
> layers at once, extract the inner loop into a function and use `return` to return, or use a flag
> (break/continue have no labels; if loop labels are introduced in the future, they will go through
> the RFC process following the loop declaration side syntax, decided together with the multi-exit
> design of the proof pipeline)

- **Restricted to loop bodies**: `break` can only appear within the body of a `while`/`for`

> loop (including blocks/if/match nested inside it); appearing outside a loop is a compile error
> (E1102 `'break' outside of a loop`)

- **Does not affect termination proof**: The `while` loop must still be provably terminating (a
  `decreases` measure); `break`

> does not participate in the termination argument, so `while true { break }` is not accepted

- **Borrow semantics**: Control-flow edges of `break` participate in the structural cut of
  RFC-009a's reverse BFS liveness analysis (iterations jumped out do not participate in back-edge
  liveness derivation)

```yaoxiang
mut i = 0
while i < 10 {
    i = i + 1
    if i == 3 {
        break              // control flow continues after the loop, i == 3
    }
}

// Nested loops: break only exits the inner loop
while j < 3 {
    while k < 10 {
        if k == 2 { break }    // only terminates the inner loop
    }
    j = j + 1                  // each outer iteration reaches here
}
```

### 3.5 continue Statement

```
ContinueStmt::= 'continue'
```

**Semantics**: Skips the remaining statements in this iteration and proceeds directly to the next
iteration of the innermost loop that contains it—`while` re-evaluates the condition, `for` takes the
next element.

- **Affects only the nearest layer**: Same as `break`, no label
- **Restricted to loop bodies**: Appearing outside a loop is a compile error (E1102)

```yaoxiang
mut sum = 0
mut n = 0
while n < 5 {
    n = n + 1
    if n == 3 {
        continue           // skip the accumulation below, n == 3 not counted
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

#### 3.9.1 Semantics: Each Iteration Binds a New Value

YaoXiang's `for` loop semantics differ from traditional languages: **each iteration binds a new
value, rather than modifying the same variable**.

```yaoxiang
// Example: for i in 1..5
for i in 1..5 {
    print(i)
}
```

**Execution process**:

| Iteration | Behavior of the loop variable                                                         |
| --------- | ------------------------------------------------------------------------------------- |
| 1st       | Create new binding `i = 1`, loop body executes, prints 1                              |
| 2nd       | Create new binding `i = 2` (previous binding destroyed), loop body executes, prints 2 |
| 3rd       | Create new binding `i = 3`, loop body executes, prints 3                              |
| 4th       | Create new binding `i = 4`, loop body executes, prints 4                              |
| End       | Loop body ends, binding destroyed                                                     |

**Key point**: After each iteration ends, the binding created by that iteration is destroyed. The
next iteration is a completely new binding, with no relationship to the previous iteration's
binding.

#### 3.9.2 The Difference Between `for` and `for mut`

| Syntax              | Loop variable mutability | Description                                     |
| ------------------- | ------------------------ | ----------------------------------------------- |
| `for i in 1..5`     | Immutable                | The binding cannot be modified in the loop body |
| `for mut i in 1..5` | Mutable                  | The binding can be modified in the loop body    |

```yaoxiang
// Legal: each iteration binds a new value, no need to modify
for i in 1..5 {
    print(i)  // read i's value
}

// Error: immutable binding, cannot be modified
for i in 1..5 {
    i = i + 1  // error: cannot modify an immutable binding
}

// Legal: use for mut to allow modifying the binding
for mut i in 1..5 {
    i = i + 1  // modification allowed
}
```

#### 3.9.3 Shadow Check

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

This rule applies to all code blocks; see [4.3 Shadowing Rules](./modules.md#43-shadowing-rules) for
details.

#### 3.9.4 Comparison with Other Languages

| Language | `for` loop variable semantics                              |
| -------- | ---------------------------------------------------------- |
| YaoXiang | Each iteration binds a new value                           |
| Rust     | Modifies the same variable (requires `mut`)                |
| Python   | Modifies the same variable (no `mut` needed)               |
| C/C++    | Modifies the same variable (requires pointer or reference) |

**Design rationale**: YaoXiang adopts binding semantics because:

1. **More aligned with natural semantics** In natural language, "for each element x in a collection"
   means each x is an independent individual. YaoXiang's `for i in 1..5`

> is read as "for each i in 1 to 5", and the i in each iteration is a completely new binding,
> consistent with human intuition.

2. **Avoids accidental modification**

> The default immutable binding semantics means the loop variable cannot be accidentally modified
> inside the loop body. No need to worry about some place in a complex loop body accidentally
> writing `i = ...` and causing hard-to-track bugs.

3. **High-performance solutions within reach** When it is genuinely necessary to reuse a variable
   across iterations (e.g., accumulator, cache), use `for mut`

> to switch to mutable binding mode. This is clearer than implicit shared state—intent is expressed
> explicitly through syntax, not hidden in runtime behavior.

### 3.10 spawn Statement

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' 'mut'? Identifier 'in' Expr '{' Expr '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
```

**spawn block**: Explicitly declares a concurrent region; the expressions inside the block execute
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

**spawn block captures outer variables** (RFC-024 §2.3, value-capture semantics):

- When the block body references an outer variable =

> **Move value capture**: the value is snapshotted into the closure environment at the spawn
> creation point, and the block body reads it through the env (LoadUpvalue)

- **Primitives** (Int/Float/Bool/Char) are copied by value; the outer variable is unaffected
- **Handle types** (Struct/String/List, etc.) snapshot = handle copy, sharing the underlying object;
  in the Embedded runtime (default), same-thread, same-heap, the handle is valid
- Sharing between multiple tasks requires explicit `ref` (§2.13, compiler auto-selects Rc/Arc)
- Outer variables referenced by `return` inside the block are captured the same way

```yaoxiang
t1 = 1 + 1
t2 = 2 + 2
result = spawn {
    return t1 + t2    // t1/t2 are value-captured, result == 6
}
```

---

### 3.11 Program Entry and Top-Level Statements

In the compiler's view, a source file has two roles, **determined by the presence of
`yaoxiang.toml`**:

| Role       | Determination                              | Program body                                                                      |
| ---------- | ------------------------------------------ | --------------------------------------------------------------------------------- |
| **Script** | Single-file direct run, no `yaoxiang.toml` | **Top-level statements** (executed in writing order); `main` is a regular binding |
| **Bin**    | `yaoxiang.toml` exists                     | **`main` function**; top-level must not have executable statements                |

#### Script: Top-Level Statements Are the Program

Without a manifest, the file is a "script", and **top-level statements execute in writing order**:

```yaoxiang
use std.io
io.println("hello")          // executed directly
x: Int = { 42 }              // top-level binding: runtime initialization
io.println(x)                // 42
```

In this mode, `main` is **not special**—it is just an ordinary binding. To make `main` run, you must
explicitly call it:

```yaoxiang
use std.io
main: () -> Void = { io.println("only runs if called") }

main()                       // ← this line must be written
```

> **Why not auto-call `main`?** Top-level statements are already the program body. If `main` were
> also implicitly called, a script that explicitly writes `main()` would execute twice. The two
> rules cannot coexist, so in Script mode there is only one entry point: "top-level statements".

#### Bin: `main` Is the Entry

When a manifest exists, the file is an "executable target", in which case:

- `main` must be defined, and it must be a function (signature must be callable with zero arguments)
- Top-level executable statements are not allowed—the program body is `main`

```yaoxiang
main: () -> Void = {
    print("hello")
}
```

Missing `main` or `main` not being a function is a compile error (the former: no `main` means no
function is reachable; the latter: a value binding will not be called).

> **Library files**: Files `use`d by other files, or files pointed to by `[lib].path` / `[exports]`
> do not require `main`—they are not program entry points.

#### Top-Level Binding Initialization

The initialization value of top-level bindings is **evaluated at runtime**, and is not required to
be a compile-time constant:

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

Circular dependencies are a compile error (the names on the cycle will be listed):

```yaoxiang
a: Int = b + 1
b: Int = a + 1                    // error: a → b → a
```

> **Design basis**: RFC-029f (File Role Model), RFC-010a Appendix D (Block Binding Ruling).

---

## Appendix: Syntax Quick Reference

### A.1 Control Flow

```
if Expr Block (else if Expr Block)* (else Block)?
match Expr { MatchArm+ }
while Expr Block
for 'mut'? Identifier 'in' Expr Block
break | continue          // only inside loop bodies (§3.4 / §3.5)
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
