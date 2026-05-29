## Overview

`yaoxiang check` is a command-line tool for static analysis (type checking, ownership checking, lint) of YaoXiang source code, without generating any code. It should provide fast feedback, suitable for integration into CI pipelines or as a validation step during development.

## Goals

- Implement the `yaoxiang check` command that accepts one or more files/directories as input.
- Reuse the compiler's type checking module, outputting errors and warnings to stderr.
- Support `--json` option for JSON-formatted diagnostic output, facilitating integration with other tools.
- Support `--watch` mode (optional) to monitor file changes and re-check automatically.
- Error messages should include file name, line number, column number, severity level, and description.

## Specific Steps

1. **Design Command-Line Interface** (3 days)
   - Use `clap` or similar library for argument parsing: `yaoxiang check [OPTIONS] <PATH>...`.
   - Support options like `--json`, `--watch`, `--color`.

2. **Integrate Type Checker** (1 week)
   - Encapsulate the type checking module as independently callable functions that accept source code strings and return a diagnostic list.
   - Support multi-file checking: requires parsing all files and building a global symbol table.
   - Handle standard library paths and builtin definitions.

3. **Output Formatting** (3 days)
   - Implement colorized error output similar to Rustc, including error location, severity, and description.
   - Implement JSON format output following common conventions (e.g., similar to `rustc --error-format=json`).

4. **Watch Mode Support (Optional)** (1 week)
   - Use filesystem monitoring libraries (such as `notify`) to watch for file changes and automatically re-check.
   - Avoid redundant full checks; perform incremental checks when possible.

5. **Testing and Integration** (1 week)
   - Write test cases: execute checks on files with errors and verify that the output matches expectations.
   - Integrate into CI pipeline to ensure every commit passes checks.

## Acceptance Criteria

- Running `yaoxiang check` on a single error-free file should produce no output (or output success message) with exit code 0.
- Running checks on a file containing type errors should output clear error information (file name, line number, column number, description) with a non-zero exit code.
- The `--json` option should output a valid JSON array, with each element containing error location and message.
- Check speed should be close to the compiler's type checking speed, suitable for frequent use.
- Should correctly handle multiple files in a project and recognize cross-file references.

## Dependencies

- The type checking module must support reading files from the filesystem and constructing a global environment.
- Standard library definitions must be accessible (either built into the compiler or specified via path).