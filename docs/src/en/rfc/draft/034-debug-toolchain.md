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
as first-class citizens into YaoXiang IR, and the interpreter, JIT, and LLVM backends each consume
the same set of metadata. Users launch a DAP (Debug Adapter Protocol) server via
`yaoxiang run --debug`, VS Code connects over stdio, and obtains a unified experience of
breakpoints, stepping, variable inspection, call stack, expression evaluation, and concurrent
debugging—regardless of the underlying execution engine.

## Motivation

### Why is this feature needed?

The current means of debugging YaoXiang programs are extremely primitive:

```yaoxiang
io.println("DEBUG: x = " + x.to_string())
io.println("DEBUG: entered branch A")
```

Three fatal problems:

1. **Compiler bootstrapping is blocked**: The YaoXiang compiler is written in YaoXiang, but the
   compiler authors cannot debug their own code. The lack of interactive debugging during the
   bootstrapping phase is a dead end.
2. **Three engines, zero debugging**: The interpreter, JIT, and LLVM each run on their own; when
   something goes wrong, users can only look for `ALL TESTS PASSED` in stdout. Assertion failure? No
   idea which line, no idea what the variable values are.
3. **Concurrency is a black box**: `spawn` creates multiple tasks; which task hung? Who moved the
   variable? All guesswork by luck.

### Design Goals

- **Unified experience**: A breakpoint that works in the interpreter also works in JIT, and LLVM has
  consistent source mapping. Users don't perceive differences in the underlying engine.
- **One source**: Debug metadata flows with the IR; no duplicate definitions, no maintaining two
  sets of mappings.
- **Zero intrusion**: One flag, `yaoxiang run --debug`; when this flag is not added, compilation and
  execution behavior remains completely unchanged.
- **DAP standard**: Connects directly to the VS Code ecosystem; no reinventing editor protocols.

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
│  │ Session │  │ Breakpoint│  │ Expression Eval   │    │
│  │ Manager │  │ Manager   │  │ Engine             │    │
│  └─────────┘  └──────────┘  └───────────────────┘    │
└────────────────────────┬─────────────────────────────┘
                         │ Query/Control
┌────────────────────────▼─────────────────────────────┐
│              Runtime Debug Interface (trait)           │
│  pause / resume / step / get_frames / eval / ...    │
└────┬──────────────────┬──────────────────┬──────────┘
     │                  │                  │
┌────▼────┐    ┌───────▼───────┐    ┌─────▼──────┐
│ Interpreter│    │ JIT (RFC-028)│    │ LLVM AOT   │
│ Consumes  │    │ Generates    │    │ IR metadata │
│ IR meta   │    │ Lightweight  │    │ → DWARF    │
│ directly  │    │ Debug Table  │    │            │
└─────────┘    └───────────────┘    └────────────┘
```

**Key design decisions**:

1. **DAP server and runtime are decoupled through a trait**. The server doesn't care whether the
   underlying is interpreter or JIT—it only issues commands through the `DebugEngine` trait. Each
   engine implements the same trait independently.
2. **`yaoxiang run --debug` forces the use of the interpreter**. Debugging needs controllability,
   not performance. Under LLVM mode, only DWARF is generated for post-mortem analysis (core dump /
   crash report); interactive debugging is not done.
3. **Reuse entry discovery logic from `yaoxiang run`**. No new subcommands; the mental model is
   simply "run my program in debug mode".

### IR Debug Metadata

Attach metadata on top of the existing YaoXiang IR without adding new IR kinds. All metadata is
generated in **one place in the compilation frontend**; backend consumption is read-only:

| Metadata         | Attachment Point                  | Description                                       |
| ---------------- | --------------------------------- | ------------------------------------------------- |
| `SourceLocation` | Each IR node                      | Source file:line:column                           |
| `VarName`        | Variable declaration/binding node | Variable name in source code                      |
| `TypeAnnotation` | Variable/expression node          | Inferred type (including compile-time predicates) |
| `ScopeBoundary`  | Block/function entry and exit     | Lifecycle of variable scope                       |
| `SpanInfo`       | spawn node                        | Task boundary within spawn block                  |

### Startup Flow

```
yaoxiang run --debug file.yx
    │
    ├── Phase 1: Compilation (with debug metadata)
    │   ├── Parse → AST
    │   ├── Type check + compile-time predicate validation
    │   └── Lower to IR (attach debug metadata)
    │
    ├── Phase 2: Use interpreter engine
    │   └── In --debug mode, regardless of --release, use the interpreter
    │
    ├── Phase 3: Start DAP server
    │   ├── Initialize stdio transport channel
    │   ├── Wait for VS Code attach
    │   ├── Pause at program entry after attach succeeds
    │   └── Enter interactive debug loop
    │
    └── Phase 4: Program ends / debug session ends → exit
```

### Differences Across Modes

| Mode                     | Debugging Method                                                            |
| ------------------------ | --------------------------------------------------------------------------- |
| `yaoxiang run --debug`   | Forces interpreter, full-featured DAP interactive debugging                 |
| `yaoxiang run --release` | Generates DWARF for post-mortem analysis (core dump / crash report), no DAP |
| `yaoxiang run` (normal)  | No debug metadata, no debugging support                                     |

## Detailed Design

### 1. Breakpoints

```
Breakpoint types:
├── Source line breakpoint → compilation frontend generates location metadata, backend queries for matches
├── Function entry breakpoint → triggers on function call (Phase 2)
├── Conditional breakpoint → triggers when expression evaluates to true
└── Data breakpoint → triggers when variable is modified (Phase 2)
```

**Source line breakpoint core logic**:

```
VS Code sends: "Set breakpoint at file.yx:42"
    │
DAP server:
    ├── Query all nodes in IR with SourceLocation == (file.yx, 42)
    ├── Forward to runtime: "Pause at these IR addresses"
    └── Runtime returns: breakpoint ID

Program runs to IR node → runtime checks: is this node in the breakpoint list?
    ├── Normal breakpoint → pause, notify DAP server
    └── Conditional breakpoint → evaluate condition expression → pause only if true
```

**Three engine implementations**:

|                                   | Interpreter                        | JIT                                          | LLVM                                      |
| --------------------------------- | ---------------------------------- | -------------------------------------------- | ----------------------------------------- |
| Breakpoint insertion              | Check IR node ID in execution loop | Insert `int3` in machine code in JIT         | Use LLVM DWARF + hardware breakpoints     |
| Conditional breakpoint evaluation | Interpret expression directly      | Temporarily JIT compile condition expression | DWARF expression stack + evaluation       |
| Performance overhead              | One extra table lookup per IR node | Overhead only at breakpoints                 | Near zero overhead (hardware breakpoints) |

### 2. Single-step Execution

```
Step Over    → Execute current line, skip inside function calls, stop at next line
Step Into    → Enter the inside of the function call on the current line
Step Out     → Execute until current function returns
Continue     → Resume execution until next breakpoint or program ends
```

**Implementation logic**: Single-step operations are essentially **temporary breakpoints**. They
share the same mechanism as user-set breakpoints; not two systems, but two uses of one system.

```
Step Over:
    Current source line number = query_line(frame)
    → Set temporary breakpoint at next line
    → If current line is a function call: set temporary breakpoint after call site
    → Continue → hit temporary breakpoint → delete → pause

Step Into:
    First executable location of current call target
    → Find source location of first IR node of function body
    → Set temporary breakpoint → Continue → hit → pause

Step Out:
    Return address of current stack frame
    → Find caller's next line
    → Set temporary breakpoint → Continue → hit → pause
```

**Four edge case handlers for temporary breakpoints**:

1. **Concurrency ownership**: Temporary breakpoint is bound to current task ID; other tasks hitting
   it are ignored directly.
2. **Step Over spawn block**: Stepping Over outside a spawn block means running the entire spawn
   block and jumping past it. To debug inside spawn, use Step Into.
3. **Temporary breakpoint not hit**: Set watchdog timeout (30 seconds without any breakpoint hit) →
   force pause → notify VS Code. Also listen for program exit event → cleanup immediately.
4. **Multiple IR nodes on the same line**: The temporary breakpoint for Step Over is marked
   `ignore_current_line`; after hit, if source line number equals current line number → ignore,
   continue.

### 3. Variable Inspection and Scope

```
VS Code requests: "Variable list for current frame"
    │
DAP server:
    ├── Query IR node of current pause point
    ├── Iterate variable bindings within current ScopeBoundary
    │   └── Each binding returns: (name, type, runtime value reference)
    └── Assemble VariablesResponse → VS Code
```

**Scope layering**:

```
┌─ Globals ───────────────────────────┐
│  Module-level bindings: constants,  │
│  type aliases, globals              │
├─ Locals ────────────────────────────┤
│  Local variables visible in current │
│  function                            │
│  ├── Parameters (function params)   │
│  └── Local bindings (let / assign)  │
├─ Captured ──────────────────────────┤
│  External variables captured by     │
│  spawn blocks / closures            │
│  Show ownership state: moved / ref  │
│  shared                             │
└─────────────────────────────────────┘
```

**Engine differences**:

| Engine      | Variable value retrieval                                                                                      |
| ----------- | ------------------------------------------------------------------------------------------------------------- |
| Interpreter | Read VM stack frames and heap directly. Each value has a clear representation in memory                       |
| JIT         | Values in registers and stack → need JIT compilation to record "variable → register/stack slot" mapping table |
| LLVM        | DWARF `.debug_info` section → `DW_AT_location` → natively supported by LLDB                                   |

**Special type display**: Compile-time predicates refine the type display to show information
valuable for debugging:

```
x: Positive(x)  →  Display "Int (x > 0 = True)"
y: Sorted(y)    →  Display "Array(Int) (sorted guarantee)"
result: T       →  Display the runtime concrete type
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
│ #1  main()          file.yx:67  │  ← caller
│     locals: data = ["hello", ...]│
├──────────────────────────────────┤
│ #2  <entry>         file.yx:1   │  ← root
└──────────────────────────────────┘
```

Each frame records: function signature, call location (source file + line number), local variables
(lazy evaluation), spawn context (task ID).

The interpreter and JIT each maintain their own frame linked lists. Frames are not zero-cost to
obtain—but debug mode doesn't aim for zero overhead.

### 5. Expression Evaluation (Watch / REPL)

The user inputs any YaoXiang expression at a breakpoint:

```
Watch: x + y         → Return computation result
Watch: items[2].name → Access complex structure
Watch: f(x)          → Call function (risk of side effects)
```

**Evaluation strategy**:

```
User inputs expression
    │
├── Compiler frontend parses expression
├── Type check in current frame context
├── Variable values obtained from current frame (read-only references)
├── Expression executes as an independent microprogram
│   └── Not allowed to modify external variables
│   └── Not allowed to spawn
│   └── Not allowed to do IO (or optionally enabled)
└── Return result value → original frame state completely unchanged
```

**Interpreter as natural sandbox**: Expression evaluation is not creating a new sandbox—the
interpreter itself is the sandbox. Expression evaluation just temporarily pushes a frame, and it's
destroyed after use. It shares the same VM as normal execution but commits no side effects.

**Function call evaluation**: Allowed by default, but warns the user "this expression may have side
effects", requiring user confirmation before execution.

**Engine differences**:

| Engine      | Expression Evaluation                                                                     |
| ----------- | ----------------------------------------------------------------------------------------- |
| Interpreter | Reuse existing eval code path, inject current frame environment                           |
| JIT         | Temporarily compile expression → link to current frame → execute → discard temporary code |
| LLVM        | Not supported—LLVM mode does not do interactive debugging                                 |

### 6. Concurrent Debugging

**Task model visibility**:

DAP's `threads` concept maps to YaoXiang's `spawn` tasks. Each task has its own stack frame linked
list and runtime state.

```
┌─ Threads ───────────────────────────┐
│  ● task-1  main()     file.yx:10   │ ← Currently focused
│  ▶ task-2  fetch()    file.yx:34   │ ← Running
│  ⏸ task-3  process()  file.yx:56   │ ← Paused at breakpoint
│  ◼ task-4  write()    ended         │
└─────────────────────────────────────┘
```

**Breakpoints in concurrent context**:

| Pause Mode           | Behavior                                     | Use Case                       |
| -------------------- | -------------------------------------------- | ------------------------------ |
| `stop-all` (default) | One task hits → all tasks pause              | Debug data races, global state |
| `stop-this-only`     | Only pause the hitting task, others continue | Debug independent task logic   |

**Step semantics in spawn blocks**:

```
spawn {          // Step Over → run through entire spawn block
    task_a()     // Step Into → enter task_a
    task_b()     // Runs in parallel, not affected by individual step
}
```

### 7. DAP Protocol Mapping

#### Phase 1: Core Requests

| DAP Request         | YaoXiang Semantics                                                      |
| ------------------- | ----------------------------------------------------------------------- |
| `initialize`        | Capability negotiation: support breakpoint, step, variable, stack frame |
| `launch` / `attach` | Launch/attach to YaoXiang program (`--debug` uses attach mode)          |
| `setBreakpoints`    | Set source line breakpoints                                             |
| `configurationDone` | Breakpoints ready, begin execution                                      |
| `threads`           | Return list of all active spawn tasks                                   |
| `stackTrace`        | Return stack frame list for specified task                              |
| `scopes`            | Return variable scopes for current frame                                |
| `variables`         | Return variable list for specified scope                                |
| `continue`          | Resume execution                                                        |
| `next`              | Step Over                                                               |
| `stepIn`            | Step Into                                                               |
| `stepOut`           | Step Out                                                                |
| `pause`             | Interrupt all tasks                                                     |
| `evaluate`          | Evaluate expression in current frame                                    |
| `disconnect`        | End debug session                                                       |

#### Phase 2: Enhanced Requests

| DAP Request               | YaoXiang Semantics                                  |
| ------------------------- | --------------------------------------------------- |
| `setFunctionBreakpoints`  | Function name breakpoint                            |
| `setExceptionBreakpoints` | Pause on error/panic                                |
| `dataBreakpointInfo`      | Data breakpoint (triggers on variable modification) |

## Implementation Strategy

### Phase Zero: Infrastructure (Prerequisite for all phases)

**Goal**: Compilation frontend attaches debug metadata to IR.

| Component     | Change                                                                      |
| ------------- | --------------------------------------------------------------------------- |
| IR definition | Add `SourceLocation`, `VarName`, `TypeAnnotation` and other metadata fields |
| Parser        | Each AST node records source location                                       |
| TypeChecker   | Type information attached to IR nodes                                       |
| Testing       | Verify IR dump contains location and variable information                   |

**Does not involve runtime.**

**Landing progress (2026-09-17)**:

| Deliverable       | Status    | Landing Form                                                                                                                                                                                                                                                                                                      |
| ----------------- | --------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source location   | Completed | All 76 `Instruction` variants carry a `span` field; the `span()` method deliberately omits a wildcard arm, so any new variant failing to carry a span fails to compile. Location covers 40/41 instructions                                                                                                        |
| Variable name     | Completed | `LocalSlot { name, ty, scope_depth }` attached to `FunctionBody::Code::locals`; `register_local` populates it in-place at generation. The `.42` debug section v2 carries names; v1 artifacts are backward-compatibly readable                                                                                     |
| Global slot names | Completed | The `.42` debug section v3 carries a "slot number → top-level binding name" table. Top-level bindings use `Operand::Global` and don't appear in any function's local name table; without this table, only numeric values can be reported, not variable names. v1/v2 artifacts fall back to reading an empty table |
| Type information  | Partial   | Slots already contain `ty`; independent `TypeAnnotation` metadata not yet done                                                                                                                                                                                                                                    |
| dump visibility   | Completed | `dump` outputs `; <file>:<line>:<col>` for each instruction, and lists `locals: name@slot`                                                                                                                                                                                                                        |

The data forms landed in the table above directly interface with Phase 1: DAP breakpoint resolution
consumes `debug_map`; the variable panel consumes `local_names` (names) and `locals` (types).

### Phase 1: Interpreter DAP MVP

**Goal**: `yaoxiang run --debug file.yx` can set breakpoints, step, and view variables.

| Component                       | Change                                                                                                                             |
| ------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| DAP server (new yx-core module) | stdio transport layer, core request handling, breakpoint manager (source line → IR node mapping)                                   |
| Runtime debug trait (yx-core)   | `DebugEngine` trait definition (pause, resume, step, get_frames, eval, get_variables)                                              |
| Interpreter                     | Breakpoint check in execution loop, pause/resume mechanism, frame linked list maintenance, `InterpreterDebugEngine` implementation |
| CLI                             | `yaoxiang run --debug` flag                                                                                                        |

**Acceptance criteria**: For any `.yx` file under `tests/yaoxiang/`, breakpoints can be set, Step
Over executed, and variable values viewed in VS Code.

### Phase 2: Advanced Debugging Capabilities

**Goal**: Expression evaluation, function breakpoints, concurrent debugging, exception breakpoints.

| Component                     | Change                                                                                                   |
| ----------------------------- | -------------------------------------------------------------------------------------------------------- |
| Expression evaluation engine  | Microprogram compilation (reuse parser + typechecker), temporary frame push to VM, side effect isolation |
| Concurrent debugging          | spawn task list mapping, breakpoint bound to task ID, stop-all / stop-this-only pause strategy           |
| Function/exception breakpoint | `setFunctionBreakpoints`, `setExceptionBreakpoints` mapping                                              |
| VS Code extension             | Provide default `launch.json` template                                                                   |

### Phase 3: JIT Debug & LLVM DWARF

**Goal**: JIT engine reuses DAP, LLVM produces DWARF for crash backtracing.

| Component | Change                                                                                                                                               |
| --------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| JIT       | Implement `DebugEngine` trait, generate variable→register mapping table at compile time, runtime frame linked list, expression temporary compilation |
| LLVM      | IR debug metadata → LLVM `DILocation` / `DISubprogram` → DWARF (no DAP interaction)                                                                  |

### Dependencies

```
Phase Zero (IR metadata)
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
| Type safety of expression evaluation      | Reuse existing typechecker, read-only references, no side effects committed                                      |
| DAP protocol details                      | Reference debugpy / delve implementations, the protocol is mature                                                |
| stop-all livelock in concurrent debugging | Timeout mechanism + forced pause                                                                                 |

## Trade-offs

### Advantages

- **One source**: Debug metadata is generated only once, shared by three engines. Avoids
  "interpreter debug info is correct but LLVM's is wrong"
- **Zero intrusion**: One flag, `--debug`; behavior without this flag is completely unchanged
- **DAP standard**: Connects directly to the VS Code ecosystem, no need for custom editor protocols
  or debugger UI
- **Interpreter-first**: Debugging is naturally suited for interpreters—flexible, controllable,
  simple expression evaluation. Not doing interactive debugging in LLVM mode is the most pragmatic
  choice

### Disadvantages

- **Debug mode performance is poor**: Interpreter is much slower than JIT/LLVM. But debugging
  doesn't need performance—no one expects debug mode to run production loads
- **LLVM debugging is limited**: AOT compilation cannot do interactive debugging, only GDB/LLDB +
  DWARF. But this is a trade-off: under LLVM mode, there should be no behavioral difference in
  debugging
- **Concurrent pause is complex**: Implementing stop-all semantics on the interpreter requires
  iterating over all active tasks

### Alternatives

| Approach                                           | Why not chosen                                                                                                              |
| -------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| Each of three engines implements DAP independently | Triple the work, triple the bugs. Violates "good taste"                                                                     |
| Only use DWARF, no proprietary DAP                 | Interpreter and JIT have no DWARF concept, LLDB can't enter VM internals                                                    |
| Build command-line debugger like Python pdb        | VS Code experience completely trumps command-line debugger                                                                  |
| Stuff DAP into the LSP process                     | Lifecycles are completely different—LSP follows project, DAP follows debug session. Process isolation is a hard requirement |

## Open Questions

- [ ] Is the expression syntax for conditional breakpoints completely consistent with normal
      YaoXiang? (Suggestion: completely consistent, reuse parser)
- [ ] Step Into behavior inside `spawn` block: when the user presses Step Into to enter a spawn
      block, which of the multiple parallel tasks should be displayed? (Suggestion: pause on the
      first created task)
- [ ] VS Code extension: should debug configuration go under the existing `vscode-extension/`
      directory or in a separate repository?

## References

- [RFC-024: spawn-block-based Concurrency Model](../accepted/024-concurrency-model.md)
- [RFC-027: Compile-time Predicates and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md)
- [RFC-028: JIT Compiler — Multi-level Execution Engine in VM](028-jit-compiler.md)
- [RFC-030: assert Assertion Mechanism](../accepted/030-assert-mechanism.md)
- [DAP Protocol Specification](https://microsoft.github.io/debug-adapter-protocol/)
- [debugpy — Python DAP Implementation Reference](https://github.com/microsoft/debugpy)
- [Delve — Go Debugger Reference](https://github.com/go-delve/delve)
