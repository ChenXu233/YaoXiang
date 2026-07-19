# Syntax Specification

This document defines the syntax specification of the YaoXiang programming language, including lexical structure, grammar rules, and operator precedence.

---

## Chapter 1: Lexical Structure

### 1.1 Source Files

YaoXiang source files must use UTF-8 encoding. Source files typically use the `.yx` extension.

### 1.2 Token Categories

| Category | Description | Example |
|------|------|------|
| Identifier | Starts with a letter or underscore | `x`, `_private`, `my_var` |
| Keyword | Language-predefined reserved words | `Type`, `pub`, `use` |
| Literal | Fixed values | `42`, `"hello"`, `true` |
| Operator | Operation symbols | `+`, `-`, `*`, `/` |
| Separator | Syntax delimiters | `(`, `)`, `{`, `}`, `,` |

### 1.3 Keywords

YaoXiang defines a minimal set of keywords:

```
pub    use    spawn
ref    mut    if     elif
else   match  while  for    return
break  continue as     in     unsafe
```

These keywords have special meaning in any context and cannot be used as identifiers.

### 1.4 Reserved Words

YaoXiang's "reserved words" are organized into three layers, recognized by the parser and type checker at different stages:

#### 1.4.1 Literal Reserved Words

The following are literal identifiers with independent tokens in the parser and cannot be used as ordinary identifiers:

| Identifier | Belongs To | Description |
|--------|---------|------|
| `Type` | — | Meta type keyword |
| `true` | Bool | Boolean true value |
| `false` | Bool | Boolean false value |
| `void` | Void | Void literal (Unit value). Lowercase `void` is a value literal; uppercase `Void` is a type name (see §1.4.3). |

#### 1.4.2 Constructor Expressions

The following constructors are recognized by the parser in pattern matching and expression contexts:

| Constructor | Belongs To | Description |
|--------|---------|------|
| `some(T)` | Option | Option value variant construction |
| `ok(T)` | Result | Result success variant |
| `err(E)` | Result | Result error variant |

#### 1.4.3 Builtin Type Names

The following type names are pre-registered by the type checker and can be used in type positions without import. The parser treats them as ordinary identifiers—**not reserved words, can be shadowed by local bindings (not recommended)**.

| Type Name | Logical Correspondence | Description |
|--------|---------|------|
| `Void` | ⊤ (true/Unit) | Zero-field product type, with exactly one inhabitant (`void` literal, see §1.4.1) |
| `Never` | ⊥ (false/empty type) | Zero-variant sum type, with zero inhabitants. No expression can produce a `Never` value. `Never <: T` holds for all `T` (principle of explosion). |
| `Int` | — | Signed integer |
| `Float` | — | Floating-point number |
| `Bool` | — | Boolean value: `true` / `false` |
| `Char` | — | Unicode character |
| `String` | — | String |

### 1.5 Identifiers

Identifiers start with a letter or underscore, followed by letters, digits, or underscores. Identifiers are case-sensitive.

Special identifiers:
- `_` is used as a placeholder to indicate a value to be ignored
- Identifiers starting with an underscore denote private members

### 1.6 Literals

#### 1.6.1 Integers

```
Decimal     ::= [0-9][0-9_]*
Octal       ::= 0o[0-7][0-7_]*
Hex         ::= 0x[0-9a-fA-F][0-9a-fA-F_]*
Binary      ::= 0b[01][01_]*
```

#### 1.6.2 Floats

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
   can span multiple lines */
```

### 1.8 Indentation Rules

Code must use 4 spaces for indentation; Tab characters are forbidden. This is a mandatory syntax rule.

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

| Precedence | Operator | Associativity |
|--------|--------|--------|
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

| Block Type | return Semantics | Default Return |
|--------|------------|----------|
| Plain `{}` | Returns a value | Void |
| `unsafe {}` | Returns a type definition | Void |
| `spawn {}` | Returns a result | Void |

**Core Principles**:
- `return` inside `{}` always returns content to the enclosing scope
- Without `return`, the default value is `Void`
- The expression form `= expr` returns the value directly

```yaoxiang
// Plain {} block: return returns a value
result = {
    x = compute()
    return x  // returns the value to the enclosing scope
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
- When `Ok(v)`, extract the value `v` and continue execution
- When `Err(e)`, propagate the error upward (`return Err(e)`)

```yaoxiang
process: (data: Data) -> Result(Data, Error) = {
    validated = validate(data)?     // extract value on success, propagate on failure
    transform(validated)
}
```

### 2.12 Range Expressions

```
RangeExpr   ::= Expr '..' Expr
```

`..` creates a range type, used in `for` loops and slicing.

```yaoxiang
for i in 0..10 { print(i) }
slice = array[0..5]
```

### 2.13 ref Expressions

```
RefExpr     ::= 'ref' Expr
```

`ref` creates a shared holding. The compiler automatically chooses Rc (single-task) or Arc (cross-task), and users do not need to worry about implementation details.

```yaoxiang
data = ref heavy_data
spawn { use(data) }   // cross-task: the compiler automatically chooses Arc
```

### 2.14 unsafe Expressions

```
UnsafeExpr  ::= 'unsafe' Block
```

The `unsafe` block is used to define opaque types and operate on raw pointers. Use `return` to return the type definition to the enclosing scope.

**Semantics**:
- `unsafe {}` can define types and operate on raw pointers
- The returned type is available outside of `unsafe {}`
- Field access of the type requires unsafe permission

```yaoxiang
// Define an opaque type inside an unsafe block
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  // raw pointer
    }
    return SqliteDb
}

// SqliteDb is available outside of the unsafe block
db = sqlite3_open("test.db")
```

### 2.15 Scopes

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
// x is not visible outside of this scope

// Function scope
add: (a: Int, b: Int) -> Int = {
    result = a + b
    return result
}
// result is not visible outside of the function
```

**Variable Declaration and Shadowing**:
- `x = value`: search outward along the scope chain for x; if found, assign; if not found, declare a new one
- `mut x = value`: explicit new mutable declaration, forbidden to share a name with the outer scope
- Any name can only be declared once within the same scope

> **Detailed Definition**: The complete rules of scopes, variable declaration, and shadowing mechanism are described in [Module System Specification](./modules.md#chapter-4-scopes).

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

**Semantics**: `return` is used to return a value from a code block. Without `return`, the code block defaults to returning `Void`.

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

YaoXiang's for loop semantics differ from traditional languages: **each iteration binds a new value, rather than modifying the same variable**.

```yaoxiang
// Example: for i in 1..5
for i in 1..5 {
    print(i)
}
```

**Execution Process**:

| Iteration | Loop Variable Behavior |
|------|----------------|
| 1st | Create a new binding `i = 1`, execute loop body, print 1 |
| 2nd | Create a new binding `i = 2` (previous binding is destroyed), execute loop body, print 2 |
| 3rd | Create a new binding `i = 3`, execute loop body, print 3 |
| 4th | Create a new binding `i = 4`, execute loop body, print 4 |
| End | Loop body ends, bindings are destroyed |

**Key Point**: After each iteration ends, the binding created in that iteration is destroyed. The next iteration is a completely new binding, with no relation to the previous iteration's binding.

#### 3.9.2 The Difference Between for and for mut

| Syntax | Loop Variable Mutability | Description |
|------|----------------|------|
| `for i in 1..5` | Immutable | The binding cannot be modified inside the loop body |
| `for mut i in 1..5` | Mutable | The binding can be modified inside the loop body |

```yaoxiang
// Legal: each iteration binds a new value, no modification needed
for i in 1..5 {
    print(i)  // read the value of i
}

// Error: immutable binding, cannot be modified
for i in 1..5 {
    i = i + 1  // Error: cannot modify an immutable binding
}

// Legal: use for mut to allow modification of the binding
for mut i in 1..5 {
    i = i + 1  // modification allowed
}
```

#### 3.9.3 Shadowing Check

YaoXiang prohibits variable shadowing. The for loop variable cannot share a name with a variable in an outer scope:

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

| Language | for Loop Variable Semantics |
|------|------------------|
| YaoXiang | Each iteration binds a new value |
| Rust | Modifies the same variable (requires mut) |
| Python | Modifies the same variable (no mut required) |
| C/C++ | Modifies the same variable (requires pointers or references) |

**Design Rationale**: YaoXiang adopts binding semantics for the following reasons:

1. **More aligned with natural semantics**
   In natural language, "for each element x in the collection" means each x is an independent individual. YaoXiang's `for i in 1..5` reads as "for each i in 1 to 5", and the i in each iteration is a brand new binding, which aligns with human intuitive understanding.

2. **Avoiding accidental modification**
   The default immutable binding semantics means the loop variable cannot be accidentally modified inside the loop body. There is no need to worry about writing `i = ...` somewhere in a complex loop body, which would lead to hard-to-track bugs.

3. **High-performance solutions are within reach**
   When it is indeed necessary to reuse a variable across iterations (e.g., accumulators, caches), declare `for mut` to switch to mutable binding mode. This is clearer than implicit shared state—the intent is explicitly expressed through syntax, not hidden in runtime behavior.

### 3.10 spawn Statement

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
SpawnFor    ::= Identifier '=' 'spawn' 'for' 'mut'? Identifier 'in' Expr '{' Expr '}'
SpawnStmt   ::= SpawnBlock | SpawnFor
```

**spawn Block**: Explicitly declares a concurrent region; expressions inside the block execute concurrently.

```yaoxiang
(result_a, result_b) = spawn {
    parse(fetch("url1")),
    parse(fetch("url2"))
}
```

**spawn Loop**: Data-parallel loop.

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