---
title: 'RFC-034: Unified Debugging Toolchain'
status: 'Draft'
author: 'Chen Xu'
created: '2026-07-06'
updated: '2026-07-06'
issue: '#164'
---

# RFC-034: Unified Debugging Toolchain

## Summary

Introduce a unified debugging toolchain for YaoXiang. The core design is **one source, three
consumers**: the compiler frontend embeds source locations, variable names, and type information as
first-class citizens into YaoXiang IR, and the interpreter, JIT, and LLVM backends each consume the
same metadata. Users start a DAP (Debug Adapter Protocol) server via `yaoxiang run --debug`, and VS
Code connects via stdio, providing a unified experience for breakpoints, stepping, variable
inspection, call stacks, expression evaluation, and concurrent debugging—regardless of which
execution engine is underneath.

## Motivation

### Why is this feature needed?

The current methods for investigating errors in YaoXiang programs are extremely primitive:

```yaoxiang
io.println("DEBUG: x = " + x.to_string())
io.println("DEBUG: entered branch A")
```

Three critical problems:

1. **Compiler self-hosting blocked**: Writing the YaoXiang compiler in YaoXiang, but the people
   writing the compiler cannot debug their own code. The lack of interactive debugging during the
   bootstrap phase is a dead end.
2. **Three engines, zero debugging**: The interpreter, JIT, and LLVM each run independently; when
   problems occur, users can only check whether `ALL TESTS PASSED` appeared in stdout. Assertion
   failed? Don't know which line, don't know variable values.
3. **Concurrency is a black box**: `spawn` creates multiple tasks—which task crashed? Which task
   moved the variable? Pure guesswork.

### Design Goals

- **Unified experience**: Breakpoints that work in the interpreter work in JIT, and LLVM has
  consistent source mapping. Users are unaware of underlying engine differences.
- **One source**: Debugging metadata flows with IR, no duplicate definitions, no maintaining two
  sets of mappings.
- **Zero intrusion**: One parameter `yaoxiang run --debug`; when this parameter is not added,
  compilation and execution behavior are completely unchanged.
- **DAP standard**: Directly integrate with the VS Code ecosystem, do not reinvent editor protocols.

## Proposal

### Core Design

Architecture overview:

```
┌──────────────────────────────────────────────────────┐
│                    VS Code / Editor                  │
│              DAP Client (launch.json)                │
└────────────────────────┬─────────────────────────────┘
                         │ stdio
┌────────────────────────▼─────────────────────────────┐
│                 DAP Server (yx-core)                  │
│  ┌─────────┐  ┌──────────┐  ┌───────────────────┐    │
│  │ Session │  │ Breakpoint│  │ Expression Eval   │    │
│  │ Manager │  │ Manager  │  │ Engine            │    │
│  └─────────┘  └──────────┘  └───────────────────┘    │
└────────────────────────┬─────────────────────────────┘
                         │ Query/Control
┌────────────────────────▼─────────────────────────────┐
│                  Runtime Debug Interface (trait)     │
│  pause / resume / step / get_frames / eval / ...    │
└────┬──────────────────┬──────────────────┬──────────┘
     │                  │                  │
┌────▼────┐    ┌───────▼───────┐    ┌─────▼──────┐
│Interpreter│  │ JIT (RFC-028)│    │ LLVM AOT   │
│Consumes IR│  │ Generates    │    │ IR metadata│
│metadata   │  │ lightweight  │    │ → DWARF   │
│directly   │  │ debug tables │    │           │
└─────────┘    └───────────────┘    └────────────┘
```

**Key design decisions**:

1. **DAP server and runtime are decoupled via trait**. The server doesn't care whether the backend
   is interpreter or JIT—it only issues commands through the `DebugEngine` trait. Each engine
   independently implements the same trait.
2. **`yaoxiang run --debug` forces the interpreter**. Debugging requires controllability, not
   performance. LLVM mode only generates DWARF for post-mortem (core dump / crash report), no
   interactive debugging.
3. **Reuse entry point discovery logic from `yaoxiang run`**. No new subcommand; the mental model is
   simply "run my program in debug mode".

### IR Debug Metadata

Attach metadata to existing YaoXiang IR without adding new IR types. All metadata is generated in
one place at the **compiler frontend**, and backend consumption is read-only:

| Metadata         | Attachment Point                   | Description                                  |
| ---------------- | ---------------------------------- | -------------------------------------------- |
| `SourceLocation` | Each IR node                       | Source file:line:column                      |
| `VarName`        | Variable declaration/binding nodes | Variable name in source code                 |
| `TypeAnnotation` | Variable/expression nodes          | Inferred type (with compile-time predicates) |
| `ScopeBoundary`  | Block/function entry/exit          | Variable scope lifecycle                     |
| `SpanInfo`       | spawn nodes                        | Task boundaries within spawn blocks          |

### Startup Flow

```
yaoxiang run --debug file.yx
    │
    ├── Phase 1: Compilation (with debug metadata)
    │   ├── Parsing → AST
    │   ├── Type checking + compile-time predicate verification
    │   └── Lowering to IR (with debug metadata attached)
    │
    ├── Phase 2: Use interpreter engine
    │   └── In --debug mode, use the interpreter regardless of --release
    │
    ├── Phase 3: Start DAP server
    │   ├── Initialize stdio transport channel
    │   ├── Wait for VS Code attach
    │   ├── Pause at program entry after attach succeeds
    │   └── Enter interactive debug loop
    │
    └── Phase 4: Program ends / Debug session ends → Exit
```

### Mode Differences

| Mode                     | Debugging Method                                                   |
| ------------------------ | ------------------------------------------------------------------ |
| `yaoxiang run --debug`   | Forces interpreter, full-featured DAP interactive debugging        |
| `yaoxiang run --release` | Generates DWARF for post-mortem (core dump / crash report), no DAP |
| `yaoxiang run` (normal)  | No debug metadata, no debugging support                            |

## Detailed Design

### 1. Breakpoints

```
Breakpoint types:
├── Source line breakpoint    → Frontend generates location metadata, backend queries for matches
├── Function entry breakpoint → Triggers on function call (Phase 2)
├── Conditional breakpoint    → Triggers when expression evaluates to true
└── Data breakpoint           → Triggers when variable is modified (Phase 2)
```

**Source line breakpoint core logic**:

```
VS Code sends: "Set breakpoint at file.yx:42"
    │
DAP server:
    ├── Query all nodes in IR where SourceLocation == (file.yx, 42)
    ├── Forward to runtime: "Pause at these IR addresses"
    └── Runtime returns: breakpoint ID

Program runs to IR node → Runtime checks: Is this node in the breakpoint list?
    ├── Normal breakpoint → Pause, notify DAP server
    └── Conditional breakpoint → Evaluate condition expression → Pause only if true
```

**Implementation across three engines**:

|                             | Interpreter                         | JIT                                           | LLVM                                      |
| --------------------------- | ----------------------------------- | --------------------------------------------- | ----------------------------------------- |
| Breakpoint insertion        | Checks IR node ID in execution loop | JIT inserts `int3` in machine code            | Uses LLVM DWARF + hardware breakpoints    |
| Conditional breakpoint eval | Directly interprets expression      | Temporarily JIT compiles condition expression | DWARF expression stack + eval             |
| Performance overhead        | One extra table lookup per IR node  | Overhead only at breakpoints                  | Near zero overhead (hardware breakpoints) |

### 2. Stepping

```
Step Over    → Execute current line, skip inside function calls, stop at next line
Step Into    → Enter inside the function call on current line
Step Out     → Execute until current function returns
Continue     → Resume execution until next breakpoint or program end
```

**Implementation logic**: Stepping operations are essentially **temporary breakpoints**. They share
the same mechanism with user-defined breakpoints—not two systems, but two uses of one system.

```
Step Over:
    Current source line = query_line(frame)
    → Set temporary breakpoint on next line
    → If current line is a function call: Set temporary breakpoint after call point
    → Continue → Hit temporary breakpoint → Delete → Pause

Step Into:
    First executable position of current call target
    → Find source location of first IR node in function body
    → Set temporary breakpoint → Continue → Hit → Pause

Step Out:
    Return address of current stack frame
    → Find next line of caller
    → Set temporary breakpoint → Continue → Hit → Pause
```

**Four edge cases for temporary breakpoints**:

1. **Concurrency ownership**: Temporary breakpoints are bound to current task ID; other tasks
   hitting them are ignored.
2. **Step Over spawn blocks**: Step Over outside spawn runs the entire spawn block and jumps after
   it. Use Step Into to enter spawn internals for debugging.
3. **Temporary breakpoint miss**: Set watchdog timeout (30 seconds with no breakpoint hit) → Force
   pause → Notify VS Code. Also listen for program exit events → Clean up immediately.
4. **Multiple IR nodes on same line**: Step Over's temporary breakpoint marks `ignore_current_line`;
   if source line equals current line on hit → Ignore, continue.

### 3. Variable Inspection and Scopes

```
VS Code request: "Variable list for current frame"
    │
DAP server:
    ├── Query IR node at current pause point
    ├── Iterate variable bindings within current ScopeBoundary
    │   └── Each binding returns: (name, type, runtime value reference)
    └── Assemble VariablesResponse → VS Code
```

**Scope layering**:

```
┌─ Globals ───────────────────────────┐
│  Module-level bindings: constants, type aliases, globals   │
├─ Locals ────────────────────────────┤
│  Local variables visible in current function             │
│  ├── Parameters (function arguments)                       │
│  └── Local bindings (let / assignment)                   │
├─ Captured ──────────────────────────┤
│  Variables captured by spawn blocks / closures            │
│  Display ownership state: moved / shared ref              │
└─────────────────────────────────────┘
```

**Engine differences**:

| Engine      | Variable value retrieval                                                                                             |
| ----------- | -------------------------------------------------------------------------------------------------------------------- |
| Interpreter | Directly read VM stack frames and heap. Each value has a clear in-memory representation                              |
| JIT         | Values in registers and on stack → Need "variable → register/stack slot" mapping table generated at JIT compile time |
| LLVM        | DWARF `.debug_info` section → `DW_AT_location` → LLDB native support                                                 |

**Special type display**: Compile-time predicate refined type display provides valuable debugging
information:

```
x: Positive(x)  →  Display "Int (x > 0 = True)"
y: Sorted(y)    →  Display "Array(Int) (sorting guaranteed)"
result: T       →  Display concrete runtime type
```

### 4. Call Stack

```
DAP request: StackTrace
    │
Return:
┌──────────────────────────────────┐
│ #0  process_item()  file.yx:42  │  ← Current pause point
│     locals: item = "hello"       │
│     spawn task ID: task-3        │
├──────────────────────────────────┤
│ #1  main()          file.yx:67  │  ← caller
│     locals: data = ["hello", ...]│
├──────────────────────────────────┤
│ #2  <entry>         file.yx:1   │  ← root
└──────────────────────────────────┘
```

Each frame records: function signature, call location (source file + line), local variables (lazy
evaluation), spawn context (task ID).

Interpreter and JIT each maintain frame linked lists. Frames are not zero-cost to obtain—but debug
mode doesn't pursue zero overhead.

### 5. Expression Evaluation (Watch / REPL)

User inputs arbitrary YaoXiang expression at a breakpoint:

```
Watch: x + y         → Returns computed result
Watch: items[2].name → Access complex structure
Watch: f(x)          → Call function (side effect risk)
```

**Evaluation strategy**:

```
User inputs expression
    │
├── Compiler frontend parses expression
├── Type checking in current frame context
├── Variable values obtained from current frame (read-only references)
├── Expression executed as independent micro-program
│   └── No modification of external variables
│   └── No spawn allowed
│   └── No IO (or optionally enabled)
└── Return result value → Original frame state completely unchanged
```

**Interpreter as natural sandbox**: Expression evaluation is not a new sandbox—the interpreter
itself is the sandbox. Expression evaluation simply temporarily pushes a frame, which is destroyed
after use. Shares the same VM with normal execution but commits no side effects.

**Function call evaluation**: Allowed by default, but warn user "this expression may have side
effects" and require user confirmation before execution.

**Engine differences**:

| Engine      | Expression evaluation                                                                     |
| ----------- | ----------------------------------------------------------------------------------------- |
| Interpreter | Reuse existing eval code path, inject current frame environment                           |
| JIT         | Temporarily compile expression → Link to current frame → Execute → Discard temporary code |
| LLVM        | Not supported—LLVM mode does not do interactive debugging                                 |

### 6. Concurrency Debugging

**Task model visibility**:

DAP's `threads` concept maps to YaoXiang's `spawn` tasks. Each task has its own stack frame linked
list and running state.

```
┌─ Threads ───────────────────────────┐
│  ● task-1  main()     file.yx:10   │ ← Currently focused
│  ▶ task-2  fetch()    file.yx:34   │ ← Running
│  ⏸ task-3  process()  file.yx:56   │ ← Breakpoint paused
│  ◼ task-4  write()    finished      │
└─────────────────────────────────────┘
```

**Breakpoints in concurrency context**:

| Pause mode           | Behavior                             | Use case                           |
| -------------------- | ------------------------------------ | ---------------------------------- |
| `stop-all` (default) | One task hits → all tasks pause      | Debugging data races, global state |
| `stop-this-only`     | Only pause hit task, others continue | Debugging independent task logic   |

**Stepping semantics in spawn blocks**:

```
spawn {          // Step Over → Run entire spawn block
    task_a()     // Step Into → Enter task_a
    task_b()     // Runs in parallel, not affected by individual step
}
```

### 7. DAP Protocol Mapping

#### Phase 1: Core Requests

| DAP Request         | YaoXiang semantics                                                     |
| ------------------- | ---------------------------------------------------------------------- |
| `initialize`        | Capability negotiation: breakpoints, stepping, variables, stack frames |
| `launch` / `attach` | Start/attach to YaoXiang program (`--debug` uses attach mode)          |
| `setBreakpoints`    | Set source line breakpoints                                            |
| `configurationDone` | Breakpoints ready, begin execution                                     |
| `threads`           | Return all active spawn task list                                      |
| `stackTrace`        | Return stack frame list for specified task                             |
| `scopes`            | Return variable scopes for current frame                               |
| `variables`         | Return variable list for specified scope                               |
| `continue`          | Resume execution                                                       |
| `next`              | Step Over                                                              |
| `stepIn`            | Step Into                                                              |
| `stepOut`           | Step Out                                                               |
| `pause`             | Interrupt all tasks                                                    |
| `evaluate`          | Evaluate expression in current frame                                   |
| `disconnect`        | End debug session                                                      |

#### Phase 2: Enhanced Requests

| DAP Request               | YaoXiang semantics                                    |
| ------------------------- | ----------------------------------------------------- |
| `setFunctionBreakpoints`  | Function name breakpoints                             |
| `setExceptionBreakpoints` | Pause on error/panic                                  |
| `dataBreakpointInfo`      | Data breakpoints (triggered by variable modification) |

## Implementation Strategy

### Phase Zero: Infrastructure (Precedes all phases)

**Goal**: Compiler frontend attaches debug metadata to IR.

| Component     | Changes                                                                |
| ------------- | ---------------------------------------------------------------------- |
| IR definition | Add new metadata fields: `SourceLocation`, `VarName`, `TypeAnnotation` |
| Parser        | Each AST node records source location                                  |
| TypeChecker   | Type information attached to IR nodes                                  |
| Testing       | Verify IR dump includes location and variable information              |

**No runtime changes.**

**Landing progress (2026-09-17)**:

| Deliverable       | Status    | Landing form                                                                                                                                                                                                                                                                      |
| ----------------- | --------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source location   | Completed | All 76 `Instruction` variants carry `span` field; `span()` method deliberately has no wildcard arm, new variants missing span fail compilation. Location coverage 40/41 instructions                                                                                              |
| Variable names    | Completed | `LocalSlot { name, ty, scope_depth }` attached to `FunctionBody::Code::locals`; `register_local` writes at generation time. `.42` debug section v2 carries names, v1 artifacts backward-compatible for reading                                                                    |
| Global slot names | Completed | `.42` debug section v3 carries "slot number → top-level binding name" table. Top-level bindings use `Operand::Global`, not in any function's local name table; without this table, only numeric values can be reported, not variable names. v1/v2 artifacts read with empty table |
| Type information  | Partial   | Slots include `ty`; `TypeAnnotation` as independent metadata not done                                                                                                                                                                                                             |
| Dump visibility   | Completed | `dump` outputs `; <file>:<line>:<col>` per instruction, and lists `locals: name@slot`                                                                                                                                                                                             |

Data shape landed above directly interfaces with Phase 1: DAP's breakpoint resolution consumes
`debug_map`, variable panel consumes `local_names` (names) and `locals` (types).

### Phase 1: Interpreter DAP MVP

**Goal**: `yaoxiang run --debug file.yx` can set breakpoints, step, and inspect variables.

| Component                          | Changes                                                                                                                               |
| ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| DAP server (new module in yx-core) | stdio transport layer, core request handling, breakpoint manager (source line → IR node mapping)                                      |
| Runtime debug trait (yx-core)      | `DebugEngine` trait definition (pause, resume, step, get_frames, eval, get_variables)                                                 |
| Interpreter                        | Breakpoint checking in execution loop, pause/resume mechanism, frame linked list maintenance, `InterpreterDebugEngine` implementation |
| CLI                                | `yaoxiang run --debug` parameter                                                                                                      |

**Acceptance criteria**: For any `.yx` file under `tests/yaoxiang/`, can set breakpoints in VS Code,
Step Over, and view variable values.

### Phase 2: Advanced Debugging Capabilities

**Goal**: Expression evaluation, function breakpoints, concurrency debugging, exception breakpoints.

| Component                      | Changes                                                                                                     |
| ------------------------------ | ----------------------------------------------------------------------------------------------------------- |
| Expression eval engine         | Micro-program compilation (reuse parser + typechecker), temporary frame push into VM, side effect isolation |
| Concurrent debugging           | spawn task list mapping, breakpoint binding to task ID, stop-all / stop-this-only pause strategies          |
| Function/exception breakpoints | `setFunctionBreakpoints`, `setExceptionBreakpoints` mapping                                                 |
| VS Code extension              | Provide default `launch.json` template                                                                      |

### Phase 3: JIT Debugging & LLVM DWARF

**Goal**: JIT engine reuses DAP, LLVM produces DWARF for crash backtraces.

| Component | Changes                                                                                                                                        |
| --------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| JIT       | Implement `DebugEngine` trait, generate variable→register mapping at compile time, runtime frame linked list, expression temporary compilation |
| LLVM      | IR debug metadata → LLVM `DILocation` / `DISubprogram` → DWARF (no DAP interaction)                                                            |

### Dependency Relationships

```
Phase 0 (IR metadata)
    ↓
Phase 1 (Interpreter DAP MVP)  ← Can be used from here
    ↓
Phase 2 (Advanced capabilities)
    ↓
Phase 3 (JIT + LLVM DWARF)
```

### Risks

| Risk                                   | Mitigation                                                                                                       |
| -------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| Interpreter pause mechanism complexity | Use simple channel/signal instead of complex state machine; pausing just means not fetching the next instruction |
| Expression evaluation type safety      | Reuse existing typechecker, read-only references, no side effects committed                                      |
| DAP protocol details                   | Reference debugpy / Delve implementations; the protocol is mature                                                |
| Concurrent debugging stop-all livelock | Timeout mechanism + forced pause                                                                                 |

## Trade-offs

### Pros

- **One source**: Debugging metadata generated once, shared by three engines. No "interpreter debug
  info is correct but LLVM's is wrong"
- **Zero intrusion**: One `--debug` parameter; behavior without this parameter is completely
  unchanged
- **DAP standard**: Directly integrates with VS Code ecosystem, no custom editor protocol or
  debugger UI needed
- **Interpreter first**: Debugging naturally suits interpreters—flexible, controllable, simple
  expression evaluation. LLVM mode not doing interactive debugging is the most pragmatic choice

### Cons

- **Debug mode performance is poor**: Interpreter is much slower than JIT/LLVM. But debugging
  doesn't need performance—no one expects debug mode to run production workloads
- **LLVM debugging limited**: AOT compilation cannot do interactive debugging, only GDB/LLDB +
  DWARF. But this is a trade-off: there shouldn't be debugging behavior differences in LLVM mode
  anyway
- **Concurrent pause complexity**: stop-all semantics require traversing all active tasks on
  interpreter

### Alternative Approaches

| Approach                                        | Why not chosen                                                                                                         |
| ----------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| Each of three engines implements DAP separately | Triple the work, triple the bugs. Violates "good taste"                                                                |
| Use only DWARF, no custom DAP                   | Interpreters and JIT have no DWARF concept; LLDB can't enter VM internals                                              |
| Build command-line debugger like Python pdb     | VS Code experience completely blows away command-line debugger                                                         |
| Stuff DAP into LSP process                      | Lifecycle completely different—LSP follows project, DAP follows debug session. Process isolation is a hard requirement |

## Open Questions

- [ ] Should conditional breakpoint expression syntax be exactly the same as normal YaoXiang?
      (Recommendation: Exactly the same, reuse parser)
- [ ] Behavior of Step Into inside `spawn` blocks: When user presses Step Into to enter a spawn
      block, which of the multiple parallel tasks should be shown? (Recommendation: Pause at the
      first created task)
- [ ] VS Code extension: Should debug configuration go in existing `vscode-extension/` directory or
      a separate repository?

## References

- [RFC-024: Concurrency Model Based on Spawn Blocks](../accepted/024-concurrency-model.md)
- [RFC-027: Compile-time Predicates and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md)
- [RFC-028: JIT Compiler — Multi-level Execution Engine in VM](../draft/028-jit-compiler.md)
- [RFC-030: assert Mechanism](../accepted/030-assert-mechanism.md)
- [DAP Protocol Specification](https://microsoft.github.io/debug-adapter-protocol/)
- [debugpy — Python DAP Implementation Reference](https://github.com/microsoft/debugpy)
- [Delve — Go Debugger Reference](https://github.com/go-delve/delve)
