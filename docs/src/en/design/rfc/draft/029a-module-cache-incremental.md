---
title: 'RFC-029a: Module Cache and Incremental Recompilation'
status: 'Draft'
author: 'Chenxu'
created: '2026-09-07'
updated: '2026-09-07'
issue: '#293'
---

# RFC-029a: Module Cache and Incremental Recompilation

## Summary

Fulfills the 029a slot reserved in RFC-029 "Sub-RFC Planning": adding **layered caching and
incremental recompilation** to the compiler on top of the now-stable orchestrator
(`compile_project`). The core is a three-layer cache model—intra-process analysis results (L1),
in-session module-level (L2), cross-process disk (L3)—paired with unified keying and invalidation
principles and mandatory metrics hooks, converging the four currently disconnected cache silos
(discovered in the #251 P0-6 audit).

## Motivation

### Host and Boundary Basis

RFC-029 (accepted) explicitly drew this topic out of its own scope and reserved a slot for this
sub-RFC:

> **Out of scope**: caching, file watching, hot reload, incremental recompilation, cross-package
> circular dependency handling. — RFC-029 §Core Principles

| Sub-RFC | Capability                                 | Prerequisite        |
| ------- | ------------------------------------------ | ------------------- |
| 029a    | Module Cache and Incremental Recompilation | Orchestrator stable |

The prerequisite "orchestrator stable" has been met: the orchestrator was delivered with RFC-029 and
validated through #232/#243/#244/#245.

### Current Problems (#251 P0-6 Audit + 2026-09-07 Empirical Evidence)

**Cache silos are disconnected**:

| Existing                                 | Layer         | Actual Consumer  | Problem                                                           |
| ---------------------------------------- | ------------- | ---------------- | ----------------------------------------------------------------- |
| `VALIDATE_CACHE` (validate.rs)           | Intra-process | formatter only   | CLI `check`/`run` and the orchestrator don't go through it        |
| Z3Backend query cache                    | Check session | Proof pipeline   | No invalidation rules or budget design (one sentence in RFC-009a) |
| `DocumentCache` (util/cache.rs, RFC-017) | LSP session   | LSP              | Completely disconnected from the CLI compilation face             |
| `~/.yaoxiang/cache` (RFC-014)            | Disk          | Package download | Unrelated to compilation                                          |
| Code cache (RFC-028)                     | Runtime       | JIT              | Not a compile-time layer                                          |

**Multi-entry repeated full pipeline (empirical)**:

- `check_files_with_diagnostics` loop **constructs a new `Compiler` per file**
  (`src/util/diagnostic/mod.rs`): validating 19 library-layer files in the same process, the std
  embedded chain gets re-typechecked 19 times;
- runtime-error test files go through `check` + `run` as two sub-processes, the full pipeline runs
  twice;
- borrow check `fast_path_check` does a full graph BFS independently on every write, with no result
  cache (original #251 P0-6 audit finding; RFC-009a L611 only has the one sentence "a BFS result can
  be cached for reuse by multiple queries of the same token", with no mechanism design).

### Cost Dissection (--version Baseline Method, Measured 2026-09-07)

| Probe                             | Time   | Difference Meaning                 |
| --------------------------------- | ------ | ---------------------------------- |
| `yaoxiang --version`              | ~50 ms | Process spawn + CLI initialization |
| Bare file `check` (no std import) | ~60 ms | Frontend compile ≈ 10 ms           |
| `std.test` chain file `check`     | ~80 ms | Embedded std chain ≈ +20 ms        |

Measured 9.8 s ≈ 58 ms/file for 170 test-loop files: **~85% is process lifecycle cost, compilation
itself is only ~10 ms, std chain +20 ms/file**. From this, the revenue battlefield of this RFC is
located:

1. **Same-process multi-entry reuse** (L1/L2) — eliminate repeated frontend pipelines and std chain
   compilation;
2. **Cross-process std chain elimination** (L3) — the only legitimate cache revenue channel under
   the sub-process model;
3. Process spawn's ~50 ms/file is **outside cache's range** — sub-process isolation is RFC-036 §6's
   design choice, and this RFC does not overturn it (see Non-Goals).

## Proposal

### Core Design: Three-Layer Cache Model

#### L1 Intra-Process Analysis Result Cache (Lifetime = Single Check Session)

- `validate_source` consolidated as the **sole frontend validation entry point**:
  `check_files_with_diagnostics`, the orchestrator's per-file typecheck, and LSP diagnostics all go
  through it (currently only the formatter consumes it);
- Key = FNV-1a content hash (existing implementation in validate.rs), value = `ValidateResult`
  (diagnostics + AST);
- Borrow BFS unsafe-set cache: implements the design sentence from RFC-009a, lifetime = single
  function check session, aligned with #290/#292 borrow check implementations;
- Z3 query cache (RefCell + query hash) consolidated under the same principle, with invalidation and
  budget specs added.

#### L2 In-Session Module-Level Cache (Lifetime = Single Orchestration)

- The orchestrator's Registry and each module's validation result are cached by **(module path,
  content hash)**; the second `compile_project` in the same process hits directly;
- std embedded source (`yx_sources` include_str!) has constant content → compiled once per process,
  shared by N importing files;
- Invalidation: invalidates when this module's source hash changes; dependency changes propagate
  along the `use` graph (third-wave delivery).

#### L3 Cross-Process Disk Cache (Lifetime = Across Runs)

- Location `~/.yaoxiang/cache/compile/`, same root as RFC-014 download cache, different segment;
- Key = **(content hash, compiler version, semantic config fingerprint)**; product is module
  bytecode (reusing the `BytecodeFile` magic number format);
- Version or fingerprint mismatch = total miss and recompile — **no cross-module invalidation
  propagation** (conservative whole-key invalidation).

### Unified Keying and Invalidation Principles (#293 Gap 4)

- Key strength: content hash > structural hash > version number. L1/L2 use content hash (source is
  identity); L3 adds compiler version and config fingerprint guards on top;
- Invalidation propagation only exists inside the L2 dependency graph; L3 whole-key invalidation, no
  module-to-module dependency tracking;
- **Construction-time rejection**: an incomplete key (missing hash/missing version) is rejected from
  write, prefer miss over stale product — same principle as refusing compilation when i18n
  diagnostics lack translations.

### Metrics Hooks (#289 Alignment)

```rust
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub cached_bytes: u64,
}
```

- `check --json` and `test --json` summary append a `cache` field (per-layer hit rate);
- **Mandatory threshold: unobservable cache is not allowed to merge in** — prevents silent rot of
  "cached but never hit".

### Incremental Recompilation Boundary (#293 Gap 2)

- **Start at file level**: dirty files determined along the `use` graph, only recompile dirty files
  and their downstream; evolving in lockstep with RFC-017's LSP file-level cache (DocumentCache
  migrates to be a consumer of L1+L2, not an independent system);
- **Function-level increment explicitly not done** (RFC-017 already declared this simplification; a
  few-millisecond full-file parse is not worth incrementalizing).

## Detailed Design

### Integration with the RFC-029 Orchestration Flow

```text
RFC-029 orchestration flow (delivered):

  discover source files → build Registry → per-file typecheck → whole compilation

This RFC insertion point:

  discover source files → [L2] query Registry/validation result cache by path + hash
                              ├─ hit → reuse
                              └─ miss → build (and backfill)
  per-file typecheck → [L1] validate_source single entry point (content hash query/backfill)
  whole compilation → product → [L3] serialize to disk (with version guard), loaded directly next process
```

### Test Loop Expected Gains (Quantified)

| Scenario                                          | Current              | After 029a            | Dependency Layer |
| ------------------------------------------------- | -------------------- | --------------------- | ---------------- |
| Same-process runner (yx_runner mode) 19 lib files | std chain 19×20 ms   | 1×20 ms + 19 hits     | L1+L2            |
| Sub-process runner (`yaoxiang test`)              | std chain 20 ms/file | ≈ 0 after hit         | L3               |
| Borrow check on large function                    | O(V²) BFS            | Reused within session | L1               |
| Process spawn (~50 ms/file)                       | Unchanged            | Unchanged (non-goal)  | —                |

## Relationship with Existing Systems

- **RFC-029**: Host. 029a is the fulfillment of its "Sub-RFC Planning" table;
- **RFC-009a**: Mechanization of the L611 design sentence (BFS cache, L1 layer);
- **RFC-017**: DocumentCache consolidated as an L1+L2 consumer, single source of cache policy;
- **RFC-014**: Disk directory division — `cache/` download segment vs. this RFC's `cache/compile/`
  segment;
- **RFC-028**: JIT code cache belongs to the runtime layer, not under this RFC's jurisdiction;
- **RFC-031**: Optimization pass scheduling belongs to the Pass manager; this RFC caches the I/O of
  the pass pipeline, the two are orthogonal;
- **RFC-036**: Test sub-process isolation model remains unchanged; L3 is its only cross-process
  revenue channel;
- **#251/#290/#292**: Audit sources and borrow check implementations (BFS cache consumers);
- **#289**: Metrics hooks observability alignment;
- **#247**: `use` tracking discovery (on-demand discovery) layered on top of the L2 dependency
  graph, purely a performance optimization.

## Implementation Strategy

1. **First wave (lands and yields value immediately)**: `validate_source` unified entry (three
   consumers wired in: check/orchestrator/LSP), borrow BFS session cache, minimal metrics hooks;
2. **Second wave**: L2 session module cache (std embedded single-time compilation + Registry reuse);
3. **Third wave**: L3 disk cache (std priority — embedded source content is stable, hit rate is
   constant) + file-level incremental recompilation along the `use` graph + LSP cache unification.

## Non-Goals

- Function-level incremental parsing (RFC-017's existing ruling);
- JIT / runtime cache (RFC-028);
- Overturning the test loop's sub-process isolation (RFC-036 §6 design choice);
- Cross-package invalidation propagation (outside RFC-029's boundary);
- Process spawn cost optimization (not a cache topic).

## Design Decision Log

| Decision                                          | Conclusion                                                               | Date       | Basis                                                    |
| ------------------------------------------------- | ------------------------------------------------------------------------ | ---------- | -------------------------------------------------------- |
| Host                                              | RFC-029's reserved 029a slot, no new top-level RFC                       | 2026-09-07 | RFC-029 §Sub-RFC Planning; user ruling                   |
| Layered model                                     | L1 intra-process / L2 session module / L3 disk                           | 2026-09-07 | Cache silo inventory + cost dissection                   |
| Revenue positioning                               | Main battlefield = multi-entry reuse + std chain elimination             | 2026-09-07 | --version baseline method: 85% cost in process lifecycle |
| Sub-process isolation not overturned              | L3 is the only cross-process revenue channel under the sub-process model | 2026-09-07 | RFC-036 §6 + cost dissection                             |
| L3 does not propagate invalidation across modules | Version guard whole-key invalidation                                     | 2026-09-07 | Construction-time rejection principle                    |
| Metrics mandatory                                 | No metrics, no merge                                                     | 2026-09-07 | #289 observability alignment                             |
