---
title: 'RFC-029a: Module Caching and Incremental Recompilation'
status: 'Draft'
author: 'Chenxu'
created: '2026-09-07'
updated: '2026-09-07'
issue: '#293'
---

# RFC-029a: Module Caching and Incremental Recompilation

## Summary

Fulfill the 029a slot reserved by RFC-029 "Sub-RFC Planning": on top of the now-stable orchestrator,
define **layered caching and incremental recompilation semantics** — L1 in-process analysis results,
L2 in-session module-level, L3 cross-process disk — with unified key definitions, invalidation
rules, and mandatory measurement hooks, converging the currently disconnected cache scattered points
(#251 P0-6 audit).

## Motivation

### Host and Boundary Basis

RFC-029 (accepted) explicitly scopes this topic out of itself, and reserves the slot for this
sub-RFC:

> **Excludes**: caching, file watching, hot reload, incremental recompilation, cross-package
> circular dependency handling. — RFC-029 §Core Principles

| Sub-RFC | Capability                                   | Prerequisite        |
| ------- | -------------------------------------------- | ------------------- |
| 029a    | Module Caching and Incremental Recompilation | Orchestrator stable |

The prerequisite "orchestrator stable" is met: the orchestrator landed with RFC-029 and was verified
through deliveries #232/#243/#244/#245.

### Current Problems

**Disconnected cache scattered points**:

| Existing                       | Layer         | Actual Consumer  | Problem                                                     |
| ------------------------------ | ------------- | ---------------- | ----------------------------------------------------------- |
| `VALIDATE_CACHE` (validate.rs) | In-process    | Formatter only   | CLI `check`/`run` and orchestrator don't go through it      |
| Z3Backend query cache          | Check session | Proof pipeline   | No invalidation rules or budget design (RFC-009a one-liner) |
| `DocumentCache` (RFC-017)      | LSP session   | LSP              | Completely disconnected from the CLI compilation face       |
| `~/.yaoxiang/cache` (RFC-014)  | Disk          | Package download | Unrelated to compilation                                    |
| Code cache (RFC-028)           | Runtime       | JIT              | Not a compile-time layer                                    |

**Multi-entry duplicate full-pipeline (2026-09-07 empirical)**:

- `check_files_with_diagnostics` loop creates a new `Compiler` for each file: when validating 19
  library-layer files in the same process, the std embed chain is typechecked 19 times;
- runtime-error test files go through the `check` and `run` sub-processes, the full pipeline runs
  twice;
- borrow checking `fast_path_check` performs independent full-graph BFS for each write operation,
  with no result cache (#251 P0-6 audit original finding; RFC-009a only has one sentence — "one BFS
  result can be cached and reused for multiple queries of the same token" — with no mechanism
  design).

**Cost anatomy (--version baseline method)**: process spawn + CLI initialization ~50 ms; bare file
`check` ~60 ms (frontend compilation ≈ 10 ms); `std.test` chain file ~80 ms (embed chain ≈ +20 ms).
Test loop of 170 files measured 9.8 s ≈ 58 ms/file, **~85% is process lifecycle cost**. This
determines this RFC's benefit positioning: eliminate multi-entry duplication and std chain duplicate
compilation; process spawn cost is outside the cache's reach (sub-process isolation is a design
choice in RFC-036 §6, see Non-Goals).

## Proposal

### 1. Three-Layer Cache Model

Caching is layered by **lifecycle**, with each layer having a unique host and clear entries:

| Layer | Lifecycle                                  | Host                               | Typical Entries                                                        |
| ----- | ------------------------------------------ | ---------------------------------- | ---------------------------------------------------------------------- |
| L1    | Single check session                       | `validate_source` / proof pipeline | Validation results (diagnostics + AST), borrow unsafe set, SMT queries |
| L2    | Single orchestration (reusable in-process) | `CompileSession`                   | Registry, per-module validation results                                |
| L3    | Cross-run                                  | Disk `cache/compile/`              | Module bytecode                                                        |

Layering criterion: who consumes the entry, and with whom it invalidates. L1 entries are consumed by
only a single check; L2 entries are consumed by multiple entries within one orchestration; L3
entries are consumed across processes. No entry sharing across layers — the same semantic product is
allowed to exist in multiple layers simultaneously, each layer hits and counts independently.

### 2. Cache Key: Content Hash Is Identity

```rust
struct CacheKey {
    content_hash: u64,          // FNV-1a, validate.rs existing implementation
    compiler_version: Option<Version>,   // Only carried by L3
    config_fingerprint: Option<u64>,     // Only carried by L3: hash of config bits affecting semantics
}
```

- L1/L2 key = `content_hash`: source is identity, same content must hit, independent of path;
- L3 key = the entire triple: any version or fingerprint mismatch is a miss, full-key recompile;
- **Incomplete keys refuse to write**: L3 entries missing version or fingerprint are not written to
  disk — better to miss than persist stale products, the same construct-time refusal principle as
  diagnostics i18n missing translations refusing compilation.

### 3. L1: validate_source Is the Only Frontend Entry

All components needing "source → (diagnostics, AST)" must go through `validate_source`; building a
private frontend chain is not allowed. It already exists with a process-level cache, but only the
formatter consumes it; this RFC folds in the three bypassers:

| Consumer                        | Current State                                   | After Folding In                         |
| ------------------------------- | ----------------------------------------------- | ---------------------------------------- |
| `check_files_with_diagnostics`  | `Compiler::new()` per file                      | Look up / backfill via `validate_source` |
| Orchestrator per-file typecheck | Self-built Registry with duplicate construction | Look up / backfill via `validate_source` |
| LSP diagnostics                 | Independent DocumentCache system                | Via `validate_source` (third wave)       |

Borrow analysis result cache is the second line of L1. Define **function check session** as the
borrow cache's lifecycle unit:

```rust
struct FunctionCheckSession {
    /// Full-graph BFS-derived unsafe set — built on function entry, dropped on function exit
    unsafe_set: HashSet<TokenId>,
    /// SMT query cache (linear arithmetic queries within RFC-027 budget)
    smt_cache: HashMap<u64, SMTResult>,
}
```

Multiple write operations of the same function share one BFS (realizing RFC-009a's design sentence);
the SMT query cache is folded into the same session object and destroyed with the session. The
session does not outlive the function — intra-function code invariance is guaranteed by the type
checking process itself.

### 4. L2: In-Session Module Cache

The orchestration session is a first-class entity, no longer implicitly reconstructed each time:

```rust
pub struct CompileSession {
    /// Key = (module path, content_hash); path distinguishes different files with same-name content
    modules: HashMap<(PathBuf, u64), Arc<ModuleEntry>>,
}

struct ModuleEntry {
    validated: Arc<ValidateResult>,
}
```

- std embed source (`yx_sources` include_str!) content is constant, its hash computed once
  in-process (`LazyLock` static table) — N import files share one compilation;
- A second `compile_project` in the same process (e.g., the test loop's check + run, yx_runner
  multi-file) looks up `CompileSession` per module; if content is unchanged, the whole module is
  reused;
- The session dies with the process: no disk persistence, no cross-process obligation.

### 5. L3: Disk Bytecode Cache

```text
~/.yaoxiang/cache/compile/<content_hash>-<version>-<fp>.yxbc
```

- Product = existing `BytecodeFile` magic-number format (the `run` bytecode probe path consumes
  as-is), no new serialization format invented;
- Load = read file + verify that the filename triple matches the content; any mismatch is treated as
  a miss and the bad entry is deleted;
- The first batch caches only std modules: embed source content is stable, version-anchored, with a
  constant hit rate; project module caching starts after the open questions are resolved.

### 6. Invalidation Rules

| Rule | Trigger                      | Action                                                                                          |
| ---- | ---------------------------- | ----------------------------------------------------------------------------------------------- |
| R1   | Source content change        | No active invalidation — key contains content hash, old entries are naturally unreachable       |
| R2   | Dependent module change (L2) | Mark dirty downstream along the `use` graph, recompile only the dirty module and its downstream |
| R3   | Compiler version change (L3) | Full miss (key contains version, no cross-version migration)                                    |
| R4   | Incomplete key (L3)          | Refuse to write                                                                                 |

Invalidation propagation only exists within the L2 dependency graph; L3 does not track inter-module
dependencies — the version guard is the fallback, semantics never depend on an old product being
"still valid by coincidence".

### 7. Measurement: Caches Without Observability May Not Merge

```rust
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub cached_bytes: u64,
}
```

The `check --json` and `test --json` summaries attach a `cache` field:

```json
{
  "cache": {
    "l1": { "hits": 18, "misses": 1 },
    "l2": { "hits": 0, "misses": 19 },
    "l3": { "hits": 57, "misses": 3, "cached_bytes": 245760 }
  }
}
```

Any cache entry PR merging to mainline must attach hit evidence for that field, preventing the
silent rot of "cached but no one hits" (#289 observability alignment).

## Detailed Design

### Type System Impact

None. This RFC introduces no language types, syntax, or semantic changes; cache entry types all
reuse existing `ValidateResult` / `ModuleRegistry` / `BytecodeFile`.

### Runtime Behavior

- `run`'s execution semantics are unchanged; an L3 hit is equivalent to "load bytecode from disk",
  going through the existing `BytecodeFile::probe`/`load` path, byte-level isomorphic;
- On a full cache miss, behavior is completely identical to today (the cache must be transparent —
  this is an acceptance item, not a vision);
- Observable changes are only two places: latency drops; `check`/`test`'s `--json` output adds a
  `cache` field (purely additive, old consumers unaffected).

### Compiler Changes

| Component                                        | Change                                                                            |
| ------------------------------------------------ | --------------------------------------------------------------------------------- |
| `src/frontend/validate.rs`                       | Becomes the only frontend entry (consumer folding, see Proposal §3)               |
| `src/util/diagnostic/mod.rs`                     | `check_files_with_diagnostics` removes per-file `Compiler::new()`                 |
| `src/frontend/module/orchestrator.rs`            | `compile_project` accepts `CompileSession` (L2)                                   |
| `src/frontend/core/typecheck/proof/`             | `FunctionCheckSession` (L1 borrow/SMT cache)                                      |
| `src/frontend/pipeline/compilation_cache.rs`     | New — L3 read/write and `CacheStats` (placeholder tests reserved)                 |
| `src/frontend/pipeline/incremental_scheduler.rs` | New — dirty file determination along the `use` graph (placeholder tests reserved) |
| `src/util/cache.rs`                              | `DocumentCache` migrates to be an L1 consumer (third wave, RFC-017 folding)       |
| `src/main.rs` / `src/util/test_runner.rs`        | JSON report attaches `cache` field                                                |

### Backward Compatibility

- ✅ Language face: zero change; CLI face: zero breakage; JSON field: purely additive;
- ✅ The cache layer can be fully downgraded (clearing it returns to no-cache behavior), no
  migration path burden;
- L3 disk entries carry a version guard; after a version upgrade, old entries naturally miss, no
  cleanup tool needed (the semantics of `yaoxiang cache clean` extend from the existing RFC-014
  command to the `compile/` segment).

## Tradeoffs

### Pros

- Directly eliminates three measured wastes: in-process multi-entry duplicate full-pipeline, std
  embed chain N-times duplicate compilation (19 files × 20 ms → 1 × 20 ms), and borrow checking
  large-function O(V²) duplicate BFS;
- Cache strategy single-sourced: key definitions, invalidation rules, and measurement calibration
  are uniquely defined warehouse-wide, folding the five scattered points into the same set of
  semantics;
- The measurement mandate makes both cache benefits and rot visible.

### Cons

- L1 process-level cache introduces shared state: under `--parallel` it becomes a lock hotspot.
  Mitigation: entries are `Arc`-immutable, keys are `u64`, lock granularity = single bucket;
- L3 introduces a risk surface of "old products being loaded". Mitigation: version + fingerprint
  double guard, better to miss than be wrong, and the bytecode carries its own magic-number check;
- L2 makes the orchestrator from stateless to stateful, so the regression surface covers the full
  #232/#243-245 scenarios.

## Alternatives

| Alternative                                             | Why Not Selected                                                                                                                                                                            |
| ------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Only L3 disk cache                                      | Measured 85% of cost is in process lifecycle, L3 only touches the std chain ~20 ms/file; doesn't solve multi-entry duplication                                                              |
| Compilation daemon                                      | Highest benefit ceiling (also eats spawn cost), but introduces service lifecycle management and conflicts with RFC-036's sub-process isolation model; reserved for a future independent RFC |
| Each scattered point evolves separately, no unification | i.e., the current state — three sets of key/invalidation calibration, exactly the gap #293 points out                                                                                       |
| Test loop in-process runner                             | Belongs to RFC-036 isolation model adjudication; this RFC does not cross the line                                                                                                           |

## Implementation Strategy

### Dependencies

- Prerequisite: RFC-029 orchestrator (stable), RFC-009a proof pipeline (BFS cache host);
- Parallel: #247 `use` tracking discovery — incremental recompilation's dependency graph accuracy
  depends on it; before it lands, R2 dirty determination conservatively takes the full amount;
- Consumers: #290/#292 borrow checking implementations.

### Wave Division and Risk

1. **First Wave (L1)**: `validate_source` folds in three consumers + `FunctionCheckSession` +
   minimal `CacheStats`. Low risk — pure internal refactoring, full-miss behavior unchanged;
2. **Second Wave (L2)**: `CompileSession` + std embed single compilation. Medium risk — orchestrator
   becomes stateful, requires the full #232/#243-245 regression;
3. **Third Wave (L3 + Incremental)**: Disk cache (std priority) + `incremental_scheduler` +
   DocumentCache migration. High risk — cross-process product correctness depends entirely on the
   version guard.

## Open Questions

- [ ] Whether L3 products include type environment serialization (determines whether `check`-type
      consumers can hit L3, or only `run` can hit)
- [ ] The semantic bit list for `config_fingerprint`: which configuration items change compilation
      product semantics
- [ ] DocumentCache migration goes with the third wave, or as an independent small step first
- [ ] Hit rate attribution calibration: by call count or by deduplicated key count (affects the
      interpretability of metrics)

## Non-Goals

- Function-level incremental parsing (RFC-017's existing adjudication: full-file parsing is only a
  few milliseconds, not worth incrementing);
- JIT / runtime cache (RFC-028 jurisdiction);
- Overturning the test loop's sub-process isolation (RFC-036 §6 design choice; the daemon scheme is
  in Alternatives, separate RFC);
- Cross-package invalidation propagation (outside RFC-029's boundary);
- Process spawn cost optimization (not a cache topic).

---

### Appendix B: Design Decision Record

| Decision                                    | Determination                                                         | Date       | Recorder |
| ------------------------------------------- | --------------------------------------------------------------------- | ---------- | -------- |
| Host                                        | 029a slot reserved by RFC-029, no new top-level RFC                   | 2026-09-07 | Chenxu   |
| Layering model                              | L1 in-process / L2 in-session module-level / L3 disk                  | 2026-09-07 | Chenxu   |
| Benefit positioning                         | Main battlefield = multi-entry reuse and std chain elimination        | 2026-09-07 | Chenxu   |
| Sub-process isolation not overturned        | L3 is the only cross-process benefit channel in the sub-process model | 2026-09-07 | Chenxu   |
| L3 no cross-module invalidation propagation | Full-key invalidation via version guard                               | 2026-09-07 | Chenxu   |
| Measurement mandate                         | No metrics, no merge                                                  | 2026-09-07 | Chenxu   |

## References

- #293 (this RFC's issue); #251 P0-6 audit (borrow BFS no-cache finding)
- RFC-029 §Core Principles (boundary declaration) and §Sub-RFC Planning (029a slot)
- RFC-009a (borrow proof pipeline; BFS cache design sentence)
- RFC-017 §2 (LSP DocumentCache and file-level simplification adjudication)
- RFC-014 §Global Cache (disk directory); RFC-028 §Code Cache (runtime layer boundary)
