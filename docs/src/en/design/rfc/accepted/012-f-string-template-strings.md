---
title: RFC-012: F-String Template Strings
---

# RFC 012: F-String Template Strings

> **Status**: Accepted
> **Author**: Chen Xu
> **Created**: 2025-01-27
> **Last Updated**: 2026-02-12

## Abstract

Add f-string template string support to YaoXiang language, including variable interpolation, expression evaluation, and formatted output. F-strings use Python-style syntax (`f"..."` prefix), embedding expressions via `{expression}` syntax within strings, and are compiled into efficient string operations at compile-time.

> **Note**: F-string syntax and behavior are consistent with Python. For detailed specifications, refer to [Python Official Documentation](https://docs.python.org/3/tutorial/inputoutput.html#formatted-string-literals).

## Motivation

### Why is this feature needed?

Current string concatenation in YaoXiang is cumbersome:

```yaoxiang
# Current: using + concatenation
name = "Alice"
age = 30
message = "Hello ".concat(name).concat(", age: ").concat(age.to_string())
print(message)

# Or using format function
message2 = format("Hello {}, age: {}", name, age)
```

### Current Problems

1. **Poor readability**: String concatenation and formatting require multiple calls, resulting in verbose code
2. **Error-prone**: Manual type conversion, easy to miss `.to_string()`
3. **Performance concerns**: Multiple string concatenations may affect performance
4. **Limited expressiveness**: Cannot intuitively embed complex expressions in strings

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
FormatSpec      ::= [width] ['.' precision] type
width           ::= digit+
precision       ::= digit+
type            ::= 'b' | 'c' | 'd' | 'e' | 'E' | 'f' | 'F' | 'g' | 'G' | 'n' | 'o' | 's' | 'x' | 'X' | '%'
```

## Detailed Design

### Syntax Analysis

The compiler recognizes `f` prefixed string literals during the lexical analysis phase, parsing expressions inside curly braces and optional format specifiers.

### Transformation Strategy

F-strings are transformed into efficient string operations at compile-time:

**Simple interpolation**:
```yaoxiang
f"Hello {name}"
```
transforms to:
```yaoxiang
"Hello ".concat(name.to_string())
```

**Expression interpolation**:
```yaoxiang
f"Sum: {x + y}"
```
transforms to:
```yaoxiang
"Sum: ".concat((x + y).to_string())
```

**Format specifiers**:
```yaoxiang
f"Pi: {pi:.2f}"
```
transforms to:
```yaoxiang
format("Pi: {:.2f}", pi)
```

**Multiple interpolations**:
```yaoxiang
f"Hello {name}, you are {age} years old"
```
 transforms to:
```yaoxiang
"Hello ".concat(name.to_string()).concat(", you are ").concat(age.to_string()).concat(" years old")
```

### Type System Impact

- Interpolation expressions must implement the `Stringable` interface (automatically impl'd for primitive types and strings)
- Format specifiers require the type to support corresponding formatting
- Compiler checks expression type and format rule matching

### Compiler Changes

| Component | Changes |
|-----------|---------|
| lexer | Recognize f prefix, parse string interpolation syntax |
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
2. **Type safety**: Compile-time checking, reduce runtime errors
3. **Performance optimization**: Compiler can optimize string concatenation
4. **Strong expressiveness**: Support arbitrary expressions and formatting
5. **Low learning curve**: Consistent with Python ecosystem

### Disadvantages

1. **Compiler complexity**: Requires new syntax analysis and transformation logic
2. **Syntax ambiguity**: Need to distinguish from existing string syntax
3. **Debugging challenges**: Compiled code differs from source code structure

## Alternative Approaches

| Approach | Why Not Chosen |
|----------|----------------|
| Variable interpolation only | Cannot meet complex formatting needs |
| Functional style `format(...)` | Syntax not concise enough |
| Defer to v2.0 | Users have clear needs for string convenience |
| Use backticks or other prefix | Inconsistent with Python ecosystem |

## Implementation Strategy

### Phases

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
- Requires basic type system support
- Requires string library base functionality

### Risks

1. **Performance risk**: Multiple interpolations may create excessive string objects
   - **Mitigation**: Compiler optimization for adjacent string constant merging
2. **Type checking complexity**: Format specifier type checking
   - **Mitigation**: Reference Python implementation, use simple direct checking
3. **Syntax ambiguity**: Nested use of `{` and `}`
   - **Mitigation**: Clear syntax rules, limit nesting

## Open Questions

- [x] Support escaped braces? Consistent with Python: use double braces for single braces, e.g. <code v-pre>{{</code> represents <code v-pre>{</code>, <code v-pre>}}</code> represents <code v-pre>}</code>
- [x] Support custom formatting functions? Consistent with Python: support custom formatting behavior for types via `__format__` method
- [x] Complete format specifier specification? Consistent with Python, see BNF above
- [x] Specific performance optimization strategies? Consistent with Python: runtime concatenation, no special optimization needed
- [x] Best practices for error diagnostics? Consistent with Python: show original f-string content and location in error messages

## Appendices

### Appendix A: Format Specifier Reference

| Type | Specifier | Example | Output |
|------|-----------|---------|--------|
| Integer | `d` | `f"{42:d}"` | "42" |
| Float | `f` | `f"{3.14:.2f}"` | "3.14" |
| Scientific | `e` | `f"{1000:e}"` | "1.000000e+03" |
| String | `s` | `f"{name:s}"` | "Alice" |
| Hexadecimal | `x` | `f"{255:x}"` | "ff" |

### Appendix B: Usage Scenarios

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

# SQL query construction (note SQL injection risk)
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

## Lifecycle and Disposition

RFC has the following state transitions:

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
│   accepted/ │    │     rfc/     │
│ (Official   │    │ (Preserved  │
│  design)    │    │  in place)  │
└─────────────┘    └─────────────┘
```