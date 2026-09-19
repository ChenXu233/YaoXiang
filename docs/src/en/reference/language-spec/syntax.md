# Syntax Specification

This document defines the syntax specification of the YaoXiang programming language, including
lexical structure, grammar rules, and operator precedence.

---

## Chapter 1: Lexical Structure

### 1.1 Source File

YaoXiang source files must use UTF-8 encoding. Source files typically use the `.yx` extension.

### 1.2 Token Classification

| Category   | Description                    | Examples                  |
| ---------- | ------------------------------ | ------------------------- |
| Identifier | Starts with letter/underscore  | `x`, `_private`, `my_var` |
| Keyword    | Language-defined reserved word | `Type`, `pub`, `use`      |
| Literal    | Fixed value                    | `42`, `"hello"`, `true`   |
| Operator   | Operator symbol                | `+`, `-`, `*`, `/`        |
| Delimiter  | Syntax delimiter               | `(`, `)`, `{`, `}`, `,`   |

### 1.3 Keywords

YaoXiang defines a minimal set of keywords:

```
pub    use    spawn
ref    mut    if     else
else   match  while  for    return
break  continue as    in     unsafe
```

These keywords have special meaning in any context and cannot be used as identifiers.

### 1.4 Reserved Words

YaoXiang's "reserved words" are divided into three layers, recognized by the parser and type checker
at different stages:

#### 1.4.1 Literal Reserved Words

Identifiers that are separate tokens in the parser and cannot be used as ordinary identifiers:

| Identifier | Type | Description                                                                                                   |
| ---------- | ---- | ------------------------------------------------------------------------------------------------------------- |
| `Type`     | —    | Meta type keyword                                                                                             |
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
without import. The parser treats them as ordinary identifiers——**not reserved words, can be
shadowed by local bindings (not recommended)**.

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

Identifiers start with a letter or underscore, followed by letters, digits, or underscores.
Identifiers are case-sensitive.

Special identifiers:

- `_` is used as a placeholder, indicating an ignored value
- Identifiers starting with underscore indicate private members

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
Array       ::= '[' Expr (',' Expr)* ']'   // Resolves to fixed-length array when target type is annotated as Array(T, N)
```

> Set has no literal grammar, no runtime representation——set types are planned; when the need
> arises, complete them following the Dict pattern (std.set + HeapValue::Set). The resolution of
> List/Dict literals is determined by context type annotation: bare literals and `List(T)`
> annotations resolve to growable lists; `Array(T, N)` annotations applied directly to a literal
> resolve to a fixed-length array. Implicit List→Array conversion is forbidden.
>
> Array literal semantics:
>
> - The number of elements must equal N; otherwise compile-time E1002; empty literals with non-zero
>   N are also rejected
> - Each element type must be compatible with T; otherwise compile-time E1002
> - N grammar form: only integer literals (can be negative) or constant names; complex expressions
>   (e.g., `2+1`) are rejected at parse time
> - When N is a symbolic constant (function const parameter, e.g., `Array(Int, n)`), element count
>   validation is deferred to the refinement type stage
> - v1 nested array literals (`Array(Array(Int,2),2) = [[1,2],[3,4]]`) are rejected at compile-time
>   and require explicit construction layer by layer; recursive resolution is left for future
>   versions

#### 1.6.5 List Comprehension

```
ListComp    ::= '[' Expr 'for' Identifier 'in' Expr (',' Expr)* ('if' Expr)? ']'
```

> **Behavior tightening (migration note)**: The iteration variable grammar is already
> `'for' Identifier 'in'`, but the old implementation processed pattern through full pratt
> parsing——after `'in'` was registered as an infix operator, `x` would swallow `in items` into a
> membership expression. After the fix, non-identifier patterns directly fail to parse, no longer
> falling back to `_` like the old implementation (silently swallowing errors). Impact: the
> previously parseable `[x for (a, b) in pairs]` now reports an error——this form never had defined
> behavior (variable was always `_`), the tightening direction is correct with no semantic migration
> cost.

#### 1.6.6 Membership Test

```
Membership  ::= Expr 'in' Expr
```

> `in` is a binary relational operator returning `Bool`——`true` on hit, `false` on miss, no error
> reported. Semantic split: `[]` asserts existence and retrieves value (errors on failure), `in`
> asks whether something exists (miss is a normal `false`). Right operand coverage: List / Array /
> Dict (key set) / Tuple / String (substring) / Range (interval). `in` is a first-class Hoare
> predicate, serving as the base for compile-time provable propositions in the refinement type
> stage. (Set is removed from the right operand list——Set has no runtime representation, see §1.6.4)

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

> **Unary prefix operators** (`!` `-` `+`) bind tightly: only lower than call and member access,
> higher than all binary operators. Therefore `!a == b` ≡ `(!a) == b` (Zig-style semantics); `!` is
> pure unary operation, does not participate in short-circuit control flow, and is orthogonal to the
> `and`/`or` keywords (short-circuit) (RFC-010 authoritative definition).

> **Range binding power**: `..` has binding power (6, 7)——left 6 is lower than addition (7), right 7
> absorbs addition but not same-level `..`. Comparison before and after the change:
>
> | Expression   | Before change (level 1, right-assoc)                                                        | After change ((6,7), left-assoc)                                                |
> | ------------ | ------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
> | `x in 1..10` | `x in 1..10` (`in` right operand level 4, `..` level 1 cannot absorb, actually unparseable) | `x in (1..10)`——interval as a whole as `in` right operand                       |
> | `0..n+2`     | `(0..n)+2` (right-assoc trap: upper bound eaten, `for` loop directly E3004)                 | `0..(n+2)`——upper bound is an arithmetic expression                             |
> | `a == b..c`  | `a == (b..c)` (`..` level 1 < `==` level 3, naturally as a whole)                           | `a == (b..c)`——**semantics unchanged**, `..` still higher than comparison level |
> | `1..2*3`     | `(1..2)*3`                                                                                  | `1..(2*3)`——upper bound is an arithmetic expression                             |
> | `a..b..c`    | `a..(b..c)` (right-assoc chain, meaningless Range nesting)                                  | `(a..b)..c`——**step form** (`c` is step)                                        |
>
> Net effect: composite upper bound `for i in 0..n+2` changes from "parses but E3004" to "directly
> usable"; `x in 1..10` changes from "unparseable" to "interval check"; `a..b..c` changes from
> "meaningless nesting" to "step component". Level 6 falls between `+` (level 5) and `<<` (level 7),
> mathematical convention: intervals are tightly bound constructs, the upper bound is naturally a
> complete arithmetic expression.

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

> **Statement termination rules**: The separation and newline behavior between Stmts (`;` explicit
> separator, newline termination, line continuation exceptions, leading `(`/`[` never merging) are
> defined by [RFC-038](../design/rfc/draft/038-statement-termination.md).

**Unified semantics**: The value of all `{}` blocks is given by the **tail expression**, `return` is
a non-local exit of type `Never`.

| Block Type  | Value Outlet    | Empty Block `{}` |
| ----------- | --------------- | ---------------- |
| Plain `{}`  | Tail expression | `Void`           |
| `unsafe {}` | Tail expression | `Void`           |
| `spawn {}`  | Tail expression | `Void`           |

**Core principles** (see [RFC-010a](../design/rfc/accepted/010a-tail-expression-and-return.md) for
details):

- **Block value = tail expression** (the last expression), single outlet, no exceptions
- When the last position is an **assignment statement**, the block value is `Void`; if you want
  `Void`, write it explicitly
- **`return` exits the nearest function boundary** (pierces through all blocks, does not "return to
  the block"), type `Never`; `Never <: T` holds for any type (principle of explosion), so it can
  appear in any return type position
- Expression form `= expr` directly gives the value

```yaoxiang
// Plain {} block: value is given by tail expression
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

// return: pierces through the block, exits the function
f: (n: Int) -> Int = {
    if n < 0 {
        return 0     // Pierces through if and the function body, exits the function
    }
    n * 2            // Tail expression
}
```

#### Is `name = { ... }` a Function or a Block Value?

`name = { ... }` can be either a function definition (RFC-007 "zero-arg simplest form") or a block
value binding. The arbitration is **annotation first, function by default** (RFC-010a Appendix D):

| Case                              | Result          | Example                            |
| --------------------------------- | --------------- | ---------------------------------- |
| Value is Lambda (`=>`)            | Function        | `f = () => 5` → `f()` = 5          |
| Annotation is a function type     | Function        | `f: () -> Int = { 5 }` → `f()` = 5 |
| Annotation is a non-function type | **Block value** | `x: Int = { 5 }` → `x` = 5         |
| No annotation                     | Function        | `f = { 5 }` → `f()` = 5            |

**Annotation is type**: `x: Int = ...` declares `x` to be `Int`, so `{ ... }` evaluates to `Int`;
`f: () -> Int = ...` declares `f` to be a function, so `{ ... }` is the function body.

To make `{ ... }` evaluate immediately, **just write the target type** (no new syntax needed):

```yaoxiang
// Block value: immediate evaluation
x: Int = {
    y = 5
    y            // x = 5
}

// Function: no annotation defaults to function
f = { 5 }        // f() = 5
```

#### Nested Function Type: Currying or Returning a Function?

The nested function type on the right side of `->` has two readings, distinguished by
**parentheses** (RFC-004):

| Notation                        | Meaning             | Call                      |
| ------------------------------- | ------------------- | ------------------------- |
| `(a: Int) -> (b: Int) -> Int`   | **Currying**        | `f(1)(2)`                 |
| `(a: Int) -> ((b: Int) -> Int)` | **Return function** | `g(1)` returns a function |

Basis: **Annotation is type**. `g: (a: Int) -> ((b: Int) -> Int)` declares `g(1) : (b: Int) -> Int`,
so `g(1)` must **be** that function, not "the next segment of parameters".

```yaoxiang
// Currying: two segments of parameters given layer by layer
add: (a: Int) -> (b: Int) -> Int = { a + b }
add(1)(2)        // → 3

// Return function: outer one segment of parameters, the return value is the function
adder: (n: Int) -> ((x: Int) -> Int) = (x) => x + n
adder(10)(5)     // → 15
h = adder(10)    // The returned function can be stored in a variable and passed around
```

Unparenthesized nested `Fn` is always currying (including the type parameter form from RFC-011):

```yaoxiang
identity: (T: Type) -> (x: T) -> T = (x) => x   // Currying
identity(5)      // → 5
```

**Type check**: Parentheses declare the return type, the body must produce that type.
`f: () -> (() -> Int) = { 7 }` is an error——expected `() -> Int`, actually got `Int` (E1002).

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

- On `Ok(v)`, extract the value `v` and continue execution
- On `Err(e)`, propagate the error upward (`return Err(e)`)

```yaoxiang
process: (data: Data) -> Result(Data, Error) = {
    validated = validate(data)?     // Extract value on success, propagate upward on failure
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

// Step form (third component, default 1)
for i in 0..10..2 { print(i) }  // 0, 2, 4, 6, 8
for i in 10..0..(-2) { print(i) }  // 10, 8, 6, 4, 2
```

> **Step semantics**: In `a..b..c`, `c` is the step. Literal `c = 0` is rejected at compile-time;
> dynamic `c` is not checked at runtime (E6001 family; will be elevated to Result once the error
> system lands). `c < 0` is legal, the interval direction reverses with the sign (`10..0..(-2)`
> decreases).

### 2.13 ref Expression

```
RefExpr     ::= 'ref' Expr
```

`ref` creates shared ownership. The compiler automatically selects Rc (single-task) or Arc
(cross-task); users don't need to care about implementation details.

```yaoxiang
data = ref heavy_data
spawn { use(data) }   // Cross-task: compiler automatically selects Arc
```

### 2.14 unsafe Expression

```
UnsafeExpr  ::= 'unsafe' Block
```

`unsafe` blocks are used to define opaque types and operate on raw pointers. Use `return` to return
the type definition to the enclosing scope.

**Semantics**:

- Types can be defined and raw pointers operated on inside `unsafe {}`
- Returned types are usable outside `unsafe {}`
- Field access on these types requires unsafe permission

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

- `x = value`: look up `x` along the scope chain outward; if found, assign; if not, declare new
- `mut x = value`: explicit new mutable declaration, forbidden to have the same name as outer
- Any name in the same scope can be declared only once

> **Detailed definition**: The complete rules for scope, variable declaration, and shadowing
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

**Semantics**: `return` is a **non-local exit** that exits the nearest **function boundary**
(pierces through all blocks——including `if` / `while` / `for` / `match` / bare blocks / `spawn` /
`unsafe`), handing the value to the caller. It does **not "return to the block"**.

**Type**: `return e : Never` (where `e : T`). `Never <: T'` holds for any `T'` (principle of
explosion, see [Type System §2.2](./type-system.md)), so `return` can appear in any return type
position without additional rules.

**Relationship with block evaluation**: A block's value is always the **tail expression** (see
§2.9). `{ return n }` as a block has value `n` and type `Never`; meanwhile the effect of `return` is
to exit the function. **Both things hold simultaneously**, coexisting via the principle of
explosion.

`return` together with the tail expression make "early return" work, without needing extra rules for
`return` to specifically refer to the function——see
[RFC-010a](../design/rfc/accepted/010a-tail-expression-and-return.md).

```yaoxiang
factorial: (n: Int) -> Int = {
    if n <= 1 {
        return 1          // Pierces through if, exits the function (type Never)
    }
    n * factorial(n - 1)  // Tail expression = block value
}
```

### 3.4 break Statement

```
BreakStmt   ::= 'break'
```

**Semantics**: Immediately terminates the innermost `while`/`for` loop, with control flow
transferring to after the loop body.

- **Only exits the nearest level**: `break` always acts on the innermost loop containing it. To
  break out of multiple levels in nested loops, extract the inner loop into a function and use
  `return`, or use a flag (break/continue carry no label; if loop labels are introduced in the
  future, they will follow the RFC process on the loop declaration side, decided together with the
  multi-exit design of the proof pipeline)
- **Only inside loop bodies**: `break` can only appear inside `while`/`for` loop bodies (including
  nested blocks/if/match within the body); appearing outside a loop is a compile error (E1102
  `'break' outside of a loop`)
- **Does not affect termination proofs**: `while` loops must still be provably terminating
  (decreases measure); `break` does not participate in termination arguments, so
  `while true { break }` will not be accepted
- **Borrowing semantics**: The control flow edge of break participates in the structural cut of
  RFC-009a reverse BFS liveness analysis (skipped iterations do not participate in back-edge
  liveness derivation)

```yaoxiang
mut i = 0
while i < 10 {
    i = i + 1
    if i == 3 {
        break              // Control flow transfers to after the loop, i == 3
    }
}

// Nested loop: break only exits the inner one
while j < 3 {
    while k < 10 {
        if k == 2 { break }    // Only terminates the inner loop
    }
    j = j + 1                  // Each outer iteration executes to here
}
```

### 3.5 continue Statement

```
ContinueStmt::= 'continue'
```

**Semantics**: Skips the remaining statements in the current iteration and directly enters the next
iteration of the innermost loop——`while` returns to the condition re-judgment, `for` takes the next
element.

- **Only acts on the nearest level**: Same as `break`, no label
- **Only inside loop bodies**: Appearing outside a loop is a compile error (E1102)

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

YaoXiang's for loop semantics differ from traditional languages: **each iteration binds a new value,
rather than modifying the same variable**.

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
next iteration is a completely new binding, with no relation to the binding from the previous
iteration.

#### 3.9.2 Difference Between `for` and `for mut`

| Syntax              | Loop Variable Mutability | Description                                     |
| ------------------- | ------------------------ | ----------------------------------------------- |
| `for i in 1..5`     | Immutable                | Binding cannot be modified within the loop body |
| `for mut i in 1..5` | Mutable                  | Binding can be modified within the loop body    |

```yaoxiang
// Legal: each iteration binds a new value, no modification needed
for i in 1..5 {
    print(i)  // Read the value of i
}

// Error: immutable binding, cannot be modified
for i in 1..5 {
    i = i + 1  // Error: cannot modify immutable binding
}

// Legal: use for mut to allow modifying the binding
for mut i in 1..5 {
    i = i + 1  // Modification allowed
}
```

#### 3.9.3 Shadowing Check

YaoXiang forbids variable shadowing. The for loop variable cannot have the same name as a variable
in the outer scope:

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

This rule applies to all code blocks, see [4.3 Shadowing Rules](./modules.md#43-遮蔽规则) for
details.

#### 3.9.4 Comparison with Other Languages

| Language | for Loop Variable Semantics                               |
| -------- | --------------------------------------------------------- |
| YaoXiang | Each iteration binds a new value                          |
| Rust     | Modifies the same variable (requires `mut`)               |
| Python   | Modifies the same variable (no `mut` needed)              |
| C/C++    | Modifies the same variable (needs pointers or references) |

**Design rationale**: YaoXiang adopts binding semantics because:

1. **More aligned with natural semantics** In natural language, "for each element x in the
   collection" means each x is an independent individual. YaoXiang's `for i in 1..5` is read as "for
   each i in 1 to 5", where the i in each iteration is a completely new binding, which aligns with
   human intuition.

2. **Avoids accidental modification** The default immutable binding semantics means the loop
   variable cannot be accidentally modified within the loop body. No need to worry about a `i = ...`
   written somewhere in a complex loop body causing a hard-to-trace bug.

3. **High-performance solution within reach** When you really need to reuse a variable between
   iterations (e.g., accumulator, cache), use `for mut` to switch to mutable binding mode. This is
   clearer than implicit shared state——intent is expressed explicitly through syntax, not hidden in
   runtime behavior.

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

- Outer variables referenced by the block body = **Move value capture**: the value is snapshotted
  into the closure environment at spawn creation point, the block body reads via env (LoadUpvalue)
- **Primitives** (Int/Float/Bool/Char) are value-copied, outer variables are not affected
- **Handle types** (Struct/String/List etc.) snapshot = handle copy, sharing the underlying object;
  Embedded runtime (default) has the same thread and same heap, handles are valid
- Sharing between multiple tasks requires explicit `ref` (§2.13, compiler automatically selects
  Rc/Arc)
- Outer variables referenced by `return` inside the block are also captured

```yaoxiang
t1 = 1 + 1
t2 = 2 + 2
result = spawn {
    return t1 + t2    // t1/t2 value captured, result == 6
}
```

---

### 3.11 Program Entry and Top-Level Statements

A source file has two roles in the compiler's view, **determined by whether `yaoxiang.toml`
exists**:

| Role       | Determination                              | Program Body                                                                      |
| ---------- | ------------------------------------------ | --------------------------------------------------------------------------------- |
| **Script** | Single-file direct run, no `yaoxiang.toml` | **Top-level statements** (executed in writing order); `main` is a regular binding |
| **Bin**    | `yaoxiang.toml` exists                     | **`main` function**; top-level must not have executable statements                |

#### Script: Top-level Statements are the Program

Without a manifest, the file is a "script", **top-level statements execute in writing order**:

```yaoxiang
use std.io
io.println("hello")          // Direct execution
x: Int = { 42 }              // Top-level binding: runtime initialization
io.println(x)                // 42
```

`main` is **not special** in this mode——it is just a regular binding. To make `main` run, you must
explicitly call it:

```yaoxiang
use std.io
main: () -> Void = { io.println("only runs if called") }

main()                       // ← This line must be written
```

> **Why not auto-call `main`?** Top-level statements are already the program body. If `main` is also
> implicitly called, a script that explicitly writes `main()` would execute twice. The two rules
> cannot coexist, so under Script there is only "top-level statements" as the single entry point.

#### Bin: `main` is the Entry

When a manifest exists, the file is an "executable target", in which case:

- `main` must be defined, and it must be a function (signature must be callable with zero arguments)
- Top-level does not allow executable statements——the program body is `main`

```yaoxiang
main: () -> Void = {
    print("hello")
}
```

Missing `main` or `main` not being a function are both compile errors (the former: without `main`
all functions are unreachable; the latter: value binding will not be called).

> **Library files**: Files `use`d by other files, or files pointed to by `[lib].path` / `[exports]`
> do not require `main`——they are not program entries.

#### Initialization of Top-Level Bindings

The initialization value of top-level bindings **is evaluated at runtime**, not required to be a
compile-time constant:

```yaoxiang
answer: Int = { 42 }              // Block value
inc: (Int) -> Int = (x) => x + 1
computed: Int = inc(41)           // Function call
```

Initialization executes in **dependency order**, independent of writing order:

```yaoxiang
derived: Int = base * 3           // References later-declared base
base: Int = 7                     // Initializes first (topological sort)
```

Circular dependencies are compile errors (the names on the cycle will be listed):

```yaoxiang
a: Int = b + 1
b: Int = a + 1                    // Error: a → b → a
```

> **Design basis**: RFC-029f (File Role Model), RFC-010a Appendix D (Block Binding Arbitration).

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
