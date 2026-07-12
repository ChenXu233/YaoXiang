---
title: "RFC 012: F-String Template Strings"
status: "Accepted"
author: "Chen Xu"
created: "2025-01-27"
updated: "2026-07-05"
issue: "#124"
---

# RFC 012: F-String Template Strings

## Summary

Add f-string template string support to the YaoXiang language, enabling variable interpolation, expression evaluation, and formatted output. F-strings use Python-style syntax (the `f"..."` prefix), embedding expressions in strings via the `{expression}` syntax, and are converted at compile-time to efficient string operations.

> **Note**: The f-string syntax and behavior are kept consistent with Python. For detailed specifications, refer to the [Python official documentation](https://docs.python.org/3/tutorial/inputoutput.html#formatted-string-literals).

## Motivation

### Why is this feature needed?

The current string concatenation approach in YaoXiang is cumbersome:

```yaoxiang
# Current state: using + for concatenation
name = "Alice"
age = 30
message = "Hello ".concat(name).concat(", age: ").concat(age.to_string())
print(message)

# Or using the format function
message2 = format("Hello {}, age: {}", name, age)
```

### Current Issues

1. **Poor readability**: String concatenation and formatting require multiple calls, making code verbose
2. **Error-prone**: Manually handling type conversions makes it easy to miss `.to_string()`
3. **Performance concerns**: Multiple string concatenations may impact performance
4. **Insufficient expressiveness**: Cannot intuitively embed complex expressions in strings

## Proposal

### Core Design

Introduce f-string as a new string literal prefix that supports:
- **Variable interpolation**: `f"Hello {name}"`
- **Expression evaluation**: `f"Sum: {x + y}"`
- **Format specifiers**: `f"Pi: {pi:.2f}"`
- **Type safety**: Compile-time checking of expression types

### Examples

```yaoxiang
# Basic interpolation
name = "Alice"
greeting = f"Hello {name}"  # "Hello Alice"

# Expression interpolation
x = 10
y = 20
result = f"Sum: {x + y}"    # "Sum: 30"

# Format specifier
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
|------|------|
| `"Hello ".concat(name)` | `f"Hello {name}"` |
| `format("Value: {}", value)` | `f"Value: {value}"` |
| `format("Pi: {:.2f}", pi)` | `f"Pi: {pi:.2f}"` |

### Syntax Specification

```
FStringLiteral ::= 'f' '"' FStringContent* '"'
FStringContent ::= FStringChar | EscapeSequence | FStringInterpolation
FStringInterpolation ::= '{' Expression (':' FormatSpec)? '}'
FormatSpec      ::= [width] ['.' precision] type
width           ::= digit+
precision       ::= digit+
type            ::= 'b' | 'c' | 'd' | 'e' | 'E' | 'f' | 'F' | 'g' | 'G' | 'n' | 'o' | 's' | 'x' | 'X' | '%'
```

## Detailed Design

### Syntax Analysis

The compiler identifies the `f`-prefixed string literal during the lexical analysis phase, parsing expressions and optional format specifiers within curly braces.

### Conversion Strategy

F-strings are converted at compile-time into efficient string operations:

**Simple interpolation**:
```yaoxiang
f"Hello {name}"
```
Is converted to:
```yaoxiang
"Hello ".concat(name.to_string())
```

**Expression interpolation**:
```yaoxiang
f"Sum: {x + y}"
```
Is converted to:
```yaoxiang
"Sum: ".concat((x + y).to_string())
```

**Format specifier**:
```yaoxiang
f"Pi: {pi:.2f}"
```
Is converted to:
```yaoxiang
format("Pi: {:.2f}", pi)
```

**Multiple interpolations**:
```yaoxiang
f"Hello {name}, you are {age} years old"
```
Is converted to:
```yaoxiang
"Hello ".concat(name.to_string()).concat(", you are ").concat(age.to_string()).concat(" years old")
```

### Type System Impact

- Interpolated expressions must implement the `Stringable` interface (auto-implemented for primitive types and strings)
- Format specifiers require the type to support the corresponding formatting
- The compiler checks the match between expression types and format rules

### Compiler Changes

| Component | Change |
|------|------|
| lexer | Recognize the `f` prefix and parse string interpolation syntax |
| parser | Add new FStringLiteral syntax node |
| typecheck | Check interpolated expression types and validate format rules |
| codegen | Generate string concatenation or formatting call code |

### Backward Compatibility

- ✅ Fully backward compatible
- Existing string literals `"..."` remain unchanged
- f-string is a new syntax that does not affect existing code

## Trade-offs

### Advantages

1. **Concise syntax**: Reduces boilerplate code and improves readability
2. **Type safety**: Compile-time checks reduce runtime errors
3. **Performance optimization**: The compiler can optimize string concatenation
4. **Strong expressiveness**: Supports arbitrary expressions and formatting
5. **Low learning curve**: Consistent with the Python ecosystem

### Disadvantages

1. **Compiler complexity**: Requires new syntax analysis and conversion logic
2. **Syntax ambiguity**: Needs to be distinguished from existing string syntax
3. **Debugging challenges**: Compiled code structure differs from source code

## Alternatives

| Approach | Why Not Chosen |
|------|--------------|
| Variable interpolation only | Cannot meet complex formatting needs |
| Use a functional style `format(...)` | Syntax is not concise enough |
| Defer to v2.0 | Users have clear demand for string convenience |
| Use backticks or other prefixes | Inconsistent with the Python ecosystem |

## Implementation Strategy

### Phased Rollout

1. **Phase 1 (v0.9)**:
   - Basic f-string syntax support
   - Variable and simple expression interpolation
   - Basic type conversion

2. **Phase 2 (v1.0)**:
   - Format specifier support
   - Complex expression interpolation
   - Performance optimization

3. **Phase 3 (v1.1)**:
   - Debug information enhancement
   - Error message improvements
   - More formatting options

### Dependencies

- No external dependencies
- Requires basic type system support
- Requires basic string library functionality

### Risks

1. **Performance risk**: Multiple interpolations may produce too many string objects
   - **Mitigation**: Compiler optimizes by merging adjacent string constants
2. **Type check complexity**: Type checking for format specifiers
   - **Mitigation**: Reference Python's implementation, using simple and direct checks
3. **Syntax ambiguity**: Nested use of `{` and `}`
   - **Mitigation**: Clearly define syntax rules and limit nesting

## Open Questions

- [x] Is escaped curly brace supported? Consistent with Python: use double curly braces for a single curly brace, e.g. <code v-pre>{{</code> represents <code v-pre>{</code>, and <code v-pre>}}</code> represents <code v-pre>}</code>
- [x] Is custom format function supported? Consistent with Python: support customizing type formatting behavior via the `__format__` method
- [x] Complete specification for format specifiers? Consistent with Python, see the BNF above for details
- [x] Specific strategy for performance optimization? Consistent with Python: runtime concatenation, no special optimization needed
- [x] Best practices for error diagnostics? Consistent with Python: display the original f-string content and position when reporting errors

## Appendix

### Appendix A: Format Specifier Reference

| Type | Specifier | Example | Output |
|------|--------|------|------|
| Integer | `d` | `f"{42:d}"` | "42" |
| Float | `f` | `f"{3.14:.2f}"` | "3.14" |
| Scientific notation | `e` | `f"{1000:e}"` | "1.000000e+03" |
| String | `s` | `f"{name:s}"` | "Alice" |
| Hexadecimal | `x` | `f"{255:x}"` | "ff" |

### Appendix B: Use Case Examples

```yaoxiang
# Logging
log(level: String, msg: String, count: Int) = () => {
    timestamp = get_timestamp()
    print(f"[{timestamp}] {level}: {msg} (count: {count})")
}

# JSON construction
json = "{\n    \"name\": \"".concat(user.name).concat("\",\n    \"age\": ")
    .concat(user.age.to_string()).concat(",\n    \"email\": \"")
    .concat(user.email).concat("\"\n}")

# SQL query construction (note the SQL injection risk)
query = f"SELECT * FROM users WHERE age > {min_age} AND status = '{status}'"

# Debug information
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

An RFC goes through the following status transitions:

```
┌─────────────┐
│   Draft     │  ← Author creates
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Under      │  ← Community discussion
│  Review     │
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
│  (Official │    │  (Keep in   │
│   design)   │    │   place)    │
└─────────────┘    └─────────────┘
```