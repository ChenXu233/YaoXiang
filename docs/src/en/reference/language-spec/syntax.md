# Syntax Specification

This document defines the syntax specification of the YaoXiang programming language, including
lexical structure, grammar rules, and operator precedence.

---

## Chapter 1: Lexical Structure

### 1.1 Source Files

YaoXiang source files must use UTF-8 encoding. Source files typically use the `.yx` extension.

### 1.2 Lexical Token Categories

| Category   | Description                      | Examples                  |
| ---------- | -------------------------------- | ------------------------- |
| Identifier | Starts with letter or underscore | `x`, `_private`, `my_var` |
| Keyword    | Language-defined reserved word   | `Type`, `pub`, `use`      |
| Literal    | Fixed value                      | `42`, `"hello"`, `true`   |
| Operator   | Operator symbol                  | `+`, `-`, `*`, `/`        |
| Delimiter  | Syntax delimiter                 | `(`, `)`, `{`, `}`, `,`   |

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

YaoXiang's "reserved words" are split into three layers, identified at different stages by the
parser and the type checker:

#### 1.4.1 Literal Reserved Words

Literal identifiers with independent tokens in the parser, which cannot be used as ordinary
identifiers:

| Identifier | Belongs to Type | Description                                                                                                   |
| ---------- | --------------- | ------------------------------------------------------------------------------------------------------------- |
| `Type`     | —               | Meta type keyword                                                                                             |
| `true`     | Bool            | Boolean true value                                                                                            |
| `false`    | Bool            | Boolean false value                                                                                           |
| `void`     | Void            | Void literal (Unit value). Lowercase `void` is a value literal; uppercase `Void` is a type name (see §1.4.3). |

#### 1.4.2 Constructor Expressions

The following constructors are recognized by the parser in pattern matching and expression contexts:

| Constructor | Belongs to Type | Description                      |
| ----------- | --------------- | -------------------------------- |
| `some(T)`   | Option          | Option value variant constructor |
| `ok(T)`     | Result          | Result success variant           |
| `err(E)`    | Result          | Result error variant             |

#### 1.4.3 Builtin Type Names

The following type names are pre-registered by the type checker and can be used in type positions
without import. The parser treats them as ordinary identifiers—**they are not reserved words and can
be shadowed by local bindings (not recommended)**.

| Type Name | Logical Correspondence | Description                                                                                                                                  |
| --------- | ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `Void`    | ⊤ (true/Unit)          | Zero-field product type, with exactly one inhabitant (the `void` literal, see §1.4.1)                                                        |
| `Never`   | ⊥ (false/empty type)   | Zero-variant sum type, zero inhabitants. No expression can produce a `Never` value. `Never <: T` holds for all `T` (principle of explosion). |
| `Int`     | —                      | Signed integer                                                                                                                               |
| `Float`   | —                      | Floating-point number                                                                                                                        |
| `Bool`    | —                      | Boolean value: `true` / `false`                                                                                                              |
| `Char`    | —                      | Unicode character                                                                                                                            |
| `String`  | —                      | String                                                                                                                                       |

### 1.5 Identifiers

Identifiers start with a letter or underscore, and subsequent characters may be letters, digits, or
underscores. Identifiers are case-sensitive.

Special identifiers:

- `_` is used as a placeholder, indicating that a value is ignored
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
Array       ::= '[' Expr (',' Expr)* ']'   // When the target type is annotated as Array(T, N), the literal resolves to a fixed-length array
```

> Set has no literal grammar and no runtime representation—set types are planned; when the need
> arises, complete them following the Dict pattern (std.set + HeapValue::Set). The resolution point
> of List/Dict literals is determined by context type annotation: bare literals and `List(T)`
> annotations resolve to growable lists; `Array(T, N)` annotation applied directly to literals
> resolves to fixed-length arrays. Implicit List→Array conversion is forbidden.
>
> Array literal semantics:
>
> - The number of elements must equal N, otherwise compile-time E1002; an empty literal with
>   non-zero N is also rejected
> - Each element's type must be compatible with T, otherwise compile-time E1002
> - N's grammar form: only integer literals (may be negative) or constant names; compound
>   expressions (e.g., `2+1`) are rejected at parse time
> - When N is a symbolic constant (function const parameter, e.g., `Array(Int, n)`), the count check
>   is deferred to the refined type phase
> - v1 rejects nested array literals (e.g., `Array(Array(Int,2),2) = [[1,2],[3,4]]`) at compile
>   time, requiring explicit construction layer by layer; recursive resolution is reserved for
>   future versions

#### 1.6.5 List Comprehension

```
ListComp    ::= '[' Expr 'for' Identifier 'in' Expr (',' Expr)* ('if' Expr)? ']'
```

> **Behavior tightening (migration note)**: The iteration variable grammar was always
> `'for' Identifier 'in'`, but the old implementation's pattern went through full pratt
> parsing—after `'in'` was registered as an infix operator, `x` would consume `in items` as a
> membership expression. After the fix, non-identifier patterns fail to parse directly, no longer
> falling back to `_` (silently swallowing errors) like the old implementation. Impact: previously
> parseable forms like `[x for (a, b) in pairs]` now produce errors—this form never had defined
> behavior (the variable was always `_`), and the tightening direction is correct with no semantic
> migration cost.

#### 1.6.6 Membership Test

```
Membership  ::= Expr 'in' Expr
```

> `in` is a binary relational operator that returns `Bool`—`true` for hit, `false` for miss, no
> error. Semantic split: `[]` asserts existence and retrieves the value (errors on failure), `in`
> asks whether something exists (a miss is a normal `false`). Right operand coverage: List / Array /
> Dict (key set) / Tuple / String (substring) / Range (interval). `in` is a first-class Hoare
> predicate, serving as the base of compile-time provable propositions in the refined type phase.
> (Set is removed from the right operand list—Set has no runtime representation, see §1.6.4)

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

> **Unary prefix operators** (`!` `-` `+`) bind tightly: only below call and member access, above
> all binary operators. Therefore `!a == b` ≡ `(!a) == b` (Zig-style semantics); `!` is a pure unary
> operation, not participating in short-circuit control flow, and is orthogonal to the `and`/`or`
> keywords (short-circuit) (RFC-010 authoritative definition).

> **Range binding strength**: `..` has binding strength (6, 7)—left 6 is below addition (5), right 7
> swallows addition but not same-level `..`. Before/after change comparison:
>
> | Expression   | Before (level 1, right-associative)                                                          | After ((6,7), left-associative)                                                |
> | ------------ | -------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------ |
> | `x in 1..10` | `x in 1..10` (`in` right operand level 4, `..` level 1 cannot swallow, actually unparseable) | `x in (1..10)`—the range as a whole serves as `in`'s right operand             |
> | `0..n+2`     | `(0..n)+2` (right-associative trap: upper bound consumed, `for` loop directly E3004)         | `0..(n+2)`—upper bound is an arithmetic expression                             |
> | `a == b..c`  | `a == (b..c)` (`..` level 1 < `==` level 3, naturally whole)                                 | `a == (b..c)`—**semantics unchanged**, `..` still higher than comparison level |
> | `1..2*3`     | `(1..2)*3`                                                                                   | `1..(2*3)`—upper bound is an arithmetic expression                             |
> | `a..b..c`    | `a..(b..c)` (right-associative chaining, meaningless nested Range)                           | `(a..b)..c`—**step form** (`c` is the step)                                    |
>
> Net effect: compound upper bound `for i in 0..n+2` changes from "parses successfully but E3004" to
> "directly usable"; `x in 1..10` changes from "unparseable" to "interval check"; `a..b..c` changes
> from "meaningless nesting" to "step component". Level 6 falls between `+` (level 5) and `<<`
> (level 7), mathematical convention: the range is a tightly bound construct, the upper bound
> naturally being a complete arithmetic expression.

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

> **Statement termination rules**: The separation and line-break behavior between Stmts (`;`
> explicit separator, line-break termination, continuation exceptions, line-leading `(`/`[` never
> merged) are defined by [RFC-038](../design/rfc/draft/038-statement-termination.md).

**Unified semantics**: All `{}` blocks have consistent return semantics:

| Block Type  | Return Semantics        | Default Return |
| ----------- | ----------------------- | -------------- |
| Plain `{}`  | Returns value           | Void           |
| `unsafe {}` | Returns type definition | Void           |
| `spawn {}`  | Returns result          | Void           |

**Core principles**:

- `return` in `{}` always returns its content to the enclosing scope
- Without `return`, the block returns `Void` by default
- Expression form `= expr` returns the value directly

```yaoxiang
// Plain {} block: return returns a value
result = {
    x = compute()
    return x  // Returns the value to the enclosing scope
}

// unsafe {} block: return returns a type definition
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void
    }
    return SqliteDb  // Returns the type definition to the enclosing scope
}

// spawn {} block: return returns the result
(a, b) = spawn {
    result1 = fetch("url1"),
    result2 = fetch("url2")
    return (result1, result2)  // Returns the result to the enclosing scope
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

The `?` operator is a postfix operator with the same precedence as `.`. For `Result(T, E)` types:

- On `Ok(v)`, extract the value `v` and continue execution
- On `Err(e)`, propagate the error upward (`return Err(e)`)

```yaoxiang
process: (data: Data) -> Result(Data, Error) = {
    validated = validate(data)?     // On success, extract value; on failure, propagate upward
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

> **step semantics**: In `a..b..c`, `c` is the step. Literal `c = 0` is rejected at compile time;
> dynamic `c` has no runtime zero check (E6001 family; will be elevated to Result once the error
> system lands). `c < 0` is legal, the range direction reverses with the sign (`10..0..(-2)`
> decreasing).

### 2.13 ref Expression

```
RefExpr     ::= 'ref' Expr
```

`ref` creates shared ownership. The compiler automatically chooses Rc (single-task) or Arc
(cross-task); users do not need to worry about implementation details.

```yaoxiang
data = ref heavy_data
spawn { use(data) }   // Cross-task: compiler automatically chooses Arc
```

### 2.14 unsafe Expression

```
UnsafeExpr  ::= 'unsafe' Block
```

The `unsafe` block is used to define opaque types and operate on raw pointers. Use `return` to
return a type definition to the enclosing scope.

**Semantics**:

- Types can be defined and raw pointers operated on within `unsafe {}`
- The returned type is usable outside `unsafe {}`
- Field access on the type requires unsafe permission

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

- `x = value`: Search outward along the scope chain for `x`; if found, assign; if not, declare a new
  one
- `mut x = value`: Explicit new mutable declaration, forbidding same name as outer scope
- Any name can only be declared once within the same scope

> **Detailed definition**: Complete rules for scope, variable declaration, and shadowing mechanism
> are detailed in the [Module System Specification](./modules.md#chapter-4-scope).

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

**Semantics**: `return` is used to return a value from a code block. Without `return`, the code
block returns `Void` by default.

### 3.4 break Statement

```
BreakStmt   ::= 'break'
```

**Semantics**: Immediately terminates the innermost `while`/`for` loop, transferring control to
after the loop body.

- **Only exits the nearest level**: `break` always acts on the innermost loop containing it. When a
  nested loop requires exiting multiple levels at once, extract the inner loop into a function and
  use `return`, or use a flag (break/continue carry no label; if loop labels are introduced in the
  future, they will follow the loop declaration-side syntax per the RFC process, decided together
  with the multi-exit design of the proof pipeline).
- **Restricted to loop bodies**: `break` can only appear inside `while`/`for` loop bodies (including
  nested blocks/if/match within the body). Appearing outside a loop is a compile error (E1102
  `'break' outside of a loop`).
- **Does not affect termination proof**: `while` loops must still be provably terminating (decreases
  metric); `break` does not participate in termination arguments; `while true { break }` will not be
  accepted.
- **Borrow semantics**: The control-flow edge of break participates in the structural cut of
  RFC-009a reverse BFS liveness analysis (the exited iteration does not participate in back-edge
  liveness derivation).

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

**Semantics**: Skips the remaining statements in the current iteration, going directly to the next
iteration of the innermost loop— for `while`, re-evaluates the condition; for `for`, takes the next
element.

- **Only acts on the nearest level**: Same as `break`, no label
- **Restricted to loop bodies**: Appearing outside a loop is a compile error (E1102)

```yaoxiang
mut sum = 0
mut n = 0
while n < 5 {
    n = n + 1
    if n == 3 {
        continue           // Skip the following accumulation, n == 3 not counted
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
next iteration is a completely new binding, with no relation to the previous iteration's binding.

#### 3.9.2 The Difference Between for and for mut

| Syntax              | Loop Variable Mutability | Description                                |
| ------------------- | ------------------------ | ------------------------------------------ |
| `for i in 1..5`     | Immutable                | Cannot modify the binding in the loop body |
| `for mut i in 1..5` | Mutable                  | Can modify the binding in the loop body    |

```yaoxiang
// Legal: each iteration binds a new value, no modification needed
for i in 1..5 {
    print(i)  // Read the value of i
}

// Error: immutable binding, cannot modify
for i in 1..5 {
    i = i + 1  // Error: cannot modify an immutable binding
}

// Legal: use for mut to allow modifying the binding
for mut i in 1..5 {
    i = i + 1  // Modification allowed
}
```

#### 3.9.3 Shadowing Check

YaoXiang forbids variable shadowing. A for loop variable cannot share a name with a variable in an
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

| Language | For Loop Variable Semantics                               |
| -------- | --------------------------------------------------------- |
| YaoXiang | Each iteration binds a new value                          |
| Rust     | Modifies the same variable (needs `mut`)                  |
| Python   | Modifies the same variable (no `mut` needed)              |
| C/C++    | Modifies the same variable (needs pointers or references) |

**Design rationale**: YaoXiang adopts binding semantics because:

1. **More aligned with natural semantics** In natural language, "for each element x in the
   collection" means each x is an independent individual. YaoXiang's `for i in 1..5` reads as "for
   each i from 1 to 5"; the i in each iteration is a completely new binding, which is consistent
   with human intuition.

2. **Avoid accidental modification** Default immutable binding semantics means the loop variable
   cannot be accidentally modified inside the loop body. No need to worry about hard-to-track bugs
   caused by accidentally writing `i = ...` somewhere in a complex loop body.

3. **High-performance solutions at your fingertips** When you genuinely need to reuse a variable
   across iterations (e.g., accumulators, caches), use `for mut` to switch to mutable binding mode.
   This is clearer than implicit shared state—the intent is expressed explicitly through syntax, not
   hidden in runtime behavior.

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

- Block body references outer variables = **Move value capture**: the value is snapshotted into the
  closure environment at the spawn creation point, and the block body reads it via env (LoadUpvalue)
- **Primitives** (Int/Float/Bool/Char) are value-copied, outer variables are unaffected
- **Handle types** (Struct/String/List, etc.) snapshot = handle copy, sharing the underlying object;
  in the Embedded runtime (default) on the same thread and same heap, the handle is valid
- Sharing across multiple tasks requires explicit `ref` (§2.13, compiler automatically chooses
  Rc/Arc)
- Outer variables referenced by `return` inside the block are captured in the same way

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
