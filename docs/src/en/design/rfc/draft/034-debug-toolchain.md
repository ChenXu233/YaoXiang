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
as first-class citizens into YaoXiang IR, and the interpreter, JIT, and LLVM backends each consume
the same set of metadata. Users start a DAP (Debug Adapter Protocol) server via
`yaoxiang run --debug`, and VS Code connects through stdio to obtain a unified experience of
breakpoints, stepping, variable inspection, call stack, expression evaluation, and concurrent
debugging—regardless of the underlying execution engine.

## Motivation

### Why is this feature needed?

The current means of troubleshooting YaoXiang programs are extremely primitive:

```yaoxiang
io.println("DEBUG: x = " + x.to_string())
io.println("DEBUG: entered branch A")
```

Three fatal problems:

1. **Compiler development bootstrapping is blocked**: YaoXiang compilers are written in YaoXiang,
   but compiler authors cannot debug their own code. The lack of interactive debugging means the
   bootstrapping stage is a dead end.
2. **Three engines, zero debugging**: The interpreter, JIT, and LLVM each run independently. When
   something goes wrong, users can only check whether `ALL TESTS PASSED` appears in stdout.
   Assertion failure? No idea which line, no idea what the variable values are.
3. **Concurrency is a black box**: `spawn` creates multiple tasks. Which task is stuck? Who moved
   the variable? All down to guessing.

### Design Goals

- **Unified experience**: Breakpoints that work in the interpreter also work in JIT, and LLVM has
  consistent source mapping. Users don't perceive differences between underlying engines.
- **One source**: Debug metadata flows with the IR, no duplicate definitions, no maintenance of two
  sets of mappings.
- **Zero intrusion**: One parameter, `yaoxiang run --debug`. Without this parameter, compilation and
  execution behavior remains completely unchanged.
- **DAP standard**: Directly integrate with the VS Code ecosystem, no reinventing editor protocols.

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
│  │ Manager │  │  Manager  │  │      Engine       │    │
│  └─────────┘  └──────────┘  └───────────────────┘    │
└────────────────────────┬─────────────────────────────┘
                         │ Query/Control
┌────────────────────────▼─────────────────────────────┐
│              Runtime Debug Interface (trait)           │
│  pause / resume / step / get_frames / eval / ...    │
└────┬──────────────────┬──────────────────┬──────────┘
     │                  │                  │
┌────▼────┐    ┌───────▼───────┐    ┌─────▼──────┐
│Interpreter│    │ JIT (RFC-028)│    │ LLVM AOT   │
│ Directly  │    │ Generate     │    │ IR Metadata │
│ Consumes  │    │ Lightweight  │    │ → DWARF    │
│ IR Metadata│   │ Debug Tables │    │            │
└─────────┘    └───────────────┘    └────────────┘
```

**Key Design Decisions**:

1. **DAP server and runtime are decoupled through a trait**. The server doesn't care whether the
   underlying engine is an interpreter or JIT—it only issues commands through the `DebugEngine`
   trait. Each engine independently implements the same trait.
2. **`yaoxiang run --debug` forces use of the interpreter**. Debugging requires controllability, not
   performance. In LLVM mode, only DWARF is generated for post-mortem backtracing (core dump / crash
   report); no interactive debugging.
3. **Reuse entry discovery logic from `yaoxiang run`**. No new subcommands—the mental model is
   simply "run my program in debug mode."

### IR Debug Metadata

Attach metadata to existing YaoXiang IR without adding new IR variants. All metadata is generated in
**one place in the compilation frontend**, and backend consumption is read-only:

| Metadata         | Attachment Point                   | Description                                       |
| ---------------- | ---------------------------------- | ------------------------------------------------- |
| `SourceLocation` | Each IR node                       | Source file:line:column                           |
| `VarName`        | Variable declaration/binding nodes | Variable name in source code                      |
| `TypeAnnotation` | Variable/expression nodes          | Inferred type (including compile-time predicates) |
| `ScopeBoundary`  | Block/function entry-exit          | Lifecycle of variable scopes                      |
| `SpanInfo`       | spawn nodes                        | Task boundaries within spawn blocks               |

### Startup Flow

```
yaoxiang run --debug file.yx
    │
    ├── Phase 1: Compilation (with debug metadata)
    │   ├── Parse → AST
    │   ├── Type check + compile-time predicate verification
    │   └── Lower to IR (attach debug metadata)
    │
    ├── Phase 2: Use the interpreter engine
    │   └── In --debug mode, the interpreter is used regardless of --release
    │
    ├── Phase 3: Start DAP server
    │   ├── Initialize stdio transport channel
    │   ├── Wait for VS Code to attach
    │   ├── After successful attach, pause at program entry
    │   └── Enter interactive debugging loop
    │
    └── Phase 4: Program end / debug session end → exit
```

### Mode Differences

| Mode                     | Debugging Method                                                                      |
| ------------------------ | ------------------------------------------------------------------------------------- |
| `yaoxiang run --debug`   | Force interpreter, full-featured DAP interactive debugging                            |
| `yaoxiang run --release` | Generate DWARF for post-mortem backtracing (core dump / crash report), no DAP startup |
| `yaoxiang run` (normal)  | No debug metadata, no debugging support                                               |

## Detailed Design

### 1. Breakpoints

```
Breakpoint types:
├── Source line breakpoint  → Compilation frontend generates location metadata, backend queries matches
├── Function entry breakpoint → Triggers when function is called (Phase 2)
├── Conditional breakpoint   → Triggers when expression evaluates to true
└── Data breakpoint          → Triggers when variable is modified (Phase 2)
```

**Source line breakpoint core logic**:

```
VS Code sends: "Set breakpoint at file.yx:42"
    │
DAP server:
    ├── Query all IR nodes with SourceLocation == (file.yx, 42)
    ├── Forward to runtime: "Pause at these IR addresses"
    └── Runtime returns: Breakpoint ID

Program runs to IR node → Runtime checks: Is this node in the breakpoint list?
    ├── Regular breakpoint → Pause, notify DAP server
    └── Conditional breakpoint → Evaluate condition expression → pause only if true
```

**Three engine implementations**:

|                                   | Interpreter                        | JIT                                            | LLVM                                        |
| --------------------------------- | ---------------------------------- | ---------------------------------------------- | ------------------------------------------- |
| Breakpoint insertion              | Check IR node ID in execution loop | Insert `int3` in machine code via JIT          | Use LLVM DWARF + hardware breakpoints       |
| Conditional breakpoint evaluation | Directly interpret expression      | Temporarily JIT compile conditional expression | DWARF expression stack + evaluation         |
| Performance overhead              | One extra table lookup per IR node | Overhead only at breakpoint                    | Nearly zero overhead (hardware breakpoints) |

### 2. Stepping

```
Step Over    → Execute current line, skip inside function calls, stop at next line
Step Into    → Enter inside the function call on the current line
Step Out     → Execute until current function returns
Continue     → Resume execution until next breakpoint or program end
```

**Implementation logic**: Step operations are essentially **temporary breakpoints**. They share the
same mechanism with user-set breakpoints—not two systems, but two uses of one system.

```
Step Over:
    Current source line number = query_line(frame)
    → Set temporary breakpoint at next line
    → If current line is a function call: set temporary breakpoint after call site
    → Continue → hit temporary breakpoint → delete → pause

Step Into:
    First executable location of call target
    → Find source location of first IR node in function body
    → Set temporary breakpoint → Continue → hit → pause

Step Out:
    Return address of current stack frame
    → Find next line of caller
    → Set temporary breakpoint → Continue → hit → pause
```

**Four boundary cases for temporary breakpoints**:

1. **Concurrent attribution**: Temporary breakpoints are bound to the current task ID; other tasks
   that hit them are ignored directly.
2. **Step Over spawn block**: Stepping Over outside a spawn block means running through the entire
   spawn block, jumping to after it. To debug inside spawn, use Step Into.
3. **Temporary breakpoint not hit**: Set watchdog timeout (30 seconds without any breakpoint hit) →
   force pause → notify VS Code. Also listen for program exit event → cleanup immediately.
4. **Multiple IR nodes on the same line**: Step Over's temporary breakpoint is marked
   `ignore_current_line`; if the source line number equals the current line number on hit → ignore,
   continue.

### 3. Variable Inspection and Scopes

```
VS Code request: "List of variables for current frame"
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
│  Module-level bindings: constants,  │
│  type aliases, globals              │
├─ Locals ────────────────────────────┤
│  Local variables visible in current │
│  function                           │
│  ├── Parameters (function params)   │
│  └── Local bindings (let / assign)  │
├─ Captured ──────────────────────────┤
│  External variables captured by     │
│  spawn blocks / closures            │
│  Display ownership state:           │
│  moved / ref-shared                 │
└─────────────────────────────────────┘
```

**Engine differences**:

| Engine      | Variable Value Acquisition                                                                                       |
| ----------- | ---------------------------------------------------------------------------------------------------------------- |
| Interpreter | Directly read VM stack frame and heap. Each value has a clear representation in memory                           |
| JIT         | Values in registers and stack → need "variable → register/stack slot" mapping table recorded at JIT compile time |
| LLVM        | DWARF `.debug_info` section → `DW_AT_location` → native LLDB support                                             |

**Special type display**: Compile-time predicates refine type display for valuable debug
information:

```
x: Positive(x)  →  Display "Int (x > 0 = True)"
y: Sorted(y)    →  Display "Array(Int) (sort guarantee)"
result: T       →  Display the specific runtime type
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

Each frame records: function signature, call location (source file + line), local variables (lazy
evaluation), spawn context (task ID).

Both interpreter and JIT maintain their own frame linked lists. Frames aren't free to obtain—but
debug mode doesn't pursue zero overhead.

### 5. Expression Evaluation (Watch / REPL)

Users input any YaoXiang expression at a breakpoint:

```
Watch: x + y         → Return computed result
Watch: items[2].name → Access complex structures
Watch: f(x)          → Call function (side-effect risk)
```

**Evaluation strategy**:

```
User inputs expression
    │
├── Compilation frontend parses expression
├── Type check in current frame context
├── Variable values obtained from current frame (read-only references)
├── Expression executed as independent microprogram
│   └── No external variable modification allowed
│   └── No spawn allowed
│   └── No IO allowed (or optionally enabled)
└── Return result value → original frame state completely unchanged
```

**Interpreter is inherently a sandbox**: Expression evaluation isn't creating a new sandbox—the
interpreter itself is the sandbox. Expression evaluation just temporarily pushes a frame, destroys
it when done. Shares the same VM as normal execution, but commits no side effects.

**Function call evaluation**: Allowed by default, but warn the user "this expression may have side
effects" and require user confirmation before execution.

**Engine differences**:

| Engine      | Expression Evaluation                                                                     |
| ----------- | ----------------------------------------------------------------------------------------- |
| Interpreter | Reuse existing eval code path, inject current frame environment                           |
| JIT         | Temporarily compile expression → link to current frame → execute → discard temporary code |
| LLVM        | Not supported—LLVM mode does not support interactive debugging                            |

### 6. Concurrent Debugging

**Task model visibility**:

DAP's `threads` concept maps to YaoXiang's `spawn` tasks. Each task has its own stack frame linked
list and running state.

```
┌─ Threads ───────────────────────────┐
│  ● task-1  main()     file.yx:10   │ ← Currently focused
│  ▶ task-2  fetch()    file.yx:34   │ ← Running
│  ⏸ task-3  process()  file.yx:56   │ ← Paused at breakpoint
│  ◼ task-4  write()    finished       │
└─────────────────────────────────────┘
```

**Breakpoints in concurrent context**:

| Pause Mode           | Behavior                                      | Use Case                       |
| -------------------- | --------------------------------------------- | ------------------------------ |
| `stop-all` (default) | One task hits → all tasks pause               | Debug data races, global state |
| `stop-this-only`     | Only the hitting task pauses, others continue | Debug independent task logic   |

**Stepping semantics for spawn blocks**:

```
spawn {          // Step Over → run through entire spawn block
    task_a()     // Step Into → enter task_a
    task_b()     // Runs in parallel, unaffected by individual step
}
```

### 7. DAP Protocol Mapping

#### Phase 1: Core Requests

| DAP Request         | YaoXiang Semantics                                                             |
| ------------------- | ------------------------------------------------------------------------------ |
| `initialize`        | Capability negotiation: support breakpoints, stepping, variables, stack frames |
| `launch` / `attach` | Launch/attach to YaoXiang program (`--debug` uses attach mode)                 |
| `setBreakpoints`    | Set source line breakpoints                                                    |
| `configurationDone` | Breakpoints ready, begin execution                                             |
| `threads`           | Return list of all active spawn tasks                                          |
| `stackTrace`        | Return stack frame list for specified task                                     |
| `scopes`            | Return variable scopes for current frame                                       |
| `variables`         | Return variable list for specified scope                                       |
| `continue`          | Resume execution                                                               |
| `next`              | Step Over                                                                      |
| `stepIn`            | Step Into                                                                      |
| `stepOut`           | Step Out                                                                       |
| `pause`             | Interrupt all tasks                                                            |
| `evaluate`          | Evaluate expression in current frame                                           |
| `disconnect`        | End debug session                                                              |

#### Phase 2: Enhanced Requests

| DAP Request               | YaoXiang Semantics                                   |
| ------------------------- | ---------------------------------------------------- |
| `setFunctionBreakpoints`  | Function name breakpoint                             |
| `setExceptionBreakpoints` | Pause on error/panic                                 |
| `dataBreakpointInfo`      | Data breakpoint (triggered on variable modification) |

## Implementation Strategy

### Phase Zero: Infrastructure (Precedes All Phases)

**Goal**: Compilation frontend attaches debug metadata to IR.

| Component     | Changes                                                                |
| ------------- | ---------------------------------------------------------------------- |
| IR Definition | Add `SourceLocation`, `VarName`, `TypeAnnotation` etc. metadata fields |
| Parser        | Each AST node records source location                                  |
| TypeChecker   | Type information attached to IR nodes                                  |
| Testing       | Verify IR dump contains location and variable information              |

**Does not involve runtime.**

**Implementation progress (2026-09-17)**:

| Deliverable      | Status    | Implementation Form                                                                                                                                                                                                   |
| ---------------- | --------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source location  | Completed | All 76 `Instruction` variants carry `span` field; `span()` method intentionally has no wildcard arm, new variants missing span fail to compile. Location covers 40/41 instructions                                    |
| Variable name    | Completed | `LocalSlot { name, ty, scope_depth }` attached to `FunctionBody::Code::locals`; `register_local` writes in place during generation. `.42` debug section v2 carries names, v1 products backward-compatible for reading |
| Type information | Partial   | Slots already contain `ty`; `TypeAnnotation` independent metadata not done                                                                                                                                            |
| Dump visibility  | Completed | `dump` outputs `; <file>:<line>:<col>` per instruction, and lists `locals: name@slot`                                                                                                                                 |

The data form delivered in the above table directly interfaces with Phase 1: DAP's breakpoint
resolution consumes `debug_map`, variable panel consumes `local_names` (names) and `locals` (types).

### Phase 1: Interpreter DAP MVP

**Goal**: `yaoxiang run --debug file.yx` can set breakpoints, step, and view variables.

| Component                       | Changes                                                                                                                               |
| ------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| DAP server (yx-core new module) | stdio transport layer, core request handling, breakpoint manager (source line → IR node mapping)                                      |
| Runtime debug trait (yx-core)   | `DebugEngine` trait definition (pause, resume, step, get_frames, eval, get_variables)                                                 |
| Interpreter                     | Breakpoint checking in execution loop, pause/resume mechanism, frame linked list maintenance, `InterpreterDebugEngine` implementation |
| CLI                             | `yaoxiang run --debug` parameter                                                                                                      |

**Acceptance Criteria**: For any `.yx` file under `tests/yaoxiang/`, users can set breakpoints, Step
Over, and view variable values with VS Code.

### Phase 2: Advanced Debugging Capabilities

**Goal**: Expression evaluation, function breakpoints, concurrent debugging, exception breakpoints.

| Component                      | Changes                                                                                                    |
| ------------------------------ | ---------------------------------------------------------------------------------------------------------- |
| Expression evaluation engine   | Microprogram compilation (reuse parser + typechecker), temporary frame push into VM, side effect isolation |
| Concurrent debugging           | spawn task list mapping, breakpoints bound to task ID, stop-all / stop-this-only pause strategy            |
| Function/exception breakpoints | `setFunctionBreakpoints`, `setExceptionBreakpoints` mapping                                                |
| VS Code extension              | Provide default `launch.json` template                                                                     |

### Phase 3: JIT Debugging & LLVM DWARF

**Goal**: JIT engine reuses DAP, LLVM produces DWARF for crash backtracing.

| Component | Changes                                                                                                                                              |
| --------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| JIT       | Implement `DebugEngine` trait, generate variable→register mapping table at compile time, runtime frame linked list, temporary expression compilation |
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

| Risk                                      | Mitigation                                                                                                      |
| ----------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| Complexity of interpreter pause mechanism | Use simple channel/signal rather than complex state machines, pausing is just not fetching the next instruction |
| Type safety of expression evaluation      | Reuse existing typechecker, read-only references, no side effects committed                                     |
| DAP protocol details                      | Reference debugpy / delve implementations, the protocol is mature                                               |
| stop-all livelock in concurrent debugging | Timeout mechanism + force pause                                                                                 |

## Trade-offs

### Advantages

- **One source**: Debug metadata is generated only once, shared by three engines. No "interpreter
  debug info is correct but LLVM's is wrong"
- **Zero intrusion**: One `--debug` parameter; behavior is completely unchanged without it
- **DAP standard**: Directly integrates with the VS Code ecosystem, no custom editor protocol or
  debugger UI
- **Interpreter first**: Debugging is naturally suited to interpreters—flexible, controllable,
  expression evaluation simple. Not doing interactive debugging in LLVM mode is the most pragmatic
  choice

### Disadvantages

- **Poor performance in debug mode**: Interpreters are much slower than JIT/LLVM. But debugging
  doesn't need performance—no one expects debug mode to run production loads
- **Limited LLVM debugging**: AOT compilation doesn't support interactive debugging, only GDB/LLDB +
  DWARF. But this is a trade-off: there shouldn't be debug behavior differences in LLVM mode
- **Complex concurrent pausing**: Implementing stop-all semantics on the interpreter requires
  traversing all active tasks

### Alternatives

| Alternative                           | Why Not Chosen                                                                                                              |
| ------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| Three engines each implement DAP      | Triple the work, triple the bugs. Violates "good taste"                                                                     |
| Only DWARF, no custom DAP             | Interpreters and JIT have no DWARF concept, LLDB can't enter VM internals                                                   |
| Command-line debugger like Python pdb | VS Code experience completely dominates command-line debuggers                                                              |
| Stuff DAP into LSP process            | Lifecycles are completely different—LSP follows project, DAP follows debug session. Process isolation is a hard requirement |

## Open Questions

- [ ] Is conditional breakpoint expression syntax completely identical to normal YaoXiang?
      (Suggestion: completely identical, reuse parser)
- [ ] Step Into behavior inside `spawn` blocks: when user presses Step Into to enter a spawn block,
      which of the multiple parallel tasks should be displayed? (Suggestion: pause on the first
      created task)
- [ ] VS Code extension: should debug configuration be in the existing `vscode-extension/` directory
      or a separate repository?

## References

- [RFC-024: spawn block-based concurrency model](../accepted/024-concurrency-model.md)
- [RFC-027: compile-time predicates and unified static verification](../accepted/027-compile-time-evaluation-types.md)
- [RFC-028: JIT compiler — multi-level execution engine in VM](../draft/028-jit-compiler.md)
- [RFC-030: assert mechanism](../review/030-assert-mechanism.md)
- [DAP Protocol Specification](https://microsoft.github.io/debug-adapter-protocol/)
- [debugpy — Python DAP implementation reference](https://github.com/microsoft/debugpy)
- [Delve — Go debugger reference](https://github.com/go-delve/delve)
