---
title: Diagnostic System
description: Architecture design of YaoXiang's diagnostic system
---

# Diagnostic System

## Error Code System

Error codes are grouped by category:

| Range  | Category      | Description                       |
| ------ | ------------- | --------------------------------- |
| E0xxx | Lexical/Syntax | Lexical analysis and syntax errors |
| E1xxx | Type Checking | Type mismatches, undefined variables, etc. |
| E2xxx | Semantic Analysis | Semantic errors                  |
| E4xxx | Generics/Trait | Generics and trait system errors  |
| E5xxx | Module/Import | Module system errors              |
| E6xxx | Runtime      | Runtime errors                    |
| E7xxx | I/O          | I/O and system errors             |
| E8xxx | Internal     | Internal compiler errors          |
| W1xxx | Warning      | Dead code, unused variables, etc. |

## Diagnostic Data Structure

```rust
pub struct Diagnostic {
    pub code: String,           // Error code, e.g., "E1001"
    pub severity: Severity,     // Error / Warning / Info / Hint
    pub message: String,        // Rendered message
    pub span: Option<Span>,     // Source code location
    pub help: Option<String>,   // Fix suggestion
    pub related: Vec<Box<Diagnostic>>,  // Related diagnostics
}
```

## DiagnosticBuilder Pattern

Obtain a builder through `ErrorCodeDefinition`, chain calls to set parameters:

```rust
let diagnostic = ErrorCodeDefinition::unknown_variable("x")
    .at(span)
    .help("did you mean 'y'?")
    .build();
```

## i18n Support

Titles and help text for all error codes are managed through `I18nRegistry`, supporting Chinese and English switching. Message templates support `{param}` placeholders.

## Emitter Output

- `TextEmitter`：Text format output, supports colors and Unicode symbols
- `JsonEmitter`：JSON format output, used for CI and LSP
