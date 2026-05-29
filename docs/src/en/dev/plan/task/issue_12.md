## Overview
`yaoxiang check` is a command-line tool for performing static checks (type checking, ownership checking, lint) on YaoXiang source code without generating any code. It provides fast feedback and is suitable for integration into CI pipelines or as a validation step during development.

## Goals
- Implement the `yaoxiang check` command that accepts one or more files/directories as input.
- Reuse the compiler's type checking module, outputting errors and warnings to standard error.
- Support the `--json` option to output diagnostic information in JSON format for integration with other tools.
- Support `--watch` mode (optional) to monitor file changes and re-check automatically.
- Error messages should include file name, line number, column number, error level, and description.

## Specific Steps

1. **Design CLI** (3 days)
   - Use `clap` or a similar library to parse arguments: `yaoxiang check [OPTIONS] <PATH>...`.
   - Support options like `--json`, `--watch`, `--color`, etc.

2. **Integrate Type Checker** (1 week)
   - Encapsulate the type checking module as independently callable functions that receive source code strings and return a diagnostic list.
   - Support multi-file checking: requires parsing all files and constructing a global symbol table.
   - Handle standard library paths and built-in definitions.

3. **Output Formatting** (3 days)
   - Implement colored error output similar to rustc, including error location, error level, and description.
   - Implement JSON format output following common conventions (similar to `rustc --error-format=json`).

4. **Support Watch Mode (Optional)** (1 week)
   - Use a file system monitoring library (such as `notify`) to monitor file changes and automatically re-check.
   - Avoid redundant full checks; perform incremental checks where possible.

5. **Testing and Integration** (1 week)
   - Write test cases: execute checks on files with errors and verify that output matches expectations.
   - Integrate into CI pipeline to ensure every commit passes checks.

## Acceptance Criteria
- Executing `yaoxiang check` on a single error-free file should produce no output (or a success message) with exit code 0.
- Executing checks on a file containing type errors should output clear error information (file name, line number, column number, description) with a non-zero exit code.
- The `--json` option should output a valid JSON array where each element contains error location and message.
- Check speed should be close to the compiler's type checking speed, suitable for frequent use.
- Should correctly handle multiple files in a project and recognize cross-file references.

## Dependencies
- The type checking module must support reading files from the file system and constructing a global environment.
- Standard library definitions must be accessible (either built into the compiler or specified via path).