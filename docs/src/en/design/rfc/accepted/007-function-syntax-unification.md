---
title: RFC-007: Unified Function Definition Syntax
---

# RFC-007: Unified Function Definition Syntax

> **Status**: Accepted
> **Author**: 沫郁酱
> **Created**: 2025-01-05
> **Last Updated**: 2026-03-21 (aligned with type constructor rules and code block return semantics)

## Summary

This RFC establishes the final scheme for **function definition syntax** in the YaoXiang language. It uses a unified syntax `name: (params) -> Return = body`, fully consistent with the `name: type = value` model from RFC-010.

To avoid ambiguity: when a function has input parameters, parameter types must be explicitly annotated in either the "signature" or the "lambda head" (at least one); omitting types from both sides will be rejected.

The last expression in a code block `{ ... }` is used as the return value; an empty block `{}` returns `Void`.

## Motivation

### Why is this feature needed?

1. **Syntax Consistency**: Eliminates historical baggage of old syntax, unified style
2. **Conciseness**: HM algorithm automatically infers types, reducing boilerplate code
3. **Type Safety**: HM algorithm ensures type safety; explicit annotation only when inference fails
4. **Language Maturity**: HM algorithm is a mature solution in modern functional languages

### Unified Syntax Model

**Core Principle**: `name: Signature = LambdaBody`

- **Complete form**: signature (parameter names + types + `->` + return type) + lambda head (with parameter names)
- **Abbreviation rules**: Omit as much as possible without introducing ambiguity
  - `->` cannot be omitted (it's the marker for function type, otherwise parsed as a tuple)
  - **When there are input parameters**, parameter types must appear explicitly in either the signature or the lambda head
  - Lambda head can be omitted → if the signature already declares parameter names and types
  - Return type can be explicitly annotated or omitted when inferrable

```yaoxiang
# Complete form (complete signature + complete lambda head)
add: (a: Int, b: Int) -> Int = (a, b) => a + b

# Abbreviation: omit lambda head (signature already declares parameters)
add: (a: Int, b: Int) -> Int = a + b

# Abbreviation: omit signature (lambda head annotates parameter types)
add = (a: Int, b: Int) => a + b

# ❌ Error: parameter types omitted from both sides
# add = (a, b) => a + b
```

### Design Goals

```yaoxiang
# === Complete Form ===
add: (a: Int, b: Int) -> Int = (a, b) => { a + b }

# === Abbreviated Forms ===
add: (a: Int, b: Int) -> Int = a + b                 # omit lambda head
add = (a: Int, b: Int) => a + b                      # omit signature

# === Zero-Parameter Functions ===
main: () -> Void = () => { println("Hello") }          # complete form
main: () -> Void = { println("Hello") }                # omit lambda head
main = { println("Hello") }                            # most minimal (inferred as () -> Void)

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
| **Complete form** | `name: (a: Type, b) -> Ret = (a, b) => { return ... }` | Complete signature + lambda head |
| **Omit lambda head** | `name: (a: Type, b: Type) -> Ret = { ... }` | Signature already declares parameters |
| **Omit signature** | `name = (a: Type, b: Type) => { ... }` | Lambda head provides parameter types |
| **Zero-parameter complete** | `name: () -> Void = () => { return ... }` | Zero-parameter complete |
| **Zero-parameter abbreviated** | `name: () -> Void = { return ... }` | Omit lambda head |
| **Zero-parameter minimal** | `name = { return ... }` | Zero-parameter, zero-return most minimal |

**Note**: The last expression in a block `{ ... }` is used as the return value; use `return` to exit early. Empty block `{}` is inferred as `Void`.

**Note**: `->` is the marker for function type and cannot be omitted (otherwise it will be parsed as a tuple).

**Important**: `if` expressions use curly braces `{}` to wrap branches, and do not support `then/else` keywords:
```yaoxiang
# Correct: use curly braces
if n <= 1 { return 1 } else { return n * factorial(n - 1) }

# Incorrect: then/else keywords not supported
# if n <= 1 then return 1 else return n * factorial(n - 1)
```

## Proposal

### HM Algorithm and Higher-Rank Polymorphism Support

**Core Feature**: HM algorithm supports higher-rank polymorphism through generic type annotations

**Design Principle**:
- **Higher-order functions**: When functions are passed as parameters, they need generic constraints on their function types
- **Type annotation form**: `(T: Type) -> ((f: (T) -> T, x: T) -> T)` - generic parameters constrain function types
- **HM workflow**: Infer function types through generic parameters, enabling polymorphic function composition

**Example Explanation**:
```yaoxiang
# ✅ Supports higher-rank polymorphism: generic constrains function type parameters
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = {
    return f(f(x))
}
# Usage: call_twice((x) => x + 1, 5)  # infers T=Int

compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = {
    return f(g(x))
}
# Usage: compose((x) => x * 2, (x) => x + 1, 5)  # infers A=Int, B=Int, C=Int

# ❌ Not supported: higher-order functions without generic constraints
# bad_hof: (f, x) => f(f(x))  # HM cannot infer, missing generic parameters
```

**HM Inference Process**:
1. Identify higher-order function parameters: `f: (T) -> T`
2. Create generic constraints: `(T: Type)`
3. Infer concrete types through generic instantiation
4. Enable polymorphic function composition

### Lambda Expression Syntax Rules

**Important Rule**: Code blocks `{ ... }` return the value of their last expression; use `return` to exit early when needed. Empty block `{}` returns `Void`.

| Syntax Form | Syntax | Return Method |
|-------------|--------|---------------|
| **Code block form** | `{ statements }` | Last expression as return value; `return` can be used for early return |
| **Expression form** | `expression` | Directly return expression value |

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

# Expression form: return value directly (no return needed)
add: (a: Int, b: Int) -> Int = a + b            # correct: expression form
main: () -> Void = println("Hello")               # correct: expression form
```

**Core Ideas**:
1. Function definitions use HM algorithm for type inference; infer as much as possible, report explicit errors when inference fails
2. **How HM algorithm works**: Automatically infers types through operator type constraints, function call relationships, and other contextual information
3. **Generic support**: Polymorphic functions use generic syntax `(T: Type)` to explicitly constrain type parameters (RFC-010/011)
4. **Inference boundaries**: Return types and local variables can be inferred; parameter types of functions with parameters need explicit annotation (one of signature or lambda head)
5. Zero-parameter, no-return functions use `name: () -> Void = { ... }`, unified with RFC-010
6. Old syntax is deprecated; migration tools provided

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

# Higher-rank polymorphism: HM supports higher-rank polymorphism via generic type annotations
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = { return f(f(x)) }
compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = { return f(g(x)) }
```

```yaoxiang
# === Function Definition: HM Algorithm Type Inference ===

# Standard functions: HM algorithm infers return type (parameter types explicit)
add = (a: Int, b: Int) => a + b            # inferred as (a: Int, b: Int) -> Int
main = { println("Hello") }                # inferred as () -> Void

# Partially explicit parameters: HM algorithm infers remaining parts
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # inferred as (Int, Int) -> Void
greet: (name: String) -> Void = { println("Hello " + name) }  # inferred as (String) -> Void

# Generic functions: explicitly constrain polymorphic type parameters (using RFC-010 unified syntax)
identity: (T: Type) -> ((x: T) -> T) = x
map: (T: Type, R: Type) -> ((f: (T) -> R, list: List(T)) -> List(R)) = {
    # Implement map function
    return List(R)()
}

# Recursive functions: inferred through HM algorithm and recursion constraints
factorial: (n: Int) -> Int = {
    if n <= 1 { 1 } else { n * factorial(n - 1) }
}

# === Variable Assignment: HM Algorithm Type Inference ===

# Explicit type
x: Int = 42

# HM algorithm auto-infers as Int
y = 42                               # inferred as Int

# HM algorithm auto-infers as String
name = "YaoXiang"                    # inferred as String

# HM algorithm auto-infers as Float
pi = 3.14159                         # inferred as Float
```

**HM Type Inference Rules**:

| Scenario | Syntax | Omissible Parts | Example |
|----------|--------|-----------------|---------|
| **Complete form** | `name: (a: Type, b: Type) -> Ret = (a, b) => ...` | None | Complete signature + lambda head |
| **Omit lambda head** | `name: (a: Type, b: Type) -> Ret = ...` | Lambda head | Signature already declares parameters |
| **Omit signature** | `name = (a: Type, b: Type) => ...` | Signature | Lambda head provides parameter types |
| **Omit return Ret** | `name: (a: Type, b: Type) -> = ...` | Return type | HM infers return type |
| **Zero-parameter complete** | `name: () -> Void = () => { ... }` | None | Zero-parameter complete |
| **Zero-parameter abbreviated** | `name: () -> Void = { ... }` | Lambda head | Omit `() =>` |
| **Zero-parameter minimal** | `name = { ... }` | All | Zero-parameter, zero-return most minimal |
| **Variable assignment** | `name = value` | Type | HM infers type |
| **Explicit variable** | `name: Type = value` | None | Explicit type annotation |

**Core Principles**:
- `->` is the function type marker and cannot be omitted (otherwise it will be parsed as a tuple)
- Return type `Ret` can be omitted, inferred by HM from the function body
- When there are input parameters, parameter types must appear explicitly (in either signature or lambda head)
- Other parts can be omitted when inferrable and without introducing ambiguity
- No implicit type conversions, avoiding JavaScript-like chaos

## Detailed Design

### Syntax Sugar Expansion

Regardless of omissions, everything normalizes to a unified intermediate representation:

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
               | identifier '=' block                    # most minimal: zero-param, zero-return

identifier ::= [a-zA-Z_][a-zA-Z0-9_]*

type_expr ::= identifier                     # type reference
       | '()'                          # void type
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

# Note: code blocks return the value of their last expression; empty block {} infers to Void
# Example: { 1 + 1 } returns Int; { println("Hello") } returns Void
# Note: generic parameters use (T: Type) syntax, as part of function type, no separate BNF rule needed
```

### Error Handling

```yaoxiang
# === Compile Error Examples ===

# Error 1: code block return type mismatch
add: (a: Int, b: Int) -> Int = { println(a + b) }
// Error: the last expression of the block is println(...), returns Void, but signature expects Int
// Correct: add: (a: Int, b: Int) -> Int = a + b
// Or: add: (a: Int, b: Int) -> Int = { a + b }

# Error 2: using undeclared type parameter
identity: (x: T) -> T = x
// Error: T is not declared; need explicit generic parameter (RFC-010)
// Correct: identity: (T: Type) -> ((x: T) -> T) = x

# Correct: HM algorithm infers return type
double = (x: Int) => x + x

# Complete form (step-by-step abbreviation)
double: (x: Int) -> Int = (x) => x + x                # complete
double: (x: Int) -> Int = x + x                       # omit lambda head
double = (x: Int) => x + x                            # omit return type (HM infers return)
// double = (x) => x + x                               # ❌ parameter types cannot be omitted from both sides
```

## Tradeoffs

### Advantages

- **Syntax Unification**: `name: Signature = LambdaBody` model covers all scenarios
- **Flexible Abbreviation**: any part can be omitted when HM can infer it
- **Type Safety**: HM algorithm ensures type safety, avoiding implicit type conversions
- **Recursion Support**: HM algorithm and recursion constraints auto-infer types
- **Zero Burden**: smooth transition from complete to most minimal forms

### Disadvantages

- **Migration Cost**: old code requires migration tool conversion
- **Learning Cost**: need to understand the "complete form + arbitrary abbreviation" model

## Alternative Approaches

| Approach | Description | Why Not Chosen |
|----------|-------------|----------------|
| HM algorithm type inference | Use Hindley-Milner algorithm to infer types | ✅ **Adopted**, modern functional language standard |
| Explicit type declaration | All types must be explicitly written | Violates syntax simplification principle, increases boilerplate |
| Preserve old syntax | Support both old and new syntax simultaneously | Syntax fragmentation, high maintenance cost |
| fn keyword | Introduce fn to distinguish functions from variables | Violates "function is lambda" design |

## Implementation Strategy

### Phase Division

1. **Phase 1: Syntax Parsing and HM Algorithm** (v0.3)
   - Implement new syntax `name = lambda` + HM algorithm type inference
   - Implement default filling for zero-parameter no-return functions

2. **Phase 2: Migration Tool** (v0.3)
   - Develop `yaoxiang-migrate --old-to-new` tool
   - Automatically convert old syntax code

3. **Phase 3: Verification and Documentation** (v0.3)
   - Verify old code migration completion
   - Update documentation

### Migration Tool

```bash
# Migrate a single file
yaoxiang-migrate --old-to-new src/main.yaoxiang

# Migrate entire project
yaoxiang-migrate --old-to-new --recursive src/

# Preview migration (do not modify files)
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

# === Abbreviation: omit lambda head ===
add: (a: Int, b: Int) -> Int = a + b
main: () -> Void = { println("Hello") }

# === Abbreviation: HM inference ===
add = (a: Int, b: Int) => a + b              # inferred as (a: Int, b: Int) -> Int
main = { println("Hello") }                  # inferred as () -> Void

# === Most minimal form ===
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
| Migration omission | Old code fails to compile | Provide migration tool, covering all old syntax patterns |
| Parser errors | Unstable syntax parsing | Sufficient test coverage |

## Open Questions

> The following questions have been resolved in the design, recorded in Appendix A.

- ~~Q1: Should we keep `main() = body` extreme minimalist writing?~~ → Resolved: kept as `main = { ... }`
- ~~Q2: Should the `:` after function name be kept?~~ → Resolved: optional to keep; but functions with parameters still need parameter types annotated in signature or lambda head
- ~~Q3: Does HM algorithm support parameter type inference?~~ → Resolved: return type/local variables can be inferred; parameter types of functions with parameters need explicit annotation
- ~~Q4: Should we introduce `fn` keyword?~~ → Resolved: no, functions are lambdas
- ~~Q5: What is the migration strategy for old code?~~ → Resolved: provide `yaoxiang-migrate` tool
- ~~Q6: How do generic functions work?~~ → Resolved: use RFC-010 unified syntax `(T: Type)`

---

## Appendix

### Appendix A: Function Definition Syntax Reference by Language

| Language | Syntax Style | Characteristics |
|----------|--------------|-----------------|
| Rust | `fn add(a: i32, b: i32) -> i32 { ... }` | Keyword + type annotation |
| Haskell | `add a b = ...` / `add :: Int -> Int -> Int` | Type signature separated |
| OCaml | `let add a b = ...` | Parameter types can be omitted |
| MoonBit | `fn add(a: Int, b: Int): Int { ... }` | Concise type annotation |
| TypeScript | `const add = (a: number, b: number): number => ...` | Lambda style |
| Scala | `def add(a: Int, b: Int): Int = { ... }` | def keyword |
| **YaoXiang** | `name = (a: Int, b: Int) => a + b` | **Function = lambda, HM infers return value** |

### Appendix B: Design Decision Record

| Decision | Decision | Date | Recorder |
|----------|----------|------|----------|
| Syntax style | New syntax `name: (params) -> Return = body` + HM inference | 2026-02-03 | @沫郁酱 |
| Parameter position | Parameter names declared in signature, unified with RFC-010 | 2026-02-03 | @沫郁酱 |
| Default filling | Zero-parameter functions can omit signature, empty block `{}` infers to `Void` | 2026-02-03 | @沫郁酱 |
| Type inference | HM algorithm auto-infers, explicit when inference fails | 2026-01-06 | @沫郁酱 |
| Old syntax | Deprecated, migration tool provided | 2026-01-06 | @沫郁酱 |
| fn keyword | Not introduced | 2026-01-06 | @沫郁酱 |
| Recursive declaration | HM algorithm and recursion constraints auto-infer | 2026-01-06 | @沫郁酱 |

### Appendix C: Glossary

| Term | Definition |
|------|------------|
| HM algorithm | Hindley-Milner type inference algorithm, automatically infers types of functions and variables |
| Generics | Using type parameters `(T: Type)` to constrain polymorphic functions, e.g., `identity: (T: Type) -> ((x: T) -> T) = x` (RFC-010) |
| Default type filling | Zero-parameter no-return functions omit `-> Void`, compiler auto-fills |
| Syntax sugar | Syntactic simplifications that make code more readable |
| Normalization | Converting syntactic forms to unified internal representation |
| Function is lambda | Functions are essentially lambda variables, types auto-inferred through HM algorithm |

---

## References

- [MoonBit Language Design](https://moonbitlang.com/)
- [Rust Function Syntax](https://doc.rust-lang.org/book/ch03-03-how-functions-work.html)
- [Haskell Type System](https://www.haskell.org/tutorial/patterns.html)
- [OCaml Type Inference](https://v2.ocaml.org/manual/)