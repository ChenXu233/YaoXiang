# Task: Implement yaoxiang fmt Code Formatter

## Overview

`yaoxiang fmt` is a tool for automatically formatting YaoXiang source code, aimed at unifying community code style and reducing formatting disputes. It should reformat code according to language specifications (RFC-010, etc.) and user configurations.

## Goals

- Implement the `yaoxiang fmt` command, accepting files or directories as input, formatting code and outputting (or modifying in place).
- Support `--check` mode: check if code is already formatted, do not modify files, return non-zero exit code.
- Support configuration files (such as `yaoxiang.toml`) for customizing formatting rules (indent width, line break style, etc.).
- Preserve comments and original blank line intent (intelligent handling).
- Generated code should maintain semantic equivalence and conform to language syntax.

## Specific Steps

### 1. Design Formatting Rules (1 week)

- Determine indentation, whitespace, and line break conventions based on RFC-010's unified syntax.
- Reference Rustfmt's design to define configurable options: indent size, max line width, field alignment, etc.
- Write a design document for community discussion.

### 2. Implement AST to Code Reprinting (2 weeks)

- Use the existing parser to obtain a complete AST (including comment position information; if the current parser discards comments, it needs to be enhanced).
- Design a "printer" that traverses the AST and outputs formatted code according to rules.
- Handle special cases: multi-line strings, inline comments, blank lines, etc.

### 3. Support Configuration (1 week)

- Read `yaoxiang.toml` in the project root directory, merging with default configuration.
- Pass configuration to the printer, affecting indentation, line breaks, and other behaviors.

### 4. Implement `--check` Mode (3 days)

- After formatting files, compare original content with formatted content; if there are differences, report an error and list the differing files.
- Exit code non-zero indicates at least one file needs formatting.

### 5. Testing and Integration (1 week)

- Write extensive test cases covering all syntax features, ensuring semantic consistency before and after formatting.
- Test `--check` mode correctness in various scenarios.
- Ensure sufficient performance — at least thousands of lines per second.

## Acceptance Criteria

- Able to format complete YaoXiang files containing variables, functions, type definitions, methods, and control flow.
- Formatted code should pass parsing and type checking.
- Comments and blank lines should be reasonably preserved, not accidentally deleted or moved.
- `--check` mode should correctly detect unformatted files and report them.
- Support custom indent width (2 spaces / 4 spaces / tab).
- Handle common edge cases: multi-line function calls, long expression line breaks, field list alignment, etc.

## Dependencies

- The parser must preserve comment and whitespace information (or provide source mapping so code can be reconstructed from AST). If the current parser discards this information, it needs to be extended first.
- Requires understanding the complete syntax tree structure of the language.