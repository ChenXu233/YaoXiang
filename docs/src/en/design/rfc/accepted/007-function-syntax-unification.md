---
title: 'RFC-007: Unified Function Definition Syntax Proposal'
issue: '#131'
status: 'Accepted'
author: 'Mo Yu Jiang'
created: '2025-01-05'
updated: '2026-09-15'
---

# RFC-007: Unified Function Definition Syntax Proposal

> **Related**: At the overlap between the "no-param minimal" form `name = { ... }` and the "value
> block" syntax in RFC-010, the resolution is **content determines type**—`=>` is always a function,
> `Fn` annotation means a function, non-`Fn` annotation means a block value, and when there is no
> annotation, inference is done by content. See Appendix D of
> [RFC-010a](./010a-tail-expression-and-return.md).
>
> **Related supplement**: Statement termination and newline rules (`;` explicit separator, newline
> termination, continuation exception) inside function bodies (`{ ... }` code blocks) are defined by
> [RFC-038 (Draft)](./038-statement-termination.md) and are not covered by this RFC.

## Summary

This RFC finalizes the function definition syntax for the YaoXiang language. The unified syntax is
`name: (params) -> Return = body`, fully consistent with the `name: type = value` model in RFC-010.

To avoid ambiguity: when a function has input parameters, the parameter types must be explicitly
annotated in at least one of the "signature" or the "lambda head"; omitting them on both sides is
rejected.

The value of a code block `{ ... }` is given by the **tail expression**; `return` is a non-local
exit of type `Never` (see [RFC-010a](./010a-tail-expression-and-return.md)). The expression form
`= expr` directly provides the value.

## Motivation

### Why is this feature needed?

1. **Syntax consistency**: Eliminate the legacy baggage of the old syntax and unify the style
2. **Conciseness**: The HM algorithm auto-infers types, reducing boilerplate
3. **Type safety**: The HM algorithm guarantees type safety; explicit annotation is only required
   when inference fails
4. **Language maturity**: The HM algorithm is a mature solution in modern functional languages

### Unified Syntax Model

**Core principle**: `name: Signature = LambdaBody`

- **Full form**: Signature (with parameter names + types + `->` + return type) + Lambda head (with
  parameter names)
- **Abbreviation rules**: Omit as much as possible without introducing ambiguity
  - `->` cannot be omitted (it is the marker of function type; otherwise it would be parsed as a
    tuple)
  - **When there are input parameters**, parameter types must explicitly appear in at least one of
    the signature or the lambda head
  - Lambda head can be omitted → if the signature has already declared parameter names and types
  - The return type can be explicitly annotated or omitted when inferable

```yaoxiang
# Full form (complete signature + complete Lambda head)
add: (a: Int, b: Int) -> Int = (a, b) => a + b

# Abbreviation: omit the Lambda head (signature already declares parameters)
add: (a: Int, b: Int) -> Int = a + b

# Abbreviation: omit the signature (lambda head annotates parameter types)
add = (a: Int, b: Int) => a + b

# ❌ Error: parameter types omitted on both sides
# add = (a, b) => a + b
```

### Design Goals

```yaoxiang
# === Full form ===
add: (a: Int, b: Int) -> Int = (a, b) => { a + b }

# === Abbreviated form ===
add: (a: Int, b: Int) -> Int = a + b                 # Omit Lambda head
add = (a: Int, b: Int) => a + b                      # Omit signature

# === No-parameter function ===
main: () -> Void = () => { println("Hello") }          # Full form
main: () -> Void = { println("Hello") }                # Omit Lambda head
main: () -> Void = { println("Hello") }                            # Minimal form (inferred as () -> Void)

# === Generic function (using the RFC-010 unified syntax) ===
identity: (T: Type) -> ((x: T) -> T) = (x) => x         # Full form
identity: (T: Type) -> ((x: T) -> T) = x                # Omit Lambda head
identity = (x: T) => x                                  # Omit signature (lambda head annotates types)

# === Recursive function ===
factorial: (n: Int) -> Int = (n) => {
    if n <= 1 { return 1 } else { return n * factorial(n - 1) }
}
```

### Syntax Rules

| Scenario                 | Syntax                                                 | Description                           |
| ------------------------ | ------------------------------------------------------ | ------------------------------------- |
| **Full form**            | `name: (a: Type, b) -> Ret = (a, b) => { return ... }` | Signature + Lambda head complete      |
| **Omit Lambda head**     | `name: (a: Type, b: Type) -> Ret = { ... }`            | Signature already declares parameters |
| **Omit signature**       | `name = (a: Type, b: Type) => { ... }`                 | Lambda head annotates parameter types |
| **No-param full**        | `name: () -> Void = () => { return ... }`              | No-parameter function, full form      |
| **No-param abbreviated** | `name: () -> Void = { return ... }`                    | Omit Lambda head                      |
| **No-param minimal**     | `name = { return ... }`                                | No params, no return, minimal         |

**Note**: The value of a code block `{ ... }` is given by the **tail expression** (the sole exit);
`return` is a non-local exit of type `Never`, exiting the nearest function boundary. The expression
form `= expr` directly provides the value. See [RFC-010a](./010a-tail-expression-and-return.md) for
details.

**Note**: `->` is the marker of function type and cannot be omitted (otherwise it would be parsed as
a tuple).

**Important**: `if` expressions use curly braces `{}` to wrap branches; `then/else` keywords are not
supported:

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

- **Higher-order functions**: When a function is passed as a parameter, a generic type is needed to
  constrain its function type
- **Type annotation form**: `(T: Type) -> ((f: (T) -> T, x: T) -> T)` — generic parameters constrain
  function types
- **HM workflow**: Infer function types through generic instantiation, enabling polymorphic function
  composition

**Example illustration**:

```yaoxiang
# ✅ Supports higher-rank polymorphism: generics constrain function-typed parameters
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = {
    return f(f(x))
}
# Usage: call_twice((x) => x + 1, 5)  # Infers T=Int

compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = {
    return f(g(x))
}
# Usage: compose((x) => x * 2, (x) => x + 1, 5)  # Infers A=Int, B=Int, C=Int

# ❌ Not supported: higher-order function without generic constraints
# bad_hof: (f, x) => f(f(x))  # HM cannot infer; generic parameters are missing
```

**HM inference process**:

1. Identify higher-order function parameters: `f: (T) -> T`
2. Create a generic constraint: `(T: Type)`
3. Infer concrete types through generic instantiation
4. Achieve polymorphic function composition

### Lambda Expression Syntax Rules

**Important rule**: The value of a code block `{ ... }` is given by the **tail expression** (the
sole exit); `return` is a non-local exit of type `Never`, exiting the nearest function boundary. The
expression form `= expr` directly provides the value. See
[RFC-010a](./010a-tail-expression-and-return.md) for details.

| Syntax form         | Syntax           | Value exit                                   |
| ------------------- | ---------------- | -------------------------------------------- |
| **Code block form** | `{ statements }` | Tail expression (empty block `{}` is `Void`) |
| **Expression form** | `expression`     | Expression value                             |
| **`return`**        | `return e`       | Non-local exit from function, type `Never`   |

**Example**:

```yaoxiang
main: () -> Void = { println("Hello") }         # Tail expression is Void
add: (a: Int, b: Int) -> Int = { a + b }        # Tail expression provides the value
empty: () -> Void = {}                          # Empty block → Void

# Early return: use return
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

# Expression form: directly provides the value
add: (a: Int, b: Int) -> Int = a + b            # Correct: expression form
main: () -> Void = println("Hello")               # Correct: expression form
```

**Core ideas**:

1. Function definitions use the HM algorithm for type inference, inferring as much as possible and
   reporting an explicit error when inference fails
2. **How the HM algorithm works**: It auto-infers types through operator type constraints, function
   call relationships, and other contextual information
3. **Generics support**: Polymorphic functions use generic syntax `(T: Type)` to explicitly
   constrain type parameters (RFC-010/011)
4. **Inference boundaries**: Return types and local variables are inferable; parameter types of
   functions with parameters must be explicitly annotated (in either the signature or the lambda
   head)
5. No-parameter, no-return functions use `name: () -> Void = { ... }`, consistent with RFC-010
6. The old syntax is deprecated, and a migration tool is provided

**Type inference examples**:

```yaoxiang
# Generic function: explicit type parameters (using the RFC-010 unified syntax)
identity: (T: Type) -> ((x: T) -> T) = x
map: (T: Type, R: Type) -> ((f: (T) -> R, list: List(T)) -> List(R)) = {
    result = List(R)()
    for item in list { result.push(f(item)) }
    return result
}

# Polymorphic function: defined via explicit generic constraints (RFC-010/011)
add: (T: Add) -> ((a: T, b: T) -> T) = a + b
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # Inferred as (Int, Int) -> Void

# Higher-rank polymorphism: achieved via generic type annotations and the HM algorithm
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = { return f(f(x)) }
compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = { return f(g(x)) }
```

```yaoxiang
# === Function definition: HM algorithm type inference ===

# Standard function: HM algorithm infers the return type (parameter types must be explicit)
add = (a: Int, b: Int) => a + b            # Inferred as (a: Int, b: Int) -> Int
main: () -> Void = { println("Hello") }                # Inferred as () -> Void

# Partially explicit parameters: HM algorithm infers the rest
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # Inferred as (Int, Int) -> Void
greet: (name: String) -> Void = { println("Hello " + name) }  # Inferred as (String) -> Void

# Generic function: explicitly constrain polymorphic type parameters (using the RFC-010 unified syntax)
identity: (T: Type) -> ((x: T) -> T) = x
map: (T: Type, R: Type) -> ((f: (T) -> R, list: List(T)) -> List(R)) = {
    # Implementation of the map function
    return List(R)()
}

# Recursive function: inferred through the HM algorithm and recursive constraints
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 } else { return n * factorial(n - 1) }
}

# === Variable assignment: HM algorithm type inference ===

# Explicit type
x: Int = 42

# HM algorithm auto-infers as Int
y = 42                               # Inferred as Int

# HM algorithm auto-infers as String
name = "YaoXiang"                    # Inferred as String

# HM algorithm auto-infers as Float
pi = 3.14159                         # Inferred as Float
```

**HM type inference rules**:

| Scenario                 | Syntax                                            | Omissible part | Example                               |
| ------------------------ | ------------------------------------------------- | -------------- | ------------------------------------- |
| **Full form**            | `name: (a: Type, b: Type) -> Ret = (a, b) => ...` | None           | Signature + Lambda head complete      |
| **Omit Lambda head**     | `name: (a: Type, b: Type) -> Ret = ...`           | Lambda head    | Signature already declares parameters |
| **Omit signature**       | `name = (a: Type, b: Type) => ...`                | Signature      | Lambda head provides parameter types  |
| **Omit return Ret**      | `name: (a: Type, b: Type) -> = ...`               | Return type    | HM infers the return type             |
| **No-param full**        | `name: () -> Void = () => { ... }`                | None           | No-parameter function, full form      |
| **No-param abbreviated** | `name: () -> Void = { ... }`                      | Lambda head    | Omit `() =>`                          |
| **No-param minimal**     | `name = { ... }`                                  | All            | No params, no return, minimal         |
| **Variable assignment**  | `name = value`                                    | Type           | HM infers the type                    |
| **Explicit variable**    | `name: Type = value`                              | None           | Explicit type annotation              |

**Core principles**:

- `->` is the marker of function type and cannot be omitted (otherwise it would be parsed as a
  tuple)
- The return type `Ret` can be omitted, inferred by HM from the function body
- When input parameters exist, parameter types must explicitly appear (in either the signature or
  the lambda head)
- Other parts can be omitted when inferable and not introducing ambiguity
- No implicit type conversions, avoiding JavaScript-style chaos

## Detailed Design

### Syntactic Sugar Desugaring

Regardless of omissions, everything is normalized to a unified intermediate representation:

```rust
// Full form
add: (a: Int, b: Int) -> Int = (a, b) => a + b

// Desugared IR
let add: (Int, Int) -> Int = |a: Int, b: Int| -> Int {
    return a + b
};

// Omit Lambda head
add: (a: Int, b: Int) -> Int = a + b

// Desugared IR (same as the full form)
let add: (Int, Int) -> Int = |a: Int, b: Int| -> Int {
    return a + b
};

// Omit signature (lambda head annotates parameter types)
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
               | identifier '=' block                    # Minimal form: no params, no return

identifier ::= [a-zA-Z_][a-zA-Z0-9_]*

type_expr ::= identifier                     # Type reference
       | '()'                          # Unit type
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

# Note: Inside a code block, return must be used to return a value; without return, the default return is Void
# Example: { return 1 + 1 } returns Int; { println("Hello") } returns Void
# Note: Generic parameters use the (T: Type) syntax as part of the function type and need no separate BNF rule
```

### Error Handling

```yaoxiang
# === Compile error examples ===

# Error 1: code block return type mismatch
add: (a: Int, b: Int) -> Int = { println(a + b) }
// Error: no return inside the block, default returns Void, but the signature expects Int
// Correct: add: (a: Int, b: Int) -> Int = a + b
// Or:    add: (a: Int, b: Int) -> Int = { return a + b }

# Error 2: using an undeclared type parameter
identity: (x: T) -> T = x
// Error: T is not declared; explicit generic parameters are required (RFC-010)
// Correct: identity: (T: Type) -> ((x: T) -> T) = x

# Correct: HM algorithm infers the return type
double = (x: Int) => x + x

# Full form (progressive abbreviations)
double: (x: Int) -> Int = (x) => x + x                # Full
double: (x: Int) -> Int = x + x                       # Omit Lambda head
double = (x: Int) => x + x                            # Omit return type (HM infers the return)
# double = (x) => x + x                               # ❌ Parameter types are not allowed to be omitted on both sides
```

## Trade-offs

### Advantages

- **Syntax unification**: The `name: Signature = LambdaBody` model covers all scenarios
- **Flexible abbreviations**: Any part can be omitted when HM can infer it
- **Type safety**: The HM algorithm guarantees type safety and avoids implicit type conversions
- **Recursive support**: The HM algorithm and recursive constraints auto-infer types
- **Zero burden**: Smooth transition from full to minimal form

### Disadvantages

- **Migration cost**: Old code requires a migration tool to convert
- **Learning cost**: Need to understand the "full form + arbitrary abbreviations" model

## Alternatives

| Option                    | Description                                            | Why not chosen                                              |
| ------------------------- | ------------------------------------------------------ | ----------------------------------------------------------- |
| HM type inference         | Use the Hindley-Milner algorithm for type inference    | ✅ **Adopted**, the standard in modern functional languages |
| Explicit type declaration | All types must be written explicitly                   | Violates the simplified-syntax principle, adds boilerplate  |
| Keep the old syntax       | Support both old and new syntax simultaneously         | Syntax split, high maintenance cost                         |
| `fn` keyword              | Introduce `fn` to distinguish functions from variables | Violates the "function is a lambda" design                  |

## Implementation Strategy

### Phases

1. **Phase 1: Syntax parsing and HM algorithm** (v0.3)
   - Implement the new syntax `name = lambda` + the HM algorithm for type inference
   - Implement default filling for no-param, no-return functions

2. **Phase 2: Migration tool** (v0.3)
   - Develop the `yaoxiang-migrate --old-to-new` tool
   - Automatically convert old-syntax code

3. **Phase 3: Validation and documentation** (v0.3)
   - Verify completion of old-code migration
   - Update documentation

### Migration Tool

```bash
# Migrate a single file
yaoxiang-migrate --old-to-new src/main.yaoxiang

# Migrate an entire project
yaoxiang-migrate --old-to-new --recursive src/

# Preview migration (does not modify files)
yaoxiang-migrate --old-to-new --dry-run src/main.yaoxiang
```

Migration rules:

```yaoxiang
# Old syntax
add(Int, Int) -> Int = (a, b) => { a + b }
main() -> Int = { println("Hello"); 0 }
main() = { println("Hello") }

# === New syntax: full form (complete signature + complete Lambda head) ===
add: (a: Int, b: Int) -> Int = (a, b) => a + b
main: () -> Void = () => { println("Hello") }

# === Abbreviation: omit the Lambda head ===
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
| Parser errors    | Unstable syntax parsing   | Thorough test coverage                                       |

## Open Questions

> The following questions have been resolved during design and are recorded in Appendix A.

- ~~Q1: Should the extremely concise form `main() = body` be retained?~~ → Resolved: Retain it as
  `main: () -> Void = { ... }`
- ~~Q2: Should the `:` after the function name be kept?~~ → Resolved: Optionally retained; however,
  functions with parameters still need to annotate parameter types in the signature or the lambda
  head
- ~~Q3: Does the HM algorithm support parameter type inference?~~ → Resolved: Return values/locals
  are inferable; functions with parameters must explicitly annotate parameter types
- ~~Q4: Should the `fn` keyword be introduced?~~ → Resolved: Not introduced; a function is a lambda
- ~~Q5: What is the migration strategy for old code?~~ → Resolved: Provide the `yaoxiang-migrate`
  tool
- ~~Q6: How are generic functions used?~~ → Resolved: Use the RFC-010 unified syntax `(T: Type)`

---

## Appendices

### Appendix A: Reference of Function Definition Syntax in Various Languages

| Language     | Syntax style                                        | Characteristics                         |
| ------------ | --------------------------------------------------- | --------------------------------------- |
| Rust         | `fn add(a: i32, b: i32) -> i32 { ... }`             | Keyword + type annotation               |
| Haskell      | `add a b = ...` / `add :: Int -> Int -> Int`        | Type signature separated                |
| OCaml        | `let add a b = ...`                                 | Parameter types can be omitted          |
| MoonBit      | `fn add(a: Int, b: Int): Int { ... }`               | Concise type annotation                 |
| TypeScript   | `const add = (a: number, b: number): number => ...` | Lambda style                            |
| Scala        | `def add(a: Int, b: Int): Int = { ... }`            | `def` keyword                           |
| **YaoXiang** | `name = (a: Int, b: Int) => a + b`                  | **Function = lambda, HM infers return** |

### Appendix B: Design Decision Records

| Decision              | Decision                                                                           | Date       | Recorder     |
| --------------------- | ---------------------------------------------------------------------------------- | ---------- | ------------ |
| Syntax style          | New syntax `name: (params) -> Return = body` + HM inference                        | 2026-02-03 | @Mo Yu Jiang |
| Parameter position    | Parameter names declared in the signature, consistent with RFC-010                 | 2026-02-03 | @Mo Yu Jiang |
| Default filling       | No-parameter functions can omit the signature; empty block `{}` inferred as `Void` | 2026-02-03 | @Mo Yu Jiang |
| Type inference        | HM algorithm auto-infers; explicit when inference fails                            | 2026-01-06 | @Mo Yu Jiang |
| Old syntax            | Deprecated, migration tool provided                                                | 2026-01-06 | @Mo Yu Jiang |
| `fn` keyword          | Not introduced                                                                     | 2026-01-06 | @Mo Yu Jiang |
| Recursive declaration | HM algorithm and recursive constraints auto-infer                                  | 2026-01-06 | @Mo Yu Jiang |

### Appendix C: Glossary

| Term                 | Definition                                                                                                                     |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| HM algorithm         | The Hindley-Milner type inference algorithm, which auto-infers types for functions and variables                               |
| Generics             | Use type parameters `(T: Type)` to constrain polymorphic functions, e.g., `identity: (T: Type) -> ((x: T) -> T) = x` (RFC-010) |
| Default type filling | No-param, no-return functions omit `-> Void`, and the compiler fills it in automatically                                       |
| Syntactic sugar      | Syntactic simplifications that make code easier to read                                                                        |
| Normalization        | Convert syntactic forms into a unified internal representation                                                                 |
| Function is a lambda | A function is essentially a lambda variable, with types auto-inferred by the HM algorithm                                      |

---

## References

- [MoonBit Language Design](https://moonbitlang.com/)
- [Rust Function Syntax](https://doc.rust-lang.org/book/ch03-03-how-functions-work.html)
- [Haskell Type System](https://www.haskell.org/tutorial/patterns.html)
- [OCaml Type Inference](https://v2.ocaml.org/manual/)
