# Runtime Detailed Error Tracing: Detached Storage & Deferred Loading

> Status: Core path implemented (2026-03-15); DebugSection / CLI switches implemented (2026-03); More comprehensive Span coverage is still future work.

## 1. Background & Goals

Currently, when the YaoXiang interpreter throws runtime errors (e.g., `Runtime error: Function not found: int.to_string`), it can only output the function name and instruction pointer (IP), such as `at main (ip: 0)`. This makes it difficult for developers to intuitively locate errors to specific source code lines and columns.

To provide diagnostic output similar to the Rust compiler's rich and detailed code highlighting, while **absolutely not allowing any time overhead or memory overhead in the core interpretation execution loop**, this plan proposes a "Detached Storage & Deferred Loading" architecture.

## 2. Core Design Principles

1. **Zero-Cost Runtime**: The virtual machine execution logic (Interpreter/Runtime) never processes complex source file association objects (such as Span, file identifiers), only retaining the instruction pointer `ip` when an exception occurs.
2. **Side-Table Storage**: Generated `Span` tracking information is stored in a dedicated `DebugSection / DebugMap` structure, not participating in hot code caching to prevent slowing down instruction fetching performance.
3. **Deferred Rendering on Demand**: Source fragment alignment and rendering only occurs at the "exact moment an exception occurs and bubbles up to the triggering point (CLI/TUI)". During normal execution, there is no behavior of assembling strings "just in case of potential errors".

## 3. Current Implementation Overview (Implemented)

- **DebugMap贯通 (DebugMap贯通)**: Optional generation of `ip -> Span` mapping during the Codegen phase, passing through to `BytecodeFunction.debug_map` at runtime.
- **Zero overhead in interpreter**: The interpreter's main execution loop does not access `Span/debug_map`; runtime errors still rely solely on `StackFrame { function_name, ip }` for bubbling.
- **Top-level deferred rendering**: CLI top-level captures `ExecutorError`, uses `debug_map` to map `ip` back to `Span`, calls existing `TextEmitter` to output diagnostics with code fragment highlighting, and appends stack trace.

## 4. Specific Implementation Points (Corresponding to Original Plan Phases)

### Phase 1: Bytecode Data Structure Expansion (Middle Core / Bytecode) ✅

`BytecodeFunction` already includes the `debug_map: HashMap<usize, Span>` field, and the data link from Codegen to Runtime has been completed in this implementation:

```rust
pub struct BytecodeFunction {
    // ... existing logic
    /// Debug info: mapping from IP to source Span
    pub debug_map: std::collections::HashMap<usize, crate::util::span::Span>,
}
```

Note: The `FunctionCode` on the Codegen side has also added a field with the same name, which is passed through to `BytecodeFunction` during `BytecodeModule::from(BytecodeFile)` conversion.

*Future*: Bytecode file serialization/deserialization (`src/middle/passes/codegen/bytecode.rs`) DebugSection write/read is not yet implemented.

### Phase 2: Collecting Mappings During IR Generation (Codegen Translation) ✅

In the workflow of `src/middle/passes/codegen/translator.rs`:
- Before writing `FunctionCode.instructions.push(...)`, use `instructions.len()` as the upcoming `ip`.
- Extract `Span` from the current IR instruction and write `debug_map[ip] = span`.
- Controlled by `CodegenContext::set_generate_debug_info(bool)` whether to generate DebugMap (when disabled, each function retains only an empty `HashMap` to avoid additional allocation).

Currently precisely locatable IR instructions (with `Span`):
- `Call / CallVirt / CallDyn` (runtime errors related to function calls, such as `FunctionNotFound`)
- `Div / Mod` (such as division by zero errors)
- `Store / StoreField / StoreIndex` (reserved for more runtime error types in the future)

To support the above mappings, the IR layer has added `span: Span` fields to `Call/CallVirt/CallDyn/Div/Mod` in `src/middle/core/ir.rs`, and fills them in `src/middle/core/ir_gen.rs` (does not affect interpreter runtime performance).

### Phase 3: Lightweight Bubbling of Runtime Stack (Interpreter Execution) ✅

* Maintain the `StackFrame` data in `src/backends/mod.rs` unchanged, only carrying:
  ```rust
  pub struct StackFrame {
      pub function_name: String,
      pub ip: usize,
  }
  ```
* When an exception is encountered (such as: division by zero, `FunctionNotFound`), the `ExecutorError` is triggered, and the rudimentary `StackFrame` returns by popping the stack layer by layer (Err mapping).

### Phase 4: Top-Level Interception & Deferred Rendering with Highlighting (CLI / Diagnostics) ✅

Added `render_runtime_error(...)` in `src/util/diagnostic/mod.rs` and called after capturing runtime errors in `run_file_with_diagnostics(...)`:
1. Capture the finally thrown `ExecutorError`, unpack `stack_trace()`.
2. Get the frame information that triggered the crash (first frame), obtain `function_name` and `ip` from it.
3. Locate the corresponding `BytecodeFunction` in the current `BytecodeModule`, use `debug_map.get(&ip)` to retrieve `Span`.
4. **Deferred Rendering**: Only call `TextEmitter` to render code fragment highlighting when an error occurs, and append stack trace text.

Note: Currently `Span` does not carry `file_id`; single-file entry reads source code at CLI top-level and constructs `SourceFile` (required for compilation). Runtime error rendering reuses this `SourceFile`, not bringing IO/string processing into the interpreter main loop.

---

## 5. Task Breakdown Checklist

- [x] **Data Model Preparation**: `BytecodeFunction.debug_map` and `FunctionCode.debug_map` data links are connected.
- [x] **Code Generation Alignment**: Translator collects `Span` by `ip` when generating bytecode (optional switch).
- [x] **Configuration & Optional Serialization**: `CodegenContext::set_generate_debug_info(bool)` + `.42` DebugSection read/write + CLI `--debug-info` implemented (2026-03).
- [x] **Top-Level Error Capture Layer Construction**: `run_file_with_diagnostics` captures `ExecutorError` and calls `render_runtime_error`.
- [x] **Terminal Rendering**: Uses `TextEmitter` to output runtime errors with source code fragment highlighting, and appends stack trace.

## 6. Known Limitations & Future Work

1. **Span Coverage**: Expanded to `LoadField/LoadIndex` based on Phase 1 (included in DebugMap); more IR instructions can continue to have `Span` added to cover more runtime error types.
2. **Multi-file/Module**: `Span` itself still contains only row/column information, but DebugMap has been upgraded to `ip -> (file_id + span)`, and `SourceMap` (`file_id -> path/content`) has been introduced for cross-file rendering; future work can introduce real multi-file `file_id` allocation strategy in the compilation pipeline.
3. **Bytecode File DebugSection**: Extended `.42` format with optional DebugSection (sources + ip mapping) and implemented read/write; can be used to retain positioning information during offline execution/debugging.
4. **CLI Switches**: Added `--debug-info` for `run` / `build` to control DebugMap generation and `.42` DebugSection embedding.