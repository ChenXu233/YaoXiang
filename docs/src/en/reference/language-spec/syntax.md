# Syntax Specification

This document defines the syntax specification of the YaoXiang programming language, including
lexical structure, grammar rules, and operator precedence.

---

## Chapter 1: Lexical Structure

### 1.1 Source File

YaoXiang source files must use UTF-8 encoding. Source files typically use the `.yx` extension.

### 1.2 Token Categories

| Category   | Description                     | Examples                  |
| ---------- | ------------------------------- | ------------------------- |
| Identifier | Starts with a letter or `_`     | `x`, `_private`, `my_var` |
| Keyword    | Language-defined reserved words | `Type`, `pub`, `use`      |
| Literal    | Fixed values                    | `42`, `"hello"`, `true`   |
| Operator   | Operation symbols               | `+`, `-`, `*`, `/`        |
| Delimiter  | Syntax separators               | `(`, `)`, `{`, `}`, `,`   |

### 1.3 Keywords

YaoXiang defines a very small set of keywords:

```
pub    use    spawn
ref    mut    if     else
else   match  while  for    return
break  continue as     in     unsafe
```

These keywords have special meaning in every context and cannot be used as identifiers.

### 1.4 Reserved Words

YaoXiang's "reserved words" are split into three layers, recognized by the parser and type checker
at different stages:

#### 1.4.1 Literal Reserved Words

Literal identifiers that are separate tokens in the parser and cannot be used as ordinary
identifiers:

| Identifier | Owning Type | Description                                                                                                   |
| ---------- | ----------- | ------------------------------------------------------------------------------------------------------------- |
| `Type`     | —           | Meta-type keyword                                                                                             |
| `true`     | Bool        | Boolean true value                                                                                            |
| `false`    | Bool        | Boolean false value                                                                                           |
| `void`     | Void        | Void literal (Unit value). Lowercase `void` is a value literal; uppercase `Void` is a type name (see §1.4.3). |

#### 1.4.2 Constructor Expressions

The following constructors are recognized by the parser in pattern matching and expression contexts:

| Constructor | Owning Type | Description                      |
| ----------- | ----------- | -------------------------------- |
| `some(T)`   | Option      | Option value variant constructor |
| `ok(T)`     | Result      | Result success variant           |
| `err(E)`    | Result      | Result error variant             |

#### 1.4.3 Built-in Type Names

The following type names are pre-registered by the type checker and can be used in type positions
without imports. The parser treats them as ordinary identifiers — **they are not reserved words and
may be shadowed by local bindings (not recommended)**.

| Type Name | Logical Mapping        | Description                                                                                                                                      |
| --------- | ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| `Void`    | ⊤ (true / Unit)        | Zero-field product type with exactly one inhabitant (the `void` literal, see §1.4.1)                                                             |
| `Never`   | ⊥ (false / empty type) | Zero-variant sum type with zero inhabitants. No expression can produce a `Never` value. `Never <: T` holds for all `T` (principle of explosion). |
| `Int`     | —                      | Signed integer                                                                                                                                   |
| `Float`   | —                      | Floating-point number                                                                                                                            |
| `Bool`    | —                      | Boolean value: `true` / `false`                                                                                                                  |
| `Char`    | —                      | Unicode character                                                                                                                                |
| `String`  | —                      | String                                                                                                                                           |

### 1.5 Identifiers

Identifiers start with a letter or underscore, followed by letters, digits, or underscores.
Identifiers are case-sensitive.

Special identifiers:

- `_` is used as a placeholder, meaning to ignore a value
- Identifiers starting with an underscore represent private members

### 1.6 Literals

#### 1.6.1 Integer

```
Decimal     ::= [0-9][0-9_]*
Octal       ::= 0o[0-7][0-7_]*
Hex         ::= 0x[0-9a-fA-F][0-9a-fA-F_]*
Binary      ::= 0b[01][01_]*
```

#### 1.6.2 Float

```
Float       ::= [0-9][0-9_]* '.' [0-9][0-9_]* ([eE][+-]?[0-9][0-9_]*)?
```

#### 1.6.3 String

```
String      ::= '"' ([^"\\] | EscapeSequence)* '"'
Escape      ::= '\\' ([nrt'"\\] | UnicodeEscape)
Unicode     ::= 'u' '{' HexDigit+ '}'
```

#### 1.6.4 Collection

```
List        ::= '[' Expr (',' Expr)* ']'
Dict        ::= '{' String ':' Expr (',' String ':' Expr)* '}'
Array       ::= '[' Expr (',' Expr)* ']'   // When the target type annotation is Array(T, N), the literal becomes a fixed-length array
```

> Set has no literal grammar and no runtime representation — in set type planning, when requirements
> emerge, follow the Dict pattern to complete it (std.set + HeapValue::Set). The destination of a
> List/Dict literal is determined by context type annotation: a bare literal or `List(T)` annotation
> lands as a growable list; an `Array(T, N)` annotation applied directly to a literal lands as a
> fixed-length array. Implicit List→Array conversion is prohibited.
>
> Array literal semantics:
>
> - The element count must equal N; otherwise compile-time E1002; an empty literal paired with a
>   non-zero N is also rejected
> - Each element's type must be compatible with T; otherwise compile-time E1002
> - The grammatical form of N: only integer literals (possibly negative) or constant names;
>   composite expressions (e.g., `2+1`) are rejected at parse time
> - When N is a symbolic constant (function const parameter, e.g., `Array(Int, n)`), count
>   validation is deferred to the refinement type phase
> - v1 nested array literals (`Array(Array(Int,2),2) = [[1,2],[3,4]]`) are rejected at compile time
>   and must be constructed layer by layer; recursive landing is left for a later version

#### 1.6.5 List Comprehension

```
ListComp    ::= '[' Expr 'for' Identifier 'in' Expr (',' Expr)* ('if' Expr)? ']'
```

> **Behavior tightening (migration record)**: The iteration variable grammar was always
> `'for' Identifier 'in'`, but the old implementation processed patterns through full pratt parsing
> — after `'in'` was registered as an infix operator, `x` would swallow `in items` as a membership
> expression. After the fix, non-identifier patterns fail to parse directly and no longer fall back
> to `_` (silently swallowing errors) as the old implementation did. Impact: previously parseable
> forms like `[x for (a, b) in pairs]` now report an error — that form never had defined behavior
> (the variable was always `_`), so the tightening direction is correct with no semantic migration
> cost.

#### 1.6.6 Membership Test

```
Membership  ::= Expr 'in' Expr
```

> `in` is a binary relational operator returning `Bool` — `true` on hit, `false` on miss, no error.
> Semantic split: `[]` is "assert presence and fetch value" (fails with an error), `in` is "ask
> whether it exists" (miss is a normal `false`). Right operand coverage: List / Array / Dict (key
> set) / Tuple / String (substring) / Range (interval). `in` is a first-class Hoare predicate and
> serves as the base of compile-time provable propositions in the refinement type phase. (Set is
> removed from the right-operand list — Set has no runtime representation, see §1.6.4)

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

| Precedence | Operator                    | Associativity |
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

> **Unary prefix operators** (`!` `-` `+`) bind tightly: they rank just below calls and member
> access, and above all binary operators. Therefore `!a == b` ≡ `(!a) == b` (Zig-style semantics);
> `!` is purely a unary operation that does not participate in short-circuit control flow, and is
> orthogonal to the `and`/`or` keywords (short-circuiting) (authoritative definition in RFC-010).

> **Range binding strength**: `..` has binding strength (6, 7) — left 6 is lower than addition (7),
> right 7 swallows addition but does not swallow same-level `..`. Before/after comparison:
>
> | Expression   | Before (level 1, right-assoc)                                                                         | After ((6,7), left-assoc)                                                  |
> | ------------ | ----------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------- |
> | `x in 1..10` | `x in 1..10` (right operand of `in` is level 4, `..` at level 1 cannot swallow, actually unparseable) | `x in (1..10)` — the interval as a whole is the right operand of `in`      |
> | `0..n+2`     | `(0..n)+2` (right-assoc trap: upper bound is eaten, `for` loop directly E3004)                        | `0..(n+2)` — upper bound is an arithmetic expression                       |
> | `a == b..c`  | `a == (b..c)` (`..` level 1 < `==` level 3, naturally whole)                                          | `a == (b..c)` — **semantics unchanged**, `..` still above comparison level |
> | `1..2*3`     | `(1..2)*3`                                                                                            | `1..(2*3)` — upper bound is an arithmetic expression                       |
> | `a..b..c`    | `a..(b..c)` (right-assoc chain, meaningless Range inside Range)                                       | `(a..b)..c` — **step form** (`c` is the step)                              |
>
> Net effect: the composite upper bound `for i in 0..n+2` changes from "parses successfully but
> E3004" to "directly usable"; `x in 1..10` changes from "unparseable" to "interval check";
> `a..b..c` changes from "meaningless nesting" to "step component". Level 6 falls between `+`
> (level 5) and `<<` (level 7), following mathematical convention: an interval is a tight-binding
> construct, so the upper bound is naturally a complete arithmetic expression.

### 2.3 Function Call

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
> separation, newline termination, line-continuation exceptions, line-leading `(`/`[` never merging)
> are defined by [RFC-038](../design/rfc/draft/038-statement-termination.md).

**Unified semantics**: The value of every `{}` block is given by its **tail expression**; `return`
is a non-local exit of type `Never`.

| Block Type    | Value Outlet    | Empty Block `{}` |
| ------------- | --------------- | ---------------- |
| Ordinary `{}` | Tail expression | `Void`           |
| `unsafe {}`   | Tail expression | `Void`           |
| `spawn {}`    | Tail expression | `Void`           |

**Core principles** (see [RFC-010a](../design/rfc/accepted/010a-tail-expression-and-return.md) for
details):

- **Block value = tail expression** (the last expression), the sole outlet, no exceptions
- When the last item is an **assignment statement**, the block value is `Void`; if you want `Void`,
  write `Void` explicitly
- **`return` exits the nearest function boundary** (penetrating every block, not "returning to the
  block"), of type `Never`; `Never <: T`

> holds for any type (principle of explosion), so it can appear in any return-type position

- The expression form `= expr` directly gives a value

```yaoxiang
// Ordinary {} block: the tail expression gives the value
result = {
    x = compute()
    x                // block value
}

// unsafe {} block: the tail expression gives the type definition
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void
    }
    SqliteDb         // block value
}

// spawn {} block: the tail expression gives the result
(a, b) = spawn {
    result1 = fetch("url1"),
    result2 = fetch("url2")
    (result1, result2)   // block value
}

// return: penetrates blocks, exits the function
f: (n: Int) -> Int = {
    if n < 0 {
        return 0     // penetrates the if and the function body, exits the function
    }
    n * 2            // tail expression
}
```

#### Is `name = { ... }` a function or a block value?

`name = { ... }` can be either a function definition (RFC-007 "zero-arg simplest") or a block-value
binding. The rule is **annotation takes priority, default to function** (RFC-010a Appendix D):

| Case                            | Result          | Example                            |
| ------------------------------- | --------------- | ---------------------------------- |
| Value is a Lambda (`=>`)        | Function        | `f = () => 5` → `f()` = 5          |
| Annotation is a function type   | Function        | `f: () -> Int = { 5 }` → `f()` = 5 |
| Annotation is non-function type | **Block value** | `x: Int = { 5 }` → `x` = 5         |
| No annotation                   | Function        | `f = { 5 }` → `f()` = 5            |

**Annotation is type**: `x: Int = ...` declares `x` to be `Int`, so `{ ... }` evaluates to `Int`;
`f: () -> Int = ...` declares `f` to be a function, so `{ ... }` is the function body.

To evaluate `{ ... }` immediately, **just write the target type** (no new syntax needed):

```yaoxiang
// Block value: immediate evaluation
x: Int = {
    y = 5
    y            // x = 5
}

// Function: no annotation means default
f = { 5 }        // f() = 5
```

#### Nested Function Types: Curried or Returning a Function?

A nested function type on the right side of `->` has two readings, distinguished by **parentheses**
(RFC-004):

| Notation                        | Meaning                | Call                    |
| ------------------------------- | ---------------------- | ----------------------- |
| `(a: Int) -> (b: Int) -> Int`   | **Curried**            | `f(1)(2)`               |
| `(a: Int) -> ((b: Int) -> Int)` | **Returns a function** | `g(1)` gives a function |

Basis: **annotation is type**. `g: (a: Int) -> ((b: Int) -> Int)` declares `g(1) : (b: Int) -> Int`,
so `g(1)` must **be** that function, not "the next argument".

```yaoxiang
// Curried: two parameter segments given one at a time
add: (a: Int) -> (b: Int) -> Int = { a + b }
add(1)(2)        // → 3

// Returns a function: outer segment of one parameter, the returned value is the function
adder: (n: Int) -> ((x: Int) -> Int) = (x) => x + n
adder(10)(5)     // → 15
h = adder(10)    // the returned function can be stored in a variable or passed
```

Unparenthesized nested `Fn` is always curried (including the type-parameter form from RFC-011):

```yaoxiang
identity: (T: Type) -> (x: T) -> T = (x) => x   // curried
identity(5)      // → 5
```

**Type check**: Parentheses declare the return type, so the body must produce that type.
`f: () -> (() -> Int) = { 7 }` is an error — expected `() -> Int`, actually got `Int` (E1002).

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
    validated = validate(data)?     // extracts the value on success, propagates on failure
    transform(validated)
}
```

### 2.12 Range Expression

```
RangeExpr   ::= Expr '..' Expr ('..' Expr)?
```

`..` creates a Range value (Range is a first-class value, not syntactic sugar).

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

> **step semantics**: In `a..b..c`, `c` is the step. The literal `c = 0` is rejected at compile
> time; a dynamic `c` with value 0 raises a zero-check at runtime (E6001 family; will be promoted to
> Result once the error system lands). `c < 0` is legal, and the interval direction reverses with
> the sign (`10..0..(-2)` descends).

### 2.13 ref Expression

```
RefExpr     ::= 'ref' Expr
```

`ref` creates shared ownership. The compiler automatically chooses Rc (single-task) or Arc
(cross-task); users do not need to worry about implementation details.

```yaoxiang
data = ref heavy_data
spawn { use(data) }   // cross-task: the compiler automatically chooses Arc
```

### 2.14 unsafe Expression

```
UnsafeExpr  ::= 'unsafe' Block
```

The `unsafe` block is used to define opaque types and operate on raw pointers. Use `return` to
return the type definition to the enclosing scope.

**Semantics**:

- Within `unsafe {}`, types can be defined and raw pointers can be operated on
- The returned type is usable outside the `unsafe {}` block
- Field access of the type requires unsafe permission

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

- `x = value`: search outward along the scope chain for x; if found, assign; if not, declare a new
  one
- `mut x = value`: explicitly declares a new mutable variable, forbidding the same name as an outer
  one
- Within the same scope, any name can be declared only once

> **Detailed definition**: The complete rules of scope and the variable declaration and shadowing
> mechanism are detailed in [Module System Specification](./modules.md#chapter-4-scope).

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

**Semantics**: `return` is a **non-local exit** that exits the nearest **function boundary**
(penetrating every block — including `if` / `while` / `for` / `match` / bare block / `spawn` /
`unsafe`), handing the value to the caller. It does **not "return to the block"**.

**Type**: `return e : Never` (where `e : T`). `Never <: T'` holds for any `T'` (principle of
explosion, see [Type System §2.2](./type-system.md)), so `return` can appear in any return-type
position without extra rules.

**Relationship to block evaluation**: A block's value is always the **tail expression** (see §2.9).
As a block, `{ return n }` has value `n` of type `Never`; at the same time, `return`'s effect is to
exit the function. **Both are true at once**, coexisting via the principle of explosion.

`return` together with the tail expression makes "early return" work, without needing additional
rules to designate `return` as a function-specific construct — see
[RFC-010a](../design/rfc/accepted/010a-tail-expression-and-return.md).

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 {
        return 1          // penetrates the if, exits the function (type Never)
    }
    n * factorial(n - 1)  // tail expression = block value
}
```

### 3.4 break Statement

```
BreakStmt   ::= 'break'
```

**Semantics**: Immediately terminates the innermost enclosing `while`/`for` loop, transferring
control to just after that loop body.

- **Only exits the nearest level**: `break`

> always acts on the innermost loop that contains it. In nested loops, when you need to jump out of
> multiple levels at once, extract the inner loop into a function and use `return`, or use a flag
> (break/continue have no labels; if loop labels are introduced in the future, they will go through
> the RFC process following the loop declaration-side syntax, decided together with the multi-exit
> design of the proof pipeline)

- **Only valid inside a loop body**: `break` may only appear inside a `while`/`for`

> loop body (including blocks/if/match nested within); appearing outside a loop is a compile-time
> error (E1102 `'break' outside of a loop`)

- **Does not affect termination proofs**: `while` loops must still be provably terminating (a
  decreases metric); `break`

> does not participate in termination arguments, and `while true { break }` will not be accepted

- **Borrowing semantics**: The control-flow edge of break participates in the structural cut of
  RFC-009a reverse-BFS liveness analysis (the skipped iteration does not participate in back-edge
  liveness derivation)

```yaoxiang
mut i = 0
while i < 10 {
    i = i + 1
    if i == 3 {
        break              // control transfers past the loop, i == 3
    }
}

// Nested loops: break only exits the inner one
while j < 3 {
    while k < 10 {
        if k == 2 { break }    // only terminates the inner loop
    }
    j = j + 1                  // every outer iteration reaches here
}
```

### 3.5 continue Statement

```
ContinueStmt::= 'continue'
```

**Semantics**: Skips the remaining statements in the current iteration and goes directly to the next
iteration of the innermost enclosing loop — `while` re-evaluates the condition, `for` takes the next
element.

- **Only acts on the nearest level**: Same as `break`, no labels
- **Only valid inside a loop body**: Appearing outside a loop is a compile-time error (E1102)

```yaoxiang
mut sum = 0
mut n = 0
while n < 5 {
    n = n + 1
    if n == 3 {
        continue           // skip the accumulation below; n == 3 is not counted
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

YaoXiang's for-loop semantics differ from traditional languages: **each iteration binds a new value,
rather than modifying the same variable**.

```yaoxiang
// Example: for i in 1..5
for i in 1..5 {
    print(i)
}
```

**Execution process**:

| Iteration | Behavior of the loop variable                                                      |
| --------- | ---------------------------------------------------------------------------------- |
| 1st       | Create a new binding `i = 1`, body executes, prints 1                              |
| 2nd       | Create a new binding `i = 2` (previous binding destroyed), body executes, prints 2 |
| 3rd       | Create a new binding `i = 3`, body executes, prints 3                              |
| 4th       | Create a new binding `i = 4`, body executes, prints 4                              |
| End       | Loop body ends, bindings destroyed                                                 |

**Key point**: After each iteration ends, the binding created in that iteration is destroyed. The
next iteration is a brand-new binding, with no relation to the previous iteration's binding.

#### 3.9.2 Difference between `for` and `for mut`

| Syntax              | Mutability of loop variable | Description                           |
| ------------------- | --------------------------- | ------------------------------------- |
| `for i in 1..5`     | Immutable                   | Cannot modify the binding in the body |
| `for mut i in 1..5` | Mutable                     | May modify the binding in the body    |

```yaoxiang
// Legal: each iteration binds a new value, no modification needed
for i in 1..5 {
    print(i)  // read the value of i
}

// Error: immutable binding, cannot modify
for i in 1..5 {
    i = i + 1  // error: cannot modify an immutable binding
}

// Legal: use for mut to allow modifying the binding
for mut i in 1..5 {
    i = i + 1  // modification allowed
}
```

#### 3.9.3 Shadowing Check

YaoXiang forbids variable shadowing. A for-loop variable cannot have the same name as a variable in
an outer scope:

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

This rule applies to all code blocks; see [4.3 Shadowing Rules](./modules.md#43-shadowing-rules) for
details.

#### 3.9.4 Comparison with Other Languages

| Language | for-loop variable semantics                                  |
| -------- | ------------------------------------------------------------ |
| YaoXiang | Each iteration binds a new value                             |
| Rust     | Modifies the same variable (requires `mut`)                  |
| Python   | Modifies the same variable (no `mut` needed)                 |
| C/C++    | Modifies the same variable (requires pointers or references) |

**Design rationale**: YaoXiang uses binding semantics because:

1. **Closer to natural semantics** In natural language, "for every element x in the set" implies
   each x is an independent individual. YaoXiang's `for i in 1..5` reads as "for every i in 1 to 5",
   and each iteration's i is a brand-new binding, consistent with human intuition.

2. **Avoid accidental modification** The default immutable binding semantics means the loop variable
   cannot be accidentally modified inside the loop body. You don't have to worry about somewhere in
   a complex loop body accidentally writing `i = ...` and causing a hard-to-trace bug.

3. **High-performance solution within reach** When you actually need to reuse a variable across
   iterations (e.g., an accumulator or cache), use `for mut` to switch to the mutable binding mode.
   This is clearer than implicit shared state — intent is expressed explicitly through syntax, not
   hidden in runtime behavior.

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

**spawn for**: Data-parallel loop.

```yaoxiang
results = spawn for item in items {
    process(item)
}
```

**spawn block captures outer-scope variables** (RFC-024 §2.3, value-capture semantics):

- An outer variable referenced in the block body =

> **Move value capture**: the value is snapshotted into the closure environment at the spawn
> creation point, and the block body reads it through env (LoadUpvalue)

- **Primitives** (Int/Float/Bool/Char) are value-copied; the outer variable is unaffected
- **Handle types** (Struct/String/List, etc.) snapshotted = handle copied, sharing the underlying
  object; in the Embedded runtime (default), same-thread same-heap, handles remain valid
- Sharing among multiple tasks requires explicit `ref` (§2.13, the compiler automatically chooses
  Rc/Arc)
- An outer variable referenced by `return` inside the block is captured in the same way

```yaoxiang
t1 = 1 + 1
t2 = 2 + 2
result = spawn {
    return t1 + t2    // t1/t2 are value-captured, result == 6
}
```

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
