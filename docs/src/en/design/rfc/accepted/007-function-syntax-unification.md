```yaml
---
title: "RFC-007: Function Definition Syntax Unification Scheme"
---

# RFC-007: Function Definition Syntax Unification Scheme

> **Status**: Accepted
> **Author**: Mo Yu Jiang
> **Created**: 2025-01-05
> **Last Updated**: 2026-03-21 (aligned type constructor rules with code block return semantics)

## Abstract

This RFC establishes the final scheme for **function definition syntax** in YaoXiang language. Using the unified syntax `name: (params) -> Return = body`, it is fully consistent with the `name: type = value` model from RFC-010.

To avoid ambiguity: when a function has input parameters, the parameter types must be explicitly annotated in either the "signature" or the "lambda head" (at least one); omitting both is rejected.

The last expression in a code block `{ ... }` is used as the return value; an empty block `{}` returns `Void`.

## Motivation

### Why is this feature needed?

1. **Syntax Consistency**: Eliminates legacy baggage of old syntax, unifies style
2. **Conciseness**: HM algorithm automatically infers types, reducing boilerplate code
3. **Type Safety**: HM algorithm guarantees type safety; explicit annotation only when inference is impossible
4. **Language Maturity**: HM algorithm is a mature solution in modern functional languages

### Unified Syntax Model

**Core Principle**: `name: Signature = LambdaBody`

- **Full form**: Signature (with parameter names + types + `->` + return type) + Lambda head (with parameter names)
- **Abbreviation rules**: Omit where possible without introducing ambiguity
  - `->` cannot be omitted (marker of function type, otherwise parsed as tuple)
  - **When there are input parameters**, parameter types must explicitly appear in either the signature or the lambda head
  - Lambda head can be omitted → if signature already declares parameter names and types
  - Return type can be explicitly annotated, or omitted when inferrable

```yaoxiang
# Full form (complete signature + complete lambda head)
add: (a: Int, b: Int) -> Int = (a, b) => a + b

# Abbreviated: omit lambda head (signature already declares parameters)
add: (a: Int, b: Int) -> Int = a + b

# Abbreviated: omit signature (lambda head annotates parameter types)
add = (a: Int, b: Int) => a + b

# ❌ Error: parameter types not annotated on either side
# add = (a, b) => a + b
```

### Design Goals

```yaoxiang
# === Full Form ===
add: (a: Int, b: Int) -> Int = (a, b) => { a + b }

# === Abbreviated Forms ===
add: (a: Int, b: Int) -> Int = a + b                 # omit lambda head
add = (a: Int, b: Int) => a + b                      # omit signature

# === No-Parameter Functions ===
main: () -> Void = () => { println("Hello") }          # full form
main: () -> Void = { println("Hello") }                # omit lambda head
main = { println("Hello") }                            # minimal form (inferred as () -> Void)

# === Generic Functions (using RFC-010 unified syntax) ===
identity: (T: Type) -> ((x: T) -> T) = (x) => x         # full form
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
| **Full form** | `name: (a: Type, b) -> Ret = (a, b) => { return ... }` | Complete signature + lambda head |
| **Omit lambda head** | `name: (a: Type, b: Type) -> Ret = { ... }` | Signature already declares parameters |
| **Omit signature** | `name = (a: Type, b: Type) => { ... }` | Lambda head provides parameter types |
| **No-param full** | `name: () -> Void = () => { return ... }` | No-param function full form |
| **No-param abbreviated** | `name: () -> Void = { return ... }` | Omit lambda head |
| **No-param minimal** | `name = { return ... }` | Minimal no-param-no-return |

**Note**: The last expression in a block `{ ... }` is used as the return value; use `return` for early termination. Empty block `{}` is inferred as `Void`.

**Note**: `->` is the marker of function type and cannot be omitted (otherwise it will be parsed as a tuple).

**Important**: `if` expressions use curly braces `{}` to wrap branches, and do not support `then/else` keywords:
```yaoxiang
# Correct: use curly braces
if n <= 1 { return 1 } else { return n * factorial(n - 1) }

# Incorrect: then/else keywords not supported
# if n <= 1 then return 1 else return n * factorial(n - 1)
```

## Proposal

### HM Algorithm and Higher-Rank Polymorphism Support

**Core Feature**: HM algorithm supports higher-rank polymorphism through generic type annotations.

**Design Principle**:
- **Higher-order functions**: When functions are passed as arguments, generic constraints are needed to specify their function types
- **Type annotation form**: `(T: Type) -> ((f: (T) -> T, x: T) -> T)` - generic parameters constrain function types
- **HM workflow**: Infer function types through generic parameters, enabling polymorphic function composition

**Example explanation**:
```yaoxiang
# ✅ Support higher-rank polymorphism: generic constrains function type parameter
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = {
    return f(f(x))
}
# Usage: call_twice((x) => x + 1, 5)  # infers T=Int

compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = {
    return f(g(x))
}
# Usage: compose((x) => x * 2, (x) => x + 1, 5)  # infers A=Int, B=Int, C=Int

# ❌ Unsupported: higher-order function without generic constraint
# bad_hof: (f, x) => f(f(x))  # HM cannot infer, missing generic parameter
```

**HM Inference Process**:
1. Identify higher-order function parameters: `f: (T) -> T`
2. Create generic constraint: `(T: Type)`
3. Infer concrete types through generic instantiation
4. Achieve polymorphic function composition

### Lambda Expression Syntax Rules

**Important rule**: Code blocks `{ ... }` return the value of their last expression; use `return` for early termination. Empty block `{}` returns `Void`.

| Syntax form | Syntax | Return method |
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

# Expression form: directly return value (no return needed)
add: (a: Int, b: Int) -> Int = a + b            # correct: expression form
main: () -> Void = println("Hello")               # correct: expression form
```

**Core Ideas**:
1. Function definitions use HM algorithm for type inference, inferring where possible, explicit error when impossible
2. **HM algorithm working principle**: Automatically infer types through contextual information such as operator type constraints and function call relationships
3. **Generic support**: Polymorphic functions use generic syntax `(T: Type)` to explicitly constrain type parameters (RFC-010/011)
4. **Inference boundaries**: Return type and local variables can be inferred; parameter types for functions with parameters need explicit annotation (one of signature or lambda head)
5. No-parameter no-return functions use `name: () -> Void = { ... }`, unified with RFC-010
6. Old syntax is deprecated, migration tools provided

**Type inference examples**:
```yaoxiang
# Generic function: explicit type parameter (using RFC-010 unified syntax)
identity: (T: Type) -> ((x: T) -> T) = x
map: (T: Type, R: Type) -> ((f: (T) -> R, list: List(T)) -> List(R)) = {
    result = List(R)()
    for item in list { result.push(f(item)) }
    return result
}

# Polymorphic function: defined through explicit generic constraint (RFC-010/011)
add: (T: Add) -> ((a: T, b: T) -> T) = a + b
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # inferred as (Int, Int) -> Void

# Higher-rank polymorphism: implement HM higher-rank polymorphism through generic type annotations
call_twice: (T: Type) -> ((f: (T) -> T, x: T) -> T) = { return f(f(x)) }
compose: (A: Type, B: Type, C: Type) -> ((f: (B) -> C, g: (A) -> B, x: A) -> C) = { return f(g(x)) }
```

```yaoxiang
# === Function Definition: HM Algorithm Type Inference ===

# Standard function: HM algorithm infers return type (parameter types need explicit)
add = (a: Int, b: Int) => a + b            # inferred as (a: Int, b: Int) -> Int
main = { println("Hello") }                # inferred as () -> Void

# Partially explicit parameters: HM algorithm infers remaining parts
print_sum: (a: Int, b: Int) -> Void = { println(a + b) }  # inferred as (Int, Int) -> Void
greet: (name: String) -> Void = { println("Hello " + name) }  # inferred as (String) -> Void

# Generic function: explicitly constrain polymorphic type parameters (using RFC-010 unified syntax)
identity: (T: Type) -> ((x: T) -> T) = x
map: (T: Type, R: Type) -> ((f: (T) -> R, list: List(T)) -> List(R)) = {
    # implement map function
    return List(R)()
}

# Recursive function: infer through HM algorithm and recursive constraints
factorial: (n: Int) -> Int = {
    if n <= 1 { 1 } else { n * factorial(n - 1) }
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

**HM Type Inference Rules**:

| Scenario | Syntax | Omissible parts | Example |
|----------|--------|-----------------|---------|
| **Full form** | `name: (a: Type, b: Type) -> Ret = (a, b) => ...` | None | Signature + lambda head complete |
| **Omit lambda head** | `name: (a: Type, b: Type) -> Ret = ...` | Lambda head | Signature already declares parameters |
| **Omit signature** | `name = (a: Type, b: Type) => ...` | Signature | Lambda head provides parameter types |
| **Omit return Ret** | `name: (a: Type, b: Type) -> = ...` | Return type | HM infers return type |
| **No-param full** | `name: () -> Void = () => { ... }` | None | No-param function full form |
| **No-param abbreviated** | `name: () -> Void = { ... }` | Lambda head | Omit `() =>` |
| **No-param minimal** | `name = { ... }` | All | Minimal no-param-no-return |
| **Variable assignment** | `name = value` | Type | HM infers type |
| **Explicit variable** | `name: Type = value` | None | Explicit type annotation |

**Core Principles**:
- `->` is the marker of function type and cannot be omitted (otherwise parsed as tuple)
- Return type `Ret` can be omitted, inferred by HM from function body
- When there are input parameters, parameter types must explicitly appear (in either signature or lambda head)
- Other parts can be omitted when inferrable and without ambiguity
- No implicit type conversions, avoiding JavaScript-style chaos

## Detailed Design

### Syntax Sugar Expansion

Regardless of omission, everything is normalized to unified intermediate representation:

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
               | identifier '=' block                    # minimal form: no-param-no-return

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

# Note: code block returns the value of its last expression; empty block {} infers to Void
# For example: { 1 + 1 } returns Int; { println("Hello") } returns Void
# Note: generic parameters use (T: Type) syntax, as part of function type, no separate BNF rule needed
```

### Error Handling

```yaoxiang
# === Compile Error Examples ===

# Error 1: code block return type mismatch
add: (a: Int, b: Int) -> Int = { println(a + b) }
// Error: last expression of block is println(...), returns Void, but signature expects Int
// Correct: add: (a: Int, b: Int) -> Int = a + b
// Or: add: (a: Int, b: Int) -> Int = { a + b }

# Error 2: using undeclared type parameter
identity: (x: T) -> T = x
// Error: T undeclared; need explicit generic parameter (RFC-010)
// Correct: identity: (T: Type) -> ((x: T) -> T) = x

# Correct: HM algorithm infers return type
double = (x: Int) => x + x

# Full form (progressive abbreviation)
double: (x: Int) -> Int = (x) => x + x                # full
double: (x: Int) -> Int = x + x                       # omit lambda head
double = (x: Int) => x + x                            # omit return type (HM infers return)
# double = (x) => x + x                               # ❌ parameter types cannot be omitted on both sides
```

## Trade-offs

### Advantages

- **Syntax unification**: `name: Signature = LambdaBody` model covers all scenarios
- **Flexible abbreviation**: Any part can be omitted when HM can infer it
- **Type safety**: HM algorithm guarantees type safety, avoiding implicit type conversions
- **Recursion support**: HM algorithm and recursive constraints automatically infer types
- **Zero burden**: Smooth transition from full to minimal form

### Disadvantages

- **Migration cost**: Old code needs migration tool conversion
- **Learning cost**: Need to understand the "full form + arbitrary abbreviation" model

## Alternative Solutions

| Solution | Description | Why not chosen |
|----------|-------------|----------------|
| HM algorithm type inference | Use Hindley-Milner algorithm to infer types | ✅ **Adopted**, modern functional language standard |
| Explicit type declaration | All types must be explicitly written | Violates simplified syntax principle, increases boilerplate |
| Keep old syntax | Support both old and new syntax | Syntax fragmentation, high maintenance cost |
| fn keyword | Introduce fn keyword to distinguish functions and variables | Violates "functions are lambdas" design |

## Implementation Strategy

### Phase Breakdown

1. **Phase 1: Syntax parsing and HM algorithm** (v0.3)
   - Implement new syntax `name = lambda` + HM algorithm type inference
   - Implement default filling for no-param-no-return

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

# Preview migration (don't modify files)
yaoxiang-migrate --old-to-new --dry-run src/main.yaoxiang
```

Migration rules:
```yaoxiang
# Old syntax
add(Int, Int) -> Int = (a, b) => { a + b }
main() -> Int = { println("Hello"); 0 }
main() = { println("Hello") }

# === New syntax: Full form (complete signature + complete lambda head) ===
add: (a: Int, b: Int) -> Int = (a, b) => a + b
main: () -> Void = () => { println("Hello") }

# === Abbreviated: Omit lambda head ===
add: (a: Int, b: Int) -> Int = a + b
main: () -> Void = { println("Hello") }

# === Abbreviated: HM inference ===
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
| Migration omission | Old code fails to compile | Provide migration tool, covering all old syntax patterns |
| Parser errors | Unstable syntax parsing | Sufficient test coverage |

## Open Issues

> The following issues have been resolved in the design, recorded in Appendix A.

- ~~Q1: Should we keep the `main() = body` extreme shorthand?~~ → Resolved: kept as `main = { ... }`
- ~~Q2: Should we keep the `:` after function name?~~ → Resolved: optional to keep; but functions with parameters still need parameter type annotation in signature or lambda head
- ~~Q3: Does HM algorithm support parameter type inference?~~ → Resolved: return type/local variables can be inferred; parameter types for functions with parameters need explicit annotation
- ~~Q4: Should we introduce `fn` keyword?~~ → Resolved: not introduced, functions are lambdas
- ~~Q5: What is the migration strategy for old code?~~ → Resolved: provide `yaoxiang-migrate` tool
- ~~Q6: How to use generic functions?~~ → Resolved: use RFC-010 unified syntax `(T: Type)`

---

## Appendices

### Appendix A: Reference for Function Definition Syntax in Various Languages

| Language | Syntax style | Features |
|----------|-------------|----------|
| Rust | `fn add(a: i32, b: i32) -> i32 { ... }` | Keyword + type annotation |
| Haskell | `add a b = ...` / `add :: Int -> Int -> Int` | Type signature separated |
| OCaml | `let add a b = ...` | Parameter types can be omitted |
| MoonBit | `fn add(a: Int, b: Int): Int { ... }` | Concise type annotation |
| TypeScript | `const add = (a: number, b: number): number => ...` | Lambda style |
| Scala | `def add(a: Int, b: Int): Int = { ... }` | def keyword |
| **YaoXiang** | `name = (a: Int, b: Int) => a + b` | **Function = lambda, HM infers return value** |

### Appendix B: Design Decision Log

| Decision | Resolution | Date | Recorder |
|----------|-----------|------|----------|
| Syntax style | New syntax `name: (params) -> Return = body` + HM inference | 2026-02-03 | @Mo Yu Jiang |
| Parameter position | Parameter names declared in signature, unified with RFC-010 | 2026-02-03 | @Mo Yu Jiang |
| Default filling | No-param functions can omit signature, empty block `{}` infers to `Void` | 2026-02-03 | @Mo Yu Jiang |
| Type inference | HM algorithm automatically infers, explicit when inference impossible | 2026-01-06 | @Mo Yu Jiang |
| Old syntax | Deprecated, migration tool provided | 2026-01-06 | @Mo Yu Jiang |
| fn keyword | Not introduced | 2026-01-06 | @Mo Yu Jiang |
| Recursive declaration | HM algorithm and recursive constraints automatically infer | 2026-01-06 | @Mo Yu Jiang |

### Appendix C: Glossary

| Term | Definition |
|------|------------|
| HM algorithm | Hindley-Milner type inference algorithm, automatically infers types of functions and variables |
| Generics | Using type parameters `(T: Type)` to constrain polymorphic functions, e.g., `identity: (T: Type) -> ((x: T) -> T) = x` (RFC-010) |
| Default type filling | No-param-no-return functions omit `-> Void`, compiler fills automatically |
| Syntax sugar | Syntactic simplifications that make code more readable |
| Normalization | Converting syntactic forms to unified internal representation |
| Functions are lambdas | Functions are essentially lambda variables, types automatically inferred through HM algorithm |

---

## References

- [MoonBit Language Design](https://moonbitlang.com/)
- [Rust Function Syntax](https://doc.rust-lang.org/book/ch03-03-how-functions-work.html)
- [Haskell Type System](https://www.haskell.org/tutorial/patterns.html)
- [OCaml Type Inference](https://v2.ocaml.org/manual/)
```