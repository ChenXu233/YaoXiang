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
| Literal    | Fixed values                       | `42`, `"hello"`, `true`   |
| Operator   | Operator symbols                   | `+`, `-`, `*`, `/`        |
| Separator  | Syntax delimiters                  | `(`, `)`, `{`, `}`, `,`   |

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

YaoXiang's "reserved words" are divided into three layers, identified by the parser and type checker
at different stages:

#### 1.4.1 Literal Reserved Words

Identifiers that the parser treats as independent tokens, and cannot be used as ordinary
identifiers:

| Identifier | Type | Description                                                                                                   |
| ---------- | ---- | ------------------------------------------------------------------------------------------------------------- |
| `Type`     | —    | Meta-type keyword                                                                                             |
| `true`     | Bool | Boolean true                                                                                                  |
| `false`    | Bool | Boolean false                                                                                                 |
| `void`     | Void | Void literal (Unit value). Lowercase `void` is a value literal; uppercase `Void` is a type name (see §1.4.3). |

#### 1.4.2 Constructor Expressions

The following constructors are recognized by the parser in pattern matching and expression contexts:

| Constructor | Type   | Description                      |
| ----------- | ------ | -------------------------------- |
| `some(T)`   | Option | Option value variant constructor |
| `ok(T)`     | Result | Result success variant           |
| `err(E)`    | Result | Result error variant             |

#### 1.4.3 Built-in Type Names

The following type names are pre-registered by the type checker and can be used at type positions
without imports. The parser treats them as ordinary identifiers—**not reserved words, can be
shadowed by local bindings (not recommended)**.

| Type Name | Logical Correspondence | Description                                                                                                                                  |
| --------- | ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `Void`    | ⊤ (True/Unit)          | Zero-field product type, exactly one inhabitant (`void` literal, see §1.4.1)                                                                 |
| `Never`   | ⊥ (False/Empty type)   | Zero-variant sum type, zero inhabitants. No expression can produce a `Never` value. `Never <: T` holds for all `T` (principle of explosion). |
| `Int`     | —                      | Signed integer                                                                                                                               |
| `Float`   | —                      | Floating-point number                                                                                                                        |
| `Bool`    | —                      | Boolean value: `true` / `false`                                                                                                              |
| `Char`    | —                      | Unicode character                                                                                                                            |
| `String`  | —                      | String                                                                                                                                       |

### 1.5 Identifiers

Identifiers start with a letter or underscore; subsequent characters can be letters, digits, or
underscores. Identifiers are case-sensitive.

Special identifiers:

- `_` is used as a placeholder to indicate an ignored value
- Identifiers starting with an underscore denote private members

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
Array       ::= '[' Expr (',' Expr)* ']'   // When target type annotation is Array(T, N), literal resolves to fixed-length array
```

> #299/#300: Set has no literal grammar and no runtime representation—set type is in planning;
> complete following the Dict pattern (std.set + HeapValue::Set) when the requirement arises. The
> resolution of List/Dict literals is determined by the context type annotation: bare literals and
> `List(T)` annotations resolve to growable lists; `Array(T, N)` annotation directly on a literal
> resolves to a fixed-length array. Implicit List→Array conversion is forbidden.
>
> Array literal semantics (#300):
>
> - The number of elements must equal N, otherwise compile-time E1002; empty literal with non-zero N
>   is also rejected
> - Each element type must be compatible with T, otherwise compile-time E1002
> - Grammar form of N: only integer literal (possibly negative) or constant name; compound
>   expressions (e.g., `2+1`) are rejected at parse time
> - When N is a symbolic constant (function const parameter, e.g., `Array(Int, n)`), element count
>   validation is deferred to the refinement type phase
> - v1 nested array literals (`Array(Array(Int,2),2) = [[1,2],[3,4]]`) are rejected at compile time;
>   explicit construction layer-by-layer is required; recursive resolution is left to future
>   versions

#### 1.6.5 List Comprehension

```
ListComp    ::= '[' Expr 'for' Identifier 'in' Expr (',' Expr)* ('if' Expr)? ']'
```

> **#299 §3 Behavior Tightening (Migration Record)**: The iteration variable grammar was always
> `'for' Identifier 'in'`, but in the old implementation patterns went through full pratt
> parsing—after `'in'` was registered as an infix operator, `x` would consume `in items` as a
> membership expression. After the fix, non-identifier patterns fail to parse directly, no longer
> falling back to `_` (silent error swallowing) as the old implementation did. Impact: previously
> parseable expressions like `[x for (a, b) in pairs]` now report errors—this form never had defined
> behavior (the variable was always `_`), and the tightening direction is correct, with no semantic
> migration cost.

#### 1.6.6 Membership Test

```
Membership  ::= Expr 'in' Expr
```

> #299 §3: `in` is a binary relational operator that returns `Bool`—hit returns `true`, miss returns
> `false`, no error. Semantic split: `[]` asserts existence and retrieves the value (errors on
> failure), `in` asks whether it exists (miss is a normal `false`). Right operand coverage: List /
> Array / Dict (key set) / Tuple / String (substring) / Range. `in` is a first-class Hoare
> predicate, serving as the basis for compile-time provable propositions in the refinement type
> phase. (#300 Decision 4: Set is removed from the right operand list—Set has no runtime
> representation, see §1.6.4)

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
| 10         | `and` `or`                  | Left to right |
| 11         | `if...else`                 | Right to left |
| 12         | `=` `+=` `-=` `*=` `/=`     | Right to left |

> **Unary prefix operators** (`!` `-` `+`) bind tightly: only lower than call and member access,
> higher than all binary operators. Therefore `!a == b` ≡ `(!a) == b` (Zig-style semantics); `!` is
> a pure unary operation, does not participate in short-circuit control flow, and is orthogonal to
> the `and`/`or` keywords (short-circuit) (authoritative definition in RFC-010).

> **Range Binding Strength (#299 §3 / #300 Item F)**: `..` binding strength is (6, 7)—left 6 lower
> than addition (7), right 7 swallows addition but not sibling `..`. Before and after change
> comparison:
>
> | Expression   | Before Change (Level 1, right-associative)                                                    | After Change ((6,7), left-associative)                                         |
> | ------------ | --------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------ |
> | `x in 1..10` | `x in 1..10` (`in` right operand level 4, `..` level 1 cannot swallow, actually cannot parse) | `x in (1..10)`—range as a whole serves as right operand of `in`                |
> | `0..n+2`     | `(0..n)+2` (right-associative trap: upper bound is eaten, `for` loop directly E3004)          | `0..(n+2)`—upper bound is an arithmetic expression                             |
> | `a == b..c`  | `a == (b..c)` (`..` level 1 < `==` level 3, naturally whole)                                  | `a == (b..c)`—**semantics unchanged**, `..` still higher than comparison level |
> | `1..2*3`     | `(1..2)*3`                                                                                    | `1..(2*3)`—upper bound is an arithmetic expression                             |
> | `a..b..c`    | `a..(b..c)` (right-associative chained, meaningless Range inside Range)                       | `(a..b)..c`—**step form** (`c` is step size, #300 Item I)                      |
>
> Net effect: compound upper bound `for i in 0..n+2` changes from "parses successfully but E3004" to
> "directly usable"; `x in 1..10` changes from "cannot parse" to "range check"; `a..b..c` changes
> from "meaningless nesting" to "step component". Level 6 falls between `+` (level 5) and `<<`
> (level 7), mathematical convention: range is a tight-binding construct, upper bound naturally is a
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

> **Statement Termination Rules**: The separation and newline behavior between Stmts (explicit `;`
> separator, newline termination, line continuation exceptions, line-leading `(`/`[` never merging)
> is defined by [RFC-038](../design/rfc/draft/038-statement-termination.md).

**Unified Semantics**: All `{}` blocks have consistent return semantics:

| Block Type    | return Semantics        | Default Return |
| ------------- | ----------------------- | -------------- |
| Ordinary `{}` | Returns value           | Void           |
| `unsafe {}`   | Returns type definition | Void           |
| `spawn {}`    | Returns result          | Void           |

**Core Principles**:

- `return` in `{}` always returns the content to the enclosing scope
- The default without `return` is returning `Void`
- Expression form `= expr` directly returns the value

```yaoxiang
// Ordinary {} block: return returns value
result = {
    x = compute()
    return x  // Returns value to enclosing scope
}

// unsafe {} block: return returns type definition
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void
    }
    return SqliteDb  // Returns type definition to enclosing scope
}

// spawn {} block: return returns result
(a, b) = spawn {
    result1 = fetch("url1"),
    result2 = fetch("url2")
    return (result1, result2)  // Returns result to enclosing scope
}
```

### 2.10 Lambda Expression

```
Lambda      ::= '(' ParamList? ')' '=>' Expr
            |  '(' ParamList? ')' '=>' Block
```

### 2.11 Error Propagation Operator

```
ErrorPropagate ::= Expr '?'
```

The `?` operator is a postfix operator, with the same precedence as `.`. For `Result(T, E)` types:

- Extracts the value `v` on `Ok(v)` and continues execution
- Propagates the error upward (`return Err(e)`) on `Err(e)`

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

`..` creates a range value (#300 Item I: Range is a first-class value, not syntactic sugar).

```yaoxiang
for i in 0..10 { print(i) }
slice = array[0..5]

// Range is a value: bind, pass, member check
r = 1..10
assert.assert(5 in r, "membership")
for i in r { print(i) }

// step form (third component, default 1)
for i in 0..10..2 { print(i) }  // 0, 2, 4, 6, 8
for i in 10..0..(-2) { print(i) }  // 10, 8, 6, 4, 2
```

> **step Semantics**: In `a..b..c`, `c` is the step size. `c = 0` literal is rejected at compile
> time; dynamic `c` is not checked at runtime (E6001 family, upgraded to Result after #301 error
> system landing). `c < 0` is legal, range direction reverses with sign (`10..0..(-2)` decreases).

### 2.13 ref Expression

```
RefExpr     ::= 'ref' Expr
```

`ref` creates a shared holding. The compiler automatically chooses Rc (single-task) or Arc
(cross-task); the user does not need to care about implementation details.

```yaoxiang
data = ref heavy_data
spawn { use(data) }   // Cross-task: compiler automatically chooses Arc
```

### 2.14 unsafe Expression

```
UnsafeExpr  ::= 'unsafe' Block
```

The `unsafe` block is used to define opaque types and operate on raw pointers. Use `return` to
return the type definition to the enclosing scope.

**Semantics**:

- Types and raw pointer operations can be defined in `unsafe {}`
- Returned types are usable outside `unsafe {}`
- Type field access requires unsafe permission

```yaoxiang
// Define opaque type in unsafe block
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

**Basic Rules**:

- Each `{}` block creates a scope
- Inner scopes can access variables from outer scopes
- Outer scopes cannot access variables from inner scopes
- Variable declaration follows the "assignment first" principle

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

**Variable Declaration and Shadowing**:

- `x = value`: Search outward along the scope chain for x, assign if found, declare new if not found
- `mut x = value`: Explicit new mutable declaration, prohibited from having the same name as an
  outer one
- Any name can only be declared once in the same scope

> **Detailed Definition**: Complete rules for scope, variable declaration and shadowing mechanism
> are detailed in [Module System Specification](./modules.md#chapter-4-scope).

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

**Semantics**: `return` is used to return a value from a code block. Without `return`, the code
block defaults to returning `Void`.

### 3.4 break Statement

```
BreakStmt   ::= 'break'
```

**Semantics**: Immediately terminates the innermost `while`/`for` loop containing it; control flow
transfers to after the loop body.

- **Only exits the nearest level**: `break` always acts on the innermost loop containing it. In
  nested loops when one-time multi-level break is needed, extract the inner loop into a function and
  use `return`, or use a flag (#314 decision: break/continue have no labels; if loop labels are
  introduced in the future, the loop declaration-side syntax will go through the RFC process,
  decided together with the multi-exit design of the proof pipeline)
- **Only within loop body**: `break` can only appear within `while`/`for` loop bodies (including
  blocks/if/match nested in the body); appearing outside a loop reports a compile error (E1102
  `'break' outside of a loop`)
- **Does not affect termination proof**: `while` loops still need to be provably terminating
  (decreases measure); `break` does not participate in termination arguments, `while true { break }`
  will not be accepted
- **Borrow Semantics**: Control flow edges of break participate in the structural cut of RFC-009a
  reverse BFS liveness analysis (the exiting iteration does not participate in back-edge liveness
  derivation)

```yaoxiang
mut i = 0
while i < 10 {
    i = i + 1
    if i == 3 {
        break              // Control flow transfers to after the loop, i == 3
    }
}

// Nested loop: break only exits the inner loop
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

**Semantics**: Skips the remaining statements in the current iteration, directly entering the next
iteration of the innermost loop containing it—`while` returns to condition re-evaluation, `for`
takes the next element.

- **Only acts on the nearest level**: Same as `break`, no label (#314)
- **Only within loop body**: Appearing outside a loop reports a compile error (E1102)

```yaoxiang
mut sum = 0
mut n = 0
while n < 5 {
    n = n + 1
    if n == 3 {
        continue           // Skip the following accumulation, n == 3 is not counted
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

YaoXiang's for loop semantics differ from traditional languages: **each iteration binds a new value,
not modifying the same variable**.

```yaoxiang
// Example: for i in 1..5
for i in 1..5 {
    print(i)
}
```

**Execution Process**:

| Iteration | Behavior of the Loop Variable                                                            |
| --------- | ---------------------------------------------------------------------------------------- |
| 1st       | Create new binding `i = 1`, loop body executes, prints 1                                 |
| 2nd       | Create new binding `i = 2` (previous binding is destroyed), loop body executes, prints 2 |
| 3rd       | Create new binding `i = 3`, loop body executes, prints 3                                 |
| 4th       | Create new binding `i = 4`, loop body executes, prints 4                                 |
| End       | Loop body ends, binding destroyed                                                        |

**Key Point**: After each iteration ends, the binding created in that iteration is destroyed. The
next iteration is a brand new binding, with no relationship to the previous iteration's binding.

#### 3.9.2 Difference Between for and for mut

| Syntax              | Loop Variable Mutability | Description                            |
| ------------------- | ------------------------ | -------------------------------------- |
| `for i in 1..5`     | Immutable                | Cannot modify binding within loop body |
| `for mut i in 1..5` | Mutable                  | Can modify binding within loop body    |

```yaoxiang
// Legal: each iteration binds new value, no modification needed
for i in 1..5 {
    print(i)  // Read value of i
}

// Error: immutable binding, cannot modify
for i in 1..5 {
    i = i + 1  // Error: cannot modify immutable binding
}

// Legal: use for mut to allow modifying binding
for mut i in 1..5 {
    i = i + 1  // Modification allowed
}
```

#### 3.9.3 Shadowing Check

YaoXiang prohibits variable shadowing. A for loop variable cannot have the same name as a variable
in an outer scope:

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

| Language | for Loop Variable Semantics                                  |
| -------- | ------------------------------------------------------------ |
| YaoXiang | Each iteration binds a new value                             |
| Rust     | Modifies the same variable (requires mut)                    |
| Python   | Modifies the same variable (no mut needed)                   |
| C/C++    | Modifies the same variable (requires pointers or references) |

**Design Rationale**: YaoXiang adopts binding semantics because:

1. **More aligned with natural semantics** In natural language, "for each element x in the
   collection" means each x is an independent individual. YaoXiang's `for i in 1..5` reads as "for
   each i from 1 to 5", where i in each iteration is a brand new binding, which aligns with human
   intuition.

2. **Avoid accidental modification** The default immutable binding semantics mean the loop variable
   cannot be accidentally modified within the loop body. No need to worry about writing `i = ...`
   somewhere in a complex loop body causing hard-to-trace bugs.

3. **High-performance solution within reach** When it is truly necessary to reuse variables across
   iterations (e.g., accumulators, caches), declare with `for mut` to switch to mutable binding
   mode. This is clearer than implicit shared state—intent is expressed explicitly through syntax,
   not hidden in runtime behavior.

### 3.10 spawn Statement

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' 'mut'? Identifier 'in' Expr '{' Expr '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
```

**spawn block**: Explicitly declares a concurrent region; expressions within the block execute
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

- Outer variable referenced by block body = **Move value capture**: value is snapshotted into the
  closure environment at the spawn creation point, block body reads via env (LoadUpvalue)
- **Primitives** (Int/Float/Bool/Char) are value-copied, outer variables are unaffected
- **Handle types** (Struct/String/List etc.) snapshot = handle copy, sharing underlying object;
  Embedded runtime (default) same-thread same-heap, handle is valid
- Sharing between multiple tasks requires explicit `ref` (§2.13, compiler automatically chooses
  Rc/Arc)
- Outer variables referenced by `return` within the block are also captured

```yaoxiang
t1 = 1 + 1
t2 = 2 + 2
result = spawn {
    return t1 + t2    // t1/t2 value capture, result == 6
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
