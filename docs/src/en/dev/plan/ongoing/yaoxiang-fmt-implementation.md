# YaoXiang Code Formatting Tool Implementation Plan

> **Task**: Implement the `yaoxiang fmt` code formatter
> **Based on RFC**: RFC-010 Unified Syntax, RFC-017 LSP Support Design
> **Date**: 2026-03-01
> **Status**: All Completed ✅
> **Version**: v0.6.7
> **Completion Date**: 2026-03-01

---

## Overview

This plan decomposes the `yaoxiang fmt` formatter into multiple implementation phases based on the issue_13 requirements document. Each phase includes detailed objectives, acceptance criteria, and test items.

### Architecture Choice

Adopting the **AST + Source Map** approach:
1. **Lexer** records comment/blank positions → generates Token stream + Source Map
2. **Parser** generates AST (with Span information)
3. **Formatter** traverses AST + Source Map according to configuration → outputs formatted code

**Key Advantages**:
- Preserves complete comment information
- Supports complex code structure refactoring
- Deep integration with LSP

---

## Phase 0: Prerequisites (Source Map System) ✅ Completed

> **Importance**: This phase is a prerequisite for the formatter and must be completed first
> **Target Version**: v0.8
> **Implementation File**: `src/formatter/source_map.rs`

### 0.1 Source Map Data Structure

**Implementation Objectives**:
- Design and implement `SourceMap` structure to record source code position mappings
- Record content: comment positions, whitespace areas, original Token positions
- Support conversion from character offset to line/column

**Data Structure Design**:
```
SourceMap {
    source: String,                    // Original source code
    comments: Vec<Comment>,            // Comment list
    tokens: Vec<TokenWithSpan>,        // Token + position
    line_offsets: Vec<usize>,          // Start offset of each line
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

**Implementation Objectives**:
- Modify the Lexer to record comments instead of skipping during parsing
- Return `SourceMap` along with `tokenize()` output
- Maintain backward compatibility without affecting existing Token stream

**Implementation Note**: Adopting an independent SourceMap scanning approach that does not modify the Lexer itself. `SourceMap::build()` independently scans the source code to extract comment information, completely unaffected by existing Lexer/Parser flow.

**Acceptance Criteria**:
- [x] Existing Token stream output unchanged
- [x] Comments returned as independent Tokens or additional information
- [x] No impact on parsing performance

**Test Items**:
- [x] Regression test: existing code parsing results unchanged
- [x] Comment collection test

---

## Phase 1: Formatter Core (CLI Command) ✅ Completed

> **Target Version**: v0.9
> **Implementation File**: `src/formatter/` directory

### 1.1 Formatter Basic Structure ✅ Completed

**Implementation Objectives**:
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
│   └── buffer.rs          # Formatting output buffer
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

**Implementation Objectives**:
- Implement basic expression formatting: literals, variables, binary operations, unary operations
- Implement function call formatting
- Implement parenthesis preservation strategy

**Implementation File**: `src/formatter/handlers/expr.rs`

**Formatting Rules**:
```
Literals:
  - Numbers: keep original format
  - Strings: according to single_quote configuration
  - Booleans: true/false

Binary Operations:
  - One space before and after operator
  - Maintain operator precedence line breaks

Function Calls:
  - When arguments break to new line, align with commas
  - Single line keeps compact
```

**Acceptance Criteria**:
- [x] Literal formatting correct
- [x] Variable name formatting correct
- [x] Binary operation formatting correct
- [x] Function call formatting correct
- [x] Parenthesis pairs correctly preserved

**Test Items**:
- [x] Literal formatting test (numbers, strings, booleans)
- [x] Binary operation formatting test
- [x] Unary operation formatting test
- [x] Function call formatting test (single-line, multi-line arguments)
- [x] Parenthesis preservation test

---

### 1.3 Statement Formatting ✅ Completed

**Implementation Objectives**:
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
- [x] Variable declaration formatting correct
- [x] Function definition formatting correct
- [x] Type definition formatting correct
- [x] if/elif/else formatting correct
- [x] match formatting correct
- [x] while/for formatting correct

**Test Items**:
- [x] Variable declaration formatting test
- [x] Function definition formatting test (single-line, multi-line arguments)
- [x] Type definition formatting test
- [x] if/elif/else formatting test
- [x] match formatting test (multi-branch alignment)
- [x] while/for formatting test

---

### 1.4 Module Formatting ✅ Completed

**Implementation Objectives**:
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
  - Keep at most one blank line
  - Preserve blank lines around comments
  - Blank lines between top-level statements

Module Comments:
  - File header comments preserved
  - Doc comments preserved
```

**Acceptance Criteria**:
- [x] use statement formatting correct
- [x] Blank line handling correct
- [x] Module-level comments preserved
- [x] Top-level statement order unchanged

**Test Items**:
- [x] use statement formatting test
- [x] Blank line preservation test
- [x] Multiple statement formatting test
- [x] Module comment preservation test

---

### 1.5 Comment Handling ✅ Completed

**Implementation Objectives**:
- Implement single-line comment formatting
- Implement multi-line comment formatting
- Implement doc comment preservation
- Implement comment alignment

**Implementation File**: `src/formatter/handlers/comment.rs`

**Formatting Rules**:
```
Single-Line Comments:
  // Keep original position
  // Indent follows code

Multi-Line Comments:
  /* Keep original format */
  /* Multi-line
     comment */

Comment Alignment:
  // Comment
  let x = 1;
```

**Acceptance Criteria**:
- [x] Single-line comment position correct
- [x] Multi-line comment format preserved
- [x] Comment indentation correct
- [x] Comment relative position to code correct

**Test Items**:
- [x] Single-line comment preservation test
- [x] Multi-line comment preservation test
- [x] End-of-line comment test
- [x] Multi-line comment alignment test
- [x] Comment and code spacing test

---

### 1.6 Configuration Integration ✅ Completed

**Implementation Objectives**:
- Integrate existing `FmtConfig` configuration
- Support reading configuration from `yaoxiang.toml`
- Support CLI parameter override configuration

**Implementation File**: `src/formatter/options.rs`

**Configuration Items**:
```
[fmt]
line_width = 120        # Maximum line width
indent_width = 4        # Indentation width
use_tabs = false       # Use tabs
single_quote = false   # Single quotes for strings
```

**Acceptance Criteria**:
- [x] FmtConfig loaded correctly
- [x] CLI parameters override config file
- [x] Default values correct

**Test Items**:
- [x] Config file reading test
- [x] CLI parameter test
- [x] Default configuration test

---

### 1.7 CLI Command Integration ✅ Completed

**Implementation Objectives**:
- Implement `yaoxiang fmt <file>` command
- Implement `yaoxiang fmt <dir>` command
- Implement `--check` mode
- Implement `--write` in-place write mode
- Implement `--stdout` output to standard output

**Implementation File**: `src/main.rs` (Commands::Format)

**Command Design**:
```
yaoxiang fmt [OPTIONS] <PATH>...

Positional Arguments:
  PATH                  # File or directory path

Options:
  --check               # Check if formatting needed, don't modify file
  --write, -w           # Write in place (default outputs to stdout)
  --stdout              # Output to standard output (default)
  --indent <SIZE>       # Override indentation width
  --line-width <WIDTH>  # Override maximum line width
  --use-tabs            # Use tabs for indentation
  --single-quote        # Use single quotes

Exit Codes:
  0                     # File already formatted or written successfully
  1                     # Files need formatting in --check mode
  2                     # Error
```

**Acceptance Criteria**:
- [x] Formatting single file correct
- [x] Formatting directory correct (recursively processes .yx files)
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

**Implementation Objectives**:
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
- [x] Full document formatting response correct
- [x] Range formatting response correct
- [x] capabilities correctly declared

**Test Items**:
- [x] LSP full document formatting test
- [x] LSP range formatting test
- [x] capabilities declaration test

---

### 2.2 Document Change Triggers Formatting ✅ Completed

**Implementation Objectives**:
- Auto-format on document save (optional configuration)
- Implement `DocumentFormattingEdit` coordination

> **Note**: Save-triggered formatting can be implemented via editor-side configuration (such as VS Code's `editor.formatOnSave`), without requiring additional LSP server-side handling.

**Acceptance Criteria**:
- [x] Recommended to use editor configuration (VS Code formatOnSave)
- [x] No additional LSP server-side handling required

---

## Phase 3: Advanced Features ✅ Completed

### 3.1 Smart Line Breaking

**Implementation Objectives**:
- Implement smart line breaking when exceeding line width
- Maintain operator alignment
- Maintain function argument alignment

**Line Breaking Strategy**:
```
Function Call Line Breaking:
  foo(
      arg1,
      arg2,
  )

Expression Line Breaking:
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
- [x] Auto break when exceeding line width
- [x] Indentation correct after line breaks
- [x] Alignment maintained

**Test Items**:
- [x] Long expression line breaking test
- [x] Function argument line breaking test
- [x] Chained call line breaking test

---

### 3.2 Import Statement Sorting ✅ Completed

**Implementation Objectives**:
- Implement use statement auto-sorting
- Support grouped sorting (standard library, external crates, project internal)

**Sorting Rules**:
```
1. Standard Library (std, core, alloc)
2. External Crates (from Cargo.toml)
3. Project Internal (relative paths)

use std::collections::HashMap;
use crate::module::foo;
use some_crate::Bar;
```

**Acceptance Criteria**:
- [x] Import statements correctly sorted
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
- Compare snapshots after each modification
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
- [x] CLI command working correctly
- [x] --check mode working correctly

### Phase 2 ✅

- [x] LSP full document formatting
- [x] LSP range formatting
- [x] capabilities declared
- [x] Auto-format on save (recommended to use editor configuration)

### Phase 3 ✅

- [x] Smart line breaking
- [x] Import sorting

---

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Comment loss | Detailed source map system records comment positions |
| Semantic changes | AST equivalence testing before and after formatting |
| Performance issues | Incremental formatting, caching mechanism |
| Configuration conflicts | Clear configuration priority |

---

## References

- [Rustfmt Design Document](https://rust-lang.github.io/rfcs/rfcs-2437-rustfmt.html)
- [Prettier Architecture](https://prettier.io/docs/en/architecture)
- [Language Server Protocol Specification](https://microsoft.github.io/language-server-protocol/)