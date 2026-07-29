---
title: 'RFC-007: Function Definition Syntax Unification'
issue: '#131'
status: 'Accepted'
author: 'Moyujiang'
created: '2025-01-05'
updated: '2026-07-05 (synced to GH Issue #131)'
---

# RFC-007: Function Definition Syntax Unification

## Summary

This RFC finalizes the **function definition syntax** for the YaoXiang language. The unified syntax
`name: (params) -> Return = body` is adopted, which is fully consistent with the
`name: type = value` model in RFC-010.

To avoid ambiguity: when a function has input parameters, the parameter types must be explicitly
specified in either the "signature" or the "lambda head" (at least one place); omitting both will be
rejected.

Code blocks `{ ... }` must use `return` to return values; without `return`, it defaults to returning
`Void`. The expression form `= expr` returns a value directly.

## Motivation

### Why is this feature needed?

1. **Syntax consistency**: Eliminates legacy baggage from old syntax, unifies style
2. **Brevity**: HM algorithm automatically infers types, reduces boilerplate code
3. **Type safety**: HM algorithm ensures type safety; explicit annotation only when inference fails
4. **Language maturity**: HM algorithm is a mature solution in modern functional languages

### Unified Syntax Model

**Core principle**: `name: Signature = LambdaBody`

- **Complete form**: Signature (with parameter names + types + `->` + return type) + Lambda head
  (with parameter names)
- **Shorthand rules**: Omit as much as possible without introducing ambiguity
  - `->` cannot be omitted (function type marker; otherwise it would be parsed as a tuple)
  - **When there are input parameters**, parameter types must appear explicitly in either the
    signature or lambda head
  - Lambda head can be omitted → if the signature already declares parameter names and types
  - Return type can be explicitly specified, or omitted when it can be inferred

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
# === Complete form ===
add: (a: Int, b: Int) -> Int = (a, b) => { a + b }

# === Shorthand forms ===
add: (a: Int, b: Int) -> Int = a + b                 # omit lambda head
add = (a: Int, b: Int) => a + b                      # omit signature

# === No-parameter functions ===
main: () -> Void = () => { println("Hello") }          # complete form
main: () -> Void = { println("Hello") }                # omit lambda head
main = { println("Hello") }                            # simplest form (infers to () -> Void)

# === Generic functions (using RFC-010 unified syntax) ===
identity: (T: Type) -> ((x: T) -> T) = (x) => x         # complete form
identity: (T: Type) -> ((x: T) -> T) = x                # omit lambda head
identity = (x: T) => x                                  # omit signature (lambda head annotates type)

# === Recursive functions ===
factorial: (n: Int) -> Int = (n) => {
    if n <= 1 { return 1 } else { return n * factorial(n - 1) }
}
```

### Syntax Rules

| Scenario               | Syntax                                                 | Description                      |
| ---------------------- | ------------------------------------------------------ | -------------------------------- |
| **Complete form**      | `name: (a: Type, b) -> Ret = (a, b) => { return ... }` | Signature + lambda head complete |
| **Omit lambda head**   | `name: (a: Type, b: Type) -> Ret = { ... }`            | Signature declares parameters    |
| **Omit signature**     | `name = (a: Type, b: Type) => { ... }`                 | Lambda head annotates types      |
| **No-param complete**  | `name: () -> Void = () => { return ... }`              | No-param function complete       |
| **No-param shorthand** | `name: () -> Void = { return ... }`                    | Omit lambda head                 |
| **No-param simplest**  | `name = { return ... }`                                | Simplest no-param-no-return      |

**Note**: Code blocks `{ ... }` must use `return` to return values; without `return`, it defaults to
`Void`. The expression form `= expr` returns a value directly.

**Note**: `->` is the function type marker and cannot be omitted (otherwise it would be parsed as a
tuple).

**Important**: `if` expressions use curly braces `{}` to wrap branches, and do not support
`then/else` keywords:

```yaoxiang
# Correct: use curly braces
if n <= 1 { return 1 } else { return n * factorial(n - 1) }

# Wrong: then/else keywords not supported
# if n <= 1 then return 1 else return n * factorial(n - 1)
```

## Proposal

### HM Algorithm and Higher-rank Polymorphism Support

**Core feature**: The HM algorithm supports higher-rank polymorphism through generic type
annotations.

**Design principles**:

- **Higher-order functions**: When functions are passed as arguments, they need generic constraints
  on their function types
- **Type annotation form**: `(T: Type) -> ((f: (T) -> T, x: T) -> T)` - generic parameters constrain
  function types
- **HM workflow**: Infer function types through generic instantiation, enabling polymorphic function
  composition

**Example explanation**:

```yaoxiang
# ✅ Supports higher-rank polymorphism: generic constrains function type parameter
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = {
    return f(f(x))
}
# Usage: call_twice((x) => x + 1, 5)  # infers T=Int

compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = {
    return f(g(x))
}
# Usage: compose((x) => x * 2, (x) => x + 1, 5)  # infers A=Int, B=Int, C=Int

# ❌ Not supported: higher-order function without generic constraints
# bad_hof: (f, x) => f(f(x))  # HM cannot infer, missing generic parameters
```

**HM inference process**:

1. Identify higher-order function parameters: `f: (T) -> T`
2. Create generic constraints: `(T: Type)`
3. Infer concrete types through generic instantiation
4. Implement polymorphic function composition

### Lambda Expression Syntax Rules

**Important rule**: Code blocks `{ ... }` must use `return` to return values; without `return`, it
defaults to `Void`. The expression form `= expr` returns a value directly.

| Syntax form         | Syntax           | Return method                                                    |
| ------------------- | ---------------- | ---------------------------------------------------------------- |
| **Code block form** | `{ statements }` | Must use `return` to return; without `return` defaults to `Void` |
| **Expression form** | `expression`     | Directly returns expression value                                |

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

# Expression form: directly returns value (no return needed)
add: (a: Int, b: Int) -> Int = a + b            # correct: expression form
main: () -> Void = println("Hello")               # correct: expression form
```

**Core ideas**:

1. Function definitions use HM algorithm for type inference; infer as much as possible, explicitly
   report errors when inference fails
2. **How HM algorithm works**: Automatically infers types through operator type constraints,
   function call relationships, and other context information
3. **Generic support**: Polymorphic functions use generic syntax `(T: Type)` to explicitly constrain
   type parameters (RFC-010/011)
4. **Inference boundaries**: Return types and local variables can be inferred; parameter types of
   functions with parameters need explicit annotation (in either signature or lambda head)
5. No-parameter-no-return functions use `name: () -> Void = { ... }`, unified with RFC-010
6. Retire old syntax, provide migration tools

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
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # infers to (Int, Int) -> Void

# Higher-rank polymorphism: HM supports higher-rank polymorphism through generic type annotations
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = { return f(f(x)) }
compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = { return f(g(x)) }
```

```yaoxiang
# === Function definition: HM algorithm type inference ===

# Standard function: HM algorithm infers return type (parameter types need explicit)
add = (a: Int, b: Int) => a + b            # infers to (a: Int, b: Int) -> Int
main = { println("Hello") }                # infers to () -> Void

# Partial explicit parameters: HM algorithm infers the rest
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # infers to (Int, Int) -> Void
greet: (name: String) -> Void = { println("Hello " + name) }  # infers to (String) -> Void

# Generic function: explicitly constrain polymorphic type parameters (using RFC-010 unified syntax)
identity: (T: Type) -> ((x: T) -> T) = x
map: (T: Type, R: Type) -> ((f: (T) -> R, list: List(T)) -> List(R)) = {
    # Implement map function
    return List(R)()
}

# Recursive function: infer through HM algorithm and recursive constraints
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 } else { return n * factorial(n - 1) }
}

# === Variable assignment: HM algorithm type inference ===

# Explicit type
x: Int = 42

# HM algorithm automatically infers to Int
y = 42                               # infers to Int

# HM algorithm automatically infers to String
name = "YaoXiang"                    # infers to String

# HM algorithm automatically infers to Float
pi = 3.14159                         # infers to Float
```

**HM type inference rules**:

| Scenario                | Syntax                                            | Omissible parts | Example                              |
| ----------------------- | ------------------------------------------------- | --------------- | ------------------------------------ |
| **Complete form**       | `name: (a: Type, b: Type) -> Ret = (a, b) => ...` | None            | Signature + lambda head complete     |
| **Omit lambda head**    | `name: (a: Type, b: Type) -> Ret = ...`           | Lambda head     | Signature declares parameters        |
| **Omit signature**      | `name = (a: Type, b: Type) => ...`                | Signature       | Lambda head provides parameter types |
| **Omit return Ret**     | `name: (a: Type, b: Type) -> = ...`               | Return type     | HM infers return type                |
| **No-param complete**   | `name: () -> Void = () => { ... }`                | None            | No-param function complete           |
| **No-param shorthand**  | `name: () -> Void = { ... }`                      | Lambda head     | Omit `() =>`                         |
| **No-param simplest**   | `name = { ... }`                                  | All             | Simplest no-param-no-return          |
| **Variable assignment** | `name = value`                                    | Type            | HM infers type                       |
| **Explicit variable**   | `name: Type = value`                              | None            | Explicit type annotation             |

**Core principles**:

- `->` is the function type marker and cannot be omitted (otherwise it would be parsed as a tuple)
- Return type `Ret` can be omitted, inferred by HM based on function body
- When there are input parameters, parameter types must appear explicitly (in either signature or
  lambda head)
- Other parts can be omitted when they can be inferred and do not introduce ambiguity
- No implicit type conversions, avoiding JavaScript-like chaos

## Detailed Design

### Syntactic Sugar Expansion

Regardless of omission, everything is normalized to a unified intermediate representation:

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
               | identifier '=' block                    # Simplest form: no-param-no-return

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
           | expression                  # Expression statement (executes but doesn't return)
           | 'return' expression         # Return statement (returns specified value)

# Note: code blocks must use return to return values; without return, defaults to Void
# Example: { return 1 + 1 } returns Int; { println("Hello") } returns Void
# Note: Generic parameters use (T: Type) syntax, as part of function type, no separate BNF rule needed
```

### Error Handling

```yaoxiang
# === Compile error examples ===

# Error 1: Code block return type mismatch
add: (a: Int, b: Int) -> Int = { println(a + b) }
// Error: no return in block, defaults to Void, but signature expects Int
// Correct: add: (a: Int, b: Int) -> Int = a + b
// Or: add: (a: Int, b: Int) -> Int = { return a + b }

# Error 2: Using undeclared type parameter
identity: (x: T) -> T = x
// Error: T not declared; needs explicit generic parameter (RFC-010)
// Correct: identity: (T: Type) -> ((x: T) -> T) = x

# Correct: HM algorithm infers return type
double = (x: Int) => x + x

# Complete form (gradual shorthand)
double: (x: Int) -> Int = (x) => x + x                # complete
double: (x: Int) -> Int = x + x                       # omit lambda head
double = (x: Int) => x + x                            # omit return type (HM infers return)
# double = (x) => x + x                               # ❌ Parameter types cannot be omitted on both sides
```

## Trade-offs

### Advantages

- **Unified syntax**: `name: Signature = LambdaBody` model covers all scenarios
- **Flexible shorthand**: Any part can be omitted when HM can infer it
- **Type safety**: HM algorithm ensures type safety, avoids implicit type conversions
- **Recursive support**: HM algorithm and recursive constraints automatically infer types
- **Zero overhead**: Smooth transition from complete to simplest form

### Disadvantages

- **Migration cost**: Old code needs migration tools for conversion
- **Learning cost**: Need to understand the "complete form + arbitrary shorthand" model

## Alternative Approaches

| Approach                  | Description                                          | Why not chosen                                           |
| ------------------------- | ---------------------------------------------------- | -------------------------------------------------------- |
| HM algorithm inference    | Use Hindley-Milner algorithm to infer types          | ✅ **Adopted**, modern functional language standard      |
| Explicit type declaration | All types must be explicitly written                 | Violates simplification principle, increases boilerplate |
| Keep old syntax           | Support both old and new syntax simultaneously       | Syntax fragmentation, high maintenance cost              |
| fn keyword                | Introduce fn to distinguish functions from variables | Violates "function is lambda" design                     |

## Implementation Strategy

### Phase Division

1. **Phase 1: Syntax parsing and HM algorithm** (v0.3)
   - Implement new syntax `name = lambda` + HM algorithm type inference
   - Implement default filling for no-parameter-no-return functions

2. **Phase 2: Migration tool** (v0.3)
   - Develop `yaoxiang-migrate --old-to-new` tool
   - Automatically convert old syntax code

3. **Phase 3: Verification and documentation** (v0.3)
   - Complete verification of old code migration
   - Documentation update

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
add = (a: Int, b: Int) => a + b              # infers to (a: Int, b: Int) -> Int
main = { println("Hello") }                  # infers to () -> Void

# === Simplest form ===
main = {                                      # equivalent to main: () -> Void = { ... }
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

- ~~Q1: Should we keep the ultra-minimal syntax `main() = body`?~~ → Resolved: Kept as
  `main = { ... }`
- ~~Q2: Should the `:` after function name be kept?~~ → Resolved: Optional to keep; but functions
  with parameters still need parameter types annotated in signature or lambda head
- ~~Q3: Does HM algorithm support parameter type inference?~~ → Resolved: Return types/local
  variables can be inferred; parameter types of functions with parameters need explicit annotation
- ~~Q4: Should we introduce `fn` keyword?~~ → Resolved: No introduction; functions are lambdas
- ~~Q5: What is the migration strategy for old code?~~ → Resolved: Provide `yaoxiang-migrate` tool
- ~~Q6: How to use generic functions?~~ → Resolved: Use RFC-010 unified syntax `(T: Type)`

---

## Appendices

### Appendix A: Function Definition Syntax Reference for Various Languages

| Language     | Syntax style                                        | Characteristics                              |
| ------------ | --------------------------------------------------- | -------------------------------------------- |
| Rust         | `fn add(a: i32, b: i32) -> i32 { ... }`             | Keyword + type annotation                    |
| Haskell      | `add a b = ...` / `add :: Int -> Int -> Int`        | Type signature separated                     |
| OCaml        | `let add a b = ...`                                 | Parameter types can be omitted               |
| MoonBit      | `fn add(a: Int, b: Int): Int { ... }`               | Concise type annotation                      |
| TypeScript   | `const add = (a: number, b: number): number => ...` | Lambda style                                 |
| Scala        | `def add(a: Int, b: Int): Int = { ... }`            | def keyword                                  |
| **YaoXiang** | `name = (a: Int, b: Int) => a + b`                  | **Function = lambda, HM infers return type** |

### Appendix B: Design Decision Records

| Decision              | Decision                                                                     | Date       | Recorder   |
| --------------------- | ---------------------------------------------------------------------------- | ---------- | ---------- |
| Syntax style          | New syntax `name: (params) -> Return = body` + HM inference                  | 2026-02-03 | @Moyujiang |
| Parameter position    | Parameter names declared in signature, unified with RFC-010                  | 2026-02-03 | @Moyujiang |
| Default filling       | No-parameter functions can omit signature, empty block `{}` infers to `Void` | 2026-02-03 | @Moyujiang |
| Type inference        | HM algorithm automatically infers, explicit when inference fails             | 2026-01-06 | @Moyujiang |
| Old syntax            | Retire, provide migration tool                                               | 2026-01-06 | @Moyujiang |
| fn keyword            | Not introduced                                                               | 2026-01-06 | @Moyujiang |
| Recursive declaration | HM algorithm and recursive constraints automatically infer                   | 2026-01-06 | @Moyujiang |

### Appendix C: Glossary

| Term                 | Definition                                                                                                                       |
| -------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| HM algorithm         | Hindley-Milner type inference algorithm, automatically infers types for functions and variables                                  |
| Generics             | Using type parameters `(T: Type)` to constrain polymorphic functions, e.g., `identity: (T: Type) -> ((x: T) -> T) = x` (RFC-010) |
| Default type filling | Omit `-> Void` for no-parameter-no-return functions, compiler automatically fills in                                             |
| Syntactic sugar      | Syntax simplifications that make code more readable                                                                              |
| Normalization        | Convert syntax forms to unified internal representation                                                                          |
| Function is lambda   | Functions are essentially lambda variables, types automatically inferred through HM algorithm                                    |

---

## References

- [MoonBit Language Design](https://moonbitlang.com/)
- [Rust Function Syntax](https://doc.rust-lang.org/book/ch03-03-how-functions-work.html)
- [Haskell Type System](https://www.haskell.org/tutorial/patterns.html)
- [OCaml Type Inference](https://v2.ocaml.org/manual/)
