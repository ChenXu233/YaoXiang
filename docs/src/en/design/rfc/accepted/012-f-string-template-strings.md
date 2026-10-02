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

Add f-string template string support to the YaoXiang language, enabling variable interpolation,
expression evaluation, and formatted output. F-strings use Python-style syntax (with the `f"..."`
prefix), embedding expressions in strings via the `{expression}` syntax, and are compiled into
efficient string operations.

> **Note**: The syntax and behavior of f-strings are kept consistent with Python. For the formal
> specification, refer to
> [the official Python documentation](https://docs.python.org/3/tutorial/inputoutput.html#formatted-string-literals).

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

### Current Problems

1. **Poor readability**: String concatenation and formatting require multiple calls, leading to
   verbose code
2. **Error-prone**: Manual type conversion can easily miss `.to_string()`
3. **Performance concerns**: Multiple string concatenations may impact performance
4. **Limited expressiveness**: Complex expressions cannot be embedded intuitively in strings

## Proposal

### Core Design

Introduce f-strings as a new string literal prefix, supporting:

- **Variable interpolation**: `f"Hello {name}"`
- **Expression evaluation**: `f"Sum: {x + y}"`
- **Format specifiers**: `f"Pi: {pi:.2f}"`
- **Type safety**: Compile-time type checking for expressions

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
FStringLiteral       ::= 'f' '"' FStringContent* '"'
                       | 'f' '"""' FStringContent* '"""'
FStringContent       ::= FStringChar | EscapeSequence | BraceEscape | FStringInterpolation
FStringInterpolation ::= '{' Expression (':' FormatSpec)? '}'
BraceEscape          ::= '{{' | '}}'
FormatSpec           ::= [[fill] align] [sign] ['#'] ['0'] [width] ['.' precision] [type]
fill                 ::= <any character except '}'>
align                ::= '<' | '>' | '^'
sign                 ::= '+' | '-' | ' '
width                ::= digit+
precision            ::= digit+
type                 ::= 'b' | 'c' | 'd' | 'e' | 'E' | 'f' | 'F' | 'g' | 'G' | 'n' | 'o' | 's' | 'x' | 'X' | '%'
```

- **Escape protocol**: Backslash escapes (`\n`, `\u{...}`, etc.) are decoded in the lexer; the brace
  escapes `{{` / `}}` are preserved verbatim in the raw content, and the parser restores them to
  literal `{` / `}` when segmenting. The two layers are handled separately to avoid "an escaped `{`
  being treated as the start of an interpolation again".
- **Multi-line**: `f"""..."""` is isomorphic to the regular `"""` multi-line string; newlines are
  content, and the first `"""` terminates the string. Nested `"""` inside an interpolation is not
  supported (consistent with regular multi-line strings — please split the expression if you need
  that).
- **Format specifiers**: See
  [Python's Format Specification Mini-Language](https://docs.python.org/3/library/string.html#format-specification-mini-language)
  for the full specification; the BNF in this RFC is a subset. `n` is a locale-aware numeric
  display, currently locale-neutral (synonymous with `d` / default).

### Capability Support Matrix

| Capability                                   | Status             | Notes                                                                                    |
| -------------------------------------------- | ------------------ | ---------------------------------------------------------------------------------------- |
| Variable / expression interpolation          | ✅ Implemented     | Compile-time lowering to `std.string.format` call or constant folding                    |
| `{{` / `}}` literal braces                   | ✅ Implemented     | #402: lexer preserves raw text, parser restores on segment split                         |
| `f"""` multi-line template                   | ✅ Implemented     | #402: newlines are content, terminated by `"""`                                          |
| Format specifiers (width / precision / type) | ✅ Implemented     | #402: `std.string.format` formats by typed value; invalid specifiers reported at runtime |
| Nested triple-quote inside interpolation     | ❌ Not supported   | Consistent with regular `"""` strings — `"""` always terminates                          |
| Compile-time format spec validation          | ❌ Not implemented | Invalid types are currently reported at runtime; compile-time checking is left for later |

## Detailed Design

### Syntactic Analysis

The compiler recognizes `f`-prefixed string literals during the lexical analysis phase, parsing
expressions and optional format specifiers within the braces.

### Translation Strategy

F-strings are translated at compile time into efficient string operations:

**Simple interpolation**:

```yaoxiang
f"Hello {name}"
```

is translated to:

```yaoxiang
"Hello ".concat(name.to_string())
```

**Expression interpolation**:

```yaoxiang
f"Sum: {x + y}"
```

is translated to:

```yaoxiang
"Sum: ".concat((x + y).to_string())
```

**Format specifier**:

```yaoxiang
f"Pi: {pi:.2f}"
```

is translated to:

```yaoxiang
format("Pi: {:.2f}", pi)
```

**Multiple interpolations**:

```yaoxiang
f"Hello {name}, you are {age} years old"
```

is translated to:

```yaoxiang
"Hello ".concat(name.to_string()).concat(", you are ").concat(age.to_string()).concat(" years old")
```

### Type System Impact

- Interpolated expressions must implement the `Stringable` interface (auto-implemented for primitive
  types and strings)
- Format specifiers require the type to support the corresponding formatting
- The compiler checks type/expression matches against the format rules

### Compiler Changes

| Component | Changes                                                        |
| --------- | -------------------------------------------------------------- |
| lexer     | Recognize the `f` prefix; parse in-string interpolation syntax |
| parser    | Add a new FStringLiteral AST node                              |
| typecheck | Check interpolated expression types; validate format rules     |
| codegen   | Emit string concatenation or formatting call code              |

### Backward Compatibility

- ✅ Fully backward compatible
- Existing string literals `"..."` remain unchanged
- F-strings are new syntax that does not affect existing code

## Trade-offs

### Advantages

1. **Concise syntax**: Reduces boilerplate and improves readability
2. **Type safety**: Compile-time checks reduce runtime errors
3. **Performance optimization**: The compiler can optimize string concatenation
4. **Strong expressiveness**: Supports arbitrary expressions and formatting
5. **Low learning curve**: Consistent with the Python ecosystem

### Disadvantages

1. **Compiler complexity**: Requires new syntax analysis and translation logic
2. **Syntax ambiguity**: Must be distinguished from existing string syntax
3. **Debugging challenges**: The compiled code has a different structure from the source

## Alternatives

| Alternative                          | Why not chosen                                 |
| ------------------------------------ | ---------------------------------------------- |
| Only support variable interpolation  | Cannot meet complex formatting needs           |
| Use a functional style `format(...)` | Not concise enough                             |
| Defer to v2.0                        | Users have a clear need for string convenience |
| Use backticks or another prefix      | Inconsistent with the Python ecosystem         |

## Implementation Strategy

### Phased Plan

1. **Phase 1 (v0.9) — ✅ Delivered**:
   - Basic f-string syntax support
   - Variable and simple expression interpolation
   - Basic type conversion

2. **Phase 2 (v1.0) — ✅ Delivered (#402)**:
   - Format specifier support (width / precision / type, with invalid specifiers reported at
     runtime)
   - Complex expression interpolation
   - `{{` / `}}` escapes and `f"""` multi-line templates

3. **Phase 3 (v1.1)**:
   - Enhanced debug information
   - Improved error messages
   - Compile-time validation of format specs in interpolated expressions

### Dependencies

- No external dependencies
- Requires the basic type system
- Requires foundational string library functionality

### Risks

1. **Performance risk**: Multiple interpolations may create too many string objects
   - **Mitigation**: The compiler optimizes adjacent string literal merging
2. **Type-check complexity**: Type checking for format specifiers
   - **Mitigation**: Reference Python's implementation; use simple, direct checks
3. **Syntax ambiguity**: Nested use of `{` and `}`
   - **Mitigation**: Clearly defined grammar rules; nesting is restricted

## Open Questions

- [x] Are escaped braces supported? Consistent with Python: use double braces for a single brace,
      e.g. <code v-pre>{{</code> represents <code v-pre>{</code>, and <code v-pre>}}</code>
      represents <code v-pre>}</code>
- [x] Are custom format functions supported? Consistent with Python: support customizing the
      formatting behavior of a type via the `__format__` method
- [x] Full specification of format specifiers? Consistent with Python; see BNF above
- [x] Specific strategy for performance optimization? Consistent with Python: runtime concatenation,
      no special optimization needed
- [x] Best practices for error diagnostics? Consistent with Python: show the original f-string
      content and position on error

## Appendix

### Appendix A: Format Specifier Reference

| Type        | Specifier | Example         | Output         |
| ----------- | --------- | --------------- | -------------- |
| Integer     | `d`       | `f"{42:d}"`     | "42"           |
| Float       | `f`       | `f"{3.14:.2f}"` | "3.14"         |
| Scientific  | `e`       | `f"{1000:e}"`   | "1.000000e+03" |
| String      | `s`       | `f"{name:s}"`   | "Alice"        |
| Hexadecimal | `x`       | `f"{255:x}"`    | "ff"           |

### Appendix B: Usage Examples

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

# Debug info
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

RFCs go through the following state transitions:

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
│ (Formal     │    │ (Kept in    │
│  Design)    │    │  place)     │
└─────────────┘    └─────────────┘
```
