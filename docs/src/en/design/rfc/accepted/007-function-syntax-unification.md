---
title: "RFC-007: Unified Function Definition Syntax Proposal"
issue: "#131"
status: "Accepted"
author: "沫郁酱"
created: "2025-01-05"
updated: "2026-07-05 (synced to GH Issue #131)"
---

# RFC-007: Unified Function Definition Syntax Proposal

## Summary

This RFC establishes the final proposal for the **function definition syntax** of the YaoXiang language. It uses the unified syntax `name: (params) -> Return = body`, fully consistent with the `name: type = value` model from RFC-010.

To avoid ambiguity: when a function has input parameters, the parameter types must be explicitly annotated in at least one of the "signature" or "lambda header"; omitting both will be rejected.

Inside a code block `{ ... }`, the `return` keyword must be used to return a value; when there is no `return`, the default return is `Void`. The expression form `= expr` returns the value directly.

## Motivation

### Why is this feature needed?

1. **Syntax Consistency**: Eliminate the historical baggage of old syntax and unify the style
2. **Conciseness**: The HM algorithm automatically infers types, reducing boilerplate
3. **Type Safety**: The HM algorithm guarantees type safety; explicit annotation is only required when inference fails
4. **Language Maturity**: The HM algorithm is a mature solution in modern functional languages

### Unified Syntax Model

**Core Principle**: `name: Signature = LambdaBody`

- **Full form**: Signature (containing parameter names + types + `->` + return type) + Lambda header (containing parameter names)
- **Abbreviation rules**: Omit as much as possible without introducing ambiguity
  - `->` cannot be omitted (it's the marker of a function type; otherwise it would be parsed as a tuple)
  - **When there are input parameters**, the parameter types must explicitly appear in at least the signature or the lambda header
  - The Lambda header can be omitted → if the signature has already declared parameter names and types
  - The return type can be explicitly annotated or omitted when inferable

```yaoxiang
# Full form (complete signature + complete Lambda header)
add: (a: Int, b: Int) -> Int = (a, b) => a + b

# Abbreviation: omit the Lambda header (signature already declares parameters)
add: (a: Int, b: Int) -> Int = a + b

# Abbreviation: omit the signature (lambda header annotates parameter types)
add = (a: Int, b: Int) => a + b

# ❌ Error: parameter types are not annotated on either side
# add = (a, b) => a + b
```

### Design Goals

```yaoxiang
# === Full form ===
add: (a: Int, b: Int) -> Int = (a, b) => { a + b }

# === Abbreviated form ===
add: (a: Int, b: Int) -> Int = a + b                 # Omit the Lambda header
add = (a: Int, b: Int) => a + b                      # Omit the signature

# === Zero-parameter function ===
main: () -> Void = () => { println("Hello") }          # Full form
main: () -> Void = { println("Hello") }                # Omit the Lambda header
main = { println("Hello") }                            # Most concise form (inferred as () -> Void)

# === Generic function (using RFC-010 unified syntax) ===
identity: (T: Type) -> ((x: T) -> T) = (x) => x         # Full form
identity: (T: Type) -> ((x: T) -> T) = x                # Omit the Lambda header
identity = (x: T) => x                                  # Omit the signature (lambda header annotates types)

# === Recursive function ===
factorial: (n: Int) -> Int = (n) => {
    if n <= 1 { return 1 } else { return n * factorial(n - 1) }
}
```

### Syntax Rules

| Scenario | Syntax | Description |
|------|------|------|
| **Full form** | `name: (a: Type, b) -> Ret = (a, b) => { return ... }` | Complete signature + Lambda header |
| **Omit Lambda header** | `name: (a: Type, b: Type) -> Ret = { ... }` | Signature already declares parameters |
| **Omit signature** | `name = (a: Type, b: Type) => { ... }` | Lambda header annotates parameter types |
| **Zero-param full** | `name: () -> Void = () => { return ... }` | Complete zero-parameter function |
| **Zero-param abbreviated** | `name: () -> Void = { return ... }` | Omit the Lambda header |
| **Zero-param most concise** | `name = { return ... }` | No-param no-return most concise |

**Note**: Inside a code block `{ ... }`, the `return` keyword must be used to return a value; when there is no `return`, the default return is `Void`. The expression form `= expr` returns the value directly.

**Note**: `->` is the marker of a function type and cannot be omitted (otherwise it would be parsed as a tuple).

**Important**: The `if` expression uses curly braces `{}` to wrap its branches, and does not support the `then/else` keywords:
```yaoxiang
# Correct: using curly braces
if n <= 1 { return 1 } else { return n * factorial(n - 1) }

# Error: then/else keywords are not supported
# if n <= 1 then return 1 else return n * factorial(n - 1)
```

## Proposal

### HM Algorithm and Higher-Order Polymorphism Support

**Core feature**: The HM algorithm supports higher-rank polymorphism through generic type annotations

**Design rationale**:
- **Higher-order functions**: When a function is passed as an argument, generics are needed to constrain its function type
- **Type annotation form**: `(T: Type) -> ((f: (T) -> T, x: T) -> T)` - generic parameters constrain the function type
- **HM workflow**: Infer function types through generic parameters to achieve polymorphic function composition

**Example explanation**:
```yaoxiang
# ✅ Supports higher-order polymorphism: generic constraints on function-type parameters
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = {
    return f(f(x))
}
# Usage: call_twice((x) => x + 1, 5)  # Infer T=Int

compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = {
    return f(g(x))
}
# Usage: compose((x) => x * 2, (x) => x + 1, 5)  # Infer A=Int, B=Int, C=Int

# ❌ Not supported: higher-order function missing generic constraints
# bad_hof: (f, x) => f(f(x))  # HM cannot infer; missing generic parameters
```

**HM inference process**:
1. Identify higher-order function parameters: `f: (T) -> T`
2. Create generic constraints: `(T: Type)`
3. Infer concrete types through generic instantiation
4. Achieve polymorphic function composition

### Lambda Expression Syntax Rules

**Important rule**: Inside a code block `{ ... }`, the `return` keyword must be used to return a value; when there is no `return`, the default return is `Void`. The expression form `= expr` returns the value directly.

| Syntax form | Syntax | Return method |
|---------|------|----------|
| **Code block form** | `{ statements }` | Must use `return` to return a value; defaults to `Void` when there is no `return` |
| **Expression form** | `expression` | Returns the expression value directly |

**Examples**:
```yaoxiang
main: () -> Void = { println("Hello") }         # Returns Void (no return)
add: (a: Int, b: Int) -> Int = { return a + b }  # Returns Int (explicit return)
empty: () -> Void = {}                          # Empty block defaults to Void

# Early return: use return
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

# Expression form: returns value directly (no return needed)
add: (a: Int, b: Int) -> Int = a + b            # Correct: expression form
main: () -> Void = println("Hello")               # Correct: expression form
```

**Core ideas**:
1. Function definitions use the HM algorithm for type inference, inferring whenever possible, and reporting an explicit error when inference fails
2. **How the HM algorithm works**: Automatically infers types through operator type constraints, function call relationships, and other contextual information
3. **Generic support**: Polymorphic functions use the generic syntax `(T: Type)` to explicitly constrain type parameters (RFC-010/011)
4. **Inference boundary**: Return types and local variables are inferable; parameter types of functions with parameters must be explicitly annotated (in either the signature or the lambda header)
5. Zero-parameter no-return functions use `name: () -> Void = { ... }`, consistent with RFC-010
6. The old syntax is deprecated, and migration tools are provided

**Type inference examples**:
```yaoxiang
# Generic function: explicit type parameters (using RFC-010 unified syntax)
identity: (T: Type) -> ((x: T) -> T) = x
map: (T: Type, R: Type) -> ((f: (T) -> R, list: List(T)) -> List(R)) = {
    result = List(R)()
    for item in list { result.push(f(item)) }
    return result
}

# Polymorphic function: defined through explicit generic constraints (RFC-010/011)
add: (T: Add) -> ((a: T, b: T) -> T) = a + b
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # Inferred as (Int, Int) -> Void

# Higher-order polymorphism: HM support achieved through generic type annotations
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = { return f(f(x)) }
compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = { return f(g(x)) }
```

```yaoxiang
# === Function definition: HM algorithm type inference ===

# Standard function: HM algorithm infers the return type (parameter types must be explicit)
add = (a: Int, b: Int) => a + b            # Inferred as (a: Int, b: Int) -> Int
main = { println("Hello") }                # Inferred as () -> Void

# Partially explicit parameters: HM algorithm infers the rest
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # Inferred as (Int, Int) -> Void
greet: (name: String) -> Void = { println("Hello " + name) }  # Inferred as (String) -> Void

# Generic function: explicitly constrain polymorphic type parameters (using RFC-010 unified syntax)
identity: (T: Type) -> ((x: T) -> T) = x
map: (T: Type, R: Type) -> ((f: (T) -> R, list: List(T)) -> List(R)) = {
    # Implement the map function
    return List(R)()
}

# Recursive function: inferred through HM algorithm and recursive constraints
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 } else { return n * factorial(n - 1) }
}

# === Variable assignment: HM algorithm type inference ===

# Explicit type
x: Int = 42

# HM algorithm automatically infers Int
y = 42                               # Inferred as Int

# HM algorithm automatically infers String
name = "YaoXiang"                    # Inferred as String

# HM algorithm automatically infers Float
pi = 3.14159                         # Inferred as Float
```

**HM type inference rules**:

| Scenario | Syntax | Omittable part | Example |
|------|------|----------|------|
| **Full form** | `name: (a: Type, b: Type) -> Ret = (a, b) => ...` | None | Complete signature + Lambda header |
| **Omit Lambda header** | `name: (a: Type, b: Type) -> Ret = ...` | Lambda header | Signature already declares parameters |
| **Omit signature** | `name = (a: Type, b: Type) => ...` | Signature | Lambda header provides parameter types |
| **Omit return Ret** | `name: (a: Type, b: Type) -> = ...` | Return type | HM infers the return type |
| **Zero-param full** | `name: () -> Void = () => { ... }` | None | Complete zero-parameter function |
| **Zero-param abbreviated** | `name: () -> Void = { ... }` | Lambda header | Omit `() =>` |
| **Zero-param most concise** | `name = { ... }` | All | No-param no-return most concise |
| **Variable assignment** | `name = value` | Type | HM infers the type |
| **Explicit variable** | `name: Type = value` | None | Explicit type annotation |

**Core principles**:
- `->` is the marker of a function type and cannot be omitted (otherwise it would be parsed as a tuple)
- The return type `Ret` can be omitted and is inferred by HM from the function body
- When input parameters exist, parameter types must explicitly appear (in either the signature or the lambda header)
- Other parts can be omitted when inferable and unambiguous
- No implicit type conversions, avoiding JavaScript-style confusion

## Detailed Design

### Syntax Sugar Desugaring

Regardless of omissions, everything is normalized to a unified intermediate representation:

```rust
// Full form
add: (a: Int, b: Int) -> Int = (a, b) => a + b

// Desugared IR
let add: (Int, Int) -> Int = |a: Int, b: Int| -> Int {
    return a + b
};

// Omit Lambda header
add: (a: Int, b: Int) -> Int = a + b

// Desugared IR (same as full form)
let add: (Int, Int) -> Int = |a: Int, b: Int| -> Int {
    return a + b
};

// Omit signature (lambda header annotates parameter types)
add = (a: Int, b: Int) => a + b

// Desugared IR
let add: (Int, Int) -> Int = |a: Int, b: Int| -> Int {
    a + b
};
```

### Syntax Definition

```bnf
function_def ::= identifier ':' type_expr '=' expression
               | identifier '=' expression
               | identifier '=' block                    # Most concise form: no-param no-return

identifier ::= [a-zA-Z_][a-zA-Z0-9_]*

type_expr ::= identifier                     # Type reference
       | '()'                          # Empty type
       | '(' parameters ')' '->' type_expr   # Function type (parameter names in signature)
       | type_expr '->' type_expr            # Simple function type
       | identifier '(' type_expr (',' type_expr)* ')'  # Type application

expression ::= '(' parameters ')' '=>' block
             | '(' ')' '=>' block
             | '(' parameters ')' '=>' expression

parameters ::= parameter (',' parameter)*
parameter ::= identifier                # Type inference
            | identifier ':' type_expr      # Partially explicit type

block ::= '{' statement (',' statement)* '}'
        | expression

statement ::= identifier ':' expression  # Assignment statement
           | expression                  # Expression statement (executed but not returned)
           | 'return' expression         # Return statement (returns the specified value)

# Note: inside a code block, the return keyword must be used to return a value; when there is no return, the default return is Void
# For example: { return 1 + 1 } returns Int; { println("Hello") } returns Void
# Note: generic parameters use the (T: Type) syntax as part of the function type and need no independent BNF rule
```

### Error Handling

```yaoxiang
# === Compilation error examples ===

# Error 1: code block return type mismatch
add: (a: Int, b: Int) -> Int = { println(a + b) }
// Error: no return in the block, defaults to Void, but the signature expects Int
// Correct: add: (a: Int, b: Int) -> Int = a + b
// Or: add: (a: Int, b: Int) -> Int = { return a + b }

# Error 2: using an undeclared type parameter
identity: (x: T) -> T = x
// Error: T is not declared; explicit generic parameters are required (RFC-010)
// Correct: identity: (T: Type) -> ((x: T) -> T) = x

# Correct: HM algorithm infers the return type
double = (x: Int) => x + x

# Full form (gradual abbreviation)
double: (x: Int) -> Int = (x) => x + x                # Full
double: (x: Int) -> Int = x + x                       # Omit Lambda header
double = (x: Int) => x + x                            # Omit return type (HM infers the return)
# double = (x) => x + x                               # ❌ Parameter types cannot be omitted on both sides
```

## Trade-offs

### Advantages

- **Unified syntax**: The `name: Signature = LambdaBody` model covers all scenarios
- **Flexible abbreviation**: Any part can be omitted when inferable by HM
- **Type safety**: The HM algorithm guarantees type safety and avoids implicit type conversions
- **Recursive support**: HM algorithm and recursive constraints automatically infer types
- **Zero burden**: Smooth transition from full to most concise

### Disadvantages

- **Migration cost**: Old code needs to be converted by migration tools
- **Learning cost**: Requires understanding the "full form + arbitrary abbreviation" model

## Alternatives

| Approach | Description | Why not chosen |
|------|------|-----------|
| HM algorithm type inference | Use the Hindley-Milner algorithm to infer types | ✅ **Adopted**, standard for modern functional languages |
| Explicit type declarations | All types must be written explicitly | Violates the simplified syntax principle and adds boilerplate |
| Keep old syntax | Support both old and new syntax simultaneously | Syntax fragmentation with high maintenance cost |
| fn keyword | Introduce fn to distinguish functions from variables | Violates the "functions are lambdas" design |

## Implementation Strategy

### Phases

1. **Phase 1: Syntax parsing and HM algorithm** (v0.3)
   - Implement the new syntax `name = lambda` + HM algorithm type inference
   - Implement default filling for zero-parameter no-return cases

2. **Phase 2: Migration tools** (v0.3)
   - Develop the `yaoxiang-migrate --old-to-new` tool
   - Automatically convert old syntax code

3. **Phase 3: Validation and documentation** (v0.3)
   - Verify completion of old code migration
   - Update documentation

### Migration Tool

```bash
# Migrate a single file
yaoxiang-migrate --old-to-new src/main.yaoxiang

# Migrate an entire project
yaoxiang-migrate --old-to-new --recursive src/

# Preview migration (without modifying files)
yaoxiang-migrate --old-to-new --dry-run src/main.yaoxiang
```

Migration rules:
```yaoxiang
# Old syntax
add(Int, Int) -> Int = (a, b) => { a + b }
main() -> Int = { println("Hello"); 0 }
main() = { println("Hello") }

# === New syntax: full form (complete signature + complete Lambda header) ===
add: (a: Int, b: Int) -> Int = (a, b) => a + b
main: () -> Void = () => { println("Hello") }

# === Abbreviation: omit the Lambda header ===
add: (a: Int, b: Int) -> Int = a + b
main: () -> Void = { println("Hello") }

# === Abbreviation: HM inference ===
add = (a: Int, b: Int) => a + b              # Inferred as (a: Int, b: Int) -> Int
main = { println("Hello") }                  # Inferred as () -> Void

# === Most concise form ===
main = {                                      # Equivalent to main: () -> Void = { ... }
    println("Hello")
}
```

### Dependencies

- No external dependencies
- Can be implemented independently

### Risks

| Risk | Impact | Mitigation |
|------|------|---------|
| Migration omissions | Old code fails to compile | Provide migration tools covering all old syntax patterns |
| Parser errors | Unstable syntax parsing | Comprehensive test coverage |

## Open Questions

> The following questions have been resolved in the design and are recorded in Appendix A.

- ~~Q1: Should the most concise form `main() = body` be kept?~~ → Resolved: kept as `main = { ... }`
- ~~Q2: Should the `:` after the function name be kept?~~ → Resolved: optionally kept; however, functions with parameters still need parameter types annotated in either the signature or the lambda header
- ~~Q3: Does the HM algorithm support parameter type inference?~~ → Resolved: return values/locals are inferable; parameter types of functions with parameters must be explicitly annotated
- ~~Q4: Should the `fn` keyword be introduced?~~ → Resolved: not introduced; functions are lambdas
- ~~Q5: What is the migration strategy for old code?~~ → Resolved: provide the `yaoxiang-migrate` tool
- ~~Q6: How are generic functions used?~~ → Resolved: use the RFC-010 unified syntax `(T: Type)`

---

## Appendix

### Appendix A: Reference for Function Definition Syntax in Various Languages

| Language | Syntax style | Features |
|------|---------|------|
| Rust | `fn add(a: i32, b: i32) -> i32 { ... }` | Keyword + type annotations |
| Haskell | `add a b = ...` / `add :: Int -> Int -> Int` | Separate type signature |
| OCaml | `let add a b = ...` | Parameter types can be omitted |
| MoonBit | `fn add(a: Int, b: Int): Int { ... }` | Concise type annotations |
| TypeScript | `const add = (a: number, b: number): number => ...` | Lambda style |
| Scala | `def add(a: Int, b: Int): Int = { ... }` | def keyword |
| **YaoXiang** | `name = (a: Int, b: Int) => a + b` | **Function = lambda, HM infers return value** |

### Appendix B: Design Decision Records

| Decision | Decision | Date | Recorder |
|------|------|------|--------|
| Syntax style | New syntax `name: (params) -> Return = body` + HM inference | 2026-02-03 | @沫郁酱 |
| Parameter position | Parameter names declared in the signature, consistent with RFC-010 | 2026-02-03 | @沫郁酱 |
| Default filling | Zero-parameter functions can omit the signature; empty block `{}` is inferred as `Void` | 2026-02-03 | @沫郁酱 |
| Type inference | HM algorithm automatically infers; explicit when inference fails | 2026-01-06 | @沫郁酱 |
| Old syntax | Deprecated, migration tools provided | 2026-01-06 | @沫郁酱 |
| fn keyword | Not introduced | 2026-01-06 | @沫郁酱 |
| Recursive declaration | HM algorithm and recursive constraints automatically infer | 2026-01-06 | @沫郁酱 |

### Appendix C: Glossary

| Term | Definition |
|------|------|
| HM algorithm | The Hindley-Milner type inference algorithm that automatically infers types for functions and variables |
| Generics | Using type parameters `(T: Type)` to constrain polymorphic functions, e.g., `identity: (T: Type) -> ((x: T) -> T) = x` (RFC-010) |
| Default type filling | The compiler automatically fills in `-> Void` for zero-parameter no-return functions |
| Syntax sugar | Syntactic simplifications that make code easier to read |
| Normalization | Converting syntactic forms into a unified internal representation |
| Function as lambda | A function is essentially a lambda variable, and its type is automatically inferred by the HM algorithm |

---

## References

- [MoonBit Language Design](https://moonbitlang.com/)
- [Rust Function Syntax](https://doc.rust-lang.org/book/ch03-03-how-functions-work.html)
- [Haskell Type System](https://www.haskell.org/tutorial/patterns.html)
- [OCaml Type Inference](https://v2.ocaml.org/manual/)