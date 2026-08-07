---
title: 'RFC-007: Unified Function Definition Syntax Proposal'
issue: '#131'
status: 'Accepted'
author: 'Moyu-jang'
created: '2025-01-05'
updated: '2026-07-05 (synced to GH Issue #131)'
---

# RFC-007: Unified Function Definition Syntax Proposal

> **Related Supplement**: The statement termination and line break rules inside function bodies
> (`{ ... }` blocks) (explicit `;` separators, line-break termination, line-continuation exceptions)
> are defined by [RFC-038 (Draft)](../draft/038-statement-termination.md) and are not covered by
> this RFC.

## Summary

This RFC establishes the final scheme for **function definition syntax** in the YaoXiang language.
It uses the unified syntax `name: (params) -> Return = body`, fully consistent with the
`name: type = value` model from RFC-010.

To avoid ambiguity: when a function has input parameters, the parameter types must be explicitly
annotated in at least one of the "signature" or "lambda head"; omitting both sides will be rejected.

Inside a code block `{ ... }`, `return` must be used to return a value; without `return`, the
default return is `Void`. The expression form `= expr` directly returns the value.

## Motivation

### Why is this feature needed?

1. **Syntax consistency**: Eliminate the legacy baggage of old syntax, unify the style
2. **Conciseness**: The HM algorithm automatically infers types, reducing boilerplate code
3. **Type safety**: The HM algorithm guarantees type safety; explicit annotation is only required
   when inference fails
4. **Language maturity**: The HM algorithm is a mature solution in modern functional languages

### Unified Syntax Model

**Core principle**: `name: Signature = LambdaBody`

- **Full form**: Signature (containing parameter names + types + `->` + return type) + Lambda head
  (containing parameter names)
- **Shorthand rules**: Omit as much as possible without introducing ambiguity
  - `->` cannot be omitted (it's the marker of function type, otherwise it would be parsed as a
    tuple)
  - **When there are input parameters**, parameter types must appear explicitly in at least one of
    the signature or lambda head
  - The Lambda head can be omitted → if the signature already declares parameter names and types
  - The return type can be explicitly annotated, or omitted when inferable

```yaoxiang
# Full form (complete signature + complete lambda head)
add: (a: Int, b: Int) -> Int = (a, b) => a + b

# Shorthand: omit the lambda head (signature already declares parameters)
add: (a: Int, b: Int) -> Int = a + b

# Shorthand: omit the signature (lambda head annotates parameter types)
add = (a: Int, b: Int) => a + b

# ❌ Error: neither side annotates parameter types
# add = (a, b) => a + b
```

### Design Goals

```yaoxiang
# === Full form ===
add: (a: Int, b: Int) -> Int = (a, b) => { a + b }

# === Shorthand forms ===
add: (a: Int, b: Int) -> Int = a + b                 # Omit the lambda head
add = (a: Int, b: Int) => a + b                      # Omit the signature

# === Zero-argument function ===
main: () -> Void = () => { println("Hello") }          # Full form
main: () -> Void = { println("Hello") }                # Omit the lambda head
main = { println("Hello") }                            # Most concise form (inferred as () -> Void)

# === Generic function (using RFC-010 unified syntax) ===
identity: (T: Type) -> ((x: T) -> T) = (x) => x         # Full form
identity: (T: Type) -> ((x: T) -> T) = x                # Omit the lambda head
identity = (x: T) => x                                  # Omit the signature (lambda head annotates types)

# === Recursive function ===
factorial: (n: Int) -> Int = (n) => {
    if n <= 1 { return 1 } else { return n * factorial(n - 1) }
}
```

### Syntax Rules

| Scenario               | Syntax                                                 | Description                           |
| ---------------------- | ------------------------------------------------------ | ------------------------------------- |
| **Full form**          | `name: (a: Type, b) -> Ret = (a, b) => { return ... }` | Complete signature + lambda head      |
| **Omit lambda head**   | `name: (a: Type, b: Type) -> Ret = { ... }`            | Signature already declares parameters |
| **Omit signature**     | `name = (a: Type, b: Type) => { ... }`                 | Lambda head annotates parameter types |
| **Zero-arg full**      | `name: () -> Void = () => { return ... }`              | Complete zero-argument function       |
| **Zero-arg shorthand** | `name: () -> Void = { return ... }`                    | Omit the lambda head                  |
| **Zero-arg minimal**   | `name = { return ... }`                                | Most concise for no-arg, no-return    |

**Note**: Inside a code block `{ ... }`, `return` must be used to return a value; without `return`,
the default return is `Void`. The expression form `= expr` directly returns the value.

**Note**: `->` is the marker of function type and cannot be omitted (otherwise it would be parsed as
a tuple).

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

**Design principles**:

- **Higher-order functions**: When a function is passed as a parameter, generic constraints are
  needed for its function type
- **Type annotation form**: `(T: Type) -> ((f: (T) -> T, x: T) -> T)` — generic parameters constrain
  the function type
- **HM workflow**: Infers function types through generic parameters, enabling polymorphic function
  composition

**Example explanation**:

```yaoxiang
# ✅ Supports higher-rank polymorphism: generic-constrained function type parameters
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = {
    return f(f(x))
}
# Usage: call_twice((x) => x + 1, 5)  # Infer T=Int

compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = {
    return f(g(x))
}
# Usage: compose((x) => x * 2, (x) => x + 1, 5)  # Infer A=Int, B=Int, C=Int

# ❌ Not supported: higher-order function without generic constraints
# bad_hof: (f, x) => f(f(x))  # HM cannot infer, missing generic parameters
```

**HM inference process**:

1. Identify higher-order function parameters: `f: (T) -> T`
2. Create generic constraints: `(T: Type)`
3. Infer concrete types through generic instantiation
4. Implement polymorphic function composition

### Lambda Expression Syntax Rules

**Important rule**: Inside a code block `{ ... }`, `return` must be used to return a value; without
`return`, the default return is `Void`. The expression form `= expr` directly returns the value.

| Syntax Form         | Syntax           | Return Method                                                             |
| ------------------- | ---------------- | ------------------------------------------------------------------------- |
| **Code block form** | `{ statements }` | Must use `return` to return a value; without `return`, defaults to `Void` |
| **Expression form** | `expression`     | Directly returns the expression value                                     |

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

# Expression form: return value directly (no return needed)
add: (a: Int, b: Int) -> Int = a + b            # Correct: expression form
main: () -> Void = println("Hello")               # Correct: expression form
```

**Core ideas**:

1. Function definitions use the HM algorithm for type inference; infer as much as possible, and
   report an explicit error when inference fails
2. **How the HM algorithm works**: Automatically infers types from operator type constraints,
   function call relationships, and other contextual information
3. **Generic support**: Polymorphic functions use the generic syntax `(T: Type)` to explicitly
   constrain type parameters (RFC-010/011)
4. **Inference boundary**: Return type and local variables are inferable; parameter types of
   functions with arguments must be explicitly annotated (in either the signature or the lambda
   head)
5. No-arg, no-return functions use `name: () -> Void = { ... }`, unified with RFC-010
6. Old syntax is retired; migration tools are provided

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

# Higher-rank polymorphism: HM supports higher-rank polymorphism through generic type annotations
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
    # Implementation of the map function
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
y = 42                               # Inferred as Int

# HM algorithm automatically infers as String
name = "YaoXiang"                    # Inferred as String

# HM algorithm automatically infers as Float
pi = 3.14159                         # Inferred as Float
```

**HM type inference rules**:

| Scenario                | Syntax                                            | Omissible Part | Example                               |
| ----------------------- | ------------------------------------------------- | -------------- | ------------------------------------- |
| **Full form**           | `name: (a: Type, b: Type) -> Ret = (a, b) => ...` | None           | Complete signature + lambda head      |
| **Omit lambda head**    | `name: (a: Type, b: Type) -> Ret = ...`           | Lambda head    | Signature already declares parameters |
| **Omit signature**      | `name = (a: Type, b: Type) => ...`                | Signature      | Lambda head provides parameter types  |
| **Omit return Ret**     | `name: (a: Type, b: Type) -> = ...`               | Return type    | HM infers return type                 |
| **Zero-arg full**       | `name: () -> Void = () => { ... }`                | None           | Complete zero-argument function       |
| **Zero-arg shorthand**  | `name: () -> Void = { ... }`                      | Lambda head    | Omit `() =>`                          |
| **Zero-arg minimal**    | `name = { ... }`                                  | All            | Most concise for no-arg, no-return    |
| **Variable assignment** | `name = value`                                    | Type           | HM infers type                        |
| **Explicit variable**   | `name: Type = value`                              | None           | Explicit type annotation              |

**Core principles**:

- `->` is the marker of function type and cannot be omitted (otherwise it would be parsed as a
  tuple)
- The return type `Ret` can be omitted and inferred by HM from the function body
- When input parameters exist, parameter types must appear explicitly (in either the signature or
  the lambda head)
- Other parts can be omitted when inferable and unambiguous
- No implicit type conversions, avoiding JavaScript-style chaos

## Detailed Design

### Syntax Sugar Expansion

Regardless of which parts are omitted, everything is normalized to a unified intermediate
representation:

```rust
// Full form
add: (a: Int, b: Int) -> Int = (a, b) => a + b

// Expanded IR
let add: (Int, Int) -> Int = |a: Int, b: Int| -> Int {
    return a + b
};

// Omit the lambda head
add: (a: Int, b: Int) -> Int = a + b

// Expanded IR (same as the full form)
let add: (Int, Int) -> Int = |a: Int, b: Int| -> Int {
    return a + b
};

// Omit the signature (lambda head annotates parameter types)
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
               | identifier '=' block                    # Most concise form: no-arg, no-return

identifier ::= [a-zA-Z_][a-zA-Z0-9_]*

type_expr ::= identifier                     # Type reference
       | '()'                                # Unit type
       | '(' parameters ')' '->' type_expr   # Function type (parameter names in the signature)
       | type_expr '->' type_expr            # Simple function type
       | identifier '(' type_expr (',' type_expr)* ')'  # Type application

expression ::= '(' parameters ')' '=>' block
             | '(' ')' '=>' block
             | '(' parameters ')' '=>' expression

parameters ::= parameter (',' parameter)*
parameter ::= identifier                      # Type inference
            | identifier ':' type_expr        # Partially explicit type

block ::= '{' statement (',' statement)* '}'
        | expression

statement ::= identifier ':' expression  # Assignment statement
           | expression                   # Expression statement (executes but does not return)
           | 'return' expression          # Return statement (returns the specified value)

# Note: inside a code block, `return` must be used to return a value; without `return`, defaults to Void
# For example: { return 1 + 1 } returns Int; { println("Hello") } returns Void
# Note: generic parameters use the (T: Type) syntax as part of the function type, no independent BNF rule is needed
```

### Error Handling

```yaoxiang
# === Compilation error examples ===

# Error 1: Code block return type mismatch
add: (a: Int, b: Int) -> Int = { println(a + b) }
// Error: no return inside the block, defaults to Void, but the signature expects Int
// Correct: add: (a: Int, b: Int) -> Int = a + b
// Or: add: (a: Int, b: Int) -> Int = { return a + b }

# Error 2: Using an undeclared type parameter
identity: (x: T) -> T = x
// Error: T is undeclared; explicit generic parameter needed (RFC-010)
// Correct: identity: (T: Type) -> ((x: T) -> T) = x

# Correct: HM algorithm infers the return type
double = (x: Int) => x + x

# Full form (progressive shorthand)
double: (x: Int) -> Int = (x) => x + x                # Full
double: (x: Int) -> Int = x + x                       # Omit the lambda head
double = (x: Int) => x + x                            # Omit the return type (HM infers the return)
# double = (x) => x + x                               # ❌ Parameter types are not allowed to be omitted on both sides
```

## Trade-offs

### Advantages

- **Syntax unification**: The `name: Signature = LambdaBody` model covers all scenarios
- **Flexible shorthand**: Any part can be omitted when inferable by HM
- **Type safety**: The HM algorithm guarantees type safety, avoiding implicit type conversions
- **Recursive support**: The HM algorithm and recursive constraints automatically infer types
- **Zero burden**: Smooth transition from full to most concise

### Disadvantages

- **Migration cost**: Old code needs migration tools for conversion
- **Learning cost**: Need to understand the "full form + arbitrary shorthand" model

## Alternatives

| Proposal                    | Description                                         | Why Not Chosen                                                  |
| --------------------------- | --------------------------------------------------- | --------------------------------------------------------------- |
| HM algorithm type inference | Use the Hindley-Milner algorithm for type inference | ✅ **Adopted**, the modern functional language standard         |
| Explicit type declaration   | All types must be written explicitly                | Violates the simplified syntax principle, increases boilerplate |
| Keep old syntax             | Support both old and new syntax simultaneously      | Syntax fragmentation, high maintenance cost                     |
| fn keyword                  | Introduce fn to distinguish functions and variables | Violates the "function is a lambda" design                      |

## Implementation Strategy

### Phases

1. **Phase 1: Syntax Parsing and HM Algorithm** (v0.3)
   - Implement the new syntax `name = lambda` + HM algorithm type inference
   - Implement default filling for no-arg, no-return cases

2. **Phase 2: Migration Tools** (v0.3)
   - Develop the `yaoxiang-migrate --old-to-new` tool
   - Automatically convert old syntax code

3. **Phase 3: Validation and Documentation** (v0.3)
   - Old code migration completion validation
   - Documentation update

### Migration Tool

```bash
# Migrate a single file
yaoxiang-migrate --old-to-new src/main.yaoxiang

# Migrate the entire project
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

# === New syntax: full form (complete signature + complete lambda head) ===
add: (a: Int, b: Int) -> Int = (a, b) => a + b
main: () -> Void = () => { println("Hello") }

# === Shorthand: omit the lambda head ===
add: (a: Int, b: Int) -> Int = a + b
main: () -> Void = { println("Hello") }

# === Shorthand: HM inference ===
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

| Risk             | Impact                    | Mitigation                                             |
| ---------------- | ------------------------- | ------------------------------------------------------ |
| Migration misses | Old code fails to compile | Provide migration tools, cover all old syntax patterns |
| Parser errors    | Unstable syntax parsing   | Sufficient test coverage                               |

## Open Questions

> The following questions have been resolved during the design and are recorded in Appendix A.

- ~~Q1: Should the extremely concise `main() = body` form be retained?~~ → Resolved: Retained as
  `main = { ... }`
- ~~Q2: Should the `:` after the function name be retained?~~ → Resolved: Optionally retained; but
  functions with parameters still need parameter types annotated in either the signature or the
  lambda head
- ~~Q3: Does the HM algorithm support parameter type inference?~~ → Resolved: Return value / locals
  are inferable; parameter types of functions with arguments must be explicitly annotated
- ~~Q4: Should the `fn` keyword be introduced?~~ → Resolved: Not introduced; functions are lambdas
- ~~Q5: What is the migration strategy for old code?~~ → Resolved: Provide the `yaoxiang-migrate`
  tool
- ~~Q6: How are generic functions used?~~ → Resolved: Use the RFC-010 unified syntax `(T: Type)`

---

## Appendix

### Appendix A: Reference: Function Definition Syntax in Various Languages

| Language     | Syntax Style                                        | Characteristics                               |
| ------------ | --------------------------------------------------- | --------------------------------------------- |
| Rust         | `fn add(a: i32, b: i32) -> i32 { ... }`             | Keyword + type annotation                     |
| Haskell      | `add a b = ...` / `add :: Int -> Int -> Int`        | Separate type signature                       |
| OCaml        | `let add a b = ...`                                 | Parameter types can be omitted                |
| MoonBit      | `fn add(a: Int, b: Int): Int { ... }`               | Concise type annotation                       |
| TypeScript   | `const add = (a: number, b: number): number => ...` | Lambda style                                  |
| Scala        | `def add(a: Int, b: Int): Int = { ... }`            | def keyword                                   |
| **YaoXiang** | `name = (a: Int, b: Int) => a + b`                  | **Function = lambda, HM infers return value** |

### Appendix B: Design Decision Records

| Decision              | Decision                                                                          | Date       | Recorder   |
| --------------------- | --------------------------------------------------------------------------------- | ---------- | ---------- |
| Syntax style          | New syntax `name: (params) -> Return = body` + HM inference                       | 2026-02-03 | @Moyu-jang |
| Parameter position    | Parameter names declared in the signature, unified with RFC-010                   | 2026-02-03 | @Moyu-jang |
| Default filling       | Zero-arg functions can omit the signature; empty block `{}` is inferred as `Void` | 2026-02-03 | @Moyu-jang |
| Type inference        | HM algorithm automatically infers; explicit when unable to infer                  | 2026-01-06 | @Moyu-jang |
| Old syntax            | Retired, migration tool provided                                                  | 2026-01-06 | @Moyu-jang |
| fn keyword            | Not introduced                                                                    | 2026-01-06 | @Moyu-jang |
| Recursive declaration | HM algorithm and recursive constraints automatically infer                        | 2026-01-06 | @Moyu-jang |

### Appendix C: Glossary

| Term                 | Definition                                                                                                                    |
| -------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| HM algorithm         | Hindley-Milner type inference algorithm; automatically infers types of functions and variables                                |
| Generics             | Use type parameters `(T: Type)` to constrain polymorphic functions, e.g. `identity: (T: Type) -> ((x: T) -> T) = x` (RFC-010) |
| Default type filling | Zero-arg, no-return functions omit `-> Void`, and the compiler fills it in automatically                                      |
| Syntax sugar         | Syntactic shorthand that makes code easier to read                                                                            |
| Normalization        | Converting syntax forms into a unified internal representation                                                                |
| Function is lambda   | A function is essentially a lambda variable, with its type automatically inferred by the HM algorithm                         |

---

## References

- [MoonBit Language Design](https://moonbitlang.com/)
- [Rust Function Syntax](https://doc.rust-lang.org/book/ch03-03-how-functions-work.html)
- [Haskell Type System](https://www.haskell.org/tutorial/patterns.html)
- [OCaml Type Inference](https://v2.ocaml.org/manual/)
