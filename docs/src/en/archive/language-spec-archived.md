> **Note: This document has been archived and is no longer maintained.**
> **Please refer to the new language specification document: [Language Specification](../reference/language-spec/index.md)**

---

# YaoXiang Programming Language Specification

> Version: v1.8.0
> Status: Specification
> Author: Chen Xu
> Date: 2024-12-31
> Updated: 2026-02-22 - Meta type is not a keyword.

---

## Chapter 1: Introduction

### 1.1 Scope

This document defines the syntax and semantics of the YaoXiang programming language. It serves as the authoritative reference for the language, targeted at compiler and tool implementers.

For tutorials and example code, please refer to the [YaoXiang Guide](../guide/YaoXiang-book.md) and the [tutorial/](../tutorial/) directory.

### 1.2 Conformance

A program or implementation is considered conforming to the YaoXiang specification if it satisfies all rules defined in this document.

---

## Chapter 2: Lexical Structure

### 2.1 Source Files

YaoXiang source files must use UTF-8 encoding. Source files typically use `.yx` as the extension.

### 2.2 Token Classification

| Category | Description | Examples |
|----------|-------------|----------|
| Identifier | Starts with a letter or underscore | `x`, `_private`, `my_var` |
| Keyword | Language-predefined reserved words | `Type`, `pub`, `use` |
| Literal | Fixed values | `42`, `"hello"`, `true` |
| Operator | Arithmetic symbols | `+`, `-`, `*`, `/` |
| Delimiter | Syntax separators | `(`, `)`, `{`, `}`, `,` |

### 2.3 Keywords

YaoXiang defines very few keywords:

```
pub    use    spawn
ref    mut    if     elif
else   match  while  for    return
break  continue as     in     unsafe
```

These keywords have special meaning in any context and cannot be used as identifiers.

### 2.4 Reserved Words

| Reserved Word | Type | Description |
|---------------|------|-------------|
| `Type` | Type | Meta type |
| `true` | Bool | Boolean true |
| `false` | Bool | Boolean false |
| `void` | Void | void value |
| `some(T)` | Option | Option value variant |
| `ok(T)` | Result | Result success variant |
| `err(E)` | Result | Result error variant |

### 2.5 Identifiers

Identifiers start with a letter or underscore, and subsequent characters can be letters, digits, or underscores. Identifiers are case-sensitive.

Special identifiers:
- `_` is used as a placeholder to ignore a value
- Identifiers starting with an underscore indicate private members

### 2.6 Literals

#### 2.6.1 Integers

```
Decimal     ::= [0-9][0-9_]*
Octal       ::= 0o[0-7][0-7_]*
Hex         ::= 0x[0-9a-fA-F][0-9a-fA-F_]*
Binary      ::= 0b[01][01_]*
```

#### 2.6.2 Floating-Point Numbers

```
Float       ::= [0-9][0-9_]* '.' [0-9][0-9_]* ([eE][+-]?[0-9][0-9_]*)?
```

#### 2.6.3 Strings

```
String      ::= '"' ([^"\\] | EscapeSequence)* '"'
Escape      ::= '\\' ([nrt'"\\] | UnicodeEscape)
Unicode     ::= 'u' '{' HexDigit+ '}'
```

#### 2.6.4 Collections

```
List        ::= '[' Expr (',' Expr)* ']'
Dict        ::= '{' String ':' Expr (',' String ':' Expr)* '}'
Set         ::= '{' Expr (',' Expr)* '}'
```

#### 2.6.5 List Comprehensions

```
ListComp    ::= '[' Expr 'for' Identifier 'in' Expr (',' Expr)* ('if' Expr)? ']'
```

#### 2.6.6 Membership Test

```
Membership  ::= Expr 'in' Expr
```

### 2.7 Comments

```
// Single-line comment

/* Multi-line comment
   Can span multiple lines */
```

### 2.8 Indentation Rules

Code must use 4 spaces for indentation, and Tab characters are prohibited. This is a mandatory syntactic rule.

---

## Chapter 3: Types

### 3.1 Type Classification

```
TypeExpr    ::= PrimitiveType
              | StructType
              | EnumType
              | InterfaceType
              | TupleType
              | FnType
              | GenericType
              | TypeRef
              | TypeUnion
              | TypeIntersection
              | ConstrainedType
              | AssociatedType
```

### 3.2 Primitive Types

| Type | Description | Default Size |
|------|-------------|--------------|
| `Type` | Meta type | 0 bytes |
| `Void` | void value | 0 bytes |
| `Bool` | Boolean value | 1 byte |
| `Int` | Signed integer | 8 bytes |
| `Uint` | Unsigned integer | 8 bytes |
| `Float` | Floating-point number | 8 bytes |
| `String` | UTF-8 string | Variable |
| `Char` | Unicode character | 4 bytes |
| `Bytes` | Raw bytes | Variable |

Width-specified integers: `Int8`, `Int16`, `Int32`, `Int64`, `Int128`
Width-specified floating-point: `Float32`, `Float64`

### 3.3 Record Types

**Unified syntax**: `Name: Type = { field1: Type1, field2: Type2, ... }`

```
RecordType  ::= '{' FieldList? '}'
FieldList   ::= Field (',' Field)* ','?
Field       ::= Identifier ':' TypeExpr
            |  Identifier                 # Interface constraint
```

```yaoxiang
// Simple record type
Point: Type = { x: Float, y: Float }

// Empty record type
Empty: Type = {}

// Record type with generics
Pair: Type[T] = { first: T, second: T }

// Record type implementing interfaces
Point: Type = {
    x: Float,
    y: Float,
    Drawable,
    Serializable
}
```

**Rules**:
- Record types are defined using curly braces `{}`
- Field names are followed directly by a colon and type
- Interface names written within the type body indicate implementation of those interfaces

#### 3.3.1 Field Default Values

Type fields can specify default values, which are optional during construction:

```yaoxiang
// Fields with default values - optional during construction
Point: Type = {
    x: Float = 0,
    y: Float = 0
}

// Usage
Point()           # → Point(x=0, y=0)
Point(x=1)       # → Point(x=1, y=0)
Point(x=1, y=2) # → Point(x=1, y=2)

// Fields without default values - required during construction
Point2: Type = {
    x: Float,
    y: Float
}

// Usage
Point2(x=1, y=2) # ✓
Point2()          # ✗ Error
```

**Rules**:
- `field: Type = expression` → Has default value, optional during construction
- `field: Type` → No default value, required during construction

#### 3.3.2 Builtin Bindings

Methods can be directly bound within type definitions:

```yaoxiang
// Method 1: Reference external function binding
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]    # Bind to position 0
}
// Call: p1.distance(p2) → distance(p1, p2)

// Method 2: Anonymous function + position binding
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance: ((a: Point, b: Point) -> Float)[0] = ((a, b) => {
        dx = a.x - b.x
        dy = a.y - b.y
        return (dx * dx + dy * dy).sqrt()
    })
}
// Syntax: ((params) => body)[position]
// Call: p1.distance(p2) → distance(p1, p2)
```

### 3.4 Enum Types (Variant Types)

```
EnumType    ::= '{' Variant ('|' Variant)* '}'
Variant     ::= Identifier (':' TypeExpr)?
```

**Syntax**: `Name: Type = { Variant1 | Variant2(params) | ... }`

```yaoxiang
// Variants without parameters
Color: Type = { red | green | blue }

// Variants with parameters
Option: Type[T] = { some(T) | none }

// Mixed
Result: Type[T, E] = { ok(T) | err(E) }

// Variants without parameters are equivalent to parameterless constructors
Bool: Type = { true | false }
```

### 3.5 Interface Types

```
InterfaceType ::= '{' FnField (',' FnField)* ','?
FnField       ::= Identifier ':' FnType
FnType        ::= '(' ParamTypes? ')' '->' TypeExpr
```

**Syntax**: Interfaces are record types with all fields being function types

```yaoxiang
// Interface definition
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

// Empty interface
EmptyInterface: Type = {}
```

**Interface Implementation**: Types implement interfaces by listing interface names at the end of their definition

```yaoxiang
// Type implementing interfaces
Point: Type = {
    x: Float,
    y: Float,
    Drawable,        # Implement Drawable interface
    Serializable     # Implement Serializable interface
}
```

**Direct Assignment to Interface Types**: Concrete types can be directly assigned to interface type variables (structural subtyping)

```yaoxiang
// Direct assignment (concrete type determined at compile-time → zero-cost call)
d: Drawable = Circle(1)
d.draw(screen)        # After compilation: direct call to circle_draw, no vtable

// Function return value (concrete type unknown at compile-time → vtable call)
d: Drawable = get_shape()
d.draw(screen)        # Method lookup through vtable

// Interface as function parameter
process: (d: Drawable) -> Void = d.draw(screen)
```

**Compile-time Optimization Strategy**:

| Scenario | Inferred Result | Call Method |
|----------|-----------------|-------------|
| Direct assignment of concrete type | Concrete type determinable | Direct call (zero overhead) |
| Function return value | Unknown | vtable |
| Heterogeneous collection | Multiple types | vtable |

### 3.6 Tuple Types

```
TupleType   ::= '(' TypeList? ')'
TypeList    ::= TypeExpr (',' TypeExpr)* ','?
```

### 3.7 Function Types

```
FnType      ::= '(' ParamList? ')' '->' TypeExpr
ParamList   ::= TypeExpr (',' TypeExpr)*
```

### 3.8 Generic Types

#### 3.8.1 Generic Parameter Syntax

```
GenericType     ::= Identifier '[' TypeArgList ']'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
GenericParams   ::= '[' Identifier (',' Identifier)* ']'
                 |  '[' Identifier ':' TypeBound (',' Identifier ':' TypeBound)* ']'
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

#### 3.8.2 Generic Type Definitions

```yaoxiang
// Basic generic type
Option: Type[T] = {
    some: (T) -> Self,
    none: () -> Self
}

Result: Type[T, E] = {
    ok: (T) -> Self,
    err: (E) -> Self
}

List: Type[T] = {
    data: Array[T],
    length: Int,
    push: [T](self: List[T], item: T) -> Void,
    get: [T](self: List[T], index: Int) -> Option[T]
}
```

#### 3.8.3 Type Inference

```yaoxiang
// Compiler automatically infers generic parameters
numbers: List[Int] = List(1, 2, 3)  # Compiler infers List[Int]
```

### 3.9 Type Constraints

#### 3.9.1 Single Constraint

```
ConstrainedType ::= '[' Identifier ':' TypeBound ']' TypeExpr
```

```yaoxiang
// Interface type definition (as constraint)
Clone: Type = {
    clone: (Self) -> Self
}

// Using constraint
clone: [T: Clone](value: T) -> T = value.clone()
```

#### 3.9.2 Multiple Constraints

```yaoxiang
// Multiple constraint syntax
combine: [T: Clone + Add](a: T, b: T) -> T = {
    a.clone() + b
}

// Sorting generic containers
sort: [T: Clone + PartialOrd](list: List[T]) -> List[T] = {
    result = list.clone()
    quicksort(&mut result)
    return result
}
```

#### 3.9.3 Function Type Constraints

```yaoxiang
// Higher-order function constraints
call_twice: [T, F: Fn() -> T](f: F) -> (T, T) = (f(), f())

compose: [A, B, C, F: Fn(A) -> B, G: Fn(B) -> C](a: A, f: F, g: G) -> C = g(f(a))
```

### 3.10 Associated Types

#### 3.10.1 Associated Type Definition

```
AssociatedType ::= Identifier ':' TypeExpr
```

```yaoxiang
// Iterator trait (using record type syntax)
Iterator: Type[T] = {
    Item: T,                    # Associated type
    next: (Self) -> Option[T],
    has_next: (Self) -> Bool
}

// Using associated types
collect: [T, I: Iterator[T]](iter: I) -> List[T] = {
    result = List[T]()
    while iter.has_next() {
        if let Some(item) = iter.next() {
            result.push(item)
        }
    }
    return result
}
```

#### 3.10.2 Generic Associated Types (GAT)

```yaoxiang
// More complex associated types
Container: Type[T] = {
    Item: T,
    IteratorType: Iterator[T],  # Associated type is also generic
    iter: (Self) -> IteratorType
}
```

### 3.11 Compile-Time Generics

#### 3.11.1 Literal Type Constraints

```
LiteralType   ::= Identifier ':' Int          # Compile-time constant
CompileTimeFn ::= '[' Identifier ':' Int ']' '(' Identifier ')' '->' TypeExpr
```

**Core Design**: Use `[n: Int]` generic parameter + `(n: n)` value parameter to distinguish compile-time constants from runtime values.

```yaoxiang
// Compile-time factorial: parameter must be a literal known at compile-time
factorial: [n: Int](n: n) -> Int = {
    match n {
        0 => 1,
        _ => n * factorial(n - 1)
    }
}

// Compile-time constant array
StaticArray: Type[T, N: Int] = {
    data: T[N],      # Array with compile-time known size
    length: N
}

// Usage
arr: StaticArray[Int, factorial(5)]  # Compiler computes factorial(5) = 120 at compile-time
```

#### 3.11.2 Compile-Time Constant Arrays

```yaoxiang
// Matrix type usage
Matrix: Type[T, Rows: Int, Cols: Int] = {
    data: Array[Array[T, Cols], Rows]
}

// Compile-time dimension verification
identity_matrix: [T: Add + Zero + One, N: Int](size: N) -> Matrix[T, N, N] = {
    // ...
}
```

### 3.12 Conditional Types

#### 3.12.1 If Conditional Types

```
IfType        ::= 'If' '[' BoolExpr ',' TypeExpr ',' TypeExpr ']'
```

```yaoxiang
// Type-level If
If: Type[C: Bool, T, E] = match C {
    True => T,
    False => E
}

// Example: Compile-time branching
NonEmpty: Type[T] = If[T != Void, T, Never]

// Compile-time assertion
Assert: Type[C: Bool] = match C {
    True => Void,
    False => compile_error("Assertion failed")
}
```

#### 3.12.2 Type Families

```yaoxiang
// Compile-time type conversion
AsString: Type[T] = match T {
    Int => String,
    Float => String,
    Bool => String,
    _ => String
}
```

### 3.13 Type Union

```
TypeUnion     ::= TypeExpr '|' TypeExpr
```

### 3.14 Type Intersection

```
TypeIntersection ::= TypeExpr '&' TypeExpr
```

**Syntax**: Type intersection `A & B` represents types that satisfy both A and B

```yaoxiang
// Interface composition = Type intersection
DrawableSerializable: Type = Drawable & Serializable

// Using intersection type
process: [T: Drawable & Serializable](item: T, screen: Surface) -> String = {
    item.draw(screen)
    return item.serialize()
}
```

### 3.15 Function Overloading and Specialization

```yaoxiang
// Basic specialization: Using function overloading (compiler auto-selects)
sum: (arr: Array[Int]) -> Int = {
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Array[Float]) -> Float = {
    return simd_sum_float(arr.data, arr.length)
}

// Generic implementation
sum: [T: Add](arr: Array[T]) -> T = {
    result = Zero::zero()
    for item in arr {
        result = result + item
    }
    return result
}
```

### 3.16 Platform Specialization

```yaoxiang
// Platform type enum (defined in standard library)
Platform: Type = X86_64 | AArch64 | RISC_V | ARM | X86

// P is the predefined generic parameter name representing the current compilation platform
sum: [P: X86_64](arr: Array[Float]) -> Float = {
    return avx2_sum(arr.data, arr.length)
}

sum: [P: AArch64](arr: Array[Float]) -> Float = {
    return neon_sum(arr.data, arr.length)
}
```

---

## Chapter 3 (Continued): Syntax Design Notes

### 3.17 Relationship Between Named Functions and Lambdas

**Core Understanding**: Named functions and Lambda expressions are the same thing! The only difference is that named functions give a Lambda a name.

```yaoxiang
// These two are essentially identical
add: (a: Int, b: Int) -> Int = a + b           # Named function (recommended)
add: (a: Int, b: Int) -> Int = (a, b) => a + b  # Lambda form (completely equivalent)
```

**Syntactic Sugar Model**:

```
// Named function = Lambda + name
name: (Params) -> ReturnType = body

// Essentially
name: (Params) -> ReturnType = (params) => body
```

**Key Point**: When the signature fully declares parameter types, the parameter names in the Lambda header become redundant and can be omitted.

### 3.18 Parameter Scope Rules

**Parameters override outer variables**: The parameter scope in the signature overrides the function body, with inner scope having higher priority.

```yaoxiang
x = 10  # Outer variable
double: (x: Int) -> Int = x * 2  # ✅ Parameter x overrides outer x, result is 20
```

### 3.19 Type Annotation Placement

Type annotations can be in any of the following positions, **at least one position must be annotated**:

| Annotation Position | Form | Description |
|---------------------|------|-------------|
| Signature only | `double: (x: Int) -> Int = x * 2` | ✅ Recommended |
| Lambda header only | `double = (x: Int) => x * 2` | ✅ Valid |
| Both | `double: (x: Int) -> Int = (x: Int) => x * 2` | ✅ Redundant but allowed |

### 4.1 Expression Classification

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

### 4.2 Operator Precedence

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

### 4.3 Function Call

```
FnCall      ::= Expr '(' ArgList? ')'
ArgList     ::= Expr (',' Expr)* (',' NamedArg)* | NamedArg (',' NamedArg)*
NamedArg    ::= Identifier ':' Expr
```

### 4.4 Member Access

```
MemberAccess::= Expr '.' Identifier
```

### 4.5 Index Access

```
IndexAccess ::= Expr '[' Expr ']'
```

### 4.6 Type Cast

```
TypeCast    ::= Expr 'as' TypeExpr
```

### 4.7 Conditional Expression

```
IfExpr      ::= 'if' Expr Block ('elif' Expr Block)* ('else' Block)?
```

### 4.8 Pattern Matching

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

### 4.9 Block Expression

```
Block       ::= '{' Stmt* Expr? '}'
```

### 4.10 Lambda Expression

```
Lambda      ::= '(' ParamList? ')' '=>' Expr
            |  '(' ParamList? ')' '=>' Block
```

---

## Chapter 5: Statements

### 5.1 Statement Classification

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

### 5.2 Variable Declaration

```
LetStmt     ::= ('mut')? Identifier (':' TypeExpr)? '=' Expr
```

### 5.3 return Statement

```
ReturnStmt  ::= 'return' Expr?
```

### 5.4 break Statement

```
BreakStmt   ::= 'break' Identifier?
```

### 5.5 continue Statement

```
ContinueStmt::= 'continue'
```

### 5.6 if Statement

```
IfStmt      ::= 'if' Expr Block ('elif' Expr Block)* ('else' Block)?
```

### 5.7 match Statement

```
MatchStmt   ::= 'match' Expr '{' MatchArm+ '}'
```

### 5.8 while Statement

```
WhileStmt   ::= 'while' Expr Block
```

### 5.9 for Statement

```
ForStmt     ::= 'for' 'mut'? Identifier 'in' Expr Block
```

#### 5.9.1 Semantics: Each Iteration Creates a New Binding

YaoXiang's for loop semantics differ from traditional languages: **each iteration creates a new binding, rather than modifying the same variable**.

```yaoxiang
// Example: for i in 1..5
for i in 1..5 {
    print(i)
}
```

**Execution Process**:

| Iteration | Loop Variable Behavior |
|-----------|------------------------|
| 1st | Create new binding `i = 1`, execute loop body, print 1 |
| 2nd | Create new binding `i = 2` (previous binding destroyed), execute loop body, print 2 |
| 3rd | Create new binding `i = 3`, execute loop body, print 3 |
| 4th | Create new binding `i = 4`, execute loop body, print 4 |
| End | Loop body ends, binding destroyed |

**Key Point**: At the end of each iteration, the binding created during that iteration is destroyed. The next iteration is a completely new binding with no relation to the previous iteration's binding.

#### 5.9.2 Difference Between for and for mut

| Syntax | Loop Variable Mutability | Description |
|--------|--------------------------|-------------|
| `for i in 1..5` | Immutable | Cannot modify binding within loop body |
| `for mut i in 1..5` | Mutable | Can modify binding within loop body |

```yaoxiang
// ✅ Valid: Each iteration binds a new value, no modification needed
for i in 1..5 {
    print(i)  # Read value of i
}

// ❌ Error: Immutable binding, cannot modify
for i in 1..5 {
    i = i + 1  # Error: cannot modify immutable binding
}

// ✅ Valid: Using for mut allows modification
for mut i in 1..5 {
    i = i + 1  # Allowed
}
```

#### 5.9.3 Shadowing Check

for loop variables cannot shadow existing variables in the outer scope:

```yaoxiang
// ❌ Error: i already declared outside
i = 10
for i in 1..5 {
    print(i)
}

// ✅ Correct: Use different variable name
i = 10
for j in 1..5 {
    print(j)
}
```

Error code: `E2013 - Cannot shadow existing variable`

#### 5.9.4 Comparison with Other Languages

| Language | for Loop Variable Semantics |
|----------|-----------------------------|
| YaoXiang | Each iteration creates a new binding |
| Rust | Modifies the same variable (requires mut) |
| Python | Modifies the same variable (no mut needed) |
| C/C++ | Modifies the same variable (requires pointer or reference) |

**Design Rationale**: YaoXiang uses binding semantics because:
1. Variables within the loop body are destroyed at the end of each iteration
2. Each subsequent iteration is a completely new binding
3. This is safer; no need to consider state between iterations

---

## Chapter 6: Functions

### 6.1 Unified Function Model

**Core Syntax**: `name: type = value`

YaoXiang uses a **unified declaration model**: variables, functions, and methods all use the same form `name: type = value`.

```
Declaration   ::= Identifier ':' Type '=' Expression
FunctionDef   ::= Identifier GenericParams? '(' Parameters? ')' '->' Type '=' (Expression | Block)
GenericParams ::= '[' Identifier (',' Identifier)* ']'
Parameters    ::= Parameter (',' Parameter)*
Parameter     ::= Identifier ':' TypeExpr
```

### 6.2 Variable Declaration

```yaoxiang
// Basic syntax
x: Int = 42
name: String = "YaoXiang"
mut counter: Int = 0

// Type inference
y = 100  # Inferred as Int
```

### 6.3 Function Definition

#### 6.3.1 Complete Syntax

```yaoxiang
// Parameter names declared in signature
add: (a: Int, b: Int) -> Int = {
    return a + b
}

// Single parameter
inc: (x: Int) -> Int = x + 1

// No-parameter function
main: () -> Void = {
    print("Hello")
}

// Multi-line function body
calc: (x: Float, y: Float, op: String) -> Float = {
    return match op {
        "+" => x + y,
        "-" => x - y,
        _ => 0.0
    }
}
```

#### 6.3.2 Return Rules

```yaoxiang
// Non-Void return type - must use return
add: (a: Int, b: Int) -> Int = {
    return a + b
}

// Void return type - return optional
print: (msg: String) -> Void = {
    // No return needed
}

// Single-line expression - return value directly, no return needed
greet: (name: String) -> String = "Hello, ${name}!"
```

### 6.4 Generic Functions

```yaoxiang
// Generic function definition
map: [T, R](list: List[T], f: Fn(T) -> R) -> List[R] = {
    result = List[R]()
    for item in list {
        result.push(f(item))
    }
    return result
}

// Using generic constraints
clone: [T: Clone](value: T) -> T = value.clone()

// Multiple type parameters
combine: [T, U](a: T, b: U) -> (T, U) = (a, b)
```

### 6.5 Method Definition

#### 6.5.1 Type Methods

**Syntax**: `Type.method: (self: Type, ...) -> Return = ...`

```yaoxiang
// Type method: associated with specific type
Point.draw: (self: Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}

Point.serialize: (self: Point) -> String = {
    return "Point(${self.x}, ${self.y})"
}

// Using method syntax sugar
p: Point = Point(1.0, 2.0)
p.draw(screen)           # Syntax sugar → Point.draw(p, screen)
```

#### 6.5.2 Regular Methods

**Syntax**: `name: (Type, ...) -> Return = ...` (not associated with a type)

```yaoxiang
// Regular method: not associated with type, acts as independent function
distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}
```

### 6.6 Method Binding

#### 6.6.1 Manual Binding

**Syntax**: `Type.method = function[positions]`

```yaoxiang
// Bind to position 0 (default)
Point.distance = distance[0]

// Bind to position 1
Point.transform = transform[1]

// Multi-position binding
Point.scale = scale[0, 1]

// Using placeholder
Point.calc = func[0, _, 2]
```

#### 6.6.2 pub Auto-Binding

Functions declared with `pub` are automatically bound by the compiler to types defined in the same file:

```yaoxiang
// Using pub declaration, compiler auto-binds
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

// Compiler auto-infers:
// 1. Point is defined in the current file
// 2. Function parameters include Point
// 3. Executes Point.distance = distance[0]

// Invocation
d = distance(p1, p2)           # Functional style
d2 = p1.distance(p2)           # OOP syntax sugar
```

### 6.7 Method Binding Rules

| Rule | Description |
|------|-------------|
| Positions start at 0 | `func[0]` binds to the 1st parameter (index 0) |
| Maximum position | Must be < number of function parameters |
| Negative index | `[-1]` means the last parameter |
| Placeholder | `_` skips that position, provided by user |

### 6.8 Currying Support

Binding naturally supports currying. When a call provides fewer arguments than remaining parameters, it returns a function that accepts the remaining parameters:

```yaoxiang
// Original function: 5 parameters
calculate: (scale: Float, a: Point, b: Point, x: Float, y: Float) -> Float = ...

// Binding: Point.calc = calculate[1, 2]
// Remaining parameters after binding: scale, x, y

// Call scenarios
p1.calc(2.0, 10.0, 20.0)       # Provide 3 arguments → direct call
p1.calc(2.0)                    # Provide 1 argument → returns (Float, Float) -> Float
p1.calc()                       # Provide 0 arguments → returns (Float, Float, Float) -> Float
```

### 6.9 spawn Functions and Annotations

#### 6.9.1 spawn Functions (Concurrent Functions)

```
SpawnFn     ::= Identifier ':' FnType 'spawn' '=' Lambda
FnType      ::= '(' ParamTypes? ')' '->' TypeExpr ('@' Annotation)?
Annotation  ::= 'block' | 'eager'
```

**Function Annotations**:

| Annotation | Position | Behavior |
|------------|----------|----------|
| `@block` | After return type | Disable concurrent optimization, fully sequential execution |
| `@eager` | After return type | Force eager evaluation |

**Syntax Examples**:

```
// spawn function: can execute concurrently
fetch_data: (url: String) -> JSON spawn = { ... }

// @block synchronous function: fully sequential execution
main: () -> Void @block = { ... }

// @eager eager function: executes immediately
compute: (n: Int) -> Int @eager = { ... }
```

#### 6.9.2 spawn Blocks

Explicitly declared concurrency domains where tasks within the block execute concurrently:

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
```

**Example**:

```
// spawn block: explicit concurrency
(result_a, result_b) = spawn {
    parse(fetch("url1")),
    parse(fetch("url2"))
}
```

#### 6.9.3 spawn Loops

Data-parallel loops where the loop body executes concurrently on all data elements:

```
SpawnFor    ::= Identifier '=' 'spawn' 'for' Identifier 'in' Expr '{' Expr '}'
```

**Example**:

```
// spawn loop: data parallelism
results = spawn for item in items {
    process(item)
}
```

#### 6.9.4 Error Propagation Operator

```
ErrorPropagate ::= Expr '?'
```

**Example**:

```
process: (p: Point) -> Result[Data, Error] = {
    data = fetch_data()?      # Automatically propagate error
    transform(data)?
}
```

---

## Chapter 7: Modules

### 7.1 Module Definition

Modules use files as boundaries. Each `.yx` file is a module.

```
// File name is the module name
// Math.yx
pub pi: Float = 3.14159
pub sqrt(Float) -> Float = (x) => { ... }
```

### 7.2 Module Import

```
Import       ::= 'use' ModuleRef ImportSpec?
ImportSpec   ::= ('{' ImportItems '}') ('as' AliasList)?
              |  'as' AliasList
ImportItems  ::= Identifier (',' Identifier)* ','?
AliasList    ::= Identifier (',' Identifier)*
```

| Syntax | Description | Example |
|--------|-------------|---------|
| `use path;` | Import module, access via last part | `use std.io;` → `io.print` |
| `use path.{a, b};` | Import specified items | `use std.io.{print};` → `print` |
| `use path as alias;` | Import and rename | `use std.io as io;` → `io.print` |
| `use path.{i1, i2} as a, b;` | Import specified items and rename | `use std.io.{print, read} as p, r;` → `p`, `r` |

---

## Chapter 8: Memory Management

### 8.1 Ownership Model

YaoXiang uses an **ownership model** for memory management, where each value has a unique owner:

| Semantics | Description | Syntax |
|-----------|-------------|--------|
| **Move** | Default semantics, ownership transfer | `p2 = p` |
| **ref** | Shared (Arc reference counting) | `shared = ref p` |
| **clone()** | Explicit copy | `p2 = p.clone()` |

### 8.2 Move Semantics (Default)

```yaoxiang
// Assignment = Move (zero copy)
p: Point = Point(1.0, 2.0)
p2 = p              # Move, p is invalidated

// Function parameter passing = Move
process: (p: Point) -> Void = {
    // Ownership of p transferred in
}

// Return value = Move
create: () -> Point = {
    p = Point(1.0, 2.0)
    return p        # Move, ownership transferred out
}
```

### 8.3 ref Keyword (Arc)

The `ref` keyword creates a **reference-counted pointer** (Arc) for safe sharing:

```yaoxiang
// Create Arc
p: Point = Point(1.0, 2.0)
shared = ref p      # Arc, thread-safe

// Shared access
spawn(() => print(shared.x))   # ✅ Safe

// Arc automatically manages lifetime
// When shared goes out of scope, count reaches zero and is automatically freed
```

**Characteristics**:
- Thread-safe reference counting
- Automatic lifetime management
- Safe across spawn boundaries

### 8.4 clone() Explicit Copy

```yaoxiang
// Explicitly copy a value
p: Point = Point(1.0, 2.0)
p2 = p.clone()      # p and p2 are independent

// Both can be modified without affecting each other
p.x = 0.0           # ✅
p2.x = 0.0          # ✅
```

### 8.5 unsafe Code Blocks

`unsafe` code blocks allow the use of raw pointers for systems-level programming:

```yaoxiang
// Raw pointer type
PtrType ::= '*' TypeExpr

// unsafe code block
UnsafeBlock ::= 'unsafe' '{' Stmt* '}'
```

**Example**:

```yaoxiang
p: Point = Point(1.0, 2.0)

// Raw pointers can only be used in unsafe blocks
unsafe {
    ptr: *Point = &p     # Get raw pointer
    (*ptr).x = 0.0       # Dereference
}
```

**Restrictions**:
- Raw pointers can only be used in `unsafe` blocks
- User guarantees no dangling or use-after-free
- Not subject to Send/Sync checks

### 8.7 Ownership Syntax BNF

```bnf
// === Ownership Expressions ===

// Move (default)
MoveExpr     ::= Expr

// ref Arc
RefExpr      ::= 'ref' Expr

// clone
CloneExpr    ::= Expr '.clone' '(' ')'

// === Raw Pointers (unsafe only) ===

PtrType       ::= '*' TypeExpr
UnsafeBlock   ::= 'unsafe' '{' Stmt* '}'
```

### 8.8 Send / Sync Constraints

| Constraint | Semantics | Description |
|------------|-----------|-------------|
| **Send** | Can be safely transferred across threads | Value can be moved to another thread |
| **Sync** | Can be safely shared across threads | Immutable reference can be shared to another thread |

**Auto-derivation**:

```
// Send derivation rules
Struct[T1, T2]: Send ⇐ T1: Send 且 T2: Send

// Sync derivation rules
Struct[T1, T2]: Sync ⇐ T1: Sync 且 T2: Sync
```

**Type Constraints**:

| Type | Send | Sync | Description |
|------|------|------|-------------|
| `T` (value) | ✅ | ✅ | Immutable data |
| `ref T` | ✅ | ✅ | Arc thread-safe |
| `*T` | ❌ | ❌ | Raw pointer unsafe |

---

## Chapter 8 (Continued): Type System Constraints

### 8.7 Send/Sync Constraints

YaoXiang uses Rust-style type constraints to ensure concurrency safety:

| Constraint | Semantics | Description |
|------------|-----------|-------------|
| **Send** | Can be safely transferred across threads | Value can be moved to another thread |
| **Sync** | Can be safely shared across threads | Immutable reference can be shared to another thread |

**Constraint Hierarchy**:

```
Send ──► Can be safely transferred across threads
  │
  └──► Sync ──► Can be safely shared across threads
       │
       └──► Types satisfying Send + Sync can auto-concurrent

Arc[T] implements Send + Sync (thread-safe reference counting)
Mutex[T] provides interior mutability
```

### 8.8 Concurrency-Safe Types

| Type | Semantics | Concurrency Safe | Description |
|------|-----------|------------------|-------------|
| `T` | Immutable data | ✅ Safe | Default type, multi-task reads without race |
| `Ref[T]` | Mutable reference | ⚠️ Needs synchronization | Marked for concurrent modification, compiler checks lock usage |
| `Atomic[T]` | Atomic type | ✅ Safe | Low-level atomic operations, lock-free concurrency |
| `Mutex[T]` | Mutex wrapper | ✅ Safe | Auto lock/unlock, compiler-guaranteed |
| `RwLock[T]` | Read-write lock wrapper | ✅ Safe | Optimized for read-heavy, write-light scenarios |

**Syntax**:

```
Mutex[T]    # Mutex-wrapped mutable data
Atomic[T]   # Atomic type (only for Int, Float, etc.)
RwLock[T]   # Read-write lock wrapper
```

**with Syntax Sugar**:

```
with mutex.lock() {
    // Critical section: protected by Mutex
    ...
}
```

---

## Chapter 9: Error Handling

### 9.1 Result Type

```
Result: Type[T, E] = ok(T) | err(E)
```

**Variant Construction**:

| Variant | Syntax | Description |
|---------|--------|-------------|
| `ok(T)` | `ok(value)` | Success value |
| `err(E)` | `err(error)` | Error value |

### 9.2 Option Type

```
Option: Type[T] = some(T) | none
```

**Variant Construction**:

| Variant | Syntax | Description |
|---------|--------|-------------|
| `some(T)` | `some(value)` | Has value |
| `none` | `none` | No value |

### 9.3 Error Propagation

```
ErrorPropagate ::= Expr '?'
```

The `?` operator automatically propagates errors of Result types:

```
// Returns value on success, returns err upward on failure
data = fetch_data()?

// Equivalent to
data = match fetch_data() {
    ok(v) => v
    err(e) => return err(e)
}
```

---

## Appendix A: Syntax Quick Reference

### A.1 Type Definitions

```
// === Record Types (curly braces) ===

// Struct
Point: Type = { x: Float, y: Float }

// Enum (variant types)
Result: Type[T, E] = { ok(T) | err(E) }
Status: Type = { pending | processing | completed }

// === Interface Types (curly braces, all fields are functions) ===

// Interface definition
Serializable: Type = { serialize: () -> String }

// Type implementing interface
Point: Type = {
    x: Float,
    y: Float,
    Serializable    # Implement Serializable interface
}

// === Function Types ===

Adder: Type = (Int, Int) -> Int
```

### A.2 Function Definitions

```
// Form 1: Type-centralized (recommended)
name: (param1: Type1, param2: Type2) -> ReturnType = body

// Form 2: Abbreviated (parameter names omitted)
name: (Type1, Type2) -> ReturnType = (params) => body

// Generic function
name: [T, R](param: T) -> R = body

// Generic constraint
name: [T: Clone + Add](a: T, b: T) -> T = body
```

### A.3 Method Definitions

```
// Type method
Type.method: (self: Type, ...) -> ReturnType = { ... }

// Regular method
name: (Type, ...) -> ReturnType = { ... }
```

### A.4 Method Binding

```
// Single-position binding
Type.method = func[0]

// Multi-position binding
Type.method = func[0, 1]

// pub auto-binding
pub name: (Type, ...) -> ReturnType = { ... }  # Auto-bind to Type
```

### A.5 Generic Syntax

```
// Generic type
List: Type[T] = { data: Array[T], length: Int }
Result: Type[T, E] = { ok(T) | err(E) }

// Generic function
map: [T, R](list: List[T], f: Fn(T) -> R) -> List[R] = { ... }

// Type constraint
clone: [T: Clone](value: T) -> T = value.clone()
combine: [T: Clone + Add](a: T, b: T) -> T = body

// Associated type
Iterator: Type[T] = { Item: T, next: () -> Option[T] }

// Compile-time generic
factorial: [n: Int](n: n) -> Int = { ... }
StaticArray: Type[T, N: Int] = { data: T[N], length: N }

// Conditional type
If: Type[C: Bool, T, E] = match C { True => T, False => E }

// Function specialization
sum: (arr: Array[Int]) -> Int = { ... }
sum: (arr: Array[Float]) -> Float = { ... }
```

### A.6 Modules

```
// Module is a file
// FileName.yx is the module name
Import ::= 'use' ModuleRef
```

### A.7 Control Flow

```
if Expr Block (elif Expr Block)* (else Block)?
match Expr { MatchArm+ }
while Identifier in Expr Block Expr Block
for
```

### A.8 match Syntax

```
match value {
    pattern1 => expr1,
    pattern2 if guard => expr2,
    _ => default_expr,
}
```

---

## Appendix B: Differences from Code Implementation

> This section explains known differences between the language specification and the current code implementation.

### B.1 Keywords

| Keyword | Specification Status | Code Implementation | Description |
|---------|---------------------|-------------------|-------------|
| `struct` | Removed | ❌ None | Uses unified syntax `Name: Type = {...}` |
| `enum` | Removed | ❌ None | Uses variant syntax `Name: Type = { A \| B \| C }` |
| `type` | Removed | ❌ None | Uses `Type` (uppercase) as meta type keyword |

### B.2 Syntax Differences

| Syntax Element | Specification | Code Implementation | Description |
|----------------|---------------|---------------------|-------------|
| match arm separator | `->` | `=>` | Uses `=>` (FatArrow) |
| Function definition | `name(types) -> type = (params) => body` | Both forms | Supports type-centralized `name: Type = (params) =>` |
| Interface type | `type Serializable = [ serialize() -> String ]` | ❌ Not implemented | Square bracket syntax pending implementation |

### B.3 Unimplemented Features

The following features described in the specification have not yet been implemented in code:

| Feature | Priority | Description |
|---------|----------|-------------|
| Unified type syntax `Name: Type = {...}` | P0 | RFC-010: Unified syntax replaces `type Name = ...` |
| Curly brace type syntax | P0 | `Point: Type = { x: Float, y: Float }` |
| Interface types | P1 | `Drawable: Type = { draw() -> Void }` |
| List comprehensions | P2 | `[x for x in list if condition]` |
| `?` error propagation | P1 | Result type automatic error propagation |
| `ref` keyword | P1 | Arc reference counting sharing |
| `unsafe` code blocks | P1 | Raw pointers and systems-level programming |
| `*T` raw pointer type | P1 | Raw pointer type syntax |
| `clone()` semantics | P1 | Explicit copy |
| `@block` annotation | P1 | Synchronous execution guarantee |
| `spawn` function | P1 | spawn function marker |
| `spawn {}` block | P1 | Explicit concurrency domain |
| `spawn for` loop | P1 | Data parallel loop |
| Send/Sync constraints | P2 | Concurrent safety type checking |
| Mutex/Atomic types | P2 | Concurrent safety data types |
| Error graph visualization | P3 | Concurrent error propagation tracking |
| **Generic type system** | P1 | RFC-011 |
| Basic generics `[T]` | P1 | Generic type parameters and monomorphization |
| Type constraints `[T: Clone]` | P2 | Single/multiple constraint system |
| Associated types `Item: T` | P3 | GAT support |
| Compile-time generics `[N: Int]` | P3 | Literal type constraints |
| Conditional types `If[C, T, E]` | P3 | Type-level computation |
| Function overload specialization | P2 | Platform specialization and type specialization |
| Method syntax `Type.method` | P1 | `Point.draw: (...) -> ... = ...` |

### B.4 Features Not to Be Implemented

The following Rust-style features **will not be implemented**:

| Feature | Reason |
|---------|--------|
| Lifetimes `'a` | No reference concept, no lifetimes needed |
| Borrow checker | ref = Arc instead |
| `&T` borrow syntax | Uses Move semantics |
| `&mut T` mutable borrow | Uses mut + Move |

---

## Chapter 10: Method Binding

### 10.1 Binding Overview

YaoXiang uses a **purely functional design**, where all operations are implemented through functions. The binding mechanism associates functions with types, allowing callers to invoke functions as if calling methods.

```
Binding Declaration ::= Type '.' Identifier '=' FunctionName '[' PositionList ']'
PositionList        ::= Position (',' Position)* ','?
Position            ::= Integer (starting from 0) | Negative Integer | Placeholder
```

**Core Rules**:
- Position indices start at **0**
- Default binding to position **0** (first argument)
- Supports negative index `[-1]` for the last parameter
- Multi-position union binding `[0, 1, 2]`
- Placeholder `_` means skip that position

### 10.2 Binding Syntax

**Syntax**:
```
Type.method = func[position]
Type.method = func[0, 1, 2]    # Multi-position binding
Type.method = func[0, _, 2]   # Using placeholder
Type.method = func[-1]        # Negative index (last parameter)
```

**Semantics**:
- `Type.method = func[0]` means when calling `obj.method(arg)`, `obj` binds to `func`'s 0th parameter
- Remaining parameters are filled in original order

### 10.3 Binding Examples

```yaoxiang
// === Basic Binding ===

// Original function
distance: (a: Point, b: Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    (dx * dx + dy * dy).sqrt()
}

// Bind to Point type (position 0)
Point.distance = distance[0]

// Usage
p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)
d = p1.distance(p2)  # → distance(p1, p2)

// === Multi-Position Binding ===

// Original function
calculate: (scale: Float, a: Point, b: Point, x: Float, y: Float) -> Float = ...

// Bind multiple positions
Point.calc_scale = calculate[0]      # Bind only scale
Point.calc_both = calculate[1, 2]    # Bind two Point parameters

// Usage
f = p1.calc_scale(2.0)  # → calculate(2.0, p1, _, _, _)
result = f(p2, 10.0, 20.0)  # → calculate(2.0, p1, p2, 10.0, 20.0)

// === Currying (auto-return function when arguments insufficient) ===

// Bind one parameter
Point.distance_to = distance[0]

// Usage - not providing second parameter returns curried function
f = p1.distance_to(p2)  # → distance(p1, p2) direct call
f2 = p1.distance_to()   # → distance(p1, _) returns function (Point) -> Float
result = f2(p2)         # → distance(p1, p2)
```

### 10.4 Binding Rules

**Position Rules**:
| Rule | Description |
|------|-------------|
| Positions start at 0 | `func[0]` binds to the 1st parameter (index 0) |
| Maximum position | Must be < number of function parameters |
| Negative index | `[-1]` means the last parameter |
| Placeholder | `_` skips that position, provided by user |

**Type Checking**:
```yaoxiang
// ✅ Valid binding
Point.distance = distance[0]          # distance(Point, Point)
Point.calc = calculate[1, 2]          # calculate(Float, Point, Point, ...)

// ❌ Invalid binding (compile error)
Point.wrong = distance[5]             # 5 >= 2 (parameter count)
Point.wrong = distance[0, 0]          # Duplicate position (if not allowed)
Point.wrong = distance[-2]            # -2 out of range
```

### 10.5 Auto-Binding

For functions defined in a module where the first parameter matches the module type, method call syntax is automatically supported:

```yaoxiang
// === Point.yx ===
Point: Type = { x: Float, y: Float }

// First parameter is Point, auto-supports method call
distance: (a: Point, b: Point) -> Float = { ... }
add: (a: Point, b: Point) -> Point = { ... }

// === main.yx ===
use Point

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

// ✅ Auto-binding: p1.distance(p2) → distance(p1, p2)
d = p1.distance(p2)
// ✅ p1.add(p2) → add(p1, p2)
p3 = p1.add(p2)
```

**Auto-Binding Rules**:
- Function defined in module file
- Function's 0th parameter type matches module name
- Function must be `pub` for auto-binding outside module

### 10.6 Relationship Between Binding and Currying

Binding naturally supports currying. When a call provides fewer arguments than remaining parameters, it returns a function that accepts the remaining parameters:

```yaoxiang
// Original function: 5 parameters
calculate: (scale: Float, a: Point, b: Point, x: Float, y: Float) -> Float = ...

// Binding: Point.calc = calculate[1, 2]
// Remaining parameters after binding: scale, x, y

// Call scenarios
p1.calc(2.0, 10.0, 20.0)              # Provide 3 arguments → direct call
p1.calc(2.0)                          # Provide 1 argument → returns (Float, Float) -> Float
p1.calc()                             # Provide 0 arguments → returns (Float, Float, Float) -> Float
```

---

## Appendix C: Binding Syntax Quick Reference

### C.1 Binding Declaration

```
// Single-position binding (default to position 0)
Type.method = func[0]

// Multi-position binding
Type.method = func[0, 1, 2]

// Using placeholder
Type.method = func[0, _, 2]

// Negative index (last parameter)
Type.method = func[-1]
```

### C.2 Position Index Explanation

```
Function parameters: (p0, p1, p2, p3, p4)
                     ↑  ↑  ↑  ↑  ↑
Index:              0  1  2  3  4

// Binding [1, 3]
Type.method = func[1, 3]
// Call: obj.method(p0, p2, p4)
// Mapping: func(p0_bound, obj, p2, p3_bound, p4)
```

### C.3 Call Forms

```yaoxiang
// Direct call (provide all remaining arguments)
result = p.method(arg1, arg2, arg3)

// Currying (don't provide or partially provide remaining arguments)
f = p.method(arg1)          # Returns function accepting remaining arguments
result = f(arg2, arg3)
```

---

## Version History

| Version | Date | Author | Change Description |
|---------|------|--------|-------------------|
| v1.0.0 | 2024-12-31 | Chen Xu | Initial version |
| v1.1.0 | 2025-01-04 | Mo Yu Jiang | Fixed match arm using `=>` instead of `->`; Updated function definition syntax; Updated type definition syntax; Added differences from code implementation |
| v1.2.0 | 2025-01-05 | Mo Yu Jiang | Streamlined to pure specification, example code moved to tutorial/ directory |
| v1.3.0 | 2025-01-05 | Mo Yu Jiang | Added concurrency model specification (three-layer concurrency architecture, spawn syntax, annotations); Added type system constraints (Send/Sync); Added concurrency-safe types (Mutex, Atomic); Updated error handling (? operator); Updated unimplemented features list |
| v1.4.0 | 2025-01-15 | Chen Xu | Updated ownership model (default Move + explicit ref=Arc); Added unsafe keyword; Removed lifetimes `'a` and borrow checker; Updated unimplemented features list |
| v1.5.0 | 2025-01-20 | Chen Xu | Added method binding specification (RFC-004): position indices start at 0; default binding to position 0; supports negative indices and multi-position binding |
| v1.6.0 | 2025-02-06 | Chen Xu | Integrated RFC-010 (unified type syntax): updated `type Name = {...}` syntax, parameter names in function signatures, Type.method method syntax; Integrated RFC-011 (generic system): added generic types `[T]`, type constraints `[T: Clone]`, associated types `Item: T`, compile-time generics `[N: Int]`, conditional types `If[C, T, E]`, function overload specialization, platform specialization |
| v1.7.0 | 2026-02-13 | Chen Xu | RFC-010 update: `Name: Type = {...}` replaces `type Name = {...}`; only `Type` (uppercase) is the meta type keyword; all declarations use unified syntax |
| v1.8.0 | 2026-02-18 | Chen Xu | RFC-010 new features: default value initialization, builtin binding syntax; RFC-004 new features: builtin binding, anonymous function binding |
| v1.8.1 | 2026-02-20 | Chen Xu | Meta type is not a keyword. |

---

> This specification defines the core syntax and semantics of the YaoXiang programming language.
> For tutorials and example code, please refer to the [YaoXiang Guide](../guide/YaoXiang-book.md) and the [tutorial/](../tutorial/) directory.