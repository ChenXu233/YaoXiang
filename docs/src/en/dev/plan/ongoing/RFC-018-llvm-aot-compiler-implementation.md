# RFC-018 LLVM AOT Compiler and L3 Transparent Concurrency (DAG Delay Scheduling) Implementation Plan

> **Task**: Implement LLVM AOT backend + runtime DAG scheduler, implementing `@auto/@eager/@block` three scheduling strategies (L3/L2/L1)  
> **Based on RFC**: RFC-018 (draft)  
> **Dependent RFCs**: RFC-001 (Spawn Model and Error Handling), RFC-008 (Three-Layer Runtime), RFC-009 (Ownership/Arc)  
> **Date**: 2026-03-10  
> **Status**: In Progress  
> **Target Milestones**:  
> - M1: LLVM AOT (compilable and executable, serial)  
> - M2: DAG metadata + single-threaded scheduling (Standard Runtime, num_workers=1)  
> - M3: Multi-threaded parallel scheduling + granularity control (Full Runtime, num_workers>1)  
> - M4: Lazy scheduling (Lazy Task Creation) + **Resource type (Resource) side effect abstraction** + **Error propagation/Error graph** + Annotation integration

---

## Abstract (Implementation Closure)

- Add LLVM backend (feature gate) in `yaoxiang`, compile `BytecodeModule` to machine code (COFF/ELF/Mach-O) and load/execute at runtime.
- Introduce **stable ABI**: AOT generated code interacts with runtime through `extern "C"` `RtValue/RtContext`, avoiding Rust enum ABI instability issues.
- Land core of RFC-018: **function-block DAG** + **Lazy Scheduling**. Concurrency/serial execution determined jointly by **DAG edges (Data/Control/Spawn)** and **Resource type (Resource) rules**; errors propagate along dependency edges per RFC-001 and can form error graphs.

---

## Public Interface/Behavior Changes (Externally Visible)

1. **Cargo features**
   - New `llvm-aot` feature: enables LLVM/inkwell dependencies and AOT backend; disabled by default (ensures build without LLVM environment).
2. **CLI**
   - `yaoxiang run` adds `--backend {interpreter,llvm}` (default: interpreter).
   - Optional: add `--runtime {embedded,standard,full}` and `--workers <N>` to control runtime tier and concurrency (RFC-008):
     - `--runtime embedded`: immediate execution (no DAG, no scheduler features, suitable for embedded/minimal scenarios)
     - `--runtime standard`: DAG + Scheduler (num_workers=1 for async; >1 for parallel)
     - `--runtime full`: standard + WorkStealer (advanced features, optional)
3. **Runtime ABI (internal but cross-module)**
   - New `RtValue` (`#[repr(C)]`) and `RtContext` (only contains pointers/basic types) as boundary between AOT and runtime.

---

## Key Design Constraints (Aligned with RFC-001 / RFC-008 / RFC-018)

### A. Concurrency Semantics (L1/L2/L3 Are Just Mental Models)

- **L3 (default / @auto)**: Transparent concurrency; build DAG; when encountering a call, first return a "deferrable evaluation" value, **trigger evaluation only when the value is needed**.
- **L1 (@block)**: Provided by standard library (RFC-008), semantics are "force eager evaluation", does not enter DAG lazy queue; mainly for debugging and critical sequential segments.
- **L2 (spawn)**: **Can only be used within @block scope** (RFC-001/008), used to insert concurrency in synchronous code; belongs to Full Runtime capability.

### B. Three-Layer Runtime (RFC-008)

- **Embedded Runtime**: Immediate execution; optionally skip DAG construction entirely (save memory/startup); for constrained environments.
- **Standard Runtime**: DAG + Scheduler as core (lazy evaluation naturally supports async/parallel).
- **Full Runtime**: Adds WorkStealer on top of standard, plus standard library-level `@block` / `spawn` capabilities.

### C. DAG Construction Scope and Memory (RFC-001/018)

- DAG is constructed only **within a single function body/block**; callee function bodies are not recursively expanded (avoids error graph and DAG node explosion).
- DAG metadata must carry **stable node/edge IDs** and **Span** (used for error propagation and error graph localization).

### D. Side Effect Abstraction (RFC-001: Resource Types)

- No additional "explicit side effect annotation system" is introduced; side effects are unified as **resource operations**:
  - Function calls where parameter types include `Resource` (or derived resource types) are treated as resource operations;
  - Operations on the same resource automatically form **ControlEdge** (serial); operations on different resources can be parallel;
  - When it cannot be statically determined if resources are the same, conservative serialization is the default (can later introduce explicit unsafe parallel hints as an extension).

### E. Error Propagation (RFC-001)

- Errors propagate along DAG dependency edges (regardless of actual parallel execution order), and propagation paths are recorded for error graphs.

---

## Phase 0: Prerequisites and Constraint Lock-in (1-2 days)

### 0.1 Lock LLVM/inkwell Version and Build Method

**Goal**
- Select LLVM major version = **17** (unify team environment; available for Windows/Linux/macOS).
- Add `inkwell` in `Cargo.toml` (enabling `llvm17-0` corresponding feature), tied under `llvm-aot` feature.

**Acceptance Criteria**
- [ ] `cargo build` (without feature) passes (builds without LLVM environment).
- [ ] `cargo build -F llvm-aot` passes when LLVM17 environment is configured.

**Test Items**
- [ ] CI/local: both build matrices (`default` and `-F llvm-aot`) pass at least one platform.
- [ ] Minimal smoke: `cargo test -F llvm-aot` can start and execute an empty test module (only validates linking).

---

### 0.2 LLVM Environment Detection and Error Messages

**Goal**
- Add actionable error hints at build time/runtime when `llvm-config`/LLVM directory is missing (how to install/how to set prefix variables).

**Acceptance Criteria**
- [ ] When LLVM is missing, error message includes: expected version (17), available environment variables (like `LLVM_SYS_170_PREFIX` or `LLVM_CONFIG_PATH`) and example paths.

**Test Items**
- [ ] On a machine without LLVM environment, execute `cargo build -F llvm-aot`, output is complete and doesn't panic (compile-time error is acceptable).

---

### 0.3 Spawn Model Implementation Constraint Lock-in (RFC-001/008 Alignment)

**Goal**
- Clearly define and solidify the following implementation constraints (write into code comments/development documentation and test cases):
  - `spawn` is only allowed within `@block` scope (must be guarded at parse/type check/IR stages).
  - `@block` semantics are "eager evaluation", provided by standard library capability (can start with compiler built-in implementation for MVP, but must preserve interface for future delegation to standard library).
  - DAG is constructed only within function blocks; must carry stable `node_id` and `span` (supports error propagation/error graph).
  - Resource type (Resource) drives ControlEdge generation, avoiding additional user-visible effect annotation system.
  - **Parallel Safety Constraints (RFC-001/009)**: cross-thread parallel only allowed when captured/returned values of nodes satisfy `Send + Sync` (or language-side equivalent constraints); otherwise must degrade to serial (or execute on single worker).

**Acceptance Criteria**
- [ ] Compiler gives explicit error with Span when `spawn` is used in illegal scenarios.
- [ ] Semantic differences of `@block/@eager/@auto` are observable and testable in minimal examples.
- [ ] Documentation (this plan) is consistent with key decisions in RFC-001/008/018, no self-contradictory entries.

**Test Items**
- [ ] `spawn` position restriction test: spawn outside @block must error.
- [ ] DAG scope test: confirm DAG does not expand across function bodies (node count independent of call depth).
- [ ] Send/Sync constraint tests:
  - `spawn` capturing non-`Send` value must error (with Span).
  - Under `@auto`, nodes containing non-`Send + Sync` values must not be scheduled across threads (can verify using `std.concurrent.thread_id` statistics).

---

## Phase 1: LLVM Backend Skeleton and Selection Switch (1-2 days)

### 1.1 New Backend Module and Align with RFC-018 Directory Structure

**Goal**
- Add `src/backends/llvm/`, containing: `mod.rs / context.rs / types.rs / values.rs / codegen.rs / dag.rs / scheduler.rs / tests.rs` (subsequent merge/split allowed).
- In `src/backends/mod.rs`, expose module via `#[cfg(feature = "llvm-aot")] pub mod llvm;`.

**Acceptance Criteria**
- [ ] `cargo test` (default feature) passes.
- [ ] `cargo test -F llvm-aot` passes (even if LLVM backend doesn't have complete functionality yet).

**Test Items**
- [ ] Unit: at least 1 compile-time test in `src/backends/llvm/tests.rs` can run (only validates module can be referenced).

---

### 1.2 Backend Selection: CLI/Library Side Injection Points

**Goal**
- Add `--backend` parameter (ValueEnum) to CLI `Run` subcommand: `interpreter` (default) / `llvm` (requires feature).
- Add backend selection branch in `yaoxiang::run_*` path, abstracted as `fn make_executor(kind, config) -> Box<dyn Executor>` (or enum dispatch, trait object avoidance also acceptable).

**Acceptance Criteria**
- [ ] `yaoxiang run file.yx` still uses interpreter, behavior unchanged.
- [ ] `yaoxiang run --backend llvm file.yx`: if feature not enabled, gives explicit error; if feature enabled, enters LLVM execution path (even if temporarily returning "not implemented", must be a controlled error).

**Test Items**
- [ ] CLI parameter parsing test (add in `tests/integration`).
- [ ] Negative test: passing `--backend llvm` without feature returns readable error message.

---

## Phase 2: Stable ABI (RtValue/RtContext) and Runtime API (3-5 days)

> This phase is key to "LLVM generated code being executable": the cross-boundary value representation must be stabilized first.

### 2.1 Define `RtValue` (Stable C ABI)

**Goal**
- Define `#[repr(C)] struct RtValue { tag: u8, _pad: [u8; 7], payload: u64 }` (or 16-byte structure, keep alignment simple).
- Define `#[repr(u8)] enum RtTag { Unit, Bool, Int, Float, Handle, Async, Error }` (minimal set; extend later).
- Conventions:
  - Int: `payload` = `i64` bits
  - Float: `payload` = `f64` bits
  - Bool: 0/1
  - Handle: `payload` = `usize` (extended to u64)
  - Async: `payload` = `TaskId` (u64)
  - Error: `payload` = error code or pointer (MVP can use error code first)

**Acceptance Criteria**
- [ ] `RtValue` can be safely constructed/read inside Rust (no UB), with `Debug` output and basic assertion utility functions.
- [ ] Aligned with LLVM IR: able to create same-layout struct type in inkwell (consistent field order/size).

**Test Items**
- [ ] `RtValue` roundtrip: int/float/bool/unit encode/decode unit tests.
- [ ] ABI size test: `size_of::<RtValue>()` and `align_of::<RtValue>()` fixed (hardcoded assertions, prevent future accidental changes).

---

### 2.2 Define `RtContext` (Runtime Context)

**Goal**
- Define `#[repr(C)] struct RtContext`, containing only pointers/integers:
  - `heap: *mut Heap`
  - `ffi: *const FfiRegistry`
  - `scheduler: *mut DynScheduler` (or pointer to concrete implementation)
  - `max_parallelism: usize`
  - `error_graph: *mut ErrorGraph` (optional: for RFC-001 error propagation recording; can be null in MVP)
  - Reserved fields (version number/flags), but keep minimal (KISS).

**Acceptance Criteria**
- [ ] `RtContext` can be constructed by interpreter/LLVM executor and passed to generated code.
- [ ] Does not contain Rust unstable layout fields (directly embedding `Heap`/`FfiRegistry` values is prohibited).

**Test Items**
- [ ] Memory safety tests for constructing/destroying `RtContext` (no real LLVM needed).

---

### 2.3 Runtime C API: Minimum Function Set for Generated Code to Call

**Goal**
- Provide `#[no_mangle] extern "C"` functions (unified naming prefix `yx_rt_*`), MVP minimum set:
  - `yx_rt_force(ctx: *mut RtContext, v: RtValue) -> RtValue`
  - `yx_rt_lazy_call(ctx, callee: *const u8 /*function pointer*/, args: *const RtValue, argc: usize, node_id: u32, span_id: u32) -> RtValue`
  - `yx_rt_native_call(ctx, name_ptr: *const u8, name_len: usize, args: *const RtValue, argc: usize, node_id: u32, span_id: u32) -> RtValue`
  - `yx_rt_panic(code: u32)` or `yx_rt_trap(msg_ptr, len)` (for debugging)
- Constraint: AOT generated code **interacts only through the above API**, does not directly manipulate Rust structures.

**Acceptance Criteria**
- [ ] Runtime API compiles without LLVM (controlled by `llvm-aot` feature: API can be always-present or only provided under feature, but must be testable).
- [ ] `yx_rt_native_call` can call handlers in `FfiRegistry` (MVP only supports Int/Float/Bool/Unit parameters and return values; returns Error RtValue if unsupported), and records `node_id/span_id` to error graph on failure (if enabled).

**Test Items**
- [ ] Pure Rust unit test: directly call `yx_rt_native_call`, verify `std.io.println` (or self-registered function) path works.
- [ ] Error path test: pass non-existent native name, returns `Error` RtValue and can be converted to `ExecutorError::FunctionNotFound`.

---

## Phase 3: LLVM Codegen Infrastructure (2-3 days)

### 3.1 LLVM Context/Module/TargetMachine Initialization

**Goal**
- `context.rs`: encapsulate inkwell `Context/Module/Builder` lifecycle.
- Initialize Target: set target triple + data layout based on `PlatformDetector` (support `LLVM_TARGET`) and host triple.
- Support outputs:
  - LLVM IR (`.ll`) for debugging
  - Object (`.o/.obj`) for AOT

**Acceptance Criteria**
- [ ] For any empty `BytecodeModule`, can generate an LLVM Module containing `main` (even if function body only returns Unit).
- [ ] IR can be verified (call LLVM verify; return readable error on failure).

**Test Items**
- [ ] Unit: generate minimal module and verify passes.
- [ ] Snapshot test (optional): string containment assertions on key `.ll` fragments (avoid brittle full snapshots).

---

### 3.2 `TypeMap`: YaoXiang Type → LLVM Type (MVP)

**Goal**
- `types.rs`: implement `fn llvm_type(yao_type: &Type) -> BasicTypeEnum`, covering at minimum:
  - `Int` → i64
  - `Float` → f64
  - `Bool` → i1
  - `()`/Void → void (or unified return via `RtValue(Unit)`)
- Strategy selection (to reduce ABI surface): **all functions uniformly return `RtValue`** (instead of returning by type), letting codegen and scheduler/FFI handle uniformly; type information used for static checking and generating `RtValue` construction/deconstruction logic.

**Acceptance Criteria**
- [ ] `TypeMap` mapping for above types is stable, and function signatures in LLVM IR are consistent: `fn(*mut RtContext, *const RtValue, usize) -> RtValue`.

**Test Items**
- [ ] `TypeMap` unit test: given `Type::Int/Float/Bool/Void`, LLVM type generation succeeds.
- [ ] Generated function signatures are retrievable in LLVM module and parameter/return type matching is asserted.

---

## Phase 4: Instruction Translation MVP (5-8 days)

### 4.1 Virtual Register to LLVM Value Mapping (SSA Minimum Implementation)

**Goal**
- `values.rs`: implement virtual register `Reg(u16)` → LLVM `Value` mapping table (managed per basic block scope).
- Convention: all register values are represented as `RtValue` (avoid type explosion/ABI inconsistency), operations/comparisons require forced/unpacking via helpers.

**Acceptance Criteria**
- [ ] Generated code can correctly merge register values after control flow forks (using phi or unified type handling at `RtValue` level).

**Test Items**
- [ ] Unit: generate IR for BytecodeFunction containing if/else, verify passes.
- [ ] Regression: multiple assignments to same register don't cause use-before-def (insert trap/error in debug mode).

---

### 4.2 Translate Core Instruction Subset (Covering "Runnable")

**Goal**
- `codegen.rs`: implement at minimum the following `BytecodeInstr`:
  - `LoadConst` (Int/Float/Bool/String initially limited: String can degrade to Error or not supported first)
  - `Mov`
  - `BinaryOp` (Add/Sub/Mul/Div: separate paths for Int and Float)
  - `Compare` (Eq/Lt/Gt etc.)
  - `Jmp/JmpIf/JmpIfNot/Return/ReturnValue`
  - `CallNative` (via `yx_rt_native_call`)
  - `CallStatic` (two strategies: `@block` direct call; `@auto` goes through `yx_rt_lazy_call` returning Async)
- Mandatory rule: operands participating in arithmetic/comparison/branching must first go through `yx_rt_force` ("trigger evaluation when value is needed" for transparent concurrency).

**Acceptance Criteria**
- [ ] AOT backend can execute simple programs:
  - Pure arithmetic
  - if/else
  - Call `std.io.println` for output
- [ ] Unsupported instructions produce readable errors (not panic).

**Test Items**
- [ ] Integration: new `tests/integration/llvm_aot_smoke.rs` (feature gate), run 5 program snippets and assert results/outputs (output via stdout redirection).
- [ ] Negative: encountering `MakeClosure/CallVirt/...` returns clear "not implemented" error.

---

## Phase 5: Machine Code Artifact and Execution (AOT Closure) (3-6 days)

### 5.1 Artifact Format: Object + Metadata (Two Files, Keep Simple First)

**Goal**
- `CompiledArtifact` (Rust-side structure) minimum contains:
  - `object_bytes: Vec<u8>` (COFF/ELF/Mach-O)
  - `dag_meta: DAGMetadata` (can be empty initially)
  - `entries: Vec<EntryPoint>` (at least main)
  - `type_info: TypeInfo` (MVP can be empty)
- Output strategy:
  - `yaoxiang build-aot input.yx -o out/` generates `program.obj` + `program.dag.ron` (or `.json`)
  - `yaoxiang run --backend llvm` defaults to "in-memory compile + direct execution" (no disk), convenient for development.

**Acceptance Criteria**
- [ ] build-aot can generate two files, and metadata can be deserialized.
- [ ] run/llvm path can execute without relying on disk files.

**Test Items**
- [ ] File generation test: verify `.obj` is non-empty, `.dag.ron` is parseable and version number matches.
- [ ] Compatibility test: different build_mode (Debug/Release) outputs different optimization levels (at least distinguishable).

---

### 5.2 Execution: "In-Memory Execution" First, Then "Disk Load" (Accept in Two Steps)

**Goal**
- Step A (deliver first): use LLVM ExecutionEngine (or ORC JIT) to execute already generated module (for semantic closure verification, highest development efficiency).
- Step B (true AOT): use TargetMachine to generate object bytes, and execute via "dynamic library linking/loading" path:
  - Link object to `.dll/.so/.dylib` (call system linker or lld; as extra requirement for `llvm-aot` feature)
  - Use `libloading` to load symbols and call entry function

**Acceptance Criteria**
- [ ] Step A: `--backend llvm` can execute in same process (no external linker dependency).
- [ ] Step B: artifacts generated by `build-aot` can be loaded and executed by `run-aot` (new subcommand or internal path).

**Test Items**
- [ ] Step A: unit/integration tests run by default (fast development).
- [ ] Step B: marked as "requires external linker" optional integration tests (enabled in CI when environment available; manual locally).

---

## Phase 6: DAG Metadata Generation (4-7 days)

### 6.1 Define `DAGMetadata` (Versioned)

**Goal**
- `dag.rs` defines:
  - `dag_version: u32`
  - `nodes: Vec<DAGNode>` (carrying `node_id` and `span_id`, used for error propagation)
  - `edges: Vec<DAGEdge>` (with edge type: Data/Control/Spawn)
- `DAGEdge` minimum contains:
  - `from: u32`
  - `to: u32`
  - `kind: EdgeKind { Data, Control, Spawn }`
- Conflict/scheduling rules (RFC-001):
  - DataEdge + DataEdge: can parallel (if no other dependencies)
  - Any combination involving ControlEdge: must serialize (maintain order)
- `DAGNode` minimum contains:
  - `node_id: u32` (unique within function)
  - `ip: u32` (call instruction position)
  - `strategy: ScheduleStrategy { Serial, Eager, Lazy }` (derived from annotation or default strategy)
  - `span_id: u32` (localization and error graph)
  - `thread_safety: ThreadSafety { SendSync, LocalOnly }` (derived from type system; `LocalOnly` nodes are prohibited from cross-thread scheduling)
- Convention: nodes only describe "call sites that can be scheduled", parameters are captured at runtime during `yx_rt_lazy_call` (avoids static expression evaluation complexity).

**Acceptance Criteria**
- [ ] `DAGMetadata` can be serialized/deserialized (using existing `ron` or `serde_json`).
- [ ] Load errors when `dag_version` doesn't match.

**Test Items**
- [ ] Serialization roundtrip unit test.
- [ ] Version mismatch unit test (manually construct old version).
- [ ] `thread_safety` derivation test: cover at least 1 `LocalOnly` scenario, verify it doesn't execute across threads under `num_workers>1`.

---

### 6.2 Resource Types and ControlEdge Generation (Minimum Usable Side Effect Abstraction)

**Goal**
> **Update**: per RFC-001, side effects are not expressed through additional effect system, but abstracted as resource operations via **resource types (Resource)**, generating ControlEdge.

- Resource operation recognition (MVP):
  - If any parameter type of a call is `Resource` or its derived resource type (like `Console/FilePath/HttpUrl/DBUrl`), that call site is a resource operation;
  - Resource operation functions in standard library must have recognizable type constraints (recommended approach: explicitly carry resource types in std export signatures; or mark "resource operation" in FFI registry export metadata and associate resource parameter position).
- ControlEdge Generation (MVP):
  - Multiple resource operations on **the same resource value/handle** (same SSA value/same constant stored key) add ControlEdge in program order (automatic serialization).
  - When unable to determine if resources are the same (aliasing/complex sources), conservative serialization by default (can introduce explicit unsafe parallel hints as extension in future).
  - **Resource identifier propagates along data flow (RFC-001)**: resource conflict detection is based on "value equality/same source" rather than "same resource type" (two different `FilePath` values can parallel; same `FilePath` value must serialize).

**Acceptance Criteria**
- [ ] For example: `log → save → log` forms ControlEdge due to Console/FilePath resources, stable serialization; different resource operations can parallel.
- [ ] Resource operation recognition is stable (same input module yields consistent results).

**Test Items**
- [ ] Unit: resource type parameter recognition test (when Resource parameter exists, ControlEdge must be generated).
- [ ] Unit: two resource operations on same resource value (same variable/same constant) must generate ControlEdge; different resource values (different variables/different constants) may not generate.
- [ ] Integration: run example with multiple `std.io.println/std.io.write_file`, assert output/write order matches interpreter.

---

### 6.3 L1 Automatic Fallback (Small Functions Downgrade to @block, Avoid Scheduling Overhead)

> **Source**: RFC-001 5.2 (L1 Automatic Fallback).  
> **Purpose**: reduce DAG/scheduler overhead for small functions without changing semantics (especially for unified behavior between interpreter backend and AOT backend).

**Goal**
- Lightweight threshold judgment on functions at compile time; if any condition is met, downgrade that function's (or some blocks within it) default strategy to `Serial`:
  - Instruction count `< 50`
  - DAG node count `< 10`
- Expose switches via CLI/config (MVP: internal config also acceptable):
  - `--l1-threshold=<N>` adjust threshold
  - `--no-l1-fallback` disable automatic fallback

**Acceptance Criteria**
- [ ] Small functions under `@auto` actually don't enter DAG/scheduler queue (can be verified via statistics fields or trace), result consistent with non-fallback.
- [ ] Large functions unaffected; forced annotations `@eager/@block` have higher priority than automatic fallback.

**Test Items**
- [ ] Unit: construct boundary values (49/50 instructions, 9/10 nodes) verify fallback triggers or not.
- [ ] Regression: same source code with/without fallback enabled, output and return value consistent.

---

## Phase 7: Runtime DAG Scheduler (Lazy Scheduling Core) (6-10 days)

### 7.1 Implement Task Model (Docking with `RtValue::Async`)

**Goal**
- `scheduler.rs` (or migrated to `src/backends/runtime/` for "interpreter/LLVM shared") implements:
  - `TaskId` allocation
  - `TaskState { Pending, Running, Completed(RtValue), Failed(RtValue) }`
  - `spawn(task_fn_ptr, args, deps, node_id, span_id)`: create task but can delay start (used for error propagation/error graph)
  - `force(task_id)`: trigger execution by dependency topology and wait for result

**Acceptance Criteria**
- [ ] `yx_rt_lazy_call` returns `Async(TaskId)` and task is recorded (not immediately executed).
- [ ] `yx_rt_force` can trigger task execution and return result (including dependency chain).

**Test Items**
- [ ] Pure Rust: use mock "compiled fn" pointer (actually Rust `extern "C"` function) to construct 3-node DAG, verify dependency order and correct results.
- [ ] Error propagation: downstream force gets Error, and doesn't deadlock.

---

### 7.2 Implement Scheduling Strategies (Serial/Eager/Lazy)

**Goal**
- `Serial` (corresponds to `@block`): don't create Async; call executes immediately; scheduler interface can be bypassed.
- `Eager`: create task but immediately `force` (guarantee order), for debugging/semantic alignment.
- `Lazy` (default `@auto`): `force` only when value is needed; scheduler can proactively start "ready" tasks in background window (subject to concurrency limit).
- Bottom-up (RFC-001/008): runtime behavior should reflect "reverse-trigger evaluation from where result is needed" characteristic; **DAG branches/islands not consumed and not involving resource operations (no ControlEdge) should not execute**, reducing overhead; resource operations must be guaranteed order by ControlEdge and completed (consistent with interpreter).
- Background DAG (RFC-018): when multiple long-running/infinite loop tasks exist in same scope, scheduler needs to provide **cooperative slicing** (e.g., based on budget or explicit `yield_now`), avoiding main DAG starvation and "stuck in loop".

**Acceptance Criteria**
- [ ] Same program yields consistent results under Serial/Eager/Lazy three strategies.
- [ ] Under Lazy, when a call result is never forced/used, task doesn't execute (Lazy Task Creation).

**Test Items**
- [ ] Comparison test: three strategies produce identical output.
- [ ] Lazy skip test: write a branch/variable that "computes but doesn't use", assert corresponding task execution count is 0 (scheduler statistics field).
- [ ] Background slicing test: construct 2 long-running tasks + 1 main task, assert all three make progress within time window (can use counters or `thread_id` + log statistics).

---

### 7.3 Concurrency Control and Granularity Control

**Goal**
- Concurrency upper limit: `max_parallelism = num_workers * 2` (RFC-018 recommendation).
- Resource constraints: operations on same resource must execute serially per ControlEdge (RFC-001 resource type rules), scheduler must not reorder ControlEdge.
- Thread safety constraints (RFC-001/009): scheduler must respect `DAGNode.thread_safety`:
  - `SendSync`: can execute across workers (subject to concurrency limit and dependency constraints)
  - `LocalOnly`: prohibited from cross-thread scheduling/work-stealing; when necessary, degrade to serial (or fixed to execute on creating worker)
- Adaptive granularity (MVP): when pending task count far exceeds concurrency limit, batch submit "multiple ready tasks **without ControlEdge constraints**" (implemented as same worker executing batch sequentially, reducing scheduling overhead).

**Acceptance Criteria**
- [ ] Large number of independent, unconstrained tasks (1e4) don't cause memory explosion (task structure O(concurrency) or controllable upper bound).
- [ ] `LocalOnly` nodes under `num_workers>1` don't execute across threads (can verify with `std.concurrent.thread_id`).
- [ ] Output/side effect order of resource operations (e.g., `std.io.*`) strictly maintains interpreter order.

**Test Items**
- [ ] Stress test unit: construct 10000 mock tasks, peak memory/task count controlled (use statistical assertions, exact memory measurement not required).
- [ ] LocalOnly integration test: construct example containing `LocalOnly` nodes, assert execution thread ID doesn't change under `num_workers>1`.
- [ ] Resource order integration test: multiple resource operations (println/write_file) must output/write to disk in source order.

---

### 7.4 Error Propagation and Error Graph Recording (RFC-001 Minimum Closure)

**Goal**
- Define minimum `ErrorGraph` data structure (can be used for debugging/trace first):
  - Nodes: `node_id + span_id + message/error_code`
  - Edges: `from_node_id -> to_node_id` (represents "error propagates from dependency node to consumer node")
- Recording strategy (RFC-001 resolution):
  - Errors propagate along dependency edges upstream, **not dependent on actual execution order**;
  - DAG is constructed only within functions, so error graph is also limited to function level, avoiding memory explosion.
- Aligned with ABI:
  - `yx_rt_lazy_call/yx_rt_native_call` must carry `node_id/span_id` (locked in Phase 2.3)
  - When task fails and `force` returns error, write to `ErrorGraph` (if `ctx.error_graph != null`)

**Acceptance Criteria**
- [ ] When bottom node in dependency chain fails, top-level consumer receives error (and can locate to failing node's span).
- [ ] Under parallel execution, error propagation path is stable and reproducible (independent of scheduling order).

**Test Items**
- [ ] Unit: construct 3-node dependency chain, simulate middle node failure, assert ErrorGraph edges are `leaf->mid->root`.
- [ ] Concurrent regression: multiple runs under num_workers>1, ErrorGraph structure consistent.

---

## Phase 8: Syntax Annotation Integration (@block/@eager/@auto) (5-8 days)

### 8.1 Frontend Support Annotations and Propagate to Bytecode/Metadata

**Goal**
- Parse layer: recognize function/block annotations `@block`, `@eager`; default is `@auto`.
- Parse/type check: enforce `spawn` can only appear within `@block` scope (RFC-001/008).
- IR/Bytecode: carry default strategy in `BytecodeFunction` or additional side-table; at call site, determine whether to go lazy/eager/direct.

**Acceptance Criteria**
- [ ] No annotation: default Lazy (@auto).
- [ ] `@block`: within this scope, no Async created, behavior consistent with interpreter serial.
- [ ] `@eager`: create task then immediately force (consistent result, convenient for debugging).

**Test Items**
- [ ] Frontend: parse/IR generation test with annotations (AST/IR assertions).
- [ ] Backend: same source code with different annotations, runtime behavior matches strategy.

---

### 8.2 Standard Library: `@block` and `spawn` Runtime Interface (Full Runtime)

> **Source**: RFC-008 (@block provided by standard library), RFC-001 (spawn wait semantics controlled by standard library).

**Goal**
- Add standard library runtime module (suggested path: `std.runtime` or `std.full`), providing:
  - `block`: force eager evaluation (equivalent to setting scope strategy to `Serial`/not entering DAG queue)
  - `spawn`/`join_all` (or implicit join): create concurrent tasks and wait for completion within `@block` scope
- Compiler can start with built-in implementation for MVP, but must abstract interface that can be delegated to standard library (avoid future refactoring cost).

**Acceptance Criteria**
- [ ] `spawn { ... }` blocks within `@block` function can execute concurrently, and complete before scope ends (no "silent background task leak").
- [ ] `@block` behavior is clearly distinguishable from L3 default concurrency behavior (e.g., whether entering DAG queue, whether producing Async value).

**Test Items**
- [ ] Integration: example with two `spawn { std.concurrent.sleep(50) }` takes time close to single sleep under multiple workers (coarse-grained concurrency verification).
- [ ] Negative: spawn outside @block errors (consistent with 0.3/8.1).

---

## Phase 9: End-to-End and Performance Benchmark (Continuous)

### 9.1 Interpreter Consistency Tests (Semantic Alignment)

**Goal**
- Select a set of test cases "covering instruction subset": arithmetic, branching, function calls, native IO.
- Execute same source code with interpreter and llvm backend respectively, compare:
  - Return value (if any)
  - stdout output (needs injection/redirection)
  - Error type (align `ExecutorError` as much as possible)

**Acceptance Criteria**
- [ ] In test suite, LLVM backend results consistent with interpreter.

**Test Items**
- [ ] `tests/integration/llvm_vs_interpreter.rs` (feature gate) at least 10 test cases.
- [ ] Regression: new test cases must run both backends.

---

### 9.2 Benchmark: Interpreter vs AOT (Coarse-Grained)

**Goal**
- Add benchmarks in `benches/`: pure computation (no IO), many call tasks (concurrency benefit), mixed IO (order constraints).

**Acceptance Criteria**
- [ ] AOT significantly faster than interpreter on pure computation cases (no specific multiple promised, but cannot be noticeably slower).
- [ ] Lazy scheduling overhead is observable and locatable (output scheduler stats).

**Test Items**
- [ ] criterion bench (manual/CI optional) generates report, records baseline.

---

## Assumptions and Defaults (Choices When Not Covered by Business Requirements)

- Default LLVM major version selected as **17**; if team toolchain differs, unified update `inkwell` feature and documentation.
- AOT execution path takes "two-step approach": in-memory execution first (development verification), then disk linking/loading (true AOT).
- Early `llvm-aot` only commits to covering one MVP instruction subset; closures/dynamic dispatch/exceptions and other advanced features extended on demand (encountering them returns explicit "not implemented" error).
- DAG dependency edges **can be dynamically inferred by runtime from args' Async TaskId**; compile-time edges field serves as optional optimization and debug verification first, not blocking M2 delivery.
  - **Supplement (RFC-001)**: ControlEdge's primary source is resource type rules; if resource information is missing, conservative serialization by default to guarantee correctness.