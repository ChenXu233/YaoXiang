# Syntax Specification

This document defines the syntax specification for the YaoXiang programming language, including lexical structure, syntax rules, and operator precedence.

---

## Chapter 1: Lexical Structure

### 1.1 Source Files

YaoXiang source files must use UTF-8 encoding. Source files typically use the `.yx` extension.

### 1.2 Token Classification

| Category   | Description             | Examples                   |
| ---------- | ----------------------- | -------------------------- |
| Identifier | Starts with letter or `_` | `x`, `_private`, `my_var` |
| Keyword    | Language-reserved words | `Type`, `pub`, `use`       |
| Literal    | Fixed values            | `42`, `"hello"`, `true`    |
| Operator   | Operation symbols       | `+`, `-`, `*`, `/`         |
| Delimiter  | Syntax separators       | `(`, `)`, `{`, `}`, `,`    |

### 1.3 Keywords

YaoXiang defines very few keywords:

```
pub    use    spawn
ref    mut    if     else
else   match  while  for    return
break  continue as     in     unsafe
```

These keywords have special meaning in any context and cannot be used as identifiers.

### 1.4 Reserved Words

YaoXiang's "reserved words" are divided into three layers, recognized by the parser and type checker at different stages:

#### 1.4.1 Literal Reserved Words

Literal identifiers that the parser has separate tokens for and cannot be used as regular identifiers:

| Identifier | Belongs To | Description                                                                           |
| ---------- | ---------- | ------------------------------------------------------------------------------------- |
| `Type`     | —          | Meta type keyword                                                                     |
| `true`     | Bool       | Boolean truth value                                                                   |
| `false`    | Bool       | Boolean false value                                                                   |
| `void`     | Void       | Void literal (Unit value). Lowercase `void` is a value literal; uppercase `Void` is a type name (see §1.4.3). |

#### 1.4.2 Constructor Expressions

The following constructors are recognized by the parser in pattern matching and expression contexts:

| Constructor | Belongs To | Description           |
| ----------- | ---------- | --------------------- |
| `some(T)`   | Option     | Option value variant construction |
| `ok(T)`     | Result     | Result success variant |
| `err(E)`    | Result     | Result error variant   |

#### 1.4.3 Builtin Type Names

The following type names are pre-registered by the type checker and can be used in type positions without importing. The parser treats them as regular identifiers—**not reserved words, and can be shadowed by local bindings (not recommended)**.

| Type Name | Logical Correspondence | Description                                                                                                      |
| --------- | ---------------------- | ---------------------------------------------------------------------------------------------------------------- |
| `Void`    | ⊤ (True/Unit)          | Zero-field product type with exactly one inhabitant (`void` literal, see §1.4.1)                                 |
| `Never`   | ⊥ (False/Empty type)  | Zero-variant enum type with zero inhabitants. No expression can produce a `Never` value. `Never <: T` holds for all `T` (ex falso quodlibet). |
| `Int`     | —                      | Signed integer                                                                           |
| `Float`   | —                      | Floating-point number                                                                       |
| `Bool`    | —                      | Boolean value: `true` / `false`                                                                   |
| `Char`    | —                      | Unicode character                                                                           |
| `String`  | —                      | String                                                                                       |

### 1.5 Identifiers

Identifiers start with a letter or underscore, and subsequent characters can be letters, digits, or underscores. Identifiers are case-sensitive.

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

#### 1.6.2 Floating-point Numbers

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
// Single-line comment

/* Multi-line comment
   Can span multiple lines */
```

### 1.8 Indentation Rules

Code must use 4 spaces for indentation; Tab characters are prohibited. This is a mandatory syntax rule.

---

## Chapter 2: Syntax Rules

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
| 3          | `*` `/` `%`                 | Left to right |
| 4          | `+` `-`                     | Left to right |
| 5          | `..`                        | Left to right |
| 6          | `<<` `>>`                   | Left to right |
| 7          | `&` `\|` `^`                | Left to right |
| 8          | `==` `!=` `<` `>` `<=` `>=` | Left to right |
| 9          | `not`                       | Right to left |
| 10         | `and` `or`                  | Left to right |
| 11         | `if...else`                 | Right to left |
| 12         | `=` `+=` `-=` `*=` `/=`     | Right to left |

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

**Unified semantics**: All `{}` blocks have consistent return semantics:

| Block Type  | return semantics     | Default return |
| ----------- | -------------------- | -------------- |
| Plain `{}`  | Returns value        | Void           |
| `unsafe {}` | Returns type definition | Void        |
| `spawn {}`  | Returns result       | Void           |

**Core principles**:

- `return` in `{}` always returns the content to the outer scope
- By default, no `return` returns `Void`
- Expression form `= expr` returns the value directly

```yaoxiang
// Plain {} block: return returns value
result = {
    x = compute()
    return x  // Returns value to outer scope
}

// unsafe {} block: return returns type definition
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void
    }
    return SqliteDb  // Returns type definition to outer scope
}

// spawn {} block: return returns result
(a, b) = spawn {
    result1 = fetch("url1"),
    result2 = fetch("url2")
    return (result1, result2)  // Returns result to outer scope
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

The `?` operator is a postfix operator with the same precedence as `.`. For `Result(T, E)` type:

- `Ok(v)` extracts value `v` and continues execution
- `Err(e)` propagates the error upward (`return Err(e)`)

```yaoxiang
process: (data: Data) -> Result(Data, Error) = {
    validated = validate(data)?     // Extract value on success, propagate on failure
    transform(validated)
}
```

### 2.12 Range Expression

```
RangeExpr   ::= Expr '..' Expr
```

`..` creates a range type, used for `for` loops and slicing.

```yaoxiang
for i in 0..10 { print(i) }
slice = array[0..5]
```

### 2.13 ref Expression

```
RefExpr     ::= 'ref' Expr
```

`ref` creates shared ownership. The compiler automatically chooses Rc (single-task) or Arc (cross-task), and users don't need to care about implementation details.

```yaoxiang
data = ref heavy_data
spawn { use(data) }   // Cross-task: compiler automatically chooses Arc
```

### 2.14 unsafe Expression

```
UnsafeExpr  ::= 'unsafe' Block
```

`unsafe` blocks are used to define opaque types and operate on raw pointers. Use `return` to return the type definition to the outer scope.

**Semantics**:

- Types can be defined and raw pointers can be operated on inside `unsafe {}`
- The returned type is available outside `unsafe {}`
- Accessing fields of the type requires unsafe permission

```yaoxiang
// Define opaque type inside unsafe block
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  // Raw pointer
    }
    return SqliteDb
}

// SqliteDb is available outside unsafe block
db = sqlite3_open("test.db")
```

### 2.15 Scopes

**Basic rules**:

- Each `{}` block creates a scope
- Inner scopes can access variables from outer scopes
- Outer scopes cannot access variables from inner scopes
- Variable declarations follow the "assignment preference" principle

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

- `x = value`: Search outward along the scope chain for x; assign if found, otherwise declare new
- `mut x = value`: Explicitly declare new mutable binding; prohibits same name in outer scope
- Any name can only be declared once in the same scope

> **Detailed definition**: Complete rules for scopes, variable declarations, and shadowing mechanisms are detailed in [Module System Specification](./modules.md#chapter-4-scopes).

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

**Semantics**: `return` is used to return a value from a code block. If there is no `return`, the code block returns `Void` by default.

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

#### 3.9.1 Semantics: Each iteration binds a new value

The `for` loop semantics in YaoXiang differ from traditional languages: **each iteration binds a new value, rather than modifying the same variable**.

```yaoxiang
// Example: for i in 1..5
for i in 1..5 {
    print(i)
}
```

**Execution process**:

| Iteration | Behavior of loop variable                                  |
| --------- | ---------------------------------------------------------- |
| 1st       | Create new binding `i = 1`, execute loop body, print 1     |
| 2nd       | Create new binding `i = 2` (previous binding destroyed), execute loop body, print 2 |
| 3rd       | Create new binding `i = 3`, execute loop body, print 3     |
| 4th       | Create new binding `i = 4`, execute loop body, print 4     |
| End       | Loop body ends, binding destroyed                         |

**Key point**: After each iteration ends, the binding created in that iteration is destroyed. The next iteration is a brand new binding with no relationship to the previous iteration's binding.

#### 3.9.2 Difference between for and for mut

| Syntax               | Loop variable mutability | Description                         |
| -------------------- | ------------------------ | ----------------------------------- |
| `for i in 1..5`      | Immutable                | Binding cannot be modified in loop body |
| `for mut i in 1..5`  | Mutable                  | Binding can be modified in loop body   |

```yaoxiang
// Valid: each iteration binds new value, no need to modify
for i in 1..5 {
    print(i)  // Read value of i
}

// Error: immutable binding, cannot modify
for i in 1..5 {
    i = i + 1  // Error: cannot modify immutable binding
}

// Valid: using for mut allows modification
for mut i in 1..5 {
    i = i + 1  // Allowed
}
```

#### 3.9.3 Shadowing Check

YaoXiang prohibits variable shadowing. for loop variables cannot have the same name as variables in outer scopes:

```yaoxiang
// Error: i already declared externally
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

| Language  | for loop variable semantics               |
| --------- | ---------------------------------------- |
| YaoXiang  | Each iteration binds a new value         |
| Rust      | Modifies the same variable (needs mut)   |
| Python    | Modifies the same variable (no mut)      |
| C/C++     | Modifies the same variable (needs pointer or reference) |

**Design rationale**: YaoXiang uses binding semantics because:

1. **More aligned with natural semantics** In natural language, "for each element x in a set" means each x is an independent individual. YaoXiang's
   `for i in 1..5`
   reads as "for each i in 1 to 5", and the i in each iteration is a brand new binding. This is consistent with human intuitive understanding.

2. **Avoid accidental modification**
   The default immutable binding semantics mean the loop variable cannot be accidentally modified inside the loop body. You don't need to worry about accidentally writing `i = ...` somewhere in a complex loop body causing hard-to-track bugs.

3. **High-performance solutions readily available** When you really need to reuse a variable between iterations (e.g., accumulator, cache), you can switch to mutable binding mode with `for mut`. This is clearer than implicit shared state—the intent is expressed explicitly through syntax rather than hidden in runtime behavior.

### 3.10 spawn Statement

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' 'mut'? Identifier 'in' Expr '{' Expr '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
```

**spawn block**: Explicitly declares a concurrency boundary; expressions inside the block execute concurrently.

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
