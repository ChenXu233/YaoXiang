---
title: 'RFC-034: Unified Debug Toolchain'
status: 'Draft'
author: 'Chenxu'
created: '2026-07-06'
updated: '2026-09-18'
issue: '#164'
---

# RFC-034: Unified Debug Toolchain

## Summary

Introduce a unified debug toolchain for YaoXiang. The core design is **one source, three
consumers**: the compilation frontend embeds source locations, variable names, and type information
as first-class citizens into the YaoXiang IR; the three backends — interpreter, JIT, and LLVM — each
consume the same set of metadata. Users launch a DAP (Debug Adapter Protocol) server via
`yaoxiang run --debug`, VS Code connects over stdio, and gets a unified experience of breakpoints,
stepping, variable inspection, call stacks, expression evaluation, and concurrent debugging —
regardless of the underlying execution engine.

## Motivation

### Why is this feature needed?

Current ways of troubleshooting YaoXiang programs are extremely primitive:

```yaoxiang
io.println("DEBUG: x = " + x.to_string())
io.println("DEBUG: entered branch A")
```

Three fatal problems:

1. **Compiler self-hosting is blocked**: YaoXiang is used to write the YaoXiang compiler, but people
   writing the compiler cannot debug their own code. The lack of interactive debugging means the
   self-hosting phase hits a dead end.
2. **Three engines, zero debugging**: The interpreter, JIT, and LLVM each run on their own; when
   something goes wrong, users can only check whether `ALL TESTS PASSED` appears on stdout. An
   assertion failed? Don't know which line, don't know the variable value.
3. **Concurrency is a black box**: `spawn` creates multiple tasks — which one is stuck? Who moved a
   variable? Pure guesswork.

### Design goals

- **Unified experience**: A breakpoint hit under the interpreter must also be hit under JIT, and
  LLVM must have consistent source mapping. Users don't perceive differences in the underlying
  engine.
- **One source**: Debug metadata flows with the IR, defined once, no two mappings to maintain.
- **Zero intrusion**: A single `yaoxiang run --debug` flag; without it, compilation and execution
  behavior is completely unchanged.
- **DAP standard**: Plug directly into the VS Code ecosystem; don't reinvent the editor protocol.

## Proposal

### Core design

Architecture overview:

```
┌──────────────────────────────────────────────────────┐
│                    VS Code / Editor                   │
│              DAP Client (launch.json)                 │
└────────────────────────┬─────────────────────────────┘
                         │ stdio
┌────────────────────────▼─────────────────────────────┐
│                 DAP Server (yx-core)                  │
│  ┌─────────┐  ┌──────────┐  ┌───────────────────┐    │
│  │ Session │  │ Breakpoint│  │ Expression Eval   │    │
│  │ Mgmt    │  │ Mgmt     │  │ Engine            │    │
│  └─────────┘  └──────────┘  └───────────────────┘    │
└────────────────────────┬─────────────────────────────┘
                         │ Query/Control
┌────────────────────────▼─────────────────────────────┐
│              Runtime Debug Interface (trait)          │
│  pause / resume / step / get_frames / eval / ...    │
└────┬──────────────────┬──────────────────┬──────────┘
     │                  │                  │
┌────▼────┐    ┌───────▼───────┐    ┌─────▼──────┐
│Interpreter   │   JIT (RFC-028) │    │ LLVM AOT   │
│ Directly │    │ Generate light │    │ IR metadata│
│ consumes │    │ debug tables   │    │ → DWARF    │
│ IR meta  │    │                │    │            │
└─────────┘    └────────────────┘    └────────────┘
```

**Key design decisions**:

1. **The DAP server and runtime are decoupled through a trait**. The server doesn't care whether the
   underlying engine is interpreter or JIT — it only issues commands through the `DebugEngine`
   trait. Each engine implements the same trait independently.
2. **`yaoxiang run --debug` forces the interpreter**. Debugging needs controllability, not
   performance. Under LLVM mode, only DWARF is generated for post-mortem tracing (core dump / crash
   report), no interactive debugging.
3. **Reuse entry discovery logic from `yaoxiang run`**. No new subcommand; the mental model is
   simply "run my program in debug mode".

### IR debug metadata

Attach metadata to the existing YaoXiang IR without introducing new IR kinds. All metadata is
generated in **one place — the compilation frontend**; backend consumption is read-only:

| Metadata         | Attachment Point          | Description                                   |
| ---------------- | ------------------------- | --------------------------------------------- |
| `SourceLocation` | Each IR node              | Source file:line:column                       |
| `VarName`        | Variable decl/bind node   | Variable name from source                     |
| `TypeAnnotation` | Variable/expression node  | Inferred type (incl. compile-time predicates) |
| `ScopeBoundary`  | Block/function entry/exit | Variable scope lifetime                       |
| `SpanInfo`       | spawn node                | Task boundary inside a spawn block            |

### Startup flow

```
yaoxiang run --debug file.yx
    │
    ├── Phase 1: Compilation (with debug metadata)
    │   ├── Parse → AST
    │   ├── Type check + compile-time predicate validation
    │   └── Lower to IR (with debug metadata)
    │
    ├── Phase 2: Use the interpreter engine
    │   └── Under --debug, interpreter is used regardless of --release
    │
    ├── Phase 3: Launch DAP server
    │   ├── Initialize stdio transport channel
    │   ├── Wait for VS Code attach
    │   ├── After successful attach, pause at program entry
    │   └── Enter interactive debug loop
    │
    └── Phase 4: Program end / debug session end → exit
```

### Differences across modes

| Mode                     | Debug approach                                                            |
| ------------------------ | ------------------------------------------------------------------------- |
| `yaoxiang run --debug`   | Force interpreter, full DAP interactive debugging                         |
| `yaoxiang run --release` | Generate DWARF for post-mortem tracing (core dump / crash report), no DAP |
| `yaoxiang run` (normal)  | No debug metadata, no debugging support                                   |

## Detailed design

### 1. Breakpoints

```
Breakpoint types:
├── Source-line breakpoint   → Frontend generates location metadata, backend queries match
├── Function-entry breakpoint→ Triggered on function call (Phase 2)
├── Conditional breakpoint   → Triggered when expression evaluates to true
└── Data breakpoint          → Triggered when variable is modified (Phase 2)
```

**Source-line breakpoint core logic**:

```
VS Code sends: "Set breakpoint at file.yx:42"
    │
DAP server:
    ├── Query all IR nodes whose SourceLocation == (file.yx, 42)
    ├── Forward to runtime: "Pause at these IR addresses"
    └── Runtime returns: breakpoint ID

Program runs to an IR node → Runtime checks: is this node in the breakpoint list?
    ├── Normal breakpoint → Pause, notify DAP server
    └── Conditional breakpoint → Evaluate condition expression → only pause if true
```

**Implementation across three engines**:

|                             | Interpreter                   | JIT                               | LLVM                                  |
| --------------------------- | ----------------------------- | --------------------------------- | ------------------------------------- |
| Breakpoint insertion        | Check IR node ID in exec loop | Insert `int3` in machine code     | Use LLVM DWARF + hardware breakpoints |
| Conditional breakpoint eval | Interpret expression directly | Temporarily JIT-compile condition | DWARF expression stack + evaluate     |
| Performance overhead        | One extra lookup per IR node  | Overhead only at breakpoint       | Near zero (hardware breakpoint)       |

### 2. Single-step execution

```
Step Over    → Execute current line, skip into function call, stop at next line
Step Into    → Enter the function call on the current line
Step Out     → Execute until current function returns
Continue     → Resume execution until next breakpoint or program end
```

**Implementation logic**: Single-step is essentially a **temporary breakpoint**. It shares the same
mechanism as user-set breakpoints — not two systems, but two uses of one system.

```
Step Over:
    Current source line = query_line(frame)
    → Set temporary breakpoint on next line
    → If current line is a function call: set temporary breakpoint after call site
    → Continue → hit temporary breakpoint → delete → pause

Step Into:
    First executable position of call target
    → Find source location of first IR node in function body
    → Set temporary breakpoint → Continue → hit → pause

Step Out:
    Return address of current stack frame
    → Find next line of caller
    → Set temporary breakpoint → Continue → hit → pause
```

**Four boundary cases for temporary breakpoints**:

1. **Concurrent ownership**: Temporary breakpoint binds to current task ID; hits by other tasks are
   ignored.
2. **Step Over a spawn block**: Step Over outside a spawn block equals running through the entire
   spawn block, jumping to afterward. To debug inside spawn, use Step Into.
3. **Temporary breakpoint not hit**: Set a watchdog timeout (30 seconds without any breakpoint hit)
   → force pause → notify VS Code. Also listen for program-exit events → clean up immediately.
4. **Multiple IR nodes on the same line**: The temporary breakpoint for Step Over is marked
   `ignore_current_line`; on hit, if source line equals current line → ignore and continue.

### 3. Variable inspection and scope

```
VS Code request: "Variable list of current frame"
    │
DAP server:
    ├── Query IR node at current pause point
    ├── Iterate variable bindings within current ScopeBoundary
    │   └── For each binding return: (name, type, runtime value reference)
    └── Assemble VariablesResponse → VS Code
```

**Scope hierarchy**:

```
┌─ Globals ───────────────────────────┐
│  Module-level bindings: constants,  │
│  type aliases, globals              │
├─ Locals ────────────────────────────┤
│  Local variables visible in current │
│  function                           │
│  ├── Parameters (function args)     │
│  └── Local bindings (let / assign)  │
├─ Captured ──────────────────────────┤
│  External variables captured by     │
│  spawn blocks / closures            │
│  Display ownership state:           │
│  moved / ref-shared                 │
└─────────────────────────────────────┘
```

**Engine differences**:

| Engine      | Variable value retrieval                                                                                |
| ----------- | ------------------------------------------------------------------------------------------------------- |
| Interpreter | Directly read VM stack frame and heap. Each value has a clear in-memory representation                  |
| JIT         | Register and stack values → need JIT to record "variable → register/stack slot" mapping at compile time |
| LLVM        | DWARF `.debug_info` section → `DW_AT_location` → native LLDB support                                    |

**Special type display**: Compile-time predicates refine type display to show useful debugging
information:

```
x: Positive(x)  →  Display "Int (x > 0 = True)"
y: Sorted(y)    →  Display "Array(Int) (sorted invariant)"
result: T       →  Display the concrete runtime type
```

### 4. Call stack

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

Interpreter and JIT each maintain their own frame linked lists. Frames are not zero-cost to obtain —
but debug mode does not pursue zero overhead.

### 5. Expression evaluation (Watch / REPL)

Users input arbitrary YaoXiang expressions at a breakpoint:

```
Watch: x + y         → Return computed result
Watch: items[2].name → Access complex structure
Watch: f(x)          → Call function (side-effect risk)
```

**Evaluation strategy**:

```
User inputs expression
    │
├── Frontend parses expression
├── Type check in current frame context
├── Variable values obtained from current frame (read-only reference)
├── Expression executes as an independent micro-program
│   └── Modifying external variables is forbidden
│   └── spawn is forbidden
│   └── IO is forbidden (or optionally enabled)
└── Return result value → Original frame state completely unchanged
```

**Interpreter as natural sandbox**: Expression evaluation does not create a new sandbox — the
interpreter itself is a sandbox. Expression evaluation simply pushes a temporary frame, then
destroys it when done. It shares the same VM as normal execution but commits no side effects.

**Function call evaluation**: Allowed by default, but the user is warned that "this expression may
have side effects" and must confirm before execution.

**Engine differences**:

| Engine      | Expression evaluation                                                                     |
| ----------- | ----------------------------------------------------------------------------------------- |
| Interpreter | Reuse existing eval code path, inject current frame environment                           |
| JIT         | Temporarily compile expression → link to current frame → execute → discard temporary code |
| LLVM        | Not supported — LLVM mode does not provide interactive debugging                          |

### 6. Concurrent debugging

**Task model visibility**:

DAP's `threads` concept maps to YaoXiang's `spawn` tasks. Each task has its own stack frame list and
run state.

```
┌─ Threads ───────────────────────────┐
│  ● task-1  main()     file.yx:10   │ ← Currently focused
│  ▶ task-2  fetch()    file.yx:34   │ ← Running
│  ⏸ task-3  process()  file.yx:56   │ ← Paused at breakpoint
│  ◼ task-4  write()    finished       │
└─────────────────────────────────────┘
```

**Breakpoints in concurrent context**:

| Pause mode           | Behavior                                      | Use case                       |
| -------------------- | --------------------------------------------- | ------------------------------ |
| `stop-all` (default) | One task hits → all tasks pause               | Debug data races, global state |
| `stop-this-only`     | Only the hitting task pauses, others continue | Debug independent task logic   |

**Single-step semantics in spawn blocks**:

```
spawn {          // Step Over → run through entire spawn block
    task_a()     // Step Into → enter task_a
    task_b()     // Runs in parallel, not affected by individual step
}
```

### 7. DAP protocol mapping

#### Phase 1: Core requests

| DAP request         | YaoXiang semantics                                                     |
| ------------------- | ---------------------------------------------------------------------- |
| `initialize`        | Capability negotiation: breakpoints, stepping, variables, stack frames |
| `launch` / `attach` | Launch / attach to YaoXiang program (`--debug` uses attach mode)       |
| `setBreakpoints`    | Set source-line breakpoints                                            |
| `configurationDone` | Breakpoints ready, begin execution                                     |
| `threads`           | Return list of all active spawn tasks                                  |
| `stackTrace`        | Return stack frame list of specified task                              |
| `scopes`            | Return variable scopes of current frame                                |
| `variables`         | Return variable list of specified scope                                |
| `continue`          | Resume execution                                                       |
| `next`              | Step Over                                                              |
| `stepIn`            | Step Into                                                              |
| `stepOut`           | Step Out                                                               |
| `pause`             | Interrupt all tasks                                                    |
| `evaluate`          | Evaluate expression in current frame                                   |
| `disconnect`        | End debug session                                                      |

#### Phase 2: Enhanced requests

| DAP request               | YaoXiang semantics                                   |
| ------------------------- | ---------------------------------------------------- |
| `setFunctionBreakpoints`  | Function-name breakpoint                             |
| `setExceptionBreakpoints` | Pause on error / panic                               |
| `dataBreakpointInfo`      | Data breakpoint (triggered on variable modification) |

## Implementation strategy

### Phase 0: Infrastructure (prerequisite for all phases)

**Goal**: Compilation frontend attaches debug metadata to the IR.

| Component     | Change                                                                   |
| ------------- | ------------------------------------------------------------------------ |
| IR definition | Add metadata fields: `SourceLocation`, `VarName`, `TypeAnnotation`, etc. |
| Parser        | Record source location for every AST node                                |
| TypeChecker   | Attach type information to IR nodes                                      |
| Tests         | Verify IR dump contains location and variable info                       |

**No runtime changes.**

**Landing progress (2026-09-17)**:

| Deliverable       | Status  | Form                                                                                                                                                                                                                                                                                          |
| ----------------- | ------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source location   | Done    | All 76 `Instruction` variants carry a `span` field; the `span()` method deliberately has no wildcard arm, so a new variant that omits `span` fails to compile. Location coverage: 40/41 instructions                                                                                          |
| Variable name     | Done    | `LocalSlot { name, ty, scope_depth }` attached to `FunctionBody::Code::locals`; `register_local` writes inline at generation time. The `.42` debug section v2 carries names; v1 artifacts are read backward-compatibly.                                                                       |
| Global slot names | Done    | The `.42` debug section v3 carries a "slot number → top-level binding name" table. Top-level bindings use `Operand::Global` and are not in any function's local name table; without this table only numeric IDs can be reported, not variable names. v1/v2 artifacts read with an empty table |
| Type info         | Partial | Slots already contain `ty`; standalone `TypeAnnotation` metadata not yet done                                                                                                                                                                                                                 |
| Dump visibility   | Done    | `dump` outputs `; <file>:<line>:<col>` per instruction, and lists `locals: name@slot`                                                                                                                                                                                                         |

The data form landed above directly interfaces with Phase 1: DAP's breakpoint resolution consumes
`debug_map`; the variables panel consumes `local_names` (names) and `locals` (types).

### Phase 1: Interpreter DAP MVP

**Goal**: `yaoxiang run --debug file.yx` can set breakpoints, step, and view variables.

| Component                          | Change                                                                                                                 |
| ---------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| DAP server (new module in yx-core) | stdio transport layer, core request handling, breakpoint manager (source line → IR node mapping)                       |
| Runtime debug trait (yx-core)      | `DebugEngine` trait definition (pause, resume, step, get_frames, eval, get_variables)                                  |
| Interpreter                        | Breakpoint check in exec loop, pause/resume mechanism, frame list maintenance, `InterpreterDebugEngine` implementation |
| CLI                                | `yaoxiang run --debug` flag                                                                                            |

**Acceptance criteria**: For any `.yx` file under `tests/yaoxiang/`, VS Code can set breakpoints,
Step Over, and view variable values.

### Phase 2: Advanced debugging capabilities

**Goal**: Expression evaluation, function breakpoints, concurrent debugging, exception breakpoints.

| Component                        | Change                                                                                                    |
| -------------------------------- | --------------------------------------------------------------------------------------------------------- |
| Expression eval engine           | Micro-program compilation (reuse parser + typechecker), temporary frame push to VM, side-effect isolation |
| Concurrent debugging             | spawn task list mapping, breakpoint bound to task ID, stop-all / stop-this-only pause policy              |
| Function / exception breakpoints | `setFunctionBreakpoints`, `setExceptionBreakpoints` mapping                                               |
| VS Code extension                | Provide default `launch.json` template                                                                    |

### Phase 3: JIT debugging & LLVM DWARF

**Goal**: JIT engine reuses DAP; LLVM emits DWARF for crash backtracing.

| Component | Change                                                                                                                                  |
| --------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| JIT       | Implement `DebugEngine` trait, generate variable→register mapping at compile time, runtime frame list, temporary compile of expressions |
| LLVM      | IR debug metadata → LLVM `DILocation` / `DISubprogram` → DWARF (no DAP interaction)                                                     |

### Dependencies

```
Phase 0 (IR metadata)
    ↓
Phase 1 (Interpreter DAP MVP)  ← Usable from here
    ↓
Phase 2 (Advanced capabilities)
    ↓
Phase 3 (JIT + LLVM DWARF)
```

### Risks

| Risk                                      | Mitigation                                                                                                       |
| ----------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| Complexity of interpreter pause mechanism | Use simple channel/signal instead of complex state machine; pausing just means not fetching the next instruction |
| Type safety of expression evaluation      | Reuse existing typechecker, read-only references, commit no side effects                                         |
| DAP protocol details                      | Reference debugpy / delve implementations; protocol is mature                                                    |
| stop-all livelock in concurrent debugging | Timeout mechanism + forced pause                                                                                 |

## Trade-offs

### Pros

- **One source**: Debug metadata is generated only once, shared by three engines. Avoids
  "interpreter debug info is right but LLVM's is wrong"
- **Zero intrusion**: A single `--debug` flag; without it, behavior is completely unchanged
- **DAP standard**: Plug directly into the VS Code ecosystem; no custom editor protocol or debugger
  UI needed
- **Interpreter-first**: Debugging is naturally suited to the interpreter — flexible, controllable,
  simple expression evaluation. Not doing interactive debugging in LLVM mode is the most pragmatic
  choice

### Cons

- **Poor debug-mode performance**: Interpreter is much slower than JIT/LLVM. But debugging doesn't
  need performance — no one expects debug mode to run production load
- **Limited LLVM debugging**: AOT compilation cannot provide interactive debugging; only GDB/LLDB +
  DWARF. But this is a trade-off: under LLVM mode, there shouldn't be behavioral difference in
  debugging
- **Complexity of concurrent pause**: Implementing stop-all semantics on the interpreter requires
  iterating over all active tasks

### Alternatives

| Alternative                                     | Why not chosen                                                                                                                        |
| ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| Each of three engines implements DAP separately | Triple the work, triple the bugs. Violates "good taste"                                                                               |
| Only use DWARF, no native DAP                   | Interpreter and JIT have no DWARF concept; LLDB can't enter the VM                                                                    |
| Command-line debugger similar to Python pdb     | VS Code experience completely outshines command-line debuggers                                                                        |
| Put DAP inside the LSP process                  | Lifecycles are completely different — LSP follows the project, DAP follows the debug session. Process isolation is a hard requirement |

## Open questions

- [ ] Is conditional breakpoint expression syntax identical to normal YaoXiang? (Suggestion:
      identical, reuse parser)
- [ ] Step Into behavior inside a `spawn` block: when the user presses Step Into to enter a spawn
      block, which of the parallel tasks should be shown? (Suggestion: pause on the first
      already-created task)
- [ ] VS Code extension: should the debug configuration be placed under the existing
      `vscode-extension/` directory or in a separate repository?

## References

- [RFC-024: Concurrency Model Based on spawn Blocks](../accepted/024-concurrency-model.md)
- [RFC-027: Compile-time Predicates and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md)
- [RFC-028: JIT Compiler — Multi-level Execution Engine in the VM](028-jit-compiler.md)
- [RFC-030: assert Mechanism](../accepted/030-assert-mechanism.md)
- [DAP Protocol Specification](https://microsoft.github.io/debug-adapter-protocol/)
- [debugpy — Python DAP implementation reference](https://github.com/microsoft/debugpy)
- [Delve — Go debugger reference](https://github.com/go-delve/delve)
