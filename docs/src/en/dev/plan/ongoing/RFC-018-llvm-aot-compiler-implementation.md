# RFC-018: LLVM AOT Compiler and L3 Transparent Concurrency (DAG Lazy Scheduling) Implementation Plan

> **Task**: Implement LLVM AOT backend + runtime DAG scheduler, delivering three scheduling strategies `@auto/@eager/@block` (L3/L2/L1)  
> **Based on RFC**: RFC-018 (draft)  
> **Dependent RFCs**: RFC-001 (spawn model and error handling), RFC-008 (three-layer runtime), RFC-009 (ownership/Arc)  
> **Date**: 2026-03-10  
> **Status**: In Progress  
> **Target Milestones**:  
> - M1: LLVM AOT (compilable and executable, serial)  
> - M2: DAG metadata + single-threaded scheduling (Standard Runtime, num_workers=1)  
> - M3: Multi-threaded parallel scheduling + granularity control (Full Runtime, num_workers>1)  
> - M4: Lazy scheduling (Lazy Task Creation) + **Resource types (Resource) side-effect abstraction** + **error propagation/error graph** + annotation integration

---

## Abstract (Implementation Closure)

- Add LLVM backend (feature gate) to `yaoxiang`, compile `BytecodeModule` into machine code (COFF/ELF/Mach-O) and load/execute at runtime.
- Introduce **stable ABI**: AOT-generated code interacts with runtime through `extern "C"` `RtValue/RtContext`, avoiding Rust enum ABI instability issues.
- Deliver the core of RFC-018: **intra-function DAG** + **lazy scheduling**. Concurrency/serial execution is jointly determined by **DAG edges (Data/Control/Spawn)** and **Resource type rules**; errors propagate along dependency edges per RFC-001 and can form error graphs.

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
   - New `RtValue` (`#[repr(C)]`) and `RtContext` (pointers/primitives only) as the interaction boundary between AOT and runtime.

---

## Key Design Constraints (Aligned with RFC-001 / RFC-008 / RFC-018)

### A. Concurrency Semantics (L1/L2/L3 are Mental Models)

- **L3 (default / @auto)**: Transparent concurrency; builds DAG; on encountering a call, returns a "deferrable evaluation" value first, **only triggers evaluation when the value is needed**.
- **L1 (@block)**: Provided by standard library (RFC-008), semantics: "force eager evaluation", does not enter DAG lazy queue; mainly for debugging and critical sequential sections.
- **L2 (spawn)**: **Can only be used within @block scope** (RFC-001/008), used to insert concurrency in synchronous code; belongs to Full Runtime capability.

### B. Three Runtime Tiers (RFC-008)

- **Embedded Runtime**: Immediate execution; optionally skip DAG construction entirely (save memory/startup); for constrained environments.
- **Standard Runtime**: DAG + Scheduler as core (lazy evaluation naturally supports async/parallel).
- **Full Runtime**: Adds WorkStealer on top of standard, plus standard library-level `@block` / `spawn` capabilities.

### C. DAG Construction Scope and Memory (RFC-001/018)

- DAG is only constructed **within a single function body/block**; does not recursively expand called function bodies (avoids error graph and DAG node explosion).
- DAG metadata must carry **stable node/edge IDs** and **Span** (for error propagation and error graph localization).

### D. Side-effect Abstraction (RFC-001: Resource Types)

- No additional "explicit side-effect annotation system" introduced; side-effects unified as **resource operations**:
  - Function calls with `Resource` (or derived resource type) in parameter types are treated as resource operations;
  - Operations on the same resource automatically form **ControlEdge** (serial); operations on different resources can be parallel;
  - When static determination of same-resource is impossible, conservatively default to serial (may introduce explicit unsafe parallel hints as extension later).

### E. Error Propagation (RFC-001)

- Errors propagate along DAG dependency edges upward (independent of actual parallel execution order), with propagation path recorded for error graph.

---

## Phase 0: Prerequisites and Constraint Lockdown (1-2 days)

### 0.1 Lock LLVM/inkwell Version and Build Method

**Goal**
- Select LLVM major version = **17** (unify team environment; Windows/Linux/macOS all have corresponding distribution packages).
- Add `inkwell` in `Cargo.toml` (enabling `llvm17-0` corresponding feature), tied under `llvm-aot` feature.

**Acceptance Criteria**
- [ ] `cargo build` (without feature) passes (builds without LLVM environment).
- [ ] `cargo build -F llvm-aot` passes when LLVM17 environment is configured.

**Test Items**
- [ ] CI/local: two build matrices (`default` and `-F llvm-aot`) at least one platform passes.
- [ ] Minimal smoke: `cargo test -F llvm-aot` can start and execute an empty test module (only validates linking).

---

### 0.2 LLVM Environment Detection and Error Messages

**Goal**
- Add build-time/runtime detection instructions: when `llvm-config`/LLVM directory is missing, provide actionable error hints (how to install/how to set prefix env var).

**Acceptance Criteria**
- [ ] When LLVM is missing, error message includes: expected version (17), available env vars (like `LLVM_SYS_170_PREFIX` or `LLVM_CONFIG_PATH`) with example paths.

**Test Items**
- [ ] Execute `cargo build -F llvm-aot` on machine without LLVM environment, output hint is complete and not panicking (compile-time error is acceptable).

---

### 0.3 spawn Model Implementation Constraint Lockdown (RFC-001/008 Alignment)

**Goal**
- Clearly define and solidify the following implementation constraints (write into code comments/dev docs and test cases):
  - `spawn` is only allowed within `@block` scope (must guard at parse/type check/IR stages).
  - `@block` semantics: "eager evaluation", provided by standard library capability (may start with compiler-builtin MVP, but must preserve interface for future move to standard library).
  - DAG only constructed within function block; must carry stable `node_id` and `span` (supports error propagation/error graph).
  - Resource types (Resource) drive ControlEdge generation, avoiding additional user-visible effect annotation system.
  - **Parallel safety constraints (RFC-001/009)**: parallel cross-thread execution only allowed when node captures/return values satisfy `Send + Sync` (or language-side equivalent constraints); otherwise must degrade to serial (or fixed to single worker execution).

**Acceptance Criteria**
- [ ] Compiler gives clear error (with Span) for illegal `spawn` scenarios.
- [ ] Semantic differences of `@block/@eager/@auto` are observable and testable in minimal examples.
- [ ] This document is consistent with key decisions in RFC-001/008/018, no self-contradicting items.

**Test Items**
- [ ] `spawn` position restriction test: spawn outside @block must error.
- [ ] DAG scope test: confirm DAG does not expand across function bodies (node count independent of call hierarchy).
- [ ] Send/Sync constraint tests:
  - `spawn` capturing non-`Send` value must error (with Span).
  - Nodes containing non-`Send + Sync` values under `@auto` must not be cross-thread scheduled (can use `std.concurrent.thread_id` statistics for verification).

---

## Phase 1: LLVM Backend Skeleton and Selection Switch (1-2 days)

### 1.1 New Backend Module and RFC-018 Directory Structure Alignment

**Goal**
- Add `src/backends/llvm/`, containing: `mod.rs / context.rs / types.rs / values.rs / codegen.rs / dag.rs / scheduler.rs / tests.rs` (subsequent merge/split allowed).
- In `src/backends/mod.rs`, expose module via `#[cfg(feature = "llvm-aot")] pub mod llvm;`.

**Acceptance Criteria**
- [ ] `cargo test` (default feature) passes.
- [ ] `cargo test -F llvm-aot` passes (even if LLVM backend doesn't implement full functionality yet).

**Test Items**
- [ ] Unit: at least 1 compile-time test in `src/backends/llvm/tests.rs` can run (validates module can be referenced).

---

### 1.2 Backend Selection: CLI/Library-side Injection Points

**Goal**
- Add `--backend` parameter (ValueEnum) to CLI `Run` subcommand: `interpreter` (default) / `llvm` (requires feature).
- Add backend selection branch in `yaoxiang::run_*` path, abstracted as `fn make_executor(kind, config) -> Box<dyn Executor>` (or enum dispatch, avoiding trait object is also acceptable).

**Acceptance Criteria**
- [ ] `yaoxiang run file.yx` still uses interpreter, behavior unchanged.
- [ ] `yaoxiang run --backend llvm file.yx`: if feature not enabled, gives clear error; if enabled, enters LLVM execution path (even if temporarily returning "not implemented", must be a controlled error).

**Test Items**
- [ ] CLI parameter parsing test (add in `tests/integration`).
- [ ] Negative test: passing `--backend llvm` without feature returns readable error message.

---

## Phase 2: Stable ABI (RtValue/RtContext) and Runtime API (3-5 days)

> This phase is critical for "LLVM-generated code can execute": the cross-boundary value representation must be stabilized first.

### 2.1 Define `RtValue` (Stable C ABI)

**Goal**
- Define `#[repr(C)] struct RtValue { tag: u8, _pad: [u8; 7], payload: u64 }` (or 16-byte struct, keeping alignment simple).
- Define `#[repr(u8)] enum RtTag { Unit, Bool, Int, Float, Handle, Async, Error }` (minimal set; extend later).
- Conventions:
  - Int: `payload` = `i64` bits
  - Float: `payload` = `f64` bits
  - Bool: 0/1
  - Handle: `payload` = `usize` (extends to u64)
  - Async: `payload` = `TaskId` (u64)
  - Error: `payload` = error code or pointer (MVP can use error code first)

**Acceptance Criteria**
- [ ] `RtValue` can be safely constructed/read internally in Rust (no UB), with `Debug` output and basic assertion utility functions.
- [ ] Aligned with LLVM IR: can create same-layout struct type in inkwell (field order/size consistent).

**Test Items**
- [ ] `RtValue` roundtrip: encode/decode unit tests for int/float/bool/unit.
- [ ] ABI size test: `size_of::<RtValue>()` and `align_of::<RtValue>()` fixed (hardcoded assertions, prevent future accidental changes).

---

### 2.2 Define `RtContext` (Runtime Context)

**Goal**
- Define `#[repr(C)] struct RtContext`, containing only pointers/integers:
  - `heap: *mut Heap`
  - `ffi: *const FfiRegistry`
  - `scheduler: *mut DynScheduler` (or pointer to concrete implementation)
  - `max_parallelism: usize`
  - `error_graph: *mut ErrorGraph` (optional: for RFC-001 error propagation recording; MVP can be null)
  - Reserved fields (version number/flags), but keep minimal (KISS).

**Acceptance Criteria**
- [ ] `RtContext` can be constructed by interpreter/LLVM executor and passed to generated code.
- [ ] Does not contain Rust unstable-layout fields (prohibited from directly embedding `Heap`/`FfiRegistry` values).

**Test Items**
- [ ] Memory safety test for constructing/destroying `RtContext` (no real LLVM needed).

---

### 2.3 Runtime C API: Minimal Function Set Called by Generated Code

**Goal**
- Provide `#[no_mangle] extern "C"` functions (unified prefix `yx_rt_*`), MVP minimum includes:
  - `yx_rt_force(ctx: *mut RtContext, v: RtValue) -> RtValue`
  - `yx_rt_lazy_call(ctx, callee: *const u8 /*function pointer*/, args: *const RtValue, argc: usize, node_id: u32, span_id: u32) -> RtValue`
  - `yx_rt_native_call(ctx, name_ptr: *const u8, name_len: usize, args: *const RtValue, argc: usize, node_id: u32, span_id: u32) -> RtValue`
  - `yx_rt_panic(code: u32)` or `yx_rt_trap(msg_ptr, len)` (for debugging)
- Constraint: AOT generated code **only interacts through the above API**, does not directly operate on Rust structs.

**Acceptance Criteria**
- [ ] Runtime API compiles without LLVM (controlled by `llvm-aot` feature: API can be always-present or only provided under feature, but must be testable).
- [ ] `yx_rt_native_call` can call `FfiRegistry` handlers (MVP only supports Int/Float/Bool/Unit parameters and return values; unsupported returns Error RtValue), and on failure records `node_id/span_id` to error graph (if enabled).

**Test Items**
- [ ] Pure Rust unit test: directly call `yx_rt_native_call`, verify `std.io.println` (or self-registered function) path works.
- [ ] Error path test: passing non-existent native name returns `Error` RtValue convertible to `ExecutorError::FunctionNotFound`.

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
- [ ] IR can be verified (call LLVM verify; on failure return readable error).

**Test Items**
- [ ] Unit: generate minimal module and verify passes.
- [ ] Snapshot test (optional): string containment assertion on key `.ll` fragments (avoid brittle full snapshots).

---

### 3.2 `TypeMap`: YaoXiang Type → LLVM Type (MVP)

**Goal**
- `types.rs`: implement `fn llvm_type(yao_type: &Type) -> BasicTypeEnum`, covering initially:
  - `Int` → i64
  - `Float` → f64
  - `Bool` → i1
  - `()`/Void → void (or use `RtValue(Unit)` for unified return)
- Strategy choice (to reduce ABI surface): **all functions uniformly return `RtValue`** (rather than by type), letting codegen and scheduler/FFI handle uniformly; type information used for static checking and generating `RtValue` construction/deconstruction logic.

**Acceptance Criteria**
- [ ] `TypeMap` mappings for above types are stable, and function signatures in LLVM IR are consistent: `fn(*mut RtContext, *const RtValue, usize) -> RtValue`.

**Test Items**
- [ ] `TypeMap` unit test: given `Type::Int/Float/Bool/Void`, LLVM type generation succeeds.
- [ ] Generated function signatures are retrievable in LLVM module and parameter/return type matches are asserted.

---

## Phase 4: Instruction Translation MVP (5-8 days)

### 4.1 Register to LLVM Value Mapping (Minimal SSA Implementation)

**Goal**
- `values.rs`: implement virtual register `Reg(u16)` → LLVM `Value` mapping table (managed per basic block scope).
- Convention: all register values represented as `RtValue` (avoid type explosion/ABI inconsistency), force/unwrap via helper before operations/comparisons.

**Acceptance Criteria**
- [ ] Generated code correctly merges register values after control flow branches (use phi or unified type handling at `RtValue` level).

**Test Items**
- [ ] Unit: generate IR for BytecodeFunction containing if/else, verify passes.
- [ ] Regression: multiple assignments to same register does not cause use-before-def (insert trap/error in debug mode).

---

### 4.2 Translate Core Instruction Subset (Covering "Runnable")

**Goal**
- `codegen.rs`: implement at least the following `BytecodeInstr`:
  - `LoadConst` (Int/Float/Bool/String limited initially: String can degrade to Error or not supported first)
  - `Mov`
  - `BinaryOp` (Add/Sub/Mul/Div: separate paths for Int and Float)
  - `Compare` (Eq/Lt/Gt etc.)
  - `Jmp/JmpIf/JmpIfNot/Return/ReturnValue`
  - `CallNative` (via `yx_rt_native_call`)
  - `CallStatic` (two strategies: `@block` direct call; `@auto` goes through `yx_rt_lazy_call` returning Async)
- Mandatory rule: operands participating in arithmetic/comparison/branching must first `yx_rt_force` ("trigger when value needed" for transparent concurrency).

**Acceptance Criteria**
- [ ] AOT backend can execute simple programs:
  - Pure arithmetic
  - if/else
  - Call `std.io.println` for output
- [ ] Unsupported instructions produce readable errors (not panic).

**Test Items**
- [ ] Integration: new `tests/integration/llvm_aot_smoke.rs` (feature gate), run 5 program snippets and assert results/output (output via stdout redirection).
- [ ] Negative: encountering `MakeClosure/CallVirt/...` returns clear "not implemented" error.

---

## Phase 5: Machine Code Artifact and Execution (AOT Closure) (3-6 days)

### 5.1 Artifact Format: Object + Metadata (Two Files, Initially Simple)

**Goal**
- `CompiledArtifact` (Rust-side structure) contains at minimum:
  - `object_bytes: Vec<u8>` (COFF/ELF/Mach-O)
  - `dag_meta: DAGMetadata` (initially can be empty)
  - `entries: Vec<EntryPoint>` (at least main)
  - `type_info: TypeInfo` (MVP initially empty)
- Output strategy:
  - `yaoxiang build-aot input.yx -o out/` generates `program.obj` + `program.dag.ron` (or `.json`)
  - `yaoxiang run --backend llvm` defaults to "in-memory compile + direct execute" (no disk), convenient for development.

**Acceptance Criteria**
- [ ] build-aot generates two files, metadata deserializable.
- [ ] run/llvm path can execute without relying on disk files.

**Test Items**
- [ ] File generation test: verify `.obj` non-empty, `.dag.ron` parseable and version number matches.
- [ ] Compatibility test: different build_mode (Debug/Release) outputs different optimization levels (at least distinguishable).

---

### 5.2 Execution: "Memory Execute" First, Then "Disk Load" (Two-Step Acceptance)

**Goal**
- Step A (deliver first): Use LLVM ExecutionEngine (or ORC JIT) to execute already generated module (for semantic closure verification, highest development efficiency).
- Step B (true AOT): Use TargetMachine to generate object bytes, and execute via "dynamic library linking/loading" path:
  - Link object to `.dll/.so/.dylib` (call system linker or lld; as extra requirement for `llvm-aot` feature)
  - Load symbols with `libloading` and call entry function

**Acceptance Criteria**
- [ ] Step A: `--backend llvm` can execute in same process (no external linker dependency).
- [ ] Step B: artifacts from `build-aot` can be loaded and executed by `run-aot` (new subcommand or internal path).

**Test Items**
- [ ] Step A: unit/integration tests run by default (fast development).
- [ ] Step B: marked as "requires external linker" optional integration test (enabled in CI with environment; local manual).

---

## Phase 6: DAG Metadata Generation (4-7 days)

### 6.1 Define `DAGMetadata` (Versioned)

**Goal**
- `dag.rs` defines:
  - `dag_version: u32`
  - `nodes: Vec<DAGNode>` (carrying `node_id` and `span_id` for error propagation)
  - `edges: Vec<DAGEdge>` (with edge type: Data/Control/Spawn)
- `DAGEdge` contains at minimum:
  - `from: u32`
  - `to: u32`
  - `kind: EdgeKind { Data, Control, Spawn }`
- Conflict/scheduling rules (RFC-001):
  - DataEdge + DataEdge: can be parallel (if no other dependencies)
  - Any combination involving ControlEdge: must be serialized (maintain order)
- `DAGNode` contains at minimum:
  - `node_id: u32` (unique within function)
  - `ip: u32` (call instruction position)
  - `strategy: ScheduleStrategy { Serial, Eager, Lazy }` (from annotation or default strategy)
  - `span_id: u32` (localization and error graph)
  - `thread_safety: ThreadSafety { SendSync, LocalOnly }` (derived from type system; `LocalOnly` nodes prohibited from cross-thread scheduling)
- Convention: nodes only describe "schedulable call sites", parameters captured at runtime in `yx_rt_lazy_call` (avoids static expression evaluation complexity).

**Acceptance Criteria**
- [ ] `DAGMetadata` serializable/deserializable (using existing `ron` or `serde_json`).
- [ ] `dag_version` mismatch on load errors.

**Test Items**
- [ ] Serialization roundtrip unit test.
- [ ] Version mismatch unit test (manually construct old version).
- [ ] `thread_safety` derivation test: cover at least 1 `LocalOnly` scenario, verify under `num_workers>1` it does not execute cross-thread.

---

### 6.2 Resource Types and ControlEdge Generation (Minimum Viable Side-effect Abstraction)

**Goal**
> **Update**: per RFC-001, side-effects are not expressed through additional effect system, but abstracted as resource operations via **resource types (Resource)**, generating ControlEdge.

- Resource operation identification (MVP):
  - If any parameter type of a call is `Resource` or derived resource type (like `Console/FilePath/HttpUrl/DBUrl`), that call site is a resource operation;
  - Standard library resource operation functions must have identifiable type constraints (recommended approach: explicitly carry resource types in std export signatures; or mark "resource operation" in FFI registry export metadata and associate resource parameter position).
- ControlEdge Generation (MVP):
  - For multiple resource operations on **the same resource value/handle** (same SSA value or same constant resident key), add ControlEdge in program order (automatic serialization).
  - When cannot determine if same resource (aliasing/complex source), conservatively default to serial (future may introduce explicit unsafe parallel hints as extension).
  - **Resource identity propagates along data flow (RFC-001)**: resource conflict detection based on "value equality/same source" not "same resource type" (two different `FilePath` values can be parallel; same `FilePath` value must be serial).

**Acceptance Criteria**
- [ ] For example: `log → save → log` forms ControlEdge due to Console/FilePath resources, stably serial; different resource operations can be parallel.
- [ ] Resource operation identification is stable (same result for same input module across runs).

**Test Items**
- [ ] Unit: resource type parameter identification test (when Resource parameter present, must generate ControlEdge).
- [ ] Unit: two resource operations on same resource value (same variable/same constant) must generate ControlEdge; different resource values (different variables/different constants) may not generate.
- [ ] Integration: run example with multiple `std.io.println/std.io.write_file`, assert output/write order matches interpreter.

---

### 6.3 L1 Automatic Fallback (Small Functions Degrade to @block, Avoid Scheduling Overhead)

> **Source**: RFC-001 5.2 (L1 Automatic Fallback).  
> **Purpose**: reduce small function DAG/scheduler overhead without changing semantics (especially for unified behavior between interpreter and AOT backends).

**Goal**
- Lightweight threshold judgment on functions at compile time; if any condition met, degrade that function's (or certain blocks within it) default strategy to `Serial`:
  - Instruction count `< 50`
  - DAG node count `< 10`
- Expose switch via CLI/config (MVP: internal config acceptable):
  - `--l1-threshold=<N>` adjusts threshold
  - `--no-l1-fallback` disables automatic fallback

**Acceptance Criteria**
- [ ] Small functions under `@auto` actually don't enter DAG/scheduler queue (verify via statistics field or trace), result consistent with non-fallback.
- [ ] Large functions unaffected; mandatory annotation `@eager/@block` takes priority over automatic fallback.

**Test Items**
- [ ] Unit: construct boundary values (49/50 instructions, 9/10 nodes) verify fallback triggers or not.
- [ ] Regression: same source code with fallback on/off, output and return value consistent.

---

## Phase 7: Runtime DAG Scheduler (Lazy Scheduling Core) (6-10 days)

### 7.1 Implement Task Model (Interface with `RtValue::Async`)

**Goal**
- `scheduler.rs` (or migrated to `src/backends/runtime/` for "interpreter/LLVM shared") implements:
  - `TaskId` allocation
  - `TaskState { Pending, Running, Completed(RtValue), Failed(RtValue) }`
  - `spawn(task_fn_ptr, args, deps, node_id, span_id)`: create task but can delay start (for error propagation/error graph)
  - `force(task_id)`: trigger execution by dependency topology and wait for result

**Acceptance Criteria**
- [ ] `yx_rt_lazy_call` returns `Async(TaskId)` and task is recorded (not immediately executed).
- [ ] `yx_rt_force` can trigger task execution and return result (including dependency chain).

**Test Items**
- [ ] Pure Rust: construct 3-node DAG with mock "compiled fn" pointer (actually Rust `extern "C"` function), verify dependency order and correct result.
- [ ] Error propagation: downstream force gets Error, no deadlock.

---

### 7.2 Implement Scheduling Strategies (Serial/Eager/Lazy)

**Goal**
- `Serial` (corresponds to `@block`): don't create Async; call executes immediately; scheduler interface can be bypassed.
- `Eager`: create task but immediately `force` (ensures order), for debugging/semantic alignment.
- `Lazy` (default `@auto`): only `force` when value needed; scheduler can proactively start "ready" tasks in background window (subject to concurrency limit).
- Bottom-up (RFC-001/008): runtime behavior should reflect "reverse trigger evaluation from where result is needed" characteristic; **unused branches/isolated DAG islands not involving resource operations (no ControlEdge) should not execute**, reducing overhead; resource operations must ensure order and completion per ControlEdge (consistent with interpreter).
- Background DAG (RFC-018): when long-running/infinite loop tasks exist in same scope, scheduler needs to provide **cooperative slicing** (e.g., budget-based or explicit `yield_now`), avoiding main DAG starvation and "stuck in loop".

**Acceptance Criteria**
- [ ] Same program produces consistent results under Serial/Eager/Lazy strategies.
- [ ] Under Lazy, when a call result is never forced/used, task does not execute (Lazy Task Creation).

**Test Items**
- [ ] Comparison test: three strategies produce identical output.
- [ ] Lazy skip test: write a "compute but don't use" branch/variable, assert corresponding task execution count is 0 (scheduler statistics field).
- [ ] Background slicing test: construct 2 long-running tasks + 1 main task, assert all three make progress within time window (can use counter or `thread_id` + log statistics).

---

### 7.3 Concurrency Control and Granularity Control

**Goal**
- Concurrency limit: `max_parallelism = num_workers * 2` (RFC-018 recommendation).
- Resource constraints: operations on same resource must execute serially per ControlEdge (RFC-001 resource type rules), scheduler must not disrupt ControlEdge order.
- Thread safety constraints (RFC-001/009): scheduler must respect `DAGNode.thread_safety`:
  - `SendSync`: can execute across workers (subject to concurrency limit and dependency constraints)
  - `LocalOnly`: prohibited from cross-thread scheduling/work-stealing theft; degrade to serial if necessary (or fixed to creating worker)
- Adaptive granularity (MVP): when pending task count far exceeds concurrency limit, merge "multiple ready tasks **without ControlEdge constraints**" for batch submission (implemented as same worker sequentially executing batch, reducing scheduling overhead).

**Acceptance Criteria**
- [ ] Large number of independent, unconstrained tasks (1e4) does not cause memory explosion (task structure O(concurrency) or controlled upper bound).
- [ ] `LocalOnly` nodes under `num_workers>1` do not execute cross-thread (verify with `std.concurrent.thread_id`).
- [ ] Resource operations (e.g., `std.io.*`) output/side-effect order strictly maintains interpreter order.

**Test Items**
- [ ] Stress unit: construct 10000 mock tasks, peak memory/task count controlled (use statistical assertions, exact memory measurement not required).
- [ ] LocalOnly integration test: construct example containing `LocalOnly` nodes, under `num_workers>1` assert its executing thread ID does not change.
- [ ] Resource order integration test: multiple resource operations (println/write_file) must output/write to disk in source order.

---

### 7.4 Error Propagation and Error Graph Recording (RFC-001 Minimum Closure)

**Goal**
- Define minimal `ErrorGraph` data structure (initially for debugging/trace only):
  - Nodes: `node_id + span_id + message/error_code`
  - Edges: `from_node_id -> to_node_id` (indicating "error propagates from dependency node to consumer node")
- Recording strategy (RFC-001 resolution):
  - Errors propagate upstream along dependency edges, **independent of actual execution order**;
  - DAG constructed only within function, so error graph also limited to function level, avoiding memory explosion.
- Alignment with ABI:
  - `yx_rt_lazy_call/yx_rt_native_call` must carry `node_id/span_id` (locked in Phase 2.3)
  - On task failure and `force` returning error, write to `ErrorGraph` (if `ctx.error_graph != null`)

**Acceptance Criteria**
- [ ] When bottom node in dependency chain fails, top-level consumer receives error (and can localize to failed node's span).
- [ ] Under parallel execution, error propagation path is stable and reproducible (independent of scheduling order).

**Test Items**
- [ ] Unit: construct 3-node dependency chain, simulate middle node failure, assert ErrorGraph edges are `leaf->mid->root`.
- [ ] Concurrency regression: multiple runs under num_workers>1, ErrorGraph structure consistent.

---

## Phase 8: Syntax Annotation Integration (@block/@eager/@auto) (5-8 days)

### 8.1 Frontend Support Annotations and Pass to Bytecode/Metadata

**Goal**
- Parser layer: recognize function/block annotations `@block`, `@eager`; default `@auto`.
- Parse/type check: enforce `spawn` only in `@block` scope (RFC-001/008).
- IR/Bytecode: carry default strategy in `BytecodeFunction` or extra side-table; at call-site can decide lazy/eager/direct path.

**Acceptance Criteria**
- [ ] No annotation: default Lazy (@auto).
- [ ] `@block`: no Async created in scope, behavior consistent with interpreter serial.
- [ ] `@eager`: create task then immediately force (consistent result, convenient for debugging).

**Test Items**
- [ ] Frontend: parse/IR generation tests with annotations (AST/IR assertions).
- [ ] Backend: same source with different annotations, run behavior matches strategy.

---

### 8.2 Standard Library: `@block` and `spawn` Runtime Interfaces (Full Runtime)

> **Source**: RFC-008 (@block provided by standard library), RFC-001 (spawn wait semantics controlled by standard library).

**Goal**
- Add standard library runtime module (suggested path: `std.runtime` or `std.full`), providing:
  - `block`: force eager evaluation (equivalent to setting scope strategy to `Serial`/not entering DAG queue)
  - `spawn`/`join_all` (or implicit join): create concurrent tasks within `@block` scope and wait for completion
- Compiler can start with builtin implementation MVP, but must abstract interfaces that can be moved to standard library (avoid future refactoring cost).

**Acceptance Criteria**
- [ ] `spawn { ... }` blocks within `@block` function can execute concurrently, and complete before scope ends (no "silent background task leak").
- [ ] `@block` behavior clearly distinguishable from L3 default concurrency behavior (e.g., whether entering DAG queue, whether producing Async values).

**Test Items**
- [ ] Integration: example with two `spawn { std.concurrent.sleep(50) }` under multiple workers takes close to single sleep duration (coarse-grained concurrency verification).
- [ ] Negative: spawn outside @block errors (consistent with 0.3/8.1).

---

## Phase 9: End-to-End and Performance Benchmarks (Continuous Progress)

### 9.1 Interpreter Consistency Tests (Semantic Alignment)

**Goal**
- Select a set of test cases "covering instruction subset": arithmetic, branching, function calls, native IO.
- Execute same source with interpreter and llvm backend respectively, compare:
  - Return values (if any)
  - stdout output (needs injection/redirection)
  - Error types (align `ExecutorError` as much as possible)

**Acceptance Criteria**
- [ ] In test suite, LLVM backend and interpreter results consistent.

**Test Items**
- [ ] `tests/integration/llvm_vs_interpreter.rs` (feature gate) at least 10 test cases.
- [ ] Regression: new test cases must run on both backends.

---

### 9.2 Benchmark: Interpreter vs AOT (Coarse-Grained)

**Goal**
- Add benchmarks in `benches/`: pure computation (no IO), many call tasks (concurrency benefit), mixed IO (sequential constraints).

**Acceptance Criteria**
- [ ] AOT significantly faster than interpreter on pure computation cases (no specific speedup promised, but must not be noticeably slower).
- [ ] Lazy scheduling overhead observable and traceable (output scheduler stats).

**Test Items**
- [ ] Criterion bench (manual/CI optional) generates report, record baseline.

---

## Assumptions and Defaults (Choices When Not Covered by Business Requirements)

- Default LLVM major version is **17**; if team toolchain differs, uniformly modify `inkwell` feature and documentation.
- AOT execution path takes "two-step approach": memory execution first (development verification), then disk linking/loading (true AOT).
- Initial `llvm-aot` only commits to covering one MVP instruction subset; advanced features like closures/dynamic dispatch/exceptions return clear "not implemented" error on encountering (extend later as needed).
- DAG dependency edges **can be dynamically derived at runtime from args' Async TaskId**; compile-time edges field first as optional optimization and debug validation, does not block M2 delivery.
  - **Supplement (RFC-001)**: ControlEdge's primary source is resource type rules; if resource information is missing, conservatively default to serial for correctness.