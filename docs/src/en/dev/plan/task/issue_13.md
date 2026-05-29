# Task: Implement yaoxiang fmt Code Formatter

## Overview

`yaoxiang fmt` is an automatic formatting tool for YaoXiang source code, designed to unify community code style and reduce formatting disputes. It should reformat code according to language specifications (RFC-010, etc.) and user configurations.

## Goals

- Implement the `yaoxiang fmt` command that accepts files or directories as input, formats code and outputs (or modifies in place).
- Support `--check` mode: check if code is already formatted, do not modify files, return non-zero exit code.
- Support configuration files (such as `yaoxiang.toml`) to customize formatting rules (indent width, line breaking style, etc.).
- Preserve the intent of comments and original blank lines (intelligent handling).
- Generated code should maintain semantic equivalence and conform to language syntax.

## Specific Steps

1. **Design Formatting Rules** (1 week)
   - Determine conventions for indentation, spacing, and line breaks based on RFC-010 unified syntax.
   - Reference Rustfmt's design to define configurable options: indent size, max line width, field alignment, etc.
   - Write a design document for community discussion.

2. **Implement AST to Code Reprinting** (2 weeks)
   - Use the existing parser to obtain a complete AST (including comment position information; if the current parser discards comments, it needs enhancement).
   - Design a "printer" that traverses the AST and outputs formatted code according to rules.
   - Handle special cases: multi-line strings, inline comments, blank lines, etc.

3. **Configuration Support** (1 week)
   - Read `yaoxiang.toml` in the project root directory and merge with default configuration.
   - Pass configuration to the printer to affect indentation, line breaking, and other behaviors.

4. **Implement `--check` Mode** (3 days)
   - Compare original content with formatted content after formatting; if there are differences, report an error and list the differing files.
   - Exit code is non-zero if at least one file needs formatting.

5. **Testing and Integration** (1 week)
   - Write extensive test cases covering all syntax features to ensure semantic consistency before and after formatting.
   - Test `--check` mode correctness under various scenarios.
   - Ensure sufficient performance, at least processing thousands of lines of code per second.

## Acceptance Criteria

- Capable of formatting complete YaoXiang files containing variables, functions, type definitions, methods, and control flow.
- Formatted code should pass parsing and type checking.
- Comments and blank lines should be reasonably preserved and not accidentally deleted or moved.
- `--check` mode should correctly detect unformatted files and report them.
- Support custom indent width (2 spaces / 4 spaces / tab).
- Handle common edge cases: multi-line function calls, long expression line breaks, field list alignment, etc.

## Dependencies

- The parser must preserve comment and whitespace information (or provide source mapping so code can be reconstructed from AST). If the current parser discards this information, it needs to be extended first.
- Knowledge of the complete syntax tree structure of the language is required.