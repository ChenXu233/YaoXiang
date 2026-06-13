# RFC-018 LLVM AOT Compiler and L3 Transparent Concurrency (DAG Lazy Scheduling) Implementation Plan

> **⚠️ Alignment Note**: This document is based on the old concurrency model (`@block`/`@eager`/`@auto` annotations, `Send`/`Sync` traits, L1/L2/L3 layers), and has been superseded by [RFC-024 New Concurrency Model](/design/rfc/accepted/024-concurrency-model.md). This document needs to be aligned with RFC-024 before it can proceed. The current concurrency model uses `spawn {}` blocks as the sole parallel primitive, with no annotations and no Send/Sync.

> **Task**: Implement the LLVM AOT backend + runtime DAG scheduler (~~landing `@auto/@eager/@block` three scheduling strategies~~ deprecated, needs alignment with RFC-024)  
> **Based on RFC**: RFC-018 (draft)  
> **Dependent RFCs**: RFC-001 (spawn model and error handling), RFC-008 (three-layer runtime), RFC-009 (ownership/Arc)  
> **Date**: 2026-03-10  
> **Status**: In progress  
> **Target Milestones**:  
> - M1: LLVM AOT (compilable and executable, serial)  
> - M2: DAG metadata + single-threaded scheduling (Standard Runtime, num_workers=1)  
> - M3: Multi-threaded parallel scheduling + granularity control (Full Runtime, num_workers>1)  
> - M4: Lazy scheduling (Lazy Task Creation) + **Resource type side-effect abstraction** + **error propagation/error graph** + annotation integration

---

## Abstract (Implementation Closure)

- Add an LLVM backend in `yaoxiang` (feature-gated), compile `BytecodeModule` into machine code (COFF/ELF/Mach-O) and load and execute it at runtime.
- Introduce a **stable ABI**: AOT-generated code and the runtime interact through `extern "C"` `RtValue/RtContext` to avoid Rust enum ABI instability issues.
- Land the core of RFC-018: **function-block-internal DAG** + **lazy scheduling**. Concurrency/serialization is jointly determined by **DAG edges (Data/Control/Spawn)** and **Resource type rules**; errors propagate along dependency edges per RFC-001 and can form an error graph.

---

## Public Interface/Behavior Changes (Externally Visible)

1. **Cargo features**
   - Add the `llvm-aot` feature: enables LLVM/inkwell dependencies and the AOT backend; disabled by default (to ensure builds without an LLVM environment).
2. **CLI**
   - `yaoxiang run` adds `--backend {interpreter,llvm}` (default is interpreter).
   - Optional: add `--runtime {embedded,standard,full}` and `--workers <N>` to control the runtime layer and concurrency degree (RFC-008):
     - `--runtime embedded`: immediate execution (no DAG, no scheduler features, suitable for embedded/minimal scenarios)
     - `--runtime standard`: DAG + Scheduler (num_workers=1 is async; >1 is parallel)
     - `--runtime full`: standard + WorkStealer (advanced features, optional)
3. **Runtime ABI (internal but cross-module)**
   - Add `RtValue` (`#[repr(C)]`) and `RtContext` (containing only pointers/primitive types) as the AOT/runtime interaction boundary.

---

## Key Design Constraints (Aligned with RFC-001 / RFC-008 / RFC-018)

### A. Concurrency Semantics (L1/L2/L3 Are Only Mental Models)

- **L3 (default / @auto)**: Transparent concurrency; build DAG; when encountering a call, first return a "deferred evaluable" value, **triggering evaluation only when the value is needed**.
- **L1 (@block)**: Provided by the standard library (RFC-008); semantically means "force eager evaluation" and does not enter the DAG lazy queue; primarily used for debugging and critical sequential sections.
- **L2 (spawn)**: **Can only be used within @block scope** (RFC-001/008), used to insert concurrency in synchronous code; belongs to Full Runtime capability.

### B. Three Runtime Layers (RFC-008)

- **Embedded Runtime**: Immediate execution; can choose not to build the DAG at all (saves memory/startup); used in constrained environments.
- **Standard Runtime**: DAG + Scheduler as the core (lazy evaluation naturally supports async/parallel).
- **Full Runtime**: Adds WorkStealer on top of standard, plus standard-library-level `@block` / `spawn` capabilities.

### C. DAG Build Scope and Memory (RFC-001/018)

- The DAG is only built within a **single function body/block**; it does not recursively expand the body of called functions (to avoid error graph and DAG node explosion).
- DAG metadata must carry **stable IDs for nodes/edges** and **Span** (for error propagation and error graph localization).

### D. Side-Effect Abstraction (RFC-001: Resource Types)

- Do not introduce an additional "explicit side-effect annotation system"; side effects are uniformly **resource operations**:
  - Function calls whose parameter types include `Resource` (or its derived resource types) are treated as resource operations;
  - Operations on the same resource automatically form a **ControlEdge** (serial); operations on different resources can run in parallel;
  - When it cannot be statically determined whether it is the same resource, default to conservative serial (an explicit unsafe parallel hint may be introduced later as an extension).

### E. Error Propagation (RFC-001)

- Errors propagate upward along the DAG dependency edges (independent of actual parallel execution order), and the propagation path is recorded for the error graph.

---

## Stage 0: Prerequisites and Constraint Lock-In (1-2 days)

### 0.1 Lock LLVM/inkwell Version and Build Method

**Goal**
- Select LLVM major version = **17** (unify team environment; Windows/Linux/macOS can all obtain corresponding distribution packages).
- Add `inkwell` to `Cargo.toml` (enable the `llvm17-0` corresponding feature), and place it under the `llvm-aot` feature.

**Acceptance Criteria**
- [ ] `cargo build` (without feature) passes (builds without an LLVM environment).
- [ ] `cargo build -F llvm-aot` passes when the LLVM17 environment is configured.

**Test Items**
- [ ] CI/local: Two build matrices (`default` and `-F llvm-aot`) pass on at least one platform.
- [ ] Minimal smoke: `cargo test -F llvm-aot` can start and execute an empty test module (verifying only the link).

---

### 0.2 LLVM Environment Detection and Error Messages

**Goal**
- Add build-time/runtime detection notes: when `llvm-config`/LLVM directory is missing, give actionable error hints (how to install/how to set prefix variables).

**Acceptance Criteria**
- [ ] When LLVM is missing, the error message includes: expected version (17), available environment variables (such as `LLVM_SYS_170_PREFIX` or `LLVM_CONFIG_PATH`), and example paths.

**Test Items**
- [ ] Execute `cargo build -F llvm-aot` on a machine without an LLVM environment; output hints are complete and do not panic (compile-time error is sufficient).

---

### 0.3 Lock Spawn Model Implementation Constraints (Aligned with RFC-001/008)

**Goal**
- Clarify and solidify the following implementation constraints (written into code comments/development docs and test cases):
  - `spawn` is only allowed to appear within `@block` scope (parse/type-check/IR stages must all guard against this).
  - `@block` semantics mean "eager evaluation", provided by standard library capabilities (can be first implemented as a compiler built-in for MVP, but must retain the interface for future delegation to the standard library).
  - DAG is only built within function blocks; must carry stable `node_id` and `span` (supporting error propagation/error graph).
  - Resource types drive ControlEdge generation, avoiding the introduction of an additional user-visible effect annotation system.
  - **Parallel safety constraint (RFC-001/009)**: Parallel cross-thread execution is allowed only when captured/return values of nodes satisfy `Send + Sync` (or language-side equivalent constraint); otherwise must degrade to serial (or be fixed to execute on a single worker).

**Acceptance Criteria**
- [ ] Compiler gives clear errors (with Span) in illegal `spawn` scenarios.
- [ ] Semantic differences of `@block/@eager/@auto` are observable and testable in minimal examples.
- [ ] Documentation (this plan) is consistent with key decisions of RFC-001/008/018, with no self-contradictory items.

**Test Items**
- [ ] `spawn` location restriction test: spawn outside @block must report an error.
- [ ] DAG scope test: confirm DAG does not expand across function bodies (node count independent of call depth).
- [ ] Send/Sync constraint test:
  - `spawn` capturing non-`Send` values must report an error (with Span).
  - Nodes under `@auto` containing non-`Send + Sync` values must not be scheduled across threads (verifiable via `std.concurrent.thread_id` statistics).

---

## Stage 1: LLVM Backend Skeleton and Selection Switch (1-2 days)

### 1.1 Add Backend Module and Align with RFC-018 Directory Structure

**Goal**
- Add `src/backends/llvm/`, containing: `mod.rs / context.rs / types.rs / values.rs / codegen.rs / dag.rs / scheduler.rs / tests.rs` (subsequent merging/splitting allowed).
- Expose the module in `src/backends/mod.rs` via `#[cfg(feature = "llvm-aot")] pub mod llvm;`.

**Acceptance Criteria**
- [ ] `cargo test` (default feature) passes.
- [ ] `cargo test -F llvm-aot` passes (even if LLVM backend is not yet fully implemented).

**Test Items**
- [ ] Unit: At least 1 compile-time test in `src/backends/llvm/tests.rs` can run (verifying only that the module is referenceable).

---

### 1.2 Backend Selection: CLI/Library-Side Injection Point

**Goal**
- Add the `--backend` parameter (ValueEnum) to the CLI `Run` subcommand: `interpreter` (default) / `llvm` (requires feature).
- Add backend selection branches in the `yaoxiang::run_*` path, abstracted as `fn make_executor(kind, config) -> Box<dyn Executor>` (or enum dispatch; trait object is not required).

**Acceptance Criteria**
- [ ] `yaoxiang run file.yx` still goes through the interpreter, behavior unchanged.
- [ ] `yaoxiang run --backend llvm file.yx`: if feature is not enabled, gives a clear error; if feature is enabled, enters the LLVM execution path (even if temporarily returning "not implemented" must be a controllable error).

**Test Items**
- [ ] CLI parameter parsing test (added in `tests/integration`).
- [ ] Negative test: passing `--backend llvm` without feature returns a readable error message.

---

## Stage 2: Stable ABI (RtValue/RtContext) and Runtime API (3-5 days)

> This stage is key to "LLVM-generated code being executable": the cross-boundary value representation must be stabilized first.

### 2.1 Define `RtValue` (Stable C ABI)

**Goal**
- Define `#[repr(C)] struct RtValue { tag: u8, _pad: [u8; 7], payload: u64 }` (or 16-byte structure, keeping alignment simple).
- Define `#[repr(u8)] enum RtTag { Unit, Bool, Int, Float, Handle, Async, Error }` (minimum set; to be extended).
- Conventions:
  - Int: `payload` = `i64` bits
  - Float: `payload` = `f64` bits
  - Bool: 0/1
  - Handle: `payload` = `usize` (extended to u64)
  - Async: `payload` = `TaskId` (u64)
  - Error: `payload` = error code or pointer (MVP can use error code first)

**Acceptance Criteria**
- [ ] `RtValue` can be safely constructed/read inside Rust (no UB), and has `Debug` output and basic assertion utility functions.
- [ ] Aligned with LLVM IR: able to create struct types of the same layout in inkwell (field order/size consistent).

**Test Items**
- [ ] `RtValue` roundtrip: unit tests for int/float/bool/unit encode/decode.
- [ ] ABI size test: `size_of::<RtValue>()` and `align_of::<RtValue>()` are fixed (write hard assertions to prevent accidental future changes).

---

### 2.2 Define `RtContext` (Runtime Context)

**Goal**
- Define `#[repr(C)] struct RtContext`, containing only pointers/integers:
  - `heap: *mut Heap`
  - `ffi: *const FfiRegistry`
  - `scheduler: *mut DynScheduler` (or pointing to a specific implementation)
  - `max_parallelism: usize`
  - `error_graph: *mut ErrorGraph` (optional: for RFC-001 error propagation recording; MVP can be null)
  - Reserved fields (version number/flags), but kept minimal (KISS).

**Acceptance Criteria**
- [ ] `RtContext` can be constructed by the interpreter/LLVM executor and passed to generated code.
- [ ] Does not contain Rust non-stable-layout fields (prohibits directly inlining `Heap`/`FfiRegistry` values).

**Test Items**
- [ ] Memory safety test for constructing/destroying `RtContext` (no real LLVM required).

---

### 2.3 Runtime C API: Minimum Function Set Called by Generated Code

**Goal**
- Provide `#[no_mangle] extern "C"` functions (unified prefix `yx_rt_`), MVP minimum includes:
  - `yx_rt_force(ctx: *mut RtContext, v: RtValue) -> RtValue`
  - `yx_rt_lazy_call(ctx, callee: *const u8 /*function pointer*/, args: *const RtValue, argc: usize, node_id: u32, span_id: u32) -> RtValue`
  - `yx_rt_native_call(ctx, name_ptr: *const u8, name_len: usize, args: *const RtValue, argc: usize, node_id: u32, span_id: u32) -> RtValue`
  - `yx_rt_panic(code: u32)` or `yx_rt_trap(msg_ptr, len)` (for debugging)
- Constraint: AOT-generated code **interacts only through the above API**, not directly with Rust structs.

**Acceptance Criteria**
- [ ] The runtime API can compile even without LLVM (controlled by `llvm-aot` feature: API can be persistent or only provided under feature, but must be testable).
- [ ] `yx_rt_native_call` can invoke the `FfiRegistry` handler (MVP only supports Int/Float/Bool/Unit parameters and return values; returns Error RtValue if not supported), and records `node_id/span_id` to the error graph on failure (if enabled).

**Test Items**
- [ ] Pure Rust unit test: directly call `yx_rt_native_call` to verify that the `std.io.println` (or self-registered function) path is available.
- [ ] Error path test: passing a non-existent native name returns `Error` RtValue and can be converted to `ExecutorError::FunctionNotFound`.

---

## Stage 3: LLVM Codegen Infrastructure (2-3 days)

### 3.1 LLVM Context/Module/TargetMachine Initialization

**Goal**
- `context.rs`: encapsulate inkwell `Context/Module/Builder` lifecycle.
- Initialize Target: set target triple + data layout based on `PlatformDetector` (supports `LLVM_TARGET`) and host triple.
- Support output:
  - LLVM IR (`.ll`) for debugging
  - Object (`.o/.obj`) for AOT

**Acceptance Criteria**
- [ ] For any empty `BytecodeModule`, can generate an LLVM Module containing `main` (even if the function body only returns Unit).
- [ ] IR can be verified (call LLVM verify; return readable error on failure).

**Test Items**
- [ ] Unit: generate minimal module and pass verify.
- [ ] Snapshot test (optional): do string containment assertions on key `.ll` fragments (avoid brittle full snapshots).

---

### 3.2 `TypeMap`: YaoXiang Type → LLVM Type (MVP)

**Goal**
- `types.rs`: implement `fn llvm_type(yao_type: &Type) -> BasicTypeEnum`, first covering:
  - `Int` → i64
  - `Float` → f64
  - `Bool` → i1
  - `()`/Void → void (or uniformly return `RtValue(Unit)`)
- Strategy choice (to reduce ABI surface): **all functions uniformly return `RtValue`** (instead of returning by type), allowing codegen, scheduler, and FFI to handle uniformly; type information is used for static checking and generating `RtValue` construction/destruction logic.

**Acceptance Criteria**
- [ ] `TypeMap` has stable mapping for the above types, and the function signature in LLVM IR is consistent: `fn(*mut RtContext, *const RtValue, usize) -> RtValue`.

**Test Items**
- [ ] `TypeMap` unit test: given `Type::Int/Float/Bool/Void`, LLVM types are generated successfully.
- [ ] Generated function signatures can be retrieved in the LLVM module and asserted to match parameter/return types.

---

## Stage 4: Instruction Translation MVP (5-8 days)

### 4.1 Register to LLVM Value Mapping (Minimal SSA Implementation)

**Goal**
- `values.rs`: implement a virtual register `Reg(u16)` → LLVM `Value` mapping table (managed by basic block scope).
- Convention: all register values are represented as `RtValue` (to avoid type explosion/ABI inconsistency); before operations/comparisons, force/unpack via helper.

**Acceptance Criteria**
- [ ] Generated code can correctly merge register values after control flow branching (using phi or uniformly handling at the `RtValue` layer).

**Test Items**
- [ ] Unit: generate IR for a BytecodeFunction containing if/else, and pass verify.
- [ ] Regression: multiple assignments to the same register will not cause use-before-def (insert trap/error in debug mode).

---

### 4.2 Translate Core Instruction Subset (Covering "Runnable")

**Goal**
- `codegen.rs`: implement at least the following `BytecodeInstr`:
  - `LoadConst` (Int/Float/Bool/String first limited: String can degrade to Error or be unsupported first)
  - `Mov`
  - `BinaryOp` (Add/Sub/Mul/Div: separate paths for Int and Float)
  - `Compare` (Eq/Lt/Gt etc.)
  - `Jmp/JmpIf/JmpIfNot/Return/ReturnValue`
  - `CallNative` (via `yx_rt_native_call`)
  - `CallStatic` (two strategies: `@block` direct call; `@auto` goes through `yx_rt_lazy_call` returning Async)
- Mandatory rule: any operand participating in arithmetic/comparison/branching must first be `yx_rt_force` ("triggered when value is needed" for transparent concurrency).

**Acceptance Criteria**
- [ ] AOT backend can execute simple programs:
  - Pure arithmetic
  - if/else
  - Call `std.io.println` for output
- [ ] Unsupported instructions produce readable errors (not panics).

**Test Items**
- [ ] Integration: add `tests/integration/llvm_aot_smoke.rs` (feature gated), run 5 program fragments and assert results/output (output achievable via redirecting stdout).
- [ ] Negative: encountering `MakeClosure/CallVirt/...` returns a clear "not implemented" error.

---

## Stage 5: Machine Code Artifact and Execution (AOT Closure) (3-6 days)

### 5.1 Artifact Format: Object + Metadata (Two Files, Simple First)

**Goal**
- `CompiledArtifact` (Rust-side structure) contains at least:
  - `object_bytes: Vec<u8>` (COFF/ELF/Mach-O)
  - `dag_meta: DAGMetadata` (can be empty first)
  - `entries: Vec<EntryPoint>` (at least main)
  - `type_info: TypeInfo` (empty for MVP)
- Output strategy:
  - `yaoxiang build-aot input.yx -o out/` generates `program.obj` + `program.dag.ron` (or `.json`)
  - `yaoxiang run --backend llvm` defaults to "in-memory compile + direct execute" (no disk writes), convenient for development.

**Acceptance Criteria**
- [ ] build-aot can generate two files, and metadata can be deserialized.
- [ ] The run/llvm path does not depend on disk files to execute.

**Test Items**
- [ ] File generation test: verify `.obj` is non-empty, `.dag.ron` is parseable and version number matches.
- [ ] Compatibility test: different build_modes (Debug/Release) output different optimization levels (at least distinguishable).

---

### 5.2 Execution Method: First "In-Memory Execution", Then "Disk Load" (Two-Step Acceptance)

**Goal**
- Step A (deliver first): use LLVM ExecutionEngine (or ORC JIT) to execute the generated module (for verifying semantic closure, highest development efficiency).
- Step B (truly AOT): use TargetMachine to generate object bytes, and execute via "dynamic library link/load" path:
  - Link the object into `.dll/.so/.dylib` (call system linker or lld; as additional requirement of `llvm-aot` feature)
  - Use `libloading` to load symbols and call entry functions

**Acceptance Criteria**
- [ ] Step A: `--backend llvm` can execute within the same process (no external linker dependency).
- [ ] Step B: artifacts generated by `build-aot` can be loaded and executed by `run-aot` (new subcommand or internal path).

**Test Items**
- [ ] Step A: unit/integration tests run by default (fast development).
- [ ] Step B: marked as "requires external linker" optional integration tests (enabled in CI when environment available; manually runnable locally).

---

## Stage 6: DAG Metadata Generation (4-7 days)

### 6.1 Define `DAGMetadata` (Versioned)

**Goal**
- `dag.rs` defines:
  - `dag_version: u32`
  - `nodes: Vec<DAGNode>` (carrying `node_id` and `span_id`, for error propagation)
  - `edges: Vec<DAGEdge>` (with edge type: Data/Control/Spawn)
- `DAGEdge` contains at least:
  - `from: u32`
  - `to: u32`
  - `kind: EdgeKind { Data, Control, Spawn }`
- Conflict/scheduling rules (RFC-001):
  - DataEdge + DataEdge: can run in parallel (if no other dependencies)
  - Any combination containing ControlEdge: must be serialized (preserve order)
- `DAGNode` contains at least:
  - `node_id: u32` (unique within function)
  - `ip: u32` (call instruction position)
  - `strategy: ScheduleStrategy { Serial, Eager, Lazy }` (from annotation or default strategy)
  - `span_id: u32` (localization and error graph)
  - `thread_safety: ThreadSafety { SendSync, LocalOnly }` (inferred by type system; `LocalOnly` nodes prohibited from cross-thread scheduling)
- Convention: nodes only describe "schedulable call sites"; parameters are captured at runtime `yx_rt_lazy_call` time (to avoid static expression evaluation complexity).

**Acceptance Criteria**
- [ ] `DAGMetadata` can be serialized/deserialized (using existing `ron` or `serde_json`).
- [ ] Load error when `dag_version` does not match.

**Test Items**
- [ ] Serialization roundtrip unit test.
- [ ] Version mismatch unit test (manually construct old version).
- [ ] `thread_safety` inference test: covers at least 1 `LocalOnly` scenario, and verifies it does not execute across threads under `num_workers>1`.

---

### 6.2 Resource Type and ControlEdge Generation (Minimum Viable Side-Effect Abstraction)

**Goal**
> **Update**: Per RFC-001, side effects are not expressed through an additional effect system, but abstracted as **resource operations** through **Resource types**, generating ControlEdges.

- Resource operation identification (MVP):
  - If any parameter type of the call is `Resource` or its derived resource type (e.g., `Console/FilePath/HttpUrl/DBUrl`), then the call site is a resource operation;
  - Standard library resource operation functions must have identifiable type constraints (recommended: explicitly carry resource types in std export signatures; or mark "resource operation" in FFI registry export metadata and associate resource parameter positions).
- ControlEdge generation (MVP):
  - For multiple resource operations on the **same resource value/handle (same SSA value/same constant residency key)**, add ControlEdges in program order (automatically serial).
  - When it cannot be determined to be the same resource (aliases/complex origins), default to conservative serial (an explicit unsafe parallel hint may be introduced later as an extension).
  - **Resource identity propagates along data flow (RFC-001)**: resource conflict detection is based on "value equality/same origin", not "same resource type" (two different `FilePath` values can run in parallel; the same `FilePath` value must be serial).

**Acceptance Criteria**
- [ ] For example: `log → save → log` forms ControlEdges due to Console/FilePath resources, stable serial; different resource operations can run in parallel.
- [ ] Resource operation identification is stable (same input module yields consistent results multiple times).

**Test Items**
- [ ] Unit: resource type parameter identification test (ControlEdge must be generated when Resource parameter exists).
- [ ] Unit: two resource operations on the same resource value (same variable/same constant) must generate ControlEdge; different resource values (different variables/different constants) need not generate.
- [ ] Integration: run examples containing multiple `std.io.println/std.io.write_file`, assert output/write order is consistent with the interpreter.

---

### 6.3 L1 Automatic Fallback (Small Functions Degrade to @block, Avoid Scheduling Overhead)

> **Source**: RFC-001 5.2 (L1 Automatic Fallback).  
> **Purpose**: Without changing semantics, reduce DAG/scheduler overhead for small functions (especially the unified behavior of the interpreter backend and AOT backend).

**Goal**
- Perform lightweight threshold judgment on functions at compile time; if any condition is met, default strategy of the function (or certain blocks within the function) degrades to `Serial`:
  - Instruction count `< 50`
  - DAG node count `< 10`
- Expose switches via CLI/configuration (MVP: internal configuration is also acceptable):
  - `--l1-threshold=<N>` to adjust threshold
  - `--no-l1-fallback` to disable automatic fallback

**Acceptance Criteria**
- [ ] Small functions under `@auto` actually do not enter the DAG/scheduler queue (verifiable via statistics fields or trace), results consistent with no fallback.
- [ ] Large functions are unaffected; explicit annotations `@eager/@block` have priority over automatic fallback.

**Test Items**
- [ ] Unit: construct boundary values (49/50 instructions, 9/10 nodes) to verify whether fallback is triggered.
- [ ] Regression: enable/disable fallback on the same source code, output and return values are consistent.

---

## Stage 7: Runtime DAG Scheduler (Lazy Scheduling Core) (6-10 days)

### 7.1 Implement Task Model (Interfacing with `RtValue::Async`)

**Goal**
- `scheduler.rs` (or migrate to `src/backends/runtime/` to achieve "interpreter/LLVM shared") implements:
  - `TaskId` allocation
  - `TaskState { Pending, Running, Completed(RtValue), Failed(RtValue) }`
  - `spawn(task_fn_ptr, args, deps, node_id, span_id)`: create task but can delay startup (for error propagation/error graph)
  - `force(task_id)`: trigger execution per dependency topology and wait for result

**Acceptance Criteria**
- [ ] `yx_rt_lazy_call` returns `Async(TaskId)` and the task is recorded (not immediately executed).
- [ ] `yx_rt_force` can trigger task execution and return result (including dependency chain).

**Test Items**
- [ ] Pure Rust: use mock "compiled fn" pointers (actually Rust `extern "C"` functions) to construct a 3-node DAG, verify dependency order and result correctness.
- [ ] Error propagation: downstream force gets Error, and does not deadlock.

---

### 7.2 Scheduling Strategy Implementation (Serial/Eager/Lazy)

**Goal**
- `Serial` (corresponding to `@block`): does not create Async; call executes immediately; scheduler interface can be bypassed.
- `Eager`: create task but immediately `force` (guarantee order), used for debugging/semantic alignment.
- `Lazy` (default `@auto`): `force` only when value is needed; scheduler can pre-start "ready" tasks in background window (subject to concurrency limit).
- Bottom-up (RFC-001/008): runtime behavior should reflect the characteristic of "triggering evaluation in reverse from where the result is needed"; **branches/island DAGs that are not consumed and do not involve resource operations (no ControlEdge) should not execute**, to reduce overhead; resource operations must be ordered and completed per ControlEdge (consistent with interpreter).
- Background DAG (RFC-018): when multiple long-running/infinite loop tasks exist in the same scope, scheduler needs to provide **cooperative slicing** (e.g., based on budget or explicit `yield_now`) to avoid main DAG starvation and "stuck in loop".

**Acceptance Criteria**
- [ ] Same program yields consistent results under Serial/Eager/Lazy three strategies.
- [ ] Under Lazy, when a call result is never forced/used, the task will not execute (Lazy Task Creation).

**Test Items**
- [ ] Comparison test: three strategies output is consistent.
- [ ] Lazy skip test: write a "compute but do not use" branch/variable, assert corresponding task execution count is 0 (scheduler statistics field).
- [ ] Background slicing test: construct 2 long-running tasks + 1 main task, assert all three have progress within the time window (verifiable via counters or `thread_id` + log statistics).

---

### 7.3 Concurrency Control and Granularity Control

**Goal**
- Concurrency limit: `max_parallelism = num_workers * 2` (RFC-018 suggestion).
- Resource constraint: operations on the same resource must execute serially per ControlEdge (RFC-001 resource type rules); scheduler must not disrupt ControlEdge order.
- Thread safety constraint (RFC-001/009): scheduler must respect `DAGNode.thread_safety`:
  - `SendSync`: can execute across workers (affected by concurrency limit and dependency constraints)
  - `LocalOnly`: prohibited from cross-thread scheduling/prohibited from being stolen by work-stealing; if necessary, degrade to serial (or fix execution on the worker that created it)
- Adaptive granularity (MVP): when the number of pending tasks far exceeds the concurrency limit, merge "multiple ready tasks with **no ControlEdge constraints**" into a batch submission (implemented as the same worker executing a batch in sequence, reducing scheduling overhead).

**Acceptance Criteria**
- [ ] A large number of independent, resource-unconstrained tasks (1e4) will not cause memory explosion (task structure O(concurrency) or controllable upper bound).
- [ ] `LocalOnly` nodes will not execute across threads under `num_workers>1` (verifiable via `std.concurrent.thread_id`).
- [ ] Resource operations (e.g., `std.io.*`) output/side-effect order strictly maintains interpreter order.

**Test Items**
- [ ] Stress test unit: construct 10000 mock tasks, peak memory/task count is controlled (using statistical assertions, precise memory measurement is not required).
- [ ] LocalOnly integration test: construct an example containing `LocalOnly` nodes, assert under `num_workers>1` that its execution thread ID does not change.
- [ ] Resource order integration test: multiple resource operations (println/write_file) must output/write to disk in source code order.

---

### 7.4 Error Propagation and Error Graph Recording (RFC-001 Minimum Closure)

**Goal**
- Define the minimum `ErrorGraph` data structure (can be used for debugging/trace first):
  - Nodes: `node_id + span_id + message/error_code`
  - Edges: `from_node_id -> to_node_id` (representing "error propagating from dependent node to consumer node")
- Recording strategy (RFC-001 resolution):
  - Errors propagate upward along dependency edges, **independent of actual execution order**;
  - DAG is only built within functions, so the error graph is also limited to function level, avoiding memory explosion.
- Aligned with ABI:
  - `yx_rt_lazy_call/yx_rt_native_call` must carry `node_id/span_id` (locked at stage 2.3)
  - Write to `ErrorGraph` on task failure and `force` returning error (if `ctx.error_graph != null`)

**Acceptance Criteria**
- [ ] When the bottom node of the dependency chain fails, the top-level consumer receives the error (and can locate the span of the failed node).
- [ ] Under parallel execution, error propagation path is stable and reproducible (independent of scheduling order).

**Test Items**
- [ ] Unit: construct 3-node dependency chain, simulate middle node failure, assert ErrorGraph edges are `leaf->mid->root`.
- [ ] Concurrency regression: under num_workers>1, run multiple times, ErrorGraph structure is consistent.

---

## Stage 8: Syntax Annotation Integration (@block/@eager/@auto) (5-8 days)

### 8.1 Frontend Supports Annotations and Propagates Down to Bytecode/Metadata

**Goal**
- Parsing layer: recognize function/block annotations `@block`, `@eager`; default `@auto`.
- Parsing/type checking: enforce that `spawn` can only appear within `@block` scope (RFC-001/008).
- IR/Bytecode: carry default strategy in `BytecodeFunction` or in an additional side-table; can decide to go lazy/eager/direct at call-site.

**Acceptance Criteria**
- [ ] No annotation: default is Lazy (@auto).
- [ ] `@block`: no Async is created within this scope, behavior is consistent with interpreter serial.
- [ ] `@eager`: create task then immediately force (result consistent and easy to debug).

**Test Items**
- [ ] Frontend: parsing/IR generation tests containing annotations (AST/IR assertions).
- [ ] Backend: same source code with different annotations, running behavior matches strategy.

---

### 8.2 Standard Library: Runtime Interface for `@block` and `spawn` (Full Runtime)

> **Source**: RFC-008 (@block provided by standard library), RFC-001 (waiting semantics of spawn controlled by standard library).

**Goal**
- Add a standard library runtime module (suggested path: `std.runtime` or `std.full`), providing:
  - `block`: force eager evaluation (equivalent to setting scope strategy to `Serial`/not entering DAG queue)
  - `spawn`/`join_all` (or implicit join): create concurrent tasks within `@block` scope and wait for completion
- Compiler can implement MVP as built-in first, but must abstract an interface that can be delegated to the standard library (to avoid future refactoring costs).

**Acceptance Criteria**
- [ ] `spawn { ... }` blocks within an `@block` function can execute concurrently and complete before the scope ends (no "silently background-leaking tasks").
- [ ] The behavior of `@block` can be clearly distinguished from L3 default concurrent behavior (e.g., whether it enters the DAG queue, whether it generates Async values).

**Test Items**
- [ ] Integration: example with two `spawn { std.concurrent.sleep(50) }` blocks, time consumed under multiple workers is close to a single sleep (coarse-grained concurrency verification).
- [ ] Negative: using spawn outside @block reports an error (consistent with 0.3/8.1).

## Stage 9: End-to-End and Performance Benchmarking (Ongoing)

### 9.1 Consistency Test with Interpreter (Semantic Alignment)

**Goal**
- Select a set of "instruction-subset-covering" cases: arithmetic, branching, function calls, native IO.
- For the same source code, execute with interpreter and llvm backend respectively, compare:
  - Return value (if any)
  - stdout output (requires injection/redirection)
  - Error type (try to align with `ExecutorError`)

**Acceptance Criteria**
- [ ] In the case set, LLVM backend results are consistent with the interpreter.

**Test Items**
- [ ] `tests/integration/llvm_vs_interpreter.rs` (feature gated) at least 10 cases.
- [ ] Regression: new cases must run on both backends.

---

### 9.2 Benchmark: Interpreter vs AOT (Coarse-Grained)

**Goal**
- Add benchmarks in `benches/`: pure computation (no IO), large number of call tasks (concurrency benefit), mixed IO (ordering constraints).

**Acceptance Criteria**
- [ ] AOT is significantly faster than the interpreter on pure computation cases (specific multiples not promised, but must not be obviously slower).
- [ ] Lazy scheduling overhead is observable and localizable (output scheduler stats).

**Test Items**
- [ ] criterion bench (manual/CI optional) generates reports, records baseline.

---

## Assumptions and Defaults (Choices When Not Covered by Business Requirements)

- Default LLVM major version is **17**; if the team's toolchain differs, uniformly modify the `inkwell` feature and documentation.
- AOT execution path takes "two-step approach": first in-memory execution (development verification), then disk link/load (truly AOT).
- The initial `llvm-aot` only commits to covering a set of MVP instruction subsets; advanced features like closures/dynamic dispatch/exceptions are extended as needed later (return clear "not implemented" error when encountered).
- DAG dependency edges **can be dynamically inferred at runtime from Async TaskIds of args**; compile-time edges field is first an optional optimization and debug validation, not blocking M2 delivery.
  - **Supplement (RFC-001)**: The main source of ControlEdges is resource type rules; if resource information is missing, default to conservative serial to ensure correctness.