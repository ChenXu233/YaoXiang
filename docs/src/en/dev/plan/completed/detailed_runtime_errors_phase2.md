# Phase 2: Detailed Runtime Error Tracking — Span Coverage, Multi-file SourceMap, Bytecode DebugSection, CLI Flags

> Status: Planned (driving implementation and acceptance)  
> Related: `detailed_runtime_errors.md` (Phase 1 core chain already landed)

## Goals (4 limitations to solve this phase)

1. **Span coverage**: Add `Span` to more IR instructions, and include them in DebugMap during the Codegen phase.
2. **Multi-file/module support**: Record source files in DebugMap; introduce `SourceMap` (`file_id -> path/content`) to support cross-module location and rendering.
3. **Bytecode file DebugSection**: Extend `.42` (BytecodeFile) format with an optional DebugSection; implement read/write to preserve location information during offline execution/debugging.
4. **CLI flags**: Add `--debug-info` to `run` / `build` (build-bytecode) commands, controlling DebugMap generation and (this phase) serialization.

## Design constraints (must satisfy)

- **Zero-cost interpreter main loop**: Runtime execution logic only depends on `ip` and `StackFrame` bubbling; no access/parsing of debug information on hot paths.
- **On-demand rendering**: Only when an error finally bubbles up to the CLI layer, load/lookup/render source code snippets.
- **Optional flag**: When `--debug-info` is off, no DebugMap is produced, no DebugSection is written, avoiding extra allocation and file bloat.

## Data structure proposal (avoiding breaking changes)

To avoid making `file_id` invasive to the existing `Span` (currently heavily relied upon by many components), this phase uses "compositional location":

- `Span` continues to express only line/column/offset (maintaining current state).
- New `FileId` (integer ID) and `DebugSpan { file_id, span }` added.
- New `SourceMap`: indexes `SourceFile { name, content, ... }` by `file_id`.
- Upgrade `BytecodeFunction.debug_map` value from `Span` to `DebugSpan`.

> Benefit: No widespread rewrites of Parser/LSP/TypeCheck span logic; only the Debug pipeline gets upgraded.

## BytecodeFile DebugSection (file format extension)

### Trigger conditions

- `DEBUG_INFO` bit set in `BytecodeFile.header.flags` (aligned with Codegen/CLI).

### Stored content (minimal closed loop)

- **SourceMap**: `file_count`, each file writes `path` and `content`.
- **DebugMap**: Stores `ip -> DebugSpan` mapping by function order:
  - `entry_count`
  - Each entry: `ip`, `file_id`, `Span.start(line/col/offset)`, `Span.end(line/col/offset)`

### Compatibility strategy

- When `DEBUG_INFO` is off: DebugSection is not written; old format remains unchanged.
- Reading: If flags not set, skip DebugSection; if set, parse and populate `FunctionCode.debug_map`.

## CLI flag behavior (acceptance criteria)

### `yaoxiang run <file> --debug-info`

- Enables DebugMap collection in Codegen.
- Runtime error output includes source code snippet highlighting and correct file name/path (for cross-file cases).

### `yaoxiang build <file> -o out.42 --debug-info`

- Writes DebugSection (containing SourceMap + DebugMap).
- Without the flag, `.42` contains no DebugSection.

## Span coverage expansion (suggested priority)

> Prioritize IR instructions that "may trigger runtime errors", filling in progressively.

### P0 (directly associated with existing error types)

- `Call/CallVirt/CallDyn`: `FunctionNotFound`
- `Div/Mod`: `DivisionByZero`

### P1 (reserving for future error types/checks)

- `LoadIndex/StoreIndex`: `IndexOutOfBounds` (future combination with `BoundsCheck`)
- `LoadField/StoreField`: `FieldNotFound`
- `PtrDeref/PtrLoad/PtrStore`: `InvalidHandle` / unsafe errors

## Testing and acceptance

- `BytecodeFile`: After DebugSection write, `read_from` completely restores `SourceMap` and `DebugMap` (round-trip).
- `render_runtime_error`: Can select the correct file from `SourceMap` based on `DebugSpan.file_id` for rendering (1 test case each for single-file and multi-file scenarios).
- CLI: `--debug-info` flag significantly changes output (on: source highlighting; off: ip/function name only).