---
title: 'RFC-013: Error Code Specification'
---

# RFC 013: Error Code Specification

> **Status**: Accepted
> **Author**: ChenXu
> **Created Date**: 2026-02-02
> **Last Updated**: 2026-02-12

## Summary

This RFC proposes the error code classification specification for YaoXiang compiler, using a single-level numbering system similar to Rust, with JSON resource files for multilingual support, providing error explanation functionality through `yaoxiang explain` command.

## Motivation

### Why standardized error codes are needed?

1. **User Experience**: Users can quickly determine error type and severity by seeing error code
2. **Documentation Organization**: Categorized grouping makes writing and maintaining error reference documentation easier
3. **Tool Integration**: IDE/LSP can provide quick-fix suggestions and documentation links based on error codes
4. **Internationalization**: Error messages separated from codes, facilitating multi-language translation

### Design Goals

- **Concise**: Single-level numbering, no complex classification rules for users to memorize
- **Friendly**: Similar to Rust error message format, with help information and examples
- **Extensible**: Resource file-driven, easy to add new errors and new languages
- **Tool-Friendly**: explain command + JSON output, supports IDE/LSP integration

---

## Proposal

### Core Design: Single-Level Numbering System

Using four-digit numbering, grouped by compilation phase:

```
Exxxx
││││
│││└── Sequence number (000-999)
││└─── Compilation phase (0-9)
└──── Fixed prefix 'E'
```

### Phase Division

| Phase | Range | Description |
|-------|-------|-------------|
| **0** | E0xxx | Lexical and Syntax Analysis |
| **1** | E1xxx | Type Checking |
| **2** | E2xxx | Semantic Analysis |
| **3** | E3xxx | Code Generation |
| **4** | E4xxx | Generics and Traits |
| **5** | E5xxx | Modules and Imports |
| **6** | E6xxx | Runtime Errors |
| **7** | E7xxx | I/O and System Errors |
| **8** | E8xxx | Internal Compiler Errors |
| **9** | E9xxx | Reserved/Experimental |

### Error Category Enumeration

```rust
/// Error Categories
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    Lexer,      // E0xxx: Lexical and syntax analysis
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

### Error Code Structure

```typescript
interface ErrorCode {
    code: string;           // E1001
    category: ErrorCategory;
    message: string;        // Error message (i18n key)
    explanation: string;     // Detailed explanation
    example?: string;       // Code example
    hint?: string;          // Suggestion for fixing
    see_also?: string[];    // Related error codes
}
```

### Example Error Codes

| Code | Category | Message |
|------|----------|---------|
| E1001 | Lexer | Invalid character in source file |
| E1002 | Lexer | Unterminated string literal |
| E1101 | Parser | Expected expression |
| E1102 | Parser | Expected ')' after expression |
| E1201 | TypeCheck | Type mismatch |
| E1202 | TypeCheck | Cannot find value of type |
| E5101 | Module | Module not found |
| E5102 | Module | Circular module dependency |

---

## Implementation

### Error Code Registry

```rust
pub struct ErrorRegistry {
    codes: HashMap<String, ErrorCode>,
}

impl ErrorRegistry {
    pub fn register(&mut self, code: ErrorCode) {
        self.codes.insert(code.code.clone(), code);
    }

    pub fn lookup(&self, code: &str) -> Option<&ErrorCode> {
        self.codes.get(code)
    }
}
```

### JSON Resource File Format

```json
{
  "errors": {
    "E1001": {
      "message": "Invalid character '{char}' in source file",
      "explanation": "The compiler encountered a character that is not valid in the current context.",
      "example": "# This will cause E1001\n$var = 42  # $ is not a valid identifier start",
      "hint": "Remove the invalid character or use a valid identifier name.",
      "zh": {
        "message": "源文件中存在无效字符 '{char}'",
        "explanation": "编译器在当前上下文中遇到了无效字符。",
        "example": "# 这将导致 E1001\n$var = 42  # $ 不是有效的标识符开头",
        "hint": "移除无效字符或使用有效的标识符名称。"
      }
    }
  }
}
```

### CLI Command

```bash
# Explain error code
yaoxiang explain E1001

# Output:
# E1001: Invalid character
#
# Explanation: The compiler encountered a character that is not valid...
#
# Example:
# $var = 42  # Error: $ is not a valid identifier start
#
# Hint: Remove the invalid character...
```

---

## Appendix A: Error Code List

### E0xxx - Lexer/Parser

| Code | Message |
|------|---------|
| E1001 | Invalid character |
| E1002 | Unterminated string |
| E1003 | Invalid escape sequence |
| E1101 | Expected expression |
| E1102 | Expected ')' |
| E1103 | Expected ':' after field name |
| E1104 | Unexpected token |

### E1xxx - Type Checking

| Code | Message |
|------|---------|
| E1201 | Type mismatch |
| E1202 | Cannot find value of type |
| E1203 | Generic parameter not found |
| E1204 | Trait not implemented |
| E1205 | Type does not support operation |

### E5xxx - Modules

| Code | Message |
|------|---------|
| E5101 | Module not found |
| E5102 | Circular dependency |
| E5103 | Duplicate export |
| E5104 | Private item referenced |

---

## Appendix B: Glossary

| Term | Definition |
|------|------------|
| Error Code | Unique identifier for compiler error |
| Error Category | Grouping of related errors |
| Error Message | Brief description of error |
| Error Explanation | Detailed explanation and suggestions |
| i18n | Internationalization |
