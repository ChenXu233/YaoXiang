---
title: 'RFC-013: Error Code Standards'
---

# RFC 013: Error Code Standards

> **Status**: Accepted
> **Author**: Chen Xu
> **Created**: 2026-02-02
> **Last Updated**: 2026-02-12

## Summary

This RFC proposes an error code classification standard for the YaoXiang compiler, using a single-level numbering system similar to Rust's, with JSON resource files for multi-language support, and an `yaoxiang explain` command for providing error explanations.

## Motivation

### Why do we need standardized error codes?

1. **User Experience**: Users can quickly identify error types and severity levels by looking at error codes
2. **Documentation Organization**: Grouping by category makes it easier to write and maintain error reference documentation
3. **Tool Integration**: IDEs/LSPs can provide quick-fix suggestions and documentation links based on error codes
4. **Internationalization Support**: Separating error messages from codes facilitates multi-language translation

### Design Goals

- **Concise**: Single-level numbering; users don't need to remember complex classification rules
- **Friendly**: Rust-like error message format with help information and examples
- **Extensible**: Resource file-driven; easy to add new errors and new languages
- **Tool-friendly**: explain command + JSON output; supports IDE/LSP integration

---

## Proposal

### Core Design: Single-Level Numbering System

Uses a four-digit numbering scheme, grouped by compilation phase:

```
Exxxx
││││
│││└── Index (000-999)
││└─── Compilation phase (0-9)
└───── Fixed prefix 'E'
```

### Phase Division

| Phase | Range | Description |
|------|-------|-------------|
| **0** | E0xxx | Lexical and syntactic analysis |
| **1** | E1xxx | Type checking |
| **2** | E2xxx | Semantic analysis |
| **3** | E3xxx | Code generation |
| **4** | E4xxx | Generics and traits |
| **5** | E5xxx | Modules and imports |
| **6** | E6xxx | Runtime errors |
| **7** | E7xxx | I/O and system errors |
| **8** | E8xxx | Internal compiler errors |
| **9** | E9xxx | Reserved/experimental |

### Error Category enum

```rust
/// Error category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    Lexer,      // E0xxx: Lexical and syntactic analysis
    Parser,     // E0xxx: Parser errors
    TypeCheck,  // E1xxx: Type checking
    Semantic,   // E2xxx: Semantic analysis
    Generic,    // E4xxx: Generics and traits
    Module,     // E5xxx: Modules and imports
    Runtime,    // E6xxx: Runtime errors
    Io,         // E7xxx: I/O and system errors
    Internal,   // E8xxx: Internal compiler errors
}
```

### Error Code Definition and Universal Builder

**Core Principle**: Separate error code definitions from display text

- `ErrorCodeDefinition`: Error code metadata (code, category, template), without display text
- `i18n/*.json`: Language-specific display text (title, message, help)
- `DiagnosticBuilder`: Universal builder, replacing the trait-per-error design

#### Error Code Definition

```rust
// diagnostic/codes/mod.rs

use crate::util::span::Span;
use crate::util::diagnostic::{Diagnostic, Severity};

/// Error code definition (metadata only; display text in i18n files)
#[derive(Debug, Clone, Copy)]
pub struct ErrorCodeDefinition {
    pub code: &'static str,
    pub category: ErrorCategory,
    pub message_template: &'static str,  // Message template, supports {param} placeholders
}

/// Universal diagnostic builder
pub struct DiagnosticBuilder {
    code: &'static str,
    message_template: &'static str,
    params: Vec<(&'static str, String)>,
    span: Option<Span>,
}

impl DiagnosticBuilder {
    pub fn new(code: &'static str, template: &'static str) -> Self {
        Self {
            code,
            message_template: template,
            params: Vec::new(),
            span: None,
        }
    }

    /// Add template parameter
    pub fn param(mut self, key: &'static str, value: impl Into<String>) -> Self {
        self.params.push((key, value.into()));
        self
    }

    /// Set location
    pub fn at(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    /// Build Diagnostic (template rendering done at compile-time)
    pub fn build(&self, i18n: &I18nRegistry) -> Diagnostic {
        // Verify all {key} placeholders in the template have corresponding parameters
        self.validate_params();

        let message = i18n.render(self.message_template, &self.params);
        let help = self.help(i18n);

        Diagnostic {
            severity: Severity::Error,
            code: self.code.to_string(),
            message,
            help,
            span: self.span,
            related: Vec::new(),
        }
    }
}
```

#### Convenience Methods for Each Error Code

```rust
// diagnostic/codes/e1xxx.rs

impl ErrorCodeDefinition {
    /// E1001 Unknown variable
    pub fn unknown_variable(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E1001").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template)
            .param("name", name)
    }

    /// E1002 Type mismatch
    pub fn type_mismatch(expected: &str, found: &str) -> DiagnosticBuilder {
        let def = Self::find("E1002").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template)
            .param("expected", expected)
            .param("found", found)
    }
}
```

#### Usage Examples

```rust
// checking/mod.rs

use crate::util::diagnostic::codes::{ErrorCodeDefinition, E1001};

// Simplified approach
return Err(E1001::unknown_variable(&var_name)
    .at(span)
    .build(&i18n_registry));

// Manual approach
return Err(ErrorCodeDefinition::find("E1001")
    .builder()
    .param("name", var_name)
    .at(span)
    .build(&i18n_registry));
```

#### Error Code Definition Example

```rust
// diagnostic/codes/e1xxx.rs

pub static E1XXX: &[ErrorCodeDefinition] = &[
    ErrorCodeDefinition {
        code: "E1001",
        category: ErrorCategory::TypeCheck,
        message_template: "Unknown variable: '{name}'",
    },
    ErrorCodeDefinition {
        code: "E1002",
        category: ErrorCategory::TypeCheck,
        message_template: "Expected type '{expected}', found type '{found}'",
    },
    // ... Other error codes
];
```

#### Design Advantages

| Feature | Description |
|---------|-------------|
| **Single Builder** | One `DiagnosticBuilder` works for all error codes |
| **Type Safety** | Convenience methods ensure parameter correctness |
| **Self-Documenting** | `E1001::unknown_variable(name)` is self-explanatory |
| **Template Separation** | Message templates separated from code; easy i18n |
| **Zero Runtime Overhead** | Compile-time rendering; AOT binaries have no lookup tables |

---

### Error Macro Simplification

#### error! Macro (Auto-Injects Context)

```rust
/// Macro that automatically obtains span and i18n config at compile-time
macro_rules! error {
    ($code:ident, $($key:ident = $value:expr),* $(,)?) => {
        $code()
            $(.$key($value))*
            .at(crate::util::span::Span::current())
            .build(crate::util::diagnostic::I18nRegistry::current())
    };
}

/// Usage: Only pass parameters; span and i18n are injected automatically
return Err(error!(E1001, name = var_name));
return Err(error!(E1002, expected = "bool", found = cond_ty));
```

#### Manual Builder Usage

```rust
// When manual control is needed
E1001::unknown_variable(&var_name)
    .at(my_span)           // Custom span
    .build(&custom_i18n)   // Custom i18n
```

---

## Detailed Design

### Error Code List

#### E0xxx: Lexical and Syntactic Analysis

| Code | Error Type | Description |
|------|------------|-------------|
| E0001 | Invalid character | Source code contains illegal character |
| E0002 | Invalid number literal | Number literal format is incorrect |
| E0003 | Unterminated string | Multi-line string missing closing quote |
| E0004 | Invalid character literal | Character literal is incorrect |
| E0010 | Expected token | Parser expected a specific token |
| E0011 | Unexpected token | Encountered unexpected token |
| E0012 | Invalid syntax | Expression/statement syntax error |
| E0013 | Mismatched brackets | Parentheses, square brackets, or curly braces mismatch |
| E0014 | Missing semicolon | Statement missing trailing semicolon |

#### E1xxx: Type Checking

| Code | Error Type | Description |
|------|------------|-------------|
| E1001 | Unknown variable | Referenced variable is not defined |
| E1002 | Type mismatch | Expected type does not match actual type |
| E1003 | Unknown type | Referenced type does not exist |
| E1010 | Parameter count mismatch | Function call parameter count doesn't match definition |
| E1011 | Parameter type mismatch | Parameter type check failed |
| E1012 | Return type mismatch | Function return value type error |
| E1013 | Function not found | Calling an undefined function |
| E1020 | Cannot infer type | Cannot infer type from context |
| E1021 | Type inference conflict | Multiple constraints lead to type contradiction |
| E1030 | Pattern non-exhaustive | match expression doesn't cover all cases |
| E1031 | Unreachable pattern | Pattern that can never match |
| E1040 | Operation not supported | Type doesn't support this operation |
| E1041 | Index out of bounds | Array/list index out of range |
| E1042 | Field not found | Accessing non-existent struct field |

#### E2xxx: Semantic Analysis

| Code | Error Type | Description |
|------|------------|-------------|
| E2001 | Scope error | Variable not in current scope |
| E2002 | Duplicate definition | Duplicate definition in same scope |
| E2003 | Lifetime error | Lifetime constraint not satisfied |
| E2010 | Immutable assignment | Attempt to modify immutable variable |
| E2011 | Uninitialized use | Using uninitialized variable |
| E2012 | Mutability conflict | Using mutable reference in immutable context |

#### E4xxx: Generics and Traits

| Code | Error Type | Description |
|------|------------|-------------|
| E4001 | Generic parameter mismatch | Generic parameter count/type mismatch |
| E4002 | Trait bound violated | Trait constraint not satisfied |
| E4003 | Associated type error | Associated type definition/use error |
| E4004 | Duplicate trait implementation | Implementing same trait twice |
| E4005 | Trait not found | Required trait not found |
| E4006 | Sized bound violated | Sized constraint not satisfied |

#### E5xxx: Modules and Imports

| Code | Error Type | Description |
|------|------------|-------------|
| E5001 | Module not found | Imported module doesn't exist |
| E5002 | Cyclic import | Circular dependency between modules |
| E5003 | Symbol not exported | Attempting to access non-exported symbol |
| E5004 | Invalid module path | Module path format error |
| E5005 | Private access | Accessing private symbol |

#### E6xxx: Runtime Errors

| Code | Error Type | Description |
|------|------------|-------------|
| E6001 | Division by zero | Integer divided by zero |
| E6002 | Assertion failed | assert! macro failed |
| E6003 | Arithmetic overflow | Arithmetic operation overflow |
| E6004 | Stack overflow | Stack space exhausted |
| E6005 | Heap allocation failed | Memory allocation failed |
| E6006 | Runtime index out of bounds | Runtime index out of bounds |
| E6007 | Type cast failed | Attempting to cast type to incompatible type |

#### E7xxx: I/O and System Errors

| Code | Error Type | Description |
|------|------------|-------------|
| E7001 | File not found | Attempting to read non-existent file |
| E7002 | Permission denied | Insufficient file permissions |
| E7003 | I/O error | General I/O error |
| E7004 | Network error | Network operation failed |

#### E8xxx: Internal Compiler Errors

| Code | Error Type | Description |
|------|------------|-------------|
| E8001 | Internal compiler error | Internal compiler error |
| E8002 | Codegen error | IR/bytecode generation failed |
| E8003 | Unimplemented feature | Using unimplemented feature |
| E8004 | Optimization error | Compiler optimization error |

---

### Multi-Language Resource Files

#### Resource File Format

```json
// diagnostic/codes/i18n/en.json
{
  "E1001": {
    "title": "Unknown variable",
    "message": "Referenced variable is not defined",
    "template": "Unknown variable: '{name}'",
    "help": "Check if the variable name is spelled correctly, or define it first",
    "example": "x = 100;",
    "error_output": "error[E1001]: Unknown variable: 'x'\n  --> example.yx:1:1\n   |\n 1 | print(x)\n   | ^ unknown variable 'x'"
  },
  "E1002": {
    "title": "Type mismatch",
    "message": "Expected type does not match actual type",
    "template": "Expected type '{expected}', found type '{found}'",
    "help": "Use the correct type or add a type conversion",
    "example": "x: Int = \"hello\";",
    "error_output": "error[E1002]: Type mismatch\n  --> example.yx:1:12\n   |\n 1 | x: Int = \"hello\";\n   |            ^ expected 'Int', found 'String'"
  }
}
```

```json
// diagnostic/codes/i18n/zh.json
{
  "E1001": {
    "title": "未知变量",
    "message": "引用的变量未定义",
    "template": "未知变量：'{name}'",
    "help": "检查变量名是否拼写正确，或先定义它",
    "example": "x = 100;",
    "error_output": "error[E1001]: 未知变量：'x'\n  --> example.yx:1:1\n   |\n 1 | print(x)\n   | ^ 未知变量 'x'"
  },
  "E1002": {
    "title": "类型不匹配",
    "message": "期望类型与实际类型不匹配",
    "template": "期望类型 '{expected}'，实际类型 '{found}'",
    "help": "使用正确的类型或添加类型转换",
    "example": "x: Int = \"hello\";",
    "error_output": "error[E1002]: 类型不匹配\n  --> example.yx:1:12\n   |\n 1 | x: Int = \"hello\";\n   |            ^ 期望 'Int'，找到 'String'"
  }
}
```

#### I18nRegistry Implementation

```rust
// diagnostic/codes/i18n/mod.rs

/// i18n display text registry (loaded from JSON at compile-time; zero lookup at runtime)
pub struct I18nRegistry {
    /// Titles
    titles: HashMap<&'static str, &'static str>,
    /// Descriptions
    messages: HashMap<&'static str, &'static str>,
    /// Help information
    helps: HashMap<&'static str, &'static str>,
    /// Example code
    examples: HashMap<&'static str, &'static str>,
    /// Error output examples
    error_outputs: HashMap<&'static str, &'static str>,
}

/// Single error code information
#[derive(Clone, Copy)]
pub struct ErrorInfo<'a> {
    pub title: &'a str,
    pub message: &'a str,
    pub help: &'a str,
    pub example: Option<&'a str>,
    pub error_output: Option<&'a str>,
}

impl I18nRegistry {
    /// Get registry by language code
    pub fn new(lang: &str) -> Self {
        match lang {
            "zh" => Self::zh(),
            _ => Self::en(),
        }
    }

    /// Get error information
    pub fn get_info(&self, code: &str) -> Option<ErrorInfo<'_>> {
        Some(ErrorInfo {
            title: self.titles.get(code)?,
            message: self.messages.get(code)?,
            help: self.helps.get(code)?,
            example: self.examples.get(code).copied(),
            error_output: self.error_outputs.get(code).copied(),
        })
    }

    /// Render template (done at compile-time; zero overhead at runtime)
    pub fn render(&self, template: &'static str, params: &[(&str, String)]) -> String {
        let mut result = String::with_capacity(template.len() + 64);
        let mut chars = template.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '{' {
                let mut key = String::new();
                while let Some(&c) = chars.peek() {
                    if c == '}' {
                        chars.next();
                        if let Some((_, value)) = params.iter().find(|(k, _)| k == &key) {
                            result.push_str(value);
                        } else {
                            result.push_str(&format!("{{{}}}", key));
                        }
                        break;
                    }
                    key.push(c);
                    chars.next();
                }
            } else {
                result.push(c);
            }
        }
        result
    }
}
```

#### Template Placeholders

##### Predefined Placeholders (Common)

| Placeholder | Purpose | Example |
|------------|---------|---------|
| `{name}` | Variable name/type name/trait name and other identifiers | `Unknown variable: '{name}'` |
| `{expected}` | Expected type | `Expected type '{expected}'` |
| `{found}` | Actual/found type | `, found type '{found}'` |
| `{method}` | Method name | `Method {method} is not a function` |
| `{trait}` | trait name | `Cannot find trait: {trait}` |
| `{path}` | Module path | `Invalid path: {path}` |
| `{ty}` | Type expression | `Invalid type: {ty}` |
| `{message}` | Internal error message | `Internal error: {message}` |

##### Arbitrary Key Support

**params supports arbitrary keys, not limited to predefined ones**. Callers can pass any `key`:

```rust
// Using arbitrary keys
E1001::unknown_variable(&var_name)
    .param("location", "global scope")
    .param("hint", "try declaring it first")
    .at(span)
    .build(&i18n);

// Template definition
"Unknown variable: '{name}' at {location}. {hint}"
```

> **Note**: Not all error codes use placeholders. Some error codes (like E0001) are static messages that don't need parameters.

#### Language Priority

```
1. yaoxiang.toml [language.default]
2. ~/.yaoxiang/yaoxiang.toml [language.default]
3. Default: en
```

### yaoxiang.toml Configuration

#### Project-Level Configuration

```toml
# yaoxiang.toml
[project]
name = "my-project"
version = "0.1.0"

[language]
# Error message language, options: en, zh, ja, ...
default = "zh"
```

#### User-Level Configuration

```toml
# ~/.yaoxiang/yaoxiang.toml
[language]
default = "zh"
```

#### Compile-Time Language Selection

```
1. Read language.default from project-level yaoxiang.toml
2. If not configured, read from user-level ~/.yaoxiang/yaoxiang.toml
3. If neither is configured, default to "en"
4. Compiler creates I18nRegistry based on selected language (once)
5. All errors use that I18nRegistry for message rendering
```

#### Key to Zero Lookup Overhead

**Rendering happens when compiling the user's project, not at runtime.**

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Stage 1: Rust compiling YaoXiang compiler                              │
│                                                                           │
│  JSON packed into compiler binary                                         │
│  Purpose: explain command can directly read i18n data                    │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  Stage 2: YaoXiang compiling user's project (rendering happens here)     │
│                                                                           │
│  When error! macro is called:                                             │
│  1. Read yaoxiang.toml to get language preference                        │
│  2. Load corresponding language's i18n JSON from compiler binary        │
│  3. Template + params → render() → "Unknown variable: 'x'"              │
│  4. Diagnostic.message = pre-rendered string                             │
│                                                                           │
│  AOT binary stores final strings directly; no templates, no lookups      │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  Stage 3: User program runtime                                            │
│                                                                           │
│  println!("{}", diagnostic.message)                                      │
│  // Direct output of final string; no lookups at all                     │
└─────────────────────────────────────────────────────────────────────────┘
```

| Component | Responsibility | Rendering Timing |
|-----------|---------------|------------------|
| `I18nRegistry` | Provides templates and display text | When compiling user's project |
| `DiagnosticBuilder.render()` | Template + params → final string | When compiling user's project |
| `Diagnostic.message` | Pre-rendered string | Stores final result |
| AOT binary | Contains final strings | Used directly at runtime |

---

### Error Message Format

Error messages use the following format:

```
error[E####]: <short description>
  --> <file>:<line>:<column>
   <line> | <code snippet>
          ^^^<highlight>
```

#### Complete Example

```
error[E1001]: Unknown variable: x
  --> src/main.yx:5:12
   5 |   print(x)
          ^
          help: Did you mean to define it?
```

---

### Severity Levels

Error severity levels are managed through the `DiagnosticLevel` enum, decoupled from error code numbering:

```rust
pub enum DiagnosticLevel {
    Error,    // Causes compilation failure
    Warning,  // Doesn't affect compilation, but fixing is recommended
    Note,     // Supplementary information
    Help,     // Fix suggestion
}
```

| Level | Prefix | Description |
|-------|--------|-------------|
| Error | `error[E####]:` | Causes compilation failure |
| Warning | `warning[E####]:` | Doesn't affect compilation |
| Note | `note[E####]:` | Supplementary information |
| Help | `help[E####]:` | Fix suggestions |

---

### `yaoxiang explain` Command

#### Command Syntax

```bash
yaoxiang explain <ERROR_CODE> [OPTIONS]
```

#### Options

| Option | Description |
|--------|-------------|
| `--lang <code>` | Specify language (en-US, zh-CN, default en-US) |
| `--json` | JSON format output (for IDE/LSP use) |
| `--json-pretty` | Formatted JSON output |
| `--examples` | Show example code only |
| `--help` | Display help information |

#### Usage Examples

```bash
# Default English
$ yaoxiang explain E1001
error[E1001]: Unknown variable: {name}
  --> <file>:<line>:<col>

Help: Did you mean to define it?

Example:
  let {name} = value;

# Chinese output
$ yaoxiang explain E1001 --lang zh
error[E1001]: 未知变量: {name}
  --> <file>:<line>:<col>

帮助: 你是否想要定义它？

示例:
  let {name} = value;

# JSON output (LSP integration)
$ yaoxiang explain E1001 --json
{
  "code": "E1001",
  "message": "Unknown variable: {name}",
  "help": "Did you mean to define it?",
  "examples": ["let {name} = value;"],
  "language": "en-US"
}
```

#### JSON Output Format

```json
{
  "code": "E1001",
  "message": "Unknown variable: {name}",
  "help": "Did you mean to define it?",
  "examples": [
    "let {name} = value;"
  ],
  "language": "en-US"
}
```

---

### Backward Compatibility

Since this RFC designs the error code system from scratch, there are no backward compatibility issues.

**Future Migration Strategy** (for reference in subsequent versions):

1. Maintain old-to-new error code mappings
2. Display both old and new codes during migration period
3. Provide deprecation timeline

---

## Implementation Strategy

### Phase One: Error Code Infrastructure

1. Create `src/diagnostics/` directory structure
2. Implement `ErrorCode` enum
3. Implement `Diagnostic` and `DiagnosticLevel`
4. Create resource file directory and sample JSON

### Phase Two: explain Command

1. Implement `yaoxiang explain` CLI command
2. Support `--lang` and `--json` options
3. Integrate resource file loading
4. Implement parameter template rendering

### Phase Three: Compile-Time Integration

1. Update all error reporting points to use new system
2. Implement message template parameter injection
3. Add language priority logic
4. Unit test coverage

### Phase Four: IDE/LSP Integration

1. LSP server integration with explain JSON output
2. Display error code links in IDE
3. Hover to show error explanations
4. Quick-fix suggestions

---

## Appendix

### Complete Error Code Quick Reference

| Range | Category |
|-------|----------|
| E0xxx | Lexical and syntactic analysis |
| E1xxx | Type checking |
| E2xxx | Semantic analysis |
| E3xxx | Code generation |
| E4xxx | Generics and traits |
| E5xxx | Modules and imports |
| E6xxx | Runtime errors |
| E7xxx | I/O and system errors |
| E8xxx | Internal compiler errors |
| E9xxx | Reserved |

### Supported Languages

| Code | Language | Status |
|------|----------|--------|
| en-US | English (US) | Default |
| zh-CN | Simplified Chinese | Planned |

### Error Message Comparison Examples

```
# English (en-US)
error[E1001]: Unknown variable: x
  --> src/main.yx:5:12
   5 |   print(x)
          ^
          help: Did you mean to define it?

# Chinese (zh-CN)
error[E1001]: 未知变量: x
  --> src/main.yx:5:12
   5 |   print(x)
          ^
          帮助: 你是否想要定义它？
```

## References

- [Rust Compiler Error Index](https://doc.rust-lang.org/error_codes/error-index.html)
- [GCC Error Message Format](https://gcc.gnu.org/onlinedocs/gcc-13.1.0/gcc/Warning-Options.html)
- [Clang Diagnostic Format](https://clang.llvm.org/diagnostics.html)
- [Language Server Protocol](https://microsoft.github.io/language-server-protocol/)