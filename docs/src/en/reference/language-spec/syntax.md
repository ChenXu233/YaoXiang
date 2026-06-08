# Syntax Specification

This document defines the syntax specification of the YaoXiang programming language, including lexical structure, grammar rules, and operator precedence.

---

## Chapter 1: Lexical Structure

### 1.1 Source Files

YaoXiang source files must use UTF-8 encoding. Source files typically use the `.yx` extension.

### 1.2 Token Classification

| Category | Description | Examples |
|----------|-------------|----------|
| Identifiers | Start with a letter or underscore | `x`, `_private`, `my_var` |
| Keywords | Language-defined reserved words | `Type`, `pub`, `use` |
| Literals | Fixed values | `42`, `"hello"`, `true` |
| Operators | Arithmetic symbols | `+`, `-`, `*`, `/` |
| Delimiters | Syntax separators | `(`, `)`, `{`, `}`, `,` |

### 1.3 Keywords

YaoXiang defines a very small set of keywords:

```
pub    use    spawn
ref    mut    if     elif
else   match  while  for    return
break  continue as     in     unsafe
```

These keywords have special meaning in any context and cannot be used as identifiers.

### 1.4 Reserved Words

| Reserved Word | Type | Description |
|---------------|------|-------------|
| `Type` | Type | Meta type |
| `true` | Bool | Boolean true |
| `false` | Bool | Boolean false |
| `void` | Void | Empty value |
| `some(T)` | Option | Option value variant |
| `ok(T)` | Result | Result success variant |
| `err(E)` | Result | Result error variant |

### 1.5 Identifiers

Identifiers start with a letter or underscore, followed by letters, digits, or underscores. Identifiers are case-sensitive.

Special identifiers:
- `_` is used as a placeholder to ignore a value
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
// Single-line comment

/* Multi-line comment
   Can span multiple lines */
```

### 1.8 Indentation Rules

Code must use 4 spaces for indentation. Tab characters are prohibited. This is a mandatory syntactic rule.

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

| Precedence | Operators | Associativity |
|------------|-----------|---------------|
| 1 | `()` `[]` `.` `?` | Left to right |
| 2 | `as` | Left to right |
| 3 | `*` `/` `%` | Left to right |
| 4 | `+` `-` | Left to right |
| 5 | `..` | Left to right |
| 6 | `<<` `>>` | Left to right |
| 7 | `&` `\|` `^` | Left to right |
| 8 | `==` `!=` `<` `>` `<=` `>=` | Left to right |
| 9 | `not` | Right to left |
| 10 | `and` `or` | Left to right |
| 11 | `if...else` | Right to left |
| 12 | `=` `+=` `-=` `*=` `/=` | Right to left |

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

### 2.7 Conditional Expressions

```
IfExpr      ::= 'if' Expr Block ('elif' Expr Block)* ('else' Block)?
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

**Unified Semantics**: All `{}` blocks have consistent return semantics:

| Block Type | Return Semantics | Default Return |
|------------|------------------|----------------|
| Regular `{}` | Returns value | Void |
| `unsafe {}` | Returns type definition | Void |
| `spawn {}` | Returns result | Void |

**Core Principles**:
- `return` in `{}` always returns content to the outer scope
- Without `return`, the default is `Void`
- Expression form `= expr` returns value directly

```yaoxiang
# Regular {} block: return returns value
result = {
    x = compute()
    return x  # Returns value to outer scope
}

# unsafe {} block: return returns type definition
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void
    }
    return SqliteDb  # Returns type definition to outer scope
}

# spawn {} block: return returns result
(a, b) = spawn {
    result1 = fetch("url1"),
    result2 = fetch("url2")
    return (result1, result2)  # Returns result to outer scope
}
```

### 2.10 Lambda Expressions

```
Lambda      ::= '(' ParamList? ')' '=>' Expr
            |  '(' ParamList? ')' '=>' Block
```

### 2.11 Error Propagation Operator

```
ErrorPropagate ::= Expr '?'
```

The `?` operator is a suffix operator with the same precedence as `.`. For `Result(T, E)` type:
- `Ok(v)` extracts value `v` and continues execution
- `Err(e)` propagates the error upward (`return Err(e)`)

```yaoxiang
process: (data: Data) -> Result(Data, Error) = {
    validated = validate(data)?     // Extract value on success, propagate on failure
    transform(validated)
}
```

### 2.12 Range Expressions

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

`ref` creates a shared reference. The compiler automatically selects Rc (single-task) or Arc (cross-task), and users don't need to care about implementation details.

```yaoxiang
data = ref heavy_data
spawn { use(data) }   // Cross-task: compiler automatically selects Arc
```

### 2.14 unsafe Expression

```
UnsafeExpr  ::= 'unsafe' Block
```

`unsafe` blocks are used to define opaque types and operate on raw pointers. Use `return` to return the type definition to the outer scope.

**Semantics**:
- Types can be defined and raw pointers can be operated on inside `unsafe {}`
- Returned types are usable outside `unsafe {}`
- Accessing fields of the type requires unsafe permission

```yaoxiang
# Define opaque type in unsafe block
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  # Raw pointer
    }
    return SqliteDb
}

# SqliteDb is usable outside the unsafe block
db = sqlite3_open("test.db")
```

### 2.15 Scopes

**Basic Rules**:
- Each `{}` block creates a scope
- Inner scopes can access variables from outer scopes
- Outer scopes cannot access variables from inner scopes
- Variable declarations follow "assignment preference" principle

```yaoxiang
# Block scope
{
    x = 10
    # x is visible in this scope
}
# x is not visible in this scope

# Function scope
add: (a: Int, b: Int) -> Int = {
    result = a + b
    return result
}
# result is not visible outside the function
```

**Variable Declaration and Shadowing**:
- `x = value`: Search outward along the scope chain for x; if found, assign; if not found, declare new
- `mut x = value`: Explicitly new mutable declaration, prohibits same name as outer layer
- Any name can only be declared once within the same scope

> **Detailed Definition**: Complete rules for scopes, variable declarations, and shadowing mechanisms are detailed in [Module System Specification](./modules.md#chapter-4-scopes).

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

**Semantics**: `return` is used to return a value from a code block. If no `return` is present, the code block defaults to returning `Void`.

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
IfStmt      ::= 'if' Expr Block ('elif' Expr Block)* ('else' Block)?
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

The YaoXiang for loop semantics differ from traditional languages: **each iteration binds a new value, rather than modifying the same variable**.

```yaoxiang
// Example: for i in 1..5
for i in 1..5 {
    print(i)
}
```

**Execution Process**:

| Iteration | Behavior of Loop Variable |
|-----------|----------------------------|
| First | Create new binding `i = 1`, execute loop body, print 1 |
| Second | Create new binding `i = 2` (previous binding destroyed), execute loop body, print 2 |
| Third | Create new binding `i = 3`, execute loop body, print 3 |
| Fourth | Create new binding `i = 4`, execute loop body, print 4 |
| End | Loop body ends, binding destroyed |

**Key Point**: After each iteration ends, the binding created in that iteration is destroyed. The next iteration is a brand new binding with no relation to the previous iteration's binding.

#### 3.9.2 Difference Between for and for mut

| Syntax | Loop Variable Mutability | Description |
|--------|--------------------------|-------------|
| `for i in 1..5` | Immutable | Cannot modify binding in loop body |
| `for mut i in 1..5` | Mutable | Can modify binding in loop body |

```yaoxiang
// Valid: Each iteration binds a new value, no need to modify
for i in 1..5 {
    print(i)  # Read value of i
}

// Error: Immutable binding, cannot modify
for i in 1..5 {
    i = i + 1  # Error: cannot modify immutable binding
}

// Valid: Use for mut to allow modification
for mut i in 1..5 {
    i = i + 1  # Allowed
}
```

#### 3.9.3 Shadowing Check

YaoXiang prohibits variable shadowing. The for loop variable cannot have the same name as a variable in the outer scope:

```yaoxiang
// Error: i is already declared outside
i = 10
for i in 1..5 {
    print(i)
}

// Correct: Use a different variable name
i = 10
for j in 1..5 {
    print(j)
}
```

This rule applies to all code blocks. See [4.3 Shadowing Rules](./modules.md#43-shadowing-rules) for details.

#### 3.9.4 Comparison with Other Languages

| Language | for Loop Variable Semantics |
|----------|------------------------------|
| YaoXiang | Each iteration binds a new value |
| Rust | Modifies the same variable (requires mut) |
| Python | Modifies the same variable (no mut needed) |
| C/C++ | Modifies the same variable (requires pointer or reference) |

**Design Rationale**: YaoXiang uses binding semantics because:

1. **More aligned with natural semantics**
   In natural language, "for each element x in the collection" means each x is an independent individual. YaoXiang's `for i in 1..5` reads as "for each i in 1 to 5", and each iteration's i is a brand new binding. This is consistent with human intuitive understanding.

2. **Avoids accidental modifications**
   The default immutable binding semantics mean the loop body cannot accidentally modify the loop variable. You don't need to worry about accidentally writing `i = ...` somewhere in a complex loop body causing hard-to-track bugs.

3. **High-performance solutions are readily available**
   When you genuinely need to reuse a variable across iterations (e.g., accumulators, caches), you can simply use `for mut` to switch to mutable binding mode. This is clearer than implicit shared state—the intent is explicitly expressed through syntax rather than hidden in runtime behavior.

### 3.10 spawn Statement

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' 'mut'? Identifier 'in' Expr '{' Expr '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
```

**spawn block**: Explicitly declares a concurrency boundary, expressions inside the block execute concurrently.

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

---

## Appendix: Syntax Quick Reference

### A.1 Control Flow

```
if Expr Block (elif Expr Block)* (else Block)?
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