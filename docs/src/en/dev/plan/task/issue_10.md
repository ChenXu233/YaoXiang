# Task: Developing Small Applications to Verify Language Features and Build Ecosystem

## Overview

The frontend, middle-end, and interpreter backend of YaoXiang have been completed, and the compiler backend is under development. To test the practicality of the language, discover hidden bugs, accumulate example code, and attract early users, a series of small applications need to be developed. These applications should run smoothly on the interpreter, not relying on the high-performance optimizations of the compiler backend, while fully demonstrating core design features such as YaoXiang's unified syntax, ownership model, and field-level mutability.

## Goals

- Develop at least 3 small applications across different domains, covering scenarios such as scripting tools, algorithm implementations, and configuration parsing.

- Each application should include complete source code, test cases, and brief documentation.

- Through the development process, provide feedback on language design issues and improve the standard library and toolchain.

## Suggested Application Directions (Optional or Self-Designed)

### Batch File Renaming Tool

**Functionality**: Traverse a specified directory, match filenames according to regular expressions, and perform renaming (replacement, adding prefix/suffix, etc.).

**Involved Features**: File system operations (requires FFI or standard library), string processing, ownership management (preventing resource leaks).

**Output**: Executable script with command-line argument support.

### Quick Sort Algorithm Demo

**Functionality**: Implement the classic quick sort algorithm, supporting sorting of integer arrays.

**Involved Features**: Recursion, array/list operations, generics (if supported), ownership flow (array element movement).

**Output**: Function library + test program demonstrating algorithm correctness.

### TOML Configuration Parser Prototype

**Functionality**: Parse simple TOML configuration files (only basic types and tables), mapping to YaoXiang record types.

**Involved Features**: String parsing, record types, error handling.

**Output**: Parser function + example configuration file and usage code.

### Terminal Tic-Tac-Toe Game

**Functionality**: Two-player tic-tac-toe game in the terminal, supporting board display, move placement, and win/loss determination.

**Involved Features**: State management (field mutability), input/output, loop control.

**Output**: Runnable game program.

## Acceptance Criteria

- Applications run as expected and pass all manual tests.

- Code follows YaoXiang style conventions (if any).

- No memory safety errors (guaranteed by ownership rules).

- Any language bugs or missing standard library features discovered during development are recorded as separate issues.

## Dependencies

- Interpreter backend must be stable and usable.

- May require file I/O, string manipulation modules from the standard library.