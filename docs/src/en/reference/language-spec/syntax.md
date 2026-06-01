# Syntax Specification

This document defines the syntax specification of the YaoXiang programming language, including lexical structure, syntax rules, and operator precedence.

---

## Chapter 1: Lexical Structure

### 1.1 Source Files

YaoXiang source files must use UTF-8 encoding. Source files typically use the `.yx` extension.

### 1.2 Token Classification

| Category | Description | Example |
|------|------|------|
| Identifiers | Starting with letter or underscore | `x`, `_private`, `my_var` |
| Keywords | Language-defined reserved words | `Type`, `pub`, `use` |
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
|--------|------|------|
| `Type` | Type | Meta type |
| `true` | Bool | Boolean true |
| `false` | Bool | Boolean false |
| `void` | Void | Void |
| `some(T)` | Option | Option value variant |
| `ok(T)` | Result | Result success variant |
| `err(E)` | Result | Result error variant |

### 1.5 Identifiers

Identifiers start with a letter or underscore, and subsequent characters can be letters, digits, or underscores. Identifiers are case-sensitive.

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

Code must use 4 spaces for indentation. Tab characters are prohibited. This is a mandatory syntax rule.

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

| Precedence | Operators | Associativity |
|--------|--------|--------|
| 1 | `()` `[]` `.` `?` | Left to Right |
| 2 | `as` | Left to Right |
| 3 | `*` `/` `%` | Left to Right |
| 4 | `+` `-` | Left to Right |
| 5 | `..` | Left to Right |
| 6 | `<<` `>>` | Left to Right |
| 7 | `&` `\|` `^` | Left to Right |
| 8 | `==` `!=` `<` `>` `<=` `>=` | Left to Right |
| 9 | `not` | Right to Left |
| 10 | `and` `or` | Left to Right |
| 11 | `if...else` | Right to Left |
| 12 | `=` `+=` `-=` `*=` `/=` | Right to Left |

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

### 2.11 Error Propagation Operator

```
ErrorPropagate ::= Expr '?'
```

The `?` operator is a postfix operator with the same precedence as `.`. For `Result(T, E)` type:
- On `Ok(v)`: extracts value `v` and continues execution
- On `Err(e)`: propagates the error upward (`return Err(e)`)

```yaoxiang
process: (data: Data) -> Result(Data, Error) = {
    validated = validate(data)?     // On success: extract value; on failure: propagate
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

`ref` creates a shared reference. The compiler automatically chooses Rc (single-task) or Arc (cross-task), and users don't need to worry about implementation details.

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

#### 3.9.1 Semantics: Each Iteration Creates a New Binding

The YaoXiang for loop semantics differ from traditional languages: **each iteration creates a new binding, rather than modifying the same variable**.

```yaoxiang
// Example: for i in 1..5
for i in 1..5 {
    print(i)
}
```

**Execution Process**:

| Iteration | Loop Variable Behavior |
|------|----------------|
| 1st | Create new binding `i = 1`, execute loop body, print 1 |
| 2nd | Create new binding `i = 2` (previous binding destroyed), execute loop body, print 2 |
| 3rd | Create new binding `i = 3`, execute loop body, print 3 |
| 4th | Create new binding `i = 4`, execute loop body, print 4 |
| End | Loop body ends, binding destroyed |

**Key Point**: After each iteration ends, the binding created during that iteration is destroyed. The next iteration is a completely new binding with no relationship to the previous iteration's binding.

#### 3.9.2 Difference Between for and for mut

| Syntax | Loop Variable Mutability | Description |
|------|----------------|------|
| `for i in 1..5` | Immutable | Cannot modify binding in loop body |
| `for mut i in 1..5` | Mutable | Can modify binding in loop body |

```yaoxiang
// Valid: each iteration creates a new binding, no need to modify
for i in 1..5 {
    print(i)  // Read value of i
}

// Invalid: immutable binding, cannot modify
for i in 1..5 {
    i = i + 1  // Error: cannot modify immutable binding
}

// Valid: using for mut allows modification
for mut i in 1..5 {
    i = i + 1  // Allowed
}
```

#### 3.9.3 Shadowing Check

YaoXiang prohibits variable shadowing. For loop variables cannot have the same name as variables in the outer scope:

```yaoxiang
// Invalid: i already declared externally
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

This rule applies to all code blocks. See [4.3 Shadowing Rules](./modules.md#43-shadowing-rules) for details.

#### 3.9.4 Comparison with Other Languages

| Language | for Loop Variable Semantics |
|------|------------------|
| YaoXiang | Each iteration creates a new binding |
| Rust | Modifies the same variable (requires mut) |
| Python | Modifies the same variable (no mut needed) |
| C/C++ | Modifies the same variable (requires pointer or reference) |

**Design Rationale**: YaoXiang adopts binding semantics because:

1. **More aligned with natural semantics**
   In natural language, "for each element x in a collection" implies each x is an independent individual. YaoXiang's `for i in 1..5` reads as "for each i in 1 to 5", and the i in each iteration is a brand new binding. This is consistent with human intuition.

2. **Avoids accidental modifications**
   The default immutable binding semantics mean the loop body cannot accidentally modify the loop variable. There's no need to worry about accidentally writing `i = ...` somewhere in a complex loop body causing hard-to-track bugs.

3. **High-performance solutions are readily accessible**
   When you genuinely need to reuse a variable between iterations (e.g., accumulator, cache), you can simply use `for mut` to switch to mutable binding mode. This is clearer than implicit shared state—the intent is expressed explicitly through syntax rather than hidden in runtime behavior.

### 3.10 spawn Statements

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' 'mut'? Identifier 'in' Expr '{' Expr '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
```

**spawn block**: Explicitly declares concurrency scope, expressions inside the block execute concurrently.

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