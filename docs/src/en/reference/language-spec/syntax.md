# Syntax Specification

This document defines the syntax specification of the YaoXiang programming language, including
lexical structure, grammar rules, and operator precedence.

---

## Chapter 1: Lexical Structure

### 1.1 Source File

YaoXiang source files must use UTF-8 encoding. Source files typically use the `.yx` extension.

### 1.2 Lexical Unit Classification

| Category   | Description                        | Example                   |
| ---------- | ---------------------------------- | ------------------------- |
| Identifier | Begins with a letter or underscore | `x`, `_private`, `my_var` |
| Keyword    | Language-predefined reserved words | `Type`, `pub`, `use`      |
| Literal    | Fixed values                       | `42`, `"hello"`, `true`   |
| Operator   | Operation symbols                  | `+`, `-`, `*`, `/`        |
| Delimiter  | Syntax separators                  | `(`, `)`, `{`, `}`, `,`   |

### 1.3 Keywords

YaoXiang defines a very small set of keywords:

```
pub    use    spawn
ref    mut    if     else
else   match  while  for    return
break  continue as     in     unsafe
```

These keywords have special meaning in any context and cannot be used as identifiers.

### 1.4 Reserved Words

YaoXiang's "reserved words" are organized into three layers, recognized at different stages by the
parser and type checker:

#### 1.4.1 Literal Reserved Words

Literal identifiers that have independent tokens in the parser and cannot be used as ordinary
identifiers:

| Identifier | Owning Type | Description                                                                                                           |
| ---------- | ----------- | --------------------------------------------------------------------------------------------------------------------- |
| `Type`     | —           | Meta-type keyword                                                                                                     |
| `true`     | Bool        | Boolean true value                                                                                                    |
| `false`    | Bool        | Boolean false value                                                                                                   |
| `void`     | Void        | Void literal (Unit value). The lowercase `void` is a value literal; the uppercase `Void` is a type name (see §1.4.3). |

#### 1.4.2 Constructor Expressions

The following constructors are recognized by the parser in pattern matching and expression contexts:

| Constructor | Owning Type | Description                      |
| ----------- | ----------- | -------------------------------- |
| `some(T)`   | Option      | Option value variant constructor |
| `ok(T)`     | Result      | Result success variant           |
| `err(E)`    | Result      | Result error variant             |

#### 1.4.3 Built-in Type Names

The following type names are pre-registered by the type checker and can be used in type positions
without imports. The parser treats them as ordinary identifiers—**not reserved words, and can be
shadowed by local bindings (not recommended)**.

| Type Name | Logical Correspondence | Description                                                                                                                                        |
| --------- | ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Void`    | ⊤ (true/Unit)          | A zero-field product type with exactly one inhabitant (the `void` literal, see §1.4.1)                                                             |
| `Never`   | ⊥ (false/empty type)   | A zero-variant sum type with zero inhabitants. No expression can produce a `Never` value. `Never <: T` holds for all `T` (principle of explosion). |
| `Int`     | —                      | Signed integer                                                                                                                                     |
| `Float`   | —                      | Floating-point number                                                                                                                              |
| `Bool`    | —                      | Boolean value: `true` / `false`                                                                                                                    |
| `Char`    | —                      | Unicode character                                                                                                                                  |
| `String`  | —                      | String                                                                                                                                             |

### 1.5 Identifiers

Identifiers begin with a letter or underscore, followed by zero or more letters, digits, or
underscores. Identifiers are case-sensitive.

Special identifiers:

- `_` is used as a placeholder, indicating that a value is ignored
- Identifiers beginning with an underscore denote private members

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
Array       ::= '[' Expr (',' Expr)* ']'   // When target type annotation is Array(T, N), the literal resolves to a fixed-length array
```

> #299: Set has no literal grammar—construct sets using `std.set`. The resolution of List/Dict
> literals is determined by context type annotation: a bare literal or `List(T)` annotation resolves
> to a growable list; an `Array(T, N)` annotation applied directly to the literal resolves to a
> fixed-length array. Implicit List→Array conversion is forbidden.

#### 1.6.5 List Comprehensions

```
ListComp    ::= '[' Expr 'for' Identifier 'in' Expr (',' Expr)* ('if' Expr)? ']'
```

#### 1.6.6 Membership Test

```
Membership  ::= Expr 'in' Expr
```

> #299 §3: `in` is a binary relational operator that returns `Bool`—returns `true` on hit, `false`
> on miss, and does not error. Semantic division: `[]` is an assertion of existence and fetches the
> value (errors on failure), while `in` asks whether something exists (a miss is a normal `false`).
> Right operand coverage: List / Array / Dict (key set) / Set / Tuple / String (substring) / Range
> (interval). `in` is a first-class Hoare predicate, serving as the basis for compile-time provable
> propositions in the refinement type stage.

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

> **Unary prefix operators** (`!` `-` `+`) bind tightly: they are only lower than calls and member
> access, and higher than all binary operators. Therefore `!a == b` ≡ `(!a) == b` (Zig-style
> semantics); `!` is a pure unary operation, does not participate in short-circuit control flow, and
> is orthogonal to the `and`/`or` keywords (short-circuit) (authoritative definition in RFC-010).

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

### 2.9 Block Expressions

```
Block       ::= '{' Stmt* Expr? '}'
```

> **Statement termination rules**: The separation between Stmts and newline behavior (explicit `;`
> separation, newline termination, line continuation exceptions, and `(`/`[` at the start of a line
> never merging) are defined by [RFC-038](../design/rfc/draft/038-statement-termination.md).

**Unified semantics**: All `{}` blocks have consistent return semantics:

| Block Type    | return Semantics          | Default Return |
| ------------- | ------------------------- | -------------- |
| Ordinary `{}` | Returns a value           | Void           |
| `unsafe {}`   | Returns a type definition | Void           |
| `spawn {}`    | Returns a result          | Void           |

**Core principles**:

- `return` inside `{}` always returns its content to the enclosing scope
- Without an explicit `return`, the default return is `Void`
- The expression form `= expr` directly returns the value

```yaoxiang
// Ordinary {} block: return returns a value
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

// spawn {} block: return returns a result
(a, b) = spawn {
    result1 = fetch("url1"),
    result2 = fetch("url2")
    return (result1, result2)  // Returns the result to the enclosing scope
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

The `?` operator is a postfix operator with the same precedence as `.`. For the `Result(T, E)` type:

- On `Ok(v)`, extract the value `v` and continue execution
- On `Err(e)`, propagate the error upward (`return Err(e)`)

```yaoxiang
process: (data: Data) -> Result(Data, Error) = {
    validated = validate(data)?     // Extracts the value on success, propagates on failure
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

`ref` creates a shared holding. The compiler automatically chooses Rc (single-task) or Arc
(cross-task); users do not need to worry about implementation details.

```yaoxiang
data = ref heavy_data
spawn { use(data) }   // Cross-task: compiler automatically picks Arc
```

### 2.14 unsafe Expressions

```
UnsafeExpr  ::= 'unsafe' Block
```

The `unsafe` block is used to define opaque types and operate on raw pointers. Use `return` to
return a type definition to the enclosing scope.

**Semantics**:

- Types and raw pointer operations can be defined inside `unsafe {}`
- The returned type is usable outside the `unsafe {}`
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

### 2.15 Scopes

**Basic rules**:

- Each `{}` block creates a scope
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

**Variable declaration and shadowing**:

- `x = value`: Search outward along the scope chain for x; if found, assign; if not, declare a new
  one
- `mut x = value`: Explicit new mutable declaration; same name as an outer variable is forbidden
- Within the same scope, any name can only be declared once

> **Detailed definition**: For the complete rules of scopes, variable declarations, and the
> shadowing mechanism, see [Module System Specification](./modules.md#chapter-4-scopes).

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

**Semantics**: `return` is used to return a value from a code block. Without a `return`, the code
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

YaoXiang's for loop semantics differ from traditional languages: **each iteration binds a new value,
rather than mutating the same variable**.

```yaoxiang
// Example: for i in 1..5
for i in 1..5 {
    print(i)
}
```

**Execution process**:

| Iteration | Behavior of the loop variable                                                                       |
| --------- | --------------------------------------------------------------------------------------------------- |
| 1st       | Creates a new binding `i = 1`, executes the loop body, prints 1                                     |
| 2nd       | Creates a new binding `i = 2` (the previous binding is destroyed), executes the loop body, prints 2 |
| 3rd       | Creates a new binding `i = 3`, executes the loop body, prints 3                                     |
| 4th       | Creates a new binding `i = 4`, executes the loop body, prints 4                                     |
| End       | Loop body ends, binding is destroyed                                                                |

**Key point**: After each iteration, the binding created in that iteration is destroyed. The next
iteration is a completely fresh binding, with no relationship to the previous iteration's binding.

#### 3.9.2 Difference Between for and for mut

| Syntax              | Loop Variable Mutability | Description                                         |
| ------------------- | ------------------------ | --------------------------------------------------- |
| `for i in 1..5`     | Immutable                | The binding cannot be modified inside the loop body |
| `for mut i in 1..5` | Mutable                  | The binding can be modified inside the loop body    |

```yaoxiang
// Valid: each iteration binds a new value; no modification needed
for i in 1..5 {
    print(i)  // Read the value of i
}

// Error: immutable binding cannot be modified
for i in 1..5 {
    i = i + 1  // Error: cannot modify an immutable binding
}

// Valid: use for mut to allow modifying the binding
for mut i in 1..5 {
    i = i + 1  // Modification allowed
}
```

#### 3.9.3 Shadowing Check

YaoXiang forbids variable shadowing. A for loop variable cannot share the same name as a variable in
an outer scope:

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

This rule applies to all code blocks; see [4.3 Shadowing Rules](./modules.md#43-shadowing-rules) for
details.

#### 3.9.4 Comparison with Other Languages

| Language | for Loop Variable Semantics                               |
| -------- | --------------------------------------------------------- |
| YaoXiang | Each iteration binds a new value                          |
| Rust     | Mutates the same variable (requires mut)                  |
| Python   | Mutates the same variable (no mut required)               |
| C/C++    | Mutates the same variable (requires pointer or reference) |

**Design rationale**: YaoXiang adopts binding semantics because:

1. **More aligned with natural semantics** In natural language, "for every element x in the
   collection" means that each x is an independent individual. YaoXiang's `for i in 1..5` is read as
   "for every i from 1 to 5", where the i in each iteration is a completely new binding, consistent
   with human intuition.

2. **Prevents accidental modification** The default immutable binding semantics means the loop
   variable cannot be accidentally modified inside the loop body. There is no need to worry about a
   hard-to-trace bug caused by accidentally writing `i = ...` somewhere deep inside a complex loop
   body.

3. **High-performance solutions are within reach** When it is genuinely necessary to reuse a
   variable across iterations (e.g., accumulators, caches), use `for mut` to switch to the mutable
   binding mode. This is clearer than implicit shared state—the intent is expressed explicitly
   through syntax, not hidden in runtime behavior.

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

**spawn blocks capture outer variables** (RFC-024 §2.3, value capture semantics):

- The block body referencing outer variables = **Move value capture**: the value is snapshotted into
  the closure environment at the spawn creation point, and the block body reads it via the env
  (LoadUpvalue)
- **Primitives** (Int/Float/Bool/Char) are value-copied, leaving outer variables unaffected
- **Handle types** (Struct/String/List, etc.) are snapshotted = handle copy, sharing the underlying
  object; in the Embedded runtime (default), same thread and same heap make the handle valid
- Sharing between multiple tasks requires an explicit `ref` (§2.13, compiler automatically picks
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
