# Syntax Specification

This document defines the syntax specification of the YaoXiang programming language, including
lexical structure, grammar rules, and operator precedence.

---

## Chapter 1: Lexical Structure

### 1.1 Source Files

YaoXiang source files must use UTF-8 encoding. Source files typically use the `.yx` extension.

### 1.2 Lexical Token Categories

| Category   | Description                        | Example                   |
| ---------- | ---------------------------------- | ------------------------- |
| Identifier | Starts with a letter or underscore | `x`, `_private`, `my_var` |
| Keyword    | Language predefined reserved words | `Type`, `pub`, `use`      |
| Literal    | Fixed value                        | `42`, `"hello"`, `true`   |
| Operator   | Operation symbols                  | `+`, `-`, `*`, `/`        |
| Delimiter  | Syntax delimiters                  | `(`, `)`, `{`, `}`, `,`   |

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

YaoXiang's "reserved words" are divided into three layers, recognized at different stages by the
parser and the type checker:

#### 1.4.1 Literal Reserved Words

Literal identifiers that the parser has as independent tokens; they cannot be used as ordinary
identifiers:

| Identifier | Belongs to Type | Description                                                                                                           |
| ---------- | --------------- | --------------------------------------------------------------------------------------------------------------------- |
| `Type`     | —               | Meta type keyword                                                                                                     |
| `true`     | Bool            | Boolean true value                                                                                                    |
| `false`    | Bool            | Boolean false value                                                                                                   |
| `void`     | Void            | Void literal (Unit value). The lowercase `void` is a value literal; the uppercase `Void` is a type name (see §1.4.3). |

#### 1.4.2 Constructor Expressions

The following constructors are recognized by the parser in pattern matching and expression contexts:

| Constructor | Belongs to Type | Description                      |
| ----------- | --------------- | -------------------------------- |
| `some(T)`   | Option          | Option value variant constructor |
| `ok(T)`     | Result          | Result success variant           |
| `err(E)`    | Result          | Result error variant             |

#### 1.4.3 Built-in Type Names

The following type names are pre-registered by the type checker and can be used in type positions
without import. The parser treats them as ordinary identifiers—**they are not reserved words and can
be shadowed by local bindings (not recommended)**.

| Type Name | Logical Correspondence | Description                                                                                                                                  |
| --------- | ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `Void`    | ⊤ (True / Unit)        | Zero-field product type with exactly one inhabitant (the `void` literal, see §1.4.1)                                                         |
| `Never`   | ⊥ (False / Empty type) | Zero-variant sum type with zero inhabitants. No expression can produce a `Never` value. `Never <: T` holds for all `T` (ex falso principle). |
| `Int`     | —                      | Signed integer                                                                                                                               |
| `Float`   | —                      | Floating-point number                                                                                                                        |
| `Bool`    | —                      | Boolean value: `true` / `false`                                                                                                              |
| `Char`    | —                      | Unicode character                                                                                                                            |
| `String`  | —                      | String                                                                                                                                       |

### 1.5 Identifiers

Identifiers start with a letter or underscore, followed by letters, digits, or underscores.
Identifiers are case-sensitive.

Special identifiers:

- `_` is used as a placeholder, indicating that a value is to be ignored.
- Identifiers starting with an underscore denote private members.

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
Array       ::= '[' Expr (',' Expr)* ']'   // When the target type is annotated as Array(T, N), the literal lands as a fixed-length array
```

> **Dictionary literals require at least one key-value pair**: `{}` is **not** an empty
> dictionary—it is an empty block (value `Void`, see [§2.9](#_2-9-block-expressions)). For an empty
> dictionary, use the constructor `dict.new()`:
>
> ```yaoxiang
> empty = dict.new()                  // ✅ Empty dictionary
> d = { "a": 1 }                       // ✅ Dictionary literal
> wrong = {}                           // ❌ This is not a dictionary, it's an empty block (Void)
> ```
>
> **The criterion is content**: The `Dict` grammar requires at least one `String ':' Expr`; since
> `{}` has no content to go by, it takes the zero form of block structure. The non-empty form is
> self-describing by content (`{ "k": v }` has a key-value pair → dictionary)—this is consistent
> with `f = { 5 }` being an `Int` value rather than a function: **the type is determined by
> content**.

> Set has no literal grammar and no runtime representation—within the set type planning, the Dict
> pattern will be used to complete the implementation (std.set + HeapValue::Set) when the need
> arises. The landing point of List/Dict literals is determined by context type annotation: a bare
> literal and a `List(T)` annotation land as a growable list; an `Array(T, N)` annotation applied
> directly to a literal lands as a fixed-length array. Implicit List→Array conversion is prohibited.
>
> Array literal semantics:
>
> - The number of elements must equal N; otherwise, a compile-time E1002 is raised. An empty literal
>   paired with a non-zero N is also rejected.
> - Each element's type must be compatible with T; otherwise, a compile-time E1002 is raised.
> - The grammatical form of N: only integer literals (possibly negative) or constant names; compound
>   expressions (e.g., `2+1`) are rejected at parse time.
> - When N is a symbolic constant (a function const parameter, e.g., `Array(Int, n)`), the count
>   check is deferred to the refined type phase.
> - v1 nested array literals (e.g., `Array(Array(Int,2),2) = [[1,2],[3,4]]`) are rejected at compile
>   time; explicit construction is required layer by layer. Recursive landing points are left for
>   future versions.

#### 1.6.5 List Comprehensions

```
ListComp    ::= '[' Expr 'for' Identifier 'in' Expr (',' Expr)* ('if' Expr)? ']'
```

> **Behavior tightening (migration note)**: The grammar for the iteration variable was already
> `'for' Identifier 'in'`, but in the old implementation, the pattern went through full Pratt
> parsing—after `'in'` was registered as an infix operator, `x` would swallow `in items` as a
> membership expression. After the fix, non-identifier patterns fail to parse directly, no longer
> falling back to `_` as in the old implementation (silently swallowing errors). Impact: forms like
> `[x for (a, b) in pairs]` that could previously be parsed now produce errors—this form never had
> defined behavior (the variable was always `_`), so the tightening direction is correct, with no
> semantic migration cost.

#### 1.6.6 Membership Test

```
Membership  ::= Expr 'in' Expr
```

> `in` is a binary relational operator returning `Bool`—returns `true` on a hit, `false` on a miss,
> with no error. Semantic split: `[]` asserts existence and retrieves a value (errors on failure),
> while `in` asks whether something exists (a miss is a normal `false`). Right operand coverage:
> List / Array / Dict (key set) / Tuple / String (substring) / Range (interval). `in` is a
> first-class Hoare predicate, serving as the base of compile-time provable propositions at the
> refined type phase. (Set is removed from the right operand list—Set has no runtime representation,
> see §1.6.4)

### 1.7 Comments

```
// Single-line comment

/* Multi-line comment
   can span multiple lines */
```

### 1.8 Indentation Rules

Code must use 4 spaces for indentation; tab characters are prohibited. This is a mandatory syntax
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

> **Unary prefix operators** (`!` `-` `+`) bind tightly: only below call and member access, and
> above all binary operators. Therefore `!a == b` ≡ `(!a) == b` (Zig-style semantics); `!` is a pure
> unary operation, does not participate in short-circuit control flow, and is orthogonal to the
> `and`/`or` keywords (short-circuit) (authoritative definition in RFC-010).

> **Range binding power**: `..` has binding power (6, 7)—the left side 6 is below addition (7), and
> the right side 7 consumes addition but not the same-level `..`. Comparison before and after the
> change:
>
> | Expression   | Before change (level 1, right-associative)                                                            | After change ((6,7), left-associative)                                            |
> | ------------ | ----------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
> | `x in 1..10` | `x in 1..10` (right operand of `in` at level 4, `..` at level 1 cannot swallow; actually unparseable) | `x in (1..10)` — the whole interval as the right operand of `in`                  |
> | `0..n+2`     | `(0..n)+2` (right-associativity trap: upper bound eaten, `for` loop directly E3004)                   | `0..(n+2)` — upper bound is an arithmetic expression                              |
> | `a == b..c`  | `a == (b..c)` (`..` level 1 < `==` level 3, naturally a whole)                                        | `a == (b..c)` — **semantics unchanged**, `..` is still above the comparison level |
> | `1..2*3`     | `(1..2)*3`                                                                                            | `1..(2*3)` — upper bound is an arithmetic expression                              |
> | `a..b..c`    | `a..(b..c)` (right-associative chain, meaningless Range nested in Range)                              | `(a..b)..c` — **step form** (`c` is the step)                                     |
>
> Net effect: the compound upper bound `for i in 0..n+2` changes from "parses successfully but
> E3004" to "directly usable"; `x in 1..10` changes from "unparseable" to "interval check";
> `a..b..c` changes from "meaningless nesting" to "step component". Level 6 falls between `+`
> (level 5) and `<<` (level 7), per the mathematical convention: intervals are tight-binding
> constructs, and the upper bound is naturally a complete arithmetic expression.

### 2.3 Function Calls

```
FnCall      ::= Expr '(' ArgList? ')'
ArgList     ::= Expr (',' Expr)* (',' NamedArg)* | NamedArg (',' NamedArg)*
NamedArg    ::= Identifier '=' Expr
```

Named arguments use `name = value` (RFC-010 §Function Definitions, RFC-011 §Construction Forms).
Positional arguments must come before named arguments; in named arguments, the order of arguments
can be arbitrary:

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b

add(3, 5)          // Positional
add(a = 3, b = 5)  // Named
add(b = 5, a = 3)  // Arbitrary order
add(3, b = 5)      // Mixed, positional first
```

Wrong name in a named argument raises **E1014**, specifying the same formal parameter both
positionally and by name raises **E1015**, and mismatched arity raises **E1010** (RFC-013).

### 2.4 Member Access

```
MemberAccess::= Expr '.' Identifier
```

### 2.5 Index Access

```
IndexAccess ::= Expr '[' Expr ']'
```

### 2.6 Type Casts

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

> **Statement termination rules**: The separation and newline behavior between Stmts (explicit `;`
> separation, newline termination, line-continuation exceptions, leading `(`/`[` never merging) are
> defined in [RFC-038](../design/rfc/draft/038-statement-termination.md).

#### 2.9.1 The Three Forms of `{`

`{` in expression position has exactly three interpretations, determined once by **content**:

| Form                   | Notation          | Type                 | Example          |
| ---------------------- | ----------------- | -------------------- | ---------------- |
| **Empty block**        | `{}`              | `Void`               | `x: Void = {}`   |
| **Dictionary literal** | `{ "k": v, ... }` | `Dict(K, V)`         | `d = { "a": 1 }` |
| **Block**              | `{ Stmt* Expr? }` | Tail expression type | `y = { 1 + 1 }`  |

**Determination order**:

1. `{}` (no content) → **empty block**, value `Void`
2. The first element is a `String ':' Expr` key-value pair → **dictionary literal**
3. Otherwise → **block**, value given by the tail expression

> **Why `{}` is not an empty dictionary**: The "emptiness" of an empty dictionary cannot
> self-describe (it could be either `Dict(K, V)` or an empty block), whereas the dictionary grammar
> [§1.6.4](#_1-6-4-collections) requires at least one key-value pair. When there is no content to go
> by, the zero form of block structure is taken: this is consistent with `unsafe {}` / `spawn {}`,
> introducing no special case. Use `dict.new()` for an empty dictionary.
>
> **Why functions need annotations**: `f = { stmt }` is a **value** (tail expression type), not a
> function. To define a function, write the Fn annotation explicitly: `f: () -> Int = { 5 }`. This
> follows the same principle as dictionaries: **the type is determined by content**, not by the
> existence of an annotation. (This rule is in
> [RFC-010a](./design/rfc/accepted/010a-tail-expression-and-return.md) Appendix D.)

**Unified semantics**: The value of all `{}` blocks is given by the **tail expression**; `return` is
a non-local exit of type `Never`.

| Block type  | Value exit      | Empty block `{}` |
| ----------- | --------------- | ---------------- |
| Plain `{}`  | Tail expression | `Void`           |
| `unsafe {}` | Tail expression | `Void`           |
| `spawn {}`  | Tail expression | `Void`           |

**Core principles** (see [RFC-010a](../design/rfc/accepted/010a-tail-expression-and-return.md) for
details):

- **Block value = tail expression** (the last expression); the sole exit, no exceptions.
- When the last item is an **assignment statement**, the block value is `Void`; if you want `Void`,
  write `Void` explicitly.
- **`return` exits the nearest function boundary** (passes through all blocks—not "returning to the
  block"); its type is `Never`. Since `Never <: T` holds for any type (ex falso principle), it can
  appear in any return-type position.
- The expression form `= expr` directly yields a value.

```yaoxiang
// Plain {} block: the tail expression gives the value
result = {
    x = compute()
    x                // Block value
}

// unsafe {} block: the tail expression gives the type definition
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void
    }
    SqliteDb         // Block value
}

// spawn {} block: the tail expression gives the result
(a, b) = spawn {
    result1 = fetch("url1"),
    result2 = fetch("url2")
    (result1, result2)   // Block value
}

// return: passes through the block, exits the function
f: (n: Int) -> Int = {
    if n < 0 {
        return 0     // Passes out of `if` and the function body, exits the function
    }
    n * 2            // Tail expression
}
```

#### Is `name = { ... }` a Function or a Block Value?

`name = { ... }` can be either a function definition (RFC-007 "Empty-parameter Simplest Form") or a
block-value binding. The decision is made by **annotation priority, with function as default**
(RFC-010a Appendix D):

| Case                              | Result          | Example                            |
| --------------------------------- | --------------- | ---------------------------------- |
| Value is a Lambda (`=>`)          | Function        | `f = () => 5` → `f()` = 5          |
| Annotation is a function type     | Function        | `f: () -> Int = { 5 }` → `f()` = 5 |
| Annotation is a non-function type | **Block value** | `x: Int = { 5 }` → `x` = 5         |
| No annotation                     | Function        | `f = { 5 }` → `f()` = 5            |

**Annotation is the type**: `x: Int = ...` declares `x` to be `Int`, so `{ ... }` evaluates to
`Int`; `f: () -> Int = ...` declares `f` to be a function, so `{ ... }` is the function body.

To have `{ ... }` evaluate immediately, **just write the target type** (no new syntax needed):

```yaoxiang
// Block value: evaluated immediately
x: Int = {
    y = 5
    y            // x = 5
}

// Function: no annotation, defaults to function
f = { 5 }        // f() = 5
```

#### Nested Function Types: Currying or Returning a Function?

A nested function type to the right of `->` can be read in two ways, distinguished by
**parentheses** (RFC-004):

| Notation                        | Meaning                | Call                     |
| ------------------------------- | ---------------------- | ------------------------ |
| `(a: Int) -> (b: Int) -> Int`   | **Currying**           | `f(1)(2)`                |
| `(a: Int) -> ((b: Int) -> Int)` | **Returns a function** | `g(1)` yields a function |

Basis: **Annotation is the type**. `g: (a: Int) -> ((b: Int) -> Int)` declares
`g(1) : (b: Int) -> Int`, so `g(1)` must **be** that function, not "the next argument segment."

```yaoxiang
// Currying: two argument segments supplied layer by layer
add: (a: Int) -> (b: Int) -> Int = { a + b }
add(1)(2)        // → 3

// Returns a function: outer layer takes one argument; the result is the function
adder: (n: Int) -> ((x: Int) -> Int) = (x) => x + n
adder(10)(5)     // → 15
h = adder(10)    // The returned function can be stored in a variable or passed
```

Unparenthesized nested `Fn` is always currying (including the type-parameter form in RFC-011):

```yaoxiang
identity: (T: Type) -> (x: T) -> T = (x) => x   // Currying
identity(5)      // → 5
```

**Type checking**: The parentheses declare the return type, and the body must produce that type.
`f: () -> (() -> Int) = { 7 }` is an error—`() -> Int` is expected, but `Int` is actually produced
(E1002).

### 2.10 Lambda Expressions

```
Lambda      ::= '(' ParamList? ')' '=>' Expr
            |  '(' ParamList? ')' '=>' Block
```

### 2.11 Error Propagation Operator

```
ErrorPropagate ::= Expr '?'
```

The `?` operator is a postfix operator with the same precedence as `.`. For `Result(T, E)`:

- When `Ok(v)`, extract value `v` and continue execution.
- When `Err(e)`, propagate the error upward (`return Err(e)`).

```yaoxiang
process: (data: Data) -> Result(Data, Error) = {
    validated = validate(data)?     // On success extract the value; on failure propagate upward
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

// Range is a value: bind, pass, and check membership
r = 1..10
assert.assert(5 in r, "membership")
for i in r { print(i) }

// Step form (third component, default 1)
for i in 0..10..2 { print(i) }  // 0, 2, 4, 6, 8
for i in 10..0..(-2) { print(i) }  // 10, 8, 6, 4, 2
```

> **Step semantics**: `c` in `a..b..c` is the step. A literal `c = 0` is rejected at compile-time; a
> dynamic `c` is zero-checked at runtime (E6001 family; promoted to Result after the error system
> lands). `c < 0` is legal, and the interval direction reverses with the sign (`10..0..(-2)`
> decreases).

### 2.13 ref Expressions

```
RefExpr     ::= 'ref' Expr
```

`ref` creates shared ownership. The compiler automatically selects Rc (single-task) or Arc
(cross-task); the user does not need to worry about implementation details.

```yaoxiang
data = ref heavy_data
spawn { use(data) }   // Cross-task: the compiler automatically chooses Arc
```

### 2.14 unsafe Expressions

```
UnsafeExpr  ::= 'unsafe' Block
```

The `unsafe` block is used to define opaque types and operate on raw pointers. Use `return` to
return a type definition to the enclosing scope.

**Semantics**:

- In `unsafe {}`, you can define types and operate on raw pointers.
- The returned type is usable outside `unsafe {}`.
- Field access on the type requires unsafe permission.

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

- Every `{}` block creates a scope.
- Inner scopes can access variables from outer scopes.
- Outer scopes cannot access variables from inner scopes.
- Variable declaration follows the "assignment-first" principle.

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

- `x = value`: search up the scope chain for x; assign if found, otherwise declare a new one.
- `mut x = value`: explicit new mutable declaration; prohibited from sharing a name with an outer
  binding.
- Any name can only be declared once within the same scope.

> **Detailed definition**: For the complete rules of scope, variable declaration, and shadowing
> mechanisms, see [Module System Specification](./modules.md#chapter-4-scope).

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

### 3.3 return Statement

```
ReturnStmt  ::= 'return' Expr?
```

**Semantics**: `return` is a **non-local exit**, exiting the nearest **function boundary** (passing
through all blocks—including `if` / `while` / `for` / `match` / bare blocks / `spawn` /
`unsafe`)—and passing the value to the caller. It does **not "return to the block."**

**Type**: `return e : Never` (where `e : T`). Since `Never <: T'` holds for any `T'` (ex falso
principle, see [Type System §2.2](./type-system.md)), `return` can appear in any return-type
position without additional rule constraints.

**Relationship with block evaluation**: A block's value is always the **tail expression** (see
§2.9). `{ return n }` as a block has value `n`, type `Never`; at the same time, `return` has the
effect of exiting the function. **Both hold simultaneously**, coexisting via the ex falso principle.

`return` and the tail expression together make "early return" work, without needing additional rules
specifying `return` for functions—see
[RFC-010a](../design/rfc/accepted/010a-tail-expression-and-return.md).

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 {
        return 1          // Passes out of `if`, exits the function (type Never)
    }
    n * factorial(n - 1)  // Tail expression = block value
}
```

### 3.4 break Statement

```
BreakStmt   ::= 'break'
```

**Semantics**: Immediately terminates the innermost `while`/`for` loop, with control flow proceeding
after that loop body.

- **Only exits the nearest level**: `break` always acts on the innermost loop containing it. When a
  nested loop needs to jump out of multiple levels at once, extract the inner loop as a function and
  use `return`, or use a flag (break/continue have no labels; if loop labels are introduced in the
  future, they will follow the RFC process on the loop-declaration side, decided together with the
  multi-exit design of the proof pipeline).
- **Only inside loop bodies**: `break` can only appear inside `while`/`for` loop bodies (including
  blocks/if/match nested within the body); appearing outside a loop raises a compile error (E1102
  `'break' outside of a loop`).
- **Does not affect termination proofs**: `while` loops must still be provably terminating (via the
  `decreases` measure); `break` does not participate in termination arguments, and
  `while true { break }` will not be accepted.
- **Borrow semantics**: The control-flow edge of `break` participates in the structural cut of
  RFC-009a's reverse-BFS liveness analysis (iterations that are jumped out of do not participate in
  back-edge liveness derivation).

```yaoxiang
mut i = 0
while i < 10 {
    i = i + 1
    if i == 3 {
        break              // Control flow proceeds past the loop; i == 3
    }
}

// Nested loop: break only exits the inner loop
while j < 3 {
    while k < 10 {
        if k == 2 { break }    // Only terminates the inner loop
    }
    j = j + 1                  // Each outer iteration executes up to here
}
```

### 3.5 continue Statement

```
ContinueStmt::= 'continue'
```

**Semantics**: Skips the remaining statements of the current iteration, directly entering the next
round of the innermost loop—for `while`, it returns to the condition re-evaluation; for `for`, it
takes the next element.

- **Only affects the nearest level**: same as `break`; no labels.
- **Only inside loop bodies**: appearing outside a loop raises a compile error (E1102).

```yaoxiang
mut sum = 0
mut n = 0
while n < 5 {
    n = n + 1
    if n == 3 {
        continue           // Skip the accumulation below; n == 3 is not counted
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

YaoXiang's for-loop semantics differ from traditional languages: **each iteration binds a new value,
rather than modifying the same variable**.

```yaoxiang
// Example: for i in 1..5
for i in 1..5 {
    print(i)
}
```

**Execution process**:

| Iteration | Behavior of the loop variable                                                                      |
| --------- | -------------------------------------------------------------------------------------------------- |
| 1st       | Create a new binding `i = 1`; the loop body executes; prints 1                                     |
| 2nd       | Create a new binding `i = 2` (the previous binding is destroyed); the loop body executes; prints 2 |
| 3rd       | Create a new binding `i = 3`; the loop body executes; prints 3                                     |
| 4th       | Create a new binding `i = 4`; the loop body executes; prints 4                                     |
| End       | The loop body ends; the binding is destroyed                                                       |

**Key point**: After each iteration ends, the binding created in that iteration is destroyed. The
next iteration is a completely new binding, with no relation to the previous iteration's binding.

#### 3.9.2 Difference Between `for` and `for mut`

| Syntax              | Loop variable mutability | Description                             |
| ------------------- | ------------------------ | --------------------------------------- |
| `for i in 1..5`     | Immutable                | The loop body cannot modify the binding |
| `for mut i in 1..5` | Mutable                  | The loop body may modify the binding    |

```yaoxiang
// Legal: each iteration binds a new value; no modification is needed
for i in 1..5 {
    print(i)  // Reads the value of i
}

// Error: immutable binding; cannot be modified
for i in 1..5 {
    i = i + 1  // Error: cannot modify an immutable binding
}

// Legal: use `for mut` to allow modification
for mut i in 1..5 {
    i = i + 1  // Modification allowed
}
```

#### 3.9.3 Shadowing Check

YaoXiang prohibits variable shadowing. The for-loop variable cannot share the same name as a
variable in an outer scope:

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

This rule applies to all code blocks; see [4.3 Shadowing Rules](./modules.md#43-shadowing-rules).

#### 3.9.4 Comparison with Other Languages

| Language | for-loop variable semantics                                |
| -------- | ---------------------------------------------------------- |
| YaoXiang | Each iteration binds a new value                           |
| Rust     | Modifies the same variable (requires `mut`)                |
| Python   | Modifies the same variable (no `mut` needed)               |
| C/C++    | Modifies the same variable (requires pointer or reference) |

**Design rationale**: YaoXiang adopts binding semantics for the following reasons:

1. **More natural semantics** In natural language, "for each element x in a collection" means each x
   is an independent individual. YaoXiang's `for i in 1..5` reads as "for each i from 1 to 5"; the i
   in each iteration is a completely new binding, consistent with human intuitive understanding.

2. **Avoids accidental modification** The default immutable binding semantics mean the loop variable
   cannot be accidentally modified within the loop body. There is no need to worry about
   accidentally writing `i = ...` somewhere in a complex loop body, leading to hard-to-trace bugs.

3. **High-performance solutions are within reach** When variables really need to be reused across
   iterations (e.g., accumulators, caches), use `for mut` to switch to mutable binding mode. This is
   clearer than implicit shared state—the intent is expressed explicitly through syntax rather than
   hidden in runtime behavior.

### 3.10 spawn Statement

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' 'mut'? Identifier 'in' Expr '{' Expr '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
```

**spawn block**: Explicitly declares a concurrent region; the expressions within the block execute
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

- Block body references outer variables = **Move value capture**: the value is snapshotted into the
  closure environment at the spawn creation point, and the block body reads it via env
  (LoadUpvalue).
- **Primitives** (Int / Float / Bool / Char) are value-copied; outer variables are unaffected.
- **Handle types** (Struct / String / List, etc.) snapshot = handle copy, sharing the underlying
  object; the Embedded runtime (default) is same-thread same-heap, and handles are valid.
- Sharing between multiple tasks requires explicit `ref` (§2.13; the compiler automatically selects
  Rc / Arc).
- Outer variables referenced by `return` inside the block are captured in the same way.

```yaoxiang
t1 = 1 + 1
t2 = 2 + 2
result = spawn {
    return t1 + t2    // t1 / t2 captured by value; result == 6
}
```

---

### 3.11 Program Entry and Top-Level Statements

A source file plays two roles in the compiler's eyes, **determined by whether `yaoxiang.toml`
exists**:

| Role       | Determination                       | Program body                                                                      |
| ---------- | ----------------------------------- | --------------------------------------------------------------------------------- |
| **Script** | Single-file run, no `yaoxiang.toml` | **Top-level statements** (executed in writing order); `main` is a regular binding |
| **Bin**    | `yaoxiang.toml` exists              | **`main` function**; top-level executable statements are not allowed              |

#### Script: Top-Level Statements Are the Program

Without a manifest, the file is a "script"; **top-level statements execute in writing order**:

```yaoxiang
use std.io
io.println("hello")          // Executed directly
x: Int = { 42 }              // Top-level binding: initialized at runtime
io.println(x)                // 42
```

In this mode, `main` is **not special**—it is just a regular binding. To make `main` run, you must
call it explicitly:

```yaoxiang
use std.io
main: () -> Void = { io.println("only runs if called") }

main()                       // ← This line is required
```

> **Why not call `main` automatically?** Top-level statements are already the program body. If
> `main` were implicitly called, a script that explicitly writes `main()` would execute twice. The
> two rules cannot coexist, so in Script mode there is only the "top-level statements" execution
> entry.

#### Bin: `main` Is the Entry

With a manifest, the file is an "executable target"; in this case:

- `main` must be defined, and it must be a function (the signature must be zero-argument callable).
- Top-level executable statements are not allowed—the program body is `main`.

```yaoxiang
main: () -> Void = {
    print("hello")
}
```

Missing `main`, or `main` not being a function, are both compile errors (the former: without `main`,
all functions are unreachable; the latter: a value binding will not be called).

> **Library files**: Files used by other files via `use`, or pointed to by `[lib].path` /
> `[exports]`, do not require `main`—they are not program entries.

#### Initialization of Top-Level Bindings

The initialization values of top-level bindings **are evaluated at runtime** and are not required to
be compile-time constants:

```yaoxiang
answer: Int = { 42 }              // Block value
inc: (Int) -> Int = (x) => x + 1
computed: Int = inc(41)           // Function call
```

Initialization executes in **dependency order**, regardless of writing order:

```yaoxiang
derived: Int = base * 3           // References `base` declared later
base: Int = 7                     // Initialized first (topological sort)
```

Circular dependencies are compile errors (the names in the cycle will be listed):

```yaoxiang
a: Int = b + 1
b: Int = a + 1                    // Error: a → b → a
```

> **Design basis**: RFC-029f (File Role Model), RFC-010a Appendix D (Block Binding Adjudication).

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
