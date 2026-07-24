> **Note: This document is archived and no longer maintained.**
> **Please refer to the new language specification document: [Language Specification](../reference/language-spec/index.md)**

---

# YaoXiang Programming Language Specification

> Version: v1.8.0
> Status: Specification
> Author: 晨煦
> Date: 2024-12-31
> Update: 2026-02-22 - Meta type is not a keyword.

---

## Chapter 1: Introduction

### 1.1 Scope

This document defines the syntax and semantics of the YaoXiang programming language. It is the authoritative reference for the language, intended for compiler and tool implementers.

For tutorials and example code, please refer to the [tutorial/](../tutorial/) directory.

### 1.2 Conformance

A program or implementation is considered compliant with the YaoXiang specification if it satisfies all the rules defined in this document.

---

## Chapter 2: Lexical Structure

### 2.1 Source Files

YaoXiang source files must use UTF-8 encoding. Source files typically use the `.yx` extension.

### 2.2 Lexical Token Categories

| Category | Description | Example |
|----------|-------------|---------|
| Identifier | Starts with a letter or underscore | `x`, `_private`, `my_var` |
| Keyword | Language-defined reserved words | `Type`, `pub`, `use` |
| Literal | Fixed values | `42`, `"hello"`, `true` |
| Operator | Operator symbols | `+`, `-`, `*`, `/` |
| Separator | Syntax delimiters | `(`, `)`, `{`, `}`, `,` |

### 2.3 Keywords

YaoXiang defines a very small set of keywords:

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
| `true` | Bool | Boolean true value |
| `false` | Bool | Boolean false value |
| `void` | Void | Void value |
| `some(T)` | Option | Option value variant |
| `ok(T)` | Result | Result success variant |
| `err(E)` | Result | Result error variant |

### 2.5 Identifiers

Identifiers start with a letter or underscore, followed by letters, digits, or underscores. Identifiers are case-sensitive.

Special identifiers:
- `_` is used as a placeholder, indicating an ignored value
- Identifiers starting with an underscore represent private members

### 2.6 Literals

#### 2.6.1 Integers

```
Decimal     ::= [0-9][0-9_]*
Octal       ::= 0o[0-7][0-7_]*
Hex         ::= 0x[0-9a-fA-F][0-9a-fA-F_]*
Binary      ::= 0b[01][01_]*
```

#### 2.6.2 Floats

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
   can span multiple lines */
```

### 2.8 Indentation Rules

Code must use 4 spaces for indentation; Tab characters are forbidden. This is a mandatory syntactic rule.

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
| `Void` | Void value | 0 bytes |
| `Bool` | Boolean value | 1 byte |
| `Int` | Signed integer | 8 bytes |
| `Uint` | Unsigned integer | 8 bytes |
| `Float` | Floating-point number | 8 bytes |
| `String` | UTF-8 string | Variable |
| `Char` | Unicode character | 4 bytes |
| `Bytes` | Raw bytes | Variable |

Integers with bit widths: `Int8`, `Int16`, `Int32`, `Int64`, `Int128`
Floats with bit widths: `Float32`, `Float64`

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

// Record type implementing an interface
Point: Type = {
    x: Float,
    y: Float,
    Drawable,
    Serializable
}
```

**Rules**:
- Record types are defined using curly braces `{}`
- Field names are directly followed by a colon and type
- Interface names written in the type body indicate implementation of that interface

#### 3.3.1 Field Default Values

Type fields can specify default values, which are optional when constructing:

```yaoxiang
// Fields with default values - optional when constructing
Point: Type = {
    x: Float = 0,
    y: Float = 0
}

// Usage
Point()           # → Point(x=0, y=0)
Point(x=1)       # → Point(x=1, y=0)
Point(x=1, y=2) # → Point(x=1, y=2)

// Fields without default values - required when constructing
Point2: Type = {
    x: Float,
    y: Float
}

// Usage
Point2(x=1, y=2) # ✓
Point2()          # ✗ Error
```

**Rules**:
- `field: Type = expression` → has default value, optional when constructing
- `field: Type` → no default value, required when constructing

#### 3.3.2 Built-in Bindings

Methods can be bound directly within a type definition body:

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
// Parameterless variants
Color: Type = { red | green | blue }

// Variants with parameters
Option: Type[T] = { some(T) | none }

// Mixed
Result: Type[T, E] = { ok(T) | err(E) }

// Parameterless variant is equivalent to parameterless constructor
Bool: Type = { true | false }
```

### 3.5 Interface Types

```
InterfaceType ::= '{' FnField (',' FnField)* ','?
FnField       ::= Identifier ':' FnType
FnType        ::= '(' ParamTypes? ')' '->' TypeExpr
```

**Syntax**: An interface is a record type whose fields are all function types

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

**Interface implementation**: A type implements an interface by listing the interface name at the end of its definition

```yaoxiang
// A type that implements interfaces
Point: Type = {
    x: Float,
    y: Float,
    Drawable,        # Implement Drawable interface
    Serializable     # Implement Serializable interface
}
```

**Direct interface assignment**: A concrete type can be directly assigned to an interface type variable (structural subtyping)

```yaoxiang
// Direct assignment (concrete type determinable at compile time → zero-overhead call)
d: Drawable = Circle(1)
d.draw(screen)        # After compilation: directly call circle_draw, no vtable

// Function return value (undeterminable at compile time → vtable call)
d: Drawable = get_shape()
d.draw(screen)        # Look up method through vtable

// Interface as function parameter
process: (d: Drawable) -> Void = d.draw(screen)
```

**Compile-time optimization strategy**:

| Scenario | Inference Result | Call Method |
|----------|------------------|-------------|
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
// Interface type definition (as a constraint)
Clone: Type = {
    clone: (Self) -> Self
}

// Using constraints
clone: [T: Clone](value: T) -> T = value.clone()
```

#### 3.9.2 Multiple Constraints

```yaoxiang
// Multiple constraint syntax
combine: [T: Clone + Add](a: T, b: T) -> T = {
    return a.clone() + b
}

// Generic container sorting
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

### 3.11 Compile-time Generics

#### 3.11.1 Literal Type Constraints

```
LiteralType   ::= Identifier ':' Int          # Compile-time constant
CompileTimeFn ::= '[' Identifier ':' Int ']' '(' Identifier ')' '->' TypeExpr
```

**Core design**: Use `[n: Int]` generic parameter + `(n: n)` value parameter to distinguish compile-time constants from runtime values.

```yaoxiang
// Compile-time factorial: parameter must be a literal known at compile time
factorial: [n: Int](n: n) -> Int = {
    return match n {
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
arr: StaticArray[Int, factorial(5)]  # Compiler computes factorial(5) = 120 at compile time
```

#### 3.11.2 Compile-time Constant Arrays

```yaoxiang
// Matrix type usage
Matrix: Type[T, Rows: Int, Cols: Int] = {
    data: Array[Array[T, Cols], Rows]
}

// Compile-time dimension validation
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

// Example: compile-time branch
NonEmpty: Type[T] = If[T != Void, T, Never]

// Compile-time validation
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

**Syntax**: Type intersection `A & B` represents a type that satisfies both A and B

```yaoxiang
// Interface composition = type intersection
DrawableSerializable: Type = Drawable & Serializable

// Using intersection type
process: [T: Drawable & Serializable](item: T, screen: Surface) -> String = {
    item.draw(screen)
    return item.serialize()
}
```

### 3.15 Function Overloading and Specialization

```yaoxiang
// Basic specialization: using function overloading (compiler selects automatically)
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

// P is a predefined generic parameter name representing the current compilation platform
sum: [P: X86_64](arr: Array[Float]) -> Float = {
    return avx2_sum(arr.data, arr.length)
}

sum: [P: AArch64](arr: Array[Float]) -> Float = {
    return neon_sum(arr.data, arr.length)
}
```

---

## Chapter 3 (continued): Syntax Design Notes

### 3.17 Relationship Between Named Functions and Lambdas

**Core understanding**: Named functions and Lambda expressions are the same thing! The only difference is that a named function gives the Lambda a name.

```yaoxiang
// These two are essentially identical
add: (a: Int, b: Int) -> Int = a + b           # Named function (recommended)
add: (a: Int, b: Int) -> Int = (a, b) => a + b  # Lambda form (completely equivalent)
```

**Syntactic sugar model**:

```
// Named function = Lambda + name
name: (Params) -> ReturnType = body

// Essentially
name: (Params) -> ReturnType = (params) => body
```

**Key point**: When the signature fully declares parameter types, the parameter names in the Lambda head become redundant and can be omitted.

### 3.18 Parameter Scope Rules

**Parameters override outer variables**: Parameters in the signature take precedence over function body, with inner scope having higher priority.

```yaoxiang
x = 10  # Outer variable
double: (x: Int) -> Int = x * 2  # ✅ Parameter x overrides outer x, result is 20
```

### 3.19 Type Annotation Position

Type annotations can be at any of the following positions, **at least one annotation is required**:

| Annotation Position | Form | Description |
|---------------------|------|-------------|
| Signature only | `double: (x: Int) -> Int = x * 2` | ✅ Recommended |
| Lambda head only | `double = (x: Int) => x * 2` | ✅ Valid |
| Both sides | `double: (x: Int) -> Int = (x: Int) => x * 2` | ✅ Redundant but allowed |

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

| Precedence | Operator | Associativity |
|------------|----------|---------------|
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

### 4.3 Function Calls

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

#### 5.9.1 Semantics: Each Iteration Binds a New Value

YaoXiang's for loop semantics differ from traditional languages: **each iteration binds a new value, rather than modifying the same variable**.

```yaoxiang
// Example: for i in 1..5
for i in 1..5 {
    print(i)
}
```

**Execution process**:

| Iteration | Behavior of Loop Variable |
|-----------|---------------------------|
| 1st | Create new binding `i = 1`, execute loop body, print 1 |
| 2nd | Create new binding `i = 2` (previous binding destroyed), execute loop body, print 2 |
| 3rd | Create new binding `i = 3`, execute loop body, print 3 |
| 4th | Create new binding `i = 4`, execute loop body, print 4 |
| End | Loop body ends, binding destroyed |

**Key point**: After each iteration ends, the binding created for that iteration is destroyed. The next iteration is a brand new binding, unrelated to the previous iteration's binding.

#### 5.9.2 Difference Between for and for mut

| Syntax | Loop Variable Mutability | Description |
|--------|--------------------------|-------------|
| `for i in 1..5` | Immutable | Cannot modify the binding in the loop body |
| `for mut i in 1..5` | Mutable | Can modify the binding in the loop body |

```yaoxiang
// ✅ Valid: each iteration binds a new value, no modification needed
for i in 1..5 {
    print(i)  # Read value of i
}

// ❌ Error: immutable binding, cannot modify
for i in 1..5 {
    i = i + 1  # Error: cannot modify immutable binding
}

// ✅ Valid: use for mut to allow modifying the binding
for mut i in 1..5 {
    i = i + 1  # Modification allowed
}
```

#### 5.9.3 Shadowing Check

For loop variables cannot shadow variables that already exist in the outer scope:

```yaoxiang
// ❌ Error: i is already declared externally
i = 10
for i in 1..5 {
    print(i)
}

// ✅ Correct: use a different variable name
i = 10
for j in 1..5 {
    print(j)
}
```

Error code: `E2013 - Cannot shadow existing variable`

#### 5.9.4 Comparison with Other Languages

| Language | For Loop Variable Semantics |
|----------|------------------------------|
| YaoXiang | Each iteration binds a new value |
| Rust | Modifies the same variable (requires mut) |
| Python | Modifies the same variable (no mut needed) |
| C/C++ | Modifies the same variable (requires pointer or reference) |

**Design rationale**: YaoXiang uses binding semantics because:
1. After each iteration, variables in the loop body are destroyed
2. The next iteration is a brand new binding
3. This is safer, no need to consider state between iterations

---

## Chapter 6: Functions

### 6.1 Unified Function Model

**Core syntax**: `name: type = value`

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
// Parameter names declared in the signature
add: (a: Int, b: Int) -> Int = {
    return a + b
}

// Single parameter
inc: (x: Int) -> Int = x + 1

// Parameterless function
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

// Void return type - return is optional
print: (msg: String) -> Void = {
    // No return needed
}

// Single-line expression - returns the value directly, no return needed
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
// Type method: associated with a specific type
Point.draw: (self: Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}

Point.serialize: (self: Point) -> String = {
    return "Point(${self.x}, ${self.y})"
}

// Using method syntactic sugar
p: Point = Point(1.0, 2.0)
p.draw(screen)           # Syntactic sugar → Point.draw(p, screen)
```

#### 6.5.2 Regular Methods

**Syntax**: `name: (Type, ...) -> Return = ...` (not associated with a type)

```yaoxiang
// Regular method: not associated with a type, as an independent function
distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
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

// Using placeholders
Point.calc = func[0, _, 2]
```

#### 6.6.2 pub Automatic Binding

Functions declared with `pub` are automatically bound to types defined in the same file by the compiler:

```yaoxiang
// Using pub declaration, compiler automatically binds
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}

// Compiler automatically infers:
// 1. Point is defined in the current file
// 2. Function parameters include Point
// 3. Execute Point.distance = distance[0]

// Call
d = distance(p1, p2)           # Functional
d2 = p1.distance(p2)           # OOP syntactic sugar
```

### 6.7 Method Binding Rules

| Rule | Description |
|------|-------------|
| Positions start from 0 | `func[0]` binds the 1st parameter (index 0) |
| Maximum position | Must be < number of function parameters |
| Negative index | `[-1]` means the last parameter |
| Placeholder | `_` skips that position, provided by the user |

### 6.8 Currying Support

Binding naturally supports currying. When fewer parameters than remaining are provided at call time, a function accepting the remaining parameters is returned:

```yaoxiang
// Original function: 5 parameters
calculate: (scale: Float, a: Point, b: Point, x: Float, y: Float) -> Float = ...

// Binding: Point.calc = calculate[1, 2]
// Remaining parameters after binding: scale, x, y

// Call scenarios
p1.calc(2.0, 10.0, 20.0)       # Provide 3 parameters → direct call
p1.calc(2.0)                    # Provide 1 parameter → returns (Float, Float) -> Float
p1.calc()                       # Provide 0 parameters → returns (Float, Float, Float) -> Float
```

### 6.9 Spawn Functions and Annotations

#### 6.9.1 spawn Functions

```
SpawnFn     ::= Identifier ':' FnType 'spawn' '=' Lambda
FnType      ::= '(' ParamTypes? ')' '->' TypeExpr ('@' Annotation)?
Annotation  ::= 'block' | 'eager'
```

**Function annotations**:

| Annotation | Position | Behavior |
|------------|----------|----------|
| `@block` | After return type | Disable concurrency optimization, completely sequential execution |
| `@eager` | After return type | Force eager evaluation |

**Syntax examples**:

```
// spawn function: can execute concurrently
fetch_data: (url: String) -> JSON spawn = { ... }

// @block synchronous function: completely sequential execution
main: () -> Void @block = { ... }

// @eager function: immediate execution
compute: (n: Int) -> Int @eager = { ... }
```

#### 6.9.2 spawn Blocks

Explicit concurrency region declaration; tasks within the block execute concurrently:

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

#### 6.9.3 spawn for Loops

Data-parallel loops; loop body executes concurrently over all data elements:

```
SpawnFor    ::= Identifier '=' 'spawn' 'for' Identifier 'in' Expr '{' Expr '}'
```

**Example**:

```
// spawn for loop: data parallel
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
    return transform(data)?
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
| `use path;` | Import module, access using the last part | `use std.io;` → `io.print` |
| `use path.{a, b};` | Import specified items | `use std.io.{print};` → `print` |
| `use path as alias;` | Import and rename | `use std.io as io;` → `io.print` |
| `use path.{i1, i2} as a, b;` | Import specified items and rename | `use std.io.{print, read} as p, r;` → `p`, `r` |

---

## Chapter 8: Memory Management

### 8.1 Ownership Model

YaoXiang uses an **ownership model** to manage memory, where each value has a unique owner:

| Semantic | Description | Syntax |
|----------|-------------|--------|
| **Move** | Default semantic, ownership transfer | `p2 = p` |
| **ref** | Shared (Arc reference counting) | `shared = ref p` |
| **clone()** | Explicit copy | `p2 = p.clone()` |

### 8.2 Move Semantics (Default)

```yaoxiang
// Assignment = Move (zero-copy)
p: Point = Point(1.0, 2.0)
p2 = p              # Move, p becomes invalid

// Function parameter passing = Move
process: (p: Point) -> Void = {
    // Ownership of p transferred in
}

// Return value = Move
create: () -> Point = {
    p = Point(1.0, 2.0)
    return p        # Move, ownership transferred
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
// When shared goes out of scope, count reaches zero and it's automatically released
```

**Features**:
- Thread-safe reference counting
- Automatic lifetime management
- Safe across spawn boundaries

### 8.4 clone() Explicit Copy

```yaoxiang
// Explicitly copy value
p: Point = Point(1.0, 2.0)
p2 = p.clone()      # p and p2 are independent

// Both can be modified, without affecting each other
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
- User guarantees no dangling pointers, no use-after-free
- Does not participate in Send/Sync checks

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

| Constraint | Semantic | Description |
|------------|----------|-------------|
| **Send** | Can be safely transferred across threads | Value can be moved to another thread |
| **Sync** | Can be safely shared across threads | Immutable references can be shared to another thread |

**Auto derivation**:

```
// Send derivation rules
Struct[T1, T2]: Send ⇐ T1: Send and T2: Send

// Sync derivation rules
Struct[T1, T2]: Sync ⇐ T1: Sync and T2: Sync
```

**Type constraints**:

| Type | Send | Sync | Description |
|------|------|------|-------------|
| `T` (value) | ✅ | ✅ | Immutable data |
| `ref T` | ✅ | ✅ | Arc thread-safe |
| `*T` | ❌ | ❌ | Raw pointer unsafe |

---

## Chapter 8 (continued): Type System Constraints

### 8.7 Send/Sync Constraints

YaoXiang uses Rust-style type constraints to ensure concurrency safety:

| Constraint | Semantic | Description |
|------------|----------|-------------|
| **Send** | Can be safely transferred across threads | Value can be moved to another thread |
| **Sync** | Can be safely shared across threads | Immutable references can be shared to another thread |

**Constraint hierarchy**:

```
Send ──► Can be safely transferred across threads
  │
  └──► Sync ──► Can be safely shared across threads
       │
       └──► Types satisfying Send + Sync can automatically be concurrent

Arc[T] implements Send + Sync (thread-safe reference counting)
Mutex[T] provides interior mutability
```

### 8.8 Concurrency-Safe Types

| Type | Semantic | Concurrency Safety | Description |
|------|----------|--------------------|-------------|
| `T` | Immutable data | ✅ Safe | Default type, multi-task reads without races |
| `Ref[T]` | Mutable reference | ⚠️ Needs sync | Marked as concurrently modifiable, compile checks lock usage |
| `Atomic[T]` | Atomic type | ✅ Safe | Low-level atomic operations, lock-free concurrency |
| `Mutex[T]` | Mutex wrapper | ✅ Safe | Automatic lock/unlock, compile-time guarantee |
| `RwLock[T]` | Read-write lock wrapper | ✅ Safe | Optimization for read-heavy scenarios |

**Syntax**:

```
Mutex[T]    # Mutable data wrapped in a mutex
Atomic[T]   # Atomic type (limited to Int, Float, etc.)
RwLock[T]   # Read-write lock wrapper
```

**with syntactic sugar**:

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

**Variant construction**:

| Variant | Syntax | Description |
|---------|--------|-------------|
| `ok(T)` | `ok(value)` | Success value |
| `err(E)` | `err(error)` | Error value |

### 9.2 Option Type

```
Option: Type[T] = some(T) | none
```

**Variant construction**:

| Variant | Syntax | Description |
|---------|--------|-------------|
| `some(T)` | `some(value)` | Has value |
| `none` | `none` | No value |

### 9.3 Error Propagation

```
ErrorPropagate ::= Expr '?'
```

The `?` operator automatically propagates errors of Result type:

```
// On success, return the value; on failure, return err upward
data = fetch_data()?

// Equivalent to
data = match fetch_data() {
    ok(v) => v
    err(e) => return err(e)
}
```

---

## Appendix A: Syntax Quick Reference

### A.1 Type Definition

```
// === Record Types (curly braces) ===

// Struct
Point: Type = { x: Float, y: Float }

// Enum (variant type)
Result: Type[T, E] = { ok(T) | err(E) }
Status: Type = { pending | processing | completed }

// === Interface Types (curly braces, all fields are functions) ===

// Interface definition
Serializable: Type = { serialize: () -> String }

// Type implementing an interface
Point: Type = {
    x: Float,
    y: Float,
    Serializable    # Implement Serializable interface
}

// === Function Type ===

Adder: Type = (Int, Int) -> Int
```

### A.2 Function Definition

```
// Form 1: type-centralized (recommended)
name: (param1: Type1, param2: Type2) -> ReturnType = body

// Form 2: shorthand (parameter names omitted)
name: (Type1, Type2) -> ReturnType = (params) => body

// Generic function
name: [T, R](param: T) -> R = body

// Generic constraint
name: [T: Clone + Add](a: T, b: T) -> T = body
```

### A.3 Method Definition

```
// Type method
Type.method: (self: Type, ...) -> ReturnType = { ... }

// Regular method
name: (Type, ...) -> ReturnType = { ... }
```

### A.4 Method Binding

```
// Single position binding
Type.method = func[0]

// Multi-position binding
Type.method = func[0, 1]

// pub automatic binding
pub name: (Type, ...) -> ReturnType = { ... }  # Automatically bound to Type
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
// filename.yx is the module name
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

> This section describes known differences between the language specification and the current code implementation.

### B.1 Keywords

| Keyword | Specification Status | Code Implementation | Description |
|---------|---------------------|---------------------|-------------|
| `struct` | Removed | ❌ None | Use unified syntax `Name: Type = {...}` |
| `enum` | Removed | ❌ None | Use variant syntax `Name: Type = { A \| B \| C }` |
| `type` | Removed | ❌ None | Use `Type` (capitalized) as the meta type keyword |

### B.2 Syntax Differences

| Syntax Element | Specification | Code Implementation | Description |
|----------------|---------------|---------------------|-------------|
| match arm separator | `->` | `=>` | Use `=>` (FatArrow) |
| Function definition | `name(types) -> type = (params) => body` | Two forms | Supports type-centralized `name: Type = (params) =>` |
| Interface type | `type Serializable = [ serialize() -> String ]` | ❌ Not implemented | Square bracket syntax pending implementation |

### B.3 Features to be Implemented

The following features described in the specification have not yet been implemented in the code:

| Feature | Priority | Description |
|---------|----------|-------------|
| Unified type syntax `Name: Type = {...}` | P0 | RFC-010: Unified syntax replaces `type Name = ...` |
| Curly brace type syntax | P0 | `Point: Type = { x: Float, y: Float }` |
| Interface type | P1 | `Drawable: Type = { draw() -> Void }` |
| List comprehension | P2 | `[x for x in list if condition]` |
| `?` error propagation | P1 | Automatic error propagation for Result type |
| `ref` keyword | P1 | Arc reference-counted sharing |
| `unsafe` code block | P1 | Raw pointers and systems-level programming |
| `*T` raw pointer type | P1 | Raw pointer type syntax |
| `clone()` semantics | P1 | Explicit copy |
| `@block` annotation | P1 | Synchronous execution guarantee |
| `spawn` function | P1 | spawn function marker |
| `spawn {}` block | P1 | Explicit concurrency region |
| `spawn for` loop | P1 | Data-parallel loop |
| Send/Sync constraints | P2 | Concurrency-safe type checking |
| Mutex/Atomic types | P2 | Concurrency-safe data types |
| Error graph visualization | P3 | Concurrency error propagation tracing |
| **Generic type system** | P1 | RFC-011 |
| Basic generics `[T]` | P1 | Generic type parameters and monomorphization |
| Type constraint `[T: Clone]` | P2 | Single/multiple constraint system |
| Associated type `Item: T` | P3 | GAT support |
| Compile-time generic `[N: Int]` | P3 | Literal type constraint |
| Conditional type `If[C, T, E]` | P3 | Type-level computation |
| Function overloading specialization | P2 | Platform and type specialization |
| Method syntax `Type.method` | P1 | `Point.draw: (...) -> ... = ...` |

### B.4 Features Not to be Implemented

The following Rust-style features **will not be implemented**:

| Feature | Reason |
|---------|--------|
| Lifetime `'a` | No reference concept, no need for lifetime |
| Borrow checker | ref = Arc replacement |
| `&T` borrow syntax | Use Move semantics |
| `&mut T` mutable borrow | Use mut + Move |

---

## Chapter 10: Method Binding

### 10.1 Binding Overview

YaoXiang adopts a **pure functional design**, where all operations are implemented through functions. The binding mechanism associates functions with types, allowing callers to invoke functions as if they were calling methods.

```
BindingDeclaration ::= Type '.' Identifier '=' FunctionName '[' PositionList ']'
PositionList ::= Position (',' Position)* ','?
Position     ::= Integer (starting from 0) | NegativeInteger | Placeholder
```

**Core rules**:
- Position index starts from **0**
- Default binding to position **0** (first parameter)
- Supports negative index `[-1]` meaning the last parameter
- Multi-position joint binding `[0, 1, 2]`
- Placeholder `_` indicates skipping that position

### 10.2 Binding Syntax

**Syntax**:
```
Type.method = func[position]
Type.method = func[0, 1, 2]    # Multi-position binding
Type.method = func[0, _, 2]   # Using placeholders
Type.method = func[-1]        # Negative index (last parameter)
```

**Semantics**:
- `Type.method = func[0]` means when calling `obj.method(arg)`, `obj` is bound to the 0th parameter of `func`
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

// === Multi-position Binding ===

// Original function
calculate: (scale: Float, a: Point, b: Point, x: Float, y: Float) -> Float = ...

// Bind multiple positions
Point.calc_scale = calculate[0]      # Only bind scale
Point.calc_both = calculate[1, 2]    # Bind two Point parameters

// Usage
f = p1.calc_scale(2.0)  # → calculate(2.0, p1, _, _, _)
result = f(p2, 10.0, 20.0)  # → calculate(2.0, p1, p2, 10.0, 20.0)

// === Currying (returns function when parameters are insufficient) ===

// Bind one parameter
Point.distance_to = distance[0]

// Usage - not providing second parameter returns curried function
f = p1.distance_to(p2)  # → distance(p1, p2) direct call
f2 = p1.distance_to()   # → distance(p1, _) returns function (Point) -> Float
result = f2(p2)         # → distance(p1, p2)
```

### 10.4 Binding Rules

**Position rules**:
| Rule | Description |
|------|-------------|
| Positions start from 0 | `func[0]` binds the 1st parameter (index 0) |
| Maximum position | Must be < number of function parameters |
| Negative index | `[-1]` means the last parameter |
| Placeholder | `_` skips that position, provided by the user |

**Type checking**:
```yaoxiang
// ✅ Legal binding
Point.distance = distance[0]          # distance(Point, Point)
Point.calc = calculate[1, 2]          # calculate(Float, Point, Point, ...)

// ❌ Illegal binding (compile error)
Point.wrong = distance[5]             # 5 >= 2 (number of parameters)
Point.wrong = distance[0, 0]          # Duplicate position (if not allowed)
Point.wrong = distance[-2]            # -2 out of range
```

### 10.5 Automatic Binding

For functions defined in a module whose first parameter matches the module type, method call syntax is automatically supported:

```yaoxiang
// === Point.yx ===
Point: Type = { x: Float, y: Float }

// First parameter is Point, automatically supports method calls
distance: (a: Point, b: Point) -> Float = { ... }
add: (a: Point, b: Point) -> Point = { ... }

// === main.yx ===
use Point

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

// ✅ Automatic binding: p1.distance(p2) → distance(p1, p2)
d = p1.distance(p2)
// ✅ p1.add(p2) → add(p1, p2)
p3 = p1.add(p2)
```

**Automatic binding rules**:
- Function is defined in the module file
- Function's 0th parameter type matches the module name
- Function must be `pub` to be automatically bound outside the module

### 10.6 Relationship Between Binding and Currying

Binding naturally supports currying. When fewer parameters than remaining are provided at call time, a function accepting the remaining parameters is returned:

```yaoxiang
// Original function: 5 parameters
calculate: (scale: Float, a: Point, b: Point, x: Float, y: Float) -> Float = ...

// Binding: Point.calc = calculate[1, 2]
// Remaining parameters after binding: scale, x, y

// Call scenarios
p1.calc(2.0, 10.0, 20.0)              # Provide 3 parameters → direct call
p1.calc(2.0)                          # Provide 1 parameter → returns (Float, Float) -> Float
p1.calc()                             # Provide 0 parameters → returns (Float, Float, Float) -> Float
```

---

## Appendix C: Binding Syntax Quick Reference

### C.1 Binding Declaration

```
// Single position binding (default binding to position 0)
Type.method = func[0]

// Multi-position binding
Type.method = func[0, 1, 2]

// Using placeholders
Type.method = func[0, _, 2]

// Negative index (last parameter)
Type.method = func[-1]
```

### C.2 Position Index Description

```
Function parameters:    (p0, p1, p2, p3, p4)
                       ↑  ↑  ↑  ↑  ↑
Index:                 0  1  2  3  4

// Binding [1, 3]
Type.method = func[1, 3]
// Call: obj.method(p0, p2, p4)
// Mapping: func(p0_bound, obj, p2, p3_bound, p4)
```

### C.3 Call Forms

```yaoxiang
// Direct call (provide all remaining parameters)
result = p.method(arg1, arg2, arg3)

// Currying (don't provide or partially provide remaining parameters)
f = p.method(arg1)          # Returns function accepting remaining parameters
result = f(arg2, arg3)
```

---

## Version History

| Version | Date | Author | Change Description |
|---------|------|--------|--------------------|
| v1.0.0 | 2024-12-31 | 晨煦 | Initial version |
| v1.1.0 | 2025-01-04 | 沫郁酱 | Fixed match arm using `=>` instead of `->`; updated function definition syntax; updated type definition syntax; added differences from code implementation |
| v1.2.0 | 2025-01-05 | 沫郁酱 | Streamlined to pure specification, example code moved to tutorial/ directory |
| v1.3.0 | 2025-01-05 | 沫郁酱 | Added spawn model specification (three-layer concurrency architecture, spawn syntax, annotations); added type system constraints (Send/Sync); added concurrency-safe types (Mutex, Atomic); updated error handling (`?` operator); updated features to be implemented list |
| v1.4.0 | 2025-01-15 | 晨煦 | Updated ownership model (default Move + explicit ref=Arc); added unsafe keyword; removed lifetime `'a` and borrow checker; updated features to be implemented list |
| v1.5.0 | 2025-01-20 | 晨煦 | Added method binding specification (RFC-004): position index starts from 0; default binding to position 0; supports negative index and multi-position binding |
| v1.6.0 | 2025-02-06 | 晨煦 | Integrated RFC-010 (unified type syntax): updated `type Name = {...}` syntax, parameter names in signature for function definition, Type.method method syntax; integrated RFC-011 (generic system): added generic type `[T]`, type constraint `[T: Clone]`, associated type `Item: T`, compile-time generic `[N: Int]`, conditional type `If[C, T, E]`, function overloading specialization, platform specialization |
| v1.7.0 | 2026-02-13 | 晨煦 | RFC-010 update: `Name: Type = {...}` replaces `type Name = {...}`; only `Type` (capitalized) is the meta type keyword; all declarations use unified syntax |
| v1.8.0 | 2026-02-18 | 晨煦 | RFC-010 added default value initialization, built-in binding syntax; RFC-004 added built-in binding, anonymous function binding |
| v1.8.1 | 2026-02-20 | 晨煦 | Meta type is not a keyword. |

---

> For tutorials and example code, please refer to the [tutorial/](../tutorial/) directory.