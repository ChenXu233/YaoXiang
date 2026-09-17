---
title: 'RFC-007: Function Definition Syntax Unification Plan'
issue: '#131'
status: 'Accepted'
author: '沫郁酱'
created: '2025-01-05'
updated: '2026-09-15 (Return semantics corrected per RFC-010a)'
---

# RFC-007: Function Definition Syntax Unification Plan

> **Errata (2026-09-15, RFC-010a)**: This document previously stated "code blocks must use `return`
> to return a value; when no `return` is present, the default return is `Void`." That statement is
> now obsolete—**a block's value = its tail expression**, and `return` is a `Never`-typed non-local
> exit. See [RFC-010a](./010a-tail-expression-and-return.md). The function form definitions in this
> document (full form / omitted lambda head / omitted signature / no-argument variants) **remain
> unchanged**, and the "early return" semantics are already consistent with this RFC.
>
> **Addendum (Ruling C)**: This document's "no-argument simplest" form `name = { ... }` overlaps
> syntactically with the "value block" form in RFC-010. The ruling is **annotation priority,
> function by default**—without an annotation, the "no-argument simplest" form here stands (it is a
> function); a non-`Fn` annotation makes it a block value (`x: Int = { ... }`). See RFC-010a
> Appendix D.
>
> **Related addendum**: Statement termination and line-break rules within function bodies (the
> `{ ... }` code block) — explicit `;` separators, line-break termination, and line-continuation
> exceptions — are defined by [RFC-038 (draft)](../draft/038-statement-termination.md) and are out
> of scope for this RFC.

## Summary

This RFC finalizes the **function definition syntax** for the YaoXiang language. It uses the unified
syntax `name: (params) -> Return = body`, fully consistent with the `name: type = value` model from
RFC-010.

To avoid ambiguity: when a function has input parameters, parameter types must be explicitly
annotated in at least one of the "signature" or the "lambda head"; omitting both sides is rejected.

The value of a code block `{ ... }` is given by its **tail expression**; `return` is a `Never`-typed
non-local exit (see [RFC-010a](./010a-tail-expression-and-return.md)). The expression form `= expr`
directly provides the value.

## Motivation

### Why is this feature needed?

1. **Syntax consistency**: Eliminate the historical baggage of old syntax and unify the style.
2. **Conciseness**: The HM algorithm performs type inference automatically, reducing boilerplate.
3. **Type safety**: The HM algorithm guarantees type safety; types are only explicitly annotated
   when inference fails.
4. **Language maturity**: The HM algorithm is a mature approach in modern functional languages.

### Unified Syntax Model

**Core principle**: `name: Signature = LambdaBody`

- **Full form**: A signature (with parameter names + types + `->` + return type) + a lambda head
  (with parameter names).
- **Abbreviation rules**: Omit as much as possible without introducing ambiguity.
  - `->` cannot be omitted (it is the marker of a function type; otherwise it would be parsed as a
    tuple).
  - **When there are input parameters**, parameter types must appear explicitly in either the
    signature or the lambda head.
  - The lambda head can be omitted — if the signature already declares parameter names and types.
  - The return type can be explicitly annotated, or omitted when inferrable.

```yaoxiang
# Full form (signature complete + lambda head complete)
add: (a: Int, b: Int) -> Int = (a, b) => a + b

# Abbreviation: omit the lambda head (signature already declares parameters)
add: (a: Int, b: Int) -> Int = a + b

# Abbreviation: omit the signature (lambda head annotates parameter types)
add = (a: Int, b: Int) => a + b

# ❌ Error: parameter types annotated on neither side
# add = (a, b) => a + b
```

### Design Goals

```yaoxiang
# === Full form ===
add: (a: Int, b: Int) -> Int = (a, b) => { a + b }

# === Abbreviated forms ===
add: (a: Int, b: Int) -> Int = a + b                 # Omit the lambda head
add = (a: Int, b: Int) => a + b                      # Omit the signature

# === No-argument functions ===
main: () -> Void = () => { println("Hello") }          # Full form
main: () -> Void = { println("Hello") }                # Omit the lambda head
main = { println("Hello") }                            # Simplest form (inferred as () -> Void)

# === Generic functions (using RFC-010 unified syntax) ===
identity: (T: Type) -> ((x: T) -> T) = (x) => x         # Full form
identity: (T: Type) -> ((x: T) -> T) = x                # Omit the lambda head
identity = (x: T) => x                                  # Omit the signature (lambda head annotates types)

# === Recursive functions ===
factorial: (n: Int) -> Int = (n) => {
    if n <= 1 { return 1 } else { return n * factorial(n - 1) }
}
```

### Syntax Rules

| Scenario               | Syntax                                                 | Description                           |
| ---------------------- | ------------------------------------------------------ | ------------------------------------- |
| **Full form**          | `name: (a: Type, b) -> Ret = (a, b) => { return ... }` | Signature + lambda head complete      |
| **Omit lambda head**   | `name: (a: Type, b: Type) -> Ret = { ... }`            | Signature already declares parameters |
| **Omit signature**     | `name = (a: Type, b: Type) => { ... }`                 | Lambda head annotates parameter types |
| **No-arg full**        | `name: () -> Void = () => { return ... }`              | No-argument function, full form       |
| **No-arg abbreviated** | `name: () -> Void = { return ... }`                    | Omit the lambda head                  |
| **No-arg simplest**    | `name = { return ... }`                                | No parameters, no return, simplest    |

**Note**: The value of a code block `{ ... }` is given by its **tail expression** (the sole exit);
`return` is a `Never`-typed non-local exit that exits the nearest function boundary. The expression
form `= expr` directly provides the value. See [RFC-010a](./010a-tail-expression-and-return.md).

**Note**: `->` is the marker of a function type and cannot be omitted (otherwise it would be parsed
as a tuple).

**Important**: `if` expressions use curly braces `{}` to enclose branches and do **not** support the
`then/else` keywords:

```yaoxiang
# Correct: use curly braces
if n <= 1 { return 1 } else { return n * factorial(n - 1) }

# Wrong: then/else keywords are not supported
# if n <= 1 then return 1 else return n * factorial(n - 1)
```

## Proposal

### HM Algorithm and Higher-Rank Polymorphism Support

**Core feature**: The HM algorithm supports higher-rank polymorphism through generic type
annotations.

**Design rationale**:

- **Higher-order functions**: When functions are passed as arguments, generic constraints are needed
  on their function types.
- **Type annotation form**: `(T: Type) -> ((f: (T) -> T, x: T) -> T)` — generic parameters constrain
  function types.
- **HM workflow**: Infer function types through generic parameter instantiation, enabling
  polymorphic function composition.

**Example explanation**:

```yaoxiang
# ✅ Supports higher-rank polymorphism: generic constraints on function-type parameters
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = {
    return f(f(x))
}
# Usage: call_twice((x) => x + 1, 5)  # Infers T=Int

compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = {
    return f(g(x))
}
# Usage: compose((x) => x * 2, (x) => x + 1, 5)  # Infers A=Int, B=Int, C=Int

# ❌ Not supported: higher-order functions lacking generic constraints
# bad_hof: (f, x) => f(f(x))  # HM cannot infer; missing generic parameters
```

**HM inference process**:

1. Identify higher-order function parameters: `f: (T) -> T`.
2. Create a generic constraint: `(T: Type)`.
3. Infer concrete types via generic instantiation.
4. Enable polymorphic function composition.

### Lambda Expression Syntax Rules

**Important rule**: The value of a code block `{ ... }` is given by its **tail expression** (the
sole exit); `return` is a `Never`-typed non-local exit that exits the nearest function boundary. The
expression form `= expr` directly provides the value. See
[RFC-010a](./010a-tail-expression-and-return.md).

| Syntax form         | Syntax           | Value exit                                     |
| ------------------- | ---------------- | ---------------------------------------------- |
| **Code block form** | `{ statements }` | Tail expression (empty block `{}` is `Void`)   |
| **Expression form** | `expression`     | The expression value                           |
| **`return`**        | `return e`       | Non-local exit from the function; type `Never` |

**Examples**:

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

1. Function definitions use the HM algorithm for type inference—infer as much as possible, and
   report an explicit error when inference fails.
2. **How the HM algorithm works**: It automatically infers types from contextual information such as
   operator type constraints and function call relationships.
3. **Generics support**: Polymorphic functions use the generic syntax `(T: Type)` to explicitly
   constrain type parameters (RFC-010/011).
4. **Inference boundaries**: Return types and local variables are inferrable; parameter types of
   functions with parameters must be explicitly annotated (in either the signature or the lambda
   head).
5. No-argument, no-return functions use `name: () -> Void = { ... }`, unified with RFC-010.
6. The old syntax is retired; a migration tool is provided.

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

# Higher-rank polymorphism: implemented via generic type annotations so HM supports higher-rank polymorphism
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

# Generic functions: explicitly constrain polymorphic type parameters (using RFC-010 unified syntax)
identity: (T: Type) -> ((x: T) -> T) = x
map: (T: Type, R: Type) -> ((f: (T) -> R, list: List(T)) -> List(R)) = {
    # Implementation of map
    return List(R)()
}

# Recursive functions: inferred via the HM algorithm and recursive constraints
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

| Scenario                | Syntax                                            | Omissible parts | Example                               |
| ----------------------- | ------------------------------------------------- | --------------- | ------------------------------------- |
| **Full form**           | `name: (a: Type, b: Type) -> Ret = (a, b) => ...` | None            | Signature + lambda head complete      |
| **Omit lambda head**    | `name: (a: Type, b: Type) -> Ret = ...`           | Lambda head     | Signature already declares parameters |
| **Omit signature**      | `name = (a: Type, b: Type) => ...`                | Signature       | Lambda head provides parameter types  |
| **Omit return `Ret`**   | `name: (a: Type, b: Type) -> = ...`               | Return type     | HM infers the return type             |
| **No-arg full**         | `name: () -> Void = () => { ... }`                | None            | No-argument function, full form       |
| **No-arg abbreviated**  | `name: () -> Void = { ... }`                      | Lambda head     | Omit `() =>`                          |
| **No-arg simplest**     | `name = { ... }`                                  | All             | No parameters, no return, simplest    |
| **Variable assignment** | `name = value`                                    | Type            | HM infers the type                    |
| **Explicit variable**   | `name: Type = value`                              | None            | Explicit type annotation              |

**Core principles**:

- `->` is the marker of a function type and cannot be omitted (otherwise it would be parsed as a
  tuple).
- The return type `Ret` can be omitted and is inferred by HM from the function body.
- When there are input parameters, parameter types must appear explicitly (in either the signature
  or the lambda head).
- Other parts may be omitted when inferrable and unambiguous.
- No implicit type conversions; this avoids JavaScript-style confusion.

## Detailed Design

### Syntactic Sugar Desugaring

Regardless of what is omitted, the form is normalized to a unified intermediate representation:

```rust
// Full form
add: (a: Int, b: Int) -> Int = (a, b) => a + b

// Desugared IR
let add: (Int, Int) -> Int = |a: Int, b: Int| -> Int {
    return a + b
};

// Omit lambda head
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
               | identifier '=' block                    # Simplest form: no parameters, no return

identifier ::= [a-zA-Z_][a-zA-Z0-9_]*

type_expr ::= identifier                     # Type reference
       | '()'                          # Empty type
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
           | expression                  # Expression statement (executes but does not return)
           | 'return' expression         # Return statement (returns the given value)

# Note: inside a code block, return must be used to return a value; without return, the default return is Void.
# For example: { return 1 + 1 } returns Int; { println("Hello") } returns Void.
# Note: generic parameters use the (T: Type) syntax as part of the function type and need no separate BNF rule.
```

### Error Handling

```yaoxiang
# === Compilation error examples ===

# Error 1: code block return type does not match
add: (a: Int, b: Int) -> Int = { println(a + b) }
// Error: no return in the block, default return is Void, but the signature expects Int
// Correct: add: (a: Int, b: Int) -> Int = a + b
// Or:      add: (a: Int, b: Int) -> Int = { return a + b }

# Error 2: using an undeclared type parameter
identity: (x: T) -> T = x
// Error: T is undeclared; an explicit generic parameter is required (RFC-010)
// Correct: identity: (T: Type) -> ((x: T) -> T) = x

# Correct: HM algorithm infers the return type
double = (x: Int) => x + x

# Full form (stepwise abbreviation)
double: (x: Int) -> Int = (x) => x + x                # Full
double: (x: Int) -> Int = x + x                       # Omit the lambda head
double = (x: Int) => x + x                            # Omit the return type (HM infers it)
# double = (x) => x + x                               # ❌ Parameter types are not allowed to be omitted on both sides
```

## Trade-offs

### Advantages

- **Unified syntax**: The `name: Signature = LambdaBody` model covers every scenario.
- **Flexible abbreviations**: Any part can be omitted when HM can infer it.
- **Type safety**: The HM algorithm guarantees type safety and avoids implicit type conversions.
- **Recursive support**: The HM algorithm and recursive constraints infer types automatically.
- **Zero burden**: Smooth progression from full to simplest.

### Disadvantages

- **Migration cost**: Existing code needs a migration tool to convert.
- **Learning cost**: Must understand the "full form + arbitrary abbreviation" model.

## Alternatives

| Alternative                 | Description                                            | Why not chosen                                                       |
| --------------------------- | ------------------------------------------------------ | -------------------------------------------------------------------- |
| HM algorithm type inference | Use the Hindley-Milner algorithm to infer types        | ✅ **Adopted**; the standard for modern functional languages         |
| Explicit type declarations  | All types must be written explicitly                   | Violates the syntactic-simplification principle and adds boilerplate |
| Preserve old syntax         | Support both old and new syntax simultaneously         | Splits the language and increases maintenance cost                   |
| `fn` keyword                | Introduce `fn` to distinguish functions from variables | Violates the "functions are lambdas" design                          |

## Implementation Strategy

### Phased Plan

1. **Phase 1: Syntax parsing and HM algorithm** (v0.3)
   - Implement the new syntax `name = lambda` together with the HM algorithm for type inference.
   - Implement default filling for no-argument, no-return cases.

2. **Phase 2: Migration tool** (v0.3)
   - Develop the `yaoxiang-migrate --old-to-new` tool.
   - Automatically convert code written in the old syntax.

3. **Phase 3: Verification and documentation** (v0.3)
   - Verify the migration of old code is complete.
   - Update documentation.

### Migration Tool

```bash
# Migrate a single file
yaoxiang-migrate --old-to-new src/main.yaoxiang

# Migrate an entire project
yaoxiang-migrate --old-to-new --recursive src/

# Preview the migration (does not modify files)
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
main = { println("Hello") }                  # Inferred as () -> Void

# === Simplest form ===
main = {                                      # Equivalent to main: () -> Void = { ... }
    println("Hello")
}
```

### Dependencies

- No external dependencies.
- Can be implemented independently.

### Risks

| Risk              | Impact                    | Mitigation                                                |
| ----------------- | ------------------------- | --------------------------------------------------------- |
| Missed migrations | Old code fails to compile | Provide a migration tool covering all old syntax patterns |
| Parser bugs       | Unstable syntax parsing   | Thorough test coverage                                    |

## Open Questions

> The questions below have been resolved in the design and are recorded in Appendix A.

- ~~Q1: Should the very compact form `main() = body` be retained?~~ → Resolved: retained as
  `main = { ... }`.
- ~~Q2: Should the `:` after the function name be retained?~~ → Resolved: optionally retained;
  however, functions with parameters still need parameter types annotated in the signature or the
  lambda head.
- ~~Q3: Does the HM algorithm support parameter type inference?~~ → Resolved: return values / locals
  are inferrable; parameter types of functions with parameters must be explicitly annotated.
- ~~Q4: Should the `fn` keyword be introduced?~~ → Resolved: not introduced; a function is a lambda.
- ~~Q5: What is the migration strategy for old code?~~ → Resolved: provide the `yaoxiang-migrate`
  tool.
- ~~Q6: How are generic functions used?~~ → Resolved: use the RFC-010 unified syntax `(T: Type)`.

---

## Appendices

### Appendix A: Reference of Function Definition Syntaxes Across Languages

| Language     | Syntax style                                        | Features                                          |
| ------------ | --------------------------------------------------- | ------------------------------------------------- |
| Rust         | `fn add(a: i32, b: i32) -> i32 { ... }`             | Keyword + type annotation                         |
| Haskell      | `add a b = ...` / `add :: Int -> Int -> Int`        | Type signature separated                          |
| OCaml        | `let add a b = ...`                                 | Parameter types can be omitted                    |
| MoonBit      | `fn add(a: Int, b: Int): Int { ... }`               | Concise type annotation                           |
| TypeScript   | `const add = (a: number, b: number): number => ...` | Lambda style                                      |
| Scala        | `def add(a: Int, b: Int): Int = { ... }`            | `def` keyword                                     |
| **YaoXiang** | `name = (a: Int, b: Int) => a + b`                  | **Function = lambda; HM infers the return value** |

### Appendix B: Design Decision Records

| Decision              | Decision                                                                             | Date       | Recorder |
| --------------------- | ------------------------------------------------------------------------------------ | ---------- | -------- |
| Syntax style          | New syntax `name: (params) -> Return = body` + HM inference                          | 2026-02-03 | @沫郁酱  |
| Parameter position    | Parameter names declared in the signature, unified with RFC-010                      | 2026-02-03 | @沫郁酱  |
| Default filling       | No-argument functions can omit the signature; empty block `{}` is inferred as `Void` | 2026-02-03 | @沫郁酱  |
| Type inference        | HM algorithm infers automatically; explicit when inference fails                     | 2026-01-06 | @沫郁酱  |
| Old syntax            | Retired; a migration tool is provided                                                | 2026-01-06 | @沫郁酱  |
| `fn` keyword          | Not introduced                                                                       | 2026-01-06 | @沫郁酱  |
| Recursive declaration | HM algorithm and recursive constraints infer automatically                           | 2026-01-06 | @沫郁酱  |

### Appendix C: Glossary

| Term                 | Definition                                                                                                                    |
| -------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| HM algorithm         | The Hindley-Milner type inference algorithm; automatically infers function and variable types                                 |
| Generics             | Use type parameters `(T: Type)` to constrain polymorphic functions, e.g. `identity: (T: Type) -> ((x: T) -> T) = x` (RFC-010) |
| Default type filling | No-argument, no-return functions omit `-> Void`; the compiler fills it in automatically                                       |
| Syntactic sugar      | A simplified syntax that makes code easier to read                                                                            |
| Normalization        | Converting syntactic forms into a unified internal representation                                                             |
| Function-as-lambda   | A function is essentially a lambda variable whose type is inferred automatically by the HM algorithm                          |

---

## References

- [MoonBit Language Design](https://moonbitlang.com/)
- [Rust Function Syntax](https://doc.rust-lang.org/book/ch03-03-how-functions-work.html)
- [Haskell Type System](https://www.haskell.org/tutorial/patterns.html)
- [OCaml Type Inference](https://v2.ocaml.org/manual/)
