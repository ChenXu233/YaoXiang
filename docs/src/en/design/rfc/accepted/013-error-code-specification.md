---
title: 'RFC 013: Error Code Specification'
status: 'Accepted'
author: 'Chenxu'
created: '2026-02-02'
updated: '2026-02-12'
issue: '#125'
issues_impl:
  - '#125'
pr_impl:
  - '#7'
  - '#9'
  - '#29'
  - '#66'
---

# RFC 013: Error Code Specification

## Summary

This RFC proposes a classification specification for YaoXiang compiler error codes. It adopts a
single-level numbering system similar to Rust, combined with JSON resource files to support
multilingual support, and provides error explanation functionality through the `yaoxiang explain`
command.

## Motivation

### Why do we need standardized error codes?

1. **User Experience**: Users can quickly determine the error type and severity when they see an
   error code
2. **Documentation Organization**: Grouping by category makes it easier to write and maintain error
   reference documentation
3. **Tool Integration**: IDE/LSP can provide quick-fix suggestions and documentation links based on
   error codes
4. **Internationalization Support**: Separation of error messages from codes facilitates
   multilingual translation

### Design Goals

- **Concise**: Single-level numbering, no need for users to memorize complex classification rules
- **Friendly**: Rust-like error message format with help information and examples
- **Extensible**: Resource-file-driven, easy to add new errors and new languages
- **Tool-friendly**: explain command + JSON output, supports IDE/LSP integration

---

## Proposal

### Core Design: Single-Level Numbering System

Adopt a four-digit numbering, grouped by compilation phase:

```
Exxxx
││││
│││└── Sequence number (000-999)
││└─── Compilation phase (0-9)
└───── Fixed prefix 'E'
```

### Phase Division

| Phase | Range | Description               |
| ----- | ----- | ------------------------- |
| **0** | E0xxx | Lexical & Syntax Analysis |
| **1** | E1xxx | Type Check                |
| **2** | E2xxx | Semantic Analysis         |
| **3** | E3xxx | Code Generation           |
| **4** | E4xxx | Generics & Traits         |
| **5** | E5xxx | Module & Import           |
| **6** | E6xxx | Runtime Error             |
| **7** | E7xxx | I/O & System Error        |
| **8** | E8xxx | Internal Compiler Error   |
| **9** | E9xxx | Reserved / Experimental   |

### Error Category Enum

```rust
/// Error category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    Lexer,      // E0xxx: Lexical and syntax analysis
    Parser,     // E0xxx: Parser errors
    TypeCheck,  // E1xxx: Type check
    Semantic,   // E2xxx: Semantic analysis
    Generic,    // E4xxx: Generics and traits
    Module,     // E5xxx: Module and import
    Runtime,    // E6xxx: Runtime error
    Io,         // E7xxx: I/O and system error
    Internal,   // E8xxx: Internal compiler error
}
```

### Error Code Definition and Generic Builder

**Core Principle**: Error code definitions are separated from display text

- `ErrorCodeDefinition`: Error code metadata (code, category, template), without display text
- `locales/*.json`: Display text for each language (title, message, help, error code as nested
  object)
- `DiagnosticBuilder`: Generic builder, replacing the trait-per-error design

#### Error Code Definition

```rust
// diagnostic/codes/mod.rs

use crate::util::span::Span;
use crate::util::diagnostic::{Diagnostic, Severity};

/// Error code definition (metadata only; display text is in i18n files)
#[derive(Debug, Clone, Copy)]
pub struct ErrorCodeDefinition {
    pub code: &'static str,
    pub category: ErrorCategory,
    pub message_template: &'static str,  // Message template, supports {param} placeholders
}

/// Generic diagnostic builder
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

    /// Set position
    pub fn at(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    /// Build Diagnostic (template rendering is completed at compile-time)
    pub fn build(&self, i18n: &I18nRegistry) -> Diagnostic {
        // Check that all {key} in the template have corresponding parameters
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

#### Shortcut Methods for Each Error Code

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

// Simplified way
return Err(E1001::unknown_variable(&var_name)
    .at(span)
    .build(&i18n_registry));

// Manual way
return Err(ErrorCodeDefinition::find("E1001")
    .builder()
    .param("name", var_name)
    .at(span)
    .build(&i18n_registry));
```

#### Error Code Definition Examples

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
    // ... other error codes
];
```

#### Design Advantages

| Feature                   | Description                                              |
| ------------------------- | -------------------------------------------------------- |
| **Single Builder**        | One `DiagnosticBuilder` for all error codes              |
| **Type-safe**             | Shortcut methods ensure parameter correctness            |
| **Self-documenting**      | `E1001::unknown_variable(name)` is self-evident          |
| **Template Separation**   | Message templates are separated from code, easy for i18n |
| **Zero Runtime Overhead** | Compile-time rendering, no lookup tables in AOT binary   |

---

### Error Macro Simplification

#### error! Macro (Automatically Injects Context)

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

/// Usage: Only pass parameters; span and i18n are automatically injected
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

#### E0xxx: Lexical & Syntax Analysis

| Code  | Error Type                | Description                                                |
| ----- | ------------------------- | ---------------------------------------------------------- |
| E0001 | Invalid character         | Source code contains invalid characters                    |
| E0002 | Invalid number literal    | Number literal format is incorrect                         |
| E0003 | Unterminated string       | Multi-line string missing closing quote                    |
| E0004 | Invalid character literal | Character literal is incorrect                             |
| E0010 | Expected token            | Expected specific token during syntax analysis             |
| E0011 | Unexpected token          | Encountered an unexpected token                            |
| E0012 | Invalid syntax            | Expression/statement syntax error                          |
| E0013 | Mismatched brackets       | Parentheses, square brackets, or curly brackets mismatched |
| E0014 | Missing semicolon         | Statement missing semicolon at end                         |

#### E1xxx: Type Check

| Code  | Error Type               | Description                                             |
| ----- | ------------------------ | ------------------------------------------------------- |
| E1001 | Unknown variable         | Referenced variable is not defined                      |
| E1002 | Type mismatch            | Expected type does not match actual type                |
| E1003 | Unknown type             | Referenced type does not exist                          |
| E1010 | Parameter count mismatch | Function call parameter count does not match definition |
| E1011 | Parameter type mismatch  | Parameter type check failed                             |
| E1012 | Return type mismatch     | Function return type is wrong                           |
| E1013 | Function not found       | Calling an undefined function                           |
| E1020 | Cannot infer type        | Context cannot infer the type                           |
| E1021 | Type inference conflict  | Multiple constraints lead to type contradiction         |
| E1030 | Pattern non-exhaustive   | match expression does not cover all cases               |
| E1031 | Unreachable pattern      | Pattern that can never be matched                       |
| E1040 | Operation not supported  | Type does not support this operation                    |
| E1041 | Index out of bounds      | Array/list index out of range                           |
| E1042 | Field not found          | Accessing a non-existent struct field                   |

#### E2xxx: Semantic Analysis

| Code  | Error Type           | Description                                       |
| ----- | -------------------- | ------------------------------------------------- |
| E2001 | Scope error          | Variable is not in the current scope              |
| E2002 | Duplicate definition | Duplicate definition within the same scope        |
| E2003 | Lifetime error       | Lifetime constraint not satisfied                 |
| E2010 | Immutable assignment | Attempting to modify an immutable variable        |
| E2011 | Uninitialized use    | Using an uninitialized variable                   |
| E2012 | Mutability conflict  | Using a mutable reference in an immutable context |

#### E4xxx: Generics & Traits

| Code  | Error Type                     | Description                                |
| ----- | ------------------------------ | ------------------------------------------ |
| E4001 | Generic parameter mismatch     | Generic parameter count/type mismatch      |
| E4002 | Trait bound violated           | Trait constraint not satisfied             |
| E4003 | Associated type error          | Associated type definition/usage error     |
| E4004 | Duplicate trait implementation | Duplicate implementation of the same trait |
| E4005 | Trait not found                | Cannot find the required trait             |
| E4006 | Sized bound violated           | Sized constraint not satisfied             |

#### E5xxx: Module & Import

| Code  | Error Type          | Description                                        |
| ----- | ------------------- | -------------------------------------------------- |
| E5001 | Module not found    | Imported module does not exist                     |
| E5002 | Cyclic import       | Cyclic dependency between modules                  |
| E5003 | Symbol not exported | Attempting to access a symbol that is not exported |
| E5004 | Invalid module path | Module path format is incorrect                    |
| E5005 | Private access      | Accessing a private symbol                         |

#### E6xxx: Runtime Error

| Code  | Error Type                  | Description                                  |
| ----- | --------------------------- | -------------------------------------------- |
| E6001 | Division by zero            | Integer division by zero                     |
| E6002 | Assertion failed            | assert! macro failed                         |
| E6003 | Arithmetic overflow         | Arithmetic operation overflow                |
| E6004 | Stack overflow              | Stack space exhausted                        |
| E6005 | Heap allocation failed      | Memory allocation failed                     |
| E6006 | Runtime index out of bounds | Runtime index out of bounds                  |
| E6007 | Type cast failed            | Attempting to cast type to incompatible type |

#### E7xxx: I/O & System Error

| Code  | Error Type        | Description                            |
| ----- | ----------------- | -------------------------------------- |
| E7001 | File not found    | Attempting to read a non-existent file |
| E7002 | Permission denied | Insufficient file permissions          |
| E7003 | I/O error         | Generic I/O error                      |
| E7004 | Network error     | Network operation failed               |

#### E8xxx: Internal Compiler Error

| Code  | Error Type              | Description                    |
| ----- | ----------------------- | ------------------------------ |
| E8001 | Internal compiler error | Internal compiler error        |
| E8002 | Codegen error           | IR/bytecode generation failed  |
| E8003 | Unimplemented feature   | Using an unimplemented feature |
| E8004 | Optimization error      | Compiler optimization error    |

---

### Multilingual Resource Files

#### Resource File Format

```json
// locales/en.json
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
// locales/zh.json
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
// locales/*.json (error code object)

/// i18n display text registry (loaded from JSON at compile-time, zero lookup at runtime)
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

/// Information for a single error code
#[derive(Clone, Copy)]
pub struct ErrorInfo<'a> {
    pub title: &'a str,
    pub message: &'a str,
    pub help: &'a str,
    pub example: Option<&'a str>,
    pub error_output: Option<&'a str>,
}

impl I18nRegistry {
    /// Get registry based on language code
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

    /// Render template (completed at compile-time, zero overhead at runtime)
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

| Placeholder  | Purpose                                    | Example                             |
| ------------ | ------------------------------------------ | ----------------------------------- |
| `{name}`     | Variable/type/trait name, etc. identifiers | `Unknown variable: '{name}'`        |
| `{expected}` | Expected type                              | `Expected type '{expected}'`        |
| `{found}`    | Actual/found type                          | `, found type '{found}'`            |
| `{method}`   | Method name                                | `Method {method} is not a function` |
| `{trait}`    | Trait name                                 | `Cannot find trait: {trait}`        |
| `{path}`     | Module path                                | `Invalid path: {path}`              |
| `{ty}`       | Type expression                            | `Invalid type: {ty}`                |
| `{message}`  | Internal error message                     | `Internal error: {message}`         |

##### Arbitrary Key Support

**params supports any key, not limited to predefined ones**. The caller can pass any `key`:

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

> **Note**: Not all error codes use placeholders. Some error codes (such as E0001) have static
> messages and do not require parameters.

#### Language Priority

```
1. yaoxiang.toml [language.default]
2. ~/.yaoxiang/yaoxiang.toml [language.default]
3. Default value: en
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
1. Read project-level yaoxiang.toml's language.default
2. If not configured, read user-level ~/.yaoxiang/yaoxiang.toml
3. If neither is configured, default to "en"
4. The compiler creates an I18nRegistry based on the selected language (once)
5. All errors use that I18nRegistry to render messages
```

#### The Key to Zero Lookup Overhead

**Rendering happens when compiling the user project, not at runtime.**

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Phase 1: Rust compiles the YaoXiang compiler                           │
│                                                                           │
│  JSON is packaged into the compiler binary                              │
│  Purpose: explain command can directly read i18n data                    │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  Phase 2: YaoXiang compiles the user project (rendering happens here)   │
│                                                                           │
│  When error! macro is called:                                            │
│  1. Read yaoxiang.toml to get language preference                       │
│  2. Load the corresponding language's i18n JSON from the compiler binary│
│  3. Template + params → render() → "Unknown variable: 'x'"              │
│  4. Diagnostic.message = rendered string                                 │
│                                                                           │
│  AOT binary directly stores the final string, no template, no lookup    │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  Phase 3: User program runtime                                          │
│                                                                           │
│  println!("{}", diagnostic.message)                                      │
│  // Directly outputs the final string, no lookups                        │
└─────────────────────────────────────────────────────────────────────────┘
```

| Component                    | Responsibility                      | Rendering Timing            |
| ---------------------------- | ----------------------------------- | --------------------------- |
| `I18nRegistry`               | Provides templates and display text | When compiling user project |
| `DiagnosticBuilder.render()` | Template + params → final string    | When compiling user project |
| `Diagnostic.message`         | Rendered string                     | Stores final result         |
| AOT binary                   | Contains the final string           | Used directly at runtime    |

---

### Error Message Format

Error messages use the following format:

```
error[E####]: <short description>
  --> <file>:<line>:<col>
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

Error severity is managed through the `DiagnosticLevel` enum, decoupled from the error code
numbering:

```rust
pub enum DiagnosticLevel {
    Error,    // Causes compilation failure
    Warning,  // Does not affect compilation, but recommended to fix
    Note,     // Supplementary information
    Help,     // Fix suggestion
}
```

| Level   | Prefix            | Description                 |
| ------- | ----------------- | --------------------------- |
| Error   | `error[E####]:`   | Causes compilation failure  |
| Warning | `warning[E####]:` | Does not affect compilation |
| Note    | `note[E####]:`    | Supplementary information   |
| Help    | `help[E####]:`    | Fix suggestion              |

---

### `yaoxiang explain` Command

#### Command Syntax

```bash
yaoxiang explain <ERROR_CODE> [OPTIONS]
```

#### Options

| Option          | Description                                    |
| --------------- | ---------------------------------------------- |
| `--lang <code>` | Specify language (en-US, zh-CN, default en-US) |
| `--json`        | JSON format output (for IDE/LSP use)           |
| `--json-pretty` | Pretty-formatted JSON output                   |
| `--examples`    | Show only example code                         |
| `--help`        | Show help information                          |

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
  "examples": ["let {name} = value;"],
  "language": "en-US"
}
```

---

### Backward Compatibility

Since this RFC designs the error code system from scratch, there is no backward compatibility issue.

**Future Migration Strategy** (for reference in later versions):

1. Maintain a mapping from old error codes to new error codes
2. Display both old and new codes during the migration period
3. Provide a deprecation timeline

---

## Implementation Strategy

### Phase 1: Error Code Infrastructure

1. Create the `src/diagnostics/` directory structure
2. Implement the `ErrorCode` enum
3. Implement `Diagnostic` and `DiagnosticLevel`
4. Create the resource file directory and sample JSON

### Phase 2: explain Command

1. Implement the `yaoxiang explain` CLI command
2. Support `--lang` and `--json` options
3. Integrate resource file loading
4. Implement parameter template rendering

### Phase 3: Compile-Time Integration

1. Update all error reporting sites to use the new system
2. Implement message template parameter injection
3. Add language priority logic
4. Unit test coverage

### Phase 4: IDE/LSP Integration

1. LSP server integrates explain JSON output
2. Display error code links in IDE
3. Show error explanation on hover
4. Quick-fix suggestions

---

## Appendix

### Complete Error Code Quick Reference

| Range | Category                  |
| ----- | ------------------------- |
| E0xxx | Lexical & Syntax Analysis |
| E1xxx | Type Check                |
| E2xxx | Semantic Analysis         |
| E3xxx | Code Generation           |
| E4xxx | Generics & Traits         |
| E5xxx | Module & Import           |
| E6xxx | Runtime Error             |
| E7xxx | I/O & System Error        |
| E8xxx | Internal Compiler Error   |
| E9xxx | Reserved                  |

### Supported Languages

| Code  | Language     | Status  |
| ----- | ------------ | ------- |
| en-US | English (US) | Default |
| zh-CN | 简体中文     | Planned |

### Error Message Example Comparison

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
