---
title: 'Diagnostic System'
description: 'YaoXiang Diagnostic System Architecture Design'
---

# Diagnostic System

## Error Code System

Error codes are grouped by category:

| Range | Category          | Description                              |
| ----- | ----------------- | ---------------------------------------- |
| E0xxx | Lexical/Syntax    | Lexical analysis and parsing errors      |
| E1xxx | Type checking     | Type mismatch, undefined variables, etc. |
| E2xxx | Semantic analysis | Semantic errors                          |
| E4xxx | Generics/Trait    | Generics and trait system errors         |
| E5xxx | Module/Import     | Module system errors                     |
| E6xxx | Runtime           | Runtime errors                           |
| E7xxx | I/O               | I/O and system errors                    |
| E8xxx | Internal          | Internal compiler errors                 |
| W1xxx | Warning           | Dead code, unused variables, etc.        |

## Diagnostic Data Structure

```rust
pub struct Diagnostic {
    pub code: String,           // 错误码，如 "E1001"
    pub severity: Severity,     // Error / Warning / Info / Hint
    pub message: String,        // 渲染后的消息
    pub span: Option<Span>,     // 源码位置
    pub help: Option<String>,   // 修复建议
    pub related: Vec<Box<Diagnostic>>,  // 关联诊断
}
```

## DiagnosticBuilder Pattern

Obtain a builder via `ErrorCodeDefinition` and set parameters through chained calls:

```rust
let diagnostic = ErrorCodeDefinition::unknown_variable("x")
    .at(span)
    .help("did you mean 'y'?")
    .build();
```

## i18n Support

Titles and help text for all error codes are managed through `I18nRegistry`, supporting
Chinese-English switching. Message templates support `{param}` placeholders.

## Emitter Output

- `TextEmitter`: Text format output, supports color and Unicode symbols
- `JsonEmitter`: JSON format output, used for CI and LSP
