```markdown
---
title: "RFC 012: F-String Template Strings"
status: "Accepted"
author: "Chen Xu"
created: "2025-01-27"
updated: "2026-02-12"
---

# RFC 012: F-String Template Strings

## Summary

Add f-string template string trait to the YaoXiang language, supporting variable interpolation, expression evaluation, and formatted output. F-strings use Python-style syntax (the `f"..."` prefix), embedding expressions within strings using `{expression}` syntax, which are converted to efficient string operations at compile time.

> **Note**: The f-string syntax and behavior are consistent with Python. For specific specifications, refer to the [Python official documentation](https://docs.python.org/3/tutorial/inputoutput.html#formatted-string-literals).

## Motivation

### Why is this trait needed?

The current string concatenation method in YaoXiang is cumbersome:

```yaoxiang
# Current: using + concatenation
name = "Alice"
age = 30
message = "Hello ".concat(name).concat(", age: ").concat(age.to_string())
print(message)

# Or using the format function
message2 = format("Hello {}, age: {}", name, age)
```

### Current problems

1. **Poor readability**: String concatenation and formatting require multiple calls, resulting in verbose code
2. **Error-prone**: Manual type conversion，容易遗漏 `.to_string()` (prone to missing `.to_string()`)
3. **Performance concerns**: Multiple string concatenations may affect performance
4. **Insufficient expressiveness**: Unable to intuitively embed complex expressions in strings

## Proposal

### Core design

Introduce f-strings as a new string literal prefix, supporting:
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

### Syntax changes

| Before | After |
|--------|-------|
| `"Hello ".concat(name)` | `f"Hello {name}"` |
| `format("Value: {}", value)` | `f"Value: {value}"` |
| `format("Pi: {:.2f}", pi)` | `f"Pi: {pi:.2f}"` |

### Grammar specification

```
FStringLiteral ::= 'f' '"' FStringContent* '"'
FStringContent ::= FStringChar | EscapeSequence | FStringInterpolation
FStringInterpolation ::= '{' Expression (':' FormatSpec)? '}'
FormatSpec      ::= [width] ['.' precision] type
width           ::= digit+
precision       ::= digit+
type            ::= 'b' | 'c' | 'd' | 'e' | 'E' | 'f' | 'F' | 'g' | 'G' | 'n' | 'o' | 's' | 'x' | 'X' | '%'
```

## Detailed design

### Syntax analysis

The compiler recognizes `f`-prefixed string literals during lexical analysis, parsing expressions inside curly braces and optional format specifiers.

### Conversion strategy

F-strings are converted to efficient string operations at compile time:

**Simple interpolation**:
```yaoxiang
f"Hello {name}"
```
Converts to:
```yaoxiang
"Hello ".concat(name.to_string())
```

**Expression interpolation**:
```yaoxiang
f"Sum: {x + y}"
```
Converts to:
```yaoxiang
"Sum: ".concat((x + y).to_string())
```

**Format specifiers**:
```yaoxiang
f"Pi: {pi:.2f}"
```
Converts to:
```yaoxiang
format("Pi: {:.2f}", pi)
```

**Multiple interpolations**:
```yaoxiang
f"Hello {name}, you are {age} years old"
```
Converts to:
```yaoxiang
"Hello ".concat(name.to_string()).concat(", you are ").concat(age.to_string()).concat(" years old")
```

### Type system impact

- Interpolation expressions must impl the `Stringable` trait (automatically implemented for primitive types and strings)
- Format specifiers require the type to support corresponding formatting
- The compiler checks the matching between expression types and format rules

### Compiler changes

| Component | Changes |
|-----------|---------|
| lexer | Recognize f prefix, parse string interpolation syntax |
| parser | Add FStringLiteral syntax node |
| typecheck | Check interpolation expression types, validate format rules |
| codegen | Generate string concatenation or format call code |

### Backward compatibility

- ✅ Fully backward compatible
- Existing string literals `"..."` remain unchanged
- F-strings are new syntax that does not affect existing code

## Trade-offs

### Advantages

1. **Concise syntax**: Reduce boilerplate code, improve readability
2. **Type safety**: Compile-time checking reduces runtime errors
3. **Performance optimization**: Compiler can optimize string concatenation
4. **Strong expressiveness**: Support arbitrary expressions and formatting
5. **Low learning curve**: Consistent with Python ecosystem

### Disadvantages

1. **Compiler complexity**: Requires new syntax analysis and conversion logic
2. **Syntax ambiguity**: Needs to be distinguished from existing string syntax
3. **Debugging challenges**: Converted code differs from source code structure

## Alternative solutions

| Solution | Why not chosen |
|----------|---------------|
| Support only variable interpolation | Cannot meet complex formatting needs |
| Use functional style `format(...)` | Syntax is not concise enough |
| Defer to v2.0 | Users have clear needs for string convenience |
| Use backticks or other prefixes | Inconsistent with Python ecosystem |

## Implementation strategy

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
- Requires string library basic functionality

### Risks

1. **Performance risk**: Multiple interpolations may create too many string objects
   - **Mitigation**: Compiler optimization for merging adjacent string constants
2. **Type checking complexity**: Type checking for format specifiers
   - **Mitigation**: Reference Python implementation, use simple and direct checks
3. **Syntax ambiguity**: Nested use of `{` and `}`
   - **Mitigation**: Clear grammar rules, limit nesting

## Open questions

- [x] Should escaped curly braces be supported? Consistent with Python: use double curly braces to represent single curly braces, e.g. <code v-pre>{{</code> represents <code v-pre>{</code>, <code v-pre>}}</code> represents <code v-pre>}</code>
- [x] Should custom formatting functions be supported? Consistent with Python: support custom formatting behavior for types through the `__format__` method
- [x] Complete specification for format specifiers? Consistent with Python, see BNF above
- [x] Specific strategy for performance optimization? Consistent with Python: runtime concatenation, no special optimization needed
- [x] Best practices for error diagnostics? Consistent with Python: show original f-string content and position in error messages

## Appendix

### Appendix A: Format specifier reference

| Type | Specifier | Example | Output |
|------|-----------|---------|--------|
| Integer | `d` | `f"{42:d}"` | "42" |
| Float | `f` | `f"{3.14:.2f}"` | "3.14" |
| Scientific notation | `e` | `f"{1000:e}"` | "1.000000e+03" |
| String | `s` | `f"{name:s}"` | "Alice" |
| Hexadecimal | `x` | `f"{255:x}"` | "ff" |

### Appendix B: Usage scenario examples

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

# Debugging information
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

## Lifecycle and disposition

RFC has the following status transitions:

```
┌─────────────┐
│   Draft     │  ← Author creates
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Under Review│  ← Community discussion
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
│ (official)  │    │ (preserved) │
└─────────────┘    └─────────────┘
```
```