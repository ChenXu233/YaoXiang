---
title: 'RFC-007: Unified Function Definition Syntax Proposal'
issue: '#131'
status: 'Accepted'
author: 'Moyujang'
created: '2025-01-05'
updated: '2026-09-15'
---

# RFC-007: Unified Function Definition Syntax Proposal

> **Related**: Where the "no-arg minimal" `name = { ... }` overlaps with RFC-010's "valued block"
> syntax position, the ruling is **content determines type**——`=>` is always a function, `Fn`
> annotation is a function, a non-`Fn` annotation is a block value, and with no annotation it is
> inferred from content. See [RFC-010a](010a-tail-expression-and-return.md) Appendix D.
>
> **Related supplement**: The statement termination and newline rules (`;` explicit separation,
> newline termination, continuation exception) within a function body (`{ ... }` code block) are
> defined by [RFC-038 (Draft)](038-statement-termination.md) and are not covered by this RFC.

## Summary

This RFC determines the final proposal for **function definition syntax** in the YaoXiang language.
It uses the unified syntax `name: (params) -> Return = body`, fully consistent with the
`name: type = value` model of RFC-010.

To avoid ambiguity: when a function has input parameters, the parameter types must be explicitly
annotated in at least one of either the "signature" or the "lambda head"; omitting them in both
places will be rejected.

The value of a code block `{ ... }` is given by the **tail expression**; `return` is a non-local
exit of type `Never` (see [RFC-010a](010a-tail-expression-and-return.md)). The expression form
`= expr` directly provides the value.

## Motivation

### Why is this feature needed?

1. **Syntax consistency**: Eliminate the historical baggage of old syntax and unify the style
2. **Conciseness**: The HM algorithm automatically infers types, reducing boilerplate code
3. **Type safety**: The HM algorithm guarantees type safety, with explicit annotation only when
   inference fails
4. **Language maturity**: The HM algorithm is a mature approach in modern functional languages

### Unified Syntax Model

**Core principle**: `name: Signature = LambdaBody`

- **Full form**: Signature (containing parameter names + types + `->` + return type) + Lambda head
  (containing parameter names)
- **Shorthand rules**: Omit whenever possible without introducing ambiguity
  - `->` cannot be omitted (it marks the function type, otherwise it would be parsed as a tuple)
  - **When there are input parameters**, parameter types must explicitly appear in at least one of
    either the signature or the lambda head
  - Lambda head can be omitted → if the signature already declares parameter names and types
  - Return type can be explicitly annotated, or omitted when inferable

```yaoxiang
# Full form (complete signature + complete lambda head)
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
# === Full form ===
add: (a: Int, b: Int) -> Int = (a, b) => { a + b }

# === Shorthand forms ===
add: (a: Int, b: Int) -> Int = a + b                 # Omit lambda head
add = (a: Int, b: Int) => a + b                      # Omit signature

# === No-argument functions ===
main: () -> Void = () => { println("Hello") }          # Full form
main: () -> Void = { println("Hello") }                # Omit lambda head
main: () -> Void = { println("Hello") }                            # Minimal form (inferred as () -> Void)

# === Generic functions (using RFC-010 unified syntax) ===
identity: (T: Type) -> ((x: T) -> T) = (x) => x         # Full form
identity: (T: Type) -> ((x: T) -> T) = x                # Omit lambda head
identity = (x: T) => x                                  # Omit signature (lambda head annotates types)

# === Recursive functions ===
factorial: (n: Int) -> Int = (n) => {
    if n <= 1 { return 1 } else { return n * factorial(n - 1) }
}
```

### Syntax Rules

| Scenario             | Syntax                                                 | Description                           |
| -------------------- | ------------------------------------------------------ | ------------------------------------- |
| **Full form**        | `name: (a: Type, b) -> Ret = (a, b) => { return ... }` | Complete signature + lambda head      |
| **Omit lambda head** | `name: (a: Type, b: Type) -> Ret = { ... }`            | Signature already declares parameters |
| **Omit signature**   | `name = (a: Type, b: Type) => { ... }`                 | Lambda head annotates parameter types |
| **No-arg full**      | `name: () -> Void = () => { return ... }`              | Complete no-argument function         |
| **No-arg shorthand** | `name: () -> Void = { return ... }`                    | Omit lambda head                      |
| **No-arg minimal**   | `name = { return ... }`                                | Minimal form with no args, no return  |

**Note**: The value of a code block `{ ... }` is given by the **tail expression** (the sole exit);
`return` is a non-local exit of type `Never`, exiting the nearest function boundary. The expression
form `= expr` directly provides the value. See [RFC-010a](010a-tail-expression-and-return.md) for
details.

**Note**: `->` is the marker of the function type and cannot be omitted (otherwise it would be
parsed as a tuple).

**Important**: The `if` expression uses curly braces `{}` to wrap branches and does not support
`then/else` keywords:

```yaoxiang
# Correct: use curly braces
if n <= 1 { return 1 } else { return n * factorial(n - 1) }

# Error: then/else keywords are not supported
# if n <= 1 then return 1 else return n * factorial(n - 1)
```

## Proposal

### HM Algorithm and Higher-Rank Polymorphism Support

**Core feature**: The HM algorithm supports higher-rank polymorphism through generic type
annotations

**Design rationale**:

- **Higher-order functions**: When functions are passed as arguments, generic constraints are needed
  for their function types
- **Type annotation form**: `(T: Type) -> ((f: (T) -> T, x: T) -> T)` - generic parameters constrain
  function types
- **HM workflow**: Infers function types through generic parameter instantiation, enabling
  polymorphic function composition

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

# ❌ Not supported: higher-order function without generic constraints
# bad_hof: (f, x) => f(f(x))  # HM cannot infer, missing generic parameters
```

**HM inference process**:

1. Identify higher-order function parameters: `f: (T) -> T`
2. Create generic constraints: `(T: Type)`
3. Infer concrete types through generic instantiation
4. Implement polymorphic function composition

### Lambda Expression Syntax Rules

**Important rule**: The value of a code block `{ ... }` is given by the **tail expression** (the
sole exit); `return` is a non-local exit of type `Never`, exiting the nearest function boundary. The
expression form `= expr` directly provides the value. See
[RFC-010a](010a-tail-expression-and-return.md) for details.

| Syntax form         | Syntax           | Value exit                                   |
| ------------------- | ---------------- | -------------------------------------------- |
| **Code block form** | `{ statements }` | Tail expression (empty block `{}` is `Void`) |
| **Expression form** | `expression`     | Expression value                             |
| **`return`**        | `return e`       | Non-local function exit, type `Never`        |

**Example**:

```yaoxiang
main: () -> Void = { println("Hello") }         # Tail expression is Void
add: (a: Int, b: Int) -> Int = { a + b }        # Tail expression gives the value
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

1. Function definitions use the HM algorithm for type inference, inferring whenever possible, with
   explicit errors when inference fails
2. **How the HM algorithm works**: Automatically infers types from contextual information such as
   operator type constraints and function call relationships
3. **Generic support**: Polymorphic functions use the generic syntax `(T: Type)` to explicitly
   constrain type parameters (RFC-010/011)
4. **Inference boundaries**: Return type and local variables can be inferred; parameter types for
   functions with parameters must be explicitly annotated (in either signature or lambda head)
5. No-arg, no-return functions use `name: () -> Void = { ... }`, unified with RFC-010
6. The old syntax is retired, with migration tools provided

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
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # Inferred as (Int, Int) -> Void

# Higher-rank polymorphism: achieved through generic type annotations, enabling HM to support higher-rank polymorphism
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = { return f(f(x)) }
compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = { return f(g(x)) }
```

```yaoxiang
# === Function definition: HM algorithm type inference ===

# Standard function: HM algorithm infers return type (parameter types must be explicit)
add = (a: Int, b: Int) => a + b            # Inferred as (a: Int, b: Int) -> Int
main: () -> Void = { println("Hello") }                # Inferred as () -> Void

# Partially explicit parameters: HM algorithm infers the rest
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # Inferred as (Int, Int) -> Void
greet: (name: String) -> Void = { println("Hello " + name) }  # Inferred as (String) -> Void

# Generic functions: explicit constraints on polymorphic type parameters (using RFC-010 unified syntax)
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

| Scenario                | Syntax                                            | Omissible part | Example                               |
| ----------------------- | ------------------------------------------------- | -------------- | ------------------------------------- |
| **Full form**           | `name: (a: Type, b: Type) -> Ret = (a, b) => ...` | None           | Complete signature + lambda head      |
| **Omit lambda head**    | `name: (a: Type, b: Type) -> Ret = ...`           | Lambda head    | Signature already declares parameters |
| **Omit signature**      | `name = (a: Type, b: Type) => ...`                | Signature      | Lambda head provides parameter types  |
| **Omit return Ret**     | `name: (a: Type, b: Type) -> = ...`               | Return type    | HM infers return type                 |
| **No-arg full**         | `name: () -> Void = () => { ... }`                | None           | Complete no-argument function         |
| **No-arg shorthand**    | `name: () -> Void = { ... }`                      | Lambda head    | Omit `() =>`                          |
| **No-arg minimal**      | `name = { ... }`                                  | All            | Minimal form with no args, no return  |
| **Variable assignment** | `name = value`                                    | Type           | HM infers type                        |
| **Explicit variable**   | `name: Type = value`                              | None           | Explicit type annotation              |

**Core principles**:

- `->` is the marker of the function type and cannot be omitted (otherwise it would be parsed as a
  tuple)
- The return type `Ret` can be omitted and inferred by HM from the function body
- When there are input parameters, the parameter types must explicitly appear (in either the
  signature or the lambda head)
- Other parts can be omitted when inferable and unambiguous
- No implicit type conversions, avoiding JavaScript-style chaos

## Detailed Design

### Syntactic Sugar Expansion

Regardless of what is omitted, everything is normalized to a unified intermediate representation:

```rust
// Full form
add: (a: Int, b: Int) -> Int = (a, b) => a + b

// Expanded IR
let add: (Int, Int) -> Int = |a: Int, b: Int| -> Int {
    return a + b
};

// Omit lambda head
add: (a: Int, b: Int) -> Int = a + b

// Expanded IR (same as full form)
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
               | identifier '=' block                    # Minimal form: no args, no return

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
           | expression                  # Expression statement (executes but does not return)
           | 'return' expression         # Return statement (returns specified value)

# Note: must use return inside a code block to return a value; defaults to Void when there is no return
# E.g.: { return 1 + 1 } returns Int; { println("Hello") } returns Void
# Note: generic parameters use the (T: Type) syntax, as part of the function type; no independent BNF rule needed
```

### Error Handling

```yaoxiang
# === Compilation error examples ===

# Error 1: code block return type mismatch
add: (a: Int, b: Int) -> Int = { println(a + b) }
// Error: no return in block, defaults to Void, but signature expects Int
// Correct: add: (a: Int, b: Int) -> Int = a + b
// Or: add: (a: Int, b: Int) -> Int = { return a + b }

# Error 2: using an undeclared type parameter
identity: (x: T) -> T = x
// Error: T is not declared; explicit generic parameters required (RFC-010)
// Correct: identity: (T: Type) -> ((x: T) -> T) = x

# Correct: HM algorithm infers return type
double = (x: Int) => x + x

# Full form (stepwise shorthand)
double: (x: Int) -> Int = (x) => x + x                # Full
double: (x: Int) -> Int = x + x                       # Omit lambda head
double = (x: Int) => x + x                            # Omit return type (HM infers return)
# double = (x) => x + x                               # ❌ Parameter types are not allowed to be omitted on both sides
```

## Trade-offs

### Advantages

- **Syntax unification**: The `name: Signature = LambdaBody` model covers all scenarios
- **Flexible shorthand**: Any part can be omitted whenever HM can infer it
- **Type safety**: The HM algorithm guarantees type safety, avoiding implicit type conversions
- **Recursive support**: HM algorithm and recursive constraints automatically infer types
- **Zero burden**: Smooth transition from full to minimal form

### Disadvantages

- **Migration cost**: Old code needs migration tools for conversion
- **Learning cost**: Need to understand the "full form + arbitrary shorthand" model

## Alternatives

| Approach                    | Description                                          | Why not chosen                                                        |
| --------------------------- | ---------------------------------------------------- | --------------------------------------------------------------------- |
| HM algorithm type inference | Use the Hindley-Milner algorithm to infer types      | ✅ **Adopted**, the standard for modern functional languages          |
| Explicit type declaration   | All types must be written explicitly                 | Violates the principle of simplified syntax and adds boilerplate code |
| Keep the old syntax         | Support both old and new syntax simultaneously       | Syntax fragmentation, high maintenance cost                           |
| fn keyword                  | Introduce fn to distinguish functions from variables | Violates the "function is lambda" design                              |

## Implementation Strategy

### Phases

1. **Phase 1: Syntax parsing and HM algorithm** (v0.3)
   - Implement the new syntax `name = lambda` + HM algorithm type inference
   - Implement default filling for no-arg, no-return cases

2. **Phase 2: Migration tool** (v0.3)
   - Develop the `yaoxiang-migrate --old-to-new` tool
   - Automatically convert old-syntax code

3. **Phase 3: Validation and documentation** (v0.3)
   - Validate the migration of old code
   - Update documentation

### Migration Tool

```bash
# Migrate a single file
yaoxiang-migrate --old-to-new src/main.yaoxiang

# Migrate the entire project
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

# === Shorthand: omit lambda head ===
add: (a: Int, b: Int) -> Int = a + b
main: () -> Void = { println("Hello") }

# === Shorthand: HM inference ===
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

| Risk             | Impact                    | Mitigation                                               |
| ---------------- | ------------------------- | -------------------------------------------------------- |
| Missed migration | Old code fails to compile | Provide migration tools covering all old syntax patterns |
| Parser errors    | Unstable syntax parsing   | Sufficient test coverage                                 |

## Open Questions

> The following questions have been resolved in the design and are recorded in Appendix A.

- ~~Q1: Should the extremely minimal form `main() = body` be kept?~~ → Resolved: kept as
  `main: () -> Void = { ... }`
- ~~Q2: Should the `:` after the function name be kept?~~ → Resolved: optionally kept; however,
  functions with parameters still need to annotate parameter types in either the signature or the
  lambda head
- ~~Q3: Does the HM algorithm support parameter type inference?~~ → Resolved: return value/locals
  can be inferred; parameter types for functions with parameters must be explicitly annotated
- ~~Q4: Should the `fn` keyword be introduced?~~ → Resolved: not introduced; a function is a lambda
- ~~Q5: What is the migration strategy for old code?~~ → Resolved: provide the `yaoxiang-migrate`
  tool
- ~~Q6: How are generic functions used?~~ → Resolved: use the RFC-010 unified syntax `(T: Type)`

---

## Appendix

### Appendix A: Reference of Function Definition Syntax in Various Languages

| Language     | Syntax style                                        | Features                                      |
| ------------ | --------------------------------------------------- | --------------------------------------------- |
| Rust         | `fn add(a: i32, b: i32) -> i32 { ... }`             | Keyword + type annotation                     |
| Haskell      | `add a b = ...` / `add :: Int -> Int -> Int`        | Separate type signature                       |
| OCaml        | `let add a b = ...`                                 | Parameter types can be omitted                |
| MoonBit      | `fn add(a: Int, b: Int): Int { ... }`               | Concise type annotation                       |
| TypeScript   | `const add = (a: number, b: number): number => ...` | Lambda style                                  |
| Scala        | `def add(a: Int, b: Int): Int = { ... }`            | def keyword                                   |
| **YaoXiang** | `name = (a: Int, b: Int) => a + b`                  | **Function = lambda, HM infers return value** |

### Appendix B: Design Decision Records

| Decision              | Decision                                                                        | Date       | Recorder  |
| --------------------- | ------------------------------------------------------------------------------- | ---------- | --------- |
| Syntax style          | New syntax `name: (params) -> Return = body` + HM inference                     | 2026-02-03 | @Moyujang |
| Parameter position    | Parameter names are declared in the signature, unified with RFC-010             | 2026-02-03 | @Moyujang |
| Default filling       | No-arg functions can omit the signature, empty block `{}` is inferred as `Void` | 2026-02-03 | @Moyujang |
| Type inference        | HM algorithm automatically infers; explicit when inference fails                | 2026-01-06 | @Moyujang |
| Old syntax            | Retired, with migration tools provided                                          | 2026-01-06 | @Moyujang |
| fn keyword            | Not introduced                                                                  | 2026-01-06 | @Moyujang |
| Recursive declaration | HM algorithm and recursive constraints automatically infer                      | 2026-01-06 | @Moyujang |

### Appendix C: Glossary

| Term                 | Definition                                                                                                                       |
| -------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| HM algorithm         | The Hindley-Milner type inference algorithm, which automatically infers the types of functions and variables                     |
| Generic              | Using type parameters `(T: Type)` to constrain polymorphic functions, e.g., `identity: (T: Type) -> ((x: T) -> T) = x` (RFC-010) |
| Default type filling | A no-arg, no-return function omits `-> Void`, and the compiler fills it in automatically                                         |
| Syntactic sugar      | Syntactic simplifications that make code easier to read                                                                          |
| Normalization        | Converting syntactic forms to a unified internal representation                                                                  |
| Function is lambda   | A function is essentially a lambda variable, and its type is automatically inferred by the HM algorithm                          |

---

## References

- [MoonBit Language Design](https://moonbitlang.com/)
- [Rust Function Syntax](https://doc.rust-lang.org/book/ch03-03-how-functions-work.html)
- [Haskell Type System](https://www.haskell.org/tutorial/patterns.html)
- [OCaml Type Inference](https://v2.ocaml.org/manual/)
