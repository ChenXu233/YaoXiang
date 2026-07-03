---
title: "RFC Example: Enhanced Pattern Matching Syntax"
---

# RFC Example: Enhanced Pattern Matching Syntax

> **Note**: This is an example RFC template, demonstrating how to write a complete RFC proposal.
> Please refer to this template when writing your own RFC.
>
> **Status**: Example (for reference only)

> **Author**: Chenxu (Example Author)
> **Created**: 2025-01-05
> **Last Updated**: 2026-02-12

## Summary

Add more powerful pattern matching capabilities to YaoXiang, including nested patterns, guard expressions, and `let` pattern bindings.

## Motivation

### Why is this feature needed?

The current `match` expression has limited functionality and cannot handle the following common scenarios:

```yaoxiang
# Cannot destructure nested structures
Person: Type = { name: String, address: Address }
Address: Type = { city: String, zip: Int }
match person {
    Person(name: "Alice", address: Address(city: "Beijing", _)) => "Alice from Beijing"  # ❌ Not supported
}

# Cannot bind variables in patterns
match result {
    ok(value) => print(value)          # ❌ Requires explicit destructuring
}
```

### Current Problems

1. Nested pattern destructuring is not supported
2. Guard expressions cannot be used in patterns
3. `let` statements do not support pattern matching

## Proposal

### Core Design

Extend the `match` expression syntax to support:

1. **Nested pattern destructuring**: destructuring of structs at any depth
2. **Guard expressions**: adding `if` conditions after a pattern
3. **Pattern variable binding**: binding variables directly from patterns

### Examples

```yaoxiang
# Nested destructuring
Person: Type = { name: String, address: Address }
Address: Type = { city: String, zip: Int }

match person {
    Person(name: "Alice", address: Address(city: "Beijing", _)) => "Alice from Beijing"
    Person(name: n, address: Address(city: c, _)) => n + " from " + c
}

# Guard expressions
match n {
    n if n > 0 && n < 10 => "1-9"
    n if n >= 10 => "10+"
    _ => "unknown"
}

# Pattern binding
match result {
    ok(value) => print(value)          # value is already bound
    err(e) => log_error(e)
}

# Nested + binding
match data {
    User(name: first, profile: Profile(age: a)) if a >= 18 => first + " is adult"
}
```

### `let` Statement Pattern Matching

```yaoxiang
# New syntax
let Point(x: 0, y: _) = point  # Bind only when x == 0
let Ok(value) = result         # Destructuring Result

# Multiple bindings
let (a, b, c) = tuple          # Tuple destructuring
```

## Detailed Design

### Syntax Changes

```
MatchExpr   ::= 'match' Expr '{' MatchArm+ '}'
MatchArm    ::= Pattern ('|' Pattern)* ('if' Expr)? '=>' Expr ','
Pattern     ::= LiteralPattern
              | IdentifierPattern
              | StructPattern
              | TuplePattern
              | OrPattern
              | RestPattern

LiteralPattern ::= '_' | Literal
IdentifierPattern ::= Identifier (':' Pattern)?
StructPattern ::= Identifier '(' FieldPattern (',' FieldPattern)* ','? ')'
FieldPattern  ::= Identifier ':' Pattern | Identifier
TuplePattern  ::= '(' Pattern (',' Pattern)* ','? ')'
OrPattern     ::= Pattern '|' Pattern
RestPattern   ::= '...'
```

### Impact on the Type System

- Type checking for pattern matching needs to be extended
- Pattern variables receive the correct type upon a successful match

### Compiler Changes

| Component | Change |
|------|------|
| lexer | Add pattern-related tokens |
| parser | Add pattern parsing logic |
| typecheck | Pattern type inference and binding |
| codegen | Pattern matching code generation |

### Backward Compatibility

- ✅ Fully backward compatible
- Only new syntax is added; the existing `match` syntax remains unchanged

## Trade-offs

### Advantages

- More expressive syntax, more concise code
- Consistency with pattern matching in mainstream languages (Rust, Scala, Elixir)
- Fewer runtime errors; mismatches are caught early

### Disadvantages

- Increased compiler implementation complexity
- A slightly steeper learning curve

## Alternatives

| Alternative | Why not chosen |
|------|--------------|
| Support only top-level destructuring | Cannot handle common nested scenarios |
| Use a functional style | Mixes awkwardly with imperative code |
| Defer to v2.0 | Users already have strong demand |

## Implementation Strategy

### Dependencies

- No external dependencies
- Requires the basic type system to be completed first

### Risks

- Pattern compilation complexity may lead to performance issues
- Excessively deep nesting may cause stack overflow

## Open Questions

1. [ ] Syntax for cycle patterns (`@` binding)?
2. [ ] Support for compile-time exhaustiveness checking?
3. [ ] Performance optimization strategies?

## References

- [Rust Pattern Matching](https://doc.rust-lang.org/book/ch18-00-patterns.html)
- [Scala Pattern Matching](https://docs.scala-lang.org/tour/pattern-matching.html)
- [Elixir Pattern Matching](https://elixir-lang.org/getting-started/pattern-matching.html)