---
title: 'RFC-007: Function Definition Syntax Unification Scheme'
---

# RFC-007: Function Definition Syntax Unification Scheme

> **Status**: Accepted
> **Author**: MoYu酱
> **Created Date**: 2025-01-05
> **Last Updated**: 2026-02-12 (Aligned with RFC-010 unified syntax)

## Summary

This RFC determines the final **function definition syntax** for YaoXiang language. Using unified syntax `name: (params) -> Return = body`, parameter names declared in signature, types can be automatically inferred via HM algorithm. **Complete form** includes parameter types in signature, **any portion** can be simplified when HM can infer it, compatible with RFC-010's `name: type = value` model.

## Motivation

### Why is this feature needed?

1. **Syntax Consistency**: Eliminate historical baggage of old syntax, unified style
2. **Conciseness**: HM algorithm automatically infers types, reducing boilerplate
3. **Type Safety**: HM algorithm guarantees type safety, explicit annotation only when cannot infer
4. **Language Maturity**: HM algorithm is mature solution in modern functional languages

### Unified Syntax Model

**Core Principle**: `name: Signature = LambdaBody`

- **Complete Form**: Signature (parameter names + types + `->` + return type) + Lambda header (parameter names)
- **Shorthand Rules**: Any portion can be omitted when HM can infer it
  - `->` cannot be omitted (function type marker, otherwise parsed as tuple)
  - Parameter types can be omitted → HM infers from usage
  - Lambda header parameter names can be omitted → if already declared in signature
  - Return type must be fully annotated

```yaoxiang
# Complete form (complete signature + complete lambda header)
add: (a: Int, b: Int) -> Int = (a, b) => a + b

# Shorthand: Omit Lambda header (signature already declares parameters)
add: (a: Int, b: Int) -> Int = { return a + b }

# Shorthand: Omit parameter types (HM infers from usage)
add: (a, b) -> Int = (a, b) => { return a + b }

# Minimal form (HM fully infers)
add = (a, b) => { return a + b }              # Inferred as [T: Add](T, T) -> T
```

### Design Goals

```yaoxiang
# === Complete Form ===
add: (a: Int, b: Int) -> Int = (a, b) => { return a + b }

# === Shorthand Forms ===
add: (a: Int, b: Int) -> Int = { return a + b }   # Omit Lambda header
add: (a, b) -> Int = (a, b) => { return a + b }    # Omit parameter types
add = (a, b) => { return a + b }                     # HM fully infers

# === No-Parameter Functions ===
main: () -> Void = () => { return println("Hello") }  # Complete form
main: () -> Void = { return println("Hello") }         # Omit Lambda header
main = { return println("Hello") }                     # Minimal form

# === Generic Functions (using RFC-011 syntax) ===
identity: [T](x: T) -> T = (x) => { return x }         # Complete form
identity: [T](x: T) -> T = { return x }                # Omit Lambda header
identity = [T](x) => { return x }                      # HM infers
```

### Syntax Rules

| Scenario | Syntax | Description |
|----------|--------|-------------|
| Complete form | `name: (a: T, b: T) -> R = (a, b) => body` | All components explicit |
| Omit lambda header | `name: (a: T, b: T) -> R = { body }` | Use block body |
| Omit param types | `name: (a, b) -> R = (a, b) => body` | HM infers types |
| HM fully infers | `name = (a, b) => body` | All implicit |

### Return Type Annotation Rules

| Return Type | Annotation Required |
|-------------|-------------------|
| Non-Void | ✅ Required |
| Void | ✅ Required (for clarity) |

```yaoxiang
# Non-Void return - must annotate return type
add: (a: Int, b: Int) -> Int = { return a + b }

# Void return - return type annotation still required
print: (msg: String) -> Void = { println(msg) }
```

## Detailed Design

### Grammar

```
FunctionDef      ::= Identifier ':' FunctionSignature '=' FunctionBody
FunctionSignature ::= '(' ParameterList? ')' '->' TypeExpr
ParameterList    ::= Parameter (',' Parameter)*
Parameter        ::= Identifier (':' TypeExpr)?
FunctionBody     ::= Lambda | Block
Lambda           ::= '(' ParameterList? ')' '=>' Expr
Block            ::= '{' Stmt* Expr? '}'
```

### Type Inference Rules

| Context | Inferred Type |
|---------|--------------|
| `x = 42` | `Int` |
| `x = "hello"` | `String` |
| `x = true` | `Bool` |
| `x = fn(y)` | Return type of `fn` |

### Compatibility

| Feature | Compatible | Notes |
|---------|-----------|-------|
| RFC-004 method binding | ✅ Yes | Works with unified syntax |
| RFC-011 generics | ✅ Yes | Works with unified syntax |
| Existing code | ⚠️ Migration | Simple migration path |

## Implementation Requirements

### Lexer Changes

| Token | Description |
|-------|-------------|
| `:` | Type annotation separator |
| `->` | Function type arrow |
| `=>` | Lambda arrow |

### Parser Changes

| Rule | Description |
|------|-------------|
| `FunctionDef` | Parse unified syntax |
| `FunctionSignature` | Parse signature |
| `TypeAnnotation` | Parse type annotations |

### Type Checker Changes

| Check | Description |
|-------|-------------|
| `HM inference` | Hindley-Milner type inference |
| `Signature parsing` | Parse function signatures |
| `Return type check` | Verify return type compatibility |

## Migration Strategy

### Automated Migration

```python
# migrate.py - Automated migration script
import re

patterns = [
    # Old: fn name(params) -> type = body
    (r'fn (\w+)\(([^)]*)\) -> (\w+) = (.+)',
     r'\1: (\2) -> \3 = \4'),
    # Add more patterns as needed
]

def migrate(code):
    for old, new in patterns:
        code = re.sub(old, new, code)
    return code
```

### Manual Migration Cases

| Old Syntax | New Syntax | Migration |
|-----------|------------|-----------|
| `fn add(a: Int, b: Int) -> Int { a + b }` | `add: (a: Int, b: Int) -> Int = { a + b }` | Automated |
| `fn identity[T](x: T) -> T { x }` | `identity: [T](x: T) -> T = { x }` | Automated |

## Trade-offs

### Advantages

- **Unified Syntax**: Single syntax for all functions
- **Type Safety**: HM inference with compile-time guarantees
- **Conciseness**: No unnecessary annotations
- **Readability**: Clear separation of signature and body

### Disadvantages

- **Learning Curve**: Understanding HM inference
- **Migration Cost**: Converting existing code

---

## Appendix A: Design Decision Records

| Decision | Decision | Date | Recorder |
|----------|----------|------|----------|
| Syntax model | `name: Signature = Body` | 2025-01-05 | MoYu酱 |
| HM inference | Full inference support | 2025-01-05 | MoYu酱 |
| Return type | Always required | 2025-01-05 | MoYu酱 |

## Appendix B: Glossary

| Term | Definition |
|------|------------|
| HM Algorithm | Hindley-Milner type inference algorithm |
| Type Inference | Automatically determining types from context |
| Function Signature | Type signature including parameter and return types |
| Lambda | Anonymous function expression |
