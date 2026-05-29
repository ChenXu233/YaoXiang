# yaoxiang check Command Implementation Record (Landed)

## Overview

`yaoxiang check` has been extended to serve as a static checking command for CI and local quick validation: supports multiple path inputs, text/JSON output, watch re-checking, and unified error counting.

This document has been updated from "Implementation Plan" to "Implementation Record + Follow-up Improvements", and is now consistent with the current codebase.

## Implemented Capabilities

- Supports one or more input paths (files or directories): `yaoxiang check <PATH...>`
- Recursively collects `.yx` files in directories
- Supports `--json` for structured diagnostic output
- Supports `--watch` for monitoring and automatic re-checking
- Supports `--color auto|always|never`
- Supports `--no-progress` to disable progress/summary info
- Exit code `1` on errors, exit code `0` on success, exit code `2` when no valid files in input

## CLI Arguments (Current)

`yaoxiang check [OPTIONS] <PATH>...`

- `--json`: Output JSON
- `--watch` / `-w`: Monitor file changes
- `--color <auto|always|never>`: Control colored output
- `--no-progress`: Suppress progress and summary output

## Core Implementation

### 1) CLI Extension

- `Commands::Check` has been upgraded from single file argument to multi-path argument `paths: Vec<PathBuf>`
- Added `json/watch/color/no_progress` options
- Check flow split into:
  - `run_check_once(...)`
  - `run_check_watch(...)`

### 2) Diagnostic Aggregation

Added in `util/diagnostic/mod.rs`:

```rust
pub struct CheckDiagnostic {
    pub file: String,
    pub diagnostic: Diagnostic,
}

pub struct CheckResult {
    pub diagnostics: Vec<CheckDiagnostic>,
    pub source_files: HashMap<String, SourceFile>,
    pub error_count: usize,
    pub warning_count: usize,
}

pub fn check_files_with_diagnostics(files: &[PathBuf]) -> anyhow::Result<CheckResult>
```

Behavior:
- Compiles each input file individually and aggregates diagnostics
- No longer "stops on first error"
- Retains `check_file_with_diagnostics` as a compatible entry point (internally reuses multi-file implementation)

### 3) Output Format

- Text output: uses `TextEmitter`, supports color toggle
- JSON output: includes
  - `error_count`
  - `warning_count`
  - `diagnostics[]` (containing `file/severity/code/message/line/column/...`)
  - `lsp` field (converted by `JsonEmitter`)

### 4) Watch Mode

- Based on `notify`'s `RecommendedWatcher`
- Monitors input paths (recursive for directories, non-recursive for files)
- Only triggers checks for `.yx` related events
- Includes debounce window (250ms)

## Differences from Original Plan

1. Current watch uses "full re-check after debounce", without implementing "only re-check changed files + incremental result caching".
2. Current compilation pipeline exposes mainly the first structured diagnostic for each failing file, without achieving "complete diagnostic list returned per file".
3. Cross-file global symbol union analysis has not been additionally implemented in the `check` command (relies on existing compiler behavior).

## Verification Results

Verification completed:

- `cargo check --bin yaoxiang` passes
- Unit tests pass:
  - `test_check_files_with_diagnostics_ok`
  - `test_check_files_with_diagnostics_error`
- Smoke tests pass:
  - `cargo run -- check tests/yaoxiang/list_test.yx --json --no-progress`
  - Temporary error file check returns exit code `1`, with filename, line/column, severity, and message in output

## Follow-up Suggestions

1. Add incremental caching for `check --watch` to avoid full scan every time.
2. Expand error collection at the frontend/pipeline layer to support complete multi-diagnostic output per file.
3. Add CLI integration tests (process-level) covering exit codes, JSON structure, directory input, and watch behavior.