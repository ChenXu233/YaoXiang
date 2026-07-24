---
title: "RFC-034: Unified Debugging Toolchain"
status: "Draft"
author: "Chenxu"
created: "2026-07-06"
updated: "2026-07-06"
issue: "#164"
---

# RFC-034: Unified Debugging Toolchain

## Summary

Introduce a unified debugging toolchain for YaoXiang. The core design is **one source, three consumers**: the compilation frontend embeds source locations, variable names, and type information as first-class citizens into the YaoXiang IR, and the three backends (interpreter, JIT, LLVM) each consume the same set of metadata. Users launch the DAP (Debug Adapter Protocol) server via `yaoxiang run --debug`, VS Code connects via stdio, and obtains a unified experience of breakpoints, stepping, variable viewing, call stacks, expression evaluation, and concurrent debugging—no matter which execution engine is underneath.

## Motivation

### Why is this feature needed?

The current means of troubleshooting YaoXiang program errors are extremely primitive:

```yaoxiang
io.println("DEBUG: x = " + x.to_string())
io.println("DEBUG: entered branch A")
```

Three fatal problems:

1. **Compiler bootstrap is hindered**: Write the YaoXiang compiler in YaoXiang, but those writing the compiler cannot debug their own code. The lack of interactive debugging means in the bootstrap stage is a dead end.
2. **Three engines, zero debugging**: The interpreter, JIT, and LLVM each run their own. When something goes wrong, users can only check whether `ALL TESTS PASSED` appears in stdout. Assertion failure? Don't know which line, don't know the variable values.
3. **Concurrency is a black box**: `spawn` creates multiple tasks, which task hung? Who moved the variable? All guesswork.

### Design Goals

- **Unified Experience**: Breakpoints that work in the interpreter also work in JIT, and LLVM also has consistent source mapping. Users don't perceive the underlying engine differences.
- **One Source**: Debugging metadata flows with the IR, no redundant definitions, no need to maintain two sets of mappings.
- **Zero Intrusion**: `yaoxiang run --debug` is a single argument; without it, compilation and execution behavior is completely unchanged.
- **DAP Standard**: Directly integrates with the VS Code ecosystem, no need to reinvent the editor protocol.

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
│  │ Session │  │Breakpoint│  │ Expression Eval   │    │
│  │   Mgmt  │  │   Mgmt   │  │      Engine       │    │
│  └─────────┘  └──────────┘  └───────────────────┘    │
└────────────────────────┬─────────────────────────────┘
                         │ Query / Control
┌────────────────────────▼─────────────────────────────┐
│              Runtime Debug Interface (trait)           │
│  pause / resume / step / get_frames / eval / ...    │
└────┬──────────────────┬──────────────────┬──────────┘
     │                  │                  │
┌────▼────┐    ┌───────▼───────┐    ┌─────▼──────┐
│Interpreter│   │ JIT (RFC-028) │   │  LLVM AOT  │
│ Directly │    │  Generates    │    │ IR metadata│
│Consumes  │    │ Lightweight   │    │   → DWARF  │
│IR metadata│   │  Debug Tables │    │            │
└─────────┘    └───────────────┘    └────────────┘
```

**Key Design Decisions**:

1. **The DAP server and runtime are decoupled via trait**. The server doesn't care whether the underlying is an interpreter or JIT—it issues commands only through the `DebugEngine` trait. Each engine independently implements the same trait.
2. **`yaoxiang run --debug` forces the use of the interpreter**. Debugging needs controllability, not performance. In LLVM mode, only DWARF is generated for post-hoc backtracing (core dump / crash report), without interactive debugging.
3. **Reuse entry discovery logic from `yaoxiang run`**. No new subcommands are introduced; the mental model is "run my program in debug mode".

### IR Debugging Metadata

Attach metadata to the existing YaoXiang IR, without adding new IR types. All metadata is generated in one place at the **compilation frontend**, and backend consumption is read-only:

| Metadata | Attachment Point | Description |
|----------|------------------|-------------|
| `SourceLocation` | Every IR node | source file:line:column |
| `VarName` | Variable declaration / binding node | variable name in source code |
| `TypeAnnotation` | Variable / expression node | inferred type (including compile-time predicates) |
| `ScopeBoundary` | Block / function entry & exit | lifecycle of variable scope |
| `SpanInfo` | spawn node | task boundary within spawn block |

### Startup Process

```
yaoxiang run --debug file.yx
    │
    ├── Phase 1: Compilation (with debugging metadata)
    │   ├── Parsing → AST
    │   ├── Type checking + compile-time predicate validation
    │   └── Lowering to IR (attaching debugging metadata)
    │
    ├── Phase 2: Use the interpreter engine
    │   └── In --debug mode, regardless of --release, the interpreter is used
    │
    ├── Phase 3: Launch DAP server
    │   ├── Initialize stdio transport channel
    │   ├── Wait for VS Code to attach
    │   ├── Pause at program entry after successful attach
    │   └── Switch to interactive debug loop
    │
    └── Phase 4: Program ends / debug session ends → exit
```

### Differences Between Modes

| Mode | Debugging Method |
|------|------------------|
| `yaoxiang run --debug` | Force interpreter, full-feature DAP interactive debugging |
| `yaoxiang run --release` | Generate DWARF for post-hoc backtracing (core dump / crash report), do not launch DAP |
| `yaoxiang run` (normal) | No debugging metadata, no debugging support |

## Detailed Design

### 1. Breakpoints

```
Breakpoint Types:
├── Source line breakpoint   → Compilation frontend generates location metadata, backend queries and matches
├── Function entry breakpoint → Triggered on function call (Phase Two)
├── Conditional breakpoint   → Triggered when expression evaluates to true
└── Data breakpoint          → Triggered when variable is modified (Phase Two)
```

**Source line breakpoint core logic**:

```
VS Code sends: "Set breakpoint at file.yx:42"
    │
DAP Server:
    ├── Query all IR nodes with SourceLocation == (file.yx, 42)
    ├── Forward to runtime: "Pause at these IR addresses"
    └── Runtime returns: breakpoint ID

Program runs to IR node → Runtime checks: Is this node in the breakpoint list?
    ├── Regular breakpoint   → Pause, notify DAP server
    └── Conditional breakpoint → Evaluate condition expression → pause only if true
```

**Three engine implementations**:

| | Interpreter | JIT | LLVM |
|---|---|---|---|
| Breakpoint insertion | Check IR node ID in execution loop | JIT inserts `int3` into machine code | Use LLVM DWARF + hardware breakpoints |
| Conditional breakpoint evaluation | Directly interpret expression | Temporarily JIT compile condition expression | DWARF expression stack + evaluation |
| Performance overhead | One extra table lookup per IR node | Overhead only at breakpoint | Nearly zero overhead (hardware breakpoint) |

### 2. Step Execution

```
Step Over    → Execute current line, skip inside function calls, stop at next line
Step Into    → Enter inside the function call on the current line
Step Out     → Execute until the current function returns
Continue     → Resume execution until the next breakpoint or program end
```

**Implementation Logic**: Step operations are essentially **temporary breakpoints**. They share the same mechanism with user-explicit breakpoints; they are not two systems, but two usages of one system.

```
Step Over:
    Current source line number = query_line(frame)
    → Set a temporary breakpoint on the next line
    → If the current line is a function call: set a temporary breakpoint after the call site
    → Continue → hit temporary breakpoint → delete → pause

Step Into:
    First executable position of the current call target
    → Find the source position of the first IR node in the function body
    → Set temporary breakpoint → Continue → hit → pause

Step Out:
    Return address of the current stack frame
    → Find the caller's next line
    → Set temporary breakpoint → Continue → hit → pause
```

**Four edge case handling for temporary breakpoints**:

1. **Concurrency ownership**: Temporary breakpoints are bound to the current task ID; hits by other tasks are directly ignored.
2. **Step Over spawn block**: Step Over outside a spawn block means running the entire spawn block to completion, jumping to after it. To debug inside a spawn block, use Step Into.
3. **Temporary breakpoint not hit**: Set a watchdog timeout (30 seconds without any breakpoint hit) → force pause → notify VS Code. Also listen for program exit events → clean up immediately.
4. **Multiple IR nodes on the same line**: Step Over's temporary breakpoint is marked `ignore_current_line`; when hit, if the source line number equals the current line number → ignore and continue.

### 3. Variable Inspection and Scope

```
VS Code request: "List of variables in the current frame"
    │
DAP Server:
    ├── Query the IR node of the current pause point
    ├── Iterate over variable bindings within the current ScopeBoundary
    │   └── Each binding returns: (name, type, runtime value reference)
    └── Assemble VariablesResponse → VS Code
```

**Scope Layering**:

```
┌─ Globals ───────────────────────────┐
│  Module-level bindings: constants,  │
│  type aliases, globals              │
├─ Locals ────────────────────────────┤
│  Local variables visible within     │
│  the current function               │
│  ├── Parameters (function args)     │
│  └── Local bindings (let / assign)  │
├─ Captured ──────────────────────────┤
│  External variables captured by     │
│  spawn block / closure              │
│  Show ownership state: moved /      │
│  ref shared                         │
└─────────────────────────────────────┘
```

**Engine Differences**:

| Engine | Variable Value Retrieval |
|--------|--------------------------|
| Interpreter | Directly read VM stack frame and heap. Each value has a clear representation in memory |
| JIT | Values on registers and stack → need to record "variable → register/stack slot" mapping table at JIT compile time |
| LLVM | DWARF's `.debug_info` section → `DW_AT_location` → natively supported by LLDB |

**Special Type Display**: Compile-time predicates refine type display to show debugging-useful information:

```
x: Positive(x)  →  displays "Int (x > 0 = True)"
y: Sorted(y)    →  displays "Array(Int) (sorted guarantee)"
result: T       →  displays the concrete type at runtime
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
│ #1  main()          file.yx:67  │  ← Caller
│     locals: data = ["hello", ...]│
├──────────────────────────────────┤
│ #2  <entry>         file.yx:1   │  ← Root
└──────────────────────────────────┘
```

Each frame records: function signature, call location (source file + line number), local variables (lazy evaluation), spawn context (task ID).

The interpreter and JIT each maintain their own frame linked list. Retrieving frames is not zero-cost—but debug mode does not pursue zero overhead.

### 5. Expression Evaluation (Watch / REPL)

The user enters any YaoXiang expression at a breakpoint:

```
Watch: x + y         → Returns the calculation result
Watch: items[2].name → Accesses complex structures
Watch: f(x)          → Calls a function (risk of side effects)
```

**Evaluation Strategy**:

```
User enters expression
    │
├── Compilation frontend parses the expression
├── Type check in the context of the current frame
├── Variable values obtained from the current frame (read-only reference)
├── Expression executed as an independent micro-program
│   └── Not allowed to modify external variables
│   └── Not allowed to spawn
│   └── Not allowed to do IO (or optionally enabled)
└── Return result value → original frame state completely unchanged
```

**Interpreter Natural Sandbox**: Expression evaluation doesn't create a new sandbox—the interpreter itself is a sandbox. Expression evaluation just temporarily pushes a frame, which is destroyed when done. It shares the same VM as normal execution, but commits no side effects.

**Function Call Evaluation**: Allowed by default, but warn the user that "this expression may have side effects", requiring user confirmation before execution.

**Engine Differences**:

| Engine | Expression Evaluation |
|--------|----------------------|
| Interpreter | Reuse existing eval code path, inject current frame environment |
| JIT | Temporarily compile expression → link to current frame → execute → discard temporary code |
| LLVM | Not supported—LLVM mode does not do interactive debugging |

### 6. Concurrent Debugging

**Task Model Visibility**:

DAP's `threads` concept maps to YaoXiang's `spawn` tasks. Each task has its own stack frame list and running state.

```
┌─ Threads ───────────────────────────┐
│  ● task-1  main()     file.yx:10   │ ← Currently focused
│  ▶ task-2  fetch()    file.yx:34   │ ← Running
│  ⏸ task-3  process()  file.yx:56   │ ← Paused at breakpoint
│  ◼ task-4  write()    Finished     │
└─────────────────────────────────────┘
```

**Breakpoints in Concurrent Context**:

| Pause Mode | Behavior | Applicable Scenario |
|------------|----------|---------------------|
| `stop-all` (default) | One task hits → all tasks pause | Debugging data races, global state |
| `stop-this-only` | Only the hit task pauses, others continue | Debugging independent task logic |

**Step Semantics of spawn Block**:

```
spawn {          // Step Over → runs the entire spawn block
    task_a()     // Step Into → enters task_a
    task_b()     // Runs in parallel, not affected by individual step
}
```

### 7. DAP Protocol Mapping

#### Phase One: Core Requests

| DAP Request | YaoXiang Semantics |
|-------------|--------------------|
| `initialize` | Capability negotiation: supports breakpoints, stepping, variables, stack frames |
| `launch` / `attach` | Launch / attach to YaoXiang program (`--debug` uses attach mode) |
| `setBreakpoints` | Set source line breakpoints |
| `configurationDone` | Breakpoints ready, begin execution |
| `threads` | Return list of all active spawn tasks |
| `stackTrace` | Return stack frame list of specified task |
| `scopes` | Return variable scopes of the current frame |
| `variables` | Return variable list of the specified scope |
| `continue` | Resume execution |
| `next` | Step Over |
| `stepIn` | Step Into |
| `stepOut` | Step Out |
| `pause` | Pause all tasks |
| `evaluate` | Evaluate expression in the current frame |
| `disconnect` | End debug session |

#### Phase Two: Enhanced Requests

| DAP Request | YaoXiang Semantics |
|-------------|--------------------|
| `setFunctionBreakpoints` | Function name breakpoints |
| `setExceptionBreakpoints` | Pause on error / panic |
| `dataBreakpointInfo` | Data breakpoint (triggered on variable modification) |

## Implementation Strategy

### Phase Zero: Infrastructure (Predecessor to All Phases)

**Goal**: The compilation frontend attaches debugging metadata to IR.

| Component | Change |
|-----------|--------|
| IR Definition | Add `SourceLocation`, `VarName`, `TypeAnnotation` and other metadata fields |
| Parser | Each AST node records source location |
| TypeChecker | Type information attached to IR nodes |
| Testing | Verify IR dump includes location and variable information |

**Does not involve runtime.**

### Phase One: Interpreter DAP MVP

**Goal**: `yaoxiang run --debug file.yx` can set breakpoints, step, and view variables.

| Component | Change |
|-----------|--------|
| DAP Server (new yx-core module) | stdio transport layer, core request handling, breakpoint manager (source line → IR node mapping) |
| Runtime debugging trait (yx-core) | `DebugEngine` trait definition (pause, resume, step, get_frames, eval, get_variables) |
| Interpreter | Breakpoint check in execution loop, pause / resume mechanism, frame list maintenance, `InterpreterDebugEngine` implementation |
| CLI | `yaoxiang run --debug` argument |

**Acceptance Criteria**: For any `.yx` file under `tests/yaoxiang/`, can use VS Code to set breakpoints, Step Over, and view variable values.

### Phase Two: Advanced Debugging Capabilities

**Goal**: Expression evaluation, function breakpoints, concurrent debugging, exception breakpoints.

| Component | Change |
|-----------|--------|
| Expression Evaluation Engine | Micro-program compilation (reuse parser + typechecker), temporary frame push into VM, side effect isolation |
| Concurrent Debugging | spawn task list mapping, breakpoint bound to task ID, stop-all / stop-this-only pause strategy |
| Function / Exception Breakpoints | `setFunctionBreakpoints`, `setExceptionBreakpoints` mapping |
| VS Code Extension | Provide default `launch.json` template |

### Phase Three: JIT Debugging & LLVM DWARF

**Goal**: JIT engine reuses DAP; LLVM produces DWARF for crash backtracing.

| Component | Change |
|-----------|--------|
| JIT | Implement `DebugEngine` trait, generate variable → register mapping table at compile time, runtime frame list, temporary expression compilation |
| LLVM | IR debugging metadata → LLVM `DILocation` / `DISubprogram` → DWARF (no DAP interaction) |

### Dependencies

```
Phase Zero (IR metadata)
    ↓
Phase One (Interpreter DAP MVP)  ← Usable from here
    ↓
Phase Two (Advanced capabilities)
    ↓
Phase Three (JIT + LLVM DWARF)
```

### Risks

| Risk | Mitigation |
|------|-----------|
| Interpreter pause mechanism complexity | Use simple channel / signal instead of complex state machine; pausing means preventing fetch of the next instruction |
| Type safety of expression evaluation | Reuse existing typechecker, read-only references, no side effects committed |
| DAP protocol details | Reference debugpy / delve implementations, the protocol is mature |
| Deadlock in concurrent debugging stop-all | Timeout mechanism + forced pause |

## Trade-offs

### Advantages

- **One source**: Debugging metadata is generated only once, shared by three engines. There will be no "interpreter debugging info is correct but LLVM's is wrong" situation.
- **Zero intrusion**: `--debug` is a single argument; behavior without it is completely unchanged.
- **DAP standard**: Directly integrate with the VS Code ecosystem, no need for custom editor protocol or debugger UI.
- **Interpreter first**: Debugging is naturally suited to the interpreter—flexible, controllable, simple expression evaluation. LLVM mode not doing interactive debugging is the most pragmatic choice.

### Disadvantages

- **Debug mode performance is poor**: The interpreter is much slower than JIT / LLVM. But debugging doesn't need performance—no one expects debug mode to run production loads.
- **LLVM debugging is limited**: AOT compilation cannot do interactive debugging, can only use GDB / LLDB + DWARF. But this is a trade-off: in LLVM mode, there shouldn't be debugging behavior differences to begin with.
- **Concurrent pause is complex**: Implementing stop-all semantics on the interpreter requires traversing all active tasks.

### Alternatives

| Option | Why not chosen |
|--------|----------------|
| Each of the three engines implements DAP independently | Triple the work, three sets of bugs. Violates "good taste" |
| Use only DWARF, no in-house DAP | The interpreter and JIT have no DWARF concept, LLDB cannot enter the inside of the VM |
| Build a command-line debugger modeled on Python pdb | VS Code experience utterly trumps command-line debuggers |
| Embed DAP inside the LSP process | Lifecycle is completely different—LSP follows the project, DAP follows the debug session. Process isolation is a hard requirement |

## Open Questions

- [ ] Should conditional breakpoint expression syntax be exactly the same as normal YaoXiang? (Suggestion: exactly the same, reuse parser)
- [ ] Step Into behavior inside a `spawn` block: when the user presses Step Into to enter a spawn block, which of the multiple parallel tasks should be displayed? (Suggestion: pause on the first created task)
- [ ] VS Code extension: should the debug configuration be placed under the existing `vscode-extension/` directory or in a separate repository?

## References

- [RFC-024: spawn-block-based Concurrency Model](../accepted/024-concurrency-model.md)
- [RFC-027: Compile-time Predicates and Unified Static Verification](../accepted/027-compile-time-evaluation-types.md)
- [RFC-028: JIT Compiler — Multi-level Execution Engine in VM](../draft/028-jit-compiler.md)
- [RFC-030: assert Mechanism](../review/030-assert-mechanism.md)
- [DAP Protocol Specification](https://microsoft.github.io/debug-adapter-protocol/)
- [debugpy — Python DAP Implementation Reference](https://github.com/microsoft/debugpy)
- [Delve — Go Debugger Reference](https://github.com/go-delve/delve)