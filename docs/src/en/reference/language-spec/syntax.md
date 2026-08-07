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
| Keyword    | Language-defined reserved words    | `Type`, `pub`, `use`      |
| Literal    | Fixed values                       | `42`, `"hello"`, `true`   |
| Operator   | Operator symbols                   | `+`, `-`, `*`, `/`        |
| Delimiter  | Syntax delimiters                  | `(`, `)`, `{`, `}`, `,`   |

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

YaoXiang's "reserved words" are organized in three layers, recognized by the parser and type checker
at different stages:

#### 1.4.1 Literal Reserved Words

The parser has independent tokens for these literal identifiers, which cannot be used as ordinary
identifiers:

| Identifier | Owning Type | Description                                                                                                   |
| ---------- | ----------- | ------------------------------------------------------------------------------------------------------------- |
| `Type`     | —           | Meta-type keyword                                                                                             |
| `true`     | Bool        | Boolean true                                                                                                  |
| `false`    | Bool        | Boolean false                                                                                                 |
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
without imports. The parser treats them as ordinary identifiers—**they are not reserved words and
can be shadowed by local bindings (not recommended)**.

| Type Name | Logical Correspondence | Description                                                                                                                                         |
| --------- | ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Void`    | ⊤ (True/Unit)          | A product type with zero fields, with exactly one inhabitant (the `void` literal, see §1.4.1)                                                       |
| `Never`   | ⊥ (False/Empty type)   | A type with zero variants and zero inhabitants. No expression can produce a `Never` value. `Never <: T` holds for all `T` (principle of explosion). |
| `Int`     | —                      | Signed integer                                                                                                                                      |
| `Float`   | —                      | Floating-point number                                                                                                                               |
| `Bool`    | —                      | Boolean value: `true` / `false`                                                                                                                     |
| `Char`    | —                      | Unicode character                                                                                                                                   |
| `String`  | —                      | String                                                                                                                                              |

### 1.5 Identifiers

Identifiers start with a letter or underscore; subsequent characters may be letters, digits, or
underscores. Identifiers are case-sensitive.

Special identifiers:

- `_` is used as a placeholder to indicate that a value is ignored
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
Set         ::= '{' Expr (',' Expr)* '}'
```

#### 1.6.5 List Comprehensions

```
ListComp    ::= '[' Expr 'for' Identifier 'in' Expr (',' Expr)* ('if' Expr)? ']'
```

#### 1.6.6 Membership Test

```
Membership  ::= Expr 'in' Expr
```

### 1.7 Comments

```
// single-line comment

/* multi-line comment
   can span multiple lines */
```

### 1.8 Indentation Rules

Code must use 4-space indentation; Tab characters are prohibited. This is a mandatory syntactic
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

> **Unary prefix operators** (`!` `-` `+`) are tightly bound: they rank only below call and member
> access, and above all binary operators. Therefore `!a == b` ≡ `(!a) == b` (Zig-style semantics);
> `!` is a pure unary operation that does not participate in short-circuit control flow, and is
> orthogonal to the `and`/`or` keywords (short-circuit) (authoritative definition in RFC-010).

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

> **Statement Termination Rules**: The separation and newline behavior between Stmts (explicit `;`
> separators, newline termination, line continuation exceptions, line-leading `(`/`[` never merging)
> is defined by [RFC-038](../design/rfc/draft/038-statement-termination.md).

**Unified Semantics**: All `{}` blocks share consistent return semantics:

| Block Type    | return semantics          | Default Return |
| ------------- | ------------------------- | -------------- |
| Ordinary `{}` | Returns a value           | Void           |
| `unsafe {}`   | Returns a type definition | Void           |
| `spawn {}`    | Returns a result          | Void           |

**Core Principles**:

- `return` inside `{}` always returns content to the enclosing scope
- Without an explicit `return`, the default return is `Void`
- The expression form `= expr` directly returns a value

```yaoxiang
// Ordinary {} block: return returns a value
result = {
    x = compute()
    return x  // returns a value to the enclosing scope
}

// unsafe {} block: return returns a type definition
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void
    }
    return SqliteDb  // returns the type definition to the enclosing scope
}

// spawn {} block: return returns a result
(a, b) = spawn {
    result1 = fetch("url1"),
    result2 = fetch("url2")
    return (result1, result2)  // returns the result to the enclosing scope
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

The `?` operator is a postfix operator with the same precedence as `.`. For the `Result(T, E)` type:

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
RangeExpr   ::= Expr '..' Expr
```

`..` creates a range type, used in `for` loops and slicing.

```yaoxiang
for i in 0..10 { print(i) }
slice = array[0..5]
```

### 2.13 ref Expression

```
RefExpr     ::= 'ref' Expr
```

`ref` creates a shared holding. The compiler automatically chooses Rc (single task) or Arc
(cross-task); users do not need to care about implementation details.

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
- Types returned from `unsafe {}` are usable outside the block
- Field access on such types requires unsafe permissions

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

**Basic Rules**:

- Every `{}` block creates a scope
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

**Variable Declaration and Shadowing**:

- `x = value`: walks the scope chain outward looking for x; if found, assigns; if not found,
  declares a new variable
- `mut x = value`: explicit new mutable declaration, prohibited from sharing a name with an outer
  variable
- Within the same scope, any name can only be declared once

> **Detailed Definition**: The complete rules for scope, variable declaration, and shadowing
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

**Semantics**: `return` is used to return a value from a code block. Without `return`, the code
block defaults to returning `Void`.

### 3.4 break Statement

```
BreakStmt   ::= 'break' Identifier?
```

### 3.5 continue Statement

```
ContinueStmt::= 'continue'
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

The semantics of the YaoXiang `for` loop differ from traditional languages: **each iteration binds a
new value, rather than modifying the same variable**.

```yaoxiang
// Example: for i in 1..5
for i in 1..5 {
    print(i)
}
```

**Execution Process**:

| Iteration | Behavior of the Loop Variable                                                       |
| --------- | ----------------------------------------------------------------------------------- |
| 1st       | Creates a new binding `i = 1`, loop body executes, prints 1                         |
| 2nd       | Creates a new binding `i = 2` (previous binding destroyed), body executes, prints 2 |
| 3rd       | Creates a new binding `i = 3`, loop body executes, prints 3                         |
| 4th       | Creates a new binding `i = 4`, loop body executes, prints 4                         |
| End       | Loop body ends, bindings are destroyed                                              |

**Key Point**: After each iteration, the binding created in that iteration is destroyed. The next
iteration is a brand-new binding, with no relation to the binding of the previous iteration.

#### 3.9.2 Difference Between `for` and `for mut`

| Syntax              | Loop Variable Mutability | Description                                        |
| ------------------- | ------------------------ | -------------------------------------------------- |
| `for i in 1..5`     | Immutable                | Cannot modify the binding inside the loop body     |
| `for mut i in 1..5` | Mutable                  | Allowed to modify the binding inside the loop body |

```yaoxiang
// Valid: bind a new value each iteration, no modification needed
for i in 1..5 {
    print(i)  // read i's value
}

// Error: immutable binding, cannot be modified
for i in 1..5 {
    i = i + 1  // error: cannot modify an immutable binding
}

// Valid: use for mut to allow modifying the binding
for mut i in 1..5 {
    i = i + 1  // modification allowed
}
```

#### 3.9.3 Shadowing Check

YaoXiang prohibits variable shadowing. A `for` loop variable cannot share a name with a variable
from an outer scope:

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

| Language | for Loop Variable Semantics                                |
| -------- | ---------------------------------------------------------- |
| YaoXiang | Binds a new value each iteration                           |
| Rust     | Modifies the same variable (requires `mut`)                |
| Python   | Modifies the same variable (no `mut` needed)               |
| C/C++    | Modifies the same variable (requires pointer or reference) |

**Design Rationale**: YaoXiang adopts binding semantics because:

1. **More aligned with natural semantics** In natural language, "for every element x in a
   collection" implies each x is an independent individual. YaoXiang's `for i in 1..5` reads as "for
   every i from 1 to 5", where i in each iteration is a brand-new binding, consistent with human
   intuition.

2. **Avoiding accidental modification** The default immutable binding semantics mean that the loop
   variable cannot be accidentally modified inside the loop body. No need to worry about a
   hard-to-trace bug caused by writing `i = ...` somewhere in a complex loop body.

3. **High-performance solutions within reach** When it is genuinely necessary to reuse a variable
   across iterations (e.g., accumulators, caches), use `for mut` to switch to mutable binding mode.
   This is clearer than implicit shared state—intent is expressed explicitly through syntax, not
   hidden in runtime behavior.

### 3.10 spawn Statement

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' 'mut'? Identifier 'in' Expr '{' Expr '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
```

**spawn block**: explicitly declares a concurrent region; expressions within the block execute
concurrently.

```yaoxiang
(result_a, result_b) = spawn {
    parse(fetch("url1")),
    parse(fetch("url2"))
}
```

**spawn loop**: a data-parallel loop.

```yaoxiang
results = spawn for item in items {
    process(item)
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
