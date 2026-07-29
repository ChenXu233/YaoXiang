---
title: 'RFC 012: F-String Template Strings'
status: 'Accepted'
author: 'Chen Xu'
created: '2025-01-27'
updated: '2026-07-05'
issue: '#124'
---

# RFC 012: F-String Template Strings

## Summary

Add f-string template string feature to the YaoXiang language, supporting variable interpolation,
expression evaluation, and formatted output. F-strings use Python-style syntax (`f"..."` prefix),
embedding expressions within strings via `{expression}` syntax, and are transformed into efficient
string operations at compile time.

> **Note**: F-string syntax and behavior are consistent with Python. For detailed specifications,
> refer to
> [Python Official Documentation](https://docs.python.org/3/tutorial/inputoutput.html#formatted-string-literals).

## Motivation

### Why is this feature needed?

Current string concatenation in YaoXiang is cumbersome:

```yaoxiang
# Current: using + concatenation
name = "Alice"
age = 30
message = "Hello ".concat(name).concat(", age: ").concat(age.to_string())
print(message)

# Or using the format function
message2 = format("Hello {}, age: {}", name, age)
```

### Current Problems

1. **Poor readability**: String concatenation and formatting require multiple calls, resulting in
   verbose code
2. **Error-prone**: Manual type conversion，容易遗漏 `.to_string()`
3. **Performance concerns**: Multiple string concatenations may impact performance
4. **Limited expressiveness**: Cannot intuitively embed complex expressions within strings

## Proposal

### Core Design

Introduce f-string as a new string literal prefix, supporting:

- **Variable interpolation**: `f"Hello {name}"`
- **Expression evaluation**: `f"Sum: {x + y}"`
- **Format specifiers**: `f"Pi: {pi:.2f}"`
- **Type safety**: Compile-time type checking of expressions

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

| Before                       | After               |
| ---------------------------- | ------------------- |
| `"Hello ".concat(name)`      | `f"Hello {name}"`   |
| `format("Value: {}", value)` | `f"Value: {value}"` |
| `format("Pi: {:.2f}", pi)`   | `f"Pi: {pi:.2f}"`   |

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

The compiler recognizes `f`-prefixed string literals during lexical analysis, parsing expressions
within curly braces and optional format specifiers.

### Transformation Strategy

F-strings are transformed into efficient string operations at compile time:

**Simple interpolation**:

```yaoxiang
f"Hello {name}"
```

Transforms to:

```yaoxiang
"Hello ".concat(name.to_string())
```

**Expression interpolation**:

```yaoxiang
f"Sum: {x + y}"
```

Transforms to:

```yaoxiang
"Sum: ".concat((x + y).to_string())
```

**Format specifiers**:

```yaoxiang
f"Pi: {pi:.2f}"
```

Transforms to:

```yaoxiang
format("Pi: {:.2f}", pi)
```

**Multiple interpolations**:

```yaoxiang
f"Hello {name}, you are {age} years old"
```

Transforms to:

```yaoxiang
"Hello ".concat(name.to_string()).concat(", you are ").concat(age.to_string()).concat(" years old")
```

### Type System Impact

- Interpolation expressions must implement the `Stringable` interface (automatically implemented for
  primitive types and strings)
- Format specifiers require types to support corresponding formatting
- Compiler checks expression type and format specification matching

### Compiler Changes

| Component | Changes                                                              |
| --------- | -------------------------------------------------------------------- |
| lexer     | Recognize f prefix, parse string interpolation syntax                |
| parser    | Add FStringLiteral syntax node                                       |
| typecheck | Check interpolation expression types, validate format specifications |
| codegen   | Generate string concatenation or format call code                    |

### Backward Compatibility

- ✅ Fully backward compatible
- Existing string literals `"..."` remain unchanged
- F-string is new syntax and does not affect existing code

## Trade-offs

### Advantages

1. **Concise syntax**: Reduces boilerplate code, improves readability
2. **Type safety**: Compile-time checking reduces runtime errors
3. **Performance optimization**: Compiler can optimize string concatenation
4. **Strong expressiveness**: Supports arbitrary expressions and formatting
5. **Low learning curve**: Consistent with Python ecosystem

### Disadvantages

1. **Compiler complexity**: Requires new syntax analysis and transformation logic
2. **Syntax ambiguity**: Needs to distinguish from existing string syntax
3. **Debugging challenges**: Transformed code differs from source code structure

## Alternative Solutions

| Solution                       | Why not chosen                                |
| ------------------------------ | --------------------------------------------- |
| Variable interpolation only    | Cannot meet complex formatting needs          |
| Functional style `format(...)` | Not concise enough                            |
| Defer to v2.0                  | Users have clear needs for string convenience |
| Use backticks or other prefix  | Inconsistent with Python ecosystem            |

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
   - Additional formatting options

### Dependencies

- No external dependencies
- Requires basic type system support
- Requires string library basic functionality

### Risks

1. **Performance risk**: Multiple interpolations may create excessive string objects
   - **Mitigation**: Compiler optimization for merging adjacent string constants
2. **Type checking complexity**: Format specifier type checking
   - **Mitigation**: Reference Python implementation, use simple and direct checking
3. **Syntax ambiguity**: Nested use of `{` and `}`
   - **Mitigation**: Clear syntax rules, limit nesting

## Open Questions

- [x] Should escaped braces be supported? Consistent with Python: use double braces for single
      braces, e.g. <code v-pre>{{</code> represents <code v-pre>{</code>, <code v-pre>}}</code>
      represents <code v-pre>}</code>
- [x] Should custom formatting functions be supported? Consistent with Python: support custom
      formatting behavior for types via `__format__` method
- [x] Complete specification for format specifiers? Consistent with Python, see BNF above
- [x] Specific strategies for performance optimization? Consistent with Python: runtime
      concatenation, no special optimization needed
- [x] Best practices for error diagnostics? Consistent with Python: display original f-string
      content and position when reporting errors

## Appendices

### Appendix A: Format Specifier Reference

| Type        | Specifier | Example         | Output         |
| ----------- | --------- | --------------- | -------------- |
| Integer     | `d`       | `f"{42:d}"`     | "42"           |
| Float       | `f`       | `f"{3.14:.2f}"` | "3.14"         |
| Scientific  | `e`       | `f"{1000:e}"`   | "1.000000e+03" |
| String      | `s`       | `f"{name:s}"`   | "Alice"        |
| Hexadecimal | `x`       | `f"{255:x}"`    | "ff"           |

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

RFC has the following status transitions:

```
┌─────────────┐
│   Draft     │  ← Author creates
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Under Review │  ← Community discussion
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
│   accepted/ │    │     rfc/    │
│(official design)| (kept in place)│
└─────────────┘    └─────────────┘
```
