---
title: 'RFC-007: Function Definition Syntax Unification'
issue: '#131'
status: 'Accepted'
author: 'Mo Yu Jiang'
created: '2025-01-05'
updated: '2026-09-15'
---

# RFC-007: Function Definition Syntax Unification

> **Related**: Where the "no-parameter minimum" `name = { ... }` from RFC-010 overlaps with
> RFC-010's "value block" syntax position, the ruling is **content determines type** — `=>` is
> always function, `Fn` annotation is function, non-`Fn` annotation is block value, and inference is
> based on content when unannotated. See [RFC-010a](./010a-tail-expression-and-return.md) Appendix
> D.
>
> **Related Addition**: Statement termination and newline rules (`;` explicit separation, newline
> termination, line continuation exceptions) inside function bodies (`{ ... }` code blocks) are
> defined by [RFC-038 (Draft)](./038-statement-termination.md), this RFC does not cover them.

## Summary

This RFC establishes the final solution for **function definition syntax** in the YaoXiang language.
Using a unified syntax `name: (params) -> Return = body`, completely consistent with RFC-010's
`name: type = value` model.

To avoid ambiguity: when a function has input parameters, parameter types must be explicitly
annotated in either the "signature" or "lambda head" (at least one place); omitting from both sides
will be rejected.

The value of a code block `{ ... }` is given by the **tail expression**; `return` is a non-local
exit of type `Never` (see [RFC-010a](./010a-tail-expression-and-return.md)). The expression form
`= expr` directly gives the value.

## Motivation

### Why is this feature needed?

1. **Syntax Consistency**: Eliminate historical baggage of old syntax, unify style
2. **Brevity**: HM algorithm infers types automatically, reducing boilerplate
3. **Type Safety**: HM algorithm ensures type safety, explicit annotation only when inference fails
4. **Language Maturity**: HM algorithm is a mature solution in modern functional languages

### Unified Syntax Model

**Core Principle**: `name: Signature = LambdaBody`

- **Full form**: Signature (with parameter names + types + `->` + return type) + Lambda head (with
  parameter names)
- **Shorthand rules**: Omit as much as possible without introducing ambiguity
  - `->` cannot be omitted (function type marker, otherwise parsed as tuple)
  - **When there are input parameters**, parameter types must explicitly appear in either the
    signature or lambda head
  - Lambda head can be omitted → if signature already declares parameter names and types
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
# === Full Form ===
add: (a: Int, b: Int) -> Int = (a, b) => { a + b }

# === Shorthand Forms ===
add: (a: Int, b: Int) -> Int = a + b                 # omit lambda head
add = (a: Int, b: Int) => a + b                      # omit signature

# === No-Parameter Functions ===
main: () -> Void = () => { println("Hello") }          # full form
main: () -> Void = { println("Hello") }                # omit lambda head
main: () -> Void = { println("Hello") }                            # minimum form (inferred as () -> Void)

# === Generic Functions (using RFC-010 unified syntax) ===
identity: (T: Type) -> ((x: T) -> T) = (x) => x         # full form
identity: (T: Type) -> ((x: T) -> T) = x                # omit lambda head
identity = (x: T) => x                                  # omit signature (lambda head annotates type)

# === Recursive Functions ===
factorial: (n: Int) -> Int = (n) => {
    if n <= 1 { return 1 } else { return n * factorial(n - 1) }
}
```

### Syntax Rules

| Scenario               | Syntax                                                 | Description                 |
| ---------------------- | ------------------------------------------------------ | --------------------------- |
| **Full form**          | `name: (a: Type, b) -> Ret = (a, b) => { return ... }` | Signature + complete lambda |
| **Omit lambda head**   | `name: (a: Type, b: Type) -> Ret = { ... }`            | Signature declares params   |
| **Omit signature**     | `name = (a: Type, b: Type) => { ... }`                 | Lambda head annotates types |
| **No-param full**      | `name: () -> Void = () => { return ... }`              | No-param function full      |
| **No-param shorthand** | `name: () -> Void = { return ... }`                    | Omit lambda head            |
| **No-param minimum**   | `name = { return ... }`                                | No-param no-return minimum  |

**Note**: The value of code block `{ ... }` is given by the **tail expression** (unique exit point);
`return` is a non-local exit of type `Never`, exiting the nearest function boundary. The expression
form `= expr` directly gives the value. See [RFC-010a](./010a-tail-expression-and-return.md) for
details.

**Note**: `->` is the function type marker, cannot be omitted (otherwise parsed as tuple).

**Important**: `if` expressions use braces `{}` to wrap branches, `then/else` keywords are not
supported:

```yaoxiang
# Correct: use braces
if n <= 1 { return 1 } else { return n * factorial(n - 1) }

# Incorrect: then/else keywords not supported
# if n <= 1 then return 1 else return n * factorial(n - 1)
```

## Proposal

### HM Algorithm and Higher-Rank Polymorphism Support

**Core Feature**: HM algorithm supports higher-rank polymorphism through generic type annotations

**Design Principles**:

- **Higher-order functions**: When functions are passed as arguments, generic constraints on
  function types are needed
- **Type annotation form**: `(T: Type) -> ((f: (T) -> T, x: T) -> T)` - generic parameters constrain
  function types
- **HM workflow**: Infer function types through generic parameter instantiation, enabling
  polymorphic function composition

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
4. Implement polymorphic function composition

### Lambda Expression Syntax Rules

**Important Rule**: The value of code block `{ ... }` is given by the **tail expression** (unique
exit point); `return` is a non-local exit of type `Never`, exiting the nearest function boundary.
The expression form `= expr` directly gives the value. See
[RFC-010a](./010a-tail-expression-and-return.md) for details.

| Syntax Form         | Syntax           | Value Exit                             |
| ------------------- | ---------------- | -------------------------------------- |
| **Code block form** | `{ statements }` | Tail expression (empty `{}` is `Void`) |
| **Expression form** | `expression`     | Expression value                       |
| **`return`**        | `return e`       | Non-local function exit, type `Never`  |

**Example**:

```yaoxiang
main: () -> Void = { println("Hello") }         # tail expression is Void
add: (a: Int, b: Int) -> Int = { a + b }        # tail expression gives value
empty: () -> Void = {}                          # empty block → Void

# Early return: use return
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

# Expression form: directly gives value
add: (a: Int, b: Int) -> Int = a + b            # correct: expression form
main: () -> Void = println("Hello")               # correct: expression form
```

**Core Ideas**:

1. Function definitions use HM algorithm for type inference, infer as much as possible, explicitly
   error when inference fails
2. **How HM algorithm works**: Automatically infers types through context like operator type
   constraints and function call relationships
3. **Generic support**: Polymorphic functions use generic syntax `(T: Type)` to explicitly constrain
   type parameters (RFC-010/011)
4. **Inference boundaries**: Return types and local variables are inferable; parameter types for
   functions with parameters need explicit annotation (in signature or lambda head)
5. No-parameter no-return functions use `name: () -> Void = { ... }`, unified with RFC-010
6. Retire old syntax, provide migration tools

**Type Inference Examples**:

```yaoxiang
# Generic function: explicit type parameters (using the RFC-010 unified syntax)
identity: (T: Type) -> ((x: T) -> T) = x
map: (T: Type, R: Type) -> ((f: (T) -> R, list: List(T)) -> List(R)) = {
    result = List(R)()
    for item in list { result.push(f(item)) }
    return result
}

# Polymorphic functions: defined through explicit generic constraints (RFC-010/011)
add: (T: Add) -> ((a: T, b: T) -> T) = a + b
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # infers (Int, Int) -> Void

# Higher-rank polymorphism: HM supports higher-rank polymorphism through generic type annotations
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = { return f(f(x)) }
compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = { return f(g(x)) }
```

```yaoxiang
# === Function Definition: HM Algorithm Type Inference ===

# Standard functions: HM algorithm infers return type (parameter types need explicit)
add = (a: Int, b: Int) => a + b            # infers to (a: Int, b: Int) -> Int
main: () -> Void = { println("Hello") }                # infers to () -> Void

# Partially explicit parameters: HM algorithm infers remaining parts
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # infers to (Int, Int) -> Void
greet: (name: String) -> Void = { println("Hello " + name) }  # infers to (String) -> Void

# Generic function: explicitly constrain polymorphic type parameters (using the RFC-010 unified syntax)
identity: (T: Type) -> ((x: T) -> T) = x
map: (T: Type, R: Type) -> ((f: (T) -> R, list: List(T)) -> List(R)) = {
    # implement map function
    return List(R)()
}

# Recursive functions: inferred through HM algorithm and recursive constraints
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 } else { return n * factorial(n - 1) }
}

# === Variable Assignment: HM Algorithm Type Inference ===

# Explicit type
x: Int = 42

# HM algorithm automatically infers to Int
y = 42                               # infers to Int

# HM algorithm automatically infers to String
name = "YaoXiang"                    # infers to String

# HM algorithm automatically infers to Float
pi = 3.14159                         # infers to Float
```

**HM Type Inference Rules**:

| Scenario                | Syntax                                            | Omissible Parts | Example                    |
| ----------------------- | ------------------------------------------------- | --------------- | -------------------------- |
| **Full form**           | `name: (a: Type, b: Type) -> Ret = (a, b) => ...` | None            | Signature + lambda head    |
| **Omit lambda head**    | `name: (a: Type, b: Type) -> Ret = ...`           | Lambda head     | Signature declares params  |
| **Omit signature**      | `name = (a: Type, b: Type) => ...`                | Signature       | Lambda head provides types |
| **Omit return Ret**     | `name: (a: Type, b: Type) -> = ...`               | Return type     | HM infers return type      |
| **No-param full**       | `name: () -> Void = () => { ... }`                | None            | No-param function full     |
| **No-param shorthand**  | `name: () -> Void = { ... }`                      | Lambda head     | Omit `() =>`               |
| **No-param minimum**    | `name = { ... }`                                  | All             | No-param no-return min     |
| **Variable assignment** | `name = value`                                    | Type            | HM infers type             |
| **Explicit variable**   | `name: Type = value`                              | None            | Explicit type annotation   |

**Core Principles**:

- `->` is the function type marker, cannot be omitted (otherwise parsed as tuple)
- Return type `Ret` can be omitted, HM infers from function body
- When there are input parameters, parameter types must explicitly appear (in signature or lambda
  head)
- Other parts can be omitted when inferable and not introducing ambiguity
- No implicit type conversions, avoiding JavaScript-style chaos

## Detailed Design

### Syntax Sugar Expansion

Regardless of omission, everything is normalized to a unified intermediate representation:

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
               | identifier '=' block                    # minimum form: no-param no-return

identifier ::= [a-zA-Z_][a-zA-Z0-9_]*

type_expr ::= identifier                     # type reference
       | '()'                          # unit type
       | '(' parameters ')' '->' type_expr   # function type (param names in signature)
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
           | expression                  # expression statement (executes but no return)
           | 'return' expression         # return statement (returns specified value)

# Note: code blocks must use return to return values; no return defaults to Void
# e.g.: { return 1 + 1 } returns Int; { println("Hello") } returns Void
# Note: generic parameters use (T: Type) syntax, as part of function type, no separate BNF rule needed
```

### Error Handling

```yaoxiang
# === Compile Error Examples ===

# Error 1: code block return type mismatch
add: (a: Int, b: Int) -> Int = { println(a + b) }
// Error: no return in block, defaults to Void, but signature expects Int
// Correct: add: (a: Int, b: Int) -> Int = a + b
// Or: add: (a: Int, b: Int) -> Int = { return a + b }

# Error 2: using undeclared type parameter
identity: (x: T) -> T = x
// Error: T not declared; needs explicit generic parameter (RFC-010)
// Correct: identity: (T: Type) -> ((x: T) -> T) = x

# Correct: HM algorithm infers return type
double = (x: Int) => x + x

# Full form (progressively shortened)
double: (x: Int) -> Int = (x) => x + x                # full
double: (x: Int) -> Int = x + x                       # omit lambda head
double = (x: Int) => x + x                            # omit return type (HM infers)
# double = (x) => x + x                               # ❌ parameter types cannot be omitted on both sides
```

## Trade-offs

### Advantages

- **Syntax unification**: `name: Signature = LambdaBody` model covers all scenarios
- **Flexible shorthand**: Any part can be omitted when HM can infer
- **Type safety**: HM algorithm ensures type safety, avoids implicit type conversions
- **Recursive support**: HM algorithm and recursive constraints auto-infer types
- **Zero burden**: Smooth transition from full to minimum form

### Disadvantages

- **Migration cost**: Old code needs migration tool conversion
- **Learning cost**: Need to understand "full form + arbitrary shorthand" model

## Alternative Solutions

| Solution        | Description                                          | Why Not Chosen                                                  |
| --------------- | ---------------------------------------------------- | --------------------------------------------------------------- |
| HM algorithm    | Use Hindley-Milner algorithm for type inference      | ✅ **Adopted**, modern functional language standard             |
| Explicit type   | All types must be explicitly written                 | Violates syntax simplification principle, increases boilerplate |
| Keep old syntax | Support both old and new syntax                      | Syntax fragmentation, high maintenance cost                     |
| fn keyword      | Introduce fn to distinguish functions from variables | Violates "functions are lambdas" design                         |

## Implementation Strategy

### Phase Division

1. **Phase 1: Syntax Parsing and HM Algorithm** (v0.3)
   - Implement new syntax `name = lambda` + HM algorithm type inference
   - Implement default filling for no-parameter no-return

2. **Phase 2: Migration Tool** (v0.3)
   - Develop `yaoxiang-migrate --old-to-new` tool
   - Automatically convert old syntax code

3. **Phase 3: Validation and Documentation** (v0.3)
   - Validate old code migration completion
   - Update documentation

### Migration Tool

```bash
# Migrate single file
yaoxiang-migrate --old-to-new src/main.yaoxiang

# Migrate entire project
yaoxiang-migrate --old-to-new --recursive src/

# Preview migration (no file modification)
yaoxiang-migrate --old-to-new --dry-run src/main.yaoxiang
```

Migration rules:

```yaoxiang
# Old syntax
add(Int, Int) -> Int = (a, b) => { a + b }
main() -> Int = { println("Hello"); 0 }
main() = { println("Hello") }

# === New syntax: full form (complete signature + complete lambda head)===
add: (a: Int, b: Int) -> Int = (a, b) => a + b
main: () -> Void = () => { println("Hello") }

# === Shorthand: omit lambda head ===
add: (a: Int, b: Int) -> Int = a + b
main: () -> Void = { println("Hello") }

# === Shorthand: HM inference ===
add = (a: Int, b: Int) => a + b              # infers to (a: Int, b: Int) -> Int
main: () -> Void = { println("Hello") }                  # infers to () -> Void

# === Minimum form ===
main: () -> Void = {                                      # equivalent to main: () -> Void = { ... }
    println("Hello")
}
```

### Dependencies

- No external dependencies
- Can be implemented independently

### Risks

| Risk           | Impact                    | Mitigation                                            |
| -------------- | ------------------------- | ----------------------------------------------------- |
| Migration miss | Old code fails to compile | Provide migration tool, cover all old syntax patterns |
| Parser errors  | Unstable syntax parsing   | Adequate test coverage                                |

## Open Questions

> The following questions have been resolved in the design, recorded in Appendix A.

- ~~Q1: Should the ultra-minimal `main() = body` syntax be preserved?~~ → Resolved: Preserved as
  `main: () -> Void = { ... }`
- ~~Q2: Should the `:` after function name be preserved?~~ → Resolved: Optionally preserved; but
  functions with parameters still need parameter types annotated in signature or lambda head
- ~~Q3: Does HM algorithm support parameter type inference?~~ → Resolved:
  Return/局部可推断；有参函数的参数类型需显式标注
- ~~Q4: Should a `fn` keyword be introduced?~~ → Resolved: Not introduced, functions are lambdas
- ~~Q5: What is the migration strategy for old code?~~ → Resolved: Provide `yaoxiang-migrate` tool
- ~~Q6: How to use generic functions?~~ → Resolved: Use RFC-010 unified syntax `(T: Type)`

---

## Appendices

### Appendix A: Function Definition Syntax Reference by Language

| Language     | Syntax Style                                        | Characteristics                         |
| ------------ | --------------------------------------------------- | --------------------------------------- |
| Rust         | `fn add(a: i32, b: i32) -> i32 { ... }`             | Keyword + type annotation               |
| Haskell      | `add a b = ...` / `add :: Int -> Int -> Int`        | Type signature separation               |
| OCaml        | `let add a b = ...`                                 | Parameter types can be omitted          |
| MoonBit      | `fn add(a: Int, b: Int): Int { ... }`               | Concise type annotation                 |
| TypeScript   | `const add = (a: number, b: number): number => ...` | Lambda style                            |
| Scala        | `def add(a: Int, b: Int): Int = { ... }`            | def keyword                             |
| **YaoXiang** | `name = (a: Int, b: Int) => a + b`                  | **Function = lambda, HM infers return** |

### Appendix B: Design Decision Record

| Decision           | Decision                                                                 | Date       | Recorder     |
| ------------------ | ------------------------------------------------------------------------ | ---------- | ------------ |
| Syntax style       | New syntax `name: (params) -> Return = body` + HM inference              | 2026-02-03 | @Mo Yu Jiang |
| Parameter location | Parameter names declared in signature, unified with RFC-010              | 2026-02-03 | @Mo Yu Jiang |
| Default filling    | No-param functions can omit signature, empty block `{}` infers to `Void` | 2026-02-03 | @Mo Yu Jiang |
| Type inference     | HM algorithm auto-infers, explicit when cannot infer                     | 2026-01-06 | @Mo Yu Jiang |
| Old syntax         | Retire, provide migration tool                                           | 2026-01-06 | @Mo Yu Jiang |
| fn keyword         | Not introduced                                                           | 2026-01-06 | @Mo Yu Jiang |
| Recursive decl     | HM algorithm and recursive constraints auto-infer                        | 2026-01-06 | @Mo Yu Jiang |

### Appendix C: Glossary

| Term                 | Definition                                                                                                                       |
| -------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| HM algorithm         | Hindley-Milner type inference algorithm, automatically infers function and variable types                                        |
| Generics             | Using type parameters `(T: Type)` to constrain polymorphic functions, e.g., `identity: (T: Type) -> ((x: T) -> T) = x` (RFC-010) |
| Default type fill    | No-param no-return functions omit `-> Void`, compiler auto-fills                                                                 |
| Syntax sugar         | Syntax simplifications that make code more readable                                                                              |
| Normalization        | Convert syntax forms to unified internal representation                                                                          |
| Functions as lambdas | Functions are essentially lambda variables, types auto-inferred through HM algorithm                                             |

---

## References

- [MoonBit Language Design](https://moonbitlang.com/)
- [Rust Function Syntax](https://doc.rust-lang.org/book/ch03-03-how-functions-work.html)
- [Haskell Type System](https://www.haskell.org/tutorial/patterns.html)
- [OCaml Type Inference](https://v2.ocaml.org/manual/)
