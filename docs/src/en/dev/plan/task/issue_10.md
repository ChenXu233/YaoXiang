# Task: Develop Small Applications to Verify Language Features and Build Ecosystem

## Overview

The front-end, middle-end, and interpreter backend of YaoXiang have been completed, while the compiler backend is under development. To test the language's practicality, discover hidden bugs, accumulate example code, and attract early users, a series of small applications need to be developed. These applications should run smoothly on the interpreter without relying on high-performance optimizations from the compiler backend, while fully demonstrating YaoXiang's core design features such as unified syntax, ownership model, and field-level mutability.

## Goals

- Develop at least 3 small applications across different domains, covering scenarios like scripting tools, algorithm implementations, and configuration parsing.

- Each application should include complete source code, test cases, and brief documentation.

- Through the development process, provide feedback on language design issues and improve the standard library and toolchain.

## Suggested Application Directions (Optional or Self-Designed)

### Batch File Renaming Tool

**Features**: Traverse a specified directory, match filenames using regular expressions, and perform renaming (replacement, adding prefixes/suffixes, etc.).

**Involves**: File system operations (requires FFI or standard library), string processing, ownership management (preventing resource leaks).

**Output**: Executable script supporting command-line arguments.

### QuickSort Algorithm Demonstration

**Features**: Implement the classic QuickSort algorithm, supporting sorting of integer arrays.

**Involves**: Recursion, array/list operations, generics (if supported), ownership flow (array element movement).

**Output**: Function library + test program, demonstrating algorithm correctness.

### TOML Configuration Parser Prototype

**Features**: Parse simple TOML configuration files (basic types and tables only), mapping to YaoXiang record types.

**Involves**: String parsing, record types, error propagation.

**Output**: Parser function + example configuration files and usage code.

### Terminal Tic-Tac-Toe Game

**Features**: Two-player terminal Tic-Tac-Toe with board display, moves, and win detection.

**Involves**: State management (field-level mutability), input/output, loop control.

**Output**: Runnable game program.

## Acceptance Criteria

- Applications run as expected and pass all manual tests.

- Code follows YaoXiang style conventions (if any).

- No memory safety errors (guaranteed by ownership rules).

- Any language bugs or standard library deficiencies discovered during development have been recorded as separate issues.

## Dependencies

- The interpreter backend must be stable and usable.

- May require file I/O and string manipulation modules from the standard library.