---
title: RFC Example: Enhanced Pattern Matching Syntax
---

# RFC Example: Enhanced Pattern Matching Syntax

> **Note**: This is an RFC template example demonstrating the complete structure of an RFC proposal.
> Please use this template as a reference when writing your own RFC.
>
> **Status**: Example (for reference only)

> **Author**: Chen Xu (example author)
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

### Current problems

1. Nested pattern destructuring not supported
2. Guard expressions not usable in patterns
3. `let` statements do not support pattern matching

## Proposal

### Core Design

Extend `match` expression syntax to support:

1. **Nested pattern destructuring**: Struct destructuring at arbitrary depth
2. **Guard expressions**: Add `if` conditions after patterns
3. **Pattern variable binding**: Bind variables directly from patterns

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
let Point(x: 0, y: _) = point  # Only binds when x == 0
let Ok(value) = result         # Destructures Result

# Multiple binding
let (a, b, c) = tuple          # Destructures tuple
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

### Type System Impact

- Type checking for pattern matching needs to be extended
- Pattern variables receive correct types upon successful match

### Compiler Changes

| Component | Changes |
|-----------|---------|
| lexer | New tokens for patterns |
| parser | New pattern parsing logic |
| typecheck | Pattern type inference and binding |
| codegen | Pattern matching code generation |

### Backward Compatibility

- ✅ Fully backward compatible
- Only adds new syntax, original `match` syntax unchanged

## Trade-offs

### Advantages

- More expressive syntax, more concise code
- Consistent with pattern matching in mainstream languages (Rust, Scala, Elixir)
- Reduces runtime errors by catching non-matches earlier

### Disadvantages

- Increased compiler implementation complexity
- Slightly steeper learning curve

## Alternative Solutions

| Solution | Why Not Chosen |
|----------|----------------|
| Top-level destructuring only | Cannot handle common nested scenarios |
| Use functional style | Not natural when mixed with imperative code |
| Defer to v2.0 | Users already have strong demand |

## Implementation Strategy

### Phase Breakdown

1. **Phase 1 (v0.6)**: Nested destructuring and guard expressions
2. **Phase 2 (v0.7)**: Pattern variable binding
3. **Phase 3 (v0.8)**: `let` pattern matching

### Dependencies

- No external dependencies
- Requires completion of basic type system first

### Risks

- Pattern compilation complexity may cause performance issues
- Deeply nested patterns may cause stack overflow

## Open Questions

1. [ ] What is the syntax for at-patterns (`@` binding)?
2. [ ] Should compile-time pattern exhaustiveness checking be supported?
3. [ ] What are the performance optimization strategies?

## References

- [Rust Pattern Matching](https://doc.rust-lang.org/book/ch18-00-patterns.html)
- [Scala Pattern Matching](https://docs.scala-lang.org/tour/pattern-matching.html)
- [Elixir Pattern Matching](https://elixir-lang.org/getting-started/pattern-matching.html)