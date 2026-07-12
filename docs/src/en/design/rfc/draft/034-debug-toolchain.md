---
title: "RFC-034: Unified Debugging Toolchain"
status: "Draft"
author: "Chenxu"
created: "2026-07-06"
updated: "2026-07-06"
---

# RFC-034: Unified Debugging Toolchain

## Summary

Introduce a unified debugging toolchain for YaoXiang. The core design is **one source, three consumers**: the compilation front-end embeds source locations, variable names, and type information as first-class citizens into YaoXiang IR; the interpreter, JIT, and LLVM backends each consume the same metadata. Users launch a DAP (Debug Adapter Protocol) server via `yaoxiang run --debug`, and VS Code connects over stdio, gaining a unified experience of breakpoints, stepping, variable inspection, call stacks, expression evaluation, and concurrent debugging—regardless of the underlying execution engine.

## Motivation

### Why is this feature needed?

The current means of debugging YaoXiang programs are extremely primitive:

```yaoxiang
io.println("DEBUG: x = " + x.to_string())
io.println("DEBUG: entered branch A")
```

Three fatal problems:

1. **Compiler self-hosting is blocked**: YaoXiang is used to write the YaoXiang compiler, but the people writing the compiler cannot debug their own code. The lack of interactive debugging during the bootstrapping phase is a dead end.
2. **Three engines, zero debugging**: The interpreter, JIT, and LLVM each run their own course. When something goes wrong, users can only check whether `ALL TESTS PASSED` appears in stdout. An assertion failed? You don't know which line, you don't know the variable value.
3. **Concurrency is a black box**: `spawn` creates multiple tasks—which one hung? Who moved a variable? Pure guesswork.

### Design Goals

- **Unified experience**: A breakpoint that works in the interpreter also works in JIT, and LLVM has consistent source mapping. Users don't perceive differences in the underlying engine.
- **One source**: Debug metadata flows with the IR—no redundant definitions, no maintaining two sets of mappings.
- **Zero intrusion**: One parameter, `yaoxiang run --debug`. Without this parameter, compilation and execution behavior remains completely unchanged.
- **DAP standard**: Connect directly to the VS Code ecosystem, don't reinvent editor protocols.

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
│  │ Session  │  │ Breakpoint│  │ Expression Eval   │    │
│  │ Manager  │  │  Manager  │  │      Engine       │    │
│  └─────────┘  └──────────┘  └───────────────────┘    │
└────────────────────────┬─────────────────────────────┘
                         │ query/control
┌────────────────────────▼─────────────────────────────┐
│              Runtime Debug Interface (trait)           │
│  pause / resume / step / get_frames / eval / ...    │
└────┬──────────────────┬──────────────────┬──────────┘
     │                  │                  │
┌────▼────┐    ┌───────▼───────┐    ┌─────▼──────┐
│Interpreter│    │ JIT (RFC-028)│    │  LLVM AOT  │
│Direct IR  │    │ Generates    │    │ IR metadata│
│metadata   │    │ lightweight  │    │ → DWARF    │
│consumer   │    │ debug tables │    │            │
└─────────┘    └───────────────┘    └────────────┘
```

**Key design decisions**:

1. **DAP server and runtime are decoupled through a trait**. The server doesn't care whether the backend is an interpreter or JIT—it only issues commands through the `DebugEngine` trait. Each engine independently implements the same trait.
2. **`yaoxiang run --debug` forces the use of the interpreter**. Debugging requires controllability, not performance. In LLVM mode, only DWARF is generated for post-mortem analysis (core dump / crash report), no interactive debugging.
3. **Reuse entry-point discovery logic from `yaoxiang run`**. No new subcommand is introduced—the mental model is simply "run my program in debug mode".

### IR Debug Metadata

Attach metadata to the existing YaoXiang IR without adding new IR variants. All metadata is generated in **one place—the compilation front-end**—and backend consumption is read-only:

| Metadata | Attachment Point | Description |
|----------|------------------|-------------|
| `SourceLocation` | Every IR node | Source file:line:column |
| `VarName` | Variable declaration/binding node | Variable name in source code |
| `TypeAnnotation` | Variable/expression node | Inferred type (including compile-time predicates) |
| `ScopeBoundary` | Block/function entry and exit | Variable scope lifecycle |
| `SpanInfo` | spawn node | Task boundaries within spawn blocks |

### Startup Flow

```
yaoxiang run --debug file.yx
    │
    ├── Phase 1: Compilation (with debug metadata)
    │   ├── Parse → AST
    │   ├── Type checking + compile-time predicate verification
    │   └── Lower to IR (with debug metadata attached)
    │
    ├── Phase 2: Use the interpreter engine
    │   └── In --debug mode, the interpreter is used regardless of --release
    │
    ├── Phase 3: Launch DAP server
    │   ├── Initialize stdio transport channel
    │   ├── Wait for VS Code attach
    │   ├── Pause at program entry after successful attach
    │   └── Enter interactive debug loop
    │
    └── Phase 4: Program ends / debug session ends → exit
```

### Mode Differences

| Mode | Debugging Method |
|------|------------------|
| `yaoxiang run --debug` | Forces interpreter, full-featured DAP interactive debugging |
| `yaoxiang run --release` | Generates DWARF for post-mortem analysis (core dump / crash report), no DAP |
| `yaoxiang run` (normal) | No debug metadata, no debugging support |

## Detailed Design

### 1. Breakpoints

```
Breakpoint types:
├── Source line breakpoint    → Compilation front-end generates location metadata, backend queries match
├── Function entry breakpoint → Triggered on function call (Phase 2)
├── Conditional breakpoint    → Triggered when expression evaluates to true
└── Data breakpoint           → Triggered when variable is modified (Phase 2)
```

**Source line breakpoint core logic**:

```
VS Code sends: "Set breakpoint at file.yx:42"
    │
DAP server:
    ├── Query all IR nodes with SourceLocation == (file.yx, 42)
    ├── Forward to runtime: "Pause at these IR addresses"
    └── Runtime returns: breakpoint ID

Program runs to IR node → Runtime checks: Is this node in the breakpoint list?
    ├── Regular breakpoint → Pause, notify DAP server
    └── Conditional breakpoint → Evaluate condition expression → pause only if true
```

**Three engine implementations**:

| | Interpreter | JIT | LLVM |
|---|---|---|---|
| Breakpoint insertion | Check IR node ID in execution loop | JIT inserts `int3` in machine code | Use LLVM DWARF + hardware breakpoints |
| Conditional breakpoint evaluation | Interpret expression directly | Temporarily JIT-compile condition expression | DWARF expression stack + evaluation |
| Performance overhead | One extra lookup per IR node | Overhead only at breakpoint | Nearly zero overhead (hardware breakpoints) |

### 2. Stepping

```
Step Over    → Execute current line, skip inside function calls, stop at next line
Step Into    → Enter inside the function call on the current line
Step Out     → Execute until current function returns
Continue     → Resume execution until next breakpoint or program end
```

**Implementation logic**: Stepping operations are essentially **temporary breakpoints**. They share the same mechanism as user-set breakpoints—not two systems, but two uses of one system.

```
Step Over:
    Current source line number = query_line(frame)
    → Set temporary breakpoint at next line
    → If current line is a function call: set temporary breakpoint after the call site
    → Continue → hit temporary breakpoint → delete → pause

Step Into:
    First executable line of current call target
    → Find source location of first IR node in function body
    → Set temporary breakpoint → Continue → hit → pause

Step Out:
    Return address of current stack frame
    → Find caller's next line
    → Set temporary breakpoint → Continue → hit → pause
```

**Four edge case handling for temporary breakpoints**:

1. **Concurrent ownership**: Temporary breakpoints are bound to the current task ID; other tasks that hit them simply ignore them.
2. **Step Over spawn blocks**: Stepping Over a spawn block from outside is equivalent to running through the entire spawn block, jumping to afterward. To debug inside a spawn block, use Step Into.
3. **Temporary breakpoint not hit**: Set a watchdog timeout (30 seconds without any breakpoint hit) → force pause → notify VS Code. Simultaneously monitor program exit events → clean up immediately.
4. **Multiple IR nodes on the same line**: Step Over's temporary breakpoint is marked `ignore_current_line`; if the hit's source line number equals the current line number → ignore and continue.

### 3. Variable Inspection and Scopes

```
VS Code requests: "Variable list for the current frame"
    │
DAP server:
    ├── Query IR node of current pause point
    ├── Iterate variable bindings within the current ScopeBoundary
    │   └── Each binding returns: (name, type, runtime value reference)
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
│  ├── Parameters (function params)   │
│  └── Local bindings (let / assign)  │
├─ Captured ──────────────────────────┤
│  External variables captured by     │
│  spawn blocks / closures            │
│  Display ownership state: moved /   │
│  ref-shared                         │
└─────────────────────────────────────┘
```

**Engine differences**:

| Engine | Variable Value Acquisition |
|--------|---------------------------|
| Interpreter | Directly read VM stack frames and heap. Every value has a clear representation in memory |
| JIT | Values in registers and on stack → need JIT compilation to record "variable → register/stack slot" mapping table |
| LLVM | DWARF `.debug_info` section → `DW_AT_location` → LLDB native support |

**Special type display**: Refined types from compile-time predicates display information valuable for debugging:

```
x: Positive(x)  →  Display "Int (x > 0 = True)"
y: Sorted(y)    →  Display "Array(Int) (sorted guarantee)"
result: T       →  Display the concrete runtime type
```

### 4. Call Stacks

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

Each frame records: function signature, call location (source file + line number), local variables (lazy evaluation), spawn context (task ID).

The interpreter and JIT each maintain their own frame linked list. Frames are not zero-cost to obtain—but debug mode does not pursue zero overhead.

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
├── Compilation front-end parses expression
├── Type-check in current frame context
├── Variable values obtained from current frame (read-only references)
├── Execute expression as a standalone micro-program
│   └── Not allowed to modify external variables
│   └── Not allowed to spawn
│   └── Not allowed to perform IO (or optionally enabled)
└── Return result value → original frame state completely unchanged
```

**Interpreter's natural sandbox**: Expression evaluation doesn't create a new sandbox—the interpreter itself is the sandbox. Expression evaluation is just temporarily pushing a frame, destroying it when done. It shares the same VM as normal execution, but commits no side effects.

**Function call evaluation**: Allowed by default, but warns the user "this expression may have side effects" and requires user confirmation to execute.

**Engine differences**:

| Engine | Expression Evaluation |
|--------|----------------------|
| Interpreter | Reuse existing eval code path, inject current frame environment |
| JIT | Temporarily compile expression → link to current frame → execute → discard temporary code |
| LLVM | Not supported—LLVM mode does not do interactive debugging |

### 6. Concurrent Debugging

**Task model visibility**:

The DAP `threads` concept maps to YaoXiang's `spawn` tasks. Each task has its own stack frame linked list and running state.

```
┌─ Threads ───────────────────────────┐
│  ● task-1  main()     file.yx:10   │ ← Currently focused
│  ▶ task-2  fetch()    file.yx:34   │ ← Running
│  ⏸ task-3  process()  file.yx:56   │ ← Paused at breakpoint
│  ◼ task-4  write()    finished      │
└─────────────────────────────────────┘
```

**Breakpoints in concurrent context**:

| Pause Mode | Behavior | Use Case |
|------------|----------|----------|
| `stop-all` (default) | One task hits → all tasks pause | Debug data races, global state |
| `stop-this-only` | Pause only the hitting task, others continue | Debug independent task logic |

**Stepping semantics for spawn blocks**:

```
spawn {          // Step Over → run through entire spawn block
    task_a()     // Step Into → enter task_a
    task_b()     // Runs in parallel, unaffected by individual steps
}
```

### 7. DAP Protocol Mapping

#### Phase 1: Core Requests

| DAP Request | YaoXiang Semantics |
|-------------|--------------------|
| `initialize` | Capability negotiation: supports breakpoints, stepping, variables, stack frames |
| `launch` / `attach` | Launch/attach to YaoXiang program (`--debug` uses attach mode) |
| `setBreakpoints` | Set source line breakpoints |
| `configurationDone` | Breakpoints ready, begin execution |
| `threads` | Return list of all active spawn tasks |
| `stackTrace` | Return stack frame list for specified task |
| `scopes` | Return variable scopes for current frame |
| `variables` | Return variable list for specified scope |
| `continue` | Resume execution |
| `next` | Step Over |
| `stepIn` | Step Into |
| `stepOut` | Step Out |
| `pause` | Interrupt all tasks |
| `evaluate` | Evaluate expression in current frame |
| `disconnect` | End debug session |

#### Phase 2: Enhanced Requests

| DAP Request | YaoXiang Semantics |
|-------------|--------------------|
| `setFunctionBreakpoints` | Function name breakpoints |
| `setExceptionBreakpoints` | Pause on error/panic |
| `dataBreakpointInfo` | Data breakpoints (triggered on variable modification) |

## Implementation Strategy

### Phase Zero: Infrastructure (Precedes All Other Phases)

**Goal**: Compilation front-end attaches debug metadata to IR.

| Component | Change |
|-----------|--------|
| IR definition | Add `SourceLocation`, `VarName`, `TypeAnnotation` and other metadata fields |
| Parser | Each AST node records source location |
| TypeChecker | Type information attached to IR nodes |
| Tests | Verify IR dump contains location and variable information |

**Runtime is not involved.**

### Phase 1: Interpreter DAP MVP

**Goal**: `yaoxiang run --debug file.yx` can set breakpoints, step, and view variables.

| Component | Change |
|-----------|--------|
| DAP server (new yx-core module) | stdio transport layer, core request handling, breakpoint manager (source line → IR node mapping) |
| Runtime debug trait (yx-core) | `DebugEngine` trait definition (pause, resume, step, get_frames, eval, get_variables) |
| Interpreter | Breakpoint check in execution loop, pause/resume mechanism, frame linked list maintenance, `InterpreterDebugEngine` implementation |
| CLI | `yaoxiang run --debug` parameter |

**Acceptance criteria**: For any `.yx` file under `tests/yaoxiang/`, VS Code can be used to set breakpoints, Step Over, and view variable values.

### Phase 2: Advanced Debugging Capabilities

**Goal**: Expression evaluation, function breakpoints, concurrent debugging, exception breakpoints.

| Component | Change |
|-----------|--------|
| Expression evaluation engine | Micro-program compilation (reuse parser + typechecker), temporary frame push into VM, side effect isolation |
| Concurrent debugging | spawn task list mapping, breakpoint binding to task ID, stop-all / stop-this-only pause strategy |
| Function/exception breakpoints | `setFunctionBreakpoints`, `setExceptionBreakpoints` mapping |
| VS Code extension | Provide default `launch.json` template |

### Phase 3: JIT Debugging & LLVM DWARF

**Goal**: JIT engine reuses DAP, LLVM produces DWARF for crash backtracing.

| Component | Change |
|-----------|--------|
| JIT | Implement `DebugEngine` trait, generate variable→register mapping table at compile time, runtime frame linked list, temporary compilation for expressions |
| LLVM | IR debug metadata → LLVM `DILocation` / `DISubprogram` → DWARF (no DAP interaction) |

### Dependency Relationships

```
Phase Zero (IR metadata)
    ↓
Phase 1 (Interpreter DAP MVP)  ← Usable starting here
    ↓
Phase 2 (Advanced capabilities)
    ↓
Phase 3 (JIT + LLVM DWARF)
```

### Risks

| Risk | Mitigation |
|------|------------|
| Complexity of interpreter pause mechanism | Use simple channel/signal instead of complex state machines—pausing is just not fetching the next instruction |
| Type safety of expression evaluation | Reuse existing typechecker, read-only references, no side effects committed |
| DAP protocol details | Reference debugpy / delve implementations—the protocol is mature |
| stop-all livelock in concurrent debugging | Timeout mechanism + force pause |

## Trade-offs

### Advantages

- **One source**: Debug metadata is generated only once, shared by three engines. No scenario where "interpreter debug info is correct but LLVM's is wrong"
- **Zero intrusion**: One `--debug` parameter, behavior is completely unchanged without it
- **DAP standard**: Direct integration with VS Code ecosystem, no custom editor protocol or debugger UI required
- **Interpreter-first**: Debugging is naturally suited to interpreters—flexible, controllable, simple expression evaluation. The choice not to do interactive debugging in LLVM mode is the most pragmatic option

### Disadvantages

- **Poor debug mode performance**: The interpreter is much slower than JIT/LLVM. But debugging doesn't need performance—no one expects debug mode to handle production load
- **Limited LLVM debugging**: AOT compilation cannot support interactive debugging, only GDB/LLDB + DWARF. But this is a trade-off: there shouldn't be debugging behavior differences in LLVM mode anyway
- **Complexity of concurrent pause**: Implementing stop-all semantics on the interpreter requires iterating over all active tasks

### Alternative Approaches

| Approach | Why Not Chosen |
|----------|----------------|
| Three engines each implement DAP | Triple the work, triple the bugs. Violates "good taste" |
| Use only DWARF, no proprietary DAP | Interpreter and JIT have no DWARF concept; LLDB cannot penetrate into VM internals |
| Build a command-line debugger like Python pdb | VS Code experience completely outclasses command-line debuggers |
| Embed DAP into the LSP process | Lifecycles are completely different—LSP follows the project, DAP follows debug sessions. Process isolation is a hard requirement |

## Open Questions

- [ ] Should conditional breakpoint expression syntax be exactly identical to normal YaoXiang? (Suggestion: identical, reuse parser)
- [ ] Step Into behavior inside `spawn` blocks: when a user presses Step Into to enter a spawn block, which of the multiple parallel tasks should be displayed? (Suggestion: pause at the first created task)
- [ ] VS Code extension: should the debug configuration go in the existing `vscode-extension/` directory or a separate repository?

## References

- [RFC-024: spawn Block-Based Concurrency Model](../accepted/024-concurrency-model.md)
- [RFC-027: Compile-Time Predicates and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md)
- [RFC-028: JIT Compiler — Multi-Level Execution Engine within VM](../draft/028-jit-compiler.md)
- [RFC-030: assert Mechanism](../review/030-assert-mechanism.md)
- [DAP Protocol Specification](https://microsoft.github.io/debug-adapter-protocol/)
- [debugpy — Python DAP Implementation Reference](https://github.com/microsoft/debugpy)
- [Delve — Go Debugger Reference](https://github.com/go-delve/delve)