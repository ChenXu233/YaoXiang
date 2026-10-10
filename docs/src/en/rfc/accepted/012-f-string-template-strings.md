---
title: 'RFC 012: F-String Template Strings'
status: 'Accepted'
author: 'Chen Xu'
created: '2025-01-27'
updated: '2026-10-02'
issue: '#124'
---

# RFC 012: F-String Template Strings

## Summary

Add f-string template string feature to the YaoXiang language, supporting variable interpolation,
expression evaluation, and formatted output. f-strings use Python-style syntax (the `f"..."` prefix)
and embed expressions in strings via the `{expression}` syntax, which the compiler converts into
efficient string operations.

> **Note**: f-string syntax and behavior are kept consistent with Python; for the full
> specification, refer to the
> [Python official documentation](https://docs.python.org/3/tutorial/inputoutput.html#formatted-string-literals).

## Motivation

### Why is this feature needed?

The current string concatenation style in YaoXiang is cumbersome:

```yaoxiang
# Current state: using + concatenation
name = "Alice"
age = 30
message = "Hello ".concat(name).concat(", age: ").concat(age.to_string())
print(message)

# Or use the format function
message2 = format("Hello {}, age: {}", name, age)
```

### Current Problems

1. **Poor readability**: String concatenation and formatting require many calls, making the code
   verbose
2. **Error-prone**: Manual type conversion makes it easy to miss `.to_string()`
3. **Performance concerns**: Multiple string concatenations may affect performance
4. **Insufficient expressiveness**: Complex expressions cannot be embedded in strings intuitively

## Proposal

### Core Design

Introduce f-string as a new string literal prefix, supporting:

- **Variable interpolation**: `f"Hello {name}"`
- **Expression evaluation**: `f"Sum: {x + y}"`
- **Format specifiers**: `f"Pi: {pi:.2f}"`
- **Type safety**: Compile-time check of expression types

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

| Before                       | After               |
| ---------------------------- | ------------------- |
| `"Hello ".concat(name)`      | `f"Hello {name}"`   |
| `format("Value: {}", value)` | `f"Value: {value}"` |
| `format("Pi: {:.2f}", pi)`   | `f"Pi: {pi:.2f}"`   |

### Syntax Specification

```
FStringLiteral       ::= 'f' '"' FStringContent* '"'
                       | 'f' '"""' FStringContent* '"""'
FStringContent       ::= FStringChar | EscapeSequence | BraceEscape | FStringInterpolation
FStringInterpolation ::= '{' Expression (':' FormatSpec)? '}'
BraceEscape          ::= '{{' | '}}'
FormatSpec           ::= [[fill] align] [sign] ['#'] ['0'] [width] ['.' precision] [type]
fill                 ::= <any non '}' character>
align                ::= '<' | '>' | '^'
sign                 ::= '+' | '-' | ' '
width                ::= digit+
precision            ::= digit+
type                 ::= 'b' | 'c' | 'd' | 'e' | 'E' | 'f' | 'F' | 'g' | 'G' | 'n' | 'o' | 's' | 'x' | 'X' | '%'
```

- **Escape protocol**: Backslash escapes (`\n`, `\u{...}`, etc.) are decoded by the lexer; brace
  escapes <code v-pre>{{</code> / <code v-pre>}}</code> are preserved verbatim in the raw content,
  and are restored to literal `{` / `}` when the parser splits segments. The two layers are handled
  separately, so that an "escaped `{`" is not treated again as the start of an interpolation.
- **Multi-line**: `f"""..."""` is isomorphic to ordinary `"""` multi-line strings; newlines are
  content, and the literal ends at the first `"""`. Nested `"""` inside an interpolation is not
  supported (consistent with ordinary multi-line strings — please split the expression if needed).
- **Format specifier**: For the full specification, see
  [Python Format Specification Mini-Language](https://docs.python.org/3/library/string.html#format-specification-mini-language).
  The BNF in this RFC is a subset of it. `n` is locale-aware numeric display; it is currently
  locale-neutral (synonymous with `d` / default).

### Capability Support Matrix

| Capability                                                   | Status             | Description                                                                            |
| ------------------------------------------------------------ | ------------------ | -------------------------------------------------------------------------------------- |
| Variable / expression interpolation                          | ✅ Implemented     | Compile-time conversion to `std.string.format` call or constant folding                |
| <code v-pre>{{</code> / <code v-pre>}}</code> literal braces | ✅ Implemented     | #402: lexer preserves raw content, parser splits and restores                          |
| `f"""` multi-line template                                   | ✅ Implemented     | #402: newlines are content, `"""` ends the literal                                     |
| Format specifier (width / precision / type)                  | ✅ Implemented     | #402: `std.string.format` formats by typed value, illegal specifiers report at runtime |
| Nested same-kind triple quotes inside interpolation          | ❌ Not supported   | Consistent with ordinary `"""` strings — `"""` is always the terminator                |
| Compile-time format check of interpolation expression        | ❌ Not implemented | Illegal `type` currently reports at runtime; compile-time check is left for later      |

## Detailed Design

### Parsing

The compiler recognizes `f`-prefixed string literals at the lexing stage, and parses the expressions
and optional format specifiers inside braces.

### Conversion Strategy

f-strings are converted into efficient string operations at compile time:

**Simple interpolation**:

```yaoxiang
f"Hello {name}"
```

is converted to:

```yaoxiang
"Hello ".concat(name.to_string())
```

**Expression interpolation**:

```yaoxiang
f"Sum: {x + y}"
```

is converted to:

```yaoxiang
"Sum: ".concat((x + y).to_string())
```

**Format specifier**:

```yaoxiang
f"Pi: {pi:.2f}"
```

is converted to:

```yaoxiang
format("Pi: {:.2f}", pi)
```

**Multiple interpolations**:

```yaoxiang
f"Hello {name}, you are {age} years old"
```

is converted to:

```yaoxiang
"Hello ".concat(name.to_string()).concat(", you are ").concat(age.to_string()).concat(" years old")
```

### Type System Impact

- Interpolation expressions must implement the `Stringable` interface (automatically implemented for
  primitive types and `String`)
- Format specifiers require that the type supports the corresponding formatting
- The compiler checks the matching of expression types against format rules

### Compiler Changes

| Component | Changes                                                               |
| --------- | --------------------------------------------------------------------- |
| lexer     | Recognize the `f` prefix and parse in-string interpolation syntax     |
| parser    | Add a new `FStringLiteral` syntax node                                |
| typecheck | Check the type of interpolation expressions and validate format rules |
| codegen   | Emit string concatenation or formatting call code                     |

### Backward Compatibility

- ✅ Fully backward compatible
- Existing string literals `"..."` remain unchanged
- f-string is new syntax and does not affect existing code

## Trade-offs

### Advantages

1. **Concise syntax**: Reduces boilerplate and improves readability
2. **Type safety**: Compile-time checks reduce runtime errors
3. **Performance optimization**: The compiler can optimize string concatenation
4. **Strong expressiveness**: Supports any expression and any formatting
5. **Low learning cost**: Consistent with the Python ecosystem

### Disadvantages

1. **Compiler complexity**: Requires new syntax analysis and conversion logic
2. **Syntax ambiguity**: Must be distinguished from existing string syntax
3. **Debugging difficulty**: The compiled code structure differs from the source

## Alternatives

| Plan                                 | Why not chosen                                |
| ------------------------------------ | --------------------------------------------- |
| Support variable interpolation only  | Cannot satisfy complex formatting needs       |
| Use a functional `format(...)` style | Not concise enough                            |
| Defer to v2.0                        | Users have a clear need for string ergonomics |
| Use backticks or other prefixes      | Inconsistent with the Python ecosystem        |

## Implementation Strategy

### Phases

1. **Phase 1 (v0.9) — ✅ Delivered**:
   - Basic f-string syntax support
   - Variable and simple expression interpolation
   - Basic type conversion

2. **Phase 2 (v1.0) — ✅ Delivered (#402)**:
   - Format specifier support (width / precision / type; illegal specifiers report at runtime)
   - Complex expression interpolation
   - <code v-pre>{{</code> / <code v-pre>}}</code> escaping and `f"""` multi-line templates

3. **Phase 3 (v1.1)**:
   - Enhanced debug information
   - Improved error messages
   - Compile-time format validation for interpolation expressions

### Dependencies

- No external dependencies
- Requires the basic type system to be in place
- Requires basic string library functionality

### Risks

1. **Performance risk**: Multiple interpolations may create too many string objects
   - **Mitigation**: The compiler optimizes merging of adjacent string constants
2. **Type check complexity**: Type checking of format specifiers
   - **Mitigation**: Reference the Python implementation and use simple, direct checks
3. **Syntax ambiguity**: Nested use of `{` and `}`
   - **Mitigation**: Make the grammar rules explicit and limit nesting

## Open Questions

- [x] Support escaped braces? Consistent with Python: use double braces for a single brace, e.g.
      <code v-pre>{{</code> represents <code v-pre>{</code>, and
                                                                                                              <code v-pre>}}</code> represents
      <code v-pre>}</code>
- [x] Support custom format functions? Consistent with Python: support customizing a type's
      formatting behavior through an `__format__` method
- [x] Full specification of format specifiers? Consistent with Python; see the BNF above
- [x] Specific strategy for performance optimization? Consistent with Python: concatenation at
      runtime, no special optimization needed
- [x] Best practices for error diagnostics? Consistent with Python: when reporting an error, show
      the original f-string content and position

## Appendix

### Appendix A: Format Specifier Reference

| Type        | Specifier | Example         | Output         |
| ----------- | --------- | --------------- | -------------- |
| Integer     | `d`       | `f"{42:d}"`     | "42"           |
| Float       | `f`       | `f"{3.14:.2f}"` | "3.14"         |
| Scientific  | `e`       | `f"{1000:e}"`   | "1.000000e+03" |
| String      | `s`       | `f"{name:s}"`   | "Alice"        |
| Hexadecimal | `x`       | `f"{255:x}"`    | "ff"           |

### Appendix B: Usage Scenario Examples

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

## Lifecycle and Destination

RFCs have the following state transitions:

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
│   accepted/ │    │    rfc/     │
│ (official   │    │ (stays in   │
│   design)   │    │   place)    │
└─────────────┘    └─────────────┘
```
