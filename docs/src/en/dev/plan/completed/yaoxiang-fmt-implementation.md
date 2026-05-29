# YaoXiang Code Formatter Implementation Plan

> **Task**: Implement `yaoxiang fmt` code formatter
> **Based on RFC**: RFC-010 Unified Syntax, RFC-017 LSP Support Design
> **Date**: 2026-03-01
> **Status**: All Complete ✅
> **Version**: v0.6.7
> **Completion Date**: 2026-03-01

---

## Overview

This plan decomposes the `yaoxiang fmt` formatter into multiple implementation phases based on issue_13 requirements. Each phase includes detailed goals, acceptance criteria, and test items.

### Architecture Choice

Adopting the **AST + Source Mapping** approach:
1. **Lexer** records comment/whitespace positions → generates Token stream + source map
2. **Parser** generates AST (with Span information)
3. **Formatter** traverses AST + source map according to configuration → outputs formatted code

**Key Advantages**:
- Preserves complete comment information
- Supports complex code structure refactoring
- Deep integration with LSP

---

## Phase 0: Prerequisites (Source Mapping System) ✅ Completed

> **Importance**: This phase is a prerequisite for the formatter and must be completed first
> **Target Version**: v0.8
> **Implementation File**: `src/formatter/source_map.rs`

### 0.1 SourceMap Data Structure

**Implementation Goals**:
- Design and implement `SourceMap` structure to record source code position mappings
- Record content: comment positions, whitespace regions, original Token positions
- Support conversion from character offset to line/column

**Data Structure Design**:
```
SourceMap {
    source: String,                    // Original source code
    comments: Vec<Comment>,            // Comment list
    tokens: Vec<TokenWithSpan>,        // Token + position
    line_offsets: Vec<usize>,          // Line start offsets
    blank_lines: Vec<usize>,           // Blank line number list
}

Comment {
    content: String,
    span: Span,
    style: CommentStyle,  // SingleLine, MultiLine, Doc
}
```

**Acceptance Criteria**:
- [x] SourceMap correctly parses source code
- [x] All comments (single-line, multi-line, doc comments) positions are recorded
- [x] Blank line positions are correctly recorded
- [x] Supports byte offset to line/column conversion

**Test Items**:
- [x] Single-line comment position recording test
- [x] Multi-line comment position recording test
- [x] Nested comment position recording test
- [x] Blank line detection test
- [x] Source map offset conversion test

---

### 0.2 Lexer Enhancement ✅ Completed

**Implementation Goals**:
- Modify Lexer to record comments instead of skipping them during parsing
- Return `SourceMap` along with token stream from `tokenize()`
- Maintain backward compatibility without affecting existing Token stream

**Implementation Notes**: Adopting an independent SourceMap scanning approach without modifying the Lexer itself. `SourceMap::build()` independently scans the source code to extract comment information, completely unaffected by existing Lexer/Parser flow.

**Acceptance Criteria**:
- [x] Existing Token stream output unchanged
- [x] Comments returned as independent Tokens or attached information
- [x] No impact on parsing performance

**Test Items**:
- [x] Regression test: existing code parsing results unchanged
- [x] Comment collection test

---

## Phase 1: Formatter Core (CLI Command) ✅ Completed

> **Target Version**: v0.9
> **Implementation Files**: `src/formatter/` directory

### 1.1 Formatter Basic Structure ✅ Completed

**Implementation Goals**:
- Create `src/formatter/` directory structure
- Implement `Formatter` core structure
- Implement `FormatOptions` configuration

**Directory Structure**:
```
src/formatter/
├── mod.rs                 # Module exports
├── formatter.rs           # Formatter main structure
├── options.rs             # Formatting options
├── context.rs             # Formatting context
├── writers/
│   ├── mod.rs
│   └── buffer.rs          # Formatted output buffer
├── handlers/
│   ├── mod.rs
│   ├── expr.rs            # Expression formatting
│   ├── stmt.rs            # Statement formatting
│   ├── module.rs          # Module formatting
│   └── comment.rs         # Comment formatting
├── rules/
│   ├── mod.rs
│   ├── indent.rs          # Indentation rules
│   ├── spacing.rs         # Spacing rules
│   ├── line_break.rs      # Line break rules
│   └── alignment.rs       # Alignment rules
└── tests/
    └── mod.rs
```

**Acceptance Criteria**:
- [x] Directory structure created
- [x] Formatter core structure compiles
- [x] Basic modules can be invoked

**Test Items**:
- [x] Module compilation test

---

### 1.2 Expression Formatting ✅ Completed

**Implementation Goals**:
- Implement basic expression formatting: literals, variables, binary operations, unary operations
- Implement function call formatting
- Implement parenthesis preservation strategy

**Implementation File**: `src/formatter/handlers/expr.rs`

**Formatting Rules**:
```
Literals:
  - Numbers: preserve original format
  - Strings: follow single_quote configuration
  - Booleans: true/false

Binary Operations:
  - One space before and after operators
  - Maintain operator precedence line breaks

Function Calls:
  - Use comma alignment for multi-line arguments
  - Keep compact for single line
```

**Acceptance Criteria**:
- [x] Literals formatted correctly
- [x] Variable names formatted correctly
- [x] Binary operations formatted correctly
- [x] Function calls formatted correctly
- [x] Parentheses correctly preserved

**Test Items**:
- [x] Literal formatting test (numbers, strings, booleans)
- [x] Binary operation formatting test
- [x] Unary operation formatting test
- [x] Function call formatting test (single-line, multi-line arguments)
- [x] Parenthesis preservation test

---

### 1.3 Statement Formatting ✅ Completed

**Implementation Goals**:
- Implement variable declaration formatting (let, mut)
- Implement function definition formatting
- Implement type definition formatting
- Implement control flow formatting (if, match, while, for)

**Implementation File**: `src/formatter/handlers/stmt.rs`

**Formatting Rules**:
```
Variable Declarations:
  x = 1
  mut y = 2

Function Definitions:
  add(a: i32, b: i32) -> i32 {
      a + b
  }

Control Flow:
  if condition {
      // ...
  } elif condition {
      // ...
  } else {
      // ...
  }

  match expr {
      pattern => value,
      _ => default,
  }
```

**Acceptance Criteria**:
- [x] Variable declarations formatted correctly
- [x] Function definitions formatted correctly
- [x] Type definitions formatted correctly
- [x] if/elif/else formatted correctly
- [x] match formatted correctly
- [x] while/for formatted correctly

**Test Items**:
- [x] Variable declaration formatting test
- [x] Function definition formatting test (single-line, multi-line arguments)
- [x] Type definition formatting test
- [x] if/elif/else formatting test
- [x] match formatting test (multi-branch alignment)
- [x] while/for formatting test

---

### 1.4 Module Formatting ✅ Completed

**Implementation Goals**:
- Implement file-level formatting (Module)
- Implement import statement formatting (use)
- Implement blank line handling between statements
- Implement module-level comment preservation

**Implementation File**: `src/formatter/handlers/module.rs`

**Formatting Rules**:
```
Import Statements:
  use foo
  use foo.bar
  use foo.{ a, b, c }

Blank Line Handling:
  - Preserve at most one blank line
  - Preserve blank lines around comments
  - Blank lines between top-level statements

Module Comments:
  - Preserve file header comments
  - Preserve doc comments
```

**Acceptance Criteria**:
- [x] use statements formatted correctly
- [x] Blank line handling correct
- [x] Module-level comments preserved
- [x] Top-level statement order unchanged

**Test Items**:
- [x] use statement formatting test
- [x] Blank line preservation test
- [x] Multi-statement formatting test
- [x] Module comment preservation test

---

### 1.5 Comment Handling ✅ Completed

**Implementation Goals**:
- Implement single-line comment formatting
- Implement multi-line comment formatting
- Implement doc comment preservation
- Implement comment alignment

**Implementation File**: `src/formatter/handlers/comment.rs`

**Formatting Rules**:
```
Single-line Comments:
  // Preserve original position
  // Follow code when indenting

Multi-line Comments:
  /* Preserve original format */
  /* Multi-line
     comment */

Comment Alignment:
  // Comment
  let x = 1;
```

**Acceptance Criteria**:
- [x] Single-line comment positions correct
- [x] Multi-line comment format preserved
- [x] Comment indentation correct
- [x] Comment relative position to code correct

**Test Items**:
- [x] Single-line comment preservation test
- [x] Multi-line comment preservation test
- [x] End-of-line comment test
- [x] Multi-line comment alignment test
- [x] Comment-to-code spacing test

---

### 1.6 Configuration Integration ✅ Completed

**Implementation Goals**:
- Integrate existing `FmtConfig` configuration
- Support reading configuration from `yaoxiang.toml`
- Support CLI argument override configuration

**Implementation File**: `src/formatter/options.rs`

**Configuration Items**:
```
[fmt]
line_width = 120        # Maximum line width
indent_width = 4        # Indentation width
use_tabs = false       # Use tab
single_quote = false   # String single quotes
```

**Acceptance Criteria**:
- [x] FmtConfig loaded correctly
- [x] CLI arguments override config file
- [x] Default values correct

**Test Items**:
- [x] Config file reading test
- [x] CLI argument test
- [x] Default configuration test

---

### 1.7 CLI Command Integration ✅ Completed

**Implementation Goals**:
- Implement `yaoxiang fmt <file>` command
- Implement `yaoxiang fmt <dir>` command
- Implement `--check` mode
- Implement `--write` in-place mode
- Implement `--stdout` output to stdout

**Implementation File**: `src/main.rs` (Commands::Format)

**Command Design**:
```
yaoxiang fmt [OPTIONS] <PATH>...

Positional Arguments:
  PATH                  # File or directory path

Options:
  --check               # Check if formatting is needed, don't modify files
  --write, -w           # Write in place (default outputs to stdout)
  --stdout              # Output to stdout (default)
  --indent <SIZE>       # Override indentation width
  --line-width <WIDTH>  # Override maximum line width
  --use-tabs            # Use tab indentation
  --single-quote        # Use single quotes

Exit Codes:
  0                     # File is formatted or written successfully
  1                     # Files need formatting in --check mode
  2                     # Error
```

**Acceptance Criteria**:
- [x] Formatting single file correct
- [x] Formatting directory correct (recursive processing of .yx files)
- [x] --check mode correctly detects unformatted files
- [x] --write mode correctly writes in place
- [x] Exit codes correct

**Test Items**:
- [x] Single file formatting test
- [x] Directory formatting test
- [x] --check mode test
- [x] --write mode test
- [x] Exit code test

---

## Phase 2: LSP Integration ✅ Completed

> **Target Version**: v0.9

### 2.1 LSP Formatting Handler ✅ Completed

**Implementation Goals**:
- Implement `textDocument/formatting` handler
- Implement `textDocument/rangeFormatting` handler
- Declare support in `capabilities.rs`

**Implementation Files**:
- `src/lsp/handlers/formatting.rs` — Formatting request handler
- `src/lsp/server.rs` — Request dispatch
- `src/lsp/capabilities.rs` — Capability declarations

**LSP Methods**:
```
textDocument/formatting
  - Input: DocumentFormattingParams
  - Output: Vec<TextEdit>

textDocument/rangeFormatting
  - Input: DocumentRangeFormattingParams
  - Output: Vec<TextEdit>
```

**Acceptance Criteria**:
- [x] Whole file formatting response correct
- [x] Range formatting response correct
- [x] Capabilities correctly declared

**Test Items**:
- [x] LSP whole file formatting test
- [x] LSP range formatting test
- [x] Capabilities declaration test

---

### 2.2 Document Change Trigger Formatting ✅ Completed

**Implementation Goals**:
- Auto-format on document save (optional configuration)
- Implement `DocumentFormattingEdit` coordination

> **Note**: Save-triggered formatting can be implemented through editor-side configuration (e.g., VS Code's `editor.formatOnSave`), no additional LSP server-side handling required.

**Acceptance Criteria**:
- [x] Recommended to use editor configuration (VS Code formatOnSave)
- [x] No additional LSP server-side handling required

---

## Phase 3: Advanced Features ✅ Completed

### 3.1 Smart Line Wrapping

**Implementation Goals**:
- Implement smart line wrapping when exceeding line width
- Maintain operator alignment
- Maintain function argument alignment

**Line Wrapping Strategy**:
```
Function Call Wrapping:
  foo(
      arg1,
      arg2,
  )

Expression Wrapping:
  result = some_function()
      .chain()
      .filter()

Match Branch Alignment:
  match expr {
      Pattern1 => value1,
      Pattern2 => value2,
  }
```

**Acceptance Criteria**:
- [x] Auto-wrap when exceeding line width
- [x] Indentation correct after wrapping
- [x] Alignment preserved

**Test Items**:
- [x] Long expression wrapping test
- [x] Function argument wrapping test
- [x] Chained call wrapping test

---

### 3.2 Import Statement Sorting ✅ Completed

**Implementation Goals**:
- Implement automatic sorting of use statements
- Support grouped sorting (standard library, external crates, project internal)

**Sorting Rules**:
```
1. Standard library (std, core, alloc)
2. External crates (from Cargo.toml)
3. Project internal (relative paths)

use std::collections::HashMap;
use crate::module::foo;
use some_crate::Bar;
```

**Acceptance Criteria**:
- [x] Import statements sorted correctly
- [x] Grouping correct

**Test Items**:
- [x] Import sorting test
- [x] Grouping test

---

## Testing Strategy

### Unit Tests

Independent unit tests for each formatting module:
- Expression formatting tests
- Statement formatting tests
- Configuration options tests

### Integration Tests

- CLI command tests
- LSP protocol tests
- Real editor integration tests (VS Code)

### Snapshot Testing

Using snapshot testing to ensure stable formatting output:
- Collect community standard code styles
- Compare snapshots after each change
- Automatic snapshot update scripts

### Performance Tests

- Large file formatting performance
- Batch file formatting performance
- Memory usage tests

---

## Acceptance Criteria Summary

### Phase 0 ✅

- [x] SourceMap correctly parses source code
- [x] Comment positions recorded
- [x] Blank lines recorded

### Phase 1 ✅

- [x] Expression formatting correct
- [x] Statement formatting correct
- [x] Comment preservation correct
- [x] Configuration loaded correctly
- [x] CLI commands working correctly
- [x] --check mode working correctly

### Phase 2 ✅

- [x] LSP whole file formatting
- [x] LSP range formatting
- [x] Capabilities declared
- [x] Auto-format on save (recommended to use editor configuration)

### Phase 3 ✅

- [x] Smart line wrapping
- [x] Import sorting

---

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Comment loss | Detailed source mapping system records comment positions |
| Semantic changes | AST equivalence test before and after formatting |
| Performance issues | Incremental formatting, caching mechanism |
| Configuration conflicts | Clear configuration priority |

---

## References

- [Rustfmt Design Document](https://rust-lang.github.io/rfcs/rfcs-2437-rustfmt.html)
- [Prettier Architecture](https://prettier.io/docs/en/architecture)
- [Language Server Protocol Specification](https://microsoft.github.io/language-server-protocol/)