---
title: 'RFC-012: F-String Template Strings'
---

# RFC 012: F-String Template Strings

> **Status**: Draft
> **Author**: Chen Xu
> **Created Date**: 2025-01-27
> **Last Updated**: 2026-02-12 (Aligned with RFC-010 unified syntax)

## Summary

Add f-string template string feature to YaoXiang language, supporting variable interpolation, expression evaluation, and formatted output. F-strings use Python-style syntax (`f"..."` prefix), embedding expressions through `{expression}` syntax in strings, compiled into efficient string operations at compile time.

## Motivation

### Why this feature is needed?

Current YaoXiang string concatenation methods are cumbersome:

```yaoxiang
# Current: use + concatenation
name = "Alice"
age = 30
message = "Hello ".concat(name).concat(", age: ").concat(age.to_string())
print(message)

# Or use format function
message2 = format("Hello {}, {}", name, age)
```

### Current Problems

1. **Poor readability**: String concatenation and formatting require multiple calls, code is verbose
2. **Error-prone**: Manual type conversion, easy to miss `.to_string()`
3. **Performance consideration**: Multiple string concatenations may affect performance
4. **Insufficient expressiveness**: Cannot intuitively embed complex expressions in strings

## Proposal

### Core Design

Introduce f-string as a new string literal prefix, supporting:
- **Variable interpolation**: `f"Hello {name}"`
- **Expression evaluation**: `f"Sum: {x + y}"`
- **Format specifiers**: `f"Pi: {pi:.2f}"`
- **Type safety**: Compile-time expression type checking

### Examples

```yaoxiang
# Basic interpolation
name = "Alice"
greeting = f"Hello {name}"  # "Hello Alice"

# Expression interpolation
x = 10
y = 20
result = f"Sum: {x + y}"    # "Sum: 30"

# Format specifiers
pi = 3.14159
formatted = f"Pi: {pi:.2f}"  # "Pi: 3.14"

# Complex expressions
items = [1, 2, 3]
s = f"Count: {len(items)}, sum: {sum(items)}"  # "Count: 3, sum: 6"

# Object method calls
user = User("Bob", 25)
bio = f"Name: {user.name}, age: {user.get_age()}"
```

### Syntax Changes

| Before | After |
|--------|-------|
| `"Hello ".concat(name)` | `f"Hello {name}"` |
| `format("Value: {}", value)` | `f"Value: {value}"` |
| `format("Pi: {:.2f}", pi)` | `f"Pi: {pi:.2f}"` |

### Syntax Specification

```
FStringLiteral ::= 'f' '"' FStringContent* '"'
FStringContent ::= FStringChar | EscapeSequence | FStringInterpolation
FStringInterpolation ::= '{' Expression (':' FormatSpec)? '}'
FormatSpec      ::= [precision][type]
```

## Detailed Design

### Syntax Analysis

The compiler recognizes `f` prefix string literals during lexical analysis, parsing expressions and optional format specifiers inside curly braces.

### Conversion Strategy

F-strings are converted to efficient string operations at compile time:

**Simple Interpolation**:
```yaoxiang
f"Hello {name}"
```
Converts to:
```yaoxiang
"Hello ".concat(name.to_string())
```

**Expression Interpolation**:
```yaoxiang
f"Sum: {x + y}"
```
Converts to:
```yaoxiang
"Sum: ".concat((x + y).to_string())
```

**Format Specifiers**:
```yaoxiang
f"Pi: {pi:.2f}"
```
Converts to:
```yaoxiang
format("Pi: {:.2f}", pi)
```

**Multiple Interpolations**:
```yaoxiang
f"Hello {name}, you are {age} years old"
```
Converts to:
```yaoxiang
"Hello ".concat(name.to_string()).concat(", you are ").concat(age.to_string()).concat(" years old")
```

### Type System Impact

- Interpolation expressions must implement `Stringable` interface (automatically implemented for primitive types and strings)
- Format specifiers require types to support corresponding formatting
- Compiler checks expression type and format specifier matching

### Compiler Changes

| Component | Change |
|-----------|--------|
| lexer | Recognize f prefix, parse in-string interpolation syntax |
| parser | New FStringLiteral syntax node |
| typecheck | Check interpolation expression types, validate format rules |
| codegen | Generate string concatenation or format call code |

### Backward Compatibility

- ✅ Fully backward compatible
- Existing string literals `"..."` remain unchanged
- F-string is new syntax, does not affect existing code

## Trade-offs

### Advantages

1. **Concise syntax**: Reduce boilerplate code, improve readability
2. **Type safety**: Compile-time checks, reduce runtime errors
3. **Performance optimization**: Compiler can optimize string concatenation
4. **Strong expressiveness**: Support arbitrary expressions and formatting
5. **Low learning cost**: Consistent with Python ecosystem

### Disadvantages

1. **Compiler complexity**: Need new syntax analysis and conversion logic
2. **Syntax ambiguity**: Need to distinguish from existing string syntax
3. **Debugging challenges**: Compiled code differs from source structure

## Alternative Solutions

| Solution | Why Not Chosen |
|----------|----------------|
| Only support variable interpolation | Cannot meet complex formatting needs |
| Use functional style `format(...)` | Syntax not concise enough |
| Delay to v2.0 | Users have clear needs for string convenience |
| Use backticks or other prefix | Inconsistent with Python ecosystem |

## Implementation Strategy

### Phase Division

1. **Phase 1 (v0.9)**:
   - Basic f-string syntax support
   - Variable and simple expression interpolation
   - Basic type conversion

2. **Phase 2 (v1.0)**:
   - Format specifier support
   - Complex expression interpolation
   - Performance optimization

3. **Phase 3 (v1.1)**:
   - Enhanced debugging information
   - Improved error messages
   - More formatting options

### Dependencies

- No external dependencies
- Needs basic type system support
- Needs string library basic functionality

### Risks

1. **Performance risk**: Multiple interpolations may create too many string objects
   - **Mitigation**: Compiler optimizes adjacent string constant merging
2. **Type checking complexity**: Format specifier type checking
   - **Mitigation**: Reference Python implementation, use simple direct checking
3. **Syntax ambiguity**: Nested use of `{` and `}`
   - **Mitigation**: Clear syntax rules, limit nesting

## Open Questions

- [ ] Whether to support escaped braces? `f"{{ literal }}" → "{"`
- [ ] Whether to support custom formatting functions?
- [ ] Complete specification of format specifiers?
- [ ] Specific strategies for performance optimization?
- [ ] Best practices for error diagnostics?

## Appendix

### Appendix A: Format Specifier Reference

| Type | Specifier | Example | Output |
|------|-----------|---------|--------|
| Integer | `d` | `f"{42:d}"` | "42" |
| Float | `f` | `f"{3.14:.2f}"` | "3.14" |
| Scientific | `e` | `f"{1000:e}"` | "1.000000e+03" |
| String | `s` | `f"{name:s}"` | "Alice" |
| Hexadecimal | `x` | `f"{255:x}"` | "ff" |

### Appendix B: Usage Scenario Examples

```yaoxiang
# Logging
log(level: String, msg: String, count: Int) = () => {
    timestamp = get_timestamp()
    print(f"[{timestamp}] {level}: {msg} (count: {count})")
}

# JSON Building
json = "{\n    \"name\": \"".concat(user.name).concat("\",\n    \"age\": ")
    .concat(user.age.to_string()).concat(",\n    \"email\": \"")
    .concat(user.email).concat("\"\n}")

# SQL Query Building (note SQL injection risks)
query = f"SELECT * FROM users WHERE age > {min_age} AND status = '{status}'"

# Debugging info
debug_info = f"Point({x:.2f}, {y:.2f}) at {timestamp}"

# Conditional formatting
status_msg = if is_active {
    f"User {name} is active"
} else {
    f"User {name} is inactive"
}
```

---

## References

- [Python f-strings](https://docs.python.org/3/tutorial/inputoutput.html#formatted-string-literals)
- [Rust format! macro](https://doc.rust-lang.org/std/macro.format.html)
- [JavaScript template literals](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Template_literals)
- [C# interpolated strings](https://docs.microsoft.com/en-us/dotnet/csharp/language-reference/tokens/interpolated)

---

## Lifecycle and Destination

RFC has the following status transitions:

```
┌─────────────┐
│   Draft     │  ← Author creates
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  In Review  │  ← Community discussion
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  Accepted   │    │  Rejected   │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   accepted/ │    │    rfc/     │
│ (Final Doc) │    │ (Keep site) │
└─────────────┘    └─────────────┘
```
