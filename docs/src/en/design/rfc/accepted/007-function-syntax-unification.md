---
title: 'RFC-007: Function Definition Syntax Unification Proposal'
issue: '#131'
status: 'Accepted'
author: 'Moyujiang'
created: '2025-01-05'
updated: '2026-09-15'
---

# RFC-007: Function Definition Syntax Unification Proposal

> **Related**: For the overlap between the "no-parameter minimal" form `name = { ... }` and the
> "valued block" syntax position in RFC-010, the ruling is **content determines type** — `=>` is
> always a function, the `Fn` annotation marks a function, a non-`Fn` annotation marks a block
> value, and without annotation the type is inferred from the content. See Appendix D of
> [RFC-010a](./010a-tail-expression-and-return.md).
>
> **Related supplement**: Statement termination and newline rules within function bodies (`{ ... }`
> code blocks) — i.e. explicit `;` separators, newline termination, and line-continuation exceptions
> — are defined by [RFC-038 (draft)](../draft/038-statement-termination.md) and are not covered by
> this RFC.

## Summary

This RFC establishes the final proposal for **function definition syntax** in the YaoXiang language.
It uses a unified syntax `name: (params) -> Return = body`, which is fully consistent with the
`name: type = value` model of RFC-010.

To avoid ambiguity, when a function has input parameters, the parameter types must be explicitly
annotated in at least one of the "signature" or the "lambda head"; omitting them in both is
rejected.

The value of a code block `{ ... }` is given by the **tail expression**; `return` is a non-local
exit of type `Never` (see [RFC-010a](./010a-tail-expression-and-return.md)). The expression form
`= expr` provides the value directly.

## Motivation

### Why is this feature needed?

1. **Syntactic consistency**: Eliminate the historical baggage of the old syntax and unify the style
2. **Conciseness**: HM algorithm automatically infers types, reducing boilerplate code
3. **Type safety**: The HM algorithm guarantees type safety; types are explicit only when inference
   fails
4. **Language maturity**: The HM algorithm is a mature solution in modern functional languages

### Unified Syntax Model

**Core principle**: `name: Signature = LambdaBody`

- **Full form**: signature (including parameter names + types + `->` + return type) + lambda head
  (including parameter names)
- **Abbreviation rules**: Omit as much as possible without introducing ambiguity
  - `->` cannot be omitted (the marker of a function type; otherwise it would be parsed as a tuple)
  - **When there are input parameters**, the parameter types must appear explicitly in either the
    signature or the lambda head
  - The lambda head can be omitted — if the signature has already declared parameter names and types
  - The return type can be annotated explicitly, or omitted when inferable

```yaoxiang
# Full form (signature complete + lambda head complete)
add: (a: Int, b: Int) -> Int = (a, b) => a + b

# Abbreviation: omit the lambda head (the signature has already declared the parameters)
add: (a: Int, b: Int) -> Int = a + b

# Abbreviation: omit the signature (the lambda head annotates parameter types)
add = (a: Int, b: Int) => a + b

# ❌ Error: parameter types omitted on both sides
# add = (a, b) => a + b
```

### Design Goals

```yaoxiang
# === Full form ===
add: (a: Int, b: Int) -> Int = (a, b) => { a + b }

# === Abbreviated forms ===
add: (a: Int, b: Int) -> Int = a + b                 # Omit the lambda head
add = (a: Int, b: Int) => a + b                      # Omit the signature

# === No-parameter functions ===
main: () -> Void = () => { println("Hello") }          # Full form
main: () -> Void = { println("Hello") }                # Omit the lambda head
main: () -> Void = { println("Hello") }                            # Minimal form (inferred as () -> Void)

# === Generic functions (using RFC-010 unified syntax) ===
identity: (T: Type) -> ((x: T) -> T) = (x) => x         # Full form
identity: (T: Type) -> ((x: T) -> T) = x                # Omit the lambda head
identity = (x: T) => x                                  # Omit the signature (the lambda head annotates types)

# === Recursive functions ===
factorial: (n: Int) -> Int = (n) => {
    if n <= 1 { return 1 } else { return n * factorial(n - 1) }
}
```

### Syntax Rules

| Scenario               | Syntax                                                 | Description                           |
| ---------------------- | ------------------------------------------------------ | ------------------------------------- |
| **Full form**          | `name: (a: Type, b) -> Ret = (a, b) => { return ... }` | Signature + lambda head complete      |
| **Omit lambda head**   | `name: (a: Type, b: Type) -> Ret = { ... }`            | Signature has declared parameters     |
| **Omit signature**     | `name = (a: Type, b: Type) => { ... }`                 | Lambda head annotates parameter types |
| **No-param full**      | `name: () -> Void = () => { return ... }`              | No-parameter function, full form      |
| **No-param shorthand** | `name: () -> Void = { return ... }`                    | Omit the lambda head                  |
| **No-param minimal**   | `name = { return ... }`                                | No parameter, no return, minimal      |

**Note**: The value of a code block `{ ... }` is given by the **tail expression** (the sole exit
point); `return` is a non-local exit of type `Never`, exiting the nearest function boundary. The
expression form `= expr` provides the value directly. See
[RFC-010a](./010a-tail-expression-and-return.md) for details.

**Note**: `->` is the marker of a function type and cannot be omitted (otherwise it would be parsed
as a tuple).

**Important**: `if` expressions use curly braces `{}` to wrap branches; the `then/else` keywords are
not supported:

```yaoxiang
# Correct: use curly braces
if n <= 1 { return 1 } else { return n * factorial(n - 1) }

# Error: then/else keywords are not supported
# if n <= 1 then return 1 else return n * factorial(n - 1)
```

## Proposal

### HM Algorithm and Higher-Rank Polymorphism Support

**Core feature**: The HM algorithm supports higher-rank polymorphism through generic type
annotations.

**Design rationale**:

- **Higher-order functions**: When functions are passed as arguments, generics are needed to
  constrain the function type
- **Type annotation form**: `(T: Type) -> ((f: (T) -> T, x: T) -> T)` — generic parameters constrain
  the function type
- **HM workflow**: Function types are inferred through generic parameters, enabling polymorphic
  function composition

**Examples**:

```yaoxiang
# ✅ Supports higher-rank polymorphism: generics constrain function-type parameters
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = {
    return f(f(x))
}
# Usage: call_twice((x) => x + 1, 5)  # Infers T=Int

compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = {
    return f(g(x))
}
# Usage: compose((x) => x * 2, (x) => x + 1, 5)  # Infers A=Int, B=Int, C=Int

# ❌ Not supported: higher-order functions lacking generic constraints
# bad_hof: (f, x) => f(f(x))  # HM cannot infer; generic parameters are missing
```

**HM inference process**:

1. Identify the higher-order function parameter: `f: (T) -> T`
2. Create a generic constraint: `(T: Type)`
3. Infer concrete types through generic instantiation
4. Achieve polymorphic function composition

### Lambda Expression Syntax Rules

**Important rule**: The value of a code block `{ ... }` is given by the **tail expression** (the
sole exit point); `return` is a non-local exit of type `Never`, exiting the nearest function
boundary. The expression form `= expr` provides the value directly. See
[RFC-010a](./010a-tail-expression-and-return.md) for details.

| Syntax form         | Syntax           | Value exit                                     |
| ------------------- | ---------------- | ---------------------------------------------- |
| **Code block form** | `{ statements }` | Tail expression (empty block `{}` is `Void`)   |
| **Expression form** | `expression`     | The expression value                           |
| **`return`**        | `return e`       | Non-local exit from the function, type `Never` |

**Examples**:

```yaoxiang
main: () -> Void = { println("Hello") }         # Tail expression is Void
add: (a: Int, b: Int) -> Int = { a + b }        # The tail expression provides the value
empty: () -> Void = {}                          # Empty block → Void

# Early return: use return
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

# Expression form: provide the value directly
add: (a: Int, b: Int) -> Int = a + b            # Correct: expression form
main: () -> Void = println("Hello")               # Correct: expression form
```

**Core ideas**:

1. Function definitions use the HM algorithm for type inference — infer whenever possible, and
   report an error explicitly when inference is impossible
2. **How the HM algorithm works**: It automatically infers types through context information such as
   operator type constraints and function call relationships
3. **Generic support**: Polymorphic functions use the generic syntax `(T: Type)` to explicitly
   constrain type parameters (RFC-010/011)
4. **Inference boundary**: Return types and local variables are inferable; parameter types of
   functions with arguments must be annotated explicitly (in either the signature or the lambda
   head)
5. No-parameter, no-return functions use `name: () -> Void = { ... }`, unified with RFC-010
6. The old syntax is retired, and migration tools are provided

**Type inference examples**:

```yaoxiang
# Generic functions: explicit type parameters (using RFC-010 unified syntax)
identity: (T: Type) -> ((x: T) -> T) = x
map: (T: Type, R: Type) -> ((f: (T) -> R, list: List(T)) -> List(R)) = {
    result = List(R)()
    for item in list { result.push(f(item)) }
    return result
}

# Polymorphic functions: defined via explicit generic constraints (RFC-010/011)
add: (T: Add) -> ((a: T, b: T) -> T) = a + b
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # Inferred as (Int, Int) -> Void

# Higher-rank polymorphism: implemented through generic type annotations so that HM supports higher-rank polymorphism
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = { return f(f(x)) }
compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = { return f(g(x)) }
```

```yaoxiang
# === Function definition: HM type inference ===

# Standard function: the HM algorithm infers the return type (parameter types must be explicit)
add = (a: Int, b: Int) => a + b            # Inferred as (a: Int, b: Int) -> Int
main: () -> Void = { println("Hello") }                # Inferred as () -> Void

# Partially explicit parameters: the HM algorithm infers the rest
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # Inferred as (Int, Int) -> Void
greet: (name: String) -> Void = { println("Hello " + name) }  # Inferred as (String) -> Void

# Generic functions: explicitly constrain polymorphic type parameters (using RFC-010 unified syntax)
identity: (T: Type) -> ((x: T) -> T) = x
map: (T: Type, R: Type) -> ((f: (T) -> R, list: List(T)) -> List(R)) = {
    # Implementing the map function
    return List(R)()
}

# Recursive functions: inferred through the HM algorithm and recursive constraints
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 } else { return n * factorial(n - 1) }
}

# === Variable assignment: HM type inference ===

# Explicit type
x: Int = 42

# The HM algorithm automatically infers Int
y = 42                               # Inferred as Int

# The HM algorithm automatically infers String
name = "YaoXiang"                    # Inferred as String

# The HM algorithm automatically infers Float
pi = 3.14159                         # Inferred as Float
```

**HM type inference rules**:

| Scenario                | Syntax                                            | Omissible part | Example                              |
| ----------------------- | ------------------------------------------------- | -------------- | ------------------------------------ |
| **Full form**           | `name: (a: Type, b: Type) -> Ret = (a, b) => ...` | None           | Signature + lambda head complete     |
| **Omit lambda head**    | `name: (a: Type, b: Type) -> Ret = ...`           | Lambda head    | Signature has declared parameters    |
| **Omit signature**      | `name = (a: Type, b: Type) => ...`                | Signature      | Lambda head provides parameter types |
| **Omit return type**    | `name: (a: Type, b: Type) -> = ...`               | Return type    | HM infers the return type            |
| **No-param full**       | `name: () -> Void = () => { ... }`                | None           | No-parameter function, full form     |
| **No-param shorthand**  | `name: () -> Void = { ... }`                      | Lambda head    | Omit `() =>`                         |
| **No-param minimal**    | `name = { ... }`                                  | All            | No parameter, no return, minimal     |
| **Variable assignment** | `name = value`                                    | Type           | HM infers the type                   |
| **Explicit variable**   | `name: Type = value`                              | None           | Explicit type annotation             |

**Core principles**:

- `->` is the marker of a function type and cannot be omitted (otherwise it would be parsed as a
  tuple)
- The return type `Ret` can be omitted and is inferred by HM from the function body
- When input parameters exist, parameter types must appear explicitly (in either the signature or
  the lambda head)
- The remaining parts can be omitted whenever they are inferable and ambiguity is avoided
- No implicit type conversions — to avoid JavaScript-style confusion

## Detailed Design

### Syntactic Sugar Desugaring

Regardless of what is omitted, everything is ultimately normalized to a unified intermediate
representation:

```rust
// Full form
add: (a: Int, b: Int) -> Int = (a, b) => a + b

// Desugared IR
let add: (Int, Int) -> Int = |a: Int, b: Int| -> Int {
    return a + b
};

// Omit the lambda head
add: (a: Int, b: Int) -> Int = a + b

// Desugared IR (same as the full form)
let add: (Int, Int) -> Int = |a: Int, b: Int| -> Int {
    return a + b
};

// Omit the signature (the lambda head annotates parameter types)
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
               | identifier '=' block                    # Minimal form: no parameter, no return

identifier ::= [a-zA-Z_][a-zA-Z0-9_]*

type_expr ::= identifier                     # Type reference
       | '()'                          # Void type
       | '(' parameters ')' '->' type_expr   # Function type (parameter names live in the signature)
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
           | 'return' expression         # Return statement (returns the given value)

# Note: Inside a code block, return must be used to return a value; without return, Void is returned by default
# For example: { return 1 + 1 } returns Int; { println("Hello") } returns Void
# Note: Generic parameters use the (T: Type) syntax as part of the function type; no separate BNF rule is required
```

### Error Handling

```yaoxiang
# === Compile-error examples ===

# Error 1: Code-block return type mismatch
add: (a: Int, b: Int) -> Int = { println(a + b) }
// Error: there is no return inside the block; the default return is Void, but the signature expects Int
// Correct: add: (a: Int, b: Int) -> Int = a + b
// Or:    add: (a: Int, b: Int) -> Int = { return a + b }

# Error 2: Using an undeclared type parameter
identity: (x: T) -> T = x
// Error: T is undeclared; explicit generic parameters are required (RFC-010)
// Correct: identity: (T: Type) -> ((x: T) -> T) = x

# Correct: the HM algorithm infers the return type
double = (x: Int) => x + x

# Full form (with progressive abbreviations)
double: (x: Int) -> Int = (x) => x + x                # Full
double: (x: Int) -> Int = x + x                       # Omit the lambda head
double = (x: Int) => x + x                            # Omit the return type (HM infers the return)
# double = (x) => x + x                               # ❌ Parameter types cannot be omitted on both sides
```

## Trade-offs

### Advantages

- **Syntactic unification**: the `name: Signature = LambdaBody` model covers all scenarios
- **Flexible abbreviations**: any part can be omitted whenever HM can infer it
- **Type safety**: the HM algorithm guarantees type safety and avoids implicit type conversions
- **Recursive support**: the HM algorithm and recursive constraints automatically infer types
- **Zero overhead**: a smooth transition from the full form to the minimal form

### Disadvantages

- **Migration cost**: old code requires migration tools to convert
- **Learning cost**: the "full form + arbitrary abbreviations" model must be understood

## Alternatives

| Proposal                   | Description                                            | Why not chosen                                                     |
| -------------------------- | ------------------------------------------------------ | ------------------------------------------------------------------ |
| HM type inference          | Use the Hindley-Milner algorithm to infer types        | ✅ **Adopted** — the standard for modern functional languages      |
| Explicit type declarations | All types must be written explicitly                   | Violates the principle of simplified syntax; increases boilerplate |
| Keep the old syntax        | Support both the old and new syntaxes                  | Splits the syntax; high maintenance cost                           |
| `fn` keyword               | Introduce `fn` to distinguish functions from variables | Violates the design that "functions are lambdas"                   |

## Implementation Strategy

### Phased Plan

1. **Phase 1: Syntax parsing and HM algorithm** (v0.3)
   - Implement the new syntax `name = lambda` + HM type inference
   - Implement default filling for no-parameter, no-return functions

2. **Phase 2: Migration tool** (v0.3)
   - Develop the `yaoxiang-migrate --old-to-new` tool
   - Automatically convert old-syntax code

3. **Phase 3: Validation and documentation** (v0.3)
   - Validate completion of the old-code migration
   - Update documentation

### Migration Tool

```bash
# Migrate a single file
yaoxiang-migrate --old-to-new src/main.yaoxiang

# Migrate an entire project
yaoxiang-migrate --old-to-new --recursive src/

# Preview the migration (do not modify files)
yaoxiang-migrate --old-to-new --dry-run src/main.yaoxiang
```

Migration rules:

```yaoxiang
# Old syntax
add(Int, Int) -> Int = (a, b) => { a + b }
main() -> Int = { println("Hello"); 0 }
main() = { println("Hello") }

# === New syntax: full form (signature complete + lambda head complete) ===
add: (a: Int, b: Int) -> Int = (a, b) => a + b
main: () -> Void = () => { println("Hello") }

# === Abbreviation: omit the lambda head ===
add: (a: Int, b: Int) -> Int = a + b
main: () -> Void = { println("Hello") }

# === Abbreviation: HM inference ===
add = (a: Int, b: Int) => a + b              # Inferred as (a: Int, b: Int) -> Int
main: () -> Void = { println("Hello") }                  # Inferred as () -> Void

# === Minimal form ===
main: () -> Void = {                                      # Equivalent to main: () -> Void = { ... }
    println("Hello")
}
```

### Dependencies

- No external dependencies
- Can be implemented independently

### Risks

| Risk             | Impact                    | Mitigation                                                   |
| ---------------- | ------------------------- | ------------------------------------------------------------ |
| Missed migration | Old code fails to compile | Provide a migration tool that covers all old syntax patterns |
| Parser errors    | Unstable syntax parsing   | Comprehensive test coverage                                  |

## Open Questions

> The following questions have already been resolved in the design and are recorded in Appendix A.

- ~~Q1: Should the extremely terse form `main() = body` be kept?~~ → Resolved: kept as
  `main: () -> Void = { ... }`
- ~~Q2: Should the `:` after the function name be kept?~~ → Resolved: optionally kept; however,
  functions with parameters still need to annotate parameter types in either the signature or the
  lambda head
- ~~Q3: Does the HM algorithm support parameter type inference?~~ → Resolved: return values and
  local variables are inferable; parameter types of functions with arguments must be annotated
  explicitly
- ~~Q4: Should the `fn` keyword be introduced?~~ → Resolved: not introduced; functions are lambdas
- ~~Q5: What is the migration strategy for old code?~~ → Resolved: provide the `yaoxiang-migrate`
  tool
- ~~Q6: How are generic functions used?~~ → Resolved: use the RFC-010 unified syntax `(T: Type)`

---

## Appendix

### Appendix A: Function Definition Syntax Reference Across Languages

| Language     | Syntax style                                        | Characteristics                             |
| ------------ | --------------------------------------------------- | ------------------------------------------- |
| Rust         | `fn add(a: i32, b: i32) -> i32 { ... }`             | Keyword + type annotation                   |
| Haskell      | `add a b = ...` / `add :: Int -> Int -> Int`        | Separate type signature                     |
| OCaml        | `let add a b = ...`                                 | Parameter types may be omitted              |
| MoonBit      | `fn add(a: Int, b: Int): Int { ... }`               | Concise type annotation                     |
| TypeScript   | `const add = (a: number, b: number): number => ...` | Lambda style                                |
| Scala        | `def add(a: Int, b: Int): Int = { ... }`            | `def` keyword                               |
| **YaoXiang** | `name = (a: Int, b: Int) => a + b`                  | **Function = lambda, HM infers the return** |

### Appendix B: Design Decision Records

| Decision              | Resolution                                                                            | Date       | Recorder   |
| --------------------- | ------------------------------------------------------------------------------------- | ---------- | ---------- |
| Syntax style          | New syntax `name: (params) -> Return = body` + HM inference                           | 2026-02-03 | @Moyujiang |
| Parameter position    | Parameter names are declared in the signature, unified with RFC-010                   | 2026-02-03 | @Moyujiang |
| Default filling       | No-parameter functions can omit the signature; empty block `{}` is inferred as `Void` | 2026-02-03 | @Moyujiang |
| Type inference        | The HM algorithm infers automatically; explicit when inference fails                  | 2026-01-06 | @Moyujiang |
| Old syntax            | Retired; a migration tool is provided                                                 | 2026-01-06 | @Moyujiang |
| `fn` keyword          | Not introduced                                                                        | 2026-01-06 | @Moyujiang |
| Recursive declaration | The HM algorithm and recursive constraints automatically infer types                  | 2026-01-06 | @Moyujiang |

### Appendix C: Glossary

| Term                  | Definition                                                                                                                    |
| --------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| HM algorithm          | The Hindley-Milner type inference algorithm, which automatically infers the types of functions and variables                  |
| Generics              | Use type parameters `(T: Type)` to constrain polymorphic functions, e.g. `identity: (T: Type) -> ((x: T) -> T) = x` (RFC-010) |
| Default type filling  | No-parameter, no-return functions omit `-> Void`; the compiler fills it in automatically                                      |
| Syntactic sugar       | A simplified syntax form that makes code easier to read                                                                       |
| Normalization         | Converting a syntactic form into a unified internal representation                                                            |
| Functions are lambdas | A function is essentially a lambda variable; the type is inferred automatically by the HM algorithm                           |

---

## References

- [MoonBit Language Design](https://moonbitlang.com/)
- [Rust Function Syntax](https://doc.rust-lang.org/book/ch03-03-how-functions-work.html)
- [Haskell Type System](https://www.haskell.org/tutorial/patterns.html)
- [OCaml Type Inference](https://v2.ocaml.org/manual/)
