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
first-class citizens in YaoXiang IR, and the interpreter, JIT, and LLVM backends each consume the
same metadata. Users launch a DAP (Debug Adapter Protocol) server via `yaoxiang run --debug`, VS
Code connects via stdio, and get a unified experience with breakpoints, stepping, variable
inspection, call stacks, expression evaluation, and concurrent debugging—regardless of which
execution engine is underneath.

## Motivation

### Why is this feature needed?

Current methods for troubleshooting YaoXiang program errors are extremely primitive:

```yaoxiang
io.println("DEBUG: x = " + x.to_string())
io.println("DEBUG: entered branch A")
```

Three critical problems:

1. **Compiler bootstrap blocked by self-hosting**: Writing the YaoXiang compiler in YaoXiang, but
   those writing the compiler can't debug their own code. The lack of interactive debugging during
   the bootstrap phase is a dead end.
2. **Three engines, zero debugging**: The interpreter, JIT, and LLVM each run independently; when
   problems occur, users can only check if `ALL TESTS PASSED` appears in stdout. Assertion failure?
   No idea which line, no idea variable values.
3. **Concurrency is a black box**: `spawn` creates multiple tasks—which task crashed? Who moved a
   variable? Pure luck guessing.

### Design Goals

- **Unified experience**: Breakpoints that work on the interpreter work on JIT and LLVM with
  consistent source mapping. Users don't perceive underlying engine differences.
- **One source**: Debugging metadata flows with IR, no duplicate definitions, no maintaining two
  sets of mappings.
- **Zero intrusion**: `yaoxiang run --debug` is one parameter; without it, compilation and execution
  behavior remain completely unchanged.
- **DAP standard**: Directly integrate with VS Code ecosystem, don't reinvent editor protocols.

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
│                 DAP Server (yx-core)                 │
│  ┌─────────┐  ┌──────────┐  ┌───────────────────┐    │
│  │ Session │  │ Breakpoint│  │ Expression        │    │
│  │ Manager │  │ Manager  │  │ Evaluator Engine  │    │
│  └─────────┘  └──────────┘  └───────────────────┘    │
└────────────────────────┬─────────────────────────────┘
                         │ Query/control
┌────────────────────────▼─────────────────────────────┐
│                  Runtime Debug Interface (trait)      │
│  pause / resume / step / get_frames / eval / ...    │
└────┬──────────────────┬──────────────────┬──────────┘
     │                  │                  │
┌────▼────┐    ┌───────▼───────┐    ┌─────▼──────┐
│Interpreter│    │ JIT (RFC-028)│    │ LLVM AOT   │
│ Direct    │    │ Generate     │    │ IR metadata│
│ Consume   │    │ Lightweight  │    │ → DWARF   │
│ IR        │    │ Debug Tables │    │            │
└─────────┘    └───────────────┘    └────────────┘
```

**Key design decisions**:

1. **DAP server and runtime are decoupled via trait**. The server doesn't care if the backend is an
   interpreter or JIT—it only issues commands through the `DebugEngine` trait. Each engine
   independently implements the same trait.
2. **`yaoxiang run --debug` forces use of the interpreter**. Debugging requires controllability, not
   performance. In LLVM mode, only DWARF is generated for post-mortem analysis (core dump / crash
   report), without interactive debugging.
3. **Reuse entry discovery logic from `yaoxiang run`**. No new subcommands introduced; the mental
   model is simply "run my program in debug mode".

### IR Debug Metadata

Attach metadata to existing YaoXiang IR without adding new IR types. All metadata is generated once
at **compiler frontend**, and backend consumption is read-only:

| Metadata         | Attachment Point                   | Description                                       |
| ---------------- | ---------------------------------- | ------------------------------------------------- |
| `SourceLocation` | Every IR node                      | Source file:line:column                           |
| `VarName`        | Variable declaration/binding nodes | Source variable name                              |
| `TypeAnnotation` | Variable/expression nodes          | Inferred type (including compile-time predicates) |
| `ScopeBoundary`  | Block/function entry/exit          | Variable scope lifecycle                          |
| `SpanInfo`       | spawn nodes                        | Task boundaries inside spawn blocks               |

### Startup Flow

```
yaoxiang run --debug file.yx
    │
    ├── Phase 1: Compilation (with debug metadata)
    │   ├── Parse → AST
    │   ├── Type check + compile-time predicate verification
    │   └── Lower to IR (attach debug metadata)
    │
    ├── Phase 2: Use interpreter engine
    │   └── In --debug mode, always use interpreter regardless of --release
    │
    ├── Phase 3: Start DAP server
    │   ├── Initialize stdio transport channel
    │   ├── Wait for VS Code attach
    │   ├── Pause at program entry after attach succeeds
    │   └── Enter interactive debug loop
    │
    └── Phase 4: Program ends / debug session ends → Exit
```

### Mode Differences

| Mode                     | Debugging Method                                                   |
| ------------------------ | ------------------------------------------------------------------ |
| `yaoxiang run --debug`   | Force interpreter, full-featured DAP interactive debugging         |
| `yaoxiang run --release` | Generate DWARF, for post-mortem (core dump / crash report), no DAP |
| `yaoxiang run` (normal)  | No debug metadata, no debugging support                            |

## Detailed Design

### 1. Breakpoints

```
Breakpoint Types:
├── Source line breakpoints    → Frontend generates location metadata, backend queries and matches
├── Function entry breakpoints → Triggered on function call (Phase 2)
├── Conditional breakpoints     → Trigger when expression evaluates to true
└── Data breakpoints           → Trigger when variable is modified (Phase 2)
```

**Source line breakpoint core logic**:

```
VS Code sends: "Set breakpoint at file.yx:42"
    │
DAP server:
    ├── Query all IR nodes with SourceLocation == (file.yx, 42)
    ├── Forward to runtime: "Pause at these IR addresses"
    └── Runtime returns: breakpoint ID

Program reaches IR node → Runtime checks: Is this node in the breakpoint list?
    ├── Regular breakpoint → Pause, notify DAP server
    └── Conditional breakpoint → Evaluate condition → pause only if true
```

**Three engine implementations**:

|                                   | Interpreter                         | JIT                                          | LLVM                                      |
| --------------------------------- | ----------------------------------- | -------------------------------------------- | ----------------------------------------- |
| Breakpoint insertion              | Check IR node IDs in execution loop | JIT inserts `int3` in machine code           | Use LLVM DWARF + hardware breakpoints     |
| Conditional breakpoint evaluation | Directly interpret expression       | Temporarily JIT-compile condition expression | DWARF expression stack + evaluation       |
| Performance overhead              | One extra table lookup per IR node  | Overhead only at breakpoints                 | Near-zero overhead (hardware breakpoints) |

### 2. Stepping

```
Step Over    → Execute current line, skip function call internals, stop at next line
Step Into    → Enter function call on current line
Step Out     → Execute until current function returns
Continue     → Resume execution until next breakpoint or program end
```

**Implementation logic**: Stepping operations are essentially **temporary breakpoints**. They share
the same mechanism as user-set breakpoints—not two systems, but one system with two use cases.

```
Step Over:
    Current source line = query_line(frame)
    → Set temporary breakpoint at next line
    → If current line is a function call: set temporary breakpoint after call site
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

1. **Concurrency ownership**: Temporary breakpoint is bound to current task ID; other tasks hitting
   it are ignored.
2. **Step Over spawn block**: Step Over outside spawn equals running the entire spawn block and
   jumping to after it. Use Step Into to enter spawn internals for debugging.
3. **Temporary breakpoint missed**: Set watchdog timeout (30 seconds with no breakpoint hit) → force
   pause → notify VS Code. Also listen for program exit events → clean up immediately.
4. **Multiple IR nodes on same line**: Step Over's temporary breakpoint marks `ignore_current_line`;
   if hit and source line equals current line → ignore, continue.

### 3. Variable Inspection and Scopes

```
VS Code requests: "Variable list for current frame"
    │
DAP server:
    ├── Query IR node at current pause point
    ├── Traverse variable bindings within current ScopeBoundary
    │   └── Each binding returns: (name, type, runtime value reference)
    └── Assemble VariablesResponse → VS Code
```

**Scope layering**:

```
┌─ Globals ───────────────────────────┐
│  Module-level bindings: constants, type aliases, globals │
├─ Locals ────────────────────────────┤
│  Local variables visible in current function           │
│  ├── Parameters (function arguments)                  │
│  └── Local bindings (let / assignment)                │
├─ Captured ──────────────────────────┤
│  External variables captured by spawn blocks / closures│
│  Display ownership status: moved / shared ref         │
└─────────────────────────────────────┘
```

**Engine differences**:

| Engine      | Variable value retrieval                                                                                |
| ----------- | ------------------------------------------------------------------------------------------------------- |
| Interpreter | Directly read VM stack frames and heap. Each value has clear in-memory representation                   |
| JIT         | Values in registers and on stack → need JIT compile-time "variable → register/stack slot" mapping table |
| LLVM        | DWARF's `.debug_info` section → `DW_AT_location` → native LLDB support                                  |

**Special type display**: Compile-time predicate refined type display shows valuable debugging
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
Returns:
┌──────────────────────────────────┐
│ #0  process_item()  file.yx:42  │  ← Current pause point
│     locals: item = "hello"       │
│     spawn task ID: task-3        │
├──────────────────────────────────┤
│ #1  main()          file.yx:67  │  ← Caller
│     locals: data = ["hello", ...]│
├──────────────────────────────────┤
│ #2  <entry>         file.yx:1   │  ← Root
└──────────────────────────────────┘
```

Each frame records: function signature, call location (source file + line), local variables (lazy
evaluation), spawn context (task ID).

Interpreters and JIT each maintain frame linked lists. Frames are not zero-cost to obtain—but
debugging mode doesn't pursue zero overhead.

### 5. Expression Evaluation (Watch / REPL)

User inputs any YaoXiang expression at a breakpoint:

```
Watch: x + y         → Returns computed result
Watch: items[2].name → Access complex structures
Watch: f(x)          → Call function (side effect risk)
```

**Evaluation strategy**:

```
User inputs expression
    │
├── Frontend parses expression
├── Type check in current frame context
├── Variable values obtained from current frame (read-only reference)
├── Expression executed as independent micro-program
│   └── No external variable modification allowed
│   └── No spawn allowed
│   └── No IO (or optionally enabled)
└── Return result value → Original frame state completely unchanged
```

**Interpreter's natural sandbox**: Expression evaluation doesn't create a new sandbox—the
interpreter itself is the sandbox. Expression evaluation just temporarily pushes a frame, destroys
it after use. Executes in the same VM as normal execution, but doesn't commit any side effects.

**Function call evaluation**: Allowed by default, but warn user "this expression may have side
effects" and require user confirmation before execution.

**Engine differences**:

| Engine      | Expression evaluation                                                                |
| ----------- | ------------------------------------------------------------------------------------ |
| Interpreter | Reuse existing eval code path, inject current frame environment                      |
| JIT         | Temporarily compile expression → link to current frame → execute → discard temp code |
| LLVM        | Not supported—LLVM mode doesn't do interactive debugging                             |

### 6. Concurrent Debugging

**Task model visibility**:

DAP's `threads` concept maps to YaoXiang's `spawn` tasks. Each task has its own frame linked list
and running state.

```
┌─ Threads ───────────────────────────┐
│  ● task-1  main()     file.yx:10   │ ← Current focus
│  ▶ task-2  fetch()    file.yx:34   │ ← Running
│  ⏸ task-3  process()  file.yx:56   │ ← Paused at breakpoint
│  ◼ task-4  write()    Finished      │
└─────────────────────────────────────┘
```

**Breakpoints in concurrent context**:

| Pause mode           | Behavior                                   | Use case                       |
| -------------------- | ------------------------------------------ | ------------------------------ |
| `stop-all` (default) | One task hits breakpoint → all tasks pause | Debug data races, global state |
| `stop-this-only`     | Only hit task pauses, others continue      | Debug independent task logic   |

**Stepping semantics in spawn blocks**:

```
spawn {          // Step Over → run entire spawn block
    task_a()     // Step Into → enter task_a
    task_b()     // Runs in parallel, not affected by individual step
}
```

### 7. DAP Protocol Mapping

#### Phase 1: Core Requests

| DAP Request         | YaoXiang semantics                                                     |
| ------------------- | ---------------------------------------------------------------------- |
| `initialize`        | Capability negotiation: breakpoints, stepping, variables, stack frames |
| `launch` / `attach` | Launch/attach to YaoXiang program (--debug uses attach mode)           |
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

| DAP Request               | YaoXiang semantics                                |
| ------------------------- | ------------------------------------------------- |
| `setFunctionBreakpoints`  | Function name breakpoints                         |
| `setExceptionBreakpoints` | Pause on error/panic                              |
| `dataBreakpointInfo`      | Data breakpoints (variable modification triggers) |

## Implementation Strategy

### Phase Zero: Infrastructure (precedes all phases)

**Goal**: Compiler frontend attaches debug metadata to IR.

| Component     | Changes                                                                |
| ------------- | ---------------------------------------------------------------------- |
| IR definition | Add new metadata fields: `SourceLocation`, `VarName`, `TypeAnnotation` |
| Parser        | Record source location on every AST node                               |
| TypeChecker   | Attach type information to IR nodes                                    |
| Tests         | Verify IR dump includes location and variable information              |

**No runtime involvement.**

### Phase One: Interpreter DAP MVP

**Goal**: `yaoxiang run --debug file.yx` supports setting breakpoints, stepping, inspecting
variables.

| Component                       | Changes                                                                                                                            |
| ------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| DAP server (yx-core new module) | stdio transport layer, core request handling, breakpoint manager (source line → IR node mapping)                                   |
| Runtime debug trait (yx-core)   | `DebugEngine` trait definition (pause, resume, step, get_frames, eval, get_variables)                                              |
| Interpreter                     | Execution loop breakpoint checking, pause/resume mechanism, frame linked list maintenance, `InterpreterDebugEngine` implementation |
| CLI                             | `yaoxiang run --debug` parameter                                                                                                   |

**Acceptance criteria**: For any `.yx` file under `tests/yaoxiang/`, able to set breakpoints, Step
Over, and inspect variable values using VS Code.

### Phase Two: Advanced Debugging Capabilities

**Goal**: Expression evaluation, function breakpoints, concurrent debugging, exception breakpoints.

| Component                      | Changes                                                                                                     |
| ------------------------------ | ----------------------------------------------------------------------------------------------------------- |
| Expression evaluator           | Micro-program compilation (reuse parser + typechecker), temporary frame push into VM, side effect isolation |
| Concurrent debugging           | spawn task list mapping, breakpoint task ID binding, stop-all / stop-this-only pause strategies             |
| Function/exception breakpoints | `setFunctionBreakpoints`, `setExceptionBreakpoints` mapping                                                 |
| VS Code extension              | Provide default `launch.json` template                                                                      |

### Phase Three: JIT Debugging & LLVM DWARF

**Goal**: JIT engine reuses DAP, LLVM produces DWARF for crash post-mortem.

| Component | Changes                                                                                                                               |
| --------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| JIT       | Implement `DebugEngine` trait, compile-time variable → register mapping table, runtime frame linked list, expression temp compilation |
| LLVM      | IR debug metadata → LLVM `DILocation` / `DISubprogram` → DWARF (no DAP interaction)                                                   |

### Dependencies

```
Phase Zero (IR metadata)
    ↓
Phase One (Interpreter DAP MVP)  ← From here it's usable
    ↓
Phase Two (Advanced capabilities)
    ↓
Phase Three (JIT + LLVM DWARF)
```

### Risks

| Risk                                   | Mitigation                                                                                           |
| -------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| Interpreter pause mechanism complexity | Use simple channel/signal instead of complex state machine; pause means don't fetch next instruction |
| Expression evaluation type safety      | Reuse existing typechecker, read-only references, don't commit side effects                          |
| DAP protocol details                   | Reference debugpy / Delve implementations; protocol is mature                                        |
| Concurrent debugging stop-all livelock | Timeout mechanism + forced pause                                                                     |

## Trade-offs

### Advantages

- **One source**: Debugging metadata generated once, shared by three engines. No "interpreter debug
  info is correct but LLVM's is wrong"
- **Zero intrusion**: `--debug` is one parameter; behavior without it is completely unchanged
- **DAP standard**: Directly integrate with VS Code ecosystem; no custom editor protocol or debugger
  UI needed
- **Interpreter-first**: Debugging naturally suits interpreters—flexible, controllable, simple
  expression evaluation. LLVM mode not doing interactive debugging is the most pragmatic choice

### Disadvantages

- **Debug mode performance is poor**: Interpreter is much slower than JIT/LLVM. But debugging
  doesn't need performance—no one expects debug mode to run production workloads
- **LLVM debugging limited**: AOT compilation cannot do interactive debugging, only GDB/LLDB +
  DWARF. But this is a trade-off: LLVM mode shouldn't have debugging behavior differences anyway
- **Concurrent pause is complex**: stop-all semantics require traversing all active tasks on the
  interpreter

### Alternatives

| Alternative                           | Why not chosen                                                                                                          |
| ------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| Each engine implements DAP separately | Triple the work, triple the bugs. Violates "good taste"                                                                 |
| Only use DWARF, no custom DAP         | Interpreters and JIT don't have DWARF concepts; LLDB can't reach inside VM                                              |
| Command-line debugger like Python pdb | VS Code experience completely dominates CLI debuggers                                                                   |
| Put DAP inside LSP process            | Completely different lifecycles—LSP follows project, DAP follows debug session. Process isolation is a hard requirement |

## Open Questions

- [ ] Should conditional breakpoint expression syntax be exactly the same as normal YaoXiang?
      (Suggestion: yes, exactly the same, reuse parser)
- [ ] Step Into behavior inside `spawn` blocks: when user presses Step Into to enter a spawn block,
      which of the parallel tasks should be shown? (Suggestion: pause on the first created task)
- [ ] VS Code extension: should debug configuration be in existing `vscode-extension/` directory or
      a separate repo?

## References

- [RFC-024: Concurrency Model Based on Spawn Blocks](../accepted/024-concurrency-model.md)
- [RFC-027: Compile-Time Predicates and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md)
- [RFC-028: JIT Compiler — Multi-Level Execution Engine in VM](../draft/028-jit-compiler.md)
- [RFC-030: assert Mechanism](../review/030-assert-mechanism.md)
- [DAP Protocol Specification](https://microsoft.github.io/debug-adapter-protocol/)
- [debugpy — Python DAP Implementation Reference](https://github.com/microsoft/debugpy)
- [Delve — Go Debugger Reference](https://github.com/go-delve/delve)
