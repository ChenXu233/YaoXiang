---
title: 'RFC-034: Unified Debugging Toolchain'
status: 'Draft'
author: 'Chenxu'
created: '2026-07-06'
updated: '2026-07-06'
issue: '#164'
---

# RFC-034: Unified Debugging Toolchain

## Summary

Introduce a unified debugging toolchain for YaoXiang. The core design is **one source, three
consumers**: the compilation frontend embeds source locations, variable names, and type information
as first-class citizens in YaoXiang IR, and the interpreter, JIT, and LLVM backends each consume the
same set of metadata. Users launch a DAP (Debug Adapter Protocol) server via `yaoxiang run --debug`,
and VS Code connects through stdio to obtain a unified experience of breakpoints, stepping, variable
viewing, call stacks, expression evaluation, and concurrent debugging—regardless of the underlying
execution engine.

## Motivation

### Why is this feature needed?

The current means of troubleshooting YaoXiang programs are extremely primitive:

```yaoxiang
io.println("DEBUG: x = " + x.to_string())
io.println("DEBUG: entered branch A")
```

Three fatal problems:

1. **Compiler development self-hosting is blocked**: YaoXiang is used to write the YaoXiang
   compiler, but the compiler writer cannot debug their own code. The lack of interactive debugging
   during the self-hosting stage is a dead end.
2. **Three engines, zero debugging**: The interpreter, JIT, and LLVM each run their own way. When
   something goes wrong, users can only check whether `ALL TESTS PASSED` appears in stdout.
   Assertion failure? Don't know which line, don't know the variable values.
3. **Concurrency is a black box**: `spawn` creates multiple tasks; which task hung? Who moved the
   variable? It's all guesswork.

### Design Goals

- **Unified experience**: Breakpoints that work in the interpreter also work in JIT, and LLVM has
  consistent source mapping. Users don't perceive differences between underlying engines.
- **One source**: Debug metadata flows with the IR; no duplicate definitions, no maintaining two
  sets of mappings.
- **Zero intrusion**: A single `--debug` parameter to `yaoxiang run`; when this parameter is not
  added, compilation and execution behavior is completely unchanged.
- **DAP standard**: Directly integrates with the VS Code ecosystem; no reinventing the editor
  protocol.

## Proposal

### Core Design

Architecture overview:

```
┌──────────────────────────────────────────────────────┐
│                    VS Code / Editor                    │
│              DAP Client (launch.json)                 │
└────────────────────────┬─────────────────────────────┘
                         │ stdio
┌────────────────────────▼─────────────────────────────┐
│                 DAP Server (yx-core)                   │
│  ┌─────────┐  ┌──────────┐  ┌───────────────────┐    │
│  │ Session │  │Breakpoint│  │   Expression      │    │
│  │ Manager │  │ Manager  │  │ Evaluation Engine │    │
│  └─────────┘  └──────────┘  └───────────────────┘    │
└────────────────────────┬─────────────────────────────┘
                         │ Query / Control
┌────────────────────────▼─────────────────────────────┐
│              Runtime Debug Interface (trait)           │
│  pause / resume / step / get_frames / eval / ...    │
└────┬──────────────────┬──────────────────┬──────────┘
     │                  │                  │
┌────▼────┐    ┌───────▼───────┐    ┌─────▼──────┐
│Interpreter│  │ JIT (RFC-028) │    │  LLVM AOT   │
│ Direct   │  │ Lightweight   │    │ IR metadata │
│ IR meta  │  │ debug tables  │    │ → DWARF     │
└─────────┘    └───────────────┘    └─────────────┘
```

**Key design decisions**:

1. **The DAP server and runtime are decoupled through a trait**. The server doesn't care whether the
   underlying is an interpreter or JIT—it only issues commands through the `DebugEngine` trait. Each
   engine independently implements the same trait.
2. **`yaoxiang run --debug` forces the use of the interpreter**. Debugging requires controllability,
   not performance. In LLVM mode, only DWARF is generated for post-mortem backtraces (core dump /
   crash report), without interactive debugging.
3. **Reuse entry discovery logic from `yaoxiang run`**. No new subcommands are introduced; the
   mental model is simply "run my program in debug mode."

### IR Debug Metadata

Attach metadata to the existing YaoXiang IR without adding new IR kinds. All metadata is generated
in **one place—the compilation frontend**—and backend consumption is read-only:

| Metadata         | Attachment Point                    | Description                                       |
| ---------------- | ----------------------------------- | ------------------------------------------------- |
| `SourceLocation` | Each IR node                        | source file:line:column                           |
| `VarName`        | Variable declaration / binding node | Variable name in source code                      |
| `TypeAnnotation` | Variable / expression node          | Inferred type (including compile-time predicates) |
| `ScopeBoundary`  | Block / function entry / exit       | Lifecycle of variable scopes                      |
| `SpanInfo`       | spawn node                          | Task boundary within the spawn block              |

### Startup Flow

```
yaoxiang run --debug file.yx
    │
    ├── Phase 1: Compile (with debug metadata)
    │   ├── Parse → AST
    │   ├── Type check + compile-time predicate verification
    │   └── Lower to IR (attach debug metadata)
    │
    ├── Phase 2: Use interpreter engine
    │   └── In --debug mode, interpreter is used regardless of --release
    │
    ├── Phase 3: Launch DAP server
    │   ├── Initialize stdio transport channel
    │   ├── Wait for VS Code to attach
    │   ├── Pause at program entry after attach succeeds
    │   └── Enter interactive debug loop
    │
    └── Phase 4: Program ends / debug session ends → exit
```

### Differences Between Modes

| Mode                     | Debug Method                                                                          |
| ------------------------ | ------------------------------------------------------------------------------------- |
| `yaoxiang run --debug`   | Forced interpreter, full-featured DAP interactive debugging                           |
| `yaoxiang run --release` | Generate DWARF for post-mortem backtraces (core dump / crash report); DAP not started |
| `yaoxiang run` (normal)  | No debug metadata, no debugging support                                               |

## Detailed Design

### 1. Breakpoints

```
Breakpoint types:
├── Source line breakpoint    → Compilation frontend generates location metadata, backend queries for matches
├── Function entry breakpoint  → Triggered when the function is called (Phase 2)
├── Conditional breakpoint    → Triggers when the expression evaluates to true
└── Data breakpoint            → Triggered when a variable is modified (Phase 2)
```

**Core logic for source line breakpoints**:

```
VS Code sends: "Set breakpoint at file.yx:42"
    │
DAP server:
    ├── Query all IR nodes with SourceLocation == (file.yx, 42)
    ├── Forward to runtime: "Pause at these IR addresses"
    └── Runtime returns: breakpoint ID

Program runs to IR node → Runtime checks: is this node in the breakpoint list?
    ├── Regular breakpoint → pause, notify DAP server
    └── Conditional breakpoint → evaluate condition expression → pause only if true
```

**Three engine implementations**:

|                             | Interpreter                            | JIT                                   | LLVM                                      |
| --------------------------- | -------------------------------------- | ------------------------------------- | ----------------------------------------- |
| Breakpoint insertion        | Check IR node ID in the execution loop | JIT inserts `int3` into machine code  | LLVM DWARF + hardware breakpoints         |
| Conditional breakpoint eval | Directly interpret the expression      | Temporarily JIT compile the condition | DWARF expression stack + evaluation       |
| Performance overhead        | One extra table lookup per IR node     | Overhead only at breakpoints          | Near-zero overhead (hardware breakpoints) |

### 2. Stepping

```
Step Over    → Execute current line, skip inside function calls, stop at next line
Step Into    → Enter the function call on the current line
Step Out     → Execute until the current function returns
Continue     → Resume execution until the next breakpoint or program end
```

**Implementation logic**: Stepping operations are essentially **temporary breakpoints**. They share
the same mechanism as user-explicit breakpoints—it's not two systems, it's two uses of one system.

```
Step Over:
    Current source line number = query_line(frame)
    → Set a temporary breakpoint at the next line
    → If the current line is a function call: set a temporary breakpoint after the call point
    → Continue → hit the temporary breakpoint → delete → pause

Step Into:
    First executable line of the current call target
    → Find the source location of the first IR node in the function body
    → Set a temporary breakpoint → Continue → hit → pause

Step Out:
    Return address of the current stack frame
    → Find the next line of the caller
    → Set a temporary breakpoint → Continue → hit → pause
```

**Edge case handling for temporary breakpoints**:

1. **Concurrency ownership**: Temporary breakpoints are bound to the current task ID; other tasks
   ignore them when hit.
2. **Step Over on a spawn block**: Step Over outside a `spawn` means running through the entire
   spawn block, then jumping to after it. To debug inside the spawn, use Step Into.
3. **Temporary breakpoint not hit**: Set a watchdog timeout (30 seconds without any breakpoint hit)
   → force pause → notify VS Code. Also listen for program-exit events → clean up immediately.
4. **Multiple IR nodes on the same line**: The temporary breakpoint for Step Over is marked
   `ignore_current_line`; if the source line number equals the current line number when hit → ignore
   and continue.

### 3. Variable Inspection and Scopes

```
VS Code requests: "Variable list for current frame"
    │
DAP server:
    ├── Query the IR node at the current pause point
    ├── Iterate variable bindings within the current ScopeBoundary
    │   └── Each binding returns: (name, type, runtime value reference)
    └── Assemble VariablesResponse → VS Code
```

**Scope hierarchy**:

```
┌─ Globals ───────────────────────────┐
│  Module-level bindings: constants, type aliases, globals  │
├─ Locals ────────────────────────────┤
│  Local variables visible in the current function │
│  ├── Parameters (function parameters)        │
│  └── Local bindings (let / assignment)       │
├─ Captured ──────────────────────────┤
│  Variables captured by spawn block / closure from outside │
│  Display ownership state: moved / ref shared  │
└─────────────────────────────────────┘
```

**Engine differences**:

| Engine      | Variable Value Retrieval                                                                                           |
| ----------- | ------------------------------------------------------------------------------------------------------------------ |
| Interpreter | Directly read VM stack frame and heap. Every value has a clear representation in memory                            |
| JIT         | Values in registers and on the stack → need to record a "variable → register / stack slot" map at JIT compile time |
| LLVM        | DWARF `.debug_info` section → `DW_AT_location` → natively supported by LLDB                                        |

**Special type display**: Compile-time predicates refine the type display to show valuable debugging
information:

```
x: Positive(x)  →  Display "Int (x > 0 = True)"
y: Sorted(y)    →  Display "Array(Int) (sorted guarantee)"
result: T       →  Display the concrete runtime type
```

### 4. Call Stack

```
DAP request: StackTrace
    │
Returns:
┌──────────────────────────────────┐
│ #0  process_item()  file.yx:42  │  ← Current pause point
│     locals: item = "hello"      │
│     spawn task ID: task-3       │
├──────────────────────────────────┤
│ #1  main()          file.yx:67  │  ← Caller
│     locals: data = ["hello", ...]│
├──────────────────────────────────┤
│ #2  <entry>         file.yx:1   │  ← Root
└──────────────────────────────────┘
```

Each frame records: function signature, call location (source file + line number), local variables
(lazy evaluation), and spawn context (task ID).

The interpreter and JIT each maintain their own frame linked list. Frames are not zero-cost to
retrieve—but debug mode does not pursue zero overhead.

### 5. Expression Evaluation (Watch / REPL)

The user enters any YaoXiang expression at a breakpoint:

```
Watch: x + y         → Returns the computed result
Watch: items[2].name → Accesses complex structures
Watch: f(x)          → Calls a function (risk of side effects)
```

**Evaluation strategy**:

```
User enters expression
    │
    ├── Compilation frontend parses the expression
    ├── Type check in the current frame's context
    ├── Variable values fetched from the current frame (read-only references)
    ├── Expression executed as a standalone micro-program
    │   └── Modifying external variables is not allowed
    │   └── spawn is not allowed
    │   └── IO is not allowed (optionally enabled)
    └── Return the result value → original frame state completely unchanged
```

**The interpreter is a natural sandbox**: Expression evaluation does not create a new sandbox—the
interpreter itself is a sandbox. Expression evaluation simply temporarily pushes a frame, and
destroys it when done. It shares the same VM as normal execution, but commits no side effects.

**Function call evaluation**: Allowed by default, but warns the user "this expression may have side
effects" and requires user confirmation before execution.

**Engine differences**:

| Engine      | Expression Evaluation                                                                         |
| ----------- | --------------------------------------------------------------------------------------------- |
| Interpreter | Reuse the existing eval code path; inject the current frame's environment                     |
| JIT         | Temporarily compile expression → link to the current frame → execute → discard temporary code |
| LLVM        | Not supported—LLVM mode does not do interactive debugging                                     |

### 6. Concurrent Debugging

**Task model visibility**:

DAP's `threads` concept maps to YaoXiang's `spawn` tasks. Each task has its own stack frame linked
list and running state.

```
┌─ Threads ───────────────────────────┐
│  ● task-1  main()     file.yx:10   │ ← Current focus
│  ▶ task-2  fetch()    file.yx:34   │ ← Running
│  ⏸ task-3  process()  file.yx:56   │ ← Paused at breakpoint
│  ◼ task-4  write()    ended         │
└─────────────────────────────────────┘
```

**Breakpoints in concurrent context**:

| Pause Mode           | Behavior                                     | Applicable Scenario            |
| -------------------- | -------------------------------------------- | ------------------------------ |
| `stop-all` (default) | One task hits → all tasks pause              | Debug data races, global state |
| `stop-this-only`     | Only pause the hitting task; others continue | Debug independent task logic   |

**Stepping semantics for a spawn block**:

```
spawn {          // Step Over → run through the entire spawn block
    task_a()     // Step Into → enter task_a
    task_b()     // Runs in parallel, not affected by individual step
}
```

### 7. DAP Protocol Mapping

#### Phase 1: Core Requests

| DAP Request         | YaoXiang Semantics                                                             |
| ------------------- | ------------------------------------------------------------------------------ |
| `initialize`        | Capability negotiation: support breakpoints, stepping, variables, stack frames |
| `launch` / `attach` | Launch / attach to a YaoXiang program (`--debug` goes through attach mode)     |
| `setBreakpoints`    | Set source line breakpoints                                                    |
| `configurationDone` | Breakpoints ready, start execution                                             |
| `threads`           | Return list of all active spawn tasks                                          |
| `stackTrace`        | Return stack frame list for the specified task                                 |
| `scopes`            | Return variable scopes for the current frame                                   |
| `variables`         | Return variable list for the specified scope                                   |
| `continue`          | Resume execution                                                               |
| `next`              | Step Over                                                                      |
| `stepIn`            | Step Into                                                                      |
| `stepOut`           | Step Out                                                                       |
| `pause`             | Interrupt all tasks                                                            |
| `evaluate`          | Evaluate expression in the current frame                                       |
| `disconnect`        | End debug session                                                              |

#### Phase 2: Enhanced Requests

| DAP Request               | YaoXiang Semantics                                      |
| ------------------------- | ------------------------------------------------------- |
| `setFunctionBreakpoints`  | Function name breakpoint                                |
| `setExceptionBreakpoints` | Pause on error / panic                                  |
| `dataBreakpointInfo`      | Data breakpoint (triggered when a variable is modified) |

## Implementation Strategy

### Phase Zero: Infrastructure (prerequisite to all phases)

**Goal**: The compilation frontend attaches debug metadata to the IR.

| Component     | Changes                                                                     |
| ------------- | --------------------------------------------------------------------------- |
| IR Definition | Add `SourceLocation`, `VarName`, `TypeAnnotation` and other metadata fields |
| Parser        | Each AST node records its source position                                   |
| TypeChecker   | Type information attached to IR nodes                                       |
| Tests         | Verify the IR dump contains position and variable information               |

**Does not involve the runtime.**

**Landing progress (2026-09-17)**:

| Deliverable       | Status    | Landing Form                                                                                                                                                                                                                                                                                                                       |
| ----------------- | --------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source location   | Completed | All 76 `Instruction` variants carry a `span` field; the `span()` method deliberately has no wildcard arm—adding a new variant without `span` fails to compile. Location coverage: 40 / 41 instructions.                                                                                                                            |
| Variable names    | Completed | `LocalSlot { name, ty, scope_depth }` attached to `FunctionBody::Code::locals`; `register_local` writes in place during generation. The `.42` debug section v2 carries names; v1 artifacts are read backwards compatibly.                                                                                                          |
| Global slot names | Completed | The `.42` debug section v3 carries a "slot number → top-level binding name" table. Top-level bindings go through `Operand::Global` and are not in any function's local name table; without this table only the numeric value can be reported, not the variable name. v1 / v2 artifacts are backfilled with an empty table on read. |
| Type information  | Partial   | Slots already carry `ty`; standalone `TypeAnnotation` metadata not yet done.                                                                                                                                                                                                                                                       |
| Dump visibility   | Completed | `dump` outputs `; <file>:<line>:<col>` for each instruction, and lists `locals: name@slot`.                                                                                                                                                                                                                                        |

The data form landed in the table above interfaces directly with Phase One: DAP breakpoint
resolution consumes `debug_map`; the variable panel consumes `local_names` (names) and `locals`
(types).

### Phase 1: Interpreter DAP MVP

**Goal**: `yaoxiang run --debug file.yx` can set breakpoints, step, and view variables.

| Component                       | Changes                                                                                                                                  |
| ------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| DAP Server (new yx-core module) | stdio transport layer, core request handling, breakpoint manager (source line → IR node mapping)                                         |
| Runtime debug trait (yx-core)   | `DebugEngine` trait definition (pause, resume, step, get_frames, eval, get_variables)                                                    |
| Interpreter                     | Breakpoint check in the execution loop, pause / resume mechanism, frame linked list maintenance, `InterpreterDebugEngine` implementation |
| CLI                             | `yaoxiang run --debug` parameter                                                                                                         |

**Acceptance criteria**: For any `.yx` file under `tests/yaoxiang/`, it is possible to set
breakpoints, Step Over, and view variable values in VS Code.

### Phase 2: Advanced Debugging Capabilities

**Goal**: Expression evaluation, function breakpoints, concurrent debugging, exception breakpoints.

| Component                        | Changes                                                                                                         |
| -------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| Expression evaluation engine     | Micro-program compilation (reuse parser + typechecker), temporary frame push into the VM, side-effect isolation |
| Concurrent debugging             | spawn task list mapping, breakpoints bound to task ID, `stop-all` / `stop-this-only` pause strategy             |
| Function / exception breakpoints | Mapping of `setFunctionBreakpoints`, `setExceptionBreakpoints`                                                  |
| VS Code extension                | Provide a default `launch.json` template                                                                        |

### Phase 3: JIT Debugging & LLVM DWARF

**Goal**: The JIT engine reuses DAP, and LLVM produces DWARF for crash backtraces.

| Component | Changes                                                                                                                                                        |
| --------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| JIT       | Implement the `DebugEngine` trait, generate the variable → register mapping table at compile time, runtime frame linked list, temporary expression compilation |
| LLVM      | IR debug metadata → LLVM `DILocation` / `DISubprogram` → DWARF (no DAP interaction)                                                                            |

### Dependencies

```
Phase Zero (IR metadata)
    ↓
Phase One (Interpreter DAP MVP)  ← Usable from here onward
    ↓
Phase Two (Advanced capabilities)
    ↓
Phase Three (JIT + LLVM DWARF)
```

### Risks

| Risk                                            | Mitigation                                                                                                                |
| ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| Complexity of the interpreter's pause mechanism | Use a simple channel / signal rather than a complex state machine; pausing simply means not fetching the next instruction |
| Type safety of expression evaluation            | Reuse the existing typechecker, read-only references, commit no side effects                                              |
| DAP protocol details                            | Reference debugpy / delve implementations; the protocol is mature                                                         |
| `stop-all` livelock in concurrent debugging     | Timeout mechanism + forced pause                                                                                          |

## Trade-offs

### Advantages

- **One source**: Debug metadata is generated only once, and shared by the three engines. There is
  no "interpreter debug info is correct but LLVM's is wrong" scenario.
- **Zero intrusion**: A single `--debug` parameter; behavior is completely unchanged when it is not
  added.
- **DAP standard**: Drops directly into the VS Code ecosystem, no custom editor protocol or debugger
  UI needed.
- **Interpreter first**: Debugging is naturally suited to the interpreter—flexible, controllable,
  simple expression evaluation. The choice to not do interactive debugging in LLVM mode is the most
  pragmatic.

### Disadvantages

- **Poor debug-mode performance**: The interpreter is much slower than JIT / LLVM. But debugging
  does not need performance—no one expects debug mode to run production loads.
- **Limited LLVM debugging**: AOT compilation cannot do interactive debugging, only GDB / LLDB +
  DWARF. But this is a trade-off: there should be no behavioral difference in debugging in LLVM
  mode.
- **Complex concurrent pause**: Implementing `stop-all` semantics on the interpreter requires
  iterating over all active tasks.

### Alternatives

| Plan                                             | Why not chosen                                                                                                                  |
| ------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------- |
| Each of the three engines implements DAP         | Triple the work, triple the bugs. Violates "good taste."                                                                        |
| Use only DWARF, no proprietary DAP               | The interpreter and JIT have no DWARF concept; LLDB cannot enter the VM internals                                               |
| Build a Python `pdb`-style command-line debugger | The VS Code experience completely outclasses a command-line debugger                                                            |
| Pack DAP into the LSP process                    | Completely different lifecycles—LSP follows the project, DAP follows the debug session. Process isolation is a hard requirement |

## Open Questions

- [ ] Are conditional breakpoint expressions syntactically identical to ordinary YaoXiang?
      (Suggestion: identical; reuse the parser)
- [ ] `spawn` block Step Into behavior: when the user presses Step Into entering a spawn block,
      which of the parallel tasks should be displayed? (Suggestion: pause on the first task that has
      been created)
- [ ] VS Code extension: should the debug configuration be placed under the existing
      `vscode-extension/` directory, or in a separate repository?

## References

- [RFC-024: Concurrency Model Based on spawn Blocks](../accepted/024-concurrency-model.md)
- [RFC-027: Compile-Time Predicates and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md)
- [RFC-028: JIT Compiler — Multi-Level Execution Engine in VM](../draft/028-jit-compiler.md)
- [RFC-030: assert Assertion Mechanism](../review/030-assert-mechanism.md)
- [DAP Protocol Specification](https://microsoft.github.io/debug-adapter-protocol/)
- [debugpy — Python DAP Implementation Reference](https://github.com/microsoft/debugpy)
- [Delve — Go Debugger Reference](https://github.com/go-delve/delve)
