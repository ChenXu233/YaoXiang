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
as first-class citizens into the YaoXiang IR, and the three backends—interpreter, JIT, and LLVM—each
consume the same set of metadata. Users launch the DAP (Debug Adapter Protocol) server via
`yaoxiang run --debug`, and VS Code connects over stdio, providing a unified experience for
breakpoints, stepping, variable inspection, call stacks, expression evaluation, and concurrency
debugging—regardless of the underlying execution engine.

## Motivation

### Why is this feature needed?

The current means of debugging YaoXiang programs are extremely primitive:

```yaoxiang
io.println("DEBUG: x = " + x.to_string())
io.println("DEBUG: entered branch A")
```

Three fatal problems:

1. **Compiler self-hosting is blocked**: YaoXiang is used to write the YaoXiang compiler, but
   compiler authors cannot debug their own code. The lack of interactive debugging during the
   self-hosting stage is a dead end.
2. **Three engines, zero debugging**: The interpreter, JIT, and LLVM each run independently, and
   when something goes wrong users can only check whether `ALL TESTS PASSED` appears in stdout. An
   assertion failed? No idea what line, no idea what the variable values are.
3. **Concurrency is a black box**: `spawn` creates multiple tasks— which one crashed? Who moved the
   variable? All guesswork.

### Design Goals

- **Unified experience**: A breakpoint that works in the interpreter also works in JIT, and LLVM has
  consistent source mapping. Users don't perceive differences in the underlying engine.
- **One source**: Debug metadata flows with the IR—no duplicate definitions, no maintaining two sets
  of mappings.
- **Zero intrusion**: One parameter, `yaoxiang run --debug`. Without this parameter, compilation and
  execution behavior is completely unchanged.
- **DAP standard**: Direct integration with the VS Code ecosystem—no need to reinvent editor
  protocols.

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
│              DAP Server (yx-core)                     │
│  ┌─────────┐  ┌──────────┐  ┌───────────────────┐    │
│  │ Session │  │ Breakpoint│  │ Expression         │    │
│  │ Manager │  │  Manager  │  │ Evaluation Engine  │    │
│  └─────────┘  └──────────┘  └───────────────────┘    │
└────────────────────────┬─────────────────────────────┘
                         │ Query/Control
┌────────────────────────▼─────────────────────────────┐
│              Runtime Debug Interface (trait)          │
│  pause / resume / step / get_frames / eval / ...    │
└────┬──────────────────┬──────────────────┬──────────┘
     │                  │                  │
┌────▼────┐    ┌───────▼───────┐    ┌─────▼──────┐
│Interpreter│   │ JIT (RFC-028)│    │ LLVM AOT   │
│ Directly │    │ Generate     │    │ IR metadata │
│ Consumes │    │ lightweight  │    │ → DWARF     │
│ IR meta  │    │ debug tables │    │             │
└─────────┘    └───────────────┘    └─────────────┘
```

**Key design decisions**:

1. **DAP server and runtime are decoupled through a trait**. The server doesn't care whether the
   backend is the interpreter or JIT—it only issues commands through the `DebugEngine` trait. Each
   engine independently implements the same trait.
2. **`yaoxiang run --debug` forces the use of the interpreter**. Debugging requires controllability,
   not performance. In LLVM mode, only DWARF is generated for post-mortem backtraces (core dump /
   crash report); no interactive debugging.
3. **Reuse entry discovery logic from `yaoxiang run`**. No new subcommand—the mental model is simply
   "run my program in debug mode."

### IR Debug Metadata

Metadata is attached to the existing YaoXiang IR—no new IR variants are introduced. All metadata is
generated in one place at the **compilation frontend**, and backend consumption is read-only:

| Metadata         | Attachment Point                   | Description                                       |
| ---------------- | ---------------------------------- | ------------------------------------------------- |
| `SourceLocation` | Every IR node                      | source file:line:column                           |
| `VarName`        | Variable declaration/binding nodes | Variable name in source code                      |
| `TypeAnnotation` | Variable/expression nodes          | Inferred type (including compile-time predicates) |
| `ScopeBoundary`  | Block/function entry/exit          | Lifecycle of variable scope                       |
| `SpanInfo`       | spawn nodes                        | Task boundary within spawn block                  |

### Startup Flow

```
yaoxiang run --debug file.yx
    │
    ├── Phase 1: Compile (with debug metadata)
    │   ├── Parse → AST
    │   ├── Type check + compile-time predicate verification
    │   └── Lower to IR (with debug metadata attached)
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
    └── Phase 4: Program end / debug session end → exit
```

### Mode Differences

| Mode                     | Debugging Method                                                              |
| ------------------------ | ----------------------------------------------------------------------------- |
| `yaoxiang run --debug`   | Forces interpreter, full DAP interactive debugging                            |
| `yaoxiang run --release` | Generates DWARF for post-mortem backtraces (core dump / crash report), no DAP |
| `yaoxiang run` (normal)  | No debug metadata, no debug support                                           |

## Detailed Design

### 1. Breakpoints

```
Breakpoint types:
├── Source line breakpoint    → compilation frontend generates location metadata, backend queries match
├── Function entry breakpoint  → triggered on function call (Phase 2)
├── Conditional breakpoint    → triggered when expression evaluates to true
└── Data breakpoint           → triggered when variable is modified (Phase 2)
```

**Source line breakpoint core logic**:

```
VS Code sends: "Set breakpoint at file.yx:42"
    │
DAP server:
    ├── Query all IR nodes where SourceLocation == (file.yx, 42)
    ├── Forward to runtime: "Pause at these IR addresses"
    └── Runtime returns: breakpoint ID

Program runs to IR node → runtime checks: is this node in the breakpoint list?
    ├── Normal breakpoint → pause, notify DAP server
    └── Conditional breakpoint → evaluate condition expression → pause only if true
```

**Three engine implementations**:

|                                   | Interpreter                        | JIT                                          | LLVM                                       |
| --------------------------------- | ---------------------------------- | -------------------------------------------- | ------------------------------------------ |
| Breakpoint insertion              | Check IR node ID in execution loop | JIT inserts `int3` in machine code           | Leverage LLVM DWARF + hardware breakpoints |
| Conditional breakpoint evaluation | Directly interpret expression      | Temporarily JIT-compile condition expression | DWARF expression stack + evaluation        |
| Performance overhead              | One extra lookup per IR node       | Overhead only at breakpoints                 | Near-zero (hardware breakpoints)           |

### 2. Stepping

```
Step Over    → Execute current line, skip over function call internals, stop at next line
Step Into    → Enter function call internals on the current line
Step Out     → Execute until current function returns
Continue     → Resume execution until next breakpoint or program end
```

**Implementation logic**: Step operations are essentially **temporary breakpoints**. They share the
same mechanism as user-explicit breakpoints—not two systems, but two uses of one system.

```
Step Over:
    Current source line number = query_line(frame)
    → Set temporary breakpoint at next line
    → If current line is a function call: set temporary breakpoint after call site
    → Continue → hit temporary breakpoint → delete → pause

Step Into:
    First executable line of current call target
    → Find source location of first IR node in function body
    → Set temporary breakpoint → Continue → hit → pause

Step Out:
    Return address of current stack frame
    → Find next line of caller
    → Set temporary breakpoint → Continue → hit → pause
```

**Four edge cases for temporary breakpoints**:

1. **Concurrency attribution**: Temporary breakpoints are bound to the current task ID; other tasks
   that hit them are ignored.
2. **Step Over spawn block**: Stepping Over outside a spawn block is equivalent to running through
   the entire spawn block and jumping past it. To debug inside spawn, use Step Into.
3. **Temporary breakpoint not hit**: Set watchdog timeout (30 seconds without any breakpoint hit) →
   force pause → notify VS Code. Also listen for program exit events → immediate cleanup.
4. **Multiple IR nodes on the same line**: Step Over's temporary breakpoint is marked
   `ignore_current_line`; if the hit's source line equals the current line → ignore, continue.

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
│  Module-level bindings: constants,   │
│  type aliases, globals               │
├─ Locals ────────────────────────────┤
│  Local variables visible in current  │
│  function                            │
│  ├── Parameters (function args)      │
│  └── Local bindings (let / assign)   │
├─ Captured ──────────────────────────┤
│  External variables captured by      │
│  spawn block / closure               │
│  Display ownership state:            │
│  moved / ref-shared                  │
└─────────────────────────────────────┘
```

**Engine differences**:

| Engine      | Variable value retrieval                                                                         |
| ----------- | ------------------------------------------------------------------------------------------------ |
| Interpreter | Directly read VM stack frame and heap. Each value has a clear in-memory representation           |
| JIT         | Register and stack values → need JIT-compile-time "variable → register/stack slot" mapping table |
| LLVM        | DWARF's `.debug_info` section → `DW_AT_location` → LLDB native support                           |

**Special type display**: Compile-time predicates refine type display to show useful debugging
information:

```
x: Positive(x)  →  Display "Int (x > 0 = True)"
y: Sorted(y)    →  Display "Array(Int) (sorted guarantee)"
result: T       →  Display runtime concrete type
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

The interpreter and JIT each maintain their own frame linked lists. Frames are not zero-cost to
obtain—but debug mode doesn't aim for zero overhead.

### 5. Expression Evaluation (Watch / REPL)

User inputs any YaoXiang expression at a breakpoint:

```
Watch: x + y         → Return computed result
Watch: items[2].name → Access complex structure
Watch: f(x)          → Call function (side-effect risk)
```

**Evaluation strategy**:

```
User inputs expression
    │
├── Compiler frontend parses expression
├── Type check in current frame context
├── Variable values obtained from current frame (read-only references)
├── Expression executed as independent microprogram
│   └── Not allowed to modify external variables
│   └── Not allowed to spawn
│   └── Not allowed IO (or optionally enable)
└── Return result value → original frame state completely unchanged
```

**Interpreter is a natural sandbox**: Expression evaluation doesn't create a new sandbox—the
interpreter itself is a sandbox. Expression evaluation just temporarily pushes a frame and destroys
it when done. It shares the same VM as normal execution but doesn't commit any side effects.

**Function call evaluation**: Allowed by default, but warn the user "this expression may have side
effects" and require user confirmation before execution.

**Engine differences**:

| Engine      | Expression evaluation                                                                |
| ----------- | ------------------------------------------------------------------------------------ |
| Interpreter | Reuse existing eval code path, inject current frame environment                      |
| JIT         | Temporarily compile expression → link to current frame → execute → discard temp code |
| LLVM        | Not supported—LLVM mode does not do interactive debugging                            |

### 6. Concurrency Debugging

**Task model visibility**:

DAP's `threads` concept maps to YaoXiang's `spawn` tasks. Each task has its own stack frame linked
list and running state.

```
┌─ Threads ───────────────────────────┐
│  ● task-1  main()     file.yx:10   │ ← Currently focused
│  ▶ task-2  fetch()    file.yx:34   │ ← Running
│  ⏸ task-3  process()  file.yx:56   │ ← Paused at breakpoint
│  ◼ task-4  write()    Finished       │
└─────────────────────────────────────┘
```

**Breakpoints in concurrency context**:

| Pause mode           | Behavior                                     | Use case                       |
| -------------------- | -------------------------------------------- | ------------------------------ |
| `stop-all` (default) | One task hits → all tasks pause              | Debug data races, global state |
| `stop-this-only`     | Only pause the hitting task, others continue | Debug independent task logic   |

**Step semantics for spawn blocks**:

```
spawn {          // Step Over → runs through entire spawn block
    task_a()     // Step Into → enters task_a
    task_b()     // Runs in parallel, unaffected by individual step
}
```

### 7. DAP Protocol Mapping

#### Phase 1: Core Requests

| DAP Request         | YaoXiang Semantics                                                              |
| ------------------- | ------------------------------------------------------------------------------- |
| `initialize`        | Capability negotiation: supports breakpoints, stepping, variables, stack frames |
| `launch` / `attach` | Launch / attach to YaoXiang program (`--debug` uses attach mode)                |
| `setBreakpoints`    | Set source line breakpoints                                                     |
| `configurationDone` | Breakpoints ready, start execution                                              |
| `threads`           | Return list of all active spawn tasks                                           |
| `stackTrace`        | Return stack frame list for specified task                                      |
| `scopes`            | Return variable scopes for current frame                                        |
| `variables`         | Return variable list for specified scope                                        |
| `continue`          | Resume execution                                                                |
| `next`              | Step Over                                                                       |
| `stepIn`            | Step Into                                                                       |
| `stepOut`           | Step Out                                                                        |
| `pause`             | Interrupt all tasks                                                             |
| `evaluate`          | Evaluate expression in current frame                                            |
| `disconnect`        | End debug session                                                               |

#### Phase 2: Enhanced Requests

| DAP Request               | YaoXiang Semantics                                 |
| ------------------------- | -------------------------------------------------- |
| `setFunctionBreakpoints`  | Function name breakpoints                          |
| `setExceptionBreakpoints` | Pause on error/panic                               |
| `dataBreakpointInfo`      | Data breakpoint (trigger on variable modification) |

## Implementation Strategy

### Phase Zero: Infrastructure (Precedes all phases)

**Goal**: Compilation frontend attaches debug metadata to IR.

| Component     | Changes                                                                     |
| ------------- | --------------------------------------------------------------------------- |
| IR definition | Add `SourceLocation`, `VarName`, `TypeAnnotation` and other metadata fields |
| Parser        | Each AST node records source location                                       |
| TypeChecker   | Type information attached to IR nodes                                       |
| Testing       | Verify IR dump contains location and variable information                   |

**Does not involve runtime.**

**Landing progress (2026-09-17)**:

| Deliverable       | Status  | Landing Form                                                                                                                                                                                                                                                                                   |
| ----------------- | ------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source location   | Done    | All 76 `Instruction` variants carry the `span` field; `span()` method intentionally has no wildcard arm, so new variants missing span fail to compile. Location covers 40/41 instructions                                                                                                      |
| Variable name     | Done    | `LocalSlot { name, ty, scope_depth }` attached to `FunctionBody::Code::locals`; `register_local` generates and writes in place. `.42` debug section v2 carries names, v1 artifacts backward-compatible reading                                                                                 |
| Global slot names | Done    | `.42` debug section v3 carries "slot number → top-level binding name" table. Top-level bindings go through `Operand::Global`, not in any function's local name table; without this table, only the numeric value can be reported, not the variable name. v1/v2 artifacts read with empty table |
| Type information  | Partial | Slots already contain `ty`; `TypeAnnotation` as independent metadata not done                                                                                                                                                                                                                  |
| dump visibility   | Done    | `dump` outputs `; <file>:<line>:<col>` per instruction, and lists `locals: name@slot`                                                                                                                                                                                                          |

The data forms landed in the table above interface directly with Phase 1: DAP's breakpoint
resolution consumes `debug_map`, and the variable panel consumes `local_names` (name) and `locals`
(type).

### Phase 1: Interpreter DAP MVP

**Goal**: `yaoxiang run --debug file.yx` can set breakpoints, step, and view variables.

| Component                       | Changes                                                                                                                            |
| ------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| DAP server (new yx-core module) | stdio transport layer, core request handling, breakpoint manager (source line → IR node mapping)                                   |
| Runtime debug trait (yx-core)   | `DebugEngine` trait definition (pause, resume, step, get_frames, eval, get_variables)                                              |
| Interpreter                     | Breakpoint check in execution loop, pause/resume mechanism, frame linked list maintenance, `InterpreterDebugEngine` implementation |
| CLI                             | `yaoxiang run --debug` parameter                                                                                                   |

**Acceptance criteria**: For any `.yx` file under `tests/yaoxiang/`, can use VS Code to set
breakpoints, Step Over, and view variable values.

### Phase 2: Advanced Debugging Capabilities

**Goal**: Expression evaluation, function breakpoints, concurrency debugging, exception breakpoints.

| Component                      | Changes                                                                                                    |
| ------------------------------ | ---------------------------------------------------------------------------------------------------------- |
| Expression evaluation engine   | Microprogram compilation (reuse parser + typechecker), temporary frame push onto VM, side-effect isolation |
| Concurrency debugging          | spawn task list mapping, breakpoint bound to task ID, stop-all / stop-this-only pause strategy             |
| Function/exception breakpoints | `setFunctionBreakpoints`, `setExceptionBreakpoints` mapping                                                |
| VS Code extension              | Provide default `launch.json` template                                                                     |

### Phase 3: JIT Debugging & LLVM DWARF

**Goal**: JIT engine reuses DAP, LLVM produces DWARF for crash backtraces.

| Component | Changes                                                                                                                                              |
| --------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| JIT       | Implement `DebugEngine` trait, generate variable→register mapping table at compile time, runtime frame linked list, temporary expression compilation |
| LLVM      | IR debug metadata → LLVM `DILocation` / `DISubprogram` → DWARF (no DAP interaction)                                                                  |

### Dependency Relationships

```
Phase 0 (IR metadata)
    ↓
Phase 1 (Interpreter DAP MVP)  ← Usable starting here
    ↓
Phase 2 (Advanced capabilities)
    ↓
Phase 3 (JIT + LLVM DWARF)
```

### Risks

| Risk                                       | Mitigation                                                                                                          |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------- |
| Complexity of interpreter pause mechanism  | Use simple channel/signal instead of complex state machine—pausing is just preventing fetching the next instruction |
| Type safety of expression evaluation       | Reuse existing typechecker, read-only references, don't commit side effects                                         |
| DAP protocol details                       | Reference debugpy / delve implementations—the protocol is mature                                                    |
| stop-all livelock in concurrency debugging | Timeout mechanism + force pause                                                                                     |

## Trade-offs

### Advantages

- **One source**: Debug metadata generated only once, shared by three engines. No "interpreter debug
  info is correct but LLVM's is wrong" situations.
- **Zero intrusion**: One parameter, `--debug`. Behavior is completely unchanged without it.
- **DAP standard**: Direct integration with the VS Code ecosystem, no need for custom editor
  protocols or debugger UIs.
- **Interpreter-first**: Debugging is naturally suited for the interpreter—flexible, controllable,
  simple expression evaluation. Not having interactive debugging in LLVM mode is the most pragmatic
  choice.

### Disadvantages

- **Poor performance in debug mode**: The interpreter is much slower than JIT/LLVM. But debugging
  doesn't need performance—nobody expects debug mode to handle production load.
- **Limited LLVM debugging**: AOT compilation cannot do interactive debugging, only GDB/LLDB +
  DWARF. But this is a trade-off: there shouldn't be debug behavior differences in LLVM mode.
- **Complex concurrency pause**: Implementing stop-all semantics on the interpreter requires
  iterating all active tasks.

### Alternative Approaches

| Approach                              | Why not chosen                                                                                                                      |
| ------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| Three engines each implement DAP      | Triple the work, triple the bugs. Violates "good taste"                                                                             |
| Only use DWARF, no proprietary DAP    | Interpreter and JIT have no DWARF concept, LLDB can't enter VM internals                                                            |
| Command-line debugger like Python pdb | VS Code experience vastly outperforms command-line debuggers                                                                        |
| Embed DAP in LSP process              | Lifecycles are completely different—LSP follows the project, DAP follows the debug session. Process isolation is a hard requirement |

## Open Questions

- [ ] Is the conditional breakpoint expression syntax completely identical to normal YaoXiang?
      (Recommendation: completely identical, reuse parser)
- [ ] Step Into behavior inside `spawn` blocks: when the user presses Step Into entering a spawn
      block, which of the multiple parallel tasks should be displayed? (Recommendation: pause on the
      first already-created task)
- [ ] VS Code extension: should the debug configuration be placed in the existing
      `vscode-extension/` directory or a separate repository?

## References

- [RFC-024: spawn block-based Concurrency Model](../accepted/024-concurrency-model.md)
- [RFC-027: Compile-time Predicates and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md)
- [RFC-028: JIT Compiler — Multi-level Execution Engine in VM](../draft/028-jit-compiler.md)
- [RFC-030: assert Mechanism](../accepted/030-assert-mechanism.md)
- [DAP Protocol Specification](https://microsoft.github.io/debug-adapter-protocol/)
- [debugpy — Python DAP Implementation Reference](https://github.com/microsoft/debugpy)
- [Delve — Go Debugger Reference](https://github.com/go-delve/delve)
