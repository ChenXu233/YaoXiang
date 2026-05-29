# Syntax Specification

This document defines the syntax specification for the YaoXiang programming language, including lexical structure, grammar rules, and operator precedence.

---

## Chapter 1: Lexical Structure

### 1.1 Source Files

YaoXiang source files must use UTF-8 encoding. Source files typically use the `.yx` extension.

### 1.2 Token Classification

| Category | Description | Examples |
|----------|-------------|----------|
| Identifiers | Starting with a letter or underscore | `x`, `_private`, `my_var` |
| Keywords | Language-reserved words | `Type`, `pub`, `use` |
| Literals | Fixed values | `42`, `"hello"`, `true` |
| Operators | Arithmetic symbols | `+`, `-`, `*`, `/` |
| Delimiters | Syntax separators | `(`, `)`, `{`, `}`, `,` |

### 1.3 Keywords

YaoXiang defines very few keywords:

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

Identifiers start with a letter or underscore, and subsequent characters can be letters, digits, or underscores. Identifiers are case-sensitive.

Special identifiers:
- `_` is used as a placeholder, indicating ignoring a value
- Identifiers starting with underscore indicate private members

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

#### 1.6.6 Membership Tests

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
              | IfExpr
              | MatchExpr
              | Block
              | Lambda
```

### 2.2 Operator Precedence

| Precedence | Operators | Associativity |
|------------|-----------|---------------|
| 1 | `()` `[]` `.` | Left to right |
| 2 | `as` | Left to right |
| 3 | `*` `/` `%` | Left to right |
| 4 | `+` `-` | Left to right |
| 5 | `<<` `>>` | Left to right |
| 6 | `&` `\|` `^` | Left to right |
| 7 | `==` `!=` `<` `>` `<=` `>=` | Left to right |
| 8 | `not` | Right to left |
| 9 | `and` `or` | Left to right |
| 10 | `if...else` | Right to left |
| 11 | `=` `+=` `-=` `*=` `/=` | Right to left |

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

### 2.6 Type Casting

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

### 2.10 Lambda Expressions

```
Lambda      ::= '(' ParamList? ')' '=>' Expr
            |  '(' ParamList? ')' '=>' Block
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
              | LoopStmt
              | WhileStmt
              | ForStmt
```

### 3.2 Variable Declarations

```
LetStmt     ::= ('mut')? Identifier (':' TypeExpr)? '=' Expr
```

### 3.3 return Statements

```
ReturnStmt  ::= 'return' Expr?
```

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

The semantics of YaoXiang's for loop differs from traditional languages: **each iteration binds a new value, rather than modifying the same variable**.

```yaoxiang
// Example: for i in 1..5
for i in 1..5 {
    print(i)
}
```

**Execution Process**:

| Iteration | Behavior of Loop Variable |
|-----------|---------------------------|
| 1st | Create new binding `i = 1`, execute loop body, print 1 |
| 2nd | Create new binding `i = 2` (previous binding destroyed), execute loop body, print 2 |
| 3rd | Create new binding `i = 3`, execute loop body, print 3 |
| 4th | Create new binding `i = 4`, execute loop body, print 4 |
| End | Loop body ends, binding destroyed |

**Key Point**: After each iteration ends, the binding created during that iteration is destroyed. The next iteration is a completely new binding that has no relationship with the previous iteration's binding.

#### 3.9.2 Difference Between for and for mut

| Syntax | Loop Variable Mutability | Description |
|--------|---------------------------|-------------|
| `for i in 1..5` | Immutable | Cannot modify binding in loop body |
| `for mut i in 1..5` | Mutable | Can modify binding in loop body |

```yaoxiang
// Valid: Each iteration binds a new value, no need to modify
for i in 1..5 {
    print(i)  // Read the value of i
}

// Error: Immutable binding, cannot modify
for i in 1..5 {
    i = i + 1  // Error: Cannot modify immutable binding
}

// Valid: Using for mut allows modification
for mut i in 1..5 {
    i = i + 1  // Allowed to modify
}
```

#### 3.9.3 Shadowing Check

for loop variables cannot shadow variables that already exist in the outer scope:

```yaoxiang
// Error: i is already declared externally
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

Error code: `E2013 - Cannot shadow existing variable`

#### 3.9.4 Comparison with Other Languages

| Language | for Loop Variable Semantics |
|----------|----------------------------|
| YaoXiang | Each iteration binds a new value |
| Rust | Modifies the same variable (requires mut) |
| Python | Modifies the same variable (no mut needed) |
| C/C++ | Modifies the same variable (requires pointer or reference) |

**Design Rationale**: YaoXiang uses binding semantics because:
1. Variables in the loop body are destroyed after each iteration ends
2. The next iteration is a completely new binding
3. This is safer; no need to consider state between iterations

---

## Appendix: Syntax Quick Reference

### A.1 Control Flow

```
if Expr Block (elif Expr Block)* (else Block)?
match Expr { MatchArm+ }
while Identifier in Expr Block Expr Block
for
```

### A.2 match Syntax

```
match value {
    pattern1 => expr1,
    pattern2 if guard => expr2,
    _ => default_expr,
}
```