# Task: Developing Small Applications to Verify Language Features and Build Ecosystem

## Overview

The frontend, middle-end, and interpreter backend of YaoXiang are complete, while the compiler backend is under development. To test the language's practicality, discover hidden bugs, accumulate sample code, and attract early users, a series of small applications needs to be developed. These applications should run smoothly on the interpreter without relying on high-performance optimizations from the compiler backend, while fully demonstrating YaoXiang's core design features such as unified syntax, ownership model, and field-level mutability.

## Goals

Develop at least 3 small applications across different domains, covering scenarios such as scripting tools, algorithm implementations, and configuration parsing.

Each application should include complete source code, test cases, and brief documentation.

Through the development process, provide feedback on language design issues and improve the standard library and toolchain.

## Suggested Application Directions (optional or self-designed)

### Batch File Renaming Tool

**Functionality:** Traverse a specified directory, match filenames using regular expressions, and perform renaming (replacement, adding prefix/suffix, etc.).

**Involved Features:** File system operations (requires FFI or standard library), string processing, ownership management (preventing resource leaks).

**Output:** Executable script with command-line argument support.

### QuickSort Algorithm Demonstration

**Functionality:** Implement the classic QuickSort algorithm, supporting sorting of integer arrays.

**Involved Features:** Recursion, array/list operations, generics (if supported), ownership flow (array element moving).

**Output:** Function library + test program demonstrating algorithm correctness.

### TOML Configuration Parser Prototype

**Functionality:** Parse simple TOML configuration files (only basic types and tables supported), mapping to YaoXiang record types.

**Involved Features:** String parsing, record types, error propagation.

**Output:** Parser function + example configuration files and usage code.

### Terminal Tic-Tac-Toe Game

**Functionality:** Two-player terminal Tic-Tac-Toe with board display, move input, and win detection.

**Involved Features:** State management (field mutability), input/output, loop control.

**Output:** Runnable game program.

## Acceptance Criteria

Applications run as expected and pass all manual tests.

Code follows YaoXiang style guidelines (if any).

No memory safety errors (guaranteed by ownership rules).

Any language bugs or standard library gaps discovered during development are recorded as separate issues.

## Dependencies

The interpreter backend must be stable and usable.

Standard library modules for file I/O, string operations, etc. may be required.