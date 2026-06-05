# Syntax Specification

This document defines the syntax specification of the YaoXiang programming language, including lexical structure, grammar rules, and operator precedence.

---

## Chapter 1: Lexical Structure

### 1.1 Source Files

YaoXiang source files must use UTF-8 encoding. Source files typically use the `.yx` extension.

### 1.2 Token Categories

| Category | Description | Examples |
|----------|-------------|----------|
| Identifiers | Start with a letter or underscore | `x`, `_private`, `my_var` |
| Keywords | Language-predefined reserved words | `Type`, `pub`, `use` |
| Literals | Fixed values | `42`, `"hello"`, `true` |
| Operators | Operation symbols | `+`, `-`, `*`, `/` |
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
| `true` | Bool | Boolean true value |
| `false` | Bool | Boolean false value |
| `void` | Void | Empty value |
| `some(T)` | Option | Option value variant |
| `ok(T)` | Result | Result success variant |
| `err(E)` | Result | Result error variant |

### 1.5 Identifiers

Identifiers start with a letter or underscore, and subsequent characters can be letters, digits, or underscores. Identifiers are case-sensitive.

Special identifiers:
- `_` is used as a placeholder, indicating ignoring a value
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

Code must use 4 spaces for indentation; Tab characters are prohibited. This is a mandatory syntax rule.

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

**Semantics**: Code blocks must use `return` to return values; without `return`, the default return value is `Void`. The expression form `= expr` returns a value directly.

### 2.10 Lambda Expressions

```
Lambda      ::= '(' ParamList? ')' '=>' Expr
            |  '(' ParamList? ')' '=>' Block
```

### 2.11 Error Propagation Operator

```
ErrorPropagate ::= Expr '?'
```

The `?` operator is a postfix operator with the same precedence as `.`. For `Result(T, E)` types:
- `Ok(v)`: Extracts value `v` and continues execution
- `Err(e)`: Propagates the error upward (`return Err(e)`)

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

### 2.13 ref Expressions

```
RefExpr     ::= 'ref' Expr
```

`ref` creates a shared reference. The compiler automatically chooses Rc (single-task) or Arc (cross-task); users don't need to worry about implementation details.

```yaoxiang
data = ref heavy_data
spawn { use(data) }   // Cross-task: compiler automatically chooses Arc
```

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

### 3.2 Variable Declarations

```
LetStmt     ::= ('mut')? Identifier (':' TypeExpr)? '=' Expr
```

### 3.3 return Statements

```
ReturnStmt  ::= 'return' Expr?
```

**Semantics**: `return` is used to return values from code blocks. Without `return`, code blocks default to returning `Void`.

### 3.4 break Statements

```
BreakStmt   ::= 'break' Identifier?
```

### 3.5 continue Statements

```
ContinueStmt::= 'continue'
```

### 3.6 if Statements

```
IfStmt      ::= 'if' Expr Block ('elif' Expr Block)* ('else' Block)?
```

### 3.7 match Statements

```
MatchStmt   ::= 'match' Expr '{' MatchArm+ '}'
```

### 3.8 while Statements

```
WhileStmt   ::= 'while' Expr Block
```

### 3.9 for Statements

```
ForStmt     ::= 'for' 'mut'? Identifier 'in' Expr Block
```

#### 3.9.1 Semantics: Each Iteration Binds a New Value

The for loop semantics in YaoXiang differs from traditional languages: **Each iteration binds a new value, rather than modifying the same variable**.

```yaoxiang
// Example: for i in 1..5
for i in 1..5 {
    print(i)
}
```

**Execution Process**:

| Iteration | Behavior of Loop Variable |
|-----------|---------------------------|
| 1st | Creates new binding `i = 1`, loop body executes, prints 1 |
| 2nd | Creates new binding `i = 2` (previous binding destroyed), loop body executes, prints 2 |
| 3rd | Creates new binding `i = 3`, loop body executes, prints 3 |
| 4th | Creates new binding `i = 4`, loop body executes, prints 4 |
| End | Loop body ends, binding destroyed |

**Key Point**: At the end of each iteration, the binding created during that iteration is destroyed. The next iteration is a completely new binding with no relationship to the previous iteration's binding.

#### 3.9.2 Difference Between for and for mut

| Syntax | Loop Variable Mutability | Description |
|--------|-------------------------|-------------|
| `for i in 1..5` | Immutable | Cannot modify binding in loop body |
| `for mut i in 1..5` | Mutable | Can modify binding in loop body |

```yaoxiang
// Valid: Each iteration binds a new value, no need to modify
for i in 1..5 {
    print(i)  // Read value of i
}

// Error: Immutable binding, cannot modify
for i in 1..5 {
    i = i + 1  // Error: cannot modify immutable binding
}

// Valid: Using for mut allows modification
for mut i in 1..5 {
    i = i + 1  // Allowed to modify
}
```

#### 3.9.3 Shadowing Check

YaoXiang prohibits variable shadowing. Loop variables cannot have the same name as variables in outer scopes:

```yaoxiang
// Error: i already declared outside
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

This rule applies to all code blocks. See [4.3 Shadowing Rules](./modules.md#43-shadowing-rules).

#### 3.9.4 Comparison with Other Languages

| Language | for Loop Variable Semantics |
|----------|----------------------------|
| YaoXiang | Each iteration binds a new value |
| Rust | Modifies the same variable (requires mut) |
| Python | Modifies the same variable (no mut needed) |
| C/C++ | Modifies the same variable (requires pointer or reference) |

**Design Rationale**: YaoXiang adopts binding semantics because:

1. **More aligned with natural semantics**
   In natural language, "for each element x in a set" means each x is an independent entity. YaoXiang's `for i in 1..5` reads as "for each i in 1 to 5", and each iteration's i is a completely new binding. This aligns with human intuition.

2. **Prevents accidental modification**
   The default immutable binding semantics means the loop body cannot accidentally modify the loop variable. You don't need to worry about accidentally writing `i = ...` somewhere in a complex loop body, causing hard-to-trace bugs.

3. **High-performance solutions are readily available**
   When you genuinely need to reuse a variable between iterations (e.g., accumulator, cache), you can simply switch to mutable binding mode with `for mut`. This is clearer than implicit shared state—intent is expressed explicitly through syntax rather than hidden in runtime behavior.

### 3.10 spawn Statements

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' 'mut'? Identifier 'in' Expr '{' Expr '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
```

**spawn blocks**: Explicitly declare concurrency scope; expressions inside the block execute concurrently.

```yaoxiang
(result_a, result_b) = spawn {
    parse(fetch("url1")),
    parse(fetch("url2"))
}
```

**spawn loops**: Data-parallel loops.

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