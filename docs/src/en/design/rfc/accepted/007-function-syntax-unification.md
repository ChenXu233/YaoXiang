```markdown
---
title: "RFC-007: Unified Function Definition Syntax"
status: "Accepted"
author: "Moyujiang"
created: "2025-01-05"
updated: "2026-03-21 (aligned with type constructor rules and code block return semantics)"
---

# RFC-007: Unified Function Definition Syntax

## Summary

This RFC establishes the final scheme for **function definition syntax** in YaoXiang language. Using the unified syntax `name: (params) -> Return = body`, which is fully consistent with the `name: type = value` model from RFC-010.

To avoid ambiguity: when a function has input parameters, the parameter types must be explicitly annotated in either the "signature" or the "lambda head" (at least one); omitting both will be rejected.

Code blocks `{ ... }` must use `return` to return a value; without `return`, they default to returning `Void`. The expression form `= expr` returns a value directly.

## Motivation

### Why is this feature needed?

1. **Syntax consistency**: Eliminates legacy baggage from the old syntax, unified style
2. **Conciseness**: HM algorithm automatically infers types, reducing boilerplate
3. **Type safety**: HM algorithm ensures type safety; explicit annotation only when inference is impossible
4. **Language maturity**: HM algorithm is a mature solution in modern functional languages

### Unified Syntax Model

**Core principle**: `name: Signature = LambdaBody`

- **Complete form**: Signature (with parameter names + types + `->` + return type) + Lambda head (with parameter names)
- **Shorthand rules**: Omit where possible without introducing ambiguity
  - `->` cannot be omitted (it marks function type, otherwise parsed as tuple)
  - **When there are input parameters**, parameter types must appear explicitly in either the signature or the lambda head (at least one)
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

# === No-Parameter Functions ===
main: () -> Void = () => { println("Hello") }          # complete form
main: () -> Void = { println("Hello") }                # omit lambda head
main = { println("Hello") }                            # minimal form (inferred as () -> Void)

# === Generic Functions (using RFC-010 unified syntax) ===
identity: (T: Type) -> ((x: T) -> T) = (x) => x         # complete form
identity: (T: Type) -> ((x: T) -> T) = x                # omit lambda head
identity = (x: T) => x                                  # omit signature (lambda head annotates type)

# === Recursive Functions ===
factorial: (n: Int) -> Int = (n) => {
    if n <= 1 { return 1 } else { return n * factorial(n - 1) }
}
```

### Syntax Rules

| Scenario | Syntax | Description |
|----------|--------|-------------|
| **Complete form** | `name: (a: Type, b) -> Ret = (a, b) => { return ... }` | Signature + complete lambda head |
| **Omit lambda head** | `name: (a: Type, b: Type) -> Ret = { ... }` | Signature declares parameters |
| **Omit signature** | `name = (a: Type, b: Type) => { ... }` | Lambda head provides parameter types |
| **No-param complete** | `name: () -> Void = () => { return ... }` | No-param function complete |
| **No-param shorthand** | `name: () -> Void = { return ... }` | Omit lambda head |
| **No-param minimal** | `name = { return ... }` | No-param, no-return minimal |

**Note**: Code blocks `{ ... }` must use `return` to return a value; without `return`, they default to returning `Void`. The expression form `= expr` returns a value directly.

**Note**: `->` is the marker for function type and cannot be omitted (otherwise it will be parsed as a tuple).

**Important**: `if` expressions use curly braces `{}` to wrap branches, and do not support `then/else` keywords:
```yaoxiang
# Correct: using curly braces
if n <= 1 { return 1 } else { return n * factorial(n - 1) }

# Error: then/else keywords not supported
# if n <= 1 then return 1 else return n * factorial(n - 1)
```

## Proposal

### HM Algorithm and Higher-Rank Polymorphism Support

**Core feature**: HM algorithm supports higher-rank polymorphism through generic type annotations

**Design principle**:
- **Higher-order functions**: When functions are passed as arguments, they need generic constraints on their function types
- **Type annotation form**: `(T: Type) -> ((f: (T) -> T, x: T) -> T)` - generic parameters constrain function types
- **HM workflow**: Infer function types through generic parameter instantiation, enabling polymorphic function composition

**Example explanation**:
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

# ❌ Not supported: higher-order function without generic constraint
# bad_hof: (f, x) => f(f(x))  # HM cannot infer, missing generic parameters
```

**HM inference process**:
1. Identify higher-order function parameters: `f: (T) -> T`
2. Create generic constraint: `(T: Type)`
3. Infer concrete types through generic instantiation
4. Enable polymorphic function composition

### Lambda Expression Syntax Rules

**Important rule**: Code blocks `{ ... }` must use `return` to return a value; without `return`, they default to returning `Void`. The expression form `= expr` returns a value directly.

| Syntax form | Syntax | Return method |
|-------------|--------|---------------|
| **Code block form** | `{ statements }` | Must use `return` to return value; defaults to `Void` without `return` |
| **Expression form** | `expression` | Directly returns expression value |

**Examples**:
```yaoxiang
main: () -> Void = { println("Hello") }         # returns Void (no return)
add: (a: Int, b: Int) -> Int = { return a + b }  # returns Int (explicit return)
empty: () -> Void = {}                          # empty block defaults to Void

# Early return: use return
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

# Expression form: return value directly (no return needed)
add: (a: Int, b: Int) -> Int = a + b            # correct: expression form
main: () -> Void = println("Hello")               # correct: expression form
```

**Core ideas**:
1. Function definitions use HM algorithm for type inference, inferring where possible, explicitly erroring when inference is impossible
2. **How HM algorithm works**: automatically infers types through contextual information like operator type constraints and function call relationships
3. **Generics support**: polymorphic functions use generic syntax `(T: Type)` to explicitly constrain type parameters (RFC-010/011)
4. **Inference boundaries**: return types and local variables can be inferred; parameter types for functions with parameters must be explicitly annotated (in signature or lambda head)
5. No-parameter, no-return functions use `name: () -> Void = { ... }`, unified with RFC-010
6. Retire old syntax, provide migration tools

**Type inference examples**:
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

# Standard functions: HM algorithm infers return type (parameter types must be explicit)
add = (a: Int, b: Int) => a + b            # inferred as (a: Int, b: Int) -> Int
main = { println("Hello") }                # inferred as () -> Void

# Partially explicit parameters: HM algorithm infers the rest
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
    if n <= 1 { return 1 } else { return n * factorial(n - 1) }
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

**HM type inference rules**:

| Scenario | Syntax | Omissible parts | Example |
|----------|--------|-----------------|---------|
| **Complete form** | `name: (a: Type, b: Type) -> Ret = (a, b) => ...` | None | Signature + complete lambda head |
| **Omit lambda head** | `name: (a: Type, b: Type) -> Ret = ...` | Lambda head | Signature declares parameters |
| **Omit signature** | `name = (a: Type, b: Type) => ...` | Signature | Lambda head provides parameter types |
| **Omit return Ret** | `name: (a: Type, b: Type) -> = ...` | Return type | HM infers return type |
| **No-param complete** | `name: () -> Void = () => { ... }` | None | No-param function complete |
| **No-param shorthand** | `name: () -> Void = { ... }` | Lambda head | Omit `() =>` |
| **No-param minimal** | `name = { ... }` | All | No-param, no-return minimal |
| **Variable assignment** | `name = value` | Type | HM infers type |
| **Explicit variable** | `name: Type = value` | None | Explicit type annotation |

**Core principles**:
- `->` is the marker for function type and cannot be omitted (otherwise it will be parsed as a tuple)
- Return type `Ret` can be omitted, inferred by HM from function body
- When input parameters exist, parameter types must appear explicitly (in either signature or lambda head)
- Other parts can be omitted when inferrable and without introducing ambiguity
- No implicit type conversions, avoiding JavaScript-style chaos

## Detailed Design

### Syntax Sugar Expansion

Whether omitted or not, everything is normalized to a unified intermediate representation:

```rust
// Complete form
add: (a: Int, b: Int) -> Int = (a, b) => a + b

// Expanded IR
let add: (Int, Int) -> Int = |a: Int, b: Int| -> Int {
    return a + b
};

// Omit lambda head
add: (a: Int, b: Int) -> Int = a + b

// Expanded IR (identical to complete form)
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
               | identifier '=' block                    # minimal form: no-param, no-return

identifier ::= [a-zA-Z_][a-zA-Z0-9_]*

type_expr ::= identifier                     # type reference
       | '()'                          # unit type
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

# Note: code blocks must use return to return a value; without return, defaults to Void
# Example: { return 1 + 1 } returns Int; { println("Hello") } returns Void
# Note: generic parameters use (T: Type) syntax, as part of function type, no separate BNF rule needed
```

### Error Handling

```yaoxiang
# === Compilation Error Examples ===

# Error 1: code block return type mismatch
add: (a: Int, b: Int) -> Int = { println(a + b) }
// Error: no return in block, defaults to Void, but signature expects Int
// Correct: add: (a: Int, b: Int) -> Int = a + b
// Or: add: (a: Int, b: Int) -> Int = { return a + b }

# Error 2: using undeclared type parameter
identity: (x: T) -> T = x
// Error: T not declared; explicit generic parameter needed (RFC-010)
// Correct: identity: (T: Type) -> ((x: T) -> T) = x

# Correct: HM algorithm infers return type
double = (x: Int) => x + x

# Complete form (gradual shorthand)
double: (x: Int) -> Int = (x) => x + x                # complete
double: (x: Int) -> Int = x + x                       # omit lambda head
double = (x: Int) => x + x                            # omit return type (HM infers return)
# double = (x) => x + x                               # ❌ parameter types not allowed to be omitted on both sides
```

## Trade-offs

### Advantages

- **Syntax unification**: `name: Signature = LambdaBody` model covers all scenarios
- **Flexible shorthand**: any part can be omitted when HM can infer it
- **Type safety**: HM algorithm ensures type safety, avoiding implicit type conversions
- **Recursion support**: HM algorithm and recursion constraints automatically infer types
- **Zero burden**: smooth transition from complete to minimal form

### Disadvantages

- **Migration cost**: old code needs migration tools to convert
- **Learning curve**: need to understand the "complete form + arbitrary shorthand" model

## Alternative Approaches

| Approach | Description | Why not chosen |
|----------|-------------|----------------|
| HM algorithm type inference | Use Hindley-Milner algorithm for type inference | ✅ **Adopted**, standard for modern functional languages |
| Explicit type declaration | All types must be explicitly written | Violates simplified syntax principle, increases boilerplate |
| Keep old syntax | Support both old and new syntax | Syntax fragmentation, high maintenance cost |
| fn keyword | Introduce fn to distinguish functions from variables | Violates "functions are just lambdas" design |

## Implementation Strategy

### Phases

1. **Phase 1: Syntax parsing and HM algorithm** (v0.3)
   - Implement new syntax `name = lambda` + HM algorithm type inference
   - Implement default filling for no-param, no-return functions

2. **Phase 2: Migration tool** (v0.3)
   - Develop `yaoxiang-migrate --old-to-new` tool
   - Automatically convert old syntax code

3. **Phase 3: Verification and documentation** (v0.3)
   - Verify old code migration completion
   - Update documentation

### Migration Tool

```bash
# Migrate single file
yaoxiang-migrate --old-to-new src/main.yaoxiang

# Migrate entire project
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

# === New syntax: complete form (complete signature + complete lambda head) ===
add: (a: Int, b: Int) -> Int = (a, b) => a + b
main: () -> Void = () => { println("Hello") }

# === Shorthand: omit lambda head ===
add: (a: Int, b: Int) -> Int = a + b
main: () -> Void = { println("Hello") }

# === Shorthand: HM inference ===
add = (a: Int, b: Int) => a + b              # inferred as (a: Int, b: Int) -> Int
main = { println("Hello") }                  # inferred as () -> Void

# === Minimal form ===
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
| Parser errors | Unstable syntax parsing | Sufficient test coverage |

## Open Issues

> The following issues have been resolved in the design and are recorded in Appendix A.

- ~~Q1: Should we keep `main() = body` minimal form?~~ → Resolved: kept as `main = { ... }`
- ~~Q2: Should we keep the `:` after function names?~~ → Resolved: optional to keep; but functions with parameters still need parameter types annotated in signature or lambda head
- ~~Q3: Does HM algorithm support parameter type inference?~~ → Resolved: return type/local variables can be inferred; parameter types for functions with parameters must be explicitly annotated
- ~~Q4: Should we introduce `fn` keyword?~~ → Resolved: not introduced, functions are just lambdas
- ~~Q5: What is the migration strategy for old code?~~ → Resolved: provide `yaoxiang-migrate` tool
- ~~Q6: How to use generic functions?~~ → Resolved: use RFC-010 unified syntax `(T: Type)`

---

## Appendices

### Appendix A: Function Definition Syntax Reference for Various Languages

| Language | Syntax style | Characteristics |
|----------|--------------|-----------------|
| Rust | `fn add(a: i32, b: i32) -> i32 { ... }` | Keyword + type annotation |
| Haskell | `add a b = ...` / `add :: Int -> Int -> Int` | Type signature separated |
| OCaml | `let add a b = ...` | Parameter types can be omitted |
| MoonBit | `fn add(a: Int, b: Int): Int { ... }` | Concise type annotation |
| TypeScript | `const add = (a: number, b: number): number => ...` | Lambda style |
| Scala | `def add(a: Int, b: Int): Int = { ... }` | def keyword |
| **YaoXiang** | `name = (a: Int, b: Int) => a + b` | **Function = lambda, HM infers return value** |

### Appendix B: Design Decision Records

| Decision | Resolution | Date | Recorder |
|----------|-----------|------|----------|
| Syntax style | New syntax `name: (params) -> Return = body` + HM inference | 2026-02-03 | @Moyujiang |
| Parameter position | Parameter names declared in signature, unified with RFC-010 | 2026-02-03 | @Moyujiang |
| Default filling | No-param functions can omit signature, empty block `{}` inferred as `Void` | 2026-02-03 | @Moyujiang |
| Type inference | HM algorithm automatically infers, explicit when inference impossible | 2026-01-06 | @Moyujiang |
| Old syntax | Retired, migration tool provided | 2026-01-06 | @Moyujiang |
| fn keyword | Not introduced | 2026-01-06 | @Moyujiang |
| Recursive declaration | HM algorithm and recursion constraints automatically infer | 2026-01-06 | @Moyujiang |

### Appendix C: Glossary

| Term | Definition |
|------|------------|
| HM algorithm | Hindley-Milner type inference algorithm, automatically infers function and variable types |
| Generics | Using type parameters `(T: Type)` to constrain polymorphic functions, e.g., `identity: (T: Type) -> ((x: T) -> T) = x` (RFC-010) |
| Default type filling | No-param, no-return functions omit `-> Void`, compiler automatically fills |
| Syntax sugar | Simplified syntax that makes code more readable |
| Normalization | Converting syntactic forms into unified internal representation |
| Functions are lambdas | Functions are essentially lambda variables, types automatically inferred through HM algorithm |

---

## References

- [MoonBit Language Design](https://moonbitlang.com/)
- [Rust Function Syntax](https://doc.rust-lang.org/book/ch03-03-how-functions-work.html)
- [Haskell Type System](https://www.haskell.org/tutorial/patterns.html)
- [OCaml Type Inference](https://v2.ocaml.org/manual/)
```