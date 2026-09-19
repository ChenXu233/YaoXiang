# Syntax Specification

This document defines the syntax specification of the YaoXiang programming language, including
lexical structure, grammar rules, and operator precedence.

---

## Chapter 1: Lexical Structure

### 1.1 Source Files

YaoXiang source files must use UTF-8 encoding. Source files typically use the `.yx` extension.

### 1.2 Token Categories

| Category   | Description                        | Examples                  |
| ---------- | ---------------------------------- | ------------------------- |
| Identifier | Starts with letter or underscore   | `x`, `_private`, `my_var` |
| Keyword    | Language predefined reserved words | `Type`, `pub`, `use`      |
| Literal    | Fixed values                       | `42`, `"hello"`, `true`   |
| Operator   | Operation symbols                  | `+`, `-`, `*`, `/`        |
| Separator  | Syntactic separators               | `(`, `)`, `{`, `}`, `,`   |

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

YaoXiang's "reserved words" are divided into three layers, identified at different stages by the
parser and type checker:

#### 1.4.1 Literal Reserved Words

Literal identifiers that have independent tokens in the parser and cannot be used as ordinary
identifiers:

| Identifier | Type | Description                                                                                                   |
| ---------- | ---- | ------------------------------------------------------------------------------------------------------------- |
| `Type`     | —    | Meta-type keyword                                                                                             |
| `true`     | Bool | Boolean true value                                                                                            |
| `false`    | Bool | Boolean false value                                                                                           |
| `void`     | Void | Void literal (Unit value). Lowercase `void` is a value literal; uppercase `Void` is a type name (see §1.4.3). |

#### 1.4.2 Constructor Expressions

The following constructors are recognized by the parser in pattern matching and expression contexts:

| Constructor | Type   | Description                      |
| ----------- | ------ | -------------------------------- |
| `some(T)`   | Option | Option value variant constructor |
| `ok(T)`     | Result | Result success variant           |
| `err(E)`    | Result | Result error variant             |

#### 1.4.3 Built-in Type Names

The following type names are pre-registered by the type checker and can be used in type positions
without import. The parser treats them as ordinary identifiers—**they are not reserved words and can
be shadowed by local bindings (not recommended)**.

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

- `_` is used as a placeholder to indicate that a value is ignored
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
> dictionary—it's an empty block (value `Void`, see [§2.9](#_2-9-块表达式)). For an empty
> dictionary, use the constructor `dict.new()`:
>
> ```yaoxiang
> empty = dict.new()                  // ✅ Empty dictionary
> d = { "a": 1 }                       // ✅ Dictionary literal
> wrong = {}                           // ❌ This is not a dictionary, it's an empty block (Void)
> ```
>
> **The criterion is the content**: The `Dict` grammar requires at least one `String ':' Expr`.
> Since `{}` has no content to determine, it takes the zero form of the block structure. The
> non-empty form is self-describing by content (`{ "k": v }` has key-value pairs → dictionary)—this
> is consistent with `f = { 5 }` being an `Int` value rather than a function: **the type is
> determined by the content**.

> Set has no literal grammar, no runtime representation—in the set type planning, when the
> requirement arises, complete it following the Dict pattern (std.set + HeapValue::Set). The
> destination of List/Dict literals is determined by the context type annotation: bare literals and
> `List(T)` annotations land in growable lists; when `Array(T, N)` annotations directly act on
> literals, they land in fixed-length arrays. Implicit List→Array conversion is forbidden.
>
> Array literal semantics:
>
> - The number of elements must equal N; if unequal, compile-time E1002; empty literals with
>   non-zero N are also rejected
> - Each element type must be compatible with T; if mismatched, compile-time E1002
> - N's grammar form: only integer literals (can be negative) or constant names; compound
>   expressions (like `2+1`) are rejected at parse time
> - When N is a symbolic constant (function const parameter, like `Array(Int, n)`), the count check
>   is deferred to the refinement type phase
> - v1 nested array literals (`Array(Array(Int,2),2) = [[1,2],[3,4]]`) are rejected at compile time
>   and must be constructed layer by layer explicitly; recursive landing is left to a later version

#### 1.6.5 List Comprehensions

```
ListComp    ::= '[' Expr 'for' Identifier 'in' Expr (',' Expr)* ('if' Expr)? ']'
```

> **Behavior tightening (migration record)**: The iteration variable grammar was already
> `'for' Identifier 'in'`, but in the old implementation, patterns went through full pratt
> parsing—after `'in'` was registered as an infix operator, `x` would swallow `in items` as a
> membership expression. After the fix, non-identifier patterns fail to parse directly, no longer
> falling back to `_` like the old implementation (silently swallowing errors). Impact: previously
> parseable `[x for (a, b) in pairs]` now reports an error—this form never had defined behavior
> (variable was always `_`), the tightening direction is correct, with no semantic migration cost.

#### 1.6.6 Membership Testing

```
Membership  ::= Expr 'in' Expr
```

> `in` is a binary relational operator that returns `Bool`—returns `true` on hit, `false` on miss,
> no error. Semantic split: `[]` asserts existence and retrieves the value (errors on failure), `in`
> asks whether it exists (missing is a normal `false`). Right operand coverage: List / Array / Dict
> (key set) / Tuple / String (substring) / Range (interval). `in` is a first-class Hoare predicate,
> serving as the basis of compile-time provable propositions in the refinement type phase. (Set is
> removed from the right operand list—Set has no runtime representation, see §1.6.4)

### 1.7 Comments

```
// Single-line comment

/* Multi-line comment
   can span multiple lines */
```

### 1.8 Indentation Rules

Code must use 4 spaces for indentation, Tab characters are forbidden. This is a mandatory syntax
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

> **Unary prefix operators** (`!` `-` `+`) are tightly bound: only lower than call and member
> access, higher than all binary operators. Thus `!a == b` ≡ `(!a) == b` (Zig-style semantics); `!`
> is pure unary operation, not participating in short-circuit control flow, orthogonal to the
> `and`/`or` keywords (short-circuit) (RFC-010 authoritative definition).

> **Range binding power**: `..` has binding power (6, 7)—left 6 is lower than addition (7), right 7
> swallows addition but not the same-level `..`. Before and after the change comparison:
>
> | Expression   | Before change (level 1, right-associative)                                                       | After change ((6,7), left-associative)                                         |
> | ------------ | ------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------ |
> | `x in 1..10` | `x in 1..10` (right operand of `in` level 4, `..` level 1 cannot swallow, actually cannot parse) | `x in (1..10)`—the interval as a whole is the right operand of `in`            |
> | `0..n+2`     | `(0..n)+2` (right-associative trap: upper bound is eaten, `for` loop directly E3004)             | `0..(n+2)`—upper bound is an arithmetic expression                             |
> | `a == b..c`  | `a == (b..c)` (`..` level 1 < `==` level 3, naturally whole)                                     | `a == (b..c)`—**semantics unchanged**, `..` still higher than comparison level |
> | `1..2*3`     | `(1..2)*3`                                                                                       | `1..(2*3)`—upper bound is an arithmetic expression                             |
> | `a..b..c`    | `a..(b..c)` (right-associative chained, meaningless Range within Range)                          | `(a..b)..c`—**step form** (`c` is the step)                                    |
>
> Net effect: the compound upper bound `for i in 0..n+2` changes from "parses successfully but
> E3004" to "directly usable"; `x in 1..10` changes from "cannot parse" to "interval check";
> `a..b..c` changes from "meaningless nesting" to "step component". Level 6 falls between `+`
> (level 5) and `<<` (level 7), mathematical convention: intervals are tightly bound constructs, the
> upper bound is naturally a complete arithmetic expression.

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

> **Statement termination rules**: The separation between Stmts and newline behavior (explicit `;`
> separation, newline termination, line continuation exception, leading `(`/`[` never merging) are
> defined by [RFC-038](../design/rfc/draft/038-statement-termination.md).

#### 2.9.1 Three Forms of `{`

`{` has exactly three interpretations in expression position, determined once by **content**:

| Form             | Notation          | Type                 | Example          |
| ---------------- | ----------------- | -------------------- | ---------------- |
| **Empty block**  | `{}`              | `Void`               | `x: Void = {}`   |
| **Dict literal** | `{ "k": v, ... }` | `Dict(K, V)`         | `d = { "a": 1 }` |
| **Block**        | `{ Stmt* Expr? }` | Tail expression type | `y = { 1 + 1 }`  |

**Determination order**:

1. `{}` (no content) → **Empty block**, value `Void`
2. First element is `String ':' Expr` key-value pair → **Dict literal**
3. Otherwise → **Block**, value given by tail expression

> **Why `{}` is not an empty dictionary**: The "empty" of an empty dictionary cannot self-describe
> (it can be either `Dict(K, V)` or an empty block), while the dict grammar [§1.6.4](#_1-6-4-集合)
> requires at least one key-value pair. When there is no content to rely on, take the zero form of
> the block structure: this is consistent with `unsafe {}` / `spawn {}`, no special case is
> introduced. For empty dictionary, use `dict.new()`.
>
> **Why functions need annotations**: `f = { stmt }` is a **value** (tail expression type), not a
> function. To define a function, write the Fn annotation: `f: () -> Int = { 5 }`. This follows the
> same principle as dictionaries: **the type is determined by the content**, not by whether an
> annotation exists. (This rule is in
> [RFC-010a](./design/rfc/accepted/010a-tail-expression-and-return.md) Appendix D.)

**Unified semantics**: The value of all `{}` blocks is given by the **tail expression**, `return` is
a non-local exit of type `Never`.

| Block type  | Value exit      | Empty block `{}` |
| ----------- | --------------- | ---------------- |
| Normal `{}` | Tail expression | `Void`           |
| `unsafe {}` | Tail expression | `Void`           |
| `spawn {}`  | Tail expression | `Void`           |

**Core principles** (see [RFC-010a](../design/rfc/accepted/010a-tail-expression-and-return.md) for
details):

- **Block value = tail expression** (the last expression), the only exit, no exceptions
- When the last position is an **assignment statement**, the block value is `Void`; if you want
  `Void`, write `Void` explicitly
- **`return` exits the nearest function boundary** (passes through all blocks, does not "return to
  the block"), type `Never`; `Never <: T` holds for any type (principle of explosion), so it can
  appear in any return type position
- Expression form `= expr` directly gives a value

```yaoxiang
// Normal {} block: tail expression gives the value
result = {
    x = compute()
    x                // Block value
}

// unsafe {} block: tail expression gives the type definition
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void
    }
    SqliteDb         // Block value
}

// spawn {} block: tail expression gives the result
(a, b) = spawn {
    result1 = fetch("url1"),
    result2 = fetch("url2")
    (result1, result2)   // Block value
}

// return: passes through the block, exits the function
f: (n: Int) -> Int = {
    if n < 0 {
        return 0     // Passes through if and the function body, exits the function
    }
    n * 2            // Tail expression
}
```

#### Is `name = { ... }` a Function or a Block Value?

`name = { ... }` can be either a function definition (RFC-007 "Simplest zero-arg") or a block value
binding. Ruling by **annotation priority, default function** (RFC-010a Appendix D):

| Case                              | Result          | Example                            |
| --------------------------------- | --------------- | ---------------------------------- |
| Value is a Lambda (`=>`)          | Function        | `f = () => 5` → `f()` = 5          |
| Annotation is a function type     | Function        | `f: () -> Int = { 5 }` → `f()` = 5 |
| Annotation is a non-function type | **Block value** | `x: Int = { 5 }` → `x` = 5         |
| No annotation                     | Function        | `f = { 5 }` → `f()` = 5            |

**Annotation is type**: `x: Int = ...` declares that `x` is `Int`, then `{ ... }` evaluates to
`Int`; `f: () -> Int = ...` declares that `f` is a function, then `{ ... }` is the function body.

To evaluate `{ ... }` on the spot, **just write the target type** (no new syntax needed):

```yaoxiang
// Block value: immediately evaluated
x: Int = {
    y = 5
    y            // x = 5
}

// Function: no annotation defaults
f = { 5 }        // f() = 5
```

#### Nested Function Types: Currying or Returning a Function?

The nested function type on the right side of `->` has two readings, distinguished by
**parentheses** (RFC-004):

| Writing                         | Meaning             | Call                   |
| ------------------------------- | ------------------- | ---------------------- |
| `(a: Int) -> (b: Int) -> Int`   | **Currying**        | `f(1)(2)`              |
| `(a: Int) -> ((b: Int) -> Int)` | **Return function** | `g(1)` gets a function |

Basis: **annotation is type**. `g: (a: Int) -> ((b: Int) -> Int)` declares that
`g(1) : (b: Int) -> Int`, so `g(1)` must **be** that function, not the "next segment of parameters".

```yaoxiang
// Currying: two segments of parameters given layer by layer
add: (a: Int) -> (b: Int) -> Int = { a + b }
add(1)(2)        // → 3

// Return function: outer segment of parameters, returns a function
adder: (n: Int) -> ((x: Int) -> Int) = (x) => x + n
adder(10)(5)     // → 15
h = adder(10)    // The returned function can be stored in a variable, passed around
```

Unparenthesized nested `Fn` is always currying (including the type parameter form in RFC-011):

```yaoxiang
identity: (T: Type) -> (x: T) -> T = (x) => x   // Currying
identity(5)      // → 5
```

**Type checking**: Parentheses declare the return type, the body must produce that type.
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

The `?` operator is a postfix operator with the same precedence as `.`. For `Result(T, E)` types:

- On `Ok(v)`, extracts value `v` and continues execution
- On `Err(e)`, propagates the error upward (`return Err(e)`)

```yaoxiang
process: (data: Data) -> Result(Data, Error) = {
    validated = validate(data)?     // Extract value on success, propagate on failure
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

// Range is a value: bind, pass, membership test
r = 1..10
assert.assert(5 in r, "membership")
for i in r { print(i) }

// step form (third component, default 1)
for i in 0..10..2 { print(i) }  // 0, 2, 4, 6, 8
for i in 10..0..(-2) { print(i) }  // 10, 8, 6, 4, 2
```

> **Step semantics**: In `a..b..c`, `c` is the step. `c = 0` literal is rejected at compile time;
> dynamic `c` zero-checked at runtime (E6001 family; promoted to Result after the error system
> lands). `c < 0` is legal, the interval direction reverses with the sign (`10..0..(-2)` decreases).

### 2.13 ref Expression

```
RefExpr     ::= 'ref' Expr
```

`ref` creates shared ownership. The compiler automatically chooses Rc (single-task) or Arc
(cross-task), users do not need to care about implementation details.

```yaoxiang
data = ref heavy_data
spawn { use(data) }   // Cross-task: compiler automatically chooses Arc
```

### 2.14 unsafe Expression

```
UnsafeExpr  ::= 'unsafe' Block
```

`unsafe` blocks are used to define opaque types and operate raw pointers. Use `return` to return the
type definition to the previous scope.

**Semantics**:

- Types and raw pointer operations can be defined in `unsafe {}`
- The returned type is available outside `unsafe {}`
- Field access of the type requires unsafe permission

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

- Each `{}` block creates a scope
- Inner scopes can access variables in outer scopes
- Outer scopes cannot access variables in inner scopes
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

- `x = value`: look up x along the scope chain, assign if found, declare new if not found
- `mut x = value`: explicit new mutable declaration, forbidding the same name as the outer layer
- Any name can only be declared once within the same scope

> **Detailed definition**: The complete rules of scopes, variable declarations and shadowing
> mechanisms are detailed in [Module System Specification](./modules.md#第四章作用域).

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

**Semantics**: `return` is a **non-local exit**, exiting the nearest **function boundary** (passes
through all blocks—including `if` / `while` / `for` / `match` / bare block / `spawn` / `unsafe`),
passing the value to the caller. It does **not "return to the block"**.

**Type**: `return e : Never` (where `e : T`). `Never <: T'` holds for any `T'` (principle of
explosion, see [Type System §2.2](./type-system.md)), so `return` can appear in any return type
position without additional rule constraints.

**Relationship with block evaluation**: The block value is always the **tail expression** (see
§2.9). `{ return n }` as a block, its value is `n`, type `Never`; at the same time the effect of
`return` is to exit the function. **Both hold simultaneously**, coexisting via the principle of
explosion.

`return` together with the tail expression makes "early return" possible, without `return`
specifically designating function additional rules—see
[RFC-010a](../design/rfc/accepted/010a-tail-expression-and-return.md).

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

**Semantics**: Immediately terminates the innermost `while`/`for` loop, control flow transfers to
after the loop body.

- **Only exits the nearest layer**: `break` always acts on the innermost loop containing it. When
  nested loops need to break out of multiple layers at once, extract the inner loop as a function
  and use `return`, or use flag variables (break/continue do not carry labels; if loop labels are
  introduced in the future, they will follow the loop declaration-side syntax through the RFC
  process, judged together with the multi-exit design of the proof pipeline)
- **Only within loop body**: `break` can only appear in `while`/`for` loop bodies (including
  blocks/if/match nested within the body), appearing outside a loop reports a compile error (E1102
  `'break' outside of a loop`)
- **Does not affect termination proof**: `while` loops still need to be provably terminating
  (decreases metric); `break` does not participate in termination arguments, `while true { break }`
  will not be accepted
- **Borrowing semantics**: The control flow edge of break participates in the structural cut of
  RFC-009a reverse BFS liveness analysis (the iteration that breaks out does not participate in
  back-edge liveness derivation)

```yaoxiang
mut i = 0
while i < 10 {
    i = i + 1
    if i == 3 {
        break              // Control flow transfers to after the loop, i == 3
    }
}

// Nested loop: break only exits the inner layer
while j < 3 {
    while k < 10 {
        if k == 2 { break }    // Only terminates the inner loop
    }
    j = j + 1                  // Each outer iteration executes here
}
```

### 3.5 continue Statement

```
ContinueStmt::= 'continue'
```

**Semantics**: Skips the remaining statements in this iteration, directly entering the next round of
the innermost loop—`while` returns to condition re-evaluation, `for` takes the next element.

- **Only acts on the nearest layer**: Same as `break`, no label
- **Only within loop body**: Appearing outside a loop reports a compile error (E1102)

```yaoxiang
mut sum = 0
mut n = 0
while n < 5 {
    n = n + 1
    if n == 3 {
        continue           // Skips the accumulation below, n == 3 is not counted
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

YaoXiang's for loop semantics is different from traditional languages: **each iteration binds a new
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

**Key point**: After each iteration ends, the binding created in that iteration is destroyed. The
next iteration is a brand new binding, with no relation to the binding from the previous iteration.

#### 3.9.2 Difference between for and for mut

| Syntax              | Loop variable mutability | Description                            |
| ------------------- | ------------------------ | -------------------------------------- |
| `for i in 1..5`     | Immutable                | Cannot modify binding within loop body |
| `for mut i in 1..5` | Mutable                  | Can modify binding within loop body    |

```yaoxiang
// Legal: each iteration binds a new value, no modification needed
for i in 1..5 {
    print(i)  // Read the value of i
}

// Error: immutable binding, cannot modify
for i in 1..5 {
    i = i + 1  // Error: cannot modify immutable binding
}

// Legal: use for mut to allow modification
for mut i in 1..5 {
    i = i + 1  // Modification allowed
}
```

#### 3.9.3 Shadowing Check

YaoXiang forbids variable shadowing. for loop variables cannot share names with variables in the
outer scope:

```yaoxiang
// Error: i is already declared outside
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

This rule applies to all code blocks, see [4.3 Shadowing Rules](./modules.md#43-遮蔽规则).

#### 3.9.4 Comparison with Other Languages

| Language | for loop variable semantics                             |
| -------- | ------------------------------------------------------- |
| YaoXiang | Each iteration binds a new value                        |
| Rust     | Modifies the same variable (needs mut)                  |
| Python   | Modifies the same variable (no mut needed)              |
| C/C++    | Modifies the same variable (needs pointer or reference) |

**Design rationale**: YaoXiang adopts binding semantics because:

1. **More in line with natural semantics** In natural language, "for each element x in the
   collection" means each x is an independent individual. YaoXiang's `for i in 1..5` reads as "for
   each i in 1 to 5", the i of each iteration is a brand new binding, which is consistent with human
   intuitive understanding.

2. **Avoid accidental modification** The default immutable binding semantics means the loop variable
   cannot be accidentally modified within the loop body. No need to worry about some place in a
   complex loop body accidentally writing `i = ...` causing hard-to-track bugs.

3. **High-performance solutions are within reach** When it is indeed necessary to reuse variables
   between iterations (such as accumulators, caches), use `for mut` declaration to switch to mutable
   binding mode. This is clearer than implicit shared state—intent is expressed explicitly through
   syntax, not hidden in runtime behavior.

### 3.10 spawn Statement

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' 'mut'? Identifier 'in' Expr '{' Expr '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
```

**spawn block**: Explicitly declares a concurrent region, expressions within the block execute
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

- Block body references outer variable = **Move value capture**: the value is snapshotted into the
  closure environment at the spawn creation point, the block body reads through env (LoadUpvalue)
- **Primitives** (Int/Float/Bool/Char) are value-copied, outer variables are not affected
- **Handle types** (Struct/String/List etc.) snapshot = handle copy, sharing the underlying object;
  Embedded runtime (default) same-thread same-heap, handle is valid
- Sharing between multiple tasks requires explicit `ref` (§2.13, compiler automatically chooses
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

### 3.11 Program Entry and Top-level Statements

A source file has two roles in the compiler's eyes, **determined by whether `yaoxiang.toml`
exists**:

| Role       | Determination                              | Program body                                                                        |
| ---------- | ------------------------------------------ | ----------------------------------------------------------------------------------- |
| **Script** | Single-file direct run, no `yaoxiang.toml` | **Top-level statements** (executed in writing order); `main` is an ordinary binding |
| **Bin**    | `yaoxiang.toml` exists                     | **`main` function**; top-level cannot have executable statements                    |

#### Script: Top-level Statements are the Program

Without a manifest, the file is a "script", **top-level statements execute in writing order**:

```yaoxiang
use std.io
io.println("hello")          // Direct execution
x: Int = { 42 }              // Top-level binding: runtime initialization
io.println(x)                // 42
```

In this mode, `main` is **not special**—it's just an ordinary binding. To make `main` run, you must
call it explicitly:

```yaoxiang
use std.io
main: () -> Void = { io.println("only runs if called") }

main()                       // ← This line must be written
```

> **Why not auto-call `main`?** Top-level statements are already the program body. If `main` is
> implicitly called, a script that explicitly wrote `main()` would execute twice. The two rules
> cannot coexist, so in Script mode there is only one execution entry: "top-level statements".

#### Bin: `main` is the Entry

When a manifest exists, the file is an "executable target", at which point:

- `main` must be defined, and it must be a function (signature must be callable with zero arguments)
- Top-level cannot have executable statements—the program body is `main`

```yaoxiang
main: () -> Void = {
    print("hello")
}
```

Missing `main` or `main` not being a function both result in compile errors (the former: when no
`main` exists, all functions are unreachable; the latter: value bindings will not be called).

> **Library files**: Files `use`d by other files, or files pointed to by `[lib].path` / `[exports]`
> do not require `main`—they are not program entry points.

#### Initialization of Top-level Bindings

The initialization values of top-level bindings are **evaluated at runtime**, not required to be
compile-time constants:

```yaoxiang
answer: Int = { 42 }              // Block value
inc: (Int) -> Int = (x) => x + 1
computed: Int = inc(41)           // Function call
```

Initialization executes in **dependency order**, regardless of writing order:

```yaoxiang
derived: Int = base * 3           // References base declared later
base: Int = 7                     // Initialized first (topological sort)
```

Cyclic dependencies are compile errors (the names on the cycle will be listed):

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
break | continue          // Only within loop body (§3.4 / §3.5)
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
