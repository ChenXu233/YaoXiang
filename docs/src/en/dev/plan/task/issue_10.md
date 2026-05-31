# Task: Developing Small Applications to Validate Language Features and Accumulate Ecosystem

## Overview

The front-end, middle-end, and interpreter back-end of YaoXiang have been completed, and the compiler back-end is under development. To test the practicality of the language, discover hidden bugs, accumulate sample code, and attract early users, a series of small applications need to be developed. These applications should run smoothly on the interpreter without relying on the compiler back-end's high-performance optimizations, while fully demonstrating YaoXiang's core design features such as unified syntax, ownership model, and field-level mutability.

## Goals

Develop at least 3 small applications across different domains, covering scenarios such as scripting tools, algorithm implementations, and configuration parsing.

Each application should include complete source code, test cases, and brief documentation.

Through the development process, provide feedback on language design issues and improve the standard library and toolchain.

## Suggested Application Directions (optional or self-designed)

### File Batch Rename Tool

Functionality: Traverse a specified directory, match filenames according to regular expressions, and perform renaming (replacement, adding prefixes/suffixes, etc.).

Involved features: File system operations (requires FFI or standard library), string processing, ownership management (preventing resource leaks).

Output: Executable script with command-line argument support.

### QuickSort Algorithm Demo

Functionality: Implement the classic quicksort algorithm, supporting sorting of integer arrays.

Involved features: Recursion, array/list operations, generics (if supported), ownership flow (array element movement).

Output: Function library + test program demonstrating algorithm correctness.

### TOML Configuration Parser Prototype

Functionality: Parse simple TOML configuration files (only basic types and tables), mapping to YaoXiang record types.

Involved features: String parsing, record types, error handling.

Output: Parser function + example configuration file and usage code.

### Terminal Tic-Tac-Toe Game

Functionality: Two-player tic-tac-toe in the terminal, supporting board display, move placement, and win/loss determination.

Involved features: State management (field mutability), input/output, loop control.

Output: Runnable game program.

## Acceptance Criteria

Applications run as expected and pass all manual tests.

Code follows YaoXiang style guidelines (if any).

No memory safety errors (guaranteed by ownership rules).

Any language bugs or standard library deficiencies discovered during development have been recorded as separate issues.

## Dependencies

The interpreter back-end must be stable and usable.

May require file I/O, string manipulation modules from the standard library.