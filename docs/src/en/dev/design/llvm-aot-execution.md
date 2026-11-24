---
title: 'LLVM AOT Backend Integration Execution Plan'
description:
  'For compiler contributors: from a native Linux sample to dual-backend equivalence acceptance,
  defining shared IR prerequisites, runtime boundaries, and release criteria.'
---

# LLVM AOT Backend Integration Execution Plan

**First deliverable**: On Linux x86-64, compile a real YaoXiang source program into an ELF
executable. When run on a comparable system without LLVM or the YaoXiang VM, it must both compute
the correct result and identify the source location when an error occurs. Producing LLVM IR text
does not count as completion.

**Status: pending review, not yet scheduled.**
[RFC-018](../../rfc/accepted/018-llvm-aot-compiler.md) has accepted the direction of LLVM
ahead-of-time (AOT) compilation;
[tracking issue #134](https://github.com/ChenXu233/YaoXiang/issues/134) is the entry point for this
work. This document proposes a delivery sequence; it neither registers nor starts implementation
tasks. [RFC-039](../../rfc/accepted/039-compiler-architecture.md) separates backend selection from
compiler refactoring: the prototype does not need to wait for the entire refactoring, and production
code generation must not take over the P7 SSA work.

## Integration Point

The table below lists the facts to revalidate before starting work. Source links are pinned to the
same
[Git revision](https://github.com/ChenXu233/YaoXiang/commit/e64139dc3d10fb6e376ac6980a04a2ef2e296432);
they are not requirements for a future directory layout.

| Existing boundary                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Constraint on implementation                                                                                                                                                                                                                                                                                                                                                                                            |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [Driver stage table](https://github.com/ChenXu233/YaoXiang/blob/e64139dc3d10fb6e376ac6980a04a2ef2e296432/src/driver/program.rs): A single file is monomorphized after IR generation; multiple files have their IR merged first, followed by processing whole-program instantiation requests.                                                                                                                                                                                                                                                                                                                                   | Obtain the final `ModuleIR` after compilation succeeds and all obligations have been discharged. Follow the existing multi-file orchestration; do not create a separate parsing or type-checking pipeline. Do not invoke LLVM when the frontend fails.                                                                                                                                                                  |
| The default output of the [build command](https://github.com/ChenXu233/YaoXiang/blob/e64139dc3d10fb6e376ac6980a04a2ef2e296432/src/main.rs) is `.42`; the [bytecode generator](https://github.com/ChenXu233/YaoXiang/blob/e64139dc3d10fb6e376ac6980a04a2ef2e296432/src/middle/passes/codegen/mod.rs) consumes module IR; the [`Executor`](https://github.com/ChenXu233/YaoXiang/blob/e64139dc3d10fb6e376ac6980a04a2ef2e296432/src/backends/mod.rs) consumes bytecode and executes it.                                                                                                                                           | Let bytecode and LLVM diverge from the same module IR. LLVM must not infer types from `.42` or implement a bytecode execution interface.                                                                                                                                                                                                                                                                                |
| [Slot generation](https://github.com/ChenXu233/YaoXiang/blob/e64139dc3d10fb6e376ac6980a04a2ef2e296432/src/middle/ir/ir_gen.rs) fills temporary slots with `Int(64)`; the [IR verifier](https://github.com/ChenXu233/YaoXiang/blob/e64139dc3d10fb6e376ac6980a04a2ef2e296432/src/middle/ir/verify.rs) interprets current jumps using flattened instruction indices. In the [modulo snapshot](https://github.com/ChenXu233/YaoXiang/blob/e64139dc3d10fb6e376ac6980a04a2ef2e296432/src/middle/ir/tests/snapshots/tests/yaoxiang/01-syntax/basics/modulo_math.snap), the same slot first holds a string and later holds an integer. | A slot number is not a single-assignment value, and a padding type is not a type proof. Passing `verify_loose` does not mean that arbitrary functions can be translated. The existing [snapshot normalizer](https://github.com/ChenXu233/YaoXiang/blob/e64139dc3d10fb6e376ac6980a04a2ef2e296432/src/middle/ir/normalize.rs) is only used for printing; it is not a compilation pass that eliminates stack instructions. |
| The [module IR](https://github.com/ChenXu233/YaoXiang/blob/e64139dc3d10fb6e376ac6980a04a2ef2e296432/src/middle/ir/ir.rs) already carries initialization sequences, the entry point, source files, FFI bindings, drop instructions, and spawn execution plans; the [runtime interface](https://github.com/ChenXu233/YaoXiang/blob/e64139dc3d10fb6e376ac6980a04a2ef2e296432/src/backends/runtime/facade.rs) uses Rust task closures.                                                                                                                                                                                             | The original module remains the source of truth for shared semantics. The native backend is responsible only for representation, calling conventions, and emission; Rust closures and the VM's `RuntimeValue` are not native binary interfaces (ABIs).                                                                                                                                                                  |

Select the artifact at the L1 orchestration layer, and manage Inkwell, LLVM modules, optimization,
and target files within L4's `backends::llvm`. L2/L3 do not introduce LLVM types; L4 does not query
the AST or type checker. Do not invent a generic backend trait in advance. The first batch excludes
JIT compilation, cross-platform cross-compilation, default backend switching, and performance
commitments.

## What to Confirm Before Starting

| Decision point               | Recommended approach and cost                                                                                                                                                                                                                                                                                                                 | Alternative approach and boundary                                                                                                                                                                                                                                                       |
| ---------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Public entry point           | Explicitly select LLVM in the existing build command, for example `yx build demo.yx --backend llvm` (**proposed syntax; currently unavailable**). Preserve the VM behavior of `yx run` and the default `.42`; return a clear error when an unavailable backend is explicitly selected. The cost is maintaining an additional artifact format. | Changing the default would eliminate the need for an option, but existing scripts would receive a different format and require a new toolchain. This is a breaking change and is out of scope for this plan.                                                                            |
| Toolchain and first platform | Following RFC-018, adopt Inkwell and initially target `x86_64-unknown-linux-gnu`. Enable the optional Cargo feature only on native hosts; pin and validate a compatible combination of Inkwell, its LLVM bindings, and the system LLVM. The existing Linux CI can be reused, but it cannot be used to announce support for other ABIs.        | Expanding to multiple targets immediately would expose platform differences earlier, but the first error would be difficult to distinguish between language translation, the runtime, or the platform ABI. Validate additional targets incrementally.                                   |
| Relationship with P7         | First validate the toolchain on a decidable subset; formal control-flow translation waits for shared types and SSA. The cost is that the prototype explicitly rejects some legal programs.                                                                                                                                                    | Waiting until all of P7 is complete avoids temporary adaptations, but postpones linking and release risks until scope expansion. Do not choose an approach that independently restores the entire type system and SSA in the LLVM backend: that would create a second set of semantics. |

Maintainers should first confirm the initial scope, CLI contract, and cross-task prerequisites, then
register the work under RFC-018's tracking item. This does not consume a number from the
[RFC-039 work breakdown](../architecture/09-execution-wbs.md). The batch names below indicate only
dependency relationships; they do not imply that the work has been approved or completed.

### Review Before Registering the Initial Tasks

Run the following read-only commands at the repository root. The first two locate the actual
compilation fork and IR verification entry point; the last two verify the build arguments and test
budget supported by the batch tooling. If the stage table or verification contract has changed,
update the task prerequisites first rather than copying this page's baseline revision.

```bash
git grep -n -e 'const SINGLE_FILE_STAGES' -e 'const MULTI_FILE_STAGES' -- src/driver/program.rs
git grep -n -e 'pub fn verify_loose' -e 'pub fn verify_ssa' -- src/middle/ir/verify.rs
python scripts/dev/scratch.py build --help
python scripts/dev/testsuite.py plan
```

## Implementation Sequence and Deliverables

**Dependencies**: toolchain → native sample; once both the native sample and shared SSA requirements
are satisfied, proceed to production code generation. After that, establish the lifetimes of native
values and resources, then integrate FFI and spawn. For each batch, the implementer delivers the
artifacts and reproduction steps, and an independent verifier reviews the release criteria. Corpus
differential testing starts with the first executable rather than being concentrated immediately
before release.

### 1. Stabilize the Build Environment and Isolate LLVM Dependencies

- **Implementation**: The build lead selects compatible versions and locks the dependencies. Record
  the host, target triple, LLVM version, system linker driver, and the locations and acquisition
  methods for the target runtime libraries. Use LLVM's `TargetMachine` to set the target triple and
  `DataLayout` for the module; object layout must not use the sizes of host Rust types.
- **Deliverable**: Two reproducible build channels. The channel without LLVM builds the default CLI,
  VM, and Wasm Playground; the channel with LLVM builds the optional backend. The
  [existing CI](https://github.com/ChenXu233/YaoXiang/blob/e64139dc3d10fb6e376ac6980a04a2ef2e296432/.github/workflows/ci.yml)
  uses `--all-features`; these jobs explicitly provision the pinned LLVM and do not remove the
  feature merely to make the build pass.
- **Release criteria**: An environment without LLVM can complete the default build; an environment
  with LLVM can validate a module, generate a target file, link, and run it. Also verify the two
  failure modes of a missing linker and an LLVM major-version mismatch, distinguishing environment
  errors from source-program errors. The
  [batch build tool](https://github.com/ChenXu233/YaoXiang/blob/e64139dc3d10fb6e376ac6980a04a2ef2e296432/scripts/dev/scratch.py)
  currently builds only the default engine; first register and approve explicit feature support,
  then verify the optional build according to the [measurement procedure](../HOWTO.md). Do not
  create a separate build command that bypasses the batch tooling.

### 2. Deliver the First Real Source Program Without Waiting for P7

- **Input**: Have the existing Driver compile a single-file script. Accept only data flow whose
  types can be determined from constants, known function signatures, and straight-line instructions.
  Support `Int`, `Bool`, direct calls, returns, top-level initialization, and the minimum assertion
  entry point needed to observe results. Do not promise general standard-library support.
- **Generation**: First use `verify_loose` to exclude structural errors, then check the type
  justification for each instruction. Declare functions first, then translate their bodies and
  module initialization. For scripts, call only the initialization sequence; do not look up `main`
  again by name. Perform LLVM module verification, target-file emission, and linking in sequence.
  Deliver the final file only after linking succeeds; a failure must not overwrite the last
  successful artifact.
- **Positive and negative examples**: Have the script pass arguments to an arithmetic function, then
  compare the return value through the existing
  [`std.assert.assert` contract](https://github.com/ChenXu233/YaoXiang/blob/e64139dc3d10fb6e376ac6980a04a2ef2e296432/src/std/assert.rs).
  When the expectation is correct, the program exits normally; when the expectation is deliberately
  made incorrect, both the VM and native programs report E6005 at the same source line. Resolve
  assertion symbols through the existing exported bindings; do not add special cases based on sample
  filenames in the compiler. Do not temporarily define the source program's function return value as
  the process exit code.
- **Stop conditions**: If the input contains unknown types, unproven indirect calls, or unsupported
  reachable instructions, the build must fail explicitly and identify the source; do not produce a
  partial artifact or fall back to the VM. If a code region cannot be proven unreachable, do not
  skip it either. A prototype failure is a capability boundary and does not change whether the same
  program is legal in the VM.
- **Release criteria**: The same source produces three results: VM, native O0, and native O2. Run
  the native artifact in an environment containing only the target system's base libraries, with no
  LLVM, compiler, or bytecode interpreter. Manually assembling an LLVM module, checking only for
  exit code 0, or saving only a `.ll` file does not satisfy this verification.

### 3. Integrate Production Shared IR and Extend Coverage to Control Flow and Multiple Files

**Prerequisites are determined by artifacts, not dates.** Check against the
[SSA design](../architecture/04-ssa.md) and the P7 work breakdown: the compiler must actually emit
per-value types, distinguish writable storage from single-assignment temporaries, and represent
predecessors and values at merge points; `verify_ssa` must pass on production IR containing branches
and loops. When parameters with zero quantification or proof fields are involved, also verify the
corresponding erasure contracts; do not treat every generic original as erased. If any condition is
missing, retain only Batch 2 and do not broaden the support statement.

1. First create basic blocks, then translate instructions and complete the predecessor inputs for
   `phi`. Map SSA values directly to LLVM values, and use storage for addressable mutable locals.
   For stack instructions that remain reachable, verify stack depth and element types along the
   control flow before eliminating them; reject inconsistent predecessors so that invalid IR does
   not enter the optimizer.
2. Declare call targets according to their post-monomorphization signatures. The project uses the
   qualified names, global slots, and initialization order supplied by the orchestrator. Bin first
   executes initialization and then calls the designated entry point; Script does not implicitly
   call `main`, even if it defines one. An error in the initialization of a dependency module uses
   that module's file identity rather than always being attributed to the entry file.
3. Use the same source to validate both branches, a loop backedge, a direct cross-module generic
   call, import initialization order, and a script explicitly calling `main` exactly once. Use one
   erroneous input to validate the diagnostic file and line. Once the production path covers the
   prototype, remove prototype-specific adaptations; do not retain two instruction translation
   paths.

### 4. Link the Native Runtime, Then Integrate FFI and Spawn

| Deliverable slice                | Implementation boundary                                                                                                                                                                                                                                                                                                                             | Acceptance example                                                                                                                                                                                                                                                                         |
| -------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Values, closures, and resources  | Statically link the required runtime through a narrow C ABI. Define string byte lengths and ownership, container layouts, closure environments, and the releaser. Consume IR operations such as `Drop`, `RcNew`, and `ArcNew`; do not redo ownership analysis in the backend. Basis: [ownership design](../../rfc/accepted/009-ownership-model.md). | A closure can still read its captured value after leaving its defining scope; a moved value is not destroyed twice; resources on normal return, early return, and `?` propagation are each released exactly once in the prescribed order.                                                  |
| Foreign Function Interface (FFI) | Consume bindings and known signatures, distinguishing standard-library handlers from external C symbols. Convert parameters and return values according to the target ABI; do not treat Rust enum or closure layouts as C layouts. Basis: [FFI core mechanism](../../rfc/accepted/026-ffi-core-mechanism.md).                                       | Compile a C test library with real signatures using the same target toolchain; round-trip values between YaoXiang and C. Cover null pointers, string boundaries, and missing symbols; merely verifying that a simple call does not crash is insufficient.                                  |
| Spawn                            | Consume the `ExecutionPlan` carried by `Spawn` and `SpawnFromList`, and pass task entry points, environments, and dependencies to the scheduler. Sequential code does not start the scheduler. Basis: [runtime architecture](../../rfc/accepted/008-runtime-concurrency-model.md), [spawn semantics](../../rfc/accepted/024-concurrency-model.md).  | Tasks with dependencies consume results in order, conflicting resources are not accessed concurrently, and task failures propagate to the caller. For tasks without dependencies, verify only the permitted outcomes; do not define a particular thread scheduling order as the semantics. |

Start each slice from the IR produced by the responsible producer. If a necessary layout, type, or
release fact is missing, first create work in the layer that owns that fact to fill the gap; do not
complete the interface with a guessed layout. At this stage, the runtime ABI is treated as an
internal interface and does not imply compatibility with third-party components linked by users.

## How to Establish Equivalence Between the Two Backends

The verification owner compares **VM / native O0 /native O2** using the same revision, target, and
input. Reuse the three categories of expected outcomes from the
[corpus probe](https://github.com/ChenXu233/YaoXiang/blob/e64139dc3d10fb6e376ac6980a04a2ef2e296432/examples/corpus_probe.rs):
normal behavior, compile errors, and runtime errors. Record compilation exit status and runtime exit
status separately, preventing “failed to compile” from being miscounted as “failed at runtime as
expected.” Supported corpus entries must not be skipped.

| Error to prevent                                          | Input and criterion                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| --------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Integer semantics become LLVM defaults                    | Use function parameters to force runtime computation: `-7 % 3 == 2` and `7 % -3 == -2`; division by zero reports E6001, adding one to the maximum signed integer reports E6007, and dividing the minimum value by -1 follows the overflow diagnostic path. Do not emit `sdiv` without safeguards or treat raw `srem` as mathematical modulo. Ready-made comparisons: [modulo](https://github.com/ChenXu233/YaoXiang/blob/e64139dc3d10fb6e376ac6980a04a2ef2e296432/tests/yaoxiang/01-syntax/basics/modulo_math.yx), [division by zero](https://github.com/ChenXu233/YaoXiang/blob/e64139dc3d10fb6e376ac6980a04a2ef2e296432/tests/yaoxiang/06-compile-errors/runtime/div_zero.yx), [overflow](https://github.com/ChenXu233/YaoXiang/blob/e64139dc3d10fb6e376ac6980a04a2ef2e296432/tests/yaoxiang/06-compile-errors/runtime/int_overflow.yx). |
| Optimization changes behavior                             | Compare the observable values and error paths of O0, O2, and the VM with one another. When expanding floating-point coverage, include NaN, signed zero, and mixed numerical operations; do not enable fast-math assumptions that change these results. Passing the LLVM verifier only establishes structural validity.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| Sequencing or position errors are masked by normalization | Compare exit codes, output, and the code, file, and line of structured diagnostics; do not compare translated text, machine code, or the text of the complete LLVM IR. The existing probe sorts standard output by line; ordered side effects such as initialization and destruction require separate ordered event recording or final-state assertions.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| Mistaking “unsupported” for “pass”                        | For every scope expansion, record the supported corpus, expected success count, expected error count, unsupported features, and their corresponding implementation batches. A backend rejection of a supported item is a regression; it must not be removed from the set merely to make the gate pass. When the frontend rejects a program, neither build path produces a file.                                                                                                                                                                                                                                                                                                                                                                                                                                                            |

Continue to follow the snapshot, corpus, and resource-budget rules in the
[equivalence criteria](../architecture/07-equivalence-oracle.md) and the
[implementer's guide](../HOWTO.md). Use controlled mutations to prove that key probes are effective;
for example, replace mathematical modulo with truncating remainder and verify that cases with
negative operands fail. When adding the native path, do not update the VM baseline merely because
the AOT result differs. If the language specification conflicts with the VM, record a minimal source
program and route it according to D59; do not adjudicate language semantics within the backend.

## Release and Rollback

- **Label prototypes, limited previews, and formal support separately.** A prototype proves only a
  single-platform pipeline. A preview lists its supported scope and explicit exclusions in the
  release notes. Before formally supporting a target, reconcile each commitment in RFC-018,
  including standard-library dependencies, reflection metadata, and declared runtime modes;
  undelivered items do not count as complete.
- **Accept each target separately.** After the Linux ELF passes, separately validate COFF on Windows
  x64 MSVC and Mach-O on macOS. For each target, actually compile, link, and run on the
  corresponding system; inspect the architecture, dynamic-library dependencies, and FFI calling
  convention. Verify `x86_64-pc-windows-msvc` for Windows; record `x86_64-apple-darwin` and
  `aarch64-apple-darwin` for macOS separately. Record compilation time, artifact size, and startup
  and execution time separately; make no speedup claims without measured values.
- **Action on failure.** Roll back unreleased slices by batch; do not release a platform that has
  not passed the gate, and retain the default VM build. A released explicit option must continue to
  produce clear diagnostics and must not silently revert to bytecode. Changing the default artifact,
  changing the results of existing successful programs, exposing a public ABI, or changing an ABI
  that has already been promised all require separate review.
- **Handoff evidence.** Associate each batch delivery with a commit or PR, publicly reproducible
  source code, toolchain versions, target triple, commands, and expected and actual results. An
  independent verifier must rerun the positive and negative examples before the supported scope is
  expanded. Approval of the documentation does not constitute approval of scheduling or release.
