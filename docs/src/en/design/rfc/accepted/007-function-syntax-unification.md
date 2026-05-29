---
title: RFC-007: Unified Function Definition Syntax
---

# RFC-007: Unified Function Definition Syntax

> **Status**: Accepted
> **Author**: Mo Yu Jiang
> **Created**: 2025-01-05
> **Last Updated**: 2026-03-21 (aligned with type constructor rules and code block return semantics)

## Summary

This RFC establishes the final scheme for **function definition syntax** in YaoXiang language. It uses the unified syntax `name: (params) -> Return = body`, fully consistent with the `name: type = value` model from RFC-010.

To avoid ambiguity: when a function has input parameters, the parameter types must be explicitly annotated in either the "signature" or the "lambda head" (at least one side); omitting types on both sides will be rejected.

The last expression in a code block `{ ... }` is used as the return value; an empty block `{}` returns `Void`.

## Motivation

### Why is this feature needed?

1. **Syntax Consistency**: Eliminates legacy baggage of old syntax, unifies style
2. **Conciseness**: HM algorithm automatically infers types, reducing boilerplate code
3. **Type Safety**: HM algorithm ensures type safety; explicit annotations only when inference fails
4. **Language Maturity**: HM algorithm is a mature solution in modern functional languages

### Unified Syntax Model

**Core Principle**: `name: Signature = LambdaBody`

- **Complete Form**: Signature (with parameter names + types + `->` + return type) + Lambda head (with parameter names)
- **Shorthand Rules**: Omit parts where doing so doesn't introduce ambiguity
  - `->` cannot be omitted (it's the marker for function type, otherwise parsed as tuple)
  - **When there are input parameters**, parameter types must appear explicitly in either the signature or lambda head
  - Lambda head can be omitted → if signature already declares parameter names and types
  - Return type can be explicitly annotated, or omitted when inferrable

```yaoxiang
# Complete form (complete signature + complete lambda head)
add: (a: Int, b: Int) -> Int = (a, b) => a + b

# Shorthand: omit lambda head (signature already declares parameters)
add: (a: Int, b: Int) -> Int = a + b

# Shorthand: omit signature (lambda head annotates parameter types)
add = (a: Int, b: Int) => a + b

# ❌ Error: parameter types not annotated on either side
# add = (a, b) => a + b
```

### Design Goals

```yaoxiang
# === Complete Form ===
add: (a: Int, b: Int) -> Int = (a, b) => { a + b }

# === Shorthand Forms ===
add: (a: Int, b: Int) -> Int = a + b                 # omit lambda head
add = (a: Int, b: Int) => a + b                      # omit signature

# === Zero-Parameter Functions ===
main: () -> Void = () => { println("Hello") }          # complete form
main: () -> Void = { println("Hello") }                # omit lambda head
main = { println("Hello") }                            # minimal form (inferred as () -> Void)

# === Generic Functions (using RFC-010 unified syntax) ===
identity: (T: Type) -> ((x: T) -> T) = (x) => x         # complete form
identity: (T: Type) -> ((x: T) -> T) = x                # omit lambda head
identity = (x: T) => x                                  # omit signature (lambda head annotates type)

# === Recursive Functions ===
factorial: (n: Int) -> Int = (n) => {
    if n <= 1 { 1 } else { n * factorial(n - 1) }
}
```

### Syntax Rules

| Scenario | Syntax | Description |
|----------|--------|-------------|
| **Complete Form** | `name: (a: Type, b) -> Ret = (a, b) => { return ... }` | Complete signature + lambda head |
| **Omit Lambda Head** | `name: (a: Type, b: Type) -> Ret = { ... }` | Signature already declares parameters |
| **Omit Signature** | `name = (a: Type, b: Type) => { ... }` | Lambda head annotates parameter types |
| **Zero-Param Complete** | `name: () -> Void = () => { return ... }` | Zero-parameter function complete |
| **Zero-Param Shorthand** | `name: () -> Void = { return ... }` | Omit lambda head |
| **Zero-Param Minimal** | `name = { return ... }` | Minimal form, no params, no return |

**Note**: The last expression in block `{ ... }` is used as the return value; use `return` for early exit. Empty block `{}` is inferred as `Void`.

**Note**: `->` is the marker for function type and cannot be omitted (otherwise it will be parsed as tuple).

**Important**: `if` expressions use curly braces `{}` to wrap branches, and do not support `then/else` keywords:
```yaoxiang
# Correct: use curly braces
if n <= 1 { return 1 } else { return n * factorial(n - 1) }

# Error: then/else keywords not supported
# if n <= 1 then return 1 else return n * factorial(n - 1)
```

## Proposal

### HM Algorithm and Higher-Rank Polymorphism Support

**Core Feature**: HM algorithm supports higher-rank polymorphism through generic type annotations.

**Design Principle**:
- **Higher-order functions**: When functions are passed as arguments, they need generic constraints on their function types
- **Type annotation form**: `(T: Type) -> ((f: (T) -> T, x: T) -> T)` - generic parameters constrain function types
- **HM workflow**: Infer function types through generic parameter instantiation, enabling polymorphic function composition

**Example Explanation**:
```yaoxiang
# ✅ Supports higher-rank polymorphism: generic constraints on function type parameters
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = {
    return f(f(x))
}
# Usage: call_twice((x) => x + 1, 5)  # infers T=Int

compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = {
    return f(g(x))
}
# Usage: compose((x) => x * 2, (x) => x + 1, 5)  # infers A=Int, B=Int, C=Int

# ❌ Not supported: higher-order function without generic constraints
# bad_hof: (f, x) => f(f(x))  # HM cannot infer, missing generic parameters
```

**HM Inference Process**:
1. Identify higher-order function parameters: `f: (T) -> T`
2. Create generic constraints: `(T: Type)`
3. Infer concrete types through generic instantiation
4. Enable polymorphic function composition

### Lambda Expression Syntax Rules

**Important Rule**: Code block `{ ... }` returns the value of its last expression; use `return` for early exit. Empty block `{}` returns `Void`.

| Syntax Form | Syntax | Return Method |
|-------------|--------|---------------|
| **Code Block Form** | `{ statements }` | Last expression as return value; `return` can be used for early return |
| **Expression Form** | `expression` | Directly return expression value |

**Examples**:
```yaoxiang
main: () -> Void = { println("Hello") }         # returns Void (last expression is println)
add: (a: Int, b: Int) -> Int = { a + b }        # returns Int (last expression is a + b)
empty: () -> Void = {}                          # empty block returns Void

# Early return: use return
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    n * factorial(n - 1)
}

# Expression form: directly return value (no return needed)
add: (a: Int, b: Int) -> Int = a + b            # correct: expression form
main: () -> Void = println("Hello")               # correct: expression form
```

**Core Ideas**:
1. Function definitions use HM algorithm for type inference; infer where possible, report explicit errors otherwise
2. **How HM Algorithm Works**: Automatically infers types through contextual information like operator type constraints and function call relationships
3. **Generics Support**: Polymorphic functions use generic syntax `(T: Type)` to explicitly constrain type parameters (RFC-010/011)
4. **Inference Boundaries**: Return types and local variables can be inferred; parameter types for functions with parameters must be explicitly annotated (in signature or lambda head)
5. Zero-parameter, no-return functions use `name: () -> Void = { ... }`, unified with RFC-010
6. Old syntax retired, migration tools provided

**Type Inference Examples**:
```yaoxiang
# Generic functions: explicit type parameters (using RFC-010 unified syntax)
identity: (T: Type) -> ((x: T) -> T) = x
map: (T: Type, R: Type) -> ((f: (T) -> R, list: List(T)) -> List(R)) = {
    result = List(R)()
    for item in list { result.push(f(item)) }
    return result
}

# Polymorphic functions: defined through explicit generic constraints (RFC-010/011)
add: (T: Add) -> ((a: T, b: T) -> T) = a + b
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # inferred as (Int, Int) -> Void

# Higher-rank polymorphism: achieved through generic type annotations for HM support
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = { return f(f(x)) }
compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = { return f(g(x)) }
```

```yaoxiang
# === Function Definition: HM Algorithm Type Inference ===

# Standard function: HM algorithm infers return type (parameter types need explicit)
add = (a: Int, b: Int) => a + b            # inferred as (a: Int, b: Int) -> Int
main = { println("Hello") }                # inferred as () -> Void

# Partial explicit parameters: HM algorithm infers remaining parts
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # inferred as (Int, Int) -> Void
greet: (name: String) -> Void = { println("Hello " + name) }  # inferred as (String) -> Void

# Generic functions: explicitly constrain polymorphic type parameters (using RFC-010 unified syntax)
identity: (T: Type) -> ((x: T) -> T) = x
map: (T: Type, R: Type) -> ((f: (T) -> R, list: List(T)) -> List(R)) = {
    # Implement map function
    return List(R)()
}

# Recursive functions: inferred through HM algorithm and recursive constraints
factorial: (n: Int) -> Int = {
    if n <= 1 { 1 } else { n * factorial(n - 1) }
}

# === Variable Assignment: HM Algorithm Type Inference ===

# Explicit type
x: Int = 42

# HM algorithm automatically infers as Int
y = 42                               # inferred as Int

# HM algorithm automatically infers as String
name = "YaoXiang"                    # inferred as String

# HM algorithm automatically infers as Float
pi = 3.14159                         # inferred as Float
```

**HM Type Inference Rules**:

| Scenario | Syntax | Omissible Parts | Example |
|----------|--------|-----------------|---------|
| **Complete Form** | `name: (a: Type, b: Type) -> Ret = (a, b) => ...` | None | Signature + lambda head complete |
| **Omit Lambda Head** | `name: (a: Type, b: Type) -> Ret = ...` | Lambda head | Signature already declares parameters |
| **Omit Signature** | `name = (a: Type, b: Type) => ...` | Signature | Lambda head provides parameter types |
| **Omit Return Ret** | `name: (a: Type, b: Type) -> = ...` | Return type | HM infers return type |
| **Zero-Param Complete** | `name: () -> Void = () => { ... }` | None | Zero-parameter function complete |
| **Zero-Param Shorthand** | `name: () -> Void = { ... }` | Lambda head | Omit `() =>` |
| **Zero-Param Minimal** | `name = { ... }` | All | No params, no return minimal |
| **Variable Assignment** | `name = value` | Type | HM infers type |
| **Explicit Variable** | `name: Type = value` | None | Explicit type annotation |

**Core Principles**:
- `->` is the marker for function type and cannot be omitted (otherwise it will be parsed as tuple)
- Return type `Ret` can be omitted, inferred by HM from function body
- When input parameters exist, parameter types must appear explicitly (in either signature or lambda head)
- Other parts can be omitted when inferrable and not introducing ambiguity
- No implicit type conversions, avoiding JavaScript-like confusion

## Detailed Design

### Syntax Sugar Expansion

Regardless of omissions, everything is normalized to a unified intermediate representation:

```rust
// Complete form
add: (a: Int, b: Int) -> Int = (a, b) => a + b

// Expanded IR
let add: (Int, Int) -> Int = |a: Int, b: Int| -> Int {
    return a + b
};

// Omit lambda head
add: (a: Int, b: Int) -> Int = a + b

// Expanded IR (same as complete form)
let add: (Int, Int) -> Int = |a: Int, b: Int| -> Int {
    return a + b
};

// Omit signature (lambda head annotates parameter types)
add = (a: Int, b: Int) => a + b

// Expanded IR
let add: (Int, Int) -> Int = |a: Int, b: Int| -> Int {
    a + b
};
```

### Syntax Definition

```bnf
function_def ::= identifier ':' type_expr '=' expression
               | identifier '=' expression
               | identifier '=' block                    # minimal form: no params, no return

identifier ::= [a-zA-Z_][a-zA-Z0-9_]*

type_expr ::= identifier                     # type reference
       | '()'                          # empty type
       | '(' parameters ')' '->' type_expr   # function type (parameter names in signature)
       | type_expr '->' type_expr            # simple function type
       | identifier '(' type_expr (',' type_expr)* ')'  # type application

expression ::= '(' parameters ')' '=>' block
             | '(' ')' '=>' block
             | '(' parameters ')' '=>' expression

parameters ::= parameter (',' parameter)*
parameter ::= identifier                # type inference
            | identifier ':' type_expr      # partially explicit type

block ::= '{' statement (',' statement)* '}'
        | expression

statement ::= identifier ':' expression  # assignment statement
           | expression                  # expression statement (executes but doesn't return)
           | 'return' expression         # return statement (returns specified value)

# Note: code block returns the value of its last expression; empty block {} infers as Void
# Example: { 1 + 1 } returns Int; { println("Hello") } returns Void
# Note: generic parameters use (T: Type) syntax, as part of function type, no separate BNF rule needed
```

### Error Handling

```yaoxiang
# === Compilation Error Examples ===

# Error 1: code block return type mismatch
add: (a: Int, b: Int) -> Int = { println(a + b) }
// Error: the last expression in block is println(...), returns Void, but signature expects Int
// Correct: add: (a: Int, b: Int) -> Int = a + b
// Or: add: (a: Int, b: Int) -> Int = { a + b }

# Error 2: using undeclared type parameter
identity: (x: T) -> T = x
// Error: T is undeclared; need explicit generic parameter (RFC-010)
// Correct: identity: (T: Type) -> ((x: T) -> T) = x

# Correct: HM algorithm infers return type
double = (x: Int) => x + x

# Complete form (progressive shorthand)
double: (x: Int) -> Int = (x) => x + x                # complete
double: (x: Int) -> Int = x + x                       # omit lambda head
double = (x: Int) => x + x                            # omit return type (HM infers)
# double = (x) => x + x                               # ❌ parameter types cannot be omitted on both sides
```

## Trade-offs

### Advantages

- **Unified Syntax**: `name: Signature = LambdaBody` model covers all scenarios
- **Flexible Shorthand**: Any part can be omitted when inferrable by HM
- **Type Safety**: HM algorithm ensures type safety, no implicit type conversions
- **Recursion Support**: HM algorithm and recursive constraints automatically infer types
- **Zero Burden**: Smooth transition from complete to minimal forms

### Disadvantages

- **Migration Cost**: Old code requires migration tool conversion
- **Learning Curve**: Need to understand the "complete form + arbitrary shorthand" model

## Alternative Approaches

| Approach | Description | Why Not Chosen |
|----------|-------------|----------------|
| HM Algorithm Type Inference | Use Hindley-Milner algorithm to infer types | ✅ **Adopted**, standard for modern functional languages |
| Explicit Type Declaration | All types must be explicitly written | Violates syntax simplification principle, increases boilerplate |
| Keep Old Syntax | Support both old and new syntax | Syntax split, high maintenance cost |
| fn Keyword | Introduce fn to distinguish functions from variables | Violates "function is lambda" design |

## Implementation Strategy

### Phase Division

1. **Phase 1: Syntax Parsing and HM Algorithm** (v0.3)
   - Implement new syntax `name = lambda` + HM algorithm type inference
   - Implement default filling for zero-parameter, no-return functions

2. **Phase 2: Migration Tool** (v0.3)
   - Develop `yaoxiang-migrate --old-to-new` tool
   - Automatically convert old syntax code

3. **Phase 3: Validation and Documentation** (v0.3)
   - Complete old code migration validation
   - Update documentation

### Migration Tool

```bash
# Migrate single file
yaoxiang-migrate --old-to-new src/main.yaoxiang

# Migrate entire project
yaoxiang-migrate --old-to-new --recursive src/

# Preview migration (don't modify files)
yaoxiang-migrate --old-to-new --dry-run src/main.yaoxiang
```

Migration rules:
```yaoxiang
# Old syntax
add(Int, Int) -> Int = (a, b) => { a + b }
main() -> Int = { println("Hello"); 0 }
main() = { println("Hello") }

# === New syntax: complete form (complete signature + complete lambda head) ===
add: (a: Int, b: Int) -> Int = (a, b) => a + b
main: () -> Void = () => { println("Hello") }

# === Shorthand: omit lambda head ===
add: (a: Int, b: Int) -> Int = a + b
main: () -> Void = { println("Hello") }

# === Shorthand: HM inference ===
add = (a: Int, b: Int) => a + b              # inferred as (a: Int, b: Int) -> Int
main = { println("Hello") }                  # inferred as () -> Void

# === Minimal Form ===
main = {                                      # equivalent to main: () -> Void = { ... }
    println("Hello")
}
```

### Dependencies

- No external dependencies
- Can be implemented independently

### Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Migration omission | Old code fails to compile | Provide migration tool covering all old syntax patterns |
| Parser errors | Syntax parsing unstable | Adequate test coverage |

## Open Questions

> The following issues have been resolved in the design, recorded in Appendix A.

- ~~Q1: Should we keep the ultra-minimal `main() = body` syntax?~~ → Resolved: Keep as `main = { ... }`
- ~~Q2: Should the `:` after function name be kept?~~ → Resolved: Optionally keep; but functions with parameters still need parameter types annotated in signature or lambda head
- ~~Q3: Does HM algorithm support parameter type inference?~~ → Resolved: Return/局部 variables can be inferred; parameter types for functions with parameters need explicit annotation
- ~~Q4: Should we introduce the `fn` keyword?~~ → Resolved: No, functions are lambdas
- ~~Q5: What is the migration strategy for old code?~~ → Resolved: Provide `yaoxiang-migrate` tool
- ~~Q6: How to use generic functions?~~ → Resolved: Use RFC-010 unified syntax `(T: Type)`

---

## Appendix

### Appendix A: Function Definition Syntax Reference by Language

| Language | Syntax Style | Characteristics |
|----------|-------------|-----------------|
| Rust | `fn add(a: i32, b: i32) -> i32 { ... }` | Keyword + type annotation |
| Haskell | `add a b = ...` / `add :: Int -> Int -> Int` | Type signature separated |
| OCaml | `let add a b = ...` | Parameter types can be omitted |
| MoonBit | `fn add(a: Int, b: Int): Int { ... }` | Concise type annotation |
| TypeScript | `const add = (a: number, b: number): number => ...` | Lambda style |
| Scala | `def add(a: Int, b: Int): Int = { ... }` | def keyword |
| **YaoXiang** | `name = (a: Int, b: Int) => a + b` | **Function = lambda, HM infers return value** |

### Appendix B: Design Decision Record

| Decision | Decision Made | Date | Recorded By |
|----------|---------------|------|-------------|
| Syntax style | New syntax `name: (params) -> Return = body` + HM inference | 2026-02-03 | @Mo Yu Jiang |
| Parameter placement | Parameter names declared in signature, unified with RFC-010 | 2026-02-03 | @Mo Yu Jiang |
| Default filling | Zero-parameter functions can omit signature, empty block `{}` infers as `Void` | 2026-02-03 | @Mo Yu Jiang |
| Type inference | HM algorithm automatically infers, explicit when cannot infer | 2026-01-06 | @Mo Yu Jiang |
| Old syntax | Retired, migration tool provided | 2026-01-06 | @Mo Yu Jiang |
| fn keyword | Not introduced | 2026-01-06 | @Mo Yu Jiang |
| Recursive declaration | HM algorithm and recursive constraints automatically infer | 2026-01-06 | @Mo Yu Jiang |

### Appendix C: Glossary

| Term | Definition |
|------|------------|
| HM Algorithm | Hindley-Milner type inference algorithm, automatically infers types for functions and variables |
| Generics | Using type parameters `(T: Type)` to constrain polymorphic functions, e.g., `identity: (T: Type) -> ((x: T) -> T) = x` (RFC-010) |
| Default Type Filling | Zero-parameter, no-return functions can omit `-> Void`, compiler fills automatically |
| Syntax Sugar | Syntactic simplifications that make code more readable |
| Normalization | Converting syntax forms to unified internal representation |
| Function is Lambda | Function is essentially a lambda variable, type automatically inferred through HM algorithm |

---

## References

- [MoonBit Language Design](https://moonbitlang.com/)
- [Rust Functions](https://doc.rust-lang.org/book/ch03-03-how-functions-work.html)
- [Haskell Type System](https://www.haskell.org/tutorial/patterns.html)
- [OCaml Type Inference](https://v2.ocaml.org/manual/)