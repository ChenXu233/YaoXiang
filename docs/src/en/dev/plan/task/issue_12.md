## Overview

`yaoxiang check` is a command-line tool for performing static analysis (type checking, ownership checking, lint) on YaoXiang source code, without generating any code. It should provide fast feedback and be suitable for integration into CI pipelines or as a validation step during development.

## Goals

- Implement the `yaoxiang check` command, accepting one or more files/directories as input.
- Reuse the compiler's type checking module, outputting errors and warnings to standard error.
- Support the `--json` option for JSON-formatted diagnostic output, facilitating integration with other tools.
- Support `--watch` mode (optional) to monitor file changes and re-check automatically.
- Error messages should include filename, line number, column number, severity level, and description.

## Specific Steps

1. **Design Command-Line Interface** (3 days)
   - Use `clap` or a similar library for argument parsing: `yaoxiang check [OPTIONS] <PATH>...`.
   - Support options such as `--json`, `--watch`, `--color`, etc.

2. **Integrate Type Checker** (1 week)
   - Encapsulate the type checking module as an independently callable function that accepts source code strings and returns a diagnostic list.
   - Support multi-file checking: requires parsing all files and building a global symbol table.
   - Handle standard library paths and builtin definitions.

3. **Output Formatting** (3 days)
   - Implement colored error output similar to rustc, including error location, severity level, and description.
   - Implement JSON format output, following common conventions (such as similar to `rustc --error-format=json`).

4. **Watch Mode Support (Optional)** (1 week)
   - Use a file system monitoring library (such as `notify`) to watch for file changes and automatically re-check.
   - Avoid redundant full checks, using incremental checking where possible.

5. **Testing and Integration** (1 week)
   - Write test cases: execute check on files with errors, verify that output matches expectations.
   - Integrate into CI pipeline to ensure every commit passes checks.

## Acceptance Criteria

- Running `yaoxiang check` on a single error-free file should produce no output (or a success message) with exit code 0.
- Running check on a file containing type errors should output clear error information (filename, line number, column number, description) with a non-zero exit code.
- The `--json` option should output a valid JSON array, where each element contains error location and message.
- Check speed should be close to the compiler's type checking speed, suitable for frequent use.
- Should correctly handle multiple files in a project and recognize cross-file references.

## Dependencies

- The type checking module must support reading files from the file system and building a global environment.
- Standard library definitions need to be accessible (either builtin in the compiler or specified via path).