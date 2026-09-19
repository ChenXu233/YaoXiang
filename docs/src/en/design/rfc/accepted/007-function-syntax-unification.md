---
title: 'RFC-007: Unified Function Definition Syntax Proposal'
issue: '#131'
status: 'Accepted'
author: 'Mo Yu Jiang'
created: '2025-01-05'
updated: '2026-09-15 (Return semantics corrected per RFC-010a)'
---

# RFC-007: Unified Function Definition Syntax Proposal

> **Erratum (2026-09-15, RFC-010a)**: This document previously stated that "code blocks must use
> `return` to return a value, and default to returning `Void` when `return` is absent." That
> statement is hereby abolished — **the value of a block = its tail expression**, and `return` is a
> `Never`-typed non-local exit. See [RFC-010a](./010a-tail-expression-and-return.md). The function
> form definitions in this document (full form / omitted lambda head / omitted signature / empty
> parameter form) **remain unchanged**, and the "early return" semantics were already consistent
> with this RFC.
>
> **Addendum (Adjudication C)**: The "no-parameter minimal" form `name = { ... }` in this document
> overlaps syntactically with the "valued block" form in RFC-010. The adjudication is **annotation
> priority, function by default** — when no annotation is present, the "no-parameter minimal" form
> here applies (it is a function); only when a non-`Fn` annotation is written does it become a block
> value (e.g., `x: Int = { ... }`). See Appendix D of RFC-010a.
>
> **Related Addendum**: Statement termination and newline rules inside function bodies (`{ ... }`
> blocks) — explicit `;` separation, newline termination, and continuation-line exceptions — are
> defined by [RFC-038 (draft)](../draft/038-statement-termination.md), and are not addressed in this
> RFC.

## Summary

This RFC establishes the final proposal for the **function definition syntax** of the YaoXiang
language. It uses the unified syntax `name: (params) -> Return = body`, fully consistent with the
`name: type = value` model of RFC-010.

To avoid ambiguity, when a function has input parameters, the parameter types must be explicitly
annotated in **at least one** of the "signature" or "lambda head"; omitting both sides will be
rejected.

The value of a code block `{ ... }` is given by its **tail expression**; `return` is a `Never`-typed
non-local exit (see [RFC-010a](./010a-tail-expression-and-return.md)). The expression form `= expr`
provides the value directly.

## Motivation

### Why is this feature needed?

1. **Syntax consistency**: Eliminate historical baggage from old syntax and unify the style
2. **Conciseness**: HM algorithm automatically infers types, reducing boilerplate code
3. **Type safety**: The HM algorithm guarantees type safety; explicit annotation is required only
   when inference fails
4. **Language maturity**: The HM algorithm is a mature solution in modern functional languages

### Unified Syntax Model

**Core principle**: `name: Signature = LambdaBody`

- **Full form**: Signature (containing parameter names + types + `->` + return type) + Lambda head
  (containing parameter names)
- **Abbreviation rules**: Omit as much as possible without introducing ambiguity
  - `->` cannot be omitted (it is the marker of a function type; otherwise it would be parsed as a
    tuple)
  - **When there are input parameters**, parameter types must appear explicitly in at least one of
    the signature or the lambda head
  - The lambda head can be omitted → if the signature has already declared the parameter names and
    types
  - The return type can be explicitly annotated, or omitted when inferable

```yaoxiang
# Full form (complete signature + complete lambda head)
add: (a: Int, b: Int) -> Int = (a, b) => a + b

# Abbreviation: omitted lambda head (signature already declares parameters)
add: (a: Int, b: Int) -> Int = a + b

# Abbreviation: omitted signature (lambda head annotates parameter types)
add = (a: Int, b: Int) => a + b

# ❌ Error: parameter types not annotated on either side
# add = (a, b) => a + b
```

### Design Goals

```yaoxiang
# === Full form ===
add: (a: Int, b: Int) -> Int = (a, b) => { a + b }

# === Abbreviated form ===
add: (a: Int, b: Int) -> Int = a + b                 # omitted lambda head
add = (a: Int, b: Int) => a + b                      # omitted signature

# === No-parameter function ===
main: () -> Void = () => { println("Hello") }          # full form
main: () -> Void = { println("Hello") }                # omitted lambda head
main: () -> Void = { println("Hello") }                            # minimal form (inferred as () -> Void)

# === Generic function (using RFC-010 unified syntax) ===
identity: (T: Type) -> ((x: T) -> T) = (x) => x         # full form
identity: (T: Type) -> ((x: T) -> T) = x                # omitted lambda head
identity = (x: T) => x                                  # omitted signature (lambda head annotates types)

# === Recursive function ===
factorial: (n: Int) -> Int = (n) => {
    if n <= 1 { return 1 } else { return n * factorial(n - 1) }
}
```

### Syntax Rules

| Scenario                 | Syntax                                                 | Description                           |
| ------------------------ | ------------------------------------------------------ | ------------------------------------- |
| **Full form**            | `name: (a: Type, b) -> Ret = (a, b) => { return ... }` | signature + lambda head complete      |
| **Omitted lambda head**  | `name: (a: Type, b: Type) -> Ret = { ... }`            | signature has declared parameters     |
| **Omitted signature**    | `name = (a: Type, b: Type) => { ... }`                 | lambda head annotates parameter types |
| **No-parameter full**    | `name: () -> Void = () => { return ... }`              | no-parameter function full form       |
| **No-parameter short**   | `name: () -> Void = { return ... }`                    | omitted lambda head                   |
| **No-parameter minimal** | `name = { return ... }`                                | minimal no-parameter, no-return       |

**Note**: The value of a code block `{ ... }` is given by its **tail expression** (the sole outlet);
`return` is a `Never`-typed non-local exit that exits the nearest function boundary. The expression
form `= expr` provides the value directly. See [RFC-010a](./010a-tail-expression-and-return.md).

**Note**: `->` is the marker of a function type and cannot be omitted (otherwise it would be parsed
as a tuple).

**Important**: `if` expressions use curly braces `{}` to enclose branches, and do not support
`then/else` keywords:

```yaoxiang
# Correct: uses curly braces
if n <= 1 { return 1 } else { return n * factorial(n - 1) }

# Wrong: then/else keywords are not supported
# if n <= 1 then return 1 else return n * factorial(n - 1)
```

## Proposal

### HM Algorithm and Higher-Rank Polymorphism Support

**Core feature**: The HM algorithm supports higher-rank polymorphism through generic type
annotations

**Design rationale**:

- **Higher-order functions**: When functions are passed as arguments, generics are needed to
  constrain their function types
- **Type annotation form**: `(T: Type) -> ((f: (T) -> T, x: T) -> T)` — generic parameters constrain
  function types
- **HM workflow**: Function types are inferred through generic parameter instantiation, enabling
  polymorphic function composition

**Example explanation**:

```yaoxiang
# ✅ Higher-rank polymorphism supported: generics constrain function-type arguments
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = {
    return f(f(x))
}
# Usage: call_twice((x) => x + 1, 5)  # infers T=Int

compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = {
    return f(g(x))
}
# Usage: compose((x) => x * 2, (x) => x + 1, 5)  # infers A=Int, B=Int, C=Int

# ❌ Not supported: higher-order function without generic constraint
# bad_hof: (f, x) => f(f(x))  # HM cannot infer, generic parameter is missing
```

**HM inference process**:

1. Identify the higher-order function parameter: `f: (T) -> T`
2. Create a generic constraint: `(T: Type)`
3. Infer concrete types through generic instantiation
4. Achieve polymorphic function composition

### Lambda Expression Syntax Rules

**Important rule**: The value of a code block `{ ... }` is given by its **tail expression** (the
sole outlet); `return` is a `Never`-typed non-local exit that exits the nearest function boundary.
The expression form `= expr` provides the value directly. See
[RFC-010a](./010a-tail-expression-and-return.md).

| Syntax form         | Syntax           | Value outlet                                 |
| ------------------- | ---------------- | -------------------------------------------- |
| **Code block form** | `{ statements }` | Tail expression (empty block `{}` is `Void`) |
| **Expression form** | `expression`     | Expression value                             |
| **`return`**        | `return e`       | Non-local exit from function, type `Never`   |

**Examples**:

```yaoxiang
main: () -> Void = { println("Hello") }         # tail expression is Void
add: (a: Int, b: Int) -> Int = { a + b }        # tail expression provides the value
empty: () -> Void = {}                          # empty block → Void

# Early return: use return
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

# Expression form: provides the value directly
add: (a: Int, b: Int) -> Int = a + b            # correct: expression form
main: () -> Void = println("Hello")               # correct: expression form
```

**Core ideas**:

1. Function definitions rely on the HM algorithm for type inference — infer whenever possible, and
   raise an explicit error when inference is impossible
2. **How the HM algorithm works**: It automatically infers types from context such as operator type
   constraints and function call relationships
3. **Generics support**: Polymorphic functions use the generic syntax `(T: Type)` to explicitly
   constrain type parameters (RFC-010/011)
4. **Inference boundary**: Return types and local variables are inferable; parameter types of
   functions with parameters must be explicitly annotated (in either the signature or the lambda
   head)
5. No-parameter, no-return functions use `name: () -> Void = { ... }`, unified with RFC-010
6. The old syntax is retired, and a migration tool is provided

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
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # inferred as (Int, Int) -> Void

# Higher-rank polymorphism: HM support through generic type annotations
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = { return f(f(x)) }
compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = { return f(g(x)) }
```

```yaoxiang
# === Function definition: HM algorithm type inference ===

# Standard function: HM algorithm infers the return type (parameter types must be explicit)
add = (a: Int, b: Int) => a + b            # inferred as (a: Int, b: Int) -> Int
main: () -> Void = { println("Hello") }                # inferred as () -> Void

# Partially explicit parameters: HM algorithm infers the rest
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # inferred as (Int, Int) -> Void
greet: (name: String) -> Void = { println("Hello " + name) }  # inferred as (String) -> Void

# Generic function: explicitly constrain polymorphic type parameters (using RFC-010 unified syntax)
identity: (T: Type) -> ((x: T) -> T) = x
map: (T: Type, R: Type) -> ((f: (T) -> R, list: List(T)) -> List(R)) = {
    # implementation of map
    return List(R)()
}

# Recursive function: inferred through HM algorithm and recursive constraints
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 } else { return n * factorial(n - 1) }
}

# === Variable assignment: HM algorithm type inference ===

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

| Scenario                 | Syntax                                            | Omissible part | Example                              |
| ------------------------ | ------------------------------------------------- | -------------- | ------------------------------------ |
| **Full form**            | `name: (a: Type, b: Type) -> Ret = (a, b) => ...` | none           | signature + lambda head complete     |
| **Omitted lambda head**  | `name: (a: Type, b: Type) -> Ret = ...`           | lambda head    | signature has declared parameters    |
| **Omitted signature**    | `name = (a: Type, b: Type) => ...`                | signature      | lambda head provides parameter types |
| **Omitted return Ret**   | `name: (a: Type, b: Type) -> = ...`               | return type    | HM infers return type                |
| **No-parameter full**    | `name: () -> Void = () => { ... }`                | none           | no-parameter function full form      |
| **No-parameter short**   | `name: () -> Void = { ... }`                      | lambda head    | omit `() =>`                         |
| **No-parameter minimal** | `name = { ... }`                                  | all            | minimal no-parameter, no-return      |
| **Variable assignment**  | `name = value`                                    | type           | HM infers type                       |
| **Explicit variable**    | `name: Type = value`                              | none           | explicit type annotation             |

**Core principles**:

- `->` is the marker of a function type and cannot be omitted (otherwise it would be parsed as a
  tuple)
- The return type `Ret` can be omitted, and is inferred by HM from the function body
- When input parameters exist, parameter types must appear explicitly (in either the signature or
  the lambda head)
- The remaining parts can be omitted when inferable and unambiguous
- No implicit type conversions, avoiding JavaScript-style confusion

## Detailed Design

### Syntactic Sugar Desugaring

Regardless of omissions, the result is normalized to a unified intermediate representation:

```rust
// Full form
add: (a: Int, b: Int) -> Int = (a, b) => a + b

// Desugared IR
let add: (Int, Int) -> Int = |a: Int, b: Int| -> Int {
    return a + b
};

// Omitted lambda head
add: (a: Int, b: Int) -> Int = a + b

// Desugared IR (same as full form)
let add: (Int, Int) -> Int = |a: Int, b: Int| -> Int {
    return a + b
};

// Omitted signature (lambda head annotates parameter types)
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
               | identifier '=' block                    # minimal form: no-parameter, no-return

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
           | expression                  # expression statement (executed but not returned)
           | 'return' expression         # return statement (returns the specified value)

# Note: code blocks must use return to return a value; when return is absent, Void is returned by default
# For example: { return 1 + 1 } returns Int; { println("Hello") } returns Void
# Note: generic parameters use the (T: Type) syntax, as part of the function type, with no independent BNF rule required
```

### Error Handling

```yaoxiang
# === Compilation error examples ===

# Error 1: code block return type mismatch
add: (a: Int, b: Int) -> Int = { println(a + b) }
// Error: the block has no return, defaults to returning Void, but the signature expects Int
// Correct: add: (a: Int, b: Int) -> Int = a + b
// Or:     add: (a: Int, b: Int) -> Int = { return a + b }

# Error 2: using an undeclared type parameter
identity: (x: T) -> T = x
// Error: T is undeclared; explicit generic parameters (RFC-010) are required
// Correct: identity: (T: Type) -> ((x: T) -> T) = x

# Correct: HM algorithm infers return type
double = (x: Int) => x + x

# Full form (stepwise abbreviation)
double: (x: Int) -> Int = (x) => x + x                # full
double: (x: Int) -> Int = x + x                       # omitted lambda head
double = (x: Int) => x + x                            # omitted return type (HM infers return)
# double = (x) => x + x                               # ❌ omitting parameter types on both sides is not allowed
```

## Trade-offs

### Advantages

- **Unified syntax**: The `name: Signature = LambdaBody` model covers all scenarios
- **Flexible abbreviations**: Any part can be omitted when inferable by HM
- **Type safety**: The HM algorithm guarantees type safety and avoids implicit type conversions
- **Recursive support**: The HM algorithm and recursive constraints automatically infer types
- **Zero overhead**: Smooth transition from full to minimal form

### Disadvantages

- **Migration cost**: Old code requires a migration tool to convert
- **Learning cost**: One must understand the "full form + arbitrary abbreviation" model

## Alternatives

| Proposal                    | Description                                            | Why not chosen                                                   |
| --------------------------- | ------------------------------------------------------ | ---------------------------------------------------------------- |
| HM algorithm type inference | Use the Hindley-Milner algorithm to infer types        | ✅ **Adopted**, the standard for modern functional languages     |
| Explicit type declaration   | All types must be written explicitly                   | Violates the principle of simplified syntax and adds boilerplate |
| Retain old syntax           | Support both old and new syntax simultaneously         | Splits the syntax and increases maintenance cost                 |
| `fn` keyword                | Introduce `fn` to distinguish functions from variables | Violates the "function is a lambda" design                       |

## Implementation Strategy

### Phased Plan

1. **Phase 1: Syntax parsing and HM algorithm** (v0.3)
   - Implement the new syntax `name = lambda` + HM algorithm type inference
   - Implement default filling for no-parameter, no-return cases

2. **Phase 2: Migration tool** (v0.3)
   - Develop the `yaoxiang-migrate --old-to-new` tool
   - Automatically convert old-syntax code

3. **Phase 3: Validation and documentation** (v0.3)
   - Validate the completion of old code migration
   - Update documentation

### Migration Tool

```bash
# Migrate a single file
yaoxiang-migrate --old-to-new src/main.yaoxiang

# Migrate an entire project
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

# === New syntax: full form (complete signature + complete lambda head) ===
add: (a: Int, b: Int) -> Int = (a, b) => a + b
main: () -> Void = () => { println("Hello") }

# === Abbreviation: omitted lambda head ===
add: (a: Int, b: Int) -> Int = a + b
main: () -> Void = { println("Hello") }

# === Abbreviation: HM inference ===
add = (a: Int, b: Int) => a + b              # inferred as (a: Int, b: Int) -> Int
main: () -> Void = { println("Hello") }                  # inferred as () -> Void

# === Minimal form ===
main: () -> Void = {                                      # equivalent to main: () -> Void = { ... }
    println("Hello")
}
```

### Dependencies

- No external dependencies
- Can be implemented independently

### Risks

| Risk           | Impact                    | Mitigation                                                |
| -------------- | ------------------------- | --------------------------------------------------------- |
| Migration gaps | Old code fails to compile | Provide a migration tool covering all old syntax patterns |
| Parser bugs    | Unstable syntax parsing   | Sufficient test coverage                                  |

## Open Questions

> The following questions have been resolved in the design and are recorded in Appendix A.

- ~~Q1: Should the extremely compact form `main() = body` be retained?~~ → Resolved: retained as
  `main: () -> Void = { ... }`
- ~~Q2: Should the `:` after a function name be retained?~~ → Resolved: optionally retained;
  however, functions with parameters still need parameter types annotated in the signature or lambda
  head
- ~~Q3: Does the HM algorithm support parameter type inference?~~ → Resolved: return values / locals
  are inferable; parameter types of functions with parameters must be explicitly annotated
- ~~Q4: Should the `fn` keyword be introduced?~~ → Resolved: not introduced — a function is a lambda
- ~~Q5: What is the migration strategy for old code?~~ → Resolved: provide the `yaoxiang-migrate`
  tool
- ~~Q6: How are generic functions used?~~ → Resolved: use the RFC-010 unified syntax `(T: Type)`

---

## Appendices

### Appendix A: Function Definition Syntax Reference Across Languages

| Language     | Syntax style                                        | Features                                      |
| ------------ | --------------------------------------------------- | --------------------------------------------- |
| Rust         | `fn add(a: i32, b: i32) -> i32 { ... }`             | Keyword + type annotation                     |
| Haskell      | `add a b = ...` / `add :: Int -> Int -> Int`        | Separate type signature                       |
| OCaml        | `let add a b = ...`                                 | Parameter types can be omitted                |
| MoonBit      | `fn add(a: Int, b: Int): Int { ... }`               | Concise type annotation                       |
| TypeScript   | `const add = (a: number, b: number): number => ...` | Lambda style                                  |
| Scala        | `def add(a: Int, b: Int): Int = { ... }`            | `def` keyword                                 |
| **YaoXiang** | `name = (a: Int, b: Int) => a + b`                  | **Function = lambda, HM infers return value** |

### Appendix B: Design Decision Records

| Decision              | Decision                                                                           | Date       | Recorder     |
| --------------------- | ---------------------------------------------------------------------------------- | ---------- | ------------ |
| Syntax style          | New syntax `name: (params) -> Return = body` + HM inference                        | 2026-02-03 | @Mo Yu Jiang |
| Parameter position    | Parameter names declared in the signature, unified with RFC-010                    | 2026-02-03 | @Mo Yu Jiang |
| Default filling       | No-parameter functions can omit the signature; empty block `{}` inferred as `Void` | 2026-02-03 | @Mo Yu Jiang |
| Type inference        | HM algorithm automatically infers; explicit when impossible                        | 2026-01-06 | @Mo Yu Jiang |
| Old syntax            | Retired, migration tool provided                                                   | 2026-01-06 | @Mo Yu Jiang |
| `fn` keyword          | Not introduced                                                                     | 2026-01-06 | @Mo Yu Jiang |
| Recursive declaration | HM algorithm and recursive constraints automatically infer                         | 2026-01-06 | @Mo Yu Jiang |

### Appendix C: Glossary

| Term                 | Definition                                                                                                                       |
| -------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| HM algorithm         | The Hindley-Milner type inference algorithm, which automatically infers function and variable types                              |
| Generics             | Using type parameters `(T: Type)` to constrain polymorphic functions, e.g., `identity: (T: Type) -> ((x: T) -> T) = x` (RFC-010) |
| Default type filling | Omitting `-> Void` for no-parameter, no-return functions, automatically filled by the compiler                                   |
| Syntactic sugar      | A simplified syntax that makes code easier to read                                                                               |
| Normalization        | Converting a syntax form to a unified internal representation                                                                    |
| Function as lambda   | A function is essentially a lambda variable, with its type automatically inferred by the HM algorithm                            |

---

## References

- [MoonBit Language Design](https://moonbitlang.com/)
- [Rust Function Syntax](https://doc.rust-lang.org/book/ch03-03-how-functions-work.html)
- [Haskell Type System](https://www.haskell.org/tutorial/patterns.html)
- [OCaml Type Inference](https://v2.ocaml.org/manual/)
